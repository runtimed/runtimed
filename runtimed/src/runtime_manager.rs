use anyhow::{anyhow, Error, Result};
use notify::{
    event::CreateKind, Config, Event, EventKind::Create, RecommendedWatcher, RecursiveMode, Watcher,
};
use runtimelib::jupyter::client::{JupyterRuntime, RuntimeId};
use runtimelib::jupyter::discovery::{get_jupyter_runtime_instances, is_connection_file};
use runtimelib::messaging::JupyterMessage;
use serde::Serialize;
use sqlx::Pool;
use sqlx::Sqlite;
use std::collections::HashMap;
use std::process::ExitStatus;
use std::sync::Arc;
use tokio::runtime::Handle;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::oneshot::Sender;
use tokio::sync::{broadcast, mpsc};
use tokio::sync::{Mutex, RwLock};

pub struct ChildRuntime {
    pub process: tokio::process::Child,
    pub exit_status: Option<ExitStatus>,
}

/// State maintained for an individual runtime in the RuntimeManager
#[derive(Serialize, Clone)]
pub struct RuntimeInstance {
    #[serde(flatten)]
    pub runtime: JupyterRuntime,
    /// To send commands/messages to the runtime
    #[serde(skip)]
    pub send_tx: mpsc::Sender<JupyterMessage>,
    /// To follow all messages from the runtime
    #[serde(skip)]
    pub broadcast_tx: broadcast::Sender<JupyterMessage>,
    /// For child process runtimes
    #[serde(skip)]
    pub child: Option<Arc<Mutex<ChildRuntime>>>,
}

impl RuntimeInstance {
    pub async fn get_receiver(&self) -> broadcast::Receiver<JupyterMessage> {
        self.broadcast_tx.subscribe()
    }

    pub async fn get_sender(&self) -> mpsc::Sender<JupyterMessage> {
        self.send_tx.clone()
    }
}

/// A collection of all known active runtimes
#[derive(Clone)]
pub struct RuntimeManager {
    lock: Arc<RwLock<HashMap<RuntimeId, RuntimeInstance>>>,
    db: Pool<Sqlite>,
}

impl RuntimeManager {
    /// Create a new Runtime Manager.
    /// 1. Initializes a `RuntimeManager` with all the runtimes found in the jupyter runtime folder
    /// 2. Sets up a notify watcher of the jupyter runtime diretory to automatically insert new
    ///    runtimes
    pub async fn new(db: &Pool<Sqlite>, shutdown_tx: Sender<()>) -> Result<RuntimeManager> {
        let manager = RuntimeManager {
            lock: Arc::new(RwLock::new(HashMap::<RuntimeId, RuntimeInstance>::new())),
            db: db.clone(),
        };

        // Watch the jupyter runtime directory
        let watcher_manager = manager.clone();
        tokio::spawn(async move { watcher_manager.watch_runtime_dir().await });

        // Load all the runtimes already in the runtime directory
        let initial_runtimes = get_jupyter_runtime_instances().await;
        for runtime in initial_runtimes {
            log::debug!("Gathering messages for runtime {:?}", runtime.id);
            match manager.insert(&runtime, None).await {
                Ok(_) => {}
                Err(e) => {
                    // What should we do? Delete the file? Log and move on?
                    log::error!("Failed to insert runtime: {}", e);
                }
            }
        }

        manager.spawn_daemon_signal_handler(shutdown_tx).await?;

        Ok(manager)
    }

    /// Establish a signal handler to send a signal to gracefully shutdown the runtimed web server.
    /// There is a 2 second delay to allow child runtimes to be killed and reaped.
    async fn spawn_daemon_signal_handler(&self, shutdown_tx: Sender<()>) -> Result<()> {
        let mut stream = signal(SignalKind::interrupt())?;

        tokio::spawn(async move {
            stream.recv().await;
            log::info!("Recieved interrupt signal, shutting down");
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            // Using expect() as we want everything to die anyway
            shutdown_tx
                .send(())
                .expect("Failed to send shutdown signal");
        });
        Ok(())
    }

    /// Get an iterator over all the runtimes in the collection
    pub async fn get_all(&self) -> impl Iterator<Item = RuntimeInstance> {
        self.lock.read().await.clone().into_values()
    }

    /// Get a single runtime by id
    pub async fn get(&self, id: RuntimeId) -> Option<RuntimeInstance> {
        self.lock.read().await.get(&id).cloned()
    }

    fn spawn_child_reaper(&self, async_child: Arc<Mutex<ChildRuntime>>) {
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                let mut child_proc = async_child.lock().await;
                match child_proc.process.try_wait() {
                    Ok(None) => {
                        // child lives, fall through to next loop iteration
                    }
                    Ok(Some(status)) => {
                        log::info!("Runtime finished: {}", status);
                        child_proc.exit_status = Some(status);
                        // TODO need to remove ConnectionFile from runtime directory
                        // TODO need to remove runtime from runtime manager
                        // This could happen in whatever checks the runtime states
                        break;
                    }
                    Err(e) => {
                        log::error!("Error waiting for runtime: {}", e);
                        break;
                    }
                }
            }
        });
    }

    fn spawn_child_signal_handler(&self, async_child: Arc<Mutex<ChildRuntime>>) {
        let mut stream = signal(SignalKind::interrupt()).unwrap();
        tokio::spawn(async move {
            stream.recv().await;

            let mut child = async_child.lock().await;
            if let Some(exit_status) = child.exit_status {
                log::debug!(
                    "Child SIGINT handler: child previously finished: {}",
                    exit_status
                );
                return;
            }
            log::debug!("Child SIGINT handler: sending SIGKILL to child process");

            // No need to wait around to see if it worked or not
            let _ = child.process.start_kill();
        });
    }

    pub async fn new_instance(&self, kernel_name: &String) -> Result<RuntimeId> {
        let k = runtimelib::jupyter::KernelspecDir::new(kernel_name).await?;
        let ci = runtimelib::jupyter::client::ConnectionInfo::new("127.0.0.1", kernel_name).await?;
        let connection_file_path = ci.generate_file_path();
        let runtime = JupyterRuntime::new(ci, connection_file_path);
        let mut command = k.command(&runtime.connection_file).await?;
        let child = Arc::new(Mutex::new(ChildRuntime {
            process: command.spawn()?,
            exit_status: None,
        }));

        self.spawn_child_reaper(child.clone());
        self.spawn_child_signal_handler(child.clone());

        // Insert the runtime into the RuntimeManager before writing the connection file
        // because the watcher will try to insert the runtime into the database, but will
        // not have access to the ChildRuntime process handle
        self.insert(&runtime, Some(child)).await?;
        runtime
            .connection_info
            .write(&runtime.connection_file)
            .await?;
        log::debug!(
            "Launched new {} runtime with id: {}",
            kernel_name,
            runtime.id
        );
        Ok(runtime.id)
    }

    /// 1. Insert the runtime by id into the runtime map
    /// 2. Start a task to watch all messages from the runtime and insert them into the database
    /// 3. Start a task to recieve messages and send time to the runtime
    ///
    /// Returns true if the runtime was inserted, false if the runtime was already present
    async fn insert(
        &self,
        runtime: &JupyterRuntime,
        child: Option<Arc<Mutex<ChildRuntime>>>,
    ) -> Result<()> {
        let (mpsc_tx, mut mpsc_rx) = mpsc::channel::<JupyterMessage>(1);
        let (broadcast_tx, _) = broadcast::channel::<JupyterMessage>(1);

        // Insert the runtime into the runtime collection if not present
        {
            let mut map = self.lock.write().await;
            if map.contains_key(&runtime.id) {
                return Err(anyhow!(
                    "Found connection file for existing runtime {:?}",
                    runtime.id
                ));
            }
            map.insert(
                runtime.id.clone(),
                RuntimeInstance {
                    runtime: runtime.clone(),
                    send_tx: mpsc_tx,
                    broadcast_tx: broadcast_tx.clone(),
                    child,
                },
            );
        }

        // Spawn the task to send messages to the runtime client
        let id = runtime.id.clone();
        let send_runtime = runtime.clone();
        let db = self.db.clone();
        tokio::spawn(async move {
            let client = send_runtime.attach().await;
            if let Ok(mut client) = client {
                while let Some(message) = mpsc_rx.recv().await {
                    crate::db::insert_message(&db, id.0, &message).await;

                    let response = client.send(message).await;
                    // TODO: Handle errors here
                    if let Ok(response) = response {
                        crate::db::insert_message(&db, id.0, &response).await;
                    }
                }
            }
        });

        // Spawn the task to process messages from the runtime client
        let recv_runtime = runtime.clone();
        let db = self.db.clone();
        tokio::spawn(async move {
            // TODO: This will hang indefinitely if the client can't be attached to
            let client = recv_runtime.attach().await;

            // TODO: how should we handle an error here?
            if let Ok(mut client) = client {
                loop {
                    let maybe_message = client.next_io().await;
                    if let Ok(message) = maybe_message {
                        crate::db::insert_message(&db, id.0, &message).await;

                        // This should only fail if there are no receivers
                        let _ = broadcast_tx.send(message);
                    } else {
                        // Log error with the actual error that occurred
                        log::error!(
                            "Failed to receive message from IOPub: {:?}",
                            maybe_message.err()
                        );
                    }
                }
            }
        });
        Ok(())
    }

    /// Watch the jupyter runtimes directory and insert new runtimes into the runtimes manager
    async fn watch_runtime_dir(&self) -> Result<(), Error> {
        let (tx, mut rx) = mpsc::channel(1);
        let handle = Handle::current();

        let mut watcher = RecommendedWatcher::new(
            move |res| {
                handle.block_on(async {
                    tx.send(res).await.unwrap();
                })
            },
            Config::default(),
        )?;

        watcher.watch(
            runtimelib::jupyter::dirs::runtime_dir().as_path(),
            RecursiveMode::Recursive,
        )?;

        loop {
            let res = match rx.recv().await {
                Some(res) => res,
                None => continue,
            };

            match res {
                Ok(event) => {
                    if let Create(CreateKind::File) = event.kind {
                        self.process_runtime_dir_create_event(event).await;
                    }
                }
                Err(error) => log::error!("Error: {error:?}"),
            }
        }
    }

    async fn process_runtime_dir_create_event(&self, event: Event) {
        for path in event.paths {
            if !is_connection_file(&path) {
                continue;
            }

            // Continue if runtime is already known
            {
                let runtime_id = RuntimeId::new(path.clone());
                let map = self.lock.write().await;
                if map.contains_key(&runtime_id) {
                    continue;
                }
            }

            log::debug!("New runtime file found {:?}", path);
            match JupyterRuntime::from_path_set_state(path.clone()).await {
                Ok(runtime) => {
                    let result = self.insert(&runtime, None).await;

                    if let Err(err) = result {
                        log::info!("{:?}", err);
                    } else {
                        log::debug!("Connected to runtime {:?}", path);
                    }
                }
                Err(err) => log::error!("Could not load runtime {:?}", err),
            };
        }
    }
}

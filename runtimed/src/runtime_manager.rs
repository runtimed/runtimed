use anyhow::{Error, Result};
use notify::{
    event::CreateKind, Config, Event, EventKind::Create, RecommendedWatcher, RecursiveMode, Watcher,
};
use runtimelib::jupyter::client::JupyterRuntime;
use runtimelib::jupyter::discovery::{get_jupyter_runtime_instances, is_connection_file};
use runtimelib::messaging::JupyterMessage;
use serde::Serialize;
use sqlx::Pool;
use sqlx::Sqlite;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::runtime::Handle;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::oneshot::Sender;
use tokio::sync::{broadcast, mpsc};
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

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
    pub child: Option<Arc<Mutex<tokio::process::Child>>>,
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
    lock: Arc<RwLock<HashMap<Uuid, RuntimeInstance>>>,
    db: Pool<Sqlite>,
}

impl RuntimeManager {
    /// Create a new Runtime Manager.
    /// 1. Initializes a `RuntimeManager` with all the runtimes found in the jupyter runtime folder
    /// 2. Sets up a notify watcher of the jupyter runtime diretory to automatically insert new
    ///    runtimes
    pub async fn new(db: &Pool<Sqlite>, shutdown_tx: Sender<()>) -> Result<RuntimeManager> {
        let manager = RuntimeManager {
            lock: Arc::new(RwLock::new(HashMap::<Uuid, RuntimeInstance>::new())),
            db: db.clone(),
        };

        // Watch the jupyter runtime directory
        let watcher_manager = manager.clone();
        tokio::spawn(async move { watcher_manager.watch_runtime_dir().await });

        // Load all the runtimes already in the runtime directory
        let initial_runtimes = get_jupyter_runtime_instances().await;
        for runtime in initial_runtimes {
            log::debug!("Gathering messages for runtime {}", runtime.id);
            manager.insert(&runtime, None).await;
        }

        manager.spawn_signal_handler(shutdown_tx).await?;

        Ok(manager)
    }

    /// Establish a signal handler to send a signal to gracefully shutdown the runtimed web server.
    /// There is a 2 second delay to allow child runtimes to be killed and reaped.
    async fn spawn_signal_handler(&self, shutdown_tx: Sender<()>) -> Result<()> {
        let mut stream = signal(SignalKind::interrupt())?;

        tokio::spawn(async move {
            stream.recv().await;
            log::debug!("Recieved interrupt signal");
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
    pub async fn get(&self, id: Uuid) -> Option<RuntimeInstance> {
        self.lock.read().await.get(&id).cloned()
    }

    pub async fn new_instance(&self, kernel_name: &String) -> Result<Uuid> {
        let k = runtimelib::jupyter::KernelspecDir::new(kernel_name).await?;
        let ci = runtimelib::jupyter::client::ConnectionInfo::new("127.0.0.1", kernel_name).await?;
        let connection_file_path = ci.generate_file_path();
        let runtime = JupyterRuntime::new(ci, connection_file_path);
        let mut command = k.command(&runtime.connection_file)?;
        let child = Arc::new(Mutex::new(command.spawn()?));

        let async_child = child.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                let mut child_proc = async_child.lock().await;
                match child_proc.try_wait() {
                    Ok(None) => {
                        // Fall through to next loop iteration
                    }
                    Ok(Some(status)) => {
                        log::info!("Runtime exited with status: {}", status);
                        break;
                    }
                    Err(e) => {
                        log::error!("Error waiting for runtime: {}", e);
                        break;
                    }
                }
            }
        });

        let mut stream = signal(SignalKind::interrupt())?;
        tokio::spawn(async move {
            stream.recv().await;
            log::debug!("CHILD PROCESS HANDLER Recieved interrupt signal");
            // TODO is there a more concise, idiomatic way to disregard errors?
            // Disregard the error
            match child.lock().await.start_kill() {
                Ok(_) => {}
                Err(_) => {}
            };
            log::debug!("CHILD PROCESS HANDLER finished kill");
        });

        // insert the runtime into the runtime manager, otherwise
        // the file watcher might not insert it before the client needs it
        self.insert(&runtime, None).await;
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
        child: Option<Arc<Mutex<tokio::process::Child>>>,
    ) -> bool {
        let (mpsc_tx, mut mpsc_rx) = mpsc::channel::<JupyterMessage>(1);
        let (broadcast_tx, _) = broadcast::channel::<JupyterMessage>(1);

        // Insert the runtime into the runtime collection if not present
        {
            let mut map = self.lock.write().await;
            if map.contains_key(&runtime.id) {
                return false;
            }
            map.insert(
                runtime.id,
                RuntimeInstance {
                    runtime: runtime.clone(),
                    send_tx: mpsc_tx,
                    broadcast_tx: broadcast_tx.clone(),
                    child: child,
                },
            );
        }

        // Spawn the task to send messages to the runtime client
        let id = runtime.id;
        let send_runtime = runtime.clone();
        let db = self.db.clone();
        tokio::spawn(async move {
            let client = send_runtime.attach().await;
            if let Ok(mut client) = client {
                while let Some(message) = mpsc_rx.recv().await {
                    crate::db::insert_message(&db, id, &message).await;

                    let response = client.send(message).await;
                    // TODO: Handle herrors here
                    if let Ok(response) = response {
                        crate::db::insert_message(&db, id, &response).await;
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
                        crate::db::insert_message(&db, id, &message).await;

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
        true
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

            log::debug!("New runtime file found {:?}", path);
            match JupyterRuntime::from_path_set_state(path.clone()).await {
                Ok(runtime) => {
                    if self.insert(&runtime, None).await {
                        log::debug!("Connected to runtime {:?}", path);
                    } else {
                        log::debug!("Runtime already exists {:?}", path);
                    }
                }
                Err(err) => log::error!("Could not load runtime {:?}", err),
            };
        }
    }
}

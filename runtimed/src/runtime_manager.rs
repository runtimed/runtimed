use crate::child_runtime::ChildRuntime;
use runtimelib::jupyter::client::{ConnectionInfo, JupyterRuntime, RuntimeId};
use runtimelib::jupyter::discovery::{get_jupyter_runtime_instances, is_connection_file};
use runtimelib::messaging::content::ReplyStatus;
use runtimelib::messaging::{JupyterMessage, JupyterMessageContent, ShutdownRequest};

use anyhow::{anyhow, Error, Result};
use notify::{
    event::CreateKind, Config, Event, EventKind::Create, RecommendedWatcher, RecursiveMode, Watcher,
};
use serde::Serialize;
use sqlx::Pool;
use sqlx::Sqlite;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::runtime::Handle;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::oneshot::Sender;
use tokio::sync::RwLock;
use tokio::sync::{broadcast, mpsc};

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
    pub child: Option<ChildRuntime>,
}

impl RuntimeInstance {
    pub async fn get_receiver(&self) -> broadcast::Receiver<JupyterMessage> {
        self.broadcast_tx.subscribe()
    }

    pub async fn get_sender(&self) -> mpsc::Sender<JupyterMessage> {
        self.send_tx.clone()
    }

    pub async fn stop(&self) -> Result<()> {
        log::debug!("Starting a stop request");

        let mut client = self.runtime.attach().await?;
        log::debug!("Attached to the client");

        let message = JupyterMessage::new(
            JupyterMessageContent::ShutdownRequest(ShutdownRequest { restart: false }),
            None,
        );
        log::debug!("Made a message");

        let reply = client.send_control(message).await?;

        let JupyterMessageContent::ShutdownReply(reply) = reply.content else {
            return Err(anyhow!("Unexpected reply to shutdown request: {:?}", reply));
        };

        match reply.status {
            ReplyStatus::Ok => Ok(()),
            ReplyStatus::Error => Err(anyhow!("Unexpected reply to shutdown request: {:?}", reply)),
            _ => Err(anyhow!("Unexpected reply to shutdown request: {:?}", reply)),
        }
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
    pub async fn new(db: &Pool<Sqlite>, shutdown_tx: Option<Sender<()>>) -> Result<RuntimeManager> {
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
    async fn spawn_daemon_signal_handler(&self, shutdown_tx: Option<Sender<()>>) -> Result<()> {
        let mut stream = signal(SignalKind::interrupt())?;

        tokio::spawn(async move {
            stream.recv().await;
            log::info!("Recieved interrupt signal, shutting down");
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            if let Some(shutdown_tx) = shutdown_tx {
                // Using expect() as we want everything to die anyway
                shutdown_tx
                    .send(())
                    .expect("Failed to send shutdown signal");
            }
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

    pub async fn new_instance(&self, kernel_name: &String) -> Result<RuntimeId> {
        let k = runtimelib::jupyter::KernelspecDir::new(kernel_name).await?;
        let ci = ConnectionInfo::from_peeking_ports("127.0.0.1", kernel_name).await?;
        let connection_file_path = ci.generate_file_path();
        let runtime = JupyterRuntime::new(ci, connection_file_path.clone());

        // Get the runtime into the map, so that the NotifyWatcher doesn't
        // conflict on insertion
        self.insert(&runtime, None).await?;

        runtime
            .connection_info
            .write(&runtime.connection_file)
            .await?;

        let child = ChildRuntime::new(k, &runtime, self.lock.clone()).await?;
        self.update_runtime(runtime.id.clone(), child.clone())
            .await?;

        log::info!("Launched new {kernel_name} runtime with id: {}", runtime.id);
        Ok(runtime.id)
    }

    async fn update_runtime(&self, id: RuntimeId, child: ChildRuntime) -> Result<()> {
        let mut map = self.lock.write().await;
        if let Some(runtime) = map.get_mut(&id) {
            runtime.child = Some(child);
            Ok(())
        } else {
            Err(anyhow!("Runtime not found: {}", id))
        }
    }

    /// 1. Insert the runtime by id into the runtime map
    /// 2. Start a task to watch all messages from the runtime and insert them into the database
    /// 3. Start a task to recieve messages and send time to the runtime
    ///
    /// Returns true if the runtime was inserted, false if the runtime was already present
    async fn insert(&self, runtime: &JupyterRuntime, child: Option<ChildRuntime>) -> Result<()> {
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
                    let maybe_message = client.recv_io().await;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use sqlx::sqlite::SqlitePoolOptions;

    const TESTING_DATABASE_URL: &str = "sqlite:TESTING-runtimed.db?mode=rwc";

    #[tokio::test]
    async fn hello_world() -> Result<(), Error> {
        assert_eq!(1, 1);

        let dbpool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(TESTING_DATABASE_URL)
            .await?;

        let manager = RuntimeManager::new(&dbpool, None).await?;

        let runtime = JupyterRuntime {
            connection_info: ConnectionInfo::from_peeking_ports("127.0.0.1", "testing").await?,
            state: "testing".to_string(),
            id: RuntimeId::new(PathBuf::from("test")),
            connection_file: PathBuf::from("test"),
            kernel_info: None,
        };

        manager.insert(&runtime, None).await.unwrap();

        let fetched_runtime = manager.get(runtime.id).await.unwrap();
        assert_eq!(fetched_runtime.runtime.id, runtime.id);
        Ok(())
    }
}

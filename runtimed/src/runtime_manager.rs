use anyhow::Error;
use notify::{
    event::CreateKind, Config, EventKind::Create, RecommendedWatcher, RecursiveMode, Watcher,
};
use runtimelib::jupyter::client::JupyterRuntime;
use runtimelib::jupyter::discovery::{
    check_runtime_up, get_jupyter_runtime_instances, is_connection_file,
};
use runtimelib::jupyter::messaging_old::JupyterMessage;
use serde::Serialize;
use sqlx::Pool;
use sqlx::Sqlite;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::runtime::Handle;
use tokio::sync::RwLock;
use tokio::sync::{broadcast, mpsc};
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
    pub async fn new(db: &Pool<Sqlite>) -> RuntimeManager {
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
            manager.insert(runtime).await;
        }
        manager
    }

    /// Get an iterator over all the runtimes in the collection
    pub async fn get_all(&self) -> impl Iterator<Item = RuntimeInstance> {
        self.lock.read().await.clone().into_values()
    }

    /// Get a single runtime by id
    pub async fn get(&self, id: Uuid) -> Option<RuntimeInstance> {
        self.lock.read().await.get(&id).map(|state| state.clone())
    }

    /// 1. Insert the runtime by id into the runtime map
    /// 2. Start a task to watch all messages from the runtime and insert them into the database
    /// 3. Start a task to recieve messages and send time to the runtime
    async fn insert(&self, runtime: JupyterRuntime) {
        let (mpsc_tx, mut mpsc_rx) = mpsc::channel::<JupyterMessage>(1);
        let (broadcast_tx, _) = broadcast::channel::<JupyterMessage>(1);

        // Inser the runtime into the runtime collection
        self.lock.write().await.insert(
            runtime.id,
            RuntimeInstance {
                runtime: runtime.clone(),
                send_tx: mpsc_tx,
                broadcast_tx: broadcast_tx.clone(),
            },
        );

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
                    // As each message comes in on iopub, shove to database
                    if let Ok(message) = client.next_io().await {
                        crate::db::insert_message(&db, id, &message).await;

                        // This should only fail if there are no receivers
                        let _ = broadcast_tx.send(message);
                    } else {
                        // Log error
                        log::error!("Failed to receive message from IOPub");
                    }
                }
            }
        });
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
            if let Some(res) = rx.recv().await {
                match res {
                    Ok(event) => {
                        if let Create(CreateKind::File) = event.kind {
                            for path in event.paths {
                                if is_connection_file(&path) {
                                    log::debug!("New runtime file found {:?}", path);
                                    let runtime = check_runtime_up(path.clone()).await;

                                    match runtime {
                                        Ok(runtime) => {
                                            self.insert(runtime).await;
                                            log::debug!("Connected to runtime {:?}", path);
                                        }
                                        Err(err) => log::error!("Could not load runtime {:?}", err),
                                    };
                                }
                            }
                        }
                    }
                    Err(error) => {
                        log::error!("Error: {error:?}");
                    }
                }
            }
        }
    }
}

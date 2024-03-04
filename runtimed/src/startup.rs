/*
  On start we need to collect all the Jupyter runtimes currently in the system and track new ones.

  With runtimelib, we can detect all the existing Jupyter kernels:

  ```rust
  use runtimelib::jupyter::discovery;

  discovery::get_jupyter_runtime_instances().await;
  ```

*/

use crate::state::RuntimesLock;
use anyhow::Error;
use notify::{
    event::CreateKind, Config, EventKind::Create, RecommendedWatcher, RecursiveMode, Watcher,
};
use runtimelib::jupyter::client::JupyterRuntime;
use runtimelib::jupyter::discovery::{
    check_runtime_up, get_jupyter_runtime_instances, is_connection_file,
};
use sqlx::Pool;
use sqlx::Sqlite;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::runtime::Handle;
use tokio::sync::mpsc::channel;
use tokio::sync::RwLock;
use uuid::Uuid;

/**
 * Wishing for:
 * - What runtime ID did this come from?
 * - What execution did this come from? (likely known with the parent_header.message_id)
 *
 * Note:
 * We could drop any messages that are not outputs or which aren't
 */
pub async fn gather_messages(runtime: JupyterRuntime, db: Pool<Sqlite>) {
    // TODO: This will never timeout and will just sit there watching indefinitely
    if let Ok(mut client) = runtime.attach().await {
        loop {
            // As each message comes in on iopub, shove to database
            if let Ok(message) = client.next_io().await {
                crate::db::insert_message(&db, runtime.id, &message).await;
            } else {
                // Log error
                log::error!("Failed to recieve message from IOPub");
            }
        }
    }
}

/**
* Initialize a `HashMap` of runtimes
* Spawn a thread to watch the runtime folder for new runtimes
* Spawn threads to recieve and record messages for each runtime
*/
pub async fn initialize_runtimes(db: &Pool<Sqlite>) -> RuntimesLock {
    log::debug!("Gathering runtimes");
    let runtimes = get_jupyter_runtime_instances()
        .await
        .into_iter()
        .map(|runtime| (runtime.id, runtime))
        .collect::<HashMap<Uuid, JupyterRuntime>>();

    for (_, runtime) in runtimes.iter() {
        log::debug!("Gathering messages for runtime {}", runtime.id);
        tokio::spawn(gather_messages(runtime.clone(), db.clone()));
    }

    let runtimes = Arc::new(RwLock::new(runtimes));

    tokio::spawn(watch_runtime_dir(runtimes.clone(), db.clone()));

    runtimes
}

pub async fn watch_runtime_dir(runtimes: RuntimesLock, db: Pool<Sqlite>) -> Result<(), Error> {
    let (tx, mut rs) = channel(1);
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
        if let Some(res) = rs.recv().await {
            match res {
                Ok(event) => {
                    if let Create(CreateKind::File) = event.kind {
                        for path in event.paths {
                            if is_connection_file(&path) {
                                log::debug!("New runtime file found {:?}", path);
                                let runtime = check_runtime_up(path.clone()).await;

                                match runtime {
                                    Ok(runtime) => {
                                        tokio::spawn(gather_messages(runtime.clone(), db.clone()));
                                        log::debug!("Connected to runtime {:?}", path);
                                        runtimes.write().await.insert(runtime.id, runtime);
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

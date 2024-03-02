/*
  On start we need to collect all the Jupyter runtimes currently in the system and track new ones.

  With runtimelib, we can detect all the existing Jupyter kernels:

  ```rust
  use runtimelib::jupyter::discovery;

  discovery::get_jupyter_runtime_instances().await;
  ```

*/

use chrono::Utc;
use runtimelib::jupyter::client::JupyterClient;
use sqlx::Sqlite;
use uuid::Uuid;

use crate::AppState;

/**
 * Wishing for:
 * - What runtime ID did this come from?
 * - What execution did this come from? (likely known with the parent_header.message_id)
 *
 * Note:
 * We could drop any messages that are not outputs or which aren't
 */
pub async fn gather_messages(runtime_id: Uuid, mut client: JupyterClient, db: sqlx::Pool<Sqlite>) {
    loop {
        // As each message comes in on iopub, shove to database
        if let Ok(message) = client.next_io().await {
            crate::db::insert_message(&db, runtime_id, message).await;
        } else {
            // Log error
            log::error!("Failed to recieve message from IOPub");
        }
    }
}

pub async fn startup(state: AppState) {
    // Get all the runtimes
    let runtimes = runtimelib::jupyter::discovery::get_jupyter_runtime_instances().await;

    for runtime in runtimes {
        // Runtimes don't necessarily have an ID so we need to either generate one
        // or use the connection file path as the ID
        let runtime_id = Uuid::new_v5(&Uuid::NAMESPACE_URL, runtime.connection_file.as_bytes());

        let client = runtime.clone().attach().await;

        if let Ok(client) = client {
            tokio::spawn(gather_messages(runtime_id, client, state.dbpool.clone()));
        }
    }
}

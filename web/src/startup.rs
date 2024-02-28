/*
  On start we need to collect all the Jupyter runtimes currently in the system and track new ones.

  With runtimelib, we can detect all the existing Jupyter kernels:

  ```rust
  use runtimelib::jupyter::discovery;

  discovery::get_jupyter_runtime_instances().await;
  ```

*/

use runtimelib::jupyter::client::JupyterClient;
use sqlx::Sqlite;

/**
 * Wishing for:
 * - What runtime ID did this come from?
 * - What execution did this come from? (likely known with the parent_header.message_id)
 *
 * Note:
 * We could drop any messages that are not outputs or which aren't
 */
async fn gather_messages(runtime_id: String, client: JupyterClient, db: sqlx::Pool<Sqlite>) {
    loop {
        // As each message comes in on iopub, shove to database
        let message = client.next_io().await;

        if let Ok(message) = message {
            // Database the message
            sqlx::query!(
                r#"INSERT INTO disorganized_messages VALUES($1, $2, $3, $4, $5)"#,
                message.header["msg_id"],
                message.header["msg_type"],
                message.content,
                message.metadata,
                message.parent_header["msg_id"],
                message.parent_header["msg_type"],
            )
        } else {
            // Log error
        }
    }
}

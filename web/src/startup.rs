/*
  On start we need to collect all the Jupyter runtimes currently in the system and track new ones.

  With runtimelib, we can detect all the existing Jupyter kernels:

  ```rust
  use runtimelib::jupyter::discovery;

  discovery::get_jupyter_runtime_instances().await;
  ```

*/

use runtimelib::jupyter::client::Client;
use sqlx::Pool;
use sqlx::Sqlite;

/**
 * Wishing for:
 * - What runtime ID did this come from?
 * - What execution did this come from? (likely known with the parent_header.message_id)
 */
async fn gather_outputs(client: Client, db: sqlx::Pool<Sqlite>) {
    loop {
        // As each message comes in on iopub, shove to database
        let message = client.next_io().await;

        if let Ok(message) = message {
            // Database the message
            sqlx::query!(
                r#"INSERT INTO disorganized_messages VALUES($1, $2, $3, $4, $5)"#,
                message.parent_header.message_id,
                message.id,
                message.message_type,
                message.content,
                message.metadata,
            )
        } else {
            // Log error
        }
    }
}

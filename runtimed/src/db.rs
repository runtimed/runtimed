use chrono::Utc;
use runtimelib::jupyter::messaging::JupyterMessage;
use sqlx::{Pool, Sqlite};
use uuid::Uuid;

pub async fn insert_message(dbpool: &Pool<Sqlite>, runtime_id: Uuid, message: JupyterMessage) {
    let id = Uuid::new_v4();
    let created_at = Utc::now();

    let result = sqlx::query!(
        r#"INSERT INTO disorganized_messages
            (id, msg_id, msg_type, content, metadata, runtime_id, parent_msg_id, parent_msg_type, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"#,
        id,
        message.header["msg_id"],
        message.header["msg_type"],
        message.content,
        message.metadata,
        runtime_id,
        message.parent_header["msg_id"],
        message.parent_header["msg_type"],
        created_at,
    )
    .execute(dbpool)
    .await;

    if let Ok(_) = result {
        // Log success
        log::debug!("Message saved to database: {:?}", message.header["msg_id"]);
    } else {
        // Log error
        log::error!(
            "Failed ot log message to database: {:?}",
            message.header["msg_id"]
        );
    }
}

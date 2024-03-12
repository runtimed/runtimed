use anyhow::Result;
use chrono::Utc;
use runtimelib::jupyter::messaging_old::JupyterMessage;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use uuid::Uuid;

pub async fn insert_message(dbpool: &Pool<Sqlite>, runtime_id: Uuid, message: &JupyterMessage) {
    let id = Uuid::new_v4();
    let created_at = Utc::now();
    let parent_msg_id = message.parent_header["msg_id"].as_str();
    let parent_msg_type = message.parent_header["msg_type"].as_str();
    let msg_id = message.header["msg_id"].as_str();
    let msg_type = message.header["msg_type"].as_str();

    let result = sqlx::query!(
        r#"INSERT INTO disorganized_messages
            (id, msg_id, msg_type, content, metadata, runtime_id, parent_msg_id, parent_msg_type, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"#,
        id,
        msg_id,
        msg_type,
        message.content,
        message.metadata,
        runtime_id,
        parent_msg_id,
        parent_msg_type,
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
            "Failed to log message to database: {:?}",
            message.header["msg_id"]
        );
    }
}

#[derive(Deserialize, Serialize)]
pub struct DbJupyterMessage {
    pub id: Uuid,
    pub msg_id: Option<String>,
    pub msg_type: Option<String>,
    pub metadata: serde_json::Value,
    pub content: serde_json::Value,
    pub created_at: chrono::DateTime<Utc>,
}

pub async fn get_messages_by_parent_id(
    dbpool: &Pool<Sqlite>,
    parent_id: Uuid,
) -> Result<Vec<DbJupyterMessage>, sqlx::Error> {
    let parent_id = parent_id.to_string();
    sqlx::query_as!(
        DbJupyterMessage,
        r#"
        SELECT
            id as "id: uuid::Uuid",
            msg_id,
            msg_type,
            content as "content: serde_json::Value",
            metadata as "metadata: serde_json::Value",
            created_at as "created_at: chrono::DateTime<Utc>"
        FROM disorganized_messages
        WHERE parent_msg_id = $1
        "#,
        parent_id,
    )
    .fetch_all(dbpool)
    .await
}

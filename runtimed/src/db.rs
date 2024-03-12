use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use uuid::Uuid;

use runtimelib::jupyter::message::{Message, MessageLike};

pub async fn insert_message(dbpool: &Pool<Sqlite>, runtime_id: Uuid, message: DbJupyterMessage) {
    let result = sqlx::query!(
        r#"INSERT INTO disorganized_messages
            (id, msg_id, msg_type, content, metadata, runtime_id, parent_msg_id, parent_msg_type, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"#,
        message.id,
        message.msg_id,
        message.msg_type,
        serde_json::to_string(&message.content).unwrap(),
        serde_json::to_string(&message.metadata).unwrap(),
        runtime_id,
        message.parent_msg_id,
        message.parent_msg_type,
        message.created_at,
    )
    .execute(dbpool)
    .await;

    if let Ok(_) = result {
        // Log success
        log::debug!("Message saved to database: {:?}", message.msg_id);
    } else {
        // Log error
        log::error!("Failed to log message to database: {:?}", message.msg_id);
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

// Let's write an into method for converting from Request and Reply into DbJupyterMessage

impl From<dyn MessageLike> for DbJupyterMessage {
    fn from(message_like: dyn MessageLike) -> Self {
        let message: Message = message_like.into();
        let id = Uuid::new_v4();
        let created_at = Utc::now();
        let parent_msg_id = message
            .parent_header()
            .get("msg_id")
            .and_then(|v| v.as_str());
        let parent_msg_type = message
            .parent_header()
            .get("msg_type")
            .and_then(|v| v.as_str());
        let msg_id = message.header().get("msg_id").and_then(|v| v.as_str());
        let msg_type = message.header().get("msg_type").and_then(|v| v.as_str());

        DbJupyterMessage {
            id,
            msg_id: msg_id.map(|s| s.to_string()),
            msg_type: msg_type.map(|s| s.to_string()),
            content: message.content().clone(),
            metadata: message.metadata().clone(),
            created_at,
        }
    }
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

/*
Used building Message<T>, see message.rs.

Ref: https://jupyter-client.readthedocs.io/en/latest/messaging.html#message-header
*/
use bytes::Bytes;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Header {
    pub msg_id: String,
    session: String,
    username: String,
    date: DateTime<Utc>,
    pub msg_type: String,
    version: String,
}

impl Header {
    pub fn new(msg_type: String) -> Self {
        Header {
            msg_id: uuid::Uuid::new_v4().to_string(),
            session: uuid::Uuid::new_v4().to_string(),
            username: "kernel_sidecar".to_string(),
            date: Utc::now(),
            msg_type,
            version: "5.3".to_string(),
        }
    }
}

impl From<Bytes> for Header {
    fn from(bytes: Bytes) -> Self {
        match serde_json::from_slice::<Header>(&bytes) {
            Ok(header) => header,
            Err(e) => {
                // Print the error and the bytes
                eprintln!("Failed to deserialize ZMQ bytes to Header: {}", e);
                eprintln!("Bytes: {:?}", bytes);
                // You can then decide to panic or handle the error differently
                panic!("Deserialization error");
            }
        }
    }
}

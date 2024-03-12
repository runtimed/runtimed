/*
For the most part, Message<T> isn't something you as a developer will use in application code.

For creating requests from client to kernel, the impl From<T> for Request is in the appropriate
message_content files, where Message<T> is used as part of that impl.

For deserialiing responses from kernel to client, the impl From<WireProtocol> for Response creates
the appropriate Message<T> based on the msg_type in the header.

Ref: https://jupyter-client.readthedocs.io/en/latest/messaging.html#a-full-message
*/
use bytes::Bytes;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Header and parent header are used to track the message through the system.
///
/// The `msg_id` is a unique identifier for the message.
/// Ref: https://jupyter-client.readthedocs.io/en/latest/messaging.html#message-header
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Header {
    pub msg_id: String,
    pub session: String,
    pub username: String,
    pub date: DateTime<Utc>,
    pub msg_type: String,
    pub version: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata(serde_json::Value);

impl From<Bytes> for Metadata {
    fn from(bytes: Bytes) -> Self {
        Metadata(serde_json::from_slice(&bytes).expect("Error deserializing metadata"))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Message<T> {
    pub header: Header,
    pub parent_header: Option<Header>,
    pub metadata: Option<Metadata>,
    pub content: T,
}

impl<T> Message<T> {
    pub fn parent_msg_id(&self) -> Option<String> {
        self.parent_header
            .as_ref()
            .map(|header| header.msg_id.to_owned())
    }

    pub fn msg_type(&self) -> String {
        self.header.msg_type.to_owned()
    }
}

pub trait MessageLike {
    fn header(&self) -> &Header;
    fn parent_header(&self) -> &Option<Header>;
    fn metadata(&self) -> &Option<Metadata>;
}

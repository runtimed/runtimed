/*
Used building Message<T>, see message.rs

Ref: https://jupyter-client.readthedocs.io/en/latest/messaging.html#metadata
*/
use bytes::Bytes;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata(serde_json::Value);

impl From<Bytes> for Metadata {
    fn from(bytes: Bytes) -> Self {
        Metadata(serde_json::from_slice(&bytes).expect("Error deserializing metadata"))
    }
}

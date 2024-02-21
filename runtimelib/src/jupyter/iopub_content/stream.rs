/*
https://jupyter-client.readthedocs.io/en/latest/messaging.html#streams-stdout-stderr-etc
*/
use crate::jupyter::schema_tools::list_or_string_to_string;
use bytes::Bytes;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, PartialEq, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum StreamName {
    Stdout,
    Stderr,
}

#[allow(dead_code)]
#[derive(Clone, Serialize, PartialEq, Deserialize, Debug)]
pub struct Stream {
    pub name: StreamName,
    #[serde(deserialize_with = "list_or_string_to_string")]
    pub text: String,
}

impl From<Bytes> for Stream {
    fn from(bytes: Bytes) -> Self {
        serde_json::from_slice(&bytes).expect("Failed to deserialize Stream")
    }
}

/*
https://jupyter-client.readthedocs.io/en/latest/messaging.html#code-inputs
*/

use bytes::Bytes;
use serde::Deserialize;

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct ExecuteInput {
    code: String,
    execution_count: u32,
}

impl From<Bytes> for ExecuteInput {
    fn from(bytes: Bytes) -> Self {
        serde_json::from_slice(&bytes).expect("Failed to deserialize ExecuteInput")
    }
}

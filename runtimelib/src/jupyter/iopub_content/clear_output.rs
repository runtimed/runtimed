/*
https://jupyter-client.readthedocs.io/en/latest/messaging.html#clear-output
*/

use bytes::Bytes;

use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct ClearOutput {
    pub wait: bool,
}

impl From<Bytes> for ClearOutput {
    fn from(bytes: Bytes) -> Self {
        serde_json::from_slice(&bytes).expect("Failed to deserialize ClearOutput")
    }
}

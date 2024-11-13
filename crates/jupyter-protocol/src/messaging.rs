use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JupyterMessage {}

impl JupyterMessage {
    pub fn from_value(_message: Value) -> Result<JupyterMessage, anyhow::Error> {
        Ok(JupyterMessage {})
    }
}

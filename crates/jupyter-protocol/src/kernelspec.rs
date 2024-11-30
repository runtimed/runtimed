use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Contents of a Jupyter JSON kernelspec file
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JupyterKernelspec {
    /// argv must contain `{connection_file}` to be replaced by the client launching the kernel
    /// For example, `["python3", "-m", "ipykernel_launcher", "-f", "{connection_file}"]`
    #[serde(default)]
    pub argv: Vec<String>,
    pub display_name: String,
    pub language: String,
    pub metadata: Option<HashMap<String, Value>>,
    pub interrupt_mode: Option<String>,
    pub env: Option<HashMap<String, String>>,
}

use crate::jupyter_dirs;
use serde::{Deserialize, Serialize};
use tokio::fs;

use std::sync::Arc;

use crate::jupyter::client::Client;
use crate::jupyter::connection_file::ConnectionInfo;

use crate::jupyter::handlers::Handler;
use crate::jupyter::response::Response;

// To execute the above function, it must be called from within an async context, e.g.,
// tokio::runtime::Runtime::new().unwrap().block_on(connect_and_request_kernel_info());
#[derive(Serialize, Clone)]
pub struct JupyterEnvironment {
    process: String,
    argv: Vec<String>,
    display_name: String,
    language: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JupyterRuntime {
    pub connection_info: ConnectionInfo,
    // We'll track the connection file path here as well
    #[serde(skip_deserializing)]
    pub connection_file: String,
}

pub async fn get_jupyter_runtime_instances() -> Vec<JupyterRuntime> {
    let runtime_dir = jupyter_dirs::runtime_dir();
    let mut runtimes = Vec::new();

    if let Ok(mut entries) = fs::read_dir(runtime_dir).await {
        while let Some(entry) = entries.next_entry().await.unwrap_or(None) {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(connection_info) = ConnectionInfo::from_file(&path).await {
                    let runtime = JupyterRuntime {
                        connection_info,
                        connection_file: path.to_str().unwrap_or_default().to_string(),
                    };

                    check_kernel_health(&runtime).await;
                    runtimes.push(runtime);
                }
            }
        }
    }

    runtimes
}

#[derive(Debug)]
struct DebugHandler;

#[async_trait::async_trait]
impl Handler for DebugHandler {
    async fn handle(&mut self, msg: &Response) {
        dbg!(msg);
    }
}

pub async fn check_kernel_health(runtime: &JupyterRuntime) {
    let client = Client::new(&runtime.connection_info).await;

    let handler = DebugHandler {};
    // Wrap the handler in Mutex and then in Arc
    let handlers = vec![Arc::new(tokio::sync::Mutex::new(handler)) as Arc<tokio::sync::Mutex<dyn Handler>>];

    let action = client.kernel_info_request(handlers).await;
    action.await
}

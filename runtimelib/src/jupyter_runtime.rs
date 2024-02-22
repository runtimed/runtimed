use crate::jupyter::shell_content::kernel_info::KernelInfoReply;
use crate::jupyter_dirs;
use serde::{Deserialize, Serialize};
use tokio::fs;

use std::time::Duration;

use crate::jupyter::client::Client;
use crate::jupyter::connection_file::ConnectionInfo;

use anyhow::Error;

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

                    let kr = check_kernel_health(&runtime).await;
                    println!("Kernel Info {:?}", kr);
                    runtimes.push(runtime);
                }
            }
        }
    }

    runtimes
}


pub async fn check_kernel_health(runtime: &JupyterRuntime) -> Result<KernelInfoReply, Error> {
    let client = Client::new(&runtime.connection_info).await;

    let kr = client.request_kernel_info_with_timeout(Duration::from_secs(3)).await;

    kr
}

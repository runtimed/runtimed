use crate::jupyter_dirs;
use serde_json;
use serde_json::from_str;
use serde_json::json;
use serde_json::Value;
use tokio::fs;

use anyhow::Error;

use crate::jupyter::client;

use crate::jupyter::messaging;

pub async fn get_jupyter_runtime_instances() -> Vec<client::JupyterRuntime> {
    let runtime_dir = jupyter_dirs::runtime_dir();
    let mut runtimes = Vec::new();

    if let Ok(mut entries) = fs::read_dir(runtime_dir).await {
        while let Some(entry) = entries.next_entry().await.unwrap_or(None) {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                let content = fs::read_to_string(&path).await.unwrap_or_default();
                if let Ok(mut runtime) = from_str::<client::JupyterRuntime>(&content) {
                    runtime.connection_file = path.to_str().unwrap_or_default().to_string();

                    match check_kernel_info(runtime.clone()).await {
                        Ok(kernel_info) => {
                            runtime.kernel_info = kernel_info;
                            runtime.state = "alive".to_string();
                        }
                        Err(_) => runtime.state = "unresponsive".to_string()
                    }

                    runtimes.push(runtime);
                }
            }
        }
    }

    runtimes
}

pub async fn check_kernel_info(runtime: client::JupyterRuntime) -> Result<Value, Error> {
    let res = tokio::time::timeout(std::time::Duration::from_secs(1), async {
        let mut client = runtime.attach().await;

        let message = messaging::JupyterMessage::new_with_type(
            "kernel_info_request",
            Some(json!({})),
            Some(json!({})),
        );

        let _res = message.send(&mut client.shell).await;

        let reply = messaging::JupyterMessage::read(&mut client.shell).await;

        match reply {
            Ok(msg) => {
                if msg.message_type() == "kernel_info_reply" {
                    Ok(msg.content)
                } else {
                    Err(anyhow::anyhow!("Expected kernel_info_reply, got {}", msg.message_type()))
                }
            },
            Err(e) => {
                println!("Failed to receive kernel info reply: {:?}", e);
                Err(anyhow::anyhow!("Failed to receive kernel info reply: {:?}", e)) // Ensure this arm also returns a Result
            },
        }
    }).await;

   res?
}

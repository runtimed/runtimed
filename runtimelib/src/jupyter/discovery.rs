use crate::jupyter::dirs;
use serde_json;
use serde_json::from_str;
use serde_json::json;
use serde_json::Value;
use tokio::fs;
use tokio::task::JoinSet;
use uuid::Uuid;

use anyhow::{anyhow, Error, Result, Context};

use crate::jupyter::client;

use crate::jupyter::messaging;

pub fn is_connection_file(path: &std::path::Path) -> bool {
    path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json")
}

pub async fn get_jupyter_runtime_instances() -> Vec<client::JupyterRuntime> {
    let runtime_dir = dirs::runtime_dir();

    let mut join_set = JoinSet::new();

    if let Ok(mut entries) = fs::read_dir(runtime_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let connection_file_path = entry.path();
            if is_connection_file(&connection_file_path) {
                join_set.spawn(async move { check_runtime_up(connection_file_path).await });
            }
        }
    }

    let mut runtimes: Vec<client::JupyterRuntime> = Vec::new();
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(Ok(runtime)) => runtimes.push(runtime),
            _ => continue, // Ignore skipped connection files
        }
    }

    runtimes
}

pub async fn load_connection_file(
    connection_file_path: std::path::PathBuf,
) -> Result<client::JupyterRuntime, Error> {
    let content = fs::read_to_string(&connection_file_path)
        .await
        .unwrap_or_default();
    match from_str::<client::JupyterRuntime>(&content) {
        Ok(mut runtime) => {
            runtime.connection_file = connection_file_path
                .to_str()
                .ok_or(anyhow!("Non-unicode runtime file name"))?
                .to_string();
            runtime.id = Uuid::new_v5(&Uuid::NAMESPACE_URL, runtime.connection_file.as_bytes());
            Ok(runtime)
        }
        err => err,
    }.context("Failed to parse JupyterRuntime from file")
}

pub async fn check_runtime_up(
    connection_file_path: std::path::PathBuf,
) -> Result<client::JupyterRuntime, Error> {
    let mut runtime = load_connection_file(connection_file_path).await?;

    match check_kernel_info(runtime.clone()).await {
        Ok(kernel_info) => {
            runtime.kernel_info = kernel_info;
            runtime.state = "alive".to_string();
            Ok(runtime)
        }
        Err(_) => {
            runtime.state = "unresponsive".to_string();
            Ok(runtime)
        }
    }
}

pub async fn check_kernel_info(runtime: client::JupyterRuntime) -> Result<Value, Error> {
    let res = tokio::time::timeout(std::time::Duration::from_secs(1), async {
        let mut client = match runtime.attach().await {
            Ok(client) => client,
            Err(e) => return Err(anyhow::anyhow!("Failed to attach to runtime: {}", e)),
        };

        let message = messaging::JupyterMessage::new("kernel_info_request").with_content(json!({}));

        message.send(&mut client.shell).await?;

        let reply = messaging::JupyterMessage::read(&mut client.shell).await;

        let result = match reply {
            Ok(msg) => {
                if msg.message_type() == "kernel_info_reply" {
                    Ok(msg.content)
                } else {
                    Err(anyhow::anyhow!(
                        "Expected kernel_info_reply, got {}",
                        msg.message_type()
                    ))
                }
            }
            Err(e) => {
                println!("Failed to receive kernel info reply: {:?}", e);
                Err(anyhow::anyhow!(
                    "Failed to receive kernel info reply: {:?}",
                    e
                )) // Ensure this arm also returns a Result
            }
        };

        if let Err(e) = client.detach().await {
            println!("Failed to detach client: {:?}", e);
        }

        result
    })
    .await;

    res?
}

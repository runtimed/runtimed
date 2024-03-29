use crate::jupyter::dirs;
use tokio::fs;
use tokio::task::JoinSet;

use anyhow::{Error, Result};

use crate::jupyter::client::JupyterRuntime;

use crate::messaging::{JupyterMessage, JupyterMessageContent, KernelInfoReply, KernelInfoRequest};

use super::client::{self};

pub fn is_connection_file(path: &std::path::Path) -> bool {
    path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json")
}

pub async fn get_jupyter_runtime_instances() -> Vec<JupyterRuntime> {
    let runtime_dir = dirs::runtime_dir();

    let mut join_set = JoinSet::new();

    if let Ok(mut entries) = fs::read_dir(runtime_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let connection_file_path = entry.path();
            if is_connection_file(&connection_file_path) {
                join_set.spawn(async move {
                    client::JupyterRuntime::from_path_set_state(connection_file_path).await
                });
            }
        }
    }

    let mut runtimes: Vec<JupyterRuntime> = Vec::new();
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(Ok(runtime)) => runtimes.push(runtime),
            _ => continue, // Ignore skipped connection files
        }
    }

    runtimes
}

impl client::JupyterRuntime {
    pub async fn from_path_set_state(
        connection_file_path: std::path::PathBuf,
    ) -> Result<client::JupyterRuntime, Error> {
        let mut runtime = JupyterRuntime::from_path(connection_file_path).await?;
        runtime.set_state().await?;
        Ok(runtime)
    }

    pub async fn set_state(&mut self) -> Result<(), Error> {
        match self.check_kernel_info().await {
            Ok(kernel_info) => {
                self.kernel_info = Some(kernel_info);
                self.state = "alive".to_string();
                Ok(())
            }
            Err(_) => {
                self.state = "unresponsive".to_string();
                Ok(())
            }
        }
    }

    pub async fn check_kernel_info(&self) -> Result<KernelInfoReply, Error> {
        let res = tokio::time::timeout(std::time::Duration::from_secs(1), async {
            let mut client = match self.attach().await {
                Ok(client) => client,
                Err(e) => return Err(anyhow::anyhow!("Failed to attach to runtime: {}", e)),
            };

            let kernel_info_request = KernelInfoRequest {};

            let message: JupyterMessage = kernel_info_request.into();

            message.send(&mut client.shell).await?;

            let reply = JupyterMessage::read(&mut client.shell).await;

            let result = match reply {
                Ok(msg) => {
                    // Check that msg is a kernel_info_reply using the JupyterMessageContent enum
                    if let JupyterMessageContent::KernelInfoReply(kernel_info_reply) = msg.content {
                        Ok(kernel_info_reply)
                    } else {
                        Err(anyhow::anyhow!(
                            "Expected kernel_info_reply, got {}",
                            msg.message_type()
                        ))
                    }
                }
                Err(e) => Err(e),
            };

            if let Err(e) = client.detach().await {
                println!("Failed to detach client: {:?}", e);
            }

            result
        })
        .await;

        res?
    }
}

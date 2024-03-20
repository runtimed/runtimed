use crate::jupyter::dirs;
use serde_json::from_str;

use tokio::fs;
use tokio::task::JoinSet;
use uuid::Uuid;

use anyhow::{anyhow, Context, Error, Result};

use crate::jupyter::client;

use crate::messaging::{JupyterMessage, JupyterMessageContent, KernelInfoReply, KernelInfoRequest};

/// Checks if the given path points to a Jupyter connection file.
///
/// A connection file is identified by having a `.json` file extension.
///
/// # Arguments
///
/// * `path` - A reference to the `Path` that is being checked.
///
/// # Returns
///
/// Returns `true` if the path is a file with a `.json` extension, otherwise `false`.
pub fn is_connection_file(path: &std::path::Path) -> bool {
    path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json")
}

/// Retrieves a list of Jupyter runtime instances from the Jupyter runtime directory.
///
/// Asynchronously reads the runtime directory and checks each connection file to determine
/// if the corresponding runtime is up. Runtimes that are up are collected and returned.
///
/// # Returns
///
/// Returns a `Vec` of `JupyterRuntime` instances that are currently running.
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

/// Loads a Jupyter connection file and constructs a `JupyterRuntime` instance from it.
///
/// # Arguments
///
/// * `connection_file_path` - The path to the Jupyter connection file.
///
/// # Returns
///
/// Returns a `Result` which is either a `JupyterRuntime` instance or an `Error` if the file
/// could not be read or parsed.
pub async fn load_connection_file(
    connection_file_path: std::path::PathBuf,
) -> Result<client::JupyterRuntime, Error> {
    let content = fs::read_to_string(&connection_file_path)
        .await
        .unwrap_or_default();
    from_str::<client::JupyterRuntime>(&content)
        .map_err(Error::from)
        .and_then(|mut runtime| {
            runtime.connection_file = connection_file_path
                .to_str()
                .ok_or_else(|| anyhow!("Non-unicode runtime file name"))?
                .to_string();
            runtime.id = Uuid::new_v5(&Uuid::NAMESPACE_URL, runtime.connection_file.as_bytes());
            Ok(runtime)
        })
        .context("Failed to parse JupyterRuntime from file")
}

/// Checks if the Jupyter runtime corresponding to the connection file is up and updates its state.
///
/// # Arguments
///
/// * `connection_file_path` - The path to the Jupyter connection file.
///
/// # Returns
///
/// Returns a `Result` which is either a `JupyterRuntime` instance with updated state or an `Error`
/// if the file could not be read or the runtime is unresponsive.
pub async fn check_runtime_up(
    connection_file_path: std::path::PathBuf,
) -> Result<client::JupyterRuntime, Error> {
    let mut runtime = load_connection_file(connection_file_path).await?;

    match check_kernel_info(runtime.clone()).await {
        Ok(kernel_info) => {
            runtime.kernel_info = Some(kernel_info);
            runtime.state = "alive".to_string();
            Ok(runtime)
        }
        Err(_) => {
            runtime.state = "unresponsive".to_string();
            Ok(runtime)
        }
    }
}

/// Sends a kernel info request to the Jupyter runtime and awaits the reply.
///
/// # Arguments
///
/// * `runtime` - A `JupyterRuntime` instance to send the request to.
///
/// # Returns
///
/// Returns a `Result` which is either a `KernelInfoReply` or an `Error` if the request
/// fails or the runtime does not respond in time.
pub async fn check_kernel_info(runtime: client::JupyterRuntime) -> Result<KernelInfoReply, Error> {
    let res = tokio::time::timeout(std::time::Duration::from_secs(1), async {
        let mut client = match runtime.attach().await {
            Ok(client) => client,
            Err(e) => return Err(anyhow::anyhow!("Failed to attach to runtime: {}", e)),
        };

        let kernel_info_request = KernelInfoRequest {};

        let message: JupyterMessage = kernel_info_request.into();

        message.send(&mut client.shell).await?;

        let reply = JupyterMessage::read(&mut client.shell).await;

        let result = match reply {
            Ok(msg) => {
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

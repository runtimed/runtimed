//! Methods for discovering Jupyter runtimes on the local machine.
// #[cfg(feature = "tokio-runtime")]
// use tokio::{fs, task::JoinSet, time::timeout};
//
// #[cfg(feature = "tokio-runtime")]
// use anyhow::{Error, Result};

// use crate::jupyter::client::JupyterRuntime;
// #[cfg(feature = "tokio-runtime")]
// use jupyter_serde::messaging::JupyterMessageContent;

/// Check if a path looks like a connection file.
///
/// Currently this only checks that it is both a file and has a `.json` extension.
pub fn is_connection_file(path: &std::path::Path) -> bool {
    path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json")
}

// /// Get a list of all Jupyter runtimes on the local machine.
// ///
// /// This reads connection files from Jupyter runtime directories from the `dirs` module.
// #[cfg(feature = "tokio-runtime")]
// pub async fn get_jupyter_runtime_instances() -> Vec<JupyterRuntime> {
//     let runtime_dir = crate::jupyter::dirs::runtime_dir();

//     let mut join_set = JoinSet::new();

//     if let Ok(mut entries) = fs::read_dir(runtime_dir).await {
//         while let Ok(Some(entry)) = entries.next_entry().await {
//             let connection_file_path = entry.path();
//             if is_connection_file(&connection_file_path) {
//                 join_set.spawn(async move {
//                     JupyterRuntime::from_path_set_state(connection_file_path).await
//                 });
//             }
//         }
//     }

//     let mut runtimes: Vec<JupyterRuntime> = Vec::new();
//     while let Some(result) = join_set.join_next().await {
//         match result {
//             Ok(Ok(runtime)) => runtimes.push(runtime),
//             _ => continue, // Ignore skipped connection files
//         }
//     }

//     runtimes
// }

// #[cfg(feature = "tokio-runtime")]
// impl JupyterRuntime {
//     /// Read a connection file from disk and parse it into a JupyterRuntime object,
//     /// and set the state of the runtime by attempting to connect to the underlying kernel.
//     // pub async fn from_path_set_state(
//     //     connection_file_path: std::path::PathBuf,
//     // ) -> Result<JupyterRuntime, Error> {
//     //     let mut runtime = JupyterRuntime::from_path(connection_file_path).await?;
//     //     runtime.set_state().await?;
//     //     Ok(runtime)
//     // }

//     // /// Update the state of the runtime by attempting to connect to the underlying kernel.
//     // pub async fn set_state(&mut self) -> Result<(), Error> {
//     //     match self.check_kernel_info().await {
//     //         Ok(kernel_info) => {
//     //             self.kernel_info = Some(kernel_info);
//     //             self.state = "alive".to_string();
//     //             Ok(())
//     //         }
//     //         Err(_) => {
//     //             self.state = "unresponsive".to_string();
//     //             Ok(())
//     //         }
//     //     }
//     // }

//     // /// Send a message to the kernel to check its status.
//     // pub async fn check_kernel_info(&self) -> Result<Box<KernelInfoReply>, Error> {
//     //     let res = timeout(std::time::Duration::from_secs(1), async {
//     //         let mut client = match self.attach().await {
//     //             Ok(client) => client,
//     //             Err(e) => return Err(anyhow::anyhow!("Failed to attach to runtime: {}", e)),
//     //         };

//     //         todo!();

//     //         // let kernel_info_request = KernelInfoRequest {};

//     //         // let message: JupyterMessage = kernel_info_request.into();

//     //         // let reply = client.send(message).await;

//     //         // let result = match reply {
//     //         //     Ok(msg) => {
//     //         //         // Check that msg is a kernel_info_reply using the JupyterMessageContent enum
//     //         //         if let JupyterMessageContent::KernelInfoReply(kernel_info_reply) = msg.content {
//     //         //             Ok(kernel_info_reply)
//     //         //         } else {
//     //         //             Err(anyhow::anyhow!(
//     //         //                 "Expected kernel_info_reply, got {}",
//     //         //                 msg.message_type()
//     //         //             ))
//     //         //         }
//     //         //     }
//     //         //     Err(e) => Err(e),
//     //         // };

//     //         // if let Err(e) = client.detach().await {
//     //         //     println!("Failed to detach client: {:?}", e);
//     //         // }

//     //         // result
//     //     })
//     //     .await;

//     //     res?
//     // }
// }

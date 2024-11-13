```rust
use std::env;
use std::process::exit;

use crate::client::JupyterClient;

use anyhow::Result;
use futures::{SinkExt as _, StreamExt as _};
use runtimelib::ExecutionState;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <jupyter-server-url>", args[0]);
        exit(1);
    }

    let jupyter_client = JupyterClient::from_url(&args[1])?;
    println!("Created JupyterClient instance");

    // List available kernel specs
    let kernel_specs = jupyter_client.list_kernel_specs().await?;
    println!("Available kernel specs:");
    for (name, spec) in kernel_specs.kernelspecs {
        println!("  - {}: {}", name, spec.spec.display_name);
    }
    println!("Default kernel spec: {}", kernel_specs.default);

    // List current kernels
    let kernels = jupyter_client.list_kernels().await?;
    println!("\nCurrent kernels:");
    for kernel in kernels {
        println!(
            "  - ID: {}, Name: {}, State: {}",
            kernel.id, kernel.name, kernel.execution_state
        );
    }

    // List current sessions
    let sessions = jupyter_client.list_sessions().await?;
    println!("\nCurrent sessions:");
    for session in sessions {
        println!(
            "  - ID: {}, Name: {}, Path: {}",
            session.id, session.name, session.path
        );
    }

    let kernel_id = jupyter_client.start_kernel("python").await?;
    println!("Started Jupyter kernel");

    let jupyter_websocket = jupyter_client.connect_to_kernel(&kernel_id).await?;
    // Can `.send` and `.next` on the websocket, or split into
    // separate sender and receiver halves.

    let (mut w, mut r) = jupyter_websocket.split();

    w.send(runtimelib::KernelInfoRequest {}.into()).await?;

    while let Some(response) = r.next().await.transpose()? {
        match response.content {
            runtimelib::JupyterMessageContent::KernelInfoReply(kernel_info_reply) => {
                println!("Received kernel_info_reply");
                println!("{:?}", kernel_info_reply);
                break;
            }
            other => {
                println!("Received");
                println!("{:?}", other);
            }
        }
    }

    w.send(
        runtimelib::ExecuteRequest {
            code: "print('Hello, world!')".to_string(),
            silent: false,
            store_history: true,
            user_expressions: Default::default(),
            allow_stdin: false,
            stop_on_error: true,
        }
        .into(),
    )
    .await?;

    while let Some(response) = r.next().await.transpose()? {
        match response.content {
            runtimelib::JupyterMessageContent::Status(status) => {
                println!("Received status");
                println!("{:?}", status);

                if status.execution_state == ExecutionState::Idle {
                    break;
                }
            }
            other => {
                println!("Received");
                println!("{:?}", other);
            }
        }
    }

    jupyter_client.shutdown(&kernel_id).await?;

    println!("Kernel shut down successfully");

    Ok(())
}
```

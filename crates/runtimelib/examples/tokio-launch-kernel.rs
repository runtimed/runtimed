use std::net::{IpAddr, Ipv4Addr};

use runtimelib::jupyter::KernelspecDir;
use runtimelib::messaging::{ExecuteRequest, JupyterMessage};
use runtimelib::{dirs, peek_ports, ConnectionInfo, ExecutionState, JupyterMessageContent};

use uuid::Uuid;

#[cfg(feature = "async-std-runtime")]
fn main() -> anyhow::Result<()> {
    // todo: Show example using async-std-runtime
    // For now, check out for something similar https://github.com/runtimed/smoke/blob/main/src/main.rs
    anyhow::Ok(())
}

#[cfg(feature = "tokio-runtime")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let kernel_specification = KernelspecDir::new(&"python".to_string()).await?;

    let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
    let ports = peek_ports(ip, 5).await?;
    assert_eq!(ports.len(), 5);

    let connection_info = ConnectionInfo {
        transport: "tcp".to_string(),
        ip: ip.to_string(),
        stdin_port: ports[0],
        control_port: ports[1],
        hb_port: ports[2],
        shell_port: ports[3],
        iopub_port: ports[4],
        signature_scheme: "hmac-sha256".to_string(),
        key: uuid::Uuid::new_v4().to_string(),
        kernel_name: Some(format!("zed-{}", kernel_specification.kernel_name)),
    };

    let runtime_dir = dirs::runtime_dir();
    tokio::fs::create_dir_all(&runtime_dir).await.map_err(|e| {
        anyhow::anyhow!(
            "Failed to create jupyter runtime dir {:?}: {}",
            runtime_dir,
            e
        )
    })?;

    let connection_path = runtime_dir.join(format!("kernel-example.json"));
    let content = serde_json::to_string(&connection_info)?;
    tokio::fs::write(connection_path.clone(), content).await?;

    let mut cmd = kernel_specification.command(&connection_path, None, None)?;

    let working_directory = "/tmp";

    let mut process = cmd
        .current_dir(&working_directory)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .stdin(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;

    let session_id = Uuid::new_v4().to_string();

    // Listen for display data, execute result, stdout messages, etc.
    let mut iopub_socket =
        runtimelib::create_client_iopub_connection(&connection_info, "", &session_id).await?;
    let mut shell_socket =
        runtimelib::create_client_shell_connection(&connection_info, &session_id).await?;
    // let mut control_socket =
    //     runtimelib::create_client_control_connection(&connection_info, &session_id).await?;

    let execute_request = ExecuteRequest::new("print('Hello, World!')".to_string());
    let execute_request: JupyterMessage = execute_request.into();

    let execute_request_id = execute_request.header.msg_id.clone();

    let iopub_handle = tokio::spawn({
        async move {
            loop {
                match iopub_socket.read().await {
                    Ok(message) => match message.content {
                        JupyterMessageContent::Status(status) => {
                            //
                            if status.execution_state == ExecutionState::Idle
                                && message.parent_header.as_ref().map(|h| h.msg_id.as_str())
                                    == Some(execute_request_id.as_str())
                            {
                                println!("Execution finalized, exiting...");
                                break;
                            }
                        }
                        _ => {
                            println!("{:?}", message.content);
                        }
                    },
                    Err(e) => {
                        eprintln!("Error receiving iopub message: {}", e);
                        break;
                    }
                }
            }
        }
    });

    shell_socket.send(execute_request).await?;

    iopub_handle.await?;

    process.start_kill()?;

    Ok(())
}

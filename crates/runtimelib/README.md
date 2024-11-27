# runtimelib

`runtimelib` is a Rust library for interacting with Jupyter kernels and managing interactive computing environments. It provides a set of tools and abstractions to simplify the process of working with various programming language runtimes, enabling developers to build powerful applications that leverage the capabilities of Jupyter kernels.

## Introduction

runtimelib serves as the foundation for building interactive computing applications, REPLs, and notebook-like interfaces. It abstracts away the complexities of communicating with Jupyter kernels, allowing developers to focus on creating rich, interactive experiences for users.

Key features of runtimelib include:

- Easy integration with Jupyter kernels
- Asynchronous communication with kernels
- Support for multiple runtime environments
- Extensible architecture for custom kernel implementations

Whether you're building a new notebook application, creating a specialized REPL, or integrating interactive computing capabilities into your existing projects, runtimelib provides the tools and flexibility you need to get started quickly and efficiently.


## Installation

Runtimelib allows you to pick which async runtime you want to use. If you're using tokio, include the `tokio-runtime` flag. For `async-dispatcher` users (AKA GPUI devs), use `async-dispatcher-runtime`. The async dispatcher runtime is also compatible for smol/async-std users.

### Tokio Users

```toml
[dependencies]
runtimelib = { version = "0.22.0", features = ["tokio-runtime"] }
```

### Async-dispatcher Users

```toml
[dependencies]
runtimelib = { version = "0.22.0", features = ["async-dispatcher-runtime"] }
```

## Key Features

- **Jupyter Kernel Management**: Discover, start, and manage Jupyter kernels.
- **Messaging Protocol**: Implement the Jupyter messaging protocol for communication with kernels.
- **Runtime Management**: Create and manage runtime instances for interactive computing.
- **Flexible Async Runtime**: Support for both Tokio and async-dispatcher runtimes.
- **Media Handling**: Work with various media types used in Jupyter, including images, HTML, and more.

## Usage Example

Here's an example of how to use Runtimelib to start a Jupyter kernel and execute some code:

```rust
use std::net::{IpAddr, Ipv4Addr};

use runtimelib::jupyter::KernelspecDir;
use runtimelib::messaging::{ExecuteRequest, JupyterMessage};
use runtimelib::{dirs, peek_ports, ConnectionInfo, ExecutionState, JupyterMessageContent};

use uuid::Uuid;

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
```

This example demonstrates how to start a Python kernel, attach to it, and execute a simple Python command.

## Documentation

For more detailed information about the API and its usage, please refer to the [API documentation](https://docs.rs/runtimelib).

## Contributing

We welcome contributions to RuntimeLib! If you'd like to contribute, please:

1. Fork the repository
2. Create a new branch for your feature or bug fix
3. Write tests for your changes
4. Implement your changes
5. Submit a pull request

Please make sure to update tests as appropriate and adhere to the existing coding style.

## License

RuntimeLib is distributed under the terms of both the MIT license and the Apache License (Version 2.0). See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT) for details.

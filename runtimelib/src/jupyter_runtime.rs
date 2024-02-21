use crate::jupyter_dirs;
use serde::{Deserialize, Serialize};
use serde_json::from_str;
use tokio::fs;

use zeromq;
use zeromq::Socket;

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
    pub shell_port: u16,
    pub iopub_port: u16,
    pub stdin_port: u16,
    pub control_port: u16,
    pub hb_port: u16,
    pub kernel_name: String,
    pub ip: String,
    key: String,
    pub transport: String, // TODO: Enumify with tcp, ipc
    signature_scheme: String,
    // We'll track the connection file path here as well
    #[serde(skip_deserializing)]
    pub connection_file: String,
}

impl JupyterRuntime {
    pub async fn attach(self) -> JupyterClient {
        let iopub_socket = zeromq::SubSocket::new();
        let mut iopub_connection = jupyter_msg::Connection::new(iopub_socket, &self.key);
        iopub_connection
            .socket
            .connect(&format!(
                "{}://{}:{}",
                self.transport, self.ip, self.iopub_port
            ))
            .await
            .unwrap();

        let shell_socket = zeromq::DealerSocket::new();
        let mut shell_connection = jupyter_msg::Connection::new(shell_socket, &self.key);
        shell_connection
            .socket
            .connect(&format!(
                "{}://{}:{}",
                self.transport, self.ip, self.shell_port
            ))
            .await
            .unwrap();

        let stdin_socket = zeromq::DealerSocket::new();
        let mut stdin_connection = jupyter_msg::Connection::new(stdin_socket, &self.key);
        stdin_connection
            .socket
            .connect(&format!(
                "{}://{}:{}",
                self.transport, self.ip, self.stdin_port
            ))
            .await
            .unwrap();

        let control_socket = zeromq::DealerSocket::new();
        let mut control_connection = jupyter_msg::Connection::new(control_socket, &self.key);
        control_connection
            .socket
            .connect(&format!(
                "{}://{}:{}",
                self.transport, self.ip, self.control_port
            ))
            .await
            .unwrap();

        let heartbeat_socket = zeromq::ReqSocket::new();
        let mut heartbeat_connection = jupyter_msg::Connection::new(heartbeat_socket, &self.key);
        heartbeat_connection
            .socket
            .connect(&format!(
                "{}://{}:{}",
                self.transport, self.ip, self.hb_port
            ))
            .await
            .unwrap();

        return JupyterClient {
            iopub: iopub_connection,
            shell: shell_connection,
            stdin: stdin_connection,
            control: control_connection,
            heartbeat: heartbeat_connection,
        };
    }
}

pub struct JupyterClient {
    shell: jupyter_msg::Connection<zeromq::DealerSocket>,
    iopub: jupyter_msg::Connection<zeromq::SubSocket>,
    stdin: jupyter_msg::Connection<zeromq::DealerSocket>,
    control: jupyter_msg::Connection<zeromq::DealerSocket>,
    heartbeat: jupyter_msg::Connection<zeromq::ReqSocket>,
}

pub async fn get_jupyter_runtime_instances() -> Vec<JupyterRuntime> {
    let runtime_dir = jupyter_dirs::runtime_dir();
    let mut runtimes = Vec::new();

    if let Ok(mut entries) = fs::read_dir(runtime_dir).await {
        while let Some(entry) = entries.next_entry().await.unwrap_or(None) {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                let content = fs::read_to_string(&path).await.unwrap_or_default();
                if let Ok(mut runtime) = from_str::<JupyterRuntime>(&content) {
                    runtime.connection_file = path.to_str().unwrap_or_default().to_string();
                    runtimes.push(runtime);
                }
            }
        }
    }

    runtimes
}

pub async fn attach_to_runtime(runtime: JupyterRuntime) -> JupyterClient {
    let jc = runtime.attach().await;

    jc
}

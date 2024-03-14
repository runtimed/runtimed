use std::collections::HashMap;

use crate::messaging::{Connection, ExecuteRequest, JupyterMessage, KernelInfoReply};
use tokio::time::{timeout, Duration};

use serde::{Deserialize, Serialize};

use uuid::Uuid;
use zeromq;
use zeromq::Socket;

use anyhow::anyhow;
use anyhow::Error;

#[derive(Serialize, Clone)]
pub struct JupyterEnvironment {
    process: String,
    argv: Vec<String>,
    display_name: String,
    language: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JupyterRuntime {
    #[serde(default)]
    pub id: Uuid,
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
    #[serde(default)]
    pub connection_file: String,
    #[serde(default)]
    pub state: String, // TODO: Use an enum
    pub kernel_info: Option<KernelInfoReply>,
}

impl JupyterRuntime {
    pub async fn attach(&self) -> Result<JupyterClient, Error> {
        let mut iopub_socket = zeromq::SubSocket::new();
        match iopub_socket.subscribe("").await {
            Ok(_) => (),
            Err(e) => return Err(anyhow!("Error subscribing to iopub: {}", e)),
        }

        let mut iopub_connection = Connection::new(iopub_socket, &self.key);
        iopub_connection
            .socket
            .connect(&format!(
                "{}://{}:{}",
                self.transport, self.ip, self.iopub_port
            ))
            .await
            .unwrap();

        let shell_socket = zeromq::DealerSocket::new();
        let mut shell_connection = Connection::new(shell_socket, &self.key);
        shell_connection
            .socket
            .connect(&format!(
                "{}://{}:{}",
                self.transport, self.ip, self.shell_port
            ))
            .await
            .unwrap();

        let stdin_socket = zeromq::DealerSocket::new();
        let mut stdin_connection = Connection::new(stdin_socket, &self.key);
        stdin_connection
            .socket
            .connect(&format!(
                "{}://{}:{}",
                self.transport, self.ip, self.stdin_port
            ))
            .await
            .unwrap();

        let control_socket = zeromq::DealerSocket::new();
        let mut control_connection = Connection::new(control_socket, &self.key);
        control_connection
            .socket
            .connect(&format!(
                "{}://{}:{}",
                self.transport, self.ip, self.control_port
            ))
            .await
            .unwrap();

        let heartbeat_socket = zeromq::ReqSocket::new();
        let mut heartbeat_connection = Connection::new(heartbeat_socket, &self.key);
        heartbeat_connection
            .socket
            .connect(&format!(
                "{}://{}:{}",
                self.transport, self.ip, self.hb_port
            ))
            .await
            .unwrap();

        Ok(JupyterClient {
            iopub: iopub_connection,
            shell: shell_connection,
            stdin: stdin_connection,
            control: control_connection,
            heartbeat: heartbeat_connection,
        })
    }
}

pub struct JupyterClient {
    pub(crate) shell: Connection<zeromq::DealerSocket>,
    pub(crate) iopub: Connection<zeromq::SubSocket>,
    pub(crate) stdin: Connection<zeromq::DealerSocket>,
    pub(crate) control: Connection<zeromq::DealerSocket>,
    pub(crate) heartbeat: Connection<zeromq::ReqSocket>,
}

impl JupyterClient {
    pub async fn detach(self) -> Result<(), Error> {
        let timeout_duration = Duration::from_millis(60);

        let close_sockets = async {
            let _ = tokio::join!(
                self.shell.socket.close(),
                self.iopub.socket.close(),
                self.stdin.socket.close(),
                self.control.socket.close(),
                self.heartbeat.socket.close(),
            );
        };

        match timeout(timeout_duration, close_sockets).await {
            Ok(_) => Ok(()),
            Err(_) => Err(anyhow!("Timeout reached while closing sockets.")),
        }
    }

    pub async fn send(&mut self, message: JupyterMessage) -> Result<JupyterMessage, Error> {
        message.send(&mut self.shell).await?;
        let response = JupyterMessage::read(&mut self.shell).await?;
        Ok(response)
    }

    pub async fn next_io(&mut self) -> Result<JupyterMessage, Error> {
        JupyterMessage::read(&mut self.iopub).await
    }
}

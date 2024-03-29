use crate::messaging::{Connection, JupyterMessage, KernelInfoReply};
use tokio::fs;
use tokio::time::{timeout, Duration};

use serde::{Deserialize, Serialize};
use serde_json;

use uuid::Uuid;
use zeromq;
use zeromq::Socket;

use anyhow::anyhow;
use anyhow::{Context, Result};
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConnectionInfo {
    pub ip: String,
    pub transport: String,
    pub shell_port: u16,
    pub iopub_port: u16,
    pub stdin_port: u16,
    pub control_port: u16,
    pub hb_port: u16,
    pub key: String,
    pub signature_scheme: String,
    pub kernel_name: String,
}

impl ConnectionInfo {
    pub async fn from_path(connection_file_path: &std::path::PathBuf) -> Result<ConnectionInfo> {
        let content = fs::read_to_string(&connection_file_path)
            .await
            .unwrap_or_default();

        serde_json::from_str::<ConnectionInfo>(&content).context("Failed to parse connection file")
    }

    pub fn iopub_url(&self) -> String {
        format!("{}://{}:{}", self.transport, self.ip, self.iopub_port)
    }

    pub fn shell_url(&self) -> String {
        format!("{}://{}:{}", self.transport, self.ip, self.shell_port)
    }

    pub fn stdin_url(&self) -> String {
        format!("{}://{}:{}", self.transport, self.ip, self.stdin_port)
    }

    pub fn control_url(&self) -> String {
        format!("{}://{}:{}", self.transport, self.ip, self.control_port)
    }

    pub fn hb_url(&self) -> String {
        format!("{}://{}:{}", self.transport, self.ip, self.hb_port)
    }
}

// This thing can't be cloned, serialized, or deserialized because of the process
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JupyterRuntime {
    pub connection_info: ConnectionInfo,
    pub connection_file: PathBuf,
    pub id: Uuid,
    // TODO: create an enum for activity state
    pub state: String,
    pub kernel_info: Option<KernelInfoReply>,
}

impl JupyterRuntime {
    pub async fn from_path(connection_file: PathBuf) -> Result<Self> {
        let connection_info = ConnectionInfo::from_path(&connection_file).await?;
        Ok(Self::new(connection_info, connection_file))
    }
    pub fn new(connection_info: ConnectionInfo, connection_file: PathBuf) -> Self {
        // TODO evaluate UUID generation, should we also use connection info contents?
        let id = Uuid::new_v5(&Uuid::NAMESPACE_URL, connection_file.as_os_str().as_bytes());
        Self {
            connection_info,
            connection_file,
            id,
            state: "idle".to_string(),
            kernel_info: None,
        }
    }

    pub async fn attach(&self) -> Result<JupyterClient> {
        let mut iopub_socket = zeromq::SubSocket::new();
        match iopub_socket.subscribe("").await {
            Ok(_) => (),
            Err(e) => return Err(anyhow!("Error subscribing to iopub: {}", e)),
        }

        let mut iopub_connection = Connection::new(iopub_socket, &self.connection_info.key);
        iopub_connection
            .socket
            .connect(&self.connection_info.iopub_url())
            .await
            .unwrap();

        let shell_socket = zeromq::DealerSocket::new();
        let mut shell_connection = Connection::new(shell_socket, &self.connection_info.key);
        shell_connection
            .socket
            .connect(&self.connection_info.shell_url())
            .await
            .unwrap();

        let stdin_socket = zeromq::DealerSocket::new();
        let mut stdin_connection = Connection::new(stdin_socket, &self.connection_info.key);
        stdin_connection
            .socket
            .connect(&self.connection_info.stdin_url())
            .await
            .unwrap();

        let control_socket = zeromq::DealerSocket::new();
        let mut control_connection = Connection::new(control_socket, &self.connection_info.key);
        control_connection
            .socket
            .connect(&self.connection_info.control_url())
            .await
            .unwrap();

        let heartbeat_socket = zeromq::ReqSocket::new();
        let mut heartbeat_connection = Connection::new(heartbeat_socket, &self.connection_info.key);
        heartbeat_connection
            .socket
            .connect(&self.connection_info.hb_url())
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
    pub async fn detach(self) -> Result<()> {
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

    pub async fn send(&mut self, message: JupyterMessage) -> Result<JupyterMessage> {
        message.send(&mut self.shell).await?;
        let response = JupyterMessage::read(&mut self.shell).await?;
        Ok(response)
    }

    pub async fn next_io(&mut self) -> Result<JupyterMessage> {
        JupyterMessage::read(&mut self.iopub).await
    }
}

//! Interfacing and connecting with Jupyter kernels
//!
//! This module provides structures for understanding the connection information,
//! existing jupyter runtimes, and a client with ZeroMQ sockets to
//! communicate with the kernels.

use crate::jupyter::dirs;
use crate::messaging::{
    ClientControlConnection, ClientHeartbeatConnection, ClientIoPubConnection,
    ClientShellConnection, ClientStdinConnection, Connection, JupyterMessage,
    KernelControlConnection, KernelHeartbeatConnection, KernelIoPubConnection,
    KernelShellConnection, KernelStdinConnection,
};

#[cfg(feature = "tokio-runtime")]
use tokio::{fs, net::TcpListener};

#[cfg(feature = "async-dispatcher-runtime")]
use async_std::{fs, net::TcpListener};

use serde::{Deserialize, Serialize};
use serde_json;

use rand::{distributions::Alphanumeric, Rng};
use uuid::Uuid;
use zeromq;
use zeromq::Socket;

use anyhow::anyhow;
use anyhow::{Context, Result};
use std::fmt::{Display, Formatter};
use std::net::{IpAddr, SocketAddr};

#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;

#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;

use std::path::PathBuf;

/// Connection information for a Jupyter kernel, as represented in a
/// JSON connection file.
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
    // Ignore if not present
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kernel_name: Option<String>,
}

impl ConnectionInfo {
    pub async fn from_peeking_ports(ip: &str, kernel_name: &str) -> Result<Self> {
        let ip = ip.to_string();
        let addr: IpAddr = ip.parse()?;
        let transport: String = "tcp".into(); // TODO: make this configurable?
        let ports = Self::peek_ports(addr, 5).await?;
        Ok(Self {
            ip,
            transport,
            shell_port: ports[0],
            iopub_port: ports[1],
            stdin_port: ports[2],
            control_port: ports[3],
            hb_port: ports[4],
            key: Self::jupyter_style_key(),
            signature_scheme: String::from("hmac-sha256"),
            kernel_name: Some(String::from(kernel_name)),
        })
    }

    /// Generate a random key in the style of Jupyter: "AAAAAAAA-AAAAAAAAAAAAAAAAAAAAAAAA"
    /// (A comment in the Python source indicates the author intended a dash
    /// every 8 characters, but only actually does it for the first chunk)
    fn jupyter_style_key() -> String {
        let a: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(8)
            .map(char::from)
            .collect();
        let b: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(24)
            .map(char::from)
            .collect();
        format!("{}-{}", a, b,)
    }

    /// Private function for finding a set of open ports. This function creates a listener with the port set to 0.
    /// The listener is closed at the end of the function when the listener goes out of scope.
    ///
    /// This of course opens a race condition in between closing the port and usage by a kernel,
    /// but it is inherent to the design of the Jupyter protocol.
    async fn peek_ports(ip: IpAddr, num: usize) -> Result<Vec<u16>> {
        let mut addr_zeroport: SocketAddr = SocketAddr::new(ip, 0);
        addr_zeroport.set_port(0);

        let mut ports: Vec<u16> = Vec::new();
        for _ in 0..num {
            let listener = TcpListener::bind(addr_zeroport).await?;
            let bound_port = listener.local_addr()?.port();
            ports.push(bound_port);
        }
        Ok(ports)
    }

    pub fn generate_file_path(&self) -> PathBuf {
        let kernel_fs_uuid = Uuid::new_v4();
        let connection_file_path: PathBuf =
            dirs::runtime_dir().join(format!("kernel-{}.json", kernel_fs_uuid));
        connection_file_path
    }

    /// Write the connection info to a file on disk inside dirs::runtime_dir()
    pub async fn write(&self, connection_file_path: &PathBuf) -> Result<PathBuf> {
        let content = serde_json::to_string_pretty(&self)?;
        fs::write(&connection_file_path, content).await?;
        Ok(PathBuf::from(connection_file_path))
    }

    /// Read a connection file from disk and parse it into a ConnectionInfo object
    pub async fn from_path(connection_file_path: &std::path::PathBuf) -> Result<ConnectionInfo> {
        let content = fs::read_to_string(&connection_file_path)
            .await
            .unwrap_or_default();

        serde_json::from_str::<ConnectionInfo>(&content).context("Failed to parse connection file")
    }

    /// format the iopub url for a ZeroMQ connection
    pub fn iopub_url(&self) -> String {
        format!("{}://{}:{}", self.transport, self.ip, self.iopub_port)
    }

    /// format the shell url for a ZeroMQ connection
    pub fn shell_url(&self) -> String {
        format!("{}://{}:{}", self.transport, self.ip, self.shell_port)
    }

    /// format the stdin url for a ZeroMQ connection
    pub fn stdin_url(&self) -> String {
        format!("{}://{}:{}", self.transport, self.ip, self.stdin_port)
    }

    /// format the control url for a ZeroMQ connection
    pub fn control_url(&self) -> String {
        format!("{}://{}:{}", self.transport, self.ip, self.control_port)
    }

    /// format the heartbeat url for a ZeroMQ connection
    pub fn hb_url(&self) -> String {
        format!("{}://{}:{}", self.transport, self.ip, self.hb_port)
    }

    pub async fn create_kernel_iopub_connection(
        &self,
        session_id: &str,
    ) -> anyhow::Result<KernelIoPubConnection> {
        let endpoint = self.iopub_url();

        let mut socket = zeromq::PubSocket::new();
        socket.bind(&endpoint).await?;
        anyhow::Ok(Connection::new(socket, &self.key, session_id))
    }

    pub async fn create_kernel_shell_connection(
        &self,
        session_id: &str,
    ) -> anyhow::Result<KernelShellConnection> {
        let endpoint = self.shell_url();

        let mut socket = zeromq::RouterSocket::new();
        socket.bind(&endpoint).await?;
        anyhow::Ok(Connection::new(socket, &self.key, session_id))
    }

    pub async fn create_kernel_control_connection(
        &self,
        session_id: &str,
    ) -> anyhow::Result<KernelControlConnection> {
        let endpoint = self.control_url();

        let mut socket = zeromq::RouterSocket::new();
        socket.bind(&endpoint).await?;
        anyhow::Ok(Connection::new(socket, &self.key, session_id))
    }

    pub async fn create_kernel_stdin_connection(
        &self,
        session_id: &str,
    ) -> anyhow::Result<KernelStdinConnection> {
        let endpoint = self.stdin_url();

        let mut socket = zeromq::RouterSocket::new();
        socket.bind(&endpoint).await?;
        anyhow::Ok(Connection::new(socket, &self.key, session_id))
    }

    pub async fn create_kernel_heartbeat_connection(
        &self,
    ) -> anyhow::Result<KernelHeartbeatConnection> {
        let endpoint = self.hb_url();

        let mut socket = zeromq::RepSocket::new();
        socket.bind(&endpoint).await?;
        anyhow::Ok(KernelHeartbeatConnection { socket })
    }

    pub async fn create_client_iopub_connection(
        &self,
        topic: &str,
        session_id: &str,
    ) -> anyhow::Result<ClientIoPubConnection> {
        let endpoint = self.iopub_url();

        let mut socket = zeromq::SubSocket::new();
        socket.subscribe(topic).await?;

        socket.connect(&endpoint).await?;
        anyhow::Ok(Connection::new(socket, &self.key, session_id))
    }

    pub async fn create_client_shell_connection(
        &self,
        session_id: &str,
    ) -> anyhow::Result<ClientShellConnection> {
        let endpoint = self.shell_url();

        let mut socket = zeromq::DealerSocket::new();
        socket.connect(&endpoint).await?;
        anyhow::Ok(Connection::new(socket, &self.key, session_id))
    }

    pub async fn create_client_control_connection(
        &self,
        session_id: &str,
    ) -> anyhow::Result<ClientControlConnection> {
        let endpoint = self.control_url();

        let mut socket = zeromq::DealerSocket::new();
        socket.connect(&endpoint).await?;
        anyhow::Ok(Connection::new(socket, &self.key, session_id))
    }

    pub async fn create_client_stdin_connection(
        &self,
        session_id: &str,
    ) -> anyhow::Result<ClientStdinConnection> {
        let endpoint = self.stdin_url();

        let mut socket = zeromq::DealerSocket::new();
        socket.connect(&endpoint).await?;
        anyhow::Ok(Connection::new(socket, &self.key, session_id))
    }

    pub async fn create_client_heartbeat_connection(
        &self,
    ) -> anyhow::Result<ClientHeartbeatConnection> {
        let endpoint = self.hb_url();

        let mut socket = zeromq::ReqSocket::new();
        socket.connect(&endpoint).await?;
        anyhow::Ok(ClientHeartbeatConnection { socket })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash, Copy, PartialOrd)]
pub struct RuntimeId(pub Uuid);

impl RuntimeId {
    pub fn new(connection_file: PathBuf) -> Self {
        Self(Uuid::new_v5(
            &Uuid::NAMESPACE_URL,
            #[cfg(unix)]
            connection_file.as_os_str().as_bytes(),
            #[cfg(windows)]
            connection_file.as_os_str().as_encoded_bytes(),
        ))
    }
}

impl Display for RuntimeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// A Jupyter runtime, representing the state of a running kernel
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JupyterRuntime {
    pub connection_info: ConnectionInfo,
    pub connection_file: PathBuf,
    pub id: RuntimeId,
    // TODO: create an enum for activity state
    pub state: String,
    // pub kernel_info: Option<Box<KernelInfoReply>>,
}

impl JupyterRuntime {
    /// Create a new JupyterRuntime from an on-disk connection file
    pub async fn from_path(connection_file: PathBuf) -> Result<Self> {
        let connection_info = ConnectionInfo::from_path(&connection_file).await?;
        Ok(Self::new(connection_info, connection_file))
    }

    /// Create a new JupyterRuntime from a connection file and an existing ConnectionInfo object
    /// This does not read from the connection_file path, but assumes that the ConnectionInfo
    /// object was read from it already.
    pub fn new(connection_info: ConnectionInfo, connection_file: PathBuf) -> Self {
        let id = RuntimeId::new(connection_file.clone());
        Self {
            connection_info,
            connection_file,
            id,
            state: "idle".to_string(),
            // kernel_info: None,
        }
    }

    pub async fn remove_connection_file(&self) -> Result<()> {
        fs::remove_file(&self.connection_file)
            .await
            .context("Failed to remove connection file")
    }

    /// Connect the ZeroMQ sockets to a running kernel, and return
    /// a `JupyterClient` object that can be used to interact with the kernel.
    pub async fn attach(&self) -> Result<JupyterClient> {
        let mut iopub_socket = zeromq::SubSocket::new();
        match iopub_socket.subscribe("").await {
            Ok(_) => (),
            Err(e) => return Err(anyhow!("Error subscribing to iopub: {}", e)),
        }

        let session_id = Uuid::new_v4().to_string();

        let mut iopub_connection =
            Connection::new(iopub_socket, &self.connection_info.key, &session_id);
        iopub_connection
            .socket
            .connect(&self.connection_info.iopub_url())
            .await?;

        let shell_socket = zeromq::DealerSocket::new();
        let mut shell_connection =
            Connection::new(shell_socket, &self.connection_info.key, &session_id);
        shell_connection
            .socket
            .connect(&self.connection_info.shell_url())
            .await?;

        let stdin_socket = zeromq::DealerSocket::new();
        let mut stdin_connection =
            Connection::new(stdin_socket, &self.connection_info.key, &session_id);
        stdin_connection
            .socket
            .connect(&self.connection_info.stdin_url())
            .await?;

        let control_socket = zeromq::DealerSocket::new();
        let mut control_connection =
            Connection::new(control_socket, &self.connection_info.key, &session_id);
        control_connection
            .socket
            .connect(&self.connection_info.control_url())
            .await?;

        let heartbeat_socket = zeromq::ReqSocket::new();
        let mut heartbeat_connection =
            Connection::new(heartbeat_socket, &self.connection_info.key, &session_id);
        heartbeat_connection
            .socket
            .connect(&self.connection_info.hb_url())
            .await?;

        Ok(JupyterClient {
            iopub: iopub_connection,
            shell: shell_connection,
            stdin: stdin_connection,
            control: control_connection,
            heartbeat: heartbeat_connection,
        })
    }
}

/// A Jupyter client connection to a running kernel
pub struct JupyterClient {
    pub shell: Connection<zeromq::DealerSocket>,
    pub iopub: Connection<zeromq::SubSocket>,
    pub stdin: Connection<zeromq::DealerSocket>,
    pub control: Connection<zeromq::DealerSocket>,
    pub heartbeat: Connection<zeromq::ReqSocket>,
}

impl JupyterClient {
    /// Close all connections to the kernel
    #[cfg(feature = "tokio-runtime")]
    pub async fn detach(self) -> Result<()> {
        use std::time::Duration;
        use tokio::time::timeout;

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

    /// Send a message over the shell connection, returning the response
    /// Note: Output messages will end up on IOPub via `recv_io()`
    pub async fn send(&mut self, message: JupyterMessage) -> Result<JupyterMessage> {
        self.shell.send(message).await?;
        let response = self.shell.read().await?;
        Ok(response)
    }

    /// Send a `*_request` message to the kernel, receive the corresponding
    /// `*_reply` message, and return it. Output messages will end up on IOPub
    pub async fn send_control(&mut self, message: JupyterMessage) -> Result<JupyterMessage> {
        self.control.send(message).await?;
        let response = self.control.read().await?;
        Ok(response)
    }

    pub async fn recv_io(&mut self) -> Result<JupyterMessage> {
        self.iopub.read().await
    }
}

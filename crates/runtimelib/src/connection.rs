//! Interfacing and connecting with Jupyter kernels
//!
//! This module provides structures for understanding the connection information,
//! existing jupyter runtimes, and a client with ZeroMQ sockets to
//! communicate with the kernels.
use bytes::Bytes;
use data_encoding::HEXLOWER;

use std::net::{IpAddr, SocketAddr};

#[cfg(feature = "aws-lc-rs")]
use aws_lc_rs::hmac;
#[cfg(feature = "ring")]
use ring::hmac;
use serde_json;
use serde_json::Value;

pub use jupyter_protocol::ConnectionInfo;

pub use jupyter_protocol::messaging::*;
// For backwards compatibility, for now:
pub mod content {
    pub use jupyter_protocol::messaging::*;
}

#[cfg(feature = "tokio-runtime")]
use tokio::net::TcpListener;

#[cfg(feature = "async-dispatcher-runtime")]
use async_std::net::TcpListener;

use zeromq::Socket as _;

use zeromq::SocketRecv as _;
use zeromq::SocketSend as _;

use crate::{Result, RuntimeError};

/// Find a set of open ports. This function creates a listener with the port set to 0.
/// The listener is closed at the end of the function when the listener goes out of scope.
///
/// This of course opens a race condition in between closing the port and usage by a kernel,
/// but it is inherent to the design of the Jupyter protocol.
pub async fn peek_ports(ip: IpAddr, num: usize) -> Result<Vec<u16>> {
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

pub struct Connection<S> {
    pub socket: S,
    /// Will be None if our key was empty (digest authentication disabled).
    pub mac: Option<hmac::Key>,
    pub session_id: String,
}

pub type KernelIoPubConnection = Connection<zeromq::PubSocket>;
pub type KernelShellConnection = Connection<zeromq::RouterSocket>;
pub type KernelControlConnection = Connection<zeromq::RouterSocket>;
pub type KernelStdinConnection = Connection<zeromq::RouterSocket>;
pub struct KernelHeartbeatConnection {
    pub socket: zeromq::RepSocket,
}

pub type ClientIoPubConnection = Connection<zeromq::SubSocket>;
pub type ClientShellConnection = Connection<zeromq::DealerSocket>;
pub type ClientControlConnection = Connection<zeromq::DealerSocket>;
pub type ClientStdinConnection = Connection<zeromq::DealerSocket>;
pub struct ClientHeartbeatConnection {
    pub socket: zeromq::ReqSocket,
}

impl<S: zeromq::Socket> Connection<S> {
    pub fn new(socket: S, key: &str, session_id: &str) -> Self {
        let mac = if key.is_empty() {
            None
        } else {
            Some(hmac::Key::new(hmac::HMAC_SHA256, key.as_bytes()))
        };

        Connection {
            socket,
            mac,
            session_id: session_id.to_string(),
        }
    }
}

impl<S: zeromq::SocketSend> Connection<S> {
    pub async fn send(&mut self, message: JupyterMessage) -> Result<()> {
        let message = message.with_session(&self.session_id);
        let raw_message: RawMessage = RawMessage::from_jupyter_message(message)?;
        let zmq_message = raw_message.into_zmq_message(&self.mac)?;

        self.socket.send(zmq_message).await?;
        Ok(())
    }
}

impl<S: zeromq::SocketRecv> Connection<S> {
    pub async fn read(&mut self) -> Result<JupyterMessage> {
        let raw_message = RawMessage::from_multipart(self.socket.recv().await?, &self.mac)?;
        let message = raw_message.into_jupyter_message()?;
        Ok(message)
    }
}

impl KernelHeartbeatConnection {
    pub async fn single_heartbeat(&mut self) -> Result<()> {
        let _msg = self.socket.recv().await?;
        self.socket
            .send(zeromq::ZmqMessage::from(b"pong".to_vec()))
            .await?;
        Ok(())
    }
}

impl ClientHeartbeatConnection {
    pub async fn single_heartbeat(&mut self) -> Result<()> {
        self.socket
            .send(zeromq::ZmqMessage::from(b"ping".to_vec()))
            .await?;
        let _msg = self.socket.recv().await?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct RawMessage {
    pub zmq_identities: Vec<Bytes>,
    pub jparts: Vec<Bytes>,
}

// ZeroMQ delimiter
const DELIMITER: &[u8] = b"<IDS|MSG>";

impl RawMessage {
    pub fn from_multipart(
        multipart: zeromq::ZmqMessage,
        key: &Option<hmac::Key>,
    ) -> Result<RawMessage> {
        let delimiter_index = multipart
            .iter()
            .position(|part| &part[..] == DELIMITER)
            .ok_or(RuntimeError::MissingDelimiter)?;
        let mut parts = multipart.into_vec();

        let jparts: Vec<_> = parts.drain(delimiter_index + 2..).collect();
        let expected_hmac = parts.pop().ok_or(RuntimeError::MissingHmac)?;
        // Remove delimiter, so that what's left is just the identities.
        parts.pop();
        let zmq_identities = parts;

        let raw_message = RawMessage {
            zmq_identities,
            jparts,
        };

        if let Some(key) = key {
            let sig = HEXLOWER.decode(&expected_hmac)?;
            let mut msg = Vec::new();
            // Only include header, parent_header, metadata, and content in the HMAC.
            // Buffers are not included
            for part in &raw_message.jparts[..4] {
                msg.extend(part);
            }

            hmac::verify(key, msg.as_ref(), sig.as_ref()).map_err(RuntimeError::VerifyError)?;
        }

        Ok(raw_message)
    }

    fn hmac(&self, key: &Option<hmac::Key>) -> String {
        let hmac = if let Some(key) = key {
            let ctx = self.digest(key);
            let tag = ctx.sign();
            HEXLOWER.encode(tag.as_ref())
        } else {
            String::new()
        };
        hmac
    }

    fn digest(&self, mac: &hmac::Key) -> hmac::Context {
        let mut hmac_ctx = hmac::Context::with_key(mac);
        for part in &self.jparts {
            hmac_ctx.update(part);
        }
        hmac_ctx
    }

    fn into_zmq_message(self, key: &Option<hmac::Key>) -> Result<zeromq::ZmqMessage> {
        let hmac = self.hmac(key);

        let mut parts: Vec<bytes::Bytes> = Vec::new();
        for part in &self.zmq_identities {
            parts.push(part.to_vec().into());
        }
        parts.push(DELIMITER.into());
        parts.push(hmac.as_bytes().to_vec().into());
        for part in &self.jparts {
            parts.push(part.to_vec().into());
        }
        // ZmqMessage::try_from only fails if parts is empty, which it never
        // will be here.
        let message = zeromq::ZmqMessage::try_from(parts)
            .map_err(|e| RuntimeError::ZmqMessageError(e.to_string()))?;
        Ok(message)
    }

    fn from_jupyter_message(jupyter_message: JupyterMessage) -> Result<RawMessage> {
        let mut jparts: Vec<Bytes> = vec![
            serde_json::to_vec(&jupyter_message.header)?.into(),
            if let Some(parent_header) = jupyter_message.parent_header.as_ref() {
                serde_json::to_vec(parent_header)?.into()
            } else {
                serde_json::to_vec(&serde_json::Map::new())?.into()
            },
            serde_json::to_vec(&jupyter_message.metadata)?.into(),
            serde_json::to_vec(&jupyter_message.content)?.into(),
        ];
        jparts.extend_from_slice(&jupyter_message.buffers);
        let raw_message = RawMessage {
            zmq_identities: jupyter_message.zmq_identities.clone(),
            jparts,
        };
        Ok(raw_message)
    }

    fn into_jupyter_message(self) -> Result<JupyterMessage> {
        if self.jparts.len() < 4 {
            // Be explicit with error here
            return Err(RuntimeError::InsufficientMessageParts(self.jparts.len()));
        }

        let header: Header = serde_json::from_slice(&self.jparts[0])?;
        let content: Value = serde_json::from_slice(&self.jparts[3])?;

        let content = JupyterMessageContent::from_type_and_content(&header.msg_type, content);

        let content = match content {
            Ok(content) => content,
            Err(err) => {
                return Err(RuntimeError::ParseError {
                    msg_type: header.msg_type,
                    source: err,
                });
            }
        };

        let parent_header = serde_json::from_slice(&self.jparts[1]).ok();

        let message = JupyterMessage {
            zmq_identities: self.zmq_identities,
            header,
            parent_header,
            metadata: serde_json::from_slice(&self.jparts[2])?,
            content,
            buffers: if self.jparts.len() > 4 {
                self.jparts[4..].to_vec()
            } else {
                vec![]
            },
            channel: None,
        };

        Ok(message)
    }
}

pub async fn create_kernel_iopub_connection(
    connection_info: &ConnectionInfo,
    session_id: &str,
) -> Result<KernelIoPubConnection> {
    let endpoint = connection_info.iopub_url();

    let mut socket = zeromq::PubSocket::new();
    socket.bind(&endpoint).await?;
    Ok(Connection::new(socket, &connection_info.key, session_id))
}

pub async fn create_kernel_shell_connection(
    connection_info: &ConnectionInfo,
    session_id: &str,
) -> Result<KernelShellConnection> {
    let endpoint = connection_info.shell_url();

    let mut socket = zeromq::RouterSocket::new();
    socket.bind(&endpoint).await?;
    Ok(Connection::new(socket, &connection_info.key, session_id))
}

pub async fn create_kernel_control_connection(
    connection_info: &ConnectionInfo,
    session_id: &str,
) -> Result<KernelControlConnection> {
    let endpoint = connection_info.control_url();

    let mut socket = zeromq::RouterSocket::new();
    socket.bind(&endpoint).await?;
    Ok(Connection::new(socket, &connection_info.key, session_id))
}

pub async fn create_kernel_stdin_connection(
    connection_info: &ConnectionInfo,
    session_id: &str,
) -> Result<KernelStdinConnection> {
    let endpoint = connection_info.stdin_url();

    let mut socket = zeromq::RouterSocket::new();
    socket.bind(&endpoint).await?;
    Ok(Connection::new(socket, &connection_info.key, session_id))
}

pub async fn create_kernel_heartbeat_connection(
    connection_info: &ConnectionInfo,
) -> Result<KernelHeartbeatConnection> {
    let endpoint = connection_info.hb_url();

    let mut socket = zeromq::RepSocket::new();
    socket.bind(&endpoint).await?;
    Ok(KernelHeartbeatConnection { socket })
}

pub async fn create_client_iopub_connection(
    connection_info: &ConnectionInfo,
    topic: &str,
    session_id: &str,
) -> Result<ClientIoPubConnection> {
    let endpoint = connection_info.iopub_url();

    let mut socket = zeromq::SubSocket::new();
    socket.subscribe(topic).await?;

    socket.connect(&endpoint).await?;

    Ok(Connection::new(socket, &connection_info.key, session_id))
}

pub async fn create_client_shell_connection(
    connection_info: &ConnectionInfo,
    session_id: &str,
) -> Result<ClientShellConnection> {
    let endpoint = connection_info.shell_url();

    let mut socket = zeromq::DealerSocket::new();
    socket.connect(&endpoint).await?;
    Ok(Connection::new(socket, &connection_info.key, session_id))
}

pub async fn create_client_control_connection(
    connection_info: &ConnectionInfo,
    session_id: &str,
) -> Result<ClientControlConnection> {
    let endpoint = connection_info.control_url();

    let mut socket = zeromq::DealerSocket::new();
    socket.connect(&endpoint).await?;
    Ok(Connection::new(socket, &connection_info.key, session_id))
}

pub async fn create_client_stdin_connection(
    connection_info: &ConnectionInfo,
    session_id: &str,
) -> Result<ClientStdinConnection> {
    let endpoint = connection_info.stdin_url();

    let mut socket = zeromq::DealerSocket::new();
    socket.connect(&endpoint).await?;
    Ok(Connection::new(socket, &connection_info.key, session_id))
}

pub async fn create_client_heartbeat_connection(
    connection_info: &ConnectionInfo,
) -> Result<ClientHeartbeatConnection> {
    let endpoint = connection_info.hb_url();

    let mut socket = zeromq::ReqSocket::new();
    socket.connect(&endpoint).await?;
    Ok(ClientHeartbeatConnection { socket })
}

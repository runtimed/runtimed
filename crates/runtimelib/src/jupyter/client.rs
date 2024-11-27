//! Interfacing and connecting with Jupyter kernels
//!
//! This module provides structures for understanding the connection information,
//! existing jupyter runtimes, and a client with ZeroMQ sockets to
//! communicate with the kernels.

use crate::messaging::{
    ClientControlConnection, ClientHeartbeatConnection, ClientIoPubConnection,
    ClientShellConnection, ClientStdinConnection, Connection, KernelControlConnection,
    KernelHeartbeatConnection, KernelIoPubConnection, KernelShellConnection, KernelStdinConnection,
};

#[cfg(feature = "tokio-runtime")]
use tokio::net::TcpListener;

#[cfg(feature = "async-dispatcher-runtime")]
use async_std::net::TcpListener;

use zeromq::Socket as _;

pub use jupyter_protocol::ConnectionInfo;

use anyhow::Result;
use std::net::{IpAddr, SocketAddr};

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

pub async fn create_kernel_iopub_connection(
    connection_info: &ConnectionInfo,
    session_id: &str,
) -> anyhow::Result<KernelIoPubConnection> {
    let endpoint = connection_info.iopub_url();

    let mut socket = zeromq::PubSocket::new();
    socket.bind(&endpoint).await?;
    anyhow::Ok(Connection::new(socket, &connection_info.key, session_id))
}

pub async fn create_kernel_shell_connection(
    connection_info: &ConnectionInfo,
    session_id: &str,
) -> anyhow::Result<KernelShellConnection> {
    let endpoint = connection_info.shell_url();

    let mut socket = zeromq::RouterSocket::new();
    socket.bind(&endpoint).await?;
    anyhow::Ok(Connection::new(socket, &connection_info.key, session_id))
}

pub async fn create_kernel_control_connection(
    connection_info: &ConnectionInfo,
    session_id: &str,
) -> anyhow::Result<KernelControlConnection> {
    let endpoint = connection_info.control_url();

    let mut socket = zeromq::RouterSocket::new();
    socket.bind(&endpoint).await?;
    anyhow::Ok(Connection::new(socket, &connection_info.key, session_id))
}

pub async fn create_kernel_stdin_connection(
    connection_info: &ConnectionInfo,
    session_id: &str,
) -> anyhow::Result<KernelStdinConnection> {
    let endpoint = connection_info.stdin_url();

    let mut socket = zeromq::RouterSocket::new();
    socket.bind(&endpoint).await?;
    anyhow::Ok(Connection::new(socket, &connection_info.key, session_id))
}

pub async fn create_kernel_heartbeat_connection(
    connection_info: &ConnectionInfo,
) -> anyhow::Result<KernelHeartbeatConnection> {
    let endpoint = connection_info.hb_url();

    let mut socket = zeromq::RepSocket::new();
    socket.bind(&endpoint).await?;
    anyhow::Ok(KernelHeartbeatConnection { socket })
}

pub async fn create_client_iopub_connection(
    connection_info: &ConnectionInfo,
    topic: &str,
    session_id: &str,
) -> anyhow::Result<ClientIoPubConnection> {
    let endpoint = connection_info.iopub_url();

    let mut socket = zeromq::SubSocket::new();
    socket.subscribe(topic).await?;

    socket.connect(&endpoint).await?;

    anyhow::Ok(Connection::new(socket, &connection_info.key, session_id))
}

pub async fn create_client_shell_connection(
    connection_info: &ConnectionInfo,
    session_id: &str,
) -> anyhow::Result<ClientShellConnection> {
    let endpoint = connection_info.shell_url();

    let mut socket = zeromq::DealerSocket::new();
    socket.connect(&endpoint).await?;
    anyhow::Ok(Connection::new(socket, &connection_info.key, session_id))
}

pub async fn create_client_control_connection(
    connection_info: &ConnectionInfo,
    session_id: &str,
) -> anyhow::Result<ClientControlConnection> {
    let endpoint = connection_info.control_url();

    let mut socket = zeromq::DealerSocket::new();
    socket.connect(&endpoint).await?;
    anyhow::Ok(Connection::new(socket, &connection_info.key, session_id))
}

pub async fn create_client_stdin_connection(
    connection_info: &ConnectionInfo,
    session_id: &str,
) -> anyhow::Result<ClientStdinConnection> {
    let endpoint = connection_info.stdin_url();

    let mut socket = zeromq::DealerSocket::new();
    socket.connect(&endpoint).await?;
    anyhow::Ok(Connection::new(socket, &connection_info.key, session_id))
}

pub async fn create_client_heartbeat_connection(
    connection_info: &ConnectionInfo,
) -> anyhow::Result<ClientHeartbeatConnection> {
    let endpoint = connection_info.hb_url();

    let mut socket = zeromq::ReqSocket::new();
    socket.connect(&endpoint).await?;
    anyhow::Ok(ClientHeartbeatConnection { socket })
}

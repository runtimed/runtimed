use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use jupyter_protocol::{
    ConnectionInfo, ExecutionState, JupyterMessage, JupyterMessageContent, KernelInfoReply,
    KernelInfoRequest, ShutdownRequest, Transport,
};
use runtimelib::{
    create_client_control_connection, create_client_heartbeat_connection,
    create_client_iopub_connection, create_client_shell_connection_with_identity,
    create_client_stdin_connection_with_identity, peek_ports, peer_identity_for_session,
    ClientControlConnection, ClientHeartbeatConnection, ClientIoPubConnection,
    ClientShellConnection, ClientStdinConnection, KernelspecDir,
};
use uuid::Uuid;

use crate::types::{KernelReport, TestOutcome, TestResult};

/// Default timeout for individual protocol operations.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// Time to wait for ZMQ pub/sub subscription to establish.
const IOPUB_SETTLE_TIME: Duration = Duration::from_millis(200);

/// Connected client channels for talking to a running kernel.
pub struct KernelClient {
    pub shell: ClientShellConnection,
    pub iopub: ClientIoPubConnection,
    pub control: ClientControlConnection,
    pub stdin: ClientStdinConnection,
    pub heartbeat: ClientHeartbeatConnection,
    pub session_id: String,
    pub timeout: Duration,
}

impl KernelClient {
    /// Send a message on the shell channel and return the reply.
    pub async fn shell_request(&mut self, msg: JupyterMessage) -> Result<JupyterMessage, String> {
        self.shell
            .send(msg)
            .await
            .map_err(|e| format!("shell send: {e}"))?;
        tokio::time::timeout(self.timeout, self.shell.read())
            .await
            .map_err(|_| "shell reply timeout".to_string())?
            .map_err(|e| format!("shell read: {e}"))
    }

    /// Send a message on the control channel and return the reply.
    pub async fn control_request(&mut self, msg: JupyterMessage) -> Result<JupyterMessage, String> {
        self.control
            .send(msg)
            .await
            .map_err(|e| format!("control send: {e}"))?;
        tokio::time::timeout(self.timeout, self.control.read())
            .await
            .map_err(|_| "control reply timeout".to_string())?
            .map_err(|e| format!("control read: {e}"))
    }

    /// Send a shell request and collect all iopub messages until idle.
    ///
    /// Returns (shell_reply, iopub_messages) where iopub_messages only includes
    /// messages whose parent_header matches the request.
    pub async fn execute_and_collect(
        &mut self,
        msg: JupyterMessage,
    ) -> Result<(JupyterMessage, Vec<JupyterMessage>), String> {
        let request_id = msg.header.msg_id.clone();

        self.shell
            .send(msg)
            .await
            .map_err(|e| format!("shell send: {e}"))?;

        let mut iopub_msgs = Vec::new();
        let deadline = Instant::now() + self.timeout;

        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err("timeout collecting iopub messages".to_string());
            }

            let msg = tokio::time::timeout(remaining, self.iopub.read())
                .await
                .map_err(|_| "iopub read timeout".to_string())?
                .map_err(|e| format!("iopub read: {e}"))?;

            // Only collect messages related to our request
            let is_ours = msg
                .parent_header
                .as_ref()
                .map(|h| h.msg_id == request_id)
                .unwrap_or(false);

            if !is_ours {
                continue;
            }

            // Check for idle status — signals end of execution
            if let JupyterMessageContent::Status(status) = &msg.content {
                if status.execution_state == ExecutionState::Idle {
                    iopub_msgs.push(msg);
                    break;
                }
            }

            iopub_msgs.push(msg);
        }

        // Now read the shell reply
        let reply = tokio::time::timeout(self.timeout, self.shell.read())
            .await
            .map_err(|_| "shell reply timeout".to_string())?
            .map_err(|e| format!("shell read: {e}"))?;

        Ok((reply, iopub_msgs))
    }
}

/// A running kernel process with connected client channels.
pub struct KernelUnderTest {
    pub client: KernelClient,
    pub kernel_info: Option<KernelInfoReply>,
    process: tokio::process::Child,
    _connection_path: PathBuf,
}

impl KernelUnderTest {
    /// Launch a kernel from a kernelspec and connect all client channels.
    pub async fn launch(kernelspec: &KernelspecDir) -> Result<Self, String> {
        let ip: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let ports = peek_ports(ip, 5)
            .await
            .map_err(|e| format!("peek_ports: {e}"))?;

        let connection_info = ConnectionInfo {
            transport: Transport::TCP,
            ip: ip.to_string(),
            shell_port: ports[0],
            iopub_port: ports[1],
            stdin_port: ports[2],
            control_port: ports[3],
            hb_port: ports[4],
            signature_scheme: "hmac-sha256".to_string(),
            key: Uuid::new_v4().to_string(),
            kernel_name: Some(kernelspec.kernel_name.clone()),
        };

        // Write connection file
        let runtime_dir = runtimelib::dirs::runtime_dir();
        tokio::fs::create_dir_all(&runtime_dir)
            .await
            .map_err(|e| format!("create runtime dir: {e}"))?;

        let connection_path = runtime_dir.join(format!(
            "kernel-test-{}.json",
            &Uuid::new_v4().to_string()[..8]
        ));
        let content =
            serde_json::to_string(&connection_info).map_err(|e| format!("serialize: {e}"))?;
        tokio::fs::write(&connection_path, &content)
            .await
            .map_err(|e| format!("write connection file: {e}"))?;

        // Launch kernel process
        let process = kernelspec
            .clone()
            .command(
                &connection_path,
                Some(std::process::Stdio::piped()),
                Some(std::process::Stdio::piped()),
            )
            .map_err(|e| format!("kernel command: {e}"))?
            .current_dir("/tmp")
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| format!("spawn kernel: {e}"))?;

        // Connect all client channels
        let session_id = Uuid::new_v4().to_string();
        let identity = peer_identity_for_session(&session_id)
            .map_err(|e| format!("peer identity: {e}"))?;

        let shell = create_client_shell_connection_with_identity(
            &connection_info,
            &session_id,
            identity.clone(),
        )
        .await
        .map_err(|e| format!("shell connect: {e}"))?;

        let iopub = create_client_iopub_connection(&connection_info, "", &session_id)
            .await
            .map_err(|e| format!("iopub connect: {e}"))?;

        let control = create_client_control_connection(&connection_info, &session_id)
            .await
            .map_err(|e| format!("control connect: {e}"))?;

        let stdin = create_client_stdin_connection_with_identity(
            &connection_info,
            &session_id,
            identity,
        )
        .await
        .map_err(|e| format!("stdin connect: {e}"))?;

        let heartbeat = create_client_heartbeat_connection(&connection_info)
            .await
            .map_err(|e| format!("heartbeat connect: {e}"))?;

        // Wait for ZMQ pub/sub to settle
        tokio::time::sleep(IOPUB_SETTLE_TIME).await;

        let client = KernelClient {
            shell,
            iopub,
            control,
            stdin,
            heartbeat,
            session_id,
            timeout: DEFAULT_TIMEOUT,
        };

        Ok(Self {
            client,
            kernel_info: None,
            process,
            _connection_path: connection_path,
        })
    }

    /// Fetch kernel_info and store it for later use by tests.
    pub async fn fetch_kernel_info(&mut self) -> Result<KernelInfoReply, String> {
        let request: JupyterMessage = KernelInfoRequest {}.into();
        let reply = self.client.shell_request(request).await?;

        match reply.content {
            JupyterMessageContent::KernelInfoReply(info) => {
                let info = *info;
                self.kernel_info = Some(info.clone());
                Ok(info)
            }
            other => Err(format!(
                "expected kernel_info_reply, got {}",
                other.message_type()
            )),
        }
    }

    /// Shut down the kernel cleanly, falling back to kill.
    pub async fn shutdown(mut self) {
        // Try clean shutdown via control channel
        let request: JupyterMessage = ShutdownRequest { restart: false }.into();
        let _ = self.client.control_request(request).await;

        // Give it a moment to exit
        let exit = tokio::time::timeout(Duration::from_secs(5), self.process.wait()).await;
        if exit.is_err() {
            let _ = self.process.kill().await;
        }

        // Clean up connection file
        let _ = tokio::fs::remove_file(&self._connection_path).await;
    }
}

/// A protocol test: a named function that exercises one aspect of the protocol.
pub struct ProtocolTest {
    pub name: &'static str,
    pub description: &'static str,
    pub category: crate::types::TestCategory,
    pub run: fn(&mut KernelUnderTest) -> std::pin::Pin<Box<dyn std::future::Future<Output = TestOutcome> + Send + '_>>,
}

/// Run the full conformance suite against a kernel.
pub async fn run_conformance_suite(
    kernelspec: &KernelspecDir,
    tests: &[ProtocolTest],
) -> Result<KernelReport, String> {
    let run_start = Instant::now();

    eprintln!(
        "Launching kernel '{}' ({})",
        kernelspec.kernel_name, kernelspec.kernelspec.display_name
    );

    let mut kernel = KernelUnderTest::launch(kernelspec).await?;

    // Get kernel info first — many tests need it and it proves the kernel is alive
    let kernel_info = kernel.fetch_kernel_info().await.ok();

    let mut results = Vec::new();

    for test in tests {
        eprint!("  {} ... ", test.name);
        let test_start = Instant::now();
        let outcome = (test.run)(&mut kernel).await;
        let duration = test_start.elapsed();
        eprintln!("{} ({:.0?})", outcome.symbol(), duration);

        results.push(TestResult {
            test_name: test.name.to_string(),
            description: test.description.to_string(),
            category: test.category,
            tier: test.category.tier(),
            outcome,
            duration,
        });
    }

    kernel.shutdown().await;

    let timestamp = chrono::Utc::now().to_rfc3339();

    Ok(KernelReport {
        kernel_name: kernelspec.kernel_name.clone(),
        display_name: kernelspec.kernelspec.display_name.clone(),
        language: kernelspec.kernelspec.language.clone(),
        protocol_version: kernel_info.as_ref().map(|i| i.protocol_version.clone()),
        implementation: kernel_info.as_ref().map(|i| i.implementation.clone()),
        results,
        timestamp,
        total_duration: run_start.elapsed(),
    })
}

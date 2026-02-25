//! Notebook session combining Y.Doc sync and kernel execution.
//!
//! This module provides a high-level API for collaborative notebook editing
//! and code execution, combining:
//! - Y-sync for real-time document synchronization
//! - Kernel WebSocket for code execution
//! - Execution coordinator for routing outputs
//!
//! ## Example
//!
//! ```rust,ignore
//! use jupyter_ysync::session::{NotebookSession, SessionConfig};
//!
//! // Connect to a jupyter-server-documents server
//! let config = SessionConfig::new("http://localhost:8888", "test.ipynb")
//!     .with_token("mytoken");
//!
//! let mut session = NotebookSession::connect(config).await?;
//!
//! // Execute a cell
//! let events = session.execute_cell(0).await?;
//! for event in events {
//!     println!("{:?}", event);
//! }
//!
//! session.close().await?;
//! ```

use futures::{SinkExt, StreamExt};
use jupyter_protocol::{ExecuteRequest, ExecutionState, JupyterMessage, JupyterMessageContent};
use jupyter_websocket_client::{JupyterWebSocket, ProtocolMode, RemoteServer};
use yrs::{Array, Map, Text, Transact};

use crate::client::{build_room_url, ClientConfig, RoomId, YSyncClient};
use crate::doc::{cell_types, keys, NotebookDoc};
use crate::error::{Result, YSyncError};
use crate::executor::{CellExecutor, ExecutionEvent};

/// Configuration for connecting to a notebook session.
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Base URL of the Jupyter server (e.g., "http://localhost:8888")
    pub base_url: String,
    /// Path to the notebook file
    pub notebook_path: String,
    /// Authentication token
    pub token: Option<String>,
    /// Kernel name to launch (if not provided, uses default)
    pub kernel_name: Option<String>,
    /// Whether to write outputs to Y.Doc locally (for Use Case 2)
    /// If false, assumes server handles output storage (Use Case 1)
    pub write_outputs_locally: bool,
}

impl SessionConfig {
    /// Create a new session configuration.
    pub fn new(base_url: impl Into<String>, notebook_path: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            notebook_path: notebook_path.into(),
            token: None,
            kernel_name: None,
            write_outputs_locally: false,
        }
    }

    /// Set the authentication token.
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Set the kernel name to launch.
    pub fn with_kernel(mut self, kernel_name: impl Into<String>) -> Self {
        self.kernel_name = Some(kernel_name.into());
        self
    }

    /// Enable local output writing (for Use Case 2).
    pub fn with_local_outputs(mut self) -> Self {
        self.write_outputs_locally = true;
        self
    }
}

/// A notebook session for collaborative editing and execution.
///
/// This combines Y-sync document synchronization with kernel execution,
/// providing a unified API for notebook operations.
pub struct NotebookSession {
    /// The synchronized notebook document
    doc: NotebookDoc,
    /// Y-sync client for document synchronization
    ysync_client: YSyncClient,
    /// Kernel WebSocket connection (if connected)
    kernel: Option<KernelConnection>,
    /// Execution coordinator
    executor: CellExecutor,
    /// Session configuration
    config: SessionConfig,
    /// Jupyter session ID (for kernel connection association)
    session_id: Option<String>,
    /// Whether we created the session (vs. found an existing one)
    /// If we found an existing session (e.g., JupyterLab's), we should NOT
    /// shut down the kernel when we close - that would kill JupyterLab's kernel.
    owns_session: bool,
}

/// Kernel connection state.
struct KernelConnection {
    /// The kernel ID
    kernel_id: String,
    /// WebSocket connection split into reader and writer
    writer: futures::stream::SplitSink<JupyterWebSocket, JupyterMessage>,
    reader: futures::stream::SplitStream<JupyterWebSocket>,
}

impl NotebookSession {
    /// Connect to a notebook session.
    ///
    /// This connects to the Y-sync collaboration room and synchronizes
    /// the notebook document. Use `connect_kernel()` to also connect
    /// to a kernel for execution.
    pub async fn connect(config: SessionConfig) -> Result<Self> {
        // First, get the file ID from the server
        // jupyter-server-documents uses file IDs (UUIDs), not paths
        let file_id = Self::get_file_id(&config).await?;

        // Build room URL with file ID
        // jupyter-server-documents uses format:contentType:file_id
        // For notebooks, contentType is "notebook"
        let room_id = RoomId::new("json", "notebook", &file_id);
        let room_url = build_room_url(&config.base_url, &room_id, config.token.as_deref());

        // Connect to Y-sync room
        let ysync_config = ClientConfig::new(&room_url);
        let mut ysync_client = YSyncClient::connect(ysync_config).await?;

        // Create document and sync
        let doc = NotebookDoc::new();
        ysync_client.sync(doc.doc()).await?;

        Ok(Self {
            doc,
            ysync_client,
            kernel: None,
            executor: CellExecutor::new(),
            config,
            session_id: None,
            owns_session: false,
        })
    }

    /// Get the file ID for a notebook path from jupyter-server-documents.
    ///
    /// The server uses file IDs (UUIDs) rather than paths for room identification.
    async fn get_file_id(config: &SessionConfig) -> Result<String> {
        let client = reqwest::Client::new();

        let base_url = format!("{}/api/fileid/index", config.base_url);

        #[derive(serde::Deserialize)]
        struct FileIdResponse {
            id: String,
        }

        let mut request = client
            .post(&base_url)
            .query(&[("path", &config.notebook_path)]);

        if let Some(token) = &config.token {
            request = request.query(&[("token", token)]);
        }

        let resp: FileIdResponse = request
            .send()
            .await
            .map_err(|e| YSyncError::ProtocolError(format!("Failed to get file ID: {}", e)))?
            .json()
            .await
            .map_err(|e| {
                YSyncError::ProtocolError(format!("Failed to parse file ID response: {}", e))
            })?;

        Ok(resp.id)
    }

    /// Connect to a kernel for code execution.
    ///
    /// If `kernel_id` is None, launches a new kernel using the configured
    /// kernel name (or the server's default).
    pub async fn connect_kernel(&mut self, kernel_id: Option<&str>) -> Result<()> {
        let server = RemoteServer {
            base_url: self.config.base_url.clone(),
            token: self.config.token.clone().unwrap_or_default(),
        };

        // Get or launch kernel (and session for association)
        let kernel_id = match kernel_id {
            Some(id) => id.to_string(),
            None => {
                let (session_id, kernel_id, created) = self.launch_kernel(&server).await?;
                self.session_id = Some(session_id);
                self.owns_session = created;
                kernel_id
            }
        };

        // Connect to kernel WebSocket with session_id for proper association
        // This tells the kernel manager that this connection belongs to a session,
        // preventing premature kernel shutdown.
        let (kernel_ws, _response) = server
            .connect_to_kernel_with_session(&kernel_id, self.session_id.as_deref())
            .await
            .map_err(|e| {
                YSyncError::ProtocolError(format!("Failed to connect to kernel: {}", e))
            })?;

        // Check if v1 binary protocol is being used (which sends IoPubWelcome)
        let is_v1_protocol = kernel_ws.protocol_mode == ProtocolMode::BinaryV1;

        let (writer, reader) = kernel_ws.split();

        self.kernel = Some(KernelConnection {
            kernel_id,
            writer,
            reader,
        });

        // Wait for iopub_welcome only if using v1 binary protocol
        // JSON/legacy protocol doesn't send this message
        if is_v1_protocol {
            self.wait_for_iopub_ready().await?;
        }

        Ok(())
    }

    /// Find or create a session for the notebook.
    ///
    /// jupyter-server-documents uses the YDocSessionManager which links
    /// sessions/kernels to Y.Doc rooms. Creating a session via the sessions API
    /// (not the kernels API directly) ensures proper linkage.
    ///
    /// Returns (session_id, kernel_id, created) where:
    /// - session_id: Used when connecting to kernel WebSocket
    /// - kernel_id: The kernel to connect to
    /// - created: true if we created a new session, false if we found an existing one
    async fn launch_kernel(&self, server: &RemoteServer) -> Result<(String, String, bool)> {
        let client = reqwest::Client::new();
        let sessions_url = format!("{}/api/sessions?token={}", server.base_url, server.token);

        #[derive(serde::Deserialize)]
        struct Session {
            id: String,
            kernel: SessionKernel,
            path: Option<String>,
        }
        #[derive(serde::Deserialize)]
        struct SessionKernel {
            id: String,
        }

        // First, check for an existing session for this notebook
        if let Ok(resp) = client.get(&sessions_url).send().await {
            if let Ok(sessions) = resp.json::<Vec<Session>>().await {
                for session in sessions {
                    if session.path.as_deref() == Some(&self.config.notebook_path) {
                        // Found existing session (e.g., JupyterLab's) - don't own it
                        return Ok((session.id, session.kernel.id, false));
                    }
                }
            }
        }

        // No existing session - create one via the sessions API
        // This is CRITICAL: YDocSessionManager intercepts session creation
        // and links the kernel to the YRoom. Using /api/kernels directly
        // creates orphan kernels not linked to the Y.Doc.
        let kernel_name = match &self.config.kernel_name {
            Some(name) => name.clone(),
            None => {
                let url = format!("{}/api/kernelspecs?token={}", server.base_url, server.token);
                let resp: jupyter_websocket_client::KernelSpecsResponse = client
                    .get(&url)
                    .send()
                    .await
                    .map_err(|e| {
                        YSyncError::ProtocolError(format!("Failed to get kernelspecs: {}", e))
                    })?
                    .json()
                    .await
                    .map_err(|e| {
                        YSyncError::ProtocolError(format!("Failed to parse kernelspecs: {}", e))
                    })?;
                resp.default
            }
        };

        // Create session via POST /api/sessions (triggers YDocSessionManager)
        #[derive(serde::Serialize)]
        struct CreateSessionRequest {
            path: String,
            name: String,
            #[serde(rename = "type")]
            session_type: String,
            kernel: KernelSpec,
        }
        #[derive(serde::Serialize)]
        struct KernelSpec {
            name: String,
        }

        let create_req = CreateSessionRequest {
            path: self.config.notebook_path.clone(),
            name: self.config.notebook_path.clone(),
            session_type: "notebook".to_string(),
            kernel: KernelSpec { name: kernel_name },
        };

        let session: Session = client
            .post(&sessions_url)
            .json(&create_req)
            .send()
            .await
            .map_err(|e| YSyncError::ProtocolError(format!("Failed to create session: {}", e)))?
            .json()
            .await
            .map_err(|e| {
                YSyncError::ProtocolError(format!("Failed to parse session response: {}", e))
            })?;

        // We created this session, so we own it
        Ok((session.id, session.kernel.id, true))
    }

    /// Wait for the IOPub channel to be ready.
    ///
    /// Waits for `IoPubWelcome` message or times out after receiving other messages.
    /// Some servers may not send IoPubWelcome, so we proceed after a timeout.
    async fn wait_for_iopub_ready(&mut self) -> Result<()> {
        let kernel = self
            .kernel
            .as_mut()
            .ok_or_else(|| YSyncError::ProtocolError("No kernel connected".into()))?;

        // Wait for first few messages to check for IoPubWelcome
        // If we get other messages (like status), assume the channel is ready
        for _ in 0..3 {
            let recv_future = kernel.reader.next();
            let timeout_duration = tokio::time::Duration::from_millis(500);

            match tokio::time::timeout(timeout_duration, recv_future).await {
                Ok(Some(Ok(msg))) => {
                    if matches!(&msg.content, JupyterMessageContent::IoPubWelcome(_)) {
                        return Ok(());
                    }
                    // Got another message type - channel is working, continue checking
                }
                Ok(Some(Err(e))) => {
                    return Err(YSyncError::ProtocolError(format!(
                        "Kernel message error: {}",
                        e
                    )));
                }
                Ok(None) => {
                    return Err(YSyncError::ProtocolError("Connection closed".into()));
                }
                Err(_) => {
                    // Timeout - no IoPubWelcome but that's OK for some servers
                    return Ok(());
                }
            }
        }

        // Received messages but no IoPubWelcome - channel is working
        Ok(())
    }

    /// Get a reference to the notebook document.
    pub fn doc(&self) -> &NotebookDoc {
        &self.doc
    }

    /// Get the number of cells in the notebook.
    pub fn cell_count(&self) -> u32 {
        self.doc.cell_count()
    }

    /// Get the source code of a cell.
    pub fn get_cell_source(&self, cell_index: u32) -> Option<String> {
        let cell = self.doc.get_cell(cell_index)?;
        let txn = self.doc.doc().transact();
        cell.source_as_string(&txn)
    }

    /// Set the source code of a cell.
    pub fn set_cell_source(&self, cell_index: u32, source: &str) -> Result<()> {
        let txn = self.doc.doc().transact();
        let cells = self.doc.cells(&txn);

        if cell_index >= cells.len(&txn) {
            return Err(YSyncError::ConversionError(format!(
                "Cell index {} out of bounds",
                cell_index
            )));
        }

        let cell = cells
            .get(&txn, cell_index)
            .ok_or_else(|| YSyncError::ConversionError(format!("Cell {} not found", cell_index)))?;

        let yrs::Out::YMap(cell_map) = cell else {
            return Err(YSyncError::ConversionError("Cell is not a map".into()));
        };

        // Get source - it might be Y.Text or string
        match cell_map.get(&txn, keys::SOURCE) {
            Some(yrs::Out::YText(text)) => {
                drop(txn);
                let mut txn = self.doc.doc().transact_mut();
                let len = text.len(&txn);
                text.remove_range(&mut txn, 0, len);
                text.insert(&mut txn, 0, source);
            }
            _ => {
                drop(txn);
                let mut txn = self.doc.doc().transact_mut();
                cell_map.insert(&mut txn, keys::SOURCE, yrs::Any::String(source.into()));
            }
        }

        Ok(())
    }

    /// Execute a cell and return execution events.
    ///
    /// This sends an execute_request to the kernel and processes responses
    /// until execution completes (kernel goes idle after busy).
    ///
    /// If `write_outputs_locally` is enabled in the config, outputs are
    /// written directly to the Y.Doc. Otherwise, outputs are expected to
    /// come from the server via Y-sync.
    pub async fn execute_cell(&mut self, cell_index: u32) -> Result<Vec<ExecutionEvent>> {
        // Get cell source
        let source = self
            .get_cell_source(cell_index)
            .ok_or_else(|| YSyncError::ConversionError(format!("Cell {} not found", cell_index)))?;

        // Verify it's a code cell
        let cell = self
            .doc
            .get_cell(cell_index)
            .ok_or_else(|| YSyncError::ConversionError(format!("Cell {} not found", cell_index)))?;
        let txn = self.doc.doc().transact();
        let cell_type = cell.cell_type(&txn);
        drop(txn);

        if cell_type.as_deref() != Some(cell_types::CODE) {
            return Err(YSyncError::ConversionError(format!(
                "Cell {} is not a code cell",
                cell_index
            )));
        }

        // Clear outputs before execution if writing locally
        if self.config.write_outputs_locally {
            self.doc.clear_cell_outputs(cell_index)?;
        }

        // Get kernel connection
        let kernel = self
            .kernel
            .as_mut()
            .ok_or_else(|| YSyncError::ProtocolError("No kernel connected".into()))?;

        // Create execute request
        let execute_request = ExecuteRequest {
            code: source,
            silent: false,
            store_history: true,
            user_expressions: Default::default(),
            allow_stdin: false,
            stop_on_error: true,
        };

        let msg = JupyterMessage::new(execute_request, None);
        let msg_id = msg.header.msg_id.clone();

        // Register execution with coordinator
        self.executor.register_execution(msg_id.clone(), cell_index);

        // Send execute request - cancel registration if send fails
        if let Err(e) = kernel.writer.send(msg).await {
            self.executor.cancel(&msg_id);
            return Err(YSyncError::ProtocolError(format!(
                "Failed to send execute request: {}",
                e
            )));
        }

        // Collect events
        let mut all_events = Vec::new();

        // Process responses until completion
        while self.executor.has_pending() {
            if let Some(response) = kernel.reader.next().await {
                let msg = response.map_err(|e| {
                    YSyncError::ProtocolError(format!("Kernel message error: {}", e))
                })?;

                // Handle message with executor if writing outputs locally
                if self.config.write_outputs_locally {
                    if let Some(events) = self.executor.handle_message(&msg, &self.doc)? {
                        all_events.extend(events);
                    }
                } else {
                    // Just track execution state, don't write outputs
                    if let Some(events) = handle_message_without_outputs(
                        &mut self.executor,
                        &msg,
                        &msg_id,
                        cell_index,
                    )? {
                        all_events.extend(events);
                    }
                }
            } else {
                break;
            }
        }

        Ok(all_events)
    }

    /// Sync local changes to the server.
    pub async fn sync_to_server(&mut self) -> Result<()> {
        self.ysync_client.send_update(self.doc.doc()).await
    }

    /// Receive and apply updates from the server.
    pub async fn receive_updates(&mut self) -> Result<bool> {
        if let Some(msg) = self.ysync_client.receive_message().await? {
            self.ysync_client
                .handle_message(self.doc.doc(), &msg)
                .await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Get the kernel ID if connected.
    pub fn kernel_id(&self) -> Option<&str> {
        self.kernel.as_ref().map(|k| k.kernel_id.as_str())
    }

    /// Get the session ID if a session has been created/found.
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// Check if a kernel is connected.
    pub fn has_kernel(&self) -> bool {
        self.kernel.is_some()
    }

    /// Shutdown the kernel and session if we own it.
    ///
    /// If we found and reused an existing session (e.g., JupyterLab's), we don't
    /// shut down the kernel - that would destroy JupyterLab's kernel and all
    /// execution state.
    pub async fn shutdown_kernel(&mut self) -> Result<()> {
        // Always drop our kernel connection
        let _kernel = self.kernel.take();

        // Only delete the session/kernel if we created it
        if self.owns_session {
            if let Some(session_id) = self.session_id.take() {
                let client = reqwest::Client::new();

                // Delete the session (which also cleans up the kernel)
                let url = format!(
                    "{}/api/sessions/{}?token={}",
                    self.config.base_url,
                    session_id,
                    self.config.token.as_deref().unwrap_or("")
                );

                // Best effort - don't fail if cleanup fails
                let _ = client.delete(&url).send().await;
            }
        }
        Ok(())
    }

    /// Close the session, shutting down the kernel and disconnecting.
    ///
    /// This includes a brief delay to allow the server to process any pending
    /// sync updates before the WebSocket connection is closed.
    pub async fn close(mut self) -> Result<()> {
        self.shutdown_kernel().await?;
        // Brief delay to allow server to process pending updates
        // This works around a race condition in jupyter-server-documents where
        // the server may still be processing our last update when we disconnect
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        self.ysync_client.close().await?;
        Ok(())
    }
}

/// Handle kernel message without writing outputs (for server-managed outputs).
fn handle_message_without_outputs(
    executor: &mut CellExecutor,
    msg: &JupyterMessage,
    expected_msg_id: &str,
    cell_index: u32,
) -> Result<Option<Vec<ExecutionEvent>>> {
    // Check parent message ID
    let parent_msg_id = match &msg.parent_header {
        Some(header) => &header.msg_id,
        None => return Ok(None),
    };

    if parent_msg_id != expected_msg_id {
        return Ok(None);
    }

    let mut events = Vec::new();

    match &msg.content {
        JupyterMessageContent::Status(status) => {
            match status.execution_state {
                ExecutionState::Busy => {
                    executor.mark_busy(expected_msg_id);
                    events.push(ExecutionEvent::Started {
                        cell_index,
                        msg_id: expected_msg_id.to_string(),
                    });
                }
                ExecutionState::Idle if executor.saw_busy(expected_msg_id) => {
                    // Execution complete - cancel tracking
                    executor.cancel(expected_msg_id);
                    events.push(ExecutionEvent::Completed {
                        cell_index,
                        msg_id: expected_msg_id.to_string(),
                    });
                }
                _ => {}
            }
        }
        JupyterMessageContent::ExecuteReply(reply) => {
            let count = reply.execution_count.value();
            if count > 0 {
                events.push(ExecutionEvent::ExecutionCountUpdated {
                    cell_index,
                    count: count as i32,
                });
            }
        }
        JupyterMessageContent::ErrorOutput(error) => {
            events.push(ExecutionEvent::Error {
                cell_index,
                msg_id: expected_msg_id.to_string(),
                ename: error.ename.clone(),
                evalue: error.evalue.clone(),
            });
        }
        _ => {}
    }

    if events.is_empty() {
        Ok(None)
    } else {
        Ok(Some(events))
    }
}

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
use jupyter_websocket_client::{JupyterWebSocket, RemoteServer};
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

        let mut request = client.post(&base_url).query(&[("path", &config.notebook_path)]);

        if let Some(token) = &config.token {
            request = request.query(&[("token", token)]);
        }

        let resp: FileIdResponse = request
            .send()
            .await
            .map_err(|e| YSyncError::ProtocolError(format!("Failed to get file ID: {}", e)))?
            .json()
            .await
            .map_err(|e| YSyncError::ProtocolError(format!("Failed to parse file ID response: {}", e)))?;

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

        // Get or launch kernel
        let kernel_id = match kernel_id {
            Some(id) => id.to_string(),
            None => self.launch_kernel(&server).await?,
        };

        // Connect to kernel WebSocket
        let (kernel_ws, _response) = server
            .connect_to_kernel(&kernel_id)
            .await
            .map_err(|e| YSyncError::ProtocolError(format!("Failed to connect to kernel: {}", e)))?;

        let (writer, reader) = kernel_ws.split();

        self.kernel = Some(KernelConnection {
            kernel_id,
            writer,
            reader,
        });

        // Wait for iopub_welcome to ensure IOPub channel is ready
        self.wait_for_iopub_ready().await?;

        Ok(())
    }

    /// Launch a new kernel and return its ID.
    async fn launch_kernel(&self, server: &RemoteServer) -> Result<String> {
        let client = reqwest::Client::new();

        // Get default kernel name if not specified
        let kernel_name = match &self.config.kernel_name {
            Some(name) => name.clone(),
            None => {
                let url = format!(
                    "{}/api/kernelspecs?token={}",
                    server.base_url, server.token
                );
                let resp: jupyter_websocket_client::KernelSpecsResponse = client
                    .get(&url)
                    .send()
                    .await
                    .map_err(|e| YSyncError::ProtocolError(format!("Failed to get kernelspecs: {}", e)))?
                    .json()
                    .await
                    .map_err(|e| YSyncError::ProtocolError(format!("Failed to parse kernelspecs: {}", e)))?;
                resp.default
            }
        };

        // Launch kernel
        let url = format!("{}/api/kernels?token={}", server.base_url, server.token);
        let launch_req = jupyter_websocket_client::KernelLaunchRequest {
            name: kernel_name,
            path: Some(self.config.notebook_path.clone()),
        };

        let kernel: jupyter_websocket_client::Kernel = client
            .post(&url)
            .json(&launch_req)
            .send()
            .await
            .map_err(|e| YSyncError::ProtocolError(format!("Failed to launch kernel: {}", e)))?
            .json()
            .await
            .map_err(|e| YSyncError::ProtocolError(format!("Failed to parse kernel response: {}", e)))?;

        Ok(kernel.id)
    }

    /// Wait for the IOPub channel to be ready.
    async fn wait_for_iopub_ready(&mut self) -> Result<()> {
        let kernel = self.kernel.as_mut().ok_or_else(|| {
            YSyncError::ProtocolError("No kernel connected".into())
        })?;

        while let Some(response) = kernel.reader.next().await {
            let msg = response.map_err(|e| {
                YSyncError::ProtocolError(format!("Kernel message error: {}", e))
            })?;

            if matches!(&msg.content, JupyterMessageContent::IoPubWelcome(_)) {
                return Ok(());
            }
        }

        Err(YSyncError::ProtocolError("Connection closed before IOPub ready".into()))
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

        let cell = cells.get(&txn, cell_index).ok_or_else(|| {
            YSyncError::ConversionError(format!("Cell {} not found", cell_index))
        })?;

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
        let source = self.get_cell_source(cell_index).ok_or_else(|| {
            YSyncError::ConversionError(format!("Cell {} not found", cell_index))
        })?;

        // Verify it's a code cell
        let cell = self.doc.get_cell(cell_index).ok_or_else(|| {
            YSyncError::ConversionError(format!("Cell {} not found", cell_index))
        })?;
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
        let kernel = self.kernel.as_mut().ok_or_else(|| {
            YSyncError::ProtocolError("No kernel connected".into())
        })?;

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
            return Err(YSyncError::ProtocolError(format!("Failed to send execute request: {}", e)));
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
                    if let Some(events) = handle_message_without_outputs(&mut self.executor, &msg, &msg_id, cell_index)? {
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
            self.ysync_client.handle_message(self.doc.doc(), &msg).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Get the kernel ID if connected.
    pub fn kernel_id(&self) -> Option<&str> {
        self.kernel.as_ref().map(|k| k.kernel_id.as_str())
    }

    /// Check if a kernel is connected.
    pub fn has_kernel(&self) -> bool {
        self.kernel.is_some()
    }

    /// Shutdown the kernel.
    pub async fn shutdown_kernel(&mut self) -> Result<()> {
        if let Some(kernel) = self.kernel.take() {
            let client = reqwest::Client::new();
            let url = format!(
                "{}/api/kernels/{}?token={}",
                self.config.base_url,
                kernel.kernel_id,
                self.config.token.as_deref().unwrap_or("")
            );

            client
                .delete(&url)
                .send()
                .await
                .map_err(|e| YSyncError::ProtocolError(format!("Failed to shutdown kernel: {}", e)))?;
        }
        Ok(())
    }

    /// Close the session, shutting down the kernel and disconnecting.
    pub async fn close(mut self) -> Result<()> {
        self.shutdown_kernel().await?;
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

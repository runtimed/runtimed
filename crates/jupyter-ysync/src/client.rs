//! Y-sync WebSocket client for connecting to collaboration servers.
//!
//! This module provides a client for connecting to jupyter-server-documents
//! or any y-sync compatible WebSocket server.
//!
//! ## Example
//!
//! ```rust,ignore
//! use jupyter_ysync::client::YSyncClient;
//! use jupyter_ysync::NotebookDoc;
//!
//! // Create a document
//! let doc = NotebookDoc::new();
//!
//! // Connect to a collaboration server
//! let client = YSyncClient::connect("ws://localhost:8888/api/collaboration/room/notebook:file:path.ipynb", None).await?;
//!
//! // Sync the document
//! client.sync(&doc).await?;
//! ```

use async_tungstenite::tokio::connect_async;
use async_tungstenite::tungstenite::client::IntoClientRequest;
use async_tungstenite::tungstenite::Message as WsMessage;
use async_tungstenite::WebSocketStream;
use futures::StreamExt;
use yrs::{Doc, ReadTxn, StateVector, Transact};

use crate::error::{Result, YSyncError};
use crate::protocol::awareness::ClientAwareness;
use crate::protocol::message::Message;
use crate::protocol::sync::{SyncProtocol, SyncState};

/// Type alias for the WebSocket stream returned by connect_async.
/// Handles both plain TCP and TLS connections automatically.
type WsStream = WebSocketStream<async_tungstenite::tokio::ConnectStream>;

/// Configuration for connecting to a y-sync server.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// WebSocket URL to connect to (e.g., ws://localhost:8888/api/collaboration/room/...)
    pub url: String,
    /// Optional authentication token
    pub token: Option<String>,
    /// User agent string
    pub user_agent: String,
}

impl ClientConfig {
    /// Create a new client configuration.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            token: None,
            user_agent: "jupyter-ysync/0.1".to_string(),
        }
    }

    /// Set the authentication token.
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Set the user agent string.
    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = user_agent.into();
        self
    }

    /// Build the WebSocket URL with optional token.
    fn build_url(&self) -> String {
        match &self.token {
            Some(token) => {
                if self.url.contains('?') {
                    format!("{}&token={}", self.url, token)
                } else {
                    format!("{}?token={}", self.url, token)
                }
            }
            None => self.url.clone(),
        }
    }
}

/// A y-sync WebSocket client for connecting to collaboration servers.
///
/// This client handles the y-sync protocol over WebSocket, managing
/// document synchronization and awareness updates.
pub struct YSyncClient {
    stream: WsStream,
    protocol: SyncProtocol,
    /// Last known remote state vector for computing incremental updates
    last_synced_sv: Option<StateVector>,
}

impl YSyncClient {
    /// Connect to a y-sync server using the given configuration.
    pub async fn connect(config: ClientConfig) -> Result<Self> {
        let url = config.build_url();

        // Use into_client_request() to properly set up WebSocket upgrade headers
        let mut request = url
            .into_client_request()
            .map_err(|e| YSyncError::ProtocolError(format!("Failed to build request: {}", e)))?;

        // Add custom headers
        request
            .headers_mut()
            .insert("User-Agent", config.user_agent.parse().unwrap());

        let (stream, _response) = connect_async(request)
            .await
            .map_err(|e| YSyncError::ProtocolError(format!("Failed to connect: {}", e)))?;

        Ok(Self {
            stream,
            protocol: SyncProtocol::new(),
            last_synced_sv: None,
        })
    }

    /// Connect to a y-sync server with a simple URL.
    pub async fn connect_url(url: &str, token: Option<&str>) -> Result<Self> {
        let mut config = ClientConfig::new(url);
        if let Some(t) = token {
            config = config.with_token(t);
        }
        Self::connect(config).await
    }

    /// Get the current sync state.
    pub fn sync_state(&self) -> SyncState {
        self.protocol.state()
    }

    /// Check if the initial sync is complete.
    pub fn is_synced(&self) -> bool {
        self.protocol.is_synced()
    }

    /// Perform initial synchronization with the server.
    ///
    /// This sends a SyncStep1 message with the document's state vector
    /// and processes the server's SyncStep2 response.
    pub async fn sync(&mut self, doc: &Doc) -> Result<()> {
        // Send SyncStep1
        let step1 = self.protocol.start(doc);
        self.send_message(&step1).await?;

        // Wait for SyncStep2
        while !self.protocol.is_synced() {
            if let Some(msg) = self.receive_message().await? {
                self.handle_message(doc, &msg).await?;
            }
        }

        // Store the current state vector for computing incremental updates
        let txn = doc.transact();
        self.last_synced_sv = Some(txn.state_vector());

        Ok(())
    }

    /// Send a y-sync protocol message.
    pub async fn send_message(&mut self, msg: &Message) -> Result<()> {
        let encoded = msg.encode();
        self.stream
            .send(WsMessage::Binary(encoded.into()))
            .await
            .map_err(|e| YSyncError::ProtocolError(format!("Failed to send message: {}", e)))?;
        Ok(())
    }

    /// Receive a y-sync protocol message.
    pub async fn receive_message(&mut self) -> Result<Option<Message>> {
        loop {
            match self.stream.next().await {
                Some(Ok(WsMessage::Binary(data))) => {
                    let msg = Message::decode(&data)?;
                    return Ok(Some(msg));
                }
                Some(Ok(WsMessage::Close(_))) => return Ok(None),
                Some(Ok(WsMessage::Ping(data))) => {
                    // Respond to ping with pong
                    self.stream
                        .send(WsMessage::Pong(data))
                        .await
                        .map_err(|e| {
                            YSyncError::ProtocolError(format!("Failed to send pong: {}", e))
                        })?;
                    // Continue receiving
                }
                Some(Ok(WsMessage::Pong(_))) => {
                    // Ignore pong, continue receiving
                }
                Some(Ok(WsMessage::Text(_))) => {
                    // Y-sync protocol uses binary, ignore text
                }
                Some(Ok(WsMessage::Frame(_))) => {
                    // Raw frame, continue receiving
                }
                Some(Err(e)) => {
                    return Err(YSyncError::ProtocolError(format!("WebSocket error: {}", e)))
                }
                None => return Ok(None),
            }
        }
    }

    /// Handle an incoming message and update the document.
    pub async fn handle_message(&mut self, doc: &Doc, msg: &Message) -> Result<()> {
        match msg {
            Message::Sync(sync_msg) => {
                let responses = self.protocol.handle_sync_message(doc, sync_msg)?;
                for response in responses {
                    self.send_message(&response).await?;
                }
            }
            Message::Awareness(_data) => {
                // Awareness updates are handled separately
                // The caller can use receive_message() directly and handle awareness
            }
            Message::AwarenessQuery => {
                // Server is querying our awareness state
                // The caller should respond with their awareness
            }
            Message::Auth(reason) => {
                if let Some(reason) = reason {
                    return Err(YSyncError::ProtocolError(format!(
                        "Authentication failed: {}",
                        reason
                    )));
                }
            }
            Message::Custom(_, _) => {
                // Ignore custom messages
            }
        }
        Ok(())
    }

    /// Send an update for local changes.
    ///
    /// Call this after making changes to the document to broadcast
    /// them to other connected clients.
    ///
    /// This sends only the changes since the last sync, not the full document.
    pub async fn send_update(&mut self, doc: &Doc) -> Result<()> {
        let msg = match &self.last_synced_sv {
            Some(sv) => SyncProtocol::encode_update(doc, sv),
            None => SyncProtocol::encode_full_update(doc),
        };
        self.send_message(&msg).await?;

        // Update the state vector after sending
        let txn = doc.transact();
        self.last_synced_sv = Some(txn.state_vector());

        Ok(())
    }

    /// Send an awareness update.
    pub async fn send_awareness(&mut self, awareness: &ClientAwareness) -> Result<()> {
        let update = awareness.encode_update()?;
        let msg = Message::awareness(&update);
        self.send_message(&msg).await
    }

    /// Close the connection gracefully.
    pub async fn close(mut self) -> Result<()> {
        self.stream
            .close(None)
            .await
            .map_err(|e| YSyncError::ProtocolError(format!("Failed to close connection: {}", e)))?;
        Ok(())
    }
}

/// Room identifier for jupyter-server-documents.
///
/// Rooms are identified by a string in the format: `{format}:{type}:{path}`
/// For example: `json:file:notebooks/example.ipynb`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RoomId {
    /// Document format (e.g., "json" for Jupyter notebooks)
    pub format: String,
    /// Document type (e.g., "file" for file-based documents)
    pub doc_type: String,
    /// Document path
    pub path: String,
}

impl RoomId {
    /// Create a new room ID.
    pub fn new(format: impl Into<String>, doc_type: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            format: format.into(),
            doc_type: doc_type.into(),
            path: path.into(),
        }
    }

    /// Create a room ID for a Jupyter notebook file.
    pub fn notebook(path: impl Into<String>) -> Self {
        Self::new("json", "file", path)
    }

    /// Parse a room ID from a string.
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.splitn(3, ':').collect();
        if parts.len() == 3 {
            Some(Self {
                format: parts[0].to_string(),
                doc_type: parts[1].to_string(),
                path: parts[2].to_string(),
            })
        } else {
            None
        }
    }
}

impl std::fmt::Display for RoomId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}", self.format, self.doc_type, self.path)
    }
}

/// Build a WebSocket URL for connecting to a jupyter-server-documents room.
pub fn build_room_url(base_url: &str, room_id: &RoomId, token: Option<&str>) -> String {
    let ws_url = base_url
        .replace("http://", "ws://")
        .replace("https://", "wss://");

    let url = format!("{}/api/collaboration/room/{}", ws_url.trim_end_matches('/'), room_id);

    match token {
        Some(t) => format!("{}?token={}", url, t),
        None => url,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_room_id_new() {
        let room = RoomId::new("json", "file", "notebooks/test.ipynb");
        assert_eq!(room.format, "json");
        assert_eq!(room.doc_type, "file");
        assert_eq!(room.path, "notebooks/test.ipynb");
    }

    #[test]
    fn test_room_id_notebook() {
        let room = RoomId::notebook("test.ipynb");
        assert_eq!(room.format, "json");
        assert_eq!(room.doc_type, "file");
        assert_eq!(room.path, "test.ipynb");
    }

    #[test]
    fn test_room_id_parse() {
        let room = RoomId::parse("json:file:notebooks/test.ipynb").unwrap();
        assert_eq!(room.format, "json");
        assert_eq!(room.doc_type, "file");
        assert_eq!(room.path, "notebooks/test.ipynb");
    }

    #[test]
    fn test_room_id_parse_with_colons_in_path() {
        let room = RoomId::parse("json:file:path:with:colons").unwrap();
        assert_eq!(room.format, "json");
        assert_eq!(room.doc_type, "file");
        assert_eq!(room.path, "path:with:colons");
    }

    #[test]
    fn test_room_id_display() {
        let room = RoomId::notebook("test.ipynb");
        assert_eq!(room.to_string(), "json:file:test.ipynb");
    }

    #[test]
    fn test_build_room_url() {
        let url = build_room_url("http://localhost:8888", &RoomId::notebook("test.ipynb"), None);
        assert_eq!(url, "ws://localhost:8888/api/collaboration/room/json:file:test.ipynb");
    }

    #[test]
    fn test_build_room_url_with_token() {
        let url = build_room_url("http://localhost:8888", &RoomId::notebook("test.ipynb"), Some("abc123"));
        assert_eq!(url, "ws://localhost:8888/api/collaboration/room/json:file:test.ipynb?token=abc123");
    }

    #[test]
    fn test_build_room_url_https() {
        let url = build_room_url("https://example.com/jupyter", &RoomId::notebook("test.ipynb"), None);
        assert_eq!(url, "wss://example.com/jupyter/api/collaboration/room/json:file:test.ipynb");
    }

    #[test]
    fn test_client_config() {
        let config = ClientConfig::new("ws://localhost:8888")
            .with_token("mytoken")
            .with_user_agent("test-agent");

        assert_eq!(config.url, "ws://localhost:8888");
        assert_eq!(config.token, Some("mytoken".to_string()));
        assert_eq!(config.user_agent, "test-agent");
    }

    #[test]
    fn test_client_config_build_url() {
        let config = ClientConfig::new("ws://localhost:8888/room").with_token("abc");
        assert_eq!(config.build_url(), "ws://localhost:8888/room?token=abc");

        let config2 = ClientConfig::new("ws://localhost:8888/room?foo=bar").with_token("abc");
        assert_eq!(config2.build_url(), "ws://localhost:8888/room?foo=bar&token=abc");
    }
}

//! Y-sync protocol message types.
//!
//! This module provides message types for the y-sync protocol, wrapping
//! the yrs sync protocol for use with Jupyter notebooks.

use yrs::encoding::read::Cursor;
use yrs::encoding::read::Read;
use yrs::encoding::write::Write;
use yrs::sync::awareness::AwarenessUpdate;
use yrs::updates::decoder::Decode;
use yrs::updates::encoder::Encode;
use yrs::StateVector;

use crate::error::{Result, YSyncError};

/// Message type tags for the y-sync protocol.
pub mod message_type {
    /// Sync protocol messages (SyncStep1, SyncStep2, Update)
    pub const SYNC: u8 = 0;
    /// Awareness update messages
    pub const AWARENESS: u8 = 1;
    /// Authentication messages
    pub const AUTH: u8 = 2;
    /// Query for current awareness state
    pub const AWARENESS_QUERY: u8 = 3;
}

/// Sync message type tags.
pub mod sync_type {
    /// Initial sync request with state vector
    pub const SYNC_STEP1: u8 = 0;
    /// Response with missing updates
    pub const SYNC_STEP2: u8 = 1;
    /// Incremental document update
    pub const UPDATE: u8 = 2;
}

/// A y-sync protocol message.
#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    /// Sync protocol message for document synchronization.
    Sync(SyncMessage),
    /// Awareness update containing user presence information.
    Awareness(Vec<u8>),
    /// Authentication/authorization message.
    Auth(Option<String>),
    /// Query for current awareness state of all clients.
    AwarenessQuery,
    /// Custom message with user-defined tag and payload.
    Custom(u8, Vec<u8>),
}

impl Message {
    /// Decode a message from bytes.
    pub fn decode(data: &[u8]) -> Result<Self> {
        let mut cursor = Cursor::new(data);
        Self::decode_from(&mut cursor)
    }

    /// Decode a message from a reader.
    pub fn decode_from<R: Read>(reader: &mut R) -> Result<Self> {
        let tag = reader
            .read_var::<u8>()
            .map_err(|e| YSyncError::ProtocolError(format!("Failed to read message tag: {}", e)))?;

        match tag {
            message_type::SYNC => {
                let sync_msg = SyncMessage::decode_from(reader)?;
                Ok(Message::Sync(sync_msg))
            }
            message_type::AWARENESS => {
                let data = reader.read_buf().map_err(|e| {
                    YSyncError::ProtocolError(format!("Failed to read awareness data: {}", e))
                })?;
                Ok(Message::Awareness(data.to_vec()))
            }
            message_type::AUTH => {
                let has_reason = reader.read_var::<u8>().map_err(|e| {
                    YSyncError::ProtocolError(format!("Failed to read auth flag: {}", e))
                })?;
                let reason = if has_reason != 0 {
                    let reason_bytes = reader.read_buf().map_err(|e| {
                        YSyncError::ProtocolError(format!("Failed to read auth reason: {}", e))
                    })?;
                    Some(String::from_utf8_lossy(reason_bytes).into_owned())
                } else {
                    None
                };
                Ok(Message::Auth(reason))
            }
            message_type::AWARENESS_QUERY => Ok(Message::AwarenessQuery),
            other => {
                let data = reader.read_buf().map_err(|e| {
                    YSyncError::ProtocolError(format!("Failed to read custom message data: {}", e))
                })?;
                Ok(Message::Custom(other, data.to_vec()))
            }
        }
    }

    /// Encode this message to bytes.
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.encode_to(&mut buf);
        buf
    }

    /// Encode this message to a writer.
    pub fn encode_to<W: Write>(&self, writer: &mut W) {
        match self {
            Message::Sync(sync_msg) => {
                writer.write_var(message_type::SYNC);
                sync_msg.encode_to(writer);
            }
            Message::Awareness(data) => {
                writer.write_var(message_type::AWARENESS);
                writer.write_buf(data);
            }
            Message::Auth(reason) => {
                writer.write_var(message_type::AUTH);
                if let Some(ref reason) = reason {
                    writer.write_var(1u8);
                    writer.write_buf(reason.as_bytes());
                } else {
                    writer.write_var(0u8);
                }
            }
            Message::AwarenessQuery => {
                writer.write_var(message_type::AWARENESS_QUERY);
            }
            Message::Custom(tag, data) => {
                writer.write_var(*tag);
                writer.write_buf(data);
            }
        }
    }

    /// Create a SyncStep1 message from a state vector.
    pub fn sync_step1(sv: &StateVector) -> Self {
        Message::Sync(SyncMessage::SyncStep1(sv.encode_v1()))
    }

    /// Create a SyncStep2 message from an update.
    pub fn sync_step2(update: Vec<u8>) -> Self {
        Message::Sync(SyncMessage::SyncStep2(update))
    }

    /// Create an Update message.
    pub fn update(update: Vec<u8>) -> Self {
        Message::Sync(SyncMessage::Update(update))
    }

    /// Create an Awareness message from an awareness update.
    pub fn awareness(update: &AwarenessUpdate) -> Self {
        Message::Awareness(update.encode_v1())
    }
}

/// A sync protocol message for document synchronization.
#[derive(Debug, Clone, PartialEq)]
pub enum SyncMessage {
    /// Initial sync request containing the client's state vector.
    /// The receiver should respond with SyncStep2 containing any
    /// updates the sender is missing.
    SyncStep1(Vec<u8>),

    /// Response to SyncStep1 containing updates the requester is missing.
    /// The data is an encoded Y.Doc update.
    SyncStep2(Vec<u8>),

    /// An incremental document update. Sent after initial sync
    /// whenever the document changes.
    Update(Vec<u8>),
}

impl SyncMessage {
    /// Decode a sync message from a reader.
    pub fn decode_from<R: Read>(reader: &mut R) -> Result<Self> {
        let tag = reader.read_var::<u8>().map_err(|e| {
            YSyncError::ProtocolError(format!("Failed to read sync message tag: {}", e))
        })?;

        let data = reader.read_buf().map_err(|e| {
            YSyncError::ProtocolError(format!("Failed to read sync message data: {}", e))
        })?;

        match tag {
            sync_type::SYNC_STEP1 => Ok(SyncMessage::SyncStep1(data.to_vec())),
            sync_type::SYNC_STEP2 => Ok(SyncMessage::SyncStep2(data.to_vec())),
            sync_type::UPDATE => Ok(SyncMessage::Update(data.to_vec())),
            other => Err(YSyncError::ProtocolError(format!(
                "Unknown sync message type: {}",
                other
            ))),
        }
    }

    /// Encode this sync message to a writer.
    pub fn encode_to<W: Write>(&self, writer: &mut W) {
        match self {
            SyncMessage::SyncStep1(data) => {
                writer.write_var(sync_type::SYNC_STEP1);
                writer.write_buf(data);
            }
            SyncMessage::SyncStep2(data) => {
                writer.write_var(sync_type::SYNC_STEP2);
                writer.write_buf(data);
            }
            SyncMessage::Update(data) => {
                writer.write_var(sync_type::UPDATE);
                writer.write_buf(data);
            }
        }
    }

    /// Parse the state vector from a SyncStep1 message.
    pub fn parse_state_vector(&self) -> Result<StateVector> {
        match self {
            SyncMessage::SyncStep1(data) => StateVector::decode_v1(data).map_err(|e| {
                YSyncError::ProtocolError(format!("Failed to decode state vector: {}", e))
            }),
            _ => Err(YSyncError::ProtocolError(
                "Expected SyncStep1 message".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_step1_roundtrip() {
        let sv = StateVector::default();
        let msg = Message::sync_step1(&sv);
        let encoded = msg.encode();
        let decoded = Message::decode(&encoded).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn test_sync_step2_roundtrip() {
        let update = vec![1, 2, 3, 4, 5];
        let msg = Message::sync_step2(update.clone());
        let encoded = msg.encode();
        let decoded = Message::decode(&encoded).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn test_update_roundtrip() {
        let update = vec![10, 20, 30, 40];
        let msg = Message::update(update.clone());
        let encoded = msg.encode();
        let decoded = Message::decode(&encoded).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn test_awareness_query_roundtrip() {
        let msg = Message::AwarenessQuery;
        let encoded = msg.encode();
        let decoded = Message::decode(&encoded).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn test_auth_with_reason_roundtrip() {
        let msg = Message::Auth(Some("permission denied".to_string()));
        let encoded = msg.encode();
        let decoded = Message::decode(&encoded).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn test_auth_without_reason_roundtrip() {
        let msg = Message::Auth(None);
        let encoded = msg.encode();
        let decoded = Message::decode(&encoded).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn test_custom_message_roundtrip() {
        let msg = Message::Custom(42, vec![1, 2, 3]);
        let encoded = msg.encode();
        let decoded = Message::decode(&encoded).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn test_message_type_tags() {
        // Verify message type tags match expected values
        assert_eq!(message_type::SYNC, 0);
        assert_eq!(message_type::AWARENESS, 1);
        assert_eq!(message_type::AUTH, 2);
        assert_eq!(message_type::AWARENESS_QUERY, 3);
    }

    #[test]
    fn test_sync_type_tags() {
        // Verify sync type tags match expected values
        assert_eq!(sync_type::SYNC_STEP1, 0);
        assert_eq!(sync_type::SYNC_STEP2, 1);
        assert_eq!(sync_type::UPDATE, 2);
    }
}

//! Sync protocol state machine for document synchronization.
//!
//! This module provides the sync protocol handler that manages the
//! connection state and processes sync messages.

use yrs::updates::decoder::Decode;
use yrs::{Doc, ReadTxn, StateVector, Transact, Update};

use crate::error::{Result, YSyncError};

use super::message::{Message, SyncMessage};

/// The state of a sync connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SyncState {
    /// Initial state, no sync has occurred.
    #[default]
    Pending,
    /// SyncStep1 has been sent, waiting for SyncStep2.
    SyncStep1Sent,
    /// Initial sync complete, now in update mode.
    Synced,
}

/// Protocol handler for Y-sync document synchronization.
///
/// This handles the sync protocol state machine and processes
/// incoming messages to keep a Y.Doc in sync with remote peers.
#[derive(Debug)]
pub struct SyncProtocol {
    state: SyncState,
}

impl Default for SyncProtocol {
    fn default() -> Self {
        Self::new()
    }
}

impl SyncProtocol {
    /// Create a new sync protocol handler.
    pub fn new() -> Self {
        Self {
            state: SyncState::Pending,
        }
    }

    /// Get the current sync state.
    pub fn state(&self) -> SyncState {
        self.state
    }

    /// Check if the initial sync is complete.
    pub fn is_synced(&self) -> bool {
        self.state == SyncState::Synced
    }

    /// Generate the initial SyncStep1 message to start synchronization.
    ///
    /// Call this when connecting to a peer to initiate the sync protocol.
    pub fn start(&mut self, doc: &Doc) -> Message {
        let txn = doc.transact();
        let sv = txn.state_vector();
        self.state = SyncState::SyncStep1Sent;
        Message::sync_step1(&sv)
    }

    /// Handle an incoming sync message and generate a response if needed.
    ///
    /// Returns a list of messages to send in response.
    pub fn handle_sync_message(&mut self, doc: &Doc, msg: &SyncMessage) -> Result<Vec<Message>> {
        match msg {
            SyncMessage::SyncStep1(sv_data) => self.handle_sync_step1(doc, sv_data),
            SyncMessage::SyncStep2(update_data) => self.handle_sync_step2(doc, update_data),
            SyncMessage::Update(update_data) => self.handle_update(doc, update_data),
        }
    }

    /// Handle a SyncStep1 message (remote peer's state vector).
    ///
    /// Computes the diff and returns SyncStep2 with missing updates,
    /// plus our own SyncStep1 if not already synced.
    fn handle_sync_step1(&mut self, doc: &Doc, sv_data: &[u8]) -> Result<Vec<Message>> {
        let remote_sv = StateVector::decode_v1(sv_data).map_err(|e| {
            YSyncError::ProtocolError(format!("Failed to decode state vector: {}", e))
        })?;

        let mut responses = Vec::new();

        // Compute what the remote is missing and send SyncStep2
        let txn = doc.transact();
        let update = txn.encode_state_as_update_v1(&remote_sv);
        responses.push(Message::sync_step2(update));

        // If we haven't started sync yet, also send our SyncStep1
        if self.state == SyncState::Pending {
            let sv = txn.state_vector();
            responses.push(Message::sync_step1(&sv));
            self.state = SyncState::SyncStep1Sent;
        }

        Ok(responses)
    }

    /// Handle a SyncStep2 message (updates we're missing).
    ///
    /// Applies the updates to the local document.
    fn handle_sync_step2(&mut self, doc: &Doc, update_data: &[u8]) -> Result<Vec<Message>> {
        self.apply_update(doc, update_data)?;
        self.state = SyncState::Synced;
        Ok(vec![])
    }

    /// Handle an Update message (incremental changes).
    ///
    /// Applies the update to the local document.
    fn handle_update(&mut self, doc: &Doc, update_data: &[u8]) -> Result<Vec<Message>> {
        self.apply_update(doc, update_data)?;
        Ok(vec![])
    }

    /// Apply an encoded update to the document.
    fn apply_update(&self, doc: &Doc, update_data: &[u8]) -> Result<()> {
        if update_data.is_empty() {
            return Ok(());
        }

        let update = Update::decode_v1(update_data)
            .map_err(|e| YSyncError::ProtocolError(format!("Failed to decode update: {}", e)))?;

        let mut txn = doc.transact_mut();
        txn.apply_update(update)
            .map_err(|e| YSyncError::ProtocolError(format!("Failed to apply update: {}", e)))?;

        Ok(())
    }

    /// Create an update message from local changes.
    ///
    /// Call this after making local changes to broadcast them to peers.
    pub fn encode_update(doc: &Doc, since: &StateVector) -> Message {
        let txn = doc.transact();
        let update = txn.encode_state_as_update_v1(since);
        Message::update(update)
    }

    /// Create an update message containing the full document state.
    ///
    /// Useful for sending to a new peer that has no prior state.
    pub fn encode_full_update(doc: &Doc) -> Message {
        let txn = doc.transact();
        let update = txn.encode_state_as_update_v1(&StateVector::default());
        Message::update(update)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yrs::updates::encoder::Encode;
    use yrs::{Array, Transact, WriteTxn};

    fn create_test_doc() -> Doc {
        let doc = Doc::new();
        {
            let mut txn = doc.transact_mut();
            txn.get_or_insert_array("cells");
            txn.get_or_insert_map("metadata");
        }
        doc
    }

    #[test]
    fn test_sync_protocol_initial_state() {
        let protocol = SyncProtocol::new();
        assert_eq!(protocol.state(), SyncState::Pending);
        assert!(!protocol.is_synced());
    }

    #[test]
    fn test_sync_protocol_start() {
        let mut protocol = SyncProtocol::new();
        let doc = create_test_doc();

        let msg = protocol.start(&doc);
        assert_eq!(protocol.state(), SyncState::SyncStep1Sent);

        match msg {
            Message::Sync(SyncMessage::SyncStep1(_)) => {}
            _ => panic!("Expected SyncStep1 message"),
        }
    }

    #[test]
    fn test_sync_protocol_handle_sync_step1() {
        let mut protocol = SyncProtocol::new();
        let doc = create_test_doc();
        let remote_sv = StateVector::default();

        let sv_data = remote_sv.encode_v1();
        let responses = protocol
            .handle_sync_message(&doc, &SyncMessage::SyncStep1(sv_data))
            .unwrap();

        // Should respond with SyncStep2 and SyncStep1
        assert_eq!(responses.len(), 2);
        assert!(matches!(
            &responses[0],
            Message::Sync(SyncMessage::SyncStep2(_))
        ));
        assert!(matches!(
            &responses[1],
            Message::Sync(SyncMessage::SyncStep1(_))
        ));
        assert_eq!(protocol.state(), SyncState::SyncStep1Sent);
    }

    #[test]
    fn test_sync_protocol_handle_sync_step2() {
        let mut protocol = SyncProtocol::new();
        protocol.state = SyncState::SyncStep1Sent;

        let doc = create_test_doc();

        // Create an empty update (drop txn before calling handle_sync_message)
        let update = {
            let empty_sv = StateVector::default();
            let txn = doc.transact();
            txn.encode_state_as_update_v1(&empty_sv)
        };

        let responses = protocol
            .handle_sync_message(&doc, &SyncMessage::SyncStep2(update))
            .unwrap();

        assert!(responses.is_empty());
        assert_eq!(protocol.state(), SyncState::Synced);
        assert!(protocol.is_synced());
    }

    #[test]
    fn test_sync_protocol_handle_update() {
        let mut protocol = SyncProtocol::new();
        protocol.state = SyncState::Synced;

        let doc1 = create_test_doc();
        let doc2 = create_test_doc();

        // Make a change in doc1
        {
            let mut txn = doc1.transact_mut();
            let cells = txn.get_or_insert_array("cells");
            cells.push_back(&mut txn, "test");
        }

        // Get update from doc1
        let txn = doc1.transact();
        let update = txn.encode_state_as_update_v1(&StateVector::default());

        // Apply to doc2 via protocol
        let responses = protocol
            .handle_sync_message(&doc2, &SyncMessage::Update(update))
            .unwrap();

        assert!(responses.is_empty());

        // Verify doc2 has the change
        let txn = doc2.transact();
        let cells = txn.get_array("cells").unwrap();
        assert_eq!(cells.len(&txn), 1);
    }

    #[test]
    fn test_two_doc_sync() {
        let doc1 = create_test_doc();
        let doc2 = create_test_doc();

        // Make changes in doc1
        {
            let mut txn = doc1.transact_mut();
            let cells = txn.get_or_insert_array("cells");
            cells.push_back(&mut txn, "cell1");
            cells.push_back(&mut txn, "cell2");
        }

        let mut protocol1 = SyncProtocol::new();
        let mut protocol2 = SyncProtocol::new();

        // doc1 initiates sync
        let step1_msg = protocol1.start(&doc1);
        let Message::Sync(sync_msg) = step1_msg else {
            panic!("Expected Sync message")
        };

        // doc2 handles SyncStep1 and responds
        let responses = protocol2.handle_sync_message(&doc2, &sync_msg).unwrap();

        // Process responses on doc1
        for resp in responses {
            let Message::Sync(sync_msg) = resp else {
                continue;
            };
            protocol1.handle_sync_message(&doc1, &sync_msg).unwrap();
        }

        // Now doc2 should have doc1's state
        // Get doc1's update and send to doc2
        let txn1 = doc1.transact();
        let sv2 = doc2.transact().state_vector();
        let update = txn1.encode_state_as_update_v1(&sv2);

        protocol2
            .handle_sync_message(&doc2, &SyncMessage::SyncStep2(update))
            .unwrap();

        // Verify both docs have the same content
        let txn2 = doc2.transact();
        let cells2 = txn2.get_array("cells").unwrap();
        assert_eq!(cells2.len(&txn2), 2);
    }
}

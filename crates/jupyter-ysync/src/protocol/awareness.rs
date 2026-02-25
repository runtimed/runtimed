//! Awareness protocol for tracking user presence.
//!
//! The awareness protocol allows clients to share ephemeral state that
//! doesn't need conflict resolution, such as:
//! - Cursor positions
//! - Selection ranges
//! - User names and colors
//! - Online/offline status
//!
//! Unlike Y.Doc state, awareness data is not persisted and uses a simple
//! clock-based update mechanism with a 30-second timeout for detecting
//! offline clients.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
pub use yrs::sync::awareness::Event as AwarenessEvent;
use yrs::sync::awareness::{Awareness, AwarenessUpdate};
use yrs::Doc;

use crate::error::{Result, YSyncError};

/// Default timeout for considering a client offline (30 seconds).
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// User awareness state for collaborative editing.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AwarenessState {
    /// User's display name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<UserInfo>,

    /// Current cursor position.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<CursorPosition>,

    /// Current selection range.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selection: Option<SelectionRange>,
}

/// User identification information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    /// Display name.
    pub name: String,

    /// User color (e.g., for cursor highlighting).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,

    /// User avatar URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
}

/// Cursor position within a cell.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPosition {
    /// Cell index or ID.
    pub cell: String,

    /// Character offset within the cell source.
    pub offset: u32,
}

/// Selection range within a cell.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionRange {
    /// Cell index or ID.
    pub cell: String,

    /// Start offset of selection.
    pub start: u32,

    /// End offset of selection.
    pub end: u32,
}

/// Wrapper around yrs Awareness with Jupyter-specific functionality.
pub struct ClientAwareness {
    inner: Awareness,
    timeout: Duration,
    last_update: HashMap<u64, Instant>,
}

impl ClientAwareness {
    /// Create a new awareness instance for a document.
    pub fn new(doc: &Doc) -> Self {
        Self {
            inner: Awareness::new(doc.clone()),
            timeout: DEFAULT_TIMEOUT,
            last_update: HashMap::new(),
        }
    }

    /// Create a new awareness instance with a custom timeout.
    pub fn with_timeout(doc: &Doc, timeout: Duration) -> Self {
        Self {
            inner: Awareness::new(doc.clone()),
            timeout,
            last_update: HashMap::new(),
        }
    }

    /// Get the local client ID.
    pub fn client_id(&self) -> u64 {
        self.inner.client_id()
    }

    /// Get a reference to the inner yrs Awareness.
    pub fn inner(&self) -> &Awareness {
        &self.inner
    }

    /// Get a mutable reference to the inner yrs Awareness.
    pub fn inner_mut(&mut self) -> &mut Awareness {
        &mut self.inner
    }

    /// Set the local client's awareness state.
    pub fn set_local_state(&mut self, state: &AwarenessState) -> Result<()> {
        self.inner
            .set_local_state(state)
            .map_err(|e| YSyncError::ProtocolError(e.to_string()))?;
        Ok(())
    }

    /// Clear the local client's awareness state (mark as offline).
    pub fn clear_local_state(&mut self) {
        self.inner.clean_local_state();
    }

    /// Get the local client's awareness state.
    pub fn local_state(&self) -> Option<AwarenessState> {
        self.inner.local_state()
    }

    /// Get a specific client's awareness state by iterating over states.
    pub fn get_state(&self, target_client_id: u64) -> Option<AwarenessState> {
        for (client_id, client_state) in self.inner.iter() {
            if client_id == target_client_id {
                if let Some(ref json_str) = client_state.data {
                    return serde_json::from_str(json_str).ok();
                }
            }
        }
        None
    }

    /// Get all connected clients' awareness states.
    pub fn get_all_states(&self) -> HashMap<u64, AwarenessState> {
        let mut result = HashMap::new();
        for (client_id, client_state) in self.inner.iter() {
            if let Some(ref json_str) = client_state.data {
                if let Ok(state) = serde_json::from_str(json_str) {
                    result.insert(client_id, state);
                }
            }
        }
        result
    }

    /// Get all client IDs.
    pub fn client_ids(&self) -> Vec<u64> {
        self.inner.iter().map(|(client_id, _)| client_id).collect()
    }

    /// Apply an awareness update from a remote peer.
    pub fn apply_update(&mut self, update: AwarenessUpdate) -> Result<()> {
        // Track update time for timeout detection
        let now = Instant::now();
        for &client_id in update.clients.keys() {
            self.last_update.insert(client_id, now);
        }

        self.inner
            .apply_update(update)
            .map_err(|e| YSyncError::ProtocolError(e.to_string()))?;
        Ok(())
    }

    /// Encode the full awareness state as an update.
    pub fn encode_update(&self) -> Result<AwarenessUpdate> {
        let client_ids: Vec<u64> = self.client_ids();
        self.inner
            .update_with_clients(client_ids)
            .map_err(|e| YSyncError::ProtocolError(e.to_string()))
    }

    /// Encode an awareness update for specific clients.
    pub fn encode_update_for_clients(&self, client_ids: Vec<u64>) -> Result<AwarenessUpdate> {
        self.inner
            .update_with_clients(client_ids)
            .map_err(|e| YSyncError::ProtocolError(e.to_string()))
    }

    /// Check for and remove timed-out clients.
    ///
    /// Returns the client IDs that were removed.
    pub fn remove_timed_out_clients(&mut self) -> Vec<u64> {
        let now = Instant::now();
        let timed_out: Vec<u64> = self
            .last_update
            .iter()
            .filter(|(_, &last)| now.duration_since(last) > self.timeout)
            .map(|(&id, _)| id)
            .collect();

        for &client_id in &timed_out {
            self.last_update.remove(&client_id);
            self.inner.remove_state(client_id);
        }

        timed_out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_doc() -> Doc {
        Doc::new()
    }

    #[test]
    fn test_awareness_state_serialization() {
        let state = AwarenessState {
            user: Some(UserInfo {
                name: "Alice".to_string(),
                color: Some("#ff0000".to_string()),
                avatar: None,
            }),
            cursor: Some(CursorPosition {
                cell: "cell-1".to_string(),
                offset: 42,
            }),
            selection: None,
        };

        let json = serde_json::to_string(&state).unwrap();
        let parsed: AwarenessState = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.user.as_ref().unwrap().name, "Alice");
        assert_eq!(parsed.cursor.as_ref().unwrap().offset, 42);
    }

    #[test]
    fn test_client_awareness_new() {
        let doc = create_test_doc();
        let awareness = ClientAwareness::new(&doc);
        assert!(awareness.client_id() > 0);
    }

    #[test]
    fn test_set_and_get_local_state() {
        let doc = create_test_doc();
        let mut awareness = ClientAwareness::new(&doc);

        let state = AwarenessState {
            user: Some(UserInfo {
                name: "Bob".to_string(),
                color: None,
                avatar: None,
            }),
            cursor: None,
            selection: None,
        };

        awareness.set_local_state(&state).unwrap();
        let retrieved = awareness.local_state().unwrap();
        assert_eq!(retrieved.user.as_ref().unwrap().name, "Bob");
    }

    #[test]
    fn test_get_all_states() {
        let doc = create_test_doc();
        let mut awareness = ClientAwareness::new(&doc);

        let state = AwarenessState {
            user: Some(UserInfo {
                name: "Charlie".to_string(),
                color: Some("#00ff00".to_string()),
                avatar: None,
            }),
            cursor: None,
            selection: None,
        };

        awareness.set_local_state(&state).unwrap();
        let all_states = awareness.get_all_states();

        assert_eq!(all_states.len(), 1);
        let (&client_id, state) = all_states.iter().next().unwrap();
        assert_eq!(client_id, awareness.client_id());
        assert_eq!(state.user.as_ref().unwrap().name, "Charlie");
    }

    #[test]
    fn test_clear_local_state() {
        let doc = create_test_doc();
        let mut awareness = ClientAwareness::new(&doc);

        let state = AwarenessState {
            user: Some(UserInfo {
                name: "David".to_string(),
                color: None,
                avatar: None,
            }),
            cursor: None,
            selection: None,
        };

        awareness.set_local_state(&state).unwrap();
        assert!(awareness.local_state().is_some());

        awareness.clear_local_state();
        assert!(awareness.local_state().is_none());
    }

    #[test]
    fn test_encode_update() {
        let doc = create_test_doc();
        let mut awareness = ClientAwareness::new(&doc);

        let state = AwarenessState {
            user: Some(UserInfo {
                name: "Eve".to_string(),
                color: None,
                avatar: None,
            }),
            cursor: None,
            selection: None,
        };

        awareness.set_local_state(&state).unwrap();
        let update = awareness.encode_update().unwrap();

        // Update should contain our client
        assert!(update.clients.contains_key(&awareness.client_id()));
    }

    #[test]
    fn test_awareness_update_roundtrip() {
        let doc1 = create_test_doc();
        let doc2 = create_test_doc();

        let mut awareness1 = ClientAwareness::new(&doc1);
        let mut awareness2 = ClientAwareness::new(&doc2);

        // Set state on awareness1
        let state = AwarenessState {
            user: Some(UserInfo {
                name: "Frank".to_string(),
                color: Some("#0000ff".to_string()),
                avatar: None,
            }),
            cursor: Some(CursorPosition {
                cell: "cell-0".to_string(),
                offset: 10,
            }),
            selection: None,
        };

        awareness1.set_local_state(&state).unwrap();

        // Get update and apply to awareness2
        let update = awareness1.encode_update().unwrap();
        awareness2.apply_update(update).unwrap();

        // Verify awareness2 can see awareness1's state
        let states = awareness2.get_all_states();
        assert!(!states.is_empty());

        let client1_state = awareness2.get_state(awareness1.client_id()).unwrap();
        assert_eq!(client1_state.user.as_ref().unwrap().name, "Frank");
    }
}

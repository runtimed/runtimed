//! # jupyter-ysync
//!
//! Y-sync protocol implementation for Jupyter notebook collaboration.
//!
//! This crate provides CRDT-based document synchronization for Jupyter notebooks
//! using the [yrs](https://docs.rs/yrs) library (Rust port of Y.js). The protocol
//! is compatible with [jupyter-server-documents](https://github.com/jupyter-ai-contrib/jupyter-server-documents)
//! and enables multi-client collaborative editing.
//!
//! ## Features
//!
//! - **NotebookDoc**: CRDT document representation of Jupyter notebooks
//! - **Conversion**: Bidirectional conversion between `nbformat::v4::Notebook` and Y.Doc
//! - **Character-level editing**: Cell sources use Y.Text for fine-grained collaboration
//!
//! ### Client Feature (Beta)
//!
//! With the `client` feature enabled, this crate provides:
//!
//! - **YSyncClient**: WebSocket client for connecting to Y-sync servers
//! - **NotebookSession**: High-level API combining Y-sync with kernel execution
//!
//! **Note**: The client feature is experimental. The notebook must be open in
//! JupyterLab for the collaboration room to be active. See the `nb` example.
//!
//! ## Example
//!
//! ```rust
//! use jupyter_ysync::{NotebookDoc, notebook_to_ydoc, ydoc_to_notebook};
//! use nbformat::v4::Notebook;
//!
//! // Convert an existing notebook to a collaborative document
//! // let doc = notebook_to_ydoc(&notebook)?;
//!
//! // Make edits via the Y.Doc API
//! // ...
//!
//! // Convert back to nbformat for saving
//! // let updated_notebook = ydoc_to_notebook(&doc)?;
//! ```

pub mod convert;
pub mod doc;
pub mod error;
pub mod executor;
pub mod output_mapping;
pub mod protocol;

#[cfg(feature = "client")]
pub mod client;

#[cfg(feature = "client")]
pub mod session;

#[cfg(feature = "python")]
pub mod python;

pub use convert::{any_to_json, json_to_any, notebook_to_ydoc, output_to_any, ydoc_to_notebook};
pub use doc::{cell_types, keys, NotebookDoc};
pub use error::{Result, YSyncError};
pub use output_mapping::{
    display_data_to_output, error_to_output, execute_result_to_output, is_output_message,
    message_to_kernel_output, stream_to_output, KernelOutput,
};
pub use executor::{CellExecutor, ExecutionEvent};
pub use protocol::{AwarenessState, ClientAwareness, Message, SyncMessage, SyncProtocol, SyncState};

#[cfg(feature = "client")]
pub use client::{build_room_url, ClientConfig, RoomId, YSyncClient};

#[cfg(feature = "client")]
pub use session::{NotebookSession, SessionConfig};

// Re-export for Python bindings
#[cfg(feature = "python")]
pub use python::jupyter_ysync;

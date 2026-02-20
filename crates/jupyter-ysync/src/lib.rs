//! # jupyter-ysync
//!
//! > ⚠️ **Early Alpha** — This crate is in early alpha. The API is unstable and will change.
//! > It is being developed alongside conformance testing against
//! > [jupyter-server-documents](https://github.com/jupyter-ai-contrib/jupyter-server-documents)
//! > and [jupyter-collaboration](https://github.com/jupyterlab/jupyter-collaboration).
//!
//! Y-sync protocol implementation for Jupyter notebook collaboration, built on
//! [yrs](https://docs.rs/yrs) (Rust port of Y.js).
//!
//! ## What it does
//!
//! - **NotebookDoc**: CRDT document representation of Jupyter notebooks
//! - **Conversion**: Bidirectional conversion between `nbformat::v4::Notebook` and Y.Doc
//! - **Character-level editing**: Cell sources use Y.Text for fine-grained collaboration
//! - **Protocol**: Y-sync v1 message encoding/decoding and sync state machine
//!
//! ## Features
//!
//! | Feature | Description |
//! |---------|-------------|
//! | *(default)* | Core document types, conversion, and protocol |
//! | `client` | WebSocket client for connecting to Y-sync servers (`YSyncClient`, `NotebookSession`) |
//! | `python` | PyO3 bindings for conformance testing against `jupyter_ydoc` and `pycrdt` |
//!
//! ## Conformance status
//!
//! This crate is being tested for compatibility with the Jupyter CRDT ecosystem:
//!
//! - **jupyter_ydoc** — Y.Doc schema compatibility (cell structure, metadata layout, shared types)
//! - **pycrdt** — Update and state vector encoding roundtrips
//! - **jupyter-server-documents** — WebSocket sync protocol (via the `client` feature)
//!
//! Conformance is partial and actively in progress. Expect breaking changes.
//!
//! ## Example
//!
//! ```rust,no_run
//! use jupyter_ysync::{NotebookDoc, notebook_to_ydoc, ydoc_to_notebook};
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
pub use executor::{CellExecutor, ExecutionEvent};
pub use output_mapping::{
    display_data_to_output, error_to_output, execute_result_to_output, is_output_message,
    message_to_kernel_output, stream_to_output, KernelOutput,
};
pub use protocol::{
    AwarenessState, ClientAwareness, Message, SyncMessage, SyncProtocol, SyncState,
};

#[cfg(feature = "client")]
pub use client::{build_room_url, ClientConfig, RoomId, YSyncClient};

#[cfg(feature = "client")]
pub use session::{NotebookSession, SessionConfig};

// Re-export for Python bindings
#[cfg(feature = "python")]
pub use python::jupyter_ysync;

//! Python bindings for jupyter-ysync using PyO3.
//!
//! These bindings expose the core NotebookDoc functionality to Python,
//! enabling conformance testing against jupyter_ydoc and pycrdt.

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use yrs::types::ToJson;
use yrs::updates::encoder::Encode;
use yrs::{ReadTxn, Transact};

use crate::doc::NotebookDoc as RustNotebookDoc;

/// A CRDT-based notebook document.
///
/// This is a Python wrapper around the Rust NotebookDoc implementation,
/// providing interoperability with jupyter_ydoc and pycrdt.
#[pyclass(name = "NotebookDoc")]
pub struct PyNotebookDoc {
    inner: RustNotebookDoc,
}

#[pymethods]
impl PyNotebookDoc {
    /// Create a new empty notebook document.
    #[new]
    pub fn new() -> Self {
        Self {
            inner: RustNotebookDoc::new(),
        }
    }

    /// Get the number of cells in the notebook.
    pub fn cell_count(&self) -> u32 {
        self.inner.cell_count()
    }

    /// Get the state vector as bytes.
    ///
    /// The state vector represents the current state of the document
    /// and is used during synchronization to determine what updates
    /// are needed.
    pub fn get_state_vector<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        let txn = self.inner.doc().transact();
        let sv = txn.state_vector();
        let encoded = sv.encode_v1();
        PyBytes::new(py, &encoded)
    }

    /// Encode the full document state as an update.
    ///
    /// This can be used to transfer the entire document to another client.
    pub fn get_update<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        let update = self.inner.encode_state_as_update();
        PyBytes::new(py, &update)
    }

    /// Apply an update from another client.
    ///
    /// This merges changes from another document into this one.
    /// The update should be in Y.js v1 update format.
    pub fn apply_update(&mut self, update: &[u8]) -> PyResult<()> {
        self.inner
            .apply_update(update)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Get the document state as JSON.
    ///
    /// Returns a JSON representation of the entire document,
    /// useful for debugging and comparison.
    pub fn to_json(&self) -> PyResult<String> {
        let txn = self.inner.doc().transact();

        // Get cells array
        let cells = self.inner.cells(&txn);
        let cells_json = cells.to_json(&txn);

        // Get metadata map
        let metadata = self.inner.metadata(&txn);
        let metadata_json = metadata.to_json(&txn);

        // Build combined JSON
        let doc_json = serde_json::json!({
            "cells": crate::convert::any_to_json(&cells_json),
            "metadata": crate::convert::any_to_json(&metadata_json),
        });

        serde_json::to_string_pretty(&doc_json)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Get the client ID of this document.
    pub fn client_id(&self) -> u64 {
        self.inner.doc().client_id()
    }
}

/// Python module for jupyter_ysync.
// Preserve the existing GIL requirement; free-threaded support needs separate validation.
#[pymodule(gil_used = true)]
pub fn jupyter_ysync(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyNotebookDoc>()?;
    Ok(())
}

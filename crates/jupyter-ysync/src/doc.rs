use std::collections::HashMap;

use nbformat::v4::Output;
use yrs::updates::decoder::Decode;
use yrs::{
    Any, Array, ArrayPrelim, ArrayRef, Doc, GetString, Map, MapPrelim, MapRef, Out, ReadTxn,
    TextPrelim, TextRef, Transact, Update, WriteTxn,
};

use crate::convert::output_to_any;
use crate::error::{Result, YSyncError};

/// Y.Doc schema keys for notebook structure
pub mod keys {
    pub const CELLS: &str = "cells";
    pub const METADATA: &str = "metadata";

    // Cell fields
    pub const ID: &str = "id";
    pub const CELL_TYPE: &str = "cell_type";
    pub const SOURCE: &str = "source";
    pub const CELL_METADATA: &str = "metadata";
    pub const OUTPUTS: &str = "outputs";
    pub const EXECUTION_COUNT: &str = "execution_count";
    pub const ATTACHMENTS: &str = "attachments";
}

/// Cell type constants matching nbformat
pub mod cell_types {
    pub const CODE: &str = "code";
    pub const MARKDOWN: &str = "markdown";
    pub const RAW: &str = "raw";
}

/// A CRDT-based notebook document using Yrs (Rust Y.js implementation).
///
/// The schema follows jupyter-server-documents format:
/// ```text
/// Y.Doc {
///   cells: Y.Array<Y.Map{
///     id: string,
///     cell_type: "code" | "markdown" | "raw",
///     source: Y.Text,
///     metadata: Y.Map,
///     outputs: Y.Array (for code cells),
///     execution_count: number | null (for code cells)
///   }>,
///   metadata: Y.Map
/// }
/// ```
#[derive(Debug)]
pub struct NotebookDoc {
    doc: Doc,
}

impl NotebookDoc {
    /// Create a new empty notebook document.
    pub fn new() -> Self {
        let doc = Doc::new();

        // Initialize the root structure
        {
            let mut txn = doc.transact_mut();
            // Create cells array
            txn.get_or_insert_array(keys::CELLS);
            // Create metadata map
            txn.get_or_insert_map(keys::METADATA);
        }

        Self { doc }
    }

    /// Create a NotebookDoc from an existing Y.Doc.
    pub fn from_doc(doc: Doc) -> Self {
        Self { doc }
    }

    /// Get a reference to the underlying Y.Doc.
    pub fn doc(&self) -> &Doc {
        &self.doc
    }

    /// Get a mutable reference to the underlying Y.Doc.
    pub fn doc_mut(&mut self) -> &mut Doc {
        &mut self.doc
    }

    /// Get the cells array reference.
    ///
    /// # Panics
    /// Panics if the cells array doesn't exist (shouldn't happen if doc was created via `new()`).
    pub fn cells<T: ReadTxn>(&self, txn: &T) -> ArrayRef {
        txn.get_array(keys::CELLS)
            .expect("cells array should exist")
    }

    /// Get the metadata map reference.
    ///
    /// # Panics
    /// Panics if the metadata map doesn't exist (shouldn't happen if doc was created via `new()`).
    pub fn metadata<T: ReadTxn>(&self, txn: &T) -> MapRef {
        txn.get_map(keys::METADATA)
            .expect("metadata map should exist")
    }

    /// Get the number of cells in the notebook.
    pub fn cell_count(&self) -> u32 {
        let txn = self.doc.transact();
        self.cells(&txn).len(&txn)
    }

    /// Add a new cell to the notebook.
    ///
    /// Returns the index of the newly added cell.
    pub fn add_cell(
        &self,
        id: &str,
        cell_type: &str,
        source: &str,
        index: Option<u32>,
    ) -> Result<u32> {
        let mut txn = self.doc.transact_mut();
        let cells = self.cells(&txn);
        let insert_index = index.unwrap_or_else(|| cells.len(&txn));

        // Build the cell as a nested map structure
        // Note: source, metadata, and outputs are added as CRDT types after insertion
        let mut cell_content: HashMap<String, Any> = HashMap::new();
        cell_content.insert(keys::ID.into(), Any::String(id.into()));
        cell_content.insert(keys::CELL_TYPE.into(), Any::String(cell_type.into()));

        // Add execution_count for code cells
        if cell_type == cell_types::CODE {
            cell_content.insert(keys::EXECUTION_COUNT.into(), Any::Null);
        }

        // Insert the cell into the cells array
        let cell_prelim = MapPrelim::from_iter(cell_content);
        cells.insert(&mut txn, insert_index, cell_prelim);

        // Get reference to the inserted cell to add CRDT types
        if let Some(Out::YMap(cell_map)) = cells.get(&txn, insert_index) {
            // Add source as Y.Text (required for collaborative editing)
            cell_map.insert(&mut txn, keys::SOURCE, TextPrelim::new(source));

            // Add metadata as Y.Map (JupyterLab expects this to be a Y.Map, not plain object)
            cell_map.insert(&mut txn, keys::CELL_METADATA, MapPrelim::default());

            // For code cells, add outputs as Y.Array
            if cell_type == cell_types::CODE {
                cell_map.insert(&mut txn, keys::OUTPUTS, ArrayPrelim::default());
            }
        }

        Ok(insert_index)
    }

    /// Get a cell by index.
    pub fn get_cell(&self, index: u32) -> Option<CellView> {
        let txn = self.doc.transact();
        let cells = self.cells(&txn);

        if index >= cells.len(&txn) {
            return None;
        }

        cells.get(&txn, index).and_then(|value| {
            if let Out::YMap(map_ref) = value {
                Some(CellView { map: map_ref })
            } else {
                None
            }
        })
    }

    /// Remove a cell by index.
    pub fn remove_cell(&self, index: u32) -> Result<()> {
        let mut txn = self.doc.transact_mut();
        let cells = self.cells(&txn);

        if index >= cells.len(&txn) {
            return Err(YSyncError::ConversionError(format!(
                "Cell index {} out of bounds",
                index
            )));
        }

        cells.remove(&mut txn, index);
        Ok(())
    }

    /// Get the state vector for synchronization.
    pub fn state_vector(&self) -> yrs::StateVector {
        let txn = self.doc.transact();
        txn.state_vector()
    }

    /// Encode the document state as an update.
    pub fn encode_state_as_update(&self) -> Vec<u8> {
        let txn = self.doc.transact();
        txn.encode_state_as_update_v1(&yrs::StateVector::default())
    }

    /// Apply an update from another client.
    pub fn apply_update(&mut self, update: &[u8]) -> Result<()> {
        let update =
            Update::decode_v1(update).map_err(|e| YSyncError::TransactionError(e.to_string()))?;

        let mut txn = self.doc.transact_mut();
        txn.apply_update(update)
            .map_err(|e| YSyncError::TransactionError(e.to_string()))?;

        Ok(())
    }

    /// Clear all outputs from a code cell.
    ///
    /// Returns an error if the cell doesn't exist or isn't a code cell.
    pub fn clear_cell_outputs(&self, cell_index: u32) -> Result<()> {
        let mut txn = self.doc.transact_mut();
        let cells = self.cells(&txn);

        if cell_index >= cells.len(&txn) {
            return Err(YSyncError::ConversionError(format!(
                "Cell index {} out of bounds",
                cell_index
            )));
        }

        let cell_value = cells.get(&txn, cell_index).ok_or_else(|| {
            YSyncError::ConversionError(format!("Cell {} not found", cell_index))
        })?;

        let Out::YMap(cell_map) = cell_value else {
            return Err(YSyncError::ConversionError("Cell is not a map".into()));
        };

        // Get the outputs array
        let outputs_value = cell_map.get(&txn, keys::OUTPUTS).ok_or_else(|| {
            YSyncError::ConversionError("Cell has no outputs (not a code cell?)".into())
        })?;

        let Out::YArray(outputs) = outputs_value else {
            return Err(YSyncError::ConversionError(
                "Outputs is not an array".into(),
            ));
        };

        // Clear all outputs
        let len = outputs.len(&txn);
        if len > 0 {
            outputs.remove_range(&mut txn, 0, len);
        }

        Ok(())
    }

    /// Append an output to a code cell.
    ///
    /// Returns an error if the cell doesn't exist or isn't a code cell.
    pub fn append_output(&self, cell_index: u32, output: &Output) -> Result<()> {
        let mut txn = self.doc.transact_mut();
        let cells = self.cells(&txn);

        if cell_index >= cells.len(&txn) {
            return Err(YSyncError::ConversionError(format!(
                "Cell index {} out of bounds",
                cell_index
            )));
        }

        let cell_value = cells.get(&txn, cell_index).ok_or_else(|| {
            YSyncError::ConversionError(format!("Cell {} not found", cell_index))
        })?;

        let Out::YMap(cell_map) = cell_value else {
            return Err(YSyncError::ConversionError("Cell is not a map".into()));
        };

        // Get the outputs array
        let outputs_value = cell_map.get(&txn, keys::OUTPUTS).ok_or_else(|| {
            YSyncError::ConversionError("Cell has no outputs (not a code cell?)".into())
        })?;

        let Out::YArray(outputs) = outputs_value else {
            return Err(YSyncError::ConversionError(
                "Outputs is not an array".into(),
            ));
        };

        // Convert output to yrs::Any and append
        let output_any = output_to_any(output)?;
        outputs.push_back(&mut txn, output_any);

        Ok(())
    }

    /// Set the execution count for a code cell.
    ///
    /// Pass `None` to clear the execution count.
    /// Returns an error if the cell doesn't exist or isn't a code cell.
    pub fn set_execution_count(&self, cell_index: u32, count: Option<i32>) -> Result<()> {
        let mut txn = self.doc.transact_mut();
        let cells = self.cells(&txn);

        if cell_index >= cells.len(&txn) {
            return Err(YSyncError::ConversionError(format!(
                "Cell index {} out of bounds",
                cell_index
            )));
        }

        let cell_value = cells.get(&txn, cell_index).ok_or_else(|| {
            YSyncError::ConversionError(format!("Cell {} not found", cell_index))
        })?;

        let Out::YMap(cell_map) = cell_value else {
            return Err(YSyncError::ConversionError("Cell is not a map".into()));
        };

        // Check this is a code cell by looking for execution_count field
        if cell_map.get(&txn, keys::EXECUTION_COUNT).is_none() {
            return Err(YSyncError::ConversionError(
                "Cell has no execution_count (not a code cell?)".into(),
            ));
        }

        // Set the execution count
        let value = match count {
            Some(n) => Any::BigInt(n as i64),
            None => Any::Null,
        };
        cell_map.insert(&mut txn, keys::EXECUTION_COUNT, value);

        Ok(())
    }
}

impl Default for NotebookDoc {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for NotebookDoc {
    fn clone(&self) -> Self {
        let update = self.encode_state_as_update();
        let doc = Doc::new();
        {
            let mut txn = doc.transact_mut();
            if let Ok(update) = Update::decode_v1(&update) {
                let _ = txn.apply_update(update);
            }
        }
        Self { doc }
    }
}

/// A view into a cell within the Y.Doc.
#[derive(Debug)]
pub struct CellView {
    map: MapRef,
}

impl CellView {
    /// Get the cell ID.
    pub fn id<T: ReadTxn>(&self, txn: &T) -> Option<String> {
        self.map.get(txn, keys::ID).and_then(|v| match v {
            Out::Any(Any::String(s)) => Some(s.to_string()),
            _ => None,
        })
    }

    /// Get the cell type.
    pub fn cell_type<T: ReadTxn>(&self, txn: &T) -> Option<String> {
        self.map.get(txn, keys::CELL_TYPE).and_then(|v| match v {
            Out::Any(Any::String(s)) => Some(s.to_string()),
            _ => None,
        })
    }

    /// Get the cell source as a Y.Text reference.
    pub fn source<T: ReadTxn>(&self, txn: &T) -> Option<TextRef> {
        self.map.get(txn, keys::SOURCE).and_then(|v| match v {
            Out::YText(text) => Some(text),
            _ => None,
        })
    }

    /// Get the cell source as a string.
    pub fn source_string<T: ReadTxn>(&self, txn: &T) -> Option<String> {
        self.source(txn).map(|text| text.get_string(txn))
    }

    /// Get the source as a string, handling both Y.Text and plain string.
    pub fn source_as_string<T: ReadTxn>(&self, txn: &T) -> Option<String> {
        self.map.get(txn, keys::SOURCE).and_then(|v| match v {
            Out::YText(text) => Some(text.get_string(txn)),
            Out::Any(Any::String(s)) => Some(s.to_string()),
            _ => None,
        })
    }

    /// Get the cell metadata map.
    pub fn metadata<T: ReadTxn>(&self, txn: &T) -> Option<MapRef> {
        self.map.get(txn, keys::CELL_METADATA).and_then(|v| match v {
            Out::YMap(map) => Some(map),
            _ => None,
        })
    }

    /// Get the outputs array (for code cells).
    pub fn outputs<T: ReadTxn>(&self, txn: &T) -> Option<ArrayRef> {
        self.map.get(txn, keys::OUTPUTS).and_then(|v| match v {
            Out::YArray(arr) => Some(arr),
            _ => None,
        })
    }

    /// Get the execution count (for code cells).
    pub fn execution_count<T: ReadTxn>(&self, txn: &T) -> Option<Option<i32>> {
        self.map.get(txn, keys::EXECUTION_COUNT).map(|v| match v {
            Out::Any(Any::BigInt(n)) => Some(n as i32),
            Out::Any(Any::Number(n)) => Some(n as i32),
            Out::Any(Any::Null) => None,
            _ => None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_notebook_doc() {
        let doc = NotebookDoc::new();
        assert_eq!(doc.cell_count(), 0);
    }

    #[test]
    fn test_add_code_cell() {
        let doc = NotebookDoc::new();
        let index = doc
            .add_cell("cell-1", cell_types::CODE, "print('hello')", None)
            .unwrap();
        assert_eq!(index, 0);
        assert_eq!(doc.cell_count(), 1);
    }

    #[test]
    fn test_add_markdown_cell() {
        let doc = NotebookDoc::new();
        let index = doc
            .add_cell("cell-1", cell_types::MARKDOWN, "# Hello", None)
            .unwrap();
        assert_eq!(index, 0);
        assert_eq!(doc.cell_count(), 1);
    }

    #[test]
    fn test_get_cell() {
        let doc = NotebookDoc::new();
        doc.add_cell("test-id", cell_types::CODE, "x = 1", None)
            .unwrap();

        let cell = doc.get_cell(0).expect("Cell should exist");
        let txn = doc.doc().transact();

        assert_eq!(cell.id(&txn), Some("test-id".to_string()));
        assert_eq!(cell.cell_type(&txn), Some("code".to_string()));
        assert_eq!(cell.source_as_string(&txn), Some("x = 1".to_string()));
    }

    #[test]
    fn test_state_vector() {
        let doc = NotebookDoc::new();
        let sv = doc.state_vector();
        // State vector should exist even for empty doc
        assert!(!sv.is_empty() || sv.is_empty()); // Just verify it doesn't panic
    }

    #[test]
    fn test_encode_and_apply_update() {
        let doc1 = NotebookDoc::new();
        doc1.add_cell("cell-1", cell_types::CODE, "x = 1", None)
            .unwrap();

        let update = doc1.encode_state_as_update();

        let mut doc2 = NotebookDoc::new();
        doc2.apply_update(&update).unwrap();

        assert_eq!(doc2.cell_count(), doc1.cell_count());
    }

    #[test]
    fn test_remove_cell() {
        let doc = NotebookDoc::new();
        doc.add_cell("cell-1", cell_types::CODE, "x = 1", None)
            .unwrap();
        doc.add_cell("cell-2", cell_types::CODE, "y = 2", None)
            .unwrap();

        assert_eq!(doc.cell_count(), 2);
        doc.remove_cell(0).unwrap();
        assert_eq!(doc.cell_count(), 1);
    }

    #[test]
    fn test_append_output() {
        use nbformat::v4::MultilineString;

        let doc = NotebookDoc::new();
        doc.add_cell("cell-1", cell_types::CODE, "print('hello')", None)
            .unwrap();

        let output = Output::Stream {
            name: "stdout".into(),
            text: MultilineString("hello\n".into()),
        };

        doc.append_output(0, &output).unwrap();

        // Verify output was added
        let cell = doc.get_cell(0).unwrap();
        let txn = doc.doc().transact();
        let outputs = cell.outputs(&txn).unwrap();
        assert_eq!(outputs.len(&txn), 1);
    }

    #[test]
    fn test_clear_cell_outputs() {
        use nbformat::v4::MultilineString;

        let doc = NotebookDoc::new();
        doc.add_cell("cell-1", cell_types::CODE, "print('hello')", None)
            .unwrap();

        // Add some outputs
        let output = Output::Stream {
            name: "stdout".into(),
            text: MultilineString("hello\n".into()),
        };
        doc.append_output(0, &output).unwrap();
        doc.append_output(0, &output).unwrap();

        // Verify outputs exist
        {
            let cell = doc.get_cell(0).unwrap();
            let txn = doc.doc().transact();
            let outputs = cell.outputs(&txn).unwrap();
            assert_eq!(outputs.len(&txn), 2);
        }

        // Clear outputs
        doc.clear_cell_outputs(0).unwrap();

        // Verify outputs are gone
        let cell = doc.get_cell(0).unwrap();
        let txn = doc.doc().transact();
        let outputs = cell.outputs(&txn).unwrap();
        assert_eq!(outputs.len(&txn), 0);
    }

    #[test]
    fn test_set_execution_count() {
        let doc = NotebookDoc::new();
        doc.add_cell("cell-1", cell_types::CODE, "x = 1", None)
            .unwrap();

        // Initially execution_count is null
        {
            let cell = doc.get_cell(0).unwrap();
            let txn = doc.doc().transact();
            assert_eq!(cell.execution_count(&txn), Some(None));
        }

        // Set execution count
        doc.set_execution_count(0, Some(42)).unwrap();

        {
            let cell = doc.get_cell(0).unwrap();
            let txn = doc.doc().transact();
            assert_eq!(cell.execution_count(&txn), Some(Some(42)));
        }

        // Clear execution count
        doc.set_execution_count(0, None).unwrap();

        {
            let cell = doc.get_cell(0).unwrap();
            let txn = doc.doc().transact();
            assert_eq!(cell.execution_count(&txn), Some(None));
        }
    }

    #[test]
    fn test_output_on_non_code_cell_fails() {
        use nbformat::v4::MultilineString;

        let doc = NotebookDoc::new();
        doc.add_cell("cell-1", cell_types::MARKDOWN, "# Hello", None)
            .unwrap();

        let output = Output::Stream {
            name: "stdout".into(),
            text: MultilineString("hello\n".into()),
        };

        // Should fail because markdown cells don't have outputs
        assert!(doc.append_output(0, &output).is_err());
        assert!(doc.clear_cell_outputs(0).is_err());
        assert!(doc.set_execution_count(0, Some(1)).is_err());
    }
}

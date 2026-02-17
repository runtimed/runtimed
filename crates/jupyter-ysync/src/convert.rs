use std::collections::HashMap;

use nbformat::v4::{Cell, CellId, CellMetadata, Metadata, Notebook, Output};
use serde_json::Value;
use yrs::types::ToJson;
use yrs::{
    Any, Array, ArrayPrelim, GetString, Map, MapPrelim, MapRef, Out, ReadTxn, Transact, WriteTxn,
};

use crate::doc::{cell_types, keys, NotebookDoc};
use crate::error::{Result, YSyncError};

/// Convert an nbformat Notebook to a NotebookDoc (Y.Doc).
pub fn notebook_to_ydoc(notebook: &Notebook) -> Result<NotebookDoc> {
    let doc = NotebookDoc::new();

    {
        let mut txn = doc.doc().transact_mut();

        // Convert metadata
        let metadata_map = txn.get_or_insert_map(keys::METADATA);
        convert_metadata_to_ymap(&notebook.metadata, &metadata_map, &mut txn)?;

        // Convert cells
        let cells_array = txn.get_or_insert_array(keys::CELLS);
        for (idx, cell) in notebook.cells.iter().enumerate() {
            let (cell_prelim, outputs) = convert_cell_to_ymap_prelim(cell)?;
            cells_array.push_back(&mut txn, cell_prelim);

            // For code cells, add outputs as a proper YArray (for CRDT modification support)
            if let Some(outputs) = outputs {
                if let Some(Out::YMap(cell_map)) = cells_array.get(&txn, idx as u32) {
                    // Create the outputs YArray
                    cell_map.insert(&mut txn, keys::OUTPUTS, ArrayPrelim::default());

                    // Add the outputs to the array
                    if let Some(Out::YArray(outputs_array)) = cell_map.get(&txn, keys::OUTPUTS) {
                        for output in outputs {
                            outputs_array.push_back(&mut txn, output);
                        }
                    }
                }
            }
        }
    }

    Ok(doc)
}

/// Convert a NotebookDoc (Y.Doc) back to an nbformat Notebook.
pub fn ydoc_to_notebook(doc: &NotebookDoc) -> Result<Notebook> {
    let txn = doc.doc().transact();

    // Convert metadata
    let metadata_map = doc.metadata(&txn);
    let metadata = convert_ymap_to_metadata(&metadata_map, &txn)?;

    // Convert cells
    let cells_array = doc.cells(&txn);
    let mut cells = Vec::with_capacity(cells_array.len(&txn) as usize);

    for value in cells_array.iter(&txn) {
        if let Out::YMap(cell_map) = value {
            let cell = convert_ymap_to_cell(&cell_map, &txn)?;
            cells.push(cell);
        }
    }

    Ok(Notebook {
        metadata,
        nbformat: 4,
        nbformat_minor: 5,
        cells,
    })
}

/// Convert notebook metadata to Y.Map entries.
fn convert_metadata_to_ymap(
    metadata: &Metadata,
    map: &MapRef,
    txn: &mut yrs::TransactionMut,
) -> Result<()> {
    // Convert kernelspec if present
    if let Some(ref kernelspec) = metadata.kernelspec {
        let mut ks_content: HashMap<String, Any> = HashMap::new();
        ks_content.insert(
            "display_name".into(),
            Any::String(kernelspec.display_name.clone().into()),
        );
        ks_content.insert("name".into(), Any::String(kernelspec.name.clone().into()));
        if let Some(ref lang) = kernelspec.language {
            ks_content.insert("language".into(), Any::String(lang.clone().into()));
        }
        for (k, v) in &kernelspec.additional {
            ks_content.insert(k.clone(), json_to_any(v));
        }
        map.insert(txn, "kernelspec", MapPrelim::from_iter(ks_content));
    }

    // Convert language_info if present
    if let Some(ref lang_info) = metadata.language_info {
        let mut li_content: HashMap<String, Any> = HashMap::new();
        li_content.insert("name".into(), Any::String(lang_info.name.clone().into()));
        if let Some(ref version) = lang_info.version {
            li_content.insert("version".into(), Any::String(version.clone().into()));
        }
        for (k, v) in &lang_info.additional {
            li_content.insert(k.clone(), json_to_any(v));
        }
        map.insert(txn, "language_info", MapPrelim::from_iter(li_content));
    }

    // Convert authors if present
    if let Some(ref authors) = metadata.authors {
        let authors_array: Vec<Any> = authors
            .iter()
            .map(|a| {
                let mut author_map = HashMap::new();
                author_map.insert("name".to_string(), Any::String(a.name.clone().into()));
                Any::Map(author_map.into())
            })
            .collect();
        map.insert(txn, "authors", authors_array);
    }

    // Convert additional metadata fields
    for (k, v) in &metadata.additional {
        map.insert(txn, k.as_str(), json_to_any(v));
    }

    Ok(())
}

/// Convert a cell to a Y.Map prelim for insertion.
///
/// Returns the map prelim and optionally outputs (for code cells).
/// Outputs are returned separately so they can be inserted as a proper YArray.
fn convert_cell_to_ymap_prelim(cell: &Cell) -> Result<(MapPrelim, Option<Vec<Any>>)> {
    let mut prelim: HashMap<String, Any> = HashMap::new();

    match cell {
        Cell::Code {
            id,
            metadata,
            execution_count,
            source,
            outputs,
        } => {
            prelim.insert(keys::ID.into(), Any::String(id.as_str().into()));
            prelim.insert(keys::CELL_TYPE.into(), Any::String(cell_types::CODE.into()));

            // Source as joined string
            let source_str = source.join("");
            prelim.insert(keys::SOURCE.into(), Any::String(source_str.into()));

            // Execution count
            prelim.insert(
                keys::EXECUTION_COUNT.into(),
                execution_count
                    .map(|c| Any::BigInt(c as i64))
                    .unwrap_or(Any::Null),
            );

            // Convert outputs to Any values (will be inserted as YArray separately)
            let outputs_any: Vec<Any> = outputs
                .iter()
                .filter_map(|o| output_to_any(o).ok())
                .collect();

            // Convert cell metadata
            prelim.insert(keys::CELL_METADATA.into(), cell_metadata_to_any(metadata));

            Ok((MapPrelim::from_iter(prelim), Some(outputs_any)))
        }
        Cell::Markdown {
            id,
            metadata,
            source,
            attachments,
        } => {
            prelim.insert(keys::ID.into(), Any::String(id.as_str().into()));
            prelim.insert(
                keys::CELL_TYPE.into(),
                Any::String(cell_types::MARKDOWN.into()),
            );

            let source_str = source.join("");
            prelim.insert(keys::SOURCE.into(), Any::String(source_str.into()));

            prelim.insert(keys::CELL_METADATA.into(), cell_metadata_to_any(metadata));

            if let Some(ref att) = attachments {
                prelim.insert(keys::ATTACHMENTS.into(), json_to_any(att));
            }

            Ok((MapPrelim::from_iter(prelim), None))
        }
        Cell::Raw {
            id,
            metadata,
            source,
        } => {
            prelim.insert(keys::ID.into(), Any::String(id.as_str().into()));
            prelim.insert(keys::CELL_TYPE.into(), Any::String(cell_types::RAW.into()));

            let source_str = source.join("");
            prelim.insert(keys::SOURCE.into(), Any::String(source_str.into()));

            prelim.insert(keys::CELL_METADATA.into(), cell_metadata_to_any(metadata));

            Ok((MapPrelim::from_iter(prelim), None))
        }
    }
}

/// Convert Y.Map metadata back to nbformat Metadata.
fn convert_ymap_to_metadata<T: ReadTxn>(map: &MapRef, txn: &T) -> Result<Metadata> {
    let json = map.to_json(txn);
    let value = any_to_json(&json);

    // Parse the JSON value into Metadata
    let metadata: Metadata = serde_json::from_value(value).map_err(|e| {
        YSyncError::ConversionError(format!("Failed to deserialize metadata: {}", e))
    })?;

    Ok(metadata)
}

/// Convert Y.Map cell back to nbformat Cell.
fn convert_ymap_to_cell<T: ReadTxn>(map: &MapRef, txn: &T) -> Result<Cell> {
    let cell_type = map
        .get(txn, keys::CELL_TYPE)
        .and_then(|v| match v {
            Out::Any(Any::String(s)) => Some(s.to_string()),
            _ => None,
        })
        .ok_or_else(|| YSyncError::MissingField("cell_type".into()))?;

    let id_str = map
        .get(txn, keys::ID)
        .and_then(|v| match v {
            Out::Any(Any::String(s)) => Some(s.to_string()),
            _ => None,
        })
        .ok_or_else(|| YSyncError::MissingField("id".into()))?;

    let id = CellId::new(&id_str)
        .map_err(|e| YSyncError::ConversionError(format!("Invalid cell ID: {}", e)))?;

    // Get source - could be Y.Text or string
    let source = get_source_from_ymap(map, txn)?;

    // Get metadata
    let metadata = get_cell_metadata_from_ymap(map, txn)?;

    match cell_type.as_str() {
        cell_types::CODE => {
            let execution_count = map.get(txn, keys::EXECUTION_COUNT).and_then(|v| match v {
                Out::Any(Any::BigInt(n)) => Some(n as i32),
                Out::Any(Any::Number(n)) => Some(n as i32),
                Out::Any(Any::Null) => None,
                _ => None,
            });

            let outputs = get_outputs_from_ymap(map, txn)?;

            Ok(Cell::Code {
                id,
                metadata,
                execution_count,
                source,
                outputs,
            })
        }
        cell_types::MARKDOWN => {
            let attachments = map.get(txn, keys::ATTACHMENTS).map(|v| out_to_json(&v, txn));

            Ok(Cell::Markdown {
                id,
                metadata,
                source,
                attachments,
            })
        }
        cell_types::RAW => Ok(Cell::Raw {
            id,
            metadata,
            source,
        }),
        _ => Err(YSyncError::InvalidCellType(cell_type)),
    }
}

/// Get source from Y.Map (handles both Y.Text and string).
fn get_source_from_ymap<T: ReadTxn>(map: &MapRef, txn: &T) -> Result<Vec<String>> {
    let source_value = map
        .get(txn, keys::SOURCE)
        .ok_or_else(|| YSyncError::MissingField("source".into()))?;

    let source_str = match source_value {
        Out::YText(text) => text.get_string(txn),
        Out::Any(Any::String(s)) => s.to_string(),
        _ => {
            return Err(YSyncError::ConversionError(
                "Invalid source type".into(),
            ))
        }
    };

    // Split into lines, preserving newlines at the end of each line (nbformat style)
    let lines: Vec<String> = source_str
        .split_inclusive('\n')
        .map(|s| s.to_string())
        .collect();

    // Handle case where source doesn't end with newline or is empty
    if lines.is_empty() && !source_str.is_empty() {
        Ok(vec![source_str])
    } else if lines.is_empty() {
        Ok(vec!["".to_string()])
    } else {
        Ok(lines)
    }
}

/// Get cell metadata from Y.Map.
fn get_cell_metadata_from_ymap<T: ReadTxn>(map: &MapRef, txn: &T) -> Result<CellMetadata> {
    let metadata_value = map.get(txn, keys::CELL_METADATA);

    match metadata_value {
        Some(Out::YMap(metadata_map)) => {
            let json = metadata_map.to_json(txn);
            let value = any_to_json(&json);
            serde_json::from_value(value).map_err(|e| {
                YSyncError::ConversionError(format!("Failed to deserialize cell metadata: {}", e))
            })
        }
        Some(Out::Any(any)) => {
            let value = any_to_json(&any);
            serde_json::from_value(value).map_err(|e| {
                YSyncError::ConversionError(format!("Failed to deserialize cell metadata: {}", e))
            })
        }
        None => Ok(default_cell_metadata()),
        _ => Ok(default_cell_metadata()),
    }
}

/// Get outputs from Y.Map (for code cells).
fn get_outputs_from_ymap<T: ReadTxn>(map: &MapRef, txn: &T) -> Result<Vec<Output>> {
    let outputs_value = map.get(txn, keys::OUTPUTS);

    match outputs_value {
        Some(Out::YArray(arr)) => {
            let mut outputs = Vec::new();
            for value in arr.iter(txn) {
                let json = out_to_json(&value, txn);
                if let Ok(output) = serde_json::from_value(json) {
                    outputs.push(output);
                }
            }
            Ok(outputs)
        }
        Some(Out::Any(Any::Array(arr))) => {
            let mut outputs = Vec::new();
            for any in arr.iter() {
                let json = any_to_json(any);
                if let Ok(output) = serde_json::from_value(json) {
                    outputs.push(output);
                }
            }
            Ok(outputs)
        }
        None => Ok(Vec::new()),
        _ => Ok(Vec::new()),
    }
}

/// Convert serde_json::Value to yrs::Any.
pub fn json_to_any(value: &Value) -> Any {
    match value {
        Value::Null => Any::Null,
        Value::Bool(b) => Any::Bool(*b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Any::BigInt(i)
            } else if let Some(f) = n.as_f64() {
                Any::Number(f)
            } else {
                Any::Null
            }
        }
        Value::String(s) => Any::String(s.clone().into()),
        Value::Array(arr) => {
            let items: Vec<Any> = arr.iter().map(json_to_any).collect();
            Any::Array(items.into())
        }
        Value::Object(obj) => {
            let map: HashMap<String, Any> = obj
                .iter()
                .map(|(k, v)| (k.clone(), json_to_any(v)))
                .collect();
            Any::Map(map.into())
        }
    }
}

/// Convert yrs::Any to serde_json::Value.
pub fn any_to_json(any: &yrs::Any) -> Value {
    match any {
        Any::Null | Any::Undefined => Value::Null,
        Any::Bool(b) => Value::Bool(*b),
        Any::Number(n) => Value::Number(serde_json::Number::from_f64(*n).unwrap_or(0.into())),
        Any::BigInt(i) => Value::Number((*i).into()),
        Any::String(s) => Value::String(s.to_string()),
        Any::Buffer(b) => {
            // Encode buffer as base64 string
            use base64::Engine;
            Value::String(base64::engine::general_purpose::STANDARD.encode(b.as_ref()))
        }
        Any::Array(arr) => {
            let items: Vec<Value> = arr.iter().map(any_to_json).collect();
            Value::Array(items)
        }
        Any::Map(map) => {
            let obj: serde_json::Map<String, Value> = map
                .iter()
                .map(|(k, v)| (k.clone(), any_to_json(v)))
                .collect();
            Value::Object(obj)
        }
    }
}

/// Convert yrs::Out to serde_json::Value.
fn out_to_json<T: ReadTxn>(value: &Out, txn: &T) -> Value {
    match value {
        Out::Any(any) => any_to_json(any),
        Out::YText(text) => Value::String(text.get_string(txn)),
        Out::YArray(arr) => {
            let items: Vec<Value> = arr.iter(txn).map(|v| out_to_json(&v, txn)).collect();
            Value::Array(items)
        }
        Out::YMap(map) => {
            let json = map.to_json(txn);
            any_to_json(&json)
        }
        _ => Value::Null,
    }
}

/// Convert CellMetadata to yrs::Any.
fn cell_metadata_to_any(metadata: &CellMetadata) -> Any {
    let json = serde_json::to_value(metadata).unwrap_or(Value::Object(Default::default()));
    json_to_any(&json)
}

/// Convert Output to yrs::Any for storage in Y.Array.
pub fn output_to_any(output: &Output) -> Result<Any> {
    let json = serde_json::to_value(output)?;
    Ok(json_to_any(&json))
}

/// Create a default CellMetadata with all fields set to None.
fn default_cell_metadata() -> CellMetadata {
    CellMetadata {
        id: None,
        collapsed: None,
        scrolled: None,
        deletable: None,
        editable: None,
        format: None,
        name: None,
        tags: None,
        jupyter: None,
        execution: None,
        additional: Default::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nbformat::v4::{KernelSpec, LanguageInfo, MultilineString};

    fn create_test_notebook() -> Notebook {
        Notebook {
            metadata: Metadata {
                kernelspec: Some(KernelSpec {
                    display_name: "Python 3".into(),
                    name: "python3".into(),
                    language: Some("python".into()),
                    additional: Default::default(),
                }),
                language_info: Some(LanguageInfo {
                    name: "python".into(),
                    version: Some("3.10".into()),
                    codemirror_mode: None,
                    additional: Default::default(),
                }),
                authors: None,
                additional: Default::default(),
            },
            nbformat: 4,
            nbformat_minor: 5,
            cells: vec![
                Cell::Markdown {
                    id: CellId::new("cell-1").unwrap(),
                    metadata: default_cell_metadata(),
                    source: vec!["# Hello World\n".into()],
                    attachments: None,
                },
                Cell::Code {
                    id: CellId::new("cell-2").unwrap(),
                    metadata: default_cell_metadata(),
                    execution_count: Some(1),
                    source: vec!["print('hello')\n".into()],
                    outputs: vec![Output::Stream {
                        name: "stdout".into(),
                        text: MultilineString("hello\n".into()),
                    }],
                },
            ],
        }
    }

    #[test]
    fn test_notebook_to_ydoc() {
        let notebook = create_test_notebook();
        let doc = notebook_to_ydoc(&notebook).unwrap();

        assert_eq!(doc.cell_count(), 2);
    }

    #[test]
    fn test_ydoc_to_notebook() {
        let notebook = create_test_notebook();
        let doc = notebook_to_ydoc(&notebook).unwrap();
        let converted = ydoc_to_notebook(&doc).unwrap();

        assert_eq!(converted.cells.len(), notebook.cells.len());
        assert_eq!(converted.nbformat, 4);
    }

    #[test]
    fn test_roundtrip_code_cell() {
        let notebook = Notebook {
            metadata: Metadata {
                kernelspec: None,
                language_info: None,
                authors: None,
                additional: Default::default(),
            },
            nbformat: 4,
            nbformat_minor: 5,
            cells: vec![Cell::Code {
                id: CellId::new("test-cell").unwrap(),
                metadata: default_cell_metadata(),
                execution_count: Some(42),
                source: vec!["x = 1\n".into(), "y = 2\n".into()],
                outputs: vec![],
            }],
        };

        let doc = notebook_to_ydoc(&notebook).unwrap();
        let converted = ydoc_to_notebook(&doc).unwrap();

        assert_eq!(converted.cells.len(), 1);
        if let Cell::Code {
            execution_count,
            source,
            ..
        } = &converted.cells[0]
        {
            assert_eq!(*execution_count, Some(42));
            assert_eq!(source.join(""), "x = 1\ny = 2\n");
        } else {
            panic!("Expected code cell");
        }
    }

    #[test]
    fn test_roundtrip_markdown_cell() {
        let notebook = Notebook {
            metadata: Metadata {
                kernelspec: None,
                language_info: None,
                authors: None,
                additional: Default::default(),
            },
            nbformat: 4,
            nbformat_minor: 5,
            cells: vec![Cell::Markdown {
                id: CellId::new("md-cell").unwrap(),
                metadata: default_cell_metadata(),
                source: vec!["# Title\n".into(), "\n".into(), "Some text\n".into()],
                attachments: None,
            }],
        };

        let doc = notebook_to_ydoc(&notebook).unwrap();
        let converted = ydoc_to_notebook(&doc).unwrap();

        assert_eq!(converted.cells.len(), 1);
        if let Cell::Markdown { source, .. } = &converted.cells[0] {
            assert_eq!(source.join(""), "# Title\n\nSome text\n");
        } else {
            panic!("Expected markdown cell");
        }
    }

    #[test]
    fn test_roundtrip_raw_cell() {
        let notebook = Notebook {
            metadata: Metadata {
                kernelspec: None,
                language_info: None,
                authors: None,
                additional: Default::default(),
            },
            nbformat: 4,
            nbformat_minor: 5,
            cells: vec![Cell::Raw {
                id: CellId::new("raw-cell").unwrap(),
                metadata: default_cell_metadata(),
                source: vec!["raw content".into()],
            }],
        };

        let doc = notebook_to_ydoc(&notebook).unwrap();
        let converted = ydoc_to_notebook(&doc).unwrap();

        assert_eq!(converted.cells.len(), 1);
        if let Cell::Raw { source, .. } = &converted.cells[0] {
            assert_eq!(source.join(""), "raw content");
        } else {
            panic!("Expected raw cell");
        }
    }

    #[test]
    fn test_json_to_any_roundtrip() {
        let original = serde_json::json!({
            "string": "hello",
            "number": 42,
            "float": 1.5,
            "bool": true,
            "null": null,
            "array": [1, 2, 3],
            "object": {"nested": "value"}
        });

        let any = json_to_any(&original);
        let back = any_to_json(&any);

        assert_eq!(original["string"], back["string"]);
        assert_eq!(original["number"], back["number"]);
        assert_eq!(original["bool"], back["bool"]);
        assert!(back["null"].is_null());
    }

    #[test]
    fn test_metadata_roundtrip() {
        let notebook = Notebook {
            metadata: Metadata {
                kernelspec: Some(KernelSpec {
                    display_name: "Python 3".into(),
                    name: "python3".into(),
                    language: Some("python".into()),
                    additional: Default::default(),
                }),
                language_info: Some(LanguageInfo {
                    name: "python".into(),
                    version: Some("3.10.0".into()),
                    codemirror_mode: None,
                    additional: Default::default(),
                }),
                authors: None,
                additional: Default::default(),
            },
            nbformat: 4,
            nbformat_minor: 5,
            cells: vec![],
        };

        let doc = notebook_to_ydoc(&notebook).unwrap();
        let converted = ydoc_to_notebook(&doc).unwrap();

        assert!(converted.metadata.kernelspec.is_some());
        let ks = converted.metadata.kernelspec.unwrap();
        assert_eq!(ks.name, "python3");
        assert_eq!(ks.display_name, "Python 3");
    }

    #[test]
    fn test_converted_doc_supports_output_modification() {
        // Create a notebook with a code cell and some outputs
        let notebook = Notebook {
            metadata: Metadata {
                kernelspec: None,
                language_info: None,
                authors: None,
                additional: Default::default(),
            },
            nbformat: 4,
            nbformat_minor: 5,
            cells: vec![Cell::Code {
                id: CellId::new("cell-1").unwrap(),
                metadata: default_cell_metadata(),
                execution_count: Some(1),
                source: vec!["print('hello')".into()],
                outputs: vec![Output::Stream {
                    name: "stdout".into(),
                    text: MultilineString("hello\n".into()),
                }],
            }],
        };

        let doc = notebook_to_ydoc(&notebook).unwrap();

        // Verify the output is there
        {
            let cell = doc.get_cell(0).unwrap();
            let txn = doc.doc().transact();
            let outputs = cell.outputs(&txn).unwrap();
            assert_eq!(outputs.len(&txn), 1);
        }

        // Add another output - this should work because outputs is a YArray
        let new_output = Output::Stream {
            name: "stdout".into(),
            text: MultilineString("world\n".into()),
        };
        doc.append_output(0, &new_output).unwrap();

        // Verify both outputs are there
        {
            let cell = doc.get_cell(0).unwrap();
            let txn = doc.doc().transact();
            let outputs = cell.outputs(&txn).unwrap();
            assert_eq!(outputs.len(&txn), 2);
        }

        // Clear outputs
        doc.clear_cell_outputs(0).unwrap();

        // Verify outputs are cleared
        {
            let cell = doc.get_cell(0).unwrap();
            let txn = doc.doc().transact();
            let outputs = cell.outputs(&txn).unwrap();
            assert_eq!(outputs.len(&txn), 0);
        }
    }
}

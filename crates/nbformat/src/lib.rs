pub mod legacy;
pub mod v4;

use serde::Serialize as _;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NotebookError {
    #[error("Unsupported notebook version: {0}.{1}")]
    UnsupportedVersion(i32, i32),
    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("Validation error: {0}")]
    ValidationError(String),
}

#[derive(Debug)]
pub enum Notebook {
    V4(v4::Notebook),
    Legacy(legacy::Notebook),
}

pub fn parse_notebook(json: &str) -> Result<Notebook, NotebookError> {
    let value: serde_json::Value = serde_json::from_str(json)?;
    let nbformat = value["nbformat"].as_i64().unwrap_or(0) as i32;
    let nbformat_minor = value["nbformat_minor"].as_i64().unwrap_or(0) as i32;

    match (nbformat, nbformat_minor) {
        (4, 5) => Ok(Notebook::V4(serde_json::from_value::<v4::Notebook>(value)?)),
        (4, 0) | (4, 1) | (4, 2) | (4, 3) | (4, 4) => Ok(Notebook::Legacy(
            serde_json::from_value::<legacy::Notebook>(value)?,
        )),
        _ => Err(NotebookError::UnsupportedVersion(nbformat, nbformat_minor)),
    }
}

pub fn serialize_notebook(notebook: &Notebook) -> Result<String, NotebookError> {
    match notebook {
        Notebook::V4(notebook) => {
            let value = serde_json::to_value(notebook)?;
            let mut buf = Vec::new();
            let formatter = serde_json::ser::PrettyFormatter::with_indent(b" ");
            let mut ser = serde_json::Serializer::with_formatter(&mut buf, formatter);
            value.serialize(&mut ser)?;

            // Append a newline to the buffer to match the python implementation of nbformat
            buf.append(&mut b"\n".to_vec());

            let notebook_json = String::from_utf8(buf)
                .map_err(|e| NotebookError::ValidationError(e.to_string()))?;

            Ok(notebook_json)
        }
        Notebook::Legacy(notebook) => Err(NotebookError::UnsupportedVersion(
            notebook.nbformat,
            notebook.nbformat_minor,
        )),
    }
}

pub fn upgrade_legacy_notebook(legacy_notebook: legacy::Notebook) -> anyhow::Result<v4::Notebook> {
    let cells: Vec<v4::Cell> = legacy_notebook
        .cells
        .into_iter()
        .map(|cell: legacy::Cell| match cell {
            legacy::Cell::Markdown {
                id,
                metadata,
                source,
                attachments,
            } => v4::Cell::Markdown {
                id: id.unwrap_or_else(|| uuid::Uuid::new_v4().into()),
                metadata,
                source,
                attachments,
            },
            legacy::Cell::Code {
                id,
                metadata,
                execution_count,
                source,
                outputs,
            } => v4::Cell::Code {
                id: id.unwrap_or_else(|| uuid::Uuid::new_v4().into()),
                metadata,
                execution_count,
                source,
                outputs,
            },
            legacy::Cell::Raw {
                id,
                metadata,
                source,
            } => v4::Cell::Raw {
                id: id.unwrap_or_else(|| uuid::Uuid::new_v4().into()),
                metadata,
                source,
            },
        })
        .collect();

    // If any of the cell IDs are not unique, bail
    let mut seen_ids = std::collections::HashSet::new();
    for cell in &cells {
        if !seen_ids.insert(cell.id()) {
            return Err(anyhow::anyhow!("Duplicate Cell ID found: {}", cell.id()));
        }
    }

    Ok(v4::Notebook {
        cells,
        metadata: legacy_notebook.metadata,
        nbformat: 4,
        nbformat_minor: 5,
    })
}

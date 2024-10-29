pub mod legacy;
pub mod v4;

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
        (4, 1) | (4, 2) | (4, 3) | (4, 4) => Ok(Notebook::Legacy(serde_json::from_value::<
            legacy::Notebook,
        >(value)?)),
        _ => Err(NotebookError::UnsupportedVersion(nbformat, nbformat_minor)),
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

    Ok(v4::Notebook {
        cells,
        metadata: legacy_notebook.metadata,
        nbformat: 4,
        nbformat_minor: 5,
    })
}

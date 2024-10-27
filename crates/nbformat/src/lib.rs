pub mod legacy;
pub mod v4;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum NotebookError {
    #[error("Unsupported notebook version: {0}.{1}")]
    UnsupportedVersion(i32, i32),
    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),
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

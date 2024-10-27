use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use core::fmt;
use std::{
    collections::HashMap,
    fmt::{Display, Formatter},
};

use runtimelib::{DisplayData, ErrorOutput, ExecuteResult, StreamContent};

#[derive(Deserialize, Debug)]
pub struct Notebook {
    pub metadata: DeserializedMetadata,
    pub nbformat: i32,
    pub nbformat_minor: i32,
    #[serde(deserialize_with = "deserialize_cells")]
    pub cells: Vec<DeserializedCell>,
}

#[derive(Deserialize, Debug)]
pub struct DeserializedMetadata {
    pub kernelspec: Option<DeserializedKernelSpec>,
    pub language_info: Option<DeserializedLanguageInfo>,
    #[serde(flatten)]
    pub additional: HashMap<String, serde_json::Value>,
}

#[derive(Deserialize, Debug)]
pub struct DeserializedKernelSpec {
    pub name: String,
    pub language: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct DeserializedLanguageInfo {
    pub name: String,
    pub version: Option<String>,
    #[serde(default)]
    pub codemirror_mode: Option<CodemirrorMode>,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum CodemirrorMode {
    String(String),
    Object(Value),
}

#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct CellId(pub String);

impl Display for CellId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for CellId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid.to_string())
    }
}

impl From<String> for CellId {
    fn from(string: String) -> Self {
        Self(string)
    }
}

impl From<Option<String>> for CellId {
    fn from(string: Option<String>) -> Self {
        if string.is_some() {
            string.into()
        } else {
            CellId {
                0: Uuid::new_v4().to_string(),
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CellType {
    Code,
    Markdown,
    Raw,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "cell_type")]
pub enum DeserializedCell {
    #[serde(rename = "markdown")]
    Markdown {
        id: Option<String>,
        metadata: DeserializedCellMetadata,
        source: Vec<String>,
        #[serde(default)]
        attachments: Option<Value>,
    },
    #[serde(rename = "code")]
    Code {
        id: Option<String>,
        metadata: DeserializedCellMetadata,
        execution_count: Option<i32>,
        source: Vec<String>,
        #[serde(deserialize_with = "deserialize_outputs")]
        outputs: Vec<DeserializedOutput>,
    },
    #[serde(rename = "raw")]
    Raw {
        id: Option<String>,
        metadata: DeserializedCellMetadata,
        source: Vec<String>,
    },
}

pub fn deserialize_cells<'de, D>(deserializer: D) -> Result<Vec<DeserializedCell>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let cells: Vec<serde_json::Value> = Deserialize::deserialize(deserializer)?;
    cells
        .into_iter()
        .enumerate()
        .filter_map(
            |(index, cell)| match serde_json::from_value::<DeserializedCell>(cell) {
                Ok(cell) => Some(Ok(cell)),
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to deserialize cell at index {}: {}",
                        index, e
                    );
                    None
                }
            },
        )
        .collect()
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeserializedCellMetadata {
    id: Option<String>,
    collapsed: Option<bool>,
    scrolled: Option<serde_json::Value>,
    deletable: Option<bool>,
    editable: Option<bool>,
    format: Option<String>,
    name: Option<String>,
    tags: Option<Vec<String>>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "output_type")]
pub enum DeserializedOutput {
    #[serde(rename = "stream")]
    Stream(StreamContent),
    #[serde(rename = "display_data")]
    DisplayData(DisplayData),
    #[serde(rename = "execute_result")]
    ExecuteResult(ExecuteResult),
    #[serde(rename = "error")]
    Error(ErrorOutput),
}

pub fn deserialize_outputs<'de, D>(deserializer: D) -> Result<Vec<DeserializedOutput>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let outputs: Vec<serde_json::Value> = Deserialize::deserialize(deserializer)?;
    outputs
        .into_iter()
        .enumerate()
        .filter_map(|(index, output)| {
            match serde_json::from_value::<DeserializedOutput>(output.clone()) {
                Ok(output) => Some(Ok(output)),
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to deserialize output at index {} of cell: {}",
                        index, e
                    );
                    eprintln!(
                        "Output JSON: {}",
                        serde_json::to_string_pretty(&output).unwrap_or_default()
                    );
                    None
                }
            }
        })
        .collect()
}

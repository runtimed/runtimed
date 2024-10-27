use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use uuid::Uuid;

use runtimelib::{DisplayData, ErrorOutput, ExecuteResult};

use core::fmt;
use std::{
    collections::HashMap,
    fmt::{Display, Formatter},
};

#[derive(Debug, Clone, PartialEq)]
pub struct MultilineString(pub String);

impl Serialize for MultilineString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for MultilineString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(MultilineString(s))
    }
}
fn deserialize_multiline_string<'de, D>(deserializer: D) -> Result<MultilineString, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct MultilineStringVisitor;

    impl<'de> serde::de::Visitor<'de> for MultilineStringVisitor {
        type Value = MultilineString;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("string or array of strings")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(MultilineString(value.to_string()))
        }

        fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(MultilineString(value))
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let mut result = String::new();
            while let Some(s) = seq.next_element::<String>()? {
                result.push_str(&s);
            }
            Ok(MultilineString(result))
        }
    }

    deserializer.deserialize_any(MultilineStringVisitor)
}

#[derive(Deserialize, Debug)]
pub struct Notebook {
    pub metadata: Metadata,
    pub nbformat: i32,
    pub nbformat_minor: i32,
    #[serde(deserialize_with = "deserialize_cells")]
    pub cells: Vec<Cell>,
}

#[derive(Deserialize, Debug)]
pub struct Metadata {
    pub kernelspec: Option<KernelSpec>,
    pub language_info: Option<LanguageInfo>,
    pub authors: Option<Vec<Author>>,
    #[serde(flatten)]
    pub additional: HashMap<String, serde_json::Value>,
}

#[derive(Deserialize, Debug)]
pub struct Author {
    pub name: String,
}

#[derive(Deserialize, Debug)]
pub struct KernelSpec {
    pub name: String,
    pub language: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct LanguageInfo {
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

#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub struct CellId(String);

impl CellId {
    fn is_valid(s: &str) -> bool {
        !s.is_empty()
            && s.len() <= 64
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    }

    pub fn new(s: &str) -> Result<Self, &'static str> {
        if Self::is_valid(s) {
            Ok(CellId(s.to_string()))
        } else {
            Err("Invalid cell ID")
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for CellId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for CellId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for CellId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        CellId::new(&s).map_err(de::Error::custom)
    }
}

impl From<Uuid> for CellId {
    fn from(uuid: Uuid) -> Self {
        // Assume UUID is always valid for CellId
        CellId(uuid.to_string())
    }
}

impl TryFrom<String> for CellId {
    type Error = &'static str;

    fn try_from(string: String) -> Result<Self, Self::Error> {
        CellId::new(&string)
    }
}

impl<'a> TryFrom<&'a str> for CellId {
    type Error = &'static str;

    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        CellId::new(s)
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
pub enum Cell {
    #[serde(rename = "markdown")]
    Markdown {
        id: CellId,
        metadata: CellMetadata,
        source: Vec<String>,
        #[serde(default)]
        attachments: Option<Value>,
    },
    #[serde(rename = "code")]
    Code {
        id: CellId,
        metadata: CellMetadata,
        execution_count: Option<i32>,
        source: Vec<String>,
        #[serde(deserialize_with = "deserialize_outputs")]
        outputs: Vec<Output>,
    },
    #[serde(rename = "raw")]
    Raw {
        id: CellId,
        metadata: CellMetadata,
        source: Vec<String>,
    },
}

pub fn deserialize_cells<'de, D>(deserializer: D) -> Result<Vec<Cell>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let cells: Vec<serde_json::Value> = Deserialize::deserialize(deserializer)?;
    cells
        .into_iter()
        .enumerate()
        .map(|(index, cell)| {
            serde_json::from_value::<Cell>(cell).map_err(|e| {
                de::Error::custom(format!(
                    "Failed to deserialize cell at index {}: {}",
                    index, e
                ))
            })
        })
        .collect()
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CellMetadata {
    pub id: Option<String>,
    pub collapsed: Option<bool>,
    pub scrolled: Option<bool>,
    pub deletable: Option<bool>,
    pub editable: Option<bool>,
    pub format: Option<String>,
    pub name: Option<String>,
    pub tags: Option<Vec<String>>,
    pub jupyter: Option<JupyterCellMetadata>,
    pub execution: Option<ExecutionMetadata>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JupyterCellMetadata {
    pub source_hidden: Option<bool>,
    pub outputs_hidden: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ExecutionMetadata {
    #[serde(rename = "iopub.execute_input")]
    pub iopub_execute_input: Option<String>,
    #[serde(rename = "iopub.status.busy")]
    pub iopub_status_busy: Option<String>,
    #[serde(rename = "shell.execute_reply")]
    pub shell_execute_reply: Option<String>,
    #[serde(rename = "iopub.status.idle")]
    pub iopub_status_idle: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "output_type")]
pub enum Output {
    #[serde(rename = "stream")]
    Stream {
        name: String,
        #[serde(deserialize_with = "deserialize_multiline_string")]
        text: MultilineString,
    },
    // todo!(): transient does not belong _in_ nbformat, though it is on the raw
    // jupyter protocol. We'll need to accept a subset here.
    #[serde(rename = "display_data")]
    DisplayData(DisplayData),
    // todo!() Same goes for handling execute result
    #[serde(rename = "execute_result")]
    ExecuteResult(ExecuteResult),
    #[serde(rename = "error")]
    Error(ErrorOutput),
}

pub fn deserialize_outputs<'de, D>(deserializer: D) -> Result<Vec<Output>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let outputs: Vec<serde_json::Value> = Deserialize::deserialize(deserializer)?;
    outputs
        .into_iter()
        .enumerate()
        .filter_map(
            |(index, output)| match serde_json::from_value::<Output>(output.clone()) {
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
            },
        )
        .collect()
}

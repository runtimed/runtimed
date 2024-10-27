use serde::{Deserialize, Serialize};
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
        id: Option<CellId>,
        metadata: DeserializedCellMetadata,
        source: Vec<String>,
        #[serde(default)]
        attachments: Option<Value>,
    },
    #[serde(rename = "code")]
    Code {
        id: Option<CellId>,
        metadata: DeserializedCellMetadata,
        execution_count: Option<i32>,
        source: Vec<String>,
        #[serde(deserialize_with = "deserialize_outputs")]
        outputs: Vec<DeserializedOutput>,
    },
    #[serde(rename = "raw")]
    Raw {
        id: Option<CellId>,
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
    pub id: Option<String>,
    pub collapsed: Option<bool>,
    pub scrolled: Option<serde_json::Value>,
    pub deletable: Option<bool>,
    pub editable: Option<bool>,
    pub format: Option<String>,
    pub name: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "output_type")]
pub enum DeserializedOutput {
    #[serde(rename = "stream")]
    Stream {
        name: String,
        #[serde(deserialize_with = "deserialize_multiline_string")]
        text: MultilineString,
    },
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

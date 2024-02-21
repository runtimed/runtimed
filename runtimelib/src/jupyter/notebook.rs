/*
Models a Notebook document. https://ipython.org/ipython-doc/3/notebook/nbformat.html
*/

use crate::jupyter::iopub_content::display_data::DisplayData;
use crate::jupyter::iopub_content::errors::Error;
use crate::jupyter::iopub_content::execute_result::ExecuteResult;
use crate::jupyter::iopub_content::stream::Stream;
use enum_as_inner::EnumAsInner;
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Notebook {
    pub cells: Vec<Cell>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    #[serde(
        serialize_with = "serialize_json_value_as_empty_object",
        deserialize_with = "serde_json::value::Value::deserialize"
    )]
    pub metadata: serde_json::Value,
    pub nbformat: u32,
    pub nbformat_minor: u32,
}

impl Default for Notebook {
    fn default() -> Self {
        Self::new()
    }
}

impl Notebook {
    pub fn new() -> Self {
        Self {
            cells: vec![],
            signature: None,
            metadata: serde_json::Value::Null,
            nbformat: 4,
            nbformat_minor: 5,
        }
    }

    pub fn from_file(filename: &str) -> Self {
        let content = std::fs::read_to_string(filename).unwrap();
        serde_json::from_str(&content).unwrap()
    }

    pub fn save(&self, filename: &str) {
        let js = serde_json::to_string_pretty(&self).expect("Failed to serialize notebook on save");
        std::fs::write(filename, js).unwrap();
    }

    pub fn dumps(&self) -> String {
        serde_json::to_string_pretty(&self).expect("Failed to serialize notebook on save")
    }

    pub fn get_cell(&self, id: &str) -> Option<&Cell> {
        self.cells.iter().find(|&cell| cell.id() == id)
    }

    pub fn get_mut_cell(&mut self, id: &str) -> Option<&mut Cell> {
        self.cells.iter_mut().find(|cell| cell.id() == id)
    }

    pub fn add_cell(&mut self, cell: Cell) {
        self.cells.push(cell);
    }

    pub fn add_code_cell(&mut self, source: &str) -> Cell {
        let cell = Cell::Code(CodeCell {
            id: uuid::Uuid::new_v4().to_string(),
            source: source.to_owned(),
            metadata: serde_json::Value::Null,
            execution_count: None,
            outputs: vec![],
        });
        self.cells.push(cell.clone());
        cell
    }

    pub fn add_markdown_cell(&mut self, source: &str) -> Cell {
        let cell = Cell::Markdown(MarkdownCell {
            id: uuid::Uuid::new_v4().to_string(),
            source: source.to_owned(),
            metadata: serde_json::Value::Null,
        });
        self.cells.push(cell.clone());
        cell
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, EnumAsInner)]
#[serde(tag = "output_type", rename_all = "snake_case")]
pub enum Output {
    DisplayData(DisplayData),
    Stream(Stream),
    ExecuteResult(ExecuteResult),
    Error(Error),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "cell_type", rename_all = "lowercase")]
pub enum Cell {
    Code(CodeCell),
    Markdown(MarkdownCell),
    Raw(RawCell),
}

impl Cell {
    pub fn id(&self) -> &str {
        match self {
            Cell::Code(cell) => &cell.id,
            Cell::Markdown(cell) => &cell.id,
            Cell::Raw(cell) => &cell.id,
        }
    }

    pub fn get_source(&self) -> String {
        match self {
            Cell::Code(cell) => cell.source.to_string(),
            Cell::Markdown(cell) => cell.source.to_string(),
            Cell::Raw(cell) => cell.source.to_string(),
        }
    }

    pub fn set_source(&mut self, source: &str) {
        match self {
            Cell::Code(cell) => cell.source = source.to_string(),
            Cell::Markdown(cell) => cell.source = source.to_string(),
            Cell::Raw(cell) => cell.source = source.to_string(),
        }
    }

    pub fn metadata(&self) -> &serde_json::Value {
        match self {
            Cell::Code(cell) => &cell.metadata,
            Cell::Markdown(cell) => &cell.metadata,
            Cell::Raw(cell) => &cell.metadata,
        }
    }

    pub fn add_output(&mut self, output: Output) {
        if let Cell::Code(cell) = self {
            cell.add_output(output);
        }
    }

    pub fn clear_output(&mut self) {
        if let Cell::Code(cell) = self {
            cell.clear_output();
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CodeCell {
    pub id: String,
    #[serde(deserialize_with = "list_or_string_to_string")]
    pub source: String,
    #[serde(
        serialize_with = "serialize_json_value_as_empty_object",
        deserialize_with = "serde_json::value::Value::deserialize"
    )]
    pub metadata: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_count: Option<u32>,
    pub outputs: Vec<Output>,
}

impl CodeCell {
    pub fn add_output(&mut self, output: Output) {
        self.outputs.push(output);
    }

    pub fn clear_output(&mut self) {
        self.outputs = vec![];
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MarkdownCell {
    pub id: String,
    #[serde(deserialize_with = "list_or_string_to_string")]
    pub source: String,
    #[serde(
        serialize_with = "serialize_json_value_as_empty_object",
        deserialize_with = "serde_json::value::Value::deserialize"
    )]
    pub metadata: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RawCell {
    pub id: String,
    #[serde(deserialize_with = "list_or_string_to_string")]
    pub source: String,
    #[serde(
        serialize_with = "serialize_json_value_as_empty_object",
        deserialize_with = "serde_json::value::Value::deserialize"
    )]
    pub metadata: serde_json::Value,
}

// Custom deserialization for source field since it may be a Vec<String> or String
pub fn list_or_string_to_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    // Deserialize the source field as a serde_json::Value
    let source_value: serde_json::Value = Deserialize::deserialize(deserializer)?;

    // Check if the source is an array of strings
    if let Some(source_array) = source_value.as_array() {
        // Join the array of strings into a single string
        let source_string = source_array
            .iter()
            .map(|s| s.as_str().unwrap_or_default())
            .collect::<Vec<_>>()
            .join("\n");

        Ok(source_string)
    } else if let Some(source_str) = source_value.as_str() {
        // If source is already a string, return it
        Ok(source_str.to_string())
    } else {
        Err(serde::de::Error::custom("Invalid source format"))
    }
}

// Custom serialization for when metadata fields are null, make them empty objects instead
fn serialize_json_value_as_empty_object<S>(
    value: &serde_json::Value,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        serde_json::Value::Null => serializer.serialize_map(Some(0))?.end(),
        _ => value.serialize(serializer),
    }
}

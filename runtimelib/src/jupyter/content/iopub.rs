use bytes::Bytes;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Debug)]
pub struct ClearOutput {
    pub wait: bool,
}

impl From<Bytes> for ClearOutput {
    fn from(bytes: Bytes) -> Self {
        serde_json::from_slice(&bytes).expect("Failed to deserialize ClearOutput")
    }
}

#[derive(Clone, Deserialize, Debug, Serialize, PartialEq)]
pub struct Transient {
    pub display_id: String,
}

// If the transient field is an empty dict, deserialize it as None
// otherwise deserialize it as Some(Transient)
fn deserialize_transient<'de, D>(deserializer: D) -> Result<Option<Transient>, D::Error>
where
    D: Deserializer<'de>,
{
    let v: Option<serde_json::Value> = Option::deserialize(deserializer)?;
    match v {
        Some(serde_json::Value::Object(map)) if map.is_empty() => Ok(None),
        Some(value) => serde_json::from_value(value)
            .map(Some)
            .map_err(serde::de::Error::custom),
        None => Ok(None),
    }
}

/// DisplayData is rich output that can be displayed in any client, whether a
/// notebook, an editor, or a console.
///
/// Read more about Jupyter's [display_data](https://jupyter-client.readthedocs.io/en/latest/messaging.html#display-data)
///
#[derive(Clone, Deserialize, Debug, Serialize, PartialEq)]
pub struct DisplayData {
    pub data: HashMap<String, serde_json::Value>,
    pub metadata: serde_json::Value,
    // Dev note: serde(default) is important here, when using custom deserialize_with and Option
    // then it will throw errors when the field is missing unless default is included.
    #[serde(default, deserialize_with = "deserialize_transient")]
    pub transient: Option<Transient>,
}

impl From<Bytes> for DisplayData {
    fn from(bytes: Bytes) -> Self {
        dbg!(&bytes);
        serde_json::from_slice(&bytes).expect("Failed to deserialize DisplayData")
    }
}

#[derive(Deserialize, Debug)]
pub struct UpdateDisplayData {
    pub data: HashMap<String, serde_json::Value>,
    pub metadata: serde_json::Value,
    // Dev note: serde(default) is important here, when using custom deserialize_with and Option
    // then it will throw errors when the field is missing unless default is included.
    #[serde(default, deserialize_with = "deserialize_transient")]
    pub transient: Option<Transient>,
}

impl From<Bytes> for UpdateDisplayData {
    fn from(bytes: Bytes) -> Self {
        serde_json::from_slice(&bytes).expect("Failed to deserialize UpdateDisplayData")
    }
}

#[derive(Clone, Serialize, PartialEq, Deserialize, Debug)]
pub struct Error {
    pub ename: String,
    pub evalue: String,
    pub traceback: Vec<String>,
}

impl From<Bytes> for Error {
    fn from(bytes: Bytes) -> Self {
        serde_json::from_slice(&bytes).expect("Failed to deserialize Error")
    }
}

#[derive(Deserialize, Debug)]
pub struct ExecuteInput {
    pub code: String,
    pub execution_count: u32,
}

impl From<Bytes> for ExecuteInput {
    fn from(bytes: Bytes) -> Self {
        serde_json::from_slice(&bytes).expect("Failed to deserialize ExecuteInput")
    }
}

#[derive(Clone, Serialize, PartialEq, Deserialize, Debug)]
pub struct ExecuteResult {
    pub execution_count: u32,
    pub data: HashMap<String, serde_json::Value>,
    pub metadata: serde_json::Value,
}

impl From<Bytes> for ExecuteResult {
    fn from(bytes: Bytes) -> Self {
        serde_json::from_slice(&bytes).expect("Failed to deserialize ExecuteResult")
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum KernelStatus {
    Busy,
    Idle,
    Starting,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Status {
    pub execution_state: KernelStatus,
}

impl From<Bytes> for Status {
    fn from(bytes: Bytes) -> Self {
        serde_json::from_slice(&bytes).unwrap()
    }
}

#[derive(Clone, Serialize, PartialEq, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum StreamName {
    Stdout,
    Stderr,
}

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

#[derive(Clone, Serialize, PartialEq, Deserialize, Debug)]
pub struct Stream {
    pub name: StreamName,
    #[serde(deserialize_with = "list_or_string_to_string")]
    pub text: String,
}

impl From<Bytes> for Stream {
    fn from(bytes: Bytes) -> Self {
        serde_json::from_slice(&bytes).expect("Failed to deserialize Stream")
    }
}

/*
https://jupyter-client.readthedocs.io/en/latest/messaging.html#display-data
*/
use std::collections::HashMap;

use bytes::Bytes;
use serde::{Deserialize, Deserializer, Serialize};

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

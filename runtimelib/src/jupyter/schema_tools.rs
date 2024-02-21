use bytes::Bytes;
use lazy_static::lazy_static;
use serde::{Deserialize, Deserializer};
use serde_json::{self, Value};

lazy_static! {
    pub static ref EMPTY_DICT_BYTES: Bytes = {
        let empty_dict: Value = serde_json::json!({});
        let empty_dict_bytes = serde_json::to_vec(&empty_dict).unwrap();
        Bytes::from(empty_dict_bytes)
    };
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

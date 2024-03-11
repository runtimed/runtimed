use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, Clone)]
#[serde(rename_all = "snake_case")]
pub enum MimeType {
    #[serde(rename = "text/plain")]
    Plain,
    #[serde(rename = "text/html")]
    Html,
    #[serde(rename = "application/json")]
    Json,
    #[serde(rename = "application/vnd.dataresource+json")]
    DataTable,
    #[serde(rename = "application/vnd.plotly.v1+json")]
    Plotly,
    #[serde(rename = "image/svg+xml")]
    Svg,
    #[serde(rename = "image/png")]
    Png,
    #[serde(rename = "image/jpeg")]
    Jpeg,
    #[serde(rename = "image/gif")]
    Gif,
    #[serde(other)]
    Other,
}

impl From<String> for MimeType {
    fn from(s: String) -> Self {
        match s.as_str() {
            "text/plain" => MimeType::Plain,
            "text/html" => MimeType::Html,
            "application/json" => MimeType::Json,
            "application/vnd.dataresource+json" => MimeType::DataTable,
            "application/vnd.plotly.v1+json" => MimeType::Plotly,
            "image/svg+xml" => MimeType::Svg,
            "image/png" => MimeType::Png,
            "image/jpeg" => MimeType::Jpeg,
            "image/gif" => MimeType::Gif,
            _ => MimeType::Other,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct MimeBundle {
    #[serde(flatten)]
    content: HashMap<MimeType, Value>,
}

impl MimeBundle {
    // Finds the first richest mime type in the list matching this bundle if available
    pub fn richest(&self, mime_types: &[MimeType]) -> Option<(MimeType, Value)> {
        for mime_type in mime_types {
            if let Some(content) = self.content.get(mime_type) {
                return Some((mime_type.clone(), content.clone()));
            }
        }
        None
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn richest_middle() {
        let raw = r#"{
            "text/plain": "Hello, world!",
            "text/html": "<h1>Hello, world!</h1>",
            "application/json": {
                "name": "John Doe",
                "age": 30
            },
            "application/vnd.dataresource+json": {
                "data": [
                    {"name": "Alice", "age": 25},
                    {"name": "Bob", "age": 35}
                ],
                "schema": {
                    "fields": [
                        {"name": "name", "type": "string"},
                        {"name": "age", "type": "integer"}
                    ]
                }
            },
            "application/octet-stream": "Binary data"
        }"#;

        let bundle: MimeBundle = serde_json::from_str(raw).unwrap();

        let richest = bundle.richest(&[MimeType::Html]);

        let (mime_type, content) = richest.unwrap();

        assert_eq!(mime_type, MimeType::Html);
        assert_eq!(content, "<h1>Hello, world!</h1>");
    }

    #[test]
    fn find_table() {
        let raw = r#"{
            "text/plain": "Hello, world!",
            "text/html": "<h1>Hello, world!</h1>",
            "application/json": {
                "name": "John Doe",
                "age": 30
            },
            "application/vnd.dataresource+json": {
                "data": [
                    {"name": "Alice", "age": 25},
                    {"name": "Bob", "age": 35}
                ],
                "schema": {
                    "fields": [
                        {"name": "name", "type": "string"},
                        {"name": "age", "type": "integer"}
                    ]
                }
            },
            "application/octet-stream": "Binary data"
        }"#;

        let bundle: MimeBundle = serde_json::from_str(raw).unwrap();

        let richest = bundle.richest(&[MimeType::DataTable, MimeType::Json, MimeType::Html]);

        let (mime_type, content) = richest.unwrap();

        assert_eq!(mime_type, MimeType::DataTable);
        assert_eq!(
            content,
            serde_json::json!({
                "data": [
                    {"name": "Alice", "age": 25},
                    {"name": "Bob", "age": 35}
                ],
                "schema": {
                    "fields": [
                        {"name": "name", "type": "string"},
                        {"name": "age", "type": "integer"}
                    ]
                }
            })
        );
    }
}

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, Clone)]
#[serde(rename_all = "snake_case")]
pub enum MimeType {
    /* Plaintext media types */
    #[serde(rename = "text/plain")]
    Plain,
    #[serde(rename = "text/html")]
    Html,
    #[serde(rename = "text/latex")]
    Latex,
    #[serde(rename = "application/javascript")]
    Javascript,

    /* Text based Images */
    #[serde(rename = "image/svg+xml")]
    Svg,

    /* Binary Images */
    #[serde(rename = "image/png")]
    Png,
    #[serde(rename = "image/jpeg")]
    Jpeg,
    #[serde(rename = "image/gif")]
    Gif,

    /* Pure JSON for viewing */
    #[serde(rename = "application/json")]
    Json,

    /* Special JSON Media Types that require custom renderers */
    /* GeoJSON */
    #[serde(rename = "application/geo+json")]
    GeoJson,
    /* Data Table, e.g. `{data: [], schema: {}}` */
    #[serde(rename = "application/vnd.dataresource+json")]
    DataTable,
    /* Plotly */
    #[serde(rename = "application/vnd.plotly.v1+json")]
    Plotly,
    /* Jupyter/IPython widgets */
    #[serde(rename = "application/vnd.jupyter.widget-view+json")]
    WidgetView,
    #[serde(rename = "application/vnd.jupyter.widget-state+json")]
    WidgetState,
    /* Vega & VegaLite */
    #[serde(rename = "application/vnd.vegalite.v2+json")]
    VegaLite2,
    #[serde(rename = "application/vnd.vegalite.v3+json")]
    VegaLiteV3,
    #[serde(rename = "application/vnd.vegalite.v4+json")]
    VegaLiteV4,
    #[serde(rename = "application/vnd.vegalite.v5+json")]
    VegaLiteV5,
    #[serde(rename = "application/vnd.vegalite.v6+json")]
    VegaLiteV6,
    #[serde(rename = "application/vnd.vega.v3+json")]
    VegaV3,
    #[serde(rename = "application/vnd.vega.v4+json")]
    VegaV4,
    #[serde(rename = "application/vnd.vega.v5+json")]
    VegaV5,

    /* Virtual DOM (nteract/vdom) */
    #[serde(rename = "application/vdom.v1+json")]
    Vdom,

    /* Anything goes */
    #[serde(other)]
    Other,
}

impl From<String> for MimeType {
    fn from(s: String) -> Self {
        match s.as_str() {
            "text/plain" => MimeType::Plain,
            "text/html" => MimeType::Html,
            "text/latex" => MimeType::Latex,
            "application/javascript" => MimeType::Javascript,

            "application/json" => MimeType::Json,
            "application/vnd.dataresource+json" => MimeType::DataTable,
            "application/vnd.plotly.v1+json" => MimeType::Plotly,
            "image/svg+xml" => MimeType::Svg,
            "image/png" => MimeType::Png,
            "image/jpeg" => MimeType::Jpeg,
            "image/gif" => MimeType::Gif,

            "application/vnd.jupyter.widget-view+json" => MimeType::WidgetView,
            "application/vnd.jupyter.widget-state+json" => MimeType::WidgetState,

            "application/geo+json" => MimeType::GeoJson,

            "application/vnd.vegalite.v2+json" => MimeType::VegaLite2,
            "application/vnd.vegalite.v3+json" => MimeType::VegaLiteV3,
            "application/vnd.vegalite.v4+json" => MimeType::VegaLiteV4,
            "application/vnd.vegalite.v5+json" => MimeType::VegaLiteV5,
            "application/vnd.vegalite.v6+json" => MimeType::VegaLiteV6,

            "application/vnd.vega.v3+json" => MimeType::VegaV3,
            "application/vnd.vega.v4+json" => MimeType::VegaV4,
            "application/vnd.vega.v5+json" => MimeType::VegaV5,

            "application/vdom.v1+json" => MimeType::Vdom,

            _ => MimeType::Other,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct MimeBundle {
    #[serde(flatten)]
    pub content: HashMap<MimeType, Value>,
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

    #[test]
    fn find_nothing_and_be_happy() {
        let raw = r#"{
            "application/octet-stream": "Binary data"
        }"#;

        let bundle: MimeBundle = serde_json::from_str(raw).unwrap();

        let richest = bundle.richest(&[
            MimeType::DataTable,
            MimeType::Json,
            MimeType::Html,
            MimeType::Svg,
            MimeType::Plain,
        ]);

        assert_eq!(richest, None);
    }

    #[test]
    fn from_string() {
        let mime_type: MimeType = "text/plain".to_string().into();
        assert_eq!(mime_type, MimeType::Plain);
    }

    #[test]
    fn from_string_unknown() {
        let mime_type: MimeType = "application/octet-stream".to_string().into();
        assert_eq!(mime_type, MimeType::Other);
    }

    #[test]
    fn edge_case() {
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

        let richest = bundle.richest(&[]);
        assert_eq!(richest, None);
    }

    #[test]
    fn direct_access() {
        let raw = r#"{
            "text/plain": "🦀 Hello from Rust! 🦀",
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

        assert_eq!(
            bundle.content.get(&MimeType::Html).unwrap(),
            &serde_json::json!("<h1>Hello, world!</h1>")
        );

        assert_eq!(
            bundle
                .content
                .get(&MimeType::from("text/plain".to_string()))
                .unwrap(),
            "🦀 Hello from Rust! 🦀"
        )
    }
}

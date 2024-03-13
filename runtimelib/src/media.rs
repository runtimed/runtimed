use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// An enumeration representing various MIME (Multipurpose Internet Mail Extensions) types.
/// These types are used to indicate the nature of the data in a rich content message.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, Clone)]
#[serde(rename_all = "snake_case")]
pub enum MimeType {
    /// Plain text, typically representing unformatted text. (e.g. Python's `_repr_` or `_repr_pretty_` methods).
    #[serde(rename = "text/plain")]
    Plain,
    /// HTML, (as displayed via Python's `_repr_html_` method).
    #[serde(rename = "text/html")]
    Html,
    /// LaTeX, (as displayed using Python's `_repr_latex_` method).
    #[serde(rename = "text/latex")]
    Latex,
    /// Raw JavaScript code.
    #[serde(rename = "application/javascript")]
    Javascript,
    /// Markdown text, (as displayed using Python's `_repr_markdown_` method).
    #[serde(rename = "text/markdown")]
    Markdown,

    /// SVG image text, (as displayed using Python's `_repr_svg_` method).
    #[serde(rename = "image/svg+xml")]
    Svg,

    /// PNG image data.
    #[serde(rename = "image/png")]
    Png,
    /// JPEG image data.
    #[serde(rename = "image/jpeg")]
    Jpeg,
    /// GIF image data.
    #[serde(rename = "image/gif")]
    Gif,

    /// Raw JSON Object
    #[serde(rename = "application/json")]
    Json,

    /// GeoJSON data, a format for encoding a variety of geographic data structures.
    #[serde(rename = "application/geo+json")]
    GeoJson,
    /// Data table in JSON format, requires both a `data` and `schema`.
    /// Example: `{data: [{'ghost': true, 'says': "boo"}], schema: {fields: [{name: 'ghost', type: 'boolean'}, {name: 'says', type: 'string'}]}}`.
    #[serde(rename = "application/vnd.dataresource+json")]
    DataTable,
    /// Plotly JSON Schema for for rendering graphs and charts.
    #[serde(rename = "application/vnd.plotly.v1+json")]
    Plotly,
    /// Jupyter/IPython widget view in JSON format.
    #[serde(rename = "application/vnd.jupyter.widget-view+json")]
    WidgetView,
    /// Jupyter/IPython widget state in JSON format.
    #[serde(rename = "application/vnd.jupyter.widget-state+json")]
    WidgetState,
    /// VegaLite data in JSON format for version 2 visualizations.
    #[serde(rename = "application/vnd.vegalite.v2+json")]
    VegaLite2,
    /// VegaLite data in JSON format for version 3 visualizations.
    #[serde(rename = "application/vnd.vegalite.v3+json")]
    VegaLiteV3,
    /// VegaLite data in JSON format for version 4 visualizations.
    #[serde(rename = "application/vnd.vegalite.v4+json")]
    VegaLiteV4,
    /// VegaLite data in JSON format for version 5 visualizations.
    #[serde(rename = "application/vnd.vegalite.v5+json")]
    VegaLiteV5,
    /// VegaLite data in JSON format for version 6 visualizations.
    #[serde(rename = "application/vnd.vegalite.v6+json")]
    VegaLiteV6,
    /// Vega data in JSON format for version 3 visualizations.
    #[serde(rename = "application/vnd.vega.v3+json")]
    VegaV3,
    /// Vega data in JSON format for version 4 visualizations.
    #[serde(rename = "application/vnd.vega.v4+json")]
    VegaV4,
    /// Vega data in JSON format for version 5 visualizations.
    #[serde(rename = "application/vnd.vega.v5+json")]
    VegaV5,

    /// Represents Virtual DOM (nteract/vdom) data in JSON format.
    #[serde(rename = "application/vdom.v1+json")]
    Vdom,

    /// Represents any other MIME type not listed above.
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
            "text/markdown" => MimeType::Markdown,

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

/// A `MimeBundle` is a collection of data associated with different MIME types.
/// It allows for the representation of rich content that can be displayed in multiple formats.
/// These are found in the `data` field of a `DisplayData` and `ExecuteResult` messages/output types.
///
/// # Examples
///
/// ```rust
/// use runtimelib::media::{MimeBundle, MimeType};
///
/// let raw = r#"{
///    "text/plain": "FancyThing()",
///    "text/html": "<h1>Fancy!</h1>",
///    "application/json": {"fancy": true}
/// }"#;
///
/// let mime_bundle: MimeBundle = serde_json::from_str(raw).unwrap();
///
/// let richest = mime_bundle.richest(&[MimeType::Html, MimeType::Json, MimeType::Plain]);
///
/// if let Some((mime_type, content)) = richest {
///    println!("Richest MIME type: {:?}", mime_type);
///    println!("Content: {:?}", content);
/// }
/// ```
///
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct MimeBundle {
    /// A map of MIME types to their corresponding data, represented as JSON `Value`.
    #[serde(flatten)]
    pub content: HashMap<MimeType, Value>,
}

impl MimeBundle {
    /// Find the richest media based on a priority order of MIME types.
    /// The richest content is the first MIME type in the priority order that exists in the bundle.
    ///
    /// # Arguments
    ///
    /// * `priority_order` - A slice of `MimeType` representing the desired priority order.
    ///
    /// # Returns
    ///
    /// An `Option` containing a tuple of the selected `MimeType` and its corresponding content as a `Value`.
    /// Returns `None` if none of the MIME types in the priority order are present in the bundle.
    pub fn richest(&self, priority_order: &[MimeType]) -> Option<(MimeType, Value)> {
        for mime_type in priority_order {
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

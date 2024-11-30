//! Provides types and utilities for working with rich media content in Jupyter messages.
//!
//! This module defines the `Media` and `MediaType` structures, which represent
//! MIME bundles in Jupyter messages. These are used for rich content display
//! in notebooks and other Jupyter frontends.
//!
//! The main types in this module are:
//!
//! - [`Media`]: Represents a collection of media types.
//! - [`MediaType`]: An enum representing various MIME types.
//!
//! # Examples
//!
//! Creating a media bundle with multiple types:
//!
//! ```rust
//! use jupyter_protocol::media::{Media, MediaType};
//!
//! let media = Media::new(vec![
//!     MediaType::Plain("Hello, world!".to_string()),
//!     MediaType::Html("<h1>Hello, world!</h1>".to_string()),
//! ]);
//! ```
//!
//! Finding the richest media type:
//!
//! ```rust
//! use jupyter_protocol::media::{Media, MediaType};
//!
//! let media = Media::new(vec![
//!     MediaType::Plain("Hello, world!".to_string()),
//!     MediaType::Html("<h1>Hello, world!</h1>".to_string()),
//!     MediaType::Markdown("**Hello, world!**".to_string()),
//! ]);
//!
//! let richest = media.richest(|media_type| match media_type {
//!     MediaType::Html(_) => 3,
//!     MediaType::Markdown(_) => 2,
//!     MediaType::Plain(_) => 1,
//!     _ => 0,
//! });
//!
//! assert!(matches!(richest, Some(MediaType::Html(_))));
//! ```
use serde::{de, Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

pub mod datatable;

pub use datatable::TabularDataResource;

pub type JsonObject = serde_json::Map<String, serde_json::Value>;

/// An enumeration representing various Media types, otherwise known as MIME (Multipurpose Internet Mail Extensions) types.
/// These types are used to indicate the nature of the data in a rich content message such as `DisplayData`, `UpdateDisplayData`, and `ExecuteResult`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type", content = "data")]
pub enum MediaType {
    /// Plain text, typically representing unformatted text. (e.g. Python's `_repr_` or `_repr_pretty_` methods).
    #[serde(rename = "text/plain")]
    Plain(String),
    /// HTML, (as displayed via Python's `_repr_html_` method).
    #[serde(rename = "text/html")]
    Html(String),
    /// LaTeX, (as displayed using Python's `_repr_latex_` method).
    #[serde(rename = "text/latex")]
    Latex(String),
    /// Raw JavaScript code.
    #[serde(rename = "application/javascript")]
    Javascript(String),
    /// Markdown text, (as displayed using Python's `_repr_markdown_` method).
    #[serde(rename = "text/markdown")]
    Markdown(String),

    /// SVG image text, (as displayed using Python's `_repr_svg_` method).
    #[serde(rename = "image/svg+xml")]
    Svg(String),

    // Image data is all base64 encoded. These variants could all accept <Vec<u8>> as the
    // data. However, not all users of this library will need immediate decoding of the data.
    /// PNG image data.
    #[serde(rename = "image/png")]
    Png(String),
    /// JPEG image data.
    #[serde(rename = "image/jpeg")]
    Jpeg(String),
    /// GIF image data.
    #[serde(rename = "image/gif")]
    Gif(String),

    /// Raw JSON Object
    #[serde(rename = "application/json")]
    Json(JsonObject),

    /// GeoJSON data, a format for encoding a variety of geographic data structures.
    #[serde(rename = "application/geo+json")]
    GeoJson(JsonObject),
    /// Data table in JSON format, requires both a `data` and `schema`.
    /// Example: `{data: [{'ghost': true, 'says': "boo"}], schema: {fields: [{name: 'ghost', type: 'boolean'}, {name: 'says', type: 'string'}]}}`.
    #[serde(rename = "application/vnd.dataresource+json")]
    DataTable(Box<TabularDataResource>),
    /// Plotly JSON Schema for for rendering graphs and charts.
    #[serde(rename = "application/vnd.plotly.v1+json")]
    Plotly(JsonObject),
    /// Jupyter/IPython widget view in JSON format.
    #[serde(rename = "application/vnd.jupyter.widget-view+json")]
    WidgetView(JsonObject),
    /// Jupyter/IPython widget state in JSON format.
    #[serde(rename = "application/vnd.jupyter.widget-state+json")]
    WidgetState(JsonObject),
    /// VegaLite data in JSON format for version 2 visualizations.
    #[serde(rename = "application/vnd.vegalite.v2+json")]
    VegaLiteV2(JsonObject),
    /// VegaLite data in JSON format for version 3 visualizations.
    #[serde(rename = "application/vnd.vegalite.v3+json")]
    VegaLiteV3(JsonObject),
    /// VegaLite data in JSON format for version 4 visualizations.
    #[serde(rename = "application/vnd.vegalite.v4+json")]
    VegaLiteV4(JsonObject),
    /// VegaLite data in JSON format for version 5 visualizations.
    #[serde(rename = "application/vnd.vegalite.v5+json")]
    VegaLiteV5(JsonObject),
    /// VegaLite data in JSON format for version 6 visualizations.
    #[serde(rename = "application/vnd.vegalite.v6+json")]
    VegaLiteV6(JsonObject),
    /// Vega data in JSON format for version 3 visualizations.
    #[serde(rename = "application/vnd.vega.v3+json")]
    VegaV3(JsonObject),
    /// Vega data in JSON format for version 4 visualizations.
    #[serde(rename = "application/vnd.vega.v4+json")]
    VegaV4(JsonObject),
    /// Vega data in JSON format for version 5 visualizations.
    #[serde(rename = "application/vnd.vega.v5+json")]
    VegaV5(JsonObject),

    /// Represents Virtual DOM (nteract/vdom) data in JSON format.
    #[serde(rename = "application/vdom.v1+json")]
    Vdom(JsonObject),

    // Catch all type for serde ease.
    // TODO: Implement a custom deserializer so this extra type isn't in resulting serializations.
    Other((String, Value)),
}

impl std::hash::Hash for MediaType {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match &self {
            MediaType::Plain(_) => "text/plain",
            MediaType::Html(_) => "text/html",
            MediaType::Latex(_) => "text/latex",
            MediaType::Javascript(_) => "application/javascript",
            MediaType::Markdown(_) => "text/markdown",
            MediaType::Svg(_) => "image/svg+xml",
            MediaType::Png(_) => "image/png",
            MediaType::Jpeg(_) => "image/jpeg",
            MediaType::Gif(_) => "image/gif",
            MediaType::Json(_) => "application/json",
            MediaType::GeoJson(_) => "application/geo+json",
            MediaType::DataTable(_) => "application/vnd.dataresource+json",
            MediaType::Plotly(_) => "application/vnd.plotly.v1+json",
            MediaType::WidgetView(_) => "application/vnd.jupyter.widget-view+json",
            MediaType::WidgetState(_) => "application/vnd.jupyter.widget-state+json",
            MediaType::VegaLiteV2(_) => "application/vnd.vegalite.v2+json",
            MediaType::VegaLiteV3(_) => "application/vnd.vegalite.v3+json",
            MediaType::VegaLiteV4(_) => "application/vnd.vegalite.v4+json",
            MediaType::VegaLiteV5(_) => "application/vnd.vegalite.v5+json",
            MediaType::VegaLiteV6(_) => "application/vnd.vegalite.v6+json",
            MediaType::VegaV3(_) => "application/vnd.vega.v3+json",
            MediaType::VegaV4(_) => "application/vnd.vega.v4+json",
            MediaType::VegaV5(_) => "application/vnd.vega.v5+json",
            MediaType::Vdom(_) => "application/vdom.v1+json",
            MediaType::Other((key, _)) => key.as_str(),
        }
        .hash(state)
    }
}

/// A `Media` is a collection of data associated with different Media types.
/// It allows for the representation of rich content that can be displayed in multiple formats.
/// These are found in the `data` field of a `DisplayData` and `ExecuteResult` messages/output types.
///
#[derive(Default, Serialize, Deserialize, Debug, Clone)]
pub struct Media {
    /// A map of Media types to their corresponding data, represented as JSON `Value`.
    #[serde(
        flatten,
        deserialize_with = "deserialize_media",
        serialize_with = "serialize_media_for_wire"
    )]
    pub content: Vec<MediaType>,
}

fn deserialize_media<'de, D>(deserializer: D) -> Result<Vec<MediaType>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    // Jupyter protocol does pure Map<String, Value> for media types.
    // Our deserializer goes a step further by having enums that have their data fully typed
    let map: HashMap<String, Value> = HashMap::deserialize(deserializer)?;
    let mut content = Vec::new();

    for (key, value) in map {
        // Check if the key matches ^application/(.*\\+)?json$ in order to skip the multiline string handling
        if key.starts_with("application/") && key.ends_with("json") {
            let media_type =
                match serde_json::from_value(Value::Object(serde_json::Map::from_iter([
                    ("type".to_string(), Value::String(key.clone())),
                    ("data".to_string(), value.clone()),
                ]))) {
                    Ok(mediatype) => mediatype,
                    Err(_) => MediaType::Other((key, value)),
                };
            content.push(media_type);
            continue;
        }

        // Now we know we're getting a plain string or an array of strings
        let text: String = match value.clone() {
            Value::String(s) => s,
            Value::Array(arr) => arr
                .into_iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect::<Vec<String>>()
                .join(""),
            _ => return Err(de::Error::custom("Invalid value for text-based media type")),
        };

        if key.starts_with("image/") {
            // If we ever want to turn this into Vec<u8> we could do that here. We would need to strip all the whitespace from the base64
            // encoded image too though. `let text = text.replace("\n", "").replace(" ", "");`
            // For consistency with other notebook frontends though, we'll keep it the same

            let mediatype: MediaType = match key.as_str() {
                "image/png" => MediaType::Png(text),
                "image/jpeg" => MediaType::Jpeg(text),
                "image/gif" => MediaType::Gif(text),
                _ => MediaType::Other((key.clone(), value)),
            };
            content.push(mediatype);
            continue;
        }

        let mediatype: MediaType = match key.as_str() {
            "text/plain" => MediaType::Plain(text),
            "text/html" => MediaType::Html(text),
            "text/latex" => MediaType::Latex(text),
            "application/javascript" => MediaType::Javascript(text),
            "text/markdown" => MediaType::Markdown(text),
            "image/svg+xml" => MediaType::Svg(text),

            // Keep unknown mediatypes exactly as they were
            _ => MediaType::Other((key.clone(), value)),
        };

        content.push(mediatype);
    }

    Ok(content)
}

pub fn serialize_media_for_wire<S>(
    content: &Vec<MediaType>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serialize_media_with_options(content, serializer, false)
}

pub fn serialize_media_for_notebook<S>(media: &Media, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serialize_media_with_options(&media.content, serializer, true)
}

pub fn serialize_media_with_options<S>(
    content: &Vec<MediaType>,
    serializer: S,
    with_multiline: bool,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let mut map = HashMap::new();

    for media_type in content {
        let (key, value) = match media_type {
            MediaType::Plain(text)
            | MediaType::Html(text)
            | MediaType::Latex(text)
            | MediaType::Javascript(text)
            | MediaType::Markdown(text)
            | MediaType::Svg(text) => {
                let key = match media_type {
                    MediaType::Plain(_) => "text/plain",
                    MediaType::Html(_) => "text/html",
                    MediaType::Latex(_) => "text/latex",
                    MediaType::Javascript(_) => "application/javascript",
                    MediaType::Markdown(_) => "text/markdown",
                    MediaType::Svg(_) => "image/svg+xml",
                    _ => unreachable!(),
                };
                let value = if with_multiline {
                    let lines: Vec<&str> = text.lines().collect();

                    if lines.len() > 1 {
                        let entries = lines
                            .iter()
                            .map(|line| Value::String(format!("{}\n", line)));

                        Value::Array(entries.collect())
                    } else {
                        Value::Array(vec![Value::String(text.clone())])
                    }
                } else {
                    Value::String(text.clone())
                };
                (key.to_string(), value)
            }
            // ** Treat images in a special way **
            // Jupyter, in practice, will attempt to keep the multiline version of the image around if it was written in
            // that way. We'd have to do extra tracking in order to keep this enum consistent, so this is an area
            // where we may wish to diverge from practice (not protocol or schema, just practice).
            //
            // As an example, some frontends will convert images to base64 and then split them into 80 character chunks
            // with newlines interspersed. We could perform the chunking but then in many cases we will no longer match.
            MediaType::Jpeg(text) | MediaType::Png(text) | MediaType::Gif(text) => {
                let key = match media_type {
                    MediaType::Jpeg(_) => "image/jpeg",
                    MediaType::Png(_) => "image/png",
                    MediaType::Gif(_) => "image/gif",
                    _ => unreachable!(),
                };
                let value = if with_multiline {
                    let lines: Vec<&str> = text.lines().collect();

                    if lines.len() > 1 {
                        let entries = lines
                            .iter()
                            .map(|line| Value::String(format!("{}\n", line)));

                        Value::Array(entries.collect())
                    } else {
                        Value::String(text.clone())
                    }
                } else {
                    Value::String(text.clone())
                };

                (key.to_string(), value)
            }
            // Keep unknown media types as is
            MediaType::Other((key, value)) => (key.clone(), value.clone()),
            _ => {
                let serialized =
                    serde_json::to_value(media_type).map_err(serde::ser::Error::custom)?;
                if let Value::Object(obj) = serialized {
                    if let (Some(Value::String(key)), Some(data)) =
                        (obj.get("type"), obj.get("data"))
                    {
                        (key.clone(), data.clone())
                    } else {
                        continue;
                    }
                } else {
                    continue;
                }
            }
        };
        map.insert(key, value);
    }

    map.serialize(serializer)
}

impl Media {
    /// Find the richest media type in the bundle, based on the provided ranker function.
    /// A rank of 0 indicates that the media type is not supported. Higher numbers indicate
    /// that the media type is preferred over other media types.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use jupyter_protocol::media::{Media, MediaType};
    ///
    /// let raw = r#"{
    ///    "text/plain": "FancyThing()",
    ///    "text/html": "<h1>Fancy!</h1>",
    ///    "application/json": {"fancy": true}
    /// }"#;
    ///
    /// let media: Media = serde_json::from_str(raw).unwrap();
    ///
    /// let ranker = |media_type: &MediaType| match media_type {
    ///    MediaType::Html(_) => 3,
    ///    MediaType::Json(_) => 2,
    ///    MediaType::Plain(_) => 1,
    ///    _ => 0,
    /// };
    ///
    /// let richest = media.richest(ranker);
    ///
    /// assert_eq!(
    ///    richest,
    ///    Some(MediaType::Html(String::from("<h1>Fancy!</h1>"))).as_ref()
    /// );
    ///
    /// ```
    pub fn richest(&self, ranker: fn(&MediaType) -> usize) -> Option<&MediaType> {
        self.content
            .iter()
            .filter_map(|mediatype| {
                let rank = ranker(mediatype);
                if rank > 0 {
                    Some((rank, mediatype))
                } else {
                    None
                }
            })
            .max_by_key(|(rank, _)| *rank)
            .map(|(_, mediatype)| mediatype)
    }

    pub fn new(content: Vec<MediaType>) -> Self {
        Self { content }
    }
}

impl From<MediaType> for Media {
    fn from(media_type: MediaType) -> Self {
        Media {
            content: vec![media_type],
        }
    }
}

impl From<Vec<MediaType>> for Media {
    fn from(content: Vec<MediaType>) -> Self {
        Media { content }
    }
}

// Backwards compatibility with previous versions and Jupyter naming
pub type MimeBundle = Media;
pub type MimeType = MediaType;

#[cfg(test)]
mod test {
    use datatable::TableSchemaField;
    use serde_json::json;

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

        let bundle: Media = serde_json::from_str(raw).unwrap();

        let ranker = |mediatype: &MediaType| match mediatype {
            MediaType::Plain(_) => 1,
            MediaType::Html(_) => 2,
            _ => 0,
        };

        match bundle.richest(ranker) {
            Some(MediaType::Html(data)) => assert_eq!(data, "<h1>Hello, world!</h1>"),
            _ => panic!("Unexpected media type"),
        }
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

        let bundle: Media = serde_json::from_str(raw).unwrap();

        let ranker = |mediatype: &MediaType| match mediatype {
            MediaType::Html(_) => 1,
            MediaType::Json(_) => 2,
            MediaType::DataTable(_) => 3,
            _ => 0,
        };

        let richest = bundle.richest(ranker);

        match richest {
            Some(MediaType::DataTable(table)) => {
                assert_eq!(
                    table.data,
                    Some(vec![
                        json!({"name": "Alice", "age": 25}),
                        json!({"name": "Bob", "age": 35})
                    ])
                );
                assert_eq!(
                    table.schema.fields,
                    vec![
                        TableSchemaField {
                            name: "name".to_string(),
                            field_type: datatable::FieldType::String,
                            ..Default::default()
                        },
                        TableSchemaField {
                            name: "age".to_string(),
                            field_type: datatable::FieldType::Integer,
                            ..Default::default()
                        }
                    ]
                );
            }
            _ => panic!("Unexpected mime type"),
        }
    }

    #[test]
    fn find_nothing_and_be_happy() {
        let raw = r#"{
            "application/fancy": "Too ✨ Fancy ✨ for you!"
        }"#;

        let bundle: Media = serde_json::from_str(raw).unwrap();

        let ranker = |mediatype: &MediaType| match mediatype {
            MediaType::Html(_) => 1,
            MediaType::Json(_) => 2,
            MediaType::DataTable(_) => 3,
            _ => 0,
        };

        let richest = bundle.richest(ranker);

        assert_eq!(richest, None);

        assert!(bundle.content.contains(&MediaType::Other((
            "application/fancy".to_string(),
            json!("Too ✨ Fancy ✨ for you!")
        ))));
    }

    #[test]
    fn no_media_type_supported() {
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

        let bundle: Media = serde_json::from_str(raw).unwrap();
        let richest = bundle.richest(|_| 0);
        assert_eq!(richest, None);
    }

    #[test]
    fn ensure_array_of_text_processed() {
        let raw = r#"{
            "text/plain": ["Hello, world!"],
            "text/html": "<h1>Hello, world!</h1>"
        }"#;

        let bundle: Media = serde_json::from_str(raw).unwrap();

        assert_eq!(bundle.content.len(), 2);
        assert!(bundle
            .content
            .contains(&MediaType::Plain("Hello, world!".to_string())));
        assert!(bundle
            .content
            .contains(&MediaType::Html("<h1>Hello, world!</h1>".to_string())));

        let raw = r#"{
            "text/plain": ["Hello, world!\n", "Welcome to zombo.com"],
            "text/html": ["<h1>\n", "  Hello, world!\n", "</h1>"]
        }"#;

        let bundle: Media = serde_json::from_str(raw).unwrap();

        assert_eq!(bundle.content.len(), 2);
        assert!(bundle.content.contains(&MediaType::Plain(
            "Hello, world!\nWelcome to zombo.com".to_string()
        )));
        assert!(bundle
            .content
            .contains(&MediaType::Html("<h1>\n  Hello, world!\n</h1>".to_string())));
    }
}

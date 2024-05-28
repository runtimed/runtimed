use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

pub mod datatable;

pub use datatable::TabularDataResource;

type JsonObject = serde_json::Map<String, serde_json::Value>;

/// An enumeration representing various MIME (Multipurpose Internet Mail Extensions) types.
/// These types are used to indicate the nature of the data in a rich content message.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type", content = "data")]
pub enum MimeType {
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
    DataTable(TabularDataResource),
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

impl std::hash::Hash for MimeType {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let mime_type_str = match &self {
            MimeType::Plain(_) => "text/plain",
            MimeType::Html(_) => "text/html",
            MimeType::Latex(_) => "text/latex",
            MimeType::Javascript(_) => "application/javascript",
            MimeType::Markdown(_) => "text/markdown",
            MimeType::Svg(_) => "image/svg+xml",
            MimeType::Png(_) => "image/png",
            MimeType::Jpeg(_) => "image/jpeg",
            MimeType::Gif(_) => "image/gif",
            MimeType::Json(_) => "application/json",
            MimeType::GeoJson(_) => "application/geo+json",
            MimeType::DataTable(_) => "application/vnd.dataresource+json",
            MimeType::Plotly(_) => "application/vnd.plotly.v1+json",
            MimeType::WidgetView(_) => "application/vnd.jupyter.widget-view+json",
            MimeType::WidgetState(_) => "application/vnd.jupyter.widget-state+json",
            MimeType::VegaLiteV2(_) => "application/vnd.vegalite.v2+json",
            MimeType::VegaLiteV3(_) => "application/vnd.vegalite.v3+json",
            MimeType::VegaLiteV4(_) => "application/vnd.vegalite.v4+json",
            MimeType::VegaLiteV5(_) => "application/vnd.vegalite.v5+json",
            MimeType::VegaLiteV6(_) => "application/vnd.vegalite.v6+json",
            MimeType::VegaV3(_) => "application/vnd.vega.v3+json",
            MimeType::VegaV4(_) => "application/vnd.vega.v4+json",
            MimeType::VegaV5(_) => "application/vnd.vega.v5+json",
            MimeType::Vdom(_) => "application/vdom.v1+json",
            MimeType::Other((key, _)) => key.as_str(),
        };

        mime_type_str.hash(state);
    }
}

/// A `MimeBundle` is a collection of data associated with different MIME types.
/// It allows for the representation of rich content that can be displayed in multiple formats.
/// These are found in the `data` field of a `DisplayData` and `ExecuteResult` messages/output types.
///
#[derive(Default, Serialize, Deserialize, Debug, Clone)]
pub struct MimeBundle {
    /// A map of MIME types to their corresponding data, represented as JSON `Value`.
    #[serde(flatten, deserialize_with = "deserialize_mimebundle")]
    pub content: HashSet<MimeType>,
}

fn deserialize_mimebundle<'de, D>(deserializer: D) -> Result<HashSet<MimeType>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let map: HashMap<String, Value> = HashMap::deserialize(deserializer)?;
    let mut set = HashSet::new();

    for (key, value) in map {
        let mut mime_value_map = serde_json::Map::new();
        mime_value_map.insert("type".to_string(), Value::String(key.clone()));
        mime_value_map.insert("data".to_string(), value.clone());

        let mime_type: MimeType = match serde_json::from_value(Value::Object(mime_value_map)) {
            Ok(mime_type) => mime_type,
            Err(_) => MimeType::Other((key, value)),
        };

        set.insert(mime_type);
    }

    Ok(set)
}

impl MimeBundle {
    /// Find the richest MIME type in the bundle, based on the provided ranker function.
    /// A rank of 0 indicates that the MIME type is not supported. Higher numbers indicate
    /// that the MIME type is preferred over other MIME types.
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
    /// let ranker = |mime_type: &MimeType| match mime_type {
    ///    MimeType::Html(_) => 3,
    ///    MimeType::Json(_) => 2,
    ///    MimeType::Plain(_) => 1,
    ///    _ => 0,
    /// };
    ///
    /// let richest = mime_bundle.richest(ranker);
    ///
    /// assert_eq!(
    ///    richest,
    ///    Some(MimeType::Html(String::from("<h1>Fancy!</h1>"))).as_ref()
    /// );
    ///
    /// ```
    pub fn richest(&self, ranker: fn(&MimeType) -> usize) -> Option<&MimeType> {
        self.content
            .iter()
            .filter_map(|mimetype| {
                let rank = ranker(mimetype);
                if rank > 0 {
                    Some((rank, mimetype))
                } else {
                    None
                }
            })
            .max_by_key(|(rank, _)| *rank)
            .map(|(_, mimetype)| mimetype)
    }
}

#[cfg(test)]
mod test {
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

        let bundle: MimeBundle = serde_json::from_str(raw).unwrap();

        let ranker = |mime_type: &MimeType| match mime_type {
            MimeType::Plain(_) => 1,
            MimeType::Html(_) => 2,
            _ => 0,
        };

        match bundle.richest(ranker) {
            Some(MimeType::Html(data)) => assert_eq!(data, "<h1>Hello, world!</h1>"),
            _ => panic!("Unexpected mime type"),
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

        let bundle: MimeBundle = serde_json::from_str(raw).unwrap();

        let ranker = |mime_type: &MimeType| match mime_type {
            MimeType::Html(_) => 1,
            MimeType::Json(_) => 2,
            MimeType::DataTable(_) => 3,
            _ => 0,
        };

        let richest = bundle.richest(ranker);

        match richest {
            Some(MimeType::DataTable(data)) => {
                assert_eq!(
                    data["data"],
                    json!([{"name": "Alice", "age": 25}, {"name": "Bob", "age": 35}])
                );
                assert_eq!(
                    data["schema"]["fields"],
                    json!([{"name": "name", "type": "string"}, {"name": "age", "type": "integer"}])
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

        let bundle: MimeBundle = serde_json::from_str(raw).unwrap();

        let ranker = |mime_type: &MimeType| match mime_type {
            MimeType::Html(_) => 1,
            MimeType::Json(_) => 2,
            MimeType::DataTable(_) => 3,
            _ => 0,
        };

        let richest = bundle.richest(ranker);

        assert_eq!(richest, None);

        assert!(bundle.content.contains(&MimeType::Other((
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

        let bundle: MimeBundle = serde_json::from_str(raw).unwrap();
        let richest = bundle.richest(|_| 0);
        assert_eq!(richest, None);
    }
}

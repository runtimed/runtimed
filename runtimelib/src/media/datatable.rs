use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// See https://specs.frictionlessdata.io/tabular-data-resource/
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TabularDataResource {
    pub path: Option<PathOrPaths>,
    pub data: Option<Vec<serde_json::Value>>,
    pub schema: TableSchema,

    pub title: Option<String>,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub sources: Option<Vec<Source>>,
    pub licenses: Option<Vec<License>>,
    pub dialect: Option<Dialect>,
    pub format: Option<String>,
    pub mediatype: Option<String>,
    pub encoding: Option<String>,
    pub bytes: Option<i64>,
    pub hash: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum PathOrPaths {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TableSchema {
    pub fields: Vec<TableSchemaField>,
    pub primary_key: Option<PrimaryKey>,
    pub foreign_keys: Option<Vec<ForeignKey>>,
    pub missing_values: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase", untagged)]
pub enum PrimaryKey {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ForeignKey {
    pub fields: ForeignKeyFields,
    pub reference: ForeignKeyReference,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum ForeignKeyFields {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ForeignKeyReference {
    pub resource: String,
    pub fields: ForeignKeyFields,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TableSchemaField {
    pub name: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub example: Option<String>,
    #[serde(rename = "type")]
    pub field_type: FieldType,
    pub format: Option<FieldFormat>,
    pub constraints: Option<HashMap<String, serde_json::Value>>,
    pub rdf_type: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    String,
    Number,
    Integer,
    Date,
    Time,
    Datetime,
    Year,
    Yearmonth,
    Boolean,
    Object,
    Geopoint,
    Geojson,
    Array,
    Duration,
    Any,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FieldFormat {
    Default,
    Email,
    Uri,
    Binary,
    Uuid,
    Any,
    Array,
    Object,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub title: String,
    pub path: Option<String>,
    pub email: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct License {
    pub name: Option<String>,
    pub path: Option<String>,
    pub title: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Dialect {
    pub delimiter: Option<String>,
    pub double_quote: Option<bool>,
    pub line_terminator: Option<String>,
    pub null_sequence: Option<String>,
    pub quote_char: Option<String>,
    pub escape_char: Option<String>,
    pub skip_initial_space: Option<bool>,
    pub header: Option<bool>,
    pub comment_char: Option<String>,
    pub case_sensitive_header: Option<bool>,
}

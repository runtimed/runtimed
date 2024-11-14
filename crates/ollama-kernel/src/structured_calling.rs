use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Structured {
    Completions(Completions),
}

#[derive(Deserialize, Serialize)]
pub struct Completions {
    pub options: Vec<String>,
}

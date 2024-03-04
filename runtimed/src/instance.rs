use serde::Deserialize;

#[derive(Deserialize)]
pub struct RuntimeInstanceRunCode {
    pub code: String,
}

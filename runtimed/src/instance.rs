use serde::Deserialize;

#[derive(Deserialize)]
pub struct RuntimeInstanceRunCode {
    pub code: String,
}

#[derive(Deserialize)]
pub struct NewRuntimeInstance {
    pub environment: String,
}

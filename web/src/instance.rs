use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct CreateRuntimeInstance {
    pub process: String,
}

#[derive(Serialize, Clone)]
pub struct RuntimeInstance {
    pub id: Uuid,
    pub name: String,
}

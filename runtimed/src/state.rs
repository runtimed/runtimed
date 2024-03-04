use runtimelib::jupyter::client::JupyterRuntime;
use sqlx::Pool;
use sqlx::Sqlite;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

pub type RuntimesLock = Arc<RwLock<HashMap<Uuid, JupyterRuntime>>>;

#[derive(Clone)]
pub struct AppState {
    pub dbpool: Pool<Sqlite>,
    pub runtimes: RuntimesLock,
}

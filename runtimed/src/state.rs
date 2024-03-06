use crate::runtime_manager::RuntimeManager;
use sqlx::Pool;
use sqlx::Sqlite;

#[derive(Clone)]
pub struct AppState {
    pub dbpool: Pool<Sqlite>,
    pub runtimes: RuntimeManager,
}

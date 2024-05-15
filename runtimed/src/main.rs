use anyhow::Error;
use axum::{routing::get, Router};
use sqlx::sqlite::SqlitePoolOptions;
use std::net::IpAddr;
use std::net::SocketAddr;

const IP: &str = "0.0.0.0";
const PORT: u16 = 12397;
// TODO: Instead of the rwc flag. Actually test if db exists and log if new db is created
const DB_STRING: &str = "sqlite:runtimed.db?mode=rwc";

mod child_runtime;
mod db;
mod execution;
mod instance;
mod routes;
mod runtime_manager;
mod state;

fn init_logger() {
    let level = if cfg!(debug_assertions) {
        "debug"
    } else {
        "info"
    };
    std::env::set_var("RUST_LOG", level);
    env_logger::init();
}

type AxumSharedState = axum::extract::State<state::AppState>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    init_logger();

    let ip: IpAddr = IP.parse().expect("Could not parse IP Address");
    let addr = SocketAddr::from((ip, PORT));
    let dbpool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(DB_STRING)
        .await?;

    log::debug!("Running migrations");
    sqlx::migrate!("../migrations").run(&dbpool).await?;

    log::debug!("Database connected and migrations run");

    // Channel for graceful shutdown of the server
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let runtimes = runtime_manager::RuntimeManager::new(&dbpool, Some(shutdown_tx)).await?;

    log::debug!("Runtimes initialized");
    let shared_state = state::AppState { dbpool, runtimes };
    let app = Router::new()
        .merge(routes::instance_routes())
        .route("/", get(get_root))
        .with_state(shared_state.clone());

    let listener = tokio::net::TcpListener::bind(addr).await?;
    log::info!("Listening on {}:{}", IP, PORT);

    axum::serve(listener, app)
        .with_graceful_shutdown(async { shutdown_rx.await.expect("Err: shutdown_tx was dropped") })
        .await
        .map_err(|e| e.into())
}

async fn get_root() -> &'static str {
    "Welcome to RunTimeD"
}

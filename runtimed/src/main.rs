use anyhow::Error;
use axum::{routing::get, Router};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::Pool;
use sqlx::Sqlite;
use std::net::IpAddr;
use std::net::SocketAddr;

const IP: &str = "0.0.0.0";
const PORT: u16 = 12397;
// TODO: Instead of the rwc flag. Actually test if db exists and log if new db is created
const DB_STRING: &str = "sqlite:runtimed.db?mode=rwc";

mod instance;
mod routes;
mod startup;
mod db;

fn init_logger() {
    let level = if cfg!(debug_assertions) {
        "debug"
    } else {
        "info"
    };
    std::env::set_var("RUST_LOG", level);
    env_logger::init();
}

#[derive(Clone)]
pub struct AppState {
    dbpool: Pool<Sqlite>,
}

type SharedState = AppState;
type AxumSharedState = axum::extract::State<SharedState>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    init_logger();

    let ip: IpAddr = IP.parse().expect("Could not parse IP Address");
    let addr = SocketAddr::from((ip, PORT));
    let dbpool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(DB_STRING)
        .await?;
    sqlx::migrate!("../migrations").run(&dbpool).await?;

    let shared_state = AppState { dbpool };
    let app = Router::new()
        .merge(routes::instance_routes())
        .route("/", get(get_root))
        .with_state(shared_state.clone());

    // Background threads to process all existing runtimes
    tokio::spawn(startup::startup(shared_state.clone()));

    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("Listening on {}:{}", IP, PORT);

    axum::serve(listener, app).await?;

    Ok(())
}

async fn get_root() -> &'static str {
    "Welcome to RunTimeD"
}

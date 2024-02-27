use axum::{routing::get, Router};
use std::net::IpAddr;
use std::net::SocketAddr;
use sqlx::sqlite::SqlitePoolOptions;
use anyhow::Error;
use sqlx::Pool;
use sqlx::Sqlite;

const IP: &str = "0.0.0.0";
const PORT: u16 = 12397;
// TODO: Instead of the rwc flag. Actually test if db exists and log if new db is created
const DB_STRING: &str = "sqlite:runtimed.db?mode=rwc";

pub mod routes;
pub mod instance;

#[derive(Clone)]
pub struct AppState {
    dbpool: Pool<Sqlite>,
}

type SharedState = AppState;
type AxumSharedState = axum::extract::State<SharedState>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let ip: IpAddr = IP.parse().expect("Could not parse IP Address");
    let addr = SocketAddr::from((ip, PORT));
    let dbpool = SqlitePoolOptions::new()
                .max_connections(5)
                .connect(DB_STRING)
                .await?;
    sqlx::migrate!("../migrations").run(&dbpool).await?;

    let shared_state = AppState {dbpool};
    let app = Router::new()
        .merge(routes::instance_routes())
        .route("/", get(get_root))
        .with_state(shared_state);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("Listening on {}:{}", IP, PORT);

    axum::serve(listener, app).await?;

    Ok(())
}

async fn get_root() -> &'static str {
    "Welcome to RunTimeD"
}

use axum::{http::StatusCode, routing::get, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::net::SocketAddr;
const IP: &str = "0.0.0.0";
const PORT: u16 = 63409;

#[derive(Debug)]
pub enum EnvError {
    InvalidUnicode(String),
}

// TODO: Get rid of these unwrap statements
#[tokio::main]
async fn main() -> Result<(), EnvError> {
    let ip: IpAddr = IP.parse().unwrap();
    let addr = SocketAddr::from((ip, PORT));

    let app = Router::new()
        .route("/", get(root))
        .route("/v2/create_runtime", post(create_runtime));

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

async fn root() -> &'static str {
    "Hello, World!\n\nWelcome to RunTimeD"
}

async fn create_runtime(Json(payload): Json<CreateRuntime>) -> (StatusCode, Json<Runtime>) {
    let runtime = Runtime {
        process: payload.process,
    };

    // TODO: Actually launch and track a runtime instance

    (StatusCode::CREATED, Json(runtime))
}

#[derive(Deserialize)]
struct CreateRuntime {
    process: String,
}

#[derive(Serialize)]
struct Runtime {
    process: String,
}

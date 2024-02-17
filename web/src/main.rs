use axum::{extract::State, http::StatusCode, routing::get, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
const IP: &str = "0.0.0.0";
const PORT: u16 = 12397;

#[derive(Debug)]
pub enum EnvError {
    InvalidUnicode(String),
}

struct AppState {
    runtime_instances: HashMap<String, RuntimeInstance>,
}

// TODO: Get rid of these unwrap statements
#[tokio::main]
async fn main() -> Result<(), EnvError> {
    let ip: IpAddr = IP.parse().unwrap();
    let addr = SocketAddr::from((ip, PORT));
    let shared_state = Arc::new(RwLock::new(AppState {
        runtime_instances: HashMap::new(),
    }));
    let app = Router::new()
        .route("/", get(get_root))
        .route("/v0/create_instance", post(post_create_runtime_instance))
        .route("/v0/instances", get(get_runtime_instances))
        .with_state(shared_state);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

async fn get_root() -> &'static str {
    "Hello, World!\n\nWelcome to RunTimeD"
}

async fn get_runtime_instances(
    State(state): State<Arc<RwLock<AppState>>>,
) -> (StatusCode, Json<HashMap<String, RuntimeInstance>>) {
    let instances = state.read().unwrap().runtime_instances.clone();
    (StatusCode::CREATED, Json(instances))
}

async fn post_create_runtime_instance(
    State(state): State<Arc<RwLock<AppState>>>,
    Json(payload): Json<CreateRuntimeInstance>,
) -> (StatusCode, Json<RuntimeInstance>) {
    let runtime = RuntimeInstance {
        process: payload.process,
    };

    state
        .write()
        .unwrap()
        .runtime_instances
        .insert("a".to_string(), runtime.clone());

    (StatusCode::CREATED, Json(runtime))
}

#[derive(Deserialize)]
struct CreateRuntimeInstance {
    process: String,
}

#[derive(Serialize, Clone)]
struct RuntimeInstance {
    process: String,
}

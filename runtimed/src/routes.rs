use crate::db::DbJupyterMessage;
use crate::instance::RuntimeInstanceRunCode;
use crate::state::AppState;
use crate::AxumSharedState;
use axum::{
    extract::Path, extract::State, http::StatusCode, routing::get, routing::post, Json, Router,
};
use runtimelib::jupyter::client::JupyterRuntime;
use serde_json::Value;
use uuid::Uuid;

pub fn instance_routes() -> Router<AppState> {
    Router::new()
        .route("/v0/runtime_instances/:id", get(get_runtime_instance))
        .route("/v0/runtime_instances", get(get_runtime_instances))
        .route(
            "/v0/runtime_instances/:id/run_code",
            post(post_runtime_instance_run_code),
        )
        .route("/v0/executions/:msg_id", get(get_executions))
}

async fn get_runtime_instances(
    State(state): AxumSharedState,
) -> Result<Json<Vec<JupyterRuntime>>, StatusCode> {
    let runtimes = state.runtimes.read().await;
    Ok(Json(runtimes.clone().into_values().collect()))
}

async fn get_runtime_instance(
    Path(id): Path<Uuid>,
    State(state): AxumSharedState,
) -> Result<Json<JupyterRuntime>, StatusCode> {
    let runtimes = state.runtimes.read().await;
    let instance = runtimes.get(&id).ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(instance.clone()))
}

async fn post_runtime_instance_run_code(
    Path(id): Path<Uuid>,
    State(state): AxumSharedState,
    Json(payload): Json<RuntimeInstanceRunCode>,
) -> Result<Json<Value>, StatusCode> {
    let runtimes = state.runtimes.read().await;
    let instance = runtimes.get(&id).ok_or(StatusCode::NOT_FOUND)?.clone();

    let mut client = instance
        .attach()
        .await
        .or(Err(StatusCode::INTERNAL_SERVER_ERROR))?;

    let (message, response) = client
        .run_code(&payload.code)
        .await
        .or(Err(StatusCode::INTERNAL_SERVER_ERROR))?;

    crate::db::insert_message(&state.dbpool, id, &message).await;
    crate::db::insert_message(&state.dbpool, id, &response).await;

    Ok(axum::Json(message.header))
}

async fn get_executions(
    Path(id): Path<Uuid>,
    State(state): AxumSharedState,
) -> Result<Json<Vec<DbJupyterMessage>>, StatusCode> {
    let messages = crate::db::get_messages_by_parent_id(&state.dbpool, id)
        .await
        .or(Err(StatusCode::INTERNAL_SERVER_ERROR))?;

    Ok(axum::Json(messages))
}

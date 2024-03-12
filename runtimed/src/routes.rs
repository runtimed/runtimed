use crate::db::DbJupyterMessage;
use crate::instance::RuntimeInstanceRunCode;
use crate::runtime_manager::RuntimeInstance;
use crate::state::AppState;
use crate::AxumSharedState;
use axum::{
    extract::Path,
    extract::State,
    http::StatusCode,
    response::sse::{Event, KeepAlive, Sse},
    routing::get,
    routing::post,
    Json, Router,
};
use futures::stream::Stream;
use runtimelib::jupyter::request::ExecuteRequest;
use serde_json::json;
use serde_json::Value;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use uuid::Uuid;

pub fn instance_routes() -> Router<AppState> {
    Router::new()
        .route("/v0/runtime_instances/:id", get(get_runtime_instance))
        .route(
            "/v0/runtime_instances/:id/attach",
            get(get_runtime_instance_attach),
        )
        .route("/v0/runtime_instances", get(get_runtime_instances))
        .route(
            "/v0/runtime_instances/:id/run_code",
            post(post_runtime_instance_run_code),
        )
        .route("/v0/executions/:msg_id", get(get_executions))
}

async fn get_runtime_instances(
    State(state): AxumSharedState,
) -> Result<Json<Vec<RuntimeInstance>>, StatusCode> {
    let runtimes = state.runtimes.get_all().await.collect();
    Ok(Json(runtimes))
}

async fn get_runtime_instance(
    Path(id): Path<Uuid>,
    State(state): AxumSharedState,
) -> Result<Json<RuntimeInstance>, StatusCode> {
    let instance = state.runtimes.get(id).await.ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(instance))
}

async fn post_runtime_instance_run_code(
    Path(id): Path<Uuid>,
    State(state): AxumSharedState,
    Json(payload): Json<RuntimeInstanceRunCode>,
) -> Result<Json<Value>, StatusCode> {
    let instance = state.runtimes.get(id).await.ok_or(StatusCode::NOT_FOUND)?;
    let sender = instance.get_sender().await;

    let execute = ExecuteRequest {
        code: payload.code,
        silent: false,
        store_history: true,
        user_expressions: Default::default(),
        allow_stdin: false,
    };

    // Not quite finished converting over here

    let response = message.header.clone();

    sender
        .send(message)
        .await
        .or(Err(StatusCode::INTERNAL_SERVER_ERROR))?;

    Ok(axum::Json(response))
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

async fn get_runtime_instance_attach(
    Path(id): Path<Uuid>,
    State(state): AxumSharedState,
) -> Result<Sse<impl Stream<Item = Result<Event, anyhow::Error>>>, StatusCode> {
    let instance = state.runtimes.get(id).await.ok_or(StatusCode::NOT_FOUND)?;
    let receiver = instance.get_receiver().await;

    let broadcast_stream = BroadcastStream::new(receiver);
    let sse_stream = broadcast_stream.map(|event| {
        let event = event?;
        let event = serde_json::to_string(&event)?;
        Ok(Event::default().data(event))
    });

    Ok(Sse::new(sse_stream).keep_alive(KeepAlive::default()))
}

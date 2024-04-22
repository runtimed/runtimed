use crate::instance::RuntimeInstanceRunCode;
use crate::runtime_manager::RuntimeInstance;
use crate::state::AppState;
use crate::AxumSharedState;
use crate::{db::DbJupyterMessage, instance::NewRuntimeInstance};
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
use runtimelib::jupyter::client::RuntimeId;
use runtimelib::jupyter::KernelspecDir;
use runtimelib::messaging::{CodeExecutionOutput, ExecuteRequest, Header, JupyterMessage};

use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use uuid::Uuid;

pub fn instance_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/v0/runtime_instances/:id",
            get(get_runtime_instance).delete(delete_runtime_instance),
        )
        .route(
            "/v0/runtime_instances/:id/attach",
            get(get_runtime_instance_attach),
        )
        .route(
            "/v0/runtime_instances",
            get(get_runtime_instances).post(post_runtime_instance),
        )
        .route(
            "/v0/runtime_instances/:id/run_code",
            post(post_runtime_instance_run_code),
        )
        .route(
            "/v0/runtime_instances/:id/eval_code",
            post(post_runtime_instance_eval_code),
        )
        .route("/v0/executions/:msg_id", get(get_executions))
        .route("/v0/environments", get(get_environments))
}

async fn get_runtime_instances(
    State(state): AxumSharedState,
) -> Result<Json<Vec<RuntimeInstance>>, StatusCode> {
    let runtimes = state.runtimes.get_all().await.collect();
    Ok(Json(runtimes))
}

async fn get_runtime_instance(
    Path(id): Path<RuntimeId>,
    State(state): AxumSharedState,
) -> Result<Json<RuntimeInstance>, StatusCode> {
    let instance = state.runtimes.get(id).await.ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(instance))
}

async fn delete_runtime_instance(
    Path(id): Path<RuntimeId>,
    State(state): AxumSharedState,
) -> Result<(), StatusCode> {
    log::info!("Deleting runtime: {id}");
    let runtime = state.runtimes.get(id).await;
    log::debug!("During delete: got runtime result");
    if let Some(runtime) = runtime {
        log::debug!("Got some runtime");
        runtime
            .stop()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(())
    } else {
        log::debug!("Got None runtime");
        Err(StatusCode::NOT_FOUND)
    }
}

async fn post_runtime_instance(
    State(state): AxumSharedState,
    Json(payload): Json<NewRuntimeInstance>,
) -> Result<Json<RuntimeInstance>, StatusCode> {
    log::info!("Starting new runtime of type: {}", payload.environment);
    match state.runtimes.new_instance(&payload.environment).await {
        Ok(id) => {
            let instance = state
                .runtimes
                .get(id)
                .await
                .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
            log::info!(
                "Created new instance (environment {}): {}",
                payload.environment,
                instance.runtime.id
            );
            Ok(Json(instance))
        }
        Err(error) => {
            log::error!("Failed to create new instance - {error}");
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

async fn post_runtime_instance_run_code(
    Path(id): Path<RuntimeId>,
    State(state): AxumSharedState,
    Json(payload): Json<RuntimeInstanceRunCode>,
) -> Result<Json<Header>, StatusCode> {
    let instance = state.runtimes.get(id).await.ok_or(StatusCode::NOT_FOUND)?;
    let sender = instance.get_sender().await;

    let execute_request = ExecuteRequest {
        code: payload.code,
        silent: false,
        store_history: true,
        user_expressions: Default::default(),
        allow_stdin: false,
    };
    let message: JupyterMessage = execute_request.into();

    let response = message.header.clone();

    sender
        .send(message)
        .await
        .or(Err(StatusCode::INTERNAL_SERVER_ERROR))?;

    Ok(axum::Json(response))
}

/// Return a struct of all the results of a code execution. Since this is a
/// structured response, it is not streamed.
async fn post_runtime_instance_eval_code(
    Path(id): Path<RuntimeId>,
    State(state): AxumSharedState,
    Json(payload): Json<RuntimeInstanceRunCode>,
) -> Result<Json<CodeExecutionOutput>, StatusCode> {
    let instance = state.runtimes.get(id).await.ok_or(StatusCode::NOT_FOUND)?;
    let sender = instance.get_sender().await;
    let mut broadcaster = instance.get_receiver().await;

    let execute_request = ExecuteRequest {
        code: payload.code,
        silent: false,
        store_history: true,
        user_expressions: Default::default(),
        allow_stdin: false,
    };
    let message: JupyterMessage = execute_request.into();

    let mut response = CodeExecutionOutput::new(message.header.clone());

    sender
        .send(message)
        .await
        .or(Err(StatusCode::INTERNAL_SERVER_ERROR))?;

    loop {
        match broadcaster.recv().await {
            Ok(output_msg) => {
                log::debug!("Got output message: {:?}", output_msg);
                response.add_message(output_msg);
                if response.is_complete() {
                    break;
                }
            }
            _ => {
                log::debug!("Got None");
                break;
            }
        }
    }

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
    Path(id): Path<RuntimeId>,
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

async fn get_environments() -> Result<Json<Vec<KernelspecDir>>, StatusCode> {
    let kernelspecs = runtimelib::jupyter::list_kernelspecs().await;
    Ok(Json(kernelspecs))
}

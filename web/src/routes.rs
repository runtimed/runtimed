use crate::instance::CreateRuntimeInstance;
use crate::instance::RuntimeInstance;
use crate::AxumSharedState;
use crate::SharedState;
use axum::{extract::State, http::StatusCode, routing::get, routing::post, Json, Router};
use uuid::Uuid;

pub fn instance_routes() -> Router<SharedState> {
    Router::new()
        .route("/v0/runtime_instances", post(post_runtime_instance))
        .route("/v0/runtime_instances", get(get_runtime_instances))
}

async fn get_runtime_instances(
    State(state): AxumSharedState,
) -> (StatusCode, Json<Vec<RuntimeInstance>>) {
    let instances = sqlx::query_as!(
        RuntimeInstance,
        r#"SELECT id "id: uuid::Uuid", name FROM runtime_instances;"#
    )
    .fetch_all(&state.dbpool)
    .await
    .unwrap();

    (StatusCode::CREATED, Json(instances))
}

async fn post_runtime_instance(
    State(state): AxumSharedState,
    Json(payload): Json<CreateRuntimeInstance>,
) -> (StatusCode, Json<RuntimeInstance>) {
    let instance = RuntimeInstance {
        id: Uuid::new_v4(),
        name: payload.process,
    };

    sqlx::query_as!(
        RuntimeInstance,
        r#"INSERT INTO runtime_instances VALUES($1, $2)"#,
        instance.id,
        instance.name,
    )
    .execute(&state.dbpool)
    .await
    .unwrap();

    (StatusCode::CREATED, Json(instance))
}

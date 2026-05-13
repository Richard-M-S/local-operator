use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use crate::{app_state::AppState, error::AppError};

#[derive(Debug, Deserialize)]
pub struct CreateOpTaskRequest {
    pub name: String,
    pub task_type: String,
    pub description: Option<String>,
    #[serde(default)]
    pub input_json: Value,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

pub async fn create(
    State(state): State<AppState>,
    Json(req): Json<CreateOpTaskRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let _input_json = req.input_json;

    let task = state
        .op_tasks
        .create_task(req.task_type, req.name, req.description, req.enabled)
        .await?;

    Ok(Json(serde_json::json!({ "task": task })))
}

pub async fn list(State(state): State<AppState>) -> Result<Json<serde_json::Value>, AppError> {
    let tasks = state.op_tasks.list_tasks().await?;
    Ok(Json(serde_json::json!({ "items": tasks })))
}

pub async fn run(
    State(state): State<AppState>,
    Path(task_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let run = state.op_tasks.run_task(task_id).await?;
    Ok(Json(serde_json::json!({ "run": run })))
}

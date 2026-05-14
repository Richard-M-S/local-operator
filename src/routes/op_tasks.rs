use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::{
    app_state::AppState,
    context::models::SavedContext,
    error::AppError,
    op_tasks::models::{
        ArtifactSearch, OpTask, OpTaskRun, PromoteArtifactToContextRequest, TaskArtifact,
    },
};

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

#[derive(Serialize)]
pub struct TaskResponse {
    pub task: OpTask,
}

#[derive(Serialize)]
pub struct RunResponse {
    pub run: OpTaskRun,
}

#[derive(Serialize)]
pub struct ListRunsResponse {
    pub runs: Vec<OpTaskRun>,
}

#[derive(Debug, Deserialize)]
pub struct ListArtifactsQuery {
    pub run_id: Option<Uuid>,
    pub task_id: Option<Uuid>,
    pub artifact_type: Option<String>,
    pub source_url: Option<String>,
    pub include_content: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl From<ListArtifactsQuery> for ArtifactSearch {
    fn from(query: ListArtifactsQuery) -> Self {
        Self {
            run_id: query.run_id,
            task_id: query.task_id,
            artifact_type: query.artifact_type,
            source_url: query.source_url,
            include_content: query.include_content,
            limit: query.limit,
            offset: query.offset,
        }
    }
}

#[derive(Serialize)]
pub struct ListArtifactsResponse {
    pub artifacts: Vec<TaskArtifact>,
}

#[derive(Serialize)]
pub struct ArtifactResponse {
    pub artifact: TaskArtifact,
}

#[derive(Serialize)]
pub struct ArtifactContentResponse {
    pub artifact_id: Uuid,
    pub name: String,
    pub artifact_type: String,
    pub content_text: Option<String>,
    pub content_json: Option<Value>,
}

#[derive(Serialize)]
pub struct SaveArtifactContextResponse {
    pub context: SavedContext,
}

pub async fn list_artifacts(
    State(state): State<AppState>,
    Query(query): Query<ListArtifactsQuery>,
) -> Result<Json<ListArtifactsResponse>, AppError> {
    let artifacts = state.op_tasks.list_artifacts(query.into()).await?;

    Ok(Json(ListArtifactsResponse { artifacts }))
}

pub async fn get_artifact_content(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ArtifactContentResponse>, AppError> {
    let artifact = state.op_tasks.get_artifact(id).await?;

    Ok(Json(ArtifactContentResponse {
        artifact_id: artifact.id,
        name: artifact.name,
        artifact_type: artifact.artifact_type,
        content_text: artifact.content_text,
        content_json: artifact.content_json,
    }))
}

pub async fn get_artifact(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ArtifactResponse>, AppError> {
    let artifact = state.op_tasks.get_artifact(id).await?;

    Ok(Json(ArtifactResponse { artifact }))
}

pub async fn save_artifact_context(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<PromoteArtifactToContextRequest>,
) -> Result<Json<SaveArtifactContextResponse>, AppError> {
    let context = state
        .op_tasks
        .promote_artifact_to_context(&state.context, id, req)
        .await?;

    Ok(Json(SaveArtifactContextResponse { context }))
}

fn default_enabled() -> bool {
    true
}

pub async fn create(
    State(state): State<AppState>,
    Json(req): Json<CreateOpTaskRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let task = state
        .op_tasks
        .create_task(
            req.task_type,
            req.name,
            req.description,
            req.input_json,
            req.enabled,
        )
        .await?;

    Ok(Json(serde_json::json!({ "task": task })))
}

pub async fn list(State(state): State<AppState>) -> Result<Json<serde_json::Value>, AppError> {
    let tasks = state.op_tasks.list_tasks().await?;
    Ok(Json(serde_json::json!({ "items": tasks })))
}

pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<TaskResponse>, AppError> {
    let task = state
        .op_tasks
        .get_op_task(id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("task {} not found", id)))?;

    Ok(Json(TaskResponse { task }))
}

pub async fn run(
    State(state): State<AppState>,
    Path(task_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let run = state.op_tasks.run_task(task_id).await?;
    Ok(Json(serde_json::json!({ "run": run })))
}

pub async fn get_run(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<RunResponse>, AppError> {
    let run = state.op_tasks.get_run(id).await?;

    Ok(Json(RunResponse { run }))
}

pub async fn list_runs(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ListRunsResponse>, AppError> {
    let runs = state.op_tasks.list_runs_for_task(id).await?;

    Ok(Json(ListRunsResponse { runs }))
}

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
    domains::employment::models::default_employment_profile_id,
    error::AppError,
    models::session::{TaskLink, TaskRequest},
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
            profile_id: None,
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
    list_artifacts_for_profile_id(state, default_employment_profile_id(), query).await
}

pub async fn list_artifacts_for_profile(
    State(state): State<AppState>,
    Path(profile_id): Path<Uuid>,
    Query(query): Query<ListArtifactsQuery>,
) -> Result<Json<ListArtifactsResponse>, AppError> {
    list_artifacts_for_profile_id(state, profile_id, query).await
}

async fn list_artifacts_for_profile_id(
    state: AppState,
    profile_id: Uuid,
    query: ListArtifactsQuery,
) -> Result<Json<ListArtifactsResponse>, AppError> {
    let mut search: ArtifactSearch = query.into();
    search.profile_id = Some(profile_id);
    let artifacts = state.op_tasks.list_artifacts(search).await?;

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

pub async fn get_artifact_content_for_profile(
    State(state): State<AppState>,
    Path((profile_id, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ArtifactContentResponse>, AppError> {
    let artifact = state.op_tasks.get_artifact(id).await?;
    if artifact.profile_id != profile_id {
        return Err(AppError::NotFound("Op Task artifact not found".to_string()));
    }

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

pub async fn get_artifact_for_profile(
    State(state): State<AppState>,
    Path((profile_id, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ArtifactResponse>, AppError> {
    let artifact = state.op_tasks.get_artifact(id).await?;
    if artifact.profile_id != profile_id {
        return Err(AppError::NotFound("Op Task artifact not found".to_string()));
    }

    Ok(Json(ArtifactResponse { artifact }))
}

pub async fn save_artifact_context(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<PromoteArtifactToContextRequest>,
) -> Result<Json<SaveArtifactContextResponse>, AppError> {
    save_artifact_context_for_profile_id(state, default_employment_profile_id(), id, req).await
}

pub async fn save_artifact_context_for_profile(
    State(state): State<AppState>,
    Path((profile_id, id)): Path<(Uuid, Uuid)>,
    Json(req): Json<PromoteArtifactToContextRequest>,
) -> Result<Json<SaveArtifactContextResponse>, AppError> {
    save_artifact_context_for_profile_id(state, profile_id, id, req).await
}

async fn save_artifact_context_for_profile_id(
    state: AppState,
    profile_id: Uuid,
    id: Uuid,
    req: PromoteArtifactToContextRequest,
) -> Result<Json<SaveArtifactContextResponse>, AppError> {
    let context = state
        .op_tasks
        .promote_artifact_to_context(&state.context, profile_id, id, req)
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
    create_for_profile_id(state, default_employment_profile_id(), req).await
}

pub async fn create_for_profile(
    State(state): State<AppState>,
    Path(profile_id): Path<Uuid>,
    Json(req): Json<CreateOpTaskRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    create_for_profile_id(state, profile_id, req).await
}

async fn create_for_profile_id(
    state: AppState,
    profile_id: Uuid,
    req: CreateOpTaskRequest,
) -> Result<Json<serde_json::Value>, AppError> {
    let task = state
        .op_tasks
        .create_task(
            profile_id,
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
    let tasks = state
        .op_tasks
        .list_tasks(default_employment_profile_id())
        .await?;
    Ok(Json(serde_json::json!({ "items": tasks })))
}

pub async fn list_for_profile(
    State(state): State<AppState>,
    Path(profile_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let tasks = state.op_tasks.list_tasks(profile_id).await?;
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
    run_task_with_request(state, task_id, None, "api").await
}

pub async fn run_for_profile(
    State(state): State<AppState>,
    Path((profile_id, task_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, AppError> {
    run_task_with_request(state, task_id, Some(profile_id), "api").await
}

async fn run_task_with_request(
    state: AppState,
    task_id: Uuid,
    required_profile_id: Option<Uuid>,
    source: &str,
) -> Result<Json<serde_json::Value>, AppError> {
    let task = state
        .op_tasks
        .get_op_task(task_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("task {} not found", task_id)))?;

    if required_profile_id.is_some_and(|profile_id| task.profile_id != profile_id) {
        return Err(AppError::NotFound(format!("task {} not found", task_id)));
    }

    let mut task_request = TaskRequest::new(
        task.profile_id,
        source.to_string(),
        format!("Run OpTask '{}' ({})", task.name, task.id),
    );
    task_request.intent = Some(task.task_type.clone());
    task_request.status = "running".to_string();
    task_request.op_task_id = Some(task.id);
    let task_request = state
        .session_memory
        .create_task_request(task_request)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;
    create_task_link(
        &state,
        task_request.id,
        "op_task",
        task.id,
        "manual_run_task",
    )
    .await?;

    let run_result = state.op_tasks.run_task(task_id).await;
    let run = match run_result {
        Ok(run) => run,
        Err(err) => {
            state
                .session_memory
                .update_task_request(task_request.id, "failed", Some(task.id), None, None)
                .await
                .map_err(|update_err| AppError::Internal(update_err.to_string()))?;
            return Err(err);
        }
    };

    let artifact_id = run.artifacts.first().map(|artifact| artifact.id);
    let status = if matches!(
        run.status,
        crate::op_tasks::models::OpTaskRunStatus::Succeeded
    ) {
        "succeeded"
    } else {
        "failed"
    };
    state
        .session_memory
        .update_task_request(
            task_request.id,
            status,
            Some(task.id),
            Some(run.id),
            artifact_id,
        )
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;
    create_task_link(
        &state,
        task_request.id,
        "op_task_run",
        run.id,
        "manual_run_result",
    )
    .await?;
    if let Some(artifact_id) = artifact_id {
        create_task_link(
            &state,
            task_request.id,
            "task_artifact",
            artifact_id,
            "primary_artifact",
        )
        .await?;
    }

    Ok(Json(serde_json::json!({
        "run": run,
        "task_request_id": task_request.id
    })))
}

async fn create_task_link(
    state: &AppState,
    task_request_id: Uuid,
    target_type: &str,
    target_id: Uuid,
    relationship: &str,
) -> Result<(), AppError> {
    state
        .session_memory
        .create_task_link(TaskLink::new(
            "task_request".to_string(),
            task_request_id,
            target_type.to_string(),
            target_id,
            relationship.to_string(),
        ))
        .await
        .map(|_| ())
        .map_err(|err| AppError::Internal(err.to_string()))
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

pub async fn list_runs_for_profile(
    State(state): State<AppState>,
    Path((profile_id, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ListRunsResponse>, AppError> {
    let task = state
        .op_tasks
        .get_op_task(id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("task {} not found", id)))?;

    if task.profile_id != profile_id {
        return Err(AppError::NotFound(format!("task {} not found", id)));
    }

    let runs = state.op_tasks.list_runs_for_task(id).await?;

    Ok(Json(ListRunsResponse { runs }))
}

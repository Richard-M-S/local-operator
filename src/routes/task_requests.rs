use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    app_state::AppState,
    domains::employment::models::default_employment_profile_id,
    error::AppError,
    models::session::TaskLink,
    op_tasks::models::{OpTaskRunStatus, TaskArtifact},
    services::operator_service::CreateTaskFromMessageResponse,
};

#[derive(Debug, Deserialize)]
pub struct CreateTaskRequest {
    pub message: String,
    pub profile_id: Option<Uuid>,
    pub source: Option<String>,
    #[serde(default)]
    pub confirm: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct RunTaskRequest {
    pub profile_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct TaskRunArtifactSummary {
    pub id: Uuid,
    pub artifact_type: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct TaskRunResponse {
    pub ok: bool,
    pub task_request_id: Uuid,
    pub task_id: Uuid,
    pub run_id: Uuid,
    pub status: OpTaskRunStatus,
    pub summary: Option<String>,
    pub artifacts: Vec<TaskRunArtifactSummary>,
    pub next_actions: Vec<String>,
    pub next_suggested_action: Value,
}

pub async fn create(
    State(state): State<AppState>,
    Json(req): Json<CreateTaskRequest>,
) -> Result<Json<CreateTaskFromMessageResponse>, AppError> {
    let profile_id = req.profile_id.unwrap_or_else(default_employment_profile_id);
    let source = req.source.as_deref().unwrap_or("api");
    let response = state
        .operator
        .create_task_from_message(
            &req.message,
            profile_id,
            source,
            req.confirm.unwrap_or(false),
        )
        .await?;

    Ok(Json(response))
}

pub async fn run(
    State(state): State<AppState>,
    Path(task_request_id): Path<Uuid>,
    body: Option<Json<RunTaskRequest>>,
) -> Result<Json<TaskRunResponse>, AppError> {
    let task_request = state
        .session_memory
        .get_task_request(task_request_id)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("task request {} not found", task_request_id)))?;

    if let Some(profile_id) = body.as_ref().and_then(|Json(req)| req.profile_id) {
        if task_request.profile_id != profile_id {
            return Err(AppError::NotFound(format!(
                "task request {} not found",
                task_request_id
            )));
        }
    }

    let task_id = task_request.op_task_id.ok_or_else(|| {
        AppError::BadRequest(format!(
            "task request {} is not linked to an OpTask",
            task_request_id
        ))
    })?;
    let task = state
        .op_tasks
        .get_op_task(task_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("task {} not found", task_id)))?;

    if task.profile_id != task_request.profile_id {
        return Err(AppError::NotFound(format!("task {} not found", task_id)));
    }

    state
        .session_memory
        .update_task_request(task_request.id, "running", Some(task.id), None, None)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;

    let run = match state.op_tasks.run_task(task.id).await {
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
    let request_status = if matches!(run.status, OpTaskRunStatus::Succeeded) {
        "succeeded"
    } else {
        "failed"
    };
    state
        .session_memory
        .update_task_request(
            task_request.id,
            request_status,
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
        "produced_run",
    )
    .await?;
    for artifact in &run.artifacts {
        create_task_link(
            &state,
            task_request.id,
            "task_artifact",
            artifact.id,
            "produced_artifact",
        )
        .await?;
    }

    let artifacts = run
        .artifacts
        .iter()
        .map(TaskRunArtifactSummary::from)
        .collect::<Vec<_>>();
    let next_actions = if artifacts.is_empty() {
        vec!["inspect_run".to_string()]
    } else {
        vec![
            "show_latest_artifacts".to_string(),
            "continue_from_artifact".to_string(),
        ]
    };
    let next_suggested_action = if let Some(first_artifact) = run.artifacts.first() {
        json!({
            "action": "show_artifact",
            "method": "GET",
            "path": format!("/api/employment/profiles/{}/op-task-artifacts/{}/content", task.profile_id, first_artifact.id),
            "artifact_id": first_artifact.id
        })
    } else {
        json!({
            "action": "inspect_run",
            "method": "GET",
            "path": format!("/api/op-task-runs/{}", run.id),
            "run_id": run.id
        })
    };

    Ok(Json(TaskRunResponse {
        ok: matches!(run.status, OpTaskRunStatus::Succeeded),
        task_request_id: task_request.id,
        task_id: task.id,
        run_id: run.id,
        status: run.status,
        summary: run.summary,
        artifacts,
        next_actions,
        next_suggested_action,
    }))
}

impl From<&TaskArtifact> for TaskRunArtifactSummary {
    fn from(artifact: &TaskArtifact) -> Self {
        Self {
            id: artifact.id,
            artifact_type: artifact.artifact_type.clone(),
            name: artifact.name.clone(),
        }
    }
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

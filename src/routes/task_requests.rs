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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::models::session::TaskRequest;
    use axum::extract::{Path, State};
    use sqlx::SqlitePool;

    fn default_profile_id() -> Uuid {
        default_employment_profile_id()
    }

    async fn test_state() -> AppState {
        let config = AppConfig::load().expect("load default test config");
        let db = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations")
            .run(&db)
            .await
            .expect("run migrations");
        AppState::new(config, db).await.expect("create app state")
    }

    #[tokio::test]
    async fn run_task_request_success_creates_run_and_links() {
        let state = test_state().await;
        let profile_id = default_profile_id();

        let task = state
            .op_tasks
            .create_task(
                profile_id,
                "system.status_report".to_string(),
                "Run system status report".to_string(),
                None,
                json!({}),
                true,
            )
            .await
            .expect("create task");

        let task_request = state
            .session_memory
            .create_task_request(TaskRequest {
                id: Uuid::new_v4(),
                profile_id,
                source: "unit_test".to_string(),
                user_request: "Run a status report".to_string(),
                intent: Some(task.task_type.clone()),
                status: "created".to_string(),
                op_task_id: Some(task.id),
                run_id: None,
                primary_artifact_id: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
            .await
            .expect("create task request");

        let response = run(State(state.clone()), Path(task_request.id), None)
            .await
            .expect("run task request")
            .0;

        assert!(response.ok);
        assert_eq!(response.status, OpTaskRunStatus::Succeeded);
        assert_eq!(response.task_request_id, task_request.id);
        assert_eq!(response.task_id, task.id);
        assert_eq!(response.artifacts.len(), 1);

        let (stored_status, stored_run_id): (String, Option<String>) =
            sqlx::query_as("SELECT status, run_id FROM task_requests WHERE id = ?1")
                .bind(task_request.id.to_string())
                .fetch_one(&state.db)
                .await
                .expect("fetch task request row");
        assert_eq!(stored_status, "succeeded");
        assert_eq!(
            stored_run_id.expect("run id").as_str(),
            response.run_id.to_string()
        );

        let produced_run_links: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM task_links WHERE source_type = 'task_request' AND source_id = ?1 AND target_type = 'op_task_run' AND relationship = 'produced_run'",
        )
        .bind(response.task_request_id.to_string())
        .fetch_one(&state.db)
        .await
        .expect("count produced run links");
        assert_eq!(produced_run_links, 1);

        let produced_artifact_links: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM task_links WHERE source_type = 'task_request' AND source_id = ?1 AND target_type = 'task_artifact' AND relationship = 'produced_artifact'",
        )
        .bind(response.task_request_id.to_string())
        .fetch_one(&state.db)
        .await
        .expect("count produced artifact links");
        assert_eq!(produced_artifact_links, 1);
    }

    #[tokio::test]
    async fn run_task_request_failure_updates_status_and_marks_failed() {
        let state = test_state().await;
        let profile_id = default_profile_id();

        let task = state
            .op_tasks
            .create_task(
                profile_id,
                "unsupported.task_type".to_string(),
                "Unsupported task".to_string(),
                None,
                json!({}),
                true,
            )
            .await
            .expect("create task");

        let task_request = state
            .session_memory
            .create_task_request(TaskRequest {
                id: Uuid::new_v4(),
                profile_id,
                source: "unit_test".to_string(),
                user_request: "Run unsupported task".to_string(),
                intent: Some(task.task_type.clone()),
                status: "created".to_string(),
                op_task_id: Some(task.id),
                run_id: None,
                primary_artifact_id: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
            .await
            .expect("create task request");

        let response = run(State(state.clone()), Path(task_request.id), None)
            .await
            .expect("run task request")
            .0;

        assert!(!response.ok);
        assert_eq!(response.status, OpTaskRunStatus::Failed);

        let (stored_status, stored_run_id): (String, Option<String>) =
            sqlx::query_as("SELECT status, run_id FROM task_requests WHERE id = ?1")
                .bind(task_request.id.to_string())
                .fetch_one(&state.db)
                .await
                .expect("fetch task request row");
        assert_eq!(stored_status, "failed");
        assert_eq!(stored_run_id.expect("run id"), response.run_id.to_string());

        assert!(response.artifacts.is_empty());

        let produced_run_links: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM task_links WHERE source_type = 'task_request' AND source_id = ?1 AND target_type = 'op_task_run' AND relationship = 'produced_run'",
        )
        .bind(response.task_request_id.to_string())
        .fetch_one(&state.db)
        .await
        .expect("count produced run links");
        assert_eq!(produced_run_links, 1);

        let produced_artifact_links: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM task_links WHERE source_type = 'task_request' AND source_id = ?1 AND target_type = 'task_artifact' AND relationship = 'produced_artifact'",
        )
        .bind(response.task_request_id.to_string())
        .fetch_one(&state.db)
        .await
        .expect("count produced artifact links");
        assert_eq!(produced_artifact_links, 0);
    }
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

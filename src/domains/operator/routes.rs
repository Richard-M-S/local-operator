use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    app_state::AppState,
    domains::employment::models::default_employment_profile_id,
    error::AppError,
    models::session::{TaskLink, TaskRequest},
    op_tasks::models::{ArtifactSearch, TaskArtifact},
    services::operator_service::CreateTaskFromMessageResponse,
};

use super::models::{
    OperatorTaskStateQuery, OperatorTaskStateSnapshot, OPERATOR_ARTIFACT_TYPES,
    OPERATOR_TASK_DIAGNOSTIC,
};

#[derive(Debug, Deserialize)]
pub struct ReviewFailedTaskRequest {
    pub run_id: Uuid,
    pub profile_id: Option<Uuid>,
    #[serde(default = "default_true")]
    pub include_task: bool,
    #[serde(default = "default_true")]
    pub include_artifacts: bool,
    #[serde(default = "default_true")]
    pub include_recent_audit: bool,
    #[serde(default)]
    pub include_repo_context: bool,
    #[serde(default)]
    pub escalate_if_needed: bool,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ReviewRecentTasksRequest {
    pub profile_id: Option<Uuid>,
    #[serde(default)]
    pub include_content: Option<bool>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct GeneratePatchPlanRequest {
    pub artifact_id: Uuid,
    pub profile_id: Option<Uuid>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ConvertRecommendationToTasksRequest {
    pub profile_id: Option<Uuid>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ShowOperatorDiagnosticsQuery {
    pub profile_id: Option<Uuid>,
    #[serde(default)]
    pub run_id: Option<Uuid>,
    #[serde(default)]
    pub include_content: Option<bool>,
    #[serde(default)]
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ShowOperatorDiagnosticsResponse {
    pub artifacts: Vec<TaskArtifact>,
    pub next_actions: Vec<String>,
}

pub async fn capabilities() -> Json<Value> {
    Json(json!({
        "domain": "operator",
        "service": "OperatorMetaService",
        "read_only_mvp_task": "operator.review_failed_task",
        "safety_boundaries": [
            {
                "level": 1,
                "name": "Diagnose only",
                "allowed": [
                    "read_task_state",
                    "read_artifacts",
                    "summarize_failure",
                    "suggest_fix"
                ],
                "requires_confirmation": false,
                "status": "active"
            },
            {
                "level": 2,
                "name": "Plan only",
                "allowed": [
                    "generate_patch_plan",
                    "generate_task_specs",
                    "generate_tool_specs",
                    "generate_openapi_update_plans"
                ],
                "requires_confirmation": false,
                "status": "active"
            },
            {
                "level": 3,
                "name": "Create draft tasks",
                "allowed": [
                    "create_draft_implementation_tasks",
                    "create_draft_docs_tasks",
                    "create_draft_test_tasks"
                ],
                "requires_confirmation": true,
                "status": "active"
            },
            {
                "level": 4,
                "name": "Modify repo/code/config",
                "allowed": [
                    "create_branch",
                    "write_patch",
                    "run_tests",
                    "open_pr"
                ],
                "requires_confirmation": true,
                "status": "blocked_for_now"
            },
            {
                "level": 5,
                "name": "Execute operational changes",
                "allowed": [
                    "restart_containers",
                    "change_home_assistant_automations",
                    "alter_secrets_or_config"
                ],
                "requires_confirmation": true,
                "status": "blocked_for_now"
            }
        ],
        "max_active_level": 3,
        "repo_code_config_modification_enabled": false,
        "operational_change_execution_enabled": false,
        "artifact_types": OPERATOR_ARTIFACT_TYPES,
        "implemented_artifact_types": [
            "operator_task_diagnostic",
            "operator_patch_plan",
            "operator_implementation_task_set"
        ],
        "risky_operations_executed_directly": false
    }))
}

pub async fn inspect_state(
    State(state): State<AppState>,
    Query(query): Query<OperatorTaskStateQuery>,
) -> Result<Json<OperatorTaskStateSnapshot>, AppError> {
    let snapshot = state
        .operator_meta
        .inspect_task_state(query)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(Json(snapshot))
}

pub async fn review_failed_task(
    State(state): State<AppState>,
    Json(req): Json<ReviewFailedTaskRequest>,
) -> Result<Json<CreateTaskFromMessageResponse>, AppError> {
    let profile_id = req.profile_id.unwrap_or_else(default_employment_profile_id);
    let source = normalized_source(req.source.as_deref());
    let response = create_operator_task_request(
        &state,
        profile_id,
        source,
        "Review Failed Task",
        "operator.review_failed_task",
        Some("Created from operator-meta reviewFailedTask endpoint.".to_string()),
        json!({
            "run_id": req.run_id,
            "include_task": req.include_task,
            "include_artifacts": req.include_artifacts,
            "include_recent_audit": req.include_recent_audit,
            "include_repo_context": req.include_repo_context,
            "escalate_if_needed": req.escalate_if_needed,
            "priority": "normal",
            "model_purpose": "failure_classification",
            "source": source
        }),
        format!("Review failed task run {} and recommend fixes.", req.run_id),
        "operator.review_failed_task",
    )
    .await?;

    Ok(Json(response))
}

pub async fn review_recent_tasks(
    State(state): State<AppState>,
    body: Option<Json<ReviewRecentTasksRequest>>,
) -> Result<Json<OperatorTaskStateSnapshot>, AppError> {
    let req = body
        .map(|Json(req)| req)
        .unwrap_or(ReviewRecentTasksRequest {
            profile_id: None,
            include_content: None,
            limit: None,
            offset: None,
        });
    let snapshot = state
        .operator_meta
        .inspect_task_state(OperatorTaskStateQuery {
            profile_id: req.profile_id,
            task_id: None,
            run_id: None,
            artifact_id: None,
            artifact_type: None,
            source_url: None,
            include_content: req.include_content,
            limit: req.limit.or(Some(25)),
            offset: req.offset.or(Some(0)),
        })
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(Json(snapshot))
}

pub async fn generate_patch_plan(
    State(state): State<AppState>,
    Json(req): Json<GeneratePatchPlanRequest>,
) -> Result<Json<CreateTaskFromMessageResponse>, AppError> {
    let profile_id = req.profile_id.unwrap_or_else(default_employment_profile_id);
    let source = normalized_source(req.source.as_deref());
    let mut input_json = json!({
        "artifact_id": req.artifact_id,
        "priority": "normal",
        "model_purpose": "patch_plan",
        "source": source
    });
    if let Some(title) = clean_optional(req.title.as_deref()) {
        input_json["title"] = Value::String(title);
    }

    let response = create_operator_task_request(
        &state,
        profile_id,
        source,
        "Generate Patch Plan",
        "operator.generate_patch_plan",
        Some("Created from operator-meta generatePatchPlan endpoint.".to_string()),
        input_json,
        format!(
            "Generate an operator patch plan from diagnostic artifact {}.",
            req.artifact_id
        ),
        "operator.generate_patch_plan",
    )
    .await?;

    Ok(Json(response))
}

pub async fn convert_recommendation_to_tasks(
    State(state): State<AppState>,
    Path(artifact_id): Path<Uuid>,
    body: Option<Json<ConvertRecommendationToTasksRequest>>,
) -> Result<Json<CreateTaskFromMessageResponse>, AppError> {
    let req = body
        .map(|Json(req)| req)
        .unwrap_or(ConvertRecommendationToTasksRequest {
            profile_id: None,
            source: None,
        });
    let profile_id = req.profile_id.unwrap_or_else(default_employment_profile_id);
    let source = normalized_source(req.source.as_deref());
    let response = create_operator_task_request(
        &state,
        profile_id,
        source,
        "Create Implementation Task Set",
        "operator.convert_recommendation_to_tasks",
        Some("Created from operator-meta convertRecommendationToTasks endpoint.".to_string()),
        json!({
            "artifact_id": artifact_id,
            "priority": "normal",
            "model_purpose": "implementation_task_planning",
            "source": source
        }),
        format!(
            "Convert operator recommendation artifact {} into implementation tasks.",
            artifact_id
        ),
        "operator.convert_recommendation_to_tasks",
    )
    .await?;

    Ok(Json(response))
}

pub async fn show_operator_diagnostics(
    State(state): State<AppState>,
    Query(query): Query<ShowOperatorDiagnosticsQuery>,
) -> Result<Json<ShowOperatorDiagnosticsResponse>, AppError> {
    let artifacts = state
        .op_tasks
        .list_artifacts(ArtifactSearch {
            profile_id: query.profile_id,
            run_id: query.run_id,
            task_id: None,
            artifact_type: Some(OPERATOR_TASK_DIAGNOSTIC.to_string()),
            source_url: None,
            include_content: query.include_content,
            limit: Some(query.limit.unwrap_or(10).clamp(1, 50)),
            offset: Some(0),
        })
        .await?;

    let next_actions = if artifacts.is_empty() {
        vec![
            "review_failed_task".to_string(),
            "review_recent_tasks".to_string(),
        ]
    } else {
        vec![
            "generate_patch_plan".to_string(),
            "escalate_to_chatgpt".to_string(),
            "show_latest_artifacts".to_string(),
        ]
    };

    Ok(Json(ShowOperatorDiagnosticsResponse {
        artifacts,
        next_actions,
    }))
}

async fn create_operator_task_request(
    state: &AppState,
    profile_id: Uuid,
    source: &str,
    name: &str,
    task_type: &str,
    description: Option<String>,
    input_json: Value,
    user_request: String,
    intent: &str,
) -> Result<CreateTaskFromMessageResponse, AppError> {
    let task = state
        .op_tasks
        .create_task(
            profile_id,
            task_type.to_string(),
            name.to_string(),
            description,
            input_json,
            true,
        )
        .await?;

    let mut task_request = TaskRequest::new(profile_id, source.to_string(), user_request);
    task_request.intent = Some(intent.to_string());
    task_request.status = "created".to_string();
    task_request.op_task_id = Some(task.id);
    let task_request = state
        .session_memory
        .create_task_request(task_request)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;

    create_task_link(state, task_request.id, "op_task", task.id, "created_task").await?;

    let suggested_next_action = json!({
        "action": "run_task",
        "method": "POST",
        "path": format!("/api/task-requests/{}/run", task_request.id),
        "description": "Run the created task when ready."
    });

    Ok(CreateTaskFromMessageResponse {
        task_request,
        task,
        intent: intent.to_string(),
        suggested_next_action,
    })
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

fn normalized_source(source: Option<&str>) -> &str {
    source
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("operator_meta")
}

fn clean_optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn default_true() -> bool {
    true
}

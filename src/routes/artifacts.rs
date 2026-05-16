use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    app_state::AppState,
    domains::employment::models::default_employment_profile_id,
    error::AppError,
    models::session::{TaskLink, TaskRequest},
    op_tasks::models::{ArtifactSearch, OpTask, OpTaskRunStatus, TaskArtifact},
};

#[derive(Debug, Deserialize)]
pub struct LatestArtifactsQuery {
    pub profile_id: Option<Uuid>,
    pub task_request_id: Option<Uuid>,
    pub task_id: Option<Uuid>,
    pub run_id: Option<Uuid>,
    pub artifact_type: Option<String>,
    pub limit: Option<i64>,
    pub include_content: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct LatestArtifactSummary {
    pub id: Uuid,
    pub profile_id: Uuid,
    pub run_id: Uuid,
    pub work_item_id: Option<Uuid>,
    pub artifact_type: String,
    pub name: String,
    pub location: Option<String>,
    pub created_at: DateTime<Utc>,
    pub metadata: Option<Value>,
    pub content_text: Option<String>,
    pub content_json: Option<Value>,
    pub allowed_continuations: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct LatestArtifactsResponse {
    pub artifacts: Vec<LatestArtifactSummary>,
    pub filters: LatestArtifactFilters,
    pub next_actions: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct LatestArtifactFilters {
    pub profile_id: Option<Uuid>,
    pub task_request_id: Option<Uuid>,
    pub task_id: Option<Uuid>,
    pub run_id: Option<Uuid>,
    pub artifact_type: Option<String>,
    pub limit: i64,
    pub include_content: bool,
}

#[derive(Debug, Deserialize)]
pub struct ContinueArtifactRequest {
    pub message: String,
    pub profile_id: Option<Uuid>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub confirm: bool,
    #[serde(default)]
    pub create_tasks: bool,
}

#[derive(Debug, Serialize)]
pub struct ContinueArtifactResponse {
    pub ok: bool,
    pub intent: String,
    pub source_artifact_type: String,
    pub allowed_continuations: Vec<String>,
    pub task_request: TaskRequest,
    pub task: OpTask,
    pub run: OpTaskRunSummary,
    pub artifacts: Vec<TaskRunArtifactSummary>,
    pub recommended_actions: Vec<RecommendedEscalationAction>,
    pub created_tasks: Vec<OpTask>,
    pub requires_confirmation: bool,
    pub next_actions: Vec<String>,
    pub next_suggested_action: Value,
}

#[derive(Clone, Debug, Serialize)]
pub struct RecommendedEscalationAction {
    pub title: String,
    pub detail: Option<String>,
    pub suggested_task_type: String,
    pub input_json: Value,
}

#[derive(Debug, Serialize)]
pub struct OpTaskRunSummary {
    pub id: Uuid,
    pub status: OpTaskRunStatus,
    pub summary: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TaskRunArtifactSummary {
    pub id: Uuid,
    pub artifact_type: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateChatGptEscalationRequest {
    pub run_id: Uuid,
    pub profile_id: Option<Uuid>,
    #[serde(default)]
    pub confirm: bool,
    #[serde(default)]
    pub work_item_id: Option<Uuid>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub metadata: Option<Value>,
    #[serde(default)]
    pub content_text: Option<String>,
    pub content_json: Value,
}

#[derive(Debug, Deserialize)]
pub struct SaveChatGptEscalationResponse {
    pub profile_id: Option<Uuid>,
    #[serde(default)]
    pub work_item_id: Option<Uuid>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub metadata: Option<Value>,
    #[serde(default)]
    pub content_text: Option<String>,
    #[serde(default)]
    pub response_text: Option<String>,
    #[serde(default)]
    pub content_json: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct ChatGptEscalationArtifactResponse {
    pub artifact: TaskArtifact,
    pub linked_request_artifact_id: Option<Uuid>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum EscalationArtifactPrivacyClass {
    TechnicalOnly,
    Personal,
    Employment,
}

#[derive(Clone, Debug, Serialize)]
struct EscalationArtifactPolicyDecision {
    allowed: bool,
    requires_confirmation: bool,
    privacy_classification: EscalationArtifactPrivacyClass,
    reason: String,
}

pub async fn latest(
    State(state): State<AppState>,
    Query(query): Query<LatestArtifactsQuery>,
) -> Result<Json<LatestArtifactsResponse>, AppError> {
    let include_content = query.include_content.unwrap_or(false);
    let limit = query.limit.unwrap_or(10).clamp(1, 50);
    let mut search = ArtifactSearch {
        profile_id: query.profile_id,
        run_id: query.run_id,
        task_id: query.task_id,
        artifact_type: query.artifact_type.clone(),
        source_url: None,
        include_content: Some(include_content),
        limit: Some(limit),
        offset: Some(0),
    };

    if let Some(task_request_id) = query.task_request_id {
        let task_request = state
            .session_memory
            .get_task_request(task_request_id)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?
            .ok_or_else(|| {
                AppError::NotFound(format!("task request {} not found", task_request_id))
            })?;

        if query
            .profile_id
            .is_some_and(|profile_id| profile_id != task_request.profile_id)
        {
            return Err(AppError::NotFound(format!(
                "task request {} not found",
                task_request_id
            )));
        }

        if query
            .task_id
            .is_some_and(|task_id| Some(task_id) != task_request.op_task_id)
        {
            return Err(AppError::NotFound(format!(
                "task request {} not found",
                task_request_id
            )));
        }

        if query
            .run_id
            .is_some_and(|run_id| Some(run_id) != task_request.run_id)
        {
            return Err(AppError::NotFound(format!(
                "task request {} not found",
                task_request_id
            )));
        }

        search.profile_id = Some(task_request.profile_id);
        if search.run_id.is_none() {
            search.run_id = task_request.run_id;
        }
        if search.run_id.is_none() && search.task_id.is_none() {
            search.task_id = task_request.op_task_id;
        }
    }

    let filters = LatestArtifactFilters {
        profile_id: search.profile_id,
        task_request_id: query.task_request_id,
        task_id: search.task_id,
        run_id: search.run_id,
        artifact_type: search.artifact_type.clone(),
        limit,
        include_content,
    };
    let artifacts = state.op_tasks.list_artifacts(search).await?;
    let summaries = artifacts
        .into_iter()
        .map(|artifact| LatestArtifactSummary::from_artifact(artifact, include_content))
        .collect::<Vec<_>>();

    let next_actions = if summaries.is_empty() {
        vec![
            "run_task".to_string(),
            "broaden_artifact_filters".to_string(),
        ]
    } else if include_content {
        vec![
            "answer_from_artifacts".to_string(),
            "continue_from_artifact".to_string(),
        ]
    } else {
        vec![
            "fetch_artifact_content".to_string(),
            "continue_from_artifact".to_string(),
        ]
    };

    Ok(Json(LatestArtifactsResponse {
        artifacts: summaries,
        filters,
        next_actions,
    }))
}

pub async fn continue_from_artifact(
    State(state): State<AppState>,
    Path(artifact_id): Path<Uuid>,
    Json(req): Json<ContinueArtifactRequest>,
) -> Result<Json<ContinueArtifactResponse>, AppError> {
    let message = req.message.trim();
    if message.is_empty() {
        return Err(AppError::BadRequest("message cannot be empty".to_string()));
    }

    let profile_id = req.profile_id.unwrap_or_else(default_employment_profile_id);
    let source = req
        .source
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("artifact_continue");

    let artifact = state.op_tasks.get_artifact(artifact_id).await?;
    if artifact.profile_id != profile_id {
        return Err(AppError::NotFound(format!(
            "artifact {} not found",
            artifact_id
        )));
    }

    let is_escalation_response = artifact.artifact_type == "chatgpt_escalation_response";
    let recommended_actions = if is_escalation_response {
        extract_escalation_recommended_actions(&artifact, source)
    } else {
        vec![]
    };
    let follow_up_creation_approved = req.confirm || req.create_tasks;

    let continuation = build_artifact_continuation(&artifact, message, source)?;
    let task = state
        .op_tasks
        .create_task(
            profile_id,
            continuation.task_type.clone(),
            continuation.name,
            continuation.description,
            continuation.input_json,
            true,
        )
        .await?;

    let mut task_request = TaskRequest::new(profile_id, source.to_string(), message.to_string());
    task_request.intent = Some(continuation.intent.clone());
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
        "task_artifact",
        artifact.id,
        "continued_from",
    )
    .await?;
    create_task_link(&state, task_request.id, "op_task", task.id, "created_task").await?;

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
    let mut task_request = task_request;
    task_request.status = request_status.to_string();
    task_request.run_id = Some(run.id);
    task_request.primary_artifact_id = artifact_id;

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

    let created_tasks = if is_escalation_response && follow_up_creation_approved {
        create_escalation_follow_up_tasks(&state, profile_id, &artifact, &recommended_actions)
            .await?
    } else {
        vec![]
    };

    let artifacts = run
        .artifacts
        .iter()
        .map(TaskRunArtifactSummary::from)
        .collect::<Vec<_>>();
    let next_suggested_action = if let Some(first_artifact) = run.artifacts.first() {
        json_path_action(
            "show_artifact",
            "GET",
            format!(
                "/api/employment/profiles/{}/op-task-artifacts/{}/content",
                profile_id, first_artifact.id
            ),
            Some(first_artifact.id),
            None,
        )
    } else {
        json_path_action(
            "inspect_run",
            "GET",
            format!("/api/op-task-runs/{}", run.id),
            None,
            Some(run.id),
        )
    };
    let run_summary = OpTaskRunSummary {
        id: run.id,
        status: run.status,
        summary: run.summary.clone(),
    };
    let requires_confirmation =
        is_escalation_response && !recommended_actions.is_empty() && !follow_up_creation_approved;
    let next_actions = if is_escalation_response && requires_confirmation {
        vec![
            "approve_follow_up_tasks".to_string(),
            "show_latest_artifacts".to_string(),
            "continue_from_artifact".to_string(),
        ]
    } else if is_escalation_response && !created_tasks.is_empty() {
        vec![
            "review_draft_tasks".to_string(),
            "show_latest_artifacts".to_string(),
            "continue_from_artifact".to_string(),
        ]
    } else {
        vec![
            "show_latest_artifacts".to_string(),
            "continue_from_artifact".to_string(),
        ]
    };

    Ok(Json(ContinueArtifactResponse {
        ok: matches!(run.status, OpTaskRunStatus::Succeeded),
        intent: continuation.intent,
        source_artifact_type: artifact.artifact_type.clone(),
        allowed_continuations: allowed_continuations_for_artifact_type(&artifact.artifact_type),
        task_request,
        task,
        run: run_summary,
        artifacts,
        recommended_actions,
        created_tasks,
        requires_confirmation,
        next_actions,
        next_suggested_action,
    }))
}

pub async fn create_chatgpt_escalation_request(
    State(state): State<AppState>,
    Json(req): Json<CreateChatGptEscalationRequest>,
) -> Result<Json<ChatGptEscalationArtifactResponse>, AppError> {
    ensure_structured_json(&req.content_json, "content_json")?;
    let policy_decision = classify_escalation_artifact_policy(
        req.content_text.as_deref(),
        &req.content_json,
        req.confirm,
    )?;
    let run = state.op_tasks.get_run(req.run_id).await?;
    if req
        .profile_id
        .is_some_and(|profile_id| profile_id != run.profile_id)
    {
        return Err(AppError::NotFound(format!(
            "Op Task run {} not found",
            req.run_id
        )));
    }

    let artifact = TaskArtifact {
        id: Uuid::new_v4(),
        profile_id: run.profile_id,
        run_id: run.id,
        work_item_id: req.work_item_id,
        name: clean_optional(req.name.as_deref())
            .unwrap_or_else(|| "ChatGPT escalation request".to_string()),
        artifact_type: "chatgpt_escalation_request".to_string(),
        location: None,
        created_at: Utc::now(),
        metadata: Some(merge_metadata(
            req.metadata,
            json!({
                "escalation_provider": "chatgpt",
                "direction": "request",
                "run_id": run.id,
                "task_id": run.task_id,
                "privacy_classification": policy_decision.privacy_classification,
                "policy_decision": policy_decision,
            }),
        )),
        content_text: req.content_text,
        content_json: Some(req.content_json),
    };

    let artifact = state.op_tasks.save_artifact(artifact).await?;
    Ok(Json(ChatGptEscalationArtifactResponse {
        artifact,
        linked_request_artifact_id: None,
    }))
}

pub async fn save_chatgpt_escalation_response(
    State(state): State<AppState>,
    Path(request_artifact_id): Path<Uuid>,
    Json(req): Json<SaveChatGptEscalationResponse>,
) -> Result<Json<ChatGptEscalationArtifactResponse>, AppError> {
    let response_text = clean_optional(req.response_text.as_deref());
    if req.content_json.is_none() && response_text.is_none() {
        return Err(AppError::BadRequest(
            "either content_json or response_text is required".to_string(),
        ));
    }
    let content_text = clean_optional(req.content_text.as_deref()).or(response_text.clone());
    let content_json = req.content_json.unwrap_or_else(|| {
        json!({
            "response_text": response_text,
        })
    });
    ensure_structured_json(&content_json, "content_json")?;
    ensure_no_secret_content(content_text.as_deref(), &content_json)?;
    let request_artifact = state.op_tasks.get_artifact(request_artifact_id).await?;
    if request_artifact.artifact_type != "chatgpt_escalation_request" {
        return Err(AppError::BadRequest(
            "request_artifact_id must reference a chatgpt_escalation_request artifact".to_string(),
        ));
    }
    if req
        .profile_id
        .is_some_and(|profile_id| profile_id != request_artifact.profile_id)
    {
        return Err(AppError::NotFound(format!(
            "artifact {} not found",
            request_artifact_id
        )));
    }

    let artifact = TaskArtifact {
        id: Uuid::new_v4(),
        profile_id: request_artifact.profile_id,
        run_id: request_artifact.run_id,
        work_item_id: req.work_item_id.or(request_artifact.work_item_id),
        name: clean_optional(req.name.as_deref())
            .unwrap_or_else(|| "ChatGPT escalation response".to_string()),
        artifact_type: "chatgpt_escalation_response".to_string(),
        location: None,
        created_at: Utc::now(),
        metadata: Some(merge_metadata(
            req.metadata,
            json!({
                "escalation_provider": "chatgpt",
                "direction": "response",
                "request_artifact_id": request_artifact.id,
                "run_id": request_artifact.run_id,
            }),
        )),
        content_text,
        content_json: Some(content_json),
    };

    let artifact = state.op_tasks.save_artifact(artifact).await?;
    create_artifact_link(
        &state,
        artifact.id,
        request_artifact.id,
        "responds_to_escalation_request",
    )
    .await?;

    Ok(Json(ChatGptEscalationArtifactResponse {
        artifact,
        linked_request_artifact_id: Some(request_artifact.id),
    }))
}

impl LatestArtifactSummary {
    fn from_artifact(artifact: TaskArtifact, include_content: bool) -> Self {
        let allowed_continuations =
            allowed_continuations_for_artifact_type(artifact.artifact_type.as_str());
        Self {
            id: artifact.id,
            profile_id: artifact.profile_id,
            run_id: artifact.run_id,
            work_item_id: artifact.work_item_id,
            artifact_type: artifact.artifact_type,
            name: artifact.name,
            location: artifact.location,
            created_at: artifact.created_at,
            metadata: artifact.metadata,
            content_text: include_content.then_some(artifact.content_text).flatten(),
            content_json: include_content.then_some(artifact.content_json).flatten(),
            allowed_continuations,
        }
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

struct ArtifactContinuationPlan {
    intent: String,
    task_type: String,
    name: String,
    description: Option<String>,
    input_json: Value,
}

fn build_artifact_continuation(
    artifact: &TaskArtifact,
    message: &str,
    source: &str,
) -> Result<ArtifactContinuationPlan, AppError> {
    match artifact.artifact_type.as_str() {
        "search_result_set" => continue_from_search_result_set(artifact, message, source),
        "readable_web_page" if is_employment_continuation(message) => {
            continue_from_readable_web_page(artifact, message, source)
        }
        "extracted_opportunity_candidates" => {
            continue_from_candidates_artifact(artifact, message, source)
        }
        "scored_opportunity_matches" if wants_create_opportunities(message) => {
            continue_from_scored_matches_artifact(artifact, message, source)
        }
        "operator_task_diagnostic" => {
            continue_from_operator_task_diagnostic(artifact, message, source)
        }
        "operator_patch_plan" => continue_from_operator_patch_plan(artifact, message, source),
        "chatgpt_escalation_response" => {
            continue_from_chatgpt_escalation_response(artifact, message, source)
        }
        _ => Ok(generic_artifact_summary_plan(artifact, message, source)),
    }
}

fn allowed_continuations_for_artifact_type(artifact_type: &str) -> Vec<String> {
    let continuations: &[&str] = match artifact_type {
        "operator_task_diagnostic" => &[
            "generate_patch_plan",
            "escalate_to_chatgpt",
            "create_follow_up_tasks",
        ],
        "operator_patch_plan" => &[
            "create_implementation_tasks",
            "create_docs_update_task",
            "create_test_plan",
        ],
        "operator_tool_spec" => &["create_tool_implementation_plan"],
        "operator_openapi_review" => &["create_openapi_update_task"],
        "chatgpt_escalation_response" => &[
            "generate_patch_plan",
            "create_implementation_task_set",
            "summarize_recommendation",
            "convert_recommendation_to_tasks",
        ],
        "operator_implementation_task_set" => &["approve_create_tasks", "continue_from_task_set"],
        "chatgpt_escalation_request" => &["save_chatgpt_response"],
        "operator_gap_analysis" => &["design_task_type", "design_tool", "generate_patch_plan"],
        "operator_task_type_spec" => &["create_implementation_tasks", "review_openapi_surface"],
        "operator_test_plan" => &["create_test_task", "summarize_test_plan"],
        _ => &["summarize_artifact"],
    };

    continuations
        .iter()
        .map(|continuation| (*continuation).to_string())
        .collect()
}

fn continue_from_operator_task_diagnostic(
    artifact: &TaskArtifact,
    message: &str,
    source: &str,
) -> Result<ArtifactContinuationPlan, AppError> {
    let normalized = message.to_lowercase();
    if contains_any(&normalized, &["escalate", "chatgpt", "chat gpt"]) {
        return Ok(operator_continuation_plan(
            "artifact.continue.operator_task_diagnostic.escalate_to_chatgpt",
            "Escalate diagnostic to ChatGPT",
            "operator.escalate_to_chatgpt",
            "escalation_packet",
            artifact,
            message,
            source,
            json!({
                "mode": "manual",
                "confirm": false,
                "desired_output": "Review this operator diagnostic and return structured recommendations and next steps.",
                "context_json": artifact.content_json,
                "context_text": artifact.content_text,
            }),
        ));
    }

    Ok(operator_continuation_plan(
        "artifact.continue.operator_task_diagnostic.generate_patch_plan",
        "Generate patch plan",
        "operator.generate_patch_plan",
        "patch_plan",
        artifact,
        message,
        source,
        json!({
            "artifact_id": artifact.id,
        }),
    ))
}

fn continue_from_operator_patch_plan(
    artifact: &TaskArtifact,
    message: &str,
    source: &str,
) -> Result<ArtifactContinuationPlan, AppError> {
    let normalized = message.to_lowercase();
    if contains_any(
        &normalized,
        &[
            "implementation",
            "implement",
            "task set",
            "tasks",
            "create tasks",
        ],
    ) {
        return Ok(operator_continuation_plan(
            "artifact.continue.operator_patch_plan.create_implementation_tasks",
            "Create implementation task set",
            "operator.convert_recommendation_to_tasks",
            "implementation_task_planning",
            artifact,
            message,
            source,
            json!({
                "artifact_id": artifact.id,
            }),
        ));
    }

    let intent = if contains_any(&normalized, &["doc", "readme", "openapi"]) {
        "artifact.continue.operator_patch_plan.create_docs_update_task"
    } else if contains_any(&normalized, &["test", "validation", "verify"]) {
        "artifact.continue.operator_patch_plan.create_test_plan"
    } else {
        "artifact.continue.operator_patch_plan.summarize"
    };
    Ok(operator_continuation_plan(
        intent,
        "Continue from patch plan",
        "artifact.summarize",
        "artifact_continuation",
        artifact,
        message,
        source,
        json!({}),
    ))
}

fn continue_from_chatgpt_escalation_response(
    artifact: &TaskArtifact,
    message: &str,
    source: &str,
) -> Result<ArtifactContinuationPlan, AppError> {
    let normalized = message.to_lowercase();
    let intent = if contains_any(&normalized, &["implementation", "task", "convert"]) {
        "artifact.continue.chatgpt_escalation_response.convert_recommendation_to_tasks"
    } else if contains_any(&normalized, &["patch plan", "plan"]) {
        "artifact.continue.chatgpt_escalation_response.generate_patch_plan"
    } else {
        "artifact.continue.chatgpt_escalation_response.summarize_recommendation"
    };

    Ok(operator_continuation_plan(
        intent,
        "Continue from ChatGPT escalation response",
        "artifact.summarize",
        "escalation_follow_up",
        artifact,
        message,
        source,
        json!({}),
    ))
}

fn operator_continuation_plan(
    intent: &str,
    name: &str,
    task_type: &str,
    model_purpose: &str,
    artifact: &TaskArtifact,
    message: &str,
    source: &str,
    mut input_json: Value,
) -> ArtifactContinuationPlan {
    let input = input_json.as_object_mut().expect("object literal");
    input.insert(
        "user_request".to_string(),
        Value::String(message.to_string()),
    );
    input.insert(
        "source_artifact_id".to_string(),
        Value::String(artifact.id.to_string()),
    );
    input.insert(
        "source_artifact_type".to_string(),
        Value::String(artifact.artifact_type.clone()),
    );
    input.insert("priority".to_string(), Value::String("normal".to_string()));
    input.insert(
        "model_purpose".to_string(),
        Value::String(model_purpose.to_string()),
    );
    input.insert("source".to_string(), Value::String(source.to_string()));

    ArtifactContinuationPlan {
        intent: intent.to_string(),
        task_type: task_type.to_string(),
        name: name.to_string(),
        description: Some(format!(
            "Created from artifact continuation for {}.",
            artifact.id
        )),
        input_json,
    }
}

fn continue_from_search_result_set(
    artifact: &TaskArtifact,
    message: &str,
    source: &str,
) -> Result<ArtifactContinuationPlan, AppError> {
    let content = artifact_content_json(artifact)?;
    let results = content
        .get("results")
        .cloned()
        .ok_or_else(|| AppError::BadRequest("search_result_set has no results".to_string()))?;
    let limit = requested_limit(message).unwrap_or(10);

    Ok(employment_continuation_plan(
        "artifact.continue.search_result_set",
        "Continue from search results",
        artifact,
        message,
        source,
        json!({
            "seed_search_results": results,
            "source_query": content.get("query").cloned().unwrap_or(Value::Null),
            "limit": limit,
            "create_opportunities": wants_create_opportunities(message),
            "min_score": requested_min_score(message).unwrap_or(0),
        }),
    ))
}

fn continue_from_readable_web_page(
    artifact: &TaskArtifact,
    message: &str,
    source: &str,
) -> Result<ArtifactContinuationPlan, AppError> {
    let content = artifact_content_json(artifact).unwrap_or_else(|_| json!({}));
    let source_url = artifact
        .location
        .clone()
        .or_else(|| {
            content
                .get("source_url")
                .and_then(|value| value.as_str())
                .map(str::to_string)
        })
        .ok_or_else(|| AppError::BadRequest("readable_web_page missing source URL".to_string()))?;
    let text = content
        .get("cleaned_text")
        .or_else(|| content.get("raw_text"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .or_else(|| artifact.content_text.clone())
        .ok_or_else(|| AppError::BadRequest("readable_web_page has no content".to_string()))?;
    let title = content
        .get("title")
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| artifact.name.clone());

    Ok(employment_continuation_plan(
        "artifact.continue.readable_web_page",
        "Continue from readable web page",
        artifact,
        message,
        source,
        json!({
            "seed_readable_pages": [{
                "artifact_id": artifact.id,
                "source_url": source_url,
                "title": title,
                "text": text,
            }],
            "limit": requested_limit(message).unwrap_or(1),
            "create_opportunities": wants_create_opportunities(message),
            "min_score": requested_min_score(message).unwrap_or(0),
        }),
    ))
}

fn continue_from_candidates_artifact(
    artifact: &TaskArtifact,
    message: &str,
    source: &str,
) -> Result<ArtifactContinuationPlan, AppError> {
    let content = artifact_content_json(artifact)?;
    let candidates = content.get("candidates").cloned().ok_or_else(|| {
        AppError::BadRequest("extracted_opportunity_candidates has no candidates".to_string())
    })?;

    Ok(employment_continuation_plan(
        "artifact.continue.extracted_opportunity_candidates",
        "Continue from opportunity candidates",
        artifact,
        message,
        source,
        json!({
            "seed_candidates": candidates,
            "limit": requested_limit(message).unwrap_or(10),
            "create_opportunities": wants_create_opportunities(message),
            "min_score": requested_min_score(message).unwrap_or(0),
        }),
    ))
}

fn continue_from_scored_matches_artifact(
    artifact: &TaskArtifact,
    message: &str,
    source: &str,
) -> Result<ArtifactContinuationPlan, AppError> {
    let content = artifact_content_json(artifact)?;
    let matches = content.get("matches").cloned().ok_or_else(|| {
        AppError::BadRequest("scored_opportunity_matches has no matches".to_string())
    })?;

    Ok(employment_continuation_plan(
        "artifact.continue.scored_opportunity_matches",
        "Continue from scored opportunity matches",
        artifact,
        message,
        source,
        json!({
            "seed_scored_matches": matches,
            "limit": requested_limit(message).unwrap_or(10),
            "create_opportunities": wants_create_opportunities(message),
            "min_score": requested_min_score(message).unwrap_or(0),
        }),
    ))
}

fn employment_continuation_plan(
    intent: &str,
    name: &str,
    artifact: &TaskArtifact,
    message: &str,
    source: &str,
    mut input_json: Value,
) -> ArtifactContinuationPlan {
    let input = input_json.as_object_mut().expect("object literal");
    input.insert(
        "user_request".to_string(),
        Value::String(message.to_string()),
    );
    input.insert(
        "source_artifact_id".to_string(),
        Value::String(artifact.id.to_string()),
    );
    input.insert(
        "source_artifact_type".to_string(),
        Value::String(artifact.artifact_type.clone()),
    );
    input.insert("priority".to_string(), Value::String("normal".to_string()));
    input.insert(
        "model_purpose".to_string(),
        Value::String("artifact_continuation".to_string()),
    );
    input.insert("source".to_string(), Value::String(source.to_string()));

    ArtifactContinuationPlan {
        intent: intent.to_string(),
        task_type: "employment.search_opportunities".to_string(),
        name: name.to_string(),
        description: Some(format!(
            "Created from artifact continuation for {}.",
            artifact.id
        )),
        input_json,
    }
}

fn artifact_content_json(artifact: &TaskArtifact) -> Result<Value, AppError> {
    artifact.content_json.clone().ok_or_else(|| {
        AppError::BadRequest(format!("artifact {} has no JSON content", artifact.id))
    })
}

fn generic_artifact_summary_plan(
    artifact: &TaskArtifact,
    message: &str,
    source: &str,
) -> ArtifactContinuationPlan {
    ArtifactContinuationPlan {
        intent: "artifact.continue.summary".to_string(),
        task_type: "artifact.summarize".to_string(),
        name: "Continue from artifact".to_string(),
        description: Some(format!(
            "Created from artifact continuation for {}.",
            artifact.id
        )),
        input_json: json!({
            "user_request": message,
            "artifact_name": artifact.name,
            "artifact_type": artifact.artifact_type,
            "source_artifact_id": artifact.id,
            "content_text": artifact.content_text,
            "content_json": artifact.content_json,
            "source": source,
            "model_purpose": "artifact_continuation",
        }),
    }
}

fn extract_escalation_recommended_actions(
    artifact: &TaskArtifact,
    source: &str,
) -> Vec<RecommendedEscalationAction> {
    let mut values = Vec::new();
    if let Some(content_json) = &artifact.content_json {
        collect_recommended_action_values(content_json, &mut values);
        if let Some(parsed_response) = content_json.get("parsed_response") {
            collect_recommended_action_values(parsed_response, &mut values);
        }
        if let Some(raw_response) = content_json.get("raw_response") {
            collect_recommended_action_values(raw_response, &mut values);
        }
    }
    if values.is_empty() {
        if let Some(content_text) = &artifact.content_text {
            values.extend(
                extract_text_recommendations(content_text)
                    .into_iter()
                    .map(Value::String),
            );
        }
    }

    let mut actions = Vec::new();
    for value in values {
        if let Some(action) = recommended_action_from_value(&value, artifact, source) {
            if !actions
                .iter()
                .any(|existing: &RecommendedEscalationAction| {
                    existing.title.eq_ignore_ascii_case(&action.title)
                })
            {
                actions.push(action);
            }
        }
    }

    actions
}

fn collect_recommended_action_values(value: &Value, values: &mut Vec<Value>) {
    for key in [
        "recommended_next_steps",
        "recommended_actions",
        "next_steps",
        "actions",
        "follow_up_tasks",
        "tasks",
        "recommendations",
    ] {
        if let Some(array) = value.get(key).and_then(|value| value.as_array()) {
            values.extend(array.iter().cloned());
        }
    }
}

fn extract_text_recommendations(content_text: &str) -> Vec<String> {
    content_text
        .lines()
        .filter_map(|line| {
            let cleaned = line
                .trim()
                .trim_start_matches(|ch: char| {
                    ch.is_ascii_digit() || matches!(ch, '.' | ')' | '-' | '*' | '[' | ']' | ' ')
                })
                .trim();
            if cleaned.len() >= 12
                && contains_any(
                    &cleaned.to_lowercase(),
                    &[
                        "create",
                        "add",
                        "run",
                        "search",
                        "read",
                        "score",
                        "summarize",
                        "review",
                        "follow up",
                    ],
                )
            {
                Some(cleaned.to_string())
            } else {
                None
            }
        })
        .take(10)
        .collect()
}

fn recommended_action_from_value(
    value: &Value,
    artifact: &TaskArtifact,
    source: &str,
) -> Option<RecommendedEscalationAction> {
    let title = match value {
        Value::String(text) => clean_optional(Some(text)),
        Value::Object(object) => ["title", "action", "task", "summary", "name"]
            .iter()
            .find_map(|key| object.get(*key).and_then(|value| value.as_str()))
            .and_then(|value| clean_optional(Some(value))),
        _ => None,
    }?;
    let detail = match value {
        Value::Object(object) => ["detail", "description", "reason", "notes"]
            .iter()
            .find_map(|key| object.get(*key).and_then(|value| value.as_str()))
            .and_then(|value| clean_optional(Some(value))),
        _ => None,
    };
    let suggested_task_type = match value {
        Value::Object(object) => object
            .get("task_type")
            .and_then(|value| value.as_str())
            .filter(|task_type| is_supported_follow_up_task_type(task_type))
            .map(str::to_string)
            .unwrap_or_else(|| classify_recommended_task_type(&title, detail.as_deref())),
        _ => classify_recommended_task_type(&title, detail.as_deref()),
    };
    let input_json = build_recommended_action_input(
        &suggested_task_type,
        &title,
        detail.as_deref(),
        artifact,
        source,
    );

    Some(RecommendedEscalationAction {
        title,
        detail,
        suggested_task_type,
        input_json,
    })
}

fn classify_recommended_task_type(title: &str, detail: Option<&str>) -> String {
    let normalized = format!("{} {}", title, detail.unwrap_or_default()).to_lowercase();
    if contains_any(
        &normalized,
        &[
            "job",
            "opportunit",
            "employment",
            "resume",
            "cover letter",
            "candidate",
            "interview",
            "score",
        ],
    ) {
        "employment.search_opportunities".to_string()
    } else if contains_any(
        &normalized,
        &["search", "find", "look up", "web", "internet"],
    ) {
        "reader.search_web".to_string()
    } else if contains_any(&normalized, &["read url", "read page", "fetch url"]) {
        "reader.read_url".to_string()
    } else if contains_any(&normalized, &["status", "health check", "system report"]) {
        "system.status_report".to_string()
    } else if contains_any(&normalized, &["escalate", "chatgpt", "chat gpt"]) {
        "operator.escalate_to_chatgpt".to_string()
    } else {
        "artifact.summarize".to_string()
    }
}

fn is_supported_follow_up_task_type(task_type: &str) -> bool {
    matches!(
        task_type,
        "artifact.summarize"
            | "employment.search_opportunities"
            | "operator.escalate_to_chatgpt"
            | "reader.read_url"
            | "reader.search_web"
            | "system.status_report"
            | "system.escalate_to_chatgpt"
    )
}

fn build_recommended_action_input(
    task_type: &str,
    title: &str,
    detail: Option<&str>,
    artifact: &TaskArtifact,
    source: &str,
) -> Value {
    let user_request = detail
        .map(|detail| format!("{}\n\n{}", title, detail))
        .unwrap_or_else(|| title.to_string());
    match task_type {
        "reader.search_web" => json!({
            "query": user_request,
            "limit": 10,
            "source": source,
            "source_artifact_id": artifact.id,
            "source_artifact_type": artifact.artifact_type,
            "model_purpose": "escalation_follow_up",
        }),
        "reader.read_url" => json!({
            "url": title,
            "source": source,
            "source_artifact_id": artifact.id,
            "source_artifact_type": artifact.artifact_type,
            "model_purpose": "escalation_follow_up",
        }),
        "employment.search_opportunities" => json!({
            "user_request": user_request,
            "limit": 10,
            "create_opportunities": wants_create_opportunities(&user_request),
            "min_score": requested_min_score(&user_request).unwrap_or(0),
            "source": source,
            "source_artifact_id": artifact.id,
            "source_artifact_type": artifact.artifact_type,
            "model_purpose": "escalation_follow_up",
        }),
        "system.status_report" => json!({
            "user_request": user_request,
            "source": source,
            "source_artifact_id": artifact.id,
            "source_artifact_type": artifact.artifact_type,
            "model_purpose": "escalation_follow_up",
        }),
        "operator.escalate_to_chatgpt" | "system.escalate_to_chatgpt" => json!({
            "user_request": user_request,
            "mode": "manual",
            "confirm": false,
            "desired_output": "Return structured findings, recommendations, and next steps.",
            "source": source,
            "source_artifact_id": artifact.id,
            "source_artifact_type": artifact.artifact_type,
            "model_purpose": "escalation_packet",
        }),
        _ => json!({
            "user_request": user_request,
            "artifact_name": artifact.name,
            "artifact_type": artifact.artifact_type,
            "source_artifact_id": artifact.id,
            "content_text": artifact.content_text,
            "content_json": artifact.content_json,
            "source": source,
            "model_purpose": "escalation_follow_up",
        }),
    }
}

async fn create_escalation_follow_up_tasks(
    state: &AppState,
    profile_id: Uuid,
    artifact: &TaskArtifact,
    actions: &[RecommendedEscalationAction],
) -> Result<Vec<OpTask>, AppError> {
    let mut tasks = Vec::new();
    for action in actions {
        let task = state
            .op_tasks
            .create_task(
                profile_id,
                action.suggested_task_type.clone(),
                format!("Follow up: {}", action.title),
                action.detail.clone().or_else(|| {
                    Some(format!(
                        "Draft task created from ChatGPT escalation response artifact {}.",
                        artifact.id
                    ))
                }),
                action.input_json.clone(),
                false,
            )
            .await?;
        create_entity_link(
            state,
            "op_task",
            task.id,
            "task_artifact",
            artifact.id,
            "created_from_escalation_response",
        )
        .await?;
        tasks.push(task);
    }

    Ok(tasks)
}

fn is_employment_continuation(message: &str) -> bool {
    let normalized = message.to_lowercase();
    normalized.contains("job")
        || normalized.contains("opportunit")
        || normalized.contains("score")
        || normalized.contains("profile")
        || normalized.contains("candidate")
        || normalized.contains("employment")
}

fn wants_create_opportunities(message: &str) -> bool {
    let normalized = message.to_lowercase();
    normalized.contains("create opportunit")
        || normalized.contains("save opportunit")
        || normalized.contains("add opportunit")
        || normalized.contains("create employment record")
}

fn requested_limit(message: &str) -> Option<usize> {
    let normalized = message.to_lowercase();
    let words = normalized
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();

    for (index, word) in words.iter().enumerate() {
        if matches!(*word, "top" | "limit" | "first") {
            if let Some(next) = words.get(index + 1) {
                if let Ok(limit) = next.parse::<usize>() {
                    return Some(limit.clamp(1, 25));
                }
            }
        }
    }

    None
}

fn requested_min_score(message: &str) -> Option<i64> {
    let normalized = message.to_lowercase();
    let words = normalized
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();

    for (index, word) in words.iter().enumerate() {
        if matches!(*word, "score" | "min" | "minimum") {
            if let Some(next) = words.get(index + 1) {
                if let Ok(score) = next.parse::<i64>() {
                    return Some(score.clamp(0, 100));
                }
            }
        }
    }

    None
}

fn ensure_structured_json(value: &Value, field_name: &str) -> Result<(), AppError> {
    if value.is_object() || value.is_array() {
        Ok(())
    } else {
        Err(AppError::BadRequest(format!(
            "{} must be structured JSON object or array",
            field_name
        )))
    }
}

fn classify_escalation_artifact_policy(
    content_text: Option<&str>,
    content_json: &Value,
    confirm: bool,
) -> Result<EscalationArtifactPolicyDecision, AppError> {
    ensure_no_secret_content(content_text, content_json)?;
    let text = format!(
        "{} {}",
        content_text.unwrap_or_default(),
        serde_json::to_string(content_json).unwrap_or_default()
    )
    .to_lowercase();

    let privacy_classification = if contains_any(
        &text,
        &[
            "resume",
            "employment",
            "job",
            "opportunity",
            "salary",
            "cover letter",
            "interview",
            "candidate",
            "career",
        ],
    ) {
        EscalationArtifactPrivacyClass::Employment
    } else if contains_any(
        &text,
        &[
            "email",
            "phone",
            "address",
            "medical",
            "health",
            "family",
            "personal",
            "ssn",
            "social security",
        ],
    ) {
        EscalationArtifactPrivacyClass::Personal
    } else {
        EscalationArtifactPrivacyClass::TechnicalOnly
    };

    match privacy_classification {
        EscalationArtifactPrivacyClass::TechnicalOnly => Ok(EscalationArtifactPolicyDecision {
            allowed: true,
            requires_confirmation: false,
            privacy_classification,
            reason: "Technical-only escalation is allowed without confirmation.".to_string(),
        }),
        EscalationArtifactPrivacyClass::Personal | EscalationArtifactPrivacyClass::Employment
            if confirm =>
        {
            Ok(EscalationArtifactPolicyDecision {
                allowed: true,
                requires_confirmation: true,
                privacy_classification,
                reason: "Confirmed personal or employment escalation.".to_string(),
            })
        }
        EscalationArtifactPrivacyClass::Personal | EscalationArtifactPrivacyClass::Employment => {
            Err(AppError::PolicyDenied(
                "personal or employment escalation requires confirmation".to_string(),
            ))
        }
    }
}

fn ensure_no_secret_content(
    content_text: Option<&str>,
    content_json: &Value,
) -> Result<(), AppError> {
    let text = format!(
        "{} {}",
        content_text.unwrap_or_default(),
        serde_json::to_string(content_json).unwrap_or_default()
    );
    let lower = text.to_lowercase();
    if contains_any(
        &lower,
        &[
            "password",
            "api_key",
            "apikey",
            "secret",
            "authorization",
            "bearer ",
            "private_key",
            "token",
            "cookie",
        ],
    ) || text.split_whitespace().any(looks_like_secret_token)
    {
        return Err(AppError::PolicyDenied(
            "escalation artifact contains secrets and is blocked".to_string(),
        ));
    }

    Ok(())
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

fn looks_like_secret_token(value: &str) -> bool {
    let trimmed = value.trim_matches(|ch: char| {
        matches!(ch, ',' | '.' | ';' | ':' | '"' | '\'' | ')' | ']' | '}')
    });
    if trimmed.len() == 36 && trimmed.chars().filter(|ch| *ch == '-').count() == 4 {
        return false;
    }

    trimmed.starts_with("sk-")
        || trimmed.starts_with("ghp_")
        || trimmed.starts_with("xoxb-")
        || (trimmed.len() >= 32
            && trimmed
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.')))
}

fn clean_optional(value: Option<&str>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn merge_metadata(metadata: Option<Value>, required: Value) -> Value {
    let mut merged = metadata.unwrap_or_else(|| json!({}));
    if !merged.is_object() {
        merged = json!({ "user_metadata": merged });
    }

    if let (Some(target), Some(required)) = (merged.as_object_mut(), required.as_object()) {
        for (key, value) in required {
            target.insert(key.clone(), value.clone());
        }
    }

    merged
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

async fn create_artifact_link(
    state: &AppState,
    response_artifact_id: Uuid,
    request_artifact_id: Uuid,
    relationship: &str,
) -> Result<(), AppError> {
    state
        .session_memory
        .create_task_link(TaskLink::new(
            "task_artifact".to_string(),
            response_artifact_id,
            "task_artifact".to_string(),
            request_artifact_id,
            relationship.to_string(),
        ))
        .await
        .map(|_| ())
        .map_err(|err| AppError::Internal(err.to_string()))
}

async fn create_entity_link(
    state: &AppState,
    source_type: &str,
    source_id: Uuid,
    target_type: &str,
    target_id: Uuid,
    relationship: &str,
) -> Result<(), AppError> {
    state
        .session_memory
        .create_task_link(TaskLink::new(
            source_type.to_string(),
            source_id,
            target_type.to_string(),
            target_id,
            relationship.to_string(),
        ))
        .await
        .map(|_| ())
        .map_err(|err| AppError::Internal(err.to_string()))
}

fn json_path_action(
    action: &str,
    method: &str,
    path: String,
    artifact_id: Option<Uuid>,
    run_id: Option<Uuid>,
) -> Value {
    let mut value = json!({
        "action": action,
        "method": method,
        "path": path,
    });
    if let Some(artifact_id) = artifact_id {
        value["artifact_id"] = Value::String(artifact_id.to_string());
    }
    if let Some(run_id) = run_id {
        value["run_id"] = Value::String(run_id.to_string());
    }
    value
}

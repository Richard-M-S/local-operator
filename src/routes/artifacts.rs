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
}

#[derive(Debug, Serialize)]
pub struct ContinueArtifactResponse {
    pub ok: bool,
    pub intent: String,
    pub task_request: TaskRequest,
    pub task: OpTask,
    pub run: OpTaskRunSummary,
    pub artifacts: Vec<TaskRunArtifactSummary>,
    pub next_actions: Vec<String>,
    pub next_suggested_action: Value,
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

    Ok(Json(ContinueArtifactResponse {
        ok: matches!(run.status, OpTaskRunStatus::Succeeded),
        intent: continuation.intent,
        task_request,
        task,
        run: run_summary,
        artifacts,
        next_actions: vec![
            "show_latest_artifacts".to_string(),
            "continue_from_artifact".to_string(),
        ],
        next_suggested_action,
    }))
}

impl LatestArtifactSummary {
    fn from_artifact(artifact: TaskArtifact, include_content: bool) -> Self {
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
        _ => Ok(generic_artifact_summary_plan(artifact, message, source)),
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

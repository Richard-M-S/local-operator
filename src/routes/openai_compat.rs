use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    app_state::AppState,
    domains::employment::models::default_employment_profile_id,
    error::AppError,
    models::session::{ChatMessage, ChatSession},
    op_tasks::models::TaskArtifact,
};

#[derive(Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(default)]
    pub user: Option<String>,
    #[serde(default)]
    pub session_id: Option<Uuid>,
    #[serde(default)]
    pub profile_id: Option<Uuid>,
    #[serde(default)]
    pub metadata: Option<Value>,
}

#[derive(Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Serialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub session_id: String,
    pub choices: Vec<Choice>,
}

#[derive(Serialize)]
pub struct Choice {
    pub index: u32,
    pub message: MessageOut,
}

#[derive(Serialize)]
pub struct MessageOut {
    pub role: String,
    pub content: String,
}

pub async fn models() -> Json<serde_json::Value> {
    Json(json!({
        "object": "list",
        "data": [
            {
                "id": "local-operator-home",
                "object": "model",
                "owned_by": "local"
            }
        ]
    }))
}

pub async fn chat_completions(
    State(state): State<AppState>,
    Json(req): Json<ChatCompletionRequest>,
) -> Result<Json<ChatCompletionResponse>, AppError> {
    let session = resolve_chat_session(&state, &req).await?;
    let profile_id = session.profile_id;
    let user_message = req
        .messages
        .iter()
        .rev()
        .find(|m| m.role == "user")
        .map(|m| m.content.clone())
        .unwrap_or_default();

    let stored_user_message =
        persist_chat_message(&state, session.id, "user", &user_message, None, None, None).await?;

    let persisted_messages = state
        .session_memory
        .list_chat_messages(session.id, 40)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;

    let mut response_task_request_id = stored_user_message.task_request_id;
    let mut response_run_id = stored_user_message.run_id;
    let mut response_artifact_id = stored_user_message.artifact_id;

    let content = if is_artifact_follow_up(&user_message) {
        if let Some(artifact_id) = state
            .session_memory
            .last_artifact_id_for_session(session.id)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?
        {
            let artifact = state.op_tasks.get_artifact(artifact_id).await?;
            response_artifact_id = Some(artifact.id);
            answer_from_artifact(&artifact, &user_message, &persisted_messages)
        } else {
            "I do not have a prior search artifact in this chat session yet.".to_string()
        }
    } else if req.model == "local-operator-home" {
        let operator_message = if should_use_latest_message_for_action(&user_message) {
            user_message.clone()
        } else {
            render_full_transcript(&persisted_messages, &req.messages)
        };
        let response = state
            .operator
            .run_chat_with_session(
                &operator_message,
                true,
                profile_id,
                session.id,
                "openai_compat",
            )
            .await?;
        response_task_request_id = json_uuid(&response.data, "task_request_id");
        response_run_id = json_uuid_path(&response.data, &["run", "id"]);
        response_artifact_id = json_uuid_path(&response.data, &["artifact", "id"]);
        response.message
    } else {
        let llm = state
            .llm
            .as_ref()
            .ok_or_else(|| AppError::Internal("LLM service is not enabled".to_string()))?;

        let system = req
            .messages
            .iter()
            .filter(|m| m.role == "system")
            .map(|m| m.content.as_str())
            .collect::<Vec<_>>()
            .join("\n\n");

        let transcript = render_full_transcript(&persisted_messages, &req.messages);

        llm.ask_model(&req.model, &system, &transcript).await?
    };

    persist_chat_message(
        &state,
        session.id,
        "assistant",
        &content,
        response_task_request_id,
        response_run_id,
        response_artifact_id,
    )
    .await?;
    state
        .session_memory
        .update_chat_session_memory(
            session.id,
            response_task_request_id,
            response_run_id,
            response_artifact_id,
        )
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(Json(ChatCompletionResponse {
        id: "chatcmpl-local".to_string(),
        object: "chat.completion".to_string(),
        session_id: session.id.to_string(),
        choices: vec![Choice {
            index: 0,
            message: MessageOut {
                role: "assistant".to_string(),
                content,
            },
        }],
    }))
}

async fn resolve_chat_session(
    state: &AppState,
    req: &ChatCompletionRequest,
) -> Result<ChatSession, AppError> {
    if let Some(session_id) = request_session_id(req) {
        if let Some(session) = state
            .session_memory
            .get_chat_session(session_id)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?
        {
            return Ok(session);
        }

        let mut session = ChatSession::new(request_profile_id(req));
        session.id = session_id;
        return state
            .session_memory
            .create_chat_session(session)
            .await
            .map_err(|err| AppError::Internal(err.to_string()));
    }

    if let Some(external_conversation_id) = request_external_conversation_id(req) {
        return state
            .session_memory
            .get_or_create_external_chat_session(
                request_profile_id(req),
                "openai_compat",
                &external_conversation_id,
            )
            .await
            .map_err(|err| AppError::Internal(err.to_string()));
    }

    state
        .session_memory
        .create_chat_session(ChatSession::with_external_source(
            request_profile_id(req),
            "openai_compat".to_string(),
            Uuid::new_v4().to_string(),
        ))
        .await
        .map_err(|err| AppError::Internal(err.to_string()))
}

async fn persist_chat_message(
    state: &AppState,
    session_id: Uuid,
    role: &str,
    content: &str,
    task_request_id: Option<Uuid>,
    run_id: Option<Uuid>,
    artifact_id: Option<Uuid>,
) -> Result<ChatMessage, AppError> {
    let mut message = ChatMessage::new(session_id, role.to_string(), content.to_string());
    message.task_request_id = task_request_id;
    message.run_id = run_id;
    message.artifact_id = artifact_id;
    state
        .session_memory
        .create_chat_message(message)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))
}

fn request_profile_id(req: &ChatCompletionRequest) -> Uuid {
    req.profile_id
        .or_else(|| metadata_uuid(req, "profile_id"))
        .unwrap_or_else(default_employment_profile_id)
}

fn request_session_id(req: &ChatCompletionRequest) -> Option<Uuid> {
    req.session_id.or_else(|| metadata_uuid(req, "session_id"))
}

fn request_external_conversation_id(req: &ChatCompletionRequest) -> Option<String> {
    metadata_string(req, "conversation_id")
        .or_else(|| metadata_string(req, "thread_id"))
        .or_else(|| req.user.clone())
        .filter(|value| !value.trim().is_empty())
}

fn metadata_uuid(req: &ChatCompletionRequest, key: &str) -> Option<Uuid> {
    metadata_string(req, key).and_then(|value| Uuid::parse_str(&value).ok())
}

fn metadata_string(req: &ChatCompletionRequest, key: &str) -> Option<String> {
    req.metadata
        .as_ref()
        .and_then(|metadata| metadata.get(key))
        .and_then(|value| value.as_str())
        .map(str::to_string)
}

fn is_artifact_follow_up(message: &str) -> bool {
    let normalized = message.trim().to_lowercase();
    matches!(
        normalized.as_str(),
        "what are they?"
            | "what are they"
            | "what were they?"
            | "what were they"
            | "show them"
            | "show me them"
            | "list them"
            | "show the results"
            | "what are the results?"
            | "what are the results"
    ) || ((normalized.contains("they") || normalized.contains("them"))
        && (normalized.contains("what")
            || normalized.contains("list")
            || normalized.contains("show")))
        || normalized.contains("those results")
        || normalized.contains("these results")
}

fn answer_from_artifact(
    artifact: &TaskArtifact,
    user_message: &str,
    _history: &[ChatMessage],
) -> String {
    if artifact.artifact_type == "search_result_set" {
        return answer_from_search_result_set(artifact);
    }

    let content = artifact
        .content_text
        .clone()
        .filter(|content| !content.trim().is_empty())
        .or_else(|| {
            artifact
                .content_json
                .as_ref()
                .and_then(|json| serde_json::to_string_pretty(json).ok())
        })
        .unwrap_or_else(|| "The prior artifact has no readable content.".to_string());

    format!(
        "The last relevant artifact is '{}'. For '{}', here is the saved content:\n\n{}",
        artifact.name,
        user_message,
        truncate_chars(&content, 3000)
    )
}

fn answer_from_search_result_set(artifact: &TaskArtifact) -> String {
    let results = artifact
        .content_json
        .as_ref()
        .and_then(|json| json.get("results"))
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();

    if results.is_empty() {
        return "The last search artifact does not contain any results.".to_string();
    }

    let query = artifact
        .content_json
        .as_ref()
        .and_then(|json| json.get("query"))
        .and_then(|value| value.as_str())
        .unwrap_or("the prior search");

    let mut response = format!("They are the saved results for '{}':", query);
    for (index, result) in results.iter().take(10).enumerate() {
        let title = result
            .get("title")
            .and_then(|value| value.as_str())
            .unwrap_or("Untitled result");
        let url = result
            .get("url")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let snippet = result
            .get("snippet")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        response.push_str(&format!(
            "\n{}. {}\n   {}\n   {}",
            index + 1,
            title,
            url,
            snippet
        ));
    }

    response
}

fn render_full_transcript(persisted: &[ChatMessage], request_messages: &[Message]) -> String {
    let persisted = persisted
        .iter()
        .map(|message| format!("{}: {}", message.role, message.content))
        .collect::<Vec<_>>()
        .join("\n");
    let current = request_messages
        .iter()
        .filter(|message| message.role != "system")
        .map(|message| format!("{}: {}", message.role, message.content))
        .collect::<Vec<_>>()
        .join("\n");

    match (persisted.is_empty(), current.is_empty()) {
        (true, true) => String::new(),
        (true, false) => current,
        (false, true) => persisted,
        (false, false) => format!(
            "Persisted conversation:\n{}\n\nCurrent request:\n{}",
            persisted, current
        ),
    }
}

fn should_use_latest_message_for_action(message: &str) -> bool {
    let normalized = message.to_lowercase();
    let has_url = normalized.contains("http://") || normalized.contains("https://");
    let read_action = has_url
        && (normalized.contains("read")
            || normalized.contains("fetch")
            || normalized.contains("open")
            || normalized.contains("url"));
    let search_action = normalized.contains("search")
        || normalized.contains("find jobs")
        || normalized.contains("look for jobs")
        || normalized.contains("opportunit")
        || normalized.contains("employment");
    let home_action = normalized.contains("home")
        || normalized.contains("house")
        || normalized.contains("front door")
        || normalized.contains("lock")
        || normalized.contains("weather")
        || normalized.contains("vacuum");

    read_action || search_action || home_action
}

fn json_uuid(data: &Value, key: &str) -> Option<Uuid> {
    data.get(key)
        .and_then(|value| value.as_str())
        .and_then(|value| Uuid::parse_str(value).ok())
}

fn json_uuid_path(data: &Value, path: &[&str]) -> Option<Uuid> {
    let mut value = data;
    for key in path {
        value = value.get(*key)?;
    }
    value.as_str().and_then(|value| Uuid::parse_str(value).ok())
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut result = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        result.push_str("\n\n[truncated]");
    }
    result
}

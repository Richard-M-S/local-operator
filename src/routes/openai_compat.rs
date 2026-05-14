use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    app_state::AppState, domains::employment::models::default_employment_profile_id,
    error::AppError,
};

#[derive(Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<Message>,
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
    let user_message = req
        .messages
        .iter()
        .rev()
        .find(|m| m.role == "user")
        .map(|m| m.content.clone())
        .unwrap_or_default();

    let content = if req.model == "local-operator-home" {
        state
            .operator
            .run_chat(&user_message, true, Some(default_employment_profile_id()))
            .await?
            .message
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

        let transcript = req
            .messages
            .iter()
            .filter(|m| m.role != "system")
            .map(|m| format!("{}: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n");

        llm.ask_model(&req.model, &system, &transcript).await?
    };

    Ok(Json(ChatCompletionResponse {
        id: "chatcmpl-local".to_string(),
        object: "chat.completion".to_string(),
        choices: vec![Choice {
            index: 0,
            message: MessageOut {
                role: "assistant".to_string(),
                content,
            },
        }],
    }))
}

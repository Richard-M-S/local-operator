use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{app_state::AppState, error::AppError};

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
    // get last user message
    let user_message = req
        .messages
        .iter()
        .rev()
        .find(|m| m.role == "user")
        .map(|m| m.content.clone())
        .unwrap_or_default();

    let result = state.operator.run_chat(&user_message, true).await?;

    Ok(Json(ChatCompletionResponse {
        id: "chatcmpl-local".to_string(),
        object: "chat.completion".to_string(),
        choices: vec![Choice {
            index: 0,
            message: MessageOut {
                role: "assistant".to_string(),
                content: result.message,
            },
        }],
    }))
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CommandRequest {
    pub input: String,
    pub confirm: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct CommandResponse {
    pub request_id: uuid::Uuid,
    pub parsed_intent: String,
    pub allowed: bool,
    pub risk_tier: i32,
    pub message: String,
    pub actions: serde_json::Value,
    pub results: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct ToolExecuteRequest {
    pub tool: String,
    pub args: serde_json::Value,
    pub confirm: Option<bool>,
}
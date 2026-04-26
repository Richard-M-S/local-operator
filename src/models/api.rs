use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
pub struct ChatRequest {
    pub message: String,
    #[serde(default)]
    pub include_home: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatResponse {
    pub ok: bool,
    pub mode: String,
    pub message: String,
    pub data: Value,
}

#[derive(Debug, Deserialize)]
pub struct CommandRequest {
    pub input: String,
    pub confirm: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct CommandResponse {
    pub ok: bool,
    pub mode: String,
    pub message: String,
    pub data: Value,
}

#[derive(Debug, Deserialize)]
pub struct ToolExecuteRequest {
    pub tool: String,

    #[serde(default)]
    pub args: Value,

    pub confirm: Option<bool>,
}

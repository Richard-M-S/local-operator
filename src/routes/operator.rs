use axum::{extract::State, Json};
use serde::Deserialize;
use serde_json::Value;

use crate::{
    app_state::AppState,
    error::AppError,
    models::api::{ChatRequest, ChatResponse, CommandRequest, CommandResponse, ToolExecuteRequest},
};

pub async fn chat(
    State(state): State<AppState>,
    Json(req): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, AppError> {
    let include_home = req.include_home.unwrap_or(true);

    let result = state
        .operator
        .run_chat(&req.message, include_home)
        .await?;

    Ok(Json(result))
}

pub async fn command(
    State(state): State<AppState>,
    Json(req): Json<CommandRequest>,
) -> Result<Json<CommandResponse>, AppError> {
    let result = state
        .operator
        .run_command(&req.input, req.confirm.unwrap_or(false))
        .await?;
    Ok(Json(result))
}

pub async fn execute_tool(
    State(state): State<AppState>,
    Json(req): Json<ToolExecuteRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let descriptor = state.tools.describe(&req.tool).await?;
    state
        .policy
        .check_tool_execution(descriptor.risk_tier, req.confirm.unwrap_or(false))?;

    let result = state.tools.execute(&req.tool, req.args).await?;
    let _ = state.audit.record_tool_call(&req.tool, true).await;

    Ok(Json(
        serde_json::to_value(result).map_err(|e| AppError::Internal(e.to_string()))?,
    ))
}

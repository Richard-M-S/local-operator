use axum::{extract::State, Json};
//use serde::Deserialize;
//use serde_json::Value;

use crate::{
    app_state::AppState,
    domains::employment::models::default_employment_profile_id,
    error::AppError,
    models::api::{ChatRequest, ChatResponse, CommandRequest, CommandResponse, ToolExecuteRequest},
    services::execution::ExecutionContext,
};

pub async fn chat(
    State(state): State<AppState>,
    Json(req): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, AppError> {
    let include_home = req.include_home.unwrap_or(true);

    let result = state
        .operator
        .run_chat(
            &req.message,
            include_home,
            Some(req.profile_id.unwrap_or_else(default_employment_profile_id)),
        )
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
    let result = state
        .tool_execution
        .execute(
            &req.tool,
            req.args,
            req.confirm.unwrap_or(false),
            ExecutionContext::default().with_input_summary("direct /api/tools/execute call"),
        )
        .await?;

    Ok(Json(
        serde_json::to_value(result).map_err(|e| AppError::Internal(e.to_string()))?,
    ))
}

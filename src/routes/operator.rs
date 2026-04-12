use axum::{extract::State, Json};

use crate::{
    app_state::AppState,
    error::AppError,
    models::api::{CommandRequest, CommandResponse, ToolExecuteRequest},
};

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

    Ok(Json(serde_json::to_value(result).map_err(|e| AppError::Internal(e.to_string()))?))

}
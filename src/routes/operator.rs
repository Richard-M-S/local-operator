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
    let result = state.tools.execute(&req.tool, req.args).await?;
    Ok(Json(result))
}
use axum::{extract::State, Json};

use crate::{app_state::AppState, error::AppError};

pub async fn recent(State(state): State<AppState>) -> Result<Json<serde_json::Value>, AppError> {
    let items = state.audit.recent(25).await.map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(serde_json::json!({ "items": items })))
}
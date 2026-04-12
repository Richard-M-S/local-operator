use axum::{extract::State, Json};

use crate::{app_state::AppState, error::AppError};

pub async fn status(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let system = state
        .tools
        .execute("system.get_status", serde_json::json!({}))
        .await?;

    let docker = state
        .tools
        .execute("docker.list_containers", serde_json::json!({}))
        .await?;

    let home = state
        .tools
        .execute("ha.get_summary", serde_json::json!({}))
        .await
        .ok();

    Ok(Json(serde_json::json!({
        "system": system,
        "docker": docker,
        "home": home
    })))
}
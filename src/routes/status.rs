use axum::{extract::State, Json};

use crate::{app_state::AppState, error::AppError};

pub async fn status(State(state): State<AppState>) -> Result<Json<serde_json::Value>, AppError> {
    let system = subsystem_result(
        state
            .tools
            .execute("system.get_status", serde_json::json!({}))
            .await,
    );

    let docker = subsystem_result(
        state
            .tools
            .execute("docker.list_containers", serde_json::json!({}))
            .await,
    );

    let home = subsystem_result(
        state
            .tools
            .execute("ha.get_summary", serde_json::json!({}))
            .await,
    );

    Ok(Json(serde_json::json!({
        "system": system,
        "docker": docker,
        "home": home
    })))
}

fn subsystem_result(
    result: Result<crate::models::tool::ToolExecutionResult, AppError>,
) -> serde_json::Value {
    match result {
        Ok(value) => serde_json::to_value(value).unwrap_or_else(|_| {
            serde_json::json!({
                "ok": false,
                "error": "failed to serialize subsystem status"
            })
        }),
        Err(err) => serde_json::json!({
            "ok": false,
            "error": err.to_string()
        }),
    }
}

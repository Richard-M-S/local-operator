use axum::{
    routing::{get, post},
    Router,
};

use crate::app_state::AppState;

pub mod audit;
pub mod health;
pub mod openai_compat;
pub mod operator;
pub mod status;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health::health))
        .route("/api/status", get(status::status))
        .route("/api/operator/command", post(operator::command))
        .route("/api/operator/chat", post(operator::chat))
        .route("/api/tools/execute", post(operator::execute_tool))
        .route("/api/audit/recent", get(audit::recent))
        .route("/v1/models", get(openai_compat::models))
        .route(
            "/v1/chat/completions",
            post(openai_compat::chat_completions),
        )
        .with_state(state)
}

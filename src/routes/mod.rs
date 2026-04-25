use axum::{
    routing::{get, post},
    Router,
};

use crate::app_state::AppState;

pub mod audit;
pub mod health;
pub mod operator;
pub mod status;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health::health))
        .route("/api/status", get(status::status))
        .route("/api/operator/command", post(operator::command))
        .route("/api/tools/execute", post(operator::execute_tool))
        .route("/api/audit/recent", get(audit::recent))
        .with_state(state)
}

use axum::{
    middleware,
    routing::{get, post},
    Router,
};

use crate::app_state::AppState;

pub mod audit;
pub mod auth;
pub mod health;
pub mod op_tasks;
pub mod openai_compat;
pub mod operator;
pub mod status;

pub fn router(state: AppState) -> Router {
    let protected_routes = Router::new()
        .route("/api/status", get(status::status))
        .route("/api/op-tasks", post(op_tasks::create).get(op_tasks::list))
        .route("/api/op-tasks/{id}/run", post(op_tasks::run))
        .route("/api/operator/command", post(operator::command))
        .route("/api/operator/chat", post(operator::chat))
        .route("/api/tools/execute", post(operator::execute_tool))
        .route("/api/audit/recent", get(audit::recent))
        .route("/v1/models", get(openai_compat::models))
        .route(
            "/v1/chat/completions",
            post(openai_compat::chat_completions),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_api_token,
        ));

    Router::new()
        .route("/health", get(health::health))
        .merge(protected_routes)
        .with_state(state)
}

use axum::{
    middleware,
    routing::{get, post},
    Router,
};

use crate::app_state::AppState;

pub mod audit;
pub mod auth;
pub mod context;
pub mod employment;
pub mod health;
pub mod op_tasks;
pub mod openai_compat;
pub mod operator;
pub mod status;

pub fn router(state: AppState) -> Router {
    let protected_routes = Router::new()
        .route("/api/status", get(status::status))
        .route("/api/context", get(context::list).post(context::create))
        .route("/api/context/search", get(context::search))
        .route("/api/context/:id", get(context::get))
        .route("/api/op-tasks", post(op_tasks::create).get(op_tasks::list))
        .route("/api/op-tasks/:id", get(op_tasks::get))
        .route("/api/op-tasks/:id/run", post(op_tasks::run))
        .route("/api/op-tasks/:id/runs", get(op_tasks::list_runs))
        .route("/api/op-task-artifacts", get(op_tasks::list_artifacts))
        .route(
            "/api/op-task-artifacts/:id/content",
            get(op_tasks::get_artifact_content),
        )
        .route(
            "/api/op-task-artifacts/:id/save-context",
            post(op_tasks::save_artifact_context),
        )
        .route("/api/op-task-artifacts/:id", get(op_tasks::get_artifact))
        .route("/api/op-task-runs/:id", get(op_tasks::get_run))
        .route("/api/operator/command", post(operator::command))
        .route("/api/operator/chat", post(operator::chat))
        .route("/api/tools/execute", post(operator::execute_tool))
        .route("/api/audit/recent", get(audit::recent))
        .route("/v1/models", get(openai_compat::models))
        .route(
            "/api/employment/opportunities",
            get(employment::list_opportunities).post(employment::create_opportunity),
        )
        .route(
            "/api/employment/opportunities/:id",
            get(employment::get_opportunity),
        )
        .route(
            "/api/employment/opportunities/:id/parse",
            post(employment::parse_opportunity),
        )
        .route(
            "/api/employment/opportunities/:id/score",
            post(employment::score_opportunity),
        )
        .route(
            "/api/employment/opportunities/from-artifact/:artifact_id",
            post(employment::create_opportunity_from_artifact),
        )
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

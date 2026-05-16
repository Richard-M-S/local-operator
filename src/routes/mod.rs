use axum::{
    http::{HeaderValue, Method},
    middleware,
    routing::{get, post},
    Router,
};
use tower_http::cors::CorsLayer;

use crate::app_state::AppState;

pub mod audit;
pub mod auth;
pub mod context;
pub mod employment;
pub mod health;
pub mod op_tasks;
pub mod openai_compat;
pub mod openapi;
pub mod operator;
pub mod status;

pub fn router(state: AppState) -> Router {
    let protected_routes = Router::new()
        .route("/api/status", get(status::status))
        .route(
            "/api/employment/profiles",
            get(employment::list_profiles).post(employment::create_profile),
        )
        .route(
            "/api/employment/profiles/:profile_id",
            get(employment::get_profile).put(employment::update_profile),
        )
        .route(
            "/api/employment/profiles/:profile_id/context",
            get(context::list_for_profile).post(context::create_for_profile),
        )
        .route(
            "/api/employment/profiles/:profile_id/context/search",
            get(context::search_for_profile),
        )
        .route("/api/context", get(context::list).post(context::create))
        .route("/api/context/search", get(context::search))
        .route("/api/context/:id", get(context::get))
        .route(
            "/api/employment/profiles/:profile_id/op-tasks",
            post(op_tasks::create_for_profile).get(op_tasks::list_for_profile),
        )
        .route(
            "/api/employment/profiles/:profile_id/op-tasks/:id/run",
            post(op_tasks::run_for_profile),
        )
        .route(
            "/api/employment/profiles/:profile_id/op-tasks/:id/runs",
            get(op_tasks::list_runs_for_profile),
        )
        .route(
            "/api/employment/profiles/:profile_id/op-task-artifacts",
            get(op_tasks::list_artifacts_for_profile),
        )
        .route(
            "/api/employment/profiles/:profile_id/op-task-artifacts/:id/content",
            get(op_tasks::get_artifact_content_for_profile),
        )
        .route(
            "/api/employment/profiles/:profile_id/op-task-artifacts/:id/save-context",
            post(op_tasks::save_artifact_context_for_profile),
        )
        .route(
            "/api/employment/profiles/:profile_id/op-task-artifacts/:id",
            get(op_tasks::get_artifact_for_profile),
        )
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
            "/api/employment/profiles/:profile_id/opportunities",
            get(employment::list_opportunities_for_profile)
                .post(employment::create_opportunity_for_profile),
        )
        .route(
            "/api/employment/profiles/:profile_id/opportunities/:id",
            get(employment::get_opportunity_for_profile),
        )
        .route(
            "/api/employment/profiles/:profile_id/opportunities/:id/parse",
            post(employment::parse_opportunity_for_profile),
        )
        .route(
            "/api/employment/profiles/:profile_id/opportunities/:id/score",
            post(employment::score_opportunity_for_profile),
        )
        .route(
            "/api/employment/profiles/:profile_id/opportunities/:id/cover-letter",
            post(employment::generate_cover_letter_for_profile),
        )
        .route(
            "/api/employment/profiles/:profile_id/opportunities/:id/archive",
            post(employment::archive_opportunity_for_profile),
        )
        .route(
            "/api/employment/profiles/:profile_id/opportunities/:id/reject",
            post(employment::reject_opportunity_for_profile),
        )
        .route(
            "/api/employment/profiles/:profile_id/opportunities/:id/restore",
            post(employment::restore_opportunity_for_profile),
        )
        .route(
            "/api/employment/profiles/:profile_id/opportunities/from-artifact/:artifact_id",
            post(employment::create_opportunity_from_artifact_for_profile),
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
            "/api/employment/opportunities/:id/archive",
            post(employment::archive_opportunity),
        )
        .route(
            "/api/employment/opportunities/:id/reject",
            post(employment::reject_opportunity),
        )
        .route(
            "/api/employment/opportunities/:id/restore",
            post(employment::restore_opportunity),
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

    let cors = CorsLayer::new()
        .allow_origin([
            "http://localhost:5173".parse::<HeaderValue>().unwrap(),
            "http://127.0.0.1:5173".parse::<HeaderValue>().unwrap(),
            "http://192.168.0.100:5173".parse::<HeaderValue>().unwrap(),
            "http://100.71.130.87:5173".parse::<HeaderValue>().unwrap(),
            "http://localhost:3000".parse::<HeaderValue>().unwrap(),
            "http://127.0.0.1:3000".parse::<HeaderValue>().unwrap(),
            "http://192.168.0.100:3000".parse::<HeaderValue>().unwrap(),
            "http://100.71.130.87:3000".parse::<HeaderValue>().unwrap(),
        ])
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers(tower_http::cors::Any);

    Router::new()
        .route("/health", get(health::health))
        .route("/openapi.json", get(openapi::openapi_json))
        .route("/api/tools", get(openapi::openapi_json))
        .route("/api/tools/", get(openapi::openapi_json))
        .route("/api/tools/openapi.json", get(openapi::openapi_json))
        .merge(protected_routes)
        .with_state(state)
        .layer(cors)
}

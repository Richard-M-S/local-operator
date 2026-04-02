use axum::{routing::get, Router};

pub mod health;

pub fn router() -> Router {
    Router::new()
        .route("/health", get(health::health))
}
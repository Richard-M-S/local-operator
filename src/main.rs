use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use sqlx::SqlitePool;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod adapters;
mod app_state;
mod config;
mod error;
mod models;
mod routes;
mod services;
mod tools;

use app_state::AppState;
use config::AppConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load config
    let config = AppConfig::load()?;

    // DB
    let db = SqlitePool::connect(&config.database.url).await?;

    // Build state
    let state = Arc::new(AppState::new(config.clone(), db).await?);

    // Router
    let app = build_router(state);

    let addr: SocketAddr = format!("{}:{}", config.server.host, config.server.port).parse()?;

    tracing::info!("listening on {}", addr);

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn build_router(state: Arc<AppState>) -> Router {
    Router::new().nest("/api", routes::routes()).with_state(state)
}
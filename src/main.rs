mod app_state;
mod config;
mod error;
mod models;
mod routes;
mod services;
mod tools;

use std::net::SocketAddr;

use anyhow::Context;
use sqlx::sqlite::SqlitePoolOptions;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{app_state::AppState, config::AppConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let config = AppConfig::load().context("failed to load config")?;

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_new(config.logging.level.clone())
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&config.database.url)
        .await
        .context("failed to connect to sqlite")?;

    let state = AppState::new(config.clone(), db).await?;

    let app = routes::router(state);

    let addr: SocketAddr = format!("{}:{}", config.server.host, config.server.port)
        .parse()
        .context("invalid bind address")?;

    tracing::info!("local-operator listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
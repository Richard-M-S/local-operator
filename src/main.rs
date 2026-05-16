#![recursion_limit = "256"]

use std::net::SocketAddr;

use sqlx::SqlitePool;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod adapters;
mod app_state;
mod config;
mod context;
mod db;
mod domains;
mod error;
mod models;
mod op_tasks;
mod readers;
mod routes;
mod services;
mod session_memory;
mod tools;

use app_state::AppState;
use config::AppConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = AppConfig::load()?;
    let db = SqlitePool::connect(&config.database.url).await?;
    sqlx::migrate!("./migrations").run(&db).await?;
    let state = AppState::new(config.clone(), db).await?;

    let app = routes::router(state);

    let addr: SocketAddr = format!("{}:{}", config.server.host, config.server.port).parse()?;
    tracing::info!("listening on {}", addr);

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

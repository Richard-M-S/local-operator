use axum::{routing::get, Router};
use std::net::SocketAddr;

async fn health() -> &'static str {
    "ok"
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/health", get(health));

    let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    println!("local-operator running at http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
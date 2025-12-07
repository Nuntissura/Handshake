use axum::{routing::get, Json, Router};
use serde::Serialize;
use std::{net::SocketAddr, process};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    component: &'static str,
    version: &'static str,
}

async fn health() -> Json<HealthResponse> {
    println!("/health 200");
    Json(HealthResponse {
        status: "ok",
        component: "handshake_core",
        version: env!("CARGO_PKG_VERSION"),
    })
}

#[tokio::main]
async fn main() {
    let addr: SocketAddr = ([127, 0, 0, 1], 37501).into();

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new().route("/health", get(health)).layer(cors);

    println!(
        "handshake_core listening on {} (pid {})",
        addr,
        process::id()
    );

    let listener = TcpListener::bind(addr)
        .await
        .expect("failed to bind listener");

    axum::serve(listener, app).await.expect("server error");
}

use axum::{extract::State, routing::get, Json, Router};
use sqlx::{sqlite::SqliteConnectOptions, sqlite::SqlitePoolOptions, SqlitePool};
use std::{net::SocketAddr, path::Path, path::PathBuf, process, str::FromStr};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

mod api;
mod logging;
mod models;

use models::HealthResponse;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
}

#[tokio::main]
async fn main() {
    let addr: SocketAddr = ([127, 0, 0, 1], 37501).into();

    logging::init_logging();

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let pool = init_db().await.expect("failed to init database");
    let state = AppState { pool };

    let app = Router::new()
        .route("/health", get(health))
        .with_state(state.clone())
        .merge(api::routes(state))
        .layer(cors);

    tracing::info!(target: "handshake_core", listen_addr = %addr, pid = process::id(), "handshake_core started");

    let listener = TcpListener::bind(addr)
        .await
        .expect("failed to bind listener");

    axum::serve(listener, app).await.expect("server error");
}

async fn init_db() -> Result<SqlitePool, sqlx::Error> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root_dir = manifest_dir
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .expect("failed to resolve repo root");
    let data_dir = root_dir.join("data");
    if !data_dir.exists() {
        std::fs::create_dir_all(&data_dir).map_err(|err| {
            tracing::error!(target: "handshake_core", path = %data_dir.display(), error = %err, "failed to create data directory");
            sqlx::Error::Io(err)
        })?;
    }

    let db_path = data_dir.join("handshake.db");
    let db_url = format!("sqlite://{}", db_path.to_string_lossy());

    let connect_options = SqliteConnectOptions::from_str(&db_url)?
        .create_if_missing(true)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connect_options)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    tracing::info!(target: "handshake_core", db_url = %db_url, "database ready");

    Ok(pool)
}

async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    let db_check = sqlx::query("SELECT 1").execute(&state.pool).await;
    let db_status = match db_check {
        Ok(_) => "ok",
        Err(err) => {
            tracing::error!(target: "handshake_core", route = "/health", error = %err, "db check error");
            "error"
        }
    };

    let overall_status = if db_status == "ok" { "ok" } else { "error" };

    tracing::info!(target: "handshake_core", route = "/health", status = overall_status, db_status = db_status, "health check");

    Json(HealthResponse {
        status: overall_status.to_string(),
        component: "handshake_core",
        version: env!("CARGO_PKG_VERSION"),
        db_status: db_status.to_string(),
    })
}

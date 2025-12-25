use axum::{extract::State, routing::get, Json, Router};
use duckdb::Connection as DuckDbConnection;
use sqlx::{sqlite::SqliteConnectOptions, sqlite::SqlitePoolOptions, SqlitePool};
use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    process,
    str::FromStr,
    sync::{Arc, Mutex},
};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

mod api;
mod flight_recorder;
mod jobs;
mod llm;
mod logging;
mod models;
mod storage;
mod terminal;
mod workflows;

use crate::llm::{LLMClient, MockLLMClient, OllamaClient};
use models::HealthResponse;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub fr_pool: Arc<Mutex<DuckDbConnection>>,
    pub llm_client: Arc<dyn LLMClient>,
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
    let fr_pool = init_flight_recorder()
        .await
        .expect("failed to init flight recorder");

    // Initialize LLM Client. Defaulting to Ollama if configured, otherwise Mock.
    let llm_client: Arc<dyn LLMClient> = if let Ok(url) = std::env::var("OLLAMA_URL") {
        let model = match std::env::var("OLLAMA_MODEL") {
            Ok(val) => val,
            Err(_) => "llama3".to_string(),
        };
        tracing::info!(target: "handshake_core", url = %url, model = %model, "using Ollama LLM client");
        Arc::new(OllamaClient::new(url, model))
    } else {
        tracing::info!(target: "handshake_core", "using Mock LLM client");
        Arc::new(MockLLMClient {
            response: "This is a mock AI response from Handshake Core.".to_string(),
        })
    };

    let state = AppState {
        pool,
        fr_pool,
        llm_client,
    };
    let api_routes = api::routes(state.clone());

    let app = Router::new()
        .route("/health", get(health))
        .with_state(state.clone())
        .merge(api_routes.clone())
        .nest("/api", api_routes)
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

async fn init_flight_recorder() -> Result<Arc<Mutex<DuckDbConnection>>, duckdb::Error> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root_dir = manifest_dir
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .expect("failed to resolve repo root");
    let data_dir = root_dir.join("data");
    // Flight recorder gets its own file
    let fr_db_path = data_dir.join("flight_recorder.db");

    let conn = DuckDbConnection::open(&fr_db_path)?;

    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS events (
            timestamp DATETIME DEFAULT current_timestamp,
            event_type TEXT NOT NULL,
            job_id TEXT,
            workflow_id TEXT,
            payload JSON
        );
    "#,
    )?;

    tracing::info!(target: "handshake_core", db_path = %fr_db_path.display(), "flight recorder ready");

    Ok(Arc::new(Mutex::new(conn)))
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

    let response = build_health_response(db_status);
    tracing::info!(
        target: "handshake_core",
        route = "/health",
        status = response.status,
        db_status = db_status,
        "health check"
    );

    Json(response)
}

fn build_health_response(db_status: &str) -> HealthResponse {
    let overall_status = if db_status == "ok" { "ok" } else { "error" };

    HealthResponse {
        status: overall_status.to_string(),
        component: "handshake_core",
        version: env!("CARGO_PKG_VERSION"),
        db_status: db_status.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::build_health_response;

    #[test]
    fn health_response_ok_sets_status_ok() {
        let response = build_health_response("ok");
        assert_eq!(response.status, "ok");
        assert_eq!(response.component, "handshake_core");
        assert_eq!(response.db_status, "ok");
    }

    #[test]
    fn health_response_error_maps_to_overall_error() {
        let response = build_health_response("error");
        assert_eq!(response.status, "error");
        assert_eq!(response.db_status, "error");
    }
}

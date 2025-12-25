use axum::{extract::State, routing::get, Json, Router};
use duckdb::Connection as DuckDbConnection;
use handshake_core::{
    api,
    capabilities::CapabilityRegistry,
    llm::{LLMClient, OllamaClient},
    logging,
    models::HealthResponse,
    storage::{
        retention::{Janitor, JanitorConfig},
        sqlite::SqliteDatabase,
        Database,
    },
    AppState,
};
use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        tracing::error!(target: "handshake_core", error = %err, "handshake_core failed to start");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let addr: SocketAddr = ([127, 0, 0, 1], 37501).into();

    logging::init_logging();

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let (storage, sqlite_pool) = init_storage().await?;
    let fr_pool = init_flight_recorder().await?;
    let llm_client = init_llm_client()?;
    let capability_registry = Arc::new(CapabilityRegistry::new_default());

    let state = AppState {
        storage,
        fr_pool: fr_pool.clone(),
        llm_client,
        capability_registry,
    };

    // Start Janitor background service [§2.3.11]
    // Configuration via environment or defaults
    let janitor_config = init_janitor_config();
    let janitor = Arc::new(Janitor::new(sqlite_pool, fr_pool, janitor_config));
    let _janitor_handle = janitor.spawn_background();

    let api_routes = api::routes(state.clone());

    let app = Router::new()
        .route("/health", get(health))
        .with_state(state.clone())
        .merge(api_routes.clone())
        .nest("/api", api_routes)
        .layer(cors);

    tracing::info!(target: "handshake_core", listen_addr = %addr, "handshake_core started");

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

/// Initialize Janitor configuration from environment variables.
///
/// Environment variables:
/// - `JANITOR_DRY_RUN`: Set to "true" to enable dry-run mode (default: false)
/// - `JANITOR_INTERVAL_SECS`: Prune interval in seconds (default: 3600)
/// - `JANITOR_RETENTION_DAYS`: Days to retain AI job results (default: 30)
fn init_janitor_config() -> JanitorConfig {
    use handshake_core::storage::retention::RetentionPolicy;

    let dry_run = std::env::var("JANITOR_DRY_RUN")
        .map(|v| v.to_lowercase() == "true")
        .unwrap_or(false);

    let interval_secs = std::env::var("JANITOR_INTERVAL_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3600);

    let retention_days = std::env::var("JANITOR_RETENTION_DAYS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);

    let policy = RetentionPolicy {
        kind: handshake_core::storage::retention::ArtifactKind::Result,
        window_days: retention_days,
        min_versions: 3,
    };

    tracing::info!(
        target: "handshake_core::janitor",
        dry_run,
        interval_secs,
        retention_days,
        "Janitor config initialized"
    );

    JanitorConfig {
        policies: vec![policy],
        dry_run,
        interval_secs,
        batch_size: 1000,
    }
}

async fn init_storage() -> Result<(Arc<dyn Database>, sqlx::SqlitePool), Box<dyn std::error::Error>>
{
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root_dir = manifest_dir
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or("failed to resolve repo root")?;
    let data_dir = root_dir.join("data");
    if !data_dir.exists() {
        std::fs::create_dir_all(&data_dir)?;
    }

    let db_path = data_dir.join("handshake.db");
    let db_url = format!("sqlite://{}", db_path.to_string_lossy());

    let sqlite = SqliteDatabase::connect(&db_url, 5).await?;
    sqlite.run_migrations().await?;

    // Clone pool before converting to Arc<dyn Database> for Janitor service
    let pool = sqlite.pool().clone();

    tracing::info!(target: "handshake_core", db_url = %db_url, "database ready");
    Ok((sqlite.into_arc(), pool))
}

async fn init_flight_recorder() -> Result<Arc<Mutex<DuckDbConnection>>, Box<dyn std::error::Error>>
{
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root_dir = manifest_dir
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or("failed to resolve repo root")?;
    let data_dir = root_dir.join("data");
    if !data_dir.exists() {
        std::fs::create_dir_all(&data_dir)?;
    }
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

fn init_llm_client() -> Result<Arc<dyn LLMClient>, Box<dyn std::error::Error>> {
    let url = std::env::var("OLLAMA_URL")
        .map_err(|_| "OLLAMA_URL not configured; LLM client cannot be initialized")?;
    let model = match std::env::var("OLLAMA_MODEL") {
        Ok(val) => val,
        Err(_) => "llama3".to_string(),
    };
    tracing::info!(target: "handshake_core", url = %url, model = %model, "using Ollama LLM client");
    Ok(Arc::new(OllamaClient::new(url, model)))
}

async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    let db_status = match state.storage.ping().await {
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

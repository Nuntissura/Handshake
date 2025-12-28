use axum::{extract::State, routing::get, Json, Router};
use handshake_core::{
    api,
    capabilities::CapabilityRegistry,
    diagnostics::DiagnosticsStore,
    flight_recorder::{duckdb::DuckDbFlightRecorder, FlightRecorder},
    llm::{ollama::OllamaAdapter, LlmClient},
    logging,
    models::HealthResponse,
    storage::{
        self,
        retention::{Janitor, JanitorConfig},
    },
    workflows, AppState,
};
use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
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

    let storage = storage::init_storage()
        .await
        .map_err(|e| format!("failed to initialize storage: {}", e))?;
    let recorder = init_flight_recorder().await?;
    let flight_recorder: Arc<dyn FlightRecorder> = recorder.clone();
    let diagnostics: Arc<dyn DiagnosticsStore> = recorder.clone();
    let llm_client = init_llm_client(flight_recorder.clone())?;
    let capability_registry = Arc::new(CapabilityRegistry::new());

    let state = AppState {
        storage: storage.clone(),
        flight_recorder: flight_recorder.clone(),
        diagnostics,
        llm_client,
        capability_registry,
    };

    // [HSK-WF-003] Startup Recovery Loop
    // Scan for and mark 'Running' workflows > 30s old as 'Stalled'.
    // Executed non-blockingly but initiated before server start.
    let recovery_state = state.clone();
    tokio::spawn(async move {
        tracing::info!(target: "handshake_core::recovery", "Starting boot-time workflow recovery scan...");
        match workflows::mark_stalled_workflows(&recovery_state, 30, true).await {
            Ok(recovered) => {
                if !recovered.is_empty() {
                    tracing::info!(target: "handshake_core::recovery", count = recovered.len(), "Workflow recovery complete");
                } else {
                    tracing::info!(target: "handshake_core::recovery", "No workflows required recovery");
                }
            }
            Err(err) => {
                tracing::error!(target: "handshake_core::recovery", error = %err, "Workflow recovery failed");
            }
        }
    });

    // Start Janitor background service [§2.3.11]
    // Configuration via environment or defaults
    let janitor_config = init_janitor_config();
    let janitor = Arc::new(Janitor::new(
        storage,
        flight_recorder.clone(),
        janitor_config,
    ));
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
    use handshake_core::storage::{ArtifactKind, RetentionPolicy};

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
        kind: ArtifactKind::Result,
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

async fn init_flight_recorder() -> Result<Arc<DuckDbFlightRecorder>, Box<dyn std::error::Error>> {
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

    let recorder = DuckDbFlightRecorder::new_on_path(&fr_db_path, 7)?;
    tracing::info!(target: "handshake_core", db_path = %fr_db_path.display(), "flight recorder ready");

    Ok(Arc::new(recorder))
}

fn init_llm_client(
    flight_recorder: Arc<dyn FlightRecorder>,
) -> Result<Arc<dyn LlmClient>, Box<dyn std::error::Error>> {
    let url = std::env::var("OLLAMA_URL")
        .map_err(|_| "OLLAMA_URL not configured; LLM client cannot be initialized")?;
    let model = match std::env::var("OLLAMA_MODEL") {
        Ok(val) => val,
        Err(_) => "llama3".to_string(),
    };
    tracing::info!(target: "handshake_core", url = %url, model = %model, "using Ollama LLM adapter");
    Ok(Arc::new(OllamaAdapter::new(
        url,
        model,
        8192,
        flight_recorder,
    )))
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

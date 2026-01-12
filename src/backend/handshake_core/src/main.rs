use axum::{extract::State, routing::get, Json, Router};
use handshake_core::{
    api,
    capabilities::CapabilityRegistry,
    diagnostics::DiagnosticsStore,
    flight_recorder::{duckdb::DuckDbFlightRecorder, FlightRecorder},
    llm::{ollama::OllamaAdapter, DisabledLlmClient, LlmClient},
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
    time::Duration,
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

    let storage = storage::init_storage().await?;
    let recorder = init_flight_recorder().await?;
    let flight_recorder: Arc<dyn FlightRecorder> = recorder.clone();
    let diagnostics: Arc<dyn DiagnosticsStore> = recorder.clone();
    let llm_client = init_llm_client(flight_recorder.clone()).await;
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
    workflows::enable_startup_recovery_gate();
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
                workflows::mark_startup_recovery_complete();
            }
            Err(err) => {
                tracing::error!(target: "handshake_core::recovery", error = %err, "Workflow recovery failed");
                workflows::mark_startup_recovery_failed(err.to_string());
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

async fn init_llm_client(flight_recorder: Arc<dyn FlightRecorder>) -> Arc<dyn LlmClient> {
    let base_url =
        std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".to_string());
    let base_url = base_url.trim_end_matches('/').to_string();

    let model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama3".to_string());

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
    {
        Ok(client) => client,
        Err(err) => {
            let reason = format!("Ollama detection client init failed: {err}");
            tracing::warn!(
                target: "handshake_core::llm",
                error = %reason,
                base_url = %base_url,
                model = %model,
                "Ollama disabled (cannot build HTTP client)"
            );
            return Arc::new(DisabledLlmClient::new(model, reason));
        }
    };

    let tags_url = format!("{}/api/tags", base_url);
    match client.get(&tags_url).send().await {
        Ok(resp) if resp.status().is_success() => {
            tracing::info!(
                target: "handshake_core::llm",
                url = %base_url,
                model = %model,
                "Ollama available; enabling Ollama LLM adapter"
            );
            Arc::new(OllamaAdapter::new(base_url, model, 8192, flight_recorder))
        }
        Ok(resp) => {
            let reason = format!(
                "Ollama detection failed: GET {tags_url} returned {}",
                resp.status()
            );
            tracing::warn!(
                target: "handshake_core::llm",
                error = %reason,
                url = %base_url,
                model = %model,
                "Ollama disabled (tags endpoint not healthy)"
            );
            Arc::new(DisabledLlmClient::new(model, reason))
        }
        Err(err) => {
            let reason = format!("Ollama detection failed: GET {tags_url} error: {err}");
            tracing::warn!(
                target: "handshake_core::llm",
                error = %reason,
                url = %base_url,
                model = %model,
                "Ollama disabled (tags endpoint unreachable)"
            );
            Arc::new(DisabledLlmClient::new(model, reason))
        }
    }
}

async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    let (db_status, migration_version) = match state.storage.ping().await {
        Ok(_) => match state.storage.migration_version().await {
            Ok(version) => ("ok", Some(version)),
            Err(err) => {
                tracing::error!(target: "handshake_core", route = "/health", error = %err, "db migration version check error");
                ("error", None)
            }
        },
        Err(err) => {
            tracing::error!(target: "handshake_core", route = "/health", error = %err, "db check error");
            ("error", None)
        }
    };

    let response = build_health_response(db_status, migration_version);
    tracing::info!(
        target: "handshake_core",
        route = "/health",
        status = response.status,
        db_status = db_status,
        "health check"
    );

    Json(response)
}

fn build_health_response(db_status: &str, migration_version: Option<i64>) -> HealthResponse {
    let overall_status = if db_status == "ok" { "ok" } else { "error" };

    HealthResponse {
        status: overall_status.to_string(),
        component: "handshake_core",
        version: env!("CARGO_PKG_VERSION"),
        db_status: db_status.to_string(),
        migration_version,
    }
}

#[cfg(test)]
mod tests {
    use super::build_health_response;

    #[test]
    fn health_response_ok_sets_status_ok() {
        let response = build_health_response("ok", Some(9));
        assert_eq!(response.status, "ok");
        assert_eq!(response.component, "handshake_core");
        assert_eq!(response.db_status, "ok");
        assert_eq!(response.migration_version, Some(9));
    }

    #[test]
    fn health_response_error_maps_to_overall_error() {
        let response = build_health_response("error", None);
        assert_eq!(response.status, "error");
        assert_eq!(response.db_status, "error");
        assert_eq!(response.migration_version, None);
    }
}

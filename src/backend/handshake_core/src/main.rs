use axum::{Json, Router, extract::State, routing::get};
use handshake_core::{
    AppState, api,
    capabilities::CapabilityRegistry,
    diagnostics::DiagnosticsStore,
    flight_recorder::{FlightRecorder, duckdb::DuckDbFlightRecorder},
    llm::{
        DisabledLlmClient, LlmClient, ModelTier,
        guard::CloudEscalationGuard,
        ollama::OllamaAdapter,
        openai_compat::{ApiKey, OpenAiCompatAdapter},
        registry::{ProviderKind, ProviderRegistry, RuntimeRole},
    },
    logging,
    models::HealthResponse,
    process_ledger::restart_resume::PostgresRestartResumeRunner,
    storage::{
        self,
        retention::{Janitor, JanitorConfig},
    },
    workflows,
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

    // Managed PostgreSQL lifecycle (task #9): start (or adopt) Handshake's own
    // hidden cluster BEFORE storage init, so no operator has to launch Postgres
    // manually and no console window pops. Idempotent — an already-running
    // cluster on the configured port is adopted, never double-started.
    let managed_pg = handshake_core::managed_postgres::ManagedPostgres::ensure_running(
        handshake_core::managed_postgres::ManagedPostgresConfig::from_env(),
    )
    .await?;
    if managed_pg.is_enabled() && std::env::var(storage::DATABASE_URL_ENV).is_err() {
        std::env::set_var(storage::DATABASE_URL_ENV, managed_pg.database_url());
        tracing::info!(
            target: "handshake_core::managed_postgres",
            "DATABASE_URL resolved from the managed cluster"
        );
    }

    let storage_config = storage::ControlPlaneStorageConfig::from_env()?;
    tracing::info!(
        target: "handshake_core",
        storage_mode = %storage_config.mode,
        "control-plane storage mode resolved"
    );
    let control_plane = storage::init_control_plane_storage_with_config(&storage_config).await?;
    let restart_report = PostgresRestartResumeRunner::new(control_plane.postgres_pool.clone())
        .run()
        .await?;
    tracing::info!(
        target: "handshake_core::restart_resume",
        report_id = %restart_report.report_id,
        sessions_examined = restart_report.sessions_examined,
        sessions_resumed = restart_report.sessions_resumed.len(),
        sessions_recovery_failed = restart_report.sessions_recovery_failed.len(),
        "startup restart-resume pass completed"
    );
    if startup_recovery_only_requested() {
        write_startup_recovery_report(&restart_report)?;
        return Ok(());
    }
    let storage = control_plane.database.clone();
    let recorder = init_flight_recorder().await?;
    let flight_recorder: Arc<dyn FlightRecorder> = recorder.clone();
    let diagnostics: Arc<dyn DiagnosticsStore> = recorder.clone();
    let llm_client = init_llm_client(flight_recorder.clone()).await;
    let capability_registry = Arc::new(CapabilityRegistry::new());
    let session_registry = Arc::new(workflows::SessionRegistry::new(
        workflows::SessionSchedulerConfig::from_env(),
    ));

    let state = AppState {
        storage: storage.clone(),
        flight_recorder: flight_recorder.clone(),
        diagnostics,
        llm_client,
        capability_registry,
        session_registry,
        postgres_pool: control_plane.postgres_pool.clone(),
    };

    // Bootstrap the WP-KERNEL-005 atelier schema (idempotent, advisory-locked)
    // on the shared pool so the atelier HTTP surface is queryable from startup.
    {
        let atelier = handshake_core::atelier::AtelierStore::with_observability(
            control_plane.postgres_pool.clone(),
            storage.clone(),
            flight_recorder.clone(),
        );
        if let Err(err) = atelier.ensure_schema().await {
            tracing::error!(target: "handshake_core::atelier", error = %err, "atelier ensure_schema failed at startup");
            return Err(Box::new(err));
        }
        tracing::info!(target: "handshake_core::atelier", "atelier schema ensured");

        // MT-206: project the FULL builtin CKC command corpus into the action
        // catalog (cross-checked live against the ModelManual) so the Dev
        // Command Center `/atelier/command-corpus` projection serves the full
        // enumeration from boot. Idempotent; the catalog is a rebuildable
        // projection, so a bootstrap failure is logged loudly but does not
        // abort startup.
        match atelier.bootstrap_builtin_command_corpus().await {
            Ok(receipt) => tracing::info!(
                target: "handshake_core::atelier",
                total_commands = receipt.total_commands,
                covered_count = receipt.covered_count,
                blocked_count = receipt.blocked_count,
                "builtin command corpus bootstrapped"
            ),
            Err(err) => tracing::error!(
                target: "handshake_core::atelier",
                error = %err,
                "builtin command corpus bootstrap failed at startup"
            ),
        }
    }

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
    let serve_result = axum::serve(listener, app).await;

    // Best-effort teardown when the serve loop ends: stop the cluster only if
    // Handshake started it (adopted/external clusters are left untouched).
    if let Err(err) = managed_pg.stop().await {
        tracing::warn!(target: "handshake_core::managed_postgres", error = %err, "managed PostgreSQL stop failed at shutdown");
    }
    serve_result?;
    Ok(())
}

fn startup_recovery_only_requested() -> bool {
    std::env::var("HANDSHAKE_STARTUP_RECOVERY_ONLY")
        .ok()
        .as_deref()
        == Some("1")
}

fn write_startup_recovery_report(
    report: &handshake_core::session_checkpoint::ResumeReport,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(path) = std::env::var_os("HANDSHAKE_STARTUP_RECOVERY_REPORT_FILE").map(PathBuf::from)
    else {
        return Ok(());
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let payload = serde_json::json!({
        "report_id": report.report_id,
        "sessions_examined": report.sessions_examined,
        "sessions_resumed": report.sessions_resumed.len(),
        "sessions_recovery_failed": report.sessions_recovery_failed.len(),
        "fr_events_emitted": report.fr_events_emitted,
    });
    std::fs::write(path, serde_json::to_vec_pretty(&payload)?)?;
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
    let registry = match ProviderRegistry::from_env() {
        Ok(registry) => registry,
        Err(err) => {
            let reason = format!("LLM registry init failed: {err}");
            tracing::warn!(
                target: "handshake_core::llm",
                error = %reason,
                "LLM disabled (cannot load ProviderRegistry)"
            );
            return Arc::new(DisabledLlmClient::new("unknown".to_string(), reason));
        }
    };

    let resolved = match registry.resolve(RuntimeRole::Orchestrator) {
        Ok(resolved) => resolved,
        Err(err) => {
            let reason = format!("LLM provider resolution failed: {err}");
            tracing::warn!(
                target: "handshake_core::llm",
                error = %reason,
                "LLM disabled (cannot resolve provider)"
            );
            return Arc::new(DisabledLlmClient::new("unknown".to_string(), reason));
        }
    };

    let client: Arc<dyn LlmClient> = match resolved.kind {
        ProviderKind::Ollama => {
            let detection_client = match reqwest::Client::builder()
                .timeout(Duration::from_secs(2))
                .build()
            {
                Ok(client) => client,
                Err(err) => {
                    let reason = format!("Ollama detection client init failed: {err}");
                    tracing::warn!(
                        target: "handshake_core::llm",
                        error = %reason,
                        base_url = %resolved.base_url,
                        model = %resolved.model_id,
                        "Ollama disabled (cannot build HTTP client)"
                    );
                    return Arc::new(DisabledLlmClient::new(resolved.model_id, reason));
                }
            };

            let tags_url = format!("{}/api/tags", resolved.base_url);
            match detection_client.get(&tags_url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    tracing::info!(
                        target: "handshake_core::llm",
                        url = %resolved.base_url,
                        model = %resolved.model_id,
                        "Ollama available; enabling Ollama LLM adapter"
                    );
                    Arc::new(OllamaAdapter::new(
                        resolved.base_url,
                        resolved.model_id,
                        8192,
                        flight_recorder,
                    ))
                }
                Ok(resp) => {
                    let reason = format!(
                        "Ollama detection failed: GET {tags_url} returned {}",
                        resp.status()
                    );
                    tracing::warn!(
                        target: "handshake_core::llm",
                        error = %reason,
                        url = %resolved.base_url,
                        model = %resolved.model_id,
                        "Ollama disabled (tags endpoint not healthy)"
                    );
                    Arc::new(DisabledLlmClient::new(resolved.model_id, reason))
                }
                Err(err) => {
                    let reason = format!("Ollama detection failed: GET {tags_url} error: {err}");
                    tracing::warn!(
                        target: "handshake_core::llm",
                        error = %reason,
                        url = %resolved.base_url,
                        model = %resolved.model_id,
                        "Ollama disabled (tags endpoint unreachable)"
                    );
                    Arc::new(DisabledLlmClient::new(resolved.model_id, reason))
                }
            }
        }
        ProviderKind::OpenAiCompat => {
            let api_key = resolved
                .api_key_env
                .as_deref()
                .and_then(ApiKey::from_env)
                .or_else(|| ApiKey::from_env("OPENAI_API_KEY"));

            Arc::new(OpenAiCompatAdapter::new(
                resolved.base_url,
                resolved.model_id,
                8192,
                resolved.tier,
                api_key,
                flight_recorder,
            ))
        }
    };

    if client.profile().model_tier == ModelTier::Cloud {
        match CloudEscalationGuard::from_env(client) {
            Ok(guarded) => Arc::new(guarded),
            Err(err) => {
                let reason = format!("CloudEscalationGuard init failed: {err}");
                tracing::warn!(
                    target: "handshake_core::llm",
                    error = %reason,
                    "LLM disabled (cloud guard init failed)"
                );
                Arc::new(DisabledLlmClient::new("unknown".to_string(), reason))
            }
        }
    } else {
        client
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

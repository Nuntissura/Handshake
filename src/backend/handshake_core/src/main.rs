use axum::{extract::State, routing::get, Json, Router};
use handshake_core::{
    api,
    capabilities::CapabilityRegistry,
    diagnostics::DiagnosticsStore,
    flight_recorder::{duckdb::DuckDbFlightRecorder, FlightRecorder},
    llm::{boot::resolve_default_llm_client, LlmClient},
    logging,
    model_runtime::{ModelRegistryStore, ScopedModelRegistryAuthority},
    models::HealthResponse,
    process_ledger::{
        restart_resume::{
            BoundedRestartResumeOutcome, SurrealRestartResumeRunner,
            RESTART_RESUME_BOOT_TIMEOUT_DEFAULT,
        },
        LedgerBatcher, ProcessReclaimRuntime, ReclaimResourceScope,
    },
    storage::{
        retention::{Janitor, JanitorConfig},
        surreal::{
            bootstrap_production_schemas, SurrealDatabase, SurrealStorage, SurrealStorageConfig,
        },
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

const HTTP_CONNECTION_DRAIN_TIMEOUT: Duration = Duration::from_secs(30);
const MODEL_LANE_BOOT_RECOVERY_TIMEOUT: Duration = Duration::from_secs(30);
/// Environment override (milliseconds) for the hard restart-resume boot bound.
/// Absent/blank/unparseable falls back to [`RESTART_RESUME_BOOT_TIMEOUT_DEFAULT`]
/// (30s), consistent with the two sibling boot bounds above.
const RESTART_RESUME_BOOT_TIMEOUT_MS_ENV: &str = "HANDSHAKE_RESTART_RESUME_BOOT_TIMEOUT_MS";

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

    // The desktop host owns the persisted product-local identity. Parse its
    // strict five-dimensional handoff before opening account-facing services;
    // a missing or corrupt scope aborts boot instead of falling back to caller
    // headers or an unscoped node-global view.
    let product_local_scope = api::account_scope::ProductLocalResourceScope::from_env()?;
    std::env::remove_var(api::account_scope::PRODUCT_LOCAL_RESOURCE_SCOPE_ENV);
    let exact_scope = product_local_scope.exact();
    let reclaim_resource_scope = ReclaimResourceScope::try_from_stored(
        &exact_scope.owner_account_id.to_string(),
        &exact_scope.actor_principal_id.to_string(),
        &exact_scope.authenticated_session_id.to_string(),
        exact_scope.workspace_id.as_str(),
        &exact_scope.access_space_id.to_string(),
    )?;

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let surreal_storage = SurrealStorage::open(SurrealStorageConfig::from_env()?).await?;
    bootstrap_production_schemas(&surreal_storage).await?;
    let shared_database = Arc::new(SurrealDatabase::new(surreal_storage.clone()));
    // The restart-resume boot pass is bounded by a hard outer wall clock,
    // consistent with the sibling ModelLane boot recovery below and the
    // process-ledger boot reconcile (`time::timeout(startup_timeout)`). A single
    // resumable candidate whose orphan-reclaim UPDATE blocks must not stall boot
    // forever. On timeout the pass fails closed (no candidate is falsely marked
    // resumed), records a durable bounded-abort report, and boot continues so the
    // staleness reclaim task started later can finish reconciling; it never
    // panics or hangs.
    let restart_outcome =
        SurrealRestartResumeRunner::open(surreal_storage.clone(), reclaim_resource_scope.clone())
            .await?
            .run_with_bound(restart_resume_boot_timeout())
            .await?;
    let restart_report = match restart_outcome {
        BoundedRestartResumeOutcome::Completed(report) => {
            tracing::info!(
                target: "handshake_core::restart_resume",
                report_id = %report.report_id,
                sessions_examined = report.sessions_examined,
                sessions_resumed = report.sessions_resumed.len(),
                sessions_recovery_failed = report.sessions_recovery_failed.len(),
                "startup restart-resume pass completed"
            );
            report
        }
        BoundedRestartResumeOutcome::TimedOut {
            timeout,
            report,
            evidence_persisted,
        } => {
            tracing::error!(
                target: "handshake_core::restart_resume",
                report_id = %report.report_id,
                timeout_ms = timeout.as_millis() as u64,
                evidence_persisted,
                "startup restart-resume pass exceeded its hard wall-clock bound; it fails closed (no session falsely resumed) and boot continues so the staleness reclaim task and the next boot pass finish reconciling the still-open resumable sessions"
            );
            report
        }
    };
    let recovered_model_lane_runs = recover_model_lanes_at_core_boot_with_timeout(
        surreal_storage.clone(),
        product_local_scope.resource_scope(),
        MODEL_LANE_BOOT_RECOVERY_TIMEOUT,
    )
    .await?;
    tracing::info!(
        target: "handshake_core::model_lane_recovery",
        recovered_runs = recovered_model_lane_runs,
        "bounded core-owned ModelLane boot recovery completed"
    );
    let startup_recovery_only = startup_recovery_only_requested();

    // Process liveness is host-local even though lifecycle authority is stored
    // in embedded SurrealDB. Require the explicit host identity; never derive it
    // from a relational endpoint or fall back to an unscoped node identity.
    let runtime_host_scope_id =
        handshake_core::process_ledger::resolve_embedded_runtime_host_scope()?;
    if startup_recovery_only {
        // The production Surreal runtime performs one bounded restart-orphan
        // reconcile before starting its periodic task. Recovery-only boot drains
        // that task and its retained ledger writer before any storage teardown.
        let recovery_runtime = ProcessReclaimRuntime::production(
            surreal_storage.clone(),
            reclaim_resource_scope.clone(),
            None,
            handshake_core::process_ledger::production_process_sandbox_registry_async().await?,
            runtime_host_scope_id.clone(),
            Duration::from_secs(30),
        )
        .await?;
        let report = recovery_runtime.boot_reconcile_report();
        tracing::info!(
            target: "handshake_core::process_ledger",
            sessions_reconciled = report.sessions_reconciled,
            processes_reclaimed = report.processes_reclaimed,
            processes_kill_failed = report.processes_kill_failed,
            sweep_reclaim_errors = ?report.sweep_reclaim_errors,
            runtime_host_scope_id,
            "startup recovery-only restart-orphan reconcile complete"
        );
        let drain = recovery_runtime
            .shutdown_and_drain(Duration::from_secs(10))
            .await;
        tracing::info!(
            target: "handshake_core::process_ledger",
            reclaim_task_quiesced = drain.reclaim_task_quiesced,
            ledger = ?drain.ledger,
            lease_released = drain.lease_released,
            lease_retained_reason = ?drain.lease_retained_reason,
            "startup recovery-only process runtime drained"
        );
        let ledger_flushed = matches!(
            drain.ledger,
            handshake_core::process_ledger::LedgerDrainJoinOutcome::Flushed
                | handshake_core::process_ledger::LedgerDrainJoinOutcome::AlreadyDrained
        );
        if !drain.reclaim_task_quiesced || !ledger_flushed || !drain.lease_released {
            return Err(std::io::Error::other(
                "startup recovery-only runtime did not prove a complete durable drain",
            )
            .into());
        }
        let report_result = write_startup_recovery_report(&restart_report);
        report_result?;
        drop(recovery_runtime);
        drop(shared_database);
        surreal_storage.shutdown().await?;
        return Ok(());
    }

    let recorder = init_flight_recorder().await?;
    let flight_recorder: Arc<dyn FlightRecorder> = recorder.clone();
    let diagnostics: Arc<dyn DiagnosticsStore> = recorder.clone();

    // WP-KERNEL-009 MT-195/MT-196: seed (or hash-resync) the built-in
    // UserManual corpus so the no-context manual surface is queryable from
    // boot. Idempotent (content-hash short-circuit; receipts only for changed
    // rows). A seed failure is logged loudly but does not abort startup — the
    // freshness route reports `unseeded_version` and the gated
    // POST /usermanual/resync recovers.
    {
        let manual_store =
            handshake_core::user_manual::store::UserManualStore::new(surreal_storage.clone());
        match manual_store.ensure_seeded().await {
            Ok(report) => tracing::info!(
                target: "handshake_core::user_manual",
                manual_version = %report.manual_version,
                pages_total = report.pages_total,
                pages_changed = report.pages_changed,
                tools_total = report.tools_total,
                "UserManual corpus ensured"
            ),
            Err(err) => tracing::error!(
                target: "handshake_core::user_manual",
                error = %err,
                "UserManual seed failed at startup (POST /usermanual/resync to recover)"
            ),
        }
    }

    // Bind the public socket before opening model artifacts. A port conflict is
    // a normal fallible startup gate; resolving it first guarantees that bind
    // failure cannot strand a loaded runtime or an undrained START row.
    let listener = TcpListener::bind(addr).await?;

    // Install OS-signal observation before any model artifact can be opened or
    // any durable START acknowledgement can block. The shared START budget in
    // `llm::boot` bounds the remaining model-init interval; once it returns,
    // an already-received signal immediately drives the normal quiescence path.
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    let signal_task = tokio::spawn(async move {
        shutdown_signal().await;
        let _ = shutdown_tx.send(true);
    });

    // One process-wide composition owns lifecycle writes, sandbox reclaim,
    // boot reconciliation, and the liveness lease for both embedded inference
    // and operator-chat/Official-CLI work.
    let process_runtime = ProcessReclaimRuntime::production(
        surreal_storage.clone(),
        reclaim_resource_scope.clone(),
        None,
        handshake_core::process_ledger::production_process_sandbox_registry_async().await?,
        runtime_host_scope_id,
        Duration::from_secs(30),
    )
    .await?;
    let runtime_instance = Some(process_runtime.runtime_instance().clone());
    let model_registry_authority = ScopedModelRegistryAuthority::new(
        ModelRegistryStore::new(surreal_storage.clone()),
        exact_scope.clone(),
    );
    let llm_client = init_llm_client(
        flight_recorder.clone(),
        Some(process_runtime.ledger().ledger()),
        Some(model_registry_authority),
        runtime_instance,
    )
    .await;
    let capability_registry = Arc::new(CapabilityRegistry::new());
    let session_registry = Arc::new(workflows::SessionRegistry::new(
        workflows::SessionSchedulerConfig::from_env(),
    ));

    let state = AppState {
        storage: shared_database.clone(),
        surreal_storage: surreal_storage.clone(),
        flight_recorder: flight_recorder.clone(),
        diagnostics,
        llm_client,
        capability_registry,
        session_registry,
    };

    // [HSK-WF-003] Startup Recovery Loop
    // Scan for and mark 'Running' workflows > 30s old as 'Stalled'.
    // Executed non-blockingly but initiated before server start.
    workflows::enable_startup_recovery_gate();
    let recovery_state = state.clone();
    let recovery_handle = tokio::spawn(async move {
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
        shared_database,
        flight_recorder.clone(),
        janitor_config,
    ));
    let janitor_handle = janitor.clone().spawn_background();

    let api::ApiRoutes {
        router: api_routes,
        runtime: api_runtime,
    } = api::routes_with_process_reclaim_runtime(
        state.clone(),
        process_runtime,
        product_local_scope,
    );
    let app = Router::new()
        .route("/health", get(health))
        .with_state(state.clone())
        .merge(api_routes.clone())
        .nest("/api", api_routes)
        .layer(cors);

    tracing::info!(target: "handshake_core", listen_addr = %addr, "handshake_core started");

    // WP-1 MT-013 (F1 graceful shutdown): stop accepting after the OS signal and
    // give already-accepted connections a bounded drain window. Dropping only
    // Axum's outer serve future does not prove its spawned connection tasks have
    // ended, so the timeout path still drives the model-runtime quiescence
    // barrier below and exits the process after cleanup.
    let server_shutdown_rx = shutdown_rx.clone();
    let deadline_shutdown_rx = shutdown_rx;
    let mut server = Box::pin(std::future::IntoFuture::into_future(
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(async move {
            wait_for_shutdown_request(server_shutdown_rx).await;
        }),
    ));
    let connection_drain_deadline = async move {
        wait_for_shutdown_request(deadline_shutdown_rx).await;
        tokio::time::sleep(HTTP_CONNECTION_DRAIN_TIMEOUT).await;
    };
    tokio::pin!(connection_drain_deadline);
    let (serve_result, connection_drain_timed_out) = tokio::select! {
        result = server.as_mut() => (Some(result), false),
        _ = &mut connection_drain_deadline => (None, true),
    };
    // Release the Axum service graph before draining its AppState-owned runtime
    // and storage clones. On the deadline branch, accepted connection tasks are
    // still handled by the explicit fail-closed process-exit path below.
    drop(server);
    signal_task.abort();
    let _ = signal_task.await;

    // The one-shot recovery scan owns an AppState clone, and therefore an LLM
    // runtime owner. Cancel and join it after Axum returns or reaches its
    // bounded connection deadline. On the deadline path the still-live Axum
    // connection owners force the explicit no-STOP process-exit branch below.
    // Stop the janitor before draining durable runtime infrastructure.
    recovery_handle.abort();
    let _ = recovery_handle.await;
    janitor_handle.abort();
    let _ = janitor_handle.await;

    // A timed-out Axum drain leaves spawned connection tasks alive with
    // AppState clones. Pre-abandon STOP authority before quiescing so even a
    // successful worker barrier cannot publish a false clean lifecycle while
    // those accepted tasks still exist.
    if connection_drain_timed_out {
        tracing::error!(
            target: "handshake_core",
            timeout_seconds = HTTP_CONNECTION_DRAIN_TIMEOUT.as_secs(),
            "accepted HTTP connections missed the drain deadline; embedded START will remain open until process-death reconciliation"
        );
        state.llm_client.leave_open_for_reconciliation();
    }

    // 1) Close runtime admission, cancel active workers, and wait for the
    //    worker-owned quiescence guards. Only a proven idle result may emit the
    //    embedded ProcessOwnershipLedger STOP. On failure the client releases
    //    its reserved STOP permits without emitting rows, leaving START open
    //    for next-boot reconciliation.
    let runtime_shutdown = if connection_drain_timed_out {
        Err(handshake_core::llm::LlmError::ProviderError(
            "accepted HTTP connections still own AppState; runtime unload ownership is unproven"
                .to_string(),
        ))
    } else {
        state.llm_client.shutdown_gracefully().await
    };
    if let Err(err) = &runtime_shutdown {
        tracing::error!(
            target: "handshake_core::process_ledger",
            error = %err,
            "embedded runtime did not prove quiescence; STOP remains open and shutdown will exit with the OS liveness lease held"
        );
    }

    // Retain the final runtime-owning AppState on failed quiescence or an Axum
    // connection-drain timeout. Only the fully-drained + quiesced path can drop
    // the final owner before lease release. The Option keeps both paths explicit
    // without an accidental Drop during error unwinding.
    let mut state_owner = Some(state);

    // 2) Quiesce the single reclaimer before closing and draining the single
    // process-ledger writer. The shared runtime releases its OS lease only after
    // both boundaries are proven.
    let api_drain_report = api_runtime
        .drain_and_join(std::time::Duration::from_secs(35))
        .await;
    tracing::info!(
        target: "handshake_core::process_ledger",
        outcome = ?api_drain_report.operator_chat_process_ledger,
        swarm_events_flushed = api_drain_report.swarm_events_flushed,
        "operator-chat process ledger drain-and-join at shutdown"
    );

    let operator_chat_ledger_flushed = matches!(
        &api_drain_report.operator_chat_process_ledger,
        handshake_core::process_ledger::LedgerDrainJoinOutcome::Flushed
            | handshake_core::process_ledger::LedgerDrainJoinOutcome::AlreadyDrained
    );

    if connection_drain_timed_out
        || runtime_shutdown.is_err()
        || !api_drain_report.process_reclaim_quiesced
        || !api_drain_report.swarm_events_flushed
        || !operator_chat_ledger_flushed
    {
        if let Err(err) = &runtime_shutdown {
            tracing::error!(
                target: "handshake_core::process_ledger",
                error = %err,
                "runtime shutdown did not reach a graceful terminal state"
            );
        }
        tracing::error!(
            target: "handshake_core::process_ledger",
            connection_drain_timed_out,
            process_reclaim_quiesced = api_drain_report.process_reclaim_quiesced,
            swarm_events_flushed = api_drain_report.swarm_events_flushed,
            operator_chat_ledger_flushed,
            "shutdown durability was not fully proven; exiting with failure after preserving truthful process-ledger state"
        );
        // Do not unwind `state_owner` or release the lease while a worker may
        // or an accepted connection task does still exist. Process termination
        // ends them and releases the OS socket atomically from another
        // instance's point of view.
        std::process::exit(1);
    }

    // Runtime quiescence and queue acceptance are not sufficient to release
    // the final runtime owner. The process-ledger writer has now proved that
    // every accepted START/STOP row reached embedded SurrealDB, so dropping
    // AppState can no longer race durable lifecycle evidence.
    drop(state_owner.take());

    drop(api_runtime);
    drop(janitor);
    surreal_storage.shutdown().await?;
    if let Some(serve_result) = serve_result {
        serve_result?;
    }
    Ok(())
}

async fn wait_for_shutdown_request(mut shutdown_rx: tokio::sync::watch::Receiver<bool>) {
    if *shutdown_rx.borrow() {
        return;
    }
    while shutdown_rx.changed().await.is_ok() {
        if *shutdown_rx.borrow() {
            return;
        }
    }
}

/// WP-1 MT-013 (F1 graceful shutdown): resolves on the first OS shutdown signal
/// so `axum::serve(...).with_graceful_shutdown(...)` returns cleanly, letting the
/// ordered teardown (embedded-model STOP emit + ledger drain + lease release +
/// embedded-storage shutdown) run instead of being OS-killed mid-flight. Awaits Ctrl-C on
/// all platforms and SIGTERM additionally on Unix.
async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(err) = tokio::signal::ctrl_c().await {
            tracing::error!(target: "handshake_core", error = %err, "failed to install Ctrl-C handler");
            // If the handler cannot be installed, never resolve on this arm.
            std::future::pending::<()>().await;
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sigterm) => {
                sigterm.recv().await;
            }
            Err(err) => {
                tracing::error!(target: "handshake_core", error = %err, "failed to install SIGTERM handler");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }

    tracing::info!(target: "handshake_core", "shutdown signal received; beginning graceful shutdown");
}

fn startup_recovery_only_requested() -> bool {
    std::env::var("HANDSHAKE_STARTUP_RECOVERY_ONLY")
        .ok()
        .as_deref()
        == Some("1")
}

/// Resolve the hard wall-clock bound for the startup restart-resume boot pass.
/// A positive integer in `HANDSHAKE_RESTART_RESUME_BOOT_TIMEOUT_MS` overrides the
/// 30s default for hosts with a large resumable backlog; any absent, blank,
/// zero, or unparseable value falls back to the default so the pass can never be
/// left unbounded.
fn restart_resume_boot_timeout() -> Duration {
    std::env::var(RESTART_RESUME_BOOT_TIMEOUT_MS_ENV)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|millis| *millis > 0)
        .map(Duration::from_millis)
        .unwrap_or(RESTART_RESUME_BOOT_TIMEOUT_DEFAULT)
}

/// The product-core boot owner for ModelLane recovery. This runs after embedded
/// SurrealDB is ready and before any HTTP/native consumer is exposed. Keeping
/// the bounded await here makes recovery independent of the legacy Tauri host
/// while preserving the store's transaction/idempotency law.
async fn recover_model_lanes_at_core_boot_with_timeout(
    surreal_storage: SurrealStorage,
    resource_scope: handshake_core::swarm_orchestration::resource_scope::ResourceScope,
    timeout: Duration,
) -> Result<usize, std::io::Error> {
    let store = handshake_core::swarm_orchestration::model_lane::ModelLaneStore::new_scoped(
        surreal_storage,
        resource_scope,
    );
    match tokio::time::timeout(timeout, store.recover_restartable_runs_at_boot()).await {
        Ok(Ok(recovered)) => Ok(recovered.len()),
        Ok(Err(error)) => Err(std::io::Error::other(format!(
            "ModelLane startup recovery failed: {error}"
        ))),
        Err(_) => Err(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            format!(
                "ModelLane startup recovery exceeded the bounded timeout of {}ms",
                timeout.as_millis()
            ),
        )),
    }
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

/// MT-003 (WP-1) Ollama-kill: the default LlmClient now resolves LOCAL inference
/// through the embedded ModelRuntime (Candle CPU baseline / llama.cpp opt-in),
/// never an auto-detected Ollama daemon. All resolution logic lives in
/// [`resolve_default_llm_client`] (in `handshake_core::llm::boot`) so it is
/// unit-testable from the integration test crate; this binary only delegates.
async fn init_llm_client(
    flight_recorder: Arc<dyn FlightRecorder>,
    ledger: Option<LedgerBatcher>,
    model_registry_authority: Option<ScopedModelRegistryAuthority>,
    runtime_instance: Option<handshake_core::process_ledger::EmbeddedRuntimeInstanceDescriptor>,
) -> Arc<dyn LlmClient> {
    resolve_default_llm_client(
        flight_recorder,
        ledger,
        model_registry_authority,
        runtime_instance,
    )
    .await
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
    use super::{build_health_response, recover_model_lanes_at_core_boot_with_timeout};
    use std::time::Duration;

    #[test]
    fn health_response_ok_sets_status_ok() {
        let response = build_health_response("ok", Some(9));
        assert_eq!(response.status, "ok");
        assert_eq!(response.component, "handshake_core");
        assert_eq!(response.db_status, "ok");
        assert_eq!(response.migration_version, Some(9));
    }

    #[tokio::test]
    async fn core_boot_model_lane_recovery_is_bounded_and_fails_closed() {
        use handshake_core::storage::surreal::{SurrealStorage, SurrealStorageConfig};
        use handshake_core::swarm_orchestration::resource_scope::{
            AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, OwnerAccountId,
            ResourceScope, WorkspaceScopeRef,
        };

        let temp = tempfile::tempdir().expect("create temporary Surreal root");
        let storage = SurrealStorage::open(
            SurrealStorageConfig::for_scoped_store(
                temp.path().join("model-lane-boot"),
                "handshake_test",
                "model_lane_boot",
            )
            .expect("configure embedded Surreal store"),
        )
        .await
        .expect("open embedded Surreal store");
        let scope = ResourceScope::new(OwnerAccountId::mint(), ActorPrincipalId::mint())
            .with_session(AuthenticatedSessionRef::mint())
            .with_access_space(AccessSpaceRef::mint())
            .with_workspace(WorkspaceScopeRef::new("workspace-test").expect("workspace scope"));
        let started = std::time::Instant::now();
        let error =
            recover_model_lanes_at_core_boot_with_timeout(storage.clone(), scope, Duration::ZERO)
                .await
                .expect_err("core boot must not expose the runtime after recovery timeout");
        assert!(
            matches!(
                error.kind(),
                std::io::ErrorKind::TimedOut | std::io::ErrorKind::Other
            ),
            "provider failure or the explicit timeout are both fail-closed: {error}"
        );
        assert!(started.elapsed() < Duration::from_secs(1));
        storage.shutdown().await.expect("shutdown embedded store");
    }

    #[test]
    fn health_response_error_maps_to_overall_error() {
        let response = build_health_response("error", None);
        assert_eq!(response.status, "error");
        assert_eq!(response.db_status, "error");
        assert_eq!(response.migration_version, None);
    }
}

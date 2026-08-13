use axum::{extract::State, routing::get, Json, Router};
use handshake_core::{
    api,
    capabilities::CapabilityRegistry,
    diagnostics::DiagnosticsStore,
    flight_recorder::{duckdb::DuckDbFlightRecorder, FlightRecorder},
    llm::{boot::resolve_default_llm_client, LlmClient},
    logging,
    model_runtime::ModelRegistryStore,
    models::HealthResponse,
    process_ledger::{
        restart_resume::{
            BoundedRestartResumeOutcome, PostgresRestartResumeRunner,
            RESTART_RESUME_BOOT_TIMEOUT_DEFAULT,
        },
        LedgerBatcher, PostgresProcessLedgerStore, ProcessReclaimRuntime,
    },
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

    // WP-1 MT-013 (F1 hard-crash reconcile): capture the boot timestamp BEFORE
    // any embedded-model ProcessOwnershipLedger START row is written this run, so
    // the pid-less orphan reclaim sweep below can safely close only rows started
    // by a PRIOR (crashed) process and never this boot's own live row.
    let boot_started_at = chrono::Utc::now();

    logging::init_logging();

    // The desktop host owns the persisted product-local identity. Parse its
    // strict five-dimensional handoff before opening account-facing services;
    // a missing or corrupt scope aborts boot instead of falling back to caller
    // headers or an unscoped node-global view.
    let product_local_scope = api::account_scope::ProductLocalResourceScope::from_env()?;
    std::env::remove_var(api::account_scope::PRODUCT_LOCAL_RESOURCE_SCOPE_ENV);

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
    // The restart-resume boot pass is bounded by a hard outer wall clock,
    // consistent with the sibling ModelLane boot recovery below and the
    // process-ledger boot reconcile (`time::timeout(startup_timeout)`). A single
    // resumable candidate whose orphan-reclaim UPDATE blocks must not stall boot
    // forever. On timeout the pass fails closed (no candidate is falsely marked
    // resumed), records a durable bounded-abort report, and boot continues so the
    // staleness reclaim task started later can finish reconciling; it never
    // panics or hangs.
    let restart_outcome = PostgresRestartResumeRunner::new(control_plane.postgres_pool.clone())
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
        control_plane.postgres_pool.clone(),
        MODEL_LANE_BOOT_RECOVERY_TIMEOUT,
    )
    .await?;
    tracing::info!(
        target: "handshake_core::model_lane_recovery",
        recovered_runs = recovered_model_lane_runs,
        "bounded core-owned ModelLane boot recovery completed"
    );
    let startup_recovery_only = startup_recovery_only_requested();

    // WP-1 MT-013 hard-crash ownership: prior embedded STARTs are reconciled on
    // every same-host boot, even after the operator switches to a cloud or
    // unconfigured provider. Only an actually configured embedded local lane
    // needs a new OS liveness lease. Provider/host/ledger failure keeps the
    // backend available but with local inference disabled before artifact access.
    let embedded_runtime_requested =
        match handshake_core::llm::boot::embedded_runtime_boot_requested_from_env() {
            Ok(requested) => requested,
            Err(error) => {
                tracing::warn!(
                    target: "handshake_core::llm",
                    error = %error,
                    "embedded runtime lease preflight could not resolve provider; deferring to fail-closed LLM resolution"
                );
                false
            }
        };
    let mut embedded_runtime_instance_lease = None;
    let mut runtime_instance = None;
    // Local-endpoint provenance is separate from shutdown ownership. A
    // postmaster adopted after a Handshake crash is non-owning (`is_managed`
    // remains false) but still carries an opaque proof token after SQL
    // data_directory/system_identifier, pg_ctl, postmaster.pid, and port
    // validation.
    let proven_local_postgres_endpoint = match managed_pg.proven_local_endpoint() {
        Some(proof) => {
            match handshake_core::process_ledger::verify_proven_local_postgres_endpoint_pool(
                &control_plane.postgres_pool,
                proof,
            )
            .await
            {
                Ok(()) => Some(proof),
                Err(error) => {
                    tracing::error!(
                        target: "handshake_core::process_ledger",
                        error = %error,
                        "control-plane PostgreSQL no longer matches the managed local-endpoint proof; automatic host scope is disabled"
                    );
                    None
                }
            }
        }
        None => None,
    };
    let explicit_host_scope =
        std::env::var(handshake_core::process_ledger::HANDSHAKE_HOST_SCOPE_ID_ENV).ok();
    let runtime_host_scope_id =
        match handshake_core::process_ledger::resolve_embedded_runtime_host_scope_with_managed_local(
            &storage_config.database_url,
            explicit_host_scope.as_deref(),
            proven_local_postgres_endpoint,
        ) {
            Ok(host_scope_id) => Some(host_scope_id),
            Err(error) => {
                tracing::warn!(
                    target: "handshake_core::process_ledger",
                    error = %error,
                    embedded_runtime_requested,
                    "embedded runtime host scope unavailable; orphan sweep is deferred and any configured local inference will fail closed"
                );
                None
            }
        };
    let mut ledger_authority_healthy = false;
    if let Some(runtime_host_scope_id) = runtime_host_scope_id.as_deref() {
        let pool_proof_still_matches = match proven_local_postgres_endpoint {
            Some(proof) => {
                match handshake_core::process_ledger::verify_proven_local_postgres_endpoint_pool(
                    &control_plane.postgres_pool,
                    proof,
                )
                .await
                {
                    Ok(()) => true,
                    Err(error) => {
                        tracing::error!(
                            target: "handshake_core::process_ledger",
                            error = %error,
                            "control-plane PostgreSQL changed after host-scope derivation; orphan reclaim is withheld"
                        );
                        false
                    }
                }
            }
            None => true,
        };
        let reclaim_result = if pool_proof_still_matches {
            Some(
                handshake_core::process_ledger::reclaim_pidless_embedded_orphans(
                    &control_plane.postgres_pool,
                    boot_started_at,
                    runtime_host_scope_id,
                )
                .await,
            )
        } else {
            None
        };
        match reclaim_result {
            None => {}
            Some(Ok(reclaim_report)) if reclaim_report.is_complete() => {
                ledger_authority_healthy = true;
                tracing::info!(
                    target: "handshake_core::process_ledger",
                    orphans_closed = reclaim_report.closed_rows,
                    runtime_host_scope_id,
                    "instance-aware pid-less embedded-model orphan reclaim sweep complete"
                );
            }
            Some(Ok(reclaim_report)) => {
                ledger_authority_healthy = true;
                tracing::warn!(
                    target: "handshake_core::process_ledger",
                    orphans_closed = reclaim_report.closed_rows,
                    deferred_instances = reclaim_report.deferred_instances,
                    candidate_scan_timed_out = reclaim_report.candidate_scan_timed_out,
                    candidate_instance_limit_reached = reclaim_report.candidate_instance_limit_reached,
                    legacy_host_scope_open_rows = ?reclaim_report.legacy_host_scope_open_rows,
                    runtime_host_scope_id,
                    "instance-aware pid-less embedded-model orphan reclaim sweep is incomplete; legacy rows require operator inspection and transient deferrals retry on a later boot"
                );
            }
            Some(Err(error)) => {
                tracing::error!(
                    target: "handshake_core::process_ledger",
                    error = %error,
                    runtime_host_scope_id,
                    "embedded-model ledger authority preflight failed; prior START rows remain open and configured local inference will fail closed"
                );
            }
        }
    }

    if startup_recovery_only {
        // MT-019 F4: this branch returns BEFORE `ProcessReclaimRuntime::production_with_lease`,
        // so it used to run only `reclaim_pidless_embedded_orphans` and never
        // surfaced generic spawned-process (Official-CLI bridge) restart orphans.
        // A recovery-only pass is exactly when an operator most needs them
        // reconciled. This must complete BEFORE `managed_pg.stop()` below, because
        // the reconcile is PostgreSQL-authoritative.
        //
        // Honest limitation: the P-4(b) dead-owner corroboration requires two
        // observations at least one scan interval apart, and a recovery-only pass
        // is a single short-lived process, so it normally records the FIRST
        // observation and reclaims on a later run. That is the intended fail-safe
        // direction: never kill a possibly-live process on one sample.
        if let Some(runtime_host_scope_id) = runtime_host_scope_id.as_deref() {
            match ProcessReclaimRuntime::production(
                control_plane.postgres_pool.clone(),
                Arc::new(PostgresProcessLedgerStore::new(
                    control_plane.postgres_pool.clone(),
                )),
                None,
                handshake_core::process_ledger::production_process_sandbox_registry_async().await?,
                runtime_host_scope_id.to_string(),
                Duration::from_secs(30),
            )
            .await
            {
                Ok(recovery_runtime) => {
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
                        lease_released = drain.lease_released,
                        lease_retained_reason = ?drain.lease_retained_reason,
                        "startup recovery-only process runtime drained"
                    );
                }
                Err(error) => tracing::error!(
                    target: "handshake_core::process_ledger",
                    error = %error,
                    runtime_host_scope_id,
                    "startup recovery-only restart-orphan reconcile failed; restart orphans remain open for a later pass"
                ),
            }
        }
        let report_result = write_startup_recovery_report(&restart_report);
        if let Err(err) = managed_pg.stop().await {
            tracing::warn!(
                target: "handshake_core::managed_postgres",
                error = %err,
                "managed PostgreSQL stop failed after startup recovery-only pass"
            );
        }
        report_result?;
        return Ok(());
    }

    if ledger_authority_healthy {
        if let Some(proof) = proven_local_postgres_endpoint {
            if let Err(error) =
                handshake_core::process_ledger::verify_proven_local_postgres_endpoint_pool(
                    &control_plane.postgres_pool,
                    proof,
                )
                .await
            {
                ledger_authority_healthy = false;
                tracing::error!(
                    target: "handshake_core::process_ledger",
                    error = %error,
                    "control-plane PostgreSQL changed after orphan reclaim; embedded runtime lease acquisition is withheld"
                );
            }
        }
    }

    match (
        embedded_runtime_requested,
        ledger_authority_healthy,
        runtime_host_scope_id.as_ref(),
    ) {
        (true, true, Some(runtime_host_scope_id)) => {
            match handshake_core::process_ledger::acquire_embedded_runtime_instance_lease(
                uuid::Uuid::now_v7(),
                runtime_host_scope_id.clone(),
            ) {
                Ok(lease) => {
                    runtime_instance = Some(lease.descriptor().clone());
                    embedded_runtime_instance_lease = Some(lease);
                }
                Err(error) => {
                    tracing::error!(
                        target: "handshake_core::process_ledger",
                        error = %error,
                        "embedded runtime lease unavailable; local inference will fail closed without aborting the backend"
                    );
                }
            }
        }
        (false, _, _) => {
            tracing::info!(
                target: "handshake_core::process_ledger",
                "no new embedded runtime lease required by the resolved default provider"
            );
        }
        (true, _, _) => {
            tracing::error!(
                target: "handshake_core::process_ledger",
                "embedded runtime lease withheld because ledger authority preflight did not succeed; local inference will fail closed"
            );
        }
    }

    let storage = control_plane.database.clone();
    let recorder = init_flight_recorder().await?;
    let flight_recorder: Arc<dyn FlightRecorder> = recorder.clone();
    let diagnostics: Arc<dyn DiagnosticsStore> = recorder.clone();

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

    // WP-KERNEL-009 MT-195/MT-196: seed (or hash-resync) the built-in
    // UserManual corpus so the no-context manual surface is queryable from
    // boot. Idempotent (content-hash short-circuit; receipts only for changed
    // rows). A seed failure is logged loudly but does not abort startup — the
    // freshness route reports `unseeded_version` and the gated
    // POST /usermanual/resync recovers.
    {
        let manual_db = handshake_core::storage::postgres::PostgresDatabase::new(
            control_plane.postgres_pool.clone(),
        );
        match handshake_core::user_manual::seed::ensure_seeded(&manual_db).await {
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
    if embedded_runtime_instance_lease.is_none() {
        let host_scope_id = runtime_host_scope_id.clone().ok_or_else(|| {
            handshake_core::process_ledger::ProcessLedgerError::InvalidConfig(
                "process runtime requires a verified host scope".to_string(),
            )
        })?;
        embedded_runtime_instance_lease = Some(
            handshake_core::process_ledger::acquire_embedded_runtime_instance_lease(
                uuid::Uuid::now_v7(),
                host_scope_id,
            )?,
        );
    }
    let process_runtime_lease = embedded_runtime_instance_lease.take().ok_or_else(|| {
        handshake_core::process_ledger::ProcessLedgerError::InvalidConfig(
            "process runtime liveness lease was not acquired".to_string(),
        )
    })?;
    let process_ledger_store = Arc::new(PostgresProcessLedgerStore::new(
        control_plane.postgres_pool.clone(),
    ));
    let process_runtime = ProcessReclaimRuntime::production_with_lease(
        control_plane.postgres_pool.clone(),
        process_ledger_store,
        None,
        handshake_core::process_ledger::production_process_sandbox_registry_async().await?,
        process_runtime_lease,
        Duration::from_secs(30),
    )
    .await?;
    runtime_instance = Some(process_runtime.runtime_instance().clone());
    let model_registry_store = ModelRegistryStore::new_scoped(
        control_plane.postgres_pool.clone(),
        product_local_scope.resource_scope(),
    );
    let llm_client = init_llm_client(
        flight_recorder.clone(),
        Some(process_runtime.ledger().ledger()),
        Some(model_registry_store),
        runtime_instance,
    )
    .await;
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
        storage,
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
    let server = std::future::IntoFuture::into_future(
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(async move {
            wait_for_shutdown_request(server_shutdown_rx).await;
        }),
    );
    tokio::pin!(server);
    let connection_drain_deadline = async move {
        wait_for_shutdown_request(deadline_shutdown_rx).await;
        tokio::time::sleep(HTTP_CONNECTION_DRAIN_TIMEOUT).await;
    };
    tokio::pin!(connection_drain_deadline);
    let (serve_result, connection_drain_timed_out) = tokio::select! {
        result = &mut server => (Some(result), false),
        _ = &mut connection_drain_deadline => (None, true),
    };
    signal_task.abort();
    let _ = signal_task.await;

    // The one-shot recovery scan owns an AppState clone, and therefore an LLM
    // runtime owner. Cancel and join it after Axum returns or reaches its
    // bounded connection deadline. On the deadline path the still-live Axum
    // connection owners force the explicit no-STOP process-exit branch below.
    // Stop the janitor before managed PostgreSQL teardown as well.
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
            operator_chat_ledger_flushed,
            "shutdown durability was not fully proven; exiting with failure after preserving truthful process-ledger state"
        );
        // Do not unwind `state_owner` or release the lease while a worker may
        // or an accepted connection task does still exist. Process termination
        // ends them and releases the OS socket atomically from another
        // instance's point of view. The managed PostgreSQL child is a separate
        // OS process, so stop only the cluster this handle started before the
        // hard exit; adopted/external clusters remain untouched by stop().
        if let Err(err) = managed_pg.stop().await {
            tracing::warn!(
                target: "handshake_core::managed_postgres",
                error = %err,
                "managed PostgreSQL stop failed before hard shutdown exit"
            );
        }
        std::process::exit(1);
    }

    // Runtime quiescence and queue acceptance are not sufficient to release
    // the final runtime owner. The process-ledger writer has now proved that
    // every accepted START/STOP row reached PostgreSQL, so dropping AppState can
    // no longer race durable lifecycle evidence.
    drop(state_owner.take());

    // 3) Best-effort teardown: stop the cluster only if Handshake started it
    //    (adopted/external clusters are left untouched).
    if let Err(err) = managed_pg.stop().await {
        tracing::warn!(target: "handshake_core::managed_postgres", error = %err, "managed PostgreSQL stop failed at shutdown");
    }
    drop(janitor);
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
/// PostgreSQL stop) run instead of being OS-killed mid-flight. Awaits Ctrl-C on
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

/// The product-core boot owner for ModelLane recovery. This runs after managed
/// PostgreSQL and migrations are ready and before any HTTP/native consumer is
/// exposed. Keeping the bounded await here makes recovery independent of the
/// legacy Tauri host while preserving the store's transaction/idempotency law.
async fn recover_model_lanes_at_core_boot_with_timeout(
    pool: sqlx::PgPool,
    timeout: Duration,
) -> Result<usize, std::io::Error> {
    // Restart recovery is the one ModelLane read that is legitimately
    // cross-owner: it runs before any account has authenticated and must not
    // strand another account's abandoned run. The authority is therefore named
    // explicitly rather than left as an unscoped store (HBR-PRIV-002).
    let store =
        handshake_core::swarm_orchestration::model_lane::ModelLaneStore::new_system_authority(
            pool,
            handshake_core::swarm_orchestration::resource_scope::SystemScopeAuthority::boot_recovery(
            ),
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
    model_registry_store: Option<ModelRegistryStore>,
    runtime_instance: Option<handshake_core::process_ledger::EmbeddedRuntimeInstanceDescriptor>,
) -> Arc<dyn LlmClient> {
    resolve_default_llm_client(
        flight_recorder,
        ledger,
        model_registry_store,
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
        let pool = sqlx::postgres::PgPoolOptions::new()
            .acquire_timeout(Duration::from_secs(5))
            .connect_lazy("postgresql://127.0.0.1:1/handshake_core_boot_recovery_unavailable")
            .expect("construct deterministic unavailable PostgreSQL pool");
        let started = std::time::Instant::now();
        let error = recover_model_lanes_at_core_boot_with_timeout(pool, Duration::from_millis(25))
            .await
            .expect_err("core boot must not expose the runtime after recovery timeout");
        assert!(
            matches!(
                error.kind(),
                std::io::ErrorKind::TimedOut | std::io::ErrorKind::Other
            ),
            "connection refusal or the explicit timeout are both fail-closed: {error}"
        );
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn health_response_error_maps_to_overall_error() {
        let response = build_health_response("error", None);
        assert_eq!(response.status, "error");
        assert_eq!(response.db_status, "error");
        assert_eq!(response.migration_version, None);
    }
}

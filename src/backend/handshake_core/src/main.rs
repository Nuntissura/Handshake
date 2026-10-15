use axum::{extract::State, routing::get, Json, Router};
use handshake_core::{
    api,
    atelier::AtelierStore,
    capabilities::CapabilityRegistry,
    diagnostics::DiagnosticsStore,
    flight_recorder::{duckdb::DuckDbFlightRecorder, FlightRecorder},
    llm::{boot::resolve_default_llm_client, LlmClient},
    logging,
    model_runtime::{ModelRegistryStore, ScopedModelRegistryAuthority},
    models::HealthResponse,
    process_ledger::{
        restart_resume::{
            BoundedRestartResumeOutcome, RegistryStartupProcessCleanup,
            SurrealRestartOrphanReclaimer, SurrealRestartResumeRunner,
            RESTART_RESUME_BOOT_TIMEOUT_DEFAULT,
        },
        LedgerBatcher, ProcessReclaimRuntime, ReclaimResourceScope, SurrealProcessLedgerStore,
    },
    sandbox::build_startup_recovery_registry_async,
    storage::{
        self,
        retention::{Janitor, JanitorConfig},
        surreal::SurrealDatabase,
    },
    user_manual, workflows, AppState,
};
use std::{
    fs::OpenOptions,
    io::{self, Write},
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

const HTTP_CONNECTION_DRAIN_TIMEOUT: Duration = Duration::from_secs(30);

/// Explicit equivalent of `#[tokio::main]` (multi-thread, all drivers) with a 10 MiB worker
/// stack: the embedded SurrealDB 3.2.0 engine evaluates permission expressions recursively and its
/// docs recommend this size (surrealdb-3.2.0/src/engine/local/mod.rs:51-66; MT-164 remediation_v1).
fn main() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(10 * 1024 * 1024)
        .build()
        .expect("Failed building the Runtime")
        .block_on(async {
            if let Err(err) = run().await {
                tracing::error!(target: "handshake_core", error = %err, "handshake_core failed to start");
                std::process::exit(1);
            }
        })
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let requested_addr = backend_listen_addr()?;

    logging::init_logging();
    let recovery_only_scope = if startup_recovery_only_requested() {
        Some(api::account_scope::ProductLocalResourceScope::from_env()?)
    } else { None };
    std::env::remove_var(api::account_scope::PRODUCT_LOCAL_RESOURCE_SCOPE_ENV);

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // The database is EMBEDDED. `init_control_plane_storage_with_config` below
    // opens Handshake's own in-process SurrealDB store, so there is no cluster
    // to start or adopt, no console window to suppress, no connection string to
    // resolve, and no operator step at all. The store is closed again at the end
    // of this function, so its lifetime is exactly this process's lifetime
    // [CX-503R/CX-503S].
    let storage_config = storage::ControlPlaneStorageConfig::from_env()?;
    tracing::info!(
        target: "handshake_core",
        storage_mode = %storage_config.mode,
        "control-plane storage mode resolved"
    );
    let control_plane = storage::init_control_plane_storage_with_config(&storage_config).await?;
    if let Some(scope) = recovery_only_scope {
        perform_startup_recovery(&control_plane.surreal, ReclaimResourceScope::from_exact(scope.exact())?).await?;
        control_plane.surreal.shutdown().await?;
        return Ok(());
    }
    let recorder = init_flight_recorder().await?;
    let capability_registry = Arc::new(CapabilityRegistry::new());
    let bootstrap_state = handshake_core::AccountBootstrapState::new(
        control_plane.database.clone(), control_plane.surreal.clone(), capability_registry.clone(),
    );
    let (activation_tx, mut activation_rx) = tokio::sync::mpsc::channel(16);
    let mounted: api::workspace_activation_readiness::MountedApiState =
        Arc::new(tokio::sync::RwLock::new(None));
    let bootstrap = api::authority::bootstrap_routes(bootstrap_state.clone(), activation_tx)
        .merge(api::workspaces::bootstrap_routes(bootstrap_state.clone()))
        .merge(Router::new().route("/health", get(bootstrap_health)).with_state(bootstrap_state.clone()))
        .layer(axum::Extension(mounted.clone()));
    let dispatch_mount = mounted.clone();
    let app = bootstrap.clone().nest("/api", bootstrap)
        .fallback(move |request: axum::extract::Request| {
            let mounted = dispatch_mount.clone();
            async move {
                let guard = mounted.read_owned().await;
                match guard.as_ref() {
                    Some(current) => {
                        use futures::StreamExt;
                        let response = dispatch_router(current.router(), request).await;
                        let (parts, body) = response.into_parts();
                        let body = body.into_data_stream().map(move |chunk| {
                            let _runtime_dispatch_owner = &guard;
                            chunk
                        });
                        axum::response::Response::from_parts(parts, axum::body::Body::from_stream(body))
                    },
                    None => axum::response::IntoResponse::into_response((
                        axum::http::StatusCode::SERVICE_UNAVAILABLE,
                        Json(serde_json::json!({"code":"workspace_activation_required"})),
                    )),
                }
            }
        }).layer(cors);
    let listener = TcpListener::bind(requested_addr).await?;
    let actual_addr = listener.local_addr()?;
    write_backend_listen_report(actual_addr)?;
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    let signal_task = tokio::spawn(async move {
        shutdown_signal().await;
        let _ = shutdown_tx.send(true);
    });
    let server_shutdown_rx = shutdown_rx.clone();
    let mut server = tokio::spawn(async move {
        axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
            .with_graceful_shutdown(async move { wait_for_shutdown_request(server_shutdown_rx).await; })
            .await
    });
    let deadline_shutdown_rx = shutdown_rx.clone();
    let connection_drain_deadline = async move {
        wait_for_shutdown_request(deadline_shutdown_rx).await;
        tokio::time::sleep(HTTP_CONNECTION_DRAIN_TIMEOUT).await;
    };
    tokio::pin!(connection_drain_deadline);
    let mut activation = tokio::select! {
        request = activation_rx.recv() => request.ok_or("workspace activation channel closed")?,
        result = &mut server => { server_result(result)?; control_plane.surreal.shutdown().await?; return Ok(()); },
        _ = &mut connection_drain_deadline => { return Err("account HTTP drain timed out".into()); },
    };
    loop {
        if activation.authorize_for_publication().await.is_err() {
            activation.deny();
            activation = tokio::select! {
                request = activation_rx.recv() => request.ok_or("workspace activation channel closed")?,
                result = &mut server => { server_result(result)?; break; },
                _ = &mut connection_drain_deadline => { return Err("account HTTP drain timed out".into()); },
            };
            continue;
        }
        let product_local_scope = activation.scope().clone();
        let reclaim_resource_scope = ReclaimResourceScope::from_exact(product_local_scope.exact())?;
        perform_startup_recovery(&control_plane.surreal, reclaim_resource_scope.clone()).await?;
        let storage = control_plane.database.clone();
        let flight_recorder: Arc<dyn FlightRecorder> = recorder.clone();
        let diagnostics: Arc<dyn DiagnosticsStore> = recorder.clone();
        // WP-KERNEL-012 MT-138: fail closed before serving Atelier routes unless
        // every canonical atelier_* table is present, then project the builtin CKC
        // command corpus into the same embedded authority. Both operations are
        // idempotent, so every restart converges without a compatibility store.
        let atelier = AtelierStore::with_observability(
            control_plane.surreal.clone(),
            storage.clone(),
            flight_recorder.clone(),
        );
        let atelier_schema = atelier.bootstrap_schema().await?;
        let atelier_corpus = atelier.bootstrap_builtin_command_corpus().await?;
        tracing::info!(
            target: "handshake_core::atelier",
            schema_applied = atelier_schema.applied,
            atelier_table_count = atelier_schema.table_count,
            total_commands = atelier_corpus.total_commands,
            covered_commands = atelier_corpus.covered_count,
            blocked_commands = atelier_corpus.blocked_count,
            "atelier schema readiness and builtin command-corpus projection completed against embedded SurrealDB"
        );

        // WP-KERNEL-009 MT-195/MT-196: seed (or hash-resync) the built-in
        // UserManual corpus so the no-context manual surface is queryable from
        // boot. Idempotent (content-hash short-circuit; receipts only for changed
        // rows). A seed failure is logged loudly but does not abort startup — the
        // freshness route reports `unseeded_version` and the gated
        // POST /usermanual/resync recovers.
        let manual_db = SurrealDatabase::new(control_plane.surreal.clone());
        match user_manual::seed::ensure_seeded(&manual_db).await {
            Ok(report) => tracing::info!(
                target: "handshake_core::user_manual",
                manual_version = %report.manual_version,
                pages_changed = report.pages_changed,
                tools_changed = report.tools_changed,
                features_changed = report.features_changed,
                aliases_changed = report.aliases_changed,
                "UserManual corpus seed synchronized against embedded SurrealDB"
            ),
            Err(error) => tracing::error!(
                target: "handshake_core::user_manual",
                error = %error,
                "UserManual corpus seed failed; freshness and resync surfaces remain available"
            ),
        }

        let runtime_host_scope_id =
            handshake_core::process_ledger::resolve_embedded_runtime_host_scope()?;
        let process_runtime = ProcessReclaimRuntime::production(
            control_plane.surreal.clone(),
            reclaim_resource_scope,
            None,
            handshake_core::process_ledger::production_process_sandbox_registry_async().await?,
            runtime_host_scope_id,
            Duration::from_secs(30),
        )
        .await?;
        let (model_access_state, cloud, cloud_registry, cli_providers) =
            match model_access_state(&process_runtime, &product_local_scope) {
                Ok(dependencies) => dependencies,
                Err(error) => {
                    tracing::error!(error = %error, "scoped model access initialization failed; no activation ACK published");
                    drain_unpublished_runtime(&process_runtime, None).await;
                    activation.unavailable();
                    drop(process_runtime);
                    activation = tokio::select! {
                        request = activation_rx.recv() => request.ok_or("workspace activation channel closed")?,
                        result = &mut server => { server_result(result)?; break; },
                        _ = &mut connection_drain_deadline => { return Err("account HTTP drain timed out".into()); },
                    };
                    continue;
                }
            };
        let model_registry_authority = ScopedModelRegistryAuthority::new(
            ModelRegistryStore::new(control_plane.surreal.clone()),
            product_local_scope.exact().clone(),
        );
        let llm_client = init_llm_client(
            flight_recorder.clone(),
            Some(process_runtime.ledger().ledger()),
            Some(model_registry_authority),
            Some(process_runtime.runtime_instance().clone()),
        )
        .await;
        let session_registry = Arc::new(workflows::SessionRegistry::new(
            workflows::SessionSchedulerConfig::from_env(),
        ));

        let state = AppState {
            storage: storage.clone(),
            surreal: control_plane.surreal.clone(),
            flight_recorder: flight_recorder.clone(),
            diagnostics,
            llm_client,
            capability_registry: capability_registry.clone(),
            session_registry,
        };

        let catalog = state.llm_client.model_catalog();

        match workflows::repair_model_run_workflow_bindings(&state).await {
            Ok(repaired) => tracing::info!(
                target: "handshake_core::model_run_scheduler",
                repaired,
                "model_run workflow bindings repaired at startup"
            ),
            Err(err) => {
                tracing::error!(
                    target: "handshake_core::model_run_scheduler",
                    error = %err,
                    "model_run workflow binding startup repair failed"
                );
                drain_unpublished_runtime(&process_runtime, Some(state.llm_client.as_ref())).await;
                activation.unavailable();
                drop(state);
                drop(process_runtime);
                activation = tokio::select! {
                    request = activation_rx.recv() => request.ok_or("workspace activation channel closed")?,
                    result = &mut server => { server_result(result)?; break; },
                    _ = &mut connection_drain_deadline => { return Err("account HTTP drain timed out".into()); },
                };
                continue;
            }
        }

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

        let (launch_service, swarm_drain) = handshake_core::swarm_orchestration::production_factory::
            build_scoped_operator_chat_launch_service_with_sandbox_registry_and_lifecycle(
                state.surreal.clone(), state.flight_recorder.clone(), catalog.clone(),
                process_runtime.ledger(), process_runtime.reclaim(), cloud,
                Some(process_runtime.sandbox_registry()), state.surreal.resource_access_lifecycle(),
                product_local_scope.exact().clone(), uuid::Uuid::now_v7(),
            );
        let operator_chat_state = api::operator_chat::OperatorChatState::with_production_dependencies(
            launch_service.clone(), catalog, cloud_registry, model_access_state.cli_auth_probe(),
            cli_providers, state.flight_recorder.clone(), state.session_registry.clone(),
        );
        let mounted_runtime = process_runtime.clone();
        let runtime_instance = process_runtime.runtime_instance().clone();
        let api::ApiRoutes { router: api_routes, runtime: api_runtime } =
            api::routes_with_scoped_operator_chat(
                state.clone(), process_runtime, product_local_scope.clone(), model_access_state,
                operator_chat_state, launch_service, swarm_drain,
            );

        let scoped_app = Router::new()
            .route("/health", get(health))
            .with_state(state.clone())
            .merge(api_routes.clone())
            .nest("/api", api_routes);

        tracing::info!(target: "handshake_core", listen_addr = %actual_addr, "handshake_core started");

        let mut publication = mounted.write().await;
        let publication_result = activation.acknowledge_with(&runtime_instance, || {
            *publication = Some(api::workspace_activation_readiness::MountedApi::new(
                scoped_app, mounted_runtime, bootstrap_state.clone(), product_local_scope.exact().clone(),
            ));
            Ok(())
        }).await;
        if publication_result.is_err() {
            *publication = None;
        }
        drop(publication);
        let mut next_activation = None;
        let (serve_result, connection_drain_timed_out) = if publication_result.is_err() {
            (None, false)
        } else {
            loop {
                tokio::select! {
                    result = &mut server => break (Some(server_result(result)), false),
                    _ = &mut connection_drain_deadline => break (None, true),
                    request = activation_rx.recv() => match request {
                        Some(request) if request.scope() == &product_local_scope => {
                            let _ = request.acknowledge_with(&runtime_instance, || Ok(())).await;
                        }
                        Some(request) => { next_activation = Some(request); break (None, false); }
                        None => break (None, false),
                    },
                }
            }
        };
        let mount_closed = tokio::time::timeout(HTTP_CONNECTION_DRAIN_TIMEOUT, mounted.write()).await;
        let connection_drain_timed_out = connection_drain_timed_out || mount_closed.is_err();
        if let Ok(mut guard) = mount_closed { *guard = None; }

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

        if let Err(error) = api_runtime.quiesce_swarm(Duration::from_secs(35)).await {
            state.llm_client.leave_open_for_reconciliation();
            tracing::error!(target: "handshake_core::swarm_orchestration", error = %error,
                "actual coordinator did not retire; downstream drains and scope replacement are forbidden");
            std::process::exit(1);
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
        // the final owner. Lease release remains the shared runtime's own decision. The Option keeps both paths explicit
        // without an accidental Drop during error unwinding.
        let mut state_owner = Some(state);

        // 2) Quiesce the single reclaimer before closing and draining the single
        // process-ledger writer. The shared runtime releases its OS lease only after
        // both boundaries are proven and its exact-scope open-row check permits release.
        let api_drain_report = api_runtime
            .drain_and_join(std::time::Duration::from_secs(35))
            .await;
        tracing::info!(
            target: "handshake_core::process_ledger",
            outcome = ?api_drain_report.operator_chat_process_ledger,
            "operator-chat process ledger drain-and-join at shutdown"
        );

        let operator_chat_ledger_flushed = api_drain_report
            .operator_chat_process_ledger
            .as_ref()
            .map(|outcome| {
                matches!(
                    outcome,
                    handshake_core::process_ledger::LedgerDrainJoinOutcome::Flushed
                        | handshake_core::process_ledger::LedgerDrainJoinOutcome::AlreadyDrained
                )
            });

        if connection_drain_timed_out
            || runtime_shutdown.is_err()
            || !api_drain_report.coordinator_quiesced
            || !api_drain_report.lease_released
            || api_drain_report.lease_retained_reason.is_some()
            || !api_drain_report.process_reclaim_quiesced
            || !api_drain_report.swarm_events_flushed
            || operator_chat_ledger_flushed != Some(true)
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
                coordinator_quiesced = api_drain_report.coordinator_quiesced,
                coordinator_error = ?api_drain_report.coordinator_error,
                lease_released = api_drain_report.lease_released,
                lease_retained_reason = ?api_drain_report.lease_retained_reason,
                process_reclaim_quiesced = api_drain_report.process_reclaim_quiesced,
                swarm_events_flushed = api_drain_report.swarm_events_flushed,
                operator_chat_ledger_flushed = ?operator_chat_ledger_flushed,
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
        if let Some(request) = next_activation {
            activation = request;
            continue;
        }
        if publication_result.is_err() {
            activation = tokio::select! {
                request = activation_rx.recv() => request.ok_or("workspace activation channel closed")?,
                result = &mut server => { server_result(result)?; break; },
                _ = &mut connection_drain_deadline => { return Err("account HTTP drain timed out".into()); },
            };
            continue;
        }
        if !server.is_finished() {
            server.abort();
            let _ = server.await;
        }
        signal_task.abort();
        let _ = signal_task.await;
        let shutdown_result = control_plane.surreal.shutdown().await;
        if let Err(err) = &shutdown_result {
            tracing::error!(target: "handshake_core::storage", error = %err,
                "embedded SurrealDB store did not close cleanly at shutdown");
        } else {
            tracing::info!(target: "handshake_core::storage", "embedded SurrealDB store closed");
        }
        return match (serve_result.unwrap_or(Ok(())), shutdown_result) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(serve_error), Ok(())) => Err(serve_error.into()),
            (Ok(()), Err(shutdown_error)) => Err(shutdown_error.into()),
            (Err(serve_error), Err(shutdown_error)) => Err(format!(
                "serve loop failed: {serve_error}; embedded SurrealDB shutdown also failed: {shutdown_error}"
            ).into()),
        };
    }
    control_plane.surreal.shutdown().await?;
    Ok(())
}

fn server_result(result: Result<io::Result<()>, tokio::task::JoinError>) -> io::Result<()> {
    result.map_err(io::Error::other)?
}

async fn drain_unpublished_runtime(runtime: &ProcessReclaimRuntime, client: Option<&dyn LlmClient>) {
    let runtime_quiesced = match client {
        Some(client) => client.shutdown_gracefully().await.is_ok(),
        None => true,
    };
    let report = runtime.shutdown_and_drain(Duration::from_secs(35)).await;
    let ledger_flushed = matches!(report.ledger,
        handshake_core::process_ledger::LedgerDrainJoinOutcome::Flushed
        | handshake_core::process_ledger::LedgerDrainJoinOutcome::AlreadyDrained);
    if !runtime_quiesced || !report.reclaim_task_quiesced || !ledger_flushed || !report.lease_released {
        if let Some(client) = client { client.leave_open_for_reconciliation(); }
        tracing::error!(runtime_quiesced, ledger_flushed, reclaim_quiesced = report.reclaim_task_quiesced,
            lease_released = report.lease_released, "unpublished runtime did not prove durable retirement");
        std::process::exit(1);
    }
}

async fn perform_startup_recovery(
    surreal: &handshake_core::storage::surreal::SurrealStorage,
    reclaim_resource_scope: ReclaimResourceScope,
) -> Result<(), Box<dyn std::error::Error>> {
    let sandbox_registry = Arc::new(build_startup_recovery_registry_async().await);
    let process_store = Arc::new(SurrealProcessLedgerStore::new(
        surreal.clone(),
    ));
    let startup_cleanup = Arc::new(RegistryStartupProcessCleanup::new(sandbox_registry));
    let orphan_reclaimer = Arc::new(SurrealRestartOrphanReclaimer::new(
        process_store,
        startup_cleanup,
    ));
    let restart_outcome = SurrealRestartResumeRunner::open(
        surreal.clone(),
        reclaim_resource_scope.clone(),
        orphan_reclaimer,
    )
    .await?
    .run_with_bound(restart_resume_boot_timeout())
    .await?;
    let restart_report = match restart_outcome {
        BoundedRestartResumeOutcome::Completed(report) => report,
        BoundedRestartResumeOutcome::TimedOut {
            timeout,
            report,
            evidence_persisted,
        } => {
            tracing::error!(target: "handshake_core::restart_resume",
                report_id = %report.report_id, timeout_ms = timeout.as_millis() as u64,
                evidence_persisted, "startup restart-resume timed out; incomplete report persistence is confirmed only when acknowledged");
            report
        }
    };
    write_startup_recovery_report(&restart_report)?;
    tracing::info!(
        target: "handshake_core::restart_resume",
        sessions_examined = restart_report.sessions_examined,
        sessions_resumed = restart_report.sessions_resumed.len(),
        sessions_recovery_failed = restart_report.sessions_recovery_failed.len(),
        "startup restart-resume outcome recorded against embedded SurrealDB"
    );
    Ok(())
}

async fn dispatch_router<S>(mut service: S, request: axum::extract::Request) -> axum::response::Response
where S: axum::ServiceExt<axum::extract::Request, Response = axum::response::Response, Error = std::convert::Infallible> {
    std::future::poll_fn(|context| service.poll_ready(context)).await.unwrap();
    service.call(request).await.unwrap()
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

fn backend_listen_addr() -> Result<SocketAddr, io::Error> {
    let value = std::env::var("HANDSHAKE_BACKEND_LISTEN_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:37501".to_owned());
    let addr = value.parse::<SocketAddr>().map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid HANDSHAKE_BACKEND_LISTEN_ADDR={value:?}: {error}"),
        )
    })?;
    if !addr.ip().is_loopback() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("HANDSHAKE_BACKEND_LISTEN_ADDR must be loopback-only, got {addr}"),
        ));
    }
    Ok(addr)
}

fn write_backend_listen_report(addr: SocketAddr) -> Result<(), io::Error> {
    let Some(path) = std::env::var_os("HANDSHAKE_BACKEND_LISTEN_REPORT_FILE").map(PathBuf::from)
    else {
        return Ok(());
    };
    if !path.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "HANDSHAKE_BACKEND_LISTEN_REPORT_FILE must be absolute",
        ));
    }
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut report = options.open(&path)?;
    serde_json::to_writer(
        &mut report,
        &serde_json::json!({
            "schema_id": "handshake.backend-listen-report.v1",
            "pid": std::process::id(),
            "listen_addr": addr.to_string(),
        }),
    )
    .map_err(io::Error::other)?;
    report.write_all(b"\n")?;
    report.sync_all()?;
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
    let data_dir = if let Some(configured) = std::env::var_os("HANDSHAKE_DATA_DIR") {
        PathBuf::from(configured)
    } else {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let root_dir = manifest_dir
            .parent()
            .and_then(Path::parent)
            .and_then(Path::parent)
            .map(Path::to_path_buf)
            .ok_or("failed to resolve repo root")?;
        root_dir.join("data")
    };
    if !data_dir.exists() {
        std::fs::create_dir_all(&data_dir)?;
    }
    // Flight recorder gets its own file
    let fr_db_path = data_dir.join("flight_recorder.db");

    let recorder = DuckDbFlightRecorder::new_on_path(&fr_db_path, 7)?;
    tracing::info!(target: "handshake_core", db_path = %fr_db_path.display(), "flight recorder ready");

    Ok(Arc::new(recorder))
}

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

fn model_access_state(
    runtime: &ProcessReclaimRuntime,
    scope: &api::account_scope::ProductLocalResourceScope,
) -> Result<(
    api::model_access::ModelAccessState,
    handshake_core::swarm_orchestration::production_factory::CloudLaneFactoryConfig,
    Arc<dyn handshake_core::model_runtime::cloud::ProviderAccessRegistry>,
    Vec<String>,
), Box<dyn std::error::Error>> {
    use handshake_core::model_runtime::cloud::{
        CliBridgeProvider, LiveCliSpawner, ProductionCliBridgeAuthStatusProbe,
    };
    use handshake_core::model_runtime::cloud::official_cli_bridge::CliSubprocessSpawner;
    let spawner = Arc::new(LiveCliSpawner::new(
        Arc::new(runtime.ledger().ledger()), runtime.sandbox_registry(),
    ).with_reclaim(runtime.reclaim(), ReclaimResourceScope::from_exact(scope.exact())?));
    let mut launches = Vec::new();
    let mut launch_configs = Vec::new();
    match std::env::var_os(api::model_access::HANDSHAKE_OFFICIAL_CLI_CONFIG_PATH) {
        Some(path) => {
            let path = PathBuf::from(path);
            let config = match api::model_access::load_official_cli_config_from_path(
                &path, CliBridgeProvider::Codex,
            ) {
                Ok(config) => (CliBridgeProvider::Codex, config),
                Err(api::model_access::OfficialCliConfigError::Contract("provider")) => (
                    CliBridgeProvider::ClaudeCode,
                    api::model_access::load_official_cli_config_from_path(
                        &path, CliBridgeProvider::ClaudeCode,
                    )?,
                ),
                Err(error) => return Err(error.into()),
            };
            let (launch, _) = config.1.clone().into_parts();
            launch_configs.push((config.0.id().to_owned(), config.1));
            spawner.pin_config(&launch).map_err(|_| io::Error::new(
                io::ErrorKind::InvalidInput, "official CLI launch graph rejected",
            ))?;
            launches.push((config.0, launch));
        }
        None => tracing::warn!(
            target: "handshake_core::model_access",
            code = "official_cli_configuration_unavailable",
            "CLI access remains unavailable without the explicit configuration record",
        ),
    }
    let probe = Arc::new(ProductionCliBridgeAuthStatusProbe::from_canonical_launches(
        spawner.clone(), launches,
    ));
    let state = api::model_access::ModelAccessState::production_for_owner(
        scope.exact().clone(), probe.clone(), probe,
    );
    let (mut cloud, registry) = match state.production_cloud_dependencies() {
        Ok(dependencies) => dependencies,
        Err(api::model_access::ModelAccessProductionError::CloudAccess(
            handshake_core::model_runtime::cloud::AccessConfigError::KeychainUnavailable,
        )) => (
            handshake_core::swarm_orchestration::production_factory::CloudLaneFactoryConfig::unconfigured(),
            state.production_provider_registry()?,
        ),
        Err(error) => return Err(error.into()),
    };
    let mut providers = Vec::new();
    for (provider, config) in launch_configs {
        providers.push(provider.clone());
        cloud = cloud.with_official_cli_provider_replay_captured(
            provider, spawner.clone(), config, None,
        );
    }
    Ok((state, cloud, registry, providers))
}

async fn bootstrap_health(State(state): State<handshake_core::AccountBootstrapState>) -> Json<HealthResponse> {
    let (status, version) = match state.storage.ping().await {
        Ok(()) => match state.storage.migration_version().await {
            Ok(version) => ("ok", Some(version)),
            Err(_) => ("error", None),
        },
        Err(_) => ("error", None),
    };
    Json(build_health_response(status, version))
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

fn restart_resume_boot_timeout() -> Duration {
    std::env::var("HANDSHAKE_RESTART_RESUME_BOOT_TIMEOUT_MS")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|millis| *millis > 0)
        .map(Duration::from_millis)
        .unwrap_or(RESTART_RESUME_BOOT_TIMEOUT_DEFAULT)
}

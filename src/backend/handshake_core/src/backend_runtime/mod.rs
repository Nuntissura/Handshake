use axum::{extract::State, routing::get, Json, Router};
use crate::{
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

mod configuration;
mod genie_x;
mod proof_transport;
mod startup_recovery;
#[cfg(feature = "os-keychain")]
pub(crate) mod member_recipient_startup;
use configuration::*;
use proof_transport::*;
use startup_recovery::*;

pub type ConsumerFuture<'a> = std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + 'a>>;

/// Runs only while the genuinely activated production state remains mounted.
/// Cancellation must retire all consumer work before returning.
pub trait RuntimeConsumer: 'static {
    fn consume<'a>(&'a self, state: &'a AppState, cancelled: tokio::sync::watch::Receiver<bool>) -> ConsumerFuture<'a>;
}

pub struct RuntimeOptions {
    pub listener: Option<TcpListener>,
    pub storage: storage::ControlPlaneStorageConfig,
}

impl RuntimeOptions {
    pub async fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            listener: None,
            storage: storage::ControlPlaneStorageConfig::from_env()?,
        })
    }
}

/// The owner task is deliberately detached, never aborted by dropping this handle.
/// Start inside a Tokio LocalSet; failed retirement retains its entire live stack.
/// The caller must keep that LocalSet running on failure or deadline expiry.
/// An error alone does not prove cleanup; query retirement_settled and join the owner.
pub struct RuntimeHandle {
    shutdown: tokio::sync::watch::Sender<bool>,
    terminal: tokio::sync::watch::Receiver<Option<Result<(), String>>>,
    owner: Option<tokio::task::JoinHandle<()>>,
    settled: Arc<std::sync::atomic::AtomicBool>,
}

impl RuntimeHandle {
    pub fn start(options: RuntimeOptions, consumer: Option<Arc<dyn RuntimeConsumer>>) -> Self {
        let (shutdown, shutdown_rx) = tokio::sync::watch::channel(false);
        let (terminal_tx, terminal) = tokio::sync::watch::channel(None);
        let owner_shutdown = shutdown.clone();
        let settled = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let owner_settled = settled.clone();
        let owner = tokio::task::spawn_local(async move {
            let result = run(options, consumer, owner_shutdown, shutdown_rx, terminal_tx.clone(), owner_settled).await;
            let _ = terminal_tx.send(Some(result.map_err(|error| error.to_string())));
        });
        Self { shutdown, terminal, owner: Some(owner), settled }
    }

    /// True only after the real production retirement and storage close succeeded.
    /// A failed consumer case may still be settled; join the owner before dropping its LocalSet.
    pub fn retirement_settled(&self) -> bool {
        self.settled.load(std::sync::atomic::Ordering::Acquire)
    }

    pub fn request_shutdown(&self) { let _ = self.shutdown.send(true); }

    pub async fn wait(&mut self) -> Result<(), String> {
        loop {
            let terminal = self.terminal.borrow().clone();
            match terminal {
                Some(Ok(())) => return self.join_retained_owner().await,
                Some(Err(_)) if self.retirement_settled() => return self.join_retained_owner().await,
                Some(Err(error)) => return Err(error),
                None => {}
            }
            self.terminal.changed().await.map_err(|_| "backend owner ended without a terminal result".to_owned())?;
        }
    }

    /// Await the same owner without aborting it or resetting a shutdown budget.
    /// After failed retirement this remains pending, keeping the caller's LocalSet
    /// running with the live owners. Only actual task completion can return.
    pub async fn join_retained_owner(&mut self) -> Result<(), String> {
        if let Some(owner) = self.owner.as_mut() {
            let joined = owner.await;
            self.owner.take();
            joined.map_err(|error| error.to_string())?;
        }
        self.terminal.borrow().clone()
            .unwrap_or_else(|| Err("backend owner ended without a terminal result".to_owned()))
    }

    pub async fn shutdown_at(&mut self, deadline: tokio::time::Instant) -> Result<(), String> {
        self.request_shutdown();
        tokio::time::timeout_at(deadline, self.wait()).await
            .map_err(|_| "backend shutdown missed the caller deadline; owner retained".to_owned())?
    }
}

impl Drop for RuntimeHandle {
    fn drop(&mut self) { self.request_shutdown(); }
}

async fn retain_failed_runtime(
    terminal: &tokio::sync::watch::Sender<Option<Result<(), String>>>, message: &str,
) -> ! {
    let _ = terminal.send(Some(Err(message.to_owned())));
    std::future::pending().await
}

async fn retain_failed_runtime_with_consumer(
    terminal: &tokio::sync::watch::Sender<Option<Result<(), String>>>,
    retirement_reason: &str,
    consumer_error: Option<&str>,
) -> ! {
    let message = match consumer_error {
        Some(error) => format!("consumer case failed: {error}; retirement failed: {retirement_reason}"),
        None => retirement_reason.to_owned(),
    };
    retain_failed_runtime(terminal, &message).await
}

async fn run(
    options: RuntimeOptions,
    consumer: Option<Arc<dyn RuntimeConsumer>>,
    shutdown_tx: tokio::sync::watch::Sender<bool>,
    shutdown_rx: tokio::sync::watch::Receiver<bool>,
    terminal: tokio::sync::watch::Sender<Option<Result<(), String>>>,
    settled: Arc<std::sync::atomic::AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {

    let requested_addr = backend_listen_addr()?;
    if let Some(listener) = options.listener.as_ref() {
        let adopted_addr = listener.local_addr()?;
        if !adopted_addr.ip().is_loopback() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("adopted backend listener must be loopback-only, got {adopted_addr}"),
            ).into());
        }
    }
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
    let storage_config = options.storage;
    tracing::info!(
        target: "handshake_core",
        storage_mode = %storage_config.mode,
        "control-plane storage mode resolved"
    );
    let control_plane = storage::init_control_plane_storage_with_config(&storage_config).await?;
    if let Some(scope) = recovery_only_scope {
        perform_startup_recovery(&control_plane.surreal, ReclaimResourceScope::from_exact(scope.exact())?).await?;
        control_plane.surreal.shutdown().await?;
        settled.store(true, std::sync::atomic::Ordering::Release);
        return Ok(());
    }
    let recorder = init_flight_recorder().await?;
    let capability_registry = Arc::new(CapabilityRegistry::new());
    macro_rules! retain_on_error {
        ($result:expr) => {
            match $result {
                Ok(value) => value,
                Err(error) => retain_failed_runtime(&terminal, &error.to_string()).await,
            }
        };
    }
    #[cfg(feature = "os-keychain")]
    let mut recipient_startup = member_recipient_startup::member_recipient_startup(control_plane.surreal.clone());
    #[cfg(feature = "os-keychain")]
    let recipient_runtime = match recipient_startup.observe().await {
        Ok(ready) => match ready.recipient() {
            Ok(runtime) => Arc::clone(runtime),
            Err(_) => retain_failed_runtime(&terminal, "member recipient factory failed; backend owner retained").await,
        },
        Err(_) => retain_failed_runtime(&terminal, "member recipient initialization failed; backend owner retained").await,
    };
    #[cfg(feature = "os-keychain")]
    let recipient_lifecycle = Arc::new(member_recipient_startup::MemberRecipientLifecycle::new(Arc::clone(&recipient_runtime)));
    #[cfg(feature = "os-keychain")]
    let invitation_lifecycle = Arc::new(crate::model_runtime::cloud::member_enrollment::InvitationLifecycle::new());
    #[cfg(feature = "os-keychain")]
    let revocation_runtime = Arc::new(crate::storage::surreal::resource_authority::MemberRevocationRuntime::new(control_plane.surreal.clone()));
    #[cfg(feature = "os-keychain")]
    let revocation_lifecycle = Arc::new(member_recipient_startup::MemberRevocationLifecycle::new(Arc::clone(&revocation_runtime)));
    #[cfg(feature = "os-keychain")]
    let member_session_lifecycle = member_recipient_startup::MemberSessionLifecycle::new(control_plane.surreal.clone());
    #[cfg(feature = "os-keychain")]
    let member_enrollment_http_runtime = crate::api::authority::MemberEnrollmentHttpRuntime::new(
        control_plane.surreal.clone(), Arc::clone(&capability_registry), Arc::clone(&invitation_lifecycle),
    );
    #[cfg(feature = "os-keychain")]
    let member_enrollment_http_lifecycle = Arc::new(member_recipient_startup::MemberEnrollmentHttpLifecycle::new(Arc::clone(&member_enrollment_http_runtime)));
    macro_rules! drain_enrollment {
        ($consumer_error:expr) => {{
            #[cfg(feature = "os-keychain")]
            {
                member_enrollment_http_lifecycle.begin_shutdown();
                recipient_lifecycle.begin_shutdown();
                invitation_lifecycle.begin_shutdown();
                revocation_lifecycle.begin_shutdown();
                let http_settled = member_enrollment_http_lifecycle.drain().await;
                let (recipient_settled, invitations_settled, revocations_settled, sessions_settled) = tokio::join!(
                    recipient_lifecycle.drain(), invitation_lifecycle.drain(), revocation_lifecycle.drain(),
                    member_session_lifecycle.drain(),
                );
                if !http_settled || !recipient_settled || !invitations_settled || !revocations_settled || !sessions_settled {
                    retain_failed_runtime_with_consumer(
                        &terminal, "member enrollment retirement remains unproven; backend owner retained", $consumer_error,
                    ).await;
                }
            }
        }};
    }
    let bootstrap_state = crate::AccountBootstrapState::new(
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
    #[cfg(feature = "os-keychain")]
    let app = app
        .layer(axum::Extension(Arc::clone(&recipient_runtime)))
        .layer(axum::Extension(Arc::clone(&invitation_lifecycle)))
        .layer(axum::Extension(Arc::clone(&revocation_runtime)))
        .layer(axum::Extension(Arc::clone(&member_enrollment_http_runtime)));
    let listener = match options.listener {
        Some(listener) => listener,
        None => retain_on_error!(TcpListener::bind(requested_addr).await),
    };
    let actual_addr = retain_on_error!(listener.local_addr());
    retain_on_error!(write_backend_listen_report(actual_addr));
    #[cfg(all(feature = "test-utils", feature = "duckdb-flight-recorder"))]
    let _recorder_proof_attachment = retain_on_error!(attach_recorder_shutdown_proof(
        retain_on_error!(recorder.recorder_proof_control().ok_or("recorder proof control unavailable")),
        shutdown_tx.clone(),
        shutdown_rx.clone(),
    ));
    let server_shutdown_rx = shutdown_rx.clone();
    #[cfg(feature = "os-keychain")]
    let server_enrollment_owners = (Arc::clone(&recipient_lifecycle), Arc::clone(&invitation_lifecycle), Arc::clone(&revocation_lifecycle), Arc::clone(&member_enrollment_http_lifecycle));
    let mut server = tokio::spawn(async move {
        axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
            .with_graceful_shutdown(async move {
                wait_for_shutdown_request(server_shutdown_rx).await;
                #[cfg(feature = "os-keychain")]
                {
                    server_enrollment_owners.3.begin_shutdown();
                    server_enrollment_owners.0.begin_shutdown();
                    server_enrollment_owners.1.begin_shutdown();
                    server_enrollment_owners.2.begin_shutdown();
                }
            })
            .await
    });
    let deadline_shutdown_rx = shutdown_rx.clone();
    let connection_drain_deadline = async move {
        wait_for_shutdown_request(deadline_shutdown_rx).await;
        tokio::time::sleep(HTTP_CONNECTION_DRAIN_TIMEOUT).await;
    };
    tokio::pin!(connection_drain_deadline);
    let mut activation = tokio::select! {
        request = activation_rx.recv() => retain_on_error!(request.ok_or("workspace activation channel closed")),
        result = &mut server => { retain_on_error!(server_result(result)); drain_enrollment!(None); retain_on_error!(control_plane.surreal.shutdown().await); settled.store(true, std::sync::atomic::Ordering::Release); return Ok(()); },
        _ = &mut connection_drain_deadline => { retain_failed_runtime(&terminal, "account HTTP drain timed out").await; },
    };
    let mut consumer_error = None;
    loop {
        if activation.authorize_for_publication().await.is_err() {
            activation.deny();
            activation = tokio::select! {
                request = activation_rx.recv() => retain_on_error!(request.ok_or("workspace activation channel closed")),
                result = &mut server => { retain_on_error!(server_result(result)); break; },
                _ = &mut connection_drain_deadline => { retain_failed_runtime(&terminal, "account HTTP drain timed out").await; },
            };
            continue;
        }
        let product_local_scope = activation.scope().clone();
        let reclaim_resource_scope = retain_on_error!(ReclaimResourceScope::from_exact(product_local_scope.exact()));
        retain_on_error!(perform_startup_recovery(&control_plane.surreal, reclaim_resource_scope.clone()).await);
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
        let atelier_schema = retain_on_error!(atelier.bootstrap_schema().await);
        let atelier_corpus = retain_on_error!(atelier.bootstrap_builtin_command_corpus().await);
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
            retain_on_error!(crate::process_ledger::resolve_embedded_runtime_host_scope());
        let process_runtime = retain_on_error!(ProcessReclaimRuntime::production(
            control_plane.surreal.clone(),
            reclaim_resource_scope,
            None,
            retain_on_error!(crate::process_ledger::production_process_sandbox_registry_async().await),
            runtime_host_scope_id,
            Duration::from_secs(30),
        )
        .await);
        let (model_access_state, cloud, cloud_registry, cli_providers) =
            match model_access_state(&process_runtime, &product_local_scope) {
                Ok(dependencies) => dependencies,
                Err(error) => {
                    tracing::error!(error = %error, "scoped model access initialization failed; no activation ACK published");
                    drain_unpublished_runtime(&process_runtime, None, &terminal).await;
                    activation.unavailable();
                    drop(process_runtime);
                    activation = tokio::select! {
                        request = activation_rx.recv() => retain_on_error!(request.ok_or("workspace activation channel closed")),
                        result = &mut server => { retain_on_error!(server_result(result)); break; },
                        _ = &mut connection_drain_deadline => { retain_failed_runtime(&terminal, "account HTTP drain timed out").await; },
                    };
                    continue;
                }
            };
        let model_registry_authority = ScopedModelRegistryAuthority::new(
            ModelRegistryStore::new(control_plane.surreal.clone()),
            product_local_scope.exact().clone(),
        );
        let initialize_default = init_llm_client(
            flight_recorder.clone(),
            Some(process_runtime.ledger().ledger()),
            Some(model_registry_authority.clone()),
            Some(process_runtime.runtime_instance().clone()),
        );
        let initializes_registry = crate::llm::boot::embedded_runtime_boot_requested_from_env()
            .unwrap_or(false);
        let initialized_default = if initializes_registry {
            activation.run_registry_initializer(initialize_default).await
        } else {
            Ok(initialize_default.await)
        };
        let mut llm_client = match initialized_default {
            Ok(client) => client,
            Err(_) => {
                drain_unpublished_runtime(&process_runtime, None, &terminal).await;
                activation.deny();
                drop(process_runtime);
                activation = tokio::select! {
                    request = activation_rx.recv() => retain_on_error!(request.ok_or("workspace activation channel closed")),
                    result = &mut server => { retain_on_error!(server_result(result)); break; },
                    _ = &mut connection_drain_deadline => { retain_failed_runtime(&terminal, "account HTTP drain timed out").await; },
                };
                continue;
            }
        };
        let mut genie_x_resolver = None;
        let configured_result: Result<(), String> = async {
            if let Some(path) = std::env::var_os("HANDSHAKE_GENIEX_CONFIG_PATH") {
                activation.run_registry_initializer(async {
                    let configured = genie_x::GenieXBootClient::prepare(
                        PathBuf::from(path),
                        llm_client.clone(),
                        model_registry_authority.clone(),
                        process_runtime.ledger().ledger(),
                        flight_recorder.clone(),
                        process_runtime.sandbox_registry(),
                    )
                    .await?;
                    // Retain the full owner before any load, START or persistence can fail.
                    llm_client = configured.clone();
                    configured
                        .initialize(process_runtime.ledger().ledger())
                        .await?;
                    genie_x_resolver = Some(configured.binding_resolver());
                    Ok::<(), String>(())
                }).await.map_err(|_| "configured model initializer authority denied".to_owned())??;
            }
            Ok(())
        }
        .await;
        if let Err(error) = configured_result {
            tracing::error!(target: "handshake_core::model_runtime", error = %error,
                "configured GenieX startup failed; no activation acknowledgement published");
            drain_unpublished_runtime(&process_runtime, Some(llm_client.as_ref()), &terminal).await;
            activation.unavailable();
            drop(llm_client);
            drop(process_runtime);
            activation = tokio::select! {
                request = activation_rx.recv() => retain_on_error!(request.ok_or("workspace activation channel closed")),
                result = &mut server => { retain_on_error!(server_result(result)); break; },
                _ = &mut connection_drain_deadline => { retain_failed_runtime(&terminal, "account HTTP drain timed out").await; },
            };
            continue;
        }
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
                drain_unpublished_runtime(&process_runtime, Some(state.llm_client.as_ref()), &terminal).await;
                activation.unavailable();
                drop(state);
                drop(process_runtime);
                activation = tokio::select! {
                    request = activation_rx.recv() => retain_on_error!(request.ok_or("workspace activation channel closed")),
                    result = &mut server => { retain_on_error!(server_result(result)); break; },
                    _ = &mut connection_drain_deadline => { retain_failed_runtime(&terminal, "account HTTP drain timed out").await; },
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

        let (launch_service, swarm_drain) = crate::swarm_orchestration::production_factory::
            build_scoped_operator_chat_launch_service_with_sandbox_registry_and_lifecycle(
                state.surreal.clone(), state.flight_recorder.clone(), catalog.clone(),
                process_runtime.ledger(), process_runtime.reclaim(), cloud,
                Some(process_runtime.sandbox_registry()), genie_x_resolver,
                state.surreal.resource_access_lifecycle(),
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
        let (consumer_cancel, consumer_cancelled) = tokio::sync::watch::channel(false);
        let mut consumer_work: ConsumerFuture<'_> = match (&consumer, publication_result.is_ok()) {
            (Some(consumer), true) => consumer.consume(&state, consumer_cancelled),
            _ => Box::pin(std::future::pending()),
        };
        let mut consumer_finished = false;
        let mut next_activation = None;
        let (mut serve_result, mut connection_drain_timed_out) = if publication_result.is_err() {
            (None, false)
        } else {
            loop {
                tokio::select! {
                    result = &mut consumer_work, if !consumer_finished => {
                        consumer_finished = true;
                        if let Err(error) = result {
                            consumer_error = Some(error);
                            let _ = shutdown_tx.send(true);
                        }
                    },
                    result = &mut server => break (Some(server_result(result)), false),
                    _ = &mut connection_drain_deadline => break (None, true),
                    request = activation_rx.recv() => match request {
                        Some(request) if consumer_error.is_some() => { request.unavailable(); }
                        Some(request) if request.scope() == &product_local_scope => {
                            let _ = request.acknowledge_with(&runtime_instance, || Ok(())).await;
                        }
                        Some(request) => { next_activation = Some(request); break (None, false); }
                        None => break (None, false),
                    },
                }
            }
        };
        let _ = consumer_cancel.send(true);
        if consumer.is_some() && publication_result.is_ok() && !consumer_finished {
            if let Err(error) = (&mut consumer_work).await {
                consumer_error = Some(error);
                let _ = shutdown_tx.send(true);
            }
        }
        drop(consumer_work);
        if consumer_error.is_some() {
            next_activation = None;
            if serve_result.is_none() && !connection_drain_timed_out {
                (serve_result, connection_drain_timed_out) = tokio::select! {
                    result = &mut server => (Some(server_result(result)), false),
                    _ = &mut connection_drain_deadline => (None, true),
                };
            }
        }
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

        let process_shutdown_requested = *shutdown_rx.borrow();
        if next_activation.is_none()
            && (publication_result.is_ok() || connection_drain_timed_out
                || serve_result.is_some() || process_shutdown_requested)
        {
            drain_enrollment!(consumer_error.as_deref());
        }

        if let Err(error) = api_runtime.quiesce_swarm(Duration::from_secs(35)).await {
            state.llm_client.leave_open_for_reconciliation();
            tracing::error!(target: "handshake_core::swarm_orchestration", error = %error,
                "actual coordinator did not retire; downstream drains and scope replacement are forbidden");
            retain_failed_runtime_with_consumer(
                &terminal, "production runtime retirement failed", consumer_error.as_deref(),
            ).await;
        }

        // 1) Close runtime admission, cancel active workers, and wait for the
        //    worker-owned quiescence guards. Only a proven idle result may emit the
        //    embedded ProcessOwnershipLedger STOP. On failure the client releases
        //    its reserved STOP permits without emitting rows, leaving START open
        //    for next-boot reconciliation.
        let runtime_shutdown = if connection_drain_timed_out {
            Err(crate::llm::LlmError::ProviderError(
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
                    crate::process_ledger::LedgerDrainJoinOutcome::Flushed
                        | crate::process_ledger::LedgerDrainJoinOutcome::AlreadyDrained
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
            retain_failed_runtime_with_consumer(
                &terminal, "production runtime retirement failed", consumer_error.as_deref(),
            ).await;
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
                request = activation_rx.recv() => retain_on_error!(request.ok_or("workspace activation channel closed")),
                result = &mut server => { retain_on_error!(server_result(result)); break; },
                _ = &mut connection_drain_deadline => { retain_failed_runtime(&terminal, "account HTTP drain timed out").await; },
            };
            continue;
        }
        if !server.is_finished() {
            server.abort();
            let _ = server.await;
        }
        drain_enrollment!(consumer_error.as_deref());
        let shutdown_result = control_plane.surreal.shutdown().await;
        if let Err(err) = &shutdown_result {
            tracing::error!(target: "handshake_core::storage", error = %err,
                "embedded SurrealDB store did not close cleanly at shutdown");
            retain_failed_runtime_with_consumer(
                &terminal,
                "embedded SurrealDB store did not close cleanly at shutdown",
                consumer_error.as_deref(),
            ).await;
        } else {
            tracing::info!(target: "handshake_core::storage", "embedded SurrealDB store closed");
        }
        settled.store(true, std::sync::atomic::Ordering::Release);
        if let Some(error) = consumer_error {
            return Err(error.into());
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
    drain_enrollment!(consumer_error.as_deref());
    retain_on_error!(control_plane.surreal.shutdown().await);
    settled.store(true, std::sync::atomic::Ordering::Release);
    Ok(())
}

fn server_result(result: Result<io::Result<()>, tokio::task::JoinError>) -> io::Result<()> {
    result.map_err(io::Error::other)?
}

async fn drain_unpublished_runtime(runtime: &ProcessReclaimRuntime, client: Option<&dyn LlmClient>, terminal: &tokio::sync::watch::Sender<Option<Result<(), String>>>) {
    let runtime_quiesced = match client {
        Some(client) => client.shutdown_gracefully().await.is_ok(),
        None => true,
    };
    let report = runtime.shutdown_and_drain(Duration::from_secs(35)).await;
    let ledger_flushed = matches!(report.ledger,
        crate::process_ledger::LedgerDrainJoinOutcome::Flushed
        | crate::process_ledger::LedgerDrainJoinOutcome::AlreadyDrained);
    if !runtime_quiesced || !report.reclaim_task_quiesced || !ledger_flushed || !report.lease_released {
        if let Some(client) = client { client.leave_open_for_reconciliation(); }
        tracing::error!(runtime_quiesced, ledger_flushed, reclaim_quiesced = report.reclaim_task_quiesced,
            lease_released = report.lease_released, "unpublished runtime did not prove durable retirement");
        retain_failed_runtime(terminal, "unpublished runtime did not prove durable retirement").await;
    }
}


/// Test-only bridge owned by the same parent as the private configured producer. It uses the
/// normal verified host configuration and retains the real owner even when initialize fails.
#[cfg(feature = "test-utils")]
pub struct ConfiguredGenieXFixtureFailure {
    pub reason: String,
    pub retained_client: Option<Arc<dyn LlmClient>>,
}

#[cfg(feature = "test-utils")]
pub async fn load_genuine_configured_geniex_fixture(
    path: PathBuf,
    inner: Arc<dyn LlmClient>,
    authority: ScopedModelRegistryAuthority,
    runtime: &ProcessReclaimRuntime,
    recorder: Arc<dyn FlightRecorder>,
    account: &crate::AccountBootstrapState,
    headers: &axum::http::HeaderMap,
) -> Result<Arc<dyn LlmClient>, ConfiguredGenieXFixtureFailure> {
    let scope = authority.scope().clone();
    crate::api::authority::with_registry_initializer_authority(account, headers, &scope, async {
        let configured = genie_x::GenieXBootClient::prepare(
            path, inner, authority, runtime.ledger().ledger(), recorder, runtime.sandbox_registry(),
        ).await.map_err(|reason| ConfiguredGenieXFixtureFailure { reason, retained_client: None })?;
        if let Err(reason) = configured.initialize(runtime.ledger().ledger()).await {
            return Err(ConfiguredGenieXFixtureFailure { reason, retained_client: Some(configured) });
        }
        Ok(configured as Arc<dyn LlmClient>)
    }).await.map_err(|_| ConfiguredGenieXFixtureFailure {
        reason: "original issued configured-adapter Create denied".into(), retained_client: None,
    })?
}
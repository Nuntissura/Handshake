//! MT-012 embedded-Surreal operator-chat launch, denial, and recovery proof.

mod surreal_test_store_support;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;
use futures::stream;
use serde_json::{json, Value};
use uuid::Uuid;

use handshake_core::api::operator_chat::resolve_operator_chat_lineage;
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::model_runtime::catalog::ModelCatalog;
use handshake_core::model_runtime::registry::RuntimeBinding as RuntimeAdapterBinding;
use handshake_core::model_runtime::{
    BaseModelTag, CancellationToken, Embedding, FinishReason, GenerateRequest, GeneratedToken,
    KvCacheHandle, LoraStackHandle, ModelCapabilities, ModelId, ModelRegistration, ModelRegistry,
    ModelRuntime, ModelRuntimeError, OperatorId, ProviderKind, Score, SteeringHookHandle,
    TokenStream,
};
use handshake_core::process_ledger::{
    LedgerBatcher, LedgerBatcherConfig, NoopOverflowSink, ProcessEngineKind,
    ProcessOwnershipRecordId, ProcessStart, ReclaimResourceScope, SurrealProcessLedgerStore,
};
use handshake_core::storage::surreal::{RowFilter, ScalarValue, SurrealStorage};
use handshake_core::storage::{ModelSession, ModelSessionState};
use handshake_core::swarm_orchestration::model_lane::{
    LaunchAuthority, ModelLaneError, ModelLaneKind, ModelLaneProviderKind, ModelLaneRunRecord,
    ModelLaneStatus, ModelLaneStore, NewModelLaneSelectionAudit, RuntimeBinding,
};
use handshake_core::swarm_orchestration::operator_chat::{
    OperatorChatLaneKind, OperatorChatLaunchService, OperatorChatSelection,
};
use handshake_core::swarm_orchestration::resource_scope::{
    AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, ExactResourceScopeAttribution,
    OwnerAccountId, ResourceAccessContext, ResourceAccessLifecycleRegistry,
    ResourceAccessLifecycleTransitionError, ResourceScope, ScopeDenied, WorkspaceScopeRef,
};
use handshake_core::swarm_orchestration::{
    LiveSession, ModelInstanceId, ModelSessionFactory, RecordingSwarmSink, RunBudget,
    SessionTeardown, SpawnRequest, SwarmConfig, SwarmCoordinator, SwarmError,
};
use handshake_core::workflows::{SessionRegistry, SessionSchedulerConfig};
use surreal_test_store_support::EmbeddedSurrealTestScope;

const WP_ID: &str = "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1";
const MT_ID: &str = "MT-012";

#[derive(Default)]
struct CapturingRecorder {
    events: Mutex<Vec<FlightRecorderEvent>>,
}

#[async_trait]
impl FlightRecorder for CapturingRecorder {
    async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
        self.events.lock().expect("recorder lock").push(event);
        Ok(())
    }

    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        Ok(0)
    }

    async fn list_events(
        &self,
        _filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Ok(self.events.lock().expect("recorder lock").clone())
    }
}

#[derive(Clone, Copy)]
enum RuntimeMode {
    Complete,
    FailAfterPrefix,
    WaitForCancellation,
}

#[derive(Default)]
struct LaunchProbe {
    instance_id: Mutex<Option<ModelInstanceId>>,
    run_id: Mutex<Option<String>>,
    lane_id: Mutex<Option<String>>,
    factory_calls: AtomicUsize,
}

impl LaunchProbe {
    fn record(&self, request: &SpawnRequest) {
        self.factory_calls.fetch_add(1, Ordering::SeqCst);
        *self.instance_id.lock().expect("instance probe") = Some(request.instance_id);
        if let Some(contract) = &request.dexterity_launch {
            *self.run_id.lock().expect("run probe") = Some(contract.run_id.clone());
            *self.lane_id.lock().expect("lane probe") = Some(contract.lane_id.clone());
        }
    }

    fn instance_id(&self) -> Option<ModelInstanceId> {
        *self.instance_id.lock().expect("instance probe")
    }

    fn run_id(&self) -> Option<String> {
        self.run_id.lock().expect("run probe").clone()
    }

    fn lane_id(&self) -> Option<String> {
        self.lane_id.lock().expect("lane probe").clone()
    }
}

struct ProofFactory {
    ledger: LedgerBatcher,
    process_scope: ReclaimResourceScope,
    access: ResourceAccessContext,
    mode: RuntimeMode,
    probe: Arc<LaunchProbe>,
}

#[async_trait]
impl ModelSessionFactory for ProofFactory {
    async fn create(&self, request: &SpawnRequest) -> Result<LiveSession, SwarmError> {
        self.access
            .require_lifecycle_active()
            .map_err(|denied| {
                SwarmError::FactoryFailed(format!(
                    "{}: authenticated resource context is not active",
                    denied.reason_code()
                ))
            })?;
        self.probe.record(request);
        let process_id = ProcessOwnershipRecordId::new_v7();
        let os_pid = 61_000 + request.instance_id.instance;
        let engine = match request.provider {
            Some(ProviderKind::OfficialCli) => ProcessEngineKind::OfficialCliBridge,
            Some(ProviderKind::ByokCloud) => ProcessEngineKind::HelperSubprocess,
            Some(ProviderKind::ExternalCompat) => ProcessEngineKind::ExternalCompat,
            Some(ProviderKind::Local) | None => match request.runtime_binding {
                RuntimeAdapterBinding::Candle => ProcessEngineKind::Candle,
                RuntimeAdapterBinding::LlamaCpp => ProcessEngineKind::LlamaCpp,
            },
        };
        let start = ProcessStart::new(engine, request.owner_role.clone(), request.owner_wp.clone())
            .with_process_uuid(process_id.as_uuid())
            .with_os_pid(os_pid)
            .with_parent_session_id(request.parent_session_id.clone())
            .with_wp_id(request.wp_id.clone().unwrap_or_else(|| WP_ID.to_owned()))
            .with_mt_id(request.mt_id.clone().unwrap_or_else(|| MT_ID.to_owned()))
            .with_metadata_jsonb(process_scope_metadata(&self.process_scope));
        self.ledger
            .record_start(start)
            .map_err(|error| SwarmError::LedgerFailed(error.to_string()))?;

        let model_id = ModelId::new_v7();
        let runtime = Arc::new(ProofRuntime::new(self.mode));
        let teardown: SessionTeardown = Arc::new(|| Box::pin(async { Ok(()) }));
        Ok(LiveSession::new(
            runtime,
            model_id,
            CancellationToken::new(),
            teardown,
            process_id,
            os_pid,
        ))
    }
}

struct ProofRuntime {
    mode: RuntimeMode,
    capabilities: ModelCapabilities,
    kv: KvCacheHandle,
    lora: LoraStackHandle,
    steering: SteeringHookHandle,
}

impl ProofRuntime {
    fn new(mode: RuntimeMode) -> Self {
        Self {
            mode,
            capabilities: ModelCapabilities::default(),
            kv: KvCacheHandle::new("operator-chat-surreal-kv"),
            lora: LoraStackHandle::new("operator-chat-surreal-lora"),
            steering: SteeringHookHandle::new("operator-chat-surreal-steering"),
        }
    }
}

#[async_trait]
impl ModelRuntime for ProofRuntime {
    async fn load(
        &mut self,
        _spec: handshake_core::model_runtime::LoadSpec,
    ) -> Result<ModelId, ModelRuntimeError> {
        Ok(ModelId::new_v7())
    }

    async fn unload(&mut self, _id: ModelId) -> Result<(), ModelRuntimeError> {
        Ok(())
    }

    fn generate(&self, request: GenerateRequest) -> TokenStream {
        let prefix = GeneratedToken {
            token_id: 1,
            text: format!(
                "{}\n",
                json!({
                    "type": "item.completed",
                    "item": {
                        "id": "operator-chat-proof-output",
                        "type": "agent_message",
                        "text": request.prompt.as_str()
                    }
                })
            ),
            logprob: None,
            finish_reason: None,
        };
        match self.mode {
            RuntimeMode::Complete => Box::pin(stream::iter(vec![Ok(prefix)])),
            RuntimeMode::FailAfterPrefix => Box::pin(stream::iter(vec![
                Ok(prefix),
                Err(ModelRuntimeError::GenerateError(
                    "deterministic operator-chat stream failure".to_owned(),
                )),
            ])),
            RuntimeMode::WaitForCancellation => {
                let cancel = request.cancel;
                Box::pin(stream::once(async move {
                    cancel.cancelled().await;
                    Ok(GeneratedToken {
                        token_id: 2,
                        text: String::new(),
                        logprob: None,
                        finish_reason: Some(FinishReason::Cancelled),
                    })
                }))
            }
        }
    }

    async fn score(&self, _id: ModelId, _sequence: Vec<u32>) -> Result<Score, ModelRuntimeError> {
        Ok(Score {
            token_logprobs: Vec::new(),
            mean_logprob: 0.0,
        })
    }

    async fn embed(&self, _id: ModelId, _text: &str) -> Result<Embedding, ModelRuntimeError> {
        Ok(Embedding { vector: Vec::new() })
    }

    fn capabilities(&self, _id: ModelId) -> Result<&ModelCapabilities, ModelRuntimeError> {
        Ok(&self.capabilities)
    }

    fn kv_cache(&self, _id: ModelId) -> Result<KvCacheHandle, ModelRuntimeError> {
        Ok(self.kv.clone())
    }

    fn lora_stack(&self, _id: ModelId) -> Result<LoraStackHandle, ModelRuntimeError> {
        Ok(self.lora.clone())
    }

    fn steering_hooks(&self, _id: ModelId) -> Result<SteeringHookHandle, ModelRuntimeError> {
        Ok(self.steering.clone())
    }

    fn cancel(&self, token: CancellationToken) {
        token.cancel();
    }
}

struct ServiceHarness {
    service: Arc<OperatorChatLaunchService>,
    coordinator: Arc<SwarmCoordinator>,
    probe: Arc<LaunchProbe>,
    drain: handshake_core::process_ledger::ProcessLedgerDrain,
}

fn build_service(
    store: ModelLaneStore,
    process_scope: ReclaimResourceScope,
    mode: RuntimeMode,
    catalog: Arc<ModelCatalog>,
) -> ServiceHarness {
    let (ledger, drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("create bounded ProcessLedger channel");
    let probe = Arc::new(LaunchProbe::default());
    let access = store.access().clone();
    let coordinator = Arc::new(SwarmCoordinator::new_with_model_lane_store(
        SwarmConfig::new(RunBudget::defaulted(8)),
        Arc::new(ProofFactory {
            ledger: ledger.clone(),
            process_scope,
            access,
            mode,
            probe: probe.clone(),
        }),
        Arc::new(RecordingSwarmSink::new()),
        ledger,
        store,
    ));
    let service = Arc::new(OperatorChatLaunchService::new(
        coordinator.clone(),
        catalog,
        Arc::new(CapturingRecorder::default()),
    ));
    ServiceHarness {
        service,
        coordinator,
        probe,
        drain,
    }
}

async fn open_scope(
    label: &str,
) -> (
    EmbeddedSurrealTestScope,
    SurrealStorage,
    ResourceScope,
    ReclaimResourceScope,
    Arc<SurrealProcessLedgerStore>,
) {
    let mut isolated = EmbeddedSurrealTestScope::create()
        .await
        .expect("allocate isolated embedded Surreal scope");
    let storage = isolated
        .activate_storage()
        .await
        .expect("activate injected SurrealStorage");
    let scope = exact_scope(label);
    let process_scope = process_scope(&scope);
    let process_store = Arc::new(
        SurrealProcessLedgerStore::open(storage.clone())
            .await
            .expect("open ProcessLedger on the same injected storage"),
    );
    (isolated, storage, scope, process_scope, process_store)
}

fn exact_scope(label: &str) -> ResourceScope {
    ResourceScope::new(OwnerAccountId::mint(), ActorPrincipalId::mint())
        .with_session(AuthenticatedSessionRef::mint())
        .with_access_space(AccessSpaceRef::mint())
        .with_workspace(
            WorkspaceScopeRef::new(format!("workspace-operator-chat-{label}"))
                .expect("nonblank workspace"),
        )
}

fn active_store(
    storage: SurrealStorage,
    scope: ResourceScope,
) -> (ModelLaneStore, ResourceAccessLifecycleRegistry) {
    let exact = ExactResourceScopeAttribution::try_from_resource_scope(&scope)
        .expect("active store requires an exact scope");
    let lifecycle = ResourceAccessLifecycleRegistry::new();
    lifecycle
        .register_active(exact)
        .expect("register active resource context");
    (
        ModelLaneStore::new_scoped_with_lifecycle(storage, scope, lifecycle.clone()),
        lifecycle,
    )
}

fn process_scope(scope: &ResourceScope) -> ReclaimResourceScope {
    ReclaimResourceScope {
        account_uuid: scope.owner_account_id.as_uuid(),
        actor_uuid: scope.actor_principal_id.as_uuid(),
        session_uuid: scope
            .authenticated_session
            .expect("exact scope session")
            .as_uuid(),
        workspace_id: scope
            .workspace
            .as_ref()
            .expect("exact scope workspace")
            .as_str()
            .to_owned(),
        access_space_uuid: scope
            .access_space
            .expect("exact scope access space")
            .as_uuid(),
    }
}

fn process_scope_metadata(scope: &ReclaimResourceScope) -> Value {
    json!({
        "owner_account_id": scope.account_uuid.to_string(),
        "actor_principal_id": scope.actor_uuid.to_string(),
        "authenticated_session_id": scope.session_uuid.to_string(),
        "access_space_id": scope.access_space_uuid.to_string(),
        "workspace_id": scope.workspace_id,
    })
}

fn selection(
    lane_kind: OperatorChatLaneKind,
    model_id: impl Into<String>,
    owner: &str,
) -> OperatorChatSelection {
    let (cloud_provider, cli_provider) = match lane_kind {
        OperatorChatLaneKind::Cloud => (Some("anthropic".to_owned()), None),
        OperatorChatLaneKind::Cli => (None, Some("codex".to_owned())),
        OperatorChatLaneKind::Local | OperatorChatLaneKind::Subagent => (None, None),
    };
    OperatorChatSelection {
        lane_kind,
        model_id: model_id.into(),
        cloud_provider,
        cli_provider,
        working_dir: env!("CARGO_MANIFEST_DIR").to_owned(),
        worktree_id: Some("wt-operator-chat-surreal".to_owned()),
        prompt: format!("operator-chat proof prompt for {owner}"),
        owner_session: owner.to_owned(),
        parent_session_id: "operator-chat-parent".to_owned(),
        work_packet_id: Some(WP_ID.to_owned()),
        micro_task_id: Some(MT_ID.to_owned()),
    }
}

fn selection_audit_for_run(
    run: &ModelLaneRunRecord,
    selected_model_id: &str,
    suffix: &str,
) -> NewModelLaneSelectionAudit {
    NewModelLaneSelectionAudit {
        audit_id: format!("selection-audit-{suffix}"),
        run_id: run.run_id.clone(),
        selected_model_id: selected_model_id.to_owned(),
        actor_ref: "principal://operator-chat-proof".to_owned(),
        reason: "operator_chat_lifecycle_proof".to_owned(),
        selection_context: json!({"source": "operator_chat_surreal_tests"}),
        event_ledger_stream_id: run.event_ledger_stream_id.clone(),
        work_packet_id: run.work_packet_id.clone().expect("run WP Locus"),
        micro_task_id: run.micro_task_id.clone().expect("run MT Locus"),
        task_board_id: run.task_board_id.clone().expect("run task-board Locus"),
        owner_session: run.owner_session.clone(),
        idempotency_key: format!("idem-selection-audit-{suffix}"),
        created_at_utc: Utc::now().to_rfc3339(),
    }
}

fn registered_local_catalog() -> (Arc<ModelCatalog>, String) {
    let model_id = ModelId::new_v7();
    let mut registry = ModelRegistry::default();
    registry
        .register(ModelRegistration {
            model_id,
            artifact_path: "operator-chat-surreal-proof.gguf".into(),
            sha256: [42; 32],
            runtime_binding: RuntimeAdapterBinding::Candle,
            declared_capabilities: ModelCapabilities::default(),
            base_model_tag: BaseModelTag::new("Operator Chat Surreal Proof"),
            registered_at_utc: Utc::now(),
            registered_by: OperatorId::new("operator-chat-surreal-test"),
            provider: ProviderKind::Local,
        })
        .expect("register local proof model");
    registry
        .mark_loaded(model_id)
        .expect("mark local model ready");
    (
        ModelCatalog::from_registry(Arc::new(registry)),
        model_id.to_string(),
    )
}

async fn table_count(storage: &SurrealStorage, table_name: &str) -> u64 {
    let inspector = storage.test_inspector();
    let table = inspector
        .table_selector(table_name)
        .await
        .unwrap_or_else(|error| panic!("select {table_name}: {error}"));
    inspector
        .row_count(&table, RowFilter::All)
        .await
        .unwrap_or_else(|error| panic!("count {table_name}: {error}"))
}

async fn event_exists(storage: &SurrealStorage, event_id: &str) -> bool {
    let inspector = storage.test_inspector();
    let table = inspector
        .table_selector("kernel_event_ledger")
        .await
        .expect("select canonical EventLedger");
    let field = table.field("event_id").expect("EventLedger event_id field");
    inspector
        .exists(
            &table,
            RowFilter::FieldEquals {
                field,
                value: ScalarValue::from(event_id),
            },
        )
        .await
        .expect("inspect canonical EventLedger event")
}

async fn cleanup_scope(mut isolated: EmbeddedSurrealTestScope, storage: SurrealStorage) {
    drop(storage);
    isolated
        .shutdown_storage_for_reopen()
        .await
        .expect("shutdown isolated storage");
    let diagnostics = isolated.cleanup().await.expect("clean isolated scope");
    assert!(diagnostics.database_absent);
    assert!(diagnostics.namespace_absent_after_reopen);
    assert!(diagnostics.error.is_none());
}

#[tokio::test]
async fn operator_chat_surreal_launch_capture_scope_restart_and_eventledger() {
    let (mut isolated, storage, scope, process_scope, process_store) =
        open_scope("launch-restart").await;
    let namespace = isolated.namespace().to_owned();
    let database = isolated.database().to_owned();
    let (store, lifecycle) = active_store(storage.clone(), scope.clone());
    let harness = build_service(
        store.clone(),
        process_scope.clone(),
        RuntimeMode::Complete,
        ModelCatalog::empty(),
    );
    let launched = harness
        .service
        .launch(&selection(
            OperatorChatLaneKind::Cli,
            "gpt-5-codex",
            "owner-launch-restart",
        ))
        .await
        .expect("launch and capture through the Surreal-backed service");
    assert_eq!(launched.captured_message_count, 1);
    let replay = store
        .replay_run(&launched.run_id)
        .await
        .expect("replay launched run in exact scope");
    assert_eq!(
        replay.messages.len(),
        2,
        "operator prompt plus model output"
    );
    for record in replay
        .messages
        .iter()
        .map(|message| (&message.event_ledger_event_id, message.event_ledger_seq))
        .chain(std::iter::once((
            &replay.run.event_ledger_event_id,
            replay.run.event_ledger_seq,
        )))
    {
        assert!(!record.0.is_empty());
        assert!(record.1 > 0);
        assert!(event_exists(&storage, record.0).await);
    }
    let process_uuid = replay
        .lanes
        .iter()
        .find_map(|lane| lane.process_ownership_ref.as_deref())
        .and_then(|reference| reference.strip_prefix("process-ledger://"))
        .and_then(|value| Uuid::parse_str(value).ok())
        .expect("process-backed lane carries a ProcessLedger UUID");
    harness
        .drain
        .drain_available_to(process_store.clone())
        .await
        .expect("flush ProcessLedger START/STOP to the same SurrealStorage");
    let ownership = process_store
        .inspect_ownership_by_process_uuid(&process_scope, process_uuid)
        .await
        .expect("inspect exact-scope process authority")
        .expect("process lifecycle exists");
    assert!(ownership.stopped_at.is_some());
    assert_eq!(ownership.resource_scope, process_scope);
    assert!(table_count(&storage, "kernel_event_ledger").await > 0);

    drop(harness.service);
    drop(harness.coordinator);
    drop(harness.probe);
    drop(store);
    drop(process_store);
    drop(storage);
    isolated
        .shutdown_storage_for_reopen()
        .await
        .expect("close original storage before restart");
    isolated
        .reopen()
        .await
        .expect("reopen original embedded scope");
    assert_eq!(isolated.namespace(), namespace);
    assert_eq!(isolated.database(), database);
    let restarted_storage = isolated
        .activate_storage()
        .await
        .expect("reactivate the same namespace and database");
    let restarted_store =
        ModelLaneStore::new_scoped_with_lifecycle(restarted_storage.clone(), scope, lifecycle);
    let restarted = restarted_store
        .replay_run(&launched.run_id)
        .await
        .expect("replay survives restart on the same storage identity");
    assert_eq!(restarted.messages, replay.messages);
    drop(restarted_store);
    cleanup_scope(isolated, restarted_storage).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn operator_chat_surreal_cancellation_failure_retry_and_cleanup_recovery() {
    let (isolated, storage, scope, process_scope, process_store) = open_scope("terminal").await;
    let (store, _lifecycle) = active_store(storage.clone(), scope);

    let failing = build_service(
        store.clone(),
        process_scope.clone(),
        RuntimeMode::FailAfterPrefix,
        ModelCatalog::empty(),
    );
    let terminal_control = store.test_terminal_commit_control();
    terminal_control.fail_next();
    let failed = failing
        .service
        .launch(&selection(
            OperatorChatLaneKind::Cli,
            "gpt-5-codex",
            "owner-failure",
        ))
        .await
        .expect_err("stream failure must fail the launch");
    assert!(
        failed.to_string().contains("stream failed")
            || failed.to_string().contains("terminal commit failure")
    );
    assert_eq!(failing.coordinator.live_session_count(), 0);
    let failed_run_id = failing.probe.run_id().expect("failed run id captured");
    let failed_lane_id = failing.probe.lane_id().expect("failed lane id captured");
    store
        .record_lane_terminal_status(
            &failed_lane_id,
            ModelLaneStatus::Cancelled,
            "retry after injected fail-once",
        )
        .await
        .expect("fail-once leaves the terminal commit retryable");
    let failed_replay = store
        .replay_run(&failed_run_id)
        .await
        .expect("failed run remains replayable");
    assert_eq!(failed_replay.messages.len(), 2);
    assert!(failed_replay
        .lanes
        .iter()
        .any(|lane| lane.status == ModelLaneStatus::Cancelled));
    failing
        .drain
        .drain_available_to(process_store.clone())
        .await
        .expect("flush failed-launch process lifecycle");

    let cancelling = build_service(
        store.clone(),
        process_scope.clone(),
        RuntimeMode::WaitForCancellation,
        ModelCatalog::empty(),
    );
    let cancel_selection = selection(
        OperatorChatLaneKind::Cli,
        "gpt-5-codex",
        "owner-cancellation",
    );
    let launch_task = {
        let service = cancelling.service.clone();
        tokio::spawn(async move { service.launch(&cancel_selection).await })
    };
    let instance_id = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(instance_id) = cancelling.probe.instance_id() {
                break instance_id;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("launch reaches coordinator before cancellation");
    let receipts_before_pause = store
        .test_cleanup_receipts_bounded(64)
        .await
        .expect("bounded cleanup receipt baseline");
    terminal_control.pause_next();
    let cancel_task = {
        let coordinator = cancelling.coordinator.clone();
        tokio::spawn(async move {
            coordinator
                .cancel_session(instance_id, "operator-requested-cancellation")
                .await
        })
    };
    tokio::time::timeout(Duration::from_secs(5), terminal_control.wait_until_paused())
        .await
        .expect("terminal commit reaches deterministic pause");
    assert_eq!(
        store
            .test_cleanup_receipts_bounded(64)
            .await
            .expect("pause occurs before cleanup receipt mutation"),
        receipts_before_pause
    );
    terminal_control.release_paused();
    cancel_task
        .await
        .expect("paused cancellation task joins")
        .expect("coordinator commits cancellation after release");
    let cancel_error = launch_task
        .await
        .expect("cancelled launch task joins")
        .expect_err("cancelled launch cannot report success");
    assert!(
        cancel_error.to_string().contains("cancelled")
            || cancel_error.to_string().contains("terminal")
    );
    assert_eq!(cancelling.coordinator.live_session_count(), 0);
    cancelling
        .drain
        .drain_available_to(process_store.clone())
        .await
        .expect("flush cancelled process lifecycle");

    let retry = build_service(
        store.clone(),
        process_scope,
        RuntimeMode::Complete,
        ModelCatalog::empty(),
    );
    let retried = retry
        .service
        .launch(&selection(
            OperatorChatLaneKind::Cli,
            "gpt-5-codex",
            "owner-cancellation",
        ))
        .await
        .expect("fresh retry after terminal cleanup succeeds");
    assert_eq!(retried.captured_message_count, 1);
    retry
        .drain
        .drain_available_to(process_store)
        .await
        .expect("flush retry process lifecycle");
    assert!(
        table_count(&storage, "model_lane_authority").await >= 1,
        "terminal paths leave durable cleanup and replay authority"
    );
    let bounded_cleanup = store
        .test_cleanup_receipts_bounded(64)
        .await
        .expect("cleanup receipts remain bounded and inspectable");
    assert!(bounded_cleanup.len() <= 64);
    for receipt in &bounded_cleanup {
        assert_ne!(receipt.terminal_event_id, Uuid::nil());
        assert_ne!(receipt.resource_evicted_event_id, Uuid::nil());
    }
    let scoped_receipts = store
        .test_scoped_authority_receipts(&retried.run_id, 64)
        .await
        .expect("terminal retry retains scoped canonical authority receipts");
    assert!(!scoped_receipts.is_empty());
    assert!(scoped_receipts.len() <= 64);
    drop(failing);
    drop(cancelling);
    drop(retry);
    drop(store);
    cleanup_scope(isolated, storage).await;
}

#[tokio::test]
async fn operator_chat_surreal_five_field_stale_revoked_denials_are_non_mutating() {
    let (isolated, storage, scope, process_resource_scope, process_store) =
        open_scope("denials").await;
    let (store, owner_lifecycle) = active_store(storage.clone(), scope.clone());
    let harness = build_service(
        store.clone(),
        process_resource_scope,
        RuntimeMode::Complete,
        ModelCatalog::empty(),
    );
    let launched = harness
        .service
        .launch(&selection(
            OperatorChatLaneKind::Cli,
            "gpt-5-codex",
            "owner-denials",
        ))
        .await
        .expect("seed exact-scope operator-chat run");
    harness
        .drain
        .drain_available_to(process_store)
        .await
        .expect("flush seed process lifecycle");
    let seeded = store
        .replay_run(&launched.run_id)
        .await
        .expect("seed run remains readable while active");
    let authority_before = table_count(&storage, "model_lane_authority").await;
    let events_before = table_count(&storage, "kernel_event_ledger").await;
    let processes_before = table_count(&storage, "kernel_process_lifecycle").await;
    let cleanup_before = store
        .test_cleanup_receipts_bounded(64)
        .await
        .expect("baseline bounded cleanup receipts");
    let receipts_before = store
        .test_scoped_authority_receipts(&launched.run_id, 64)
        .await
        .expect("baseline scoped authority receipts");
    let provider_calls_before = harness.probe.factory_calls.load(Ordering::SeqCst);

    let mut mismatches = Vec::new();
    let mut owner = scope.clone();
    owner.owner_account_id = OwnerAccountId::mint();
    mismatches.push(("owner_account_id", owner));
    let mut actor = scope.clone();
    actor.actor_principal_id = ActorPrincipalId::mint();
    mismatches.push(("actor_principal_id", actor));
    let mut session = scope.clone();
    session.authenticated_session = Some(AuthenticatedSessionRef::mint());
    mismatches.push(("authenticated_session_id", session));
    let mut access = scope.clone();
    access.access_space = Some(AccessSpaceRef::mint());
    mismatches.push(("access_space_id", access));
    let mut workspace = scope.clone();
    workspace.workspace =
        Some(WorkspaceScopeRef::new("workspace-operator-chat-foreign").expect("foreign workspace"));
    mismatches.push(("workspace_id", workspace));
    for (dimension, mismatch) in mismatches {
        let (denied, _mismatch_lifecycle) = active_store(storage.clone(), mismatch);
        let error = denied
            .replay_run(&launched.run_id)
            .await
            .expect_err("one-field mismatch must not replay the run");
        let detail = error.to_string();
        assert!(
            !detail.contains(scope.workspace.as_ref().expect("workspace").as_str())
                && !detail.contains(&scope.owner_account_id.to_string()),
            "{dimension} denial leaked stored scope metadata: {detail}"
        );
    }
    let incomplete = ResourceScope::new(scope.owner_account_id, scope.actor_principal_id)
        .with_workspace(scope.workspace.clone().expect("workspace"));
    let incomplete_error = ModelLaneStore::new_scoped(storage.clone(), incomplete)
        .replay_run(&launched.run_id)
        .await
        .expect_err("incomplete exact scope must fail before querying");
    assert!(matches!(
        incomplete_error,
        ModelLaneError::ScopeDenied(ScopeDenied::LifecycleUnknown)
    ));

    let exact =
        ExactResourceScopeAttribution::try_from_resource_scope(&scope).expect("exact denial tuple");
    owner_lifecycle
        .mark_stale(&exact)
        .expect("mark active tuple stale");
    let stale_read = store
        .replay_run(&launched.run_id)
        .await
        .expect_err("stale context must deny reads");
    assert!(matches!(
        stale_read,
        ModelLaneError::ScopeDenied(ScopeDenied::LifecycleStale)
    ));
    let stale_audit = store
        .record_selection_audit_atomic(selection_audit_for_run(&seeded.run, "gpt-5-codex", "stale"))
        .await
        .expect_err("stale context must deny selection audit writes");
    assert!(matches!(
        stale_audit,
        ModelLaneError::ScopeDenied(ScopeDenied::LifecycleStale)
    ));
    let stale_launch = harness
        .service
        .launch(&selection(
            OperatorChatLaneKind::Cli,
            "gpt-5-codex",
            "owner-stale-denial",
        ))
        .await
        .expect_err("stale context must deny before provider launch");
    assert!(!stale_launch.to_string().contains(&launched.run_id));
    assert_eq!(
        harness.probe.factory_calls.load(Ordering::SeqCst),
        provider_calls_before
    );

    let revoked_lifecycle = active_store(storage.clone(), scope.clone()).1;
    let revoked_store = ModelLaneStore::new_scoped_with_lifecycle(
        storage.clone(),
        scope.clone(),
        revoked_lifecycle.clone(),
    );
    let revoked_harness = build_service(
        revoked_store.clone(),
        process_scope(&scope),
        RuntimeMode::Complete,
        ModelCatalog::empty(),
    );
    revoked_lifecycle
        .revoke(&exact)
        .expect("revoke active exact tuple");
    assert!(matches!(
        revoked_store.replay_run(&launched.run_id).await,
        Err(ModelLaneError::ScopeDenied(ScopeDenied::LifecycleRevoked))
    ));
    assert!(matches!(
        revoked_store
            .record_selection_audit_atomic(selection_audit_for_run(
                &seeded.run,
                "gpt-5-codex",
                "revoked",
            ))
            .await,
        Err(ModelLaneError::ScopeDenied(ScopeDenied::LifecycleRevoked))
    ));
    let revoked_launch = revoked_harness
        .service
        .launch(&selection(
            OperatorChatLaneKind::Cli,
            "gpt-5-codex",
            "owner-revoked-denial",
        ))
        .await
        .expect_err("revoked context must deny before provider launch");
    assert!(!revoked_launch.to_string().contains(&launched.run_id));
    assert_eq!(
        revoked_harness.probe.factory_calls.load(Ordering::SeqCst),
        0
    );
    assert_eq!(
        revoked_lifecycle.register_active(exact.clone()),
        Err(ResourceAccessLifecycleTransitionError::TerminalContext)
    );
    let mut new_session_scope = scope.clone();
    new_session_scope.authenticated_session = Some(AuthenticatedSessionRef::mint());
    revoked_lifecycle
        .register_active(
            ExactResourceScopeAttribution::try_from_resource_scope(&new_session_scope)
                .expect("new exact session"),
        )
        .expect("new session identity may register active");

    let (verifier, _verifier_lifecycle) = active_store(storage.clone(), scope.clone());
    assert_eq!(
        table_count(&storage, "model_lane_authority").await,
        authority_before
    );
    assert_eq!(
        table_count(&storage, "kernel_event_ledger").await,
        events_before
    );
    assert_eq!(
        table_count(&storage, "kernel_process_lifecycle").await,
        processes_before
    );
    assert_eq!(
        verifier
            .test_cleanup_receipts_bounded(64)
            .await
            .expect("denials do not create cleanup receipts"),
        cleanup_before
    );
    assert_eq!(
        verifier
            .test_scoped_authority_receipts(&launched.run_id, 64)
            .await
            .expect("denials do not create scoped authority receipts"),
        receipts_before
    );
    drop(revoked_harness);
    drop(revoked_store);
    drop(harness);
    drop(store);
    cleanup_scope(isolated, storage).await;
}

#[tokio::test]
async fn operator_chat_surreal_route_local_cloud_cli_subagent_end_to_end() {
    let (isolated, storage, scope, process_scope, process_store) = open_scope("all-routes").await;
    let exact = ExactResourceScopeAttribution::try_from_resource_scope(&scope)
        .expect("all five scope dimensions");
    let (store, _lifecycle) = active_store(storage.clone(), scope);
    let (catalog, local_model_id) = registered_local_catalog();
    let harness = build_service(
        store.clone(),
        process_scope.clone(),
        RuntimeMode::Complete,
        catalog,
    );

    let local = harness
        .service
        .launch(&selection(
            OperatorChatLaneKind::Local,
            local_model_id,
            "owner-local",
        ))
        .await
        .expect("local route launches");
    let cloud = harness
        .service
        .launch_scoped(
            &selection(
                OperatorChatLaneKind::Cloud,
                "claude-sonnet-4-byok",
                "owner-cloud",
            ),
            &exact,
        )
        .await
        .expect("cloud route launches with exact scope");
    let cli = harness
        .service
        .launch(&selection(
            OperatorChatLaneKind::Cli,
            "gpt-5-codex",
            "owner-cli",
        ))
        .await
        .expect("CLI route launches");
    let subagent = harness
        .service
        .launch(&selection(
            OperatorChatLaneKind::Subagent,
            "subagent://operator-chat/coder",
            "owner-subagent",
        ))
        .await
        .expect("subagent no-OS route launches");

    for (launched, expected_kind, expected_binding, expected_authority, expected_provider) in [
        (
            &local,
            ModelLaneKind::LocalModel,
            RuntimeBinding::Local,
            LaunchAuthority::ModelRuntime,
            ModelLaneProviderKind::LocalRuntime,
        ),
        (
            &cloud,
            ModelLaneKind::CloudModel,
            RuntimeBinding::Cloud,
            LaunchAuthority::CloudLane,
            ModelLaneProviderKind::Anthropic,
        ),
        (
            &cli,
            ModelLaneKind::CliModel,
            RuntimeBinding::CliBridge,
            LaunchAuthority::CliBridge,
            ModelLaneProviderKind::OfficialCli,
        ),
        (
            &subagent,
            ModelLaneKind::Subagent,
            RuntimeBinding::Subagent,
            LaunchAuthority::SubagentManager,
            ModelLaneProviderKind::Subagent,
        ),
    ] {
        let replay = store
            .replay_run(&launched.run_id)
            .await
            .expect("route run replays from the injected store");
        let lane = replay
            .lanes
            .iter()
            .find(|lane| lane.kind == expected_kind)
            .expect("expected routed lane persisted");
        assert_eq!(lane.runtime_binding, expected_binding);
        assert_eq!(lane.launch_authority, expected_authority);
        assert_eq!(lane.provider_kind, expected_provider);
        assert!(replay
            .messages
            .iter()
            .any(|message| message.diagnostic_payload["turn_role"] == json!("operator")));
        assert!(event_exists(&storage, &replay.run.event_ledger_event_id).await);
        if expected_kind == ModelLaneKind::Subagent {
            assert!(lane.process_ownership_ref.is_none());
            assert!(launched.instance_id.starts_with("no-os:"));
        } else {
            assert!(lane.process_ownership_ref.is_some());
        }
    }
    assert_eq!(harness.probe.factory_calls.load(Ordering::SeqCst), 3);
    harness
        .drain
        .drain_available_to(process_store.clone())
        .await
        .expect("flush three process-backed route lifecycles");
    assert_eq!(
        table_count(&storage, "kernel_process_lifecycle").await,
        3,
        "subagent route must not fabricate a process lifecycle"
    );
    for launched in [&local, &cloud, &cli] {
        let replay = store
            .replay_run(&launched.run_id)
            .await
            .expect("route replay");
        let process_uuid = replay
            .lanes
            .iter()
            .find_map(|lane| lane.process_ownership_ref.as_deref())
            .and_then(|reference| reference.strip_prefix("process-ledger://"))
            .and_then(|value| Uuid::parse_str(value).ok())
            .expect("process-backed route reference");
        let ownership = process_store
            .inspect_ownership_by_process_uuid(&process_scope, process_uuid)
            .await
            .expect("inspect route ProcessLedger receipt")
            .expect("route ProcessLedger row exists");
        assert!(ownership.stopped_at.is_some());
    }
    drop(harness);
    drop(store);
    drop(process_store);
    cleanup_scope(isolated, storage).await;
}

fn registry_session(
    session_id: &str,
    parent_session_id: Option<&str>,
    spawn_depth: i32,
    state: ModelSessionState,
) -> ModelSession {
    ModelSession {
        session_id: session_id.to_owned(),
        parent_session_id: parent_session_id.map(str::to_owned),
        spawn_depth,
        state,
        model_id: "operator-chat-test-model".to_owned(),
        backend: "embedded".to_owned(),
        parameter_class: "standard".to_owned(),
        role: "CODER".to_owned(),
        wp_id: Some(WP_ID.to_owned()),
        mt_id: Some(MT_ID.to_owned()),
        work_profile_id: None,
        execution_mode: "delegated".to_owned(),
        memory_policy: "SESSION_SCOPED".to_owned(),
        consent_receipt_id: None,
        capability_grants: Vec::new(),
        capability_token_ids: None,
        job_id: None,
        checkpoint_artifact_id: None,
        last_checkpoint_at: None,
        checkpoint_count: 0,
        merge_back_artifact: None,
        agent: None,
        purpose: None,
        close_reason: None,
        closed_by_actor: None,
        closed_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

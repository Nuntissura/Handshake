//! WP-1 MT-013 V6 deterministic embedded-model lifecycle and reclaim proof.
//!
//! Every durable operation in this target uses one injected embedded
//! `SurrealStorage` clone and one exact five-field `ReclaimResourceScope`.
//! The ignored real-Candle load remains the separate production-realism gate.

mod process_ledger_surreal_support;

use std::{collections::BTreeSet, sync::Arc, time::Duration};

use async_trait::async_trait;
use chrono::Utc;
use handshake_core::{
    flight_recorder::{EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError},
    llm::{boot::assemble_local_runtime_client, DisabledLlmClient, LlmClient},
    model_runtime::{
        BaseModelTag, CancellationToken, Embedding, GenerateRequest, KvCacheHandle, LoadSpec,
        LoraStackHandle, ModelCapabilities, ModelId, ModelRegistration, ModelRuntime,
        ModelRuntimeError, OperatorId, ProviderKind, RuntimeBinding, RuntimeQuiesceError, Score,
        SteeringHookHandle, TokenStream,
    },
    process_ledger::{
        acquire_embedded_runtime_instance_lease, drain_and_join_ledger_writer, LedgerBatcher,
        LedgerBatcherConfig, LedgerDrainJoinOutcome, LedgerEvent, NoopOverflowSink,
        ProcessEngineKind, ProcessLedgerError, ProcessLedgerStore, ProcessRuntimeOwner,
        ProcessStart, ProcessStop, ReclaimProcessStore, ReclaimResourceScope, StaleSessionSource,
        SurrealModelLaneStaleSessionSource, SurrealProcessLedgerStore,
    },
    storage::surreal::SurrealStorage,
};
use process_ledger_surreal_support::{scope_metadata, ProcessLedgerSurrealHarness};
use serde_json::{json, Value};
use surrealdb::types::{RecordId, SurrealValue};
use uuid::Uuid;

use handshake_core::process_ledger::restart_resume::SurrealRestartResumeRunner;

#[derive(Default)]
struct NoopRecorder;

#[async_trait]
impl FlightRecorder for NoopRecorder {
    async fn record_event(&self, _event: FlightRecorderEvent) -> Result<(), RecorderError> {
        Ok(())
    }

    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        Ok(0)
    }

    async fn list_events(
        &self,
        _filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Ok(Vec::new())
    }
}

enum TestUnload {
    Immediate,
    Fail(String),
}

struct NoopRuntime {
    capabilities: ModelCapabilities,
    unload: TestUnload,
}

impl NoopRuntime {
    fn ready() -> Self {
        Self {
            capabilities: ModelCapabilities::default(),
            unload: TestUnload::Immediate,
        }
    }

    fn failing_unload(reason: &str) -> Self {
        Self {
            capabilities: ModelCapabilities::default(),
            unload: TestUnload::Fail(reason.to_owned()),
        }
    }
}

#[async_trait]
impl ModelRuntime for NoopRuntime {
    async fn load(&mut self, _spec: LoadSpec) -> Result<ModelId, ModelRuntimeError> {
        Ok(ModelId::new_v7())
    }

    async fn unload(&mut self, _id: ModelId) -> Result<(), ModelRuntimeError> {
        match &self.unload {
            TestUnload::Immediate => Ok(()),
            TestUnload::Fail(reason) => Err(ModelRuntimeError::UnloadError(reason.clone())),
        }
    }

    async fn generate(
        &self,
        _id: ModelId,
        _request: GenerateRequest,
    ) -> Result<TokenStream, ModelRuntimeError> {
        Ok(Box::pin(futures::stream::empty()))
    }

    async fn score(
        &self,
        _id: ModelId,
        _prefix: &str,
        _continuation: &str,
    ) -> Result<Score, ModelRuntimeError> {
        Ok(Score {
            log_probability: 0.0,
        })
    }

    async fn embed(&self, _id: ModelId, _text: &str) -> Result<Embedding, ModelRuntimeError> {
        Ok(Embedding { vector: Vec::new() })
    }

    async fn quiesce(&self, _timeout: Duration) -> Result<(), RuntimeQuiesceError> {
        Ok(())
    }

    fn capabilities(&self, _id: ModelId) -> Result<&ModelCapabilities, ModelRuntimeError> {
        Ok(&self.capabilities)
    }

    fn kv_cache(&self, _id: ModelId) -> Result<KvCacheHandle, ModelRuntimeError> {
        Ok(KvCacheHandle::new("mt013-noop-kv"))
    }

    fn lora_stack(&self, _id: ModelId) -> Result<LoraStackHandle, ModelRuntimeError> {
        Ok(LoraStackHandle::new("mt013-noop-lora"))
    }

    fn steering_hooks(&self, _id: ModelId) -> Result<SteeringHookHandle, ModelRuntimeError> {
        Ok(SteeringHookHandle::new("mt013-noop-steering"))
    }

    fn cancel(&self, token: CancellationToken) {
        token.cancel();
    }
}

/// The test-only assembly helper lacks the production model-registry scope
/// parameter. This adapter stamps the already-authoritative test scope before
/// forwarding to the real Surreal provider; it cannot bypass provider checks.
struct ExactScopeForwardingStore {
    inner: Arc<SurrealProcessLedgerStore>,
    scope: ReclaimResourceScope,
}

#[async_trait]
impl ProcessLedgerStore for ExactScopeForwardingStore {
    async fn preflight(&self) -> Result<(), ProcessLedgerError> {
        self.inner.preflight().await
    }

    async fn write_batch(&self, mut events: Vec<LedgerEvent>) -> Result<(), ProcessLedgerError> {
        let scope = scope_metadata(&self.scope);
        let scope = scope.as_object().expect("scope metadata is an object");
        for event in &mut events {
            let metadata = match event {
                LedgerEvent::Start(start) => &mut start.metadata_jsonb,
                LedgerEvent::Stop(stop) => &mut stop.metadata_jsonb,
            };
            let object = metadata.as_object_mut().ok_or_else(|| {
                ProcessLedgerError::InvalidConfig(
                    "embedded lifecycle metadata must be an object".to_owned(),
                )
            })?;
            object.extend(scope.clone());
        }
        self.inner.write_batch(events).await
    }
}

fn embedded_registration(model_id: ModelId) -> ModelRegistration {
    ModelRegistration {
        model_id,
        artifact_path: "fixtures/models/embedded-default.safetensors".into(),
        sha256: [0x13; 32],
        runtime_binding: RuntimeBinding::Candle,
        declared_capabilities: ModelCapabilities::default(),
        base_model_tag: BaseModelTag::new("mt013-surreal-embedded-model"),
        registered_at_utc: Utc::now(),
        registered_by: OperatorId::new("mt013-surreal-proof"),
        provider: ProviderKind::Local,
    }
}

fn scoped_store(harness: &ProcessLedgerSurrealHarness) -> Arc<ExactScopeForwardingStore> {
    Arc::new(ExactScopeForwardingStore {
        inner: harness.store(),
        scope: harness.resource_scope().clone(),
    })
}

fn local_client(
    model_id: ModelId,
    ledger: LedgerBatcher,
    candle: Arc<dyn ModelRuntime>,
) -> Arc<dyn LlmClient> {
    Arc::new(
        assemble_local_runtime_client(
            embedded_registration(model_id),
            Arc::new(NoopRuntime::ready()),
            candle,
            Arc::new(DisabledLlmClient::new(
                "mt013-fallback".to_owned(),
                "no external fallback".to_owned(),
            )),
            Arc::new(NoopRecorder),
            8192,
            Some(ledger),
        )
        .expect("assemble exact-scope embedded runtime client"),
    )
}

#[derive(Debug, SurrealValue)]
struct ExactProcessBindings {
    record: RecordId,
    process_uuid: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

impl ExactProcessBindings {
    fn new(scope: &ReclaimResourceScope, process_uuid: Uuid) -> Self {
        Self {
            record: RecordId::new("kernel_process_lifecycle", process_uuid.to_string()),
            process_uuid: process_uuid.to_string(),
            owner_account_id: scope.account_uuid.to_string(),
            actor_principal_id: scope.actor_uuid.to_string(),
            authenticated_session_id: scope.session_uuid.to_string(),
            access_space_id: scope.access_space_uuid.to_string(),
            workspace_id: scope.workspace_id.clone(),
        }
    }
}

#[derive(Debug, SurrealValue)]
struct CanonicalProcessEventProbe {
    event_id: String,
    event_sequence: i64,
    event_type: String,
    aggregate_id: String,
}

const READ_EXACT_PROCESS_EVENTS: &str = r#"
SELECT event_id, event_sequence, event_type, aggregate_id
FROM kernel_event_ledger
WHERE aggregate_type = 'process_ownership'
    AND aggregate_id = $process_uuid
    AND payload.metadata_jsonb.owner_account_id = $owner_account_id
    AND payload.metadata_jsonb.actor_principal_id = $actor_principal_id
    AND payload.metadata_jsonb.authenticated_session_id = $authenticated_session_id
    AND payload.metadata_jsonb.access_space_id = $access_space_id
    AND payload.metadata_jsonb.workspace_id = $workspace_id
ORDER BY event_sequence ASC;
"#;

async fn exact_process_events(
    storage: &SurrealStorage,
    scope: &ReclaimResourceScope,
    process_uuid: Uuid,
) -> Vec<CanonicalProcessEventProbe> {
    let bindings = ExactProcessBindings::new(scope, process_uuid);
    storage
        .with_data_operation(|database| {
            Box::pin(async move {
                database
                    .query_values::<CanonicalProcessEventProbe, _>(
                        READ_EXACT_PROCESS_EVENTS,
                        bindings,
                    )
                    .await
            })
        })
        .await
        .expect("read exact-scope canonical lifecycle events")
}

#[tokio::test]
async fn embedded_load_shutdown_is_exact_scope_durable_and_canonically_linked() {
    let harness = ProcessLedgerSurrealHarness::open().await;
    let forwarding = scoped_store(&harness);
    let (ledger, drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("manual exact-scope lifecycle writer");
    let model_id = ModelId::new_v7();
    let client = local_client(model_id, ledger, Arc::new(NoopRuntime::ready()));

    drain
        .drain_available_to(Arc::clone(&forwarding))
        .await
        .expect("durably drain embedded START");
    let started = harness
        .lifecycle(model_id.as_uuid())
        .await
        .expect("exact-scope embedded START exists");
    assert_eq!(started.process_uuid, model_id.as_uuid());
    assert_eq!(started.os_pid, None);
    assert_eq!(started.engine_kind, "candle");
    assert_eq!(started.stopped_at, None);
    assert_eq!(
        started.metadata["os_pid_absent_reason"],
        "in_process_library_load_no_os_process"
    );

    client
        .shutdown_gracefully()
        .await
        .expect("quiesce and unload before STOP");
    drain
        .drain_available_to(forwarding)
        .await
        .expect("durably drain embedded STOP");
    let stopped = harness
        .lifecycle(model_id.as_uuid())
        .await
        .expect("exact-scope embedded lifecycle remains readable");
    assert!(stopped.stopped_at.is_some());
    assert_eq!(stopped.stop_reason.as_deref(), Some("llm-client-shutdown"));
    assert!(stopped.event_ledger_event_id.is_some());

    let events = exact_process_events(
        &harness.storage(),
        harness.resource_scope(),
        model_id.as_uuid(),
    )
    .await;
    assert_eq!(events.len(), 2);
    assert!(events[0].event_sequence < events[1].event_sequence);
    assert_eq!(events[0].aggregate_id, model_id.to_string());
    assert_eq!(events[1].aggregate_id, model_id.to_string());
    assert_ne!(events[0].event_id, events[1].event_id);
    assert_ne!(events[0].event_type, events[1].event_type);
    harness.close().await;
}

#[tokio::test]
async fn unload_failure_leaves_only_the_exact_scope_start_open() {
    let harness = ProcessLedgerSurrealHarness::open().await;
    let forwarding = scoped_store(&harness);
    let (ledger, drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("manual exact-scope lifecycle writer");
    let model_id = ModelId::new_v7();
    let client = local_client(
        model_id,
        ledger,
        Arc::new(NoopRuntime::failing_unload("injected unload failure")),
    );

    let error = client
        .shutdown_gracefully()
        .await
        .expect_err("unload failure must fail closed");
    assert!(error.to_string().contains("injected unload failure"));
    drain
        .drain_available_to(forwarding)
        .await
        .expect("drain only accepted lifecycle rows");
    let row = harness
        .lifecycle(model_id.as_uuid())
        .await
        .expect("failed unload leaves START open");
    assert_eq!(row.stopped_at, None);
    assert_eq!(harness.process_event_count().await, 1);
    harness.close().await;
}

#[tokio::test]
async fn repeated_shutdown_is_idempotent_and_background_drain_precedes_storage_close() {
    let harness = ProcessLedgerSurrealHarness::open().await;
    let forwarding = scoped_store(&harness);
    let (ledger, writer_join) = LedgerBatcher::spawn(
        forwarding,
        Arc::new(NoopOverflowSink),
        LedgerBatcherConfig {
            capacity: 4,
            batch_size: 2,
            ..LedgerBatcherConfig::default()
        },
    );
    let close = ledger.clone();
    let model_id = ModelId::new_v7();
    let client = local_client(model_id, ledger, Arc::new(NoopRuntime::ready()));
    client
        .shutdown_gracefully()
        .await
        .expect("first graceful shutdown");
    client
        .shutdown_gracefully()
        .await
        .expect("second graceful shutdown is idempotent");
    drop(client);

    let outcome = drain_and_join_ledger_writer(&close, writer_join, Duration::from_secs(5)).await;
    assert!(matches!(outcome, LedgerDrainJoinOutcome::Flushed));
    assert_eq!(harness.lifecycle_count().await, 1);
    assert_eq!(harness.open_lifecycle_count().await, 0);
    assert_eq!(harness.process_event_count().await, 2);
    harness.close().await;
}

fn scoped_process_start(
    scope: &ReclaimResourceScope,
    process_uuid: Uuid,
    session_id: Option<&str>,
    runtime_owner: Option<ProcessRuntimeOwner>,
) -> ProcessStart {
    let mut start = ProcessStart::new(
        ProcessEngineKind::Candle,
        "mt013-surreal-reclaim-proof",
        Some("WP-1".to_owned()),
    )
    .with_process_uuid(process_uuid)
    .with_sandbox_adapter_id("mt013-test-adapter")
    .with_sandbox_internal_id(process_uuid.to_string())
    .with_metadata_jsonb(scope_metadata(scope));
    if let Some(session_id) = session_id {
        start = start.with_parent_session_id(session_id);
    }
    if let Some(runtime_owner) = runtime_owner {
        start = start.with_runtime_owner(runtime_owner);
    }
    start
}

#[derive(Debug, SurrealValue)]
struct ClaimProbe {
    stopped_at: Option<chrono::DateTime<Utc>>,
    stop_reason: Option<String>,
    reclaim_claimant_uuid: Option<Uuid>,
    reclaim_generation: Option<i64>,
}

const READ_EXACT_CLAIM_STATE: &str = r#"
SELECT stopped_at, stop_reason, reclaim_claimant_uuid, reclaim_generation
FROM ONLY $record
WHERE owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id;
"#;

async fn claim_state(
    storage: &SurrealStorage,
    scope: &ReclaimResourceScope,
    process_uuid: Uuid,
) -> Option<ClaimProbe> {
    let bindings = ExactProcessBindings::new(scope, process_uuid);
    storage
        .with_data_operation(|database| {
            Box::pin(async move {
                database
                    .query_first::<ClaimProbe, _>(READ_EXACT_CLAIM_STATE, bindings)
                    .await
            })
        })
        .await
        .expect("read exact-scope reclaim state")
}

#[tokio::test]
async fn exact_start_stop_replay_is_idempotent_and_conflict_preserves_terminal_row() {
    let harness = ProcessLedgerSurrealHarness::open().await;
    let process_uuid = Uuid::now_v7();
    let start = scoped_process_start(harness.resource_scope(), process_uuid, None, None);
    let stop = ProcessStop::from_start(&start, Some(0)).with_stop_reason("graceful");
    let exact_batch = vec![
        LedgerEvent::Start(start.clone()),
        LedgerEvent::Stop(stop.clone()),
    ];
    harness
        .write_batch(exact_batch.clone())
        .await
        .expect("commit exact START/STOP batch");
    harness
        .write_batch(exact_batch)
        .await
        .expect("exact START/STOP replay is idempotent");

    let conflicting = ProcessStop::from_start(&start, Some(9)).with_stop_reason("rewrite");
    let error = harness
        .write_batch(vec![LedgerEvent::Stop(conflicting)])
        .await
        .expect_err("terminal rewrite must fail closed");
    assert!(matches!(
        error,
        ProcessLedgerError::StopIdentityConflict { .. } | ProcessLedgerError::Store(_)
    ));
    let row = harness.lifecycle(process_uuid).await.expect("terminal row");
    assert_eq!(row.exit_code, Some(0));
    assert_eq!(row.stop_reason.as_deref(), Some("graceful"));
    assert_eq!(harness.lifecycle_count().await, 1);
    assert_eq!(harness.process_event_count().await, 2);
    harness.close().await;
}

#[tokio::test]
async fn missing_and_one_field_mismatched_scope_deny_without_mutation_or_leakage() {
    let harness = ProcessLedgerSurrealHarness::open().await;
    let missing_uuid = Uuid::now_v7();
    let missing = ProcessStart::new(ProcessEngineKind::Candle, "missing-scope", None)
        .with_process_uuid(missing_uuid);
    harness
        .write_batch(vec![LedgerEvent::Start(missing)])
        .await
        .expect_err("missing five-field scope must be rejected");
    assert_eq!(harness.lifecycle_count().await, 0);
    assert_eq!(harness.process_event_count().await, 0);

    let session_id = format!("session-{}", Uuid::now_v7());
    let process_uuid = Uuid::now_v7();
    harness
        .write_batch(vec![LedgerEvent::Start(scoped_process_start(
            harness.resource_scope(),
            process_uuid,
            Some(&session_id),
            None,
        ))])
        .await
        .expect("seed exact-scope lifecycle");
    let mut wrong = harness.resource_scope().clone();
    wrong.access_space_uuid = Uuid::now_v7();
    assert!(harness
        .store()
        .active_processes_for_session(&wrong, &session_id)
        .await
        .expect("one-field mismatch is a safe empty claim")
        .is_empty());
    assert!(claim_state(&harness.storage(), &wrong, process_uuid)
        .await
        .is_none());
    let exact = claim_state(&harness.storage(), harness.resource_scope(), process_uuid)
        .await
        .expect("exact row remains visible");
    assert_eq!(exact.stopped_at, None);
    assert_eq!(exact.stop_reason, None);
    assert_eq!(exact.reclaim_claimant_uuid, None);
    assert_eq!(exact.reclaim_generation, None);
    assert_eq!(harness.lifecycle_count().await, 1);
    assert_eq!(harness.process_event_count().await, 1);
    harness.close().await;
}

#[tokio::test]
async fn concurrent_exact_scope_claimants_never_receive_the_same_process() {
    let harness = ProcessLedgerSurrealHarness::open().await;
    let session_id = format!("session-{}", Uuid::now_v7());
    let expected = BTreeSet::from([Uuid::now_v7(), Uuid::now_v7()]);
    for process_uuid in &expected {
        harness
            .write_batch(vec![LedgerEvent::Start(scoped_process_start(
                harness.resource_scope(),
                *process_uuid,
                Some(&session_id),
                None,
            ))])
            .await
            .expect("seed concurrency lifecycle");
    }

    let first_store = harness.store();
    let second_store = harness.store();
    let first_scope = harness.resource_scope().clone();
    let second_scope = harness.resource_scope().clone();
    let first_session = session_id.clone();
    let second_session = session_id.clone();
    let first = tokio::spawn(async move {
        first_store
            .active_processes_for_session(&first_scope, &first_session)
            .await
    });
    let second = tokio::spawn(async move {
        second_store
            .active_processes_for_session(&second_scope, &second_session)
            .await
    });
    let first = first
        .await
        .expect("join first claimant")
        .expect("first claim");
    let second = second
        .await
        .expect("join second claimant")
        .expect("second claim");
    let mut observed = BTreeSet::new();
    for process in first.iter().chain(second.iter()) {
        assert!(
            observed.insert(process.process_uuid),
            "a durable process was returned to both claimants"
        );
    }
    assert_eq!(observed, expected);
    for process in first.iter().chain(second.iter()) {
        harness
            .store()
            .release_reclaim_claim(process.process_uuid, &process.reclaim_claim)
            .await
            .expect("release exact concurrency claim");
    }
    assert_eq!(harness.open_lifecycle_count().await, 2);
    assert_eq!(harness.process_event_count().await, 2);
    harness.close().await;
}

#[derive(Debug, SurrealValue)]
struct LaneBindings {
    record: RecordId,
    lane_id: String,
    run_id: String,
    idempotency_key: String,
    record_json: String,
    event_id: String,
    event_seq: i64,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

const CREATE_LANE_AUTHORITY: &str = r#"
CREATE $record CONTENT {
    record_kind: 'lane', aggregate_id: $lane_id, run_id: $run_id,
    idempotency_key: $idempotency_key, record_json: $record_json,
    search_terms: [], event_id: $event_id, event_seq: $event_seq,
    event_stream_version: 1, transaction_seq: $event_seq,
    owner_account_id: $owner_account_id,
    actor_principal_id: $actor_principal_id,
    authenticated_session_id: $authenticated_session_id,
    access_space_id: $access_space_id, workspace_id: $workspace_id
};
"#;

async fn seed_lane(
    storage: &SurrealStorage,
    scope: &ReclaimResourceScope,
    session_id: &str,
    process_uuid: Uuid,
    status: &str,
) {
    let lane_id = format!("lane-{}", Uuid::now_v7());
    let bindings = LaneBindings {
        record: RecordId::new("model_lane_authority", lane_id.clone()),
        lane_id: lane_id.clone(),
        run_id: format!("run-{}", Uuid::now_v7()),
        idempotency_key: format!("idem-{}", Uuid::now_v7()),
        record_json: json!({
            "lane_id": lane_id,
            "coordinator_session_id": session_id,
            "process_ownership_ref": format!("process-ledger://{process_uuid}"),
            "status": status,
            "heartbeat_at_utc": Utc::now().to_rfc3339(),
            "reclaim_after_utc": Value::Null,
        })
        .to_string(),
        event_id: format!("event-{}", Uuid::now_v7()),
        event_seq: Utc::now().timestamp_micros(),
        owner_account_id: scope.account_uuid.to_string(),
        actor_principal_id: scope.actor_uuid.to_string(),
        authenticated_session_id: scope.session_uuid.to_string(),
        access_space_id: scope.access_space_uuid.to_string(),
        workspace_id: scope.workspace_id.clone(),
    };
    let changed = storage
        .with_data_operation(|database| {
            Box::pin(async move {
                database
                    .execute_returning(CREATE_LANE_AUTHORITY, bindings)
                    .await
            })
        })
        .await
        .expect("seed exact-scope model-lane authority");
    assert_eq!(changed, 1);
}

#[derive(Debug, SurrealValue)]
struct TamperScopeBindings {
    record: RecordId,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

const REMOVE_ACCESS_SPACE: &str = r#"
UPDATE $record SET access_space_id = NONE
WHERE owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
RETURN AFTER;
"#;

async fn remove_one_scope_field(
    storage: &SurrealStorage,
    scope: &ReclaimResourceScope,
    process_uuid: Uuid,
) {
    let exact = ExactProcessBindings::new(scope, process_uuid);
    let bindings = TamperScopeBindings {
        record: exact.record,
        owner_account_id: exact.owner_account_id,
        actor_principal_id: exact.actor_principal_id,
        authenticated_session_id: exact.authenticated_session_id,
        access_space_id: exact.access_space_id,
        workspace_id: exact.workspace_id,
    };
    let changed = storage
        .with_data_operation(|database| {
            Box::pin(async move {
                database
                    .execute_returning(REMOVE_ACCESS_SPACE, bindings)
                    .await
            })
        })
        .await
        .expect("remove one scope field for fail-closed counterfactual");
    assert_eq!(changed, 1);
}

#[tokio::test]
async fn stale_source_denies_mixed_health_and_incomplete_scope_without_mutation() {
    let harness = ProcessLedgerSurrealHarness::open().await;
    let owner_lease = acquire_embedded_runtime_instance_lease(Uuid::now_v7(), "mt013-host")
        .expect("acquire stale-source owner lease");
    let owner = owner_lease.descriptor().process_runtime_owner();
    let session_id = format!("session-{}", Uuid::now_v7());
    let failed = Uuid::now_v7();
    let healthy = Uuid::now_v7();
    for process_uuid in [failed, healthy] {
        harness
            .write_batch(vec![LedgerEvent::Start(scoped_process_start(
                harness.resource_scope(),
                process_uuid,
                Some(&session_id),
                Some(owner.clone()),
            ))])
            .await
            .expect("seed exact-scope stale-source lifecycle");
    }
    seed_lane(
        &harness.storage(),
        harness.resource_scope(),
        &session_id,
        failed,
        "failed",
    )
    .await;
    seed_lane(
        &harness.storage(),
        harness.resource_scope(),
        &session_id,
        healthy,
        "running",
    )
    .await;
    let source = SurrealModelLaneStaleSessionSource::new(
        harness.storage(),
        owner_lease.descriptor().clone(),
    );
    assert!(source
        .stale_session_process_sets(Duration::from_secs(300))
        .await
        .expect("mixed-health scan")
        .is_empty());
    for process_uuid in [failed, healthy] {
        let state = claim_state(&harness.storage(), harness.resource_scope(), process_uuid)
            .await
            .expect("mixed-health lifecycle remains exact and visible");
        assert_eq!(state.stopped_at, None);
        assert_eq!(state.reclaim_claimant_uuid, None);
        assert_eq!(state.reclaim_generation, None);
    }

    remove_one_scope_field(&harness.storage(), harness.resource_scope(), failed).await;
    assert!(source
        .restart_session_process_sets()
        .await
        .expect("incomplete attribution globally vetoes restart reclaim")
        .is_empty());
    let healthy_state = claim_state(&harness.storage(), harness.resource_scope(), healthy)
        .await
        .expect("complete sibling remains visible");
    assert_eq!(healthy_state.stopped_at, None);
    assert_eq!(healthy_state.reclaim_claimant_uuid, None);
    assert_eq!(healthy_state.reclaim_generation, None);
    assert_eq!(harness.process_event_count().await, 2);
    drop(source);
    drop(owner_lease);
    harness.close().await;
}

#[tokio::test]
async fn stale_claim_cannot_release_or_advance_the_current_exact_scope_claim() {
    let harness = ProcessLedgerSurrealHarness::open().await;
    let session_id = format!("session-{}", Uuid::now_v7());
    let process_uuid = Uuid::now_v7();
    harness
        .write_batch(vec![LedgerEvent::Start(scoped_process_start(
            harness.resource_scope(),
            process_uuid,
            Some(&session_id),
            None,
        ))])
        .await
        .expect("seed exact-scope claim row");
    let current = harness
        .store()
        .active_process_for_session(harness.resource_scope(), &session_id, process_uuid)
        .await
        .expect("claim exact lifecycle")
        .expect("lifecycle is claimable");
    let mut stale = current.reclaim_claim.clone();
    stale.claimant_uuid = Uuid::now_v7();
    stale.generation = stale.generation.saturating_sub(1);
    harness
        .store()
        .release_reclaim_claim(process_uuid, &stale)
        .await
        .expect_err("stale claimant cannot release current claim");
    harness
        .store()
        .mark_reclaim_kill_started(process_uuid, &stale)
        .await
        .expect_err("stale claimant cannot advance current claim");
    let state = claim_state(&harness.storage(), harness.resource_scope(), process_uuid)
        .await
        .expect("current claim remains exact and visible");
    assert_eq!(state.stopped_at, None);
    assert_eq!(
        state.reclaim_claimant_uuid,
        Some(current.reclaim_claim.claimant_uuid)
    );
    assert_eq!(
        state.reclaim_generation,
        i64::try_from(current.reclaim_claim.generation).ok()
    );
    harness
        .store()
        .release_reclaim_claim(process_uuid, &current.reclaim_claim)
        .await
        .expect("current claimant releases its own claim");
    assert_eq!(harness.process_event_count().await, 1);
    harness.close().await;
}

#[derive(Debug, SurrealValue)]
struct ExactSeedBindings {
    record: RecordId,
    content: Value,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

impl ExactSeedBindings {
    fn new(scope: &ReclaimResourceScope, record: RecordId, mut content: Value) -> Self {
        let object = content.as_object_mut().expect("seed content is an object");
        object.extend(
            scope_metadata(scope)
                .as_object()
                .expect("scope metadata is an object")
                .clone(),
        );
        Self {
            record,
            content,
            owner_account_id: scope.account_uuid.to_string(),
            actor_principal_id: scope.actor_uuid.to_string(),
            authenticated_session_id: scope.session_uuid.to_string(),
            access_space_id: scope.access_space_uuid.to_string(),
            workspace_id: scope.workspace_id.clone(),
        }
    }
}

const CREATE_AND_VERIFY_EXACT_RECORD: &str = r#"
BEGIN TRANSACTION;
CREATE $record CONTENT $content;
LET $verified = array::len(SELECT VALUE id FROM ONLY $record
    WHERE owner_account_id = $owner_account_id
        AND actor_principal_id = $actor_principal_id
        AND authenticated_session_id = $authenticated_session_id
        AND access_space_id = $access_space_id
        AND workspace_id = $workspace_id);
IF $verified != 1 {
    THROW 'MT013_RESTART_SEED_SCOPE_MISMATCH';
};
RETURN $verified;
COMMIT TRANSACTION;
"#;

async fn create_exact_record(
    storage: &SurrealStorage,
    scope: &ReclaimResourceScope,
    record: RecordId,
    content: Value,
) {
    let bindings = ExactSeedBindings::new(scope, record, content);
    let verified = storage
        .with_data_operation(|database| {
            Box::pin(async move {
                database
                    .query_values_at::<i64, _>(CREATE_AND_VERIFY_EXACT_RECORD, bindings, 4)
                    .await
            })
        })
        .await
        .expect("create and verify exact-scope restart fixture");
    assert_eq!(verified.as_slice(), [1]);
}

#[derive(Debug, SurrealValue)]
struct QueueStateProbe {
    state: String,
    claimed_by: Option<String>,
}

const READ_EXACT_QUEUE_STATE: &str = r#"
SELECT state, claimed_by FROM ONLY $record
WHERE owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id;
"#;

#[tokio::test]
async fn restart_resume_replays_canonical_events_on_the_same_exact_scope_storage() {
    let harness = ProcessLedgerSurrealHarness::open().await;
    let storage = harness.storage();
    let scope = harness.resource_scope().clone();
    let session_id = Uuid::now_v7();
    let queue_record = RecordId::new("kernel_session_queue", session_id.to_string());
    create_exact_record(
        &storage,
        &scope,
        queue_record.clone(),
        json!({
            "session_run_id": session_id.to_string(),
            "kernel_task_run_id": format!("KTR-{session_id}"),
            "adapter_id": "mt013-surreal-restart",
            "state": "RUNNING",
            "claimed_by": "prior-worker",
            "lease_expires_at": Utc::now(),
            "attempt_count": 1,
            "available_at": Utc::now(),
            "created_at": Utc::now(),
            "updated_at": Utc::now(),
        }),
    )
    .await;
    let checkpoint_id = Uuid::now_v7();
    create_exact_record(
        &storage,
        &scope,
        RecordId::new("kernel_session_checkpoint", checkpoint_id.to_string()),
        json!({
            "checkpoint_id": checkpoint_id,
            "session_id": session_id,
            "model_session_id": Uuid::now_v7(),
            "last_event_ledger_seq": 0,
            "compact_state": { "counter": 4 },
            "state_kind": "periodic",
            "pending_artifacts": [],
            "created_at_utc": Utc::now(),
            "created_by_process": 1234,
            "schema_version": 1,
        }),
    )
    .await;
    let event_id = format!("KE-{}", Uuid::now_v7());
    create_exact_record(
        &storage,
        &scope,
        RecordId::new("kernel_event_ledger", event_id.clone()),
        json!({
            "event_id": event_id,
            "event_sequence": 1,
            "event_version": "kernel_event_v1",
            "kernel_task_run_id": format!("KTR-{session_id}"),
            "session_run_id": session_id.to_string(),
            "aggregate_type": "session_run",
            "aggregate_id": session_id.to_string(),
            "idempotency_key": format!("restart-{session_id}-1"),
            "event_type": "MODEL_RESPONSE_RECORDED",
            "actor_kind": "session_broker",
            "actor_id": "mt013-surreal-restart",
            "payload_hash": "0000000000000000000000000000000000000000000000000000000000000000",
            "source_component": "embedded_model_surreal_ledger_tests",
            "payload": { "by": 3 },
            "created_at": Utc::now(),
        }),
    )
    .await;

    let report = SurrealRestartResumeRunner::new(storage.clone(), scope.clone())
        .run()
        .await
        .expect("run same-storage restart-resume");
    assert_eq!(report.sessions_examined, 1);
    assert_eq!(report.sessions_resumed.len(), 1);
    assert!(report.sessions_recovery_failed.is_empty());
    assert_eq!(report.sessions_resumed[0].session_id, session_id);
    assert_eq!(report.sessions_resumed[0].events_applied, 1);
    assert_eq!(report.sessions_resumed[0].final_seq, 1);
    assert!(report
        .fr_events_emitted
        .iter()
        .any(|event| event == "FR-EVT-RESTART-RESUME-COMPLETED"));

    let queue = storage
        .with_data_operation(|database| {
            let bindings = ExactProcessBindings::new(&scope, session_id);
            let bindings = ExactSeedBindings {
                record: queue_record,
                content: Value::Null,
                owner_account_id: bindings.owner_account_id,
                actor_principal_id: bindings.actor_principal_id,
                authenticated_session_id: bindings.authenticated_session_id,
                access_space_id: bindings.access_space_id,
                workspace_id: bindings.workspace_id,
            };
            Box::pin(async move {
                database
                    .query_first::<QueueStateProbe, _>(READ_EXACT_QUEUE_STATE, bindings)
                    .await
            })
        })
        .await
        .expect("read exact-scope resumed queue")
        .expect("resumed queue exists");
    assert_eq!(queue.state, "RETRY_SCHEDULED");
    assert_eq!(queue.claimed_by, None);
    assert_eq!(harness.lifecycle_count().await, 0);
    drop(storage);
    harness.close().await;
}

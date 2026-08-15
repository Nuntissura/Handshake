//! WP-1 MT-008: Dexterity lane diagnostics projection proof.

mod knowledge_pg_support;
mod model_lane_crdt_support;

use axum::{
    body::Body,
    http::{Request, StatusCode},
    Extension,
};
use futures::stream;
use handshake_core::kernel::context_bundle::ContextBundle;
use handshake_core::model_runtime::registry::RuntimeBinding as RuntimeAdapterBinding;
use handshake_core::model_runtime::{
    CancellationToken, Embedding, GenPrompt, GenerateRequest, GeneratedToken, KvCacheHandle,
    LoadSpec, LoraStackHandle, ModelCapabilities, ModelId, ModelRuntime, ModelRuntimeError,
    SamplingParams, Score, SteeringHookHandle, TokenStream,
};
use handshake_core::process_ledger::{
    LedgerBatcher, LedgerBatcherConfig, NoopOverflowSink, ProcessEngineKind,
    ProcessOwnershipRecordId, ProcessStart,
};
use handshake_core::swarm_orchestration::model_lane::{
    DexterityLaunchContract, ModelLanePromotionOutcome, ModelLaneRoutingPolicy,
};
use handshake_core::swarm_orchestration::model_lane::{
    LaunchAuthority, ModelLaneAuthority, ModelLaneDiagnosticTier, ModelLaneDiagnosticTierState,
    ModelLaneKind, ModelLaneLeaseScope, ModelLaneLeaseState, ModelLaneLocusBinding,
    ModelLaneMessageKind, ModelLaneMtRuntimeStatus, ModelLaneProviderKind, ModelLaneRecoveryState,
    ModelLaneRoutingMetadata, ModelLaneStatus, ModelLaneStore, ModelLaneTarget, NewModelLane,
    NewModelLaneContextBundleArtifactBinding, NewModelLaneDiagnosticTierStatus, NewModelLaneLease,
    NewModelLaneMessage, NewModelLaneMtRuntimeStatus, NewModelLanePromotionDecision,
    NewModelLaneRun, RuntimeBinding,
};
use handshake_core::swarm_orchestration::routing::{
    ModelLaneRoutingAuthority, ModelLaneRoutingDispatchTarget, ModelLaneRoutingGraph,
    ModelLaneRoutingStageLaunchPlan,
};
use handshake_core::swarm_orchestration::routing_execution::{
    ModelLaneRoutingExecutionContext, ModelLaneRoutingStageLaunch,
};
use handshake_core::swarm_orchestration::{
    LiveSession, ModelInstanceId, ModelSessionFactory, RecordingSwarmSink, RunBudget, SpawnRequest,
    SwarmConfig, SwarmCoordinator, SwarmError,
};
use model_lane_crdt_support::{
    apply_crdt_posture, build_admissible_crdt_posture, seed_real_crdt_document,
    CrdtProposalAnchoring,
};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tower::ServiceExt;

const WP_ID: &str = "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1";
const OWNER: &str = "KERNEL_BUILDER-20260630-045713";

struct DiagnosticsRoutingRuntime {
    capabilities: ModelCapabilities,
    kv: KvCacheHandle,
    lora: LoraStackHandle,
    steering: SteeringHookHandle,
}

impl DiagnosticsRoutingRuntime {
    fn new() -> Self {
        Self {
            capabilities: ModelCapabilities::default(),
            kv: KvCacheHandle::new("mt017-diagnostics-kv"),
            lora: LoraStackHandle::new("mt017-diagnostics-lora"),
            steering: SteeringHookHandle::new("mt017-diagnostics-steering"),
        }
    }
}

#[async_trait::async_trait]
impl ModelRuntime for DiagnosticsRoutingRuntime {
    async fn load(&mut self, _spec: LoadSpec) -> Result<ModelId, ModelRuntimeError> {
        Ok(ModelId::new_v7())
    }

    async fn unload(&mut self, _id: ModelId) -> Result<(), ModelRuntimeError> {
        Ok(())
    }

    fn generate(&self, _request: GenerateRequest) -> TokenStream {
        Box::pin(stream::iter(vec![Ok(GeneratedToken {
            token_id: 1,
            text: "MT-017 diagnostics routing candidate".into(),
            logprob: None,
            finish_reason: None,
        })]))
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

struct DiagnosticsRoutingFactory {
    ledger: LedgerBatcher,
}

#[async_trait::async_trait]
impl ModelSessionFactory for DiagnosticsRoutingFactory {
    async fn create(&self, request: &SpawnRequest) -> Result<LiveSession, SwarmError> {
        let record_id = ProcessOwnershipRecordId::new_v7();
        let start = ProcessStart::new(
            ProcessEngineKind::Candle,
            request.owner_role.clone(),
            request.owner_wp.clone(),
        )
        .with_process_uuid(record_id.as_uuid())
        .with_os_pid(62_017)
        .with_parent_session_id(request.parent_session_id.clone())
        .with_wp_id(request.wp_id.clone().unwrap_or_default())
        .with_mt_id(request.mt_id.clone().unwrap_or_default());
        self.ledger
            .record_start(start.clone())
            .map_err(|error| SwarmError::LedgerFailed(error.to_string()))?;
        let teardown: handshake_core::swarm_orchestration::SessionTeardown =
            Arc::new(|| Box::pin(async { Ok(()) }));
        Ok(LiveSession::new(
            Arc::new(DiagnosticsRoutingRuntime::new()),
            request.instance_id.model_id,
            CancellationToken::new(),
            teardown,
            record_id,
            62_017,
        )
        .with_ledger_start(ProcessEngineKind::Candle, start))
    }
}

#[derive(Default)]
struct NoopRecorder;

#[async_trait::async_trait]
impl handshake_core::flight_recorder::FlightRecorder for NoopRecorder {
    async fn record_event(
        &self,
        _event: handshake_core::flight_recorder::FlightRecorderEvent,
    ) -> Result<(), handshake_core::flight_recorder::RecorderError> {
        Ok(())
    }

    async fn enforce_retention(
        &self,
    ) -> Result<u64, handshake_core::flight_recorder::RecorderError> {
        Ok(0)
    }

    async fn list_events(
        &self,
        _filter: handshake_core::flight_recorder::EventFilter,
    ) -> Result<
        Vec<handshake_core::flight_recorder::FlightRecorderEvent>,
        handshake_core::flight_recorder::RecorderError,
    > {
        Ok(Vec::new())
    }
}

#[async_trait::async_trait]
impl handshake_core::diagnostics::DiagnosticsStore for NoopRecorder {
    async fn record_diagnostic(
        &self,
        _diagnostic: handshake_core::diagnostics::Diagnostic,
    ) -> Result<(), handshake_core::storage::StorageError> {
        Ok(())
    }

    async fn list_problems(
        &self,
        _filter: handshake_core::diagnostics::DiagFilter,
    ) -> Result<Vec<handshake_core::diagnostics::ProblemGroup>, handshake_core::storage::StorageError>
    {
        Ok(Vec::new())
    }

    async fn get_diagnostic(
        &self,
        _id: uuid::Uuid,
    ) -> Result<handshake_core::diagnostics::Diagnostic, handshake_core::storage::StorageError>
    {
        Err(handshake_core::storage::StorageError::NotFound(
            "diagnostic",
        ))
    }

    async fn list_diagnostics(
        &self,
        _filter: handshake_core::diagnostics::DiagFilter,
    ) -> Result<Vec<handshake_core::diagnostics::Diagnostic>, handshake_core::storage::StorageError>
    {
        Ok(Vec::new())
    }
}

struct CatalogLlmClient {
    inner: handshake_core::llm::InMemoryLlmClient,
    catalog: Arc<handshake_core::model_runtime::ModelCatalog>,
}

#[async_trait::async_trait]
impl handshake_core::llm::LlmClient for CatalogLlmClient {
    async fn completion(
        &self,
        request: handshake_core::llm::CompletionRequest,
    ) -> Result<handshake_core::llm::CompletionResponse, handshake_core::llm::LlmError> {
        handshake_core::llm::LlmClient::completion(&self.inner, request).await
    }

    fn profile(&self) -> &handshake_core::llm::ModelProfile {
        handshake_core::llm::LlmClient::profile(&self.inner)
    }

    fn model_catalog(&self) -> Option<Arc<handshake_core::model_runtime::ModelCatalog>> {
        Some(self.catalog.clone())
    }
}

#[tokio::test]
async fn swarm_lane_diagnostics_backend_projection_matches_eventledger() {
    // Explicit kpg setup (not `diagnostics_store`) so the diagnostics source
    // message can reference a REAL persisted CRDT document + snapshot + lease that
    // resolve at the shared admission boundary.
    let kpg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("PostgreSQL/EventLedger is required for MT-008 proof");
    let schema_url = kpg.schema_url.clone();
    let workspace_id = kpg.create_workspace().await;
    let db = kpg.db;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&schema_url)
        .await
        .expect("connect isolated diagnostics schema");
    // HBR-PRIV: the mounted diagnostics route reads through an account-scoped
    // store, so rows seeded with a NULL owner are invisible to it and the HTTP
    // leg below would assert against an empty projection. Seed as a real owning
    // account and send that account's scope on the request.
    use handshake_core::swarm_orchestration::resource_scope::{
        AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, ExactResourceScopeAttribution,
        OwnerAccountId, ResourceScope, WorkspaceScopeRef,
    };
    let diagnostics_owner = OwnerAccountId::mint();
    let diagnostics_scope = ResourceScope::new(diagnostics_owner, ActorPrincipalId::mint())
        .with_session(AuthenticatedSessionRef::mint())
        .with_access_space(AccessSpaceRef::mint())
        .with_workspace(
            WorkspaceScopeRef::new(workspace_id.to_string())
                .expect("diagnostics workspace is nonblank"),
        );
    let diagnostics_exact =
        ExactResourceScopeAttribution::try_from_resource_scope(&diagnostics_scope)
            .expect("diagnostics fixture carries exact five-dimensional scope");
    let store = ModelLaneStore::new_scoped(pool.clone(), diagnostics_scope);
    let model_id = handshake_core::model_runtime::ModelId::new_v7();
    let model_id_text = model_id.to_string();
    let model_registration = handshake_core::model_runtime::ModelRegistration {
        model_id: model_id.clone(),
        artifact_path: std::path::PathBuf::from("models/mt014-diagnostics-model.gguf"),
        sha256: [0x5a; 32],
        runtime_binding: handshake_core::model_runtime::RuntimeBinding::LlamaCpp,
        declared_capabilities: handshake_core::model_runtime::ModelCapabilities::default(),
        base_model_tag: handshake_core::model_runtime::BaseModelTag::new("mt014-diagnostics-base"),
        registered_at_utc: chrono::Utc::now(),
        registered_by: handshake_core::model_runtime::OperatorId::new("mt014-diagnostics-proof"),
        provider: handshake_core::model_runtime::ProviderKind::Local,
    };
    handshake_core::model_runtime::ModelRegistryStore::new(pool.clone())
        .persist_and_read_back(&model_registration)
        .await
        .expect("persist diagnostics model stable anchor authority");
    let stale_model_id = handshake_core::model_runtime::ModelId::new_v7().to_string();

    let mut run = sample_run("run-mt008-diag", "lane-mt008-local");
    run.lane_ids.push("lane-mt014-stale".into());
    run.lane_ids.push("lane-mt017-validator".into());
    store.record_run(run).await.expect("record diagnostics run");
    let mut known_lane = sample_lane("lane-mt008-local", "run-mt008-diag");
    known_lane.model_id = Some(model_id_text.clone());
    store
        .record_lane(known_lane)
        .await
        .expect("record diagnostics lane");
    let mut stale_lane = sample_lane("lane-mt014-stale", "run-mt008-diag");
    stale_lane.model_id = Some(stale_model_id.clone());
    store
        .record_lane(stale_lane)
        .await
        .expect("record stale-model diagnostics lane");
    let mut validator_lane = sample_lane("lane-mt017-validator", "run-mt008-diag");
    validator_lane.kind = ModelLaneKind::Validator;
    validator_lane.role = "validator".into();
    validator_lane.backend = RuntimeBinding::Validator.as_str().into();
    validator_lane.model_id = None;
    validator_lane.model_session_id = "model-session-lane-mt017-validator".into();
    validator_lane.runtime_binding = RuntimeBinding::Validator;
    validator_lane.launch_authority = LaunchAuthority::ValidatorRunner;
    validator_lane.provider_kind = ModelLaneProviderKind::Validator;
    validator_lane.process_ownership_ref = None;
    validator_lane.no_os_process_reason_ref =
        Some("no-os://mt017-diagnostics/validator-runner".into());
    store
        .record_lane(validator_lane)
        .await
        .expect("record routing validator diagnostics lane");

    // Seed a REAL CRDT document on `lane-mt008-local`'s session through the ONE
    // canonical MT-018 posture builder, so the diagnostics source message carries
    // CRDT authority references that resolve (rather than synthetic refs that fail
    // closed) and its downstream promotion decision has real base-snapshot /
    // state-vector inputs.
    //
    // `CRDT_SEED_LABEL` is the lane id on purpose: `seed_real_crdt_document`
    // derives the seeded update session as `session-{label}` and `sample_lane`
    // derives the lane session as `session-{lane_id}`, so they must be the same
    // string for `validate_crdt_lane_session_uniqueness_tx` to hold.
    //
    // WP-1 MT-018 (commit e5888b3d) removed the un-admittable Proposal-kind rule
    // (`applied_update_sha256` is the approved-DIFF hash, cross-checked against
    // the proposal's own `diff_sha256`, while Yjs update identity is carried by
    // `applied_update_id`). This producer therefore emits the COMPLETE CRDT
    // posture -- including a `crdt_proposal_ref` minted through the real
    // `record_ai_edit_proposal` -> `decide_ai_edit_proposal` ->
    // `apply_approved_ai_edit` chain and anchored to the SAME update the message
    // cites -- instead of the pre-MT-018 Status-kind / `crdt_proposal_ref = None`
    // carve-out. That carve-out produced a partial CRDT posture the native
    // schema-v3 consumer contract rejects (`CRDT refs missing`).
    const CRDT_SEED_LABEL: &str = "lane-mt008-local";
    let crdt_document_id = format!("crdt-mt008-diag-{workspace_id}");
    let crdt_receipts = seed_real_crdt_document(
        &db,
        &workspace_id,
        &format!("doc-mt008-diag-{workspace_id}"),
        &crdt_document_id,
        CRDT_SEED_LABEL,
    )
    .await;
    let crdt_posture = build_admissible_crdt_posture(
        &db,
        &pool,
        &crdt_receipts,
        "lane-mt008-local",
        CRDT_SEED_LABEL,
        CrdtProposalAnchoring::ApprovedApplied,
    )
    .await;
    let crdt_proposal_ref = crdt_posture
        .crdt_proposal_ref
        .clone()
        .expect("ApprovedApplied anchoring mints a real crdt_proposal_ref");

    let mut source_message = sample_message("msg-mt008-001", "run-mt008-diag", "lane-mt008-local");
    apply_crdt_posture(
        &mut source_message,
        &crdt_posture,
        ModelLaneMessageKind::Proposal,
    );
    source_message.payload_sha256 = ContextBundle::new(
        "mt017-diagnostics-routing-input",
        &source_message.run_id,
        routing_input_payload(&source_message),
    )
    .expect("canonicalize routing input through the public ContextBundle contract")
    .context_hash;
    let source_record = store
        .record_message_with_payload_binding(
            source_message.clone(),
            routing_input_binding(&source_message),
        )
        .await
        .expect("record diagnostics message");
    store
        .record_lane_lease(sample_lease(
            "lease-mt008-expired",
            "run-mt008-diag",
            "lane-mt008-local",
            "2020-01-01T00:00:00Z",
        ))
        .await
        .expect("record expired active lease");
    for (tier, state, evidence) in [
        (
            ModelLaneDiagnosticTier::FlightRecorder,
            ModelLaneDiagnosticTierState::Wired,
            "eventledger://kernel/model-lane/diagnostics",
        ),
        (
            ModelLaneDiagnosticTier::InternalDiagnostics,
            ModelLaneDiagnosticTierState::DeferredWithReason,
            "internal-diagnostics://separate-worktree/panic-heartbeat-frame-resource-open-event",
        ),
        (
            ModelLaneDiagnosticTier::Palmistry,
            ModelLaneDiagnosticTierState::DeferredWithReason,
            "palmistry://external-worktree/in-progress",
        ),
    ] {
        store
            .record_diagnostic_tier_status(sample_tier("run-mt008-diag", tier, state, evidence))
            .await
            .expect("record diagnostic tier");
    }
    store
        .record_mt_runtime_status(sample_mt_status("run-mt008-diag"))
        .await
        .expect("record MT runtime status");

    let routing_graph = ModelLaneRoutingGraph::for_policy(ModelLaneRoutingPolicy::ValidatorLane);
    let routing_instance = ModelInstanceId::new(model_id, 17);
    let mut routing_request = SpawnRequest::new(
        routing_instance,
        RuntimeAdapterBinding::Candle,
        OWNER,
        "coordinator-run-mt008-diag",
    )
    .with_wp(WP_ID)
    .with_mt("MT-008")
    .with_local_artifact("models/mt014-diagnostics-model.gguf", "5a".repeat(32));
    routing_request.owner_wp = Some(WP_ID.into());
    let mut launch_contract = DexterityLaunchContract::from_spawn_request(&routing_request)
        .expect("construct MT-017 routing launch contract");
    launch_contract.run_id = "run-mt008-diag".into();
    launch_contract.lane_id = "lane-mt017-routing-local".into();
    launch_contract.trace_id = "trace-run-mt008-diag".into();
    launch_contract.run_span_id = "span-run-mt008-diag".into();
    launch_contract.lane_span_id = "span-lane-mt017-routing-local".into();
    launch_contract.routing_policy = "mixed_local_cloud_subagent".into();
    launch_contract.context_bundle_id = "ctx-run-mt008-diag".into();
    launch_contract.event_ledger_stream_id = "mlane-stream-run-mt008-diag".into();
    launch_contract.artifact_namespace = "artifact://model-lane/run-mt008-diag".into();
    launch_contract.task_board_id = "task-board://wp-1".into();
    launch_contract.locus_binding_ref =
        "locus://wp1/mt008/run-mt008-diag/coordinator-run-mt008-diag".into();
    launch_contract.memory_pack_ref = "memory-pack://fems/run-mt008-diag".into();
    launch_contract.memory_pack_hash = sample_sha256();
    launch_contract.determinism_mode = "deterministic_replay".into();
    launch_contract.budget_summary_ref = "budget://mt008".into();
    launch_contract.candidate_model_ids = vec![routing_instance.model_id.to_string()];
    launch_contract.procedural_review_status = "reviewed_by_kernel_builder".into();
    routing_request = routing_request.with_dexterity_launch(launch_contract);
    let routing_launch_plan = vec![
        ModelLaneRoutingStageLaunchPlan {
            stage_id: "validation-candidate".into(),
            dispatch_target: ModelLaneRoutingDispatchTarget::LocalModel,
            lane_id: Some("lane-mt017-routing-local".into()),
            model_id: Some(routing_instance.model_id.to_string()),
            provider: None,
        },
        ModelLaneRoutingStageLaunchPlan {
            stage_id: "validator-verdict".into(),
            dispatch_target: ModelLaneRoutingDispatchTarget::Validator,
            lane_id: Some("lane-mt017-validator".into()),
            model_id: None,
            provider: None,
        },
    ];
    let routing_decision = store
        .record_promotion_decision(NewModelLanePromotionDecision {
            decision_id: "decision-mt017-diagnostics-awaiting".into(),
            run_id: "run-mt008-diag".into(),
            trace_id: "trace-run-mt008-diag".into(),
            decision_span_id: "span-decision-mt017-diagnostics-awaiting".into(),
            parent_span_id: Some(source_record.message_span_id.clone()),
            linked_span_contexts: vec![source_record.message_span_id.clone()],
            coordinator_session_id: "coordinator-run-mt008-diag".into(),
            routing_policy: ModelLaneRoutingPolicy::ValidatorLane,
            routing_launch_plan,
            input_refs: vec![format!("model-lane-message://{}", source_record.message_id)],
            selected_input_refs: vec![format!("model-lane-message://{}", source_record.message_id)],
            rejected_input_refs: Vec::new(),
            validator_authority_ref: Some("validator://mt017/diagnostics".into()),
            operator_authority_ref: None,
            expected_event_ledger_aggregate_type: "model_lane_message".into(),
            expected_event_ledger_aggregate_id: source_record.message_id.clone(),
            expected_event_ledger_version: source_record.event_ledger_seq,
            base_snapshot_ref: source_record
                .crdt_base_snapshot_ref
                .clone()
                .expect("source message CRDT base"),
            current_base_snapshot_ref: source_record
                .crdt_base_snapshot_ref
                .clone()
                .expect("source message current CRDT base"),
            state_vector: source_record
                .crdt_state_vector
                .clone()
                .expect("source message CRDT state vector"),
            current_state_vector: source_record
                .crdt_state_vector
                .clone()
                .expect("source message current CRDT state vector"),
            schema_id: "hsk.model_lane_message@1".into(),
            deterministic_tie_break_rule: "event_ledger_seq_then_message_id".into(),
            promotion_gate_ref: "promotion-gate://mt017/diagnostics".into(),
            promotion_receipt_ref: Some("promotion-receipt://mt017/diagnostics".into()),
            promoted_artifact_ref: Some("artifact://promoted/mt017/diagnostics".into()),
            promoted_artifact_sha256: Some(sample_sha256()),
            promoted_artifact_version: Some("1".into()),
            direct_authority_mutation_attempt_ref: None,
            event_ledger_stream_id: "mlane-stream-run-mt008-diag".into(),
            work_packet_id: Some(WP_ID.into()),
            micro_task_id: Some("MT-008".into()),
            task_board_id: Some("task-board://wp-1".into()),
            owner_session: OWNER.into(),
            idempotency_key: "idem-decision-mt017-diagnostics-awaiting".into(),
            replay_order_key: "00000090/promotion/mt017-diagnostics-awaiting".into(),
            recovery_hint_ref: Some("usermanual://model-lane-validation-harness#recovery".into()),
            created_at_utc: "2026-07-18T00:00:00Z".into(),
            diagnostic_payload: json!({
                "fixture": "mt017-routing-lifecycle-diagnostics",
                "routing_graph": routing_graph,
            }),
        })
        .await
        .expect("record approved MT-017 routing selection decision");
    assert_eq!(
        routing_decision.outcome,
        ModelLanePromotionOutcome::Approved
    );
    let awaiting_instance = ModelInstanceId::new(model_id, 18);
    let mut awaiting_request = routing_request.clone();
    awaiting_request.instance_id = awaiting_instance;
    let awaiting_contract = awaiting_request
        .dexterity_launch
        .as_mut()
        .expect("awaiting execution retains its Dexterity launch contract");
    awaiting_contract.lane_id = "lane-mt017-routing-awaiting".into();
    awaiting_contract.lane_span_id = "span-lane-mt017-routing-awaiting".into();
    let awaiting_launch_plan = vec![
        ModelLaneRoutingStageLaunchPlan {
            stage_id: "validation-candidate".into(),
            dispatch_target: ModelLaneRoutingDispatchTarget::LocalModel,
            lane_id: Some("lane-mt017-routing-awaiting".into()),
            model_id: Some(awaiting_instance.model_id.to_string()),
            provider: None,
        },
        ModelLaneRoutingStageLaunchPlan {
            stage_id: "validator-verdict".into(),
            dispatch_target: ModelLaneRoutingDispatchTarget::Validator,
            lane_id: Some("lane-mt017-validator".into()),
            model_id: None,
            provider: None,
        },
    ];
    let mut awaiting_decision_input = routing_decision.inner.clone();
    awaiting_decision_input.decision_id = "decision-mt017-diagnostics-retained-awaiting".into();
    awaiting_decision_input.decision_span_id =
        "span-decision-mt017-diagnostics-retained-awaiting".into();
    awaiting_decision_input.routing_launch_plan = awaiting_launch_plan;
    awaiting_decision_input.promotion_gate_ref =
        "promotion-gate://mt017/diagnostics-retained-awaiting".into();
    awaiting_decision_input.promotion_receipt_ref =
        Some("promotion-receipt://mt017/diagnostics-retained-awaiting".into());
    awaiting_decision_input.idempotency_key =
        "idem-decision-mt017-diagnostics-retained-awaiting".into();
    awaiting_decision_input.replay_order_key =
        "00000091/promotion/mt017-diagnostics-retained-awaiting".into();
    let awaiting_decision = store
        .record_promotion_decision(awaiting_decision_input)
        .await
        .expect("record approved retained-awaiting routing selection decision");
    assert_eq!(
        awaiting_decision.outcome,
        ModelLanePromotionOutcome::Approved
    );
    let (routing_ledger, _routing_ledger_drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("construct MT-017 diagnostics routing process ledger");
    let coordinator = SwarmCoordinator::new_with_model_lane_store(
        SwarmConfig::new(RunBudget::defaulted(2)),
        Arc::new(DiagnosticsRoutingFactory {
            ledger: routing_ledger.clone(),
        }),
        Arc::new(RecordingSwarmSink::new()),
        routing_ledger,
        store.clone(),
    );
    let routing_launches = vec![
        ModelLaneRoutingStageLaunch {
            stage_id: "validation-candidate".into(),
            request: Some(routing_request),
            generate_request: Some(GenerateRequest {
                id: routing_instance.model_id,
                prompt: GenPrompt::new("produce MT-017 diagnostics candidate"),
                sampling: SamplingParams::default(),
                lora_overrides: Vec::new(),
                steering_overrides: Vec::new(),
                kv_prefix_handle: None,
                cancel: CancellationToken::new(),
                max_tokens: 8,
                stop_sequences: Vec::new(),
                speculative_mode: None,
                structured_decoding: None,
            }),
            authority_lane_id: None,
            expected_run_id: "run-mt008-diag".into(),
            expected_lane_id: "lane-mt017-routing-local".into(),
            expected_model_id: routing_instance.model_id.to_string(),
            expected_provider: None,
        },
        ModelLaneRoutingStageLaunch {
            stage_id: "validator-verdict".into(),
            request: None,
            generate_request: None,
            authority_lane_id: Some("lane-mt017-validator".into()),
            expected_run_id: "run-mt008-diag".into(),
            expected_lane_id: "lane-mt017-validator".into(),
            expected_model_id: String::new(),
            expected_provider: None,
        },
    ];
    let awaiting_launches = vec![
        ModelLaneRoutingStageLaunch {
            stage_id: "validation-candidate".into(),
            request: Some(awaiting_request),
            generate_request: Some(GenerateRequest {
                id: awaiting_instance.model_id,
                prompt: GenPrompt::new("produce retained MT-017 diagnostics candidate"),
                sampling: SamplingParams::default(),
                lora_overrides: Vec::new(),
                steering_overrides: Vec::new(),
                kv_prefix_handle: None,
                cancel: CancellationToken::new(),
                max_tokens: 8,
                stop_sequences: Vec::new(),
                speculative_mode: None,
                structured_decoding: None,
            }),
            authority_lane_id: None,
            expected_run_id: "run-mt008-diag".into(),
            expected_lane_id: "lane-mt017-routing-awaiting".into(),
            expected_model_id: awaiting_instance.model_id.to_string(),
            expected_provider: None,
        },
        ModelLaneRoutingStageLaunch {
            stage_id: "validator-verdict".into(),
            request: None,
            generate_request: None,
            authority_lane_id: Some("lane-mt017-validator".into()),
            expected_run_id: "run-mt008-diag".into(),
            expected_lane_id: "lane-mt017-validator".into(),
            expected_model_id: String::new(),
            expected_provider: None,
        },
    ];
    let routing_context = ModelLaneRoutingExecutionContext {
        run_id: "run-mt008-diag".into(),
        trace_id: "trace-run-mt008-diag".into(),
        run_span_id: "span-run-mt008-diag".into(),
        coordinator_session_id: "coordinator-run-mt008-diag".into(),
        locus_ref: "locus://wp1/mt008/run-mt008-diag/coordinator-run-mt008-diag".into(),
        work_packet_id: WP_ID.into(),
        micro_task_id: Some("MT-008".into()),
        task_board_id: "task-board://wp-1".into(),
        owner_session: OWNER.into(),
        initial_input_ref: format!("model-lane-message://{}", source_record.message_id),
        initial_input_sha256: source_record.payload_sha256.clone(),
    };
    let failed_routing_batch = coordinator
        .execute_routing_lifecycle(
            "execution-mt017-diagnostics-authority-failed",
            &routing_decision.decision_id,
            &ModelLaneRoutingAuthority::default(),
            routing_context.clone(),
            routing_launches.clone(),
        )
        .await
        .expect("persist real MT-017 begin-time authority failure");
    assert_eq!(
        failed_routing_batch.execution.status,
        handshake_core::swarm_orchestration::routing_execution::ModelLaneRoutingExecutionStatus::Failed
    );
    assert!(failed_routing_batch
        .execution
        .failure_reason
        .as_deref()
        .is_some_and(|reason| reason.contains("authority")));
    let cancellation_source = coordinator
        .execute_routing_lifecycle(
            "execution-mt017-diagnostics-cancelled",
            &routing_decision.decision_id,
            &ModelLaneRoutingAuthority {
                cloud_consent_receipt_ref: None,
                validator_authority_ref: Some("validator://mt017/diagnostics".into()),
                operator_authority_ref: None,
            },
            routing_context.clone(),
            routing_launches.clone(),
        )
        .await
        .expect("drive real MT-017 cancellation source to awaiting authority");
    assert_eq!(
        cancellation_source.execution.status,
        handshake_core::swarm_orchestration::routing_execution::ModelLaneRoutingExecutionStatus::AwaitingAuthority,
        "routing failure={:?}; stages={:?}",
        cancellation_source.execution.failure_reason,
        cancellation_source.execution.stages,
    );
    let cancelled_routing = coordinator
        .cancel_routing_execution(
            "execution-mt017-diagnostics-cancelled",
            "operator cancelled MT-017 diagnostics execution",
        )
        .await
        .expect("persist real MT-017 routing cancellation");
    assert_eq!(
        cancelled_routing.status,
        handshake_core::swarm_orchestration::routing_execution::ModelLaneRoutingExecutionStatus::Cancelled
    );
    assert_eq!(
        cancelled_routing.cancel_reason.as_deref(),
        Some("operator cancelled MT-017 diagnostics execution")
    );
    let routing_batch = coordinator
        .execute_routing_lifecycle(
            "execution-mt017-diagnostics-awaiting",
            &awaiting_decision.decision_id,
            &ModelLaneRoutingAuthority {
                cloud_consent_receipt_ref: None,
                validator_authority_ref: Some("validator://mt017/diagnostics".into()),
                operator_authority_ref: None,
            },
            routing_context,
            awaiting_launches,
        )
        .await
        .expect("drive real MT-017 routing lifecycle to awaiting authority");
    assert_eq!(
        routing_batch.execution.status,
        handshake_core::swarm_orchestration::routing_execution::ModelLaneRoutingExecutionStatus::AwaitingAuthority
    );

    let reminted_model_id = handshake_core::model_runtime::ModelId::new_v7();
    let reminted_registration = handshake_core::model_runtime::ModelRegistration {
        model_id: reminted_model_id.clone(),
        artifact_path: std::path::PathBuf::from("models/mt014-diagnostics-model-moved.gguf"),
        sha256: [0x5a; 32],
        runtime_binding: handshake_core::model_runtime::RuntimeBinding::LlamaCpp,
        declared_capabilities: handshake_core::model_runtime::ModelCapabilities::default(),
        base_model_tag: handshake_core::model_runtime::BaseModelTag::new("mt014-diagnostics-base"),
        registered_at_utc: chrono::Utc::now(),
        registered_by: handshake_core::model_runtime::OperatorId::new("mt014-diagnostics-proof"),
        provider: handshake_core::model_runtime::ProviderKind::Local,
    };
    handshake_core::model_runtime::ModelRegistryStore::new(pool.clone())
        .persist_and_read_back(&reminted_registration)
        .await
        .expect("persist re-minted UUID observation for the same artifact");
    let mut reminted_registry = handshake_core::model_runtime::ModelRegistry::default();
    reminted_registry
        .register(reminted_registration)
        .expect("register re-minted UUID in current catalog");
    reminted_registry
        .mark_loaded(reminted_model_id)
        .expect("mark re-minted catalog model loaded");
    let catalog = handshake_core::model_runtime::ModelCatalog::from_registry(std::sync::Arc::new(
        reminted_registry,
    ));

    let projection = store
        .diagnostics_projection_with_model_catalog("run-mt008-diag", Some(catalog.as_ref()))
        .await
        .expect("projection from PostgreSQL/EventLedger rows");

    assert_eq!(
        projection.surface_contract_id,
        "native_swarm_lane_diagnostics"
    );
    assert_eq!(projection.run.run_id, "run-mt008-diag");
    assert_eq!(
        projection.run.coordinator_session_id,
        "coordinator-run-mt008-diag"
    );
    assert_eq!(projection.run.routing_policy, "mixed_local_cloud_subagent");
    assert_eq!(
        projection.run.artifact_namespace,
        "artifact://model-lane/run-mt008-diag"
    );
    assert_eq!(projection.run.work_packet_id.as_deref(), Some(WP_ID));
    assert_eq!(projection.run.micro_task_id.as_deref(), Some("MT-008"));
    assert_eq!(
        projection.run.task_board_id.as_deref(),
        Some("task-board://wp-1")
    );
    assert_eq!(projection.run.owner_session, OWNER);
    assert_eq!(projection.run.context_bundle_id, "ctx-run-mt008-diag");
    assert_eq!(
        projection.run.memory_pack_ref,
        "memory-pack://fems/run-mt008-diag"
    );
    assert!(!projection.run.event_ledger_event_id.is_empty());
    assert!(projection.run.event_ledger_seq > 0);
    assert_eq!(
        projection.run.flight_recorder_correlation_id,
        projection.run.event_ledger_event_id
    );
    assert_eq!(
        projection.lanes.len(),
        5,
        "three display lanes plus two real routing-execution lanes"
    );
    let known_lane = projection
        .lanes
        .iter()
        .find(|lane| lane.lane_id == "lane-mt008-local")
        .expect("known-model lane projected");
    let stale_lane = projection
        .lanes
        .iter()
        .find(|lane| lane.lane_id == "lane-mt014-stale")
        .expect("stale-model lane projected");
    assert_eq!(known_lane.model_id.as_deref(), Some(model_id_text.as_str()));
    assert_eq!(known_lane.model_display_name, "mt014-diagnostics-base");
    assert_eq!(
        known_lane.model_stable_anchor.as_deref(),
        Some("5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a")
    );
    assert!(known_lane.model_anchor_unavailable_reason.is_none());
    assert_eq!(
        stale_lane.model_id.as_deref(),
        Some(stale_model_id.as_str())
    );
    assert_eq!(
        stale_lane.model_display_name,
        handshake_core::model_runtime::UNKNOWN_MODEL_LABEL
    );
    assert!(stale_lane
        .model_anchor_unavailable_reason
        .as_deref()
        .is_some_and(|reason| reason.contains("legacy")));
    assert_eq!(
        known_lane.message_count, 1,
        "lane message_count is scoped to messages originating from this lane"
    );
    assert_eq!(known_lane.role, "implementer");
    assert_eq!(known_lane.session_id, "session-lane-mt008-local");
    assert_eq!(
        known_lane.model_session_id,
        "model-session-lane-mt008-local"
    );
    assert_eq!(known_lane.launch_authority, "model_runtime");
    assert_eq!(
        known_lane.locus_ref.as_deref(),
        Some("locus://wp1/mt008/run-mt008-diag/session-lane-mt008-local")
    );
    assert_eq!(
        known_lane.last_runtime_status_ref.as_deref(),
        Some("runtime-status://mt008/running")
    );
    assert_eq!(
        known_lane.recovery_hint_ref.as_deref(),
        Some("usermanual://dexterity/diagnostics#lane")
    );
    assert_eq!(known_lane.orphan_state, "reclaimable");
    assert_eq!(
        known_lane.flight_recorder_correlation_id,
        known_lane.event_ledger_event_id
    );
    assert_eq!(projection.messages.len(), 5);
    assert_eq!(
        projection.messages[0].payload_ref,
        "artifact://model-lane/messages/msg-mt008-001"
    );
    assert_eq!(projection.messages[0].to_lane, "coordinator");
    assert_eq!(
        projection.messages[0].routing_target_role.as_deref(),
        Some("coordinator")
    );
    assert_eq!(
        projection.messages[0].routing_correlation_id.as_deref(),
        Some("corr-run-mt008-diag-msg-mt008-001")
    );
    assert!(projection.messages[0].routing_requires_ack);
    assert_eq!(
        projection.messages[0].promotion_decision_id.as_deref(),
        Some("promotion://msg-mt008-001")
    );
    assert_eq!(
        projection.messages[0].promotion_gate_ref.as_deref(),
        Some("promotion-gate://msg-mt008-001")
    );
    assert_eq!(
        projection.messages[0].promotion_receipt_ref.as_deref(),
        Some("promotion-receipt://msg-mt008-001")
    );
    assert_eq!(
        projection.messages[0].proposal_ref.as_deref(),
        Some("proposal://mt008/msg-mt008-001")
    );
    // The projected CRDT posture is COMPLETE and every ref is real: the update
    // ref is the persisted `postgres://` reference resolved at admission, and
    // `crdt_proposal_ref` names an approved + applied `knowledge_crdt_ai_edit_proposals`
    // row minted through the real product API chain and bound (by
    // `applied_update_id`) to the SAME update this message cites. The native
    // schema-v3 consumer requires exactly this complete set whenever a message
    // targets CRDT at all.
    assert_eq!(
        projection.messages[0].crdt_update_ref.as_deref(),
        Some(crdt_posture.crdt_update_ref.as_str())
    );
    assert_eq!(
        projection.messages[0].crdt_base_snapshot_ref.as_deref(),
        Some(crdt_posture.crdt_base_snapshot_ref.as_str())
    );
    assert_eq!(
        projection.messages[0].crdt_state_vector.as_deref(),
        Some(crdt_posture.crdt_state_vector.as_str())
    );
    assert_eq!(
        projection.messages[0].crdt_proposal_ref.as_deref(),
        Some(crdt_proposal_ref.as_str())
    );
    assert_eq!(
        projection.messages[0].recovery_hint_ref.as_deref(),
        Some("usermanual://dexterity/diagnostics#message")
    );
    assert_eq!(
        projection.messages[0].flight_recorder_correlation_id,
        projection.messages[0].event_ledger_event_id
    );
    assert_eq!(
        projection.messages[0].loom_ref.as_deref(),
        Some("loom://run-mt008-diag/msg-mt008-001")
    );
    assert_eq!(
        projection.messages[0].fems_ref.as_deref(),
        Some("fems://run-mt008-diag/msg-mt008-001")
    );
    assert_eq!(projection.diagnostic_tiers.len(), 3);
    assert!(projection
        .diagnostic_tiers
        .iter()
        .any(|tier| tier.tier == "flight_recorder" && tier.state == "wired"));
    assert_eq!(projection.mt_runtime_statuses.len(), 1);
    assert_eq!(projection.mt_runtime_statuses[0].micro_task_id, "MT-008");
    assert_eq!(projection.routing_executions.len(), 3);
    let failed_routing = projection
        .routing_executions
        .iter()
        .find(|execution| execution.execution_id == "execution-mt017-diagnostics-authority-failed")
        .expect("begin-time authority failure projected");
    assert_eq!(failed_routing.status, "failed");
    assert!(failed_routing
        .failure_reason
        .as_deref()
        .is_some_and(|reason| reason.contains("authority")));
    let cancelled_routing = projection
        .routing_executions
        .iter()
        .find(|execution| execution.execution_id == "execution-mt017-diagnostics-cancelled")
        .expect("cancelled execution projected");
    assert_eq!(cancelled_routing.status, "cancelled");
    assert_eq!(
        cancelled_routing.cancel_reason.as_deref(),
        Some("operator cancelled MT-017 diagnostics execution")
    );
    assert!(cancelled_routing
        .stages
        .iter()
        .all(|stage| stage.outbox.status == "acked" || stage.outbox.status == "cancelled"));
    let routing = projection
        .routing_executions
        .iter()
        .find(|execution| execution.execution_id == "execution-mt017-diagnostics-awaiting")
        .expect("awaiting-authority execution projected");
    assert_eq!(routing.execution_id, "execution-mt017-diagnostics-awaiting");
    assert_eq!(routing.status, "awaiting_authority");
    assert!(routing.revision >= 4);
    assert!(routing.failure_reason.is_none());
    assert!(routing.cancel_reason.is_none());
    assert_eq!(
        routing.validator_authority_ref.as_deref(),
        Some("validator://mt017/diagnostics")
    );
    assert!(!routing.selecting_decision_event_id.is_empty());
    assert!(routing.selecting_decision_event_seq > 0);
    assert!(!routing.event_ledger_event_id.is_empty());
    assert!(routing.event_ledger_seq > 0);
    let candidate_stage = routing
        .stages
        .iter()
        .find(|stage| stage.stage_id == "validation-candidate")
        .expect("successful routing candidate stage projected");
    assert_eq!(candidate_stage.state, "succeeded");
    assert_eq!(candidate_stage.outbox.status, "acked");
    assert!(candidate_stage.output_ref.is_some());
    assert!(candidate_stage.output_message_ref.is_some());
    assert!(candidate_stage.output_sha256.is_some());
    let authority_stage = routing
        .stages
        .iter()
        .find(|stage| stage.stage_id == "validator-verdict")
        .expect("awaiting-authority routing stage projected");
    assert_eq!(authority_stage.state, "awaiting_authority");
    assert_eq!(
        authority_stage.dependency_stage_ids,
        vec!["validation-candidate"]
    );
    assert_eq!(authority_stage.outbox.status, "claimed");
    assert_eq!(
        authority_stage.outbox.lease_owner,
        authority_stage.lease_owner
    );
    assert_eq!(
        authority_stage.outbox.fencing_token,
        authority_stage.fencing_token
    );
    assert_eq!(
        authority_stage.outbox.lease_expires_at_unix_ms,
        authority_stage.lease_expires_at_unix_ms
    );
    assert!(authority_stage.authority_request_message_ref.is_some());
    assert!(authority_stage.authority_ref.is_some());
    assert!(!authority_stage.event_ledger_event_id.is_empty());
    assert!(!authority_stage.outbox.event_ledger_event_id.is_empty());
    assert_eq!(
        projection.reclaimable_lease_ids,
        vec!["lease-mt008-expired"]
    );

    let latest = store
        .latest_diagnostics_projection_with_model_catalog(Some(catalog.as_ref()))
        .await
        .expect("latest diagnostics projection resolves newest run");
    assert_eq!(latest.run.run_id, "run-mt008-diag");
    let serialized = serde_json::to_value(&projection).expect("serialize backend JSON contract");
    let serialized_known = serialized["lanes"]
        .as_array()
        .and_then(|lanes| {
            lanes
                .iter()
                .find(|lane| lane["lane_id"] == "lane-mt008-local")
        })
        .expect("serialized known-model lane");
    assert_eq!(serialized_known["model_id"], model_id_text);
    assert_eq!(
        serialized_known["model_display_name"],
        "mt014-diagnostics-base"
    );
    assert_eq!(serialized_known["model_stable_anchor"], "5a".repeat(32));
    let (cloud_label, cloud_reason) =
        handshake_core::swarm_orchestration::model_lane::diagnostics_model_identity_label(
            "cloud_model",
            "cloud",
            "openai",
            Some("gpt-4o-mini"),
            None,
            Some(catalog.as_ref()),
        );
    assert_eq!(cloud_label, "openai / gpt-4o-mini");
    assert!(cloud_reason.is_none());
    let (subagent_label, subagent_reason) =
        handshake_core::swarm_orchestration::model_lane::diagnostics_model_identity_label(
            "subagent",
            "subagent",
            "subagent",
            Some("subagent://coder"),
            None,
            Some(catalog.as_ref()),
        );
    assert_eq!(subagent_label, "subagent / subagent://coder");
    assert!(subagent_reason.is_none());

    let recorder = Arc::new(NoopRecorder);
    let state = handshake_core::AppState {
        storage: Arc::new(handshake_core::storage::postgres::PostgresDatabase::new(
            pool.clone(),
        )),
        flight_recorder: recorder.clone(),
        diagnostics: recorder,
        llm_client: Arc::new(CatalogLlmClient {
            inner: handshake_core::llm::InMemoryLlmClient::new("mounted-diagnostics-proof".into()),
            catalog: catalog.clone(),
        }),
        capability_registry: Arc::new(handshake_core::capabilities::CapabilityRegistry::new()),
        session_registry: Arc::new(handshake_core::workflows::SessionRegistry::new(
            handshake_core::workflows::SessionSchedulerConfig::from_env(),
        )),
        postgres_pool: pool.clone(),
    };
    let app = handshake_core::api::diagnostics::routes(state.clone()).layer(Extension(
        handshake_core::api::account_scope::ProductLocalResourceScope::from_exact(
            diagnostics_exact.clone(),
        )
        .expect("install exact product diagnostics scope"),
    ));
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/swarm/model-lanes/diagnostics/run-mt008-diag")
                .header(
                    "x-handshake-owner-account",
                    diagnostics_owner.as_uuid().to_string(),
                )
                .body(Body::empty())
                .expect("build mounted diagnostics request"),
        )
        .await
        .expect("mounted diagnostics route responds");
    assert_eq!(response.status(), StatusCode::OK);

    // A caller assertion may only equal the server-owned scope. It cannot select
    // another owner, and the denial remains identifier-free.
    let wrong_owner_assertion = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/swarm/model-lanes/diagnostics/run-mt008-diag")
                .header(
                    "x-handshake-owner-account",
                    OwnerAccountId::mint().as_uuid().to_string(),
                )
                .body(Body::empty())
                .expect("build wrong-owner diagnostics request"),
        )
        .await
        .expect("wrong-owner diagnostics route responds");
    assert_eq!(
        wrong_owner_assertion.status(),
        StatusCode::FORBIDDEN,
        "a caller assertion cannot replace the server-owned account"
    );
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read mounted diagnostics response body");
    let http_projection: handshake_core::swarm_orchestration::model_lane::ModelLaneDiagnosticsProjection =
        serde_json::from_slice(&body).expect("typed mounted diagnostics projection envelope");
    assert_eq!(
        http_projection.schema_id,
        "hsk.model_lane_diagnostics_projection@3"
    );
    assert_eq!(http_projection.run.run_id, "run-mt008-diag");
    let http_json: serde_json::Value =
        serde_json::from_slice(&body).expect("read scoped diagnostics response JSON");
    let exact_scope_values = [
        diagnostics_exact.owner_account_id.to_string(),
        diagnostics_exact.actor_principal_id.to_string(),
        diagnostics_exact.authenticated_session_id.to_string(),
        diagnostics_exact.access_space_id.to_string(),
        diagnostics_exact.workspace_id.to_string(),
    ];
    let safe_scope = http_json["resource_scope"]
        .as_object()
        .expect("HTTP diagnostics exposes a typed privacy-safe scope contract");
    assert_eq!(
        safe_scope.len(),
        7,
        "privacy-safe scope has no raw-id fields"
    );
    let fingerprints = [
        "owner_account_fingerprint",
        "actor_principal_fingerprint",
        "authenticated_session_fingerprint",
        "access_space_fingerprint",
        "workspace_fingerprint",
    ]
    .map(|field| {
        let value = safe_scope[field]
            .as_str()
            .unwrap_or_else(|| panic!("privacy-safe scope omitted {field}"));
        assert!(
            value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()),
            "{field} must be a keyed opaque fingerprint"
        );
        value
    });
    assert_eq!(
        fingerprints
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        5,
        "distinct exact scope dimensions must have distinct fingerprints"
    );
    assert_eq!(safe_scope["visibility"], "private_exact_scope_only");
    assert_eq!(
        safe_scope["denial_posture"],
        "foreign_scope_is_absent_restricted_metadata_withheld"
    );
    let http_text = String::from_utf8_lossy(&body);
    for raw_scope_value in &exact_scope_values {
        assert!(
            !http_text.contains(raw_scope_value),
            "privacy-safe HTTP projection leaked an actual exact-scope value"
        );
    }
    let http_known_lane = http_projection
        .lanes
        .iter()
        .find(|lane| lane.lane_id == "lane-mt008-local")
        .expect("mounted route serializes the known-model lane");
    assert_eq!(http_known_lane.model_display_name, "mt014-diagnostics-base");
    assert_eq!(
        http_known_lane.model_stable_anchor.as_deref(),
        Some("5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a")
    );
    assert_eq!(http_projection.routing_executions.len(), 3);
    assert!(http_projection
        .routing_executions
        .iter()
        .any(|execution| execution.status == "awaiting_authority"));
    assert!(http_projection
        .routing_executions
        .iter()
        .any(|execution| execution.status == "failed"));
    assert!(http_projection
        .routing_executions
        .iter()
        .any(|execution| execution.status == "cancelled"));

    // Re-host the same route under authorities that differ in exactly one
    // dimension. Every request must be absent-shaped, proving actor/session/
    // AccessSpace are enforced even though callers cannot override those
    // server-owned dimensions with headers.
    let mut wrong_owner_scope = diagnostics_exact.clone();
    wrong_owner_scope.owner_account_id = OwnerAccountId::mint();
    let mut wrong_actor_scope = diagnostics_exact.clone();
    wrong_actor_scope.actor_principal_id = ActorPrincipalId::mint();
    let mut wrong_session_scope = diagnostics_exact.clone();
    wrong_session_scope.authenticated_session_id = AuthenticatedSessionRef::mint();
    let mut wrong_access_space_scope = diagnostics_exact.clone();
    wrong_access_space_scope.access_space_id = AccessSpaceRef::mint();
    let mut wrong_workspace_scope = diagnostics_exact.clone();
    wrong_workspace_scope.workspace_id =
        WorkspaceScopeRef::new("workspace-mt008-foreign").expect("foreign workspace nonblank");
    for (dimension, foreign_scope) in [
        ("owner_account_id", wrong_owner_scope),
        ("actor_principal_id", wrong_actor_scope),
        ("authenticated_session_id", wrong_session_scope),
        ("access_space_id", wrong_access_space_scope),
        ("workspace_id", wrong_workspace_scope),
    ] {
        let foreign_app = handshake_core::api::diagnostics::routes(state.clone()).layer(Extension(
            handshake_core::api::account_scope::ProductLocalResourceScope::from_exact(
                foreign_scope,
            )
            .expect("install complete foreign diagnostics scope"),
        ));
        let response = foreign_app
            .oneshot(
                Request::builder()
                    .uri("/swarm/model-lanes/diagnostics/run-mt008-diag")
                    .body(Body::empty())
                    .expect("build foreign exact-scope diagnostics request"),
            )
            .await
            .expect("foreign exact-scope diagnostics route responds");
        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "foreign {dimension} must not distinguish a protected run from absence"
        );
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read foreign exact-scope denial");
        let denial = String::from_utf8_lossy(&body);
        for protected in ["run-mt008-diag", "lane-mt008-local", "msg-mt008"] {
            assert!(
                !denial.contains(protected),
                "foreign {dimension} denial leaked protected identifier {protected}"
            );
        }
    }

    // Account/AccessSpace switches can overlap while separate native clients are
    // still reading. Drive those authorities concurrently against the same real
    // PostgreSQL state: the authorized projection must retain its exact scope
    // while both foreign reads remain absent-shaped and metadata-free.
    let authorized_app = handshake_core::api::diagnostics::routes(state.clone()).layer(Extension(
        handshake_core::api::account_scope::ProductLocalResourceScope::from_exact(
            diagnostics_exact.clone(),
        )
        .expect("install concurrent authorized diagnostics scope"),
    ));
    let mut concurrent_foreign_owner = diagnostics_exact.clone();
    concurrent_foreign_owner.owner_account_id = OwnerAccountId::mint();
    let foreign_owner_app =
        handshake_core::api::diagnostics::routes(state.clone()).layer(Extension(
            handshake_core::api::account_scope::ProductLocalResourceScope::from_exact(
                concurrent_foreign_owner,
            )
            .expect("install concurrent foreign-owner diagnostics scope"),
        ));
    let mut concurrent_foreign_space = diagnostics_exact.clone();
    concurrent_foreign_space.access_space_id = AccessSpaceRef::mint();
    let foreign_space_app =
        handshake_core::api::diagnostics::routes(state.clone()).layer(Extension(
            handshake_core::api::account_scope::ProductLocalResourceScope::from_exact(
                concurrent_foreign_space,
            )
            .expect("install concurrent foreign-AccessSpace diagnostics scope"),
        ));
    let request = || {
        Request::builder()
            .uri("/swarm/model-lanes/diagnostics/run-mt008-diag")
            .body(Body::empty())
            .expect("build concurrent diagnostics request")
    };
    let (authorized, foreign_owner, foreign_space) = tokio::join!(
        authorized_app.oneshot(request()),
        foreign_owner_app.oneshot(request()),
        foreign_space_app.oneshot(request()),
    );
    let authorized = authorized.expect("concurrent authorized diagnostics response");
    let foreign_owner = foreign_owner.expect("concurrent foreign-owner diagnostics response");
    let foreign_space = foreign_space.expect("concurrent foreign-Space diagnostics response");
    assert_eq!(authorized.status(), StatusCode::OK);
    assert_eq!(foreign_owner.status(), StatusCode::NOT_FOUND);
    assert_eq!(foreign_space.status(), StatusCode::NOT_FOUND);
    let authorized_body = axum::body::to_bytes(authorized.into_body(), usize::MAX)
        .await
        .expect("read concurrent authorized diagnostics body");
    let authorized_json: serde_json::Value =
        serde_json::from_slice(&authorized_body).expect("concurrent authorized body is JSON");
    assert_eq!(
        authorized_json["resource_scope"],
        http_json["resource_scope"],
        "concurrent foreign account/Space reads cannot change the authorized privacy-safe scope commitment"
    );
    let authorized_text = String::from_utf8_lossy(&authorized_body);
    for raw_scope_value in &exact_scope_values {
        assert!(
            !authorized_text.contains(raw_scope_value),
            "concurrent authorized response leaked an actual exact-scope value"
        );
    }
    for (dimension, response) in [
        ("owner_account_id", foreign_owner),
        ("access_space_id", foreign_space),
    ] {
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read concurrent foreign diagnostics denial");
        let denial = String::from_utf8_lossy(&body);
        for protected in ["run-mt008-diag", "lane-mt008-local", "msg-mt008"] {
            assert!(
                !denial.contains(protected),
                "concurrent foreign {dimension} denial leaked {protected}"
            );
        }
    }
    coordinator
        .drain_all()
        .await
        .expect("release MT-017 diagnostics routing runtime after projection capture");

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/swarm/model-lanes/diagnostics/run-does-not-exist")
                // Scoped, so this asserts the ABSENT case for an authorized
                // caller. Without the header the route would refuse at the
                // scope boundary (403) and never reach the not-found path this
                // leg exists to pin.
                .header(
                    "x-handshake-owner-account",
                    diagnostics_owner.as_uuid().to_string(),
                )
                .body(Body::empty())
                .expect("build missing-run diagnostics request"),
        )
        .await
        .expect("mounted missing-run route responds");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read typed diagnostics error envelope");
    let error: DiagnosticsStatusEnvelope =
        serde_json::from_slice(&body).expect("deserialize typed diagnostics status envelope");
    assert_eq!(error.error, "MODEL_LANE_DIAGNOSTICS_NOT_FOUND");
    // DELIBERATELY INVERTED from `detail.contains("run-does-not-exist")`.
    //
    // Under HBR-PRIV the scoped store answers "absent" for BOTH a run that does
    // not exist and a run that exists but belongs to another account. That is
    // only an anti-oracle if the two answers are indistinguishable, which means
    // the detail must be a constant that does not vary with the request. An
    // echoed run id is a per-request detail, so requiring it would have locked
    // in a shape where a future "not found: <id>" vs "forbidden" split could
    // reappear and leak existence. The stable code above is what a caller
    // should branch on.
    assert!(
        !error.detail.contains("run-does-not-exist"),
        "the not-found detail must not echo the requested run id, or absence stops being uniform: {}",
        error.detail
    );

    let routing_command_id =
        "routing-command:execution-mt017-diagnostics-awaiting:validator-verdict:1";
    let (original_outbox_event_id, original_outbox_event_seq) =
        sqlx::query_as::<_, (String, i64)>(
        "SELECT event_ledger_event_id, event_ledger_seq FROM model_lane_routing_outbox WHERE command_id = $1",
    )
    .bind(routing_command_id)
    .fetch_one(&pool)
    .await
    .expect("read canonical routing outbox EventLedger lineage");
    sqlx::query("UPDATE model_lane_routing_outbox SET event_ledger_seq = $2 WHERE command_id = $1")
        .bind(routing_command_id)
        .bind(original_outbox_event_seq + 1_000_000)
        .execute(&pool)
        .await
        .expect("tamper routing outbox EventLedger lineage for fail-closed proof");
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/swarm/model-lanes/diagnostics/run-mt008-diag")
                .header(
                    "x-handshake-owner-account",
                    diagnostics_owner.as_uuid().to_string(),
                )
                .body(Body::empty())
                .expect("build routing-lineage integrity request"),
        )
        .await
        .expect("mounted routing-lineage integrity request responds");
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read routing-lineage integrity envelope");
    let error: DiagnosticsStatusEnvelope =
        serde_json::from_slice(&body).expect("deserialize routing-lineage integrity envelope");
    assert_eq!(error.error, "MODEL_LANE_DIAGNOSTICS_INTEGRITY_FAILURE");
    assert!(
        error.detail.contains("routing outbox"),
        "got {}",
        error.detail
    );
    sqlx::query(
        "UPDATE model_lane_routing_outbox SET event_ledger_event_id = $2, event_ledger_seq = $3 WHERE command_id = $1",
    )
    .bind(routing_command_id)
    .bind(original_outbox_event_id)
    .bind(original_outbox_event_seq)
    .execute(&pool)
    .await
    .expect("restore canonical routing outbox EventLedger lineage");

    sqlx::query(
        r#"UPDATE model_lanes
           SET record_json = jsonb_set(record_json, '{role}', '"tampered-role"'::jsonb)
           WHERE lane_id = $1"#,
    )
    .bind("lane-mt008-local")
    .execute(&pool)
    .await
    .expect("tamper persisted lane authority for deterministic integrity proof");
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/swarm/model-lanes/diagnostics/run-mt008-diag")
                .header(
                    "x-handshake-owner-account",
                    diagnostics_owner.as_uuid().to_string(),
                )
                .body(Body::empty())
                .expect("build integrity-failure diagnostics request"),
        )
        .await
        .expect("mounted integrity-failure route responds");
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read integrity diagnostics error envelope");
    let error: DiagnosticsStatusEnvelope =
        serde_json::from_slice(&body).expect("deserialize integrity diagnostics envelope");
    assert_eq!(error.error, "MODEL_LANE_DIAGNOSTICS_INTEGRITY_FAILURE");

    pool.close().await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/swarm/model-lanes/diagnostics/latest")
                .header(
                    "x-handshake-owner-account",
                    diagnostics_owner.as_uuid().to_string(),
                )
                .body(Body::empty())
                .expect("build unavailable-authority diagnostics request"),
        )
        .await
        .expect("mounted unavailable-authority route responds");
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read unavailable-authority diagnostics error envelope");
    let error: DiagnosticsStatusEnvelope = serde_json::from_slice(&body)
        .expect("deserialize unavailable-authority diagnostics envelope");
    assert_eq!(error.error, "MODEL_LANE_DIAGNOSTICS_AUTHORITY_UNAVAILABLE");

    // Publish only after every backend assertion has passed. The native proof
    // consumes this completion receipt with the same invocation nonce.
    let artifact_root = std::env::var("HANDSHAKE_ARTIFACTS_DIR")
        .expect("HANDSHAKE_ARTIFACTS_DIR must point at the Handshake_Artifacts directory");
    let artifact_root = std::fs::canonicalize(artifact_root)
        .expect("HANDSHAKE_ARTIFACTS_DIR must resolve to an existing directory");
    let manifest_dir = std::fs::canonicalize(env!("CARGO_MANIFEST_DIR"))
        .expect("backend crate manifest directory must resolve");
    let worktree_root = manifest_dir
        .parent()
        .and_then(std::path::Path::parent)
        .and_then(std::path::Path::parent)
        .expect("backend crate must live below the worktree src directory");
    let expected_root = std::fs::canonicalize(
        worktree_root
            .parent()
            .expect("worktree must have a parent")
            .join("Handshake_Artifacts"),
    )
    .expect("canonical sibling Handshake_Artifacts directory must exist");
    assert_eq!(
        artifact_root, expected_root,
        "artifact root must be the canonical sibling Handshake_Artifacts directory"
    );
    let artifact = artifact_root
        .join("handshake-test")
        .join("wp1-final-audit")
        .join("mt014_swarm_lane_diagnostics_projection.json");
    std::fs::create_dir_all(
        artifact
            .parent()
            .expect("diagnostics artifact has a parent directory"),
    )
    .expect("create standardized diagnostics artifact directory");
    let proof_nonce = std::env::var("HANDSHAKE_MT017_DIAGNOSTICS_PROOF_NONCE")
        .expect("fresh diagnostics proof nonce is required");
    let artifact_bytes = serde_json::to_vec_pretty(&http_projection)
        .expect("serialize typed mounted diagnostics artifact");
    let artifact_sha256 = hex::encode(Sha256::digest(&artifact_bytes));
    let producer_completed_at_unix_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock must be after Unix epoch")
        .as_millis() as u64;
    std::fs::write(&artifact, &artifact_bytes)
        .expect("write backend-generated diagnostics projection artifact");
    std::fs::write(
        artifact.with_extension("provenance.json"),
        serde_json::to_vec_pretty(&json!({
            "schema_id": "hsk.mt017_diagnostics_projection_provenance@1",
            "proof_nonce": proof_nonce,
            "projection_schema_id": http_projection.schema_id,
            "artifact_sha256": artifact_sha256,
            "producer_test_id": "swarm_lane_diagnostics_backend_projection_matches_eventledger",
            "producer_status": "passed_all_backend_assertions",
            "producer_completed_at_unix_ms": producer_completed_at_unix_ms,
        }))
        .expect("serialize diagnostics artifact provenance"),
    )
    .expect("write fresh diagnostics artifact completion receipt");
}

#[derive(Debug, Deserialize)]
struct DiagnosticsStatusEnvelope {
    error: String,
    detail: String,
}

#[tokio::test]
async fn swarm_lane_diagnostics_rejects_flight_recorder_only_hbr_posture() {
    let (_pool, store) = diagnostics_store().await;
    store
        .record_run(sample_run("run-mt008-fr-only", "lane-mt008-fr-only"))
        .await
        .expect("record diagnostics run");
    store
        .record_lane(sample_lane("lane-mt008-fr-only", "run-mt008-fr-only"))
        .await
        .expect("record diagnostics lane");
    store
        .record_message(sample_message(
            "msg-mt008-fr-only",
            "run-mt008-fr-only",
            "lane-mt008-fr-only",
        ))
        .await
        .expect("record diagnostics message");
    store
        .record_diagnostic_tier_status(sample_tier(
            "run-mt008-fr-only",
            ModelLaneDiagnosticTier::FlightRecorder,
            ModelLaneDiagnosticTierState::Wired,
            "eventledger://kernel/model-lane/diagnostics",
        ))
        .await
        .expect("record only FlightRecorder tier");

    let err = store
        .diagnostics_projection("run-mt008-fr-only")
        .await
        .expect_err("HBR-INT-009 posture must not accept FlightRecorder-only diagnostics");
    let err = err.to_string();
    assert!(
        err.contains("internal_diagnostics") || err.contains("palmistry"),
        "missing-tier error should name absent HBR tier, got {err}"
    );
}

async fn diagnostics_store() -> (sqlx::PgPool, ModelLaneStore) {
    let kpg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("PostgreSQL/EventLedger is required for MT-008 proof");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&kpg.schema_url)
        .await
        .expect("connect isolated diagnostics schema");
    let store = ModelLaneStore::new(pool.clone());
    (pool, store)
}

fn sample_run(run_id: &str, lane_id: &str) -> NewModelLaneRun {
    NewModelLaneRun {
        run_id: run_id.into(),
        trace_id: format!("trace-{run_id}"),
        run_span_id: format!("span-{run_id}"),
        coordinator_session_id: format!("coordinator-{run_id}"),
        routing_policy: "mixed_local_cloud_subagent".into(),
        context_bundle_id: format!("ctx-{run_id}"),
        lane_ids: vec![lane_id.into()],
        event_ledger_stream_id: format!("mlane-stream-{run_id}"),
        artifact_namespace: format!("artifact://model-lane/{run_id}"),
        projection_plan_ref: None,
        consent_receipt_ref: None,
        work_packet_id: Some(WP_ID.into()),
        micro_task_id: Some("MT-008".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: OWNER.into(),
        idempotency_key: format!("idem-{run_id}"),
        replay_order_key: "00000001/run".into(),
        replay_after_event_ledger_seq: None,
        recovery_state: ModelLaneRecoveryState::Restartable,
        failstate_code: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://dexterity/diagnostics".into()),
        locus_binding: Some(sample_locus(run_id, &format!("coordinator-{run_id}"))),
        memory_pack_ref: format!("memory-pack://fems/{run_id}"),
        memory_pack_hash: sample_sha256(),
        determinism_mode: "deterministic_replay".into(),
        budget_summary_ref: "budget://mt008".into(),
        selected_model_id: Some("model://mt008/local".into()),
        candidate_model_ids: vec!["model://mt008/local".into()],
        procedural_review_status: "reviewed_by_kernel_builder".into(),
        truncation_warning_ref: None,
        rejection_reason_refs: vec![],
    }
}

fn sample_lane(lane_id: &str, run_id: &str) -> NewModelLane {
    NewModelLane {
        lane_id: lane_id.into(),
        run_id: run_id.into(),
        trace_id: format!("trace-{run_id}"),
        lane_span_id: format!("span-{lane_id}"),
        event_ledger_stream_id: format!("mlane-stream-{run_id}"),
        kind: ModelLaneKind::LocalModel,
        role: "implementer".into(),
        backend: RuntimeBinding::Local.as_str().into(),
        model_id: Some("model://mt008/local".into()),
        session_id: format!("session-{lane_id}"),
        model_session_id: format!("model-session-{lane_id}"),
        adapter_id: "local-runtime".into(),
        runtime_binding: RuntimeBinding::Local,
        launch_authority: LaunchAuthority::ModelRuntime,
        provider_kind: ModelLaneProviderKind::LocalRuntime,
        capability_token_ids: vec!["capability://mt008/read".into()],
        effective_capability_snapshot_ref: Some("capability-snapshot://mt008".into()),
        capability_negotiation_ref: Some("capability-negotiation://mt008".into()),
        provider_feature_profile_ref: Some("provider-feature-profile://mt008".into()),
        requested_execution_policy_ref: Some("execution-policy://requested/mt008".into()),
        effective_execution_policy_ref: Some("execution-policy://effective/mt008".into()),
        projection_plan_ref: None,
        consent_receipt_ref: None,
        tool_gate_decision_refs: vec!["toolgate://mt008/allow".into()],
        status: ModelLaneStatus::Running,
        recovery_state: ModelLaneRecoveryState::Restartable,
        heartbeat_at_utc: Some("2026-06-30T00:00:00Z".into()),
        lease_expires_at_utc: Some("2099-01-01T00:00:00Z".into()),
        reclaim_after_utc: Some("2099-01-01T00:01:00Z".into()),
        restart_generation: 0,
        cancellation_ref: Some(format!("cancel-token://mt008/{lane_id}")),
        reclaim_policy_ref: Some("reclaim-policy://mt008".into()),
        terminal_status_mapping_ref: Some("terminal-status://mt008".into()),
        process_ownership_ref: Some(format!("process-ledger://mt008/{lane_id}")),
        no_os_process_reason_ref: None,
        backpressure_ref: None,
        loop_counter_ref: Some("loop-counter://mt008".into()),
        last_runtime_status_ref: Some("runtime-status://mt008/running".into()),
        last_recovery_event_ref: Some("recovery://mt008/running".into()),
        failstate_code: None,
        startup_failure_ref: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://dexterity/diagnostics#lane".into()),
        work_packet_id: Some(WP_ID.into()),
        micro_task_id: Some("MT-008".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: OWNER.into(),
        locus_binding: Some(sample_locus(run_id, &format!("session-{lane_id}"))),
    }
}

fn sample_message(message_id: &str, run_id: &str, lane_id: &str) -> NewModelLaneMessage {
    NewModelLaneMessage {
        message_id: message_id.into(),
        run_id: run_id.into(),
        trace_id: format!("trace-{run_id}"),
        message_span_id: format!("span-{message_id}"),
        parent_span_id: Some(format!("span-{lane_id}")),
        linked_span_contexts: vec![format!("trace-link://{run_id}/{lane_id}")],
        from_lane_id: lane_id.into(),
        to_lane: ModelLaneTarget::Coordinator,
        routing: Some(ModelLaneRoutingMetadata {
            target_role: "coordinator".into(),
            target_session: format!("coordinator-{run_id}"),
            correlation_id: format!("corr-{run_id}-{message_id}"),
            requires_ack: true,
            ack_for: None,
        }),
        kind: ModelLaneMessageKind::Proposal,
        payload_ref: format!("artifact://model-lane/messages/{message_id}"),
        payload_sha256: sha256_hex(message_id.as_bytes()),
        event_ledger_stream_id: format!("mlane-stream-{run_id}"),
        summary: "MT-008 diagnostics payload".into(),
        authority: ModelLaneAuthority::PromotionCandidate,
        promotion_decision_id: Some(format!("promotion://{message_id}")),
        promotion_gate_ref: Some(format!("promotion-gate://{message_id}")),
        promotion_receipt_ref: Some(format!("promotion-receipt://{message_id}")),
        validator_verdict_ref: None,
        operator_decision_ref: None,
        promoted_artifact_ref: Some(format!("artifact://promoted/{message_id}")),
        promoted_artifact_sha256: Some(sample_sha256()),
        promoted_artifact_version: Some("1".into()),
        tool_gate_decision_refs: vec!["toolgate://mt008/allow".into()],
        coordinator_session_id: format!("coordinator-{run_id}"),
        work_packet_id: Some(WP_ID.into()),
        micro_task_id: Some("MT-008".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: OWNER.into(),
        locus_binding: Some(sample_locus(run_id, &format!("session-{lane_id}"))),
        idempotency_key: format!("idem-{message_id}"),
        replay_order_key: "00000002/message".into(),
        replay_after_event_ledger_seq: Some(1),
        proposal_ref: Some(format!("proposal://mt008/{message_id}")),
        // No CRDT authority by default: synthetic `crdt-update://...` refs cannot
        // resolve against persisted kernel_crdt_updates and are denied at the
        // shared admission boundary. Tests that need CRDT-bearing diagnostics seed
        // a REAL Yjs update + snapshot + lease and set these fields to the real
        // refs (see `swarm_lane_diagnostics_backend_projection_matches_eventledger`).
        crdt_update_ref: None,
        crdt_base_snapshot_ref: None,
        crdt_state_vector: None,
        crdt_proposal_ref: None,
        crdt_stale_base_ref: None,
        failstate_code: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://dexterity/diagnostics#message".into()),
        created_at_utc: "2026-06-30T00:00:00Z".into(),
        diagnostic_payload: json!({
            "artifact_ref": format!("artifact://model-lane/messages/{message_id}"),
            "loom_ref": format!("loom://{run_id}/{message_id}"),
            "fems_ref": format!("fems://{run_id}/{message_id}"),
            "payload_error": null
        }),
    }
}

fn routing_input_payload(message: &NewModelLaneMessage) -> serde_json::Value {
    json!({
        "schema_id": "hsk.model_lane_message_payload@1",
        "message_id": message.message_id,
        "run_id": message.run_id,
        "payload_ref": message.payload_ref,
        "crdt_update_ref": message.crdt_update_ref,
        "locus": message
            .diagnostic_payload
            .get("locus_ref")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default(),
    })
}

fn routing_input_binding(
    message: &NewModelLaneMessage,
) -> NewModelLaneContextBundleArtifactBinding {
    NewModelLaneContextBundleArtifactBinding {
        artifact_binding_id: format!("artifact-binding-{}", message.message_id),
        run_id: message.run_id.clone(),
        trace_id: message.trace_id.clone(),
        artifact_ref: message.payload_ref.clone(),
        artifact_sha256: message.payload_sha256.clone(),
        content_hash: message.payload_sha256.clone(),
        artifact_kind: "model_lane_message_payload".into(),
        artifact_manifest_ref: format!(
            "artifact-store://model-lane/mt017/{}/artifact.json",
            message.message_id
        ),
        artifact_payload_ref: message.payload_ref.clone(),
        payload_json: routing_input_payload(message),
        event_ledger_stream_id: message.event_ledger_stream_id.clone(),
        work_packet_id: WP_ID.into(),
        micro_task_id: "MT-008".into(),
        task_board_id: "task-board://wp-1".into(),
        owner_session: OWNER.into(),
        idempotency_key: format!("idem-routing-input-binding-{}", message.message_id),
        created_at_utc: "2026-07-18T00:00:00Z".into(),
        diagnostic_payload: json!({
            "flight_recorder": "ArtifactStore/EventLedger binding for MT-017 routing diagnostics",
            "internal_diagnostics": "hbr-int-009",
        }),
    }
}

fn sample_lease(
    lease_id: &str,
    run_id: &str,
    lane_id: &str,
    lease_expires_at_utc: &str,
) -> NewModelLaneLease {
    NewModelLaneLease {
        lease_id: lease_id.into(),
        run_id: run_id.into(),
        lane_id: Some(lane_id.into()),
        scope: ModelLaneLeaseScope::Lane,
        scope_ref: format!("model-lane://{run_id}/{lane_id}"),
        holder_actor_id: "actor://kernel-builder/mt008".into(),
        holder_session_id: OWNER.into(),
        lease_expires_at_utc: lease_expires_at_utc.into(),
        takeover_policy_ref: "lease-policy://mt008/recover-or-reclaim".into(),
        state: ModelLaneLeaseState::Active,
        event_ledger_stream_id: format!("mlane-stream-{run_id}"),
        work_packet_id: WP_ID.into(),
        micro_task_id: "MT-008".into(),
        task_board_id: "task-board://wp-1".into(),
        owner_session: OWNER.into(),
        idempotency_key: format!("idem-{lease_id}"),
        recovery_hint_ref: Some("usermanual://dexterity/diagnostics#lease".into()),
        diagnostic_payload: json!({"tier": "internal_diagnostics"}),
    }
}

fn sample_tier(
    run_id: &str,
    tier: ModelLaneDiagnosticTier,
    state: ModelLaneDiagnosticTierState,
    evidence_ref: &str,
) -> NewModelLaneDiagnosticTierStatus {
    NewModelLaneDiagnosticTierStatus {
        diagnostic_status_id: format!("diag-{run_id}-mt008-{}", tier.as_str()),
        behavior_id: "HBR-INT-009".into(),
        run_id: run_id.into(),
        tier,
        state,
        reason: format!("MT-008 diagnostics posture for {run_id}"),
        evidence_ref: evidence_ref.into(),
        follow_up_ref: Some("palmistry://external-worktree/in-progress".into()),
        event_ledger_stream_id: format!("mlane-stream-{run_id}"),
        work_packet_id: WP_ID.into(),
        micro_task_id: "MT-008".into(),
        task_board_id: "task-board://wp-1".into(),
        owner_session: OWNER.into(),
        idempotency_key: format!("idem-diag-{run_id}-mt008-{}", tier.as_str()),
        diagnostic_payload: json!({"behavior_id": "HBR-INT-009", "run_id": run_id}),
    }
}

fn sample_mt_status(run_id: &str) -> NewModelLaneMtRuntimeStatus {
    NewModelLaneMtRuntimeStatus {
        mt_status_id: "mt-status-mt008-rfv".into(),
        run_id: run_id.into(),
        work_packet_id: WP_ID.into(),
        micro_task_id: "MT-008".into(),
        task_board_id: "task-board://wp-1".into(),
        status: ModelLaneMtRuntimeStatus::ReadyForValidation,
        claimed_by_ref: Some(format!("session://{OWNER}")),
        blocker_ref: None,
        missing_resource_ref: None,
        proof_status_ref: Some("proof://mt008/swarm_lane_diagnostics_pg_tests".into()),
        hbr_status_ref: Some("hbr-int-009://dexterity/diagnostics".into()),
        last_recovery_event_ref: None,
        last_runtime_status_ref: Some("runtime-status://mt008/ready-for-validation".into()),
        event_ledger_stream_id: format!("mlane-stream-{run_id}"),
        owner_session: OWNER.into(),
        idempotency_key: "idem-mt-status-mt008-rfv".into(),
        diagnostic_payload: json!({"state_recovery": true}),
    }
}

fn sample_locus(run_id: &str, session_id: &str) -> ModelLaneLocusBinding {
    let model_session_id = if let Some(lane_suffix) = session_id.strip_prefix("session-") {
        format!("model-session-{lane_suffix}")
    } else {
        format!("model-session-{session_id}")
    };
    ModelLaneLocusBinding {
        work_packet_id: WP_ID.into(),
        micro_task_id: "MT-008".into(),
        task_board_id: Some("task-board://wp-1".into()),
        coordinator_session_id: format!("coordinator-{run_id}"),
        session_id: session_id.into(),
        model_session_id,
        owner_session: OWNER.into(),
        locus_binding_ref: format!("locus://wp1/mt008/{run_id}/{session_id}"),
    }
}

fn sample_sha256() -> String {
    sha256_hex(b"mt008-diagnostics")
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

// ===========================================================================
// WP-1 MT-011 (HBR-PRIV-004/005): the diagnostics surface must be owner
// filtered, and must not become a cross-account side channel.
//
// The shared scope substrate was already proven (migrations 0363+0364,
// resource_scope.rs, api/account_scope.rs, model_lane_resource_scope_pg_tests).
// What was NOT proven is the DIAGNOSTIC surface specifically, which is the one
// place where "show me what went wrong" is most tempting to answer globally.
// Two disclosure shapes are covered here because they fail differently:
//
//   1. by-id     - answering "does this run exist?" for a run you do not own,
//                  which leaks existence even when the body is withheld;
//   2. by-latest - resolving "the newest run" GLOBALLY and then scoping, which
//                  leaks another account's run id through the selection itself.
//
// Both must be absent-shaped (NotFound), never a rich denial.
// ===========================================================================

#[tokio::test]
async fn mt011_diagnostics_are_owner_filtered_and_latest_is_not_a_cross_account_oracle() {
    let Some(kpg) = knowledge_pg_support::knowledge_pg().await else {
        panic!("PostgreSQL/EventLedger is required for the MT-011 diagnostics scope proof");
    };
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&kpg.schema_url)
        .await
        .expect("connect isolated diagnostics scope schema");

    use handshake_core::swarm_orchestration::model_lane::ModelLaneError;
    use handshake_core::swarm_orchestration::resource_scope::{
        ActorPrincipalId, OwnerAccountId, ResourceScope,
    };

    let alice = OwnerAccountId::mint();
    let bob = OwnerAccountId::mint();
    assert_ne!(alice, bob, "the two owning accounts must be distinct");

    let alice_store = ModelLaneStore::new_scoped(
        pool.clone(),
        ResourceScope::new(alice, ActorPrincipalId::mint()),
    );
    let bob_store = ModelLaneStore::new_scoped(
        pool.clone(),
        ResourceScope::new(bob, ActorPrincipalId::mint()),
    );

    // Alice owns a complete, diagnostics-bearing run. Bob owns nothing at all,
    // which is deliberately the WORST case for "latest": with no rows of his
    // own, any global fallback would hand Bob exactly Alice's run.
    let run_id = "run-mt011-alice-diagnostics";
    let lane_id = "lane-mt011-alice";
    alice_store
        .record_run(sample_run(run_id, lane_id))
        .await
        .expect("record alice diagnostics run");
    alice_store
        .record_lane(sample_lane(lane_id, run_id))
        .await
        .expect("record alice diagnostics lane");
    alice_store
        .record_message(sample_message("msg-mt011-alice", run_id, lane_id))
        .await
        .expect("record alice diagnostics message");
    for (tier, state, evidence) in [
        (
            ModelLaneDiagnosticTier::FlightRecorder,
            ModelLaneDiagnosticTierState::Wired,
            "eventledger://kernel/model-lane/mt011/scope",
        ),
        (
            ModelLaneDiagnosticTier::InternalDiagnostics,
            ModelLaneDiagnosticTierState::Wired,
            "hbr-int-009://dexterity/mt011-scope",
        ),
        // All three tiers, because the posture validator rejects a
        // FlightRecorder-only record outright. A partial posture would fail
        // before the scope filter under test was ever reached.
        (
            ModelLaneDiagnosticTier::Palmistry,
            ModelLaneDiagnosticTierState::DeferredWithReason,
            "palmistry://wp1/model-lane/mt011",
        ),
    ] {
        alice_store
            .record_diagnostic_tier_status(sample_tier(run_id, tier, state, evidence))
            .await
            .expect("record alice diagnostic tier");
    }
    alice_store
        .record_mt_runtime_status(sample_mt_status(run_id))
        .await
        .expect("record alice MT runtime status");

    // Positive control. Without this the two negatives below could both pass
    // against a surface that simply returns nothing to anyone.
    let alice_view = alice_store
        .diagnostics_projection_with_model_catalog(run_id, None)
        .await
        .expect("alice must be able to read her own diagnostics");
    assert_eq!(alice_view.run.run_id, run_id);
    assert!(
        !alice_view.diagnostic_tiers.is_empty(),
        "the positive control must carry real diagnostic tier rows, or it proves nothing"
    );

    // (1) By-id. Bob must get ABSENT, not a denial: a 403-shaped answer would
    // confirm the run exists and belongs to someone, which is the disclosure.
    match bob_store
        .diagnostics_projection_with_model_catalog(run_id, None)
        .await
    {
        Err(ModelLaneError::NotFound(_)) => {}
        Ok(view) => panic!(
            "cross-account diagnostics read leaked run {}",
            view.run.run_id
        ),
        Err(other) => {
            panic!("cross-account diagnostics read must be absent-shaped NotFound, got {other}")
        }
    }

    // (2) By-latest. Bob owns no runs, so "latest" must resolve INSIDE his
    // scope and find nothing - never fall back to the globally newest row.
    match bob_store
        .latest_diagnostics_projection_with_model_catalog(None)
        .await
    {
        Err(ModelLaneError::NotFound(_)) => {}
        Ok(view) => panic!(
            "latest-diagnostics leaked another account's run id: {}",
            view.run.run_id
        ),
        Err(other) => panic!("latest-diagnostics must be absent-shaped NotFound, got {other}"),
    }

    // Alice's own latest still resolves, so the filter bounds the query rather
    // than disabling the surface.
    let alice_latest = alice_store
        .latest_diagnostics_projection_with_model_catalog(None)
        .await
        .expect("alice's own latest diagnostics must still resolve");
    assert_eq!(alice_latest.run.run_id, run_id);
}

//! WP-1 MT-002: Dexterity ModelLane schema/storage proof on embedded SurrealDB.
//!
//! This is the embedded-SurrealDB/EventLedger replacement for the superseded
//! `model_lane_schema_pg_tests` suite. Every test allocates one exact
//! WP-scoped namespace/database through `surreal_test_store_support`, drives
//! the production `SurrealStorage` + `ModelLaneStore` path, and reads back
//! durable rows plus their EventLedger receipts. There is no PostgreSQL,
//! SQLite, mock, in-memory, or structs-only fallback in this proof path.

mod model_lane_cloud_support;
mod surreal_test_store_support;

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use async_trait::async_trait;
use futures::stream;
use handshake_core::kernel::{
    ArtifactRecord, ContextBundle, DummyEchoModelAdapter, KernelActor, KernelEventType,
    ModelAdapter, ModelAdapterRequest, ToolDecisionRecord,
};
use handshake_core::mcp::gate::{evaluate_kernel_tool_gate_decision, KernelMcpToolGateRequest};
use handshake_core::model_runtime::registry::RuntimeBinding as RuntimeAdapterBinding;
use handshake_core::model_runtime::{
    CancellationToken, Embedding, GenerateRequest, KvCacheHandle, KvCachePolicy, KvQuantSupport,
    LoadSpec, LoraStackHandle, ModelCapabilities, ModelId, ModelRuntime, ModelRuntimeError,
    ProviderKind, RuntimeKind, SamplingParams, Score, SteeringHookHandle, TokenStream,
};
use handshake_core::process_ledger::{
    LedgerBatcher, LedgerBatcherConfig, NoopOverflowSink, ProcessEngineKind,
    ProcessOwnershipRecordId, ProcessStart,
};
use handshake_core::storage::surreal::{
    bootstrap_schema, RowFilter, SurrealStorage, SurrealTestInspector,
};
use handshake_core::swarm_orchestration::model_lane::{
    DexterityLaunchContract, LaunchAuthority, ModelLaneAuthority, ModelLaneAuthorityTestCorruption,
    ModelLaneKind, ModelLaneLocusBinding, ModelLaneMessageKind, ModelLaneMessageRecord,
    ModelLaneProviderKind, ModelLaneRecoveryState, ModelLaneRoutingMetadata, ModelLaneStatus,
    ModelLaneStore, ModelLaneTarget, NewModelLane, NewModelLaneMessage, NewModelLaneRun,
    RuntimeBinding,
};
use handshake_core::swarm_orchestration::resource_scope::{
    AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, ExactResourceScopeAttribution,
    OwnerAccountId, ResourceAccessLifecycleRegistry, ResourceScope, WorkspaceScopeRef,
};
use handshake_core::swarm_orchestration::{
    LiveSession, ModelInstanceId, ModelSessionFactory, RecordingSwarmSink, RunBudget, SpawnRequest,
    SwarmConfig, SwarmCoordinator, SwarmError,
};
use serde_json::json;
use surreal_test_store_support::EmbeddedSurrealTestScope;

const WP_ID: &str = "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1";
const MT_ID: &str = "MT-002";
const TASK_BOARD_ID: &str = "task-board://wp-1";
/// Every canonical ModelLane EventLedger receipt carries this event id prefix
/// on the embedded substrate (the PostgreSQL-era `KE-` prefix is superseded).
const MODEL_LANE_EVENT_ID_PREFIX: &str = "evt-model-lane-";

struct Harness {
    isolated: EmbeddedSurrealTestScope,
    storage: SurrealStorage,
    scope: ResourceScope,
    lifecycle: ResourceAccessLifecycleRegistry,
    store: ModelLaneStore,
}

impl Harness {
    async fn create(label: &str) -> Self {
        let mut isolated = EmbeddedSurrealTestScope::create()
            .await
            .expect("allocate exact ModelLane embedded scope");
        let storage = isolated
            .activate_storage()
            .await
            .expect("activate production SurrealStorage");
        bootstrap_schema(&storage)
            .await
            .expect("bootstrap canonical embedded schema");
        let scope = exact_scope(label);
        let lifecycle = ResourceAccessLifecycleRegistry::new();
        register_active_context(&lifecycle, &scope);
        let store = ModelLaneStore::new_scoped_with_lifecycle(
            storage.clone(),
            scope.clone(),
            lifecycle.clone(),
        );
        Self {
            isolated,
            storage,
            scope,
            lifecycle,
            store,
        }
    }

    /// Registers `scope` as an ACTIVE authenticated context before building its store, so a
    /// cross-owner denial proves five-field ResourceScope isolation rather than an
    /// unregistered-lifecycle accident.
    fn store_for(&self, scope: ResourceScope) -> ModelLaneStore {
        register_active_context(&self.lifecycle, &scope);
        ModelLaneStore::new_scoped_with_lifecycle(self.storage.clone(), scope, self.lifecycle.clone())
    }

    /// Read-only, catalog-validated inspector over this exact namespace/database.
    fn inspector(&self) -> SurrealTestInspector {
        self.storage.test_inspector()
    }

    async fn ledger_rows_where(&self, field: &str, value: &str) -> u64 {
        let inspector = self.inspector();
        let ledger = inspector
            .table_selector("kernel_event_ledger")
            .await
            .expect("kernel_event_ledger is in the embedded catalog");
        inspector
            .row_count(
                &ledger,
                RowFilter::FieldEquals {
                    field: ledger.field(field).expect("ledger field exists"),
                    value: value.into(),
                },
            )
            .await
            .expect("count EventLedger rows")
    }

    async fn seed_cloud_authority(&self, label: &str, lane_id: &str) {
        model_lane_cloud_support::seed_cloud_lane_authority(
            &self.store,
            model_lane_cloud_support::CloudLaneAuthoritySpec {
                run_id: &run_id(label),
                lane_id,
                model_session_id: &model_session_id_for(lane_id),
                provider_kind: ModelLaneProviderKind::OpenAi.as_str(),
                requested_model_id: MODEL_ID,
                projection_plan_id: &projection_plan_ref(label),
                consent_receipt_id: &consent_receipt_ref(label),
                event_ledger_stream_id: &stream_id(label),
                work_packet_id: WP_ID,
                micro_task_id: MT_ID,
                task_board_id: TASK_BOARD_ID,
                owner_session: &owner_session(label),
            },
        )
        .await;
    }

    async fn cleanup(mut self) {
        drop(self.store);
        drop(self.storage);
        self.isolated
            .cleanup()
            .await
            .expect("clean exact ModelLane embedded scope");
    }
}

#[tokio::test]
async fn model_lane_schema_persists_and_replays_eventledger_rows() {
    let label = "schema-replay";
    let mut harness = Harness::create(label).await;
    let lane_ids = [
        default_lane_id(label),
        "lane-cloud".to_owned(),
        "lane-cli".to_owned(),
        "lane-human".to_owned(),
        "lane-subagent".to_owned(),
        "lane-validator".to_owned(),
    ];
    let run = sample_run_with_lanes(label, lane_ids.to_vec(), true);
    let message = sample_message(label);

    let stored_run = harness
        .store
        .record_run(run.clone())
        .await
        .expect("record run");
    assert!(
        stored_run
            .event_ledger_event_id
            .starts_with(MODEL_LANE_EVENT_ID_PREFIX),
        "run must carry EventLedger evidence"
    );
    assert!(stored_run.event_ledger_seq > 0);

    // Cloud lanes fail closed unless durable ProjectionPlan/ConsentReceipt
    // authority already exists (spec 4.3.9.2.5); seed it in the same exact
    // namespace/database before the cloud lane is recorded.
    harness.seed_cloud_authority(label, "lane-cloud").await;

    let mut stored_lanes = Vec::new();
    for lane in [
        sample_lane(label),
        scoped_lane(
            label,
            "lane-cloud",
            ModelLaneKind::CloudModel,
            RuntimeBinding::Cloud,
            LaunchAuthority::CloudLane,
        ),
        scoped_lane(
            label,
            "lane-cli",
            ModelLaneKind::CliModel,
            RuntimeBinding::CliBridge,
            LaunchAuthority::CliBridge,
        ),
        scoped_lane(
            label,
            "lane-human",
            ModelLaneKind::HumanOperator,
            RuntimeBinding::Human,
            LaunchAuthority::Operator,
        ),
        scoped_lane(
            label,
            "lane-subagent",
            ModelLaneKind::Subagent,
            RuntimeBinding::Subagent,
            LaunchAuthority::SubagentManager,
        ),
        scoped_lane(
            label,
            "lane-validator",
            ModelLaneKind::Validator,
            RuntimeBinding::Validator,
            LaunchAuthority::ValidatorRunner,
        ),
    ] {
        let stored = harness.store.record_lane(lane).await.expect("record lane");
        assert!(stored
            .event_ledger_event_id
            .starts_with(MODEL_LANE_EVENT_ID_PREFIX));
        assert!(stored.event_ledger_seq > stored_run.event_ledger_seq);
        stored_lanes.push(stored);
    }

    let stored_message = harness
        .store
        .record_message(message.clone())
        .await
        .expect("record message");
    assert!(stored_message
        .event_ledger_event_id
        .starts_with(MODEL_LANE_EVENT_ID_PREFIX));
    assert!(stored_message.event_ledger_seq > stored_run.event_ledger_seq);

    let replay = harness
        .store
        .replay_run(&run.run_id)
        .await
        .expect("replay exact scoped run");
    assert_eq!(replay.run.run_id, run.run_id);
    assert_eq!(replay.lanes.len(), 6);
    assert_eq!(replay.messages.len(), 1);
    assert_eq!(replay.messages[0].payload_sha256, "b".repeat(64));
    // Optional-reference round-trip through record_json is proven via a
    // reference that does not claim CRDT authority. An advisory ModelLane
    // message must replay with every CRDT authority field null; a non-null one
    // would have to dereference to real persisted Yjs bytes, never to a
    // synthetic string.
    assert_eq!(
        replay.messages[0].proposal_ref.as_deref(),
        Some(format!("proposal://mt002/{label}").as_str())
    );
    assert!(
        replay.messages[0].crdt_update_ref.is_none()
            && replay.messages[0].crdt_base_snapshot_ref.is_none()
            && replay.messages[0].crdt_state_vector.is_none()
            && replay.messages[0].crdt_proposal_ref.is_none()
            && replay.messages[0].crdt_stale_base_ref.is_none(),
        "an advisory ModelLane message must replay with null CRDT authority"
    );
    assert!(
        replay
            .lanes
            .windows(2)
            .all(|pair| pair[0].event_ledger_seq < pair[1].event_ledger_seq),
        "lane replay must be ordered by EventLedger sequence, not timestamps"
    );
    assert_eq!(
        replay.lanes.iter().map(|lane| lane.kind.clone()).collect::<Vec<_>>(),
        vec![
            ModelLaneKind::LocalModel,
            ModelLaneKind::CloudModel,
            ModelLaneKind::CliModel,
            ModelLaneKind::HumanOperator,
            ModelLaneKind::Subagent,
            ModelLaneKind::Validator,
        ]
    );
    assert_eq!(
        harness
            .store
            .record_message(message.clone())
            .await
            .expect("identical retry"),
        stored_message
    );

    // Every durable row is event-backed in the same exact five-field scope:
    // run + six lanes + message = eight canonical receipts joined to their
    // authority rows and bound to `run_id` as the EventLedger stream.
    let receipts = harness
        .store
        .test_scoped_authority_receipts(&run.run_id, 64)
        .await
        .expect("read exact scoped authority receipts");
    assert_eq!(receipts.len(), 8, "run + six lanes + message must be event-backed");
    assert_eq!(
        receipts
            .iter()
            .filter(|receipt| receipt.record_kind == "lane")
            .count(),
        6
    );
    assert_eq!(
        receipts
            .iter()
            .filter(|receipt| receipt.record_kind == "run")
            .count(),
        1
    );
    assert_eq!(
        receipts
            .iter()
            .filter(|receipt| receipt.record_kind == "message")
            .count(),
        1
    );
    let expected_scope = expected_scope_fields(&harness.scope);
    for receipt in &receipts {
        assert_eq!(receipt.event_type, "MODEL_LANE_AUTHORITY_RECORDED");
        assert_eq!(receipt.run_id, run.run_id);
        assert_eq!(
            [
                receipt.owner_account_id.as_str(),
                receipt.actor_principal_id.as_str(),
                receipt.authenticated_session_id.as_str(),
                receipt.access_space_id.as_str(),
                receipt.workspace_id.as_str(),
            ],
            [
                expected_scope[0].as_str(),
                expected_scope[1].as_str(),
                expected_scope[2].as_str(),
                expected_scope[3].as_str(),
                expected_scope[4].as_str(),
            ],
            "every durable ModelLane row carries the exact five-field ResourceScope"
        );
    }
    assert_eq!(
        harness.ledger_rows_where("session_run_id", &run.run_id).await,
        8,
        "run, lane, and message events must bind to the run's EventLedger stream"
    );
    assert_eq!(
        harness
            .ledger_rows_where("session_run_id", &coordinator_session_id(label))
            .await,
        0,
        "ModelLane EventLedger rows must not use coordinator_session_id as the stream"
    );

    let catalog = harness
        .inspector()
        .table_catalog("model_lane_authority")
        .await
        .expect("model_lane_authority catalog");
    assert!(catalog.schemafull);
    let index_names: Vec<&str> = catalog
        .indexes
        .iter()
        .map(|index| index.name.as_str())
        .collect();
    for required in [
        "model_lane_authority_aggregate",
        "model_lane_authority_run",
        "model_lane_authority_idempotency",
        "model_lane_authority_event",
        "model_lane_authority_kernel_event",
    ] {
        assert!(
            index_names.contains(&required),
            "model_lane_authority must define replay/idempotency index {required}; have {index_names:?}"
        );
    }
    for scope_field in [
        "owner_account_id",
        "actor_principal_id",
        "authenticated_session_id",
        "access_space_id",
        "workspace_id",
    ] {
        assert!(
            catalog.fields.iter().any(|field| field.name == scope_field),
            "model_lane_authority must define scope field {scope_field}"
        );
    }

    let registry = harness
        .store
        .schema_registry_rows()
        .await
        .expect("read exact scoped schema registry");
    for schema_id in [
        "hsk.model_lane_run@1",
        "hsk.model_lane@1",
        "hsk.model_lane_message@1",
        "hsk.model_lane_terminal@1",
    ] {
        assert!(
            registry.iter().any(|row| row.schema_id == schema_id),
            "missing schema registry row {schema_id}"
        );
    }

    let encoded = serde_json::to_value(&replay.messages[0]).expect("message serializes");
    let decoded: ModelLaneMessageRecord =
        serde_json::from_value(encoded).expect("message deserializes");
    assert_eq!(decoded.message_id, replay.messages[0].message_id);
    assert_eq!(decoded, replay.messages[0]);

    drop(harness.store);
    drop(harness.storage);
    harness
        .isolated
        .shutdown_storage_for_reopen()
        .await
        .expect("close storage before restart");
    harness.isolated.reopen().await.expect("reopen same scope");
    let reopened_storage = harness
        .isolated
        .activate_storage()
        .await
        .expect("reactivate same namespace/database");
    let reopened = ModelLaneStore::new_scoped_with_lifecycle(
        reopened_storage.clone(),
        harness.scope.clone(),
        harness.lifecycle.clone(),
    );
    let restarted = reopened
        .replay_run(&run.run_id)
        .await
        .expect("replay survives same-store restart");
    assert_eq!(restarted.run, stored_run);
    assert_eq!(restarted.lanes, stored_lanes);
    assert_eq!(restarted.messages, vec![stored_message]);
    harness.store = reopened;
    harness.storage = reopened_storage;
    harness.cleanup().await;
}

#[tokio::test]
async fn dexterity_launch_records_real_swarm_spawn_session_runtime_path() {
    let harness = Harness::create("dexterity-runtime").await;

    let bundle = ContextBundle::new(
        "KTR-MT003-RUNTIME",
        "SR-MT003-RUNTIME",
        json!({
            "visible_messages": [{"role": "user", "content": "prove Dexterity launch path"}],
            "tool_grants": ["read_trace"],
            "redactions": []
        }),
    )
    .expect("real ContextBundle");
    let adapter = DummyEchoModelAdapter::new("dummy-echo-mt003");
    let adapter_output = adapter
        .invoke(ModelAdapterRequest::new(
            bundle.clone(),
            KernelActor::ModelAdapter("dummy-echo-mt003".into()),
        ))
        .await
        .expect("real ModelAdapter invocation from ContextBundle");
    let tool_decision = ToolDecisionRecord::from_mcp_gate_decision(
        evaluate_kernel_tool_gate_decision(
            "dexterity-mt003-toolgate",
            ["read_trace".to_string()],
            KernelMcpToolGateRequest {
                tool_request_id: adapter_output.tool_request.tool_request_id.clone(),
                tool_id: adapter_output.tool_request.tool_id.clone(),
                reason: adapter_output.tool_request.reason.clone(),
            },
        ),
        KernelEventType::ToolDecisionRecorded,
    );
    let artifact_workspace = tempfile::tempdir().expect("ArtifactStore workspace");
    let artifact_record = ArtifactRecord::store_adapter_output(
        artifact_workspace.path(),
        "KTR-MT003-RUNTIME",
        "SR-MT003-RUNTIME",
        &adapter_output,
    )
    .expect("model output lands in ArtifactStore");

    let (ledger, _drain) = LedgerBatcher::manual_for_tests(
        LedgerBatcherConfig {
            capacity: 128,
            ..LedgerBatcherConfig::default()
        },
        Arc::new(NoopOverflowSink),
    )
    .expect("manual process ledger");
    let unloads = Arc::new(AtomicUsize::new(0));
    let factory = Arc::new(DexterityProofFactory {
        ledger: ledger.clone(),
        unloads: unloads.clone(),
    });
    let sink = Arc::new(RecordingSwarmSink::new());
    let coordinator = SwarmCoordinator::new_with_model_lane_store(
        SwarmConfig::new(RunBudget::defaulted(1)),
        factory,
        sink,
        ledger,
        harness.store.clone(),
    );
    let instance_id = ModelInstanceId::new(ModelId::new_v7(), 42);
    let request = SpawnRequest::new(
        instance_id,
        RuntimeAdapterBinding::LlamaCpp,
        "KERNEL_BUILDER-MT003",
        "coordinator-session-mt003",
    )
    .with_wp(WP_ID)
    .with_mt("MT-003")
    .with_dexterity_launch(DexterityLaunchContract {
        run_id: "run-runtime-001".into(),
        lane_id: "lane-runtime-local".into(),
        restart_generation: 0,
        trace_id: "trace-mt003-runtime".into(),
        run_span_id: "span-runtime-run".into(),
        lane_span_id: "span-runtime-lane".into(),
        routing_policy: "single_local_runtime_spawn".into(),
        context_bundle_id: bundle.context_bundle_id.clone(),
        event_ledger_stream_id: "mlane-stream-runtime-001".into(),
        artifact_namespace: artifact_record.artifact_payload_ref.clone(),
        task_board_id: TASK_BOARD_ID.into(),
        locus_binding_ref: "locus://wp1/mt003/runtime-spawn".into(),
        role: "runtime-local-lane".into(),
        backend: "llama_cpp".into(),
        adapter_id: adapter.adapter_id().into(),
        capability_token_ids: vec!["capability://mt003/read-trace".into()],
        effective_capability_snapshot_ref: "capability-snapshot://mt003/runtime".into(),
        projection_plan_ref: None,
        consent_receipt_ref: None,
        tool_gate_decision_refs: vec![format!("toolgate://{}", tool_decision.tool_decision_id)],
        memory_pack_ref: "memory-pack://fems/mt003/runtime".into(),
        memory_pack_hash: "a".repeat(64),
        determinism_mode: "deterministic_replay".into(),
        budget_summary_ref: "budget://mt003/runtime".into(),
        candidate_model_ids: vec!["model://mt003/runtime-local".into()],
        procedural_review_status: "toolgate_context_artifact_boundaries_exercised".into(),
        truncation_warning_ref: None,
        rejection_reason_refs: vec![],
        run_recovery_hint_ref: Some("usermanual://model-lane-schema#runtime-launch".into()),
        lane_recovery_hint_ref: Some("usermanual://model-lane-schema#lane-recovery".into()),
    });

    let spawned = coordinator
        .spawn_session(request)
        .await
        .expect("Dexterity-recorded spawn_session succeeds");
    assert_eq!(spawned, instance_id);
    assert_eq!(
        unloads.load(Ordering::SeqCst),
        0,
        "successful spawn remains live until drained"
    );

    let replay = harness
        .store
        .replay_run("run-runtime-001")
        .await
        .expect("Dexterity replay from real spawn");
    assert_eq!(replay.run.context_bundle_id, bundle.context_bundle_id);
    assert_eq!(
        replay.run.artifact_namespace,
        artifact_record.artifact_payload_ref
    );
    assert_eq!(replay.run.memory_pack_hash, "a".repeat(64));
    assert_eq!(replay.lanes.len(), 1);
    assert_eq!(replay.lanes[0].lane_id, "lane-runtime-local");
    assert_eq!(replay.lanes[0].runtime_binding, RuntimeBinding::Local);
    assert!(replay.lanes[0]
        .last_runtime_status_ref
        .as_deref()
        .expect("process ledger ref")
        .starts_with("process-ledger://"));
    assert_eq!(
        replay.lanes[0].tool_gate_decision_refs,
        vec![format!("toolgate://{}", tool_decision.tool_decision_id)]
    );
    let receipts = harness
        .store
        .test_scoped_authority_receipts("run-runtime-001", 16)
        .await
        .expect("runtime launch receipts");
    assert_eq!(
        receipts
            .iter()
            .map(|receipt| receipt.record_kind.as_str())
            .collect::<Vec<_>>(),
        vec!["run", "lane"],
        "runtime run and lane events must bind to the run's EventLedger stream"
    );
    assert_eq!(
        harness
            .ledger_rows_where("session_run_id", "coordinator-session-mt003")
            .await,
        0,
        "Dexterity EventLedger rows must not use coordinator_session_id as the stream"
    );

    coordinator
        .drain_all()
        .await
        .expect("drain Dexterity proof session");
    assert_eq!(
        unloads.load(Ordering::SeqCst),
        1,
        "runtime proof factory must unload the real LiveSession"
    );
    harness.cleanup().await;
}

#[tokio::test]
async fn model_lane_schema_serializes_competing_terminal_updates() {
    let label = "terminal-race";
    let harness = Harness::create(label).await;
    let run = sample_run(label);
    let lane = sample_lane(label);
    harness
        .store
        .record_prepared_launch((run.clone(), lane.clone()))
        .await
        .expect("record terminal race launch");

    let cancel_store = harness.store.clone();
    let fail_store = harness.store.clone();
    let cancel_lane = lane.lane_id.clone();
    let fail_lane = lane.lane_id.clone();
    let (cancelled, failed) = tokio::join!(
        async move {
            cancel_store
                .record_lane_terminal_status(
                    &cancel_lane,
                    ModelLaneStatus::Cancelled,
                    "operator_cancelled_terminal_race",
                )
                .await
        },
        async move {
            fail_store
                .record_lane_terminal_status(
                    &fail_lane,
                    ModelLaneStatus::Failed,
                    "runtime_failed_terminal_race",
                )
                .await
        }
    );

    let mut terminal_statuses = Vec::new();
    let mut conflict_count = 0usize;
    for result in [cancelled, failed] {
        match result {
            Ok(record) => terminal_statuses.push(record.status.clone()),
            Err(err) => {
                let text = err.to_string();
                assert!(
                    text.contains("already terminal")
                        || text.contains("changed while terminal status was committing"),
                    "expected terminal idempotency/compare-and-set conflict, got {err}"
                );
                conflict_count += 1;
            }
        }
    }
    assert_eq!(
        terminal_statuses.len(),
        1,
        "exactly one competing terminal update may win"
    );
    assert_eq!(
        conflict_count, 1,
        "exactly one competing terminal update must fail closed"
    );
    let winning_status = terminal_statuses[0].clone();
    assert!(
        winning_status == ModelLaneStatus::Cancelled || winning_status == ModelLaneStatus::Failed,
        "winning terminal status must be one submitted by the race"
    );

    let replay = harness
        .store
        .replay_run(&run.run_id)
        .await
        .expect("replay terminal race run");
    assert_eq!(replay.lanes.len(), 1);
    assert_eq!(replay.lanes[0].status, winning_status);
    if replay.lanes[0].status == ModelLaneStatus::Failed {
        assert_eq!(
            replay.lanes[0].startup_failure_ref.as_deref(),
            Some(format!("terminal-failure://dexterity/{}", lane.lane_id).as_str())
        );
    }
    assert_eq!(
        harness.ledger_rows_where("aggregate_id", &lane.lane_id).await,
        2,
        "competing terminal updates must append exactly one terminal EventLedger row after the lane's creation row"
    );
    let lane_receipt = harness
        .store
        .test_scoped_authority_receipts(&run.run_id, 16)
        .await
        .expect("terminal receipts")
        .into_iter()
        .find(|receipt| receipt.record_kind == "lane")
        .expect("lane receipt");
    assert_eq!(lane_receipt.event_type, "MODEL_LANE_AUTHORITY_REPLACED");
    assert_eq!(
        lane_receipt.event_id,
        replay.lanes[0].event_ledger_event_id,
        "the winning terminal receipt is the lane's canonical EventLedger event"
    );

    let failed_label = "terminal-failed";
    let failed_lane_id = "lane-terminal-failed";
    let failed_run = sample_run_with_lanes(failed_label, vec![failed_lane_id.to_owned()], true);
    let failed_lane = scoped_lane(
        failed_label,
        failed_lane_id,
        ModelLaneKind::CloudModel,
        RuntimeBinding::Cloud,
        LaunchAuthority::CloudLane,
    );
    harness
        .seed_cloud_authority(failed_label, failed_lane_id)
        .await;
    harness
        .store
        .record_prepared_launch((failed_run.clone(), failed_lane))
        .await
        .expect("record deterministic failed terminal launch");
    let failed_terminal = harness
        .store
        .record_lane_terminal_status(
            failed_lane_id,
            ModelLaneStatus::Failed,
            "runtime_failed_terminal_ref",
        )
        .await
        .expect("record deterministic failed terminal status");
    assert_eq!(failed_terminal.status, ModelLaneStatus::Failed);
    assert_eq!(failed_terminal.failstate_code.as_deref(), Some("failed"));
    assert_eq!(
        failed_terminal.startup_failure_ref.as_deref(),
        Some("terminal-failure://dexterity/lane-terminal-failed")
    );
    assert_eq!(
        harness.ledger_rows_where("aggregate_id", failed_lane_id).await,
        2,
        "failed terminal update must append exactly one failed terminal EventLedger row"
    );
    let failed_replay = harness
        .store
        .replay_run(&failed_run.run_id)
        .await
        .expect("replay failed terminal run");
    assert_eq!(failed_replay.lanes.len(), 1);
    assert_eq!(failed_replay.lanes[0], failed_terminal);
    harness.cleanup().await;
}

#[tokio::test]
async fn model_lane_schema_rejects_missing_locus_binding_and_idempotency_conflict() {
    let label = "schema-denials";
    let harness = Harness::create(label).await;
    let store = harness.store.clone();

    let mut missing_locus = sample_run("missing-locus");
    missing_locus.locus_binding = None;
    let err = store
        .record_run(missing_locus)
        .await
        .expect_err("missing Locus binding must fail closed before persistence");
    assert!(
        err.to_string().contains("locus"),
        "expected Locus validation error, got {err}"
    );

    let mut mismatched_locus = sample_run("mismatched-locus");
    mismatched_locus
        .locus_binding
        .as_mut()
        .expect("sample has locus")
        .work_packet_id = "WP-WRONG".into();
    let err = store
        .record_run(mismatched_locus)
        .await
        .expect_err("mismatched Locus WP must fail closed");
    assert!(
        err.to_string().contains("work_packet_id"),
        "expected Locus WP validation error, got {err}"
    );

    let err = store
        .record_prepared_launch((
            sample_run("prepared-mismatch-run"),
            sample_lane("prepared-mismatch-lane"),
        ))
        .await
        .expect_err("prepared launch must reject mismatched run/lane ids");
    assert!(
        err.to_string().contains("run_id"),
        "expected prepared pair run_id validation error, got {err}"
    );

    let run = sample_run_with_lanes(
        label,
        vec![default_lane_id(label), "lane-cloud".to_owned()],
        true,
    );
    store
        .record_run(run.clone())
        .await
        .expect("record canonical run");
    let mut divergent_run_retry = run.clone();
    divergent_run_retry.idempotency_key = "idem-run-divergent".into();
    let err = store
        .record_run(divergent_run_retry)
        .await
        .expect_err("same run_id with divergent idempotency must fail closed");
    assert!(
        err.to_string().contains("idempotency"),
        "expected run idempotency conflict, got {err}"
    );
    store
        .record_lane(sample_lane(label))
        .await
        .expect("record source lane");
    harness.seed_cloud_authority(label, "lane-cloud").await;
    store
        .record_lane(scoped_lane(
            label,
            "lane-cloud",
            ModelLaneKind::CloudModel,
            RuntimeBinding::Cloud,
            LaunchAuthority::CloudLane,
        ))
        .await
        .expect("record cloud lane");

    let mut unsupported_provider = scoped_lane(
        label,
        "lane-provider-other",
        ModelLaneKind::LocalModel,
        RuntimeBinding::Local,
        LaunchAuthority::ModelRuntime,
    );
    unsupported_provider.provider_kind = ModelLaneProviderKind::Other;
    let err = store
        .record_lane(unsupported_provider)
        .await
        .expect_err("unsupported provider must fail closed");
    assert!(
        err.to_string().contains("provider_kind"),
        "expected provider validation error, got {err}"
    );

    let mut missing_capability_snapshot = scoped_lane(
        label,
        "lane-missing-capability",
        ModelLaneKind::LocalModel,
        RuntimeBinding::Local,
        LaunchAuthority::ModelRuntime,
    );
    missing_capability_snapshot.effective_capability_snapshot_ref = None;
    let err = store
        .record_lane(missing_capability_snapshot)
        .await
        .expect_err("missing capability snapshot must fail closed");
    assert!(
        err.to_string()
            .contains("effective_capability_snapshot_ref"),
        "expected capability validation error, got {err}"
    );

    let mut missing_hash = sample_message_with(label, "msg-missing-hash");
    missing_hash.payload_sha256.clear();
    let err = store
        .record_message(missing_hash)
        .await
        .expect_err("missing payload hash must fail closed");
    assert!(
        err.to_string().contains("payload_sha256"),
        "expected payload hash validation error, got {err}"
    );

    let mut missing_crdt = crdt_posture_message(label, "msg-missing-crdt");
    missing_crdt.crdt_base_snapshot_ref = None;
    let err = store
        .record_message(missing_crdt)
        .await
        .expect_err("proposal missing CRDT base snapshot must fail closed");
    assert!(
        err.to_string().contains("crdt_base_snapshot_ref"),
        "expected CRDT validation error, got {err}"
    );

    let mut non_proposal_with_partial_crdt = crdt_posture_message(label, "msg-partial-crdt-status");
    non_proposal_with_partial_crdt.kind = ModelLaneMessageKind::Status;
    non_proposal_with_partial_crdt.crdt_base_snapshot_ref = None;
    let err = store
        .record_message(non_proposal_with_partial_crdt)
        .await
        .expect_err("every CRDT-targeting message kind must fail closed on partial metadata");
    assert!(
        err.to_string().contains("crdt_base_snapshot_ref"),
        "expected kind-independent CRDT validation error, got {err}"
    );

    // Partial-CRDT admission is refused by two independent fail-closed layers
    // (the synchronous completeness check and the durable authority
    // resolution); both name the missing `crdt_update_ref`, so the assertion
    // is pinned to that shared invariant rather than one layer's wording.
    let mut proposal_ref_only = crdt_posture_message(label, "msg-crdt-proposal-ref-only");
    proposal_ref_only.crdt_update_ref = None;
    proposal_ref_only.crdt_base_snapshot_ref = None;
    proposal_ref_only.crdt_state_vector = None;
    proposal_ref_only.crdt_stale_base_ref = None;
    let err = store
        .record_message(proposal_ref_only)
        .await
        .expect_err("proposal-only CRDT metadata must not bypass authority resolution");
    assert!(
        err.to_string().contains("crdt_update_ref"),
        "expected proposal-only CRDT validation error naming crdt_update_ref, got {err}"
    );

    let mut stale_ref_only = crdt_posture_message(label, "msg-crdt-stale-ref-only");
    stale_ref_only.crdt_update_ref = None;
    stale_ref_only.crdt_base_snapshot_ref = None;
    stale_ref_only.crdt_state_vector = None;
    stale_ref_only.crdt_proposal_ref = None;
    stale_ref_only.crdt_stale_base_ref = Some("crdt-stale-base://mt002/stale-only".into());
    let err = store
        .record_message(stale_ref_only)
        .await
        .expect_err("stale-only CRDT metadata must not bypass authority resolution");
    assert!(
        err.to_string().contains("crdt_update_ref"),
        "expected stale-only CRDT validation error naming crdt_update_ref, got {err}"
    );

    let mut advisory_proposal =
        sample_message_with(label, "msg-advisory-proposal-without-crdt");
    advisory_proposal.proposal_ref = None;
    advisory_proposal.authority = ModelLaneAuthority::Advisory;
    advisory_proposal.idempotency_key = "idem-message-mt002-advisory-without-crdt".into();
    store
        .record_message(advisory_proposal)
        .await
        .expect("ordinary advisory Proposal without CRDT posture is valid");

    let mut malformed_trace = sample_message_with(label, "msg-malformed-trace");
    malformed_trace.parent_span_id = Some(malformed_trace.message_span_id.clone());
    let err = store
        .record_message(malformed_trace)
        .await
        .expect_err("malformed trace linkage must fail closed");
    assert!(
        err.to_string().contains("parent_span_id"),
        "expected span validation error, got {err}"
    );

    let first = sample_message(label);
    let left_store = store.clone();
    let right_store = store.clone();
    let (first_left, first_right) = (first.clone(), first);
    let (left, right) = tokio::join!(
        async move { left_store.record_message(first_left).await },
        async move { right_store.record_message(first_right).await }
    );
    let left = left.expect("left concurrent message insert");
    let right = right.expect("right concurrent idempotent replay");
    assert_eq!(left.message_id, message_id(label));
    assert_eq!(right.message_id, message_id(label));
    assert_eq!(left.event_ledger_event_id, right.event_ledger_event_id);
    let message_receipts = store
        .test_scoped_authority_receipts(&run.run_id, 32)
        .await
        .expect("message receipts after concurrent retry")
        .into_iter()
        .filter(|receipt| receipt.record_kind == "message")
        .count();
    assert_eq!(
        message_receipts, 2,
        "the advisory proposal plus one concurrent same-key insert must produce exactly two EventLedger rows"
    );

    let canonical_key = sample_message(label).idempotency_key;
    let mut retry = sample_message_with(label, "msg-retry");
    retry.idempotency_key = canonical_key.clone();
    let replayed = store
        .record_message(retry)
        .await
        .expect("same idempotency key + same payload is idempotent");
    assert_eq!(replayed.message_id, message_id(label));

    let mut conflicting = sample_message_with(label, "msg-conflict");
    conflicting.idempotency_key = canonical_key;
    conflicting.payload_sha256 = "f".repeat(64);
    let err = store
        .record_message(conflicting)
        .await
        .expect_err("same idempotency key + different payload must fail closed");
    assert!(
        err.to_string().contains("idempotency"),
        "expected idempotency conflict, got {err}"
    );

    let incomplete = ResourceScope::new(OwnerAccountId::mint(), ActorPrincipalId::mint());
    let denied = ModelLaneStore::new_scoped(harness.storage.clone(), incomplete)
        .record_run(sample_run("incomplete-scope"))
        .await
        .expect_err("incomplete five-field scope must fail closed");
    assert!(denied.to_string().contains("exact owner"));
    drop(store);
    harness.cleanup().await;
}

#[tokio::test]
async fn model_lane_schema_isolates_all_five_scope_fields_with_equal_logical_ids() {
    let harness = Harness::create("scope-owner").await;
    let run = sample_run("scope-collision");
    let owner_record = harness
        .store
        .record_run(run.clone())
        .await
        .expect("owner stores run");

    for foreign in one_field_mismatches(&harness.scope) {
        let foreign_store = harness.store_for(foreign);
        assert!(foreign_store.replay_run(&run.run_id).await.is_err());
        let foreign_record = foreign_store
            .record_run(run.clone())
            .await
            .expect("same logical id is independent in another exact scope");
        assert_ne!(
            foreign_record.event_ledger_event_id,
            owner_record.event_ledger_event_id
        );
    }
    assert_eq!(
        harness
            .store
            .replay_run(&run.run_id)
            .await
            .expect("owner remains available")
            .run,
        owner_record
    );
    harness.cleanup().await;
}

#[tokio::test]
async fn model_lane_schema_denies_incomplete_attribution_rows_without_grandfathering() {
    let label = "legacy-attribution";
    let harness = Harness::create(label).await;
    let run = sample_run(label);
    let lane = sample_lane(label);
    harness
        .store
        .record_prepared_launch((run.clone(), lane.clone()))
        .await
        .expect("owner records launch pair");
    let before = harness
        .store
        .replay_run(&run.run_id)
        .await
        .expect("owner replay before fixture");

    // The SCHEMAFULL authority table refuses a blank attribution field outright:
    // the fixture seam must fail and leave the launch pair readable unchanged.
    let blank = harness
        .store
        .test_corrupt_scoped_authority(
            "lane",
            &lane.lane_id,
            ModelLaneAuthorityTestCorruption::BlankAttribution,
        )
        .await
        .expect_err("embedded schema must refuse a blank scope field");
    assert!(
        !blank.to_string().contains(&harness_scope_owner(&harness)),
        "blank-attribution denial must not echo the owner account id"
    );
    assert_eq!(
        harness
            .store
            .replay_run(&run.run_id)
            .await
            .expect("blank-attribution refusal is non-mutating"),
        before
    );

    // Seed exactly one deliberately incomplete-attribution row in the same
    // namespace/database: the owner's run row loses its authenticated session
    // and AccessSpace attribution, structurally like a row that predates
    // five-field attribution.
    harness
        .store
        .test_corrupt_scoped_authority(
            "run",
            &run.run_id,
            ModelLaneAuthorityTestCorruption::IncompleteAttribution,
        )
        .await
        .expect("seed one incomplete-attribution run row");

    // The fixture must actually be incomplete, or this proves nothing.
    let inspector = harness.inspector();
    let authority = inspector
        .table_selector("model_lane_authority")
        .await
        .expect("model_lane_authority selector");
    let fields = [
        "owner_account_id",
        "actor_principal_id",
        "authenticated_session_id",
        "access_space_id",
        "workspace_id",
    ]
    .iter()
    .map(|field| authority.field(field).expect("scope field selector"))
    .collect::<Vec<_>>();
    let rows = inspector
        .project(
            &authority,
            &fields,
            RowFilter::FieldEquals {
                field: authority.field("aggregate_id").expect("aggregate_id"),
                value: run.run_id.as_str().into(),
            },
        )
        .await
        .expect("project the seeded run row");
    assert_eq!(rows.len(), 1, "exactly one run row is seeded");
    let expected = expected_scope_fields(&harness.scope);
    assert_eq!(rows[0].values["owner_account_id"], json!(expected[0]));
    assert_eq!(rows[0].values["actor_principal_id"], json!(expected[1]));
    assert_eq!(
        rows[0].values["authenticated_session_id"],
        json!("legacy-unattributed")
    );
    assert_eq!(rows[0].values["access_space_id"], json!("legacy-unattributed"));
    assert_eq!(rows[0].values["workspace_id"], json!(expected[4]));

    // LAYER 1: the owner scope cannot see it, and the denial leaks nothing.
    let denied = harness
        .store
        .replay_run(&run.run_id)
        .await
        .expect_err("owner replaying an incomplete-attribution run is denied");
    for identifier in &expected {
        assert!(
            !denied.to_string().contains(identifier.as_str()),
            "denial must not leak scope identifier {identifier}"
        );
    }
    assert!(!denied.to_string().contains("legacy-unattributed"));
    assert!(
        harness.store.navigation_by_run(&run.run_id).await.is_err(),
        "owner navigating an incomplete-attribution run is denied"
    );
    let receipts = harness
        .store
        .test_scoped_authority_receipts(&run.run_id, 16)
        .await
        .expect("scoped receipts exclude the incomplete row");
    assert_eq!(
        receipts
            .iter()
            .map(|receipt| receipt.record_kind.as_str())
            .collect::<Vec<_>>(),
        vec!["lane"],
        "the canonical receipt join must drop the incomplete run row and keep the intact lane row"
    );

    // LAYER 2: no other exact scope can claim it either; a partial match is not
    // grandfathered into any account.
    for foreign in one_field_mismatches(&harness.scope) {
        let foreign_store = harness.store_for(foreign);
        assert!(foreign_store.replay_run(&run.run_id).await.is_err());
        assert!(foreign_store
            .test_scoped_authority_receipts(&run.run_id, 16)
            .await
            .expect("foreign receipt scan is non-leaking")
            .is_empty());
    }
    harness.cleanup().await;
}

#[tokio::test]
async fn model_lane_launch_path_persists_one_atomic_run_lane_pair() {
    let harness = Harness::create("prepared-launch").await;
    let run = sample_run("prepared-launch");
    let lane = sample_lane("prepared-launch");
    let (stored_run, stored_lane) = harness
        .store
        .record_prepared_launch((run.clone(), lane.clone()))
        .await
        .expect("production prepared-launch path persists the pair");
    assert_eq!(stored_run.run_id, run.run_id);
    assert_eq!(stored_lane.lane_id, lane.lane_id);
    assert_eq!(stored_lane.run_id, stored_run.run_id);
    assert!(stored_lane.event_ledger_seq > stored_run.event_ledger_seq);

    let replay = harness
        .store
        .replay_run(&run.run_id)
        .await
        .expect("prepared launch replays from canonical authority");
    assert_eq!(replay.run, stored_run);
    assert_eq!(replay.lanes, vec![stored_lane]);
    harness.cleanup().await;
}

#[tokio::test]
async fn competing_terminal_updates_use_versioned_cas_and_failure_is_non_mutating() {
    let harness = Harness::create("terminal-cas").await;
    let run = sample_run("terminal-cas");
    let lane = sample_lane("terminal-cas");
    harness
        .store
        .record_prepared_launch((run.clone(), lane.clone()))
        .await
        .expect("seed ready launch");

    let control = harness.store.test_terminal_commit_control();
    control.fail_next();
    let injected = harness
        .store
        .record_lane_terminal_status(&lane.lane_id, ModelLaneStatus::Failed, "injected")
        .await
        .expect_err("pre-commit failure must propagate");
    assert!(injected.to_string().contains("before durable mutation"));
    assert_eq!(
        harness
            .store
            .replay_run(&run.run_id)
            .await
            .expect("failure leaves launch readable")
            .lanes[0]
            .status,
        ModelLaneStatus::Ready
    );

    control.pause_next();
    let paused_store = harness.store.clone();
    let paused_lane_id = lane.lane_id.clone();
    let paused = tokio::spawn(async move {
        paused_store
            .record_lane_terminal_status(
                &paused_lane_id,
                ModelLaneStatus::Completed,
                "completed concurrently",
            )
            .await
    });
    control.wait_until_paused().await;

    let winner = harness
        .store
        .record_lane_terminal_status(
            &lane.lane_id,
            ModelLaneStatus::Cancelled,
            "cancelled concurrently",
        )
        .await
        .expect("one terminal writer wins");
    control.release_paused();
    let loser = paused
        .await
        .expect("paused writer task joins")
        .expect_err("stale terminal writer loses its compare-and-set");
    assert!(loser
        .to_string()
        .contains("changed while terminal status was committing"));
    assert_eq!(winner.status, ModelLaneStatus::Cancelled);

    let replay = harness
        .store
        .replay_run(&run.run_id)
        .await
        .expect("terminal winner is canonical");
    assert_eq!(replay.lanes.len(), 1);
    assert_eq!(replay.lanes[0].status, ModelLaneStatus::Cancelled);
    assert_eq!(
        replay.lanes[0].event_ledger_event_id,
        winner.event_ledger_event_id
    );
    harness.cleanup().await;
}

const MODEL_ID: &str = "model://local/mt002";

fn run_id(label: &str) -> String {
    format!("run-mt002-{label}")
}

fn default_lane_id(label: &str) -> String {
    format!("lane-mt002-{label}")
}

fn message_id(label: &str) -> String {
    format!("message-mt002-{label}")
}

fn stream_id(label: &str) -> String {
    format!("model-lane://mt002/{label}")
}

fn coordinator_session_id(label: &str) -> String {
    format!("coordinator-mt002-{label}")
}

fn owner_session(label: &str) -> String {
    format!("owner-mt002-{label}")
}

fn projection_plan_ref(label: &str) -> String {
    format!("projection-plan://mt002/{label}")
}

fn consent_receipt_ref(label: &str) -> String {
    format!("consent://mt002/{label}")
}

fn session_id_for(lane_id: &str) -> String {
    format!("session-{lane_id}")
}

fn model_session_id_for(lane_id: &str) -> String {
    format!("model-session-{lane_id}")
}

/// Registers the exact five-field attribution of `scope` as an ACTIVE authenticated
/// resource-access context. Production composes this registry from the authentication/session
/// authority; ModelLaneStore::new_scoped is intentionally fail-closed without it.
fn register_active_context(lifecycle: &ResourceAccessLifecycleRegistry, scope: &ResourceScope) {
    let exact = ExactResourceScopeAttribution::try_from_resource_scope(scope)
        .expect("proof scope must carry all five exact attribution fields");
    lifecycle
        .register_active(exact)
        .expect("register active authenticated resource-access context");
}

fn exact_scope(label: &str) -> ResourceScope {
    ResourceScope::new(OwnerAccountId::mint(), ActorPrincipalId::mint())
        .with_session(AuthenticatedSessionRef::mint())
        .with_access_space(AccessSpaceRef::mint())
        .with_workspace(
            WorkspaceScopeRef::new(format!("workspace-mt002-{label}")).expect("nonblank workspace"),
        )
}

/// The five stored scope strings for `scope`, in canonical column order.
fn expected_scope_fields(scope: &ResourceScope) -> [String; 5] {
    [
        scope.owner_account_id.as_uuid().to_string(),
        scope.actor_principal_id.as_uuid().to_string(),
        scope
            .authenticated_session
            .expect("exact session")
            .as_uuid()
            .to_string(),
        scope
            .access_space
            .expect("exact access space")
            .as_uuid()
            .to_string(),
        scope
            .workspace
            .clone()
            .expect("exact workspace")
            .as_str()
            .to_owned(),
    ]
}

fn harness_scope_owner(harness: &Harness) -> String {
    harness.scope.owner_account_id.as_uuid().to_string()
}

fn one_field_mismatches(scope: &ResourceScope) -> Vec<ResourceScope> {
    let workspace = scope.workspace.clone().expect("exact workspace");
    vec![
        ResourceScope::new(OwnerAccountId::mint(), scope.actor_principal_id)
            .with_session(scope.authenticated_session.expect("exact session"))
            .with_access_space(scope.access_space.expect("exact access space"))
            .with_workspace(workspace.clone()),
        ResourceScope::new(scope.owner_account_id, ActorPrincipalId::mint())
            .with_session(scope.authenticated_session.expect("exact session"))
            .with_access_space(scope.access_space.expect("exact access space"))
            .with_workspace(workspace.clone()),
        ResourceScope::new(scope.owner_account_id, scope.actor_principal_id)
            .with_session(AuthenticatedSessionRef::mint())
            .with_access_space(scope.access_space.expect("exact access space"))
            .with_workspace(workspace.clone()),
        ResourceScope::new(scope.owner_account_id, scope.actor_principal_id)
            .with_session(scope.authenticated_session.expect("exact session"))
            .with_access_space(AccessSpaceRef::mint())
            .with_workspace(workspace),
        ResourceScope::new(scope.owner_account_id, scope.actor_principal_id)
            .with_session(scope.authenticated_session.expect("exact session"))
            .with_access_space(scope.access_space.expect("exact access space"))
            .with_workspace(
                WorkspaceScopeRef::new("workspace-mt002-foreign").expect("nonblank workspace"),
            ),
    ]
}

fn sample_run(label: &str) -> NewModelLaneRun {
    sample_run_with_lanes(label, vec![default_lane_id(label)], false)
}

fn sample_run_with_lanes(label: &str, lane_ids: Vec<String>, cloud: bool) -> NewModelLaneRun {
    NewModelLaneRun {
        run_id: run_id(label),
        trace_id: format!("trace-mt002-{label}"),
        run_span_id: format!("span-run-mt002-{label}"),
        coordinator_session_id: coordinator_session_id(label),
        routing_policy: if cloud {
            "mixed_local_cloud_subagent".into()
        } else {
            "local_first".into()
        },
        context_bundle_id: format!("context-mt002-{label}"),
        lane_ids,
        event_ledger_stream_id: stream_id(label),
        artifact_namespace: format!("artifact://mt002/{label}"),
        projection_plan_ref: cloud.then(|| projection_plan_ref(label)),
        consent_receipt_ref: cloud.then(|| consent_receipt_ref(label)),
        work_packet_id: Some(WP_ID.into()),
        micro_task_id: Some(MT_ID.into()),
        task_board_id: Some(TASK_BOARD_ID.into()),
        owner_session: owner_session(label),
        idempotency_key: format!("mt002-run-{label}"),
        replay_order_key: format!("0001-{label}"),
        replay_after_event_ledger_seq: None,
        recovery_state: ModelLaneRecoveryState::Restartable,
        failstate_code: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane/recovery".into()),
        locus_binding: Some(sample_locus(label)),
        memory_pack_ref: format!("memory-pack://mt002/{label}"),
        memory_pack_hash: "a".repeat(64),
        determinism_mode: "strict".into(),
        budget_summary_ref: format!("budget://mt002/{label}"),
        selected_model_id: Some(MODEL_ID.into()),
        candidate_model_ids: vec![MODEL_ID.into()],
        procedural_review_status: "approved".into(),
        truncation_warning_ref: None,
        rejection_reason_refs: Vec::new(),
    }
}

fn sample_lane(label: &str) -> NewModelLane {
    scoped_lane(
        label,
        &default_lane_id(label),
        ModelLaneKind::LocalModel,
        RuntimeBinding::Local,
        LaunchAuthority::ModelRuntime,
    )
}

fn scoped_lane(
    label: &str,
    lane_id: &str,
    kind: ModelLaneKind,
    runtime_binding: RuntimeBinding,
    launch_authority: LaunchAuthority,
) -> NewModelLane {
    let provider_kind = provider_kind_for(&runtime_binding);
    let cloud = runtime_binding == RuntimeBinding::Cloud;
    let process_backed = matches!(
        runtime_binding,
        RuntimeBinding::Local | RuntimeBinding::Cloud | RuntimeBinding::CliBridge
    );
    let session_id = session_id_for(lane_id);
    let model_session_id = model_session_id_for(lane_id);
    NewModelLane {
        lane_id: lane_id.into(),
        run_id: run_id(label),
        trace_id: format!("trace-mt002-{label}"),
        lane_span_id: format!("span-{lane_id}-{label}"),
        event_ledger_stream_id: stream_id(label),
        kind,
        role: format!("role-{lane_id}"),
        backend: format!("{runtime_binding:?}").to_ascii_lowercase(),
        model_id: Some(MODEL_ID.into()),
        session_id: session_id.clone(),
        model_session_id: model_session_id.clone(),
        adapter_id: format!("adapter-{lane_id}"),
        runtime_binding: runtime_binding.clone(),
        launch_authority: launch_authority.clone(),
        provider_kind: provider_kind.clone(),
        capability_token_ids: vec!["capability://mt002/read".into()],
        effective_capability_snapshot_ref: Some("capability://mt002/snapshot".into()),
        capability_negotiation_ref: Some(format!("capability://mt002/negotiation/{lane_id}")),
        provider_feature_profile_ref: Some(format!(
            "provider-feature-profile://{}",
            provider_kind.as_str()
        )),
        requested_execution_policy_ref: Some(format!(
            "execution-policy://requested/{}",
            runtime_binding.as_str()
        )),
        effective_execution_policy_ref: Some(format!(
            "execution-policy://effective/{}",
            launch_authority.as_str()
        )),
        projection_plan_ref: cloud.then(|| projection_plan_ref(label)),
        consent_receipt_ref: cloud.then(|| consent_receipt_ref(label)),
        tool_gate_decision_refs: vec!["tool-gate://mt002/read".into()],
        status: ModelLaneStatus::Ready,
        recovery_state: ModelLaneRecoveryState::Restartable,
        heartbeat_at_utc: Some("2026-09-02T00:00:00Z".into()),
        lease_expires_at_utc: Some("2099-09-02T00:00:00Z".into()),
        reclaim_after_utc: Some("2099-09-02T00:01:00Z".into()),
        restart_generation: 0,
        cancellation_ref: Some(format!("cancel://mt002/{lane_id}")),
        reclaim_policy_ref: Some("reclaim://mt002".into()),
        terminal_status_mapping_ref: Some(format!(
            "terminal-status://session-broker/{}",
            runtime_binding.as_str()
        )),
        process_ownership_ref: process_backed.then(|| format!("process://mt002/{lane_id}")),
        no_os_process_reason_ref: (!process_backed)
            .then(|| format!("no-os-process://mt002/{lane_id}")),
        backpressure_ref: None,
        loop_counter_ref: Some("loop://mt002".into()),
        last_runtime_status_ref: Some("runtime://mt002/ready".into()),
        last_recovery_event_ref: None,
        failstate_code: None,
        startup_failure_ref: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane/recovery".into()),
        work_packet_id: Some(WP_ID.into()),
        micro_task_id: Some(MT_ID.into()),
        task_board_id: Some(TASK_BOARD_ID.into()),
        owner_session: owner_session(label),
        locus_binding: Some(sample_locus_for(label, &session_id, &model_session_id)),
    }
}

fn provider_kind_for(runtime_binding: &RuntimeBinding) -> ModelLaneProviderKind {
    match runtime_binding {
        RuntimeBinding::Local => ModelLaneProviderKind::LocalRuntime,
        RuntimeBinding::Cloud => ModelLaneProviderKind::OpenAi,
        RuntimeBinding::CliBridge => ModelLaneProviderKind::OfficialCli,
        RuntimeBinding::Human => ModelLaneProviderKind::Human,
        RuntimeBinding::Subagent => ModelLaneProviderKind::Subagent,
        RuntimeBinding::Validator => ModelLaneProviderKind::Validator,
    }
}

fn sample_message(label: &str) -> NewModelLaneMessage {
    sample_message_with(label, &message_id(label))
}

fn sample_message_with(label: &str, message_id: &str) -> NewModelLaneMessage {
    NewModelLaneMessage {
        message_id: message_id.into(),
        run_id: run_id(label),
        trace_id: format!("trace-mt002-{label}"),
        message_span_id: format!("span-{message_id}"),
        parent_span_id: Some(format!("span-lane-mt002-{label}-{label}")),
        linked_span_contexts: vec![format!("trace-mt002-{label}")],
        from_lane_id: default_lane_id(label),
        to_lane: ModelLaneTarget::Coordinator,
        routing: Some(ModelLaneRoutingMetadata {
            target_role: "coordinator".into(),
            target_session: coordinator_session_id(label),
            correlation_id: format!("correlation-{message_id}"),
            requires_ack: true,
            ack_for: None,
        }),
        kind: ModelLaneMessageKind::Proposal,
        payload_ref: format!("artifact://mt002/{label}/{message_id}"),
        payload_sha256: "b".repeat(64),
        event_ledger_stream_id: stream_id(label),
        summary: "typed local advisory proposal".into(),
        authority: ModelLaneAuthority::Advisory,
        promotion_decision_id: None,
        promotion_gate_ref: None,
        promotion_receipt_ref: None,
        validator_verdict_ref: None,
        operator_decision_ref: None,
        promoted_artifact_ref: None,
        promoted_artifact_sha256: None,
        promoted_artifact_version: None,
        tool_gate_decision_refs: vec!["tool-gate://mt002/read".into()],
        coordinator_session_id: coordinator_session_id(label),
        work_packet_id: Some(WP_ID.into()),
        micro_task_id: Some(MT_ID.into()),
        task_board_id: Some(TASK_BOARD_ID.into()),
        owner_session: owner_session(label),
        locus_binding: Some(sample_locus(label)),
        idempotency_key: format!("mt002-{message_id}"),
        replay_order_key: format!("0002-{message_id}"),
        replay_after_event_ledger_seq: None,
        proposal_ref: Some(format!("proposal://mt002/{label}")),
        // No CRDT posture by default: ordinary advisory messages carry null
        // crdt_* fields in production, and every non-null crdt_update_ref is
        // dereferenced against real persisted Yjs bytes at admission.
        crdt_update_ref: None,
        crdt_base_snapshot_ref: None,
        crdt_state_vector: None,
        crdt_proposal_ref: None,
        crdt_stale_base_ref: None,
        failstate_code: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane/message-replay".into()),
        created_at_utc: "2026-09-02T00:00:00Z".into(),
        diagnostic_payload: json!({
            "flight_recorder": "wired",
            "internal_diagnostics": "deferred",
            "palmistry": "deferred"
        }),
    }
}

/// A message declaring a COMPLETE but synthetic CRDT posture, used only by
/// validation negatives that are rejected before any durable authority
/// resolution runs.
fn crdt_posture_message(label: &str, message_id: &str) -> NewModelLaneMessage {
    let mut message = sample_message_with(label, message_id);
    message.crdt_update_ref = Some(format!("crdt-update://mt002/{message_id}"));
    message.crdt_base_snapshot_ref = Some("crdt-snapshot://mt002/base".into());
    message.crdt_state_vector = Some("sv:1".into());
    message.crdt_proposal_ref = Some(format!("crdt-proposal://mt002/{message_id}"));
    message
}

fn sample_locus(label: &str) -> ModelLaneLocusBinding {
    let lane_id = default_lane_id(label);
    sample_locus_for(
        label,
        &session_id_for(&lane_id),
        &model_session_id_for(&lane_id),
    )
}

fn sample_locus_for(
    label: &str,
    session_id: &str,
    model_session_id: &str,
) -> ModelLaneLocusBinding {
    ModelLaneLocusBinding {
        work_packet_id: WP_ID.into(),
        micro_task_id: MT_ID.into(),
        task_board_id: Some(TASK_BOARD_ID.into()),
        coordinator_session_id: coordinator_session_id(label),
        session_id: session_id.into(),
        model_session_id: model_session_id.into(),
        owner_session: owner_session(label),
        locus_binding_ref: format!("locus://wp1/mt002/{label}"),
    }
}

struct DexterityProofFactory {
    ledger: LedgerBatcher,
    unloads: Arc<AtomicUsize>,
}

#[async_trait]
impl ModelSessionFactory for DexterityProofFactory {
    async fn create(&self, request: &SpawnRequest) -> Result<LiveSession, SwarmError> {
        let record_id = ProcessOwnershipRecordId::new_v7();
        let os_pid = 55000 + request.instance_id.instance;
        let start = ProcessStart::new(
            ProcessEngineKind::LlamaCpp,
            request.owner_role.clone(),
            request.owner_wp.clone(),
        )
        .with_process_uuid(record_id.as_uuid())
        .with_os_pid(os_pid)
        .with_parent_session_id(request.parent_session_id.clone())
        .with_wp_id(request.wp_id.clone().unwrap_or_default())
        .with_mt_id(request.mt_id.clone().unwrap_or_default());
        self.ledger
            .record_start(start)
            .map_err(|err| SwarmError::LedgerFailed(err.to_string()))?;

        let mut owned_runtime = DexterityProofRuntime::new(self.unloads.clone());
        let model_id = owned_runtime
            .load(dexterity_load_spec())
            .await
            .map_err(|err| SwarmError::FactoryFailed(err.to_string()))?;
        let owned_runtime = Arc::new(tokio::sync::Mutex::new(owned_runtime));
        let shared_runtime = DexterityProofRuntime::new(self.unloads.clone());
        let teardown: handshake_core::swarm_orchestration::SessionTeardown = Arc::new(move || {
            let owned_runtime = Arc::clone(&owned_runtime);
            Box::pin(async move {
                owned_runtime
                    .lock()
                    .await
                    .unload(model_id)
                    .await
                    .map_err(|err| SwarmError::Internal(err.to_string()))
            })
        });
        Ok(LiveSession::new(
            Arc::new(shared_runtime),
            model_id,
            CancellationToken::new(),
            teardown,
            record_id,
            os_pid,
        ))
    }
}

struct DexterityProofRuntime {
    capabilities: ModelCapabilities,
    kv: KvCacheHandle,
    lora: LoraStackHandle,
    steering: SteeringHookHandle,
    unloads: Arc<AtomicUsize>,
}

impl DexterityProofRuntime {
    fn new(unloads: Arc<AtomicUsize>) -> Self {
        Self {
            capabilities: ModelCapabilities::default(),
            kv: KvCacheHandle::new("dexterity-mt003-kv"),
            lora: LoraStackHandle::new("dexterity-mt003-lora"),
            steering: SteeringHookHandle::new("dexterity-mt003-steering"),
            unloads,
        }
    }
}

#[async_trait]
impl ModelRuntime for DexterityProofRuntime {
    async fn load(&mut self, _spec: LoadSpec) -> Result<ModelId, ModelRuntimeError> {
        Ok(ModelId::new_v7())
    }

    async fn unload(&mut self, _id: ModelId) -> Result<(), ModelRuntimeError> {
        self.unloads.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn generate(&self, req: GenerateRequest) -> TokenStream {
        let cancel = req.cancel.clone();
        let items = (0..req.max_tokens.min(2)).map(move |i| {
            if cancel.is_cancelled() {
                Err(ModelRuntimeError::Cancelled)
            } else {
                Ok(handshake_core::model_runtime::GeneratedToken {
                    token_id: i,
                    text: format!("dexterity-token-{i}"),
                    logprob: None,
                    finish_reason: None,
                })
            }
        });
        Box::pin(stream::iter(items.collect::<Vec<_>>()))
    }

    async fn score(&self, _id: ModelId, _sequence: Vec<u32>) -> Result<Score, ModelRuntimeError> {
        Ok(Score {
            token_logprobs: vec![],
            mean_logprob: 0.0,
        })
    }

    async fn embed(&self, _id: ModelId, _text: &str) -> Result<Embedding, ModelRuntimeError> {
        Ok(Embedding { vector: vec![] })
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

fn dexterity_load_spec() -> LoadSpec {
    LoadSpec {
        artifact_path: std::path::PathBuf::from("dexterity-mt003-artifact"),
        sha256_expected: "a".repeat(64),
        runtime_kind: RuntimeKind::LlamaCpp,
        sampling_defaults: SamplingParams::default(),
        kv_cache_policy: KvCachePolicy::Default {
            quant: KvQuantSupport::Q4,
            prefix_cache_ttl_seconds: 0,
            max_bytes: None,
        },
        declared_capabilities: ModelCapabilities::default(),
        provider: ProviderKind::Local,
        engine_origin: None,
        external_engine_import: None,
    }
}

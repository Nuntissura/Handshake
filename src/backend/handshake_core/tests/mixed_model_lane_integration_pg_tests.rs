//! WP-1 MT-009: Dexterity mixed local/cloud/subagent integration proof.
//!
//! These tests use real PostgreSQL plus kernel_event_ledger rows. They prove
//! that a mixed ModelLaneRun is replayable, restartable, diagnosable by the
//! native Argus projection contract, and fail-closed when launch, payload,
//! CRDT, replay-order, direct-endpoint, or HBR posture authority is missing.

mod knowledge_pg_support;

use async_trait::async_trait;
use base64::Engine;
use futures::{stream, StreamExt};
use handshake_core::kernel::crdt::actor_site::{
    derive_knowledge_site_id, knowledge_crdt_identity, KnowledgeActorIdV1, KnowledgeActorKind,
};
use handshake_core::kernel::crdt::snapshot::{
    build_snapshot_bounded_replay_plan, new_crdt_snapshot_record, plan_crdt_compaction,
    CrdtCompactionAuditMode, CrdtCompactionDisposition, CrdtCompactionPolicyV1,
    CrdtSnapshotRecordInputV1,
};
use handshake_core::kernel::crdt::state_vector::{verify_causal_chain, KnowledgeStateVectorV1};
use handshake_core::kernel::crdt::yjs_bridge::{
    pull_yjs_updates, push_yjs_update, read_draft_head, YjsPushDenialReasonV1, YjsPushDenialV1,
    YjsPushOutcomeV1, YjsUpdateEnvelopeV1, YJS_UPDATE_ENCODING_V1, YJS_UPDATE_ENVELOPE_SCHEMA_ID,
};
use handshake_core::kernel::{KernelEventType, NewKernelEvent};
use handshake_core::model_runtime::{
    CancellationToken, Embedding, GenPrompt, GenerateRequest, GeneratedToken, KvCacheHandle,
    LoadSpec, LoraStackHandle, ModelCapabilities, ModelId, ModelRuntime, ModelRuntimeError,
    SamplingParams, Score, SteeringHookHandle, TokenStream,
};
use handshake_core::process_ledger::{
    LedgerBatcher, LedgerBatcherConfig, LedgerEventKind, LedgerOverflowEvent,
    PostgresProcessLedgerStore, ProcessEngineKind, ProcessLedgerError, ProcessLedgerOverflowSink,
    ProcessStart, ProcessStop,
};
use handshake_core::storage::postgres::PostgresDatabase;
use handshake_core::storage::Database;
use handshake_core::swarm_orchestration::model_lane::{
    LaunchAuthority, ModelLaneAuthority, ModelLaneCloudConsentReceiptStatus,
    ModelLaneCloudConsentScope, ModelLaneCloudExportPosture, ModelLaneCloudProjectionPlanStatus,
    ModelLaneCloudRetentionPolicy, ModelLaneDiagnosticTier, ModelLaneDiagnosticTierState,
    ModelLaneDiagnosticsLane, ModelLaneKind, ModelLaneLeaseScope, ModelLaneLeaseState,
    ModelLaneLocusBinding, ModelLaneMessageKind, ModelLaneMessageRecord, ModelLaneMtRuntimeStatus,
    ModelLanePromotionDenialReason, ModelLanePromotionOutcome, ModelLaneProviderKind,
    ModelLaneRecord, ModelLaneRecoveryEventKind, ModelLaneRecoveryFailureKind,
    ModelLaneRecoveryState, ModelLaneRecoveryStatus, ModelLaneRoutingMetadata,
    ModelLaneRoutingPolicy, ModelLaneStatus, ModelLaneStore, ModelLaneTarget, NewModelLane,
    NewModelLaneCloudConsentReceipt, NewModelLaneCloudProjectionPlan,
    NewModelLaneContextBundleArtifactBinding, NewModelLaneDiagnosticTierStatus, NewModelLaneLease,
    NewModelLaneMessage, NewModelLaneMtRuntimeStatus, NewModelLanePromotionDecision,
    NewModelLaneRecoveryCheckpoint, NewModelLaneRecoveryEvent, NewModelLaneRun, RuntimeBinding,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::Barrier;
use yrs::updates::{decoder::Decode, encoder::Encode};
use yrs::{Doc, GetString, ReadTxn, StateVector, Text, Transact, Update};

const WP_ID: &str = "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1";
const MT_ID: &str = "MT-009";
const TASK_BOARD_ID: &str = "task-board://wp-1";
const OWNER: &str = "KERNEL_BUILDER-MT009";
const USERMANUAL_BEHAVIOR: &str = "usermanual://model-lane-validation-harness#mixed-runtime";
const RUN_ID: &str = "run-mt009-mixed";
const LOCAL_LANE_ID: &str = "lane-mt009-local";
const CLOUD_LANE_ID: &str = "lane-mt009-cloud";
const SUBAGENT_LANE_ID: &str = "lane-mt009-subagent";
const LOCAL_MESSAGE_ID: &str = "msg-mt009-local";
const CLOUD_MESSAGE_ID: &str = "msg-mt009-cloud";
const SUBAGENT_MESSAGE_ID: &str = "msg-mt009-subagent";

#[tokio::test]
async fn mixed_local_cloud_subagent_run_persists_restarts_replays_and_projects() {
    let (pool, store) = model_lane_store().await;
    seed_cloud_authority(&store, RUN_ID, CLOUD_LANE_ID).await;

    let lane_ids = vec![
        LOCAL_LANE_ID.to_owned(),
        CLOUD_LANE_ID.to_owned(),
        SUBAGENT_LANE_ID.to_owned(),
    ];
    store
        .record_run(sample_run(RUN_ID, lane_ids.clone()))
        .await
        .expect("record mixed ModelLaneRun");
    store
        .record_lane(sample_lane(
            LOCAL_LANE_ID,
            RUN_ID,
            ModelLaneKind::LocalModel,
            RuntimeBinding::Local,
            LaunchAuthority::ModelRuntime,
        ))
        .await
        .expect("record local lane");
    store
        .record_lane(sample_lane(
            CLOUD_LANE_ID,
            RUN_ID,
            ModelLaneKind::CloudModel,
            RuntimeBinding::Cloud,
            LaunchAuthority::CloudLane,
        ))
        .await
        .expect("record cloud lane with durable ProjectionPlan/ConsentReceipt");
    store
        .record_lane(sample_lane(
            SUBAGENT_LANE_ID,
            RUN_ID,
            ModelLaneKind::Subagent,
            RuntimeBinding::Subagent,
            LaunchAuthority::SubagentManager,
        ))
        .await
        .expect("record no-OS subagent lane");

    let messages = vec![
        sample_message(LOCAL_MESSAGE_ID, RUN_ID, LOCAL_LANE_ID, "local", 1),
        sample_message(CLOUD_MESSAGE_ID, RUN_ID, CLOUD_LANE_ID, "cloud", 2),
        sample_message(SUBAGENT_MESSAGE_ID, RUN_ID, SUBAGENT_LANE_ID, "subagent", 3),
    ];
    for message in &messages {
        store
            .record_message(message.clone())
            .await
            .expect("record mixed lane message");
        store
            .record_context_bundle_artifact_binding(sample_artifact_binding_for_message(message))
            .await
            .expect("record ArtifactStore/EventLedger payload authority");
    }

    store
        .record_recovery_event(sample_recovery_event(
            "recovery-event-mt009-crdt-001",
            RUN_ID,
            Some(LOCAL_LANE_ID),
            ModelLaneRecoveryEventKind::CrdtUpdateObserved,
            1,
            Some(payload_ref(LOCAL_MESSAGE_ID)),
            None,
            Some("crdt-snapshot://mt009/base"),
            Some("sv:mt009:3"),
        ))
        .await
        .expect("record checkpoint-bounded CRDT recovery event");
    store
        .record_lane_lease(sample_lease(
            "lease-mt009-local-active",
            RUN_ID,
            LOCAL_LANE_ID,
            "2099-01-01T00:00:00Z",
            ModelLaneLeaseState::Active,
        ))
        .await
        .expect("record active lane lease");
    for (tier, state, evidence) in [
        (
            ModelLaneDiagnosticTier::FlightRecorder,
            ModelLaneDiagnosticTierState::Wired,
            "eventledger://kernel/model-lane/mt009",
        ),
        (
            ModelLaneDiagnosticTier::InternalDiagnostics,
            ModelLaneDiagnosticTierState::Wired,
            "hbr-int-009://dexterity/mixed-runtime",
        ),
        (
            ModelLaneDiagnosticTier::Palmistry,
            ModelLaneDiagnosticTierState::DeferredWithReason,
            "palmistry://wp1/model-lane/mt009/external-worktree",
        ),
    ] {
        store
            .record_diagnostic_tier_status(sample_tier(RUN_ID, tier, state, evidence))
            .await
            .expect("record HBR diagnostic tier");
    }
    store
        .record_mt_runtime_status(sample_mt_status(
            "mt-status-mt009-ready",
            RUN_ID,
            ModelLaneMtRuntimeStatus::ReadyForValidation,
        ))
        .await
        .expect("record MT runtime status");

    let checkpoint_high_watermark =
        event_stream_high_watermark(&pool, &event_stream_id(RUN_ID)).await;
    let checkpoint = store
        .record_recovery_checkpoint(sample_checkpoint(
            "checkpoint-mt009-mixed",
            RUN_ID,
            Some(LOCAL_LANE_ID),
            Some(SUBAGENT_MESSAGE_ID),
            Some("lease-mt009-local-active"),
            checkpoint_high_watermark,
            messages
                .iter()
                .map(|message| message.payload_ref.clone())
                .collect(),
        ))
        .await
        .expect("record restart checkpoint");

    let replay = store.replay_run(RUN_ID).await.expect("replay mixed run");
    assert_eq!(replay.run.run_id, RUN_ID);
    assert_eq!(replay.run.routing_policy, "mixed_local_cloud_subagent");
    assert_eq!(replay.run.lane_ids, lane_ids);
    assert_eq!(
        replay.run.candidate_model_ids,
        vec![
            "model://mt009/local/tinyllama".to_owned(),
            "model://mt009/cloud/openai/gpt-4o-mini".to_owned(),
            "subagent://mt009/coder".to_owned(),
        ]
    );
    assert_eq!(replay.lanes.len(), 3);
    assert_eq!(replay.messages.len(), 3);
    assert!(replay
        .messages
        .iter()
        .all(|message| message.event_ledger_event_id.starts_with("KE-")));
    assert_process_backed_lane_runtime_contract(
        replay_lane(&replay.lanes, LOCAL_LANE_ID),
        "local-runtime",
        RuntimeBinding::Local,
        LaunchAuthority::ModelRuntime,
        ModelLaneProviderKind::LocalRuntime,
    );
    assert_process_backed_lane_runtime_contract(
        replay_lane(&replay.lanes, CLOUD_LANE_ID),
        "openai-byok",
        RuntimeBinding::Cloud,
        LaunchAuthority::CloudLane,
        ModelLaneProviderKind::OpenAi,
    );
    assert_no_os_lane_runtime_contract(replay_lane(&replay.lanes, SUBAGENT_LANE_ID));

    let recovered = store
        .recover_run_after_restart(RUN_ID)
        .await
        .expect("recover mixed run from PostgreSQL/EventLedger checkpoint");
    assert_eq!(recovered.checkpoint.checkpoint_id, checkpoint.checkpoint_id);
    assert_eq!(recovered.replay.messages.len(), 3);
    assert_eq!(recovered.recovery_events.len(), 1);
    assert_eq!(recovered.active_leases.len(), 1);
    assert_eq!(recovered.mt_runtime_statuses.len(), 1);
    assert_eq!(
        recovered.mt_runtime_statuses[0].status,
        ModelLaneMtRuntimeStatus::ReadyForValidation
    );

    let materialized = materialize_crdt(&replay.messages);
    let mut shuffled = replay.messages.clone();
    shuffled.reverse();
    shuffled.push(replay.messages[0].clone());
    assert_eq!(
        materialized,
        materialize_crdt(&shuffled),
        "CRDT replay projection must be order-stable and duplicate-id tolerant"
    );
    assert_eq!(
        materialized.get("mt009.local"),
        Some(&"local proposed deterministic edit".to_owned())
    );
    assert_eq!(
        materialized.get("mt009.cloud"),
        Some(&"cloud advisory review".to_owned())
    );
    assert_eq!(
        materialized.get("mt009.subagent"),
        Some(&"subagent implementation note".to_owned())
    );

    let projection = store
        .diagnostics_projection(RUN_ID)
        .await
        .expect("build native diagnostics projection");
    assert_eq!(
        projection.schema_id,
        "hsk.model_lane_diagnostics_projection@1"
    );
    assert_eq!(
        projection.surface_contract_id,
        "native_swarm_lane_diagnostics"
    );
    assert_eq!(projection.run.run_id, RUN_ID);
    assert_eq!(projection.run.micro_task_id.as_deref(), Some(MT_ID));
    assert_eq!(projection.lanes.len(), 3);
    assert_eq!(projection.messages.len(), 3);
    assert_eq!(
        projection
            .lanes
            .iter()
            .map(|lane| lane.message_count)
            .sum::<usize>(),
        projection.messages.len()
    );
    let cloud_lane = projection
        .lanes
        .iter()
        .find(|lane| lane.lane_id == CLOUD_LANE_ID)
        .expect("cloud lane in projection");
    let expected_projection_plan_id = projection_plan_id(RUN_ID, CLOUD_LANE_ID);
    let expected_consent_receipt_id = consent_receipt_id(RUN_ID, CLOUD_LANE_ID);
    assert_eq!(
        cloud_lane.projection_plan_ref.as_deref(),
        Some(expected_projection_plan_id.as_str())
    );
    assert_eq!(
        cloud_lane.consent_receipt_ref.as_deref(),
        Some(expected_consent_receipt_id.as_str())
    );
    let subagent_lane = projection
        .lanes
        .iter()
        .find(|lane| lane.lane_id == SUBAGENT_LANE_ID)
        .expect("subagent lane in projection");
    assert!(subagent_lane.process_ownership_ref.is_none());
    assert!(subagent_lane
        .no_os_process_reason_ref
        .as_deref()
        .is_some_and(|value| value.contains("subagent_manager")));
    assert_eq!(projection.active_lease_count, 1);
    assert_eq!(projection.mt_runtime_statuses.len(), 1);
    assert_eq!(
        projection.mt_runtime_statuses[0].status,
        "ready_for_validation"
    );
    assert_eq!(
        projection
            .diagnostic_tiers
            .iter()
            .map(|tier| tier.tier.as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["flight_recorder", "internal_diagnostics", "palmistry"])
    );
    assert!(projection
        .diagnostic_tiers
        .iter()
        .any(|tier| tier.tier == "palmistry"
            && tier.state == "deferred_with_reason"
            && tier.follow_up_ref.is_some()));

    let overflow_events = record_mixed_runtime_process_ledger_evidence(&pool).await;
    assert_process_ledger_linked(
        &pool,
        projection_lane(&projection.lanes, LOCAL_LANE_ID),
        ProcessEngineKind::LlamaCpp,
        "local-runtime",
    )
    .await;
    assert_process_ledger_linked(
        &pool,
        projection_lane(&projection.lanes, CLOUD_LANE_ID),
        ProcessEngineKind::HelperSubprocess,
        "openai-byok",
    )
    .await;
    assert_eq!(
        overflow_events.len(),
        1,
        "MT-009 bounded ProcessOwnershipLedger writer must emit overflow evidence"
    );
    assert_eq!(overflow_events[0].event_type, "FR_EVT_LEDGER_OVERFLOW");
    assert_eq!(overflow_events[0].capacity, 4);
    assert_eq!(
        overflow_events[0].dropped_event_kind,
        LedgerEventKind::Start
    );
    assert_eq!(
        overflow_events[0].sampled_event_payload["metadata_jsonb"]["mt_id"],
        MT_ID
    );
}

fn replay_lane<'a>(lanes: &'a [ModelLaneRecord], lane_id: &str) -> &'a ModelLaneRecord {
    lanes
        .iter()
        .find(|lane| lane.lane_id == lane_id)
        .unwrap_or_else(|| panic!("expected replay lane {lane_id}"))
}

fn projection_lane<'a>(
    lanes: &'a [ModelLaneDiagnosticsLane],
    lane_id: &str,
) -> &'a ModelLaneDiagnosticsLane {
    lanes
        .iter()
        .find(|lane| lane.lane_id == lane_id)
        .unwrap_or_else(|| panic!("expected projection lane {lane_id}"))
}

fn assert_process_backed_lane_runtime_contract(
    lane: &ModelLaneRecord,
    expected_adapter_id: &str,
    expected_runtime_binding: RuntimeBinding,
    expected_launch_authority: LaunchAuthority,
    expected_provider_kind: ModelLaneProviderKind,
) {
    assert_eq!(lane.adapter_id, expected_adapter_id);
    assert_eq!(lane.runtime_binding, expected_runtime_binding);
    assert_eq!(lane.launch_authority, expected_launch_authority);
    assert_eq!(lane.provider_kind, expected_provider_kind);
    assert!(
        lane.process_ownership_ref.is_some(),
        "process-backed lane must carry ProcessOwnershipLedger ref"
    );
    assert!(
        lane.no_os_process_reason_ref.is_none(),
        "process-backed lane must not carry a no-OS-process reason"
    );
    assert!(
        lane.cancellation_ref
            .as_deref()
            .is_some_and(|value| value.starts_with("cancel-token://mt009/")),
        "lane must expose cooperative cancellation token"
    );
    assert_eq!(
        lane.terminal_status_mapping_ref.as_deref(),
        Some("terminal-status://mt009/mixed-runtime")
    );
    assert_eq!(
        lane.loop_counter_ref.as_deref(),
        Some("loop-counter://mt009/mixed-runtime"),
        "lane must expose bounded retry loop counter evidence"
    );
}

fn assert_no_os_lane_runtime_contract(lane: &ModelLaneRecord) {
    assert_eq!(lane.runtime_binding, RuntimeBinding::Subagent);
    assert_eq!(lane.launch_authority, LaunchAuthority::SubagentManager);
    assert_eq!(lane.provider_kind, ModelLaneProviderKind::Subagent);
    assert!(lane.process_ownership_ref.is_none());
    assert!(
        lane.no_os_process_reason_ref
            .as_deref()
            .is_some_and(|value| value.contains("subagent_manager")),
        "subagent lane must explain why no OS process exists"
    );
}

async fn record_mixed_runtime_process_ledger_evidence(pool: &PgPool) -> Vec<LedgerOverflowEvent> {
    let overflow = RecordingOverflowSink::default();
    let (batcher, drain) = LedgerBatcher::manual_for_tests(
        LedgerBatcherConfig {
            capacity: 4,
            batch_size: 4,
            flush_interval: Duration::from_millis(250),
        },
        Arc::new(overflow.clone()),
    )
    .expect("manual MT-009 ProcessOwnershipLedger writer");

    for (lane_id, engine_kind, adapter_id, os_pid) in [
        (
            LOCAL_LANE_ID,
            ProcessEngineKind::LlamaCpp,
            "local-runtime",
            59001,
        ),
        (
            CLOUD_LANE_ID,
            ProcessEngineKind::HelperSubprocess,
            "openai-byok",
            59002,
        ),
    ] {
        let start = process_start_for_lane(lane_id, engine_kind, adapter_id, os_pid);
        let stop = ProcessStop::from_start(&start, Some(0)).with_stop_reason("completed");
        batcher
            .record_start(start)
            .expect("enqueue MT-009 START evidence");
        batcher
            .record_stop(stop)
            .expect("enqueue MT-009 STOP evidence");
    }

    batcher
        .record_start(process_start_for_lane(
            "lane-mt009-overflow",
            ProcessEngineKind::HelperSubprocess,
            "overflow-proof",
            59009,
        ))
        .expect("bounded writer emits overflow without blocking spawn path");

    let ledger_store = Arc::new(PostgresProcessLedgerStore::new(pool.clone()));
    ledger_store
        .apply_migration()
        .await
        .expect("process ledger migration applies");
    drain
        .drain_available_to(ledger_store)
        .await
        .expect("MT-009 process ledger rows drain to PostgreSQL");
    overflow.events()
}

fn process_start_for_lane(
    lane_id: &str,
    engine_kind: ProcessEngineKind,
    adapter_id: &str,
    os_pid: u32,
) -> ProcessStart {
    ProcessStart::new(engine_kind, OWNER, Some(WP_ID.to_owned()))
        .with_process_uuid(process_uuid_for_lane(lane_id))
        .with_os_pid(os_pid)
        .with_parent_session_id(RUN_ID)
        .with_sandbox_adapter_id(adapter_id)
        .with_model_artifact_sha256(sample_sha256())
        .with_work_profile_id(format!("work-profile://mt009/{lane_id}"))
        .with_wp_id(WP_ID)
        .with_mt_id(MT_ID)
        .with_metadata_jsonb(json!({
            "adapter_id": adapter_id,
            "authority_path": "model_lane_store",
            "cancellation_boundary": "stream_chunk_or_tool_call",
            "direct_endpoint_bypass": false,
            "lane_id": lane_id,
            "mt_id": MT_ID,
            "retry_policy": "bounded_no_direct_endpoint",
            "run_id": RUN_ID,
        }))
}

fn process_uuid_for_lane(lane_id: &str) -> uuid::Uuid {
    let digest = Sha256::digest(format!("process-ledger://mt009/{lane_id}").as_bytes());
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x70;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    uuid::Uuid::from_bytes(bytes)
}

async fn assert_process_ledger_linked(
    pool: &PgPool,
    lane: &ModelLaneDiagnosticsLane,
    expected_engine_kind: ProcessEngineKind,
    expected_adapter_id: &str,
) {
    let process_uuid = lane_process_uuid(lane);
    let row = sqlx::query(
        r#"
        SELECT
            engine_kind,
            stopped_at IS NOT NULL AS has_stop,
            stop_reason,
            os_pid,
            parent_session_id,
            sandbox_adapter_id,
            wp_id,
            mt_id,
            metadata_jsonb
        FROM kernel_process_lifecycle
        WHERE process_uuid = $1::uuid
        "#,
    )
    .bind(process_uuid.to_string())
    .fetch_one(pool)
    .await
    .expect("lane ProcessOwnershipLedger ref resolves to durable row");

    let engine_kind: String = row.get("engine_kind");
    let has_stop: bool = row.get("has_stop");
    let stop_reason: Option<String> = row.get("stop_reason");
    let os_pid: Option<i64> = row.get("os_pid");
    let parent_session_id: Option<String> = row.get("parent_session_id");
    let sandbox_adapter_id: Option<String> = row.get("sandbox_adapter_id");
    let wp_id: Option<String> = row.get("wp_id");
    let mt_id: Option<String> = row.get("mt_id");
    let metadata_jsonb: Value = row.get("metadata_jsonb");

    assert_eq!(engine_kind, expected_engine_kind.as_str());
    assert!(has_stop, "START row must be paired with STOP evidence");
    assert_eq!(stop_reason.as_deref(), Some("completed"));
    assert!(
        os_pid.is_some(),
        "process-backed lane must carry OS pid evidence"
    );
    assert_eq!(parent_session_id.as_deref(), Some(RUN_ID));
    assert_eq!(sandbox_adapter_id.as_deref(), Some(expected_adapter_id));
    assert_eq!(wp_id.as_deref(), Some(WP_ID));
    assert_eq!(mt_id.as_deref(), Some(MT_ID));
    assert_eq!(metadata_jsonb["lane_id"], lane.lane_id);
    assert_eq!(metadata_jsonb["adapter_id"], expected_adapter_id);
    assert_eq!(
        metadata_jsonb["cancellation_boundary"],
        "stream_chunk_or_tool_call"
    );
    assert_eq!(metadata_jsonb["retry_policy"], "bounded_no_direct_endpoint");
    assert_eq!(metadata_jsonb["direct_endpoint_bypass"], json!(false));
    assert_eq!(metadata_jsonb["authority_path"], "model_lane_store");
}

fn lane_process_uuid(lane: &ModelLaneDiagnosticsLane) -> uuid::Uuid {
    let raw = lane
        .process_ownership_ref
        .as_deref()
        .and_then(|value| value.strip_prefix("process-ledger://"))
        .unwrap_or_else(|| {
            panic!(
                "lane {} must carry process-ledger://<uuid> ownership ref",
                lane.lane_id
            )
        });
    uuid::Uuid::parse_str(raw).unwrap_or_else(|error| {
        panic!(
            "lane {} ProcessOwnershipLedger ref must contain a UUID, got {raw}: {error}",
            lane.lane_id
        )
    })
}

#[derive(Clone, Default)]
struct RecordingOverflowSink {
    events: Arc<Mutex<Vec<LedgerOverflowEvent>>>,
}

impl RecordingOverflowSink {
    fn events(&self) -> Vec<LedgerOverflowEvent> {
        self.events.lock().expect("overflow sink lock").clone()
    }
}

impl ProcessLedgerOverflowSink for RecordingOverflowSink {
    fn emit_overflow(&self, event: LedgerOverflowEvent) -> Result<(), ProcessLedgerError> {
        self.events.lock().expect("overflow sink lock").push(event);
        Ok(())
    }
}

/// WP-1 MT-009 / HBR-SWARM-001 (REQUIRED acceptance row): two concurrent
/// agent/model lanes (local + subagent) PLUS a simulated operator lane write to
/// the SAME shared CRDT key at the same time.
///
/// The pre-existing mixed run gave each lane a DISJOINT key (`mt009.local` /
/// `mt009.cloud` / `mt009.subagent`) and recorded messages sequentially, so
/// convergence held by construction and shared-state contention was never
/// exercised. This drives real `tokio` concurrency against ONE key and proves:
///   * no deadlock / starvation - every concurrent `record_message` completes;
///   * no silent overwrite - all three updates persist with distinct EventLedger
///     sequences;
///   * deterministic convergence - `materialize_crdt` resolves exactly one
///     winner for the shared key by EventLedger order (not submission order),
///     and is stable under reordering + duplicate replay;
///   * the operator edit is a first-class `HumanOperator` lane
///     (`launch_authority=Operator`, `runtime_binding=Human`).
#[tokio::test]
async fn mixed_concurrent_model_and_operator_lanes_converge_on_shared_crdt_key() {
    const SWARM_RUN_ID: &str = "run-mt009-swarm001";
    const SWARM_LOCAL_LANE: &str = "lane-mt009-swarm-local";
    const SWARM_SUBAGENT_LANE: &str = "lane-mt009-swarm-subagent";
    const SWARM_OPERATOR_LANE: &str = "lane-mt009-swarm-operator";
    const SHARED_CRDT_KEY: &str = "mt009.shared.plan";

    let (_pool, store) = model_lane_store().await;
    store
        .record_run(sample_run(
            SWARM_RUN_ID,
            vec![
                SWARM_LOCAL_LANE.to_owned(),
                SWARM_SUBAGENT_LANE.to_owned(),
                SWARM_OPERATOR_LANE.to_owned(),
            ],
        ))
        .await
        .expect("record swarm ModelLaneRun");

    for (lane_id, kind, binding, authority) in [
        (
            SWARM_LOCAL_LANE,
            ModelLaneKind::LocalModel,
            RuntimeBinding::Local,
            LaunchAuthority::ModelRuntime,
        ),
        (
            SWARM_SUBAGENT_LANE,
            ModelLaneKind::Subagent,
            RuntimeBinding::Subagent,
            LaunchAuthority::SubagentManager,
        ),
        (
            SWARM_OPERATOR_LANE,
            ModelLaneKind::HumanOperator,
            RuntimeBinding::Human,
            LaunchAuthority::Operator,
        ),
    ] {
        store
            .record_lane(sample_lane(lane_id, SWARM_RUN_ID, kind, binding, authority))
            .await
            .unwrap_or_else(|err| panic!("record lane {lane_id}: {err}"));
    }

    let writers = [
        (
            "msg-mt009-swarm-local",
            SWARM_LOCAL_LANE,
            "local",
            "local proposes plan v1",
            1i64,
        ),
        (
            "msg-mt009-swarm-subagent",
            SWARM_SUBAGENT_LANE,
            "subagent",
            "subagent proposes plan v2",
            2i64,
        ),
        (
            "msg-mt009-swarm-operator",
            SWARM_OPERATOR_LANE,
            "operator",
            "operator edits plan v3",
            3i64,
        ),
    ];

    // Concurrent writers against one shared CRDT key.
    let shared_store = std::sync::Arc::new(store);
    let mut handles = Vec::new();
    for (message_id, lane_id, label, value, seq) in writers {
        let store = std::sync::Arc::clone(&shared_store);
        let mut message = sample_message(message_id, SWARM_RUN_ID, lane_id, label, seq);
        message.diagnostic_payload["crdt_key"] = json!(SHARED_CRDT_KEY);
        message.diagnostic_payload["crdt_value"] = json!(value);
        handles.push(tokio::spawn(
            async move { store.record_message(message).await },
        ));
    }
    for handle in handles {
        handle
            .await
            .expect("concurrent writer must not panic, deadlock, or starve")
            .expect("concurrent record_message on a shared CRDT key must succeed");
    }

    // Durable payload authority for each concurrently-written message.
    for (message_id, lane_id, label, _value, seq) in writers {
        let message = sample_message(message_id, SWARM_RUN_ID, lane_id, label, seq);
        shared_store
            .record_context_bundle_artifact_binding(sample_artifact_binding_for_message(&message))
            .await
            .expect("record ArtifactStore/EventLedger payload authority");
    }

    let replay = shared_store
        .replay_run(SWARM_RUN_ID)
        .await
        .expect("replay concurrent swarm run");

    // No silent overwrite: every concurrent update is durable and distinctly sequenced.
    assert_eq!(
        replay.messages.len(),
        3,
        "all three concurrent shared-key updates must persist"
    );
    let sequences: BTreeSet<i64> = replay
        .messages
        .iter()
        .map(|message| message.event_ledger_seq)
        .collect();
    assert_eq!(
        sequences.len(),
        3,
        "each concurrent write must receive a distinct EventLedger sequence"
    );

    // The operator edit is a first-class HumanOperator lane.
    let operator_lane = replay
        .lanes
        .iter()
        .find(|lane| lane.lane_id == SWARM_OPERATOR_LANE)
        .expect("operator lane persisted");
    assert_eq!(operator_lane.kind, ModelLaneKind::HumanOperator);
    assert_eq!(operator_lane.launch_authority, LaunchAuthority::Operator);
    assert_eq!(operator_lane.runtime_binding, RuntimeBinding::Human);

    // Convergence is decided by EventLedger order, not submission order.
    let expected_winner = replay
        .messages
        .iter()
        .max_by_key(|message| message.event_ledger_seq)
        .and_then(|message| {
            message
                .diagnostic_payload
                .get("crdt_value")
                .and_then(Value::as_str)
        })
        .expect("winning crdt_value")
        .to_owned();

    let materialized = materialize_crdt(&replay.messages);
    assert_eq!(
        materialized.len(),
        1,
        "all writers targeted a single shared key"
    );
    assert_eq!(
        materialized.get(SHARED_CRDT_KEY),
        Some(&expected_winner),
        "shared-key convergence must follow EventLedger order, with no lost update"
    );

    // Order-stable and duplicate-tolerant (idempotent replay).
    let mut shuffled = replay.messages.clone();
    shuffled.reverse();
    shuffled.push(replay.messages[0].clone());
    assert_eq!(
        materialize_crdt(&shuffled),
        materialized,
        "concurrent shared-key convergence must be order-stable and duplicate-id tolerant"
    );
}

/// MT-009 durable storage cancellation boundary: a real `ModelRuntime` token
/// stream emits one prefix chunk, then the same `CancellationToken` is
/// cancelled before its next poll. The persisted prefix remains replayable,
/// while the terminal EventLedger row blocks every late ModelLane message
/// (including a tool request) at the durable store boundary. The production
/// coordinator/CLI-capture path is separately exercised by
/// `operator_chat_launch_coordinator_cancellation_preserves_prefix_and_rejects_late_activity`.
#[tokio::test]
async fn mt009_midstream_cancellation_preserves_prefix_and_rejects_late_messages() {
    const RUN_ID: &str = "run-mt009-midstream-cancel";
    const LANE_ID: &str = "lane-mt009-midstream-cancel";
    const PREFIX_MESSAGE_ID: &str = "msg-mt009-midstream-prefix";

    let (pool, store) = model_lane_store().await;
    seed_run_lane(&store, RUN_ID, LANE_ID, RuntimeBinding::Local).await;

    let cancellation = CancellationToken::new();
    let runtime = CancellationProbeRuntime::new();
    let mut tokens = runtime.generate(cancellation_probe_request(cancellation.clone()));
    let prefix = tokens
        .next()
        .await
        .expect("runtime must emit a first chunk before cancellation")
        .expect("first chunk must be successful");
    assert_eq!(prefix.text, "mt009-prefix");

    let mut prefix_message = sample_message(PREFIX_MESSAGE_ID, RUN_ID, LANE_ID, "local", 1);
    prefix_message.diagnostic_payload["stream_token_id"] = json!(prefix.token_id);
    let prefix_record = store
        .record_message(prefix_message.clone())
        .await
        .expect("persist partial capture before cancellation");
    store
        .record_context_bundle_artifact_binding(sample_artifact_binding_for_message(
            &prefix_message,
        ))
        .await
        .expect("persist prefix payload authority before cancellation");
    record_checkpoint_at_highwater(
        &pool,
        &store,
        RUN_ID,
        LANE_ID,
        Some(PREFIX_MESSAGE_ID),
        vec![prefix_message.payload_ref.clone()],
        "checkpoint-mt009-midstream-cancel-prefix",
    )
    .await;

    cancellation.cancel();
    assert!(
        cancellation.is_cancelled(),
        "the exact GenerateRequest cancellation token must be flipped"
    );
    assert!(matches!(
        tokens
            .next()
            .await
            .expect("runtime must surface cancellation at its midstream boundary"),
        Err(ModelRuntimeError::Cancelled)
    ));

    let terminal = store
        .record_lane_terminal_status(
            LANE_ID,
            ModelLaneStatus::Cancelled,
            "model runtime cancellation token observed after partial capture",
        )
        .await
        .expect("persist cancelled terminal lane state");
    assert!(terminal.event_ledger_seq > prefix_record.event_ledger_seq);
    assert_eq!(terminal.status, ModelLaneStatus::Cancelled);

    let late_chunk = sample_message(
        "msg-mt009-midstream-late-chunk",
        RUN_ID,
        LANE_ID,
        "local",
        2,
    );
    let late_chunk_error = store
        .record_message(late_chunk.clone())
        .await
        .expect_err("a post-cancel chunk must fail closed before EventLedger append");
    assert!(
        late_chunk_error
            .to_string()
            .contains("terminal source lane"),
        "late chunk denial must identify the durable terminal source boundary: {late_chunk_error}"
    );
    assert_no_message_row(&pool, &late_chunk.message_id).await;

    let mut late_tool =
        sample_message("msg-mt009-midstream-late-tool", RUN_ID, LANE_ID, "local", 3);
    late_tool.kind = ModelLaneMessageKind::ToolRequest;
    let late_tool_error = store
        .record_message(late_tool.clone())
        .await
        .expect_err("a post-cancel tool request must fail closed before EventLedger append");
    assert!(late_tool_error.to_string().contains("terminal source lane"));
    assert_no_message_row(&pool, &late_tool.message_id).await;

    // The actual operator-capture persistence shape binds payload + message in
    // one transaction. A terminal rejection must roll both paths back rather
    // than leaving the old binding-before-message orphan behind.
    let late_bound = sample_message(
        "msg-mt009-midstream-late-bound",
        RUN_ID,
        LANE_ID,
        "local",
        4,
    );
    let late_binding = sample_artifact_binding_for_message(&late_bound);
    let late_binding_error = store
        .record_message_with_payload_binding(late_bound.clone(), late_binding.clone())
        .await
        .expect_err("terminal rejection must atomically reject payload binding and message");
    assert!(late_binding_error
        .to_string()
        .contains("terminal source lane"));
    assert_no_message_row(&pool, &late_bound.message_id).await;
    let late_binding_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM model_lane_context_bundle_artifacts WHERE artifact_binding_id = $1",
    )
    .bind(&late_binding.artifact_binding_id)
    .fetch_one(&pool)
    .await
    .expect("count atomically rejected artifact bindings");
    assert_eq!(
        late_binding_rows, 0,
        "terminal rejection must not leave an orphan payload binding"
    );

    let post_terminal_messages: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger \
         WHERE session_run_id = $1 \
           AND aggregate_type = 'model_lane_message' \
           AND event_sequence > $2",
    )
    .bind(event_stream_id(RUN_ID))
    .bind(terminal.event_ledger_seq)
    .fetch_one(&pool)
    .await
    .expect("count post-terminal ModelLaneMessage EventLedger rows");
    assert_eq!(
        post_terminal_messages, 0,
        "no chunk/tool EventLedger row may follow cancel"
    );

    let cancelled_terminal_events: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger \
         WHERE aggregate_type = 'model_lane_terminal' \
           AND aggregate_id = $1 \
           AND payload->>'status' = 'cancelled'",
    )
    .bind(LANE_ID)
    .fetch_one(&pool)
    .await
    .expect("count cancelled terminal EventLedger rows");
    assert_eq!(
        cancelled_terminal_events, 1,
        "cancel must produce exactly one terminal event"
    );

    let replay = store
        .replay_run(RUN_ID)
        .await
        .expect("replay cancelled run");
    assert_eq!(
        replay.messages.len(),
        1,
        "only the prefix remains replayable"
    );
    assert_eq!(replay.messages[0].message_id, PREFIX_MESSAGE_ID);
    assert_eq!(
        replay
            .lanes
            .iter()
            .find(|lane| lane.lane_id == LANE_ID)
            .expect("cancelled lane remains visible to replay")
            .status,
        ModelLaneStatus::Cancelled
    );
    let recovered = store
        .recover_run_after_restart(RUN_ID)
        .await
        .expect("checkpoint recovery preserves cancelled lane and captured prefix");
    assert_eq!(recovered.replay.messages.len(), 1);
    assert_eq!(
        recovered.replay.messages[0].trace_id,
        prefix_message.trace_id
    );
}

/// Source and target lane rows are deliberately locked in a canonical order.
/// This real-PostgreSQL probe exercises simultaneous A->B and B->A traffic;
/// it would deadlock with the former source-then-target lock order.
#[tokio::test]
async fn mt009_bidirectional_lane_messages_do_not_deadlock() {
    const RUN_ID: &str = "run-mt009-bidirectional-lock";
    const LANE_A: &str = "lane-mt009-bidirectional-a";
    const LANE_B: &str = "lane-mt009-bidirectional-b";

    let (_pool, store) = model_lane_store().await;
    store
        .record_run(sample_run(RUN_ID, vec![LANE_A.into(), LANE_B.into()]))
        .await
        .expect("record bidirectional run");
    for lane_id in [LANE_A, LANE_B] {
        store
            .record_lane(sample_lane(
                lane_id,
                RUN_ID,
                ModelLaneKind::LocalModel,
                RuntimeBinding::Local,
                LaunchAuthority::ModelRuntime,
            ))
            .await
            .expect("record bidirectional local lane");
    }

    let mut a_to_b = sample_message("msg-mt009-a-to-b", RUN_ID, LANE_A, "local", 1);
    a_to_b.to_lane = ModelLaneTarget::Lane(LANE_B.into());
    let mut b_to_a = sample_message("msg-mt009-b-to-a", RUN_ID, LANE_B, "local", 2);
    b_to_a.to_lane = ModelLaneTarget::Lane(LANE_A.into());
    let store = Arc::new(store);
    let first_store = Arc::clone(&store);
    let second_store = Arc::clone(&store);

    let joined = tokio::time::timeout(Duration::from_secs(5), async move {
        tokio::join!(
            first_store.record_message(a_to_b),
            second_store.record_message(b_to_a)
        )
    })
    .await
    .expect("opposite-direction messages must not deadlock");
    joined.0.expect("A->B message persists");
    joined.1.expect("B->A message persists");
}

/// MT-009 V2 CRDT boundary: a real PostgreSQL/EventLedger-backed document
/// receives Yjs-compatible update envelopes from two model lanes and an
/// operator lane. The proof exercises duplicate idempotency, stale-base
/// denial, snapshot-bounded replay, append-only compaction planning, and the
/// exact derived CRDT receipts carried by ModelLane messages.
#[tokio::test]
async fn mt009_real_postgres_yjs_updates_compaction_receipts_and_lane_state_converge() {
    const RUN_ID: &str = "run-mt009-real-yjs-crdt";
    const LOCAL_LANE: &str = "lane-mt009-real-yjs-local";
    const CLOUD_LANE: &str = "lane-mt009-real-yjs-cloud";
    const OPERATOR_LANE: &str = "lane-mt009-real-yjs-operator";
    const DOCUMENT_SCHEMA_ID: &str = "hsk.doc.rich_document@1";

    let Some(kpg) = knowledge_pg_support::knowledge_pg().await else {
        panic!("PostgreSQL/EventLedger is required for MT-009 Yjs CRDT proof");
    };
    let schema_url = kpg.schema_url.clone();
    let workspace_id = kpg.create_workspace().await;
    let db = kpg.db;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&schema_url)
        .await
        .expect("connect isolated schema for ModelLane CRDT receipts");
    let store = ModelLaneStore::new(pool);
    let document_id = format!("doc-mt009-yjs-{workspace_id}");
    let crdt_document_id = format!("crdt-mt009-yjs-{workspace_id}");

    seed_cloud_authority(&store, RUN_ID, CLOUD_LANE).await;
    store
        .record_run(sample_run(
            RUN_ID,
            vec![LOCAL_LANE.into(), CLOUD_LANE.into(), OPERATOR_LANE.into()],
        ))
        .await
        .expect("record real Yjs mixed-lane run");
    for (lane_id, kind, binding, authority) in [
        (
            LOCAL_LANE,
            ModelLaneKind::LocalModel,
            RuntimeBinding::Local,
            LaunchAuthority::ModelRuntime,
        ),
        (
            CLOUD_LANE,
            ModelLaneKind::CloudModel,
            RuntimeBinding::Cloud,
            LaunchAuthority::CloudLane,
        ),
        (
            OPERATOR_LANE,
            ModelLaneKind::HumanOperator,
            RuntimeBinding::Human,
            LaunchAuthority::Operator,
        ),
    ] {
        store
            .record_lane(sample_lane(lane_id, RUN_ID, kind, binding, authority))
            .await
            .expect("record local/cloud/operator lane for real Yjs proof");
    }

    let local_actor = KnowledgeActorIdV1::new(KnowledgeActorKind::LocalModel, "mt009-local-model")
        .expect("typed local model actor");
    let cloud_actor = KnowledgeActorIdV1::new(KnowledgeActorKind::CloudModel, "mt009-cloud-model")
        .expect("typed cloud model actor");
    let operator_actor = KnowledgeActorIdV1::new(KnowledgeActorKind::Operator, "mt009-operator")
        .expect("typed operator actor");
    let local_site = derive_knowledge_site_id(&workspace_id, &crdt_document_id, &local_actor);
    let cloud_site = derive_knowledge_site_id(&workspace_id, &crdt_document_id, &cloud_actor);
    let operator_site = derive_knowledge_site_id(&workspace_id, &crdt_document_id, &operator_actor);

    let mut state_vector = KnowledgeStateVectorV1::new();
    let canonical_yjs_doc = Doc::new();
    let local_pre_snapshot_bytes = mt009_append_yjs_text_update(
        &canonical_yjs_doc,
        u64::from(local_site.yjs_client_id),
        "[local-pre-snapshot]",
    );
    let local_pre_snapshot = mt009_push_yjs_update(
        &db,
        &workspace_id,
        &document_id,
        &crdt_document_id,
        DOCUMENT_SCHEMA_ID,
        "mt009-yjs-pre-local",
        &local_actor,
        &local_site.site_id,
        "session-mt009-yjs",
        &local_pre_snapshot_bytes,
        &mut state_vector,
        1,
    )
    .await;
    let cloud_pre_snapshot_bytes = mt009_append_yjs_text_update(
        &canonical_yjs_doc,
        u64::from(cloud_site.yjs_client_id),
        "[cloud-pre-snapshot]",
    );
    let cloud_pre_snapshot = mt009_push_yjs_update(
        &db,
        &workspace_id,
        &document_id,
        &crdt_document_id,
        DOCUMENT_SCHEMA_ID,
        "mt009-yjs-pre-cloud",
        &cloud_actor,
        &cloud_site.site_id,
        "session-mt009-yjs",
        &cloud_pre_snapshot_bytes,
        &mut state_vector,
        2,
    )
    .await;
    let operator_pre_snapshot_bytes = mt009_append_yjs_text_update(
        &canonical_yjs_doc,
        u64::from(operator_site.yjs_client_id),
        "[operator-pre-snapshot]",
    );
    let operator_pre_snapshot = mt009_push_yjs_update(
        &db,
        &workspace_id,
        &document_id,
        &crdt_document_id,
        DOCUMENT_SCHEMA_ID,
        "mt009-yjs-pre-operator",
        &operator_actor,
        &operator_site.site_id,
        "session-mt009-yjs",
        &operator_pre_snapshot_bytes,
        &mut state_vector,
        3,
    )
    .await;
    let snapshot_state_vector = state_vector.encode();
    let snapshot_bytes = canonical_yjs_doc
        .transact()
        .encode_state_as_update_v1(&StateVector::default());

    let snapshot_identity = knowledge_crdt_identity(
        &workspace_id,
        &document_id,
        &crdt_document_id,
        DOCUMENT_SCHEMA_ID,
        &operator_actor,
        "trace-mt009-yjs-snapshot",
    );
    let snapshot_event = NewKernelEvent::builder(
        format!("KTR-MT009-YJS-{workspace_id}"),
        "session-mt009-yjs".to_string(),
        KernelEventType::KnowledgeCrdtSnapshotRecorded,
        operator_actor.to_kernel_actor(),
    )
    .aggregate("knowledge_crdt_document", crdt_document_id.clone())
    .idempotency_key(format!("mt009-yjs:{workspace_id}:snapshot"))
    .source_component("mixed_model_lane_integration_pg_tests")
    .payload(json!({
        "covered_update_seq": 3,
        "state_vector": &snapshot_state_vector,
        "document_id": &document_id,
    }))
    .build()
    .expect("build snapshot EventLedger event");
    let snapshot_event = db
        .append_kernel_event(snapshot_event)
        .await
        .expect("append snapshot EventLedger event");
    let snapshot = new_crdt_snapshot_record(CrdtSnapshotRecordInputV1 {
        identity: &snapshot_identity,
        snapshot_id: "mt009-yjs-snapshot-3",
        covered_update_seq: 3,
        snapshot_bytes: &snapshot_bytes,
        snapshot_bytes_ref: &format!(
            "postgres://kernel_crdt_snapshots/{crdt_document_id}/mt009-yjs-snapshot-3"
        ),
        state_vector: &snapshot_state_vector,
        event_ledger_event_id: &snapshot_event.event_id,
        promotion_evidence_update_ids: &["mt009-yjs-pre-cloud"],
    });
    db.append_kernel_crdt_snapshot(snapshot.clone(), snapshot_bytes.clone())
        .await
        .expect("persist snapshot receipt and bytes in PostgreSQL");

    let local_post_snapshot_bytes = mt009_append_yjs_text_update(
        &canonical_yjs_doc,
        u64::from(local_site.yjs_client_id),
        "[local-post-snapshot]",
    );
    let local_post_snapshot = mt009_push_yjs_update(
        &db,
        &workspace_id,
        &document_id,
        &crdt_document_id,
        DOCUMENT_SCHEMA_ID,
        "mt009-yjs-post-local",
        &local_actor,
        &local_site.site_id,
        "session-mt009-yjs",
        &local_post_snapshot_bytes,
        &mut state_vector,
        4,
    )
    .await;
    let cloud_post_snapshot_bytes = mt009_append_yjs_text_update(
        &canonical_yjs_doc,
        u64::from(cloud_site.yjs_client_id),
        "[cloud-post-snapshot]",
    );
    let cloud_post_snapshot = mt009_push_yjs_update(
        &db,
        &workspace_id,
        &document_id,
        &crdt_document_id,
        DOCUMENT_SCHEMA_ID,
        "mt009-yjs-post-cloud",
        &cloud_actor,
        &cloud_site.site_id,
        "session-mt009-yjs",
        &cloud_post_snapshot_bytes,
        &mut state_vector,
        5,
    )
    .await;
    let operator_post_snapshot_bytes = mt009_append_yjs_text_update(
        &canonical_yjs_doc,
        u64::from(operator_site.yjs_client_id),
        "[operator-post-snapshot]",
    );
    let operator_post_snapshot = mt009_push_yjs_update(
        &db,
        &workspace_id,
        &document_id,
        &crdt_document_id,
        DOCUMENT_SCHEMA_ID,
        "mt009-yjs-post-operator",
        &operator_actor,
        &operator_site.site_id,
        "session-mt009-yjs",
        &operator_post_snapshot_bytes,
        &mut state_vector,
        6,
    )
    .await;
    let final_state_vector = state_vector.encode();
    let envelopes = vec![
        local_pre_snapshot,
        cloud_pre_snapshot,
        operator_pre_snapshot,
        local_post_snapshot,
        cloud_post_snapshot.clone(),
        operator_post_snapshot,
    ];

    match push_yjs_update(&db, &cloud_post_snapshot)
        .await
        .expect("duplicate update must return a typed outcome")
    {
        YjsPushOutcomeV1::AlreadyStored { update_seq, .. } => assert_eq!(update_seq, 5),
        other => panic!("expected idempotent cloud update replay, got {other:?}"),
    }

    let mut stale_after = KnowledgeStateVectorV1::new();
    stale_after.increment(&local_site.site_id);
    let stale = mt009_yjs_envelope(
        &workspace_id,
        &document_id,
        &crdt_document_id,
        DOCUMENT_SCHEMA_ID,
        "mt009-yjs-stale-base",
        &local_actor,
        "session-mt009-yjs",
        &local_post_snapshot_bytes,
        &KnowledgeStateVectorV1::new(),
        &stale_after,
    );
    match push_yjs_update(&db, &stale)
        .await
        .expect("stale update returns typed denial rather than writing")
    {
        YjsPushOutcomeV1::Denied { denial } => assert!(matches!(
            denial.reason,
            YjsPushDenialReasonV1::StaleBase {
                head_update_seq: 6,
                ..
            }
        )),
        other => panic!("stale state vector must be denied, got {other:?}"),
    }

    let mut malformed_after = state_vector.clone();
    malformed_after.increment(&local_site.site_id);
    let malformed_hash_consistent = mt009_yjs_envelope(
        &workspace_id,
        &document_id,
        &crdt_document_id,
        DOCUMENT_SCHEMA_ID,
        "mt009-yjs-malformed-but-hash-consistent",
        &local_actor,
        "session-mt009-yjs",
        b"not-a-yjs-v1-update",
        &state_vector,
        &malformed_after,
    );
    match push_yjs_update(&db, &malformed_hash_consistent)
        .await
        .expect("malformed Yjs bytes must be a typed denial, not a storage failure")
    {
        YjsPushOutcomeV1::Denied { denial } => match denial.reason {
            YjsPushDenialReasonV1::EnvelopeInvalid { messages } => assert!(
                messages
                    .iter()
                    .any(|message| message.contains("decodable Yjs v1 update")),
                "hash-consistent malformed bytes must be rejected at ingress: {messages:?}"
            ),
            other => panic!("expected envelope validation denial, got {other:?}"),
        },
        other => panic!("malformed Yjs bytes must never be stored, got {other:?}"),
    }

    // The client cannot forge a higher or foreign causal clock.  Both fields
    // are checked against the durable head and the server-derived next vector
    // before an EventLedger or CRDT row can be appended.
    let mut forged_after = state_vector.clone();
    forged_after.increment(&local_site.site_id);
    forged_after.increment("site-forged-by-client");
    let forged_state_vector = mt009_yjs_envelope(
        &workspace_id,
        &document_id,
        &crdt_document_id,
        DOCUMENT_SCHEMA_ID,
        "mt009-yjs-forged-state-vector",
        &local_actor,
        "session-mt009-yjs",
        &local_post_snapshot_bytes,
        &state_vector,
        &forged_after,
    );
    match push_yjs_update(&db, &forged_state_vector)
        .await
        .expect("forged state vector returns a typed denial")
    {
        YjsPushOutcomeV1::Denied { denial } => match denial.reason {
            YjsPushDenialReasonV1::EnvelopeInvalid { messages } => assert!(
                messages
                    .iter()
                    .any(|message| message.contains("server-derived next vector")),
                "forged vector must be denied before durable append: {messages:?}"
            ),
            other => panic!("expected state-vector envelope denial, got {other:?}"),
        },
        other => panic!("forged state vector must never be stored, got {other:?}"),
    }

    // A valid Yjs update from another deterministic client cannot be claimed
    // by the local actor/site.  This protects the actor-to-Yjs-client binding
    // at ingress rather than relying on fixture discipline alone.
    let mut foreign_after = state_vector.clone();
    foreign_after.increment(&local_site.site_id);
    let foreign_client_update = mt009_yjs_envelope(
        &workspace_id,
        &document_id,
        &crdt_document_id,
        DOCUMENT_SCHEMA_ID,
        "mt009-yjs-foreign-client-attribution",
        &local_actor,
        "session-mt009-yjs",
        &cloud_post_snapshot_bytes,
        &state_vector,
        &foreign_after,
    );
    match push_yjs_update(&db, &foreign_client_update)
        .await
        .expect("foreign Yjs client attribution returns a typed denial")
    {
        YjsPushOutcomeV1::Denied { denial } => match denial.reason {
            YjsPushDenialReasonV1::EnvelopeInvalid { messages } => assert!(
                messages
                    .iter()
                    .any(|message| message.contains("deterministic Yjs client id")),
                "foreign client bytes must be denied before durable append: {messages:?}"
            ),
            other => panic!("expected foreign-client envelope denial, got {other:?}"),
        },
        other => panic!("foreign client bytes must never be stored, got {other:?}"),
    }

    let head = read_draft_head(&db, &workspace_id, &document_id, &crdt_document_id)
        .await
        .expect("read persisted draft head");
    assert_eq!(head.head_update_seq, 6);
    assert_eq!(head.head_state_vector, final_state_vector);

    let records = db
        .list_kernel_crdt_updates(&workspace_id, &document_id, &crdt_document_id)
        .await
        .expect("list real PostgreSQL update receipts");
    assert_eq!(
        records.len(),
        6,
        "duplicate and stale updates never add rows"
    );
    let mut persisted_yjs_update_bytes = Vec::with_capacity(records.len());
    for (record, envelope) in records.iter().zip(&envelopes) {
        assert_eq!(record.update_id, envelope.update_id);
        assert_eq!(record.update_sha256, envelope.update_sha256);
        assert_eq!(record.state_vector_before, envelope.state_vector_before);
        assert_eq!(record.state_vector_after, envelope.state_vector_after);
        assert_eq!(record.replay_metadata.encoding, YJS_UPDATE_ENCODING_V1);
        let persisted_bytes = db
            .read_kernel_crdt_update_bytes(&record.update_bytes_ref)
            .await
            .expect("read persisted Yjs update bytes");
        assert_eq!(sha256_hex(&persisted_bytes), envelope.update_sha256);
        Update::decode_v1(&persisted_bytes)
            .expect("PostgreSQL-returned update bytes must remain decodable Yjs v1 payloads");
        persisted_yjs_update_bytes.push(persisted_bytes);
    }

    let expected_yjs_state = mt009_yjs_materialize_doc(&canonical_yjs_doc);
    let ledger_yjs_state = mt009_yjs_materialize_updates(&persisted_yjs_update_bytes);
    assert_eq!(
        ledger_yjs_state, expected_yjs_state,
        "materialized document state must come from the exact Yjs bytes returned by PostgreSQL"
    );
    let mut varied_yjs_bytes = persisted_yjs_update_bytes.clone();
    varied_yjs_bytes.reverse();
    varied_yjs_bytes.push(persisted_yjs_update_bytes[4].clone());
    assert_eq!(
        mt009_yjs_materialize_updates(&varied_yjs_bytes),
        expected_yjs_state,
        "Yjs materialization and its real state vector must converge across reverse replay and a duplicate update"
    );

    let causal_proof = verify_causal_chain(&records).expect("prove persisted causal chain");
    assert_eq!(causal_proof.final_state_vector, final_state_vector);
    let merged_in_ledger_order =
        records
            .iter()
            .fold(KnowledgeStateVectorV1::new(), |state, row| {
                state.merge(
                    &KnowledgeStateVectorV1::parse(&row.state_vector_after)
                        .expect("persisted state vector must parse"),
                )
            });
    let mut shuffled_with_duplicate = records.clone();
    shuffled_with_duplicate.reverse();
    shuffled_with_duplicate.push(records[4].clone());
    let merged_in_varied_order =
        shuffled_with_duplicate
            .iter()
            .fold(KnowledgeStateVectorV1::new(), |state, row| {
                state.merge(
                    &KnowledgeStateVectorV1::parse(&row.state_vector_after)
                        .expect("persisted state vector must parse"),
                )
            });
    assert_eq!(merged_in_ledger_order.encode(), final_state_vector);
    assert_eq!(
        merged_in_varied_order, merged_in_ledger_order,
        "derived state vector is stable across replay-order variation and duplicate receipts"
    );

    let snapshots = db
        .list_kernel_crdt_snapshots(&workspace_id, &document_id, &crdt_document_id)
        .await
        .expect("list real snapshot receipts");
    assert_eq!(snapshots, vec![snapshot.clone()]);
    assert_eq!(
        db.read_kernel_crdt_snapshot_bytes(&snapshot.snapshot_bytes_ref)
            .await
            .expect("read persisted snapshot bytes"),
        snapshot_bytes
    );
    let bounded_replay = build_snapshot_bounded_replay_plan(&snapshots[0], &records)
        .expect("snapshot must bound replay to post-snapshot updates");
    assert_eq!(bounded_replay.replay_from_update_seq, 4);
    assert_eq!(bounded_replay.ordered_updates.len(), 3);
    assert_eq!(bounded_replay.final_state_vector, final_state_vector);

    let compaction = plan_crdt_compaction(
        &snapshots[0],
        &records,
        &CrdtCompactionPolicyV1 {
            policy_id: "mt009-postgres-eventledger-append-only".into(),
            compact_through_update_seq: 3,
            audit_mode: CrdtCompactionAuditMode::EventLedgerAuditRefs,
            preserve_promotion_evidence: true,
        },
    )
    .expect("compaction receipt must retain audit and promotion evidence");
    assert!(compaction.decisions.iter().any(|decision| {
        decision.update_id == "mt009-yjs-pre-cloud"
            && decision.disposition == CrdtCompactionDisposition::RetainPromotionEvidence
    }));
    assert!(compaction.decisions.iter().any(|decision| {
        decision.update_id == "mt009-yjs-pre-local"
            && decision.disposition == CrdtCompactionDisposition::CompactWithAudit
            && decision.audit_ref.starts_with("eventledger://")
    }));
    assert!(
        compaction
            .decisions
            .iter()
            .filter(|decision| {
                decision.update_seq > 3
                    && decision.disposition == CrdtCompactionDisposition::RetainForReplay
            })
            .count()
            == 3
    );
    let records_after_compaction_plan = db
        .list_kernel_crdt_updates(&workspace_id, &document_id, &crdt_document_id)
        .await
        .expect("append-only compaction planning must not destroy receipt rows");
    assert_eq!(records_after_compaction_plan, records);

    let pulled = pull_yjs_updates(
        &db,
        &workspace_id,
        &document_id,
        &crdt_document_id,
        3,
        DOCUMENT_SCHEMA_ID,
    )
    .await
    .expect("pull post-snapshot replay envelopes from PostgreSQL");
    assert_eq!(pulled.head_state_vector, final_state_vector);
    assert_eq!(
        pulled
            .updates
            .iter()
            .map(|update| update.update_id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "mt009-yjs-post-local",
            "mt009-yjs-post-cloud",
            "mt009-yjs-post-operator",
        ]
    );
    let mut snapshot_bounded_yjs_bytes = vec![snapshot_bytes.clone()];
    snapshot_bounded_yjs_bytes.extend(pulled.updates.iter().map(|update| {
        base64::engine::general_purpose::STANDARD
            .decode(&update.update_b64)
            .expect("pulled update bytes must decode from the stored wire envelope")
    }));
    assert_eq!(
        mt009_yjs_materialize_updates(&snapshot_bounded_yjs_bytes),
        expected_yjs_state,
        "snapshot plus pulled PostgreSQL update bytes must restore the same materialized Yjs state"
    );

    let mut recorded_messages = Vec::new();
    for ((message_id, lane_id, lane_label, crdt_value), record) in [
        (
            "msg-mt009-yjs-post-local",
            LOCAL_LANE,
            "local",
            "local model merged Yjs edit",
        ),
        (
            "msg-mt009-yjs-post-cloud",
            CLOUD_LANE,
            "cloud",
            "cloud model merged Yjs review",
        ),
        (
            "msg-mt009-yjs-post-operator",
            OPERATOR_LANE,
            "operator",
            "operator merged Yjs decision",
        ),
    ]
    .into_iter()
    .zip(records.iter().skip(3))
    {
        let mut message = sample_message(
            message_id,
            RUN_ID,
            lane_id,
            lane_label,
            record.update_seq as i64,
        );
        message.crdt_update_ref = Some(record.update_bytes_ref.clone());
        message.crdt_base_snapshot_ref = Some(snapshot.snapshot_bytes_ref.clone());
        message.crdt_state_vector = Some(record.state_vector_after.clone());
        message.diagnostic_payload["crdt_update_id"] = json!(record.update_id);
        message.diagnostic_payload["crdt_key"] = json!("mt009.yjs.shared-document");
        message.diagnostic_payload["crdt_value"] = json!(crdt_value);
        message.payload_sha256 = sha256_hex(&canonical_json_bytes(
            &artifact_payload_json_for_message(&message),
        ));
        let stored_message = store
            .record_message_with_payload_binding(
                message.clone(),
                sample_artifact_binding_for_message(&message),
            )
            .await
            .expect("atomically record ModelLane message and payload authority for persisted Yjs receipt");
        recorded_messages.push(stored_message);
    }

    let replay = store
        .replay_run(RUN_ID)
        .await
        .expect("replay lane messages with derived PostgreSQL CRDT receipts");
    let replayed_crdt_refs = replay
        .messages
        .iter()
        .filter_map(|message| message.crdt_update_ref.as_deref())
        .collect::<Vec<_>>();
    assert_eq!(
        replayed_crdt_refs.len(),
        3,
        "each post-snapshot ModelLane replay message must retain its durable CRDT update reference"
    );
    let mut replayed_yjs_bytes = vec![snapshot_bytes.clone()];
    for update_ref in &replayed_crdt_refs {
        replayed_yjs_bytes.push(
            db.read_kernel_crdt_update_bytes(update_ref)
                .await
                .expect("replayed ModelLane CRDT reference must resolve to PostgreSQL bytes"),
        );
    }
    assert_eq!(
        mt009_yjs_materialize_updates(&replayed_yjs_bytes),
        expected_yjs_state,
        "ModelLane replay must materialize from its PostgreSQL-backed Yjs references, not diagnostic labels"
    );
    let mut varied_message_replay = replay.messages.clone();
    varied_message_replay.reverse();
    varied_message_replay.push(recorded_messages[0].clone());
    let mut varied_replayed_yjs_bytes = vec![snapshot_bytes.clone()];
    for update_ref in varied_message_replay
        .iter()
        .filter_map(|message| message.crdt_update_ref.as_deref())
    {
        varied_replayed_yjs_bytes.push(
            db.read_kernel_crdt_update_bytes(update_ref)
                .await
                .expect("varied ModelLane replay CRDT reference must resolve to PostgreSQL bytes"),
        );
    }
    assert_eq!(
        mt009_yjs_materialize_updates(&varied_replayed_yjs_bytes),
        expected_yjs_state,
        "ModelLane replay materialization is stable across varied order and a duplicate receipt"
    );

    let selected_message = &recorded_messages[0];
    let selected_ref = format!("model-lane-message://{}", selected_message.message_id);
    let current_state_vector = selected_message
        .crdt_state_vector
        .clone()
        .expect("linked Yjs message has derived state vector");
    let promotion_input = NewModelLanePromotionDecision {
        decision_id: "promotion-mt009-yjs-current".into(),
        run_id: RUN_ID.into(),
        trace_id: format!("trace-{RUN_ID}"),
        decision_span_id: "span-promotion-mt009-yjs-current".into(),
        parent_span_id: Some(selected_message.message_span_id.clone()),
        linked_span_contexts: vec![format!("trace-link://{RUN_ID}/promotion")],
        coordinator_session_id: format!("coordinator-{RUN_ID}"),
        routing_policy: ModelLaneRoutingPolicy::OperatorLane,
        input_refs: vec![selected_ref.clone()],
        selected_input_refs: vec![selected_ref],
        rejected_input_refs: vec![],
        validator_authority_ref: None,
        operator_authority_ref: Some("operator://mt009/yjs-merge".into()),
        expected_event_ledger_aggregate_type: "model_lane_message".into(),
        expected_event_ledger_aggregate_id: selected_message.message_id.clone(),
        expected_event_ledger_version: selected_message.event_ledger_seq,
        base_snapshot_ref: snapshot.snapshot_bytes_ref.clone(),
        current_base_snapshot_ref: snapshot.snapshot_bytes_ref.clone(),
        state_vector: current_state_vector.clone(),
        current_state_vector: current_state_vector.clone(),
        schema_id: "hsk.model_lane_message@1".into(),
        deterministic_tie_break_rule: "event_ledger_seq_then_message_id".into(),
        promotion_gate_ref: "promotion-gate://mt009/yjs/current".into(),
        promotion_receipt_ref: Some("promotion-receipt://mt009/yjs/current".into()),
        promoted_artifact_ref: Some("artifact://promoted/mt009/yjs/current".into()),
        promoted_artifact_sha256: Some(sample_sha256()),
        promoted_artifact_version: Some("1".into()),
        direct_authority_mutation_attempt_ref: None,
        event_ledger_stream_id: event_stream_id(RUN_ID),
        work_packet_id: Some(WP_ID.into()),
        micro_task_id: Some(MT_ID.into()),
        task_board_id: Some(TASK_BOARD_ID.into()),
        owner_session: OWNER.into(),
        idempotency_key: "idem-promotion-mt009-yjs-current".into(),
        replay_order_key: "00000007/promotion/mt009-yjs/current".into(),
        recovery_hint_ref: Some("usermanual://model-lane-validation-harness#recovery".into()),
        created_at_utc: "2026-07-01T00:00:00Z".into(),
        diagnostic_payload: json!({
            "crdt_snapshot_ref": &snapshot.snapshot_bytes_ref,
            "crdt_state_vector": selected_message.crdt_state_vector,
            "denial_probe": "current",
        }),
    };

    let mut stale_base_input = promotion_input.clone();
    stale_base_input.decision_id = "promotion-mt009-yjs-stale-base".into();
    stale_base_input.decision_span_id = "span-promotion-mt009-yjs-stale-base".into();
    stale_base_input.base_snapshot_ref = format!("{}-stale", snapshot.snapshot_bytes_ref);
    stale_base_input.promotion_gate_ref = "promotion-gate://mt009/yjs/stale-base".into();
    stale_base_input.promotion_receipt_ref =
        Some("promotion-receipt://mt009/yjs/stale-base".into());
    stale_base_input.promoted_artifact_ref =
        Some("artifact://promoted/mt009/yjs/stale-base".into());
    stale_base_input.idempotency_key = "idem-promotion-mt009-yjs-stale-base".into();
    stale_base_input.replay_order_key = "00000007/promotion/mt009-yjs/stale-base".into();
    stale_base_input.diagnostic_payload["denial_probe"] = json!("stale_base_snapshot_ref");
    let stale_promotion = store
        .record_promotion_decision(stale_base_input)
        .await
        .expect("persist stale-base promotion denial in EventLedger");
    assert_eq!(stale_promotion.outcome, ModelLanePromotionOutcome::Denied);
    assert_eq!(
        stale_promotion.denial_reason,
        Some(ModelLanePromotionDenialReason::StaleBase),
        "a promotion that cites a stale snapshot must be denied against the selected persisted receipt"
    );

    let stale_state_vector = KnowledgeStateVectorV1::new().encode();
    assert_ne!(
        stale_state_vector, current_state_vector,
        "the stale-state probe must differ from the selected persisted Yjs state vector"
    );
    let mut stale_state_input = promotion_input;
    stale_state_input.decision_id = "promotion-mt009-yjs-stale-state-vector".into();
    stale_state_input.decision_span_id = "span-promotion-mt009-yjs-stale-state-vector".into();
    stale_state_input.state_vector = stale_state_vector;
    stale_state_input.promotion_gate_ref = "promotion-gate://mt009/yjs/stale-state-vector".into();
    stale_state_input.promotion_receipt_ref =
        Some("promotion-receipt://mt009/yjs/stale-state-vector".into());
    stale_state_input.promoted_artifact_ref =
        Some("artifact://promoted/mt009/yjs/stale-state-vector".into());
    stale_state_input.idempotency_key = "idem-promotion-mt009-yjs-stale-state-vector".into();
    stale_state_input.replay_order_key = "00000007/promotion/mt009/yjs/stale-state-vector".into();
    stale_state_input.diagnostic_payload["denial_probe"] = json!("stale_state_vector");
    let stale_state_promotion = store
        .record_promotion_decision(stale_state_input)
        .await
        .expect("persist stale-state-vector promotion denial in EventLedger");
    assert_eq!(
        stale_state_promotion.outcome,
        ModelLanePromotionOutcome::Denied
    );
    assert_eq!(
        stale_state_promotion.denial_reason,
        Some(ModelLanePromotionDenialReason::StaleStateVector),
        "a promotion with the current snapshot but stale state vector must be denied independently"
    );
}

/// Two separately pooled PostgreSQL clients race from the same CRDT base. The
/// database-scoped advisory lock must choose exactly one durable transition;
/// it must never leave a committed EventLedger success receipt without the
/// corresponding `kernel_crdt_updates` row. The second half repeats the race
/// with the same update id to prove idempotency converges on one receipt.
#[tokio::test]
async fn mt009_yjs_atomic_cross_connection_race_keeps_eventledger_and_crdt_receipts_in_lockstep() {
    let Some(kpg) = knowledge_pg_support::knowledge_pg().await else {
        panic!("real PostgreSQL is required for the MT-009 CRDT atomicity proof");
    };
    let db_a = Arc::new(
        PostgresDatabase::connect(&kpg.schema_url, 5)
            .await
            .expect("open first independent PostgreSQL CRDT writer"),
    );
    let db_b = Arc::new(
        PostgresDatabase::connect(&kpg.schema_url, 5)
            .await
            .expect("open second independent PostgreSQL CRDT writer"),
    );
    let workspace_id = kpg.create_workspace().await;
    let document_id = format!("doc-mt009-yjs-atomic-{workspace_id}");
    let crdt_document_id = format!("crdt-mt009-yjs-atomic-{workspace_id}");
    let local_actor = KnowledgeActorIdV1::new(KnowledgeActorKind::LocalModel, "mt009-race-local")
        .expect("typed local writer actor");
    let cloud_actor = KnowledgeActorIdV1::new(KnowledgeActorKind::CloudModel, "mt009-race-cloud")
        .expect("typed cloud writer actor");
    let local_site = derive_knowledge_site_id(&workspace_id, &crdt_document_id, &local_actor);
    let cloud_site = derive_knowledge_site_id(&workspace_id, &crdt_document_id, &cloud_actor);
    let empty = KnowledgeStateVectorV1::new();
    let mut local_after = empty.clone();
    local_after.increment(&local_site.site_id);
    let mut cloud_after = empty.clone();
    cloud_after.increment(&cloud_site.site_id);
    let local_update = mt009_append_yjs_text_update(
        &Doc::new(),
        u64::from(local_site.yjs_client_id),
        "[atomic-local]",
    );
    let cloud_update = mt009_append_yjs_text_update(
        &Doc::new(),
        u64::from(cloud_site.yjs_client_id),
        "[atomic-cloud]",
    );
    let local_envelope = mt009_yjs_envelope(
        &workspace_id,
        &document_id,
        &crdt_document_id,
        "hsk.doc.rich_document@1",
        "mt009-yjs-atomic-local",
        &local_actor,
        "session-mt009-yjs-atomic",
        &local_update,
        &empty,
        &local_after,
    );
    let cloud_envelope = mt009_yjs_envelope(
        &workspace_id,
        &document_id,
        &crdt_document_id,
        "hsk.doc.rich_document@1",
        "mt009-yjs-atomic-cloud",
        &cloud_actor,
        "session-mt009-yjs-atomic",
        &cloud_update,
        &empty,
        &cloud_after,
    );

    let start = Arc::new(Barrier::new(3));
    let local_task = {
        let db = db_a.clone();
        let envelope = local_envelope.clone();
        let start = start.clone();
        tokio::spawn(async move {
            start.wait().await;
            push_yjs_update(db.as_ref(), &envelope).await
        })
    };
    let cloud_task = {
        let db = db_b.clone();
        let envelope = cloud_envelope.clone();
        let start = start.clone();
        tokio::spawn(async move {
            start.wait().await;
            push_yjs_update(db.as_ref(), &envelope).await
        })
    };
    start.wait().await;
    let local_outcome = local_task
        .await
        .expect("local task joins")
        .expect("local CRDT push returns typed outcome");
    let cloud_outcome = cloud_task
        .await
        .expect("cloud task joins")
        .expect("cloud CRDT push returns typed outcome");
    let outcomes = [&local_outcome, &cloud_outcome];
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| matches!(outcome, YjsPushOutcomeV1::Stored { .. }))
            .count(),
        1,
        "same-base cross-connection race must commit exactly one CRDT update"
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| {
                matches!(
                    outcome,
                    YjsPushOutcomeV1::Denied {
                        denial: YjsPushDenialV1 {
                            reason: YjsPushDenialReasonV1::StaleBase { .. },
                            ..
                        }
                    }
                )
            })
            .count(),
        1,
        "same-base loser must receive StaleBase, not SequenceSlotRace"
    );

    let records = db_a
        .list_kernel_crdt_updates(&workspace_id, &document_id, &crdt_document_id)
        .await
        .expect("read raced CRDT rows");
    assert_eq!(records.len(), 1);
    let events = db_a
        .list_kernel_events_for_aggregate("knowledge_crdt_document", &crdt_document_id)
        .await
        .expect("read EventLedger receipts for raced document");
    assert_eq!(events.len(), 1, "no orphan EventLedger success receipt");
    assert_eq!(events[0].event_id, records[0].event_ledger_event_id);

    let duplicate_document_id = format!("doc-mt009-yjs-atomic-duplicate-{workspace_id}");
    let duplicate_crdt_document_id = format!("crdt-mt009-yjs-atomic-duplicate-{workspace_id}");
    let duplicate_site =
        derive_knowledge_site_id(&workspace_id, &duplicate_crdt_document_id, &local_actor);
    let mut duplicate_after = KnowledgeStateVectorV1::new();
    duplicate_after.increment(&duplicate_site.site_id);
    let duplicate_update = mt009_append_yjs_text_update(
        &Doc::new(),
        u64::from(duplicate_site.yjs_client_id),
        "[atomic-duplicate]",
    );
    let duplicate_envelope = mt009_yjs_envelope(
        &workspace_id,
        &duplicate_document_id,
        &duplicate_crdt_document_id,
        "hsk.doc.rich_document@1",
        "mt009-yjs-atomic-duplicate",
        &local_actor,
        "session-mt009-yjs-atomic",
        &duplicate_update,
        &KnowledgeStateVectorV1::new(),
        &duplicate_after,
    );
    let duplicate_start = Arc::new(Barrier::new(3));
    let duplicate_a = {
        let db = db_a.clone();
        let envelope = duplicate_envelope.clone();
        let start = duplicate_start.clone();
        tokio::spawn(async move {
            start.wait().await;
            push_yjs_update(db.as_ref(), &envelope).await
        })
    };
    let duplicate_b = {
        let db = db_b.clone();
        let envelope = duplicate_envelope.clone();
        let start = duplicate_start.clone();
        tokio::spawn(async move {
            start.wait().await;
            push_yjs_update(db.as_ref(), &envelope).await
        })
    };
    duplicate_start.wait().await;
    let duplicate_outcomes = [
        duplicate_a
            .await
            .expect("first duplicate task joins")
            .expect("first duplicate returns typed outcome"),
        duplicate_b
            .await
            .expect("second duplicate task joins")
            .expect("second duplicate returns typed outcome"),
    ];
    assert_eq!(
        duplicate_outcomes
            .iter()
            .filter(|outcome| matches!(outcome, YjsPushOutcomeV1::Stored { .. }))
            .count(),
        1
    );
    assert_eq!(
        duplicate_outcomes
            .iter()
            .filter(|outcome| matches!(outcome, YjsPushOutcomeV1::AlreadyStored { .. }))
            .count(),
        1
    );
    let duplicate_records = db_a
        .list_kernel_crdt_updates(
            &workspace_id,
            &duplicate_document_id,
            &duplicate_crdt_document_id,
        )
        .await
        .expect("read duplicate CRDT rows");
    let duplicate_events = db_a
        .list_kernel_events_for_aggregate("knowledge_crdt_document", &duplicate_crdt_document_id)
        .await
        .expect("read duplicate EventLedger rows");
    assert_eq!(duplicate_records.len(), 1);
    assert_eq!(duplicate_events.len(), 1);
    assert_eq!(
        duplicate_events[0].event_id,
        duplicate_records[0].event_ledger_event_id
    );

    let rollback_document_id = format!("doc-mt009-yjs-atomic-rollback-{workspace_id}");
    let rollback_crdt_document_id = format!("crdt-mt009-yjs-atomic-rollback-{workspace_id}");
    let rollback_site =
        derive_knowledge_site_id(&workspace_id, &rollback_crdt_document_id, &local_actor);
    let mut rollback_after = KnowledgeStateVectorV1::new();
    rollback_after.increment(&rollback_site.site_id);
    let rollback_update = mt009_append_yjs_text_update(
        &Doc::new(),
        u64::from(rollback_site.yjs_client_id),
        "[atomic-rollback]",
    );
    let rollback_envelope = mt009_yjs_envelope(
        &workspace_id,
        &rollback_document_id,
        &rollback_crdt_document_id,
        "hsk.doc.rich_document@1",
        "mt009-yjs-force-rollback",
        &local_actor,
        "session-mt009-yjs-atomic",
        &rollback_update,
        &KnowledgeStateVectorV1::new(),
        &rollback_after,
    );
    let mut raw = kpg.raw_connection().await;
    sqlx::query(
        r#"
        CREATE FUNCTION mt009_fail_crdt_insert() RETURNS trigger AS $$
        BEGIN
            IF NEW.update_id = 'mt009-yjs-force-rollback' THEN
                RAISE EXCEPTION 'mt009 forced CRDT insert failure';
            END IF;
            RETURN NEW;
        END;
        $$ LANGUAGE plpgsql
        "#,
    )
    .execute(&mut raw)
    .await
    .expect("install isolated real-PostgreSQL CRDT failure trigger");
    sqlx::query(
        "CREATE TRIGGER mt009_fail_crdt_insert BEFORE INSERT ON kernel_crdt_updates FOR EACH ROW EXECUTE FUNCTION mt009_fail_crdt_insert()",
    )
    .execute(&mut raw)
    .await
    .expect("install isolated real-PostgreSQL CRDT failure trigger hook");
    let rollback_error = push_yjs_update(db_a.as_ref(), &rollback_envelope)
        .await
        .expect_err("forced CRDT insert failure must roll back the whole atomic write");
    assert!(
        rollback_error
            .to_string()
            .contains("mt009 forced CRDT insert failure"),
        "rollback error must retain the real database cause: {rollback_error}"
    );
    sqlx::query("DROP TRIGGER mt009_fail_crdt_insert ON kernel_crdt_updates")
        .execute(&mut raw)
        .await
        .expect("remove isolated failure trigger");
    sqlx::query("DROP FUNCTION mt009_fail_crdt_insert()")
        .execute(&mut raw)
        .await
        .expect("remove isolated failure trigger function");
    drop(raw);
    assert!(
        db_a.list_kernel_crdt_updates(
            &workspace_id,
            &rollback_document_id,
            &rollback_crdt_document_id,
        )
        .await
        .expect("read rollback CRDT rows")
        .is_empty(),
        "failed insert must leave no CRDT row"
    );
    assert!(
        db_a.list_kernel_events_for_aggregate(
            "knowledge_crdt_document",
            &rollback_crdt_document_id,
        )
        .await
        .expect("read rollback EventLedger rows")
        .is_empty(),
        "failed insert must roll back its EventLedger receipt too"
    );
    let rollback_head = read_draft_head(
        db_a.as_ref(),
        &workspace_id,
        &rollback_document_id,
        &rollback_crdt_document_id,
    )
    .await
    .expect("read rollback document head");
    assert_eq!(rollback_head.head_update_seq, 0);
    assert_eq!(
        rollback_head.head_state_vector,
        KnowledgeStateVectorV1::new().encode()
    );
}

#[tokio::test]
async fn mixed_model_lane_recovery_replays_post_checkpoint_eventledger_catchup() {
    let (pool, store) = model_lane_store().await;
    let run_id = "run-mt009-post-checkpoint-catchup";
    let lane_id = "lane-mt009-post-checkpoint-catchup";
    seed_run_lane(&store, run_id, lane_id, RuntimeBinding::Local).await;

    let before_checkpoint =
        sample_message("msg-mt009-before-checkpoint", run_id, lane_id, "local", 1);
    store
        .record_message(before_checkpoint.clone())
        .await
        .expect("record pre-checkpoint message");
    store
        .record_context_bundle_artifact_binding(sample_artifact_binding_for_message(
            &before_checkpoint,
        ))
        .await
        .expect("record pre-checkpoint payload authority");

    let checkpoint_highwater = event_stream_high_watermark(&pool, &event_stream_id(run_id)).await;
    store
        .record_recovery_checkpoint(sample_checkpoint(
            "checkpoint-mt009-post-checkpoint-catchup",
            run_id,
            Some(lane_id),
            Some(&before_checkpoint.message_id),
            None,
            checkpoint_highwater,
            vec![before_checkpoint.payload_ref.clone()],
        ))
        .await
        .expect("record checkpoint before catch-up rows");

    let after_checkpoint =
        sample_message("msg-mt009-after-checkpoint", run_id, lane_id, "local", 2);
    store
        .record_message(after_checkpoint.clone())
        .await
        .expect("record post-checkpoint message");
    store
        .record_context_bundle_artifact_binding(sample_artifact_binding_for_message(
            &after_checkpoint,
        ))
        .await
        .expect("record post-checkpoint payload authority");
    store
        .record_recovery_event(sample_recovery_event(
            "recovery-event-mt009-post-checkpoint",
            run_id,
            Some(lane_id),
            ModelLaneRecoveryEventKind::CrdtUpdateObserved,
            1,
            Some(after_checkpoint.payload_ref.clone()),
            None,
            Some("crdt-snapshot://mt009/base"),
            Some("sv:mt009:3"),
        ))
        .await
        .expect("record post-checkpoint recovery event");
    store
        .record_mt_runtime_status(sample_mt_status(
            "mt-status-mt009-post-checkpoint",
            run_id,
            ModelLaneMtRuntimeStatus::ProofRunning,
        ))
        .await
        .expect("record post-checkpoint MT runtime status");

    let recovered = store
        .recover_run_after_restart(run_id)
        .await
        .expect("recover with EventLedger catch-up after checkpoint");
    let message_ids = recovered
        .replay
        .messages
        .iter()
        .map(|message| message.message_id.as_str())
        .collect::<BTreeSet<_>>();
    assert!(message_ids.contains("msg-mt009-before-checkpoint"));
    assert!(message_ids.contains("msg-mt009-after-checkpoint"));
    assert_eq!(
        recovered.recovery_events[0].recovery_event_id,
        "recovery-event-mt009-post-checkpoint"
    );
    assert_eq!(
        recovered.mt_runtime_statuses[0].status,
        ModelLaneMtRuntimeStatus::ProofRunning
    );
}

#[tokio::test]
async fn mixed_model_lane_recovery_rejects_eventledger_aggregate_mismatch() {
    for aggregate_type in [
        "model_lane",
        "model_lane_message",
        "model_lane_context_bundle_artifact",
    ] {
        let (pool, store) = model_lane_store().await;
        let suffix = aggregate_type.replace('_', "-");
        let run_id = format!("run-mt009-aggregate-mismatch-{suffix}");
        let lane_id = format!("lane-mt009-aggregate-mismatch-{suffix}");
        let message_id = format!("msg-mt009-aggregate-mismatch-{suffix}");
        seed_run_lane(&store, &run_id, &lane_id, RuntimeBinding::Local).await;
        let message = sample_message(&message_id, &run_id, &lane_id, "local", 1);
        store
            .record_message(message.clone())
            .await
            .expect("record aggregate mismatch message");
        store
            .record_context_bundle_artifact_binding(sample_artifact_binding_for_message(&message))
            .await
            .expect("record aggregate mismatch artifact authority");
        record_checkpoint_at_highwater(
            &pool,
            &store,
            &run_id,
            &lane_id,
            Some(&message_id),
            vec![message.payload_ref.clone()],
            &format!("checkpoint-mt009-aggregate-mismatch-{suffix}"),
        )
        .await;
        let original_aggregate_id = match aggregate_type {
            "model_lane" => lane_id.clone(),
            "model_lane_message" => message_id.clone(),
            "model_lane_context_bundle_artifact" => format!("artifact-binding-{message_id}"),
            other => panic!("unexpected aggregate type {other}"),
        };
        sqlx::query(
            r#"
            UPDATE kernel_event_ledger
            SET aggregate_id = $3
            WHERE aggregate_type = $1
              AND aggregate_id = $2
            "#,
        )
        .bind(aggregate_type)
        .bind(&original_aggregate_id)
        .bind(format!("{original_aggregate_id}-tampered"))
        .execute(&pool)
        .await
        .expect("tamper EventLedger aggregate_id");

        let err = store
            .recover_run_after_restart(&run_id)
            .await
            .expect_err("recovery must reject EventLedger aggregate-id mismatch");
        assert_error_contains(&err, "aggregate_id mismatch");
    }
}

#[tokio::test]
async fn mixed_model_lane_recovery_rejects_cloud_denial_aggregate_mismatch() {
    let (pool, store) = model_lane_store().await;
    let run_id = "run-mt009-cloud-denial-aggregate-mismatch";
    let local_lane_id = "lane-mt009-cloud-denial-local";
    let denied_lane_id = "lane-mt009-cloud-denial-cloud";
    seed_run_lane(&store, run_id, local_lane_id, RuntimeBinding::Local).await;

    let denied_err = store
        .record_prepared_launch((
            sample_run(run_id, vec![denied_lane_id.to_owned()]),
            sample_lane(
                denied_lane_id,
                run_id,
                ModelLaneKind::CloudModel,
                RuntimeBinding::Cloud,
                LaunchAuthority::CloudLane,
            ),
        ))
        .await
        .expect_err("cloud lane without ProjectionPlan/ConsentReceipt must be denied");
    assert_error_contains(&denied_err, "CX-MM-007");

    record_checkpoint_at_highwater(
        &pool,
        &store,
        run_id,
        local_lane_id,
        None,
        vec![],
        "checkpoint-mt009-cloud-denial-aggregate-mismatch",
    )
    .await;

    let tampered = sqlx::query(
        r#"
        UPDATE kernel_event_ledger
        SET aggregate_id = $3
        WHERE aggregate_type = 'model_lane_cloud_consent_denial'
          AND aggregate_id = $1
          AND payload->>'run_id' = $2
        "#,
    )
    .bind(denied_lane_id)
    .bind(run_id)
    .bind(format!("{denied_lane_id}-tampered"))
    .execute(&pool)
    .await
    .expect("tamper cloud denial aggregate_id");
    assert_eq!(tampered.rows_affected(), 1);

    let err = store
        .recover_run_after_restart(run_id)
        .await
        .expect_err("recovery must reject cloud denial aggregate mismatch");
    assert_error_contains(&err, "aggregate_id mismatch");
}

#[tokio::test]
async fn mixed_model_lane_negative_guards_fail_closed() {
    let (pool, store) = model_lane_store().await;

    let denied_run_id = "run-mt009-cloud-denied";
    let denied_lane_id = "lane-mt009-cloud-denied";
    let denied_err = store
        .record_prepared_launch((
            sample_run(denied_run_id, vec![denied_lane_id.to_owned()]),
            sample_lane(
                denied_lane_id,
                denied_run_id,
                ModelLaneKind::CloudModel,
                RuntimeBinding::Cloud,
                LaunchAuthority::CloudLane,
            ),
        ))
        .await
        .expect_err("cloud lane without durable consent must fail closed");
    assert_error_contains(&denied_err, "CX-MM-007");
    assert_no_lane_row(&pool, denied_lane_id).await;
    assert_denial_event(&pool, denied_run_id, denied_lane_id).await;

    seed_run_lane(
        &store,
        "run-mt009-direct-bypass",
        "lane-mt009-direct-bypass",
        RuntimeBinding::Local,
    )
    .await;
    let mut hidden_payload = sample_message(
        "msg-mt009-hidden-provider",
        "run-mt009-direct-bypass",
        "lane-mt009-direct-bypass",
        "local",
        1,
    );
    hidden_payload.payload_ref = "provider-session://openai/thread-hidden".into();
    let hidden_err = store
        .record_message(hidden_payload)
        .await
        .expect_err("hidden provider endpoint cannot become payload authority");
    assert_error_contains(&hidden_err, "hidden provider/session memory");

    seed_run_lane(
        &store,
        "run-mt009-off-stream",
        "lane-mt009-off-stream",
        RuntimeBinding::Local,
    )
    .await;
    let mut off_stream_msg = sample_message(
        "msg-mt009-off-stream",
        "run-mt009-off-stream",
        "lane-mt009-off-stream",
        "local",
        1,
    );
    off_stream_msg.event_ledger_stream_id = "mlane-stream-run-mt009-off-stream-shadow".into();
    let off_stream_err = store
        .record_message(off_stream_msg)
        .await
        .expect_err("message writes must use the source lane/run EventLedger stream");
    assert_error_contains(&off_stream_err, "message.event_ledger_stream_id");
    assert_no_message_row(&pool, "msg-mt009-off-stream").await;

    seed_run_lane(
        &store,
        "run-mt009-off-stream-lease",
        "lane-mt009-off-stream-lease",
        RuntimeBinding::Local,
    )
    .await;
    let mut off_stream_lease = sample_lease(
        "lease-mt009-off-stream",
        "run-mt009-off-stream-lease",
        "lane-mt009-off-stream-lease",
        "2026-07-01T00:05:00Z",
        ModelLaneLeaseState::Active,
    );
    off_stream_lease.event_ledger_stream_id =
        "mlane-stream-run-mt009-off-stream-lease-shadow".into();
    let off_stream_lease_err = store
        .record_lane_lease(off_stream_lease)
        .await
        .expect_err("lease writes must use the run EventLedger stream");
    assert_error_contains(
        &off_stream_lease_err,
        "model_lane_lease.event_ledger_stream_id",
    );
    assert_no_lease_row(&pool, "lease-mt009-off-stream").await;

    seed_run_lane(
        &store,
        "run-mt009-off-stream-status",
        "lane-mt009-off-stream-status",
        RuntimeBinding::Local,
    )
    .await;
    let mut off_stream_status = sample_mt_status(
        "mt-status-mt009-off-stream",
        "run-mt009-off-stream-status",
        ModelLaneMtRuntimeStatus::ProofRunning,
    );
    off_stream_status.event_ledger_stream_id =
        "mlane-stream-run-mt009-off-stream-status-shadow".into();
    let off_stream_status_err = store
        .record_mt_runtime_status(off_stream_status)
        .await
        .expect_err("MT runtime status writes must use the run EventLedger stream");
    assert_error_contains(
        &off_stream_status_err,
        "model_lane_mt_runtime_status.event_ledger_stream_id",
    );
    assert_no_mt_status_row(&pool, "mt-status-mt009-off-stream").await;

    seed_run_lane(
        &store,
        "run-mt009-target-missing",
        "lane-mt009-target-source",
        RuntimeBinding::Local,
    )
    .await;
    let mut missing_target_msg = sample_message(
        "msg-mt009-target-missing",
        "run-mt009-target-missing",
        "lane-mt009-target-source",
        "local",
        1,
    );
    missing_target_msg.to_lane = ModelLaneTarget::Lane("lane-mt009-target-missing".into());
    let missing_target_err = store
        .record_message(missing_target_msg)
        .await
        .expect_err("messages targeting a lane must reference an existing lane in the run");
    assert_error_contains(&missing_target_err, "lane_id lane-mt009-target-missing");

    seed_run_lane(
        &store,
        "run-mt009-target-source",
        "lane-mt009-target-source-local",
        RuntimeBinding::Local,
    )
    .await;
    seed_run_lane(
        &store,
        "run-mt009-target-other",
        "lane-mt009-target-other-local",
        RuntimeBinding::Local,
    )
    .await;
    let mut cross_run_target_msg = sample_message(
        "msg-mt009-cross-run-target",
        "run-mt009-target-source",
        "lane-mt009-target-source-local",
        "local",
        1,
    );
    cross_run_target_msg.to_lane = ModelLaneTarget::Lane("lane-mt009-target-other-local".into());
    let cross_run_target_err = store
        .record_message(cross_run_target_msg)
        .await
        .expect_err("messages cannot target a lane from another run");
    assert_error_contains(&cross_run_target_err, "lane.run_id");

    seed_run_lane(
        &store,
        "run-mt009-idempotency",
        "lane-mt009-idempotency",
        RuntimeBinding::Local,
    )
    .await;
    let idempotent_msg = sample_message(
        "msg-mt009-idempotency",
        "run-mt009-idempotency",
        "lane-mt009-idempotency",
        "local",
        1,
    );
    store
        .record_message(idempotent_msg.clone())
        .await
        .expect("record idempotency baseline message");
    let mut divergent_idempotent_msg = idempotent_msg.clone();
    divergent_idempotent_msg.message_id = "msg-mt009-idempotency-divergent".into();
    divergent_idempotent_msg.to_lane = ModelLaneTarget::Broadcast;
    let divergent_idempotency_err = store
        .record_message(divergent_idempotent_msg)
        .await
        .expect_err("same message idempotency key cannot mask divergent message semantics");
    assert_error_contains(&divergent_idempotency_err, "semantic_hash");

    seed_run_lane(
        &store,
        "run-mt009-diagnostics-row-drift",
        "lane-mt009-diagnostics-row-drift",
        RuntimeBinding::Local,
    )
    .await;
    let drift_msg = sample_message(
        "msg-mt009-diagnostics-row-drift",
        "run-mt009-diagnostics-row-drift",
        "lane-mt009-diagnostics-row-drift",
        "local",
        1,
    );
    store
        .record_message(drift_msg)
        .await
        .expect("record diagnostics row drift message");
    sqlx::query(
        r#"
        UPDATE model_lane_messages
        SET record_json = jsonb_set(record_json, '{summary}', '"tampered-summary"', false)
        WHERE message_id = 'msg-mt009-diagnostics-row-drift'
        "#,
    )
    .execute(&pool)
    .await
    .expect("tamper mutable diagnostics row");
    let diagnostics_drift_err = store
        .diagnostics_projection("run-mt009-diagnostics-row-drift")
        .await
        .expect_err("diagnostics projection must reject mutable row drift");
    assert_error_contains(&diagnostics_drift_err, "row drift");

    seed_run_lane(
        &store,
        "run-mt009-diagnostics-metadata-drift",
        "lane-mt009-diagnostics-metadata-drift",
        RuntimeBinding::Local,
    )
    .await;
    let metadata_drift_msg = sample_message(
        "msg-mt009-diagnostics-metadata-drift",
        "run-mt009-diagnostics-metadata-drift",
        "lane-mt009-diagnostics-metadata-drift",
        "local",
        1,
    );
    store
        .record_message(metadata_drift_msg)
        .await
        .expect("record diagnostics metadata drift message");
    sqlx::query(
        r#"
        UPDATE model_lane_messages
        SET record_json = jsonb_set(record_json, '{event_ledger_seq}', '999999', false)
        WHERE message_id = 'msg-mt009-diagnostics-metadata-drift'
        "#,
    )
    .execute(&pool)
    .await
    .expect("tamper diagnostics row EventLedger metadata");
    let metadata_drift_err = store
        .diagnostics_projection("run-mt009-diagnostics-metadata-drift")
        .await
        .expect_err("diagnostics projection must reject forged row metadata");
    assert_error_contains(&metadata_drift_err, "event_ledger_seq");

    seed_run_lane(
        &store,
        "run-mt009-diagnostics-record-event-id-drift",
        "lane-mt009-diagnostics-record-event-id-drift",
        RuntimeBinding::Local,
    )
    .await;
    let record_event_id_drift_msg = sample_message(
        "msg-mt009-diagnostics-record-event-id-drift",
        "run-mt009-diagnostics-record-event-id-drift",
        "lane-mt009-diagnostics-record-event-id-drift",
        "local",
        1,
    );
    store
        .record_message(record_event_id_drift_msg)
        .await
        .expect("record diagnostics record event id drift message");
    sqlx::query(
        r#"
        UPDATE model_lane_messages
        SET record_json = jsonb_set(record_json, '{event_ledger_event_id}', '"KE-tampered"', false)
        WHERE message_id = 'msg-mt009-diagnostics-record-event-id-drift'
        "#,
    )
    .execute(&pool)
    .await
    .expect("tamper record_json EventLedger event id");
    let record_event_id_drift_err = store
        .diagnostics_projection("run-mt009-diagnostics-record-event-id-drift")
        .await
        .expect_err("diagnostics projection must reject forged record event id");
    assert_error_contains(&record_event_id_drift_err, "event_ledger_event_id");

    seed_run_lane(
        &store,
        "run-mt009-diagnostics-record-stream-version-drift",
        "lane-mt009-diagnostics-record-stream-version-drift",
        RuntimeBinding::Local,
    )
    .await;
    let record_stream_version_drift_msg = sample_message(
        "msg-mt009-diagnostics-record-stream-version-drift",
        "run-mt009-diagnostics-record-stream-version-drift",
        "lane-mt009-diagnostics-record-stream-version-drift",
        "local",
        1,
    );
    store
        .record_message(record_stream_version_drift_msg)
        .await
        .expect("record diagnostics event_stream_version drift message");
    sqlx::query(
        r#"
        UPDATE model_lane_messages
        SET record_json = jsonb_set(record_json, '{event_stream_version}', '999999', false)
        WHERE message_id = 'msg-mt009-diagnostics-record-stream-version-drift'
        "#,
    )
    .execute(&pool)
    .await
    .expect("tamper record_json event_stream_version");
    let record_stream_version_drift_err = store
        .diagnostics_projection("run-mt009-diagnostics-record-stream-version-drift")
        .await
        .expect_err("diagnostics projection must reject forged event_stream_version");
    assert_error_contains(&record_stream_version_drift_err, "event_stream_version");

    seed_run_lane(
        &store,
        "run-mt009-diagnostics-record-transaction-seq-drift",
        "lane-mt009-diagnostics-record-transaction-seq-drift",
        RuntimeBinding::Local,
    )
    .await;
    let record_transaction_seq_drift_msg = sample_message(
        "msg-mt009-diagnostics-record-transaction-seq-drift",
        "run-mt009-diagnostics-record-transaction-seq-drift",
        "lane-mt009-diagnostics-record-transaction-seq-drift",
        "local",
        1,
    );
    store
        .record_message(record_transaction_seq_drift_msg)
        .await
        .expect("record diagnostics transaction_seq drift message");
    sqlx::query(
        r#"
        UPDATE model_lane_messages
        SET record_json = jsonb_set(record_json, '{transaction_seq}', '999999', false)
        WHERE message_id = 'msg-mt009-diagnostics-record-transaction-seq-drift'
        "#,
    )
    .execute(&pool)
    .await
    .expect("tamper record_json transaction_seq");
    let record_transaction_seq_drift_err = store
        .diagnostics_projection("run-mt009-diagnostics-record-transaction-seq-drift")
        .await
        .expect_err("diagnostics projection must reject forged transaction_seq");
    assert_error_contains(&record_transaction_seq_drift_err, "transaction_seq");

    seed_run_lane(
        &store,
        "run-mt009-diagnostics-row-event-seq-drift",
        "lane-mt009-diagnostics-row-event-seq-drift",
        RuntimeBinding::Local,
    )
    .await;
    let row_event_seq_drift_msg = sample_message(
        "msg-mt009-diagnostics-row-event-seq-drift",
        "run-mt009-diagnostics-row-event-seq-drift",
        "lane-mt009-diagnostics-row-event-seq-drift",
        "local",
        1,
    );
    store
        .record_message(row_event_seq_drift_msg)
        .await
        .expect("record diagnostics row event seq drift message");
    sqlx::query(
        r#"
        UPDATE model_lane_messages
        SET event_ledger_seq = event_ledger_seq + 1
        WHERE message_id = 'msg-mt009-diagnostics-row-event-seq-drift'
        "#,
    )
    .execute(&pool)
    .await
    .expect("tamper row EventLedger sequence column");
    let row_event_seq_drift_err = store
        .diagnostics_projection("run-mt009-diagnostics-row-event-seq-drift")
        .await
        .expect_err("diagnostics projection must reject row EventLedger column drift");
    assert_error_contains(&row_event_seq_drift_err, "row EventLedger columns");

    seed_run_lane(
        &store,
        "run-mt009-diagnostics-row-event-id-missing",
        "lane-mt009-diagnostics-row-event-id-missing",
        RuntimeBinding::Local,
    )
    .await;
    let row_event_id_missing_msg = sample_message(
        "msg-mt009-diagnostics-row-event-id-missing",
        "run-mt009-diagnostics-row-event-id-missing",
        "lane-mt009-diagnostics-row-event-id-missing",
        "local",
        1,
    );
    store
        .record_message(row_event_id_missing_msg)
        .await
        .expect("record diagnostics missing EventLedger row message");
    {
        let mut conn = pool
            .acquire()
            .await
            .expect("acquire pinned connection for missing-ledger tamper");
        sqlx::query("SET session_replication_role = replica")
            .execute(&mut *conn)
            .await
            .expect("disable FK triggers for missing-ledger tamper");
        sqlx::query(
            r#"
            UPDATE model_lane_messages
            SET event_ledger_event_id = 'KE-missing-mt009'
            WHERE message_id = 'msg-mt009-diagnostics-row-event-id-missing'
            "#,
        )
        .execute(&mut *conn)
        .await
        .expect("tamper row EventLedger event id to missing ledger row");
        sqlx::query("SET session_replication_role = origin")
            .execute(&mut *conn)
            .await
            .expect("restore FK triggers after missing-ledger tamper");
    }
    let row_event_id_missing_err = store
        .diagnostics_projection("run-mt009-diagnostics-row-event-id-missing")
        .await
        .expect_err("diagnostics projection must reject row EventLedger id with no ledger row");
    assert_error_contains(&row_event_id_missing_err, "kernel_event_ledger");

    seed_run_lane(
        &store,
        "run-mt009-diagnostics-row-stream-version-drift",
        "lane-mt009-diagnostics-row-stream-version-drift",
        RuntimeBinding::Local,
    )
    .await;
    let row_stream_version_drift_msg = sample_message(
        "msg-mt009-diagnostics-row-stream-version-drift",
        "run-mt009-diagnostics-row-stream-version-drift",
        "lane-mt009-diagnostics-row-stream-version-drift",
        "local",
        1,
    );
    store
        .record_message(row_stream_version_drift_msg)
        .await
        .expect("record diagnostics row event_stream_version drift message");
    sqlx::query(
        r#"
        UPDATE model_lane_messages
        SET event_stream_version = event_stream_version + 1
        WHERE message_id = 'msg-mt009-diagnostics-row-stream-version-drift'
        "#,
    )
    .execute(&pool)
    .await
    .expect("tamper row event_stream_version column");
    let row_stream_version_drift_err = store
        .diagnostics_projection("run-mt009-diagnostics-row-stream-version-drift")
        .await
        .expect_err("diagnostics projection must reject row event_stream_version drift");
    assert_error_contains(&row_stream_version_drift_err, "row event_stream_version");

    seed_run_lane(
        &store,
        "run-mt009-diagnostics-row-transaction-seq-drift",
        "lane-mt009-diagnostics-row-transaction-seq-drift",
        RuntimeBinding::Local,
    )
    .await;
    let row_transaction_seq_drift_msg = sample_message(
        "msg-mt009-diagnostics-row-transaction-seq-drift",
        "run-mt009-diagnostics-row-transaction-seq-drift",
        "lane-mt009-diagnostics-row-transaction-seq-drift",
        "local",
        1,
    );
    store
        .record_message(row_transaction_seq_drift_msg)
        .await
        .expect("record diagnostics row transaction_seq drift message");
    sqlx::query(
        r#"
        UPDATE model_lane_messages
        SET transaction_seq = transaction_seq + 1
        WHERE message_id = 'msg-mt009-diagnostics-row-transaction-seq-drift'
        "#,
    )
    .execute(&pool)
    .await
    .expect("tamper row transaction_seq column");
    let row_transaction_seq_drift_err = store
        .diagnostics_projection("run-mt009-diagnostics-row-transaction-seq-drift")
        .await
        .expect_err("diagnostics projection must reject row transaction_seq drift");
    assert_error_contains(&row_transaction_seq_drift_err, "row transaction_seq");

    seed_run_lane(
        &store,
        "run-mt009-diagnostics-record-id-drift",
        "lane-mt009-diagnostics-record-id-drift",
        RuntimeBinding::Local,
    )
    .await;
    let record_id_drift_msg = sample_message(
        "msg-mt009-diagnostics-record-id-drift",
        "run-mt009-diagnostics-record-id-drift",
        "lane-mt009-diagnostics-record-id-drift",
        "local",
        1,
    );
    store
        .record_message(record_id_drift_msg)
        .await
        .expect("record diagnostics record message id drift message");
    sqlx::query(
        r#"
        UPDATE model_lane_messages
        SET record_json = jsonb_set(record_json, '{message_id}', '"msg-mt009-diagnostics-record-id-tampered"', false)
        WHERE message_id = 'msg-mt009-diagnostics-record-id-drift'
        "#,
    )
    .execute(&pool)
    .await
    .expect("tamper record_json message_id");
    let record_id_drift_err = store
        .diagnostics_projection("run-mt009-diagnostics-record-id-drift")
        .await
        .expect_err("diagnostics projection must reject record_json message id drift");
    assert_error_contains(&record_id_drift_err, "mutable row");

    seed_run_lane(
        &store,
        "run-mt009-diagnostics-sql-row-id-drift",
        "lane-mt009-diagnostics-sql-row-id-drift",
        RuntimeBinding::Local,
    )
    .await;
    let sql_row_id_original_msg = sample_message(
        "msg-mt009-diagnostics-sql-row-id-original",
        "run-mt009-diagnostics-sql-row-id-drift",
        "lane-mt009-diagnostics-sql-row-id-drift",
        "local",
        1,
    );
    let sql_row_id_other_msg = sample_message(
        "msg-mt009-diagnostics-sql-row-id-other",
        "run-mt009-diagnostics-sql-row-id-drift",
        "lane-mt009-diagnostics-sql-row-id-drift",
        "local",
        2,
    );
    store
        .record_message(sql_row_id_original_msg)
        .await
        .expect("record diagnostics SQL row id original message");
    store
        .record_message(sql_row_id_other_msg)
        .await
        .expect("record diagnostics SQL row id other message");
    sqlx::query(
        r#"
        WITH other AS (
            SELECT event_id, event_sequence, payload->'record' AS record_json
            FROM kernel_event_ledger
            WHERE aggregate_type = 'model_lane_message'
              AND aggregate_id = 'msg-mt009-diagnostics-sql-row-id-other'
        )
        UPDATE model_lane_messages
        SET event_ledger_event_id = other.event_id,
            event_ledger_seq = other.event_sequence,
            event_stream_version = other.event_sequence,
            transaction_seq = other.event_sequence,
            record_json = other.record_json
        FROM other
        WHERE message_id = 'msg-mt009-diagnostics-sql-row-id-original'
        "#,
    )
    .execute(&pool)
    .await
    .expect("redirect mutable row to another valid EventLedger payload");
    let sql_row_id_drift_err = store
        .diagnostics_projection("run-mt009-diagnostics-sql-row-id-drift")
        .await
        .expect_err("diagnostics projection must reject SQL row id drift");
    assert_error_contains(&sql_row_id_drift_err, "SQL row message_id");

    seed_run_lane(
        &store,
        "run-mt009-diagnostics-lease-row-drift",
        "lane-mt009-diagnostics-lease-row-drift",
        RuntimeBinding::Local,
    )
    .await;
    store
        .record_lane_lease(sample_lease(
            "lease-mt009-diagnostics-row-drift",
            "run-mt009-diagnostics-lease-row-drift",
            "lane-mt009-diagnostics-lease-row-drift",
            "2026-07-01T00:05:00Z",
            ModelLaneLeaseState::Active,
        ))
        .await
        .expect("record diagnostics lease row drift");
    sqlx::query(
        r#"
        UPDATE model_lane_leases
        SET record_json = jsonb_set(record_json, '{holder_actor_id}', '"actor://tampered"', false)
        WHERE lease_id = 'lease-mt009-diagnostics-row-drift'
        "#,
    )
    .execute(&pool)
    .await
    .expect("tamper mutable lease diagnostics row");
    let lease_drift_err = store
        .diagnostics_projection("run-mt009-diagnostics-lease-row-drift")
        .await
        .expect_err("diagnostics projection must reject mutable lease row drift");
    assert_error_contains(&lease_drift_err, "model_lane_lease");

    seed_run_lane(
        &store,
        "run-mt009-diagnostics-tier-row-drift",
        "lane-mt009-diagnostics-tier-row-drift",
        RuntimeBinding::Local,
    )
    .await;
    store
        .record_diagnostic_tier_status(sample_tier(
            "run-mt009-diagnostics-tier-row-drift",
            ModelLaneDiagnosticTier::FlightRecorder,
            ModelLaneDiagnosticTierState::Wired,
            "eventledger://kernel/model-lane/mt009/tier-drift",
        ))
        .await
        .expect("record diagnostics tier row drift");
    sqlx::query(
        r#"
        UPDATE model_lane_diagnostic_tier_statuses
        SET record_json = jsonb_set(record_json, '{reason}', '"tampered tier reason"', false)
        WHERE diagnostic_status_id = 'diag-run-mt009-diagnostics-tier-row-drift-HBR-INT-009-flight_recorder'
        "#,
    )
    .execute(&pool)
    .await
    .expect("tamper mutable diagnostic tier row");
    let tier_drift_err = store
        .diagnostics_projection("run-mt009-diagnostics-tier-row-drift")
        .await
        .expect_err("diagnostics projection must reject mutable diagnostic tier row drift");
    assert_error_contains(&tier_drift_err, "model_lane_diagnostic_tier");

    seed_run_lane(
        &store,
        "run-mt009-diagnostics-status-row-drift",
        "lane-mt009-diagnostics-status-row-drift",
        RuntimeBinding::Local,
    )
    .await;
    store
        .record_mt_runtime_status(sample_mt_status(
            "mt-status-mt009-diagnostics-row-drift",
            "run-mt009-diagnostics-status-row-drift",
            ModelLaneMtRuntimeStatus::ProofRunning,
        ))
        .await
        .expect("record diagnostics MT status row drift");
    sqlx::query(
        r#"
        UPDATE model_lane_mt_runtime_statuses
        SET record_json = jsonb_set(record_json, '{proof_status_ref}', '"proof://tampered"', false)
        WHERE mt_status_id = 'mt-status-mt009-diagnostics-row-drift'
        "#,
    )
    .execute(&pool)
    .await
    .expect("tamper mutable MT status row");
    let status_drift_err = store
        .diagnostics_projection("run-mt009-diagnostics-status-row-drift")
        .await
        .expect_err("diagnostics projection must reject mutable MT status row drift");
    assert_error_contains(&status_drift_err, "model_lane_mt_runtime_status");

    seed_run_lane(
        &store,
        "run-mt009-missing-payload",
        "lane-mt009-missing-payload",
        RuntimeBinding::Local,
    )
    .await;
    let missing_payload_msg = sample_message(
        "msg-mt009-missing-payload",
        "run-mt009-missing-payload",
        "lane-mt009-missing-payload",
        "local",
        1,
    );
    store
        .record_message(missing_payload_msg.clone())
        .await
        .expect("record message without ArtifactStore authority");
    record_checkpoint_at_highwater(
        &pool,
        &store,
        "run-mt009-missing-payload",
        "lane-mt009-missing-payload",
        Some("msg-mt009-missing-payload"),
        vec![missing_payload_msg.payload_ref.clone()],
        "checkpoint-mt009-missing-payload",
    )
    .await;
    assert_recovery_failure(
        &store,
        "run-mt009-missing-payload",
        ModelLaneRecoveryFailureKind::MissingPayloadAuthority,
    )
    .await;

    seed_run_lane(
        &store,
        "run-mt009-payload-hash-mismatch",
        "lane-mt009-payload-hash-mismatch",
        RuntimeBinding::Local,
    )
    .await;
    let hash_mismatch_msg = sample_message(
        "msg-mt009-payload-hash-mismatch",
        "run-mt009-payload-hash-mismatch",
        "lane-mt009-payload-hash-mismatch",
        "local",
        1,
    );
    store
        .record_message(hash_mismatch_msg.clone())
        .await
        .expect("record message with declared payload hash");
    let mut mismatched_artifact = sample_artifact_binding_for_message(&hash_mismatch_msg);
    let mismatched_payload_json = json!({
        "message_id": hash_mismatch_msg.message_id,
        "run_id": hash_mismatch_msg.run_id,
        "payload_ref": hash_mismatch_msg.payload_ref,
        "tampered": true,
        "crdt_update_ref": hash_mismatch_msg.crdt_update_ref,
    });
    let mismatched_hash = sha256_hex(&canonical_json_bytes(&mismatched_payload_json));
    mismatched_artifact.artifact_binding_id =
        "artifact-binding-mt009-payload-hash-mismatch-tampered".into();
    mismatched_artifact.artifact_sha256 = mismatched_hash.clone();
    mismatched_artifact.content_hash = mismatched_hash;
    mismatched_artifact.payload_json = mismatched_payload_json;
    mismatched_artifact.idempotency_key =
        "idem-artifact-binding-mt009-payload-hash-mismatch-tampered".into();
    store
        .record_context_bundle_artifact_binding(mismatched_artifact)
        .await
        .expect("record ArtifactStore authority with mismatched content hash");
    record_checkpoint_at_highwater(
        &pool,
        &store,
        "run-mt009-payload-hash-mismatch",
        "lane-mt009-payload-hash-mismatch",
        Some("msg-mt009-payload-hash-mismatch"),
        vec![payload_ref("msg-mt009-payload-hash-mismatch")],
        "checkpoint-mt009-payload-hash-mismatch",
    )
    .await;
    assert_recovery_failure(
        &store,
        "run-mt009-payload-hash-mismatch",
        ModelLaneRecoveryFailureKind::MissingPayloadAuthority,
    )
    .await;

    seed_run_lane(
        &store,
        "run-mt009-stale-crdt",
        "lane-mt009-stale-crdt",
        RuntimeBinding::Local,
    )
    .await;
    let stale_msg = sample_message(
        "msg-mt009-stale-crdt",
        "run-mt009-stale-crdt",
        "lane-mt009-stale-crdt",
        "local",
        1,
    );
    store
        .record_message(stale_msg.clone())
        .await
        .expect("record CRDT message");
    store
        .record_context_bundle_artifact_binding(sample_artifact_binding_for_message(&stale_msg))
        .await
        .expect("record payload authority before stale CRDT failure");
    store
        .record_recovery_event(sample_recovery_event(
            "recovery-event-mt009-stale-crdt",
            "run-mt009-stale-crdt",
            Some("lane-mt009-stale-crdt"),
            ModelLaneRecoveryEventKind::CrdtUpdateObserved,
            1,
            Some(stale_msg.payload_ref.clone()),
            Some("crdt-stale-base://mt009/stale"),
            Some("crdt-snapshot://mt009/base"),
            Some("sv:mt009:3"),
        ))
        .await
        .expect("record stale CRDT recovery event");
    record_checkpoint_at_highwater(
        &pool,
        &store,
        "run-mt009-stale-crdt",
        "lane-mt009-stale-crdt",
        Some("msg-mt009-stale-crdt"),
        vec![stale_msg.payload_ref],
        "checkpoint-mt009-stale-crdt",
    )
    .await;
    assert_recovery_failure(
        &store,
        "run-mt009-stale-crdt",
        ModelLaneRecoveryFailureKind::StaleCrdtBase,
    )
    .await;

    seed_run_lane(
        &store,
        "run-mt009-replay-gap",
        "lane-mt009-replay-gap",
        RuntimeBinding::Local,
    )
    .await;
    let gap_msg = sample_message(
        "msg-mt009-replay-gap",
        "run-mt009-replay-gap",
        "lane-mt009-replay-gap",
        "local",
        1,
    );
    store
        .record_message(gap_msg.clone())
        .await
        .expect("record gap message");
    store
        .record_context_bundle_artifact_binding(sample_artifact_binding_for_message(&gap_msg))
        .await
        .expect("record gap payload authority");
    for (event_id, replay_order_seq) in [
        ("recovery-event-mt009-gap-001", 1_i64),
        ("recovery-event-mt009-gap-003", 3_i64),
    ] {
        store
            .record_recovery_event(sample_recovery_event(
                event_id,
                "run-mt009-replay-gap",
                Some("lane-mt009-replay-gap"),
                ModelLaneRecoveryEventKind::CheckpointRestored,
                replay_order_seq,
                None,
                None,
                Some("crdt-snapshot://mt009/base"),
                Some("sv:mt009:3"),
            ))
            .await
            .expect("record replay gap recovery event");
    }
    record_checkpoint_at_highwater(
        &pool,
        &store,
        "run-mt009-replay-gap",
        "lane-mt009-replay-gap",
        Some("msg-mt009-replay-gap"),
        vec![gap_msg.payload_ref],
        "checkpoint-mt009-replay-gap",
    )
    .await;
    assert_recovery_failure(
        &store,
        "run-mt009-replay-gap",
        ModelLaneRecoveryFailureKind::EventLedgerSequenceGap,
    )
    .await;

    seed_run_lane(
        &store,
        "run-mt009-hbr-complete-other",
        "lane-mt009-hbr-complete-other",
        RuntimeBinding::Local,
    )
    .await;
    for (tier, state, evidence) in [
        (
            ModelLaneDiagnosticTier::FlightRecorder,
            ModelLaneDiagnosticTierState::Wired,
            "eventledger://kernel/model-lane/mt009/other",
        ),
        (
            ModelLaneDiagnosticTier::InternalDiagnostics,
            ModelLaneDiagnosticTierState::Wired,
            "hbr-int-009://dexterity/mixed-runtime/other",
        ),
        (
            ModelLaneDiagnosticTier::Palmistry,
            ModelLaneDiagnosticTierState::DeferredWithReason,
            "palmistry://wp1/model-lane/mt009/other",
        ),
    ] {
        store
            .record_diagnostic_tier_status(sample_tier(
                "run-mt009-hbr-complete-other",
                tier,
                state,
                evidence,
            ))
            .await
            .expect("record unrelated full HBR posture");
    }
    seed_run_lane(
        &store,
        "run-mt009-fr-only",
        "lane-mt009-fr-only",
        RuntimeBinding::Local,
    )
    .await;
    let fr_msg = sample_message(
        "msg-mt009-fr-only",
        "run-mt009-fr-only",
        "lane-mt009-fr-only",
        "local",
        1,
    );
    store
        .record_message(fr_msg)
        .await
        .expect("record FR-only diagnostic message");
    store
        .record_diagnostic_tier_status(sample_tier(
            "run-mt009-fr-only",
            ModelLaneDiagnosticTier::FlightRecorder,
            ModelLaneDiagnosticTierState::Wired,
            "eventledger://kernel/model-lane/mt009/fr-only",
        ))
        .await
        .expect("record only FlightRecorder tier");
    let fr_only_err = store
        .diagnostics_projection("run-mt009-fr-only")
        .await
        .expect_err("FlightRecorder-only HBR posture must not project to native Argus");
    let fr_only_message = fr_only_err.to_string();
    assert!(
        fr_only_message.contains("FlightRecorder-only")
            || fr_only_message.contains("internal_diagnostics"),
        "expected FlightRecorder-only HBR failure, got {fr_only_err}"
    );
}

/// Deterministic production-shaped runtime used only to drive the real
/// `GenerateRequest` cancellation boundary. PostgreSQL/EventLedger persistence
/// is never mocked in the MT-009 tests.
struct CancellationProbeRuntime {
    capabilities: ModelCapabilities,
    kv: KvCacheHandle,
    lora: LoraStackHandle,
    steering: SteeringHookHandle,
}

impl CancellationProbeRuntime {
    fn new() -> Self {
        Self {
            capabilities: ModelCapabilities::default(),
            kv: KvCacheHandle::new("mt009-cancel-kv"),
            lora: LoraStackHandle::new("mt009-cancel-lora"),
            steering: SteeringHookHandle::new("mt009-cancel-steering"),
        }
    }
}

#[async_trait]
impl ModelRuntime for CancellationProbeRuntime {
    async fn load(&mut self, _spec: LoadSpec) -> Result<ModelId, ModelRuntimeError> {
        Ok(ModelId::new_v7())
    }

    async fn unload(&mut self, _id: ModelId) -> Result<(), ModelRuntimeError> {
        Ok(())
    }

    fn generate(&self, req: GenerateRequest) -> TokenStream {
        let cancellation = req.cancel;
        Box::pin(stream::unfold(
            (0_u8, cancellation),
            |(phase, cancellation)| async move {
                match phase {
                    0 => Some((
                        Ok(GeneratedToken {
                            token_id: 0,
                            text: "mt009-prefix".into(),
                            logprob: None,
                            finish_reason: None,
                        }),
                        (1, cancellation),
                    )),
                    1 if cancellation.is_cancelled() => {
                        Some((Err(ModelRuntimeError::Cancelled), (2, cancellation)))
                    }
                    1 => Some((
                        Ok(GeneratedToken {
                            token_id: 1,
                            text: "mt009-late-token".into(),
                            logprob: None,
                            finish_reason: None,
                        }),
                        (2, cancellation),
                    )),
                    _ => None,
                }
            },
        ))
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

fn cancellation_probe_request(cancel: CancellationToken) -> GenerateRequest {
    GenerateRequest {
        id: ModelId::new_v7(),
        prompt: GenPrompt::from("MT-009 cancellation boundary"),
        sampling: SamplingParams::default(),
        lora_overrides: Vec::new(),
        steering_overrides: Vec::new(),
        kv_prefix_handle: None,
        cancel,
        max_tokens: 2,
        stop_sequences: Vec::new(),
        speculative_mode: None,
        structured_decoding: None,
    }
}

#[allow(clippy::too_many_arguments)]
const MT009_YJS_TEXT_NAME: &str = "mt009-shared-document";

/// Generate a real Yjs v1 incremental update from a distinct author replica,
/// apply it to the canonical authoring document, and return only the binary
/// update that crossed the persistence boundary. This keeps the test's payload
/// path identical to a real Yjs client rather than substituting label bytes.
fn mt009_append_yjs_text_update(canonical: &Doc, client_id: u64, text: &str) -> Vec<u8> {
    let canonical_state = canonical
        .transact()
        .encode_state_as_update_v1(&StateVector::default());
    let author = Doc::with_client_id(client_id);
    let author_text = author.get_or_insert_text(MT009_YJS_TEXT_NAME);
    if !canonical_state.is_empty() {
        author
            .transact_mut()
            .apply_update(Update::decode_v1(&canonical_state).expect("decode canonical Yjs state"))
            .expect("apply canonical Yjs state to author replica");
    }

    let before = author.transact().state_vector();
    {
        let mut transaction = author.transact_mut();
        let offset = author_text.len(&transaction);
        author_text.insert(&mut transaction, offset, text);
    }
    let update = author.transact().encode_diff_v1(&before);
    canonical
        .transact_mut()
        .apply_update(Update::decode_v1(&update).expect("decode generated Yjs update"))
        .expect("apply generated Yjs update to canonical replica");
    update
}

fn mt009_yjs_materialize_doc(doc: &Doc) -> (String, String) {
    let text = doc.get_or_insert_text(MT009_YJS_TEXT_NAME);
    let transaction = doc.transact();
    (
        text.get_string(&transaction),
        base64::engine::general_purpose::STANDARD.encode(transaction.state_vector().encode_v1()),
    )
}

/// Apply persisted Yjs bytes exactly as returned by PostgreSQL. The assertion
/// helper intentionally does not read ModelLane diagnostic metadata, so a
/// bogus label cannot make a corrupt update look materialized.
fn mt009_yjs_materialize_updates(updates: &[Vec<u8>]) -> (String, String) {
    let document = Doc::new();
    for update_bytes in updates {
        document
            .transact_mut()
            .apply_update(Update::decode_v1(update_bytes).expect("decode persisted Yjs update"))
            .expect("apply persisted Yjs update");
    }
    mt009_yjs_materialize_doc(&document)
}

async fn mt009_push_yjs_update(
    db: &(dyn Database + '_),
    workspace_id: &str,
    document_id: &str,
    crdt_document_id: &str,
    document_schema_id: &str,
    update_id: &str,
    actor: &KnowledgeActorIdV1,
    site_id: &str,
    session_id: &str,
    update_bytes: &[u8],
    state_vector: &mut KnowledgeStateVectorV1,
    expected_seq: u64,
) -> YjsUpdateEnvelopeV1 {
    let before = state_vector.clone();
    state_vector.increment(site_id);
    let envelope = mt009_yjs_envelope(
        workspace_id,
        document_id,
        crdt_document_id,
        document_schema_id,
        update_id,
        actor,
        session_id,
        update_bytes,
        &before,
        state_vector,
    );
    match push_yjs_update(db, &envelope)
        .await
        .expect("store real Yjs update in PostgreSQL/EventLedger")
    {
        YjsPushOutcomeV1::Stored { update_seq, .. } => {
            assert_eq!(update_seq, expected_seq, "Yjs updates must be sequenced")
        }
        other => panic!("expected stored Yjs update, got {other:?}"),
    }
    envelope
}

#[allow(clippy::too_many_arguments)]
fn mt009_yjs_envelope(
    workspace_id: &str,
    document_id: &str,
    crdt_document_id: &str,
    document_schema_id: &str,
    update_id: &str,
    actor: &KnowledgeActorIdV1,
    session_id: &str,
    update_bytes: &[u8],
    before: &KnowledgeStateVectorV1,
    after: &KnowledgeStateVectorV1,
) -> YjsUpdateEnvelopeV1 {
    let site = derive_knowledge_site_id(workspace_id, crdt_document_id, actor);
    YjsUpdateEnvelopeV1 {
        schema_id: YJS_UPDATE_ENVELOPE_SCHEMA_ID.to_string(),
        workspace_id: workspace_id.to_string(),
        document_id: document_id.to_string(),
        crdt_document_id: crdt_document_id.to_string(),
        update_id: update_id.to_string(),
        actor_id: actor.canonical(),
        site_id: site.site_id,
        session_id: session_id.to_string(),
        trace_id: format!("trace-{update_id}"),
        document_schema_id: document_schema_id.to_string(),
        update_b64: base64::engine::general_purpose::STANDARD.encode(update_bytes),
        update_sha256: sha256_hex(update_bytes),
        state_vector_before: before.encode(),
        state_vector_after: after.encode(),
        encoding: YJS_UPDATE_ENCODING_V1.to_string(),
    }
}

async fn model_lane_store() -> (PgPool, ModelLaneStore) {
    let Some(kpg) = knowledge_pg_support::knowledge_pg().await else {
        panic!("PostgreSQL/EventLedger is required for MT-009 mixed ModelLane proof");
    };
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&kpg.schema_url)
        .await
        .expect("connect isolated Dexterity mixed ModelLane schema");
    let store = ModelLaneStore::new(pool.clone());
    (pool, store)
}

async fn seed_cloud_authority(store: &ModelLaneStore, run_id: &str, lane_id: &str) {
    let plan = store
        .record_cloud_projection_plan(sample_projection_plan(run_id, lane_id))
        .await
        .expect("record cloud ProjectionPlan authority");
    store
        .record_cloud_consent_receipt(sample_consent_receipt(
            run_id,
            lane_id,
            &plan.projection_plan_id,
            &plan.projection_plan_hash,
        ))
        .await
        .expect("record cloud ConsentReceipt authority");
}

async fn seed_run_lane(
    store: &ModelLaneStore,
    run_id: &str,
    lane_id: &str,
    runtime_binding: RuntimeBinding,
) {
    let (kind, launch_authority) = match runtime_binding {
        RuntimeBinding::Local => (ModelLaneKind::LocalModel, LaunchAuthority::ModelRuntime),
        RuntimeBinding::Cloud => (ModelLaneKind::CloudModel, LaunchAuthority::CloudLane),
        RuntimeBinding::Subagent => (ModelLaneKind::Subagent, LaunchAuthority::SubagentManager),
        RuntimeBinding::CliBridge => (ModelLaneKind::CliModel, LaunchAuthority::CliBridge),
        RuntimeBinding::Human => (ModelLaneKind::HumanOperator, LaunchAuthority::Operator),
        RuntimeBinding::Validator => (ModelLaneKind::Validator, LaunchAuthority::ValidatorRunner),
    };
    store
        .record_run(sample_run(run_id, vec![lane_id.to_owned()]))
        .await
        .expect("record negative-path run");
    store
        .record_lane(sample_lane(
            lane_id,
            run_id,
            kind,
            runtime_binding,
            launch_authority,
        ))
        .await
        .expect("record negative-path lane");
}

async fn record_checkpoint_at_highwater(
    pool: &PgPool,
    store: &ModelLaneStore,
    run_id: &str,
    lane_id: &str,
    last_message_id: Option<&str>,
    open_payload_refs: Vec<String>,
    checkpoint_id: &str,
) {
    let highwater = event_stream_high_watermark(pool, &event_stream_id(run_id)).await;
    store
        .record_recovery_checkpoint(sample_checkpoint(
            checkpoint_id,
            run_id,
            Some(lane_id),
            last_message_id,
            None,
            highwater,
            open_payload_refs,
        ))
        .await
        .expect("record checkpoint at EventLedger high-watermark");
}

async fn assert_recovery_failure(
    store: &ModelLaneStore,
    run_id: &str,
    failure: ModelLaneRecoveryFailureKind,
) {
    let err = store
        .recover_run_after_restart(run_id)
        .await
        .expect_err("recovery must fail closed");
    assert!(
        err.to_string().contains(failure.code()),
        "expected {}, got {err}",
        failure.code()
    );
}

async fn assert_no_lane_row(pool: &PgPool, lane_id: &str) {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM model_lanes WHERE lane_id = $1")
        .bind(lane_id)
        .fetch_one(pool)
        .await
        .expect("count denied lane rows");
    assert_eq!(count, 0, "denied cloud launch must not create lane row");
}

async fn assert_denial_event(pool: &PgPool, run_id: &str, lane_id: &str) {
    let payload: Value = sqlx::query_scalar(
        "SELECT payload FROM kernel_event_ledger \
         WHERE aggregate_type = 'model_lane_cloud_consent_denial' \
           AND aggregate_id = $1 \
         ORDER BY event_sequence DESC LIMIT 1",
    )
    .bind(lane_id)
    .fetch_one(pool)
    .await
    .expect("cloud consent denial EventLedger row");
    assert_eq!(payload["reason_code"], json!("CX-MM-007"));
    assert_eq!(payload["run_id"], json!(run_id));
    assert_eq!(payload["provider_call_attempted"], json!(false));
    assert_eq!(payload["partial_authority_state_created"], json!(false));
}

async fn assert_no_message_row(pool: &PgPool, message_id: &str) {
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM model_lane_messages WHERE message_id = $1")
            .bind(message_id)
            .fetch_one(pool)
            .await
            .expect("count message rows");
    assert_eq!(
        count, 0,
        "message {message_id} should not be persisted after failed authority check"
    );
}

async fn assert_no_lease_row(pool: &PgPool, lease_id: &str) {
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM model_lane_leases WHERE lease_id = $1")
            .bind(lease_id)
            .fetch_one(pool)
            .await
            .expect("count lease rows");
    assert_eq!(
        count, 0,
        "lease {lease_id} should not be persisted after failed authority check"
    );
}

async fn assert_no_mt_status_row(pool: &PgPool, mt_status_id: &str) {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM model_lane_mt_runtime_statuses WHERE mt_status_id = $1",
    )
    .bind(mt_status_id)
    .fetch_one(pool)
    .await
    .expect("count MT runtime status rows");
    assert_eq!(
        count, 0,
        "MT runtime status {mt_status_id} should not be persisted after failed authority check"
    );
}

async fn event_stream_high_watermark(pool: &PgPool, event_ledger_stream_id: &str) -> i64 {
    sqlx::query_scalar(
        "SELECT COALESCE(MAX(event_sequence), 0) \
         FROM kernel_event_ledger \
         WHERE session_run_id = $1",
    )
    .bind(event_ledger_stream_id)
    .fetch_one(pool)
    .await
    .expect("query EventLedger stream high-watermark")
}

fn sample_run(run_id: &str, lane_ids: Vec<String>) -> NewModelLaneRun {
    NewModelLaneRun {
        run_id: run_id.into(),
        trace_id: format!("trace-{run_id}"),
        run_span_id: format!("span-{run_id}"),
        coordinator_session_id: format!("coordinator-{run_id}"),
        routing_policy: "mixed_local_cloud_subagent".into(),
        context_bundle_id: format!("ctx-{run_id}"),
        lane_ids,
        event_ledger_stream_id: event_stream_id(run_id),
        artifact_namespace: format!("artifact://model-lane/mt009/{run_id}"),
        projection_plan_ref: None,
        consent_receipt_ref: None,
        work_packet_id: Some(WP_ID.into()),
        micro_task_id: Some(MT_ID.into()),
        task_board_id: Some(TASK_BOARD_ID.into()),
        owner_session: OWNER.into(),
        idempotency_key: format!("idem-run-{run_id}"),
        replay_order_key: format!("00000000/{run_id}/run"),
        replay_after_event_ledger_seq: None,
        recovery_state: ModelLaneRecoveryState::Restartable,
        failstate_code: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane-validation-harness#recovery".into()),
        locus_binding: Some(sample_locus(
            run_id,
            &format!("coordinator-{run_id}"),
            &format!("model-session-coordinator-{run_id}"),
        )),
        memory_pack_ref: format!("memory-pack://fems/mt009/{run_id}"),
        memory_pack_hash: sample_sha256(),
        determinism_mode: "deterministic_replay".into(),
        budget_summary_ref: "budget://mt009/mixed-runtime".into(),
        selected_model_id: Some("model://mt009/local/tinyllama".into()),
        candidate_model_ids: vec![
            "model://mt009/local/tinyllama".into(),
            "model://mt009/cloud/openai/gpt-4o-mini".into(),
            "subagent://mt009/coder".into(),
        ],
        procedural_review_status: "reviewed_by_kernel_builder".into(),
        truncation_warning_ref: None,
        rejection_reason_refs: vec![],
    }
}

fn sample_lane(
    lane_id: &str,
    run_id: &str,
    kind: ModelLaneKind,
    runtime_binding: RuntimeBinding,
    launch_authority: LaunchAuthority,
) -> NewModelLane {
    let provider_kind = match runtime_binding {
        RuntimeBinding::Local => ModelLaneProviderKind::LocalRuntime,
        RuntimeBinding::Cloud => ModelLaneProviderKind::OpenAi,
        RuntimeBinding::CliBridge => ModelLaneProviderKind::OfficialCli,
        RuntimeBinding::Human => ModelLaneProviderKind::Human,
        RuntimeBinding::Subagent => ModelLaneProviderKind::Subagent,
        RuntimeBinding::Validator => ModelLaneProviderKind::Validator,
    };
    let model_id = match runtime_binding {
        RuntimeBinding::Local => "model://mt009/local/tinyllama",
        RuntimeBinding::Cloud => "model://mt009/cloud/openai/gpt-4o-mini",
        RuntimeBinding::Subagent => "subagent://mt009/coder",
        RuntimeBinding::CliBridge => "model://mt009/cli",
        RuntimeBinding::Human => "operator://mt009/human",
        RuntimeBinding::Validator => "validator://mt009",
    };
    let process_backed = matches!(
        runtime_binding,
        RuntimeBinding::Local | RuntimeBinding::Cloud | RuntimeBinding::CliBridge
    );
    NewModelLane {
        lane_id: lane_id.into(),
        run_id: run_id.into(),
        trace_id: format!("trace-{run_id}"),
        lane_span_id: format!("span-{lane_id}"),
        event_ledger_stream_id: event_stream_id(run_id),
        kind,
        role: match runtime_binding {
            RuntimeBinding::Local => "local_implementer",
            RuntimeBinding::Cloud => "cloud_reviewer",
            RuntimeBinding::Subagent => "subagent_coder",
            RuntimeBinding::CliBridge => "cli_bridge",
            RuntimeBinding::Human => "operator",
            RuntimeBinding::Validator => "validator",
        }
        .into(),
        backend: runtime_binding.as_str().into(),
        model_id: Some(model_id.into()),
        session_id: format!("session-{lane_id}"),
        model_session_id: format!("model-session-{lane_id}"),
        adapter_id: match runtime_binding {
            RuntimeBinding::Local => "local-runtime",
            RuntimeBinding::Cloud => "openai-byok",
            RuntimeBinding::Subagent => "subagent-manager",
            RuntimeBinding::CliBridge => "official-cli",
            RuntimeBinding::Human => "operator-console",
            RuntimeBinding::Validator => "validator-runner",
        }
        .into(),
        runtime_binding: runtime_binding.clone(),
        launch_authority,
        provider_kind,
        capability_token_ids: vec![format!("capability://mt009/{lane_id}/read")],
        effective_capability_snapshot_ref: Some(format!("capability-snapshot://mt009/{lane_id}")),
        capability_negotiation_ref: Some(format!("capability-negotiation://mt009/{lane_id}")),
        provider_feature_profile_ref: Some(format!("provider-feature-profile://mt009/{lane_id}")),
        requested_execution_policy_ref: Some(format!(
            "execution-policy://requested/mt009/{lane_id}"
        )),
        effective_execution_policy_ref: Some(format!(
            "execution-policy://effective/mt009/{lane_id}"
        )),
        projection_plan_ref: (runtime_binding == RuntimeBinding::Cloud)
            .then(|| projection_plan_id(run_id, lane_id)),
        consent_receipt_ref: (runtime_binding == RuntimeBinding::Cloud)
            .then(|| consent_receipt_id(run_id, lane_id)),
        tool_gate_decision_refs: vec![format!("toolgate://mt009/{lane_id}/allow")],
        status: ModelLaneStatus::Ready,
        recovery_state: ModelLaneRecoveryState::Restartable,
        heartbeat_at_utc: Some("2026-07-01T00:00:00Z".into()),
        lease_expires_at_utc: Some("2099-01-01T00:00:00Z".into()),
        reclaim_after_utc: Some("2099-01-01T00:01:00Z".into()),
        restart_generation: 0,
        cancellation_ref: Some(format!("cancel-token://mt009/{lane_id}")),
        reclaim_policy_ref: Some("reclaim-policy://mt009/mixed-runtime".into()),
        terminal_status_mapping_ref: Some("terminal-status://mt009/mixed-runtime".into()),
        process_ownership_ref: process_backed
            .then(|| format!("process-ledger://{}", process_uuid_for_lane(lane_id))),
        no_os_process_reason_ref: (!process_backed)
            .then(|| format!("no-os-process://subagent_manager/{lane_id}")),
        backpressure_ref: None,
        loop_counter_ref: Some("loop-counter://mt009/mixed-runtime".into()),
        last_runtime_status_ref: Some(format!("runtime-status://mt009/{lane_id}/ready")),
        last_recovery_event_ref: Some(format!("recovery://mt009/{lane_id}/startable")),
        failstate_code: None,
        startup_failure_ref: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane-validation-harness#lane".into()),
        work_packet_id: Some(WP_ID.into()),
        micro_task_id: Some(MT_ID.into()),
        task_board_id: Some(TASK_BOARD_ID.into()),
        owner_session: OWNER.into(),
        locus_binding: Some(sample_locus(
            run_id,
            &format!("session-{lane_id}"),
            &format!("model-session-{lane_id}"),
        )),
    }
}

fn sample_message(
    message_id: &str,
    run_id: &str,
    lane_id: &str,
    lane_label: &str,
    replay_seq: i64,
) -> NewModelLaneMessage {
    let (crdt_key, crdt_value) = match lane_label {
        "cloud" => ("mt009.cloud", "cloud advisory review"),
        "subagent" => ("mt009.subagent", "subagent implementation note"),
        _ => ("mt009.local", "local proposed deterministic edit"),
    };
    let payload_ref = payload_ref(message_id);
    let crdt_update_ref = format!("crdt-update://mt009/{message_id}");
    let locus_ref = format!("locus://wp1/mt009/{run_id}/{lane_id}/{message_id}");
    let payload_json = artifact_payload_json_parts(
        message_id,
        run_id,
        &payload_ref,
        &crdt_update_ref,
        &locus_ref,
    );
    let payload_sha256 = sha256_hex(&canonical_json_bytes(&payload_json));
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
        payload_ref,
        payload_sha256,
        event_ledger_stream_id: event_stream_id(run_id),
        summary: format!("MT-009 mixed runtime payload from {lane_label}"),
        authority: ModelLaneAuthority::PromotionCandidate,
        promotion_decision_id: Some(format!("promotion://mt009/{message_id}")),
        promotion_gate_ref: Some(format!("promotion-gate://mt009/{message_id}")),
        promotion_receipt_ref: Some(format!("promotion-receipt://mt009/{message_id}")),
        validator_verdict_ref: None,
        operator_decision_ref: None,
        promoted_artifact_ref: Some(format!("artifact://promoted/mt009/{message_id}")),
        promoted_artifact_sha256: Some(sample_sha256()),
        promoted_artifact_version: Some("1".into()),
        tool_gate_decision_refs: vec![format!("toolgate://mt009/{lane_id}/allow")],
        coordinator_session_id: format!("coordinator-{run_id}"),
        work_packet_id: Some(WP_ID.into()),
        micro_task_id: Some(MT_ID.into()),
        task_board_id: Some(TASK_BOARD_ID.into()),
        owner_session: OWNER.into(),
        locus_binding: Some(sample_locus(
            run_id,
            &format!("session-{lane_id}"),
            &format!("model-session-{lane_id}"),
        )),
        idempotency_key: format!("idem-message-{message_id}"),
        replay_order_key: format!("{replay_seq:08}/message/{message_id}"),
        replay_after_event_ledger_seq: Some(1),
        proposal_ref: Some(format!("proposal://mt009/{message_id}")),
        crdt_update_ref: Some(crdt_update_ref),
        crdt_base_snapshot_ref: Some("crdt-snapshot://mt009/base".into()),
        crdt_state_vector: Some("sv:mt009:3".into()),
        crdt_proposal_ref: Some(format!("crdt-proposal://mt009/{message_id}")),
        crdt_stale_base_ref: None,
        failstate_code: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane-validation-harness#message".into()),
        created_at_utc: "2026-07-01T00:00:00Z".into(),
        diagnostic_payload: json!({
            "artifact_ref": format!("artifact://model-lane/messages/{message_id}"),
            "crdt_update_id": format!("crdt-update-id://mt009/{message_id}"),
            "crdt_key": crdt_key,
            "crdt_value": crdt_value,
            "locus_ref": locus_ref,
            "loom_ref": format!("loom://mt009/{run_id}/{message_id}"),
            "fems_ref": format!("fems://mt009/{run_id}/{message_id}"),
            "flight_recorder": "kernel_event_ledger",
            "internal_diagnostics": "hbr-int-009",
            "palmistry": "deferred_external_worktree"
        }),
    }
}

fn sample_recovery_event(
    event_id: &str,
    run_id: &str,
    lane_id: Option<&str>,
    kind: ModelLaneRecoveryEventKind,
    replay_order_seq: i64,
    payload_ref: Option<String>,
    crdt_stale_base_ref: Option<&str>,
    crdt_base_snapshot_ref: Option<&str>,
    crdt_state_vector: Option<&str>,
) -> NewModelLaneRecoveryEvent {
    NewModelLaneRecoveryEvent {
        recovery_event_id: event_id.into(),
        run_id: run_id.into(),
        lane_id: lane_id.map(str::to_string),
        trace_id: format!("trace-{run_id}"),
        span_id: format!("span-{event_id}"),
        parent_span_id: lane_id.map(|lane| format!("span-{lane}")),
        linked_span_contexts: vec![format!("trace-link://{run_id}/{event_id}")],
        session_id: lane_id.map(|lane| format!("session-{lane}")),
        model_session_id: lane_id.map(|lane| format!("model-session-{lane}")),
        event_kind: kind,
        recovery_status: ModelLaneRecoveryStatus::Observed,
        replay_order_seq,
        source_event_ledger_seq: None,
        payload_refs: payload_ref.into_iter().collect(),
        artifact_refs: vec![],
        crdt_base_snapshot_ref: crdt_base_snapshot_ref.map(str::to_string),
        crdt_state_vector: crdt_state_vector.map(str::to_string),
        crdt_stale_base_ref: crdt_stale_base_ref.map(str::to_string),
        lease_id: None,
        failure_kind: None,
        error_code: None,
        replay_hint:
            "Replay mixed ModelLane state from PostgreSQL/EventLedger before runtime memory".into(),
        event_ledger_stream_id: event_stream_id(run_id),
        work_packet_id: WP_ID.into(),
        micro_task_id: MT_ID.into(),
        task_board_id: TASK_BOARD_ID.into(),
        owner_session: OWNER.into(),
        idempotency_key: format!("idem-recovery-{event_id}"),
        recovery_hint_ref: Some("usermanual://model-lane-validation-harness#recovery-event".into()),
        diagnostic_payload: json!({"tier": "internal_diagnostics", "mt": MT_ID}),
    }
}

fn sample_checkpoint(
    checkpoint_id: &str,
    run_id: &str,
    lane_id: Option<&str>,
    last_message_id: Option<&str>,
    lease_id: Option<&str>,
    last_event_ledger_seq: i64,
    open_payload_refs: Vec<String>,
) -> NewModelLaneRecoveryCheckpoint {
    NewModelLaneRecoveryCheckpoint {
        checkpoint_id: checkpoint_id.into(),
        run_id: run_id.into(),
        lane_id: lane_id.map(str::to_string),
        session_id: lane_id
            .map(|lane| format!("session-{lane}"))
            .unwrap_or_else(|| format!("coordinator-{run_id}")),
        model_session_id: lane_id
            .map(|lane| format!("model-session-{lane}"))
            .unwrap_or_else(|| format!("model-session-coordinator-{run_id}")),
        lane_status: ModelLaneStatus::Ready,
        checkpoint_status: ModelLaneRecoveryStatus::Checkpointed,
        last_event_ledger_seq,
        last_message_id: last_message_id.map(str::to_string),
        open_payload_refs,
        lease_id: lease_id.map(str::to_string),
        idempotency_scope: format!("model-lane-mixed:{run_id}:{checkpoint_id}"),
        recovery_state: ModelLaneRecoveryState::Restartable,
        recovery_event_ref: Some(format!("recovery-event://{checkpoint_id}")),
        event_ledger_stream_id: event_stream_id(run_id),
        work_packet_id: WP_ID.into(),
        micro_task_id: MT_ID.into(),
        task_board_id: TASK_BOARD_ID.into(),
        owner_session: OWNER.into(),
        idempotency_key: format!("idem-checkpoint-{checkpoint_id}"),
        created_at_utc: "2026-07-01T00:00:01Z".into(),
        recovery_hint_ref: Some("usermanual://model-lane-validation-harness#checkpoint".into()),
        diagnostic_payload: json!({"tier": "internal_diagnostics", "mt": MT_ID}),
    }
}

fn sample_lease(
    lease_id: &str,
    run_id: &str,
    lane_id: &str,
    lease_expires_at_utc: &str,
    state: ModelLaneLeaseState,
) -> NewModelLaneLease {
    NewModelLaneLease {
        lease_id: lease_id.into(),
        run_id: run_id.into(),
        lane_id: Some(lane_id.into()),
        scope: ModelLaneLeaseScope::Lane,
        scope_ref: format!("model-lane://{run_id}/{lane_id}"),
        holder_actor_id: "actor://kernel-builder/mt009".into(),
        holder_session_id: OWNER.into(),
        lease_expires_at_utc: lease_expires_at_utc.into(),
        takeover_policy_ref: "lease-policy://mt009/recover-or-reclaim".into(),
        state,
        event_ledger_stream_id: event_stream_id(run_id),
        work_packet_id: WP_ID.into(),
        micro_task_id: MT_ID.into(),
        task_board_id: TASK_BOARD_ID.into(),
        owner_session: OWNER.into(),
        idempotency_key: format!("idem-lease-{lease_id}"),
        recovery_hint_ref: Some("usermanual://model-lane-validation-harness#lease".into()),
        diagnostic_payload: json!({"tier": "internal_diagnostics", "mt": MT_ID}),
    }
}

fn sample_tier(
    run_id: &str,
    tier: ModelLaneDiagnosticTier,
    state: ModelLaneDiagnosticTierState,
    evidence_ref: &str,
) -> NewModelLaneDiagnosticTierStatus {
    NewModelLaneDiagnosticTierStatus {
        diagnostic_status_id: format!("diag-{run_id}-HBR-INT-009-{}", tier.as_str()),
        behavior_id: "HBR-INT-009".into(),
        run_id: run_id.into(),
        tier,
        state,
        reason: format!("MT-009 mixed-runtime diagnostic posture for {run_id}"),
        evidence_ref: evidence_ref.into(),
        follow_up_ref: Some("palmistry://wp1/model-lane/mt009".into()),
        event_ledger_stream_id: event_stream_id(run_id),
        work_packet_id: WP_ID.into(),
        micro_task_id: MT_ID.into(),
        task_board_id: TASK_BOARD_ID.into(),
        owner_session: OWNER.into(),
        idempotency_key: format!("idem-diag-{run_id}-HBR-INT-009-{}", tier.as_str()),
        diagnostic_payload: json!({"behavior_id": "HBR-INT-009", "run_id": run_id, "mt": MT_ID}),
    }
}

fn sample_mt_status(
    status_id: &str,
    run_id: &str,
    status: ModelLaneMtRuntimeStatus,
) -> NewModelLaneMtRuntimeStatus {
    NewModelLaneMtRuntimeStatus {
        mt_status_id: status_id.into(),
        run_id: run_id.into(),
        work_packet_id: WP_ID.into(),
        micro_task_id: MT_ID.into(),
        task_board_id: TASK_BOARD_ID.into(),
        status,
        claimed_by_ref: Some(format!("session://{OWNER}")),
        blocker_ref: None,
        missing_resource_ref: None,
        proof_status_ref: Some("proof://mt009/mixed_model_lane_integration_pg_tests".into()),
        hbr_status_ref: Some("hbr-int-009://dexterity/mixed-runtime".into()),
        last_recovery_event_ref: Some("recovery-event://recovery-event-mt009-crdt-001".into()),
        last_runtime_status_ref: Some("runtime-status://mt009/ready-for-validation".into()),
        event_ledger_stream_id: event_stream_id(run_id),
        owner_session: OWNER.into(),
        idempotency_key: format!("idem-mt-status-{status_id}"),
        diagnostic_payload: json!({"mixed_runtime": true, "mt": MT_ID}),
    }
}

fn sample_artifact_binding_for_message(
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
            "artifact-store://model-lane/mt009/{}/artifact.json",
            message.message_id
        ),
        artifact_payload_ref: message.payload_ref.clone(),
        payload_json: artifact_payload_json_for_message(message),
        event_ledger_stream_id: message.event_ledger_stream_id.clone(),
        work_packet_id: WP_ID.into(),
        micro_task_id: MT_ID.into(),
        task_board_id: TASK_BOARD_ID.into(),
        owner_session: OWNER.into(),
        idempotency_key: format!("idem-artifact-binding-{}", message.message_id),
        created_at_utc: "2026-07-01T00:00:02Z".into(),
        diagnostic_payload: json!({
            "flight_recorder": "ArtifactStore/EventLedger binding for MT-009 mixed runtime",
            "internal_diagnostics": "hbr-int-009",
            "palmistry": "deferred_external_worktree"
        }),
    }
}

fn sample_projection_plan(run_id: &str, lane_id: &str) -> NewModelLaneCloudProjectionPlan {
    NewModelLaneCloudProjectionPlan {
        projection_plan_id: projection_plan_id(run_id, lane_id),
        run_id: run_id.into(),
        trace_id: format!("trace-{run_id}"),
        lane_id: lane_id.into(),
        model_session_id: format!("model-session-{lane_id}"),
        provider_kind: "openai".into(),
        requested_model_id: "model://mt009/cloud/openai/gpt-4o-mini".into(),
        scope_hash: sample_scope_hash(),
        source_artifact_refs: vec![
            format!("artifact-store://mt009/{run_id}/{lane_id}/context.json"),
            "context-bundle://mt009/cloud-safe".into(),
        ],
        payload_artifact_ref: format!("artifact-store://mt009/{run_id}/{lane_id}/payload.json"),
        payload_sha256: sample_sha256(),
        redaction_policy_ref: "redaction-policy://mt009/cloud-safe".into(),
        redaction_summary: "workspace-local secrets and local-only memory are excluded".into(),
        retention_policy: ModelLaneCloudRetentionPolicy::NoTrainingEphemeral,
        export_posture: ModelLaneCloudExportPosture::RedactedContextOnly,
        provider_profile_ref: "provider-profile://mt009/openai".into(),
        fan_out_targets: vec!["provider://openai/byok".into()],
        consent_scope: ModelLaneCloudConsentScope::SingleLane,
        status: ModelLaneCloudProjectionPlanStatus::Active,
        event_ledger_stream_id: event_stream_id(run_id),
        work_packet_id: WP_ID.into(),
        micro_task_id: MT_ID.into(),
        task_board_id: TASK_BOARD_ID.into(),
        owner_session: OWNER.into(),
        idempotency_key: format!("idem-projection-{run_id}-{lane_id}"),
        created_at_utc: "2026-07-01T00:00:00Z".into(),
        user_manual_behavior_ref: USERMANUAL_BEHAVIOR.into(),
        diagnostic_payload: json!({
            "flight_recorder": "EventLedger",
            "internal_diagnostics": "hbr-int-009",
            "palmistry": "deferred_external_worktree",
            "locus": format!("locus://wp1/mt009/{run_id}/{lane_id}")
        }),
    }
}

fn sample_consent_receipt(
    run_id: &str,
    lane_id: &str,
    projection_plan_id: &str,
    projection_plan_hash: &str,
) -> NewModelLaneCloudConsentReceipt {
    NewModelLaneCloudConsentReceipt {
        consent_receipt_id: consent_receipt_id(run_id, lane_id),
        projection_plan_id: projection_plan_id.into(),
        projection_plan_hash: projection_plan_hash.into(),
        run_id: run_id.into(),
        trace_id: format!("trace-{run_id}"),
        lane_id: lane_id.into(),
        model_session_id: format!("model-session-{lane_id}"),
        provider_kind: "openai".into(),
        requested_model_id: "model://mt009/cloud/openai/gpt-4o-mini".into(),
        scope_hash: sample_scope_hash(),
        consent_scope: ModelLaneCloudConsentScope::SingleLane,
        retention_policy: ModelLaneCloudRetentionPolicy::NoTrainingEphemeral,
        export_posture: ModelLaneCloudExportPosture::RedactedContextOnly,
        fan_out_targets: vec!["provider://openai/byok".into()],
        approved: true,
        approved_by_ref: "operator://mt009/approval".into(),
        approved_at_utc: "2026-07-01T00:00:10Z".into(),
        valid_from_utc: "2026-01-01T00:00:00Z".into(),
        valid_until_utc: "2027-01-01T00:00:00Z".into(),
        revoked_at_utc: None,
        revocation_ref: None,
        status: ModelLaneCloudConsentReceiptStatus::Approved,
        event_ledger_stream_id: event_stream_id(run_id),
        work_packet_id: WP_ID.into(),
        micro_task_id: MT_ID.into(),
        task_board_id: TASK_BOARD_ID.into(),
        owner_session: OWNER.into(),
        idempotency_key: format!("idem-consent-{run_id}-{lane_id}"),
        created_at_utc: "2026-07-01T00:00:15Z".into(),
        user_manual_behavior_ref: USERMANUAL_BEHAVIOR.into(),
        diagnostic_payload: json!({
            "flight_recorder": "EventLedger",
            "provider_call_attempted": false,
            "locus": format!("locus://wp1/mt009/{run_id}/{lane_id}")
        }),
    }
}

fn materialize_crdt(messages: &[ModelLaneMessageRecord]) -> BTreeMap<String, String> {
    let mut ordered = messages
        .iter()
        .filter_map(|message| {
            let update_id = message
                .diagnostic_payload
                .get("crdt_update_id")
                .and_then(Value::as_str)?;
            let key = message
                .diagnostic_payload
                .get("crdt_key")
                .and_then(Value::as_str)?;
            let value = message
                .diagnostic_payload
                .get("crdt_value")
                .and_then(Value::as_str)?;
            Some((message.event_ledger_seq, update_id, key, value))
        })
        .collect::<Vec<_>>();
    ordered.sort_by_key(|(event_ledger_seq, _, _, _)| *event_ledger_seq);
    let mut seen = BTreeSet::new();
    let mut state = BTreeMap::new();
    for (_, update_id, key, value) in ordered {
        if seen.insert(update_id.to_owned()) {
            state.insert(key.to_owned(), value.to_owned());
        }
    }
    state
}

fn sample_locus(run_id: &str, session_id: &str, model_session_id: &str) -> ModelLaneLocusBinding {
    ModelLaneLocusBinding {
        work_packet_id: WP_ID.into(),
        micro_task_id: MT_ID.into(),
        task_board_id: Some(TASK_BOARD_ID.into()),
        coordinator_session_id: format!("coordinator-{run_id}"),
        session_id: session_id.into(),
        model_session_id: model_session_id.into(),
        owner_session: OWNER.into(),
        locus_binding_ref: format!("locus://wp1/mt009/{run_id}/{session_id}"),
    }
}

fn event_stream_id(run_id: &str) -> String {
    format!("mlane-stream-{run_id}")
}

fn payload_ref(message_id: &str) -> String {
    format!("artifact://model-lane/messages/{message_id}")
}

fn projection_plan_id(run_id: &str, lane_id: &str) -> String {
    format!("cloud-projection-plan://{run_id}/{lane_id}")
}

fn consent_receipt_id(run_id: &str, lane_id: &str) -> String {
    format!("cloud-consent-receipt://{run_id}/{lane_id}")
}

fn sample_sha256() -> String {
    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".into()
}

fn sample_scope_hash() -> String {
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into()
}

fn assert_error_contains(err: &impl std::fmt::Display, expected: &str) {
    let message = err.to_string();
    assert!(
        message.contains(expected),
        "expected error containing {expected}, got {message}"
    );
}

fn artifact_payload_json_for_message(message: &NewModelLaneMessage) -> Value {
    artifact_payload_json_parts(
        &message.message_id,
        &message.run_id,
        &message.payload_ref,
        message.crdt_update_ref.as_deref().unwrap_or_default(),
        message
            .diagnostic_payload
            .get("locus_ref")
            .and_then(Value::as_str)
            .unwrap_or_default(),
    )
}

fn artifact_payload_json_parts(
    message_id: &str,
    run_id: &str,
    payload_ref: &str,
    crdt_update_ref: &str,
    locus_ref: &str,
) -> Value {
    json!({
        "schema_id": "hsk.model_lane_message_payload@1",
        "message_id": message_id,
        "run_id": run_id,
        "payload_ref": payload_ref,
        "crdt_update_ref": crdt_update_ref,
        "locus": locus_ref,
    })
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn canonical_json_bytes(value: &Value) -> Vec<u8> {
    let mut output = String::new();
    write_canonical_json(&mut output, value);
    output.into_bytes()
}

fn write_canonical_json(output: &mut String, value: &Value) {
    match value {
        Value::Null => output.push_str("null"),
        Value::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
        Value::Number(value) => output.push_str(&value.to_string()),
        Value::String(value) => {
            output.push('"');
            for ch in value.chars() {
                match ch {
                    '"' => output.push_str("\\\""),
                    '\\' => output.push_str("\\\\"),
                    '\n' => output.push_str("\\n"),
                    '\r' => output.push_str("\\r"),
                    '\t' => output.push_str("\\t"),
                    ch => output.push(ch),
                }
            }
            output.push('"');
        }
        Value::Array(items) => {
            output.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_canonical_json(output, item);
            }
            output.push(']');
        }
        Value::Object(map) => {
            output.push('{');
            let mut keys = map.keys().collect::<Vec<_>>();
            keys.sort();
            for (index, key) in keys.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_canonical_json(output, &Value::String((*key).clone()));
                output.push(':');
                if let Some(value) = map.get(*key) {
                    write_canonical_json(output, value);
                }
            }
            output.push('}');
        }
    }
}

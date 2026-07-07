//! WP-1 MT-009: Dexterity mixed local/cloud/subagent integration proof.
//!
//! These tests use real PostgreSQL plus kernel_event_ledger rows. They prove
//! that a mixed ModelLaneRun is replayable, restartable, diagnosable by the
//! native Argus projection contract, and fail-closed when launch, payload,
//! CRDT, replay-order, direct-endpoint, or HBR posture authority is missing.

mod knowledge_pg_support;

use handshake_core::process_ledger::{
    LedgerBatcher, LedgerBatcherConfig, LedgerEventKind, LedgerOverflowEvent,
    PostgresProcessLedgerStore, ProcessEngineKind, ProcessLedgerError, ProcessLedgerOverflowSink,
    ProcessStart, ProcessStop,
};
use handshake_core::swarm_orchestration::model_lane::{
    LaunchAuthority, ModelLaneAuthority, ModelLaneCloudConsentReceiptStatus,
    ModelLaneCloudConsentScope, ModelLaneCloudExportPosture, ModelLaneCloudProjectionPlanStatus,
    ModelLaneCloudRetentionPolicy, ModelLaneDiagnosticTier, ModelLaneDiagnosticTierState,
    ModelLaneDiagnosticsLane, ModelLaneKind, ModelLaneLeaseScope, ModelLaneLeaseState,
    ModelLaneLocusBinding, ModelLaneMessageKind, ModelLaneMessageRecord, ModelLaneMtRuntimeStatus,
    ModelLaneProviderKind, ModelLaneRecord, ModelLaneRecoveryEventKind,
    ModelLaneRecoveryFailureKind, ModelLaneRecoveryState, ModelLaneRecoveryStatus,
    ModelLaneRoutingMetadata, ModelLaneStatus, ModelLaneStore, ModelLaneTarget, NewModelLane,
    NewModelLaneCloudConsentReceipt, NewModelLaneCloudProjectionPlan,
    NewModelLaneContextBundleArtifactBinding, NewModelLaneDiagnosticTierStatus, NewModelLaneLease,
    NewModelLaneMessage, NewModelLaneMtRuntimeStatus, NewModelLaneRecoveryCheckpoint,
    NewModelLaneRecoveryEvent, NewModelLaneRun, RuntimeBinding,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex},
    time::Duration,
};

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

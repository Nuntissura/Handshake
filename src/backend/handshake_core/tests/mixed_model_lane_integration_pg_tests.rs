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
use handshake_core::kernel::crdt::agent_lease::{
    claim_lease, release_lease, KnowledgeLeaseScopeKind, LeaseClaimOutcomeV1, LeaseClaimRequestV1,
};
use handshake_core::kernel::crdt::snapshot::{
    apply_crdt_compaction, build_snapshot_bounded_replay_plan, new_crdt_snapshot_record,
    plan_crdt_compaction, CrdtCompactionAuditMode, CrdtCompactionDisposition,
    CrdtCompactionPolicyV1, CrdtSnapshotRecordInputV1, CrdtSnapshotReplayError,
};
use handshake_core::kernel::crdt::state_vector::{verify_causal_chain, KnowledgeStateVectorV1};
use handshake_core::kernel::crdt::yjs_bridge::{
    pull_yjs_updates, push_yjs_update, read_draft_head, YjsPushDenialReasonV1, YjsPushDenialV1,
    YjsPushOutcomeV1, YjsUpdateEnvelopeV1, YJS_UPDATE_ENCODING_V1, YJS_UPDATE_ENVELOPE_SCHEMA_ID,
};
use handshake_core::kernel::{KernelEventType, NewKernelEvent};
use handshake_core::model_runtime::registry::RuntimeBinding as RuntimeAdapterBinding;
use handshake_core::model_runtime::{
    CancellationToken, Embedding, GenPrompt, GenerateRequest, GeneratedToken, KvCacheHandle,
    LoadSpec, LoraStackHandle, ModelCapabilities, ModelId, ModelRuntime, ModelRuntimeError,
    ProviderKind, SamplingParams, Score, SteeringHookHandle, TokenStream,
};
use handshake_core::process_ledger::{
    LedgerBatcher, LedgerBatcherConfig, LedgerEventKind, LedgerOverflowEvent,
    PostgresProcessLedgerStore, ProcessEngineKind, ProcessLedgerDrain, ProcessLedgerError,
    ProcessLedgerOverflowSink, ProcessOwnershipRecordId, ProcessStart, ProcessStop,
};
use handshake_core::storage::postgres::PostgresDatabase;
use handshake_core::storage::Database;
use handshake_core::swarm_orchestration::model_lane::{
    build_successful_launch_records, dexterity_spawn_model_session_id, DexterityLaunchAdapterKind,
    DexterityLaunchAdapterRequest, DexterityLaunchContract, LaunchAuthority, ModelLaneAuthority,
    ModelLaneCloudConsentReceiptStatus, ModelLaneCloudConsentScope, ModelLaneCloudExportPosture,
    ModelLaneCloudProjectionPlanStatus, ModelLaneCloudRetentionPolicy, ModelLaneDiagnosticTier,
    ModelLaneDiagnosticTierState, ModelLaneDiagnosticsLane, ModelLaneDiagnosticsProjection,
    ModelLaneKind, ModelLaneLeaseScope, ModelLaneLeaseState, ModelLaneLocusBinding,
    ModelLaneMessageKind, ModelLaneMessageRecord, ModelLaneMtRuntimeStatus,
    ModelLanePromotionDenialReason, ModelLanePromotionOutcome, ModelLaneProviderKind,
    ModelLaneRecord, ModelLaneRecoveryEventKind, ModelLaneRecoveryFailureKind,
    ModelLaneRecoveryState, ModelLaneRecoveryStatus, ModelLaneRoutingMetadata,
    ModelLaneRoutingPolicy, ModelLaneStatus, ModelLaneStore, ModelLaneTarget, NewModelLane,
    NewModelLaneCloudConsentReceipt, NewModelLaneCloudProjectionPlan,
    NewModelLaneContextBundleArtifactBinding, NewModelLaneDiagnosticTierStatus, NewModelLaneLease,
    NewModelLaneMessage, NewModelLaneMtRuntimeStatus, NewModelLanePromotionDecision,
    NewModelLaneRecoveryCheckpoint, NewModelLaneRecoveryEvent, NewModelLaneRun, RuntimeBinding,
};
use handshake_core::swarm_orchestration::production_factory::{
    execute_production_routing_lifecycle, execute_production_routing_wave,
};
use handshake_core::swarm_orchestration::routing::{
    ModelLaneRoutingAuthority, ModelLaneRoutingDispatchTarget,
};
use handshake_core::swarm_orchestration::routing_execution::{
    ModelLaneRoutingDispatchBatch, ModelLaneRoutingExecutionContext,
    ModelLaneRoutingExecutionStatus, ModelLaneRoutingStageLaunch, ModelLaneRoutingStageStateKind,
};
use handshake_core::swarm_orchestration::ModelLaneRoutingGraph;
use handshake_core::swarm_orchestration::{
    ByokCloudProvider, LiveSession, ModelInstanceId, ModelSessionFactory, RecordingSwarmSink,
    RunBudget, SpawnRequest, SwarmConfig, SwarmCoordinator, SwarmError,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::Barrier;
use yrs::updates::{decoder::Decode, encoder::Encode};
use yrs::{Doc, GetString, ReadTxn, StateVector, Text, Transact, Update};

#[tokio::test]
async fn mt009_kernel_crdt_authority_rejects_truncate() {
    let (pool, _store) = model_lane_store().await;
    for table in ["kernel_crdt_updates", "kernel_crdt_snapshots"] {
        let mut tx = pool.begin().await.expect("begin CRDT TRUNCATE probe");
        let error = sqlx::query(&format!("TRUNCATE TABLE {table}"))
            .execute(&mut *tx)
            .await
            .expect_err("append-only CRDT authority must reject TRUNCATE");
        assert!(
            error.to_string().contains("append-only CRDT authority"),
            "TRUNCATE rejection for {table} must name append-only authority: {error}"
        );
        tx.rollback()
            .await
            .expect("rollback rejected TRUNCATE probe");
    }
}

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

struct ForbiddenSubagentOsFactory {
    calls: Arc<AtomicUsize>,
}

#[async_trait]
impl ModelSessionFactory for ForbiddenSubagentOsFactory {
    async fn create(&self, _request: &SpawnRequest) -> Result<LiveSession, SwarmError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Err(SwarmError::FactoryFailed(
            "no-OS subagent lane must not invoke ModelSessionFactory".into(),
        ))
    }
}

fn mt009_diagnostics_projection_artifact_paths() -> (PathBuf, PathBuf) {
    let artifact_root = if let Ok(configured) = std::env::var("HANDSHAKE_ARTIFACTS_DIR") {
        std::fs::canonicalize(configured)
            .expect("HANDSHAKE_ARTIFACTS_DIR must resolve to an existing directory")
    } else {
        let manifest_dir = std::fs::canonicalize(env!("CARGO_MANIFEST_DIR"))
            .expect("backend crate manifest directory must resolve");
        let worktree_root = manifest_dir
            .parent()
            .and_then(std::path::Path::parent)
            .and_then(std::path::Path::parent)
            .expect("backend crate must live below the worktree src directory");
        std::fs::canonicalize(
            worktree_root
                .parent()
                .expect("worktree must have a parent")
                .join("Handshake_Artifacts"),
        )
        .expect("canonical sibling Handshake_Artifacts directory must exist")
    };
    let artifact = artifact_root
        .join("handshake-test")
        .join("wp1-final-audit")
        .join("mt009_mixed_model_lane_diagnostics_projection.json");
    std::fs::create_dir_all(
        artifact
            .parent()
            .expect("MT-009 diagnostics artifact has a parent directory"),
    )
    .expect("create MT-009 diagnostics artifact directory");
    let provenance = artifact.with_extension("provenance.json");
    (artifact, provenance)
}

fn clear_mt009_diagnostics_projection_artifact() {
    let (artifact, provenance) = mt009_diagnostics_projection_artifact_paths();
    for path in [artifact, provenance] {
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!(
                "remove stale MT-009 proof artifact {}: {error}",
                path.display()
            ),
        }
    }
}

fn emit_mt009_diagnostics_projection_artifact(projection: &ModelLaneDiagnosticsProjection) {
    let (artifact, provenance) = mt009_diagnostics_projection_artifact_paths();
    let proof_nonce = std::env::var("HANDSHAKE_MT009_DIAGNOSTICS_PROOF_NONCE")
        .unwrap_or_else(|_| format!("standalone-{}", uuid::Uuid::now_v7()));
    let artifact_bytes =
        serde_json::to_vec_pretty(projection).expect("serialize MT-009 diagnostics projection");
    let artifact_sha256 = hex::encode(Sha256::digest(&artifact_bytes));
    let producer_completed_at_unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be after Unix epoch")
        .as_millis() as u64;
    let temp_suffix = uuid::Uuid::now_v7();
    let artifact_temp = artifact.with_extension(format!("projection.{temp_suffix}.tmp"));
    let provenance_temp = provenance.with_extension(format!("provenance.{temp_suffix}.tmp"));
    std::fs::write(&artifact_temp, &artifact_bytes)
        .expect("write temporary backend-generated MT-009 diagnostics projection");
    std::fs::write(
        &provenance_temp,
        serde_json::to_vec_pretty(&json!({
            "schema_id": "hsk.mt009_diagnostics_projection_provenance@1",
            "proof_nonce": proof_nonce,
            "projection_schema_id": projection.schema_id,
            "artifact_sha256": artifact_sha256,
            "producer_test_id": "mixed_local_cloud_subagent_run_persists_restarts_replays_and_projects",
            "producer_status": "passed_all_backend_assertions",
            "producer_completed_at_unix_ms": producer_completed_at_unix_ms,
        }))
        .expect("serialize MT-009 diagnostics provenance"),
    )
    .expect("write temporary backend-generated MT-009 diagnostics provenance");
    std::fs::rename(&artifact_temp, &artifact)
        .expect("atomically publish backend-generated MT-009 diagnostics projection");
    std::fs::rename(&provenance_temp, &provenance)
        .expect("publish MT-009 producer completion receipt after projection bytes");
}

#[tokio::test]
async fn diagnostics_projection_rejects_model_stable_anchor_column_tamper() {
    const RUN_ID: &str = "run-mt009-stable-anchor-tamper";
    const LANE_ID: &str = "lane-mt009-stable-anchor-tamper";
    let (pool, store) = model_lane_store().await;
    seed_run_lane(&store, RUN_ID, LANE_ID, RuntimeBinding::Local).await;

    let original_anchor = sqlx::query_scalar::<_, Option<String>>(
        "SELECT model_stable_anchor FROM model_lanes WHERE lane_id = $1",
    )
    .bind(LANE_ID)
    .fetch_one(&pool)
    .await
    .expect("read initial durable model anchor");
    let forged_anchor = if original_anchor.as_deref() == Some(&"a".repeat(64)) {
        "b".repeat(64)
    } else {
        "a".repeat(64)
    };
    sqlx::query("UPDATE model_lanes SET model_stable_anchor = $2 WHERE lane_id = $1")
        .bind(LANE_ID)
        .bind(&forged_anchor)
        .execute(&pool)
        .await
        .expect("tamper mutable stable-anchor projection column");

    let error = store
        .diagnostics_projection(RUN_ID)
        .await
        .expect_err("forged stable anchor must fail before diagnostics identity projection");
    assert_error_contains(
        &error,
        "model_stable_anchor does not match initial EventLedger payload",
    );
}

#[tokio::test]
async fn mixed_local_cloud_subagent_run_persists_restarts_replays_and_projects() {
    let proof_phase = Arc::new(AtomicUsize::new(0));
    let active_phase = Arc::clone(&proof_phase);
    let proof = async move {
        active_phase.store(1, Ordering::SeqCst);
        clear_mt009_diagnostics_projection_artifact();
        let (pool, store) = model_lane_store().await;
        active_phase.store(2, Ordering::SeqCst);
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
        let (subagent_ledger, _subagent_ledger_drain) = LedgerBatcher::manual_for_tests(
            LedgerBatcherConfig::default(),
            Arc::new(RecordingOverflowSink::default()),
        )
        .expect("construct no-OS subagent coordinator ledger");
        let forbidden_factory_calls = Arc::new(AtomicUsize::new(0));
        let subagent_coordinator = SwarmCoordinator::new_with_model_lane_store(
            SwarmConfig::new(RunBudget::defaulted(1)),
            Arc::new(ForbiddenSubagentOsFactory {
                calls: forbidden_factory_calls.clone(),
            }),
            Arc::new(RecordingSwarmSink::new()),
            subagent_ledger,
            store.clone(),
        );
        active_phase.store(3, Ordering::SeqCst);
        let (_subagent_run, subagent_lane, subagent_manager) = subagent_coordinator
            .launch_operator_subagent_model_lane(sample_subagent_launch_request(
                RUN_ID,
                SUBAGENT_LANE_ID,
            ))
            .await
            .expect("launch no-OS subagent lane through coordinator-owned manager seam");
        active_phase.store(4, Ordering::SeqCst);
        assert_no_os_lane_runtime_contract(&subagent_lane);
        assert_eq!(
            forbidden_factory_calls.load(Ordering::SeqCst),
            0,
            "SubagentManager-owned no-OS lane must not invoke the OS/runtime factory"
        );

        let messages = vec![
            sample_message(LOCAL_MESSAGE_ID, RUN_ID, LOCAL_LANE_ID, "local", 1),
            sample_message(CLOUD_MESSAGE_ID, RUN_ID, CLOUD_LANE_ID, "cloud", 2),
            sample_message(SUBAGENT_MESSAGE_ID, RUN_ID, SUBAGENT_LANE_ID, "subagent", 3),
        ];
        for message in messages.iter().take(2) {
            store
                .record_message_with_payload_binding(
                    message.clone(),
                    sample_artifact_binding_for_message(message),
                )
                .await
                .expect("atomically record process-backed lane message and payload authority");
        }
        active_phase.store(5, Ordering::SeqCst);
        let stored_subagent_message = subagent_coordinator
            .record_operator_subagent_manager_output(
                &subagent_manager,
                messages[2].clone(),
                sample_artifact_binding_for_message(&messages[2]),
            )
            .await
            .expect(
                "SubagentManager receipt atomically records typed output and payload authority",
            );
        active_phase.store(6, Ordering::SeqCst);
        assert!(stored_subagent_message
            .diagnostic_payload
            .get("subagent_manager_receipt_ref")
            .and_then(Value::as_str)
            .is_some_and(|value| value.starts_with("subagent-manager-receipt://")));

        store
            .record_recovery_event(sample_recovery_event(
                "recovery-event-mt009-payload-001",
                RUN_ID,
                Some(LOCAL_LANE_ID),
                ModelLaneRecoveryEventKind::PayloadRefObserved,
                1,
                Some(payload_ref(LOCAL_MESSAGE_ID)),
                None,
                None,
                None,
            ))
            .await
            .expect("record checkpoint-bounded payload recovery event");
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

        active_phase.store(7, Ordering::SeqCst);
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
            "hsk.model_lane_diagnostics_projection@3"
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
            .is_some_and(
                |value| value.contains("subagent_manager") || value.contains("subagent-manager")
            ));
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

        active_phase.store(8, Ordering::SeqCst);
        let overflow_events =
            record_mixed_runtime_process_ledger_evidence(&pool, active_phase.as_ref()).await;
        active_phase.store(9, Ordering::SeqCst);
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

        active_phase.store(10, Ordering::SeqCst);
        store
            .record_lane_terminal_status(
                SUBAGENT_LANE_ID,
                ModelLaneStatus::Cancelled,
                "SubagentManager cancellation fence proof after durable output",
            )
            .await
            .expect("persist the no-OS subagent terminal cancellation boundary");
        let late_subagent_message = sample_message(
            "msg-mt009-subagent-late-after-cancel",
            RUN_ID,
            SUBAGENT_LANE_ID,
            "subagent",
            4,
        );
        let late_binding = sample_artifact_binding_for_message(&late_subagent_message);
        let late_binding_id = late_binding.artifact_binding_id.clone();
        active_phase.store(11, Ordering::SeqCst);
        let late_error = subagent_coordinator
            .record_operator_subagent_manager_output(
                &subagent_manager,
                late_subagent_message.clone(),
                late_binding,
            )
            .await
            .expect_err("SubagentManager output after cancellation must fail closed");
        active_phase.store(12, Ordering::SeqCst);
        assert!(
            late_error.to_string().contains("terminal source lane"),
            "late SubagentManager output must identify the terminal source boundary: {late_error}"
        );
        assert_no_message_row(&pool, &late_subagent_message.message_id).await;
        let late_binding_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM model_lane_context_bundle_artifacts WHERE artifact_binding_id=$1",
        )
        .bind(&late_binding_id)
        .fetch_one(&pool)
        .await
        .expect("count rejected late SubagentManager artifact binding");
        assert_eq!(
            late_binding_count, 0,
            "terminal-fenced SubagentManager output must roll back its payload binding"
        );
        let completed_projection = store
            .diagnostics_projection(RUN_ID)
            .await
            .expect("rebuild diagnostics after all backend assertions and cancellation probe");
        assert_eq!(completed_projection.messages.len(), 3);
        assert_eq!(
            completed_projection
                .lanes
                .iter()
                .find(|lane| lane.lane_id == SUBAGENT_LANE_ID)
                .map(|lane| lane.status.as_str()),
            Some("cancelled")
        );
        emit_mt009_diagnostics_projection_artifact(&completed_projection);
        active_phase.store(13, Ordering::SeqCst);
    };
    tokio::time::timeout(Duration::from_secs(600), proof)
        .await
        .unwrap_or_else(|_| {
            panic!(
                "MT-009 mixed producer proof exceeded 600 seconds in phase {}",
                proof_phase.load(Ordering::SeqCst)
            )
        });
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
            .is_some_and(
                |value| value.contains("subagent_manager") || value.contains("subagent-manager")
            ),
        "subagent lane must explain why no OS process exists"
    );
}

async fn record_mixed_runtime_process_ledger_evidence(
    pool: &PgPool,
    active_phase: &AtomicUsize,
) -> Vec<LedgerOverflowEvent> {
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
    active_phase.store(80, Ordering::SeqCst);
    let ledger_table_exists: bool =
        sqlx::query_scalar("SELECT pg_catalog.to_regclass('kernel_process_lifecycle') IS NOT NULL")
            .fetch_one(pool)
            .await
            .expect("inspect process-ledger relation in the already-migrated isolated schema");
    assert!(
        ledger_table_exists,
        "knowledge_pg must fully migrate the isolated schema before MT-009 ledger evidence"
    );
    active_phase.store(81, Ordering::SeqCst);
    drain
        .drain_available_to(ledger_store)
        .await
        .expect("MT-009 process ledger rows drain to PostgreSQL");
    active_phase.store(82, Ordering::SeqCst);
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
    let store = ModelLaneStore::new(pool.clone());
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
        "session-lane-mt009-real-yjs-local",
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
        "session-lane-mt009-real-yjs-cloud",
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
        "session-lane-mt009-real-yjs-operator",
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
    let mut forged_compaction = compaction.clone();
    forged_compaction
        .decisions
        .iter_mut()
        .find(|decision| decision.disposition == CrdtCompactionDisposition::CompactWithAudit)
        .expect("compaction contains an audited removal")
        .audit_ref = "eventledger://forged/event".into();
    assert!(matches!(
        apply_crdt_compaction(&snapshots[0], &records, &forged_compaction),
        Err(CrdtSnapshotReplayError::InvalidCompactionPlan {
            field: "decisions[].audit_ref",
            ..
        })
    ));
    let mut forged_promotion_compaction = compaction.clone();
    forged_promotion_compaction
        .decisions
        .iter_mut()
        .find(|decision| decision.update_id == "mt009-yjs-pre-cloud")
        .expect("compaction contains promotion evidence")
        .disposition = CrdtCompactionDisposition::CompactWithAudit;
    assert!(matches!(
        apply_crdt_compaction(&snapshots[0], &records, &forged_promotion_compaction),
        Err(CrdtSnapshotReplayError::PromotionEvidenceWouldBeDropped { update_id })
            if update_id == "mt009-yjs-pre-cloud"
    ));

    let applied_compaction = apply_crdt_compaction(&snapshots[0], &records, &compaction)
        .expect("apply compaction to the active replay representation");
    assert_eq!(
        applied_compaction.snapshot_sha256, snapshots[0].snapshot_sha256,
        "applied compaction keeps the authoritative snapshot hash"
    );
    assert_eq!(
        applied_compaction.snapshot_state_vector, snapshots[0].state_vector,
        "applied compaction keeps the snapshot state vector"
    );
    assert_eq!(applied_compaction.final_state_vector, final_state_vector);
    assert_eq!(
        applied_compaction.compacted_update_audits.len(),
        2,
        "two covered non-promotion updates move out of active replay into audit records"
    );
    for audit in &applied_compaction.compacted_update_audits {
        let original = records
            .iter()
            .find(|record| record.update_id == audit.update_id)
            .expect("compacted audit maps to an authoritative PostgreSQL update");
        assert_eq!(audit.update_sha256, original.update_sha256);
        assert_eq!(audit.update_bytes_ref, original.update_bytes_ref);
        assert_eq!(audit.state_vector_before, original.state_vector_before);
        assert_eq!(audit.state_vector_after, original.state_vector_after);
        assert_eq!(audit.replay_metadata, original.replay_metadata);
        assert_eq!(audit.event_ledger_event_id, original.event_ledger_event_id);
        assert_eq!(
            audit.audit_ref,
            format!(
                "eventledger://{}/{}",
                original.event_ledger_stream_id, original.event_ledger_event_id
            )
        );
    }
    assert!(applied_compaction
        .retained_updates
        .iter()
        .any(|record| record.update_id == "mt009-yjs-pre-cloud"));
    assert_eq!(
        applied_compaction
            .retained_updates
            .iter()
            .filter(|record| record.update_seq > snapshots[0].covered_update_seq)
            .count(),
        3
    );
    let replay_after_compaction =
        build_snapshot_bounded_replay_plan(&snapshots[0], &applied_compaction.retained_updates)
            .expect("post-compaction representation remains replayable from the snapshot");
    assert_eq!(
        replay_after_compaction.final_state_vector,
        final_state_vector
    );
    assert_eq!(
        replay_after_compaction
            .ordered_updates
            .iter()
            .map(|step| (&step.update_sha256, &step.state_vector_after))
            .collect::<Vec<_>>(),
        bounded_replay
            .ordered_updates
            .iter()
            .map(|step| (&step.update_sha256, &step.state_vector_after))
            .collect::<Vec<_>>(),
        "post-compaction replay preserves update hashes and state vectors"
    );
    let records_after_compaction_plan = db
        .list_kernel_crdt_updates(&workspace_id, &document_id, &crdt_document_id)
        .await
        .expect("authoritative PostgreSQL/EventLedger receipts remain available for audit");
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
    for ((message_id, lane_id, lane_label, crdt_value, actor), record) in [
        (
            "msg-mt009-yjs-post-local",
            LOCAL_LANE,
            "local",
            "local model merged Yjs edit",
            local_actor.clone(),
        ),
        (
            "msg-mt009-yjs-post-cloud",
            CLOUD_LANE,
            "cloud",
            "cloud model merged Yjs review",
            cloud_actor.clone(),
        ),
        (
            "msg-mt009-yjs-post-operator",
            OPERATOR_LANE,
            "operator",
            "operator merged Yjs decision",
            operator_actor.clone(),
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
        message.kind = ModelLaneMessageKind::Status;
        // `authority=PromotionCandidate` requires a proposal_ref (an advisory
        // routing proposal id, distinct from crdt_proposal_ref). Without it,
        // `validate_message_authority` rejects the message before the durable
        // CRDT resolver runs. It is NOT a CRDT authority field: STATUS-kind CRDT
        // messages carry `crdt_proposal_ref=None` (a Proposal-kind message would
        // require an approved applied-proposal row, currently unsatisfiable).
        message.proposal_ref = Some(format!("proposal://mt009/real-yjs/{message_id}"));
        message.crdt_update_ref = Some(record.update_bytes_ref.clone());
        message.crdt_base_snapshot_ref = Some(snapshot.snapshot_bytes_ref.clone());
        message.crdt_state_vector = Some(record.state_vector_after.clone());
        message
            .linked_span_contexts
            .push(format!("trace-{}", record.update_id));
        message.diagnostic_payload["crdt_update_id"] = json!(record.update_id);
        message.diagnostic_payload["crdt_key"] = json!("mt009.yjs.shared-document");
        message.diagnostic_payload["crdt_value"] = json!(crdt_value);
        message.payload_sha256 = sha256_hex(&canonical_json_bytes(
            &artifact_payload_json_for_message(&message),
        ));
        let lease = match claim_lease(
            &db,
            &pool,
            LeaseClaimRequestV1 {
                lane_id: lane_id.into(),
                actor: actor.clone(),
                session_id: format!("session-lane-mt009-real-yjs-{lane_label}"),
                correlation_id: format!("trace-{}", record.update_id),
                scope_kind: KnowledgeLeaseScopeKind::Document,
                scope_id: crdt_document_id.clone(),
                ttl_seconds: 3600,
            },
        )
        .await
        .expect("claim exact active MT-009 CRDT message lease")
        {
            LeaseClaimOutcomeV1::Claimed(lease) => lease,
            other => panic!("MT-009 CRDT message lease must claim: {other:?}"),
        };
        let stored_message = store
            .record_message_with_payload_binding(
                message.clone(),
                sample_artifact_binding_for_message(&message),
            )
            .await
            .expect("atomically record ModelLane message and payload authority for persisted Yjs receipt");
        assert_eq!(
            stored_message
                .crdt_authority_binding
                .as_ref()
                .expect("real Yjs message has CRDT lease authority")
                .lease_id,
            lease.lease_id
        );
        release_lease(&db, &pool, &lease.lease_id, &actor)
            .await
            .expect("release admitted MT-009 CRDT message lease")
            .expect("admitted MT-009 CRDT message lease exists");
        recorded_messages.push(stored_message);
    }

    let mut state_vector_mismatch = sample_message(
        "msg-mt009-yjs-state-vector-mismatch",
        RUN_ID,
        LOCAL_LANE,
        "local",
        7,
    );
    state_vector_mismatch.kind = ModelLaneMessageKind::Status;
    // proposal_ref makes the PromotionCandidate message pass the synchronous
    // authority check so the FABRICATED state vector is denied at the durable
    // CRDT resolver (its intended failure), not earlier on a missing proposal_ref.
    state_vector_mismatch.proposal_ref =
        Some("proposal://mt009/real-yjs/state-vector-mismatch".into());
    state_vector_mismatch.crdt_update_ref = Some(records[3].update_bytes_ref.clone());
    state_vector_mismatch.crdt_base_snapshot_ref = Some(snapshot.snapshot_bytes_ref.clone());
    state_vector_mismatch.crdt_state_vector = Some("hsk-sv1:fabricated-state".into());
    state_vector_mismatch.payload_sha256 = sha256_hex(&canonical_json_bytes(
        &artifact_payload_json_for_message(&state_vector_mismatch),
    ));
    let state_vector_mismatch_err = store
        .record_message_with_payload_binding(
            state_vector_mismatch.clone(),
            sample_artifact_binding_for_message(&state_vector_mismatch),
        )
        .await
        .expect_err("persisted update ref with a mismatched state vector must fail closed");
    assert_error_contains(&state_vector_mismatch_err, "crdt_state_vector");
    assert_no_message_row(&pool, &state_vector_mismatch.message_id).await;

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
        routing_launch_plan: Vec::new(),
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

    let current_promotion = store
        .record_promotion_decision(promotion_input.clone())
        .await
        .expect("persist current Yjs promotion approval in EventLedger");
    assert_eq!(
        current_promotion.outcome,
        ModelLanePromotionOutcome::Approved,
        "a promotion carrying the current persisted snapshot and state vector must pass ValidationRunner and PromotionGate authority"
    );
    assert_eq!(
        current_promotion.denial_reason, None,
        "the valid current Yjs promotion path must not carry a denial reason"
    );

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
    let mut stale_state_input = promotion_input.clone();
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

    sqlx::query(
        r#"
        UPDATE kernel_event_ledger
        SET payload = jsonb_set(
            payload,
            '{crdt_authority_binding,lease_id}',
            '"forged-lease-binding"'
        )
        WHERE event_id = $1
        "#,
    )
    .bind(&selected_message.event_ledger_event_id)
    .execute(&pool)
    .await
    .expect("tamper selected CRDT message EventLedger binding");
    let mut tampered_projection_input = promotion_input;
    tampered_projection_input.decision_id = "promotion-mt009-yjs-tampered-ledger".into();
    tampered_projection_input.decision_span_id = "span-promotion-mt009-yjs-tampered-ledger".into();
    tampered_projection_input.promotion_gate_ref =
        "promotion-gate://mt009/yjs/tampered-ledger".into();
    tampered_projection_input.promotion_receipt_ref =
        Some("promotion-receipt://mt009/yjs/tampered-ledger".into());
    tampered_projection_input.promoted_artifact_ref =
        Some("artifact://promoted/mt009/yjs/tampered-ledger".into());
    tampered_projection_input.idempotency_key = "idem-promotion-mt009-yjs-tampered-ledger".into();
    tampered_projection_input.replay_order_key =
        "00000007/promotion/mt009/yjs/tampered-ledger".into();
    let tampered_projection = store
        .record_promotion_decision(tampered_projection_input)
        .await
        .expect("tampered selected-message authority produces a durable promotion denial");
    assert_eq!(
        tampered_projection.outcome,
        ModelLanePromotionOutcome::Denied
    );
    assert_eq!(
        tampered_projection.denial_reason,
        Some(ModelLanePromotionDenialReason::InputRefMismatch),
        "promotion must fail closed when the source projection differs from MODEL_RESPONSE_RECORDED authority"
    );
}

/// Durable receipts persisted for one real CRDT document by
/// [`mt009_seed_real_crdt_document`]. Everything here is a genuine
/// PostgreSQL/EventLedger row created through `push_yjs_update` and
/// `append_kernel_crdt_snapshot`, so a message that references these values
/// exercises the real resolver, not a fabricated shortcut.
struct Mt009RealCrdtReceipts {
    /// `snapshot_bytes_ref` of a real snapshot covering `snapshot_covered_seq`.
    snapshot_bytes_ref: String,
    /// The snapshot's `covered_update_seq` (strictly less than the post-update
    /// seq, so the resolver's causal-ordering guard is satisfied).
    snapshot_covered_seq: i64,
    /// `update_bytes_ref` of a real post-snapshot update (seq == 2) that fully
    /// validates against its EventLedger event.
    post_update_bytes_ref: String,
    /// The post-snapshot update's server-derived `state_vector_after`.
    post_update_state_vector_after: String,
}

/// Persist one real CRDT document into the isolated schema behind `db`: a
/// pre-snapshot update (seq 1), a snapshot covering seq 1, and a post-snapshot
/// update (seq 2). Mirrors the persistence path proven by
/// `mt009_real_postgres_yjs_updates_compaction_receipts_and_lane_state_converge`
/// but trimmed to the minimum needed by the CRDT authority-binding negatives.
async fn mt009_seed_real_crdt_document(
    db: &(dyn Database + '_),
    workspace_id: &str,
    document_id: &str,
    crdt_document_id: &str,
    label: &str,
) -> Mt009RealCrdtReceipts {
    const DOCUMENT_SCHEMA_ID: &str = "hsk.doc.rich_document@1";
    let actor = KnowledgeActorIdV1::new(KnowledgeActorKind::LocalModel, &format!("{label}-local"))
        .expect("typed local model actor for real CRDT seed");
    let site = derive_knowledge_site_id(workspace_id, crdt_document_id, &actor);
    let session_id = format!("session-{label}");
    let mut state_vector = KnowledgeStateVectorV1::new();
    let canonical = Doc::new();

    let pre_update_id = format!("{label}-yjs-pre");
    let pre_bytes =
        mt009_append_yjs_text_update(&canonical, u64::from(site.yjs_client_id), &format!("[{label}-pre]"));
    mt009_push_yjs_update(
        db,
        workspace_id,
        document_id,
        crdt_document_id,
        DOCUMENT_SCHEMA_ID,
        &pre_update_id,
        &actor,
        &site.site_id,
        &session_id,
        &pre_bytes,
        &mut state_vector,
        1,
    )
    .await;

    let snapshot_state_vector = state_vector.encode();
    let snapshot_bytes = canonical
        .transact()
        .encode_state_as_update_v1(&StateVector::default());
    let snapshot_identity = knowledge_crdt_identity(
        workspace_id,
        document_id,
        crdt_document_id,
        DOCUMENT_SCHEMA_ID,
        &actor,
        &format!("trace-{label}-snapshot"),
    );
    let snapshot_event = NewKernelEvent::builder(
        format!("KTR-{}-SNAP", label.to_uppercase()),
        session_id.clone(),
        KernelEventType::KnowledgeCrdtSnapshotRecorded,
        actor.to_kernel_actor(),
    )
    .aggregate("knowledge_crdt_document", crdt_document_id.to_string())
    .idempotency_key(format!("{label}:snapshot"))
    .source_component("mixed_model_lane_integration_pg_tests")
    .payload(json!({
        "covered_update_seq": 1,
        "state_vector": &snapshot_state_vector,
        "document_id": document_id,
    }))
    .build()
    .expect("build real CRDT snapshot EventLedger event");
    let snapshot_event = db
        .append_kernel_event(snapshot_event)
        .await
        .expect("append real CRDT snapshot EventLedger event");
    let snapshot = new_crdt_snapshot_record(CrdtSnapshotRecordInputV1 {
        identity: &snapshot_identity,
        snapshot_id: &format!("{label}-snapshot-1"),
        covered_update_seq: 1,
        snapshot_bytes: &snapshot_bytes,
        snapshot_bytes_ref: &format!(
            "postgres://kernel_crdt_snapshots/{crdt_document_id}/{label}-snapshot-1"
        ),
        state_vector: &snapshot_state_vector,
        event_ledger_event_id: &snapshot_event.event_id,
        promotion_evidence_update_ids: &[pre_update_id.as_str()],
    });
    db.append_kernel_crdt_snapshot(snapshot.clone(), snapshot_bytes.clone())
        .await
        .expect("persist real CRDT snapshot receipt and bytes");

    let post_update_id = format!("{label}-yjs-post");
    let post_bytes = mt009_append_yjs_text_update(
        &canonical,
        u64::from(site.yjs_client_id),
        &format!("[{label}-post]"),
    );
    mt009_push_yjs_update(
        db,
        workspace_id,
        document_id,
        crdt_document_id,
        DOCUMENT_SCHEMA_ID,
        &post_update_id,
        &actor,
        &site.site_id,
        &session_id,
        &post_bytes,
        &mut state_vector,
        2,
    )
    .await;

    let records = db
        .list_kernel_crdt_updates(workspace_id, document_id, crdt_document_id)
        .await
        .expect("list persisted real CRDT updates");
    let post = records
        .iter()
        .find(|record| record.update_id == post_update_id)
        .expect("post-snapshot update is durably persisted");

    Mt009RealCrdtReceipts {
        snapshot_bytes_ref: snapshot.snapshot_bytes_ref.clone(),
        snapshot_covered_seq: 1,
        post_update_bytes_ref: post.update_bytes_ref.clone(),
        post_update_state_vector_after: post.state_vector_after.clone(),
    }
}

/// Everything a caller needs to drive one ADMISSIBLE CRDT-bearing
/// ModelLaneMessage through the shared `ModelLaneStore::record_message*`
/// admission boundary, produced by [`mt009_build_admissible_crdt_message`].
struct Mt009AdmissibleCrdtMessage {
    /// A STATUS-kind ModelLaneMessage that PASSES admission as-is: it references
    /// a real persisted Yjs update + real base snapshot, carries the server
    /// derived post-update state vector, links the update's CRDT trace, and is
    /// authorised by the active lease below. Record it with `record_message` (or
    /// recompute `payload_sha256` first if using the payload-binding variant).
    message: NewModelLaneMessage,
    /// The active `knowledge_crdt_agent_lane_leases` lease that authorises the
    /// message. Release it with `release_lease` after recording if desired.
    lease_id: String,
    /// The real CRDT receipts the message references.
    receipts: Mt009RealCrdtReceipts,
    run_id: String,
    lane_id: String,
    crdt_document_id: String,
}

/// The single canonical way to build ONE admissible CRDT-bearing
/// ModelLaneMessage. Future tests that need a message that PASSES CRDT admission
/// should reuse this instead of hand-wiring the CRDT identity triangle, which is
/// easy to get subtly wrong:
///   * the seeded update's `session_id` must equal the source lane's
///     `session_id` (`validate_crdt_lane_session_uniqueness_tx`),
///   * the lease's `correlation_id` must equal the update's `trace_id`
///     (`trace-{update_id}`) and its actor/session/scope must match
///     (`resolve_active_crdt_actor_lane_lease_tx`), and
///   * the message must link that trace in `linked_span_contexts`
///     (`bind_crdt_authority_to_lane`).
///
/// It (1) seeds a real ModelLaneRun + local lane whose `session_id` is
/// `session-{label}` via [`seed_run_lane`] (using `label` as the lane id so
/// `sample_lane`'s `session-{lane_id}` matches the seeded update session),
/// (2) seeds a real Yjs document (pre-update, snapshot, post-update) via
/// [`mt009_seed_real_crdt_document`], (3) claims the exact active
/// knowledge-agent lane lease, and (4) returns a STATUS-kind message carrying
/// the real update/snapshot/state-vector refs plus the CRDT trace link.
///
/// STATUS kind (not Proposal) is deliberate: a Proposal-kind CRDT message would
/// require an approved applied-proposal row whose `applied_update_sha256` equals
/// the Yjs-update hash, which is currently unsatisfiable (diff-hash vs
/// update-hash conflation, deferred to the MT-018 CRDT-admission context), so
/// `crdt_proposal_ref` is left `None`. `proposal_ref` is a routing-advisory id
/// required by `authority=PromotionCandidate`, not a CRDT authority field.
async fn mt009_build_admissible_crdt_message(
    store: &ModelLaneStore,
    db: &(dyn Database + '_),
    pool: &PgPool,
    workspace_id: &str,
    label: &str,
    message_id: &str,
) -> Mt009AdmissibleCrdtMessage {
    let run_id = format!("run-{label}");
    let lane_id = label.to_string();
    seed_run_lane(store, &run_id, &lane_id, RuntimeBinding::Local).await;

    let document_id = format!("doc-{label}-{workspace_id}");
    let crdt_document_id = format!("crdt-{label}-{workspace_id}");
    let receipts =
        mt009_seed_real_crdt_document(db, workspace_id, &document_id, &crdt_document_id, label)
            .await;

    // The seeded update's identity is deterministic in `label` (see
    // `mt009_seed_real_crdt_document`): actor = LocalModel `{label}-local`,
    // session = `session-{label}`, post-update id = `{label}-yjs-post`, and its
    // trace = `trace-{label}-yjs-post`. `sample_lane` stamps the lane session as
    // `session-{lane_id}` = `session-{label}`, so the update session is owned by
    // exactly this lane.
    let actor = KnowledgeActorIdV1::new(KnowledgeActorKind::LocalModel, &format!("{label}-local"))
        .expect("typed local model actor matching the seeded CRDT document");
    let session_id = format!("session-{label}");
    let update_trace_id = format!("trace-{label}-yjs-post");

    let lease = match claim_lease(
        db,
        pool,
        LeaseClaimRequestV1 {
            lane_id: lane_id.clone(),
            actor: actor.clone(),
            session_id: session_id.clone(),
            correlation_id: update_trace_id.clone(),
            scope_kind: KnowledgeLeaseScopeKind::Document,
            scope_id: crdt_document_id.clone(),
            ttl_seconds: 3600,
        },
    )
    .await
    .expect("claim the exact active knowledge-agent lane lease for the admissible message")
    {
        LeaseClaimOutcomeV1::Claimed(lease) => lease,
        other => panic!("admissible CRDT message lease must claim, got {other:?}"),
    };

    // seq 2 == the post-snapshot update sequence produced by the seed helper.
    let mut message = sample_message(message_id, &run_id, &lane_id, "local", 2);
    message.kind = ModelLaneMessageKind::Status;
    message.proposal_ref = Some(format!("proposal://mt009/admissible/{message_id}"));
    message.crdt_update_ref = Some(receipts.post_update_bytes_ref.clone());
    message.crdt_base_snapshot_ref = Some(receipts.snapshot_bytes_ref.clone());
    message.crdt_state_vector = Some(receipts.post_update_state_vector_after.clone());
    message.crdt_proposal_ref = None;
    message.crdt_stale_base_ref = None;
    message.linked_span_contexts.push(update_trace_id);

    Mt009AdmissibleCrdtMessage {
        message,
        lease_id: lease.lease_id,
        receipts,
        run_id,
        lane_id,
        crdt_document_id,
    }
}

/// Proof that the canonical [`mt009_build_admissible_crdt_message`] helper
/// actually PASSES the shared CRDT admission boundary and produces a resolved
/// CRDT lease authority binding on the stored message.
#[tokio::test]
async fn mt009_admissible_crdt_message_helper_records_and_binds() {
    let Some(kpg) = knowledge_pg_support::knowledge_pg().await else {
        eprintln!(
            "SKIP mt009_admissible_crdt_message_helper_records_and_binds: PostgreSQL binaries absent"
        );
        return;
    };
    let schema_url = kpg.schema_url.clone();
    let workspace_id = kpg.create_workspace().await;
    let db = kpg.db;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&schema_url)
        .await
        .expect("connect isolated schema for the admissible CRDT helper proof");
    let store = ModelLaneStore::new(pool.clone());

    let admissible = mt009_build_admissible_crdt_message(
        &store,
        &db,
        &pool,
        &workspace_id,
        "mt009-admissible-helper",
        "msg-mt009-admissible-helper",
    )
    .await;

    let stored = store
        .record_message(admissible.message.clone())
        .await
        .expect("the canonical admissible CRDT message must pass the shared admission boundary");
    let binding = stored
        .crdt_authority_binding
        .as_ref()
        .expect("an admitted CRDT message must carry a resolved CRDT lease authority binding");
    assert_eq!(binding.lease_id, admissible.lease_id);
    assert_eq!(
        binding.update_bytes_ref,
        admissible.receipts.post_update_bytes_ref
    );
    assert_eq!(binding.crdt_document_id, admissible.crdt_document_id);
    assert_eq!(binding.lane_id, admissible.lane_id);
    assert_eq!(stored.run_id, admissible.run_id);
}

/// Count MODEL_RESPONSE_RECORDED EventLedger appends for one ModelLane message
/// aggregate. A rejected admission must leave this at zero.
async fn mt009_model_lane_message_event_count(pool: &PgPool, message_id: &str) -> i64 {
    sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM kernel_event_ledger
        WHERE aggregate_type = 'model_lane_message'
          AND aggregate_id = $1
        "#,
    )
    .bind(message_id)
    .fetch_one(pool)
    .await
    .expect("count model_lane_message EventLedger rows")
}

/// MT-004 remediation (c): a `kernel_crdt_updates` row whose `update_sha256`
/// does not match `SHA-256(update_bytes)` must be rejected at the resolver's
/// stored-bytes hash check (`model_lane.rs` ~13076) before any message row or
/// EventLedger append. The tampered row is INSERTed directly (migration 0358
/// forbids UPDATE/DELETE/TRUNCATE, not INSERT) as a canonical clone of a real
/// update — same schema, storage authority, EventLedger event and valid Yjs
/// bytes — with only `update_sha256` corrupted, so the hash mismatch is the
/// exact and only reason admission fails.
#[tokio::test]
async fn mt009_crdt_update_bytes_hash_mismatch_fails_closed() {
    const RUN_ID: &str = "run-mt009-crdt-update-hash";
    const LANE_ID: &str = "lane-mt009-crdt-update-hash";
    const MESSAGE_ID: &str = "msg-mt009-crdt-update-hash-mismatch";
    const LABEL: &str = "mt009-uhash";

    let Some(kpg) = knowledge_pg_support::knowledge_pg().await else {
        eprintln!("SKIP mt009_crdt_update_bytes_hash_mismatch_fails_closed: PostgreSQL binaries absent");
        return;
    };
    let schema_url = kpg.schema_url.clone();
    let workspace_id = kpg.create_workspace().await;
    let db = kpg.db;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&schema_url)
        .await
        .expect("connect isolated schema for CRDT update hash-mismatch proof");
    let store = ModelLaneStore::new(pool.clone());
    let document_id = format!("doc-{LABEL}-{workspace_id}");
    let crdt_document_id = format!("crdt-{LABEL}-{workspace_id}");

    let receipts =
        mt009_seed_real_crdt_document(&db, &workspace_id, &document_id, &crdt_document_id, LABEL).await;
    seed_run_lane(&store, RUN_ID, LANE_ID, RuntimeBinding::Local).await;

    // INSERT a canonical clone of the real post-snapshot update, corrupting only
    // update_sha256 (and the columns that carry UNIQUE indexes so the clone can
    // coexist with its source). update_bytes stays valid, so the resolver's
    // recomputed hash cannot match the persisted, tampered update_sha256.
    let tampered_update_ref =
        format!("postgres://kernel_crdt_updates/{crdt_document_id}/{LABEL}-tampered-hash");
    let tampered_stream = format!("knowledge-crdt-tampered:{crdt_document_id}:{LABEL}");
    let wrong_sha = "0".repeat(64);
    let inserted = sqlx::query(
        r#"
        INSERT INTO kernel_crdt_updates
            (schema_id, workspace_id, document_id, crdt_document_id, update_id, update_seq,
             update_sha256, update_bytes_ref, update_bytes, actor_id, actor_kind, session_id,
             trace_id, state_vector_before, state_vector_after, replay_metadata_json,
             event_ledger_stream_id, event_ledger_event_id, storage_authority)
        SELECT schema_id, workspace_id, document_id, crdt_document_id,
               $2, update_seq + 100000, $3, $4, update_bytes, actor_id, actor_kind, session_id,
               trace_id, state_vector_before, state_vector_after, replay_metadata_json,
               $5, event_ledger_event_id, storage_authority
        FROM kernel_crdt_updates
        WHERE update_bytes_ref = $1
        "#,
    )
    .bind(&receipts.post_update_bytes_ref)
    .bind(format!("{LABEL}-tampered-hash"))
    .bind(&wrong_sha)
    .bind(&tampered_update_ref)
    .bind(&tampered_stream)
    .execute(&pool)
    .await
    .expect("INSERT tampered kernel_crdt_updates clone (INSERT is not blocked by migration 0358)");
    assert_eq!(
        inserted.rows_affected(),
        1,
        "tampered update clone must be inserted exactly once"
    );

    // The posture must be COMPLETE: validate_message_authority (~15137) rejects a
    // partial CRDT posture synchronously, which would short-circuit this test
    // before the durable stored-bytes hash check it exists to prove.
    let mut message = sample_message(MESSAGE_ID, RUN_ID, LANE_ID, "local", 2);
    message.kind = ModelLaneMessageKind::Status;
    message.proposal_ref = Some("proposal://mt009/update-hash-mismatch".into());
    message.crdt_update_ref = Some(tampered_update_ref);
    message.crdt_base_snapshot_ref = Some(receipts.snapshot_bytes_ref.clone());
    message.crdt_state_vector = Some(receipts.post_update_state_vector_after.clone());
    message.crdt_proposal_ref = Some("crdt-proposal://mt009-update-hash-mismatch".into());
    let error = store
        .record_message(message.clone())
        .await
        .expect_err("a CRDT update whose stored bytes hash mismatches update_sha256 must fail closed");
    assert_error_contains(&error, "CRDT authority resolution failed");
    assert_error_contains(&error, "does not match persisted update_sha256");

    assert_no_message_row(&pool, MESSAGE_ID).await;
    assert_eq!(
        mt009_model_lane_message_event_count(&pool, MESSAGE_ID).await,
        0,
        "a hash-mismatched CRDT update must not append a ModelLane message EventLedger event"
    );
}

/// MT-004 remediation (c), snapshot arm: a `kernel_crdt_snapshots` row whose
/// `snapshot_sha256` does not match `SHA-256(snapshot_bytes)` must be rejected
/// at the resolver's snapshot hash check (`model_lane.rs` ~13271). The
/// referenced update is fully real and validates end to end; only the base
/// snapshot is a canonical clone with a corrupted `snapshot_sha256`, proving
/// the snapshot-bytes integrity gate fires independently of the update gate.
#[tokio::test]
async fn mt009_crdt_snapshot_bytes_hash_mismatch_fails_closed() {
    const RUN_ID: &str = "run-mt009-crdt-snapshot-hash";
    const LANE_ID: &str = "lane-mt009-crdt-snapshot-hash";
    const MESSAGE_ID: &str = "msg-mt009-crdt-snapshot-hash-mismatch";
    const LABEL: &str = "mt009-shash";

    let Some(kpg) = knowledge_pg_support::knowledge_pg().await else {
        eprintln!("SKIP mt009_crdt_snapshot_bytes_hash_mismatch_fails_closed: PostgreSQL binaries absent");
        return;
    };
    let schema_url = kpg.schema_url.clone();
    let workspace_id = kpg.create_workspace().await;
    let db = kpg.db;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&schema_url)
        .await
        .expect("connect isolated schema for CRDT snapshot hash-mismatch proof");
    let store = ModelLaneStore::new(pool.clone());
    let document_id = format!("doc-{LABEL}-{workspace_id}");
    let crdt_document_id = format!("crdt-{LABEL}-{workspace_id}");

    let receipts =
        mt009_seed_real_crdt_document(&db, &workspace_id, &document_id, &crdt_document_id, LABEL).await;
    seed_run_lane(&store, RUN_ID, LANE_ID, RuntimeBinding::Local).await;

    // Canonical clone of the real snapshot with only snapshot_sha256 corrupted.
    // covered_update_seq is preserved (== 1 < post-update seq 2) so the causal
    // and entity guards pass and the hash check is the sole failure reason.
    let tampered_snapshot_ref =
        format!("postgres://kernel_crdt_snapshots/{crdt_document_id}/{LABEL}-tampered-hash");
    let tampered_stream = format!("knowledge-crdt-tampered:{crdt_document_id}:{LABEL}");
    let wrong_sha = "0".repeat(64);
    let inserted = sqlx::query(
        r#"
        INSERT INTO kernel_crdt_snapshots
            (schema_id, snapshot_id, workspace_id, document_id, crdt_document_id, covered_update_seq,
             state_vector, snapshot_sha256, snapshot_bytes_ref, snapshot_bytes, actor_id, actor_kind,
             event_ledger_stream_id, event_ledger_event_id, promotion_evidence_update_ids,
             storage_authority)
        SELECT schema_id, $2, workspace_id, document_id, crdt_document_id, covered_update_seq,
               state_vector, $3, $4, snapshot_bytes, actor_id, actor_kind,
               $5, event_ledger_event_id, promotion_evidence_update_ids, storage_authority
        FROM kernel_crdt_snapshots
        WHERE snapshot_bytes_ref = $1
        "#,
    )
    .bind(&receipts.snapshot_bytes_ref)
    .bind(format!("{LABEL}-tampered-hash"))
    .bind(&wrong_sha)
    .bind(&tampered_snapshot_ref)
    .bind(&tampered_stream)
    .execute(&pool)
    .await
    .expect("INSERT tampered kernel_crdt_snapshots clone");
    assert_eq!(
        inserted.rows_affected(),
        1,
        "tampered snapshot clone must be inserted exactly once"
    );
    assert!(
        receipts.snapshot_covered_seq < 2,
        "cloned snapshot must remain causally before the referenced post-update seq"
    );

    // COMPLETE posture -- see the update-hash test above for why a partial
    // posture would never reach the durable snapshot integrity gate.
    let mut message = sample_message(MESSAGE_ID, RUN_ID, LANE_ID, "local", 2);
    message.kind = ModelLaneMessageKind::Status;
    message.proposal_ref = Some("proposal://mt009/snapshot-hash-mismatch".into());
    message.crdt_update_ref = Some(receipts.post_update_bytes_ref.clone());
    message.crdt_base_snapshot_ref = Some(tampered_snapshot_ref);
    message.crdt_state_vector = Some(receipts.post_update_state_vector_after.clone());
    message.crdt_proposal_ref = Some("crdt-proposal://mt009-snapshot-hash-mismatch".into());
    let error = store
        .record_message(message.clone())
        .await
        .expect_err("a base snapshot whose stored bytes hash mismatches snapshot_sha256 must fail closed");
    assert_error_contains(&error, "CRDT authority resolution failed");
    assert_error_contains(&error, "does not match persisted snapshot_sha256");

    assert_no_message_row(&pool, MESSAGE_ID).await;
    assert_eq!(
        mt009_model_lane_message_event_count(&pool, MESSAGE_ID).await,
        0,
        "a hash-mismatched CRDT snapshot must not append a ModelLane message EventLedger event"
    );
}

/// MT-004/MT-009 hardening: a message that references a real, fully valid
/// persisted update but supplies a `state_vector_after` belonging to a
/// different CRDT document must fail closed at the resolver's state-vector
/// identity check (`model_lane.rs` ~13141). This is stronger than a fabricated
/// state-vector string: the supplied vector is itself a genuine server-derived
/// vector, just for the wrong document, proving the binding is by identity and
/// not merely by format.
#[tokio::test]
async fn mt009_crdt_unrelated_document_state_vector_fails_closed() {
    const RUN_ID: &str = "run-mt009-crdt-foreign-sv";
    const LANE_ID: &str = "lane-mt009-crdt-foreign-sv";
    const MESSAGE_ID: &str = "msg-mt009-crdt-foreign-state-vector";
    const LABEL_A: &str = "mt009-fsv-a";
    const LABEL_B: &str = "mt009-fsv-b";

    let Some(kpg) = knowledge_pg_support::knowledge_pg().await else {
        eprintln!("SKIP mt009_crdt_unrelated_document_state_vector_fails_closed: PostgreSQL binaries absent");
        return;
    };
    let schema_url = kpg.schema_url.clone();
    let workspace_id = kpg.create_workspace().await;
    let db = kpg.db;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&schema_url)
        .await
        .expect("connect isolated schema for foreign-document state-vector proof");
    let store = ModelLaneStore::new(pool.clone());

    let document_a = format!("doc-{LABEL_A}-{workspace_id}");
    let crdt_document_a = format!("crdt-{LABEL_A}-{workspace_id}");
    let receipts_a =
        mt009_seed_real_crdt_document(&db, &workspace_id, &document_a, &crdt_document_a, LABEL_A).await;

    let document_b = format!("doc-{LABEL_B}-{workspace_id}");
    let crdt_document_b = format!("crdt-{LABEL_B}-{workspace_id}");
    let receipts_b =
        mt009_seed_real_crdt_document(&db, &workspace_id, &document_b, &crdt_document_b, LABEL_B).await;

    // Two distinct documents derive distinct site ids, so their server-derived
    // state vectors are genuinely different -- the negative is real, not a
    // coincidental string collision.
    assert_ne!(
        receipts_a.post_update_state_vector_after, receipts_b.post_update_state_vector_after,
        "distinct CRDT documents must yield distinct state vectors for a real negative"
    );

    seed_run_lane(&store, RUN_ID, LANE_ID, RuntimeBinding::Local).await;

    // COMPLETE posture -- a partial one is rejected synchronously and would
    // never reach the state-vector identity check under test.
    let mut message = sample_message(MESSAGE_ID, RUN_ID, LANE_ID, "local", 2);
    message.kind = ModelLaneMessageKind::Status;
    message.proposal_ref = Some("proposal://mt009/foreign-state-vector".into());
    message.crdt_update_ref = Some(receipts_a.post_update_bytes_ref.clone());
    message.crdt_base_snapshot_ref = Some(receipts_a.snapshot_bytes_ref.clone());
    // Real, but from document B -- it does not match document A's persisted
    // state_vector_after.
    message.crdt_state_vector = Some(receipts_b.post_update_state_vector_after.clone());
    message.crdt_proposal_ref = Some("crdt-proposal://mt009-foreign-state-vector".into());
    let error = store
        .record_message(message.clone())
        .await
        .expect_err("a real update paired with a foreign-document state vector must fail closed");
    assert_error_contains(&error, "CRDT authority resolution failed");
    assert_error_contains(&error, "does not match persisted state_vector_after");

    assert_no_message_row(&pool, MESSAGE_ID).await;
    assert_eq!(
        mt009_model_lane_message_event_count(&pool, MESSAGE_ID).await,
        0,
        "a foreign-document state vector must not append a ModelLane message EventLedger event"
    );
}

// ===========================================================================
// MT-004 remediation step 3 -- all-six-policy CRDT admission negatives.
//
// MT-004 `validation_v4.remediation_plan[2]` requires "all-six-policy negative
// tests for missing rows, hash mismatch, stale vectors, duplicates, and replay
// order against real PostgreSQL". MT-009 `validation_v2.remediation_plan[1]`
// states the same requirement per routing policy.
//
// Why the tests are shaped this way (read before adding a 7th policy):
//
//   Every routing policy commits its stage output through ONE shared admission
//   path -- `ModelLaneStore::record_message*` -> `record_message_tx`
//   (`swarm_orchestration/model_lane.rs` ~342) -> duplicate/idempotency gate
//   (~357-384) -> `validate_message_crdt_authority_tx` (~13705) ->
//   `resolve_model_lane_crdt_authority_tx` (~13011). `ModelLaneRoutingPolicy`
//   is not an argument to any of those functions, so no per-policy CRDT
//   admission branch exists that could diverge.
//
//   That structural fact is proven, not assumed, by
//   `mt004_every_routing_policy_stage_output_routes_through_shared_crdt_admission_boundary`,
//   which walks EVERY stage of EVERY policy graph. The five class tests below
//   additionally drive each negative through all six policies on their own
//   run/lane/message identities, so a failure names the exact policy instead of
//   relying on the shared-boundary argument alone. Proving one policy and
//   inferring the rest is deliberately NOT done here.
//
// All six policies come from `ModelLaneRoutingPolicy::all()`, so a new policy
// variant is picked up automatically and the drift guard in the structural
// test fails loudly if the canonical set changes.
// ===========================================================================

/// One routing policy's isolated durable identity for an MT-004 negative probe.
struct Mt004PolicyProbe {
    policy: ModelLaneRoutingPolicy,
    run_id: String,
    lane_id: String,
}

/// Seed one real ModelLaneRun + lane per routing policy inside a single
/// isolated schema. Distinct ids per policy mean an assertion failure names the
/// exact policy that regressed rather than a shared fixture.
async fn mt004_seed_policy_probes(store: &ModelLaneStore, case: &str) -> Vec<Mt004PolicyProbe> {
    let mut probes = Vec::new();
    for policy in ModelLaneRoutingPolicy::all().iter().copied() {
        let run_id = format!("run-mt004-{case}-{}", policy.as_str());
        let lane_id = format!("lane-mt004-{case}-{}", policy.as_str());
        seed_run_lane(store, &run_id, &lane_id, RuntimeBinding::Local).await;
        probes.push(Mt004PolicyProbe {
            policy,
            run_id,
            lane_id,
        });
    }
    assert_eq!(
        probes.len(),
        6,
        "MT-004 requires all six routing policies to be probed; ModelLaneRoutingPolicy::all() returned {}",
        probes.len()
    );
    probes
}

/// Assert one CRDT-bearing ModelLaneMessage is denied at the shared admission
/// boundary and leaves no durable trace: no `model_lane_messages` row and no
/// `model_lane_message` EventLedger append. `expected_detail` pins the exact
/// resolver gate that fired so a test cannot pass on an unrelated denial.
async fn mt004_assert_crdt_admission_denied(
    store: &ModelLaneStore,
    pool: &PgPool,
    policy: ModelLaneRoutingPolicy,
    message: NewModelLaneMessage,
    expected_detail: &str,
) {
    let message_id = message.message_id.clone();
    let error = store.record_message(message).await.expect_err(&format!(
        "routing policy {} must fail closed on: {expected_detail}",
        policy.as_str()
    ));
    // CX-MM-006 is the declared ModelLane CRDT authority failstate code.
    assert_error_contains(&error, "CX-MM-006");
    assert_error_contains(&error, "CRDT authority resolution failed");
    assert_error_contains(&error, expected_detail);
    assert_no_message_row(pool, &message_id).await;
    assert_eq!(
        mt009_model_lane_message_event_count(pool, &message_id).await,
        0,
        "policy {} denied message {message_id} must not append a ModelLane EventLedger event",
        policy.as_str()
    );
}

/// Build a CRDT-bearing probe message on one policy's run/lane.
///
/// The COMPLETE CRDT posture is supplied deliberately. `validate_message_authority`
/// (`model_lane.rs` ~15137) treats any single `crdt_*` field as a declaration of
/// CRDT authority and then requires `proposal_ref`, `crdt_update_ref`,
/// `crdt_base_snapshot_ref`, `crdt_state_vector` and `crdt_proposal_ref` to all
/// be present. A partial posture is therefore rejected synchronously, before the
/// durable resolver ever runs -- which would silently turn these tests into
/// field-presence tests instead of the authority-resolution tests they must be.
/// `Status` kind additionally keeps the `Proposal`-only precondition
/// (~13737) from pre-empting the gate under test.
fn mt004_crdt_probe_message(
    probe: &Mt004PolicyProbe,
    case: &str,
    arm: &str,
    update_ref: String,
    snapshot_ref: String,
    state_vector: String,
) -> NewModelLaneMessage {
    let policy = probe.policy.as_str();
    let message_id = format!("msg-mt004-{case}-{arm}-{policy}");
    let mut message = sample_message(&message_id, &probe.run_id, &probe.lane_id, "local", 2);
    message.kind = ModelLaneMessageKind::Status;
    message.proposal_ref = Some(format!("proposal://mt004/{case}/{arm}/{policy}"));
    message.crdt_update_ref = Some(update_ref);
    message.crdt_base_snapshot_ref = Some(snapshot_ref);
    message.crdt_state_vector = Some(state_vector);
    // Resolution of the update/snapshot refs happens before the proposal is
    // dereferenced (~13743 vs ~13747), so every arm below still fails on the
    // gate it names rather than on the proposal lookup.
    message.crdt_proposal_ref = Some(format!("crdt-proposal://mt004-{case}-{arm}-{policy}"));
    message
}

/// Open an isolated real-PostgreSQL schema plus one seeded real CRDT document
/// for an MT-004 all-six-policy negative test. Returns `None` only when the
/// PostgreSQL binaries are genuinely absent (the helper never falls back to a
/// mock or SQLite path).
async fn mt004_case_fixture(
    label: &str,
) -> Option<(
    PgPool,
    ModelLaneStore,
    PostgresDatabase,
    String,
    String,
    Mt009RealCrdtReceipts,
)> {
    let kpg = knowledge_pg_support::knowledge_pg().await?;
    let schema_url = kpg.schema_url.clone();
    let workspace_id = kpg.create_workspace().await;
    let db = kpg.db;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&schema_url)
        .await
        .expect("connect isolated schema for MT-004 all-six-policy CRDT negative proof");
    let store = ModelLaneStore::new(pool.clone());
    let document_id = format!("doc-{label}-{workspace_id}");
    let crdt_document_id = format!("crdt-{label}-{workspace_id}");
    let receipts =
        mt009_seed_real_crdt_document(&db, &workspace_id, &document_id, &crdt_document_id, label)
            .await;
    Some((
        pool,
        store,
        db,
        workspace_id,
        crdt_document_id,
        receipts,
    ))
}

/// MT-004 class 1/5 -- MISSING ROWS, all six routing policies.
///
/// A syntactically perfect `postgres://kernel_crdt_updates/...` /
/// `postgres://kernel_crdt_snapshots/...` reference that has no backing row is
/// exactly the FAIL_V4 finding ("successful routing can emit ... authority
/// references that are not backed by a persisted kernel_crdt_updates/Yjs
/// object"). Both arms must be denied for every policy before persistence.
#[tokio::test]
async fn mt004_all_six_policies_reject_missing_crdt_rows() {
    const CASE: &str = "missingrow";
    const LABEL: &str = "mt004-missing";

    let Some((pool, store, _db, _workspace_id, crdt_document_id, receipts)) =
        mt004_case_fixture(LABEL).await
    else {
        eprintln!(
            "SKIP mt004_all_six_policies_reject_missing_crdt_rows: PostgreSQL binaries absent"
        );
        return;
    };
    let probes = mt004_seed_policy_probes(&store, CASE).await;

    for probe in &probes {
        // Arm A: fabricated update ref, real snapshot and real state vector.
        mt004_assert_crdt_admission_denied(
            &store,
            &pool,
            probe.policy,
            mt004_crdt_probe_message(
                probe,
                CASE,
                "update",
                format!(
                    "postgres://kernel_crdt_updates/{crdt_document_id}/{}-fabricated",
                    probe.policy.as_str()
                ),
                receipts.snapshot_bytes_ref.clone(),
                receipts.post_update_state_vector_after.clone(),
            ),
            "does not resolve to kernel_crdt_updates",
        )
        .await;

        // Arm B: real update ref, fabricated base snapshot ref. Proves the
        // snapshot arm of the resolver is not satisfied by a valid update.
        mt004_assert_crdt_admission_denied(
            &store,
            &pool,
            probe.policy,
            mt004_crdt_probe_message(
                probe,
                CASE,
                "snapshot",
                receipts.post_update_bytes_ref.clone(),
                format!(
                    "postgres://kernel_crdt_snapshots/{crdt_document_id}/{}-fabricated",
                    probe.policy.as_str()
                ),
                receipts.post_update_state_vector_after.clone(),
            ),
            "does not resolve to kernel_crdt_snapshots",
        )
        .await;
    }
}

/// MT-004 class 2/5 -- HASH MISMATCH, all six routing policies.
///
/// Canonical clones of a real update and a real snapshot are INSERTed with only
/// the persisted digest corrupted (migration 0358 blocks UPDATE/DELETE/TRUNCATE,
/// not INSERT). Bytes, schema, storage authority, EventLedger event and causal
/// ordering all stay valid, so the recomputed-vs-persisted hash comparison is
/// the single reason admission fails for every policy.
#[tokio::test]
async fn mt004_all_six_policies_reject_crdt_hash_mismatch() {
    const CASE: &str = "hashmismatch";
    const LABEL: &str = "mt004-hash";

    let Some((pool, store, _db, _workspace_id, crdt_document_id, receipts)) =
        mt004_case_fixture(LABEL).await
    else {
        eprintln!(
            "SKIP mt004_all_six_policies_reject_crdt_hash_mismatch: PostgreSQL binaries absent"
        );
        return;
    };
    let wrong_sha = "0".repeat(64);

    let tampered_update_ref =
        format!("postgres://kernel_crdt_updates/{crdt_document_id}/{LABEL}-badhash");
    let inserted = sqlx::query(
        r#"
        INSERT INTO kernel_crdt_updates
            (schema_id, workspace_id, document_id, crdt_document_id, update_id, update_seq,
             update_sha256, update_bytes_ref, update_bytes, actor_id, actor_kind, session_id,
             trace_id, state_vector_before, state_vector_after, replay_metadata_json,
             event_ledger_stream_id, event_ledger_event_id, storage_authority)
        SELECT schema_id, workspace_id, document_id, crdt_document_id,
               $2, update_seq + 100000, $3, $4, update_bytes, actor_id, actor_kind, session_id,
               trace_id, state_vector_before, state_vector_after, replay_metadata_json,
               $5, event_ledger_event_id, storage_authority
        FROM kernel_crdt_updates
        WHERE update_bytes_ref = $1
        "#,
    )
    .bind(&receipts.post_update_bytes_ref)
    .bind(format!("{LABEL}-badhash"))
    .bind(&wrong_sha)
    .bind(&tampered_update_ref)
    .bind(format!("knowledge-crdt-mt004-badhash:{crdt_document_id}"))
    .execute(&pool)
    .await
    .expect("INSERT hash-tampered kernel_crdt_updates clone");
    assert_eq!(inserted.rows_affected(), 1);

    let tampered_snapshot_ref =
        format!("postgres://kernel_crdt_snapshots/{crdt_document_id}/{LABEL}-badhash");
    let inserted = sqlx::query(
        r#"
        INSERT INTO kernel_crdt_snapshots
            (schema_id, snapshot_id, workspace_id, document_id, crdt_document_id, covered_update_seq,
             state_vector, snapshot_sha256, snapshot_bytes_ref, snapshot_bytes, actor_id, actor_kind,
             event_ledger_stream_id, event_ledger_event_id, promotion_evidence_update_ids,
             storage_authority)
        SELECT schema_id, $2, workspace_id, document_id, crdt_document_id, covered_update_seq,
               state_vector, $3, $4, snapshot_bytes, actor_id, actor_kind,
               $5, event_ledger_event_id, promotion_evidence_update_ids, storage_authority
        FROM kernel_crdt_snapshots
        WHERE snapshot_bytes_ref = $1
        "#,
    )
    .bind(&receipts.snapshot_bytes_ref)
    .bind(format!("{LABEL}-badhash"))
    .bind(&wrong_sha)
    .bind(&tampered_snapshot_ref)
    .bind(format!("knowledge-crdt-mt004-badhash-snap:{crdt_document_id}"))
    .execute(&pool)
    .await
    .expect("INSERT hash-tampered kernel_crdt_snapshots clone");
    assert_eq!(inserted.rows_affected(), 1);
    assert!(
        receipts.snapshot_covered_seq < 2,
        "cloned snapshot must stay causally before the referenced post-update seq"
    );

    let probes = mt004_seed_policy_probes(&store, CASE).await;
    for probe in &probes {
        // Arm A: update bytes no longer hash to the persisted update_sha256.
        mt004_assert_crdt_admission_denied(
            &store,
            &pool,
            probe.policy,
            mt004_crdt_probe_message(
                probe,
                CASE,
                "update",
                tampered_update_ref.clone(),
                receipts.snapshot_bytes_ref.clone(),
                receipts.post_update_state_vector_after.clone(),
            ),
            "does not match persisted update_sha256",
        )
        .await;

        // Arm B: snapshot bytes no longer hash to the persisted snapshot_sha256.
        mt004_assert_crdt_admission_denied(
            &store,
            &pool,
            probe.policy,
            mt004_crdt_probe_message(
                probe,
                CASE,
                "snapshot",
                receipts.post_update_bytes_ref.clone(),
                tampered_snapshot_ref.clone(),
                receipts.post_update_state_vector_after.clone(),
            ),
            "does not match persisted snapshot_sha256",
        )
        .await;
    }
}

/// MT-004 class 3/5 -- STALE STATE VECTORS, all six routing policies.
///
/// Arm A supplies the document's own causally EARLIER state vector (the
/// snapshot's, covering update_seq 1) against the seq-2 update -- the literal
/// "stale vector" case. Arm B supplies a genuine server-derived vector that
/// belongs to a DIFFERENT CRDT document, proving the binding is by persisted
/// entity identity and not merely by string shape. Both are real vectors, so
/// neither negative can pass for a formatting reason.
#[tokio::test]
async fn mt004_all_six_policies_reject_stale_crdt_state_vectors() {
    const CASE: &str = "stalevector";
    const LABEL: &str = "mt004-stale";
    const FOREIGN_LABEL: &str = "mt004-stale-foreign";

    let Some((pool, store, db, workspace_id, _crdt_document_id, receipts)) =
        mt004_case_fixture(LABEL).await
    else {
        eprintln!(
            "SKIP mt004_all_six_policies_reject_stale_crdt_state_vectors: PostgreSQL binaries absent"
        );
        return;
    };

    // The snapshot covers update_seq 1; the referenced update is seq 2. Its
    // state vector is therefore genuine, server-derived, and stale.
    let stale_state_vector: String = sqlx::query_scalar(
        "SELECT state_vector FROM kernel_crdt_snapshots WHERE snapshot_bytes_ref = $1",
    )
    .bind(&receipts.snapshot_bytes_ref)
    .fetch_one(&pool)
    .await
    .expect("read the real snapshot's causally earlier state vector");
    assert_ne!(
        stale_state_vector, receipts.post_update_state_vector_after,
        "the pre-update snapshot vector must differ from the post-update vector for a real negative"
    );

    let foreign_document_id = format!("doc-{FOREIGN_LABEL}-{workspace_id}");
    let foreign_crdt_document_id = format!("crdt-{FOREIGN_LABEL}-{workspace_id}");
    let foreign = mt009_seed_real_crdt_document(
        &db,
        &workspace_id,
        &foreign_document_id,
        &foreign_crdt_document_id,
        FOREIGN_LABEL,
    )
    .await;
    assert_ne!(
        foreign.post_update_state_vector_after, receipts.post_update_state_vector_after,
        "distinct CRDT documents must yield distinct state vectors for a real negative"
    );

    let probes = mt004_seed_policy_probes(&store, CASE).await;
    for probe in &probes {
        // Arm A: the document's own stale (pre-update) state vector.
        mt004_assert_crdt_admission_denied(
            &store,
            &pool,
            probe.policy,
            mt004_crdt_probe_message(
                probe,
                CASE,
                "stale",
                receipts.post_update_bytes_ref.clone(),
                receipts.snapshot_bytes_ref.clone(),
                stale_state_vector.clone(),
            ),
            "does not match persisted state_vector_after",
        )
        .await;

        // Arm B: a real state vector belonging to a different document.
        mt004_assert_crdt_admission_denied(
            &store,
            &pool,
            probe.policy,
            mt004_crdt_probe_message(
                probe,
                CASE,
                "foreign",
                receipts.post_update_bytes_ref.clone(),
                receipts.snapshot_bytes_ref.clone(),
                foreign.post_update_state_vector_after.clone(),
            ),
            "does not match persisted state_vector_after",
        )
        .await;
    }
}

/// MT-004 class 4/5 -- DUPLICATES, all six routing policies.
///
/// The duplicate gate lives at the top of `record_message_tx` (~357-384), ahead
/// of CRDT resolution, so it is proven with a real admitted baseline message
/// per policy plus two retries:
///   Arm A: same `idempotency_key`, different payload -> `IdempotencyConflict`.
///   Arm B: same `idempotency_key` AND same payload hash, but the retry adds
///          fabricated CRDT authority fields. An idempotent replay MUST NOT be
///          a smuggling channel for authority the original never carried, so
///          the semantic-hash comparison must reject it instead of returning
///          the stored row.
/// The baseline record also proves these probes fail for the intended reason
/// and not because the message shape itself is inadmissible for that policy.
#[tokio::test]
async fn mt004_all_six_policies_reject_duplicate_idempotency_keys() {
    const CASE: &str = "duplicate";
    const LABEL: &str = "mt004-dup";

    let Some((pool, store, _db, _workspace_id, crdt_document_id, receipts)) =
        mt004_case_fixture(LABEL).await
    else {
        eprintln!(
            "SKIP mt004_all_six_policies_reject_duplicate_idempotency_keys: PostgreSQL binaries absent"
        );
        return;
    };
    let probes = mt004_seed_policy_probes(&store, CASE).await;

    for probe in &probes {
        let policy = probe.policy.as_str();
        let baseline_id = format!("msg-mt004-{CASE}-baseline-{policy}");
        // The mixed-file `sample_message` is `authority=PromotionCandidate,
        // proposal_ref=None`, which `validate_message_authority` (~model_lane.rs
        // 15196) rejects with "proposal_ref is required" before the duplicate
        // gate can run. This test isolates the idempotency-key conflict, so the
        // baseline (and the conflicting retry below) carry a proposal_ref to
        // become admissible without any CRDT authority. No `crdt_*` field is set.
        let mut baseline = sample_message(&baseline_id, &probe.run_id, &probe.lane_id, "local", 2);
        baseline.proposal_ref = Some(format!("proposal://mt004/{CASE}/baseline/{policy}"));
        let baseline_key = baseline.idempotency_key.clone();
        let stored = store
            .record_message_with_payload_binding(
                baseline.clone(),
                sample_artifact_binding_for_message(&baseline),
            )
            .await
            .unwrap_or_else(|error| {
                panic!("policy {policy} baseline message must be admitted: {error}")
            });
        assert_eq!(stored.message_id, baseline_id);

        // Arm A: same idempotency_key, different payload hash.
        let conflicting_id = format!("msg-mt004-{CASE}-conflict-{policy}");
        let mut conflicting =
            sample_message(&conflicting_id, &probe.run_id, &probe.lane_id, "local", 3);
        conflicting.proposal_ref = Some(format!("proposal://mt004/{CASE}/conflict/{policy}"));
        conflicting.idempotency_key = baseline_key.clone();
        assert_ne!(
            conflicting.payload_sha256, baseline.payload_sha256,
            "the conflicting retry must carry a genuinely different payload hash"
        );
        let error = store
            .record_message(conflicting)
            .await
            .expect_err(&format!(
                "policy {policy} must reject a duplicate idempotency_key with a different payload"
            ));
        assert_error_contains(&error, "idempotency conflict");
        assert_error_contains(&error, "already belongs to payload_sha256");
        assert_no_message_row(&pool, &conflicting_id).await;

        // Arm B: byte-identical replay that tries to add CRDT authority.
        let mut smuggling = sample_message(&baseline_id, &probe.run_id, &probe.lane_id, "local", 2);
        assert_eq!(
            smuggling.payload_sha256, baseline.payload_sha256,
            "the smuggling retry must be an otherwise byte-identical idempotent replay"
        );
        // A COMPLETE posture is attached so the retry is rejected by the
        // duplicate gate rather than by the synchronous completeness check --
        // this arm must prove the idempotency path itself refuses the upgrade.
        smuggling.proposal_ref = Some(format!("proposal://mt004/{CASE}/smuggled/{policy}"));
        smuggling.crdt_update_ref = Some(format!(
            "postgres://kernel_crdt_updates/{crdt_document_id}/{policy}-smuggled"
        ));
        smuggling.crdt_base_snapshot_ref = Some(receipts.snapshot_bytes_ref.clone());
        smuggling.crdt_state_vector = Some(receipts.post_update_state_vector_after.clone());
        smuggling.crdt_proposal_ref = Some(format!("crdt-proposal://mt004-{CASE}-smuggled-{policy}"));
        let error = store
            .record_message(smuggling)
            .await
            .expect_err(&format!(
                "policy {policy} must not let an idempotent replay attach CRDT authority"
            ));
        assert_error_contains(&error, "idempotency conflict");
        assert_error_contains(&error, "already belongs to semantic_hash");

        // Exactly one durable row survives both retries, still without CRDT
        // authority.
        let rows: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM model_lane_messages WHERE idempotency_key = $1",
        )
        .bind(&baseline_key)
        .fetch_one(&pool)
        .await
        .expect("count durable rows for the baseline idempotency key");
        assert_eq!(
            rows, 1,
            "policy {policy} must keep exactly one durable row for idempotency_key {baseline_key}"
        );
        // record_json is the durable projection of the stored message record
        // (NewModelLaneMessage is serde-flattened into it), so this reads the
        // authority the row actually kept, not a test-side copy.
        let stored_update_ref: Option<String> = sqlx::query_scalar(
            "SELECT record_json->>'crdt_update_ref' FROM model_lane_messages WHERE message_id = $1",
        )
        .bind(&baseline_id)
        .fetch_one(&pool)
        .await
        .expect("read the stored baseline crdt_update_ref from record_json");
        assert!(
            stored_update_ref.is_none(),
            "policy {policy} baseline row must still carry null CRDT authority after the smuggling retry"
        );
    }
}

/// MT-004 class 5/5 -- REPLAY ORDER, all six routing policies.
///
/// Arm A pairs a real update with a base snapshot that is NOT causally before
/// it (`covered_update_seq >= update_seq`, the resolver's replay-order gate at
/// `model_lane.rs` ~13257), which is how a stale/rewound base would try to
/// re-enter authority. Arm B corrupts the update's persisted replay metadata
/// encoding so the canonical Yjs-v1 replay contract (~13105) no longer holds --
/// a row that cannot be deterministically replayed must never back a
/// ModelLaneMessage.
#[tokio::test]
async fn mt004_all_six_policies_reject_crdt_replay_order_violations() {
    const CASE: &str = "replayorder";
    const LABEL: &str = "mt004-replay";

    let Some((pool, store, _db, _workspace_id, crdt_document_id, receipts)) =
        mt004_case_fixture(LABEL).await
    else {
        eprintln!(
            "SKIP mt004_all_six_policies_reject_crdt_replay_order_violations: PostgreSQL binaries absent"
        );
        return;
    };

    // Arm A fixture: canonical snapshot clone whose covered_update_seq is at or
    // after the referenced update, so it cannot be a causal base for it.
    let non_causal_snapshot_ref =
        format!("postgres://kernel_crdt_snapshots/{crdt_document_id}/{LABEL}-noncausal");
    let inserted = sqlx::query(
        r#"
        INSERT INTO kernel_crdt_snapshots
            (schema_id, snapshot_id, workspace_id, document_id, crdt_document_id, covered_update_seq,
             state_vector, snapshot_sha256, snapshot_bytes_ref, snapshot_bytes, actor_id, actor_kind,
             event_ledger_stream_id, event_ledger_event_id, promotion_evidence_update_ids,
             storage_authority)
        SELECT schema_id, $2, workspace_id, document_id, crdt_document_id, $3,
               state_vector, snapshot_sha256, $4, snapshot_bytes, actor_id, actor_kind,
               $5, event_ledger_event_id, promotion_evidence_update_ids, storage_authority
        FROM kernel_crdt_snapshots
        WHERE snapshot_bytes_ref = $1
        "#,
    )
    .bind(&receipts.snapshot_bytes_ref)
    .bind(format!("{LABEL}-noncausal"))
    .bind(999_i64)
    .bind(&non_causal_snapshot_ref)
    .bind(format!("knowledge-crdt-mt004-noncausal:{crdt_document_id}"))
    .execute(&pool)
    .await
    .expect("INSERT non-causal kernel_crdt_snapshots clone");
    assert_eq!(inserted.rows_affected(), 1);

    // Arm B fixture: canonical update clone whose replay metadata no longer
    // declares the canonical Yjs v1 encoding. Bytes and digest stay valid so the
    // replay-metadata gate is the only reason admission fails.
    let non_replayable_update_ref =
        format!("postgres://kernel_crdt_updates/{crdt_document_id}/{LABEL}-nonreplayable");
    let inserted = sqlx::query(
        r#"
        INSERT INTO kernel_crdt_updates
            (schema_id, workspace_id, document_id, crdt_document_id, update_id, update_seq,
             update_sha256, update_bytes_ref, update_bytes, actor_id, actor_kind, session_id,
             trace_id, state_vector_before, state_vector_after, replay_metadata_json,
             event_ledger_stream_id, event_ledger_event_id, storage_authority)
        SELECT schema_id, workspace_id, document_id, crdt_document_id,
               $2, update_seq + 200000, update_sha256, $3, update_bytes, actor_id, actor_kind,
               session_id, trace_id, state_vector_before, state_vector_after,
               jsonb_set(replay_metadata_json, '{encoding}', '"yjs-update-v0-out-of-order"'),
               $4, event_ledger_event_id, storage_authority
        FROM kernel_crdt_updates
        WHERE update_bytes_ref = $1
        "#,
    )
    .bind(&receipts.post_update_bytes_ref)
    .bind(format!("{LABEL}-nonreplayable"))
    .bind(&non_replayable_update_ref)
    .bind(format!("knowledge-crdt-mt004-nonreplayable:{crdt_document_id}"))
    .execute(&pool)
    .await
    .expect("INSERT non-replayable kernel_crdt_updates clone");
    assert_eq!(inserted.rows_affected(), 1);

    let probes = mt004_seed_policy_probes(&store, CASE).await;
    for probe in &probes {
        // Arm A: base snapshot is not causally before the referenced update.
        mt004_assert_crdt_admission_denied(
            &store,
            &pool,
            probe.policy,
            mt004_crdt_probe_message(
                probe,
                CASE,
                "noncausal",
                receipts.post_update_bytes_ref.clone(),
                non_causal_snapshot_ref.clone(),
                receipts.post_update_state_vector_after.clone(),
            ),
            "is not causally before update_seq",
        )
        .await;

        // Arm B: persisted replay metadata is not canonical Yjs v1.
        mt004_assert_crdt_admission_denied(
            &store,
            &pool,
            probe.policy,
            mt004_crdt_probe_message(
                probe,
                CASE,
                "nonreplayable",
                non_replayable_update_ref.clone(),
                receipts.snapshot_bytes_ref.clone(),
                receipts.post_update_state_vector_after.clone(),
            ),
            "replay metadata is not canonical Yjs v1",
        )
        .await;
    }
}

/// MT-004 structural proof: EVERY stage of EVERY routing policy graph commits
/// its output through the one shared CRDT admission boundary.
///
/// The five class tests above prove the gates. This test proves there is no
/// per-policy or per-stage bypass around them: for each policy returned by
/// `ModelLaneRoutingPolicy::all()` it walks every stage of
/// `ModelLaneRoutingGraph::for_policy`, rebuilds the exact message shape
/// `routing_execution.rs` commits for that stage (advisory, coordinator-target,
/// per-stage kind), poisons it with a fabricated CRDT reference, and requires
/// the shared boundary to deny it.
///
/// Production keeps routing-stage CRDT fields null (`routing_execution.rs`
/// ~1681-1756) precisely because a stage output is advisory; this test proves
/// that if any stage ever started emitting CRDT authority, it could not
/// fabricate it. `CoordinatorJoin` stages commit through
/// `record_context_bundle_artifact_binding_with_validation_tx` rather than a
/// message in production, and are probed here too so a future change that gives
/// them a message cannot land unguarded.
///
/// The drift guard makes a newly added seventh policy fail this test loudly
/// instead of silently going uncovered.
#[tokio::test]
async fn mt004_every_routing_policy_stage_output_routes_through_shared_crdt_admission_boundary() {
    const CASE: &str = "structural";
    const LABEL: &str = "mt004-structural";

    let policies: Vec<ModelLaneRoutingPolicy> =
        ModelLaneRoutingPolicy::all().iter().copied().collect();
    let policy_names: Vec<&str> = policies.iter().map(|policy| policy.as_str()).collect();
    assert_eq!(
        policy_names,
        vec![
            "local_first",
            "cloud_review",
            "cloud_plan_local_execute",
            "parallel_debate",
            "validator_lane",
            "operator_lane",
        ],
        "MT-004 acceptance names exactly these six routing policies; update the MT-004 \
         all-six-policy negative tests before changing the canonical policy set"
    );

    let Some((pool, store, _db, _workspace_id, crdt_document_id, receipts)) =
        mt004_case_fixture(LABEL).await
    else {
        eprintln!(
            "SKIP mt004_every_routing_policy_stage_output_routes_through_shared_crdt_admission_boundary: PostgreSQL binaries absent"
        );
        return;
    };
    let probes = mt004_seed_policy_probes(&store, CASE).await;

    let mut probed_stages = 0usize;
    for probe in &probes {
        let policy = probe.policy.as_str();
        let graph = ModelLaneRoutingGraph::for_policy(probe.policy);
        assert!(
            !graph.stages.is_empty(),
            "policy {policy} must declare at least one executable stage"
        );
        for stage in &graph.stages {
            let stage_id = stage.stage_id.as_str();
            let message_id = format!("routing-output:mt004-{CASE}:{policy}:{stage_id}:1");
            let mut message =
                sample_message(&message_id, &probe.run_id, &probe.lane_id, "local", 2);
            // Mirror routing_execution.rs commit_stage_output message shaping.
            message.kind = if stage_id == "cloud-review" {
                ModelLaneMessageKind::Critique
            } else if stage.target == ModelLaneRoutingDispatchTarget::CoordinatorJoin {
                ModelLaneMessageKind::Status
            } else {
                ModelLaneMessageKind::Proposal
            };
            message.authority = ModelLaneAuthority::Advisory;
            message.promotion_decision_id = None;
            message.promotion_gate_ref = None;
            message.promotion_receipt_ref = None;
            message.promoted_artifact_ref = None;
            message.promoted_artifact_sha256 = None;
            message.promoted_artifact_version = None;
            message.idempotency_key = message_id.clone();
            message.replay_order_key = format!("routing/mt004-{CASE}/{policy}/{stage_id}/0001");
            // Fabricated but COMPLETE authority posture: the synchronous
            // completeness check (~15137) and the Proposal-kind precondition
            // (~13737) are both satisfied, so every stage of every policy fails
            // at the same durable resolution gate for the same reason.
            message.proposal_ref =
                Some(format!("proposal://mt004/{CASE}/{policy}/{stage_id}"));
            message.crdt_update_ref = Some(format!(
                "postgres://kernel_crdt_updates/{crdt_document_id}/{policy}-{stage_id}-fabricated"
            ));
            message.crdt_base_snapshot_ref = Some(receipts.snapshot_bytes_ref.clone());
            message.crdt_state_vector = Some(receipts.post_update_state_vector_after.clone());
            message.crdt_proposal_ref =
                Some(format!("crdt-proposal://mt004-{CASE}-{policy}-{stage_id}"));

            mt004_assert_crdt_admission_denied(
                &store,
                &pool,
                probe.policy,
                message,
                "does not resolve to kernel_crdt_updates",
            )
            .await;
            probed_stages += 1;
        }
    }

    assert_eq!(
        probed_stages, 13,
        "the six canonical routing graphs declare 13 stages in total; a changed stage set must \
         be re-reviewed against the MT-004 all-six-policy negative coverage"
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
            ModelLaneRecoveryEventKind::PayloadRefObserved,
            1,
            Some(after_checkpoint.payload_ref.clone()),
            None,
            None,
            None,
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
async fn recovery_reconciles_current_post_checkpoint_leases_without_moving_replay_bound() {
    let (pool, store) = model_lane_store().await;
    let run_id = "run-mt017-post-checkpoint-lease-reconciliation";
    let checkpoint_lane_id = "lane-mt017-checkpoint";
    let post_checkpoint_lane_id = "lane-mt017-post-checkpoint";
    let active_lease_id = "lease-mt017-post-checkpoint-active";
    let expired_lease_id = "lease-mt017-post-checkpoint-expired";
    seed_run_lane(&store, run_id, checkpoint_lane_id, RuntimeBinding::Local).await;
    record_checkpoint_at_highwater(
        &pool,
        &store,
        run_id,
        checkpoint_lane_id,
        None,
        vec![],
        "checkpoint-mt017-before-new-lane-leases",
    )
    .await;

    store
        .record_lane(sample_lane(
            post_checkpoint_lane_id,
            run_id,
            ModelLaneKind::LocalModel,
            RuntimeBinding::Local,
            LaunchAuthority::ModelRuntime,
        ))
        .await
        .expect("record lane after checkpoint without moving replay watermark");
    store
        .record_lane_lease(sample_lease(
            active_lease_id,
            run_id,
            post_checkpoint_lane_id,
            "2099-01-01T00:00:00Z",
            ModelLaneLeaseState::Active,
        ))
        .await
        .expect("record active post-checkpoint lease");
    store
        .record_lane_lease(sample_lease(
            expired_lease_id,
            run_id,
            post_checkpoint_lane_id,
            "2020-01-01T00:00:00Z",
            ModelLaneLeaseState::Active,
        ))
        .await
        .expect("record expired post-checkpoint lease");

    let first = store
        .recover_run_after_restart(run_id)
        .await
        .expect("reconcile current leases independently of checkpoint replay");
    assert!(
        first
            .replay
            .lanes
            .iter()
            .all(|lane| lane.lane_id != post_checkpoint_lane_id),
        "post-checkpoint lane must not widen the checkpoint replay watermark"
    );
    assert_eq!(
        first.checkpoint.last_event_ledger_seq,
        first
            .replay
            .run
            .event_ledger_seq
            .max(first.checkpoint.last_event_ledger_seq)
    );
    assert!(
        first
            .active_leases
            .iter()
            .any(|lease| lease.lease_id == active_lease_id
                && lease.lane_id.as_deref() == Some(post_checkpoint_lane_id)),
        "active post-checkpoint ownership must be visible during restart"
    );
    assert!(
        first
            .reclaimable_lease_ids
            .iter()
            .any(|lease_id| lease_id == expired_lease_id),
        "expired post-checkpoint ownership must be reclaimable"
    );
    let first_orphan = first
        .recovery_events
        .iter()
        .find(|event| event.lease_id.as_deref() == Some(expired_lease_id))
        .expect("expired lease produces an orphan recovery event");
    assert_eq!(
        first_orphan.model_session_id.as_deref(),
        Some("model-session-lane-mt017-post-checkpoint"),
        "orphan evidence must attribute the post-checkpoint lease lane"
    );

    let second = store
        .recover_run_after_restart(run_id)
        .await
        .expect("repeated recovery is idempotent");
    assert!(second
        .active_leases
        .iter()
        .any(|lease| lease.lease_id == active_lease_id));
    assert!(second
        .reclaimable_lease_ids
        .iter()
        .any(|lease_id| lease_id == expired_lease_id));
    let orphan_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger \
         WHERE aggregate_type = 'model_lane_recovery_event' \
           AND session_run_id = $1 \
           AND payload->'record'->>'lease_id' = $2 \
           AND payload->'record'->>'event_kind' = 'orphan_detected'",
    )
    .bind(event_stream_id(run_id))
    .bind(expired_lease_id)
    .fetch_one(&pool)
    .await
    .expect("count idempotent post-checkpoint orphan evidence");
    assert_eq!(
        orphan_rows, 1,
        "repeated recovery must not duplicate orphan EventLedger evidence"
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
        "run-mt009-fabricated-crdt",
        "lane-mt009-fabricated-crdt",
        RuntimeBinding::Local,
    )
    .await;
    let mut fabricated_crdt = sample_message(
        "msg-mt009-fabricated-crdt",
        "run-mt009-fabricated-crdt",
        "lane-mt009-fabricated-crdt",
        "local",
        1,
    );
    fabricated_crdt.crdt_update_ref =
        Some("postgres://kernel_crdt_updates/fabricated/update-001".into());
    fabricated_crdt.crdt_base_snapshot_ref =
        Some("postgres://kernel_crdt_snapshots/fabricated/snapshot-001".into());
    fabricated_crdt.crdt_state_vector = Some("hsk-sv1:ZmFicmljYXRlZA==".into());
    let fabricated_crdt_err = store
        .record_message(fabricated_crdt)
        .await
        .expect_err("well-formed but nonexistent CRDT refs must fail closed");
    assert_error_contains(&fabricated_crdt_err, "CRDT authority resolution failed");
    assert_no_message_row(&pool, "msg-mt009-fabricated-crdt").await;
    let fabricated_event_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM kernel_event_ledger
        WHERE aggregate_type = 'model_lane_message'
          AND aggregate_id = $1
        "#,
    )
    .bind("msg-mt009-fabricated-crdt")
    .fetch_one(&pool)
    .await
    .expect("count rejected fabricated CRDT EventLedger rows");
    assert_eq!(
        fabricated_event_count, 0,
        "rejected CRDT metadata must not append a successful message event"
    );

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
        let recorded = store
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
        if replay_order_seq == 3 {
            sqlx::query(
                r#"
                UPDATE model_lane_recovery_events
                SET replay_order_seq = 3,
                    record_json = jsonb_set(record_json, '{replay_order_seq}', '3'::jsonb)
                WHERE recovery_event_id = $1
                "#,
            )
            .bind(&recorded.recovery_event_id)
            .execute(&pool)
            .await
            .expect("corrupt mutable replay sequence to create a gap");
            sqlx::query(
                r#"
                UPDATE kernel_event_ledger
                SET payload = jsonb_set(payload, '{record,replay_order_seq}', '3'::jsonb)
                WHERE event_id = $1
                "#,
            )
            .bind(&recorded.event_ledger_event_id)
            .execute(&pool)
            .await
            .expect("corrupt EventLedger replay sequence to the same gap");
        }
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
    seed_cloud_authority_for_model_session(
        store,
        run_id,
        lane_id,
        &format!("model-session-{lane_id}"),
        "model://mt009/cloud/openai/gpt-4o-mini",
    )
    .await;
}

async fn seed_cloud_authority_for_model_session(
    store: &ModelLaneStore,
    run_id: &str,
    lane_id: &str,
    model_session_id: &str,
    requested_model_id: &str,
) {
    let mut projection_plan = sample_projection_plan(run_id, lane_id);
    projection_plan.model_session_id = Some(model_session_id.to_string());
    projection_plan.requested_model_id = Some(requested_model_id.to_string());
    let plan = store
        .record_cloud_projection_plan(projection_plan)
        .await
        .expect("record cloud ProjectionPlan authority");
    let mut consent_receipt = sample_consent_receipt(
        run_id,
        lane_id,
        &plan.projection_plan_id,
        &plan.projection_plan_hash,
    );
    consent_receipt.model_session_id = Some(model_session_id.to_string());
    consent_receipt.requested_model_id = Some(requested_model_id.to_string());
    store
        .record_cloud_consent_receipt(consent_receipt)
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

fn sample_subagent_launch_request(run_id: &str, lane_id: &str) -> DexterityLaunchAdapterRequest {
    let run = sample_run(run_id, vec![lane_id.to_owned()]);
    let lane = sample_lane(
        lane_id,
        run_id,
        ModelLaneKind::Subagent,
        RuntimeBinding::Subagent,
        LaunchAuthority::SubagentManager,
    );
    let locus = lane
        .locus_binding
        .clone()
        .expect("MT-009 subagent launch carries Locus binding");
    DexterityLaunchAdapterRequest {
        adapter_kind: DexterityLaunchAdapterKind::Subagent,
        run_id: run.run_id,
        lane_id: lane.lane_id,
        trace_id: run.trace_id,
        run_span_id: run.run_span_id,
        lane_span_id: lane.lane_span_id,
        coordinator_session_id: run.coordinator_session_id,
        routing_policy: run.routing_policy,
        context_bundle_id: run.context_bundle_id,
        event_ledger_stream_id: run.event_ledger_stream_id,
        artifact_namespace: run.artifact_namespace,
        work_packet_id: run.work_packet_id,
        micro_task_id: run.micro_task_id,
        task_board_id: run.task_board_id,
        owner_session: run.owner_session,
        locus_binding_ref: locus.locus_binding_ref,
        role: lane.role,
        backend: Some(lane.backend),
        adapter_id: Some(lane.adapter_id),
        model_id: lane.model_id,
        session_id: lane.session_id,
        model_session_id: lane.model_session_id,
        extra_capability_token_ids: lane.capability_token_ids,
        requested_tool_capability_tokens: vec!["tool-capability://read-context".into()],
        effective_capability_snapshot_ref: lane.effective_capability_snapshot_ref,
        capability_negotiation_ref: lane.capability_negotiation_ref,
        provider_feature_profile_ref: lane.provider_feature_profile_ref,
        requested_execution_policy_ref: lane.requested_execution_policy_ref,
        effective_execution_policy_ref: lane.effective_execution_policy_ref,
        projection_plan_ref: lane.projection_plan_ref,
        consent_receipt_ref: lane.consent_receipt_ref,
        tool_gate_decision_refs: lane.tool_gate_decision_refs,
        status: Some(lane.status),
        heartbeat_at_utc: lane.heartbeat_at_utc,
        lease_expires_at_utc: lane.lease_expires_at_utc,
        reclaim_after_utc: lane.reclaim_after_utc,
        restart_generation: lane.restart_generation,
        cancellation_ref: lane.cancellation_ref,
        reclaim_policy_ref: lane.reclaim_policy_ref,
        terminal_status_mapping_ref: lane.terminal_status_mapping_ref,
        process_ownership_ref: None,
        no_os_process_reason_ref: None,
        backpressure_ref: lane.backpressure_ref,
        loop_counter_ref: lane.loop_counter_ref,
        last_runtime_status_ref: lane.last_runtime_status_ref,
        last_recovery_event_ref: lane.last_recovery_event_ref,
        startup_failure_code: lane.failstate_code,
        startup_failure_ref: lane.startup_failure_ref,
        reason_ref: lane.reason_ref,
        run_recovery_hint_ref: run.recovery_hint_ref,
        lane_recovery_hint_ref: lane.recovery_hint_ref,
        memory_pack_ref: run.memory_pack_ref,
        memory_pack_hash: run.memory_pack_hash,
        determinism_mode: run.determinism_mode,
        budget_summary_ref: run.budget_summary_ref,
        selected_model_id: run.selected_model_id,
        candidate_model_ids: run.candidate_model_ids,
        procedural_review_status: run.procedural_review_status,
        truncation_warning_ref: run.truncation_warning_ref,
        rejection_reason_refs: run.rejection_reason_refs,
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
    let locus_ref = format!("locus://wp1/mt009/{run_id}/{lane_id}/{message_id}");
    let payload_json =
        artifact_payload_json_parts(message_id, run_id, &payload_ref, "", &locus_ref);
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
        proposal_ref: None,
        crdt_update_ref: None,
        crdt_base_snapshot_ref: None,
        crdt_state_vector: None,
        crdt_proposal_ref: None,
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
        lane_id: Some(lane_id.into()),
        model_session_id: Some(format!("model-session-{lane_id}")),
        provider_kind: Some("openai".into()),
        requested_model_id: Some("model://mt009/cloud/openai/gpt-4o-mini".into()),
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
        target_bindings: vec![],
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
        lane_id: Some(lane_id.into()),
        model_session_id: Some(format!("model-session-{lane_id}")),
        provider_kind: Some("openai".into()),
        requested_model_id: Some("model://mt009/cloud/openai/gpt-4o-mini".into()),
        scope_hash: sample_scope_hash(),
        consent_scope: ModelLaneCloudConsentScope::SingleLane,
        target_bindings: vec![],
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
        revocation_input_hash: None,
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
struct Ac9ProductionRuntime {
    capabilities: ModelCapabilities,
    kv: KvCacheHandle,
    lora: LoraStackHandle,
    steering: SteeringHookHandle,
    hold_generation: Arc<AtomicBool>,
    model_outputs: Arc<Mutex<HashMap<ModelId, String>>>,
}

impl Ac9ProductionRuntime {
    fn new(
        hold_generation: Arc<AtomicBool>,
        model_outputs: Arc<Mutex<HashMap<ModelId, String>>>,
    ) -> Self {
        Self {
            capabilities: ModelCapabilities::default(),
            kv: KvCacheHandle::new("ac9-production-kv"),
            lora: LoraStackHandle::new("ac9-production-lora"),
            steering: SteeringHookHandle::new("ac9-production-steering"),
            hold_generation,
            model_outputs,
        }
    }
}

#[async_trait]
impl ModelRuntime for Ac9ProductionRuntime {
    async fn load(&mut self, _spec: LoadSpec) -> Result<ModelId, ModelRuntimeError> {
        Ok(ModelId::new_v7())
    }

    async fn unload(&mut self, _id: ModelId) -> Result<(), ModelRuntimeError> {
        Ok(())
    }

    fn generate(&self, req: GenerateRequest) -> TokenStream {
        if self.hold_generation.load(Ordering::SeqCst) {
            return Box::pin(stream::unfold(
                (req.cancel, self.hold_generation.clone(), false),
                |(cancel, hold_generation, terminal_emitted)| async move {
                    if terminal_emitted {
                        return None;
                    }
                    tokio::time::sleep(Duration::from_millis(25)).await;
                    if cancel.is_cancelled() {
                        Some((
                            Err(ModelRuntimeError::Cancelled),
                            (cancel, hold_generation, true),
                        ))
                    } else if !hold_generation.load(Ordering::SeqCst) {
                        Some((
                            Ok(GeneratedToken {
                                token_id: 1,
                                text: "AC-9 released held proposal".into(),
                                logprob: None,
                                finish_reason: None,
                            }),
                            (cancel, hold_generation, true),
                        ))
                    } else {
                        Some((
                            Ok(GeneratedToken {
                                token_id: 0,
                                text: String::new(),
                                logprob: None,
                                finish_reason: None,
                            }),
                            (cancel, hold_generation, false),
                        ))
                    }
                },
            ));
        }
        let forced = self
            .model_outputs
            .lock()
            .expect("AC-9 model-output lock")
            .get(&req.id)
            .cloned();
        let text = if let Some(forced) = forced {
            forced
        } else if req.prompt.text.contains("output_contract:") {
            r#"{"verdict":"accept","review":"AC-9 typed cloud review"}"#.to_string()
        } else {
            format!("AC-9 proposal for {}", req.id)
        };
        Box::pin(stream::iter(vec![Ok(GeneratedToken {
            token_id: 1,
            text,
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

struct Ac9ProductionFactory {
    ledger: LedgerBatcher,
    creates: Arc<AtomicUsize>,
    teardowns: Arc<AtomicUsize>,
    hold_generation: Arc<AtomicBool>,
    hold_create: Arc<AtomicBool>,
    fail_model: Arc<Mutex<Option<ModelId>>>,
    model_outputs: Arc<Mutex<HashMap<ModelId, String>>>,
    held_models: Arc<Mutex<HashSet<ModelId>>>,
}

#[async_trait]
impl ModelSessionFactory for Ac9ProductionFactory {
    async fn create(&self, request: &SpawnRequest) -> Result<LiveSession, SwarmError> {
        self.creates.fetch_add(1, Ordering::SeqCst);
        while self.hold_create.load(Ordering::SeqCst)
            || self
                .held_models
                .lock()
                .expect("AC-9 held-model lock")
                .contains(&request.instance_id.model_id)
        {
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        if self
            .fail_model
            .lock()
            .expect("AC-9 fail-model lock")
            .as_ref()
            == Some(&request.instance_id.model_id)
        {
            return Err(SwarmError::FactoryFailed(format!(
                "injected AC-9 factory failure for {}",
                request.instance_id.model_id
            )));
        }
        let record_id = ProcessOwnershipRecordId::new_v7();
        let os_pid = 61_000 + request.instance_id.instance;
        let engine = if request.provider == Some(ProviderKind::ByokCloud) {
            ProcessEngineKind::HelperSubprocess
        } else {
            ProcessEngineKind::Candle
        };
        let start = ProcessStart::new(engine, request.owner_role.clone(), request.owner_wp.clone())
            .with_process_uuid(record_id.as_uuid())
            .with_os_pid(os_pid)
            .with_parent_session_id(request.parent_session_id.clone())
            .with_wp_id(request.wp_id.clone().unwrap_or_default())
            .with_mt_id(request.mt_id.clone().unwrap_or_default());
        self.ledger
            .record_start(start.clone())
            .map_err(|error| SwarmError::LedgerFailed(error.to_string()))?;
        let teardown_count = self.teardowns.clone();
        let teardown: handshake_core::swarm_orchestration::SessionTeardown = Arc::new(move || {
            let teardown_count = teardown_count.clone();
            Box::pin(async move {
                teardown_count.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
        });
        Ok(LiveSession::new(
            Arc::new(Ac9ProductionRuntime::new(
                self.hold_generation.clone(),
                self.model_outputs.clone(),
            )),
            request.instance_id.model_id,
            CancellationToken::new(),
            teardown,
            record_id,
            os_pid,
        )
        .with_ledger_start(engine, start))
    }
}

#[derive(Clone)]
struct Ac9StageSpec {
    stage_id: String,
    target: ModelLaneRoutingDispatchTarget,
    lane_id: Option<String>,
    authority_lane_id: Option<String>,
    instance_id: Option<ModelInstanceId>,
}

struct Ac9ProductionFixture {
    pool: PgPool,
    store: ModelLaneStore,
    coordinator: SwarmCoordinator,
    policy: ModelLaneRoutingPolicy,
    execution_id: String,
    decision_id: String,
    context: ModelLaneRoutingExecutionContext,
    authority: ModelLaneRoutingAuthority,
    specs: Vec<Ac9StageSpec>,
    creates: Arc<AtomicUsize>,
    teardowns: Arc<AtomicUsize>,
    hold_generation: Arc<AtomicBool>,
    hold_create: Arc<AtomicBool>,
    fail_model: Arc<Mutex<Option<ModelId>>>,
    model_outputs: Arc<Mutex<HashMap<ModelId, String>>>,
    held_models: Arc<Mutex<HashSet<ModelId>>>,
    ledger_drain: ProcessLedgerDrain,
}

impl Ac9ProductionFixture {
    fn launches(&self) -> Vec<ModelLaneRoutingStageLaunch> {
        self.specs
            .iter()
            .map(|spec| {
                let request = spec.instance_id.map(|instance_id| {
                    let mut request = SpawnRequest::new(
                        instance_id,
                        RuntimeAdapterBinding::Candle,
                        OWNER,
                        self.context.coordinator_session_id.clone(),
                    )
                    .with_wp(WP_ID)
                    .with_mt(MT_ID);
                    request.owner_wp = Some(WP_ID.into());
                    if spec.target == ModelLaneRoutingDispatchTarget::CloudModel {
                        request = request
                            .with_cloud_provider(ProviderKind::ByokCloud, "gpt-4o-mini")
                            .with_byok_cloud_provider(ByokCloudProvider::OpenAi);
                    } else {
                        request = request.with_local_artifact("ac9-model.gguf", sample_sha256());
                    }
                    let mut contract = DexterityLaunchContract::from_spawn_request(&request)
                        .expect("construct real AC-9 Dexterity launch contract");
                    let lane_id = spec.lane_id.clone().expect("model stage has planned lane");
                    contract.run_id = self.context.run_id.clone();
                    contract.lane_id = lane_id.clone();
                    contract.trace_id = self.context.trace_id.clone();
                    contract.run_span_id = self.context.run_span_id.clone();
                    contract.lane_span_id = format!("span-{lane_id}");
                    contract.routing_policy = "mixed_local_cloud_subagent".into();
                    contract.context_bundle_id = format!("ctx-{}", self.context.run_id);
                    contract.event_ledger_stream_id = event_stream_id(&self.context.run_id);
                    contract.artifact_namespace =
                        format!("artifact://model-lane/mt009/{}", self.context.run_id);
                    contract.task_board_id = TASK_BOARD_ID.into();
                    contract.locus_binding_ref = self.context.locus_ref.clone();
                    contract.memory_pack_ref =
                        format!("memory-pack://fems/mt009/{}", self.context.run_id);
                    contract.memory_pack_hash = sample_sha256();
                    contract.determinism_mode = "deterministic_replay".into();
                    contract.budget_summary_ref = "budget://mt009/mixed-runtime".into();
                    contract.candidate_model_ids =
                        if spec.target == ModelLaneRoutingDispatchTarget::CloudModel {
                            vec!["model://dexterity/byok_cloud/gpt-4o-mini".into()]
                        } else {
                            vec![instance_id.model_id.to_string()]
                        };
                    contract.procedural_review_status = "reviewed_by_kernel_builder".into();
                    contract.projection_plan_ref = (spec.target
                        == ModelLaneRoutingDispatchTarget::CloudModel)
                        .then(|| projection_plan_id(&self.context.run_id, &lane_id));
                    contract.consent_receipt_ref = (spec.target
                        == ModelLaneRoutingDispatchTarget::CloudModel)
                        .then(|| consent_receipt_id(&self.context.run_id, &lane_id));
                    request.with_dexterity_launch(contract)
                });
                let generate_request = request.as_ref().map(|request| GenerateRequest {
                    id: request.instance_id.model_id,
                    prompt: GenPrompt::new(format!("execute canonical stage {}", spec.stage_id)),
                    sampling: SamplingParams::default(),
                    lora_overrides: Vec::new(),
                    steering_overrides: Vec::new(),
                    kv_prefix_handle: None,
                    cancel: CancellationToken::new(),
                    max_tokens: 16,
                    stop_sequences: Vec::new(),
                    speculative_mode: None,
                    structured_decoding: None,
                });
                ModelLaneRoutingStageLaunch {
                    stage_id: spec.stage_id.clone(),
                    expected_run_id: self.context.run_id.clone(),
                    expected_lane_id: spec.lane_id.clone().unwrap_or_default(),
                    expected_model_id: spec
                        .instance_id
                        .map(|instance| instance.model_id.to_string())
                        .unwrap_or_default(),
                    expected_provider: request.as_ref().and_then(|request| request.provider),
                    request,
                    generate_request,
                    authority_lane_id: spec.authority_lane_id.clone(),
                }
            })
            .collect()
    }

    async fn wave(&self) -> Result<ModelLaneRoutingDispatchBatch, SwarmError> {
        execute_production_routing_wave(
            &self.coordinator,
            &self.execution_id,
            &self.decision_id,
            &self.authority,
            self.context.clone(),
            self.launches(),
        )
        .await
    }

    async fn lifecycle(&self) -> Result<ModelLaneRoutingDispatchBatch, SwarmError> {
        execute_production_routing_lifecycle(
            &self.coordinator,
            &self.execution_id,
            &self.decision_id,
            &self.authority,
            self.context.clone(),
            self.launches(),
        )
        .await
    }

    async fn complete_waiting_authority(
        &self,
        batch: &ModelLaneRoutingDispatchBatch,
    ) -> Option<ModelLaneRoutingDispatchBatch> {
        let stage = batch
            .execution
            .stages
            .values()
            .find(|stage| stage.state == ModelLaneRoutingStageStateKind::AwaitingAuthority)?;
        let authority_lane_id = stage.lane_id.clone().expect("authority stage stores lane");
        let request_message_ref = stage
            .authority_request_message_ref
            .clone()
            .expect("authority stage stores causal request");
        let message_id = format!(
            "ac9-authority-response:{}:{}:{}",
            self.execution_id, stage.stage_id, stage.attempt
        );
        let mut response = sample_message(
            &message_id,
            &self.context.run_id,
            &authority_lane_id,
            "authority",
            90 + i64::from(stage.attempt),
        );
        response.kind = ModelLaneMessageKind::Status;
        response.authority = if stage.dispatch_target == ModelLaneRoutingDispatchTarget::Validator {
            ModelLaneAuthority::ValidatorVerdict
        } else {
            ModelLaneAuthority::OperatorDecision
        };
        response.promotion_decision_id = None;
        response.promotion_gate_ref = None;
        response.promotion_receipt_ref = None;
        response.promoted_artifact_ref = None;
        response.promoted_artifact_sha256 = None;
        response.promoted_artifact_version = None;
        response.proposal_ref = None;
        response.crdt_update_ref = None;
        response.crdt_base_snapshot_ref = None;
        response.crdt_state_vector = None;
        response.crdt_proposal_ref = None;
        response.crdt_stale_base_ref = None;
        response.validator_verdict_ref = (stage.dispatch_target
            == ModelLaneRoutingDispatchTarget::Validator)
            .then(|| stage.authority_ref.clone())
            .flatten();
        response.operator_decision_ref = (stage.dispatch_target
            == ModelLaneRoutingDispatchTarget::Operator)
            .then(|| stage.authority_ref.clone())
            .flatten();
        response.routing = Some(ModelLaneRoutingMetadata {
            target_role: "coordinator".into(),
            target_session: self.context.coordinator_session_id.clone(),
            correlation_id: format!("routing:{}:{}", self.execution_id, stage.stage_id),
            requires_ack: false,
            ack_for: Some(request_message_ref),
        });
        response.diagnostic_payload = json!({
            "schema_id": "hsk.model_lane_routing_authority_response@1",
            "execution_id": self.execution_id,
            "stage_id": stage.stage_id,
            "attempt": stage.attempt,
            "verdict": "approve",
        });
        response.payload_sha256 = sha256_hex(&canonical_json_bytes(
            &artifact_payload_json_for_message(&response),
        ));
        let stored = self
            .store
            .record_message_with_payload_binding(
                response.clone(),
                sample_artifact_binding_for_message(&response),
            )
            .await
            .expect("record real typed authority response and payload binding");
        Some(
            self.coordinator
                .complete_authority_and_resume_routing_lifecycle(
                    &self.execution_id,
                    &stage.stage_id,
                    &stored.message_id,
                    self.launches(),
                )
                .await
                .expect("complete authority and auto-resume production lifecycle"),
        )
    }
}

async fn ac9_fixture(policy: ModelLaneRoutingPolicy, suffix: &str) -> Ac9ProductionFixture {
    let (pool, store) = model_lane_store().await;
    let run_id = format!("run-ac9-{suffix}-{}", policy.as_str());
    let execution_id = format!("execution-ac9-{suffix}-{}", policy.as_str());
    let decision_id = format!("decision-ac9-{suffix}-{}", policy.as_str());
    let source_lane_id = format!("lane-ac9-{suffix}-source");
    let source_message_id = format!("message-ac9-{suffix}-source");
    let graph = ModelLaneRoutingGraph::for_policy(policy);
    let mut initial_lane_ids = vec![source_lane_id.clone()];
    let mut specs = Vec::new();
    let mut cloud_receipt_ref = None;
    for (index, stage) in graph.stages.iter().enumerate() {
        let (lane_id, authority_lane_id, instance_id) = match stage.target {
            ModelLaneRoutingDispatchTarget::LocalModel
            | ModelLaneRoutingDispatchTarget::CloudModel => {
                let lane_id = format!("lane-ac9-{suffix}-{}", stage.stage_id);
                let instance_id = ModelInstanceId::new(ModelId::new_v7(), index as u32);
                if stage.target == ModelLaneRoutingDispatchTarget::CloudModel {
                    let authority_request = SpawnRequest::new(
                        instance_id,
                        RuntimeAdapterBinding::Candle,
                        OWNER,
                        format!("coordinator-{run_id}"),
                    );
                    seed_cloud_authority_for_model_session(
                        &store,
                        &run_id,
                        &lane_id,
                        &dexterity_spawn_model_session_id(&authority_request),
                        "model://dexterity/byok_cloud/gpt-4o-mini",
                    )
                    .await;
                    cloud_receipt_ref = Some(consent_receipt_id(&run_id, &lane_id));
                }
                (Some(lane_id), None, Some(instance_id))
            }
            ModelLaneRoutingDispatchTarget::Validator => {
                let lane_id = format!("lane-ac9-{suffix}-validator");
                initial_lane_ids.push(lane_id.clone());
                (None, Some(lane_id), None)
            }
            ModelLaneRoutingDispatchTarget::Operator => {
                let lane_id = format!("lane-ac9-{suffix}-operator");
                initial_lane_ids.push(lane_id.clone());
                (None, Some(lane_id), None)
            }
            ModelLaneRoutingDispatchTarget::CoordinatorJoin => (None, None, None),
        };
        specs.push(Ac9StageSpec {
            stage_id: stage.stage_id.clone(),
            target: stage.target,
            lane_id,
            authority_lane_id,
            instance_id,
        });
    }
    store
        .record_run(sample_run(&run_id, initial_lane_ids.clone()))
        .await
        .expect("record AC-9 canonical ModelLaneRun");
    store
        .record_lane(sample_lane(
            &source_lane_id,
            &run_id,
            ModelLaneKind::LocalModel,
            RuntimeBinding::Local,
            LaunchAuthority::ModelRuntime,
        ))
        .await
        .expect("record AC-9 source lane");
    for spec in &specs {
        let Some(authority_lane_id) = spec.authority_lane_id.as_deref() else {
            continue;
        };
        let (kind, binding, launch_authority) =
            if spec.target == ModelLaneRoutingDispatchTarget::Validator {
                (
                    ModelLaneKind::Validator,
                    RuntimeBinding::Validator,
                    LaunchAuthority::ValidatorRunner,
                )
            } else {
                (
                    ModelLaneKind::HumanOperator,
                    RuntimeBinding::Human,
                    LaunchAuthority::Operator,
                )
            };
        store
            .record_lane(sample_lane(
                authority_lane_id,
                &run_id,
                kind,
                binding,
                launch_authority,
            ))
            .await
            .expect("record AC-9 authority lane");
    }
    let source_message = sample_message(&source_message_id, &run_id, &source_lane_id, "local", 1);
    let source_record = store
        .record_message_with_payload_binding(
            source_message.clone(),
            sample_artifact_binding_for_message(&source_message),
        )
        .await
        .expect("record AC-9 selected input message and payload binding");
    let selected_ref = format!("model-lane-message://{}", source_record.message_id);
    // Promotion authority is independent of whether the routing graph itself
    // contains a validator stage. Every fixture below records an Approved
    // selecting decision, so all six policies require an explicit promotion
    // authority reference before their production wave can launch.
    let validator_ref = Some(format!("validator://ac9/{suffix}"));
    let operator_ref = (policy == ModelLaneRoutingPolicy::OperatorLane)
        .then(|| format!("operator://ac9/{suffix}"));
    let mut diagnostic_payload = json!({"fixture": "ac9-production-matrix"});
    if let Some(receipt) = cloud_receipt_ref.as_ref() {
        diagnostic_payload["cloud_consent_receipt_ref"] = json!(receipt);
    }
    let decision = store
        .record_promotion_decision(NewModelLanePromotionDecision {
            decision_id: decision_id.clone(),
            run_id: run_id.clone(),
            trace_id: format!("trace-{run_id}"),
            decision_span_id: format!("span-{decision_id}"),
            parent_span_id: Some(source_record.message_span_id.clone()),
            linked_span_contexts: vec![source_record.message_span_id.clone()],
            coordinator_session_id: format!("coordinator-{run_id}"),
            routing_policy: policy,
            routing_launch_plan: specs
                .iter()
                .map(|spec| {
                    handshake_core::swarm_orchestration::routing::ModelLaneRoutingStageLaunchPlan {
                        stage_id: spec.stage_id.clone(),
                        dispatch_target: spec.target,
                        lane_id: spec
                            .lane_id
                            .clone()
                            .or_else(|| spec.authority_lane_id.clone()),
                        model_id: spec
                            .instance_id
                            .map(|instance| instance.model_id.to_string()),
                        provider: (spec.target == ModelLaneRoutingDispatchTarget::CloudModel)
                            .then_some(ProviderKind::ByokCloud),
                    }
                })
                .collect(),
            input_refs: vec![selected_ref.clone()],
            selected_input_refs: vec![selected_ref.clone()],
            rejected_input_refs: Vec::new(),
            validator_authority_ref: validator_ref.clone(),
            operator_authority_ref: operator_ref.clone(),
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
                .expect("source message state vector"),
            current_state_vector: source_record
                .crdt_state_vector
                .clone()
                .expect("source message current state vector"),
            schema_id: "hsk.model_lane_message@1".into(),
            deterministic_tie_break_rule: "event_ledger_seq_then_message_id".into(),
            promotion_gate_ref: format!("promotion-gate://ac9/{suffix}"),
            promotion_receipt_ref: Some(format!("promotion-receipt://ac9/{suffix}")),
            promoted_artifact_ref: Some(format!("artifact://promoted/ac9/{suffix}")),
            promoted_artifact_sha256: Some(sample_sha256()),
            promoted_artifact_version: Some("1".into()),
            direct_authority_mutation_attempt_ref: None,
            event_ledger_stream_id: event_stream_id(&run_id),
            work_packet_id: Some(WP_ID.into()),
            micro_task_id: Some(MT_ID.into()),
            task_board_id: Some(TASK_BOARD_ID.into()),
            owner_session: OWNER.into(),
            idempotency_key: format!("idem-{decision_id}"),
            replay_order_key: format!("00000090/promotion/{decision_id}"),
            recovery_hint_ref: Some("usermanual://model-lane-validation-harness#recovery".into()),
            created_at_utc: "2026-07-14T00:00:00Z".into(),
            diagnostic_payload,
        })
        .await
        .expect("record approved AC-9 selecting promotion decision");
    assert_eq!(decision.outcome, ModelLanePromotionOutcome::Approved);
    let locus_ref = format!("locus://wp1/mt009/{run_id}/coordinator-{run_id}");
    let context = ModelLaneRoutingExecutionContext {
        run_id: run_id.clone(),
        trace_id: format!("trace-{run_id}"),
        run_span_id: format!("span-{run_id}"),
        coordinator_session_id: format!("coordinator-{run_id}"),
        locus_ref,
        work_packet_id: WP_ID.into(),
        micro_task_id: Some(MT_ID.into()),
        task_board_id: TASK_BOARD_ID.into(),
        owner_session: OWNER.into(),
        initial_input_ref: selected_ref,
        initial_input_sha256: source_record.payload_sha256.clone(),
    };
    let authority = ModelLaneRoutingAuthority {
        cloud_consent_receipt_ref: cloud_receipt_ref,
        validator_authority_ref: validator_ref,
        operator_authority_ref: operator_ref,
    };
    let (ledger, ledger_drain) = LedgerBatcher::manual_for_tests(
        LedgerBatcherConfig {
            capacity: 256,
            ..LedgerBatcherConfig::default()
        },
        Arc::new(RecordingOverflowSink::default()),
    )
    .expect("manual AC-9 process ledger");
    let creates = Arc::new(AtomicUsize::new(0));
    let teardowns = Arc::new(AtomicUsize::new(0));
    let hold_generation = Arc::new(AtomicBool::new(false));
    let hold_create = Arc::new(AtomicBool::new(false));
    let fail_model = Arc::new(Mutex::new(None));
    let model_outputs = Arc::new(Mutex::new(HashMap::new()));
    let held_models = Arc::new(Mutex::new(HashSet::new()));
    let coordinator = SwarmCoordinator::new_with_model_lane_store(
        SwarmConfig::new(RunBudget::defaulted(8)),
        Arc::new(Ac9ProductionFactory {
            ledger: ledger.clone(),
            creates: creates.clone(),
            teardowns: teardowns.clone(),
            hold_generation: hold_generation.clone(),
            hold_create: hold_create.clone(),
            fail_model: fail_model.clone(),
            model_outputs: model_outputs.clone(),
            held_models: held_models.clone(),
        }),
        Arc::new(RecordingSwarmSink::new()),
        ledger,
        store.clone(),
    );
    Ac9ProductionFixture {
        pool,
        store,
        coordinator,
        policy,
        execution_id,
        decision_id,
        context,
        authority,
        specs,
        creates,
        teardowns,
        hold_generation,
        hold_create,
        fail_model,
        model_outputs,
        held_models,
        ledger_drain,
    }
}

async fn ac9_wait_for_stage_state(
    pool: &PgPool,
    execution_id: &str,
    stage_id: &str,
    expected_state: &str,
) -> Value {
    for _ in 0..200 {
        if let Some(record) = sqlx::query_scalar::<_, Value>(
            "SELECT record_json FROM model_lane_routing_executions WHERE execution_id = $1",
        )
        .bind(execution_id)
        .fetch_optional(pool)
        .await
        .expect("poll AC-9 execution projection")
        {
            if record
                .pointer(&format!("/stages/{stage_id}/state"))
                .and_then(Value::as_str)
                == Some(expected_state)
            {
                return record;
            }
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("stage {stage_id} did not reach {expected_state}");
}

async fn ac9_wait_for_stage_attempt_state(
    pool: &PgPool,
    execution_id: &str,
    stage_id: &str,
    expected_attempt: u64,
    expected_state: &str,
) -> Value {
    for _ in 0..200 {
        if let Some(record) = sqlx::query_scalar::<_, Value>(
            "SELECT record_json FROM model_lane_routing_executions WHERE execution_id = $1",
        )
        .bind(execution_id)
        .fetch_optional(pool)
        .await
        .expect("poll AC-9 execution attempt projection")
        {
            let stage = &record["stages"][stage_id];
            if stage["attempt"].as_u64() == Some(expected_attempt)
                && stage["state"].as_str() == Some(expected_state)
            {
                return record;
            }
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("stage {stage_id} attempt {expected_attempt} did not reach {expected_state}");
}

async fn ac9_force_expired_lease(pool: &PgPool, execution_id: &str, stage_id: &str) {
    let row = sqlx::query(
        "SELECT record_json, event_ledger_event_id FROM model_lane_routing_executions WHERE execution_id = $1",
    )
    .bind(execution_id)
    .fetch_one(pool)
    .await
    .expect("load AC-9 execution for deterministic expiry");
    let mut execution: Value = row.get("record_json");
    execution["stages"][stage_id]["lease_expires_at_unix_ms"] = json!(0);
    let execution_event_id: String = row.get("event_ledger_event_id");
    sqlx::query("UPDATE model_lane_routing_executions SET record_json=$2 WHERE execution_id=$1")
        .bind(execution_id)
        .bind(&execution)
        .execute(pool)
        .await
        .expect("expire execution-stage lease projection");
    let mut event_payload: Value =
        sqlx::query_scalar("SELECT payload FROM kernel_event_ledger WHERE event_id=$1")
            .bind(&execution_event_id)
            .fetch_one(pool)
            .await
            .expect("load authoritative execution event");
    event_payload["record"] = execution.clone();
    sqlx::query("UPDATE kernel_event_ledger SET payload=$2 WHERE event_id=$1")
        .bind(&execution_event_id)
        .bind(event_payload)
        .execute(pool)
        .await
        .expect("expire authoritative execution-stage lease");

    let attempt_row = sqlx::query(
        "SELECT attempt, record_json, event_ledger_event_id FROM model_lane_routing_stage_attempts WHERE execution_id=$1 AND stage_id=$2 ORDER BY attempt DESC LIMIT 1",
    )
    .bind(execution_id)
    .bind(stage_id)
    .fetch_one(pool)
    .await
    .expect("load AC-9 attempt for deterministic expiry");
    let attempt: i64 = attempt_row.get("attempt");
    let mut attempt_record: Value = attempt_row.get("record_json");
    let attempt_event_id: String = attempt_row.get("event_ledger_event_id");
    attempt_record["lease_expires_at_unix_ms"] = json!(0);
    sqlx::query(
        "UPDATE model_lane_routing_stage_attempts SET lease_expires_at_unix_ms=0, record_json=$4 WHERE execution_id=$1 AND stage_id=$2 AND attempt=$3",
    )
    .bind(execution_id)
    .bind(stage_id)
    .bind(attempt)
    .bind(&attempt_record)
    .execute(pool)
    .await
    .expect("expire AC-9 attempt lease");
    let mut attempt_event_payload: Value =
        sqlx::query_scalar("SELECT payload FROM kernel_event_ledger WHERE event_id=$1")
            .bind(&attempt_event_id)
            .fetch_one(pool)
            .await
            .expect("load authoritative AC-9 attempt event");
    attempt_event_payload["lease_expires_at_unix_ms"] = json!(0);
    attempt_event_payload["record"] = attempt_record;
    sqlx::query("UPDATE kernel_event_ledger SET payload=$2 WHERE event_id=$1")
        .bind(&attempt_event_id)
        .bind(attempt_event_payload)
        .execute(pool)
        .await
        .expect("expire authoritative AC-9 attempt lease");

    let outbox_event_id: String = sqlx::query_scalar(
        "SELECT event_ledger_event_id FROM model_lane_routing_outbox WHERE execution_id=$1 AND stage_id=$2 AND attempt=$3",
    )
    .bind(execution_id)
    .bind(stage_id)
    .bind(attempt)
    .fetch_one(pool)
    .await
    .expect("load AC-9 outbox event pointer for deterministic expiry");
    sqlx::query(
        "UPDATE model_lane_routing_outbox SET lease_expires_at_unix_ms=0 WHERE execution_id=$1 AND stage_id=$2 AND attempt=$3",
    )
    .bind(execution_id)
    .bind(stage_id)
    .bind(attempt)
    .execute(pool)
    .await
    .expect("expire AC-9 outbox lease");
    let mut outbox_event_payload: Value =
        sqlx::query_scalar("SELECT payload FROM kernel_event_ledger WHERE event_id=$1")
            .bind(&outbox_event_id)
            .fetch_one(pool)
            .await
            .expect("load authoritative AC-9 outbox event");
    outbox_event_payload["lease_expires_at_unix_ms"] = json!(0);
    sqlx::query("UPDATE kernel_event_ledger SET payload=$2 WHERE event_id=$1")
        .bind(&outbox_event_id)
        .bind(outbox_event_payload)
        .execute(pool)
        .await
        .expect("expire authoritative AC-9 outbox lease");
}

fn ac9_stage_is_terminal(state: ModelLaneRoutingStageStateKind) -> bool {
    matches!(
        state,
        ModelLaneRoutingStageStateKind::Succeeded
            | ModelLaneRoutingStageStateKind::Failed
            | ModelLaneRoutingStageStateKind::Joined
            | ModelLaneRoutingStageStateKind::Cancelled
            | ModelLaneRoutingStageStateKind::Compensated
    )
}

fn ac9_stage_is_success(state: ModelLaneRoutingStageStateKind) -> bool {
    matches!(
        state,
        ModelLaneRoutingStageStateKind::Succeeded | ModelLaneRoutingStageStateKind::Joined
    )
}

#[test]
fn ac9_rejects_policy_labelled_arbitrary_dags_and_exposes_only_production_wrapper() {
    use handshake_core::swarm_orchestration::routing::{
        ModelLaneRoutingGraph, ModelLaneRoutingPolicy,
    };

    let _production_entrypoint = execute_production_routing_wave;
    for policy in ModelLaneRoutingPolicy::all().iter().copied() {
        let canonical = ModelLaneRoutingGraph::for_policy(policy);
        canonical
            .validate()
            .expect("canonical policy graph validates");
        let mut forged = canonical.clone();
        forged.stages.reverse();
        assert!(
            forged.validate().is_err(),
            "policy-labelled arbitrary DAG must be rejected for {}",
            policy.as_str()
        );
    }
}

#[tokio::test]
async fn ac9_all_six_policies_execute_real_production_waves_with_typed_lineage() {
    for policy in ModelLaneRoutingPolicy::all().iter().copied() {
        eprintln!("AC-9 six-policy proof: {} fixture start", policy.as_str());
        let fixture = ac9_fixture(policy, &format!("positive-{}", policy.as_str())).await;
        eprintln!("AC-9 six-policy proof: {} lifecycle start", policy.as_str());
        let batch = fixture
            .lifecycle()
            .await
            .expect("production lifecycle drives every ready wave without a test-side loop");
        eprintln!(
            "AC-9 six-policy proof: {} lifecycle returned {:?}",
            policy.as_str(),
            batch.execution.status
        );
        let final_batch =
            if batch.execution.status == ModelLaneRoutingExecutionStatus::AwaitingAuthority {
                eprintln!(
                    "AC-9 six-policy proof: {} authority completion start",
                    policy.as_str()
                );
                fixture
                    .complete_waiting_authority(&batch)
                    .await
                    .expect("external authority completion auto-resumes the lifecycle")
            } else {
                batch
            };
        let final_execution = final_batch.execution;
        assert_eq!(
            final_execution.status,
            ModelLaneRoutingExecutionStatus::Succeeded,
            "{} must complete its exact canonical DAG",
            policy.as_str()
        );
        assert_eq!(
            final_execution.selecting_decision_id, fixture.decision_id,
            "execution remains bound to the selecting decision"
        );
        assert_eq!(final_execution.run_id, fixture.context.run_id);
        assert_eq!(final_execution.trace_id, fixture.context.trace_id);
        assert_eq!(final_execution.locus_ref, fixture.context.locus_ref);
        for stage in final_execution.stages.values() {
            assert_eq!(stage.expected_run_id, fixture.context.run_id);
            assert!(
                ac9_stage_is_terminal(stage.state),
                "terminal execution has no live stage"
            );
            if matches!(
                stage.dispatch_target,
                ModelLaneRoutingDispatchTarget::LocalModel
                    | ModelLaneRoutingDispatchTarget::CloudModel
            ) {
                assert!(!stage.expected_lane_id.is_empty());
                assert!(!stage.expected_model_id.is_empty());
                assert!(stage.output_message_ref.is_some());
                assert!(stage.output_ref.is_some());
                assert!(stage.output_sha256.is_some());
            }
        }
        let replay = fixture
            .store
            .replay_run(&fixture.context.run_id)
            .await
            .expect("replay production routing ModelLaneRun");
        for stage in final_execution
            .stages
            .values()
            .filter(|stage| ac9_stage_is_success(stage.state))
        {
            let Some(message_id) = stage.output_message_ref.as_deref() else {
                continue;
            };
            let message = replay
                .messages
                .iter()
                .find(|message| message.message_id == message_id)
                .expect("successful stage output replays as a ModelLaneMessage");
            assert_eq!(message.run_id, fixture.context.run_id);
            assert_eq!(message.trace_id, fixture.context.trace_id);
            assert_eq!(
                message.coordinator_session_id,
                fixture.context.coordinator_session_id
            );
            assert_eq!(message.work_packet_id.as_deref(), Some(WP_ID));
            assert_eq!(message.micro_task_id.as_deref(), Some(MT_ID));
            assert_eq!(
                stage.output_ref.as_deref(),
                Some(message.payload_ref.as_str())
            );
            assert_eq!(
                stage.output_sha256.as_deref(),
                Some(message.payload_sha256.as_str())
            );
            let artifact_projection = fixture
                .store
                .navigation_by_artifact_or_context(
                    Some(message.payload_ref.as_str()),
                    None,
                    Some(fixture.context.run_id.as_str()),
                )
                .await
                .expect("resolve stage output through the authoritative artifact navigation route");
            let artifact = artifact_projection
                .artifacts
                .iter()
                .find(|artifact| artifact.artifact_ref == message.payload_ref)
                .expect("each successful output resolves to its exact durable payload artifact");
            assert_eq!(artifact.artifact_sha256, message.payload_sha256);
            assert_eq!(
                sha256_hex(&canonical_json_bytes(&artifact.payload_json)),
                message.payload_sha256,
                "artifact bytes, message binding, and execution output share one canonical hash"
            );
            let expected_authority = match stage.dispatch_target {
                ModelLaneRoutingDispatchTarget::Validator => ModelLaneAuthority::ValidatorVerdict,
                ModelLaneRoutingDispatchTarget::Operator => ModelLaneAuthority::OperatorDecision,
                _ => ModelLaneAuthority::Advisory,
            };
            assert_eq!(
                message.authority,
                expected_authority,
                "ordinary routing output remains advisory while authority responses retain their typed authority"
            );
            for (field, value) in [
                ("proposal_ref", message.proposal_ref.as_deref()),
                ("crdt_update_ref", message.crdt_update_ref.as_deref()),
                (
                    "crdt_base_snapshot_ref",
                    message.crdt_base_snapshot_ref.as_deref(),
                ),
                ("crdt_state_vector", message.crdt_state_vector.as_deref()),
                ("crdt_proposal_ref", message.crdt_proposal_ref.as_deref()),
                (
                    "crdt_stale_base_ref",
                    message.crdt_stale_base_ref.as_deref(),
                ),
            ] {
                assert!(
                    value.is_none(),
                    "{} stage {} fabricated {field}={value:?} instead of persisting real CRDT authority",
                    policy.as_str(),
                    stage.stage_id,
                );
            }
        }
        if policy == ModelLaneRoutingPolicy::CloudReview {
            assert_eq!(
                final_execution.stages["cloud-review"]
                    .output_payload
                    .as_ref()
                    .and_then(|value| value.pointer("/typed_output/schema_id"))
                    .and_then(Value::as_str),
                Some("hsk.model_lane_cloud_review_verdict@1")
            );
            assert_eq!(
                final_execution.stages["cloud-review"]
                    .output_payload
                    .as_ref()
                    .and_then(|value| value.pointer("/typed_output/verdict"))
                    .and_then(Value::as_str),
                Some("accept")
            );
        }
        if policy == ModelLaneRoutingPolicy::ParallelDebate {
            assert_eq!(
                final_execution.stages["debate-join"]
                    .output_payload
                    .as_ref()
                    .and_then(|value| value.pointer("/typed_output/schema_id"))
                    .and_then(Value::as_str),
                Some("hsk.model_lane_parallel_debate_adjudication@1")
            );
            assert_eq!(
                final_execution.stages["debate-join"].input_refs.len(),
                3,
                "debate join keeps the initial causal input plus both sibling artifacts"
            );
            assert_eq!(
                final_execution.stages["debate-join"]
                    .input_refs
                    .iter()
                    .filter(|input_ref| *input_ref != &fixture.context.initial_input_ref)
                    .count(),
                2,
                "typed debate adjudication compares only the two sibling artifacts"
            );
        }
        if matches!(
            policy,
            ModelLaneRoutingPolicy::ValidatorLane | ModelLaneRoutingPolicy::OperatorLane
        ) {
            let terminal_stage_id = if policy == ModelLaneRoutingPolicy::ValidatorLane {
                "validator-verdict"
            } else {
                "operator-decision"
            };
            let stage = &final_execution.stages[terminal_stage_id];
            let request_ref = stage
                .authority_request_message_ref
                .as_deref()
                .expect("authority stage preserves its typed causal request");
            let request = replay
                .messages
                .iter()
                .find(|message| message.message_id == request_ref)
                .expect("typed authority request replays");
            assert_eq!(
                request.diagnostic_payload["schema_id"],
                json!("hsk.model_lane_routing_authority_request@1")
            );
            assert_eq!(request.run_id, fixture.context.run_id);
            assert_eq!(request.trace_id, fixture.context.trace_id);
            assert_eq!(
                request.parent_span_id.as_deref(),
                Some(fixture.context.run_span_id.as_str()),
                "authority request remains a child of the routing run span"
            );
            let predecessor_message_ref = request.diagnostic_payload["predecessor_message_ref"]
                .as_str()
                .expect("authority request names its causal predecessor message");
            let predecessor_message = replay
                .messages
                .iter()
                .find(|message| message.message_id == predecessor_message_ref)
                .expect("causal predecessor message replays");
            assert_eq!(
                request.linked_span_contexts,
                vec![predecessor_message.message_span_id.clone()],
                "authority request links the predecessor message span, not an EventLedger event id"
            );
            assert_eq!(
                request.routing.as_ref().map(|routing| routing.requires_ack),
                Some(true)
            );
            let response = replay
                .messages
                .iter()
                .find(|message| {
                    stage.output_message_ref.as_deref() == Some(message.message_id.as_str())
                })
                .expect("typed authority response replays");
            assert_eq!(
                response
                    .routing
                    .as_ref()
                    .and_then(|routing| routing.ack_for.as_deref()),
                stage.authority_request_message_ref.as_deref(),
                "authority response explicitly acknowledges the causal request"
            );
        }
        fixture
            .ledger_drain
            .drain_available_to(Arc::new(PostgresProcessLedgerStore::new(
                fixture.pool.clone(),
            )))
            .await
            .expect("production-shaped START and STOP rows drain with identical ownership lineage");
        let ledger_counts: (i64, i64, i64) = sqlx::query_as(
            r#"
            SELECT
                COUNT(*),
                COUNT(*) FILTER (WHERE stopped_at IS NOT NULL),
                COUNT(*) FILTER (
                    WHERE owner_role = $2
                      AND owner_wp = $3
                      AND wp_id = $3
                      AND mt_id = $4
                )
            FROM kernel_process_lifecycle
            WHERE parent_session_id = $1
            "#,
        )
        .bind(&fixture.context.coordinator_session_id)
        .bind(OWNER)
        .bind(WP_ID)
        .bind(MT_ID)
        .fetch_one(&fixture.pool)
        .await
        .expect("read production-shaped process lifecycle proof");
        let expected_lifecycle_rows = fixture.creates.load(Ordering::SeqCst) as i64;
        assert_eq!(ledger_counts.0, expected_lifecycle_rows);
        assert_eq!(
            ledger_counts.1, expected_lifecycle_rows,
            "every production-shaped START must have a matching STOP"
        );
        assert_eq!(
            ledger_counts.2, expected_lifecycle_rows,
            "START and STOP must preserve exact owner/WP/MT lineage"
        );
        assert_eq!(
            fixture.creates.load(Ordering::SeqCst),
            fixture.teardowns.load(Ordering::SeqCst),
            "all real model sessions terminate without orphan handles"
        );
        eprintln!("AC-9 six-policy proof: {} complete", policy.as_str());
    }
}

#[tokio::test]
async fn ac9_rejects_policy_decision_projection_pointer_run_and_attempt_tamper() {
    let graph_fixture = ac9_fixture(ModelLaneRoutingPolicy::CloudReview, "tamper-graph").await;
    let decision_row = sqlx::query(
        "SELECT record_json, event_ledger_event_id FROM model_lane_promotion_decisions WHERE decision_id=$1",
    )
    .bind(&graph_fixture.decision_id)
    .fetch_one(&graph_fixture.pool)
    .await
    .expect("load selecting decision for canonical-graph tamper");
    let mut decision: Value = decision_row.get("record_json");
    decision["diagnostic_payload"]["routing_graph"]["stages"]
        .as_array_mut()
        .expect("routing graph stages")
        .reverse();
    let decision_event_id: String = decision_row.get("event_ledger_event_id");
    sqlx::query("UPDATE model_lane_promotion_decisions SET record_json=$2 WHERE decision_id=$1")
        .bind(&graph_fixture.decision_id)
        .bind(&decision)
        .execute(&graph_fixture.pool)
        .await
        .expect("tamper selecting decision projection graph");
    let mut decision_event: Value =
        sqlx::query_scalar("SELECT payload FROM kernel_event_ledger WHERE event_id=$1")
            .bind(&decision_event_id)
            .fetch_one(&graph_fixture.pool)
            .await
            .expect("load selecting decision EventLedger payload");
    decision_event["record"] = decision;
    sqlx::query("UPDATE kernel_event_ledger SET payload=$2 WHERE event_id=$1")
        .bind(&decision_event_id)
        .bind(decision_event)
        .execute(&graph_fixture.pool)
        .await
        .expect("tamper selecting decision EventLedger graph");
    let graph_error = graph_fixture
        .wave()
        .await
        .expect_err("noncanonical policy-labelled graph must fail");
    assert!(graph_error.to_string().contains("exact canonical graph"));

    let decision_fixture =
        ac9_fixture(ModelLaneRoutingPolicy::CloudReview, "tamper-decision").await;
    sqlx::query(
        "UPDATE model_lane_promotion_decisions SET record_json=jsonb_set(record_json, '{owner_session}', '\"tampered-owner\"'::jsonb) WHERE decision_id=$1",
    )
    .bind(&decision_fixture.decision_id)
    .execute(&decision_fixture.pool)
    .await
    .expect("tamper decision projection only");
    assert!(decision_fixture
        .wave()
        .await
        .expect_err("decision projection drift must fail")
        .to_string()
        .contains("projection/EventLedger"));

    let run_fixture = ac9_fixture(ModelLaneRoutingPolicy::CloudReview, "tamper-run").await;
    sqlx::query(
        "UPDATE model_lane_runs SET record_json=jsonb_set(record_json, '{trace_id}', '\"trace-tampered\"'::jsonb) WHERE run_id=$1",
    )
    .bind(&run_fixture.context.run_id)
    .execute(&run_fixture.pool)
    .await
    .expect("tamper run projection only");
    assert!(run_fixture
        .wave()
        .await
        .expect_err("run projection drift must fail")
        .to_string()
        .contains("projection/EventLedger"));

    let pointer_fixture = ac9_fixture(ModelLaneRoutingPolicy::CloudReview, "tamper-pointer").await;
    pointer_fixture
        .wave()
        .await
        .expect("initialize pointer fixture");
    sqlx::query(
        "UPDATE model_lane_routing_executions SET event_ledger_seq=event_ledger_seq+1 WHERE execution_id=$1",
    )
    .bind(&pointer_fixture.execution_id)
    .execute(&pointer_fixture.pool)
    .await
    .expect("tamper execution EventLedger pointer");
    assert!(pointer_fixture
        .wave()
        .await
        .expect_err("dangling execution pointer must fail")
        .to_string()
        .contains("integrity"));

    let attempt_fixture = ac9_fixture(ModelLaneRoutingPolicy::CloudReview, "tamper-attempt").await;
    attempt_fixture
        .wave()
        .await
        .expect("initialize attempt fixture");
    sqlx::query(
        "UPDATE model_lane_routing_stage_attempts SET record_json=jsonb_set(record_json, '{expected_model_id}', '\"tampered-model\"'::jsonb) WHERE execution_id=$1 AND stage_id='local-candidate'",
    )
    .bind(&attempt_fixture.execution_id)
    .execute(&attempt_fixture.pool)
    .await
    .expect("tamper active attempt projection");
    assert!(attempt_fixture
        .wave()
        .await
        .expect_err("attempt projection drift must fail")
        .to_string()
        .contains("routing attempt"));
}

#[tokio::test]
async fn ac9_concurrent_production_claims_and_run_extension_are_single_effect_idempotent() {
    let fixture =
        Arc::new(ac9_fixture(ModelLaneRoutingPolicy::CloudReview, "concurrent-claim").await);
    fixture.hold_generation.store(true, Ordering::SeqCst);
    let left_worker = {
        let fixture = fixture.clone();
        tokio::spawn(async move { fixture.wave().await })
    };
    ac9_wait_for_stage_state(
        &fixture.pool,
        &fixture.execution_id,
        "local-candidate",
        "in_flight",
    )
    .await;
    let right = fixture
        .wave()
        .await
        .expect("competing production claim wave");
    fixture.hold_generation.store(false, Ordering::SeqCst);
    let left = left_worker
        .await
        .expect("join winning production wave")
        .expect("winning production wave succeeds");
    assert_eq!(
        left.dispatched.len() + right.dispatched.len(),
        1,
        "SKIP LOCKED outbox claim permits one local-candidate dispatcher"
    );
    assert_eq!(fixture.creates.load(Ordering::SeqCst), 1);
    let local_lane_id = fixture
        .specs
        .iter()
        .find(|spec| spec.stage_id == "local-candidate")
        .and_then(|spec| spec.lane_id.as_deref())
        .expect("local candidate planned lane");
    let lane_rows: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM model_lanes WHERE run_id=$1 AND lane_id=$2")
            .bind(&fixture.context.run_id)
            .bind(local_lane_id)
            .fetch_one(&fixture.pool)
            .await
            .expect("count concurrently attached lane rows");
    assert_eq!(lane_rows, 1);
    let run_lane_occurrences: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM model_lane_runs run CROSS JOIN LATERAL jsonb_array_elements_text(run.record_json->'lane_ids') lane WHERE run.run_id=$1 AND lane=$2",
    )
    .bind(&fixture.context.run_id)
    .bind(local_lane_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("count canonical run lane membership");
    assert_eq!(run_lane_occurrences, 1);
    let final_batch = fixture
        .wave()
        .await
        .expect("dispatch cloud review after claim race");
    assert_eq!(
        final_batch.execution.status,
        ModelLaneRoutingExecutionStatus::Succeeded
    );
}

#[tokio::test]
async fn ac9_concurrent_exact_run_extension_replays_one_canonical_lane() {
    let fixture = ac9_fixture(ModelLaneRoutingPolicy::CloudReview, "run-extension-race").await;
    let mut launches = fixture.launches();
    let request = launches
        .iter_mut()
        .find(|launch| launch.stage_id == "local-candidate")
        .and_then(|launch| launch.request.take())
        .expect("real local-candidate SpawnRequest");
    let record_id = ProcessOwnershipRecordId::new_v7();
    let teardown: handshake_core::swarm_orchestration::SessionTeardown =
        Arc::new(|| Box::pin(async { Ok(()) }));
    let live = LiveSession::new(
        Arc::new(Ac9ProductionRuntime::new(
            Arc::new(AtomicBool::new(false)),
            Arc::new(Mutex::new(HashMap::new())),
        )),
        request.instance_id.model_id,
        CancellationToken::new(),
        teardown,
        record_id,
        62_001,
    );
    let records = build_successful_launch_records(&request, &live)
        .expect("build exact successful launch records for extension race");
    let (left, right) = tokio::join!(
        fixture.store.record_prepared_launch(records.clone()),
        fixture.store.record_prepared_launch(records),
    );
    let (left_run, left_lane) = left.expect("left exact run extension");
    let (right_run, right_lane) = right.expect("right exact run extension replay");
    assert_eq!(left_lane.lane_id, right_lane.lane_id);
    assert_eq!(
        left_lane.event_ledger_event_id,
        right_lane.event_ledger_event_id
    );
    assert_eq!(left_run.run_id, right_run.run_id);
    assert_eq!(
        right_run
            .lane_ids
            .iter()
            .filter(|lane_id| *lane_id == &right_lane.lane_id)
            .count(),
        1
    );
    let replay = fixture
        .store
        .replay_run(&fixture.context.run_id)
        .await
        .expect("replay concurrently extended run");
    assert_eq!(
        replay
            .lanes
            .iter()
            .filter(|lane| lane.lane_id == right_lane.lane_id)
            .count(),
        1
    );
}

#[tokio::test]
async fn ac9_local_first_failure_dispatches_the_cloud_escalation_contract() {
    let fixture = ac9_fixture(ModelLaneRoutingPolicy::LocalFirst, "local-fallback").await;
    let local_model = fixture
        .specs
        .iter()
        .find(|spec| spec.stage_id == "local-attempt")
        .and_then(|spec| spec.instance_id)
        .expect("local-attempt model")
        .model_id;
    *fixture.fail_model.lock().expect("set local failure") = Some(local_model);
    let local = fixture
        .wave()
        .await
        .expect("persist failed local production attempt");
    assert_eq!(
        local.execution.stages["local-attempt"].state,
        ModelLaneRoutingStageStateKind::Failed
    );
    assert_eq!(
        local.execution.status,
        ModelLaneRoutingExecutionStatus::Running
    );
    *fixture.fail_model.lock().expect("clear local failure") = None;
    let cloud = fixture
        .wave()
        .await
        .expect("dispatch cloud escalation after local failure");
    assert_eq!(
        cloud.execution.status,
        ModelLaneRoutingExecutionStatus::Succeeded
    );
    assert_eq!(
        cloud.execution.stages["cloud-escalation"].state,
        ModelLaneRoutingStageStateKind::Succeeded
    );
    assert_eq!(
        cloud.execution.stages["cloud-escalation"].expected_provider,
        Some(ProviderKind::ByokCloud)
    );
}

#[tokio::test]
async fn ac9_parallel_peer_failure_immediately_cancels_live_sibling() {
    let fixture =
        Arc::new(ac9_fixture(ModelLaneRoutingPolicy::ParallelDebate, "peer-failure").await);
    fixture.hold_generation.store(true, Ordering::SeqCst);
    let cloud_model = fixture
        .specs
        .iter()
        .find(|spec| spec.stage_id == "debate-cloud")
        .and_then(|spec| spec.instance_id)
        .expect("debate-cloud model")
        .model_id;
    *fixture.fail_model.lock().expect("set cloud peer failure") = Some(cloud_model);
    let result = fixture.wave().await;
    assert!(
        result.is_err(),
        "cancelled sibling worker reports its stale terminal write"
    );
    let record: Value = sqlx::query_scalar(
        "SELECT record_json FROM model_lane_routing_executions WHERE execution_id=$1",
    )
    .bind(&fixture.execution_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("load peer-failure execution");
    assert_eq!(record["status"], json!("failed"));
    assert_eq!(record["stages"]["debate-cloud"]["state"], json!("failed"));
    assert_eq!(
        record["stages"]["debate-local"]["state"],
        json!("cancelled")
    );
    assert_eq!(fixture.teardowns.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn ac9_crash_after_intent_before_spawn_completion_recovers_without_orphan() {
    let fixture = Arc::new(ac9_fixture(ModelLaneRoutingPolicy::LocalFirst, "crash-intent").await);
    fixture.hold_create.store(true, Ordering::SeqCst);
    let worker = {
        let fixture = fixture.clone();
        tokio::spawn(async move { fixture.wave().await })
    };
    let intent = ac9_wait_for_stage_state(
        &fixture.pool,
        &fixture.execution_id,
        "local-attempt",
        "in_flight",
    )
    .await;
    assert!(intent["stages"]["local-attempt"]["instance_id"].is_string());
    let planned_lane_id = fixture
        .specs
        .iter()
        .find(|spec| spec.stage_id == "local-attempt")
        .and_then(|spec| spec.lane_id.as_deref())
        .expect("planned local-attempt lane");
    let lane_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM model_lanes WHERE lane_id=$1")
        .bind(planned_lane_id)
        .fetch_one(&fixture.pool)
        .await
        .expect("count lanes before held factory returns");
    assert_eq!(
        lane_count, 0,
        "intent is durable before spawn records exist"
    );
    assert_eq!(fixture.teardowns.load(Ordering::SeqCst), 0);
    worker.abort();
    ac9_force_expired_lease(&fixture.pool, &fixture.execution_id, "local-attempt").await;
    fixture.hold_create.store(false, Ordering::SeqCst);
    let recovered = fixture
        .coordinator
        .recover_routing_execution(&fixture.execution_id, fixture.launches())
        .await
        .expect("recover intent-only crash boundary");
    assert_eq!(
        recovered.execution.stages["local-attempt"].state,
        ModelLaneRoutingStageStateKind::Succeeded
    );
    assert_eq!(recovered.execution.stages["local-attempt"].attempt, 2);
    assert_eq!(fixture.creates.load(Ordering::SeqCst), 2);
    assert_eq!(fixture.teardowns.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn ac9_crash_after_persisted_spawn_intent_recovers_with_new_fence_and_compensation() {
    let fixture = Arc::new(
        ac9_fixture(
            ModelLaneRoutingPolicy::CloudPlanLocalExecute,
            "crash-recover",
        )
        .await,
    );
    fixture.hold_generation.store(true, Ordering::SeqCst);
    let worker = {
        let fixture = fixture.clone();
        tokio::spawn(async move { fixture.wave().await })
    };
    let before_crash = ac9_wait_for_stage_state(
        &fixture.pool,
        &fixture.execution_id,
        "cloud-plan",
        "in_flight",
    )
    .await;
    let old_fence = before_crash["stages"]["cloud-plan"]["fencing_token"]
        .as_str()
        .expect("persisted pre-spawn fence")
        .to_string();
    assert!(before_crash["stages"]["cloud-plan"]["instance_id"].is_string());
    worker.abort();
    ac9_force_expired_lease(&fixture.pool, &fixture.execution_id, "cloud-plan").await;
    fixture.hold_generation.store(false, Ordering::SeqCst);
    let recovered = fixture
        .coordinator
        .recover_routing_execution(&fixture.execution_id, fixture.launches())
        .await
        .expect("recover and redispatch expired production stage");
    let stage = &recovered.execution.stages["cloud-plan"];
    assert_eq!(stage.attempt, 2);
    assert_eq!(stage.state, ModelLaneRoutingStageStateKind::Succeeded);
    let retry_fence: String = sqlx::query_scalar(
        "SELECT ledger.payload->>'fencing_token' FROM model_lane_routing_stage_attempts attempt JOIN kernel_event_ledger ledger ON ledger.event_id=attempt.event_ledger_event_id AND ledger.event_sequence=attempt.event_ledger_seq WHERE attempt.execution_id=$1 AND attempt.stage_id='cloud-plan' AND attempt.attempt=2",
    )
    .bind(&fixture.execution_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("read retry fencing token from terminal attempt event");
    assert_ne!(retry_fence, old_fence);
    let compensated = sqlx::query(
        "SELECT attempt.status, attempt.record_json, ledger.aggregate_type, ledger.aggregate_id, ledger.payload FROM model_lane_routing_stage_attempts attempt JOIN kernel_event_ledger ledger ON ledger.event_id=attempt.event_ledger_event_id AND ledger.event_sequence=attempt.event_ledger_seq WHERE attempt.execution_id=$1 AND attempt.stage_id='cloud-plan' AND attempt.attempt=1",
    )
    .bind(&fixture.execution_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("read compensated stale attempt and its EventLedger record");
    let compensated_status: String = compensated.get("status");
    let compensated_record: Value = compensated.get("record_json");
    let compensated_aggregate_type: String = compensated.get("aggregate_type");
    let compensated_aggregate_id: String = compensated.get("aggregate_id");
    let compensated_payload: Value = compensated.get("payload");
    assert_eq!(compensated_status, "compensated");
    assert_eq!(compensated_record["state"], json!("compensated"));
    assert_eq!(compensated_record["fencing_token"], Value::Null);
    assert_eq!(
        compensated_aggregate_type,
        "model_lane_routing_stage_attempt"
    );
    assert_eq!(
        compensated_aggregate_id,
        format!("{}:cloud-plan:1", fixture.execution_id)
    );
    assert_eq!(compensated_payload["state"], json!("compensated"));
    assert_eq!(fixture.creates.load(Ordering::SeqCst), 2);
    assert_eq!(fixture.teardowns.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn ac9_bounded_retry_exhaustion_fails_after_three_durable_attempts() {
    let fixture =
        Arc::new(ac9_fixture(ModelLaneRoutingPolicy::LocalFirst, "retry-exhaustion").await);
    fixture.hold_create.store(true, Ordering::SeqCst);

    for expected_attempt in 1..=3_u64 {
        let worker = if expected_attempt == 1 {
            let fixture = fixture.clone();
            tokio::spawn(async move { fixture.wave().await })
        } else {
            let fixture = fixture.clone();
            tokio::spawn(async move {
                fixture
                    .coordinator
                    .recover_routing_execution(&fixture.execution_id, fixture.launches())
                    .await
            })
        };
        ac9_wait_for_stage_attempt_state(
            &fixture.pool,
            &fixture.execution_id,
            "local-attempt",
            expected_attempt,
            "in_flight",
        )
        .await;
        worker.abort();
        ac9_force_expired_lease(&fixture.pool, &fixture.execution_id, "local-attempt").await;
    }

    fixture.hold_create.store(false, Ordering::SeqCst);
    let exhausted = fixture
        .coordinator
        .recover_routing_execution(&fixture.execution_id, fixture.launches())
        .await
        .expect("bounded local exhaustion dispatches the canonical cloud fallback");
    let stage = &exhausted.execution.stages["local-attempt"];
    assert_eq!(stage.attempt, 3);
    assert_eq!(stage.state, ModelLaneRoutingStageStateKind::Failed);
    assert_eq!(
        stage.detail.as_deref(),
        Some("routing stage exhausted bounded recovery attempts")
    );
    assert_eq!(
        exhausted.execution.status,
        ModelLaneRoutingExecutionStatus::Succeeded,
        "LocalFirst must continue through its bounded cloud fallback"
    );
    assert_eq!(
        exhausted.execution.stages["cloud-escalation"].state,
        ModelLaneRoutingStageStateKind::Succeeded
    );
    assert_eq!(fixture.creates.load(Ordering::SeqCst), 4);
    assert_eq!(fixture.teardowns.load(Ordering::SeqCst), 1);

    let attempts: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM model_lane_routing_stage_attempts WHERE execution_id=$1 AND stage_id='local-attempt'",
    )
    .bind(&fixture.execution_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("count bounded retry attempts");
    assert_eq!(attempts, 3, "retry authority must never create attempt 4");
    let exhausted_payload: Value = sqlx::query_scalar(
        "SELECT payload FROM kernel_event_ledger WHERE aggregate_type='model_lane_routing_stage_attempt' AND aggregate_id=$1 ORDER BY event_sequence DESC LIMIT 1",
    )
    .bind(format!("{}:local-attempt:3", fixture.execution_id))
    .fetch_one(&fixture.pool)
    .await
    .expect("load bounded recovery exhaustion EventLedger evidence");
    assert_eq!(
        exhausted_payload["reason"],
        json!("bounded_recovery_exhausted")
    );
    assert_eq!(exhausted_payload["attempt"], json!(3));
    assert_eq!(exhausted_payload["state"], json!("failed"));
}

#[tokio::test]
async fn ac9_redispatch_rejects_the_live_prior_attempt_stale_fence() {
    let fixture =
        Arc::new(ac9_fixture(ModelLaneRoutingPolicy::CloudPlanLocalExecute, "stale-fence").await);
    fixture.hold_generation.store(true, Ordering::SeqCst);
    let prior_worker = {
        let fixture = fixture.clone();
        tokio::spawn(async move { fixture.wave().await })
    };
    let prior = ac9_wait_for_stage_attempt_state(
        &fixture.pool,
        &fixture.execution_id,
        "cloud-plan",
        1,
        "in_flight",
    )
    .await;
    let prior_fence = prior["stages"]["cloud-plan"]["fencing_token"]
        .as_str()
        .expect("attempt-1 fence")
        .to_string();
    ac9_force_expired_lease(&fixture.pool, &fixture.execution_id, "cloud-plan").await;
    let recovery_worker = {
        let fixture = fixture.clone();
        tokio::spawn(async move {
            fixture
                .coordinator
                .recover_routing_execution(&fixture.execution_id, fixture.launches())
                .await
        })
    };
    let retry = ac9_wait_for_stage_attempt_state(
        &fixture.pool,
        &fixture.execution_id,
        "cloud-plan",
        2,
        "in_flight",
    )
    .await;
    assert_ne!(
        retry["stages"]["cloud-plan"]["fencing_token"].as_str(),
        Some(prior_fence.as_str())
    );
    fixture.hold_generation.store(false, Ordering::SeqCst);
    let recovered = recovery_worker
        .await
        .expect("join retry worker")
        .expect("attempt 2 commits through its current fence");
    assert_eq!(
        recovered.execution.stages["cloud-plan"].state,
        ModelLaneRoutingStageStateKind::Succeeded
    );
    let stale = prior_worker
        .await
        .expect("join prior worker")
        .expect_err("attempt 1 cannot commit after attempt 2 owns the stage");
    assert!(stale.to_string().contains("stale routing claim"));
    let stale_artifacts: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM model_lane_context_bundle_artifacts WHERE artifact_ref LIKE $1",
    )
    .bind(format!(
        "artifact://model-lane-routing/{}/cloud-plan/1/%",
        fixture.execution_id
    ))
    .fetch_one(&fixture.pool)
    .await
    .expect("count stale-attempt artifacts");
    assert_eq!(
        stale_artifacts, 0,
        "stale fence is rejected before artifact/message persistence"
    );
    assert_eq!(fixture.creates.load(Ordering::SeqCst), 2);
    assert_eq!(fixture.teardowns.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn ac9_cancel_terminalizes_parallel_siblings_and_live_sessions() {
    let fixture =
        Arc::new(ac9_fixture(ModelLaneRoutingPolicy::ParallelDebate, "cancel-siblings").await);
    fixture.hold_generation.store(true, Ordering::SeqCst);
    let worker = {
        let fixture = fixture.clone();
        tokio::spawn(async move { fixture.wave().await })
    };
    ac9_wait_for_stage_state(
        &fixture.pool,
        &fixture.execution_id,
        "debate-local",
        "in_flight",
    )
    .await;
    ac9_wait_for_stage_state(
        &fixture.pool,
        &fixture.execution_id,
        "debate-cloud",
        "in_flight",
    )
    .await;
    let cancelled = fixture
        .coordinator
        .cancel_routing_execution(&fixture.execution_id, "operator AC-9 cancellation")
        .await
        .expect("cancel production routing execution");
    assert_eq!(cancelled.status, ModelLaneRoutingExecutionStatus::Cancelled);
    assert!(cancelled.stages.values().all(|stage| {
        stage.state == ModelLaneRoutingStageStateKind::Cancelled
            || ac9_stage_is_terminal(stage.state)
    }));
    let _ = worker.await;
    assert_eq!(fixture.creates.load(Ordering::SeqCst), 2);
    assert_eq!(fixture.teardowns.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn ac9_inflight_heartbeat_renews_lease_and_eventledger_pointer() {
    let fixture = Arc::new(ac9_fixture(ModelLaneRoutingPolicy::LocalFirst, "heartbeat").await);
    fixture.hold_generation.store(true, Ordering::SeqCst);
    let worker = {
        let fixture = fixture.clone();
        tokio::spawn(async move { fixture.wave().await })
    };
    let before = ac9_wait_for_stage_state(
        &fixture.pool,
        &fixture.execution_id,
        "local-attempt",
        "in_flight",
    )
    .await;
    let before_event = before["stages"]["local-attempt"]["event_ledger_event_id"]
        .as_str()
        .expect("pre-heartbeat event pointer")
        .to_string();
    let before_expiry = before["stages"]["local-attempt"]["lease_expires_at_unix_ms"]
        .as_u64()
        .expect("pre-heartbeat lease expiry");
    tokio::time::sleep(Duration::from_secs(11)).await;
    let after: Value = sqlx::query_scalar(
        "SELECT record_json FROM model_lane_routing_executions WHERE execution_id=$1",
    )
    .bind(&fixture.execution_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("load post-heartbeat execution projection");
    assert_ne!(
        after["stages"]["local-attempt"]["event_ledger_event_id"].as_str(),
        Some(before_event.as_str()),
        "heartbeat advances the attempt EventLedger pointer"
    );
    assert!(
        after["stages"]["local-attempt"]["lease_expires_at_unix_ms"]
            .as_u64()
            .expect("post-heartbeat lease expiry")
            > before_expiry,
        "heartbeat extends the live lease"
    );
    fixture
        .coordinator
        .cancel_routing_execution(&fixture.execution_id, "heartbeat proof complete")
        .await
        .expect("cancel heartbeat proof execution");
    let _ = worker.await;
}

#[tokio::test]
async fn ac9_replay_rejects_message_and_artifact_projection_drift() {
    let message_fixture = ac9_fixture(ModelLaneRoutingPolicy::LocalFirst, "tamper-message").await;
    let batch = message_fixture
        .wave()
        .await
        .expect("produce message tamper fixture output");
    let message_id = batch.execution.stages["local-attempt"]
        .output_message_ref
        .as_deref()
        .expect("local output message");
    sqlx::query(
        "UPDATE model_lane_messages SET event_ledger_seq=event_ledger_seq+1 WHERE message_id=$1",
    )
    .bind(message_id)
    .execute(&message_fixture.pool)
    .await
    .expect("tamper ModelLaneMessage pointer");
    assert!(message_fixture
        .store
        .replay_run(&message_fixture.context.run_id)
        .await
        .expect_err("message pointer drift must fail replay")
        .to_string()
        .contains("EventLedger"));

    let artifact_fixture = ac9_fixture(ModelLaneRoutingPolicy::LocalFirst, "tamper-artifact").await;
    let batch = artifact_fixture
        .wave()
        .await
        .expect("produce artifact tamper fixture output");
    let artifact_ref = batch.execution.stages["local-attempt"]
        .output_ref
        .as_deref()
        .expect("local output artifact");
    sqlx::query(
        "UPDATE model_lane_context_bundle_artifacts SET record_json=jsonb_set(record_json, '{payload_json,typed_output,proposal}', '\"tampered-output\"'::jsonb) WHERE artifact_ref=$1",
    )
    .bind(artifact_ref)
    .execute(&artifact_fixture.pool)
    .await
    .expect("tamper output artifact projection");
    assert!(artifact_fixture
        .store
        .replay_run(&artifact_fixture.context.run_id)
        .await
        .expect_err("artifact projection drift must fail replay")
        .to_string()
        .contains("EventLedger"));
}

#[tokio::test]
async fn ac9_authority_retry_rejects_late_prior_attempt_ack_and_uses_new_fence() {
    let fixture = ac9_fixture(ModelLaneRoutingPolicy::ValidatorLane, "authority-stale").await;
    fixture.wave().await.expect("produce validator candidate");
    let first_authority = fixture
        .wave()
        .await
        .expect("dispatch validator authority attempt 1");
    let first_stage = &first_authority.execution.stages["validator-verdict"];
    assert_eq!(
        first_stage.state,
        ModelLaneRoutingStageStateKind::AwaitingAuthority
    );
    let first_request = first_stage
        .authority_request_message_ref
        .clone()
        .expect("attempt-1 authority request");
    let first_fence = first_stage
        .fencing_token
        .clone()
        .expect("attempt-1 authority fence");
    let authority_lane_id = first_stage.lane_id.clone().expect("validator lane");
    let late_message_id = format!("ac9-late-authority-{}", fixture.execution_id);
    let mut late = sample_message(
        &late_message_id,
        &fixture.context.run_id,
        &authority_lane_id,
        "authority",
        120,
    );
    late.kind = ModelLaneMessageKind::Status;
    late.authority = ModelLaneAuthority::ValidatorVerdict;
    late.promotion_decision_id = None;
    late.promotion_gate_ref = None;
    late.promotion_receipt_ref = None;
    late.promoted_artifact_ref = None;
    late.promoted_artifact_sha256 = None;
    late.promoted_artifact_version = None;
    late.validator_verdict_ref = first_stage.authority_ref.clone();
    late.routing = Some(ModelLaneRoutingMetadata {
        target_role: "coordinator".into(),
        target_session: fixture.context.coordinator_session_id.clone(),
        correlation_id: format!("routing:{}:validator-verdict", fixture.execution_id),
        requires_ack: false,
        ack_for: Some(first_request.clone()),
    });
    late.diagnostic_payload = json!({
        "schema_id": "hsk.model_lane_routing_authority_response@1",
        "execution_id": fixture.execution_id,
        "stage_id": "validator-verdict",
        "attempt": 1,
        "verdict": "approve",
    });
    late.payload_sha256 = sha256_hex(&canonical_json_bytes(&artifact_payload_json_for_message(
        &late,
    )));
    let late_record = fixture
        .store
        .record_message_with_payload_binding(
            late.clone(),
            sample_artifact_binding_for_message(&late),
        )
        .await
        .expect("persist valid but deliberately late attempt-1 authority response");
    ac9_force_expired_lease(&fixture.pool, &fixture.execution_id, "validator-verdict").await;
    let second_authority = fixture
        .coordinator
        .recover_routing_execution(&fixture.execution_id, fixture.launches())
        .await
        .expect("compensate and redispatch validator authority attempt 2");
    let second_stage = &second_authority.execution.stages["validator-verdict"];
    assert_eq!(second_stage.attempt, 2);
    assert_eq!(
        second_stage.state,
        ModelLaneRoutingStageStateKind::AwaitingAuthority
    );
    assert_ne!(
        second_stage.fencing_token.as_deref(),
        Some(first_fence.as_str())
    );
    assert_ne!(
        second_stage.authority_request_message_ref.as_deref(),
        Some(first_request.as_str())
    );
    let stale_error = fixture
        .coordinator
        .complete_authority_routing_stage(
            &fixture.execution_id,
            "validator-verdict",
            &late_record.message_id,
        )
        .await
        .expect_err("attempt-1 authority ack cannot satisfy attempt 2");
    assert!(stale_error.to_string().contains("causally bound"));
    let final_batch = fixture
        .complete_waiting_authority(&second_authority)
        .await
        .expect("record and complete attempt-2 authority response");
    assert_eq!(
        final_batch.execution.status,
        ModelLaneRoutingExecutionStatus::Succeeded
    );
    let attempt_one = sqlx::query(
        "SELECT attempt.status, attempt.record_json, ledger.aggregate_id, ledger.payload FROM model_lane_routing_stage_attempts attempt JOIN kernel_event_ledger ledger ON ledger.event_id=attempt.event_ledger_event_id AND ledger.event_sequence=attempt.event_ledger_seq WHERE attempt.execution_id=$1 AND attempt.stage_id='validator-verdict' AND attempt.attempt=1",
    )
    .bind(&fixture.execution_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("read authority attempt-1 compensation lineage");
    let attempt_one_status: String = attempt_one.get("status");
    let attempt_one_record: Value = attempt_one.get("record_json");
    let attempt_one_aggregate_id: String = attempt_one.get("aggregate_id");
    let attempt_one_payload: Value = attempt_one.get("payload");
    assert_eq!(attempt_one_status, "compensated");
    assert_eq!(attempt_one_record["state"], json!("compensated"));
    assert_eq!(attempt_one_record["fencing_token"], Value::Null);
    assert_eq!(
        attempt_one_aggregate_id,
        format!("{}:validator-verdict:1", fixture.execution_id)
    );
    assert_eq!(attempt_one_payload["state"], json!("compensated"));
}

#[tokio::test]
async fn ac9_cloud_review_strict_typed_decoder_rejects_malformed_outputs_without_artifacts() {
    let invalid = [
        ("missing", r#"{}"#),
        ("null", r#"{"verdict":null,"review":"x"}"#),
        ("numeric", r#"{"verdict":7,"review":"x"}"#),
        ("blank", r#"{"verdict":"accept","review":"   "}"#),
        ("invalid", r#"{"verdict":"maybe","review":"x"}"#),
    ];
    for (suffix, output) in invalid {
        let fixture = ac9_fixture(
            ModelLaneRoutingPolicy::CloudReview,
            &format!("review-{suffix}"),
        )
        .await;
        let cloud_model = fixture
            .specs
            .iter()
            .find(|spec| spec.stage_id == "cloud-review")
            .and_then(|spec| spec.instance_id)
            .expect("cloud-review model")
            .model_id;
        fixture
            .model_outputs
            .lock()
            .expect("set malformed review")
            .insert(cloud_model, output.to_string());
        let error = fixture
            .lifecycle()
            .await
            .expect_err("malformed CloudReview must fail closed");
        assert!(error.to_string().contains("cloud-review"));
        let artifacts: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM model_lane_context_bundle_artifacts WHERE artifact_ref LIKE $1",
        )
        .bind(format!(
            "artifact://model-lane-routing/{}/cloud-review/%",
            fixture.execution_id
        ))
        .fetch_one(&fixture.pool)
        .await
        .expect("count rejected review artifacts");
        assert_eq!(
            artifacts, 0,
            "typed decoding precedes artifact/message side effects"
        );
    }
}

#[tokio::test]
async fn ac9_parallel_debate_adjudication_is_content_sensitive_for_contradictory_fixtures() {
    let cases = [
        (
            "contradiction-a",
            "Never deploy this change.",
            "Deploy this change immediately.",
        ),
        (
            "contradiction-b",
            "Delete the migration.",
            "Retain and expand the migration.",
        ),
    ];
    let mut selected_outputs = Vec::new();
    for (suffix, local, cloud) in cases {
        let fixture = ac9_fixture(ModelLaneRoutingPolicy::ParallelDebate, suffix).await;
        for spec in &fixture.specs {
            let Some(instance) = spec.instance_id else {
                continue;
            };
            let output = if spec.stage_id == "debate-local" {
                local
            } else {
                cloud
            };
            fixture
                .model_outputs
                .lock()
                .expect("set debate output")
                .insert(instance.model_id, output.to_string());
        }
        let batch = fixture
            .lifecycle()
            .await
            .expect("drive contradictory debate lifecycle");
        let join = &batch.execution.stages["debate-join"];
        let output_ref = join.output_ref.as_deref().expect("debate artifact");
        let projection = fixture
            .store
            .navigation_by_artifact_or_context(
                Some(output_ref),
                None,
                Some(&fixture.context.run_id),
            )
            .await
            .expect("load debate adjudication artifact");
        let artifact = projection
            .artifacts
            .iter()
            .find(|artifact| artifact.artifact_ref == output_ref)
            .expect("exact debate artifact");
        assert_eq!(
            artifact
                .payload_json
                .pointer("/typed_output/decision")
                .and_then(Value::as_str),
            Some("selected_canonical_candidate")
        );
        assert!(artifact
            .payload_json
            .pointer("/typed_output/rationale")
            .and_then(Value::as_str)
            .is_some_and(|value| value.contains("proposals conflict")));
        selected_outputs.push(
            artifact
                .payload_json
                .pointer("/typed_output/selected_output")
                .and_then(Value::as_str)
                .expect("selected content-sensitive output")
                .to_string(),
        );
    }
    assert_ne!(selected_outputs[0], selected_outputs[1]);
}

#[tokio::test]
async fn ac9_selecting_decision_launch_plan_rejects_local_cloud_and_provider_tamper() {
    for (suffix, stage_id, tamper_provider) in [
        ("plan-local", "local-candidate", false),
        ("plan-cloud", "cloud-review", false),
        ("plan-provider", "cloud-review", true),
    ] {
        let fixture = ac9_fixture(ModelLaneRoutingPolicy::CloudReview, suffix).await;
        let mut launches = fixture.launches();
        let launch = launches
            .iter_mut()
            .find(|launch| launch.stage_id == stage_id)
            .expect("planned stage launch");
        if tamper_provider {
            launch.expected_provider = Some(ProviderKind::OfficialCli);
            launch.request.as_mut().expect("model request").provider =
                Some(ProviderKind::OfficialCli);
        } else {
            let replacement = ModelId::new_v7();
            launch.expected_model_id = replacement.to_string();
            launch
                .request
                .as_mut()
                .expect("model request")
                .instance_id
                .model_id = replacement;
            launch
                .generate_request
                .as_mut()
                .expect("generate request")
                .id = replacement;
        }
        let error = execute_production_routing_lifecycle(
            &fixture.coordinator,
            &fixture.execution_id,
            &fixture.decision_id,
            &fixture.authority,
            fixture.context.clone(),
            launches,
        )
        .await
        .expect_err("IPC-equivalent launch tamper must differ from selecting decision");
        assert!(error.to_string().contains("selecting-decision"));
    }
}

#[tokio::test]
async fn ac9_outbox_projection_and_pointer_tamper_fail_eventledger_replay() {
    let projection = ac9_fixture(ModelLaneRoutingPolicy::CloudReview, "outbox-projection").await;
    projection
        .wave()
        .await
        .expect("initialize outbox projection fixture");
    sqlx::query("UPDATE model_lane_routing_outbox SET command_json=jsonb_set(command_json,'{expected_model_id}','\"tampered\"'::jsonb) WHERE execution_id=$1 AND stage_id='local-candidate'")
        .bind(&projection.execution_id).execute(&projection.pool).await.expect("tamper outbox command");
    assert!(projection
        .wave()
        .await
        .expect_err("outbox projection drift must fail")
        .to_string()
        .contains("outbox"));

    let pointer = ac9_fixture(ModelLaneRoutingPolicy::CloudReview, "outbox-pointer").await;
    pointer
        .wave()
        .await
        .expect("initialize outbox pointer fixture");
    sqlx::query("UPDATE model_lane_routing_outbox SET event_ledger_seq=event_ledger_seq+1 WHERE execution_id=$1 AND stage_id='local-candidate'")
        .bind(&pointer.execution_id).execute(&pointer.pool).await.expect("tamper outbox pointer");
    assert!(pointer
        .wave()
        .await
        .expect_err("outbox pointer drift must fail")
        .to_string()
        .contains("outbox"));
}

#[tokio::test]
async fn ac9_cancel_and_peer_failure_propagate_into_blocked_factory_create() {
    let cancelled =
        Arc::new(ac9_fixture(ModelLaneRoutingPolicy::LocalFirst, "cancel-create").await);
    cancelled.hold_create.store(true, Ordering::SeqCst);
    let worker = {
        let fixture = cancelled.clone();
        tokio::spawn(async move { fixture.lifecycle().await })
    };
    ac9_wait_for_stage_state(
        &cancelled.pool,
        &cancelled.execution_id,
        "local-attempt",
        "in_flight",
    )
    .await;
    let state = cancelled
        .coordinator
        .cancel_routing_execution(&cancelled.execution_id, "cancel blocked create")
        .await
        .expect("pending create cancellation completes before DB terminalization");
    assert_eq!(state.status, ModelLaneRoutingExecutionStatus::Cancelled);
    assert!(worker.await.expect("join cancelled create worker").is_err());
    assert_eq!(cancelled.teardowns.load(Ordering::SeqCst), 0);

    let peer = Arc::new(ac9_fixture(ModelLaneRoutingPolicy::ParallelDebate, "peer-create").await);
    let local_model = peer
        .specs
        .iter()
        .find(|spec| spec.stage_id == "debate-local")
        .and_then(|spec| spec.instance_id)
        .expect("local peer")
        .model_id;
    let cloud_model = peer
        .specs
        .iter()
        .find(|spec| spec.stage_id == "debate-cloud")
        .and_then(|spec| spec.instance_id)
        .expect("cloud peer")
        .model_id;
    peer.held_models
        .lock()
        .expect("hold local create")
        .insert(local_model);
    *peer.fail_model.lock().expect("fail cloud create") = Some(cloud_model);
    assert!(peer.lifecycle().await.is_err());
    assert_eq!(peer.coordinator.live_session_count(), 0);
}

#[tokio::test]
async fn ac9_production_wrapper_rejects_unpersisted_selecting_decision_in_managed_postgres() {
    use handshake_core::swarm_orchestration::production_factory::{
        build_production_swarm_coordinator, CloudLaneFactoryConfig,
    };
    let (_pool, model_lane_store) = model_lane_store().await;
    let (ledger, _drain) = LedgerBatcher::manual_for_tests(
        LedgerBatcherConfig {
            capacity: 8,
            batch_size: 8,
            flush_interval: Duration::from_millis(250),
        },
        Arc::new(RecordingOverflowSink::default()),
    )
    .expect("manual AC-9 production ProcessOwnershipLedger writer");
    let coordinator = build_production_swarm_coordinator(
        ledger,
        CloudLaneFactoryConfig::unconfigured(),
        model_lane_store,
        Some(2),
        uuid::Uuid::now_v7(),
        |_| Ok(()),
    );
    let run_id = format!("run-ac9-unknown-decision-{}", uuid::Uuid::now_v7());
    let error = execute_production_routing_wave(
        &coordinator,
        "execution-ac9-unknown-decision",
        "promotion-decision://ac9/missing",
        &ModelLaneRoutingAuthority::default(),
        ModelLaneRoutingExecutionContext {
            run_id: run_id.clone(),
            trace_id: format!("trace-{run_id}"),
            run_span_id: format!("span-{run_id}"),
            coordinator_session_id: format!("coordinator-{run_id}"),
            locus_ref: format!("locus://{run_id}"),
            work_packet_id: WP_ID.into(),
            micro_task_id: Some(MT_ID.into()),
            task_board_id: TASK_BOARD_ID.into(),
            owner_session: OWNER.into(),
            initial_input_ref: format!("model-lane-message://missing-{run_id}"),
            initial_input_sha256: sample_sha256(),
        },
        Vec::new(),
    )
    .await
    .expect_err("production wrapper must require persisted selecting decision authority");
    assert!(error.to_string().contains("selecting promotion decision"));
}

#[tokio::test]
async fn ac9_reassignment_cannot_cross_post_validation_output_barrier_or_create_stale_rows() {
    let fixture = Arc::new(ac9_fixture(ModelLaneRoutingPolicy::LocalFirst, "output-toctou").await);
    fixture.hold_generation.store(true, Ordering::SeqCst);
    let worker = {
        let fixture = fixture.clone();
        tokio::spawn(async move { fixture.lifecycle().await })
    };
    ac9_wait_for_stage_state(
        &fixture.pool,
        &fixture.execution_id,
        "local-attempt",
        "in_flight",
    )
    .await;

    let barrier_key = format!("routing-output:{}:local-attempt:1", fixture.execution_id);
    let mut barrier_connection = fixture
        .pool
        .acquire()
        .await
        .expect("acquire barrier connection");
    sqlx::query("SELECT pg_advisory_lock(hashtextextended($1, 0))")
        .bind(&barrier_key)
        .execute(&mut *barrier_connection)
        .await
        .expect("hold deterministic post-validation barrier");
    fixture.hold_generation.store(false, Ordering::SeqCst);

    for _ in 0..200 {
        let waiting: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM pg_locks WHERE locktype='advisory' AND NOT granted)",
        )
        .fetch_one(&fixture.pool)
        .await
        .expect("observe output barrier waiter");
        if waiting {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let waiting: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM pg_locks WHERE locktype='advisory' AND NOT granted)",
    )
    .fetch_one(&fixture.pool)
    .await
    .expect("confirm output barrier waiter");
    assert!(
        waiting,
        "output writer reached the barrier after exact claim validation"
    );

    let reassignment = {
        let pool = fixture.pool.clone();
        let execution_id = fixture.execution_id.clone();
        tokio::spawn(async move {
            ac9_force_expired_lease(&pool, &execution_id, "local-attempt").await;
        })
    };
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(
        !reassignment.is_finished(),
        "reassignment must block on the execution row locked by validated output"
    );

    sqlx::query("SELECT pg_advisory_unlock(hashtextextended($1, 0))")
        .bind(&barrier_key)
        .execute(&mut *barrier_connection)
        .await
        .expect("release deterministic output barrier");
    worker
        .await
        .expect("join output writer")
        .expect("validated output commits atomically");
    reassignment.await.expect("join attempted reassignment");
    let recovered = fixture
        .coordinator
        .recover_routing_execution(&fixture.execution_id, fixture.launches())
        .await
        .expect("recovery observes committed terminal attempt");
    assert_eq!(recovered.execution.stages["local-attempt"].attempt, 1);
    assert_eq!(
        recovered.execution.stages["local-attempt"].state,
        ModelLaneRoutingStageStateKind::Succeeded
    );

    let message_rows: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM model_lane_messages WHERE message_id=$1")
            .bind(format!(
                "routing-output:{}:local-attempt:1",
                fixture.execution_id
            ))
            .fetch_one(&fixture.pool)
            .await
            .expect("count fenced output messages");
    let artifact_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM model_lane_context_bundle_artifacts WHERE artifact_binding_id=$1",
    )
    .bind(format!(
        "routing-output-binding:{}:local-attempt:1",
        fixture.execution_id
    ))
    .fetch_one(&fixture.pool)
    .await
    .expect("count fenced output artifacts");
    let reassigned_attempt_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM model_lane_routing_stage_attempts WHERE execution_id=$1 AND stage_id='local-attempt' AND attempt=2",
    )
    .bind(&fixture.execution_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("count forbidden reassigned attempts");
    assert_eq!(
        (message_rows, artifact_rows, reassigned_attempt_rows),
        (1, 1, 0)
    );
}

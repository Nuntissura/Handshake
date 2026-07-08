//! WP-1 MT-006: Dexterity cloud projection and consent runtime proof.
//!
//! These tests use real PostgreSQL plus the kernel EventLedger. They prove that
//! BYOK cloud lanes cannot launch from synthetic refs, cannot call a provider
//! before durable ProjectionPlan/ConsentReceipt authority exists, and remain
//! advisory until promoted by Dexterity.

mod knowledge_pg_support;

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use async_trait::async_trait;
use handshake_core::model_runtime::registry::RuntimeBinding as RuntimeAdapterBinding;
use handshake_core::model_runtime::{ModelId, ProviderKind};
use handshake_core::process_ledger::{LedgerBatcher, LedgerBatcherConfig, NoopOverflowSink};
use handshake_core::swarm_orchestration::model_lane::{
    DexterityLaunchContract, LaunchAuthority, ModelLaneAuthority,
    ModelLaneCloudConsentReceiptStatus, ModelLaneCloudConsentScope, ModelLaneCloudExportPosture,
    ModelLaneCloudProjectionPlanStatus, ModelLaneCloudRetentionPolicy, ModelLaneKind,
    ModelLaneLocusBinding, ModelLaneMessageKind, ModelLaneProviderKind, ModelLaneRecoveryState,
    ModelLaneRoutingMetadata, ModelLaneStatus, ModelLaneStore, ModelLaneTarget, NewModelLane,
    NewModelLaneCloudConsentReceipt, NewModelLaneCloudProjectionPlan, NewModelLaneMessage,
    NewModelLaneRun, RuntimeBinding,
};
use handshake_core::swarm_orchestration::{
    ByokCloudProvider, LiveSession, ModelInstanceId, ModelSessionFactory, RecordingSwarmSink,
    RunBudget, SpawnRequest, SwarmConfig, SwarmCoordinator, SwarmError,
};
use serde_json::{json, Value};

const WP_ID: &str = "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1";
const MT_ID: &str = "MT-006";
const TASK_BOARD_ID: &str = "task-board://wp-1";
const OWNER: &str = "KERNEL_BUILDER-MT006";
const USERMANUAL_BEHAVIOR: &str = "usermanual://model-lane-cloud-projection-consent#launch";

#[tokio::test]
async fn cloud_projection_and_consent_receipts_persist_and_replay() {
    let (pool, store) = model_lane_store().await;
    let plan = sample_projection_plan("run-cloud-ok", "lane-cloud-ok", "openai");
    let stored_plan = store
        .record_cloud_projection_plan(plan.clone())
        .await
        .expect("record cloud ProjectionPlan authority");
    assert!(stored_plan.event_ledger_event_id.starts_with("KE-"));
    assert_eq!(
        stored_plan.status,
        ModelLaneCloudProjectionPlanStatus::Active
    );
    assert_eq!(stored_plan.scope_hash, sample_scope_hash());

    let receipt = sample_consent_receipt(
        "run-cloud-ok",
        "lane-cloud-ok",
        "openai",
        &stored_plan.projection_plan_id,
        &stored_plan.projection_plan_hash,
        ModelLaneCloudConsentReceiptStatus::Approved,
        "2026-06-29T00:00:00Z",
        "2027-06-29T00:00:00Z",
    );
    let stored_receipt = store
        .record_cloud_consent_receipt(receipt)
        .await
        .expect("record cloud ConsentReceipt authority");
    assert!(stored_receipt.event_ledger_event_id.starts_with("KE-"));
    assert_eq!(
        stored_receipt.projection_plan_hash,
        stored_plan.projection_plan_hash
    );

    let authority = store
        .replay_cloud_consent_authority("run-cloud-ok")
        .await
        .expect("replay cloud consent authority");
    assert_eq!(authority.projection_plans.len(), 1);
    assert_eq!(authority.consent_receipts.len(), 1);
    assert_eq!(
        authority.projection_plans[0].event_ledger_seq,
        stored_plan.event_ledger_seq
    );
    assert_eq!(
        authority.consent_receipts[0].event_ledger_seq,
        stored_receipt.event_ledger_seq
    );

    let (run, lane) =
        sample_cloud_run_lane("run-cloud-ok", "lane-cloud-ok", ModelLaneStatus::Ready);
    let (_stored_run, stored_lane) = store
        .record_prepared_launch((run, lane))
        .await
        .expect("durable ProjectionPlan/ConsentReceipt allows cloud lane launch");
    assert_eq!(stored_lane.provider_kind, ModelLaneProviderKind::OpenAi);
    assert_eq!(
        stored_lane.projection_plan_ref.as_deref(),
        Some("cloud-projection-plan://run-cloud-ok/lane-cloud-ok")
    );
    assert_eq!(
        stored_lane.consent_receipt_ref.as_deref(),
        Some("cloud-consent-receipt://run-cloud-ok/lane-cloud-ok")
    );

    let advisory = store
        .record_message(cloud_advisory_message("run-cloud-ok", "lane-cloud-ok"))
        .await
        .expect("cloud advisory output records projection metadata");
    assert_eq!(advisory.authority, ModelLaneAuthority::Advisory);
    assert_eq!(
        advisory.diagnostic_payload["projection_plan_id"],
        json!("cloud-projection-plan://run-cloud-ok/lane-cloud-ok")
    );
    assert_eq!(
        advisory.diagnostic_payload["redaction_policy_ref"],
        json!("redaction-policy://mt006/cloud-safe")
    );
    assert_eq!(
        advisory.diagnostic_payload["flight_recorder"],
        json!("EventLedger cloud consent/projection authority")
    );

    let promoted_err = store
        .record_message(cloud_promoted_without_gate("run-cloud-ok", "lane-cloud-ok"))
        .await
        .expect_err("cloud provider output cannot become authority without PromotionGate");
    assert!(
        promoted_err.to_string().contains("PromotionGate"),
        "expected PromotionGate failure, got {promoted_err}"
    );

    let operator_decision_err = store
        .record_message(cloud_operator_decision_message(
            "run-cloud-ok",
            "lane-cloud-ok",
        ))
        .await
        .expect_err("cloud output cannot self-assert operator authority");
    assert!(
        operator_decision_err
            .to_string()
            .contains("Cloud ModelLaneMessage authority"),
        "cloud operator authority shortcut must fail explicitly: {operator_decision_err}"
    );
    let validator_verdict_err = store
        .record_message(cloud_validator_verdict_message(
            "run-cloud-ok",
            "lane-cloud-ok",
        ))
        .await
        .expect_err("cloud output cannot self-assert validator authority");
    assert!(
        validator_verdict_err
            .to_string()
            .contains("Cloud ModelLaneMessage authority"),
        "cloud validator authority shortcut must fail explicitly: {validator_verdict_err}"
    );

    let mut hidden_payload = cloud_message(
        "run-cloud-ok",
        "lane-cloud-ok",
        "msg-cloud-hidden-payload",
        ModelLaneAuthority::Advisory,
    );
    hidden_payload.payload_ref = "provider-session://openai/thread-hidden".into();
    let hidden_payload_err = store
        .record_message(hidden_payload)
        .await
        .expect_err("hidden provider session cannot become advisory payload authority");
    assert!(
        hidden_payload_err
            .to_string()
            .contains("hidden provider/session memory"),
        "hidden payload ref must fail explicitly: {hidden_payload_err}"
    );

    let mut hidden_promoted_artifact = cloud_message(
        "run-cloud-ok",
        "lane-cloud-ok",
        "msg-cloud-hidden-promoted-artifact",
        ModelLaneAuthority::Promoted,
    );
    hidden_promoted_artifact.promotion_decision_id = Some("promotion-decision://mt006/fake".into());
    hidden_promoted_artifact.promotion_gate_ref = Some("promotion-gate://mt006/fake".into());
    hidden_promoted_artifact.promotion_receipt_ref = Some("promotion-receipt://mt006/fake".into());
    hidden_promoted_artifact.promoted_artifact_ref =
        Some("provider-memory://openai/promoted-hidden".into());
    hidden_promoted_artifact.promoted_artifact_sha256 = Some(sample_sha256());
    hidden_promoted_artifact.promoted_artifact_version = Some("v1".into());
    let hidden_promoted_artifact_err = store
        .record_message(hidden_promoted_artifact)
        .await
        .expect_err("hidden provider memory cannot become promoted artifact authority");
    assert!(
        hidden_promoted_artifact_err
            .to_string()
            .contains("hidden provider/session memory"),
        "hidden promoted artifact ref must fail explicitly: {hidden_promoted_artifact_err}"
    );

    let ledger_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger \
         WHERE session_run_id = $1 \
           AND aggregate_type IN ( \
             'model_lane_cloud_projection_plan', \
             'model_lane_cloud_consent_receipt', \
             'model_lane', \
             'model_lane_message' \
           )",
    )
    .bind("mlane-stream-run-cloud-ok")
    .fetch_one(&pool)
    .await
    .expect("count cloud policy EventLedger rows");
    assert_eq!(ledger_rows, 4);

    let registry_rows = store
        .schema_registry_rows()
        .await
        .expect("schema registry rows");
    assert!(registry_rows
        .iter()
        .any(|row| row.schema_id == "hsk.model_lane_cloud_projection_plan@1"));
    assert!(registry_rows
        .iter()
        .any(|row| row.schema_id == "hsk.model_lane_cloud_consent_receipt@1"));
    assert!(registry_rows
        .iter()
        .any(|row| row.schema_id == "hsk.model_lane_cloud_consent_denial@1"));
}

#[tokio::test]
async fn cloud_lane_rejects_missing_expired_mismatched_and_revoked_consent() {
    let (pool, store) = model_lane_store().await;

    let (missing_run, missing_lane) = sample_cloud_run_lane(
        "run-cloud-missing",
        "lane-cloud-missing",
        ModelLaneStatus::Ready,
    );
    let missing_err = store
        .record_prepared_launch((missing_run, missing_lane))
        .await
        .expect_err("missing ProjectionPlan/ConsentReceipt must fail closed");
    assert_cx_mm_007(&missing_err);
    assert_no_lane_row(&pool, "lane-cloud-missing").await;
    assert_denial_event(&pool, "run-cloud-missing", "lane-cloud-missing").await;
    let missing_retry_err = store
        .record_prepared_launch(sample_cloud_run_lane(
            "run-cloud-missing",
            "lane-cloud-missing",
            ModelLaneStatus::Ready,
        ))
        .await
        .expect_err("duplicate missing consent launch must replay denial deterministically");
    assert_cx_mm_007(&missing_retry_err);
    assert_denial_event_count(&pool, "lane-cloud-missing", 1).await;

    seed_cloud_authority(
        &store,
        "run-cloud-expired",
        "lane-cloud-expired",
        "openai",
        ModelLaneCloudConsentReceiptStatus::Approved,
        "2024-01-01T00:00:00Z",
        "2025-01-01T00:00:00Z",
    )
    .await;
    let expired_err = store
        .record_prepared_launch(sample_cloud_run_lane(
            "run-cloud-expired",
            "lane-cloud-expired",
            ModelLaneStatus::Ready,
        ))
        .await
        .expect_err("expired ConsentReceipt must fail closed");
    assert_cx_mm_007(&expired_err);
    assert_no_lane_row(&pool, "lane-cloud-expired").await;

    let plan = store
        .record_cloud_projection_plan(sample_projection_plan(
            "run-cloud-mismatch",
            "lane-cloud-mismatch",
            "openai",
        ))
        .await
        .expect("record mismatch projection");
    let mismatched = sample_consent_receipt(
        "run-cloud-mismatch",
        "lane-cloud-mismatch",
        "anthropic",
        &plan.projection_plan_id,
        &sample_other_hash(),
        ModelLaneCloudConsentReceiptStatus::Approved,
        "2026-06-29T00:00:00Z",
        "2027-06-29T00:00:00Z",
    );
    store
        .record_cloud_consent_receipt(mismatched)
        .await
        .expect("record durable but mismatched ConsentReceipt evidence");
    let mismatch_err = store
        .record_prepared_launch(sample_cloud_run_lane(
            "run-cloud-mismatch",
            "lane-cloud-mismatch",
            ModelLaneStatus::Ready,
        ))
        .await
        .expect_err("mismatched ConsentReceipt must fail closed");
    assert_cx_mm_007(&mismatch_err);
    assert_no_lane_row(&pool, "lane-cloud-mismatch").await;

    seed_cloud_authority(
        &store,
        "run-cloud-revoked",
        "lane-cloud-revoked",
        "openai",
        ModelLaneCloudConsentReceiptStatus::Approved,
        "2026-06-29T00:00:00Z",
        "2027-06-29T00:00:00Z",
    )
    .await;
    store
        .revoke_cloud_consent_receipt(
            "cloud-consent-receipt://run-cloud-revoked/lane-cloud-revoked",
            "operator://mt006/revoke-before-launch",
            "operator revoked before provider call",
        )
        .await
        .expect("revoke ConsentReceipt");
    let revoked_err = store
        .record_prepared_launch(sample_cloud_run_lane(
            "run-cloud-revoked",
            "lane-cloud-revoked",
            ModelLaneStatus::Ready,
        ))
        .await
        .expect_err("revoked ConsentReceipt must fail closed");
    assert_cx_mm_007(&revoked_err);
    assert_no_lane_row(&pool, "lane-cloud-revoked").await;

    let calls = Arc::new(AtomicUsize::new(0));
    let (ledger, _drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("manual process ledger");
    let coordinator = SwarmCoordinator::new_with_model_lane_store(
        SwarmConfig::new(RunBudget::defaulted(1)),
        Arc::new(CountingFactory {
            calls: calls.clone(),
        }),
        Arc::new(RecordingSwarmSink::new()),
        ledger,
        store.clone(),
    );
    let err = coordinator
        .spawn_session(cloud_spawn_request(
            "run-cloud-preflight-missing",
            "lane-cloud-preflight-missing",
        ))
        .await
        .expect_err("missing durable cloud consent must stop before factory call");
    assert!(matches!(err, SwarmError::LedgerFailed(_)), "got {err}");
    assert_cx_mm_007_string(&err.to_string());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_no_lane_row(&pool, "lane-cloud-preflight-missing").await;
    assert_denial_event(
        &pool,
        "run-cloud-preflight-missing",
        "lane-cloud-preflight-missing",
    )
    .await;

    let mut missing_projection = cloud_spawn_request(
        "run-cloud-missing-projection-ref",
        "lane-cloud-missing-projection-ref",
    );
    missing_projection
        .dexterity_launch
        .as_mut()
        .expect("cloud Dexterity contract")
        .projection_plan_ref = None;
    let err = coordinator
        .spawn_session(missing_projection)
        .await
        .expect_err("missing projection ref must be denied before factory call");
    assert_cx_mm_007_string(&err.to_string());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_denial_event(
        &pool,
        "run-cloud-missing-projection-ref",
        "lane-cloud-missing-projection-ref",
    )
    .await;

    store
        .record_cloud_projection_plan(sample_projection_plan(
            "run-cloud-missing-consent-ref",
            "lane-cloud-missing-consent-ref",
            "openai",
        ))
        .await
        .expect("record projection so missing consent is the failing condition");
    let mut missing_consent = cloud_spawn_request(
        "run-cloud-missing-consent-ref",
        "lane-cloud-missing-consent-ref",
    );
    missing_consent
        .dexterity_launch
        .as_mut()
        .expect("cloud Dexterity contract")
        .consent_receipt_ref = None;
    let err = coordinator
        .spawn_session(missing_consent)
        .await
        .expect_err("missing consent ref must be denied before factory call");
    assert_cx_mm_007_string(&err.to_string());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_denial_event(
        &pool,
        "run-cloud-missing-consent-ref",
        "lane-cloud-missing-consent-ref",
    )
    .await;

    let mut missing_byok_provider = cloud_spawn_request(
        "run-cloud-missing-byok-provider",
        "lane-cloud-missing-byok-provider",
    );
    missing_byok_provider.byok_cloud_provider = None;
    let err = coordinator
        .spawn_session(missing_byok_provider)
        .await
        .expect_err("missing BYOK provider must be denied before factory call");
    assert_cx_mm_007_string(&err.to_string());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_denial_event(
        &pool,
        "run-cloud-missing-byok-provider",
        "lane-cloud-missing-byok-provider",
    )
    .await;
}

#[tokio::test]
async fn cloud_consent_revocation_cancels_pending_lanes_with_eventledger_evidence() {
    let (pool, store) = model_lane_store().await;
    seed_cloud_authority(
        &store,
        "run-cloud-cancel",
        "lane-cloud-cancel",
        "openai",
        ModelLaneCloudConsentReceiptStatus::Approved,
        "2026-06-29T00:00:00Z",
        "2027-06-29T00:00:00Z",
    )
    .await;
    store
        .record_prepared_launch(sample_cloud_run_lane(
            "run-cloud-cancel",
            "lane-cloud-cancel",
            ModelLaneStatus::Running,
        ))
        .await
        .expect("launch cloud lane before consent revocation");

    let cancelled = store
        .revoke_cloud_consent_receipt(
            "cloud-consent-receipt://run-cloud-cancel/lane-cloud-cancel",
            "operator://mt006/runtime-revoke",
            "operator revoked cloud consent while lane was pending",
        )
        .await
        .expect("revocation cancels covered cloud lanes");
    assert_eq!(cancelled.len(), 1);
    assert_eq!(cancelled[0].status, ModelLaneStatus::Cancelled);
    assert_eq!(cancelled[0].failstate_code.as_deref(), Some("CX-MM-007"));
    assert_eq!(
        cancelled[0].recovery_state,
        ModelLaneRecoveryState::Terminal
    );

    let replay = store
        .replay_run("run-cloud-cancel")
        .await
        .expect("replay cancelled cloud lane");
    assert_eq!(replay.lanes.len(), 1);
    assert_eq!(replay.lanes[0].status, ModelLaneStatus::Cancelled);
    assert_eq!(replay.lanes[0].failstate_code.as_deref(), Some("CX-MM-007"));

    let terminal_payload: Value = sqlx::query_scalar(
        "SELECT payload FROM kernel_event_ledger \
         WHERE aggregate_type = 'model_lane_terminal' \
           AND aggregate_id = 'lane-cloud-cancel' \
         ORDER BY event_sequence DESC LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .expect("terminal EventLedger payload");
    assert_eq!(terminal_payload["consent_status"], json!("CX-MM-007"));
    assert_eq!(
        terminal_payload["consent_receipt_id"],
        json!("cloud-consent-receipt://run-cloud-cancel/lane-cloud-cancel")
    );
    assert_eq!(terminal_payload["provider_call_cancelled"], json!(true));
    assert_eq!(terminal_payload["flight_recorder"], json!("EventLedger"));

    let retry_cancelled = store
        .revoke_cloud_consent_receipt(
            "cloud-consent-receipt://run-cloud-cancel/lane-cloud-cancel",
            "operator://mt006/runtime-revoke",
            "operator revoked cloud consent while lane was pending",
        )
        .await
        .expect("duplicate revocation replays without EventLedger idempotency conflict");
    assert!(
        retry_cancelled.is_empty(),
        "already-revoked receipt should not recancel terminal lanes"
    );
    let revocation_events: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger \
         WHERE aggregate_type = 'model_lane_cloud_consent_receipt' \
           AND aggregate_id = $1 \
           AND payload->>'reason_code' = 'CX-MM-007'",
    )
    .bind("cloud-consent-receipt://run-cloud-cancel/lane-cloud-cancel")
    .fetch_one(&pool)
    .await
    .expect("count revocation EventLedger rows");
    assert_eq!(revocation_events, 1);
}

/// WP-1 MT-006: cloud authority is LANE-BOUND. A ProjectionPlan + ConsentReceipt
/// issued for `lane-cloud-a` must not authorize a different lane in the same run.
/// Preflight (`ensure_cloud_launch_authority_tx`) requires the plan/receipt
/// `lane_id` to equal the launching lane, so cross-lane reuse of cloud consent
/// fails closed BEFORE any provider call: no `model_lanes` row is created and a
/// durable CX-MM-007 denial event is appended for the rejected lane.
///
/// This closes the untested half of the consent boundary: the suite proved
/// missing/expired/mismatched/revoked receipts, but never that an otherwise
/// VALID, APPROVED receipt cannot be borrowed by a sibling lane.
#[tokio::test]
async fn cloud_consent_receipt_bound_to_other_lane_fails_closed() {
    let (pool, store) = model_lane_store().await;

    // Durable, approved, in-window cloud authority exists ONLY for lane-cloud-a.
    seed_cloud_authority(
        &store,
        "run-cloud-crosslane",
        "lane-cloud-a",
        "openai",
        ModelLaneCloudConsentReceiptStatus::Approved,
        "2026-06-29T00:00:00Z",
        "2027-06-29T00:00:00Z",
    )
    .await;

    // lane-cloud-b attempts to launch by borrowing lane-cloud-a's plan + receipt.
    let (mut run, mut lane) = sample_cloud_run_lane(
        "run-cloud-crosslane",
        "lane-cloud-b",
        ModelLaneStatus::Running,
    );
    let borrowed_plan = projection_plan_id("run-cloud-crosslane", "lane-cloud-a");
    let borrowed_receipt = consent_receipt_id("run-cloud-crosslane", "lane-cloud-a");
    run.projection_plan_ref = Some(borrowed_plan.clone());
    run.consent_receipt_ref = Some(borrowed_receipt.clone());
    lane.projection_plan_ref = Some(borrowed_plan);
    lane.consent_receipt_ref = Some(borrowed_receipt);

    let err = store
        .record_prepared_launch((run, lane))
        .await
        .expect_err("cloud authority bound to another lane must not authorize this lane");
    let msg = err.to_string();
    assert!(
        msg.contains("CX-MM-007"),
        "cross-lane cloud authority reuse must be denied with CX-MM-007, got: {msg}"
    );
    assert!(
        msg.contains("lane_id"),
        "denial must name the lane_id mismatch, got: {msg}"
    );

    // Fail-closed: no partial authority state, and the denial is durable evidence.
    assert_no_lane_row(&pool, "lane-cloud-b").await;
    assert_denial_event(&pool, "run-cloud-crosslane", "lane-cloud-b").await;
}

async fn model_lane_store() -> (sqlx::PgPool, ModelLaneStore) {
    let kpg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("PostgreSQL/EventLedger is required for MT-006 proof");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&kpg.schema_url)
        .await
        .expect("connect isolated Dexterity cloud policy schema");
    let store = ModelLaneStore::new(pool.clone());
    (pool, store)
}

async fn seed_cloud_authority(
    store: &ModelLaneStore,
    run_id: &str,
    lane_id: &str,
    provider_kind: &str,
    status: ModelLaneCloudConsentReceiptStatus,
    valid_from_utc: &str,
    valid_until_utc: &str,
) {
    let plan = store
        .record_cloud_projection_plan(sample_projection_plan(run_id, lane_id, provider_kind))
        .await
        .expect("record ProjectionPlan authority");
    store
        .record_cloud_consent_receipt(sample_consent_receipt(
            run_id,
            lane_id,
            provider_kind,
            &plan.projection_plan_id,
            &plan.projection_plan_hash,
            status,
            valid_from_utc,
            valid_until_utc,
        ))
        .await
        .expect("record ConsentReceipt authority");
}

fn sample_projection_plan(
    run_id: &str,
    lane_id: &str,
    provider_kind: &str,
) -> NewModelLaneCloudProjectionPlan {
    NewModelLaneCloudProjectionPlan {
        projection_plan_id: projection_plan_id(run_id, lane_id),
        run_id: run_id.into(),
        trace_id: format!("trace-{run_id}"),
        lane_id: lane_id.into(),
        model_session_id: model_session_id_for(lane_id),
        provider_kind: provider_kind.into(),
        requested_model_id: requested_model_id(provider_kind),
        scope_hash: sample_scope_hash(),
        source_artifact_refs: vec![
            format!("artifact-store://mt006/{run_id}/{lane_id}/context.json"),
            "context-bundle://mt006/cloud-safe".into(),
        ],
        payload_artifact_ref: format!("artifact-store://mt006/{run_id}/{lane_id}/payload.json"),
        payload_sha256: sample_sha256(),
        redaction_policy_ref: "redaction-policy://mt006/cloud-safe".into(),
        redaction_summary: "workspace-local secrets and local-only memory are excluded".into(),
        retention_policy: ModelLaneCloudRetentionPolicy::NoTrainingEphemeral,
        export_posture: ModelLaneCloudExportPosture::RedactedContextOnly,
        provider_profile_ref: format!("provider-profile://mt006/{provider_kind}"),
        fan_out_targets: vec![format!("provider://{provider_kind}/byok")],
        consent_scope: ModelLaneCloudConsentScope::SingleLane,
        status: ModelLaneCloudProjectionPlanStatus::Active,
        event_ledger_stream_id: event_stream_id(run_id),
        work_packet_id: WP_ID.into(),
        micro_task_id: MT_ID.into(),
        task_board_id: TASK_BOARD_ID.into(),
        owner_session: OWNER.into(),
        idempotency_key: format!("idem-projection-{run_id}-{lane_id}"),
        created_at_utc: "2026-06-29T09:00:00Z".into(),
        user_manual_behavior_ref: USERMANUAL_BEHAVIOR.into(),
        diagnostic_payload: json!({
            "flight_recorder": "EventLedger",
            "internal_diagnostics": "deferred: MT-006 backend policy exposes EventLedger payloads; internal_diagnostics surface ships separately",
            "palmistry": "deferred: Palmistry external watcher is linked by behavior ref when available",
            "locus": format!("locus://wp1/mt006/{run_id}/{lane_id}")
        }),
    }
}

fn sample_consent_receipt(
    run_id: &str,
    lane_id: &str,
    provider_kind: &str,
    projection_plan_id: &str,
    projection_plan_hash: &str,
    status: ModelLaneCloudConsentReceiptStatus,
    valid_from_utc: &str,
    valid_until_utc: &str,
) -> NewModelLaneCloudConsentReceipt {
    NewModelLaneCloudConsentReceipt {
        consent_receipt_id: consent_receipt_id(run_id, lane_id),
        projection_plan_id: projection_plan_id.into(),
        projection_plan_hash: projection_plan_hash.into(),
        run_id: run_id.into(),
        trace_id: format!("trace-{run_id}"),
        lane_id: lane_id.into(),
        model_session_id: model_session_id_for(lane_id),
        provider_kind: provider_kind.into(),
        requested_model_id: requested_model_id(provider_kind),
        scope_hash: sample_scope_hash(),
        consent_scope: ModelLaneCloudConsentScope::SingleLane,
        retention_policy: ModelLaneCloudRetentionPolicy::NoTrainingEphemeral,
        export_posture: ModelLaneCloudExportPosture::RedactedContextOnly,
        fan_out_targets: vec![format!("provider://{provider_kind}/byok")],
        approved: status == ModelLaneCloudConsentReceiptStatus::Approved,
        approved_by_ref: "operator://mt006/approval".into(),
        approved_at_utc: "2026-06-29T09:00:10Z".into(),
        valid_from_utc: valid_from_utc.into(),
        valid_until_utc: valid_until_utc.into(),
        revoked_at_utc: None,
        revocation_ref: None,
        status,
        event_ledger_stream_id: event_stream_id(run_id),
        work_packet_id: WP_ID.into(),
        micro_task_id: MT_ID.into(),
        task_board_id: TASK_BOARD_ID.into(),
        owner_session: OWNER.into(),
        idempotency_key: format!("idem-consent-{run_id}-{lane_id}"),
        created_at_utc: "2026-06-29T09:00:15Z".into(),
        user_manual_behavior_ref: USERMANUAL_BEHAVIOR.into(),
        diagnostic_payload: json!({
            "flight_recorder": "EventLedger",
            "provider_call_attempted": false,
            "locus": format!("locus://wp1/mt006/{run_id}/{lane_id}")
        }),
    }
}

fn sample_cloud_run_lane(
    run_id: &str,
    lane_id: &str,
    status: ModelLaneStatus,
) -> (NewModelLaneRun, NewModelLane) {
    (
        NewModelLaneRun {
            run_id: run_id.into(),
            trace_id: format!("trace-{run_id}"),
            run_span_id: format!("span-{run_id}"),
            coordinator_session_id: format!("coordinator-session-{run_id}"),
            routing_policy: "cloud_plan_local_execute".into(),
            context_bundle_id: format!("context-bundle://mt006/{run_id}/bootstrap"),
            lane_ids: vec![lane_id.into()],
            event_ledger_stream_id: event_stream_id(run_id),
            artifact_namespace: format!("artifact://model-lane/mt006/{run_id}"),
            projection_plan_ref: Some(projection_plan_id(run_id, lane_id)),
            consent_receipt_ref: Some(consent_receipt_id(run_id, lane_id)),
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
            recovery_hint_ref: Some(
                "usermanual://model-lane-cloud-projection-consent#recovery".into(),
            ),
            locus_binding: Some(sample_locus(run_id, lane_id)),
            memory_pack_ref: format!("memory-pack://fems/mt006/{run_id}"),
            memory_pack_hash: sample_sha256(),
            determinism_mode: "deterministic_replay".into(),
            budget_summary_ref: format!("budget://mt006/{run_id}"),
            selected_model_id: Some("model://dexterity/byok_cloud/gpt-4o-mini".into()),
            candidate_model_ids: vec!["model://dexterity/byok_cloud/gpt-4o-mini".into()],
            procedural_review_status: "cloud_projection_consent_preflighted".into(),
            truncation_warning_ref: None,
            rejection_reason_refs: vec![],
        },
        NewModelLane {
            lane_id: lane_id.into(),
            run_id: run_id.into(),
            trace_id: format!("trace-{run_id}"),
            lane_span_id: format!("span-{lane_id}"),
            event_ledger_stream_id: event_stream_id(run_id),
            kind: ModelLaneKind::CloudModel,
            role: "cloud-review-lane".into(),
            backend: "cloud_lane_openai".into(),
            model_id: Some("model://dexterity/byok_cloud/gpt-4o-mini".into()),
            session_id: format!("session-{lane_id}"),
            model_session_id: model_session_id_for(lane_id),
            adapter_id: "openai_byok".into(),
            runtime_binding: RuntimeBinding::Cloud,
            launch_authority: LaunchAuthority::CloudLane,
            provider_kind: ModelLaneProviderKind::OpenAi,
            capability_token_ids: vec!["capability://dexterity/cloud-generate".into()],
            effective_capability_snapshot_ref: Some(format!("capability-snapshot://{lane_id}")),
            capability_negotiation_ref: Some(format!("capability-negotiation://{lane_id}")),
            provider_feature_profile_ref: Some("provider-profile://mt006/openai".into()),
            requested_execution_policy_ref: Some(format!("execution-policy://requested/{lane_id}")),
            effective_execution_policy_ref: Some(format!("execution-policy://effective/{lane_id}")),
            projection_plan_ref: Some(projection_plan_id(run_id, lane_id)),
            consent_receipt_ref: Some(consent_receipt_id(run_id, lane_id)),
            tool_gate_decision_refs: vec!["toolgate://mt006/cloud-read-context".into()],
            status,
            recovery_state: ModelLaneRecoveryState::Restartable,
            heartbeat_at_utc: Some("2026-06-29T09:01:00Z".into()),
            lease_expires_at_utc: Some("2026-06-29T09:10:00Z".into()),
            reclaim_after_utc: Some("2026-06-29T09:11:00Z".into()),
            restart_generation: 0,
            cancellation_ref: Some(format!("cancel-token://{lane_id}")),
            reclaim_policy_ref: Some("reclaim-policy://mt006/cloud".into()),
            terminal_status_mapping_ref: Some("terminal-status://mt006/cloud".into()),
            process_ownership_ref: Some(format!("process-ledger://{lane_id}")),
            no_os_process_reason_ref: None,
            backpressure_ref: None,
            loop_counter_ref: Some("loop-counter://mt006".into()),
            last_runtime_status_ref: Some("runtime-status://cloud-ready".into()),
            last_recovery_event_ref: None,
            failstate_code: None,
            startup_failure_ref: None,
            reason_ref: None,
            recovery_hint_ref: Some(
                "usermanual://model-lane-cloud-projection-consent#recovery".into(),
            ),
            work_packet_id: Some(WP_ID.into()),
            micro_task_id: Some(MT_ID.into()),
            task_board_id: Some(TASK_BOARD_ID.into()),
            owner_session: OWNER.into(),
            locus_binding: Some(sample_locus(run_id, lane_id)),
        },
    )
}

fn cloud_advisory_message(run_id: &str, lane_id: &str) -> NewModelLaneMessage {
    cloud_message(
        run_id,
        lane_id,
        "msg-cloud-advisory",
        ModelLaneAuthority::Advisory,
    )
}

fn cloud_promoted_without_gate(run_id: &str, lane_id: &str) -> NewModelLaneMessage {
    cloud_message(
        run_id,
        lane_id,
        "msg-cloud-promoted-no-gate",
        ModelLaneAuthority::Promoted,
    )
}

fn cloud_operator_decision_message(run_id: &str, lane_id: &str) -> NewModelLaneMessage {
    let mut message = cloud_message(
        run_id,
        lane_id,
        "msg-cloud-operator-decision",
        ModelLaneAuthority::OperatorDecision,
    );
    message.operator_decision_ref = Some(format!("operator-decision://mt006/{run_id}/{lane_id}"));
    message
}

fn cloud_validator_verdict_message(run_id: &str, lane_id: &str) -> NewModelLaneMessage {
    let mut message = cloud_message(
        run_id,
        lane_id,
        "msg-cloud-validator-verdict",
        ModelLaneAuthority::ValidatorVerdict,
    );
    message.validator_verdict_ref = Some(format!("validator-verdict://mt006/{run_id}/{lane_id}"));
    message
}

fn cloud_message(
    run_id: &str,
    lane_id: &str,
    message_id: &str,
    authority: ModelLaneAuthority,
) -> NewModelLaneMessage {
    NewModelLaneMessage {
        message_id: format!("{message_id}-{run_id}"),
        run_id: run_id.into(),
        trace_id: format!("trace-{run_id}"),
        message_span_id: format!("span-{message_id}-{run_id}"),
        parent_span_id: Some(format!("span-{lane_id}")),
        linked_span_contexts: vec![format!("trace-{run_id}:cloud")],
        from_lane_id: lane_id.into(),
        to_lane: ModelLaneTarget::Coordinator,
        routing: Some(ModelLaneRoutingMetadata {
            target_role: "coordinator".into(),
            target_session: format!("coordinator-session-{run_id}"),
            correlation_id: format!("corr-{message_id}-{run_id}"),
            ack_for: None,
            requires_ack: false,
        }),
        kind: ModelLaneMessageKind::Critique,
        payload_ref: format!("artifact-store://mt006/{run_id}/{message_id}.json"),
        payload_sha256: sample_sha256(),
        event_ledger_stream_id: event_stream_id(run_id),
        summary: "redacted cloud critique output".into(),
        authority,
        promotion_decision_id: None,
        promotion_gate_ref: None,
        promotion_receipt_ref: None,
        validator_verdict_ref: None,
        operator_decision_ref: None,
        promoted_artifact_ref: None,
        promoted_artifact_sha256: None,
        promoted_artifact_version: None,
        tool_gate_decision_refs: vec!["toolgate://mt006/cloud-read-context".into()],
        coordinator_session_id: format!("coordinator-session-{run_id}"),
        work_packet_id: Some(WP_ID.into()),
        micro_task_id: Some(MT_ID.into()),
        task_board_id: Some(TASK_BOARD_ID.into()),
        owner_session: OWNER.into(),
        locus_binding: Some(sample_locus(run_id, lane_id)),
        idempotency_key: format!("idem-{message_id}-{run_id}"),
        replay_order_key: format!("00000020/{run_id}/{message_id}"),
        replay_after_event_ledger_seq: Some(1),
        proposal_ref: None,
        crdt_update_ref: None,
        crdt_base_snapshot_ref: None,
        crdt_state_vector: None,
        crdt_proposal_ref: None,
        crdt_stale_base_ref: None,
        failstate_code: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane-cloud-projection-consent#advisory".into()),
        created_at_utc: "2026-06-29T09:02:00Z".into(),
        diagnostic_payload: json!({
            "projection_plan_id": projection_plan_id(run_id, lane_id),
            "consent_receipt_id": consent_receipt_id(run_id, lane_id),
            "redaction_policy_ref": "redaction-policy://mt006/cloud-safe",
            "export_posture": "redacted_context_only",
            "retention_policy": "no_training_ephemeral",
            "authority": "advisory_until_promotion",
            "flight_recorder": "EventLedger cloud consent/projection authority",
            "locus": format!("locus://wp1/mt006/{run_id}/{lane_id}"),
            "palmistry": "external watcher link expected when feature is available"
        }),
    }
}

fn cloud_spawn_request(run_id: &str, lane_id: &str) -> SpawnRequest {
    SpawnRequest::new(
        ModelInstanceId::new(ModelId::new_v7(), 600),
        RuntimeAdapterBinding::LlamaCpp,
        OWNER,
        format!("coordinator-session-{run_id}"),
    )
    .with_cloud_provider(ProviderKind::ByokCloud, "gpt-4o-mini")
    .with_byok_cloud_provider(ByokCloudProvider::OpenAi)
    .with_wp(WP_ID)
    .with_mt(MT_ID)
    .with_dexterity_launch(cloud_spawn_contract(run_id, lane_id))
}

fn cloud_spawn_contract(run_id: &str, lane_id: &str) -> DexterityLaunchContract {
    DexterityLaunchContract {
        run_id: run_id.into(),
        lane_id: lane_id.into(),
        trace_id: format!("trace-{run_id}"),
        run_span_id: format!("span-{run_id}"),
        lane_span_id: format!("span-{lane_id}"),
        routing_policy: "cloud_plan_local_execute".into(),
        context_bundle_id: format!("context-bundle://mt006/{run_id}/bootstrap"),
        event_ledger_stream_id: event_stream_id(run_id),
        artifact_namespace: format!("artifact://model-lane/mt006/{run_id}"),
        task_board_id: TASK_BOARD_ID.into(),
        locus_binding_ref: format!("locus://wp1/mt006/{run_id}/{lane_id}"),
        role: "cloud-review-lane".into(),
        backend: "cloud_lane_openai".into(),
        adapter_id: "openai_byok".into(),
        capability_token_ids: vec!["capability://dexterity/cloud-generate".into()],
        effective_capability_snapshot_ref: format!("capability-snapshot://{lane_id}"),
        projection_plan_ref: Some(projection_plan_id(run_id, lane_id)),
        consent_receipt_ref: Some(consent_receipt_id(run_id, lane_id)),
        tool_gate_decision_refs: vec!["toolgate://mt006/cloud-read-context".into()],
        memory_pack_ref: format!("memory-pack://fems/mt006/{run_id}"),
        memory_pack_hash: sample_sha256(),
        determinism_mode: "deterministic_replay".into(),
        budget_summary_ref: format!("budget://mt006/{run_id}"),
        candidate_model_ids: vec!["model://dexterity/byok_cloud/gpt-4o-mini".into()],
        procedural_review_status: "cloud_projection_consent_preflighted".into(),
        truncation_warning_ref: None,
        rejection_reason_refs: vec![],
        run_recovery_hint_ref: Some("usermanual://model-lane-cloud-projection-consent#run".into()),
        lane_recovery_hint_ref: Some(
            "usermanual://model-lane-cloud-projection-consent#lane".into(),
        ),
    }
}

fn sample_locus(run_id: &str, lane_id: &str) -> ModelLaneLocusBinding {
    ModelLaneLocusBinding {
        work_packet_id: WP_ID.into(),
        micro_task_id: MT_ID.into(),
        task_board_id: Some(TASK_BOARD_ID.into()),
        coordinator_session_id: format!("coordinator-session-{run_id}"),
        session_id: format!("session-{lane_id}"),
        model_session_id: model_session_id_for(lane_id),
        owner_session: OWNER.into(),
        locus_binding_ref: format!("locus://wp1/mt006/{run_id}/{lane_id}"),
    }
}

fn projection_plan_id(run_id: &str, lane_id: &str) -> String {
    format!("cloud-projection-plan://{run_id}/{lane_id}")
}

fn consent_receipt_id(run_id: &str, lane_id: &str) -> String {
    format!("cloud-consent-receipt://{run_id}/{lane_id}")
}

fn event_stream_id(run_id: &str) -> String {
    format!("mlane-stream-{run_id}")
}

fn model_session_id_for(lane_id: &str) -> String {
    format!("model-session-{lane_id}")
}

fn requested_model_id(provider_kind: &str) -> String {
    match provider_kind {
        "anthropic" => "model://dexterity/byok_cloud/claude-sonnet-4".into(),
        _ => "model://dexterity/byok_cloud/gpt-4o-mini".into(),
    }
}

fn sample_sha256() -> String {
    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".into()
}

fn sample_other_hash() -> String {
    "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210".into()
}

fn sample_scope_hash() -> String {
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into()
}

fn assert_cx_mm_007(err: &impl std::fmt::Display) {
    assert_cx_mm_007_string(&err.to_string());
}

fn assert_cx_mm_007_string(message: &str) {
    assert!(
        message.contains("CX-MM-007"),
        "expected CX-MM-007 error, got {message}"
    );
}

async fn assert_no_lane_row(pool: &sqlx::PgPool, lane_id: &str) {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM model_lanes WHERE lane_id = $1")
        .bind(lane_id)
        .fetch_one(pool)
        .await
        .expect("count lane rows");
    assert_eq!(count, 0, "denied cloud launch must not create lane row");
}

async fn assert_denial_event(pool: &sqlx::PgPool, run_id: &str, lane_id: &str) {
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
    assert_eq!(
        payload["user_manual_behavior_ref"],
        json!(USERMANUAL_BEHAVIOR)
    );
}

async fn assert_denial_event_count(pool: &sqlx::PgPool, lane_id: &str, expected: i64) {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger \
         WHERE aggregate_type = 'model_lane_cloud_consent_denial' \
           AND aggregate_id = $1",
    )
    .bind(lane_id)
    .fetch_one(pool)
    .await
    .expect("count cloud consent denial EventLedger rows");
    assert_eq!(
        count, expected,
        "stable denial idempotency must keep one row for {lane_id}"
    );
}

struct CountingFactory {
    calls: Arc<AtomicUsize>,
}

#[async_trait]
impl ModelSessionFactory for CountingFactory {
    async fn create(&self, _request: &SpawnRequest) -> Result<LiveSession, SwarmError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Err(SwarmError::FactoryFailed(
            "CountingFactory must not be called before cloud consent authority".into(),
        ))
    }
}

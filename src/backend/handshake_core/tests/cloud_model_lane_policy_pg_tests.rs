//! WP-1 MT-006: Dexterity cloud projection and consent runtime proof.
//!
//! These tests use real PostgreSQL plus the kernel EventLedger. They prove that
//! BYOK cloud lanes cannot launch from synthetic refs, cannot call a provider
//! before durable ProjectionPlan/ConsentReceipt authority exists, and remain
//! advisory until promoted by Dexterity.

mod knowledge_pg_support;

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

use async_trait::async_trait;
use handshake_core::model_runtime::registry::RuntimeBinding as RuntimeAdapterBinding;
use handshake_core::model_runtime::{
    CancellationToken, Embedding, GenerateRequest, KvCacheHandle, LoadSpec, LoraStackHandle,
    ModelCapabilities, ModelId, ModelRuntime, ModelRuntimeError, ProviderKind, Score,
    SteeringHookHandle, TokenStream,
};
use handshake_core::process_ledger::{
    LedgerBatcher, LedgerBatcherConfig, NoopOverflowSink, ProcessEngineKind,
    ProcessOwnershipRecordId, ProcessStart,
};
use handshake_core::swarm_orchestration::model_lane::{
    CloudExportDelegation, DexterityLaunchContract, LaunchAuthority, ModelLaneAuthority,
    ModelLaneCloudConsentReceiptRecord, ModelLaneCloudConsentReceiptStatus,
    ModelLaneCloudConsentScope, ModelLaneCloudConsentTargetBinding, ModelLaneCloudExportPosture,
    ModelLaneCloudProjectionPlanRecord, ModelLaneCloudProjectionPlanStatus,
    ModelLaneCloudRetentionPolicy, ModelLaneKind, ModelLaneLocusBinding, ModelLaneMessageKind,
    ModelLaneProviderKind, ModelLaneRecoveryState, ModelLaneRoutingMetadata, ModelLaneStatus,
    ModelLaneStore, ModelLaneTarget, NewModelLane, NewModelLaneCloudConsentReceipt,
    NewModelLaneCloudProjectionPlan, NewModelLaneMessage, NewModelLaneRun, RuntimeBinding,
};
use handshake_core::swarm_orchestration::resource_scope::AccountBoundAuthority;
use handshake_core::swarm_orchestration::{
    ByokCloudProvider, LiveSession, ModelInstanceId, ModelSessionFactory, RecordingSwarmSink,
    RunBudget, SpawnRequest, SwarmConfig, SwarmCoordinator, SwarmError,
};
use serde_json::{json, Value};
use tokio::sync::Notify;

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
        .test_commit_cloud_consent_revocation(
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
        .test_commit_cloud_consent_revocation(
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
    assert_eq!(terminal_payload["provider_call_cancelled"], json!(false));
    assert_eq!(
        terminal_payload["provider_cancel_outcome"],
        json!("not_live_at_revocation"),
        "store-only revocation must not fabricate runtime-token cancellation"
    );
    assert_eq!(terminal_payload["flight_recorder"], json!("EventLedger"));

    let retry_cancelled = store
        .test_commit_cloud_consent_revocation(
            "cloud-consent-receipt://run-cloud-cancel/lane-cloud-cancel",
            "operator://mt006/runtime-revoke",
            "operator revoked cloud consent while lane was pending",
        )
        .await
        .expect("duplicate revocation replays without EventLedger idempotency conflict");
    assert_eq!(retry_cancelled.len(), 1);
    assert_eq!(retry_cancelled[0].status, ModelLaneStatus::Cancelled);
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

#[tokio::test]
async fn broadcast_replay_rejects_row_only_plan_and_receipt_tamper() {
    let (pool, store) = model_lane_store().await;
    for (suffix, table, id_column, tamper_path) in [
        (
            "plan-row-tamper",
            "model_lane_cloud_projection_plans",
            "projection_plan_id",
            "scope_hash",
        ),
        (
            "receipt-row-tamper",
            "model_lane_cloud_consent_receipts",
            "consent_receipt_id",
            "approved_by_ref",
        ),
    ] {
        let run_id = format!("run-cloud-broadcast-{suffix}");
        let lane_a = format!("lane-{suffix}-a");
        let lane_b = format!("lane-{suffix}-b");
        let (plan, receipt) = seed_broadcast_authority(
            &store,
            &run_id,
            vec![
                broadcast_target(
                    &lane_a,
                    "openai",
                    "model://dexterity/byok_cloud/gpt-4o-mini",
                ),
                broadcast_target(
                    &lane_b,
                    "anthropic",
                    "model://dexterity/byok_cloud/claude-sonnet-4",
                ),
            ],
        )
        .await;
        let authority_id = if table.ends_with("projection_plans") {
            &plan.projection_plan_id
        } else {
            &receipt.consent_receipt_id
        };
        let sql = format!(
            "UPDATE {table} SET record_json = jsonb_set(record_json, '{{{tamper_path}}}', to_jsonb('row-only-tamper'::text)) WHERE {id_column} = $1"
        );
        sqlx::query(&sql)
            .bind(authority_id)
            .execute(&pool)
            .await
            .expect("tamper mutable authority row only");
        let error = store
            .replay_cloud_consent_authority(&run_id)
            .await
            .expect_err("replay must compare mutable row with EventLedger in one snapshot");
        assert_cx_mm_007(&error);
    }
}

#[tokio::test]
async fn run_scoped_coordinator_reaches_factory_for_heterogeneous_targets() {
    let (pool, store) = model_lane_store().await;
    let run_id = "run-cloud-broadcast-coordinator-heterogeneous";
    let mut openai = coordinator_cloud_request(
        run_id,
        "lane-cloud-coordinator-openai",
        ByokCloudProvider::OpenAi,
    );
    let mut anthropic = coordinator_cloud_request(
        run_id,
        "lane-cloud-coordinator-anthropic",
        ByokCloudProvider::Anthropic,
    );
    let (plan, receipt) = seed_broadcast_authority(
        &store,
        run_id,
        vec![
            target_from_spawn_request(&openai),
            target_from_spawn_request(&anthropic),
        ],
    )
    .await;
    bind_spawn_request_authority(&mut openai, &plan, &receipt);
    bind_spawn_request_authority(&mut anthropic, &plan, &receipt);

    let calls = Arc::new(Mutex::new(Vec::new()));
    let cancellation_tokens = Arc::new(Mutex::new(Vec::new()));
    let unloads = Arc::new(AtomicUsize::new(0));
    let (ledger, _drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("manual process ledger");
    let factory_ledger = ledger.clone();
    let coordinator = SwarmCoordinator::new_with_model_lane_store(
        SwarmConfig::new(RunBudget::defaulted(2)),
        Arc::new(RecordingBoundaryFactory {
            calls: calls.clone(),
            cancellation_tokens: cancellation_tokens.clone(),
            unloads: unloads.clone(),
            ledger: factory_ledger,
            entered: None,
            release: None,
        }),
        Arc::new(RecordingSwarmSink::new()),
        ledger,
        store,
    );
    let instances = coordinator
        .spawn_cloud_consent_batch(vec![openai, anthropic])
        .await
        .expect("authorized heterogeneous batch crosses coordinator/factory boundary");
    let calls = calls.lock().expect("recording boundary factory calls");
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].0, "openai");
    assert_eq!(calls[1].0, "anthropic");
    assert!(calls
        .iter()
        .all(|call| call.1.starts_with("swarm-session:")));
    assert_eq!(calls[0].2, "model://dexterity/byok_cloud/gpt-4o-mini");
    assert_eq!(calls[1].2, "model://dexterity/byok_cloud/claude-sonnet-4");
    assert_eq!(
        calls[0].3,
        "capability-snapshot://lane-cloud-coordinator-openai"
    );
    assert_eq!(
        calls[1].3,
        "capability-snapshot://lane-cloud-coordinator-anthropic"
    );
    assert_eq!(calls[0].4, "openai_byok");
    assert_eq!(calls[1].4, "anthropic_byok");
    let launched_lanes: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM model_lanes WHERE run_id = $1 AND status = 'ready'",
    )
    .bind(run_id)
    .fetch_one(&pool)
    .await
    .expect("count coordinator-launched heterogeneous lanes");
    assert_eq!(launched_lanes, 2);
    sqlx::query("DELETE FROM model_lanes WHERE lane_id = $1")
        .bind("lane-cloud-coordinator-openai")
        .execute(&pool)
        .await
        .expect(
            "delete one mutable lane projection while retaining canonical EventLedger authority",
        );
    drop(calls);
    let revoked = coordinator
        .revoke_cloud_consent_receipt(
            &receipt.consent_receipt_id,
            "operator://mt017/coordinator-revoke",
            "coordinator-owned live broadcast revoke",
        )
        .await
        .expect("coordinator cancels live targets and commits revocation authority");
    assert_eq!(revoked.len(), 2);
    for instance in &instances {
        assert!(coordinator.session_runtime(*instance).is_none());
    }
    assert!(cancellation_tokens
        .lock()
        .expect("boundary cancellation tokens")
        .iter()
        .all(CancellationToken::is_cancelled));
    assert_eq!(unloads.load(Ordering::SeqCst), 2);
    let rebuilt_cancelled_lanes: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM model_lanes WHERE run_id = $1 AND status = 'cancelled'",
    )
    .bind(run_id)
    .fetch_one(&pool)
    .await
    .expect("count rebuilt terminal lane projections");
    assert_eq!(
        rebuilt_cancelled_lanes, 2,
        "revocation must cancel from canonical EventLedger authority and rebuild a missing projection"
    );
    let terminal_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger WHERE aggregate_type = 'model_lane_terminal' AND payload->>'consent_receipt_id' = $1",
    )
    .bind(&receipt.consent_receipt_id)
    .fetch_one(&pool)
    .await
    .expect("count coordinator-owned receipt terminal rows");
    assert_eq!(terminal_count, 2);
}

#[tokio::test]
async fn run_scoped_coordinator_cross_run_or_receipt_mismatches_stop_before_factory() {
    let (pool, store) = model_lane_store().await;
    for mismatch in ["run", "receipt"] {
        let run_id = format!("run-cloud-broadcast-mismatch-{mismatch}");
        let lane_id = format!("lane-cloud-broadcast-mismatch-{mismatch}");
        let sibling_id = format!("lane-cloud-broadcast-mismatch-{mismatch}-sibling");
        let mut request = coordinator_cloud_request(&run_id, &lane_id, ByokCloudProvider::OpenAi);
        let mut sibling_request =
            coordinator_cloud_request(&run_id, &sibling_id, ByokCloudProvider::Anthropic);
        let target = target_from_spawn_request(&request);
        let sibling = target_from_spawn_request(&sibling_request);
        let (plan, receipt) =
            seed_broadcast_authority(&store, &run_id, vec![target, sibling]).await;
        bind_spawn_request_authority(&mut request, &plan, &receipt);
        bind_spawn_request_authority(&mut sibling_request, &plan, &receipt);
        match mismatch {
            "run" => sibling_request
                .dexterity_launch
                .as_mut()
                .expect("sibling contract")
                .run_id
                .push_str("-wrong"),
            "receipt" => {
                sibling_request
                    .dexterity_launch
                    .as_mut()
                    .expect("sibling contract")
                    .consent_receipt_ref = Some("cloud-consent-receipt://wrong".into())
            }
            _ => unreachable!(),
        }
        let calls = Arc::new(AtomicUsize::new(0));
        let (ledger, _drain) = LedgerBatcher::manual_for_tests(
            LedgerBatcherConfig::default(),
            Arc::new(NoopOverflowSink),
        )
        .expect("manual process ledger");
        let coordinator = SwarmCoordinator::new_with_model_lane_store(
            SwarmConfig::new(RunBudget::defaulted(2)),
            Arc::new(CountingFactory {
                calls: calls.clone(),
            }),
            Arc::new(RecordingSwarmSink::new()),
            ledger,
            store.clone(),
        );
        let error = coordinator
            .spawn_cloud_consent_batch(vec![sibling_request, request])
            .await
            .expect_err("one bad target must deny the complete batch before dispatch");
        assert_cx_mm_007_string(&error.to_string());
        assert_eq!(calls.load(Ordering::SeqCst), 0, "{mismatch}");
        assert_no_lane_row(&pool, &lane_id).await;
        assert_no_lane_row(&pool, &sibling_id).await;
    }
}

#[tokio::test]
async fn broadcast_revoke_fences_factory_inflight_before_durable_lane_insert() {
    let (pool, store) = model_lane_store().await;
    let run_id = "run-cloud-broadcast-revoke-interleaving";
    let mut request = coordinator_cloud_request(
        run_id,
        "lane-cloud-revoke-interleaving-a",
        ByokCloudProvider::OpenAi,
    );
    let sibling = coordinator_cloud_request(
        run_id,
        "lane-cloud-revoke-interleaving-b",
        ByokCloudProvider::Anthropic,
    );
    let (plan, receipt) = seed_broadcast_authority(
        &store,
        run_id,
        vec![
            target_from_spawn_request(&request),
            target_from_spawn_request(&sibling),
        ],
    )
    .await;
    bind_spawn_request_authority(&mut request, &plan, &receipt);
    let calls = Arc::new(Mutex::new(Vec::new()));
    let cancellations = Arc::new(Mutex::new(Vec::new()));
    let unloads = Arc::new(AtomicUsize::new(0));
    let entered = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let (ledger, _drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("manual process ledger");
    let coordinator = Arc::new(SwarmCoordinator::new_with_model_lane_store(
        SwarmConfig::new(RunBudget::defaulted(2)),
        Arc::new(RecordingBoundaryFactory {
            calls: calls.clone(),
            cancellation_tokens: cancellations.clone(),
            unloads: unloads.clone(),
            ledger: ledger.clone(),
            entered: Some(entered.clone()),
            release: Some(release.clone()),
        }),
        Arc::new(RecordingSwarmSink::new()),
        ledger,
        store,
    ));
    let spawn_coordinator = coordinator.clone();
    let spawn = tokio::spawn(async move { spawn_coordinator.spawn_session(request).await });
    entered.notified().await;
    let revoked = coordinator
        .revoke_cloud_consent_receipt(
            &receipt.consent_receipt_id,
            "operator://mt017/interleaving-revoke",
            "revoke while target factory creation is in flight",
        )
        .await
        .expect("revoke commits while no target lane row exists");
    assert!(revoked.is_empty());
    release.notify_one();
    let spawn_error = spawn
        .await
        .expect("join in-flight spawn")
        .expect_err("post-revoke durable insertion must fail closed");
    assert_cx_mm_007_string(&spawn_error.to_string());
    assert_eq!(calls.lock().expect("factory calls").len(), 1);
    assert_eq!(unloads.load(Ordering::SeqCst), 1);
    assert!(cancellations
        .lock()
        .expect("factory cancellation tokens")
        .iter()
        .all(CancellationToken::is_cancelled));
    assert_no_lane_row(&pool, "lane-cloud-revoke-interleaving-a").await;
    assert_no_lane_row(&pool, "lane-cloud-revoke-interleaving-b").await;
}

#[tokio::test]
async fn single_run_revocation_retry_cancels_after_fence_only_crash_and_missing_projection() {
    let (pool, store) = model_lane_store().await;
    let run_id = "run-cloud-revoke-fence-retry";
    let lane_id = "lane-cloud-revoke-fence-retry";
    let mut request = coordinator_cloud_request(run_id, lane_id, ByokCloudProvider::OpenAi);
    let sibling = coordinator_cloud_request(
        run_id,
        "lane-cloud-revoke-fence-retry-sibling",
        ByokCloudProvider::Anthropic,
    );
    let (plan, receipt) = seed_broadcast_authority(
        &store,
        run_id,
        vec![
            target_from_spawn_request(&request),
            target_from_spawn_request(&sibling),
        ],
    )
    .await;
    bind_spawn_request_authority(&mut request, &plan, &receipt);

    let calls = Arc::new(Mutex::new(Vec::new()));
    let cancellations = Arc::new(Mutex::new(Vec::new()));
    let unloads = Arc::new(AtomicUsize::new(0));
    let (ledger, _drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("manual process ledger");
    let coordinator = SwarmCoordinator::new_with_model_lane_store(
        SwarmConfig::new(RunBudget::defaulted(2)),
        Arc::new(RecordingBoundaryFactory {
            calls: calls.clone(),
            cancellation_tokens: cancellations.clone(),
            unloads: unloads.clone(),
            ledger: ledger.clone(),
            entered: None,
            release: None,
        }),
        Arc::new(RecordingSwarmSink::new()),
        ledger,
        store.clone(),
    );
    coordinator
        .spawn_session(request)
        .await
        .expect("launch coordinator-owned cloud lane");
    assert_eq!(coordinator.live_session_count(), 1);

    sqlx::query("DELETE FROM model_lanes WHERE lane_id = $1")
        .bind(lane_id)
        .execute(&pool)
        .await
        .expect("remove only the mutable lane projection");
    let actor = "operator://mt017/fence-retry";
    let reason = "retry after crash immediately after durable revocation fence";
    let fenced = store
        .test_fence_cloud_consent_revocation(&receipt.consent_receipt_id, actor, reason)
        .await
        .expect("durable fence reconstructs canonical covered lane");
    assert_eq!(fenced.len(), 1);
    assert_eq!(fenced[0].lane_id, lane_id);
    assert_eq!(fenced[0].status, ModelLaneStatus::Ready);
    assert_eq!(
        coordinator.live_session_count(),
        1,
        "fence-only crash point must leave runtime available for retry cleanup"
    );

    let finalized = coordinator
        .revoke_cloud_consent_receipt(&receipt.consent_receipt_id, actor, reason)
        .await
        .expect("identical retry cancels runtime and finalizes terminal authority");
    assert_eq!(finalized.len(), 1);
    assert_eq!(finalized[0].status, ModelLaneStatus::Cancelled);
    assert_eq!(coordinator.live_session_count(), 0);
    assert_eq!(unloads.load(Ordering::SeqCst), 1);
    assert!(cancellations
        .lock()
        .expect("factory cancellation tokens")
        .iter()
        .all(CancellationToken::is_cancelled));
    let terminal_payload: Value = sqlx::query_scalar(
        "SELECT payload FROM kernel_event_ledger WHERE aggregate_type = 'model_lane_terminal' AND aggregate_id = $1 ORDER BY event_sequence DESC LIMIT 1",
    )
    .bind(lane_id)
    .fetch_one(&pool)
    .await
    .expect("load finalized terminal evidence");
    assert_eq!(terminal_payload["provider_call_cancelled"], json!(true));
    assert_eq!(
        terminal_payload["provider_cancel_outcome"],
        json!("cancelled_by_coordinator")
    );
    let rebuilt_status: String =
        sqlx::query_scalar("SELECT status FROM model_lanes WHERE lane_id = $1")
            .bind(lane_id)
            .fetch_one(&pool)
            .await
            .expect("terminal finalizer rebuilds the mutable projection");
    assert_eq!(rebuilt_status, "cancelled");
}

#[tokio::test]
async fn single_run_revocation_retry_recovers_provider_cancel_after_finalizer_failure() {
    let (pool, store) = model_lane_store().await;
    let run_id = "run-cloud-revoke-finalizer-retry";
    let lane_id = "lane-cloud-revoke-finalizer-retry";
    let mut request = coordinator_cloud_request(run_id, lane_id, ByokCloudProvider::OpenAi);
    let sibling = coordinator_cloud_request(
        run_id,
        "lane-cloud-revoke-finalizer-retry-sibling",
        ByokCloudProvider::Anthropic,
    );
    let (plan, receipt) = seed_broadcast_authority(
        &store,
        run_id,
        vec![
            target_from_spawn_request(&request),
            target_from_spawn_request(&sibling),
        ],
    )
    .await;
    bind_spawn_request_authority(&mut request, &plan, &receipt);

    let calls = Arc::new(Mutex::new(Vec::new()));
    let cancellations = Arc::new(Mutex::new(Vec::new()));
    let unloads = Arc::new(AtomicUsize::new(0));
    let (ledger, _drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("manual process ledger");
    let coordinator = SwarmCoordinator::new_with_model_lane_store(
        SwarmConfig::new(RunBudget::defaulted(2)),
        Arc::new(RecordingBoundaryFactory {
            calls,
            cancellation_tokens: cancellations.clone(),
            unloads: unloads.clone(),
            ledger: ledger.clone(),
            entered: None,
            release: None,
        }),
        Arc::new(RecordingSwarmSink::new()),
        ledger,
        store,
    );
    let instance = coordinator
        .spawn_session(request)
        .await
        .expect("launch coordinator-owned cloud lane");

    sqlx::query(
        r#"
        CREATE FUNCTION mt017_fail_live_terminal_finalize() RETURNS trigger AS $$
        BEGIN
            IF NEW.aggregate_type = 'model_lane_terminal'
               AND NEW.aggregate_id = 'lane-cloud-revoke-finalizer-retry' THEN
                RAISE EXCEPTION 'forced live terminal finalizer failure';
            END IF;
            RETURN NEW;
        END;
        $$ LANGUAGE plpgsql;
        "#,
    )
    .execute(&pool)
    .await
    .expect("install deterministic live terminal failure function");
    sqlx::query(
        "CREATE TRIGGER mt017_fail_live_terminal_finalize_trigger BEFORE INSERT ON kernel_event_ledger FOR EACH ROW EXECUTE FUNCTION mt017_fail_live_terminal_finalize()",
    )
    .execute(&pool)
    .await
    .expect("install deterministic live terminal failure trigger");

    let actor = "operator://mt017/finalizer-retry";
    let reason = "retry after provider cancellation but before terminal finalization";
    let first_error = coordinator
        .revoke_cloud_consent_receipt(&receipt.consent_receipt_id, actor, reason)
        .await
        .expect_err("terminal finalizer failure must surface after runtime cleanup");
    assert!(first_error
        .to_string()
        .contains("forced live terminal finalizer failure"));
    assert_eq!(coordinator.live_session_count(), 0);
    assert_eq!(unloads.load(Ordering::SeqCst), 1);
    assert!(cancellations
        .lock()
        .expect("factory cancellation tokens")
        .iter()
        .all(CancellationToken::is_cancelled));
    let cleanup_completed: bool = sqlx::query_scalar(
        "SELECT status = 'completed' AND terminal_state = 'Cancelled' AND reason = $2 FROM swarm_session_cleanup_receipts WHERE instance_id = $1",
    )
    .bind(instance.to_string())
    .bind(format!("CX-MM-007 cloud consent revoked: {reason}"))
    .fetch_one(&pool)
    .await
    .expect("runtime cleanup receipt survives terminal finalizer failure");
    assert!(cleanup_completed);
    let pre_retry_terminal_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger WHERE aggregate_type = 'model_lane_terminal' AND aggregate_id = $1",
    )
    .bind(lane_id)
    .fetch_one(&pool)
    .await
    .expect("count terminal evidence before retry");
    assert_eq!(pre_retry_terminal_count, 0);

    sqlx::query("DROP TRIGGER mt017_fail_live_terminal_finalize_trigger ON kernel_event_ledger")
        .execute(&pool)
        .await
        .expect("remove deterministic live terminal failure trigger");
    sqlx::query("DROP FUNCTION mt017_fail_live_terminal_finalize()")
        .execute(&pool)
        .await
        .expect("remove deterministic live terminal failure function");

    let finalized = coordinator
        .revoke_cloud_consent_receipt(&receipt.consent_receipt_id, actor, reason)
        .await
        .expect("identical retry recovers cancellation from durable cleanup receipt");
    assert_eq!(finalized.len(), 1);
    assert_eq!(finalized[0].status, ModelLaneStatus::Cancelled);
    assert_eq!(
        unloads.load(Ordering::SeqCst),
        1,
        "retry must not unload twice"
    );
    let terminal_payload: Value = sqlx::query_scalar(
        "SELECT payload FROM kernel_event_ledger WHERE aggregate_type = 'model_lane_terminal' AND aggregate_id = $1 ORDER BY event_sequence DESC LIMIT 1",
    )
    .bind(lane_id)
    .fetch_one(&pool)
    .await
    .expect("load retried terminal evidence");
    assert_eq!(terminal_payload["provider_call_cancelled"], json!(true));
    assert_eq!(
        terminal_payload["provider_cancel_outcome"],
        json!("cancelled_by_coordinator")
    );
    let terminal_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger WHERE aggregate_type = 'model_lane_terminal' AND aggregate_id = $1",
    )
    .bind(lane_id)
    .fetch_one(&pool)
    .await
    .expect("count retried terminal evidence");
    assert_eq!(terminal_count, 1);
}

#[tokio::test]
async fn single_run_revocation_fence_includes_lane_committed_before_fence() {
    let (_pool, store) = model_lane_store().await;
    let run_id = "run-cloud-launch-wins-revoke-fence";
    let lane_id = "lane-cloud-launch-wins-revoke-fence";
    let target_request = coordinator_cloud_request(run_id, lane_id, ByokCloudProvider::OpenAi);
    let sibling_request = coordinator_cloud_request(
        run_id,
        "lane-cloud-launch-wins-revoke-fence-sibling",
        ByokCloudProvider::Anthropic,
    );
    let (plan, receipt) = seed_broadcast_authority(
        &store,
        run_id,
        vec![
            target_from_spawn_request(&target_request),
            target_from_spawn_request(&sibling_request),
        ],
    )
    .await;
    let (mut run, mut lane) = sample_cloud_run_lane(run_id, lane_id, ModelLaneStatus::Running);
    run.lane_ids = vec![lane_id.into()];
    bind_broadcast_authority(&mut run, &mut lane, &plan, &receipt);

    let entered = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let launch_store = store.clone();
    let launch_entered = entered.clone();
    let launch_release = release.clone();
    let launch = tokio::spawn(async move {
        launch_store
            .test_record_prepared_launch_holding_receipt_fence(
                (run, lane),
                launch_entered,
                launch_release,
            )
            .await
    });
    entered.notified().await;

    let revoke_store = store.clone();
    let receipt_id = receipt.consent_receipt_id.clone();
    let actor = "operator://mt017/launch-wins-fence".to_string();
    let reason = "launch commits immediately before revocation fence".to_string();
    let mut revoke = tokio::spawn(async move {
        revoke_store
            .test_fence_cloud_consent_revocation(&receipt_id, &actor, &reason)
            .await
    });
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(100), &mut revoke)
            .await
            .is_err(),
        "revocation must queue behind launch-owned receipt fence"
    );
    release.notify_one();
    launch
        .await
        .expect("join launch holding receipt fence")
        .expect("launch commits before queued revocation");
    let fenced = revoke
        .await
        .expect("join queued revocation")
        .expect("queued revocation fences after launch commit");
    assert_eq!(fenced.len(), 1);
    assert_eq!(fenced[0].lane_id, lane_id);

    let finalized = store
        .test_finalize_cloud_consent_revocation(
            &receipt.consent_receipt_id,
            "operator://mt017/launch-wins-fence",
            "launch commits immediately before revocation fence",
            &std::collections::BTreeSet::new(),
        )
        .await
        .expect("finalize launch-wins lane after durable fence");
    assert_eq!(finalized.len(), 1);
    assert_eq!(finalized[0].status, ModelLaneStatus::Cancelled);
}

#[tokio::test]
async fn run_scoped_consent_covers_current_and_future_sibling_lanes() {
    let (pool, store) = model_lane_store().await;
    let run_id = "run-cloud-broadcast-homogeneous";
    let lane_a = "lane-cloud-broadcast-a";
    let lane_b = "lane-cloud-broadcast-b";
    let sibling = "lane-cloud-broadcast-sibling";
    let future = "lane-cloud-broadcast-future";
    let targets = vec![
        broadcast_target(lane_b, "openai", "model://dexterity/byok_cloud/gpt-4o-mini"),
        broadcast_target(lane_a, "openai", "model://dexterity/byok_cloud/gpt-4o-mini"),
    ];
    let (plan, receipt) = seed_broadcast_authority(&store, run_id, targets).await;
    assert!(plan.target_bindings.is_empty());
    assert!(receipt.target_bindings.is_empty());
    assert_eq!(plan.target_bindings_hash, receipt.target_bindings_hash);
    assert!(plan.target_bindings_hash.is_none());
    let registry = store
        .schema_registry_rows()
        .await
        .expect("load broadcast consent schema registry");
    assert!(registry
        .iter()
        .any(|row| row.schema_id == "hsk.model_lane_cloud_projection_plan@2"));
    assert!(registry
        .iter()
        .any(|row| row.schema_id == "hsk.model_lane_cloud_consent_receipt@2"));

    let (mut run, mut first_lane) = sample_cloud_run_lane(run_id, lane_a, ModelLaneStatus::Running);
    run.lane_ids = vec![lane_a.into(), lane_b.into(), sibling.into(), future.into()];
    bind_broadcast_authority(&mut run, &mut first_lane, &plan, &receipt);
    store
        .record_prepared_launch((run, first_lane))
        .await
        .expect("first run-scoped homogeneous lane launches");

    let (_, mut second_lane) = sample_cloud_run_lane(run_id, lane_b, ModelLaneStatus::Running);
    bind_broadcast_lane(&mut second_lane, &plan, &receipt);
    store
        .record_lane(second_lane)
        .await
        .expect("second run-scoped homogeneous lane launches");

    for covered_lane_id in [sibling, future] {
        let (_, mut covered_lane) =
            sample_cloud_run_lane(run_id, covered_lane_id, ModelLaneStatus::Running);
        bind_broadcast_lane(&mut covered_lane, &plan, &receipt);
        store
            .record_lane(covered_lane)
            .await
            .expect("run-scoped authority covers current and future sibling lanes");
    }
    let covered_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM model_lanes WHERE run_id = $1 AND record_json->>'consent_receipt_ref' = $2",
    )
    .bind(run_id)
    .bind(&receipt.consent_receipt_id)
    .fetch_one(&pool)
    .await
    .expect("count all run-scoped covered lanes");
    assert_eq!(covered_count, 4);

    let (mut wrong_run, mut wrong_lane) = sample_cloud_run_lane(
        "run-cloud-broadcast-other",
        "lane-cloud-broadcast-other",
        ModelLaneStatus::Running,
    );
    bind_broadcast_authority(&mut wrong_run, &mut wrong_lane, &plan, &receipt);
    let error = store
        .record_prepared_launch((wrong_run, wrong_lane))
        .await
        .expect_err("run-scoped authority must not cross run_id");
    assert_cx_mm_007(&error);

    seed_cloud_authority(
        &store,
        "run-cloud-legacy-compatible",
        "lane-cloud-legacy-compatible",
        "openai",
        ModelLaneCloudConsentReceiptStatus::Approved,
        "2026-01-01T00:00:00Z",
        "2027-01-01T00:00:00Z",
    )
    .await;
    store
        .record_prepared_launch(sample_cloud_run_lane(
            "run-cloud-legacy-compatible",
            "lane-cloud-legacy-compatible",
            ModelLaneStatus::Running,
        ))
        .await
        .expect("legacy single_lane authority remains launch compatible");
}

#[tokio::test]
async fn broadcast_scoped_heterogeneous_targets_revoke_atomically_and_detect_tamper() {
    let (pool, store) = model_lane_store().await;
    let run_id = "run-cloud-broadcast-heterogeneous";
    let lane_openai = "lane-cloud-broadcast-openai";
    let lane_anthropic = "lane-cloud-broadcast-anthropic";
    let targets = vec![
        broadcast_target(
            lane_openai,
            "openai",
            "model://dexterity/byok_cloud/gpt-4o-mini",
        ),
        broadcast_target(
            lane_anthropic,
            "anthropic",
            "model://dexterity/byok_cloud/claude-sonnet-4",
        ),
    ];
    let (plan, receipt) = seed_broadcast_authority(&store, run_id, targets).await;
    let (mut run, mut first_lane) =
        sample_cloud_run_lane(run_id, lane_openai, ModelLaneStatus::Running);
    run.lane_ids = vec![lane_openai.into(), lane_anthropic.into()];
    bind_broadcast_authority(&mut run, &mut first_lane, &plan, &receipt);
    store
        .record_prepared_launch((run, first_lane))
        .await
        .expect("heterogeneous openai target launches");
    let (_, mut second_lane) =
        sample_cloud_run_lane(run_id, lane_anthropic, ModelLaneStatus::Running);
    bind_broadcast_lane(&mut second_lane, &plan, &receipt);
    store
        .record_lane(second_lane)
        .await
        .expect("heterogeneous anthropic target launches");

    let predecessors: Vec<(String, String, i64)> = sqlx::query_as(
        "SELECT lane_id, event_ledger_event_id, event_ledger_seq FROM model_lanes WHERE run_id = $1 ORDER BY lane_id",
    )
    .bind(run_id)
    .fetch_all(&pool)
    .await
    .expect("capture exact predecessor authority before revoke");

    let cancelled = store
        .test_commit_cloud_consent_revocation(
            &receipt.consent_receipt_id,
            "operator://mt017/revoke-broadcast",
            "revoke all durable run-scoped lanes",
        )
        .await
        .expect("broadcast revocation commits all target cancellation state");
    assert_eq!(cancelled.len(), 2);
    assert!(cancelled
        .iter()
        .all(|lane| lane.status == ModelLaneStatus::Cancelled));
    let terminal_rows: Vec<(String, Value)> = sqlx::query_as(
        r#"
        SELECT aggregate_id, payload
        FROM kernel_event_ledger
        WHERE aggregate_type = 'model_lane_terminal'
          AND payload->>'consent_receipt_id' = $1
        ORDER BY aggregate_id
        "#,
    )
    .bind(&receipt.consent_receipt_id)
    .fetch_all(&pool)
    .await
    .expect("load exact terminal evidence for covered targets");
    assert_eq!(terminal_rows.len(), 2, "one terminal row per covered lane");
    for (lane_id, payload) in &terminal_rows {
        assert!(
            lane_id == lane_openai || lane_id == lane_anthropic,
            "no decoy lane"
        );
        assert_eq!(payload["reason_code"], json!("CX-MM-007"));
        assert_eq!(
            payload["consent_receipt_id"],
            json!(&receipt.consent_receipt_id)
        );
        assert_eq!(payload["status"], json!("cancelled"));
        assert_eq!(payload["record"]["status"], json!("cancelled"));
        let predecessor = predecessors
            .iter()
            .find(|(predecessor_lane_id, _, _)| predecessor_lane_id == lane_id)
            .expect("terminal lane has captured predecessor");
        assert_eq!(
            payload["previous_event_ledger_event_id"],
            json!(&predecessor.1)
        );
        assert_eq!(payload["previous_event_ledger_seq"], json!(predecessor.2));
    }
    let identical_replay = store
        .test_commit_cloud_consent_revocation(
            &receipt.consent_receipt_id,
            "operator://mt017/revoke-broadcast",
            "revoke all durable run-scoped lanes",
        )
        .await
        .expect("identical revoke input replays");
    assert_eq!(identical_replay.len(), 2);
    assert!(identical_replay.iter().all(|lane| {
        lane.status == ModelLaneStatus::Cancelled
            && (lane.lane_id == lane_openai || lane.lane_id == lane_anthropic)
    }));
    let conflict = store
        .test_commit_cloud_consent_revocation(
            &receipt.consent_receipt_id,
            "operator://mt017/different-actor",
            "revoke all durable run-scoped lanes",
        )
        .await
        .expect_err("different revoke actor must conflict");
    assert!(matches!(
        conflict,
        handshake_core::swarm_orchestration::model_lane::ModelLaneError::IdempotencyConflict(_)
    ));

    let rollback_run = "run-cloud-broadcast-revoke-rollback";
    let rollback_a = "lane-cloud-revoke-rollback-a";
    let rollback_b = "lane-cloud-revoke-rollback-b";
    let (rollback_plan, rollback_receipt) = seed_broadcast_authority(
        &store,
        rollback_run,
        vec![
            broadcast_target(
                rollback_a,
                "openai",
                "model://dexterity/byok_cloud/gpt-4o-mini",
            ),
            broadcast_target(
                rollback_b,
                "anthropic",
                "model://dexterity/byok_cloud/claude-sonnet-4",
            ),
        ],
    )
    .await;
    let (mut rollback_run_record, mut rollback_first) =
        sample_cloud_run_lane(rollback_run, rollback_a, ModelLaneStatus::Running);
    rollback_run_record.lane_ids = vec![rollback_a.into(), rollback_b.into()];
    bind_broadcast_authority(
        &mut rollback_run_record,
        &mut rollback_first,
        &rollback_plan,
        &rollback_receipt,
    );
    store
        .record_prepared_launch((rollback_run_record, rollback_first))
        .await
        .expect("launch first rollback target");
    let (_, mut rollback_second) =
        sample_cloud_run_lane(rollback_run, rollback_b, ModelLaneStatus::Running);
    bind_broadcast_lane(&mut rollback_second, &rollback_plan, &rollback_receipt);
    store
        .record_lane(rollback_second)
        .await
        .expect("launch second rollback target");
    sqlx::query(
        r#"
        CREATE FUNCTION mt017_fail_second_terminal() RETURNS trigger AS $$
        BEGIN
            IF NEW.aggregate_type = 'model_lane_terminal'
               AND NEW.aggregate_id = 'lane-cloud-revoke-rollback-b' THEN
                RAISE EXCEPTION 'forced second target terminal failure';
            END IF;
            RETURN NEW;
        END;
        $$ LANGUAGE plpgsql;
        "#,
    )
    .execute(&pool)
    .await
    .expect("install deterministic second-target failure function");
    sqlx::query(
        "CREATE TRIGGER mt017_fail_second_terminal_trigger BEFORE INSERT ON kernel_event_ledger FOR EACH ROW EXECUTE FUNCTION mt017_fail_second_terminal()",
    )
    .execute(&pool)
    .await
    .expect("install deterministic second-target terminal trigger");
    let rollback_error = store
        .test_commit_cloud_consent_revocation(
            &rollback_receipt.consent_receipt_id,
            "operator://mt017/rollback",
            "must roll back all covered targets",
        )
        .await
        .expect_err("second target append failure must roll back terminal finalization");
    assert!(rollback_error
        .to_string()
        .contains("forced second target terminal failure"));
    sqlx::query("DROP TRIGGER mt017_fail_second_terminal_trigger ON kernel_event_ledger")
        .execute(&pool)
        .await
        .expect("remove deterministic terminal failure trigger");
    sqlx::query("DROP FUNCTION mt017_fail_second_terminal()")
        .execute(&pool)
        .await
        .expect("remove deterministic terminal failure function");
    let receipt_status: String = sqlx::query_scalar(
        "SELECT status FROM model_lane_cloud_consent_receipts WHERE consent_receipt_id = $1",
    )
    .bind(&rollback_receipt.consent_receipt_id)
    .fetch_one(&pool)
    .await
    .expect("load rolled-back receipt status");
    assert_eq!(
        receipt_status, "revoked",
        "the durable fence must survive terminal-finalizer rollback"
    );
    let active_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM model_lanes WHERE run_id = $1 AND status = 'running'",
    )
    .bind(rollback_run)
    .fetch_one(&pool)
    .await
    .expect("load rolled-back lanes");
    assert_eq!(active_count, 2);
    let rollback_terminal_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger WHERE aggregate_type = 'model_lane_terminal' AND payload->>'consent_receipt_id' = $1",
    )
    .bind(&rollback_receipt.consent_receipt_id)
    .fetch_one(&pool)
    .await
    .expect("count rolled-back terminal evidence");
    assert_eq!(rollback_terminal_count, 0);
    let retried = store
        .test_commit_cloud_consent_revocation(
            &rollback_receipt.consent_receipt_id,
            "operator://mt017/rollback",
            "must roll back all covered targets",
        )
        .await
        .expect("identical retry finalizes every covered lane");
    assert_eq!(retried.len(), 2);

    let missing_row_run = "run-cloud-broadcast-missing-target-row";
    let missing_row_a = "lane-cloud-missing-target-row-a";
    let missing_row_b = "lane-cloud-missing-target-row-b";
    let (missing_plan, missing_receipt) = seed_broadcast_authority(
        &store,
        missing_row_run,
        vec![
            broadcast_target(
                missing_row_a,
                "openai",
                "model://dexterity/byok_cloud/gpt-4o-mini",
            ),
            broadcast_target(
                missing_row_b,
                "anthropic",
                "model://dexterity/byok_cloud/claude-sonnet-4",
            ),
        ],
    )
    .await;
    let (mut missing_run, mut missing_lane) =
        sample_cloud_run_lane(missing_row_run, missing_row_a, ModelLaneStatus::Running);
    missing_run.lane_ids = vec![missing_row_a.into(), missing_row_b.into()];
    bind_broadcast_authority(
        &mut missing_run,
        &mut missing_lane,
        &missing_plan,
        &missing_receipt,
    );
    store
        .record_prepared_launch((missing_run, missing_lane))
        .await
        .expect("create canonical launch authority before deleting mutable row");
    sqlx::query("DELETE FROM model_lanes WHERE lane_id = $1")
        .bind(missing_row_a)
        .execute(&pool)
        .await
        .expect("remove mutable target row only");
    let reconstructed = store
        .test_commit_cloud_consent_revocation(
            &missing_receipt.consent_receipt_id,
            "operator://mt017/missing-row",
            "must consult canonical EventLedger before treating target as never launched",
        )
        .await
        .expect("canonical EventLedger authority reconstructs a missing mutable row");
    assert_eq!(reconstructed.len(), 1);
    assert_eq!(reconstructed[0].lane_id, missing_row_a);
    assert_eq!(reconstructed[0].status, ModelLaneStatus::Cancelled);
    let rebuilt_status: String =
        sqlx::query_scalar("SELECT status FROM model_lanes WHERE lane_id = $1")
            .bind(missing_row_a)
            .fetch_one(&pool)
            .await
            .expect("terminal projection is rebuilt from canonical EventLedger authority");
    assert_eq!(rebuilt_status, "cancelled");

    let tamper_run = "run-cloud-broadcast-tamper";
    let tamper_lane_a = "lane-cloud-tamper-a";
    let tamper_lane_b = "lane-cloud-tamper-b";
    let (tamper_plan, tamper_receipt) = seed_broadcast_authority(
        &store,
        tamper_run,
        vec![
            broadcast_target(
                tamper_lane_a,
                "openai",
                "model://dexterity/byok_cloud/gpt-4o-mini",
            ),
            broadcast_target(
                tamper_lane_b,
                "openai",
                "model://dexterity/byok_cloud/gpt-4o-mini",
            ),
        ],
    )
    .await;
    sqlx::query(
        r#"
        UPDATE kernel_event_ledger
        SET payload = jsonb_set(
            payload,
            '{record,target_bindings}',
            jsonb_build_array(payload #> '{record,target_bindings,0}')
        )
        WHERE event_id = $1
        "#,
    )
    .bind(&tamper_receipt.event_ledger_event_id)
    .execute(&pool)
    .await
    .expect("tamper partial EventLedger target binding");
    let (mut tamper_run_record, mut tamper_lane) =
        sample_cloud_run_lane(tamper_run, tamper_lane_a, ModelLaneStatus::Running);
    tamper_run_record.lane_ids = vec![tamper_lane_a.into(), tamper_lane_b.into()];
    bind_broadcast_authority(
        &mut tamper_run_record,
        &mut tamper_lane,
        &tamper_plan,
        &tamper_receipt,
    );
    let error = store
        .record_prepared_launch((tamper_run_record, tamper_lane))
        .await
        .expect_err("partial/tampered EventLedger target authority fails closed");
    assert_cx_mm_007(&error);
    assert_no_lane_row(&pool, tamper_lane_a).await;
}

#[tokio::test]
async fn broadcast_scoped_concurrent_grant_and_revoke_are_idempotent() {
    let (pool, store) = model_lane_store().await;
    let run_id = "run-cloud-broadcast-concurrent";
    let targets = vec![
        broadcast_target(
            "lane-cloud-concurrent-a",
            "openai",
            "model://dexterity/byok_cloud/gpt-4o-mini",
        ),
        broadcast_target(
            "lane-cloud-concurrent-b",
            "openai",
            "model://dexterity/byok_cloud/gpt-4o-mini",
        ),
    ];
    let plan_input = sample_broadcast_projection_plan(run_id, targets);
    let (plan_a, plan_b) = tokio::join!(
        store.record_cloud_projection_plan(plan_input.clone()),
        store.record_cloud_projection_plan(plan_input)
    );
    let plan_a = plan_a.expect("first concurrent projection grant");
    let plan_b = plan_b.expect("second concurrent projection grant replays");
    assert_eq!(plan_a.event_ledger_event_id, plan_b.event_ledger_event_id);

    let receipt_input = sample_broadcast_consent_receipt(&plan_a);
    let (receipt_a, receipt_b) = tokio::join!(
        store.record_cloud_consent_receipt(receipt_input.clone()),
        store.record_cloud_consent_receipt(receipt_input)
    );
    let receipt_a = receipt_a.expect("first concurrent consent grant");
    let receipt_b = receipt_b.expect("second concurrent consent grant replays");
    assert_eq!(
        receipt_a.event_ledger_event_id,
        receipt_b.event_ledger_event_id
    );

    let lane_a = "lane-cloud-concurrent-a";
    let lane_b = "lane-cloud-concurrent-b";
    let (mut run, mut first_lane) = sample_cloud_run_lane(run_id, lane_a, ModelLaneStatus::Running);
    run.lane_ids = vec![lane_a.into(), lane_b.into()];
    bind_broadcast_authority(&mut run, &mut first_lane, &plan_a, &receipt_a);
    store
        .record_prepared_launch((run, first_lane))
        .await
        .expect("launch first concurrent revoke target");
    let (_, mut second_lane) = sample_cloud_run_lane(run_id, lane_b, ModelLaneStatus::Running);
    bind_broadcast_lane(&mut second_lane, &plan_a, &receipt_a);
    store
        .record_lane(second_lane)
        .await
        .expect("launch second concurrent revoke target");

    let (revoke_a, revoke_b) = tokio::join!(
        store.test_commit_cloud_consent_revocation(
            &receipt_a.consent_receipt_id,
            "operator://mt017/concurrent-revoke",
            "concurrent broadcast revoke",
        ),
        store.test_commit_cloud_consent_revocation(
            &receipt_a.consent_receipt_id,
            "operator://mt017/concurrent-revoke",
            "concurrent broadcast revoke",
        )
    );
    let revoke_a = revoke_a.expect("first concurrent revoke");
    let revoke_b = revoke_b.expect("second concurrent revoke replays");
    assert_eq!(revoke_a.len(), 2);
    assert_eq!(revoke_b.len(), 2);
    let event_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger WHERE aggregate_type = 'model_lane_cloud_consent_receipt' AND aggregate_id = $1",
    )
    .bind(&receipt_a.consent_receipt_id)
    .fetch_one(&pool)
    .await
    .expect("count idempotent grant and revoke authority");
    assert_eq!(event_count, 2);
    let terminal_counts: Vec<(String, i64)> = sqlx::query_as(
        "SELECT aggregate_id, COUNT(*) FROM kernel_event_ledger WHERE aggregate_type = 'model_lane_terminal' AND payload->>'consent_receipt_id' = $1 GROUP BY aggregate_id ORDER BY aggregate_id",
    )
    .bind(&receipt_a.consent_receipt_id)
    .fetch_all(&pool)
    .await
    .expect("count exactly-once concurrent cancellation evidence");
    assert_eq!(
        terminal_counts,
        vec![(lane_a.into(), 1), (lane_b.into(), 1)]
    );
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
        lane_id: Some(lane_id.into()),
        model_session_id: Some(model_session_id_for(lane_id)),
        provider_kind: Some(provider_kind.into()),
        requested_model_id: Some(requested_model_id(provider_kind)),
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
        // Seeded through an unscoped store, so the source scope must be the
        // explicitly unattributed one (see `ensure_authority_matches_write_scope`).
        export_delegation: CloudExportDelegation {
            audience_refs: vec![format!("provider://{provider_kind}/byok")],
            source_scope: AccountBoundAuthority::unattributed(
                "MT006_PROOF_FIXTURE_WITHOUT_ACCOUNT_CONTEXT",
            ),
            authorization_receipt_ref: None,
        },
        consent_scope: ModelLaneCloudConsentScope::SingleLane,
        target_bindings: vec![],
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
        lane_id: Some(lane_id.into()),
        model_session_id: Some(model_session_id_for(lane_id)),
        provider_kind: Some(provider_kind.into()),
        requested_model_id: Some(requested_model_id(provider_kind)),
        scope_hash: sample_scope_hash(),
        consent_scope: ModelLaneCloudConsentScope::SingleLane,
        target_bindings: vec![],
        retention_policy: ModelLaneCloudRetentionPolicy::NoTrainingEphemeral,
        export_posture: ModelLaneCloudExportPosture::RedactedContextOnly,
        fan_out_targets: vec![format!("provider://{provider_kind}/byok")],
        approved: status == ModelLaneCloudConsentReceiptStatus::Approved,
        approver: AccountBoundAuthority::unattributed(
            "MT006_PROOF_FIXTURE_WITHOUT_ACCOUNT_CONTEXT",
        ),
        approved_by_ref: "operator://mt006/approval".into(),
        approved_at_utc: "2026-06-29T09:00:10Z".into(),
        valid_from_utc: valid_from_utc.into(),
        valid_until_utc: valid_until_utc.into(),
        revoked_at_utc: None,
        revocation_ref: None,
        revocation_input_hash: None,
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

fn broadcast_target(
    lane_id: &str,
    provider_kind: &str,
    requested_model_id: &str,
) -> ModelLaneCloudConsentTargetBinding {
    ModelLaneCloudConsentTargetBinding {
        lane_id: lane_id.into(),
        model_session_id: model_session_id_for(lane_id),
        provider_kind: provider_kind.into(),
        requested_model_id: requested_model_id.into(),
        capability_snapshot_ref: format!("capability-snapshot://{lane_id}"),
        provider_endpoint_ref: format!("{provider_kind}_byok"),
    }
}

fn sample_broadcast_projection_plan(
    run_id: &str,
    target_bindings: Vec<ModelLaneCloudConsentTargetBinding>,
) -> NewModelLaneCloudProjectionPlan {
    let first = target_bindings
        .first()
        .expect("broadcast authority requires targets");
    let mut plan = sample_projection_plan(run_id, &first.lane_id, &first.provider_kind);
    plan.projection_plan_id = format!("cloud-projection-plan://{run_id}/broadcast");
    plan.lane_id = None;
    plan.model_session_id = None;
    plan.provider_kind = None;
    plan.requested_model_id = None;
    plan.payload_artifact_ref = format!("artifact-store://mt017/{run_id}/broadcast/payload.json");
    plan.fan_out_targets = target_bindings
        .iter()
        .map(|target| format!("provider-endpoint://{}", target.provider_endpoint_ref))
        .collect();
    // A broadcast plan replaces the single-lane fan-out with the enumerated
    // provider endpoints, so its HBR-PRIV-007 audience is those endpoints. Left
    // at the inherited single-lane value it would name a destination this plan
    // no longer discloses, which the non-widening gate correctly refuses.
    plan.export_delegation.audience_refs = plan.fan_out_targets.clone();
    plan.consent_scope = ModelLaneCloudConsentScope::SingleRun;
    plan.target_bindings = vec![];
    plan.idempotency_key = format!("idem-projection-{run_id}-broadcast");
    plan.micro_task_id = "MT-017".into();
    plan
}

fn sample_broadcast_consent_receipt(
    plan: &ModelLaneCloudProjectionPlanRecord,
) -> NewModelLaneCloudConsentReceipt {
    let mut receipt = sample_consent_receipt(
        &plan.run_id,
        "run-scoped-placeholder",
        "openai",
        &plan.projection_plan_id,
        &plan.projection_plan_hash,
        ModelLaneCloudConsentReceiptStatus::Approved,
        "2026-01-01T00:00:00Z",
        "2027-01-01T00:00:00Z",
    );
    receipt.consent_receipt_id = format!("cloud-consent-receipt://{}/broadcast", plan.run_id);
    receipt.lane_id = None;
    receipt.model_session_id = None;
    receipt.provider_kind = None;
    receipt.requested_model_id = None;
    receipt.consent_scope = ModelLaneCloudConsentScope::SingleRun;
    receipt.target_bindings = vec![];
    receipt.fan_out_targets = plan.fan_out_targets.clone();
    receipt.idempotency_key = format!("idem-consent-{}-broadcast", plan.run_id);
    receipt.micro_task_id = "MT-017".into();
    receipt
}

async fn seed_broadcast_authority(
    store: &ModelLaneStore,
    run_id: &str,
    target_bindings: Vec<ModelLaneCloudConsentTargetBinding>,
) -> (
    ModelLaneCloudProjectionPlanRecord,
    ModelLaneCloudConsentReceiptRecord,
) {
    let plan = store
        .record_cloud_projection_plan(sample_broadcast_projection_plan(run_id, target_bindings))
        .await
        .expect("record broadcast projection authority");
    let receipt = store
        .record_cloud_consent_receipt(sample_broadcast_consent_receipt(&plan))
        .await
        .expect("record broadcast consent authority");
    (plan, receipt)
}

fn bind_broadcast_authority(
    run: &mut NewModelLaneRun,
    lane: &mut NewModelLane,
    plan: &ModelLaneCloudProjectionPlanRecord,
    receipt: &ModelLaneCloudConsentReceiptRecord,
) {
    run.projection_plan_ref = Some(plan.projection_plan_id.clone());
    run.consent_receipt_ref = Some(receipt.consent_receipt_id.clone());
    bind_broadcast_lane(lane, plan, receipt);
}

fn bind_broadcast_lane(
    lane: &mut NewModelLane,
    plan: &ModelLaneCloudProjectionPlanRecord,
    receipt: &ModelLaneCloudConsentReceiptRecord,
) {
    lane.projection_plan_ref = Some(plan.projection_plan_id.clone());
    lane.consent_receipt_ref = Some(receipt.consent_receipt_id.clone());
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
        restart_generation: 0,
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

fn coordinator_cloud_request(
    run_id: &str,
    lane_id: &str,
    provider: ByokCloudProvider,
) -> SpawnRequest {
    let (model_name, model_id, backend, adapter_id) = match &provider {
        ByokCloudProvider::OpenAi => (
            "gpt-4o-mini",
            "model://dexterity/byok_cloud/gpt-4o-mini",
            "cloud_lane_openai",
            "openai_byok",
        ),
        ByokCloudProvider::Anthropic => (
            "claude-sonnet-4",
            "model://dexterity/byok_cloud/claude-sonnet-4",
            "cloud_lane_anthropic",
            "anthropic_byok",
        ),
    };
    let instance = if matches!(&provider, ByokCloudProvider::Anthropic) {
        701
    } else {
        700
    };
    let mut contract = cloud_spawn_contract(run_id, lane_id);
    contract.backend = backend.into();
    contract.adapter_id = adapter_id.into();
    contract.candidate_model_ids = vec![model_id.into()];
    SpawnRequest::new(
        ModelInstanceId::new(ModelId::new_v7(), instance),
        RuntimeAdapterBinding::LlamaCpp,
        OWNER,
        format!("coordinator-session-{run_id}"),
    )
    .with_cloud_provider(ProviderKind::ByokCloud, model_name)
    .with_byok_cloud_provider(provider)
    .with_wp(WP_ID)
    .with_mt(MT_ID)
    .with_dexterity_launch(contract)
}

fn target_from_spawn_request(request: &SpawnRequest) -> ModelLaneCloudConsentTargetBinding {
    let contract = request
        .dexterity_launch
        .as_ref()
        .expect("coordinator cloud request has Dexterity contract");
    ModelLaneCloudConsentTargetBinding {
        lane_id: contract.lane_id.clone(),
        model_session_id: format!("swarm-session:{}", request.instance_id),
        provider_kind: match request.byok_cloud_provider {
            Some(ByokCloudProvider::Anthropic) => "anthropic".into(),
            _ => "openai".into(),
        },
        requested_model_id: contract
            .candidate_model_ids
            .first()
            .expect("coordinator cloud request has candidate model")
            .clone(),
        capability_snapshot_ref: contract.effective_capability_snapshot_ref.clone(),
        provider_endpoint_ref: contract.adapter_id.clone(),
    }
}

fn bind_spawn_request_authority(
    request: &mut SpawnRequest,
    plan: &ModelLaneCloudProjectionPlanRecord,
    receipt: &ModelLaneCloudConsentReceiptRecord,
) {
    let contract = request
        .dexterity_launch
        .as_mut()
        .expect("coordinator cloud request has Dexterity contract");
    contract.projection_plan_ref = Some(plan.projection_plan_id.clone());
    contract.consent_receipt_ref = Some(receipt.consent_receipt_id.clone());
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

struct RecordingBoundaryFactory {
    calls: Arc<Mutex<Vec<(String, String, String, String, String)>>>,
    cancellation_tokens: Arc<Mutex<Vec<CancellationToken>>>,
    unloads: Arc<AtomicUsize>,
    ledger: LedgerBatcher,
    entered: Option<Arc<Notify>>,
    release: Option<Arc<Notify>>,
}

#[async_trait]
impl ModelSessionFactory for RecordingBoundaryFactory {
    async fn create(&self, request: &SpawnRequest) -> Result<LiveSession, SwarmError> {
        let contract = request
            .dexterity_launch
            .as_ref()
            .expect("authorized cloud dispatch retains Dexterity contract");
        let provider = match request.byok_cloud_provider {
            Some(ByokCloudProvider::Anthropic) => "anthropic",
            _ => "openai",
        };
        self.calls.lock().expect("record boundary call").push((
            provider.into(),
            format!("swarm-session:{}", request.instance_id),
            contract
                .candidate_model_ids
                .first()
                .cloned()
                .unwrap_or_default(),
            contract.effective_capability_snapshot_ref.clone(),
            contract.adapter_id.clone(),
        ));
        if let Some(entered) = &self.entered {
            entered.notify_one();
        }
        if let Some(release) = &self.release {
            release.notified().await;
        }
        let record_id = ProcessOwnershipRecordId::new_v7();
        let os_pid = 58000 + request.instance_id.instance;
        self.ledger
            .record_start(
                ProcessStart::new(
                    ProcessEngineKind::HelperSubprocess,
                    request.owner_role.clone(),
                    request.owner_wp.clone(),
                )
                .with_process_uuid(record_id.as_uuid())
                .with_os_pid(os_pid)
                .with_parent_session_id(request.parent_session_id.clone())
                .with_wp_id(request.wp_id.clone().unwrap_or_default())
                .with_mt_id(request.mt_id.clone().unwrap_or_default()),
            )
            .map_err(|error| SwarmError::LedgerFailed(error.to_string()))?;
        let unloads = self.unloads.clone();
        let teardown: handshake_core::swarm_orchestration::SessionTeardown = Arc::new(move || {
            let unloads = unloads.clone();
            Box::pin(async move {
                unloads.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
        });
        let cancel = CancellationToken::new();
        self.cancellation_tokens
            .lock()
            .expect("record boundary cancellation token")
            .push(cancel.clone());
        Ok(LiveSession::new(
            Arc::new(BoundaryRuntime),
            request.instance_id.model_id,
            cancel,
            teardown,
            record_id,
            os_pid,
        ))
    }
}

/// The coordinator proof does not generate tokens, but it owns a concrete
/// runtime object whose cancellation and teardown are exercised by cleanup.
struct BoundaryRuntime;

#[async_trait]
impl ModelRuntime for BoundaryRuntime {
    async fn load(&mut self, _spec: LoadSpec) -> Result<ModelId, ModelRuntimeError> {
        unreachable!("the boundary factory receives the already-selected model id")
    }

    async fn unload(&mut self, _id: ModelId) -> Result<(), ModelRuntimeError> {
        Ok(())
    }

    fn generate(&self, _request: GenerateRequest) -> TokenStream {
        unreachable!("boundary launch proof does not generate")
    }

    async fn score(&self, _id: ModelId, _sequence: Vec<u32>) -> Result<Score, ModelRuntimeError> {
        unreachable!("boundary launch proof does not score")
    }

    async fn embed(&self, _id: ModelId, _text: &str) -> Result<Embedding, ModelRuntimeError> {
        unreachable!("boundary launch proof does not embed")
    }

    fn capabilities(&self, _id: ModelId) -> Result<&ModelCapabilities, ModelRuntimeError> {
        unreachable!("boundary launch proof does not query capabilities")
    }

    fn kv_cache(&self, _id: ModelId) -> Result<KvCacheHandle, ModelRuntimeError> {
        unreachable!("boundary launch proof does not use kv cache")
    }

    fn lora_stack(&self, _id: ModelId) -> Result<LoraStackHandle, ModelRuntimeError> {
        unreachable!("boundary launch proof does not use lora")
    }

    fn steering_hooks(&self, _id: ModelId) -> Result<SteeringHookHandle, ModelRuntimeError> {
        unreachable!("boundary launch proof does not use steering")
    }

    fn cancel(&self, _token: CancellationToken) {}
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

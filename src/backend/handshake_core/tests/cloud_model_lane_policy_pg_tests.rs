//! WP-1 MT-006: Dexterity cloud ProjectionPlan/ConsentReceipt authority proof.
//!
//! These tests require real PostgreSQL through `knowledge_pg_support` and prove
//! that cloud model lanes cannot launch or persist from refs alone. Projection
//! and consent records must exist as PostgreSQL/EventLedger authority, bind to
//! the exact run/lane/model-session/provider/scope/fan-out target, and revoke
//! deterministically before any provider call.

mod knowledge_pg_support;

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use async_trait::async_trait;
use futures::stream;
use handshake_core::model_runtime::registry::RuntimeBinding as RuntimeAdapterBinding;
use handshake_core::model_runtime::{
    CancellationToken, Embedding, GenerateRequest, KvCacheHandle, KvCachePolicy, LoadSpec,
    LoraStackHandle, ModelCapabilities, ModelId, ModelRuntime, ModelRuntimeError, ProviderKind,
    RuntimeKind, SamplingParams, Score, SteeringHookHandle, TokenStream,
};
use handshake_core::process_ledger::{
    LedgerBatcher, LedgerBatcherConfig, NoopOverflowSink, ProcessEngineKind,
    ProcessOwnershipRecordId, ProcessStart,
};
use handshake_core::swarm_orchestration::model_lane::{
    DexterityLaunchContract, LaunchAuthority, ModelLaneAuthority,
    ModelLaneCloudConsentReceiptStatus, ModelLaneCloudConsentScope, ModelLaneCloudExportPosture,
    ModelLaneCloudProjectionPlanStatus, ModelLaneCloudRetentionPolicy, ModelLaneKind,
    ModelLaneLocusBinding, ModelLaneMessageKind, ModelLaneProviderKind, ModelLaneRecoveryState,
    ModelLaneRoutingMetadata, ModelLaneStatus, ModelLaneStore, ModelLaneTarget,
    NewModelLane, NewModelLaneCloudConsentReceipt, NewModelLaneCloudProjectionPlan,
    NewModelLaneMessage, NewModelLaneRun, RuntimeBinding,
};
use handshake_core::swarm_orchestration::{
    ByokCloudProvider, LiveSession, ModelInstanceId, ModelSessionFactory, RecordingSwarmSink,
    RunBudget, SpawnRequest, SwarmConfig, SwarmCoordinator, SwarmError,
};
use serde_json::{json, Value};

#[tokio::test]
async fn cloud_projection_and_consent_receipts_persist_and_replay() {
    let (pool, store) = model_lane_store().await;
    let projection = store
        .record_cloud_projection_plan(sample_projection_plan("lane-cloud", "openai", "active"))
        .await
        .expect("record cloud projection plan");
    assert_eq!(projection.run_id, "run-mt006");
    assert_eq!(projection.lane_id, "lane-cloud");
    assert_eq!(projection.provider_kind, ModelLaneProviderKind::OpenAi);
    assert_eq!(projection.status, ModelLaneCloudProjectionPlanStatus::Active);
    assert!(projection.event_ledger_event_id.starts_with("KE-"));
    assert!(projection.projection_plan_hash.len() == 64);

    let consent = store
        .record_cloud_consent_receipt(sample_consent_receipt(
            "lane-cloud",
            "openai",
            &projection.projection_plan_hash,
            "valid",
        ))
        .await
        .expect("record cloud consent receipt");
    assert_eq!(consent.status, ModelLaneCloudConsentReceiptStatus::Approved);
    assert_eq!(consent.projection_plan_hash, projection.projection_plan_hash);
    assert!(consent.event_ledger_seq > projection.event_ledger_seq);
    assert!(consent.consent_receipt_hash.len() == 64);

    let replay = store
        .replay_cloud_consent_authority("run-mt006")
        .await
        .expect("replay cloud consent authority");
    assert_eq!(replay.projection_plans.len(), 1);
    assert_eq!(replay.consent_receipts.len(), 1);
    assert_eq!(
        replay.consent_receipts[0].projection_plan_id,
        "projection-plan://mt006/lane-cloud"
    );

    store
        .record_prepared_launch((sample_run("run-mt006"), sample_cloud_lane("lane-cloud")))
        .await
        .expect("valid cloud launch records through durable projection/consent authority");

    let advisory = store
        .record_message(sample_cloud_message(
            "msg-cloud-advisory",
            ModelLaneAuthority::Advisory,
            json!({
                "projection_plan_id": "projection-plan://mt006/lane-cloud",
                "consent_receipt_id": "consent://mt006/lane-cloud",
                "redaction_policy_ref": "redaction-policy://mt006/lane-cloud",
                "authority": "advisory_until_promotion"
            }),
        ))
        .await
        .expect("cloud advisory output records projection/redaction metadata");
    assert_eq!(advisory.authority, ModelLaneAuthority::Advisory);
    assert_eq!(
        advisory.diagnostic_payload["projection_plan_id"],
        "projection-plan://mt006/lane-cloud"
    );

    let direct_promoted = store
        .record_message(sample_cloud_message(
            "msg-cloud-direct-promoted",
            ModelLaneAuthority::Promoted,
            json!({
                "projection_plan_id": "projection-plan://mt006/lane-cloud",
                "consent_receipt_id": "consent://mt006/lane-cloud",
                "redaction_policy_ref": "redaction-policy://mt006/lane-cloud"
            }),
        ))
        .await
        .expect_err("cloud output must remain advisory until explicit promotion");
    assert!(
        direct_promoted.to_string().contains("PromotionGate"),
        "direct promoted cloud output must fail closed: {direct_promoted}"
    );

    let ledger_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger \
         WHERE aggregate_type IN ( \
           'model_lane_cloud_projection_plan', \
           'model_lane_cloud_consent_receipt', \
           'model_lane', \
           'model_lane_message' \
         )",
    )
    .fetch_one(&pool)
    .await
    .expect("count cloud policy EventLedger rows");
    assert!(
        ledger_rows >= 5,
        "projection, consent, lane, and advisory message rows must be EventLedger-backed"
    );

    let registry_rows = store
        .schema_registry_rows()
        .await
        .expect("schema registry rows");
    for schema_id in [
        "hsk.model_lane_cloud_projection_plan@1",
        "hsk.model_lane_cloud_consent_receipt@1",
    ] {
        assert!(
            registry_rows.iter().any(|row| row.schema_id == schema_id),
            "missing cloud authority schema registry row {schema_id}"
        );
    }
}

#[tokio::test]
async fn cloud_lane_rejects_missing_expired_mismatched_and_revoked_consent() {
    let (pool, store) = model_lane_store().await;
    let missing = store
        .record_prepared_launch((sample_run("run-missing"), sample_cloud_lane_for("run-missing", "lane-missing")))
        .await
        .expect_err("cloud lane refs without rows must fail closed");
    assert!(
        missing.to_string().contains("CX-MM-007"),
        "missing consent/projection must surface CX-MM-007: {missing}"
    );
    let no_partial_rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM model_lanes")
        .fetch_one(&pool)
        .await
        .expect("count no partial lanes");
    assert_eq!(no_partial_rows, 0, "denied cloud launch must not create model_lanes rows");

    let calls = Arc::new(AtomicUsize::new(0));
    let (ledger, _drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("manual process ledger");
    let coordinator = SwarmCoordinator::new_with_model_lane_store(
        SwarmConfig::new(RunBudget::defaulted(1)),
        Arc::new(CountingCloudFactory {
            calls: calls.clone(),
        }),
        Arc::new(RecordingSwarmSink::new()),
        ledger,
        store.clone(),
    );
    let err = coordinator
        .spawn_session(cloud_spawn_request("run-missing-spawn", "lane-missing-spawn"))
        .await
        .expect_err("missing durable consent must fail before cloud factory");
    assert!(
        err.to_string().contains("CX-MM-007"),
        "coordinator preflight must surface typed consent status: {err}"
    );
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "cloud provider factory must not be called when consent/projection authority is missing"
    );

    seed_projection_and_consent(&store, "run-expired", "lane-expired", "expired").await;
    let expired = store
        .record_prepared_launch((
            sample_run("run-expired"),
            sample_cloud_lane_for("run-expired", "lane-expired"),
        ))
        .await
        .expect_err("expired consent must fail closed");
    assert!(
        expired.to_string().contains("expired") && expired.to_string().contains("CX-MM-007"),
        "expired receipt must be typed: {expired}"
    );

    let projection = store
        .record_cloud_projection_plan(sample_projection_plan_for(
            "run-mismatched",
            "lane-mismatched",
            "openai",
            "active",
        ))
        .await
        .expect("projection");
    let mut mismatched = sample_consent_receipt_for(
        "run-mismatched",
        "lane-other",
        "openai",
        &projection.projection_plan_hash,
        "valid",
    );
    mismatched.projection_plan_id = "projection-plan://mt006/lane-mismatched".into();
    mismatched.consent_receipt_id = "consent://mt006/lane-mismatched".into();
    store
        .record_cloud_consent_receipt(mismatched)
        .await
        .expect("record mismatched consent row; launch validation rejects exact lane mismatch");
    let mismatch = store
        .record_prepared_launch((
            sample_run("run-mismatched"),
            sample_cloud_lane_for("run-mismatched", "lane-mismatched"),
        ))
        .await
        .expect_err("lane-mismatched consent must fail closed");
    assert!(
        mismatch.to_string().contains("lane_id") && mismatch.to_string().contains("CX-MM-007"),
        "mismatch must name bound lane: {mismatch}"
    );

    seed_projection_and_consent(&store, "run-revoked", "lane-revoked", "revoked").await;
    let revoked = store
        .record_prepared_launch((
            sample_run("run-revoked"),
            sample_cloud_lane_for("run-revoked", "lane-revoked"),
        ))
        .await
        .expect_err("revoked consent must fail closed");
    assert!(
        revoked.to_string().contains("revoked") && revoked.to_string().contains("CX-MM-007"),
        "revoked receipt must be typed: {revoked}"
    );

    let denial_events: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger \
         WHERE aggregate_type = 'model_lane_cloud_consent_denial' \
           AND payload->>'consent_status' = 'CX-MM-007'",
    )
    .fetch_one(&pool)
    .await
    .expect("count denial evidence rows");
    assert!(
        denial_events >= 4,
        "every denied cloud launch path must append EventLedger evidence"
    );
}

#[tokio::test]
async fn cloud_consent_revocation_cancels_pending_lanes_with_eventledger_evidence() {
    let (pool, store) = model_lane_store().await;
    seed_projection_and_consent(&store, "run-revoke-active", "lane-revoke-active", "valid").await;
    store
        .record_prepared_launch((
            sample_run("run-revoke-active"),
            sample_cloud_lane_for("run-revoke-active", "lane-revoke-active"),
        ))
        .await
        .expect("record pending cloud lane");

    let cancelled = store
        .revoke_cloud_consent_receipt(
            "consent://mt006/lane-revoke-active",
            "operator://mt006/revoke",
            "operator_revoked_cloud_fanout",
        )
        .await
        .expect("revoke cloud consent receipt");
    assert_eq!(cancelled.len(), 1);
    assert_eq!(cancelled[0].lane_id, "lane-revoke-active");
    assert_eq!(cancelled[0].status, ModelLaneStatus::Cancelled);
    assert_eq!(cancelled[0].failstate_code.as_deref(), Some("CX-MM-007"));
    assert!(cancelled[0]
        .reason_ref
        .as_deref()
        .unwrap_or_default()
        .contains("consent_revoked"));
    assert_eq!(
        cancelled[0].diagnostic_payload["consent_status"],
        "CX-MM-007"
    );

    let replay = store
        .replay_run("run-revoke-active")
        .await
        .expect("replay revoked lane");
    assert_eq!(replay.lanes[0].status, ModelLaneStatus::Cancelled);
    assert_eq!(replay.lanes[0].failstate_code.as_deref(), Some("CX-MM-007"));

    let consent_replay = store
        .replay_cloud_consent_authority("run-revoke-active")
        .await
        .expect("replay revoked consent");
    assert_eq!(
        consent_replay.consent_receipts[0].status,
        ModelLaneCloudConsentReceiptStatus::Revoked
    );
    assert!(consent_replay.consent_receipts[0].revoked_at_utc.is_some());

    let terminal_payload: Value = sqlx::query_scalar(
        "SELECT payload FROM kernel_event_ledger \
         WHERE aggregate_type = 'model_lane_terminal' \
           AND aggregate_id = 'lane-revoke-active' \
           AND event_type = 'SESSION_CANCELLED' \
         ORDER BY event_sequence DESC LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .expect("revocation terminal payload");
    assert_eq!(terminal_payload["consent_status"], "CX-MM-007");
    assert_eq!(
        terminal_payload["consent_receipt_id"],
        "consent://mt006/lane-revoke-active"
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
        .expect("connect isolated cloud policy schema");
    let store = ModelLaneStore::new(pool.clone());
    (pool, store)
}

async fn seed_projection_and_consent(
    store: &ModelLaneStore,
    run_id: &str,
    lane_id: &str,
    status: &str,
) {
    let projection = store
        .record_cloud_projection_plan(sample_projection_plan_for(
            run_id, lane_id, "openai", "active",
        ))
        .await
        .expect("seed projection");
    store
        .record_cloud_consent_receipt(sample_consent_receipt_for(
            run_id,
            lane_id,
            "openai",
            &projection.projection_plan_hash,
            status,
        ))
        .await
        .expect("seed consent");
}

fn sample_projection_plan(
    lane_id: &str,
    provider: &str,
    status: &str,
) -> NewModelLaneCloudProjectionPlan {
    sample_projection_plan_for("run-mt006", lane_id, provider, status)
}

fn sample_projection_plan_for(
    run_id: &str,
    lane_id: &str,
    provider: &str,
    status: &str,
) -> NewModelLaneCloudProjectionPlan {
    NewModelLaneCloudProjectionPlan {
        projection_plan_id: format!("projection-plan://mt006/{lane_id}"),
        run_id: run_id.into(),
        trace_id: format!("trace-{run_id}"),
        lane_id: lane_id.into(),
        model_session_id: model_session_id_for(lane_id),
        provider_kind: provider_kind(provider),
        requested_model_id: "model://dexterity/byok_cloud/gpt-4o".into(),
        scope_hash: sample_sha256('6'),
        source_artifact_refs: vec![format!("artifact://mt006/{lane_id}/source")],
        payload_artifact_ref: format!("artifact://mt006/{lane_id}/cloud-payload"),
        payload_sha256: sample_sha256('7'),
        redaction_policy_ref: format!("redaction-policy://mt006/{lane_id}"),
        redaction_summary: "workspace paths and local-only memory redacted".into(),
        retention_policy: ModelLaneCloudRetentionPolicy::NoTrainingEphemeral,
        export_posture: ModelLaneCloudExportPosture::RedactedPromptOnly,
        provider_profile_ref: format!("provider-profile://mt006/{provider}"),
        fan_out_targets: vec![lane_id.into()],
        consent_scope: ModelLaneCloudConsentScope::SingleLane,
        status: match status {
            "active" => ModelLaneCloudProjectionPlanStatus::Active,
            "superseded" => ModelLaneCloudProjectionPlanStatus::Superseded,
            other => panic!("unknown projection status {other}"),
        },
        event_ledger_stream_id: format!("event-ledger://mt006/{run_id}"),
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-006".into(),
        task_board_id: "task-board://wp-1".into(),
        owner_session: "KERNEL_BUILDER-MT006".into(),
        idempotency_key: format!("idem-projection-{run_id}-{lane_id}"),
        created_at_utc: "2026-06-29T09:00:00Z".into(),
        user_manual_behavior_id: "usermanual://model-lane-cloud-projection-consent".into(),
        diagnostic_payload: json!({
            "flight_recorder": "EventLedger-backed cloud projection authority",
            "internal_diagnostics": "DEFERRED: MT-008 diagnostic tier",
            "palmistry": "DEFERRED: external watcher worktree"
        }),
    }
}

fn sample_consent_receipt(
    lane_id: &str,
    provider: &str,
    projection_hash: &str,
    status: &str,
) -> NewModelLaneCloudConsentReceipt {
    sample_consent_receipt_for("run-mt006", lane_id, provider, projection_hash, status)
}

fn sample_consent_receipt_for(
    run_id: &str,
    lane_id: &str,
    provider: &str,
    projection_hash: &str,
    status: &str,
) -> NewModelLaneCloudConsentReceipt {
    let (valid_from_utc, valid_until_utc, revoked_at_utc, receipt_status) = match status {
        "valid" => (
            "2026-01-01T00:00:00Z",
            "2027-01-01T00:00:00Z",
            None,
            ModelLaneCloudConsentReceiptStatus::Approved,
        ),
        "expired" => (
            "2025-01-01T00:00:00Z",
            "2025-02-01T00:00:00Z",
            None,
            ModelLaneCloudConsentReceiptStatus::Approved,
        ),
        "revoked" => (
            "2026-01-01T00:00:00Z",
            "2027-01-01T00:00:00Z",
            Some("2026-06-29T09:05:00Z".to_string()),
            ModelLaneCloudConsentReceiptStatus::Revoked,
        ),
        other => panic!("unknown consent status {other}"),
    };
    NewModelLaneCloudConsentReceipt {
        consent_receipt_id: format!("consent://mt006/{lane_id}"),
        projection_plan_id: format!("projection-plan://mt006/{lane_id}"),
        projection_plan_hash: projection_hash.into(),
        run_id: run_id.into(),
        trace_id: format!("trace-{run_id}"),
        lane_id: lane_id.into(),
        model_session_id: model_session_id_for(lane_id),
        provider_kind: provider_kind(provider),
        requested_model_id: "model://dexterity/byok_cloud/gpt-4o".into(),
        scope_hash: sample_sha256('6'),
        consent_scope: ModelLaneCloudConsentScope::SingleLane,
        retention_policy: ModelLaneCloudRetentionPolicy::NoTrainingEphemeral,
        export_posture: ModelLaneCloudExportPosture::RedactedPromptOnly,
        fan_out_targets: vec![lane_id.into()],
        approved: true,
        approved_by_ref: "operator://mt006/approve-cloud".into(),
        approved_at_utc: "2026-06-29T09:01:00Z".into(),
        valid_from_utc: valid_from_utc.into(),
        valid_until_utc: valid_until_utc.into(),
        revoked_at_utc,
        revocation_ref: None,
        status: receipt_status,
        event_ledger_stream_id: format!("event-ledger://mt006/{run_id}"),
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-006".into(),
        task_board_id: "task-board://wp-1".into(),
        owner_session: "KERNEL_BUILDER-MT006".into(),
        idempotency_key: format!("idem-consent-{run_id}-{lane_id}"),
        created_at_utc: "2026-06-29T09:01:00Z".into(),
        user_manual_behavior_id: "usermanual://model-lane-cloud-projection-consent".into(),
        diagnostic_payload: json!({
            "flight_recorder": "EventLedger-backed cloud consent receipt",
            "consent_status": "approved"
        }),
    }
}

fn sample_run(run_id: &str) -> NewModelLaneRun {
    NewModelLaneRun {
        run_id: run_id.into(),
        trace_id: format!("trace-{run_id}"),
        run_span_id: format!("span-{run_id}"),
        coordinator_session_id: "coordinator-session-mt006".into(),
        routing_policy: "cloud_projection_consent_authority".into(),
        context_bundle_id: format!("context-bundle://mt006/{run_id}"),
        lane_ids: vec!["lane-cloud".into()],
        event_ledger_stream_id: format!("event-ledger://mt006/{run_id}"),
        artifact_namespace: format!("artifact://mt006/{run_id}"),
        projection_plan_ref: Some("projection-plan://mt006/lane-cloud".into()),
        consent_receipt_ref: Some("consent://mt006/lane-cloud".into()),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-006".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: "KERNEL_BUILDER-MT006".into(),
        idempotency_key: format!("idem-run-{run_id}"),
        replay_order_key: "00000001/run".into(),
        replay_after_event_ledger_seq: None,
        recovery_state: ModelLaneRecoveryState::Restartable,
        failstate_code: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane-cloud-projection-consent#run".into()),
        locus_binding: Some(sample_locus(run_id, "lane-cloud")),
        memory_pack_ref: "memory-pack://fems/mt006/cloud-safe".into(),
        memory_pack_hash: sample_sha256('8'),
        determinism_mode: "deterministic_replay".into(),
        budget_summary_ref: "budget://mt006/cloud".into(),
        selected_model_id: Some("model://dexterity/byok_cloud/gpt-4o".into()),
        candidate_model_ids: vec!["model://dexterity/byok_cloud/gpt-4o".into()],
        procedural_review_status: "projection_consent_preflighted".into(),
        truncation_warning_ref: None,
        rejection_reason_refs: vec![],
    }
}

fn sample_cloud_lane(lane_id: &str) -> NewModelLane {
    sample_cloud_lane_for("run-mt006", lane_id)
}

fn sample_cloud_lane_for(run_id: &str, lane_id: &str) -> NewModelLane {
    NewModelLane {
        lane_id: lane_id.into(),
        run_id: run_id.into(),
        trace_id: format!("trace-{run_id}"),
        lane_span_id: format!("span-{lane_id}"),
        event_ledger_stream_id: format!("event-ledger://mt006/{run_id}"),
        kind: ModelLaneKind::CloudModel,
        role: "cloud-reviewer".into(),
        backend: "cloud_lane_openai".into(),
        model_id: Some("model://dexterity/byok_cloud/gpt-4o".into()),
        session_id: format!("session-{lane_id}"),
        model_session_id: model_session_id_for(lane_id),
        adapter_id: "openai_byok".into(),
        runtime_binding: RuntimeBinding::Cloud,
        launch_authority: LaunchAuthority::CloudLane,
        provider_kind: ModelLaneProviderKind::OpenAi,
        capability_token_ids: vec!["capability://dexterity/cloud-generate".into()],
        effective_capability_snapshot_ref: Some(format!("capability-snapshot://mt006/{lane_id}")),
        capability_negotiation_ref: Some(format!("capability-negotiation://mt006/{lane_id}")),
        provider_feature_profile_ref: Some("provider-feature-profile://dexterity/openai".into()),
        requested_execution_policy_ref: Some("execution-policy://requested/cloud".into()),
        effective_execution_policy_ref: Some("execution-policy://effective/cloud".into()),
        projection_plan_ref: Some(format!("projection-plan://mt006/{lane_id}")),
        consent_receipt_ref: Some(format!("consent://mt006/{lane_id}")),
        tool_gate_decision_refs: vec!["toolgate://mt006/read-context".into()],
        status: ModelLaneStatus::Ready,
        recovery_state: ModelLaneRecoveryState::Restartable,
        heartbeat_at_utc: Some("2026-06-29T09:02:00Z".into()),
        lease_expires_at_utc: Some("2026-06-29T09:12:00Z".into()),
        reclaim_after_utc: Some("2026-06-29T09:13:00Z".into()),
        restart_generation: 0,
        cancellation_ref: Some(format!("cancel-token://mt006/{lane_id}")),
        reclaim_policy_ref: Some("reclaim-policy://mt006/cloud".into()),
        terminal_status_mapping_ref: Some("terminal-status://session-broker/cloud".into()),
        process_ownership_ref: Some(format!("process-ledger://mt006/{lane_id}")),
        no_os_process_reason_ref: None,
        backpressure_ref: None,
        loop_counter_ref: Some("loop-counter://mt006/cloud".into()),
        last_runtime_status_ref: Some("runtime-status://mt006/ready".into()),
        last_recovery_event_ref: None,
        failstate_code: None,
        startup_failure_ref: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane-cloud-projection-consent#lane".into()),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-006".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: "KERNEL_BUILDER-MT006".into(),
        locus_binding: Some(sample_locus(run_id, lane_id)),
    }
}

fn sample_cloud_message(
    message_id: &str,
    authority: ModelLaneAuthority,
    diagnostic_payload: Value,
) -> NewModelLaneMessage {
    NewModelLaneMessage {
        message_id: message_id.into(),
        run_id: "run-mt006".into(),
        trace_id: "trace-run-mt006".into(),
        message_span_id: format!("span-{message_id}"),
        parent_span_id: Some("span-lane-cloud".into()),
        linked_span_contexts: vec!["span-run-mt006".into()],
        from_lane_id: "lane-cloud".into(),
        to_lane: ModelLaneTarget::Coordinator,
        routing: Some(ModelLaneRoutingMetadata {
            target_role: "coordinator".into(),
            target_session: "coordinator-session-mt006".into(),
            correlation_id: format!("corr-{message_id}"),
            requires_ack: true,
            ack_for: None,
        }),
        kind: ModelLaneMessageKind::Critique,
        payload_ref: format!("artifact://mt006/{message_id}"),
        payload_sha256: sample_sha256('9'),
        event_ledger_stream_id: "event-ledger://mt006/run-mt006".into(),
        summary: "cloud lane critique over redacted projection".into(),
        authority,
        promotion_decision_id: None,
        promotion_gate_ref: None,
        promotion_receipt_ref: None,
        validator_verdict_ref: None,
        operator_decision_ref: None,
        promoted_artifact_ref: None,
        promoted_artifact_sha256: None,
        promoted_artifact_version: None,
        tool_gate_decision_refs: vec!["toolgate://mt006/read-context".into()],
        coordinator_session_id: "coordinator-session-mt006".into(),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-006".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: "KERNEL_BUILDER-MT006".into(),
        locus_binding: Some(sample_locus("run-mt006", "lane-cloud")),
        idempotency_key: format!("idem-message-{message_id}"),
        replay_order_key: format!("00000020/{message_id}"),
        replay_after_event_ledger_seq: Some(1),
        proposal_ref: None,
        crdt_update_ref: None,
        crdt_base_snapshot_ref: None,
        crdt_state_vector: None,
        crdt_proposal_ref: None,
        crdt_stale_base_ref: None,
        failstate_code: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane-cloud-projection-consent#messages".into()),
        created_at_utc: "2026-06-29T09:03:00Z".into(),
        diagnostic_payload,
    }
}

fn cloud_spawn_request(run_id: &str, lane_id: &str) -> SpawnRequest {
    SpawnRequest::new(
        ModelInstanceId::new(ModelId::new_v7(), 606),
        RuntimeAdapterBinding::LlamaCpp,
        "KERNEL_BUILDER-MT006",
        "coordinator-session-mt006",
    )
    .with_wp("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1")
    .with_mt("MT-006")
    .with_cloud_provider(ProviderKind::ByokCloud, "gpt-4o")
    .with_byok_cloud_provider(ByokCloudProvider::OpenAi)
    .with_dexterity_launch(DexterityLaunchContract {
        run_id: run_id.into(),
        lane_id: lane_id.into(),
        trace_id: format!("trace-{run_id}"),
        run_span_id: format!("span-run-{run_id}"),
        lane_span_id: format!("span-{lane_id}"),
        routing_policy: "cloud_projection_consent_authority".into(),
        context_bundle_id: format!("context-bundle://mt006/{run_id}"),
        event_ledger_stream_id: format!("event-ledger://mt006/{run_id}"),
        artifact_namespace: format!("artifact://mt006/{run_id}"),
        task_board_id: "task-board://wp-1".into(),
        locus_binding_ref: format!("locus://wp1/mt006/{lane_id}"),
        role: "cloud-reviewer".into(),
        backend: "cloud_lane_openai".into(),
        adapter_id: "openai_byok".into(),
        capability_token_ids: vec!["capability://dexterity/cloud-generate".into()],
        effective_capability_snapshot_ref: format!("capability-snapshot://mt006/{lane_id}"),
        projection_plan_ref: Some(format!("projection-plan://mt006/{lane_id}")),
        consent_receipt_ref: Some(format!("consent://mt006/{lane_id}")),
        tool_gate_decision_refs: vec!["toolgate://mt006/read-context".into()],
        memory_pack_ref: "memory-pack://fems/mt006/cloud-safe".into(),
        memory_pack_hash: sample_sha256('8'),
        determinism_mode: "deterministic_replay".into(),
        budget_summary_ref: "budget://mt006/cloud".into(),
        candidate_model_ids: vec!["model://dexterity/byok_cloud/gpt-4o".into()],
        procedural_review_status: "projection_consent_preflighted".into(),
        truncation_warning_ref: None,
        rejection_reason_refs: vec![],
        run_recovery_hint_ref: Some("usermanual://model-lane-cloud-projection-consent#run".into()),
        lane_recovery_hint_ref: Some("usermanual://model-lane-cloud-projection-consent#lane".into()),
    })
}

fn sample_locus(run_id: &str, lane_id: &str) -> ModelLaneLocusBinding {
    ModelLaneLocusBinding {
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-006".into(),
        task_board_id: Some("task-board://wp-1".into()),
        coordinator_session_id: "coordinator-session-mt006".into(),
        session_id: format!("session-{lane_id}"),
        model_session_id: model_session_id_for(lane_id),
        owner_session: "KERNEL_BUILDER-MT006".into(),
        locus_binding_ref: format!("locus://wp1/mt006/{run_id}/{lane_id}"),
    }
}

fn model_session_id_for(lane_id: &str) -> String {
    format!("model-session-{lane_id}")
}

fn provider_kind(provider: &str) -> ModelLaneProviderKind {
    match provider {
        "openai" => ModelLaneProviderKind::OpenAi,
        "anthropic" => ModelLaneProviderKind::Anthropic,
        other => panic!("unknown provider {other}"),
    }
}

fn sample_sha256(ch: char) -> String {
    std::iter::repeat(ch).take(64).collect()
}

struct CountingCloudFactory {
    calls: Arc<AtomicUsize>,
}

#[async_trait]
impl ModelSessionFactory for CountingCloudFactory {
    async fn create(&self, request: &SpawnRequest) -> Result<LiveSession, SwarmError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let record_id = ProcessOwnershipRecordId::new_v7();
        let start = ProcessStart::new(
            ProcessEngineKind::HelperSubprocess,
            request.owner_role.clone(),
            request.owner_wp.clone(),
        )
        .with_process_uuid(record_id.as_uuid())
        .with_os_pid(60600)
        .with_parent_session_id(request.parent_session_id.clone())
        .with_wp_id(request.wp_id.clone().unwrap_or_default())
        .with_mt_id(request.mt_id.clone().unwrap_or_default());
        let (ledger, _drain) = LedgerBatcher::manual_for_tests(
            LedgerBatcherConfig::default(),
            Arc::new(NoopOverflowSink),
        )
        .map_err(|err| SwarmError::LedgerFailed(err.to_string()))?;
        ledger
            .record_start(start)
            .map_err(|err| SwarmError::LedgerFailed(err.to_string()))?;
        Ok(LiveSession::new(
            Arc::new(DummyRuntime::new()),
            ModelId::new_v7(),
            CancellationToken::new(),
            Box::new(|| Box::pin(async { Ok(()) })),
            record_id,
            60600,
        ))
    }
}

struct DummyRuntime {
    capabilities: ModelCapabilities,
    kv: KvCacheHandle,
    lora: LoraStackHandle,
    steering: SteeringHookHandle,
}

impl DummyRuntime {
    fn new() -> Self {
        Self {
            capabilities: ModelCapabilities::default(),
            kv: KvCacheHandle::new("mt006-kv"),
            lora: LoraStackHandle::new("mt006-lora"),
            steering: SteeringHookHandle::new("mt006-steering"),
        }
    }
}

#[async_trait]
impl ModelRuntime for DummyRuntime {
    async fn load(&mut self, _spec: LoadSpec) -> Result<ModelId, ModelRuntimeError> {
        Ok(ModelId::new_v7())
    }

    async fn unload(&mut self, _id: ModelId) -> Result<(), ModelRuntimeError> {
        Ok(())
    }

    fn generate(&self, req: GenerateRequest) -> TokenStream {
        let cancel = req.cancel.clone();
        let items = (0..req.max_tokens.min(1)).map(move |i| {
            if cancel.is_cancelled() {
                Err(ModelRuntimeError::Cancelled)
            } else {
                Ok(handshake_core::model_runtime::GeneratedToken {
                    token_id: i,
                    text: format!("mt006-token-{i}"),
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

fn _load_spec() -> LoadSpec {
    LoadSpec {
        artifact_path: "mt006-cloud-proof".into(),
        sha256_expected: sample_sha256('a'),
        runtime_kind: RuntimeKind::LlamaCpp,
        sampling_defaults: SamplingParams::default(),
        kv_cache_policy: KvCachePolicy::default(),
        declared_capabilities: ModelCapabilities::default(),
        provider: ProviderKind::ByokCloud,
        engine_origin: Some("mt006-cloud-proof".into()),
        external_engine_import: None,
    }
}

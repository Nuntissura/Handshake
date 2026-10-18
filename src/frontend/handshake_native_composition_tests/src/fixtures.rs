//! Shared fixtures for the native Swarm diagnostics composition target.

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;
use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;
use handshake_native::swarm_lane_diagnostics::{decode_projection_for_native_surface, SwarmLaneDiagnosticsLane, SwarmLaneDiagnosticsMessage, SwarmLaneDiagnosticsMtStatus, SwarmLaneDiagnosticsProjection, SwarmLaneDiagnosticsResourceScope, SwarmLaneDiagnosticsRun, SwarmLaneDiagnosticsTier, SwarmLaneDiagnosticsTransport, SwarmLaneRoutingExecutionDiagnostics, SwarmLaneRoutingOutboxDiagnostics, SwarmLaneRoutingStageDiagnostics};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

#[derive(Clone, Deserialize)]
pub struct DiagnosticsProjectionProvenance {
    pub schema_id: String,
    pub proof_nonce: String,
    pub projection_schema_id: String,
    pub artifact_sha256: String,
    pub producer_test_id: String,
    pub producer_status: String,
    pub producer_completed_at_unix_ms: u64,
}

pub fn validate_projection_provenance(
    provenance: &DiagnosticsProjectionProvenance,
    artifact_bytes: &[u8],
    expected_schema: &str,
    expected_nonce: &str,
    expected_producer: &str,
    now_unix_ms: u64,
) -> Result<(), String> {
    if provenance.schema_id != expected_schema {
        return Err("diagnostics provenance schema_id mismatch".into());
    }
    if provenance.proof_nonce != expected_nonce {
        return Err("diagnostics provenance proof_nonce mismatch".into());
    }
    if provenance.projection_schema_id != "hsk.model_lane_diagnostics_projection@3" {
        return Err("diagnostics provenance projection_schema_id mismatch".into());
    }
    if provenance.producer_test_id != expected_producer {
        return Err("diagnostics provenance producer_test_id mismatch".into());
    }
    if provenance.producer_status != "passed_all_backend_assertions" {
        return Err("diagnostics provenance producer_status incomplete".into());
    }
    if provenance.producer_completed_at_unix_ms > now_unix_ms.saturating_add(30_000) {
        return Err("diagnostics provenance completion is from the future".into());
    }
    if now_unix_ms.saturating_sub(provenance.producer_completed_at_unix_ms) > 30 * 60 * 1_000 {
        return Err("diagnostics provenance completion is stale".into());
    }
    if provenance.artifact_sha256 != format!("{:x}", Sha256::digest(artifact_bytes)) {
        return Err("diagnostics provenance artifact_sha256 mismatch".into());
    }
    Ok(())
}

pub fn load_current_mt009_backend_projection() -> SwarmLaneDiagnosticsProjection {
    let artifact_root = std::env::var("HANDSHAKE_ARTIFACTS_DIR")
        .expect("backend MT-009 proof must provide HANDSHAKE_ARTIFACTS_DIR");
    let artifact_root = std::fs::canonicalize(artifact_root)
        .expect("HANDSHAKE_ARTIFACTS_DIR must resolve to an existing directory");
    let manifest_dir = std::fs::canonicalize(env!("CARGO_MANIFEST_DIR"))
        .expect("native crate manifest directory must resolve");
    let worktree_root = manifest_dir
        .parent()
        .and_then(std::path::Path::parent)
        .and_then(std::path::Path::parent)
        .expect("native crate must live below the worktree src directory");
    let expected_root = std::fs::canonicalize(
        worktree_root
            .parent()
            .expect("worktree must have a parent")
            .join("Handshake_Artifacts"),
    )
    .expect("canonical sibling Handshake_Artifacts directory must exist");
    assert_eq!(
        artifact_root, expected_root,
        "native MT-009 consumer must read the canonical sibling Handshake_Artifacts directory"
    );
    let artifact = artifact_root
        .join("handshake-test")
        .join("wp1-final-audit")
        .join("mt009_mixed_model_lane_diagnostics_projection.json");
    let artifact_bytes =
        std::fs::read(&artifact).expect("backend-generated MT-009 projection is required");
    let provenance: DiagnosticsProjectionProvenance = serde_json::from_slice(
        &std::fs::read(artifact.with_extension("provenance.json"))
            .expect("backend-generated MT-009 provenance is required"),
    )
    .expect("MT-009 diagnostics provenance is typed JSON");
    let now_unix_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock must be after Unix epoch")
        .as_millis() as u64;
    validate_projection_provenance(
        &provenance,
        &artifact_bytes,
        "hsk.mt009_diagnostics_projection_provenance@1",
        &std::env::var("HANDSHAKE_MT009_DIAGNOSTICS_PROOF_NONCE")
            .expect("native MT-009 proof requires the producer nonce"),
        "mixed_local_cloud_subagent_run_persists_restarts_replays_and_projects",
        now_unix_ms,
    )
    .expect("MT-009 artifact must match its current full backend producer provenance");
    let raw_projection: Value = serde_json::from_slice(&artifact_bytes)
        .expect("backend-generated MT-009 projection is JSON");
    let projection = decode_projection_for_native_surface(raw_projection)
        .expect("MT-009 backend artifact must carry valid server-authorized scope and exact native wire shape");
    assert_eq!(provenance.projection_schema_id, projection.schema_id);
    projection
}

pub fn ok_app() -> HandshakeApp {
    app_with_projection(fixture_projection())
}

pub fn app_with_projection(projection: SwarmLaneDiagnosticsProjection) -> HandshakeApp {
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }));
    app.set_swarm_lane_diagnostics_projection_for_test(projection);
    app
}

pub fn shell_harness() -> Harness<'static, HandshakeApp> {
    Harness::builder().build_state(|ctx, a: &mut HandshakeApp| a.ui(ctx), ok_app())
}

pub fn shell_harness_with_projection(
    projection: SwarmLaneDiagnosticsProjection,
) -> Harness<'static, HandshakeApp> {
    Harness::builder().build_state(
        |ctx, a: &mut HandshakeApp| a.ui(ctx),
        app_with_projection(projection),
    )
}

pub fn live_author_ids(harness: &Harness<'_, HandshakeApp>) -> Vec<String> {
    raw_live_author_ids(harness)
        .into_iter()
        .map(|author_id| {
            author_id
                .split_once(".pane.")
                .map(|(logical, _)| logical.to_owned())
                .unwrap_or(author_id)
        })
        .collect()
}

pub fn raw_live_author_ids(harness: &Harness<'_, HandshakeApp>) -> Vec<String> {
    harness
        .root()
        .children_recursive()
        .filter_map(|n| n.accesskit_node().author_id().map(str::to_owned))
        .collect()
}

pub fn assert_unique_swarm_author_ids(harness: &Harness<'_, HandshakeApp>) {
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    for author_id in raw_live_author_ids(harness)
        .into_iter()
        .filter(|id| id.starts_with("swarm-lane-diagnostics."))
    {
        *counts.entry(author_id).or_insert(0) += 1;
    }
    let duplicates = counts
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .collect::<Vec<_>>();
    assert!(
        duplicates.is_empty(),
        "swarm lane diagnostics AccessKit author IDs must be unique: {duplicates:?}"
    );
}

pub fn node_by_author<'a>(
    harness: &'a Harness<'_, HandshakeApp>,
    author_id: &str,
) -> egui_kittest::Node<'a> {
    harness
        .root()
        .children_recursive()
        .find(|n| {
            n.accesskit_node().author_id().is_some_and(|actual| {
                !actual.ends_with("::egui-response")
                    && (actual == author_id || actual.starts_with(&format!("{author_id}.pane.")))
            })
        })
        .unwrap_or_else(|| {
            panic!(
                "{author_id} missing from live tree: {:?}",
                live_author_ids(harness)
            )
        })
}

pub fn accesskit_label(harness: &Harness<'_, HandshakeApp>, author_id: &str) -> String {
    node_by_author(harness, author_id)
        .accesskit_node()
        .label()
        .unwrap_or_default()
        .to_owned()
}

#[derive(Default)]
pub struct SequencedDiagnosticsTransport {
    results: Mutex<VecDeque<Result<SwarmLaneDiagnosticsProjection, String>>>,
}

impl SequencedDiagnosticsTransport {
    pub fn new(
        results: impl IntoIterator<Item = Result<SwarmLaneDiagnosticsProjection, String>>,
    ) -> Self {
        Self {
            results: Mutex::new(results.into_iter().collect()),
        }
    }

    fn deliver(&self, cell: handshake_native::swarm_lane_diagnostics::SwarmLaneDiagnosticsCell) {
        let result = self
            .results
            .lock()
            .expect("lock diagnostics transport sequence")
            .pop_front()
            .expect("diagnostics transport result available");
        *cell.lock().expect("lock diagnostics delivery cell") = Some(result);
    }
}

impl SwarmLaneDiagnosticsTransport for SequencedDiagnosticsTransport {
    fn fetch_latest(
        &self,
        cell: handshake_native::swarm_lane_diagnostics::SwarmLaneDiagnosticsCell,
    ) {
        self.deliver(cell);
    }

    fn fetch_run(
        &self,
        _run_id: &str,
        cell: handshake_native::swarm_lane_diagnostics::SwarmLaneDiagnosticsCell,
    ) {
        self.deliver(cell);
    }
}

pub fn routing_stage_fixture(
    execution_id: &str,
    stage_id: &str,
    state: &str,
    attempt: u32,
    dependencies: &[&str],
    outbox_status: &str,
    active_lease: bool,
    expired_lease: bool,
    awaiting_authority: bool,
) -> SwarmLaneRoutingStageDiagnostics {
    let event_ledger_seq = 100 + i64::from(attempt);
    let lease_expires_at_unix_ms = active_lease.then_some(if expired_lease { 1 } else { u64::MAX });
    let output = (state == "succeeded").then(|| {
        (
            format!("artifact://routing/{execution_id}/{stage_id}"),
            format!("message://routing/{execution_id}/{stage_id}"),
            "ab".repeat(32),
        )
    });
    let command_id = format!("routing-command:{execution_id}:{stage_id}:{attempt}");
    SwarmLaneRoutingStageDiagnostics {
        execution_id: execution_id.into(),
        stage_id: stage_id.into(),
        state: state.into(),
        attempt,
        dispatch_target: if awaiting_authority {
            "validator"
        } else {
            "local_model"
        }
        .into(),
        dependency_stage_ids: dependencies.iter().map(|value| (*value).into()).collect(),
        expected_run_id: "run-mt008-ui".into(),
        expected_lane_id: format!("lane-{stage_id}"),
        expected_model_id: if awaiting_authority {
            String::new()
        } else {
            "model://mt017/local".into()
        },
        expected_provider: None,
        instance_id: active_lease.then(|| format!("instance-{stage_id}")),
        lane_id: Some(format!("lane-{stage_id}")),
        input_refs: vec!["model-lane-message://msg-mt008-001".into()],
        output_ref: output.as_ref().map(|value| value.0.clone()),
        output_message_ref: output.as_ref().map(|value| value.1.clone()),
        authority_request_message_ref: awaiting_authority
            .then(|| format!("message://authority-request/{execution_id}/{stage_id}")),
        output_sha256: output.as_ref().map(|value| value.2.clone()),
        authority_ref: awaiting_authority.then(|| "validator://mt017/authority".into()),
        lease_owner: active_lease.then(|| "routing-executor:mt017".into()),
        fencing_token: active_lease.then(|| format!("fence-{execution_id}-{stage_id}-{attempt}")),
        lease_expires_at_unix_ms,
        lease_expired: expired_lease,
        detail: expired_lease.then(|| "expired lease is recoverable with a fenced retry".into()),
        event_ledger_event_id: format!("evt-{execution_id}-{stage_id}-{attempt}"),
        event_ledger_seq,
        updated_at_unix_ms: 1_752_000_000_000,
        outbox: SwarmLaneRoutingOutboxDiagnostics {
            command_id,
            status: outbox_status.into(),
            fencing_token: active_lease
                .then(|| format!("fence-{execution_id}-{stage_id}-{attempt}")),
            lease_owner: active_lease.then(|| "routing-executor:mt017".into()),
            lease_expires_at_unix_ms,
            event_ledger_event_id: format!("evt-outbox-{execution_id}-{stage_id}-{attempt}"),
            event_ledger_seq: event_ledger_seq + 1,
            created_at_unix_ms: 1_751_999_999_000,
            updated_at_unix_ms: 1_752_000_000_000,
        },
    }
}

pub fn routing_execution_fixture(
    execution_id: &str,
    status: &str,
    failure_reason: Option<&str>,
    cancel_reason: Option<&str>,
    stages: Vec<SwarmLaneRoutingStageDiagnostics>,
) -> SwarmLaneRoutingExecutionDiagnostics {
    SwarmLaneRoutingExecutionDiagnostics {
        execution_id: execution_id.into(),
        run_id: "run-mt008-ui".into(),
        selecting_decision_id: format!("decision-{execution_id}"),
        selecting_decision_event_id: format!("evt-decision-{execution_id}"),
        selecting_decision_event_seq: 80,
        trace_id: "trace-run-mt008-ui".into(),
        run_span_id: "span-run-mt008-ui".into(),
        coordinator_session_id: "coordinator-run-mt008-ui".into(),
        locus_ref: "locus://wp1/mt008/run-mt008-ui".into(),
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: Some("MT-008".into()),
        task_board_id: "task-board://wp-1".into(),
        owner_session: "KERNEL_BUILDER-20260630-045713".into(),
        canonical_graph_sha256: "cd".repeat(32),
        canonical_launch_plan_sha256: "ef".repeat(32),
        cloud_consent_receipt_ref: None,
        validator_authority_ref: (status == "awaiting_authority")
            .then(|| "validator://mt017/authority".into()),
        operator_authority_ref: None,
        initial_input_ref: Some("model-lane-message://msg-mt008-001".into()),
        initial_input_sha256: Some("12".repeat(32)),
        status: status.into(),
        failure_reason: failure_reason.map(str::to_owned),
        cancel_reason: cancel_reason.map(str::to_owned),
        revision: 3,
        stages,
        event_ledger_event_id: format!("evt-execution-{execution_id}"),
        event_ledger_seq: 90,
    }
}

pub fn routing_lifecycle_fixture() -> Vec<SwarmLaneRoutingExecutionDiagnostics> {
    let awaiting_id = "execution-mt017-awaiting";
    let succeeded = routing_stage_fixture(
        awaiting_id,
        "validation-candidate",
        "succeeded",
        1,
        &[],
        "acked",
        false,
        false,
        false,
    );
    let awaiting = routing_stage_fixture(
        awaiting_id,
        "validator-verdict",
        "awaiting_authority",
        1,
        &["validation-candidate"],
        "claimed",
        true,
        false,
        true,
    );
    let recovery_id = "execution-mt017-expired-recovery";
    let recovery = routing_stage_fixture(
        recovery_id,
        "local-attempt",
        "claimed",
        2,
        &[],
        "claimed",
        true,
        true,
        false,
    );
    let failed_id = "execution-mt017-failed";
    let failed = routing_stage_fixture(
        failed_id,
        "local-attempt",
        "failed",
        1,
        &[],
        "acked",
        false,
        false,
        false,
    );
    let cancelled_id = "execution-mt017-cancelled";
    let cancelled = routing_stage_fixture(
        cancelled_id,
        "local-attempt",
        "cancelled",
        1,
        &[],
        "cancelled",
        false,
        false,
        false,
    );
    vec![
        routing_execution_fixture(
            awaiting_id,
            "awaiting_authority",
            None,
            None,
            vec![succeeded, awaiting],
        ),
        routing_execution_fixture(recovery_id, "running", None, None, vec![recovery]),
        routing_execution_fixture(
            failed_id,
            "failed",
            Some("model runtime failed"),
            None,
            vec![failed],
        ),
        routing_execution_fixture(
            cancelled_id,
            "cancelled",
            None,
            Some("operator cancelled run"),
            vec![cancelled],
        ),
    ]
}

pub fn fixture_projection() -> SwarmLaneDiagnosticsProjection {
    SwarmLaneDiagnosticsProjection {
        schema_id: "hsk.model_lane_diagnostics_projection@3".into(),
        surface_contract_id: "native_swarm_lane_diagnostics".into(),
        run: SwarmLaneDiagnosticsRun {
            run_id: "run-mt008-ui".into(),
            trace_id: "trace-run-mt008-ui".into(),
            run_span_id: "span-run-mt008-ui".into(),
            coordinator_session_id: "coordinator-run-mt008-ui".into(),
            routing_policy: "mixed_local_cloud_subagent".into(),
            artifact_namespace: "artifact://model-lane/run-mt008-ui".into(),
            projection_plan_ref: None,
            consent_receipt_ref: None,
            work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
            micro_task_id: Some("MT-008".into()),
            task_board_id: Some("task-board://wp-1".into()),
            owner_session: "KERNEL_BUILDER-20260630-045713".into(),
            event_ledger_event_id: "evt_run_mt008_ui".into(),
            event_ledger_seq: 1,
            flight_recorder_correlation_id: "evt_run_mt008_ui".into(),
            context_bundle_id: "ctx-run-mt008-ui".into(),
            memory_pack_ref: "memory-pack://fems/run-mt008-ui".into(),
            memory_pack_hash: "2f5f9f7bb8d38bb4a5a212c05cfb767c8aa97930c2da1b7d5cfa8f7f03f1b2e4"
                .into(),
            locus_ref: Some("locus://wp1/mt008/run-mt008-ui".into()),
            loom_ref: Some("loom://run-mt008-ui".into()),
            fems_ref: Some("fems://run-mt008-ui".into()),
            status: "restartable".into(),
            recovery_hint_ref: Some("usermanual://dexterity/diagnostics".into()),
            selected_model_id: Some("model://mt008/local".into()),
            candidate_model_ids: vec!["model://mt008/local".into()],
            budget_summary_ref: "budget://mt008".into(),
            determinism_mode: "deterministic_replay".into(),
        },
        lanes: vec![SwarmLaneDiagnosticsLane {
            lane_id: "lane-mt008-local".into(),
            kind: "local_model".into(),
            role: "implementer".into(),
            backend: "local".into(),
            status: "running".into(),
            recovery_state: "restartable".into(),
            model_id: Some("model://mt008/local".into()),
            model_display_name: "MT-014 Diagnostics Catalog Model".into(),
            model_stable_anchor: Some("5a".repeat(32)),
            model_anchor_unavailable_reason: None,
            session_id: "session-lane-mt008-local".into(),
            model_session_id: "model-session-lane-mt008-local".into(),
            adapter_id: "local-runtime".into(),
            provider_kind: "local_runtime".into(),
            runtime_binding: "local".into(),
            launch_authority: "model_runtime".into(),
            capability_token_ids: vec!["capability://mt008/read".into()],
            effective_capability_snapshot_ref: Some("capability-snapshot://mt008".into()),
            capability_negotiation_ref: Some("capability-negotiation://mt008".into()),
            provider_feature_profile_ref: Some("provider-feature-profile://mt008".into()),
            requested_execution_policy_ref: Some("execution-policy://requested/mt008".into()),
            effective_execution_policy_ref: Some("execution-policy://effective/mt008".into()),
            projection_plan_ref: None,
            consent_receipt_ref: None,
            tool_gate_decision_refs: vec!["toolgate://mt008/allow".into()],
            trace_id: "trace-run-mt008-ui".into(),
            lane_span_id: "span-lane-mt008-local".into(),
            event_ledger_event_id: "evt_lane_mt008_ui".into(),
            event_ledger_seq: 2,
            flight_recorder_correlation_id: "evt_lane_mt008_ui".into(),
            last_activity_utc: Some("2026-06-30T00:00:00Z".into()),
            message_count: 1,
            payload_error_count: 0,
            orphan_state: "none".into(),
            cancellation_ref: Some("cancel-token://mt008/lane-mt008-local".into()),
            reclaim_policy_ref: Some("reclaim-policy://mt008".into()),
            terminal_status_mapping_ref: Some("terminal-status://mt008".into()),
            process_ownership_ref: Some("process-ledger://mt008/lane-mt008-local".into()),
            no_os_process_reason_ref: None,
            last_runtime_status_ref: Some("runtime-status://mt008/running".into()),
            last_recovery_event_ref: Some("recovery://mt008/running".into()),
            failstate_code: None,
            startup_failure_ref: None,
            reason_ref: None,
            recovery_hint_ref: Some("usermanual://dexterity/diagnostics#lane".into()),
            work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
            micro_task_id: Some("MT-008".into()),
            task_board_id: Some("task-board://wp-1".into()),
            owner_session: "KERNEL_BUILDER-20260630-045713".into(),
            locus_ref: Some("locus://wp1/mt008/run-mt008-ui/session-lane-mt008-local".into()),
        }],
        messages: vec![SwarmLaneDiagnosticsMessage {
            message_id: "msg-mt008-001".into(),
            from_lane_id: "lane-mt008-local".into(),
            to_lane: "coordinator".into(),
            routing_target_role: Some("coordinator".into()),
            routing_target_session: Some("coordinator-run-mt008-ui".into()),
            routing_correlation_id: Some("corr-run-mt008-ui-msg-mt008-001".into()),
            routing_requires_ack: true,
            routing_ack_for: None,
            kind: "proposal".into(),
            authority: "promotion_candidate".into(),
            promotion_state: "decision_recorded".into(),
            payload_ref: "artifact://model-lane/messages/msg-mt008-001".into(),
            payload_sha256: "ea3f3f4f1dfefde7fd04790cc36dd02d850154a10aa199eb48097a35714f29f0"
                .into(),
            artifact_ref: Some("artifact://promoted/msg-mt008-001".into()),
            promotion_decision_id: Some("promotion://msg-mt008-001".into()),
            promotion_gate_ref: Some("promotion-gate://msg-mt008-001".into()),
            promotion_receipt_ref: Some("promotion-receipt://msg-mt008-001".into()),
            validator_verdict_ref: None,
            operator_decision_ref: None,
            promoted_artifact_sha256: Some(
                "2f5f9f7bb8d38bb4a5a212c05cfb767c8aa97930c2da1b7d5cfa8f7f03f1b2e4".into(),
            ),
            promoted_artifact_version: Some("1".into()),
            tool_gate_decision_refs: vec!["toolgate://mt008/allow".into()],
            coordinator_session_id: "coordinator-run-mt008-ui".into(),
            work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
            micro_task_id: Some("MT-008".into()),
            task_board_id: Some("task-board://wp-1".into()),
            owner_session: "KERNEL_BUILDER-20260630-045713".into(),
            trace_id: "trace-run-mt008-ui".into(),
            message_span_id: "span-msg-mt008-001".into(),
            parent_span_id: Some("span-lane-mt008-local".into()),
            linked_span_contexts: vec!["trace-link://run-mt008-ui/lane-mt008-local".into()],
            event_ledger_event_id: "evt_msg_mt008_ui".into(),
            event_ledger_seq: 3,
            flight_recorder_correlation_id: "evt_msg_mt008_ui".into(),
            locus_ref: Some("locus://wp1/mt008/run-mt008-ui/lane".into()),
            loom_ref: Some("loom://run-mt008-ui/msg-mt008-001".into()),
            fems_ref: Some("fems://run-mt008-ui/msg-mt008-001".into()),
            proposal_ref: Some("proposal://mt008/msg-mt008-001".into()),
            crdt_update_ref: Some("crdt-update://mt008/msg-mt008-001".into()),
            crdt_base_snapshot_ref: Some("crdt-snapshot://mt008/base".into()),
            crdt_state_vector: Some("sv:mt008:1".into()),
            crdt_proposal_ref: Some("crdt-proposal://mt008/msg-mt008-001".into()),
            crdt_stale_base_ref: None,
            payload_error: None,
            reason_ref: None,
            recovery_hint_ref: Some("usermanual://dexterity/diagnostics#message".into()),
            created_at_utc: "2026-06-30T00:00:00Z".into(),
        }],
        diagnostic_tiers: vec![
            SwarmLaneDiagnosticsTier {
                tier: "flight_recorder".into(),
                state: "wired".into(),
                reason: "MT-008 diagnostics projection emits EventLedger evidence".into(),
                evidence_ref: "eventledger://kernel/model-lane/diagnostics".into(),
                follow_up_ref: None,
            },
            SwarmLaneDiagnosticsTier {
                tier: "internal_diagnostics".into(),
                state: "wired".into(),
                reason: "MT-008 backend projection validates internal diagnostics rows".into(),
                evidence_ref: "hbr-int-009://dexterity/diagnostics".into(),
                follow_up_ref: None,
            },
            SwarmLaneDiagnosticsTier {
                tier: "palmistry".into(),
                state: "deferred_with_reason".into(),
                reason: "Palmistry watcher integration is tracked by a follow-up worktree".into(),
                evidence_ref: "palmistry://external-worktree/in-progress".into(),
                follow_up_ref: Some("palmistry://follow-up".into()),
            },
        ],
        mt_runtime_statuses: vec![SwarmLaneDiagnosticsMtStatus {
            micro_task_id: "MT-008".into(),
            status: "ready_for_validation".into(),
            proof_status_ref: Some("proof://mt008/native-argus".into()),
            hbr_status_ref: Some("hbr-int-009://dexterity/diagnostics".into()),
            event_ledger_event_id: "evt_mt_status_mt008_ui".into(),
            event_ledger_seq: 4,
        }],
        routing_executions: routing_lifecycle_fixture(),
        active_lease_count: 1,
        reclaimable_lease_ids: vec![],
        orphan_state: "none".into(),
        resource_scope: Some(SwarmLaneDiagnosticsResourceScope {
            owner_account_fingerprint: "a".repeat(64),
            actor_principal_fingerprint: "b".repeat(64),
            authenticated_session_fingerprint: "c".repeat(64),
            access_space_fingerprint: "d".repeat(64),
            workspace_fingerprint: "e".repeat(64),
            visibility: "private_exact_scope_only".into(),
            denial_posture: "foreign_scope_is_absent_restricted_metadata_withheld".into(),
        }),
    }
}


pub fn validate_joint_projection_pair(
    artifact_bytes: &[u8],
    provenance_bytes: &[u8],
    expected_nonce: &str,
    expected_run_id: &str,
    expected_scope: &SwarmLaneDiagnosticsResourceScope,
    now_unix_ms: u64,
) -> Result<SwarmLaneDiagnosticsProjection, String> {
    let receipt: Value = serde_json::from_slice(provenance_bytes)
        .map_err(|error| format!("joint provenance JSON: {error}"))?;
    let fields = receipt.as_object().ok_or("joint provenance must be an object")?;
    let required = [
        "schema_id", "proof_nonce", "projection_schema_id", "artifact_sha256",
        "producer_target", "producer_test_id", "producer_status",
        "producer_completed_at_unix_ms", "run_id", "resource_scope",
    ];
    if fields.len() != required.len() || required.iter().any(|key| !fields.contains_key(*key)) {
        return Err("joint provenance field shape mismatch".into());
    }
    for (key, expected) in [
        ("schema_id", "hsk.mt017_native_joint_diagnostics_provenance@1"),
        ("proof_nonce", expected_nonce),
        ("projection_schema_id", "hsk.model_lane_diagnostics_projection@3"),
        ("producer_target", "test_swarm_lane_diagnostics_argus"),
        ("producer_test_id", "native_consumes_backend_generated_schema_v3_projection_artifact"),
        ("producer_status", "passed_joint_backend_native_assertions_and_cleanup"),
        ("run_id", expected_run_id),
    ] {
        if receipt[key].as_str() != Some(expected) {
            return Err(format!("joint provenance {key} mismatch"));
        }
    }
    if expected_nonce.is_empty() || expected_run_id.is_empty() {
        return Err("joint provenance expected nonce/run must be bound".into());
    }
    let completed = receipt["producer_completed_at_unix_ms"].as_u64()
        .ok_or("joint provenance completion must be u64")?;
    if completed > now_unix_ms.saturating_add(30_000) {
        return Err("joint provenance completion is from the future".into());
    }
    if now_unix_ms.saturating_sub(completed) > 30 * 60 * 1_000 {
        return Err("joint provenance completion is stale".into());
    }
    let sha = format!("{:x}", Sha256::digest(artifact_bytes));
    if receipt["artifact_sha256"].as_str() != Some(sha.as_str()) {
        return Err("joint provenance artifact_sha256 mismatch".into());
    }
    let expected_scope = serde_json::to_value(expected_scope)
        .map_err(|error| format!("joint expected scope serialization: {error}"))?;
    if receipt["resource_scope"] != expected_scope {
        return Err("joint provenance resource_scope mismatch".into());
    }
    let raw: Value = serde_json::from_slice(artifact_bytes)
        .map_err(|error| format!("joint projection JSON: {error}"))?;
    let projection = decode_projection_for_native_surface(raw)?;
    if projection.run.run_id != expected_run_id {
        return Err("joint projection run_id mismatch".into());
    }
    let actual_scope = serde_json::to_value(projection.resource_scope.as_ref()
        .ok_or("joint projection resource_scope missing")?)
        .map_err(|error| format!("joint projection scope serialization: {error}"))?;
    if actual_scope != expected_scope {
        return Err("joint projection resource_scope mismatch".into());
    }
    Ok(projection)
}

pub fn consume_joint_projection_pair(
    artifact_owner: &std::path::Path,
    expected_nonce: &str,
    expected_run_id: &str,
    expected_scope: &SwarmLaneDiagnosticsResourceScope,
) -> Result<SwarmLaneDiagnosticsProjection, String> {
    let manifest_dir = std::fs::canonicalize(env!("CARGO_MANIFEST_DIR"))
        .map_err(|error| format!("joint manifest directory: {error}"))?;
    let worktree_root = manifest_dir.parent().and_then(std::path::Path::parent)
        .and_then(std::path::Path::parent).ok_or("joint native worktree root missing")?;
    let canonical_owner = std::fs::canonicalize(
        worktree_root.parent().ok_or("joint worktree parent missing")?
            .join("Handshake_Artifacts").join("handshake-test").join("wp1-final-audit"),
    ).map_err(|error| format!("joint canonical artifact owner: {error}"))?;
    let supplied_owner = std::fs::canonicalize(artifact_owner)
        .map_err(|error| format!("joint supplied artifact owner: {error}"))?;
    if supplied_owner != canonical_owner {
        return Err("joint artifact owner is not the admitted canonical directory".into());
    }
    let bytes = std::fs::read(canonical_owner.join("mt017_native_joint_diagnostics_projection.json"))
        .map_err(|error| format!("joint projection read: {error}"))?;
    let receipt = std::fs::read(
        canonical_owner.join("mt017_native_joint_diagnostics_projection.provenance.json"),
    ).map_err(|error| format!("joint provenance read: {error}"))?;
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| format!("joint system clock: {error}"))?.as_millis();
    let now = u64::try_from(now).map_err(|_| "joint system clock does not fit u64")?;
    validate_joint_projection_pair(
        &bytes, &receipt, expected_nonce, expected_run_id, expected_scope, now,
    )
}

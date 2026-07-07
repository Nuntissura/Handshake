//! WP-1 MT-008 — native Swarm lane diagnostics live UI proof.

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;
use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;
use handshake_native::pane_registry::PaneType;
use handshake_native::swarm_lane_diagnostics::{
    lane_author_id, message_author_id, message_payload_author_id, message_promotion_author_id,
    mt_status_author_id, run_author_id, selected_message_author_id,
    validate_projection_for_native_surface, visible_message_ids_for_filters,
    SwarmLaneDiagnosticsLane, SwarmLaneDiagnosticsMessage, SwarmLaneDiagnosticsMtStatus,
    SwarmLaneDiagnosticsProjection, SwarmLaneDiagnosticsRun, SwarmLaneDiagnosticsTier,
    LANE_FILTER_AUTHOR_ID, MESSAGE_FILTER_AUTHOR_ID, RUN_FILTER_AUTHOR_ID, SURFACE_AUTHOR_ID,
};

fn ok_app() -> HandshakeApp {
    app_with_projection(fixture_projection())
}

fn app_with_projection(projection: SwarmLaneDiagnosticsProjection) -> HandshakeApp {
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }));
    app.set_swarm_lane_diagnostics_projection_for_test(projection);
    app
}

fn shell_harness() -> Harness<'static, HandshakeApp> {
    Harness::builder().build_state(|ctx, a: &mut HandshakeApp| a.ui(ctx), ok_app())
}

fn shell_harness_with_projection(
    projection: SwarmLaneDiagnosticsProjection,
) -> Harness<'static, HandshakeApp> {
    Harness::builder().build_state(
        |ctx, a: &mut HandshakeApp| a.ui(ctx),
        app_with_projection(projection),
    )
}

fn live_author_ids(harness: &Harness<'_, HandshakeApp>) -> Vec<String> {
    harness
        .root()
        .children_recursive()
        .filter_map(|n| n.accesskit_node().author_id().map(str::to_owned))
        .collect()
}

fn assert_unique_swarm_author_ids(harness: &Harness<'_, HandshakeApp>) {
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    for author_id in live_author_ids(harness)
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

fn node_by_author<'a>(
    harness: &'a Harness<'_, HandshakeApp>,
    author_id: &str,
) -> egui_kittest::Node<'a> {
    harness
        .root()
        .children_recursive()
        .find(|n| n.accesskit_node().author_id() == Some(author_id))
        .unwrap_or_else(|| {
            panic!(
                "{author_id} missing from live tree: {:?}",
                live_author_ids(harness)
            )
        })
}

#[test]
fn swarm_lane_diagnostics_argus_lists_filters_and_drills_down() {
    let mut harness = shell_harness();
    harness.run();

    harness.get_by_label("RUN").click();
    harness.run();
    harness.get_by_label("Open Lane Diagnostics").click();
    harness.run();
    harness.run();

    assert!(
        harness.state().tab_bar_states().values().any(|bar| bar
            .tabs
            .iter()
            .any(|tab| tab.pane_type == PaneType::SwarmLaneDiagnostics)),
        "Run menu opened a native SwarmLaneDiagnostics tab"
    );

    for expected in [
        SURFACE_AUTHOR_ID,
        RUN_FILTER_AUTHOR_ID,
        LANE_FILTER_AUTHOR_ID,
        MESSAGE_FILTER_AUTHOR_ID,
        &run_author_id("run-mt008-ui"),
        &lane_author_id("lane-mt008-local"),
        &message_author_id("msg-mt008-001"),
        &message_payload_author_id("msg-mt008-001"),
        &message_promotion_author_id("msg-mt008-001"),
        &mt_status_author_id("MT-008"),
    ] {
        assert!(
            live_author_ids(&harness).iter().any(|id| id == expected),
            "{expected} present in live AccessKit tree"
        );
    }
    assert_unique_swarm_author_ids(&harness);

    node_by_author(&harness, LANE_FILTER_AUTHOR_ID).focus();
    harness.run();
    node_by_author(&harness, LANE_FILTER_AUTHOR_ID).type_text("local");
    harness.run();
    assert!(
        live_author_ids(&harness)
            .iter()
            .any(|id| id == &lane_author_id("lane-mt008-local")),
        "lane filter keeps matching lane"
    );

    node_by_author(&harness, MESSAGE_FILTER_AUTHOR_ID).focus();
    harness.run();
    node_by_author(&harness, MESSAGE_FILTER_AUTHOR_ID).type_text("001");
    harness.run();
    assert!(
        live_author_ids(&harness)
            .iter()
            .any(|id| id == &message_author_id("msg-mt008-001")),
        "message filter keeps matching message"
    );

    node_by_author(&harness, &message_payload_author_id("msg-mt008-001")).click_accesskit();
    harness.run();
    let authors = live_author_ids(&harness);
    assert!(
        authors
            .iter()
            .any(|id| id == &selected_message_author_id("msg-mt008-001")),
        "payload drilldown exposes selected message details: {authors:?}"
    );

    node_by_author(&harness, &message_promotion_author_id("msg-mt008-001")).click_accesskit();
    harness.run();
    assert!(
        live_author_ids(&harness)
            .iter()
            .any(|id| id == &selected_message_author_id("msg-mt008-001")),
        "promotion drilldown reuses selected message details"
    );
}

#[test]
fn swarm_lane_diagnostics_argus_rejects_missing_author_id_and_count_mismatch() {
    let mut projection = fixture_projection();
    projection.lanes[0].lane_id.clear();
    let err = validate_projection_for_native_surface(&projection)
        .expect_err("empty lane id would produce an unusable row author_id");
    assert!(err.contains("lane author_id"), "got {err}");

    let mut projection = fixture_projection();
    projection.lanes[0].message_count = 99;
    let err = validate_projection_for_native_surface(&projection)
        .expect_err("canonical lane count mismatch must fail validation");
    assert!(err.contains("message_count mismatch"), "got {err}");

    let mut projection = fixture_projection();
    projection.messages[0].from_lane_id = "lane-mt008-missing".into();
    let err = validate_projection_for_native_surface(&projection)
        .expect_err("messages must reference an existing lane");
    assert!(err.contains("unknown lane"), "got {err}");

    let mut projection = fixture_projection();
    projection.messages[0].to_lane = "lane:lane-mt008-missing".into();
    let err = validate_projection_for_native_surface(&projection)
        .expect_err("destination lane refs must reference an existing lane");
    assert!(err.contains("unknown to_lane"), "got {err}");

    let mut projection = fixture_projection();
    projection.messages[0].to_lane = "not-a-target".into();
    let err = validate_projection_for_native_surface(&projection)
        .expect_err("unsupported destination lane target must fail validation");
    assert!(err.contains("routing target unsupported"), "got {err}");

    let mut projection = fixture_projection();
    projection.messages[0].message_id.clear();
    let err = validate_projection_for_native_surface(&projection)
        .expect_err("empty message id would produce unusable message author_ids");
    assert!(err.contains("message author_id"), "got {err}");

    let mut projection = fixture_projection();
    projection
        .mt_runtime_statuses
        .push(projection.mt_runtime_statuses[0].clone());
    projection.mt_runtime_statuses[1].micro_task_id = "MT 008".into();
    let err = validate_projection_for_native_surface(&projection)
        .expect_err("lossy AccessKit token collisions must fail validation");
    assert!(err.contains("duplicate AccessKit author_id"), "got {err}");

    let mut projection = fixture_projection();
    projection.schema_id = "hsk.wrong_projection@1".into();
    let err = validate_projection_for_native_surface(&projection)
        .expect_err("wrong backend schema id must fail before rendering");
    assert!(err.contains("schema_id mismatch"), "got {err}");

    let mut projection = fixture_projection();
    projection
        .diagnostic_tiers
        .retain(|tier| tier.tier != "internal_diagnostics");
    let err = validate_projection_for_native_surface(&projection)
        .expect_err("FlightRecorder-only diagnostics posture must fail");
    assert!(err.contains("internal_diagnostics"), "got {err}");

    let mut projection = fixture_projection();
    projection.diagnostic_tiers[0].state = "missing".into();
    let err = validate_projection_for_native_surface(&projection)
        .expect_err("missing HBR tier state must fail like backend posture validation");
    assert!(err.contains("state cannot be missing"), "got {err}");

    let mut projection = fixture_projection();
    projection.diagnostic_tiers[1].state = "deferred_with_reason".into();
    projection.diagnostic_tiers[1].reason =
        "WP-KERNEL-012 internal diagnostics integration is not shipped in this worktree".into();
    projection.diagnostic_tiers[1].follow_up_ref =
        Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1/MT-011".into());
    projection.diagnostic_tiers[2].state = "not_applicable_with_reason".into();
    projection.diagnostic_tiers[2].reason =
        "Palmistry is not applicable to this settings-only behavior".into();
    projection.diagnostic_tiers[2].follow_up_ref = None;
    validate_projection_for_native_surface(&projection)
        .expect("HBR-INT-009 accepts explicit deferred and not-applicable reason states");

    let mut projection = fixture_projection();
    projection.diagnostic_tiers[2].state = "not_applicable_with_reason".into();
    projection.diagnostic_tiers[2].reason.clear();
    let err = validate_projection_for_native_surface(&projection)
        .expect_err("not-applicable HBR tier requires a visible reason");
    assert!(err.contains("reason missing"), "got {err}");

    let mut projection = fixture_projection();
    projection.diagnostic_tiers[2].follow_up_ref = None;
    let err = validate_projection_for_native_surface(&projection)
        .expect_err("deferred HBR tier requires a follow-up ref");
    assert!(err.contains("follow_up_ref"), "got {err}");

    let mut projection = fixture_projection();
    projection.mt_runtime_statuses.clear();
    let err = validate_projection_for_native_surface(&projection)
        .expect_err("MT status signaling must be present");
    assert!(err.contains("MT runtime status rows missing"), "got {err}");

    let mut projection = fixture_projection();
    projection.mt_runtime_statuses[0].micro_task_id = "MT-OTHER".into();
    let err = validate_projection_for_native_surface(&projection)
        .expect_err("MT status must cover the run micro_task_id");
    assert!(
        err.contains("run micro_task_id MT-008 missing"),
        "got {err}"
    );

    let mut projection = fixture_projection();
    projection.mt_runtime_statuses[0].proof_status_ref = None;
    let err = validate_projection_for_native_surface(&projection)
        .expect_err("MT status proof refs must be visible");
    assert!(err.contains("status/proof/HBR/EventLedger"), "got {err}");

    let mut projection = fixture_projection();
    projection.mt_runtime_statuses[0].event_ledger_seq = 0;
    let err = validate_projection_for_native_surface(&projection)
        .expect_err("MT status EventLedger sequence must be positive");
    assert!(err.contains("status/proof/HBR/EventLedger"), "got {err}");
}

#[test]
fn mixed_model_lane_run_is_inspectable_through_argus() {
    let projection = mixed_fixture_projection();
    validate_projection_for_native_surface(&projection)
        .expect("MT-009 mixed projection satisfies native Argus contract");
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

    let mut harness = shell_harness_with_projection(projection.clone());
    harness.run();
    harness.get_by_label("RUN").click();
    harness.run();
    harness.get_by_label("Open Lane Diagnostics").click();
    harness.run();
    harness.run();

    let expected_authors = vec![
        SURFACE_AUTHOR_ID.to_string(),
        RUN_FILTER_AUTHOR_ID.to_string(),
        LANE_FILTER_AUTHOR_ID.to_string(),
        MESSAGE_FILTER_AUTHOR_ID.to_string(),
        run_author_id("run-mt009-mixed"),
        lane_author_id("lane-mt009-local"),
        lane_author_id("lane-mt009-cloud"),
        lane_author_id("lane-mt009-subagent"),
        message_author_id("msg-mt009-local"),
        message_author_id("msg-mt009-cloud"),
        message_author_id("msg-mt009-subagent"),
        message_payload_author_id("msg-mt009-local"),
        message_payload_author_id("msg-mt009-cloud"),
        message_payload_author_id("msg-mt009-subagent"),
        message_promotion_author_id("msg-mt009-local"),
        message_promotion_author_id("msg-mt009-cloud"),
        message_promotion_author_id("msg-mt009-subagent"),
        mt_status_author_id("MT-009"),
    ];
    let authors = live_author_ids(&harness);
    for expected in expected_authors {
        assert!(
            authors.iter().any(|id| id == &expected),
            "{expected} present in live AccessKit tree: {authors:?}"
        );
    }
    assert_unique_swarm_author_ids(&harness);

    node_by_author(&harness, LANE_FILTER_AUTHOR_ID).focus();
    harness.run();
    node_by_author(&harness, LANE_FILTER_AUTHOR_ID).type_text("cloud");
    harness.run();
    let authors = live_author_ids(&harness);
    assert!(
        authors
            .iter()
            .any(|id| id == &lane_author_id("lane-mt009-cloud")),
        "lane filter keeps the cloud lane"
    );
    assert!(
        authors
            .iter()
            .any(|id| id == &message_author_id("msg-mt009-cloud")),
        "lane filter keeps messages from the cloud lane"
    );
    assert!(
        !authors
            .iter()
            .any(|id| id == &message_author_id("msg-mt009-local")),
        "lane filter removes nonmatching local message rows: {authors:?}"
    );
    assert_eq!(
        visible_message_ids_for_filters(&projection, "cloud", ""),
        vec!["msg-mt009-cloud".to_owned()],
        "lane filter scopes rendered message rows to the matching cloud lane"
    );

    let mut harness = shell_harness_with_projection(projection.clone());
    harness.run();
    harness.get_by_label("RUN").click();
    harness.run();
    harness.get_by_label("Open Lane Diagnostics").click();
    harness.run();
    harness.run();
    node_by_author(&harness, &message_payload_author_id("msg-mt009-local")).click_accesskit();
    harness.run();
    assert!(
        live_author_ids(&harness)
            .iter()
            .any(|id| id == &selected_message_author_id("msg-mt009-local")),
        "local message is selected before filtering"
    );
    node_by_author(&harness, LANE_FILTER_AUTHOR_ID).focus();
    harness.run();
    node_by_author(&harness, LANE_FILTER_AUTHOR_ID).type_text("cloud");
    harness.run();
    harness.run();
    let authors = live_author_ids(&harness);
    assert!(
        !authors
            .iter()
            .any(|id| id == &selected_message_author_id("msg-mt009-local")),
        "lane filter clears selected details for filtered-out messages: {authors:?}"
    );

    let mut harness = shell_harness_with_projection(projection);
    harness.run();
    harness.get_by_label("RUN").click();
    harness.run();
    harness.get_by_label("Open Lane Diagnostics").click();
    harness.run();
    harness.run();
    node_by_author(&harness, MESSAGE_FILTER_AUTHOR_ID).focus();
    harness.run();
    node_by_author(&harness, MESSAGE_FILTER_AUTHOR_ID).type_text("subagent");
    harness.run();
    assert!(
        live_author_ids(&harness)
            .iter()
            .any(|id| id == &message_author_id("msg-mt009-subagent")),
        "message filter keeps the subagent message"
    );

    node_by_author(&harness, &message_payload_author_id("msg-mt009-subagent")).click_accesskit();
    harness.run();
    assert!(
        live_author_ids(&harness)
            .iter()
            .any(|id| id == &selected_message_author_id("msg-mt009-subagent")),
        "payload drilldown exposes selected MT-009 message"
    );

    node_by_author(&harness, &message_promotion_author_id("msg-mt009-subagent")).click_accesskit();
    harness.run();
    assert!(
        live_author_ids(&harness)
            .iter()
            .any(|id| id == &selected_message_author_id("msg-mt009-subagent")),
        "promotion drilldown exposes selected MT-009 message"
    );
}

fn fixture_projection() -> SwarmLaneDiagnosticsProjection {
    SwarmLaneDiagnosticsProjection {
        schema_id: "hsk.model_lane_diagnostics_projection@1".into(),
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
        active_lease_count: 1,
        reclaimable_lease_ids: vec![],
        orphan_state: "none".into(),
    }
}

fn mixed_fixture_projection() -> SwarmLaneDiagnosticsProjection {
    let base = fixture_projection();
    SwarmLaneDiagnosticsProjection {
        schema_id: "hsk.model_lane_diagnostics_projection@1".into(),
        surface_contract_id: "native_swarm_lane_diagnostics".into(),
        run: SwarmLaneDiagnosticsRun {
            run_id: "run-mt009-mixed".into(),
            trace_id: "trace-run-mt009-mixed".into(),
            run_span_id: "span-run-mt009-mixed".into(),
            coordinator_session_id: "coordinator-run-mt009-mixed".into(),
            routing_policy: "mixed_local_cloud_subagent".into(),
            artifact_namespace: "artifact://model-lane/mt009/run-mt009-mixed".into(),
            projection_plan_ref: None,
            consent_receipt_ref: None,
            work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
            micro_task_id: Some("MT-009".into()),
            task_board_id: Some("task-board://wp-1".into()),
            owner_session: "KERNEL_BUILDER-MT009".into(),
            event_ledger_event_id: "evt_run_mt009_mixed".into(),
            event_ledger_seq: 1,
            flight_recorder_correlation_id: "evt_run_mt009_mixed".into(),
            context_bundle_id: "ctx-run-mt009-mixed".into(),
            memory_pack_ref: "memory-pack://fems/mt009/run-mt009-mixed".into(),
            memory_pack_hash: "2f5f9f7bb8d38bb4a5a212c05cfb767c8aa97930c2da1b7d5cfa8f7f03f1b2e4"
                .into(),
            locus_ref: Some("locus://wp1/mt009/run-mt009-mixed".into()),
            loom_ref: Some("loom://mt009/run-mt009-mixed".into()),
            fems_ref: Some("fems://mt009/run-mt009-mixed".into()),
            status: "restartable".into(),
            recovery_hint_ref: Some("usermanual://model-lane-validation-harness#recovery".into()),
            selected_model_id: Some("model://mt009/local/tinyllama".into()),
            candidate_model_ids: vec![
                "model://mt009/local/tinyllama".into(),
                "model://mt009/cloud/openai/gpt-4o-mini".into(),
                "subagent://mt009/coder".into(),
            ],
            budget_summary_ref: "budget://mt009/mixed-runtime".into(),
            determinism_mode: "deterministic_replay".into(),
        },
        lanes: vec![
            mixed_lane(
                &base.lanes[0],
                "lane-mt009-local",
                "local_model",
                "local_implementer",
                "local",
                "local_runtime",
                "local",
                "model_runtime",
                "model://mt009/local/tinyllama",
                None,
                None,
                Some("process-ledger://mt009/lane-mt009-local"),
                None,
                2,
            ),
            mixed_lane(
                &base.lanes[0],
                "lane-mt009-cloud",
                "cloud_model",
                "cloud_reviewer",
                "cloud",
                "openai",
                "cloud",
                "cloud_lane",
                "model://mt009/cloud/openai/gpt-4o-mini",
                Some("cloud-projection-plan://run-mt009-mixed/lane-mt009-cloud"),
                Some("cloud-consent-receipt://run-mt009-mixed/lane-mt009-cloud"),
                Some("process-ledger://mt009/lane-mt009-cloud"),
                None,
                3,
            ),
            mixed_lane(
                &base.lanes[0],
                "lane-mt009-subagent",
                "subagent",
                "subagent_coder",
                "subagent",
                "subagent",
                "subagent",
                "subagent_manager",
                "subagent://mt009/coder",
                None,
                None,
                None,
                Some("no-os-process://subagent_manager/lane-mt009-subagent"),
                4,
            ),
        ],
        messages: vec![
            mixed_message(
                &base.messages[0],
                "msg-mt009-local",
                "lane-mt009-local",
                "local",
                "model://mt009/local/tinyllama",
                5,
            ),
            mixed_message(
                &base.messages[0],
                "msg-mt009-cloud",
                "lane-mt009-cloud",
                "cloud",
                "model://mt009/cloud/openai/gpt-4o-mini",
                6,
            ),
            mixed_message(
                &base.messages[0],
                "msg-mt009-subagent",
                "lane-mt009-subagent",
                "subagent",
                "subagent://mt009/coder",
                7,
            ),
        ],
        diagnostic_tiers: vec![
            SwarmLaneDiagnosticsTier {
                tier: "flight_recorder".into(),
                state: "wired".into(),
                reason: "MT-009 mixed runtime emits EventLedger evidence".into(),
                evidence_ref: "eventledger://kernel/model-lane/mt009".into(),
                follow_up_ref: None,
            },
            SwarmLaneDiagnosticsTier {
                tier: "internal_diagnostics".into(),
                state: "wired".into(),
                reason: "MT-009 mixed runtime exposes internal diagnostic rows".into(),
                evidence_ref: "hbr-int-009://dexterity/mixed-runtime".into(),
                follow_up_ref: None,
            },
            SwarmLaneDiagnosticsTier {
                tier: "palmistry".into(),
                state: "deferred_with_reason".into(),
                reason: "Palmistry external watcher is deferred to the watcher worktree".into(),
                evidence_ref: "palmistry://wp1/model-lane/mt009/external-worktree".into(),
                follow_up_ref: Some("palmistry://wp1/model-lane/mt009".into()),
            },
        ],
        mt_runtime_statuses: vec![SwarmLaneDiagnosticsMtStatus {
            micro_task_id: "MT-009".into(),
            status: "ready_for_validation".into(),
            proof_status_ref: Some("proof://mt009/mixed_model_lane_integration_pg_tests".into()),
            hbr_status_ref: Some("hbr-int-009://dexterity/mixed-runtime".into()),
            event_ledger_event_id: "evt_mt_status_mt009_ready".into(),
            event_ledger_seq: 8,
        }],
        active_lease_count: 1,
        reclaimable_lease_ids: vec![],
        orphan_state: "none".into(),
    }
}

fn mixed_lane(
    base: &SwarmLaneDiagnosticsLane,
    lane_id: &str,
    kind: &str,
    role: &str,
    backend: &str,
    provider_kind: &str,
    runtime_binding: &str,
    launch_authority: &str,
    model_id: &str,
    projection_plan_ref: Option<&str>,
    consent_receipt_ref: Option<&str>,
    process_ownership_ref: Option<&str>,
    no_os_process_reason_ref: Option<&str>,
    event_ledger_seq: i64,
) -> SwarmLaneDiagnosticsLane {
    let mut lane = base.clone();
    lane.lane_id = lane_id.into();
    lane.kind = kind.into();
    lane.role = role.into();
    lane.backend = backend.into();
    lane.status = "ready".into();
    lane.recovery_state = "restartable".into();
    lane.model_id = Some(model_id.into());
    lane.session_id = format!("session-{lane_id}");
    lane.model_session_id = format!("model-session-{lane_id}");
    lane.adapter_id = format!("adapter-{lane_id}");
    lane.provider_kind = provider_kind.into();
    lane.runtime_binding = runtime_binding.into();
    lane.launch_authority = launch_authority.into();
    lane.capability_token_ids = vec![format!("capability://mt009/{lane_id}/read")];
    lane.effective_capability_snapshot_ref = Some(format!("capability-snapshot://mt009/{lane_id}"));
    lane.capability_negotiation_ref = Some(format!("capability-negotiation://mt009/{lane_id}"));
    lane.provider_feature_profile_ref = Some(format!("provider-feature-profile://mt009/{lane_id}"));
    lane.requested_execution_policy_ref =
        Some(format!("execution-policy://requested/mt009/{lane_id}"));
    lane.effective_execution_policy_ref =
        Some(format!("execution-policy://effective/mt009/{lane_id}"));
    lane.projection_plan_ref = projection_plan_ref.map(str::to_string);
    lane.consent_receipt_ref = consent_receipt_ref.map(str::to_string);
    lane.tool_gate_decision_refs = vec![format!("toolgate://mt009/{lane_id}/allow")];
    lane.trace_id = "trace-run-mt009-mixed".into();
    lane.lane_span_id = format!("span-{lane_id}");
    lane.event_ledger_event_id = format!("evt_{lane_id}");
    lane.event_ledger_seq = event_ledger_seq;
    lane.flight_recorder_correlation_id = lane.event_ledger_event_id.clone();
    lane.message_count = 1;
    lane.process_ownership_ref = process_ownership_ref.map(str::to_string);
    lane.no_os_process_reason_ref = no_os_process_reason_ref.map(str::to_string);
    lane.last_runtime_status_ref = Some(format!("runtime-status://mt009/{lane_id}/ready"));
    lane.last_recovery_event_ref = Some(format!("recovery://mt009/{lane_id}/startable"));
    lane.recovery_hint_ref = Some("usermanual://model-lane-validation-harness#lane".into());
    lane.work_packet_id = Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into());
    lane.micro_task_id = Some("MT-009".into());
    lane.task_board_id = Some("task-board://wp-1".into());
    lane.owner_session = "KERNEL_BUILDER-MT009".into();
    lane.locus_ref = Some(format!("locus://wp1/mt009/run-mt009-mixed/{lane_id}"));
    lane
}

fn mixed_message(
    base: &SwarmLaneDiagnosticsMessage,
    message_id: &str,
    lane_id: &str,
    lane_label: &str,
    model_ref: &str,
    event_ledger_seq: i64,
) -> SwarmLaneDiagnosticsMessage {
    let mut message = base.clone();
    message.message_id = message_id.into();
    message.from_lane_id = lane_id.into();
    message.to_lane = "coordinator".into();
    message.routing_target_role = Some("coordinator".into());
    message.routing_target_session = Some("coordinator-run-mt009-mixed".into());
    message.routing_correlation_id = Some(format!("corr-run-mt009-mixed-{message_id}"));
    message.routing_requires_ack = true;
    message.kind = "proposal".into();
    message.authority = "promotion_candidate".into();
    message.promotion_state = "decision_recorded".into();
    message.payload_ref = format!("artifact://model-lane/messages/{message_id}");
    message.payload_sha256 =
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".into();
    message.artifact_ref = Some(format!("artifact://promoted/mt009/{message_id}"));
    message.promotion_decision_id = Some(format!("promotion://mt009/{message_id}"));
    message.promotion_gate_ref = Some(format!("promotion-gate://mt009/{message_id}"));
    message.promotion_receipt_ref = Some(format!("promotion-receipt://mt009/{message_id}"));
    message.promoted_artifact_sha256 =
        Some("2f5f9f7bb8d38bb4a5a212c05cfb767c8aa97930c2da1b7d5cfa8f7f03f1b2e4".into());
    message.promoted_artifact_version = Some("1".into());
    message.tool_gate_decision_refs = vec![format!("toolgate://mt009/{lane_id}/allow")];
    message.coordinator_session_id = "coordinator-run-mt009-mixed".into();
    message.work_packet_id = Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into());
    message.micro_task_id = Some("MT-009".into());
    message.task_board_id = Some("task-board://wp-1".into());
    message.owner_session = "KERNEL_BUILDER-MT009".into();
    message.trace_id = "trace-run-mt009-mixed".into();
    message.message_span_id = format!("span-{message_id}");
    message.parent_span_id = Some(format!("span-{lane_id}"));
    message.linked_span_contexts = vec![format!("trace-link://run-mt009-mixed/{lane_id}")];
    message.event_ledger_event_id = format!("evt_{message_id}");
    message.event_ledger_seq = event_ledger_seq;
    message.flight_recorder_correlation_id = message.event_ledger_event_id.clone();
    message.locus_ref = Some(format!(
        "locus://wp1/mt009/run-mt009-mixed/{lane_id}/{message_id}"
    ));
    message.loom_ref = Some(format!("loom://mt009/run-mt009-mixed/{message_id}"));
    message.fems_ref = Some(format!("fems://mt009/run-mt009-mixed/{message_id}"));
    message.proposal_ref = Some(format!("proposal://mt009/{message_id}/{model_ref}"));
    message.crdt_update_ref = Some(format!("crdt-update://mt009/{message_id}"));
    message.crdt_base_snapshot_ref = Some("crdt-snapshot://mt009/base".into());
    message.crdt_state_vector = Some(format!("sv:mt009:{lane_label}"));
    message.crdt_proposal_ref = Some(format!("crdt-proposal://mt009/{message_id}"));
    message.crdt_stale_base_ref = None;
    message.payload_error = None;
    message.reason_ref = None;
    message.recovery_hint_ref = Some("usermanual://model-lane-validation-harness#message".into());
    message.created_at_utc = "2026-07-01T00:00:00Z".into();
    message
}

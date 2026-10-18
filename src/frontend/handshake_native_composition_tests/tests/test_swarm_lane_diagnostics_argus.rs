//! WP-1 MT-008 — native Swarm lane diagnostics live UI proof.

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;
use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;
use handshake_native::pane_registry::PaneType;
use handshake_native::swarm_lane_diagnostics::{
    lane_author_id, message_author_id, message_payload_author_id, message_promotion_author_id,
    mt_status_author_id, routing_execution_author_id, routing_outbox_author_id,
    routing_stage_author_id, run_author_id, selected_message_author_id,
    decode_projection_for_native_surface, validate_projection_for_native_surface,
    visible_message_ids_for_filters,
    SwarmLaneDiagnosticsLane, SwarmLaneDiagnosticsMessage, SwarmLaneDiagnosticsMtStatus,
    SwarmLaneDiagnosticsProjection, SwarmLaneDiagnosticsResourceScope, SwarmLaneDiagnosticsRun,
    SwarmLaneDiagnosticsTier, SwarmLaneDiagnosticsTransport, SwarmLaneRoutingExecutionDiagnostics,
    SwarmLaneRoutingOutboxDiagnostics, SwarmLaneRoutingStageDiagnostics, EMPTY_MESSAGES_AUTHOR_ID,
    ERROR_AUTHOR_ID, FRESHNESS_AUTHOR_ID, LANE_FILTER_AUTHOR_ID, MESSAGE_FILTER_AUTHOR_ID,
    PRIVACY_ACCESS_SPACE_AUTHOR_ID, PRIVACY_DENIAL_AUTHOR_ID, PRIVACY_OWNER_AUTHOR_ID,
    PRIVACY_PRINCIPAL_AUTHOR_ID, PRIVACY_SESSION_AUTHOR_ID, PRIVACY_VISIBILITY_AUTHOR_ID,
    PRIVACY_WORKSPACE_AUTHOR_ID, REFRESH_AUTHOR_ID, RUN_FILTER_AUTHOR_ID, SURFACE_AUTHOR_ID,
};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use handshake_native_composition_tests::fixtures::*;


#[test]
fn swarm_lane_diagnostics_argus_lists_filters_and_drills_down() {
    let mut harness = shell_harness();
    harness.run();

    harness.get_by_label("MODELS").click();
    harness.run();
    harness.get_by_label("Open Lane Diagnostics").click();
    harness.run();

    let authors = live_author_ids(&harness);
    assert!(authors.iter().any(|author_id| {
        author_id
            == &handshake_native::swarm_lane_diagnostics::lane_model_identity_author_id(
                "lane-mt008-local",
            )
    }));
    assert!(authors.iter().any(|author_id| {
        author_id
            == &handshake_native::swarm_lane_diagnostics::lane_model_label_author_id(
                "lane-mt008-local",
            )
    }));
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
        PRIVACY_OWNER_AUTHOR_ID,
        PRIVACY_PRINCIPAL_AUTHOR_ID,
        PRIVACY_SESSION_AUTHOR_ID,
        PRIVACY_ACCESS_SPACE_AUTHOR_ID,
        PRIVACY_WORKSPACE_AUTHOR_ID,
        PRIVACY_VISIBILITY_AUTHOR_ID,
        PRIVACY_DENIAL_AUTHOR_ID,
        &run_author_id("run-mt008-ui"),
        &lane_author_id("lane-mt008-local"),
        &message_author_id("msg-mt008-001"),
        &message_payload_author_id("msg-mt008-001"),
        &message_promotion_author_id("msg-mt008-001"),
        &mt_status_author_id("MT-008"),
        &routing_execution_author_id("execution-mt017-awaiting"),
        &routing_stage_author_id("execution-mt017-awaiting", "validator-verdict", 1),
        &routing_outbox_author_id("routing-command:execution-mt017-awaiting:validator-verdict:1"),
        &routing_execution_author_id("execution-mt017-expired-recovery"),
        &routing_execution_author_id("execution-mt017-failed"),
        &routing_execution_author_id("execution-mt017-cancelled"),
    ] {
        assert!(
            live_author_ids(&harness).iter().any(|id| id == expected),
            "{expected} present in live AccessKit tree"
        );
    }
    assert!(
        accesskit_label(
            &harness,
            &routing_stage_author_id("execution-mt017-awaiting", "validator-verdict", 1,),
        )
        .contains("state awaiting_authority"),
        "awaiting-authority lifecycle is visible"
    );
    assert!(
        accesskit_label(
            &harness,
            &routing_stage_author_id("execution-mt017-expired-recovery", "local-attempt", 2,),
        )
        .contains("expired=true"),
        "expired recoverable lease is visible"
    );
    assert!(
        accesskit_label(
            &harness,
            &routing_outbox_author_id("routing-command:execution-mt017-failed:local-attempt:1",),
        )
        .contains("status acked"),
        "current architecture's durable failed-stage outbox acknowledgement is visible"
    );
    assert!(
        accesskit_label(
            &harness,
            &routing_execution_author_id("execution-mt017-cancelled"),
        )
        .contains("cancel operator cancelled run"),
        "cancellation reason is visible"
    );
    assert_unique_swarm_author_ids(&harness);

    assert!(
        accesskit_label(&harness, PRIVACY_VISIBILITY_AUTHOR_ID)
            .contains("exact account + Principal + session + AccessSpace + workspace"),
        "operator-visible privacy state names every enforced scope dimension"
    );
    assert!(
        accesskit_label(&harness, PRIVACY_DENIAL_AUTHOR_ID)
            .contains("foreign stored scope: not found"),
        "operator-visible privacy state explains the metadata-safe denial posture"
    );
    let rendered_text = harness
        .root()
        .children_recursive()
        .flat_map(|node| {
            let access = node.accesskit_node();
            [
                access.label().unwrap_or_default().to_owned(),
                access.value().unwrap_or_default().to_owned(),
            ]
        })
        .collect::<Vec<_>>()
        .join("\n");
    for raw_scope_value in [
        "01900000-0000-7000-8000-000000000001",
        "01900000-0000-7000-8000-000000000002",
        "01900000-0000-7000-8000-000000000003",
        "01900000-0000-7000-8000-000000000004",
        "workspace-mt008-ui",
    ] {
        assert!(
            !rendered_text.contains(raw_scope_value),
            "raw scope value leaked into operator-visible AccessKit text"
        );
    }

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
fn swarm_lane_diagnostics_argus_exposes_message_filter_empty_state() {
    let mut harness = shell_harness();
    harness.run();
    harness.get_by_label("MODELS").click();
    harness.run();
    harness.get_by_label("Open Lane Diagnostics").click();
    harness.run();

    node_by_author(&harness, MESSAGE_FILTER_AUTHOR_ID).focus();
    harness.run();
    node_by_author(&harness, MESSAGE_FILTER_AUTHOR_ID)
        .type_text("no-message-can-match-this-edge-state");
    harness.run();

    assert!(
        live_author_ids(&harness)
            .iter()
            .any(|id| id == EMPTY_MESSAGES_AUTHOR_ID),
        "empty filtered diagnostics state is explicitly visible to Argus"
    );
    assert!(
        accesskit_label(&harness, EMPTY_MESSAGES_AUTHOR_ID).contains("No messages match"),
        "empty-state landmark carries an operator-readable reason"
    );
    assert!(
        !live_author_ids(&harness)
            .iter()
            .any(|id| id == &message_author_id("msg-mt008-001")),
        "empty-state capture cannot retain a filtered-out message row"
    );
}

#[test]
fn two_swarm_lane_panes_have_unique_pane_scoped_author_ids() {
    let mut harness = shell_harness();
    harness.run();
    for pane_index in 0..2 {
        if pane_index == 1 {
            harness.get_by_label("GO").click();
            harness.run();
            harness.get_by_label("Go to Next Pane").click();
            harness.run();
        }
        harness.get_by_label("MODELS").click();
        harness.run();
        harness.get_by_label("Open Lane Diagnostics").click();
        harness.run();
    }
    let surface_ids = raw_live_author_ids(&harness)
        .into_iter()
        .filter(|id| id.starts_with(&format!("{SURFACE_AUTHOR_ID}.pane.")))
        .collect::<Vec<_>>();
    assert_eq!(surface_ids.len(), 2, "both diagnostics panes are visible");
    assert_ne!(
        surface_ids[0], surface_ids[1],
        "pane scope differentiates surfaces"
    );
    assert_unique_swarm_author_ids(&harness);
}

#[test]
fn known_and_stale_model_identity_labels_are_visible() {
    let mut projection = fixture_projection();
    let known_name = projection.lanes[0].model_display_name.clone();
    let known_anchor = projection.lanes[0]
        .model_stable_anchor
        .clone()
        .expect("known fixture carries a stable anchor");
    let mut stale = projection.lanes[0].clone();
    stale.lane_id = "lane-mt008-stale".into();
    stale.model_id = Some("01900000-0000-7000-8000-000000000099".into());
    stale.model_display_name = "Unknown / stale model registration".into();
    stale.model_stable_anchor = None;
    stale.model_anchor_unavailable_reason =
        Some("legacy model id is absent from the current registry".into());
    stale.message_count = 0;
    projection.lanes.push(stale);

    let mut harness = shell_harness_with_projection(projection);
    harness.run();
    harness.get_by_label("MODELS").click();
    harness.run();
    harness.get_by_label("Open Lane Diagnostics").click();
    harness.run();

    let known = accesskit_label(
        &harness,
        &handshake_native::swarm_lane_diagnostics::lane_model_label_author_id("lane-mt008-local"),
    );
    assert!(
        known.contains(&known_name),
        "known label names the registered model: {known}"
    );
    assert!(
        known.contains(&known_anchor),
        "known label exposes the stable anchor: {known}"
    );
    let stale = accesskit_label(
        &harness,
        &handshake_native::swarm_lane_diagnostics::lane_model_label_author_id("lane-mt008-stale"),
    );
    assert!(
        stale.contains("Unknown / stale model registration"),
        "stale label is explicit: {stale}"
    );
    assert!(
        stale.contains("legacy model id"),
        "stale label explains anchor loss: {stale}"
    );
}

#[test]
fn argus_keeps_last_success_visible_when_refresh_fails() {
    let transport = Arc::new(SequencedDiagnosticsTransport::new([
        Ok(fixture_projection()),
        Err("simulated diagnostics refresh failure".into()),
    ]));
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }));
    app.set_swarm_lane_diagnostics_transport_for_test(transport);
    let mut harness = Harness::builder().build_state(|ctx, a: &mut HandshakeApp| a.ui(ctx), app);
    harness.run();
    harness.get_by_label("MODELS").click();
    harness.run();
    harness.get_by_label("Open Lane Diagnostics").click();
    harness.run();
    harness.run();

    let current = accesskit_label(&harness, FRESHNESS_AUTHOR_ID);
    let last_success = current
        .strip_prefix("CURRENT | last success unix_ms ")
        .expect("successful fetch publishes CURRENT last-success status")
        .to_owned();
    node_by_author(&harness, REFRESH_AUTHOR_ID).click_accesskit();
    harness.run();
    harness.run();

    assert_eq!(
        accesskit_label(&harness, FRESHNESS_AUTHOR_ID),
        format!("STALE | last success unix_ms {last_success}"),
        "refresh failure preserves and marks the last successful projection stale"
    );
    assert_eq!(
        accesskit_label(&harness, ERROR_AUTHOR_ID),
        "simulated diagnostics refresh failure"
    );
    assert!(
        live_author_ids(&harness)
            .iter()
            .any(|id| id == &lane_author_id("lane-mt008-local")),
        "stale projection remains inspectable after refresh failure"
    );
}

#[test]
fn native_consumes_backend_generated_schema_v3_projection_artifact() {
    let artifact_root = std::env::var("HANDSHAKE_ARTIFACTS_DIR")
        .expect("backend diagnostics proof must provide HANDSHAKE_ARTIFACTS_DIR");
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
        "native consumer must read the canonical sibling Handshake_Artifacts directory"
    );
    let artifact = artifact_root
        .join("handshake-test")
        .join("wp1-final-audit")
        .join("mt014_swarm_lane_diagnostics_projection.json");
    let artifact_bytes =
        std::fs::read(&artifact).expect("backend-generated diagnostics projection is required");
    let provenance: DiagnosticsProjectionProvenance = serde_json::from_slice(
        &std::fs::read(artifact.with_extension("provenance.json"))
            .expect("backend-generated diagnostics provenance is required"),
    )
    .expect("diagnostics provenance is typed JSON");
    let now_unix_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock must be after Unix epoch")
        .as_millis() as u64;
    validate_projection_provenance(
        &provenance,
        &artifact_bytes,
        "hsk.mt017_diagnostics_projection_provenance@1",
        &std::env::var("HANDSHAKE_MT017_DIAGNOSTICS_PROOF_NONCE")
            .expect("native MT-017 proof requires the producer nonce"),
        "swarm_lane_diagnostics_backend_projection_matches_eventledger",
        now_unix_ms,
    )
    .expect("MT-017 artifact must match its current full backend producer provenance");
    let raw_projection: Value = serde_json::from_slice(&artifact_bytes)
        .expect("backend-generated diagnostics projection is JSON");
    assert_projection_wire_rejections(raw_projection.clone());
    let projection = decode_projection_for_native_surface(raw_projection)
        .expect("MT-017 backend artifact must carry valid server-authorized scope and exact native wire shape");
    assert_eq!(
        projection.schema_id,
        "hsk.model_lane_diagnostics_projection@3"
    );
    assert_eq!(provenance.projection_schema_id, projection.schema_id);

    let known_lane = projection
        .lanes
        .iter()
        .find(|lane| lane.lane_id == "lane-mt008-local")
        .expect("backend artifact contains the known-model lane");
    let known_name = known_lane.model_display_name.clone();
    let known_anchor = known_lane
        .model_stable_anchor
        .clone()
        .expect("backend artifact exposes the known stable anchor");
    let stale_lane = projection
        .lanes
        .iter()
        .find(|lane| lane.lane_id == "lane-mt014-stale")
        .expect("backend artifact contains the legacy stale-model lane");
    let stale_name = stale_lane.model_display_name.clone();
    let stale_reason = stale_lane
        .model_anchor_unavailable_reason
        .clone()
        .expect("backend artifact explains the unavailable legacy anchor");
    let routing = projection
        .routing_executions
        .iter()
        .find(|execution| execution.execution_id == "execution-mt017-diagnostics-awaiting")
        .expect("backend artifact contains the real routing lifecycle");
    assert_eq!(routing.status, "awaiting_authority");
    let authority_stage = routing
        .stages
        .iter()
        .find(|stage| stage.stage_id == "validator-verdict")
        .expect("backend artifact contains the real awaiting-authority stage");
    assert_eq!(authority_stage.state, "awaiting_authority");
    assert_eq!(authority_stage.outbox.status, "claimed");
    assert_eq!(
        authority_stage.outbox.fencing_token,
        authority_stage.fencing_token
    );
    assert!(authority_stage.authority_request_message_ref.is_some());

    let mut harness = shell_harness_with_projection(projection);
    harness.run();
    harness.get_by_label("MODELS").click();
    harness.run();
    harness.get_by_label("Open Lane Diagnostics").click();
    harness.run();
    harness.run();

    let known_author =
        handshake_native::swarm_lane_diagnostics::lane_model_label_author_id("lane-mt008-local");
    let stale_author =
        handshake_native::swarm_lane_diagnostics::lane_model_label_author_id("lane-mt014-stale");
    let known_label = accesskit_label(&harness, &known_author);
    assert!(
        known_label.contains(&known_name),
        "backend known-model label renders through the native pane: {known_label}"
    );
    assert!(
        known_label.contains(&known_anchor),
        "backend stable anchor renders through the native pane: {known_label}"
    );
    let stale_label = accesskit_label(&harness, &stale_author);
    assert!(
        stale_label.contains(&stale_name),
        "backend stale-model label renders through the native pane: {stale_label}"
    );
    assert!(
        stale_label.contains(&stale_reason),
        "backend legacy-anchor reason renders through the native pane: {stale_label}"
    );
    let routing_author = routing_execution_author_id("execution-mt017-diagnostics-awaiting");
    let routing_label = accesskit_label(&harness, &routing_author);
    assert!(
        routing_label.contains("status awaiting_authority"),
        "backend routing execution status renders through the native pane: {routing_label}"
    );
    let authority_stage_author = routing_stage_author_id(
        "execution-mt017-diagnostics-awaiting",
        "validator-verdict",
        1,
    );
    let authority_stage_label = accesskit_label(&harness, &authority_stage_author);
    assert!(
        authority_stage_label.contains("state awaiting_authority")
            && authority_stage_label.contains("fence="),
        "backend authority-stage fencing renders through the native pane: {authority_stage_label}"
    );

    let logical_authors = live_author_ids(&harness);
    for logical_author in [&known_author, &stale_author] {
        assert!(
            logical_authors
                .iter()
                .any(|actual| actual == logical_author),
            "stable logical model-label author ID is present: {logical_author}"
        );
        let scoped = raw_live_author_ids(&harness)
            .into_iter()
            .filter(|actual| {
                actual.starts_with(&format!("{logical_author}.pane."))
                    && !actual.ends_with("::egui-response")
            })
            .collect::<Vec<_>>();
        assert_eq!(
            scoped.len(),
            1,
            "logical author ID has exactly one pane-scoped production node: {logical_author}"
        );
    }
    assert_unique_swarm_author_ids(&harness);
}

#[test]
fn native_projection_provenance_rejects_wrong_bindings_incomplete_stale_and_malformed_receipts() {
    let bytes = b"provenance-validator-unit-input";
    let now = 2_000_000;
    let provenance = DiagnosticsProjectionProvenance {
        schema_id: "hsk.mt017_diagnostics_projection_provenance@1".into(),
        proof_nonce: "unit-nonce".into(),
        projection_schema_id: "hsk.model_lane_diagnostics_projection@3".into(),
        artifact_sha256: format!("{:x}", Sha256::digest(bytes)),
        producer_test_id: "swarm_lane_diagnostics_backend_projection_matches_eventledger".into(),
        producer_status: "passed_all_backend_assertions".into(),
        producer_completed_at_unix_ms: now,
    };
    let validate = |value: &DiagnosticsProjectionProvenance| {
        validate_projection_provenance(
            value, bytes, "hsk.mt017_diagnostics_projection_provenance@1", "unit-nonce",
            "swarm_lane_diagnostics_backend_projection_matches_eventledger", now,
        )
    };
    validate(&provenance).expect("unit receipt exercises validator only, not backend completion");
    for (field, expected) in [
        ("schema", "diagnostics provenance schema_id mismatch"),
        ("nonce", "diagnostics provenance proof_nonce mismatch"),
        ("projection", "diagnostics provenance projection_schema_id mismatch"),
        ("producer", "diagnostics provenance producer_test_id mismatch"),
        ("status", "diagnostics provenance producer_status incomplete"),
        ("sha", "diagnostics provenance artifact_sha256 mismatch"),
        ("stale", "diagnostics provenance completion is stale"),
        ("future", "diagnostics provenance completion is from the future"),
    ] {
        let mut changed = provenance.clone();
        match field {
            "schema" => changed.schema_id.push_str("-wrong"),
            "nonce" => changed.proof_nonce.push_str("-wrong"),
            "projection" => changed.projection_schema_id.push_str("-wrong"),
            "producer" => changed.producer_test_id.push_str("-wrong"),
            "status" => changed.producer_status = "incomplete".into(),
            "sha" => changed.artifact_sha256.push('0'),
            "stale" => changed.producer_completed_at_unix_ms = now - 30 * 60 * 1_000 - 1,
            "future" => changed.producer_completed_at_unix_ms = now + 30_001,
            _ => unreachable!(),
        }
        assert_eq!(validate(&changed).expect_err(field), expected);
    }
    assert!(serde_json::from_slice::<DiagnosticsProjectionProvenance>(b"{").is_err());
    assert!(serde_json::from_slice::<DiagnosticsProjectionProvenance>(b"{}").is_err());
}

#[tokio::test]
async fn diagnostics_production_client_without_account_delivers_errors_to_both_cells_before_http() {
    use handshake_native::backend_client::SwarmLaneDiagnosticsClient;
    use handshake_native::swarm_lane_diagnostics::SwarmLaneDiagnosticsCell;

    let client = SwarmLaneDiagnosticsClient::production(tokio::runtime::Handle::current());
    let latest: SwarmLaneDiagnosticsCell = Arc::new(Mutex::new(None));
    let run: SwarmLaneDiagnosticsCell = Arc::new(Mutex::new(None));
    client.fetch_latest(latest.clone());
    client.fetch_run("not-sent-without-account", run.clone());
    for cell in [latest, run] {
        let result = tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                let delivery = cell.lock().expect("lock production delivery cell").take();
                if let Some(result) = delivery {
                    break result;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("production client must deliver an account error, not leave an empty cell");
        assert_eq!(result.expect_err("missing account cannot succeed"), "http: Account login required");
    }
}

#[test]
fn native_projection_decoder_rejects_renamed_missing_extra_and_malformed_wire_fields() {
    assert_projection_wire_rejections(
        serde_json::to_value(fixture_projection()).expect("serialize scoped projection"),
    );
}

fn assert_projection_wire_rejections(raw: Value) {
    decode_projection_for_native_surface(raw.clone())
        .expect("unchanged scoped wire projection is accepted");

    let mut renamed = raw.clone();
    let lane = renamed["lanes"][0].as_object_mut().expect("lane object");
    let name = lane.remove("model_display_name").expect("critical field exists");
    lane.insert("renamed_model_display_name".into(), name);
    let err = decode_projection_for_native_surface(renamed)
        .expect_err("renamed required field must fail at the production decoder");
    assert!(err.contains("model_display_name"), "got {err}");

    let mut missing = raw.clone();
    missing["run"].as_object_mut().expect("run object").remove("candidate_model_ids");
    let err = decode_projection_for_native_surface(missing)
        .expect_err("missing required field must fail at the production decoder");
    assert!(err.contains("candidate_model_ids"), "got {err}");

    let mut extra = raw.clone();
    extra["run"].as_object_mut().expect("run object")
        .insert("unexpected_candidate_model_ids".into(), serde_json::json!(["untrusted"]));
    assert_eq!(
        decode_projection_for_native_surface(extra).expect_err("extra wire data cannot be lost"),
        "backend/native projection JSON field shape changed or lost data"
    );

    let mut defaulted = raw.clone();
    defaulted["run"].as_object_mut().expect("run object")
        .remove("coordinator_session_id").expect("defaulted field exists in emitted wire data");
    assert_eq!(
        decode_projection_for_native_surface(defaulted)
            .expect_err("serde default materialization must fail shape validation before semantics"),
        "backend/native projection JSON field shape changed or lost data"
    );

    let mut stale_schema = raw.clone();
    stale_schema["schema_id"] = Value::String("hsk.model_lane_diagnostics_projection@2".into());
    assert_eq!(
        decode_projection_for_native_surface(stale_schema)
            .expect_err("stale projection schema must fail through the production decoder"),
        "schema_id mismatch: hsk.model_lane_diagnostics_projection@2"
    );

    let mut explicit_null = raw.clone();
    explicit_null["resource_scope"] = Value::Null;
    assert_eq!(
        decode_projection_for_native_surface(explicit_null)
            .expect_err("explicit null scope is not equivalent to omitted serialized scope"),
        "backend/native projection JSON field shape changed or lost data"
    );
    let mut omitted = raw;
    omitted.as_object_mut().expect("projection object").remove("resource_scope");
    let err = decode_projection_for_native_surface(omitted)
        .expect_err("omitted server scope must fail semantic validation");
    assert!(err.contains("resource_scope attribution"), "got {err}");

    let err = serde_json::from_slice::<Value>(b"{")
        .map_err(|error| error.to_string())
        .and_then(decode_projection_for_native_surface)
        .expect_err("malformed JSON cannot produce a projection");
    assert!(!err.is_empty());
}

#[test]
fn swarm_lane_diagnostics_argus_rejects_missing_author_id_and_count_mismatch() {
    let mut projection = fixture_projection();
    projection.resource_scope = None;
    let err = validate_projection_for_native_surface(&projection)
        .expect_err("missing exact privacy attribution must fail closed");
    assert!(err.contains("resource_scope attribution"), "got {err}");

    let mut projection = fixture_projection();
    projection
        .resource_scope
        .as_mut()
        .expect("fixture carries exact privacy scope")
        .authenticated_session_fingerprint
        .clear();
    let err = validate_projection_for_native_surface(&projection)
        .expect_err("partial exact privacy attribution must fail closed");
    assert!(err.contains("resource_scope attribution"), "got {err}");

    let mut serialized = serde_json::to_value(fixture_projection()).expect("serialize fixture");
    serialized["lanes"][0]
        .as_object_mut()
        .expect("lane JSON object")
        .remove("model_display_name");
    let err = serde_json::from_value::<SwarmLaneDiagnosticsProjection>(serialized)
        .expect_err("backend/native contract requires model_display_name");
    assert!(err.to_string().contains("model_display_name"), "got {err}");

    let mut serialized = serde_json::to_value(fixture_projection()).expect("serialize fixture");
    serialized["run"]
        .as_object_mut()
        .expect("run JSON object")
        .remove("candidate_model_ids");
    let err = serde_json::from_value::<SwarmLaneDiagnosticsProjection>(serialized)
        .expect_err("backend/native contract requires candidate_model_ids");
    assert!(err.to_string().contains("candidate_model_ids"), "got {err}");

    let mut projection = fixture_projection();
    projection.run.candidate_model_ids.clear();
    let err = validate_projection_for_native_surface(&projection)
        .expect_err("empty candidate model authority must fail closed");
    assert!(
        err.contains("run coordinator/routing/owner/model/recovery refs missing"),
        "got {err}"
    );

    let mut serialized = serde_json::to_value(fixture_projection()).expect("serialize fixture");
    serialized["run"]
        .as_object_mut()
        .expect("run JSON object")
        .remove("locus_ref");
    let _: SwarmLaneDiagnosticsProjection = serde_json::from_value(serialized.clone())
        .expect("optional field omission is accepted by serde before the boundary guard");
    let err = decode_projection_for_native_surface(serialized)
        .expect_err("backend field omissions must not survive the exact boundary guard");
    assert!(err.contains("field shape changed"), "got {err}");

    let mut projection = fixture_projection();
    projection.lanes[0].capability_token_ids.clear();
    let err = validate_projection_for_native_surface(&projection)
        .expect_err("empty lane capability authority must fail closed");
    assert!(err.contains("capability/ToolGate"), "got {err}");

    let mut projection = fixture_projection();
    projection.lanes[0].tool_gate_decision_refs.clear();
    let err = validate_projection_for_native_surface(&projection)
        .expect_err("empty lane ToolGate authority must fail closed");
    assert!(err.contains("capability/ToolGate"), "got {err}");

    let mut projection = fixture_projection();
    projection.routing_executions[0].stages[1].dependency_stage_ids =
        vec!["tampered-missing-stage".into()];
    let err = validate_projection_for_native_surface(&projection)
        .expect_err("tampered routing dependency lineage must fail closed");
    assert!(err.contains("dependency lineage"), "got {err}");

    let mut projection = fixture_projection();
    projection.routing_executions[1].stages[0].input_refs = vec![String::new()];
    let err = validate_projection_for_native_surface(&projection)
        .expect_err("missing routing input lineage must fail closed");
    assert!(err.contains("input/EventLedger lineage"), "got {err}");

    let mut projection = fixture_projection();
    projection.routing_executions[0].stages[1].authority_request_message_ref = None;
    let err = validate_projection_for_native_surface(&projection)
        .expect_err("awaiting authority without causal request lineage must fail closed");
    assert!(err.contains("causal authority lineage"), "got {err}");

    let mut projection = fixture_projection();
    projection.routing_executions[2].stages[0].outbox.status = "dead_letter".into();
    let err = validate_projection_for_native_surface(&projection)
        .expect_err("outbox status must match the persisted current lifecycle state");
    assert!(err.contains("outbox state"), "got {err}");

    let mut projection = fixture_projection();
    projection.routing_executions[3].cancel_reason = None;
    let err = validate_projection_for_native_surface(&projection)
        .expect_err("cancelled execution without authority reason must fail closed");
    assert!(err.contains("cancel_reason"), "got {err}");

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
    projection.messages[0].kind = "status".into();
    projection.messages[0].crdt_base_snapshot_ref = None;
    let err = validate_projection_for_native_surface(&projection)
        .expect_err("non-proposal CRDT targets must fail closed on partial metadata");
    assert!(err.contains("CRDT refs missing"), "got {err}");

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
    let projection = load_current_mt009_backend_projection();
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
    harness.get_by_label("MODELS").click();
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
    harness.get_by_label("MODELS").click();
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
    harness.get_by_label("MODELS").click();
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


#[test]
fn joint_projection_pair_rejects_wrong_bindings_shape_status_time_and_raw_identity() {
    let projection = fixture_projection();
    let scope = projection.resource_scope.as_ref().expect("unit scope");
    let raw = serde_json::to_vec(&projection).expect("serialize unit projection");
    let now = 2_000_000_u64;
    let receipt = serde_json::json!({
        "schema_id": "hsk.mt017_native_joint_diagnostics_provenance@1",
        "proof_nonce": "joint-validator-unit-nonce",
        "projection_schema_id": "hsk.model_lane_diagnostics_projection@3",
        "artifact_sha256": format!("{:x}", Sha256::digest(&raw)),
        "producer_target": "test_swarm_lane_diagnostics_argus",
        "producer_test_id": "native_consumes_backend_generated_schema_v3_projection_artifact",
        "producer_status": "passed_joint_backend_native_assertions_and_cleanup",
        "producer_completed_at_unix_ms": now,
        "run_id": projection.run.run_id.clone(),
        "resource_scope": scope,
    });
    let validate = |bytes: &[u8], value: &Value| {
        validate_joint_projection_pair(
            bytes, &serde_json::to_vec(value).expect("serialize unit receipt"),
            "joint-validator-unit-nonce", &projection.run.run_id, scope, now,
        )
    };
    validate(&raw, &receipt).expect("unit pair is validator input, not joint runtime proof");
    for key in [
        "schema_id", "proof_nonce", "projection_schema_id", "artifact_sha256",
        "producer_target", "producer_test_id", "producer_status", "run_id",
    ] {
        let mut wrong = receipt.clone();
        wrong[key] = Value::String("wrong-unit-value".into());
        assert!(validate(&raw, &wrong).is_err(), "{key} must bind");
    }
    let mut missing = receipt.clone();
    missing.as_object_mut().expect("receipt object").remove("resource_scope");
    assert_eq!(validate(&raw, &missing).expect_err("missing field"),
        "joint provenance field shape mismatch");
    let mut extra = receipt.clone();
    extra["unrecognized"] = Value::Bool(true);
    assert_eq!(validate(&raw, &extra).expect_err("unknown field"),
        "joint provenance field shape mismatch");
    for (value, expected) in [
        (serde_json::json!(now - 30 * 60 * 1_000 - 1), "joint provenance completion is stale"),
        (serde_json::json!(now + 30_001), "joint provenance completion is from the future"),
        (Value::String(now.to_string()), "joint provenance completion must be u64"),
        (serde_json::json!(-1), "joint provenance completion must be u64"),
    ] {
        let mut wrong = receipt.clone();
        wrong["producer_completed_at_unix_ms"] = value;
        assert_eq!(validate(&raw, &wrong).expect_err("invalid completion"), expected);
    }
    let mut wrong_scope = receipt.clone();
    wrong_scope["resource_scope"] = Value::Null;
    assert_eq!(validate(&raw, &wrong_scope).expect_err("wrong receipt scope"),
        "joint provenance resource_scope mismatch");

    for key in [
        "owner_account_fingerprint", "actor_principal_fingerprint",
        "authenticated_session_fingerprint", "access_space_fingerprint",
        "workspace_fingerprint", "visibility", "denial_posture",
    ] {
        let mut wrong = receipt.clone();
        wrong["resource_scope"][key] = Value::String("wrong-unit-value".into());
        assert_eq!(validate(&raw, &wrong).expect_err("every scope field must bind"),
            "joint provenance resource_scope mismatch");
    }
    let mut foreign_wire = serde_json::to_value(&projection).expect("unit raw");
    foreign_wire["resource_scope"]["owner_account_fingerprint"] =
        Value::String("f".repeat(64));
    let foreign_bytes = serde_json::to_vec(&foreign_wire).expect("foreign scope raw");
    let mut foreign_receipt = receipt.clone();
    foreign_receipt["artifact_sha256"] =
        Value::String(format!("{:x}", Sha256::digest(&foreign_bytes)));
    assert_eq!(validate(&foreign_bytes, &foreign_receipt)
        .expect_err("matching raw hash cannot bless foreign projection scope"),
        "joint projection resource_scope mismatch");

    let mut wrong_run = receipt.clone();
    wrong_run["run_id"] = Value::String("foreign-unit-run".into());
    assert_eq!(validate_joint_projection_pair(
        &raw, &serde_json::to_vec(&wrong_run).expect("wrong-run receipt"),
        "joint-validator-unit-nonce", "foreign-unit-run", scope, now,
    ).expect_err("matching receipt cannot bless a different valid projection run"),
        "joint projection run_id mismatch");

    assert_eq!(validate(b"{", &receipt).expect_err("changed malformed bytes fail hash first"),
        "joint provenance artifact_sha256 mismatch");
    let mut malformed_receipt = receipt.clone();
    malformed_receipt["artifact_sha256"] = Value::String(format!("{:x}", Sha256::digest(b"{")));
    assert!(validate(b"{", &malformed_receipt).expect_err("malformed raw").starts_with("joint projection JSON:"));
    assert!(validate_joint_projection_pair(
        &raw, b"{", "joint-validator-unit-nonce", &projection.run.run_id, scope, now,
    ).expect_err("malformed receipt").starts_with("joint provenance JSON:"));
}


async fn await_joint_delivery<T>(
    cell: Arc<Mutex<Option<Result<T, String>>>>,
    original_deadline: tokio::time::Instant,
    mut cancellation: tokio::sync::watch::Receiver<bool>,
) -> Result<Result<T, String>, String> {
    loop {
        if *cancellation.borrow() {
            return Err("joint callback cancelled; Core retains request settlement".into());
        }
        if tokio::time::Instant::now() >= original_deadline {
            return Err("joint callback original deadline expired; Core retains request settlement".into());
        }
        let delivered = cell.lock().map_err(|_| "joint delivery cell poisoned")?.take();
        if let Some(result) = delivered {
            return Ok(result);
        }
        tokio::select! {
            _ = tokio::time::sleep_until(original_deadline) => {
                return Err("joint callback original deadline expired; Core retains request settlement".into());
            }
            changed = cancellation.changed() => {
                changed.map_err(|_| "joint callback cancellation channel closed")?;
            }
            _ = tokio::task::yield_now() => {}
        }
    }
}

async fn assert_native_joint_fixture(
    inputs: handshake_core::test_harness::swarm_lane_diagnostics::NativeFixtureCallbackInputs,
) -> Result<(), String> {
    use handshake_native::backend_client::SwarmLaneDiagnosticsClient;
    use handshake_native::local_account::AuthenticatedContext;

    if *inputs.cancellation.borrow() || tokio::time::Instant::now() >= inputs.original_deadline {
        return Err("joint fixture callback has no live original deadline".into());
    }
    let context: AuthenticatedContext = serde_json::from_value(inputs.issued_auth_context)
        .map_err(|error| format!("joint issued authentication response: {error}"))?;
    let context = context.bind(&inputs.origin, inputs.raw_channel_token)?;
    if context.account_id != inputs.scope.owner_account_id.to_string()
        || context.principal_id != inputs.scope.actor_principal_id.to_string()
        || context.session_id != inputs.scope.authenticated_session_id.to_string()
        || context.access_space_id != inputs.scope.access_space_id.to_string()
        || inputs.scope.workspace_id.as_str().is_empty()
    {
        return Err("joint issued authentication does not match the exact fixture scope".into());
    }
    if inputs.run_id.is_empty() || inputs.denied_run_id.is_empty()
        || inputs.run_id == inputs.denied_run_id || inputs.expected_denied_status.as_u16() != 404
    {
        return Err("joint fixture canonical/denied run bindings are invalid".into());
    }
    let expected = decode_projection_for_native_surface(inputs.public_wire.clone())?;
    if expected.run.run_id != inputs.run_id {
        return Err("joint fixture public wire does not name the canonical run".into());
    }
    let expected_scope = expected.resource_scope.as_ref()
        .ok_or("joint fixture public wire has no authorized scope")?;
    let mut base_wire = inputs.public_wire.clone();
    base_wire.as_object_mut().ok_or("joint fixture wire is not an object")?
        .remove("resource_scope");
    if serde_json::to_value(&inputs.diagnostics)
        .map_err(|error| format!("joint canonical diagnostics serialization: {error}"))? != base_wire
    {
        return Err("joint public wire differs from canonical fixture diagnostics".into());
    }

    let account = Arc::new(context);
    let client = SwarmLaneDiagnosticsClient::new(
        inputs.origin, tokio::runtime::Handle::current(),
    ).with_authenticated_context(Some(account.clone()));
    let latest = Arc::new(Mutex::new(None));
    let run = Arc::new(Mutex::new(None));
    let denied = Arc::new(Mutex::new(None));
    let raw = Arc::new(Mutex::new(None));
    let raw_client = handshake_native::backend_client::shared_http_client();
    let raw_request = account.authorize_builder(
        raw_client.clone(), raw_client.get(client.run_request(&inputs.run_id).url),
    )?;
    if *inputs.cancellation.borrow() || tokio::time::Instant::now() >= inputs.original_deadline {
        return Err("joint callback expired before request admission".into());
    }

    let latest_id = inputs.requests.spawn_retained(client.latest_request_future(latest.clone()))
        .map_err(|error| format!("joint latest admission: {error}"))?;
    let run_id = inputs.requests.spawn_retained(client.run_request_future(&inputs.run_id, run.clone()))
        .map_err(|error| format!("joint run admission: {error}"))?;
    let denied_id = inputs.requests.spawn_retained(
        client.run_request_future(&inputs.denied_run_id, denied.clone()),
    ).map_err(|error| format!("joint denied-run admission: {error}"))?;
    let raw_delivery = raw.clone();
    let raw_id = inputs.requests.spawn_retained(async move {
        let result = async {
            let response = raw_request.send().await
                .map_err(|error| format!("joint authorized raw GET: {error}"))?;
            account.observe_status(response.status());
            if !response.status().is_success() {
                return Err(format!("joint authorized raw GET non-success status {}", response.status()));
            }
            if !account.is_active() {
                return Err("joint authorized raw GET account invalidated".into());
            }
            let bytes = response.bytes().await
                .map_err(|error| format!("joint authorized raw body: {error}"))?;
            Ok(bytes.to_vec())
        }.await;
        if let Ok(mut slot) = raw_delivery.lock() {
            *slot = Some(result);
        }
    }).map_err(|error| format!("joint raw GET admission: {error}"))?;
    let ids = [latest_id.index(), run_id.index(), denied_id.index(), raw_id.index()];
    if ids.iter().enumerate().any(|(index, id)| ids[..index].contains(id)) {
        return Err("joint retained request IDs are not unique".into());
    }

    let actual_latest = await_joint_delivery(
        latest, inputs.original_deadline, inputs.cancellation.clone(),
    ).await?.map_err(|error| format!("joint latest request: {error}"))?;
    let actual_run = await_joint_delivery(
        run, inputs.original_deadline, inputs.cancellation.clone(),
    ).await?.map_err(|error| format!("joint run request: {error}"))?;
    let denied_result = await_joint_delivery(
        denied, inputs.original_deadline, inputs.cancellation.clone(),
    ).await?;
    match denied_result {
        Err(error) if error == format!("http: GET non-success status {}", inputs.expected_denied_status) => {}
        Err(error) => return Err(format!("joint denied-run error differs from the fixture status: {error}")),
        Ok(_) => return Err("joint denied run unexpectedly returned a projection".into()),
    }
    let raw_bytes = await_joint_delivery(
        raw, inputs.original_deadline, inputs.cancellation.clone(),
    ).await?.map_err(|error| format!("joint authorized raw request: {error}"))?;
    let actual_raw: Value = serde_json::from_slice(&raw_bytes)
        .map_err(|error| format!("joint authorized raw JSON: {error}"))?;
    if actual_raw != inputs.public_wire {
        return Err("joint authorized raw GET differs from canonical scoped wire".into());
    }
    for (label, projection) in [("latest", &actual_latest), ("run", &actual_run)] {
        if serde_json::to_value(projection)
            .map_err(|error| format!("joint {label} serialization: {error}"))? != inputs.public_wire
        {
            return Err(format!("joint production {label} cell differs from canonical scoped wire"));
        }
    }

    {
        let mut harness = shell_harness_with_projection(actual_latest);
        harness.run();
        harness.get_by_label("MODELS").click();
        harness.run();
        harness.get_by_label("Open Lane Diagnostics").click();
        harness.run();
        for (author, value) in [
            (PRIVACY_OWNER_AUTHOR_ID, &expected_scope.owner_account_fingerprint),
            (PRIVACY_PRINCIPAL_AUTHOR_ID, &expected_scope.actor_principal_fingerprint),
            (PRIVACY_SESSION_AUTHOR_ID, &expected_scope.authenticated_session_fingerprint),
            (PRIVACY_ACCESS_SPACE_AUTHOR_ID, &expected_scope.access_space_fingerprint),
            (PRIVACY_WORKSPACE_AUTHOR_ID, &expected_scope.workspace_fingerprint),
        ] {
            if !accesskit_label(&harness, author).contains(value.as_str()) {
                return Err("joint mounted scope fingerprint does not match actual scoped wire".into());
            }
        }
        if accesskit_label(&harness, PRIVACY_VISIBILITY_AUTHOR_ID) != format!(
            "visibility {} | exact account + Principal + session + AccessSpace + workspace | this read cannot widen sharing",
            expected_scope.visibility,
        ) {
            return Err("joint mounted visibility does not match actual scoped wire".into());
        }
        if accesskit_label(&harness, PRIVACY_DENIAL_AUTHOR_ID) != format!(
            "denial posture {} | mismatched caller scope: forbidden | foreign stored scope: not found | restricted metadata withheld",
            expected_scope.denial_posture,
        ) {
            return Err("joint mounted denial posture does not match actual scoped wire".into());
        }
        assert_unique_swarm_author_ids(&harness);
    }
    if *inputs.cancellation.borrow() || tokio::time::Instant::now() >= inputs.original_deadline {
        return Err("joint callback completed after cancellation or original deadline".into());
    }
    // Core alone joins retained requests and performs all cleanup before publication.
    Ok(())
}

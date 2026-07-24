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
    validate_projection_for_native_surface, visible_message_ids_for_filters,
    SwarmLaneDiagnosticsLane, SwarmLaneDiagnosticsMessage, SwarmLaneDiagnosticsMtStatus,
    SwarmLaneDiagnosticsProjection, SwarmLaneDiagnosticsRun, SwarmLaneDiagnosticsTier,
    SwarmLaneDiagnosticsTransport, SwarmLaneRoutingExecutionDiagnostics,
    SwarmLaneRoutingOutboxDiagnostics, SwarmLaneRoutingStageDiagnostics, ERROR_AUTHOR_ID,
    FRESHNESS_AUTHOR_ID, LANE_FILTER_AUTHOR_ID, MESSAGE_FILTER_AUTHOR_ID, REFRESH_AUTHOR_ID,
    RUN_FILTER_AUTHOR_ID, SURFACE_AUTHOR_ID,
};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

#[derive(Deserialize)]
struct DiagnosticsProjectionProvenance {
    schema_id: String,
    proof_nonce: String,
    projection_schema_id: String,
    artifact_sha256: String,
    producer_test_id: String,
    producer_status: String,
    producer_completed_at_unix_ms: u64,
}

fn validate_exact_backend_json_shape(
    raw: &Value,
    projection: &SwarmLaneDiagnosticsProjection,
) -> Result<(), String> {
    let native = serde_json::to_value(projection)
        .map_err(|error| format!("native projection reserialization failed: {error}"))?;
    if &native == raw {
        Ok(())
    } else {
        Err("backend/native projection JSON field shape changed or lost data".to_owned())
    }
}

fn load_current_mt009_backend_projection() -> SwarmLaneDiagnosticsProjection {
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
    assert_eq!(
        provenance.schema_id,
        "hsk.mt009_diagnostics_projection_provenance@1"
    );
    assert_eq!(
        provenance.proof_nonce,
        std::env::var("HANDSHAKE_MT009_DIAGNOSTICS_PROOF_NONCE")
            .expect("native MT-009 proof requires the producer nonce"),
        "native proof must consume the artifact from the current backend producer run"
    );
    assert_eq!(
        provenance.producer_test_id,
        "mixed_local_cloud_subagent_run_persists_restarts_replays_and_projects"
    );
    assert_eq!(
        provenance.producer_status, "passed_all_backend_assertions",
        "native proof requires the backend producer completion receipt"
    );
    let now_unix_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock must be after Unix epoch")
        .as_millis() as u64;
    assert!(
        provenance.producer_completed_at_unix_ms <= now_unix_ms.saturating_add(30_000),
        "MT-009 producer completion receipt cannot be from the future"
    );
    assert!(
        now_unix_ms.saturating_sub(provenance.producer_completed_at_unix_ms) <= 30 * 60 * 1_000,
        "MT-009 producer completion receipt is stale; rerun the backend producer"
    );
    assert_eq!(
        provenance.artifact_sha256,
        format!("{:x}", Sha256::digest(&artifact_bytes)),
        "MT-009 artifact bytes must match backend provenance"
    );
    let raw_projection: Value = serde_json::from_slice(&artifact_bytes)
        .expect("backend-generated MT-009 projection is JSON");
    let projection: SwarmLaneDiagnosticsProjection = serde_json::from_value(raw_projection.clone())
        .expect("native contract consumes backend-generated MT-009 projection JSON");
    validate_exact_backend_json_shape(&raw_projection, &projection)
        .expect("MT-009 backend/native projection field shape round-trips exactly");
    assert_eq!(provenance.projection_schema_id, projection.schema_id);
    projection
}

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

fn raw_live_author_ids(harness: &Harness<'_, HandshakeApp>) -> Vec<String> {
    harness
        .root()
        .children_recursive()
        .filter_map(|n| n.accesskit_node().author_id().map(str::to_owned))
        .collect()
}

fn assert_unique_swarm_author_ids(harness: &Harness<'_, HandshakeApp>) {
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

fn node_by_author<'a>(
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

fn accesskit_label(harness: &Harness<'_, HandshakeApp>, author_id: &str) -> String {
    node_by_author(harness, author_id)
        .accesskit_node()
        .label()
        .unwrap_or_default()
        .to_owned()
}

#[derive(Default)]
struct SequencedDiagnosticsTransport {
    results: Mutex<VecDeque<Result<SwarmLaneDiagnosticsProjection, String>>>,
}

impl SequencedDiagnosticsTransport {
    fn new(
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
    assert_eq!(
        provenance.schema_id,
        "hsk.mt017_diagnostics_projection_provenance@1"
    );
    assert_eq!(
        provenance.proof_nonce,
        std::env::var("HANDSHAKE_MT017_DIAGNOSTICS_PROOF_NONCE")
            .expect("native proof requires the producer nonce"),
        "native proof must consume an artifact from the current backend producer run"
    );
    assert_eq!(
        provenance.producer_test_id,
        "swarm_lane_diagnostics_backend_projection_matches_eventledger"
    );
    assert_eq!(
        provenance.producer_status, "passed_all_backend_assertions",
        "native proof requires the backend producer completion receipt"
    );
    let now_unix_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock must be after Unix epoch")
        .as_millis() as u64;
    assert!(
        provenance.producer_completed_at_unix_ms <= now_unix_ms.saturating_add(30_000),
        "MT-017 producer completion receipt cannot be from the future"
    );
    assert!(
        now_unix_ms.saturating_sub(provenance.producer_completed_at_unix_ms) <= 30 * 60 * 1_000,
        "MT-017 producer completion receipt is stale; rerun the backend producer"
    );
    assert_eq!(
        provenance.artifact_sha256,
        format!("{:x}", Sha256::digest(&artifact_bytes)),
        "artifact bytes must match backend provenance"
    );
    let raw_projection: Value = serde_json::from_slice(&artifact_bytes)
        .expect("backend-generated diagnostics projection is JSON");
    let projection: SwarmLaneDiagnosticsProjection = serde_json::from_value(raw_projection.clone())
        .expect("native contract consumes backend-generated projection JSON");
    validate_exact_backend_json_shape(&raw_projection, &projection)
        .expect("MT-017 backend/native projection field shape round-trips exactly");
    assert_eq!(
        projection.schema_id,
        "hsk.model_lane_diagnostics_projection@3"
    );
    assert_eq!(provenance.projection_schema_id, projection.schema_id);
    validate_projection_for_native_surface(&projection)
        .expect("backend-generated projection satisfies native schema-v3 contract");

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
fn swarm_lane_diagnostics_argus_rejects_missing_author_id_and_count_mismatch() {
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
    let projection: SwarmLaneDiagnosticsProjection = serde_json::from_value(serialized.clone())
        .expect("optional field omission is accepted by serde before the boundary guard");
    let err = validate_exact_backend_json_shape(&serialized, &projection)
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

fn routing_stage_fixture(
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

fn routing_execution_fixture(
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

fn routing_lifecycle_fixture() -> Vec<SwarmLaneRoutingExecutionDiagnostics> {
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

fn fixture_projection() -> SwarmLaneDiagnosticsProjection {
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
    }
}

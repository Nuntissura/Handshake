//! WP-KERNEL-012 E3 MT-026 remediation (FAIL_V2): canonical Argus inspect / safe-steer / mutate /
//! re-observe proof for the MOUNTED Loom canvas board over REAL persisted SurrealDB data.
//!
//! `validation_v2` failed MT-026 because "no current canonical Argus run proves mounted placement,
//! movement, grouping, semantic/visual edges, deletion, and post-action state against real persisted
//! data." The existing `test_canvas_board.rs` proves the mutation/reload/removal suite against real
//! SurrealDB, but it clicks the mounted controls via raw AccessKit action requests — it never drives the
//! MOUNTED `HandshakeApp` through the real localhost `SwarmMcpServer` transport (`argus.inspect` /
//! `argus.click`) the way an out-of-process swarm agent does, and it never re-observes the mounted tree
//! through canonical Argus after a mutation. This test closes that exact gap:
//!
//!   1. seeds a REAL Handshake-managed SurrealDB workspace with two source LoomBlocks, one canvas board,
//!      and two placements through the production HTTP routes (`POST /loom/blocks`,
//!      `POST /loom/canvas-boards`, `POST /loom/canvas-boards/{id}/placements`),
//!   2. mounts the production `HandshakeApp` shell with the Canvas pane bound to that seeded board and lets
//!      the app's OWN per-frame feed fetch the board projection and drain it into the mounted
//!      `LoomCanvasBoard` (no injected fixture),
//!   3. binds the CANONICAL Argus driver (real localhost JSON-RPC) to the mounted app,
//!   4. `argus.inspect` proves each real-SurrealDB placement card is addressable by its stable author_id
//!      (`canvas.placement.{sanitized_placement_id}`) plus the fixed toolbar controls,
//!   5. drives a SAFE control action (`canvas.zoom-in`) through the real Argus transport and re-observes
//!      that the placements remain addressable and the zoom-value control is present,
//!   6. drives a MUTATION (`canvas.placement.{id}.remove`) through the real Argus transport, drives the
//!      mounted host until the real DELETE + re-fetch removes the placement, then a FRESH `argus.inspect`
//!      re-observes that the removed card is gone from the canonical tree while the sibling card remains —
//!      and a direct backend GET proves the source LoomBlock survived the placement removal, and
//!   7. writes before/after tree evidence + a screenshot marker (headless DEFERRED acceptable) and deletes
//!      the workspace.
//!
//! A second test proves a real canvas board with zero placements renders + inspects through canonical
//! Argus with no placement cards and no panic (AC10).
//!
//! Requires the `integration` feature and a reachable managed backend. Feature-gated but NOT ignored.
#![cfg(feature = "integration")]

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;
use screenshot_harness::ScreenshotHarness as Harness;

#[path = "native_gui_support/canonical_argus_driver.rs"]
mod canonical_argus_driver;
use canonical_argus_driver::{json_has_author_id, json_node_by_author_id, CanonicalArgusDriver};

#[path = "interconnect_support/mod.rs"]
mod interconnect_support;

use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::{
    HealthInfo, LoomGraphCell, LoomGraphClient, LoomGraphRequestIdentity,
};
use handshake_native::command_registry::CMD_VIEW_CANVAS;
use handshake_native::graph::canvas_board::{
    placement_author_id, placement_remove_author_id, LoomCanvasBoard, ADD_CARD_AUTHOR_ID,
    EDGE_MODE_AUTHOR_ID, PAN_LEFT_AUTHOR_ID, PAN_RIGHT_AUTHOR_ID,
    PLACEMENT_MUTATION_COMPLETION_AUTHOR_ID, PLACEMENT_REMOVAL_DETAIL_SCHEMA,
    PLACEMENT_REMOVAL_SEMANTIC_SCHEMA, START_EDGE_AUTHOR_ID, STATUS_AUTHOR_ID,
    VIEWPORT_ACTION_SEMANTIC_SCHEMA, VIEWPORT_COMPLETION_AUTHOR_ID,
    VIEWPORT_COMPLETION_DETAIL_SCHEMA, ZOOM_IN_AUTHOR_ID, ZOOM_OUT_AUTHOR_ID, ZOOM_VALUE_AUTHOR_ID,
};

/// The opt-in completion-token schema `crate::mcp::action` acknowledges (`handshake.click-completion/v1`).
const CLICK_COMPLETION_SCHEMA: &str = "handshake.click-completion/v1";

/// The exact action receipt for `receipt_id` inside a canonical `argus.inspect` tree.
fn receipt_for(after: &serde_json::Value, receipt_id: u64) -> Option<&serde_json::Value> {
    after["action_receipts"]
        .as_array()?
        .iter()
        .find(|receipt| receipt["receipt_id"].as_u64() == Some(receipt_id))
}

/// Parse the observer completion token a terminal receipt observed at acknowledgement.
fn completion_token(receipt: &serde_json::Value) -> Option<serde_json::Value> {
    serde_json::from_str(receipt["observed_value"].as_str()?).ok()
}

/// Parse a JSON-in-string token field (`semantic_value` / `terminal_detail`).
fn nested_json(token: &serde_json::Value, field: &str) -> Option<serde_json::Value> {
    serde_json::from_str(token[field].as_str()?).ok()
}

fn advanced(after: Option<u64>, before: Option<u64>) -> bool {
    matches!((after, before), (Some(a), Some(b)) if a > b)
}

fn close(value: Option<f64>, expected: f64) -> bool {
    value.is_some_and(|value| (value - expected).abs() < 1e-6)
}

fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

fn assert_no_local_artifact_dir() {
    for local in ["test_output", "tests/screenshots"] {
        let p = Path::new(local);
        assert!(
            !p.exists(),
            "CX-212E: no repo-local '{local}' artifact dir may exist (found {})",
            p.display()
        );
    }
}

fn collect_author_ids(value: &serde_json::Value, out: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(id) = map.get("author_id").and_then(|v| v.as_str()) {
                out.push(id.to_owned());
            }
            for v in map.values() {
                collect_author_ids(v, out);
            }
        }
        serde_json::Value::Array(items) => {
            for v in items {
                collect_author_ids(v, out);
            }
        }
        _ => {}
    }
}

struct LiveWorkspaceCleanup<'a> {
    backend: &'a interconnect_support::LiveBackend,
    workspace_id: String,
    cleaned: bool,
}

impl LiveWorkspaceCleanup<'_> {
    fn assert_cleaned(&mut self) {
        let status = self.backend.delete_workspace(&self.workspace_id);
        assert!(
            matches!(status, 200 | 202 | 204 | 404),
            "managed-SurrealDB workspace cleanup returned HTTP {status}"
        );
        self.cleaned = true;
    }
}

impl Drop for LiveWorkspaceCleanup<'_> {
    fn drop(&mut self) {
        if !self.cleaned {
            let _ = self.backend.delete_workspace(&self.workspace_id);
        }
    }
}

/// Mount a production shell bound to the live backend + seeded workspace, with the Canvas pane bound to
/// the seeded canvas board and opened on the active work surface.
fn canvas_shell(
    base: &str,
    workspace_id: &str,
    canvas_block_id: &str,
) -> (
    HandshakeApp,
    tokio::runtime::Runtime,
    Arc<Mutex<LoomCanvasBoard>>,
) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("build multi-thread runtime");
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }));
    app.set_backend_base_url_for_test(base, runtime.handle().clone());
    assert!(
        app.switch_project(workspace_id),
        "switch to the seeded managed-SurrealDB workspace"
    );
    let board = app.mounted_canvas_board();
    {
        let mut guard = board.lock().unwrap();
        guard.workspace_id = workspace_id.to_owned();
        guard.canvas_block_id = canvas_block_id.to_owned();
    }
    assert!(
        app.dispatch_palette_action_for_test(CMD_VIEW_CANVAS),
        "the View Canvas command mounts the production Canvas pane"
    );
    (app, runtime, board)
}

fn drive_until(
    harness: &mut Harness<'_, HandshakeApp>,
    board: &Arc<Mutex<LoomCanvasBoard>>,
    condition: impl Fn(&LoomCanvasBoard) -> bool,
    proof: &str,
) {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        harness.run_steps(2);
        if board.lock().map(|b| condition(&b)).unwrap_or(false) {
            return;
        }
        assert!(Instant::now() < deadline, "timed out waiting for '{proof}'");
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn unique_suffix() -> String {
    format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock after unix epoch")
            .as_nanos()
    )
}

#[test]
fn mt026_mounted_canvas_canonical_argus_inspect_steer_mutate_reobserve() {
    let live = interconnect_support::require_reachable_backend();
    let unique = format!("mt026-argus-{}", unique_suffix());
    let workspace = live.create_workspace(&unique);
    let workspace_id = workspace["id"]
        .as_str()
        .expect("workspace create returns id")
        .to_owned();
    let mut cleanup = LiveWorkspaceCleanup {
        backend: &live,
        workspace_id: workspace_id.clone(),
        cleaned: false,
    };

    // Seed two source blocks, a canvas board, and two placements referencing the sources.
    let create_block = |title: &str| {
        let block = live.post_json(
            &format!("/workspaces/{workspace_id}/loom/blocks"),
            &serde_json::json!({ "content_type": "note", "title": title }),
        );
        block["block_id"]
            .as_str()
            .expect("Loom block create returns block_id")
            .to_owned()
    };
    let source_one = create_block("MT-026 Argus source one");
    let source_two = create_block("MT-026 Argus source two");
    let canvas = live.post_json(
        &format!("/workspaces/{workspace_id}/loom/canvas-boards"),
        &serde_json::json!({ "title": format!("MT-026 Argus canvas {unique}") }),
    );
    let canvas_id = canvas["block_id"]
        .as_str()
        .expect("canvas create returns block_id")
        .to_owned();
    let place = |placed_block_id: &str, x: f64, y: f64| {
        let placement = live.post_json(
            &format!("/workspaces/{workspace_id}/loom/canvas-boards/{canvas_id}/placements"),
            &serde_json::json!({ "placed_block_id": placed_block_id, "x": x, "y": y, "w": 200.0, "h": 120.0 }),
        );
        placement["placement_id"]
            .as_str()
            .expect("placement create returns placement_id")
            .to_owned()
    };
    let placement_one = place(&source_one, 40.0, 40.0);
    let placement_two = place(&source_two, 320.0, 220.0);

    // Mount the production shell; the app self-fetches the board and drains the two real placements in.
    let (app, _rt, board) = canvas_shell(&live.base, &workspace_id, &canvas_id);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    drive_until(
        &mut harness,
        &board,
        |b| b.placements.len() == 2 && !b.loading && b.error.is_none(),
        "mounted canvas self-fetches the two real-SurrealDB placements",
    );

    let artifact_dir = external_artifact_dir("wp-kernel-012-mt-026/canonical-argus");
    std::fs::create_dir_all(&artifact_dir).expect("create external MT-026 Argus artifact dir");

    let mut argus = CanonicalArgusDriver::bind(harness.state(), "wp-kernel-012-mt-026-canvas");

    // (1) Canonical inspect: both real-SurrealDB placement cards are addressable, plus the fixed toolbar controls.
    let author_one = placement_author_id(&placement_one);
    let author_two = placement_author_id(&placement_two);
    let before = argus.inspect(&mut harness);
    for author in [&author_one, &author_two] {
        assert!(
            json_has_author_id(&before, author),
            "canonical argus.inspect must see the mounted real-SurrealDB placement card '{author}'"
        );
    }
    for control in [
        PAN_LEFT_AUTHOR_ID,
        PAN_RIGHT_AUTHOR_ID,
        ZOOM_IN_AUTHOR_ID,
        ZOOM_OUT_AUTHOR_ID,
        ZOOM_VALUE_AUTHOR_ID,
        ADD_CARD_AUTHOR_ID,
        EDGE_MODE_AUTHOR_ID,
        START_EDGE_AUTHOR_ID,
        STATUS_AUTHOR_ID,
    ] {
        assert!(
            json_has_author_id(&before, control),
            "canonical argus.inspect must see the fixed canvas control '{control}'"
        );
    }

    // Both durable completion observers must be addressable BEFORE any action: `crate::mcp::action`
    // registers an observer-backed completion only when the declaration and its observer are both
    // visible in the exact pre-dispatch tree.
    for observer in [
        VIEWPORT_COMPLETION_AUTHOR_ID,
        PLACEMENT_MUTATION_COMPLETION_AUTHOR_ID,
    ] {
        assert!(
            json_has_author_id(&before, observer),
            "canonical argus.inspect must see the durable completion observer '{observer}'"
        );
    }

    // (2) Safe control steer: zoom in through the real Argus transport; the placements remain addressable.
    let zoom = argus.click_and_reinspect(&mut harness, ZOOM_IN_AUTHOR_ID);
    assert_eq!(
        zoom.receipt_status, "applied",
        "MT-026 V4: the canonical zoom-in receipt must be TERMINAL and NON-INDETERMINATE — a plausible \
         post-state is not causal proof (validation_v4 fail_report)"
    );
    assert!(
        zoom.agent_id
            .contains(":client:wp-kernel-012-mt-026-canvas-agent"),
        "the canonical receipt retains the external caller attribution: {}",
        zoom.agent_id
    );
    for author in [&author_one, &author_two] {
        assert!(
            json_has_author_id(&zoom.after, author),
            "after the safe zoom-in the real-SurrealDB placement '{author}' remains addressable"
        );
    }
    assert!(
        json_has_author_id(&zoom.after, ZOOM_VALUE_AUTHOR_ID),
        "the canvas zoom-value control is addressable after the zoom action"
    );

    // MT-026 V4 remediation step 1 + 3: bind canonical completion to the TERMINAL VIEWPORT RECEIPT.
    // The receipt must carry board ID, prior AND resulting viewport revision/scale/offset, the action
    // id, and the authoritative persisted state — and the resulting board generation must be FRESH.
    let zoom_receipt_id = zoom.receipt_id;
    let zoom_board_id = canvas_id.clone();
    let zoom_workspace_id = workspace_id.clone();
    let zoom_terminal = argus.assert_latest_terminal_predicate_with_evidence(
        &mut harness,
        "mt026.viewport.terminal-receipt.zoom-in",
        serde_json::json!({
            "receipt_id": zoom_receipt_id,
            "board_id": canvas_id,
            "workspace_id": workspace_id,
            "target": ZOOM_IN_AUTHOR_ID,
            "zoom_step": 0.25,
        }),
        move |after| {
            let Some(receipt) = receipt_for(after, zoom_receipt_id) else {
                return false;
            };
            if receipt["status"].as_str() != Some("applied") {
                return false;
            }
            let Some(token) = completion_token(receipt) else {
                return false;
            };
            if token["schema"].as_str() != Some(CLICK_COMPLETION_SCHEMA)
                || token["mode"].as_str() != Some("observer")
                || token["state"].as_str() != Some("applied")
                || token["effect"].as_str() != Some("canvas-viewport")
                || token["pending_target"].as_str() != Some(ZOOM_IN_AUTHOR_ID)
            {
                return false;
            }
            let (Some(semantic), Some(detail)) = (
                nested_json(&token, "semantic_value"),
                nested_json(&token, "terminal_detail"),
            ) else {
                return false;
            };
            let Some(prior_scale) = semantic["prior"]["scale"].as_f64() else {
                return false;
            };
            semantic["schema_id"].as_str() == Some(VIEWPORT_ACTION_SEMANTIC_SCHEMA)
                && semantic["board_id"].as_str() == Some(zoom_board_id.as_str())
                && semantic["workspace_id"].as_str() == Some(zoom_workspace_id.as_str())
                && semantic["action"].as_str() == Some(ZOOM_IN_AUTHOR_ID)
                && semantic["action_id"]
                    .as_str()
                    .is_some_and(|id| !id.is_empty())
                && semantic["prior"]["offset_x"].as_f64().is_some()
                && semantic["prior"]["offset_y"].as_f64().is_some()
                && close(semantic["requested"]["scale"].as_f64(), prior_scale + 0.25)
                && detail["schema_id"].as_str() == Some(VIEWPORT_COMPLETION_DETAIL_SCHEMA)
                && detail["action_id"] == semantic["action_id"]
                && detail["board_id"].as_str() == Some(zoom_board_id.as_str())
                && detail["authority"].as_str() == Some("persisted")
                && detail["persist_route"].as_str().is_some_and(|route| {
                    route.starts_with("PUT /workspaces/") && route.ends_with("/viewport")
                })
                && close(detail["resulting"]["scale"].as_f64(), prior_scale + 0.25)
                && detail["resulting"]["offset_x"].as_f64().is_some()
                && detail["resulting"]["offset_y"].as_f64().is_some()
                && advanced(
                    detail["resulting"]["viewport_revision"].as_u64(),
                    semantic["prior"]["viewport_revision"].as_u64(),
                )
                && advanced(
                    detail["resulting"]["board_generation"].as_u64(),
                    semantic["prior"]["board_generation"].as_u64(),
                )
        },
    );
    let zoom_observation = argus.latest_terminal_observation();
    assert_ne!(
        zoom_observation.receipt_status, "indeterminate",
        "MT-026 V4: the persisted zoom receipt must not be indeterminate"
    );

    // (3) Mutation steer: remove placement one through the real Argus transport. The canonical driver
    // now blocks until the receipt TERMINALIZES, which the product only does after (a) an authoritative
    // refreshed board proves the placement absent at a NEW board generation and (b) an explicit
    // getLoomBlock proves the SOURCE block survived. Target disappearance alone can never terminalize it.
    let remove_author = placement_remove_author_id(&placement_one);
    let remove = argus.click_and_reinspect(&mut harness, &remove_author);
    assert_eq!(
        remove.receipt_status, "applied",
        "MT-026 V4: the canonical placement-removal receipt must be TERMINAL and NON-INDETERMINATE; receipts: {:?}; target: {:?}",
        remove.after["action_receipts"],
        json_node_by_author_id(&remove.after, &remove_author)
    );
    drive_until(
        &mut harness,
        &board,
        |b| {
            b.placements
                .iter()
                .all(|p| p.placement_id != placement_one)
                && b.placements.iter().any(|p| p.placement_id == placement_two)
        },
        "mounted canvas removes placement one via the real backend DELETE + re-fetch, keeping placement two",
    );
    let after_remove = argus.inspect(&mut harness);
    assert!(
        !json_has_author_id(&after_remove, &author_one),
        "fresh canonical re-inspection must NOT see the removed placement card '{author_one}'"
    );
    assert!(
        json_has_author_id(&after_remove, &author_two),
        "the sibling real-SurrealDB placement card '{author_two}' remains addressable after the removal"
    );
    // Source retention: the placement removal keeps the source LoomBlock (getLoomBlock still 200).
    let source_status = live.get_status(&format!(
        "/workspaces/{workspace_id}/loom/blocks/{source_one}"
    ));
    assert_eq!(
        source_status, 200,
        "removing a placement must NOT delete its source LoomBlock (source retention)"
    );

    // MT-026 V4 remediation step 2 + 3: bind canonical completion to the TERMINAL PLACEMENT-REMOVAL
    // RECEIPT. The receipt must carry workspace/board/placement/block ids, the mutation revision pair,
    // the backend route correlation, the authoritative refreshed placement ABSENCE, and the EXPLICIT
    // source-block existence confirmation.
    let remove_receipt_id = remove.receipt_id;
    let remove_target = remove_author.clone();
    let remove_board_id = canvas_id.clone();
    let remove_workspace_id = workspace_id.clone();
    let removed_placement = placement_one.clone();
    let removed_block = source_one.clone();
    let author_one_gone = author_one.clone();
    let author_two_kept = author_two.clone();
    let remove_terminal = argus.assert_latest_terminal_predicate_with_evidence(
        &mut harness,
        "mt026.placement-removal.terminal-receipt",
        serde_json::json!({
            "receipt_id": remove_receipt_id,
            "workspace_id": workspace_id,
            "board_id": canvas_id,
            "placement_id": placement_one,
            "block_id": source_one,
            "target": remove_author,
        }),
        move |after| {
            let Some(receipt) = receipt_for(after, remove_receipt_id) else {
                return false;
            };
            if receipt["status"].as_str() != Some("applied") {
                return false;
            }
            let Some(token) = completion_token(receipt) else {
                return false;
            };
            if token["schema"].as_str() != Some(CLICK_COMPLETION_SCHEMA)
                || token["mode"].as_str() != Some("observer")
                || token["state"].as_str() != Some("applied")
                || token["effect"].as_str() != Some("canvas-placement-mutation")
                || token["pending_target"].as_str() != Some(remove_target.as_str())
            {
                return false;
            }
            let (Some(semantic), Some(detail)) = (
                nested_json(&token, "semantic_value"),
                nested_json(&token, "terminal_detail"),
            ) else {
                return false;
            };
            semantic["schema_id"].as_str() == Some(PLACEMENT_REMOVAL_SEMANTIC_SCHEMA)
                && semantic["workspace_id"].as_str() == Some(remove_workspace_id.as_str())
                && semantic["board_id"].as_str() == Some(remove_board_id.as_str())
                && semantic["placement_id"].as_str() == Some(removed_placement.as_str())
                && semantic["block_id"].as_str() == Some(removed_block.as_str())
                && detail["schema_id"].as_str() == Some(PLACEMENT_REMOVAL_DETAIL_SCHEMA)
                && detail["action_id"] == semantic["action_id"]
                && detail["placement_id"].as_str() == Some(removed_placement.as_str())
                && detail["block_id"].as_str() == Some(removed_block.as_str())
                && detail["backend"]["route"].as_str().is_some_and(|route| {
                    route.starts_with("DELETE /workspaces/")
                        && route.ends_with(removed_placement.as_str())
                })
                && detail["backend"]["source_probe_route"]
                    .as_str()
                    .is_some_and(|route| route.ends_with(removed_block.as_str()))
                && detail["placement_absent_after_refresh"].as_bool() == Some(true)
                && detail["source_block_present"].as_bool() == Some(true)
                && detail["source_block_content_type"].as_str().is_some()
                && advanced(
                    detail["mutation_revision"]["refreshed_board_generation"].as_u64(),
                    semantic["prior_board_generation"].as_u64(),
                )
                // The receipt is bound to a FRESH authoritative projection: the removed card is gone
                // from the exact terminal tree while its sibling remains addressable.
                && !json_has_author_id(after, &author_one_gone)
                && json_has_author_id(after, &author_two_kept)
        },
    );
    let remove_observation = argus.latest_terminal_observation();
    assert_ne!(
        remove_observation.receipt_status, "indeterminate",
        "MT-026 V4: the placement-removal receipt must not be indeterminate"
    );

    // (4) Evidence: before/after canonical trees + the two terminal receipts + a screenshot marker.
    let tree_path = artifact_dir.join("mt026-mounted-canvas-argus.json");
    std::fs::write(
        &tree_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "workspace_id": workspace_id,
            "canvas_block_id": canvas_id,
            "placement_one": placement_one,
            "placement_two": placement_two,
            "before": before,
            "after_zoom": zoom.after,
            "after_remove": after_remove,
            "zoom_receipt_status": zoom_observation.receipt_status,
            "remove_receipt_status": remove_observation.receipt_status,
            "zoom_terminal_receipt": receipt_for(&zoom_terminal, zoom_receipt_id),
            "remove_terminal_receipt": receipt_for(&remove_terminal, remove_receipt_id),
            "zoom_terminal_predicates": zoom_observation.terminal_predicates,
            "remove_terminal_predicates": remove_observation.terminal_predicates,
            "source_one_status_after_remove": source_status,
        }))
        .expect("serialize canonical MT-026 canvas tree evidence"),
    )
    .expect("write canonical MT-026 canvas tree evidence externally");
    assert!(tree_path.is_file());

    let screenshot_marker = match harness.render() {
        Ok(image) => {
            let path = artifact_dir.join("mt026-mounted-canvas.png");
            image.save(&path).expect("save mounted canvas screenshot");
            format!("CAPTURED {}", path.display())
        }
        Err(deferred) => format!("DEFERRED (headless): {deferred}"),
    };
    println!(
        "MT-026 canonical Argus mounted canvas (LIVE SURREALDB workspace={workspace_id} board={canvas_id}): \
         inspect(2 real-SurrealDB placements + controls) -> click({ZOOM_IN_AUTHOR_ID}) -> \
         click({remove_author}) -> reinspect(placement one gone, placement two present, source kept); \
         zoom_receipt={} remove_receipt={} screenshot={} tree={}",
        zoom_observation.receipt_status,
        remove_observation.receipt_status,
        screenshot_marker,
        tree_path.display()
    );

    // STRICT: every canonical action must carry a terminal, NON-INDETERMINATE receipt.
    argus.finish_require_no_indeterminate();
    cleanup.assert_cleaned();
    assert_no_local_artifact_dir();
}

/// Fetch the current Global-graph edge count from real SurrealDB through the real Loom graph client.
fn global_edge_count(client: &LoomGraphClient, workspace_id: &str, generation: u64) -> usize {
    let cell: LoomGraphCell = Arc::new(Mutex::new(std::collections::VecDeque::new()));
    client.fetch_global(workspace_id, generation, Arc::clone(&cell));
    let expected = LoomGraphRequestIdentity::global(generation, workspace_id);
    for _ in 0..200 {
        if let Some(delivery) = cell.lock().unwrap().pop_front() {
            assert_eq!(
                &delivery.request, &expected,
                "global fetch identity matches"
            );
            return delivery
                .result
                .expect("global graph fetch from real SurrealDB succeeds")
                .edges
                .len();
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    panic!("global graph fetch did not resolve within 5s");
}

#[test]
fn mt026_mounted_canvas_canonical_argus_semantic_and_visual_edges() {
    // V2 (edges): prove SEMANTIC and VISUAL canvas edges between placements through canonical Argus over
    // real SurrealDB. Both are driven by a single parameterized swarm dispatch `canvas.add-edge`
    // (`{source_id,target_id,edge_mode}`) — the real localhost MCP transport, not event injection.
    let live = interconnect_support::require_reachable_backend();
    let unique = format!("mt026-argus-edges-{}", unique_suffix());
    let workspace = live.create_workspace(&unique);
    let workspace_id = workspace["id"]
        .as_str()
        .expect("workspace create returns id")
        .to_owned();
    let mut cleanup = LiveWorkspaceCleanup {
        backend: &live,
        workspace_id: workspace_id.clone(),
        cleaned: false,
    };

    let create_block = |title: &str| {
        let block = live.post_json(
            &format!("/workspaces/{workspace_id}/loom/blocks"),
            &serde_json::json!({ "content_type": "note", "title": title }),
        );
        block["block_id"]
            .as_str()
            .expect("Loom block create returns block_id")
            .to_owned()
    };
    let source_one = create_block("MT-026 Argus edge source one");
    let source_two = create_block("MT-026 Argus edge source two");
    let canvas = live.post_json(
        &format!("/workspaces/{workspace_id}/loom/canvas-boards"),
        &serde_json::json!({ "title": format!("MT-026 Argus edges canvas {unique}") }),
    );
    let canvas_id = canvas["block_id"]
        .as_str()
        .expect("canvas create returns block_id")
        .to_owned();
    let place = |placed_block_id: &str, x: f64, y: f64| {
        let placement = live.post_json(
            &format!("/workspaces/{workspace_id}/loom/canvas-boards/{canvas_id}/placements"),
            &serde_json::json!({ "placed_block_id": placed_block_id, "x": x, "y": y, "w": 200.0, "h": 120.0 }),
        );
        placement["placement_id"]
            .as_str()
            .expect("placement create returns placement_id")
            .to_owned()
    };
    let placement_one = place(&source_one, 40.0, 40.0);
    let placement_two = place(&source_two, 360.0, 260.0);

    let (app, rt, board) = canvas_shell(&live.base, &workspace_id, &canvas_id);
    let graph_client = LoomGraphClient::new(live.base.clone(), rt.handle().clone());
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    drive_until(
        &mut harness,
        &board,
        |b| b.placements.len() == 2 && !b.loading && b.error.is_none(),
        "mounted canvas self-fetches the two real-SurrealDB placements",
    );

    // Baseline: no loom edges and no visual edges yet.
    assert_eq!(
        global_edge_count(&graph_client, &workspace_id, 1),
        0,
        "baseline: real SurrealDB has zero loom edges before the semantic-edge dispatch"
    );
    assert_eq!(
        board.lock().unwrap().visual_edges.len(),
        0,
        "baseline: no visual edges"
    );

    let artifact_dir = external_artifact_dir("wp-kernel-012-mt-026/canonical-argus");
    std::fs::create_dir_all(&artifact_dir).expect("create external MT-026 Argus artifact dir");
    let mut argus =
        CanonicalArgusDriver::bind(harness.state(), "wp-kernel-012-mt-026-canvas-edges");

    let before = argus.inspect(&mut harness);
    assert!(
        json_has_author_id(&before, "canvas.add-edge"),
        "the canonical canvas.add-edge swarm control is addressable in the mounted tree"
    );

    // (1) SEMANTIC edge between the two source BLOCKS via parameterized canonical Argus dispatch.
    let semantic = argus.click_with_payload_and_reinspect(
        &mut harness,
        "canvas.add-edge",
        serde_json::json!({
            "source_id": source_one,
            "target_id": source_two,
            "edge_mode": "semantic"
        }),
    );
    assert!(
        matches!(
            semantic.receipt_status.as_str(),
            "applied" | "indeterminate"
        ),
        "the canonical semantic add-edge receipt is terminal: {}",
        semantic.receipt_status
    );
    // Drive the host until the real POST /loom/edges persists, observed via a fresh real-SurrealDB graph fetch.
    let mut generation = 2u64;
    let deadline = Instant::now() + Duration::from_secs(30);
    let semantic_edges = loop {
        harness.run_steps(2);
        let count = global_edge_count(&graph_client, &workspace_id, generation);
        generation += 1;
        if count == 1 {
            break count;
        }
        assert!(
            Instant::now() < deadline,
            "canonical semantic edge did not persist to real SurrealDB within 30s (edges={count})"
        );
        std::thread::sleep(Duration::from_millis(25));
    };
    assert_eq!(
        semantic_edges, 1,
        "canonical Argus semantic add-edge persisted exactly one real loom edge in SurrealDB"
    );
    // Bind the canonical action to a terminal re-observation. The CAUSAL proof for this action is the
    // real-SurrealDB edge count asserted above (carried as evidence); the tree predicate pins that the exact
    // endpoint cards and the dispatching control survive the mutation in the authoritative terminal tree.
    let semantic_endpoint_one = placement_author_id(&placement_one);
    let semantic_endpoint_two = placement_author_id(&placement_two);
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut harness,
        "mt026.semantic-edge.persisted-in-surrealdb",
        serde_json::json!({
            "loom_edges_in_surrealdb": semantic_edges,
            "source_block_one": source_one,
            "source_block_two": source_two,
        }),
        move |after| {
            json_has_author_id(after, &semantic_endpoint_one)
                && json_has_author_id(after, &semantic_endpoint_two)
                && json_has_author_id(after, "canvas.add-edge")
        },
    );

    // (2) VISUAL edge between the two PLACEMENTS via parameterized canonical Argus dispatch.
    let visual = argus.click_with_payload_and_reinspect(
        &mut harness,
        "canvas.add-edge",
        serde_json::json!({
            "source_id": placement_one,
            "target_id": placement_two,
            "edge_mode": "visual"
        }),
    );
    assert!(
        matches!(visual.receipt_status.as_str(), "applied" | "indeterminate"),
        "the canonical visual add-edge receipt is terminal: {}",
        visual.receipt_status
    );
    drive_until(
        &mut harness,
        &board,
        |b| b.visual_edges.len() == 1 && !b.loading,
        "mounted canvas re-fetch reflects the persisted visual edge from real SurrealDB",
    );
    // Independent backend confirmation of the persisted visual edge.
    let board_json = live.get_json(&format!(
        "/workspaces/{workspace_id}/loom/canvas-boards/{canvas_id}"
    ));
    let visual_count = board_json["visual_edges"]
        .as_array()
        .map(|a| a.len())
        .unwrap_or(0);
    assert_eq!(
        visual_count, 1,
        "backend GET confirms exactly one persisted canvas visual edge: {board_json}"
    );
    let visual_endpoint_one = placement_author_id(&placement_one);
    let visual_endpoint_two = placement_author_id(&placement_two);
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut harness,
        "mt026.visual-edge.persisted-in-surrealdb",
        serde_json::json!({
            "visual_edges_in_surrealdb": visual_count,
            "from_placement_id": placement_one,
            "to_placement_id": placement_two,
        }),
        move |after| {
            json_has_author_id(after, &visual_endpoint_one)
                && json_has_author_id(after, &visual_endpoint_two)
                && json_has_author_id(after, "canvas.add-edge")
        },
    );

    let after = argus.inspect(&mut harness);
    let tree_path = artifact_dir.join("mt026-canvas-edges-argus.json");
    std::fs::write(
        &tree_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "workspace_id": workspace_id,
            "canvas_block_id": canvas_id,
            "source_one": source_one,
            "source_two": source_two,
            "placement_one": placement_one,
            "placement_two": placement_two,
            "semantic_receipt": semantic.receipt_status,
            "visual_receipt": visual.receipt_status,
            "loom_edges_after_semantic": semantic_edges,
            "visual_edges_after_visual": visual_count,
            "after": after,
        }))
        .expect("serialize MT-026 edges tree evidence"),
    )
    .expect("write MT-026 edges tree evidence externally");
    assert!(tree_path.is_file());

    println!(
        "MT-026 canonical Argus canvas edges (LIVE SURREALDB workspace={workspace_id} board={canvas_id}): \
         click(canvas.add-edge semantic) -> real loom edge persisted (edges={semantic_edges}); \
         click(canvas.add-edge visual) -> persisted visual edge (backend visual_edges={visual_count}). tree={}",
        tree_path.display()
    );

    argus.finish();
    cleanup.assert_cleaned();
    assert_no_local_artifact_dir();
}

/// Read one placement object from an independent backend canvas-board GET.
fn backend_placement(board_json: &serde_json::Value, placement_id: &str) -> serde_json::Value {
    board_json["placements"]
        .as_array()
        .expect("backend canvas board GET returns placements array")
        .iter()
        .find(|p| p["placement_id"].as_str() == Some(placement_id))
        .unwrap_or_else(|| {
            panic!("backend board is missing placement {placement_id}: {board_json}")
        })
        .clone()
}

#[test]
fn mt026_mounted_canvas_canonical_argus_move_placement() {
    // V2 (movement): reposition a placement through canonical Argus (`canvas.move-placement`
    // click-with-payload) and prove the new x/y PERSIST to real SurrealDB via an independent backend GET.
    let live = interconnect_support::require_reachable_backend();
    let unique = format!("mt026-argus-move-{}", unique_suffix());
    let workspace = live.create_workspace(&unique);
    let workspace_id = workspace["id"]
        .as_str()
        .expect("workspace create returns id")
        .to_owned();
    let mut cleanup = LiveWorkspaceCleanup {
        backend: &live,
        workspace_id: workspace_id.clone(),
        cleaned: false,
    };

    let source = {
        let block = live.post_json(
            &format!("/workspaces/{workspace_id}/loom/blocks"),
            &serde_json::json!({ "content_type": "note", "title": "MT-026 Argus move source" }),
        );
        block["block_id"].as_str().expect("block_id").to_owned()
    };
    let canvas = live.post_json(
        &format!("/workspaces/{workspace_id}/loom/canvas-boards"),
        &serde_json::json!({ "title": format!("MT-026 Argus move canvas {unique}") }),
    );
    let canvas_id = canvas["block_id"]
        .as_str()
        .expect("canvas block_id")
        .to_owned();
    let placement_id = {
        let placement = live.post_json(
            &format!("/workspaces/{workspace_id}/loom/canvas-boards/{canvas_id}/placements"),
            &serde_json::json!({ "placed_block_id": source, "x": 40.0, "y": 40.0, "w": 200.0, "h": 120.0 }),
        );
        placement["placement_id"]
            .as_str()
            .expect("placement_id")
            .to_owned()
    };
    // Baseline persisted coordinates.
    let baseline = backend_placement(
        &live.get_json(&format!(
            "/workspaces/{workspace_id}/loom/canvas-boards/{canvas_id}"
        )),
        &placement_id,
    );
    assert!(
        (baseline["x"].as_f64().unwrap_or(0.0) - 40.0).abs() < 1.0,
        "baseline persisted x is the seeded 40: {baseline}"
    );

    let (app, _rt, board) = canvas_shell(&live.base, &workspace_id, &canvas_id);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    drive_until(
        &mut harness,
        &board,
        |b| b.placements.len() == 1 && !b.loading && b.error.is_none(),
        "mounted canvas self-fetches the real-SurrealDB placement",
    );

    let artifact_dir = external_artifact_dir("wp-kernel-012-mt-026/canonical-argus");
    std::fs::create_dir_all(&artifact_dir).expect("create external MT-026 Argus artifact dir");
    let mut argus = CanonicalArgusDriver::bind(harness.state(), "wp-kernel-012-mt-026-canvas-move");

    let author = placement_author_id(&placement_id);
    let before = argus.inspect(&mut harness);
    assert!(
        json_has_author_id(&before, "canvas.move-placement"),
        "the canonical canvas.move-placement swarm control is addressable"
    );
    assert!(
        json_has_author_id(&before, &author),
        "the placement card is addressable before the move"
    );

    // Canonical Argus reposition to distinct coordinates.
    let (new_x, new_y) = (520.0_f64, 340.0_f64);
    let mv = argus.click_with_payload_and_reinspect(
        &mut harness,
        "canvas.move-placement",
        serde_json::json!({ "placement_id": placement_id, "x": new_x, "y": new_y }),
    );
    assert!(
        matches!(mv.receipt_status.as_str(), "applied" | "indeterminate"),
        "the canonical move receipt is terminal: {}",
        mv.receipt_status
    );

    // Drive the host until the real PATCH persists the new coordinates, observed via independent GET.
    let deadline = Instant::now() + Duration::from_secs(30);
    let persisted = loop {
        harness.run_steps(2);
        let board_json = live.get_json(&format!(
            "/workspaces/{workspace_id}/loom/canvas-boards/{canvas_id}"
        ));
        let pl = backend_placement(&board_json, &placement_id);
        let x = pl["x"].as_f64().unwrap_or(0.0);
        let y = pl["y"].as_f64().unwrap_or(0.0);
        if (x - new_x).abs() < 1.0 && (y - new_y).abs() < 1.0 {
            break pl;
        }
        assert!(
            Instant::now() < deadline,
            "canonical move did not persist to real SurrealDB within 30s (x={x}, y={y})"
        );
        std::thread::sleep(Duration::from_millis(25));
    };

    // Bind the canonical move action to a terminal re-observation; the persisted x/y read back from an
    // INDEPENDENT backend GET above is the causal evidence.
    let moved_author = author.clone();
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut harness,
        "mt026.move-placement.persisted-in-surrealdb",
        serde_json::json!({
            "placement_id": placement_id,
            "persisted_x": persisted["x"],
            "persisted_y": persisted["y"],
            "requested_x": new_x,
            "requested_y": new_y,
        }),
        move |after| json_has_author_id(after, &moved_author),
    );

    let after = argus.inspect(&mut harness);
    assert!(
        json_has_author_id(&after, &author),
        "the moved placement card remains addressable after the reposition"
    );

    let tree_path = artifact_dir.join("mt026-canvas-move-argus.json");
    std::fs::write(
        &tree_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "workspace_id": workspace_id,
            "canvas_block_id": canvas_id,
            "placement_id": placement_id,
            "baseline_xy": [baseline["x"], baseline["y"]],
            "persisted_xy": [persisted["x"], persisted["y"]],
            "move_receipt": mv.receipt_status,
            "after": after,
        }))
        .expect("serialize MT-026 move evidence"),
    )
    .expect("write MT-026 move evidence externally");
    assert!(tree_path.is_file());

    println!(
        "MT-026 canonical Argus move (LIVE SURREALDB workspace={workspace_id} board={canvas_id}): \
         click(canvas.move-placement {{{new_x},{new_y}}}) -> persisted x={} y={} (baseline x=40,y=40). tree={}",
        persisted["x"], persisted["y"], tree_path.display()
    );

    argus.finish();
    cleanup.assert_cleaned();
    assert_no_local_artifact_dir();
}

#[test]
fn mt026_mounted_canvas_canonical_argus_group_placements() {
    // V2 (grouping): group two placements through canonical Argus (`canvas.group` click-with-payload) and
    // prove the shared group id PERSISTS to real SurrealDB via an independent backend GET.
    let live = interconnect_support::require_reachable_backend();
    let unique = format!("mt026-argus-group-{}", unique_suffix());
    let workspace = live.create_workspace(&unique);
    let workspace_id = workspace["id"]
        .as_str()
        .expect("workspace create returns id")
        .to_owned();
    let mut cleanup = LiveWorkspaceCleanup {
        backend: &live,
        workspace_id: workspace_id.clone(),
        cleaned: false,
    };

    let create_block = |title: &str| {
        let block = live.post_json(
            &format!("/workspaces/{workspace_id}/loom/blocks"),
            &serde_json::json!({ "content_type": "note", "title": title }),
        );
        block["block_id"].as_str().expect("block_id").to_owned()
    };
    let source_one = create_block("MT-026 Argus group source one");
    let source_two = create_block("MT-026 Argus group source two");
    let canvas = live.post_json(
        &format!("/workspaces/{workspace_id}/loom/canvas-boards"),
        &serde_json::json!({ "title": format!("MT-026 Argus group canvas {unique}") }),
    );
    let canvas_id = canvas["block_id"]
        .as_str()
        .expect("canvas block_id")
        .to_owned();
    let place = |placed_block_id: &str, x: f64, y: f64| {
        let placement = live.post_json(
            &format!("/workspaces/{workspace_id}/loom/canvas-boards/{canvas_id}/placements"),
            &serde_json::json!({ "placed_block_id": placed_block_id, "x": x, "y": y, "w": 200.0, "h": 120.0 }),
        );
        placement["placement_id"]
            .as_str()
            .expect("placement_id")
            .to_owned()
    };
    let placement_one = place(&source_one, 40.0, 40.0);
    let placement_two = place(&source_two, 360.0, 260.0);

    let (app, _rt, board) = canvas_shell(&live.base, &workspace_id, &canvas_id);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    drive_until(
        &mut harness,
        &board,
        |b| b.placements.len() == 2 && !b.loading && b.error.is_none(),
        "mounted canvas self-fetches the two real-SurrealDB placements",
    );

    let artifact_dir = external_artifact_dir("wp-kernel-012-mt-026/canonical-argus");
    std::fs::create_dir_all(&artifact_dir).expect("create external MT-026 Argus artifact dir");
    let mut argus =
        CanonicalArgusDriver::bind(harness.state(), "wp-kernel-012-mt-026-canvas-group");

    let before = argus.inspect(&mut harness);
    assert!(
        json_has_author_id(&before, "canvas.group-placements"),
        "the canonical canvas.group swarm control is addressable"
    );
    for pid in [&placement_one, &placement_two] {
        assert!(
            json_has_author_id(&before, &placement_author_id(pid)),
            "placement {pid} is addressable before grouping"
        );
    }

    // Canonical Argus group of two placements (no OS modifier keys).
    let grp = argus.click_with_payload_and_reinspect(
        &mut harness,
        "canvas.group-placements",
        serde_json::json!({ "placement_ids": [placement_one, placement_two] }),
    );
    assert!(
        matches!(grp.receipt_status.as_str(), "applied" | "indeterminate"),
        "the canonical group receipt is terminal: {}",
        grp.receipt_status
    );

    // Drive the host until both placements carry the SAME non-null group id in real SurrealDB.
    let deadline = Instant::now() + Duration::from_secs(30);
    let (g1, g2) = loop {
        harness.run_steps(2);
        let board_json = live.get_json(&format!(
            "/workspaces/{workspace_id}/loom/canvas-boards/{canvas_id}"
        ));
        let one = backend_placement(&board_json, &placement_one);
        let two = backend_placement(&board_json, &placement_two);
        let g1 = one["group_id"].as_str().map(str::to_owned);
        let g2 = two["group_id"].as_str().map(str::to_owned);
        if g1.is_some() && g1 == g2 {
            break (g1.unwrap(), g2.unwrap());
        }
        assert!(
            Instant::now() < deadline,
            "canonical group did not persist a shared group id within 30s (g1={g1:?}, g2={g2:?})"
        );
        std::thread::sleep(Duration::from_millis(25));
    };
    assert_eq!(
        g1, g2,
        "both placements persist the SAME group id in real SurrealDB"
    );

    // Bind the canonical group action to a terminal re-observation whose predicate is recomputable from
    // the tree alone: BOTH placement cards must expose the exact PERSISTED group id in their AccessKit
    // value (AC6 `data-group-id`), not merely still exist.
    let grouped_author_one = placement_author_id(&placement_one);
    let grouped_author_two = placement_author_id(&placement_two);
    let expected_group_value = format!("group_id={g1}");
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut harness,
        "mt026.group-placements.shared-group-id-in-tree",
        serde_json::json!({
            "persisted_group_id": g1,
            "placement_one": placement_one,
            "placement_two": placement_two,
        }),
        move |after| {
            [&grouped_author_one, &grouped_author_two]
                .iter()
                .all(|author| {
                    canonical_argus_driver::json_node_by_author_id(after, author)
                        .and_then(|node| node["value"].as_str())
                        == Some(expected_group_value.as_str())
                })
        },
    );

    let after = argus.inspect(&mut harness);
    for pid in [&placement_one, &placement_two] {
        assert!(
            json_has_author_id(&after, &placement_author_id(pid)),
            "placement {pid} remains addressable after grouping"
        );
    }

    let tree_path = artifact_dir.join("mt026-canvas-group-argus.json");
    std::fs::write(
        &tree_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "workspace_id": workspace_id,
            "canvas_block_id": canvas_id,
            "placement_one": placement_one,
            "placement_two": placement_two,
            "persisted_group_id": g1,
            "group_receipt": grp.receipt_status,
            "after": after,
        }))
        .expect("serialize MT-026 group evidence"),
    )
    .expect("write MT-026 group evidence externally");
    assert!(tree_path.is_file());

    println!(
        "MT-026 canonical Argus group (LIVE SURREALDB workspace={workspace_id} board={canvas_id}): \
         click(canvas.group-placements [p1,p2]) -> both placements persist shared group_id={g1}. tree={}",
        tree_path.display()
    );

    argus.finish();
    cleanup.assert_cleaned();
    assert_no_local_artifact_dir();
}

#[test]
fn mt026_mounted_canvas_empty_state_canonical_argus() {
    // AC10 against a REAL managed-SurrealDB canvas board with zero placements: renders + inspects through
    // canonical Argus with no placement cards and no panic.
    let live = interconnect_support::require_reachable_backend();
    let unique = format!("mt026-argus-empty-{}", unique_suffix());
    let workspace = live.create_workspace(&unique);
    let workspace_id = workspace["id"]
        .as_str()
        .expect("workspace create returns id")
        .to_owned();
    let mut cleanup = LiveWorkspaceCleanup {
        backend: &live,
        workspace_id: workspace_id.clone(),
        cleaned: false,
    };
    let canvas = live.post_json(
        &format!("/workspaces/{workspace_id}/loom/canvas-boards"),
        &serde_json::json!({ "title": format!("MT-026 Argus empty canvas {unique}") }),
    );
    let canvas_id = canvas["block_id"]
        .as_str()
        .expect("canvas create returns block_id")
        .to_owned();

    let (app, _rt, board) = canvas_shell(&live.base, &workspace_id, &canvas_id);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1000.0, 700.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    drive_until(
        &mut harness,
        &board,
        |b| b.placements.is_empty() && !b.loading && b.error.is_none(),
        "mounted canvas self-fetches a confirmed empty real-SurrealDB board",
    );

    let mut argus =
        CanonicalArgusDriver::bind(harness.state(), "wp-kernel-012-mt-026-canvas-empty");
    let tree = argus.inspect(&mut harness);

    let mut ids = Vec::new();
    collect_author_ids(&tree, &mut ids);
    assert!(
        !ids.iter().any(|id| id.starts_with("canvas.placement.")),
        "empty real-SurrealDB canvas board must expose NO placement cards through canonical Argus; got {:?}",
        ids.iter().filter(|id| id.starts_with("canvas.")).collect::<Vec<_>>()
    );
    assert!(
        json_has_author_id(&tree, ADD_CARD_AUTHOR_ID),
        "the fixed canvas toolbar controls remain addressable on an empty board"
    );

    println!(
        "MT-026 canonical Argus empty canvas (LIVE SURREALDB workspace={workspace_id} board={canvas_id}): \
         inspect() returned {} author_ids, 0 canvas.placement.* (AC10)",
        ids.len()
    );

    argus.finish();
    cleanup.assert_cleaned();
    assert_no_local_artifact_dir();
}

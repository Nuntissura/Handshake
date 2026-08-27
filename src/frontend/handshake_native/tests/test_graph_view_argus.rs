//! WP-KERNEL-012 E3 MT-021 remediation (FAIL_V2): canonical Argus inspect / safe-steer / re-observe
//! proof for the MOUNTED local/global Loom graph view over REAL SurrealDB data.
//!
//! `validation_v2` failed MT-021 because "Local/global graph UI states have no current canonical Argus
//! action and post-action observation covering populated real-SurrealDB graph data. AccessKit or fixture
//! rendering alone does not satisfy the mandatory visual/steering gate." The existing
//! `test_graph_view.rs::graph_view_live_surrealdb_self_seeds_local_global` drives the graph WIDGET (`build_ui`)
//! and dispatches raw AccessKit events — it never drives the MOUNTED `HandshakeApp` through the real
//! localhost `SwarmMcpServer` transport (`argus.inspect` / `argus.click`) the way an out-of-process swarm
//! agent does, and it never re-observes the mounted tree after a canonical action. This test closes that
//! exact gap:
//!
//!   1. seeds a REAL Handshake-managed SurrealDB workspace with four LoomBlocks + two LoomEdges through
//!      the production HTTP routes (`POST /loom/blocks`, `POST /loom/edges`),
//!   2. mounts the production `HandshakeApp` shell with the Graph View pane and lets the app's OWN
//!      per-frame feed (`drive_graph_and_canvas_feeds`) fetch the Global projection from that live
//!      workspace and drain it into the mounted `LoomGraphView` (no injected fixture),
//!   3. binds the CANONICAL Argus driver (real localhost JSON-RPC) to the mounted app,
//!   4. `argus.inspect` proves each real-SurrealDB graph node is addressable by its stable author_id
//!      (`graph.node.{sanitized_block_id}`) plus the fixed global controls,
//!   5. drives ONE safe, reversible action (`graph.relayout`) through the real Argus transport, and
//!   6. FRESH `argus.inspect` re-observes the post-action tree (the real-SurrealDB nodes remain addressable — the
//!      re-layout was additive), then writes before/after tree evidence + a screenshot marker (headless
//!      DEFERRED is an acceptable typed outcome) and deletes the workspace.
//!
//! A second test proves the empty real-SurrealDB workspace renders and inspects through canonical Argus with no
//! graph nodes and no panic (AC7).
//!
//! Requires the `integration` feature and a reachable managed backend (attach on 37501, or an owned
//! `HSK_TEST_BACKEND_BIN` + `HANDSHAKE_DATA_DIR`). It is feature-gated but NOT ignored.
#![cfg(feature = "integration")]

use std::path::{Path, PathBuf};
use std::sync::Arc;
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
use handshake_native::backend_client::HealthInfo;
use handshake_native::command_registry::CMD_VIEW_GRAPH;
use handshake_native::graph::graph_view::{
    node_author_id, MODE_GLOBAL_AUTHOR_ID, MODE_LOCAL_AUTHOR_ID, RELAYOUT_AUTHOR_ID,
    RELAYOUT_STATUS_AUTHOR_ID, ZOOM_IN_AUTHOR_ID, ZOOM_OUT_AUTHOR_ID,
};

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

/// Every `author_id` present anywhere in a canonical Argus inspect tree.
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
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                self.backend.delete_workspace(&self.workspace_id)
            }));
        }
    }
}

fn relayout_observation(tree: &serde_json::Value) -> serde_json::Value {
    let value = json_node_by_author_id(tree, RELAYOUT_STATUS_AUTHOR_ID)
        .and_then(|node| node.get("value"))
        .and_then(serde_json::Value::as_str)
        .expect("graph.relayout.status exposes a machine-readable observation value");
    serde_json::from_str(value).expect("graph.relayout.status observation value is valid JSON")
}

fn relayout_completion(tree: &serde_json::Value) -> serde_json::Value {
    let value = json_node_by_author_id(tree, RELAYOUT_AUTHOR_ID)
        .and_then(|node| node.get("value"))
        .and_then(serde_json::Value::as_str)
        .expect("graph.relayout exposes the click-completion token");
    serde_json::from_str(value).expect("graph.relayout completion token is valid JSON")
}

fn graph_node_bounds(tree: &serde_json::Value, author_id: &str) -> Option<(f64, f64, f64, f64)> {
    let bounds = json_node_by_author_id(tree, author_id)?.get("bounds")?;
    Some((
        bounds.get("x")?.as_f64()?,
        bounds.get("y")?.as_f64()?,
        bounds.get("w")?.as_f64()?,
        bounds.get("h")?.as_f64()?,
    ))
}

fn remove_owned_prior_artifact(path: &Path) {
    match std::fs::remove_file(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => panic!(
            "remove stale owned proof artifact {}: {error}",
            path.display()
        ),
    }
}

/// Mount a production `HandshakeApp` shell bound to the live backend + the seeded workspace, with the
/// Graph View pane opened on the active work surface. The multi-thread runtime is returned so it outlives
/// the harness (the per-frame graph feed dispatches onto it).
fn graph_shell(base: &str, workspace_id: &str) -> (HandshakeApp, tokio::runtime::Runtime) {
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
    assert!(
        app.dispatch_palette_action_for_test(CMD_VIEW_GRAPH),
        "the View Graph command mounts the production Graph View pane"
    );
    (app, runtime)
}

/// Run mounted frames until `condition` holds against the mounted graph view, or panic.
fn drive_until(
    harness: &mut Harness<'_, HandshakeApp>,
    graph: &std::sync::Arc<std::sync::Mutex<handshake_native::graph::graph_view::LoomGraphView>>,
    condition: impl Fn(&handshake_native::graph::graph_view::LoomGraphView) -> bool,
    proof: &str,
) {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        harness.run_steps(2);
        if graph.lock().map(|g| condition(&g)).unwrap_or(false) {
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
fn mt021_mounted_graph_canonical_argus_inspect_steer_reobserve() {
    let artifact_dir = external_artifact_dir("wp-kernel-012-mt-021/canonical-argus");
    let tree_path = artifact_dir.join("mt021-mounted-graph-argus.json");
    let screenshot_path = artifact_dir.join("mt021-mounted-graph.png");
    remove_owned_prior_artifact(&tree_path);
    remove_owned_prior_artifact(&screenshot_path);

    let live = interconnect_support::require_reachable_backend();
    let unique = format!("mt021-argus-{}", unique_suffix());
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

    // Seed a real graph topology: four LoomBlocks (A,B,C,isolated) + two LoomEdges (A-B, B-C).
    let seed_block = |title: &str| {
        let block = live.post_json(
            &format!("/workspaces/{workspace_id}/loom/blocks"),
            &serde_json::json!({ "content_type": "note", "title": title }),
        );
        block["block_id"]
            .as_str()
            .expect("block create returns block_id")
            .to_owned()
    };
    let alpha = seed_block("MT-021 Argus Alpha");
    let beta = seed_block("MT-021 Argus Beta");
    let gamma = seed_block("MT-021 Argus Gamma");
    let isolated = seed_block("MT-021 Argus Isolated");
    for (source, target) in [(&alpha, &beta), (&beta, &gamma)] {
        live.post_json(
            &format!("/workspaces/{workspace_id}/loom/edges"),
            &serde_json::json!({
                "source_block_id": source,
                "target_block_id": target,
                "edge_type": "mention",
                "created_by": "user"
            }),
        );
    }
    let seeded = [alpha.clone(), beta.clone(), gamma.clone(), isolated.clone()];

    // Mount the production shell; the app's own per-frame feed fetches the Global projection from the live
    // workspace and drains it into the mounted graph view.
    let (app, _rt) = graph_shell(&live.base, &workspace_id);
    let graph_view = app.mounted_graph_view();
    let bus_workspace_id = workspace_id.clone();
    let mut bus_prebound = false;
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .build_state(
            move |ctx, app: &mut HandshakeApp| {
                if !bus_prebound {
                    let bus = handshake_native::interop::InteractionBus::get_or_init(ctx);
                    let rebound =
                        handshake_native::interop::InteractionBus::with_try_lock(&bus, |bus| {
                            bus.bind_workspace(&bus_workspace_id)
                        });
                    assert_eq!(
                        rebound,
                        Some(true),
                        "pre-bind the graph workspace before the first shell frame"
                    );
                    bus_prebound = true;
                }
                app.ui(ctx);
            },
            app,
        );
    drive_until(
        &mut harness,
        &graph_view,
        |g| g.nodes.len() == 4 && !g.loading && g.error.is_none() && g.layout_stable(),
        "mounted graph self-fetches and lays out the four real-SurrealDB nodes from the live Global projection",
    );
    assert_eq!(
        graph_view.lock().unwrap().edges.len(),
        2,
        "the mounted graph carries the two real persisted LoomEdges (relationships from real SurrealDB)"
    );

    let mut argus = CanonicalArgusDriver::bind(harness.state(), "wp-kernel-012-mt-021-graph");

    // (1) Canonical inspect: every real-SurrealDB graph node is addressable by stable author_id, and the fixed
    // global controls are present regardless of content (AC6).
    let before = argus.inspect(&mut harness);
    for block_id in &seeded {
        let author = node_author_id(block_id);
        assert!(
            json_has_author_id(&before, &author),
            "canonical argus.inspect must see the mounted real-SurrealDB graph node '{author}'"
        );
    }
    for control in [
        MODE_LOCAL_AUTHOR_ID,
        MODE_GLOBAL_AUTHOR_ID,
        ZOOM_IN_AUTHOR_ID,
        ZOOM_OUT_AUTHOR_ID,
        RELAYOUT_AUTHOR_ID,
        RELAYOUT_STATUS_AUTHOR_ID,
    ] {
        assert!(
            json_has_author_id(&before, control),
            "canonical argus.inspect must see the fixed global graph control '{control}'"
        );
    }
    let before_layout = relayout_observation(&before);
    let before_completion = relayout_completion(&before);
    let generation_before = before_layout["layout_generation"]
        .as_u64()
        .expect("pre-action layout generation");
    assert_eq!(before_layout["layout_status"], "stable");
    assert_eq!(before_layout["node_count"], 4);
    assert_eq!(before_layout["edge_count"], 2);
    assert_eq!(before_completion["schema"], "handshake.click-completion/v1");
    assert_eq!(before_completion["mode"], "same_target");
    assert_eq!(before_completion["effect"], "graph-relayout");
    assert_eq!(before_completion["generation"], generation_before);
    assert_ne!(before_completion["state"], "pending");

    // (2) Safe, reversible steer: re-run the force layout through the real Argus transport. This mutates
    // no durable/backend state — it re-seeds ephemeral node positions in the panel only.
    let observation = argus.click_and_reinspect(&mut harness, RELAYOUT_AUTHOR_ID);
    assert_eq!(
        observation.receipt_status, "applied",
        "the canonical relayout action must carry an exact terminal completion acknowledgement"
    );
    let immediate_completion = relayout_completion(&observation.after);
    assert_eq!(immediate_completion["generation"], generation_before + 1);
    assert_eq!(immediate_completion["state"], "applied");
    assert!(
        observation
            .agent_id
            .contains(":client:wp-kernel-012-mt-021-graph-agent"),
        "the canonical receipt retains the external caller attribution: {}",
        observation.agent_id
    );

    // (3) Drive the real mounted layout to its terminal state, then bind the latest canonical action to
    // the exact +1 generation and authoritative stable digest. Any background graph refresh increments
    // the same epoch and makes this predicate fail rather than being misattributed to the click.
    drive_until(
        &mut harness,
        &graph_view,
        |graph| graph.layout_generation() == generation_before + 1 && graph.layout_stable(),
        "relayout reaches the exact next stable layout generation",
    );
    let (terminal_generation, terminal_digest) = {
        let graph = graph_view.lock().unwrap();
        (graph.layout_generation(), graph.layout_state_sha256())
    };
    assert_eq!(terminal_generation, generation_before + 1);
    assert_eq!(terminal_digest.len(), 64);
    let expected_authors = seeded
        .iter()
        .map(|id| node_author_id(id))
        .collect::<Vec<_>>();
    let predicate_authors = expected_authors.clone();
    let predicate_digest = terminal_digest.clone();
    let terminal = argus.assert_latest_terminal_predicate_with_evidence(
        &mut harness,
        "graph.relayout.exact-next-stable-layout-v1",
        serde_json::json!({
            "before_generation": generation_before,
            "expected_generation": terminal_generation,
            "expected_layout_state_sha256": terminal_digest.clone(),
            "expected_node_authors": expected_authors.clone(),
            "expected_node_count": 4,
            "expected_edge_count": 2,
        }),
        move |tree| {
            let state = relayout_observation(tree);
            let bounds = predicate_authors
                .iter()
                .filter_map(|author| graph_node_bounds(tree, author))
                .collect::<Vec<_>>();
            let distinct_bounds = bounds
                .iter()
                .map(|(x, y, w, h)| (x.to_bits(), y.to_bits(), w.to_bits(), h.to_bits()))
                .collect::<std::collections::HashSet<_>>();
            state["layout_generation"].as_u64() == Some(generation_before + 1)
                && state["layout_status"] == "stable"
                && state["layout_state_sha256"].as_str() == Some(predicate_digest.as_str())
                && state["node_count"].as_u64() == Some(4)
                && state["edge_count"].as_u64() == Some(2)
                && predicate_authors
                    .iter()
                    .all(|author| json_has_author_id(tree, author))
                && bounds.len() == predicate_authors.len()
                && bounds.iter().all(|(x, y, w, h)| {
                    x.is_finite()
                        && y.is_finite()
                        && w.is_finite()
                        && h.is_finite()
                        && *w > 0.0
                        && *h > 0.0
                })
                && distinct_bounds.len() == predicate_authors.len()
        },
    );
    let terminal_layout = relayout_observation(&terminal);

    // Finish validates that the action has a refreshed terminal snapshot and a passing product-state
    // predicate. Only after that succeeds do we publish the terminal proof artifacts.
    let receipt_id = observation.receipt_id;
    let receipt_status = observation.receipt_status.clone();
    let agent_id = observation.agent_id.clone();
    let immediate_after = observation.after.clone();
    let rendered = harness.render();
    cleanup.assert_cleaned();
    argus.finish();

    // (4) Evidence: before/immediate/terminal canonical trees + a post-finish screenshot marker.
    std::fs::create_dir_all(&artifact_dir).expect("create external MT-021 Argus artifact dir");
    std::fs::write(
        &tree_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "workspace_id": &workspace_id,
            "seeded_blocks": &seeded,
            "before": before,
            "immediate_after": immediate_after,
            "terminal_after": terminal,
            "before_layout": before_layout,
            "terminal_layout": terminal_layout,
            "receipt_id": receipt_id,
            "receipt_status": &receipt_status,
            "agent_id": &agent_id,
        }))
        .expect("serialize canonical MT-021 graph tree evidence"),
    )
    .expect("write canonical MT-021 graph tree evidence externally");
    assert!(tree_path.is_file());

    let screenshot_marker = match rendered {
        Ok(image) => {
            image
                .save(&screenshot_path)
                .expect("save mounted graph screenshot");
            format!("CAPTURED {}", screenshot_path.display())
        }
        Err(deferred) => format!("DEFERRED (headless): {deferred}"),
    };
    println!(
        "MT-021 canonical Argus mounted graph (LIVE SURREALDB workspace={workspace_id}): \
         inspect(4 real-SurrealDB nodes + 5 controls) -> click({RELAYOUT_AUTHOR_ID}) \
         -> exact +1 stable generation={} digest={} with 4 nodes; receipt={} agent={} screenshot={} tree={}",
        terminal_generation,
        terminal_digest,
        receipt_status,
        agent_id,
        screenshot_marker,
        tree_path.display()
    );
    assert_no_local_artifact_dir();
}

#[test]
fn mt021_mounted_graph_canonical_argus_local_global_switch_distinct_queries() {
    // V2 R1 (distinct local vs global queries) + R2 (safe switching): drive the local/global mode control
    // through canonical Argus and prove the switch produces a DISTINCT real-SurrealDB query result, not a
    // client-side re-render. Global returns all 4 seeded nodes; Local (focused on beta) returns only
    // beta's real neighbourhood {alpha,beta,gamma} — the disconnected `isolated` block is a real seeded
    // node that a client-side filter could not remove but a distinct backend /loom/graph/local query does.
    let artifact_dir = external_artifact_dir("wp-kernel-012-mt-021/canonical-argus");
    let tree_path = artifact_dir.join("mt021-graph-local-global-switch-argus.json");
    remove_owned_prior_artifact(&tree_path);

    let live = interconnect_support::require_reachable_backend();
    let unique = format!("mt021-argus-switch-{}", unique_suffix());
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

    let seed_block = |title: &str| {
        let block = live.post_json(
            &format!("/workspaces/{workspace_id}/loom/blocks"),
            &serde_json::json!({ "content_type": "note", "title": title }),
        );
        block["block_id"]
            .as_str()
            .expect("block create returns block_id")
            .to_owned()
    };
    let alpha = seed_block("MT-021 Switch Alpha");
    let beta = seed_block("MT-021 Switch Beta");
    let gamma = seed_block("MT-021 Switch Gamma");
    let isolated = seed_block("MT-021 Switch Isolated");
    for (source, target) in [(&alpha, &beta), (&beta, &gamma)] {
        live.post_json(
            &format!("/workspaces/{workspace_id}/loom/edges"),
            &serde_json::json!({
                "source_block_id": source,
                "target_block_id": target,
                "edge_type": "mention",
                "created_by": "user"
            }),
        );
    }

    let (app, _rt) = graph_shell(&live.base, &workspace_id);
    let graph_view = app.mounted_graph_view();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    drive_until(
        &mut harness,
        &graph_view,
        |g| g.nodes.len() == 4 && !g.loading && g.error.is_none(),
        "mounted graph self-fetches the four real-SurrealDB Global nodes",
    );

    let mut argus =
        CanonicalArgusDriver::bind(harness.state(), "wp-kernel-012-mt-021-graph-switch");

    // (1) Global inspect: all four seeded nodes are addressable, including the disconnected `isolated`.
    let global_before = argus.inspect(&mut harness);
    let author = |id: &str| node_author_id(id);
    for id in [&alpha, &beta, &gamma, &isolated] {
        assert!(
            json_has_author_id(&global_before, &author(id)),
            "Global canonical inspect sees every seeded node incl. isolated ('{}')",
            author(id)
        );
    }

    // (2) Canonically SELECT the focus node (no navigation) via a parameterized swarm dispatch, then
    // canonically SWITCH to Local mode. The host reads the selected focus and issues a real /loom/graph
    // local neighbourhood query.
    let select_beta = argus.click_with_payload_and_reinspect(
        &mut harness,
        "graph.select-node",
        serde_json::json!({ "block_id": &beta }),
    );
    let beta_for_predicate = beta.clone();
    let beta_author_for_predicate = author(&beta);
    let selected_graph = Arc::clone(&graph_view);
    let selected_tree = argus.assert_latest_terminal_predicate_with_evidence(
        &mut harness,
        "graph.select-node.exact-beta-v1",
        serde_json::json!({ "block_id": &beta }),
        move |tree| {
            selected_graph
                .lock()
                .map(|graph| graph.selected.as_deref() == Some(beta_for_predicate.as_str()))
                .unwrap_or(false)
                && json_has_author_id(tree, &beta_author_for_predicate)
        },
    );
    let switch_local = argus.click_and_reinspect(&mut harness, MODE_LOCAL_AUTHOR_ID);
    assert!(
        matches!(
            switch_local.receipt_status.as_str(),
            "applied" | "indeterminate"
        ),
        "the canonical local-switch receipt is terminal: {}",
        switch_local.receipt_status
    );
    let isolated_for_local = isolated.clone();
    drive_until(
        &mut harness,
        &graph_view,
        move |g| {
            !g.loading
                && g.error.is_none()
                && g.nodes.len() == 3
                && g.nodes.iter().all(|n| n.block_id != isolated_for_local)
        },
        "canonical local switch issues a distinct real-SurrealDB neighbourhood query (3 nodes, no isolated)",
    );

    // (3) Bind the Local action to the DISTINCT terminal real-SurrealDB projection and stable layout.
    let local_expected = [author(&alpha), author(&beta), author(&gamma)];
    let local_isolated = author(&isolated);
    let local_graph = Arc::clone(&graph_view);
    let local_tree = argus.assert_latest_terminal_predicate_with_evidence(
        &mut harness,
        "graph.mode.local.exact-neighbourhood-v1",
        serde_json::json!({
            "expected_node_authors": &local_expected,
            "excluded_node_author": &local_isolated,
            "expected_node_count": 3,
        }),
        move |tree| {
            local_graph
                .lock()
                .map(|graph| {
                    matches!(
                        &graph.mode,
                        handshake_native::graph::graph_view::GraphMode::Local { .. }
                    ) && graph.nodes.len() == 3
                        && graph.layout_stable()
                        && graph.error.is_none()
                })
                .unwrap_or(false)
                && local_expected
                    .iter()
                    .all(|expected| json_has_author_id(tree, expected))
                && !json_has_author_id(tree, &local_isolated)
        },
    );
    for id in [&alpha, &beta, &gamma] {
        assert!(
            json_has_author_id(&local_tree, &author(id)),
            "Local canonical inspect sees the neighbourhood node '{}'",
            author(id)
        );
    }
    assert!(
        !json_has_author_id(&local_tree, &author(&isolated)),
        "Local canonical inspect must NOT see the disconnected 'isolated' node — the switch produced a \
         distinct real-SurrealDB neighbourhood query, not a re-render of the global set"
    );

    // (4) Canonically SWITCH back to Global; the distinct query restores the full set incl. isolated.
    let switch_global = argus.click_and_reinspect(&mut harness, MODE_GLOBAL_AUTHOR_ID);
    assert!(
        matches!(
            switch_global.receipt_status.as_str(),
            "applied" | "indeterminate"
        ),
        "the canonical global-switch receipt is terminal: {}",
        switch_global.receipt_status
    );
    drive_until(
        &mut harness,
        &graph_view,
        |g| g.nodes.len() == 4 && !g.loading && g.error.is_none(),
        "canonical global switch re-queries the full real-SurrealDB projection (4 nodes)",
    );
    let isolated_author = author(&isolated);
    let global_graph = Arc::clone(&graph_view);
    let global_after = argus.assert_latest_terminal_predicate_with_evidence(
        &mut harness,
        "graph.mode.global.exact-full-projection-v1",
        serde_json::json!({
            "expected_node_count": 4,
            "required_isolated_author": &isolated_author,
        }),
        move |tree| {
            global_graph
                .lock()
                .map(|graph| {
                    matches!(
                        &graph.mode,
                        handshake_native::graph::graph_view::GraphMode::Global
                    ) && graph.nodes.len() == 4
                        && graph.layout_stable()
                        && graph.error.is_none()
                })
                .unwrap_or(false)
                && json_has_author_id(tree, &isolated_author)
        },
    );
    assert!(
        json_has_author_id(&global_after, &author(&isolated)),
        "switching back to Global re-queries real SurrealDB and the disconnected 'isolated' node returns"
    );

    let select_receipt = select_beta.receipt_status.clone();
    let local_receipt = switch_local.receipt_status.clone();
    let global_receipt = switch_global.receipt_status.clone();
    cleanup.assert_cleaned();
    argus.finish();
    std::fs::create_dir_all(&artifact_dir).expect("create external MT-021 Argus artifact dir");
    std::fs::write(
        &tree_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "workspace_id": workspace_id,
            "seeded": { "alpha": alpha, "beta": beta, "gamma": gamma, "isolated": isolated },
            "global_before": global_before,
            "selected_beta": selected_tree,
            "local_after_switch": local_tree,
            "global_after_switch_back": global_after,
            "select_receipt": select_receipt,
            "local_switch_receipt": local_receipt,
            "global_switch_receipt": global_receipt,
        }))
        .expect("serialize MT-021 switch tree evidence"),
    )
    .expect("write MT-021 switch tree evidence externally");
    assert!(tree_path.is_file());

    println!(
        "MT-021 canonical Argus local/global switch (LIVE SURREALDB workspace={workspace_id}): \
         Global inspect(4 incl isolated) -> select-node(beta) + click({MODE_LOCAL_AUTHOR_ID}) \
         -> Local inspect(3 neighbourhood, isolated ABSENT = distinct real-SurrealDB query) \
         -> click({MODE_GLOBAL_AUTHOR_ID}) -> Global inspect(isolated returns). tree={}",
        tree_path.display()
    );
    assert_no_local_artifact_dir();
}

#[test]
fn mt021_mounted_graph_empty_state_canonical_argus() {
    // AC7 against a REAL unseeded managed-SurrealDB workspace: the mounted panel renders + inspects through
    // canonical Argus with no graph nodes and no panic.
    let live = interconnect_support::require_reachable_backend();
    let unique = format!("mt021-argus-empty-{}", unique_suffix());
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

    let (app, _rt) = graph_shell(&live.base, &workspace_id);
    let graph_view = app.mounted_graph_view();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1000.0, 700.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    // Drive until the initial fetch resolves to a confirmed empty projection (no nodes, not loading).
    drive_until(
        &mut harness,
        &graph_view,
        |g| g.nodes.is_empty() && !g.loading && g.error.is_none(),
        "mounted graph self-fetches an empty projection from a fresh real-SurrealDB workspace",
    );

    let mut argus = CanonicalArgusDriver::bind(harness.state(), "wp-kernel-012-mt-021-graph-empty");
    let tree = argus.inspect(&mut harness);

    let mut ids = Vec::new();
    collect_author_ids(&tree, &mut ids);
    assert!(
        !ids.iter().any(|id| id.starts_with("graph.node.")),
        "empty real-SurrealDB workspace mounted graph must expose NO graph nodes through canonical Argus; got {:?}",
        ids.iter().filter(|id| id.starts_with("graph.")).collect::<Vec<_>>()
    );
    // The fixed global controls are still present at zero blocks (AC6/AC-042-08).
    assert!(
        json_has_author_id(&tree, MODE_GLOBAL_AUTHOR_ID),
        "the fixed global graph controls remain addressable at zero blocks"
    );

    println!(
        "MT-021 canonical Argus empty graph (LIVE SURREALDB workspace={workspace_id}): inspect() returned {} \
         author_ids, 0 graph.node.* (AC7)",
        ids.len()
    );

    argus.finish();
    cleanup.assert_cleaned();
    assert_no_local_artifact_dir();
}

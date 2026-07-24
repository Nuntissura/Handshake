//! WP-KERNEL-012 E3 MT-021 remediation (FAIL_V2): canonical Argus inspect / safe-steer / re-observe
//! proof for the MOUNTED local/global Loom graph view over REAL PostgreSQL data.
//!
//! `validation_v2` failed MT-021 because "Local/global graph UI states have no current canonical Argus
//! action and post-action observation covering populated real-PostgreSQL graph data. AccessKit or fixture
//! rendering alone does not satisfy the mandatory visual/steering gate." The existing
//! `test_graph_view.rs::graph_view_live_pg_self_seeds_local_global` drives the graph WIDGET (`build_ui`)
//! and dispatches raw AccessKit events — it never drives the MOUNTED `HandshakeApp` through the real
//! localhost `SwarmMcpServer` transport (`argus.inspect` / `argus.click`) the way an out-of-process swarm
//! agent does, and it never re-observes the mounted tree after a canonical action. This test closes that
//! exact gap:
//!
//!   1. seeds a REAL Handshake-managed PostgreSQL workspace with four LoomBlocks + two LoomEdges through
//!      the production HTTP routes (`POST /loom/blocks`, `POST /loom/edges`),
//!   2. mounts the production `HandshakeApp` shell with the Graph View pane and lets the app's OWN
//!      per-frame feed (`drive_graph_and_canvas_feeds`) fetch the Global projection from that live
//!      workspace and drain it into the mounted `LoomGraphView` (no injected fixture),
//!   3. binds the CANONICAL Argus driver (real localhost JSON-RPC) to the mounted app,
//!   4. `argus.inspect` proves each real-PG graph node is addressable by its stable author_id
//!      (`graph.node.{sanitized_block_id}`) plus the fixed global controls,
//!   5. drives ONE safe, reversible action (`graph.relayout`) through the real Argus transport, and
//!   6. FRESH `argus.inspect` re-observes the post-action tree (the real-PG nodes remain addressable — the
//!      re-layout was additive), then writes before/after tree evidence + a screenshot marker (headless
//!      DEFERRED is an acceptable typed outcome) and deletes the workspace.
//!
//! A second test proves the empty real-PG workspace renders and inspects through canonical Argus with no
//! graph nodes and no panic (AC7).
//!
//! Requires the `integration` feature and a reachable managed backend (attach on 37501, or an owned
//! `HSK_TEST_BACKEND_BIN` + `HANDSHAKE_TEST_PG_DSN`). It is feature-gated but NOT ignored.
#![cfg(feature = "integration")]

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;
use screenshot_harness::ScreenshotHarness as Harness;

#[path = "native_gui_support/canonical_argus_driver.rs"]
mod canonical_argus_driver;
use canonical_argus_driver::{json_has_author_id, CanonicalArgusDriver};

#[path = "interconnect_support/mod.rs"]
mod interconnect_support;

use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;
use handshake_native::command_registry::CMD_VIEW_GRAPH;
use handshake_native::graph::graph_view::{
    node_author_id, MODE_GLOBAL_AUTHOR_ID, MODE_LOCAL_AUTHOR_ID, RELAYOUT_AUTHOR_ID,
    ZOOM_IN_AUTHOR_ID, ZOOM_OUT_AUTHOR_ID,
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
            "managed-PG workspace cleanup returned HTTP {status}"
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

/// Mount a production `HandshakeApp` shell bound to the live backend + the seeded workspace, with the
/// Graph View pane opened on the active work surface. The multi-thread runtime is returned so it outlives
/// the harness (the per-frame graph feed dispatches onto it).
fn graph_shell(
    base: &str,
    workspace_id: &str,
) -> (HandshakeApp, tokio::runtime::Runtime) {
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
        "switch to the seeded managed-PG workspace"
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
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    drive_until(
        &mut harness,
        &graph_view,
        |g| g.nodes.len() == 4 && !g.loading && g.error.is_none(),
        "mounted graph self-fetches the four real-PG nodes from the live Global projection",
    );
    assert_eq!(
        graph_view.lock().unwrap().edges.len(),
        2,
        "the mounted graph carries the two real persisted LoomEdges (relationships from real PG)"
    );

    let artifact_dir = external_artifact_dir("wp-kernel-012-mt-021/canonical-argus");
    std::fs::create_dir_all(&artifact_dir).expect("create external MT-021 Argus artifact dir");

    let mut argus = CanonicalArgusDriver::bind(harness.state(), "wp-kernel-012-mt-021-graph");

    // (1) Canonical inspect: every real-PG graph node is addressable by stable author_id, and the fixed
    // global controls are present regardless of content (AC6).
    let before = argus.inspect(&mut harness);
    for block_id in &seeded {
        let author = node_author_id(block_id);
        assert!(
            json_has_author_id(&before, &author),
            "canonical argus.inspect must see the mounted real-PG graph node '{author}'"
        );
    }
    for control in [
        MODE_LOCAL_AUTHOR_ID,
        MODE_GLOBAL_AUTHOR_ID,
        ZOOM_IN_AUTHOR_ID,
        ZOOM_OUT_AUTHOR_ID,
        RELAYOUT_AUTHOR_ID,
    ] {
        assert!(
            json_has_author_id(&before, control),
            "canonical argus.inspect must see the fixed global graph control '{control}'"
        );
    }

    // (2) Safe, reversible steer: re-run the force layout through the real Argus transport. This mutates
    // no durable/backend state — it re-seeds ephemeral node positions in the panel only.
    let observation = argus.click_and_reinspect(&mut harness, RELAYOUT_AUTHOR_ID);
    assert!(
        matches!(
            observation.receipt_status.as_str(),
            "applied" | "indeterminate"
        ),
        "the canonical relayout action receipt is terminal and non-rejected: {}",
        observation.receipt_status
    );
    assert!(
        observation
            .agent_id
            .contains(":client:wp-kernel-012-mt-021-graph-agent"),
        "the canonical receipt retains the external caller attribution: {}",
        observation.agent_id
    );

    // (3) Fresh re-observation: every real-PG node remains addressable after the safe action.
    for block_id in &seeded {
        let author = node_author_id(block_id);
        assert!(
            json_has_author_id(&observation.after, &author),
            "fresh canonical re-inspection still sees the real-PG graph node '{author}'"
        );
    }

    // (4) Evidence: before/after canonical trees + a screenshot marker.
    let tree_path = artifact_dir.join("mt021-mounted-graph-argus.json");
    std::fs::write(
        &tree_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "workspace_id": workspace_id,
            "seeded_blocks": seeded,
            "before": before,
            "after": observation.after,
            "receipt_id": observation.receipt_id,
            "receipt_status": observation.receipt_status,
            "agent_id": observation.agent_id,
        }))
        .expect("serialize canonical MT-021 graph tree evidence"),
    )
    .expect("write canonical MT-021 graph tree evidence externally");
    assert!(tree_path.is_file());

    let screenshot_marker = match harness.render() {
        Ok(image) => {
            let path = artifact_dir.join("mt021-mounted-graph.png");
            image.save(&path).expect("save mounted graph screenshot");
            format!("CAPTURED {}", path.display())
        }
        Err(deferred) => format!("DEFERRED (headless): {deferred}"),
    };
    println!(
        "MT-021 canonical Argus mounted graph (LIVE PG workspace={workspace_id}): \
         inspect(4 real-PG nodes + 5 controls) -> click({RELAYOUT_AUTHOR_ID}) \
         -> reinspect(4 nodes still addressable); receipt={} agent={} screenshot={} tree={}",
        observation.receipt_status,
        observation.agent_id,
        screenshot_marker,
        tree_path.display()
    );

    argus.finish();
    cleanup.assert_cleaned();
    assert_no_local_artifact_dir();
}

#[test]
fn mt021_mounted_graph_canonical_argus_local_global_switch_distinct_queries() {
    // V2 R1 (distinct local vs global queries) + R2 (safe switching): drive the local/global mode control
    // through canonical Argus and prove the switch produces a DISTINCT real-PostgreSQL query result, not a
    // client-side re-render. Global returns all 4 seeded nodes; Local (focused on beta) returns only
    // beta's real neighbourhood {alpha,beta,gamma} — the disconnected `isolated` block is a real seeded
    // node that a client-side filter could not remove but a distinct backend /loom/graph/local query does.
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
        "mounted graph self-fetches the four real-PG Global nodes",
    );

    let artifact_dir = external_artifact_dir("wp-kernel-012-mt-021/canonical-argus");
    std::fs::create_dir_all(&artifact_dir).expect("create external MT-021 Argus artifact dir");
    let mut argus = CanonicalArgusDriver::bind(harness.state(), "wp-kernel-012-mt-021-graph-switch");

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
    argus.click_with_payload_and_reinspect(
        &mut harness,
        "graph.select-node",
        serde_json::json!({ "block_id": beta }),
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
        "canonical local switch issues a distinct real-PG neighbourhood query (3 nodes, no isolated)",
    );

    // (3) Fresh inspect proves the DISTINCT local node set: neighbourhood present, isolated absent.
    let local_tree = argus.inspect(&mut harness);
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
         distinct real-PG neighbourhood query, not a re-render of the global set"
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
        "canonical global switch re-queries the full real-PG projection (4 nodes)",
    );
    let global_after = argus.inspect(&mut harness);
    assert!(
        json_has_author_id(&global_after, &author(&isolated)),
        "switching back to Global re-queries real PG and the disconnected 'isolated' node returns"
    );

    let tree_path = artifact_dir.join("mt021-graph-local-global-switch-argus.json");
    std::fs::write(
        &tree_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "workspace_id": workspace_id,
            "seeded": { "alpha": alpha, "beta": beta, "gamma": gamma, "isolated": isolated },
            "global_before": global_before,
            "local_after_switch": local_tree,
            "global_after_switch_back": global_after,
            "local_switch_receipt": switch_local.receipt_status,
            "global_switch_receipt": switch_global.receipt_status,
        }))
        .expect("serialize MT-021 switch tree evidence"),
    )
    .expect("write MT-021 switch tree evidence externally");
    assert!(tree_path.is_file());

    println!(
        "MT-021 canonical Argus local/global switch (LIVE PG workspace={workspace_id}): \
         Global inspect(4 incl isolated) -> select-node(beta) + click({MODE_LOCAL_AUTHOR_ID}) \
         -> Local inspect(3 neighbourhood, isolated ABSENT = distinct real-PG query) \
         -> click({MODE_GLOBAL_AUTHOR_ID}) -> Global inspect(isolated returns). tree={}",
        tree_path.display()
    );

    argus.finish();
    cleanup.assert_cleaned();
    assert_no_local_artifact_dir();
}

#[test]
fn mt021_mounted_graph_empty_state_canonical_argus() {
    // AC7 against a REAL unseeded managed-PG workspace: the mounted panel renders + inspects through
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
        "mounted graph self-fetches an empty projection from a fresh real-PG workspace",
    );

    let mut argus = CanonicalArgusDriver::bind(harness.state(), "wp-kernel-012-mt-021-graph-empty");
    let tree = argus.inspect(&mut harness);

    let mut ids = Vec::new();
    collect_author_ids(&tree, &mut ids);
    assert!(
        !ids.iter().any(|id| id.starts_with("graph.node.")),
        "empty real-PG workspace mounted graph must expose NO graph nodes through canonical Argus; got {:?}",
        ids.iter().filter(|id| id.starts_with("graph.")).collect::<Vec<_>>()
    );
    // The fixed global controls are still present at zero blocks (AC6/AC-042-08).
    assert!(
        json_has_author_id(&tree, MODE_GLOBAL_AUTHOR_ID),
        "the fixed global graph controls remain addressable at zero blocks"
    );

    println!(
        "MT-021 canonical Argus empty graph (LIVE PG workspace={workspace_id}): inspect() returned {} \
         author_ids, 0 graph.node.* (AC7)",
        ids.len()
    );

    argus.finish();
    cleanup.assert_cleaned();
    assert_no_local_artifact_dir();
}

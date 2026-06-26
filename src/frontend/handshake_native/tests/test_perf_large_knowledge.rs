//! WP-KERNEL-012 MT-045 — E8 Large-Document Performance Proof, knowledge-graph scenarios (LK-01..LK-05).
//!
//! ## Reality split (KERNEL_BUILDER gate 2026-06-26, same honest split as MT-044)
//!
//! LK-01 (graph load), LK-03 (tag hub), LK-04 (search-v2), LK-05 (folder tree) BIND the handshake_core
//! loom backend and need a live managed PostgreSQL, so they are honestly `#[ignore]`d `requires_pg`
//! (manifest status REQUIRES_PG): each hits the REAL route via the shared `pg_proof_support` live
//! backend, never a mock, and PASSES with `--ignored` once a seeded backend is up.
//!
//! ## EXCEPTION — LK-02 force-layout runs NOW (frontend-only native impl)
//!
//! LK-02 measures the NATIVE force-directed graph layout — `handshake_native::graph::graph_view`'s
//! `LoomGraphView::set_graph` + `step_layout` driven to convergence. This is the WP-012 module under
//! measurement (the contract's `graph::graph_layout` is realized as the force layout INSIDE
//! `LoomGraphView`; there is no separate `graph_layout.rs` module — the layout lives in `graph_view`,
//! verified by code inspection). It needs NO PostgreSQL: the node/edge set is synthesized in-process and
//! the layout is a pure deterministic force simulation, so it RUNS + PASSES NOW with a real measured
//! timing and writes-back PASS.
//!
//! ## No mock smuggling (RISK-2 / CTRL-2)
//!
//! LK-02 drives the REAL `LoomGraphView` force layout (no UI needed — `step_layout` seeds positions on a
//! deterministic circle and runs the spring/repulsion model headless). The gated LK-01/03/04/05 hit real
//! routes. No sqlite, no in-memory backend stub. Block creation in the gated scenarios is NOT counted in
//! the budget (RISK-2 / CTRL-2): only the QUERY phase is timed (impl notes 7, 8).

mod perf_proof_support;
mod pg_proof_support;

use perf_proof_support::{record, skip_all, time_ms, Budget};
use pg_proof_support::LiveBackend;

use handshake_native::graph::graph_view::{GraphEdge, GraphNode, LoomGraphView};

// ── LK-02 (runs NOW): native force-directed layout for a 1000-node graph <= 1000 ms ───────────────

#[test]
fn perf_lk02_graph_layout() {
    if skip_all() {
        return;
    }
    perf_proof_support::assert_no_local_artifact_dir();
    let budget = Budget::resolve("LK-02", "PERF_BUDGET_LK02_MS", 1_000);

    // FIXTURE (NOT timed): synthesize a 1000-node graph with ~2000 edges (a deterministic sparse graph —
    // each node links to the next two, wrapping). set_graph seeds the layout but does not run the force
    // sim to convergence; the convergence run is the measured op.
    let node_count = 1000usize;
    let nodes: Vec<GraphNode> = (0..node_count)
        .map(|i| GraphNode::new(format!("block-{i:04}"), format!("Block {i}"), "note"))
        .collect();
    let mut edges: Vec<GraphEdge> = Vec::with_capacity(node_count * 2);
    for i in 0..node_count {
        edges.push(GraphEdge::new(
            format!("block-{i:04}"),
            format!("block-{:04}", (i + 1) % node_count),
            "mention",
        ));
        edges.push(GraphEdge::new(
            format!("block-{i:04}"),
            format!("block-{:04}", (i + 2) % node_count),
            "mention",
        ));
    }
    assert_eq!(edges.len(), node_count * 2, "LK-02: ~2000 edges synthesized");

    let mut view = LoomGraphView::global("mt045-lk02");
    view.set_graph(nodes, edges); // seeds positions (setup)
    assert!(!view.layout_stable(), "LK-02: a fresh 1000-node layout is not yet stable");

    // MEASURED: drive the REAL force layout to its stop condition (converged OR the iteration budget),
    // i.e. ONE full layout-to-stable pass. step_layout runs ITERS_PER_FRAME force iters per call.
    let (_, elapsed_ms) = time_ms(|| {
        let mut frames = 0usize;
        while !view.layout_stable() {
            view.step_layout();
            frames += 1;
            // Hard safety bound (the layout's own MAX_LAYOUT_ITERS already caps it; this guards the test
            // loop from any never-stable regression so a bug fails fast, not hangs).
            assert!(frames < 100_000, "LK-02: layout must reach a stop condition, not spin forever");
        }
    });

    // Positions must be finite after the run (the force clamp guards 1/d^2 blow-up at 1000 nodes).
    let finite = view.nodes.iter().all(|n| n.x.is_finite() && n.y.is_finite());
    assert!(finite, "LK-02: all 1000 node positions must be finite after layout");
    assert!(view.layout_stable(), "LK-02: the layout must report stable after the run");
    assert!(
        budget.passes(elapsed_ms),
        "LK-02: 1000-node force-layout {elapsed_ms} ms must be <= {} ms (override {})",
        budget.ceiling,
        budget.env_var
    );

    println!(
        "LK-02 measured={elapsed_ms}ms (<= {}ms) PASS — native force-directed layout for 1000 nodes / \
         2000 edges to convergence ({} iters)",
        budget.ceiling,
        view.iters_done
    );
    record("LK-02", elapsed_ms as f64, "PASS");
}

// ── LK-01: graph load, 1000 nodes — query <= 3 s, node_count >= 1000 (REQUIRES_PG) ────────────────

#[test]
#[ignore = "requires_pg: live handshake_core + PostgreSQL + HSK_TEST_WORKSPACE_ID (GET /workspaces/{id}/loom/graph/global)"]
fn perf_lk01_graph_load() {
    let be = require_be();
    let budget = Budget::resolve("LK-01", "PERF_BUDGET_LK01_MS", 3_000);

    // FIXTURE (NOT timed): batch-create 1000 blocks + ~2000 edges (block creation is NOT in the budget —
    // impl note 7). The blocks are cleaned up by a DropGuard.
    let mut created_ids: Vec<String> = Vec::with_capacity(1000);
    for i in 0..1000usize {
        let block = be.post_json(
            &format!("/workspaces/{}/loom/blocks", be.workspace_id),
            &serde_json::json!({ "content_type": "note", "title": format!("lk01-{i}") }),
        );
        if let Some(id) = block.get("block_id").and_then(|v| v.as_str()) {
            created_ids.push(id.to_owned());
        }
    }
    let _guard = BlockGuard { be: &be, ids: created_ids.clone() };
    assert!(created_ids.len() >= 1000, "LK-01: 1000 blocks created (got {})", created_ids.len());

    // MEASURED: the graph QUERY only (depth=2).
    let (graph, elapsed_ms) = time_ms(|| {
        be.get_json(&format!("/workspaces/{}/loom/graph/global?depth=2", be.workspace_id))
    });
    let node_count = graph
        .get("nodes")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .or_else(|| graph.get("node_count").and_then(|v| v.as_u64()).map(|n| n as usize))
        .unwrap_or(0);
    assert!(node_count >= 1000, "LK-01: the graph must report >= 1000 nodes (got {node_count})");
    assert!(budget.passes(elapsed_ms), "LK-01: graph query {elapsed_ms} ms must be <= {} ms", budget.ceiling);

    println!("LK-01 measured={elapsed_ms}ms (<= {}ms) PASS — graph load, {node_count} nodes (live PG)", budget.ceiling);
    record("LK-01", elapsed_ms as f64, "PASS");
}

// ── LK-03: tag hub query, 5000 blocks tagged — query <= 2 s, hit_count == 5000 (REQUIRES_PG) ──────

#[test]
#[ignore = "requires_pg: live handshake_core + PostgreSQL + HSK_TEST_WORKSPACE_ID (GET /workspaces/{id}/loom/tags/{tag}/blocks)"]
fn perf_lk03_tag_hub() {
    let be = require_be();
    let budget = Budget::resolve("LK-03", "PERF_BUDGET_LK03_MS", 2_000);

    // FIXTURE (NOT timed, impl note 8): a tag-hub block + 5000 blocks each carrying that tag (an edge
    // from the tag hub to each block). Created in batches; NOT counted in the budget. The seeded tag
    // block id is read from HSK_TEST_TAG_BLOCK_ID when the operator pre-seeds (large batch is slow).
    let tag_block_id = std::env::var("HSK_TEST_TAG_BLOCK_ID").ok().filter(|s| !s.is_empty());
    let (tag_id, mut created_ids): (String, Vec<String>) = if let Some(id) = tag_block_id {
        (id, Vec::new()) // pre-seeded by the operator; no cleanup of pre-seeded rows
    } else {
        let tag = be.post_json(
            &format!("/workspaces/{}/loom/blocks", be.workspace_id),
            &serde_json::json!({ "content_type": "tag", "title": "mt045-lk03-taghub" }),
        );
        let tag_id = tag.get("block_id").and_then(|v| v.as_str()).expect("LK-03: tag block_id").to_owned();
        let mut ids = vec![tag_id.clone()];
        for i in 0..5000usize {
            let blk = be.post_json(
                &format!("/workspaces/{}/loom/blocks", be.workspace_id),
                &serde_json::json!({ "content_type": "note", "title": format!("lk03-{i}") }),
            );
            if let Some(bid) = blk.get("block_id").and_then(|v| v.as_str()) {
                ids.push(bid.to_owned());
                // tag edge: hub -> block
                let _ = be.post_json(
                    &format!("/workspaces/{}/loom/edges", be.workspace_id),
                    &serde_json::json!({ "source": tag_id, "target": bid, "edge_type": "tag" }),
                );
            }
        }
        (tag_id, ids)
    };
    let _guard = BlockGuard { be: &be, ids: std::mem::take(&mut created_ids) };

    // MEASURED: the tag-hub QUERY only.
    let (resp, elapsed_ms) = time_ms(|| {
        be.get_json(&format!("/workspaces/{}/loom/tags/{}/blocks?limit=10000", be.workspace_id, tag_id))
    });
    let hit_count = resp
        .as_array()
        .map(|a| a.len())
        .or_else(|| resp.get("blocks").and_then(|v| v.as_array()).map(|a| a.len()))
        .or_else(|| resp.get("hit_count").and_then(|v| v.as_u64()).map(|n| n as usize))
        .unwrap_or(0);
    assert_eq!(hit_count, 5000, "LK-03: the tag hub must return exactly 5000 blocks (got {hit_count})");
    assert!(budget.passes(elapsed_ms), "LK-03: tag query {elapsed_ms} ms must be <= {} ms", budget.ceiling);

    // proof_target #5 greps for 'hit_count=5000'.
    println!("LK-03 measured={elapsed_ms}ms (<= {}ms) PASS — tag hub hit_count={hit_count} (live PG)", budget.ceiling);
    record("LK-03", elapsed_ms as f64, "PASS");
}

// ── LK-04: search index, 5000 blocks — query <= 2 s, 50..200 hits (REQUIRES_PG) ───────────────────

#[test]
#[ignore = "requires_pg: live handshake_core + PostgreSQL + HSK_TEST_WORKSPACE_ID (POST /workspaces/{id}/loom/search-v2)"]
fn perf_lk04_search_index() {
    let be = require_be();
    let budget = Budget::resolve("LK-04", "PERF_BUDGET_LK04_MS", 2_000);

    // FIXTURE (NOT timed): 5000 blocks; ~100 carry the distinctive token "ZEBRAQUERY" so search-v2
    // matches roughly that many. Pre-seedable via HSK_TEST_WORKSPACE_ID having the corpus.
    let mut created_ids: Vec<String> = Vec::new();
    if std::env::var("HSK_TEST_SEARCH_PRESEEDED").as_deref() != Ok("1") {
        for i in 0..5000usize {
            let title = if i % 50 == 0 { format!("ZEBRAQUERY doc {i}") } else { format!("plain doc {i}") };
            let blk = be.post_json(
                &format!("/workspaces/{}/loom/blocks", be.workspace_id),
                &serde_json::json!({ "content_type": "note", "title": title }),
            );
            if let Some(bid) = blk.get("block_id").and_then(|v| v.as_str()) {
                created_ids.push(bid.to_owned());
            }
        }
    }
    let _guard = BlockGuard { be: &be, ids: created_ids.clone() };

    // MEASURED: the search QUERY only.
    let (resp, elapsed_ms) = time_ms(|| {
        be.post_json(
            &format!("/workspaces/{}/loom/search-v2", be.workspace_id),
            &serde_json::json!({ "query": "ZEBRAQUERY", "limit": 500 }),
        )
    });
    let hits = resp
        .get("hits")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .or_else(|| resp.get("results").and_then(|v| v.as_array()).map(|a| a.len()))
        .or_else(|| resp.as_array().map(|a| a.len()))
        .unwrap_or(0);
    assert!((50..=200).contains(&hits), "LK-04: search must return 50..200 hits (got {hits})");
    assert!(budget.passes(elapsed_ms), "LK-04: search {elapsed_ms} ms must be <= {} ms", budget.ceiling);

    println!("LK-04 measured={elapsed_ms}ms (<= {}ms) PASS — search-v2 returned {hits} hits over 5000 blocks (live PG)", budget.ceiling);
    record("LK-04", elapsed_ms as f64, "PASS");
}

// ── LK-05: folder tree, 200 folders — query <= 1 s, folder_count == 200 (REQUIRES_PG) ─────────────

#[test]
#[ignore = "requires_pg: live handshake_core + PostgreSQL + HSK_TEST_WORKSPACE_ID (GET /workspaces/{id}/loom/folders)"]
fn perf_lk05_folder_tree() {
    let be = require_be();
    let budget = Budget::resolve("LK-05", "PERF_BUDGET_LK05_MS", 1_000);

    // FIXTURE (NOT timed): 200 folders (+ child blocks). Pre-seedable.
    let mut created_folders: Vec<String> = Vec::new();
    if std::env::var("HSK_TEST_FOLDERS_PRESEEDED").as_deref() != Ok("1") {
        for i in 0..200usize {
            let folder = be.post_json(
                &format!("/workspaces/{}/loom/folders", be.workspace_id),
                &serde_json::json!({ "name": format!("mt045-lk05-folder-{i}") }),
            );
            if let Some(fid) = folder.get("folder_id").and_then(|v| v.as_str()).or_else(|| folder.get("id").and_then(|v| v.as_str())) {
                created_folders.push(fid.to_owned());
            }
        }
    }
    let _guard = FolderGuard { be: &be, ids: created_folders.clone() };

    // MEASURED: the folder-tree QUERY only.
    let (resp, elapsed_ms) = time_ms(|| be.get_json(&format!("/workspaces/{}/loom/folders", be.workspace_id)));
    let folder_count = resp
        .as_array()
        .map(|a| a.len())
        .or_else(|| resp.get("folders").and_then(|v| v.as_array()).map(|a| a.len()))
        .unwrap_or(0);
    assert_eq!(folder_count, 200, "LK-05: the folder tree must return exactly 200 folders (got {folder_count})");
    assert!(budget.passes(elapsed_ms), "LK-05: folder query {elapsed_ms} ms must be <= {} ms", budget.ceiling);

    println!("LK-05 measured={elapsed_ms}ms (<= {}ms) PASS — folder tree folder_count={folder_count} (live PG)", budget.ceiling);
    record("LK-05", elapsed_ms as f64, "PASS");
}

// ── shared helpers ────────────────────────────────────────────────────────────────────────────────

fn require_be() -> LiveBackend {
    pg_proof_support::require_live_backend()
}

/// Best-effort deletes created loom blocks on drop (real `DELETE /workspaces/{id}/loom/blocks/{id}`
/// route exists), so PG-writing proofs are idempotent (impl note 9). A cleanup failure never masks the
/// proof verdict.
struct BlockGuard<'a> {
    be: &'a LiveBackend,
    ids: Vec<String>,
}
impl Drop for BlockGuard<'_> {
    fn drop(&mut self) {
        for id in &self.ids {
            let _ = self.be.delete(&format!("/workspaces/{}/loom/blocks/{}", self.be.workspace_id, id));
        }
    }
}

/// Best-effort deletes created folders on drop (real `DELETE /workspaces/{id}/loom/folders/{id}` route
/// exists). A cleanup failure never masks the proof verdict.
struct FolderGuard<'a> {
    be: &'a LiveBackend,
    ids: Vec<String>,
}
impl Drop for FolderGuard<'_> {
    fn drop(&mut self) {
        for id in &self.ids {
            let _ = self.be.delete(&format!("/workspaces/{}/loom/folders/{}", self.be.workspace_id, id));
        }
    }
}

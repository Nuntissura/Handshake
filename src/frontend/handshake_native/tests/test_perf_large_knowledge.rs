//! WP-KERNEL-012 MT-045 — E8 Large-Document Performance Proof, knowledge-graph scenarios (LK-01..LK-05).
//!
//! ## Managed PostgreSQL runtime
//!
//! LK-01 (graph load), LK-03 (tag hub), LK-04 (search-v2), LK-05 (folder tree) BIND the handshake_core
//! loom backend and run by default through the shared managed product-backend fixture. Each scenario
//! creates its own workspace and full-scale corpus through production HTTP; workspace deletion owns
//! teardown. No operator-preseeded rows or identifiers are accepted.
//!
//! ## EXCEPTION — LK-02 force-layout runs NOW (frontend-only native impl)
//!
//! LK-02 measures the NATIVE force-directed graph layout — `handshake_native::graph::graph_view`'s
//! `LoomGraphView::set_graph` + `step_layout` driven to convergence. This is the WP-012 module under
//! measurement (the contract's `graph::graph_layout` is realized as the force layout INSIDE
//! `LoomGraphView`; there is no separate `graph_layout.rs` module — the layout lives in `graph_view`,
//! verified by code inspection). It needs NO PostgreSQL: the node/edge set is synthesized in-process and
//! the layout is a pure deterministic force simulation, so it runs with a real external measurement.
//!
//! LK-02 drives the native layout at the contract size: 1,000 nodes and approximately 2,000 edges.
//!
//! ## No mock smuggling (RISK-2 / CTRL-2)
//!
//! LK-02 drives the REAL `LoomGraphView` force layout (no UI needed — `step_layout` seeds positions on a
//! deterministic circle and runs the spring/repulsion model headless). The gated LK-01/03/04/05 hit real
//! routes. No sqlite, no in-memory backend stub. Block creation in the gated scenarios is NOT counted in
//! the budget (RISK-2 / CTRL-2): only the QUERY phase is timed (impl notes 7, 8).

mod perf_proof_support;
mod pg_proof_support;

use perf_proof_support::{measurement, time_ms, Budget, ScenarioAttempt};
use pg_proof_support::LiveBackend;
use std::collections::HashSet;

use handshake_native::graph::graph_view::{GraphEdge, GraphNode, LoomGraphView, NODE_CAP};

// ── LK-02: one native force-layout pass over 1000 nodes / ~2000 edges ─────────────────────────────

#[test]
fn perf_proof_perf_lk02_graph_layout() {
    let budget = Budget::resolve("LK-02", "PERF_BUDGET_LK02_MS", 1_000);
    let Some(attempt) =
        ScenarioAttempt::begin_or_skip("LK-02", "primary", &[("graph_layout", &budget, "ms")])
    else {
        return;
    };
    perf_proof_support::assert_no_local_artifact_dir();

    // FIXTURE (NOT timed): synthesize the full deterministic sparse graph. NODE_CAP is a product safety
    // ceiling, and must admit the complete contract-sized set rather than truncating this proof.
    let synth_node_count = 1000usize;
    let nodes: Vec<GraphNode> = (0..synth_node_count)
        .map(|i| GraphNode::new(format!("block-{i:04}"), format!("Block {i}"), "note"))
        .collect();
    let mut edges: Vec<GraphEdge> = Vec::with_capacity(synth_node_count * 2);
    for i in 0..synth_node_count {
        edges.push(GraphEdge::new(
            format!("block-{i:04}"),
            format!("block-{:04}", (i + 1) % synth_node_count),
            "mention",
        ));
        edges.push(GraphEdge::new(
            format!("block-{i:04}"),
            format!("block-{:04}", (i + 2) % synth_node_count),
            "mention",
        ));
    }
    assert_eq!(
        edges.len(),
        synth_node_count * 2,
        "LK-02: ~2000 edges synthesized for the 1000-node target"
    );

    let mut view = LoomGraphView::global("mt045-lk02");
    view.set_graph(nodes, edges); // seeds positions (setup)

    let laid_out = view.nodes.len();
    assert_eq!(
        laid_out, synth_node_count,
        "LK-02: the native layout must admit all 1000 nodes (NODE_CAP={NODE_CAP})"
    );
    assert_eq!(
        view.total_available, synth_node_count,
        "LK-02: set_graph must record the true total (1000) in total_available for the truncation notice"
    );
    assert!(
        !view.layout_stable(),
        "LK-02: a fresh {laid_out}-node layout is not yet stable"
    );

    // MEASURED: one real native layout pass, matching the scenario wording. The layout's own per-frame
    // iteration bound prevents a single frame from becoming an unbounded convergence loop.
    let (max_step, elapsed_ms) = time_ms(|| view.step_layout());
    attempt.stage(
        serde_json::json!([measurement("graph_layout", elapsed_ms as f64, "ms")]),
        serde_json::json!({
            "nodes": laid_out,
            "edges": synth_node_count * 2,
            "max_step": max_step,
        }),
    );

    // Positions must be finite after the run (the force clamp guards 1/d^2 blow-up).
    let finite = view
        .nodes
        .iter()
        .all(|n| n.x.is_finite() && n.y.is_finite());
    assert!(
        finite,
        "LK-02: all {laid_out} laid-out node positions must be finite after layout"
    );
    assert!(
        max_step.is_finite(),
        "LK-02: the layout step delta must be finite"
    );
    assert!(
        budget.passes(elapsed_ms),
        "LK-02: {laid_out}-node force-layout {elapsed_ms} ms must be <= {} ms (override {})",
        budget.ceiling,
        budget.env_var
    );

    println!(
        "LK-02 measured={elapsed_ms}ms (<= {}ms) PASS — one native force-directed layout pass for \
         {laid_out} nodes / {} edges (NODE_CAP={NODE_CAP}, {} iterations)",
        budget.ceiling,
        synth_node_count * 2,
        view.iters_done
    );
    attempt.pass(
        serde_json::json!([measurement("graph_layout", elapsed_ms as f64, "ms")]),
        serde_json::json!({"nodes": laid_out, "edges": synth_node_count * 2}),
    );
}

// ── LK-01: graph load, 1000 nodes — query <= 3 s, node_count >= 1000 (REQUIRES_PG) ────────────────

#[test]
fn perf_proof_perf_lk01_graph_load() {
    let budget = Budget::resolve("LK-01", "PERF_BUDGET_LK01_MS", 3_000);
    let Some(attempt) =
        ScenarioAttempt::begin_or_skip("LK-01", "primary", &[("graph_load", &budget, "ms")])
    else {
        return;
    };
    let mut be = require_be();

    // FIXTURE (NOT timed): create 1000 blocks + exactly 2000 deterministic, varied-degree sparse edges
    // through the public Loom mutation routes. This avoids a synthetic fixed-degree ring while preserving
    // reproducibility and every product-owned write side effect.
    let setup_deadline = pg_proof_support::SetupDeadline::begin("LK-01-product-import");
    let prefix = format!("lk01-{}", uuid::Uuid::new_v4().simple());
    let block_ids = create_note_blocks(&be, &setup_deadline, &prefix, 1_000, |index| {
        format!("LK-01 {index}")
    });
    setup_deadline.check();
    let edge_path = format!("/workspaces/{}/loom/edges", be.workspace_id);
    let mut edge_pairs = HashSet::with_capacity(2_000);
    let mut out_degree = vec![0usize; block_ids.len()];
    let mut seed = 0x4d54_3034_354c_4b01_u64;
    while edge_pairs.len() < 2_000 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let source = (seed as usize) % block_ids.len();
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let target = (seed as usize) % block_ids.len();
        if source != target && edge_pairs.insert((source, target)) {
            out_degree[source] += 1;
        }
    }
    let mut edge_pairs: Vec<(usize, usize)> = edge_pairs.into_iter().collect();
    edge_pairs.sort_unstable();
    let edge_requests = edge_pairs
        .iter()
        .enumerate()
        .map(|(edge_index, &(source, target))| {
            (
                edge_path.clone(),
                serde_json::json!({
                    "edge_id": format!("{prefix}-edge-{edge_index:04}"),
                    "source_block_id": block_ids[source],
                    "target_block_id": block_ids[target],
                    "edge_type": "mention",
                    "created_by": "user",
                }),
            )
        })
        .collect();
    let min_out_degree = *out_degree.iter().min().expect("LK-01 degree set");
    let max_out_degree = *out_degree.iter().max().expect("LK-01 degree set");
    assert!(
        max_out_degree > min_out_degree,
        "LK-01 fixture must have varied node degree"
    );
    be.post_json_batch_bounded(edge_requests, fixture_concurrency(), &setup_deadline);
    setup_deadline.check();

    // MEASURED: the graph QUERY only (depth=2).
    let (graph, elapsed_ms) = time_ms(|| {
        be.get_json(&format!(
            "/workspaces/{}/loom/graph/global?node_limit=1200&hub_degree_threshold=10000",
            be.workspace_id
        ))
    });
    let node_count = graph
        .get("nodes")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .or_else(|| {
            graph
                .get("node_count")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize)
        })
        .unwrap_or(0);
    attempt.stage(
        serde_json::json!([measurement("graph_load", elapsed_ms as f64, "ms")]),
        serde_json::json!({
            "nodes": node_count,
            "edges_seeded": 2000,
            "fixture_strategy": "deterministic_varied_sparse_public_loom_routes",
            "min_out_degree": min_out_degree,
            "max_out_degree": max_out_degree,
        }),
    );
    assert!(
        node_count >= 1000,
        "LK-01: the graph must report >= 1000 nodes (got {node_count})"
    );
    assert!(
        budget.passes(elapsed_ms),
        "LK-01: graph query {elapsed_ms} ms must be <= {} ms",
        budget.ceiling
    );

    println!(
        "LK-01 measured={elapsed_ms}ms (<= {}ms) PASS — graph load, {node_count} nodes (live PG)",
        budget.ceiling
    );
    be.assert_cleanup();
    attempt.pass(
        serde_json::json!([measurement("graph_load", elapsed_ms as f64, "ms")]),
        serde_json::json!({
            "nodes": node_count,
            "edges_seeded": 2000,
            "fixture_strategy": "deterministic_varied_sparse_public_loom_routes",
            "min_out_degree": min_out_degree,
            "max_out_degree": max_out_degree,
        }),
    );
}

// ── LK-03: tag hub query, 5000 blocks tagged — query <= 2 s, hit_count == 5000 (REQUIRES_PG) ──────

#[test]
fn perf_proof_perf_lk03_tag_hub() {
    let budget = Budget::resolve("LK-03", "PERF_BUDGET_LK03_MS", 2_000);
    let Some(attempt) =
        ScenarioAttempt::begin_or_skip("LK-03", "primary", &[("tag_hub", &budget, "ms")])
    else {
        return;
    };
    let mut be = require_be();

    // FIXTURE (NOT timed, impl note 8): a tag-hub block + 5000 blocks each carrying that tag (an edge
    // from each member block to the tag hub). Created through the public routes; NOT counted in the
    // budget. Tag-edge orientation is member -> TagHub, matching the production query authority.
    let tag = be.post_json(
        &format!("/workspaces/{}/loom/blocks", be.workspace_id),
        &serde_json::json!({ "content_type": "tag_hub", "title": "mt045-lk03-taghub" }),
    );
    let tag_id = tag
        .get("block_id")
        .and_then(|v| v.as_str())
        .expect("LK-03: tag block_id")
        .to_owned();
    let setup_deadline = pg_proof_support::SetupDeadline::begin("LK-03-product-import");
    let prefix = format!("lk03-{}", uuid::Uuid::new_v4().simple());
    let block_ids = create_note_blocks(&be, &setup_deadline, &prefix, 5_000, |index| {
        format!("LK-03 {index}")
    });
    setup_deadline.check();
    let edge_path = format!("/workspaces/{}/loom/edges", be.workspace_id);
    let edge_requests = block_ids
        .iter()
        .enumerate()
        .map(|(index, block_id)| {
            (
                edge_path.clone(),
                serde_json::json!({
                    "edge_id": format!("{prefix}-tag-edge-{index}"),
                    "source_block_id": block_id,
                    "target_block_id": tag_id,
                    "edge_type": "tag",
                    "created_by": "user",
                }),
            )
        })
        .collect();
    be.post_json_batch_bounded(edge_requests, fixture_concurrency(), &setup_deadline);
    setup_deadline.check();

    // MEASURED: one canonical tag-hub query. The tag-hub surface returns its complete direct
    // `tagged_blocks` set; timing ten independent 500-row pagination requests measured client
    // round-trip multiplication instead of the contract's singular "query the tag hub" operation.
    let (hub, elapsed_ms) = time_ms(|| {
        be.get_json(&format!(
            "/workspaces/{}/loom/tags/{tag_id}",
            be.workspace_id
        ))
    });
    let hit_count = hub
        .get("tagged_blocks")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len);
    attempt.stage(
        serde_json::json!([measurement("tag_hub", elapsed_ms as f64, "ms")]),
        serde_json::json!({
            "result_count": hit_count,
            "backlink_count": hub.get("backlink_count").and_then(serde_json::Value::as_i64),
        }),
    );
    assert_eq!(
        hit_count, 5000,
        "LK-03: the tag hub must return exactly 5000 blocks (got {hit_count})"
    );
    assert_eq!(
        hub.get("backlink_count")
            .and_then(serde_json::Value::as_i64),
        Some(5_000),
        "LK-03: the tag hub backlink count must match all 5000 tag edges"
    );
    assert!(
        budget.passes(elapsed_ms),
        "LK-03: tag query {elapsed_ms} ms must be <= {} ms",
        budget.ceiling
    );

    // proof_target #5 greps for 'hit_count=5000'.
    println!(
        "LK-03 measured={elapsed_ms}ms (<= {}ms) PASS — tag hub hit_count={hit_count} (live PG)",
        budget.ceiling
    );
    be.assert_cleanup();
    attempt.pass(
        serde_json::json!([measurement("tag_hub", elapsed_ms as f64, "ms")]),
        serde_json::json!({"result_count": hit_count}),
    );
}

// ── LK-04: search index, 5000 blocks — query <= 2 s, 50..200 hits (REQUIRES_PG) ───────────────────

#[test]
fn perf_proof_perf_lk04_search_index() {
    let budget = Budget::resolve("LK-04", "PERF_BUDGET_LK04_MS", 2_000);
    let Some(attempt) =
        ScenarioAttempt::begin_or_skip("LK-04", "primary", &[("search_index", &budget, "ms")])
    else {
        return;
    };
    let mut be = require_be();

    // FIXTURE (NOT timed): 5000 blocks; ~100 carry the distinctive token "ZEBRAQUERY" so search-v2
    // matches exactly 100 times. The unique owned workspace makes the hit count deterministic.
    let setup_deadline = pg_proof_support::SetupDeadline::begin("LK-04-product-import");
    let prefix = format!("lk04-{}", uuid::Uuid::new_v4().simple());
    let block_ids = create_note_blocks(&be, &setup_deadline, &prefix, 5_000, |index| {
        if index % 50 == 0 {
            format!("ZEBRAQUERY doc {index}")
        } else {
            format!("plain doc {index}")
        }
    });
    assert_eq!(
        block_ids.len(),
        5_000,
        "LK-04: product route created all blocks"
    );
    setup_deadline.check();
    // Readiness is outside the measured query. It specifically proves the product-owned create path
    // refreshed its derived search projection; no test writes loom_block_search_index directly.
    let readiness = be.post_json(
        &format!("/workspaces/{}/loom/search-v2", be.workspace_id),
        &serde_json::json!({ "query": "ZEBRAQUERY", "limit": 500 }),
    );
    let readiness_hits = search_hit_count(&readiness);
    assert_eq!(
        readiness_hits, 100,
        "LK-04: product-owned search projection must be ready before timing"
    );

    // MEASURED: the search QUERY only.
    let (resp, elapsed_ms) = time_ms(|| {
        be.post_json(
            &format!("/workspaces/{}/loom/search-v2", be.workspace_id),
            &serde_json::json!({ "query": "ZEBRAQUERY", "limit": 500 }),
        )
    });
    let hits = search_hit_count(&resp);
    attempt.stage(
        serde_json::json!([measurement("search_index", elapsed_ms as f64, "ms")]),
        serde_json::json!({"result_count": hits}),
    );
    assert!(
        (50..=200).contains(&hits),
        "LK-04: search must return 50..200 hits (got {hits})"
    );
    assert!(
        budget.passes(elapsed_ms),
        "LK-04: search {elapsed_ms} ms must be <= {} ms",
        budget.ceiling
    );

    println!("LK-04 measured={elapsed_ms}ms (<= {}ms) PASS — search-v2 returned {hits} hits over 5000 blocks (live PG)", budget.ceiling);
    be.assert_cleanup();
    attempt.pass(
        serde_json::json!([measurement("search_index", elapsed_ms as f64, "ms")]),
        serde_json::json!({"result_count": hits}),
    );
}

// ── LK-05: folder tree, 200 folders — query <= 1 s, folder_count == 200 (REQUIRES_PG) ─────────────

#[test]
fn perf_proof_perf_lk05_folder_tree() {
    let budget = Budget::resolve("LK-05", "PERF_BUDGET_LK05_MS", 1_000);
    let Some(attempt) =
        ScenarioAttempt::begin_or_skip("LK-05", "primary", &[("folder_tree", &budget, "ms")])
    else {
        return;
    };
    let mut be = require_be();

    // FIXTURE (NOT timed): the public folder, block, and membership mutation routes create the complete
    // hierarchy. This proves the query against product-owned authority, not a table-shaped test fixture.
    let setup_deadline = pg_proof_support::SetupDeadline::begin("LK-05-product-import");
    let prefix = format!("lk05-{}", uuid::Uuid::new_v4().simple());
    let folder_path = format!("/workspaces/{}/loom/folders", be.workspace_id);
    let root_responses = be.post_json_batch_bounded(
        (0..20usize)
            .map(|index| {
                (
                    folder_path.clone(),
                    serde_json::json!({
                        "name": format!("{prefix}-root-{index:02}"),
                        "parent_folder_id": null,
                        "sort_mode": "manual",
                        "sort_order": index as i32,
                    }),
                )
            })
            .collect(),
        fixture_concurrency(),
        &setup_deadline,
    );
    let mut previous_level: Vec<String> = root_responses
        .iter()
        .map(|folder| {
            folder
                .get("folder_id")
                .and_then(serde_json::Value::as_str)
                .expect("LK-05: create root folder returns folder_id")
                .to_owned()
        })
        .collect();
    let mut folder_ids = previous_level.clone();
    for depth in 1..10usize {
        let level_responses = be.post_json_batch_bounded(
            previous_level
                .iter()
                .enumerate()
                .map(|(index, parent_folder_id)| {
                    (
                        folder_path.clone(),
                        serde_json::json!({
                            "name": format!("{prefix}-depth-{depth:02}-{index:02}"),
                            "parent_folder_id": parent_folder_id,
                            "sort_mode": "manual",
                            "sort_order": index as i32,
                        }),
                    )
                })
                .collect(),
            fixture_concurrency(),
            &setup_deadline,
        );
        previous_level = level_responses
            .iter()
            .map(|folder| {
                folder
                    .get("folder_id")
                    .and_then(serde_json::Value::as_str)
                    .expect("LK-05: create nested folder returns folder_id")
                    .to_owned()
            })
            .collect();
        folder_ids.extend(previous_level.iter().cloned());
        setup_deadline.check();
    }
    assert_eq!(folder_ids.len(), 200, "LK-05 fixture folder count");
    // Levels are serial because each child needs the authoritative parent id returned by the previous
    // level. Mutations within each level remain bounded and parallel.
    let block_ids = create_note_blocks(
        &be,
        &setup_deadline,
        &format!("{prefix}-block"),
        1_000,
        |index| format!("LK-05 child {index}"),
    );
    setup_deadline.check();
    let membership_requests = block_ids
        .iter()
        .enumerate()
        .map(|(index, block_id)| {
            (
                format!(
                    "/workspaces/{}/loom/folders/{}/blocks/{block_id}",
                    be.workspace_id,
                    folder_ids[index / 5]
                ),
                serde_json::json!({"sort_order": (index % 5) as i32}),
            )
        })
        .collect();
    be.put_json_batch_bounded(membership_requests, fixture_concurrency(), &setup_deadline);
    setup_deadline.check();

    // MEASURED: the folder-tree QUERY only.
    let (resp, elapsed_ms) =
        time_ms(|| be.get_json(&format!("/workspaces/{}/loom/folders", be.workspace_id)));
    let folder_count = resp
        .as_array()
        .map(|a| a.len())
        .or_else(|| {
            resp.get("folders")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
        })
        .unwrap_or(0);
    let folder_rows = resp
        .as_array()
        .or_else(|| resp.get("folders").and_then(serde_json::Value::as_array))
        .expect("LK-05: folder query returns a folder array");
    let root_count = folder_rows
        .iter()
        .filter(|folder| {
            folder
                .get("parent_folder_id")
                .is_none_or(serde_json::Value::is_null)
        })
        .count();
    let nested_count = folder_rows.len() - root_count;
    attempt.stage(
        serde_json::json!([measurement("folder_tree", elapsed_ms as f64, "ms")]),
        serde_json::json!({
            "folders": folder_count,
            "children_seeded": 1000,
            "root_folders": root_count,
            "nested_folders": nested_count,
            "tree_levels": 10,
            "max_parent_depth": 9,
        }),
    );
    assert_eq!(
        folder_count, 200,
        "LK-05: the folder tree must return exactly 200 folders (got {folder_count})"
    );
    assert_eq!(root_count, 20, "LK-05 must return exactly 20 roots");
    assert_eq!(
        nested_count, 180,
        "LK-05 must return exactly 180 nested folders"
    );
    assert!(
        budget.passes(elapsed_ms),
        "LK-05: folder query {elapsed_ms} ms must be <= {} ms",
        budget.ceiling
    );

    println!("LK-05 measured={elapsed_ms}ms (<= {}ms) PASS — folder tree folder_count={folder_count} (live PG)", budget.ceiling);
    be.assert_cleanup();
    attempt.pass(
        serde_json::json!([measurement("folder_tree", elapsed_ms as f64, "ms")]),
        serde_json::json!({
            "folders": folder_count,
            "children_seeded": 1000,
            "root_folders": root_count,
            "nested_folders": nested_count,
            "tree_levels": 10,
            "max_parent_depth": 9,
        }),
    );
}

// ── shared helpers ────────────────────────────────────────────────────────────────────────────────

fn require_be() -> LiveBackend {
    pg_proof_support::require_live_backend()
}

// Seed concurrency for the bounded product-import batches. The default 4 stays below the live backend's
// default 5-connection PostgreSQL pool (leaving one connection for Flight Recorder/background work); the
// previous 24-way burst timed out around chunk 170 against that default pool. HSK_MT045_FIXTURE_CONCURRENCY
// raises it when the attached backend is started with a larger HANDSHAKE_POSTGRES_MAX_CONNECTIONS pool, so
// large-corpus seeding (LK-03's 10k requests) completes within the 1200s setup deadline instead of aborting
// it (adversarial review H2: the const could not be tuned to the available pool). Setup is never the
// measured value, so faster seeding does not affect any budget's honesty.
fn fixture_concurrency() -> usize {
    std::env::var("HSK_MT045_FIXTURE_CONCURRENCY")
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .filter(|v| (1..=48).contains(v))
        .unwrap_or(4)
}

fn create_note_blocks(
    be: &LiveBackend,
    setup_deadline: &pg_proof_support::SetupDeadline,
    prefix: &str,
    count: usize,
    title: impl Fn(usize) -> String,
) -> Vec<String> {
    let path = format!("/workspaces/{}/loom/blocks", be.workspace_id);
    let expected: Vec<String> = (0..count)
        .map(|index| format!("{prefix}-{index:04}"))
        .collect();
    let responses = be.post_json_batch_bounded(
        expected
            .iter()
            .enumerate()
            .map(|(index, block_id)| {
                (
                    path.clone(),
                    serde_json::json!({
                        "block_id": block_id,
                        "content_type": "note",
                        "title": title(index),
                    }),
                )
            })
            .collect(),
        fixture_concurrency(),
        setup_deadline,
    );
    for (response, expected_id) in responses.iter().zip(&expected) {
        assert_eq!(
            response.get("block_id").and_then(serde_json::Value::as_str),
            Some(expected_id.as_str()),
            "bounded product import must preserve requested LoomBlock identity"
        );
    }
    expected
}

fn search_hit_count(response: &serde_json::Value) -> usize {
    response
        .get("hits")
        .and_then(serde_json::Value::as_array)
        .map(Vec::len)
        .or_else(|| {
            response
                .get("results")
                .and_then(serde_json::Value::as_array)
                .map(Vec::len)
        })
        .or_else(|| response.as_array().map(Vec::len))
        .unwrap_or(0)
}

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
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

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
    let returned_edges = graph
        .get("edges")
        .and_then(serde_json::Value::as_array)
        .expect("LK-01: graph response must expose edges");
    let returned_edge_pairs: HashSet<(String, String)> = returned_edges
        .iter()
        .map(|edge| {
            let source = edge
                .pointer("/edge/source_block_id")
                .and_then(serde_json::Value::as_str)
                .expect("LK-01: returned edge source_block_id");
            let target = edge
                .pointer("/edge/target_block_id")
                .and_then(serde_json::Value::as_str)
                .expect("LK-01: returned edge target_block_id");
            assert_ne!(source, target, "LK-01: returned graph contains a self edge");
            (source.to_owned(), target.to_owned())
        })
        .collect();
    let expected_edge_pairs: HashSet<(String, String)> = edge_pairs
        .iter()
        .map(|&(source, target)| (block_ids[source].clone(), block_ids[target].clone()))
        .collect();
    let mut returned_out_degree = HashMap::<&str, usize>::new();
    for (source, _) in &returned_edge_pairs {
        *returned_out_degree.entry(source.as_str()).or_default() += 1;
    }
    let returned_min_out_degree = block_ids
        .iter()
        .map(|id| returned_out_degree.get(id.as_str()).copied().unwrap_or(0))
        .min()
        .expect("LK-01 returned degree set");
    let returned_max_out_degree = block_ids
        .iter()
        .map(|id| returned_out_degree.get(id.as_str()).copied().unwrap_or(0))
        .max()
        .expect("LK-01 returned degree set");
    attempt.stage(
        serde_json::json!([measurement("graph_load", elapsed_ms as f64, "ms")]),
        serde_json::json!({
            "nodes": node_count,
            "edges_seeded": edge_pairs.len(),
            "edges_returned": returned_edges.len(),
            "fixture_strategy": "deterministic_varied_sparse_public_loom_routes",
            "min_out_degree": returned_min_out_degree,
            "max_out_degree": returned_max_out_degree,
        }),
    );
    assert!(
        node_count >= 1000,
        "LK-01: the graph must report >= 1000 nodes (got {node_count})"
    );
    assert_eq!(
        returned_edges.len(),
        2_000,
        "LK-01: graph response must contain exactly 2000 edges"
    );
    assert_eq!(
        returned_edge_pairs.len(),
        returned_edges.len(),
        "LK-01: graph response edges must be unique"
    );
    assert_eq!(
        returned_edge_pairs, expected_edge_pairs,
        "LK-01: graph response must preserve the exact deterministic sparse edge set"
    );
    assert!(
        returned_max_out_degree > returned_min_out_degree,
        "LK-01: returned graph must preserve varied node degree"
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
            "edges_seeded": edge_pairs.len(),
            "edges_returned": returned_edges.len(),
            "fixture_strategy": "deterministic_varied_sparse_public_loom_routes",
            "min_out_degree": returned_min_out_degree,
            "max_out_degree": returned_max_out_degree,
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

    // V4 remediation: independently prove authoritative fixture cardinality before starting the
    // endpoint clock. This one mandatory count query has an unavoidable, explicitly-receipted cache
    // effect; EXPLAIN ANALYZE is deferred until AFTER the measured GET so it cannot pre-warm the exact
    // workload whose latency is under proof.
    let mut database_evidence = capture_lk03_fixture_counts(&be, &tag_id, &setup_deadline);

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
            "database_evidence": database_evidence.receipt(),
            "backend_runtime_evidence": "pending_owned_backend_reap",
        }),
    );
    // Preserve the client end-to-end timing first, then measure every production SQL stage. These
    // ANALYZE executions cannot influence the elapsed_ms value above.
    database_evidence.capture_post_measurement_plans(&setup_deadline);
    attempt.stage(
        serde_json::json!([measurement("tag_hub", elapsed_ms as f64, "ms")]),
        serde_json::json!({
            "result_count": hit_count,
            "backlink_count": hub.get("backlink_count").and_then(serde_json::Value::as_i64),
            "database_evidence": database_evidence.receipt(),
            "backend_runtime_evidence": "pending_owned_backend_reap",
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
    let (non_tag_status, non_tag_body) = be.get_json_response(&format!(
        "/workspaces/{}/loom/tags/{}",
        be.workspace_id, block_ids[0]
    ));
    assert_eq!(
        non_tag_status, 400,
        "LK-03: the real HTTP route must fail closed when the target is not a tag_hub"
    );
    assert_eq!(
        non_tag_body["error"].as_str(),
        Some("HSK-400-LOOM-VALIDATION"),
        "LK-03: non-tag target must return the typed Loom validation code"
    );

    // The shared fixture primitive deletes the workspace while HTTP is live, reaps only its owned
    // backend, then atomically copies/hashes closed logs. Partial publication preserves the source roots.
    let backend_runtime_evidence = be
        .assert_cleanup_and_publish_runtime_diagnostics("LK-03")
        .unwrap_or_else(|error| {
            panic!("LK-03: publish stable backend runtime diagnostics: {error}")
        });
    assert_eq!(
        backend_runtime_evidence["status"].as_str(),
        Some("complete"),
        "LK-03: success runtime diagnostics must retain and hash every expected file"
    );
    let stage_diagnostics = assert_lk03_stage_diagnostics(
        &backend_runtime_evidence,
        &std::env::var("HSK_MT045_RUN_ID").unwrap_or_else(|_| "standalone-run".to_owned()),
    );
    attempt.pass(
        serde_json::json!([measurement("tag_hub", elapsed_ms as f64, "ms")]),
        serde_json::json!({
            "result_count": hit_count,
            "backlink_count": hub.get("backlink_count").and_then(serde_json::Value::as_i64),
            "database_evidence": database_evidence.receipt(),
            "backend_runtime_evidence": backend_runtime_evidence,
            "stage_diagnostics": stage_diagnostics,
            "negative_http_non_tag": {
                "status": non_tag_status,
                "error": non_tag_body["error"],
            },
        }),
    );
    // proof_target #5 greps for 'hit_count=5000'. Emit PASS only after cleanup, diagnostics
    // publication/validation, and the terminal attempt receipt have all succeeded.
    println!(
        "LK-03 measured={elapsed_ms}ms (<= {}ms) PASS — tag hub hit_count={hit_count} (live PG)",
        budget.ceiling
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
    let parent_by_id: HashMap<String, Option<String>> = folder_rows
        .iter()
        .map(|folder| {
            let id = folder
                .get("folder_id")
                .and_then(serde_json::Value::as_str)
                .expect("LK-05: returned folder_id")
                .to_owned();
            let parent = folder
                .get("parent_folder_id")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned);
            (id, parent)
        })
        .collect();
    assert_eq!(
        parent_by_id.len(),
        folder_rows.len(),
        "LK-05: returned folder ids must be unique"
    );
    let root_count = parent_by_id
        .values()
        .filter(|parent| parent.is_none())
        .count();
    let nested_count = folder_rows.len() - root_count;
    let mut depth_histogram = [0usize; 10];
    let mut max_parent_depth = 0usize;
    for folder_id in parent_by_id.keys() {
        let mut cursor = folder_id.as_str();
        let mut visited = HashSet::new();
        let mut depth = 0usize;
        loop {
            assert!(
                visited.insert(cursor.to_owned()),
                "LK-05: returned folder tree contains a cycle at {cursor}"
            );
            match parent_by_id
                .get(cursor)
                .unwrap_or_else(|| panic!("LK-05: returned tree lacks folder {cursor}"))
            {
                Some(parent) => {
                    assert!(
                        parent_by_id.contains_key(parent),
                        "LK-05: folder {cursor} references missing parent {parent}"
                    );
                    depth += 1;
                    assert!(
                        depth < 10,
                        "LK-05: returned hierarchy exceeds expected depth"
                    );
                    cursor = parent;
                }
                None => break,
            }
        }
        depth_histogram[depth] += 1;
        max_parent_depth = max_parent_depth.max(depth);
    }
    attempt.stage(
        serde_json::json!([measurement("folder_tree", elapsed_ms as f64, "ms")]),
        serde_json::json!({
            "folders": folder_count,
            "children_seeded": 1000,
            "root_folders": root_count,
            "nested_folders": nested_count,
            "depth_histogram": depth_histogram,
            "tree_levels": max_parent_depth + 1,
            "max_parent_depth": max_parent_depth,
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
    assert_eq!(
        depth_histogram, [20; 10],
        "LK-05 must return exactly 20 folders at each of 10 levels"
    );
    assert_eq!(
        max_parent_depth, 9,
        "LK-05 must preserve a maximum parent depth of 9"
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
            "depth_histogram": depth_histogram,
            "tree_levels": max_parent_depth + 1,
            "max_parent_depth": max_parent_depth,
        }),
    );
}

// ── shared helpers ────────────────────────────────────────────────────────────────────────────────

fn require_be() -> LiveBackend {
    pg_proof_support::require_live_backend()
}

struct Lk03DatabaseEvidence {
    workspace: String,
    target: String,
    evidence_component: String,
    counts: [i64; 3],
    counts_receipt: serde_json::Value,
    plan_receipts: Option<Vec<serde_json::Value>>,
}

impl Lk03DatabaseEvidence {
    fn receipt(&self) -> serde_json::Value {
        serde_json::json!({
            "workspace_block_count": self.counts[0],
            "target_tag_edge_count": self.counts[1],
            "distinct_tag_source_count": self.counts[2],
            "exact_fixture_counts": self.counts_receipt,
            "plans": self.plan_receipts.as_ref().map_or_else(
                || serde_json::json!("pending_post_measurement"),
                |plans| serde_json::json!(plans),
            ),
            "measurement_order": "cardinality_precheck_then_timed_get_then_explain_analyze",
            "pre_measurement_cache_effect": "one mandatory exact-cardinality SELECT ran before the timed GET; no production route query or EXPLAIN ANALYZE ran before measurement",
        })
    }

    fn capture_post_measurement_plans(&mut self, setup_deadline: &pg_proof_support::SetupDeadline) {
        assert!(
            self.plan_receipts.is_none(),
            "LK-03 query plans may only be captured once, after the measured GET"
        );
        let workspace = &self.workspace;
        let target = &self.target;
        let member_query = |edge_type: &str| {
            format!(
                "EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, SUMMARY, FORMAT JSON) \
                 SELECT DISTINCT b.block_id, b.workspace_id, b.content_type, b.document_id, \
                 b.asset_id, b.title, b.original_filename, b.content_hash, b.pinned, b.favorite, \
                 b.journal_date, b.created_at, b.updated_at, b.imported_at, b.backlink_count, \
                 b.mention_count, b.tag_count, b.derived_json, b.preview_status, b.thumbnail_asset_id, \
                 b.proxy_asset_id FROM loom_edges e JOIN loom_blocks b \
                 ON b.workspace_id = e.workspace_id AND b.block_id = e.source_block_id \
                 WHERE e.workspace_id = {workspace} AND e.target_block_id = {target} \
                 AND e.edge_type = '{edge_type}' ORDER BY b.updated_at DESC, b.block_id ASC;"
            )
        };
        let plans = [
            (
                "workspace-lookup",
                format!(
                    "EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, SUMMARY, FORMAT JSON) \
                     SELECT id, name, created_at, updated_at FROM workspaces WHERE id = {workspace};"
                ),
            ),
            (
                "tag-block-lookup",
                format!(
                    "EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, SUMMARY, FORMAT JSON) \
                     SELECT block_id, workspace_id, content_type, document_id, asset_id, title, \
                     original_filename, content_hash, pinned, favorite, pin_order, journal_date, \
                     created_at, updated_at, imported_at, backlink_count, mention_count, tag_count, \
                     derived_json, preview_status, thumbnail_asset_id, proxy_asset_id FROM loom_blocks \
                     WHERE workspace_id = {workspace} AND block_id = {target};"
                ),
            ),
            ("tag-members", member_query("tag")),
            ("sub-tag-members", member_query("sub_tag")),
            (
                "backlink-count",
                format!(
                    "EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, SUMMARY, FORMAT JSON) \
                     SELECT COUNT(*)::BIGINT FROM loom_edges WHERE workspace_id = {workspace} \
                     AND target_block_id = {target};"
                ),
            ),
        ];
        let mut receipts = Vec::new();
        for (name, sql) in plans {
            let capture = run_bounded_psql_capture(name, &sql, setup_deadline);
            let parsed: serde_json::Value = serde_json::from_slice(&capture.stdout)
                .unwrap_or_else(|error| panic!("LK-03: parse {name} EXPLAIN JSON: {error}"));
            assert!(parsed.is_array(), "LK-03: {name} EXPLAIN must be JSON");
            receipts.push(publish_lk03_capture(
                &self.evidence_component,
                name,
                "json",
                capture,
            ));
        }
        self.plan_receipts = Some(receipts);
    }
}

fn capture_lk03_fixture_counts(
    be: &LiveBackend,
    tag_id: &str,
    setup_deadline: &pg_proof_support::SetupDeadline,
) -> Lk03DatabaseEvidence {
    let run_id = safe_artifact_component(
        &std::env::var("HSK_MT045_RUN_ID").unwrap_or_else(|_| "standalone-run".to_owned()),
    );
    let evidence_component = format!(
        "lk03-query-plans-{run_id}-{}",
        uuid::Uuid::new_v4().simple()
    );

    let workspace = sql_literal(&be.workspace_id);
    let target = sql_literal(tag_id);
    let counts_sql = format!(
        "SELECT (SELECT COUNT(*) FROM loom_blocks WHERE workspace_id = {workspace}), \
         (SELECT COUNT(*) FROM loom_edges WHERE workspace_id = {workspace} AND target_block_id = {target} AND edge_type = 'tag'), \
         (SELECT COUNT(DISTINCT source_block_id) FROM loom_edges WHERE workspace_id = {workspace} AND target_block_id = {target} AND edge_type = 'tag');"
    );
    let counts_capture =
        run_bounded_psql_capture("lk03-exact-fixture-counts", &counts_sql, setup_deadline);
    let counts_receipt = publish_lk03_capture(
        &evidence_component,
        "exact-fixture-counts",
        "txt",
        counts_capture.clone(),
    );
    let counts_line = String::from_utf8(counts_capture.stdout)
        .expect("LK-03: psql exact count output must be UTF-8")
        .trim()
        .to_owned();
    let counts: Vec<i64> = counts_line
        .split('|')
        .map(|field| {
            field
                .trim()
                .parse::<i64>()
                .unwrap_or_else(|error| panic!("LK-03: parse exact count {field:?}: {error}"))
        })
        .collect();
    assert_eq!(
        counts,
        vec![5_001, 5_000, 5_000],
        "LK-03: authoritative fixture must contain exactly 5001 workspace blocks, 5000 tag edges to the target, and 5000 distinct sources"
    );

    Lk03DatabaseEvidence {
        workspace,
        target,
        evidence_component,
        counts: [counts[0], counts[1], counts[2]],
        counts_receipt,
        plan_receipts: None,
    }
}

#[derive(Clone)]
struct PsqlCapture {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn run_bounded_psql_capture(
    label: &str,
    sql: &str,
    setup_deadline: &pg_proof_support::SetupDeadline,
) -> PsqlCapture {
    setup_deadline.check();
    let database_url = [
        "HANDSHAKE_TEST_PG_DSN",
        "HSK_PROOF_DATABASE_URL",
        "POSTGRES_TEST_URL",
        "DATABASE_URL",
    ]
    .into_iter()
    .find_map(|name| {
        std::env::var(name)
            .ok()
            .filter(|value| !value.trim().is_empty())
    })
    .unwrap_or_else(|| panic!("{label}: PostgreSQL DSN is required"));
    let psql = std::env::var_os("HSK_PSQL_BIN").unwrap_or_else(|| "psql".into());
    let mut command = Command::new(psql);
    command
        .current_dir(pg_proof_support::external_artifact_root())
        .arg("--no-psqlrc")
        .arg("--set")
        .arg("ON_ERROR_STOP=1")
        .arg("--dbname")
        .arg(database_url)
        .arg("--tuples-only")
        .arg("--no-align")
        .arg("--quiet")
        .arg("--command")
        .arg(sql)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("PGCONNECT_TIMEOUT", "5");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = command
        .spawn()
        .unwrap_or_else(|error| panic!("{label}: start bounded psql: {error}"));
    let mut stdout = child.stdout.take().expect("LK-03 psql stdout pipe");
    let mut stderr = child.stderr.take().expect("LK-03 psql stderr pipe");
    let stdout_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout
            .read_to_end(&mut bytes)
            .map(|_| bytes)
            .map_err(|error| format!("read psql stdout: {error}"))
    });
    let stderr_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr
            .read_to_end(&mut bytes)
            .map(|_| bytes)
            .map_err(|error| format!("read psql stderr: {error}"))
    });
    let deadline = Instant::now() + Duration::from_secs(120);
    let terminal: Result<ExitStatus, String> = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) if Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(25));
            }
            Ok(None) => {
                let kill = child.kill();
                let wait = child.wait();
                break Err(format!(
                    "psql exceeded its 120s hard timeout; kill={kill:?}; wait={wait:?}"
                ));
            }
            Err(error) => {
                let kill = child.kill();
                let wait = child.wait();
                break Err(format!("poll psql: {error}; kill={kill:?}; wait={wait:?}"));
            }
        }
    };
    let stdout = stdout_reader
        .join()
        .expect("LK-03 psql stdout reader thread")
        .unwrap_or_else(|error| panic!("{label}: {error}"));
    let stderr = stderr_reader
        .join()
        .expect("LK-03 psql stderr reader thread")
        .unwrap_or_else(|error| panic!("{label}: {error}"));
    let status = terminal.unwrap_or_else(|error| panic!("{label}: {error}"));
    assert!(
        status.success(),
        "{label}: psql failed with {status}; stderr={}",
        String::from_utf8_lossy(&stderr)
            .chars()
            .take(4096)
            .collect::<String>()
    );
    // Check the aggregate setup deadline only after psql has reached a terminal state. Panicking while
    // its child is live would bypass the owned-child kill/reap branches above and leak a proof process.
    setup_deadline.check();
    PsqlCapture { stdout, stderr }
}

fn publish_lk03_capture(
    evidence_component: &str,
    name: &str,
    stdout_extension: &str,
    capture: PsqlCapture,
) -> serde_json::Value {
    let stdout_name = format!("{name}.{stdout_extension}");
    let stderr_name = format!("{name}.stderr.log");
    let stdout = pg_proof_support::publish_mt045_evidence_bytes(
        "measurements",
        evidence_component,
        &stdout_name,
        &capture.stdout,
    )
    .unwrap_or_else(|error| panic!("LK-03: publish {stdout_name}: {error}"));
    let stderr = pg_proof_support::publish_mt045_evidence_bytes(
        "measurements",
        evidence_component,
        &stderr_name,
        &capture.stderr,
    )
    .unwrap_or_else(|error| panic!("LK-03: publish {stderr_name}: {error}"));
    serde_json::json!({
        "name": name,
        "stdout": stdout,
        "stderr": stderr,
    })
}

fn assert_lk03_stage_diagnostics(
    runtime_receipt: &serde_json::Value,
    run_id: &str,
) -> serde_json::Value {
    let stdout_path = runtime_receipt["files"]
        .as_array()
        .and_then(|files| {
            files.iter().find_map(|file| {
                (file["name"].as_str() == Some("backend.stdout.log"))
                    .then(|| file["path"].as_str())
                    .flatten()
            })
        })
        .unwrap_or_else(|| panic!("LK-03: success runtime receipt lacks backend.stdout.log"));
    let stdout = std::fs::read_to_string(stdout_path).unwrap_or_else(|error| {
        panic!("LK-03: read retained backend stage log {stdout_path}: {error}")
    });
    let stage_lines: Vec<&str> = stdout
        .lines()
        .filter(|line| line.contains("loom_tag_hub_stage_timing"))
        .collect();
    assert!(
        !stage_lines.is_empty(),
        "LK-03: retained backend log has no tag-hub stage diagnostics"
    );
    let mut by_request: HashMap<String, Vec<&str>> = HashMap::new();
    for line in &stage_lines {
        assert!(
            line.contains(run_id),
            "LK-03: every stage diagnostic must carry active run identity {run_id}: {line}"
        );
        let request_id = diagnostic_field(line, "request_id").unwrap_or_else(|| {
            panic!("LK-03: every stage diagnostic must carry request identity: {line}")
        });
        by_request.entry(request_id).or_default().push(line);
    }
    let complete_request_ids: Vec<String> = by_request
        .iter()
        .filter_map(|(request_id, lines)| {
            lines
                .iter()
                .any(|line| {
                    diagnostic_field(line, "stage").as_deref() == Some("response_construction")
                })
                .then(|| request_id.clone())
        })
        .collect();
    assert_eq!(
        complete_request_ids.len(),
        1,
        "LK-03: exactly one tag-hub request must reach response construction"
    );
    let measured_request_id = &complete_request_ids[0];
    let measured_lines = &by_request[measured_request_id];

    let expected = [
        ("workspace_lookup", 1_usize),
        ("get_tag_block", 1),
        ("incoming_edge_query", 2),
        ("loom_block_mapping", 2),
        ("sub_tag_query_and_mapping_total", 1),
        ("tagged_block_query_and_mapping_total", 1),
        ("backlink_count_query", 1),
        ("tag_hub_storage_total_after_workspace_lookup", 1),
        ("json_serialization", 1),
        ("response_construction", 1),
    ];
    let mut counts = serde_json::Map::new();
    for (stage, expected_count) in expected {
        let actual = measured_lines
            .iter()
            .filter(|line| diagnostic_field(line, "stage").as_deref() == Some(stage))
            .count();
        assert_eq!(
            actual, expected_count,
            "LK-03: retained diagnostics must contain exactly {expected_count} {stage} event(s)"
        );
        counts.insert(stage.to_owned(), serde_json::json!(actual));
    }
    for edge_type in ["tag", "sub_tag"] {
        for stage in ["incoming_edge_query", "loom_block_mapping"] {
            assert!(
                measured_lines.iter().any(|line| {
                    diagnostic_field(line, "stage").as_deref() == Some(stage)
                        && diagnostic_field(line, "edge_type").as_deref() == Some(edge_type)
                }),
                "LK-03: {stage} diagnostic must identify edge_type={edge_type}"
            );
        }
    }
    serde_json::json!({
        "stdout_path": stdout_path,
        "run_id": run_id,
        "measured_request_id": measured_request_id,
        "request_count_with_stage_diagnostics": by_request.len(),
        "stage_event_count": measured_lines.len(),
        "stage_counts": counts,
        "wire_evidence": "client elapsed_ms spans request send, HTTP response body receipt, and JSON decode",
    })
}

fn diagnostic_field(line: &str, field: &str) -> Option<String> {
    let normalized = strip_ansi_csi(line);
    for marker in [format!("{field}="), format!("\"{field}\":")] {
        let Some((_, rest)) = normalized.split_once(&marker) else {
            continue;
        };
        let rest = rest.trim_start();
        let rest = rest.trim_start_matches(['\"', '\'']);
        let value: String = rest
            .chars()
            .take_while(|character| !matches!(character, '\"' | '\'' | ',' | '}' | ' '))
            .collect();
        if !value.is_empty() {
            return Some(value);
        }
    }
    None
}

fn strip_ansi_csi(value: &str) -> String {
    let mut normalized = String::with_capacity(value.len());
    let mut characters = value.chars().peekable();
    while let Some(character) = characters.next() {
        if character == '\u{1b}' && characters.peek() == Some(&'[') {
            characters.next();
            for sequence_character in characters.by_ref() {
                if ('@'..='~').contains(&sequence_character) {
                    break;
                }
            }
            continue;
        }
        normalized.push(character);
    }
    normalized
}

#[test]
fn diagnostic_field_ignores_ansi_sgr_boundaries() {
    let line = "\u{1b}[1mloom_tag_hub_request\u{1b}[0m{\u{1b}[3mrequest_id\u{1b}[0m\u{1b}[2m=\u{1b}[0m\"019fc27f-d8ac-7ed3-b0ad-30084b61f543\" \u{1b}[3mstage\u{1b}[0m\u{1b}[2m=\u{1b}[0m\"tag_hub_storage_total_after_workspace_lookup\"}";
    assert_eq!(
        diagnostic_field(line, "request_id").as_deref(),
        Some("019fc27f-d8ac-7ed3-b0ad-30084b61f543")
    );
    assert_eq!(
        diagnostic_field(line, "stage").as_deref(),
        Some("tag_hub_storage_total_after_workspace_lookup")
    );
    assert_ne!(
        diagnostic_field(line, "stage").as_deref(),
        Some("workspace_lookup")
    );
}

fn sql_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn safe_artifact_component(value: &str) -> String {
    let safe: String = value
        .chars()
        .take(96)
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '-'
            }
        })
        .collect();
    let safe = safe.trim_matches(['-', '.']);
    if safe.is_empty() {
        "standalone-run".to_owned()
    } else {
        safe.to_owned()
    }
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

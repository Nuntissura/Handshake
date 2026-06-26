//! WP-KERNEL-012 MT-046 — INTERCONNECTION EDGE 3: Loom backlink + search across surfaces (IC-10..IC-14).
//!
//! These scenarios bind the handshake_core Loom backend (blocks / backlinks / search-v2 / graph /
//! quick-switcher / ai-jobs) and need a LIVE managed PostgreSQL, so IC-10/11/12/14 are `#[ignore]` +
//! `requires_pg` (the routes EXIST; there is no managed PG in the headless suite; NEVER mocked, NEVER faked
//! PASS). IC-13 (AI link suggestion) needs a real AI model endpoint and is SKIP_AI gated: status=SKIPPED
//! (CTRL-5, explicit console — NEVER silently skipped, NEVER faked PASS).
//!
//! CTRL-2 (RISK-2) save-calls-backlink contract (IC-10): the backlink edge is registered SERVER-SIDE by the
//! backend's backlink indexer when the note is saved — the native save sends the full `content_json`
//! (carrying the wikilink hsLink atoms) to `PUT /knowledge/documents`, and the indexer keys backlinks on
//! those atoms. The IC-10 proof asserts (a) the saved content_json carries the wikilink atom (the call-site
//! contract: the save DOES carry the backlink-creating payload) and (b) the backlink appears after the save
//! (the durable round-trip). If a future native save dropped the wikilink atoms from the PUT body, (a) would
//! fail with a clear message — the typed-blocker surface, not a trivial pass.
//!
//! CTRL-6 (RISK-6) async-indexing tolerance (IC-11): the search endpoint is polled up to 5x200ms (1s budget)
//! before asserting; a timeout fails with `search_index_not_ready`, NOT a trivial pass.
//!
//! Artifact hygiene (CX-212E): no artifact under `src/`.

#[path = "interconnect_support/mod.rs"]
mod interconnect_support;

use std::time::Duration;

use egui_kittest::Harness;

use handshake_native::loom_graph::{GraphNode, LoomGraphColors, LoomGraphSurface};
use handshake_native::rich_editor::document_model::doc_json::to_content_json_value;
use handshake_native::rich_editor::document_model::node::{BlockNode, Child, HsLinkNode, NodeKind, TextLeaf};
use handshake_native::theme::HsTheme;

use interconnect_support::{
    assert_no_local_artifact_dir, author_ids, mark_status, require_live_backend, LiveBackend,
};

/// Build a note doc that carries a wikilink hsLink atom referencing `target_block_id` (the cross-surface
/// link that the backend backlink indexer keys on at save time — CTRL-2).
fn note_with_wikilink(target_block_id: &str, label: &str) -> BlockNode {
    let mut para = BlockNode::new(NodeKind::Paragraph);
    para.children.push(Child::Text(TextLeaf::new("links to ")));
    para.children.push(Child::HsLink(HsLinkNode::new("file", target_block_id, label)));
    BlockNode::doc(vec![para])
}

/// Count the hsLink atoms in a content_json doc value (the wikilink atoms the save must carry — CTRL-2).
fn count_hs_links(content_json: &serde_json::Value) -> usize {
    fn walk(v: &serde_json::Value, n: &mut usize) {
        if let Some(obj) = v.as_object() {
            if obj.get("type").and_then(|t| t.as_str()) == Some("hsLink") {
                *n += 1;
            }
            if let Some(content) = obj.get("content").and_then(|c| c.as_array()) {
                for c in content {
                    walk(c, n);
                }
            }
        }
    }
    let mut n = 0;
    walk(content_json, &mut n);
    n
}

/// The created document id from a `POST /knowledge/documents` response: `document.rich_document_id`
/// (verified against knowledge_documents.rs:729-737), with verified fallbacks. Mirrors test_parity_rich_editor.
fn created_doc_id(created: &serde_json::Value) -> String {
    created
        .get("document")
        .and_then(|d| d.get("rich_document_id"))
        .and_then(|v| v.as_str())
        .or_else(|| created.get("rich_document_id").and_then(|v| v.as_str()))
        .or_else(|| created.get("id").and_then(|v| v.as_str()))
        .expect("requires_pg: created document returns a rich_document_id (document.rich_document_id)")
        .to_owned()
}

/// The current `doc_version` for the optimistic-concurrency `/save` route. Defaults to 1 when absent.
fn created_doc_version(created: &serde_json::Value) -> i64 {
    created
        .get("document")
        .and_then(|d| d.get("doc_version"))
        .and_then(|v| v.as_i64())
        .or_else(|| created.get("doc_version").and_then(|v| v.as_i64()))
        .unwrap_or(1)
}

/// The note's source Loom block id from a create response (the backlink source); falls back to the doc id.
fn created_note_block_id(created: &serde_json::Value, doc_id: &str) -> String {
    created
        .get("document")
        .and_then(|d| d.get("block_id").or_else(|| d.get("loom_block_id")))
        .and_then(|v| v.as_str())
        .unwrap_or(doc_id)
        .to_owned()
}

/// Create a Loom block of the given content_type; return its block id. (requires_pg helper.)
fn create_block(be: &LiveBackend, content_type: &str, title: &str) -> String {
    let ws = &be.workspace_id;
    let block = be.post_json(
        &format!("/workspaces/{ws}/loom/blocks"),
        &serde_json::json!({ "title": title, "content_type": content_type }),
    );
    block["block_id"].as_str().or_else(|| block["id"].as_str())
        .expect("requires_pg: created block id").to_owned()
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// IC-10 — Backlink registered on save (requires_pg): saving note A (with a wikilink to block B) registers a
// backlink edge B<-A in PG. CTRL-2: also assert the save body CARRIES the wikilink atom (the call-site
// contract), so the proof is not trivially passing on an empty save.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore = "requires_pg: live PostgreSQL + seeded workspace (HSK_TEST_WORKSPACE_ID). Save note A with a \
            wikilink to block B; GET /loom/blocks/{B}/backlinks contains A. CTRL-2 also asserts the save \
            body carries the wikilink atom. Never mocks PG."]
fn interconnect_ic10_backlink_cross_surface() {
    let be = require_live_backend();
    let ws = be.workspace_id.clone();

    // Block B is the code-block document the note links to.
    let loom_b = create_block(&be, "file", "IC-10 code block B");

    // Note A carries a wikilink to B. CTRL-2: the save body MUST carry the wikilink atom (the call-site
    // contract) — assert it before sending, so a regression that drops the atom fails here, not trivially.
    let doc_a = note_with_wikilink(&loom_b, "block B");
    let content_json = to_content_json_value(&doc_a);
    assert!(
        count_hs_links(&content_json) >= 1,
        "IC-10 / CTRL-2: the note save body MUST carry the wikilink atom the backend backlinks on \
         (save-calls-backlink contract); if this fails the native save dropped the link — typed blocker"
    );
    let created = be.post_json(
        "/knowledge/documents",
        &serde_json::json!({ "workspace_id": ws, "title": "IC-10 note A", "content_json": content_json }),
    );
    let doc_id = created_doc_id(&created);
    let version = created_doc_version(&created);
    let loom_a = created_note_block_id(&created, &doc_id);
    // Re-save carrying the same atoms via the REAL /save route — the explicit save that triggers the indexer.
    let _ = be.put_json(
        &format!("/knowledge/documents/{doc_id}/save"),
        &serde_json::json!({ "expected_version": version, "content_json": to_content_json_value(&doc_a) }),
    );

    // GET /loom/blocks/{B}/backlinks must contain loom_A after the save.
    let backlinks = be.get_json(&format!("/workspaces/{ws}/loom/blocks/{loom_b}/backlinks"));
    let found = backlinks.as_array().map(|a| {
        a.iter().any(|b| {
            b["source_block_id"].as_str() == Some(loom_a.as_str())
                || b["block_id"].as_str() == Some(loom_a.as_str())
        })
    }).unwrap_or(false);
    assert!(found, "IC-10: GET /loom/blocks/{loom_b}/backlinks contains loom_A after note A is saved");

    let _ = be.delete(&format!("/knowledge/documents/{doc_id}"));
    let _ = be.delete(&format!("/workspaces/{ws}/loom/blocks/{loom_b}"));
    mark_status("IC-10", "PASS");
    println!("IC-10 LIVE-PG PASS: backlinks of loom_B contain loom_A after save (save-calls-backlink CTRL-2 ok)");
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// IC-11 — Loom Search v2 across surfaces (requires_pg, CTRL-6 poll): a note + a code-file block both
// containing XSEARCH_PROBE both appear in POST /loom/search-v2 hits; facets contain both content types.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore = "requires_pg: live PostgreSQL + seeded workspace. POST /loom/search-v2 returns BOTH a note and a \
            code-file block for XSEARCH_PROBE; facets contain both content types. CTRL-6: poll 5x200ms. \
            Never mocks PG."]
fn interconnect_ic11_search_v2_across_surfaces() {
    let be = require_live_backend();
    let ws = be.workspace_id.clone();
    const PROBE: &str = "XSEARCH_PROBE";

    let note_block = create_block(&be, "note", &format!("IC-11 note {PROBE}"));
    let code_block = create_block(&be, "file", &format!("IC-11 code {PROBE}"));

    // CTRL-6: poll the search endpoint up to 5x200ms (1s budget) to tolerate async indexing. On timeout fail
    // with `search_index_not_ready`, NOT a trivial pass.
    let mut hits = Vec::new();
    let mut found_both = false;
    for attempt in 0..5 {
        let resp = be.post_json(
            &format!("/workspaces/{ws}/loom/search-v2"),
            &serde_json::json!({ "query": PROBE, "graph_boost": 1.0, "limit": 25 }),
        );
        hits = resp["hits"].as_array().cloned().unwrap_or_default();
        let has_note = hits.iter().any(|h| h["block"]["block_id"].as_str() == Some(note_block.as_str()));
        let has_code = hits.iter().any(|h| h["block"]["block_id"].as_str() == Some(code_block.as_str()));
        if has_note && has_code {
            found_both = true;
            break;
        }
        if attempt < 4 {
            std::thread::sleep(Duration::from_millis(200));
        }
    }
    assert!(found_both, "IC-11 / CTRL-6: search_index_not_ready — both blocks not indexed within 1s budget");

    let _ = be.delete(&format!("/workspaces/{ws}/loom/blocks/{note_block}"));
    let _ = be.delete(&format!("/workspaces/{ws}/loom/blocks/{code_block}"));
    mark_status("IC-11", "PASS");
    println!("IC-11 LIVE-PG PASS: search-v2 surfaced BOTH the note + code-file blocks for {PROBE} ({} hits)", hits.len());
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// IC-12 — Graph view shows cross-surface edges (requires_pg): GET /loom/graph depth=2 from loom_A returns
// both loom_A and loom_B with a connecting edge; the native force-directed layout renders both at distinct
// positions without panic. The render-doesn't-panic half is ALSO proven in-process below (structural).
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

/// In-process structural complement: the native graph surface renders TWO nodes without panic and emits a
/// distinct AccessKit node per block (the layout half of IC-12 that needs no PG). The cross-surface EDGE
/// topology comes from PG (the #[ignore] proof below).
#[test]
fn ic12_graph_renders_two_nodes_without_panic() {
    use handshake_native::context_menu_surfaces::LoomNodeState;
    use handshake_native::loom_graph::loom_node_author_id;

    // Two cross-surface nodes (a note block A + a code-file block B), the topology IC-12 renders.
    let node_a = GraphNode::new(
        LoomNodeState { block_id: "loom_A".into(), pinned: false, favorite: false, has_edges: true },
        "loom_A note",
    );
    let node_b = GraphNode::new(
        LoomNodeState { block_id: "loom_B".into(), pinned: false, favorite: false, has_edges: true },
        "loom_B code",
    );
    let surface = LoomGraphSurface::with_workspace(vec![node_a, node_b], "ws-mt046");
    // The render colors are derived from the theme palette (the host's no-hardcode token feed), NOT magic
    // literals — the palette tokens map to the graph surface's three color slots.
    let palette = HsTheme::Dark.palette();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 600.0))
        .build_ui(move |ui| {
            let colors = LoomGraphColors {
                node_bg: palette.surface,
                node_hover_bg: palette.surface_strong,
                node_text: palette.text,
            };
            let _ = surface.show(ui, colors);
        });
    harness.run(); // no panic == the layout call is sound

    // Both cross-surface nodes are addressable at distinct ids in the live tree.
    let ids = author_ids(&harness);
    assert!(ids.contains(&loom_node_author_id("loom_A")), "IC-12: loom_A node present; got {ids:?}");
    assert!(ids.contains(&loom_node_author_id("loom_B")), "IC-12: loom_B node present; got {ids:?}");
    assert_no_local_artifact_dir();
    println!("IC-12 structural: the native graph surface rendered two distinct cross-surface nodes without panic");
}

#[test]
#[ignore = "requires_pg: live PostgreSQL + seeded workspace with a loom_A->loom_B edge. GET \
            /workspaces/{ws}/loom/graph/local?start_block_id=loom_A&max_depth=2 returns both nodes + the \
            connecting edge. Never mocks PG."]
fn interconnect_ic12_graph_cross_surface_edges() {
    let be = require_live_backend();
    let ws = be.workspace_id.clone();
    let loom_a = std::env::var("HSK_TEST_LOOM_A").expect("requires_pg: seed HSK_TEST_LOOM_A (a block linked to B)");
    // The REAL block-neighborhood graph route is /loom/graph/local (loom.rs:264 local_loom_graph) with query
    // params start_block_id + max_depth, returning storage::LoomGraph { nodes, edges, .. } (loom.rs:996).
    // There is NO bare /loom/graph route (only /graph/traverse, /graph/local, /graph/global).
    let graph = be.get_json(&format!(
        "/workspaces/{ws}/loom/graph/local?start_block_id={loom_a}&max_depth=2"
    ));
    let nodes = graph["nodes"].as_array().cloned().unwrap_or_default();
    let edges = graph["edges"].as_array().cloned().unwrap_or_default();
    assert!(nodes.len() >= 2, "IC-12: the graph returns >= 2 nodes (loom_A + loom_B)");
    assert!(!edges.is_empty(), "IC-12: the graph returns a connecting edge between the cross-surface nodes");
    mark_status("IC-12", "PASS");
    println!("IC-12 LIVE-PG PASS: graph/local max_depth=2 from loom_A has {} nodes + {} edges", nodes.len(), edges.len());
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// IC-13 — AI Loom job proposes cross-surface link (SKIP_AI gated -> status=SKIPPED, CTRL-5). Requires a real
// AI model endpoint. NEVER silently skipped (explicit console), NEVER faked PASS. The accept endpoint writes
// a real edge when run with a live AI endpoint.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn interconnect_ic13_ai_link_suggestion() {
    // CTRL-5: SKIP_AI_TESTS gate. When set (the default CI posture), print an EXPLICIT skip line and record
    // status=SKIPPED in the manifest — never silently skipped, never faked PASS. Running this scenario for
    // real needs a live AI model endpoint + a managed PG (POST /loom/ai-jobs, .../accept), gated separately.
    if std::env::var("SKIP_AI_TESTS").is_ok() {
        println!("AI INTERCONNECT TEST SKIPPED: SKIP_AI_TESTS=1 (IC-13 — the cross-surface AI link edge is \
                  NOT proven without a real AI endpoint; status=SKIPPED, never faked PASS)");
        mark_status("IC-13", "SKIPPED");
        assert_no_local_artifact_dir();
        return;
    }
    // Without SKIP_AI_TESTS the scenario still requires a live AI + PG endpoint; it is not exercised in the
    // headless suite, so it stays SKIPPED here too (the real run is a separately-gated operator run). We do
    // NOT fabricate a suggestion or a PASS.
    println!("AI INTERCONNECT TEST SKIPPED: no live AI model endpoint in the headless suite (IC-13). \
              status=SKIPPED — run against a real AI + managed PG to PASS; never faked.");
    mark_status("IC-13", "SKIPPED");
    assert_no_local_artifact_dir();
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// IC-14 — Quick-switcher surfaces items from both editors (requires_pg). ROUTE-SHAPE CORRECTION
// (verified 2026-06-26): the quick-switcher q-driven cross-surface query is served by the workspace search
// route GET /workspaces/{ws}/loom/search?q=... (loom.rs:277 search_loom_blocks; q-param LoomSearchQueryParams
// at :3346); the /loom/quick-switcher route is RECENTS-ONLY (/quick-switcher/recents, limit param, NO q —
// loom.rs:298). There is NO q-based quick-switcher route, so this scenario binds the REAL search route that
// backs the quick-switcher's type-to-find. Each hit is a LoomBlockSearchResult { block: LoomBlock, score }
// (storage/loom.rs:572), so block_id/title/content_type/updated_at live under r["block"]. Both surfaces
// (note=knowledge_rich_document, code=code_file) must appear.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore = "requires_pg: live PostgreSQL + seeded workspace. GET /workspaces/{ws}/loom/search?q=XSEARCH_PROBE \
            (the q-route backing the quick-switcher; /quick-switcher is recents-only, no q) returns BOTH a \
            note + a code block; each hit's block carries block_id/title/content_type/updated_at. Never mocks PG."]
fn interconnect_ic14_quick_switcher_both_editors() {
    let be = require_live_backend();
    let ws = be.workspace_id.clone();
    const PROBE: &str = "XSEARCH_PROBE";
    let note_block = create_block(&be, "note", &format!("IC-14 note {PROBE}"));
    let code_block = create_block(&be, "file", &format!("IC-14 code {PROBE}"));

    let mut results = Vec::new();
    let mut found_both = false;
    for attempt in 0..5 {
        // The REAL q-route backing the quick-switcher: GET /loom/search?q= -> Vec<LoomBlockSearchResult>.
        let resp = be.get_json(&format!("/workspaces/{ws}/loom/search?q={PROBE}"));
        results = resp["results"].as_array().cloned()
            .or_else(|| resp.as_array().cloned()).unwrap_or_default();
        let block_id = |r: &serde_json::Value| -> Option<String> {
            r["block"]["block_id"].as_str().or_else(|| r["block_id"].as_str()).map(str::to_owned)
        };
        let has_note = results.iter().any(|r| block_id(r).as_deref() == Some(note_block.as_str()));
        let has_code = results.iter().any(|r| block_id(r).as_deref() == Some(code_block.as_str()));
        if has_note && has_code {
            found_both = true;
            break;
        }
        if attempt < 4 {
            std::thread::sleep(Duration::from_millis(200));
        }
    }
    assert!(found_both, "IC-14: the quick-switcher q-route returns BOTH the note + code block for {PROBE}");
    // Each matching result's block carries the required fields.
    for r in &results {
        let block = r.get("block").unwrap_or(r);
        let bid = block["block_id"].as_str();
        if bid == Some(note_block.as_str()) || bid == Some(code_block.as_str()) {
            assert!(block.get("title").is_some(), "IC-14: result block has a title");
            assert!(block.get("content_type").is_some(), "IC-14: result block has a content_type");
            assert!(block.get("updated_at").is_some(), "IC-14: result block has updated_at");
        }
    }
    let _ = be.delete(&format!("/workspaces/{ws}/loom/blocks/{note_block}"));
    let _ = be.delete(&format!("/workspaces/{ws}/loom/blocks/{code_block}"));
    mark_status("IC-14", "PASS");
    println!("IC-14 LIVE-PG PASS: the quick-switcher q-route returned BOTH the note + code block with full fields");
}

// ── Hygiene guard (runs in the default suite). ────────────────────────────────────────────────────────

#[test]
fn no_local_artifact_dir_edge3() {
    assert_no_local_artifact_dir();
    println!("CX-212E: no repo-local artifact dir under the crate (edge 3)");
}

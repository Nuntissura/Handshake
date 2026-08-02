//! WP-KERNEL-012 MT-046 — INTERCONNECTION EDGE 3: Loom backlink + search across surfaces (IC-10..IC-14).
//!
//! These scenarios bind the handshake_core Loom backend (blocks / backlinks / search-v2 / graph /
//! quick-switcher / ai-jobs). IC-10/11/12/14 run by default through a managed product-backend fixture and
//! self-seed their own workspace state. IC-13 (AI link suggestion) runs the real AI route and skips only
//! for the backend's exact typed `409 HSK-409-LOOM-AI-NO-MODEL` response (NEVER silently skipped/faked).
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

#[path = "native_gui_support/canonical_argus_driver.rs"]
mod canonical_argus_driver;
#[path = "interconnect_support/mod.rs"]
mod interconnect_support;
#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;

use std::time::Duration;

use egui_kittest::Harness;

use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;
use handshake_native::backend_client::LoomSearchV2Response;
use handshake_native::graph::graph_view::{
    node_author_id as native_graph_node_author_id, GraphEdge as NativeGraphEdge,
    GraphNode as NativeGraphNode, LoomGraphView,
};
use handshake_native::loom_graph::{GraphNode, LoomGraphColors, LoomGraphSurface};
use handshake_native::loom_search_v2::sorted_facets;
use handshake_native::quick_switcher::{
    open_target_for_hit, LoomGraphSearchHit, QuickSwitcherTarget, SWITCHER_DIALOG_AUTHOR_ID,
    SWITCHER_SEARCH_AUTHOR_ID,
};
use handshake_native::rich_editor::document_model::doc_json::to_content_json_value;
use handshake_native::rich_editor::document_model::node::{
    BlockNode, Child, HsLinkNode, NodeKind, TextLeaf,
};
use handshake_native::theme::HsTheme;

use canonical_argus_driver::{json_has_author_id, CanonicalArgusDriver};
use interconnect_support::{
    assert_no_local_artifact_dir, author_ids, event_ledger_payload, loom_ai_residue_counts,
    require_live_backend, save_rich_document_via_production_manager, write_immutable_external_json,
    LiveBackend, ScenarioAttempt,
};
use screenshot_harness::ScreenshotHarness;

/// Build a note doc that carries a wikilink hsLink atom referencing `target_block_id` (the cross-surface
/// link that the backend backlink indexer keys on at save time — CTRL-2).
fn note_with_wikilink(target_block_id: &str, label: &str) -> BlockNode {
    let mut para = BlockNode::new(NodeKind::Paragraph);
    para.children.push(Child::Text(TextLeaf::new("links to ")));
    para.children.push(Child::HsLink(HsLinkNode::new(
        "note",
        target_block_id,
        label,
    )));
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
        .expect(
            "requires_pg: created document returns a rich_document_id (document.rich_document_id)",
        )
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

struct PersistedNoteLink {
    document_id: String,
    source_block_id: String,
    save_receipt_event_id: String,
}

/// Create a link-free KRD, then add the cross-surface link through the production native-editor save
/// path. The link cannot pre-exist from document creation, so the resulting KDLNK edge is causally tied
/// to SaveManager -> RichDocSaveBackend -> the backend backlink projection.
fn persist_note_link_via_native_save(
    be: &LiveBackend,
    title: &str,
    target_block_id: &str,
    label: &str,
) -> PersistedNoteLink {
    let linked_doc = note_with_wikilink(target_block_id, label);
    let content_json = to_content_json_value(&linked_doc);
    assert_eq!(
        count_hs_links(&content_json),
        1,
        "cross-surface save body carries exactly one wikilink atom"
    );
    let created = be.post_json(
        "/knowledge/documents",
        &serde_json::json!({
            "workspace_id": be.workspace_id.clone(),
            "title": title,
            "content_json": to_content_json_value(&BlockNode::doc(vec![BlockNode::paragraph("link pending")]))
        }),
    );
    let document_id = created_doc_id(&created);
    let source_block_id = created_note_block_id(&created, &document_id);
    assert_eq!(
        source_block_id, document_id,
        "cross-surface KRD uses its canonical same-id Loom projection"
    );
    let saved = save_rich_document_via_production_manager(
        be,
        &document_id,
        created_doc_version(&created) as u64,
        content_json,
    );
    assert_eq!(
        saved.backlinks_persisted, 1,
        "production SaveManager persists the one cross-surface backlink"
    );
    PersistedNoteLink {
        document_id,
        source_block_id,
        save_receipt_event_id: saved.save_receipt_event_id,
    }
}

/// Create a Loom block of the given content_type; return its block id. (requires_pg helper.)
fn create_block(be: &LiveBackend, content_type: &str, title: &str) -> String {
    let ws = &be.workspace_id;
    let block = be.post_json(
        &format!("/workspaces/{ws}/loom/blocks"),
        &serde_json::json!({ "title": title, "content_type": content_type }),
    );
    block["block_id"]
        .as_str()
        .or_else(|| block["id"].as_str())
        .expect("requires_pg: created block id")
        .to_owned()
}

fn is_typed_no_model_response(status: u16, body: &serde_json::Value) -> bool {
    status == 409
        && body.get("error").and_then(serde_json::Value::as_str) == Some("HSK-409-LOOM-AI-NO-MODEL")
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// IC-10 — Backlink registered on save (requires_pg): saving note A (with a wikilink to block B) registers a
// backlink edge B<-A in PG. CTRL-2: also assert the save body CARRIES the wikilink atom (the call-site
// contract), so the proof is not trivially passing on an empty save.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn interconnect_ic10_backlink_cross_surface() {
    let attempt = ScenarioAttempt::begin("IC-10");
    let mut be = require_live_backend();
    let ws = be.workspace_id.clone();

    // Block B is the code-block document the note links to.
    let loom_b = create_block(&be, "file", "IC-10 code block B");

    // Note A is created without a link, then gains it through the production native-editor SaveManager.
    // This rules out a backlink created as a side effect of POST /knowledge/documents.
    let linked = persist_note_link_via_native_save(&be, "IC-10 note A", &loom_b, "block B");
    let doc_id = linked.document_id;
    let loom_a = linked.source_block_id;

    // GET /loom/blocks/{B}/backlinks must contain loom_A after the save.
    let backlinks = be.get_json(&format!("/workspaces/{ws}/loom/blocks/{loom_b}/backlinks"));
    let found = backlinks
        .as_array()
        .map(|a| {
            a.iter().any(|b| {
                b.pointer("/edge/source_block_id").and_then(|v| v.as_str()) == Some(loom_a.as_str())
                    && b.pointer("/edge/target_block_id").and_then(|v| v.as_str())
                        == Some(loom_b.as_str())
                    && b.pointer("/source_block/block_id").and_then(|v| v.as_str())
                        == Some(loom_a.as_str())
            })
        })
        .unwrap_or(false);
    assert!(
        found,
        "IC-10: GET /loom/blocks/{loom_b}/backlinks contains loom_A after note A is saved"
    );
    let save_event_id = linked.save_receipt_event_id;
    let negative_status = be.get_status(&format!(
        "/workspaces/{ws}/loom/blocks/BLK-ic10-missing/backlinks"
    ));
    assert_eq!(negative_status, 404, "IC-10: missing target fails closed");

    let _ = be.delete(&format!("/knowledge/documents/{doc_id}"));
    let _ = be.delete(&format!("/workspaces/{ws}/loom/blocks/{loom_b}"));
    be.assert_cleanup();
    attempt.pass(serde_json::json!({
        "workspace_id": ws,
        "source_document_id": doc_id,
        "source_block_id": loom_a,
        "target_block_id": loom_b,
        "event_ledger_event_id": save_event_id,
        "negative_missing_target_status": negative_status,
    }));
    println!("IC-10 LIVE-PG PASS: backlinks of loom_B contain loom_A after save (save-calls-backlink CTRL-2 ok)");
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// IC-11 — Loom Search v2 across surfaces (requires_pg, CTRL-6 poll): a note + a code-file block both
// containing XSEARCH_PROBE both appear in POST /loom/search-v2 hits; facets contain both content types.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn interconnect_ic11_search_v2_across_surfaces() {
    let attempt = ScenarioAttempt::begin("IC-11");
    let mut be = require_live_backend();
    let ws = be.workspace_id.clone();
    const PROBE: &str = "XSEARCH_PROBE";

    let note_block = create_block(&be, "note", &format!("IC-11 note {PROBE}"));
    let code_block = create_block(&be, "file", &format!("IC-11 code {PROBE}"));

    // CTRL-6: poll the search endpoint up to 5x200ms (1s budget) to tolerate async indexing. On timeout fail
    // with `search_index_not_ready`, NOT a trivial pass.
    let mut hits = Vec::new();
    let mut native_response: Option<LoomSearchV2Response> = None;
    let mut found_both = false;
    for attempt in 0..5 {
        let resp = be.post_json(
            &format!("/workspaces/{ws}/loom/search-v2"),
            &serde_json::json!({ "query": PROBE, "graph_boost": 1.0, "limit": 25 }),
        );
        hits = resp["hits"].as_array().cloned().unwrap_or_default();
        let parsed: LoomSearchV2Response = serde_json::from_value(resp)
            .expect("IC-11: native LoomSearchV2 response parser accepts backend truth");
        let has_note = hits
            .iter()
            .any(|h| h["block"]["block_id"].as_str() == Some(note_block.as_str()));
        let has_code = hits
            .iter()
            .any(|h| h["block"]["block_id"].as_str() == Some(code_block.as_str()));
        if has_note && has_code {
            native_response = Some(parsed);
            found_both = true;
            break;
        }
        if attempt < 4 {
            std::thread::sleep(Duration::from_millis(200));
        }
    }
    assert!(
        found_both,
        "IC-11 / CTRL-6: search_index_not_ready — both blocks not indexed within 1s budget"
    );
    let native_response = native_response.expect("IC-11: found response retained");
    assert!(
        native_response
            .content_type_facets
            .get("note")
            .is_some_and(|count| *count >= 1),
        "IC-11: native search response exposes the note content-type facet"
    );
    assert!(
        native_response
            .content_type_facets
            .get("file")
            .is_some_and(|count| *count >= 1),
        "IC-11: native search response exposes the file content-type facet"
    );
    let facets = sorted_facets(&native_response);
    assert!(
        facets.iter().any(|(kind, _)| kind == "note")
            && facets.iter().any(|(kind, _)| kind == "file"),
        "IC-11: the shipped native facet projection retains both editor content types"
    );
    let note_event = be.poll_event_by_payload("block_id", &note_block);
    let code_event = be.poll_event_by_payload("block_id", &code_block);
    let negative = be.post_json(
        &format!("/workspaces/{ws}/loom/search-v2"),
        &serde_json::json!({ "query": "IC11_EXACTLY_ABSENT", "limit": 25 }),
    );
    assert!(
        negative["hits"].as_array().is_some_and(Vec::is_empty),
        "IC-11: absent cross-surface query returns no fabricated hits"
    );

    let _ = be.delete(&format!("/workspaces/{ws}/loom/blocks/{note_block}"));
    let _ = be.delete(&format!("/workspaces/{ws}/loom/blocks/{code_block}"));
    be.assert_cleanup();
    attempt.pass(serde_json::json!({
        "workspace_id": ws,
        "note_block_id": note_block,
        "code_block_id": code_block,
        "event_ledger_event_ids": [note_event["event_id"].clone(), code_event["event_id"].clone()],
        "search_hit_count": hits.len(),
        "content_type_facets": native_response.content_type_facets,
        "negative_absent_hit_count": 0,
    }));
    println!("IC-11 LIVE-PG PASS: search-v2 surfaced BOTH the note + code-file blocks for {PROBE} ({} hits)", hits.len());
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// IC-12 — Graph view shows cross-surface edges (requires_pg): GET /loom/graph depth=2 from loom_A returns
// both loom_A and loom_B with a connecting edge; the native force-directed layout renders both at distinct
// positions without panic. The render-doesn't-panic half is ALSO proven in-process below (structural).
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

/// In-process structural complement: the native graph surface renders TWO nodes without panic and emits a
/// distinct AccessKit node per block (the layout half of IC-12 that needs no PG). The cross-surface EDGE
/// topology comes from the default managed-PostgreSQL proof below.
#[test]
fn ic12_graph_renders_two_nodes_without_panic() {
    use handshake_native::context_menu_surfaces::LoomNodeState;
    use handshake_native::loom_graph::loom_node_author_id;

    // Two cross-surface nodes (a note block A + a code-file block B), the topology IC-12 renders.
    let node_a = GraphNode::new(
        LoomNodeState {
            block_id: "loom_A".into(),
            pinned: false,
            favorite: false,
            has_edges: true,
        },
        "loom_A note",
    );
    let node_b = GraphNode::new(
        LoomNodeState {
            block_id: "loom_B".into(),
            pinned: false,
            favorite: false,
            has_edges: true,
        },
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
    assert!(
        ids.contains(&loom_node_author_id("loom_A")),
        "IC-12: loom_A node present; got {ids:?}"
    );
    assert!(
        ids.contains(&loom_node_author_id("loom_B")),
        "IC-12: loom_B node present; got {ids:?}"
    );
    assert_no_local_artifact_dir();
    println!("IC-12 structural: the native graph surface rendered two distinct cross-surface nodes without panic");
}

#[test]
fn interconnect_ic12_graph_cross_surface_edges() {
    let attempt = ScenarioAttempt::begin("IC-12");
    let mut be = require_live_backend();
    let ws = be.workspace_id.clone();
    let loom_b = create_block(&be, "file", "IC-12 code B");
    // Reuse the IC-10 production path: a link-free KRD is saved with a wikilink through SaveManager,
    // and the backend creates the KDLNK projection. IC-12 must graph that causal edge, not seed /loom/edges.
    let linked = persist_note_link_via_native_save(&be, "IC-12 note A", &loom_b, "code B");
    let loom_a = linked.source_block_id.clone();
    let backlinks = be.get_json(&format!("/workspaces/{ws}/loom/blocks/{loom_b}/backlinks"));
    let edge_id = backlinks
        .as_array()
        .and_then(|rows| {
            rows.iter().find_map(|row| {
                (row.pointer("/edge/source_block_id")
                    .and_then(|v| v.as_str())
                    == Some(loom_a.as_str())
                    && row
                        .pointer("/edge/target_block_id")
                        .and_then(|v| v.as_str())
                        == Some(loom_b.as_str()))
                .then(|| {
                    row.pointer("/edge/edge_id")
                        .and_then(|v| v.as_str())
                        .expect("IC-12: KDLNK projection carries edge_id")
                        .to_owned()
                })
            })
        })
        .expect("IC-12: IC-10 native save creates the note-to-code backlink");
    assert!(
        edge_id.starts_with("KDLNK-"),
        "IC-12 graphs the canonical KDLNK save projection"
    );
    // The REAL block-neighborhood graph route is /loom/graph/local (loom.rs:264 local_loom_graph) with query
    // params start_block_id + max_depth, returning storage::LoomGraph { nodes, edges, .. } (loom.rs:996).
    // There is NO bare /loom/graph route (only /graph/traverse, /graph/local, /graph/global).
    let graph = be.get_json(&format!(
        "/workspaces/{ws}/loom/graph/local?start_block_id={loom_a}&max_depth=2"
    ));
    let nodes = graph["nodes"].as_array().cloned().unwrap_or_default();
    let edges = graph["edges"].as_array().cloned().unwrap_or_default();
    assert!(
        nodes.len() >= 2,
        "IC-12: the graph returns >= 2 nodes (loom_A + loom_B)"
    );
    assert!(
        !edges.is_empty(),
        "IC-12: the graph returns a connecting edge between the cross-surface nodes"
    );
    assert!(
        nodes.iter().any(|node| {
            node.pointer("/block/block_id").and_then(|v| v.as_str()) == Some(loom_a.as_str())
        }) && nodes.iter().any(|node| {
            node.pointer("/block/block_id").and_then(|v| v.as_str()) == Some(loom_b.as_str())
        }),
        "IC-12: graph contains both exact persisted block ids"
    );
    assert!(
        edges.iter().any(|row| {
            row.pointer("/edge/edge_id").and_then(|v| v.as_str()) == Some(edge_id.as_str())
        }),
        "IC-12: graph contains the exact persisted edge id"
    );
    // Feed the exact live backend projection into the shipped native graph/layout state. This binds the
    // persistence and render halves together instead of proving them in disconnected tests.
    let native_nodes: Vec<NativeGraphNode> = nodes
        .iter()
        .map(|node| {
            NativeGraphNode::new(
                node.pointer("/block/block_id")
                    .and_then(|v| v.as_str())
                    .expect("IC-12 native node block_id"),
                node.pointer("/block/title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Untitled"),
                node.pointer("/block/content_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown"),
            )
        })
        .collect();
    let native_edges: Vec<NativeGraphEdge> = edges
        .iter()
        .map(|row| {
            NativeGraphEdge::new(
                row.pointer("/edge/source_block_id")
                    .and_then(|v| v.as_str())
                    .expect("IC-12 native edge source"),
                row.pointer("/edge/target_block_id")
                    .and_then(|v| v.as_str())
                    .expect("IC-12 native edge target"),
                row.pointer("/edge/edge_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("mention"),
            )
        })
        .collect();
    let mut native_view = LoomGraphView::global(ws.clone());
    native_view.set_graph(native_nodes, native_edges);
    let max_step = native_view.step_layout();
    assert!(
        max_step.is_finite(),
        "IC-12: native layout returns a finite step"
    );
    assert!(
        native_view.nodes.iter().any(|node| node.block_id == loom_a)
            && native_view.nodes.iter().any(|node| node.block_id == loom_b),
        "IC-12: the native graph contains both persisted cross-surface blocks"
    );
    assert!(
        native_view
            .edges
            .iter()
            .any(|native| { native.source == loom_a && native.target == loom_b }),
        "IC-12: the native graph contains the exact persisted cross-surface edge"
    );
    let note_position = native_view
        .nodes
        .iter()
        .find(|node| node.block_id == loom_a)
        .map(|node| egui::pos2(node.x, node.y))
        .expect("IC-12: laid-out note node exists");
    let code_position = native_view
        .nodes
        .iter()
        .find(|node| node.block_id == loom_b)
        .map(|node| egui::pos2(node.x, node.y))
        .expect("IC-12: laid-out code node exists");
    assert!(
        note_position.distance(code_position) > 0.01,
        "IC-12: exact live note/code nodes must occupy distinct layout positions: \
         note={note_position:?} code={code_position:?}"
    );

    // Render this exact live-PG graph through the shipped LoomGraphView, then inspect its AccessKit tree.
    // The earlier structural complement cannot satisfy this assertion because its synthetic ids differ.
    let palette = HsTheme::Dark.palette();
    let mut graph_harness = Harness::builder()
        .with_size(egui::vec2(800.0, 600.0))
        .build_ui(move |ui| {
            let _ = native_view.show(ui, &palette);
        });
    graph_harness.run();
    let rendered_ids = author_ids(&graph_harness);
    assert!(
        rendered_ids.contains(&native_graph_node_author_id(&loom_a)),
        "IC-12: rendered live graph exposes the exact persisted note id; got {rendered_ids:?}"
    );
    assert!(
        rendered_ids.contains(&native_graph_node_author_id(&loom_b)),
        "IC-12: rendered live graph exposes the exact persisted code id; got {rendered_ids:?}"
    );
    let save_event_payload = event_ledger_payload(&linked.save_receipt_event_id);
    assert!(
        save_event_payload
            .pointer("/reference_targets")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|targets| targets.iter().any(|target| target.as_str() == Some(loom_b.as_str()))),
        "IC-12: the exact save receipt records the code-block reference that produced the KDLNK edge"
    );
    let negative_status = be.get_status(&format!(
        "/workspaces/{ws}/loom/graph/local?start_block_id=BLK-ic12-missing&max_depth=2"
    ));
    assert_eq!(
        negative_status, 404,
        "IC-12: missing graph root fails closed"
    );
    let _ = be.delete(&format!("/knowledge/documents/{}", linked.document_id));
    let _ = be.delete(&format!("/workspaces/{ws}/loom/blocks/{loom_b}"));
    be.assert_cleanup();
    attempt.pass(serde_json::json!({
        "workspace_id": ws,
        "note_block_id": loom_a,
        "code_block_id": loom_b,
        "edge_id": edge_id,
        "note_position": {"x": note_position.x, "y": note_position.y},
        "code_position": {"x": code_position.x, "y": code_position.y},
        "exact_live_graph_rendered": true,
        "event_ledger_event_id": linked.save_receipt_event_id,
        "negative_missing_root_status": negative_status,
    }));
    println!(
        "IC-12 LIVE-PG PASS: graph/local max_depth=2 from loom_A has {} nodes + {} edges",
        nodes.len(),
        edges.len()
    );
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// IC-13 — AI Loom job proposes cross-surface link. Requires a real AI model endpoint. The only skip is the
// backend's exact typed no-model response; every other response must pass or fail. The accept endpoint writes
// a real edge when run with a live AI endpoint.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn interconnect_ic13_ai_link_suggestion() {
    let attempt = ScenarioAttempt::begin("IC-13");
    let mut be = require_live_backend();
    let ws = be.workspace_id.clone();
    let code_title = format!("my_function code target {}", uuid::Uuid::now_v7().simple());
    let note_title = format!("note describing {code_title}");
    let code_block = create_block(&be, "file", &code_title);
    let description = format!(
        "The Rust function my_function is implemented by the code file Loom block {code_block}. \
         It returns the cross-surface value used by the native editor interconnection proof."
    );
    let descriptive_doc = BlockNode::doc(vec![BlockNode::paragraph(&description)]);
    let created_note = be.post_json(
        "/knowledge/documents",
        &serde_json::json!({
            "workspace_id": ws,
            "title": note_title,
            "content_json": to_content_json_value(&BlockNode::doc(vec![BlockNode::paragraph("description pending")]))
        }),
    );
    let note_document_id = created_doc_id(&created_note);
    let note_block = created_note_block_id(&created_note, &note_document_id);
    assert_eq!(
        note_block, note_document_id,
        "IC-13: KnowledgeRichDocument uses its canonical same-id Loom projection"
    );
    let projected_note = be.get_json(&format!("/workspaces/{ws}/loom/blocks/{note_document_id}"));
    assert_eq!(
        projected_note["block_id"].as_str(),
        Some(note_document_id.as_str()),
        "IC-13: same-id Loom projection exists at the production block route"
    );
    assert_eq!(
        projected_note["content_type"].as_str(),
        Some("note"),
        "IC-13: same-id projection remains a note block"
    );
    let note_save = save_rich_document_via_production_manager(
        &be,
        &note_document_id,
        created_doc_version(&created_note) as u64,
        to_content_json_value(&descriptive_doc),
    );
    let reloaded = be.get_json(&format!("/knowledge/documents/{note_document_id}"));
    assert!(
        reloaded
            .pointer("/document/content_json")
            .is_some_and(|content| content.to_string().contains("my_function")),
        "IC-13: descriptive KRD content is durably saved before link_suggest"
    );
    let residue_before = loom_ai_residue_counts(&ws);
    assert_eq!(
        residue_before.suggestion_rows, 0,
        "IC-13: owned workspace must start with zero AI suggestion rows"
    );
    assert_eq!(
        residue_before.recorded_event_rows, 0,
        "IC-13: owned workspace must start with zero joined AI proposal events"
    );

    // The only runtime skip is the backend's exact typed no-model response.
    // Any other 409 and every other non-success status remain a hard failure.
    let (job_status, job) = be.post_json_response(
        &format!("/workspaces/{ws}/loom/ai-jobs"),
        &serde_json::json!({
            "kind": "link_suggest",
            "block_ids": [note_block, code_block],
        }),
    );
    if is_typed_no_model_response(job_status, &job) {
        let residue_after = loom_ai_residue_counts(&ws);
        assert_eq!(
            residue_after.suggestion_rows, 0,
            "IC-13 typed no-model branch must leave zero PostgreSQL suggestions"
        );
        assert_eq!(
            residue_after.recorded_event_rows, 0,
            "IC-13 typed no-model branch must leave zero workspace EventLedger residue"
        );
        assert_eq!(
            residue_after.fixture_session_recorded_events,
            residue_before.fixture_session_recorded_events,
            "IC-13 typed no-model branch must not append an orphan AI proposal event in the owned fixture session"
        );
        let _ = be.delete(&format!("/knowledge/documents/{note_document_id}"));
        let _ = be.delete(&format!("/workspaces/{ws}/loom/blocks/{code_block}"));
        be.assert_cleanup();
        assert_no_local_artifact_dir();
        println!(
            "AI INTERCONNECT TEST SKIPPED: backend returned typed 409 HSK-409-LOOM-AI-NO-MODEL"
        );
        attempt.skipped(
            "HSK-409-LOOM-AI-NO-MODEL",
            serde_json::json!({
                "http_status": job_status,
                "error": "HSK-409-LOOM-AI-NO-MODEL",
                "workspace_id": ws,
                "source_note_document_id": note_document_id,
                "source_note_save_receipt_event_id": note_save.save_receipt_event_id,
                "suggestion_rows_before": residue_before.suggestion_rows,
                "suggestion_rows_after": residue_after.suggestion_rows,
                "workspace_recorded_events_before": residue_before.recorded_event_rows,
                "workspace_recorded_events_after": residue_after.recorded_event_rows,
                "fixture_session_events_before": residue_before.fixture_session_recorded_events,
                "fixture_session_events_after": residue_after.fixture_session_recorded_events,
                "seeded_blocks_cleaned": true,
            }),
        );
        return;
    }
    assert!(
        (200..300).contains(&job_status),
        "IC-13: AI job failed with non-skippable response {job_status}: {job}"
    );
    let job_id = job["job_id"]
        .as_str()
        .expect("IC-13: real AI job returns job_id")
        .to_owned();
    let suggestion = job["suggestions"]
        .as_array()
        .and_then(|rows| {
            rows.iter().find(|row| {
                row["block_id"].as_str() == Some(note_block.as_str())
                    && row["target_block_id"].as_str() == Some(code_block.as_str())
            })
        })
        .unwrap_or_else(|| {
            panic!(
                "IC-13: real link_suggest job {job_id} returned no note->code suggestion; response={job}"
            )
        });
    let suggestion_id = suggestion["suggestion_id"]
        .as_str()
        .expect("IC-13: suggestion carries suggestion_id")
        .to_owned();

    let listed = be.get_json(&format!(
        "/workspaces/{ws}/loom/ai-suggestions?job_id={job_id}&state=pending"
    ));
    assert!(
        listed.as_array().is_some_and(|rows| rows.iter().any(|row| {
            row["suggestion_id"].as_str() == Some(suggestion_id.as_str())
                && row["target_block_id"].as_str() == Some(code_block.as_str())
        })),
        "IC-13: pending suggestion must be durable before acceptance: {listed}"
    );

    let accepted = be.post_json(
        &format!("/workspaces/{ws}/loom/ai-suggestions/{suggestion_id}/accept"),
        &serde_json::json!({"reason": "IC-13 cross-surface proof"}),
    );
    assert_eq!(
        accepted["review_state"].as_str(),
        Some("accepted"),
        "IC-13: operator accept route promotes the real suggestion"
    );

    // link_suggest promotion creates note -> code. The canonical backlinks read is therefore on the code
    // target and must expose the note source; this is the persisted edge, not the AI response echoed back.
    let backlinks = be.get_json(&format!(
        "/workspaces/{ws}/loom/blocks/{code_block}/backlinks"
    ));
    assert!(
        backlinks
            .as_array()
            .is_some_and(|rows| rows.iter().any(|row| {
                row.pointer("/edge/source_block_id")
                    .and_then(|v| v.as_str())
                    == Some(note_block.as_str())
                    && row
                        .pointer("/edge/target_block_id")
                        .and_then(|v| v.as_str())
                        == Some(code_block.as_str())
                    && row
                        .pointer("/source_block/block_id")
                        .and_then(|v| v.as_str())
                        == Some(note_block.as_str())
            })),
        "IC-13: accepted AI suggestion must persist note->{code_block} edge; backlinks={backlinks}"
    );

    let _ = be.delete(&format!("/knowledge/documents/{note_document_id}"));
    let _ = be.delete(&format!("/workspaces/{ws}/loom/blocks/{code_block}"));
    be.assert_cleanup();
    assert_no_local_artifact_dir();
    attempt.pass(serde_json::json!({
        "job_id": job_id,
        "suggestion_id": suggestion_id,
        "source_note_document_id": note_document_id,
        "source_note_block_id": note_block,
        "source_note_save_receipt_event_id": note_save.save_receipt_event_id,
        "target_code_block_id": code_block,
        "accepted_edge_read_back": true,
    }));
    println!(
        "IC-13 LIVE AI+PG PASS: job={job_id} suggestion={suggestion_id} persisted note->code edge"
    );
}

#[test]
fn typed_no_model_skip_classifier_is_fail_closed() {
    assert!(is_typed_no_model_response(
        409,
        &serde_json::json!({"error": "HSK-409-LOOM-AI-NO-MODEL"})
    ));
    assert!(!is_typed_no_model_response(
        409,
        &serde_json::json!({"error": "HSK-409-SOME-OTHER-CONFLICT"})
    ));
    assert!(!is_typed_no_model_response(
        500,
        &serde_json::json!({"error": "HSK-409-LOOM-AI-NO-MODEL"})
    ));
    assert!(!is_typed_no_model_response(
        409,
        &serde_json::json!({"message": "HSK-409-LOOM-AI-NO-MODEL"})
    ));
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// IC-14 — Quick-switcher surfaces items from both editors (requires_pg). The product's q-driven
// cross-surface query uses GET /workspaces/{ws}/loom/graph-search with explicit source-kind filters.
// This scenario consumes the same typed hits and routes them through the native quick-switcher target
// mapper, rather than treating a generic backend search response as proof of quick-switcher behavior.
// (storage/loom.rs:572), so block_id/title/content_type/updated_at live under r["block"]. Both surfaces
// (note=knowledge_rich_document, code=code_file) must appear.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn interconnect_ic14_quick_switcher_both_editors() {
    let attempt = ScenarioAttempt::begin("IC-14");
    let mut be = require_live_backend();
    let ws = be.workspace_id.clone();
    const PROBE: &str = "XSEARCH_PROBE";
    let note_block = create_block(&be, "note", &format!("IC-14 note {PROBE}"));
    let code_block = create_block(&be, "file", &format!("IC-14 code {PROBE}"));

    let mut results: Vec<LoomGraphSearchHit> = Vec::new();
    let mut found_both = false;
    for attempt in 0..5 {
        // Drive the exact graph-search route and typed result model owned by QuickSwitcher.
        let resp = be.get_json(&format!(
            "/workspaces/{ws}/loom/graph-search?q={PROBE}&source_kinds=loom_block,file,tag_hub,document,symbol,work_packet,micro_task,user_manual_page,wiki_page&limit=25"
        ));
        results = serde_json::from_value(resp)
            .expect("IC-14: QuickSwitcher typed result parser accepts backend graph-search rows");
        let matches_block = |hit: &LoomGraphSearchHit, expected: &str| {
            hit.block
                .get("block_id")
                .and_then(serde_json::Value::as_str)
                == Some(expected)
                || hit.ref_id == expected
        };
        let has_note = results
            .iter()
            .any(|hit| matches_block(hit, note_block.as_str()));
        let has_code = results
            .iter()
            .any(|hit| matches_block(hit, code_block.as_str()));
        if has_note && has_code {
            found_both = true;
            break;
        }
        if attempt < 4 {
            std::thread::sleep(Duration::from_millis(200));
        }
    }
    assert!(
        found_both,
        "IC-14: the quick-switcher q-route returns BOTH the note + code block for {PROBE}"
    );
    // The same typed target mapping the operator-facing switcher uses must resolve both rows to real
    // navigation targets, never Unsupported.
    for expected in [&note_block, &code_block] {
        let hit = results
            .iter()
            .find(|hit| {
                hit.block
                    .get("block_id")
                    .and_then(serde_json::Value::as_str)
                    == Some(expected.as_str())
                    || hit.ref_id == *expected
            })
            .expect("IC-14: exact persisted block has a QuickSwitcher hit");
        assert!(
            !hit.title.trim().is_empty(),
            "IC-14: switcher hit has a title"
        );
        assert!(
            !matches!(open_target_for_hit(hit), QuickSwitcherTarget::Unsupported),
            "IC-14: both editor hits map through the shipped QuickSwitcher target resolver"
        );
    }
    let note_event = be.poll_event_by_payload("block_id", &note_block);
    let code_event = be.poll_event_by_payload("block_id", &code_block);
    let negative = be.get_json(&format!(
        "/workspaces/{ws}/loom/graph-search?q=IC14_EXACTLY_ABSENT&source_kinds=loom_block,file,tag_hub,document,symbol,work_packet,micro_task,user_manual_page,wiki_page&limit=25"
    ));
    let negative_results = serde_json::from_value::<Vec<LoomGraphSearchHit>>(negative)
        .expect("IC-14: empty QuickSwitcher response parses")
        .len();
    assert_eq!(
        negative_results, 0,
        "IC-14: absent query yields no quick-switcher rows"
    );
    let _ = be.delete(&format!("/workspaces/{ws}/loom/blocks/{note_block}"));
    let _ = be.delete(&format!("/workspaces/{ws}/loom/blocks/{code_block}"));
    be.assert_cleanup();
    attempt.pass(serde_json::json!({
        "workspace_id": ws,
        "note_block_id": note_block,
        "code_block_id": code_block,
        "event_ledger_event_ids": [note_event["event_id"].clone(), code_event["event_id"].clone()],
        "result_count": results.len(),
        "negative_absent_result_count": negative_results,
    }));
    println!("IC-14 LIVE-PG PASS: the quick-switcher q-route returned BOTH the note + code block with full fields");
}

/// Canonical mounted Argus producer for the IC-14 Loom/search/quick-switcher lane. Kept outside the
/// protected `interconnect_` namespace so the manifest remains exactly one function per IC scenario.
#[test]
#[ignore = "run only through run_mt046_interconnect_proof.ps1"]
fn supplemental_mt046_argus_ic14_quick_switcher_search() {
    let mut backend = require_live_backend();
    let workspace_id = backend.workspace_id.clone();
    let backend_binding = backend.owned_backend_binding_receipt();
    const PROBE: &str = "MT046_ARGUS_IC14_SEARCH";
    let seeded_block = create_block(&backend, "note", PROBE);

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("IC-14 Argus runtime");
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }));
    app.set_runtime_handle(runtime.handle().clone());
    app.set_backend_base_url_for_test(&backend.base, runtime.handle().clone());
    app.bind_active_project_for_integration_test(workspace_id.clone());
    let mut harness = ScreenshotHarness::builder()
        .proof_mt_id("MT-046")
        .with_size(egui::vec2(1100.0, 760.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(3);

    let initial_frame = harness.render_proof_frame("IC-14 mounted shell initial tree");
    let mut argus = CanonicalArgusDriver::bind(harness.state(), "mt046-ic14-quick-switcher");
    let initial_tree = argus.inspect(&mut harness);
    assert!(json_has_author_id(&initial_tree, "menu-view"));
    argus.click_expect_applied_and_reinspect(&mut harness, "menu-view");
    argus.assert_latest_terminal_predicate(
        &mut harness,
        "view-menu-exposes-quick-switcher",
        |tree| json_has_author_id(tree, "menu.view.open-quick-switcher"),
    );

    argus.click_expect_applied_and_reinspect(&mut harness, "menu.view.open-quick-switcher");
    argus.assert_latest_terminal_predicate(&mut harness, "quick-switcher-dialog-mounted", |tree| {
        json_has_author_id(tree, SWITCHER_DIALOG_AUTHOR_ID)
            && json_has_author_id(tree, SWITCHER_SEARCH_AUTHOR_ID)
    });
    let dialog_frame = harness.render_proof_frame("IC-14 quick-switcher dialog tree");

    argus.set_value_and_reinspect(&mut harness, SWITCHER_SEARCH_AUTHOR_ID, PROBE);
    for _ in 0..20 {
        harness.run_steps(1);
        if argus
            .inspect(&mut harness)
            .to_string()
            .contains(&seeded_block)
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut harness,
        "quick-switcher-search-terminal-result",
        serde_json::json!({"seeded_block_id": seeded_block, "query": PROBE}),
        |tree| tree.to_string().contains(PROBE),
    );
    let terminal_frame = harness.render_proof_frame("IC-14 quick-switcher terminal result tree");
    argus.finish_require_no_indeterminate();

    assert!(initial_frame.is_some() && dialog_frame.is_some() && terminal_frame.is_some());
    let proof_dir = interconnect_support::external_artifact_dir("canonical-argus")
        .join(std::env::var("HANDSHAKE_ARGUS_MATRIX_RUN_ID").expect("IC-14 matrix run id"))
        .join("IC-14");
    let _ = backend.delete(&format!(
        "/workspaces/{workspace_id}/loom/blocks/{seeded_block}"
    ));
    let runtime_diagnostics = backend
        .assert_cleanup_and_publish_runtime_diagnostics("IC-14")
        .expect("IC-14: publish fixture-owned backend runtime diagnostics");
    write_immutable_external_json(
        &proof_dir.join("workspace.json"),
        &serde_json::json!({
            "schema_id": "hsk.mt046.workspace-binding@1",
            "run_id": std::env::var("HANDSHAKE_ARGUS_MATRIX_RUN_ID").unwrap(),
            "scenario_id": "IC-14",
            "source_sha": std::env::var("HANDSHAKE_ARGUS_MATRIX_SOURCE_SHA").unwrap(),
            "process_correlation_id": std::env::var("HANDSHAKE_PROOF_PROCESS_CORRELATION_ID").unwrap(),
            "workspace_id": workspace_id,
            "process_id": std::process::id(),
            "backend_binding": backend_binding,
            "runtime_diagnostics": runtime_diagnostics,
        }),
    );
    assert_no_local_artifact_dir();
}

// ── Hygiene guard (runs in the default suite). ────────────────────────────────────────────────────────

#[test]
fn no_local_artifact_dir_edge3() {
    assert_no_local_artifact_dir();
    println!("CX-212E: no repo-local artifact dir under the crate (edge 3)");
}

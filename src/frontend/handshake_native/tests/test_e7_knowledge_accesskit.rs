//! WP-KERNEL-012 MT-042 (E7 model-vision parity): LIVE proofs for the consolidated
//! **KnowledgeActionRegistry** AccessKit surface — every interactive action on the native knowledge
//! graph ([`handshake_native::graph::graph_view`]), canvas board
//! ([`handshake_native::graph::canvas_board`]), and block-collection views
//! ([`handshake_native::graph::block_collection_view`]: table/Kanban/calendar) exposed through the
//! WP-011 AccessKit channel with stable surface-namespaced author_ids, correct roles, and a REAL
//! dispatch path (no screen-scraping, no keyboard simulation).
//!
//! ## Coverage map (AC / PROOF / CTRL)
//!
//! - AC-042-01 / PROOF-042-A: open graph + canvas + collection panes with synthetic data; query the
//!   AccessKit tree; assert per-block + per-placement + per-row node presence. (zero failures)
//! - AC-042-02: every LoomBlock => `graph.node.<block_id>` Role::TreeItem with action `activate` (Click).
//! - AC-042-03: every canvas placement => `canvas.card.<placement_id>` Role::Group with `activate` +
//!   `delete` (the contract's per-placement action set — `delete` is the discoverable
//!   `canvas.remove-placement` global control + the card's own remove path).
//! - AC-042-04: dispatch `graph.open-node {block_id}` via the AccessKit Action channel => the pane emits
//!   an OpenNode for that block (the cross-pane open), observable within a frame.
//! - AC-042-05: dispatch `canvas.place-block {block_id,x,y}` => a PlaceBlock event with the right route
//!   SHAPE (the request-shape half is standalone; live graph persistence is covered by AC-042-10).
//! - AC-042-06: dispatch `collection.kanban-move {block_id,from,to}` => a CardMove event with the right
//!   `add_tags`/`remove_tags` (the updateLoomBlock tag-edge request shape).
//! - AC-042-07: dispatch `graph.add-edge {source_id,target_id}` => an AddEdge INTENT event carrying ONLY
//!   source + target (the host supplies `created_by` + `edge_type` when it builds the real
//!   `CreateLoomEdgeRequest`). The createLoomEdge WIRE SHAPE itself is proven separately in
//!   [`ac07_add_edge_event_builds_real_create_loom_edge_request`] against the real
//!   `CanvasBoardClient::semantic_edge_request` / `backend::loom::CreateLoomEdgeRequest` builders, NOT at
//!   the typed event (which is missing the two backend-required fields).
//! - AC-042-08: all graph-level control nodes (`graph.pan-left`..`graph.zoom-reset`) present REGARDLESS
//!   of whether any blocks are loaded (global controls, not per-node).
//! - PROOF-042-B / HBR-VIS: print the full knowledge.* AccessKit tree to stdout; the reviewer can locate
//!   >=2 `graph.node.<uuid>` nodes, one `canvas.card.<uuid>` node, and all graph-level control nodes.
//! - PROOF-042-C: after dispatching `canvas.place-block`, print the tree again showing the new
//!   `canvas.card.<id>` node (the host applies the event + the new placement re-registers).
//! - CTRL-042-02 / RISK-042-02: placement_ids are 36-char UUID strings, stable across a refresh cycle.
//! - CTRL-042-03 / RISK-042-03: a malformed JSON payload dispatch causes NO panic (logged + dropped).
//!
//! ## Backend reality (Spec-Realism Gate / the MT-021/026/027 pattern)
//!
//! AC-042-10 is a non-ignored integration test. It creates an isolated workspace on the reachable
//! Handshake-managed SurrealDB backend, observes the real empty projection, seeds real blocks/edges,
//! fetches through the production graph client, drives stable author_ids, persists add/remove, performs
//! fresh-client reload, and proves the backend-loss negative. It never consumes an operator-seeded
//! workspace and never substitutes an in-memory projection for the live verdict.
//!
//! ## Artifact hygiene (CX-212E)
//!
//! This MT writes NO screenshots (the AccessKit tree dump to stdout is the HBR-VIS proof — IN-042
//! CHURN/VIEWPORT/QUIET gate: "AccessKit tree dump = HBR-VIS proof printed to stdout, no screenshot
//! needed"). [`assert_no_local_artifact_dir`] still fails the run if a repo-local `tests/screenshots/`
//! or `test_output/` dir exists (the reviewer also greps `git ls-files "src/**/*.png"`).

#[path = "interconnect_support/mod.rs"]
mod interconnect_support;
#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;

use std::collections::VecDeque;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use egui_kittest::kittest::NodeT;
use egui_kittest::Harness;

use handshake_native::accessibility::knowledge_action_registry::{
    canvas_card_author_id, collection_lane_author_id, collection_row_author_id,
    graph_edge_author_id, graph_node_author_id, KnowledgeActionRegistry, CANVAS_CONTROL_CATALOG,
    COLLECTION_CONTROL_CATALOG, GRAPH_CONTROL_CATALOG, HEALTH_CANARY_AUTHOR_ID,
};
use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend::knowledge_documents::{
    CreateDocumentRequest, HskDocumentHeaders, KnowledgeDocumentsClient,
};
use handshake_native::backend::loom::{CreateLoomEdgeRequest, LoomEdgeCreatedBy, LoomEdgeType};
use handshake_native::backend_client::HealthInfo;
use handshake_native::backend_client::{BlockViewClient, CanvasBoardClient, HttpMethod};
use handshake_native::backend_client::{LoomGraphCell, LoomGraphClient, LoomGraphData};
use handshake_native::command_registry::{CMD_VIEW_CANVAS, CMD_VIEW_GRAPH};
use handshake_native::graph::block_collection_view::{
    BlockCollectionView, BlockViewDefinition, BlockViewEvent, BlockViewGroupBy, BlockViewKind,
    BlockViewLane, BlockViewResults, LoomBlockRow,
};
use handshake_native::graph::canvas_board::PAN_STEP;
use handshake_native::graph::canvas_board::{CanvasEvent, CanvasPlacementCard, LoomCanvasBoard};
use handshake_native::graph::graph_view::{GraphEdge, GraphEvent, GraphNode, LoomGraphView};
use handshake_native::quick_switcher::{NavDispatchOutcome, ShellNavigator};
use handshake_native::theme::HsTheme;

// ── artifact-hygiene guard (CX-212E) ──────────────────────────────────────────────────────────────

/// Assert NO repo-local artifact dir exists under the crate (CX-212E): neither `test_output/` nor
/// `tests/screenshots/`. This MT writes no screenshots, but the guard is required by the artifact rule
/// and the reviewer's `git ls-files "src/**/*.png"` check — call it in the dump test.
fn assert_no_local_artifact_dir() {
    for local in ["test_output", "tests/screenshots"] {
        let p = Path::new(local);
        assert!(
            !p.exists(),
            "artifact hygiene: no repo-local {local} dir may exist — artifacts go to the external \
             Handshake_Artifacts/handshake-test root only (found {})",
            p.display()
        );
    }
}

// ── synthetic fixtures (IN-042-09: >=3 LoomBlocks, 2 edges, 2 canvas placements, 2 collection rows + a
//    Kanban lane). UUID v4 ids so CTRL-042-02 / RISK-042-02 holds (real UUID, never sequential ints). ──

/// Three synthetic blocks (note / canvas / view_def) + two edges, mirroring the IN-042-09 seed.
fn fixture_blocks() -> (Vec<GraphNode>, Vec<GraphEdge>) {
    let note = uuid::Uuid::new_v4().to_string();
    let canvas = uuid::Uuid::new_v4().to_string();
    let view_def = uuid::Uuid::new_v4().to_string();
    let nodes = vec![
        GraphNode::new(note.clone(), "Daily Note", "note"),
        GraphNode::new(canvas.clone(), "Project Canvas", "canvas"),
        GraphNode::new(view_def.clone(), "Tasks View", "view_def"),
    ];
    let edges = vec![
        GraphEdge::new(note.clone(), canvas.clone(), "mention"),
        GraphEdge::new(canvas, view_def, "mention"),
    ];
    (nodes, edges)
}

/// A graph view seeded with the fixture blocks + edges, with the registry installed.
fn graph_view(registry: &Arc<Mutex<KnowledgeActionRegistry>>) -> LoomGraphView {
    let (nodes, edges) = fixture_blocks();
    let mut v = LoomGraphView::global("ws-test");
    v.set_graph(nodes, edges);
    v.install_knowledge_action_registry(Arc::clone(registry));
    v
}

/// A canvas board seeded with 2 placements (real UUID placement_ids), with the registry installed.
fn canvas_board(registry: &Arc<Mutex<KnowledgeActionRegistry>>) -> LoomCanvasBoard {
    let mut b = LoomCanvasBoard::new("ws-test", "canvas-block-1");
    let placements: Vec<CanvasPlacementCard> = (0..2)
        .map(|i| {
            let pid = uuid::Uuid::new_v4().to_string(); // CTRL-042-02: real UUID, not a sequential int.
            let mut c = CanvasPlacementCard::new(
                pid,
                uuid::Uuid::new_v4().to_string(),
                (i as f32) * 240.0 + 30.0,
                40.0,
                200.0,
                120.0,
            );
            c.live_title = Some(format!("Placed Card {}", i + 1));
            c.live_content_type = Some("note".to_owned());
            c
        })
        .collect();
    b.set_board(placements, vec![], egui::Vec2::ZERO, 1.0);
    b.install_knowledge_action_registry(Arc::clone(registry));
    b
}

/// A Kanban collection seeded with 2 rows in two lanes, with the registry installed.
fn collection_view(registry: &Arc<Mutex<KnowledgeActionRegistry>>) -> BlockCollectionView {
    let mut c = BlockCollectionView::new("ws-test", "view-block-1");
    let row = |title: &str| LoomBlockRow {
        block_id: uuid::Uuid::new_v4().to_string(),
        title: Some(title.to_owned()),
        original_filename: None,
        content_type: "note".to_owned(),
        journal_date: None,
        created_at: "2026-06-23T00:00:00Z".to_owned(),
        updated_at: "2026-06-23T00:00:00Z".to_owned(),
        pinned: false,
        favorite: false,
        backlink_count: 0,
        mention_count: 0,
        tag_count: 1,
    };
    let r1 = row("Card A");
    let r2 = row("Card B");
    let results = BlockViewResults {
        kind_str: "kanban".to_owned(),
        blocks: vec![r1.clone(), r2.clone()],
        groups: vec![
            BlockViewLane {
                key: "todo".to_owned(),
                blocks: vec![r1],
            },
            BlockViewLane {
                key: "done".to_owned(),
                blocks: vec![r2],
            },
        ],
        total_returned: 2,
    };
    c.set_loaded(BlockViewDefinition::of_kind(BlockViewKind::Kanban), results);
    c.install_knowledge_action_registry(Arc::clone(registry));
    c
}

// ── A node found in the live kittest tree, reduced to the fields the proofs assert. ─────────────────

struct FoundNode {
    node_id: egui::accesskit::NodeId,
    role: String,
    label: Option<String>,
    value: Option<String>,
    supports_click: bool,
    supports_focus: bool,
    focused: bool,
    selected: bool,
    disabled: bool,
    flow_to: Vec<egui::accesskit::NodeId>,
    /// The node's custom-action capability descriptions (e.g. a canvas card's `delete` — AC-042-03).
    custom_actions: Vec<String>,
}

fn find_node(root: &egui_kittest::Node<'_>, author_id: &str) -> Option<FoundNode> {
    for node in root.children_recursive() {
        let ak = node.accesskit_node();
        if ak.author_id() == Some(author_id) {
            // The consumer wrapper proxies to the raw NodeData via `data()`; custom_actions live there.
            let custom_actions = ak
                .data()
                .custom_actions()
                .iter()
                .map(|c| c.description.to_string())
                .collect();
            return Some(FoundNode {
                node_id: ak.id(),
                role: format!("{:?}", ak.role()),
                label: ak.label().map(|v| v.to_owned()),
                value: ak.value().map(|v| v.to_owned()),
                supports_click: ak.data().supports_action(egui::accesskit::Action::Click),
                supports_focus: ak.data().supports_action(egui::accesskit::Action::Focus),
                focused: ak.is_focused(),
                selected: ak.is_selected().unwrap_or(false),
                disabled: ak.is_disabled(),
                flow_to: ak.data().flow_to().to_vec(),
                custom_actions,
            });
        }
    }
    None
}

/// All `knowledge.*` / surface-prefixed author_ids present in the live tree (graph./canvas./collection.).
fn knowledge_author_ids(root: &egui_kittest::Node<'_>) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for node in root.children_recursive() {
        let ak = node.accesskit_node();
        if let Some(author) = ak.author_id() {
            if author.starts_with("graph.")
                || author.starts_with("canvas.")
                || author.starts_with("collection.")
                || author == HEALTH_CANARY_AUTHOR_ID
            {
                out.push((author.to_owned(), format!("{:?}", ak.role())));
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Build a Click AccessKit action request targeting `node_id`, optionally carrying a JSON payload in
/// `ActionData::Value` (the IN-042-04 parameterized-action channel; the same shape `crate::mcp::action`
/// would build for a swarm dispatch).
fn click_event(node_id: egui::accesskit::NodeId, payload: Option<&str>) -> egui::Event {
    egui::Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
        action: egui::accesskit::Action::Click,
        target: node_id,
        data: payload.map(|p| egui::accesskit::ActionData::Value(p.to_owned().into_boxed_str())),
    })
}

fn focus_event(node_id: egui::accesskit::NodeId) -> egui::Event {
    egui::Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
        action: egui::accesskit::Action::Focus,
        target: node_id,
        data: None,
    })
}

/// Build a CustomAction AccessKit request targeting `node_id` with capability index `custom_id` (the
/// AC-042-03 card `delete` path — the swarm dispatches the node's i-th declared custom action).
fn custom_action_event(node_id: egui::accesskit::NodeId, custom_id: i32) -> egui::Event {
    egui::Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
        action: egui::accesskit::Action::CustomAction,
        target: node_id,
        data: Some(egui::accesskit::ActionData::CustomAction(custom_id)),
    })
}

/// A combined harness rendering all three knowledge panes into one CentralPanel, sharing ONE registry.
/// Each frame it calls ONLY `pane.show(ui, &palette)` — the SAME call a production host makes — and then
/// `pane.drain_knowledge_events()`. The sync/emit/take loop now lives INSIDE each `show` (the MT-042
/// must-fix anti-scaffolding wiring, the MT-041 pattern), so the registry is populated, the nodes are
/// emitted into the live tree, and a swarm dispatch is consumed PURELY from the render path — the harness
/// no longer injects that wiring (the prior tautology the adversarial review flagged). A dispatched Click
/// reaches the pane in the SAME frame (RISK-042-04). Returns the shared pane handles + the harness.
struct KnowledgeHarness<'a> {
    graph: Arc<Mutex<LoomGraphView>>,
    canvas: Arc<Mutex<LoomCanvasBoard>>,
    collection: Arc<Mutex<BlockCollectionView>>,
    graph_events: Arc<Mutex<Vec<GraphEvent>>>,
    canvas_events: Arc<Mutex<Vec<CanvasEvent>>>,
    collection_events: Arc<Mutex<Vec<BlockViewEvent>>>,
    harness: Harness<'a, ()>,
}

fn build_harness<'a>() -> KnowledgeHarness<'a> {
    let registry = Arc::new(Mutex::new(KnowledgeActionRegistry::new()));
    let graph = Arc::new(Mutex::new(graph_view(&registry)));
    let canvas = Arc::new(Mutex::new(canvas_board(&registry)));
    let collection = Arc::new(Mutex::new(collection_view(&registry)));
    let graph_events = Arc::new(Mutex::new(Vec::new()));
    let canvas_events = Arc::new(Mutex::new(Vec::new()));
    let collection_events = Arc::new(Mutex::new(Vec::new()));

    let g = Arc::clone(&graph);
    let cv = Arc::clone(&canvas);
    let col = Arc::clone(&collection);
    let ge = Arc::clone(&graph_events);
    let ce = Arc::clone(&canvas_events);
    let cce = Arc::clone(&collection_events);
    let palette = HsTheme::Dark.palette();

    let harness = Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .build_ui(move |ui| {
            ui.horizontal(|ui| {
                // GRAPH pane — ONLY show() + drain (the sync/emit/take is INSIDE show now).
                ui.vertical(|ui| {
                    let mut graph = g.lock().unwrap();
                    ui.allocate_ui(egui::vec2(380.0, 360.0), |ui| {
                        if let Some(ev) = graph.show(ui, &palette) {
                            ge.lock().unwrap().push(ev);
                        }
                    });
                    ge.lock().unwrap().extend(graph.drain_knowledge_events());
                });
                // CANVAS pane.
                ui.vertical(|ui| {
                    let mut canvas = cv.lock().unwrap();
                    ui.allocate_ui(egui::vec2(380.0, 360.0), |ui| {
                        if let Some(ev) = canvas.show(ui, &palette) {
                            ce.lock().unwrap().push(ev);
                        }
                    });
                    ce.lock().unwrap().extend(canvas.drain_knowledge_events());
                });
                // COLLECTION pane.
                ui.vertical(|ui| {
                    let mut collection = col.lock().unwrap();
                    ui.allocate_ui(egui::vec2(380.0, 360.0), |ui| {
                        if let Some(ev) = collection.show(ui, &palette) {
                            cce.lock().unwrap().push(ev);
                        }
                    });
                    cce.lock()
                        .unwrap()
                        .extend(collection.drain_knowledge_events());
                });
            });
        });

    KnowledgeHarness {
        graph,
        canvas,
        collection,
        graph_events,
        canvas_events,
        collection_events,
        harness,
    }
}

// ── AC-042-01 / AC-042-02 / AC-042-03 / AC-042-08: per-identity + global control nodes present ──────

#[test]
fn ac01_02_03_08_all_knowledge_nodes_present_with_roles() {
    let mut h = build_harness();
    h.harness.run();
    h.harness.run(); // settle so the per-identity nodes (viewport-derived) emit
    let root = h.harness.root();

    // Health canary -> non-empty-tree witness (no false-green).
    assert!(
        find_node(&root, HEALTH_CANARY_AUTHOR_ID).is_some(),
        "the health canary '{HEALTH_CANARY_AUTHOR_ID}' must be in the live tree"
    );

    // AC-042-08: every graph-level control node present (global controls, content-independent).
    for entry in GRAPH_CONTROL_CATALOG {
        let n = find_node(&root, entry.author_id).unwrap_or_else(|| {
            panic!(
                "AC-042-08: graph control '{}' must be present",
                entry.author_id
            )
        });
        assert_eq!(n.role, "Button", "{} is a Button control", entry.author_id);
    }
    for entry in CANVAS_CONTROL_CATALOG {
        assert!(
            find_node(&root, entry.author_id).is_some(),
            "canvas control '{}' present",
            entry.author_id
        );
    }
    for entry in COLLECTION_CONTROL_CATALOG {
        assert!(
            find_node(&root, entry.author_id).is_some(),
            "collection control '{}' present",
            entry.author_id
        );
    }

    // AC-042-02: every graph block => graph.node.<block_id> Role::TreeItem.
    let graph = h.graph.lock().unwrap();
    assert!(graph.nodes.len() >= 3, "fixture seeds >=3 blocks");
    for node in &graph.nodes {
        let author = graph_node_author_id(&node.block_id);
        let found = find_node(&root, &author)
            .unwrap_or_else(|| panic!("AC-042-02: '{author}' (TreeItem) must be present"));
        assert_eq!(
            found.role, "TreeItem",
            "AC-042-02: '{author}' role must be TreeItem"
        );
    }
    drop(graph);

    // AC-042-03: every canvas placement => canvas.card.<placement_id> Role::Group.
    let canvas = h.canvas.lock().unwrap();
    assert!(canvas.placements.len() >= 2, "fixture seeds 2 placements");
    for card in &canvas.placements {
        let author = canvas_card_author_id(&card.placement_id);
        let found = find_node(&root, &author)
            .unwrap_or_else(|| panic!("AC-042-03: '{author}' (Group) must be present"));
        assert_eq!(
            found.role, "Group",
            "AC-042-03: '{author}' role must be Group"
        );
        // The card carries its source block_id in the AccessKit value (IN-042-02).
        assert!(
            found
                .value
                .as_deref()
                .map(|v| v.contains("block_id="))
                .unwrap_or(false),
            "AC-042-03/IN-042-02: '{author}' value must carry block_id=; got {:?}",
            found.value
        );
        // AC-042-03: the card declares 'delete' (a real AccessKit custom action). 'activate' (Click) is
        // structurally guaranteed by the registry emit (every node adds Action::Click) and proven by the
        // dispatch tests; here we assert the delete capability is genuinely declared on the live node.
        assert!(
            found.custom_actions.iter().any(|a| a == "delete"),
            "AC-042-03: '{author}' must declare a 'delete' action; got {:?}",
            found.custom_actions
        );
    }
    drop(canvas);

    // collection: rows are Role::Row, lanes are Role::Group.
    let collection = h.collection.lock().unwrap();
    let results = collection.results.as_ref().unwrap();
    for row in &results.blocks {
        let author = collection_row_author_id(&row.block_id);
        let found = find_node(&root, &author).unwrap_or_else(|| panic!("'{author}' (Row) present"));
        assert_eq!(found.role, "Row", "'{author}' role must be Row");
    }
    for lane in &results.groups {
        let author = collection_lane_author_id(&lane.key);
        let found =
            find_node(&root, &author).unwrap_or_else(|| panic!("'{author}' (Group lane) present"));
        assert_eq!(found.role, "Group", "'{author}' lane role must be Group");
        assert!(
            !found.supports_click && found.supports_focus,
            "non-openable lane containers must advertise Focus without a no-op Click"
        );
    }
    drop(collection);

    println!("AC-042-01/02/03/08: graph nodes (TreeItem) + canvas cards (Group) + collection rows (Row) + lanes (Group) + all global controls present");
}

// ── AC-042-08 (isolation): graph controls present even with ZERO blocks loaded ──────────────────────

#[test]
fn ac08_graph_controls_present_with_no_blocks() {
    let registry = Arc::new(Mutex::new(KnowledgeActionRegistry::new()));
    let graph = {
        let mut v = LoomGraphView::global("ws-empty");
        v.set_graph(vec![], vec![]); // ZERO blocks
        v.install_knowledge_action_registry(Arc::clone(&registry));
        Arc::new(Mutex::new(v))
    };
    let g = Arc::clone(&graph);
    let palette = HsTheme::Dark.palette();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(500.0, 400.0))
        .build_ui(move |ui| {
            // ONLY show() — the knowledge sync/emit/take is wired INSIDE show (MT-042 must-fix).
            g.lock().unwrap().show(ui, &palette);
        });
    harness.run();
    harness.run();
    let root = harness.root();
    for entry in GRAPH_CONTROL_CATALOG {
        assert!(
            find_node(&root, entry.author_id).is_some(),
            "AC-042-08: graph control '{}' present even with 0 blocks (global control, not per-node)",
            entry.author_id
        );
    }
    // And NO graph.node.* identity nodes exist (deletion-by-absence with an empty set).
    let any_node = root.children_recursive().any(|n| {
        n.accesskit_node()
            .author_id()
            .map(|a| a.starts_with("graph.node."))
            .unwrap_or(false)
    });
    assert!(
        !any_node,
        "AC-042-08: no per-node identity nodes when 0 blocks loaded"
    );
    println!(
        "AC-042-08: all graph-level controls present with 0 blocks; 0 per-node identity nodes"
    );
}

// ── AC-042-04: dispatch graph.open-node {block_id} -> the pane emits OpenNode for that block ─────────

#[test]
fn ac04_dispatch_graph_open_node_emits_open() {
    let mut h = build_harness();
    h.harness.run();
    h.harness.run();

    // The target block is the first fixture block.
    let block_id = h.graph.lock().unwrap().nodes[0].block_id.clone();
    let open =
        find_node(&h.harness.root(), "graph.open-node").expect("graph.open-node control present");
    let payload = format!(r#"{{"block_id":"{block_id}"}}"#);
    h.harness.event(click_event(open.node_id, Some(&payload)));
    h.harness.run(); // the pane consumes the Click + parses the payload this frame
    h.harness.run();

    let events = h.graph_events.lock().unwrap();
    assert!(
        events.iter().any(|e| matches!(e, GraphEvent::OpenNode { block_id: b } if b == &block_id)),
        "AC-042-04: dispatching graph.open-node{{block_id}} emitted OpenNode for that block; got {events:?}"
    );
    // The selection moved to the opened node (observable in-pane state).
    assert_eq!(
        h.graph.lock().unwrap().selected.as_deref(),
        Some(block_id.as_str())
    );
    println!("AC-042-04: AccessKit dispatch of graph.open-node opened the block (cross-pane open + selection)");
}

#[test]
fn ac04_mounted_host_opens_exact_document_within_200ms() {
    let live = interconnect_support::require_reachable_backend();
    let nonce = uuid::Uuid::new_v4().to_string();
    let workspace = live.create_workspace(&format!("mt042-open-{nonce}"));
    let workspace_id = workspace["id"].as_str().expect("workspace id").to_owned();
    let mut cleanup = Mt042WorkspaceCleanup {
        backend: &live,
        workspace_id: workspace_id.clone(),
        active: true,
    };
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("MT-042 mounted host runtime");
    let docs = KnowledgeDocumentsClient::with_client(reqwest::Client::new(), live.base.clone());
    let headers = HskDocumentHeaders::for_operator(format!("mt042-session-{nonce}"), &nonce);
    let created = rt
        .block_on(docs.create_document(
            &headers,
            &CreateDocumentRequest {
                workspace_id: workspace_id.clone(),
                title: format!("MT-042 Open {nonce}"),
                create_if_title_absent: false,
                content_json: None,
                schema_version: None,
                project_ref: None,
                folder_ref: None,
            },
        ))
        .expect("fixture document create");
    let document_id = created.document["rich_document_id"]
        .as_str()
        .expect("rich document id")
        .to_owned();
    let wrong_id = uuid::Uuid::new_v4().to_string();

    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }));
    app.set_backend_base_url_for_test(&live.base, rt.handle().clone());
    app.bind_active_project_for_integration_test(workspace_id.clone());
    app.set_active_pane_for_test(Some("pane-b".into()));
    assert!(app.dispatch_palette_action_for_test(CMD_VIEW_GRAPH));
    app.mounted_graph_view().lock().unwrap().set_graph(
        vec![GraphNode::new(&document_id, "Exact document", "note")],
        vec![],
    );
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    let mount_deadline = Instant::now() + Duration::from_secs(5);
    let document_author = graph_node_author_id(&document_id);
    let open_node = loop {
        harness.run_steps(1);
        if let Some(node) = find_node(&harness.root(), &document_author) {
            break node.node_id;
        }
        assert!(
            Instant::now() < mount_deadline,
            "dynamic graph.node.<document_id> identity did not mount"
        );
    };
    assert!(
        find_node(&harness.root(), &format!("rich-editor.document.{wrong_id}")).is_none(),
        "wrong document must not be exposed before dispatch"
    );

    let started = Instant::now();
    harness.event(click_event(open_node, None));
    loop {
        harness.run_steps(1);
        if find_node(
            &harness.root(),
            &format!("rich-editor.document.{document_id}"),
        )
        .is_some()
        {
            break;
        }
        assert!(
            started.elapsed() <= Duration::from_millis(200),
            "dynamic graph.node.<document_id> did not expose the exact opened document within 200ms"
        );
    }
    assert!(started.elapsed() <= Duration::from_millis(200));
    assert!(
        find_node(&harness.root(), &format!("rich-editor.document.{wrong_id}")).is_none(),
        "wrong document id must never satisfy the open proof"
    );
    assert!(matches!(
        live.delete_workspace(&workspace_id),
        200 | 202 | 204
    ));
    cleanup.active = false;
}

// ── AC-042-04 (identity path): dispatch a per-node graph.node.<id> click -> OpenNode ────────────────

#[test]
fn ac04_dispatch_graph_node_identity_emits_open() {
    let mut h = build_harness();
    h.harness.run();
    h.harness.run();
    let block_id = h.graph.lock().unwrap().nodes[1].block_id.clone();
    let author = graph_node_author_id(&block_id);
    let node = find_node(&h.harness.root(), &author).expect("graph.node identity present");
    h.harness.event(click_event(node.node_id, None));
    h.harness.run();
    h.harness.run();
    let events = h.graph_events.lock().unwrap();
    assert!(
        events.iter().any(|e| matches!(e, GraphEvent::OpenNode { block_id: b } if b == &block_id)),
        "AC-042-04: clicking the per-node graph.node.<id> emitted OpenNode for that block; got {events:?}"
    );
    drop(events);
    h.graph.lock().unwrap().selected = None;
    h.graph_events.lock().unwrap().clear();
    h.harness.run();
    assert_eq!(
        h.graph.lock().unwrap().selected,
        None,
        "the Focus proof must start from an independently unselected graph node"
    );
    let focus_target = find_node(&h.harness.root(), &author).expect("graph node remains mounted");
    h.harness.event(focus_event(focus_target.node_id));
    h.harness.run();
    h.harness.run();
    let focused =
        find_node(&h.harness.root(), &author).expect("focused graph node remains mounted");
    assert!(
        focused.focused,
        "Focus must be re-observed on the exact stable author_id"
    );
    assert_eq!(
        h.graph.lock().unwrap().selected.as_deref(),
        Some(block_id.as_str()),
        "Focus synchronizes graph selection without opening a different block"
    );
    assert!(
        h.graph_events.lock().unwrap().is_empty(),
        "Focus must not activate or open the graph node"
    );

    h.canvas_events.lock().unwrap().clear();
    let placement_id = h.canvas.lock().unwrap().placements[0].placement_id.clone();
    let canvas_author = canvas_card_author_id(&placement_id);
    let canvas_target =
        find_node(&h.harness.root(), &canvas_author).expect("canvas card remains mounted");
    h.harness.event(focus_event(canvas_target.node_id));
    h.harness.run();
    h.harness.run();
    assert!(
        find_node(&h.harness.root(), &canvas_author).is_some_and(|node| node.focused),
        "Canvas Focus must be re-observed on the exact stable card identity"
    );
    assert!(
        h.canvas_events.lock().unwrap().is_empty(),
        "Canvas Focus must not activate, move, or delete the card"
    );

    h.collection_events.lock().unwrap().clear();
    let row_id = h
        .collection
        .lock()
        .unwrap()
        .results
        .as_ref()
        .unwrap()
        .blocks[0]
        .block_id
        .clone();
    let row_author = collection_row_author_id(&row_id);
    let row_target =
        find_node(&h.harness.root(), &row_author).expect("collection row remains mounted");
    h.harness.event(focus_event(row_target.node_id));
    h.harness.run();
    h.harness.run();
    assert!(
        find_node(&h.harness.root(), &row_author).is_some_and(|node| node.focused),
        "Collection Focus must be re-observed on the exact stable row identity"
    );
    assert!(
        h.collection_events.lock().unwrap().is_empty(),
        "Collection Focus must not activate or mutate the row"
    );
    println!("AC-042-04 (identity): clicking graph.node.<block_id> opened that block");
}

#[test]
fn graph_node_identity_collision_cannot_open_the_wrong_block() {
    let mut h = build_harness();
    h.graph.lock().unwrap().set_graph(
        vec![
            GraphNode::new("a/b", "Slash", "note"),
            GraphNode::new("a:b", "Colon", "note"),
        ],
        vec![],
    );
    h.harness.run();
    h.harness.run();

    let slash_author = graph_node_author_id("a/b");
    let colon_author = graph_node_author_id("a:b");
    assert_ne!(slash_author, colon_author);
    assert!(find_node(&h.harness.root(), &slash_author).is_some());
    let colon_node = find_node(&h.harness.root(), &colon_author)
        .expect("the collision-safe colon identity is independently addressable");
    h.harness.event(click_event(colon_node.node_id, None));
    h.harness.run();
    h.harness.run();

    let events = h.graph_events.lock().unwrap();
    assert!(
        events
            .iter()
            .any(|event| matches!(event, GraphEvent::OpenNode { block_id } if block_id == "a:b")),
        "the clicked collision-safe identity must open its exact raw block id; got {events:?}"
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, GraphEvent::OpenNode { block_id } if block_id == "a/b")),
        "clicking the colon identity must never fall through to the slash block"
    );
}

#[test]
fn persisted_edge_identity_survives_offscreen_endpoint_projection() {
    let registry = Arc::new(Mutex::new(KnowledgeActionRegistry::new()));
    let mut graph = LoomGraphView::global("ws-offscreen-edge");
    let nodes: Vec<GraphNode> = (0..60)
        .map(|index| GraphNode::new(format!("node-{index}"), format!("Node {index}"), "note"))
        .collect();
    graph.set_graph(
        nodes,
        vec![GraphEdge::with_id(
            "persisted-edge-offscreen",
            "node-58",
            "node-59",
            "mention",
        )],
    );
    for (index, node) in graph.nodes.iter_mut().enumerate() {
        node.x = 10_000.0 + index as f32 * 1_000.0;
        node.y = 10_000.0;
    }
    graph.install_knowledge_action_registry(Arc::clone(&registry));
    graph.sync_knowledge_registry(Some(egui::Rect::from_min_max(
        egui::pos2(0.0, 0.0),
        egui::pos2(100.0, 100.0),
    )));

    let registry = registry.lock().unwrap();
    assert!(
        registry
            .node(&graph_edge_author_id("persisted-edge-offscreen"))
            .is_some(),
        "every persisted response edge must remain addressable outside the node viewport projection"
    );
    assert!(
        registry.node(&graph_node_author_id("node-58")).is_none()
            && registry.node(&graph_node_author_id("node-59")).is_none(),
        "the probe must actually place both endpoints beyond the bounded 50-node lookahead"
    );
}

#[test]
fn collision_safe_canvas_and_collection_actions_route_to_exact_raw_identity() {
    let mut h = build_harness();
    let mut slash_card = CanvasPlacementCard::new("a/b", "block-slash", 20.0, 20.0, 120.0, 80.0);
    slash_card.live_title = Some("Slash card".to_owned());
    let mut colon_card = CanvasPlacementCard::new("a:b", "block-colon", 180.0, 20.0, 120.0, 80.0);
    colon_card.live_title = Some("Colon card".to_owned());
    h.canvas
        .lock()
        .unwrap()
        .set_board(vec![slash_card, colon_card], vec![], egui::Vec2::ZERO, 1.0);

    let row = |block_id: &str, title: &str| LoomBlockRow {
        block_id: block_id.to_owned(),
        title: Some(title.to_owned()),
        original_filename: None,
        content_type: "note".to_owned(),
        journal_date: None,
        created_at: "2026-07-18T00:00:00Z".to_owned(),
        updated_at: "2026-07-18T00:00:00Z".to_owned(),
        pinned: false,
        favorite: false,
        backlink_count: 0,
        mention_count: 0,
        tag_count: 0,
    };
    h.collection.lock().unwrap().set_loaded(
        BlockViewDefinition::of_kind(BlockViewKind::Table),
        BlockViewResults {
            kind_str: "table".to_owned(),
            blocks: vec![row("a/b", "Slash row"), row("a:b", "Colon row")],
            groups: vec![],
            total_returned: 2,
        },
    );

    h.harness.run();
    h.harness.run();
    let colon_card_author = canvas_card_author_id("a:b");
    let colon_card_node = find_node(&h.harness.root(), &colon_card_author)
        .expect("collision-safe colon card is independently addressable");
    h.harness.event(click_event(colon_card_node.node_id, None));
    h.harness.run();
    h.harness.run();
    assert!(h.canvas.lock().unwrap().selected.contains("a:b"));
    assert!(!h.canvas.lock().unwrap().selected.contains("a/b"));

    h.canvas_events.lock().unwrap().clear();
    let colon_card_node = find_node(&h.harness.root(), &colon_card_author)
        .expect("collision-safe colon card remains mounted");
    h.harness
        .event(custom_action_event(colon_card_node.node_id, 0));
    h.harness.run();
    h.harness.run();
    assert!(h.canvas_events.lock().unwrap().iter().any(
        |event| matches!(event, CanvasEvent::RemovePlacement { placement_id } if placement_id == "a:b")
    ));
    assert!(!h.canvas_events.lock().unwrap().iter().any(
        |event| matches!(event, CanvasEvent::RemovePlacement { placement_id } if placement_id == "a/b")
    ));

    h.collection_events.lock().unwrap().clear();
    let colon_row_author = collection_row_author_id("a:b");
    let colon_row_node = find_node(&h.harness.root(), &colon_row_author)
        .expect("collision-safe colon row is independently addressable");
    h.harness.event(click_event(colon_row_node.node_id, None));
    h.harness.run();
    h.harness.run();
    assert!(h
        .collection_events
        .lock()
        .unwrap()
        .iter()
        .any(|event| matches!(event, BlockViewEvent::OpenBlock { block_id } if block_id == "a:b")));
    assert!(!h
        .collection_events
        .lock()
        .unwrap()
        .iter()
        .any(|event| matches!(event, BlockViewEvent::OpenBlock { block_id } if block_id == "a/b")));
}

#[test]
fn graph_registry_rejects_stale_hidden_and_unregistered_offscreen_actions() {
    use handshake_native::graph::graph_view::MAX_LAYOUT_ITERS;

    let registry = Arc::new(Mutex::new(KnowledgeActionRegistry::new()));
    let mut initial = LoomGraphView::global("visibility-ws");
    initial.set_graph(
        vec![
            GraphNode::new("hidden-node", "Hidden", "note"),
            GraphNode::new("far-node", "Far", "note"),
        ],
        vec![GraphEdge::new("hidden-node", "far-node", "mention")],
    );
    initial.install_knowledge_action_registry(Arc::clone(&registry));
    let graph = Arc::new(Mutex::new(initial));
    let events = Arc::new(Mutex::new(Vec::<GraphEvent>::new()));
    let graph_ui = Arc::clone(&graph);
    let events_ui = Arc::clone(&events);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 600.0))
        .build_ui(move |ui| {
            let palette = HsTheme::Dark.palette();
            let mut graph = graph_ui.lock().unwrap();
            if let Some(event) = graph.show(ui, &palette) {
                events_ui.lock().unwrap().push(event);
            }
            events_ui
                .lock()
                .unwrap()
                .extend(graph.drain_knowledge_events());
        });
    harness.run();
    harness.run();

    let hidden_author = graph_node_author_id("hidden-node");
    let far_author = graph_node_author_id("far-node");
    let hidden_node = find_node(&harness.root(), &hidden_author)
        .expect("precondition: hidden target starts visible");
    let far_node =
        find_node(&harness.root(), &far_author).expect("precondition: far target starts visible");

    {
        let mut graph = graph.lock().unwrap();
        graph.controls.show_orphans = false;
        let mut nodes = vec![
            GraphNode::new("root-node", "Root", "note"),
            GraphNode::new("hidden-node", "Hidden", "note"),
            GraphNode::new("far-node", "Far", "note"),
        ];
        nodes.extend(
            (0..52).map(|index| GraphNode::new(format!("near-offscreen-{index}"), "Near", "note")),
        );
        let mut edges = vec![GraphEdge::new("root-node", "far-node", "mention")];
        edges.extend((0..52).map(|index| {
            GraphEdge::new("root-node", format!("near-offscreen-{index}"), "mention")
        }));
        graph.set_graph(nodes, edges);
        graph.step_layout();
        for node in &mut graph.nodes {
            match node.block_id.as_str() {
                "root-node" | "hidden-node" => {
                    node.x = 0.0;
                    node.y = 0.0;
                }
                "far-node" => {
                    node.x = 100_000.0;
                    node.y = 0.0;
                }
                _ => {
                    let index = node
                        .block_id
                        .trim_start_matches("near-offscreen-")
                        .parse::<f32>()
                        .expect("fixture suffix");
                    node.x = 1_000.0 + index;
                    node.y = 0.0;
                }
            }
        }
        graph.iters_done = MAX_LAYOUT_ITERS;
    }
    events.lock().unwrap().clear();
    // Let the product settle its normal post-layout auto-fit, then establish the exact viewport this
    // bounded-registry case exercises: root on-screen at world origin, distant nodes off-screen.
    // Without this explicit viewport, the 100k-wide synthetic coordinate range is correctly auto-fit
    // and the case no longer tests the advertised on-screen/off-screen boundary.
    harness.run();
    {
        let mut graph = graph.lock().unwrap();
        graph.pan = egui::Vec2::ZERO;
        graph.zoom = 1.0;
    }
    harness.run();

    assert!(
        find_node(&harness.root(), &hidden_author).is_none(),
        "a filtered hidden node must disappear from the registry-backed AccessKit tree"
    );
    assert!(
        find_node(&harness.root(), &far_author).is_none(),
        "an offscreen node beyond the bounded lookahead must not be advertised"
    );
    let root = find_node(&harness.root(), &graph_node_author_id("root-node"))
        .expect("the on-screen source remains addressable");
    assert!(
        !root.flow_to.contains(&far_node.node_id),
        "a visible source must not expose a dangling flow_to relation to an unregistered offscreen target"
    );
    let emitted_node_ids: std::collections::HashSet<_> = harness
        .root()
        .children_recursive()
        .map(|node| node.accesskit_node().id())
        .collect();
    assert!(
        root.flow_to
            .iter()
            .all(|target| emitted_node_ids.contains(target)),
        "every graph flow_to target must exist in the current AccessKit tree"
    );
    harness.event(click_event(hidden_node.node_id, None));
    harness.event(click_event(far_node.node_id, None));
    harness.run();
    harness.run();
    assert!(
        events.lock().unwrap().is_empty(),
        "stale actions for hidden/unregistered offscreen identities must not emit OpenNode"
    );
}

// ── AC-042-05: dispatch canvas.place-block {block_id,x,y} -> PlaceBlock event (route SHAPE) + new card ─

#[test]
fn ac05_dispatch_canvas_place_block_emits_place_and_new_card() {
    let mut h = build_harness();
    h.harness.run();
    h.harness.run();

    assert!(
        h.canvas.lock().unwrap().place_block_input.is_empty(),
        "precondition: the mouse-only place-block text field starts empty"
    );
    let new_block = uuid::Uuid::new_v4().to_string();
    let place = find_node(&h.harness.root(), "canvas.place-block")
        .expect("canvas.place-block control present");
    assert!(
        !place.disabled,
        "parameterized canvas.place-block remains steerable while the mouse-only text field is empty"
    );
    let payload = format!(r#"{{"block_id":"{new_block}","x":100,"y":100}}"#);
    h.harness.event(click_event(place.node_id, Some(&payload)));
    h.harness.run();
    h.harness.run();

    // The typed PlaceBlock event carries the right block id + position (the POST .../placements SHAPE).
    {
        let events = h.canvas_events.lock().unwrap();
        let placed = events.iter().any(|e| matches!(
            e,
            CanvasEvent::PlaceBlock { placed_block_id, x, y }
                if placed_block_id == &new_block && (*x - 100.0).abs() < 0.5 && (*y - 100.0).abs() < 0.5
        ));
        assert!(placed, "AC-042-05: canvas.place-block dispatch emitted PlaceBlock with x=100,y=100; got {events:?}");
    }

    // PROOF-042-C: the host APPLIES the event (adds the placement with a real UUID placement_id — what
    // the backend would mint) and the next sync re-registers a NEW canvas.card.<id> node. We simulate
    // the host-apply here (the DB round-trip is the gated #[ignore] test); a real placement_id UUID.
    let new_placement_id = uuid::Uuid::new_v4().to_string();
    {
        let mut canvas = h.canvas.lock().unwrap();
        let mut cards = canvas.placements.clone();
        let visual = canvas.visual_edges.clone();
        let (pan, zoom) = (canvas.pan, canvas.zoom);
        let mut c = CanvasPlacementCard::new(
            new_placement_id.clone(),
            new_block.clone(),
            100.0,
            100.0,
            200.0,
            120.0,
        );
        c.live_title = Some("Newly placed".to_owned());
        cards.push(c);
        canvas.set_board(cards, visual, pan, zoom);
    }
    h.harness.run();
    h.harness.run();

    let new_card_author = canvas_card_author_id(&new_placement_id);
    assert!(
        find_node(&h.harness.root(), &new_card_author).is_some(),
        "PROOF-042-C: a new 'canvas.card.<new_placement_id>' node appears after the place + refresh"
    );
    println!("AC-042-05 + PROOF-042-C: place-block dispatch emitted PlaceBlock (route shape) + the new canvas.card node appeared after refresh");
}

// ── AC-042-03 (dispatch): a `delete` custom action on a card -> RemovePlacement for that placement ──

#[test]
fn ac03_dispatch_card_delete_emits_remove_placement() {
    let mut h = build_harness();
    h.harness.run();
    h.harness.run();

    let placement_id = h.canvas.lock().unwrap().placements[0].placement_id.clone();
    let author = canvas_card_author_id(&placement_id);
    let card = find_node(&h.harness.root(), &author).expect("card node present");
    // The card declares exactly one custom action ('delete') at index 0.
    assert_eq!(
        card.custom_actions,
        vec!["delete".to_owned()],
        "card declares the delete custom action"
    );
    h.harness.event(custom_action_event(card.node_id, 0));
    h.harness.run();
    h.harness.run();

    let events = h.canvas_events.lock().unwrap();
    assert!(
        events.iter().any(|e| matches!(e, CanvasEvent::RemovePlacement { placement_id: p } if p == &placement_id)),
        "AC-042-03: the card's delete custom action emitted RemovePlacement for that placement; got {events:?}"
    );
    println!("AC-042-03 (dispatch): card delete custom action emitted RemovePlacement");
}

// ── AC-042-06: dispatch collection.kanban-move {block_id,from,to} -> CardMove with the tag-edge shape ─

#[test]
fn ac06_dispatch_kanban_move_emits_cardmove_tag_shape() {
    let mut h = build_harness();
    h.harness.run();
    h.harness.run();

    // Move the first row's block from "todo" to "done".
    let block_id = h
        .collection
        .lock()
        .unwrap()
        .results
        .as_ref()
        .unwrap()
        .groups[0]
        .blocks[0]
        .block_id
        .clone();
    let mv = find_node(&h.harness.root(), "collection.kanban-move")
        .expect("collection.kanban-move control present");
    let payload = format!(r#"{{"block_id":"{block_id}","from_lane":"todo","to_lane":"done"}}"#);
    h.harness.event(click_event(mv.node_id, Some(&payload)));
    h.harness.run();
    h.harness.run();

    let events = h.collection_events.lock().unwrap();
    let moved = events.iter().any(|e| matches!(
        e,
        BlockViewEvent::CardMove { block_id: b, add_tags, remove_tags }
            if b == &block_id && add_tags == &vec!["done".to_owned()] && remove_tags == &vec!["todo".to_owned()]
    ));
    assert!(
        moved,
        "AC-042-06: kanban-move dispatch emitted CardMove with add_tags=[done], remove_tags=[todo] (the \
         updateLoomBlock tag-edge SHAPE); got {events:?}"
    );
    println!("AC-042-06: collection.kanban-move dispatch emitted the CardMove tag-edge request shape (add=[done], remove=[todo])");
}

#[test]
fn ac06_dispatch_collection_sort_and_open_block_emit_events() {
    let mut h = build_harness();
    h.harness.run();
    h.harness.run();

    let sort = find_node(&h.harness.root(), "collection.sort").expect("collection.sort present");
    h.harness.event(click_event(
        sort.node_id,
        Some(r#"{"field":"title","direction":"desc"}"#),
    ));
    h.harness.run();
    h.harness.run();
    {
        let events = h.collection_events.lock().unwrap();
        assert!(
            events.iter().any(|e| matches!(
                e,
                BlockViewEvent::Sort { sort }
                    if sort.field.as_str() == "title" && sort.direction.as_str() == "desc"
            )),
            "collection.sort payload should emit a backend-driven Sort event; got {events:?}"
        );
    }

    let block_id = h
        .collection
        .lock()
        .unwrap()
        .results
        .as_ref()
        .unwrap()
        .blocks[0]
        .block_id
        .clone();
    let open = find_node(&h.harness.root(), "collection.open-block")
        .expect("collection.open-block present");
    let payload = format!(r#"{{"block_id":"{block_id}"}}"#);
    h.harness.event(click_event(open.node_id, Some(&payload)));
    h.harness.run();
    h.harness.run();
    {
        let events = h.collection_events.lock().unwrap();
        assert!(
            events.iter().any(|e| matches!(
                e,
                BlockViewEvent::OpenBlock { block_id: b } if b == &block_id
            )),
            "collection.open-block payload should emit OpenBlock for {block_id}; got {events:?}"
        );
    }

    let row_author = collection_row_author_id(&block_id);
    let row = find_node(&h.harness.root(), &row_author)
        .unwrap_or_else(|| panic!("collection row '{row_author}' present"));
    h.harness.event(click_event(row.node_id, None));
    h.harness.run();
    h.harness.run();
    let events = h.collection_events.lock().unwrap();
    assert!(
        events.iter().any(|e| matches!(
            e,
            BlockViewEvent::OpenBlock { block_id: b } if b == &block_id
        )),
        "collection.row.* click should emit OpenBlock for {block_id}; got {events:?}"
    );
    println!("AC-042-06: collection.sort, collection.open-block, and collection.row.* dispatch through the real AccessKit payload path");
}

// ── AC-042-07: dispatch graph.add-edge {source_id,target_id} -> AddEdge event (createLoomEdge shape) ──

#[test]
fn ac07_dispatch_graph_add_edge_emits_add_edge() {
    let mut h = build_harness();
    h.harness.run();
    h.harness.run();

    let (src, tgt) = {
        let g = h.graph.lock().unwrap();
        (g.nodes[0].block_id.clone(), g.nodes[2].block_id.clone())
    };
    let add =
        find_node(&h.harness.root(), "graph.add-edge").expect("graph.add-edge control present");
    let payload = format!(r#"{{"source_id":"{src}","target_id":"{tgt}"}}"#);
    h.harness.event(click_event(add.node_id, Some(&payload)));
    h.harness.run();
    h.harness.run();

    let events = h.graph_events.lock().unwrap();
    assert!(
        events.iter().any(|e| matches!(
            e,
            GraphEvent::AddEdge { source_block_id, target_block_id } if source_block_id == &src && target_block_id == &tgt
        )),
        "AC-042-07: graph.add-edge dispatch emitted AddEdge{{source,target}} (the add-edge INTENT event; \
         the host supplies created_by + edge_type when building the real CreateLoomEdgeRequest); got {events:?}"
    );
    println!("AC-042-07: graph.add-edge dispatch emitted the AddEdge intent event (source+target); the createLoomEdge WIRE shape is proven in ac07_add_edge_event_builds_real_create_loom_edge_request");
}

#[test]
fn graph_parameterized_actions_reject_blank_identity_fields() {
    let mut h = build_harness();
    h.harness.run();
    h.harness.run();

    let open =
        find_node(&h.harness.root(), "graph.open-node").expect("graph.open-node control present");
    h.harness
        .event(click_event(open.node_id, Some(r#"{"block_id":"   "}"#)));
    h.harness.run();

    let select = find_node(&h.harness.root(), "graph.select-node")
        .expect("graph.select-node control present");
    h.harness
        .event(click_event(select.node_id, Some(r#"{"block_id":"\t"}"#)));
    h.harness.run();

    for payload in [
        r#"{"source_id":"   ","target_id":"target"}"#,
        r#"{"source_id":"source","target_id":"\t"}"#,
    ] {
        let add =
            find_node(&h.harness.root(), "graph.add-edge").expect("graph.add-edge control present");
        h.harness.event(click_event(add.node_id, Some(payload)));
        h.harness.run();
    }

    let remove = find_node(&h.harness.root(), "graph.remove-edge")
        .expect("graph.remove-edge control present");
    h.harness
        .event(click_event(remove.node_id, Some(r#"{"edge_id":"   "}"#)));
    h.harness.run();

    let events = h.graph_events.lock().unwrap();
    assert!(
        !events.iter().any(|event| matches!(
            event,
            GraphEvent::OpenNode { .. }
                | GraphEvent::SelectNode { .. }
                | GraphEvent::AddEdge { .. }
                | GraphEvent::RemoveEdge { .. }
        )),
        "blank/whitespace identity fields must not produce graph actions; got {events:?}"
    );
    assert!(h.graph.lock().unwrap().selected.is_none());
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// LIVE-RENDER reachability (the must-fix anti-scaffolding guard; mirrors test_ckc_embed.rs's MT-033
// live-shell guard): a harness that calls ONLY `view.show(ui, &palette)` — NOT the manual
// sync/emit/take — must still populate the knowledge AccessKit surface AND consume a swarm dispatch.
// This is the regression guard for the "unwired scaffolding" finding: before the must-fix, the three
// swarm methods had ZERO call sites in any render loop, so the surface was dead from the render path and
// the old kittest passed only because the harness closure supplied the per-frame wiring the product
// lacked. By driving ONLY `show`, this test proves the wiring is in the PRODUCT (each `show` body), not
// in the test. If a future edit deletes the in-`show` sync/emit/take, this test goes RED.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn live_render_show_only_populates_surface_and_consumes_dispatch() {
    // GRAPH: drive ONLY graph.show. The registry must populate + a dispatched Click must reach the pane.
    let registry = Arc::new(Mutex::new(KnowledgeActionRegistry::new()));
    let graph = Arc::new(Mutex::new(graph_view(&registry)));
    let graph_events = Arc::new(Mutex::new(Vec::<GraphEvent>::new()));
    let palette = HsTheme::Dark.palette();
    let g = Arc::clone(&graph);
    let ge = Arc::clone(&graph_events);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(640.0, 480.0))
        .build_ui(move |ui| {
            // ONLY show() — the knowledge sync/emit/take is wired INSIDE show (MT-042 must-fix).
            let mut graph = g.lock().unwrap();
            if let Some(ev) = graph.show(ui, &palette) {
                ge.lock().unwrap().push(ev);
            }
            ge.lock().unwrap().extend(graph.drain_knowledge_events());
        });
    harness.run();
    harness.run(); // settle so the viewport-derived per-identity nodes emit
    let root = harness.root();

    // The surface is LIVE from the render path: the canary, the global controls, and the per-node
    // identities are all in the tree although the test never called sync/emit/take by hand.
    assert!(
        find_node(&root, HEALTH_CANARY_AUTHOR_ID).is_some(),
        "live-render: the knowledge canary must be in the tree driven by show() ALONE"
    );
    for entry in GRAPH_CONTROL_CATALOG {
        assert!(
            find_node(&root, entry.author_id).is_some(),
            "live-render: graph control '{}' must be present from show() alone (no manual sync/emit)",
            entry.author_id
        );
    }
    let block_id = graph.lock().unwrap().nodes[0].block_id.clone();
    let author = graph_node_author_id(&block_id);
    let node = find_node(&root, &author)
        .expect("live-render: graph.node.<id> identity must be present from show() alone");

    // A dispatched Click on the per-node identity REACHES the pane and produces OpenNode — purely because
    // `show` itself drained the dispatch (RISK-042-04 / the must-fix wiring). The test never calls take.
    harness.event(click_event(node.node_id, None));
    harness.run();
    harness.run();
    assert!(
        graph_events
            .lock()
            .unwrap()
            .iter()
            .any(|e| matches!(e, GraphEvent::OpenNode { block_id: b } if b == &block_id)),
        "live-render: a swarm Click reached the pane through show()'s own take loop (no harness wiring)"
    );
    assert_no_local_artifact_dir();
    println!("LIVE-RENDER: show() ALONE populated the knowledge surface (canary + controls + identities) and consumed a swarm dispatch — the surface is wired in the product, not the test");
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// Toolbar double-apply guard (the RISK the adversarial review flagged + required a test for): a single
// swarm Click on a TOOLBAR-OWNED canvas control (canvas.pan-left) must move pan by EXACTLY one PAN_STEP,
// never two. The latent 2x-pan bug would fire the moment the must-fix wiring landed IF both egui's
// synthetic `.clicked()` AND `take_knowledge_dispatched` applied the same plain Click. The guard in
// `take_knowledge_dispatched` (drop plain toolbar Clicks; egui's `.clicked()` owns them) is proven here.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn toolbar_plain_click_applies_pan_exactly_once() {
    let mut h = build_harness();
    h.harness.run();
    h.harness.run();

    let pan_before = h.canvas.lock().unwrap().pan.x;
    let pan_left =
        find_node(&h.harness.root(), "canvas.pan-left").expect("canvas.pan-left toolbar node");
    // A PLAIN (no-payload) swarm Click on the toolbar-owned pan-left node.
    h.harness.event(click_event(pan_left.node_id, None));
    h.harness.run();
    h.harness.run();
    let pan_after = h.canvas.lock().unwrap().pan.x;

    // Exactly ONE PAN_STEP to the left (not two — the double-apply guard holds).
    let delta = pan_after - pan_before;
    assert!(
        (delta - (-PAN_STEP)).abs() < 0.01,
        "toolbar double-apply guard: a single swarm Click on canvas.pan-left must move pan by exactly \
         one PAN_STEP (expected {}, got {delta}; a value of {} would be the 2x-apply bug)",
        -PAN_STEP,
        -2.0 * PAN_STEP
    );
    println!("TOOLBAR-DOUBLE-APPLY: one swarm Click on canvas.pan-left moved pan by exactly one PAN_STEP (no 2x-pan)");
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// BACKEND REQUEST-SHAPE proven STANDALONE (the contract's "the E6 request-SHAPE (right route/body built)
// are provable STANDALONE" gate; the must-fix request-shape gap). These take a DISPATCHED knowledge event
// (the same typed event a swarm dispatch produces) and feed it into the REAL production request builders
// in backend_client.rs / backend/loom.rs, asserting the exact verified route + body. No live SurrealDB is needed
// — the DB ROUND-TRIP stays the gated `#[ignore]` test; this proves the host wiring (MT-043/044) would
// build a WELL-FORMED request from the event, which the typed-event-only assertions above cannot.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

/// A tokio handle for the pure `*_request` builders (the sibling-test pattern — the builders never touch
/// the network; the handle is only required by the client constructor).
fn request_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Runtime::new().expect("tokio runtime for the request builders")
}

#[test]
fn ac05_place_block_event_builds_real_placements_request() {
    // Dispatch canvas.place-block, capture the typed PlaceBlock event (the swarm path), then build the
    // REAL CanvasBoardClient::place_block_request from it and assert the verified POST .../placements shape.
    let mut h = build_harness();
    h.harness.run();
    h.harness.run();
    let new_block = uuid::Uuid::new_v4().to_string();
    let place =
        find_node(&h.harness.root(), "canvas.place-block").expect("canvas.place-block control");
    let payload = format!(r#"{{"block_id":"{new_block}","x":100,"y":100}}"#);
    h.harness.event(click_event(place.node_id, Some(&payload)));
    h.harness.run();
    h.harness.run();

    let (placed_block_id, x, y) = {
        let events = h.canvas_events.lock().unwrap();
        events
            .iter()
            .find_map(|e| match e {
                CanvasEvent::PlaceBlock {
                    placed_block_id,
                    x,
                    y,
                } => Some((placed_block_id.clone(), *x as f64, *y as f64)),
                _ => None,
            })
            .expect("a PlaceBlock event was dispatched")
    };

    let rt = request_runtime();
    let client = CanvasBoardClient::new("http://127.0.0.1:37501", rt.handle().clone());
    // The default card geometry the host would supply (DEFAULT_CARD_W/H — the MT-026 verified body).
    let spec = client.place_block_request(
        "ws-test",
        "canvas-block-1",
        &placed_block_id,
        x,
        y,
        200.0,
        120.0,
    );
    assert!(
        matches!(spec.method, HttpMethod::Post),
        "placeBlockOnCanvas is a POST"
    );
    assert_eq!(
        spec.url,
        "http://127.0.0.1:37501/workspaces/ws-test/loom/canvas-boards/canvas-block-1/placements",
        "the REAL placements route (NOT the contract's stale /loom/canvas/{{cb}}/place)"
    );
    let body = spec.body.expect("placements POST carries a body");
    assert_eq!(
        body.get("placed_block_id").and_then(|v| v.as_str()),
        Some(new_block.as_str())
    );
    assert_eq!(body.get("x").and_then(|v| v.as_f64()), Some(100.0));
    assert_eq!(body.get("y").and_then(|v| v.as_f64()), Some(100.0));
    println!("AC-042-05 (request-shape): the dispatched PlaceBlock event builds the REAL POST .../placements request (route + body verified standalone)");
}

#[test]
fn ac06_card_move_event_builds_real_update_loom_block_request() {
    // Dispatch collection.kanban-move, capture the CardMove event, then build the REAL
    // BlockViewClient::card_move_request from it and assert the verified PATCH .../loom/blocks/:id shape
    // with top-level add_tags/remove_tags (the updateLoomBlock tag mutation).
    let mut h = build_harness();
    h.harness.run();
    h.harness.run();
    let block_id = h
        .collection
        .lock()
        .unwrap()
        .results
        .as_ref()
        .unwrap()
        .groups[0]
        .blocks[0]
        .block_id
        .clone();
    let mv = find_node(&h.harness.root(), "collection.kanban-move")
        .expect("collection.kanban-move control");
    let payload = format!(r#"{{"block_id":"{block_id}","from_lane":"todo","to_lane":"done"}}"#);
    h.harness.event(click_event(mv.node_id, Some(&payload)));
    h.harness.run();
    h.harness.run();

    let (mv_block, add_tags, remove_tags) = {
        let events = h.collection_events.lock().unwrap();
        events
            .iter()
            .find_map(|e| match e {
                BlockViewEvent::CardMove {
                    block_id,
                    add_tags,
                    remove_tags,
                } => Some((block_id.clone(), add_tags.clone(), remove_tags.clone())),
                _ => None,
            })
            .expect("a CardMove event was dispatched")
    };

    let rt = request_runtime();
    let client = BlockViewClient::new("http://127.0.0.1:37501", rt.handle().clone());
    let spec = client.card_move_request("ws-test", &mv_block, &add_tags, &remove_tags);
    assert!(
        matches!(spec.method, HttpMethod::Patch),
        "updateLoomBlock is a PATCH"
    );
    assert_eq!(
        spec.url,
        format!("http://127.0.0.1:37501/workspaces/ws-test/loom/blocks/{block_id}"),
        "the REAL updateLoomBlock route (PATCH /loom/blocks/:id)"
    );
    let body = spec.body.expect("card_move PATCH carries a body");
    // add_tags/remove_tags are TOP-LEVEL string arrays (the verified LoomBlockPatchRequest shape).
    assert_eq!(
        body.get("add_tags")
            .and_then(|v| v.as_array())
            .map(|a| a.len()),
        Some(1)
    );
    assert_eq!(body["add_tags"][0].as_str(), Some("done"));
    assert_eq!(body["remove_tags"][0].as_str(), Some("todo"));
    println!("AC-042-06 (request-shape): the dispatched CardMove event builds the REAL PATCH /loom/blocks/:id request (top-level add_tags/remove_tags verified standalone)");
}

#[test]
fn ac07_add_edge_event_builds_real_create_loom_edge_request() {
    // Dispatch graph.add-edge, capture the AddEdge INTENT event (source + target ONLY), then build the
    // REAL backend CreateLoomEdgeRequest the host would send — supplying the two backend-REQUIRED fields
    // the event omits (created_by + edge_type) — and assert it serializes to the verified createLoomEdge
    // wire body. This closes the "AddEdge cannot construct a valid request body" gap: the event is an
    // intent, and the host's createLoomEdge body is well-formed (matching loom.rs's request-shape pattern).
    let mut h = build_harness();
    h.harness.run();
    h.harness.run();
    let (src, tgt) = {
        let g = h.graph.lock().unwrap();
        (g.nodes[0].block_id.clone(), g.nodes[2].block_id.clone())
    };
    let add = find_node(&h.harness.root(), "graph.add-edge").expect("graph.add-edge control");
    let payload = format!(r#"{{"source_id":"{src}","target_id":"{tgt}"}}"#);
    h.harness.event(click_event(add.node_id, Some(&payload)));
    h.harness.run();
    h.harness.run();

    let (ev_src, ev_tgt) = {
        let events = h.graph_events.lock().unwrap();
        events
            .iter()
            .find_map(|e| match e {
                GraphEvent::AddEdge {
                    source_block_id,
                    target_block_id,
                } => Some((source_block_id.clone(), target_block_id.clone())),
                _ => None,
            })
            .expect("an AddEdge event was dispatched")
    };
    assert_eq!(
        (ev_src.as_str(), ev_tgt.as_str()),
        (src.as_str(), tgt.as_str())
    );

    // (a) The host builds the REAL backend request, supplying the two backend-required fields the AddEdge
    // intent event does NOT carry (created_by=user for a manual swarm edge, edge_type=mention).
    let req = CreateLoomEdgeRequest {
        edge_id: None,
        source_block_id: ev_src.clone(),
        target_block_id: ev_tgt.clone(),
        edge_type: LoomEdgeType::Mention,
        created_by: LoomEdgeCreatedBy::User,
        crdt_site_id: None,
        source_anchor: None,
        target_title: None,
    };
    let v = serde_json::to_value(&req).expect("CreateLoomEdgeRequest serializes");
    assert_eq!(v["source_block_id"].as_str(), Some(src.as_str()));
    assert_eq!(v["target_block_id"].as_str(), Some(tgt.as_str()));
    assert_eq!(
        v["edge_type"].as_str(),
        Some("mention"),
        "edge_type is a backend-required field"
    );
    assert_eq!(
        v["created_by"].as_str(),
        Some("user"),
        "created_by is a backend-required field"
    );
    assert!(
        v.get("edge_id").is_none(),
        "an absent edge_id is omitted (the backend mints it)"
    );

    // (b) And the SAME body is what the production CanvasBoardClient::semantic_edge_request builder emits,
    // proving the host wiring (route + the two required fields) is correct against the real builder.
    let rt = request_runtime();
    let client = CanvasBoardClient::new("http://127.0.0.1:37501", rt.handle().clone());
    let spec = client.semantic_edge_request("ws-test", &ev_src, &ev_tgt);
    assert!(
        matches!(spec.method, HttpMethod::Post),
        "createLoomEdge is a POST"
    );
    assert_eq!(
        spec.url,
        "http://127.0.0.1:37501/workspaces/ws-test/loom/edges"
    );
    let body = spec.body.expect("edges POST carries a body");
    assert_eq!(body["source_block_id"].as_str(), Some(src.as_str()));
    assert_eq!(body["target_block_id"].as_str(), Some(tgt.as_str()));
    assert_eq!(body["edge_type"].as_str(), Some("mention"));
    assert_eq!(body["created_by"].as_str(), Some("user"));
    println!("AC-042-07 (request-shape): the AddEdge intent event + host-supplied created_by/edge_type build the REAL createLoomEdge body (POST /loom/edges, verified standalone)");
}

// ── PROOF-042-B / HBR-VIS: dump the full knowledge.* AccessKit tree to stdout ───────────────────────

#[test]
fn proof_b_full_knowledge_tree_dump() {
    let mut h = build_harness();
    h.harness.run();
    h.harness.run();
    let root = h.harness.root();

    let dump = knowledge_author_ids(&root);
    println!(
        "--- PROOF-042-B: knowledge.* AccessKit node dump ({} nodes) ---",
        dump.len()
    );
    for (author, role) in &dump {
        println!("{author}  role={role}");
    }

    // The reviewer must locate >=2 graph.node.<uuid>, >=1 canvas.card.<uuid>, and all graph controls.
    let graph_nodes = dump
        .iter()
        .filter(|(a, _)| a.starts_with("graph.node."))
        .count();
    let canvas_cards = dump
        .iter()
        .filter(|(a, _)| a.starts_with("canvas.card."))
        .count();
    assert!(
        graph_nodes >= 2,
        "PROOF-042-B: at least two graph.node.<uuid> nodes in the dump; got {graph_nodes}"
    );
    assert!(
        canvas_cards >= 1,
        "PROOF-042-B: at least one canvas.card.<uuid> node in the dump; got {canvas_cards}"
    );
    for entry in GRAPH_CONTROL_CATALOG {
        assert!(
            dump.iter().any(|(a, _)| a == entry.author_id),
            "PROOF-042-B: graph control '{}' must be locatable in the dump",
            entry.author_id
        );
    }
    assert_no_local_artifact_dir();
    println!("PROOF-042-B: {graph_nodes} graph.node nodes, {canvas_cards} canvas.card nodes, all graph controls located");
}

// ── CTRL-042-02 / RISK-042-02: placement_ids are 36-char UUIDs, stable across a refresh cycle ───────

#[test]
fn ctrl02_placement_ids_are_stable_uuids() {
    let mut h = build_harness();
    h.harness.run();
    h.harness.run();

    let ids_before: Vec<String> = {
        let canvas = h.canvas.lock().unwrap();
        canvas
            .placements
            .iter()
            .map(|c| c.placement_id.clone())
            .collect()
    };
    for id in &ids_before {
        assert_eq!(
            id.len(),
            36,
            "CTRL-042-02: placement_id must be a 36-char UUID; got '{id}' ({} chars)",
            id.len()
        );
        assert!(
            uuid::Uuid::parse_str(id).is_ok(),
            "CTRL-042-02: placement_id must parse as a UUID; got '{id}'"
        );
        // The card node is addressable by the sanitized UUID.
        let author = canvas_card_author_id(id);
        assert!(
            find_node(&h.harness.root(), &author).is_some(),
            "card node for '{id}' present"
        );
    }

    // A refresh cycle (set_board with the SAME placements) keeps the ids + their AccessKit nodes stable.
    {
        let mut canvas = h.canvas.lock().unwrap();
        let same = canvas.placements.clone();
        let (pan, zoom) = (canvas.pan, canvas.zoom);
        canvas.set_board(same, vec![], pan, zoom);
    }
    h.harness.run();
    h.harness.run();
    let ids_after: Vec<String> = h
        .canvas
        .lock()
        .unwrap()
        .placements
        .iter()
        .map(|c| c.placement_id.clone())
        .collect();
    assert_eq!(
        ids_before, ids_after,
        "CTRL-042-02: placement_ids are stable across a refresh cycle"
    );
    for id in &ids_after {
        let author = canvas_card_author_id(id);
        assert!(
            find_node(&h.harness.root(), &author).is_some(),
            "card node for '{id}' still present after refresh"
        );
    }
    println!("CTRL-042-02: placement_ids are 36-char UUIDs, stable across a refresh cycle (no sequential-int reuse)");
}

// ── CTRL-042-03 / RISK-042-03: a malformed JSON payload dispatch causes NO panic ────────────────────

#[test]
fn ctrl03_malformed_payload_does_not_panic() {
    let mut h = build_harness();
    h.harness.run();
    h.harness.run();

    // Dispatch graph.open-node with GARBAGE JSON; the pane's serde match must log + drop, never panic.
    let open = find_node(&h.harness.root(), "graph.open-node").expect("graph.open-node present");
    h.harness
        .event(click_event(open.node_id, Some("this is not json {{{ ]")));
    h.harness.run();
    h.harness.run();
    // No OpenNode produced (the payload was dropped) and the app is still alive (no panic).
    assert!(
        !h.graph_events
            .lock()
            .unwrap()
            .iter()
            .any(|e| matches!(e, GraphEvent::OpenNode { .. })),
        "CTRL-042-03: a malformed payload must NOT produce an OpenNode (logged + dropped)"
    );

    // Dispatch canvas.place-block with a MISSING required field; same no-panic + no-event contract.
    let place =
        find_node(&h.harness.root(), "canvas.place-block").expect("canvas.place-block present");
    h.harness
        .event(click_event(place.node_id, Some(r#"{"block_id":"x"}"#))); // missing x/y
    h.harness.run();
    h.harness.run();
    assert!(
        !h.canvas_events
            .lock()
            .unwrap()
            .iter()
            .any(|e| matches!(e, CanvasEvent::PlaceBlock { .. })),
        "CTRL-042-03: a payload missing required fields must NOT produce a PlaceBlock"
    );

    // Dispatch a parameterized action with NO payload at all; no-panic + no-event.
    let mv = find_node(&h.harness.root(), "collection.kanban-move")
        .expect("collection.kanban-move present");
    h.harness.event(click_event(mv.node_id, None));
    h.harness.run();
    h.harness.run();
    assert!(
        !h.collection_events
            .lock()
            .unwrap()
            .iter()
            .any(|e| matches!(e, BlockViewEvent::CardMove { .. })),
        "CTRL-042-03: a parameterized dispatch with no payload must NOT produce a CardMove"
    );
    println!("CTRL-042-03: malformed / missing / absent payloads are logged + dropped — no panic on the UI thread");
}

// ── AC-042 live closure: self-seeded managed SurrealDB graph + AccessKit relations ────────────────

struct LiveGraphHarness<'a> {
    harness: Harness<'a, ()>,
    graph: Arc<Mutex<LoomGraphView>>,
    events: Arc<Mutex<Vec<GraphEvent>>>,
}

fn mount_live_graph<'a>(workspace_id: &str, data: LoomGraphData) -> LiveGraphHarness<'a> {
    let registry = Arc::new(Mutex::new(KnowledgeActionRegistry::new()));
    let mut view = LoomGraphView::global(workspace_id);
    view.set_graph_projection(
        data.nodes,
        data.edges,
        data.truncated,
        data.suppressed_hub_ids.len(),
    );
    view.install_knowledge_action_registry(registry);
    let graph = Arc::new(Mutex::new(view));
    let events = Arc::new(Mutex::new(Vec::new()));
    let graph_ui = Arc::clone(&graph);
    let events_ui = Arc::clone(&events);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 680.0))
        .build_ui(move |ui| {
            let palette = HsTheme::Dark.palette();
            let mut graph = graph_ui.lock().unwrap();
            if let Some(event) = graph.show(ui, &palette) {
                events_ui.lock().unwrap().push(event);
            }
            events_ui
                .lock()
                .unwrap()
                .extend(graph.drain_knowledge_events());
        });
    harness.run();
    harness.run();
    LiveGraphHarness {
        harness,
        graph,
        events,
    }
}

fn await_graph(
    client: &LoomGraphClient,
    workspace_id: &str,
    generation: u64,
) -> Result<LoomGraphData, String> {
    let cell: LoomGraphCell = Arc::new(Mutex::new(VecDeque::new()));
    client.fetch_global(workspace_id, generation, Arc::clone(&cell));
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(delivery) = cell.lock().unwrap().pop_front() {
            assert_eq!(delivery.request.generation, generation);
            assert_eq!(delivery.request.workspace_id, workspace_id);
            return delivery.result;
        }
        assert!(
            Instant::now() < deadline,
            "managed graph fetch generation {generation} did not complete within 10 seconds"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn await_canvas(
    client: &CanvasBoardClient,
    workspace_id: &str,
    canvas_block_id: &str,
) -> Result<handshake_native::backend_client::CanvasBoardData, String> {
    let cell = Arc::new(Mutex::new(VecDeque::new()));
    client.fetch_board(workspace_id, canvas_block_id, Arc::clone(&cell));
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(delivery) = cell.lock().unwrap().pop_front() {
            return delivery.result;
        }
        assert!(
            Instant::now() < deadline,
            "managed Canvas readback did not complete within five seconds"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn create_live_block_view(
    client: &BlockViewClient,
    workspace_id: &str,
    title: &str,
    definition: &BlockViewDefinition,
) -> String {
    let cell: handshake_native::backend_client::BlockViewOpCell = Arc::new(Mutex::new(None));
    let generation = Arc::new(std::sync::atomic::AtomicU64::new(1));
    let block_id = uuid::Uuid::new_v4().to_string();
    client.create_view(
        workspace_id,
        &block_id,
        title,
        definition,
        generation,
        1,
        Arc::clone(&cell),
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(delivery) = cell.lock().unwrap().clone() {
            assert_eq!(delivery.workspace_id, workspace_id);
            return delivery.result.expect("managed Kanban creation succeeds");
        }
        assert!(
            Instant::now() < deadline,
            "managed Kanban creation did not complete within five seconds"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn await_block_view_results(
    client: &BlockViewClient,
    workspace_id: &str,
    view_id: &str,
) -> Result<BlockViewResults, String> {
    let cell: handshake_native::backend_client::BlockViewResultsCell = Arc::new(Mutex::new(None));
    client.query_results(workspace_id, view_id, 100, 0, Arc::clone(&cell));
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(result) = cell.lock().unwrap().clone() {
            return result;
        }
        assert!(
            Instant::now() < deadline,
            "managed Kanban readback did not complete within five seconds"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn graph_block_id(value: &serde_json::Value) -> String {
    value["block_id"]
        .as_str()
        .expect("created LoomBlock carries block_id")
        .to_owned()
}

struct Mt042WorkspaceCleanup<'a> {
    backend: &'a interconnect_support::LiveBackend,
    workspace_id: String,
    active: bool,
}

impl Drop for Mt042WorkspaceCleanup<'_> {
    fn drop(&mut self) {
        if self.active {
            let _ = self.backend.delete_workspace(&self.workspace_id);
        }
    }
}

/// The complete graph proof is integration-gated but deliberately NOT ignored. It owns an isolated
/// workspace, starts from a real empty SurrealDB projection, seeds three real LoomBlocks and edges,
/// fetches through the production LoomGraphClient, mounts the real graph widget, drives it only by
/// stable author_id, persists add/remove mutations, and re-fetches through fresh production clients.
#[test]
fn ac10_live_surrealdb_populated_graph_accesskit_round_trip() {
    let live = interconnect_support::require_reachable_backend();
    let nonce = format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after unix epoch")
            .as_nanos()
    );
    let workspace = live.create_workspace(&format!("mt042-{nonce}"));
    let workspace_id = workspace["id"]
        .as_str()
        .expect("workspace response carries id")
        .to_owned();
    let mut cleanup = Mt042WorkspaceCleanup {
        backend: &live,
        workspace_id: workspace_id.clone(),
        active: true,
    };
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("build MT-042 live graph runtime");
    let graph_client = LoomGraphClient::new(live.base.clone(), rt.handle().clone());

    // Empty is a real backend state, not a fabricated Vec: global fetch returns zero canonical rows,
    // while the mounted AccessKit surface retains its stable global controls and health canary.
    let empty = await_graph(&graph_client, &workspace_id, 1).expect("empty managed graph fetch");
    assert!(empty.nodes.is_empty() && empty.edges.is_empty());
    let empty_mount = mount_live_graph(&workspace_id, empty);
    assert!(knowledge_author_ids(&empty_mount.harness.root())
        .iter()
        .all(|(author, _)| !author.starts_with("graph.node.")));
    for control in GRAPH_CONTROL_CATALOG {
        assert!(find_node(&empty_mount.harness.root(), control.author_id).is_some());
    }
    drop(empty_mount);

    let create_block = |title: &str| {
        live.post_json(
            &format!("/workspaces/{workspace_id}/loom/blocks"),
            &serde_json::json!({"content_type":"note","title":format!("{title}-{nonce}")}),
        )
    };
    let alpha = create_block("Alpha");
    let beta = create_block("Beta");
    let gamma = create_block("Gamma");
    let alpha_id = graph_block_id(&alpha);
    let beta_id = graph_block_id(&beta);
    let gamma_id = graph_block_id(&gamma);
    let seed_edge = |source: &str, target: &str| {
        live.post_json(
            &format!("/workspaces/{workspace_id}/loom/edges"),
            &serde_json::json!({
                "source_block_id":source,
                "target_block_id":target,
                "edge_type":"mention",
                "created_by":"user"
            }),
        )
    };
    seed_edge(&alpha_id, &beta_id);
    seed_edge(&beta_id, &gamma_id);

    let populated =
        await_graph(&graph_client, &workspace_id, 2).expect("populated managed graph fetch");
    assert_eq!(
        populated.nodes.len(),
        3,
        "three seeded SurrealDB blocks are visible"
    );
    assert_eq!(
        populated.edges.len(),
        2,
        "two seeded SurrealDB edges are visible"
    );
    let expected_nodes: Vec<(String, String)> = populated
        .nodes
        .iter()
        .map(|node| (node.block_id.clone(), node.title.clone()))
        .collect();
    let expected_edges = populated.edges.clone();
    let mut mounted = mount_live_graph(&workspace_id, populated);

    let mut stable_node_ids = std::collections::BTreeMap::new();
    for (block_id, title) in &expected_nodes {
        let author = graph_node_author_id(block_id);
        let node = find_node(&mounted.harness.root(), &author)
            .unwrap_or_else(|| panic!("live SurrealDB node {author} is mounted"));
        assert_eq!(node.role, "TreeItem");
        assert_eq!(node.label.as_deref(), Some(title.as_str()));
        assert!(node.supports_click && node.supports_focus);
        stable_node_ids.insert(author, node.node_id);
    }
    let mut stable_edge_ids = std::collections::BTreeMap::new();
    for edge in &expected_edges {
        let source = find_node(&mounted.harness.root(), &graph_node_author_id(&edge.source))
            .expect("edge source is addressable");
        let target = find_node(&mounted.harness.root(), &graph_node_author_id(&edge.target))
            .expect("edge target is addressable");
        assert!(
            source.flow_to.contains(&target.node_id),
            "canonical {} edge {} -> {} is exposed as AccessKit flow_to",
            edge.edge_type,
            edge.source,
            edge.target
        );
        let edge_id = edge
            .edge_id
            .as_deref()
            .expect("production graph projection preserves persisted edge_id");
        let edge_author = graph_edge_author_id(edge_id);
        let edge_node = find_node(&mounted.harness.root(), &edge_author)
            .expect("persisted graph edge is independently addressable");
        assert_eq!(edge_node.role, "Link");
        assert!(edge_node.label.as_deref().is_some_and(|label| {
            label.contains(&edge.edge_type)
                && label.contains(&edge.source)
                && label.contains(&edge.target)
        }));
        assert!(edge_node.value.as_deref().is_some_and(|value| {
            value.contains(&format!("edge_id={edge_id}"))
                && value.contains(&format!("source_id={}", edge.source))
                && value.contains(&format!("target_id={}", edge.target))
        }));
        assert!(edge_node
            .custom_actions
            .iter()
            .any(|action| action == "delete"));
        stable_edge_ids.insert(edge_author, edge_node.node_id);
    }

    // Navigate solely by the target node's stable author_id. The product event and the next tree
    // observation must agree on the exact raw block id and selected state.
    let target_author = graph_node_author_id(&gamma_id);
    let target = find_node(&mounted.harness.root(), &target_author).expect("target node mounted");
    mounted.harness.event(click_event(target.node_id, None));
    mounted.harness.run();
    mounted.harness.run();
    assert!(mounted
        .events
        .lock()
        .unwrap()
        .iter()
        .any(|event| matches!(event, GraphEvent::OpenNode { block_id } if block_id == &gamma_id)));
    assert_eq!(
        mounted.graph.lock().unwrap().selected.as_deref(),
        Some(gamma_id.as_str())
    );
    assert!(
        find_node(&mounted.harness.root(), &target_author)
            .expect("selected target remains mounted")
            .selected,
        "the post-activation tree re-observes the exact target as selected"
    );

    // Move to the production HandshakeApp host before mutating. The mounted pane emits GraphEvent,
    // HandshakeApp::route_graph_events owns the HTTP mutation, and its completion drain performs the
    // authoritative refetch. The proof never applies the event or calls a write route itself.
    drop(mounted);
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }));
    app.set_backend_base_url_for_test(&live.base, rt.handle().clone());
    app.bind_active_project_for_integration_test(workspace_id.clone());
    assert!(app.dispatch_palette_action_for_test(CMD_VIEW_GRAPH));
    let mounted_graph = app.mounted_graph_view();
    let mut host = Harness::builder()
        .with_size(egui::vec2(1100.0, 760.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    let host_deadline = Instant::now() + Duration::from_secs(5);
    loop {
        host.run_steps(1);
        if find_node(&host.root(), &graph_node_author_id(&gamma_id)).is_some() {
            break;
        }
        assert!(
            Instant::now() < host_deadline,
            "production graph host did not load the managed SurrealDB projection within five seconds"
        );
        std::thread::sleep(Duration::from_millis(10));
    }

    // Dispatch add-edge by stable control author_id on the production host. Persistence must happen
    // only through the app router; a fresh product client observes the result independently.
    let add = find_node(&host.root(), "graph.add-edge").expect("host add-edge control");
    let add_payload =
        serde_json::json!({"source_id":alpha_id.clone(),"target_id":gamma_id.clone()}).to_string();
    host.event(click_event(add.node_id, Some(&add_payload)));
    host.run_steps(1);
    let fresh_client = LoomGraphClient::new(live.base.clone(), rt.handle().clone());
    let add_deadline = Instant::now() + Duration::from_secs(5);
    let after_add = loop {
        host.run_steps(1);
        if let Ok(graph) = await_graph(&fresh_client, &workspace_id, 3) {
            if graph
                .edges
                .iter()
                .any(|edge| edge.source == alpha_id && edge.target == gamma_id)
            {
                break graph;
            }
        }
        assert!(
            Instant::now() < add_deadline,
            "host-routed graph.add-edge did not persist and refetch within five seconds"
        );
        std::thread::sleep(Duration::from_millis(10));
    };
    let created_edge = after_add
        .edges
        .iter()
        .find(|edge| {
            edge.source == alpha_id && edge.target == gamma_id && edge.edge_type == "mention"
        })
        .expect("fresh production graph projection contains the host-created edge");
    let created_edge_id = created_edge
        .edge_id
        .clone()
        .expect("production projection preserves the host-created persisted edge id");
    let created_edge_author = graph_edge_author_id(&created_edge_id);
    loop {
        host.run_steps(1);
        let source = find_node(&host.root(), &graph_node_author_id(&alpha_id)).unwrap();
        let target = find_node(&host.root(), &graph_node_author_id(&gamma_id)).unwrap();
        if source.flow_to.contains(&target.node_id)
            && find_node(&host.root(), &created_edge_author).is_some()
        {
            break;
        }
        assert!(
            Instant::now() < add_deadline,
            "mounted graph did not publish the persisted edge identity and AccessKit flow_to"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let edge_node = find_node(&host.root(), &created_edge_author)
        .expect("host-created edge is addressable by persisted edge id");
    assert_eq!(edge_node.role, "Link");
    assert!(
        !edge_node.supports_click,
        "persisted edge identities must not advertise a no-op Click action"
    );
    assert!(
        edge_node.supports_focus,
        "persisted edge identities remain focus-addressable"
    );
    assert!(edge_node
        .custom_actions
        .iter()
        .any(|action| action == "delete"));

    // Activate the persisted edge identity's real delete capability. Database readback and a fresh
    // reload must both lose the relation; stale edge nodes and relations are forbidden.
    host.event(custom_action_event(edge_node.node_id, 0));
    host.run_steps(1);
    let remove_deadline = Instant::now() + Duration::from_secs(5);
    let after_remove = loop {
        host.run_steps(1);
        if let Ok(graph) = await_graph(&fresh_client, &workspace_id, 4) {
            if !graph
                .edges
                .iter()
                .any(|edge| edge.source == alpha_id && edge.target == gamma_id)
            {
                break graph;
            }
        }
        assert!(
            Instant::now() < remove_deadline,
            "host-routed graph.remove-edge did not persist and refetch within five seconds"
        );
        std::thread::sleep(Duration::from_millis(10));
    };
    assert!(!after_remove
        .edges
        .iter()
        .any(|edge| edge.source == alpha_id && edge.target == gamma_id));
    loop {
        host.run_steps(1);
        let source = find_node(&host.root(), &graph_node_author_id(&alpha_id)).unwrap();
        let target = find_node(&host.root(), &graph_node_author_id(&gamma_id)).unwrap();
        if !source.flow_to.contains(&target.node_id)
            && find_node(&host.root(), &created_edge_author).is_none()
        {
            break;
        }
        assert!(
            Instant::now() < remove_deadline,
            "mounted graph retained a stale edge identity or flow_to after persisted edge removal"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(
        mounted_graph.lock().unwrap().workspace_id,
        workspace_id,
        "the production host remains bound to the managed workspace"
    );

    // Keep the same mounted HandshakeApp and switch its operator-facing pane to Canvas. The stable
    // parameterized AccessKit action must cross the product router, persist a backend-minted placement,
    // and reconcile the mounted board from the authoritative response.
    let canvas = live.post_json(
        &format!("/workspaces/{workspace_id}/loom/canvas-boards"),
        &serde_json::json!({"title":format!("MT-042 Canvas {nonce}")}),
    );
    let canvas_id = graph_block_id(&canvas);
    let mounted_canvas = host.state().mounted_canvas_board();
    {
        let mut board = mounted_canvas.lock().unwrap();
        board.workspace_id = workspace_id.clone();
        board.canvas_block_id = canvas_id.clone();
    }
    assert!(host
        .state_mut()
        .dispatch_palette_action_for_test(CMD_VIEW_CANVAS));
    let canvas_deadline = Instant::now() + Duration::from_secs(5);
    loop {
        host.run_steps(1);
        let ready = mounted_canvas
            .lock()
            .map(|board| !board.loading && board.error.is_none())
            .unwrap_or(false);
        if ready && find_node(&host.root(), "canvas.place-block").is_some() {
            break;
        }
        assert!(
            Instant::now() < canvas_deadline,
            "production Canvas host did not load within five seconds"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let canvas_cards_before: Vec<String> = knowledge_author_ids(&host.root())
        .into_iter()
        .map(|(author, _)| author)
        .filter(|author| author.starts_with("canvas.card."))
        .collect();
    let place = find_node(&host.root(), "canvas.place-block").expect("Canvas place control");
    let place_payload =
        serde_json::json!({"block_id":alpha_id.clone(),"x":123.0,"y":234.0}).to_string();
    host.event(click_event(place.node_id, Some(&place_payload)));
    host.run_steps(1);
    let canvas_client = CanvasBoardClient::new(live.base.clone(), rt.handle().clone());
    let placement_deadline = Instant::now() + Duration::from_secs(5);
    let persisted_canvas = loop {
        host.run_steps(1);
        if let Ok(board) = await_canvas(&canvas_client, &workspace_id, &canvas_id) {
            if board.placements.iter().any(|placement| {
                placement.placed_block_id == alpha_id
                    && (placement.x - 123.0).abs() < 0.1
                    && (placement.y - 234.0).abs() < 0.1
            }) {
                break board;
            }
        }
        assert!(
            Instant::now() < placement_deadline,
            "host-routed canvas.place-block did not persist and refetch within five seconds"
        );
        std::thread::sleep(Duration::from_millis(10));
    };
    let persisted_placement = persisted_canvas
        .placements
        .iter()
        .find(|placement| placement.placed_block_id == alpha_id)
        .expect("fresh Canvas readback contains the host-created placement");
    let persisted_card_author = canvas_card_author_id(&persisted_placement.placement_id);
    loop {
        host.run_steps(1);
        if find_node(&host.root(), &persisted_card_author).is_some() {
            break;
        }
        assert!(
            Instant::now() < placement_deadline,
            "mounted Canvas did not publish the backend-minted placement after authoritative refetch"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let canvas_cards_after: Vec<String> = knowledge_author_ids(&host.root())
        .into_iter()
        .map(|(author, _)| author)
        .filter(|author| author.starts_with("canvas.card."))
        .collect();
    assert!(!canvas_cards_before.contains(&persisted_card_author));
    assert!(canvas_cards_after.contains(&persisted_card_author));
    println!(
        "PROOF-042-C Canvas AccessKit tree diff: before={canvas_cards_before:?} after={canvas_cards_after:?} added={persisted_card_author}"
    );

    // Create a canonical tag-grouped view as test setup, then move the card exclusively through the
    // production collection pane's parameterized AccessKit control. Fresh query results and the raw
    // graph projection prove that the host persisted the exact remove-tag/add-tag edge transition.
    let create_tag = |title: &str| {
        graph_block_id(&live.post_json(
            &format!("/workspaces/{workspace_id}/loom/blocks"),
            &serde_json::json!({"content_type":"tag_hub","title":title}),
        ))
    };
    let todo_tag = create_tag(&format!("todo-{nonce}"));
    let done_tag = create_tag(&format!("done-{nonce}"));
    live.post_json(
        &format!("/workspaces/{workspace_id}/loom/edges"),
        &serde_json::json!({
            "source_block_id":alpha_id,
            "target_block_id":todo_tag,
            "edge_type":"tag",
            "created_by":"user"
        }),
    );
    let block_view_client = BlockViewClient::new(live.base.clone(), rt.handle().clone());
    let mut kanban_definition = BlockViewDefinition::of_kind(BlockViewKind::Kanban);
    kanban_definition.query.content_type = Some("note".to_owned());
    kanban_definition.group_by = Some(BlockViewGroupBy::Tag);
    let kanban_id = create_live_block_view(
        &block_view_client,
        &workspace_id,
        &format!("MT-042 Kanban {nonce}"),
        &kanban_definition,
    );
    let mounted_collection = host.state().mounted_block_collection_view();
    assert!(matches!(
        host.state_mut().open_block_collection_view(&kanban_id),
        NavDispatchOutcome::Opened { .. }
    ));
    let kanban_deadline = Instant::now() + Duration::from_secs(5);
    loop {
        host.run_steps(1);
        let ready = mounted_collection
            .lock()
            .map(|view| {
                view.view_block_id == kanban_id
                    && !view.loading
                    && !view.in_flight
                    && view.error.is_none()
            })
            .unwrap_or(false);
        if ready && find_node(&host.root(), "collection.kanban-move").is_some() {
            break;
        }
        assert!(
            Instant::now() < kanban_deadline,
            "production Kanban host did not load within five seconds"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let move_node = find_node(&host.root(), "collection.kanban-move").expect("Kanban move control");
    let move_payload = serde_json::json!({
        "block_id":alpha_id,
        "from_lane":todo_tag,
        "to_lane":done_tag
    })
    .to_string();
    host.event(click_event(move_node.node_id, Some(&move_payload)));
    host.run_steps(1);
    let move_deadline = Instant::now() + Duration::from_secs(5);
    loop {
        host.run_steps(1);
        if let Ok(results) = await_block_view_results(&block_view_client, &workspace_id, &kanban_id)
        {
            let in_done = results.groups.iter().any(|lane| {
                lane.key == done_tag && lane.blocks.iter().any(|block| block.block_id == alpha_id)
            });
            let in_todo = results.groups.iter().any(|lane| {
                lane.key == todo_tag && lane.blocks.iter().any(|block| block.block_id == alpha_id)
            });
            if in_done && !in_todo {
                break;
            }
        }
        assert!(
            Instant::now() < move_deadline,
            "host-routed collection.kanban-move did not persist and refetch within five seconds"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let row_author = collection_row_author_id(&alpha_id);
    let done_lane_author = collection_lane_author_id(&done_tag);
    let todo_lane_author = collection_lane_author_id(&todo_tag);
    loop {
        host.run_steps(1);
        let row_in_done = find_node(&host.root(), &row_author).is_some_and(|row| {
            row.value
                .as_deref()
                .is_some_and(|value| value.contains(&format!("lane={done_tag}")))
        });
        let done_contains = find_node(&host.root(), &done_lane_author).is_some_and(|lane| {
            lane.value
                .as_deref()
                .is_some_and(|value| value.contains(&alpha_id))
        });
        let todo_excludes = find_node(&host.root(), &todo_lane_author).is_none_or(|lane| {
            lane.value
                .as_deref()
                .is_none_or(|value| !value.contains(&alpha_id))
        });
        if row_in_done && done_contains && todo_excludes {
            break;
        }
        assert!(
            Instant::now() < move_deadline,
            "mounted collection AccessKit tree did not reconcile the moved row/lane"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let tag_graph = live.get_json(&format!("/workspaces/{workspace_id}/loom/graph/global"));
    let tag_edges = tag_graph["edges"].as_array().expect("raw graph tag edges");
    let tag_edge_count = |target: &str| {
        tag_edges.iter().filter(|row| {
            row["edge"]["source_block_id"].as_str() == Some(alpha_id.as_str())
                && row["edge"]["target_block_id"].as_str() == Some(target)
                && row["edge"]["edge_type"].as_str() == Some("tag")
        }).count()
    };
    assert_eq!(
        tag_edge_count(&done_tag),
        1,
        "canonical graph API contains exactly one destination tag edge"
    );
    assert_eq!(
        tag_edge_count(&todo_tag),
        0,
        "canonical graph API removed the source tag edge"
    );
    println!(
        "PROOF-042-D canonical graph edges: block={alpha_id} done_tag={done_tag} count=1 todo_tag={todo_tag} count=0"
    );

    // A completely fresh pane retains the same author_id -> NodeId mapping after durable reload.
    let reloaded = await_graph(&fresh_client, &workspace_id, 5).expect("durable graph reload");
    let fresh_mount = mount_live_graph(&workspace_id, reloaded);
    for (author, expected_node_id) in stable_node_ids {
        assert_eq!(
            find_node(&fresh_mount.harness.root(), &author)
                .expect("reloaded stable author_id")
                .node_id,
            expected_node_id
        );
    }
    for (author, expected_node_id) in stable_edge_ids {
        assert_eq!(
            find_node(&fresh_mount.harness.root(), &author)
                .expect("reloaded stable edge author_id")
                .node_id,
            expected_node_id
        );
    }
    drop(fresh_mount);
    drop(host);

    // Unreachable backend is a real transport failure. It must deliver Err, mount no fabricated graph
    // nodes, and expose a bounded Retry action that emits the typed retry event.
    let down_client = LoomGraphClient::new("http://127.0.0.1:9", rt.handle().clone());
    let down = await_graph(&down_client, &workspace_id, 6)
        .expect_err("unreachable backend must never fabricate a graph");
    let registry = Arc::new(Mutex::new(KnowledgeActionRegistry::new()));
    let mut down_view = LoomGraphView::global(&workspace_id);
    down_view.error = Some(down);
    down_view.install_knowledge_action_registry(registry);
    let down_graph = Arc::new(Mutex::new(down_view));
    let down_events = Arc::new(Mutex::new(Vec::new()));
    let down_graph_ui = Arc::clone(&down_graph);
    let down_events_ui = Arc::clone(&down_events);
    let mut down_harness = Harness::builder().build_ui(move |ui| {
        let palette = HsTheme::Dark.palette();
        if let Some(event) = down_graph_ui.lock().unwrap().show(ui, &palette) {
            down_events_ui.lock().unwrap().push(event);
        }
    });
    down_harness.run();
    down_harness.run();
    assert!(knowledge_author_ids(&down_harness.root())
        .iter()
        .all(|(author, _)| !author.starts_with("graph.node.")));
    let retry = find_node(
        &down_harness.root(),
        handshake_native::graph::graph_view::RETRY_AUTHOR_ID,
    )
    .expect("backend-loss graph exposes Retry");
    assert_eq!(retry.role, "Button");
    down_harness.event(click_event(retry.node_id, None));
    down_harness.run();
    assert!(down_events
        .lock()
        .unwrap()
        .iter()
        .any(|event| matches!(event, GraphEvent::Retry)));

    let cleanup_status = live.delete_workspace(&workspace_id);
    assert!(matches!(cleanup_status, 200 | 202 | 204));
    cleanup.active = false;
    let remaining = live.get_json("/workspaces");
    assert!(remaining
        .as_array()
        .expect("workspace list")
        .iter()
        .all(|workspace| workspace["id"].as_str() != Some(workspace_id.as_str())));
    assert_no_local_artifact_dir();
    println!(
        "MT-042 LIVE: workspace={workspace_id} blocks=[{alpha_id},{beta_id},{gamma_id}] \
         stable_author_ids={} seed_edges=2 add_remove_edge={created_edge_id} empty/backend-loss negatives=PASS",
        expected_nodes.len()
    );
}

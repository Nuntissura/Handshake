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
//!   SHAPE (the request-shape half is standalone; the DB round-trip is the gated `#[ignore]` test).
//! - AC-042-06: dispatch `collection.kanban-move {block_id,from,to}` => a CardMove event with the right
//!   `add_tags`/`remove_tags` (the updateLoomBlock tag-edge request shape).
//! - AC-042-07: dispatch `graph.add-edge {source_id,target_id}` => an AddEdge event (createLoomEdge shape).
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
//! AC-042-05/06/07/10 + PROOF-042-D's DB ROUND-TRIP halves (place-block -> loom_canvas_placements row,
//! kanban-move -> tag edge, add-edge -> edge row) are NEEDS_MANAGED_RESOURCE_PROOF: they need a running
//! Handshake-managed PostgreSQL with a seeded loom canvas + view. They are the `#[ignore]`d `*_live_pg`
//! tests, gated behind the `integration` feature; absent a seeded backend they are NOT faked and NOT a
//! fake-PG (the MT contract's REAL-PG REALITY gate). The AccessKit registration + dispatch + the typed
//! EVENT SHAPE the host would send to the E6 loom client are proven STANDALONE here with an in-memory
//! graph-projection fixture (IN-042-09 permits this when a real-PG fixture is not yet wired).
//!
//! ## Artifact hygiene (CX-212E)
//!
//! This MT writes NO screenshots (the AccessKit tree dump to stdout is the HBR-VIS proof — IN-042
//! CHURN/VIEWPORT/QUIET gate: "AccessKit tree dump = HBR-VIS proof printed to stdout, no screenshot
//! needed"). [`assert_no_local_artifact_dir`] still fails the run if a repo-local `tests/screenshots/`
//! or `test_output/` dir exists (the reviewer also greps `git ls-files "src/**/*.png"`).

use std::path::Path;
use std::sync::{Arc, Mutex};

use egui_kittest::kittest::NodeT;
use egui_kittest::Harness;

use handshake_native::accessibility::knowledge_action_registry::{
    canvas_card_author_id, collection_lane_author_id, collection_row_author_id, graph_node_author_id,
    KnowledgeActionRegistry, CANVAS_CONTROL_CATALOG, COLLECTION_CONTROL_CATALOG, GRAPH_CONTROL_CATALOG,
    HEALTH_CANARY_AUTHOR_ID,
};
use handshake_native::graph::block_collection_view::{
    BlockCollectionView, BlockViewDefinition, BlockViewEvent, BlockViewKind, BlockViewLane,
    BlockViewResults, LoomBlockRow,
};
use handshake_native::graph::canvas_board::{CanvasEvent, CanvasPlacementCard, LoomCanvasBoard};
use handshake_native::graph::graph_view::{GraphEdge, GraphEvent, GraphNode, LoomGraphView};
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
            let mut c = CanvasPlacementCard::new(pid, uuid::Uuid::new_v4().to_string(), (i as f32) * 240.0 + 30.0, 40.0, 200.0, 120.0);
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
            BlockViewLane { key: "todo".to_owned(), blocks: vec![r1] },
            BlockViewLane { key: "done".to_owned(), blocks: vec![r2] },
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
    value: Option<String>,
}

fn find_node(root: &egui_kittest::Node<'_>, author_id: &str) -> Option<FoundNode> {
    for node in root.children_recursive() {
        let ak = node.accesskit_node();
        if ak.author_id() == Some(author_id) {
            return Some(FoundNode {
                node_id: ak.id(),
                role: format!("{:?}", ak.role()),
                value: ak.value().map(|v| v.to_owned()),
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

/// A combined harness rendering all three knowledge panes into one CentralPanel, sharing ONE registry.
/// Each frame: sync the registry from the live pane state, render the pane, emit the registry into the
/// live AccessKit tree, then drain swarm dispatches into the typed event sinks (so a dispatched Click
/// reaches the pane in the SAME frame — RISK-042-04). Returns the shared pane handles + the harness.
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
                // GRAPH pane.
                ui.vertical(|ui| {
                    let mut graph = g.lock().unwrap();
                    let last_rect = ui.available_rect_before_wrap();
                    graph.sync_knowledge_registry(Some(last_rect));
                    ui.allocate_ui(egui::vec2(380.0, 360.0), |ui| {
                        if let Some(ev) = graph.show(ui, &palette) {
                            ge.lock().unwrap().push(ev);
                        }
                    });
                    graph.emit_knowledge_accesskit(ui);
                    let dispatched = graph.take_knowledge_dispatched(ui);
                    ge.lock().unwrap().extend(dispatched);
                });
                // CANVAS pane.
                ui.vertical(|ui| {
                    let mut canvas = cv.lock().unwrap();
                    canvas.sync_knowledge_registry();
                    ui.allocate_ui(egui::vec2(380.0, 360.0), |ui| {
                        if let Some(ev) = canvas.show(ui, &palette) {
                            ce.lock().unwrap().push(ev);
                        }
                    });
                    canvas.emit_knowledge_accesskit(ui);
                    let dispatched = canvas.take_knowledge_dispatched(ui);
                    ce.lock().unwrap().extend(dispatched);
                });
                // COLLECTION pane.
                ui.vertical(|ui| {
                    let mut collection = col.lock().unwrap();
                    collection.sync_knowledge_registry();
                    ui.allocate_ui(egui::vec2(380.0, 360.0), |ui| {
                        if let Some(ev) = collection.show(ui, &palette) {
                            cce.lock().unwrap().push(ev);
                        }
                    });
                    collection.emit_knowledge_accesskit(ui);
                    let dispatched = collection.take_knowledge_dispatched(ui);
                    cce.lock().unwrap().extend(dispatched);
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
        let n = find_node(&root, entry.author_id)
            .unwrap_or_else(|| panic!("AC-042-08: graph control '{}' must be present", entry.author_id));
        assert_eq!(n.role, "Button", "{} is a Button control", entry.author_id);
    }
    for entry in CANVAS_CONTROL_CATALOG {
        assert!(find_node(&root, entry.author_id).is_some(), "canvas control '{}' present", entry.author_id);
    }
    for entry in COLLECTION_CONTROL_CATALOG {
        assert!(find_node(&root, entry.author_id).is_some(), "collection control '{}' present", entry.author_id);
    }

    // AC-042-02: every graph block => graph.node.<block_id> Role::TreeItem.
    let graph = h.graph.lock().unwrap();
    assert!(graph.nodes.len() >= 3, "fixture seeds >=3 blocks");
    for node in &graph.nodes {
        let author = graph_node_author_id(&node.block_id);
        let found = find_node(&root, &author)
            .unwrap_or_else(|| panic!("AC-042-02: '{author}' (TreeItem) must be present"));
        assert_eq!(found.role, "TreeItem", "AC-042-02: '{author}' role must be TreeItem");
    }
    drop(graph);

    // AC-042-03: every canvas placement => canvas.card.<placement_id> Role::Group.
    let canvas = h.canvas.lock().unwrap();
    assert!(canvas.placements.len() >= 2, "fixture seeds 2 placements");
    for card in &canvas.placements {
        let author = canvas_card_author_id(&card.placement_id);
        let found = find_node(&root, &author)
            .unwrap_or_else(|| panic!("AC-042-03: '{author}' (Group) must be present"));
        assert_eq!(found.role, "Group", "AC-042-03: '{author}' role must be Group");
        // The card carries its source block_id in the AccessKit value (IN-042-02).
        assert!(
            found.value.as_deref().map(|v| v.contains("block_id=")).unwrap_or(false),
            "AC-042-03/IN-042-02: '{author}' value must carry block_id=; got {:?}",
            found.value
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
        let found = find_node(&root, &author).unwrap_or_else(|| panic!("'{author}' (Group lane) present"));
        assert_eq!(found.role, "Group", "'{author}' lane role must be Group");
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
            let mut graph = g.lock().unwrap();
            let last_rect = ui.available_rect_before_wrap();
            graph.sync_knowledge_registry(Some(last_rect));
            graph.show(ui, &palette);
            graph.emit_knowledge_accesskit(ui);
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
    assert!(!any_node, "AC-042-08: no per-node identity nodes when 0 blocks loaded");
    println!("AC-042-08: all graph-level controls present with 0 blocks; 0 per-node identity nodes");
}

// ── AC-042-04: dispatch graph.open-node {block_id} -> the pane emits OpenNode for that block ─────────

#[test]
fn ac04_dispatch_graph_open_node_emits_open() {
    let mut h = build_harness();
    h.harness.run();
    h.harness.run();

    // The target block is the first fixture block.
    let block_id = h.graph.lock().unwrap().nodes[0].block_id.clone();
    let open = find_node(&h.harness.root(), "graph.open-node").expect("graph.open-node control present");
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
    assert_eq!(h.graph.lock().unwrap().selected.as_deref(), Some(block_id.as_str()));
    println!("AC-042-04: AccessKit dispatch of graph.open-node opened the block (cross-pane open + selection)");
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
    println!("AC-042-04 (identity): clicking graph.node.<block_id> opened that block");
}

// ── AC-042-05: dispatch canvas.place-block {block_id,x,y} -> PlaceBlock event (route SHAPE) + new card ─

#[test]
fn ac05_dispatch_canvas_place_block_emits_place_and_new_card() {
    let mut h = build_harness();
    h.harness.run();
    h.harness.run();

    let new_block = uuid::Uuid::new_v4().to_string();
    let place = find_node(&h.harness.root(), "canvas.place-block").expect("canvas.place-block control present");
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
        let mut c = CanvasPlacementCard::new(new_placement_id.clone(), new_block.clone(), 100.0, 100.0, 200.0, 120.0);
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

// ── AC-042-06: dispatch collection.kanban-move {block_id,from,to} -> CardMove with the tag-edge shape ─

#[test]
fn ac06_dispatch_kanban_move_emits_cardmove_tag_shape() {
    let mut h = build_harness();
    h.harness.run();
    h.harness.run();

    // Move the first row's block from "todo" to "done".
    let block_id = h.collection.lock().unwrap().results.as_ref().unwrap().groups[0].blocks[0].block_id.clone();
    let mv = find_node(&h.harness.root(), "collection.kanban-move").expect("collection.kanban-move control present");
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
    let add = find_node(&h.harness.root(), "graph.add-edge").expect("graph.add-edge control present");
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
        "AC-042-07: graph.add-edge dispatch emitted AddEdge{{source,target}} (the createLoomEdge SHAPE); got {events:?}"
    );
    println!("AC-042-07: graph.add-edge dispatch emitted the AddEdge createLoomEdge request shape");
}

// ── PROOF-042-B / HBR-VIS: dump the full knowledge.* AccessKit tree to stdout ───────────────────────

#[test]
fn proof_b_full_knowledge_tree_dump() {
    let mut h = build_harness();
    h.harness.run();
    h.harness.run();
    let root = h.harness.root();

    let dump = knowledge_author_ids(&root);
    println!("--- PROOF-042-B: knowledge.* AccessKit node dump ({} nodes) ---", dump.len());
    for (author, role) in &dump {
        println!("{author}  role={role}");
    }

    // The reviewer must locate >=2 graph.node.<uuid>, >=1 canvas.card.<uuid>, and all graph controls.
    let graph_nodes = dump.iter().filter(|(a, _)| a.starts_with("graph.node.")).count();
    let canvas_cards = dump.iter().filter(|(a, _)| a.starts_with("canvas.card.")).count();
    assert!(graph_nodes >= 2, "PROOF-042-B: at least two graph.node.<uuid> nodes in the dump; got {graph_nodes}");
    assert!(canvas_cards >= 1, "PROOF-042-B: at least one canvas.card.<uuid> node in the dump; got {canvas_cards}");
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
        canvas.placements.iter().map(|c| c.placement_id.clone()).collect()
    };
    for id in &ids_before {
        assert_eq!(id.len(), 36, "CTRL-042-02: placement_id must be a 36-char UUID; got '{id}' ({} chars)", id.len());
        assert!(uuid::Uuid::parse_str(id).is_ok(), "CTRL-042-02: placement_id must parse as a UUID; got '{id}'");
        // The card node is addressable by the sanitized UUID.
        let author = canvas_card_author_id(id);
        assert!(find_node(&h.harness.root(), &author).is_some(), "card node for '{id}' present");
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
    let ids_after: Vec<String> = h.canvas.lock().unwrap().placements.iter().map(|c| c.placement_id.clone()).collect();
    assert_eq!(ids_before, ids_after, "CTRL-042-02: placement_ids are stable across a refresh cycle");
    for id in &ids_after {
        let author = canvas_card_author_id(id);
        assert!(find_node(&h.harness.root(), &author).is_some(), "card node for '{id}' still present after refresh");
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
    h.harness.event(click_event(open.node_id, Some("this is not json {{{ ]")));
    h.harness.run();
    h.harness.run();
    // No OpenNode produced (the payload was dropped) and the app is still alive (no panic).
    assert!(
        !h.graph_events.lock().unwrap().iter().any(|e| matches!(e, GraphEvent::OpenNode { .. })),
        "CTRL-042-03: a malformed payload must NOT produce an OpenNode (logged + dropped)"
    );

    // Dispatch canvas.place-block with a MISSING required field; same no-panic + no-event contract.
    let place = find_node(&h.harness.root(), "canvas.place-block").expect("canvas.place-block present");
    h.harness.event(click_event(place.node_id, Some(r#"{"block_id":"x"}"#))); // missing x/y
    h.harness.run();
    h.harness.run();
    assert!(
        !h.canvas_events.lock().unwrap().iter().any(|e| matches!(e, CanvasEvent::PlaceBlock { .. })),
        "CTRL-042-03: a payload missing required fields must NOT produce a PlaceBlock"
    );

    // Dispatch a parameterized action with NO payload at all; no-panic + no-event.
    let mv = find_node(&h.harness.root(), "collection.kanban-move").expect("collection.kanban-move present");
    h.harness.event(click_event(mv.node_id, None));
    h.harness.run();
    h.harness.run();
    assert!(
        !h.collection_events.lock().unwrap().iter().any(|e| matches!(e, BlockViewEvent::CardMove { .. })),
        "CTRL-042-03: a parameterized dispatch with no payload must NOT produce a CardMove"
    );
    println!("CTRL-042-03: malformed / missing / absent payloads are logged + dropped — no panic on the UI thread");
}

// ── AC-042-10 (gated): real-PG round-trip for place-block / kanban-move / add-edge ──────────────────
//
// NEEDS_MANAGED_RESOURCE_PROOF (the MT REAL-PG REALITY gate): the AccessKit registration + dispatch +
// the typed EVENT SHAPE are proven above with an in-memory fixture. The DB ROUND-TRIP (place-block ->
// loom_canvas_placements row; kanban-move -> tag edge; add-edge -> edge row; AC-042-05/06/07/10 +
// PROOF-042-D's `SELECT tag_edges WHERE block_id=...`) needs a running Handshake-managed PostgreSQL with
// a seeded loom canvas + view. It is NOT faked and NOT a fake-PG. Run against a live, seeded backend:
//   cargo test --features integration --test test_e7_knowledge_accesskit -- --ignored
#[test]
#[ignore = "NEEDS_MANAGED_RESOURCE_PROOF: real PostgreSQL/EventLedger + seeded loom canvas/view (AC-042-05/06/07/10, PROOF-042-D)"]
fn ac10_live_pg_round_trip() {
    // Intentionally a documented gate, not a fake. A future E6-wired integration run drives the E6 loom
    // client (POST .../placements, updateLoomBlock add_tags/remove_tags, POST /loom/edges) against a real
    // PG and SELECTs the resulting rows. This MT proves the request SHAPE + the AccessKit surface; the
    // DB authority touch is the host's E6 path under a live backend.
    panic!("run only with --features integration against a seeded live backend");
}

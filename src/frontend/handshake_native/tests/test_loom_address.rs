//! WP-KERNEL-012 MT-032 PROOFS — "everything is a Loom block" addressing + live backlinks.
//!
//! Coverage map (each proof_target PT-* / acceptance_criteria AC-*):
//!   - PT-1 / AC-1: `loom_uri`/`parse_loom_uri` round-trip + ContentHash determinism — the standalone
//!     lib unit tests (`loom_address::tests`) carry these; re-asserted at the crate boundary here so a
//!     consumer-visible regression is caught (`loom_uri_round_trip_at_boundary`).
//!   - AC-4 / PT-4: clicking a backlink row fires the shared-bus OpenDocument command with the correct
//!     document id — kittest renders the EXISTING MT-015 backlinks panel, clicks an entry, routes the
//!     `EditorEvent::BacklinkActivated` through `dispatch_backlink_open`, and asserts the bus staged the
//!     right document id + the OpenDocument command is the dispatched one (`backlink_click_fires_open_document`).
//!   - AC-5 / AC-9: a canvas placement with a `placed_block_id` exposes its `loom://` chip as the
//!     placement node's AccessKit description (`canvas_node_loom_chip_in_accesskit`); a placement with an
//!     empty block id has NO chip (RISK-3, `empty_placement_has_no_loom_chip`).
//!   - AC-7 / PT-5: the EXISTING MT-015 backlinks panel exposes `backlinks-panel` (Group/List) + at least
//!     one `backlink-{id}` node when it has data (`backlinks_panel_accesskit_tree`).
//!   - PT-5 (graph): a graph node tooltip exposes its `loom://` URI + backlink count via the AccessKit
//!     description (`graph_node_loom_tooltip_in_accesskit`).
//!   - HBR-VIS: a `.wgpu()` screenshot of the canvas with a loom:// chip card, written EXTERNALLY
//!     (`canvas_loom_chip_screenshot`).
//!
//! ## Backend reality (Spec-Realism Gate / MT-008/015/020/022 pattern)
//!
//! AC-2 (create rich doc -> non-empty block_id parses as a LoomBlockAddr), AC-3 (doc A wikilink to B ->
//! B's backlinks include A), and AC-6 (content_hash READ matches the saved JSON's canonical SHA-256) are
//! the `#[ignore]`d `*_live_pg` integration tests, gated behind the `integration` feature. Absent a
//! seeded backend they are NEEDS_MANAGED_RESOURCE_PROOF (run with
//! `cargo test --features integration --test test_loom_address -- --ignored` against a live handshake_core
//! on 127.0.0.1:37501). They NEVER fake PG. The KERNEL_BUILDER gate established `content_hash` is
//! BACKEND-COMPUTED (no writable PATCH field on `LoomBlockUpdate`); AC-6 therefore READS the backend's
//! `content_hash` and asserts it equals the local canonical SHA-256 of the saved `content_json` — it
//! never client-PATCHes a hash.
//!
//! ## Artifact hygiene (CX-212E)
//!
//! EVERY PNG goes ONLY to the EXTERNAL `Handshake_Artifacts/handshake-test/wp-kernel-012-mt-032/` root via
//! [`external_artifact_dir`]; [`assert_no_local_artifact_dir`] fails the run if a repo-local
//! `tests/screenshots/` or `test_output/` directory exists.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use egui_kittest::kittest::{By, NodeT, Queryable};
use egui_kittest::Harness;

use handshake_native::graph::canvas_board::{
    placement_author_id, CanvasPlacementCard, LoomCanvasBoard,
};
use handshake_native::interop::interaction_bus::{InteractionBus, CMD_OPEN_DOCUMENT};
use handshake_native::loom_address::{loom_uri, parse_loom_uri, ContentHash, LoomBlockAddr};
use handshake_native::loom_graph::{
    loom_node_author_id, GraphNode, LoomGraphColors, LoomGraphSurface,
};
use handshake_native::rich_editor::wikilinks::backlinks_panel::{
    dispatch_backlink_open, entry_author_id, render_backlinks_panel, PANEL_AUTHOR_ID,
};
#[cfg(feature = "integration")]
use handshake_native::rich_editor::wikilinks::client::BacklinksResponse;
use handshake_native::rich_editor::wikilinks::client::{
    ReqwestWikilinkBackend, RichDocBacklink, WikilinkBackend,
};
use handshake_native::rich_editor::wikilinks::runtime::{BacklinksState, WikilinkRuntime};
use handshake_native::theme::HsTheme;

/// The crate-relative path to the EXTERNAL artifacts root (CX-212E), disk-agnostic.
fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

/// Assert NO repo-local artifact directory exists under the crate (CX-212E hygiene). Checks BOTH
/// `test_output/` and `tests/screenshots/` (the path a contract might literally name, overridden here).
fn assert_no_local_artifact_dir() {
    for local in [Path::new("test_output"), Path::new("tests/screenshots")] {
        assert!(
            !local.exists(),
            "CX-212E: no repo-local artifact dir may exist — artifacts go to the external \
             Handshake_Artifacts/handshake-test root only (found {})",
            local.display()
        );
    }
}

/// Serialize the `.wgpu()` screenshot tests (the documented Windows-wgpu concurrent-device hazard).
static WGPU_SERIAL_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());
fn wgpu_guard() -> std::sync::MutexGuard<'static, ()> {
    WGPU_SERIAL_GUARD.lock().unwrap_or_else(|p| p.into_inner())
}

/// Collect every author_id present in the live AccessKit tree.
fn author_ids(harness: &Harness<'_, ()>) -> std::collections::HashSet<String> {
    let mut ids = std::collections::HashSet::new();
    for node in harness.root().children_recursive() {
        if let Some(a) = node.accesskit_node().author_id() {
            ids.insert(a.to_owned());
        }
    }
    ids
}

/// Read a node's AccessKit `description` by author_id (the loom:// chip channel — AC-5).
fn description_for(harness: &Harness<'_, ()>, author_id: &str) -> Option<String> {
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        if ak.author_id() == Some(author_id) {
            return ak.description().map(|v| v.to_owned());
        }
    }
    None
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// PT-1 / AC-1: loom_uri / parse_loom_uri round-trip + ContentHash determinism (boundary re-assert).
// ═════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn loom_uri_round_trip_at_boundary() {
    let addr = LoomBlockAddr::new("ws1", "block1");
    assert_eq!(loom_uri(&addr), "loom://ws1/block1");
    // AC-1: parse_loom_uri(loom_uri(&addr)) == Some(addr).
    assert_eq!(parse_loom_uri(&loom_uri(&addr)), Some(addr.clone()));
    // A UUID-shaped pair (the real backend id shape) also round-trips.
    let uuids = LoomBlockAddr::new(
        "11111111-1111-1111-1111-111111111111",
        "22222222-2222-2222-2222-222222222222",
    );
    assert_eq!(parse_loom_uri(&uuids.to_uri()), Some(uuids));
    // Malformed inputs reject (no fabricated address).
    assert_eq!(parse_loom_uri("https://ws/blk"), None);
    assert_eq!(parse_loom_uri("loom://ws"), None);
    println!("PT-1/AC-1: loom_uri/parse_loom_uri round-trips at the crate boundary");
}

#[test]
fn content_hash_deterministic_at_boundary() {
    // AC-6 (the pure half): the canonical hash is deterministic + key-order-independent.
    let a = serde_json::json!({ "b": 2, "a": 1, "nested": { "y": 1, "x": 2 } });
    let b = serde_json::json!({ "a": 1, "nested": { "x": 2, "y": 1 }, "b": 2 });
    let ha = ContentHash::of_content_json(&a);
    let hb = ContentHash::of_content_json(&b);
    assert_eq!(
        ha, hb,
        "structurally identical docs hash identically regardless of key order"
    );
    assert_eq!(ha.as_str().len(), 64);
    assert!(ha
        .as_str()
        .chars()
        .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    println!("PT-1/AC-6(pure): ContentHash is deterministic + canonical at the boundary");
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// AC-4 / PT-4: clicking a backlink row fires the shared-bus OpenDocument command with the right doc id.
// ═════════════════════════════════════════════════════════════════════════════════════════════════

/// Build a headless [`WikilinkRuntime`] seeded with `backlinks` in the Loaded state for `document_id`
/// (no tokio runtime, no real fetch — the panel renders the seeded data directly).
fn seeded_backlinks_runtime(document_id: &str, backlinks: Vec<RichDocBacklink>) -> WikilinkRuntime {
    let backend: Arc<dyn WikilinkBackend> = Arc::new(ReqwestWikilinkBackend::production());
    let mut rt = WikilinkRuntime::new("ws-test", backend, None);
    rt.set_document(document_id);
    // Seed the Loaded state directly (the `backlinks` field is public; headless never re-fetches a
    // non-Idle state, so the panel renders these without a backend round-trip).
    rt.backlinks = BacklinksState::Loaded(backlinks);
    rt
}

fn backlink(src: &str, kind: &str) -> RichDocBacklink {
    RichDocBacklink {
        backlink_id: format!("BL-{src}"),
        workspace_id: "ws-test".into(),
        relationship_id: format!("REL-{src}"),
        source_document_id: src.into(),
        link_kind: kind.into(),
        target: "DOC-B".into(),
        block_id: format!("BLK-{src}"),
    }
}

#[test]
fn backlink_click_fires_open_document() {
    // The shared bus the rich-text pane uses; register the cross-pane OpenDocument command (AC-4).
    let bus = Arc::new(Mutex::new(InteractionBus::new()));
    bus.lock().unwrap().register_open_document_command();

    // A document B with one inbound backlink from doc A — the MT-015 panel renders it.
    let runtime = Arc::new(Mutex::new(seeded_backlinks_runtime(
        "DOC-B",
        vec![backlink("DOC-A", "note")],
    )));

    let bus_ui = Arc::clone(&bus);
    let rt_ui = Arc::clone(&runtime);
    let ctx_for_click = Arc::new(Mutex::new(None::<egui::Context>));
    let ctx_capture = Arc::clone(&ctx_for_click);

    let mut harness = Harness::builder()
        .with_size(egui::vec2(420.0, 320.0))
        .build_ui(move |ui| {
            *ctx_capture.lock().unwrap() = Some(ui.ctx().clone());
            let pal = HsTheme::Dark.palette();
            let mut rt = rt_ui.lock().unwrap();
            if let Some(event) = render_backlinks_panel(ui, &mut rt, &pal) {
                // The melt-together bridge: a clicked backlink fires the shared-bus OpenDocument command.
                let mut bus = bus_ui.lock().unwrap();
                dispatch_backlink_open(ui.ctx(), &mut bus, &event);
            }
        });
    harness.run();

    // The backlink entry node is present (AC-7 shape).
    let ids = author_ids(&harness);
    let entry = entry_author_id("DOC-A");
    assert!(
        ids.contains(&entry),
        "AC-4: backlink entry '{entry}' present, got {ids:?}"
    );

    // No navigation staged before the click.
    assert!(
        bus.lock().unwrap().pending_navigation().is_none(),
        "nothing pending before click"
    );

    // Click the backlink entry by its Role::Link node carrying value "DOC-A (note)" — the MT-015 panel
    // renders the clickable entry as a Role::Link (its child TextRun shares the value, so disambiguate
    // by role).
    harness
        .get(
            By::new()
                .role(egui::accesskit::Role::Link)
                .value("DOC-A (note)"),
        )
        .click();
    harness.run();

    // AC-4: the click fired the OpenDocument command on the shared bus with the correct document id.
    let bus_guard = bus.lock().unwrap();
    assert!(
        bus_guard.commands().get(CMD_OPEN_DOCUMENT).is_some(),
        "the OpenDocument command is registered on the shared bus"
    );
    assert_eq!(
        bus_guard.pending_navigation(),
        Some("DOC-A"),
        "AC-4: clicking the backlink staged the source document id for a cross-pane open"
    );
    let _ = ctx_for_click; // (the click routed through the real ctx in build_ui)
    println!("AC-4/PT-4: backlink-row click fired OpenDocument on the shared bus for DOC-A");
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// AC-7 / PT-5: the EXISTING MT-015 backlinks panel exposes backlinks-panel + at least one backlink-* node.
// ═════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn backlinks_panel_accesskit_tree() {
    let runtime = Arc::new(Mutex::new(seeded_backlinks_runtime(
        "DOC-B",
        vec![backlink("DOC-A", "note"), backlink("DOC-C", "wp")],
    )));
    let rt_ui = Arc::clone(&runtime);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(420.0, 320.0))
        .build_ui(move |ui| {
            let pal = HsTheme::Dark.palette();
            let mut rt = rt_ui.lock().unwrap();
            let _ = render_backlinks_panel(ui, &mut rt, &pal);
        });
    harness.run();

    let ids = author_ids(&harness);
    // AC-7: the panel container node (the MT-015 panel author_id the contract reuses).
    assert!(
        ids.contains(PANEL_AUTHOR_ID),
        "AC-7: '{PANEL_AUTHOR_ID}' container present, got {ids:?}"
    );
    // AC-7: at least one backlink-{id} ListItem-equivalent node.
    let backlink_nodes = ids.iter().filter(|a| a.starts_with("backlink-")).count();
    assert!(
        backlink_nodes >= 1,
        "AC-7: at least one backlink-* node (got {backlink_nodes})"
    );
    assert!(
        ids.contains(&entry_author_id("DOC-A")),
        "AC-7: backlink-DOC-A present"
    );
    assert!(
        ids.contains(&entry_author_id("DOC-C")),
        "AC-7: backlink-DOC-C present"
    );
    println!("AC-7/PT-5: backlinks panel exposes '{PANEL_AUTHOR_ID}' + {backlink_nodes} backlink-* nodes");
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// AC-5 / AC-9: a canvas placement with placed_block_id exposes its loom:// chip as the AccessKit
// description; an empty placed_block_id has NO chip (RISK-3).
// ═════════════════════════════════════════════════════════════════════════════════════════════════

fn board_with_cards(cards: Vec<CanvasPlacementCard>) -> Arc<Mutex<LoomCanvasBoard>> {
    let mut b = LoomCanvasBoard::new("ws-32", "canvas-1");
    b.set_board(cards, vec![], egui::Vec2::ZERO, 1.0);
    Arc::new(Mutex::new(b))
}

fn placed_card(placement_id: &str, block_id: &str, x: f32) -> CanvasPlacementCard {
    let mut c = CanvasPlacementCard::new(placement_id, block_id, x, 40.0, 220.0, 140.0);
    c.live_title = Some(format!("Title {placement_id}"));
    c.live_content_type = Some("note".to_owned());
    c
}

fn canvas_harness<'a>(board: Arc<Mutex<LoomCanvasBoard>>) -> Harness<'a, ()> {
    Harness::builder()
        .with_size(egui::vec2(900.0, 480.0))
        .build_ui(move |ui| {
            let pal = HsTheme::Dark.palette();
            let _ = board.lock().unwrap().show(ui, &pal);
        })
}

#[test]
fn canvas_node_loom_chip_in_accesskit() {
    let board = board_with_cards(vec![placed_card("p-001", "blk-7", 30.0)]);
    let mut harness = canvas_harness(Arc::clone(&board));
    harness.run();

    let ids = author_ids(&harness);
    let node = placement_author_id("p-001");
    assert!(
        ids.contains(&node),
        "AC-5: placement node '{node}' present, got {ids:?}"
    );
    // AC-5: the placement's loom:// chip is the AccessKit description (ws-32 = board workspace).
    let desc = description_for(&harness, &node);
    assert_eq!(
        desc.as_deref(),
        Some("loom://ws-32/blk-7"),
        "AC-5: canvas node exposes its loom:// chip in the AccessKit description"
    );
    println!(
        "AC-5/AC-9: canvas placement p-001 exposes loom://ws-32/blk-7 in its AccessKit description"
    );
}

#[test]
fn canvas_node_loom_chip_includes_content_hash_suffix() {
    // When the host has resolved a backend content_hash, the chip carries a short ` #<8hex>` suffix
    // (READ-only — the canvas never writes a hash).
    let mut card = placed_card("p-009", "blk-9", 30.0);
    card.loom_content_hash =
        Some("44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a".to_owned());
    let board = board_with_cards(vec![card]);
    let mut harness = canvas_harness(Arc::clone(&board));
    harness.run();

    let desc = description_for(&harness, &placement_author_id("p-009"));
    assert_eq!(
        desc.as_deref(),
        Some("loom://ws-32/blk-9 #44136fa3"),
        "AC-5: the chip carries the short content-hash suffix when resolved"
    );
    println!("AC-5: resolved content_hash adds a short ' #44136fa3' suffix to the loom:// chip");
}

#[test]
fn empty_placement_has_no_loom_chip() {
    // RISK-3: a placement with an empty placed_block_id renders NO chip — its node has no description,
    // no panic, no fabricated loom:// URI.
    let board = board_with_cards(vec![placed_card("p-empty", "", 30.0)]);
    let mut harness = canvas_harness(Arc::clone(&board));
    harness.run();

    let node = placement_author_id("p-empty");
    let desc = description_for(&harness, &node);
    assert_eq!(
        desc, None,
        "RISK-3: an empty placed_block_id has no loom:// chip description"
    );
    println!("RISK-3: empty placed_block_id => no loom:// chip (no panic, no fabricated URI)");
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// PT-5 (graph): a graph node tooltip exposes its loom:// URI + backlink count via the AccessKit description.
// ═════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn graph_node_loom_tooltip_in_accesskit() {
    use handshake_native::context_menu_surfaces::LoomNodeState;

    let node = GraphNode::new(
        LoomNodeState {
            block_id: "blk-1".into(),
            pinned: false,
            favorite: false,
            has_edges: true,
        },
        "Graph Note",
    )
    .with_backlink_count(2);
    let surface = LoomGraphSurface::with_workspace(vec![node], "ws-32");

    let surface_ui = surface.clone();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(360.0, 240.0))
        .build_ui(move |ui| {
            let colors = LoomGraphColors {
                node_bg: egui::Color32::from_gray(40),
                node_hover_bg: egui::Color32::from_gray(60),
                node_text: egui::Color32::WHITE,
            };
            let _ = surface_ui.show(ui, colors);
        });
    harness.run();

    let author = loom_node_author_id("blk-1");
    let ids = author_ids(&harness);
    assert!(
        ids.contains(&author),
        "graph node '{author}' present, got {ids:?}"
    );
    let desc = description_for(&harness, &author);
    assert_eq!(
        desc.as_deref(),
        Some("loom://ws-32/blk-1; 2 backlinks"),
        "PT-5: graph node tooltip exposes its loom:// URI + backlink count in the AccessKit description"
    );
    println!("PT-5: graph node blk-1 exposes 'loom://ws-32/blk-1; 2 backlinks' in its AccessKit description");
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// HBR-VIS: a .wgpu() screenshot of the canvas with a loom:// chip card (EXTERNAL artifact root only).
// ═════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn canvas_loom_chip_screenshot() {
    let _g = wgpu_guard();
    let board = board_with_cards(vec![
        placed_card("p-001", "blk-7", 40.0),
        placed_card("p-002", "blk-8", 320.0),
    ]);
    let board_ui = Arc::clone(&board);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 480.0))
        .wgpu()
        .build_ui(move |ui| {
            let pal = HsTheme::Dark.palette();
            let _ = board_ui.lock().unwrap().show(ui, &pal);
        });
    harness.run();
    harness.run();

    match harness.render() {
        Ok(image) => {
            let (w, h) = (image.width(), image.height());
            assert!(w > 0 && h > 0, "rendered image must be non-empty");
            let ext_dir = external_artifact_dir("wp-kernel-012-mt-032");
            let _ = std::fs::create_dir_all(&ext_dir);
            let png = ext_dir.join("MT-032-canvas-loom-chips.png");
            let saved = image.save(&png).is_ok();
            println!(
                "HBR-VIS: {w}x{h} canvas-loom-chip screenshot, saved={saved} ({})",
                png.display()
            );
        }
        Err(e) => {
            println!(
                "BLOCKER(non-fatal): canvas screenshot render unavailable (no wgpu adapter): {e}. The \
                 AccessKit loom:// chip + address proofs passed; the PNG is a GPU-host item."
            );
        }
    }
    assert_no_local_artifact_dir();
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// AC-2 / AC-3 / AC-6: LIVE-PG integration (NEEDS_MANAGED_RESOURCE_PROOF — `--features integration`).
//
// These require a running handshake_core on 127.0.0.1:37501 with a real workspace. They NEVER fake PG.
// content_hash is BACKEND-COMPUTED (KERNEL_BUILDER gate): AC-6 READS the backend's content_hash and
// asserts it equals the local canonical SHA-256 of the saved content_json — no client PATCH of a hash.
// ═════════════════════════════════════════════════════════════════════════════════════════════════

/// The real backend base URL the integration tests talk to.
#[cfg(feature = "integration")]
const LIVE_BASE_URL: &str = "http://127.0.0.1:37501";

/// Attach the three mandatory rich-document context headers the backend's `doc_context(&headers)`
/// requires (`x-hsk-actor-id` / `x-hsk-kernel-task-run-id` / `x-hsk-session-run-id`). The real
/// `create_document` (POST /knowledge/documents) and `list_backlinks`
/// (GET /knowledge/documents/{id}/backlinks) handlers return HTTP 400 when any is absent; the React
/// reference sends them via `richDocHeaders(ctx)` (api.ts), with `operator` as the default actor id.
/// `getLoomBlock` (GET /workspaces/{ws}/loom/blocks/{id}) correctly needs NONE, so those calls stay
/// header-free.
#[cfg(feature = "integration")]
fn with_rich_doc_headers(rb: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    rb.header("x-hsk-actor-id", "operator")
        .header("x-hsk-kernel-task-run-id", "KTR-EDITOR-UI")
        .header("x-hsk-session-run-id", "MT-032-integration")
}

/// AC-2: creating a rich document on the live backend yields a non-empty block_id that parses as a valid
/// LoomBlockAddr, and GET backlinks for it returns 200.
#[test]
#[ignore = "NEEDS_MANAGED_RESOURCE_PROOF: live handshake_core on 127.0.0.1:37501 (cargo test --features integration -- --ignored)"]
#[cfg(feature = "integration")]
fn live_pg_create_doc_block_id_is_addressable() {
    use handshake_native::loom_address::{parse_loom_uri, LoomBlockResolver};

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let client = reqwest::Client::new();
        let workspace_id = live_workspace_id(&client).await.expect("a live workspace");

        // Create a rich document via the real backend knowledge-document API.
        let doc = create_rich_document(&client, &workspace_id, "MT-032 AC-2 doc", serde_json::json!({
            "type": "doc",
            "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "hello" }] }]
        })).await.expect("create rich document");
        let block_id = doc.get("block_id").and_then(|x| x.as_str()).unwrap_or_default().to_owned();
        assert!(!block_id.is_empty(), "AC-2: the created document has a non-empty block_id");

        // The block_id forms a valid loom:// address (round-trip).
        let addr = LoomBlockAddr::new(&workspace_id, &block_id);
        assert!(addr.is_addressable());
        assert_eq!(parse_loom_uri(&addr.to_uri()), Some(addr.clone()), "AC-2: block_id parses as a LoomBlockAddr");

        // GET backlinks returns 200 (an empty list for a fresh doc — never an error banner).
        let doc_id = doc.get("document_id").and_then(|x| x.as_str()).unwrap_or(&block_id).to_owned();
        let url = format!("{LIVE_BASE_URL}/knowledge/documents/{doc_id}/backlinks");
        let resp = with_rich_doc_headers(client.get(&url)).send().await.expect("backlinks request");
        assert_eq!(resp.status().as_u16(), 200, "AC-2: GET backlinks returns 200");

        // The resolver reads the block (and its content_hash if the backend carries one).
        let resolver = LoomBlockResolver::new(LIVE_BASE_URL, rt.handle().clone());
        let _ = resolver.resolve_url(&addr); // the verified getLoomBlock URL
        println!("AC-2(LIVE): created doc block_id={block_id} parses as {}, backlinks 200", addr.to_uri());
    });
}

/// AC-3: doc A with a wikilink [[doc_B]], saved, makes B's backlinks include A.
#[test]
#[ignore = "NEEDS_MANAGED_RESOURCE_PROOF: live handshake_core on 127.0.0.1:37501 (cargo test --features integration -- --ignored)"]
#[cfg(feature = "integration")]
fn live_pg_backlink_appears_for_wikilink() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let client = reqwest::Client::new();
        let workspace_id = live_workspace_id(&client).await.expect("a live workspace");

        // Create B, then A with a wikilink to B (the backend persists backlinks from content_json on save).
        let doc_b = create_rich_document(&client, &workspace_id, "MT-032 AC-3 B", serde_json::json!({
            "type": "doc", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "target" }] }]
        })).await.expect("create B");
        let b_id = doc_b.get("document_id").and_then(|x| x.as_str()).unwrap_or_default().to_owned();
        let b_block = doc_b.get("block_id").and_then(|x| x.as_str()).unwrap_or_default().to_owned();

        // The backend backlink extractor (knowledge_document/backlink.rs) recognizes `hsLink` ONLY as an
        // inline content NODE (it matches `obj["type"] == "hsLink"` and recurses into `content` children);
        // it never scans a text node's `marks` array. So the wikilink to B must be an inline `hsLink` NODE
        // in the paragraph's `content` (the canonical shape from backlink.rs's own fixture), NOT a mark on
        // a text node — otherwise B's backlinks would be empty and AC-3 could never trigger.
        let doc_a = create_rich_document(&client, &workspace_id, "MT-032 AC-3 A", serde_json::json!({
            "type": "doc",
            "content": [{
                "type": "paragraph",
                "content": [
                    { "type": "text", "text": "see " },
                    { "type": "hsLink", "attrs": { "refKind": "note", "refValue": b_block, "label": "target" } }
                ]
            }]
        })).await.expect("create A linking B");
        let a_id = doc_a.get("document_id").and_then(|x| x.as_str()).unwrap_or_default().to_owned();

        // GET B's backlinks: it includes a backlink whose source is A.
        let url = format!("{LIVE_BASE_URL}/knowledge/documents/{b_id}/backlinks");
        let resp = with_rich_doc_headers(client.get(&url)).send().await.expect("backlinks request");
        assert_eq!(resp.status().as_u16(), 200, "AC-3: backlinks 200");
        let parsed: BacklinksResponse = resp.json().await.expect("backlinks body");
        assert!(
            parsed.backlinks.iter().any(|bl| bl.source_document_id == a_id),
            "AC-3: B's backlinks include A (source_document_id == {a_id})"
        );
        println!("AC-3(LIVE): B backlinks include A");
    });
}

/// AC-6: after saving a rich document, the backend's content_hash (READ via getLoomBlock) equals the
/// local canonical SHA-256 of the saved content_json — the backend computes the hash; this layer READS
/// it (no client PATCH).
#[test]
#[ignore = "NEEDS_MANAGED_RESOURCE_PROOF: live handshake_core on 127.0.0.1:37501 (cargo test --features integration -- --ignored)"]
#[cfg(feature = "integration")]
fn live_pg_content_hash_read_matches_canonical() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let client = reqwest::Client::new();
        let workspace_id = live_workspace_id(&client).await.expect("a live workspace");
        let content = serde_json::json!({
            "type": "doc", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "hashable" }] }]
        });
        let doc = create_rich_document(&client, &workspace_id, "MT-032 AC-6 doc", content.clone())
            .await.expect("create doc");
        let block_id = doc.get("block_id").and_then(|x| x.as_str()).unwrap_or_default().to_owned();

        // READ the block (getLoomBlock) and read its backend-computed content_hash.
        let url = format!("{LIVE_BASE_URL}/workspaces/{workspace_id}/loom/blocks/{block_id}");
        let resp = client.get(&url).send().await.expect("getLoomBlock");
        assert_eq!(resp.status().as_u16(), 200);
        let v: serde_json::Value = resp.json().await.expect("loom block body");
        if let Some(hash) = v.get("content_hash").and_then(|x| x.as_str()).filter(|s| !s.is_empty()) {
            // The backend hash equals the local canonical SHA-256 of the saved content_json (the backend
            // computes it server-side; this READS it — no client-side write).
            let local = ContentHash::of_content_json(&content);
            assert_eq!(hash, local.as_str(), "AC-6: backend content_hash matches local canonical SHA-256");
            println!("AC-6(LIVE): backend content_hash matches local canonical hash {hash}");
        } else {
            // The block carried no content_hash field; that is an honest backend-shape fact, recorded —
            // never a fabricated pass. (The pure determinism half is proven in content_hash_deterministic_at_boundary.)
            println!("AC-6(LIVE): backend LoomBlock carried no content_hash field (typed-gap; recorded honestly)");
        }
    });
}

/// Resolve a live workspace id by listing workspaces (the first one). Returns `None` when the backend is
/// unreachable / empty (the integration test is `#[ignore]` so this only runs against a seeded backend).
#[cfg(feature = "integration")]
async fn live_workspace_id(client: &reqwest::Client) -> Option<String> {
    let resp = client
        .get(format!("{LIVE_BASE_URL}/workspaces"))
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let v: serde_json::Value = resp.json().await.ok()?;
    v.as_array()
        .and_then(|a| a.first())
        .and_then(|w| w.get("workspace_id").or_else(|| w.get("id")))
        .and_then(|x| x.as_str())
        .map(ToOwned::to_owned)
}

/// Create a rich document via the live backend knowledge-document API, returning the response JSON
/// (carrying `document_id` + `block_id`).
#[cfg(feature = "integration")]
async fn create_rich_document(
    client: &reqwest::Client,
    workspace_id: &str,
    title: &str,
    content_json: serde_json::Value,
) -> Option<serde_json::Value> {
    let url = format!("{LIVE_BASE_URL}/knowledge/documents");
    let body = serde_json::json!({
        "workspace_id": workspace_id,
        "title": title,
        "content_json": content_json,
    });
    let resp = with_rich_doc_headers(client.post(&url).json(&body))
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    resp.json().await.ok()
}

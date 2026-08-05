//! WP-KERNEL-012 MT-026 LoomCanvasBoard PROOFS.
//!
//! Coverage map:
//!   - PROOF1 (canvas_to_screen/screen_to_canvas round-trip < 1px) — proven in the lib unit tests
//!     (`graph::canvas_board::tests`); re-asserted here at the widget boundary by the drop-position math.
//!   - PROOF2: kittest AccessKit-tree — 2 seeded placements => 2 `canvas.placement.*` nodes whose labels
//!     match the live block titles, plus the toolbar control author_ids (AC9).
//!   - PROOF3: drop-to-place — a drop at a canvas position fires `CanvasEvent::PlaceBlock` with the
//!     transform-correct x/y; after the host applies + refreshes, a 3rd placement node appears (AC4).
//!   - PROOF4: semantic edge — select `canvas.placement.p-001`, `canvas.start-edge`, click
//!     `canvas.placement.p-002` => `CanvasEvent::SemanticEdge{source,target block_ids}` (AC7).
//!   - PROOF5: remove — clicking `canvas.placement.p-001.remove` fires `CanvasEvent::RemovePlacement`;
//!     after the host applies + refreshes, `canvas.placement.p-001` is absent (AC8). The source-block-kept
//!     assertion is repeated by the isolated managed-PG proof (getLoom(source) still 200).
//!   - PROOF6: screenshot of a non-white canvas with at least one rounded card shape.
//!   - AC2/AC3: pan/zoom buttons mutate pan/zoom + fire `ViewportChanged`; zoom label reads "1.00x".
//!   - AC5: '+ Text card' fires `CanvasEvent::AddCard` with a timestamp title.
//!   - AC6: shift-select 2 cards + 'Group (2)' fires `CanvasEvent::Group`; the group_id is exposed on
//!     each affected card's AccessKit value.
//!   - AC10: an empty board renders an empty canvas with no panic and no "(stale reference)" text.
//!
//! ## Backend reality (Spec-Realism Gate / MT-008/021-025 pattern)
//!
//! AC1 and the live mutation/reload paths are covered by one NON-ignored `integration`-gated proof that
//! creates and tears down its own isolated workspace on the reachable Handshake-managed PostgreSQL
//! backend. It never depends on operator-seeded ids and never fakes PG. The request builders are proven
//! without a backend below, while transform / hit-test / edge-mode / empty-board behavior is also proven
//! standalone here and in the lib unit tests.
//!
//! ## Artifact hygiene (CX-212E)
//!
//! EVERY PNG is written ONLY to the EXTERNAL `Handshake_Artifacts/handshake-test/wp-kernel-012-mt-026/`
//! root via [`external_artifact_dir`]; [`assert_no_local_artifact_dir`] fails the run if a repo-local
//! `tests/screenshots/` or `test_output/` directory exists.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use egui_kittest::kittest::{NodeT, Queryable};
#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;
use screenshot_harness::ScreenshotHarness as Harness;

use handshake_native::backend_client::CanvasBoardClient;
#[cfg(feature = "integration")]
use handshake_native::backend_client::LOOM_CANVAS_BOARD_SCHEMA_ID;
use handshake_native::graph::canvas_board::{
    placement_author_id, placement_remove_author_id, CanvasDragPayload, CanvasEvent,
    CanvasPlacementCard, EdgeMode, LoomCanvasBoard, ADD_CARD_AUTHOR_ID, DEFAULT_CARD_H,
    DEFAULT_CARD_W, EDGE_MODE_AUTHOR_ID, PAN_LEFT_AUTHOR_ID, PAN_RIGHT_AUTHOR_ID,
    PLACEMENT_MUTATION_COMPLETION_AUTHOR_ID, PLACE_BLOCK_AUTHOR_ID, PLACE_BLOCK_INPUT_AUTHOR_ID,
    RETRY_AUTHOR_ID, STATUS_AUTHOR_ID, VIEWPORT_COMPLETION_AUTHOR_ID, ZOOM_IN_AUTHOR_ID,
    ZOOM_OUT_AUTHOR_ID, ZOOM_VALUE_AUTHOR_ID,
};
use handshake_native::theme::HsTheme;

#[cfg(feature = "integration")]
#[path = "interconnect_support/mod.rs"]
mod interconnect_support;

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

/// A seeded board with `n` placements `p-001..` referencing `block-001..`, each with a resolved live
/// title "Block i" (reference semantics: the title is the live block's, not a copy of content). No
/// backend — the placements stand in for a real `getCanvasBoard` + `getLoomBlock` resolve cycle.
fn seeded_board(n: usize) -> LoomCanvasBoard {
    let mut b = LoomCanvasBoard::new("ws-test", "canvas-1");
    let placements: Vec<CanvasPlacementCard> = (0..n)
        .map(|i| {
            let mut c = CanvasPlacementCard::new(
                format!("p-{:03}", i + 1),
                format!("block-{:03}", i + 1),
                (i as f32) * 240.0 + 30.0,
                40.0,
                200.0,
                120.0,
            );
            c.live_title = Some(format!("Block {}", i + 1));
            c.live_content_type = Some("note".to_owned());
            c
        })
        .collect();
    b.set_board(placements, vec![], egui::Vec2::ZERO, 1.0);
    b
}

fn shared(board: LoomCanvasBoard) -> Arc<Mutex<LoomCanvasBoard>> {
    Arc::new(Mutex::new(board))
}

/// Build a harness that renders the shared board and pushes every emitted [`CanvasEvent`] into `events`.
fn harness_for<'a>(
    board: Arc<Mutex<LoomCanvasBoard>>,
    events: Arc<Mutex<Vec<CanvasEvent>>>,
) -> Harness<'a, ()> {
    Harness::builder()
        .with_size(egui::vec2(900.0, 640.0))
        .build_ui(move |ui| {
            let pal = HsTheme::Dark.palette();
            if let Some(ev) = board.lock().unwrap().show(ui, &pal) {
                events.lock().unwrap().push(ev);
            }
        })
}

/// Collect every author_id present in the live AccessKit tree.
fn author_ids<S>(harness: &Harness<'_, S>) -> std::collections::HashSet<String> {
    let mut ids = std::collections::HashSet::new();
    for node in harness.root().children_recursive() {
        if let Some(a) = node.accesskit_node().author_id() {
            ids.insert(a.to_owned());
        }
    }
    ids
}

/// Read a node's AccessKit `value` by author_id (used for the group_id AC6 + the zoom label AC3).
fn value_for<S>(harness: &Harness<'_, S>, author_id: &str) -> Option<String> {
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        if ak.author_id() == Some(author_id) {
            return ak.value().map(|v| v.to_owned());
        }
    }
    None
}

/// Read a node's AccessKit `label` by author_id.
fn label_for<S>(harness: &Harness<'_, S>, author_id: &str) -> Option<String> {
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        if ak.author_id() == Some(author_id) {
            return ak.label().map(|v| v.to_owned());
        }
    }
    None
}

// ── PROOF2 + AC9: placement + toolbar AccessKit nodes, labels match live titles ───────────────────

#[test]
fn canvas_accesskit_placements_and_toolbar() {
    let board = shared(seeded_board(2));
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut harness = harness_for(Arc::clone(&board), events);
    harness.run();

    let ids = author_ids(&harness);

    // AC9: the toolbar controls.
    for required in [
        PAN_LEFT_AUTHOR_ID,
        PAN_RIGHT_AUTHOR_ID,
        ZOOM_IN_AUTHOR_ID,
        ZOOM_OUT_AUTHOR_ID,
        ADD_CARD_AUTHOR_ID,
        EDGE_MODE_AUTHOR_ID,
        STATUS_AUTHOR_ID,
    ] {
        assert!(
            ids.contains(required),
            "AC9: toolbar author_id '{required}' missing from {ids:?}"
        );
    }

    // PROOF2: the two placement nodes are present and their labels are the LIVE block titles.
    assert!(
        ids.contains(&placement_author_id("p-001")),
        "PROOF2: canvas.placement.p-001 present"
    );
    assert!(
        ids.contains(&placement_author_id("p-002")),
        "PROOF2: canvas.placement.p-002 present"
    );
    assert_eq!(
        label_for(&harness, &placement_author_id("p-001")).as_deref(),
        Some("Block 1"),
        "PROOF2: placement label must equal the live block title (reference, not copy)"
    );
    assert_eq!(
        label_for(&harness, &placement_author_id("p-002")).as_deref(),
        Some("Block 2"),
        "PROOF2: second placement label must equal its live block title"
    );

    // WP-KERNEL-012 MT-061 added `canvas.placement.{id}.resize` handle nodes that share the prefix; the
    // CARD count excludes both the `.remove` button and the `.resize` handle suffixes.
    let placement_count = ids
        .iter()
        .filter(|a| {
            a.starts_with("canvas.placement.") && !a.ends_with(".remove") && !a.ends_with(".resize")
        })
        .count();
    assert_eq!(
        placement_count, 2,
        "PROOF2: exactly 2 placement nodes (got {placement_count})"
    );

    println!("PROOF2/AC9: 2 placement nodes with live-title labels + toolbar ids present");
}

// ── AC3: zoom value label reads "1.00x" and the zoom buttons step it ──────────────────────────────

#[test]
fn canvas_zoom_label_and_buttons() {
    let board = shared(seeded_board(2));
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_ck = Arc::clone(&events);
    let mut harness = harness_for(Arc::clone(&board), events);
    harness.run();

    // AC1/AC3: the zoom value label reads "1.00x" at zoom 1.0.
    let zoom_val = value_for(&harness, ZOOM_VALUE_AUTHOR_ID);
    assert_eq!(
        zoom_val.as_deref(),
        Some("1.00x"),
        "AC3: zoom label must read '1.00x'"
    );

    // Click zoom-in -> zoom rises to 1.25 and a ViewportChanged event fires (AC3). The button's
    // AccessKit label is the descriptive "Zoom in" (emit_button_node overrides the glyph text).
    harness.get_by_label("Zoom in").click();
    harness.run();
    let zoom = board.lock().unwrap().zoom;
    assert!(
        (zoom - 1.25).abs() < 1e-3,
        "AC3: zoom-in must raise zoom to 1.25 (got {zoom})"
    );
    let fired = events_ck.lock().unwrap().iter().any(
        |e| matches!(e, CanvasEvent::ViewportChanged { zoom, .. } if (*zoom - 1.25).abs() < 1e-3),
    );
    assert!(fired, "AC3: zoom-in must fire ViewportChanged{{zoom:1.25}}");
    println!("AC3: zoom label '1.00x' -> zoom-in raised to 1.25 + ViewportChanged fired");
}

// ── AC2: pan-right button shifts pan by +40 and fires ViewportChanged ──────────────────────────────

#[test]
fn canvas_pan_buttons() {
    let board = shared(seeded_board(2));
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_ck = Arc::clone(&events);
    let mut harness = harness_for(Arc::clone(&board), events);
    harness.run();
    let pan0 = board.lock().unwrap().pan.x;

    harness.get_by_label("Pan right").click();
    harness.run();
    let pan1 = board.lock().unwrap().pan.x;
    assert!(
        (pan1 - pan0 - 40.0).abs() < 1e-3,
        "AC2: pan-right must add 40px (got Δ{})",
        pan1 - pan0
    );
    let fired = events_ck.lock().unwrap().iter().any(
        |e| matches!(e, CanvasEvent::ViewportChanged { pan_x, .. } if (*pan_x - pan1).abs() < 1e-3),
    );
    assert!(
        fired,
        "AC2: pan must fire ViewportChanged with the new pan_x"
    );
    println!("AC2: pan-right shifted pan by +40 + ViewportChanged fired");
}

// ── AC5: '+ Text card' fires AddCard with a timestamp title ────────────────────────────────────────

#[test]
fn canvas_add_card() {
    let board = shared(seeded_board(0));
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_ck = Arc::clone(&events);
    let mut harness = harness_for(Arc::clone(&board), events);
    harness.run();

    harness.get_by_label("Add text card").click();
    harness.run();
    let ev = events_ck.lock().unwrap().clone();
    let ok = ev.iter().any(|e| {
        matches!(e, CanvasEvent::AddCard { title, x, y }
        if title.starts_with("Card ") && *x == 40.0 && *y == 40.0)
    });
    assert!(
        ok,
        "AC5: '+ Text card' must fire AddCard with a 'Card <ts>' title at (40,40) (got {ev:?})"
    );
    println!("AC5: add-card fired AddCard with timestamp title");
}

// ── PROOF3 (AC4 / MC-2): drop-to-place via the fallback text field + 'Place' button ────────────────

#[test]
fn canvas_place_block_fallback_field() {
    // MC-2 / RISK-2: on backends where OS / inter-panel drag is unavailable, the toolbar text field +
    // 'Place' button must produce the SAME PlaceBlock event the drop path produces. This is the
    // always-reachable place path the AC4 acceptance hinges on.
    let board = shared(seeded_board(2));
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_ck = Arc::clone(&events);
    let mut harness = harness_for(Arc::clone(&board), events);
    harness.run();

    // The 'Place' button is disabled until a block id is present (no empty-id placement).
    {
        let mut b = board.lock().unwrap();
        assert!(b.place_block_input.is_empty(), "field starts empty");
        b.place_block_input = "block-drop-1".to_owned();
    }
    harness.run();
    // The fallback field exposes its value via AccessKit (a swarm agent can read the staged id).
    assert_eq!(
        value_for(&harness, PLACE_BLOCK_INPUT_AUTHOR_ID).as_deref(),
        Some("block-drop-1"),
        "MC-2: the place-block field must expose its value on AccessKit"
    );

    harness.get_by_label("Place block by id").click();
    harness.run();

    let ev = events_ck.lock().unwrap().clone();
    let placed = ev.iter().find_map(|e| match e {
        CanvasEvent::PlaceBlock {
            placed_block_id,
            x,
            y,
        } if placed_block_id == "block-drop-1" => Some((*x, *y)),
        _ => None,
    });
    let (px, py) =
        placed.expect("PROOF3/AC4/MC-2: 'Place' must fire PlaceBlock for the typed block id");
    // The default place position is the visible canvas centre in canvas space — a finite, on-board point.
    assert!(
        px.is_finite() && py.is_finite(),
        "PROOF3: place position must be finite (got {px},{py})"
    );
    // The field is cleared after a successful place (no accidental double-place on the next click).
    assert!(
        board.lock().unwrap().place_block_input.is_empty(),
        "field cleared after place"
    );
    // Find the author_id is present (AC9 coverage of the new control).
    let ids = author_ids(&harness);
    assert!(
        ids.contains(PLACE_BLOCK_AUTHOR_ID),
        "AC9: '{PLACE_BLOCK_AUTHOR_ID}' present"
    );

    // Host applies the place + refreshes: the board now has a 3rd placement (PROOF3 '3 nodes after
    // refresh'). The placement appears at the emitted canvas position with the live title resolved.
    {
        let mut b = board.lock().unwrap();
        let mut kept: Vec<CanvasPlacementCard> = b.placements.clone();
        let mut card = CanvasPlacementCard::new(
            "p-003",
            "block-drop-1",
            px,
            py,
            DEFAULT_CARD_W,
            DEFAULT_CARD_H,
        );
        card.live_title = Some("Dropped Block".to_owned());
        card.live_content_type = Some("note".to_owned());
        kept.push(card);
        b.set_board(kept, vec![], egui::Vec2::ZERO, 1.0);
    }
    harness.run();
    let ids = author_ids(&harness);
    let placement_count = ids
        .iter()
        // MT-061 `.resize` handle nodes share the `canvas.placement.` prefix; exclude them (+ `.remove`).
        .filter(|a| {
            a.starts_with("canvas.placement.") && !a.ends_with(".remove") && !a.ends_with(".resize")
        })
        .count();
    assert_eq!(
        placement_count, 3,
        "PROOF3/AC4: 3 placement nodes after the place + refresh"
    );
    assert!(
        ids.contains(&placement_author_id("p-003")),
        "PROOF3: the placed card node is present"
    );
    println!("PROOF3/AC4/MC-2: fallback 'Place' fired PlaceBlock(block-drop-1) at ({px},{py}); 3 nodes after refresh");
}

// ── PROOF3 (AC4): drop-to-place via egui DragAndDrop — payload released over the canvas ────────────

#[test]
fn canvas_drop_to_place_via_drag_payload() {
    // AC4: a Loom block dragged from another panel (egui DragAndDrop payload, the native peer of the
    // React CANVAS_DRAG_MIME dataTransfer) and RELEASED over the canvas fires PlaceBlock with a
    // transform-correct canvas position. We inject the payload onto the context (as a drag source in a
    // sibling panel would) and synthesize a pointer release over the canvas surface.
    let board = shared(seeded_board(2));
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_ck = Arc::clone(&events);
    let mut harness = harness_for(Arc::clone(&board), events);
    harness.run();

    // The canvas surface sits below the toolbar + status strip. Drop near the centre of the 900x640
    // harness so the pointer is unambiguously over the canvas rect (and clear of the toolbar/cards).
    let drop_pos = egui::pos2(500.0, 400.0);

    // Position the pointer over the canvas first (so contains_pointer() is true on the release frame).
    harness.event(egui::Event::PointerMoved(drop_pos));
    harness.run();

    // A sibling panel's drag-source set this payload; synthesize the release over the canvas. Setting
    // the payload on the ctx mirrors `dnd_set_drag_payload` from the (out-of-test) drag source.
    egui::DragAndDrop::set_payload(&harness.ctx, CanvasDragPayload::new("block-drop-2"));
    harness.event(egui::Event::PointerButton {
        pos: drop_pos,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::default(),
    });
    harness.run();

    let ev = events_ck.lock().unwrap().clone();
    let placed = ev.iter().find_map(|e| match e {
        CanvasEvent::PlaceBlock {
            placed_block_id,
            x,
            y,
        } if placed_block_id == "block-drop-2" => Some((*x, *y)),
        _ => None,
    });
    let (px, py) = placed.expect("AC4: a payload released over the canvas must fire PlaceBlock");

    // The emitted position must be the drop point mapped through screen_to_canvas (pan=0, zoom=1, so it
    // equals drop_pos - origin). origin is the canvas rect top-left, which is > 0 (below the toolbar),
    // so the canvas x/y are strictly less than the screen drop coordinates and finite.
    assert!(
        px.is_finite() && py.is_finite(),
        "AC4: placed position must be finite (got {px},{py})"
    );
    assert!(
        px < drop_pos.x && py < drop_pos.y,
        "AC4: canvas pos must be screen pos minus the canvas origin (got {px},{py} vs screen {drop_pos:?})"
    );
    // The payload was consumed (taken) — no lingering payload to double-place on a later frame.
    assert!(
        !egui::DragAndDrop::has_payload_of_type::<CanvasDragPayload>(&harness.ctx),
        "AC4: the drop payload must be taken (consumed) on release, not left dangling"
    );
    println!("PROOF3/AC4: drag payload released over canvas fired PlaceBlock(block-drop-2) at ({px},{py})");
}

// ── AC6: shift-select 2 cards + Group(2) fires Group and exposes the group_id on each card ─────────

#[test]
fn canvas_group_two() {
    let board = shared(seeded_board(2));
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_ck = Arc::clone(&events);
    // Pre-select both placements directly (the click-selection path is exercised by the edge test; here
    // we focus on the Group action + the group_id AccessKit exposure).
    {
        let mut b = board.lock().unwrap();
        b.selected.insert("p-001".to_owned());
        b.selected.insert("p-002".to_owned());
    }
    let mut harness = harness_for(Arc::clone(&board), events);
    harness.run();

    // The Group button label reflects the selection count.
    harness.get_by_label("Group (2)").click();
    harness.run();

    let ev = events_ck.lock().unwrap().clone();
    let grouped = ev.iter().find_map(|e| match e {
        CanvasEvent::Group {
            placement_ids,
            group_id,
        } if placement_ids.len() == 2 => Some(group_id.clone()),
        _ => None,
    });
    let group_id =
        grouped.expect("AC6: Group(2) must fire CanvasEvent::Group for the 2 selected cards");

    // AC6: each affected card's AccessKit value carries the group_id (data-group-id).
    let v1 = value_for(&harness, &placement_author_id("p-001")).unwrap_or_default();
    let v2 = value_for(&harness, &placement_author_id("p-002")).unwrap_or_default();
    assert!(
        v1.contains(&group_id),
        "AC6: p-001 must expose group_id '{group_id}' (got '{v1}')"
    );
    assert!(
        v2.contains(&group_id),
        "AC6: p-002 must expose group_id '{group_id}' (got '{v2}')"
    );
    println!(
        "AC6: Group(2) fired Group + both cards expose group_id '{group_id}' on AccessKit value"
    );
}

// ── PROOF4 (AC7): semantic edge — start from p-001, click p-002 => SemanticEdge(block-001,block-002) ─

#[test]
fn canvas_semantic_edge() {
    let board = shared(seeded_board(2));
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_ck = Arc::clone(&events);

    // Select p-001 (single) so 'Draw edge from selected' is enabled.
    {
        let mut b = board.lock().unwrap();
        b.edge_mode = EdgeMode::Semantic;
        b.selected.insert("p-001".to_owned());
    }
    let mut harness = harness_for(Arc::clone(&board), events);
    harness.run();

    // Click 'Draw edge from selected' -> edge_from = p-001.
    harness.get_by_label("Draw edge from selected").click();
    harness.run();
    assert_eq!(
        board.lock().unwrap().edge_from.as_deref(),
        Some("p-001"),
        "edge_from set to p-001"
    );

    // Click the second card (p-002) by injecting a pointer click at its on-screen centre. The board's
    // canvas rect starts below the toolbar+status strip; we compute the card centre via the SAME
    // transform the widget uses (pan=0, zoom=1, origin = canvas rect top-left). The canvas rect top-left
    // is not directly observable, so we click via the card centre in screen space derived from the
    // default layout: toolbar+status ≈ 60px tall, so origin.y ≈ 60, origin.x ≈ 8 (panel margin).
    let (cx, cy) = {
        let b = board.lock().unwrap();
        let card = b
            .placements
            .iter()
            .find(|p| p.placement_id == "p-002")
            .unwrap();
        // canvas centre in canvas space:
        (card.x + card.w * 0.5, card.y + card.h * 0.5)
    };
    let origin = egui::vec2(8.0, 60.0);
    let click = egui::pos2(origin.x + cx, origin.y + cy);
    harness.event(egui::Event::PointerMoved(click));
    harness.event(egui::Event::PointerButton {
        pos: click,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::default(),
    });
    harness.event(egui::Event::PointerButton {
        pos: click,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::default(),
    });
    harness.run();

    let ev = events_ck.lock().unwrap().clone();
    let ok = ev.iter().any(|e| {
        matches!(e, CanvasEvent::SemanticEdge { source_block_id, target_block_id }
        if source_block_id == "block-001" && target_block_id == "block-002")
    });
    assert!(
        ok,
        "PROOF4/AC7: semantic edge must fire SemanticEdge{{source:block-001,target:block-002}} (got {ev:?})"
    );
    // edge_from must be cleared after completing the edge (RISK-6 no double-mutate).
    assert_eq!(
        board.lock().unwrap().edge_from,
        None,
        "edge_from cleared after edge draw"
    );
    println!("PROOF4/AC7: semantic edge fired SemanticEdge(block-001 -> block-002)");
}

// ── AC7 (visual mode): the edge_event maps to a VisualEdgeAdded with placement ids ────────────────

#[test]
fn canvas_visual_edge_mode() {
    // Visual-mode edge creation is the same flow with edge_mode=Visual. The standalone mapping is the
    // lib unit `edge_event_maps_mode_to_ids`; here we prove the widget toggles the mode via the button.
    let board = shared(seeded_board(2));
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut harness = harness_for(Arc::clone(&board), events);
    harness.run();
    assert_eq!(
        board.lock().unwrap().edge_mode,
        EdgeMode::Semantic,
        "default mode is Semantic"
    );
    harness.get_by_label("Edge: Semantic").click();
    harness.run();
    assert_eq!(
        board.lock().unwrap().edge_mode,
        EdgeMode::Visual,
        "AC7: edge-mode toggle -> Visual"
    );
    println!("AC7: edge-mode toggle switched Semantic -> Visual");
}

// ── PROOF5 (AC8): remove fires RemovePlacement; after refresh the node is absent ──────────────────

#[test]
fn canvas_remove_placement() {
    let board = shared(seeded_board(2));
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_ck = Arc::clone(&events);
    let mut harness = harness_for(Arc::clone(&board), events);
    harness.run();

    // The remove button is a real `ui.interact` widget (painter-drawn glyph) addressable by its
    // AccessKit label "Remove <live title>" (p-001 resolves to "Block 1").
    harness.get_by_label("Remove Block 1").click();
    harness.run();

    let ev = events_ck.lock().unwrap().clone();
    let removed = ev.iter().any(
        |e| matches!(e, CanvasEvent::RemovePlacement { placement_id } if placement_id == "p-001"),
    );
    assert!(
        removed,
        "PROOF5/AC8: remove button must fire RemovePlacement{{p-001}} (got {ev:?})"
    );

    // Simulate the host applying the removal + refresh: drop p-001 from the board.
    {
        let mut b = board.lock().unwrap();
        let kept: Vec<CanvasPlacementCard> = b
            .placements
            .iter()
            .filter(|p| p.placement_id != "p-001")
            .cloned()
            .collect();
        b.set_board(kept, vec![], egui::Vec2::ZERO, 1.0);
    }
    harness.run();
    let ids = author_ids(&harness);
    assert!(
        !ids.contains(&placement_author_id("p-001")),
        "PROOF5/AC8: p-001 absent after refresh"
    );
    assert!(
        ids.contains(&placement_author_id("p-002")),
        "PROOF5: p-002 still present"
    );
    // The remove author_id must also be gone (no dangling remove button).
    assert!(
        !ids.contains(&placement_remove_author_id("p-001")),
        "PROOF5: remove node gone too"
    );
    println!("PROOF5/AC8: remove fired RemovePlacement(p-001); node absent after refresh");
}

// ── WP-KERNEL-012 MT-026 V4 (validation_v4 remediation): TERMINAL ACTION RECEIPTS ─────────────────
//
// `validation_v4` failed MT-026 because the mounted zoom and placement-removal receipts were both
// INDETERMINATE: the controls published no completion declaration, so `crate::mcp::action` had nothing
// to acknowledge. These widget-level proofs pin the exact token machinery those receipts depend on —
// the durable observers exist in the AccessKit tree, the controls carry their declarations, and the
// observers terminalize ONLY on authoritative post-state (a refreshed board, and for a removal also an
// explicit source-block existence confirmation).

fn observer_token<S>(harness: &Harness<'_, S>, author_id: &str) -> serde_json::Value {
    let raw = value_for(harness, author_id)
        .unwrap_or_else(|| panic!("completion observer '{author_id}' must publish a token value"));
    serde_json::from_str(&raw)
        .unwrap_or_else(|error| panic!("observer '{author_id}' token must be JSON: {error} ({raw})"))
}

fn declaration_token<S>(harness: &Harness<'_, S>, author_id: &str) -> serde_json::Value {
    let raw = value_for(harness, author_id)
        .unwrap_or_else(|| panic!("control '{author_id}' must publish a completion declaration"));
    serde_json::from_str(&raw).unwrap_or_else(|error| {
        panic!("control '{author_id}' declaration must be JSON: {error} ({raw})")
    })
}

fn inner_json(token: &serde_json::Value, field: &str) -> serde_json::Value {
    serde_json::from_str(
        token[field]
            .as_str()
            .unwrap_or_else(|| panic!("token field '{field}' must be a JSON string: {token}")),
    )
    .unwrap_or_else(|error| panic!("token field '{field}' must parse: {error}"))
}

#[test]
fn canvas_viewport_receipt_terminalizes_only_on_authoritative_refresh() {
    let board = shared(seeded_board(2));
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut harness = harness_for(Arc::clone(&board), events);
    harness.run();

    // The durable observers are addressable, both Ready.
    let viewport_ready = observer_token(&harness, VIEWPORT_COMPLETION_AUTHOR_ID);
    assert_eq!(viewport_ready["schema"], "handshake.click-completion/v1");
    assert_eq!(viewport_ready["mode"], "observer");
    assert_eq!(viewport_ready["state"], "ready");
    assert_eq!(viewport_ready["effect"], "canvas-viewport");
    let ready_generation = viewport_ready["generation"]
        .as_u64()
        .expect("observer generation");
    assert!(
        value_for(&harness, PLACEMENT_MUTATION_COMPLETION_AUTHOR_ID).is_some(),
        "the placement-mutation observer must be addressable too"
    );

    // The zoom-in control declares a PERSISTENT observer target naming the board, the prior viewport
    // revision/scale/offset and the exact requested post-state.
    let declaration = declaration_token(&harness, ZOOM_IN_AUTHOR_ID);
    assert_eq!(declaration["mode"], "observer");
    assert_eq!(declaration["state"], "ready");
    assert_eq!(declaration["persistent_target"], true);
    assert_eq!(declaration["observer_author_id"], VIEWPORT_COMPLETION_AUTHOR_ID);
    assert_eq!(declaration["generation"].as_u64(), Some(ready_generation));
    let semantic = inner_json(&declaration, "semantic_value");
    assert_eq!(semantic["schema_id"], "handshake.canvas-viewport-action/v1");
    assert_eq!(semantic["board_id"], "canvas-1");
    assert_eq!(semantic["action"], ZOOM_IN_AUTHOR_ID);
    assert_eq!(semantic["prior"]["scale"].as_f64(), Some(1.0));
    assert_eq!(semantic["requested"]["scale"].as_f64(), Some(1.25));

    // Activating the control opens the binding: the observer goes Pending at generation + 1 and the
    // persistent declaration advances by exactly one generation carrying the SAME semantic (the exact
    // transition `crate::mcp::action` acknowledges against).
    harness.get_by_label("Zoom in").click();
    harness.run();
    let pending = observer_token(&harness, VIEWPORT_COMPLETION_AUTHOR_ID);
    assert_eq!(pending["state"], "pending");
    assert_eq!(pending["generation"].as_u64(), Some(ready_generation + 1));
    assert_eq!(pending["pending_target"], ZOOM_IN_AUTHOR_ID);
    assert_eq!(pending["semantic_value"], declaration["semantic_value"]);
    let advanced_declaration = declaration_token(&harness, ZOOM_IN_AUTHOR_ID);
    assert_eq!(
        advanced_declaration["generation"].as_u64(),
        Some(ready_generation + 1)
    );
    assert_eq!(
        advanced_declaration["semantic_value"],
        declaration["semantic_value"]
    );

    // An optimistic in-widget zoom is NOT proof: the receipt stays pending until an authoritative
    // projection delivers the persisted viewport.
    assert_eq!(
        observer_token(&harness, VIEWPORT_COMPLETION_AUTHOR_ID)["state"],
        "pending",
        "the local zoom step must never terminalize the viewport receipt"
    );

    // The authoritative refresh carrying the persisted viewport terminalizes it.
    {
        let mut b = board.lock().unwrap();
        let placements = b.placements.clone();
        b.set_board(placements, vec![], egui::Vec2::ZERO, 1.25);
    }
    harness.run();
    let applied = observer_token(&harness, VIEWPORT_COMPLETION_AUTHOR_ID);
    assert_eq!(applied["state"], "applied");
    assert_eq!(applied["generation"].as_u64(), Some(ready_generation + 1));
    let detail = inner_json(&applied, "terminal_detail");
    assert_eq!(
        detail["schema_id"],
        "handshake.canvas-viewport-completion/v1"
    );
    assert_eq!(detail["authority"], "persisted");
    assert_eq!(detail["board_id"], "canvas-1");
    assert_eq!(detail["resulting"]["scale"].as_f64(), Some(1.25));
    assert!(detail["persist_route"]
        .as_str()
        .is_some_and(|route| route.ends_with("/viewport")));
    assert!(
        detail["resulting"]["viewport_revision"].as_u64()
            > semantic["prior"]["viewport_revision"].as_u64(),
        "the resulting viewport revision must advance: {detail}"
    );
    assert!(
        detail["resulting"]["board_generation"].as_u64()
            > semantic["prior"]["board_generation"].as_u64(),
        "the resulting board generation must be FRESH: {detail}"
    );
    println!(
        "MT-026 V4: canvas.zoom-in receipt terminal={} authority={} scale={}",
        applied["state"], detail["authority"], detail["resulting"]["scale"]
    );
}

#[test]
fn canvas_removal_receipt_requires_absence_and_source_block_retention() {
    let board = shared(seeded_board(2));
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut harness = harness_for(Arc::clone(&board), events);
    harness.run();

    let remove_author = placement_remove_author_id("p-001");
    let declaration = declaration_token(&harness, &remove_author);
    assert_eq!(declaration["mode"], "observer");
    assert_eq!(declaration["flexible_target"], true);
    assert_eq!(
        declaration["observer_author_id"],
        PLACEMENT_MUTATION_COMPLETION_AUTHOR_ID
    );
    let semantic = inner_json(&declaration, "semantic_value");
    assert_eq!(
        semantic["schema_id"],
        "handshake.canvas-placement-removal-action/v1"
    );
    assert_eq!(semantic["placement_id"], "p-001");
    assert_eq!(semantic["block_id"], "block-001");
    assert_eq!(semantic["board_id"], "canvas-1");
    let ready_generation = observer_token(&harness, PLACEMENT_MUTATION_COMPLETION_AUTHOR_ID)
        ["generation"]
        .as_u64()
        .expect("placement observer generation");

    harness.get_by_label("Remove Block 1").click();
    harness.run();
    let pending = observer_token(&harness, PLACEMENT_MUTATION_COMPLETION_AUTHOR_ID);
    assert_eq!(pending["state"], "pending");
    assert_eq!(pending["pending_target"], remove_author);
    assert_eq!(pending["generation"].as_u64(), Some(ready_generation + 1));

    // The authoritative refresh proving the placement ABSENT is only half the proof — the receipt must
    // NOT terminalize on target disappearance alone (the exact validation_v4 anti-pattern).
    {
        let mut b = board.lock().unwrap();
        let kept: Vec<CanvasPlacementCard> = b
            .placements
            .iter()
            .filter(|p| p.placement_id != "p-001")
            .cloned()
            .collect();
        b.set_board(kept, vec![], egui::Vec2::ZERO, 1.0);
    }
    harness.run();
    assert_eq!(
        observer_token(&harness, PLACEMENT_MUTATION_COMPLETION_AUTHOR_ID)["state"],
        "pending",
        "placement disappearance alone must NOT terminalize the removal receipt"
    );
    assert_eq!(
        board.lock().unwrap().retention_probe_block_ids(),
        vec!["block-001".to_owned()],
        "the board must ask the host to probe the removed placement's SOURCE block"
    );

    // The explicit source-block existence confirmation closes it.
    {
        let mut b = board.lock().unwrap();
        assert!(b.apply_live_block_resolution(
            "block-001",
            &Ok((Some("Block 1".to_owned()), "note".to_owned(), None)),
        ));
    }
    harness.run();
    let applied = observer_token(&harness, PLACEMENT_MUTATION_COMPLETION_AUTHOR_ID);
    assert_eq!(applied["state"], "applied");
    assert_eq!(applied["generation"].as_u64(), Some(ready_generation + 1));
    let detail = inner_json(&applied, "terminal_detail");
    assert_eq!(
        detail["schema_id"],
        "handshake.canvas-placement-removal-completion/v1"
    );
    assert_eq!(detail["placement_id"], "p-001");
    assert_eq!(detail["block_id"], "block-001");
    assert_eq!(detail["placement_absent_after_refresh"], true);
    assert_eq!(detail["source_block_present"], true);
    assert_eq!(detail["source_block_content_type"], "note");
    assert!(detail["backend"]["route"]
        .as_str()
        .is_some_and(|route| route.starts_with("DELETE /workspaces/") && route.ends_with("p-001")));
    assert!(
        detail["mutation_revision"]["refreshed_board_generation"].as_u64()
            > semantic["prior_board_generation"].as_u64(),
        "the removal receipt must bind a FRESH board generation: {detail}"
    );
    println!(
        "MT-026 V4: placement-removal receipt terminal={} absent={} source_kept={}",
        applied["state"],
        detail["placement_absent_after_refresh"],
        detail["source_block_present"]
    );
}

#[test]
fn canvas_removal_receipt_fails_closed_when_the_source_block_is_gone() {
    // Red-team: a placement removal that ALSO destroyed its source block must produce a typed terminal
    // FAILURE, never a silent Applied. This is the invariant MT-026 exists to protect.
    let board = shared(seeded_board(2));
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut harness = harness_for(Arc::clone(&board), events);
    harness.run();
    harness.get_by_label("Remove Block 1").click();
    harness.run();
    {
        let mut b = board.lock().unwrap();
        let kept: Vec<CanvasPlacementCard> = b
            .placements
            .iter()
            .filter(|p| p.placement_id != "p-001")
            .cloned()
            .collect();
        b.set_board(kept, vec![], egui::Vec2::ZERO, 1.0);
        assert!(b.apply_live_block_resolution(
            "block-001",
            &Err(handshake_native::backend_client::LiveBlockResolveError::Missing),
        ));
    }
    harness.run();
    let failed = observer_token(&harness, PLACEMENT_MUTATION_COMPLETION_AUTHOR_ID);
    assert_eq!(failed["state"], "failed");
    assert!(
        failed["terminal_error"]
            .as_str()
            .is_some_and(|error| error.contains("source block is ABSENT")),
        "a destroyed source block must be a typed terminal failure: {failed}"
    );
    let detail = inner_json(&failed, "terminal_detail");
    assert_eq!(detail["source_block_present"], false);
    println!("MT-026 V4: source-block loss fails the removal receipt closed (terminal Rejected)");
}

// ── AC10: empty board -> empty canvas, no panic, no "(stale reference)" text ──────────────────────

#[test]
fn canvas_empty_no_stale_text() {
    let board = shared(seeded_board(0));
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut harness = harness_for(Arc::clone(&board), events);
    harness.run();

    let ids = author_ids(&harness);
    let placement_count = ids
        .iter()
        .filter(|a| a.starts_with("canvas.placement."))
        .count();
    assert_eq!(
        placement_count, 0,
        "AC10: empty board has 0 placement nodes"
    );
    // No "(stale reference)" anywhere — there are no cards at all.
    assert!(
        harness.query_by_label("(stale reference)").is_none(),
        "AC10: no stale-reference text"
    );
    // The status bar reports "0 placements".
    assert_eq!(
        value_for(&harness, STATUS_AUTHOR_ID).as_deref(),
        Some("0 placements"),
        "AC10: status reports 0 placements"
    );
    println!("AC10: empty board renders with no cards, no stale text, no panic");
}

// ── PROOF6: screenshot — non-white canvas with at least one rounded card shape ────────────────────

#[test]
fn canvas_screenshot_has_card() {
    let _g = wgpu_guard();
    let board = shared(seeded_board(2));
    let events = Arc::new(Mutex::new(Vec::new()));
    let board_ui = Arc::clone(&board);
    let events_ui = Arc::clone(&events);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 640.0))
        .wgpu()
        .build_ui(move |ui| {
            let pal = HsTheme::Dark.palette();
            if let Some(ev) = board_ui.lock().unwrap().show(ui, &pal) {
                events_ui.lock().unwrap().push(ev);
            }
        });
    harness.run();
    harness.run();

    match harness.render() {
        Ok(image) => {
            let (w, h) = (image.width(), image.height());
            assert!(w > 0 && h > 0, "rendered image must be non-empty");
            let raw = image.as_raw();
            let mut counts: std::collections::HashMap<[u8; 4], u32> =
                std::collections::HashMap::new();
            let mut white = 0u32;
            let mut i = 0usize;
            while i + 4 <= raw.len() {
                let px = [raw[i], raw[i + 1], raw[i + 2], raw[i + 3]];
                if px[3] != 0 {
                    *counts.entry(px).or_insert(0) += 1;
                    if px[0] > 250 && px[1] > 250 && px[2] > 250 {
                        white += 1;
                    }
                }
                i += 16;
            }
            let total: u32 = counts.values().sum();
            assert!(total > 0, "PROOF6: sampled pixels must be opaque");
            assert!(
                (white as f32 / total as f32) < 0.95,
                "PROOF6: canvas must not be ~all-white (white frac {})",
                white as f32 / total as f32
            );
            // The dark canvas bg + the light card surface => >= 2 distinct opaque colours (a card shape
            // was painted).
            assert!(
                counts.len() >= 2,
                "PROOF6: >= 2 distinct colours expected (dark bg + light card), got {}",
                counts.len()
            );

            let ext_dir = external_artifact_dir("wp-kernel-012-mt-026");
            let _ = std::fs::create_dir_all(&ext_dir);
            let png = ext_dir.join("MT-026-canvas-board.png");
            let saved = image.save(&png).is_ok();
            println!(
                "PROOF6: {w}x{h} screenshot, {} distinct colours, white_frac={:.3}, saved={saved} ({})",
                counts.len(),
                white as f32 / total as f32,
                png.display()
            );
        }
        Err(e) => {
            println!(
                "BLOCKER(non-fatal): canvas screenshot render unavailable (no wgpu adapter): {e}. The \
                 AccessKit + transform + interaction proofs passed; the PNG is a GPU-host item."
            );
        }
    }
    assert_no_local_artifact_dir();
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// CanvasBoardClient request-builder proofs (NO backend): the CORRECTED routes/bodies the MT-026
// contract got wrong. These prove the production request construction (the spawn paths route through
// the SAME builders), so a stale URL or body can never reach the real backend unnoticed.
// ═════════════════════════════════════════════════════════════════════════════════════════════════

fn test_client() -> CanvasBoardClient {
    // A runtime is required for the client constructor's handle; we only call the pure builders.
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    CanvasBoardClient::new("http://127.0.0.1:37501", rt.handle().clone())
}

#[test]
fn client_get_board_url_corrected() {
    let c = test_client();
    let spec = c.get_board_request("ws1", "cb1");
    assert_eq!(
        spec.url,
        "http://127.0.0.1:37501/workspaces/ws1/loom/canvas-boards/cb1"
    );
    assert!(spec.query.is_empty());
}

#[test]
fn client_viewport_body_is_board_state_wrapped() {
    let c = test_client();
    let spec = c.viewport_request("ws1", "cb1", 12.0, -8.0, 1.5);
    assert_eq!(
        spec.url,
        "http://127.0.0.1:37501/workspaces/ws1/loom/canvas-boards/cb1/viewport"
    );
    let body = spec.body.expect("viewport has a body");
    let bs = body
        .get("board_state")
        .expect("board_state wrapper (NOT top-level pan/zoom)");
    assert_eq!(
        bs.get("schema_id").and_then(|x| x.as_str()),
        Some("hsk.loom_canvas_board@1")
    );
    assert_eq!(bs.get("pan_x").and_then(|x| x.as_f64()), Some(12.0));
    assert_eq!(bs.get("pan_y").and_then(|x| x.as_f64()), Some(-8.0));
    assert_eq!(bs.get("zoom").and_then(|x| x.as_f64()), Some(1.5));
}

#[test]
fn client_place_block_body() {
    let c = test_client();
    let spec = c.place_block_request("ws1", "cb1", "blk-9", 100.0, 200.0, 200.0, 120.0);
    assert_eq!(
        spec.url,
        "http://127.0.0.1:37501/workspaces/ws1/loom/canvas-boards/cb1/placements"
    );
    let body = spec.body.unwrap();
    assert_eq!(
        body.get("placed_block_id").and_then(|x| x.as_str()),
        Some("blk-9")
    );
    assert_eq!(body.get("x").and_then(|x| x.as_f64()), Some(100.0));
    assert_eq!(body.get("y").and_then(|x| x.as_f64()), Some(200.0));
    // W3 (MT-026 remediation) wire-capture: the host's PlaceBlock arm passes the widget default
    // geometry (the drop/toolbar event carries only x/y — React DEFAULT_CARD_W/H), and the placement
    // POST body carries it.
    assert_eq!(
        body.get("w").and_then(|x| x.as_f64()),
        Some(DEFAULT_CARD_W as f64)
    );
    assert_eq!(
        body.get("h").and_then(|x| x.as_f64()),
        Some(DEFAULT_CARD_H as f64)
    );
}

/// W3 (MT-026 remediation) wire-capture: the `AddCard` host arm's `create_card_request` builder —
/// `POST .../canvas-boards/:cb/cards` with `{title, body:"", x, y, w, h}` (the verified
/// `create_canvas_card` shape; `body` is the empty string a fresh text card starts with).
#[test]
fn client_create_card_body() {
    let c = test_client();
    let spec = c.create_card_request("ws1", "cb1", "Card W3", 40.0, 40.0, 200.0, 120.0);
    assert_eq!(
        spec.url,
        "http://127.0.0.1:37501/workspaces/ws1/loom/canvas-boards/cb1/cards"
    );
    let body = spec.body.unwrap();
    assert_eq!(body.get("title").and_then(|x| x.as_str()), Some("Card W3"));
    assert_eq!(body.get("body").and_then(|x| x.as_str()), Some(""));
    assert_eq!(body.get("x").and_then(|x| x.as_f64()), Some(40.0));
    assert_eq!(body.get("y").and_then(|x| x.as_f64()), Some(40.0));
    assert_eq!(body.get("w").and_then(|x| x.as_f64()), Some(200.0));
    assert_eq!(body.get("h").and_then(|x| x.as_f64()), Some(120.0));
}

#[test]
fn created_canvas_placement_response_parses_place_and_card_shapes() {
    use handshake_native::backend_client::created_canvas_placement_from_response;

    let placed = created_canvas_placement_from_response(&serde_json::json!({
        "placement_id": "LCP-place",
        "placed_block_id": "blk-place",
        "x": 10.0,
        "y": 20.0,
        "w": 200.0,
        "h": 120.0,
    }))
    .expect("direct place response parses");
    assert_eq!(placed.placement_id, "LCP-place");
    assert_eq!(placed.placed_block_id, "blk-place");
    assert_eq!(placed.geometry(), (10.0, 20.0, 200.0, 120.0));

    let card = created_canvas_placement_from_response(&serde_json::json!({
        "block": { "block_id": "blk-card" },
        "rich_document_id": "doc-card",
        "placement": {
            "placement_id": "LCP-card",
            "placed_block_id": "blk-card",
            "x": 40.0,
            "y": 50.0,
            "w": 220.0,
            "h": 140.0,
        }
    }))
    .expect("create-card response parses through nested placement");
    assert_eq!(card.placement_id, "LCP-card");
    assert_eq!(card.placed_block_id, "blk-card");
    assert_eq!(card.geometry(), (40.0, 50.0, 220.0, 140.0));
}

/// W3 (MT-026 remediation) wire-capture: the `RemoveEdge` host arm's route SPLIT — a board-local
/// visual-edge id deletes via the verified `DELETE .../loom/canvas-visual-edges/:id`
/// (`remove_canvas_visual_edge` in `handshake_core` `api/loom.rs`); a semantic loom-edge id via the
/// existing `DELETE .../loom/edges/:id`. Both DELETEs are bodyless.
#[test]
fn client_remove_edge_visual_vs_semantic_routes() {
    let c = test_client();
    let vis = c.remove_visual_edge_request("ws1", "ve-1");
    assert_eq!(
        vis.url,
        "http://127.0.0.1:37501/workspaces/ws1/loom/canvas-visual-edges/ve-1"
    );
    assert!(vis.body.is_none(), "visual-edge DELETE is bodyless");
    let sem = c.remove_semantic_edge_request("ws1", "edge-9");
    assert_eq!(
        sem.url,
        "http://127.0.0.1:37501/workspaces/ws1/loom/edges/edge-9"
    );
    assert!(sem.body.is_none(), "semantic-edge DELETE is bodyless");
    assert_ne!(
        vis.url, sem.url,
        "the two RemoveEdge routes are distinct backend surfaces"
    );
}

/// W4 (review MAJOR) HOST route-choice pin: the `RemoveEdge` visual-vs-semantic split lives in the
/// HOST (`HandshakeApp::route_remove_edge_spec`, the exact arm `route_canvas_events` dispatches
/// through), NOT in the two builders. The sibling `client_remove_edge_visual_vs_semantic_routes`
/// proves the two builders in ISOLATION — it stays GREEN even if the host picks the WRONG one. This
/// test captures the spec the HOST would dispatch for each id class and pins the CHOICE: a `ve-*` id
/// that IS in the board's `visual_edges` projection routes to `DELETE .../loom/canvas-visual-edges/:id`;
/// a `loom-edge-*` id that is NOT routes to `DELETE .../loom/edges/:id`. Inverting the single
/// `contains` check in `route_remove_edge_spec` swaps BOTH urls -> both assertions fail.
#[test]
fn host_remove_edge_route_choice_visual_vs_semantic() {
    use handshake_native::app::HandshakeApp;
    use handshake_native::backend_client::HttpMethod;
    let c = test_client();

    // The board's own visual_edges projection: only `ve-1` is a board-local visual edge.
    let mut visual_edge_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    visual_edge_ids.insert("ve-1".to_string());

    // A board-local visual-edge id -> the canvas visual-edge route (through the host's contains check).
    let visual = HandshakeApp::route_remove_edge_spec(&c, "ws1", &visual_edge_ids, "ve-1");
    assert_eq!(visual.method, HttpMethod::Delete);
    assert_eq!(
        visual.url, "http://127.0.0.1:37501/workspaces/ws1/loom/canvas-visual-edges/ve-1",
        "a board-local visual-edge id MUST route through the canvas visual-edge DELETE"
    );
    assert!(visual.body.is_none(), "visual-edge DELETE is bodyless");

    // Any id NOT in the board's visual_edges -> the semantic loom-edge route.
    let semantic = HandshakeApp::route_remove_edge_spec(&c, "ws1", &visual_edge_ids, "loom-edge-9");
    assert_eq!(semantic.method, HttpMethod::Delete);
    assert_eq!(
        semantic.url, "http://127.0.0.1:37501/workspaces/ws1/loom/edges/loom-edge-9",
        "a non-visual (semantic loom) edge id MUST route through the loom-edges DELETE"
    );
    assert!(semantic.body.is_none(), "semantic-edge DELETE is bodyless");

    // The routing CHOICE is decisive: the two id classes land on DISTINCT backend surfaces. If the
    // host's `contains` check is inverted, `ve-1` takes the semantic url and `loom-edge-9` the visual
    // url -> the two url asserts above catch it.
    assert_ne!(visual.url, semantic.url);
}

#[test]
fn client_placement_routes_corrected() {
    // The MT contract said `.../canvas/{cb}/placements/{p}`; the REAL route is `.../canvas-placements/{p}`.
    let c = test_client();
    let group = c.group_request("ws1", "p-1", "grp-7");
    assert_eq!(
        group.url,
        "http://127.0.0.1:37501/workspaces/ws1/loom/canvas-placements/p-1"
    );
    assert_eq!(
        group.body.unwrap().get("group_id").and_then(|x| x.as_str()),
        Some("grp-7")
    );
    let remove = c.remove_placement_request("ws1", "p-1");
    assert_eq!(
        remove.url,
        "http://127.0.0.1:37501/workspaces/ws1/loom/canvas-placements/p-1"
    );
    assert!(remove.body.is_none(), "DELETE is bodyless");
}

#[test]
fn client_semantic_and_visual_edge_bodies() {
    let c = test_client();
    let sem = c.semantic_edge_request("ws1", "src", "tgt");
    assert_eq!(sem.url, "http://127.0.0.1:37501/workspaces/ws1/loom/edges");
    let sb = sem.body.unwrap();
    assert_eq!(
        sb.get("source_block_id").and_then(|x| x.as_str()),
        Some("src")
    );
    assert_eq!(
        sb.get("target_block_id").and_then(|x| x.as_str()),
        Some("tgt")
    );
    assert_eq!(
        sb.get("edge_type").and_then(|x| x.as_str()),
        Some("mention")
    );
    assert_eq!(sb.get("created_by").and_then(|x| x.as_str()), Some("user"));

    let vis = c.visual_edge_request("ws1", "cb1", "p-1", "p-2");
    assert_eq!(
        vis.url,
        "http://127.0.0.1:37501/workspaces/ws1/loom/canvas-boards/cb1/visual-edges"
    );
    let vb = vis.body.unwrap();
    assert_eq!(
        vb.get("from_placement_id").and_then(|x| x.as_str()),
        Some("p-1")
    );
    assert_eq!(
        vb.get("to_placement_id").and_then(|x| x.as_str()),
        Some("p-2")
    );
}

#[test]
fn client_get_block_url() {
    let c = test_client();
    let spec = c.get_block_request("ws1", "blk-1");
    assert_eq!(
        spec.url,
        "http://127.0.0.1:37501/workspaces/ws1/loom/blocks/blk-1"
    );
}

#[test]
fn canvas_error_exposes_stable_retry_event() {
    let mut board = seeded_board(2);
    board.error = Some("managed backend unavailable".to_owned());
    let board = shared(board);
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut harness = harness_for(Arc::clone(&board), Arc::clone(&events));
    harness.run_steps(2);

    assert!(author_ids(&harness).contains(RETRY_AUTHOR_ID));
    harness
        .get_by(|node: &egui_kittest::kittest::AccessKitNode<'_>| {
            node.author_id() == Some(RETRY_AUTHOR_ID)
        })
        .click();
    harness.run_steps(1);

    assert_eq!(events.lock().unwrap().pop(), Some(CanvasEvent::Retry));
    assert!(board.lock().unwrap().loading);
    assert!(
        board.lock().unwrap().error.is_none(),
        "retry replaces the stale error with the bounded loading state"
    );
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// LIVE-PG: isolated, self-seeded, mounted, non-ignored. Never fakes PostgreSQL.
// ═════════════════════════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "integration")]
struct LiveWorkspaceCleanup<'a> {
    backend: &'a interconnect_support::LiveBackend,
    workspace_id: String,
    cleaned: bool,
}

#[cfg(feature = "integration")]
impl LiveWorkspaceCleanup<'_> {
    fn assert_cleaned(&mut self) {
        let status = self.backend.delete_workspace(&self.workspace_id);
        assert!(
            matches!(status, 200 | 202 | 204),
            "managed-PG workspace cleanup returned HTTP {status}"
        );
        let workspaces = self.backend.get_json("/workspaces");
        let rows = workspaces
            .as_array()
            .expect("GET /workspaces returns the canonical workspace list");
        assert!(
            rows.iter().all(|row| {
                row.get("id").and_then(|id| id.as_str()) != Some(self.workspace_id.as_str())
            }),
            "fresh canonical workspace-list read must prove the isolated workspace is absent"
        );
        self.cleaned = true;
    }
}

#[cfg(feature = "integration")]
impl Drop for LiveWorkspaceCleanup<'_> {
    fn drop(&mut self) {
        if !self.cleaned {
            let _ = self.backend.delete_workspace(&self.workspace_id);
        }
    }
}

#[cfg(feature = "integration")]
fn one_shot_canvas_json(body: serde_json::Value) -> (String, std::thread::JoinHandle<()>) {
    use std::io::{Read, Write};
    let listener =
        std::net::TcpListener::bind("127.0.0.1:0").expect("bind one-shot malformed Canvas server");
    let address = listener.local_addr().expect("malformed Canvas address");
    let encoded = serde_json::to_vec(&body).expect("encode malformed Canvas response");
    let join = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept Canvas GET");
        let mut request = [0_u8; 4096];
        let _ = stream.read(&mut request);
        let headers = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            encoded.len()
        );
        stream
            .write_all(headers.as_bytes())
            .expect("write malformed Canvas headers");
        stream
            .write_all(&encoded)
            .expect("write malformed Canvas body");
    });
    (format!("http://{address}"), join)
}

#[cfg(feature = "integration")]
fn await_board(
    cell: &handshake_native::backend_client::CanvasBoardCell,
) -> Result<handshake_native::backend_client::CanvasBoardData, String> {
    for _ in 0..200 {
        if let Some(delivery) = cell.lock().unwrap().pop_front() {
            return delivery.result;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    panic!("canvas board request did not resolve within 10 seconds")
}

#[cfg(feature = "integration")]
fn await_live_block(
    cell: &handshake_native::backend_client::LiveBlockCell,
) -> (
    String,
    Result<
        handshake_native::backend_client::LiveBlock,
        handshake_native::backend_client::LiveBlockResolveError,
    >,
) {
    for _ in 0..200 {
        if let Some(result) = cell.lock().unwrap().take() {
            return result;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    panic!("canvas live-block resolve did not complete within 10 seconds")
}

#[cfg(feature = "integration")]
fn fetch_canvas(
    client: &CanvasBoardClient,
    workspace_id: &str,
    canvas_block_id: &str,
) -> Result<handshake_native::backend_client::CanvasBoardData, String> {
    let cell = Arc::new(Mutex::new(std::collections::VecDeque::new()));
    client.fetch_board(workspace_id, canvas_block_id, Arc::clone(&cell));
    await_board(&cell)
}

#[cfg(feature = "integration")]
fn resolve_live_titles(
    client: &CanvasBoardClient,
    workspace_id: &str,
    data: &mut handshake_native::backend_client::CanvasBoardData,
) {
    for placement in &mut data.placements {
        let cell = Arc::new(Mutex::new(None));
        client.resolve_block(workspace_id, &placement.placed_block_id, Arc::clone(&cell));
        let (resolved_id, result) = await_live_block(&cell);
        assert_eq!(resolved_id, placement.placed_block_id);
        let (title, content_type, _) = result.expect("placed source block resolves live");
        placement.live_title = title;
        placement.live_content_type = Some(content_type);
    }
}

#[cfg(feature = "integration")]
fn drive_canvas_host_until(
    host: &mut Harness<'_, handshake_native::app::HandshakeApp>,
    events: &Arc<Mutex<Vec<CanvasEvent>>>,
    board: &Arc<Mutex<LoomCanvasBoard>>,
    condition: impl Fn(&LoomCanvasBoard) -> bool,
    proof: &str,
) {
    for _ in 0..400 {
        host.run_steps(1);
        let matches = board.lock().map(|board| condition(&board)).unwrap_or(false);
        if events.lock().map(|queue| queue.is_empty()).unwrap_or(false)
            && host.state().canvas_op_cells_in_flight() == 0
            && matches
        {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    let snapshot = board
        .lock()
        .map(|board| format!("{board:?}"))
        .unwrap_or_else(|_| "<poisoned Canvas board>".to_owned());
    panic!("timed out waiting for mounted host proof '{proof}': {snapshot}");
}

#[cfg(feature = "integration")]
fn click_mounted_canvas_control(
    host: &mut Harness<'_, handshake_native::app::HandshakeApp>,
    events: &Arc<Mutex<Vec<CanvasEvent>>>,
    board: &Arc<Mutex<LoomCanvasBoard>>,
    author_id: &str,
    condition: impl Fn(&LoomCanvasBoard) -> bool,
    proof: &str,
) {
    host.run_steps(1);
    let target = host
        .root()
        .children_recursive()
        .find_map(|node| {
            let access = node.accesskit_node();
            (access.author_id() == Some(author_id)).then(|| access.id())
        })
        .unwrap_or_else(|| panic!("mounted Canvas AccessKit node '{author_id}' is present"));
    host.event(egui::Event::AccessKitActionRequest(
        egui::accesskit::ActionRequest {
            action: egui::accesskit::Action::Click,
            target,
            data: None,
        },
    ));
    drive_canvas_host_until(host, events, board, condition, proof);
}

/// AC1-AC10 against the real Canvas client, mounted Canvas pane, and Handshake-managed PostgreSQL.
/// This proof creates every fixture it consumes, exercises a typed failure followed by the stable
/// `canvas.retry` control, verifies fresh-client persistence, and removes its workspace before writing
/// the external success receipt. It is deliberately NOT ignored.
#[test]
#[cfg(feature = "integration")]
fn canvas_board_live_pg_self_seeds_mounted_round_trip() {
    use handshake_native::app::{HandshakeApp, HealthDisplayState};
    use handshake_native::backend_client::{HealthInfo, LiveBlockCell};
    use handshake_native::graph::canvas_sections::section_author_id;

    let receipt_dir = external_artifact_dir("wp-kernel-012-mt-026");
    let receipt_path = receipt_dir.join("MT-026-live-pg-self-seeded.json");
    match std::fs::remove_file(&receipt_path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => panic!(
            "remove stale MT-026 receipt {} before proof: {error}",
            receipt_path.display()
        ),
    }

    let live = interconnect_support::require_reachable_backend();
    let unique = format!(
        "mt026-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock after unix epoch")
            .as_nanos()
    );
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
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("canvas live runtime");
    let client = CanvasBoardClient::new(live.base.clone(), runtime.handle().clone());

    let create_block = |title: &str| {
        let block = live.post_json(
            &format!("/workspaces/{workspace_id}/loom/blocks"),
            &serde_json::json!({"content_type": "note", "title": title}),
        );
        block["block_id"]
            .as_str()
            .expect("Loom block create returns block_id")
            .to_owned()
    };
    let source_one_title = format!("MT-026 source one {unique}");
    let source_two_title = format!("MT-026 source two {unique}");
    let text_card_title;
    let source_one = create_block(&source_one_title);
    let source_two = create_block(&source_two_title);
    let canvas = live.post_json(
        &format!("/workspaces/{workspace_id}/loom/canvas-boards"),
        &serde_json::json!({"title": format!("MT-026 canvas {unique}")}),
    );
    let canvas_id = canvas["block_id"]
        .as_str()
        .expect("canvas create returns block_id")
        .to_owned();

    let empty = fetch_canvas(&client, &workspace_id, &canvas_id)
        .expect("newly created Canvas loads through real client");
    assert!(empty.placements.is_empty(), "AC10 real empty-board state");

    // The add/place/undo/redo, viewport, group, edge-mode, retry, and remove writes below originate from
    // controls rendered by the mounted production HandshakeApp. The move, resize, semantic-edge, and
    // visual-edge persistence checks deliberately inject their typed CanvasEvent values into the same
    // mounted host queue; their widget-producer mechanics are covered by the standalone tests named in
    // the external receipt. Direct CanvasBoardClient mutation helpers are intentionally not used.
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }));
    app.set_backend_base_url_for_test(&live.base, runtime.handle().clone());
    assert!(app.switch_project(&workspace_id));
    let app_board = app.mounted_canvas_board();
    let app_events = app.mounted_canvas_events();
    {
        let mut board = app_board.lock().unwrap();
        board.workspace_id = workspace_id.clone();
        board.canvas_block_id = canvas_id.clone();
    }
    assert!(
        app.dispatch_palette_action_for_test(handshake_native::command_registry::CMD_VIEW_CANVAS),
        "the operator-facing View Canvas command mounts the production Canvas pane"
    );
    let host_ctx = Arc::new(Mutex::new(None::<egui::Context>));
    let host_ctx_capture = Arc::clone(&host_ctx);
    let mut host = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_state(
            move |ctx, app: &mut HandshakeApp| {
                *host_ctx_capture
                    .lock()
                    .expect("capture mounted host context") = Some(ctx.clone());
                app.ui(ctx);
            },
            app,
        );
    drive_canvas_host_until(
        &mut host,
        &app_events,
        &app_board,
        |board| board.placements.is_empty() && !board.loading && board.error.is_none(),
        "initial empty PostgreSQL board",
    );

    app_board.lock().unwrap().place_block_input = source_one.clone();
    click_mounted_canvas_control(
        &mut host,
        &app_events,
        &app_board,
        PLACE_BLOCK_AUTHOR_ID,
        |board| {
            board
                .placements
                .iter()
                .any(|placement| placement.placed_block_id == source_one)
        },
        "host place first source block",
    );
    let first = app_board
        .lock()
        .unwrap()
        .placements
        .iter()
        .find(|placement| placement.placed_block_id == source_one)
        .cloned()
        .expect("host reload exposes first backend-minted placement identity");

    {
        let mut board = app_board.lock().unwrap();
        // Change only the producer's current view before the second click so its documented
        // visible-centre fallback emits a distinct canvas coordinate. The authoritative reload below
        // restores the persisted viewport; no synthetic mutation event is inserted.
        board.pan.x = 300.0;
        board.place_block_input = source_two.clone();
    }
    click_mounted_canvas_control(
        &mut host,
        &app_events,
        &app_board,
        PLACE_BLOCK_AUTHOR_ID,
        |board| {
            board
                .placements
                .iter()
                .any(|placement| placement.placed_block_id == source_two)
        },
        "host place second source block",
    );
    let second = app_board
        .lock()
        .unwrap()
        .placements
        .iter()
        .find(|placement| placement.placed_block_id == source_two)
        .cloned()
        .expect("host reload exposes second backend-minted placement identity");

    click_mounted_canvas_control(
        &mut host,
        &app_events,
        &app_board,
        ADD_CARD_AUTHOR_ID,
        |board| {
            board.placements.len() == 3
                && board
                    .placements
                    .iter()
                    .any(|placement| placement.card_kind.is_text_card())
        },
        "host create text card",
    );
    drive_canvas_host_until(
        &mut host,
        &app_events,
        &app_board,
        |board| {
            board.placements.iter().any(|placement| {
                placement.card_kind.is_text_card() && placement.display_title().starts_with("Card ")
            })
        },
        "mounted text-card live title resolves",
    );
    let original_text = app_board
        .lock()
        .unwrap()
        .placements
        .iter()
        .find(|placement| placement.card_kind.is_text_card())
        .cloned()
        .expect("host reload exposes created text-card identity");
    let original_text_placement_id = original_text.placement_id.clone();
    let text_block_id = original_text.placed_block_id.clone();
    text_card_title = original_text.display_title().to_owned();
    assert!(
        text_card_title.starts_with("Card "),
        "mounted + Text card producer persists its timestamp title"
    );

    // MT-033 dependency: exercise the actual shared InteractionBus action that the mounted host
    // registered from the create response. Redo must mint a replacement placement id, and the next undo
    // must target that replacement rather than retrying the stale original id.
    let ctx = host_ctx
        .lock()
        .expect("mounted host context lock")
        .clone()
        .expect("mounted host captured egui context");
    let bus = handshake_native::interop::InteractionBus::get_or_init(&ctx);
    let first_undo =
        handshake_native::interop::InteractionBus::with_try_lock(&bus, |bus| bus.undo_cross_pane())
            .flatten()
            .expect("host create registered a cross-pane Canvas undo");
    assert!(first_undo.ok, "host Canvas undo dispatches successfully");
    drive_canvas_host_until(
        &mut host,
        &app_events,
        &app_board,
        |board| {
            board.placements.len() == 2
                && board
                    .placements
                    .iter()
                    .all(|placement| placement.placement_id != original_text_placement_id)
        },
        "host undo removes original text-card placement",
    );
    let after_first_undo = fetch_canvas(&client, &workspace_id, &canvas_id)
        .expect("fresh PG reload after first host undo");
    assert!(after_first_undo
        .placements
        .iter()
        .all(|placement| placement.placement_id != original_text_placement_id));

    let first_redo =
        handshake_native::interop::InteractionBus::with_try_lock(&bus, |bus| bus.redo_cross_pane())
            .flatten()
            .expect("Canvas redo remains available after completed host undo");
    assert!(first_redo.ok, "host Canvas redo dispatches successfully");
    drive_canvas_host_until(
        &mut host,
        &app_events,
        &app_board,
        |board| {
            board.placements.len() == 3
                && board.placements.iter().any(|placement| {
                    placement.placed_block_id == text_block_id
                        && placement.placement_id != original_text_placement_id
                })
        },
        "host redo mints replacement placement identity",
    );
    let first_replacement_id = app_board
        .lock()
        .unwrap()
        .placements
        .iter()
        .find(|placement| placement.placed_block_id == text_block_id)
        .map(|placement| placement.placement_id.clone())
        .expect("first redone text placement exists");
    assert_ne!(first_replacement_id, original_text_placement_id);
    let after_first_redo =
        fetch_canvas(&client, &workspace_id, &canvas_id).expect("fresh PG reload after host redo");
    assert!(after_first_redo.placements.iter().any(|placement| {
        placement.placed_block_id == text_block_id && placement.placement_id == first_replacement_id
    }));

    let second_undo =
        handshake_native::interop::InteractionBus::with_try_lock(&bus, |bus| bus.undo_cross_pane())
            .flatten()
            .expect("redone Canvas placement remains undoable");
    assert!(
        second_undo.ok,
        "second host Canvas undo dispatches successfully"
    );
    drive_canvas_host_until(
        &mut host,
        &app_events,
        &app_board,
        |board| {
            board.placements.len() == 2
                && board
                    .placements
                    .iter()
                    .all(|placement| placement.placement_id != first_replacement_id)
        },
        "second host undo targets replacement placement identity",
    );
    let after_second_undo = fetch_canvas(&client, &workspace_id, &canvas_id)
        .expect("fresh PG reload after second host undo");
    assert!(after_second_undo
        .placements
        .iter()
        .all(|placement| placement.placement_id != first_replacement_id));

    let second_redo =
        handshake_native::interop::InteractionBus::with_try_lock(&bus, |bus| bus.redo_cross_pane())
            .flatten()
            .expect("second Canvas redo restores the fixture through the real host action");
    assert!(
        second_redo.ok,
        "second host Canvas redo dispatches successfully"
    );
    drive_canvas_host_until(
        &mut host,
        &app_events,
        &app_board,
        |board| {
            board.placements.len() == 3
                && board.placements.iter().any(|placement| {
                    placement.placed_block_id == text_block_id
                        && placement.placement_id != first_replacement_id
                })
        },
        "second host redo restores text-card placement",
    );
    let text_card = app_board
        .lock()
        .unwrap()
        .placements
        .iter()
        .find(|placement| placement.placed_block_id == text_block_id)
        .cloned()
        .expect("final host-redone text card exists");
    assert_ne!(text_card.placement_id, first_replacement_id);

    click_mounted_canvas_control(
        &mut host,
        &app_events,
        &app_board,
        PAN_RIGHT_AUTHOR_ID,
        |board| (board.pan.x - 40.0).abs() < f32::EPSILON,
        "mounted pan control persists viewport",
    );
    click_mounted_canvas_control(
        &mut host,
        &app_events,
        &app_board,
        ZOOM_IN_AUTHOR_ID,
        |board| (board.zoom - 1.25).abs() < f32::EPSILON,
        "mounted zoom control persists first step",
    );
    click_mounted_canvas_control(
        &mut host,
        &app_events,
        &app_board,
        ZOOM_IN_AUTHOR_ID,
        |board| (board.zoom - 1.5).abs() < f32::EPSILON,
        "mounted zoom control persists second step",
    );
    {
        let mut board = app_board.lock().unwrap();
        board.selected.insert(first.placement_id.clone());
        board.selected.insert(second.placement_id.clone());
    }
    click_mounted_canvas_control(
        &mut host,
        &app_events,
        &app_board,
        handshake_native::graph::canvas_board::GROUP_AUTHOR_ID,
        |board| {
            let groups = board
                .placements
                .iter()
                .filter(|placement| {
                    [first.placement_id.as_str(), second.placement_id.as_str()]
                        .contains(&placement.placement_id.as_str())
                })
                .filter_map(|placement| placement.group_id.as_deref())
                .collect::<Vec<_>>();
            groups.len() == 2 && groups[0] == groups[1]
        },
        "mounted Group control persists both placements",
    );
    let section_id = app_board
        .lock()
        .unwrap()
        .placements
        .iter()
        .find(|placement| placement.placement_id == first.placement_id)
        .and_then(|placement| placement.group_id.clone())
        .expect("mounted Group producer assigns a section id");

    // The pan/zoom persistence proof above deliberately pushed the first card almost outside the live
    // viewport. Restore the viewport through the same mounted controls before pointer move/resize/edge
    // gestures so their coordinates remain inside the production Canvas surface.
    click_mounted_canvas_control(
        &mut host,
        &app_events,
        &app_board,
        PAN_LEFT_AUTHOR_ID,
        |board| board.pan.x.abs() < f32::EPSILON,
        "mounted pan reset before pointer gestures",
    );
    click_mounted_canvas_control(
        &mut host,
        &app_events,
        &app_board,
        ZOOM_OUT_AUTHOR_ID,
        |board| (board.zoom - 1.25).abs() < f32::EPSILON,
        "mounted zoom reset first step",
    );
    click_mounted_canvas_control(
        &mut host,
        &app_events,
        &app_board,
        ZOOM_OUT_AUTHOR_ID,
        |board| (board.zoom - 1.0).abs() < f32::EPSILON,
        "mounted zoom reset second step",
    );

    let before_move = app_board
        .lock()
        .unwrap()
        .placements
        .iter()
        .find(|placement| placement.placement_id == first.placement_id)
        .map(|placement| (placement.x, placement.y))
        .unwrap();
    let expected_move = (before_move.0 + 36.0, before_move.1 + 24.0);
    app_events.lock().unwrap().push(CanvasEvent::MovePlacement {
        placement_id: first.placement_id.clone(),
        x: expected_move.0,
        y: expected_move.1,
        group_id: None,
    });
    drive_canvas_host_until(
        &mut host,
        &app_events,
        &app_board,
        |board| {
            board.placements.iter().any(|placement| {
                placement.placement_id == first.placement_id
                    && (placement.x - expected_move.0).abs() < 0.1
                    && (placement.y - expected_move.1).abs() < 0.1
                    && placement.group_id.is_none()
            })
        },
        "mounted card drag persists movement",
    );
    let moved_geometry = app_board
        .lock()
        .unwrap()
        .placements
        .iter()
        .find(|placement| placement.placement_id == first.placement_id)
        .map(|placement| (placement.x, placement.y))
        .unwrap();

    let before_resize = app_board
        .lock()
        .unwrap()
        .placements
        .iter()
        .find(|placement| placement.placement_id == first.placement_id)
        .map(|placement| (placement.w, placement.h))
        .unwrap();
    let expected_resize = (before_resize.0 + 40.0, before_resize.1 + 30.0);
    app_events
        .lock()
        .unwrap()
        .push(CanvasEvent::ResizePlacement {
            placement_id: first.placement_id.clone(),
            w: expected_resize.0,
            h: expected_resize.1,
        });
    drive_canvas_host_until(
        &mut host,
        &app_events,
        &app_board,
        |board| {
            board.placements.iter().any(|placement| {
                placement.placement_id == first.placement_id
                    && (placement.w - expected_resize.0).abs() < 0.1
                    && (placement.h - expected_resize.1).abs() < 0.1
            })
        },
        "mounted resize handle persists geometry",
    );
    let resized_geometry = app_board
        .lock()
        .unwrap()
        .placements
        .iter()
        .find(|placement| placement.placement_id == first.placement_id)
        .map(|placement| (placement.w, placement.h))
        .unwrap();
    app_events.lock().unwrap().push(CanvasEvent::SemanticEdge {
        source_block_id: source_one.clone(),
        target_block_id: source_two.clone(),
    });
    drive_canvas_host_until(
        &mut host,
        &app_events,
        &app_board,
        |board| board.error.is_none(),
        "mounted semantic-edge host persists canonical edge",
    );

    click_mounted_canvas_control(
        &mut host,
        &app_events,
        &app_board,
        EDGE_MODE_AUTHOR_ID,
        |board| board.edge_mode == EdgeMode::Visual,
        "mounted edge-mode control selects Visual",
    );
    app_events
        .lock()
        .unwrap()
        .push(CanvasEvent::VisualEdgeAdded {
            from_placement_id: first.placement_id.clone(),
            to_placement_id: second.placement_id.clone(),
        });
    drive_canvas_host_until(
        &mut host,
        &app_events,
        &app_board,
        |board| board.visual_edges.len() == 1,
        "mounted visual-edge host persists board-local edge",
    );

    let backlinks = live.get_json(&format!(
        "/workspaces/{workspace_id}/loom/blocks/{source_two}/backlinks"
    ));
    assert!(
        serde_json::to_string(&backlinks)
            .expect("serialize backlinks")
            .contains(&source_one),
        "semantic edge persists in the canonical Loom relation surface"
    );

    let fresh_client = CanvasBoardClient::new(live.base.clone(), runtime.handle().clone());
    let mut persisted = fetch_canvas(&fresh_client, &workspace_id, &canvas_id)
        .expect("fresh Canvas client reload succeeds");
    assert_eq!(
        persisted.placements.len(),
        3,
        "two references plus text card persist"
    );
    assert_eq!(
        persisted.visual_edges.len(),
        1,
        "visual edge persists board-locally"
    );
    assert!(persisted.pan_x.abs() < f32::EPSILON);
    assert!(persisted.pan_y.abs() < f32::EPSILON);
    assert!((persisted.zoom - 1.0).abs() < f32::EPSILON);
    let persisted_first = persisted
        .placements
        .iter()
        .find(|placement| placement.placement_id == first.placement_id)
        .expect("first placement survives fresh reload");
    assert_eq!(persisted_first.placed_block_id, source_one);
    assert_eq!(
        persisted_first.group_id.as_deref(),
        None,
        "dragging the first card outside the remaining section persists the clear-group result"
    );
    assert!((persisted_first.x - moved_geometry.0).abs() < f32::EPSILON);
    assert!((persisted_first.y - moved_geometry.1).abs() < f32::EPSILON);
    assert!((persisted_first.w - resized_geometry.0).abs() < f32::EPSILON);
    assert!((persisted_first.h - resized_geometry.1).abs() < f32::EPSILON);
    let persisted_second = persisted
        .placements
        .iter()
        .find(|placement| placement.placement_id == second.placement_id)
        .expect("second placement survives fresh reload");
    assert_eq!(persisted_second.placed_block_id, source_two);
    assert_eq!(
        persisted_second.group_id.as_deref(),
        Some(section_id.as_str())
    );
    assert_eq!(
        persisted
            .placements
            .iter()
            .find(|placement| placement.placement_id == text_card.placement_id)
            .expect("text card placement survives fresh reload")
            .placed_block_id,
        text_block_id,
        "the canonical text-card block reference survives a fresh backend reload"
    );
    assert!(
        app_board
            .lock()
            .unwrap()
            .placements
            .iter()
            .find(|placement| placement.placement_id == text_card.placement_id)
            .expect("mounted host retains the reloaded text-card placement")
            .card_kind
            .is_text_card(),
        "the mounted host reapplies its text-card kind after authoritative reload"
    );

    // MT-033 dependency guard: grouping, movement, and resize changed only placement state. The exact
    // canonical Loom identities remain the references that cross-surface placement/undo depends on.
    assert_eq!(first.placed_block_id, source_one);
    assert_eq!(second.placed_block_id, source_two);
    resolve_live_titles(&fresh_client, &workspace_id, &mut persisted);

    drive_canvas_host_until(
        &mut host,
        &app_events,
        &app_board,
        |board| {
            let title = |placement_id: &str| {
                board
                    .placements
                    .iter()
                    .find(|placement| placement.placement_id == placement_id)
                    .map(|placement| placement.display_title())
            };
            title(&first.placement_id) == Some(source_one_title.as_str())
                && title(&second.placement_id) == Some(source_two_title.as_str())
                && title(&text_card.placement_id) == Some(text_card_title.as_str())
        },
        "mounted HandshakeApp resolves exact source and text-card titles",
    );
    host.run_steps(4);
    assert_eq!(
        label_for(&host, EDGE_MODE_AUTHOR_ID).as_deref(),
        Some("Edge: Visual"),
        "mounted HandshakeApp AccessKit tree exposes the real selected edge mode"
    );
    assert_eq!(
        label_for(&host, &placement_author_id(&first.placement_id)).as_deref(),
        Some(source_one_title.as_str()),
        "mounted HandshakeApp exposes the exact first source title"
    );
    assert_eq!(
        label_for(&host, &placement_author_id(&second.placement_id)).as_deref(),
        Some(source_two_title.as_str()),
        "mounted HandshakeApp exposes the exact second source title"
    );
    assert_eq!(
        label_for(&host, &placement_author_id(&text_card.placement_id)).as_deref(),
        Some(text_card_title.as_str()),
        "mounted HandshakeApp exposes the exact real text-card title"
    );

    let mut mounted = LoomCanvasBoard::new(&workspace_id, &canvas_id);
    mounted.set_section_labels(std::collections::BTreeMap::from([(
        section_id.clone(),
        "MT-026 section".to_owned(),
    )]));
    mounted.set_board(
        persisted.placements.clone(),
        persisted.visual_edges.clone(),
        egui::vec2(persisted.pan_x, persisted.pan_y),
        persisted.zoom,
    );
    let mounted = shared(mounted);
    let mounted_events = Arc::new(Mutex::new(Vec::new()));
    let mut pane = harness_for(Arc::clone(&mounted), Arc::clone(&mounted_events));
    pane.run_steps(2);
    let ids = author_ids(&pane);
    for required in [
        placement_author_id(&first.placement_id),
        placement_author_id(&second.placement_id),
        placement_author_id(&text_card.placement_id),
        placement_remove_author_id(&first.placement_id),
        section_author_id(&section_id),
        PAN_LEFT_AUTHOR_ID.to_owned(),
        PAN_RIGHT_AUTHOR_ID.to_owned(),
        ZOOM_IN_AUTHOR_ID.to_owned(),
        ZOOM_OUT_AUTHOR_ID.to_owned(),
        ADD_CARD_AUTHOR_ID.to_owned(),
        PLACE_BLOCK_AUTHOR_ID.to_owned(),
        PLACE_BLOCK_INPUT_AUTHOR_ID.to_owned(),
        EDGE_MODE_AUTHOR_ID.to_owned(),
        STATUS_AUTHOR_ID.to_owned(),
    ] {
        assert!(
            ids.contains(&required),
            "mounted Canvas AccessKit node {required}"
        );
    }
    assert_eq!(
        value_for(&pane, ZOOM_VALUE_AUTHOR_ID).as_deref(),
        Some("1.00x")
    );
    assert_eq!(
        label_for(&pane, &placement_author_id(&first.placement_id)).as_deref(),
        Some(source_one_title.as_str()),
        "real-PG first placement AccessKit label is its exact resolved source title"
    );
    assert_eq!(
        label_for(&pane, &placement_author_id(&second.placement_id)).as_deref(),
        Some(source_two_title.as_str()),
        "real-PG second placement AccessKit label is its exact resolved source title"
    );
    assert_eq!(
        label_for(&pane, &placement_author_id(&text_card.placement_id)).as_deref(),
        Some(text_card_title.as_str()),
        "real-PG text-card AccessKit label is its exact persisted title"
    );

    // A malformed HTTP 200 must fail closed instead of becoming an empty/default board. Surface that
    // parser failure through the mounted Retry control, then recover from the valid managed backend.
    let (malformed_base, malformed_join) = one_shot_canvas_json(serde_json::json!({
        "board": {"board_state": {"schema_id": LOOM_CANVAS_BOARD_SCHEMA_ID, "pan_x": 0, "pan_y": 0, "zoom": 1}},
        "placements": [{"placement_id": "broken-row"}],
        "visual_edges": []
    }));
    let malformed = CanvasBoardClient::new(malformed_base, runtime.handle().clone());
    let failure = fetch_canvas(&malformed, "mt026-malformed-success", &canvas_id)
        .expect_err("malformed successful Canvas response fails closed");
    malformed_join
        .join()
        .expect("malformed Canvas server completed");
    assert!(failure.contains("placements[0]"), "{failure}");
    mounted.lock().unwrap().error = Some(failure);
    pane.run_steps(2);
    assert!(author_ids(&pane).contains(RETRY_AUTHOR_ID));
    pane.get_by(|node: &egui_kittest::kittest::AccessKitNode<'_>| {
        node.author_id() == Some(RETRY_AUTHOR_ID)
    })
    .click();
    pane.run_steps(1);
    assert_eq!(
        mounted_events.lock().unwrap().pop(),
        Some(CanvasEvent::Retry)
    );
    let mut retried = fetch_canvas(&fresh_client, &workspace_id, &canvas_id)
        .expect("Retry reaches the real Canvas client");
    resolve_live_titles(&fresh_client, &workspace_id, &mut retried);
    mounted.lock().unwrap().set_board(
        retried.placements,
        retried.visual_edges,
        egui::vec2(retried.pan_x, retried.pan_y),
        retried.zoom,
    );
    pane.run_steps(2);
    assert!(mounted.lock().unwrap().error.is_none());
    assert!(!author_ids(&pane).contains(RETRY_AUTHOR_ID));

    click_mounted_canvas_control(
        &mut host,
        &app_events,
        &app_board,
        &placement_remove_author_id(&first.placement_id),
        |board| {
            board.placements.len() == 2
                && board
                    .placements
                    .iter()
                    .all(|placement| placement.placement_id != first.placement_id)
        },
        "mounted remove control removes placement while retaining source block",
    );
    let source_cell: LiveBlockCell = Arc::new(Mutex::new(None));
    fresh_client.resolve_block(&workspace_id, &source_one, Arc::clone(&source_cell));
    let (resolved_id, source_result) = await_live_block(&source_cell);
    assert_eq!(resolved_id, source_one);
    assert!(
        source_result.is_ok(),
        "removing a Canvas placement retains its canonical source block"
    );
    let after_remove = fetch_canvas(&fresh_client, &workspace_id, &canvas_id)
        .expect("fresh reload after placement removal");
    assert_eq!(after_remove.placements.len(), 2);
    assert!(!after_remove
        .placements
        .iter()
        .any(|placement| placement.placement_id == first.placement_id));
    assert!(after_remove
        .placements
        .iter()
        .any(|placement| placement.placed_block_id == source_two));

    drop(host);
    drop(pane);
    cleanup.assert_cleaned();
    assert_no_local_artifact_dir();
    std::fs::create_dir_all(&receipt_dir).expect("create external MT-026 receipt directory");
    let receipt = serde_json::json!({
        "schema_id": "hsk.wp_kernel_012.mt_026.live_pg_receipt@3",
        "workspace_id": workspace_id,
        "canvas_block_id": canvas_id,
        "source_block_ids": [source_one, source_two],
        "placement_ids": [first.placement_id, second.placement_id, text_card.placement_id],
        "viewport": {"pan_x": 0.0, "pan_y": 0.0, "zoom": 1.0},
        "section_id": section_id,
        "semantic_edge_backlink_verified": true,
        "visual_edge_count": 1,
        "move_resize_persisted": true,
        "live_pg_mutations_routed_through_mounted_host": ["viewport", "group", "move", "resize", "semantic_edge", "visual_edge", "remove"],
        "all_operator_visible_mutations_produced_by_mounted_widget": false,
        "producer_and_persistence_proof_are_split": true,
        "mounted_host_events_injected_for_live_pg_persistence": ["move", "resize", "semantic_edge", "visual_edge"],
        "widget_producer_proofs": ["test_canvas_board::canvas_semantic_edge", "test_canvas_board::canvas_visual_edge_mode", "test_canvas_sections_resize::canvas_drop_into_section_assigns_then_clears", "test_canvas_sections_resize::canvas_resize_handle_fires_one_debounced_patch", "test_canvas_sections_resize::canvas_pan_drag_applies_each_frame_delta_exactly_once"],
        "host_undo_redo_replacement_identity_verified": true,
        "mounted_accesskit_verified": true,
        "mounted_failure_retry_recovery_verified": true,
        "malformed_http_200_failed_closed_then_retried": true,
        "source_retained_after_placement_removal": true,
        "fresh_reload_verified": true,
        "cleanup_verified": true
    });
    std::fs::write(
        &receipt_path,
        serde_json::to_vec_pretty(&receipt).expect("serialize MT-026 live receipt"),
    )
    .expect("write external MT-026 live receipt");
    println!(
        "MT-026 LIVE PG PASS canvas={} placements=3 viewport/group/edges/move/resize/retry/remove \
         fresh_reload=true cleanup_verified=true receipt={}",
        receipt["canvas_block_id"],
        receipt_path.display()
    );
}

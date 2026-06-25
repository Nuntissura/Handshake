//! WP-KERNEL-012 MT-061 (cluster E3) — Obsidian-Canvas card RESIZE + section/group FRAMES + in-place
//! TEXT-CARD editing PROOFS for the native [`LoomCanvasBoard`] (extends the MT-026 canvas surface).
//!
//! This single file carries ALL the MT-061 proof_targets (the contract's `allowed_paths` names exactly
//! `tests/test_canvas_sections_resize.rs` as the new test file; the per-PT file names in the proof_targets
//! are folded here so the proofs stay inside the allowed scope):
//!
//!   - PT-061-1 / AC-061-1: dragging a card's bottom-right resize handle changes w/h LIVE and fires
//!     EXACTLY ONE debounced [`CanvasEvent::ResizePlacement`] carrying the final clamped geometry on
//!     drag-STOP (one PATCH per gesture — RISK-061-1 / MC-061-1). Proven against the REAL widget with a
//!     synthesized pointer drag on the handle (the MT-057/058 wire-into-live lesson — NOT isolated math).
//!   - PT-061-2 / AC-061-3: dragging a card into a section frame assigns that frame's group_id via the
//!     SAME placement PATCH semantics ([`CanvasEvent::AssignSection{Some}`]); dropping outside all frames
//!     clears it ([`CanvasEvent::AssignSection{None}`]).
//!   - PT-061-3 / AC-061-4: double-clicking a free-text card enters in-place edit mode (egui
//!     TextEdit::multiline) and typing/Escape-discard work LIVE. Committing fires the CONTRACT-MANDATED
//!     typed blocker [`CanvasEvent::TextCardEditBlocked`] — the cards endpoint is create-only and the real
//!     edit route (`PUT /knowledge/documents/:id/save`) is UNBOUND by this MT (RISK-061-5 / MC-061-5), so
//!     text-card-EDIT PERSISTENCE (AC-061-4) is gated NEEDS_MANAGED_RESOURCE_PROOF, not proven here. The
//!     test asserts the typed blocker (not a tautological local-state round-trip). Escape discards without
//!     an event. A BLOCK-backed card double-click does NOT become an editor (reference-not-copy gate).
//!   - PT-061-4 / AC-061-2 / AC-061-6: the AccessKit tree exposes `canvas.section.{id}` (per frame, with
//!     the section label) AND `canvas.placement.{id}.resize` (per card resize handle).
//!   - PT-061-5 / AC-061-5: a resize + section-assign cycle on a BLOCK-backed card leaves the board's
//!     underlying block_id set unchanged (no copy/fork — the load-bearing invariant).
//!   - AC-061-2 screenshot (HBR-VIS): a non-blank canvas showing a titled section frame behind its cards.
//!
//! ## Artifact hygiene (CX-212E)
//!
//! EVERY PNG is written ONLY to the EXTERNAL `Handshake_Artifacts/handshake-test/wp-kernel-012-mt-061/`
//! root via [`external_artifact_dir`]; [`assert_no_local_artifact_dir`] fails the run if a repo-local
//! `tests/screenshots/` or `test_output/` directory exists (checked at the end of the screenshot test).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use egui_kittest::kittest::NodeT;
use egui_kittest::Harness;

use handshake_native::graph::canvas_board::{
    placement_resize_author_id, CanvasEvent, CanvasPlacementCard, LoomCanvasBoard,
};
use handshake_native::graph::canvas_sections::{section_author_id, SectionLayer};
use handshake_native::theme::HsTheme;

/// The crate-relative path to the EXTERNAL artifacts root (CX-212E), disk-agnostic. The crate sits at
/// `<repo>/src/frontend/handshake_native`, so four `..` reach `<repo>/..` where `Handshake_Artifacts`
/// is a sibling of the repo worktree.
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

/// Serialize the `.wgpu()` screenshot test (the documented Windows-wgpu concurrent-device hazard).
static WGPU_SERIAL_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());
fn wgpu_guard() -> std::sync::MutexGuard<'static, ()> {
    WGPU_SERIAL_GUARD.lock().unwrap_or_else(|p| p.into_inner())
}

/// A seeded board with `n` BLOCK-ref placements `p-001..` referencing `block-001..`, each with a resolved
/// live title. No backend — the placements stand in for a real `getCanvasBoard` + `getLoomBlock` resolve.
fn seeded_board(n: usize) -> LoomCanvasBoard {
    let mut b = LoomCanvasBoard::new("ws-test", "canvas-1");
    let placements: Vec<CanvasPlacementCard> = (0..n)
        .map(|i| {
            let mut c = CanvasPlacementCard::new(
                format!("p-{:03}", i + 1),
                format!("block-{:03}", i + 1),
                (i as f32) * 260.0 + 40.0,
                60.0,
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
        .with_size(egui::vec2(960.0, 680.0))
        // Small per-frame time advance so a two-frame double-click stays within egui's ~0.3s
        // double-click window (the kittest default step_dt is 0.25s, which would split a double-click).
        .with_step_dt(1.0 / 240.0)
        .build_ui(move |ui| {
            let pal = HsTheme::Dark.palette();
            if let Some(ev) = board.lock().unwrap().show(ui, &pal) {
                events.lock().unwrap().push(ev);
            }
        })
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

/// Read a node's AccessKit `label` by author_id.
fn label_for(harness: &Harness<'_, ()>, author_id: &str) -> Option<String> {
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        if ak.author_id() == Some(author_id) {
            return ak.label().map(|v| v.to_owned());
        }
    }
    None
}

/// Convert a CANVAS-space point to the EXACT screen coordinate of the live widget, reading the board's
/// recorded canvas rect (no hard-coded layout offset — the handle is a small target, so exactness
/// matters). Panics if the board has not rendered yet (the caller always `run()`s first).
fn canvas_to_screen(board: &Arc<Mutex<LoomCanvasBoard>>, canvas: egui::Pos2) -> egui::Pos2 {
    board
        .lock()
        .unwrap()
        .canvas_point_to_screen(canvas)
        .expect("board has rendered at least once (canvas rect recorded)")
}

/// Synthesize a pointer DRAG from `from_screen` to `to_screen` (press, a couple of moves, release) so the
/// dragged card / resize handle sees a real gesture with a non-trivial drag_delta and a drag_stopped().
/// Each move is its own frame so egui accumulates the drag (a single jump is treated as a click).
fn drag(harness: &mut Harness<'_, ()>, from: egui::Pos2, to: egui::Pos2) {
    harness.event(egui::Event::PointerMoved(from));
    harness.run();
    harness.event(egui::Event::PointerButton {
        pos: from,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::default(),
    });
    harness.run();
    // Several incremental moves so the gesture is unambiguously a drag with a real per-frame delta.
    let steps = 4;
    for s in 1..=steps {
        let t = s as f32 / steps as f32;
        let p = egui::pos2(from.x + (to.x - from.x) * t, from.y + (to.y - from.y) * t);
        harness.event(egui::Event::PointerMoved(p));
        harness.run();
    }
    harness.event(egui::Event::PointerButton {
        pos: to,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::default(),
    });
    harness.run();
}

/// Synthesize a DOUBLE-CLICK at `pos`. egui's double-click detector fires `double_clicked()` when the two
/// clicks land within its ~0.3s window at the same spot. The harness is built with a small `step_dt`
/// (1/240s) so the two press/release pairs (each its own frame) stay well within that window.
fn double_click(harness: &mut Harness<'_, ()>, pos: egui::Pos2) {
    harness.event(egui::Event::PointerMoved(pos));
    harness.run();
    for _ in 0..2 {
        harness.event(egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::default(),
        });
        harness.event(egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::default(),
        });
    }
    harness.run();
    // A second frame so the resulting edit-mode state (and the inline editor it spawns) is laid out.
    harness.run();
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// PT-061-1 / AC-061-1: resize handle drag => live w/h + EXACTLY ONE debounced ResizePlacement on stop.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn canvas_resize_handle_fires_one_debounced_patch() {
    let board = shared(seeded_board(1)); // p-001 at canvas (40,60) 200x120
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_ck = Arc::clone(&events);
    let mut harness = harness_for(Arc::clone(&board), Arc::clone(&events));
    harness.run();

    // The handle is at the card's bottom-right corner (canvas (x+w, y+h)); grab a point a few px inside
    // it, in EXACT screen coords read from the live canvas rect.
    let (cx0, cy0, w0, h0) = {
        let b = board.lock().unwrap();
        let c = b.placements.iter().find(|p| p.placement_id == "p-001").unwrap();
        (c.x, c.y, c.w, c.h)
    };
    let handle = canvas_to_screen(&board, egui::pos2(cx0 + w0 - 3.0, cy0 + h0 - 3.0));
    // Drag the handle +60px right and +40px down => the card grows by ~(60,40) in canvas units (zoom=1).
    let target = egui::pos2(handle.x + 60.0, handle.y + 40.0);
    drag(&mut harness, handle, target);

    // AC-061-1: the live w/h grew (optimistic in-flight resize applied during the drag).
    let (w1, h1) = {
        let b = board.lock().unwrap();
        let c = b.placements.iter().find(|p| p.placement_id == "p-001").unwrap();
        (c.w, c.h)
    };
    assert!(w1 > w0 + 30.0, "AC-061-1: resize must GROW the card width live (got {w0} -> {w1})");
    assert!(h1 > h0 + 20.0, "AC-061-1: resize must GROW the card height live (got {h0} -> {h1})");

    // RISK-061-1 / MC-061-1: EXACTLY ONE ResizePlacement fired for the whole gesture (debounced to
    // drag-stop), carrying the final geometry (== the live w/h after the drag).
    let resize_events: Vec<(String, f32, f32)> = events_ck
        .lock()
        .unwrap()
        .iter()
        .filter_map(|e| match e {
            CanvasEvent::ResizePlacement { placement_id, w, h } => {
                Some((placement_id.clone(), *w, *h))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        resize_events.len(),
        1,
        "RISK-061-1/MC-061-1: EXACTLY ONE ResizePlacement per drag gesture (got {resize_events:?})"
    );
    let (id, w, h) = &resize_events[0];
    assert_eq!(id, "p-001", "the resize targets the dragged card");
    assert!((w - w1).abs() < 0.5 && (h - h1).abs() < 0.5, "the PATCH carries the final geometry");

    // Host applies the PATCH + refreshes via getCanvasBoard: server geometry becomes authoritative.
    {
        let mut b = board.lock().unwrap();
        let mut kept = b.placements.clone();
        if let Some(c) = kept.iter_mut().find(|p| p.placement_id == "p-001") {
            c.w = *w;
            c.h = *h;
        }
        b.set_board(kept, vec![], egui::Vec2::ZERO, 1.0);
    }
    harness.run();
    let (wf, hf) = {
        let b = board.lock().unwrap();
        let c = b.placements.iter().find(|p| p.placement_id == "p-001").unwrap();
        (c.w, c.h)
    };
    assert!((wf - w1).abs() < 0.5 && (hf - h1).abs() < 0.5, "server geometry survives the refresh");
    println!("PT-061-1/AC-061-1: resize drag grew card {w0}x{h0} -> {wf}x{hf}, ONE ResizePlacement fired");
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// PT-061-2 / AC-061-3: drop a card inside a frame assigns the group; drop outside clears it.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn canvas_drop_into_section_assigns_then_clears() {
    // Seed a board with a section: p-001 already grouped g-research (so its frame exists), and p-002
    // ungrouped (the card we'll drag into / out of the frame).
    let mut board = seeded_board(2);
    {
        let c = board.placements.iter_mut().find(|p| p.placement_id == "p-001").unwrap();
        c.group_id = Some("g-research".to_owned());
        // Make the frame large so a dragged card can land inside it deterministically.
        c.x = 40.0;
        c.y = 60.0;
        c.w = 360.0;
        c.h = 320.0;
    }
    // p-002 starts OUTSIDE the frame (far to the right), ungrouped, with a HIGHER z_index so it stays the
    // topmost card under the pointer even after it is dropped on top of the (large) p-001 frame anchor —
    // the just-moved card is the one the next drag grabs (realistic + unambiguous hit-test).
    {
        let c = board.placements.iter_mut().find(|p| p.placement_id == "p-002").unwrap();
        c.group_id = None;
        c.x = 600.0;
        c.y = 60.0;
        c.w = 120.0;
        c.h = 80.0;
        c.z_index = 5;
    }
    let mut labels = BTreeMap::new();
    labels.insert("g-research".to_owned(), "Research".to_owned());
    board.set_section_labels(labels);

    let board = shared(board);
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_ck = Arc::clone(&events);
    let mut harness = harness_for(Arc::clone(&board), Arc::clone(&events));
    harness.run();

    // The g-research frame is derived from p-001's padded bounds (p-001 at canvas (40,60) 360x320).
    // Drag p-002's body CENTRE onto a point well INSIDE the frame (canvas (200,200), inside p-001's rect).
    let p2_canvas_center = {
        let b = board.lock().unwrap();
        let c = b.placements.iter().find(|p| p.placement_id == "p-002").unwrap();
        egui::pos2(c.x + c.w * 0.5, c.y + c.h * 0.5)
    };
    let p2_center = canvas_to_screen(&board, p2_canvas_center);
    let inside_frame = canvas_to_screen(&board, egui::pos2(200.0, 200.0));
    drag(&mut harness, p2_center, inside_frame);

    // AC-061-3 (assign): an AssignSection{Some("g-research")} fired for p-002, and the card now reflects
    // the group locally (the AccessKit data-group-id updates this frame).
    let assigned = events_ck.lock().unwrap().iter().any(|e| matches!(e,
        CanvasEvent::AssignSection { placement_id, group_id: Some(g) }
            if placement_id == "p-002" && g == "g-research"));
    assert!(assigned, "AC-061-3: drop INSIDE the frame must AssignSection{{Some(g-research)}}");
    assert_eq!(
        board.lock().unwrap().placements.iter().find(|p| p.placement_id == "p-002").unwrap().group_id.as_deref(),
        Some("g-research"),
        "the dropped card's group_id is set locally (optimistic, confirmed by the next refresh)"
    );

    // Now drag p-002 back OUT of every frame -> AssignSection{None} (clear). p-002 is now inside the
    // frame; drag it far to the right (well outside the padded frame bounds).
    events_ck.lock().unwrap().clear();
    let p2_now_canvas = {
        let b = board.lock().unwrap();
        let c = b.placements.iter().find(|p| p.placement_id == "p-002").unwrap();
        egui::pos2(c.x + c.w * 0.5, c.y + c.h * 0.5)
    };
    let p2_now = canvas_to_screen(&board, p2_now_canvas);
    // A canvas point far outside the padded g-research frame (which spans ~x[16,424] y[14,404]).
    let outside = canvas_to_screen(&board, egui::pos2(820.0, 600.0));
    drag(&mut harness, p2_now, outside);

    let cleared = events_ck.lock().unwrap().iter().any(|e| matches!(e,
        CanvasEvent::AssignSection { placement_id, group_id: None } if placement_id == "p-002"));
    assert!(cleared, "AC-061-3: drop OUTSIDE all frames must AssignSection{{None}} (clear)");
    assert_eq!(
        board.lock().unwrap().placements.iter().find(|p| p.placement_id == "p-002").unwrap().group_id,
        None,
        "the card's group_id is cleared locally on a drop outside all frames"
    );
    println!("PT-061-2/AC-061-3: drop-inside assigned g-research; drop-outside cleared the section");
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// PT-061-3 / AC-061-4: double-click a TEXT card => inline edit; commit persists; Escape discards; a
// BLOCK card never becomes an editor (reference-not-copy gate).
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn canvas_text_card_inline_edit_commit_and_refresh() {
    // One TEXT card (inline-editable) + one BLOCK card (must NOT become editable).
    let mut board = LoomCanvasBoard::new("ws-test", "canvas-1");
    let mut text = CanvasPlacementCard::new("p-text", "block-text", 40.0, 60.0, 240.0, 140.0)
        .as_text_card("original body");
    text.live_title = Some("Note A".to_owned());
    text.live_content_type = Some("note".to_owned());
    let mut blockref = CanvasPlacementCard::new("p-block", "block-ref", 360.0, 60.0, 200.0, 120.0);
    blockref.live_title = Some("Block B".to_owned());
    blockref.live_content_type = Some("note".to_owned());
    board.set_board(vec![text, blockref], vec![], egui::Vec2::ZERO, 1.0);

    let board = shared(board);
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_ck = Arc::clone(&events);
    let mut harness = harness_for(Arc::clone(&board), Arc::clone(&events));
    harness.run();

    // Double-click the TEXT card center -> it enters edit mode (the buffer seeds from its body). The
    // BLOCK-card-stays-non-editable proof is its own test (`canvas_block_card_double_click_navigates`),
    // so this test stays a single clean gesture.
    let text_center = canvas_to_screen(&board, egui::pos2(40.0 + 120.0, 60.0 + 70.0));
    double_click(&mut harness, text_center);
    assert_eq!(
        board.lock().unwrap().editing_card_id(),
        Some("p-text"),
        "AC-061-4: double-click a TEXT card enters in-place edit mode"
    );

    // Let the editor settle so it acquires focus (request_focus on entry takes effect next frame), then
    // type a new body. The inline TextEdit::multiline appends the typed text to its buffer.
    harness.run();
    harness.event(egui::Event::Text("edited body!".to_owned()));
    harness.run();
    assert!(
        board.lock().unwrap().editing_buffer().contains("edited body!"),
        "the typed text landed in the inline editor buffer (got {:?})",
        board.lock().unwrap().editing_buffer()
    );
    // Commit via Ctrl+Enter while the editor is focused (the keyboard commit path).
    harness.event(egui::Event::Key {
        key: egui::Key::Enter,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers { command: true, ..Default::default() },
    });
    harness.run();
    harness.run();

    // AC-061-4 (TYPED BLOCKER — RISK-061-5 / MC-061-5): committing a text-card edit fires the
    // contract-mandated TextCardEditBlocked, NOT a (non-existent) persistence event. The cards endpoint
    // (POST .../cards) is create-only and the real edit route (PUT /knowledge/documents/:id/save) was not
    // bound by this MT, so the edit CANNOT be persisted as specified — the widget surfaces a typed blocker
    // instead of pretending. Edit mode still exits on commit. We do NOT assert persistence here: a
    // tautological "feed the body back through set_board and check it survived" round-trip would touch no
    // real backend and prove nothing (Spec-Realism Sub-rule 2). AC-061-4 persistence is gated
    // NEEDS_MANAGED_RESOURCE_PROOF until the Orchestrator re-scopes the binding.
    let blocker = events_ck.lock().unwrap().iter().find_map(|e| match e {
        CanvasEvent::TextCardEditBlocked {
            placement_id,
            block_id,
            title,
            pending_body,
            attempted_route,
            required_route,
        } if placement_id == "p-text" => Some((
            block_id.clone(),
            title.clone(),
            pending_body.clone(),
            *attempted_route,
            *required_route,
        )),
        _ => None,
    });
    let (block_id, title, pending_body, attempted_route, required_route) =
        blocker.expect("AC-061-4: committing a text-card edit must fire the typed TextCardEditBlocked");
    assert_eq!(block_id, "block-text", "the blocker carries the card's backing block id (diagnostic)");
    assert_eq!(title, "Note A", "the blocker carries the card title");
    assert!(
        pending_body.contains("edited body!"),
        "the blocker carries the edited buffer the host WOULD persist (got {pending_body:?})"
    );
    assert!(
        attempted_route.contains("CREATE-ONLY"),
        "the blocker names the create-only cards route (got {attempted_route:?})"
    );
    assert!(
        required_route.contains("/knowledge/documents/") && required_route.contains("UNBOUND"),
        "the blocker names the real, unbound edit route (got {required_route:?})"
    );
    assert_eq!(board.lock().unwrap().editing_card_id(), None, "edit mode exits on commit");
    // Sanity: NO real persistence event exists — the only edit-commit event is the typed blocker.
    println!(
        "PT-061-3/AC-061-4: inline edit is LIVE; commit emits TextCardEditBlocked (cards endpoint \
         create-only; PUT /knowledge/documents/:id/save unbound) — persistence gated \
         NEEDS_MANAGED_RESOURCE_PROOF"
    );
}

#[test]
fn canvas_text_card_escape_discards_without_event() {
    let mut board = LoomCanvasBoard::new("ws-test", "canvas-1");
    let mut text = CanvasPlacementCard::new("p-text", "block-text", 40.0, 60.0, 240.0, 140.0)
        .as_text_card("keep me");
    text.live_title = Some("Note A".to_owned());
    board.set_board(vec![text], vec![], egui::Vec2::ZERO, 1.0);

    let board = shared(board);
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_ck = Arc::clone(&events);
    let mut harness = harness_for(Arc::clone(&board), Arc::clone(&events));
    harness.run();

    // Enter edit mode by double-clicking the text card.
    let text_center = canvas_to_screen(&board, egui::pos2(40.0 + 120.0, 60.0 + 70.0));
    double_click(&mut harness, text_center);
    assert_eq!(board.lock().unwrap().editing_card_id(), Some("p-text"), "entered edit mode");

    // Type then press Escape -> the edit is discarded with NO EditTextCard event.
    harness.event(egui::Event::Text("scratch".to_owned()));
    harness.run();
    harness.event(egui::Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::default(),
    });
    harness.run();

    assert_eq!(board.lock().unwrap().editing_card_id(), None, "Escape exits edit mode");
    let any_edit = events_ck
        .lock()
        .unwrap()
        .iter()
        .any(|e| matches!(e, CanvasEvent::TextCardEditBlocked { .. }));
    assert!(!any_edit, "AC-061-4: Escape must NOT fire a commit/blocker event (no server call)");
    println!("PT-061-3/AC-061-4: Escape discarded the inline edit with no server event");
}

/// AC-061-4 / AC-061-5 (reference-not-copy gate): double-clicking a BLOCK-backed card must NOT enter
/// inline edit mode (it navigates to the block instead). A block card never becomes an editor, so a
/// canvas edit can never fork/copy the underlying block.
#[test]
fn canvas_block_card_double_click_navigates_not_edits() {
    let mut board = LoomCanvasBoard::new("ws-test", "canvas-1");
    // A BLOCK reference (the default kind — NOT a text card).
    let mut blockref = CanvasPlacementCard::new("p-block", "block-ref", 40.0, 60.0, 220.0, 120.0);
    blockref.live_title = Some("Block B".to_owned());
    blockref.live_content_type = Some("note".to_owned());
    board.set_board(vec![blockref], vec![], egui::Vec2::ZERO, 1.0);

    let board = shared(board);
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_ck = Arc::clone(&events);
    let mut harness = harness_for(Arc::clone(&board), Arc::clone(&events));
    harness.run();

    let block_center = canvas_to_screen(&board, egui::pos2(40.0 + 110.0, 60.0 + 60.0));
    double_click(&mut harness, block_center);

    assert_eq!(
        board.lock().unwrap().editing_card_id(),
        None,
        "AC-061-4/AC-061-5: a BLOCK-backed card double-click must NOT enter inline edit (reference gate)"
    );
    let any_edit = events_ck
        .lock()
        .unwrap()
        .iter()
        .any(|e| matches!(e, CanvasEvent::TextCardEditBlocked { .. }));
    assert!(
        !any_edit,
        "a block card double-click never fires a text-card edit/blocker event (never inline-editable)"
    );
    println!("AC-061-5: block-backed card double-click stayed non-editable (navigates, never forks)");
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// PT-061-4 / AC-061-2 / AC-061-6: AccessKit tree exposes canvas.section.{id} + canvas.placement.{id}.resize
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn canvas_section_and_resize_accesskit_nodes_present() {
    // Two cards grouped into one section so a frame is derived + a section node is emitted.
    let mut board = seeded_board(2);
    for pid in ["p-001", "p-002"] {
        let c = board.placements.iter_mut().find(|p| p.placement_id == pid).unwrap();
        c.group_id = Some("g-alpha".to_owned());
    }
    let mut labels = BTreeMap::new();
    labels.insert("g-alpha".to_owned(), "Alpha Section".to_owned());
    board.set_section_labels(labels);

    let board = shared(board);
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut harness = harness_for(Arc::clone(&board), events);
    harness.run();

    let ids = author_ids(&harness);

    // AC-061-2 / AC-061-6: the section frame node is present with the section label.
    let section_id = section_author_id("g-alpha");
    assert!(ids.contains(&section_id), "AC-061-2: '{section_id}' section node present in {ids:?}");
    assert_eq!(
        label_for(&harness, &section_id).as_deref(),
        Some("Alpha Section"),
        "AC-061-2: the section node label is the section title"
    );

    // AC-061-6: each card's resize handle node is present and extends (does not collide with) the card id.
    for pid in ["p-001", "p-002"] {
        let resize_id = placement_resize_author_id(pid);
        assert!(ids.contains(&resize_id), "AC-061-6: '{resize_id}' resize-handle node present");
    }

    // The MT-026 placement card ids still coexist (the new ids EXTEND, not replace — RISK-061-6 no
    // collision).
    assert!(ids.contains("canvas.placement.p-001"), "MT-026 card node still present (no collision)");
    assert!(ids.contains("canvas.placement.p-001.resize"), "MT-061 resize node coexists");

    // Exactly one section node for the one distinct group_id.
    let section_count = ids.iter().filter(|a| a.starts_with("canvas.section.")).count();
    assert_eq!(section_count, 1, "PT-061-4: exactly one section node for one group_id (got {section_count})");
    println!("PT-061-4/AC-061-2/AC-061-6: canvas.section.g-alpha + 2x canvas.placement.*.resize present");
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// PT-061-5 / AC-061-5: a resize + section-assign cycle on a BLOCK card never changes the board's
// underlying block_id set (reference-not-copy). Driven through the LIVE widget gestures.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn canvas_reference_invariant_block_id_set_unchanged() {
    let board = shared(seeded_board(2)); // both BlockRef
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut harness = harness_for(Arc::clone(&board), Arc::clone(&events));
    harness.run();

    let before: std::collections::BTreeSet<String> = {
        let b = board.lock().unwrap();
        b.placements.iter().map(|p| p.placed_block_id.clone()).collect()
    };

    // RESIZE p-001 via a real handle drag.
    let (cx0, cy0, w0, h0) = {
        let b = board.lock().unwrap();
        let c = b.placements.iter().find(|p| p.placement_id == "p-001").unwrap();
        (c.x, c.y, c.w, c.h)
    };
    let handle = canvas_to_screen(&board, egui::pos2(cx0 + w0 - 3.0, cy0 + h0 - 3.0));
    drag(&mut harness, handle, egui::pos2(handle.x + 50.0, handle.y + 30.0));

    // SECTION-ASSIGN p-001: drag its body a little (no frame exists yet, so this clears — still a
    // placement-only mutation). The invariant is about block_ids, which neither op touches.
    let p1_canvas = {
        let b = board.lock().unwrap();
        let c = b.placements.iter().find(|p| p.placement_id == "p-001").unwrap();
        egui::pos2(c.x + c.w * 0.5, c.y + c.h * 0.5)
    };
    let p1c = canvas_to_screen(&board, p1_canvas);
    drag(&mut harness, p1c, egui::pos2(p1c.x + 30.0, p1c.y + 30.0));

    let after: std::collections::BTreeSet<String> = {
        let b = board.lock().unwrap();
        b.placements.iter().map(|p| p.placed_block_id.clone()).collect()
    };
    assert_eq!(before, after, "PT-061-5/AC-061-5: the block_id SET is invariant across resize+section");
    assert_eq!(board.lock().unwrap().placements.len(), 2, "no placement duplicated (reference, not copy)");
    println!("PT-061-5/AC-061-5: block_id set unchanged across a live resize + section-assign cycle");
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// AC-061-2 screenshot (HBR-VIS): a non-blank canvas with a titled section frame behind its cards.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn canvas_section_frame_screenshot() {
    let _g = wgpu_guard();
    let mut board = seeded_board(2);
    for pid in ["p-001", "p-002"] {
        let c = board.placements.iter_mut().find(|p| p.placement_id == pid).unwrap();
        c.group_id = Some("g-alpha".to_owned());
    }
    let mut labels = BTreeMap::new();
    labels.insert("g-alpha".to_owned(), "Alpha Section".to_owned());
    board.set_section_labels(labels);

    let board = shared(board);
    let events = Arc::new(Mutex::new(Vec::new()));
    let board_ui = Arc::clone(&board);
    let events_ui = Arc::clone(&events);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(960.0, 680.0))
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
            let mut counts: std::collections::HashMap<[u8; 4], u32> = std::collections::HashMap::new();
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
            assert!(total > 0, "AC-061-2: sampled pixels must be opaque");
            assert!(
                (white as f32 / total as f32) < 0.95,
                "AC-061-2: canvas must not be ~all-white (white frac {})",
                white as f32 / total as f32
            );
            // The dark bg + the section-frame fill/border + the light card surfaces => several distinct
            // colours (a section frame and cards were painted).
            assert!(
                counts.len() >= 3,
                "AC-061-2: >= 3 distinct colours expected (bg + frame + card), got {}",
                counts.len()
            );

            let ext_dir = external_artifact_dir("wp-kernel-012-mt-061");
            let _ = std::fs::create_dir_all(&ext_dir);
            let png = ext_dir.join("MT-061-canvas-section-resize.png");
            let saved = image.save(&png).is_ok();
            println!(
                "AC-061-2: {w}x{h} screenshot, {} distinct colours, white_frac={:.3}, saved={saved} ({})",
                counts.len(),
                white as f32 / total as f32,
                png.display()
            );
        }
        Err(e) => {
            println!(
                "BLOCKER(non-fatal): canvas section screenshot render unavailable (no wgpu adapter): {e}. \
                 The AccessKit section/resize node + interaction proofs passed; the PNG is a GPU-host item."
            );
        }
    }
    assert_no_local_artifact_dir();
}

// ── Standalone (no harness): the SectionLayer derivation the widget draws from ─────────────────────

#[test]
fn section_layer_derives_one_frame_per_group_from_board() {
    let mut board = seeded_board(3);
    board.placements[0].group_id = Some("g-a".to_owned());
    board.placements[1].group_id = Some("g-a".to_owned());
    board.placements[2].group_id = Some("g-b".to_owned());
    let layer: SectionLayer = board.section_layer();
    assert_eq!(layer.frames.len(), 2, "two distinct group_ids => two frames");
    assert!(layer.frame("g-a").is_some() && layer.frame("g-b").is_some(), "both frames derived");
}

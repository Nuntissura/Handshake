//! Reading/preview mode (Obsidian "reading view") proofs (WP-KERNEL-012 MT-055).
//!
//! These prove the per-document Edit|Reading toggle + the read-only render path:
//!
//! - PT-001 / AC-007: `ReadingModeStore` toggle + per-document isolation + serde round-trip
//!   (the persistence shape egui round-trips across re-render / app restart). The pure-data store
//!   unit tests live in `src/rich_editor/reading_mode.rs`; here we ALSO prove the egui-context
//!   persisted accessor keeps a Reading choice across a second render frame (AC-007 / MC-004).
//! - AC-001 / RISK-005: a `ViewMode::Reading` host builds a `read_only=true` widget AND a Reading
//!   render mutates NOTHING — a key/text event delivered while the read-only editor is rendered
//!   leaves the DocModel byte-identical (no caret/selection alloc, no input dispatch).
//! - PT-002 / AC-002 + AC-003: a kittest seeds a DocModel (heading + bullet list + quote + wikilink),
//!   renders it in Reading mode, dumps the live AccessKit tree, and asserts the heading/list/quote
//!   block nodes are present AND that NO editable TextEdit/TextInput node exists for the document body.
//! - PT-004 / AC-004 / RISK-003: clicking a wikilink chip while in Reading mode enqueues a
//!   `WikilinkActivated` event into `pending_events` — the editor seam the shell drains and routes
//!   through the shared navigation/command bus (command_registry.rs + event_bus.rs).
//! - PT-003 / AC-005: a kittest screenshot of the read-only document renders non-empty with the
//!   heading/list visible, saved to the EXTERNAL Handshake_Artifacts root (CX-212E, never repo-local).
//! - PT-005 / AC-007: persistence — set Reading for one document_id, re-render reading from the same
//!   store, assert still Reading; a different document_id still defaults to Edit.
//! - AC-008: toggling Reading then Edit returns the EXACT MT-012 editable path — a key event then
//!   mutates the DocModel (the editor edits again), proving no regression.
//! - AC-006: the Edit|Reading toggle widget emits the `rich-reading-mode-toggle` (Group),
//!   `rich-reading-mode-edit` (Button), `rich-reading-mode-reading` (Button) AccessKit nodes and
//!   marks the active segment toggled.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;

use handshake_native::rich_editor::document_model::node::{
    BlockNode, Child, HsLinkNode, NodeKind, TextLeaf,
};
use handshake_native::rich_editor::document_model::position::DocPosition;
use handshake_native::rich_editor::document_model::selection::Selection;
use handshake_native::rich_editor::reading_mode::{
    view_mode_toggle, ReadingModeStore, ViewMode, TOGGLE_CONTAINER_AUTHOR_ID,
    TOGGLE_EDIT_AUTHOR_ID, TOGGLE_READING_AUTHOR_ID,
};
use handshake_native::rich_editor::renderer::block_author_id;
use handshake_native::rich_editor::renderer::rich_editor_widget::{
    RichEditorState, RichEditorWidget,
};
use handshake_native::rich_editor::wikilinks::inline_view::{chip_author_id, EditorEvent};

/// The crate-relative path to the EXTERNAL artifacts root (CX-212E), disk-agnostic — the crate
/// sits at `<repo>/src/frontend/handshake_native`, so four `..` reach `<repo>/..` where
/// `Handshake_Artifacts` is a sibling of the repo worktree.
fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

/// The screenshot path under the EXTERNAL artifact root. The MT contract's literal `test_output/`
/// path is OVERRIDDEN by the CX-212E external-only artifact-hygiene rule applied across the WP — a
/// committed repo-local PNG is a hygiene regression.
fn screenshot_path() -> PathBuf {
    external_artifact_dir("wp-kernel-012-mt-055").join("mt055_reading_mode.png")
}

/// Assert NO repo-local artifact dir exists under the crate (CX-212E): neither `test_output/` nor
/// `tests/screenshots/`. Screenshots/artifacts go to the external Handshake_Artifacts root ONLY.
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

/// The role string of every node in the live AccessKit tree (for the "no editable body node" proof).
fn all_roles<S>(harness: &Harness<'_, S>) -> Vec<String> {
    harness
        .root()
        .children_recursive()
        .map(|n| format!("{:?}", n.accesskit_node().role()))
        .collect()
}

/// A seeded DocModel: an h1 heading, a bullet list (one item), a blockquote, and a paragraph that
/// carries a `note` wikilink chip. The reading view must render the heading/list/quote and keep the
/// wikilink interactive.
fn seeded_doc() -> BlockNode {
    let heading = BlockNode::heading(1, "Reading View Title");
    // A bullet list with one list item holding a paragraph of text.
    let item = BlockNode::with_children(
        NodeKind::ListItem,
        vec![Child::Block(BlockNode::paragraph("First bullet"))],
    );
    let list = BlockNode::with_children(NodeKind::BulletList, vec![Child::Block(item)]);
    let quote = BlockNode::with_children(
        NodeKind::Blockquote,
        vec![Child::Block(BlockNode::paragraph("A quoted line"))],
    );
    // A paragraph carrying a `note` wikilink chip (the interactive read-only navigation target).
    let para_with_link = BlockNode::with_children(
        NodeKind::Paragraph,
        vec![
            Child::Text(TextLeaf::new("see ")),
            Child::HsLink(HsLinkNode::new("note", "target-doc-id", "Target Note")),
            Child::Text(TextLeaf::new("")),
        ],
    );
    BlockNode::doc(vec![heading, list, quote, para_with_link])
}

/// Build a read-only kittest harness over `state` with the shell Inter fonts installed.
fn reading_harness<'a>(state: Arc<Mutex<RichEditorState>>) -> Harness<'a, ()> {
    let state_for_ui = Arc::clone(&state);
    Harness::builder()
        .with_size(egui::vec2(900.0, 600.0))
        .wgpu()
        .build_ui(move |ui| {
            handshake_native::app::HandshakeApp::install_fonts(ui.ctx());
            RichEditorWidget::new_read_only(Arc::clone(&state_for_ui)).show(ui);
        })
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════
// AC-001 / RISK-005: a ViewMode::Reading host builds read_only=true; a Reading render mutates nothing.
// ═══════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn ac001_view_mode_reading_builds_read_only_widget() {
    let state = Arc::new(Mutex::new(RichEditorState::demo()));
    let reading = RichEditorWidget::for_view_mode(Arc::clone(&state), ViewMode::Reading);
    assert!(
        reading.is_read_only(),
        "AC-001: ViewMode::Reading builds a read_only=true widget"
    );
    let edit = RichEditorWidget::for_view_mode(Arc::clone(&state), ViewMode::Edit);
    assert!(
        !edit.is_read_only(),
        "AC-001: ViewMode::Edit builds a read_only=false widget"
    );
    // The plain `new` constructor is editable; `new_read_only` is read-only.
    assert!(!RichEditorWidget::new(Arc::clone(&state)).is_read_only());
    assert!(RichEditorWidget::new_read_only(Arc::clone(&state)).is_read_only());
    println!("AC-001: ViewMode::Reading -> read_only=true; Edit -> read_only=false");
}

#[test]
fn ac001_reading_render_does_not_mutate_doc() {
    // RISK-005 / MC-005: render the SAME doc read-only while delivering a text event + an Enter key;
    // the read-only branch allocates no caret/selection state and dispatches no input, so the
    // DocModel is byte-identical before/after. We compare the canonical content_json.
    let doc = seeded_doc();
    let before =
        handshake_native::rich_editor::document_model::doc_json::to_content_json_value(&doc);
    let state = Arc::new(Mutex::new(RichEditorState::new(doc)));

    let mut harness = reading_harness(Arc::clone(&state));
    harness.run();
    // Try to "type" into the read-only editor: push a Text event + an Enter key for the next frame.
    harness
        .input_mut()
        .events
        .push(egui::Event::Text("X".to_owned()));
    harness.input_mut().events.push(egui::Event::Key {
        key: egui::Key::Enter,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    });
    harness.run();
    harness.run();

    let after = {
        let st = state.lock().unwrap();
        handshake_native::rich_editor::document_model::doc_json::to_content_json_value(&st.doc)
    };
    assert_eq!(
        before, after,
        "AC-001/RISK-005: a Reading-mode render must NOT mutate the DocModel even when input events arrive"
    );
    println!("AC-001: Reading-mode render left the DocModel unchanged after a typed X + Enter");
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════
// PT-002 / AC-002 + AC-003: heading/list/quote present; NO editable TextEdit/TextInput body node.
// ═══════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn pt002_reading_tree_has_blocks_and_no_editable_body_node() {
    let state = Arc::new(Mutex::new(RichEditorState::new(seeded_doc())));
    let mut harness = reading_harness(Arc::clone(&state));
    harness.run();

    // AC-002: the heading block (top-level index 0) is present as a re-block node. The wikilink
    // paragraph (index 3) is also a re-block node — proving the SAME MT-012 block renderer ran.
    let ids = author_ids(&harness);
    let heading_author = block_author_id(&[0]);
    let link_para_author = block_author_id(&[3]);
    assert!(
        ids.contains(&heading_author),
        "AC-002: the heading block node `{heading_author}` must be present; ids: {ids:?}"
    );
    assert!(
        ids.contains(&link_para_author),
        "AC-002: the wikilink paragraph block node `{link_para_author}` must be present; ids: {ids:?}"
    );

    // AC-003 / RISK-002 / MC-002: the document body region must NOT contain ANY editable
    // TextEdit/TextInput node while in Reading mode — including the editor-root container. The
    // literal contract invariant is ZERO TextInput/MultilineTextInput nodes anywhere in the
    // read-only document body; the root container is now emitted as `Role::Document` + the AccessKit
    // `ReadOnly` flag (not `Role::TextInput`) in read-only mode, so no whitelist is needed.
    let mut editable_body_nodes = Vec::new();
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        let role = format!("{:?}", ak.role());
        let is_textinput = role == "TextInput" || role == "MultilineTextInput";
        if is_textinput {
            editable_body_nodes.push((ak.author_id().map(|s| s.to_owned()), role));
        }
    }
    assert!(
        editable_body_nodes.is_empty(),
        "AC-003: Reading mode must contain ZERO editable TextEdit/TextInput nodes anywhere in the \
         document body (root container included); found: {editable_body_nodes:?}"
    );
    let roles = all_roles(&harness);
    println!(
        "PT-002 accesskit dump: heading={heading_author} present; link-para={link_para_author} present; \
         editable-body-textinputs=0; total nodes inspected (roles)={}",
        roles.len()
    );
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════
// PT-004 / AC-004 / RISK-003: a wikilink chip click in Reading mode enqueues a navigation event.
// ═══════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn pt004_wikilink_click_in_reading_mode_routes_navigation_event() {
    let state = Arc::new(Mutex::new(RichEditorState::new(seeded_doc())));
    let state_ck = Arc::clone(&state);
    let mut harness = reading_harness(Arc::clone(&state));
    harness.run();

    // The note wikilink chip is addressable by its stable author_id.
    let chip_id = chip_author_id("target-doc-id");
    let ids = author_ids(&harness);
    assert!(
        ids.contains(&chip_id),
        "AC-004/RISK-003: the wikilink chip `{chip_id}` must stay interactive in Reading mode; ids: {ids:?}"
    );

    // Click the chip while in Reading mode; it must enqueue a WikilinkActivated event (the editor seam
    // the shell drains + routes through command_registry.rs + event_bus.rs).
    let chip = harness.get_by(|n| n.author_id() == Some(chip_id.as_str()));
    chip.click();
    harness.run();

    let event = {
        let st = state_ck.lock().unwrap();
        st.pending_events.iter().find_map(|e| match e {
            EditorEvent::WikilinkActivated {
                ref_kind,
                ref_value,
                ..
            } if ref_kind == "note" => Some((ref_kind.clone(), ref_value.clone())),
            _ => None,
        })
    };
    let (ref_kind, ref_value) = event.expect(
        "AC-004: clicking the wikilink chip in Reading mode must enqueue a WikilinkActivated event",
    );
    assert_eq!(ref_kind, "note");
    assert_eq!(
        ref_value, "target-doc-id",
        "AC-004: the event carries the navigation target"
    );
    println!(
        "PT-004: wikilink click in Reading mode routed WikilinkActivated{{note, target-doc-id}}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════
// PT-003 / AC-005: a screenshot of the read-only document renders non-empty with content visible.
// ═══════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn pt003_reading_mode_screenshot() {
    let state = Arc::new(Mutex::new(RichEditorState::new(seeded_doc())));
    let mut harness = reading_harness(Arc::clone(&state));
    harness.run();
    harness.run();

    match harness.render() {
        Ok(image) => {
            let (w, h) = (image.width(), image.height());
            assert!(w > 0 && h > 0, "rendered image must be non-empty");

            let path = screenshot_path();
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let saved = image.save(&path).is_ok();

            // Pixel proof: the read-only doc renders a dark background + multiple foreground colors
            // (heading glyphs, body glyphs, the colored wikilink chip), so there are >= 2 distinct
            // foreground colors over the background (AC-005 content visible). Sample every 4th pixel.
            let raw = image.as_raw();
            let mut counts: std::collections::HashMap<[u8; 4], u32> =
                std::collections::HashMap::new();
            let mut i = 0usize;
            while i + 4 <= raw.len() {
                let px = [raw[i], raw[i + 1], raw[i + 2], raw[i + 3]];
                if px[3] != 0 {
                    *counts.entry(px).or_insert(0) += 1;
                }
                i += 4 * 4;
            }
            let bg = counts.iter().max_by_key(|(_, c)| **c).map(|(p, _)| *p);
            let foreground: std::collections::HashSet<[u8; 4]> =
                counts.keys().filter(|p| Some(**p) != bg).copied().collect();

            println!(
                "PT-003 screenshot: {w}x{h}, {} distinct colors, {} foreground; saved={saved} ({})",
                counts.len(),
                foreground.len(),
                screenshot_path().display(),
            );
            assert!(
                foreground.len() >= 2,
                "AC-005: the read-only doc must produce >= 2 distinct foreground colors (heading/body/chip \
                 over the bg); got {} (bg={bg:?})",
                foreground.len()
            );
            assert!(
                saved,
                "the mt055_reading_mode.png screenshot must be saved to the external artifact root"
            );
            assert_no_local_artifact_dir();
        }
        Err(e) => {
            println!(
                "BLOCKER(non-fatal): mt055 reading-mode screenshot render unavailable (no wgpu adapter / \
                 headless GPU crash): {e}. The AccessKit + structural proofs stand; the PNG is a GPU-host item."
            );
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════
// PT-005 / AC-007: persistence — Reading for one doc survives a second render; another doc is Edit.
// ═══════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn pt005_mode_persists_per_document_across_render() {
    // Drive the egui-context persisted store across two frames: set Reading for "doc-A" frame 1,
    // read it back frame 2 from the SAME persisted store — still Reading. "doc-B" still defaults Edit.
    // The frame-2 read is captured into a shared cell from INSIDE the render closure (the harness has
    // no public ctx() accessor), so the assert reads exactly what the persisted store returned on a
    // later frame.
    let observed: Arc<Mutex<Option<(ViewMode, ViewMode)>>> = Arc::new(Mutex::new(None));
    let observed_ui = Arc::clone(&observed);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(400.0, 200.0))
        .build_ui(move |ui| {
            use handshake_native::rich_editor::reading_mode::{
                reading_mode_store, write_reading_mode_store,
            };
            let mut store = reading_mode_store(ui.ctx());
            // On the first frame the store is empty -> set doc-A to Reading and persist it. On a later
            // frame the persisted store already carries doc-A = Reading, so this branch is skipped and
            // the read-back below reflects the persisted value.
            if store.get("doc-A") == ViewMode::Edit {
                store.set("doc-A", ViewMode::Reading);
                write_reading_mode_store(ui.ctx(), &store);
            }
            // Re-read the persisted store this frame (a fresh accessor, like the host does each frame)
            // and record what it returns for both documents.
            let read_back = reading_mode_store(ui.ctx());
            *observed_ui.lock().unwrap() = Some((read_back.get("doc-A"), read_back.get("doc-B")));
            ui.label("persist-frame");
        });
    harness.run(); // frame 1: writes doc-A = Reading
    harness.run(); // frame 2: reads it back from the persisted store

    let (doc_a, doc_b) = observed
        .lock()
        .unwrap()
        .expect("the render closure recorded the modes");
    assert_eq!(
        doc_a,
        ViewMode::Reading,
        "AC-007: doc-A's Reading choice must persist across re-render"
    );
    assert_eq!(
        doc_b,
        ViewMode::Edit,
        "AC-007: a different document_id (doc-B) must still default to Edit (per-document isolation)"
    );
    println!("PT-005: doc-A persisted Reading across re-render; doc-B still defaults Edit");
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════
// AC-006: the Edit|Reading toggle emits the three AccessKit ids and marks the active segment toggled.
// ═══════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn ac006_toggle_emits_accesskit_ids_and_active_toggled() {
    // Render JUST the toggle widget for a document whose mode is Reading; assert the container Group
    // id + both segment Button ids are present, and the Reading segment is the toggled one.
    let mut store = ReadingModeStore::new();
    store.set("doc-A", ViewMode::Reading);
    let store_cell = Arc::new(Mutex::new(store));
    let store_ui = Arc::clone(&store_cell);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(300.0, 80.0))
        .build_ui(move |ui| {
            handshake_native::app::HandshakeApp::install_fonts(ui.ctx());
            let mut s = store_ui.lock().unwrap();
            let _mode = view_mode_toggle(ui, "doc-A", &mut s);
        });
    harness.run();

    let ids = author_ids(&harness);
    for want in [
        TOGGLE_CONTAINER_AUTHOR_ID,
        TOGGLE_EDIT_AUTHOR_ID,
        TOGGLE_READING_AUTHOR_ID,
    ] {
        assert!(
            ids.contains(want),
            "AC-006: toggle must emit `{want}`; ids: {ids:?}"
        );
    }
    // The Reading segment (the active mode) must be marked toggled True; the Edit segment must not.
    let mut reading_toggled = None;
    let mut edit_toggled = None;
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        match ak.author_id() {
            Some(a) if a == TOGGLE_READING_AUTHOR_ID => reading_toggled = Some(ak.toggled()),
            Some(a) if a == TOGGLE_EDIT_AUTHOR_ID => edit_toggled = Some(ak.toggled()),
            _ => {}
        }
    }
    assert_eq!(
        reading_toggled,
        Some(Some(egui::accesskit::Toggled::True)),
        "AC-006: the active (Reading) segment must be marked toggled=True"
    );
    assert_ne!(
        edit_toggled,
        Some(Some(egui::accesskit::Toggled::True)),
        "AC-006: the inactive (Edit) segment must NOT be toggled=True"
    );
    println!("AC-006: toggle emitted Group + both Button ids; Reading segment toggled=True");
}

#[test]
fn ac006_clicking_a_segment_sets_the_mode() {
    // Render the toggle for a doc in Edit mode; click the Reading segment; the store flips to Reading.
    let store_cell = Arc::new(Mutex::new(ReadingModeStore::new()));
    let store_ui = Arc::clone(&store_cell);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(300.0, 80.0))
        .build_ui(move |ui| {
            handshake_native::app::HandshakeApp::install_fonts(ui.ctx());
            let mut s = store_ui.lock().unwrap();
            let _mode = view_mode_toggle(ui, "doc-A", &mut s);
        });
    harness.run();
    let reading = harness.get_by(|n| n.author_id() == Some(TOGGLE_READING_AUTHOR_ID));
    reading.click();
    harness.run();
    assert_eq!(
        store_cell.lock().unwrap().get("doc-A"),
        ViewMode::Reading,
        "AC-006: clicking the Reading segment sets the document's mode to Reading"
    );
    println!("AC-006: clicking the Reading segment flipped doc-A's store to Reading");
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════
// AC-008: toggling Reading then Edit returns the EXACT MT-012 editable path (no regression).
// ═══════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn ac008_edit_after_reading_edits_the_doc() {
    // Render a doc Reading first (proves a typed key is ignored), then Edit (proves a typed key now
    // mutates the doc — the editor edits again, no regression).
    let doc = BlockNode::doc(vec![BlockNode::paragraph("hi")]);
    let state = Arc::new(Mutex::new(RichEditorState::new(doc)));
    {
        // Put the caret at the end of "hi" so a typed char appends to the run.
        let mut st = state.lock().unwrap();
        st.selection = Selection::caret(DocPosition::new(vec![0, 0], 2));
    }

    // FRAME SET 1: read-only — a typed char is ignored.
    let before = {
        let st = state.lock().unwrap();
        st.block_plain_text(0).unwrap_or_default()
    };
    {
        let state_ro = Arc::clone(&state);
        let mut ro = Harness::builder()
            .with_size(egui::vec2(500.0, 200.0))
            .build_ui(move |ui| {
                handshake_native::app::HandshakeApp::install_fonts(ui.ctx());
                RichEditorWidget::new_read_only(Arc::clone(&state_ro)).show(ui);
            });
        ro.run();
        ro.input_mut()
            .events
            .push(egui::Event::Text("Z".to_owned()));
        ro.run();
    }
    let after_reading = {
        let st = state.lock().unwrap();
        st.block_plain_text(0).unwrap_or_default()
    };
    assert_eq!(
        before, after_reading,
        "AC-008 pre: Reading mode ignored the typed Z"
    );

    // FRAME SET 2: editable — focus the surface and type; the doc mutates (no regression). The
    // editable editor schedules a caret-blink repaint while focused, so use single-frame `step()`
    // (NOT `run()`, which loops until idle and would trip the blink-animation max_steps guard — the
    // same model the MT-012 editable tests use).
    {
        let state_edit = Arc::clone(&state);
        let mut ed = Harness::builder()
            .with_size(egui::vec2(500.0, 200.0))
            .build_ui(move |ui| {
                handshake_native::app::HandshakeApp::install_fonts(ui.ctx());
                let resp = RichEditorWidget::new(Arc::clone(&state_edit)).show(ui);
                resp.request_focus();
            });
        ed.step();
        ed.step(); // settle focus
        ed.input_mut()
            .events
            .push(egui::Event::Text("Z".to_owned()));
        ed.step();
        ed.step();
    }
    let after_edit = {
        let st = state.lock().unwrap();
        st.block_plain_text(0).unwrap_or_default()
    };
    assert!(
        after_edit.contains('Z') && after_edit.len() > before.len(),
        "AC-008: after switching back to Edit, typing mutates the doc again (was {before:?}, now {after_edit:?})"
    );
    println!("AC-008: Reading ignored the edit; Edit mode then accepted it ({before:?} -> {after_edit:?})");
}

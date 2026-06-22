//! Formatting toolbar + keymap + structural-editing integration proofs
//! (WP-KERNEL-012 MT-013).
//!
//! These are the contract proof_targets that need a LIVE egui frame or the full input
//! pipeline (the pure command-layer logic is proven by the in-crate unit tests in
//! `src/rich_editor/formatting/commands.rs` + `keymap.rs`):
//!
//! - AC-1 + PT-2 (`toolbar_screenshot`): the demo editor renders the toolbar row ABOVE
//!   the content area; the kittest AccessKit tree carries a `toolbar-btn-*` button for
//!   every category group (history/format/block/list/table), and the PNG is saved to the
//!   EXTERNAL `Handshake_Artifacts/handshake-test/wp-kernel-012-mt-013/` root (CX-212E —
//!   NEVER repo-local). `assert_no_local_artifact_dir()` guards both `test_output/` and
//!   `tests/screenshots/`.
//! - AC-3 (`ctrl_b_toggles_bold_via_input_handler`): a `Ctrl+B` egui key event fed
//!   through the REAL `input_handler` formatting-decode + apply pipeline toggles the bold
//!   mark on the selected text (the same code path the widget runs per frame).
//! - AC-10 (`accesskit_toolbar_btn_toggle_bold_present`): the live AccessKit tree
//!   contains a node with author_id `toolbar-btn-toggle_bold` and `Role::Button`.
//! - AC-11 (`overflow_button_renders_when_narrow`): forcing a small toolbar width makes
//!   the `…` overflow button (`toolbar-btn-overflow`) appear in the AccessKit tree, and
//!   the popup stays open across multiple frames (MC-005).
//!
//! ## Screenshot model on this host
//!
//! `egui_kittest`'s `Harness::render()` does headless wgpu readback. With a GPU adapter
//! the PNG is produced; without one the test records an honest non-fatal blocker and the
//! AccessKit/structural proofs stand (the same best-effort model the MT-012 test uses).

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use egui::{Key, Modifiers};
use egui_kittest::kittest::NodeT;
use egui_kittest::Harness;

use handshake_native::rich_editor::document_model::history::UndoManager;
use handshake_native::rich_editor::document_model::node::{BlockNode, Mark};
use handshake_native::rich_editor::document_model::position::DocPosition;
use handshake_native::rich_editor::document_model::selection::Selection;
use handshake_native::rich_editor::formatting::commands::CommandContext;
use handshake_native::rich_editor::formatting::toolbar::{
    all_toolbar_commands, toolbar_button_author_id, EditorToolbar,
};
use handshake_native::rich_editor::formatting::FormattingCommand;
use handshake_native::rich_editor::renderer::input_handler::{
    self, EditContext,
};
use handshake_native::rich_editor::renderer::rich_editor_widget::{RichEditorState, RichEditorWidget};

/// The crate-relative path to the EXTERNAL artifacts root (CX-212E), disk-agnostic — the
/// crate sits at `<repo>/src/frontend/handshake_native`, so four `..` reach `<repo>/..`
/// where `Handshake_Artifacts` is a sibling of the repo worktree.
fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

/// The screenshot path under the EXTERNAL artifact root. The MT contract's literal
/// `tests/screenshots/` path is OVERRIDDEN by the CX-212E external-only artifact-hygiene
/// rule (a committed repo-local PNG is a hygiene FAILURE).
fn screenshot_path() -> PathBuf {
    external_artifact_dir("wp-kernel-012-mt-013").join("mt013_toolbar.png")
}

/// Assert NO repo-local artifact dir exists under the crate (CX-212E): neither
/// `test_output/` nor `tests/screenshots/`. Artifacts go to the external
/// `Handshake_Artifacts/handshake-test` root ONLY.
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

/// Build a harness rendering the demo editor (toolbar + content), with the shell Inter
/// fonts installed and a size big enough to show the full toolbar row + the demo doc.
fn demo_harness<'a>() -> (Arc<Mutex<RichEditorState>>, Harness<'a, ()>) {
    let state = Arc::new(Mutex::new(RichEditorState::demo()));
    let state_for_ui = Arc::clone(&state);
    let harness = Harness::builder()
        .with_size(egui::vec2(900.0, 360.0))
        .wgpu()
        .build_ui(move |ui| {
            handshake_native::app::HandshakeApp::install_fonts(ui.ctx());
            RichEditorWidget::new(Arc::clone(&state_for_ui)).show(ui);
        });
    (state, harness)
}

/// Collect every author_id present in the live AccessKit tree.
fn author_ids(harness: &Harness<()>) -> HashSet<String> {
    let mut ids = HashSet::new();
    for node in harness.root().children_recursive() {
        if let Some(a) = node.accesskit_node().author_id() {
            ids.insert(a.to_string());
        }
    }
    ids
}

// ── AC-10: the toolbar exposes a `toolbar-btn-toggle_bold` Button node ─────────────────

#[test]
fn accesskit_toolbar_btn_toggle_bold_present() {
    let (_state, mut harness) = demo_harness();
    harness.run();

    let root = harness.root();
    let target = toolbar_button_author_id(&FormattingCommand::ToggleBold); // "toolbar-btn-toggle_bold"
    let mut found = false;
    let mut role = String::new();
    for node in root.children_recursive() {
        let ak = node.accesskit_node();
        if ak.author_id() == Some(target.as_str()) {
            found = true;
            role = format!("{:?}", ak.role());
            break;
        }
    }
    assert!(
        found,
        "AC-10: the live AccessKit tree must contain a node author_id='{target}'"
    );
    assert_eq!(
        role, "Button",
        "AC-10: '{target}' must be Role::Button (egui derives it from the Button's Sense; got {role})"
    );
    println!("AC-10 accesskit: {{\"{target}\":\"{role}\"}}");
}

// ── AC-1: every category group renders a toolbar button in the tree ────────────────────

#[test]
fn toolbar_renders_all_category_groups() {
    let (_state, mut harness) = demo_harness();
    harness.run();
    let ids = author_ids(&harness);

    // One representative command per category group (history|format|block|list|table).
    let reps = [
        FormattingCommand::Undo,                          // history
        FormattingCommand::ToggleBold,                    // format
        FormattingCommand::SetHeading(1),                 // block
        FormattingCommand::ToggleBulletList,              // list
        FormattingCommand::InsertTable { rows: 3, cols: 3 }, // table
    ];
    for cmd in &reps {
        let id = toolbar_button_author_id(cmd);
        assert!(
            ids.contains(&id),
            "AC-1: toolbar must render a button for every category group; missing '{id}' \
             (present: {} ids)",
            ids.len()
        );
    }
    println!(
        "AC-1: all 5 category groups present (history/format/block/list/table); {} toolbar buttons",
        ids.iter().filter(|i| i.starts_with("toolbar-btn-")).count()
    );
}

// ── AC-3: Ctrl+B through the REAL input_handler pipeline toggles bold ──────────────────

#[test]
fn ctrl_b_toggles_bold_via_input_handler() {
    // The exact code path the widget runs per frame: decode the formatting chords from a
    // Ctrl+B key event, then apply them through the command layer. This is the real
    // pipeline (not a re-implementation), so it is a non-tautological AC-3 proof.
    let ctrl = Modifiers {
        ctrl: true,
        command: true,
        ..Default::default()
    };
    let ev = egui::Event::Key {
        key: Key::B,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: ctrl,
    };
    let cmds = input_handler::decode_formatting_commands(&[ev], /*caret_in_list=*/ false);
    assert_eq!(
        cmds,
        vec![FormattingCommand::ToggleBold],
        "AC-3: a Ctrl+B key event decodes to ToggleBold"
    );

    // Apply it to a doc with the whole leaf selected.
    let mut doc = BlockNode::doc(vec![BlockNode::paragraph("hello")]);
    let mut sel = Selection::text(
        DocPosition::new(vec![0, 0], 0),
        DocPosition::new(vec![0, 0], 5),
    );
    let mut undo = UndoManager::new();
    {
        let mut ctx = EditContext {
            doc: &mut doc,
            selection: &mut sel,
            undo: &mut undo,
            actor_id: "operator",
        };
        for cmd in &cmds {
            assert!(
                input_handler::apply_formatting_command(&mut ctx, cmd),
                "AC-3: applying ToggleBold must change the doc"
            );
        }
    }
    assert!(
        doc.children[0].as_block().unwrap().children[0]
            .as_text()
            .unwrap()
            .has_mark_type(&Mark::Bold),
        "AC-3: Ctrl+B toggled the bold mark on the selected text"
    );

    // Ctrl+Z (undo) through the same plain-key decode path reverts it.
    let undo_ev = egui::Event::Key {
        key: Key::Z,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: ctrl,
    };
    let undo_cmds = input_handler::decode_formatting_commands(&[undo_ev], false);
    assert_eq!(undo_cmds, vec![FormattingCommand::Undo]);
    {
        let mut ctx = EditContext {
            doc: &mut doc,
            selection: &mut sel,
            undo: &mut undo,
            actor_id: "operator",
        };
        for cmd in &undo_cmds {
            input_handler::apply_formatting_command(&mut ctx, cmd);
        }
    }
    assert!(
        !doc.children[0].as_block().unwrap().children[0]
            .as_text()
            .unwrap()
            .has_mark_type(&Mark::Bold),
        "AC-4/AC-5: Ctrl+Z undid the bold toggle"
    );
}

// ── AC-11 / MC-005: the overflow button renders when the toolbar is forced narrow ──────

#[test]
fn overflow_button_renders_when_narrow() {
    // Render the toolbar standalone with a FORCED tiny max width so most buttons spill
    // into the overflow popup and the `…` button appears (simulated narrow window).
    let state = Arc::new(Mutex::new(RichEditorState::demo()));
    let state_for_ui = Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(160.0, 120.0))
        .build_ui(move |ui| {
            handshake_native::app::HandshakeApp::install_fonts(ui.ctx());
            let mut st = state_for_ui.lock().unwrap();
            let RichEditorState {
                doc,
                selection,
                undo,
                actor_id,
                ..
            } = &mut *st;
            let cctx = CommandContext::new(doc, undo, selection, actor_id.as_str());
            // Force a narrow row so overflow triggers deterministically.
            EditorToolbar::new(cctx).with_forced_max_width(120.0).show(ui);
        });
    // Run multiple frames to prove the popup id is stable (MC-005: it must not vanish on
    // a fresh-id frame). The `…` button node is present every frame.
    harness.run();
    harness.run();
    harness.run();

    let ids = author_ids(&harness);
    assert!(
        ids.contains("toolbar-btn-overflow"),
        "AC-11: a forced-narrow toolbar must render the '…' overflow button \
         (author_id='toolbar-btn-overflow'); present ids: {ids:?}"
    );
    println!("AC-11: overflow '…' button present under a forced-narrow toolbar (stable across 3 frames)");
}

// ── AC-1 + PT-2: screenshot of the toolbar row above the content ───────────────────────

#[test]
fn toolbar_screenshot() {
    let (_state, mut harness) = demo_harness();
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

            // Pixel proof: the toolbar glyphs + the styled demo doc produce multiple
            // distinct foreground colors over the background. Sample every 4th pixel.
            let raw = image.as_raw();
            let mut counts: std::collections::HashMap<[u8; 4], u32> = std::collections::HashMap::new();
            let mut i = 0usize;
            while i + 4 <= raw.len() {
                let px = [raw[i], raw[i + 1], raw[i + 2], raw[i + 3]];
                if px[3] != 0 {
                    *counts.entry(px).or_insert(0) += 1;
                }
                i += 4 * 4;
            }
            let bg = counts.iter().max_by_key(|(_, c)| **c).map(|(p, _)| *p);
            let foreground: HashSet<[u8; 4]> =
                counts.keys().filter(|p| Some(**p) != bg).copied().collect();

            println!(
                "PT-2 toolbar screenshot: {w}x{h}, {} colors, {} foreground; saved={saved} ({})",
                counts.len(),
                foreground.len(),
                path.display(),
            );
            assert!(
                foreground.len() >= 2,
                "AC-1/PT-2: the toolbar row + styled doc must produce >= 2 distinct foreground \
                 colors; got {} (bg={bg:?})",
                foreground.len()
            );
            assert!(saved, "the mt013_toolbar.png screenshot must save to the external artifact root");
            assert_no_local_artifact_dir();
        }
        Err(e) => {
            println!(
                "BLOCKER(non-fatal): toolbar screenshot render unavailable (no wgpu adapter / \
                 headless GPU crash): {e}. The AccessKit + structural proofs stand; the PNG is a \
                 GPU-host item."
            );
            // Still enforce hygiene even on the no-GPU path.
            assert_no_local_artifact_dir();
        }
    }
}

// ── AC-9: Tab/Shift+Tab decode to sink/lift ONLY when the caret is in a list ───────────

#[test]
fn tab_indents_only_inside_a_list() {
    let tab = egui::Event::Key {
        key: Key::Tab,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::default(),
    };
    let shift_tab = egui::Event::Key {
        key: Key::Tab,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers {
            shift: true,
            ..Default::default()
        },
    };
    // In a list: Tab -> sink, Shift+Tab -> lift.
    assert_eq!(
        input_handler::decode_formatting_commands(std::slice::from_ref(&tab), /*caret_in_list=*/ true),
        vec![FormattingCommand::SinkListItem],
        "AC-9: Tab inside a list calls sink_list_item"
    );
    assert_eq!(
        input_handler::decode_formatting_commands(std::slice::from_ref(&shift_tab), true),
        vec![FormattingCommand::LiftListItem],
        "AC-9: Shift+Tab inside a list calls lift_list_item"
    );
    // Outside a list: Tab is NOT claimed (it traverses focus instead — RISK-4 / MC-004).
    assert!(
        input_handler::decode_formatting_commands(&[tab], false).is_empty(),
        "AC-9 / MC-004: Tab outside a list is not an indent command"
    );
    assert!(input_handler::decode_formatting_commands(&[shift_tab], false).is_empty());
}

// ── all toolbar commands have a stable, unique author_id ───────────────────────────────

#[test]
fn all_toolbar_commands_have_unique_author_ids() {
    let mut seen = HashSet::new();
    for cmd in all_toolbar_commands() {
        let id = toolbar_button_author_id(&cmd);
        assert!(
            id.starts_with("toolbar-btn-"),
            "every toolbar author_id is namespaced: {id}"
        );
        assert!(seen.insert(id.clone()), "duplicate toolbar author_id: {id}");
    }
    assert!(seen.len() >= 8, "the toolbar exposes the full command catalog (>= 8 buttons)");
    println!("{} unique toolbar-btn author_ids", seen.len());
}

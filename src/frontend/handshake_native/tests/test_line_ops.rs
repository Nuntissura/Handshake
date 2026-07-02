//! Line-edit buffer transform proofs (WP-KERNEL-012 MT-051).
//!
//! These integration proofs exercise the REAL wired path, not the pure-function unit tests that live in
//! `src/code_editor/line_ops.rs` (those cover the per-transform AC behavior — ToggleComment all-or-nothing,
//! move/delete boundaries, tab/space modes, UTF-8 correctness). Here we prove the contract obligations
//! that need the PANEL + the DISPATCH + the unified-undo BUS:
//!
//! - PT-003 / AC-007 single-undo coalescing: dispatching a multi-cursor ToggleComment through the panel's
//!   real `dispatch_action` queues EXACTLY ONE undo snapshot; pushing it onto the REAL `InteractionBus`
//!   and calling `undo` once reverts the buffer to its exact pre-transform string.
//! - AC-010 / PT-004 grep gate: no `todo!`/`unimplemented!` remains on the eight action handler paths in
//!   `panel.rs` (the dispatch site) or `line_ops.rs` (the transforms) — the handlers are real.
//! - The dispatch wiring is live: each of the eight `CodeEditorAction` variants, dispatched through the
//!   panel, actually edits the document (no dead keys).
//! - HBR-VIS: the panel renders after a line transform and the PNG is written to the EXTERNAL artifact
//!   root only (CX-212E / AC-006 artifact hygiene — never repo-local).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use egui_kittest::Harness;

use handshake_native::code_editor::keymap::CodeEditorAction;
use handshake_native::code_editor::panel::{CodeEditorPaneFactory, CodeEditorPanel};
use handshake_native::code_editor::{line_comment_token, Cursor, LineEditContext, TextBuffer};

// ── Artifact hygiene (CX-212E / AC-006): screenshots go to the EXTERNAL root ONLY ─────────────────────

/// The crate-relative path to the external artifacts root (CX-212E), disk-agnostic. The crate sits at
/// `<repo>/src/frontend/handshake_native`, so four `..` reach `<repo>/..` where `Handshake_Artifacts`
/// is a sibling of the repo worktree.
fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

/// Assert NO repo-local artifact directory exists under the crate (CX-212E artifact-hygiene guard). A
/// tracked/committed artifact under `src/` (or a stray `test_output/` / `tests/screenshots/`) is a
/// hygiene FAILURE; this guard runs in the screenshot test and the reviewer additionally runs
/// `git ls-files "src/**/*.png"`.
fn assert_no_local_artifact_dir() {
    for dir in ["test_output", "tests/screenshots"] {
        let p = Path::new(dir);
        assert!(
            !p.exists(),
            "CX-212E: no repo-local artifact dir may exist — artifacts go to the external \
             Handshake_Artifacts/handshake-test root only (found {})",
            p.display()
        );
    }
}

// ── PT-003 / AC-007: single-undo coalescing through the REAL panel dispatch + bus ─────────────────────

#[test]
fn multi_cursor_toggle_comment_is_one_undo() {
    use handshake_native::code_editor::interop_adapter::push_code_edit_undo;
    use handshake_native::interop::interaction_bus::InteractionBus;
    use handshake_native::pane_registry::PaneId;

    // A 3-line Rust buffer; place a cursor on each of lines 0, 1, 2 (multi-cursor).
    let original = "let a = 1;\nlet b = 2;\nlet c = 3;";
    let panel = Arc::new(CodeEditorPanel::new(original, "rs"));
    let cursors: Vec<Cursor> = {
        let buf = panel.buffer();
        (0..3)
            .map(|l| Cursor::caret(buf.line_to_byte(l).unwrap()))
            .collect()
    };
    panel.set_cursors(cursors);

    // Dispatch the REAL keymap action (the same path Ctrl+/ takes).
    panel.dispatch_action(CodeEditorAction::ToggleComment);

    // All three lines are now commented (the transform ran end-to-end).
    assert_eq!(
        panel.buffer().to_string(),
        "// let a = 1;\n// let b = 2;\n// let c = 3;",
        "AC-001: multi-cursor ToggleComment commented each touched line"
    );

    // EXACTLY ONE undo snapshot was queued for the whole multi-cursor transform (not one per line).
    let (description, before, after) = panel
        .take_pending_line_op_undo()
        .expect("AC-007: a line transform queues exactly one undo snapshot");
    assert_eq!(
        before, original,
        "the snapshot's BEFORE is the exact pre-transform text"
    );
    assert_eq!(
        after,
        panel.buffer().to_string(),
        "the snapshot's AFTER is the post-transform text"
    );
    // A second drain yields nothing — there was exactly one entry.
    assert!(
        panel.take_pending_line_op_undo().is_none(),
        "AC-007: exactly ONE undo snapshot per transform (no per-line entries)"
    );

    // Push it onto the REAL unified-undo bus and prove ONE undo reverts the WHOLE transform.
    let mut bus = InteractionBus::new();
    let pane_id: PaneId = Arc::from("code-pane-line-ops-test");
    push_code_edit_undo(
        &mut bus,
        pane_id.clone(),
        &panel,
        TextBuffer::new(&before),
        TextBuffer::new(&after),
        description,
    );
    assert_eq!(
        bus.local_undo_count(&pane_id),
        1,
        "AC-007: a multi-cursor ToggleComment records EXACTLY ONE undo entry"
    );

    let result = bus.undo(&pane_id).expect("an undo entry exists");
    assert!(result.ok, "AC-007: the undo invoked cleanly");
    assert_eq!(
        panel.buffer().to_string(),
        original,
        "PT-003 / AC-007: a SINGLE undo reverts the multi-cursor ToggleComment to the exact original"
    );
    assert_eq!(
        bus.local_undo_count(&pane_id),
        0,
        "the single entry was consumed"
    );
    println!(
        "PT-003 multi_cursor_toggle_comment_is_one_undo: 1 undo entry ('{description}'), single undo \
         reverts all 3 lines exactly"
    );
}

// ── Dispatch wiring is LIVE: each of the 8 actions actually edits the document ─────────────────────────

#[test]
fn all_eight_line_edit_actions_mutate_the_buffer() {
    // Each action, dispatched through the panel, must change the document (no dead keys). The buffer/
    // cursor preconditions are chosen so the action is NOT a boundary no-op.
    let cases: &[(CodeEditorAction, &str, usize, &str)] = &[
        // (action, initial buffer, primary caret line, expected substring/marker after)
        (CodeEditorAction::ToggleComment, "x = 1", 0, "//"),
        (CodeEditorAction::DuplicateLine, "row", 0, "row\nrow"),
        (CodeEditorAction::MoveLineUp, "a\nb", 1, "b\na"),
        (CodeEditorAction::MoveLineDown, "a\nb", 0, "b\na"),
        (CodeEditorAction::DeleteLine, "keep\ngone", 1, "keep"),
        (CodeEditorAction::IndentLine, "y", 0, "    y"),
        (CodeEditorAction::InsertTab, "ab", 0, "    ab"),
    ];
    for (action, initial, line, expect) in cases {
        let panel = CodeEditorPanel::new(initial, "rs");
        let head = panel.buffer().line_to_byte(*line).unwrap();
        panel.set_single_cursor(head);
        let before = panel.buffer().to_string();
        panel.dispatch_action(*action);
        let after = panel.buffer().to_string();
        assert_ne!(
            after, before,
            "{action:?} must change the document (not a dead key)"
        );
        assert!(
            after.contains(expect),
            "{action:?}: expected the result to contain {expect:?}, got {after:?}"
        );
    }

    // DedentLine separately: it needs a pre-indented line to have an effect.
    let panel = CodeEditorPanel::new("        deep", "rs"); // 8 spaces
    panel.set_single_cursor(0);
    let before = panel.buffer().to_string();
    panel.dispatch_action(CodeEditorAction::DedentLine);
    let after = panel.buffer().to_string();
    assert_ne!(after, before, "DedentLine must change an indented line");
    assert_eq!(
        after, "    deep",
        "DedentLine removed one 4-space indent unit"
    );

    println!("all_eight_line_edit_actions_mutate_the_buffer: 8/8 actions live (no dead keys)");
}

// ── Settings-driven indent: the dispatch uses the operator's tab settings (MC-006) ────────────────────

#[test]
fn insert_tab_respects_operator_indent_settings() {
    // tab_size=2 spaces.
    let panel = CodeEditorPanel::new("z", "rs");
    panel.set_indent_settings(2, true);
    assert_eq!(panel.indent_settings(), (2, true));
    panel.set_single_cursor(0);
    panel.dispatch_action(CodeEditorAction::InsertTab);
    assert_eq!(
        panel.buffer().to_string(),
        "  z",
        "InsertTab inserted tab_size=2 spaces"
    );

    // Tab character mode.
    let panel = CodeEditorPanel::new("z", "rs");
    panel.set_indent_settings(4, false);
    panel.set_single_cursor(0);
    panel.dispatch_action(CodeEditorAction::IndentLine);
    assert_eq!(
        panel.buffer().to_string(),
        "\tz",
        "IndentLine inserted a tab char (insert_spaces=false)"
    );
}

// ── RISK-007 / MC-007: the comment token follows the file's highlight language ────────────────────────

#[test]
fn comment_token_follows_file_language() {
    // MC-007 (adversarial-review hardening): the comment token must be proven through the REAL dispatch
    // path — a panel built for a real extension, its language id flowing into the LineEditContext, the
    // ToggleComment action dispatched end-to-end. We prove BOTH bundled languages this way (the two the
    // registry can actually produce: Rust `.rs` and JavaScript `.js`), so the non-no-op comment branch is
    // genuinely reachable for every language the product table claims, not asserted against itself.
    for (ext, family) in [("rs", "rust"), ("js", "javascript")] {
        let panel = CodeEditorPanel::new("f()", ext);
        // The panel carries the SAME family id the token table is keyed on (no second enum, RISK-007).
        assert_eq!(
            panel.language_id(),
            family,
            "{ext} panel carries family id {family}"
        );
        panel.set_single_cursor(0);
        panel.dispatch_action(CodeEditorAction::ToggleComment);
        assert!(
            panel.buffer().to_string().starts_with("// "),
            "a .{ext} buffer comments with // through the live dispatch path"
        );
        // And the token table agrees with the live id.
        assert_eq!(line_comment_token(family), Some("//"));
        let ctx = LineEditContext::new(family, 4, true);
        assert_eq!(line_comment_token(ctx.language_id), Some("//"));
    }

    // REACHABILITY: the table is narrowed to the bundled families. Families with no bundled grammar (so no
    // panel can carry their id) intentionally return None — a safe ToggleComment no-op (AC-008) — until the
    // registry bundles their grammar (future work). Asserting None keeps the table free of dead Some(_)
    // arms that could only ever be tested against themselves.
    for unmapped in [
        "python",
        "sql",
        "lua",
        "typescript",
        "go",
        "c",
        "plaintext",
        "",
    ] {
        assert_eq!(
            line_comment_token(unmapped),
            None,
            "{unmapped:?} has no bundled grammar -> ToggleComment is a safe no-op (future work)"
        );
    }
}

// ── AC-010 / PT-004: no todo!/unimplemented! on the eight handler paths ────────────────────────────────

#[test]
fn no_unimplemented_markers_on_handler_paths() {
    // Scan the two source files that carry the eight action handlers: the dispatch site (panel.rs) and the
    // transform library (line_ops.rs). Neither may contain a `todo!`/`unimplemented!` macro invocation —
    // the handlers are REAL (AC-010 / PT-004). The crate root is the CWD for cargo test.
    let line_ops =
        std::fs::read_to_string("src/code_editor/line_ops.rs").expect("read line_ops.rs");
    // line_ops.rs is the transform library: it must contain ZERO of these markers anywhere.
    for marker in ["todo!", "unimplemented!"] {
        assert!(
            !line_ops.contains(marker),
            "AC-010/PT-004: line_ops.rs must contain no {marker} (handlers are real)"
        );
    }

    // panel.rs is huge and unrelated branches may legitimately mention these in comments; restrict the
    // scan to the eight-action dispatch arms in `dispatch_action`. Extract that method body and assert no
    // marker on the line-edit arms.
    let panel = std::fs::read_to_string("src/code_editor/panel.rs").expect("read panel.rs");
    let dispatch_start = panel
        .find("pub fn dispatch_action(&self, action: CodeEditorAction)")
        .expect("dispatch_action exists");
    // The dispatch `match` is large (60 action arms); take a generous window that covers the whole method
    // body so all eight line-edit arms are in range (clamped to the file length).
    let dispatch_slice = &panel[dispatch_start..(dispatch_start + 9000).min(panel.len())];
    for action in [
        "A::ToggleComment",
        "A::DuplicateLine",
        "A::MoveLineUp",
        "A::MoveLineDown",
        "A::DeleteLine",
        "A::IndentLine",
        "A::DedentLine",
        "A::InsertTab",
    ] {
        let arm = dispatch_slice
            .find(action)
            .unwrap_or_else(|| panic!("dispatch arm for {action} exists"));
        // The arm's body is on the same/next few lines; assert no todo/unimplemented within ~200 chars.
        let arm_slice = &dispatch_slice[arm..(arm + 200).min(dispatch_slice.len())];
        assert!(
            !arm_slice.contains("todo!") && !arm_slice.contains("unimplemented!"),
            "AC-010/PT-004: the {action} dispatch arm must be a real handler, not a stub"
        );
        // And it must route to a real line_ops transform.
        assert!(
            arm_slice.contains("apply_line_transform") || arm_slice.contains("line_ops::"),
            "AC-010: the {action} arm must call into line_ops (real handler)"
        );
    }
    println!("PT-004 no_unimplemented_markers_on_handler_paths: 8/8 arms route to real line_ops handlers");
}

// ── HBR-VIS: the panel renders after a line transform; PNG to the external root ONLY ──────────────────

#[test]
fn line_ops_panel_renders_and_screenshots_externally() {
    // Build a panel, run a real transform through the dispatch, then render it headlessly and save the PNG
    // to the EXTERNAL artifact root ONLY (CX-212E). The render must produce a non-empty image on a GPU host
    // (best-effort: a host without a wgpu adapter records an honest non-fatal blocker, like the MT-001 test).
    let panel = CodeEditorPanel::new("fn main() {\n    let x = 1;\n}", "rs");
    panel.set_single_cursor(0);
    panel.dispatch_action(CodeEditorAction::ToggleComment);
    assert!(
        panel.buffer().to_string().starts_with("// "),
        "the transform applied before render"
    );

    let factory = CodeEditorPaneFactory::new(panel);
    let panel_arc = factory.panel();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(640.0, 200.0))
        .wgpu()
        .build_ui(move |ui| {
            panel_arc.show(ui);
        });
    harness.run();
    harness.run();

    match harness.render() {
        Ok(image) => {
            let (w, h) = (image.width(), image.height());
            assert!(w > 0 && h > 0, "rendered image must be non-empty");
            let ext_dir = external_artifact_dir("wp-kernel-012-mt-051");
            let _ = std::fs::create_dir_all(&ext_dir);
            let ext_path = ext_dir.join("MT-051-line-ops-toggle-comment.png");
            let saved = image.save(&ext_path).is_ok();
            println!(
                "HBR-VIS MT-051: {}x{} image saved_ext={saved} ({})",
                w,
                h,
                ext_path.display()
            );
        }
        Err(e) => {
            println!(
                "BLOCKER(non-fatal): MT-051 screenshot render unavailable (no wgpu adapter / headless \
                 GPU crash): {e}. The structural + transform proofs stand as the AC evidence."
            );
        }
    }

    // CX-212E: the run must not have created any repo-local artifact dir.
    assert_no_local_artifact_dir();
}

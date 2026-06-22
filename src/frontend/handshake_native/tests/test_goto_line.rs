//! MT-006 go-to-line palette proofs (WP-KERNEL-012 — E1 code editor).
//!
//! - AC-002 / PT-002 (`cargo test -p handshake-native goto_line`): go-to-line with a valid line
//!   navigates; go-to-line clamped past the end lands on the last line; go-to-line with non-numeric
//!   input does NOT navigate and does NOT crash. Driven through the public palette surface
//!   (`open_goto_line` / `set_goto_line_input` / `submit_goto_line`) — the same surface the modal +
//!   the Ctrl+G keymap drive.
//! - AC-005 / PT-005 (`goto_line_modal`): an egui_kittest test — Ctrl+G opens the modal (verified via
//!   the live AccessKit `code_editor_goto_line` `Role::TextInput` node), then typing `5` + submitting
//!   scrolls the editor to line 5.

use std::sync::Arc;

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;

use handshake_native::code_editor::{CodeEditorPanel, CODE_EDITOR_GOTO_LINE_AUTHOR_ID};

/// A multi-line buffer (line numbers `line0`..`line29`, 1-based 1..30) so navigation is observable.
fn numbered_buffer(lines: usize) -> String {
    (0..lines).map(|i| format!("line{i}\n")).collect()
}

// ── AC-002 / PT-002: valid / clamped / non-numeric go-to-line ─────────────────────────────────────

#[test]
fn goto_line_valid_line_navigates() {
    let panel = CodeEditorPanel::new(&numbered_buffer(30), "txt");
    panel.open_goto_line();
    assert!(panel.is_goto_line_open(), "palette opens");

    // Type the 1-based line "10" and submit -> 0-based buffer line 9.
    panel.set_goto_line_input("10");
    let navigated = panel.submit_goto_line();
    assert!(navigated, "AC-002: a valid numeric line navigates");
    assert!(!panel.is_goto_line_open(), "a successful jump closes the palette");

    // The caret moved to the start of 0-based line 9.
    let buffer = panel.buffer();
    let cursors = panel.cursors();
    let (caret_line, caret_col) =
        handshake_native::code_editor::byte_to_line_col(cursors.primary().head, &buffer);
    assert_eq!(caret_line, 9, "AC-002: 1-based line 10 -> 0-based caret line 9");
    assert_eq!(caret_col, 0, "caret sits at the start of the target line");
}

#[test]
fn goto_line_clamps_past_end_to_last_line() {
    let panel = CodeEditorPanel::new(&numbered_buffer(30), "txt");
    let last_line = panel.buffer().len_lines().saturating_sub(1);
    panel.open_goto_line();

    // 1-based line 9999 is far past the end -> clamps to the last buffer line (no crash, AC-002).
    panel.set_goto_line_input("9999");
    let navigated = panel.submit_goto_line();
    assert!(navigated, "AC-002: a too-large line still navigates (clamped)");

    let buffer = panel.buffer();
    let cursors = panel.cursors();
    let (caret_line, _) =
        handshake_native::code_editor::byte_to_line_col(cursors.primary().head, &buffer);
    assert_eq!(caret_line, last_line, "AC-002: a past-the-end line clamps to the last line");
}

#[test]
fn goto_line_zero_and_negative_clamp_to_first_line() {
    let panel = CodeEditorPanel::new(&numbered_buffer(30), "txt");
    // Start the caret somewhere in the middle so a clamp-to-top is observable.
    panel.navigate_to_line(15);

    // "0" (RISK-003: must clamp to line 1 / 0-based 0, never panic).
    panel.open_goto_line();
    panel.set_goto_line_input("0");
    assert!(panel.submit_goto_line(), "AC-002: '0' clamps to line 1 and navigates");
    let (caret_line, _) = {
        let buffer = panel.buffer();
        handshake_native::code_editor::byte_to_line_col(panel.cursors().primary().head, &buffer)
    };
    assert_eq!(caret_line, 0, "AC-002: 1-based '0' clamps to 0-based line 0");

    // "-5" (RISK-003: negative clamps to line 0, never panics).
    panel.navigate_to_line(15);
    panel.open_goto_line();
    panel.set_goto_line_input("-5");
    assert!(panel.submit_goto_line(), "AC-002: a negative line clamps to line 1 and navigates");
    let (caret_line, _) = {
        let buffer = panel.buffer();
        handshake_native::code_editor::byte_to_line_col(panel.cursors().primary().head, &buffer)
    };
    assert_eq!(caret_line, 0, "AC-002: '-5' clamps to 0-based line 0");
}

#[test]
fn goto_line_non_numeric_does_not_navigate_or_crash() {
    let panel = CodeEditorPanel::new(&numbered_buffer(30), "txt");
    // Move the caret to a known line first so we can prove it does NOT move on bad input.
    panel.navigate_to_line(7);
    let before = {
        let buffer = panel.buffer();
        handshake_native::code_editor::byte_to_line_col(panel.cursors().primary().head, &buffer).0
    };
    assert_eq!(before, 7);

    panel.open_goto_line();
    panel.set_goto_line_input("not-a-number");
    let navigated = panel.submit_goto_line();
    assert!(!navigated, "AC-002: non-numeric input does NOT navigate (no crash)");
    assert!(
        panel.is_goto_line_open(),
        "AC-002: the palette stays open on invalid input so the user can correct it"
    );

    // The caret did not move.
    let after = {
        let buffer = panel.buffer();
        handshake_native::code_editor::byte_to_line_col(panel.cursors().primary().head, &buffer).0
    };
    assert_eq!(after, before, "AC-002: a bad go-to-line leaves the caret where it was");

    // An empty input likewise does not navigate.
    panel.set_goto_line_input("");
    assert!(!panel.submit_goto_line(), "AC-002: empty input does not navigate");
    println!("PT-002 goto_line: valid/clamped/zero/negative/non-numeric all handled without panic");
}

// ── AC-005 / PT-005: goto_line_modal — Ctrl+G opens; typing 5 + Enter scrolls to line 5 ───────────

#[test]
fn goto_line_modal_ctrl_g_opens_and_navigates() {
    // A tall file so scrolling to line 5 vs the start is observable as a caret move on a known line.
    let panel = Arc::new(CodeEditorPanel::new(&numbered_buffer(200), "txt"));
    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 320.0))
        .build_ui(move |ui| {
            panel_ui.show(ui);
        });

    // Frame 1: render closed — no goto-line node yet (AC-005: no node when closed).
    harness.run();
    let root = harness.root();
    let closed_has_node = root
        .children_recursive()
        .any(|n| n.accesskit_node().author_id() == Some(CODE_EDITOR_GOTO_LINE_AUTHOR_ID));
    assert!(!closed_has_node, "AC-005: no go-to-line node while the palette is closed");

    // Inject Ctrl+G (the keymap reads modifiers off the Key event itself, like the find Ctrl+F path).
    let ctrl = egui::Modifiers { ctrl: true, ..Default::default() };
    harness.event(egui::Event::Key {
        key: egui::Key::G,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: ctrl,
    });
    harness.run();
    harness.run();

    assert!(panel.is_goto_line_open(), "AC-005: Ctrl+G opened the go-to-line palette");

    // AC-005: the live AccessKit tree now contains code_editor_goto_line with Role::TextInput.
    let root = harness.root();
    let mut goto_role: Option<String> = None;
    for node in root.children_recursive() {
        let ak = node.accesskit_node();
        if ak.author_id() == Some(CODE_EDITOR_GOTO_LINE_AUTHOR_ID) {
            goto_role = Some(format!("{:?}", ak.role()));
        }
    }
    assert_eq!(
        goto_role.as_deref(),
        Some("TextInput"),
        "AC-005: the open palette emits code_editor_goto_line with Role::TextInput; got {goto_role:?}"
    );
    assert!(
        harness.query_all_by_label("Code editor go to line").count() >= 1,
        "AC-005: the go-to-line node is labeled/addressable"
    );

    // Type "5" and submit (the Enter keymap routes through submit_goto_line). The modal input is owned
    // state, so push the typed value through the public setter (the same path the modal's TextEdit
    // change pushes) then submit, exactly as the keymap's Enter arm does.
    panel.set_goto_line_input("5");
    harness.run();
    harness.event(egui::Event::Key {
        key: egui::Key::Enter,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::default(),
    });
    harness.run();
    harness.run();

    assert!(!panel.is_goto_line_open(), "AC-005: submitting closed the palette");

    // 1-based line 5 -> 0-based caret line 4; the editor scrolled so the line is in the painted window.
    let buffer = panel.buffer();
    let (caret_line, _) =
        handshake_native::code_editor::byte_to_line_col(panel.cursors().primary().head, &buffer);
    assert_eq!(caret_line, 4, "AC-005: typing 5 + Enter moved the caret to line 5 (0-based 4)");
    let painted = panel.last_visible_range();
    assert!(
        painted.contains(&4),
        "AC-005: the editor scrolled so line 5 (0-based 4) is in the painted window {painted:?}"
    );
    println!("PT-005 goto_line_modal: Ctrl+G opened, typed 5, caret line {caret_line}, painted {painted:?}");
}

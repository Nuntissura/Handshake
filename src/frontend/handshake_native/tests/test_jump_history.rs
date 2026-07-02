//! MT-052 jump-history proofs (WP-KERNEL-012 — E1 GO-menu Navigate Back / Forward).
//!
//! - AC-003 / PT-003 (`jump_history_*`): push/back/forward match VS Code — after a jump, Navigate Back
//!   restores the prior file_path + position; a NEW jump after a Back truncates the forward tail.
//! - AC-004 / PT-004 (`jump_history_cross_file`): the stack retains the SOURCE file path; Back returns an
//!   entry whose file_path is the source file, not the currently focused file.
//! - MC-005 (`navigate_back_to_missing_file_is_graceful`): a Navigate Back into a DIFFERENT file does not
//!   move the caret in the wrong file and does not panic — it parks a pending cross-file target.
//! - MC-006 (`only_navigation_jumps_record`): ordinary typing / arrow caret moves do NOT record a jump;
//!   only navigation jumps (goto-line here) do — proven via the live panel.

use std::path::PathBuf;
use std::sync::Arc;

use handshake_native::code_editor::{
    BufferPosition, CodeEditorAction, CodeEditorPanel, JumpEntry, JumpHistory,
};

fn pos(line: usize) -> BufferPosition {
    BufferPosition::new(line, 0)
}

fn entry(path: &str, line: usize) -> JumpEntry {
    JumpEntry::new(path, pos(line))
}

fn numbered_buffer(lines: usize) -> String {
    (0..lines).map(|i| format!("line{i}\n")).collect()
}

fn caret_line(panel: &CodeEditorPanel) -> usize {
    let buffer = panel.buffer();
    let cursors = panel.cursors();
    handshake_native::code_editor::byte_to_line_col(cursors.primary().head, &buffer).0
}

// ── AC-003 / PT-003: push / back / forward + forward-tail truncation ───────────────────────────────

#[test]
fn jump_history_back_restores_prior_position() {
    let mut h = JumpHistory::new();
    // The user was at line 10 and performed a goto-def jump (record the PRE-jump origin), then is now at
    // line 80. Navigate Back restores line 10.
    h.record(entry("a.rs", 10));
    let back = h.back(entry("a.rs", 80));
    assert_eq!(
        back,
        Some(entry("a.rs", 10)),
        "Navigate Back restores the prior position"
    );
}

#[test]
fn jump_history_new_jump_after_back_truncates_forward_tail() {
    // RISK-004 / MC-004: jump origins 10, 20, 30; Back twice; a new jump truncates the stale forward
    // entries so Forward cannot reach 30.
    let mut h = JumpHistory::new();
    h.record(entry("a.rs", 10));
    h.record(entry("a.rs", 20));
    h.record(entry("a.rs", 30));
    assert_eq!(h.back(entry("a.rs", 99)), Some(entry("a.rs", 30)));
    assert_eq!(h.back(entry("a.rs", 30)), Some(entry("a.rs", 20)));
    assert!(h.can_forward(), "forward tail present after Backs");
    h.record(entry("b.rs", 5)); // NEW jump.
    assert!(!h.can_forward(), "the stale forward tail was truncated");
    assert_eq!(h.forward(), None);
}

#[test]
fn jump_history_forward_returns_to_origin() {
    let mut h = JumpHistory::new();
    h.record(entry("a.rs", 10));
    assert_eq!(h.back(entry("a.rs", 80)), Some(entry("a.rs", 10)));
    assert_eq!(
        h.forward(),
        Some(entry("a.rs", 80)),
        "Forward returns to where Back left from"
    );
}

// ── AC-004 / PT-004: cross-file path retention ─────────────────────────────────────────────────────

#[test]
fn jump_history_cross_file() {
    // AC-004 / RISK-005: record a jump FROM file A; the user is now in file B; Navigate Back returns an
    // entry whose file_path == A (NOT the currently focused B).
    let mut h = JumpHistory::new();
    h.record(entry("src/a.rs", 42));
    let restored = h
        .back(entry("src/b.rs", 7))
        .expect("Back returns the source entry");
    assert_eq!(
        restored.file_path,
        PathBuf::from("src/a.rs"),
        "cross-file Back retains the SOURCE file path, not the focused file"
    );
    assert_eq!(restored.position, pos(42));
}

// ── MC-005: cross-file Navigate Back is graceful (no caret in the wrong file, no panic) ─────────────

#[test]
fn navigate_back_to_missing_file_is_graceful() {
    // The panel currently shows "current.rs". A jump origin was recorded in a DIFFERENT file. Navigate
    // Back must NOT move the caret in current.rs (RISK-005) — it parks a pending cross-file target for
    // the host to open, and never panics.
    let panel = Arc::new(CodeEditorPanel::new(&numbered_buffer(30), "rs"));
    panel.set_file_path("current.rs");
    // Seed the history with an origin in another file, then move the panel caret somewhere.
    panel.record_jump_origin_for_test(JumpEntry::new("other.rs", pos(12)));
    panel.set_single_cursor(0);
    let before = caret_line(&panel);

    panel.dispatch_action(CodeEditorAction::NavigateBack);

    assert_eq!(
        caret_line(&panel),
        before,
        "caret in the current file is NOT moved by a cross-file Back"
    );
    let pending = panel
        .pending_cross_file_jump()
        .expect("a cross-file target was parked");
    assert_eq!(pending.file_path, PathBuf::from("other.rs"));
    assert_eq!(
        pending.position,
        pos(12),
        "the parked target carries the source line for the host"
    );
}

// ── MC-006: only navigation jumps record (not typing/arrow moves) ──────────────────────────────────

#[test]
fn only_navigation_jumps_record_through_the_panel() {
    let panel = Arc::new(CodeEditorPanel::new(&numbered_buffer(30), "rs"));
    panel.set_file_path("current.rs");

    // Ordinary caret moves (arrow keys) must NOT record a jump.
    panel.dispatch_action(CodeEditorAction::MoveCursorDown);
    panel.dispatch_action(CodeEditorAction::MoveCursorDown);
    panel.dispatch_action(CodeEditorAction::MoveCursorRight);
    assert!(
        !panel.can_navigate_back(),
        "arrow caret moves must not record a jump (RISK-006)"
    );

    // A navigation jump (goto-line) DOES record. Open the palette, target line 20, submit.
    panel.open_goto_line();
    panel.set_goto_line_input("20");
    assert!(panel.submit_goto_line(), "goto-line navigated");
    assert!(
        panel.can_navigate_back(),
        "a goto-line navigation recorded a back entry"
    );

    // Navigate Back returns to the pre-jump line (line 2, where the two MoveCursorDown left the caret).
    panel.dispatch_action(CodeEditorAction::NavigateBack);
    assert_eq!(
        caret_line(&panel),
        2,
        "Navigate Back restores the pre-goto-line caret line"
    );
    // Forward returns to line 19 (the goto-line target, 1-based 20 -> 0-based 19).
    panel.dispatch_action(CodeEditorAction::NavigateForward);
    assert_eq!(
        caret_line(&panel),
        19,
        "Navigate Forward returns to the goto-line target"
    );
}

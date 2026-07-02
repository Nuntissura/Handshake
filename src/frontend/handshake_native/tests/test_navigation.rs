//! MT-052 diagnostic-traversal proofs (WP-KERNEL-012 — E1 GO-menu navigation).
//!
//! Maps the navigation acceptance criteria to runtime proofs against the REAL public API + the REAL
//! `CodeEditorPanel` dispatch path (no stubs, no tautologies):
//!
//! - AC-001 / PT-001 (`next_diagnostic_*`): `next_diagnostic` returns the first marker strictly after
//!   the cursor and wraps at the end; empty list -> None.
//! - AC-002 / PT-002 (`prev_diagnostic_*`): the symmetric `prev_diagnostic` behavior.
//! - AC-008 (`f8_dispatch_moves_caret_to_next_diagnostic`): dispatching `GoToNextDiagnostic` through the
//!   live panel (the SAME dispatch path F8 takes) moves the caret to the next diagnostic line and wraps,
//!   and records a jump-history entry so Navigate Back is armed.
//!
//! Type-reuse proof (RISK-001 / MC-001): this test imports the MT-007 `GutterMarker` +
//! `DiagnosticSeverity` types and the MT-052 `BufferPosition` directly from the crate — there is no
//! parallel marker/position type to import, so a parallel redefinition would fail to compile here.

use std::sync::Arc;

use handshake_native::code_editor::{
    next_diagnostic, prev_diagnostic, BufferPosition, CodeEditorAction, CodeEditorPanel,
    DiagnosticSeverity, GutterMarker,
};

fn diag(line: usize) -> GutterMarker {
    GutterMarker::diagnostic(line, DiagnosticSeverity::Error, "boom")
}

fn pos(line: usize) -> BufferPosition {
    BufferPosition::new(line, 0)
}

fn numbered_buffer(lines: usize) -> String {
    (0..lines).map(|i| format!("line{i}\n")).collect()
}

/// Read the panel's primary caret 0-based line.
fn caret_line(panel: &CodeEditorPanel) -> usize {
    let buffer = panel.buffer();
    let cursors = panel.cursors();
    handshake_native::code_editor::byte_to_line_col(cursors.primary().head, &buffer).0
}

// ── AC-001 / PT-001: next_diagnostic ───────────────────────────────────────────────────────────────

#[test]
fn next_diagnostic_first_after_cursor_then_wraps() {
    let markers = [diag(5), diag(10), diag(20)];
    // First marker strictly after the cursor.
    assert_eq!(next_diagnostic(&markers, pos(0)), Some(pos(5)));
    assert_eq!(next_diagnostic(&markers, pos(7)), Some(pos(10)));
    // On a marker -> the next (strict), never the same one (RISK-002).
    assert_eq!(next_diagnostic(&markers, pos(10)), Some(pos(20)));
    // At/past the last -> wrap to the first.
    assert_eq!(next_diagnostic(&markers, pos(20)), Some(pos(5)));
    assert_eq!(next_diagnostic(&markers, pos(99)), Some(pos(5)));
}

#[test]
fn next_diagnostic_empty_list_is_none() {
    // RISK-003: empty marker list returns None (no panic on index 0).
    let markers: [GutterMarker; 0] = [];
    assert_eq!(next_diagnostic(&markers, pos(0)), None);
}

// ── AC-002 / PT-002: prev_diagnostic ───────────────────────────────────────────────────────────────

#[test]
fn prev_diagnostic_last_before_cursor_then_wraps() {
    let markers = [diag(5), diag(10), diag(20)];
    assert_eq!(prev_diagnostic(&markers, pos(50)), Some(pos(20)));
    assert_eq!(prev_diagnostic(&markers, pos(15)), Some(pos(10)));
    // On a marker -> the previous (strict).
    assert_eq!(prev_diagnostic(&markers, pos(10)), Some(pos(5)));
    // At/before the first -> wrap to the last.
    assert_eq!(prev_diagnostic(&markers, pos(5)), Some(pos(20)));
    assert_eq!(prev_diagnostic(&markers, pos(0)), Some(pos(20)));
}

#[test]
fn prev_diagnostic_empty_list_is_none() {
    let markers: [GutterMarker; 0] = [];
    assert_eq!(prev_diagnostic(&markers, pos(0)), None);
}

// ── AC-008: live panel dispatch moves the caret + records a jump ───────────────────────────────────

#[test]
fn f8_dispatch_moves_caret_to_next_diagnostic_and_wraps() {
    // A 30-line buffer with diagnostics on lines 5, 10, 20 (the SAME path F8 takes: dispatch_action ->
    // go_to_next_diagnostic -> navigate_to_line).
    let panel = Arc::new(CodeEditorPanel::new(&numbered_buffer(30), "rs"));
    panel.push_diagnostics(vec![diag(5), diag(10), diag(20)]);
    assert_eq!(caret_line(&panel), 0, "caret starts at line 0");
    assert!(!panel.can_navigate_back(), "no jump history yet");

    // F8 -> next diagnostic after line 0 is line 5.
    panel.dispatch_action(CodeEditorAction::GoToNextDiagnostic);
    assert_eq!(
        caret_line(&panel),
        5,
        "F8 jumps the caret to the next diagnostic line (5)"
    );
    assert!(
        panel.can_navigate_back(),
        "the F8 jump recorded a back entry"
    );

    // F8 again -> line 10, then 20, then wraps to 5.
    panel.dispatch_action(CodeEditorAction::GoToNextDiagnostic);
    assert_eq!(caret_line(&panel), 10);
    panel.dispatch_action(CodeEditorAction::GoToNextDiagnostic);
    assert_eq!(caret_line(&panel), 20);
    panel.dispatch_action(CodeEditorAction::GoToNextDiagnostic);
    assert_eq!(
        caret_line(&panel),
        5,
        "F8 at the last marker wraps to the first"
    );
}

#[test]
fn shift_f8_dispatch_moves_caret_to_prev_diagnostic_and_wraps() {
    let panel = Arc::new(CodeEditorPanel::new(&numbered_buffer(30), "rs"));
    panel.push_diagnostics(vec![diag(5), diag(10), diag(20)]);
    // Start at line 0; Shift+F8 (prev) wraps to the last marker (20).
    panel.dispatch_action(CodeEditorAction::GoToPrevDiagnostic);
    assert_eq!(
        caret_line(&panel),
        20,
        "Shift+F8 at/before the first wraps to the last"
    );
    panel.dispatch_action(CodeEditorAction::GoToPrevDiagnostic);
    assert_eq!(caret_line(&panel), 10);
    panel.dispatch_action(CodeEditorAction::GoToPrevDiagnostic);
    assert_eq!(caret_line(&panel), 5);
}

#[test]
fn f8_with_no_diagnostics_is_a_graceful_no_op() {
    // No diagnostics: F8 must not move the caret and must not record a phantom jump (no panic).
    let panel = Arc::new(CodeEditorPanel::new(&numbered_buffer(30), "rs"));
    panel.dispatch_action(CodeEditorAction::GoToNextDiagnostic);
    panel.dispatch_action(CodeEditorAction::GoToPrevDiagnostic);
    assert_eq!(caret_line(&panel), 0, "no diagnostics -> caret unmoved");
    assert!(
        !panel.can_navigate_back(),
        "no diagnostics -> no jump recorded"
    );
}

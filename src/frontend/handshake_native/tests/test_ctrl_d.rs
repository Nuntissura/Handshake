//! MT-003 Ctrl+D (select-next-occurrence) proof (WP-KERNEL-012 — E1 code editor).
//!
//! AC-003 (`cargo test -p handshake-native ctrl_d`): buffer `foo\nfoo\nbar`, the primary cursor on the
//! first `foo`; Ctrl+D adds a selection over the SECOND `foo` (line index 1, bytes 4..7); a further
//! Ctrl+D WRAPS back to the first `foo` rather than looping forever (RISK-003 / MC-003).
//!
//! Every test fn name contains `ctrl_d` so the PT-003 filter selects exactly this set.

use handshake_native::code_editor::{Cursor, CodeEditorPanel};

/// AC-003: starting with the first `foo` selected (the cursor is "on" the word), Ctrl+D selects the
/// next occurrence (the second `foo`), and a third Ctrl+D wraps to the first `foo` without looping.
#[test]
fn ctrl_d_selects_next_then_wraps() {
    // foo(0..3) \n foo(4..7) \n bar(8..11)
    let panel = CodeEditorPanel::new("foo\nfoo\nbar", "txt");
    // Seed the primary cursor selecting the first "foo" (bytes 0..3) — the cursor is on the word.
    panel.set_cursors(vec![Cursor::selection(0, 3)]);

    // 1st Ctrl+D: add the next occurrence -> the second "foo" at bytes 4..7 (line 1, col 0..3).
    let added = panel.select_next_occurrence();
    assert!(added, "Ctrl+D added a cursor");
    let set = panel.cursors();
    assert_eq!(set.len(), 2, "two selections now (first + second foo)");
    let ranges: Vec<_> = set.cursors().iter().map(|c| c.range()).collect();
    assert!(ranges.contains(&(0..3)), "first foo still selected: {ranges:?}");
    assert!(ranges.contains(&(4..7)), "AC-003: second foo (bytes 4..7) selected: {ranges:?}");

    // 2nd Ctrl+D: the only other occurrence is the first foo (already selected) -> wrap-around hits an
    // existing selection, so it is a no-op (RISK-003: no infinite loop, no duplicate cursor).
    let added2 = panel.select_next_occurrence();
    assert!(!added2, "RISK-003: wrap to an already-selected occurrence does not add a cursor");
    assert_eq!(panel.cursors().len(), 2, "still exactly two selections (no duplicate from the wrap)");
}

/// Monaco first-press behavior: a BARE caret on a word -> first Ctrl+D selects that word in place;
/// second Ctrl+D adds the next occurrence; third wraps.
#[test]
fn ctrl_d_bare_caret_selects_word_first() {
    let panel = CodeEditorPanel::new("foo\nfoo\nbar", "txt");
    // Bare caret inside the first "foo" (col 1, byte 1).
    panel.set_single_cursor(1);
    assert!(!panel.cursors().primary().is_selection(), "starts as a bare caret");

    // 1st Ctrl+D: select the word under the caret (the first "foo").
    panel.select_next_occurrence();
    let set = panel.cursors();
    assert_eq!(set.len(), 1, "first Ctrl+D selects the word in place (one selection)");
    assert_eq!(set.primary().range(), 0..3, "the word 'foo' (bytes 0..3) is selected");

    // 2nd Ctrl+D: add the next occurrence (second foo, 4..7).
    panel.select_next_occurrence();
    let ranges: Vec<_> = panel.cursors().cursors().iter().map(|c| c.range()).collect();
    assert!(ranges.contains(&(4..7)), "second Ctrl+D adds the next foo: {ranges:?}");
    assert_eq!(panel.cursors().len(), 2);
}

/// A unique word: Ctrl+D selects it once and a second press is a no-op (the single-occurrence
/// guard — RISK-003).
#[test]
fn ctrl_d_single_occurrence_does_not_loop() {
    let panel = CodeEditorPanel::new("alpha beta gamma", "txt");
    panel.set_single_cursor(7); // inside "beta"
    panel.select_next_occurrence(); // selects "beta"
    assert_eq!(panel.cursors().primary().range(), 6..10, "beta selected");
    let before = panel.cursors().len();
    let added = panel.select_next_occurrence(); // only one "beta" -> wrap to itself -> no-op
    assert!(!added, "single occurrence: no extra cursor");
    assert_eq!(panel.cursors().len(), before, "no duplicate added for a unique word");
}

/// Ctrl+D on whitespace (not on a word) is a no-op.
#[test]
fn ctrl_d_not_on_word_is_noop() {
    let panel = CodeEditorPanel::new("a   b", "txt");
    panel.set_single_cursor(2); // a space
    let added = panel.select_next_occurrence();
    assert!(!added, "Ctrl+D off a word does nothing");
    assert_eq!(panel.cursors().len(), 1);
}

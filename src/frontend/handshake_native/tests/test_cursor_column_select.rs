//! MT-003 column/box selection proof (WP-KERNEL-012 — E1 code editor).
//!
//! AC-002 (`cargo test -p handshake-native cursor_column_select`): a column/box selection over lines
//! 2-4, columns 3-7 on a 5-line buffer yields THREE cursors, each with the correct `(anchor, head)`
//! byte offsets for its line. Also proves the RISK-002 short-line column clamp.
//!
//! Every test fn name contains `cursor_column_select` so the PT-002 filter selects exactly this set.

use handshake_native::code_editor::{line_col_to_byte, CodeEditorPanel, Cursor, TextBuffer};

/// AC-002: box-select lines 2..=4, columns 3..7 on a 5-line buffer of equal-length lines and verify the
/// three cursors' exact byte offsets.
#[test]
fn cursor_column_select_three_lines_exact_offsets() {
    // 5 lines, each "0123456789" (10 chars + '\n' = 11 bytes). Line L starts at byte L*11.
    let doc = "0123456789\n".repeat(5);
    let panel = CodeEditorPanel::new(&doc, "txt");

    // Box select lines 2..=4 (3 lines), columns 3..7.
    panel.set_box_selection(2, 3, 4, 7);

    let set = panel.cursors();
    assert_eq!(set.len(), 3, "three lines in the column range -> three cursors");

    // Expected per-line offsets: line L start = L*11; anchor = start+3, head = start+7.
    for (i, line) in (2..=4).enumerate() {
        let start = line * 11;
        let c = set.cursors()[i];
        assert_eq!(c.anchor, start + 3, "line {line} anchor at col 3 (byte {})", start + 3);
        assert_eq!(c.head, start + 7, "line {line} head at col 7 (byte {})", start + 7);
        assert!(c.is_selection(), "line {line} cursor is a selection (col 3..7)");
    }
}

/// RISK-002: a column past the end of a SHORT line clamps to the line end (no offset past the line /
/// into the next line). Mixed-length lines.
#[test]
fn cursor_column_select_clamps_short_lines() {
    // line 0 "ab" (2), line 1 "wxyz" (4), line 2 "" (empty).
    let doc = "ab\nwxyz\n";
    let buf = TextBuffer::new(doc);
    let panel = CodeEditorPanel::new(doc, "txt");

    // Select columns 1..6 across all three lines. Lines shorter than the columns clamp to their end.
    panel.set_box_selection(0, 1, 2, 6);
    let set = panel.cursors();
    assert_eq!(set.len(), 3, "three lines -> three cursors (even the empty last line)");

    // Line 0 "ab": col 1 = byte 1, col 6 clamps to the line end (byte 2).
    let line0 = set.cursors()[0];
    assert_eq!(line0.anchor, 1, "line 0 anchor col 1");
    assert_eq!(line0.head, 2, "line 0 head clamps to end of 'ab' (byte 2), never into the newline");

    // Line 1 "wxyz" starts at byte 3: col 1 = byte 4, col 6 clamps to line end (byte 7).
    let line1 = set.cursors()[1];
    assert_eq!(line1.anchor, line_col_to_byte(1, 1, &buf), "line 1 anchor col 1");
    assert_eq!(line1.head, 7, "line 1 head clamps to end of 'wxyz' (byte 7)");

    // Line 2 is empty (starts at byte 8): both clamp to byte 8 -> an empty caret on that row.
    let line2 = set.cursors()[2];
    assert_eq!(line2.anchor, line2.head, "empty line -> empty caret (valid box row)");
}

/// The box selection is direction-agnostic: dragging bottom-up / right-to-left yields the same set.
#[test]
fn cursor_column_select_is_direction_agnostic() {
    let doc = "0123456789\n".repeat(5);
    let panel = CodeEditorPanel::new(&doc, "txt");

    panel.set_box_selection(4, 7, 2, 3); // reversed line + column order
    let set = panel.cursors();
    assert_eq!(set.len(), 3);
    for (i, line) in (2..=4).enumerate() {
        let start = line * 11;
        let c = set.cursors()[i];
        assert_eq!((c.anchor, c.head), (start + 3, start + 7), "reversed drag normalizes to col 3..7");
    }
}

/// Regression (must-fix, adversarial review): painting a SINGLE selection that spans >= 3 lines must
/// not panic. A box selection produces one single-line selection per row, so it never exercises the
/// whole-line MIDDLE-row paint branch in `paint_cursor_overlay`, which derives the row's content-end
/// column via `line_col_to_byte(line, usize::MAX, ..)`. On any middle row `line_start > 0`, so the old
/// `start_char + usize::MAX` overflowed `usize` and PANICKED the egui frame under debug overflow-checks
/// (or wrapped to a garbage end column in release). Here we set ONE selection from line 0 to line 4 of
/// a 5-line buffer so lines 1, 2, 3 hit the whole-line branch, then render a real frame: it must paint
/// without panicking. None of the four AC proofs covered this (they use bare carets or per-row box
/// selections), which is exactly why the panic was latent.
#[test]
fn cursor_column_select_paints_multiline_selection_without_overflow_panic() {
    // 5 lines, each "0123456789\n" (11 bytes); a >= 3-line selection guarantees middle rows.
    let doc = "0123456789\n".repeat(5);
    let buf = TextBuffer::new(&doc);
    let panel = CodeEditorPanel::new(&doc, "txt");

    // ONE selection from the start of line 0 to the start of line 4 -> rows 1, 2, 3 are whole-line
    // (middle) selection rows in the overlay paint loop (each with line_start > 0).
    let head = line_col_to_byte(4, 0, &buf);
    panel.set_cursors(vec![Cursor::selection(0, head)]);
    assert_eq!(panel.cursor_count(), 1, "one multi-line selection");
    assert!(panel.cursors().cursors()[0].is_selection(), "it is a selection, not a bare caret");

    // Render a real frame. The overlay paint walks rows 0..=4; rows 1, 2, 3 take the whole-line branch
    // that calls line_col_to_byte(line, usize::MAX, ..). Before the saturating-add fix this panicked
    // under debug overflow-checks. A successful run with a non-empty paint output proves it is fixed.
    let ctx = egui::Context::default();
    let output = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| panel.show(ui));
    });
    // The frame produced paint primitives (it rendered without panicking on the middle rows).
    assert!(
        !output.shapes.is_empty(),
        "the multi-line-selection frame painted shapes (no overflow panic on the whole-line rows)"
    );
}

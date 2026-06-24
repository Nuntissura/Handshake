//! Diagnostic traversal for the native code editor (WP-KERNEL-012 MT-052 — E1 GO-menu navigation).
//!
//! This module is the pure, egui-free core behind **Go to Next Problem (F8)** and **Go to Previous
//! Problem (Shift+F8)**. It walks the ordered diagnostic markers the MT-007 gutter store holds (the
//! markers MT-008's LSP client pushed via `push_diagnostics`) and computes the next/previous marker
//! position relative to the caret, with wraparound at the ends — exactly the VS Code F8 behavior.
//!
//! ## Reuse, do NOT redefine the marker type (RISK-001 / MC-001)
//!
//! The diagnostic marker type is the MT-007 [`GutterMarker`](super::gutter::GutterMarker). The MT-052
//! contract sketched a `DiagnosticMarker` type "imported from gutter.rs", but the REAL type MT-007
//! shipped is `GutterMarker { line, kind: GutterMarkerKind, message }` — there is no `DiagnosticMarker`
//! type. Introducing a parallel marker struct would fragment the diagnostic model and break ordering
//! parity with the gutter + Problems list, so this module imports `GutterMarker` directly and filters to
//! the [`GutterMarkerKind::Diagnostic`](super::gutter::GutterMarkerKind::Diagnostic) variants. (Verified
//! by reading `code_editor/gutter.rs`.)
//!
//! ## Position type
//!
//! [`BufferPosition`] is a `{ line, column }` pair (both 0-based). The editor's live cursor is a
//! BYTE-OFFSET [`Cursor`](super::cursor::Cursor) (`{ anchor, head }`), NOT a line/column pair — verified
//! by reading `code_editor/cursor.rs`. Diagnostic markers are LINE-anchored (`GutterMarker.line`, with no
//! column), so traversal happens in line/column space; the panel converts the byte caret to a
//! `BufferPosition` (via `cursor::byte_to_line_col`) before calling in, and converts the returned
//! position back to a byte offset (via `cursor::line_col_to_byte`) to move the caret. A `GutterMarker`'s
//! column is always 0 (it is a whole-line marker), so the (line, column) total order degenerates to a
//! line order; `BufferPosition` keeps the column so the same comparison generalizes if column-precise
//! markers are added later.
//!
//! ## Ordering parity with ProblemsView.tsx
//!
//! `app/src/components/operator/ProblemsView.tsx` sorts problems by file, then line, then column, then
//! severity. Within a single file (the editor traverses one file's gutter store) the ordering reduces to
//! (line, column) ascending — the total order [`sort_markers`] applies to a BORROWED copy of the marker
//! slice (it never mutates the caller's gutter store, per the implementation note). So F8 traversal order
//! equals the Problems-list order a user would see for that file.

use super::cursor::Cursor;
use super::gutter::{GutterMarker, GutterMarkerKind};

/// A `(line, column)` position in the buffer, both 0-based. The traversal coordinate space. The editor's
/// live caret is a byte offset; the panel converts at the boundary (`byte_to_line_col` /
/// `line_col_to_byte`) so this module stays a pure, egui-free, trivially-testable comparator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BufferPosition {
    /// 0-based line.
    pub line: usize,
    /// 0-based column (chars from the line start). Always 0 for a whole-line diagnostic marker.
    pub column: usize,
}

impl BufferPosition {
    /// A position at `(line, column)`.
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }

    /// The `(line, column)` of a byte-offset caret head, computed against `buffer`. The bridge from the
    /// editor's byte-offset [`Cursor`] to traversal space. `column` is the number of chars from the
    /// line start (the same convention [`super::cursor::byte_to_line_col`] uses).
    pub fn from_cursor(cursor: Cursor, buffer: &super::buffer::TextBuffer) -> Self {
        let (line, column) = super::cursor::byte_to_line_col(cursor.head, buffer);
        Self { line, column }
    }

    /// The total order key: `(line, column)`. Ascending line first, then column.
    fn key(&self) -> (usize, usize) {
        (self.line, self.column)
    }
}

/// The position of one diagnostic marker: its line, column 0 (whole-line marker). Centralized so the
/// comparison code reads the SAME (line, column) shape for both the caret and a marker.
fn marker_position(marker: &GutterMarker) -> BufferPosition {
    BufferPosition::new(marker.line, 0)
}

/// True when `marker` is a DIAGNOSTIC marker (not a breakpoint / fold-triangle marker). Only diagnostic
/// markers participate in F8/Shift+F8 traversal — a breakpoint or fold triangle is not a "problem".
fn is_diagnostic(marker: &GutterMarker) -> bool {
    matches!(marker.kind, GutterMarkerKind::Diagnostic(_))
}

/// Build a (line, column)-ascending-sorted list of the DIAGNOSTIC marker positions in `markers`,
/// de-duplicated by position so two diagnostics on the same line are one traversal stop (VS Code F8
/// visits each problem LINE once; multiple diagnostics on a line share the stop). Operates on a BORROWED
/// copy — never mutates the caller's gutter store (implementation note: "sort a borrowed copy"). Returns
/// an empty vec when there are no diagnostic markers.
fn sorted_diagnostic_positions(markers: &[GutterMarker]) -> Vec<BufferPosition> {
    let mut positions: Vec<BufferPosition> = markers
        .iter()
        .filter(|m| is_diagnostic(m))
        .map(marker_position)
        .collect();
    positions.sort_by_key(BufferPosition::key);
    positions.dedup();
    positions
}

/// The stateless diagnostic navigator. It holds no state of its own — every call reads the live marker
/// list + caret the panel hands in (the gutter/diagnostic store is the single owner of marker state), so
/// it is trivially `Send + Sync` and unit-testable without egui.
pub struct DiagnosticNavigator;

impl DiagnosticNavigator {
    /// The position of the first diagnostic marker STRICTLY after `cursor`, or — when the cursor is at or
    /// past the last marker — the FIRST marker (wraparound). Returns `None` only when there are no
    /// diagnostic markers.
    ///
    /// "Strictly after" excludes a marker the caret is exactly on (RISK-002 / MC-002): pressing F8 with
    /// the caret already on a problem line advances to the NEXT problem, never re-selecting the current
    /// one (no stall) and never skipping the adjacent one.
    pub fn next(markers: &[GutterMarker], cursor: BufferPosition) -> Option<BufferPosition> {
        let positions = sorted_diagnostic_positions(markers);
        // RISK-003 / MC-003: an empty list returns None BEFORE any indexing (no panic on `[0]` / `last`).
        if positions.is_empty() {
            return None;
        }
        positions
            .iter()
            .find(|p| p.key() > cursor.key())
            .copied()
            // Wrap: no marker strictly after the cursor -> the first marker.
            .or_else(|| positions.first().copied())
    }

    /// The position of the last diagnostic marker STRICTLY before `cursor`, or — when the cursor is at or
    /// before the first marker — the LAST marker (wraparound). Returns `None` only when there are no
    /// diagnostic markers. Symmetric to [`next`](Self::next): the strict comparison excludes the marker
    /// the caret is on so Shift+F8 always moves.
    pub fn prev(markers: &[GutterMarker], cursor: BufferPosition) -> Option<BufferPosition> {
        let positions = sorted_diagnostic_positions(markers);
        if positions.is_empty() {
            return None;
        }
        positions
            .iter()
            .rev()
            .find(|p| p.key() < cursor.key())
            .copied()
            // Wrap: no marker strictly before the cursor -> the last marker.
            .or_else(|| positions.last().copied())
    }
}

/// Free-function form of [`DiagnosticNavigator::next`] (the exact name the MT contract scope text names:
/// `next_diagnostic(markers, cursor)`). Delegates to the navigator so there is one implementation.
pub fn next_diagnostic(markers: &[GutterMarker], cursor: BufferPosition) -> Option<BufferPosition> {
    DiagnosticNavigator::next(markers, cursor)
}

/// Free-function form of [`DiagnosticNavigator::prev`] (the MT contract name `prev_diagnostic`).
pub fn prev_diagnostic(markers: &[GutterMarker], cursor: BufferPosition) -> Option<BufferPosition> {
    DiagnosticNavigator::prev(markers, cursor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_editor::gutter::{DiagnosticSeverity, GutterMarker};

    fn diag(line: usize) -> GutterMarker {
        GutterMarker::diagnostic(line, DiagnosticSeverity::Error, "boom")
    }

    fn pos(line: usize) -> BufferPosition {
        BufferPosition::new(line, 0)
    }

    // ── next_diagnostic ───────────────────────────────────────────────────────────────────────────

    #[test]
    fn next_from_before_all_markers_returns_first() {
        let markers = [diag(5), diag(10), diag(20)];
        // Cursor before the first marker -> first marker.
        assert_eq!(next_diagnostic(&markers, pos(0)), Some(pos(5)));
    }

    #[test]
    fn next_between_two_markers_returns_the_one_after() {
        let markers = [diag(5), diag(10), diag(20)];
        // Cursor between marker 5 and 10 -> 10.
        assert_eq!(next_diagnostic(&markers, pos(7)), Some(pos(10)));
    }

    #[test]
    fn next_at_last_marker_wraps_to_first() {
        let markers = [diag(5), diag(10), diag(20)];
        // Cursor ON the last marker -> wrap to the first (strict "after" excludes the current line).
        assert_eq!(next_diagnostic(&markers, pos(20)), Some(pos(5)));
    }

    #[test]
    fn next_strictly_after_excludes_the_current_marker_line() {
        // RISK-002 / MC-002: caret exactly on a marker advances to the NEXT, never re-selecting itself.
        let markers = [diag(5), diag(10), diag(20)];
        assert_eq!(next_diagnostic(&markers, pos(5)), Some(pos(10)));
        assert_eq!(next_diagnostic(&markers, pos(10)), Some(pos(20)));
    }

    #[test]
    fn next_empty_list_returns_none() {
        // RISK-003 / MC-003: empty list -> None, no panic on index 0.
        let markers: [GutterMarker; 0] = [];
        assert_eq!(next_diagnostic(&markers, pos(0)), None);
    }

    // ── prev_diagnostic ───────────────────────────────────────────────────────────────────────────

    #[test]
    fn prev_from_after_all_markers_returns_last() {
        let markers = [diag(5), diag(10), diag(20)];
        assert_eq!(prev_diagnostic(&markers, pos(50)), Some(pos(20)));
    }

    #[test]
    fn prev_between_two_markers_returns_the_one_before() {
        let markers = [diag(5), diag(10), diag(20)];
        assert_eq!(prev_diagnostic(&markers, pos(15)), Some(pos(10)));
    }

    #[test]
    fn prev_at_first_marker_wraps_to_last() {
        let markers = [diag(5), diag(10), diag(20)];
        // Cursor ON the first marker -> wrap to the last (strict "before" excludes the current line).
        assert_eq!(prev_diagnostic(&markers, pos(5)), Some(pos(20)));
    }

    #[test]
    fn prev_strictly_before_excludes_the_current_marker_line() {
        let markers = [diag(5), diag(10), diag(20)];
        assert_eq!(prev_diagnostic(&markers, pos(20)), Some(pos(10)));
        assert_eq!(prev_diagnostic(&markers, pos(10)), Some(pos(5)));
    }

    #[test]
    fn prev_empty_list_returns_none() {
        let markers: [GutterMarker; 0] = [];
        assert_eq!(prev_diagnostic(&markers, pos(0)), None);
    }

    // ── ordering parity + non-diagnostic filtering + non-mutation ───────────────────────────────────

    #[test]
    fn markers_are_traversed_in_line_order_regardless_of_input_order() {
        // ProblemsView ordering parity: out-of-order input is traversed (line) ascending.
        let markers = [diag(20), diag(5), diag(10)];
        assert_eq!(next_diagnostic(&markers, pos(0)), Some(pos(5)));
        assert_eq!(next_diagnostic(&markers, pos(5)), Some(pos(10)));
        assert_eq!(next_diagnostic(&markers, pos(10)), Some(pos(20)));
        assert_eq!(next_diagnostic(&markers, pos(20)), Some(pos(5)));
    }

    #[test]
    fn non_diagnostic_markers_are_ignored() {
        // A breakpoint / fold-triangle marker is NOT a problem; only diagnostics are traversed.
        let markers = vec![
            GutterMarker { line: 3, kind: GutterMarkerKind::Breakpoint, message: String::new() },
            diag(10),
            GutterMarker {
                line: 15,
                kind: GutterMarkerKind::FoldTriangle(true),
                message: String::new(),
            },
        ];
        // Only the diagnostic on line 10 is a stop; from line 0 -> 10, and it wraps to itself.
        assert_eq!(next_diagnostic(&markers, pos(0)), Some(pos(10)));
        assert_eq!(next_diagnostic(&markers, pos(10)), Some(pos(10)));
        assert_eq!(prev_diagnostic(&markers, pos(10)), Some(pos(10)));
    }

    #[test]
    fn duplicate_diagnostics_on_a_line_are_one_stop() {
        // Two diagnostics on line 10 (e.g. an error + a warning) are a SINGLE F8 stop.
        let markers = [diag(5), diag(10), diag(10), diag(20)];
        assert_eq!(next_diagnostic(&markers, pos(10)), Some(pos(20)));
        assert_eq!(prev_diagnostic(&markers, pos(10)), Some(pos(5)));
    }

    #[test]
    fn does_not_mutate_the_caller_marker_slice() {
        // The traversal sorts a BORROWED copy; the caller's slice order is unchanged.
        let markers = [diag(20), diag(5), diag(10)];
        let before: Vec<usize> = markers.iter().map(|m| m.line).collect();
        let _ = next_diagnostic(&markers, pos(0));
        let _ = prev_diagnostic(&markers, pos(50));
        let after: Vec<usize> = markers.iter().map(|m| m.line).collect();
        assert_eq!(before, after, "the caller's marker slice must not be reordered");
    }
}

//! MT-003 multi-cursor MODEL proofs (WP-KERNEL-012 — E1 code editor).
//!
//! AC-001 (`cargo test -p handshake-native cursor`): add two cursors -> both present; add overlapping
//! cursors -> merged; insert text with two cursors -> both insertions happened and the offsets are
//! correct after the second insertion.
//! AC-006 (`insert_at_all` offset adjustment): two cursors at offsets 0 and 5 inserting "X" -> 'X' at
//! position 0 AND 'X' at position 6 (the second offset shifted by +1 for the first insertion).
//!
//! These exercise the [`CursorSet`] public API directly (no GPU). Every test fn name contains `cursor`
//! so the PT-001 filter `cargo test -p handshake-native cursor` selects this proof set (it also
//! selects the column-select + accesskit files, which is expected and harmless — all GREEN).

use handshake_native::code_editor::{Cursor, CursorSet, MoveDir, TextBuffer};

/// AC-001 part 1: two distinct carets are both kept (not collapsed).
#[test]
fn cursor_add_two_keeps_both() {
    let buf = TextBuffer::new("hello world");
    let mut set = CursorSet::single(0);
    set.add_cursor(6, &buf);
    assert_eq!(set.len(), 2, "two distinct carets are kept");
    let heads: Vec<usize> = set.cursors().iter().map(|c| c.head).collect();
    assert_eq!(heads, vec![0, 6], "cursors sorted by head: {heads:?}");
}

/// AC-001 part 2: overlapping cursors merge into one spanning the union.
#[test]
fn cursor_overlapping_selections_merge() {
    let buf = TextBuffer::new("abcdefghij");
    let mut set = CursorSet::default();
    set.set_cursors(vec![Cursor::selection(0, 5), Cursor::selection(3, 8)], &buf);
    assert_eq!(set.len(), 1, "overlapping selections merge");
    assert_eq!(set.cursors()[0].range(), 0..8, "merged range is the union");

    // Two carets at the SAME offset also collapse to one (dedupe).
    let mut dup = CursorSet::single(4);
    dup.add_cursor(4, &buf);
    assert_eq!(dup.len(), 1, "duplicate caret offset collapses");
}

/// AC-001 part 3 + AC-006: insert at TWO cursors and verify BOTH insertions landed and the offsets are
/// adjusted (the RISK-001 reverse-order + cumulative-delta proof — the offset-adjustment unit test that
/// MC-001 requires to be in CI).
#[test]
fn cursor_insert_at_all_adjusts_offsets() {
    // Buffer "0123456789"; cursors at byte 0 and byte 5. Insert "X" at both.
    let mut buf = TextBuffer::new("0123456789");
    let mut set = CursorSet::default();
    set.set_cursors(vec![Cursor::caret(0), Cursor::caret(5)], &buf);
    let applied = set.insert_at_all("X", &mut buf);
    assert_eq!(applied, 2, "both insertions applied");

    // AC-006: 'X' at position 0 (before "0") and 'X' at position 6 ("01234" + first X shifted the
    // second insert point from 5 to 6).
    let text = buf.to_string();
    assert_eq!(
        text, "X01234X56789",
        "both X inserted, second offset shifted by +1: {text:?}"
    );
    assert_eq!(text.as_bytes()[0], b'X', "X at position 0");
    assert_eq!(
        text.as_bytes()[6],
        b'X',
        "X at position 6 (offset-adjusted)"
    );

    // Each caret lands immediately AFTER its own inserted text.
    let heads: Vec<usize> = set.cursors().iter().map(|c| c.head).collect();
    assert_eq!(
        heads,
        vec![1, 7],
        "carets sit after their inserted X: {heads:?}"
    );
}

/// AC-006 exact contract wording: cursors at 0 and 5 inserting 'X' -> buffer has 'X' at 0 and 'X' at 6.
#[test]
fn cursor_insert_two_cursors_x_lands_at_0_and_6() {
    let mut buf = TextBuffer::new("aaaaabbbbb"); // 10 bytes; offset 5 is the b boundary
    let mut set = CursorSet::default();
    set.set_cursors(vec![Cursor::caret(0), Cursor::caret(5)], &buf);
    set.insert_at_all("X", &mut buf);
    let text = buf.to_string();
    assert_eq!(
        text.as_bytes()[0],
        b'X',
        "first X at position 0; text={text:?}"
    );
    assert_eq!(
        text.as_bytes()[6],
        b'X',
        "second X at position 6 (offset adjusted by 1); text={text:?}"
    );
    assert_eq!(text, "XaaaaaXbbbbb");
}

/// Insert with a multi-char string at three cursors — the cumulative delta is `len*index`, proving the
/// adjustment is by BYTE LENGTH, not a fixed +1.
#[test]
fn cursor_insert_multichar_shifts_by_byte_length() {
    let mut buf = TextBuffer::new("..|..|.."); // carets at 0, 3, 6
    let mut set = CursorSet::default();
    set.set_cursors(
        vec![Cursor::caret(0), Cursor::caret(3), Cursor::caret(6)],
        &buf,
    );
    set.insert_at_all("AB", &mut buf); // 2-byte insert at each
    let text = buf.to_string();
    // Original "..|..|.." with AB at 0,3,6 -> "AB..AB|..AB|.." ... verify the three ABs at shifted spots.
    assert_eq!(
        text, "AB..|AB..|AB..",
        "three 2-byte inserts, each shifted by 2*priorInserts: {text:?}"
    );
}

/// Delete at all cursors: a selection is removed; a bare caret backspaces one char. Reverse-order so
/// earlier deletes never invalidate later offsets.
#[test]
fn cursor_delete_at_all_handles_selections_and_backspace() {
    // "abcXYZdef": selection over "XYZ" (3..6) + a bare caret at 9 (end) backspaces the 'f'.
    let mut buf = TextBuffer::new("abcXYZdef");
    let mut set = CursorSet::default();
    set.set_cursors(vec![Cursor::selection(3, 6), Cursor::caret(9)], &buf);
    let applied = set.delete_at_all(&mut buf);
    assert_eq!(applied, 2, "selection removed + one backspace");
    assert_eq!(
        buf.to_string(),
        "abcde",
        "XYZ removed and trailing f backspaced"
    );
}

/// move_all collapses selections to carets and is char-boundary safe (the MoveDir surface).
#[test]
fn cursor_move_all_collapses_and_moves() {
    let buf = TextBuffer::new("hello\nworld");
    let mut set = CursorSet::default();
    set.set_cursors(vec![Cursor::selection(0, 3), Cursor::caret(6)], &buf);
    set.move_all(MoveDir::Right, &buf);
    // Both become bare carets at their moved head: 3->4 (within "hello"), 6->7.
    for c in set.cursors() {
        assert!(!c.is_selection(), "move collapses selection to a caret");
    }
    let heads: Vec<usize> = set.cursors().iter().map(|c| c.head).collect();
    assert_eq!(
        heads,
        vec![4, 7],
        "each caret moved right one char: {heads:?}"
    );
}

/// The set always has at least one caret, and every offset stays within the buffer (invariants).
#[test]
fn cursor_invariants_clamp_and_nonempty() {
    let buf = TextBuffer::new("hi"); // 2 bytes
    let mut set = CursorSet::default();
    // Wild offsets: clamp to 0..=2.
    set.set_cursors(
        vec![Cursor::caret(1000), Cursor::caret(2), Cursor::caret(0)],
        &buf,
    );
    assert!(
        set.cursors().iter().all(|c| c.head <= 2),
        "all offsets clamped to len_bytes"
    );
    assert!(!set.is_empty(), "set is never empty");
    // Even an explicitly empty set normalizes to one caret at 0.
    let mut empty = CursorSet::default();
    empty.set_cursors(vec![], &buf);
    assert_eq!(empty.len(), 1, "emptied set degrades to one caret");
    assert_eq!(empty.primary().head, 0);
}

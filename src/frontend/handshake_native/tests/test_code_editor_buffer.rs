//! Integration tests for the MT-001 code-editor text buffer (WP-KERNEL-012).
//!
//! AC-001: TextBuffer insert/delete/slice covering empty buffer, single-line insert, multi-line
//! insert, delete spanning lines, and byte<->line round-trips. AC-006: every range operation is
//! panic-free (returns Option/Result). PT-001 runs these via `cargo test text_buffer`.
//!
//! The test fn names contain `text_buffer` so `cargo test -p handshake-native text_buffer` (PT-001)
//! selects exactly this proof set.

use handshake_native::code_editor::buffer::BufferError;
use handshake_native::code_editor::TextBuffer;

#[test]
fn text_buffer_empty_insert_delete_slice() {
    let mut b = TextBuffer::new("");
    assert!(b.is_empty());
    assert_eq!(b.len_bytes(), 0);
    assert_eq!(b.len_lines(), 1);
    // Insert into an empty buffer.
    b.insert(0, "hello").unwrap();
    assert_eq!(b.to_string(), "hello");
    // Delete it all back to empty.
    b.delete(0..5).unwrap();
    assert!(b.is_empty());
}

#[test]
fn text_buffer_single_line_insert() {
    let mut b = TextBuffer::new("abcdef");
    b.insert(3, "XYZ").unwrap();
    assert_eq!(b.to_string(), "abcXYZdef");
    assert_eq!(b.len_lines(), 1);
}

#[test]
fn text_buffer_multi_line_insert_changes_line_count() {
    let mut b = TextBuffer::new("one");
    assert_eq!(b.len_lines(), 1);
    b.insert(3, "\ntwo\nthree").unwrap();
    assert_eq!(b.to_string(), "one\ntwo\nthree");
    assert_eq!(b.len_lines(), 3);
}

#[test]
fn text_buffer_delete_spanning_lines() {
    let mut b = TextBuffer::new("aaa\nbbb\nccc\n");
    // Remove the whole middle line "bbb\n" (byte 4..8).
    let start = b.line_to_byte(1).unwrap();
    let end = b.line_to_byte(2).unwrap();
    b.delete(start..end).unwrap();
    assert_eq!(b.to_string(), "aaa\nccc\n");
    assert_eq!(b.len_lines(), 3); // "aaa", "ccc", "" trailing
}

#[test]
fn text_buffer_byte_line_round_trips() {
    let b = TextBuffer::new("first\nsecond\nthird");
    for line in 0..b.len_lines() {
        let byte = b.line_to_byte(line).unwrap();
        assert_eq!(b.byte_to_line(byte), Some(line), "round-trip line {line}");
    }
}

#[test]
fn text_buffer_range_ops_are_panic_free() {
    let mut b = TextBuffer::new("data");
    // Out-of-range insert -> Err, not panic.
    assert_eq!(
        b.insert(1000, "x"),
        Err(BufferError::OffsetOutOfRange { offset: 1000, len_bytes: 4 })
    );
    // Inverted / out-of-range delete -> Err.
    assert!(matches!(b.delete(3..1), Err(BufferError::InvalidRange { .. })));
    assert!(matches!(b.delete(0..1000), Err(BufferError::InvalidRange { .. })));
    // Conversions return None for out-of-range, never panic.
    assert_eq!(b.byte_to_line(1000), None);
    assert_eq!(b.line_to_byte(1000), None);
    assert_eq!(b.byte_to_char(1000), None);
    // slice_to_string clamps wild ranges to empty/valid output without panicking.
    assert_eq!(b.slice_to_string(99..200), "");
    assert_eq!(b.slice_to_string(0..99), "data");
    // The buffer survived every failed op unchanged.
    assert_eq!(b.to_string(), "data");
}

#[test]
fn text_buffer_handles_non_ascii_without_off_by_one() {
    // "café\n" : c a f = 3 bytes, é = 2 bytes, \n = 1 byte -> 6 bytes, 5 chars.
    let mut b = TextBuffer::new("café\n");
    assert_eq!(b.len_bytes(), 6);
    // Inserting at a non-char-boundary (byte 4, inside 'é') is rejected, not a panic.
    assert!(matches!(b.insert(4, "x"), Err(BufferError::NotACharBoundary { .. })));
    // Inserting at a valid boundary works.
    b.insert(3, "X").unwrap(); // after "caf"
    assert_eq!(b.to_string(), "cafXé\n");
}

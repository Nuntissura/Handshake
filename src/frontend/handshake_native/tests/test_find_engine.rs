//! MT-004 find-engine proofs (WP-KERNEL-012 E1 code editor): the in-file search engine.
//!
//! AC-001 / PT-001 (`cargo test -p handshake-native find_engine`): plain-text find (case sensitive,
//! case insensitive, whole word), regex find, no-match returns empty vec, and the full-buffer
//! occurrence walk (the panel's wrapping current-match index is driven by this complete ordered list).
//!
//! These exercise the PUBLIC `FindEngine`/`FindQuery`/`Match` API exported from the crate (not the
//! in-module unit tests), so they prove the contract surface a consumer actually sees.

use handshake_native::code_editor::{FindEngine, FindQuery, Match, TextBuffer};

fn ranges(matches: &[Match]) -> Vec<std::ops::Range<usize>> {
    matches.iter().map(|m| m.byte_range.clone()).collect()
}

#[test]
fn find_engine_plain_case_sensitive() {
    let buf = TextBuffer::new("Foo foo FOO foo");
    let q = FindQuery {
        pattern: "foo".into(),
        case_sensitive: true,
        ..Default::default()
    };
    assert_eq!(ranges(&FindEngine::search(&q, &buf)), vec![4..7, 12..15]);
}

#[test]
fn find_engine_plain_case_insensitive() {
    let buf = TextBuffer::new("Foo foo FOO");
    let q = FindQuery {
        pattern: "foo".into(),
        case_sensitive: false,
        ..Default::default()
    };
    assert_eq!(
        ranges(&FindEngine::search(&q, &buf)),
        vec![0..3, 4..7, 8..11]
    );
}

#[test]
fn find_engine_whole_word() {
    let buf = TextBuffer::new("foo foobar foo_baz foo");
    let q = FindQuery {
        pattern: "foo".into(),
        case_sensitive: true,
        whole_word: true,
        ..Default::default()
    };
    // The bare "foo" tokens (0..3 and 19..22) match; "foobar" / "foo_baz" do not.
    assert_eq!(ranges(&FindEngine::search(&q, &buf)), vec![0..3, 19..22]);
}

#[test]
fn find_engine_regex() {
    let buf = TextBuffer::new("fn a() {}\nfn bb() {}\nlet c = 1;");
    let q = FindQuery {
        pattern: r"fn \w+".into(),
        is_regex: true,
        case_sensitive: true,
        ..Default::default()
    };
    assert_eq!(ranges(&FindEngine::search(&q, &buf)), vec![0..4, 10..15]);
}

#[test]
fn find_engine_no_matches_returns_empty() {
    let buf = TextBuffer::new("hello world");
    assert!(FindEngine::search(&FindQuery::literal("zzz"), &buf).is_empty());
}

#[test]
fn find_engine_returns_every_occurrence_in_order() {
    // The full ordered list is what the panel's wrapping next/prev index walks; prove it is complete
    // and ascending (the "wrap at end of buffer" behavior is the panel indexing past the last back to 0).
    let buf = TextBuffer::new("x.x.x.x");
    let q = FindQuery::literal("x");
    let m = FindEngine::search(&q, &buf);
    assert_eq!(ranges(&m), vec![0..1, 2..3, 4..5, 6..7]);
    // Ascending by start.
    assert!(m
        .windows(2)
        .all(|w| w[0].byte_range.start < w[1].byte_range.start));
}

#[test]
fn find_engine_match_line_col_for_scroll() {
    // PT: the panel scrolls to matches[current].line, so the line/col must be correct.
    let buf = TextBuffer::new("line0\n  needle here\nline2");
    let m = FindEngine::search(&FindQuery::literal("needle"), &buf);
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].line, 1);
    assert_eq!(m[0].col, 2); // after the two leading spaces
}

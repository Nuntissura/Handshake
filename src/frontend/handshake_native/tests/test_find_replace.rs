//! MT-004 find-replace proofs (WP-KERNEL-012 E1 code editor): replace_one + replace_all over the
//! public engine API, plus the panel-level replace workflow (re-search after a replace — RISK-003).
//!
//! AC-002 / PT-002 (`cargo test -p handshake-native find_replace`): replace_one replaces the correct
//! match and adjusts the buffer; replace_all replaces all N occurrences with offset correctness
//! (MC-002 — proven with both a LONGER and a SHORTER replacement so a wrong-order walk would corrupt
//! offsets). The panel-level tests prove the find_state match list is re-searched after a replace so a
//! stale offset can never replace the wrong bytes (RISK-003).

use handshake_native::code_editor::{CodeEditorPanel, FindEngine, FindQuery, TextBuffer};

#[test]
fn replace_one_replaces_correct_match() {
    let mut buf = TextBuffer::new("foo bar foo");
    let matches = FindEngine::search(&FindQuery::literal("foo"), &buf);
    assert_eq!(matches.len(), 2);
    // Replace the SECOND occurrence; the first is untouched.
    assert!(FindEngine::replace_one(&mut buf, &matches[1], "X"));
    assert_eq!(buf.to_string(), "foo bar X");
}

#[test]
fn replace_all_longer_replacement_offset_correct() {
    // MC-002: a LONGER replacement would corrupt later offsets if processed forward; reverse-order
    // processing keeps it correct.
    let mut buf = TextBuffer::new("foo foo foo");
    let matches = FindEngine::search(&FindQuery::literal("foo"), &buf);
    let applied = FindEngine::replace_all(&mut buf, &matches, "LONGER");
    assert_eq!(applied, 3);
    assert_eq!(buf.to_string(), "LONGER LONGER LONGER");
}

#[test]
fn replace_all_shorter_replacement_offset_correct() {
    let mut buf = TextBuffer::new("aaaa aaaa aaaa");
    let matches = FindEngine::search(&FindQuery::literal("aaaa"), &buf);
    let applied = FindEngine::replace_all(&mut buf, &matches, "b");
    assert_eq!(applied, 3);
    assert_eq!(buf.to_string(), "b b b");
}

// ── Panel-level workflow: re-search after a replace (RISK-003) ─────────────────────────────────────

#[test]
fn panel_replace_current_then_research_keeps_match_list_valid() {
    let panel = CodeEditorPanel::new("foo bar foo baz foo", "txt");
    panel.open_find(true); // replace mode
    panel.set_find_query("foo");
    panel.set_replace_text("X");

    let before = panel.find_state().expect("bar open");
    assert_eq!(
        before.matches.len(),
        3,
        "three foo matches before any replace"
    );

    // Replace the current (first) match.
    assert!(panel.replace_current(), "a replacement was applied");

    // RISK-003: the match list must be RE-SEARCHED so it reflects the edited buffer (now 2 matches),
    // never a stale 3-entry list with invalid offsets.
    let after = panel.find_state().expect("bar still open");
    assert_eq!(
        after.matches.len(),
        2,
        "match list re-searched after replace (RISK-003)"
    );
    assert_eq!(panel.buffer().to_string(), "X bar foo baz foo");
}

#[test]
fn panel_replace_all_clears_matches() {
    let panel = CodeEditorPanel::new("foo foo foo", "txt");
    panel.open_find(true);
    panel.set_find_query("foo");
    panel.set_replace_text("bar");

    let n = panel.replace_all();
    assert_eq!(n, 3, "all three replaced");
    assert_eq!(panel.buffer().to_string(), "bar bar bar");

    // After replacing every "foo" the re-search finds none.
    let after = panel.find_state().expect("bar open");
    assert!(
        after.matches.is_empty(),
        "no matches remain after replace-all of the only term"
    );
}

#[test]
fn panel_next_prev_wrap_around() {
    let panel = CodeEditorPanel::new("a a a", "txt");
    panel.open_find(false);
    panel.set_find_query("a");
    let s = panel.find_state().unwrap();
    assert_eq!(s.matches.len(), 3);
    assert_eq!(s.current_match, 0, "starts at the first match");

    panel.next_match();
    assert_eq!(panel.find_state().unwrap().current_match, 1);
    panel.next_match();
    assert_eq!(panel.find_state().unwrap().current_match, 2);
    panel.next_match(); // wrap to 0
    assert_eq!(
        panel.find_state().unwrap().current_match,
        0,
        "next wraps at the end"
    );
    panel.prev_match(); // wrap back to last
    assert_eq!(
        panel.find_state().unwrap().current_match,
        2,
        "prev wraps at the start"
    );
}

#[test]
fn panel_open_find_prepopulates_from_selection() {
    // Implementation note 4: opening find with a selection pre-populates the query.
    let panel = CodeEditorPanel::new("hello target world", "txt");
    // Select "target" (bytes 6..12) via the cursor API.
    panel.set_cursors(vec![handshake_native::code_editor::Cursor::selection(
        6, 12,
    )]);
    panel.open_find(false);
    let s = panel.find_state().expect("bar open");
    assert_eq!(
        s.query.pattern, "target",
        "selection pre-populates the find query"
    );
    assert!(
        !s.matches.is_empty(),
        "the pre-populated query found its own selection"
    );
}

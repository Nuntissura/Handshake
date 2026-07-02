//! MT-004 invalid-regex proof (WP-KERNEL-012 E1 code editor): a bad regex never panics.
//!
//! AC-003 / PT-003 / MC-001 (`cargo test -p handshake-native find_regex_error`): an invalid regex
//! pattern results in an EMPTY match list AND a non-empty error string surfaced in `FindState.error`
//! — and crucially, NO panic (the find bar must stay usable while the user is mid-typing an
//! incomplete pattern). RISK-001 is also covered: the `regex` crate is linear-time, so a
//! catastrophic-backtracking pattern returns immediately rather than hanging.

use handshake_native::code_editor::{CodeEditorPanel, FindEngine, FindQuery, TextBuffer};

#[test]
fn find_regex_error_invalid_no_panic_empty_matches_with_error() {
    let buf = TextBuffer::new("some (text) here with [brackets]");
    // An unclosed group is a classic mid-typing invalid pattern.
    let q = FindQuery {
        pattern: "(".into(),
        is_regex: true,
        ..Default::default()
    };

    // No panic, empty matches.
    let matches = FindEngine::search(&q, &buf);
    assert!(
        matches.is_empty(),
        "an invalid regex finds nothing (no panic)"
    );

    // A non-empty compile-error string is surfaced.
    let err = FindEngine::compile_error(&q);
    assert!(
        err.is_some() && !err.unwrap().is_empty(),
        "invalid regex yields an error string"
    );
}

#[test]
fn find_regex_error_panel_surfaces_error_in_find_state() {
    let panel = CodeEditorPanel::new("let x = (1 + 2);", "rs");
    panel.open_find(false);
    // Turn on regex and type an invalid pattern.
    panel.set_find_toggles(false, false, true);
    panel.set_find_query("([a-z");

    let state = panel.find_state().expect("find bar open");
    assert!(
        state.matches.is_empty(),
        "AC-003: no matches for an invalid regex"
    );
    assert!(
        !state.error.is_empty(),
        "AC-003: FindState.error carries the regex compile error"
    );

    // Fixing the pattern clears the error and finds the match.
    panel.set_find_query("[a-z]");
    let fixed = panel.find_state().unwrap();
    assert!(fixed.error.is_empty(), "a valid pattern clears the error");
    assert!(
        !fixed.matches.is_empty(),
        "the corrected pattern finds matches"
    );
}

#[test]
fn find_regex_error_catastrophic_pattern_does_not_hang() {
    // RISK-001: a classic catastrophic-backtracking pattern over a long input. The `regex` crate is
    // RE2-style (linear time), so this returns immediately; a backtracking engine (fancy-regex) would
    // hang for seconds. The test simply completing is the proof.
    let buf = TextBuffer::new(&"a".repeat(5000));
    let q = FindQuery {
        pattern: "(a+)+$".into(),
        is_regex: true,
        ..Default::default()
    };
    let matches = FindEngine::search(&q, &buf);
    assert_eq!(
        matches.len(),
        1,
        "the linear engine matches the whole run without hanging"
    );
}

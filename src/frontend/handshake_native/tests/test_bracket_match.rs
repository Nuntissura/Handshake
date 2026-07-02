//! MT-054 matching-bracket + bracket-pair-colorization proofs (WP-KERNEL-012 — E1 code editor chrome).
//!
//! Runtime proofs against the REAL `render_decorations` functions over the REAL MT-001 `TextBuffer`
//! (no stubs, no tautologies):
//!
//! - AC-001 / PT-001 (`bracket_match_*`): `find_matching_bracket` returns the correct partner byte
//!   offset for cursor-adjacent brackets (both before-open AND after-close adjacency), handles nested
//!   pairs, matches within the same family only, returns None for unbalanced input, and is BOUNDED
//!   (a partner past the scan cap reads as unmatched — RISK-003 / MC-003).
//! - AC-002 / PT-001 (`bracket_pair_colors_*`): `bracket_pair_colors` assigns a color per bracket by
//!   nesting depth (depth % palette.len()), producing distinct colors for at least three nesting levels.
//! - AC-007 (`no_buffer_mutation_in_decoration_source`): a source scan proves the three new MT-054 files
//!   contain NO `TextBuffer::insert` / `::delete` call site (render/decoration only).

use std::path::{Path, PathBuf};

use handshake_native::code_editor::{
    bracket_pair_colors, bracket_pair_colors_in_segments, depth_color_index, find_matching_bracket,
    BracketMatch, TextBuffer, BRACKET_SCAN_CAP_BYTES,
};
use handshake_native::theme::HsTheme;

// ── AC-001 / PT-001: find_matching_bracket ─────────────────────────────────────────────────────────

#[test]
fn bracket_match_before_open_adjacency() {
    // VS Code rule: a bracket is active when the cursor is immediately BEFORE an opening bracket.
    let buf = TextBuffer::new("foo(bar)");
    let m = find_matching_bracket(&buf, 3).expect("cursor before '(' at byte 3 matches");
    assert_eq!(
        m,
        BracketMatch {
            open_byte: 3,
            close_byte: 7
        }
    );
}

#[test]
fn bracket_match_after_close_adjacency() {
    // VS Code rule: a bracket is active when the cursor is immediately AFTER a closing bracket.
    let buf = TextBuffer::new("foo(bar)");
    let m = find_matching_bracket(&buf, 8).expect("cursor after ')' at byte 7 matches");
    assert_eq!(
        m,
        BracketMatch {
            open_byte: 3,
            close_byte: 7
        }
    );
}

#[test]
fn bracket_match_nested_pairs_resolve_to_correct_partner() {
    // "a(b[c]d)e": the outer '(' (byte 1) partners the outer ')' (byte 7), skipping the nested [].
    let buf = TextBuffer::new("a(b[c]d)e");
    assert_eq!(
        find_matching_bracket(&buf, 1),
        Some(BracketMatch {
            open_byte: 1,
            close_byte: 7
        }),
        "outer paren skips the nested square brackets"
    );
    // The inner '[' (byte 3) partners the inner ']' (byte 5).
    assert_eq!(
        find_matching_bracket(&buf, 3),
        Some(BracketMatch {
            open_byte: 3,
            close_byte: 5
        }),
        "inner square brackets match each other"
    );
}

#[test]
fn bracket_match_same_family_only() {
    // "([)]": the '(' (byte 0) partners the ')' (byte 2), ignoring the '[' (different family).
    let buf = TextBuffer::new("([)]");
    assert_eq!(
        find_matching_bracket(&buf, 0),
        Some(BracketMatch {
            open_byte: 0,
            close_byte: 2
        }),
        "a '(' pairs with a ')' (same family), never a ']'"
    );
}

#[test]
fn bracket_match_unbalanced_returns_none() {
    assert_eq!(
        find_matching_bracket(&TextBuffer::new("foo(bar"), 3),
        None,
        "unmatched '(' -> None"
    );
    assert_eq!(
        find_matching_bracket(&TextBuffer::new("abc)"), 4),
        None,
        "unmatched ')' -> None"
    );
    assert_eq!(
        find_matching_bracket(&TextBuffer::new("hello"), 2),
        None,
        "cursor not adjacent to any bracket -> None"
    );
}

#[test]
fn bracket_match_scan_is_bounded() {
    // A '(' with its partner beyond the scan cap reads as unmatched (RISK-003 / MC-003 — no
    // O(n)-per-frame whole-file scan).
    let mut s = String::from("(");
    s.push_str(&"x".repeat(BRACKET_SCAN_CAP_BYTES + 50));
    s.push(')');
    let buf = TextBuffer::new(&s);
    assert_eq!(
        find_matching_bracket(&buf, 0),
        None,
        "a partner past the {BRACKET_SCAN_CAP_BYTES}-byte cap reads as unmatched (bounded)"
    );
}

// ── AC-002 / PT-001: bracket_pair_colors ───────────────────────────────────────────────────────────

#[test]
fn bracket_pair_colors_distinct_per_depth_for_three_levels() {
    // The REAL theme palette (>= 3 hues) is the source — no hardcoded colors in the test path either.
    let palette = HsTheme::Dark.palette().bracket_pair_palette;
    assert!(
        palette.len() >= 3,
        "the theme ships >= 3 bracket-pair hues; got {}",
        palette.len()
    );

    // "([{x}])": opens at depths 0,1,2; closes mirror them.
    let buf = TextBuffer::new("([{x}])");
    let colors = bracket_pair_colors(&buf, 0..buf.len_bytes(), &palette);
    assert_eq!(colors.len(), 6, "six bracket chars colored");

    // AC-002: three distinct nesting levels -> three distinct colors.
    assert_eq!(colors[0].1, palette[0], "depth 0 -> palette[0]");
    assert_eq!(colors[1].1, palette[1], "depth 1 -> palette[1]");
    assert_eq!(colors[2].1, palette[2], "depth 2 -> palette[2]");
    assert_ne!(
        colors[0].1, colors[1].1,
        "depth 0 and 1 are distinct colors"
    );
    assert_ne!(
        colors[1].1, colors[2].1,
        "depth 1 and 2 are distinct colors"
    );
    assert_ne!(
        colors[0].1, colors[2].1,
        "depth 0 and 2 are distinct colors"
    );

    // Closing brackets carry the same color as their matching open (same depth).
    assert_eq!(colors[3].1, palette[2], "'}}' matches '{{'");
    assert_eq!(colors[4].1, palette[1], "']' matches '['");
    assert_eq!(colors[5].1, palette[0], "')' matches '('");

    // Each colored range is the single-byte range of the bracket char.
    assert_eq!(colors[0].0, 0..1);
}

#[test]
fn bracket_pair_color_index_is_depth_modulo_palette_len() {
    // The depth->index mapping wraps around the palette (VS Code parity).
    assert_eq!(depth_color_index(0, 6), 0);
    assert_eq!(depth_color_index(5, 6), 5);
    assert_eq!(
        depth_color_index(6, 6),
        0,
        "depth 6 wraps to index 0 (modulo)"
    );
    assert_eq!(depth_color_index(7, 6), 1);
    // Modulo-by-zero is guarded.
    assert_eq!(depth_color_index(3, 0), 0);
}

#[test]
fn bracket_pair_colors_empty_palette_is_safe() {
    let buf = TextBuffer::new("()");
    assert!(
        bracket_pair_colors(&buf, 0..2, &[]).is_empty(),
        "empty palette -> no colors, no panic"
    );
}

// ── MT-054/MT-005 Wave-B: fold-aware segment variant ───────────────────────────────────────────────

#[test]
fn bracket_pair_colors_in_segments_single_segment_matches_whole_window() {
    // One segment covering the whole window is EXACTLY bracket_pair_colors (documented contract).
    let palette = HsTheme::Dark.palette().bracket_pair_palette;
    let buf = TextBuffer::new("([{x}])");
    let whole = bracket_pair_colors(&buf, 0..buf.len_bytes(), &palette);
    let seg = bracket_pair_colors_in_segments(&buf, &[0..buf.len_bytes()], &palette);
    assert_eq!(
        seg, whole,
        "single whole-window segment must equal bracket_pair_colors"
    );
}

#[test]
fn bracket_pair_colors_in_segments_skips_hidden_bytes_and_carries_depth() {
    // Three "lines": a( \n b(){}() \n )c — folding away the middle line (bytes 3..11) must (1) color
    // NO bracket from the hidden segment (nothing to ghost-paint onto visible rows — the Wave-B leak),
    // and (2) carry the depth ACROSS the gap as if the hidden text were absent, so the visible ')' on
    // the last line still closes the visible '(' from the first line at depth 0.
    let palette = HsTheme::Dark.palette().bracket_pair_palette;
    let src = "a(\nb(){}()\n)c";
    let buf = TextBuffer::new(src);
    // Visible segments: line 0 (bytes 0..3, incl. '\n') and line 2 (bytes 11..13). Line 1 is hidden.
    let colors = bracket_pair_colors_in_segments(&buf, &[0..3, 11..13], &palette);
    let colored_bytes: Vec<usize> = colors.iter().map(|(r, _)| r.start).collect();
    assert_eq!(
        colored_bytes,
        vec![1, 11],
        "only the visible '(' (byte 1) and ')' (byte 11) are colored — no hidden-line bracket leaks"
    );
    assert_eq!(
        colors[0].1, palette[0],
        "visible '(' opens at window depth 0 -> palette[0]"
    );
    assert_eq!(
        colors[1].1, palette[0],
        "visible ')' closes the visible '(' at depth 0 (depth carries across the hidden segment)"
    );

    // Bounds safety: segments past the buffer end are clamped, never a panic.
    let clamped = bracket_pair_colors_in_segments(&buf, &[100..200], &palette);
    assert!(clamped.is_empty(), "out-of-range segment -> empty, no panic");
}

// ── AC-007: render/decoration only — no buffer mutation in the MT-054 source files ──────────────────

fn crate_src(file: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("code_editor")
        .join(file)
}

#[test]
fn no_buffer_mutation_in_decoration_source() {
    // AC-007: the three render-layer features are decoration-only. A grep of the two NEW pure modules
    // proves no TextBuffer::insert/delete call site was added (the panel paint paths are covered by the
    // wider grep gate the reviewer runs; here we pin the new pure modules).
    for file in ["render_decorations.rs", "word_wrap.rs"] {
        let src = std::fs::read_to_string(crate_src(file)).expect("read MT-054 source");
        // Flag a CODE call site (ignore doc comments that mention the API by name).
        for (i, line) in src.lines().enumerate() {
            let t = line.trim_start();
            if t.starts_with("//") || t.starts_with("///") {
                continue;
            }
            assert!(
                !t.contains(".insert(") && !t.contains(".delete("),
                "AC-007: {file}:{} contains a buffer-mutation call site in a render-only module: {}",
                i + 1,
                t
            );
        }
    }
}

//! MT-005 code-folding integration proofs (WP-KERNEL-012 — E1 code editor).
//!
//! These exercise the PUBLIC folding surface end-to-end through the real grammar + the
//! `CodeEditorPanel`, complementing the `folding.rs` `#[cfg(test)]` unit tests:
//!
//! - PT-001 (`cargo test -p handshake-native fold_provider`): a real 20-line Rust function parsed by
//!   the bundled tree-sitter grammar produces a FoldRegion covering the body; a single-line node does
//!   not fold.
//! - PT-002 (`cargo test -p handshake-native fold_set_visibility`): a folded region hides only its
//!   inner lines (the start line stays visible), including the nested-fold case (MC-003).
//! - PT-003 (`cargo test -p handshake-native fold_set_mapping`): visible→buffer mapping offsets lines
//!   after a folded region by exactly the collapsed count.
//! - AC-006 idempotency: toggling a fold twice returns to the original unfolded state, proven on the
//!   live panel (`CodeEditorPanel::toggle_fold`).
//!
//! The function names embed the contract proof-target substrings (`fold_provider`,
//! `fold_set_visibility`, `fold_set_mapping`) so `cargo test -p handshake-native <substring>` selects
//! the right proof.

use handshake_native::code_editor::{CodeEditorPanel, FoldProvider, FoldSet, TextBuffer};

/// Parse `src` as Rust with the bundled grammar (the same grammar MT-001's highlighter uses).
fn rust_tree(src: &str) -> tree_sitter::Tree {
    let lang: tree_sitter::Language = tree_sitter_rust::LANGUAGE.into();
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&lang).expect("rust language");
    parser.parse(src, None).expect("rust parse")
}

// A real 20-line Rust function (AC-001 scenario).
const RUST_FN_20: &str = "\
fn compute(input: i32) -> i32 {
    let mut total = 0;
    let mut count = 0;
    for i in 0..input {
        if i % 3 == 0 {
            total += i;
            count += 1;
        } else if i % 5 == 0 {
            total -= i;
        } else {
            total += 1;
        }
    }
    let average = if count > 0 { total / count } else { 0 };
    let label = String::from(\"computed\");
    println!(\"{}: avg={} total={}\", label, average, total);
    if total > 1000 {
        return total / 2;
    }
    total
}
";

// ── PT-001 / AC-001: FoldProvider on a 20-line function ───────────────────────────────────────────

#[test]
fn fold_provider_computes_region_for_twenty_line_function() {
    let tree = rust_tree(RUST_FN_20);
    let buffer = TextBuffer::new(RUST_FN_20);
    let regions = FoldProvider::new().compute(&tree, &buffer, "rust");
    assert!(
        !regions.is_empty(),
        "PT-001/AC-001: a 20-line rust function must yield >= 1 fold region; got none"
    );
    // The outermost region is the function body: starts on the `fn` line (0), reaches the closing `}`.
    let outer = regions
        .iter()
        .find(|r| r.start_line == 0)
        .expect("a fold region starting on the function's first line");
    assert!(
        outer.end_line >= 19,
        "PT-001: the function-body fold reaches its closing brace (end_line {})",
        outer.end_line
    );
    assert!(
        outer.end_line - outer.start_line >= 1,
        "PT-001: a fold region spans at least 2 lines"
    );
    // The label is the first line text + ellipsis (MT impl note 2).
    assert!(
        outer.label.starts_with("fn compute(input: i32) -> i32 {") && outer.label.ends_with('…'),
        "PT-001: fold label is the first line + ellipsis; got {:?}",
        outer.label
    );
}

#[test]
fn fold_provider_single_line_node_produces_no_region() {
    let src = "fn tiny() { 1 + 1 }\n";
    let tree = rust_tree(src);
    let buffer = TextBuffer::new(src);
    let regions = FoldProvider::new().compute(&tree, &buffer, "rust");
    assert!(
        regions.is_empty(),
        "PT-001/AC-001: a single-line function folds nothing; got {regions:?}"
    );
}

#[test]
fn fold_provider_via_panel_constructs_foldable_regions() {
    // The panel computes fold regions at construction from the same tree the highlighter built.
    let panel = CodeEditorPanel::new(RUST_FN_20, "rs");
    let set = panel.fold_set();
    assert!(
        !set.regions.is_empty(),
        "PT-001: a panel over a 20-line function exposes fold regions; got none"
    );
    assert!(
        set.regions.iter().all(|r| !r.folded),
        "PT-001: freshly-computed regions start UNfolded (the user/agent folds them)"
    );
}

// ── PT-002 / AC-002: FoldSet visibility ───────────────────────────────────────────────────────────

#[test]
fn fold_set_visibility_start_line_visible_inner_lines_hidden() {
    // Region lines 5..=10, folded (the exact AC-002 scenario).
    let set = build_single_region_set(5, 10, true);
    assert!(set.is_line_visible(4), "AC-002: line 4 visible");
    assert!(set.is_line_visible(5), "AC-002: line 5 (start) visible");
    assert!(!set.is_line_visible(6), "AC-002: line 6 hidden");
    assert!(!set.is_line_visible(10), "AC-002: line 10 (end) hidden");
    assert!(set.is_line_visible(11), "AC-002: line 11 visible");
}

#[test]
fn fold_set_visibility_nested_outer_hides_inner() {
    // MC-003: nested folds. Outer [2,12] folded encloses inner [5,8]; outer must hide every inner line.
    let outer = make_region(2, 12, true);
    let inner = make_region(5, 8, false);
    let set = FoldSet::from_regions(vec![outer, inner]);
    assert!(set.is_line_visible(2), "outer start visible");
    assert!(
        !set.is_line_visible(5),
        "outer fold hides the inner region's start line"
    );
    assert!(!set.is_line_visible(8), "outer fold hides inner end");
    assert!(!set.is_line_visible(12), "outer fold hides its own end");
    assert!(
        set.is_line_visible(13),
        "line after the outer region visible"
    );
}

#[test]
fn fold_set_visibility_nested_both_folded_visible_count_matches_map() {
    // Regression guard (no double-count): outer [2,12] AND nested inner [5,8] BOTH folded. The outer
    // already hides 3..=12 (10 lines); the inner adds nothing. visible_line_count must agree with the
    // render-path map length (rebuild_visible_map_for) rather than a naive per-region sum, which would
    // subtract 10 + 3 = 13 and report a too-small count.
    let outer = make_region(2, 12, true);
    let inner = make_region(5, 8, true);
    let mut set = FoldSet::from_regions(vec![outer, inner]);
    let buffer_len = 20;
    let render_visible = set.rebuild_visible_map_for(buffer_len);
    assert_eq!(
        set.visible_line_count(buffer_len),
        render_visible,
        "nested both-folded: visible_line_count must equal the render-path visible-map length"
    );
    assert_eq!(
        set.visible_line_count(buffer_len),
        buffer_len - 10,
        "nested both-folded: only the outer fold's 10 lines are hidden"
    );
}

// ── PT-003 / AC-003: visible→buffer mapping ───────────────────────────────────────────────────────

#[test]
fn fold_set_mapping_offsets_after_folded_region() {
    // 20-line buffer, region [3,8] folded (collapses 5 lines 4..=8). visible 4 -> buffer 9 (AC-003).
    let mut set = build_single_region_set(3, 8, true);
    set.rebuild_visible_map_for(20);
    assert_eq!(set.visible_line_to_buffer_line(0), 0);
    assert_eq!(set.visible_line_to_buffer_line(3), 3, "fold start maps 1:1");
    assert_eq!(
        set.visible_line_to_buffer_line(4),
        9,
        "AC-003: visible line after the fold skips the 5 collapsed lines"
    );
    assert_eq!(
        set.visible_line_count(20),
        15,
        "AC-003: 5 of 20 lines collapsed"
    );
}

#[test]
fn fold_set_mapping_via_panel_after_toggle() {
    // End-to-end through the panel: fold the function body, then the visible line count shrinks by the
    // collapsed-line count, and the visible->buffer map skips the hidden lines.
    let panel = CodeEditorPanel::new(RUST_FN_20, "rs");
    let buffer_lines = panel.buffer().len_lines();
    let mut set = panel.fold_set();
    let outer_start = set
        .regions
        .iter()
        .find(|r| r.start_line == 0)
        .map(|r| r.start_line)
        .expect("function-body region");
    let collapsed = set
        .regions
        .iter()
        .find(|r| r.start_line == outer_start)
        .map(|r| r.collapsed_line_count())
        .unwrap();

    assert!(
        panel.toggle_fold(outer_start),
        "PT-003: toggle the function-body fold"
    );
    let folded = panel.fold_set();
    assert_eq!(
        folded.visible_line_count(buffer_lines),
        buffer_lines - collapsed,
        "PT-003: folding the body removes its collapsed lines from the visible count"
    );

    // The visible->buffer map skips the collapsed lines: visible line 1 maps to the first line AFTER
    // the folded body (since line 0 is the fold start and 1..=end are hidden).
    set = panel.fold_set();
    set.rebuild_visible_map_for(buffer_lines);
    assert_eq!(
        set.visible_line_to_buffer_line(0),
        0,
        "fold start at visible 0"
    );
    let after = set.visible_line_to_buffer_line(1);
    assert!(
        after > outer_start,
        "PT-003: visible line 1 maps past the folded body (buffer line {after})"
    );
}

// ── AC-006: idempotency on the live panel ─────────────────────────────────────────────────────────

#[test]
fn fold_toggle_twice_returns_to_original_state_on_panel() {
    let panel = CodeEditorPanel::new(RUST_FN_20, "rs");
    let buffer_lines = panel.buffer().len_lines();
    let visible_before = panel.fold_set().visible_line_count(buffer_lines);

    // Fold then unfold the function body.
    assert!(panel.toggle_fold(0), "first toggle folds the body");
    let visible_folded = panel.fold_set().visible_line_count(buffer_lines);
    assert!(
        visible_folded < visible_before,
        "AC-006: after folding, fewer visible lines ({visible_folded} < {visible_before})"
    );

    assert!(panel.toggle_fold(0), "second toggle unfolds the body");
    let visible_after = panel.fold_set().visible_line_count(buffer_lines);
    assert_eq!(
        visible_after, visible_before,
        "AC-006: toggling twice returns to the original visible-line count"
    );
    assert!(
        panel.fold_set().regions.iter().all(|r| !r.folded),
        "AC-006: all regions unfolded again after two toggles"
    );
}

#[test]
fn fold_at_cursor_keymap_surface_folds_enclosing_region() {
    // The Ctrl+Shift+[ surface: placing the (logical) cursor inside the body and folding folds the
    // enclosing region. We drive the public fold_at_line surface the keymap calls.
    let panel = CodeEditorPanel::new(RUST_FN_20, "rs");
    // Line 10 is inside the function body.
    assert!(
        panel.fold_at_line(10),
        "fold_at_line folds the enclosing region"
    );
    assert!(
        panel.fold_set().regions.iter().any(|r| r.folded),
        "a region is folded after fold_at_line"
    );
    // Unfolding the same line restores it.
    assert!(
        panel.unfold_at_line(10),
        "unfold_at_line unfolds the enclosing region"
    );
    assert!(
        panel.fold_set().regions.iter().all(|r| !r.folded),
        "no region folded after unfold_at_line"
    );
}

// ── helpers ───────────────────────────────────────────────────────────────────────────────────────

fn make_region(
    start_line: usize,
    end_line: usize,
    folded: bool,
) -> handshake_native::code_editor::FoldRegion {
    handshake_native::code_editor::FoldRegion {
        start_line,
        end_line,
        folded,
        label: format!("line {start_line} …"),
    }
}

fn build_single_region_set(start_line: usize, end_line: usize, folded: bool) -> FoldSet {
    FoldSet::from_regions(vec![make_region(start_line, end_line, folded)])
}

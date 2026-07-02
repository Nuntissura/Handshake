//! MT-054 word-wrap proofs (WP-KERNEL-012 — E1 code editor chrome).
//!
//! Runtime proofs against the REAL `word_wrap` layout math + the REAL `CodeEditorPanel` (no stubs, no
//! tautologies):
//!
//! - AC-003 / PT-003 (`wrap_layout_*`): `layout_visual_rows` with wrap DISABLED maps each logical line
//!   to exactly one VisualRow (wrap_index=0); with wrap ENABLED a long line is split into N>1
//!   contiguous, non-overlapping VisualRows whose byte ranges cover the whole logical line.
//! - AC-004 (`wrap_scroll_math_counts_visual_rows`): the visible-row COUNT the scroll math drives is the
//!   number of VISUAL rows under wrap (so the scrollbar/first-visible-row/row-count reflect wrapped
//!   rows), strictly greater than the logical-line count for a document with a long line.
//! - AC-005 (`alt_z_toggles_wrap_without_inserting_z` + `wrap_off_is_baseline_one_to_one`): Alt+Z flips
//!   `WrapConfig.enabled` (persisted on the panel) and inserts NO literal 'z' into the buffer; with wrap
//!   off the layout is the strict MT-002 1:1 identity (regression guard).

use std::sync::Arc;

use egui::{Key, Modifiers};
use egui_kittest::Harness;

use handshake_native::code_editor::{layout_visual_rows, CodeEditorPanel, TextBuffer, WrapConfig};

fn off() -> WrapConfig {
    WrapConfig::default()
}

fn on_cols(cols: usize) -> WrapConfig {
    WrapConfig {
        enabled: true,
        wrap_column: Some(cols),
        viewport_width_px: 0.0,
    }
}

// ── AC-003 / PT-003: layout math ──────────────────────────────────────────────────────────────────

#[test]
fn wrap_layout_disabled_is_one_to_one() {
    let buf = TextBuffer::new("alpha\nbeta\ngamma");
    let rows = layout_visual_rows(&buf, 0..buf.len_lines(), &off(), 8.0);
    assert_eq!(
        rows.len(),
        3,
        "AC-003: 3 logical lines -> 3 visual rows when wrap is off"
    );
    for (i, r) in rows.iter().enumerate() {
        assert_eq!(r.logical_line, i, "row {i} maps to logical line {i}");
        assert_eq!(
            r.wrap_index, 0,
            "AC-003: every row is wrap_index 0 under 1:1"
        );
    }
}

#[test]
fn wrap_layout_enabled_splits_long_line_contiguously() {
    // A 200-char line at wrap width ~80 -> ceil(200/80) = 3 contiguous, non-overlapping rows covering
    // the whole logical line (AC-003 exact wording).
    let line = "a".repeat(200);
    let buf = TextBuffer::new(&line);
    let rows = layout_visual_rows(&buf, 0..1, &on_cols(80), 8.0);
    assert_eq!(
        rows.len(),
        3,
        "AC-003: 200 chars at width 80 -> 3 rows; got {}",
        rows.len()
    );

    // Contiguous + non-overlapping.
    assert_eq!(rows[0].byte_start, 0, "first row starts at the line start");
    for w in rows.windows(2) {
        assert_eq!(
            w[0].byte_end, w[1].byte_start,
            "AC-003: fragments are contiguous (no gap, no overlap)"
        );
    }
    // Union covers the whole logical line.
    assert_eq!(
        rows.last().unwrap().byte_end,
        buf.len_bytes(),
        "AC-003: the visual rows cover the whole logical line"
    );
    // wrap_index increments 0,1,2 over one logical line.
    for (i, r) in rows.iter().enumerate() {
        assert_eq!(r.logical_line, 0);
        assert_eq!(r.wrap_index, i, "AC-003: fragment indices are 0..N");
    }
}

#[test]
fn wrap_layout_soft_breaks_at_whitespace() {
    // "aaaa bbbb cccc" at width 6 soft-breaks after the space.
    let buf = TextBuffer::new("aaaa bbbb cccc");
    let rows = layout_visual_rows(&buf, 0..1, &on_cols(6), 8.0);
    assert!(rows.len() >= 2, "a 14-char line at width 6 wraps");
    let first = buf.byte_slice_to_string(rows[0].byte_range());
    assert_eq!(
        first, "aaaa ",
        "soft break keeps the trailing space; got {first:?}"
    );
    assert_eq!(
        rows.last().unwrap().byte_end,
        buf.len_bytes(),
        "full coverage"
    );
}

// ── AC-004: scroll math counts visual rows ─────────────────────────────────────────────────────────

#[test]
fn wrap_scroll_math_counts_visual_rows() {
    // The row count the scroll math strides over is the VISUAL-row count under wrap. A doc with one
    // 200-char line + two short lines yields 5 visual rows on (3+1+1) vs 3 logical lines off.
    let buf = TextBuffer::new(&format!("{}\nshort\nx", "a".repeat(200)));
    let off_rows = layout_visual_rows(&buf, 0..buf.len_lines(), &off(), 8.0);
    let on_rows = layout_visual_rows(&buf, 0..buf.len_lines(), &on_cols(80), 8.0);
    assert_eq!(
        off_rows.len(),
        3,
        "wrap off -> 3 logical rows (scroll math counts lines)"
    );
    assert_eq!(
        on_rows.len(),
        5,
        "AC-004: wrap on -> 5 visual rows (scroll math counts visual rows)"
    );
    assert!(
        on_rows.len() > off_rows.len(),
        "AC-004: the scrollbar extent grows under wrap so scrolling a wrapped doc lands correctly"
    );
}

// ── AC-005: Alt+Z toggle, persistence, no stray 'z', baseline 1:1 ──────────────────────────────────

#[test]
fn wrap_off_is_baseline_one_to_one() {
    // Even an absurdly narrow viewport never wraps when disabled (the strict MT-002 baseline fast path).
    let buf =
        TextBuffer::new("a long single logical line that would wrap if word wrap were enabled");
    let cfg = WrapConfig {
        enabled: false,
        wrap_column: None,
        viewport_width_px: 1.0,
    };
    let rows = layout_visual_rows(&buf, 0..1, &cfg, 8.0);
    assert_eq!(
        rows.len(),
        1,
        "AC-005: wrap off -> exactly one row regardless of width (baseline)"
    );
    assert_eq!(rows[0].byte_range(), 0..buf.len_bytes());
    assert_eq!(rows[0].wrap_index, 0);
}

#[test]
fn alt_z_toggles_wrap_without_inserting_z() {
    // Drive the REAL panel through egui_kittest. Alt+Z must flip WrapConfig.enabled (persisted on the
    // panel) and must NOT insert a literal 'z' into the buffer (RISK-005 / MC-005 — consume_shortcut
    // before the typing loop).
    let panel = Arc::new(CodeEditorPanel::new(
        "fn main() {\n    let x = 1;\n}\n",
        "rs",
    ));
    let original = panel.buffer().to_string();
    assert!(
        !panel.is_wrap_enabled(),
        "wrap starts OFF (the MT-002 baseline default)"
    );

    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 480.0))
        .build_ui(move |ui| {
            panel_ui.show(ui);
        });
    harness.run();

    // Press Alt+Z (down). The panel's show() consumes it via consume_shortcut and flips wrap.
    harness.event(egui::Event::Key {
        key: Key::Z,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::ALT,
    });
    harness.run();

    assert!(
        panel.is_wrap_enabled(),
        "AC-005: Alt+Z flipped WrapConfig.enabled ON (persisted)"
    );
    assert_eq!(
        panel.buffer().to_string(),
        original,
        "AC-005 / MC-005: Alt+Z inserted NO literal 'z' (consume_shortcut before the typing loop)"
    );

    // A second Alt+Z flips it back OFF — proving the toggle is a real flip, not a one-way set.
    harness.event(egui::Event::Key {
        key: Key::Z,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::ALT,
    });
    harness.run();
    assert!(
        !panel.is_wrap_enabled(),
        "AC-005: a second Alt+Z toggled wrap back OFF"
    );
    assert_eq!(
        panel.buffer().to_string(),
        original,
        "still no stray 'z' after the second toggle"
    );
}

#[test]
fn wrap_toggle_persists_and_is_addressable_by_author_id() {
    // AC-005 + HBR-SWARM: the toggle routes through the SAME mutation point Alt+Z uses, and a swarm agent
    // can flip it by the contract-named `editor-wrap-toggle` author_id.
    let panel = CodeEditorPanel::new("x", "rs");
    assert!(!panel.is_wrap_enabled());
    let now_on = panel.toggle_wrap_by_author_id("editor-wrap-toggle");
    assert_eq!(now_on, Some(true), "dispatch-by-id flips wrap ON");
    assert!(
        panel.is_wrap_enabled(),
        "the flip persists on the panel state"
    );
    // An unknown id is a benign no-op (None), not a panic.
    assert_eq!(panel.toggle_wrap_by_author_id("editor-wrap-nope"), None);
    assert!(
        panel.is_wrap_enabled(),
        "an unmatched id did not change the state"
    );
}

#[test]
fn wrap_paint_is_bounded_to_window_on_large_document() {
    // PERF CAP (adversarial-review hardening): under word wrap the per-FRAME paint path must materialize
    // only the LOGICAL lines that intersect the on-screen visual-row window — O(window) — NOT re-wrap the
    // whole post-fold document every frame (the O(document)/frame regression the review caught). A 4000-
    // line doc, each line long enough to wrap into several visual rows, is painted into a fixed-size
    // harness; `frame_lines_wrapped` must stay a small fraction of the document line count across repeated
    // (scroll/hover/idle-equivalent) frames, proving the cached prefix-sum index + lazy window
    // materialization, not a full-document re-wrap.
    let line = "let value = ".to_owned() + &"abcdefghij ".repeat(12); // ~140 chars -> several wrap rows
    let src = (0..4000)
        .map(|_| line.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let total_lines = src.matches('\n').count() + 1;
    assert!(
        total_lines >= 4000,
        "large document built; got {total_lines} lines"
    );

    let panel = Arc::new(CodeEditorPanel::new(&src, "rs"));
    panel.set_wrap_enabled(true);
    panel.set_wrap_column(Some(40)); // force a deterministic narrow wrap independent of the viewport

    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(640.0, 400.0))
        .build_ui(move |ui| {
            panel_ui.show(ui);
        });

    // Run several frames (the first builds the cached index; later frames are cache hits — exactly the
    // scroll/hover/idle repaints the regression made O(document)).
    for _ in 0..4 {
        harness.run();
    }

    let stats = panel.perf_stats();
    assert_eq!(
        stats.buffer_len_lines, total_lines,
        "whole document line count reported"
    );
    assert!(
        stats.frame_lines_rendered > 0,
        "the wrap path painted a non-empty window"
    );
    // The load-bearing assertion: the wrap paint touched only a window's worth of logical lines, NOT the
    // whole document. A 400px viewport at ~13px rows shows well under 100 visual rows; each logical line
    // wraps into several of them, so the painted logical lines are far fewer still. A generous cap of 200
    // is orders of magnitude below the 4000-line document and would FAIL hard under the old full-document
    // re-wrap (which materialized all 4000 every frame).
    assert!(
        stats.frame_lines_wrapped > 0,
        "wrap on -> the paint path materialized at least one logical line"
    );
    assert!(
        stats.frame_lines_wrapped <= 200,
        "PERF CAP: wrap paint must touch only O(window) logical lines, not O(document); touched {} of {}",
        stats.frame_lines_wrapped,
        stats.buffer_len_lines
    );
    assert!(
        stats.frame_lines_wrapped < stats.buffer_len_lines / 10,
        "PERF CAP: paint touched {} logical lines, far below the {}-line document (no full-document re-wrap)",
        stats.frame_lines_wrapped,
        stats.buffer_len_lines
    );
}

#[test]
fn wrap_off_reports_zero_lines_wrapped() {
    // The non-wrap baseline path never enters the wrap materializer, so `frame_lines_wrapped` is 0 — the
    // MT-002 baseline render is untouched by the perf-cap plumbing (RISK-006 / MC-006).
    let panel = Arc::new(CodeEditorPanel::new(
        "fn main() {\n    let x = 1;\n}\n",
        "rs",
    ));
    assert!(!panel.is_wrap_enabled());
    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(640.0, 400.0))
        .build_ui(move |ui| {
            panel_ui.show(ui);
        });
    harness.run();
    harness.run();
    let stats = panel.perf_stats();
    assert_eq!(
        stats.frame_lines_wrapped, 0,
        "wrap OFF -> the wrap paint path never ran; got {stats:?}"
    );
}

#[test]
fn live_panel_renders_under_wrap_without_panic() {
    // Drive the REAL panel with wrap ENABLED + a forced narrow wrap column so the wrap render path
    // (render_wrapped_rows) actually runs against a long line. Proves the scroll-row-count + per-row
    // paint integration does not panic and the panel reports a non-empty painted window.
    let long = "let value = ".to_owned() + &"abcdefghij ".repeat(40);
    let src = format!("fn demo() {{\n    {long}\n}}\n");
    let panel = Arc::new(CodeEditorPanel::new(&src, "rs"));
    panel.set_wrap_enabled(true);
    panel.set_wrap_column(Some(40));

    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(640.0, 400.0))
        .build_ui(move |ui| {
            panel_ui.show(ui);
        });
    harness.run();
    harness.run();

    assert!(panel.is_wrap_enabled(), "wrap stayed enabled across frames");
    let stats = panel.perf_stats();
    assert!(
        stats.frame_lines_rendered > 0,
        "the wrap render path painted a non-empty window; got {stats:?}"
    );
}

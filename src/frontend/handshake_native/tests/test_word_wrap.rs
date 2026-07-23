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
use egui_kittest::kittest::{NodeT, Queryable};
#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;
use screenshot_harness::ScreenshotHarness as Harness;

use handshake_native::code_editor::{
    layout_visual_rows, CodeEditorPanel, Cursor, TextBuffer, WrapConfig,
    CODE_EDITOR_CURSOR_AUTHOR_PREFIX, CODE_EDITOR_TEXT_AUTHOR_ID,
};

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

#[test]
fn wrapped_text_input_keeps_argus_write_actions() {
    let panel = CodeEditorPanel::new("wrapped text", "rs");
    panel.set_wrap_enabled(true);
    panel.set_wrap_column(Some(8));
    let mut harness = Harness::new_ui(|ui| panel.show(ui));
    harness.run_steps(3);
    let node = harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(CODE_EDITOR_TEXT_AUTHOR_ID))
        .expect("wrapped code editor text input remains addressable");
    assert!(
        node.accesskit_node()
            .data()
            .supports_action(egui::accesskit::Action::SetValue),
        "word wrap must not remove Argus SetValue"
    );
    assert!(
        node.accesskit_node()
            .data()
            .supports_action(egui::accesskit::Action::ReplaceSelectedText),
        "word wrap must not remove Argus ReplaceSelectedText"
    );
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

// ── MT-054 Task-B: wrap mode renders caret / selection / caret AccessKit node ─────────────────────

#[test]
fn wrap_mode_renders_caret_and_selection_at_visual_row() {
    // Task-B regression: with word wrap ON, the caret, the selection, and the per-caret AccessKit node
    // must RENDER (the wrap paint path previously painted NONE of them — no caret, no selection, no
    // find-match, no whitespace, no caret node). The first line wraps into 2 visual rows, so the second
    // logical line ("TARGET") is pushed to VISUAL ROW 2 — its visual-row index differs from its logical
    // line index, which is exactly what the wrap-aware byte->visual-row overlay mapping must get right.
    let src = "wwww wwww wwww wwww\nTARGET\n";
    let panel = Arc::new(CodeEditorPanel::new(src, "txt"));
    panel.set_wrap_enabled(true);
    panel.set_wrap_column(Some(10)); // deterministic narrow wrap: line 0 -> 2 rows, "TARGET" -> row 2
                                     // A selection over "TARGET" (bytes 20..26); head=26 places the caret at line 1 col 6 (just past the
                                     // last 'T', an empty cell — so the caret bar stands alone, not overlapping a glyph).
    panel.set_cursors(vec![Cursor::selection(20, 26)]);

    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(400.0, 300.0))
        .build_ui(move |ui| {
            panel_ui.show(ui);
        });
    harness.run();
    harness.run();

    // ── Non-GPU proofs (always run) ──────────────────────────────────────────────────────────────
    assert!(panel.is_wrap_enabled(), "wrap stayed enabled across frames");
    let stats = panel.perf_stats();
    assert!(
        stats.frame_lines_wrapped > 0,
        "the wrap render path ran (materialized >=1 logical line); got {stats:?}"
    );
    assert_eq!(
        panel.primary_selection_bytes(),
        (20, 26),
        "the selection over 'TARGET' is set (bytes 20..26)"
    );

    // Structural (non-GPU): the wrap layout math places logical line 1 ("TARGET") on VISUAL ROW 2 (line 0
    // wraps into rows 0-1), so a correct overlay must paint the caret on visual row 2 — NOT logical row 1
    // (the non-wrap mapping, which under wrap has an empty painted_lines list and would draw nothing).
    let buf = panel.buffer();
    let gw = panel
        .measured_glyph_width()
        .expect("glyph width measured after a frame");
    let rows = layout_visual_rows(&buf, 0..buf.len_lines(), &on_cols(10), gw);
    let target_row = rows
        .iter()
        .position(|r| r.logical_line == 1 && r.is_first_fragment())
        .expect("logical line 1 has a visual row");
    assert_eq!(
        target_row, 2,
        "line 0 wraps into 2 visual rows -> 'TARGET' (line 1) is on visual row 2; got {target_row}"
    );

    // A per-caret AccessKit node EXISTS under wrap (a swarm agent can still address the caret by id),
    // carrying the caret's LOGICAL (line, col) — the same node render_rows emits in the non-wrap path.
    let cursor0_id = format!("{CODE_EDITOR_CURSOR_AUTHOR_PREFIX}0");
    let mut caret_value: Option<String> = None;
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        if ak.author_id() == Some(cursor0_id.as_str()) {
            caret_value = ak.value().map(|v| v.to_owned());
        }
    }
    assert_eq!(
        caret_value.as_deref(),
        Some("line 1 col 6"),
        "Task-B: the caret AccessKit node exists in wrap mode at the selection head (line 1 col 6)"
    );

    // ── Pixel proof: the caret overlay rect is painted at the correct VISUAL row ─────────────────────
    // "TARGET" is one label under plain 'txt' (one label per visual row) and egui laid it out at its
    // wrapped visual-row position, so its rect IS that row's band + column origin. The caret at col 6
    // sits at the label's right edge; a caret bar there proves the overlay mapped the head byte to THIS
    // wrapped row/col. (`get_by_label` reads layout, unaffected by the painter overlays drawn on top.)
    let target = harness.get_by_label("TARGET").rect();
    let image = harness
        .render()
        .expect("Task-B: a wgpu adapter is required to prove the wrapped caret pixel");
    let (w, h) = (image.width(), image.height());
    let ppp = harness.ctx.pixels_per_point();
    let caret_x = target.right(); // col-6 x = right edge of the 6-glyph "TARGET" label
                                  // Count pixels in a thin strip at the caret x within the "TARGET" row band that differ from the SAME
                                  // column's background reference far BELOW the 4-row document (uniform panel background).
    let count_non_bg = |x_pt: f32, y0_pt: f32, y1_pt: f32, bg_y_pt: f32| -> u32 {
        let x_px = (x_pt * ppp).round() as i64;
        let bg_y = (bg_y_pt * ppp).round().clamp(0.0, (h - 1) as f32) as u32;
        let mut count = 0u32;
        for dx in -1..=2i64 {
            let x = x_px + dx;
            if x < 0 || x >= w as i64 {
                continue;
            }
            let bg = image.get_pixel(x as u32, bg_y).0;
            let y0 = (y0_pt * ppp).round().max(0.0) as u32;
            let y1 = ((y1_pt * ppp).round() as u32).min(h - 1);
            for y in y0..y1 {
                if image.get_pixel(x as u32, y).0 != bg {
                    count += 1;
                }
            }
        }
        count
    };
    let band_h = target.height().max(1.0);
    let caret_pixels = count_non_bg(
        caret_x,
        target.top() + 1.0,
        target.bottom() - 1.0,
        target.bottom() + 6.0 * band_h, // far below the 4-row document -> panel background
    );
    assert!(
        caret_pixels > 0,
        "Task-B: the caret bar is painted at the 'TARGET' visual row, col 6 (x={caret_x:.1}); got 0 \
         non-background pixels (the wrap overlay did not render the caret at the wrapped row)"
    );
}

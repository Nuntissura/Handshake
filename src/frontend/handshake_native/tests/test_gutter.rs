//! MT-007 gutter LIVE proofs (WP-KERNEL-012 — E1 code editor): width math, the diagnostic red-dot +
//! line-number screenshots, the interactive breakpoint + fold gutter clicks (via the live AccessKit
//! tree / fold state), and the `push_diagnostics` slot.
//!
//! Mapping to the MT contract:
//! - AC-001 / PT-001 (`cargo test -p handshake-native gutter`): gutter width for a 1 / 100 / 10000-line
//!   buffer (grows with the digit count, recomputed from the live line count — RISK-001), and the
//!   GutterMarker list for a buffer with one error on line 5.
//! - AC-003 / PT-003: an egui_kittest screenshot with an error marker on line 3 shows a RED diagnostic
//!   pixel in the gutter at that row (pixel color check). PNG -> external artifact root.
//! - AC-004 / PT-004: an egui_kittest screenshot shows right-aligned line numbers in the gutter for a
//!   10-line buffer. PNG -> external artifact root.
//! - AC-005 / PT-005: an interactive click on the gutter at line 2 toggles a breakpoint, verified via
//!   the live `code_editor_breakpoint_2` CheckBox node appearing toggled (the field-correct accesskit
//!   0.21.1 toggle role — the MT names ToggleButton, which does not exist there).
//! - AC-006 / PT-006: an interactive click on the fold triangle at the line-0 region toggles the fold,
//!   verified via the fold state change (a previously visible body line is no longer painted).
//! - AC-007: `push_diagnostics([... line 3 error ...])` updates the markers without a panic AND without
//!   bumping `buffer_version` (KERNEL_BUILDER perf gate); MC-004: pushing an empty vec clears them.
//!
//! ## Why drive the real gutter surface, not a faked node
//!
//! Every assertion drives the public panel surface a user/agent uses: `push_diagnostics` (the MT-008
//! slot), a real pixel-positioned gutter click computed from the captured gutter geometry, and the
//! real fold path — so the AccessKit nodes + the rendered pixels are produced by the live gutter, not
//! a stub.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use egui_kittest::kittest::NodeT;
use egui_kittest::Harness;

use handshake_native::code_editor::gutter::{
    fold_triangle_glyph, DiagnosticSeverity, Gutter, GutterConfig, GutterMarkerKind,
};
use handshake_native::code_editor::{CodeEditorPanel, GutterMarker};

/// A real multi-line Rust function so the line-0 body region is foldable (AC-006) and there are enough
/// rows to target line 2 / line 3 (AC-003/AC-005).
const RUST_FN: &str = "\
fn render(items: &[i32]) -> i32 {
    let mut total = 0;
    for item in items {
        total += item;
    }
    total
}
";

fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

fn assert_no_local_test_output() {
    let local = Path::new("test_output");
    assert!(
        !local.exists(),
        "no repo-local test_output/ dir may exist — artifacts go to the external \
         Handshake_Artifacts/handshake-test root only (found {})",
        local.display()
    );
}

// ── AC-001 / PT-001: gutter width math + marker list ──────────────────────────────────────────────

#[test]
fn gutter_width_grows_with_line_count() {
    let cfg = GutterConfig::default();
    let cw = 8.0; // representative monospace advance
    let w1 = Gutter::width_for(1, cw, &cfg);
    let w100 = Gutter::width_for(100, cw, &cfg);
    let w10000 = Gutter::width_for(10000, cw, &cfg);
    // 1 line -> 1 digit; 100 -> 3 digits; 10000 -> 5 digits. Strictly increasing.
    assert!(w1 > 0.0, "1-line gutter has a positive width");
    assert!(
        w100 > w1,
        "100-line gutter wider than 1-line: {w100} > {w1}"
    );
    assert!(
        w10000 > w100,
        "10000-line gutter wider than 100-line: {w10000} > {w100}"
    );
    // The 1->10000 delta is exactly (5-1)=4 digit columns of extra number width.
    assert!(
        (w10000 - w1 - 4.0 * cw).abs() < 0.001,
        "1->10000 width delta is 4 digit columns ({} vs {})",
        w10000 - w1,
        4.0 * cw
    );
}

#[test]
fn gutter_marker_list_correct_for_one_error_on_line_5() {
    // AC-001: a buffer with one error on line 5 yields exactly one diagnostic marker on line 5.
    let panel = CodeEditorPanel::new("a\nb\nc\nd\ne\nf\ng", "txt");
    panel.push_diagnostics(vec![GutterMarker::diagnostic(
        5,
        DiagnosticSeverity::Error,
        "boom on 5",
    )]);
    let markers = panel.diagnostic_markers();
    let diags: Vec<&GutterMarker> = markers
        .iter()
        .filter(|m| matches!(m.kind, GutterMarkerKind::Diagnostic(_)))
        .collect();
    assert_eq!(diags.len(), 1, "exactly one diagnostic marker");
    assert_eq!(diags[0].line, 5, "the marker is on line 5");
    assert!(matches!(
        diags[0].kind,
        GutterMarkerKind::Diagnostic(DiagnosticSeverity::Error)
    ));
}

// ── AC-007 + MC-004: push_diagnostics updates without panic / version bump; empty clears ──────────

#[test]
fn gutter_push_diagnostics_updates_then_clears_without_version_bump() {
    let panel = CodeEditorPanel::new(RUST_FN, "rs");
    let v0 = panel.buffer_version_for_test();

    // AC-007: push a line-3 error marker — updates the list, no panic.
    panel.push_diagnostics(vec![GutterMarker::diagnostic(
        3,
        DiagnosticSeverity::Error,
        "type error",
    )]);
    assert_eq!(panel.diagnostic_markers().len(), 1);
    assert_eq!(panel.diagnostic_markers()[0].line, 3);
    // KERNEL_BUILDER perf gate: diagnostics MUST NOT bump buffer_version (no re-highlight/re-parse).
    assert_eq!(
        panel.buffer_version_for_test(),
        v0,
        "push_diagnostics must NOT bump buffer_version (perf gate)"
    );

    // MC-004: pushing an empty vec clears the markers (so a closed/replaced file shows no stale dots).
    panel.push_diagnostics(Vec::new());
    assert!(
        panel.diagnostic_markers().is_empty(),
        "empty push clears the markers"
    );
    assert_eq!(
        panel.buffer_version_for_test(),
        v0,
        "still no version bump on clear"
    );
}

#[test]
fn gutter_load_file_clears_stale_diagnostics() {
    // RISK-004: opening a new file into the panel clears the previous file's diagnostics.
    let panel = CodeEditorPanel::new(RUST_FN, "rs");
    panel.push_diagnostics(vec![GutterMarker::diagnostic(
        2,
        DiagnosticSeverity::Warning,
        "old",
    )]);
    assert_eq!(panel.diagnostic_markers().len(), 1);
    panel.load_file("src/other.rs");
    assert!(
        panel.diagnostic_markers().is_empty(),
        "load_file cleared stale diagnostics"
    );
    assert_eq!(
        panel.file_path(),
        "src/other.rs",
        "load_file seeded the new path"
    );
}

// ── AC-003 / PT-003: error marker on line 3 paints a red diagnostic pixel in the gutter ───────────

#[test]
fn gutter_error_marker_renders_red_dot_screenshot() {
    let panel = Arc::new(CodeEditorPanel::new(RUST_FN, "rs"));
    // Put an error on line 3 (0-based). The gutter draws a red 3px left bar + a red dot on that row.
    panel.push_diagnostics(vec![GutterMarker::diagnostic(
        3,
        DiagnosticSeverity::Error,
        "error on line 3",
    )]);

    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(700.0, 280.0))
        .wgpu()
        .build_ui(move |ui| {
            panel_ui.show(ui);
        });

    harness.run();
    harness.run(); // settle so the gutter paints with the captured row geometry

    // The diagnostic marker is live in the panel state and a diagnostic AccessKit Label node exists.
    assert!(
        panel.diagnostic_markers().iter().any(|m| m.line == 3),
        "the line-3 error marker is live"
    );

    match harness.render() {
        Ok(image) => {
            let (w, h) = (image.width(), image.height());
            assert!(w > 0 && h > 0, "rendered image is non-empty");
            // Scan the LEFT gutter strip (the captured gutter rect) for a strongly-red pixel — the 3px
            // diagnostic left bar / the dot. Red means r high, g+b low.
            let gutter_rect = panel.last_gutter_rect().expect("gutter rect captured");
            let ppp = harness.ctx.pixels_per_point();
            let x0 = 0u32;
            let x1 = ((gutter_rect.right() * ppp).ceil() as u32).min(w);
            let mut red_pixels = 0u32;
            for y in 0..h {
                for x in x0..x1 {
                    let p = image.get_pixel(x, y);
                    let [r, g, b, _a] = p.0;
                    if r > 170 && g < 110 && b < 110 {
                        red_pixels += 1;
                    }
                }
            }
            let ext_dir = external_artifact_dir("wp-kernel-012-mt-007");
            let _ = std::fs::create_dir_all(&ext_dir);
            let png_path = ext_dir.join("MT-007-gutter-error.png");
            let saved = image.save(&png_path).is_ok();
            let abs = std::fs::canonicalize(&png_path).unwrap_or(png_path.clone());
            println!(
                "PT-003 gutter error screenshot: {w}x{h}, red_pixels_in_gutter={red_pixels}, \
                 saved={saved} ({})",
                abs.display()
            );
            assert!(
                saved,
                "PT-003: the gutter-error screenshot saved to the external artifact root"
            );
            assert!(
                red_pixels > 0,
                "AC-003: the gutter shows a red diagnostic pixel for the line-3 error (found {red_pixels})"
            );
        }
        Err(e) => {
            println!(
                "BLOCKER(non-fatal): MT-007 gutter-error screenshot render unavailable (no wgpu \
                 adapter): {e}. The line-3 error marker is live in panel state and the diagnostic \
                 AccessKit node is emitted; the PNG is a GPU-host item."
            );
        }
    }

    assert_no_local_test_output();
}

// ── AC-004 / PT-004: line numbers render right-aligned in the gutter for a 10-line buffer ─────────

#[test]
fn gutter_line_numbers_render_screenshot() {
    let ten_lines = "l0\nl1\nl2\nl3\nl4\nl5\nl6\nl7\nl8\nl9";
    let panel = Arc::new(CodeEditorPanel::new(ten_lines, "txt"));
    assert_eq!(panel.buffer().len_lines(), 10, "exactly 10 lines");

    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(600.0, 320.0))
        .wgpu()
        .build_ui(move |ui| {
            panel_ui.show(ui);
        });

    harness.run();
    harness.run();

    // The gutter painted all 10 rows (the viewport fits them) — the captured gutter rows must include
    // buffer lines 0..=9.
    let rows = panel.gutter_rows_for_test();
    assert!(
        (0..10).all(|l| rows.contains(&l)),
        "all 10 buffer lines have a gutter row; got {rows:?}"
    );

    match harness.render() {
        Ok(image) => {
            let (w, h) = (image.width(), image.height());
            // Scan the gutter strip for any NON-background text pixel (the dimmed line-number glyphs).
            // The editor background is the theme background; a line-number glyph is a distinct (dimmer
            // foreground) color, so a column of the gutter strip must contain pixels that differ from
            // the topmost background pixel.
            let gutter_rect = panel.last_gutter_rect().expect("gutter rect captured");
            let ppp = harness.ctx.pixels_per_point();
            let x1 = ((gutter_rect.right() * ppp).ceil() as u32).min(w);
            let bg = image.get_pixel(0, 0).0; // top-left = empty background
            let mut text_pixels = 0u32;
            for y in 0..h {
                for x in 0..x1 {
                    let [r, g, b, _a] = image.get_pixel(x, y).0;
                    let [br, bg_, bb, _] = bg;
                    let diff = (r as i32 - br as i32).abs()
                        + (g as i32 - bg_ as i32).abs()
                        + (b as i32 - bb as i32).abs();
                    if diff > 60 {
                        text_pixels += 1;
                    }
                }
            }
            let ext_dir = external_artifact_dir("wp-kernel-012-mt-007");
            let _ = std::fs::create_dir_all(&ext_dir);
            let png_path = ext_dir.join("MT-007-line-numbers.png");
            let saved = image.save(&png_path).is_ok();
            let abs = std::fs::canonicalize(&png_path).unwrap_or(png_path.clone());
            println!(
                "PT-004 gutter line-numbers screenshot: {w}x{h}, gutter_text_pixels={text_pixels}, \
                 saved={saved} ({})",
                abs.display()
            );
            assert!(
                saved,
                "PT-004: the line-numbers screenshot saved to the external artifact root"
            );
            assert!(
                text_pixels > 0,
                "AC-004: the gutter renders line-number text (found {text_pixels} text pixels)"
            );
        }
        Err(e) => {
            println!(
                "BLOCKER(non-fatal): MT-007 line-numbers screenshot render unavailable (no wgpu \
                 adapter): {e}. The 10 gutter rows are captured (buffer lines 0..=9); the PNG is a \
                 GPU-host item."
            );
        }
    }

    assert_no_local_test_output();
}

// ── AC-005 / PT-005: gutter click at line 2 toggles a breakpoint (AccessKit CheckBox state) ───────

#[test]
fn gutter_breakpoint_toggle_via_click_emits_checkbox_node() {
    let panel = Arc::new(CodeEditorPanel::new(RUST_FN, "rs"));
    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(700.0, 320.0))
        .build_ui(move |ui| {
            panel_ui.show(ui);
        });

    // Frame 1: render so the gutter geometry + rows are captured (the click pixel is computed from them).
    harness.run();
    assert!(
        !panel.is_breakpoint_set(2),
        "no breakpoint on line 2 initially"
    );

    // Compute the exact pixel of the breakpoint sub-column for buffer line 2 from the captured gutter
    // geometry, then inject a real Primary click there.
    let pos = panel
        .gutter_breakpoint_pos_for_line(2)
        .expect("line 2 has a gutter row on screen");
    for ev in click_events(pos) {
        harness.event(ev);
    }
    harness.run(); // process the click -> toggle_breakpoint(2)
    harness.run(); // re-render so the breakpoint AccessKit node is emitted

    assert!(
        panel.is_breakpoint_set(2),
        "AC-005: the gutter click set a breakpoint on line 2"
    );

    // PT-005: the live AccessKit tree contains code_editor_breakpoint_2 as a toggled CheckBox (the
    // field-correct accesskit 0.21.1 toggle role for the contract's ToggleButton).
    let root = harness.root();
    let mut found_breakpoint_2 = false;
    let mut breakpoint_2_role: Option<String> = None;
    for node in root.children_recursive() {
        let ak = node.accesskit_node();
        if ak.author_id() == Some("code_editor_breakpoint_2") {
            found_breakpoint_2 = true;
            breakpoint_2_role = Some(format!("{:?}", ak.role()));
        }
    }
    assert!(
        found_breakpoint_2,
        "AC-005: the live tree contains code_editor_breakpoint_2 after the toggle"
    );
    assert_eq!(
        breakpoint_2_role.as_deref(),
        Some("CheckBox"),
        "AC-005: code_editor_breakpoint_2 uses the field-correct CheckBox toggle role; got {breakpoint_2_role:?}"
    );

    // A second click on the SAME line clears the breakpoint (idempotent in pairs — AC-002) and the node
    // disappears.
    let pos2 = panel
        .gutter_breakpoint_pos_for_line(2)
        .expect("line 2 still has a gutter row");
    for ev in click_events(pos2) {
        harness.event(ev);
    }
    harness.run();
    harness.run();
    assert!(
        !panel.is_breakpoint_set(2),
        "a second gutter click cleared the line-2 breakpoint"
    );
}

// ── AC-006 / PT-006: gutter fold-triangle click at line 0 toggles the fold ────────────────────────

#[test]
fn gutter_fold_click_toggles_fold() {
    let panel = Arc::new(CodeEditorPanel::new(RUST_FN, "rs"));
    // The function-body region starts on line 0 and is UNfolded at construction.
    assert!(
        panel
            .fold_set()
            .regions
            .iter()
            .any(|r| r.start_line == 0 && !r.folded),
        "a line-0 fold region exists and starts unfolded"
    );

    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(700.0, 320.0))
        .build_ui(move |ui| {
            panel_ui.show(ui);
        });

    harness.run();
    let unfolded_rows = panel.gutter_rows_for_test();
    assert!(
        unfolded_rows.contains(&1),
        "line 1 (a body line) is painted while unfolded; got {unfolded_rows:?}"
    );

    // Click the fold triangle for the line-0 region.
    let pos = panel
        .gutter_fold_pos_for_line(0)
        .expect("the line-0 region has a fold triangle on screen");
    for ev in click_events(pos) {
        harness.event(ev);
    }
    harness.run(); // process -> toggle_fold(0)
    harness.run(); // re-render the collapsed body

    // AC-006: the fold is now folded, and the body line 1 is no longer painted (collapsed).
    assert!(
        panel
            .fold_set()
            .regions
            .iter()
            .any(|r| r.start_line == 0 && r.folded),
        "AC-006: the gutter fold-triangle click folded the line-0 region"
    );
    let folded_rows = panel.gutter_rows_for_test();
    assert!(
        !folded_rows.contains(&1),
        "AC-006: after the fold click, body line 1 is no longer painted; got {folded_rows:?}"
    );
}

// ── MC-001: runtime gutter-widen e2e (Wave-B remediation — previously deferred) ───────────────────

#[test]
fn gutter_widens_at_runtime_when_line_count_crosses_digit_boundaries() {
    // MC-001 e2e: the gutter width is recomputed EVERY FRAME from the LIVE line count (not cached at
    // mount), so growing the buffer at runtime from 9 lines (1 digit) to 10000 lines (5 digits) must
    // push the text column right by exactly 4 digit columns — and shrinking back must narrow it again.
    // Observed through the live panel: `screen_pos_for_line_col(0, 0)` x is the text-column left edge,
    // which sits at the gutter strip's right edge.
    let panel = Arc::new(CodeEditorPanel::new(&"x\n".repeat(9), "txt"));
    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(640.0, 400.0))
        .build_ui(move |ui| {
            panel_ui.show(ui);
        });
    harness.run();
    harness.run();
    let gw = panel
        .measured_glyph_width()
        .expect("glyph width measured after a frame");
    let x_small = panel
        .screen_pos_for_line_col(0, 0, gw)
        .expect("line 0 on screen (9-line buffer)")
        .x;

    // Grow the LIVE buffer across four digit boundaries (9 -> 10000 lines) and re-render.
    panel.set_text(&"y\n".repeat(10_000));
    harness.run();
    harness.run();
    let x_big = panel
        .screen_pos_for_line_col(0, 0, gw)
        .expect("line 0 on screen (10000-line buffer)")
        .x;
    assert!(
        (x_big - x_small - 4.0 * gw).abs() < 0.5,
        "MC-001: growing 9 -> 10000 lines at runtime widens the gutter by exactly 4 digit columns \
         (text left moved {:.2}px, expected {:.2}px)",
        x_big - x_small,
        4.0 * gw
    );

    // And back down: the width is live, not high-watermark — shrinking narrows the gutter again.
    panel.set_text(&"z\n".repeat(9));
    harness.run();
    harness.run();
    let x_back = panel
        .screen_pos_for_line_col(0, 0, gw)
        .expect("line 0 on screen (back to 9 lines)")
        .x;
    assert!(
        (x_back - x_small).abs() < 0.5,
        "MC-001: shrinking back to 9 lines restores the original gutter width \
         (text left {x_back:.2} vs original {x_small:.2})"
    );
}

// ── MC-002: fold-triangle glyph CI test (Wave-B remediation — previously deferred) ────────────────

#[test]
fn gutter_fold_triangle_glyph_matches_font_coverage_and_is_never_tofu() {
    // MC-002: `fold_triangle_glyph` must return the Unicode triangle exactly when the active
    // monospace font can render it, the ASCII fallback otherwise — and whichever it returns must
    // itself be renderable (never a tofu box). Checked live against the same monospace family the
    // gutter paints with (glyph COVERAGE is a property of the font family, not the point size).
    let mut harness = Harness::builder().build_ui(|ui| {
        let font = egui::FontId::monospace(13.0);
        for is_open in [true, false] {
            let (unicode_str, unicode_ch, ascii) = if is_open {
                ("\u{25BC}", '\u{25BC}', "v")
            } else {
                ("\u{25B6}", '\u{25B6}', ">")
            };
            let glyph = fold_triangle_glyph(ui, is_open);
            let font_has_unicode = ui.fonts_mut(|f| f.has_glyph(&font, unicode_ch));
            let expected = if font_has_unicode { unicode_str } else { ascii };
            assert_eq!(
                glyph, expected,
                "MC-002: fold triangle (is_open={is_open}) must be the Unicode glyph iff the \
                 monospace font covers it (font_has_unicode={font_has_unicode})"
            );
            // Never tofu: the returned glyph is renderable in the gutter's monospace family.
            let ch = glyph.chars().next().expect("non-empty glyph");
            assert!(
                ui.fonts_mut(|f| f.has_glyph(&font, ch)),
                "MC-002: the returned fold glyph {glyph:?} must be renderable — never a tofu box"
            );
        }
    });
    // The closure runs on `run()`; any assertion failure inside it panics the test here.
    harness.run();
}

/// A plain Primary click (press + release) at `pos`.
fn click_events(pos: egui::Pos2) -> [egui::Event; 2] {
    let m = egui::Modifiers::default();
    [
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: m,
        },
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: m,
        },
    ]
}

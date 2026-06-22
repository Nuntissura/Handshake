//! Minimap: a scaled-down whole-file overview with a viewport indicator (WP-KERNEL-012 MT-006, E1 —
//! VS Code parity).
//!
//! The minimap (Monaco's right-edge "minimap") paints one pixel row per file line — or one row per N
//! lines when the file is taller than the panel — each colored by the dominant highlight scope of that
//! line, with a translucent rectangle marking the lines currently in the editor viewport. Clicking a
//! row scrolls the editor to that line; hovering shows the line number.
//!
//! ## Theme-driven colors (no hardcoded hex — MT step 1)
//!
//! Row colors come from [`scope_to_color`](super::panel::scope_to_color), which maps each
//! [`HighlightScope`] to a token of the active theme's `HsSyntaxTokens` — never a literal `Color32`.
//! An unhighlighted line uses the theme background darkened 10% (a derived shade of a theme color, not
//! a new hardcoded token). The viewport-indicator overlay is the one explicit RGBA the contract names
//! (`from_rgba_unmultiplied(255,255,255,30)`), exactly like the find-match / fold-summary UI tints the
//! sibling MTs use — it is a UI affordance, not a syntax token.
//!
//! ## Draw-call budget (MT impl note 1)
//!
//! For a 10k-line file the minimap paints at most `panel_height_px` rows (≈ a few hundred), not 10k:
//! the compression ratio `max(1, ceil(total_lines / panel_height_px))` groups lines into one painted
//! row, and the rows are batched into a single [`egui::Shape::Vec`] so the whole minimap is ONE draw
//! call regardless of row count (RISK-004 analog of the find-match batch).
//!
//! ## Fold-awareness (MT positioning note)
//!
//! The minimap maps a click to a BUFFER line; the panel is responsible for routing that buffer line
//! through the fold-aware visible<->buffer mapping when it scrolls (MT-002 corrected `y_for_line` /
//! `scroll_to_line` + MT-005 `visible_line_to_buffer_line`). The minimap itself works in buffer-line
//! space (the whole file) so the overview shows every line, folded or not; the `visible_range` it
//! highlights is passed in by the panel already expressed in buffer lines.

use std::ops::Range;

use super::buffer::TextBuffer;
use super::highlight::{HighlightScope, HighlightSpan};
use super::panel::scope_to_color;

/// Default minimap panel width in pixels (Monaco's minimap is ~80px). Configurable via
/// [`Minimap::with_width`].
pub const DEFAULT_MINIMAP_WIDTH: f32 = 80.0;

/// The translucent overlay marking the lines currently in the editor viewport (MT impl note). A UI
/// affordance (like the find-match tint), NOT a syntax token — the one explicit RGBA the contract
/// names for the minimap. The contract names `from_rgba_unmultiplied(255,255,255,30)`; that ctor is
/// non-const, so the equivalent PREmultiplied form is used in the const (white at alpha 30 unmultiplied
/// == 30/30/30/30 premultiplied, since premul = round(channel * a / 255) = round(255*30/255) = 30). The
/// `from_rgba_premultiplied` ctor IS const, exactly as the MT-004 find-match tint uses it.
const VIEWPORT_INDICATOR_COLOR: egui::Color32 = egui::Color32::from_rgba_premultiplied(30, 30, 30, 30);

/// A minimap widget. Stateless apart from its configured width; the panel constructs one per frame (or
/// reuses one) and calls [`render`](Minimap::render). The render result reports a click (the buffer
/// line to scroll to) so the panel drives the scroll through its fold-aware mapping.
#[derive(Debug, Clone, Copy)]
pub struct Minimap {
    width: f32,
}

/// The outcome of one [`Minimap::render`] call: whether the user clicked a row (and which BUFFER line
/// it maps to), plus the screen rect the minimap content actually painted into. The panel consumes
/// `clicked_buffer_line` to scroll the editor and `content_rect` for diagnostics + the AC-006
/// midpoint-click geometry (the true minimap content rect, which is exactly the configured width — the
/// enclosing `SidePanel` adds frame margins around it).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MinimapResponse {
    /// `Some(line)` when the user clicked (or dragged) on the minimap this frame — the BUFFER line the
    /// click maps to. The panel scrolls the editor to this line through its fold-aware mapping. `None`
    /// when there was no click on the minimap this frame.
    pub clicked_buffer_line: Option<usize>,
    /// The screen rect the minimap content (the colored rows + indicator) painted into this frame —
    /// exactly `width × available_height`. This is the AC-003/AC-006 geometry surface (the configured
    /// 80px width, not the SidePanel's margin-inflated outer rect).
    pub content_rect: egui::Rect,
}

impl Default for MinimapResponse {
    fn default() -> Self {
        Self { clicked_buffer_line: None, content_rect: egui::Rect::NOTHING }
    }
}

impl Default for Minimap {
    fn default() -> Self {
        Self { width: DEFAULT_MINIMAP_WIDTH }
    }
}

impl Minimap {
    /// A minimap with the default 80px width.
    pub fn new() -> Self {
        Self::default()
    }

    /// A minimap with a custom width (clamped to a sane minimum so it never collapses to nothing).
    pub fn with_width(width: f32) -> Self {
        Self { width: width.max(8.0) }
    }

    /// The configured width in pixels.
    pub fn width(&self) -> f32 {
        self.width
    }

    /// The compression ratio: how many file lines map to ONE painted minimap row. `1` when every line
    /// fits in `panel_height_px` rows; `ceil(total_lines / panel_height_px)` (>= 1) when the file is
    /// taller than the panel, so a 10k-line file in a 300px panel paints ~300 rows, not 10k (MT impl
    /// note 1 / MT step 1). Pure + total: a zero panel height or zero line count yields `1`.
    pub fn compression_ratio(total_lines: usize, panel_height_px: f32) -> usize {
        let rows = panel_height_px.floor().max(0.0) as usize;
        if rows == 0 || total_lines == 0 {
            return 1;
        }
        // ceil(total_lines / rows), never below 1.
        total_lines.div_ceil(rows).max(1)
    }

    /// The minimap row index a BUFFER line maps to under `ratio` (`line / ratio`), and the inverse —
    /// the FIRST buffer line a row covers (`row * ratio`). Exposed for the click-to-scroll math + tests.
    pub fn row_for_line(line: usize, ratio: usize) -> usize {
        line / ratio.max(1)
    }

    /// The first BUFFER line a minimap `row` covers under `ratio` (`row * ratio`). The click-to-scroll
    /// target. Clamped by the caller to the live line count.
    pub fn line_for_row(row: usize, ratio: usize) -> usize {
        row.saturating_mul(ratio.max(1))
    }

    /// Compute the per-minimap-row colors (the dominant highlight scope of the lines each row covers,
    /// or the neutral background-derived shade for an unhighlighted row). This is the ONLY O(spans) pass
    /// the minimap does; the panel CACHES the result keyed by `(buffer_version, painted_rows)` so the
    /// per-frame [`render`](Self::render) is O(painted_rows), not O(spans) — critical on a 100k-line file
    /// where re-walking every span each frame would blow the frame budget (MT-002 perf gate).
    ///
    /// `painted_rows` is the number of minimap rows (one per `ratio` lines); `ratio` is the compression
    /// ratio. `dark_mode` selects the theme so the colors track the shell theme. The spans must be sorted
    /// by start byte (the highlighter guarantees this).
    pub fn compute_row_colors(
        buffer: &TextBuffer,
        highlight_spans: &[HighlightSpan],
        painted_rows: usize,
        ratio: usize,
        dark_mode: bool,
    ) -> Vec<egui::Color32> {
        let syntax = if dark_mode {
            crate::theme::HsTheme::Dark.palette().syntax
        } else {
            crate::theme::HsTheme::Light.palette().syntax
        };
        let neutral = darken(syntax.background, 0.10);
        let mut row_colors = vec![neutral; painted_rows.max(1)];
        let mut row_scope: Vec<Option<HighlightScope>> = vec![None; painted_rows.max(1)];
        for span in highlight_spans {
            if span.scope == HighlightScope::Other {
                continue; // Other folds to neutral; do not let it override a real scope.
            }
            if let Some(line) = buffer.byte_to_line(span.byte_range.start) {
                let row = Self::row_for_line(line, ratio);
                if row < row_colors.len() && row_scope[row].is_none() {
                    // First non-Other scope on a row wins (deterministic; keyword/string/etc dominate).
                    row_scope[row] = Some(span.scope);
                    row_colors[row] = scope_to_color(span.scope, &syntax);
                }
            }
        }
        row_colors
    }

    /// Render the minimap into `ui` from PRECOMPUTED `row_colors` (built by
    /// [`compute_row_colors`](Self::compute_row_colors) and cached by the panel) and return the click
    /// outcome.
    ///
    /// - `row_colors`: one color per minimap row (the cached dominant-scope colors). If its length does
    ///   not match the freshly-computed `painted_rows` (e.g. the panel resized this frame) the row layout
    ///   is recomputed against `row_colors.len()` so the paint never indexes out of range; the panel
    ///   refreshes the cache on the next frame.
    /// - `visible_range`: the BUFFER-line range currently in the editor viewport, painted as the
    ///   translucent indicator rect.
    /// - `total_lines`: the document line count (the whole file the minimap scales down).
    ///
    /// Returns a [`MinimapResponse`] whose `clicked_buffer_line` is `Some(line)` when the user clicked
    /// on the minimap. The widget allocates a `width × available_height` region, paints the rows +
    /// indicator in one batched shape (MT impl note 1: O(painted_rows) shapes in ONE draw call), and adds
    /// a hover tooltip with the line number under the pointer.
    pub fn render(
        &self,
        ui: &mut egui::Ui,
        row_colors: &[egui::Color32],
        visible_range: Range<usize>,
        total_lines: usize,
    ) -> MinimapResponse {
        // Allocate the full-height minimap column. `sense(click_and_drag)` so a press anywhere on it is
        // a scroll request (Monaco scrolls on minimap click/drag).
        let desired = egui::vec2(self.width, ui.available_height().max(1.0));
        let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click_and_drag());

        if !ui.is_rect_visible(rect) || total_lines == 0 {
            return MinimapResponse { clicked_buffer_line: None, content_rect: rect };
        }

        let panel_height = rect.height();
        let ratio = Self::compression_ratio(total_lines, panel_height);
        // The number of painted rows the layout wants this frame. The cached `row_colors` may be a
        // different length if the panel just resized; paint against the cache length (it refreshes next
        // frame) so the click math + the paint never index out of range.
        let painted_rows = row_colors.len().max(1);
        let row_height = (panel_height / painted_rows as f32).max(1.0);

        // Batch every row rect into ONE shape (MT impl note 1: a single draw call, not `painted_rows`).
        let mut shapes: Vec<egui::Shape> = Vec::with_capacity(painted_rows + 1);
        for (row, color) in row_colors.iter().enumerate() {
            let y0 = rect.top() + row as f32 * row_height;
            let row_rect = egui::Rect::from_min_size(
                egui::pos2(rect.left(), y0),
                egui::vec2(self.width, row_height),
            );
            shapes.push(egui::Shape::rect_filled(row_rect, 0.0, *color));
        }

        // The viewport-indicator rect over the rows the editor viewport currently shows (MT impl note).
        let vis_start = visible_range.start.min(total_lines);
        let vis_end = visible_range.end.min(total_lines).max(vis_start);
        if vis_end > vis_start {
            let r0 = Self::row_for_line(vis_start, ratio);
            let r1 = Self::row_for_line(vis_end.saturating_sub(1), ratio) + 1;
            let y0 = rect.top() + r0 as f32 * row_height;
            let y1 = (rect.top() + r1 as f32 * row_height).min(rect.bottom());
            let indicator = egui::Rect::from_min_max(
                egui::pos2(rect.left(), y0),
                egui::pos2(rect.right(), y1.max(y0 + 1.0)),
            );
            shapes.push(egui::Shape::rect_filled(indicator, 0.0, VIEWPORT_INDICATOR_COLOR));
        }
        ui.painter().add(egui::Shape::Vec(shapes));

        // Hover tooltip: the line number under the pointer (MT step 1). `on_hover_text` is idempotent
        // (RISK-004 — it does not accumulate widgets per frame).
        let mut clicked_buffer_line = None;
        if let Some(pointer) = response.hover_pos().or_else(|| {
            // On a click/drag, use the interaction position even if `hover_pos` is None this frame.
            if response.clicked() || response.dragged() {
                response.interact_pointer_pos()
            } else {
                None
            }
        }) {
            let rel_y = (pointer.y - rect.top()).clamp(0.0, panel_height);
            let row = (rel_y / row_height).floor() as usize;
            let line = Self::line_for_row(row, ratio).min(total_lines.saturating_sub(1));
            // 1-based line number for the human-facing tooltip.
            response.clone().on_hover_text(format!("Line {}", line + 1));

            // A click or drag on the minimap is a scroll-to request (Monaco behavior).
            if response.clicked() || response.dragged() {
                clicked_buffer_line = Some(line);
            }
        }

        MinimapResponse { clicked_buffer_line, content_rect: rect }
    }
}

/// Darken a color toward black by `factor` (0.0 = unchanged, 1.0 = black). Used for the neutral
/// (unhighlighted) minimap row: a DERIVED shade of the theme background, not a hardcoded literal, so
/// the no-hardcode invariant holds (the only literal is the alpha math, not a color).
fn darken(c: egui::Color32, factor: f32) -> egui::Color32 {
    let f = (1.0 - factor.clamp(0.0, 1.0)).clamp(0.0, 1.0);
    egui::Color32::from_rgba_unmultiplied(
        (c.r() as f32 * f) as u8,
        (c.g() as f32 * f) as u8,
        (c.b() as f32 * f) as u8,
        c.a(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compression_ratio_is_one_when_file_fits() {
        // 50 lines in a 300px panel -> every line fits in its own row -> ratio 1.
        assert_eq!(Minimap::compression_ratio(50, 300.0), 1);
        // Exactly fitting (300 lines, 300px) -> ratio 1.
        assert_eq!(Minimap::compression_ratio(300, 300.0), 1);
    }

    #[test]
    fn compression_ratio_compresses_large_files() {
        // 10_000 lines in an 80px panel -> ceil(10000/80) = 125 lines per row (MT impl note 1).
        assert_eq!(Minimap::compression_ratio(10_000, 80.0), 125);
        // 10_000 lines in a 300px panel -> ceil(10000/300) = 34.
        assert_eq!(Minimap::compression_ratio(10_000, 300.0), 34);
    }

    #[test]
    fn compression_ratio_degenerate_inputs_yield_one() {
        assert_eq!(Minimap::compression_ratio(0, 300.0), 1, "no lines -> ratio 1");
        assert_eq!(Minimap::compression_ratio(100, 0.0), 1, "zero panel height -> ratio 1");
        assert_eq!(Minimap::compression_ratio(100, -5.0), 1, "negative panel height -> ratio 1");
    }

    #[test]
    fn row_line_round_trip_under_ratio() {
        // ratio 1: identity.
        assert_eq!(Minimap::row_for_line(42, 1), 42);
        assert_eq!(Minimap::line_for_row(42, 1), 42);
        // ratio 10: line 95 -> row 9 -> first line of row 9 is 90.
        assert_eq!(Minimap::row_for_line(95, 10), 9);
        assert_eq!(Minimap::line_for_row(9, 10), 90);
        // The midpoint mapping AC-006 relies on: row at the vertical middle maps near the middle line.
        let total = 1000usize;
        let ratio = Minimap::compression_ratio(total, 200.0); // ceil(1000/200)=5
        let mid_row = (200.0f32 * 0.5 / (200.0 / (total.div_ceil(ratio) as f32))) as usize;
        let mid_line = Minimap::line_for_row(mid_row, ratio);
        assert!(
            (mid_line as i64 - 500).abs() <= 3 * ratio as i64,
            "minimap vertical midpoint maps near the middle line (got {mid_line} for 1000 lines)"
        );
    }

    #[test]
    fn line_for_row_saturates_not_overflows() {
        // A huge row * ratio must saturate, never overflow/panic.
        assert_eq!(Minimap::line_for_row(usize::MAX, 4), usize::MAX);
    }

    #[test]
    fn darken_reduces_channels_keeps_alpha() {
        let c = egui::Color32::from_rgba_unmultiplied(200, 100, 50, 255);
        let d = darken(c, 0.10);
        assert!(d.r() < c.r() && d.g() < c.g() && d.b() < c.b(), "channels darkened");
        assert_eq!(d.a(), 255, "alpha preserved");
        // factor 0 -> unchanged.
        assert_eq!(darken(c, 0.0), c);
    }
}

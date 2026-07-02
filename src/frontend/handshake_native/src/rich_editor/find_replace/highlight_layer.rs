//! Match-highlight painting over the rendered content (WP-KERNEL-012 MT-018).
//!
//! After `block_renderer` paints a block's content, this layer paints a semi-transparent colored
//! rect over each [`scanner::FindMatch`] that falls inside that block. The rect is resolved via the
//! SAME mechanism MT-015's wikilink-chip painting uses: re-lay the block to its epaint
//! [`egui::Galley`] (the MT-012 `LayoutJob` engine — NOT cosmic-text, per the KERNEL_BUILDER gate),
//! hit-test the match's char span with `Galley::pos_from_cursor`, and offset by the block's painted
//! (already scroll-adjusted) screen origin (RISK / MC: the single paint origin carries the scroll
//! adjustment — [`chip_rect_for_span`] reuse).
//!
//! ## Per-leaf char range -> block-galley cursor index
//!
//! A [`scanner::FindMatch`] carries a `[char_start, char_end)` range RELATIVE TO ITS TEXT LEAF (the
//! address Replace uses). The block galley's `CCursor` index, however, is into the block's WHOLE
//! concatenated inline plain text (every run + each `hsLink`/transclusion atom's display label, in
//! `line_layout::layout_block` order). [`block_galley_cursor`] converts a `(leaf_index_in_block,
//! leaf_char_offset)` into the block-galley cursor by summing the char lengths of every inline child
//! before that leaf. This is the single load-bearing seam that keeps the highlight aligned with the
//! glyphs even when a paragraph mixes styled runs and inline atoms.
//!
//! ## Opacity (CONTROL-4 theme tokens — no hardcoded hex)
//!
//! The current match paints the theme `accent` at ~60% opacity; other matches at ~25%. The base
//! color is always a theme token; only the alpha is adjusted, so a re-themed editor re-tints the
//! highlights automatically.

use egui::epaint::text::cursor::CCursor;
use egui::{Color32, Pos2, Rect};

use crate::rich_editor::document_model::node::{BlockNode, Child};
use crate::rich_editor::renderer::line_layout;
use crate::rich_editor::wikilinks::inline_view::chip_rect_for_span;
use crate::theme::HsPalette;

use super::scanner::FindMatch;

/// The alpha (0..=255) applied to the theme accent for the CURRENT match highlight (~60%).
pub const CURRENT_MATCH_ALPHA: u8 = 153; // 0.60 * 255
/// The alpha (0..=255) applied to the theme accent for the OTHER (non-current) match highlights (~25%).
pub const OTHER_MATCH_ALPHA: u8 = 64; // 0.25 * 255

/// The fill color for a match highlight: the theme `accent` at the current/other opacity. The accent
/// is a premultiplied `Color32`; we rebuild it at the target alpha from its straight RGB so the
/// blend reads as a translucent wash over the glyphs (CONTROL-4: the base color is a theme token).
pub fn highlight_fill(palette: &HsPalette, is_current: bool) -> Color32 {
    let alpha = if is_current {
        CURRENT_MATCH_ALPHA
    } else {
        OTHER_MATCH_ALPHA
    };
    let [r, g, b, _] = palette.accent.to_array();
    Color32::from_rgba_unmultiplied(r, g, b, alpha)
}

/// Convert a `FindMatch`'s per-leaf char range into the block galley's `[CCursor, CCursor)` index
/// pair. `block` is the block the match's leaf lives in; `leaf_index` is that leaf's index within
/// the block's `children`. Returns `None` when the leaf index is not a text leaf of the block (a
/// stale match against a since-restructured block) so a bad span never panics the paint loop.
///
/// The block-galley cursor index of a leaf-local char offset is `sum(char lengths of every inline
/// child before the leaf) + leaf_offset`, matching `line_layout::layout_block`'s append order
/// (text runs contribute their text; an `hsLink`/transclusion atom contributes its display label).
pub fn block_galley_cursor(
    block: &BlockNode,
    leaf_index: usize,
    leaf_offset: usize,
) -> Option<usize> {
    // The leaf must be a text child of the block.
    if !matches!(block.children.get(leaf_index), Some(Child::Text(_))) {
        return None;
    }
    let mut base = 0usize;
    for child in block.children.iter().take(leaf_index) {
        base += inline_child_galley_len(child);
    }
    Some(base + leaf_offset)
}

/// The number of galley chars an inline child contributes to `line_layout::layout_block`'s plain
/// text. A text run contributes its char count; an `hsLink` atom contributes its display label
/// (`label` or `refKind:refValue`); a transclusion contributes its `⟢ {ref}` label; a (schema-illegal
/// here) block child contributes nothing. MUST mirror `line_layout::layout_block` exactly so the
/// galley cursor index lines up with the laid-out glyphs.
fn inline_child_galley_len(child: &Child) -> usize {
    match child {
        Child::Text(t) => t.text.len_chars(),
        Child::HsLink(l) => {
            let label = if l.label.is_empty() {
                format!("{}:{}", l.ref_kind, l.ref_value)
            } else {
                l.label.clone()
            };
            label.chars().count()
        }
        Child::Transclusion(t) => format!("⟢ {}", t.ref_value).chars().count(),
        Child::Block(_) => 0,
    }
}

/// One highlight rect to paint, in SCREEN space, with its fill color. Computed by
/// [`match_highlight_rects`] so the doc borrow ends before the painter draws (the caller paints
/// these after the block content).
#[derive(Debug, Clone, PartialEq)]
pub struct HighlightRect {
    /// The screen-space rect to fill.
    pub rect: Rect,
    /// The semi-transparent fill color (current vs. other opacity).
    pub fill: Color32,
}

/// Compute the [`HighlightRect`]s for every match in `matches` that falls inside `block`, given the
/// block's painted screen `origin` (already scroll-adjusted), the content `wrap_width`, and whether
/// the bold family is bound. `block_index_filter` is the document-order index of `block`; only
/// matches whose `node_path` first element equals it (i.e. live in this top-level block) are painted
/// — so each block paints only its own matches. `current_match` is the active match (painted at the
/// higher opacity), or `None`.
///
/// Resolves each match's leaf-local char range to a block-galley cursor span, hit-tests the galley
/// with `pos_from_cursor`, and offsets by `origin` via [`chip_rect_for_span`] (the same scroll-safe
/// rect helper the wikilink chips use).
#[allow(clippy::too_many_arguments)]
pub fn match_highlight_rects(
    block: &BlockNode,
    block_index: usize,
    matches: &[FindMatch],
    current_match: Option<&FindMatch>,
    origin: Pos2,
    wrap_width: f32,
    bold_available: bool,
    palette: &HsPalette,
    painter: &egui::Painter,
) -> Vec<HighlightRect> {
    let layout = line_layout::layout_block(block, palette, wrap_width.max(1.0), bold_available);
    let galley = painter.layout_job(layout.job);
    let max_cursor = galley.job.text.chars().count();

    let mut rects = Vec::new();
    for m in matches {
        // Only paint matches that live in THIS top-level block (the renderer paints one block at a
        // time). The match's node_path first element is its top-level block index.
        if m.node_path.first().copied() != Some(block_index) {
            continue;
        }
        let Some((&leaf_index, _)) = m.node_path.split_last() else {
            continue;
        };
        let Some(start) = block_galley_cursor(block, leaf_index, m.char_start) else {
            continue;
        };
        let Some(end) = block_galley_cursor(block, leaf_index, m.char_end) else {
            continue;
        };
        // Clamp the galley cursor span (a stale match against a shorter galley never indexes past it).
        let start = start.min(max_cursor);
        let end = end.min(max_cursor).max(start);
        let local_start = galley.pos_from_cursor(CCursor::new(start));
        let local_end = galley.pos_from_cursor(CCursor::new(end));
        let rect = chip_rect_for_span(local_start, local_end, origin);
        let is_current = current_match.is_some_and(|c| std::ptr::eq(c, m) || c == m);
        rects.push(HighlightRect {
            rect,
            fill: highlight_fill(palette, is_current),
        });
    }
    rects
}

/// Paint the highlight rects (semi-transparent filled rects) onto `painter`, AFTER the block content
/// (so the wash sits over the glyphs). A small corner radius reads as a soft highlight pill.
pub fn paint_highlights(painter: &egui::Painter, rects: &[HighlightRect]) {
    for hr in rects {
        painter.rect_filled(hr.rect, 2.0, hr.fill);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rich_editor::document_model::node::{
        BlockNode, Child, HsLinkNode, Mark, NodeKind, TextLeaf,
    };
    use crate::rich_editor::find_replace::scanner::{FindMatch, MatchKind};
    use crate::theme::HsTheme;

    fn dark() -> HsPalette {
        HsTheme::Dark.palette()
    }

    #[test]
    fn highlight_fill_uses_theme_accent_with_opacity() {
        let pal = dark();
        let current = highlight_fill(&pal, true);
        let other = highlight_fill(&pal, false);
        let [r, g, b, _] = pal.accent.to_array();
        // Both reuse the accent RGB (no hardcoded hex); only the alpha differs.
        assert_eq!(
            current,
            Color32::from_rgba_unmultiplied(r, g, b, CURRENT_MATCH_ALPHA)
        );
        assert_eq!(
            other,
            Color32::from_rgba_unmultiplied(r, g, b, OTHER_MATCH_ALPHA)
        );
    }

    // The current-match opacity must exceed the other-match opacity (a const invariant over the two
    // alpha constants, so it is enforced at compile time with `const _` — clippy flags a runtime
    // `assert!` on constants as optimized-out; same pattern as renderer/mod.rs).
    const _: () = assert!(
        CURRENT_MATCH_ALPHA > OTHER_MATCH_ALPHA,
        "the current match must be more opaque than the others"
    );

    #[test]
    fn block_galley_cursor_sums_runs_before_the_leaf() {
        // "Hello " (run 0) + bold "world" (run 1). A match at leaf-index 1, offset 2 maps to the
        // block-galley cursor 6 ("Hello ".len) + 2 == 8.
        let block = BlockNode::with_children(
            NodeKind::Paragraph,
            vec![
                Child::Text(TextLeaf::new("Hello ")),
                Child::Text(TextLeaf::with_marks("world", vec![Mark::Bold])),
            ],
        );
        assert_eq!(block_galley_cursor(&block, 0, 0), Some(0));
        assert_eq!(block_galley_cursor(&block, 0, 3), Some(3));
        assert_eq!(
            block_galley_cursor(&block, 1, 0),
            Some(6),
            "leaf 1 starts after 'Hello '"
        );
        assert_eq!(block_galley_cursor(&block, 1, 2), Some(8));
    }

    #[test]
    fn block_galley_cursor_counts_inline_atom_labels() {
        // A paragraph: text "see " + hsLink (label "Doc") + text " end". A match inside " end"
        // (leaf index 2) must account for "see "(4) + "Doc"(3) == 7 before it.
        let block = BlockNode::with_children(
            NodeKind::Paragraph,
            vec![
                Child::Text(TextLeaf::new("see ")),
                Child::HsLink(HsLinkNode::new("note", "abc", "Doc")),
                Child::Text(TextLeaf::new(" end")),
            ],
        );
        assert_eq!(
            block_galley_cursor(&block, 2, 0),
            Some(7),
            "after 'see ' + chip label 'Doc'"
        );
        assert_eq!(block_galley_cursor(&block, 2, 1), Some(8));
        // A non-text leaf index returns None (a stale span never panics).
        assert_eq!(block_galley_cursor(&block, 1, 0), None);
    }

    #[test]
    fn match_rects_only_for_this_block_and_offset_by_origin() {
        // Build a galley-backed painter (headless) and assert a match in block 0 yields one rect
        // offset by the paint origin, while a match tagged for block 1 is skipped when painting
        // block 0.
        let ctx = egui::Context::default();
        crate::app::HandshakeApp::install_fonts(&ctx);
        let pal = dark();
        let mut out = None;
        let _ = ctx.run(Default::default(), |ctx| {
            let bold = line_layout::bold_family_available(ctx);
            let painter = ctx.layer_painter(egui::LayerId::background());
            let block = BlockNode::paragraph("foo bar foo");
            let matches = vec![
                FindMatch {
                    kind: MatchKind::Prose,
                    node_path: vec![0, 0],
                    char_start: 0,
                    char_end: 3,
                },
                FindMatch {
                    kind: MatchKind::Prose,
                    node_path: vec![0, 0],
                    char_start: 8,
                    char_end: 11,
                },
                // A match that belongs to a DIFFERENT top-level block (index 1) — must be skipped.
                FindMatch {
                    kind: MatchKind::Prose,
                    node_path: vec![1, 0],
                    char_start: 0,
                    char_end: 3,
                },
            ];
            let origin = egui::pos2(40.0, 200.0); // scrolled paint origin.
            let rects = match_highlight_rects(
                &block,
                0,
                &matches,
                Some(&matches[0]),
                origin,
                400.0,
                bold,
                &pal,
                &painter,
            );
            out = Some(rects);
        });
        let rects = out.unwrap();
        assert_eq!(
            rects.len(),
            2,
            "only the two block-0 matches paint (the block-1 match is skipped)"
        );
        // The first match is the current one (higher opacity); the second is dimmer.
        assert_eq!(rects[0].fill.a(), CURRENT_MATCH_ALPHA);
        assert_eq!(rects[1].fill.a(), OTHER_MATCH_ALPHA);
        // Both rects start at-or-below the scroll-adjusted origin Y.
        for hr in &rects {
            assert!(
                hr.rect.min.y >= 200.0 - 0.01,
                "the rect Y follows the scroll-adjusted origin"
            );
            assert!(hr.rect.width() > 0.0, "a 3-char match has positive width");
        }
    }
}

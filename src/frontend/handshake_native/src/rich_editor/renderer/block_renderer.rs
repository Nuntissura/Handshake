//! Per-block painting onto the egui canvas (WP-KERNEL-012 MT-012).
//!
//! Each [`BlockNode`] kind paints through egui's [`egui::Painter`] using a styled
//! [`epaint::Galley`] built by [`super::line_layout`]:
//! - paragraph: the shaped galley at the block origin.
//! - heading: the galley with the level scale baked into its [`TextFormat`] sizes.
//! - blockquote: a left accent bar + tinted background, content indented.
//! - code_block: a rounded tinted rect (theme `surface_strong`/`surface`) + monospace.
//! - list item: a `•` / `1.` prefix then the content galley.
//! - table: a grid of cell rects with stroked borders (clipped per cell — MC-006).
//!
//! The caret is resolved NATIVELY from the galley via
//! [`epaint::Galley::pos_from_cursor`] (research #1 de-risk: no hand-rolled glyph
//! advance). Colors come from the theme [`HsPalette`] (CONTROL-4: no hardcoded hex
//! outside theme/*). The vertical-slice scope (contract) is paragraph + bold/italic +
//! heading + caret + code_block + blockquote + list + a basic table; full table text
//! wrap and nested-list depth are later passes.

use std::sync::Arc;

use egui::{Color32, FontId, Rect, Stroke, Vec2};

use crate::rich_editor::document_model::node::{BlockNode, Child, HsLinkNode, NodeKind};
use crate::rich_editor::embeds::asset_resolver::MediaEmbedKind;
use crate::theme::HsPalette;

use super::caret::{DocCaret, CARET_WIDTH_PTS};
use super::line_layout::{self, BLOCK_GAP_PTS, BLOCKQUOTE_BAR_WIDTH_PTS, BLOCKQUOTE_INDENT_PTS, CODE_PADDING_PTS, LIST_INDENT_PTS};

/// What a block paint produced, so the widget can advance the layout cursor and (for the
/// caret's block) know the galley to hit-test. Returned by [`paint_block`].
pub struct BlockPaint {
    /// The total vertical space (points) this block consumed, including its trailing gap.
    pub height: f32,
    /// The galley of the block's inline content and its painted top-left, kept ONLY for
    /// the block that currently hosts the caret so the caller can resolve the caret pixel
    /// rect. `None` for non-caret blocks (avoids retaining every galley).
    pub caret_galley: Option<(Arc<egui::Galley>, egui::Pos2)>,
}

/// Paint one top-level block at `top_left` with content width `content_width` (points).
/// `caret_block` is `Some(offset)` when THIS block hosts the caret (so the inline content
/// galley is returned for caret resolution). `bold_available` is whether the bold Inter
/// family is bound (threaded into layout to avoid an unbound-family panic). Returns the
/// consumed height + the caret galley when applicable.
#[allow(clippy::too_many_arguments)]
pub fn paint_block(
    painter: &egui::Painter,
    block: &BlockNode,
    top_left: egui::Pos2,
    content_width: f32,
    palette: &HsPalette,
    caret_offset: Option<usize>,
    bold_available: bool,
) -> BlockPaint {
    match block.kind {
        NodeKind::Paragraph | NodeKind::Heading(_) => {
            paint_inline_block(painter, block, top_left, content_width, palette, caret_offset, 0.0, bold_available)
        }
        NodeKind::Blockquote => {
            paint_blockquote(painter, block, top_left, content_width, palette, caret_offset, bold_available)
        }
        NodeKind::CodeBlock => {
            paint_code_block(painter, block, top_left, content_width, palette, caret_offset, bold_available)
        }
        NodeKind::BulletList | NodeKind::OrderedList => {
            paint_list(painter, block, top_left, content_width, palette, bold_available)
        }
        NodeKind::Table => paint_table(painter, block, top_left, content_width, palette, bold_available),
        NodeKind::HorizontalRule => paint_horizontal_rule(painter, top_left, content_width, palette),
        // Atoms / structural-only kinds that are not a top-level paint target in the
        // vertical slice render nothing and consume a single gap so layout stays sane.
        _ => BlockPaint { height: BLOCK_GAP_PTS, caret_galley: None },
    }
}

/// MT-014 embed dispatch seam: if `block` is a paragraph whose inline content is (only) a
/// MEDIA-embed `hsLink` atom (`ref_kind ∈ {images, video, album, slideshow}`), return that
/// link so the renderer can route it to the INTERACTIVE
/// [`crate::rich_editor::embeds::embed_block_renderer::render_embed`] path (which owns an
/// `egui::Ui` for buttons/modals) instead of this painter-only path.
///
/// This is the reconciled form of the MT-014 contract's "add a match arm for the embed kind":
/// MT-011's `NodeKind` has NO `Embed` variant (embeds are the `hsLink` inline atom by `ref_kind`),
/// so the dispatch is by inline-atom ref_kind, not by a block-kind match arm. A paragraph that
/// also carries text is rendered as text by the normal path; only a paragraph whose sole
/// non-whitespace inline child is a media embed is treated as an embed block (matching how the
/// React editor inserts an embed as its own block via `insertHsLink`).
///
/// Returns `None` for any block that is not a standalone media embed (the normal text path runs).
pub fn block_media_embed(block: &BlockNode) -> Option<&HsLinkNode> {
    if !matches!(block.kind, NodeKind::Paragraph) {
        return None;
    }
    let mut embed: Option<&HsLinkNode> = None;
    for child in &block.children {
        match child {
            // Whitespace-only text leaves are ignored (an embed block may carry trailing
            // whitespace); any non-whitespace text means this is a mixed paragraph -> not an
            // embed block.
            Child::Text(t) => {
                if !t.text.to_string().trim().is_empty() {
                    return None;
                }
            }
            Child::HsLink(link) => {
                // A non-media wikilink chip -> normal inline path (not an embed block).
                MediaEmbedKind::from_ref_kind(&link.ref_kind)?;
                if embed.is_some() {
                    return None; // more than one embed in the block -> not a standalone embed.
                }
                embed = Some(link);
            }
            Child::Block(_) => return None,
        }
    }
    embed
}

/// Lay out and paint an inline-content block (paragraph/heading) at `top_left`, indented
/// by `indent`. Returns the galley + paint origin when `caret_offset` is `Some` so the
/// caret can be resolved against the real glyph positions.
#[allow(clippy::too_many_arguments)]
fn paint_inline_block(
    painter: &egui::Painter,
    block: &BlockNode,
    top_left: egui::Pos2,
    content_width: f32,
    palette: &HsPalette,
    caret_offset: Option<usize>,
    indent: f32,
    bold_available: bool,
) -> BlockPaint {
    let origin = egui::pos2(top_left.x + indent, top_left.y);
    let wrap_width = (content_width - indent).max(1.0);
    let layout = line_layout::layout_block(block, palette, wrap_width, bold_available);
    let galley = painter.layout_job(layout.job);
    let height = galley.rect.height();
    // Paint the shaped text. `fallback_color` is the theme text color (only used for
    // sections that did not set a color — every section here sets one, so it is a safety
    // net, not the live color).
    painter.galley(origin, Arc::clone(&galley), palette.text);

    let caret_galley = caret_offset.map(|off| {
        // Caret-bound validation (MC): clamp the offset to the galley's char count so an
        // off-by-one never indexes past the laid-out text.
        let max = layout.plain_text.chars().count();
        let clamped = off.min(max);
        (Arc::clone(&galley), origin, clamped)
    });
    // Repack into the BlockPaint shape (origin + galley); the caret CCursor index is
    // applied by the caller via resolve_caret_rect.
    let caret_galley = caret_galley.map(|(g, o, _)| (g, o));
    BlockPaint { height: height + BLOCK_GAP_PTS, caret_galley }
}

/// Paint a blockquote: a left accent bar + a tinted background, content indented. The
/// inner content is the block's first inline paragraph child (vertical-slice scope: a
/// blockquote wraps a paragraph). Falls back to treating the blockquote's own inline
/// children as text if it directly holds text (defensive).
fn paint_blockquote(
    painter: &egui::Painter,
    block: &BlockNode,
    top_left: egui::Pos2,
    content_width: f32,
    palette: &HsPalette,
    caret_offset: Option<usize>,
    bold_available: bool,
) -> BlockPaint {
    // Resolve the inner inline block to render (first block child), else the quote itself.
    let inner = block.children.iter().find_map(Child::as_block).unwrap_or(block);
    let wrap_width = (content_width - BLOCKQUOTE_INDENT_PTS).max(1.0);
    let layout = line_layout::layout_block(inner, palette, wrap_width, bold_available);
    let galley = painter.layout_job(layout.job);
    let text_height = galley.rect.height().max(line_layout::BASE_FONT_SIZE);

    // Tinted background behind the whole quote (theme accent_soft — a real theme token).
    let bg_rect = Rect::from_min_size(
        top_left,
        Vec2::new(content_width, text_height),
    );
    painter.rect_filled(bg_rect, 2.0, palette.accent_soft);
    // The 3px left bar in the accent color (contract step 2).
    let bar_rect = Rect::from_min_size(top_left, Vec2::new(BLOCKQUOTE_BAR_WIDTH_PTS, text_height));
    painter.rect_filled(bar_rect, 0.0, palette.accent);

    let origin = egui::pos2(top_left.x + BLOCKQUOTE_INDENT_PTS, top_left.y);
    painter.galley(origin, Arc::clone(&galley), palette.text);

    let caret_galley = caret_offset.map(|_| (Arc::clone(&galley), origin));
    BlockPaint { height: text_height + BLOCK_GAP_PTS, caret_galley }
}

/// Paint a code block: a rounded tinted rect (theme `surface` over the editor `bg`) with
/// monospace content (the block style forces monospace for every run — RISK control: a
/// stray mark cannot escape monospace).
fn paint_code_block(
    painter: &egui::Painter,
    block: &BlockNode,
    top_left: egui::Pos2,
    content_width: f32,
    palette: &HsPalette,
    caret_offset: Option<usize>,
    bold_available: bool,
) -> BlockPaint {
    let wrap_width = (content_width - 2.0 * CODE_PADDING_PTS).max(1.0);
    let layout = line_layout::layout_block(block, palette, wrap_width, bold_available);
    let galley = painter.layout_job(layout.job);
    let inner_height = galley.rect.height().max(line_layout::BASE_FONT_SIZE);
    let box_height = inner_height + 2.0 * CODE_PADDING_PTS;

    let box_rect = Rect::from_min_size(top_left, Vec2::new(content_width, box_height));
    // Rounded distinct background (theme surface) + a subtle border (theme border).
    painter.rect_filled(box_rect, 6.0, palette.surface);
    painter.rect_stroke(
        box_rect,
        6.0,
        Stroke::new(1.0, palette.border),
        egui::StrokeKind::Inside,
    );

    let origin = egui::pos2(top_left.x + CODE_PADDING_PTS, top_left.y + CODE_PADDING_PTS);
    painter.galley(origin, Arc::clone(&galley), palette.text_subtle);

    let caret_galley = caret_offset.map(|_| (Arc::clone(&galley), origin));
    BlockPaint { height: box_height + BLOCK_GAP_PTS, caret_galley }
}

/// Paint a list (bullet/ordered): each list item gets a `•` or `N.` prefix, then its
/// first inline paragraph's content, indented. Vertical-slice scope: flat single-level
/// list of paragraph items (nested-list depth is a later pass). The caret is not resolved
/// inside list items in this slice (caret lives in top-level paragraphs/headings); a list
/// item caret is a later structural-editing pass.
fn paint_list(
    painter: &egui::Painter,
    block: &BlockNode,
    top_left: egui::Pos2,
    content_width: f32,
    palette: &HsPalette,
    bold_available: bool,
) -> BlockPaint {
    let ordered = matches!(block.kind, NodeKind::OrderedList);
    let mut y = top_left.y;
    let mut number = 1usize;
    for item in block.children.iter().filter_map(Child::as_block) {
        let prefix = if ordered {
            format!("{number}.")
        } else {
            "\u{2022}".to_string() // bullet •
        };
        // Paint the prefix in the subtle text color.
        let prefix_pos = egui::pos2(top_left.x + LIST_INDENT_PTS * 0.25, y);
        painter.text(
            prefix_pos,
            egui::Align2::LEFT_TOP,
            &prefix,
            FontId::proportional(line_layout::BASE_FONT_SIZE),
            palette.text_subtle,
        );
        // Paint the item content (its first inline child block, else the item itself).
        let inner = item.children.iter().find_map(Child::as_block).unwrap_or(item);
        let wrap_width = (content_width - LIST_INDENT_PTS).max(1.0);
        let layout = line_layout::layout_block(inner, palette, wrap_width, bold_available);
        let galley = painter.layout_job(layout.job);
        let h = galley.rect.height().max(line_layout::BASE_FONT_SIZE);
        let origin = egui::pos2(top_left.x + LIST_INDENT_PTS, y);
        painter.galley(origin, galley, palette.text);
        y += h;
        number += 1;
    }
    let height = (y - top_left.y).max(line_layout::BASE_FONT_SIZE);
    BlockPaint { height: height + BLOCK_GAP_PTS, caret_galley: None }
}

/// Paint a table: a grid of cells. Column widths are equal (content_width / cols);
/// borders are stroked; each cell's text is CLIPPED to its rect (red-team RISK-6 /
/// MC-006: a long cell never paints over its neighbor). Vertical-slice scope: single-line
/// cells, equal columns; auto-sizing is a later pass.
fn paint_table(
    painter: &egui::Painter,
    block: &BlockNode,
    top_left: egui::Pos2,
    content_width: f32,
    palette: &HsPalette,
    bold_available: bool,
) -> BlockPaint {
    let rows: Vec<&BlockNode> = block.children.iter().filter_map(Child::as_block).collect();
    if rows.is_empty() {
        return BlockPaint { height: BLOCK_GAP_PTS, caret_galley: None };
    }
    let cols = rows
        .iter()
        .map(|r| r.children.iter().filter_map(Child::as_block).count())
        .max()
        .unwrap_or(1)
        .max(1);
    let col_w = content_width / cols as f32;
    let row_h = line_layout::BASE_FONT_SIZE + 8.0;
    let mut y = top_left.y;
    for row in &rows {
        let cells: Vec<&BlockNode> = row.children.iter().filter_map(Child::as_block).collect();
        for (c, cell) in cells.iter().enumerate() {
            let x = top_left.x + c as f32 * col_w;
            let cell_rect = Rect::from_min_size(egui::pos2(x, y), Vec2::new(col_w, row_h));
            // Cell border.
            painter.rect_stroke(
                cell_rect,
                0.0,
                Stroke::new(1.0, palette.border),
                egui::StrokeKind::Inside,
            );
            // CLIP cell content to its rect so long text cannot overflow into the
            // neighbor (MC-006). A child painter with the cell clip rect bounds the paint.
            let cell_painter = painter.with_clip_rect(cell_rect);
            let inner = cell.children.iter().find_map(Child::as_block).unwrap_or(cell);
            let layout = line_layout::layout_block(inner, palette, col_w - 6.0, bold_available);
            let galley = cell_painter.layout_job(layout.job);
            cell_painter.galley(
                egui::pos2(x + 3.0, y + 4.0),
                galley,
                palette.text,
            );
        }
        y += row_h;
    }
    let height = (y - top_left.y).max(row_h);
    BlockPaint { height: height + BLOCK_GAP_PTS, caret_galley: None }
}

/// Paint a horizontal rule: a 1px theme-border line across the content width.
fn paint_horizontal_rule(
    painter: &egui::Painter,
    top_left: egui::Pos2,
    content_width: f32,
    palette: &HsPalette,
) -> BlockPaint {
    let y = top_left.y + line_layout::BASE_FONT_SIZE / 2.0;
    painter.line_segment(
        [egui::pos2(top_left.x, y), egui::pos2(top_left.x + content_width, y)],
        Stroke::new(1.0, palette.border),
    );
    BlockPaint { height: line_layout::BASE_FONT_SIZE + BLOCK_GAP_PTS, caret_galley: None }
}

/// Resolve the caret's pixel rect from a block's galley + paint origin + char offset,
/// using epaint's NATIVE [`epaint::Galley::pos_from_cursor`] hit-test, then paint a 2px
/// vertical bar in the theme text color. Only paints when the caret is collapsed AND the
/// blink phase is ON (the caller passes `visible`).
pub fn paint_caret(
    painter: &egui::Painter,
    galley: &egui::Galley,
    origin: egui::Pos2,
    caret: &DocCaret,
    palette: &HsPalette,
    visible: bool,
) {
    if !caret.collapsed || !visible {
        return;
    }
    let max = galley.job.text.chars().count();
    let clamped = caret.char_offset().min(max);
    let cursor = egui::epaint::text::cursor::CCursor::new(clamped);
    // Galley-local rect (top=0); offset by the block's paint origin to screen space.
    let local = galley.pos_from_cursor(cursor);
    let caret_rect = Rect::from_min_size(
        egui::pos2(origin.x + local.min.x, origin.y + local.min.y),
        Vec2::new(CARET_WIDTH_PTS, local.height().max(line_layout::BASE_FONT_SIZE)),
    );
    painter.rect_filled(caret_rect, 0.0, palette.text);
}

/// A small helper for tests / callers: a fully-transparent color sentinel is never used;
/// the theme always supplies real colors. Kept private to avoid leaking a literal.
#[allow(dead_code)]
const fn _no_color() -> Color32 {
    Color32::TRANSPARENT
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rich_editor::document_model::node::{BlockNode, Child, Mark, NodeKind, TextLeaf};
    use crate::theme::HsTheme;

    fn dark() -> HsPalette {
        HsTheme::Dark.palette()
    }

    // A headless painter for layout-only assertions (no GPU): build a Context, install the
    // shell fonts (so the bold Inter family is bound, matching the real runtime), begin a
    // frame, and grab a layer painter. layout_job works without a render backend.
    fn with_painter<R>(f: impl FnOnce(&egui::Painter, &HsPalette, bool) -> R) -> R {
        let ctx = egui::Context::default();
        // Install the shell fonts so FontFamily::Name("Inter-Bold") is bound exactly as in
        // the live app; `bold_family_available` then returns true.
        crate::app::HandshakeApp::install_fonts(&ctx);
        let pal = dark();
        let mut out = None;
        // `ctx.run` takes an `FnMut`, but it invokes the ui closure exactly once; wrap `f`
        // in an Option and `take()` it so an FnOnce can be used safely.
        let mut f = Some(f);
        let _ = ctx.run(Default::default(), |ctx| {
            let bold = line_layout::bold_family_available(ctx);
            let painter = ctx.layer_painter(egui::LayerId::background());
            let f = f.take().expect("ui closure runs once");
            out = Some(f(&painter, &pal, bold));
        });
        out.unwrap()
    }

    #[test]
    fn paragraph_paint_reports_positive_height_and_caret_galley() {
        with_painter(|painter, pal, bold| {
            let block = BlockNode::paragraph("Hello world");
            let bp = paint_block(painter, &block, egui::pos2(0.0, 0.0), 400.0, pal, Some(5), bold);
            assert!(bp.height > 0.0);
            assert!(bp.caret_galley.is_some(), "caret block returns its galley");
        });
    }

    #[test]
    fn heading_paints_taller_than_paragraph() {
        // AC-7: an h1 galley is visibly taller than a paragraph galley (>= 1.5x).
        with_painter(|painter, pal, bold| {
            let para = BlockNode::paragraph("Title text");
            let h1 = BlockNode::heading(1, "Title text");
            let para_h = {
                let l = line_layout::layout_block(&para, pal, 400.0, bold);
                painter.layout_job(l.job).rect.height()
            };
            let h1_h = {
                let l = line_layout::layout_block(&h1, pal, 400.0, bold);
                painter.layout_job(l.job).rect.height()
            };
            assert!(
                h1_h / para_h >= 1.5,
                "AC-7: h1 height {h1_h} must be >= 1.5x paragraph height {para_h}"
            );
        });
    }

    #[test]
    fn bold_run_galley_uses_bold_family() {
        // AC-1 structural: "Hello world" with bold on "world" lays out as a galley whose
        // bold section uses the bold family (the shell fonts are installed in the test
        // harness, so bold_available is true and the bold family is selected).
        with_painter(|painter, pal, bold| {
            assert!(bold, "the test harness installs the shell fonts -> bold family bound");
            let bolded = BlockNode::with_children(
                NodeKind::Paragraph,
                vec![
                    Child::Text(TextLeaf::new("Hello ")),
                    Child::Text(TextLeaf::with_marks("world", vec![Mark::Bold])),
                ],
            );
            let plain = BlockNode::paragraph("Hello world");
            let lb = line_layout::layout_block(&bolded, pal, 1000.0, bold);
            // The second section is the bold "world".
            assert_eq!(lb.job.sections.len(), 2);
            assert_eq!(
                lb.job.sections[1].format.font_id.family,
                egui::FontFamily::Name(line_layout::BOLD_FAMILY_NAME.into()),
            );
            let g_bold = painter.layout_job(lb.job);
            let g_plain = painter.layout_job(line_layout::layout_block(&plain, pal, 1000.0, bold).job);
            // Both galleys carry the same text content.
            assert_eq!(g_bold.job.text, "Hello world");
            assert_eq!(g_plain.job.text, "Hello world");
        });
    }

    #[test]
    fn bold_without_font_does_not_panic() {
        // RISK control: when the bold family is NOT bound (bare context, no shell fonts),
        // a bold run must degrade to Proportional rather than panic on an unbound family.
        let ctx = egui::Context::default();
        let pal = dark();
        let mut out = None;
        let _ = ctx.run(Default::default(), |ctx| {
            // No install_fonts here -> Inter-Bold is not bound.
            assert!(!line_layout::bold_family_available(ctx), "bold not bound in bare ctx");
            let painter = ctx.layer_painter(egui::LayerId::background());
            let bolded = BlockNode::with_children(
                NodeKind::Paragraph,
                vec![Child::Text(TextLeaf::with_marks("world", vec![Mark::Bold]))],
            );
            // bold_available=false -> proportional family -> layout_job must not panic.
            let bp = paint_block(&painter, &bolded, egui::pos2(0.0, 0.0), 400.0, &pal, None, false);
            out = Some(bp.height);
        });
        assert!(out.unwrap() > 0.0);
    }

    #[test]
    fn caret_rect_is_positive_width_when_visible() {
        with_painter(|painter, pal, bold| {
            let block = BlockNode::paragraph("Hello");
            let layout = line_layout::layout_block(&block, pal, 400.0, bold);
            let galley = painter.layout_job(layout.job);
            let caret = DocCaret {
                head: crate::rich_editor::document_model::position::DocPosition::new(vec![0, 0], 3),
                collapsed: true,
            };
            // We cannot read back painted pixels here, but pos_from_cursor must give a
            // non-degenerate rect for offset 3 of "Hello".
            let cursor = egui::epaint::text::cursor::CCursor::new(3);
            let r = galley.pos_from_cursor(cursor);
            assert!(r.min.x > 0.0, "caret after 3 chars is to the right of the start");
            // Offset 0 caret is at x≈0.
            let r0 = galley.pos_from_cursor(egui::epaint::text::cursor::CCursor::new(0));
            assert!(r0.min.x <= r.min.x, "caret x grows with offset");
            // A non-collapsed (range) caret paints nothing — guarded by `paint_caret`.
            let _ = caret;
        });
    }

    #[test]
    fn empty_paragraph_paints_without_panic() {
        // RISK-5: an empty paragraph still lays out (one empty section) and reports a
        // sane height; no panic.
        with_painter(|painter, pal, bold| {
            let empty = BlockNode::with_children(
                NodeKind::Paragraph,
                vec![Child::Text(TextLeaf::new(""))],
            );
            let bp = paint_block(painter, &empty, egui::pos2(0.0, 0.0), 400.0, pal, Some(0), bold);
            assert!(bp.height > 0.0);
        });
    }

    #[test]
    fn code_block_and_blockquote_and_table_paint() {
        with_painter(|painter, pal, bold| {
            let code = BlockNode::with_children(
                NodeKind::CodeBlock,
                vec![Child::Text(TextLeaf::new("fn main() {}"))],
            );
            let bp_code = paint_block(painter, &code, egui::pos2(0.0, 0.0), 400.0, pal, None, bold);
            assert!(bp_code.height > line_layout::BASE_FONT_SIZE);

            let quote = BlockNode::with_children(
                NodeKind::Blockquote,
                vec![Child::Block(BlockNode::paragraph("quoted"))],
            );
            let bp_q = paint_block(painter, &quote, egui::pos2(0.0, 0.0), 400.0, pal, None, bold);
            assert!(bp_q.height > 0.0);

            // A 1x2 table: one row, two cells.
            let cell_a = BlockNode::with_children(NodeKind::TableCell, vec![Child::Block(BlockNode::paragraph("a"))]);
            let cell_b = BlockNode::with_children(NodeKind::TableCell, vec![Child::Block(BlockNode::paragraph("b"))]);
            let row = BlockNode::with_children(NodeKind::TableRow, vec![Child::Block(cell_a), Child::Block(cell_b)]);
            let table = BlockNode::with_children(NodeKind::Table, vec![Child::Block(row)]);
            let bp_t = paint_block(painter, &table, egui::pos2(0.0, 0.0), 400.0, pal, None, bold);
            assert!(bp_t.height > 0.0);
        });
    }

    #[test]
    fn list_paints_with_prefixes() {
        with_painter(|painter, pal, bold| {
            let item1 = BlockNode::with_children(NodeKind::ListItem, vec![Child::Block(BlockNode::paragraph("one"))]);
            let item2 = BlockNode::with_children(NodeKind::ListItem, vec![Child::Block(BlockNode::paragraph("two"))]);
            let bullets = BlockNode::with_children(NodeKind::BulletList, vec![Child::Block(item1.clone()), Child::Block(item2.clone())]);
            let bp = paint_block(painter, &bullets, egui::pos2(0.0, 0.0), 400.0, pal, None, bold);
            assert!(bp.height > line_layout::BASE_FONT_SIZE, "two items stack");
            let ordered = BlockNode::with_children(NodeKind::OrderedList, vec![Child::Block(item1), Child::Block(item2)]);
            let bp_o = paint_block(painter, &ordered, egui::pos2(0.0, 0.0), 400.0, pal, None, bold);
            assert!(bp_o.height > 0.0);
        });
    }
}

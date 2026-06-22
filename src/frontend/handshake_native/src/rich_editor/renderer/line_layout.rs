//! Styled-line layout for the WYSIWYG renderer (WP-KERNEL-012 MT-012).
//!
//! ## Engine decision (contract RENDERING ENGINE RECONCILIATION + verified egui-0.33 fact)
//!
//! The MT contract's scope summary first describes a `cosmic-text` glyph-shaping path,
//! but its own `implementation_notes` (KERNEL_BUILDER gate 2026-06-22 + research
//! wf_ffa74d6d) OVERRIDE that: "PREFER egui LayoutJob + epaint Galley as the
//! rendering+caret engine over cosmic-text … Use cosmic-text ONLY if the mandatory
//! vertical-slice spike proves egui LayoutJob insufficient." A vertical-slice
//! implementation on `egui::text::LayoutJob` + [`epaint::Galley`] renders multi-run
//! styled text (bold/italic/code per [`TextLeaf`] mark set) in ONE galley and gives
//! NATIVE caret pixel hit-testing via [`Galley::pos_from_cursor`] /
//! [`Galley::cursor_from_pos`] — so cosmic-text is NOT used and is NOT added as a
//! dependency (it would also be a brand-new dependency family). This de-risks the
//! research's #1 hardest gap (no hand-rolled glyph-advance caret).
//!
//! ## Italic: native epaint skew, NOT a bundled font (verified deviation from the
//! contract's stale ITALIC NOTE)
//!
//! The contract's ITALIC NOTE says "egui does NOT synthesize italic" and asks to bundle
//! `Inter-Italic`. That is STALE for egui/epaint 0.33: `epaint-0.33.3`
//! `text/text_layout.rs:868` applies a real `0.25` horizontal skew when
//! `TextFormat.italics == true`, i.e. epaint DOES synthesize a visible italic. Verified
//! by reading the locked epaint source. No `Inter-Italic.ttf` exists on disk and
//! fetching one is a network-blocked path, so this MT uses the field-correct
//! `TextFormat.italics` flag (a genuine skewed glyph) instead of fabricating a font
//! asset. Bold uses the REAL bundled `Inter-Bold` family (`app::INTER_BOLD_FAMILY`);
//! bold+italic combines the bold family with the skew. This is documented as a
//! deviation in the handoff so a validator can re-verify the epaint behavior.

use egui::text::{LayoutJob, TextFormat};
use egui::{FontFamily, FontId, Stroke};

use crate::rich_editor::document_model::node::{BlockNode, Child, Mark};
use crate::theme::HsPalette;

/// The named font family for the bold Inter face registered by the WP-011 shell
/// (`app::INTER_BOLD_FAMILY` = `"Inter-Bold"`). Re-declared here as a `&str` so the
/// renderer does not depend on the `bundled-fonts` cfg gate: when the bold family is
/// NOT registered (e.g. `--no-default-features`), epaint falls back to the default
/// proportional face for that section, so bold text still renders (just not in the
/// bold face) rather than panicking. The default-features build (the shipped binary)
/// always has it.
pub const BOLD_FAMILY_NAME: &str = "Inter-Bold";

/// Inline code uses egui's built-in Monospace family (always present — eframe's
/// `default_fonts` registers it), so a code mark renders in a monospace face without a
/// new bundled asset.
const CODE_FAMILY: FontFamily = FontFamily::Monospace;

/// The base proportional font size (logical points) for body paragraph text. Heading
/// scale factors multiply this. Matches the shell's default body text size.
pub const BASE_FONT_SIZE: f32 = 15.0;

/// Heading scale factors for levels 1..=3 (contract step 2: h1=1.8, h2=1.5, h3=1.25).
/// Indexed by `level - 1`. A level outside 1..=3 is already clamped by
/// [`crate::rich_editor::document_model::node::HeadingLevel`], so this never indexes
/// out of range.
pub const HEADING_SCALE: [f32; 3] = [1.8, 1.5, 1.25];

/// Vertical gap (points) painted after each top-level block so blocks do not touch.
pub const BLOCK_GAP_PTS: f32 = 6.0;

/// Blockquote left-bar width (points) — the contract's "3 px left bar".
pub const BLOCKQUOTE_BAR_WIDTH_PTS: f32 = 3.0;

/// Blockquote content indent (points) — the contract's "indent 16 px".
pub const BLOCKQUOTE_INDENT_PTS: f32 = 16.0;

/// Code-block inner padding (points) on every side of the monospace content.
pub const CODE_PADDING_PTS: f32 = 8.0;

/// List-item content indent (points): the bullet/number prefix sits left of this.
pub const LIST_INDENT_PTS: f32 = 20.0;

/// Convert logical egui POINTS to physical PIXELS through the active context scale
/// (red-team RISK-1 control: a SINGLE conversion helper, unit-tested at
/// `pixels_per_point = 2.0`). egui's own painter works in points, so the renderer
/// stays in points for all paint/caret math and only uses this where a pixel value is
/// genuinely required (e.g. a device-pixel-snapped caret width). Keeping the one helper
/// means there is exactly one place a high-DPI mismatch could live.
pub fn pts_to_px(pts: f32, ctx: &egui::Context) -> f32 {
    pts * ctx.pixels_per_point()
}

/// The resolved per-block text style inputs the layout builder needs. Derived once per
/// block from its [`BlockNode::kind`] so heading scale / code-block monospace are
/// applied uniformly to the whole block.
#[derive(Clone, Copy, Debug)]
pub struct BlockTextStyle {
    /// The font size (points) for this block's body text (base size * heading scale, or
    /// base size for a paragraph). A code block uses the base size in monospace.
    pub size: f32,
    /// Force every run into the monospace family (true only for a `code_block`), so a
    /// stray mark inside a code block can never escape the monospace face.
    pub force_monospace: bool,
}

impl BlockTextStyle {
    /// Body paragraph style (base size, proportional).
    pub fn body() -> Self {
        Self { size: BASE_FONT_SIZE, force_monospace: false }
    }

    /// Heading style: base size scaled by the level factor.
    pub fn heading(level: u8) -> Self {
        let idx = (level.clamp(1, 3) - 1) as usize;
        Self { size: BASE_FONT_SIZE * HEADING_SCALE[idx], force_monospace: false }
    }

    /// Code-block style: base size, monospace forced.
    pub fn code_block() -> Self {
        Self { size: BASE_FONT_SIZE, force_monospace: true }
    }
}

/// Build the [`TextFormat`] for one inline run given its marks, the block style, the
/// theme palette, and whether the bold Inter family is bound in the active context.
/// Resolves color, font family (bold/code), italic skew, underline, and strike-through
/// from the marks. All colors come from the theme palette ([`HsPalette`]) — NEVER a
/// hardcoded hex (CONTROL-4 reuse of the theme layer).
///
/// `bold_available` MUST be true only when `FontFamily::Name("Inter-Bold")` is actually
/// bound (the WP-011 shell binds it via `app::install_fonts`; a bare context without the
/// shell fonts does NOT). When it is false, a bold run degrades to the proportional
/// family rather than requesting an unbound family — epaint PANICS on an unbound named
/// family, so this guard makes the renderer panic-proof when mounted before fonts load.
pub fn text_format_for_run(
    marks: &[Mark],
    style: BlockTextStyle,
    palette: &HsPalette,
    bold_available: bool,
) -> TextFormat {
    let has = |probe: &Mark| marks.iter().any(|m| m.same_type(probe));
    let bold = has(&Mark::Bold);
    let italic = has(&Mark::Italic);
    let code = has(&Mark::Code);
    let underline = has(&Mark::Underline);
    let strike = has(&Mark::Strike);
    let is_link = marks.iter().any(|m| matches!(m, Mark::Link { .. }));

    // Family precedence: a code mark (or a code_block) forces monospace; else a bold
    // mark selects the bold Inter family WHEN it is bound; else the default proportional
    // family. The `bold_available` guard prevents an unbound-family panic (RISK control).
    let family = if code || style.force_monospace {
        CODE_FAMILY
    } else if bold && bold_available {
        FontFamily::Name(BOLD_FAMILY_NAME.into())
    } else {
        FontFamily::Proportional
    };

    // Color: a link reads in the accent color (matching the React link styling); inline
    // code reads in the subtle text color over its tinted background; everything else is
    // the primary text color. All theme tokens.
    let color = if is_link {
        palette.accent
    } else if code {
        palette.text_subtle
    } else {
        palette.text
    };

    let underline_stroke = if underline || is_link {
        // Links are underlined too (the React link affordance); a 1px line in the run
        // color reads as the standard underline.
        Stroke::new(1.0, color)
    } else {
        Stroke::NONE
    };
    let strike_stroke = if strike {
        Stroke::new(1.0, color)
    } else {
        Stroke::NONE
    };

    TextFormat {
        font_id: FontId::new(style.size, family),
        color,
        // epaint 0.33 synthesizes a real italic via a 0.25 skew (verified in
        // text_layout.rs:868); a code run is never italicized (monospace code stays
        // upright like every editor).
        italics: italic && !code,
        underline: underline_stroke,
        strikethrough: strike_stroke,
        ..Default::default()
    }
}

/// A built layout job for one block plus the flat plain text it covers, so the caret
/// layer can map a per-block CHAR offset to a [`epaint::text::cursor::CCursor`] index
/// and then to a pixel rect via the galley.
///
/// `plain_text` is the concatenation of every inline child's contributed text in the
/// SAME order the job appends sections, so a char offset into `plain_text` is exactly
/// the `CCursor.index` for the galley. An `hsLink` inline atom contributes its display
/// label so it is visible AND occupies caret positions consistent with the model's
/// "atom = size 1" rule is intentionally NOT applied to the visible text (an atom shows
/// its label); the caret layer treats text-leaf offsets only, which is the MT-012
/// vertical-slice scope (wikilink caret interplay is MT-015).
pub struct BlockLayout {
    /// The styled job to hand to `painter.layout_job` / `ui.fonts(|f| f.layout_job(..))`.
    pub job: LayoutJob,
    /// The flat plain text the job renders, char-for-char aligned with the galley's
    /// CCursor indices.
    pub plain_text: String,
}

/// Build a [`BlockLayout`] for an inline-content block (`paragraph`, `heading`,
/// `code_block`). Iterates the block's inline children, appending one [`LayoutJob`]
/// section per text run (with its mark-derived [`TextFormat`]) and the display label
/// for each `hsLink` atom. `wrap_width` bounds the line wrap (the available content
/// width in points); `0.0` disables wrapping.
///
/// Empty-block control (red-team RISK-5 / RISK empty TextLeaf): a block with no inline
/// children, or whose only child is a zero-length text leaf, still produces a job
/// containing a single empty section so the galley has one row of the correct height
/// and the caret has a position to sit at — layout never panics on a 0-length rope (the
/// epaint layouter handles "" by producing an empty row; we additionally guarantee at
/// least one section so the row height is the block's font size).
pub fn layout_block(
    block: &BlockNode,
    palette: &HsPalette,
    wrap_width: f32,
    bold_available: bool,
) -> BlockLayout {
    let style = block_style(block);
    let mut job = LayoutJob::default();
    job.wrap.max_width = wrap_width;
    let mut plain = String::new();

    for child in &block.children {
        match child {
            Child::Text(leaf) => {
                let text = leaf.text.to_string();
                let fmt = text_format_for_run(&leaf.marks, style, palette, bold_available);
                job.append(&text, 0.0, fmt);
                plain.push_str(&text);
            }
            Child::HsLink(link) => {
                // Show the atom's display label (or `refKind:refValue` when blank, the
                // React default) in the accent/link style so a wikilink is visible in
                // the vertical slice. The full interactive wikilink is MT-015.
                let label = if link.label.is_empty() {
                    format!("{}:{}", link.ref_kind, link.ref_value)
                } else {
                    link.label.clone()
                };
                let fmt = text_format_for_run(
                    &[Mark::Link { href: String::new() }],
                    style,
                    palette,
                    bold_available,
                );
                job.append(&label, 0.0, fmt);
                plain.push_str(&label);
            }
            Child::Transclusion(t) => {
                // A loomTransclusion inline atom that is NOT a standalone embed block (e.g. mixed
                // into a paragraph) shows a short reference label in the subtle/link style so it is
                // visible in the inline flow. A STANDALONE transclusion block is routed to the
                // interactive transclusion_view by the renderer (the embed-dispatch seam), exactly
                // like a media embed; this inline fallback keeps a mixed-paragraph transclusion
                // visible rather than dropping it.
                let label = format!("⟢ {}", t.ref_value);
                let fmt = text_format_for_run(
                    &[Mark::Link { href: String::new() }],
                    style,
                    palette,
                    bold_available,
                );
                job.append(&label, 0.0, fmt);
                plain.push_str(&label);
            }
            Child::Block(_) => {
                // An inline-content block holds no block children (schema-enforced); a
                // stray block child is ignored here rather than panicking. Structural
                // block rendering is the caller's job (block_renderer recurses).
            }
        }
    }

    // Empty-block guarantee: ensure at least one (possibly empty) section so the galley
    // produces a row of the block's font height and the caret has somewhere to sit.
    if job.sections.is_empty() {
        let fmt = text_format_for_run(&[], style, palette, bold_available);
        job.append("", 0.0, fmt);
    }

    BlockLayout { job, plain_text: plain }
}

/// True when the bold Inter family (`FontFamily::Name("Inter-Bold")`) is actually bound in
/// the active context. The renderer queries this ONCE per frame and threads it into
/// [`layout_block`] so a bold run never requests an unbound family (which epaint would
/// panic on). The WP-011 shell binds it at startup via `app::install_fonts`.
pub fn bold_family_available(ctx: &egui::Context) -> bool {
    let target = FontFamily::Name(BOLD_FAMILY_NAME.into());
    ctx.fonts(|f| f.families().into_iter().any(|fam| fam == target))
}

/// Resolve the per-block [`BlockTextStyle`] from a block's kind. Paragraph/other inline
/// blocks use the body style; a heading uses its level scale; a code block forces
/// monospace.
pub fn block_style(block: &BlockNode) -> BlockTextStyle {
    use crate::rich_editor::document_model::node::NodeKind;
    match block.kind {
        NodeKind::Heading(level) => BlockTextStyle::heading(level.get()),
        NodeKind::CodeBlock => BlockTextStyle::code_block(),
        _ => BlockTextStyle::body(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rich_editor::document_model::node::{BlockNode, HeadingLevel, NodeKind, TextLeaf};
    use crate::theme::HsTheme;

    fn dark() -> HsPalette {
        HsTheme::Dark.palette()
    }

    // RISK-1 control: the single pts->px helper is exact at pixels_per_point = 2.0.
    #[test]
    fn pts_to_px_at_2x() {
        let ctx = egui::Context::default();
        // egui applies `set_pixels_per_point` on the NEXT frame, so run one frame to make
        // the 2.0 scale live before reading it back through the helper.
        ctx.set_pixels_per_point(2.0);
        let _ = ctx.run(Default::default(), |_| {});
        assert_eq!(ctx.pixels_per_point(), 2.0, "scale must be live after one frame");
        assert_eq!(pts_to_px(10.0, &ctx), 20.0);
        assert_eq!(pts_to_px(0.0, &ctx), 0.0);
        assert_eq!(pts_to_px(7.5, &ctx), 15.0);
    }

    #[test]
    fn heading_scale_is_at_least_1_5x_for_h1() {
        // AC-7: h1 must be >= 1.5x paragraph height. h1 = 1.8x by table.
        let body = BlockTextStyle::body().size;
        let h1 = BlockTextStyle::heading(1).size;
        assert!(h1 / body >= 1.5, "h1 {h1} / body {body} must be >= 1.5x");
        // h2/h3 monotonic-decreasing but still > body.
        assert!(BlockTextStyle::heading(2).size > body);
        assert!(BlockTextStyle::heading(3).size > body);
    }

    #[test]
    fn bold_run_uses_bold_family_when_available() {
        // bold_available=true -> the bold Inter family is selected.
        let fmt = text_format_for_run(&[Mark::Bold], BlockTextStyle::body(), &dark(), true);
        assert_eq!(fmt.font_id.family, FontFamily::Name(BOLD_FAMILY_NAME.into()));
        assert!(!fmt.italics);
    }

    #[test]
    fn bold_run_degrades_to_proportional_when_unavailable() {
        // bold_available=false -> proportional family (no unbound-family panic risk).
        let fmt = text_format_for_run(&[Mark::Bold], BlockTextStyle::body(), &dark(), false);
        assert_eq!(fmt.font_id.family, FontFamily::Proportional);
    }

    #[test]
    fn italic_run_sets_italics_skew() {
        // The verified egui-0.33 behavior: italics is a real skew, no bundled font.
        let fmt = text_format_for_run(&[Mark::Italic], BlockTextStyle::body(), &dark(), true);
        assert!(fmt.italics, "italic mark must set TextFormat.italics (epaint skews it)");
        assert_eq!(fmt.font_id.family, FontFamily::Proportional);
    }

    #[test]
    fn code_run_is_monospace_never_italic() {
        let fmt = text_format_for_run(
            &[Mark::Code, Mark::Italic],
            BlockTextStyle::body(),
            &dark(),
            true,
        );
        assert_eq!(fmt.font_id.family, FontFamily::Monospace);
        assert!(!fmt.italics, "code is upright even with an italic mark present");
    }

    #[test]
    fn code_block_forces_monospace_for_every_run() {
        let style = BlockTextStyle::code_block();
        // Even an unmarked run is monospace in a code block.
        let fmt = text_format_for_run(&[], style, &dark(), true);
        assert_eq!(fmt.font_id.family, FontFamily::Monospace);
    }

    #[test]
    fn colors_come_from_theme_not_literals() {
        let pal = dark();
        let plain = text_format_for_run(&[], BlockTextStyle::body(), &pal, true);
        assert_eq!(plain.color, pal.text);
        let link = text_format_for_run(
            &[Mark::Link { href: "x".into() }],
            BlockTextStyle::body(),
            &pal,
            true,
        );
        assert_eq!(link.color, pal.accent);
        assert!(link.underline.width > 0.0, "links are underlined");
    }

    #[test]
    fn layout_block_plain_text_matches_runs_in_order() {
        // "Hello " regular + "world" bold -> plain text "Hello world", two sections.
        let block = BlockNode::with_children(
            NodeKind::Paragraph,
            vec![
                Child::Text(TextLeaf::new("Hello ")),
                Child::Text(TextLeaf::with_marks("world", vec![Mark::Bold])),
            ],
        );
        let bl = layout_block(&block, &dark(), 400.0, true);
        assert_eq!(bl.plain_text, "Hello world");
        assert_eq!(bl.job.sections.len(), 2, "one section per run");
    }

    #[test]
    fn empty_block_yields_one_section_no_panic() {
        // A paragraph holding a single empty leaf must still produce a non-empty job
        // (one empty section) so the row has height and the caret can sit (RISK-5).
        let block = BlockNode::with_children(
            NodeKind::Paragraph,
            vec![Child::Text(TextLeaf::new(""))],
        );
        let bl = layout_block(&block, &dark(), 400.0, true);
        assert_eq!(bl.plain_text, "");
        assert_eq!(bl.job.sections.len(), 1);

        // A block with NO children at all also yields one section.
        let empty = BlockNode::new(NodeKind::Paragraph);
        let bl2 = layout_block(&empty, &dark(), 400.0, true);
        assert!(!bl2.job.sections.is_empty());
    }

    #[test]
    fn heading_block_style_scales() {
        let h1 = BlockNode::heading(1, "Title");
        let style = block_style(&h1);
        assert_eq!(style.size, BASE_FONT_SIZE * HEADING_SCALE[0]);
        let para = BlockNode::paragraph("body");
        assert_eq!(block_style(&para).size, BASE_FONT_SIZE);
        // A clamped heading level still indexes safely.
        let h = BlockNode::with_children(
            NodeKind::Heading(HeadingLevel::new(9)),
            vec![Child::Text(TextLeaf::new("x"))],
        );
        assert_eq!(block_style(&h).size, BASE_FONT_SIZE * HEADING_SCALE[2]);
    }
}

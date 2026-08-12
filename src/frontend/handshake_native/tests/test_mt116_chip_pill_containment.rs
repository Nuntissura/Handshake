//! WP-KERNEL-012 MT-116 — MOUNTED visual proof that a cross-reference chip's painted LABEL is
//! contained in, and fills, its painted PILL at NON-DEFAULT editor font sizes.
//!
//! ## Why this file exists (and why the existing unit guard was not enough)
//!
//! The MT-116 fix threads the block's resolved style size onto `WikilinkChipSpec::font_size` and
//! paints the chip label with `egui::FontId::proportional(spec.font_size)` instead of the
//! `line_layout::BASE_FONT_SIZE` CONSTANT (15.0). The in-crate unit guard
//! (`mt116_chip_metric_tests`) pins the SPEC VALUE only — the prior session disclosed verbatim:
//! "the new test pins the SPEC value, not the paint call. Reverting only the paint site to the
//! constant would NOT turn this test red."
//!
//! This file closes that gap by asserting over the PAINTED PIXELS. It renders the real
//! `RichEditorWidget` through `egui_kittest` + wgpu, captures the frame, and measures:
//!
//! - CONTAINMENT: no glyph ink may exist to the right of the pill (the below-default-size
//!   signature of the bug: a pill measured at 11.0 with a label painted at the 15.0 constant
//!   spills its tail outside the pill), and the ink box must sit inside the pill vertically.
//! - FILL: the ink box must span >= `MIN_FILL_RATIO` of the pill width (the above-default-size and
//!   heading signature: a pill measured at 24.0 / 21.6 with a label painted at the 15.0 constant
//!   leaves the pill visibly empty on the right).
//! - INK HEIGHT: the painted glyph height must scale with the block's resolved style size, not sit
//!   at the constant's cap height.
//!
//! Reverting `egui::FontId::proportional(spec.font_size)` back to
//! `egui::FontId::proportional(line_layout::BASE_FONT_SIZE)` therefore turns EVERY scenario in this
//! file RED. That non-vacuity is the deliverable.
//!
//! ## Scenario matrix (AC-116-2 / AC-116-4)
//!
//! | scenario | chip variant | block | editor font | resolved style size | bug signature caught |
//! |----------|--------------|-------|-------------|---------------------|----------------------|
//! | `above-default-resolved` | resolved (accent pill) | paragraph | 24.0 | 24.0 | under-fill |
//! | `below-default-unresolved-missing` | unresolved `-MISSING` (error pill) | paragraph | 11.0 | 11.0 | overflow |
//! | `heading-h1-code-ref` | code-ref | H1 heading | 12.0 | 21.6 (HEADING_SCALE 1.8) | under-fill |
//!
//! Every chip carries a 44-CHARACTER id (`MT116_ID_LEN`), the exact length the MT contract names.
//!
//! ## PT-116-3 (stale-frame failure mode)
//!
//! Each scenario captures a BEFORE frame at the DEFAULT 15.0 editor size, mutates the live state to
//! the scenario size, then captures the AFTER frame through `render_settled_proof_frame` (never
//! `render_proof_frame`, which in this WP has painted the PRECEDING frame and produced
//! byte-identical pairs). The two frames' raw-pixel sha256 are asserted DIFFERENT and both are
//! printed, so a stale capture cannot masquerade as proof.
//!
//! ## Running it
//!
//! ```text
//! HANDSHAKE_GPU_SCREENSHOT=1 cargo test --manifest-path src/frontend/handshake_native/Cargo.toml \
//!     --test test_mt116_chip_pill_containment -- --nocapture
//! ```
//!
//! Without `HANDSHAKE_GPU_SCREENSHOT=1` the shared screenshot harness records a typed DEFERRED
//! outcome and the test asserts THAT (it never silently passes as if pixels had been inspected).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use egui::Color32;
use sha2::{Digest, Sha256};

#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;
use screenshot_harness::screenshot_marker::gpu_screenshot_enabled;
use screenshot_harness::ScreenshotHarness as Harness;

use handshake_native::interop::cross_ref::CODE_REF_KIND;
use handshake_native::interop::locus_interop::LOCUS_REF_KIND;
use handshake_native::rich_editor::document_model::node::{BlockNode, Child, HsLinkNode, NodeKind};
use handshake_native::rich_editor::renderer::line_layout::{
    block_style_with_base, BASE_FONT_SIZE, HEADING_SCALE,
};
use handshake_native::rich_editor::renderer::rich_editor_widget::{
    RichEditorState, RichEditorWidget,
};
use handshake_native::theme::{HsPalette, HsTheme};

/// The exact id length the MT-116 contract names ("a 44-character work-unit id").
const MT116_ID_LEN: usize = 44;

/// Harness size (points). The width is NOT what bounds the chip: the mounted surface is read-only,
/// and the reading view clamps its content column to `reading_mode::READING_COLUMN_WIDTH_PTS`
/// (720.0) and CENTRES it. So every scenario's label must fit inside 720pt or the galley wraps —
/// `chip_rect_for_span` then unions both rows into one tall pill and the single-row pixel geometry
/// this proof measures becomes meaningless. Measured on this host: a 46-character label at style
/// size 36.0 wrapped (pill height 88px for a 36pt row) and the chip's own `painter.text` call, which
/// never wraps, ran 325px past the column. The pill-height guard below catches that class of
/// mis-measurement; the scenario sizes are chosen to stay inside the column. The generous frame
/// WIDTH exists so there is a large, uniformly-painted empty region right of the column for the
/// overflow scan and the background-reference strip.
const HARNESS_SIZE: egui::Vec2 = egui::vec2(2400.0, 240.0);

/// Per-channel tolerance when matching a rendered pixel to a palette token. The wgpu path converts
/// sRGB -> linear -> blend -> sRGB, so an exact byte match is not guaranteed.
const COLOR_TOL: i32 = 10;

/// The ink box must span at least this fraction of the pill width. With the fix the label advance
/// equals the galley advance the pill was measured from (minus the pill's 1px padding per side and
/// the glyphs' own side bearings). With the 15.0 constant restored at the paint site a 24.0 pill is
/// filled ~0.63 and a 36.0 heading pill ~0.42.
const MIN_FILL_RATIO: f32 = 0.90;

/// Columns of the pill row band, at the far right of the frame, sampled as the empty-background
/// reference. Overflow ink is anything in the band that differs from this reference.
const BG_REFERENCE_COLS: u32 = 40;

/// Ignore this many columns immediately right of the pill when scanning for overflow, absorbing the
/// sub-pixel rounding between the pill rect's right edge and the last matched pill pixel. The pill's
/// own antialiased corner ring is already excluded by [`is_label_ink`], so this stays small.
const PILL_EDGE_GUARD: u32 = 2;

// ── artifact placement (CX-212E: external root only, never repo-local) ───────────────────────────

fn artifact_dir() -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join("wp-kernel-012-mt-116")
}

// ── scenario model ───────────────────────────────────────────────────────────────────────────────

/// One chip variant rendered at one editor font size in one block kind.
struct Scenario {
    /// Stable id used for the artifact filenames and the printed evidence rows.
    id: &'static str,
    /// Human description of the chip variant under AC-116-4.
    variant: &'static str,
    /// The live editor font size (the Settings `settings-editor-font-size` value) for the AFTER frame.
    editor_font_size: f32,
    /// `Some(level)` renders the chip inside a heading block (HEADING_SCALE applies); `None` = paragraph.
    heading_level: Option<u8>,
    /// The chip atom.
    link: HsLinkNode,
    /// The 44-character id under test (for the evidence row).
    id_under_test: String,
    /// The mismatch direction the scenario is designed to catch if the fix is reverted.
    reverted_signature: &'static str,
}

impl Scenario {
    /// The block the chip is rendered in: a block whose ONLY child is the hsLink atom, so the frame
    /// contains exactly one pill and the region right of it is empty background.
    fn block(&self) -> BlockNode {
        let kind = match self.heading_level {
            Some(level) => NodeKind::Heading(
                handshake_native::rich_editor::document_model::node::HeadingLevel::new(level),
            ),
            None => NodeKind::Paragraph,
        };
        BlockNode::with_children(kind, vec![Child::HsLink(self.link.clone())])
    }

    /// The style size the galley (and therefore the pill) is measured at — the SAME helper the
    /// product's spec site reads.
    fn resolved_style_size(&self) -> f32 {
        block_style_with_base(&self.block(), self.editor_font_size).size
    }

    /// The pill background token for this chip variant (`chip_colors`: resolved -> accent_soft,
    /// unresolved -> error_bg).
    fn pill_bg(&self, palette: &HsPalette) -> Color32 {
        if self.link.resolved {
            palette.accent_soft
        } else {
            palette.error_bg
        }
    }

    /// The chip LABEL colour token for this variant (`chip_colors`: resolved -> accent,
    /// unresolved -> error_text). Glyph ink is classified against this.
    fn label_fg(&self, palette: &HsPalette) -> Color32 {
        if self.link.resolved {
            palette.accent
        } else {
            palette.error_text
        }
    }
}

/// A 44-character id built from `prefix` + a filler tail. Uppercase + digits guarantee full
/// cap-height ink for the glyph-height measurement.
fn id_of_len_44(prefix: &str) -> String {
    let mut id = prefix.to_owned();
    let filler = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut i = 0;
    while id.chars().count() < MT116_ID_LEN {
        let ch = filler.as_bytes()[i % filler.len()] as char;
        id.push(ch);
        i += 1;
    }
    let id: String = id.chars().take(MT116_ID_LEN).collect();
    assert_eq!(
        id.chars().count(),
        MT116_ID_LEN,
        "the MT-116 contract's id length"
    );
    id
}

/// A 44-character id whose TAIL is `-MISSING`, matching the MT-068 red unresolved chip the contract
/// evidence names.
fn missing_id_of_len_44() -> String {
    let suffix = "-MISSING";
    let head = id_of_len_44("WP-MT116-UNRESOLVED-");
    let keep = MT116_ID_LEN - suffix.chars().count();
    let id: String = head.chars().take(keep).chain(suffix.chars()).collect();
    assert_eq!(id.chars().count(), MT116_ID_LEN);
    assert!(id.ends_with("-MISSING"));
    id
}

fn scenarios() -> Vec<Scenario> {
    // AC-116-4 variant 1: RESOLVED chip (accent pill), paragraph, editor font ABOVE the 15.0 constant.
    let resolved_id = id_of_len_44("WP-KERNEL-012-MT116-RESOLVED-");
    let mut resolved = HsLinkNode::new("wp", resolved_id.clone(), resolved_id.clone());
    resolved.resolved = true;

    // AC-116-4 variant 2: UNRESOLVED `-MISSING` locus chip (error pill), paragraph, editor font BELOW
    // the constant. A locus ref is used deliberately: it is the exact chip the MT-116 evidence frame
    // (`mt068-locus-work-packet-before.png`) shows as the red `-MISSING` pill.
    let missing_id = missing_id_of_len_44();
    let mut missing = HsLinkNode::new(
        LOCUS_REF_KIND,
        format!("locus://wp/{missing_id}"),
        String::new(),
    );
    missing.resolved = false;

    // AC-116-4 variant 3: CODE-REF chip inside an H1 — the case the constant missed ENTIRELY, because
    // `block_style_with_base` multiplies a heading by HEADING_SCALE. No `#`/`::` in the ref value, so
    // `code_ref_short_name` keeps the whole 44-character id visible.
    let code_id = id_of_len_44("MT116CODEREFSYMBOL");
    let mut code_ref = HsLinkNode::new(CODE_REF_KIND, code_id.clone(), code_id.clone());
    code_ref.resolved = true;

    vec![
        Scenario {
            id: "above-default-resolved",
            variant: "resolved wikilink chip (accent pill)",
            editor_font_size: 24.0,
            heading_level: None,
            link: resolved,
            id_under_test: resolved_id,
            reverted_signature: "UNDER-FILL: pill measured at 24.0, label painted at the 15.0 constant",
        },
        Scenario {
            id: "below-default-unresolved-missing",
            variant: "unresolved -MISSING locus chip (error pill)",
            editor_font_size: 11.0,
            heading_level: None,
            link: missing,
            id_under_test: missing_id,
            reverted_signature: "OVERFLOW: pill measured at 11.0, label painted at the 15.0 constant",
        },
        Scenario {
            id: "heading-h1-code-ref",
            variant: "code-ref chip",
            // 12.0 is BELOW the constant while the H1 style size (12.0 * 1.8 = 21.6) is ABOVE it.
            // That combination is deliberate: it proves the painted size comes from
            // `block_style_with_base`, not from `editor_font_size` directly. A larger editor size
            // would push the 46-character label past the 720pt reading column and wrap it.
            editor_font_size: 12.0,
            heading_level: Some(1),
            link: code_ref,
            id_under_test: code_id,
            reverted_signature:
                "UNDER-FILL: H1 pill measured at 12.0*1.8=21.6, label painted at the 15.0 constant",
        },
    ]
}

// ── pixel analysis ───────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Box2 {
    min_x: u32,
    min_y: u32,
    max_x: u32,
    max_y: u32,
}

impl Box2 {
    fn width(&self) -> u32 {
        self.max_x - self.min_x + 1
    }
    fn height(&self) -> u32 {
        self.max_y - self.min_y + 1
    }
}

fn near(a: [u8; 4], b: [u8; 4], tol: i32) -> bool {
    (0..3).all(|c| (i32::from(a[c]) - i32::from(b[c])).abs() <= tol)
}

fn rgb_dist2(a: [u8; 4], b: [u8; 4]) -> i64 {
    (0..3)
        .map(|c| {
            let d = i64::from(a[c]) - i64::from(b[c]);
            d * d
        })
        .sum()
}

/// A pixel counts as painted LABEL INK when it is closer to the chip's text token than to EITHER
/// the pill fill or the page background.
///
/// This is the classification that makes the proof honest. A first run of this file classified "not
/// the pill token and not the background token" as ink; that swept in the pill's own antialiased
/// rounded-corner ring, which sits on the pill<->background blend line. The ink box then equalled the
/// pill box exactly (fill_ratio 1.000 in every scenario — a vacuous check) and the AA fringe three
/// pixels past the pill read as overflow. Blends between the two BACKGROUND tokens are never nearest
/// to the (much darker, saturated) text token, so this test keeps glyph ink and drops pill edges.
fn is_label_ink(px: [u8; 4], fg: [u8; 4], pill: [u8; 4], bg: [u8; 4]) -> bool {
    let d_fg = rgb_dist2(px, fg);
    d_fg < rgb_dist2(px, pill) && d_fg < rgb_dist2(px, bg)
}

fn token_rgba(color: Color32) -> [u8; 4] {
    let [r, g, b, a] = color.to_array();
    [r, g, b, a]
}

/// The largest 4-connected run of pixels matching `target`. Returns its bounding box + pixel count.
fn largest_component(image: &image::RgbaImage, target: [u8; 4]) -> Option<(Box2, usize)> {
    let (w, h) = (image.width(), image.height());
    let mut seen = vec![false; (w as usize) * (h as usize)];
    let matches = |x: u32, y: u32| near(image.get_pixel(x, y).0, target, COLOR_TOL);
    let mut best: Option<(Box2, usize)> = None;
    for y0 in 0..h {
        for x0 in 0..w {
            let idx = (y0 as usize) * (w as usize) + (x0 as usize);
            if seen[idx] || !matches(x0, y0) {
                continue;
            }
            let mut stack = vec![(x0, y0)];
            seen[idx] = true;
            let mut bbox = Box2 {
                min_x: x0,
                min_y: y0,
                max_x: x0,
                max_y: y0,
            };
            let mut count = 0usize;
            while let Some((x, y)) = stack.pop() {
                count += 1;
                bbox.min_x = bbox.min_x.min(x);
                bbox.min_y = bbox.min_y.min(y);
                bbox.max_x = bbox.max_x.max(x);
                bbox.max_y = bbox.max_y.max(y);
                let push = |nx: u32, ny: u32, stack: &mut Vec<(u32, u32)>, seen: &mut Vec<bool>| {
                    let nidx = (ny as usize) * (w as usize) + (nx as usize);
                    if !seen[nidx] && matches(nx, ny) {
                        seen[nidx] = true;
                        stack.push((nx, ny));
                    }
                };
                if x > 0 {
                    push(x - 1, y, &mut stack, &mut seen);
                }
                if x + 1 < w {
                    push(x + 1, y, &mut stack, &mut seen);
                }
                if y > 0 {
                    push(x, y - 1, &mut stack, &mut seen);
                }
                if y + 1 < h {
                    push(x, y + 1, &mut stack, &mut seen);
                }
            }
            if best.as_ref().is_none_or(|(_, c)| count > *c) {
                best = Some((bbox, count));
            }
        }
    }
    best
}

/// The modal (most frequent) colour in a rectangular region — used as the empty-background reference.
fn modal_color(image: &image::RgbaImage, region: Box2) -> [u8; 4] {
    let mut counts: HashMap<[u8; 4], usize> = HashMap::new();
    for y in region.min_y..=region.max_y {
        for x in region.min_x..=region.max_x {
            *counts.entry(image.get_pixel(x, y).0).or_insert(0) += 1;
        }
    }
    counts
        .into_iter()
        .max_by_key(|(_, c)| *c)
        .map(|(color, _)| color)
        .expect("a non-empty background reference region")
}

#[derive(Debug)]
struct ChipMeasurement {
    pill: Box2,
    pill_px: usize,
    ink: Box2,
    ink_px: usize,
    /// Rows of the pill band that carry real glyph ink (see [`measure_chip`]).
    glyph_rows: usize,
    background: [u8; 4],
    overflow_px: usize,
    overflow_max_x: Option<u32>,
    fill_ratio: f32,
    /// The longest run of consecutive columns INSIDE the pill that carry no glyph ink at all — the
    /// width of the largest empty gap in the pill.
    max_gap: u32,
    max_gap_at_x: Option<u32>,
}

/// Measure one chip in a captured frame: locate the pill by its palette token, then classify every
/// pixel in the pill's row band as pill / background / INK.
fn measure_chip(
    image: &image::RgbaImage,
    pill_bg: Color32,
    label_fg: Color32,
) -> Result<ChipMeasurement, String> {
    let (w, _h) = (image.width(), image.height());
    let target = token_rgba(pill_bg);
    let (pill, pill_px) = largest_component(image, target)
        .ok_or_else(|| format!("no pixels matched the chip pill token {target:?}"))?;
    if pill.width() < 10 || pill.height() < 4 {
        return Err(format!(
            "the located pill is implausibly small: {pill:?} ({pill_px} px)"
        ));
    }
    if pill.max_x + PILL_EDGE_GUARD + BG_REFERENCE_COLS + 2 >= w {
        return Err(format!(
            "the pill reaches the frame edge ({pill:?}, frame width {w}); widen HARNESS_SIZE so the \
             label cannot be clipped and an empty background reference strip exists"
        ));
    }
    // Empty-background reference: the far-right columns of the pill's own row band.
    let background = modal_color(
        image,
        Box2 {
            min_x: w - BG_REFERENCE_COLS,
            min_y: pill.min_y,
            max_x: w - 1,
            max_y: pill.max_y,
        },
    );
    if near(background, target, COLOR_TOL) {
        return Err(
            "the background reference strip matches the pill token; pixel classification would be \
             ambiguous"
                .to_owned(),
        );
    }

    let fg = token_rgba(label_fg);
    if is_label_ink(background, fg, target, background) || is_label_ink(target, fg, target, background)
    {
        return Err(format!(
            "the chip text token {fg:?} is not separable from the pill {target:?} / background \
             {background:?} tokens"
        ));
    }

    // INK inside the pill: pixels whose nearest token is the chip TEXT colour, restricted to rows
    // that carry REAL GLYPH ink.
    //
    // The row filter matters. The hsLink atom's own galley run paints a link underline whose extent
    // follows the GALLEY (i.e. the pill), not the chip label. Measured on the reverted-fix build, the
    // 24pt scenario's glyph rows held 109-185 ink pixels spanning x=840..1244 while a single row at
    // the pill's bottom held THREE ink pixels spanning x=840..1488. Taking a naive bounding box over
    // every ink pixel therefore reported ink_w == pill_w and a 0.998 fill ratio for a chip whose pill
    // was visibly two-fifths empty — the fill assertion was vacuous. A row counts as a glyph row only
    // when it holds enough ink to be text (never a stray underline remnant) and not so much that it
    // spans the pill as a solid rule.
    let min_row_ink = std::cmp::max(6, pill.width() / 60) as usize;
    let max_row_ink = (pill.width() as f32 * 0.7) as usize;
    let row_ink: Vec<Vec<u32>> = (pill.min_y..=pill.max_y)
        .map(|y| {
            (pill.min_x..=pill.max_x)
                .filter(|&x| is_label_ink(image.get_pixel(x, y).0, fg, target, background))
                .collect()
        })
        .collect();
    let glyph_rows: Vec<(u32, &Vec<u32>)> = row_ink
        .iter()
        .enumerate()
        .map(|(i, xs)| (pill.min_y + i as u32, xs))
        .filter(|(_, xs)| xs.len() >= min_row_ink && xs.len() <= max_row_ink)
        .collect();
    if glyph_rows.is_empty() {
        return Err(format!(
            "no glyph rows found inside the pill {pill:?} (row ink counts {:?}, accepted band \
             {min_row_ink}..={max_row_ink}); the chip painted no label",
            row_ink.iter().map(Vec::len).collect::<Vec<_>>()
        ));
    }
    let mut ink = Box2 {
        min_x: u32::MAX,
        min_y: u32::MAX,
        max_x: 0,
        max_y: 0,
    };
    let mut ink_px = 0usize;
    let mut inked_col = vec![false; pill.width() as usize];
    for (y, xs) in &glyph_rows {
        ink_px += xs.len();
        ink.min_y = ink.min_y.min(*y);
        ink.max_y = ink.max_y.max(*y);
        for &x in xs.iter() {
            ink.min_x = ink.min_x.min(x);
            ink.max_x = ink.max_x.max(x);
            inked_col[(x - pill.min_x) as usize] = true;
        }
    }

    // The largest EMPTY horizontal gap inside the pill. This is the metric that survives a stray
    // full-width mark: a label painted at a smaller metric than its pill leaves one long ink-free
    // run at the pill's tail (242px on the reverted-fix 24pt build; 4px with the fix in place).
    let mut max_gap = 0u32;
    let mut max_gap_at_x = None;
    let mut run = 0u32;
    let mut run_start = 0u32;
    for (i, inked) in inked_col.iter().enumerate() {
        if *inked {
            run = 0;
            continue;
        }
        if run == 0 {
            run_start = pill.min_x + i as u32;
        }
        run += 1;
        if run > max_gap {
            max_gap = run;
            max_gap_at_x = Some(run_start);
        }
    }

    // OVERFLOW: any non-background pixel in the pill's row band, right of the pill (past the pill's
    // own antialiased corner ring) and left of the background reference strip.
    let scan_from = pill.max_x + PILL_EDGE_GUARD;
    let scan_to = w - BG_REFERENCE_COLS - 1;
    let mut overflow_px = 0usize;
    let mut overflow_max_x = None;
    for y in pill.min_y..=pill.max_y {
        for x in scan_from..=scan_to {
            let px = image.get_pixel(x, y).0;
            if !is_label_ink(px, fg, target, background) {
                continue;
            }
            overflow_px += 1;
            overflow_max_x = Some(overflow_max_x.map_or(x, |m: u32| m.max(x)));
        }
    }

    let fill_ratio = ink.width() as f32 / pill.width() as f32;
    Ok(ChipMeasurement {
        pill,
        pill_px,
        ink,
        ink_px,
        glyph_rows: glyph_rows.len(),
        background,
        overflow_px,
        overflow_max_x,
        fill_ratio,
        max_gap,
        max_gap_at_x,
    })
}

// ── harness ──────────────────────────────────────────────────────────────────────────────────────

fn sha256_pixels(image: &image::RgbaImage) -> String {
    format!("{:x}", Sha256::digest(image.as_raw()))
}

fn save_frame(image: &image::RgbaImage, scenario_id: &str, label: &str) -> PathBuf {
    let dir = artifact_dir();
    std::fs::create_dir_all(&dir).expect("create the external MT-116 artifact dir");
    let path = dir.join(format!("mt116-{scenario_id}-{label}.png"));
    image.save(&path).expect("save the MT-116 proof frame");
    path
}

/// Crop `region` out of `image` and nearest-neighbour magnify it by `scale`.
///
/// AC-116-2 requires the frame to be inspected "at a magnification where the label is legible". A
/// full proof frame is 2400px wide and only ~30px of that height is the chip, so the raw PNG is not
/// inspectable as-is. Nearest-neighbour (not a smoothing filter) is deliberate: it magnifies without
/// inventing intermediate pixels, so a glyph edge crossing the pill boundary stays exactly where it
/// was painted.
fn save_magnified(
    image: &image::RgbaImage,
    region: Box2,
    scale: u32,
    scenario_id: &str,
    label: &str,
) -> PathBuf {
    let (w, h) = (image.width(), image.height());
    let min_x = region.min_x.min(w - 1);
    let min_y = region.min_y.min(h - 1);
    let max_x = region.max_x.min(w - 1);
    let max_y = region.max_y.min(h - 1);
    let crop_w = max_x - min_x + 1;
    let crop_h = max_y - min_y + 1;
    let out = image::RgbaImage::from_fn(crop_w * scale, crop_h * scale, |x, y| {
        *image.get_pixel(min_x + x / scale, min_y + y / scale)
    });
    let dir = artifact_dir();
    std::fs::create_dir_all(&dir).expect("create the external MT-116 artifact dir");
    let path = dir.join(format!("mt116-{scenario_id}-{label}.png"));
    out.save(&path).expect("save the MT-116 magnified crop");
    path
}

/// Emit the inspection crops for one measured chip: the whole pill at 2x, plus its HEAD and TAIL at
/// high magnification. The TAIL crop is the AC-116-2 evidence image — it shows the last characters of
/// the id together with the pill's right boundary and the empty background beyond it.
fn save_inspection_crops(
    image: &image::RgbaImage,
    scenario_id: &str,
    pill: Box2,
) -> (PathBuf, PathBuf, PathBuf) {
    let pad_y = (pill.height() / 2).max(6);
    let band = |min_x: u32, max_x: u32| Box2 {
        min_x,
        min_y: pill.min_y.saturating_sub(pad_y),
        max_x,
        max_y: pill.max_y + pad_y,
    };
    // Enough magnification that even an 11pt label reads.
    let tile_scale = (150 / pill.height().max(1)).clamp(3, 10);
    let full_scale = (72 / pill.height().max(1)).clamp(2, 4);
    let tile_w = 240u32;
    let full = save_magnified(
        image,
        band(pill.min_x.saturating_sub(24), pill.max_x + 24),
        full_scale,
        scenario_id,
        &format!("chip-full-{full_scale}x"),
    );
    let head = save_magnified(
        image,
        band(pill.min_x.saturating_sub(12), pill.min_x + tile_w),
        tile_scale,
        scenario_id,
        &format!("chip-head-{tile_scale}x"),
    );
    let tail = save_magnified(
        image,
        band(pill.max_x.saturating_sub(tile_w), pill.max_x + 48),
        tile_scale,
        scenario_id,
        &format!("chip-tail-{tile_scale}x"),
    );
    (full, head, tail)
}

/// Mount the real `RichEditorWidget` (read-only, so no caret/selection paints into the measured
/// band) over a single-chip document, in the LIGHT palette (its chip tokens are OPAQUE, so pill and
/// background are unambiguously separable in the captured pixels).
fn mount(scenario: &Scenario) -> (Arc<Mutex<RichEditorState>>, Harness<'static, ()>) {
    let mut state = RichEditorState::new(BlockNode::doc(vec![scenario.block()]));
    state.theme = HsTheme::Light;
    // Start at the DEFAULT size: the BEFORE frame of the PT-116-3 pair.
    assert!(
        !state.set_editor_font_size(BASE_FONT_SIZE) || state.editor_font_size() == BASE_FONT_SIZE,
        "the before frame starts at the default editor font size"
    );
    let state = Arc::new(Mutex::new(state));
    let state_for_ui = Arc::clone(&state);
    let harness = Harness::builder()
        .proof_mt_id("MT-116")
        .with_size(HARNESS_SIZE)
        .build_ui(move |ui| {
            handshake_native::app::HandshakeApp::install_fonts(ui.ctx());
            RichEditorWidget::new_read_only(Arc::clone(&state_for_ui)).show(ui);
        });
    (state, harness)
}

// ── the proof ────────────────────────────────────────────────────────────────────────────────────

#[test]
fn mt116_chip_label_is_contained_in_and_fills_its_pill_at_non_default_font_sizes() {
    if !gpu_screenshot_enabled() {
        // Fail closed on the harness contract rather than passing vacuously: prove the typed
        // DEFERRED outcome was durably recorded, and say so loudly.
        let scenario = &scenarios()[0];
        let (_state, mut harness) = mount(scenario);
        harness.run();
        let frame = harness.render_settled_proof_frame("MT-116 headless deferral");
        assert!(
            frame.is_none(),
            "a headless run must not return pixels; set HANDSHAKE_GPU_SCREENSHOT=1 on a real-GPU host"
        );
        let outcome = harness
            .last_screenshot_outcome()
            .expect("the harness records a typed screenshot outcome");
        assert_eq!(
            outcome.status, "DEFERRED",
            "a headless run must record a typed DEFERRED outcome, never a silent pass"
        );
        eprintln!(
            "MT116_PROOF_STATUS=DEFERRED reason=HANDSHAKE_GPU_SCREENSHOT unset; NO pixel assertion ran"
        );
        return;
    }

    println!("MT116_PROOF_STATUS=GPU id_length_tested={MT116_ID_LEN}");
    println!(
        "MT116_CONSTANT_UNDER_TEST base_font_size={BASE_FONT_SIZE} heading_scale={HEADING_SCALE:?}"
    );

    let mut checked = 0usize;
    let mut before_hashes: Vec<String> = Vec::new();
    let mut after_hashes: Vec<String> = Vec::new();
    let mut scenario_failures: Vec<String> = Vec::new();

    for scenario in scenarios() {
        let style_size = scenario.resolved_style_size();
        let label = handshake_native::rich_editor::wikilinks::inline_view::chip_label(&scenario.link);
        println!(
            "\nMT116_SCENARIO id={} variant=\"{}\" editor_font_size={} resolved_style_size={} \
             id_len={} label_len={} label=\"{}\"",
            scenario.id,
            scenario.variant,
            scenario.editor_font_size,
            style_size,
            scenario.id_under_test.chars().count(),
            label.chars().count(),
            label
        );
        assert_eq!(
            scenario.id_under_test.chars().count(),
            MT116_ID_LEN,
            "every MT-116 scenario renders the contract's 44-character id"
        );
        assert!(
            (style_size - BASE_FONT_SIZE).abs() > 1.0,
            "scenario {} must differ from the {BASE_FONT_SIZE} constant or it proves nothing \
             (resolved style size {style_size})",
            scenario.id
        );

        let (state, mut harness) = mount(&scenario);
        harness.run();

        // ── PT-116-3 pair, BEFORE: the default 15.0 editor size ──────────────────────────────────
        let before = harness
            .render_settled_proof_frame("MT-116 before frame at the default editor font size")
            .expect("a GPU run returns pixels");
        let before_sha = sha256_pixels(&before);
        let before_path = save_frame(&before, scenario.id, "before-default-15");

        // Mutate the LIVE state exactly as the Settings `settings-editor-font-size` control does.
        {
            let mut guard = state.lock().expect("state lock");
            assert!(
                guard.set_editor_font_size(scenario.editor_font_size),
                "the scenario size must actually change the live editor font size"
            );
            assert_eq!(guard.editor_font_size(), scenario.editor_font_size);
        }

        // ── PT-116-3 pair, AFTER: settled, never `render_proof_frame` ────────────────────────────
        let after = harness
            .render_settled_proof_frame("MT-116 after frame at the scenario editor font size")
            .expect("a GPU run returns pixels");
        let after_sha = sha256_pixels(&after);
        let after_path = save_frame(&after, scenario.id, "after-non-default");

        println!(
            "MT116_FRAMES scenario={} before={} sha256={} after={} sha256={}",
            scenario.id,
            before_path.display(),
            before_sha,
            after_path.display(),
            after_sha
        );
        assert_ne!(
            before_sha, after_sha,
            "PT-116-3: the before/after frames for {} are BYTE-IDENTICAL — the capture reproduced \
             the preceding painted frame and cannot evidence the non-default font size",
            scenario.id
        );
        before_hashes.push(before_sha);
        after_hashes.push(after_sha);

        // ── the painted-output assertions (the guard the unit test could not make) ───────────────
        let palette = HsTheme::Light.palette();
        let pill_bg = scenario.pill_bg(&palette);
        let label_fg = scenario.label_fg(&palette);
        let m = measure_chip(&after, pill_bg, label_fg)
            .unwrap_or_else(|error| panic!("scenario {}: {error}", scenario.id));

        println!(
            "MT116_MEASURED scenario={} pill=[x {}..{} y {}..{}] pill_w={} pill_h={} pill_px={} \
             ink=[x {}..{} y {}..{}] ink_w={} ink_h={} ink_px={} fill_ratio={:.3} bg={:?} \
             overflow_px={} overflow_max_x={:?}",
            scenario.id,
            m.pill.min_x,
            m.pill.max_x,
            m.pill.min_y,
            m.pill.max_y,
            m.pill.width(),
            m.pill.height(),
            m.pill_px,
            m.ink.min_x,
            m.ink.max_x,
            m.ink.min_y,
            m.ink.max_y,
            m.ink.width(),
            m.ink.height(),
            m.ink_px,
            m.fill_ratio,
            m.background,
            m.overflow_px,
            m.overflow_max_x
        );
        println!(
            "MT116_MEASURED2 scenario={} glyph_rows={} max_gap={} max_gap_at_x={:?}",
            scenario.id, m.glyph_rows, m.max_gap, m.max_gap_at_x
        );

        // Emit the inspection crops BEFORE asserting, so a RED run also leaves behind the magnified
        // images that show WHY it is red.
        let (full_crop, head_crop, tail_crop) =
            save_inspection_crops(&after, scenario.id, m.pill);
        println!(
            "MT116_INSPECTION scenario={} full={} head={} tail={}",
            scenario.id,
            full_crop.display(),
            head_crop.display(),
            tail_crop.display()
        );

        // Sanity: enough glyph ink was classified that the ink box is a real label, not a stray pixel.
        assert!(
            m.ink_px >= 200,
            "scenario {}: only {} label-ink pixels were classified inside the pill; the measurement \
             is not a 44-character label",
            scenario.id,
            m.ink_px
        );

        // Sanity: the component we found really is a one-row pill at the block's style size.
        assert!(
            (m.pill.height() as f32) >= style_size * 0.9
                && (m.pill.height() as f32) <= style_size * 2.2,
            "scenario {}: the located pill height {} is not ONE galley row at style size \
             {style_size}. A taller pill means the atom WRAPPED and `chip_rect_for_span` unioned two \
             rows; widen HARNESS_SIZE (currently {}pt) instead of relaxing this bound, or the \
             containment/fill measurements below are meaningless",
            scenario.id,
            m.pill.height(),
            HARNESS_SIZE.x
        );

        // ── the four acceptance checks ───────────────────────────────────────────────────────────
        //
        // These ACCUMULATE rather than panic on the first miss, so ONE run reports the verdict for
        // EVERY scenario and every chip variant. A `#[test]` that aborts on the first failing
        // scenario cannot show that scenarios 2 and 3 are independently non-vacuous.
        let mut failures: Vec<String> = Vec::new();

        // AC-116-1 / AC-116-2 CONTAINMENT — the below-default-size signature of the bug.
        if m.overflow_px != 0 {
            failures.push(format!(
                "CONTAINMENT: {} label pixels paint OUTSIDE the pill, rightmost at x={:?} while the \
                 pill ends at x={}. The label is measured by a different font metric than the pill.",
                m.overflow_px, m.overflow_max_x, m.pill.max_x
            ));
        }
        if m.ink.min_y < m.pill.min_y || m.ink.max_y > m.pill.max_y {
            failures.push(format!(
                "CONTAINMENT: the label ink box {:?} escapes its pill {:?} vertically",
                m.ink, m.pill
            ));
        }

        // AC-116-1 FILL — the above-default-size and heading signature of the bug.
        if m.fill_ratio < MIN_FILL_RATIO {
            failures.push(format!(
                "FILL: the label spans only {:.1}% of its pill (ink_w={} pill_w={}); the pill is \
                 measured at style size {style_size} while the label is painted at a smaller metric",
                m.fill_ratio * 100.0,
                m.ink.width(),
                m.pill.width()
            ));
        }

        // The largest empty run inside the pill. Independent of the ink bounding box, so a stray
        // full-width mark inside the pill cannot mask an unfilled tail.
        let max_allowed_gap = style_size * 1.5;
        if m.max_gap as f32 > max_allowed_gap {
            failures.push(format!(
                "GAP: the pill contains a {}px run of columns with NO label ink starting at x={:?}; \
                 at style size {style_size} no gap wider than {max_allowed_gap:.1}px (about one \
                 character advance) can be label spacing",
                m.max_gap, m.max_gap_at_x
            ));
        }

        // The painted glyph height must track the block's resolved style size, not the constant.
        let min_ink_h = style_size * 0.5;
        if (m.ink.height() as f32) < min_ink_h {
            failures.push(format!(
                "INK HEIGHT: the painted glyph ink is only {}px tall; a label laid out at style size \
                 {style_size} must be at least {min_ink_h:.1}px tall, while a {BASE_FONT_SIZE}pt \
                 constant gives about {:.1}px",
                m.ink.height(),
                BASE_FONT_SIZE * 0.72
            ));
        }

        if failures.is_empty() {
            println!("MT116_VERDICT scenario={} result=CONTAINED_AND_FILLED", scenario.id);
        } else {
            println!(
                "MT116_VERDICT scenario={} result=VIOLATED variant=\"{}\" expected_reverted_signature=\"{}\"",
                scenario.id, scenario.variant, scenario.reverted_signature
            );
            for f in &failures {
                println!("MT116_VIOLATION scenario={} {f}", scenario.id);
            }
            scenario_failures.push(format!("{} -> {}", scenario.id, failures.join(" | ")));
        }

        checked += 1;
    }

    assert!(
        scenario_failures.is_empty(),
        "MT-116 painted-output proof FAILED for {} of {} scenarios:\n  {}",
        scenario_failures.len(),
        checked,
        scenario_failures.join("\n  ")
    );

    // Never let this proof pass on zero executed scenarios.
    assert_eq!(
        checked,
        scenarios().len(),
        "every MT-116 scenario must have been measured"
    );
    assert_eq!(checked, 3, "AC-116-4 requires all three chip variants");

    // Distinct scenarios must produce distinct pixels — a further guard against a stale capture
    // being reused across scenarios.
    let mut all = after_hashes.clone();
    all.sort();
    all.dedup();
    assert_eq!(
        all.len(),
        after_hashes.len(),
        "two scenarios produced byte-identical AFTER frames: {after_hashes:?}"
    );
    let mut all_before = before_hashes.clone();
    all_before.sort();
    all_before.dedup();
    assert_eq!(
        all_before.len(),
        before_hashes.len(),
        "two scenarios produced byte-identical BEFORE frames: {before_hashes:?}"
    );

    println!("\nMT116_PROOF_SCENARIOS_MEASURED={checked}");
}

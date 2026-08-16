//! WP-KERNEL-012 MT-124 — MOUNTED visual proof that a wikilink chip whose label is long enough to
//! WRAP paints its label INSIDE the reading column, with the pill behind it measured by the SAME
//! rule as the label.
//!
//! ## The defect this file asserts on (and why MT-116's guard could not)
//!
//! `paint_one_wikilink_chip` drew the chip label with ONE `painter.text` call. `painter.text` NEVER
//! wraps. The galley that MEASURES the chip DOES wrap. So when a label needs two galley rows,
//! `chip_rect_for_span` unions both rows into one tall pill while the painted text runs off in a
//! single line — past the pill, past the 720pt reading column, onto the page.
//!
//! The MT-116 pixel guard (`test_mt116_chip_pill_containment.rs`) deliberately REJECTS this
//! configuration instead of asserting on it. Its verbatim bound:
//!
//! ```text
//! "the located pill height {} is not ONE galley row at style size {style_size}. A taller pill means
//!  the atom WRAPPED and `chip_rect_for_span` unioned two rows; widen HARNESS_SIZE ... instead of
//!  relaxing this bound, or the containment/fill measurements below are meaningless"
//! ```
//!
//! This file is the AC-124-4 SIBLING that turns that rejection into an assertion: every scenario
//! here is REQUIRED to wrap (a scenario that does not wrap is a hard error, never a silent pass).
//!
//! ## What is measured (AC-124-1)
//!
//! Per-SCANLINE agreement, not per-chip agreement. For every scanline of the frame that carries real
//! glyph ink, the ink on that scanline must lie inside the pill fill on THAT SAME scanline. That is
//! the literal contract wording — "the pill drawn behind it matches the rows the label actually
//! occupies. Neither may be measured by a different rule than the other" — expressed in pixels:
//!
//! - with the single `painter.text` call, the label is painted on the middle scanlines of the union
//!   pill and runs far right of the pill's own extent on those scanlines -> RED,
//! - with a per-galley-row pill + per-galley-row label segment, every ink scanline sits inside the
//!   pill fill on that scanline -> GREEN.
//!
//! Two near-vacuous traps MT-116 hit and documented are avoided here the same way:
//!
//! - the pill's antialiased rounded-corner ring is not label ink ([`is_label_ink`] classifies a pixel
//!   by NEAREST token, so a pill<->background blend is never nearest to the saturated text token),
//! - a 3-pixel underline remnant cannot hold the ink box open, because a scanline only counts as a
//!   GLYPH scanline when its ink forms at least [`MIN_INK_RUNS`] separate runs (letters), which a
//!   solid rule never does.
//!
//! ## AC-124-5 (headings at the DEFAULT editor size)
//!
//! `HEADING_SCALE` means an H1 is already 27.0 at the DEFAULT editor size 15.0, so heading chips are
//! off-default there. Scenario `h1-default-editor-size` renders at exactly `BASE_FONT_SIZE`.
//!
//! ## PT-124-3 (the `render_proof_frame` preceding-frame hazard, recorded as PT-116-3)
//!
//! Each scenario captures a BEFORE frame at a SMALL editor size where the label does NOT wrap, then
//! mutates the live state to the scenario size and captures the AFTER frame through
//! `render_settled_proof_frame` (never `render_proof_frame`, which in this WP has returned the
//! PRECEDING painted frame). Both raw-pixel sha256 are printed and asserted DIFFERENT.
//!
//! ## Running it
//!
//! ```text
//! HANDSHAKE_GPU_SCREENSHOT=1 cargo test --manifest-path src/frontend/handshake_native/Cargo.toml \
//!     --test test_mt124_chip_label_wrap -- --nocapture
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

use handshake_native::interop::locus_interop::LOCUS_REF_KIND;
use handshake_native::rich_editor::document_model::node::{BlockNode, Child, HsLinkNode, NodeKind};
use handshake_native::rich_editor::reading_mode::READING_COLUMN_WIDTH_PTS;
use handshake_native::rich_editor::renderer::line_layout::{block_style_with_base, BASE_FONT_SIZE};
use handshake_native::rich_editor::renderer::rich_editor_widget::{RichEditorState, RichEditorWidget};
use handshake_native::theme::{HsPalette, HsTheme};

/// Harness size (points). Deliberately MUCH wider than the 720pt reading column: the chip is clamped
/// to the centred column, so everything right of the column is empty page. A label that escapes the
/// column paints into that empty region, where it is unambiguously measurable, and the far-right
/// strip supplies the empty-background reference token.
const HARNESS_SIZE: egui::Vec2 = egui::vec2(2400.0, 420.0);

/// The editor font size every scenario's BEFORE frame is captured at. Small enough that no scenario
/// label wraps there, so the BEFORE/AFTER pair genuinely differs (PT-124-3) and the AFTER frame is
/// the only wrapping one.
const PT124_BEFORE_FONT_SIZE: f32 = 7.0;

/// Per-channel tolerance when matching a rendered pixel to a palette token (wgpu does
/// sRGB -> linear -> blend -> sRGB, so exact byte equality is not guaranteed).
const COLOR_TOL: i32 = 10;

/// A scanline counts as a GLYPH scanline only when its ink forms at least this many separate runs.
/// Real text is many short runs; the hsLink galley run's underline remnant is ONE long run. This is
/// what stops the underline from holding the ink box full-width (the MT-116 vacuity trap).
const MIN_INK_RUNS: usize = 3;

/// Minimum ink pixels on a scanline before it is considered at all.
const MIN_INK_PIXELS_PER_SCANLINE: usize = 6;

/// Columns of pill fill a scanline's ink may sit outside before it counts as an escape. Absorbs the
/// sub-pixel rounding between the pill rect edge and the last matched pill pixel, plus the glyphs'
/// own side bearings. The real defect overshoots by hundreds of pixels.
const PILL_EDGE_GUARD: u32 = 3;

/// Slack (pixels) beyond the computed reading-column right edge before ink counts as escaping the
/// column. The column edge is computed as an UPPER bound (a scrollbar only shifts the centred column
/// LEFT), so this never produces a false RED.
const COLUMN_SLACK_PX: u32 = 12;

/// Columns at the far right of the frame sampled as the empty-background reference.
const BG_REFERENCE_COLS: u32 = 40;

/// The label ink on a wrapped row must span at least this fraction of the pill fill on the same
/// scanline. Catches a pill measured by a different rule than the label in the UNDER-fill direction.
const MIN_FILL_RATIO: f32 = 0.80;

// ── artifact placement (CX-212E: external root only, never repo-local) ───────────────────────────

fn artifact_dir() -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join("wp-kernel-012-mt-124")
}

// ── scenario model ───────────────────────────────────────────────────────────────────────────────

/// One chip variant rendered at one editor font size in one block kind, chosen so the label WRAPS.
struct Scenario {
    /// Stable id used for artifact filenames and the printed evidence rows.
    id: &'static str,
    /// Human description of the chip variant.
    variant: &'static str,
    /// The live editor font size (the Settings `settings-editor-font-size` value) for the AFTER frame.
    editor_font_size: f32,
    /// `Some(level)` renders the chip inside a heading block (HEADING_SCALE applies); `None` = paragraph.
    heading_level: Option<u8>,
    /// The chip atom.
    link: HsLinkNode,
    /// The id under test (for the evidence row).
    id_under_test: String,
    /// What a build WITHOUT the wrapping fix does to this scenario.
    unfixed_signature: &'static str,
}

impl Scenario {
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

    fn pill_bg(&self, palette: &HsPalette) -> Color32 {
        if self.link.resolved {
            palette.accent_soft
        } else {
            palette.error_bg
        }
    }

    fn label_fg(&self, palette: &HsPalette) -> Color32 {
        if self.link.resolved {
            palette.accent
        } else {
            palette.error_text
        }
    }
}

/// A hyphen-segmented id of `len` characters. Hyphens are real row-break candidates in epaint, so a
/// realistic work-unit id wraps exactly the way an operator's document does — this is not a
/// synthetic `break_anywhere` construction.
fn segmented_id(prefix: &str, len: usize) -> String {
    let mut id = prefix.to_owned();
    let filler = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut i = 0usize;
    while id.chars().count() < len {
        if id.chars().count() % 6 == 5 {
            id.push('-');
        } else {
            id.push(filler.as_bytes()[i % filler.len()] as char);
            i += 1;
        }
    }
    let id: String = id.chars().take(len).collect();
    assert_eq!(id.chars().count(), len);
    id
}

fn scenarios() -> Vec<Scenario> {
    // AC-124-5: an H1 at the DEFAULT editor size. HEADING_SCALE 1.8 makes the style size 27.0, so a
    // heading chip is ALREADY off-default at 15.0 — the paragraph-only "no behavioural change at the
    // default size" claim from the MT-116 record does not hold for headings.
    let h1_default_id = segmented_id("WP-KERNEL-012-MT124-DEFAULT-H1-", 96);
    let mut h1_default = HsLinkNode::new("wp", h1_default_id.clone(), h1_default_id.clone());
    h1_default.resolved = true;

    // The exact configuration the MT-124 contract MEASURED: editor font 20.0, H1, a long label. At
    // style size 36.0 the contract recorded a single 88px-tall pill with the label running 325px past
    // the 720pt column.
    let h1_20_id = segmented_id("WP-KERNEL-012-MT124-H1-20PT-", 46);
    let mut h1_20 = HsLinkNode::new("wp", h1_20_id.clone(), h1_20_id.clone());
    h1_20.resolved = true;

    // A PARAGRAPH (not a heading) with an UNRESOLVED locus chip: proves the defect and the fix are
    // not heading-specific, and exercises the error-token pill of the MT-068 chip surface.
    let para_id = segmented_id("MT124-PARA-UNRESOLVED-", 88);
    let mut para = HsLinkNode::new(
        LOCUS_REF_KIND,
        format!("locus://wp/{para_id}"),
        String::new(),
    );
    para.resolved = false;

    vec![
        Scenario {
            id: "h1-default-editor-size",
            variant: "resolved wikilink chip in an H1 (accent pill)",
            editor_font_size: BASE_FONT_SIZE,
            heading_level: Some(1),
            link: h1_default,
            id_under_test: h1_default_id,
            unfixed_signature:
                "single-line painter.text at style size 27.0 runs past the union pill and the column",
        },
        Scenario {
            id: "h1-editor-20-contract-case",
            variant: "resolved wikilink chip in an H1 (accent pill)",
            editor_font_size: 20.0,
            heading_level: Some(1),
            link: h1_20,
            id_under_test: h1_20_id,
            unfixed_signature:
                "the contract's MEASURED case: 88px union pill, label 325px past the 720pt column",
        },
        Scenario {
            id: "paragraph-unresolved-locus",
            variant: "unresolved locus chip in a paragraph (error pill)",
            editor_font_size: 24.0,
            heading_level: None,
            link: para,
            id_under_test: para_id,
            unfixed_signature: "not heading-specific: a paragraph chip overflows identically",
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

/// A pixel counts as painted LABEL INK when it is closer to the chip's text token than to EITHER the
/// pill fill or the page background. See the module header: this is what keeps the pill's own
/// antialiased corner ring out of the ink measurement.
fn is_label_ink(px: [u8; 4], fg: [u8; 4], pill: [u8; 4], bg: [u8; 4]) -> bool {
    let d_fg = rgb_dist2(px, fg);
    d_fg < rgb_dist2(px, pill) && d_fg < rgb_dist2(px, bg)
}

fn token_rgba(color: Color32) -> [u8; 4] {
    let [r, g, b, a] = color.to_array();
    [r, g, b, a]
}

/// The bounding box + pixel count of EVERY pixel matching `target`. The per-row pills a wrapped chip
/// paints touch each other vertically, so a connected-component search cannot separate them; this
/// proof works per-SCANLINE instead and only needs the overall band.
fn token_extent(image: &image::RgbaImage, target: [u8; 4]) -> Option<(Box2, usize)> {
    let (w, h) = (image.width(), image.height());
    let mut bbox: Option<Box2> = None;
    let mut count = 0usize;
    for y in 0..h {
        for x in 0..w {
            if !near(image.get_pixel(x, y).0, target, COLOR_TOL) {
                continue;
            }
            count += 1;
            bbox = Some(match bbox {
                None => Box2 {
                    min_x: x,
                    min_y: y,
                    max_x: x,
                    max_y: y,
                },
                Some(b) => Box2 {
                    min_x: b.min_x.min(x),
                    min_y: b.min_y.min(y),
                    max_x: b.max_x.max(x),
                    max_y: b.max_y.max(y),
                },
            });
        }
    }
    bbox.map(|b| (b, count))
}

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

/// One scanline of the measured band.
#[derive(Debug, Clone)]
struct Scanline {
    y: u32,
    /// Horizontal extent of PILL FILL on this scanline (`None` when the scanline has no pill).
    pill: Option<(u32, u32)>,
    /// Horizontal extent of LABEL INK on this scanline (`None` when there is none).
    ink: Option<(u32, u32)>,
    ink_px: usize,
    ink_runs: usize,
}

impl Scanline {
    /// A scanline carries real GLYPH ink (not a stray underline rule, not a single AA speck).
    fn is_glyph_scanline(&self) -> bool {
        self.ink_px >= MIN_INK_PIXELS_PER_SCANLINE && self.ink_runs >= MIN_INK_RUNS
    }
}

#[derive(Debug)]
struct WrapMeasurement {
    /// Bounding box of ALL pill-token pixels (both galley rows once the chip wraps).
    pill_band: Box2,
    pill_px: usize,
    background: [u8; 4],
    scanlines: Vec<Scanline>,
    /// Contiguous groups of glyph scanlines — one per painted label row.
    ink_bands: Vec<(u32, u32)>,
    /// Rightmost glyph-ink pixel anywhere in the band.
    ink_max_x: u32,
}

fn measure_wrapped_chip(
    image: &image::RgbaImage,
    pill_bg: Color32,
    label_fg: Color32,
) -> Result<WrapMeasurement, String> {
    let w = image.width();
    let target = token_rgba(pill_bg);
    let (pill_band, pill_px) = token_extent(image, target)
        .ok_or_else(|| format!("no pixels matched the chip pill token {target:?}"))?;
    if pill_band.width() < 10 || pill_band.height() < 4 {
        return Err(format!(
            "the located pill band is implausibly small: {pill_band:?} ({pill_px} px)"
        ));
    }
    if pill_band.max_x + BG_REFERENCE_COLS + 2 >= w {
        return Err(format!(
            "the pill band reaches the frame edge ({pill_band:?}, frame width {w}); widen \
             HARNESS_SIZE so an empty background reference strip exists"
        ));
    }

    let background = modal_color(
        image,
        Box2 {
            min_x: w - BG_REFERENCE_COLS,
            min_y: pill_band.min_y,
            max_x: w - 1,
            max_y: pill_band.max_y,
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

    let mut scanlines = Vec::new();
    let mut ink_max_x = 0u32;
    for y in pill_band.min_y..=pill_band.max_y {
        let mut pill_lo: Option<u32> = None;
        let mut pill_hi: Option<u32> = None;
        let mut ink_lo: Option<u32> = None;
        let mut ink_hi: Option<u32> = None;
        let mut ink_px = 0usize;
        let mut ink_runs = 0usize;
        let mut prev_ink = false;
        // Scan the WHOLE scanline, not just the pill band: ink outside the pill is exactly the
        // defect, so it must not be excluded by the scan window.
        for x in 0..w {
            let px = image.get_pixel(x, y).0;
            if near(px, target, COLOR_TOL) {
                pill_lo = Some(pill_lo.map_or(x, |v: u32| v.min(x)));
                pill_hi = Some(pill_hi.map_or(x, |v: u32| v.max(x)));
            }
            let is_ink = is_label_ink(px, fg, target, background);
            if is_ink {
                ink_px += 1;
                ink_lo = Some(ink_lo.map_or(x, |v: u32| v.min(x)));
                ink_hi = Some(ink_hi.map_or(x, |v: u32| v.max(x)));
                if !prev_ink {
                    ink_runs += 1;
                }
            }
            prev_ink = is_ink;
        }
        let line = Scanline {
            y,
            pill: pill_lo.zip(pill_hi),
            ink: ink_lo.zip(ink_hi),
            ink_px,
            ink_runs,
        };
        if line.is_glyph_scanline() {
            if let Some((_, hi)) = line.ink {
                ink_max_x = ink_max_x.max(hi);
            }
        }
        scanlines.push(line);
    }

    // Contiguous groups of glyph scanlines = the painted label rows.
    let mut ink_bands: Vec<(u32, u32)> = Vec::new();
    for line in &scanlines {
        if !line.is_glyph_scanline() {
            continue;
        }
        match ink_bands.last_mut() {
            Some(band) if line.y <= band.1 + 2 => band.1 = line.y,
            _ => ink_bands.push((line.y, line.y)),
        }
    }

    Ok(WrapMeasurement {
        pill_band,
        pill_px,
        background,
        scanlines,
        ink_bands,
        ink_max_x,
    })
}

// ── artifacts ────────────────────────────────────────────────────────────────────────────────────

fn sha256_pixels(image: &image::RgbaImage) -> String {
    format!("{:x}", Sha256::digest(image.as_raw()))
}

fn save_frame(image: &image::RgbaImage, scenario_id: &str, label: &str) -> PathBuf {
    let dir = artifact_dir();
    std::fs::create_dir_all(&dir).expect("create the external MT-124 artifact dir");
    let path = dir.join(format!("mt124-{scenario_id}-{label}.png"));
    image.save(&path).expect("save the MT-124 proof frame");
    path
}

/// Crop + nearest-neighbour magnify, so AC-124-2's "opened and described at a magnification where the
/// label is legible" is possible on a 2400px-wide frame. Nearest-neighbour is deliberate: it invents
/// no intermediate pixels, so a glyph edge crossing the pill boundary stays where it was painted.
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
    let out = image::RgbaImage::from_fn(
        (max_x - min_x + 1) * scale,
        (max_y - min_y + 1) * scale,
        |x, y| *image.get_pixel(min_x + x / scale, min_y + y / scale),
    );
    let dir = artifact_dir();
    std::fs::create_dir_all(&dir).expect("create the external MT-124 artifact dir");
    let path = dir.join(format!("mt124-{scenario_id}-{label}.png"));
    out.save(&path).expect("save the MT-124 magnified crop");
    path
}

/// The AC-124-2 inspection images: the whole wrapped chip, and the RIGHT EDGE band where the reading
/// column ends — the exact place a single-line label escapes to.
fn save_inspection_crops(
    image: &image::RgbaImage,
    scenario_id: &str,
    band: Box2,
    column_right_px: u32,
) -> (PathBuf, PathBuf) {
    let h = image.height();
    let pad_y = (band.height() / 4).max(8);
    let full_band = Box2 {
        min_x: 0,
        min_y: band.min_y.saturating_sub(pad_y),
        max_x: image.width() - 1,
        max_y: (band.max_y + pad_y).min(h - 1),
    };
    let full = save_magnified(image, full_band, 1, scenario_id, "wrapped-chip-full-frame");
    let edge_band = Box2 {
        min_x: column_right_px.saturating_sub(320),
        min_y: full_band.min_y,
        max_x: (column_right_px + 400).min(image.width() - 1),
        max_y: full_band.max_y,
    };
    let scale = (150 / band.height().max(1)).clamp(2, 6);
    let edge = save_magnified(
        image,
        edge_band,
        scale,
        scenario_id,
        &format!("column-right-edge-{scale}x"),
    );
    (full, edge)
}

/// Mount the real `RichEditorWidget` READ-ONLY (reading view: the 720pt centred column is what bounds
/// the chip, and no caret/selection paints into the measured band) over a single-chip document, in
/// the LIGHT palette (opaque chip tokens -> pill and background are unambiguously separable).
fn mount(scenario: &Scenario) -> (Arc<Mutex<RichEditorState>>, Harness<'static, ()>) {
    let mut state = RichEditorState::new(BlockNode::doc(vec![scenario.block()]));
    state.theme = HsTheme::Light;
    state.set_editor_font_size(PT124_BEFORE_FONT_SIZE);
    assert_eq!(
        state.editor_font_size(),
        PT124_BEFORE_FONT_SIZE,
        "the BEFORE frame starts at the non-wrapping editor font size"
    );
    let state = Arc::new(Mutex::new(state));
    let state_for_ui = Arc::clone(&state);
    let harness = Harness::builder()
        .proof_mt_id("MT-124")
        .with_size(HARNESS_SIZE)
        .build_ui(move |ui| {
            handshake_native::app::HandshakeApp::install_fonts(ui.ctx());
            RichEditorWidget::new_read_only(Arc::clone(&state_for_ui)).show(ui);
        });
    (state, harness)
}

// ── the proof ────────────────────────────────────────────────────────────────────────────────────

#[test]
fn mt124_wrapped_chip_label_stays_inside_the_reading_column_and_matches_its_pill() {
    if !gpu_screenshot_enabled() {
        let scenario = &scenarios()[0];
        let (_state, mut harness) = mount(scenario);
        harness.run();
        let frame = harness.render_settled_proof_frame("MT-124 headless deferral");
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
            "MT124_PROOF_STATUS=DEFERRED reason=HANDSHAKE_GPU_SCREENSHOT unset; NO pixel assertion ran"
        );
        return;
    }

    println!("MT124_PROOF_STATUS=GPU reading_column_pts={READING_COLUMN_WIDTH_PTS}");

    let mut checked = 0usize;
    let mut before_hashes: Vec<String> = Vec::new();
    let mut after_hashes: Vec<String> = Vec::new();
    let mut scenario_failures: Vec<String> = Vec::new();

    for scenario in scenarios() {
        let style_size = scenario.resolved_style_size();
        let label = handshake_native::rich_editor::wikilinks::inline_view::chip_label(&scenario.link);
        println!(
            "\nMT124_SCENARIO id={} variant=\"{}\" editor_font_size={} resolved_style_size={} \
             id_len={} label_len={} label=\"{}\"",
            scenario.id,
            scenario.variant,
            scenario.editor_font_size,
            style_size,
            scenario.id_under_test.chars().count(),
            label.chars().count(),
            label
        );

        let (state, mut harness) = mount(&scenario);
        harness.run();

        // ── PT-124-3 pair, BEFORE: the small NON-wrapping editor size ────────────────────────────
        let before = harness
            .render_settled_proof_frame("MT-124 before frame at the non-wrapping editor font size")
            .expect("a GPU run returns pixels");
        let before_sha = sha256_pixels(&before);
        let before_path = save_frame(&before, scenario.id, "before-nonwrapping");

        {
            let mut guard = state.lock().expect("state lock");
            assert!(
                guard.set_editor_font_size(scenario.editor_font_size),
                "the scenario size must actually change the live editor font size"
            );
            assert_eq!(guard.editor_font_size(), scenario.editor_font_size);
        }

        // ── PT-124-3 pair, AFTER: settled, never `render_proof_frame` ────────────────────────────
        let after = harness
            .render_settled_proof_frame("MT-124 after frame at the wrapping editor font size")
            .expect("a GPU run returns pixels");
        let after_sha = sha256_pixels(&after);
        let after_path = save_frame(&after, scenario.id, "after-wrapping");

        println!(
            "MT124_FRAMES scenario={} before={} sha256={} after={} sha256={}",
            scenario.id,
            before_path.display(),
            before_sha,
            after_path.display(),
            after_sha
        );
        assert_ne!(
            before_sha, after_sha,
            "PT-124-3: the before/after frames for {} are BYTE-IDENTICAL — the capture reproduced \
             the preceding painted frame and cannot evidence the wrapping configuration",
            scenario.id
        );
        before_hashes.push(before_sha);
        after_hashes.push(after_sha);

        let palette = HsTheme::Light.palette();
        let m = measure_wrapped_chip(&after, scenario.pill_bg(&palette), scenario.label_fg(&palette))
            .unwrap_or_else(|error| panic!("scenario {}: {error}", scenario.id));

        // The reading column's right edge, as an UPPER bound: the column is centred in the available
        // width, and a scrollbar can only SHRINK that width, which moves the edge LEFT.
        let ppp = after.width() as f32 / HARNESS_SIZE.x;
        let column_right_px =
            (((HARNESS_SIZE.x + READING_COLUMN_WIDTH_PTS) / 2.0) * ppp).ceil() as u32;

        println!(
            "MT124_MEASURED scenario={} pill_band=[x {}..{} y {}..{}] pill_w={} pill_h={} \
             pill_px={} bg={:?} ink_bands={:?} ink_max_x={} ppp={ppp} column_right_px={column_right_px}",
            scenario.id,
            m.pill_band.min_x,
            m.pill_band.max_x,
            m.pill_band.min_y,
            m.pill_band.max_y,
            m.pill_band.width(),
            m.pill_band.height(),
            m.pill_px,
            m.background,
            m.ink_bands,
            m.ink_max_x
        );

        let (full_crop, edge_crop) =
            save_inspection_crops(&after, scenario.id, m.pill_band, column_right_px);
        println!(
            "MT124_INSPECTION scenario={} full={} column_right_edge={}",
            scenario.id,
            full_crop.display(),
            edge_crop.display()
        );

        // ── NON-VACUITY: this scenario must actually WRAP ────────────────────────────────────────
        //
        // AC-124-4: the MT-116 guard REJECTED this configuration with a pill-height bound. Here the
        // same condition is REQUIRED. A scenario that stopped wrapping proves nothing and is a hard
        // error, never a silent pass.
        assert!(
            (m.pill_band.height() as f32) >= style_size * 1.5 * ppp,
            "scenario {}: the pill band is only {}px tall at style size {style_size} (ppp {ppp}); \
             the label did NOT wrap, so this scenario asserts nothing. Lengthen the id instead of \
             relaxing this bound.",
            scenario.id,
            m.pill_band.height()
        );
        assert!(
            m.ink_bands.len() >= 2,
            "scenario {}: only {} painted label row(s) were found ({:?}); a wrapping scenario must \
             paint at least two",
            scenario.id,
            m.ink_bands.len(),
            m.ink_bands
        );

        let mut failures: Vec<String> = Vec::new();

        // ── AC-124-1: per-SCANLINE agreement between the label and the pill behind it ────────────
        let mut escaping = 0usize;
        let mut worst: Option<(u32, u32, u32)> = None; // (y, ink_max_x, pill_max_x)
        let mut underfilled: Vec<String> = Vec::new();
        for line in m.scanlines.iter().filter(|l| l.is_glyph_scanline()) {
            let Some((ink_lo, ink_hi)) = line.ink else {
                continue;
            };
            match line.pill {
                None => {
                    escaping += 1;
                    failures.push(format!(
                        "ROW MISMATCH: scanline y={} carries {} label-ink pixels (x {}..{}) with NO \
                         pill behind it on that scanline",
                        line.y, line.ink_px, ink_lo, ink_hi
                    ));
                }
                Some((pill_lo, pill_hi)) => {
                    if ink_hi > pill_hi + PILL_EDGE_GUARD || ink_lo + PILL_EDGE_GUARD < pill_lo {
                        escaping += 1;
                        if worst.is_none_or(|(_, hi, _)| ink_hi > hi) {
                            worst = Some((line.y, ink_hi, pill_hi));
                        }
                    }
                    let pill_w = (pill_hi - pill_lo + 1) as f32;
                    let ink_w = (ink_hi.min(pill_hi) - ink_lo.max(pill_lo) + 1) as f32;
                    if ink_w / pill_w < MIN_FILL_RATIO && underfilled.len() < 4 {
                        underfilled.push(format!(
                            "y={} ink[{ink_lo}..{ink_hi}] pill[{pill_lo}..{pill_hi}] fill={:.3}",
                            line.y,
                            ink_w / pill_w
                        ));
                    }
                }
            }
        }
        if escaping > 0 {
            failures.push(format!(
                "ROW MISMATCH: {escaping} glyph scanline(s) carry label ink OUTSIDE the pill fill on \
                 that same scanline; worst {worst:?} (y, ink_max_x, pill_max_x). The label and the \
                 pill are measured by different rules — the label does not wrap with its galley."
            ));
        }
        if !underfilled.is_empty() {
            failures.push(format!(
                "FILL: label ink spans < {:.0}% of the pill fill on {} scanline(s): {}",
                MIN_FILL_RATIO * 100.0,
                underfilled.len(),
                underfilled.join("; ")
            ));
        }

        // ── AC-124-1: nothing paints outside the reading column ──────────────────────────────────
        if m.ink_max_x > column_right_px + COLUMN_SLACK_PX {
            failures.push(format!(
                "COLUMN ESCAPE: label ink reaches x={} while the {READING_COLUMN_WIDTH_PTS}pt \
                 reading column ends at x={column_right_px} (+{COLUMN_SLACK_PX}px slack). That is \
                 {}px of document-surface corruption outside the column.",
                m.ink_max_x,
                m.ink_max_x as i64 - column_right_px as i64
            ));
        }
        if m.pill_band.max_x > column_right_px + COLUMN_SLACK_PX {
            failures.push(format!(
                "COLUMN ESCAPE: the pill reaches x={} while the reading column ends at \
                 x={column_right_px}",
                m.pill_band.max_x
            ));
        }

        if failures.is_empty() {
            println!(
                "MT124_VERDICT scenario={} result=WRAPPED_LABEL_MATCHES_PILL_INSIDE_COLUMN rows={}",
                scenario.id,
                m.ink_bands.len()
            );
        } else {
            println!(
                "MT124_VERDICT scenario={} result=VIOLATED variant=\"{}\" unfixed_signature=\"{}\"",
                scenario.id, scenario.variant, scenario.unfixed_signature
            );
            for f in &failures {
                println!("MT124_VIOLATION scenario={} {f}", scenario.id);
            }
            scenario_failures.push(format!("{} -> {}", scenario.id, failures.join(" | ")));
        }

        checked += 1;
    }

    assert!(
        scenario_failures.is_empty(),
        "MT-124 wrapped-chip painted-output proof FAILED for {} of {} scenarios:\n  {}",
        scenario_failures.len(),
        checked,
        scenario_failures.join("\n  ")
    );
    assert_eq!(
        checked,
        scenarios().len(),
        "every MT-124 scenario must have been measured"
    );
    assert_eq!(checked, 3, "AC-124-5 + the paragraph case require all three scenarios");

    let mut all = after_hashes.clone();
    all.sort();
    all.dedup();
    assert_eq!(
        all.len(),
        after_hashes.len(),
        "two scenarios produced byte-identical AFTER frames: {after_hashes:?}"
    );

    println!("\nMT124_PROOF_SCENARIOS_MEASURED={checked}");
}

//! WP-KERNEL-012 MT-077 (E13 i18n) — Unicode-correct text segmentation integration proofs.
//!
//! The per-module unit tests (in `src/text_intl/*`, `src/code_editor/cursor.rs`,
//! `src/rich_editor/renderer/input_handler.rs`, `src/rich_editor/renderer/line_layout.rs`) carry the
//! bulk of the AC coverage. This integration file proves the SAME behavior through the crate's PUBLIC
//! API surface (so a consumer outside the modules gets it) and produces the egui_kittest CJK-wrap
//! screenshot (PROOF2).
//!
//! PROOF MAP (contract proof_targets / acceptance_criteria):
//! - PROOF1 / AC2 + AC3 + AC4 + AC5 + AC6: the public `text_intl` functions resolve CJK break
//!   opportunities + kinsoku (AC2), grapheme caret move/delete (AC3/AC4 — the MANDATORY family-emoji
//!   case), and the Unicode word/char counts (AC5/AC6) — all asserted via `handshake_native::text_intl`.
//! - PROOF2 / AC1: `cjk_paragraph_wrap_screenshot` renders a long spaceless Chinese paragraph through a
//!   real egui frame with the MT-075 CJK fonts installed and (best-effort on a GPU host) saves
//!   `mt077_cjk_wrap.png` to the EXTERNAL artifact root; the logical wrap proof (galley.rows.len() > 1)
//!   always runs and stands as the AC1 evidence GPU-independently.
//! - PROOF3 / AC7 + AC8: LTR no-regression is the FULL `cargo test` suite staying green (this file adds
//!   coverage without weakening any existing test); the ASCII paths in the unit tests pin no-regression.
//!
//! ARTIFACT HYGIENE (CX-212E / contract screenshot rule, OVERRIDES any repo-local path the MT names):
//! the screenshot is written ONLY to the EXTERNAL `Handshake_Artifacts/handshake-test/wp-kernel-012-mt-077/`
//! root via `external_artifact_dir` (the same pattern as test_code_editor_panel.rs / test_keymap.rs /
//! test_font_coverage.rs). `assert_no_local_artifact_dir` checks BOTH `test_output/` AND
//! `tests/screenshots/` are absent after the run.

use std::path::{Path, PathBuf};

use egui::{FontId, RichText};
use egui_kittest::Harness;

use handshake_native::app::HandshakeApp;
use handshake_native::text_intl::{
    break_opportunities, char_count, is_break_before, next_grapheme_boundary, prev_grapheme_boundary,
    word_count, BreakOpportunity,
};

/// Crate-relative path to the EXTERNAL artifacts root (CX-212E), disk-agnostic. The crate sits at
/// `<repo>/src/frontend/handshake_native`, so four `..` reach `<repo>/..` where `Handshake_Artifacts`
/// is a sibling of the repo worktree. Identical to the helper in test_code_editor_panel.rs /
/// test_font_coverage.rs.
fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

/// Assert NO repo-local artifact directory exists under the crate (contract screenshot rule, CX-212E).
/// Checks BOTH `test_output/` AND `tests/screenshots/` — a tracked/written artifact under src/ or the
/// crate root is a hygiene FAILURE; all artifacts go to the external root only.
fn assert_no_local_artifact_dir() {
    for local in ["test_output", "tests/screenshots"] {
        let p = Path::new(local);
        assert!(
            !p.exists(),
            "ARTIFACT HYGIENE (CX-212E): no repo-local '{}' dir may exist — MT-077 screenshots go to \
             the external Handshake_Artifacts/handshake-test root only (found {})",
            local,
            p.display()
        );
    }
}

/// The family ZWJ emoji 👨‍👩‍👧 (man+ZWJ+woman+ZWJ+girl) — 5 scalars, 17 UTF-8 bytes, ONE grapheme
/// cluster (the MANDATORY MT-077 caret/delete case, AC3/AC4).
const FAMILY: &str = "👨‍👩‍👧";

/// A long spaceless Chinese paragraph (no whitespace anywhere) — the canonical AC1 CJK wrap run.
const CJK_PARAGRAPH: &str =
    "这是一个很长的中文段落里面完全没有任何空格所以传统的按空白换行的算法要么会让文字溢出可视区域要么永远不会换行必须按照统一码标准在表意文字之间断行";

// ── PROOF1 / AC3 + AC4: grapheme caret move + delete via the public API (MANDATORY family emoji) ───

#[test]
fn public_grapheme_boundaries_cross_clusters_whole() {
    // AC3: RIGHT over the family emoji crosses ALL its bytes in one step (never lands inside).
    assert_eq!(
        next_grapheme_boundary(FAMILY, 0),
        FAMILY.len(),
        "AC3: next grapheme boundary crosses the whole family emoji ({} bytes)",
        FAMILY.len()
    );
    // AC4: from the end, the previous boundary is 0 (Backspace removes the whole cluster).
    assert_eq!(
        prev_grapheme_boundary(FAMILY, FAMILY.len()),
        0,
        "AC4: previous grapheme boundary removes the whole family emoji"
    );
    // Combining accent + flag + decomposed Hangul are each ONE cluster too.
    assert_eq!(next_grapheme_boundary("e\u{0301}", 0), "e\u{0301}".len(), "combining é is one cluster");
    assert_eq!(next_grapheme_boundary("🇯🇵", 0), "🇯🇵".len(), "flag is one cluster");
    let hangul = "\u{1112}\u{1161}\u{11AB}"; // ᄒ ᅡ ᆫ -> 한
    assert_eq!(next_grapheme_boundary(hangul, 0), hangul.len(), "decomposed Hangul syllable is one cluster");

    // AC7 no-regression: ASCII still steps one byte per cluster.
    assert_eq!(next_grapheme_boundary("hello", 0), 1);
    assert_eq!(prev_grapheme_boundary("hello", 5), 4);
}

// ── PROOF1 / AC2: CJK break opportunities + kinsoku via the public API ─────────────────────────────

#[test]
fn public_cjk_break_opportunities_and_kinsoku() {
    // AC1 (UAX#14 side): a long spaceless Han run offers many interior break points so it CAN wrap.
    let han = "今天我写了很多中文字符";
    let interior = break_opportunities(han)
        .into_iter()
        .filter(|(i, op)| *i < han.len() && *op == BreakOpportunity::Allowed)
        .count();
    assert!(
        interior >= han.chars().count() - 2,
        "AC1: a spaceless Han run breaks between (almost) every ideograph; got {interior} interior breaks"
    );

    // AC2: no break BEFORE a closing bracket (kinsoku) — egui handles this; mirrored in text_intl.
    let s_close = "字\u{FF09}字"; // 字 ） 字
    assert!(
        !is_break_before(s_close, "字".len()),
        "AC2: UAX#14 kinsoku forbids a break immediately before the closing ）"
    );
    // AC2 (the gap egui misses): no break AFTER an opening bracket.
    let s_open = "字\u{FF08}字"; // 字 （ 字
    assert!(
        !is_break_before(s_open, "字".len() + "\u{FF08}".len()),
        "AC2: UAX#14 forbids a break immediately after the opening （"
    );
}

// ── PROOF1 / AC5 + AC6: Unicode word + grapheme char counts via the public API ────────────────────

#[test]
fn public_unicode_counts_are_documented() {
    // AC5: a mixed string counts per UAX#29 words. "Hello 世界 test" -> Hello + 世 + 界 + test = 4.
    assert_eq!(word_count("Hello 世界 test"), 4, "AC5: mixed Latin+CJK word count is 4 (documented)");
    // A spaceless CJK sentence is NOT one word.
    assert_eq!(word_count("今天我写了很多字"), 8, "AC5: spaceless CJK counts per-ideograph, not 1");
    // AC7 no-regression: ASCII prose unchanged.
    assert_eq!(word_count("hello world"), 2);

    // AC6: char count = grapheme clusters. The family emoji is ONE character.
    assert_eq!(char_count(FAMILY), 1, "AC6: the family emoji counts as 1 character (grapheme cluster)");
    assert_eq!(char_count("e\u{0301}"), 1, "AC6: combining é is 1 character");
    assert_eq!(char_count("🇯🇵"), 1, "AC6: a flag is 1 character");
    assert_eq!(char_count("日本語"), 3, "AC6: a CJK string counts each ideograph");
    // AC7 no-regression: ASCII char count unchanged.
    assert_eq!(char_count("hello"), 5);
}

// ── PROOF2 / AC1: long spaceless CJK paragraph WRAPS to multiple rows — screenshot ────────────────

#[cfg(feature = "bundled-fonts")]
#[test]
fn cjk_paragraph_wrap_screenshot() {
    // Render the long spaceless Chinese paragraph in a narrow column so egui's native CJK Galley wrap
    // breaks it across multiple visual rows (the AC1 base case — egui wraps Han/Kana natively with the
    // MT-075 NotoSansSC font; see rich_editor::renderer::line_layout module docs for the verified
    // mechanism). A narrow ui width forces the wrap.
    let wrap_px = 220.0_f32;
    let mut harness = Harness::builder()
        .with_size(egui::vec2(wrap_px + 40.0, 400.0))
        .wgpu()
        .build_ui(move |ui| {
            // Install the real MT-075 CJK fallback chain on THIS (fresh) kittest context.
            HandshakeApp::install_fonts(ui.ctx());
            ui.set_max_width(wrap_px);
            ui.label(RichText::new(CJK_PARAGRAPH).font(FontId::proportional(22.0)));
        });
    // Two frames: install/measure, then settle the wrapped galley with the installed CJK fonts.
    harness.run();
    harness.run();

    // Logical wrap proof (always runs, GPU-independent): lay the SAME paragraph out through egui's real
    // Galley path at the wrap width and assert it produced more than one row. This is the structural AC1
    // guarantee behind the screenshot.
    let rows = harness.ctx.fonts_mut(|f| {
        let mut job = egui::text::LayoutJob::default();
        job.wrap.max_width = wrap_px;
        job.append(
            CJK_PARAGRAPH,
            0.0,
            egui::text::TextFormat {
                font_id: FontId::proportional(22.0),
                color: egui::Color32::WHITE,
                ..Default::default()
            },
        );
        f.layout_job(job).rows.len()
    });
    assert!(
        rows > 1,
        "AC1 (logical): a long spaceless CJK paragraph must wrap to >1 visual row at a {wrap_px}pt wrap \
         width via egui's native CJK Galley breaking; got {rows} row(s)"
    );

    match harness.render() {
        Ok(image) => {
            let (w, h) = (image.width(), image.height());
            assert!(w > 0 && h > 0, "rendered image must be non-empty");

            let ext_dir = external_artifact_dir("wp-kernel-012-mt-077");
            let _ = std::fs::create_dir_all(&ext_dir);
            let png_path = ext_dir.join("mt077_cjk_wrap.png");
            let saved = image.save(&png_path).is_ok();

            // Pixel proof that the wrap actually happened on screen: a wrapped multi-row CJK paragraph
            // paints foreground glyphs across MULTIPLE vertical bands. Count, per horizontal scanline,
            // whether any foreground (non-background, opaque) pixel exists; a single-line render would
            // fill only ~one text-row band of scanlines, a wrapped render fills several separated bands.
            let raw = image.as_raw();
            let stride = w as usize * 4;
            // Background = the most common opaque pixel.
            let mut bg_counts: std::collections::HashMap<[u8; 4], u32> = std::collections::HashMap::new();
            for chunk in raw.chunks_exact(4) {
                if chunk[3] != 0 {
                    *bg_counts.entry([chunk[0], chunk[1], chunk[2], chunk[3]]).or_insert(0) += 1;
                }
            }
            let bg = bg_counts.iter().max_by_key(|(_, c)| **c).map(|(p, _)| *p);
            // For each row, does it contain a foreground pixel?
            let mut text_rows = 0usize;
            let mut prev_had = false;
            let mut bands = 0usize;
            for y in 0..h as usize {
                let row = &raw[y * stride..(y + 1) * stride];
                let has_fg = row.chunks_exact(4).any(|px| {
                    px[3] != 0 && Some([px[0], px[1], px[2], px[3]]) != bg
                });
                if has_fg {
                    text_rows += 1;
                    if !prev_had {
                        bands += 1; // a new vertical text band started
                    }
                }
                prev_had = has_fg;
            }
            // A wrapped paragraph spans many scanlines of text and forms multiple separated bands (one
            // per visual row, separated by inter-row gaps). Assert both: substantial text height AND
            // more than one band (the visible consequence of wrapping).
            assert!(
                text_rows > 20,
                "AC1 (pixel): expected a substantial painted CJK text area (>20 scanlines), got \
                 {text_rows} — the paragraph appears unrendered/blank"
            );
            assert!(
                bands >= 2,
                "AC1 (pixel): a WRAPPED multi-row CJK paragraph must paint >=2 separated vertical text \
                 bands; got {bands} — a non-wrapped single line would be 1 band"
            );

            println!(
                "PROOF2 screenshot: {w}x{h} saved={saved} ({}); logical_rows={rows} \
                 painted_scanlines={text_rows} text_bands={bands}. VISUAL INSPECTION REQUIRED by the \
                 reviewer (open the PNG; confirm the Chinese paragraph is wrapped across multiple lines \
                 within the narrow column, with real Han glyphs — no tofu, no overflow).",
                png_path.display()
            );
        }
        Err(e) => {
            println!(
                "BLOCKER(non-fatal): MT-077 CJK-wrap screenshot render unavailable (no wgpu adapter / \
                 headless GPU crash): {e}. The LOGICAL wrap proof (galley.rows.len()={rows} > 1) passed \
                 and stands as the AC1 evidence on this host; the PNG + pixel band-count is a GPU-host \
                 item."
            );
        }
    }

    assert_no_local_artifact_dir();
}

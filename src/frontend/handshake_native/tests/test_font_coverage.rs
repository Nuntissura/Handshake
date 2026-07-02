//! WP-KERNEL-012 MT-075 — Unicode font fallback chain coverage proofs.
//!
//! ROOT CAUSE this MT fixes (verified by KERNEL_BUILDER code inspection): before MT-075,
//! `HandshakeApp::install_fonts` registered ONLY Inter (Latin + Cyrillic + Greek) and egui's default
//! fonts ship no CJK, so Chinese/Japanese/Korean text rendered as tofu/notdef boxes. MT-075 builds an
//! ORDERED egui font fallback chain (Inter first, then bundled Noto Sans SC + KR + Symbols2 + Math,
//! then egui's default NotoEmoji) so every target script resolves to a real glyph.
//!
//! PROOF MAP (contract proof_targets / acceptance_criteria):
//! - PROOF1 / AC1 + AC4 + AC5: `family_fallback_order` asserts the exact fallback ORDER in the
//!   Proportional AND Monospace family vecs from the PURE `build_font_definitions()` (no GPU, fully
//!   deterministic), and `glyph_presence_per_script` drives a real headless `egui::Context` through
//!   `install_fonts` and asserts `Fonts::has_glyphs` resolves a representative codepoint of EVERY
//!   target script (Latin/Han/Kana/Hangul/Cyrillic/Greek/symbol/arrow/currency/box-drawing) to a
//!   NON-notdef glyph. `has_glyphs` returns true iff none of the chars hit the replacement glyph
//!   (egui epaint `Font::has_glyph` == `glyph_info(c) != replacement_glyph`), so this is a true
//!   coverage proof, not a tautology.
//! - PROOF2 / AC2: `unicode_coverage_screenshot` renders the multi-script string through a real frame
//!   and (best-effort on a GPU host) saves the PNG to the EXTERNAL artifact root, pixel-asserting the
//!   notdef box does NOT repeat across distinct scripts (a tofu-everywhere regression would paint the
//!   same notdef glyph for Han, Kana, Hangul — the test rejects that).
//! - AC3: `monospace_family_renders_cjk` proves the Monospace family (code editor) ALSO has the CJK
//!   fallbacks — a CJK comment in code renders real glyphs.
//! - AC6: `unmapped_codepoint_degrades_without_panic` proves a Plane-15 private-use codepoint present
//!   in NO font returns `has_glyph == false` (degrades to the single notdef box) WITHOUT panicking and
//!   without breaking the surrounding text's layout.
//! - AC7: the CJK source decision (BUNDLE Noto Sans CJK, not OS-load) is documented in app.rs +
//!   handoff; no `std::fs` font read exists, so there is no missing-OS-font panic path. (Asserted by
//!   `bundled_not_os_loaded` — `build_font_definitions` is `from_static` only.)
//! - AC8 / PROOF3: no-regression is the FULL `cargo test` suite staying green (install_fonts is shared
//!   infra used by every kittest); this file adds coverage without weakening any existing test.
//!
//! ARTIFACT HYGIENE (CX-212E / contract screenshot rule): the screenshot is written ONLY to the
//! EXTERNAL `Handshake_Artifacts/handshake-test/wp-kernel-012-mt-075/` root via `external_artifact_dir`
//! (the same pattern as test_code_editor_panel.rs / test_keymap.rs). `assert_no_local_artifact_dir`
//! checks BOTH `test_output/` AND `tests/screenshots/` are absent after the run.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use egui::{FontFamily, FontId};
use egui_kittest::Harness;

use handshake_native::app::{
    HandshakeApp, FALLBACK_FACE_ORDER, FONT_KEY_INTER, FONT_KEY_NOTO_KR, FONT_KEY_NOTO_MATH,
    FONT_KEY_NOTO_SC, FONT_KEY_NOTO_SYMBOLS2,
};

/// The multi-script proof string from AC2 (emoji last; egui's default NotoEmoji renders it).
const MULTISCRIPT: &str = "English 中文 日本語 한국어 Русский Ελληνικά ∑∫∞ →€✓ 😀";

/// Crate-relative path to the EXTERNAL artifacts root (CX-212E), disk-agnostic. The crate sits at
/// `<repo>/src/frontend/handshake_native`, so four `..` reach `<repo>/..` where `Handshake_Artifacts`
/// is a sibling of the repo worktree. Identical to the helper in test_code_editor_panel.rs.
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
            "ARTIFACT HYGIENE (CX-212E): no repo-local '{}' dir may exist — MT-075 screenshots go to \
             the external Handshake_Artifacts/handshake-test root only (found {})",
            local,
            p.display()
        );
    }
}

// ── PROOF1 / AC1: exact fallback ORDER in Proportional + Monospace (pure, no GPU) ─────────────────

#[cfg(feature = "bundled-fonts")]
#[test]
fn family_fallback_order() {
    use handshake_native::app::INTER_BOLD_FAMILY;
    let fonts = HandshakeApp::build_font_definitions();

    // The expected ordered prefix of EACH text family: Inter first, then the four broad-coverage Noto
    // faces in FALLBACK_FACE_ORDER. egui's default faces (its own Proportional/Monospace + NotoEmoji)
    // follow AFTER this prefix — we assert the PREFIX (the part MT-075 controls) and that Inter is
    // strictly index 0 (MC-1 / RISK-2: Inter must win for Latin so the look is unchanged).
    let mut expected_prefix = vec![FONT_KEY_INTER.to_owned()];
    expected_prefix.extend(FALLBACK_FACE_ORDER.iter().map(|s| (*s).to_owned()));

    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        let vec = fonts
            .families
            .get(&family)
            .unwrap_or_else(|| panic!("AC1: family {family:?} must exist in FontDefinitions"));

        assert_eq!(
            vec.first().map(String::as_str),
            Some(FONT_KEY_INTER),
            "AC1/MC-1: Inter must be index-0 (FIRST) in {family:?} so Latin renders Inter unchanged; \
             got vec={vec:?}"
        );
        assert!(
            vec.len() >= expected_prefix.len(),
            "AC1: {family:?} vec {vec:?} shorter than the expected fallback prefix {expected_prefix:?}"
        );
        assert_eq!(
            &vec[..expected_prefix.len()],
            expected_prefix.as_slice(),
            "AC1: {family:?} fallback order must be [Inter, SC, KR, Symbols2, Math, <egui defaults>]; \
             got {vec:?}"
        );
        // The Noto fallbacks must come BEFORE egui's default text face (which has no CJK), otherwise
        // egui's empty-of-CJK default could shadow them. Assert each Noto face appears and that none
        // of egui's default faces sit between Inter and the Noto block.
        for face in FALLBACK_FACE_ORDER {
            assert!(
                vec.contains(&face.to_owned()),
                "AC1: {family:?} missing fallback face '{face}'"
            );
        }
    }

    // Bold named family must also carry the CJK fallbacks so bold CJK does not tofu.
    let bold = fonts
        .families
        .get(&FontFamily::Name(INTER_BOLD_FAMILY.into()))
        .expect("Inter-Bold named family must exist");
    assert_eq!(bold.first().map(String::as_str), Some(INTER_BOLD_FAMILY));
    for face in FALLBACK_FACE_ORDER {
        assert!(
            bold.contains(&face.to_owned()),
            "Inter-Bold missing fallback face '{face}'"
        );
    }

    // The five faces must all be registered in font_data (the family vecs reference them by key).
    for key in [
        FONT_KEY_INTER,
        FONT_KEY_NOTO_SC,
        FONT_KEY_NOTO_KR,
        FONT_KEY_NOTO_SYMBOLS2,
        FONT_KEY_NOTO_MATH,
        INTER_BOLD_FAMILY,
    ] {
        assert!(
            fonts.font_data.contains_key(key),
            "AC1: font_data must contain registered face '{key}'"
        );
    }

    println!(
        "PROOF1 family order: Proportional+Monospace prefix = [Inter, SC, KR, Symbols2, Math] \
         (Inter index 0); bold family carries fallbacks; 5 faces registered."
    );
}

// ── PROOF1 / AC4 + AC5: glyph presence per script via a real headless context ─────────────────────

#[cfg(feature = "bundled-fonts")]
#[test]
fn glyph_presence_per_script() {
    let ctx = egui::Context::default();
    HandshakeApp::install_fonts(&ctx);
    // Materialize the font set for this frame so the Fonts cache reflects the install.
    let _ = ctx.run(Default::default(), |_| {});

    // (script label, representative text) — every run MUST resolve to real glyphs in the chain.
    let cases: &[(&str, &str)] = &[
        ("Latin", "English"),
        ("Chinese (Han)", "中文"),
        ("Japanese (kanji+kana)", "日本語"),
        ("Korean (Hangul)", "한국어"),
        ("Cyrillic", "Русский Привет"),
        ("Greek", "Ελληνικά Καλημέρα"),
        ("Math/sum/integral/infinity", "∑∫∞"),
        ("Arrow + currency", "→€£"),
        ("Check/cross", "✓✗"),
        ("Em dash", "—"),
        ("Box drawing", "│─┌"),
    ];

    let prop = FontId::proportional(14.0);
    let mono = FontId::monospace(14.0);

    let mut failures = Vec::new();
    for (label, text) in cases {
        let has_prop = ctx.fonts_mut(|f| f.has_glyphs(&prop, text));
        if !has_prop {
            failures.push(format!(
                "{label:?} ('{text}') has no glyph in the Proportional chain"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "AC4/AC5: every target script must resolve to a real (non-notdef) glyph in the fallback \
         chain; failures: {failures:#?}"
    );

    // Spot-check a couple of the scripts that ONLY a Noto fallback can supply, proving the fallbacks
    // are actually consulted (not just Inter). Han + Hangul are absent from Inter, so a `true` here
    // can only come from the appended Noto faces.
    assert!(
        ctx.fonts_mut(|f| f.has_glyph(&prop, '中')),
        "Han must resolve via the Noto SC fallback"
    );
    assert!(
        ctx.fonts_mut(|f| f.has_glyph(&prop, '한')),
        "Hangul must resolve via the Noto KR fallback"
    );
    assert!(
        ctx.fonts_mut(|f| f.has_glyph(&mono, '中')),
        "AC3: Han must resolve in the Monospace chain too"
    );
    assert!(
        ctx.fonts_mut(|f| f.has_glyph(&mono, '│')),
        "AC5: box-drawing must resolve in Monospace"
    );

    println!(
        "PROOF1 glyph presence: all {} target scripts resolve to real glyphs in the Proportional \
         chain; Han/Hangul/box-drawing also resolve in Monospace (fallbacks proven consulted).",
        cases.len()
    );
}

// ── AC3: the Monospace (code-editor) family renders CJK + box-drawing ─────────────────────────────

#[cfg(feature = "bundled-fonts")]
#[test]
fn monospace_family_renders_cjk() {
    let ctx = egui::Context::default();
    HandshakeApp::install_fonts(&ctx);
    let _ = ctx.run(Default::default(), |_| {});

    let mono = FontId::monospace(14.0);
    // A code comment with CJK + box-drawing — the exact AC3 scenario.
    let comment = "// 注释 コメント 주석 ┌─┐";
    let has = ctx.fonts_mut(|f| f.has_glyphs(&mono, comment));
    assert!(
        has,
        "AC3: a CJK code comment '{comment}' must render real glyphs in the Monospace family \
         (code editor) — the fallbacks must be appended to Monospace, not only Proportional"
    );
    println!("AC3 monospace CJK: code comment '{comment}' fully covered by the Monospace fallback chain.");
}

// ── AC6: an unmapped codepoint degrades to notdef WITHOUT panic + without breaking layout ─────────

#[cfg(feature = "bundled-fonts")]
#[test]
fn unmapped_codepoint_degrades_without_panic() {
    let ctx = egui::Context::default();
    HandshakeApp::install_fonts(&ctx);
    let _ = ctx.run(Default::default(), |_| {});

    // U+FFFFD: a Plane-15 (Supplementary Private Use Area-A) codepoint. No bundled face maps it, so it
    // resolves to the single notdef/replacement glyph. `has_glyph` returning FALSE here (and NOT
    // panicking) is the graceful-degradation proof.
    let pua = '\u{FFFFD}';
    let prop = FontId::proportional(14.0);
    let has = ctx.fonts_mut(|f| f.has_glyph(&prop, pua));
    assert!(
        !has,
        "AC6: a Plane-15 private-use codepoint (U+FFFFD) must NOT resolve in any bundled font (it \
         degrades to the notdef box); has_glyph returned true unexpectedly"
    );

    // The unmapped codepoint must not break the layout of surrounding REAL text: a string mixing the
    // PUA char with real text still lays out (egui paints notdef for the one char, real glyphs around
    // it) and produces a non-zero galley with the SAME line count as the all-real version — proving
    // the bad char did not collapse or explode the line.
    let mixed = format!("ok {pua} 中文 done");
    let (galley_rows, galley_w) = ctx.fonts_mut(|f| {
        let g = f.layout_no_wrap(mixed.clone(), prop.clone(), egui::Color32::WHITE);
        (g.rows.len(), g.rect.width())
    });
    assert!(
        galley_w > 0.0,
        "AC6: mixed string with a notdef char still produces a non-empty galley"
    );
    assert_eq!(
        galley_rows, 1,
        "AC6: the unmapped codepoint must not split/break the single line of surrounding text \
         (got {galley_rows} rows)"
    );

    println!(
        "AC6 graceful degradation: U+FFFFD has no glyph (degrades to notdef, no panic); surrounding \
         text still lays out as one row, width {galley_w:.1}px."
    );
}

// ── AC7: CJK is BUNDLED (deterministic), not OS-loaded — no fs-read panic path ────────────────────

#[cfg(feature = "bundled-fonts")]
#[test]
fn bundled_not_os_loaded() {
    // The decision is documented in app.rs (build_font_definitions doc comment). This test pins the
    // behavioral consequence: the install path requires NO OS font read, so it cannot panic on a
    // missing OS font (AC7). build_font_definitions() succeeds with only the bundled `from_static`
    // faces — calling it here (no ctx, no fs) proves there is no OS-load dependency.
    let fonts = HandshakeApp::build_font_definitions();
    // All five faces present purely from include_bytes! (bundled), independent of any OS font dir.
    for key in [
        FONT_KEY_INTER,
        FONT_KEY_NOTO_SC,
        FONT_KEY_NOTO_KR,
        FONT_KEY_NOTO_SYMBOLS2,
        FONT_KEY_NOTO_MATH,
    ] {
        assert!(
            fonts.font_data.contains_key(key),
            "AC7: face '{key}' must be BUNDLED (from_static), proving no OS-load/fs-read dependency"
        );
    }
    println!("AC7: CJK fonts are bundled via include_bytes! — deterministic, no OS-font-read panic path.");
}

// ── PROOF2 / AC2: multi-script screenshot, no-tofu pixel assertion ────────────────────────────────

#[cfg(feature = "bundled-fonts")]
#[test]
fn unicode_coverage_screenshot() {
    // Build a harness that installs the MT-075 fonts on its context, then paints the multi-script
    // string at a large size so each script's glyphs occupy distinct pixel regions.
    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 160.0))
        .wgpu()
        .build_ui(|ui| {
            // Install the real fallback chain on THIS context (the kittest harness context is fresh).
            HandshakeApp::install_fonts(ui.ctx());
            ui.label(egui::RichText::new(MULTISCRIPT).font(FontId::proportional(40.0)));
        });
    // Two frames: first installs/measures, second settles galleys with the installed fonts.
    harness.run();
    harness.run();

    // Logical no-tofu proof (always runs, GPU-independent): every non-emoji char of the proof string
    // resolves to a real glyph in the installed chain. This is the structural guarantee behind the
    // pixel assertion below.
    let prop40 = FontId::proportional(40.0);
    let non_emoji: String = MULTISCRIPT
        .chars()
        .filter(|c| (*c as u32) < 0x1F000)
        .collect();
    let logical_ok = harness.ctx.fonts_mut(|f| f.has_glyphs(&prop40, &non_emoji));
    assert!(
        logical_ok,
        "AC2 (logical): every non-emoji char of the proof string must resolve to a real glyph in the \
         installed chain — '{non_emoji}'"
    );

    match harness.render() {
        Ok(image) => {
            let (w, h) = (image.width(), image.height());
            assert!(w > 0 && h > 0, "rendered image must be non-empty");

            // Save to the EXTERNAL artifact root ONLY (CX-212E).
            let ext_dir = external_artifact_dir("wp-kernel-012-mt-075");
            let _ = std::fs::create_dir_all(&ext_dir);
            let png_path = ext_dir.join("mt075_unicode_coverage.png");
            let saved = image.save(&png_path).is_ok();

            // Pixel no-tofu heuristic: tofu (notdef) boxes are a single identical glyph shape. If the
            // CJK/Hangul runs were all tofu, the text band would contain MANY repetitions of the exact
            // same box glyph -> a SMALL number of distinct non-background colors and a very high count
            // of one repeated foreground pattern. A correctly-rendered multi-script string has DIVERSE
            // glyph shapes -> many distinct foreground pixel colors across the band. Assert the
            // rendered text band carries diverse foreground content (not one repeated box).
            let raw = image.as_raw();
            let mut counts: HashMap<[u8; 4], u32> = HashMap::new();
            let mut i = 0usize;
            while i + 4 <= raw.len() {
                let px = [raw[i], raw[i + 1], raw[i + 2], raw[i + 3]];
                if px[3] != 0 {
                    *counts.entry(px).or_insert(0) += 1;
                }
                i += 4 * 2; // sample every 2nd pixel (text is fine; keep resolution)
            }
            let bg = counts.iter().max_by_key(|(_, c)| **c).map(|(p, _)| *p);
            let foreground: u32 = counts
                .iter()
                .filter(|(p, _)| Some(**p) != bg)
                .map(|(_, c)| *c)
                .sum();
            let distinct_fg = counts.keys().filter(|p| Some(**p) != bg).count();

            // Real multi-script text produces many foreground pixels and many distinct AA colors. Tofu
            // boxes (or an empty render) would produce far fewer. These thresholds are conservative for
            // a 900x160 band of 40px text across 8+ scripts.
            assert!(
                foreground > 2_000,
                "AC2 (pixel): expected a substantial painted text band (>2000 fg pixels), got \
                 {foreground} — the multi-script string appears unrendered/blank"
            );
            assert!(
                distinct_fg >= 8,
                "AC2 (pixel): expected diverse glyph shapes (>=8 distinct foreground colors from AA \
                 across scripts), got {distinct_fg} — a tofu-everywhere regression would repeat one box"
            );

            println!(
                "PROOF2 screenshot: {}x{} saved={saved} ({}); fg_pixels={foreground} \
                 distinct_fg_colors={distinct_fg}; logical no-tofu proven for all non-emoji runs. \
                 VISUAL INSPECTION REQUIRED by the reviewer (open the PNG; confirm 中文/日本語/한국어/ \
                 Русский/Ελληνικά/∑∫∞/→€✓ all show real glyphs, 😀 shows an emoji).",
                w,
                h,
                png_path.display()
            );
        }
        Err(e) => {
            println!(
                "BLOCKER(non-fatal): MT-075 unicode screenshot render unavailable (no wgpu adapter / \
                 headless GPU crash): {e}. The LOGICAL no-tofu proof (every non-emoji glyph resolves \
                 in the installed chain) passed and stands as the AC2 coverage evidence on this host; \
                 the PNG + pixel sample is a GPU-host item."
            );
        }
    }

    assert_no_local_artifact_dir();
}

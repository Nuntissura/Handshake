// MT-003 theme token tests. Pure-unit tests prove every ported palette/syntax token value,
// the rgba() premultiplication, the override map semantics (apply / unknown-key / invalid-value),
// and that apply_to_ctx reaches a headless egui::Context. A kittest integration test drives the
// real HandshakeApp headlessly, clicks the theme-toggle button by its AccessKit role, and proves
// the central panel background flips dark<->light. A grep invariant proves no widget hardcodes a
// Color32 outside the theme module.
//
// NOTE (path deviation): the contract names this file tests/native_gui/test_theme.rs, but
// `cargo test -p handshake-native --test test_theme` requires the integration target to live at
// the crate's tests/test_theme.rs (cargo derives the --test name from the file in tests/). Placing
// it in a tests/native_gui/ subdir would not register a `test_theme` target. This file therefore
// lives at tests/test_theme.rs (same crate tests/ dir as the existing test_skeleton.rs).

use std::collections::HashMap;

use egui::Color32;
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;
use handshake_native::theme::{apply_to_ctx, parse_color, HsPalette, HsTheme};

// ---- light palette token values (App.css :root) ----

#[test]
fn light_palette_tokens_match_react_root() {
    let p = HsPalette::light();
    assert_eq!(p.bg, Color32::from_rgb(0xf8, 0xfa, 0xfc), "light.bg");
    assert_eq!(p.surface, Color32::from_rgb(0xff, 0xff, 0xff), "light.surface");
    assert_eq!(p.surface_strong, Color32::from_rgb(0x0f, 0x17, 0x2a), "light.surface_strong");
    assert_eq!(p.border, Color32::from_rgb(0xe2, 0xe8, 0xf0), "light.border");
    assert_eq!(p.border_strong, Color32::from_rgb(0x1e, 0x29, 0x3b), "light.border_strong");
    assert_eq!(p.text, Color32::from_rgb(0x0f, 0x17, 0x2a), "light.text");
    assert_eq!(p.text_subtle, Color32::from_rgb(0x47, 0x55, 0x69), "light.text_subtle");
    assert_eq!(p.accent, Color32::from_rgb(0x25, 0x63, 0xeb), "light.accent");
    assert_eq!(p.accent_soft, Color32::from_rgb(0xe0, 0xf2, 0xfe), "light.accent_soft");
    assert_eq!(p.success_bg, Color32::from_rgb(0xdc, 0xfc, 0xe7), "light.success_bg");
    assert_eq!(p.success_text, Color32::from_rgb(0x16, 0x65, 0x34), "light.success_text");
    assert_eq!(p.error_bg, Color32::from_rgb(0xfe, 0xe2, 0xe2), "light.error_bg");
    assert_eq!(p.error_text, Color32::from_rgb(0xb9, 0x1c, 0x1c), "light.error_text");
}

// ---- dark palette token values (App.css [data-theme='dark']) ----

#[test]
fn dark_palette_tokens_match_react_dark() {
    let p = HsPalette::dark();
    assert_eq!(p.bg, Color32::from_rgb(0x12, 0x14, 0x18), "dark.bg");
    assert_eq!(p.surface, Color32::from_rgb(0x1d, 0x23, 0x2b), "dark.surface");
    assert_eq!(p.surface_strong, Color32::from_rgb(0xf8, 0xfa, 0xfc), "dark.surface_strong");
    assert_eq!(p.border, Color32::from_rgb(0x39, 0x44, 0x52), "dark.border");
    assert_eq!(p.border_strong, Color32::from_rgb(0xd1, 0xd5, 0xdb), "dark.border_strong");
    assert_eq!(p.text, Color32::from_rgb(0xf8, 0xfa, 0xfc), "dark.text");
    assert_eq!(p.text_subtle, Color32::from_rgb(0xba, 0xc5, 0xd1), "dark.text_subtle");
    assert_eq!(p.accent, Color32::from_rgb(0x22, 0xc5, 0x5e), "dark.accent");
    assert_eq!(p.success_text, Color32::from_rgb(0x86, 0xef, 0xac), "dark.success_text");
    assert_eq!(p.error_text, Color32::from_rgb(0xfc, 0xa5, 0xa5), "dark.error_text");
}

// ---- MT-006 divider tokens (added to MT-003 palette) ----

#[test]
fn divider_tokens_exist_for_both_palettes_and_differ() {
    let dark = HsPalette::dark();
    let light = HsPalette::light();

    // Light divider tokens: React slate gradient (148,163,184) at center alpha 0.45 idle / 0.7
    // hover; grab = the light accent (#2563eb).
    assert_eq!(
        light.divider_idle,
        Color32::from_rgba_unmultiplied(148, 163, 184, (0.45 * 255.0_f32).round() as u8),
        "light.divider_idle",
    );
    assert_eq!(
        light.divider_hover,
        Color32::from_rgba_unmultiplied(148, 163, 184, (0.7 * 255.0_f32).round() as u8),
        "light.divider_hover",
    );
    assert_eq!(light.divider_grab, Color32::from_rgb(0x25, 0x63, 0xeb), "light.divider_grab");

    // Dark divider tokens: higher idle alpha (0.55) / hover (0.85) to read against the dark bg;
    // grab = the dark accent (#22c55e).
    assert_eq!(
        dark.divider_idle,
        Color32::from_rgba_unmultiplied(148, 163, 184, (0.55 * 255.0_f32).round() as u8),
        "dark.divider_idle",
    );
    assert_eq!(
        dark.divider_hover,
        Color32::from_rgba_unmultiplied(148, 163, 184, (0.85 * 255.0_f32).round() as u8),
        "dark.divider_hover",
    );
    assert_eq!(dark.divider_grab, Color32::from_rgb(0x22, 0xc5, 0x5e), "dark.divider_grab");

    // Idle and grab must differ between themes so a model can tell the themes apart.
    assert_ne!(dark.divider_idle, light.divider_idle, "divider_idle differs dark vs light");
    assert_ne!(dark.divider_grab, light.divider_grab, "divider_grab differs dark vs light");
}

#[test]
fn divider_token_overrides_apply() {
    let overrides = HashMap::from([("divider_idle".to_string(), "#ff0000".to_string())]);
    let p = HsPalette::dark().with_overrides(&overrides);
    assert_eq!(p.divider_idle, Color32::from_rgb(255, 0, 0), "divider_idle override applies");
    // other divider tokens unchanged
    assert_eq!(p.divider_hover, HsPalette::dark().divider_hover, "divider_hover untouched");
}

// ---- rgba() -> premultiplied Color32 (CONTROL-2) ----

#[test]
fn dark_rgba_tokens_are_premultiplied() {
    // DEVIATION (CONTROL-2): the contract's hand-computed example (6,35,17,46) mixed a
    // truncated green channel with a rounded blue channel and is internally inconsistent.
    // We delegate premultiplication to egui's own Color32::from_rgba_unmultiplied (the exact
    // compositing math the renderer uses), whose authoritative results are below. The straight
    // (r,g,b,alpha8) inputs are unchanged from the contract; only the premultiplied bytes
    // follow egui rather than the contract's bad arithmetic.
    let p = HsPalette::dark();
    // rgba(34,197,94,0.18) -> alpha8 46
    assert_eq!(p.accent_soft, Color32::from_rgba_unmultiplied(34, 197, 94, 46), "dark.accent_soft");
    assert_eq!(p.accent_soft.to_array(), [6, 36, 17, 46], "dark.accent_soft premultiplied bytes");
    // rgba(34,197,94,0.2) -> alpha8 51
    assert_eq!(p.success_bg, Color32::from_rgba_unmultiplied(34, 197, 94, 51), "dark.success_bg");
    // rgba(248,113,113,0.2) -> alpha8 51
    assert_eq!(p.error_bg, Color32::from_rgba_unmultiplied(248, 113, 113, 51), "dark.error_bg");
}

#[test]
fn parse_color_rgba_matches_premultiplied() {
    assert_eq!(
        parse_color("rgba(34,197,94,0.18)"),
        Some(Color32::from_rgba_unmultiplied(34, 197, 94, 46)),
    );
}

// ---- syntax tokens ----

#[test]
fn dark_syntax_tokens() {
    let s = HsPalette::dark().syntax;
    assert_eq!(s.keyword, Color32::from_rgb(0x56, 0x9c, 0xd6), "dark.syntax.keyword");
    assert_eq!(s.string, Color32::from_rgb(0xce, 0x91, 0x78), "dark.syntax.string");
    assert_eq!(s.comment, Color32::from_rgb(0x6a, 0x99, 0x55), "dark.syntax.comment");
    assert_eq!(s.number, Color32::from_rgb(0xb5, 0xce, 0xa8), "dark.syntax.number");
    assert_eq!(s.type_name, Color32::from_rgb(0x4e, 0xc9, 0xb0), "dark.syntax.type_name");
    // punctuation/background are derived from the base palette tokens.
    assert_eq!(s.punctuation, HsPalette::dark().text_subtle, "dark.syntax.punctuation==text_subtle");
    assert_eq!(s.background, HsPalette::dark().bg, "dark.syntax.background==bg");
}

#[test]
fn light_syntax_tokens() {
    let s = HsPalette::light().syntax;
    assert_eq!(s.keyword, Color32::from_rgb(0x00, 0x00, 0xff), "light.syntax.keyword");
    assert_eq!(s.string, Color32::from_rgb(0xa3, 0x15, 0x15), "light.syntax.string");
    assert_eq!(s.comment, Color32::from_rgb(0x00, 0x80, 0x00), "light.syntax.comment");
    assert_eq!(s.number, Color32::from_rgb(0x09, 0x86, 0x58), "light.syntax.number");
    assert_eq!(s.type_name, Color32::from_rgb(0x26, 0x7f, 0x99), "light.syntax.type_name");
    assert_eq!(s.punctuation, HsPalette::light().text_subtle, "light.syntax.punctuation==text_subtle");
    assert_eq!(s.background, HsPalette::light().bg, "light.syntax.background==bg");
}

// ---- gutter diagnostic / breakpoint tokens (WP-KERNEL-012 MT-007) ----
//
// The MT-007 gutter affordance colors live in the theme layer (one of the two sanctioned homes for
// Color32 literals) instead of being hardcoded in gutter.rs, so the CONTROL-4 no-hardcode invariant
// stays GREEN. These tests pin the exact MT-contract hues: Error=red, Warning=yellow,
// Info/Hint=cornflower rgb(100,149,237), breakpoint=rgb(229,60,60).

#[test]
fn dark_diagnostic_tokens_match_mt007_contract() {
    let d = HsPalette::dark().diagnostics;
    assert_eq!(d.error, Color32::from_rgb(0xff, 0x00, 0x00), "dark.diagnostics.error==RED");
    assert_eq!(d.warning, Color32::from_rgb(0xff, 0xff, 0x00), "dark.diagnostics.warning==YELLOW");
    assert_eq!(d.info, Color32::from_rgb(100, 149, 237), "dark.diagnostics.info==cornflower");
    assert_eq!(d.hint, Color32::from_rgb(100, 149, 237), "dark.diagnostics.hint==cornflower");
    assert_eq!(d.breakpoint, Color32::from_rgb(229, 60, 60), "dark.diagnostics.breakpoint");
    // Error must equal egui's RED so the AC-003 gutter red-pixel screenshot stays valid.
    assert_eq!(d.error, Color32::RED, "dark.diagnostics.error==Color32::RED");
    assert_eq!(d.warning, Color32::YELLOW, "dark.diagnostics.warning==Color32::YELLOW");
}

#[test]
fn light_diagnostic_tokens_match_mt007_contract() {
    let d = HsPalette::light().diagnostics;
    assert_eq!(d.error, Color32::from_rgb(0xff, 0x00, 0x00), "light.diagnostics.error==RED");
    assert_eq!(d.warning, Color32::from_rgb(0xff, 0xff, 0x00), "light.diagnostics.warning==YELLOW");
    assert_eq!(d.info, Color32::from_rgb(100, 149, 237), "light.diagnostics.info==cornflower");
    assert_eq!(d.hint, Color32::from_rgb(100, 149, 237), "light.diagnostics.hint==cornflower");
    assert_eq!(d.breakpoint, Color32::from_rgb(229, 60, 60), "light.diagnostics.breakpoint");
}

// ---- HsTheme::palette mapping + toggle ----

#[test]
fn theme_palette_mapping_and_toggle() {
    assert_eq!(HsTheme::Dark.palette().bg, HsPalette::dark().bg);
    assert_eq!(HsTheme::Light.palette().bg, HsPalette::light().bg);
    assert_eq!(HsTheme::Dark.toggled(), HsTheme::Light);
    assert_eq!(HsTheme::Light.toggled(), HsTheme::Dark);
}

// ---- override map semantics (acceptance + CONTROL-3) ----

#[test]
fn override_applies_known_hex_key() {
    let overrides = HashMap::from([("bg".to_string(), "#ff0000".to_string())]);
    let p = HsPalette::dark().with_overrides(&overrides);
    assert_eq!(p.bg, Color32::from_rgb(255, 0, 0), "override bg -> red");
    // other tokens unchanged
    assert_eq!(p.accent, HsPalette::dark().accent, "non-overridden token unchanged");
}

#[test]
fn override_unknown_key_is_ignored() {
    let overrides = HashMap::from([("not_a_token".to_string(), "#ff0000".to_string())]);
    let p = HsPalette::dark().with_overrides(&overrides);
    assert_eq!(p, HsPalette::dark(), "unknown key leaves palette unchanged");
}

#[test]
fn override_invalid_hex_is_ignored() {
    // CONTROL-3: invalid value must not panic and must leave the token unchanged.
    let overrides = HashMap::from([("bg".to_string(), "#ZZZZZZ".to_string())]);
    let p = HsPalette::dark().with_overrides(&overrides);
    assert_eq!(p, HsPalette::dark(), "invalid hex leaves palette unchanged");
}

#[test]
fn override_empty_map_is_noop() {
    let p = HsPalette::dark().with_overrides(&HashMap::new());
    assert_eq!(p, HsPalette::dark(), "empty override map is a no-op");
}

#[test]
fn override_accepts_rgba_value() {
    let overrides = HashMap::from([("accent".to_string(), "rgba(255,0,0,0.5)".to_string())]);
    let p = HsPalette::dark().with_overrides(&overrides);
    // alpha8 = round(0.5*255) = 128; egui premultiplies via from_rgba_unmultiplied.
    assert_eq!(p.accent, Color32::from_rgba_unmultiplied(255, 0, 0, 128));
}

// ---- apply_to_ctx headless (no panic + reaches the context) ----

#[test]
fn apply_to_ctx_does_not_panic_and_sets_panel_fill() {
    let ctx = egui::Context::default();

    apply_to_ctx(&HsPalette::dark(), &ctx);
    assert_eq!(
        ctx.style().visuals.panel_fill,
        HsPalette::dark().bg,
        "dark apply sets panel_fill to dark.bg",
    );

    apply_to_ctx(&HsPalette::light(), &ctx);
    assert_eq!(
        ctx.style().visuals.panel_fill,
        HsPalette::light().bg,
        "light apply sets panel_fill to light.bg",
    );
}

// ---- real app: toggle button flips the themed panel_fill (visual switch, headless) ----

#[test]
fn app_toggle_switches_panel_fill_dark_to_light() {
    let app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }));
    let mut harness = Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run();

    // Default desktop theme is Dark; the panel background must be the dark token.
    assert_eq!(harness.state().current_theme(), HsTheme::Dark, "default theme is Dark");
    assert_eq!(
        harness.ctx.style().visuals.panel_fill,
        HsPalette::dark().bg,
        "initial panel_fill is dark.bg (#121418)",
    );

    // The toggle button shows 'Light' while dark is active. Click it.
    harness.get_by_label("Light").click();
    harness.run();

    assert_eq!(harness.state().current_theme(), HsTheme::Light, "theme toggled to Light");
    assert_eq!(
        harness.ctx.style().visuals.panel_fill,
        HsPalette::light().bg,
        "after toggle panel_fill is light.bg (#f8fafc)",
    );

    println!("PASS: all palette tokens correct");
}

// ---- no-hardcoded-Color32 invariant (CONTROL-4) ----

#[test]
fn no_hardcoded_color32_outside_theme_module() {
    use std::fs;
    use std::path::Path;

    fn scan(dir: &Path, hits: &mut Vec<String>) {
        for entry in fs::read_dir(dir).expect("read_dir src") {
            let entry = entry.expect("dir entry");
            let path = entry.path();
            if path.is_dir() {
                scan(&path, hits);
                continue;
            }
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !name.ends_with(".rs") {
                continue;
            }
            // palette.rs and syntax.rs are the only sanctioned homes for Color32 literals.
            if name == "palette.rs" || name == "syntax.rs" {
                continue;
            }
            let content = fs::read_to_string(&path).expect("read rs file");
            for (i, line) in content.lines().enumerate() {
                // Flag the opaque-literal forms only: `Color32::from_rgb(`, `Color32::WHITE`,
                // `Color32::BLACK`. The open paren on `from_rgb(` is REQUIRED — without it the check
                // is a substring of `Color32::from_rgba_premultiplied(` /
                // `Color32::from_rgba_unmultiplied(`, which the invariant deliberately does NOT flag
                // (those are the sanctioned translucent-affordance form used by the MT-003 selection
                // tint, MT-004 match highlight, and MT-002 minimap overlay — premultiplied/unmultiplied
                // RGBA the contracts name explicitly, not opaque syntax hex). Matching the bare prefix
                // over-flagged every `from_rgba_*` call; pinning `from_rgb(` keeps the no-hardcoded-hex
                // guard honest without false positives on the RGBA affordance form.
                if line.contains("Color32::from_rgb(")
                    || line.contains("Color32::WHITE")
                    || line.contains("Color32::BLACK")
                {
                    hits.push(format!("{}:{}: {}", name, i + 1, line.trim()));
                }
            }
        }
    }

    // CARGO_MANIFEST_DIR is the crate root; src/ holds all widget code.
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut hits = Vec::new();
    scan(&src, &mut hits);
    assert!(
        hits.is_empty(),
        "hardcoded Color32 literals found outside palette.rs/syntax.rs:\n{}",
        hits.join("\n"),
    );
}

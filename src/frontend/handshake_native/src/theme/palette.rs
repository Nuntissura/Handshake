//! Theme enum, palette struct, and the dark/light constructors.
//!
//! Token values are ported verbatim from the legacy React app's CSS custom properties:
//!   - light: `app/src/App.css` `:root` block
//!   - dark:  `app/src/App.css` `[data-theme='dark']` block
//!
//! CSS `rgba(r,g,b,a)` values are stored as egui PREMULTIPLIED-alpha `Color32`. We delegate
//! the premultiplication to egui's own `Color32::from_rgba_unmultiplied`, which is the exact
//! compositing math the renderer uses for every other blended fill. (The MT-003 contract's
//! CONTROL-2 hand-computed example `(6,35,17,46)` is inconsistent — it truncated the green
//! channel but rounded the blue channel; egui's authoritative result for
//! `rgba(34,197,94,0.18)` is `(6,36,17,46)`. Using egui's function avoids the "subtly wrong
//! premultiplied alpha" defect RISK-2 warns about; see test_theme.rs for the deviation note.)

use crate::theme::syntax::HsSyntaxTokens;
use egui::Color32;
use std::collections::HashMap;

/// Which base palette is active. `serde` so a later MT can persist it via the workspace
/// settings API (GET/PUT /workspaces/{id}/settings) at no extra cost now.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum HsTheme {
    Light,
    Dark,
}

impl HsTheme {
    /// The fully-resolved base palette for this theme (no overrides applied).
    pub fn palette(self) -> HsPalette {
        match self {
            HsTheme::Light => HsPalette::light(),
            HsTheme::Dark => HsPalette::dark(),
        }
    }

    /// Toggle dark<->light. Used by the top-bar theme button.
    pub fn toggled(self) -> Self {
        match self {
            HsTheme::Light => HsTheme::Dark,
            HsTheme::Dark => HsTheme::Light,
        }
    }
}

/// The full set of semantic color tokens for one theme. Every widget reads colors from
/// here; no widget hardcodes a `Color32`. `Clone` so `HandshakeApp` can hold a resolved
/// palette (with overrides) independently of the active `HsTheme`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HsPalette {
    pub bg: Color32,
    pub surface: Color32,
    pub surface_strong: Color32,
    pub border: Color32,
    pub border_strong: Color32,
    pub text: Color32,
    pub text_subtle: Color32,
    pub accent: Color32,
    pub accent_soft: Color32,
    pub success_bg: Color32,
    pub success_text: Color32,
    pub error_bg: Color32,
    pub error_text: Color32,
    pub syntax: HsSyntaxTokens,
}

impl HsPalette {
    /// Light palette — ported from `App.css` `:root`.
    pub fn light() -> Self {
        let bg = Color32::from_rgb(0xf8, 0xfa, 0xfc);
        let text_subtle = Color32::from_rgb(0x47, 0x55, 0x69);
        Self {
            bg,
            surface: Color32::from_rgb(0xff, 0xff, 0xff),
            surface_strong: Color32::from_rgb(0x0f, 0x17, 0x2a),
            border: Color32::from_rgb(0xe2, 0xe8, 0xf0),
            border_strong: Color32::from_rgb(0x1e, 0x29, 0x3b),
            text: Color32::from_rgb(0x0f, 0x17, 0x2a),
            text_subtle,
            accent: Color32::from_rgb(0x25, 0x63, 0xeb),
            accent_soft: Color32::from_rgb(0xe0, 0xf2, 0xfe),
            success_bg: Color32::from_rgb(0xdc, 0xfc, 0xe7),
            success_text: Color32::from_rgb(0x16, 0x65, 0x34),
            error_bg: Color32::from_rgb(0xfe, 0xe2, 0xe2),
            error_text: Color32::from_rgb(0xb9, 0x1c, 0x1c),
            syntax: HsSyntaxTokens::light(text_subtle, bg),
        }
    }

    /// Dark palette — ported from `App.css` `[data-theme='dark']`.
    /// `accent_soft`, `success_bg`, `error_bg` are CSS `rgba(...)` -> premultiplied Color32.
    pub fn dark() -> Self {
        let bg = Color32::from_rgb(0x12, 0x14, 0x18);
        let text_subtle = Color32::from_rgb(0xba, 0xc5, 0xd1);
        Self {
            bg,
            surface: Color32::from_rgb(0x1d, 0x23, 0x2b),
            surface_strong: Color32::from_rgb(0xf8, 0xfa, 0xfc),
            border: Color32::from_rgb(0x39, 0x44, 0x52),
            border_strong: Color32::from_rgb(0xd1, 0xd5, 0xdb),
            text: Color32::from_rgb(0xf8, 0xfa, 0xfc),
            text_subtle,
            accent: Color32::from_rgb(0x22, 0xc5, 0x5e),
            // rgba(34,197,94,0.18) -> alpha8 46 -> premultiplied (6,35,17,46)
            accent_soft: rgba_premultiplied(34, 197, 94, 0.18),
            // rgba(34,197,94,0.2) -> alpha8 51 -> premultiplied (6,39,18,51)
            success_bg: rgba_premultiplied(34, 197, 94, 0.2),
            success_text: Color32::from_rgb(0x86, 0xef, 0xac),
            // rgba(248,113,113,0.2) -> alpha8 51 -> premultiplied (49,22,22,51)
            error_bg: rgba_premultiplied(248, 113, 113, 0.2),
            error_text: Color32::from_rgb(0xfc, 0xa5, 0xa5),
            syntax: HsSyntaxTokens::dark(text_subtle, bg),
        }
    }

    /// Apply a per-workspace override map on top of this palette. Keys are semantic token
    /// names (e.g. `"bg"`, `"accent"`); values are `#RRGGBB`, `#RRGGBBAA`, or `rgba(...)`
    /// strings. Unknown keys and unparseable values are silently ignored, leaving the
    /// existing token unchanged (CONTROL-3: never panic on bad input; future-proof).
    ///
    /// In MT-003 the map is always empty (it is loaded from the backend settings API in a
    /// later MT); an empty map is a no-op.
    pub fn with_overrides(mut self, overrides: &HashMap<String, String>) -> Self {
        for (key, value) in overrides {
            let Some(color) = parse_color(value) else {
                continue; // invalid value: leave token unchanged
            };
            match key.as_str() {
                "bg" => self.bg = color,
                "surface" => self.surface = color,
                "surface_strong" => self.surface_strong = color,
                "border" => self.border = color,
                "border_strong" => self.border_strong = color,
                "text" => self.text = color,
                "text_subtle" => self.text_subtle = color,
                "accent" => self.accent = color,
                "accent_soft" => self.accent_soft = color,
                "success_bg" => self.success_bg = color,
                "success_text" => self.success_text = color,
                "error_bg" => self.error_bg = color,
                "error_text" => self.error_text = color,
                _ => {} // unknown key: silently ignored
            }
        }
        self
    }

    /// Heuristic: is this a dark palette? Used to seed the egui base `Visuals`.
    /// Perceptual luminance of `bg` below mid-gray => dark.
    pub fn is_dark(&self) -> bool {
        let [r, g, b, _] = self.bg.to_array();
        let lum = 0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32;
        lum < 128.0
    }
}

/// Convert a straight-alpha CSS `rgba(r,g,b,a)` (a in 0..1) to an egui premultiplied
/// `Color32`. alpha8 = round(a*255); premultiplication is delegated to egui's
/// `from_rgba_unmultiplied` so it matches the renderer's compositing exactly.
fn rgba_premultiplied(r: u8, g: u8, b: u8, a: f32) -> Color32 {
    let a8 = (a * 255.0).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgba_unmultiplied(r, g, b, a8)
}

/// Parse a color string used in override maps / ported CSS. Accepts:
///   - `#RRGGBB`        (opaque)
///   - `#RRGGBBAA`      (straight alpha -> stored premultiplied)
///   - `rgba(r,g,b,a)`  (a is a float 0..1 -> stored premultiplied)
///
/// Returns `None` for anything unparseable (callers leave the token unchanged).
pub fn parse_color(s: &str) -> Option<Color32> {
    let s = s.trim();
    if let Some(inner) = s.strip_prefix("rgba(").and_then(|x| x.strip_suffix(')')) {
        return parse_rgba_components(inner);
    }
    parse_hex_color(s)
}

/// `#RRGGBB` or `#RRGGBBAA`. AA is straight alpha and is stored premultiplied.
fn parse_hex_color(hex: &str) -> Option<Color32> {
    let h = hex.trim().trim_start_matches('#');
    match h.len() {
        6 => {
            let r = u8::from_str_radix(&h[0..2], 16).ok()?;
            let g = u8::from_str_radix(&h[2..4], 16).ok()?;
            let b = u8::from_str_radix(&h[4..6], 16).ok()?;
            Some(Color32::from_rgb(r, g, b))
        }
        8 => {
            let r = u8::from_str_radix(&h[0..2], 16).ok()?;
            let g = u8::from_str_radix(&h[2..4], 16).ok()?;
            let b = u8::from_str_radix(&h[4..6], 16).ok()?;
            let a = u8::from_str_radix(&h[6..8], 16).ok()?;
            // Color32::from_rgba_unmultiplied premultiplies for us.
            Some(Color32::from_rgba_unmultiplied(r, g, b, a))
        }
        _ => None,
    }
}

/// Parse the inside of `rgba(...)`: `r, g, b, a` where r/g/b are 0..255 ints and a is a
/// float 0..1. Stored as premultiplied alpha.
fn parse_rgba_components(inner: &str) -> Option<Color32> {
    let mut parts = inner.split(',').map(|p| p.trim());
    let r: u8 = parts.next()?.parse().ok()?;
    let g: u8 = parts.next()?.parse().ok()?;
    let b: u8 = parts.next()?.parse().ok()?;
    let a: f32 = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None; // too many components
    }
    if !(0.0..=1.0).contains(&a) {
        return None;
    }
    Some(rgba_premultiplied(r, g, b, a))
}

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

use crate::theme::syntax::{HsDiagnosticTokens, HsSyntaxTokens};
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
    /// Split-divider line color when idle (no hover, no drag). Sourced from the React
    /// `.main-divider` slate gradient (`app/src/App.css` ~3451–3469: `rgba(148,163,184,a)`); the
    /// idle band uses the gradient's center alpha (0.45). Dark and light deliberately differ so a
    /// model can tell the themes apart (MT-006 divider-token test).
    pub divider_idle: Color32,
    /// Split-divider line color on pointer hover / keyboard focus. A stronger slate than idle so the
    /// grab affordance reads clearly (React has no explicit `:hover` divider rule; this is the
    /// native strengthen-on-hover convention).
    pub divider_hover: Color32,
    /// Split-divider line color while actively dragging (grab). Uses the theme accent — React draws
    /// the focus-visible affordance with `var(--hs-color-accent)` (`app/src/App.css` 3441–3443), so
    /// the accent is the canonical "the operator is manipulating this divider" color.
    pub divider_grab: Color32,
    /// Integrated rail (scrollbar + splitter) IDLE color (MT-010). The MT-010 GUI look-and-behavior
    /// table specifies a dedicated dark-family rail palette distinct from the React slate divider
    /// gradient, so the rails read as one cohesive native control family. Stored premultiplied.
    /// Dark: `#2A2A2F` @ 0.6; Light: `#C8C8D0` @ 0.6.
    pub scrollbar_idle: Color32,
    /// Integrated rail HOVER color (MT-010). Dark: `#4A4A55` @ 0.85; Light: `#9898A8` @ 0.85.
    pub scrollbar_hover: Color32,
    /// Integrated rail GRAB color (MT-010), fully opaque accent. Dark: `#7A7AFF`; Light: `#5050FF`.
    pub scrollbar_grab: Color32,
    /// Integrated rail DISABLED color (MT-010), e.g. a scrollbar whose content fits the viewport.
    /// Dark: `#1E1E22` @ 0.3; Light: `#E0E0E8` @ 0.3.
    pub scrollbar_disabled: Color32,
    pub syntax: HsSyntaxTokens,
    /// Editor gutter diagnostic-severity + breakpoint affordance colors (WP-KERNEL-012 MT-007). The
    /// gutter resolves these from the live theme so its dots/bars/breakpoint circle are theme tokens,
    /// not hardcoded literals (CONTROL-4 no-hardcode invariant).
    pub diagnostics: HsDiagnosticTokens,
    /// Loom-graph "canvas" content-type node colour (WP-KERNEL-012 MT-021). A DERIVED token: the
    /// 50/50 channel mean of `accent` and `diagnostics.breakpoint`, which reads as a desaturated
    /// violet/plum on either theme (distinct from the note blue + tag_hub green). Derived here in
    /// `palette.rs` — the sanctioned home for colour construction — so the graph widget never builds a
    /// `Color32` of its own (CONTROL-4 no-hardcode invariant; the architecture-guard test scans every
    /// widget `.rs` for `Color32::from_rgb(` and allows it only in palette.rs/syntax.rs).
    pub graph_canvas: Color32,
    /// Background fill for a `<mark>…</mark>` search-highlight run in the LoomSearchV2 results
    /// (WP-KERNEL-012 MT-028). Ported from the React parity reference's `<mark>` amber
    /// (`rgb(255, 214, 0)`). Defined HERE (a sanctioned `Color32` home) rather than in
    /// `loom_search_v2.rs` because the no-hardcoded-color guard (`tests/test_theme.rs`
    /// `no_hardcoded_color32_outside_theme_module`) flags `Color32::from_rgb(` outside
    /// palette.rs/syntax.rs (the MT-028 THEME-GUARD correction). The highlight text run reads this
    /// token for its `TextFormat::background`, so the panel widget holds no color literal. Identical
    /// across themes: the amber reads against both the light and dark result-row surfaces, matching the
    /// React surface's single `<mark>` colour.
    pub search_highlight_bg: Color32,
    /// Vertical INDENT-GUIDE line color for the code editor (WP-KERNEL-012 MT-054). The faint 1px
    /// vertical lines VS Code draws at each indent level. Sourced HERE (a sanctioned `Color32` home) so
    /// the panel paint path reads a theme token instead of a hardcoded literal (CONTROL-4 — the
    /// `no_hardcoded_color32_outside_theme_module` guard exempts only palette.rs/syntax.rs). A muted,
    /// low-contrast line so it reads as a hint, not as content; per-theme so it tracks dark/light.
    pub indent_guide: Color32,
    /// ACTIVE indent-guide line color (WP-KERNEL-012 MT-054): the guide enclosing the cursor's current
    /// block is drawn in this brighter token (VS Code's `editorIndentGuide.activeBackground`). Distinct
    /// from [`indent_guide`] so the operator can see which block the cursor is in.
    pub indent_guide_active: Color32,
    /// Bracket-pair colorization palette for the code editor (WP-KERNEL-012 MT-054). Each bracket is
    /// drawn in `bracket_pair_palette[depth % len]`, matching VS Code's `bracketPairColorization`.
    /// Sourced HERE so the panel paint path holds NO color literal (CONTROL-4). A six-hue rotation
    /// covering the common nesting depths; per-theme so the hues read against the editor background.
    pub bracket_pair_palette: Vec<Color32>,
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
            // Divider tokens: React slate gradient (148,163,184). Light theme: idle at the gradient
            // center alpha (0.45), a stronger hover (0.7). Grab = the light accent (#2563eb).
            divider_idle: rgba_premultiplied(148, 163, 184, 0.45),
            divider_hover: rgba_premultiplied(148, 163, 184, 0.7),
            divider_grab: Color32::from_rgb(0x25, 0x63, 0xeb),
            // MT-010 light rail palette (exact spec table).
            scrollbar_idle: rgba_premultiplied(0xC8, 0xC8, 0xD0, 0.6),
            scrollbar_hover: rgba_premultiplied(0x98, 0x98, 0xA8, 0.85),
            scrollbar_grab: Color32::from_rgb(0x50, 0x50, 0xFF),
            scrollbar_disabled: rgba_premultiplied(0xE0, 0xE0, 0xE8, 0.3),
            syntax: HsSyntaxTokens::light(text_subtle, bg),
            diagnostics: HsDiagnosticTokens::light(),
            // canvas node colour (MT-021): 50/50 mean of accent (#2563eb) and breakpoint red
            // (rgb 229,60,60) -> a violet/plum, derived here so the graph widget holds no literal.
            graph_canvas: blend_channels(
                Color32::from_rgb(0x25, 0x63, 0xeb),
                HsDiagnosticTokens::light().breakpoint,
            ),
            // <mark> search-highlight amber (MT-028), ported from the React `<mark>` rgb(255,214,0).
            search_highlight_bg: Color32::from_rgb(255, 214, 0),
            // MT-054 indent guides: a muted slate at low alpha against the light bg; the active guide a
            // stronger slate so the cursor's block reads. Stored premultiplied (UI affordance hint).
            indent_guide: rgba_premultiplied(0x94, 0xa3, 0xb8, 0.35),
            indent_guide_active: rgba_premultiplied(0x47, 0x55, 0x69, 0.85),
            // MT-054 bracket-pair palette: six distinct hues that read against the light editor bg
            // (VS Code's default bracketPairColorization rotation, light-adapted).
            bracket_pair_palette: vec![
                Color32::from_rgb(0x00, 0x70, 0xC1), // blue
                Color32::from_rgb(0x31, 0x9C, 0x31), // green
                Color32::from_rgb(0xB8, 0x66, 0x00), // amber
                Color32::from_rgb(0x9C, 0x27, 0xB0), // purple
                Color32::from_rgb(0x00, 0x88, 0x88), // teal
                Color32::from_rgb(0xC2, 0x18, 0x5B), // magenta
            ],
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
            // Divider tokens: React slate gradient (148,163,184). Dark theme needs a touch more
            // luminance to read against the dark bg, so idle uses a higher alpha (0.55) than light
            // (0.45) — making the two themes' idle premultiplied bytes genuinely differ (MT-006
            // divider-token test). Hover strengthens to 0.85; grab = the dark accent (#22c55e).
            divider_idle: rgba_premultiplied(148, 163, 184, 0.55),
            divider_hover: rgba_premultiplied(148, 163, 184, 0.85),
            divider_grab: Color32::from_rgb(0x22, 0xc5, 0x5e),
            // MT-010 dark rail palette (exact spec table).
            scrollbar_idle: rgba_premultiplied(0x2A, 0x2A, 0x2F, 0.6),
            scrollbar_hover: rgba_premultiplied(0x4A, 0x4A, 0x55, 0.85),
            scrollbar_grab: Color32::from_rgb(0x7A, 0x7A, 0xFF),
            scrollbar_disabled: rgba_premultiplied(0x1E, 0x1E, 0x22, 0.3),
            syntax: HsSyntaxTokens::dark(text_subtle, bg),
            diagnostics: HsDiagnosticTokens::dark(),
            // canvas node colour (MT-021): 50/50 mean of accent (#22c55e) and breakpoint red
            // (rgb 229,60,60), derived here so the graph widget holds no literal.
            graph_canvas: blend_channels(
                Color32::from_rgb(0x22, 0xc5, 0x5e),
                HsDiagnosticTokens::dark().breakpoint,
            ),
            // <mark> search-highlight amber (MT-028), ported from the React `<mark>` rgb(255,214,0).
            // Identical to light: the amber reads against both result-row surfaces (React parity).
            search_highlight_bg: Color32::from_rgb(255, 214, 0),
            // MT-054 indent guides: a muted slate at low alpha against the dark bg; the active guide a
            // brighter slate so the cursor's block reads. Stored premultiplied (UI affordance hint).
            indent_guide: rgba_premultiplied(0x94, 0xa3, 0xb8, 0.30),
            indent_guide_active: rgba_premultiplied(0xba, 0xc5, 0xd1, 0.85),
            // MT-054 bracket-pair palette: six distinct hues that read against the dark editor bg
            // (VS Code's default bracketPairColorization rotation: gold/orchid/blue, extended).
            bracket_pair_palette: vec![
                Color32::from_rgb(0xFF, 0xD7, 0x00), // gold
                Color32::from_rgb(0xDA, 0x70, 0xD6), // orchid
                Color32::from_rgb(0x17, 0x9F, 0xFF), // blue
                Color32::from_rgb(0x7C, 0xC8, 0x4F), // green
                Color32::from_rgb(0xFF, 0x8C, 0x42), // orange
                Color32::from_rgb(0x4E, 0xC9, 0xB0), // teal
            ],
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
                "divider_idle" => self.divider_idle = color,
                "divider_hover" => self.divider_hover = color,
                "divider_grab" => self.divider_grab = color,
                "scrollbar_idle" => self.scrollbar_idle = color,
                "scrollbar_hover" => self.scrollbar_hover = color,
                "scrollbar_grab" => self.scrollbar_grab = color,
                "scrollbar_disabled" => self.scrollbar_disabled = color,
                "graph_canvas" => self.graph_canvas = color,
                "search_highlight_bg" => self.search_highlight_bg = color,
                "indent_guide" => self.indent_guide = color,
                "indent_guide_active" => self.indent_guide_active = color,
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

/// 50/50 per-channel mean of two opaque colours, used to derive the `graph_canvas` token (MT-021)
/// from `accent` + `diagnostics.breakpoint`. Lives in `palette.rs` (a sanctioned Color32 home) so
/// graph node colours stay theme-derived and no widget constructs a `Color32` of its own.
fn blend_channels(a: Color32, b: Color32) -> Color32 {
    let [ar, ag, ab, _] = a.to_array();
    let [br, bg, bb, _] = b.to_array();
    Color32::from_rgb(
        ((ar as u16 + br as u16) / 2) as u8,
        ((ag as u16 + bg as u16) / 2) as u8,
        ((ab as u16 + bb as u16) / 2) as u8,
    )
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

/// A stable 12-hue identity palette for tag chips (WP-KERNEL-012 MT-023; relocated here 2026-06-23).
/// A tag's chip color is a hash of its title (data-driven identity), NOT a theme token, so the same
/// tag always looks the same across light/dark. It lives in palette.rs because that is a sanctioned
/// home for `Color32` literals: the no-hardcoded-color guard (`tests/test_theme.rs`
/// `no_hardcoded_color32_outside_theme_module`) exempts `palette.rs`/`syntax.rs` only. It was moved
/// out of `graph/tags_panel.rs`, which the guard correctly flagged (CONTROL-4).
pub fn tag_chip_palette() -> [Color32; 12] {
    [
        Color32::from_rgb(0xE5, 0x39, 0x35), // red
        Color32::from_rgb(0xFB, 0x8C, 0x00), // orange
        Color32::from_rgb(0xFD, 0xD8, 0x35), // amber
        Color32::from_rgb(0x7C, 0xB3, 0x42), // lime
        Color32::from_rgb(0x43, 0xA0, 0x47), // green
        Color32::from_rgb(0x00, 0x89, 0x7B), // teal
        Color32::from_rgb(0x00, 0xAC, 0xC1), // cyan
        Color32::from_rgb(0x1E, 0x88, 0xE5), // blue
        Color32::from_rgb(0x3F, 0x51, 0xB5), // indigo
        Color32::from_rgb(0x8E, 0x24, 0xAA), // purple
        Color32::from_rgb(0xD8, 0x1B, 0x60), // pink
        Color32::from_rgb(0x6D, 0x4C, 0x41), // brown
    ]
}

/// Number of distinct default group-color tokens for the WP-KERNEL-012 MT-060 graph control panel.
pub const GRAPH_GROUP_PALETTE_LEN: usize = 8;

/// A stable 8-hue default palette for graph filter/group swatches (WP-KERNEL-012 MT-060). A group's
/// default color is assigned by its discovery order modulo this palette (a deterministic, theme-stable
/// identity hue), NOT a semantic theme token — the same group key always reads with the same hue across
/// light/dark so the graph legend and node coloring are consistent. It lives in `palette.rs` because
/// that is the sanctioned home for `Color32` literals: the no-hardcoded-color guard
/// (`tests/test_theme.rs` `no_hardcoded_color32_outside_theme_module`) exempts `palette.rs`/`syntax.rs`
/// ONLY, so the MT-060 "NO `Color32::from_rgb` literal in graph_controls.rs/graph_view.rs" control
/// (CONTROL-4) is satisfied by sourcing every group hue from this function. The hues are a distinct,
/// evenly-spaced subset reusing the tag-chip vocabulary so a tag group's default graph hue is in the
/// same visual family as its chip (interop legibility).
pub fn graph_group_palette() -> [Color32; GRAPH_GROUP_PALETTE_LEN] {
    [
        Color32::from_rgb(0x1E, 0x88, 0xE5), // blue
        Color32::from_rgb(0x43, 0xA0, 0x47), // green
        Color32::from_rgb(0xFB, 0x8C, 0x00), // orange
        Color32::from_rgb(0x8E, 0x24, 0xAA), // purple
        Color32::from_rgb(0x00, 0xAC, 0xC1), // cyan
        Color32::from_rgb(0xD8, 0x1B, 0x60), // pink
        Color32::from_rgb(0xFD, 0xD8, 0x35), // amber
        Color32::from_rgb(0x00, 0x89, 0x7B), // teal
    ]
}

/// Number of distinct default section-frame color tokens for the WP-KERNEL-012 MT-061 Loom canvas
/// section/group frames.
pub const CANVAS_SECTION_PALETTE_LEN: usize = 8;

/// A stable 8-hue default palette for Obsidian-Canvas section/group FRAMES on the native Loom canvas
/// (WP-KERNEL-012 MT-061). A section frame is derived per distinct `group_id`; its hue is assigned by the
/// group's discovery order modulo this palette (a deterministic, theme-stable identity hue), NOT a
/// semantic theme token — the same group_id always reads with the same frame hue across light/dark so the
/// canvas grouping is visually stable. It lives in `palette.rs` because that is the sanctioned home for
/// `Color32` literals: the no-hardcoded-color guard (`tests/test_theme.rs`
/// `no_hardcoded_color32_outside_theme_module`) exempts `palette.rs`/`syntax.rs` ONLY, so the MT-061
/// CONTROL-4 "NO `Color32::from_rgb` literal in canvas_board.rs / canvas_sections.rs" requirement is
/// satisfied by sourcing every frame hue from this function. The hues reuse the graph-group vocabulary so
/// a canvas section reads in the same visual family as its graph/tag group (interop legibility). The
/// frame fill is drawn translucent at paint time (the caller applies a low-alpha multiply); these hues
/// are the opaque identity colors.
pub fn canvas_section_palette() -> [Color32; CANVAS_SECTION_PALETTE_LEN] {
    graph_group_palette()
}

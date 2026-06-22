//! Muted syntax-highlight token set for future code/editor surfaces (MT-011+).
//!
//! Not present in the React CSS today; derived from common muted editor schemes so the
//! native code panes have a consistent palette from day one. Values are owned here (the
//! only places `Color32` literals are allowed are `palette.rs` and `syntax.rs`).

use egui::Color32;

/// Syntax-highlighting colors for a single theme (dark or light).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HsSyntaxTokens {
    pub keyword: Color32,
    pub string: Color32,
    pub comment: Color32,
    pub number: Color32,
    pub type_name: Color32,
    /// Derived from the base palette's `text_subtle` token (passed in by the constructor).
    pub punctuation: Color32,
    /// Derived from the base palette's `bg` token (passed in by the constructor).
    pub background: Color32,
}

impl HsSyntaxTokens {
    /// Dark-mode syntax tokens. `text_subtle`/`bg` come from the owning palette so
    /// punctuation and editor background stay consistent with the surrounding chrome.
    pub fn dark(text_subtle: Color32, bg: Color32) -> Self {
        Self {
            keyword: Color32::from_rgb(0x56, 0x9c, 0xd6),
            string: Color32::from_rgb(0xce, 0x91, 0x78),
            comment: Color32::from_rgb(0x6a, 0x99, 0x55),
            number: Color32::from_rgb(0xb5, 0xce, 0xa8),
            type_name: Color32::from_rgb(0x4e, 0xc9, 0xb0),
            punctuation: text_subtle,
            background: bg,
        }
    }

    /// Light-mode syntax tokens. `text_subtle`/`bg` come from the owning palette.
    pub fn light(text_subtle: Color32, bg: Color32) -> Self {
        Self {
            keyword: Color32::from_rgb(0x00, 0x00, 0xff),
            string: Color32::from_rgb(0xa3, 0x15, 0x15),
            comment: Color32::from_rgb(0x00, 0x80, 0x00),
            number: Color32::from_rgb(0x09, 0x86, 0x58),
            type_name: Color32::from_rgb(0x26, 0x7f, 0x99),
            punctuation: text_subtle,
            background: bg,
        }
    }
}

/// Editor gutter diagnostic + breakpoint affordance colors for a single theme.
///
/// These are UI affordances of the code-editor surface (the diagnostic severity dots / 3px left
/// bars and the breakpoint circle), not arbitrary widget hex. They live here in `syntax.rs` — one of
/// the two sanctioned homes for `Color32` literals (the other is `palette.rs`) — so the no-hardcode
/// invariant (CONTROL-4, grep-enforced by `tests/test_theme.rs`) stays GREEN and the gutter reads
/// these from the live theme instead of baking literals into widget code. Resolved per frame from
/// the active `HsTheme` so the gutter affordances track dark/light like every other token.
///
/// Token values are the ones the WP-KERNEL-012 MT-007 contract names exactly: Error = pure red,
/// Warning = pure yellow, Info/Hint = cornflower blue `rgb(100,149,237)`, breakpoint = `rgb(229,60,60)`.
/// Dark and light currently share the same severity hues (these are saturated semantic signals that
/// read on either background, matching VS Code, which also keeps severity hues constant across themes);
/// the per-theme constructors exist so a later MT can diverge them via the override map without a
/// signature change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HsDiagnosticTokens {
    /// Error severity dot + 3px left bar (pure red — the AC-003 red-pixel affordance).
    pub error: Color32,
    /// Warning severity dot + left bar (pure yellow).
    pub warning: Color32,
    /// Info severity dot + left bar (cornflower blue).
    pub info: Color32,
    /// Hint severity dot + left bar (the dimmest level; shares the Info blue per the MT contract).
    pub hint: Color32,
    /// Filled breakpoint circle color (a slightly softened red so it reads distinct from the pure-red
    /// error dot).
    pub breakpoint: Color32,
}

impl HsDiagnosticTokens {
    /// Dark-theme gutter diagnostic/breakpoint colors (the MT-007 contract values).
    pub fn dark() -> Self {
        Self {
            error: Color32::from_rgb(0xff, 0x00, 0x00),
            warning: Color32::from_rgb(0xff, 0xff, 0x00),
            info: Color32::from_rgb(100, 149, 237),
            hint: Color32::from_rgb(100, 149, 237),
            breakpoint: Color32::from_rgb(229, 60, 60),
        }
    }

    /// Light-theme gutter diagnostic/breakpoint colors. Currently identical to dark (saturated
    /// severity signals that read on either background, matching VS Code's theme-constant severity
    /// hues); kept as its own constructor so a later MT can diverge without a call-site change.
    pub fn light() -> Self {
        Self::dark()
    }
}

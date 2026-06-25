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

// ===========================================================================
// WP-KERNEL-012 MT-072 — built-in syntax-palette tables (Muted | Standard).
//
// The settings Syntax section offers Muted | Standard | Custom modes. Muted and
// Standard are FIXED Color32 tables; Custom layers per-scope overrides over the
// Standard table. These tables live HERE in syntax.rs — one of the two sanctioned
// homes for Color32 literals (CONTROL-4, grep-enforced by tests/test_theme.rs) —
// so settings_editor_section.rs / code_editor::highlight.rs hold ZERO Color32
// literals (which would trip the no-hardcode invariant).
//
// Each table maps EVERY one of the eight HighlightScope variants (Keyword, String,
// Comment, Number, Function, Type, Operator, Other) to a concrete color, with NO
// gap (AC-004 — a palette-completeness test iterates all variants). The order of
// the eight colors here is the order of [`SyntaxPaletteEntry`] below; the
// highlight resolver indexes by scope, never by position.
// ===========================================================================

/// A complete eight-scope syntax color set: one color per `HighlightScope` variant, in the order
/// Keyword, String, Comment, Number, Function, Type, Operator, Other. A plain struct (not a map) so
/// the table is a compile-time const with no missing scope possible — the type itself enforces
/// completeness (AC-004).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SyntaxPaletteEntry {
    /// Keyword color (`fn`, `let`, `return`, ...).
    pub keyword: Color32,
    /// String literal color.
    pub string: Color32,
    /// Comment color.
    pub comment: Color32,
    /// Numeric literal color.
    pub number: Color32,
    /// Function/method/constructor name color.
    pub function: Color32,
    /// Type name color.
    pub type_name: Color32,
    /// Operator/punctuation-operator color.
    pub operator: Color32,
    /// Fallback color for any other/unclassified scope.
    pub other: Color32,
}

/// The STANDARD built-in syntax palette: a VS-Code-Dark-Plus-like set. Every scope distinct; the
/// `Custom` mode layers user overrides over THIS table (any un-overridden scope reads from here —
/// AC-004 no-gap fallback).
pub const STANDARD_PALETTE: SyntaxPaletteEntry = SyntaxPaletteEntry {
    keyword: Color32::from_rgb(0x56, 0x9c, 0xd6),
    string: Color32::from_rgb(0xce, 0x91, 0x78),
    comment: Color32::from_rgb(0x6a, 0x99, 0x55),
    number: Color32::from_rgb(0xb5, 0xce, 0xa8),
    function: Color32::from_rgb(0xdc, 0xdc, 0xaa),
    type_name: Color32::from_rgb(0x4e, 0xc9, 0xb0),
    operator: Color32::from_rgb(0xd4, 0xd4, 0xd4),
    other: Color32::from_rgb(0xd4, 0xd4, 0xd4),
};

/// The MUTED built-in syntax palette: a lower-saturation set for users who want gentler highlighting.
/// Still maps every scope (no gap — AC-004); the hues are desaturated relative to STANDARD.
pub const MUTED_PALETTE: SyntaxPaletteEntry = SyntaxPaletteEntry {
    keyword: Color32::from_rgb(0x7c, 0x95, 0xb0),
    string: Color32::from_rgb(0xb0, 0x95, 0x84),
    comment: Color32::from_rgb(0x7d, 0x8f, 0x76),
    number: Color32::from_rgb(0xa8, 0xb6, 0x9c),
    function: Color32::from_rgb(0xb6, 0xb6, 0x97),
    type_name: Color32::from_rgb(0x82, 0xab, 0xa1),
    operator: Color32::from_rgb(0x9d, 0x9d, 0x9d),
    other: Color32::from_rgb(0x9d, 0x9d, 0x9d),
};

#[cfg(test)]
mod tests {
    use super::*;

    /// MT-072: the Muted and Standard built-in palette tables are complete (every scope is a concrete
    /// non-default color) and differ from each other. The struct type guarantees no missing scope; this
    /// asserts the two tables carry distinct keyword colors (the mode selector visibly changes colors).
    #[test]
    fn builtin_palettes_are_complete_and_distinct() {
        // A struct field per scope means completeness is enforced at compile time; this asserts the two
        // tables are not accidentally identical.
        assert_ne!(STANDARD_PALETTE.keyword, MUTED_PALETTE.keyword, "Muted and Standard keyword differ");
        assert_ne!(STANDARD_PALETTE.string, MUTED_PALETTE.string, "Muted and Standard string differ");
        // Standard is the VS-Code-Dark-Plus keyword blue (a known anchor — guards an accidental edit).
        assert_eq!(STANDARD_PALETTE.keyword, Color32::from_rgb(0x56, 0x9c, 0xd6));
    }
}

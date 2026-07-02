//! WP-KERNEL-012 MT-072 (E12) — syntax palette LIVE update proofs (PT-002 / AC-003 / AC-004).
//!
//! The Syntax settings section's Muted | Standard | Custom palette feeds MT-001's HighlightScope ->
//! Color32 path through `code_editor::resolve_scope_color`. These proofs assert:
//!
//! - AC-003: a Custom swatch edit changes the color `resolve_scope_color` returns for that
//!   `HighlightScope` in the SAME frame (no restart, no reload) — the live-update criterion.
//! - AC-004: Muted, Standard, and Custom modes each yield a complete HighlightScope -> Color32 mapping
//!   with NO missing scope (iterates every variant for each mode).
//!
//! These are integration-level proofs over the PUBLIC crate API (the live resolver MT-001's color path
//! delegates through), complementing the in-module unit tests. They use NO Color32 literals outside the
//! sanctioned theme module: the expected colors are read from the SAME `STANDARD_PALETTE` / `MUTED_PALETTE`
//! the resolver uses, so the proof is honest (it compares against the real table, not a re-typed guess).

use handshake_native::code_editor::{resolve_scope_color, HighlightScope};
use handshake_native::theme::{MUTED_PALETTE, STANDARD_PALETTE};
use handshake_native::workspace_settings::{SyntaxPalette, SyntaxPaletteMode};

/// AC-004: every mode maps EVERY HighlightScope to a concrete color — no gap, no panic. For the built-in
/// modes, the color matches the corresponding built-in table; Custom (no overrides) falls back to
/// Standard for every scope.
#[test]
fn every_mode_resolves_every_scope() {
    // Muted.
    let muted = SyntaxPalette {
        mode: SyntaxPaletteMode::Muted,
        custom: Default::default(),
    };
    for scope in HighlightScope::ALL.iter().copied() {
        assert_eq!(
            resolve_scope_color(scope, &muted),
            scope.builtin_color(&MUTED_PALETTE),
            "Muted mode resolves {scope:?} to the Muted table color"
        );
    }
    // Standard.
    let standard = SyntaxPalette {
        mode: SyntaxPaletteMode::Standard,
        custom: Default::default(),
    };
    for scope in HighlightScope::ALL.iter().copied() {
        assert_eq!(
            resolve_scope_color(scope, &standard),
            scope.builtin_color(&STANDARD_PALETTE),
            "Standard mode resolves {scope:?} to the Standard table color"
        );
    }
    // Custom with NO overrides: every scope falls back to Standard (no gap — AC-004).
    let custom_empty = SyntaxPalette {
        mode: SyntaxPaletteMode::Custom,
        custom: Default::default(),
    };
    for scope in HighlightScope::ALL.iter().copied() {
        assert_eq!(
            resolve_scope_color(scope, &custom_empty),
            scope.builtin_color(&STANDARD_PALETTE),
            "un-overridden Custom {scope:?} falls back to Standard (no missing scope)"
        );
    }
}

/// AC-003: a Custom swatch edit changes the resolved color for that scope in the SAME frame — the call
/// reads the override map live, so mutating it and re-resolving returns the new color with no caching.
#[test]
fn custom_swatch_edit_is_picked_up_live_in_the_same_frame() {
    let mut palette = SyntaxPalette {
        mode: SyntaxPaletteMode::Custom,
        custom: Default::default(),
    };

    // Frame N: Keyword resolves to the Standard fallback.
    let before = resolve_scope_color(HighlightScope::Keyword, &palette);
    assert_eq!(
        before,
        HighlightScope::Keyword.builtin_color(&STANDARD_PALETTE)
    );

    // The user edits the Keyword swatch (the section writes the sRGBA into palette.custom).
    let edited = [0x11, 0x22, 0x33, 0xFF];
    palette.set_custom("keyword", edited);

    // Frame N (same frame, no restart): the resolver returns the NEW color immediately.
    let after = resolve_scope_color(HighlightScope::Keyword, &palette);
    assert_eq!(
        after,
        egui::Color32::from_rgba_unmultiplied(0x11, 0x22, 0x33, 0xFF),
        "AC-003: the Custom Keyword swatch edit changes the resolved color in the same frame"
    );
    assert_ne!(
        after, before,
        "the color actually changed (no caching / restart needed)"
    );

    // Only the edited scope changed; other scopes still resolve to Standard.
    for scope in HighlightScope::ALL.iter().copied() {
        if scope == HighlightScope::Keyword {
            continue;
        }
        assert_eq!(
            resolve_scope_color(scope, &palette),
            scope.builtin_color(&STANDARD_PALETTE),
            "editing Keyword left {scope:?} on the Standard fallback"
        );
    }
}

/// Switching the MODE live changes the resolved colors (the ComboBox selection is visible in the
/// highlighter immediately): the same scope resolves to different colors under Muted vs Standard.
#[test]
fn switching_mode_changes_resolved_colors_live() {
    let mut palette = SyntaxPalette {
        mode: SyntaxPaletteMode::Standard,
        custom: Default::default(),
    };
    let standard_keyword = resolve_scope_color(HighlightScope::Keyword, &palette);

    palette.mode = SyntaxPaletteMode::Muted;
    let muted_keyword = resolve_scope_color(HighlightScope::Keyword, &palette);

    assert_ne!(
        standard_keyword, muted_keyword,
        "switching Standard -> Muted changes the resolved Keyword color in the same frame"
    );
    assert_eq!(
        muted_keyword,
        HighlightScope::Keyword.builtin_color(&MUTED_PALETTE)
    );
}

/// The persisted scope keys round-trip through the resolver: a Custom override keyed by each scope's
/// `scope_key` is resolved back for the right scope (proves the section's key <-> scope mapping the
/// persistence layer relies on is consistent).
#[test]
fn custom_overrides_keyed_by_scope_key_resolve_to_the_right_scope() {
    let mut palette = SyntaxPalette {
        mode: SyntaxPaletteMode::Custom,
        custom: Default::default(),
    };
    // Distinct color per scope so a mis-keyed override would surface as a wrong color.
    for (i, scope) in HighlightScope::ALL.iter().copied().enumerate() {
        let c = (i as u8) * 10 + 1;
        palette.set_custom(scope.scope_key(), [c, c, c, 0xFF]);
    }
    for (i, scope) in HighlightScope::ALL.iter().copied().enumerate() {
        let c = (i as u8) * 10 + 1;
        assert_eq!(
            resolve_scope_color(scope, &palette),
            egui::Color32::from_rgba_unmultiplied(c, c, c, 0xFF),
            "the Custom override keyed by '{}' resolves for {scope:?}",
            scope.scope_key()
        );
    }
}

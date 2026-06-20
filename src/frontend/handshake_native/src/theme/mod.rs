//! Handshake-native theme token system (WP-KERNEL-011 MT-003).
//!
//! All colors in handshake-native come from `HsPalette`. Do not use `Color32` literals
//! outside this module (`theme/palette.rs` and `theme/syntax.rs`). Widget code consumes
//! semantic tokens (`palette.bg`, `palette.accent`, `palette.syntax.keyword`, ...) so the
//! whole app can switch dark/light at runtime and accept per-workspace token overrides
//! without touching widget code. (CONTROL-4: the no-hardcode invariant is grep-enforced
//! by `tests/test_theme.rs`.)
//!
//! Token values are ported verbatim from the legacy React app's CSS custom properties
//! (`app/src/App.css` `:root` for light, `[data-theme='dark']` for dark) so the native
//! shell is visually consistent with the web app it replaces.

pub mod egui_apply;
pub mod palette;
pub mod syntax;

pub use egui_apply::apply_to_ctx;
pub use palette::{parse_color, HsPalette, HsTheme};
pub use syntax::HsSyntaxTokens;

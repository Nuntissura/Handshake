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

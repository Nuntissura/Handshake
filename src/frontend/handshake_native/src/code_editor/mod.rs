//! Native code-editor surface (WP-KERNEL-012 E1 — VS Code parity).
//!
//! MT-001 lays the foundation every later E1 microtask builds on:
//! - [`buffer::TextBuffer`] — the rope-backed, byte-addressed document buffer (sole text owner).
//! - [`highlight`] — the tree-sitter highlight pipeline ([`Highlighter`], [`HighlightScope`],
//!   [`HighlightSpan`], [`LanguageRegistry`]).
//! - [`panel::CodeEditorPanel`] — the egui widget that renders highlighted lines and exposes the
//!   AccessKit nodes a swarm agent addresses.
//!
//! MT-002 adds [`virtual_lines::VirtualLineLayout`] — the viewport-line virtualization calculator —
//! and rewires the panel to paint only the visible line window via `egui::ScrollArea::show_rows`, so
//! a 100k-line file stays under the per-frame budget.
//!
//! MT-003 adds [`cursor::CursorSet`] — the multi-cursor + column/box selection model (the Helix
//! `Selection` shape: a `Vec` of `(anchor, head)` byte-offset [`cursor::Cursor`]s) — and wires
//! Alt+Click / Ctrl+Alt+Up/Down / Alt+Shift drag / Ctrl+D into the panel, painting every caret +
//! selection over the virtualized rows and emitting a capped set of `Role::Caret` AccessKit nodes
//! (`code_editor_cursor_{n}`; the contract named `Role::TextCursor`, which does not exist in accesskit
//! 0.21 — `Caret` is the field-correct caret role) a swarm agent addresses.
//!
//! Later E1 MTs add find/replace (MT-004), folding
//! (MT-005), minimap/outline (MT-006), the gutter (MT-007), the LSP client (MT-008), the diff editor
//! (MT-009), and the Monaco-parity keymap (MT-010) on top of these primitives. They REUSE the WP-011
//! shell modules (`pane_registry`, `split_layout`, `theme/*`, `accessibility/*`, `backend_client`),
//! which this MT also reuses rather than re-creating.

pub mod buffer;
pub mod cursor;
pub mod find_replace;
pub mod highlight;
pub mod panel;
pub mod virtual_lines;

pub use buffer::{BufferError, TextBuffer};
pub use find_replace::{FindEngine, FindQuery, Match, MAX_PATTERN_LEN};
pub use cursor::{
    byte_to_line_col, find_next_occurrence, line_col_to_byte, word_at, Cursor, CursorSet, MoveDir,
    MAX_ACCESSKIT_CURSORS,
};
pub use highlight::{HighlightScope, HighlightSpan, Highlighter, LanguageRegistry, SafeLanguage};
pub use panel::{
    CodeEditorPanel, CodeEditorPaneFactory, FindState, PerfStats, CODE_EDITOR_CURSOR_AUTHOR_PREFIX,
    CODE_EDITOR_FIND_BAR_AUTHOR_ID, CODE_EDITOR_FIND_NEXT_AUTHOR_ID, CODE_EDITOR_FIND_PREV_AUTHOR_ID,
    CODE_EDITOR_PANEL_AUTHOR_ID, CODE_EDITOR_REPLACE_BAR_AUTHOR_ID, CODE_EDITOR_SCROLL_AREA_AUTHOR_ID,
    CODE_EDITOR_TEXT_AUTHOR_ID,
};
pub use virtual_lines::{VirtualLineLayout, OVERSCAN_LINES};

//! Native code-editor surface (WP-KERNEL-012 E1 — VS Code parity).
//!
//! MT-001 lays the foundation every later E1 microtask builds on:
//! - [`buffer::TextBuffer`] — the rope-backed, byte-addressed document buffer (sole text owner).
//! - [`highlight`] — the tree-sitter highlight pipeline ([`Highlighter`], [`HighlightScope`],
//!   [`HighlightSpan`], [`LanguageRegistry`]).
//! - [`panel::CodeEditorPanel`] — the egui widget that renders highlighted lines and exposes the
//!   AccessKit nodes a swarm agent addresses.
//!
//! Later E1 MTs add virtualization (MT-002), multi-cursor (MT-003), find/replace (MT-004), folding
//! (MT-005), minimap/outline (MT-006), the gutter (MT-007), the LSP client (MT-008), the diff editor
//! (MT-009), and the Monaco-parity keymap (MT-010) on top of these primitives. They REUSE the WP-011
//! shell modules (`pane_registry`, `split_layout`, `theme/*`, `accessibility/*`, `backend_client`),
//! which this MT also reuses rather than re-creating.

pub mod buffer;
pub mod highlight;
pub mod panel;

pub use buffer::{BufferError, TextBuffer};
pub use highlight::{HighlightScope, HighlightSpan, Highlighter, LanguageRegistry, SafeLanguage};
pub use panel::{CodeEditorPanel, CodeEditorPaneFactory, CODE_EDITOR_PANEL_AUTHOR_ID, CODE_EDITOR_TEXT_AUTHOR_ID};

//! Native Rust rich-text + knowledge editor surface (WP-KERNEL-012 E2 — Obsidian /
//! Notion / Tiptap parity).
//!
//! This is the second editor pillar (alongside [`crate::code_editor`]) hosted in the
//! WP-KERNEL-011 native shell. It rebuilds, as native Rust, the React Tiptap
//! rich-text editor (KERNEL-009 "Notes") with full feature parity, then interconnects
//! it with the code editor, CKC, and Loom.
//!
//! MT-011 lays the foundation every later E2 microtask binds to:
//! - [`document_model`] — the ProseMirror/Tiptap-style typed block document model on
//!   `ropey`: typed nodes + inline marks, an atomic transform/step system, a bounded
//!   undo/redo history, and DocJson serialization to the backend `content_json`
//!   shape (`rich_document_v1`).
//!
//! Later E2 MTs add the WYSIWYG renderer (MT-012), block structure editing
//! (MT-013), embeds (MT-014), wikilinks/transclusion (MT-015), slash commands,
//! properties, find/replace, daily notes, save-to-format, and draft recovery — all
//! on top of this model, and all REUSING the WP-011 shell modules (`pane_registry`,
//! `split_layout`, `theme/*`, `accessibility/*`, `backend_client`) rather than
//! re-creating shell infrastructure.

pub mod document_model;
pub mod embeds;
pub mod formatting;
pub mod properties;
pub mod renderer;
pub mod slash_commands;
pub mod wikilinks;

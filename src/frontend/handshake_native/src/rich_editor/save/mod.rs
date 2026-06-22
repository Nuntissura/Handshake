//! Save-to-format export + draft/crash recovery + conflict resolution for the native rich-text
//! editor (WP-KERNEL-012 MT-020).
//!
//! This is the E2 save pillar: it makes the editor's in-memory document durable against the
//! backend (the canonical authority) and survivable across a crash, and resolves concurrent-edit
//! conflicts without silent data loss.
//!
//! ## Module map
//!
//! - [`canonical_hash`] — the canonical-JSON SHA-256 of `content_json`, byte-for-byte matching the
//!   backend's `knowledge_canonical_json_sha256` (the draft-hash seam, MC-005).
//! - [`export`] — one-way DocNode-tree walkers producing HTML (self-contained + reference-linked),
//!   Markdown (lossy CommonMark), PlainText, and the ProseMirror JSON projection envelope, with
//!   image size guards (per-image / cumulative / video-never-inlined) and XSS-safe HTML assembly.
//! - [`save_manager`] — canonical save (`PUT /save`) + the optimistic-concurrency 409 conflict
//!   state machine + the Keep-yours destructive-overwrite confirmation (MC-003) + the in-flight
//!   `is_saving` guard (MC-002).
//! - [`draft_manager`] — draft load on mount, the 5s debounced draft upsert (blocked during a
//!   canonical save), draft clear on save/discard, and the recovery state machine.
//! - [`conflict_ui`] — the egui conflict window, the draft-recovery banner, the export format
//!   picker, and the mockable [`conflict_ui::FileSaveSink`] (the real `rfd` dialog runs on a
//!   dedicated thread — HBR-QUIET).
//!
//! Everything REUSES the WP-011 shell layers: the theme palette (no hardcoded hex), the
//! accessibility hook (the contract author_ids, no new registry), and the existing reqwest +
//! tokio + serde stack (no new dependency family except `rfd` for the dialog). The model
//! amendments (TableHeader node, inline-atom undoable transform) live in
//! [`crate::rich_editor::document_model`], anchored to captured real backend shapes.

pub mod canonical_hash;
pub mod conflict_ui;
pub mod draft_manager;
pub mod export;
pub mod save_manager;

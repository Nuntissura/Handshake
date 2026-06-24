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

pub mod daily_notes;
pub mod document_model;
pub mod embeds;
pub mod find_replace;
pub mod formatting;
pub mod properties;
// WP-KERNEL-012 MT-055 (E2): per-document reading/preview mode (the Obsidian "reading view" parity
// item). Defines `ViewMode`, the per-document `ReadingModeStore`, and the Edit|Reading toggle widget.
pub mod reading_mode;
pub mod renderer;
pub mod save;
pub mod slash_commands;
pub mod wikilinks;

/// MT-031 (E5 melt-together): the rich-text editor's thin adapter into the shared
/// [`crate::interop::InteractionBus`]. The rich editor routes its clipboard + command surface through
/// the ONE shared bus rather than owning ad-hoc per-pane clipboard state (the contract's AC-7 rule).
/// These are the concrete `bus.register_command` + `bus.clipboard_write` call sites for the rich-text
/// surface.
///
/// Note on selection text: extracting a flattened plain-text string across the block document tree is a
/// downstream E5 concern (MT-032+ everything-is-a-block addressing); this MT exposes the two robust
/// clipboard-write paths a rich-text pane needs — a PLAIN-TEXT copy (the caller passes the already
/// materialized selected text its renderer produced) and a whole-BLOCK copy as a `loom://` reference.
pub mod interop_adapter {
    use crate::interop::adapters::register_standard_commands;
    use crate::interop::interaction_bus::{
        ClipboardPayload, EditorSurfaceKind, InteractionBus, SharedSelection,
    };
    use crate::pane_registry::PaneId;
    use crate::rich_editor::properties::metadata_client::ClipboardSink;

    /// Register the rich-text surface's melt-together command set into the shared bus (AC-4). Called
    /// once when the rich-text pane mounts.
    pub fn register(bus: &mut InteractionBus) {
        register_standard_commands(bus, EditorSurfaceKind::RichText);
    }

    /// Build the rich-text surface's [`SharedSelection::TextRange`] from the already-materialized
    /// selected text its renderer produced (the rich editor lays out galleys and knows the rendered
    /// run); `start`/`end` are the flat character offsets into that run.
    pub fn text_selection(pane_id: PaneId, start: usize, end: usize, text: impl Into<String>) -> SharedSelection {
        SharedSelection::TextRange {
            pane_id,
            surface: EditorSurfaceKind::RichText,
            start,
            end,
            text: text.into(),
        }
    }

    /// Build a whole-block reference selection (a block copied as a `loom://` ref — the contract's
    /// everything-is-addressable-as-a-Loom-block edge seed).
    pub fn block_ref_selection(pane_id: PaneId, block_id: impl Into<String>) -> SharedSelection {
        SharedSelection::BlockRef { pane_id, block_id: block_id.into() }
    }

    /// Copy a plain-text rich-text selection to the shared clipboard through the bus (Ctrl+C path on a
    /// text run). Returns `true` when non-empty text was copied. OS write goes through the mockable
    /// [`ClipboardSink`] (headless-safe — MT-017 precedent).
    pub fn copy_text_to_bus(bus: &mut InteractionBus, text: &str, sink: &dyn ClipboardSink) -> bool {
        if text.is_empty() {
            return false;
        }
        bus.clipboard_write(ClipboardPayload::PlainText(text.to_owned()), sink);
        true
    }

    /// Copy a whole block as a `loom://` reference to the shared clipboard (the rich-text "copy block
    /// reference" path). The bus caches the rich `LoomBlockRef` variant so a cross-pane Paste recovers
    /// the reference the plain-text OS clipboard would have flattened to its URI string.
    pub fn copy_block_ref_to_bus(bus: &mut InteractionBus, block_id: &str, sink: &dyn ClipboardSink) {
        bus.clipboard_write(ClipboardPayload::LoomBlockRef(block_id.to_owned()), sink);
    }

    /// Read the shared clipboard's text for a rich-text Paste (Ctrl+V path). Returns the richest
    /// cross-pane variant as text, when present.
    pub fn paste_text_from_bus(bus: &InteractionBus) -> Option<String> {
        bus.clipboard_read_text().filter(|t| !t.is_empty())
    }

    /// Publish the rich editor's current selection to the shared bus + register its command set — the
    /// LIVE per-frame wiring [`crate::rich_editor::renderer::rich_editor_widget::RichEditorPaneFactory`]
    /// calls so a MOUNTED rich-text pane is a real bus consumer (not test-only dead code). Registers the
    /// rich-text command set on first call (idempotent — last-registration-wins by id). `has_focus` gates
    /// focus ownership so a background pane never clobbers the focused pane's selection (impl note 6/7).
    /// `selected` is `(block_idx, start_char, end_char, text)` from
    /// [`crate::rich_editor::renderer::rich_editor_widget::RichEditorState::selected_text`]. All bus
    /// access is via `with_try_lock` so it never blocks the egui frame thread (RISK-1 / MC-1).
    pub fn drive_bus_in_render(
        bus: &std::sync::Arc<std::sync::Mutex<InteractionBus>>,
        pane_id: PaneId,
        has_focus: bool,
        selected: Option<(usize, usize, usize, String)>,
        already_registered: &mut bool,
    ) {
        let registered = *already_registered;
        InteractionBus::with_try_lock(bus, |b| {
            if !registered {
                register(b);
            }
            if has_focus {
                b.set_focus_owner(pane_id.clone());
                let selection = match selected {
                    Some((_, start, end, text)) => {
                        text_selection(pane_id.clone(), start, end, text)
                    }
                    None => SharedSelection::None,
                };
                b.set_selection(selection);
            }
        });
        *already_registered = true;
    }

    use crate::undo_stack::{UndoAction, UndoFn, UndoResult};
    use std::sync::{Arc, Mutex, Weak};
    use std::time::{Duration, Instant};

    /// MT-035 (E5 unified undo): the rich-text 500ms batching window (RISK-1 / MC-1). A rich-text doc
    /// snapshot is a `serde_json::Value` clone of the whole block tree — O(n), so a per-keystroke undo
    /// push on a large document would lag. This batcher coalesces rapid edits within
    /// [`RICH_UNDO_BATCH_MS`] (500ms) into a SINGLE undo entry: `should_push` returns `true` only when
    /// the window since the last pushed edit has elapsed (or the edit is the first), so N keystrokes in
    /// the same window produce ONE undo entry, not N. The pane calls `should_push` before
    /// [`push_rich_edit_undo`]; within a window it keeps mutating the SAME tail entry's `after` snapshot
    /// instead (the caller updates the redo target). This is the standard typing-coalescing model VS
    /// Code / ProseMirror use.
    #[derive(Debug, Clone)]
    pub struct RichUndoBatcher {
        last_push: Option<Instant>,
        window: Duration,
    }

    /// The rich-text undo batching window in milliseconds (RISK-1 / MC-1 contract: 500ms).
    pub const RICH_UNDO_BATCH_MS: u64 = 500;

    impl Default for RichUndoBatcher {
        fn default() -> Self {
            Self::new()
        }
    }

    impl RichUndoBatcher {
        /// A batcher with the default 500ms window.
        pub fn new() -> Self {
            Self { last_push: None, window: Duration::from_millis(RICH_UNDO_BATCH_MS) }
        }

        /// A batcher with an explicit window (focused tests use a tiny window).
        pub fn with_window(window: Duration) -> Self {
            Self { last_push: None, window }
        }

        /// True when a NEW undo entry should be pushed for an edit happening `at`: the first edit always
        /// pushes; a later edit pushes only when the window has elapsed since the last push (so rapid
        /// keystrokes coalesce into one entry — RISK-1). On a `true` result the batcher records `at` as
        /// the new last-push time.
        pub fn should_push(&mut self, at: Instant) -> bool {
            let push = match self.last_push {
                None => true,
                Some(prev) => at.duration_since(prev) >= self.window,
            };
            if push {
                self.last_push = Some(at);
            }
            push
        }

        /// Reset the window (e.g. after an explicit save / undo) so the next edit starts a fresh entry.
        pub fn reset(&mut self) {
            self.last_push = None;
        }
    }

    /// A host-supplied applier that writes a content_json snapshot back into the live rich document
    /// state `S` (the rich editor owns the doc-tree shape, so it injects HOW to apply a snapshot). Used
    /// by [`push_rich_edit_undo`]'s undo/redo closures.
    pub type RichSnapshotApplier<S> = Arc<dyn Fn(&mut S, &serde_json::Value) + Send + Sync>;

    /// MT-035 (E5 unified undo): record a LOCAL rich-text-edit undo action on the shared scope for
    /// `pane_id` (POLICY-1). `before`/`after` are content_json (`serde_json::Value`) snapshots of the
    /// block document tree (cloned per the 500ms batching window — RISK-1 / MC-1). The undo_fn restores
    /// `before` into the shared doc state; the redo_fn re-applies `after`. Both capture a `Weak` back-ref
    /// to the doc state the host holds (RISK-3 / MC-3): they upgrade only during invocation and report a
    /// benign [`UndoResult::pane_dropped`] when the pane closed — no retain cycle, no panic. `restore` is
    /// the host-supplied applier that writes a snapshot back into the live document (the rich editor owns
    /// the doc tree shape, so it injects how to apply a snapshot). The rich editor's existing
    /// `document_model::history::UndoManager` stays the in-pane transaction history; THIS bridges that
    /// surface into the ONE unified scope (no second parallel undo stack — wrap-not-fork).
    pub fn push_rich_edit_undo<S>(
        bus: &mut InteractionBus,
        pane_id: PaneId,
        doc_state: &Arc<Mutex<S>>,
        before: serde_json::Value,
        after: serde_json::Value,
        restore: RichSnapshotApplier<S>,
        description: impl Into<String>,
    ) where
        S: Send + 'static,
    {
        let action = rich_edit_undo_action(doc_state, before, after, restore, description);
        bus.push_undo_local(pane_id, action);
    }

    /// MT-035 (E5 unified undo): the LIVE coalescing entry point the mounted rich-text pane calls each
    /// frame an edit landed (RISK-1 / MC-1). Given the [`RichUndoBatcher`] decision (`should_push`):
    /// - a NEW batch (`should_push==true`) PUSHES a fresh undo entry whose `undo_fn` restores `before`
    ///   (the batch-start snapshot) and whose `redo_fn` re-applies `after`;
    /// - a CONTINUATION (`should_push==false`) REPLACES the tail entry's `redo_fn` to re-apply the latest
    ///   `after` while KEEPING `batch_before` as the `undo_fn` snapshot, so N rapid keystrokes coalesce
    ///   into ONE undo entry that reverts the WHOLE burst (never silently dropping the in-between edits).
    ///
    /// `batch_before` is the snapshot captured at the START of the current 500ms batch (the host tracks
    /// it); `before` is the snapshot just before THIS frame's edit (used only on a fresh push). On a
    /// fresh push the host should set `batch_before = before` for the next continuation. Returns `true`
    /// when a fresh entry was pushed (so the host resets its batch-before tracking), `false` when the
    /// edit coalesced into the existing tail.
    #[allow(clippy::too_many_arguments)]
    pub fn push_or_coalesce_rich_edit_undo<S>(
        bus: &mut InteractionBus,
        pane_id: PaneId,
        doc_state: &Arc<Mutex<S>>,
        should_push: bool,
        batch_before: serde_json::Value,
        before: serde_json::Value,
        after: serde_json::Value,
        restore: RichSnapshotApplier<S>,
        description: impl Into<String>,
    ) -> bool
    where
        S: Send + 'static,
    {
        if should_push {
            let action = rich_edit_undo_action(doc_state, before, after, restore, description);
            bus.push_undo_local(pane_id, action);
            true
        } else {
            // Coalesce: rebuild the tail entry with the batch-start `before` and the latest `after`.
            let action =
                rich_edit_undo_action(doc_state, batch_before, after, restore, description);
            // If there is no tail yet (first edit of the very first batch raced the batcher), fall back
            // to a push so the edit is never lost from history.
            if !bus.replace_undo_local_tail(&pane_id, action.clone()) {
                bus.push_undo_local(pane_id, action);
                return true;
            }
            false
        }
    }

    /// Build the [`UndoAction`] a rich-text edit records: `undo_fn` restores `before`, `redo_fn`
    /// re-applies `after`, both through the host `restore` applier via a `Weak` back-ref (RISK-3 / MC-3).
    fn rich_edit_undo_action<S>(
        doc_state: &Arc<Mutex<S>>,
        before: serde_json::Value,
        after: serde_json::Value,
        restore: RichSnapshotApplier<S>,
        description: impl Into<String>,
    ) -> UndoAction
    where
        S: Send + 'static,
    {
        let weak: Weak<Mutex<S>> = Arc::downgrade(doc_state);
        let undo_weak = weak.clone();
        let undo_restore = restore.clone();
        let undo_fn: UndoFn = Arc::new(move || match undo_weak.upgrade() {
            Some(state) => {
                let mut guard = state.lock().unwrap_or_else(|e| e.into_inner());
                undo_restore(&mut guard, &before);
                UndoResult::ok()
            }
            None => UndoResult::pane_dropped(),
        });
        let redo_fn: UndoFn = Arc::new(move || match weak.upgrade() {
            Some(state) => {
                let mut guard = state.lock().unwrap_or_else(|e| e.into_inner());
                restore(&mut guard, &after);
                UndoResult::ok()
            }
            None => UndoResult::pane_dropped(),
        });
        UndoAction::sync(description, undo_fn, redo_fn)
    }
}

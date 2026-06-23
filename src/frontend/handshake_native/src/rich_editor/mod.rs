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
}

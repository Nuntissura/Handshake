//! The top-level WYSIWYG editor widget + its pane factory (WP-KERNEL-012 MT-012).
//!
//! [`RichEditorState`] owns the editable document ([`BlockNode`] doc), the
//! [`Selection`], the [`UndoManager`], and the IME [`PreeditState`]. [`RichEditorWidget`]
//! is a thin per-frame view over a shared `Arc<Mutex<RichEditorState>>` that:
//! 1. reads input events and applies them via [`input_handler`] / [`ime_handler`],
//! 2. paints each block via [`block_renderer`] inside a vertical [`egui::ScrollArea`],
//! 3. resolves + paints the blinking caret natively from the caret block's galley,
//! 4. emits the AccessKit tree (root `rich-editor-root` + per-block `re-block-{hash}`)
//!    through the SAME `accesskit_node_builder` hook the WP-011 shell uses,
//! 5. drives the blink repaint ONLY while focused (RISK-3 idle-CPU guard).
//!
//! HBR-QUIET: the widget never calls `request_user_attention` / any OS focus grab; it
//! only ever requests an egui repaint (and only when focused). The
//! `no_focus_steal_calls` source test in `tests/test_rich_editor_widget.rs` greps this
//! module to prove it.
//!
//! Shell reuse: [`RichEditorPaneFactory`] implements the WP-011 `PaneFactory` so the
//! editor mounts through the EXISTING `pane_registry` / `split_layout` host
//! (`PaneHostWidget`) — the same seam the code editor uses, no shell fork.

use std::sync::{Arc, Mutex};

use egui::accesskit;

use crate::accessibility; // REUSE the WP-011 a11y layer (live emission helpers).
use crate::accessibility::editor_action_registry::{
    rich_action_catalog, rich_heading_is_unsupported, AxRole, EditorActionRegistry,
    EditorActionState, PaneType as EditorPaneType, RegistrationHandle, RichDispatch,
};
use crate::pane_registry::{PaneFactory, PaneRenderContext, PaneType};
use crate::rich_editor::document_model::history::UndoManager;
use crate::rich_editor::document_model::node::{BlockNode, Child, NodeKind};
use crate::rich_editor::document_model::position::DocPosition;
use crate::rich_editor::document_model::selection::Selection;
use crate::theme::{HsPalette, HsTheme};

use super::block_renderer::{block_media_embed, paint_caret, paint_block};
use super::caret::{blink_visible, request_blink_repaint, DocCaret};
use super::ime_handler::{self, ImeContext, PreeditState};
use super::input_handler::{self, EditContext};
use super::{block_author_id, block_node_id, root_egui_id, BLOCK_ROLE, RICH_EDITOR_ROOT_AUTHOR_ID, ROOT_ROLE};
use crate::rich_editor::embeds::asset_resolver::ReqwestAssetFetcher;
use crate::rich_editor::embeds::embed_block_renderer::{self, EmbedRuntime};

/// The persistent, mutable editor state. Held behind an `Arc<Mutex<>>` by the owner
/// (`HandshakeApp` / a test) so the per-frame [`RichEditorWidget`] borrows it; the model
/// types are single-threaded-friendly (no internal locks) per MT-011's design.
pub struct RichEditorState {
    /// The document tree (the `doc` root).
    pub doc: BlockNode,
    /// The current selection (caret = collapsed).
    pub selection: Selection,
    /// Bounded undo/redo history.
    pub undo: UndoManager,
    /// In-progress IME composition overlay.
    pub preedit: PreeditState,
    /// Active theme (resolved to a palette each frame).
    pub theme: HsTheme,
    /// Actor id for transaction provenance.
    pub actor_id: String,
    /// MT-014 embed runtime: per-editor asset-resolution + texture caches, slideshow/album/video
    /// view states, and the async transport. Owned HERE (the shell frame) so resolved assets +
    /// uploaded textures + paging persist across frames (impl note 5: NOT inside a renderer fn).
    pub embeds: EmbedRuntime,
    /// MT-015 wikilink runtime: transclusion-resolution cache, backlinks state (+ generation-counter
    /// cancellation), the autocomplete search runtime, and the off-thread delivery cells. Owned HERE
    /// so resolved transclusions/backlinks + popup state persist across frames.
    pub wikilinks: crate::rich_editor::wikilinks::runtime::WikilinkRuntime,
    /// MT-015 active wikilink autocomplete popup (`Some` while the operator is typing a `[[` trigger).
    pub wikilink_autocomplete: Option<crate::rich_editor::wikilinks::autocomplete::AutocompleteState>,
    /// MT-015 editor events enqueued for the WP-011 host shell to drain + route (WikilinkActivated /
    /// BacklinkActivated / TransclusionOpenRequested). This MT only ENQUEUES; routing is owned by the
    /// shell (E11/MT-069). The shell drains this each frame after the editor renders.
    pub pending_events: Vec<crate::rich_editor::wikilinks::inline_view::EditorEvent>,
    /// MT-016 active slash-command menu (`Some` while the operator has an open `/` trigger). Owned HERE
    /// so the popup state (filter / selection / active prompt) persists across frames; closed on focus
    /// loss (MC-004) and on Escape / execute / the `/` being deleted.
    pub slash_menu: Option<crate::rich_editor::slash_commands::SlashMenuState>,
    /// WP-KERNEL-012 MT-058 (E2 — inline `#tag` authoring): the active inline-tag autocomplete popup
    /// (`Some` while the operator is typing a `#` trigger at a word boundary). Owned HERE so the trigger
    /// span / query / selection persist across frames; closed on Escape / commit / the `#` being deleted
    /// or the body ending. Mutually exclusive with the wikilink autocomplete + slash menu.
    pub tag_autocomplete: Option<crate::rich_editor::inline_tags::TagAutocompleteState>,
    /// WP-KERNEL-012 MT-058: the cached MT-023 tag-hub list (display names) the `#` autocomplete menu
    /// sources its existing-tag rows from (`GET /loom/tags` — the VERIFIED route). Refreshed by the shell
    /// when the menu opens; a free-typed NEW tag is always committable regardless (AC-006). Empty until
    /// the shell installs the tag list (a unit/kittest seeds it directly).
    pub tag_list: Vec<String>,
    /// WP-KERNEL-012 MT-034 (E5 — code<->note cross-refs): the open `/code-ref` code-symbol search
    /// dialog (`Some` while the operator is picking a symbol). Owned HERE so the query + results + load
    /// state persist across frames; closed on select / cancel. The workspace id + runtime it needs are
    /// installed by [`Self::set_code_ref_context`] (reusing the wikilink workspace context by default).
    pub code_symbol_search:
        Option<crate::rich_editor::slash_commands::code_symbol_search::CodeSymbolSearchState>,
    /// WP-KERNEL-012 MT-034: the workspace id the `/code-ref` dialog scopes its symbol lookup to (set
    /// alongside the wikilink context by [`Self::set_code_ref_context`] / [`Self::set_workspace_context`]).
    pub code_ref_workspace_id: String,
    /// WP-KERNEL-012 MT-034: the tokio runtime handle the `/code-ref` lookup spawns onto (`None` until
    /// the shell installs it; a headless test drives the dialog state directly without it).
    pub code_ref_runtime: Option<tokio::runtime::Handle>,
    /// MT-017 document properties: the per-document metadata panel state (`None` until a document's
    /// metadata loads — the panel renders an honest "no document loaded" placeholder until then). Owned
    /// HERE so the title edit buffer + local tag list + collapsed state persist across frames.
    pub properties: Option<crate::rich_editor::properties::PropertiesState>,
    /// MT-017 properties async runtime: the title-save (rename) dispatch state + the backlinks-count
    /// state with MC-004 generation cancellation + the off-thread delivery cells. Owned HERE so the
    /// save/count state persists across frames; the count NEVER overwrites `properties.doc_metadata`.
    pub properties_runtime: crate::rich_editor::properties::metadata_client::PropertiesRuntime,
    /// MT-018 find/replace: the open find/replace panel state (`None` until Ctrl+F / Ctrl+H opens it).
    /// Owned HERE so the query + scan result + active match + replacement persist across frames; the
    /// scan is recomputed on open, on every query change, and after every document mutation while the
    /// panel is open. Cleared (with all highlights) on Escape / close.
    pub find_replace: Option<crate::rich_editor::find_replace::FindReplaceState>,
    /// MT-020 save/draft/conflict coordinator (`None` until the shell installs the document's save
    /// context via [`Self::set_save_context`] — until then Ctrl+S is a no-op and no draft is checked).
    /// Owns the canonical-save state machine (incl. the 409 conflict + Keep-yours confirmation), the
    /// `is_saving` guard, the `doc_version`, and the dirty flag.
    pub save: Option<crate::rich_editor::save::save_manager::SaveManager>,
    /// MT-020 draft / crash-recovery coordinator (`None` until [`Self::set_save_context`]). Owns the
    /// draft state machine (load on mount, 5s debounced upsert, clear on save/discard) + the recovery
    /// banner state.
    pub draft: Option<crate::rich_editor::save::draft_manager::DraftManager>,
    /// MT-020 export format-picker popup open flag (the operator clicked "Export…"). Owned HERE so the
    /// popup persists across frames until a format is chosen or it is dismissed.
    pub export_picker_open: bool,
    /// MT-020 in-flight native save-dialog handle (`Some` while the operator has an export's OS save
    /// dialog open). The dialog runs on a dedicated thread (HBR-QUIET / MC-004); this handle is POLLED
    /// non-blockingly each frame in [`RichEditorWidget::drive_save_and_draft`] so the egui frame thread
    /// NEVER blocks while the dialog is open (red-team RISK-4 — the frame-freeze fix). Cleared once the
    /// dialog resolves (path chosen / cancelled).
    pub pending_file_save: Option<crate::rich_editor::save::conflict_ui::PendingFileSave>,
    /// WP-KERNEL-012 MT-035 (E5 — unified undo): the pane id this editor's undo entries are recorded
    /// under on the shared [`crate::interop::InteractionBus`] (POLICY-1 local-first). `None` until the
    /// pane mounts and [`RichEditorPaneFactory::render`] installs the live pane id; a bare unit test that
    /// does not exercise undo leaves it unset (the bus undo wiring is then a no-op, never a fake).
    pub undo_pane_id: Option<crate::pane_registry::PaneId>,
    /// WP-KERNEL-012 MT-035: the 500ms typing-coalescing batcher (RISK-1 / MC-1). Decides whether a
    /// frame's edit starts a NEW unified-undo entry or coalesces into the current one so a burst of
    /// keystrokes is ONE undo, not N. Lives HERE so the window persists across frames.
    pub undo_batcher: crate::rich_editor::interop_adapter::RichUndoBatcher,
    /// WP-KERNEL-012 MT-035: the content_json snapshot captured at the START of the current undo batch
    /// (the `before` an in-window coalesced entry restores). `None` between batches; set on a fresh push
    /// and consumed/kept while the batcher window is open.
    pub undo_batch_before: Option<serde_json::Value>,
    /// WP-KERNEL-012 MT-041 (E7): the installed consolidated editor-action AccessKit wiring — the shared
    /// [`EditorActionRegistry`] this rich pane writes its canonical `editor.rich.<action>` nodes into,
    /// plus its stable instance handle. `None` until the host (or a kittest) installs one via
    /// [`install_editor_action_registry`](RichEditorState::install_editor_action_registry); when present,
    /// every `show` syncs + emits + consumes through it (the ONE swarm-facing rich-action surface,
    /// consolidating — not re-minting — the toolbar/find/save author_ids).
    pub editor_actions:
        Option<(std::sync::Arc<std::sync::Mutex<EditorActionRegistry>>, RegistrationHandle)>,
    /// WP-KERNEL-012 MT-056 (E2 — outline/TOC): a pending scroll target requested by the outline panel
    /// (or any caller of [`RichEditorWidget::scroll_to_block`]). Holds the top-level block path (`[idx]`)
    /// the next render pass must bring into view. The block renderer ([`RichEditorWidget::render_blocks`])
    /// consumes it on the next frame by calling `ui.scroll_to_rect` over that block's painted rect, then
    /// clears it (a one-shot request — egui only needs one frame to bring the rect into view). `None`
    /// between requests so steady-state frames do no scroll work. This is the editor's EXISTING scroll
    /// surface (the `rich-editor-scroll` ScrollArea), NOT a second scroll mechanism (RISK-002 / MC-002).
    pub pending_scroll_block: Option<Vec<usize>>,
}

impl RichEditorState {
    /// A new editor over `doc`, caret at the document start.
    ///
    /// The MT-014 embed runtime defaults to the PRODUCTION reqwest fetcher
    /// ([`ReqwestAssetFetcher`]) against the standard backend base, with NO tokio handle and an
    /// empty workspace id. The shell installs the real workspace id + runtime handle via
    /// [`Self::set_embed_context`] when it mounts the editor (so an embed actually resolves);
    /// a unit/kittest that does not exercise resolution leaves it as-is (an embed then shows the
    /// fail-closed resolving spinner / typed chip, never a panic). A test may inject a mock
    /// fetcher via [`Self::with_embed_runtime`].
    pub fn new(doc: BlockNode) -> Self {
        let base = crate::backend_client::BACKEND_BASE_URL;
        let embeds = EmbedRuntime::new(
            String::new(),
            base,
            std::sync::Arc::new(ReqwestAssetFetcher::new(base)),
            None,
        );
        // MT-015: the wikilink runtime defaults to the PRODUCTION reqwest backend, no tokio handle,
        // and an empty workspace/document. The shell installs the live workspace id + runtime handle
        // + document id via [`Self::set_wikilink_context`] when it mounts a document (so transclusion/
        // backlinks/autocomplete actually resolve); a unit/kittest that does not exercise resolution
        // leaves it as-is (a transclusion then shows the fail-closed spinner / typed chip, never a
        // panic). A test injects a mock backend via [`Self::with_wikilink_runtime`].
        let wikilinks = crate::rich_editor::wikilinks::runtime::WikilinkRuntime::new(
            String::new(),
            std::sync::Arc::new(
                crate::rich_editor::wikilinks::client::ReqwestWikilinkBackend::new(base),
            ),
            None,
        );
        // MT-017: the properties runtime defaults to the PRODUCTION reqwest metadata backend, no tokio
        // handle. The shell installs the live workspace/document + runtime handle via
        // [`Self::set_properties_context`] when it loads a document's metadata (so a title rename +
        // backlinks-count actually dispatch); a unit/kittest that does not exercise the backend leaves
        // it as-is (no perpetual Saving/Loading without a runtime — the no-spinner discipline). A test
        // injects a mock backend + seeded PropertiesState via [`Self::with_properties`].
        let properties_runtime =
            crate::rich_editor::properties::metadata_client::PropertiesRuntime::new(
                std::sync::Arc::new(
                    crate::rich_editor::properties::metadata_client::ReqwestMetadataBackend::new(base),
                ),
                None,
            );
        Self {
            doc,
            selection: Selection::caret(DocPosition::new(vec![0, 0], 0)),
            undo: UndoManager::new(),
            preedit: PreeditState::default(),
            theme: HsTheme::Dark,
            actor_id: "operator".to_owned(),
            embeds,
            wikilinks,
            wikilink_autocomplete: None,
            pending_events: Vec::new(),
            slash_menu: None,
            tag_autocomplete: None,
            tag_list: Vec::new(),
            code_symbol_search: None,
            code_ref_workspace_id: String::new(),
            code_ref_runtime: None,
            properties: None,
            properties_runtime,
            find_replace: None,
            save: None,
            draft: None,
            export_picker_open: false,
            pending_file_save: None,
            undo_pane_id: None,
            undo_batcher: crate::rich_editor::interop_adapter::RichUndoBatcher::new(),
            undo_batch_before: None,
            editor_actions: None,
            pending_scroll_block: None,
        }
    }

    /// WP-KERNEL-012 MT-058: install the cached MT-023 tag list (display names) the inline-tag `#`
    /// autocomplete menu sources its existing-tag rows from (`GET /loom/tags` — the VERIFIED route the
    /// shell fetches via [`crate::backend_client::LoomTagClient`]). The shell calls this when the editor
    /// mounts / when the menu opens; a free-typed NEW tag is always committable regardless (AC-006), so
    /// an empty list never blocks tag authoring.
    pub fn set_tag_list(&mut self, tags: Vec<String>) {
        self.tag_list = tags;
    }

    /// WP-KERNEL-012 MT-058: collect every distinct inline-tag display name in document order from the
    /// committed `Child::HsLink(ref_kind="tag")` atoms in the document tree. This is the INLINE half of
    /// the convergence union (RISK-004 / MC-004) — the caller unions it with the MT-017 property-panel
    /// tag set via [`crate::rich_editor::inline_tags::build_tag_edge_payload`] at document COMMIT/SAVE
    /// (never per keystroke). Each name is the original-case display (recovered from the chip label), so
    /// the convergence builder normalizes them to dedupe.
    pub fn collect_inline_tags(&self) -> Vec<String> {
        fn walk(block: &BlockNode, out: &mut Vec<String>) {
            for child in &block.children {
                match child {
                    Child::HsLink(link)
                        if crate::rich_editor::inline_tags::is_tag_link(link) =>
                    {
                        out.push(crate::rich_editor::inline_tags::tag_from_link(link).name);
                    }
                    Child::Block(b) => walk(b, out),
                    _ => {}
                }
            }
        }
        let mut out = Vec::new();
        walk(&self.doc, &mut out);
        out
    }

    /// WP-KERNEL-012 MT-058: build the deduped convergence tag-edge payload for a document COMMIT/SAVE —
    /// the union of this document's inline tags ([`Self::collect_inline_tags`]) with the MT-017
    /// property-panel tag set, deduped by normalized identity (one tag, one hub — RISK-004 / MC-004 /
    /// AC-005). This is the payload the shell persists via `POST /loom/edges` (one edge per distinct
    /// canonical tag) on save; the LIVE POST is gated on a managed PostgreSQL + per-canonical tag_hub
    /// resolution (NEEDS_MANAGED_RESOURCE_PROOF), but the deduped builder output is provable standalone.
    /// The property tags are read from the live [`crate::rich_editor::properties::PropertiesState::tags`]
    /// when present (MT-017's local-only list — see TB-058-NORMALIZE on its persistence gap).
    pub fn build_tag_edge_payload_for_save(
        &self,
    ) -> crate::rich_editor::inline_tags::TagEdgePayload {
        let inline = self.collect_inline_tags();
        let property: Vec<String> = self
            .properties
            .as_ref()
            .map(|p| p.tags.clone())
            .unwrap_or_default();
        crate::rich_editor::inline_tags::build_tag_edge_payload(inline, property)
    }

    /// WP-KERNEL-012 MT-041 (E7): install the shared [`EditorActionRegistry`] this rich pane registers
    /// its canonical `editor.rich.<action>` nodes into. `instance_index` is the pane's stable 0-based
    /// index (>0 suffixes the author_ids `.<idx>` — RISK-041-05). Idempotent.
    pub fn install_editor_action_registry(
        &mut self,
        registry: std::sync::Arc<std::sync::Mutex<EditorActionRegistry>>,
        instance_index: usize,
    ) {
        let handle = {
            let mut reg = registry.lock().unwrap_or_else(|e| e.into_inner());
            reg.register(EditorPaneType::Rich, instance_index)
        };
        self.editor_actions = Some((registry, handle));
    }

    /// MT-020: install the live save/draft context — the production save + draft managers bound to
    /// the active document at `doc_version`, dispatching off the app's tokio runtime. The shell calls
    /// this when it mounts a document (the production wiring point). `base_content` seeds the draft
    /// base hash (the server content the edits fork from); the draft load is triggered on mount.
    pub fn set_save_context(
        &mut self,
        document_id: impl Into<String>,
        doc_version: u64,
        runtime: tokio::runtime::Handle,
    ) {
        let document_id = document_id.into();
        let base_content =
            crate::rich_editor::document_model::doc_json::to_content_json_value(&self.doc);
        let save = crate::rich_editor::save::save_manager::SaveManager::production(
            runtime.clone(),
            document_id.clone(),
            doc_version,
        );
        let mut draft = crate::rich_editor::save::draft_manager::DraftManager::production(
            runtime,
            document_id,
            doc_version,
            &base_content,
        );
        draft.check_on_mount();
        self.save = Some(save);
        self.draft = Some(draft);
    }

    /// True when this pane has an installed MT-020 save context (a [`SaveManager`] is bound). The host
    /// menu Save predicate / wiring consults this to decide whether to invoke the real save path or
    /// install a save context first.
    pub fn has_save_context(&self) -> bool {
        self.save.is_some()
    }

    /// True while the MT-020 [`SaveManager`] has a canonical save in flight (`SaveState::Saving`). This
    /// is the SaveManager's OWN state machine (set inside `request_save`), not a host-set marker — the
    /// menu Save proof asserts THIS to show the dispatch reached the real MT-020 save entry.
    pub fn save_is_in_flight(&self) -> bool {
        self.save.as_ref().map(|s| s.is_saving()).unwrap_or(false)
    }

    /// WP-KERNEL-012 MT-069: trigger a canonical save of the live document through the MT-020
    /// [`SaveManager`] — the SAME entry point the rich editor's own Ctrl+S ([`RichDispatch::Save`] ->
    /// `trigger_save`) reaches. Captures the current `content_json` from the doc (never a stale
    /// snapshot), records it as the pending local content (so a resulting 409 conflict carries the
    /// operator's version), and calls [`SaveManager::request_save`], which moves the manager to
    /// `SaveState::Saving` and (with a runtime) spawns the real `PUT /knowledge/documents/{id}/save`
    /// backend call off the frame thread. Returns `true` when the save entry was reached (a save context
    /// is installed), `false` when no save context exists (the honest "save not yet wireable" path the
    /// host keeps the leaf disabled for). No shell-local / SQLite write — the SaveManager owns the
    /// handshake_core PostgreSQL/EventLedger write (MC-004 / RISK-004).
    pub fn request_save_for_host(&mut self) -> bool {
        let content =
            crate::rich_editor::document_model::doc_json::to_content_json_value(&self.doc);
        if let Some(save) = self.save.as_mut() {
            save.set_pending_local_content(content.clone());
            save.request_save(content);
            true
        } else {
            false
        }
    }

    /// TEST SEAM: install pre-built save + draft managers (with mock backends + no runtime) so the
    /// conflict window / draft banner / Ctrl+S flow can be exercised headlessly.
    pub fn with_save_managers(
        mut self,
        save: crate::rich_editor::save::save_manager::SaveManager,
        draft: crate::rich_editor::save::draft_manager::DraftManager,
    ) -> Self {
        self.save = Some(save);
        self.draft = Some(draft);
        self
    }

    /// Install the live wikilink context: the workspace + document the transclusion/backlinks/
    /// autocomplete resolve against, and the tokio runtime handle resolutions spawn onto. The shell
    /// calls this when it knows the active document (the production wiring point). Setting the
    /// document triggers a backlinks generation bump (MC-004) so a prior document's in-flight
    /// backlinks response is dropped.
    ///
    /// WP-KERNEL-012 MT-057 wiring (the create-from-unresolved + alias-resolution feature goes LIVE
    /// HERE — this is the single documented production-wiring point):
    ///   1. installs the production create backend ([`KnowledgeCreateNoteBackend`] -> the MT-037
    ///      `POST /knowledge/documents` binding) so a click on an unresolved `[[Title]]` actually
    ///      POSTs (without this, `create_backend` is None and `dispatch_create_note` no-ops);
    ///   2. SEEDS the resolver index from the EXISTING MT-038 Loom search binding (so a `[[Title]]`
    ///      classifies Resolved at runtime — AC-003 — instead of always-Unresolved against an empty
    ///      index); and
    ///   3. flips the alias-backend-gap banner ([`note_alias_backend_gap`]) because the backend payload
    ///      carries NO `aliases` field (grep-confirmed), so any in-session aliases are local-only and
    ///      the operator must see the VISIBLE local-only banner (AC-006).
    pub fn set_wikilink_context(
        &mut self,
        workspace_id: impl Into<String>,
        document_id: impl Into<String>,
        runtime: tokio::runtime::Handle,
    ) {
        let workspace_id = workspace_id.into();
        let document_id = document_id.into();
        self.wikilinks.workspace_id = workspace_id.clone();
        self.wikilinks.autocomplete.workspace_id = workspace_id;
        self.wikilinks.runtime = Some(runtime.clone());
        self.wikilinks.autocomplete.runtime = Some(runtime.clone());
        self.wikilinks.set_document(document_id.clone());
        // WP-KERNEL-012 MT-057 (1): install the production create backend so the create-from-unresolved
        // POST path is LIVE. The session run id folds the document id (matching the MT-037 attribution
        // convention `native-editor-doc-{id}`), so each create is attributable (HBR-SWARM).
        self.wikilinks.set_create_backend(std::sync::Arc::new(
            crate::rich_editor::wikilinks::runtime::KnowledgeCreateNoteBackend::production(format!(
                "native-editor-{document_id}"
            )),
        ));
        // WP-KERNEL-012 MT-057 (2): seed the resolver index from the EXISTING Loom search binding so a
        // `[[Title]]` resolves at runtime (AC-003). A broad empty query lists the workspace's blocks by
        // the backend's FTS; the (block_id, title) pairs land off-thread and `drain` folds them in. No
        // new endpoint — read-throughs the same `search()` the autocomplete dropdown uses.
        self.wikilinks.seed_resolver_index_from_search(
            "",
            crate::rich_editor::wikilinks::autocomplete::RESOLVER_SEED_LIMIT,
        );
        // WP-KERNEL-012 MT-057 (3): the backend payload has NO `aliases` field, so flag the alias
        // backend gap -> the editor renders the VISIBLE local-only banner whenever an in-session alias
        // is in play (AC-006). Idempotent; the flag also flips on the first `add_local_alias`.
        self.wikilinks.note_alias_backend_gap();
        // MT-034: the `/code-ref` symbol search shares the same workspace + runtime context.
        self.code_ref_workspace_id = self.wikilinks.workspace_id.clone();
        self.code_ref_runtime = Some(runtime);
    }

    /// WP-KERNEL-012 MT-034: install the live `/code-ref` code-symbol search context (the workspace the
    /// symbol lookup scopes to + the tokio runtime it spawns onto). The shell may call this directly; it
    /// is also set as a side effect of [`Self::set_wikilink_context`] (the editors share one workspace).
    pub fn set_code_ref_context(
        &mut self,
        workspace_id: impl Into<String>,
        runtime: tokio::runtime::Handle,
    ) {
        self.code_ref_workspace_id = workspace_id.into();
        self.code_ref_runtime = Some(runtime);
    }

    /// Replace the entire wikilink runtime (test seam: inject a mock backend / pre-seeded caches).
    pub fn with_wikilink_runtime(mut self, wikilinks: crate::rich_editor::wikilinks::runtime::WikilinkRuntime) -> Self {
        self.wikilinks = wikilinks;
        self
    }

    /// MT-017: install the live properties context — the loaded document metadata (so the panel shows
    /// the real fields), the document id (so the backlinks-count + rename dispatch target the right
    /// document), and the tokio runtime handle dispatches spawn onto. The shell calls this when it
    /// loads a document's metadata (the production wiring point). Setting a different document bumps the
    /// backlinks-count generation (MC-004) so a prior document's in-flight count is dropped.
    pub fn set_properties_context(
        &mut self,
        doc_metadata: crate::rich_editor::properties::metadata_client::DocMetadata,
        runtime: tokio::runtime::Handle,
    ) {
        let document_id = doc_metadata.rich_document_id.clone();
        match self.properties.as_mut() {
            Some(p) => p.set_metadata(doc_metadata),
            None => self.properties = Some(crate::rich_editor::properties::PropertiesState::new(doc_metadata)),
        }
        self.properties_runtime.runtime = Some(runtime);
        self.properties_runtime.set_document(document_id);
    }

    /// Replace the properties state + runtime (test seam: inject a seeded `PropertiesState` + a mock
    /// metadata backend so the panel renders real fields and the title-save round-trips headlessly).
    pub fn with_properties(
        mut self,
        properties: crate::rich_editor::properties::PropertiesState,
        runtime: crate::rich_editor::properties::metadata_client::PropertiesRuntime,
    ) -> Self {
        self.properties_runtime = runtime;
        self.properties = Some(properties);
        self
    }

    /// MT-017 / MC-001: the CURRENT document content as the backend `content_json` BARE doc node, pulled
    /// LIVE from `self.doc` (NOT a cached copy). The title-save path uses the `/rename` endpoint (which
    /// never sends a content body, so it cannot clobber content), but this accessor exists so a future
    /// content-bearing save and the MC-001 test can read the live content (proving the editor never
    /// holds a stale snapshot for save purposes).
    pub fn current_content_json(&self) -> serde_json::Value {
        crate::rich_editor::document_model::doc_json::to_content_json_value(&self.doc)
    }

    /// Install the live embed context: the workspace whose assets embeds resolve against and the
    /// tokio runtime handle resolutions spawn onto. The shell calls this when it knows the active
    /// document's workspace (the production wiring point). Replaces the embed runtime's
    /// workspace/runtime while keeping the production fetcher.
    pub fn set_embed_context(&mut self, workspace_id: impl Into<String>, runtime: tokio::runtime::Handle) {
        self.embeds.workspace_id = workspace_id.into();
        self.embeds.runtime = Some(runtime);
    }

    /// Replace the entire embed runtime (test seam: inject a mock fetcher / pre-seeded caches).
    pub fn with_embed_runtime(mut self, embeds: EmbedRuntime) -> Self {
        self.embeds = embeds;
        self
    }

    /// A demo document for the AC-1 vertical-slice proof: an h1 heading, then a paragraph
    /// "Hello world" with the word "world" bold. The kittest screenshot renders this.
    pub fn demo() -> Self {
        use crate::rich_editor::document_model::node::{Mark, TextLeaf};
        let heading = BlockNode::heading(1, "Heading One");
        let para = BlockNode::with_children(
            NodeKind::Paragraph,
            vec![
                Child::Text(TextLeaf::new("Hello ")),
                Child::Text(TextLeaf::with_marks("world", vec![Mark::Bold])),
            ],
        );
        let doc = BlockNode::doc(vec![heading, para]);
        // Caret at the end of the paragraph's bold run so the demo shows a live caret.
        let mut s = Self::new(doc);
        s.selection = Selection::caret(DocPosition::new(vec![1, 1], 5));
        s
    }

    /// The resolved palette for the active theme (no overrides in this MT).
    pub fn palette(&self) -> HsPalette {
        self.theme.palette()
    }

    /// MT-031 (E5 melt-together): the currently selected text as `(block_idx, start, end, text)` when the
    /// selection is a non-collapsed text range WITHIN A SINGLE BLOCK, else `None`. Cross-block / node
    /// selections flatten to a whole-document plain string only in a downstream E5 MT (MT-032+ block
    /// addressing); this MT publishes the single-block text range (the common Copy case) so a cross-pane
    /// Copy from the rich editor carries the real selected text. The substring is sliced from the block's
    /// plain text by CHAR OFFSET (the rich model is char-indexed), clamped so a stale offset never panics.
    pub fn selected_text(&self) -> Option<(usize, usize, usize, String)> {
        let (anchor, head) = match &self.selection {
            Selection::Text { anchor, head } if anchor != head => (anchor, head),
            _ => return None,
        };
        // Only a within-one-block range is materialized here (the block index is the path's first hop).
        let block_idx = *anchor.path.first()?;
        if head.path.first() != Some(&block_idx) {
            return None;
        }
        let block_text = self.block_plain_text(block_idx)?;
        let char_len = block_text.chars().count();
        let lo = anchor.char_offset.min(head.char_offset).min(char_len);
        let hi = anchor.char_offset.max(head.char_offset).min(char_len);
        if lo == hi {
            return None;
        }
        // Slice by CHAR boundary (the offsets are char indices), so multi-byte text is sliced safely.
        let text: String = block_text.chars().skip(lo).take(hi - lo).collect();
        if text.is_empty() {
            None
        } else {
            Some((block_idx, lo, hi, text))
        }
    }

    /// The plain text of the block at `idx` (for hit-testing / tests).
    pub fn block_plain_text(&self, idx: usize) -> Option<String> {
        let block = self.doc.children.get(idx)?.as_block()?;
        let mut s = String::new();
        for c in &block.children {
            match c {
                Child::Text(t) => s.push_str(&t.text.to_string()),
                Child::HsLink(l) => s.push_str(&if l.label.is_empty() {
                    format!("{}:{}", l.ref_kind, l.ref_value)
                } else {
                    l.label.clone()
                }),
                // A transclusion atom contributes a short reference label so the block's plain text
                // (used for AccessKit labels / hit-testing) reads sensibly.
                Child::Transclusion(t) => s.push_str(&format!("[transclusion:{}]", t.ref_value)),
                Child::Block(_) => {}
            }
        }
        Some(s)
    }

    /// WP-KERNEL-012 MT-056 (E2 — outline/TOC): a content-derived REVISION of the heading-relevant
    /// document structure. The outline panel gates its tree rebuild on this value: it rebuilds ONLY when
    /// the revision advances and does NOT rebuild on a frame where the revision is unchanged (RISK-001 /
    /// MC-001 / AC-004 — never a per-frame rebuild).
    ///
    /// ## Why a content fingerprint, not an instrumented counter (reconciliation)
    ///
    /// The MT contract names `doc.revision()`, but the real MT-011 DocModel is a value tree with NO
    /// revision/dirty counter, and edits flow through `apply_transaction` at ~50 decentralized call
    /// sites (input/IME/formatting/find-replace/slash). Instrumenting all of them to bump a counter is
    /// out of MT-056 scope and risky. Instead this is a PURE fingerprint over only the heading-relevant
    /// data (each top-level Heading's level + plain text + ordinal position), computed from the PUBLIC
    /// DocModel surface — no reach into DocModel internals (MC-006), no mutation-site instrumentation. A
    /// non-heading edit (typing in a paragraph) leaves the fingerprint UNCHANGED, so the outline truly
    /// does not rebuild then; adding/removing/retitling/re-leveling a heading changes it, firing exactly
    /// one rebuild on the next frame.
    pub fn doc_revision(&self) -> u64 {
        // FNV-1a over the heading-relevant structure, in document order. Deterministic + cheap (one walk
        // of the top-level blocks). The walk only fingerprints level + plain text + ordinal of each
        // top-level Heading; everything else contributes nothing, so non-heading edits do not perturb it.
        const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
        const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
        let mut hash = FNV_OFFSET;
        let mut mix = |bytes: &[u8]| {
            for &b in bytes {
                hash ^= b as u64;
                hash = hash.wrapping_mul(FNV_PRIME);
            }
        };
        for (idx, child) in self.doc.children.iter().enumerate() {
            let Some(block) = child.as_block() else { continue };
            if let Some(level) = block.heading_level() {
                mix(&(idx as u64).to_le_bytes()); // ordinal position (a re-order changes the outline)
                mix(&[level]); // heading level (a re-level changes nesting)
                if let Some(text) = self.block_plain_text(idx) {
                    mix(text.as_bytes()); // heading text (a retitle changes the entry label)
                }
                mix(&[0xff]); // entry separator
            }
        }
        hash
    }

    /// WP-KERNEL-012 MT-056: resolve a stable `block_id` (the `re-block-{hash}` string from
    /// [`super::block_author_id`]) back to its top-level block path against the LIVE document. Scans the
    /// current top-level blocks and returns the path whose [`super::block_author_id`] matches — so the
    /// lookup is authoritative against the live doc and is ITSELF the stale-id guard: a deleted heading's
    /// id matches no live block, yielding `None` (RISK-003 / MC-003). O(top-level blocks); called only on
    /// a click/Press, never per frame. The FNV path hash is one-way, so a forward scan is the correct
    /// inverse (and it cannot resolve to a block that no longer exists).
    pub fn block_path_from_id(&self, block_id: &str) -> Option<Vec<usize>> {
        for idx in 0..self.doc.children.len() {
            if self.doc.children[idx].as_block().is_some()
                && super::block_author_id(&[idx]) == block_id
            {
                return Some(vec![idx]);
            }
        }
        None
    }

    /// WP-KERNEL-012 MT-056: true when the top-level block at path `[idx]` still exists in the LIVE
    /// document (the stale-`block_id` guard, RISK-003 / MC-003). The outline addresses blocks by their
    /// top-level index path; a deleted heading leaves a dangling entry until the next revision-gated
    /// rebuild, so click/Press first checks this and skips silently if the block is gone.
    pub fn block_path_exists(&self, path: &[usize]) -> bool {
        match path.first() {
            Some(&idx) => self.doc.children.get(idx).and_then(Child::as_block).is_some(),
            None => false,
        }
    }

    /// WP-KERNEL-012 MT-056: place an anchored whole-block selection across the top-level block at
    /// `path` THROUGH the EXISTING MT-012 selection model — a [`Selection::Text`] spanning the block's
    /// first text leaf from offset 0 to its end (so the heading reads as selected), falling back to a
    /// [`Selection::Node`] whole-node selection when the block has no text leaf (an atom block). This is
    /// the ONE selection mechanism (RISK-002 / MC-002 — no second selection path), so cross-pane (E5)
    /// selection stays consistent. No-op when the block is gone (RISK-003 / MC-003).
    pub fn select_block(&mut self, path: &[usize]) {
        let Some(&idx) = path.first() else { return };
        let Some(block) = self.doc.children.get(idx).and_then(Child::as_block) else {
            return; // stale path — let the next rebuild drop the entry (MC-003).
        };
        // Find the first text leaf in the block so the selection covers its heading text. A heading is a
        // single text leaf at child 0, but we scan to be robust to a marked-run split.
        let first_text = block
            .children
            .iter()
            .position(|c| c.as_text().is_some())
            .map(|leaf_idx| (leaf_idx, block.children[leaf_idx].as_text().expect("checked").text.len_chars()));
        self.selection = match first_text {
            Some((leaf_idx, len)) => {
                let anchor = DocPosition::new(vec![idx, leaf_idx], 0);
                let head = DocPosition::new(vec![idx, leaf_idx], len);
                Selection::text(anchor, head)
            }
            None => Selection::node(vec![idx]),
        };
    }
}

/// WP-KERNEL-012 MT-035 (E5 — unified undo): a decoded undo/redo keyboard chord the rich pane routes
/// through the shared [`crate::interop::InteractionBus`] (POLICY-1). Carried OUT of the locked frame
/// region so the bus entry's restore closure (which re-locks the shared state Arc) runs without a
/// deadlock against the frame's own state guard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UndoChord {
    /// Ctrl/Cmd+Z (without Shift): undo the focused rich pane's most recent unified-scope entry.
    Undo,
    /// Ctrl/Cmd+Y: redo the focused rich pane's most recently undone entry.
    Redo,
}

/// The per-frame editor view. Construct it with the shared state and call [`Self::show`]
/// inside an egui `Ui` (or use it as an `egui::Widget` via [`Self::ui`]).
pub struct RichEditorWidget {
    state: Arc<Mutex<RichEditorState>>,
    /// WP-KERNEL-012 MT-055 (E2 — reading mode): when `true`, render the SAME MT-011 DocModel through
    /// the SAME MT-012 block renderer but as a clean, read-only reading view (RISK-001 / MC-001 — NO
    /// second renderer). The read-only branch (a) does NOT apply any input/edit dispatch and does NOT
    /// resolve/paint a caret or selection (so the document cannot be mutated and no editable
    /// TextEdit/TextInput node lands in the AccessKit tree — RISK-002/RISK-005), (b) widens the content
    /// margins to a centered reading column via the WP-011 theme/spacing tokens, and (c) keeps wikilink
    /// chips (MT-015) + embeds/node-views (MT-014) interactive (RISK-003). When `false`, behavior is
    /// EXACTLY the MT-012 editable path (AC-008 — no regression).
    read_only: bool,
}

impl RichEditorWidget {
    /// Build a widget over shared editor state in the editable (MT-012) mode (the default).
    pub fn new(state: Arc<Mutex<RichEditorState>>) -> Self {
        Self { state, read_only: false }
    }

    /// WP-KERNEL-012 MT-055: build a widget that renders `state`'s document READ-ONLY (the Obsidian
    /// reading view). Equivalent to `Self::new(state).with_read_only(true)`.
    pub fn new_read_only(state: Arc<Mutex<RichEditorState>>) -> Self {
        Self { state, read_only: true }
    }

    /// WP-KERNEL-012 MT-055: set the read-only (reading-mode) flag. The host reads
    /// `store.get(document_id) == ViewMode::Reading` and passes the result here so the editor flips
    /// between the editable and reading views without forking a second renderer.
    pub fn with_read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    /// WP-KERNEL-012 MT-055: build a widget for the given [`crate::rich_editor::reading_mode::ViewMode`]
    /// — `Reading` renders read-only, `Edit` renders the editable path. The ViewMode entry point the
    /// contract names as an alternative to the bare `read_only` flag.
    pub fn for_view_mode(
        state: Arc<Mutex<RichEditorState>>,
        mode: crate::rich_editor::reading_mode::ViewMode,
    ) -> Self {
        Self { state, read_only: mode.is_read_only() }
    }

    /// WP-KERNEL-012 MT-055: whether this widget renders read-only (the reading view). Read by tests
    /// (AC-001) to assert a `ViewMode::Reading` host builds a `read_only=true` widget.
    pub fn is_read_only(&self) -> bool {
        self.read_only
    }

    /// WP-KERNEL-012 MT-056 (E2 — outline/TOC): request that the next render bring the top-level block
    /// addressed by `block_id` into view. `block_id` is the editor's stable block address — the
    /// `re-block-{hash}` string produced by [`super::block_author_id`] for the block's top-level path —
    /// the SAME id the per-block AccessKit node and the outline entry use. The request is recorded as a
    /// pending scroll target on the shared editor state and resolved by [`Self::render_blocks`] on the
    /// next frame via the editor's EXISTING `rich-editor-scroll` ScrollArea (`ui.scroll_to_rect`), NOT a
    /// second scroll mechanism (RISK-002 / MC-002). No-op when the id does not resolve to a live block
    /// (stale id — RISK-003 / MC-003; the next revision-gated rebuild drops the entry).
    pub fn scroll_to_block(&mut self, block_id: &str) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(path) = state.block_path_from_id(block_id) {
            state.pending_scroll_block = Some(path);
        }
    }

    /// WP-KERNEL-012 MT-056: place an anchored selection across the heading block addressed by
    /// `block_id`, routing strictly through the MT-012 selection model ([`RichEditorState::select_block`]
    /// — no second selection mechanism, RISK-002 / MC-002). No-op for a stale id (RISK-003 / MC-003).
    pub fn select_block(&mut self, block_id: &str) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(path) = state.block_path_from_id(block_id) {
            state.select_block(&path);
        }
    }

    /// Render the editor into `ui`, returning the interaction [`egui::Response`] for the
    /// editor surface (so a caller can check focus / hover). This is the core entry the
    /// `egui::Widget` impl and the pane factory both call.
    pub fn show(self, ui: &mut egui::Ui) -> egui::Response {
        // WP-KERNEL-012 MT-055: the read-only (reading-mode) flag for this frame. Threaded through the
        // SAME render path (no fork) so the reading view shows exactly what Edit shows, minus editing.
        let read_only = self.read_only;
        // MT-035: keep the shared state Arc so the unified-undo entries recorded this frame can capture a
        // `Weak<Mutex<RichEditorState>>` back-ref to the live document (RISK-3 / MC-3 — no retain cycle).
        let state_arc = Arc::clone(&self.state);
        // MT-035: a Ctrl+Z / Ctrl+Y chord decoded this frame is routed through the bus AFTER the state
        // guard below is dropped (the bus restore closure re-locks the state Arc — invoking it while the
        // guard is held would deadlock). The shared bus is captured before the scope for that post-frame
        // invocation. The pane id is read from the state inside the scope.
        let bus_for_undo = crate::interop::interaction_bus::InteractionBus::get_or_init(ui.ctx());
        let mut pending_undo_chord: Option<UndoChord> = None;
        let mut undo_pane_id: Option<crate::pane_registry::PaneId> = None;
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());

        // OUTER container scope: a stable Ui id keeps the root AccessKit node id fixed
        // across frames (same pattern the code editor uses). We render the scroll area +
        // blocks inside it and emit the root node onto this scope's Ui id so the per-block
        // nodes are its descendants.
        // WP-KERNEL-012 MT-055: in reading mode the editor surface is NON-focusable (it must never
        // receive Text/Key/Ime events — RISK-002/RISK-005, the document is read-only), but it stays a
        // CLICK sense so wikilink chips/embeds inside it remain interactive (RISK-003) and the surface
        // is still scrollable/hoverable. In Edit mode it is click_and_drag (focusable) exactly as MT-012.
        let surface_sense = if read_only {
            egui::Sense::click()
        } else {
            egui::Sense::click_and_drag()
        };
        let response = ui
            .scope_builder(
                egui::UiBuilder::new()
                    .id_salt(root_egui_id())
                    .sense(surface_sense),
                |ui| {
                    let palette = state.palette();
                    // Paint the editor background from the theme (no hardcoded hex).
                    let full_rect = ui.available_rect_before_wrap();
                    if ui.is_rect_visible(full_rect) {
                        ui.painter().rect_filled(full_rect, 0.0, palette.bg);
                    }

                    // The surface response. In Edit mode: clickable+focusable so the editor can
                    // hold keyboard focus and receive Text/Key/Ime events. In Reading mode (MT-055):
                    // CLICK-only (NOT focusable) so it can never receive editing input — but it is
                    // still an interactive node, so it MUST carry a stable author_id (the shell
                    // HBR-SWARM gate panics on an unnamed interactive node) — we give it the
                    // editor-surface id so a swarm agent can locate the surface by a stable key.
                    let surface_id = ui.id().with("rich-editor-surface");
                    let surface = ui.interact(full_rect, surface_id, surface_sense);
                    // RISK-005: in reading mode the surface is never treated as focused, so the input
                    // path is skipped and no caret/selection state is ever advanced or allocated.
                    let has_focus = !read_only && surface.has_focus();
                    // Attach the stable author_id to the interactive surface node (it keeps
                    // egui's derived focusable role/actions; we only add the address).
                    crate::accessibility::emit_interactive_node(
                        ui.ctx(),
                        surface_id,
                        "rich-editor-surface",
                    );

                    // MT-016 (RISK-4 / MC-004): close the slash menu when the editor surface loses
                    // focus (e.g. the operator clicked outside the window), so an open popup never
                    // strands input to other surfaces. An ACTIVE PROMPT modal is left open — it is a
                    // top-order egui::Window that holds its own focus, so the editor-surface losing
                    // focus to the modal is expected and must NOT dismiss the modal.
                    if !has_focus
                        && state
                            .slash_menu
                            .as_ref()
                            .is_some_and(|m| !m.prompt_active())
                    {
                        state.slash_menu = None;
                    }

                    // WP-KERNEL-012 MT-055: in reading mode SKIP the entire input/edit + drag-in path
                    // (RISK-005 / MC-005 — no input dispatch, no caret/selection alloc, the DocModel is
                    // never mutated). The bus undo wiring is also skipped (nothing to record). Wikilink
                    // chips + embeds are still rendered+interactive below (RISK-003), since their click
                    // handling lives in `render_blocks`, NOT in this editable input branch.
                    if !read_only {
                        // 1) Apply input + IME for this frame (only meaningful when focused, but
                        //    we still drain events so a programmatically-focused test works). The ONE shared
                        //    InteractionBus (MT-035 unified undo) is retrieved here so a rich-pane edit
                        //    records its undo on the SAME scope the code/canvas panes share, and Ctrl+Z /
                        //    Ctrl+Y / Ctrl+Shift+Z route THROUGH the bus (POLICY-1/POLICY-2) rather than a
                        //    second per-pane stack. Bus access is via `with_try_lock` so it never blocks the
                        //    egui frame thread (RISK-1 / MC-1).
                        let bus = crate::interop::interaction_bus::InteractionBus::get_or_init(ui.ctx());
                        undo_pane_id = state.undo_pane_id.clone();
                        Self::apply_frame_input(
                            ui,
                            &mut state,
                            has_focus,
                            &bus,
                            &state_arc,
                            &mut pending_undo_chord,
                        );

                        // WP-KERNEL-012 MT-033 (E5 — CKC drag-in): a CKC/Atelier item dragged from the atelier
                        // side panel via the cross-surface [`crate::interop::DragPayload`] channel and RELEASED
                        // over the editor inserts an inline `hsLink` embed atom (by CKC `refKind`) at the
                        // caret. The embed is the EXISTING hsLink atom (the MT-014 lesson), so it ROUND-TRIPS
                        // the backend `content_json` (AC-2) — never an invented node the backend would drop.
                        // Guard the take with `has_payload_of_type` (the egui take-payload hazard: an unguarded
                        // `dnd_release_payload::<T>` unconditionally takes-then-downcasts, discarding a payload
                        // of another type meant for a sibling surface). SKIPPED in reading mode (a read-only
                        // document accepts no drag-in edits).
                        if egui::DragAndDrop::has_payload_of_type::<crate::interop::DragPayload>(ui.ctx()) {
                            if let Some(payload) = surface.dnd_release_payload::<crate::interop::DragPayload>() {
                                if let Some(link) = payload.to_hs_link() {
                                    Self::insert_atelier_embed_at_caret(&mut state, link);
                                    ui.ctx().request_repaint();
                                }
                            }
                        }
                    }

                    // 2) MT-013: the formatting toolbar above the content area, then the
                    //    blocks below it, stacked vertically (contract step 6:
                    //    `ui.vertical(|ui| { toolbar.ui(ui); content_area(ui); })`). The
                    //    toolbar borrows the SAME editor state so a button click dispatches a
                    //    command directly on it.
                    //
                    // WP-KERNEL-012 MT-055: in reading mode the EDITING chrome (properties panel,
                    // draft banner, formatting toolbar) is SUPPRESSED — a read-only view has no edit
                    // affordances, and the toolbar emits editable controls that must not appear. The
                    // BLOCKS still render (the reading presentation), centered into the reading column.
                    let palette = state.palette(); // re-resolve (theme unchanged, cheap)
                    ui.vertical(|ui| {
                        if !read_only {
                            // MT-017: the document properties panel ABOVE the content (default collapsed).
                            Self::render_properties(ui, &mut state, &palette);
                            // MT-020: the draft-recovery banner (only when a recoverable draft is available)
                            // sits above the toolbar so the operator sees it before editing.
                            Self::render_draft_banner(ui, &mut state, &palette);
                            Self::render_toolbar(ui, &mut state);
                            ui.separator();
                        }
                        Self::render_blocks(ui, &mut state, &palette, has_focus, read_only);
                    });

                    // WP-KERNEL-012 MT-055: the find/replace, save/draft, conflict, and export chrome are
                    // EDITING surfaces — skipped in reading mode (a read-only document is never saved,
                    // never has a draft conflict, and exposes no find-in-doc editing surface here).
                    if !read_only {
                        // MT-018: render the floating find/replace panel (a top-level egui::Window) when
                        // open, and apply its outcome (Replace One / Replace All / Close) against the doc
                        // + undo manager. Rendered after the content so it floats above the blocks; the
                        // window does not steal editor keyboard focus (HBR-QUIET).
                        Self::render_find_panel(ui.ctx(), &mut state, &palette);

                        // MT-020: drive the save/draft coordinators (drain completed off-thread results +
                        // fire the debounced draft upsert), then render the conflict window, the draft
                        // recovery banner, and the export format picker. All reuse the theme palette + the
                        // shell accessibility hook. Rendered after the content so the conflict window floats
                        // above the blocks.
                        Self::drive_save_and_draft(ui.ctx(), &mut state);
                        Self::render_conflict_window(ui.ctx(), &mut state, &palette);
                        Self::render_export_picker(ui, &mut state, &palette);
                    }

                    // WP-KERNEL-012 MT-041 (E7): sync + emit the consolidated `editor.rich.<action>`
                    // AccessKit nodes and consume any swarm Action::Click dispatched at them THIS frame,
                    // so a swarm agent's dispatch reaches the editor before the next frame
                    // (RISK-041-04). A no-op when no registry is installed. Run inside the editor scope
                    // so the action nodes nest under the editor surface.
                    //
                    // WP-KERNEL-012 MT-055: the editor-action surface is the set of EDITING actions
                    // (bold/italic/find/save/…) — suppressed in reading mode so a swarm agent cannot
                    // dispatch an edit at a read-only document (RISK-005). The reading-view toggle is the
                    // ONLY editor-chrome control the reading view exposes; it is mounted by the host
                    // (E11) outside this widget.
                    if !read_only {
                        Self::sync_editor_actions(ui, &mut state);
                    }

                    // 3) Emit the root AccessKit node (author_id rich-editor-root) onto THIS scope's
                    //    Ui id so the block nodes nest under it. REUSES the same accesskit_node_builder
                    //    hook as the shell.
                    //
                    // WP-KERNEL-012 MT-055 (AC-003 / RISK-002 / MC-002): in Reading mode the document
                    // body must NOT advertise an editable text field. AC-10's editable `Role::TextInput`
                    // container is correct ONLY for the editable path. In read-only mode we instead emit
                    // `Role::Document` and set the first-class `ReadOnly` flag (accesskit 0.21.1
                    // lib.rs:1646 — "a text widget that allows focus/selection but not input"), and we
                    // drop the editable-looking `set_value("{n} blocks")` so a screen reader / swarm
                    // agent does not see a populated, editable text field for a read-only note.
                    let root_node_id = ui.unique_id();
                    let value = format!("{} blocks", state.doc.children.len());
                    ui.ctx().accesskit_node_builder(root_node_id, move |node| {
                        node.set_author_id(RICH_EDITOR_ROOT_AUTHOR_ID.to_owned());
                        node.set_label("Rich text editor".to_owned());
                        if read_only {
                            node.set_role(accesskit::Role::Document);
                            node.set_read_only();
                        } else {
                            node.set_role(ROOT_ROLE);
                            node.set_value(value.clone());
                        }
                    });

                    surface
                },
            )
            .inner;

        // MT-035 (E5 — unified undo): now that the per-frame state guard is dropped, route any decoded
        // Ctrl+Z / Ctrl+Y chord through the shared bus (POLICY-1). The bus entry's restore closure
        // re-locks the shared state Arc cleanly here (the guard is gone), rebuilds the live doc from the
        // recorded content_json snapshot, and resets the per-pane UndoManager so there is ONE undo
        // authority. We request a repaint so the reverted doc paints immediately.
        drop(state);
        if let Some(chord) = pending_undo_chord {
            let handled = Self::invoke_undo_chord(&bus_for_undo, undo_pane_id.as_ref(), chord);
            if handled {
                // The undo/redo mutated the doc; reset the typing batch + mark dirty so the draft/save
                // state tracks the reverted content, then repaint.
                let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
                s.undo_batcher.reset();
                s.undo_batch_before = None;
                if let Some(save) = s.save.as_mut() {
                    save.mark_dirty();
                }
                if let Some(draft) = s.draft.as_mut() {
                    draft.mark_dirty(std::time::Instant::now());
                }
                let RichEditorState { doc, find_replace, .. } = &mut *s;
                if let Some(panel) = find_replace.as_mut() {
                    panel.rescan(doc);
                }
                drop(s);
                ui.ctx().request_repaint();
            }
        }

        response
    }

    /// Drain this frame's egui input events and apply them to the editor state. IME events
    /// route to [`ime_handler`]; key/text events route to [`input_handler`]. We snapshot
    /// the events from `ui.input` and apply them while holding the state lock.
    fn apply_frame_input(
        ui: &egui::Ui,
        state: &mut RichEditorState,
        has_focus: bool,
        bus: &Arc<Mutex<crate::interop::interaction_bus::InteractionBus>>,
        state_arc: &Arc<Mutex<RichEditorState>>,
        pending_undo_chord: &mut Option<UndoChord>,
    ) {
        let mut events: Vec<egui::Event> = ui.input(|i| i.events.clone());

        // MT-015: when the wikilink autocomplete popup is OPEN, it CLAIMS the navigation/confirm/
        // cancel keys (Up/Down/Enter/Tab/Escape) so they drive the popup instead of the editor (e.g.
        // Enter inserts the selected wikilink instead of splitting the paragraph). This runs BEFORE
        // the focus gate: egui may release editor focus on Escape in the SAME frame the key arrives,
        // so gating the popup's Escape on `has_focus` would swallow it and leave the popup stuck open.
        // The claimed key events are removed from the event list before the normal editing path runs.
        if state.wikilink_autocomplete.is_some() {
            events = Self::handle_autocomplete_keys(state, events);
        }

        // MT-016: when the slash menu is OPEN (and no autocomplete is active, and no prompt modal is
        // up — the prompt owns Enter/Escape via its own render), it CLAIMS the nav/confirm/cancel keys
        // (Up/Down/Enter/Escape) so they drive the menu instead of the editor (Enter executes the
        // selected command instead of splitting the paragraph; Escape closes + leaves the `/`). Like
        // the autocomplete claim, this runs BEFORE the focus gate so Escape is not swallowed when egui
        // releases focus in the same frame. The claimed keys are removed before the editing path runs.
        if state.wikilink_autocomplete.is_none()
            && state.slash_menu.as_ref().is_some_and(|m| !m.prompt_active())
        {
            events = Self::handle_slash_menu_keys(state, events);
        }

        // WP-KERNEL-012 MT-058: when the inline-tag `#` menu is OPEN (and no wikilink autocomplete /
        // slash menu owns the input), it CLAIMS Up/Down/Enter/Tab/Escape so they drive the menu (Enter
        // commits the selected/free-typed tag as a chip instead of splitting the paragraph; Escape
        // closes + removes the `#` trigger). Runs BEFORE the focus gate like the other inline popups so
        // Escape is not swallowed when egui releases focus in the same frame.
        if state.wikilink_autocomplete.is_none()
            && state.slash_menu.is_none()
            && state.tag_autocomplete.is_some()
        {
            events = Self::handle_tag_menu_keys(state, events);
        }

        if !has_focus {
            return; // an unfocused editor ignores the remaining input (and never schedules a repaint).
        }

        // MT-020: a Ctrl+S / Cmd+S shortcut triggers a canonical save of the live document. Handled
        // BEFORE the editing decode so the chord saves instead of being swallowed; Ctrl+S produces no
        // Text event and is not a formatting chord, so it does not double-fire. A save is a no-op when
        // no save context is installed (headless / no document) or a save is already in flight (MC-002).
        if input_handler::decode_save_shortcut(&events) {
            Self::trigger_save(state);
            return; // the chord is consumed by the save; do not also run the editing decode this frame.
        }

        // MT-018: a Ctrl+F / Ctrl+H shortcut opens (or re-focuses) the find/replace panel. Handled
        // BEFORE the editing decode so the chord opens the panel instead of being swallowed; the
        // events are NOT removed (Ctrl+F/Ctrl+H produce no Text event and no EditAction, so they do
        // not double-fire as typing). Opening triggers an initial scan against the live doc.
        if let Some(shortcut) = input_handler::decode_find_replace_shortcut(&events) {
            Self::apply_find_replace_shortcut(state, shortcut);
            return; // the chord is consumed by the panel; do not also run the editing decode this frame.
        }

        // WP-KERNEL-012 MT-035 (E5 — unified undo): DECIDE whether Ctrl+Z (undo) / Ctrl+Y (redo) is
        // present, and STRIP those chords from `events` BEFORE the formatting/edit decode so the
        // formatting pass (which maps Ctrl+Z -> FormattingCommand::Undo -> the parallel UndoManager) never
        // fires — eliminating the second competing undo stack the adversarial review flagged. The ACTUAL
        // bus undo/redo is invoked by the caller AFTER the state lock is released (the bus entry's
        // snapshot-restore closure re-locks the shared state Arc, which would deadlock if invoked while
        // this frame still holds the guard). Ctrl+Shift+Z is the bus's CROSS-PANE undo (POLICY-2), owned
        // by the shell keybind dispatch — NOT the rich pane's redo.
        let (events_after_undo, chord) = Self::decode_undo_chord(events);
        events = events_after_undo;
        if let Some(chord) = chord {
            // Hand the chord up so `show()` can route it through the bus once the state guard is dropped.
            // We do NOT run the normal edit decode this frame (the chord is consumed by undo/redo).
            *pending_undo_chord = Some(chord);
            return;
        }

        // MT-035: snapshot the doc BEFORE this frame's edits so a recorded undo entry can restore the
        // pre-edit content_json (the batcher coalesces a burst into ONE entry). Cheap relative to the
        // edit itself; only used when the frame actually mutates the doc.
        let doc_before =
            crate::rich_editor::document_model::doc_json::to_content_json_value(&state.doc);

        // Split events into IME vs key/text in ARRIVAL order so a Preedit->Commit sequence
        // interleaved with typing applies in the right order. We process each event in
        // order: IME events through the IME handler, everything else accumulates into the
        // input decoder applied at its position.
        for ev in &events {
            if let egui::Event::Ime(ime) = ev {
                let RichEditorState { doc, selection, undo, preedit, actor_id, .. } = state;
                let mut ctx = ImeContext {
                    doc,
                    selection,
                    undo,
                    preedit,
                    actor_id: actor_id.as_str(),
                };
                ime_handler::handle_ime_event(&mut ctx, ime);
            }
        }
        // MT-013: decode the formatting/structural CHORDS first (Ctrl+B, Ctrl+Z, Enter,
        // Ctrl+Alt+1, …) so they take precedence over plain text/nav handling — a chord
        // toggles a mark / splits a block instead of inserting a character (MT impl note
        // 3). Tab/Shift+Tab indent only when the caret is in a list (else Tab traverses
        // focus — RISK-4 / MC-004).
        let in_list = input_handler::caret_in_list(&state.doc, &state.selection);
        let fmt_cmds = input_handler::decode_formatting_commands(&events, in_list);
        for cmd in &fmt_cmds {
            let RichEditorState { doc, selection, undo, actor_id, .. } = state;
            let mut ctx = EditContext {
                doc,
                selection,
                undo,
                actor_id: actor_id.as_str(),
            };
            input_handler::apply_formatting_command(&mut ctx, cmd);
        }

        // Decode + apply the non-IME editing/nav events. A key event already consumed by a
        // formatting chord (e.g. Ctrl+Z -> Undo, which BOTH the formatting keymap and the
        // plain decode recognize) is filtered out here so it does not double-fire; Enter is
        // never a plain-decode action (it has no `EditAction`), so the split fires once.
        let actions = input_handler::decode_events_excluding_formatting(&events, in_list);
        // Capture whether this frame produced any edit BEFORE the loop consumes `actions`, so the
        // MT-020 dirty-mark below can read it (the loop moves `actions` into `into_iter`).
        let any_edit = !actions.is_empty() || !fmt_cmds.is_empty();
        if !actions.is_empty() {
            let RichEditorState { doc, selection, undo, actor_id, .. } = state;
            let mut ctx = EditContext {
                doc,
                selection,
                undo,
                actor_id: actor_id.as_str(),
            };
            for action in actions {
                input_handler::apply_action(&mut ctx, action);
            }
        }

        // WP-KERNEL-012 MT-035 (E5 — unified undo): if ANY input this frame (a typed/structural edit, a
        // formatting chord, OR an IME commit) actually mutated the doc, record the edit on the SHARED bus
        // scope (POLICY-1 local-first), coalesced by the 500ms batcher (RISK-1 / MC-1). This is the LIVE
        // rich-pane bus-undo recording — the entry the Ctrl+Z routed above pops + restores. The
        // `doc_after != doc_before` content_json diff is the real mutation signal (so an IME-only commit,
        // which is not in `actions`/`fmt_cmds`, is still captured). A no-op without a mounted pane id.
        // (`any_edit` is still used below for the MT-020 dirty-mark; here we use the snapshot diff.)
        let doc_after =
            crate::rich_editor::document_model::doc_json::to_content_json_value(&state.doc);
        if doc_after != doc_before {
            Self::record_rich_edit_undo(state, bus, state_arc, doc_before, doc_after);
        }

        // MT-015: after applying input, re-detect the `[[` autocomplete trigger from the caret's
        // text-before-caret. Typing `[[` opens the popup; typing more refines the query (bumping the
        // debounce + generation); typing/moving past `]]` (or out of the token) closes it.
        Self::refresh_autocomplete_trigger(state);

        // MT-016: re-detect the `/` slash-command trigger from the caret's text-before-caret. Typing
        // `/` at the start of a blank line or after whitespace opens the menu; typing more refines the
        // filter; backspacing the `/`, typing a space, or moving out of the token closes it (AC-1/AC-2/
        // AC-5). Skipped while a prompt modal is active (the prompt owns the input) or the autocomplete
        // is open (mutually exclusive surfaces).
        if state.wikilink_autocomplete.is_none()
            && !state.slash_menu.as_ref().is_some_and(|m| m.prompt_active())
        {
            Self::refresh_slash_trigger(state);
        }

        // WP-KERNEL-012 MT-058: re-detect the `#` inline-tag trigger from the caret's text-before-caret.
        // Typing `#` at a word boundary (offset 0 or after whitespace/punctuation) opens the tag menu;
        // typing the body refines the live filter; a space / non-tag char after the body, deleting the
        // `#`, or moving out of the token closes it (AC-001). Mutually exclusive with the wikilink
        // autocomplete + the slash menu (those surfaces own the input when open).
        if state.wikilink_autocomplete.is_none() && state.slash_menu.is_none() {
            Self::refresh_tag_trigger(state);
        } else {
            state.tag_autocomplete = None;
        }

        // MT-018: the find/replace panel recomputes its scan on every document change while open
        // (the React "recompute on every document change"). Any typing/delete/undo this frame may
        // have mutated the doc, so the scan + the active-index clamp are refreshed here. This is a
        // synchronous in-memory walk (no spinner, no async); the panel renders only when open.
        let RichEditorState { doc, find_replace, .. } = state;
        if let Some(panel) = find_replace.as_mut() {
            panel.rescan(doc);
        }

        // MT-020: any edit decoded above (text/structural action, IME commit, undo/redo, find-replace
        // replacement) may have mutated the doc. Mark the save manager dirty + start the draft
        // debounce window so the 5s auto-draft persists in-progress edits (crash recovery). The dirty
        // mark is idempotent; the draft debounce only starts on the first dirty change in the window.
        if any_edit {
            let now = std::time::Instant::now();
            if let Some(save) = state.save.as_mut() {
                save.mark_dirty();
            }
            if let Some(draft) = state.draft.as_mut() {
                draft.mark_dirty(now);
            }
        }
    }

    /// WP-KERNEL-012 MT-035 (E5 — unified undo): detect the FIRST Ctrl+Z (undo) / Ctrl+Y (redo) chord in
    /// `events` and STRIP it, returning `(events_without_the_chord, chord)`. Decision-only: the actual bus
    /// undo/redo is invoked by [`Self::show`] AFTER the state guard is dropped (the bus entry's restore
    /// closure re-locks the shared state Arc, so invoking it while the frame holds the guard would
    /// deadlock). Stripping the chord BEFORE the formatting decode is what removes the parallel-stack path
    /// (the formatting keymap maps Ctrl+Z -> the rich `UndoManager`). Ctrl+Shift+Z is intentionally NOT
    /// the rich redo here — it is the bus's cross-pane undo (POLICY-2), owned by the shell.
    fn decode_undo_chord(events: Vec<egui::Event>) -> (Vec<egui::Event>, Option<UndoChord>) {
        let mut decided: Option<UndoChord> = None;
        let mut remaining = Vec::with_capacity(events.len());
        for ev in events {
            if let egui::Event::Key { key, pressed: true, modifiers, .. } = &ev {
                let ctrl = modifiers.command || modifiers.ctrl;
                if ctrl && !modifiers.alt {
                    match key {
                        egui::Key::Z if modifiers.shift => {
                            // Ctrl+Shift+Z is the bus's CROSS-PANE undo (POLICY-2), owned by the shell
                            // keybind dispatch. STRIP it so the rich formatting pass (which maps it to the
                            // parallel `UndoManager` Redo) never fires — but do NOT route it as a rich
                            // chord (the shell owns cross-pane undo). The rich pane's redo is Ctrl+Y.
                            continue;
                        }
                        egui::Key::Z => {
                            decided.get_or_insert(UndoChord::Undo);
                            continue; // strip: do not let the formatting pass see Ctrl+Z.
                        }
                        egui::Key::Y => {
                            decided.get_or_insert(UndoChord::Redo);
                            continue; // strip Ctrl+Y.
                        }
                        _ => {}
                    }
                }
            }
            remaining.push(ev);
        }
        (remaining, decided)
    }

    /// WP-KERNEL-012 MT-035 (E5 — unified undo): invoke a decoded undo/redo chord through the shared bus
    /// (POLICY-1 local-first) for `pane_id`. MUST be called with NO `RichEditorState` lock held (the bus
    /// entry's restore closure re-locks the shared state Arc). Returns `true` when the bus had an entry to
    /// undo/redo (so the caller marks the doc dirty + repaints). A no-op (`false`) when no pane id is
    /// mounted — never a fake.
    fn invoke_undo_chord(
        bus: &Arc<Mutex<crate::interop::interaction_bus::InteractionBus>>,
        pane_id: Option<&crate::pane_registry::PaneId>,
        chord: UndoChord,
    ) -> bool {
        let Some(pane_id) = pane_id else {
            return false; // not mounted; nothing to route to (honest no-op, not a fake undo).
        };
        crate::interop::interaction_bus::InteractionBus::with_try_lock(bus, |b| {
            b.set_focus_owner(pane_id.clone());
            let fired = match chord {
                UndoChord::Undo => b.undo(pane_id).is_some(),
                UndoChord::Redo => b.redo(pane_id).is_some(),
            };
            // MT-036 (E5 — one event ledger): emit a REAL `undo_fired` FlightEvent at this LIVE undo
            // dispatch (the MT-035 path that is wired + tested). Only emit when an action actually fired
            // (a no-op chord on an empty ring is NOT an event). The bus is ALREADY locked here, so emit
            // directly (no re-lock). scope=local — this is the focused-pane local-first Ctrl+Z/Ctrl+Y
            // ring (POLICY-1). The emit is a no-op until the shell installs the emitter (defer policy).
            if fired {
                let workspace_id =
                    b.event_emitter().map(|e| e.workspace_id().to_owned()).unwrap_or_default();
                let ev = crate::event_emitter::NativeEditorEvent::undo_fired(
                    crate::event_emitter::UndoScope::Local,
                    pane_id.as_ref(),
                    crate::event_emitter::native_editor_actor_id(pane_id.as_ref()),
                    workspace_id,
                );
                b.emit_event(ev);
            }
            fired
        })
        .unwrap_or(false)
    }

    /// WP-KERNEL-012 MT-035 (E5 — unified undo): record a rich-text edit on the shared bus scope for this
    /// pane (POLICY-1), coalesced by the 500ms batcher (RISK-1 / MC-1). The undo entry's `undo_fn`
    /// restores the batch-START content_json; its `redo_fn` re-applies the latest content_json. The
    /// restore applier rebuilds the live `doc` from the snapshot, resets the caret to the doc start, and
    /// resets the per-pane `UndoManager` so the (now bus-driven) transaction history cannot replay stale
    /// inverse steps — there is ONE undo authority (the bus), not two. No-op without a mounted pane id.
    fn record_rich_edit_undo(
        state: &mut RichEditorState,
        bus: &Arc<Mutex<crate::interop::interaction_bus::InteractionBus>>,
        state_arc: &Arc<Mutex<RichEditorState>>,
        before: serde_json::Value,
        after: serde_json::Value,
    ) {
        use crate::rich_editor::interop_adapter::{push_or_coalesce_rich_edit_undo, RichSnapshotApplier};

        let Some(pane_id) = state.undo_pane_id.clone() else {
            return; // not mounted; the bus undo wiring is inert (never faked).
        };
        let should_push = state.undo_batcher.should_push(std::time::Instant::now());
        // The batch-start snapshot: on a fresh push it is THIS edit's `before`; within a window it is the
        // snapshot captured at the batch start (kept on the state).
        let batch_before = if should_push {
            state.undo_batch_before = Some(before.clone());
            before.clone()
        } else {
            state.undo_batch_before.clone().unwrap_or_else(|| before.clone())
        };

        // The applier the bus closures call to write a snapshot back into the live document. It rebuilds
        // the doc tree from content_json (the verified MT-011 round-trip), resets the caret, and resets
        // the parallel UndoManager so it cannot fight the bus (ONE authority).
        let restore: RichSnapshotApplier<RichEditorState> = Arc::new(|s: &mut RichEditorState, snap| {
            if let Ok(doc) =
                crate::rich_editor::document_model::doc_json::from_json_value(snap)
            {
                s.doc = doc;
                s.selection = Selection::caret(DocPosition::new(vec![0, 0], 0));
                s.undo = UndoManager::new();
            }
        });

        // The entry captures a `Weak<Mutex<RichEditorState>>` to the SHARED state Arc the widget owns
        // (RISK-3 / MC-3): it upgrades only during a bus-driven undo/redo and writes the snapshot back.
        crate::interop::interaction_bus::InteractionBus::with_try_lock(bus, |b| {
            let _pushed = push_or_coalesce_rich_edit_undo(
                b,
                pane_id.clone(),
                state_arc,
                should_push,
                batch_before,
                before,
                after,
                restore,
                "rich: edit",
            );
        });
    }

    /// MT-020: trigger a canonical save of the live document. Captures the current `content_json`
    /// from the doc (NEVER a stale snapshot — MC-001), records it as the pending local content (so a
    /// resulting 409 conflict carries the operator's version), and asks the save manager to save.
    /// No-op when no save context is installed or a save is already in flight (MC-002).
    fn trigger_save(state: &mut RichEditorState) {
        // One save substrate: the keyboard Ctrl+S path and the host menu Save both route to
        // `RichEditorState::request_save_for_host` (MT-020 SaveManager), so they can never diverge.
        let _ = state.request_save_for_host();
    }

    // ── WP-KERNEL-012 MT-041 (E7): consolidated editor-action AccessKit surface ──────────────────────

    /// Sync this rich pane's canonical `editor.rich.<action>` nodes into the installed registry, emit
    /// them into the live AccessKit tree, and CONSUME any swarm `Action::Click` dispatched at them this
    /// frame, routing each to the real editor action it aliases (RISK-041-04 / CTRL-041-04). A no-op
    /// when no registry is installed.
    ///
    /// CONSOLIDATION (anti-duplication): the nodes are the ONE swarm-facing surface; they alias the
    /// existing toolbar / find / save dispatch paths rather than re-minting parallel nodes (IN-041-08).
    /// A format ToggleButton's `checked` state is read live via `is_mark_active` so it never reports
    /// stale state when the cursor moves into/out of a mark (RISK-041-03 / CTRL-041-03).
    fn sync_editor_actions(ui: &egui::Ui, state: &mut RichEditorState) {
        use crate::rich_editor::document_model::node::Mark;
        use crate::rich_editor::formatting::commands::is_mark_active;

        let Some((registry, handle)) = state.editor_actions.clone() else {
            return;
        };
        let find_open = state.find_replace.is_some();
        // Live toggle states from the real editor (no mocks).
        let bold = is_mark_active(&state.doc, &state.selection, &Mark::Bold);
        let italic = is_mark_active(&state.doc, &state.selection, &Mark::Italic);
        let code = is_mark_active(&state.doc, &state.selection, &Mark::Code);
        let (fc_case, fc_word, fc_regex) = state
            .find_replace
            .as_ref()
            .map(|f| (f.query.case_sensitive, f.query.whole_word, f.query.is_regex))
            .unwrap_or((false, false, false));

        let catalog = rich_action_catalog();
        // 1) Register/refresh every catalog node with its live state.
        {
            let mut reg = registry.lock().unwrap_or_else(|e| e.into_inner());
            for entry in &catalog {
                let author_id = handle.author_id(entry.action_id);
                let state_for = Self::rich_action_state(
                    entry, find_open, bold, italic, code, fc_case, fc_word, fc_regex,
                );
                reg.upsert(author_id, entry.role, entry.label, state_for);
            }
            // AC-041-04 analog: a present-only `editor.rich.find-panel` node while find is open.
            reg.upsert(
                handle.author_id("find-panel"),
                AxRole::Button,
                "Find panel",
                if find_open {
                    EditorActionState { present: true, enabled: false, checked: None }
                } else {
                    EditorActionState::absent()
                },
            );
            // HBR-QUIET (IN-041-09): repaint only on a real present-set change.
            if reg.state_changed_since_last_push() {
                ui.ctx().request_repaint();
            }
        }
        // 2) Emit + 3) consume this frame's dispatch.
        let dispatched = {
            let reg = registry.lock().unwrap_or_else(|e| e.into_inner());
            reg.emit_into_tree(ui);
            reg.take_dispatched(ui)
        };
        for author_id in dispatched {
            let action_id = Self::strip_rich_author_prefix(&author_id, handle);
            if let Some(entry) = catalog.iter().find(|e| e.action_id == action_id) {
                Self::run_rich_dispatch(state, &entry.dispatch, &action_id, ui.ctx());
            }
        }
    }

    /// The live [`EditorActionState`] for one rich catalog entry, from the real editor state.
    #[allow(clippy::too_many_arguments)]
    fn rich_action_state(
        entry: &crate::accessibility::editor_action_registry::RichActionEntry,
        find_open: bool,
        bold: bool,
        italic: bool,
        code: bool,
        fc_case: bool,
        fc_word: bool,
        fc_regex: bool,
    ) -> EditorActionState {
        // Find-step / replace / find-toggle nodes are present ONLY while the find panel is open.
        let find_scoped = matches!(
            entry.action_id,
            "find-next" | "find-prev" | "find-toggle-case" | "find-toggle-word" | "find-toggle-regex"
                | "replace-one" | "replace-all"
        );
        let present = if find_scoped { find_open } else { entry.always_present };
        if !present {
            return EditorActionState::absent();
        }
        // h4..6 are a documented model gap (only h1..3 exist): present but DISABLED so a dispatch is
        // rejected by the MCP channel rather than silently mis-applied (typed limitation, no mock).
        let enabled = !rich_heading_is_unsupported(entry.action_id);
        match entry.role {
            AxRole::Button => EditorActionState { present, enabled, checked: None },
            AxRole::ToggleButton => {
                let checked = Some(match entry.action_id {
                    "format-bold" => bold,
                    "format-italic" => italic,
                    "format-code" => code,
                    "find-toggle-case" => fc_case,
                    "find-toggle-word" => fc_word,
                    "find-toggle-regex" => fc_regex,
                    _ => false,
                });
                EditorActionState { present, enabled, checked }
            }
        }
    }

    /// Strip the `editor.rich.` prefix (+ optional `.<idx>`) from a canonical author_id.
    fn strip_rich_author_prefix(author_id: &str, handle: RegistrationHandle) -> String {
        let rest = author_id.strip_prefix("editor.rich.").unwrap_or(author_id);
        if handle.instance_index() > 0 {
            let suffix = format!(".{}", handle.instance_index());
            rest.strip_suffix(&suffix).unwrap_or(rest).to_owned()
        } else {
            rest.to_owned()
        }
    }

    /// Run one canonical rich-action dispatch target against the real editor state (alias-to-real).
    fn run_rich_dispatch(
        state: &mut RichEditorState,
        target: &RichDispatch,
        action_id: &str,
        ctx: &egui::Context,
    ) {
        use crate::rich_editor::find_replace::{self, FindReplaceState};
        use crate::rich_editor::formatting::commands::{self, CommandContext};
        match target {
            RichDispatch::Format(cmd) => {
                // h4..6 are disabled nodes; a dispatch should never reach here (MCP rejects disabled),
                // but guard anyway so a stray dispatch is a no-op, not a wrong heading level.
                if rich_heading_is_unsupported(action_id) {
                    tracing::debug!(action_id, "rich heading >3 unsupported by the model; no-op");
                    return;
                }
                let RichEditorState { doc, undo, selection, actor_id, .. } = state;
                let mut cctx = CommandContext::new(doc, undo, selection, actor_id.as_str());
                let _ = commands::dispatch(&mut cctx, cmd);
            }
            RichDispatch::FindOpen => {
                if state.find_replace.is_none() {
                    state.find_replace = Some(FindReplaceState::open(false));
                    if let Some(f) = state.find_replace.as_mut() {
                        f.rescan(&state.doc);
                    }
                }
            }
            RichDispatch::FindNext => {
                if let Some(f) = state.find_replace.as_mut() {
                    f.select_next();
                }
            }
            RichDispatch::FindPrev => {
                if let Some(f) = state.find_replace.as_mut() {
                    f.select_prev();
                }
            }
            RichDispatch::ReplaceOne => {
                let RichEditorState { doc, undo, selection, find_replace, .. } = state;
                if let Some(f) = find_replace.as_mut() {
                    if let Some(active) = f.active.and_then(|i| f.scan.matches.get(i).cloned()) {
                        let repl = f.replacement.clone();
                        find_replace::replace_one(doc, undo, selection, &active, &repl);
                        f.rescan(doc);
                    }
                }
            }
            RichDispatch::ReplaceAll => {
                let RichEditorState { doc, undo, selection, find_replace, .. } = state;
                if let Some(f) = find_replace.as_mut() {
                    let matches = f.scan.matches.clone();
                    let repl = f.replacement.clone();
                    find_replace::replace_all(doc, undo, selection, &matches, &repl);
                    f.rescan(doc);
                }
            }
            RichDispatch::FindToggleCase => {
                if let Some(f) = state.find_replace.as_mut() {
                    f.query.case_sensitive = !f.query.case_sensitive;
                    f.rescan(&state.doc);
                }
            }
            RichDispatch::FindToggleWord => {
                if let Some(f) = state.find_replace.as_mut() {
                    f.query.whole_word = !f.query.whole_word;
                    f.rescan(&state.doc);
                }
            }
            RichDispatch::FindToggleRegex => {
                if let Some(f) = state.find_replace.as_mut() {
                    f.query.is_regex = !f.query.is_regex;
                    f.rescan(&state.doc);
                }
            }
            RichDispatch::Save => {
                // CTRL-041-06: route through the MT-020 SaveManager (the E6/MT-037 knowledge_documents
                // save client), never a new direct call.
                Self::trigger_save(state);
            }
            RichDispatch::InsertSlashCommand => {
                // Open the slash-command block-insert picker at the caret (the same `SlashMenuState`
                // surface the `/` trigger opens). The caret's leaf path + char offset anchor it.
                if state.slash_menu.is_none() {
                    if let crate::rich_editor::document_model::selection::Selection::Text {
                        head, ..
                    } = &state.selection
                    {
                        state.slash_menu = Some(
                            crate::rich_editor::slash_commands::SlashMenuState::open(
                                head.path.clone(),
                                head.char_offset,
                            ),
                        );
                        ctx.request_repaint();
                    }
                }
            }
            RichDispatch::CommandPaletteOpen => {
                // Route to the shared WP-011 command palette via the interaction bus command surface
                // (the EXISTING command_palette.rs modal reads `command_palette_open` — no second palette).
                let bus = crate::interop::interaction_bus::InteractionBus::get_or_init(ctx);
                crate::interop::interaction_bus::InteractionBus::with_try_lock(&bus, |b| {
                    b.open_command_palette();
                });
            }
        }
    }

    /// MT-020: drive the save + draft coordinators each frame. Drains a completed off-thread save
    /// result (applying Saved/Conflict/Failed); on a successful save clears the draft + re-bases it.
    /// Drains a completed draft-load result (offering the recovery banner). Fires the debounced draft
    /// upsert IF due AND no save is in flight (MC-002). Requests a repaint when any async result
    /// landed so the new state paints without waiting for the next input event.
    fn drive_save_and_draft(ctx: &egui::Context, state: &mut RichEditorState) {
        let mut applied = false;
        // Drain a completed save result.
        let save_outcome = state.save.as_mut().and_then(|s| s.drain());
        if let Some(outcome) = save_outcome {
            use crate::rich_editor::save::save_manager::SaveOutcome;
            applied = true;
            match outcome {
                SaveOutcome::Saved { doc_version } => {
                    // The canonical save landed: clear the server draft + re-base the draft manager on
                    // the just-saved content + version (so a later edit's draft bases correctly).
                    let saved_content =
                        crate::rich_editor::document_model::doc_json::to_content_json_value(&state.doc);
                    // MT-036 (E5 — one event ledger): emit a REAL `document_saved` FlightEvent at this
                    // LIVE save-success call site (the MT-020 path that exists + is tested). The content
                    // hash is recomputed locally via the MT-032 canonical writer (matches the backend's
                    // server-side hash). The pane id is the MT-035 undo pane id; absent it (a bare unit
                    // test that never mounted), the emit is skipped — never a fake. The emit routes off
                    // the frame thread + bounded via the bus's installed emitter (a no-op until the shell
                    // installs the emitter — the unmounted-pane defer policy).
                    if let Some(pane_id) = state.undo_pane_id.clone() {
                        let document_id =
                            state.save.as_ref().map(|s| s.document_id().to_owned()).unwrap_or_default();
                        if !document_id.is_empty() {
                            let content_hash =
                                crate::loom_address::ContentHash::of_content_json(&saved_content)
                                    .as_str()
                                    .to_owned();
                            let bus =
                                crate::interop::interaction_bus::InteractionBus::get_or_init(ctx);
                            crate::interop::interaction_bus::InteractionBus::with_try_lock(&bus, |b| {
                                let ev = crate::event_emitter::NativeEditorEvent::document_saved(
                                    document_id,
                                    content_hash,
                                    pane_id.as_ref(),
                                    crate::event_emitter::native_editor_actor_id(pane_id.as_ref()),
                                    b.event_emitter()
                                        .map(|e| e.workspace_id().to_owned())
                                        .unwrap_or_default(),
                                );
                                b.emit_event(ev);
                            });
                        }
                    }
                    if let Some(draft) = state.draft.as_mut() {
                        draft.clear_after_save(doc_version, &saved_content);
                    }
                }
                SaveOutcome::Conflict | SaveOutcome::Failed(_) => {
                    // The conflict window / error chip renders from the save state; nothing else here.
                }
            }
        }
        // Drain a completed draft-load result (recovery banner offer).
        if let Some(draft) = state.draft.as_mut() {
            if draft.drain_load() {
                applied = true;
            }
        }
        // Fire the debounced draft upsert if due and no save is in flight (MC-002).
        let save_in_flight = state.save.as_ref().map(|s| s.is_saving()).unwrap_or(false);
        let content = if state.draft.is_some() {
            Some(crate::rich_editor::document_model::doc_json::to_content_json_value(&state.doc))
        } else {
            None
        };
        if let (Some(draft), Some(content)) = (state.draft.as_mut(), content) {
            let now = std::time::Instant::now();
            if draft.maybe_upsert(content, now, save_in_flight) {
                applied = true;
            }
            // Keep animating while dirty so the debounce can fire on a later frame without new input.
            if draft.banner_visible() || save_in_flight {
                ctx.request_repaint_after(std::time::Duration::from_millis(250));
            }
        }
        // MT-020 (red-team RISK-4): poll the in-flight native save-dialog handle NON-BLOCKINGLY. The
        // dialog runs on a dedicated thread; `poll` returns `None` while it is still open (the frame
        // thread never blocks), `Some(_)` once the operator picks a path or cancels — then we drop the
        // handle. While the dialog is open we keep a slow repaint pulse so the resolution is observed
        // promptly without busy-spinning.
        if let Some(pending) = state.pending_file_save.as_ref() {
            match pending.poll() {
                Some(_outcome) => {
                    state.pending_file_save = None;
                    applied = true;
                }
                None => {
                    ctx.request_repaint_after(std::time::Duration::from_millis(150));
                }
            }
        }
        if applied {
            ctx.request_repaint();
        }
    }

    /// MT-020: render the conflict window when a save conflict is open, and apply the operator's
    /// choice against the save manager + doc. "Keep server" rebuilds the live doc from the server
    /// content; "Keep yours" routes through the MC-003 confirmation before the overwrite.
    fn render_conflict_window(ctx: &egui::Context, state: &mut RichEditorState, palette: &HsPalette) {
        use crate::rich_editor::save::conflict_ui::{show_conflict_window, ConflictOutcome};
        use crate::rich_editor::document_model::doc_json::from_json_value;

        let Some(save) = state.save.as_ref() else { return };
        if !save.has_conflict() {
            return;
        }
        let outcome = show_conflict_window(ctx, save, palette);
        match outcome {
            ConflictOutcome::None => {}
            ConflictOutcome::RequestKeepYours => {
                if let Some(s) = state.save.as_mut() {
                    s.request_keep_yours();
                }
            }
            ConflictOutcome::ConfirmKeepYours => {
                if let Some(s) = state.save.as_mut() {
                    // Re-thread the live content as the overwrite payload, then confirm.
                    let content =
                        crate::rich_editor::document_model::doc_json::to_content_json_value(&state.doc);
                    s.set_pending_local_content(content);
                    s.confirm_keep_yours();
                }
            }
            ConflictOutcome::CancelKeepYours => {
                if let Some(s) = state.save.as_mut() {
                    s.cancel_keep_yours();
                }
            }
            ConflictOutcome::KeepServer => {
                // Adopt the server content: rebuild the live doc from it, reset the caret to the start,
                // and clear the conflict.
                let server_content = state.save.as_mut().and_then(|s| s.keep_server());
                if let Some(content) = server_content {
                    if let Ok(doc) = from_json_value(&content) {
                        state.doc = doc;
                        state.selection =
                            Selection::caret(DocPosition::new(vec![0, 0], 0));
                        state.undo = UndoManager::new();
                    }
                }
            }
        }
    }

    /// MT-020: render the draft-recovery banner (only when a recoverable draft is available) and
    /// apply the operator's choice. "Restore draft" rebuilds the live doc from the draft content;
    /// "Discard" clears the server draft; "Keep editing" dismisses the banner without discarding.
    fn render_draft_banner(ui: &mut egui::Ui, state: &mut RichEditorState, palette: &HsPalette) {
        use crate::rich_editor::save::conflict_ui::{show_draft_banner, DraftBannerOutcome};
        use crate::rich_editor::document_model::doc_json::from_json_value;

        let Some(draft) = state.draft.as_ref() else { return };
        if !draft.banner_visible() {
            return;
        }
        let outcome = show_draft_banner(ui, draft, palette);
        match outcome {
            DraftBannerOutcome::None => {}
            DraftBannerOutcome::Restore => {
                let restored = state.draft.as_mut().and_then(|d| d.restore_draft());
                if let Some(content) = restored {
                    if let Ok(doc) = from_json_value(&content) {
                        state.doc = doc;
                        state.selection = Selection::caret(DocPosition::new(vec![0, 0], 0));
                        state.undo = UndoManager::new();
                        // A restored draft is unsaved -> mark dirty so a later Ctrl+S persists it.
                        if let Some(save) = state.save.as_mut() {
                            save.mark_dirty();
                        }
                    }
                }
            }
            DraftBannerOutcome::Discard => {
                if let Some(d) = state.draft.as_mut() {
                    d.discard_draft();
                }
            }
            DraftBannerOutcome::Dismiss => {
                if let Some(d) = state.draft.as_mut() {
                    d.dismiss_banner();
                }
            }
        }
    }

    /// MT-020: render the export format-picker popup when open, and run the chosen export to bytes.
    /// The bytes are written through the production [`NativeFileSaveSink::spawn`] — the `rfd` dialog
    /// on a DEDICATED thread (HBR-QUIET / MC-004), which returns a [`PendingFileSave`] handle stored
    /// on the state and polled non-blockingly in [`Self::drive_save_and_draft`] (so the frame thread
    /// never blocks while the dialog is open — red-team RISK-4). Asset resolution for HTML
    /// self-contained is left to a future wiring step; this MT exports the non-image formats directly
    /// and HTML with reference-linked media (no frame-thread network).
    fn render_export_picker(ui: &mut egui::Ui, state: &mut RichEditorState, _palette: &HsPalette) {
        use crate::rich_editor::save::conflict_ui::{show_export_picker, NativeFileSaveSink};
        use crate::rich_editor::save::export::{export_document, AssetByteSource, ExportFormat};

        if !state.export_picker_open {
            return;
        }
        let mut chosen: Option<ExportFormat> = None;
        egui::Area::new(ui.id().with("export-picker-area"))
            .order(egui::Order::Foreground)
            .show(ui.ctx(), |ui| {
                chosen = show_export_picker(ui);
            });
        if let Some(format) = chosen {
            state.export_picker_open = false;
            let workspace_id = state.embeds.workspace_id.clone();
            let title = state
                .properties
                .as_ref()
                .map(|p| p.doc_metadata.title.clone())
                .unwrap_or_else(|| "document".to_owned());
            // HTML self-contained needs resolved asset bytes (fetched off-thread); without a wired
            // resolver in this MT, an empty asset map degrades each image to a visible "unresolved"
            // reference placeholder (never a silent blank) — the export still completes.
            let assets = AssetByteSource::new();
            if let Ok(output) = export_document(
                &state.doc,
                format,
                &workspace_id,
                crate::backend_client::BACKEND_BASE_URL,
                &title,
                &assets,
            ) {
                // The real dialog is user-initiated (the operator just clicked a format) and runs on a
                // dedicated thread (HBR-QUIET). `spawn` returns IMMEDIATELY with a pollable handle — it
                // never blocks the frame thread; the host polls it in `drive_save_and_draft`.
                state.pending_file_save = Some(NativeFileSaveSink::spawn(&output));
            }
        }
    }

    /// MT-018: apply a Ctrl+F / Ctrl+H shortcut. Ctrl+F opens the panel in find-only mode; Ctrl+H in
    /// find+replace mode. If a panel is ALREADY open, the shortcut re-focuses the find input and (for
    /// Ctrl+H) reveals the replace row — it does NOT discard the in-progress query (re-pressing Ctrl+F
    /// while searching keeps the term, matching VS Code). Opening recomputes the scan against the
    /// current document so the count + highlights are live immediately.
    fn apply_find_replace_shortcut(
        state: &mut RichEditorState,
        shortcut: input_handler::FindReplaceShortcut,
    ) {
        use crate::rich_editor::find_replace::FindReplaceState as FrState;
        use input_handler::FindReplaceShortcut;

        let with_replace = matches!(shortcut, FindReplaceShortcut::OpenReplace);
        match state.find_replace.as_mut() {
            Some(panel) => {
                // Already open: re-focus the find input; Ctrl+H additionally reveals the replace row.
                panel.focus_find_input = true;
                if with_replace {
                    panel.with_replace = true;
                }
            }
            None => {
                let mut panel = FrState::open(with_replace);
                panel.rescan(&state.doc); // initial scan (empty query -> empty scan).
                state.find_replace = Some(panel);
            }
        }
    }

    /// Handle the autocomplete popup's navigation/confirm/cancel keys while it is open. Consumes
    /// Up/Down (move selection), Enter/Tab (confirm the selected result -> insert the hsLink atom),
    /// and Escape (cancel -> remove the `[[` trigger text). Returns the events that were NOT claimed
    /// by the popup (so the normal editing path still handles plain typing that refines the query).
    fn handle_autocomplete_keys(state: &mut RichEditorState, events: Vec<egui::Event>) -> Vec<egui::Event> {
        use crate::rich_editor::wikilinks::confirm;

        let mut remaining = Vec::with_capacity(events.len());
        for ev in events {
            let egui::Event::Key { key, pressed: true, .. } = &ev else {
                remaining.push(ev);
                continue;
            };
            match key {
                egui::Key::ArrowDown => {
                    if let Some(ac) = state.wikilink_autocomplete.as_mut() {
                        ac.select_next();
                    }
                }
                egui::Key::ArrowUp => {
                    if let Some(ac) = state.wikilink_autocomplete.as_mut() {
                        ac.select_prev();
                    }
                }
                egui::Key::Enter | egui::Key::Tab => {
                    // Confirm the selected result as an inserted hsLink atom (NOT a mark).
                    let confirmed = state.wikilink_autocomplete.as_ref().and_then(|ac| {
                        let result = ac.selected_result()?;
                        // The chosen result's content type maps to a backend ref kind; default to a
                        // `note` link for a document/note hit (the common autocomplete target).
                        let ref_kind = result_ref_kind(&result.content_type);
                        Some((
                            ac.leaf_path.clone(),
                            ac.trigger_start_char,
                            crate::rich_editor::document_model::node::HsLinkNode::new(
                                ref_kind,
                                result.block_id.clone(),
                                result.title.clone(),
                            ),
                        ))
                    });
                    if let Some((leaf_path, trigger_start, link)) = confirmed {
                        let caret_char = Self::caret_char_offset(state);
                        let RichEditorState { doc, selection, .. } = state;
                        confirm::confirm_wikilink(doc, selection, &leaf_path, trigger_start, caret_char, link);
                    }
                    // Close the popup whether or not a result was selected (Enter on an empty popup
                    // just closes it; the typed `[[` text is left as plain text).
                    state.wikilink_autocomplete = None;
                }
                egui::Key::Escape => {
                    // Cancel: remove the `[[query` trigger text and close (AC: Escape closes + removes).
                    if let Some(ac) = state.wikilink_autocomplete.take() {
                        let caret_char = Self::caret_char_offset(state);
                        let RichEditorState { doc, selection, .. } = state;
                        confirm::cancel_wikilink(doc, selection, &ac.leaf_path, ac.trigger_start_char, caret_char);
                    }
                }
                // Any other key (plain typing, backspace) passes through to the normal editing path,
                // which mutates the leaf text; the trigger re-detection then refines the query.
                _ => remaining.push(ev),
            }
        }
        remaining
    }

    /// Re-detect the `[[` autocomplete trigger from the caret's current text-before-caret. Opens the
    /// popup when a new open trigger appears, updates the query (debounce/generation) when it refines,
    /// and closes the popup when the caret leaves the open token (`]]` typed, or the trigger gone).
    fn refresh_autocomplete_trigger(state: &mut RichEditorState) {
        use crate::rich_editor::wikilinks::autocomplete::AutocompleteState;
        use crate::rich_editor::wikilinks::parser::open_wikilink_query;

        // The caret's text leaf + the text before the caret within it.
        let Some((leaf_path, before)) = Self::caret_text_before(state) else {
            state.wikilink_autocomplete = None; // no text caret -> no popup.
            return;
        };
        match open_wikilink_query(&before) {
            Some((trigger_start_char, query)) => {
                match state.wikilink_autocomplete.as_mut() {
                    // Same open token in the same leaf -> update the query (debounce + generation).
                    Some(ac) if ac.leaf_path == leaf_path && ac.trigger_start_char == trigger_start_char => {
                        ac.set_query(query);
                    }
                    // A new/relocated trigger -> open a fresh popup.
                    _ => {
                        state.wikilink_autocomplete =
                            Some(AutocompleteState::open(trigger_start_char, leaf_path, query));
                    }
                }
            }
            // No open trigger before the caret -> close the popup.
            None => state.wikilink_autocomplete = None,
        }
    }

    /// The caret's text leaf path + the text before the caret within that leaf (for `[[` trigger
    /// detection). Returns `None` when the selection is not a text caret resolving to a text leaf.
    fn caret_text_before(state: &RichEditorState) -> Option<(Vec<usize>, String)> {
        let Selection::Text { head, .. } = &state.selection else {
            return None;
        };
        let (leaf_idx, block_path) = head.path.split_last()?;
        let mut node = &state.doc;
        for &idx in block_path {
            node = node.children.get(idx)?.as_block()?;
        }
        let leaf = node.children.get(*leaf_idx)?.as_text()?;
        let before = leaf.text.slice_chars(0, head.char_offset);
        Some((head.path.clone(), before))
    }

    /// WP-KERNEL-012 MT-058: re-detect the inline-tag `#` trigger from the caret's text-before-caret.
    /// Opens the tag menu when a new open `#` trigger appears at a word boundary, updates the live filter
    /// when the body refines, and closes the menu when the caret leaves the open token (a space / non-tag
    /// char typed after the body, the `#` deleted, or the trigger gone). Mirrors
    /// [`Self::refresh_autocomplete_trigger`] for `[[`, using the egui-free
    /// [`crate::rich_editor::inline_tags::open_tag_query`] detector (parser.rs stays AccessKit/egui-free,
    /// AC-007). Byte-safe on multi-byte UTF-8 (the parser walks chars — RISK-003 / MC-003).
    fn refresh_tag_trigger(state: &mut RichEditorState) {
        use crate::rich_editor::inline_tags::{open_tag_query, TagAutocompleteState};

        let Some((leaf_path, before)) = Self::caret_text_before(state) else {
            state.tag_autocomplete = None; // no text caret -> no menu.
            return;
        };
        match open_tag_query(&before) {
            Some((trigger_start_char, query)) => {
                match state.tag_autocomplete.as_mut() {
                    // Same open token in the same leaf -> update the live filter (selection resets).
                    Some(ac)
                        if ac.leaf_path == leaf_path
                            && ac.trigger_start_char == trigger_start_char =>
                    {
                        ac.set_query(query);
                    }
                    // A new/relocated trigger -> open a fresh menu.
                    _ => {
                        state.tag_autocomplete =
                            Some(TagAutocompleteState::open(trigger_start_char, leaf_path, query));
                    }
                }
            }
            // No open trigger before the caret -> close the menu.
            None => state.tag_autocomplete = None,
        }
    }

    /// WP-KERNEL-012 MT-058: handle the inline-tag `#` menu's navigation/confirm/cancel keys while it is
    /// open. Consumes Up/Down (move selection over the filtered item list), Enter/Tab (commit the
    /// selected — or free-typed — tag as a `Child::HsLink(ref_kind="tag")` atom via the EXISTING
    /// [`crate::rich_editor::wikilinks::confirm::confirm_wikilink`] insert), and Escape (cancel ->
    /// remove the `#query` trigger text). Returns the events NOT claimed by the menu (plain typing that
    /// refines the filter passes through). The menu item source is the cached MT-023 tag list filtered
    /// by the live query, ALWAYS allowing a free-typed NEW tag (AC-006).
    fn handle_tag_menu_keys(state: &mut RichEditorState, events: Vec<egui::Event>) -> Vec<egui::Event> {
        use crate::rich_editor::inline_tags::tag_menu_items;

        let mut remaining = Vec::with_capacity(events.len());
        for ev in events {
            let egui::Event::Key { key, pressed: true, .. } = &ev else {
                remaining.push(ev);
                continue;
            };
            match key {
                egui::Key::ArrowDown => {
                    let count = state
                        .tag_autocomplete
                        .as_ref()
                        .map(|ac| tag_menu_items(&ac.query, &state.tag_list).len())
                        .unwrap_or(0);
                    if let Some(ac) = state.tag_autocomplete.as_mut() {
                        ac.select_next(count);
                    }
                }
                egui::Key::ArrowUp => {
                    if let Some(ac) = state.tag_autocomplete.as_mut() {
                        ac.select_prev();
                    }
                }
                egui::Key::Enter | egui::Key::Tab => {
                    Self::commit_selected_tag(state);
                }
                egui::Key::Escape => {
                    // Cancel: remove the `#query` trigger text and close the menu.
                    if let Some(ac) = state.tag_autocomplete.take() {
                        let caret_char = Self::caret_char_offset(state);
                        let RichEditorState { doc, selection, .. } = state;
                        crate::rich_editor::wikilinks::confirm::cancel_wikilink(
                            doc,
                            selection,
                            &ac.leaf_path,
                            ac.trigger_start_char,
                            caret_char,
                        );
                    }
                }
                // Any other key (plain typing, backspace) passes through to the normal editing path,
                // which mutates the leaf text; the trigger re-detection then refines the filter.
                _ => remaining.push(ev),
            }
        }
        remaining
    }

    /// WP-KERNEL-012 MT-058: commit the currently-selected inline-tag menu row (or, when the menu has no
    /// rows but the query is a valid tag, the free-typed tag) as a `Child::HsLink(ref_kind="tag")` atom
    /// at the `#` trigger span, then close the menu. Reuses the EXISTING `confirm_wikilink` insert (the
    /// `[[` confirm path) so the tag atom round-trips `content_json` and the caret lands after the chip.
    /// A no-op when the resolved tag identity is empty (a bare `#` — never committed).
    fn commit_selected_tag(state: &mut RichEditorState) {
        use crate::rich_editor::inline_tags::{menu_item_to_hs_link, tag_menu_items, TagMenuItem};

        let Some(ac) = state.tag_autocomplete.as_ref() else {
            return;
        };
        let items = tag_menu_items(&ac.query, &state.tag_list);
        // The selected row, or — when the list is empty (no existing match + an empty/invalid query) —
        // a free-typed tag from the raw query (so Enter on a typed `#wip` with no list still commits).
        let chosen: Option<TagMenuItem> = items
            .get(ac.selected)
            .cloned()
            .or_else(|| {
                let q = ac.query.trim();
                if q.is_empty() {
                    None
                } else {
                    Some(TagMenuItem::new_tag(q))
                }
            });
        let Some(item) = chosen else {
            // Nothing to commit (empty query, no rows): just close the menu, leave the `#` as text.
            state.tag_autocomplete = None;
            return;
        };
        let link = menu_item_to_hs_link(&item);
        // A bare `#` (empty canonical) is never committed (the atom would be identity-less).
        if link.ref_value.is_empty() {
            state.tag_autocomplete = None;
            return;
        }
        let leaf_path = ac.leaf_path.clone();
        let trigger_start = ac.trigger_start_char;
        let caret_char = Self::caret_char_offset(state);
        {
            let RichEditorState { doc, selection, .. } = state;
            crate::rich_editor::wikilinks::confirm::confirm_wikilink(
                doc,
                selection,
                &leaf_path,
                trigger_start,
                caret_char,
                link,
            );
        }
        state.tag_autocomplete = None;
    }

    /// The caret's in-leaf char offset (the head's `char_offset`), or 0 for a non-text selection.
    fn caret_char_offset(state: &RichEditorState) -> usize {
        match &state.selection {
            Selection::Text { head, .. } => head.char_offset,
            Selection::Node { .. } => 0,
        }
    }

    /// WP-KERNEL-012 MT-033 (E5 — CKC drag-in): insert `link` (a CKC/Atelier embed built from the dropped
    /// item, an `hsLink` atom by CKC `refKind`) as an inline atom at the caret, splitting the caret's text
    /// leaf so the atom lands at the exact drop position (text before stays in the original leaf; text
    /// after moves to the trailing leaf). The caret is then placed just after the inserted atom. Returns
    /// `true` when the insertion happened.
    ///
    /// The atom is the EXISTING [`crate::rich_editor::document_model::node::Child::HsLink`] variant (the
    /// MT-014 media-embed lesson), so the inserted embed ROUND-TRIPS the backend `content_json` exactly
    /// like a wikilink / media embed (AC-2) — never an invented `atelier_embed` node the backend would
    /// drop on save. When the selection is not a text caret (e.g. an empty doc), the atom is appended to
    /// the first paragraph's content so a drop is never a silent no-op.
    pub fn insert_atelier_embed_at_caret(
        state: &mut RichEditorState,
        link: crate::rich_editor::document_model::node::HsLinkNode,
    ) -> bool {
        use crate::rich_editor::document_model::node::{Child, TextLeaf};
        use crate::rich_editor::document_model::position::DocPosition;

        // Resolve the caret leaf path + offset, or fall back to the first inline-content block.
        let (leaf_path, caret_char) = match &state.selection {
            Selection::Text { head, .. } => (head.path.clone(), head.char_offset),
            Selection::Node { .. } => {
                // No text caret: target the first paragraph/heading's last text leaf (append).
                match Self::first_inline_leaf_path(&state.doc) {
                    Some((path, end)) => (path, end),
                    None => return false,
                }
            }
        };

        let Some((leaf_idx, parent_path)) = leaf_path.split_last() else {
            return false;
        };

        // Split the caret leaf into head `[0, caret_char)` + tail `[caret_char, end)` so the atom lands at
        // the exact drop position. The tail (when the caret is mid-leaf) becomes the caret-host leaf. When
        // the caret is NOT on a text leaf (e.g. on an atom), there is no split (`None`) and the atom is
        // inserted after that child.
        let tail_text: Option<String> = {
            let mut node = &mut state.doc;
            for &idx in parent_path {
                match node.children.get_mut(idx).and_then(Child::as_block_mut) {
                    Some(b) => node = b,
                    None => return false,
                }
            }
            match node.children.get_mut(*leaf_idx).and_then(Child::as_text_mut) {
                Some(leaf) => {
                    let len = leaf.text.len_chars();
                    let split = caret_char.min(len);
                    if split < len {
                        let full = leaf.text.to_string();
                        let tail: String = full.chars().skip(split).collect();
                        leaf.text.remove(split, len);
                        Some(tail)
                    } else {
                        None
                    }
                }
                None => None,
            }
        };

        // Re-resolve the parent block (the borrow above ended) and insert the atom after the caret leaf.
        let mut node = &mut state.doc;
        for &idx in parent_path {
            match node.children.get_mut(idx).and_then(Child::as_block_mut) {
                Some(b) => node = b,
                None => return false,
            }
        }
        if *leaf_idx >= node.children.len() {
            return false;
        }
        let insert_at = *leaf_idx + 1;
        node.children.insert(insert_at, Child::HsLink(link));

        // The caret host is the leaf immediately after the atom: the split tail (if any) or a fresh empty
        // leaf so the caret has a text position to land on (the caret model needs trailing inline text).
        let trailing_idx = insert_at + 1;
        let trailing_text = tail_text.unwrap_or_default();
        let needs_new_trailing = node
            .children
            .get(trailing_idx)
            .map(|c| c.as_text().is_none())
            .unwrap_or(true);
        if needs_new_trailing {
            node.children
                .insert(trailing_idx, Child::Text(TextLeaf::new(&trailing_text)));
        } else if !trailing_text.is_empty() {
            if let Some(leaf) = node.children.get_mut(trailing_idx).and_then(Child::as_text_mut) {
                leaf.text.insert(0, &trailing_text);
            }
        }
        // Mark the document dirty so the embed is persisted on the next save (AC-2 round-trip).
        if let Some(save) = state.save.as_mut() {
            save.mark_dirty();
        }
        let mut caret_path = parent_path.to_vec();
        caret_path.push(trailing_idx);
        state.selection = Selection::caret(DocPosition::new(caret_path, 0));
        true
    }

    /// The path + end-offset of the first inline-content block's last text leaf, for an embed dropped when
    /// no text caret is active. `None` when the doc has no inline-content block with a text leaf.
    fn first_inline_leaf_path(doc: &BlockNode) -> Option<(Vec<usize>, usize)> {
        for (bi, child) in doc.children.iter().enumerate() {
            let Some(block) = child.as_block() else { continue };
            if !block.kind.holds_inline_content() {
                continue;
            }
            // Find the last text leaf in this block.
            for (li, c) in block.children.iter().enumerate().rev() {
                if let Some(t) = c.as_text() {
                    return Some((vec![bi, li], t.text.len_chars()));
                }
            }
        }
        None
    }

    /// MT-016: claim the slash menu's nav/confirm/cancel keys while it is open. Up/Down move the
    /// selection (clamped to the filtered length), Enter executes the selected command, Escape cancels
    /// (closing the menu and leaving the `/` in the text — AC-5). Returns the events NOT claimed (so
    /// plain typing that refines the filter still reaches the editing path, and the filter is then
    /// re-detected by [`Self::refresh_slash_trigger`]).
    fn handle_slash_menu_keys(state: &mut RichEditorState, events: Vec<egui::Event>) -> Vec<egui::Event> {
        use crate::rich_editor::slash_commands::menu::SlashMenuOutcome;
        use crate::rich_editor::slash_commands::registry::filter_slash_commands;

        let mut remaining = Vec::with_capacity(events.len());
        let mut decisive: SlashMenuOutcome = SlashMenuOutcome::None;
        for ev in events {
            let egui::Event::Key { key, pressed: true, .. } = &ev else {
                remaining.push(ev);
                continue;
            };
            match key {
                egui::Key::ArrowDown | egui::Key::ArrowUp => {
                    if let Some(menu) = state.slash_menu.as_mut() {
                        let filtered = filter_slash_commands(&menu.filter);
                        if !filtered.is_empty() {
                            let max = filtered.len() as i64 - 1;
                            let delta = if matches!(key, egui::Key::ArrowDown) { 1 } else { -1 };
                            let cur = (menu.selected as i64).min(max);
                            menu.selected = (cur + delta).clamp(0, max) as usize;
                        }
                    }
                }
                egui::Key::Enter => {
                    if let Some(menu) = state.slash_menu.as_ref() {
                        let filtered = filter_slash_commands(&menu.filter);
                        if !filtered.is_empty() {
                            decisive = SlashMenuOutcome::Execute(menu.selected.min(filtered.len() - 1));
                        } else {
                            decisive = SlashMenuOutcome::Cancel;
                        }
                    }
                }
                egui::Key::Escape => {
                    decisive = SlashMenuOutcome::Cancel;
                }
                // Any other key (plain typing, backspace) passes through; the trigger re-detection
                // then refines/closes the menu from the resulting leaf text.
                _ => remaining.push(ev),
            }
        }
        match decisive {
            SlashMenuOutcome::Execute(idx) => Self::execute_slash_selection(state, idx),
            SlashMenuOutcome::Cancel => {
                // AC-5: Escape (or Enter on an empty list) closes the menu, leaving the `/` in the text.
                state.slash_menu = None;
            }
            SlashMenuOutcome::None => {}
        }
        remaining
    }

    /// MT-016: execute the slash command at `filtered_index` into the CURRENTLY filtered list, via the
    /// executor. Translates the executor outcome into editor state: `Done` closes the menu;
    /// `OpenPrompt` keeps the menu open carrying the prompt (the list hides, the modal shows);
    /// `OpenWikilinkAutocomplete` closes the menu (the autocomplete refresh then opens the popup).
    fn execute_slash_selection(state: &mut RichEditorState, filtered_index: usize) {
        use crate::rich_editor::slash_commands::executor::{
            execute_slash_command, SlashExecContext, SlashExecOutcome,
        };
        use crate::rich_editor::slash_commands::registry::filter_slash_commands;

        let Some(menu) = state.slash_menu.clone() else { return };
        let filtered = filter_slash_commands(&menu.filter);
        let Some(cmd) = filtered.get(filtered_index).copied() else {
            state.slash_menu = None;
            return;
        };
        let outcome = {
            let RichEditorState { doc, selection, undo, actor_id, .. } = state;
            let mut ctx = SlashExecContext {
                doc,
                history: undo,
                selection,
                actor_id: actor_id.as_str(),
            };
            execute_slash_command(&mut ctx, &menu, cmd)
        };
        match outcome {
            SlashExecOutcome::Done { .. } => state.slash_menu = None,
            SlashExecOutcome::OpenPrompt(prompt) => {
                // Keep the menu open carrying the prompt; the list hides and the modal renders.
                if let Some(m) = state.slash_menu.as_mut() {
                    m.prompt = Some(prompt);
                }
            }
            SlashExecOutcome::OpenWikilinkAutocomplete => {
                // The `[[` was inserted; close the slash menu and let the autocomplete refresh take it.
                state.slash_menu = None;
            }
            SlashExecOutcome::OpenCodeSymbolSearch => {
                // MT-034: close the slash menu and open the code-symbol search dialog (scoped to the
                // editor's code-ref workspace + runtime context). The dialog renders next frame; on
                // select it inserts a `code` hsLink atom at the caret (where the `/code-ref` was).
                state.slash_menu = None;
                state.code_symbol_search = Some(
                    crate::rich_editor::slash_commands::code_symbol_search::CodeSymbolSearchState::open(
                        state.code_ref_workspace_id.clone(),
                        state.code_ref_runtime.clone(),
                    ),
                );
            }
        }
    }

    /// MT-016: re-detect the `/` slash-command trigger from the caret's text-before-caret. Opens the
    /// menu when a new open `/` trigger appears (at a blank-line start or after whitespace, never inside
    /// a URL/path — RISK-1/MC-001), updates the filter when it refines, and closes the menu when the
    /// trigger is gone (the `/` was backspaced, a space ended the token, or the caret moved out).
    fn refresh_slash_trigger(state: &mut RichEditorState) {
        use crate::rich_editor::slash_commands::{
            caret_char_offset, caret_leaf_text, open_slash_trigger, SlashMenuState,
        };

        // The caret's leaf text + the caret's char offset within it.
        let (Some((leaf_path, leaf_text)), Some(caret_char)) = (
            caret_leaf_text(&state.doc, &state.selection),
            caret_char_offset(&state.selection),
        ) else {
            state.slash_menu = None; // no text caret -> no menu.
            return;
        };
        // A prompt modal owns the input — never touch the menu while one is up.
        let prompt_active = state.slash_menu.as_ref().is_some_and(|m| m.prompt_active());
        if prompt_active {
            return;
        }
        match open_slash_trigger(&leaf_text, caret_char) {
            Some((trigger_char, filter)) => {
                // Is the existing menu the SAME open token (same leaf + trigger char)?
                let same_token = state.slash_menu.as_ref().is_some_and(|m| {
                    m.trigger_leaf_path == leaf_path && m.trigger_char == trigger_char
                });
                if same_token {
                    // Refine the filter in place (reset the selection on a change).
                    if let Some(menu) = state.slash_menu.as_mut() {
                        if menu.filter != filter {
                            menu.filter = filter;
                            menu.selected = 0;
                        }
                    }
                } else {
                    // A new/relocated trigger -> open a fresh menu.
                    let mut menu = SlashMenuState::open(leaf_path, trigger_char);
                    menu.filter = filter;
                    state.slash_menu = Some(menu);
                }
            }
            // No open trigger -> close the menu.
            None => state.slash_menu = None,
        }
    }

    /// Paint ONE inline wikilink chip: a colored rounded rect at the (scroll-adjusted) glyph span +
    /// the label text on top, an interactive AccessKit node (`wikilink-chip-{hash}`, Role::Link), and
    /// click handling that returns a `WikilinkActivated` event (the caller enqueues it). `origin` is
    /// the block's painted screen top-left (already scroll-adjusted — RISK-1 / MC-001).
    fn paint_one_wikilink_chip(
        ui: &mut egui::Ui,
        spec: &WikilinkChipSpec,
        origin: egui::Pos2,
        palette: &HsPalette,
        creating: bool,
    ) -> Option<crate::rich_editor::wikilinks::inline_view::EditorEvent> {
        use crate::rich_editor::wikilinks::inline_view::{
            chip_author_id, chip_rect_for_span, code_ref_chip_author_id, create_affordance_author_id,
            is_code_ref, is_locus_ref, locus_ref_chip_author_id, EditorEvent, CHIP_ROLE,
        };

        let rect = chip_rect_for_span(spec.local_start, spec.local_end, origin);
        // Paint the chip background (rounded) then the label text in the chip text color, on top of
        // the painter-drawn label (so the chip reads as a pill). Colors are theme tokens (CONTROL-4).
        let painter = ui.painter();
        painter.rect_filled(rect, 4.0, spec.bg);
        painter.text(
            egui::pos2(rect.min.x + 1.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            &spec.label,
            egui::FontId::proportional(super::line_layout::BASE_FONT_SIZE),
            spec.fg,
        );

        // An interactive (clickable) node over the chip rect, addressable by the stable chip author_id.
        // MT-034: a code ref gets the contract `code-ref-chip-{symbol_entity_id}` id (the symbol the
        // chip references — what a kittest / swarm agent targets); other wikilinks keep the hashed id.
        // WP-KERNEL-012 MT-058: an inline TAG atom (ref_kind="tag") gets the contract `inline-tag-{name}`
        // id (the canonical tag identity — what a swarm agent / kittest targets), Role::Link.
        let is_tag = crate::rich_editor::inline_tags::is_tag_link(&spec.link);
        let author = if is_tag {
            crate::rich_editor::inline_tags::inline_tag_author_id(
                &crate::rich_editor::inline_tags::tag_from_link(&spec.link),
            )
        } else if is_code_ref(&spec.link) {
            code_ref_chip_author_id(&spec.link.ref_value)
        } else if is_locus_ref(&spec.link) {
            // WP-KERNEL-012 MT-068: a Locus ref gets the contract `locus-ref-chip-{kind}-{id}` id (the
            // WP/MT the chip references — what a kittest / swarm agent targets), Role::Link.
            locus_ref_chip_author_id(&spec.link.ref_value)
        } else {
            chip_author_id(&spec.link.ref_value)
        };
        // WP-KERNEL-012 MT-058 (MC-006): fold the chip's distinct on-screen position into the egui Id so
        // two occurrences of the SAME tag/ref (e.g. two `#rust` in one doc) get DISTINCT NodeIds while
        // sharing the same addressable `author_id` STRING. Without this, `ui.id().with((..,&author))`
        // collides for an identical author and only ONE chip node reaches the AccessKit tree (the
        // repeated-tag collision the contract red-teams). The position is quantized to whole pixels so a
        // sub-pixel jitter does not churn the id frame-to-frame.
        let pos_key = (rect.min.x.round() as i32, rect.min.y.round() as i32);
        let chip_id = ui.id().with(("wikilink-chip", &author, pos_key));
        let resp = ui.interact(rect, chip_id, egui::Sense::click());
        let role = CHIP_ROLE;
        let author_for_node = author.clone();
        let label_for_node = spec.label.clone();
        ui.ctx().accesskit_node_builder(chip_id, move |node| {
            node.set_role(role);
            node.set_author_id(author_for_node.clone());
            node.set_label(label_for_node.clone());
        });

        // WP-KERNEL-012 MT-057: an UNRESOLVED wikilink offers a "Create note" affordance to the RIGHT of
        // the chip — a small Button addressable by the STABLE `wikilink-create-{hash}` author_id
        // (MC-005), Role::Button, so a swarm agent / kittest targets it deterministically. While a
        // create for this title is in flight the affordance is DISABLED (MC-001 — no duplicate POST) and
        // reads "Creating…". Clicking it emits the COMMAND-BUS CreateNote intent (NOT an inline POST —
        // RISK-007 / MC-007); the async handler creates the note + rewrites this mark to resolved.
        let mut create_event: Option<EditorEvent> = None;
        if let Some(title) = spec.create_title.clone() {
            let create_author = create_affordance_author_id(&title);
            let create_id = ui.id().with(("wikilink-create", &create_author));
            // Place the button just right of the chip, same row height.
            let btn_rect = egui::Rect::from_min_size(
                egui::pos2(rect.max.x + 4.0, rect.min.y),
                egui::vec2(96.0, rect.height().max(super::line_layout::BASE_FONT_SIZE)),
            );
            let (label_text, label_color, sense) = if creating {
                ("Creating…", palette.text_subtle, egui::Sense::hover())
            } else {
                ("＋ Create note", palette.accent, egui::Sense::click())
            };
            let painter = ui.painter();
            painter.rect_filled(btn_rect, 4.0, palette.surface);
            painter.rect_stroke(
                btn_rect,
                4.0,
                egui::Stroke::new(1.0, palette.border),
                egui::StrokeKind::Inside,
            );
            painter.text(
                egui::pos2(btn_rect.min.x + 4.0, btn_rect.center().y),
                egui::Align2::LEFT_CENTER,
                label_text,
                egui::FontId::proportional(super::line_layout::BASE_FONT_SIZE - 1.0),
                label_color,
            );
            let create_resp = ui.interact(btn_rect, create_id, sense);
            let author_for_create = create_author.clone();
            let title_for_label = title.clone();
            ui.ctx().accesskit_node_builder(create_id, move |node| {
                node.set_role(egui::accesskit::Role::Button);
                node.set_author_id(author_for_create.clone());
                node.set_label(format!("Create note \"{title_for_label}\""));
            });
            // The chip itself, when unresolved, also activates the create (clicking the broken link is
            // the Obsidian gesture); the dedicated button is the discoverable affordance + the stable
            // swarm/kittest target.
            if !creating && (create_resp.clicked() || resp.clicked()) {
                create_event = Some(EditorEvent::CreateNote { title });
            }
            return create_event;
        }

        if resp.clicked() {
            // WP-KERNEL-012 MT-058: a TAG chip emits a TagActivated navigation event (NOT
            // WikilinkActivated) so the host routes it onto the WP-011 bus to open the MT-023 tag hub —
            // the chip never opens the hub directly (RISK-005 / MC-005). Carries the canonical identity
            // (the hub-resolution key) + the original-case display name.
            if is_tag {
                let tag = crate::rich_editor::inline_tags::tag_from_link(&spec.link);
                Some(EditorEvent::TagActivated {
                    canonical: tag.canonical(),
                    display: tag.name,
                })
            } else {
                Some(EditorEvent::WikilinkActivated {
                    ref_kind: spec.link.ref_kind.clone(),
                    ref_value: spec.link.ref_value.clone(),
                    resolved: spec.link.resolved,
                })
            }
        } else {
            None
        }
    }

    /// MT-017: render the document properties panel inside a default-collapsed `CollapsingHeader`
    /// ABOVE the content area. The header's open/closed state is keyed per document_id in egui
    /// persistent storage (impl note) so it does not reset on every re-render. Drains the properties
    /// runtime FIRST so a completed rename/count lands before the panel paints. When no document
    /// metadata is loaded yet, an honest "No document loaded" placeholder renders (NOT a spinner).
    fn render_properties(ui: &mut egui::Ui, state: &mut RichEditorState, palette: &HsPalette) {
        use crate::rich_editor::properties::metadata_client::EguiClipboard;
        use crate::rich_editor::properties::panel::PropertiesPanel;

        // Drain any completed rename/move + backlinks-count results. A fresh rename result refreshes the
        // displayed metadata (the `set_metadata` keeps the local-only tags). The count NEVER overwrites
        // the doc metadata (MC-004) — `drain` returns it separately, applied inside the runtime.
        let (fresh_metadata, applied) = state.properties_runtime.drain();
        if let (Some(meta), Some(props)) = (fresh_metadata, state.properties.as_mut()) {
            props.set_metadata(meta);
        }
        if applied {
            ui.ctx().request_repaint();
        }

        // Key the collapsing header per document so its open/closed state is per-document (impl note).
        let doc_key = state
            .properties
            .as_ref()
            .map(|p| p.doc_metadata.rich_document_id.clone())
            .unwrap_or_else(|| "no-doc".to_owned());

        let header = egui::CollapsingHeader::new("Properties")
            .id_salt(("properties-header", &doc_key))
            .default_open(false) // AC-1: collapsed by default.
            .show(ui, |ui| {
                if state.properties.is_some() {
                    // Borrow the panel pieces. The clipboard sink wraps the egui context (production
                    // surface); a headless test injects a mock via a direct `PropertiesPanel` call.
                    let clipboard = EguiClipboard::new(ui.ctx().clone());
                    let props = state.properties.as_mut().expect("checked is_some");
                    PropertiesPanel::new(props, &mut state.properties_runtime, &clipboard, palette).show(ui);
                } else {
                    // Honest empty state — no document metadata loaded yet. NOT a spinner (the panel
                    // only loads metadata when the shell installs the context).
                    ui.colored_label(palette.text_subtle, "No document loaded.");
                }
            });

        // The collapsing-header bar is an interactive (clickable) node, so it MUST carry a stable
        // author_id or the shell HBR-SWARM gate (assert_no_unnamed_interactive) panics. Give the
        // collapse control its own id (`properties-header`) distinct from the content container
        // (`properties-panel`, emitted on the grid in panel.rs) so a swarm agent can expand/collapse the
        // panel by a stable key without ambiguity.
        ui.ctx().accesskit_node_builder(header.header_response.id, move |node| {
            node.set_author_id("properties-header".to_owned());
        });
    }

    /// MT-018: render the find/replace panel (when open) and apply its outcome. The panel is a pure
    /// view that edits `find_replace.query`/`replacement`/`active` in place and returns a typed
    /// [`crate::rich_editor::find_replace::panel::PanelOutcome`]; this host re-scans on a query change
    /// and performs the actual document replace through the MT-011 transaction path
    /// (`find_replace::replace_one` / `replace_all`), so the doc-mutating logic lives in one place.
    fn render_find_panel(ctx: &egui::Context, state: &mut RichEditorState, palette: &HsPalette) {
        use crate::rich_editor::find_replace::panel::{show_find_panel, PanelOutcome};
        use crate::rich_editor::find_replace::{replace_all, replace_one};

        if state.find_replace.is_none() {
            return;
        }

        // Render the panel against its own state (a temporary take avoids a double borrow of `state`
        // while the panel edits its query/replacement; we put it back unless it asked to close).
        let mut panel = state.find_replace.take().expect("checked is_some");
        let (outcome, query_changed) = show_find_panel(ctx, &mut panel, palette);

        // A query / option change re-runs the scan immediately so the count + highlights update this
        // frame (the panel edited the query in place; the scan is the host's responsibility).
        if query_changed {
            panel.rescan(&state.doc);
        }

        match outcome {
            PanelOutcome::None => {
                state.find_replace = Some(panel);
            }
            PanelOutcome::ReplaceOne => {
                // Replace the CURRENT match (default to the first match when none is active yet),
                // then re-scan and advance to the next match (the React "keep going" behavior).
                let index = panel.active.unwrap_or(0);
                if let Some(m) = panel.scan.matches.get(index).cloned() {
                    let RichEditorState { doc, undo, selection, .. } = state;
                    let replaced = replace_one(doc, undo, selection, &m, &panel.replacement);
                    if replaced {
                        panel.rescan(&state.doc);
                        // After the replacement, the match set shrank; keep the index pointing at the
                        // NEXT match (clamped) so repeated Replace walks forward through the document.
                        panel.active = if panel.scan.is_empty() {
                            None
                        } else {
                            Some(index.min(panel.scan.len() - 1))
                        };
                    }
                }
                state.find_replace = Some(panel);
            }
            PanelOutcome::ReplaceAll => {
                let matches = panel.scan.matches.clone();
                {
                    let RichEditorState { doc, undo, selection, .. } = state;
                    let _n = replace_all(doc, undo, selection, &matches, &panel.replacement);
                }
                panel.rescan(&state.doc); // the doc changed -> the matches are gone (no active match).
                panel.active = None;
                state.find_replace = Some(panel);
            }
            PanelOutcome::Close => {
                // Drop the panel state -> the highlights stop painting next frame (cleared) and the
                // editor keeps focus. We deliberately do NOT put `panel` back.
                ctx.request_repaint();
            }
        }
    }

    /// The stable AccessKit author_id for the toolbar "Export…" button (the operator-reachable entry
    /// to the export format picker — a swarm agent can trigger an export by this key).
    pub const EXPORT_BUTTON_AUTHOR_ID: &str = "rich-editor-export-button";

    /// Render the MT-013 formatting toolbar (a horizontal glyph-button row grouped by
    /// category) above the content area, plus the MT-020 "Export…" button. The toolbar borrows the
    /// editor state by `&mut` (doc/undo/selection) so a button click dispatches a command STANDALONE
    /// on the local state (COMMAND DISPATCH REALITY gate — the host bus Sender is E11/MT-069). The
    /// Export button arms the export format picker ([`Self::render_export_picker`]); without it the
    /// picker + the export-to-bytes path are unreachable (the must-fix #2 dead-code gap).
    fn render_toolbar(ui: &mut egui::Ui, state: &mut RichEditorState) {
        ui.horizontal(|ui| {
            {
                let RichEditorState { doc, selection, undo, actor_id, .. } = state;
                let cctx = crate::rich_editor::formatting::commands::CommandContext::new(
                    doc,
                    undo,
                    selection,
                    actor_id.as_str(),
                );
                let _dispatched =
                    crate::rich_editor::formatting::toolbar::EditorToolbar::new(cctx).show(ui);
            }
            // MT-020: the "Export…" button opens the export format picker popup. It is an interactive
            // node, so it MUST carry a stable author_id (the shell HBR-SWARM gate panics on an unnamed
            // interactive node). Clicking it toggles the picker open (a second click closes it).
            let export = ui.button("Export…");
            let author = Self::EXPORT_BUTTON_AUTHOR_ID.to_owned();
            ui.ctx().accesskit_node_builder(export.id, move |node| {
                node.set_author_id(author.clone());
            });
            if export.clicked() {
                state.export_picker_open = !state.export_picker_open;
            }
        });
    }

    /// Render every top-level block inside a vertical scroll area, then paint the caret on
    /// top of the block that hosts it (resolved natively from that block's galley). Emits a
    /// per-block AccessKit node for each rendered block. Drives the blink repaint only when
    /// focused.
    fn render_blocks(
        ui: &mut egui::Ui,
        state: &mut RichEditorState,
        palette: &HsPalette,
        has_focus: bool,
        read_only: bool,
    ) {
        // WP-KERNEL-012 MT-055: in reading mode NO caret/selection is resolved (RISK-005 / MC-005 —
        // the read-only branch allocates no caret state and paints no caret). `caret_block = None`
        // means `paint_block` is called with `caret_offset = None` for every block (the same value a
        // non-caret block gets in Edit mode), so the SAME MT-012 block renderer runs with zero caret
        // affordance — no second renderer (RISK-001 / MC-001). The blink repaint is likewise never
        // scheduled in reading mode (HBR-QUIET: a static reading view does not animate).
        let caret = DocCaret::from_selection(&state.selection);
        let caret_block = if read_only { None } else { caret.block_index() };
        let time = ui.input(|i| i.time);
        let blink_on = !read_only && blink_visible(time);
        // Query once per frame whether the bold Inter family is bound (the shell binds it
        // at startup; a bare/no-fonts context does not). Threaded into every block layout
        // so a bold run never requests an unbound family (panic guard).
        let bold_available = super::line_layout::bold_family_available(ui.ctx());

        // MT-014: drain any off-thread embed resolutions that completed since last frame into
        // the embed caches BEFORE rendering, so a just-resolved embed shows its media/error this
        // frame. A delivery schedules a repaint so the new state is not stuck behind an idle frame.
        if state.embeds.drain_deliveries() {
            ui.ctx().request_repaint();
        }

        // MT-015: drain any off-thread wikilink resolutions (transclusion / backlinks) + autocomplete
        // search results into their caches BEFORE rendering, with generation-counter cancellation
        // (MC-004). Then, while a popup is open, issue the debounced search (MC-002).
        let mut wikilink_applied = state.wikilinks.drain();
        if state.wikilinks.autocomplete.drain(&mut state.wikilink_autocomplete) {
            wikilink_applied = true;
        }
        // WP-KERNEL-012 MT-057: drain a completed create-from-unresolved. On success the runtime already
        // inserted the new title into the resolver index (so the link resolves live); here we rewrite
        // the ORIGINATING mark Unresolved -> Resolved so it re-renders as a live link WITHOUT a reload
        // (AC-002). A failure clears the in-flight guard (the affordance re-enables) and the editor keeps
        // the link unresolved (no silent success).
        if let Some(outcome) = state.wikilinks.drain_create() {
            use crate::rich_editor::wikilinks::runtime::CreateNoteOutcome;
            if let CreateNoteOutcome::Created { normalized_title, display_title, document_id } = outcome {
                crate::rich_editor::wikilinks::confirm::rewrite_mark_to_resolved(
                    &mut state.doc,
                    &normalized_title,
                    &document_id,
                    &display_title,
                );
            }
            wikilink_applied = true;
        }
        if let Some(ac) = state.wikilink_autocomplete.as_mut() {
            // The autocomplete runtime lives inside the wikilink runtime; borrow it to issue the
            // debounced search for the current query (no-op until the 150ms window elapses).
            state.wikilinks.autocomplete.maybe_search(ac, std::time::Instant::now());
            // Keep animating so the debounce fires on a later frame even without new input.
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(60));
        }
        if wikilink_applied {
            ui.ctx().request_repaint();
        }

        // WP-KERNEL-012 MT-057 (AC-006 / RISK-002 / MC-002): when the backend payload lacks an
        // `aliases` field, alias resolution runs LOCAL-ONLY (the in-session stub). Render a VISIBLE
        // banner so the operator is NEVER misled into thinking aliases are persisted. Editing surface
        // only (skipped in reading mode). The banner is an addressable AccessKit-labeled node so a
        // swarm agent reads the local-only state by a stable id.
        if !read_only && state.wikilinks.alias_backend_gap {
            let banner = egui::Frame::new()
                .fill(palette.error_bg)
                .inner_margin(egui::Margin::symmetric(8, 4))
                .show(ui, |ui| {
                    ui.colored_label(
                        palette.error_text,
                        "Alias resolution is running LOCAL-ONLY — backend aliases are unavailable (not persisted).",
                    );
                });
            ui.ctx().accesskit_node_builder(banner.response.id, |node| {
                node.set_author_id("wikilink-alias-local-only-banner".to_owned());
                node.set_label(
                    "Alias resolution local-only: backend aliases unavailable".to_owned(),
                );
            });
        }

        // MT-015: reserve a bottom strip for the backlinks panel so the full-height content scroll
        // area does not push it off-screen (HBR-VIS: the panel must be visible, not just present in
        // the tree). Cap the scroll area's height to leave room for the panel below; the panel
        // collapses to its header height when closed, so the reserve is a soft upper bound.
        // WP-KERNEL-012 MT-055: in reading mode the backlinks strip (an editing/knowledge surface) is
        // NOT shown, so the reading view uses the full content height.
        const BACKLINKS_STRIP_PTS: f32 = 150.0;
        let available_h = ui.available_height().max(1.0);
        let scroll_max_h = if read_only {
            available_h
        } else {
            (available_h - BACKLINKS_STRIP_PTS).max(available_h * 0.4)
        };

        // WP-KERNEL-012 MT-055 (AC-005): reading typography. In reading mode the content column is
        // CLAMPED to a centered reading measure ([`READING_COLUMN_WIDTH_PTS`]) and an extra
        // paragraph gap is added between blocks, both NAMED consts (the THEME TYPOGRAPHY note allows
        // named column-width/spacing consts; colors stay theme tokens). In Edit mode there is no
        // clamp/indent and no extra gap (the exact MT-012 layout — AC-008 no regression).
        use crate::rich_editor::reading_mode::{
            READING_COLUMN_WIDTH_PTS, READING_EXTRA_BLOCK_SPACING_PTS,
        };
        let extra_block_gap = if read_only { READING_EXTRA_BLOCK_SPACING_PTS } else { 0.0 };

        let caret_galley_out = egui::ScrollArea::vertical()
            .id_salt("rich-editor-scroll")
            .auto_shrink([false, false])
            .max_height(scroll_max_h)
            .show(ui, |ui| {
                let avail_width = ui.available_width().max(1.0);
                // The reading-mode content column: clamp to the reading measure and CENTER it (a left
                // margin of half the slack) so long notes get a distraction-free measure. In Edit mode
                // the column is the full available width with no left margin (MT-012 unchanged).
                let (content_width, left_margin) = if read_only {
                    let col = avail_width.min(READING_COLUMN_WIDTH_PTS);
                    let margin = ((avail_width - col) / 2.0).max(0.0);
                    (col, margin)
                } else {
                    (avail_width, 0.0)
                };
                let mut top = ui.cursor().min;
                top.x += left_margin;
                // Painter for the whole content region (block_renderer paints absolute).
                let painter = ui.painter().clone();

                // Collect caret galley + origin for the caret's block so we can paint the
                // caret after all blocks (so it sits on top).
                let mut caret_galley: Option<(std::sync::Arc<egui::Galley>, egui::Pos2)> = None;

                for idx in 0..state.doc.children.len() {
                    let Some(block) = state.doc.children[idx].as_block() else { continue };

                    // MT-014: a standalone media-embed block routes to the INTERACTIVE embed
                    // renderer (it owns an egui::Ui for buttons/modals), not the painter path.
                    // We clone the small HsLinkNode out so the doc borrow ends before borrowing
                    // `state.embeds` mutably.
                    if let Some(link) = block_media_embed(block).cloned() {
                        let embed_rect = egui::Rect::from_min_size(
                            top,
                            egui::vec2(content_width, ui.available_height().max(1.0)),
                        );
                        let mut child = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(embed_rect)
                                .id_salt(("rich-editor-embed", idx))
                                .layout(egui::Layout::top_down(egui::Align::Min)),
                        );
                        embed_block_renderer::render_embed(&mut child, &link, &mut state.embeds, palette);
                        let used = child.min_rect().height().max(super::line_layout::BASE_FONT_SIZE);
                        top.y += used + super::line_layout::BLOCK_GAP_PTS + extra_block_gap;
                        continue;
                    }

                    // MT-015: a standalone transclusion block routes to the INTERACTIVE transclusion
                    // read-through view (it owns an egui::Ui for the Open block / Remove embed buttons),
                    // mirroring the media-embed dispatch. We clone the small node out so the doc borrow
                    // ends before borrowing `state.wikilinks` mutably.
                    if let Some(tnode) = super::block_renderer::block_transclusion(block).cloned() {
                        let t_rect = egui::Rect::from_min_size(
                            top,
                            egui::vec2(content_width, ui.available_height().max(1.0)),
                        );
                        let mut child = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(t_rect)
                                .id_salt(("rich-editor-transclusion", idx))
                                .layout(egui::Layout::top_down(egui::Align::Min)),
                        );
                        let (event, removed) = crate::rich_editor::wikilinks::transclusion_view::render_transclusion(
                            &mut child,
                            &tnode,
                            &mut state.wikilinks,
                            palette,
                        );
                        if let Some(ev) = event {
                            state.pending_events.push(ev);
                        }
                        // MC-003: the operator clicked "Remove embed" on a 404 transclusion -> delete the
                        // node from the doc (drop this standalone-transclusion paragraph).
                        if removed {
                            if let Some(parent) = state.doc.children.get_mut(idx).and_then(Child::as_block_mut) {
                                parent.children.retain(|c| c.as_transclusion().is_none());
                            }
                            ui.ctx().request_repaint();
                        }
                        let used = child.min_rect().height().max(super::line_layout::BASE_FONT_SIZE);
                        top.y += used + super::line_layout::BLOCK_GAP_PTS + extra_block_gap;
                        continue;
                    }

                    let block = state.doc.children[idx].as_block().expect("checked above");
                    let caret_offset = if caret_block == Some(idx) {
                        Some(caret.char_offset())
                    } else {
                        None
                    };
                    let bp = paint_block(&painter, block, top, content_width, palette, caret_offset, bold_available);
                    if let Some(g) = bp.caret_galley {
                        caret_galley = Some(g.clone());
                    }

                    // WP-KERNEL-012 MT-056 (E2 — outline/TOC): if the outline requested a scroll to THIS
                    // top-level block, bring its painted rect into view through the EXISTING scroll area
                    // (RISK-002 / MC-002 — `ui.scroll_to_rect` on the SAME `rich-editor-scroll` ScrollArea,
                    // not a second scroll mechanism). One-shot: the pending target is cleared once consumed
                    // so steady-state frames do no scroll work, and a repaint is scheduled so the scroll
                    // offset settles. The block is painted with the absolute `painter` at `top`, so its
                    // rect is `[top, content_width x bp.height]`.
                    if state.pending_scroll_block.as_deref() == Some(&[idx][..]) {
                        let block_rect = egui::Rect::from_min_size(top, egui::vec2(content_width, bp.height));
                        ui.scroll_to_rect(block_rect, Some(egui::Align::TOP));
                        state.pending_scroll_block = None;
                        ui.ctx().request_repaint();
                    }

                    // MT-015: overlay the inline wikilink CHIPs on this paragraph's hsLink atoms. We
                    // re-layout the block (cheap) to get the galley, find each hsLink's char span in
                    // the laid-out text, and paint a colored rounded chip at the glyph span — the chip
                    // Y is the (already scroll-adjusted) paint origin `top` (RISK-1 / MC-001). Each chip
                    // is an interactive AccessKit node (`wikilink-chip-{hash}`, Role::Link); a click
                    // enqueues a WikilinkActivated event for the shell. The chip specs are computed
                    // into an owned vec FIRST (ending the doc borrow) so `pending_events` can be
                    // borrowed mutably when handling a click.
                    let chip_specs = wikilink_chip_specs(
                        block,
                        palette,
                        content_width,
                        bold_available,
                        &painter,
                        &state.wikilinks.resolver_index,
                    );
                    for spec in chip_specs {
                        // WP-KERNEL-012 MT-057: an in-flight create (keyed on the normalized title)
                        // DISABLES the create affordance so a double-click cannot POST twice (MC-001).
                        let creating = spec
                            .create_title
                            .as_ref()
                            .map(|t| state.wikilinks.is_creating(t))
                            .unwrap_or(false);
                        if let Some(ev) = Self::paint_one_wikilink_chip(ui, &spec, top, palette, creating) {
                            // WP-KERNEL-012 MT-057: a CreateNote intent is DISPATCHED on the runtime
                            // (off-thread `POST /knowledge/documents` via the MT-037 binding — never
                            // inline on this frame, RISK-007/MC-007) AND enqueued for the shell to
                            // observe. The dispatch guards against a duplicate in-flight create (MC-001).
                            if let crate::rich_editor::wikilinks::inline_view::EditorEvent::CreateNote { title } = &ev {
                                state.wikilinks.dispatch_create_note(title);
                                ui.ctx().request_repaint();
                            }
                            state.pending_events.push(ev);
                        }
                    }

                    // MT-018: overlay the find/replace match highlights on THIS block. Each match in
                    // this top-level block resolves to a galley span (the same `pos_from_cursor`
                    // mechanism as the chips), painted as a semi-transparent wash over the glyphs —
                    // the current match brighter (~60%) than the others (~25%). The block is re-laid
                    // inside `match_highlight_rects` (cheap) to get its galley; the rects are painted
                    // AFTER the content so the wash sits on top. Computed into an owned vec so the doc
                    // borrow ends before painting (the panel/scan live on `state`).
                    if let Some(panel) = state.find_replace.as_ref() {
                        let block = state.doc.children[idx].as_block().expect("checked above");
                        let current = panel.active.and_then(|i| panel.scan.matches.get(i));
                        let rects = crate::rich_editor::find_replace::highlight_layer::match_highlight_rects(
                            block,
                            idx,
                            &panel.scan.matches,
                            current,
                            top,
                            content_width,
                            bold_available,
                            palette,
                            &painter,
                        );
                        crate::rich_editor::find_replace::highlight_layer::paint_highlights(&painter, &rects);
                    }

                    // Per-block AccessKit node (re-block-{hash}, Role::Paragraph) for
                    // paragraphs/headings — the addressable block units a swarm agent
                    // targets. We open a real CHILD Ui scope keyed by the stable block id
                    // and emit the node onto THAT scope's `ui.unique_id()`. A scope Ui
                    // registers its parent in egui's `parent_map`, so the node nests under
                    // the editor root (egui parents an accesskit_node_builder node via the
                    // egui Id parent chain; a bare hashed id with no registered parent would
                    // attach to the WINDOW root instead — proven by the ancestry dump). The
                    // swarm match key is the STABLE `author_id` string (re-block-{hash}),
                    // independent of the egui Id, so addressing-by-author_id is unaffected.
                    if matches!(block.kind, NodeKind::Paragraph | NodeKind::Heading(_)) {
                        let path = vec![idx];
                        let author = block_author_id(&path);
                        let label = state.block_plain_text(idx).unwrap_or_default();
                        let scope_salt = block_node_id(&path);
                        ui.scope_builder(egui::UiBuilder::new().id_salt(scope_salt), |ui| {
                            let node_id = ui.unique_id();
                            ui.ctx().accesskit_node_builder(node_id, move |node| {
                                node.set_role(BLOCK_ROLE);
                                node.set_author_id(author.clone());
                                node.set_label(label.clone());
                            });
                        });
                    }

                    top.y += bp.height + extra_block_gap;
                }

                // Reserve the painted vertical extent so the scroll area sizes correctly. The reading
                // column's left margin is included so the scroll content is sized to the centered column.
                let used_height = (top.y - ui.cursor().min.y).max(0.0);
                ui.allocate_space(egui::vec2(content_width + left_margin, used_height));

                // Paint the blinking caret over the blocks (only when collapsed + blink-on). WP-KERNEL-012
                // MT-055: never in reading mode — `caret_galley` is `None` there (caret_block is None), so
                // this is already inert, and `read_only` makes the guard explicit (RISK-005 belt-and-braces).
                if !read_only {
                    if let Some((galley, origin)) = caret_galley.clone() {
                        paint_caret(&painter, &galley, origin, &caret, palette, blink_on);
                    }
                }
                // Return the caret galley+origin so the popup (rendered OUTSIDE this scroll closure)
                // can anchor at the caret pixel.
                caret_galley
            })
            .inner;

        // WP-KERNEL-012 MT-055: the backlinks panel + the caret-anchored autocomplete / slash-command /
        // code-ref popups are EDITING/knowledge surfaces (they author links, insert blocks, or list
        // backlinks). In reading mode they are SKIPPED — a clean read-only presentation has no editing
        // popups, and skipping them guarantees no editable TextInput child lands in the AccessKit tree
        // for the read-only document body (RISK-002 / AC-003). The inline wikilink chips painted above
        // STAY interactive (RISK-003) so navigation still works in the reading view.
        if !read_only {
            // MT-015: the backlinks side panel below the content area (a collapsible header listing every
            // document linking to the current one). Reuses the existing shell theme; clicking an entry
            // enqueues a BacklinkActivated event for the host shell to route.
            ui.separator();
            if let Some(ev) = crate::rich_editor::wikilinks::backlinks_panel::render_backlinks_panel(
                ui,
                &mut state.wikilinks,
                palette,
            ) {
                state.pending_events.push(ev);
            }

            // MT-015: the autocomplete popup anchored at the CARET pixel (impl note: caret position, NOT
            // mouse, so keyboard-only typing positions it). Resolved from the caret block's galley.
            if state.wikilink_autocomplete.is_some() {
                let caret_pixel = caret_galley_out.as_ref().map(|(galley, origin)| {
                    let cursor = egui::epaint::text::cursor::CCursor::new(caret.char_offset());
                    let local = galley.pos_from_cursor(cursor);
                    egui::pos2(origin.x + local.min.x, origin.y + local.max.y)
                });
                Self::render_autocomplete_popup(ui, state, palette, caret_pixel);
            }

            // MT-016: the slash-command menu popup (list) or its active prompt modal, anchored at the caret
            // pixel (same caret-galley resolution as the autocomplete popup). Rendered AFTER the content so
            // it sits above the blocks. The widget returns an outcome the host applies to the editor state.
            if state.slash_menu.is_some() {
                let caret_pixel = caret_galley_out.as_ref().map(|(galley, origin)| {
                    let cursor = egui::epaint::text::cursor::CCursor::new(caret.char_offset());
                    let local = galley.pos_from_cursor(cursor);
                    egui::pos2(origin.x + local.min.x, origin.y + local.max.y)
                });
                Self::render_slash_surface(ui, state, palette, caret_pixel);
            }

            // WP-KERNEL-012 MT-058: the inline-tag `#` autocomplete menu, anchored at the caret pixel
            // (same caret-galley resolution as the wikilink/slash popups). Rendered AFTER the content so
            // it sits above the blocks. Reuses the same in-process egui popup pattern (HBR-QUIET — no OS
            // window, no focus theft). A clicked row commits the tag as a chip.
            if state.tag_autocomplete.is_some() {
                let caret_pixel = caret_galley_out.as_ref().map(|(galley, origin)| {
                    let cursor = egui::epaint::text::cursor::CCursor::new(caret.char_offset());
                    let local = galley.pos_from_cursor(cursor);
                    egui::pos2(origin.x + local.min.x, origin.y + local.max.y)
                });
                Self::render_tag_menu_popup(ui, state, palette, caret_pixel);
            }

            // MT-034: the `/code-ref` code-symbol search dialog (a floating Window, not caret-anchored).
            if state.code_symbol_search.is_some() {
                Self::drive_code_symbol_search(ui, state, palette);
            }
        }

        // RISK-3 idle-CPU control: schedule the next blink frame ONLY when focused. An
        // unfocused editor returns false here and never requests a repaint. In reading mode
        // `has_focus` is always false, so the reading view never animates (HBR-QUIET).
        let _scheduled = request_blink_repaint(ui.ctx(), has_focus);
    }

    /// WP-KERNEL-012 MT-034: drive the `/code-ref` code-symbol search dialog one frame. Drains the
    /// off-thread lookup cell, renders the dialog (input + results + AccessKit nodes), and acts on the
    /// outcome: a Selected result inserts a `code` hsLink atom at the caret via
    /// [`insert_code_ref_atom`](crate::rich_editor::slash_commands::executor::insert_code_ref_atom) and
    /// closes the dialog; a Cancel closes it without inserting. The insert is the SAME inline-atom path
    /// the embed/wikilink confirms use, so the code-ref round-trips `content_json` (AC-1).
    fn drive_code_symbol_search(ui: &mut egui::Ui, state: &mut RichEditorState, palette: &HsPalette) {
        use crate::rich_editor::slash_commands::executor::{insert_code_ref_atom, SlashExecContext};
        use crate::rich_editor::slash_commands::{render_code_symbol_search_dialog, CodeSymbolSearchOutcome};

        let Some(dialog) = state.code_symbol_search.as_mut() else { return };
        // Drain the off-thread lookup result; request a repaint so a just-delivered result shows.
        if dialog.drain() {
            ui.ctx().request_repaint();
        }
        if dialog.loading {
            ui.ctx().request_repaint();
        }
        let outcome = render_code_symbol_search_dialog(ui.ctx(), dialog, palette);
        match outcome {
            CodeSymbolSearchOutcome::None => {}
            CodeSymbolSearchOutcome::Cancelled => {
                state.code_symbol_search = None;
            }
            CodeSymbolSearchOutcome::Selected { symbol_entity_id, display_name } => {
                let RichEditorState { doc, selection, undo, actor_id, .. } = state;
                let mut ctx = SlashExecContext {
                    doc,
                    history: undo,
                    selection,
                    actor_id: actor_id.as_str(),
                };
                insert_code_ref_atom(&mut ctx, &symbol_entity_id, &display_name);
                state.code_symbol_search = None;
            }
        }
    }

    /// Render the wikilink autocomplete popup at the caret pixel (or, when the caret pixel is
    /// unavailable, just below the content). Lists the backend result rows (selectable), the loading
    /// spinner, or the typed error / "No results" state, THEN the WP-KERNEL-012 MT-057 ALIAS candidate
    /// rows blended from the resolver index. Clicking a row confirms it (inserts the hsLink atom) and
    /// closes the popup — the same effect as Enter. AccessKit: the popup is `wikilink-autocomplete`,
    /// each backend row is `wikilink-result-{i}`, and each alias candidate row is
    /// `wikilink-candidate-{document_id}` (the contract ids).
    fn render_autocomplete_popup(
        ui: &mut egui::Ui,
        state: &mut RichEditorState,
        palette: &HsPalette,
        caret_pixel: Option<egui::Pos2>,
    ) {
        use crate::rich_editor::wikilinks::autocomplete::{candidates_for_query, SearchPhase};
        use crate::rich_editor::wikilinks::confirm;
        use crate::rich_editor::wikilinks::inline_view::candidate_author_id;

        let Some(ac) = state.wikilink_autocomplete.clone() else { return };
        // Anchor the popup at the caret pixel (impl note), defaulting to just below the editor when the
        // caret pixel is not resolvable (e.g. an empty doc).
        let anchor = caret_pixel.unwrap_or_else(|| ui.max_rect().left_bottom());

        // WP-KERNEL-012 MT-057 (WIRE 1): the ALIAS-aware candidate list for the live query, computed
        // from the resolver index BEFORE the render closure (it borrows `state.wikilinks` immutably; the
        // closure borrows `confirmed` mutably). These are MERGED into the dropdown alongside the backend
        // `search()` rows so an ALIAS-matched document actually appears in the `[[query` dropdown (the
        // pre-fix defect: `candidates_for_query` had zero production callers). Backend rows that already
        // cover a document_id are not duplicated by a candidate row (dedupe-by-id — the MT-015 dropdown
        // contract: one row per target).
        let alias_candidates = candidates_for_query(&state.wikilinks.resolver_index, &ac.query);
        let backend_ids: std::collections::HashSet<String> = match &ac.phase {
            SearchPhase::Ready(rows) => rows.iter().map(|r| r.block_id.clone()).collect(),
            _ => std::collections::HashSet::new(),
        };

        // Confirmation can come from EITHER a backend row OR an alias candidate row (both insert an
        // hsLink atom at the trigger span); a single slot carries whichever fired this frame.
        let mut confirmed: Option<(Vec<usize>, usize, crate::rich_editor::document_model::node::HsLinkNode)> = None;
        egui::Area::new(ui.id().with("wikilink-autocomplete-area"))
            .order(egui::Order::Foreground)
            .fixed_pos(anchor + egui::vec2(0.0, 4.0))
            .show(ui.ctx(), |ui| {
                let frame = egui::Frame::popup(ui.style());
                let resp = frame
                    .show(ui, |ui| {
                        ui.set_max_width(320.0);
                        // Whether the backend section rendered ANY selectable/status content; drives the
                        // "No results" line only when BOTH backend rows AND alias candidates are empty.
                        let mut backend_rows_shown = 0usize;
                        match &ac.phase {
                            SearchPhase::Idle | SearchPhase::Loading => {
                                ui.horizontal(|ui| {
                                    ui.add(egui::Spinner::new());
                                    ui.colored_label(palette.text_subtle, format!("Searching “{}”…", ac.query));
                                });
                            }
                            SearchPhase::Err(e) => {
                                ui.colored_label(palette.error_text, format!("Search failed ({}): {e}", e.kind_str()));
                            }
                            SearchPhase::Ready(rows) if rows.is_empty() => {
                                // Only show "No results" when there are ALSO no alias candidates below;
                                // otherwise the alias rows ARE the results.
                                if alias_candidates.is_empty() {
                                    ui.colored_label(palette.text_subtle, "No results");
                                }
                            }
                            SearchPhase::Ready(rows) => {
                                for (i, row) in rows.iter().enumerate() {
                                    let selected = i == ac.selected;
                                    let label = format!("{}  ({})", row.title, row.content_type);
                                    let item = ui.add(egui::Button::selectable(selected, label));
                                    // AccessKit: each backend result row is `wikilink-result-{i}`.
                                    let author = format!("wikilink-result-{i}");
                                    let row_id = item.id;
                                    let author_for_node = author.clone();
                                    ui.ctx().accesskit_node_builder(row_id, move |node| {
                                        node.set_author_id(author_for_node.clone());
                                    });
                                    if item.clicked() {
                                        let ref_kind = result_ref_kind(&row.content_type);
                                        confirmed = Some((
                                            ac.leaf_path.clone(),
                                            ac.trigger_start_char,
                                            crate::rich_editor::document_model::node::HsLinkNode::new(
                                                ref_kind,
                                                row.block_id.clone(),
                                                row.title.clone(),
                                            ),
                                        ));
                                    }
                                    backend_rows_shown += 1;
                                }
                            }
                        }
                        let _ = backend_rows_shown;

                        // WP-KERNEL-012 MT-057 (WIRE 1): the ALIAS candidate rows, blended below the
                        // backend rows. Each renders the canonical title as the primary label and, when
                        // the match came from an alias, the `— alias: "…"` secondary label (AC-005), and
                        // carries the stable `wikilink-candidate-{document_id}` AccessKit author_id so a
                        // swarm agent / kittest targets it by the document it resolves to. A candidate
                        // whose document a backend row already lists is skipped (dedupe-by-id).
                        for cand in &alias_candidates {
                            if backend_ids.contains(&cand.document_id) {
                                continue; // a backend row already covers this target.
                            }
                            let label = match &cand.matched_alias {
                                Some(alias) => format!("{}  — alias: \"{}\"", cand.display_title, alias),
                                None => cand.display_title.clone(),
                            };
                            let item = ui.add(egui::Button::selectable(false, label.clone()));
                            let author = candidate_author_id(&cand.document_id);
                            let row_id = item.id;
                            let label_for_node = label.clone();
                            let author_for_node = author.clone();
                            ui.ctx().accesskit_node_builder(row_id, move |node| {
                                node.set_author_id(author_for_node.clone());
                                node.set_label(label_for_node.clone());
                            });
                            if item.clicked() {
                                // An alias/title candidate inserts a resolved `note` link targeting the
                                // document id, labeled with the canonical title.
                                let mut link = crate::rich_editor::document_model::node::HsLinkNode::new(
                                    "note",
                                    cand.document_id.clone(),
                                    cand.display_title.clone(),
                                );
                                link.resolved = true;
                                confirmed = Some((ac.leaf_path.clone(), ac.trigger_start_char, link));
                            }
                        }
                    })
                    .response;
                // AccessKit: the popup container is `wikilink-autocomplete` (the AC-9 id).
                let popup_id = resp.id;
                ui.ctx().accesskit_node_builder(popup_id, |node| {
                    node.set_role(egui::accesskit::Role::ListBox);
                    node.set_author_id("wikilink-autocomplete".to_owned());
                });
            });

        if let Some((leaf_path, trigger_start, link)) = confirmed {
            let caret_char = Self::caret_char_offset(state);
            let RichEditorState { doc, selection, .. } = state;
            confirm::confirm_wikilink(doc, selection, &leaf_path, trigger_start, caret_char, link);
            state.wikilink_autocomplete = None;
        }
    }

    /// WP-KERNEL-012 MT-058: render the inline-tag `#` autocomplete menu at the caret pixel (or just
    /// below the content when the caret pixel is unavailable). Lists the filtered existing-tag rows from
    /// the cached MT-023 tag list PLUS the "create new tag" row for a free-typed query (AC-006), the
    /// selected row highlighted. Clicking a row commits it as a `Child::HsLink(ref_kind="tag")` atom via
    /// [`Self::commit_selected_tag`] (the same effect as Enter) and closes the menu. In-process egui
    /// `Area` popup (HBR-QUIET — no OS window, no focus theft). AccessKit: the menu container is
    /// `inline-tag-menu` (Role::ListBox) and each row is `inline-tag-menu-row-{i}`.
    fn render_tag_menu_popup(
        ui: &mut egui::Ui,
        state: &mut RichEditorState,
        palette: &HsPalette,
        caret_pixel: Option<egui::Pos2>,
    ) {
        use crate::rich_editor::inline_tags::tag_menu_items;

        let Some(ac) = state.tag_autocomplete.clone() else { return };
        let items = tag_menu_items(&ac.query, &state.tag_list);
        let anchor = caret_pixel.unwrap_or_else(|| ui.max_rect().left_bottom());

        // The row index clicked this frame (committed after the Area closes so the state borrow is free).
        let mut clicked_index: Option<usize> = None;
        egui::Area::new(ui.id().with("inline-tag-menu-area"))
            .order(egui::Order::Foreground)
            .fixed_pos(anchor + egui::vec2(0.0, 4.0))
            .show(ui.ctx(), |ui| {
                let frame = egui::Frame::popup(ui.style());
                let resp = frame
                    .show(ui, |ui| {
                        ui.set_max_width(280.0);
                        if items.is_empty() {
                            // An empty query with no tags: a hint, not a blank popup.
                            ui.colored_label(palette.text_subtle, "Type a tag name…");
                        }
                        for (i, item) in items.iter().enumerate() {
                            let selected = i == ac.selected;
                            let label = item.label();
                            let row = ui.add(egui::Button::selectable(selected, label.clone()));
                            // AccessKit: each row is `inline-tag-menu-row-{i}` so a swarm agent / kittest
                            // targets it deterministically.
                            let author = format!("inline-tag-menu-row-{i}");
                            let row_id = row.id;
                            let label_for_node = label.clone();
                            ui.ctx().accesskit_node_builder(row_id, move |node| {
                                node.set_author_id(author.clone());
                                node.set_label(label_for_node.clone());
                            });
                            if row.clicked() {
                                clicked_index = Some(i);
                            }
                        }
                    })
                    .response;
                // AccessKit: the menu container is `inline-tag-menu` (Role::ListBox).
                let popup_id = resp.id;
                ui.ctx().accesskit_node_builder(popup_id, |node| {
                    node.set_role(egui::accesskit::Role::ListBox);
                    node.set_author_id("inline-tag-menu".to_owned());
                });
            });

        if let Some(i) = clicked_index {
            // Point the selection at the clicked row, then commit it (the same path Enter uses) so the
            // click and the keyboard commit share one insert path.
            if let Some(ac_mut) = state.tag_autocomplete.as_mut() {
                ac_mut.selected = i;
            }
            Self::commit_selected_tag(state);
        }
    }

    /// MT-016: render the open slash-command surface — either the popup menu LIST (no prompt active) or
    /// the active prompt MODAL (embed/transclusion/manual insert). Applies the widget's outcome to the
    /// editor state: a clicked menu row executes via [`Self::execute_slash_selection`]; a confirmed
    /// prompt inserts via the executor's `confirm_prompt`; a cancelled prompt/menu closes the surface.
    fn render_slash_surface(
        ui: &mut egui::Ui,
        state: &mut RichEditorState,
        palette: &HsPalette,
        caret_pixel: Option<egui::Pos2>,
    ) {
        use crate::rich_editor::slash_commands::executor::{confirm_prompt, SlashExecContext};
        use crate::rich_editor::slash_commands::menu::{
            render_slash_menu, render_slash_prompt, SlashMenuOutcome, SlashPromptOutcome,
        };

        let Some(menu) = state.slash_menu.clone() else { return };

        // A prompt modal is active -> render it (the list is hidden while a prompt is up).
        if let Some(prompt) = menu.prompt.clone() {
            let mut input = prompt.input.clone();
            let outcome = render_slash_prompt(ui.ctx(), prompt.title(), prompt.hint(), &mut input);
            // Persist the typed input back into the live menu state so it survives across frames.
            if let Some(m) = state.slash_menu.as_mut() {
                if let Some(p) = m.prompt.as_mut() {
                    p.input = input.clone();
                }
            }
            match outcome {
                SlashPromptOutcome::Confirm => {
                    let mut confirm_state = prompt.clone();
                    confirm_state.input = input;
                    let _inserted = {
                        let RichEditorState { doc, selection, undo, actor_id, .. } = state;
                        let mut ctx = SlashExecContext {
                            doc,
                            history: undo,
                            selection,
                            actor_id: actor_id.as_str(),
                        };
                        confirm_prompt(&mut ctx, &confirm_state)
                    };
                    // Whether or not the insert happened (blank input is a no-op), close the surface on
                    // confirm so a blank confirm dismisses cleanly.
                    state.slash_menu = None;
                    ui.ctx().request_repaint();
                }
                SlashPromptOutcome::Cancel => {
                    state.slash_menu = None;
                    ui.ctx().request_repaint();
                }
                SlashPromptOutcome::None => {}
            }
            return;
        }

        // No prompt -> render the menu list. A click executes the row (keyboard Enter is handled in the
        // input pass via handle_slash_menu_keys).
        let outcome = render_slash_menu(ui, &menu, palette, caret_pixel);
        match outcome {
            SlashMenuOutcome::Execute(idx) => {
                Self::execute_slash_selection(state, idx);
                ui.ctx().request_repaint();
            }
            SlashMenuOutcome::Cancel => {
                state.slash_menu = None;
            }
            SlashMenuOutcome::None => {}
        }
    }
}

/// Map an autocomplete result's backend content type to the wikilink `ref_kind` an inserted hsLink
/// atom carries. A `note`/`document` block becomes a `note` link (the knowledge-document ref kind);
/// any other content type maps to that type verbatim (so a forward-compatible backend type still
/// produces a sensible link). This is the single place the autocomplete-result -> wikilink-kind
/// mapping lives.
fn result_ref_kind(content_type: &str) -> String {
    match content_type {
        "document" | "note" => "note".to_owned(),
        other => other.to_owned(),
    }
}

/// One wikilink chip to paint: the local glyph-span rect (galley-local, top=0), the link node, and
/// the (bg, text) chip colors. Computed by [`wikilink_chip_specs`] so the doc borrow ends before the
/// chip is painted (and `pending_events` can be borrowed mutably on a click).
struct WikilinkChipSpec {
    /// The link the chip represents (carries the routing payload for the click event).
    link: crate::rich_editor::document_model::node::HsLinkNode,
    /// The galley-local rect of the chip's first glyph (top-left anchor).
    local_start: egui::Rect,
    /// The galley-local rect of the chip's last glyph (right edge).
    local_end: egui::Rect,
    /// The chip background color (theme token).
    bg: egui::Color32,
    /// The chip text color (theme token).
    fg: egui::Color32,
    /// The chip display label (the resolved/`?`-prefixed text).
    label: String,
    /// WP-KERNEL-012 MT-057: `Some(title)` when this wikilink is UNRESOLVED against the resolver index
    /// (the click offers a "Create note \"{title}\"" affordance). `None` when the link resolved (or is
    /// a code ref / known-kind chip not subject to create-from-unresolved). The title is the trimmed
    /// original-case title the create intent + the create affordance author_id are keyed on.
    create_title: Option<String>,
}

/// Compute the [`WikilinkChipSpec`]s for every hsLink atom in an inline-content block by re-laying it
/// out (the same `line_layout` path the painter used) and hit-testing the galley for each atom's char
/// span. Returns an owned vec so the caller's doc borrow can end before painting.
fn wikilink_chip_specs(
    block: &BlockNode,
    palette: &HsPalette,
    content_width: f32,
    bold_available: bool,
    painter: &egui::Painter,
    resolver_index: &crate::rich_editor::wikilinks::resolver::ResolverIndex,
) -> Vec<WikilinkChipSpec> {
    use crate::rich_editor::wikilinks::inline_view::{chip_colors, chip_label, is_code_ref, is_locus_ref};
    use crate::rich_editor::wikilinks::resolver::{resolve_wikilink, WikilinkResolution};
    use egui::epaint::text::cursor::CCursor;

    // Only inline-content blocks (paragraph/heading) carry inline hsLink atoms.
    let layout = super::line_layout::layout_block(block, palette, content_width.max(1.0), bold_available);
    let galley = painter.layout_job(layout.job);

    let mut specs = Vec::new();
    let mut char_cursor = 0usize; // running char offset into the laid-out plain text.
    for child in &block.children {
        match child {
            Child::Text(t) => {
                char_cursor += t.text.len_chars();
            }
            Child::HsLink(link) => {
                let label = chip_label(link);
                let label_chars = label.chars().count();
                let start = char_cursor;
                let end = char_cursor + label_chars;
                let local_start = galley.pos_from_cursor(CCursor::new(start));
                let local_end = galley.pos_from_cursor(CCursor::new(end.max(start)));
                let (bg, fg) = chip_colors(link, palette);
                // WP-KERNEL-012 MT-057: a create-from-unresolved affordance is offered ONLY for a link
                // the editor considers UNRESOLVED (`link.resolved == false`) — a `[[Title]]` that did
                // not bind to a known kind. A link the parser already marked resolved (a known kind such
                // as `wp:`/`note:`, or a code ref) routes via WikilinkActivated and is NOT a create
                // candidate (RISK: turning every resolved chip into a create button). For an unresolved
                // link, the resolver index is consulted: if it now resolves (e.g. a note created earlier
                // this session, or an alias), the link is treated as resolved (no create offer); only a
                // still-Unresolved result offers the create affordance keyed on the title.
                // A locus ref (MT-068) references a governed WP/MT work unit, NOT a note, so — like a code
                // ref — it is NEVER a create-from-unresolved-note candidate even when greyed (the greyed
                // state means the record/endpoint is unavailable, not "create a note named WP-KERNEL-012").
                let create_title = if link.resolved || is_code_ref(link) || is_locus_ref(link) {
                    None
                } else {
                    match resolve_wikilink(resolver_index, &link.ref_value) {
                        WikilinkResolution::Unresolved { title } if !title.is_empty() => Some(title),
                        _ => None, // the index now resolves it -> not a create candidate
                    }
                };
                specs.push(WikilinkChipSpec {
                    link: link.clone(),
                    local_start,
                    local_end,
                    bg,
                    fg,
                    label,
                    create_title,
                });
                char_cursor = end;
            }
            // A loomTransclusion that is NOT a standalone block contributes its inline reference label
            // (line_layout renders `⟢ {ref}`); it advances the cursor but is not a clickable chip here
            // (the standalone transclusion block is the interactive surface).
            Child::Transclusion(t) => {
                char_cursor += format!("⟢ {}", t.ref_value).chars().count();
            }
            Child::Block(_) => {}
        }
    }
    specs
}

impl egui::Widget for RichEditorWidget {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        self.show(ui)
    }
}

/// A WP-011 `PaneFactory` that mounts the rich-text editor through the EXISTING
/// `pane_registry` / `PaneHostWidget` (no shell fork). The editor is the Notes pillar, so
/// it mounts under the `LoomWikiPage` surface (the Obsidian-class wiki/notes pane). The
/// container's AccessKit role is `TextInput` (matching the root editor node).
pub struct RichEditorPaneFactory {
    state: Arc<Mutex<RichEditorState>>,
    /// MT-031: set once after the rich-text surface registers its melt-together command set into the
    /// shared bus, so re-registration is idempotent across frames (the registry borrows
    /// `&dyn PaneFactory` at render time, so `render` has no `&mut self`).
    bus_registered: std::sync::atomic::AtomicBool,
}

impl RichEditorPaneFactory {
    /// Build the factory over shared editor state.
    pub fn new(state: Arc<Mutex<RichEditorState>>) -> Self {
        Self { state, bus_registered: std::sync::atomic::AtomicBool::new(false) }
    }

    /// The Arc-shared editor state this factory renders (so a test/host can drive the SAME state the
    /// mounted pane shows — MT-031 cross-pane proof needs the real editor behind the factory).
    pub fn state(&self) -> Arc<Mutex<RichEditorState>> {
        Arc::clone(&self.state)
    }
}

impl PaneFactory for RichEditorPaneFactory {
    fn pane_type(&self) -> PaneType {
        PaneType::LoomWikiPage
    }

    fn render(&self, ui: &mut egui::Ui, ctx: &PaneRenderContext) {
        use std::sync::atomic::Ordering;
        // MT-031 (E5 melt-together) LIVE WIRING: a MOUNTED rich-text pane retrieves the ONE shared bus
        // and publishes its selection + registers its command set every frame — the real per-frame bus
        // consumer the contract requires (not test-only dead code). The bus is in egui app data keyed by
        // INTERACTION_BUS_KEY, so every mounted pane sees the same instance.
        let bus = crate::interop::interaction_bus::InteractionBus::get_or_init(ui.ctx());
        let pane_id: crate::pane_registry::PaneId = Arc::from(ctx.record.pane_id.as_ref());
        let has_focus = ui.memory(|m| m.focused().map(|f| f == ctx.egui_id).unwrap_or(false));
        // MT-035 (E5 — unified undo): install the live pane id on the editor state so its undo entries are
        // recorded + routed under THIS pane's ring on the shared bus (POLICY-1). Idempotent.
        let selected = {
            let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
            if s.undo_pane_id.as_deref() != Some(pane_id.as_ref()) {
                s.undo_pane_id = Some(pane_id.clone());
            }
            s.selected_text()
        };
        let mut registered = self.bus_registered.load(Ordering::Relaxed);
        crate::rich_editor::interop_adapter::drive_bus_in_render(
            &bus,
            pane_id,
            has_focus,
            selected,
            &mut registered,
        );
        self.bus_registered.store(registered, Ordering::Relaxed);

        RichEditorWidget::new(Arc::clone(&self.state)).show(ui);
    }

    fn accesskit_role(&self) -> accesskit::Role {
        accesskit::Role::TextInput
    }
}

/// Re-export the shell live-a11y check so a caller (or test) can assert the editor's
/// emitted interactive nodes all carry an author_id through the SAME gate the shell uses.
pub use accessibility::assert_no_unnamed_interactive;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_new_places_caret_at_doc_start() {
        let st = RichEditorState::new(BlockNode::doc(vec![BlockNode::paragraph("hi")]));
        assert!(matches!(
            st.selection,
            Selection::Text { ref head, .. } if head.path == vec![0, 0] && head.char_offset == 0
        ));
    }

    #[test]
    fn demo_doc_has_heading_and_bold_world() {
        let st = RichEditorState::demo();
        assert_eq!(st.doc.children.len(), 2);
        assert!(st.doc.children[0].as_block().unwrap().heading_level() == Some(1));
        // The paragraph's second run is bold "world".
        let para = st.doc.children[1].as_block().unwrap();
        let bold_run = para.children[1].as_text().unwrap();
        assert_eq!(bold_run.text.to_string(), "world");
        assert!(bold_run.has_mark_type(&crate::rich_editor::document_model::node::Mark::Bold));
        assert_eq!(st.block_plain_text(1).as_deref(), Some("Hello world"));
    }

    #[test]
    fn factory_pane_type_is_notes_surface() {
        let st = Arc::new(Mutex::new(RichEditorState::demo()));
        let f = RichEditorPaneFactory::new(st);
        assert_eq!(f.pane_type(), PaneType::LoomWikiPage);
        assert_eq!(f.accesskit_role(), accesskit::Role::TextInput);
    }
}

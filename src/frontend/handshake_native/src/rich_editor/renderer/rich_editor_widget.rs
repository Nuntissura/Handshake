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
            properties: None,
            properties_runtime,
            find_replace: None,
        }
    }

    /// Install the live wikilink context: the workspace + document the transclusion/backlinks/
    /// autocomplete resolve against, and the tokio runtime handle resolutions spawn onto. The shell
    /// calls this when it knows the active document (the production wiring point). Setting the
    /// document triggers a backlinks generation bump (MC-004) so a prior document's in-flight
    /// backlinks response is dropped.
    pub fn set_wikilink_context(
        &mut self,
        workspace_id: impl Into<String>,
        document_id: impl Into<String>,
        runtime: tokio::runtime::Handle,
    ) {
        let workspace_id = workspace_id.into();
        self.wikilinks.workspace_id = workspace_id.clone();
        self.wikilinks.autocomplete.workspace_id = workspace_id;
        self.wikilinks.runtime = Some(runtime.clone());
        self.wikilinks.autocomplete.runtime = Some(runtime);
        self.wikilinks.set_document(document_id);
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
}

/// The per-frame editor view. Construct it with the shared state and call [`Self::show`]
/// inside an egui `Ui` (or use it as an `egui::Widget` via [`Self::ui`]).
pub struct RichEditorWidget {
    state: Arc<Mutex<RichEditorState>>,
}

impl RichEditorWidget {
    /// Build a widget over shared editor state.
    pub fn new(state: Arc<Mutex<RichEditorState>>) -> Self {
        Self { state }
    }

    /// Render the editor into `ui`, returning the interaction [`egui::Response`] for the
    /// editor surface (so a caller can check focus / hover). This is the core entry the
    /// `egui::Widget` impl and the pane factory both call.
    pub fn show(self, ui: &mut egui::Ui) -> egui::Response {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());

        // OUTER container scope: a stable Ui id keeps the root AccessKit node id fixed
        // across frames (same pattern the code editor uses). We render the scroll area +
        // blocks inside it and emit the root node onto this scope's Ui id so the per-block
        // nodes are its descendants.
        let response = ui
            .scope_builder(
                egui::UiBuilder::new()
                    .id_salt(root_egui_id())
                    .sense(egui::Sense::click_and_drag()),
                |ui| {
                    let palette = state.palette();
                    // Paint the editor background from the theme (no hardcoded hex).
                    let full_rect = ui.available_rect_before_wrap();
                    if ui.is_rect_visible(full_rect) {
                        ui.painter().rect_filled(full_rect, 0.0, palette.bg);
                    }

                    // The surface response: clickable+focusable so the editor can hold
                    // keyboard focus and receive Text/Key/Ime events. Interacting on the
                    // full rect gives us a stable focusable widget. It is an interactive
                    // node, so it MUST carry a stable author_id (the shell HBR-SWARM gate
                    // panics on an unnamed interactive node) — we give it the editor-surface
                    // id so a swarm agent can focus the editor by a stable key.
                    let surface_id = ui.id().with("rich-editor-surface");
                    let surface = ui.interact(full_rect, surface_id, egui::Sense::click_and_drag());
                    let has_focus = surface.has_focus();
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

                    // 1) Apply input + IME for this frame (only meaningful when focused, but
                    //    we still drain events so a programmatically-focused test works).
                    Self::apply_frame_input(ui, &mut state, has_focus);

                    // 2) MT-013: the formatting toolbar above the content area, then the
                    //    blocks below it, stacked vertically (contract step 6:
                    //    `ui.vertical(|ui| { toolbar.ui(ui); content_area(ui); })`). The
                    //    toolbar borrows the SAME editor state so a button click dispatches a
                    //    command directly on it.
                    let palette = state.palette(); // re-resolve (theme unchanged, cheap)
                    ui.vertical(|ui| {
                        // MT-017: the document properties panel ABOVE the content (default collapsed).
                        Self::render_properties(ui, &mut state, &palette);
                        Self::render_toolbar(ui, &mut state);
                        ui.separator();
                        Self::render_blocks(ui, &mut state, &palette, has_focus);
                    });

                    // MT-018: render the floating find/replace panel (a top-level egui::Window) when
                    // open, and apply its outcome (Replace One / Replace All / Close) against the doc
                    // + undo manager. Rendered after the content so it floats above the blocks; the
                    // window does not steal editor keyboard focus (HBR-QUIET).
                    Self::render_find_panel(ui.ctx(), &mut state, &palette);

                    // 3) Emit the root AccessKit node (AC-10: author_id rich-editor-root,
                    //    Role::TextInput) onto THIS scope's Ui id so the block nodes nest
                    //    under it. REUSES the same accesskit_node_builder hook as the shell.
                    let root_node_id = ui.unique_id();
                    let value = format!("{} blocks", state.doc.children.len());
                    ui.ctx().accesskit_node_builder(root_node_id, move |node| {
                        node.set_role(ROOT_ROLE);
                        node.set_author_id(RICH_EDITOR_ROOT_AUTHOR_ID.to_owned());
                        node.set_label("Rich text editor".to_owned());
                        node.set_value(value.clone());
                    });

                    surface
                },
            )
            .inner;

        response
    }

    /// Drain this frame's egui input events and apply them to the editor state. IME events
    /// route to [`ime_handler`]; key/text events route to [`input_handler`]. We snapshot
    /// the events from `ui.input` and apply them while holding the state lock.
    fn apply_frame_input(ui: &egui::Ui, state: &mut RichEditorState, has_focus: bool) {
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

        if !has_focus {
            return; // an unfocused editor ignores the remaining input (and never schedules a repaint).
        }

        // MT-018: a Ctrl+F / Ctrl+H shortcut opens (or re-focuses) the find/replace panel. Handled
        // BEFORE the editing decode so the chord opens the panel instead of being swallowed; the
        // events are NOT removed (Ctrl+F/Ctrl+H produce no Text event and no EditAction, so they do
        // not double-fire as typing). Opening triggers an initial scan against the live doc.
        if let Some(shortcut) = input_handler::decode_find_replace_shortcut(&events) {
            Self::apply_find_replace_shortcut(state, shortcut);
            return; // the chord is consumed by the panel; do not also run the editing decode this frame.
        }

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

        // MT-018: the find/replace panel recomputes its scan on every document change while open
        // (the React "recompute on every document change"). Any typing/delete/undo this frame may
        // have mutated the doc, so the scan + the active-index clamp are refreshed here. This is a
        // synchronous in-memory walk (no spinner, no async); the panel renders only when open.
        let RichEditorState { doc, find_replace, .. } = state;
        if let Some(panel) = find_replace.as_mut() {
            panel.rescan(doc);
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

    /// The caret's in-leaf char offset (the head's `char_offset`), or 0 for a non-text selection.
    fn caret_char_offset(state: &RichEditorState) -> usize {
        match &state.selection {
            Selection::Text { head, .. } => head.char_offset,
            Selection::Node { .. } => 0,
        }
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
        _palette: &HsPalette,
    ) -> Option<crate::rich_editor::wikilinks::inline_view::EditorEvent> {
        use crate::rich_editor::wikilinks::inline_view::{chip_author_id, chip_rect_for_span, EditorEvent, CHIP_ROLE};

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
        let author = chip_author_id(&spec.link.ref_value);
        let chip_id = ui.id().with(("wikilink-chip", &author));
        let resp = ui.interact(rect, chip_id, egui::Sense::click());
        let role = CHIP_ROLE;
        let author_for_node = author.clone();
        let label_for_node = spec.label.clone();
        ui.ctx().accesskit_node_builder(chip_id, move |node| {
            node.set_role(role);
            node.set_author_id(author_for_node.clone());
            node.set_label(label_for_node.clone());
        });
        if resp.clicked() {
            Some(EditorEvent::WikilinkActivated {
                ref_kind: spec.link.ref_kind.clone(),
                ref_value: spec.link.ref_value.clone(),
                resolved: spec.link.resolved,
            })
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

    /// Render the MT-013 formatting toolbar (a horizontal glyph-button row grouped by
    /// category) above the content area. The toolbar borrows the editor state by `&mut`
    /// (doc/undo/selection) so a button click dispatches a command STANDALONE on the local
    /// state (COMMAND DISPATCH REALITY gate — the host bus Sender is E11/MT-069). Returns
    /// nothing; a dispatched command mutates `state` in place and the next paint reflects it.
    fn render_toolbar(ui: &mut egui::Ui, state: &mut RichEditorState) {
        let RichEditorState { doc, selection, undo, actor_id, .. } = state;
        let cctx = crate::rich_editor::formatting::commands::CommandContext::new(
            doc,
            undo,
            selection,
            actor_id.as_str(),
        );
        let _dispatched = crate::rich_editor::formatting::toolbar::EditorToolbar::new(cctx).show(ui);
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
    ) {
        let caret = DocCaret::from_selection(&state.selection);
        let caret_block = caret.block_index();
        let time = ui.input(|i| i.time);
        let blink_on = blink_visible(time);
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

        // MT-015: reserve a bottom strip for the backlinks panel so the full-height content scroll
        // area does not push it off-screen (HBR-VIS: the panel must be visible, not just present in
        // the tree). Cap the scroll area's height to leave room for the panel below; the panel
        // collapses to its header height when closed, so the reserve is a soft upper bound.
        const BACKLINKS_STRIP_PTS: f32 = 150.0;
        let available_h = ui.available_height().max(1.0);
        let scroll_max_h = (available_h - BACKLINKS_STRIP_PTS).max(available_h * 0.4);

        let caret_galley_out = egui::ScrollArea::vertical()
            .id_salt("rich-editor-scroll")
            .auto_shrink([false, false])
            .max_height(scroll_max_h)
            .show(ui, |ui| {
                let content_width = ui.available_width().max(1.0);
                let mut top = ui.cursor().min;
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
                        top.y += used + super::line_layout::BLOCK_GAP_PTS;
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
                        top.y += used + super::line_layout::BLOCK_GAP_PTS;
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

                    // MT-015: overlay the inline wikilink CHIPs on this paragraph's hsLink atoms. We
                    // re-layout the block (cheap) to get the galley, find each hsLink's char span in
                    // the laid-out text, and paint a colored rounded chip at the glyph span — the chip
                    // Y is the (already scroll-adjusted) paint origin `top` (RISK-1 / MC-001). Each chip
                    // is an interactive AccessKit node (`wikilink-chip-{hash}`, Role::Link); a click
                    // enqueues a WikilinkActivated event for the shell. The chip specs are computed
                    // into an owned vec FIRST (ending the doc borrow) so `pending_events` can be
                    // borrowed mutably when handling a click.
                    let chip_specs = wikilink_chip_specs(block, palette, content_width, bold_available, &painter);
                    for spec in chip_specs {
                        if let Some(ev) = Self::paint_one_wikilink_chip(ui, &spec, top, palette) {
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

                    top.y += bp.height;
                }

                // Reserve the painted vertical extent so the scroll area sizes correctly.
                let used_height = (top.y - ui.cursor().min.y).max(0.0);
                ui.allocate_space(egui::vec2(content_width, used_height));

                // Paint the blinking caret over the blocks (only when collapsed + blink-on).
                if let Some((galley, origin)) = caret_galley.clone() {
                    paint_caret(&painter, &galley, origin, &caret, palette, blink_on);
                }
                // Return the caret galley+origin so the popup (rendered OUTSIDE this scroll closure)
                // can anchor at the caret pixel.
                caret_galley
            })
            .inner;

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

        // RISK-3 idle-CPU control: schedule the next blink frame ONLY when focused. An
        // unfocused editor returns false here and never requests a repaint.
        let _scheduled = request_blink_repaint(ui.ctx(), has_focus);
    }

    /// Render the wikilink autocomplete popup at the caret pixel (or, when the caret pixel is
    /// unavailable, just below the content). Lists the result rows (selectable), the loading spinner,
    /// or the typed error / "No results" state. Clicking a row confirms it (inserts the hsLink atom)
    /// and closes the popup — the same effect as Enter. AccessKit: the popup is `wikilink-autocomplete`
    /// and each row is `wikilink-result-{i}` (the contract ids).
    fn render_autocomplete_popup(
        ui: &mut egui::Ui,
        state: &mut RichEditorState,
        palette: &HsPalette,
        caret_pixel: Option<egui::Pos2>,
    ) {
        use crate::rich_editor::wikilinks::autocomplete::SearchPhase;
        use crate::rich_editor::wikilinks::confirm;

        let Some(ac) = state.wikilink_autocomplete.clone() else { return };
        // Anchor the popup at the caret pixel (impl note), defaulting to just below the editor when the
        // caret pixel is not resolvable (e.g. an empty doc).
        let anchor = caret_pixel.unwrap_or_else(|| ui.max_rect().left_bottom());

        let mut confirmed: Option<(Vec<usize>, usize, crate::rich_editor::document_model::node::HsLinkNode)> = None;
        egui::Area::new(ui.id().with("wikilink-autocomplete-area"))
            .order(egui::Order::Foreground)
            .fixed_pos(anchor + egui::vec2(0.0, 4.0))
            .show(ui.ctx(), |ui| {
                let frame = egui::Frame::popup(ui.style());
                let resp = frame
                    .show(ui, |ui| {
                        ui.set_max_width(320.0);
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
                                ui.colored_label(palette.text_subtle, "No results");
                            }
                            SearchPhase::Ready(rows) => {
                                for (i, row) in rows.iter().enumerate() {
                                    let selected = i == ac.selected;
                                    let label = format!("{}  ({})", row.title, row.content_type);
                                    let item = ui.add(egui::Button::selectable(selected, label));
                                    // AccessKit: each result row is `wikilink-result-{i}`.
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
                                }
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
) -> Vec<WikilinkChipSpec> {
    use crate::rich_editor::wikilinks::inline_view::{chip_colors, chip_label};
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
                specs.push(WikilinkChipSpec {
                    link: link.clone(),
                    local_start,
                    local_end,
                    bg,
                    fg,
                    label,
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
}

impl RichEditorPaneFactory {
    /// Build the factory over shared editor state.
    pub fn new(state: Arc<Mutex<RichEditorState>>) -> Self {
        Self { state }
    }
}

impl PaneFactory for RichEditorPaneFactory {
    fn pane_type(&self) -> PaneType {
        PaneType::LoomWikiPage
    }

    fn render(&self, ui: &mut egui::Ui, _ctx: &PaneRenderContext) {
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

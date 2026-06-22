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
        Self {
            doc,
            selection: Selection::caret(DocPosition::new(vec![0, 0], 0)),
            undo: UndoManager::new(),
            preedit: PreeditState::default(),
            theme: HsTheme::Dark,
            actor_id: "operator".to_owned(),
            embeds,
        }
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
                        Self::render_toolbar(ui, &mut state);
                        ui.separator();
                        Self::render_blocks(ui, &mut state, &palette, has_focus);
                    });

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
        if !has_focus {
            return; // an unfocused editor ignores input (and never schedules a repaint).
        }
        let events: Vec<egui::Event> = ui.input(|i| i.events.clone());

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

        egui::ScrollArea::vertical()
            .id_salt("rich-editor-scroll")
            .auto_shrink([false, false])
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

                    let block = state.doc.children[idx].as_block().expect("checked above");
                    let caret_offset = if caret_block == Some(idx) {
                        Some(caret.char_offset())
                    } else {
                        None
                    };
                    let bp = paint_block(&painter, block, top, content_width, palette, caret_offset, bold_available);
                    if let Some(g) = bp.caret_galley {
                        caret_galley = Some(g);
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
                if let Some((galley, origin)) = caret_galley {
                    paint_caret(&painter, &galley, origin, &caret, palette, blink_on);
                }
            });

        // RISK-3 idle-CPU control: schedule the next blink frame ONLY when focused. An
        // unfocused editor returns false here and never requests a repaint.
        let _scheduled = request_blink_repaint(ui.ctx(), has_focus);
    }
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

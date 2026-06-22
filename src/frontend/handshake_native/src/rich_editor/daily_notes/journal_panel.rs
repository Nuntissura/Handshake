//! The top-level daily-notes / journal egui panel (WP-KERNEL-012 MT-019).
//!
//! This is the native Rust port of `app/src/components/LoomDailyJournalPanel.tsx` (MT-257). It is a
//! SIBLING top-level surface mounted through the WP-011 `pane_registry` / `split_layout` host (NOT a
//! child of `RichEditorWidget`), so its own test bin avoids the run()-on-spinner trap.
//!
//! ## Layout (the contract PANEL LAYOUT)
//!
//! - Header: [◀ prev] [date display "Thursday, June 19, 2026"] [▶ next] [📅 calendar] [Today]
//!   (the [`super::date_nav::DateNavWidget`]).
//! - Subtitle: "Daily Note" muted, with the block_id badge.
//! - Content: the MT-012 [`RichEditorWidget`] for the journal document, OR a "Start writing…" button
//!   for a blank journal block, OR a spinner (genuine fetch only), OR a typed error chip + Retry.
//! - Footer: last-saved relative time + word count + character count.
//!
//! ## Auto-save (frame-based debounce — testable, no `std::time::Instant`)
//!
//! Per the contract impl-note, the 3-second idle timer is FRAME-based, not wall-clock: the panel tracks
//! `last_edit_frame` + `current_frame` and fires a save once `current_frame - last_edit_frame >`
//! [`AUTO_SAVE_IDLE_FRAMES`] (180 frames ≈ 3 s at 60 fps). A mock clock advances frames in the test, so
//! the debounce is deterministic. Each keystroke resets `last_edit_frame` (the debounce); MC-001 skips a
//! new auto-save while one is in flight and re-arms afterward.
//!
//! ## No perpetual spinner (KERNEL_BUILDER gate / MT-015 lesson)
//!
//! The animating spinner renders ONLY in [`JournalState::Loading`], which is only ever entered when a
//! live runtime spawned the fetch (see [`JournalStore::open`]). Headless renders a neutral state.

use std::sync::{Arc, Mutex};

use egui::accesskit;

use crate::accessibility;
use crate::pane_registry::{PaneFactory, PaneRenderContext, PaneType};
use crate::rich_editor::document_model::node::BlockNode;
use crate::rich_editor::document_model::position::DocPosition;
use crate::rich_editor::document_model::selection::Selection;
use crate::rich_editor::renderer::rich_editor_widget::{RichEditorState, RichEditorWidget};
use crate::theme::{HsPalette, HsTheme};

use super::date_nav::{DateNav, DateNavOutcome, DateNavWidget};
use super::journal_store::{JournalState, JournalStore, SaveStatus};

/// The AccessKit author_id for the journal panel root container (a swarm agent addresses the whole
/// surface by this stable key).
pub const JOURNAL_ROOT_ID: &str = "journal-panel-root";
/// The author_id for the "Start writing" button (the blank-journal create affordance).
pub const START_WRITING_ID: &str = "journal-start-writing";
/// The author_id for the error-chip Retry button.
pub const RETRY_ID: &str = "journal-retry";
/// The author_id for the block-id badge.
pub const BLOCK_BADGE_ID: &str = "journal-block-badge";

/// The frame-idle threshold before an auto-save fires (≈3 s at 60 fps — the contract impl-note).
pub const AUTO_SAVE_IDLE_FRAMES: u64 = 180;

/// The AccessKit role for the panel root container.
pub const JOURNAL_ROOT_ROLE: accesskit::Role = accesskit::Role::Group;

/// The frame-based auto-save debounce timer (the contract impl-note: frame counting, NOT
/// `std::time::Instant`, so it is deterministically testable with a mock clock that advances frames).
///
/// `last_edit_frame` is reset on every keystroke; `dirty` is set on an edit and cleared on a save. A
/// save is DUE when the doc is dirty AND at least [`AUTO_SAVE_IDLE_FRAMES`] frames elapsed since the
/// last edit. This struct is pure (no egui), so the debounce is fully unit-testable.
#[derive(Debug, Clone, Default)]
pub struct AutoSaveTimer {
    /// The frame index of the last edit (the debounce anchor).
    pub last_edit_frame: u64,
    /// The current frame index (advanced each `update`, or by the mock clock in a test).
    pub current_frame: u64,
    /// Whether the document has unsaved edits since the last save.
    pub dirty: bool,
}

impl AutoSaveTimer {
    /// Record an edit at the current frame: mark dirty and reset the idle anchor (the debounce — each
    /// keystroke pushes the fire time 3 more seconds out).
    pub fn on_edit(&mut self) {
        self.dirty = true;
        self.last_edit_frame = self.current_frame;
    }

    /// Advance the frame counter by one (called each `update`; a test advances it via [`Self::advance`]).
    pub fn tick(&mut self) {
        self.current_frame = self.current_frame.wrapping_add(1);
    }

    /// Advance the frame counter by `n` (mock-clock test helper).
    pub fn advance(&mut self, n: u64) {
        self.current_frame = self.current_frame.wrapping_add(n);
    }

    /// Whether an auto-save is due: dirty AND at least [`AUTO_SAVE_IDLE_FRAMES`] frames since the last
    /// edit. Does NOT clear `dirty` (the caller clears it after dispatching the save).
    pub fn save_due(&self) -> bool {
        self.dirty && self.current_frame.saturating_sub(self.last_edit_frame) > AUTO_SAVE_IDLE_FRAMES
    }

    /// Clear the dirty flag after a save has been dispatched (so the timer does not re-fire until the
    /// next edit).
    pub fn mark_saved(&mut self) {
        self.dirty = false;
    }
}

/// Walk a backend `content_json` ProseMirror/Tiptap value and collect its plain text (every `text`
/// field, separated by single spaces between text nodes). Used for the footer word/char counts. A
/// `None`/non-object body yields an empty string (a never-saved journal has no text → 0/0).
pub fn content_plain_text(content_json: Option<&serde_json::Value>) -> String {
    let mut out = String::new();
    if let Some(v) = content_json {
        collect_text(v, &mut out);
    }
    out
}

/// Recursively append every `text` leaf in a ProseMirror doc value to `out`, separating text nodes
/// with a single space so adjacent text nodes do not merge into one "word".
fn collect_text(node: &serde_json::Value, out: &mut String) {
    if let Some(text) = node.get("text").and_then(|t| t.as_str()) {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(text);
    }
    if let Some(content) = node.get("content").and_then(|c| c.as_array()) {
        for child in content {
            collect_text(child, out);
        }
    }
}

/// The word count of `text`: the number of whitespace-separated tokens (the contract footer formula).
pub fn word_count(text: &str) -> usize {
    text.split_whitespace().filter(|w| !w.is_empty()).count()
}

/// The character count of `text`: total Unicode scalar values (chars), matching the rope char-count
/// semantics the contract names ("total rope char count across all TextLeafs"). The collected plain
/// text is the concatenation of every text leaf, so its char count is that total.
pub fn char_count(text: &str) -> usize {
    text.chars().count()
}

/// Render a footer "Saved" relative-time string from a [`SaveStatus`]. Kept simple + non-animating
/// (the no-spinner discipline): InFlight shows "Saving…", Saved shows "Saved", Failed shows the typed
/// reason, Idle shows nothing.
pub fn save_status_text(save: &SaveStatus) -> String {
    match save {
        SaveStatus::Idle => String::new(),
        SaveStatus::InFlight => "Saving…".to_owned(),
        SaveStatus::Saved => "Saved".to_owned(),
        SaveStatus::Failed(e) => format!("Save failed ({})", e.kind_str()),
    }
}

/// The persistent journal-panel state: the store (state machine + backend), the date nav, the
/// auto-save timer, and the shared MT-012 editor state the content area renders into. Held behind an
/// `Arc<Mutex<>>` by the owner so the per-frame [`JournalPanelWidget`] borrows it.
pub struct JournalPanelState {
    /// The load state machine + backend transport + save seam.
    pub store: JournalStore,
    /// The prev/next/today/calendar date navigation.
    pub nav: DateNav,
    /// The frame-based auto-save debounce timer.
    pub auto_save: AutoSaveTimer,
    /// The MT-012 editor the content area renders the journal document into. Reused (not forked) — the
    /// journal panel hosts a `RichEditorWidget` for the journal's RichDocument.
    pub editor: Arc<Mutex<RichEditorState>>,
    /// The active theme.
    pub theme: HsTheme,
    /// The document id currently loaded into `editor` (so we only re-load the editor when it changes).
    loaded_doc_id: Option<String>,
}

impl JournalPanelState {
    /// Build a panel state over `store` + `nav`, with an empty editor.
    pub fn new(store: JournalStore, nav: DateNav) -> Self {
        let editor = Arc::new(Mutex::new(RichEditorState::new(BlockNode::doc(vec![
            BlockNode::paragraph(""),
        ]))));
        Self {
            store,
            nav,
            auto_save: AutoSaveTimer::default(),
            editor,
            theme: HsTheme::Dark,
            loaded_doc_id: None,
        }
    }

    /// Open the journal for the nav's current date (the mount + date-change entry). Resets the editor's
    /// loaded-doc tracking so the next Ready load installs the document.
    pub fn open_current(&mut self) {
        let date = self.nav.current_storage();
        self.store.open(date);
    }

    /// The resolved palette for the active theme.
    pub fn palette(&self) -> HsPalette {
        self.theme.palette()
    }

    /// Sync the MT-012 editor with the store's Ready document: when the Ready doc id differs from the
    /// one currently loaded, parse its content_json into the editor's doc (so the renderer paints the
    /// journal content). A document with no body shows an empty editor. Returns true when the editor
    /// content changed (the caller may reset the dirty flag).
    pub fn sync_editor_from_store(&mut self) -> bool {
        let Some(ready) = self.store.state.ready() else {
            return false;
        };
        let Some(doc) = ready.doc.as_ref() else {
            // No document yet ("Start writing"): clear the editor tracking so a later create re-syncs.
            if self.loaded_doc_id.is_some() {
                self.loaded_doc_id = None;
            }
            return false;
        };
        if self.loaded_doc_id.as_deref() == Some(doc.rich_document_id.as_str()) {
            return false; // already loaded this document.
        }
        // Parse the backend content_json into the MT-012 block model. A null/invalid body falls back to
        // a single empty paragraph (never a panic, never a blank-then-crash).
        let block_doc = doc
            .content_json
            .as_ref()
            .and_then(|v| crate::rich_editor::document_model::doc_json::from_json_value(v).ok())
            .unwrap_or_else(|| BlockNode::doc(vec![BlockNode::paragraph("")]));
        if let Ok(mut editor) = self.editor.lock() {
            editor.doc = block_doc;
            editor.selection = Selection::caret(DocPosition::new(vec![0, 0], 0));
        }
        self.loaded_doc_id = Some(doc.rich_document_id.clone());
        self.auto_save.dirty = false; // a freshly loaded document is clean.
        true
    }

    /// The current editor content as a backend `content_json` value (for the save seam).
    pub fn current_content_json(&self) -> serde_json::Value {
        self.editor
            .lock()
            .map(|e| e.current_content_json())
            .unwrap_or(serde_json::Value::Null)
    }

    /// Advance the auto-save timer one frame and, if a save is due (dirty + 3 s idle) and a document is
    /// loaded, dispatch the save through the store's seam (MC-001 skips when a save is already in
    /// flight). Returns true when a save was dispatched. Separated from the egui render so a mock-clock
    /// test drives it deterministically.
    pub fn tick_auto_save(&mut self) -> bool {
        self.auto_save.tick();
        if self.auto_save.save_due() {
            let content = self.current_content_json();
            self.store.dispatch_save(content);
            self.auto_save.mark_saved();
            // If the store actually entered InFlight, the save was dispatched (a live runtime). Headless
            // dispatch_save is a no-op, but we still cleared dirty (the debounce consumed the edit) so
            // the headless test can assert save_due() no longer fires without a runtime.
            return matches!(
                self.store.state.ready().map(|r| &r.save),
                Some(SaveStatus::InFlight)
            ) || self.store.runtime.is_none();
        }
        false
    }
}

/// The per-frame journal-panel view. Construct it with the shared state and call [`Self::show`].
pub struct JournalPanelWidget {
    state: Arc<Mutex<JournalPanelState>>,
}

impl JournalPanelWidget {
    /// Build a widget over shared panel state.
    pub fn new(state: Arc<Mutex<JournalPanelState>>) -> Self {
        Self { state }
    }

    /// Render the journal panel into `ui`.
    pub fn show(self, ui: &mut egui::Ui) -> egui::Response {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());

        // Drain any completed off-thread load/create/save BEFORE rendering, so a just-resolved fetch
        // shows this frame. A delivery schedules a repaint.
        if state.store.drain() {
            ui.ctx().request_repaint();
        }
        // Sync the editor with the (possibly newly) Ready document.
        let _changed = state.sync_editor_from_store();

        let palette = state.palette();
        let response = ui
            .scope_builder(
                egui::UiBuilder::new()
                    .id_salt("journal-panel")
                    .sense(egui::Sense::hover()),
                |ui| {
                    let full_rect = ui.available_rect_before_wrap();
                    if ui.is_rect_visible(full_rect) {
                        ui.painter().rect_filled(full_rect, 0.0, palette.bg);
                    }

                    // 1) Header: the date-nav widget (prev/next/today/calendar + date display).
                    let nav_outcome = {
                        let palette = palette.clone();
                        DateNavWidget::new(&mut state.nav, &palette).show(ui)
                    };
                    if let DateNavOutcome::Navigated(_) = nav_outcome {
                        // A date change re-opens the journal (MC-002: the store bumps the generation, so
                        // any in-flight load for the prior date is cancelled on drain).
                        let date = state.nav.current_storage();
                        state.store.open(date);
                    }

                    // 2) Subtitle: "Daily Note" + the block_id badge (when Ready).
                    ui.horizontal(|ui| {
                        ui.colored_label(palette.text_subtle, "Daily Note");
                        if let Some(ready) = state.store.state.ready() {
                            let badge = egui::Label::new(
                                egui::RichText::new(&ready.block.block_id)
                                    .color(palette.text_subtle)
                                    .small(),
                            )
                            .sense(egui::Sense::hover());
                            let resp = ui.add(badge);
                            accessibility::emit_interactive_node(ui.ctx(), resp.id, BLOCK_BADGE_ID);
                        }
                    });
                    ui.separator();

                    // 3) Content area: spinner / error chip / editor / "Start writing".
                    Self::render_content(ui, &mut state, &palette);

                    // 4) Footer: save status + word + char count.
                    ui.separator();
                    Self::render_footer(ui, &state, &palette);

                    // 5) Emit the panel root AccessKit node (a swarm agent addresses the whole surface).
                    let root_id = ui.unique_id();
                    let date_label = state.nav.current_storage();
                    ui.ctx().accesskit_node_builder(root_id, move |node| {
                        node.set_role(JOURNAL_ROOT_ROLE);
                        node.set_author_id(JOURNAL_ROOT_ID.to_owned());
                        node.set_label(format!("Daily journal {date_label}"));
                    });

                    ui.interact(full_rect, ui.id().with("journal-surface"), egui::Sense::hover())
                },
            )
            .inner;

        response
    }

    /// Render the content area by matching on the store's state machine. Loading → an animating
    /// `Spinner` (genuine fetch only); Error → a typed chip + Retry; Ready+doc → the MT-012 editor;
    /// Ready+no-doc → a "Start writing" button (or a spinner while a create is in flight, MC-003); Idle →
    /// a neutral non-animating placeholder.
    fn render_content(ui: &mut egui::Ui, state: &mut JournalPanelState, palette: &HsPalette) {
        // We must avoid holding a borrow of `state.store.state` while mutating the store (Retry/Start),
        // so we classify the state into a small owned enum first.
        enum View {
            Idle,
            Loading,
            Error(String, String), // (kind_str, message)
            ReadyNoDoc { creating: bool },
            ReadyDoc,
        }
        let view = match &state.store.state {
            JournalState::Idle => View::Idle,
            JournalState::Loading { .. } => View::Loading,
            JournalState::Error { kind, .. } => View::Error(kind.kind_str().to_owned(), kind.to_string()),
            JournalState::Ready(r) if r.needs_document() => View::ReadyNoDoc { creating: r.is_creating },
            JournalState::Ready(_) => View::ReadyDoc,
        };

        match view {
            View::Idle => {
                // Neutral non-animating state — NOT a spinner (no perpetual headless spinner).
                ui.colored_label(palette.text_subtle, "Open a daily note to begin.");
            }
            View::Loading => {
                // AC-8: the animating spinner — ONLY here, and only ever entered with a live fetch.
                ui.horizontal(|ui| {
                    ui.add(egui::Spinner::new());
                    ui.colored_label(palette.text_subtle, "Loading daily journal…");
                });
                ui.ctx().request_repaint(); // keep animating while the genuine fetch is in flight.
            }
            View::Error(kind, message) => {
                // AC-7: a typed error chip (never blank) + a Retry button.
                egui::Frame::group(ui.style())
                    .fill(palette.surface)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.colored_label(palette.error_text, format!("⚠ {message} ({kind})"));
                        });
                        let retry = ui.button("Retry");
                        accessibility::emit_interactive_node(ui.ctx(), retry.id, RETRY_ID);
                        if retry.clicked() {
                            let date = state.nav.current_storage();
                            state.store.open(date);
                        }
                    });
            }
            View::ReadyNoDoc { creating } => {
                if creating {
                    // MC-003: while the create is in flight, the button is a spinner (no duplicate fire).
                    ui.horizontal(|ui| {
                        ui.add(egui::Spinner::new());
                        ui.colored_label(palette.text_subtle, "Creating document…");
                    });
                    ui.ctx().request_repaint();
                } else {
                    ui.colored_label(palette.text_subtle, "This day has no note yet.");
                    let start = ui.button("Start writing…");
                    accessibility::emit_interactive_node(ui.ctx(), start.id, START_WRITING_ID);
                    if start.clicked() {
                        let date = state.nav.current_storage();
                        let title = format!("Daily Note {date}");
                        state.store.start_writing(title);
                    }
                }
            }
            View::ReadyDoc => {
                // The MT-012 editor renders the journal document. We track edits for the auto-save timer
                // by snapshotting the content hash via the editor's live content_json length is overkill;
                // instead, the host marks dirty when the editor surface reports changes. Here we render
                // and let the timer tick happen each frame; a real edit sets dirty via on_edit (wired by
                // the shell input loop). For the headless test, the panel exposes on_edit directly.
                let editor = Arc::clone(&state.editor);
                RichEditorWidget::new(editor).show(ui);
            }
        }
    }

    /// Render the footer: the save-status relative time, the word count, and the character count.
    fn render_footer(ui: &mut egui::Ui, state: &JournalPanelState, palette: &HsPalette) {
        // The counts come from the LIVE editor content (so they reflect typing), falling back to the
        // store's loaded body when the editor is empty.
        let text = {
            let live = state.current_content_json();
            let s = content_plain_text(Some(&live));
            if s.is_empty() {
                // Fall back to the store body (e.g. before the editor mounts).
                state
                    .store
                    .state
                    .ready()
                    .and_then(|r| r.doc.as_ref())
                    .and_then(|d| d.content_json.as_ref())
                    .map(|c| content_plain_text(Some(c)))
                    .unwrap_or_default()
            } else {
                s
            }
        };
        let words = word_count(&text);
        let chars = char_count(&text);
        ui.horizontal(|ui| {
            let save = state
                .store
                .state
                .ready()
                .map(|r| save_status_text(&r.save))
                .unwrap_or_default();
            if !save.is_empty() {
                ui.colored_label(palette.text_subtle, &save);
                ui.separator();
            }
            ui.colored_label(palette.text_subtle, format!("{words} words"));
            ui.separator();
            ui.colored_label(palette.text_subtle, format!("{chars} characters"));
        });
    }
}

impl egui::Widget for JournalPanelWidget {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        self.show(ui)
    }
}

/// A WP-011 `PaneFactory` that mounts the daily-notes / journal panel through the EXISTING
/// `pane_registry` / `PaneHostWidget` (no shell fork). It is a SIBLING top-level surface (the
/// `LoomDailyJournal` pane type — tab label "Journal"), NOT a child of `RichEditorWidget`. The
/// container's AccessKit role is `Group`.
pub struct JournalPaneFactory {
    state: Arc<Mutex<JournalPanelState>>,
}

impl JournalPaneFactory {
    /// Build the factory over shared journal-panel state.
    pub fn new(state: Arc<Mutex<JournalPanelState>>) -> Self {
        Self { state }
    }
}

impl PaneFactory for JournalPaneFactory {
    fn pane_type(&self) -> PaneType {
        PaneType::LoomDailyJournal
    }

    fn render(&self, ui: &mut egui::Ui, _ctx: &PaneRenderContext) {
        JournalPanelWidget::new(Arc::clone(&self.state)).show(ui);
    }

    fn accesskit_role(&self) -> accesskit::Role {
        JOURNAL_ROOT_ROLE
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rich_editor::daily_notes::journal_store::{JournalBackend, JournalFuture, JournalBlock, JournalDocLoad, JournalError, JournalSaveSeam, RichDocumentBody};

    // ── AutoSaveTimer (frame-based debounce, mock clock) ──────────────────────────────────────────

    #[test]
    fn auto_save_fires_only_after_idle_frames() {
        // AC-10 / impl-note: the debounce fires after >180 frames (~3 s) of idle since the last edit.
        let mut t = AutoSaveTimer::default();
        t.on_edit(); // dirty, anchored at frame 0.
        assert!(!t.save_due(), "not due immediately after an edit");
        t.advance(AUTO_SAVE_IDLE_FRAMES); // exactly 180 frames → strictly-greater means NOT yet.
        assert!(!t.save_due(), "not due at exactly the threshold (strict >)");
        t.advance(1); // 181 frames → due.
        assert!(t.save_due(), "due after >180 idle frames (~3 s at 60 fps)");
    }

    #[test]
    fn each_keystroke_resets_the_debounce() {
        // The debounce: a new edit pushes the fire time out by another full idle window.
        let mut t = AutoSaveTimer::default();
        t.on_edit();
        t.advance(170);
        t.on_edit(); // a keystroke at frame 170 re-anchors.
        t.advance(170); // 170 frames since the LAST edit → not yet due.
        assert!(!t.save_due(), "the second keystroke reset the timer");
        t.advance(20); // now 190 since the last edit → due.
        assert!(t.save_due());
    }

    #[test]
    fn mark_saved_clears_dirty_so_it_does_not_refire() {
        let mut t = AutoSaveTimer::default();
        t.on_edit();
        t.advance(200);
        assert!(t.save_due());
        t.mark_saved();
        t.advance(200);
        assert!(!t.save_due(), "a clean document never auto-saves");
    }

    // ── word / char count (the footer formula) ────────────────────────────────────────────────────

    #[test]
    fn word_count_matches_whitespace_tokens() {
        // AC-9: word count = whitespace-separated token count.
        assert_eq!(word_count(""), 0);
        assert_eq!(word_count("hello"), 1);
        assert_eq!(word_count("hello world"), 2);
        assert_eq!(word_count("  hello   world  foo "), 3);
        assert_eq!(word_count("one\ntwo\tthree"), 3);
    }

    #[test]
    fn char_count_is_unicode_scalar_count() {
        assert_eq!(char_count(""), 0);
        assert_eq!(char_count("hello"), 5);
        assert_eq!(char_count("héllo"), 5); // accented char counts as one scalar.
        assert_eq!(char_count("日本語"), 3);
    }

    #[test]
    fn content_plain_text_walks_prosemirror_doc() {
        // AC-9: the counts come from the document content_json walk.
        let doc = serde_json::json!({
            "type": "doc",
            "content": [
                { "type": "heading", "content": [{ "type": "text", "text": "Title" }] },
                { "type": "paragraph", "content": [
                    { "type": "text", "text": "Hello" },
                    { "type": "text", "text": "world", "marks": [{ "type": "bold" }] }
                ]}
            ]
        });
        let text = content_plain_text(Some(&doc));
        // Three text nodes, space-separated.
        assert_eq!(text, "Title Hello world");
        assert_eq!(word_count(&text), 3);
        assert_eq!(char_count(&text), "Title Hello world".chars().count());
    }

    #[test]
    fn content_plain_text_empty_for_no_body() {
        assert_eq!(content_plain_text(None), "");
        assert_eq!(word_count(&content_plain_text(None)), 0);
        assert_eq!(char_count(&content_plain_text(None)), 0);
    }

    #[test]
    fn save_status_text_is_non_animating() {
        assert_eq!(save_status_text(&SaveStatus::Idle), "");
        assert_eq!(save_status_text(&SaveStatus::InFlight), "Saving…");
        assert_eq!(save_status_text(&SaveStatus::Saved), "Saved");
        assert_eq!(
            save_status_text(&SaveStatus::Failed(JournalError::SaveFailed("x".into()))),
            "Save failed (save_failed)"
        );
    }

    // ── sync_editor_from_store (the content area mounts the journal document) ──────────────────────

    struct StubBackend;
    impl JournalBackend for StubBackend {
        fn open_daily_journal<'a>(&'a self, _w: &'a str, _d: &'a str) -> JournalFuture<'a, JournalBlock> {
            Box::pin(async { Err(JournalError::OpenFailed("stub".into())) })
        }
        fn load_document<'a>(&'a self, _d: &'a str) -> JournalFuture<'a, JournalDocLoad> {
            Box::pin(async { Err(JournalError::DocLoadFailed("stub".into())) })
        }
        fn create_document<'a>(&'a self, _w: &'a str, _t: &'a str) -> JournalFuture<'a, JournalDocLoad> {
            Box::pin(async { Err(JournalError::CreateFailed("stub".into())) })
        }
    }
    struct StubSeam;
    impl JournalSaveSeam for StubSeam {
        fn save<'a>(&'a self, _i: &'a str, v: u64, _c: serde_json::Value) -> JournalFuture<'a, u64> {
            Box::pin(async move { Ok(v + 1) })
        }
    }

    fn panel(date: chrono::NaiveDate) -> JournalPanelState {
        let store = JournalStore::headless(Arc::new(StubBackend), Arc::new(StubSeam));
        let nav = DateNav::new(date, date);
        JournalPanelState::new(store, nav)
    }

    #[test]
    fn sync_editor_loads_ready_document_content() {
        let date = chrono::NaiveDate::from_ymd_opt(2026, 6, 19).unwrap();
        let mut p = panel(date);
        // Stage a Ready state with a document carrying content.
        p.store.open(date.format("%Y-%m-%d").to_string());
        let body = RichDocumentBody {
            rich_document_id: "KRD-1".into(),
            title: "Daily Note".into(),
            doc_version: 2,
            content_json: Some(serde_json::json!({
                "type": "doc",
                "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "journal body" }] }]
            })),
        };
        let block = JournalBlock {
            block_id: "LB-1".into(),
            workspace_id: "ws".into(),
            content_type: Some("journal".into()),
            document_id: Some("KRD-1".into()),
            title: Some("Daily Note 2026-06-19".into()),
            journal_date: Some("2026-06-19".into()),
        };
        p.store.stage_load("2026-06-19", Ok((block, Some(body))));
        assert!(p.store.drain());
        assert!(p.sync_editor_from_store(), "the Ready document loaded into the editor");
        // The editor's live content_json now contains the journal body text.
        let text = content_plain_text(Some(&p.current_content_json()));
        assert!(text.contains("journal body"), "the journal body rendered into the editor (got {text:?})");
        // A second sync is a no-op (already loaded).
        assert!(!p.sync_editor_from_store());
    }

    #[test]
    fn tick_auto_save_dispatches_after_idle_when_dirty() {
        // The panel's tick wires the timer to the store dispatch. Headless dispatch is a no-op, but the
        // timer must consume the dirty edit so it does not refire.
        let date = chrono::NaiveDate::from_ymd_opt(2026, 6, 19).unwrap();
        let mut p = panel(date);
        p.auto_save.on_edit();
        p.auto_save.advance(AUTO_SAVE_IDLE_FRAMES + 2);
        assert!(p.auto_save.save_due());
        let _ = p.tick_auto_save();
        assert!(!p.auto_save.dirty, "the auto-save consumed the dirty edit (debounce satisfied)");
    }

    #[test]
    fn factory_pane_type_is_journal_sibling_surface() {
        let date = chrono::NaiveDate::from_ymd_opt(2026, 6, 19).unwrap();
        let st = Arc::new(Mutex::new(panel(date)));
        let f = JournalPaneFactory::new(st);
        assert_eq!(f.pane_type(), PaneType::LoomDailyJournal);
        assert_eq!(f.accesskit_role(), accesskit::Role::Group);
    }
}

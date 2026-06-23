//! The "Notes mentioning this symbol" side panel for the code pane (WP-KERNEL-012 MT-034, cluster E5).
//!
//! ## What this is (the code->notes direction surface)
//!
//! When the cursor dwells on a code symbol for [`crate::interop::NOTE_REFS_DWELL_MS`]ms, the code pane
//! fires [`crate::interop::find_notes_referencing_symbol`] and this panel lists the rich documents that
//! mention the symbol. Each row is clickable; clicking dispatches the EXISTING cross-pane
//! [`CMD_OPEN_DOCUMENT`](crate::interop::CMD_OPEN_DOCUMENT) command on the MT-031
//! [`crate::interop::InteractionBus`] (NOT a per-pane ad-hoc callback). This is the native-only
//! addition the React `CodeSymbolPanel` lacks — a genuine interconnection improvement (the React panel
//! shows only the symbol definition + file lens; this adds the reverse note refs).
//!
//! ## Async load discipline (no perpetual spinner — the MT-015 lesson)
//!
//! The panel renders from a [`NoteRefsState`] the code pane owns and drains. The `Idle` and `Empty`
//! states are NON-animating (a neutral label, never an `egui::Spinner` that would request a repaint
//! every frame forever in a headless harness); only `Loading` animates while a search is genuinely in
//! flight. A failure renders as a typed inline error chip (fail-closed, never blank, never a panic).
//!
//! ## AccessKit (HBR-SWARM / HBR-VIS)
//!
//! - panel container -> [`PANEL_AUTHOR_ID`] (`note-refs-panel`), `Role::List`,
//! - each note row     -> [`row_author_id`] (`note-ref-{doc_id}`), `Role::ListItem`, `[Press]` action,
//!
//! registered through the SAME `accesskit_node_builder` hook the WP-011 shell + the backlinks panel
//! use (no separate a11y layer). The field-correct accesskit 0.21.1 roles for the contract's
//! `List`/`ListItem` are `List`/`ListItem` (both present in 0.21.x — no fallback needed).

use egui::accesskit;

use crate::interop::{CrossRefError, NoteRef};
use crate::theme::HsPalette;

/// The AccessKit author_id for the note-refs panel container (the contract id, `Role::List`).
pub const PANEL_AUTHOR_ID: &str = "note-refs-panel";

/// The AccessKit role for the panel container (the contract's `Role::List`).
pub const PANEL_ROLE: accesskit::Role = accesskit::Role::List;

/// The AccessKit role for one note-ref row (the contract's `Role::ListItem`).
pub const ROW_ROLE: accesskit::Role = accesskit::Role::ListItem;

/// The AccessKit author_id PREFIX for one note row (`note-ref-{doc_id}`).
pub const ROW_AUTHOR_ID_PREFIX: &str = "note-ref-";

/// Build the stable AccessKit author_id for the row of the document `doc_id` (`note-ref-{doc_id}`).
pub fn row_author_id(doc_id: &str) -> String {
    format!("{ROW_AUTHOR_ID_PREFIX}{doc_id}")
}

/// The max chars a note title is truncated to in a row (the MT contract: "truncated to 40 chars").
pub const TITLE_TRUNCATE: usize = 40;

/// The max chars an excerpt is truncated to in a row (keeps a row single-line).
pub const EXCERPT_TRUNCATE: usize = 80;

/// The async load state of the note-refs panel for the currently-focused symbol. The code pane owns
/// this, sets it to [`Loading`](NoteRefsState::Loading) when it fires a search, and replaces it with
/// the delivered [`Loaded`](NoteRefsState::Loaded) / [`Failed`](NoteRefsState::Failed) result.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum NoteRefsState {
    /// No symbol focused yet / no search dispatched. A neutral, NON-animating placeholder.
    #[default]
    Idle,
    /// A `find_notes_referencing_symbol` search is in flight (the only animating state).
    Loading,
    /// The search resolved. The vec MAY be empty (an honest "no notes reference this symbol" state).
    Loaded(Vec<NoteRef>),
    /// The search failed (a typed error chip; fail-closed, never blank).
    Failed(CrossRefError),
}

impl NoteRefsState {
    /// The number of loaded note refs (0 unless [`Loaded`](NoteRefsState::Loaded)).
    pub fn count(&self) -> usize {
        match self {
            NoteRefsState::Loaded(notes) => notes.len(),
            _ => 0,
        }
    }
}

/// Truncate `s` to at most `max` chars, appending an ellipsis when it was cut (char-correct so a
/// multi-byte boundary is never split). An empty string passes through unchanged.
fn truncate(s: &str, max: usize) -> String {
    let mut chars = s.chars();
    let head: String = chars.by_ref().take(max).collect();
    if chars.next().is_some() {
        format!("{head}…")
    } else {
        head
    }
}

/// The row label for a note ref: the truncated title (or block id fallback) — the primary clickable
/// text. The excerpt renders as a separate subtle line beneath it.
pub fn row_title(note: &NoteRef) -> String {
    let base = if note.document_title.trim().is_empty() {
        note.document_id.as_str()
    } else {
        note.document_title.as_str()
    };
    truncate(base, TITLE_TRUNCATE)
}

/// Render the note-refs panel into `ui`. Returns the `document_id` of a clicked row (the caller stages
/// it on the bus + dispatches [`CMD_OPEN_DOCUMENT`](crate::interop::CMD_OPEN_DOCUMENT)), or `None` when
/// nothing was clicked this frame. The panel reads `focused_symbol` only for the header/empty-state
/// text; the load itself is driven by the code pane (this widget is pure rendering of `state`).
pub fn render_note_refs_panel(
    ui: &mut egui::Ui,
    state: &NoteRefsState,
    focused_symbol: Option<&str>,
    palette: &HsPalette,
) -> Option<String> {
    let mut clicked: Option<String> = None;

    // The container emits the Role::List node. Use a scope so the container response id is the node we
    // attach the panel author_id to (the same scope_builder pattern stage_pane.rs uses for its Region).
    let resp = ui
        .scope_builder(
            egui::UiBuilder::new().id_salt(egui::Id::new(PANEL_AUTHOR_ID)),
            |ui| {
                ui.label(
                    egui::RichText::new(format!("Notes referencing this symbol ({})", state.count()))
                        .strong()
                        .color(palette.text),
                );
                ui.separator();
                match state {
                    NoteRefsState::Idle => {
                        let hint = match focused_symbol {
                            Some(_) => "Resolving notes…",
                            None => "Hover a code symbol to see the notes that reference it.",
                        };
                        // NON-animating neutral label (NOT a spinner — the idle-repaint lesson).
                        ui.colored_label(palette.text_subtle, hint);
                    }
                    NoteRefsState::Loading => {
                        ui.horizontal(|ui| {
                            ui.add(egui::Spinner::new());
                            ui.colored_label(palette.text_subtle, "Searching notes…");
                        });
                    }
                    NoteRefsState::Loaded(notes) => {
                        if notes.is_empty() {
                            ui.colored_label(palette.text_subtle, "No notes reference this symbol.");
                        } else {
                            for note in notes {
                                if let Some(doc_id) = render_note_row(ui, note, palette) {
                                    clicked = Some(doc_id);
                                }
                            }
                        }
                    }
                    NoteRefsState::Failed(err) => {
                        ui.colored_label(
                            palette.error_text,
                            format!("Notes search failed ({}): {err}", err.kind_str()),
                        );
                    }
                }
            },
        )
        .response;

    // Emit the container Role::List node carrying the panel author_id (HBR-SWARM).
    let summary = match state {
        NoteRefsState::Loaded(n) if !n.is_empty() => format!("{} notes", n.len()),
        NoteRefsState::Loaded(_) => "no notes".to_owned(),
        NoteRefsState::Loading => "loading".to_owned(),
        NoteRefsState::Failed(e) => format!("error: {}", e.kind_str()),
        NoteRefsState::Idle => "idle".to_owned(),
    };
    emit_container_node(ui.ctx(), resp.id, summary);

    clicked
}

/// Render ONE note-ref row: a clickable title (truncated) + a subtle excerpt line + a right-arrow
/// affordance. Emits the `note-ref-{doc_id}` ListItem node with a `Press` action so a swarm agent can
/// activate it by id. Returns the `document_id` when clicked.
fn render_note_row(ui: &mut egui::Ui, note: &NoteRef, palette: &HsPalette) -> Option<String> {
    let title = row_title(note);
    let label = ui.add(
        egui::Label::new(egui::RichText::new(format!("{title}  ›")).color(palette.accent))
            .sense(egui::Sense::click()),
    );
    let author = row_author_id(&note.document_id);
    emit_row_node(ui.ctx(), label.id, &author, &title);

    // The excerpt centered on the symbol mention, a subtle non-interactive line beneath the title.
    if !note.excerpt.trim().is_empty() {
        ui.colored_label(palette.text_subtle, truncate(&note.excerpt, EXCERPT_TRUNCATE));
    }

    if label.clicked() {
        Some(note.document_id.clone())
    } else {
        None
    }
}

/// Emit the panel container's `Role::List` AccessKit node with the panel author_id + a load summary
/// value (so an out-of-process agent reads what the panel currently shows).
fn emit_container_node(ctx: &egui::Context, id: egui::Id, summary: String) {
    ctx.accesskit_node_builder(id, move |node| {
        node.set_role(PANEL_ROLE);
        node.set_author_id(PANEL_AUTHOR_ID.to_owned());
        node.set_label("Notes referencing this symbol".to_owned());
        node.set_value(summary.clone());
    });
}

/// Emit one note row's `Role::ListItem` node with a stable author_id, the title as its label, and a
/// `Press` action (the contract's `actions=[Press]`) so a swarm agent activates it by id.
fn emit_row_node(ctx: &egui::Context, id: egui::Id, author_id: &str, title: &str) {
    let author = author_id.to_owned();
    let title = title.to_owned();
    ctx.accesskit_node_builder(id, move |node| {
        node.set_role(ROW_ROLE);
        node.set_author_id(author.clone());
        node.set_label(title.clone());
        node.add_action(accesskit::Action::Click);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(id: &str, title: &str, excerpt: &str) -> NoteRef {
        NoteRef {
            block_id: id.to_owned(),
            document_id: id.to_owned(),
            document_title: title.to_owned(),
            excerpt: excerpt.to_owned(),
        }
    }

    #[test]
    fn author_ids_match_contract_shape() {
        assert_eq!(PANEL_AUTHOR_ID, "note-refs-panel");
        assert_eq!(row_author_id("DOC-2"), "note-ref-DOC-2");
        assert_eq!(ROW_AUTHOR_ID_PREFIX, "note-ref-");
    }

    #[test]
    fn roles_are_list_and_list_item() {
        assert_eq!(PANEL_ROLE, accesskit::Role::List);
        assert_eq!(ROW_ROLE, accesskit::Role::ListItem);
    }

    #[test]
    fn row_title_truncates_and_falls_back_to_doc_id() {
        let long = note("DOC-1", &"x".repeat(60), "");
        let t = row_title(&long);
        assert!(t.ends_with('…'), "a long title is truncated with an ellipsis");
        assert_eq!(t.chars().count(), TITLE_TRUNCATE + 1, "40 chars + ellipsis");
        let untitled = note("DOC-2", "", "");
        assert_eq!(row_title(&untitled), "DOC-2", "an untitled note falls back to its id");
    }

    #[test]
    fn state_count_only_for_loaded() {
        assert_eq!(NoteRefsState::Idle.count(), 0);
        assert_eq!(NoteRefsState::Loading.count(), 0);
        assert_eq!(NoteRefsState::Failed(CrossRefError::NoWorkspace).count(), 0);
        assert_eq!(NoteRefsState::Loaded(vec![note("A", "A", "")]).count(), 1);
    }

    #[test]
    fn truncate_is_char_correct() {
        assert_eq!(truncate("héllo", 3), "hél…");
        assert_eq!(truncate("hi", 5), "hi", "short string passes through unchanged");
        assert_eq!(truncate("", 5), "");
    }
}

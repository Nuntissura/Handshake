//! The full document properties panel widget (WP-KERNEL-012 MT-017).
//!
//! [`PropertiesPanel`] renders the metadata as a two-column `egui::Grid` (right-aligned field label |
//! left-aligned field value), reusing the [`fields`] renderers + the [`tag_editor`]. It is mounted by
//! the shared [`crate::rich_editor::renderer::rich_editor_widget`] inside a default-collapsed
//! `CollapsingHeader` ABOVE the document content.
//!
//! The panel borrows the live [`PropertiesState`] + [`PropertiesRuntime`] by `&mut` (so a title edit
//! dispatches a rename, a tag add/remove mutates the local list) + a [`ClipboardSink`] (so the
//! document-id copy is mockable in tests). It NEVER renders a perpetual Spinner: the backlinks-count
//! chip shows a static value/placeholder, and the save hint is a static label (the no-spinner gate).

use egui::accesskit;

use crate::rich_editor::properties::fields;
use crate::rich_editor::properties::metadata_client::{
    BacklinksCountState, ClipboardSink, PropertiesRuntime,
};
use crate::rich_editor::properties::tag_editor::tag_editor;
use crate::rich_editor::properties::{PropertiesState, PANEL_AUTHOR_ID};
use crate::theme::HsPalette;

/// The properties panel view. Construct it per frame with borrowed live state and call [`Self::show`]
/// inside the host's collapsing header.
pub struct PropertiesPanel<'a> {
    state: &'a mut PropertiesState,
    runtime: &'a mut PropertiesRuntime,
    clipboard: &'a dyn ClipboardSink,
    palette: &'a HsPalette,
}

impl<'a> PropertiesPanel<'a> {
    /// Build the panel over the live editor properties state + runtime + clipboard sink + theme.
    pub fn new(
        state: &'a mut PropertiesState,
        runtime: &'a mut PropertiesRuntime,
        clipboard: &'a dyn ClipboardSink,
        palette: &'a HsPalette,
    ) -> Self {
        Self {
            state,
            runtime,
            clipboard,
            palette,
        }
    }

    /// Render the panel body (the two-column grid + tags + backlinks-count). Intended to be called
    /// inside the host's `ui.collapsing("Properties", |ui| { … })`. Emits the `properties-panel`
    /// AccessKit container author_id onto the grid.
    pub fn show(self, ui: &mut egui::Ui) {
        let PropertiesPanel {
            state,
            runtime,
            clipboard,
            palette,
        } = self;

        // Fetch the backlinks count ONCE on load (Idle -> Loading); never per frame (RISK-4).
        runtime.ensure_backlinks_count_loaded();

        let grid_resp = egui::Grid::new("properties-grid")
            .num_columns(2)
            .spacing(egui::vec2(12.0, 6.0))
            .striped(true)
            .show(ui, |ui| {
                // Title (editable).
                label(ui, "Title", palette);
                fields::title_field(ui, state, runtime, palette);
                ui.end_row();

                // Tags (local-only + backend-gap banner).
                label(ui, "Tags", palette);
                tag_editor(ui, state, palette);
                ui.end_row();

                // Document id (read-only, click-to-copy).
                label(ui, "Document ID", palette);
                fields::doc_id_field(ui, state, clipboard, palette);
                ui.end_row();

                // Version badge.
                label(ui, "Version", palette);
                fields::version_badge(ui, state.doc_metadata.doc_version, palette);
                ui.end_row();

                // Authority label badge.
                label(ui, "Authority", palette);
                fields::authority_badge(ui, &state.doc_metadata.authority_label, palette);
                ui.end_row();

                // Owner (read-only).
                label(ui, "Owner", palette);
                {
                    let owner = owner_display(state);
                    fields::read_only_value(ui, owner.as_deref(), palette);
                }
                ui.end_row();

                // Project / folder refs (read-only here; editable-via-/move is a follow-on — the
                // read-only display binds the verified fields without faking an editor the contract
                // under-specified for these two).
                label(ui, "Project", palette);
                fields::read_only_value(ui, state.doc_metadata.project_ref.as_deref(), palette);
                ui.end_row();

                label(ui, "Folder", palette);
                fields::read_only_value(ui, state.doc_metadata.folder_ref.as_deref(), palette);
                ui.end_row();

                // CRDT id (read-only, displayed if present).
                label(ui, "CRDT", palette);
                fields::read_only_value(ui, state.doc_metadata.crdt_document_id.as_deref(), palette);
                ui.end_row();

                // Created / updated (local human-readable dates).
                label(ui, "Created", palette);
                fields::date_field(ui, &state.doc_metadata.created_at, palette);
                ui.end_row();

                label(ui, "Updated", palette);
                fields::date_field(ui, &state.doc_metadata.updated_at, palette);
                ui.end_row();

                // Backlinks count chip (`↑ N backlinks`) — a STATIC value, never a perpetual Spinner.
                label(ui, "Backlinks", palette);
                backlinks_count_chip(ui, runtime, palette);
                ui.end_row();
            });

        // The panel container carries the `properties-panel` author_id (a Group enclosing the grid).
        let author = PANEL_AUTHOR_ID.to_owned();
        ui.ctx().accesskit_node_builder(grid_resp.response.id, move |node| {
            node.set_role(accesskit::Role::Group);
            node.set_author_id(author.clone());
        });
    }
}

/// A right-aligned field label cell (the grid's left column).
fn label(ui: &mut egui::Ui, text: &str, palette: &HsPalette) {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.colored_label(palette.text_subtle, text);
    });
}

/// The composed owner display (`kind:id`), or `None` when neither is present.
fn owner_display(state: &PropertiesState) -> Option<String> {
    let kind = state.doc_metadata.owner_actor_kind.as_deref().filter(|s| !s.trim().is_empty());
    let id = state.doc_metadata.owner_actor_id.as_deref().filter(|s| !s.trim().is_empty());
    match (kind, id) {
        (Some(k), Some(i)) => Some(format!("{k}:{i}")),
        (Some(k), None) => Some(k.to_owned()),
        (None, Some(i)) => Some(i.to_owned()),
        (None, None) => None,
    }
}

/// Render the backlinks-count chip. Shows `↑ N backlinks` when loaded, a neutral non-animating
/// placeholder otherwise. CRITICAL (no-spinner gate): the `Loading` state shows a STATIC "loading…"
/// text, NOT an `egui::Spinner` — an animated spinner here would request a repaint every frame forever
/// in the idle/headless state (the MT-015 spinner-regression lesson).
fn backlinks_count_chip(ui: &mut egui::Ui, runtime: &PropertiesRuntime, palette: &HsPalette) {
    match &runtime.backlinks_count {
        BacklinksCountState::Loaded(n) => {
            ui.colored_label(palette.accent, format!("↑ {n} backlinks"));
        }
        BacklinksCountState::Loading => {
            // STATIC text — no Spinner (the no-spinner discipline).
            ui.colored_label(palette.text_subtle, "↑ … backlinks");
        }
        BacklinksCountState::Idle => {
            ui.colored_label(palette.text_subtle, "↑ — backlinks");
        }
        BacklinksCountState::Failed(e) => {
            ui.colored_label(palette.error_text, format!("↑ ? backlinks ({})", e.kind_str()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rich_editor::properties::metadata_client::DocMetadata;

    fn meta() -> DocMetadata {
        DocMetadata {
            rich_document_id: "KRD-1".into(),
            workspace_id: "ws".into(),
            title: "T".into(),
            doc_version: 1,
            authority_label: "draft".into(),
            owner_actor_kind: Some("operator".into()),
            owner_actor_id: Some("ilja".into()),
            project_ref: None,
            folder_ref: None,
            crdt_document_id: None,
            created_at: "2026-06-19T14:32:00Z".into(),
            updated_at: "2026-06-19T14:32:00Z".into(),
        }
    }

    #[test]
    fn owner_display_composes_kind_and_id() {
        let st = PropertiesState::new(meta());
        assert_eq!(owner_display(&st).as_deref(), Some("operator:ilja"));

        let mut none = PropertiesState::new(meta());
        none.doc_metadata.owner_actor_kind = None;
        none.doc_metadata.owner_actor_id = None;
        assert_eq!(owner_display(&none), None, "no owner -> None -> the field shows an em-dash");
    }
}

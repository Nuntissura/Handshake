//! The backlinks side panel (WP-KERNEL-012 MT-015) — the native port of the
//! `app/src/components/RichDocumentView.tsx` backlinks section.
//!
//! Renders a collapsible `Backlinks (N)` header listing every document that links to the current
//! document. Each entry is a clickable label; clicking enqueues a
//! [`EditorEvent::BacklinkActivated`] for the WP-011 shell to route (navigate to the source
//! document). A refresh button in the header re-fetches on explicit action (NOT per frame —
//! red-team RISK-4 / impl note 3). Empty state: "No backlinks yet." Fail state: a typed inline error.
//!
//! ## Backlinks are written by the backend on save, NOT by this MT
//!
//! When a document containing wikilinks is saved (MT-020 calls `saveRichDocument`), the backend
//! AUTOMATICALLY persists backlinks from the `content_json` (`handshake_core knowledge_document/
//! backlinks.rs`). The `hsLink` marks in the DocJson output carry the `ref_value` the backend
//! indexes. This panel ONLY READS backlinks (`GET /knowledge/documents/{id}/backlinks`); it never
//! writes them. No separate backlink-creation API call is made from this MT.
//!
//! AccessKit (impl note): the panel author_id is `backlinks-panel`, each entry is
//! `backlink-{source_doc_id}`, the refresh button is `backlinks-refresh` — registered through the
//! SAME WP-011 live-emission hook (no separate a11y layer).

use egui::accesskit;

use crate::interop::interaction_bus::InteractionBus;
use crate::rich_editor::wikilinks::client::RichDocBacklink;
use crate::rich_editor::wikilinks::inline_view::EditorEvent;
use crate::rich_editor::wikilinks::runtime::{BacklinksState, WikilinkRuntime};
use crate::theme::HsPalette;

/// The AccessKit author_id for the backlinks panel container (the contract id).
pub const PANEL_AUTHOR_ID: &str = "backlinks-panel";

/// The AccessKit author_id for the backlinks refresh button.
pub const REFRESH_AUTHOR_ID: &str = "backlinks-refresh";

/// The AccessKit author_id for one backlink entry (`backlink-{source_doc_id}`).
pub fn entry_author_id(source_document_id: &str) -> String {
    format!("backlink-{source_document_id}")
}

/// The label one backlink entry shows. The REAL backend `RichDocBacklink` carries NO `source_title`
/// (the MT contract assumed one); the panel labels by the source document id + link kind (the real
/// fields), which is the stable, accurate display until a title-join endpoint exists (a typed gap,
/// not a fabricated title).
pub fn entry_label(backlink: &RichDocBacklink) -> String {
    format!("{} ({})", backlink.source_document_id, backlink.link_kind)
}

/// Render the backlinks panel into `ui`, returning an optional [`EditorEvent::BacklinkActivated`] the
/// caller enqueues into `RichEditorState.pending_events` when an entry is clicked. Fetches the
/// backlinks ONCE on load (RISK-4: not per frame); a refresh button re-fetches on explicit action.
pub fn render_backlinks_panel(
    ui: &mut egui::Ui,
    runtime: &mut WikilinkRuntime,
    palette: &HsPalette,
) -> Option<EditorEvent> {
    // Fetch on first load (Idle -> Loading) — never every frame.
    runtime.ensure_backlinks_loaded();

    let mut event: Option<EditorEvent> = None;

    let count = match &runtime.backlinks {
        BacklinksState::Loaded(links) => links.len(),
        _ => 0,
    };
    let header = format!("Backlinks ({count})");

    let mut expanded = runtime.backlinks_expanded;
    let resp = egui::CollapsingHeader::new(header)
        .id_salt("backlinks-panel-header")
        .default_open(expanded)
        .show(ui, |ui| {
            // Header row: a refresh button (explicit re-fetch only — no polling).
            let refresh = ui.button("⟳ Refresh");
            emit_node_author(
                ui.ctx(),
                refresh.id,
                accesskit::Role::Button,
                REFRESH_AUTHOR_ID,
            );
            if refresh.clicked() {
                runtime.refresh_backlinks();
            }

            match runtime.backlinks.clone() {
                BacklinksState::Idle => {
                    // No fetch in flight (no runtime/document, or load not yet dispatched). Show a
                    // neutral, NON-animating state — an egui::Spinner here would request a repaint
                    // every frame forever (idle-CPU + harness.run() max_steps) when nothing resolves it.
                    ui.colored_label(palette.text_subtle, "Backlinks not loaded.");
                }
                BacklinksState::Loading => {
                    ui.horizontal(|ui| {
                        ui.add(egui::Spinner::new());
                        ui.colored_label(palette.text_subtle, "Loading backlinks…");
                    });
                }
                BacklinksState::Loaded(links) => {
                    if links.is_empty() {
                        ui.colored_label(palette.text_subtle, "No backlinks yet.");
                    } else {
                        for link in &links {
                            let label = ui.add(
                                egui::Label::new(
                                    egui::RichText::new(entry_label(link)).color(palette.accent),
                                )
                                .sense(egui::Sense::click()),
                            );
                            emit_node_author(
                                ui.ctx(),
                                label.id,
                                accesskit::Role::Link,
                                &entry_author_id(&link.source_document_id),
                            );
                            if label.clicked() {
                                event = Some(EditorEvent::BacklinkActivated {
                                    source_document_id: link.source_document_id.clone(),
                                });
                            }
                        }
                    }
                }
                BacklinksState::Failed(err) => {
                    ui.colored_label(
                        palette.error_text,
                        format!("Backlinks failed ({}): {err}", err.kind_str()),
                    );
                }
            }
        });
    // Persist the open/closed state so it survives across frames.
    expanded = resp.fully_open() || resp.openness > 0.5;
    runtime.backlinks_expanded = expanded;

    emit_node_author(
        ui.ctx(),
        resp.header_response.id,
        accesskit::Role::Group,
        PANEL_AUTHOR_ID,
    );

    event
}

/// WP-KERNEL-012 MT-032 (E5 melt-together): route a clicked backlink to the shared cross-pane
/// Open-Document command on the [`InteractionBus`] (AC-4). The MT-015 panel reports a clicked entry as
/// [`EditorEvent::BacklinkActivated`]; this bridge stages the target `source_document_id` on the bus and
/// dispatches [`crate::interop::interaction_bus::CMD_OPEN_DOCUMENT`], so the click fires the ONE named
/// cross-pane navigation command (not a per-pane ad-hoc callback). Returns the document id dispatched
/// for, or `None` for a non-backlink event. The caller must have run
/// [`InteractionBus::register_open_document_command`] once (the open command is then always present).
pub fn dispatch_backlink_open(
    ctx: &egui::Context,
    bus: &mut InteractionBus,
    event: &EditorEvent,
) -> Option<String> {
    if let EditorEvent::BacklinkActivated { source_document_id } = event {
        bus.open_document(ctx, source_document_id.clone());
        Some(source_document_id.clone())
    } else {
        None
    }
}

/// Emit a stable AccessKit author_id (+ role) onto an already-rendered node (same helper shape as the
/// embeds / transclusion dispatch). A Button keeps egui's role; a container/link gets the role set.
fn emit_node_author(ctx: &egui::Context, id: egui::Id, role: accesskit::Role, author_id: &str) {
    let role_for_closure = role;
    let author = author_id.to_owned();
    ctx.accesskit_node_builder(id, move |node| {
        if !matches!(role_for_closure, accesskit::Role::Button) {
            node.set_role(role_for_closure);
        }
        node.set_author_id(author);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn backlink(src: &str, kind: &str) -> RichDocBacklink {
        RichDocBacklink {
            backlink_id: format!("BL-{src}"),
            workspace_id: "ws".into(),
            relationship_id: "REL".into(),
            source_document_id: src.into(),
            link_kind: kind.into(),
            target: "DOC-1".into(),
            block_id: "BLK".into(),
        }
    }

    #[test]
    fn author_ids_match_contract_shape() {
        assert_eq!(PANEL_AUTHOR_ID, "backlinks-panel");
        assert_eq!(REFRESH_AUTHOR_ID, "backlinks-refresh");
        assert_eq!(entry_author_id("DOC-2"), "backlink-DOC-2");
    }

    #[test]
    fn entry_label_uses_real_fields_not_a_fabricated_title() {
        let bl = backlink("DOC-2", "note");
        assert_eq!(
            entry_label(&bl),
            "DOC-2 (note)",
            "the label uses the REAL source_document_id + link_kind (no fabricated source_title)"
        );
    }
}

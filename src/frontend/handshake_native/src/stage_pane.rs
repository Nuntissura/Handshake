//! Stage pane (WP-KERNEL-012 MT-033, cluster E5 — route-to-Stage).
//!
//! ## What this is (the LOCAL Stage pane + the route-to-stage command)
//!
//! [`StagePane`] is the native Stage surface that DISPLAYS content routed to it from another editor
//! surface: a whole document, a text selection, or a CKC/Atelier item. The "Route to Stage" command (on
//! the MT-031 [`crate::interop::InteractionBus`] command bus) opens/focuses the Stage pane and sets its
//! content. This is the LOCAL half of the Editors<->Stage (Pillar 17) interconnection edge.
//!
//! The DEEPER Stage backend interop (capture + embed-back with SHA-256 manifest provenance) is E10
//! (MT-066), NOT this MT. MT-033 delivers the local Stage pane that displays routed content + the
//! route-to-stage command on the MT-031 bus. If a Stage BACKEND route were needed here and were absent,
//! it would be a typed blocker — but the local display + bus command need no new backend route (the
//! routed payload travels in-process over the bus), so there is no backend blocker for this MT.
//!
//! ## The command lives on the MT-031 InteractionBus (reuse, don't fork)
//!
//! The route-to-stage command is registered on the existing [`crate::interop::InteractionBus`]
//! ([`register_route_to_stage_command`]) exactly as MT-032 registered the cross-pane Open-Document
//! command — the content is STAGED on the bus ([`InteractionBus`] extension methods) just before
//! dispatching [`CMD_ROUTE_TO_STAGE`], and the shell drains the staged content to open/focus the Stage
//! pane. It also appears in the static [`crate::command_registry`] palette catalog so a model SEES the
//! action. The bus is WRAPPED, not forked.
//!
//! ## AccessKit (HBR-SWARM)
//!
//! The Stage pane container emits author_id [`STAGE_PANE_AUTHOR_ID`] (`stage-pane`), Role::Region, with
//! its current content's summary as the node value so an out-of-process agent can read what is staged.

use egui::accesskit;

use crate::rich_editor::save::save_manager::RichDocLoad;
use crate::theme::HsPalette;

/// Stable AccessKit author_id for the Stage pane container (Role::Region).
pub const STAGE_PANE_AUTHOR_ID: &str = "stage-pane";

/// The content currently displayed in the Stage pane. The variant set is the MT-033 contract list
/// (`Document(RichDocLoad) | Selection(text, document_id) | AtelierItem(AtelierRef)`). `Empty` is the
/// default (nothing routed yet).
#[derive(Debug, Clone, PartialEq, Default)]
pub enum StageContent {
    /// No content routed yet (the empty Stage pane).
    #[default]
    Empty,
    /// A whole rich document routed to the stage (carries the loaded document so the pane shows its
    /// title + a content summary).
    Document(RichDocLoad),
    /// A text selection routed from a rich-text / code surface: `(selected_text, source_document_id)`.
    Selection(String, String),
    /// A CKC/Atelier item routed to the stage (the dragged reference).
    AtelierItem(crate::interop::AtelierRef),
}

impl StageContent {
    /// A one-line human/agent summary of the staged content (shown in the pane + the AccessKit value).
    pub fn summary(&self) -> String {
        match self {
            StageContent::Empty => "(nothing routed to Stage)".to_owned(),
            StageContent::Document(doc) => {
                let title = if doc.title.trim().is_empty() {
                    doc.rich_document_id.clone()
                } else {
                    doc.title.clone()
                };
                format!("Document: {title}")
            }
            StageContent::Selection(text, doc_id) => {
                let preview: String = text.chars().take(80).collect();
                format!("Selection from {doc_id}: \"{preview}\"")
            }
            StageContent::AtelierItem(r) => {
                format!("{} item: {}", r.item_kind.badge(), r.display_label())
            }
        }
    }

    /// True when content has actually been routed (not [`StageContent::Empty`]).
    pub fn is_some(&self) -> bool {
        !matches!(self, StageContent::Empty)
    }

    /// The stable content-kind wire string (the MT-036 `route_to_stage` payload `content_kind` field).
    pub fn content_kind(&self) -> &'static str {
        match self {
            StageContent::Empty => "empty",
            StageContent::Document(_) => "document",
            StageContent::Selection(..) => "selection",
            StageContent::AtelierItem(_) => "atelier_item",
        }
    }
}

/// The Stage pane widget state. Held by the host (in `app.rs`); mutated when the route-to-stage command
/// sets new content. Rendering is read-only (the pane DISPLAYS routed content; capture/embed-back is E10).
#[derive(Debug, Clone, Default)]
pub struct StagePane {
    /// The content currently staged.
    pub content: StageContent,
}

impl StagePane {
    /// A fresh, empty Stage pane.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the staged content (the route-to-stage handler / shell drain calls this).
    pub fn set_content(&mut self, content: StageContent) {
        self.content = content;
    }

    /// Render the Stage pane into `ui`, emitting the Role::Region AccessKit node. Read-only display.
    pub fn show(&mut self, ui: &mut egui::Ui, palette: &HsPalette) {
        let region_id = egui::Id::new(STAGE_PANE_AUTHOR_ID);
        let resp = ui
            .scope_builder(egui::UiBuilder::new().id_salt(region_id), |ui| {
                ui.label(egui::RichText::new("Stage").strong().color(palette.text));
                ui.separator();
                match &self.content {
                    StageContent::Empty => {
                        ui.label(
                            egui::RichText::new(
                                "Route a note, selection, or CKC item here via the 'Route to Stage' \
                                 command (right-click a selection or canvas node).",
                            )
                            .color(palette.text_subtle),
                        );
                    }
                    StageContent::Document(doc) => {
                        let title = if doc.title.trim().is_empty() {
                            doc.rich_document_id.as_str()
                        } else {
                            doc.title.as_str()
                        };
                        ui.label(egui::RichText::new(title).strong().color(palette.text));
                        ui.label(
                            egui::RichText::new(format!("document_id: {}", doc.rich_document_id))
                                .color(palette.text_subtle),
                        );
                    }
                    StageContent::Selection(text, doc_id) => {
                        ui.label(
                            egui::RichText::new(format!("Selection from {doc_id}"))
                                .color(palette.text_subtle),
                        );
                        ui.label(egui::RichText::new(text).color(palette.text));
                    }
                    StageContent::AtelierItem(r) => {
                        ui.label(
                            egui::RichText::new(format!("[{}] {}", r.item_kind.badge(), r.display_label()))
                                .color(palette.text),
                        );
                    }
                }
            })
            .response;
        emit_region_node(ui, resp.id, STAGE_PANE_AUTHOR_ID, &self.content.summary());
    }
}

/// MT-035 (E5 unified undo) — POLICY-2 CROSS-PANE undo for route-to-Stage. A route-to-stage action
/// touches two panes atomically (the source editor's selection/document AND the Stage pane), so it goes
/// on the CROSS-PANE ring (Ctrl+Shift+Z), NOT a single pane's local ring. The undo_fn reverts the Stage
/// pane's content to `previous` (its value BEFORE the route — captured AT ACTION-CREATE time, RISK-2);
/// the redo_fn re-routes `next`. Both capture a `Weak<Mutex<StagePane>>` back-ref to the host-held Stage
/// pane (RISK-3 / MC-3): they upgrade only during invocation and report a benign
/// [`crate::undo_stack::UndoResult::pane_dropped`] if the Stage pane was dropped — no retain cycle, no
/// panic. The route-to-stage command itself is the EXISTING MT-033 bus command; this only records the
/// undo entry so Ctrl+Shift+Z reverts the route (AC-2).
pub fn push_route_to_stage_undo(
    bus: &mut crate::interop::InteractionBus,
    stage: &std::sync::Arc<std::sync::Mutex<StagePane>>,
    previous: StageContent,
    next: StageContent,
    description: impl Into<String>,
) {
    use crate::undo_stack::{UndoAction, UndoFn, UndoResult};
    use std::sync::{Arc, Weak};

    let weak: Weak<std::sync::Mutex<StagePane>> = Arc::downgrade(stage);
    let undo_weak = weak.clone();
    let undo_fn: UndoFn = Arc::new(move || match undo_weak.upgrade() {
        Some(pane) => {
            pane.lock().unwrap_or_else(|e| e.into_inner()).set_content(previous.clone());
            UndoResult::ok()
        }
        None => UndoResult::pane_dropped(),
    });
    let redo_fn: UndoFn = Arc::new(move || match weak.upgrade() {
        Some(pane) => {
            pane.lock().unwrap_or_else(|e| e.into_inner()).set_content(next.clone());
            UndoResult::ok()
        }
        None => UndoResult::pane_dropped(),
    });
    bus.push_undo_cross_pane(UndoAction::sync(description, undo_fn, redo_fn));
}

/// Emit the Stage pane's Role::Region AccessKit node, with the staged-content summary as its value so an
/// out-of-process agent reads what is currently on the stage (HBR-SWARM / AC-6).
fn emit_region_node(ui: &egui::Ui, id: egui::Id, author_id: &str, summary: &str) {
    let author = author_id.to_owned();
    let summary = summary.to_owned();
    ui.ctx().accesskit_node_builder(id, move |node| {
        node.set_role(accesskit::Role::Region);
        node.set_author_id(author.clone());
        node.set_label("Stage".to_owned());
        node.set_value(summary.clone());
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interop::{AtelierItemKind, AtelierRef};

    fn doc(id: &str, title: &str) -> RichDocLoad {
        RichDocLoad {
            rich_document_id: id.to_owned(),
            doc_version: 1,
            title: title.to_owned(),
            content_json: None,
            updated_at: None,
        }
    }

    /// The empty stage summarizes as "(nothing routed…)" and `is_some()` is false.
    #[test]
    fn empty_stage_has_no_content() {
        let pane = StagePane::new();
        assert!(!pane.content.is_some());
        assert!(pane.content.summary().contains("nothing routed"));
    }

    /// Each content variant produces a sensible one-line summary (the AccessKit value).
    #[test]
    fn content_summaries_are_descriptive() {
        assert!(StageContent::Document(doc("DOC-1", "My Note")).summary().contains("My Note"));
        // A blank-title document falls back to its id.
        assert!(StageContent::Document(doc("DOC-2", "")).summary().contains("DOC-2"));
        let sel = StageContent::Selection("hello world".to_owned(), "DOC-3".to_owned());
        assert!(sel.summary().contains("DOC-3"));
        assert!(sel.summary().contains("hello world"));
        let item = StageContent::AtelierItem(AtelierRef::new("char-1", AtelierItemKind::Character, "Aria"));
        assert!(item.summary().contains("Character"));
        assert!(item.summary().contains("Aria"));
    }

    /// `set_content` replaces the staged content.
    #[test]
    fn set_content_replaces() {
        let mut pane = StagePane::new();
        pane.set_content(StageContent::Selection("x".to_owned(), "D".to_owned()));
        assert!(pane.content.is_some());
        match &pane.content {
            StageContent::Selection(t, d) => {
                assert_eq!(t, "x");
                assert_eq!(d, "D");
            }
            other => panic!("expected Selection, got {other:?}"),
        }
    }
}

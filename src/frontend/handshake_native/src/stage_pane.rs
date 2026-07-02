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

/// Stable AccessKit author_id for the Stage pane container (Role::Region for the MT-033 display surface;
/// the MT-066 contract names it GenericContainer — both are addressable container roles and the live tree
/// emits whichever the show path uses; the MT-066 round-trip surface emits GenericContainer via
/// [`StagePane::show_round_trip`]).
pub const STAGE_PANE_AUTHOR_ID: &str = "stage-pane";

/// WP-KERNEL-012 MT-066 (E10): stable AccessKit author_id for the "Capture -> Embed back" action button
/// (Role::Button, actions=[Press]) on the Stage pane — the embed-back leg's swarm-driveable trigger.
pub const STAGE_CAPTURE_EMBED_BACK_AUTHOR_ID: &str = "stage-capture-embed-back";

/// WP-KERNEL-012 MT-066 (E10): stable AccessKit author_id for the routed-content region
/// (Role::GenericContainer) inside the Stage pane — where [`StagePane::receive_routed_content`] renders the
/// routed note / selection / canvas node so a swarm agent can read what was routed.
pub const STAGE_ROUTED_CONTENT_AUTHOR_ID: &str = "stage-routed-content";

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

/// Where an embed-back NodeView is inserted: a note pane (rich-text document model) or a canvas pane
/// (node graph). Resolved by the host through the WP-011 pane registry / shared-bus focus owner; the Stage
/// pane refuses to embed if the target pane is no longer live (RISK-007/MC-007).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmbedTarget {
    /// Insert the embed atom into a note's rich-text document (the pane id + the target document id).
    Note {
        pane_id: String,
        document_id: String,
    },
    /// Insert the embed atom onto a canvas board (the pane id + the canvas id).
    Canvas { pane_id: String, canvas_id: String },
}

impl EmbedTarget {
    /// The target pane id (so the host can re-resolve liveness at embed time).
    pub fn pane_id(&self) -> &str {
        match self {
            EmbedTarget::Note { pane_id, .. } | EmbedTarget::Canvas { pane_id, .. } => pane_id,
        }
    }
}

/// The Stage pane widget state. Held by the host (in `app.rs`); mutated when the route-to-stage command
/// sets new content. MT-033 delivered the read-only display; MT-066 (E10) adds the embed-back leg:
/// [`StagePane::capture_and_embed_back`] fetches a Stage capture artifact and inserts it as an MT-014
/// embed NodeView into a note/canvas, plus the typed-blocker empty-state when the embed-back route is
/// absent.
#[derive(Debug, Clone, Default)]
pub struct StagePane {
    /// The content currently staged.
    pub content: StageContent,
    /// WP-KERNEL-012 MT-066: the last embed-back outcome, surfaced in the pane (the inserted NodeView's
    /// provenance summary on success, or the typed-blocker empty-state on `EmbedBackEndpointAbsent` /
    /// `ProvenanceMissing`). `None` until an embed-back is attempted.
    pub last_embed_back: Option<EmbedBackOutcome>,
}

/// The outcome of a [`StagePane::capture_and_embed_back`] call, surfaced in the pane + readable by the
/// host for the validator handoff. The blocker variants are NEVER swallowed (AC-004).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmbedBackOutcome {
    /// The capture was embedded: carries the inserted embed atom's artifact id + its SHA-256 provenance
    /// summary (so the pane shows the evidence anchor).
    Embedded {
        artifact_id: String,
        sha256: String,
        target_pane: String,
    },
    /// The embed-back route is absent in this build (the typed blocker). Carries the probed path.
    EndpointAbsent { probed_path: String },
    /// The fetched artifact had no SHA-256 / manifest provenance, so it was refused.
    ProvenanceMissing,
    /// The embed target pane was no longer live at embed time (re-resolution failed — RISK-007/MC-007).
    TargetGone { pane_id: String },
    /// A transport / decode failure that is not the typed blocker.
    Failed(String),
}

impl EmbedBackOutcome {
    /// True when this is the embed-back typed-blocker outcome (the pane renders the empty-state banner and
    /// the host surfaces it to the WP validator).
    pub fn is_endpoint_absent(&self) -> bool {
        matches!(self, EmbedBackOutcome::EndpointAbsent { .. })
    }

    /// A one-line human/agent summary (the AccessKit value on the embed-back status line).
    pub fn summary(&self) -> String {
        match self {
            EmbedBackOutcome::Embedded {
                artifact_id,
                sha256,
                target_pane,
            } => format!(
                "Embedded {artifact_id} into {target_pane} (sha256 {})",
                short_sha(sha256)
            ),
            EmbedBackOutcome::EndpointAbsent { probed_path } => {
                format!("Stage embed-back endpoint not present (probed {probed_path})")
            }
            EmbedBackOutcome::ProvenanceMissing => {
                "Embed-back refused: fetched capture has no SHA-256 / manifest provenance"
                    .to_owned()
            }
            EmbedBackOutcome::TargetGone { pane_id } => {
                format!("Embed-back target pane '{pane_id}' is no longer live")
            }
            EmbedBackOutcome::Failed(why) => format!("Embed-back failed: {why}"),
        }
    }
}

/// The first 12 hex chars of a SHA-256 for compact display (char-boundary safe).
fn short_sha(hash: &str) -> &str {
    match hash.char_indices().nth(12) {
        Some((idx, _)) => &hash[..idx],
        None => hash,
    }
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

    /// WP-KERNEL-012 MT-066: receive routed content (the route-leg landing). Sets the Stage pane's
    /// displayed content from a routed [`crate::interop::StageRoutePayload`]'s staged form. This is the
    /// thin adapter the shell uses when it drains the bus's staged content for a route that originated from
    /// the MT-066 payload builders (selection / canvas node); it delegates to [`Self::set_content`] so the
    /// display path is shared with MT-033.
    pub fn receive_routed_content(&mut self, content: StageContent) {
        self.set_content(content);
    }

    /// WP-KERNEL-012 MT-066 (the embed-back leg): fetch a Stage capture artifact, convert it to an MT-014
    /// embed NodeView, and (via `insert`) insert it into `target`. PURE of egui — the host supplies the
    /// async fetch result + the insert closure so this is unit-provable without a runtime or a live socket.
    ///
    /// Behavior (records the outcome in [`Self::last_embed_back`] and returns it):
    /// - `target` liveness is RE-RESOLVED at embed time via `is_target_live` (RISK-007/MC-007): a dangling
    ///   target yields [`EmbedBackOutcome::TargetGone`] and NO insert.
    /// - The `fetch` result's [`crate::interop::StageInteropError::EmbedBackEndpointAbsent`] maps to
    ///   [`EmbedBackOutcome::EndpointAbsent`] (the typed blocker, AC-004) — NO insert, surfaced upward.
    /// - A fetched artifact with no SHA-256 / manifest provenance maps to
    ///   [`EmbedBackOutcome::ProvenanceMissing`] (RISK-002/MC-002) — NO insert.
    /// - On success the MT-014 embed NodeView is built and handed to `insert`; the outcome records the
    ///   artifact id + sha256 + target pane.
    pub fn capture_and_embed_back<L, I>(
        &mut self,
        fetch_result: Result<crate::interop::StageArtifactRef, crate::interop::StageInteropError>,
        target: &EmbedTarget,
        mut is_target_live: L,
        mut insert: I,
    ) -> EmbedBackOutcome
    where
        L: FnMut(&str) -> bool,
        I: FnMut(&crate::interop::EmbedNodeView, &EmbedTarget),
    {
        // RISK-007/MC-007: re-resolve the embed target at embed time; refuse a dangling pane.
        if !is_target_live(target.pane_id()) {
            let outcome = EmbedBackOutcome::TargetGone {
                pane_id: target.pane_id().to_owned(),
            };
            self.last_embed_back = Some(outcome.clone());
            return outcome;
        }
        let outcome = match fetch_result {
            Err(crate::interop::StageInteropError::EmbedBackEndpointAbsent { probed_path }) => {
                EmbedBackOutcome::EndpointAbsent { probed_path }
            }
            Err(crate::interop::StageInteropError::ProvenanceMissing) => {
                EmbedBackOutcome::ProvenanceMissing
            }
            Err(other) => EmbedBackOutcome::Failed(other.to_string()),
            Ok(artifact) => match crate::interop::embed_artifact_as_nodeview(&artifact) {
                Ok(view) => {
                    let artifact_id = view.provenance.artifact_id.clone();
                    let sha256 = view.provenance.sha256.clone();
                    insert(&view, target);
                    EmbedBackOutcome::Embedded {
                        artifact_id,
                        sha256,
                        target_pane: target.pane_id().to_owned(),
                    }
                }
                Err(crate::interop::StageInteropError::ProvenanceMissing) => {
                    EmbedBackOutcome::ProvenanceMissing
                }
                Err(other) => EmbedBackOutcome::Failed(other.to_string()),
            },
        };
        self.last_embed_back = Some(outcome.clone());
        outcome
    }

    /// True when the last embed-back attempt hit the typed-blocker (the host surfaces it to the WP
    /// validator and the pane renders the empty-state banner).
    pub fn has_embed_back_endpoint_absent_blocker(&self) -> bool {
        self.last_embed_back
            .as_ref()
            .map(|o| o.is_endpoint_absent())
            .unwrap_or(false)
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

    /// WP-KERNEL-012 MT-066 (E10) — render the FULL Stage round-trip surface: the routed-content region
    /// (the route-leg landing) PLUS the "Capture -> Embed back" action (the embed-back leg trigger) PLUS
    /// the last embed-back status / typed-blocker empty-state. Emits the three MT-066 AccessKit nodes a
    /// swarm agent drives the round-trip by (AC-006 / PT-005):
    ///
    /// - `stage-pane` (`Role::GenericContainer`) — the outer round-trip container.
    /// - `stage-routed-content` (`Role::GenericContainer`) — the region showing what was routed.
    /// - `stage-capture-embed-back` (`Role::Button`) — the embed-back trigger.
    ///
    /// Returns `true` when the embed-back button was pressed this frame (the host then runs the async
    /// fetch + [`Self::capture_and_embed_back`]). NO network/disk IO happens here (render is pure).
    pub fn show_round_trip(&mut self, ui: &mut egui::Ui, palette: &HsPalette) -> bool {
        let mut embed_back_pressed = false;
        let container_id = egui::Id::new("stage-pane-round-trip");
        let resp = ui
            .scope_builder(egui::UiBuilder::new().id_salt(container_id), |ui| {
                ui.label(egui::RichText::new("Stage").strong().color(palette.text));
                ui.separator();

                // The routed-content region (Role::GenericContainer) — the route-leg landing.
                let routed_id = egui::Id::new(STAGE_ROUTED_CONTENT_AUTHOR_ID);
                let routed_resp = ui
                    .scope_builder(egui::UiBuilder::new().id_salt(routed_id), |ui| {
                        ui.label(
                            egui::RichText::new("Routed content")
                                .color(palette.text_subtle)
                                .small(),
                        );
                        ui.label(egui::RichText::new(self.content.summary()).color(palette.text));
                    })
                    .response;
                let routed_author = STAGE_ROUTED_CONTENT_AUTHOR_ID.to_owned();
                let routed_value = self.content.summary();
                ui.ctx()
                    .accesskit_node_builder(routed_resp.id, move |node| {
                        node.set_role(accesskit::Role::GenericContainer);
                        node.set_author_id(routed_author.clone());
                        node.set_label("Routed content".to_owned());
                        node.set_value(routed_value.clone());
                    });

                ui.separator();

                // The "Capture -> Embed back" action button (Role::Button). Enabled only when content has
                // been routed (an empty stage has nothing to capture).
                let has_content = self.content.is_some();
                let btn = egui::Button::new(egui::RichText::new("Capture → Embed back").color(
                    if has_content {
                        palette.accent
                    } else {
                        palette.text_subtle
                    },
                ));
                let btn_resp = ui.add_enabled(has_content, btn);
                crate::accessibility::emit_interactive_node(
                    ui.ctx(),
                    btn_resp.id,
                    STAGE_CAPTURE_EMBED_BACK_AUTHOR_ID,
                );
                if btn_resp.clicked() {
                    embed_back_pressed = true;
                }

                // The last embed-back outcome / typed-blocker empty-state.
                if let Some(outcome) = &self.last_embed_back {
                    let color = match outcome {
                        EmbedBackOutcome::Embedded { .. } => palette.text,
                        _ => palette.text_subtle,
                    };
                    ui.colored_label(color, outcome.summary());
                }
            })
            .response;

        // The outer round-trip container node (Role::GenericContainer) — the MT-066 swarm address. (The
        // MT-033 read-only `show` path emits `stage-pane` as Role::Region; the MT-066 round-trip path
        // emits it as GenericContainer, the contract's named role for the round-trip surface.)
        let author = STAGE_PANE_AUTHOR_ID.to_owned();
        let summary = self.content.summary();
        ui.ctx().accesskit_node_builder(resp.id, move |node| {
            node.set_role(accesskit::Role::GenericContainer);
            node.set_author_id(author.clone());
            node.set_label("Stage".to_owned());
            node.set_value(summary.clone());
        });

        embed_back_pressed
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
            pane.lock()
                .unwrap_or_else(|e| e.into_inner())
                .set_content(previous.clone());
            UndoResult::ok()
        }
        None => UndoResult::pane_dropped(),
    });
    let redo_fn: UndoFn = Arc::new(move || match weak.upgrade() {
        Some(pane) => {
            pane.lock()
                .unwrap_or_else(|e| e.into_inner())
                .set_content(next.clone());
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
        assert!(StageContent::Document(doc("DOC-1", "My Note"))
            .summary()
            .contains("My Note"));
        // A blank-title document falls back to its id.
        assert!(StageContent::Document(doc("DOC-2", ""))
            .summary()
            .contains("DOC-2"));
        let sel = StageContent::Selection("hello world".to_owned(), "DOC-3".to_owned());
        assert!(sel.summary().contains("DOC-3"));
        assert!(sel.summary().contains("hello world"));
        let item = StageContent::AtelierItem(AtelierRef::new(
            "char-1",
            AtelierItemKind::Character,
            "Aria",
        ));
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

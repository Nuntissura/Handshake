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
//! The Stage pane container emits author_id [`STAGE_PANE_AUTHOR_ID`] (`stage-pane`),
//! Role::GenericContainer, with
//! its current content's summary as the node value so an out-of-process agent can read what is staged.

use egui::accesskit;

use crate::rich_editor::save::save_manager::RichDocLoad;
use crate::theme::HsPalette;

/// Exclusive ownership of the Stage capture action from click admission through the terminal rich-save
/// and EventLedger outcome. Dropping the lease releases the latch on every success/failure path.
pub(crate) struct StageEmbedInFlightLease(std::sync::Arc<std::sync::atomic::AtomicBool>);

impl StageEmbedInFlightLease {
    pub(crate) fn new(flag: std::sync::Arc<std::sync::atomic::AtomicBool>) -> Self {
        Self(flag)
    }
}

impl Drop for StageEmbedInFlightLease {
    fn drop(&mut self) {
        self.0.store(false, std::sync::atomic::Ordering::Release);
    }
}

/// Stable AccessKit author_id for the Stage pane container. Both render paths emit the exact
/// `Role::GenericContainer` required by the MT-066 accessibility contract.
pub const STAGE_PANE_AUTHOR_ID: &str = "stage-pane";

/// WP-KERNEL-012 MT-066 (E10): stable AccessKit author_id for the "Capture -> Embed back" action button
/// (Role::Button, actions=[Press]) on the Stage pane — the embed-back leg's swarm-driveable trigger.
pub const STAGE_CAPTURE_EMBED_BACK_AUTHOR_ID: &str = "stage-capture-embed-back";

/// WP-KERNEL-012 MT-066 (E10): stable AccessKit author_id for the routed-content region
/// (Role::GenericContainer) inside the Stage pane — where [`StagePane::receive_routed_content`] renders the
/// routed note / selection / canvas node so a swarm agent can read what was routed.
pub const STAGE_ROUTED_CONTENT_AUTHOR_ID: &str = "stage-routed-content";

/// WP-KERNEL-012 MT-066 (E10) REMEDIATION (operator reopen item 4): stable AccessKit author_id for the
/// embed-back STATUS line / typed-blocker empty-state banner (Role::Label) rendered by
/// [`StagePane::show_round_trip`]. Its node value carries the [`EmbedBackOutcome::summary`] so an operator
/// or swarm agent can READ the outcome — most importantly the `EmbedBackEndpointAbsent` typed blocker — as
/// an addressable, perceivable surface rather than only internal state. `None` until an embed-back is
/// attempted (then the banner + this node render).
pub const STAGE_EMBED_BACK_STATUS_AUTHOR_ID: &str = "stage-embed-back-status";
/// Stable AccessKit author_id for a typed Route-to-Stage failure.
pub const STAGE_ROUTE_STATUS_AUTHOR_ID: &str = "stage-route-status";
pub const STAGE_ROUTE_RETRY_AUTHOR_ID: &str = "stage-route-retry";

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
    /// Immutable action identity inherited from the StageRoutePayload and reused by embed-back FR.
    pub causal_action_id: Option<String>,
    /// Typed failure from a route command that had no valid active document/selection.
    pub route_error: Option<String>,
    /// Exact route retained after InteractionBus contention; the Stage pane exposes a stable Retry
    /// action instead of silently dropping the request.
    pub route_retry: Option<crate::interop::PendingStageRoute>,
    /// A visible Stage route was committed but its exact Flight Recorder receipt has not yet been
    /// accepted by the bounded frame/EventLedger queue. While present, admission of another route is
    /// blocked so this recoverable identity can never be overwritten by a later visible commit.
    pub route_receipt_retry: Option<crate::event_emitter::NativeEditorEvent>,
    route_receipt_in_flight: Option<String>,
    /// WP-KERNEL-012 MT-066: the last embed-back outcome, surfaced in the pane (the inserted NodeView's
    /// provenance summary on success, or the typed-blocker empty-state on `EmbedBackEndpointAbsent` /
    /// `ProvenanceMissing`). `None` until an embed-back is attempted.
    pub last_embed_back: Option<EmbedBackOutcome>,
}

/// Exact Stage state changed by one route action. Cross-pane undo/redo restores this snapshot so the
/// routed content never becomes detached from the immutable causal action id that embed-back reuses.
#[derive(Debug, Clone)]
pub struct StageRouteSnapshot {
    content: StageContent,
    causal_action_id: Option<String>,
    route_error: Option<String>,
    route_retry: Option<crate::interop::PendingStageRoute>,
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
    /// The embed atom was inserted locally and its canonical document save is in flight. This is
    /// deliberately non-terminal: no success receipt has been emitted yet.
    Persisting {
        artifact_id: String,
        sha256: String,
        target_pane: String,
    },
    /// The document already contains the exact embed, but EventLedger acknowledgement is ambiguous.
    /// The immutable receipt is retained by the shell and the Stage action retries that same event id;
    /// it must never create another hsLink while this state is visible.
    LedgerPending {
        artifact_id: String,
        sha256: String,
        target_pane: String,
        event_id: String,
        error: String,
    },
    /// The embed-back route is absent in this build (the typed blocker). Carries the probed path.
    EndpointAbsent { probed_path: String },
    /// The fetched artifact had no SHA-256 / manifest provenance, so it was refused.
    ProvenanceMissing,
    /// The embed target pane was no longer live at embed time (re-resolution failed — RISK-007/MC-007).
    TargetGone { pane_id: String },
    /// A second operator request arrived while the first capture/embed operation was still active.
    Busy,
    /// The native shell has no async runtime, so no capture request was dispatched or lost silently.
    RuntimeUnavailable,
    /// A transport / decode failure that is not the typed blocker.
    Failed(String),
}

/// Durability state returned by the product insertion callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbedInsertStatus {
    Durable,
    Persisting,
    Busy,
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
            EmbedBackOutcome::Persisting {
                artifact_id,
                sha256,
                target_pane,
            } => format!(
                "Persisting {artifact_id} into {target_pane} (sha256 {})",
                short_sha(sha256)
            ),
            EmbedBackOutcome::LedgerPending {
                artifact_id,
                sha256,
                target_pane,
                event_id,
                error,
            } => format!(
                "Document saved with {artifact_id} in {target_pane} (sha256 {}), but EventLedger receipt {event_id} is pending: {error}. Retry reuses this exact receipt.",
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
            EmbedBackOutcome::Busy => {
                "Embed-back rejected: another Stage capture is already in progress".to_owned()
            }
            EmbedBackOutcome::RuntimeUnavailable => {
                "Embed-back unavailable: the native async runtime is not running".to_owned()
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
        self.set_content_correlated(content, None);
    }

    pub fn set_content_correlated(
        &mut self,
        content: StageContent,
        causal_action_id: Option<String>,
    ) {
        self.content = content;
        self.causal_action_id = causal_action_id;
        // A different producer can be rejected while the bus still owns the route being committed
        // here. That rejected producer's exact request lives in `route_retry`; applying the older
        // admitted route must not silently discard it. A successful retry clears its own slot in the
        // retry action before the bus commit reaches this method.
        if self.route_retry.is_none() {
            self.route_error = None;
        }
        // Embed UI belongs to the prior route. The shell may re-surface LedgerPending only from its
        // separately retained exact receipt; StagePane never preserves a status without that identity.
        self.last_embed_back = None;
    }

    /// Capture the complete route-owned state for one atomic cross-pane undo action.
    pub fn route_snapshot(&self) -> StageRouteSnapshot {
        StageRouteSnapshot {
            content: self.content.clone(),
            causal_action_id: self.causal_action_id.clone(),
            route_error: self.route_error.clone(),
            route_retry: self.route_retry.clone(),
        }
    }

    fn restore_route_snapshot(&mut self, snapshot: &StageRouteSnapshot) {
        self.content = snapshot.content.clone();
        self.causal_action_id = snapshot.causal_action_id.clone();
        self.route_error = snapshot.route_error.clone();
        self.route_retry = snapshot.route_retry.clone();
        // Async embed/ledger state is not route history. Restoring it without the shell-owned exact
        // receipt would fabricate a retry button that could launch a new capture.
        self.last_embed_back = None;
    }

    /// Surface a typed route failure without fabricating Stage content.
    pub fn set_route_error(&mut self, message: impl Into<String>) {
        self.content = StageContent::Empty;
        self.causal_action_id = None;
        self.route_error = Some(message.into());
        self.route_retry = None;
        self.last_embed_back = None;
    }

    pub fn set_route_busy(&mut self, route: crate::interop::PendingStageRoute) {
        self.content = StageContent::Empty;
        self.causal_action_id = None;
        self.route_error = Some(
            "Route to Stage is busy; the request was retained. Use Retry Route to Stage."
                .to_owned(),
        );
        self.route_retry = Some(route);
        // Same ownership rule as `set_route_error`: only the shell may re-surface LedgerPending from
        // its separately retained exact receipt after this route attempt changes the visible context.
        self.last_embed_back = None;
    }

    /// Whether an exact route payload is retained for the operator-visible retry action.
    pub fn has_route_retry(&self) -> bool {
        self.route_retry.is_some()
    }

    pub fn retain_route_receipt(&mut self, receipt: crate::event_emitter::NativeEditorEvent) {
        debug_assert!(
            self.route_receipt_retry
                .as_ref()
                .is_none_or(|pending| pending.event_id == receipt.event_id),
            "route receipt admission must not overwrite a different exact event"
        );
        if self.route_receipt_retry.is_none() {
            self.route_receipt_retry = Some(receipt);
        }
    }

    pub fn pending_route_receipt(&self) -> Option<crate::event_emitter::NativeEditorEvent> {
        self.route_receipt_retry.clone()
    }

    pub fn acknowledge_route_receipt(&mut self, event_id: &str) {
        if self
            .route_receipt_retry
            .as_ref()
            .is_some_and(|receipt| receipt.event_id == event_id)
        {
            self.route_receipt_retry = None;
        }
    }

    pub fn has_pending_route_receipt(&self) -> bool {
        self.route_receipt_retry.is_some()
    }

    fn route_receipt_status(&self) -> Option<String> {
        self.route_receipt_retry.as_ref().map(|receipt| {
            format!(
                "Stage content committed; EventLedger receipt {} is retained and retrying before another route is admitted.",
                receipt.event_id
            )
        })
    }

    pub fn begin_route_receipt_attempt(
        &mut self,
    ) -> Option<crate::event_emitter::NativeEditorEvent> {
        if self.route_receipt_in_flight.is_some() {
            return None;
        }
        let receipt = self.route_receipt_retry.clone()?;
        self.route_receipt_in_flight = Some(receipt.event_id.clone());
        Some(receipt)
    }

    pub fn finish_route_receipt_attempt(&mut self, event_id: &str, persisted: bool) {
        if self.route_receipt_in_flight.as_deref() != Some(event_id) {
            return;
        }
        self.route_receipt_in_flight = None;
        if persisted {
            self.acknowledge_route_receipt(event_id);
        }
    }

    pub(crate) fn embed_back_action_label(&self) -> &'static str {
        if matches!(
            self.last_embed_back.as_ref(),
            Some(EmbedBackOutcome::LedgerPending { .. })
        ) {
            "Retry exact EventLedger receipt"
        } else {
            "Capture → Embed back"
        }
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
        L: FnMut(&EmbedTarget) -> bool,
        I: FnMut(&crate::interop::EmbedNodeView, &EmbedTarget) -> Result<(), String>,
    {
        self.capture_and_embed_back_with_status(
            fetch_result,
            target,
            is_target_live,
            |view, target| insert(view, target).map(|()| EmbedInsertStatus::Durable),
        )
    }

    /// Product-path variant whose insert callback distinguishes a backend-durable insert from a rich
    /// document insert that is still awaiting canonical save + EventLedger persistence.
    pub fn capture_and_embed_back_with_status<L, I>(
        &mut self,
        fetch_result: Result<crate::interop::StageArtifactRef, crate::interop::StageInteropError>,
        target: &EmbedTarget,
        mut is_target_live: L,
        mut insert: I,
    ) -> EmbedBackOutcome
    where
        L: FnMut(&EmbedTarget) -> bool,
        I: FnMut(&crate::interop::EmbedNodeView, &EmbedTarget) -> Result<EmbedInsertStatus, String>,
    {
        // RISK-007/MC-007: re-resolve the embed target at embed time; refuse a dangling pane.
        if !is_target_live(target) {
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
                    match insert(&view, target) {
                        Ok(EmbedInsertStatus::Durable) => EmbedBackOutcome::Embedded {
                            artifact_id,
                            sha256,
                            target_pane: target.pane_id().to_owned(),
                        },
                        Ok(EmbedInsertStatus::Persisting) => EmbedBackOutcome::Persisting {
                            artifact_id,
                            sha256,
                            target_pane: target.pane_id().to_owned(),
                        },
                        Ok(EmbedInsertStatus::Busy) => EmbedBackOutcome::Busy,
                        Err(error) => EmbedBackOutcome::Failed(error),
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

    /// Render the Stage pane into `ui`, emitting the Role::GenericContainer AccessKit node.
    pub fn show(&mut self, ui: &mut egui::Ui, palette: &HsPalette) {
        let region_id = egui::Id::new(STAGE_PANE_AUTHOR_ID);
        let resp = ui
            .scope_builder(egui::UiBuilder::new().id_salt(region_id), |ui| {
                ui.label(egui::RichText::new("Stage").strong().color(palette.text));
                ui.separator();
                if self.route_error.is_some() {
                    self.show_route_status_and_retry(ui, palette);
                    ui.separator();
                } else if let Some(message) = self.route_receipt_status() {
                    emit_route_status(ui, palette, &message);
                    ui.separator();
                }
                match &self.content {
                    StageContent::Empty => {
                        ui.label(
                            egui::RichText::new(
                                "Route the active note or selection here with 'Route to Stage'. You can \
                                 also right-click a Canvas node or route an Atelier item.",
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
        let summary = self
            .route_error
            .clone()
            .unwrap_or_else(|| self.content.summary());
        emit_region_node(ui, resp.id, STAGE_PANE_AUTHOR_ID, &summary);
    }

    /// Render the shared route failure and its exact-payload retry action. Both the compact Stage view
    /// and the docked round-trip view call this helper so an operator never loses the retry control merely
    /// by opening Stage through a different entry point.
    fn show_route_status_and_retry(&mut self, ui: &mut egui::Ui, palette: &HsPalette) {
        let Some(message) = self.route_error.clone() else {
            return;
        };
        emit_route_status(ui, palette, &message);
        if self.route_retry.is_none() {
            return;
        }

        let retry = ui
            .scope_builder(
                egui::UiBuilder::new().id_salt(egui::Id::new(STAGE_ROUTE_RETRY_AUTHOR_ID)),
                |ui| ui.button("Retry Route to Stage"),
            )
            .inner;
        ui.ctx().accesskit_node_builder(retry.id, |node| {
            node.set_role(accesskit::Role::Button);
            node.set_author_id(STAGE_ROUTE_RETRY_AUTHOR_ID.to_owned());
            node.set_label("Retry Route to Stage".to_owned());
            node.add_action(accesskit::Action::Click);
        });
        if !retry.clicked() {
            return;
        }

        let route = self.route_retry.clone();
        let bus = crate::interop::InteractionBus::get_or_init(ui.ctx());
        if crate::interop::InteractionBus::with_try_lock(&bus, |bus| {
            bus.register_route_to_stage_command();
            route.is_some_and(|route| bus.retry_pending_stage_route(ui.ctx(), route))
        })
        .unwrap_or(false)
        {
            self.route_error = None;
            self.route_retry = None;
        } else {
            self.route_error = Some(
                "Route to Stage is still busy; retry when the shared editor bus is available."
                    .to_owned(),
            );
        }
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
        let container_id = egui::Id::new(STAGE_PANE_AUTHOR_ID);
        let resp = ui
            .scope_builder(egui::UiBuilder::new().id_salt(container_id), |ui| {
                ui.label(egui::RichText::new("Stage").strong().color(palette.text));
                ui.separator();

                if self.route_error.is_some() {
                    self.show_route_status_and_retry(ui, palette);
                    ui.separator();
                } else if let Some(message) = self.route_receipt_status() {
                    emit_route_status(ui, palette, &message);
                    ui.separator();
                }

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

                // The "Capture -> Embed back" action button (Role::Button). A new capture requires routed
                // content; an already-saved ledger receipt remains recoverable even if a later route error
                // emptied the Stage content.
                let has_content = self.content.is_some();
                let ledger_retry = matches!(
                    self.last_embed_back.as_ref(),
                    Some(EmbedBackOutcome::LedgerPending { .. })
                );
                let action_label = self.embed_back_action_label();
                let action_enabled = has_content || ledger_retry;
                let btn =
                    egui::Button::new(egui::RichText::new(action_label).color(if action_enabled {
                        palette.accent
                    } else {
                        palette.text_subtle
                    }));
                let btn_resp = ui
                    .scope_builder(
                        egui::UiBuilder::new()
                            .id_salt(egui::Id::new(STAGE_CAPTURE_EMBED_BACK_AUTHOR_ID)),
                        |ui| ui.add_enabled(action_enabled, btn),
                    )
                    .inner;
                crate::accessibility::emit_interactive_node(
                    ui.ctx(),
                    btn_resp.id,
                    STAGE_CAPTURE_EMBED_BACK_AUTHOR_ID,
                );
                if btn_resp.clicked() {
                    embed_back_pressed = true;
                }

                // The last embed-back outcome / typed-blocker empty-state. Emitted as an ADDRESSABLE
                // AccessKit node (STAGE_EMBED_BACK_STATUS_AUTHOR_ID, Role::Label) whose value is the outcome
                // summary, so the operator / a swarm agent can READ the outcome — most importantly the
                // EmbedBackEndpointAbsent typed blocker — as a perceivable surface, not only internal state.
                if let Some(outcome) = &self.last_embed_back {
                    let color = match outcome {
                        EmbedBackOutcome::Embedded { .. } => palette.text,
                        _ => palette.text_subtle,
                    };
                    let status_summary = outcome.summary();
                    let status_resp = ui
                        .scope_builder(
                            egui::UiBuilder::new()
                                .id_salt(egui::Id::new(STAGE_EMBED_BACK_STATUS_AUTHOR_ID)),
                            |ui| ui.colored_label(color, status_summary.clone()),
                        )
                        .inner;
                    let status_author = STAGE_EMBED_BACK_STATUS_AUTHOR_ID.to_owned();
                    ui.ctx()
                        .accesskit_node_builder(status_resp.id, move |node| {
                            node.set_role(accesskit::Role::Label);
                            node.set_author_id(status_author.clone());
                            node.set_label("Stage embed-back status".to_owned());
                            node.set_value(status_summary.clone());
                        });
                }
            })
            .response;

        // The one Stage pane is a GenericContainer in both render paths, so its
        // stable id never changes role based on which operator entry point opened it.
        let author = STAGE_PANE_AUTHOR_ID.to_owned();
        let summary = self
            .route_error
            .clone()
            .unwrap_or_else(|| self.content.summary());
        ui.ctx().accesskit_node_builder(resp.id, move |node| {
            node.set_role(accesskit::Role::GenericContainer);
            node.set_author_id(author.clone());
            node.set_label("Stage".to_owned());
            node.set_value(summary.clone());
        });

        embed_back_pressed
    }
}

fn emit_route_status(ui: &mut egui::Ui, palette: &HsPalette, message: &str) {
    let value = message.to_owned();
    let response = ui
        .scope_builder(
            egui::UiBuilder::new().id_salt(egui::Id::new(STAGE_ROUTE_STATUS_AUTHOR_ID)),
            |ui| ui.colored_label(palette.error_text, &value),
        )
        .inner;
    ui.ctx().accesskit_node_builder(response.id, move |node| {
        node.set_role(accesskit::Role::Status);
        node.set_author_id(STAGE_ROUTE_STATUS_AUTHOR_ID.to_owned());
        node.set_label("Route to Stage status".to_owned());
        node.set_value(value.clone());
    });
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
    previous: StageRouteSnapshot,
    next: StageRouteSnapshot,
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
                .restore_route_snapshot(&previous);
            UndoResult::ok()
        }
        None => UndoResult::pane_dropped(),
    });
    let redo_fn: UndoFn = Arc::new(move || match weak.upgrade() {
        Some(pane) => {
            pane.lock()
                .unwrap_or_else(|e| e.into_inner())
                .restore_route_snapshot(&next);
            UndoResult::ok()
        }
        None => UndoResult::pane_dropped(),
    });
    bus.push_undo_cross_pane(UndoAction::sync(description, undo_fn, redo_fn));
}

/// Emit the Stage pane's Role::GenericContainer AccessKit node, with the staged-content summary as its value so an
/// out-of-process agent reads what is currently on the stage (HBR-SWARM / AC-6).
fn emit_region_node(ui: &egui::Ui, id: egui::Id, author_id: &str, summary: &str) {
    let author = author_id.to_owned();
    let summary = summary.to_owned();
    ui.ctx().accesskit_node_builder(id, move |node| {
        node.set_role(accesskit::Role::GenericContainer);
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
        pane.last_embed_back = Some(EmbedBackOutcome::Embedded {
            artifact_id: "artifact-old".to_owned(),
            sha256: "a".repeat(64),
            target_pane: "pane-old".to_owned(),
        });
        pane.set_content(StageContent::Selection("x".to_owned(), "D".to_owned()));
        assert!(pane.content.is_some());
        assert!(
            pane.last_embed_back.is_none(),
            "a new route must not display the previous route's embed result"
        );
        match &pane.content {
            StageContent::Selection(t, d) => {
                assert_eq!(t, "x");
                assert_eq!(d, "D");
            }
            other => panic!("expected Selection, got {other:?}"),
        }
    }

    #[test]
    fn route_error_clears_every_prior_terminal_embed_status() {
        let outcomes = [
            EmbedBackOutcome::Embedded {
                artifact_id: "artifact-old".to_owned(),
                sha256: "a".repeat(64),
                target_pane: "pane-old".to_owned(),
            },
            EmbedBackOutcome::Failed("old terminal failure".to_owned()),
        ];
        for outcome in outcomes {
            let mut pane = StagePane::new();
            pane.last_embed_back = Some(outcome);
            pane.set_route_error("new route failed");
            assert_eq!(pane.content, StageContent::Empty);
            assert!(pane.last_embed_back.is_none());
            assert_eq!(pane.route_error.as_deref(), Some("new route failed"));
        }
    }

    #[test]
    fn route_busy_clears_prior_causal_and_terminal_embed_status() {
        let mut pane = StagePane::new();
        pane.set_content_correlated(
            StageContent::Selection("old".to_owned(), "doc-old".to_owned()),
            Some("cause-old".to_owned()),
        );
        pane.last_embed_back = Some(EmbedBackOutcome::Embedded {
            artifact_id: "artifact-old".to_owned(),
            sha256: "a".repeat(64),
            target_pane: "pane-old".to_owned(),
        });
        let retry = crate::interop::PendingStageRoute::new(
            StageContent::Selection("new".to_owned(), "doc-new".to_owned()),
            "selection",
            Some("cause-new".to_owned()),
            "pane-new",
            "workspace-1",
        );

        pane.set_route_busy(retry.clone());

        assert_eq!(pane.content, StageContent::Empty);
        assert!(pane.causal_action_id.is_none());
        assert!(pane.last_embed_back.is_none());
        assert_eq!(pane.route_retry, Some(retry));
    }

    #[test]
    fn admitted_route_commit_cannot_discard_a_later_retained_request() {
        let mut pane = StagePane::new();
        let retry = crate::interop::PendingStageRoute::new(
            StageContent::Selection("second".to_owned(), "doc-second".to_owned()),
            "selection",
            Some("causal-second".to_owned()),
            "pane-second",
            "workspace-1",
        );
        pane.set_route_busy(retry.clone());

        pane.set_content_correlated(
            StageContent::Selection("first".to_owned(), "doc-first".to_owned()),
            Some("causal-first".to_owned()),
        );

        assert_eq!(
            pane.content,
            StageContent::Selection("first".to_owned(), "doc-first".to_owned())
        );
        assert_eq!(pane.route_retry, Some(retry));
        assert!(pane
            .route_error
            .as_deref()
            .is_some_and(|error| error.contains("retained")));
    }

    #[test]
    fn contention_and_missing_runtime_are_typed_visible_outcomes() {
        assert!(EmbedBackOutcome::Busy
            .summary()
            .contains("already in progress"));
        assert!(EmbedBackOutcome::RuntimeUnavailable
            .summary()
            .contains("runtime is not running"));
    }
}

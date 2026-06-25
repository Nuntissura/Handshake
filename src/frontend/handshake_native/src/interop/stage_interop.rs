//! Bidirectional Editors <-> Stage (Pillar 17) interop edge (WP-KERNEL-012 MT-066, cluster E10).
//!
//! ## What this is (the FULL Stage round-trip, both legs)
//!
//! MT-033 delivered the ONE-WAY editors -> Stage path: a CKC/Atelier item, a whole document, or a text
//! selection is staged on the MT-031 [`crate::interop::InteractionBus`] and the local Stage pane displays
//! it ([`crate::stage_pane::StagePane`]). This MT proves the FULL round-trip:
//!
//! - **Route leg (editors -> Stage):** [`StageRoutePayload`] + [`build_from_selection`] /
//!   [`build_from_canvas_node`] build the typed route payload deterministically from the MT-031
//!   [`crate::interop::SharedSelection`] (a `TextRange` selection, a `BlockRef`, or a graph/canvas
//!   `NodeRef`). [`route_to_stage`] carries it across panes via the EXISTING MT-033 bus command
//!   ([`crate::interop::CMD_ROUTE_TO_STAGE`] = `interop.route-to-stage`) and emits the MT-036
//!   `route_to_stage` Flight-Recorder event. It does NOT duplicate the MT-033 command id (AC-005 /
//!   MC-003); it EXTENDS it with the Selection + CanvasNode payload builders the one-way path lacked.
//!
//! - **Embed-back leg (Stage capture -> note/canvas):** [`StageClient::fetch_stage_artifact`] GETs a
//!   Stage capture artifact (with its SHA-256 manifest provenance) and
//!   [`embed_artifact_as_nodeview`] converts it into an MT-014 embed NodeView ([`EmbedNodeView`]
//!   wrapping the existing [`crate::rich_editor::document_model::node::HsLinkNode`] `hsLink` atom by
//!   `ref_kind`) so an evidence-grade capture flows back into a note or canvas with its provenance
//!   intact.
//!
//! ## NO Stage backend HTTP routes exist — the route leg is bus-only, the embed-back leg is a typed blocker
//!
//! VERIFIED (KERNEL_BUILDER gate 2026-06-25, grep over `src/backend/handshake_core`): there are NO
//! `/stage/` HTTP routes in handshake_core. Stage (Pillar 17) — like FEMS (Pillar 12) — is a separate
//! system not yet wired into the frozen handshake_core HTTP surface. The MT contract's "POST
//! /workspaces/{id}/stage/route (MT-033 uses it)" is WRONG: MT-033's route-to-Stage is the BUS COMMAND
//! [`crate::interop::CMD_ROUTE_TO_STAGE`] (`interop.route-to-stage`), drained cross-pane by the shell —
//! NOT a backend POST. So:
//!
//! 1. [`route_to_stage`] EXTENDS that bus command (it stages the payload via
//!    [`crate::interop::InteractionBus::request_route_to_stage`] and dispatches the same id). There is
//!    NO backend POST and the routing stays bus-only — exactly what MT-033 does (HARD no-backend-rewrite
//!    rule, RISK-001/MC-001).
//! 2. The embed-back route `GET /workspaces/{id}/stage/artifacts/{id}` is ABSENT, so
//!    [`StageClient::fetch_stage_artifact`] returns [`StageInteropError::EmbedBackEndpointAbsent`] — the
//!    contract's DESIGNED typed blocker (detection is BROAD: 404 / 501 / route-not-registered / a missing
//!    client method, RISK-008/MC-008). NO backend route is added; NO artifact is fabricated.
//!
//! ## SHA-256 manifest provenance MUST survive the embed (evidence-grade integrity)
//!
//! The embed-back NodeView carries `{ source: "stage_capture", artifact_id, sha256, manifest_ref }` so the
//! capture's evidence-grade provenance is durable inside the note/canvas (RISK-002/MC-002, AC-003). If a
//! fetched artifact has NO `sha256` or NO `manifest`, [`embed_artifact_as_nodeview`] returns
//! [`StageInteropError::ProvenanceMissing`] — it REFUSES to embed an unverifiable capture rather than
//! producing a provenance-stripped embed.
//!
//! ## Reuse, no second HTTP stack / no parallel embed type / no new FR event kind
//!
//! - [`StageClient`] holds a cloned [`reqwest::Client`] (the process-wide
//!   [`crate::backend_client::shared_http_client`] pool) + the config-resolved
//!   [`crate::backend_client::BACKEND_BASE_URL`] — the exact
//!   [`crate::fems::memory_client::MemoryClient`] / `KnowledgeDocumentsClient` pattern. NO new reqwest
//!   stack, NO new async runtime (RISK-006/MC-005 sibling).
//! - The embed is the EXISTING MT-014 `hsLink` atom by `ref_kind` (`"stage_capture"`), NOT an invented
//!   node (RISK-004/MC-004).
//! - [`route_to_stage`]'s FR emit reuses the MT-036 `route_to_stage` emitter exactly, via the bus's
//!   [`crate::interop::InteractionBus::route_to_stage`] call site — NO new FR event kind (RISK-005/MC-005).

use std::time::Duration;

use serde::Deserialize;

use crate::backend_client::{
    shared_http_client, BACKEND_BASE_URL, HSK_HEADER_ACTOR_ID, HSK_HEADER_KERNEL_TASK_RUN_ID,
    HSK_HEADER_SESSION_RUN_ID,
};
use crate::interop::SharedSelection;
use crate::pane_registry::PaneId;
use crate::rich_editor::document_model::node::HsLinkNode;

/// The `hsLink` `ref_kind` a Stage capture embeds under (the MT-014 embed-atom discriminator). DISTINCT
/// from the media-render kinds (`images`/`video`/`album`/`slideshow`) and the CKC family
/// (`atelier`/`media`/`character`/`moodboard`) so a Stage capture chip is never routed to the image/video
/// renderer or mistaken for a CKC drop. A valid `ref_kind` string the opaque-JSONB `content_json`
/// round-trips losslessly.
pub const STAGE_CAPTURE_REF_KIND: &str = "stage_capture";

/// The embed-back read timeout (a bounded timeout so a hung backend cannot stall the editor frame loop).
const REQUEST_TIMEOUT: Duration = Duration::from_secs(8);

/// The least-privileged read-only actor id used for the Stage artifact read (no `x-hsk-actor-kind` =>
/// read-only server-side, the same least-privilege default the FEMS/knowledge read paths use).
const STAGE_READ_ACTOR_ID: &str = "native-editor-stage-reader";

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Typed error — EmbedBackEndpointAbsent + ProvenanceMissing are the first-class typed gates.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The typed outcome of any Stage interop operation.
///
/// [`Self::EmbedBackEndpointAbsent`] is the FIRST-CLASS TYPED BLOCKER (RISK-008/MC-008, AC-004): the
/// embed-back read route is absent in this handshake_core build (Stage = Pillar 17, no `/stage/` HTTP
/// surface). It is never swallowed — the Stage pane maps it to a visible empty-state and it is surfaced
/// upward to the WP validator. [`Self::ProvenanceMissing`] guards evidence-grade integrity (RISK-002/
/// MC-002): a fetched artifact with no sha256 / manifest is REFUSED rather than embedded unverifiable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StageInteropError {
    /// A selection could not produce a route payload (e.g. [`SharedSelection::None`], or a stale/closed
    /// source pane — RISK-007/MC-007). The source carries the reason.
    BadSelection(String),
    /// The built payload had no routable content (an empty selection text / empty node id).
    EmptyPayload,
    /// The route was rejected with a non-success status (reserved: the route leg is bus-only today, so
    /// this is produced only if a Stage route backend POST is ever added and rejects).
    RouteRejected { status: u16 },
    /// THE TYPED BLOCKER: the embed-back read route `GET /workspaces/{id}/stage/artifacts/{id}` is absent
    /// in this build (404 / 501 / route-not-registered / a missing client method). Carries the probed path
    /// so the validator sees exactly which route is missing. NO backend route is added; NO artifact is
    /// fabricated (the HARD no-backend-rewrite rule).
    EmbedBackEndpointAbsent { probed_path: String },
    /// A fetched artifact lacked SHA-256 / manifest provenance, so it CANNOT be embedded as an
    /// evidence-grade capture (RISK-002/MC-002). Refuse rather than embed an unverifiable artifact.
    ProvenanceMissing,
    /// A transport / HTTP / decode failure that is NOT the typed endpoint-absent blocker (connect /
    /// timeout / TLS / a non-404/501 status / a decode error). Carries the reason.
    Transport(String),
}

impl std::fmt::Display for StageInteropError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadSelection(why) => write!(f, "stage route: bad selection ({why})"),
            Self::EmptyPayload => write!(f, "stage route: empty payload (nothing to route)"),
            Self::RouteRejected { status } => write!(f, "stage route rejected: HTTP {status}"),
            Self::EmbedBackEndpointAbsent { probed_path } => write!(
                f,
                "Stage embed-back endpoint not present in this build (probed {probed_path})"
            ),
            Self::ProvenanceMissing => write!(
                f,
                "stage embed-back refused: fetched artifact has no SHA-256 / manifest provenance"
            ),
            Self::Transport(e) => write!(f, "stage interop transport error: {e}"),
        }
    }
}

impl std::error::Error for StageInteropError {}

impl StageInteropError {
    /// True when this is the embed-back typed-blocker variant (the Stage pane renders the empty-state
    /// banner and the blocker is surfaced to the WP validator).
    pub fn is_embed_back_endpoint_absent(&self) -> bool {
        matches!(self, StageInteropError::EmbedBackEndpointAbsent { .. })
    }
}

/// A typed result alias for Stage interop operations.
pub type StageResult<T> = Result<T, StageInteropError>;

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Route leg — the editors -> Stage payload builders + the bus-carried route.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The source of a route-to-Stage action. The variant set is the MT-066 contract list; each maps from one
/// MT-031 [`SharedSelection`] variant: `TextRange` -> [`Self::Selection`], `BlockRef` -> [`Self::NoteRef`],
/// `NodeRef` -> [`Self::CanvasNode`] (see [`build_from_selection`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StageRouteSource {
    /// A whole note routed by reference (from a `BlockRef` selection — the block addresses its note).
    NoteRef {
        workspace_id: String,
        note_id: String,
    },
    /// A text selection routed from a code / rich-text surface.
    Selection {
        workspace_id: String,
        source_pane_id: String,
        text: String,
        /// The `loom://`-style source reference (`{pane}:{start}-{end}`) so a consumer can locate the
        /// span back in the source document.
        source_ref: String,
    },
    /// A graph / canvas node routed by reference.
    CanvasNode {
        workspace_id: String,
        canvas_id: String,
        node_id: String,
        node_kind: String,
    },
}

impl StageRouteSource {
    /// The stable content-kind wire string (the MT-036 `route_to_stage` payload `content_kind` field).
    /// Matches the [`crate::stage_pane::StageContent::content_kind`] vocabulary so the FR event the bus
    /// emits and the staged content agree.
    pub fn content_kind(&self) -> &'static str {
        match self {
            StageRouteSource::NoteRef { .. } => "document",
            StageRouteSource::Selection { .. } => "selection",
            StageRouteSource::CanvasNode { .. } => "canvas_node",
        }
    }

    /// The workspace id this source belongs to.
    pub fn workspace_id(&self) -> &str {
        match self {
            StageRouteSource::NoteRef { workspace_id, .. }
            | StageRouteSource::Selection { workspace_id, .. }
            | StageRouteSource::CanvasNode { workspace_id, .. } => workspace_id,
        }
    }
}

/// A reference to a graph / canvas node being routed to Stage (the [`build_from_canvas_node`] input). The
/// native peer of a `SharedSelection::NodeRef` plus the workspace + canvas context the bus selection does
/// not itself carry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasNodeRef {
    /// The workspace the canvas lives in.
    pub workspace_id: String,
    /// The canvas board id.
    pub canvas_id: String,
    /// The node id on the canvas.
    pub node_id: String,
    /// The node kind (e.g. `"loom_block"`, `"image"`) — carried into the route payload for the Stage
    /// surface to render the right node chrome.
    pub node_kind: String,
    /// The pane id of the canvas surface (so liveness can be validated against the pane registry).
    pub pane_id: String,
}

/// The typed route-to-Stage payload. Built deterministically by [`build_from_selection`] /
/// [`build_from_canvas_node`]; carried cross-pane by [`route_to_stage`] over the MT-033 bus command.
/// `correlation_id` ties the route event to any later embed-back so the round-trip is observable in the
/// Flight-Recorder timeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageRoutePayload {
    /// The workspace this route happens in.
    pub workspace_id: String,
    /// What is being routed.
    pub source: StageRouteSource,
    /// The Stage pane the content targets, when a specific Stage pane is known (else the shell
    /// opens/focuses the default Stage pane).
    pub target_stage_pane: Option<PaneId>,
    /// A stable correlation id (derived from the source) so the route and a later embed-back correlate.
    pub correlation_id: String,
}

impl StageRoutePayload {
    /// The content-kind wire string this payload routes (delegates to the source).
    pub fn content_kind(&self) -> &'static str {
        self.source.content_kind()
    }
}

/// Build a [`StageRoutePayload`] deterministically from the MT-031 shared selection (the route-leg input
/// for a text selection or a block reference). Maps:
///
/// - [`SharedSelection::TextRange`] -> [`StageRouteSource::Selection`] (the materialized selected text +
///   a `{pane}:{start}-{end}` source ref).
/// - [`SharedSelection::BlockRef`] -> [`StageRouteSource::NoteRef`] (the block id IS the note reference).
/// - [`SharedSelection::NodeRef`] -> a graph/canvas node is routed via [`build_from_canvas_node`] (it
///   needs the canvas + workspace context the bus selection does not carry), so a `NodeRef` here is a
///   [`StageInteropError::BadSelection`] directing the caller to the canvas builder.
/// - [`SharedSelection::None`] -> [`StageInteropError::BadSelection`] (nothing to route).
///
/// `workspace_id` is the active workspace (the bus selection is workspace-agnostic). RISK-007/MC-007: an
/// empty selection text is rejected as [`StageInteropError::EmptyPayload`] rather than routing an empty
/// payload; the caller validates pane liveness via [`build_from_selection_live`] before routing.
pub fn build_from_selection(
    sel: &SharedSelection,
    workspace_id: &str,
) -> StageResult<StageRoutePayload> {
    match sel {
        SharedSelection::None => {
            Err(StageInteropError::BadSelection("no active selection".to_owned()))
        }
        SharedSelection::TextRange { pane_id, start, end, text, .. } => {
            if text.trim().is_empty() {
                return Err(StageInteropError::EmptyPayload);
            }
            let source_ref = format!("{pane_id}:{start}-{end}");
            let correlation_id = format!("stage-route-sel-{pane_id}-{start}-{end}");
            Ok(StageRoutePayload {
                workspace_id: workspace_id.to_owned(),
                source: StageRouteSource::Selection {
                    workspace_id: workspace_id.to_owned(),
                    source_pane_id: pane_id.as_ref().to_owned(),
                    text: text.clone(),
                    source_ref,
                },
                target_stage_pane: None,
                correlation_id,
            })
        }
        SharedSelection::BlockRef { pane_id, block_id } => {
            if block_id.trim().is_empty() {
                return Err(StageInteropError::EmptyPayload);
            }
            // The block selection's source pane is validated by build_from_selection_live (callers pass
            // the live pane set); here it is unused beyond the liveness path.
            let _ = pane_id;
            let correlation_id = sanitize_id(&format!("stage-route-note-{block_id}"));
            Ok(StageRoutePayload {
                workspace_id: workspace_id.to_owned(),
                source: StageRouteSource::NoteRef {
                    workspace_id: workspace_id.to_owned(),
                    note_id: block_id.clone(),
                },
                target_stage_pane: None,
                correlation_id,
            })
        }
        SharedSelection::NodeRef { .. } => Err(StageInteropError::BadSelection(
            "a graph/canvas node routes via build_from_canvas_node (needs canvas + workspace context)"
                .to_owned(),
        )),
    }
}

/// Like [`build_from_selection`] but FIRST validates that the selection's source pane is still live in the
/// pane registry (RISK-007/MC-007). A stale/closed source pane_id yields [`StageInteropError::BadSelection`]
/// rather than routing a dangling payload. `live_pane_ids` is the current pane-registry id set.
pub fn build_from_selection_live(
    sel: &SharedSelection,
    workspace_id: &str,
    live_pane_ids: &[PaneId],
) -> StageResult<StageRoutePayload> {
    if let Some(pane_id) = sel.pane_id() {
        if !live_pane_ids.iter().any(|p| p == pane_id) {
            return Err(StageInteropError::BadSelection(format!(
                "source pane '{pane_id}' is no longer live (stale/closed selection)"
            )));
        }
    }
    build_from_selection(sel, workspace_id)
}

/// Build a [`StageRoutePayload`] deterministically from a graph/canvas node reference (the route-leg input
/// for a node). Maps a [`CanvasNodeRef`] -> [`StageRouteSource::CanvasNode`]. An empty `node_id` is
/// rejected as [`StageInteropError::EmptyPayload`].
pub fn build_from_canvas_node(node: &CanvasNodeRef) -> StageResult<StageRoutePayload> {
    if node.node_id.trim().is_empty() {
        return Err(StageInteropError::EmptyPayload);
    }
    let correlation_id = sanitize_id(&format!(
        "stage-route-node-{}-{}",
        node.canvas_id, node.node_id
    ));
    Ok(StageRoutePayload {
        workspace_id: node.workspace_id.clone(),
        source: StageRouteSource::CanvasNode {
            workspace_id: node.workspace_id.clone(),
            canvas_id: node.canvas_id.clone(),
            node_id: node.node_id.clone(),
            node_kind: node.node_kind.clone(),
        },
        target_stage_pane: None,
        correlation_id,
    })
}

/// Like [`build_from_canvas_node`] but validates the canvas pane is still live (RISK-007/MC-007).
pub fn build_from_canvas_node_live(
    node: &CanvasNodeRef,
    live_pane_ids: &[PaneId],
) -> StageResult<StageRoutePayload> {
    let live = live_pane_ids.iter().any(|p| p.as_ref() == node.pane_id);
    if !live {
        return Err(StageInteropError::BadSelection(format!(
            "canvas pane '{}' is no longer live (stale/closed node)",
            node.pane_id
        )));
    }
    build_from_canvas_node(node)
}

/// The acknowledgement a successful route produces. The route leg is bus-only (no backend POST), so the
/// ack records that the content was staged + the correlation id, NOT a server response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteAck {
    /// The correlation id of the routed payload (so a later embed-back correlates).
    pub correlation_id: String,
    /// The content kind that was routed (the MT-036 FR `content_kind`).
    pub content_kind: String,
    /// True — the content was staged on the bus (the bus-only route always stages on success).
    pub staged: bool,
}

/// Route `payload` to Stage by EXTENDING the MT-033 bus command. This stages the payload's content as a
/// [`crate::stage_pane::StageContent`] on the MT-031 [`crate::interop::InteractionBus`] and dispatches the
/// EXISTING [`crate::interop::CMD_ROUTE_TO_STAGE`] (`interop.route-to-stage`), which is where the bus emits
/// the MT-036 `route_to_stage` Flight-Recorder event (NO new FR event kind, RISK-005/MC-005). The shell
/// drains the staged content cross-pane to open/focus the Stage pane (the same drain MT-033 uses).
///
/// HARD no-backend-rewrite rule (RISK-001/MC-001): there is NO Stage route backend POST — Stage has no
/// `/stage/` HTTP surface — so the route stays bus-only, exactly as MT-033 does. This MT's NEW
/// contribution is the Selection + CanvasNode payload builders above; this function carries them over the
/// existing bus command without duplicating its id (AC-005/MC-003).
pub fn route_to_stage(
    ctx: &egui::Context,
    bus: &mut crate::interop::InteractionBus,
    payload: &StageRoutePayload,
) -> StageResult<RouteAck> {
    let content = stage_content_for(payload)?;
    // The bus's route_to_stage emits the MT-036 route_to_stage FR event AND dispatches the EXISTING
    // CMD_ROUTE_TO_STAGE (reuse, do not fork — AC-005/MC-003, RISK-005/MC-005).
    let dispatched = bus.route_to_stage(ctx, content);
    Ok(RouteAck {
        correlation_id: payload.correlation_id.clone(),
        content_kind: payload.content_kind().to_owned(),
        staged: dispatched,
    })
}

/// The runtime [`crate::interop::CommandDescriptor`] for the "Embed Stage Capture" command (the embed-back
/// leg, AC-005). The shell's palette dispatch arm registers this on the live MT-031
/// [`crate::interop::InteractionBus`]; the handler requests a repaint so the Stage pane's embed-back render
/// runs next frame (the same stage-then-drain split the MT-033 route-to-stage / MT-032 open-document
/// commands use). The actual fetch + insert is [`crate::stage_pane::StagePane::capture_and_embed_back`];
/// this command id is the addressable swarm trigger. Palette-driven: NO keybind (does not steal a VS Code
/// binding).
pub fn embed_stage_capture_descriptor() -> crate::interop::CommandDescriptor {
    crate::interop::CommandDescriptor {
        id: crate::interop::CMD_EMBED_STAGE_CAPTURE,
        name: "EmbedStageCapture",
        label: "Embed Stage Capture".to_owned(),
        keywords: vec![
            "embed".to_owned(),
            "stage".to_owned(),
            "capture".to_owned(),
            "provenance".to_owned(),
        ],
        keybind: None,
        handler: std::sync::Arc::new(
            |ctx: &egui::Context, _bus: &mut crate::interop::InteractionBus| {
                // The shell's dispatch arm runs the Stage pane embed-back over the live target; this
                // bus-side handler requests a repaint so that render runs. NO direct backend write here.
                ctx.request_repaint();
            },
        ),
    }
}

/// Register the "Embed Stage Capture" command (the embed-back leg) into the WP-011 command registry's
/// runtime command bus, reusing the EXISTING [`crate::interop::InteractionBus`] registration API (NO
/// duplicate registry/bus). Idempotent (last registration wins). This is the WRAP-not-fork registration:
/// the static [`crate::command_registry`] catalog carries the discoverable palette row; this registers the
/// runtime handler on the same bus the other melt-together commands use. The route-to-stage command is
/// REUSED from MT-033 ([`crate::interop::InteractionBus::register_route_to_stage_command`]), NOT duplicated
/// here (AC-005/MC-003).
pub fn register_embed_stage_capture_command(bus: &mut crate::interop::InteractionBus) {
    bus.register_command(embed_stage_capture_descriptor());
}

/// Convert a route payload into the [`crate::stage_pane::StageContent`] the bus stages. A note routes as a
/// `Document` (by id), a selection as a `Selection(text, source)`, a canvas node as a `Selection` carrying
/// a `node://` reference (the Stage pane displays the node reference; full node rendering is the Stage
/// surface's job at E11). An empty source is [`StageInteropError::EmptyPayload`].
fn stage_content_for(payload: &StageRoutePayload) -> StageResult<crate::stage_pane::StageContent> {
    use crate::rich_editor::save::save_manager::RichDocLoad;
    use crate::stage_pane::StageContent;
    match &payload.source {
        StageRouteSource::NoteRef { note_id, .. } => {
            if note_id.trim().is_empty() {
                return Err(StageInteropError::EmptyPayload);
            }
            Ok(StageContent::Document(RichDocLoad {
                rich_document_id: note_id.clone(),
                doc_version: 0,
                title: String::new(),
                content_json: None,
                updated_at: None,
            }))
        }
        StageRouteSource::Selection { text, source_ref, .. } => {
            if text.trim().is_empty() {
                return Err(StageInteropError::EmptyPayload);
            }
            Ok(StageContent::Selection(text.clone(), source_ref.clone()))
        }
        StageRouteSource::CanvasNode { canvas_id, node_id, .. } => {
            if node_id.trim().is_empty() {
                return Err(StageInteropError::EmptyPayload);
            }
            Ok(StageContent::Selection(
                format!("canvas node {node_id}"),
                format!("node://{canvas_id}/{node_id}"),
            ))
        }
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Embed-back leg — the Stage artifact read client + the MT-014 embed conversion.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The SHA-256 manifest descriptor a Stage capture artifact carries. Provenance-first: the `sha256` + the
/// `manifest_ref` are what make the capture evidence-grade. Decoded from the artifact read body.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct StageManifest {
    /// The SHA-256 hash of the captured artifact bytes (lowercase hex). MUST be present for an
    /// evidence-grade embed (else [`StageInteropError::ProvenanceMissing`]).
    #[serde(default)]
    pub sha256: String,
    /// A reference to the full manifest record (a `manifest://...` / artifact-store id). MUST be present.
    #[serde(default)]
    pub manifest_ref: String,
    /// The capture's content type (e.g. `image/png`), surfaced on the embed chip.
    #[serde(default)]
    pub content_type: String,
}

impl StageManifest {
    /// True when the manifest carries BOTH a sha256 AND a manifest_ref (the evidence-grade requirement).
    pub fn is_evidence_grade(&self) -> bool {
        !self.sha256.trim().is_empty() && !self.manifest_ref.trim().is_empty()
    }
}

/// A fetched Stage capture artifact reference (the [`StageClient::fetch_stage_artifact`] output). Carries
/// the artifact id, its workspace, the SHA-256 (hoisted for quick access) and the full [`StageManifest`].
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct StageArtifactRef {
    /// The artifact id (the Stage capture store id).
    pub artifact_id: String,
    /// The workspace the artifact lives in.
    pub workspace_id: String,
    /// The SHA-256 of the artifact bytes (lowercase hex; also inside [`Self::manifest`]).
    #[serde(default)]
    pub sha256: String,
    /// The full manifest provenance descriptor.
    pub manifest: StageManifest,
    /// A display label for the capture (shown on the embed chip; falls back to the artifact id).
    #[serde(default)]
    pub label: String,
}

impl StageArtifactRef {
    /// The display label, falling back to `"stage_capture:{artifact_id}"` when blank.
    pub fn display_label(&self) -> String {
        if self.label.trim().is_empty() {
            format!("{STAGE_CAPTURE_REF_KIND}:{}", self.artifact_id)
        } else {
            self.label.clone()
        }
    }

    /// True when the artifact carries verifiable SHA-256 manifest provenance (the gate for an
    /// evidence-grade embed). Checks the hoisted sha256 AND the manifest's evidence-grade flag.
    pub fn is_evidence_grade(&self) -> bool {
        !self.sha256.trim().is_empty() && self.manifest.is_evidence_grade()
    }
}

/// The provenance descriptor carried inside an embed-back NodeView so SHA-256 manifest provenance is
/// DURABLE in the note/canvas (RISK-002/MC-002, AC-003). Exactly the contract shape
/// `{ source: "stage_capture", artifact_id, sha256, manifest_ref }`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StageEmbedProvenance {
    /// Always `"stage_capture"` (the provenance source discriminator).
    pub source: String,
    /// The Stage artifact id this embed came from.
    pub artifact_id: String,
    /// The SHA-256 manifest hash (lowercase hex) — the evidence anchor.
    pub sha256: String,
    /// The manifest record reference.
    pub manifest_ref: String,
}

impl StageEmbedProvenance {
    /// Build the provenance descriptor for `artifact` (source pinned to `"stage_capture"`).
    pub fn from_artifact(artifact: &StageArtifactRef) -> Self {
        Self {
            source: STAGE_CAPTURE_REF_KIND.to_owned(),
            artifact_id: artifact.artifact_id.clone(),
            sha256: artifact.sha256.clone(),
            manifest_ref: artifact.manifest.manifest_ref.clone(),
        }
    }
}

/// An MT-014 embed NodeView for a Stage capture: the EXISTING [`HsLinkNode`] `hsLink` atom
/// (`ref_kind = "stage_capture"`, `ref_value = artifact_id`) PLUS the [`StageEmbedProvenance`] descriptor
/// that travels with it so the SHA-256 manifest provenance survives the embed (AC-003). This is NOT a
/// parallel embed type (RISK-004/MC-004): the renderable atom IS the MT-014 `hsLink` node; this struct only
/// bundles the provenance the bare atom cannot itself carry, so the host inserts `node` into the document
/// model and persists `provenance` alongside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbedNodeView {
    /// The MT-014 embed atom (`hsLink`, `ref_kind = "stage_capture"`) inserted into the document model.
    pub node: HsLinkNode,
    /// The evidence-grade provenance descriptor (durable SHA-256 manifest provenance).
    pub provenance: StageEmbedProvenance,
}

/// Convert a fetched Stage artifact into an MT-014 embed NodeView (the embed-back conversion, AC-003).
///
/// The embed atom is the EXISTING [`HsLinkNode`] (`ref_kind = "stage_capture"`, `ref_value = artifact_id`,
/// `label = display_label`) — NOT a parallel embed type (RISK-004/MC-004). The SHA-256 manifest provenance
/// MUST survive: if the artifact has no sha256 OR no manifest_ref, this returns
/// [`StageInteropError::ProvenanceMissing`] and REFUSES to embed an unverifiable capture (RISK-002/MC-002).
pub fn embed_artifact_as_nodeview(artifact: &StageArtifactRef) -> StageResult<EmbedNodeView> {
    if !artifact.is_evidence_grade() {
        return Err(StageInteropError::ProvenanceMissing);
    }
    let node = HsLinkNode::new(
        STAGE_CAPTURE_REF_KIND,
        artifact.artifact_id.clone(),
        artifact.display_label(),
    );
    let provenance = StageEmbedProvenance::from_artifact(artifact);
    Ok(EmbedNodeView { node, provenance })
}

/// The stateless typed read client for the Stage capture-artifact route. Holds ONLY a shared
/// [`reqwest::Client`] (the process-wide [`crate::backend_client::shared_http_client`] pool — NO second
/// HTTP stack) + the config-resolved base URL — exactly the [`crate::fems::memory_client::MemoryClient`]
/// pattern. READ-ONLY: it only ever issues a GET.
#[derive(Clone)]
pub struct StageClient {
    client: reqwest::Client,
    base_url: String,
    session_run_id: String,
}

impl Default for StageClient {
    fn default() -> Self {
        Self::production()
    }
}

impl StageClient {
    /// Construct against the production backend base URL (the config-resolved
    /// [`crate::backend_client::BACKEND_BASE_URL`], not hardcoded here), sharing the ONE process-wide
    /// [`crate::backend_client::shared_http_client`] connection pool.
    pub fn production() -> Self {
        Self::with_client(shared_http_client(), BACKEND_BASE_URL)
    }

    /// Construct against an explicit base URL on a FRESH client (used by tests to point at a mock server
    /// with an isolated pool). The base URL is the host authority — never hardcoded at a call site
    /// (GLOBAL-PORTABILITY-004).
    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            session_run_id: "native-editor-session".to_owned(),
        }
    }

    /// Reuse an already-constructed [`reqwest::Client`] (the WP-011 backend client's pool) so the app
    /// shares ONE connection pool rather than minting a second HTTP stack.
    pub fn with_client(client: reqwest::Client, base_url: impl Into<String>) -> Self {
        Self {
            client,
            base_url: base_url.into(),
            session_run_id: "native-editor-session".to_owned(),
        }
    }

    /// Override the session run id on the read identity headers (so swarm/operator co-work is
    /// attributable). Builder-style.
    pub fn with_session_run_id(mut self, session_run_id: impl Into<String>) -> Self {
        self.session_run_id = session_run_id.into();
        self
    }

    /// The artifact read path for a workspace + artifact (the documented Stage embed-back route). Built
    /// here so the [`StageInteropError::EmbedBackEndpointAbsent`] blocker can report the exact probed path.
    pub fn artifact_path(workspace_id: &str, artifact_id: &str) -> String {
        format!("/workspaces/{workspace_id}/stage/artifacts/{artifact_id}")
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    /// Fetch a Stage capture artifact for embed-back. READ-ONLY: a single GET, never a write verb.
    ///
    /// Behavior contract:
    /// - A 404 / 501 (route absent / not implemented) maps to
    ///   [`StageInteropError::EmbedBackEndpointAbsent`] — the TYPED BLOCKER, never a panic or a fabricated
    ///   artifact (RISK-008/MC-008, AC-004). This is the DESIGNED PRIMARY PATH in the current build, where
    ///   the Stage HTTP surface does not exist.
    /// - A success body is decoded into a [`StageArtifactRef`]; a decode failure is
    ///   [`StageInteropError::Transport`].
    /// - Other non-success statuses map to [`StageInteropError::Transport`] (carrying the status); a
    ///   transport failure (connect / timeout / TLS) likewise.
    pub async fn fetch_stage_artifact(
        &self,
        workspace_id: &str,
        artifact_id: &str,
    ) -> StageResult<StageArtifactRef> {
        let path = Self::artifact_path(workspace_id, artifact_id);
        let url = self.url(&path);
        let resp = self
            .client
            .get(&url)
            .timeout(REQUEST_TIMEOUT)
            // READ identity: least-privileged read-only actor (no x-hsk-actor-kind => read-only).
            .header(HSK_HEADER_ACTOR_ID, STAGE_READ_ACTOR_ID)
            .header(
                HSK_HEADER_KERNEL_TASK_RUN_ID,
                format!("native-editor-stage-{workspace_id}"),
            )
            .header(HSK_HEADER_SESSION_RUN_ID, &self.session_run_id)
            .send()
            .await
            .map_err(|e| StageInteropError::Transport(e.to_string()))?;
        let status = resp.status();

        // THE TYPED BLOCKER (BROAD detection — RISK-008/MC-008): 404 (route absent) OR 501 (not
        // implemented) both mean the Stage embed-back route is not present in this build. Surface it as
        // the typed blocker; never panic, never fabricate the artifact.
        if status == reqwest::StatusCode::NOT_FOUND
            || status == reqwest::StatusCode::NOT_IMPLEMENTED
        {
            return Err(StageInteropError::EmbedBackEndpointAbsent { probed_path: path });
        }

        if !status.is_success() {
            let code = status.as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(StageInteropError::Transport(format!("HTTP {code}: {body}")));
        }

        resp.json::<StageArtifactRef>()
            .await
            .map_err(|e| StageInteropError::Transport(format!("decode: {e}")))
    }
}

/// Sanitize an id fragment to `[a-z0-9-]` (collision-resistant, addressable) — reuses the same slug
/// helper the canvas/loom ids use so a free-form note/canvas id yields a safe correlation id.
fn sanitize_id(raw: &str) -> String {
    crate::project_tree::stable_part(raw)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interop::{EditorSurfaceKind, SharedSelection};

    fn pane(id: &str) -> PaneId {
        std::sync::Arc::from(id)
    }

    fn text_range(pane_id: &str, start: usize, end: usize, text: &str) -> SharedSelection {
        SharedSelection::TextRange {
            pane_id: pane(pane_id),
            surface: EditorSurfaceKind::RichText,
            start,
            end,
            text: text.to_owned(),
        }
    }

    fn evidence_artifact(id: &str) -> StageArtifactRef {
        StageArtifactRef {
            artifact_id: id.to_owned(),
            workspace_id: "WS-1".to_owned(),
            sha256: "a".repeat(64),
            manifest: StageManifest {
                sha256: "a".repeat(64),
                manifest_ref: format!("manifest://{id}"),
                content_type: "image/png".to_owned(),
            },
            label: "Capture 1".to_owned(),
        }
    }

    /// AC-001 (selection half): a TextRange selection builds a Selection-source payload with the
    /// materialized text + a `{pane}:{start}-{end}` source ref + a deterministic correlation id.
    #[test]
    fn build_from_text_range_selection() {
        let sel = text_range("pane-rich", 10, 25, "hello stage");
        let payload = build_from_selection(&sel, "WS-1").expect("builds");
        assert_eq!(payload.workspace_id, "WS-1");
        assert_eq!(payload.content_kind(), "selection");
        match &payload.source {
            StageRouteSource::Selection { workspace_id, source_pane_id, text, source_ref } => {
                assert_eq!(workspace_id, "WS-1");
                assert_eq!(source_pane_id, "pane-rich");
                assert_eq!(text, "hello stage");
                assert_eq!(source_ref, "pane-rich:10-25");
            }
            other => panic!("expected Selection, got {other:?}"),
        }
        assert_eq!(payload.correlation_id, "stage-route-sel-pane-rich-10-25");
    }

    /// AC-001 (block half): a BlockRef selection builds a NoteRef-source payload (document content kind).
    #[test]
    fn build_from_block_ref_selection() {
        let sel = SharedSelection::BlockRef { pane_id: pane("pane-rich"), block_id: "BLK-7".to_owned() };
        let payload = build_from_selection(&sel, "WS-1").expect("builds");
        assert_eq!(payload.content_kind(), "document");
        match &payload.source {
            StageRouteSource::NoteRef { workspace_id, note_id } => {
                assert_eq!(workspace_id, "WS-1");
                assert_eq!(note_id, "BLK-7");
            }
            other => panic!("expected NoteRef, got {other:?}"),
        }
    }

    /// AC-001 (canvas half): a CanvasNodeRef builds a CanvasNode-source payload (canvas_node content kind).
    #[test]
    fn build_from_canvas_node_payload() {
        let node = CanvasNodeRef {
            workspace_id: "WS-1".to_owned(),
            canvas_id: "CB-9".to_owned(),
            node_id: "N-3".to_owned(),
            node_kind: "loom_block".to_owned(),
            pane_id: "pane-canvas".to_owned(),
        };
        let payload = build_from_canvas_node(&node).expect("builds");
        assert_eq!(payload.content_kind(), "canvas_node");
        match &payload.source {
            StageRouteSource::CanvasNode { workspace_id, canvas_id, node_id, node_kind } => {
                assert_eq!(workspace_id, "WS-1");
                assert_eq!(canvas_id, "CB-9");
                assert_eq!(node_id, "N-3");
                assert_eq!(node_kind, "loom_block");
            }
            other => panic!("expected CanvasNode, got {other:?}"),
        }
    }

    /// RISK-007/MC-007: a None selection and an empty selection are rejected (no empty/dangling route).
    #[test]
    fn empty_and_none_selection_rejected() {
        assert!(matches!(
            build_from_selection(&SharedSelection::None, "WS-1"),
            Err(StageInteropError::BadSelection(_))
        ));
        let empty = text_range("pane-rich", 0, 0, "   ");
        assert_eq!(build_from_selection(&empty, "WS-1"), Err(StageInteropError::EmptyPayload));
    }

    /// RISK-007/MC-007: a selection whose source pane is no longer live is rejected as BadSelection.
    #[test]
    fn stale_source_pane_rejected() {
        let sel = text_range("pane-gone", 1, 5, "stale");
        // The live set does NOT contain pane-gone.
        let live: Vec<PaneId> = vec![pane("pane-a"), pane("pane-b")];
        let err = build_from_selection_live(&sel, "WS-1", &live).unwrap_err();
        assert!(matches!(err, StageInteropError::BadSelection(_)));
        assert!(err.to_string().contains("pane-gone"));
        // With the pane live, it builds.
        let live_ok: Vec<PaneId> = vec![pane("pane-gone")];
        assert!(build_from_selection_live(&sel, "WS-1", &live_ok).is_ok());
    }

    /// RISK-007/MC-007: a canvas node whose pane is gone is rejected.
    #[test]
    fn stale_canvas_pane_rejected() {
        let node = CanvasNodeRef {
            workspace_id: "WS-1".to_owned(),
            canvas_id: "CB-1".to_owned(),
            node_id: "N-1".to_owned(),
            node_kind: "image".to_owned(),
            pane_id: "pane-canvas-gone".to_owned(),
        };
        let live: Vec<PaneId> = vec![pane("pane-a")];
        assert!(matches!(
            build_from_canvas_node_live(&node, &live),
            Err(StageInteropError::BadSelection(_))
        ));
        let live_ok: Vec<PaneId> = vec![pane("pane-canvas-gone")];
        assert!(build_from_canvas_node_live(&node, &live_ok).is_ok());
    }

    /// AC-003: embed_artifact_as_nodeview produces the MT-014 hsLink atom (ref_kind = "stage_capture")
    /// and carries the SHA-256 manifest provenance descriptor.
    #[test]
    fn embed_back_builds_mt014_hslink_with_provenance() {
        let artifact = evidence_artifact("ART-42");
        let view = embed_artifact_as_nodeview(&artifact).expect("evidence-grade embeds");
        // The embed atom IS the MT-014 hsLink node (NOT a parallel type).
        assert_eq!(view.node.ref_kind, STAGE_CAPTURE_REF_KIND);
        assert_eq!(view.node.ref_kind, "stage_capture");
        assert_eq!(view.node.ref_value, "ART-42");
        assert_eq!(view.node.label, "Capture 1");
        assert!(view.node.resolved);
        // The provenance descriptor matches the contract shape AND the artifact's sha256.
        assert_eq!(view.provenance.source, "stage_capture");
        assert_eq!(view.provenance.artifact_id, "ART-42");
        assert_eq!(view.provenance.sha256, "a".repeat(64));
        assert_eq!(view.provenance.manifest_ref, "manifest://ART-42");
        // The provenance round-trips as JSON (durable inside content_json).
        let json = serde_json::to_string(&view.provenance).unwrap();
        let back: StageEmbedProvenance = serde_json::from_str(&json).unwrap();
        assert_eq!(back, view.provenance);
    }

    /// RISK-002/MC-002: an artifact with NO sha256 or NO manifest_ref is REFUSED (ProvenanceMissing),
    /// never embedded as an unverifiable capture.
    #[test]
    fn embed_back_refuses_unverifiable_artifact() {
        // No sha256.
        let mut no_hash = evidence_artifact("ART-1");
        no_hash.sha256 = String::new();
        no_hash.manifest.sha256 = String::new();
        assert_eq!(embed_artifact_as_nodeview(&no_hash), Err(StageInteropError::ProvenanceMissing));
        // No manifest_ref.
        let mut no_manifest = evidence_artifact("ART-2");
        no_manifest.manifest.manifest_ref = String::new();
        assert_eq!(
            embed_artifact_as_nodeview(&no_manifest),
            Err(StageInteropError::ProvenanceMissing)
        );
    }

    /// AC-004: the EmbedBackEndpointAbsent variant is the typed blocker; its Display names the probed path.
    #[test]
    fn embed_back_endpoint_absent_is_typed_blocker() {
        let err = StageInteropError::EmbedBackEndpointAbsent {
            probed_path: "/workspaces/WS-1/stage/artifacts/ART-1".into(),
        };
        assert!(err.is_embed_back_endpoint_absent());
        assert!(!StageInteropError::ProvenanceMissing.is_embed_back_endpoint_absent());
        assert!(err.to_string().contains("/workspaces/WS-1/stage/artifacts/ART-1"));
    }

    /// The artifact read path is the documented Stage embed-back route shape.
    #[test]
    fn artifact_path_is_documented_route() {
        assert_eq!(
            StageClient::artifact_path("WS-1", "ART-9"),
            "/workspaces/WS-1/stage/artifacts/ART-9"
        );
    }

    /// The StageArtifactRef decodes from the documented body shape (the wire the read returns) and
    /// surfaces the evidence-grade flag.
    #[test]
    fn artifact_decodes_and_is_evidence_grade() {
        let body = serde_json::json!({
            "artifact_id": "ART-7",
            "workspace_id": "WS-1",
            "sha256": "b".repeat(64),
            "manifest": {"sha256": "b".repeat(64), "manifest_ref": "manifest://ART-7", "content_type": "image/png"},
            "label": "Render 7"
        });
        let artifact: StageArtifactRef = serde_json::from_value(body).expect("decodes");
        assert_eq!(artifact.artifact_id, "ART-7");
        assert!(artifact.is_evidence_grade());
        assert_eq!(artifact.display_label(), "Render 7");
        // A blank label falls back to the stage_capture:{id} form.
        let mut blank = artifact.clone();
        blank.label = String::new();
        assert_eq!(blank.display_label(), "stage_capture:ART-7");
    }
}

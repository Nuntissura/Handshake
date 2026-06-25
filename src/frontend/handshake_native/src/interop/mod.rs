//! Cross-surface interconnection substrate (WP-KERNEL-012 cluster E5 — melt-together).
//!
//! This subtree houses the app-wide primitives that let the four editor surfaces (code, rich-text,
//! graph, canvas) act as one tool rather than four stitched-on apps:
//!
//! - [`interaction_bus`] — the [`interaction_bus::InteractionBus`]: one [`interaction_bus::SharedSelection`]
//!   model, one mockable clipboard, and one cross-pane [`interaction_bus::CommandBus`] every pane shares
//!   through egui app data. It WRAPS the existing WP-011 substrate (`command_registry`, `event_bus`,
//!   `command_palette`) — it does not fork it (MT-031).
//! - [`adapters`] — the thin per-pane glue: the standard melt-together command set each pane registers,
//!   the selection/clipboard builders each pane uses, and the cross-pane command-surface AccessKit
//!   emitter (the `command-palette-trigger` / `command-palette-search` / `cmd-{id}` nodes the contract
//!   names) so a swarm agent can drive the shared command surface by stable id.
//!
//! Later E5 MTs (MT-032..MT-035: everything-is-a-block addressing, CKC embeds, code<->note refs,
//! unified undo, the Flight-Recorder event ledger) build ON this substrate rather than re-inventing
//! selection/clipboard/command state per surface.

pub mod adapters;
// WP-KERNEL-012 MT-033 (E5 — CKC embeds / drag-in + route-to-Stage): the one typed cross-surface drag
// payload (CKC/Atelier item, Loom block, or plain text) every editor surface stages/reads through egui's
// DragAndDrop channel, plus the CKC `hsLink` ref-kind family the dropped embed atom round-trips
// (`content_json`). The embed is an EXISTING `hsLink` atom by ref_kind (MT-014 lesson), NOT an invented
// node; the canvas-add resolves to a loom block id (MT-026 placement), NOT an unsupported field.
pub mod drag_payload;
pub mod interaction_bus;
// WP-KERNEL-012 MT-034 (E5 — code<->note cross-references): the bidirectional resolution service. A
// `[[code:path#Symbol]]` reference in a note is the EXISTING `hsLink` atom (ref_kind="code"); clicking
// it dispatches `open-code-symbol` on the MT-031 bus (routed via the MT-030 ShellNavigator seam), and
// `resolve_code_ref` turns the symbol entity id into a file+line target via the existing code-nav
// backend. The reverse direction (`find_notes_referencing_symbol`) lists notes mentioning a symbol via
// the verified loom search-v2 route, feeding the code pane's NoteRefsPanel.
pub mod cross_ref;
// WP-KERNEL-012 MT-066 (E10 — Stage/Pillar 17 interop): the bidirectional editors <-> Stage round-trip.
// The route leg EXTENDS the MT-033 `interop.route-to-stage` bus command with Selection + CanvasNode
// payload builders (Stage has NO `/stage/` backend HTTP routes, so routing stays bus-only). The
// embed-back leg fetches a Stage capture artifact (with its SHA-256 manifest provenance) and converts it
// to an MT-014 `hsLink` embed atom; the absent embed-back route is the typed blocker
// `StageInteropError::EmbedBackEndpointAbsent` (no backend route added).
pub mod stage_interop;
// WP-KERNEL-012 MT-067 (E10 — Calendar/Pillar 2 interop): the editors <-> Calendar edge. The daily-note
// half is REAL — `open_or_create_daily_note` DELEGATES to the MT-019 daily-note service (idempotent,
// single doc/date). The calendar-event + activity-spans halves are TYPED BLOCKERS: handshake_core has NO
// `/calendar/` HTTP routes in this build, so both reads return `InteropError::EndpointUnavailable` (the
// designed empty-state path) — no backend route added, no event/span fabricated, no DB/SQLite touched.
pub mod calendar_interop;

pub use interaction_bus::{
    command_list_item_author_id, default_keybind_for, interaction_bus_id, ClipboardPayload, CommandBus,
    CommandDescriptor, CommandHandler, EditorSurfaceKind, InteractionBus, SharedSelection, CMD_COPY,
    CMD_CUT, CMD_EMBED_STAGE_CAPTURE, CMD_FIND, CMD_OPEN_CODE_SYMBOL, CMD_OPEN_DOCUMENT, CMD_PASTE,
    CMD_REDO, CMD_ROUTE_TO_STAGE,
    CMD_SELECT_ALL, CMD_COMMAND_PALETTE, CMD_UNDO, CMD_UNDO_CROSS_PANE, COMMAND_LIST_ITEM_AUTHOR_PREFIX,
    COMMAND_PALETTE_SEARCH_AUTHOR_ID, COMMAND_PALETTE_TRIGGER_AUTHOR_ID, INTERACTION_BUS_KEY,
};

pub use adapters::{
    register_standard_commands, surface_clipboard_payload, surface_command_ids, text_range_selection,
    CommandPaletteSurface,
};

pub use drag_payload::{
    AtelierItemKind, AtelierRef, DragPayload, LoomBlockRef, ATELIER_EMBED_REF_KINDS,
};

pub use cross_ref::{
    dispatch_code_ref_open, find_notes_referencing_symbol, find_notes_with, percent_encode_symbol,
    resolve_code_ref, resolve_code_ref_with, CodeRef, CrossRefError, FindNotesHttp, FindNotesSearch,
    NoteRef, SymbolDwellTracker, CODE_REF_KIND, NOTE_REFS_DWELL_MS, NOTE_REFS_SEARCH_LIMIT,
};

pub use stage_interop::{
    build_from_canvas_node, build_from_canvas_node_live, build_from_selection,
    build_from_selection_live, embed_artifact_as_nodeview, embed_stage_capture_descriptor,
    register_embed_stage_capture_command, route_to_stage, CanvasNodeRef, EmbedNodeView, RouteAck,
    StageArtifactRef, StageClient, StageEmbedProvenance, StageInteropError, StageManifest,
    StageRoutePayload, StageRouteSource, STAGE_CAPTURE_REF_KIND,
};

pub use calendar_interop::{
    pick_event_for_date, ActivitySpan, CalendarEvent, CalendarInteropService, DailyNoteBinding, DocId,
    InteropError, InteropResult, CMD_FOCUS_CALENDAR_EVENT, CMD_OPEN_DAILY_NOTE_FOR_DATE,
    CMD_OPEN_DOCUMENT as CMD_OPEN_ACTIVITY_DOCUMENT,
};

// ── MT-035 (E5 unified undo) — the per-pane "Undo ({n})" title-bar indicator ──────────────────────────

/// AccessKit author_id PREFIX for a pane's undo-count indicator: `undo-count-{pane_id}` (the MT-035
/// contract's exact id shape). The full id is built by [`undo_count_author_id`].
pub const UNDO_COUNT_AUTHOR_PREFIX: &str = "undo-count-";

/// The stable AccessKit author_id for one pane's undo-count indicator (`undo-count-{pane_id}`). The
/// `pane_id` is sanitized to `[a-z0-9-]` (the same `project_tree::stable_part` slug the canvas placement
/// + loom node ids use) so an arbitrary pane id yields a safe, collision-resistant address.
pub fn undo_count_author_id(pane_id: &str) -> String {
    format!("{UNDO_COUNT_AUTHOR_PREFIX}{}", crate::project_tree::stable_part(pane_id))
}

/// Render the "Undo ({n})" indicator into a pane title bar (AC-6). `count` is the focused pane's local
/// undo-ring length (from [`InteractionBus::local_undo_count`]). The text color is the
/// [`crate::theme::HsPalette::text_subtle`] semantic token — NO `Color32` literal (the no-hardcode
/// invariant; the indicator tracks dark/light like every other token). Emits a `Role::Label` AccessKit
/// node addressed `undo-count-{pane_id}` carrying the count string as its value, so an out-of-process
/// swarm agent can READ the undo depth by stable id (HBR-SWARM). `Role::Label` is the field-correct
/// accesskit role for the contract's `StaticText` (the documented MT-003/MT-007 role-deviation pattern).
pub fn render_undo_count_indicator(
    ui: &mut egui::Ui,
    pane_id: &str,
    count: usize,
    palette: &crate::theme::HsPalette,
) -> egui::Response {
    let label_text = format!("Undo ({count})");
    let resp = ui.label(egui::RichText::new(&label_text).color(palette.text_subtle));
    let author_id = undo_count_author_id(pane_id);
    let value = label_text.clone();
    ui.ctx().accesskit_node_builder(resp.id, move |node| {
        node.set_role(egui::accesskit::Role::Label);
        node.set_author_id(author_id.clone());
        node.set_label("Undo count".to_owned());
        node.set_value(value.clone());
    });
    resp
}

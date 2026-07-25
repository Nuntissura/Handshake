//! WP-KERNEL-012 MT-036 (E5 — one event ledger across surfaces).
//!
//! The [`NativeEditorEventEmitter`] is the single melt-together producer that turns a native editor
//! action (document save, undo fired, route-to-stage, code edit, embed/cross-ref, canvas node placed)
//! into a typed [`NativeEditorEvent`] and ships it to the EXISTING handshake_core Flight Recorder
//! observability ledger. It is the HBR-SWARM/HBR-INT observability seam: without it a swarm agent or the
//! operator cannot see what the native editors are doing.
//!
//! ## Runtime path
//!
//! The producer posts the closed `hsk.native_editor@0.1` envelope to the verified
//! `POST /api/flight_recorder/native_editor_event` authority route. The backend records an
//! `editor_edit` Flight Recorder event with the supplied actor, pane, workspace, action and payload,
//! and mirrors the same event id into PostgreSQL EventLedger. Dispatch stays bounded and off-frame;
//! failures remain visible in the cap-20 error ring.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use serde_json::{json, Value as JsonValue};

/// The closed schema version every native editor event carries (the MT contract's exact string). It
/// namespaces the native-editor payload so a consumer can distinguish it from other ledger producers.
pub const NATIVE_EDITOR_SCHEMA_VERSION: &str = "hsk.native_editor@0.1";

/// Work packet attribution included on every EventLedger mirror receipt.
pub const NATIVE_EDITOR_WORK_PACKET_ID: &str = "WP-KERNEL-012";

/// The cap on the in-memory error ring (RISK-2 / MC-2): a bounded buffer of the most recent emit
/// failures, surfaced by the [`crate::flight_recorder_pane::FlightRecorderPane`] so a no-context model
/// sees WHY the ledger looks empty rather than silent loss.
pub const ERROR_RING_CAP: usize = 20;

/// Capacity of the single ordered dispatch queue. The frame thread only performs `try_send`; one
/// off-frame worker awaits each POST before taking the next event, preserving producer order while
/// bounding memory under a rapid edit burst.
pub const EMIT_PERMITS: usize = 20;

/// The default native-editor actor id when no operator/model session is active. A DESCRIPTIVE but valid
/// id (RISK-5 / MC-5): `hsk:native_editor:{pane_id}` is built per-event via [`native_editor_actor_id`];
/// this fallback is used when a pane id is unknown.
pub const DEFAULT_ACTOR_ID: &str = "native_editor_human";

/// Allocate the durable participant actor id for one native-editor host.
///
/// A fresh UUID prevents attribution overlap between split/headless/desktop hosts both within and
/// across Handshake processes. The host stores this once for its lifetime. Callers may still replace
/// it pre-mount through the explicit host configuration surface.
pub fn new_native_editor_host_actor_id() -> String {
    format!("hsk:native_editor:host:{}", uuid::Uuid::new_v4())
}

/// The structured native-editor action kind. Maps 1:1 to the backend's closed native-editor vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeEditorAction {
    /// A rich document was canonically saved (rich-text SAVE path — LIVE wired, MT-020).
    DocumentSaved,
    /// A debounced batch of code edits landed (code pane — DEFERRED to E11/MT-069 live wiring).
    CodeEdit,
    /// An embed atom was inserted into a note (LIVE: slash-confirm and Atelier drag-in).
    EmbedCreated,
    /// A node was placed on the canvas (canvas live placement — DEFERRED to E11/MT-069 live wiring).
    CanvasNodePlaced,
    /// A code/note cross-reference was inserted (LIVE: code-symbol selection).
    CrossRefInserted,
    /// An undo or redo fired (rich-pane undo dispatch — LIVE wired, MT-035).
    UndoFired,
    /// Content was routed to the Stage pane (route-to-stage command — LIVE wired, MT-033).
    RouteToStage,
    /// A provenance-verified Stage artifact was inserted back into an editor target (MT-066).
    StageEmbedBack,
    /// A daily note was bound to the Calendar event for its date (MT-067).
    CalendarEventBound,
    /// A read-only Calendar activity span was correlated to edited documents (MT-067).
    ActivitySpanCorrelated,
    /// A Locus URI resolved to a persisted work packet or microtask (MT-068).
    LocusRefResolved,
    /// Persisted documents referencing a Locus URI were returned (MT-068).
    LocusReverseLookup,
}

impl NativeEditorAction {
    /// The stable snake_case wire string (the value in the payload's `action` field).
    pub fn as_str(self) -> &'static str {
        match self {
            NativeEditorAction::DocumentSaved => "document_saved",
            NativeEditorAction::CodeEdit => "code_edit",
            NativeEditorAction::EmbedCreated => "embed_created",
            NativeEditorAction::CanvasNodePlaced => "canvas_node_placed",
            NativeEditorAction::CrossRefInserted => "cross_ref_inserted",
            NativeEditorAction::UndoFired => "undo_fired",
            NativeEditorAction::RouteToStage => "route_to_stage",
            NativeEditorAction::StageEmbedBack => "stage_embed_back",
            NativeEditorAction::CalendarEventBound => "calendar_event_bound",
            NativeEditorAction::ActivitySpanCorrelated => "activity_span_correlated",
            NativeEditorAction::LocusRefResolved => "locus_ref_resolved",
            NativeEditorAction::LocusReverseLookup => "locus_reverse_lookup",
        }
    }
}

/// One native editor event: the typed melt-together record a surface emits. `payload` carries the
/// action-specific fields (document_id/content_hash, file_path/line_delta, embed_kind/item_id, …) the MT
/// contract names; the common identity (`schema_version` / `action` / `pane_id` / `actor_id` /
/// `workspace_id`) is hoisted to typed fields so a consumer needs no payload re-parse for routing.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NativeEditorEvent {
    /// Stable id generated synchronously at producer time and retained across transport retries.
    pub event_id: String,
    /// Stable producer timestamp retained across transport retries.
    pub ts_utc: String,
    /// Always [`NATIVE_EDITOR_SCHEMA_VERSION`] (`hsk.native_editor@0.1`).
    pub schema_version: String,
    /// The structured action kind.
    pub action: NativeEditorAction,
    /// The id of the pane that emitted the event (the editor surface instance).
    pub pane_id: String,
    /// The acting operator / model session id, formatted `hsk:native_editor:{pane_id}` (RISK-5 / MC-5).
    pub actor_id: String,
    /// The active workspace id (so a consumer can scope events per workspace).
    pub workspace_id: String,
    /// The action-specific structured payload (the MT contract's per-action field set).
    pub payload: JsonValue,
}

impl NativeEditorEvent {
    /// A canonical document-save receipt already binds actor identity in the immutable EventLedger
    /// row. The native emitter must preserve that actor so the backend can authenticate the causal
    /// receipt; all other event kinds remain attributed by the emitter/session authority.
    fn has_authenticated_save_actor(&self) -> bool {
        self.action == NativeEditorAction::DocumentSaved
            && !self.actor_id.trim().is_empty()
            && self
                .payload
                .get("save_receipt_event_id")
                .and_then(JsonValue::as_str)
                .is_some_and(|receipt| !receipt.trim().is_empty())
    }

    /// The base constructor: a `hsk.native_editor@0.1` event for `action` from `pane_id` in
    /// `workspace_id`, acting as `actor_id`, carrying `payload`.
    pub fn new(
        action: NativeEditorAction,
        pane_id: impl Into<String>,
        actor_id: impl Into<String>,
        workspace_id: impl Into<String>,
        payload: JsonValue,
    ) -> Self {
        Self {
            event_id: uuid::Uuid::new_v4().to_string(),
            ts_utc: chrono::Utc::now().to_rfc3339(),
            schema_version: NATIVE_EDITOR_SCHEMA_VERSION.to_owned(),
            action,
            pane_id: pane_id.into(),
            actor_id: actor_id.into(),
            workspace_id: workspace_id.into(),
            payload,
        }
    }

    /// `document_saved`: a rich document was saved. Payload: `{ document_id, content_hash }`.
    pub fn document_saved(
        document_id: impl Into<String>,
        content_hash: impl Into<String>,
        pane_id: impl Into<String>,
        actor_id: impl Into<String>,
        workspace_id: impl Into<String>,
    ) -> Self {
        let payload = json!({
            "document_id": document_id.into(),
            "content_hash": content_hash.into(),
        });
        Self::new(
            NativeEditorAction::DocumentSaved,
            pane_id,
            actor_id,
            workspace_id,
            payload,
        )
    }

    /// Authenticated `document_saved` emitted only after the canonical save returns its immutable
    /// EventLedger receipt and request attribution. The native event keeps its own UUID; the receipt is
    /// a separately verified causal reference.
    pub fn document_saved_with_receipt(
        document_id: impl Into<String>,
        content_hash: impl Into<String>,
        save_receipt_event_id: impl Into<String>,
        attribution: &crate::rich_editor::save::save_manager::SaveAttribution,
        pane_id: impl Into<String>,
        workspace_id: impl Into<String>,
    ) -> Self {
        let payload = json!({
            "document_id": document_id.into(),
            "content_hash": content_hash.into(),
            "save_receipt_event_id": save_receipt_event_id.into(),
            "actor_kind": attribution.actor_kind.clone(),
            "kernel_task_run_id": attribution.kernel_task_run_id.clone(),
            "session_run_id": attribution.session_run_id.clone(),
            "correlation_id": attribution.correlation_id.clone(),
        });
        Self::new(
            NativeEditorAction::DocumentSaved,
            pane_id,
            attribution.actor_id.clone(),
            workspace_id,
            payload,
        )
    }

    /// `code_edit`: a debounced batch of code edits landed. Payload: `{ file_path, line_delta }`.
    pub fn code_edit(
        file_path: impl Into<String>,
        line_delta: i64,
        pane_id: impl Into<String>,
        actor_id: impl Into<String>,
        workspace_id: impl Into<String>,
    ) -> Self {
        let payload = json!({
            "file_path": file_path.into(),
            "line_delta": line_delta,
        });
        Self::new(
            NativeEditorAction::CodeEdit,
            pane_id,
            actor_id,
            workspace_id,
            payload,
        )
    }

    /// `embed_created`: an embed atom was inserted into a note. Payload:
    /// `{ embed_kind, item_id, target_document_id }`.
    pub fn embed_created(
        embed_kind: impl Into<String>,
        item_id: impl Into<String>,
        target_document_id: impl Into<String>,
        pane_id: impl Into<String>,
        actor_id: impl Into<String>,
        workspace_id: impl Into<String>,
    ) -> Self {
        let payload = json!({
            "embed_kind": embed_kind.into(),
            "item_id": item_id.into(),
            "target_document_id": target_document_id.into(),
        });
        Self::new(
            NativeEditorAction::EmbedCreated,
            pane_id,
            actor_id,
            workspace_id,
            payload,
        )
    }

    /// `canvas_node_placed`: a node was placed on the canvas. Payload:
    /// `{ canvas_id, node_id, node_kind }`.
    pub fn canvas_node_placed(
        canvas_id: impl Into<String>,
        node_id: impl Into<String>,
        node_kind: impl Into<String>,
        pane_id: impl Into<String>,
        actor_id: impl Into<String>,
        workspace_id: impl Into<String>,
    ) -> Self {
        let payload = json!({
            "canvas_id": canvas_id.into(),
            "node_id": node_id.into(),
            "node_kind": node_kind.into(),
        });
        Self::new(
            NativeEditorAction::CanvasNodePlaced,
            pane_id,
            actor_id,
            workspace_id,
            payload,
        )
    }

    /// `cross_ref_inserted`: a code/note cross-reference was inserted. Payload:
    /// `{ ref_kind, symbol_entity_id, target_document_id }`.
    pub fn cross_ref_inserted(
        ref_kind: impl Into<String>,
        symbol_entity_id: impl Into<String>,
        target_document_id: impl Into<String>,
        pane_id: impl Into<String>,
        actor_id: impl Into<String>,
        workspace_id: impl Into<String>,
    ) -> Self {
        let payload = json!({
            "ref_kind": ref_kind.into(),
            "symbol_entity_id": symbol_entity_id.into(),
            "target_document_id": target_document_id.into(),
        });
        Self::new(
            NativeEditorAction::CrossRefInserted,
            pane_id,
            actor_id,
            workspace_id,
            payload,
        )
    }

    /// `undo_fired`: an undo/redo fired. Payload: `{ scope }` where `scope` ∈ {"local","cross_pane"}.
    pub fn undo_fired(
        scope: UndoScope,
        pane_id: impl Into<String>,
        actor_id: impl Into<String>,
        workspace_id: impl Into<String>,
    ) -> Self {
        let payload = json!({ "scope": scope.as_str() });
        Self::new(
            NativeEditorAction::UndoFired,
            pane_id,
            actor_id,
            workspace_id,
            payload,
        )
    }

    /// `route_to_stage`: content was routed to a Stage pane. Payload:
    /// `{ content_kind }` (the `source_pane_id` is the typed `pane_id`).
    pub fn route_to_stage(
        content_kind: impl Into<String>,
        source_pane_id: impl Into<String>,
        actor_id: impl Into<String>,
        workspace_id: impl Into<String>,
    ) -> Self {
        let payload = json!({ "content_kind": content_kind.into() });
        Self::new(
            NativeEditorAction::RouteToStage,
            source_pane_id,
            actor_id,
            workspace_id,
            payload,
        )
    }

    /// Correlated Stage route receipt. `causal_action_id` is the immutable StageRoutePayload identity
    /// inherited by the later embed-back receipt.
    pub fn route_to_stage_correlated(
        content_kind: impl Into<String>,
        source_pane_id: impl Into<String>,
        causal_action_id: impl Into<String>,
        actor_id: impl Into<String>,
        workspace_id: impl Into<String>,
    ) -> Self {
        let payload = json!({
            "content_kind": content_kind.into(),
            "causal_action_id": causal_action_id.into(),
        });
        Self::new(
            NativeEditorAction::RouteToStage,
            source_pane_id,
            actor_id,
            workspace_id,
            payload,
        )
    }

    pub fn stage_embed_back(
        artifact_id: impl Into<String>,
        target_pane_id: impl Into<String>,
        sha256: impl Into<String>,
        manifest_ref: impl Into<String>,
        actor_id: impl Into<String>,
        workspace_id: impl Into<String>,
    ) -> Self {
        let target_pane_id = target_pane_id.into();
        Self::new(
            NativeEditorAction::StageEmbedBack,
            target_pane_id.clone(),
            actor_id,
            workspace_id,
            json!({
                "artifact_id": artifact_id.into(),
                "target_pane_id": target_pane_id,
                "sha256": sha256.into(),
                "manifest_ref": manifest_ref.into(),
            }),
        )
    }

    pub fn stage_embed_back_correlated(
        artifact_id: impl Into<String>,
        target_pane_id: impl Into<String>,
        sha256: impl Into<String>,
        manifest_ref: impl Into<String>,
        causal_action_id: impl Into<String>,
        actor_id: impl Into<String>,
        workspace_id: impl Into<String>,
    ) -> Self {
        let target_pane_id = target_pane_id.into();
        Self::new(
            NativeEditorAction::StageEmbedBack,
            target_pane_id.clone(),
            actor_id,
            workspace_id,
            json!({
                "artifact_id": artifact_id.into(),
                "target_pane_id": target_pane_id,
                "sha256": sha256.into(),
                "manifest_ref": manifest_ref.into(),
                "causal_action_id": causal_action_id.into(),
            }),
        )
    }

    pub fn calendar_event_bound(
        date: impl Into<String>,
        document_id: impl Into<String>,
        calendar_event_id: impl Into<String>,
        pane_id: impl Into<String>,
        actor_id: impl Into<String>,
        workspace_id: impl Into<String>,
    ) -> Self {
        Self::new(
            NativeEditorAction::CalendarEventBound,
            pane_id,
            actor_id,
            workspace_id,
            json!({
                "date": date.into(),
                "document_id": document_id.into(),
                "calendar_event_id": calendar_event_id.into(),
            }),
        )
    }

    pub fn activity_span_correlated(
        calendar_event_id: impl Into<String>,
        activity_span_id: impl Into<String>,
        edited_document_ids: Vec<String>,
        pane_id: impl Into<String>,
        actor_id: impl Into<String>,
        workspace_id: impl Into<String>,
    ) -> Self {
        Self::new(
            NativeEditorAction::ActivitySpanCorrelated,
            pane_id,
            actor_id,
            workspace_id,
            json!({
                "calendar_event_id": calendar_event_id.into(),
                "activity_span_id": activity_span_id.into(),
                "edited_document_ids": edited_document_ids,
            }),
        )
    }

    pub fn locus_ref_resolved(
        locus_uri: impl Into<String>,
        target_kind: impl Into<String>,
        target_id: impl Into<String>,
        pane_id: impl Into<String>,
        actor_id: impl Into<String>,
        workspace_id: impl Into<String>,
    ) -> Self {
        Self::new(
            NativeEditorAction::LocusRefResolved,
            pane_id,
            actor_id,
            workspace_id,
            json!({
                "locus_uri": locus_uri.into(),
                "target_kind": target_kind.into(),
                "target_id": target_id.into(),
            }),
        )
    }

    pub fn locus_reverse_lookup(
        locus_uri: impl Into<String>,
        document_ids: Vec<String>,
        pane_id: impl Into<String>,
        actor_id: impl Into<String>,
        workspace_id: impl Into<String>,
    ) -> Self {
        Self::new(
            NativeEditorAction::LocusReverseLookup,
            pane_id,
            actor_id,
            workspace_id,
            json!({
                "locus_uri": locus_uri.into(),
                "document_ids": document_ids,
            }),
        )
    }

    /// The full native-editor payload as a self-contained JSON object (typed identity fields hoisted in
    /// alongside the action-specific payload). This is what a consumer reads to reconstruct the event and
    /// what the production transport nests under the ledger event.
    pub fn to_native_payload(&self) -> JsonValue {
        json!({
            "schema": self.schema_version,
            "action": self.action.as_str(),
            "pane_id": self.pane_id,
            "actor_id": self.actor_id,
            "workspace_id": self.workspace_id,
            "payload": self.payload,
        })
    }
}

/// The undo scope a `undo_fired` event records (the MT contract's `"local" | "cross_pane"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UndoScope {
    /// A focused-pane local undo/redo (POLICY-1).
    Local,
    /// A cross-pane undo/redo (POLICY-2).
    CrossPane,
}

impl UndoScope {
    /// The wire string.
    pub fn as_str(self) -> &'static str {
        match self {
            UndoScope::Local => "local",
            UndoScope::CrossPane => "cross_pane",
        }
    }
}

/// Build a valid, descriptive native-editor actor id (RISK-5 / MC-5): `hsk:native_editor:{pane_id}`.
/// The backend `record_event` only requires a non-empty `actor_id` string (verified in
/// `flight_recorder/mod.rs` — `actor_id must be present`), so this colon-namespaced format is accepted;
/// it is descriptive enough for a consumer to filter native-editor events by an `actor`/`surface` query
/// once the backend ingestion endpoint exists. `pane_id` is left as-is (it is already a safe slug).
pub fn native_editor_actor_id(pane_id: &str) -> String {
    if pane_id.trim().is_empty() {
        DEFAULT_ACTOR_ID.to_owned()
    } else {
        format!("hsk:native_editor:{pane_id}")
    }
}

/// Why an emit failed (recorded in the error ring; never panics the frame).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmitError {
    /// The bounded permit pool was saturated (RISK-2 / MC-2): the event was DROPPED rather than spawning
    /// an unbounded task. Carries the action that was dropped.
    Backpressure(String),
    /// No tokio runtime was installed (headless): the emit could not be dispatched. Honest, not faked.
    NoRuntime(String),
    /// The ordered worker exited; unlike saturation this is permanent and must not be frame-retried.
    WorkerClosed(String),
    /// An event captured in one workspace reached an emitter bound to another workspace. The event is
    /// rejected rather than silently rewritten to the newer workspace.
    WorkspaceMismatch { event: String, emitter: String },
    /// The bounded frame-retry queue evicted an event. Carries immutable identity for operator-visible
    /// diagnosis in the shared error ring.
    PendingOverflow {
        event_id: String,
        workspace_id: String,
    },
    /// The transport POST failed (backend unreachable / non-2xx). Carries the reason.
    Transport(String),
    /// The ordered worker did not return a persistence receipt inside the caller's hard bound. The
    /// event may have been queued, but persistence is unknown and must not be reported as success.
    PersistenceTimeout { event_id: String, timeout_ms: u64 },
}

impl std::fmt::Display for EmitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmitError::Backpressure(a) => write!(f, "emit backpressure (permits saturated): {a}"),
            EmitError::NoRuntime(a) => write!(f, "no tokio runtime for emit: {a}"),
            EmitError::WorkerClosed(a) => write!(f, "native-editor emit worker closed: {a}"),
            EmitError::WorkspaceMismatch { event, emitter } => write!(
                f,
                "native-editor event workspace mismatch: event={event}, emitter={emitter}"
            ),
            EmitError::PendingOverflow {
                event_id,
                workspace_id,
            } => write!(
                f,
                "pending frame-event queue overflow: event_id={event_id}, workspace={workspace_id}"
            ),
            EmitError::Transport(r) => write!(f, "emit transport failure: {r}"),
            EmitError::PersistenceTimeout {
                event_id,
                timeout_ms,
            } => write!(
                f,
                "native-editor persistence receipt timed out: event_id={event_id}, timeout_ms={timeout_ms}"
            ),
        }
    }
}

/// One recorded entry in the bounded error ring (the FlightRecorderPane surfaces these).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmitErrorEntry {
    /// The action that failed to emit.
    pub action: String,
    /// The failure reason.
    pub error: EmitError,
}

/// A bounded, thread-safe ring of the most recent emit failures (cap [`ERROR_RING_CAP`]). Shared between
/// the emitter (which writes from off-frame tasks + the frame thread) and the FlightRecorderPane (which
/// reads on the frame thread).
#[derive(Debug, Default, Clone)]
pub struct ErrorRing {
    inner: Arc<Mutex<VecDeque<EmitErrorEntry>>>,
    /// Event ids whose transient queue-saturation error is already represented in `inner`. Frame
    /// retries reuse an event id, so recording every retry would evict unrelated failures from the
    /// operator-visible ring. This companion queue is bounded by the same cap.
    backpressure_event_ids: Arc<Mutex<VecDeque<String>>>,
}

impl ErrorRing {
    /// A fresh empty ring.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(VecDeque::with_capacity(ERROR_RING_CAP))),
            backpressure_event_ids: Arc::new(Mutex::new(VecDeque::with_capacity(ERROR_RING_CAP))),
        }
    }

    /// Push a failure, evicting the oldest entry once the cap is reached (bounded — never unbounded).
    pub fn push(&self, entry: EmitErrorEntry) {
        if let Ok(mut q) = self.inner.lock() {
            if q.len() >= ERROR_RING_CAP {
                q.pop_front();
            }
            q.push_back(entry);
        }
    }

    /// Record at most one transient backpressure entry for one immutable event id. Returns `true`
    /// when a new ring entry was written and `false` when a retry was coalesced.
    fn push_backpressure_once(&self, event_id: &str, entry: EmitErrorEntry) -> bool {
        let mut ids = match self.backpressure_event_ids.lock() {
            Ok(ids) => ids,
            Err(_) => {
                // A poisoned dedupe lock must not suppress the underlying diagnostic.
                self.push(entry);
                return true;
            }
        };
        if ids.iter().any(|known| known == event_id) {
            return false;
        }
        if ids.len() >= ERROR_RING_CAP {
            ids.pop_front();
        }
        ids.push_back(event_id.to_owned());
        drop(ids);
        self.push(entry);
        true
    }

    /// Clear transient retry state once the event is accepted or permanently abandoned.
    fn clear_backpressure(&self, event_id: &str) {
        if let Ok(mut ids) = self.backpressure_event_ids.lock() {
            ids.retain(|known| known != event_id);
        }
    }

    /// The current entries, oldest-first (a snapshot the pane renders).
    pub fn entries(&self) -> Vec<EmitErrorEntry> {
        self.inner
            .lock()
            .map(|q| q.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// How many failures are currently held.
    pub fn len(&self) -> usize {
        self.inner.lock().map(|q| q.len()).unwrap_or(0)
    }

    /// True when no failure is held.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

fn pending_frame_events_id() -> egui::Id {
    egui::Id::new("handshake_native_editor_pending_frame_events")
}

fn frame_error_ring_id() -> egui::Id {
    egui::Id::new("handshake_native_editor_shared_error_ring")
}

#[derive(Clone)]
struct PendingFrameEvent {
    event: NativeEditorEvent,
    generation_validity: Option<(Arc<std::sync::atomic::AtomicU64>, u64)>,
}

impl PendingFrameEvent {
    fn unguarded(event: NativeEditorEvent) -> Self {
        Self {
            event,
            generation_validity: None,
        }
    }

    fn guarded(
        event: NativeEditorEvent,
        current_generation: Arc<std::sync::atomic::AtomicU64>,
        expected_generation: u64,
    ) -> Self {
        Self {
            event,
            generation_validity: Some((current_generation, expected_generation)),
        }
    }

    fn is_current(&self) -> bool {
        self.generation_validity
            .as_ref()
            .is_none_or(|(current, expected)| {
                current.load(std::sync::atomic::Ordering::Acquire) == *expected
            })
    }
}

/// Install the app/session-wide error ring used by every emitter generation and by frame-queue
/// overflow reporting. Rebinding a workspace never detaches failures from the visible pane.
pub fn install_frame_error_ring(ctx: &egui::Context, ring: ErrorRing) {
    ctx.data_mut(|data| data.insert_temp(frame_error_ring_id(), ring));
}

fn queue_pending_frame_event(ctx: &egui::Context, pending_event: PendingFrameEvent) {
    ctx.data_mut(|data| {
        let id = pending_frame_events_id();
        let mut pending = data
            .get_temp::<VecDeque<PendingFrameEvent>>(id)
            .unwrap_or_default();
        pending.retain(PendingFrameEvent::is_current);
        if pending
            .iter()
            .any(|queued| queued.event.event_id == pending_event.event.event_id)
        {
            data.insert_temp(id, pending);
            return;
        }
        if pending.len() >= EMIT_PERMITS {
            // Preserve the causal prefix already promised by the FIFO. The incoming event is the one
            // that cannot be retained; evicting the oldest would create an invisible history hole.
            if let Some(ring) = data.get_temp::<ErrorRing>(frame_error_ring_id()) {
                ring.clear_backpressure(&pending_event.event.event_id);
                ring.push(EmitErrorEntry {
                    action: pending_event.event.action.as_str().to_owned(),
                    error: EmitError::PendingOverflow {
                        event_id: pending_event.event.event_id,
                        workspace_id: pending_event.event.workspace_id,
                    },
                });
            }
            data.insert_temp(id, pending);
            return;
        }
        pending.push_back(pending_event);
        data.insert_temp(id, pending);
    });
}

/// Retry bridge for one-shot frame-thread completions (notably rich-save outcomes). A transient
/// InteractionBus lock miss or bounded-emitter backpressure must not consume the only success receipt.
/// The queue is session-local, bounded, and retried in causal order on later editor frames.
pub fn dispatch_event_from_frame(ctx: &egui::Context, event: NativeEditorEvent) -> bool {
    dispatch_pending_event_from_frame(ctx, PendingFrameEvent::unguarded(event))
}

/// Generation-aware frame dispatch used by asynchronous calendar receipts. If navigation supersedes
/// the originating request while the event waits behind a contended bus or emitter queue, the stale
/// receipt is discarded before it can reach Flight Recorder.
pub fn dispatch_generation_event_from_frame(
    ctx: &egui::Context,
    event: NativeEditorEvent,
    current_generation: Arc<std::sync::atomic::AtomicU64>,
    expected_generation: u64,
) -> bool {
    dispatch_pending_event_from_frame(
        ctx,
        PendingFrameEvent::guarded(event, current_generation, expected_generation),
    )
}

fn dispatch_pending_event_from_frame(
    ctx: &egui::Context,
    pending_event: PendingFrameEvent,
) -> bool {
    if !pending_event.is_current() {
        return true;
    }
    if !flush_pending_frame_events(ctx) {
        // An older event is still blocked. Append this event behind it without a second bus attempt;
        // otherwise contention could clear between attempts and allow the newer event to overtake.
        queue_pending_frame_event(ctx, pending_event);
        return false;
    }
    if !pending_event.is_current() {
        return true;
    }
    let event = pending_event.event.clone();
    let bus = crate::interop::interaction_bus::InteractionBus::get_or_init(ctx);
    let outcome = crate::interop::interaction_bus::InteractionBus::with_try_lock(&bus, |b| {
        b.emit_event_result(event.clone())
    });
    if matches!(outcome, Some(Ok(()))) {
        return true;
    }
    if outcome.is_none() || matches!(outcome, Some(Err(EmitError::Backpressure(_)))) {
        queue_pending_frame_event(ctx, pending_event);
    }
    false
}

/// Retry queued frame events without blocking. Stops at the first unavailable dispatch so order is
/// retained across contention/backpressure.
pub fn flush_pending_frame_events(ctx: &egui::Context) -> bool {
    let id = pending_frame_events_id();
    let mut pending = ctx.data_mut(|data| {
        let queued = data
            .get_temp::<VecDeque<PendingFrameEvent>>(id)
            .unwrap_or_default();
        data.remove::<VecDeque<PendingFrameEvent>>(id);
        queued
    });
    if pending.is_empty() {
        return true;
    }
    let bus = crate::interop::interaction_bus::InteractionBus::get_or_init(ctx);
    while let Some(pending_event) = pending.pop_front() {
        if !pending_event.is_current() {
            continue;
        }
        let event = pending_event.event.clone();
        let outcome = crate::interop::interaction_bus::InteractionBus::with_try_lock(&bus, |b| {
            b.emit_event_result(event.clone())
        });
        match outcome {
            Some(Ok(())) => {}
            None | Some(Err(EmitError::Backpressure(_))) => {
                pending.push_front(pending_event);
                break;
            }
            Some(Err(_permanent)) => {
                // The emitter already recorded the permanent failure in the shared ring. Drop this
                // retry entry so a closed worker or workspace mismatch cannot spin forever.
            }
        }
    }
    if !pending.is_empty() {
        ctx.data_mut(|data| data.insert_temp(id, pending));
        false
    } else {
        true
    }
}

/// The async transport that ships a native editor event to the ledger. A `Send + Sync` trait keeps the
/// production HTTP transport and deterministic test transports interchangeable.
pub trait EventLedgerTransport: Send + Sync {
    /// Build the exact wire body for `event` (verified-shape, unit-asserted) WITHOUT performing IO. Kept
    /// separate from [`Self::post`] so a unit test can assert every required field + snake_case key
    /// (RISK-1 / MC-1) without a runtime or a live backend.
    fn build_post_body(&self, event: &NativeEditorEvent) -> JsonValue;

    /// Perform the POST. Returns `Ok(())` on a 2xx, else an [`EmitError::Transport`] with the reason.
    /// Async because the production transport is reqwest over the app runtime.
    fn post(
        &self,
        event: NativeEditorEvent,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), EmitError>> + Send>>;
}

/// Production transport for the verified native-editor Flight Recorder ingestion route. The historical
/// type name is retained for source compatibility with dependent MTs; it now carries the native-editor
/// envelope and never routes native actions through Runtime Chat.
#[derive(Clone)]
pub struct RuntimeChatLedgerTransport {
    client: reqwest::Client,
    base_url: String,
    /// A stable, valid non-nil UUID used as the `session_id` the backend requires (it rejects a nil or
    /// non-UUID session id with 400). One per emitter session so every native-editor emit is attributable
    /// to the same Flight Recorder trace.
    session_id: String,
    /// Flight Recorder actor lane for this app/model session.
    actor_kind: String,
}

impl RuntimeChatLedgerTransport {
    /// Build a transport against `base_url` (e.g. [`crate::backend_client::BACKEND_BASE_URL`]) with a
    /// fresh per-session UUID `session_id`.
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: crate::backend_client::shared_http_client(),
            base_url: base_url.into(),
            session_id: uuid::Uuid::new_v4().to_string(),
            actor_kind: "human".to_owned(),
        }
    }

    /// Build a transport with an explicit `session_id` (tests / a shared trace id).
    pub fn with_session_id(base_url: impl Into<String>, session_id: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            session_id: session_id.into(),
            actor_kind: "human".to_owned(),
        }
    }

    /// Build a transport with the exact current app/model session identity.
    pub fn with_identity(
        base_url: impl Into<String>,
        session_id: impl Into<String>,
        actor_kind: impl Into<String>,
    ) -> Self {
        Self {
            client: crate::backend_client::shared_http_client(),
            base_url: base_url.into(),
            session_id: session_id.into(),
            actor_kind: actor_kind.into(),
        }
    }

    fn url(&self) -> String {
        format!("{}/api/flight_recorder/native_editor_event", self.base_url)
    }

    /// The session id this transport stamps on every event (a non-nil UUID string).
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
}

impl EventLedgerTransport for RuntimeChatLedgerTransport {
    fn build_post_body(&self, event: &NativeEditorEvent) -> JsonValue {
        json!({
            "schema_version": NATIVE_EDITOR_SCHEMA_VERSION,
            "event_id": event.event_id,
            "ts_utc": event.ts_utc,
            "kind": event.action.as_str(),
            "actor_id": event.actor_id,
            "actor_kind": self.actor_kind,
            "pane_id": event.pane_id,
            "surface": event.pane_id,
            "workspace_id": event.workspace_id,
            "session_id": self.session_id,
            "work_packet_id": NATIVE_EDITOR_WORK_PACKET_ID,
            "payload": event.payload,
        })
    }

    fn post(
        &self,
        event: NativeEditorEvent,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), EmitError>> + Send>> {
        let client = self.client.clone();
        let url = self.url();
        let body = self.build_post_body(&event);
        Box::pin(async move {
            let resp = client
                .post(&url)
                .json(&body)
                .send()
                .await
                .map_err(|e| EmitError::Transport(format!("network: {e}")))?;
            let status = resp.status();
            if status.is_success() {
                Ok(())
            } else {
                let text = resp.text().await.unwrap_or_default();
                Err(EmitError::Transport(format!("status {status}: {text}")))
            }
        })
    }
}

/// The single native-editor event producer. Holds the active `workspace_id`, the resolved `actor_id`, a
/// transport, the app's tokio runtime handle (so emit runs OFF the egui frame thread — HBR-QUIET), a
/// bounded ordered queue (RISK-2 / MC-2), and the [`ErrorRing`] failures surface.
#[derive(Clone)]
pub struct NativeEditorEventEmitter {
    /// The active workspace id stamped on every event.
    workspace_id: String,
    /// The resolved actor id (e.g. `hsk:native_editor:human`); per-event the pane-scoped id is used.
    actor_id: String,
    /// The ledger transport (production HTTP or a unit mock).
    transport: Arc<dyn EventLedgerTransport>,
    /// Sender for the single ordered off-frame worker. `None` headless.
    sender: Option<tokio::sync::mpsc::Sender<QueuedNativeEditorEvent>>,
    /// The bounded failures ring the FlightRecorderPane surfaces.
    error_ring: ErrorRing,
}

struct QueuedNativeEditorEvent {
    event: NativeEditorEvent,
    completion: Option<tokio::sync::oneshot::Sender<Result<(), EmitError>>>,
}

impl NativeEditorEventEmitter {
    /// Build an emitter for `workspace_id` over `transport`, dispatching on `runtime` (when present),
    /// bounded by [`EMIT_PERMITS`] concurrent emits. `actor_id` defaults to [`DEFAULT_ACTOR_ID`].
    pub fn new(
        workspace_id: impl Into<String>,
        transport: Arc<dyn EventLedgerTransport>,
        runtime: Option<tokio::runtime::Handle>,
    ) -> Self {
        Self::new_with_error_ring(workspace_id, transport, runtime, ErrorRing::new())
    }

    /// Build an emitter over an app/session-wide ring. Workspace/identity rebinding must preserve this
    /// ring so failures from an older ordered worker remain visible while it drains.
    pub fn new_with_error_ring(
        workspace_id: impl Into<String>,
        transport: Arc<dyn EventLedgerTransport>,
        runtime: Option<tokio::runtime::Handle>,
        error_ring: ErrorRing,
    ) -> Self {
        let sender = runtime.as_ref().map(|rt| {
            let (sender, mut receiver) =
                tokio::sync::mpsc::channel::<QueuedNativeEditorEvent>(EMIT_PERMITS);
            let worker_transport = Arc::clone(&transport);
            let worker_ring = error_ring.clone();
            rt.spawn(async move {
                while let Some(queued) = receiver.recv().await {
                    let QueuedNativeEditorEvent { event, completion } = queued;
                    let action = event.action.as_str().to_owned();
                    let mut last_error = None;
                    for attempt in 0..3u64 {
                        match worker_transport.post(event.clone()).await {
                            Ok(()) => {
                                last_error = None;
                                break;
                            }
                            Err(error) => {
                                last_error = Some(error);
                                if attempt < 2 {
                                    tokio::time::sleep(std::time::Duration::from_millis(
                                        25 * (attempt + 1),
                                    ))
                                    .await;
                                }
                            }
                        }
                    }
                    if let Some(error) = last_error {
                        tracing::warn!(action = %action, event_id = %event.event_id, error = %error, "MT-036 native editor event emit failed after bounded retries");
                        worker_ring.push(EmitErrorEntry {
                            action,
                            error: error.clone(),
                        });
                        if let Some(completion) = completion {
                            let _ = completion.send(Err(error));
                        }
                    } else if let Some(completion) = completion {
                        let _ = completion.send(Ok(()));
                    }
                }
            });
            sender
        });
        Self {
            workspace_id: workspace_id.into(),
            actor_id: DEFAULT_ACTOR_ID.to_owned(),
            transport,
            sender,
            error_ring,
        }
    }

    /// The production emitter over the native-editor Flight Recorder route, on `runtime`.
    pub fn production(
        workspace_id: impl Into<String>,
        base_url: impl Into<String>,
        runtime: tokio::runtime::Handle,
    ) -> Self {
        let transport = Arc::new(RuntimeChatLedgerTransport::new(base_url));
        Self::new(workspace_id, transport, Some(runtime))
    }

    /// Production emitter sharing the app/session error ring across workspace generations.
    pub fn production_with_error_ring(
        workspace_id: impl Into<String>,
        base_url: impl Into<String>,
        runtime: tokio::runtime::Handle,
        error_ring: ErrorRing,
    ) -> Self {
        let transport = Arc::new(RuntimeChatLedgerTransport::new(base_url));
        Self::new_with_error_ring(workspace_id, transport, Some(runtime), error_ring)
    }

    /// Production emitter attributed to the exact current app/model session.
    pub fn production_with_identity(
        workspace_id: impl Into<String>,
        base_url: impl Into<String>,
        runtime: tokio::runtime::Handle,
        actor_id: impl Into<String>,
        session_id: impl Into<String>,
        actor_kind: impl Into<String>,
    ) -> Self {
        let actor_id = actor_id.into();
        let transport = Arc::new(RuntimeChatLedgerTransport::with_identity(
            base_url, session_id, actor_kind,
        ));
        let mut emitter = Self::new(workspace_id, transport, Some(runtime));
        emitter.actor_id = actor_id;
        emitter
    }

    /// Identity-aware production emitter sharing the app/session error ring.
    #[allow(clippy::too_many_arguments)]
    pub fn production_with_identity_and_error_ring(
        workspace_id: impl Into<String>,
        base_url: impl Into<String>,
        runtime: tokio::runtime::Handle,
        actor_id: impl Into<String>,
        session_id: impl Into<String>,
        actor_kind: impl Into<String>,
        error_ring: ErrorRing,
    ) -> Self {
        let actor_id = actor_id.into();
        let transport = Arc::new(RuntimeChatLedgerTransport::with_identity(
            base_url, session_id, actor_kind,
        ));
        let mut emitter =
            Self::new_with_error_ring(workspace_id, transport, Some(runtime), error_ring);
        emitter.actor_id = actor_id;
        emitter
    }

    /// Override the resolved actor id (e.g. the live operator / model session id from app state).
    pub fn set_actor_id(&mut self, actor_id: impl Into<String>) {
        self.actor_id = actor_id.into();
    }

    /// The active workspace id.
    pub fn workspace_id(&self) -> &str {
        &self.workspace_id
    }

    /// The resolved actor id.
    pub fn actor_id(&self) -> &str {
        &self.actor_id
    }

    /// The shared error ring (the FlightRecorderPane reads this).
    pub fn error_ring(&self) -> &ErrorRing {
        &self.error_ring
    }

    /// The number of currently-available emit permits (tests / diagnostics).
    pub fn available_permits(&self) -> usize {
        self.sender
            .as_ref()
            .map(tokio::sync::mpsc::Sender::capacity)
            .unwrap_or(EMIT_PERMITS)
    }

    /// Build the wire body for `event` via the transport (delegates to
    /// [`EventLedgerTransport::build_post_body`]). Exposed so a unit test asserts the shape without IO.
    pub fn build_post_body(&self, event: &NativeEditorEvent) -> JsonValue {
        self.transport.build_post_body(event)
    }

    /// Emit `event`: NON-BLOCKING from the egui frame thread (RISK-2 / MC-2). Tries to acquire a permit;
    /// if none is free the event is DROPPED into the error ring (bounded — never an unbounded spawn
    /// queue). With a permit + a runtime, the POST is spawned off-frame and the permit is held until it
    /// resolves; a transport failure is logged to the error ring. With NO runtime (headless), records a
    /// `NoRuntime` failure (honest, never faked). Returns `Ok(())` when the emit was dispatched, or the
    /// [`EmitError`] when it was dropped/blocked so a caller (and a unit test) can assert the outcome.
    pub fn emit_accepted(
        &self,
        mut event: NativeEditorEvent,
    ) -> Result<NativeEditorEvent, EmitError> {
        let action = event.action.as_str().to_owned();
        let Some(sender) = &self.sender else {
            let err = EmitError::NoRuntime(action.clone());
            self.error_ring.push(EmitErrorEntry {
                action,
                error: err.clone(),
            });
            return Err(err);
        };
        if event.workspace_id.trim().is_empty() {
            event.workspace_id = self.workspace_id.clone();
        } else if event.workspace_id != self.workspace_id {
            let err = EmitError::WorkspaceMismatch {
                event: event.workspace_id,
                emitter: self.workspace_id.clone(),
            };
            self.error_ring.push(EmitErrorEntry {
                action,
                error: err.clone(),
            });
            return Err(err);
        }
        // Actor identity is normally emitter/session authority. An authenticated document save is the
        // one exception: its immutable canonical receipt already binds actor identity, and replacing it
        // here makes the backend's receipt validator correctly reject the native mirror.
        if !event.has_authenticated_save_actor() {
            event.actor_id = self.actor_id.clone();
        }
        match sender.try_send(QueuedNativeEditorEvent {
            event: event.clone(),
            completion: None,
        }) {
            Ok(()) => {
                self.error_ring.clear_backpressure(&event.event_id);
                Ok(event)
            }
            Err(tokio::sync::mpsc::error::TrySendError::Full(rejected)) => {
                let err = EmitError::Backpressure(action.clone());
                self.error_ring.push_backpressure_once(
                    &rejected.event.event_id,
                    EmitErrorEntry {
                        action,
                        error: err.clone(),
                    },
                );
                Err(err)
            }
            Err(tokio::sync::mpsc::error::TrySendError::Closed(rejected)) => {
                self.error_ring.clear_backpressure(&rejected.event.event_id);
                let err = EmitError::WorkerClosed(action.clone());
                self.error_ring.push(EmitErrorEntry {
                    action,
                    error: err.clone(),
                });
                Err(err)
            }
        }
    }

    /// Emit and discard the accepted canonical event. Callers that notify extension observers should
    /// use [`Self::emit_accepted`] so the callback sees the exact actor/workspace identity queued for
    /// persistence rather than the caller's pre-authority draft.
    pub fn emit(&self, event: NativeEditorEvent) -> Result<(), EmitError> {
        self.emit_accepted(event).map(|_| ())
    }

    /// Queue `event` and wait for the ordered worker's final transport result. This is for workflows
    /// whose terminal UI state must distinguish queue acceptance from EventLedger persistence. The
    /// caller supplies a hard bound; timeout never upgrades an unknown persistence result to success.
    pub async fn emit_persisted(
        &self,
        mut event: NativeEditorEvent,
        timeout: std::time::Duration,
    ) -> Result<NativeEditorEvent, EmitError> {
        let action = event.action.as_str().to_owned();
        let Some(sender) = &self.sender else {
            let error = EmitError::NoRuntime(action.clone());
            self.error_ring.push(EmitErrorEntry {
                action,
                error: error.clone(),
            });
            return Err(error);
        };
        if event.workspace_id.trim().is_empty() {
            event.workspace_id = self.workspace_id.clone();
        } else if event.workspace_id != self.workspace_id {
            let error = EmitError::WorkspaceMismatch {
                event: event.workspace_id,
                emitter: self.workspace_id.clone(),
            };
            self.error_ring.push(EmitErrorEntry {
                action,
                error: error.clone(),
            });
            return Err(error);
        }
        if !event.has_authenticated_save_actor() {
            event.actor_id = self.actor_id.clone();
        }
        let (completion, delivered) = tokio::sync::oneshot::channel();
        match sender.try_send(QueuedNativeEditorEvent {
            event: event.clone(),
            completion: Some(completion),
        }) {
            Ok(()) => self.error_ring.clear_backpressure(&event.event_id),
            Err(tokio::sync::mpsc::error::TrySendError::Full(rejected)) => {
                let error = EmitError::Backpressure(action.clone());
                self.error_ring.push_backpressure_once(
                    &rejected.event.event_id,
                    EmitErrorEntry {
                        action,
                        error: error.clone(),
                    },
                );
                return Err(error);
            }
            Err(tokio::sync::mpsc::error::TrySendError::Closed(rejected)) => {
                self.error_ring.clear_backpressure(&rejected.event.event_id);
                let error = EmitError::WorkerClosed(action.clone());
                self.error_ring.push(EmitErrorEntry {
                    action,
                    error: error.clone(),
                });
                return Err(error);
            }
        }
        match tokio::time::timeout(timeout, delivered).await {
            Ok(Ok(Ok(()))) => Ok(event),
            Ok(Ok(Err(error))) => Err(error),
            Ok(Err(_)) => Err(EmitError::WorkerClosed(action)),
            Err(_) => Err(EmitError::PersistenceTimeout {
                event_id: event.event_id.clone(),
                timeout_ms: timeout.as_millis().min(u64::MAX as u128) as u64,
            }),
        }
    }

    // ── Convenience helpers (each unit-testable standalone via build_post_body) ───────────────────────

    /// Emit `document_saved` (LIVE wired at the rich-text SAVE success path — MT-020).
    pub fn emit_document_saved(
        &self,
        document_id: impl Into<String>,
        content_hash: impl Into<String>,
        pane_id: impl AsRef<str>,
    ) -> Result<(), EmitError> {
        let pane = pane_id.as_ref();
        self.emit(NativeEditorEvent::document_saved(
            document_id,
            content_hash,
            pane,
            native_editor_actor_id(pane),
            self.workspace_id.clone(),
        ))
    }

    /// Emit `code_edit` (DEFERRED live wiring — the helper is unit-proven; the code pane mounts at
    /// E11/MT-069 and calls this one-liner after a 2s debounced edit batch).
    pub fn emit_code_edit(
        &self,
        file_path: impl Into<String>,
        line_delta: i64,
        pane_id: impl AsRef<str>,
    ) -> Result<(), EmitError> {
        let pane = pane_id.as_ref();
        self.emit(NativeEditorEvent::code_edit(
            file_path,
            line_delta,
            pane,
            native_editor_actor_id(pane),
            self.workspace_id.clone(),
        ))
    }

    /// Emit `embed_created` after a successful transactional embed insertion.
    pub fn emit_embed_created(
        &self,
        embed_kind: impl Into<String>,
        item_id: impl Into<String>,
        target_document_id: impl Into<String>,
        pane_id: impl AsRef<str>,
    ) -> Result<(), EmitError> {
        let pane = pane_id.as_ref();
        self.emit(NativeEditorEvent::embed_created(
            embed_kind,
            item_id,
            target_document_id,
            pane,
            native_editor_actor_id(pane),
            self.workspace_id.clone(),
        ))
    }

    /// Emit `canvas_node_placed` (DEFERRED live wiring — the canvas live-placement path mounts at
    /// E11/MT-069; the helper is unit-proven now).
    pub fn emit_canvas_node_placed(
        &self,
        canvas_id: impl Into<String>,
        node_id: impl Into<String>,
        node_kind: impl Into<String>,
        pane_id: impl AsRef<str>,
    ) -> Result<(), EmitError> {
        let pane = pane_id.as_ref();
        self.emit(NativeEditorEvent::canvas_node_placed(
            canvas_id,
            node_id,
            node_kind,
            pane,
            native_editor_actor_id(pane),
            self.workspace_id.clone(),
        ))
    }

    /// Emit `cross_ref_inserted` after a successful transactional cross-reference insertion.
    pub fn emit_cross_ref_inserted(
        &self,
        ref_kind: impl Into<String>,
        symbol_entity_id: impl Into<String>,
        target_document_id: impl Into<String>,
        pane_id: impl AsRef<str>,
    ) -> Result<(), EmitError> {
        let pane = pane_id.as_ref();
        self.emit(NativeEditorEvent::cross_ref_inserted(
            ref_kind,
            symbol_entity_id,
            target_document_id,
            pane,
            native_editor_actor_id(pane),
            self.workspace_id.clone(),
        ))
    }

    /// Emit `undo_fired` (LIVE wired at the rich-pane undo dispatch — MT-035).
    pub fn emit_undo_fired(
        &self,
        scope: UndoScope,
        pane_id: impl AsRef<str>,
    ) -> Result<(), EmitError> {
        let pane = pane_id.as_ref();
        self.emit(NativeEditorEvent::undo_fired(
            scope,
            pane,
            native_editor_actor_id(pane),
            self.workspace_id.clone(),
        ))
    }

    /// Emit `route_to_stage` (LIVE wired at the MT-033 route-to-stage command).
    pub fn emit_route_to_stage(
        &self,
        content_kind: impl Into<String>,
        source_pane_id: impl AsRef<str>,
    ) -> Result<(), EmitError> {
        let pane = source_pane_id.as_ref();
        self.emit(NativeEditorEvent::route_to_stage(
            content_kind,
            pane,
            native_editor_actor_id(pane),
            self.workspace_id.clone(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An in-memory mock transport (a headless test NEVER touches the network). Records the bodies it
    /// would post and lets a test toggle a forced failure (the failed-POST → error-ring proof).
    struct MockTransport {
        posted: Arc<Mutex<Vec<JsonValue>>>,
        fail: bool,
    }
    impl MockTransport {
        fn new(fail: bool) -> Self {
            Self {
                posted: Arc::new(Mutex::new(Vec::new())),
                fail,
            }
        }
    }
    impl EventLedgerTransport for MockTransport {
        fn build_post_body(&self, event: &NativeEditorEvent) -> JsonValue {
            // The mock mirrors the production shape so the body-shape unit test can run against it too.
            RuntimeChatLedgerTransport::with_session_id(
                "http://test",
                uuid::Uuid::new_v4().to_string(),
            )
            .build_post_body(event)
        }
        fn post(
            &self,
            event: NativeEditorEvent,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), EmitError>> + Send>>
        {
            let posted = Arc::clone(&self.posted);
            let fail = self.fail;
            let body = self.build_post_body(&event);
            Box::pin(async move {
                if fail {
                    Err(EmitError::Transport("forced".to_owned()))
                } else {
                    posted.lock().unwrap().push(body);
                    Ok(())
                }
            })
        }
    }

    #[test]
    fn document_saved_serializes_to_native_schema() {
        let ev = NativeEditorEvent::document_saved(
            "DOC-1",
            "a".repeat(64),
            "pane-rich",
            native_editor_actor_id("pane-rich"),
            "WS-1",
        );
        assert_eq!(ev.schema_version, NATIVE_EDITOR_SCHEMA_VERSION);
        assert_eq!(ev.action, NativeEditorAction::DocumentSaved);
        assert_eq!(ev.action.as_str(), "document_saved");
        assert_eq!(ev.actor_id, "hsk:native_editor:pane-rich");
        let p = ev.to_native_payload();
        assert_eq!(p["schema"], NATIVE_EDITOR_SCHEMA_VERSION);
        assert_eq!(p["action"], "document_saved");
        assert_eq!(p["pane_id"], "pane-rich");
        assert_eq!(p["payload"]["document_id"], "DOC-1");
        assert_eq!(p["payload"]["content_hash"], "a".repeat(64));
    }

    #[test]
    fn build_post_body_has_every_required_native_editor_field_snake_case() {
        let transport = RuntimeChatLedgerTransport::with_session_id(
            "http://test",
            uuid::Uuid::new_v4().to_string(),
        );
        let ev = NativeEditorEvent::document_saved(
            "DOC-1",
            "f".repeat(64),
            "pane-rich",
            native_editor_actor_id("pane-rich"),
            "WS-9",
        );
        let body = transport.build_post_body(&ev);
        let obj = body.as_object().expect("body is a JSON object");
        // Required fields:
        assert_eq!(obj["schema_version"], NATIVE_EDITOR_SCHEMA_VERSION);
        assert!(obj.contains_key("event_id"), "event_id required");
        assert!(
            uuid::Uuid::parse_str(obj["event_id"].as_str().unwrap()).is_ok(),
            "event_id is a UUID"
        );
        assert!(obj.contains_key("ts_utc"), "ts_utc required");
        assert!(
            chrono::DateTime::parse_from_rfc3339(obj["ts_utc"].as_str().unwrap()).is_ok(),
            "ts_utc is RFC3339"
        );
        // session_id MUST be a non-nil UUID (the backend 400s otherwise).
        let sid = obj["session_id"].as_str().unwrap();
        let parsed = uuid::Uuid::parse_str(sid).expect("session_id parses as UUID");
        assert_ne!(parsed, uuid::Uuid::nil(), "session_id must be non-nil");
        assert_eq!(obj["kind"], "document_saved");
        assert_eq!(obj["actor_id"], "hsk:native_editor:pane-rich");
        assert_eq!(obj["actor_kind"], "human");
        assert_eq!(obj["pane_id"], "pane-rich");
        assert_eq!(obj["surface"], "pane-rich");
        assert_eq!(obj["workspace_id"], "WS-9");
        assert_eq!(obj["work_packet_id"], NATIVE_EDITOR_WORK_PACKET_ID);
        assert_eq!(obj["payload"]["content_hash"], "f".repeat(64));
        // NO unknown/camelCase keys (deny_unknown_fields would 400). Only the allowed snake_case keys.
        let allowed: std::collections::HashSet<&str> = [
            "schema_version",
            "event_id",
            "ts_utc",
            "session_id",
            "kind",
            "actor_id",
            "actor_kind",
            "pane_id",
            "surface",
            "workspace_id",
            "work_packet_id",
            "payload",
        ]
        .into_iter()
        .collect();
        for k in obj.keys() {
            assert!(
                allowed.contains(k.as_str()),
                "unexpected key {k} (would trip deny_unknown_fields)"
            );
        }
    }

    #[test]
    fn action_payload_is_preserved_without_runtime_chat_lossy_folding() {
        let transport = RuntimeChatLedgerTransport::with_session_id(
            "http://test",
            uuid::Uuid::new_v4().to_string(),
        );
        let ev =
            NativeEditorEvent::document_saved("DOC-1", "short-hash", "pane-rich", "act", "WS-1");
        let body = transport.build_post_body(&ev);
        assert_eq!(body["payload"]["content_hash"], "short-hash");
    }

    #[test]
    fn transport_retries_reuse_producer_event_identity() {
        let transport = RuntimeChatLedgerTransport::with_session_id(
            "http://test",
            uuid::Uuid::new_v4().to_string(),
        );
        let event = NativeEditorEvent::document_saved(
            "DOC-1",
            "a".repeat(64),
            "pane-rich",
            DEFAULT_ACTOR_ID,
            "WS-1",
        );
        let first = transport.build_post_body(&event);
        let retry = transport.build_post_body(&event);
        assert_eq!(first["event_id"], retry["event_id"]);
        assert_eq!(first["ts_utc"], retry["ts_utc"]);
    }

    #[test]
    fn calendar_projection_replay_reuses_one_immutable_envelope_but_reconstruction_is_new() {
        let first = NativeEditorEvent::calendar_event_bound(
            "2026-07-18",
            "DOC-1",
            "CAL-1",
            "pane-daily-journal",
            "actor-a",
            "WS-1",
        );
        let replay = first.clone();
        let reconstructed = NativeEditorEvent::calendar_event_bound(
            "2026-07-18",
            "DOC-1",
            "CAL-1",
            "pane-daily-journal",
            "actor-a",
            "WS-1",
        );
        assert_eq!(first, replay);
        assert_eq!(first.event_id, replay.event_id);
        assert_eq!(first.ts_utc, replay.ts_utc);
        assert_ne!(first.event_id, reconstructed.event_id);
        assert!(uuid::Uuid::parse_str(&first.event_id).is_ok());

        let span_first = NativeEditorEvent::activity_span_correlated(
            "CAL-1",
            "SPAN-1",
            vec!["DOC-2".to_owned(), "DOC-1".to_owned(), "DOC-1".to_owned()],
            "pane-daily-journal",
            "actor-a",
            "WS-1",
        );
        let span_replay = span_first.clone();
        let span_reconstructed = NativeEditorEvent::activity_span_correlated(
            "CAL-1",
            "SPAN-1",
            vec!["DOC-2".to_owned(), "DOC-1".to_owned(), "DOC-1".to_owned()],
            "pane-daily-journal",
            "actor-a",
            "WS-1",
        );
        assert_eq!(span_first, span_replay);
        assert_ne!(span_first.event_id, span_reconstructed.event_id);
        assert!(uuid::Uuid::parse_str(&span_first.event_id).is_ok());
    }

    #[test]
    fn pending_frame_queue_coalesces_duplicate_ids_and_discards_stale_generation() {
        let ctx = egui::Context::default();
        let generation = Arc::new(std::sync::atomic::AtomicU64::new(7));
        let event = NativeEditorEvent::calendar_event_bound(
            "2026-07-18",
            "DOC-1",
            "CAL-1",
            "pane-daily-journal",
            "actor-a",
            "WS-1",
        );
        queue_pending_frame_event(
            &ctx,
            PendingFrameEvent::guarded(event.clone(), Arc::clone(&generation), 7),
        );
        queue_pending_frame_event(
            &ctx,
            PendingFrameEvent::guarded(event, Arc::clone(&generation), 7),
        );
        let queued = ctx.data(|data| {
            data.get_temp::<VecDeque<PendingFrameEvent>>(pending_frame_events_id())
                .unwrap_or_default()
        });
        assert_eq!(
            queued.len(),
            1,
            "same semantic receipt must occupy one queue slot"
        );

        for _ in 1..EMIT_PERMITS {
            let mut distinct = queued[0].event.clone();
            distinct.event_id = uuid::Uuid::new_v4().to_string();
            queue_pending_frame_event(
                &ctx,
                PendingFrameEvent::guarded(distinct, Arc::clone(&generation), 7),
            );
        }
        assert_eq!(
            ctx.data(|data| {
                data.get_temp::<VecDeque<PendingFrameEvent>>(pending_frame_events_id())
                    .unwrap_or_default()
                    .len()
            }),
            EMIT_PERMITS
        );

        generation.store(8, std::sync::atomic::Ordering::Release);
        let current = NativeEditorEvent::calendar_event_bound(
            "2026-07-19",
            "DOC-2",
            "CAL-2",
            "pane-daily-journal",
            "actor-a",
            "WS-1",
        );
        queue_pending_frame_event(
            &ctx,
            PendingFrameEvent::guarded(current.clone(), Arc::clone(&generation), 8),
        );
        let pruned = ctx.data(|data| {
            data.get_temp::<VecDeque<PendingFrameEvent>>(pending_frame_events_id())
                .unwrap_or_default()
        });
        assert_eq!(pruned.len(), 1, "stale capacity must be reclaimed first");
        assert_eq!(pruned[0].event.event_id, current.event_id);

        generation.store(9, std::sync::atomic::Ordering::Release);
        assert!(flush_pending_frame_events(&ctx));
        let remaining = ctx.data(|data| {
            data.get_temp::<VecDeque<PendingFrameEvent>>(pending_frame_events_id())
                .unwrap_or_default()
        });
        assert!(
            remaining.is_empty(),
            "superseded receipt must not survive the queue"
        );
    }

    #[test]
    fn emit_without_runtime_records_no_runtime_error_and_does_not_panic() {
        // AC-4: emit() failures do not crash the frame; the error is logged to the in-memory ring.
        let emitter = NativeEditorEventEmitter::new(
            "WS-1",
            Arc::new(MockTransport::new(false)),
            None, // headless: no runtime.
        );
        let res = emitter.emit_document_saved("DOC-1", "h".repeat(64), "pane-rich");
        assert_eq!(res, Err(EmitError::NoRuntime("document_saved".to_owned())));
        assert_eq!(emitter.error_ring().len(), 1);
        assert_eq!(emitter.error_ring().entries()[0].action, "document_saved");
        // Permit was released (not leaked) since no spawn occurred.
        assert_eq!(emitter.available_permits(), EMIT_PERMITS);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn failed_post_lands_in_error_ring_no_panic() {
        // AC-4 (with a runtime): a forced transport failure is logged to the ring, not panicked.
        let emitter = NativeEditorEventEmitter::new(
            "WS-1",
            Arc::new(MockTransport::new(true)), // forced failure.
            Some(tokio::runtime::Handle::current()),
        );
        emitter
            .emit_undo_fired(UndoScope::Local, "pane-rich")
            .expect("dispatched");
        // Let the spawned task run.
        for _ in 0..50 {
            if !emitter.error_ring().is_empty() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        assert_eq!(
            emitter.error_ring().len(),
            1,
            "forced failure should land in the ring"
        );
        assert_eq!(emitter.error_ring().entries()[0].action, "undo_fired");
        assert!(matches!(
            emitter.error_ring().entries()[0].error,
            EmitError::Transport(_)
        ));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn successful_emit_posts_correct_body_through_transport() {
        let mock = Arc::new(MockTransport::new(false));
        let emitter = NativeEditorEventEmitter::new(
            "WS-1",
            mock.clone(),
            Some(tokio::runtime::Handle::current()),
        );
        emitter
            .emit_route_to_stage("selection", "pane-rich")
            .expect("dispatched");
        for _ in 0..50 {
            if !mock.posted.lock().unwrap().is_empty() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let posted = mock.posted.lock().unwrap();
        assert_eq!(posted.len(), 1);
        assert_eq!(posted[0]["kind"], "route_to_stage");
        assert!(emitter.error_ring().is_empty(), "no failures expected");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn authenticated_document_save_preserves_receipt_actor_while_other_events_use_emitter_actor(
    ) {
        let mock = Arc::new(MockTransport::new(false));
        let mut emitter = NativeEditorEventEmitter::new(
            "WS-1",
            mock.clone(),
            Some(tokio::runtime::Handle::current()),
        );
        emitter.set_actor_id("emitter-actor");
        let attribution = crate::rich_editor::save::save_manager::SaveAttribution {
            actor_id: "canonical-save-actor".to_owned(),
            actor_kind: "operator".to_owned(),
            kernel_task_run_id: "task-1".to_owned(),
            session_run_id: "session-1".to_owned(),
            correlation_id: Some("correlation-1".to_owned()),
        };

        let accepted_save = emitter
            .emit_accepted(NativeEditorEvent::document_saved_with_receipt(
                "DOC-1",
                "a".repeat(64),
                "KE-receipt",
                &attribution,
                "pane-rich",
                "WS-1",
            ))
            .expect("authenticated save queued");
        assert_eq!(accepted_save.actor_id, "canonical-save-actor");

        let accepted_undo = emitter
            .emit_accepted(NativeEditorEvent::undo_fired(
                UndoScope::Local,
                "pane-rich",
                "caller-controlled-actor",
                "WS-1",
            ))
            .expect("ordinary event queued");
        assert_eq!(accepted_undo.actor_id, "emitter-actor");

        for _ in 0..50 {
            if mock.posted.lock().unwrap().len() == 2 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let posted = mock.posted.lock().unwrap();
        assert_eq!(posted.len(), 2);
        assert_eq!(posted[0]["actor_id"], "canonical-save-actor");
        assert_eq!(posted[1]["actor_id"], "emitter-actor");
    }

    #[test]
    fn backpressure_drops_to_error_ring_when_permits_exhausted() {
        // RISK-2 / MC-2: a saturated bounded queue DROPS the event into the error ring.
        let mut emitter =
            NativeEditorEventEmitter::new("WS-1", Arc::new(MockTransport::new(false)), None);
        let (sender, _receiver) = tokio::sync::mpsc::channel(1);
        sender
            .try_send(QueuedNativeEditorEvent {
                event: NativeEditorEvent::document_saved(
                    "occupied",
                    "hash",
                    "pane",
                    DEFAULT_ACTOR_ID,
                    "WS-1",
                ),
                completion: None,
            })
            .expect("fill queue");
        emitter.sender = Some(sender);
        let res = emitter.emit_document_saved("DOC-1", "h".repeat(64), "pane-rich");
        assert_eq!(
            res,
            Err(EmitError::Backpressure("document_saved".to_owned()))
        );
        assert_eq!(emitter.error_ring().len(), 1);
        assert!(matches!(
            emitter.error_ring().entries()[0].error,
            EmitError::Backpressure(_)
        ));
    }

    #[test]
    fn closed_worker_is_permanent_and_visible() {
        let mut emitter =
            NativeEditorEventEmitter::new("WS-1", Arc::new(MockTransport::new(false)), None);
        let (sender, receiver) = tokio::sync::mpsc::channel(1);
        drop(receiver);
        emitter.sender = Some(sender);

        assert_eq!(
            emitter.emit_document_saved("DOC-1", "h".repeat(64), "pane-rich"),
            Err(EmitError::WorkerClosed("document_saved".to_owned()))
        );
        assert!(matches!(
            emitter.error_ring().entries()[0].error,
            EmitError::WorkerClosed(_)
        ));
    }

    #[tokio::test]
    async fn nonempty_event_workspace_is_never_relabelled() {
        let emitter = NativeEditorEventEmitter::new(
            "workspace-b",
            Arc::new(MockTransport::new(false)),
            Some(tokio::runtime::Handle::current()),
        );
        let event = NativeEditorEvent::document_saved(
            "DOC-1",
            "h".repeat(64),
            "pane-rich",
            DEFAULT_ACTOR_ID,
            "workspace-a",
        );

        assert_eq!(
            emitter.emit(event),
            Err(EmitError::WorkspaceMismatch {
                event: "workspace-a".to_owned(),
                emitter: "workspace-b".to_owned(),
            })
        );
        assert!(matches!(
            emitter.error_ring().entries()[0].error,
            EmitError::WorkspaceMismatch { .. }
        ));
    }

    #[test]
    fn actor_id_format_is_descriptive_and_valid() {
        // RISK-5 / MC-5: a descriptive, non-empty actor id (the backend only requires non-empty).
        assert_eq!(
            native_editor_actor_id("pane-code"),
            "hsk:native_editor:pane-code"
        );
        assert_eq!(native_editor_actor_id(""), DEFAULT_ACTOR_ID);
        assert!(!native_editor_actor_id("pane-code").trim().is_empty());
    }

    #[test]
    fn all_actions_have_distinct_wire_strings() {
        use NativeEditorAction::*;
        let actions = [
            DocumentSaved,
            CodeEdit,
            EmbedCreated,
            CanvasNodePlaced,
            CrossRefInserted,
            UndoFired,
            RouteToStage,
            StageEmbedBack,
            CalendarEventBound,
            ActivitySpanCorrelated,
            LocusRefResolved,
            LocusReverseLookup,
        ];
        let mut seen = std::collections::HashSet::new();
        for a in actions {
            assert!(
                seen.insert(a.as_str()),
                "duplicate action wire string {}",
                a.as_str()
            );
        }
        assert_eq!(seen.len(), 12);
    }

    #[test]
    fn error_ring_is_bounded_at_cap() {
        let ring = ErrorRing::new();
        for i in 0..(ERROR_RING_CAP + 10) {
            ring.push(EmitErrorEntry {
                action: format!("a{i}"),
                error: EmitError::Transport("x".to_owned()),
            });
        }
        assert_eq!(
            ring.len(),
            ERROR_RING_CAP,
            "ring must stay bounded at the cap"
        );
        // The oldest entries were evicted; the newest survive.
        let entries = ring.entries();
        assert_eq!(
            entries.last().unwrap().action,
            format!("a{}", ERROR_RING_CAP + 9)
        );
    }
}

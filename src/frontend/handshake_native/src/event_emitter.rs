//! WP-KERNEL-012 MT-036 (E5 — one event ledger across surfaces).
//!
//! The [`NativeEditorEventEmitter`] is the single melt-together producer that turns a native editor
//! action (document save, undo fired, route-to-stage, code edit, embed/cross-ref, canvas node placed)
//! into a typed [`NativeEditorEvent`] and ships it to the EXISTING handshake_core Flight Recorder
//! observability ledger. It is the HBR-SWARM/HBR-INT observability seam: without it a swarm agent or the
//! operator cannot see what the native editors are doing.
//!
//! ## What is REAL and WIRED now vs DEFERRED (the MT-035 anti-scaffolding lesson)
//!
//! - The emitter, the [`NativeEditorEvent`] schema, the [`NativeEditorEventEmitter::build_post_body`]
//!   wire-shape, the [`Semaphore`]-bounded off-frame spawn (RISK-2 / MC-2), and the cap-20 in-memory
//!   error ring are ALL REAL and unit-proven standalone.
//! - The emitter is WIRED at the LIVE call sites that exist + are tested NOW: the rich-text SAVE success
//!   path (MT-020 `save_manager`) emits `document_saved`, and the rich-pane UNDO dispatch (MT-035)
//!   emits `undo_fired`. The route-to-stage emit hooks the MT-033 command.
//! - Call sites in panes NOT yet mounted in `app.rs` (the code-edit batch, the canvas live placement
//!   path) are HONESTLY DEFERRED to E11/MT-069 as a carry-forward. The convenience helpers exist + are
//!   unit-testable standalone (correct body shape) so the live wiring is a one-line call once those
//!   panes mount — no test-only emit helper that no live code calls.
//!
//! ## Backend-shape TYPED BLOCKER (verified read-only against src/backend/handshake_core, MT RISK-1/5)
//!
//! The MT contract assumed `POST /api/flight_recorder/runtime_chat_event` would accept
//! `event_type='system'` with a nested `native_editor_event` payload, then be queryable by
//! `actor_id='native_editor_human'`. Verification of `src/backend/handshake_core/src/api/flight_recorder.rs`
//! shows the REAL backend CANNOT carry that today:
//!   1. `RuntimeChatEventV0_1` is `#[serde(deny_unknown_fields)]` — a nested `native_editor_event` field
//!      is REJECTED (400).
//!   2. its `type` field is a CLOSED 3-value enum (`runtime_chat_message_appended` /
//!      `_ans001_validation` / `_session_closed`); there is NO `system` variant.
//!   3. the handler HARDCODES `actor_id = "runtime_chat"` and `actor = System`, so a native
//!      `actor_id` cannot be set through this endpoint.
//!   4. `session_id` MUST parse as a non-nil UUID, else 400 `HSK-400-INVALID-EVENT`.
//!   5. `GET /flight_recorder` has NO `actor_id` filter (only `actor` ∈ {human,agent,system}); and the
//!      only `editor_edit` FlightEvents the ledger holds are emitted SERVER-SIDE from the Atelier-apply
//!      endpoint (`api/workspaces.rs`), hardwired to `editor_surface="monaco"` — there is no generic
//!      HTTP route to POST a native-editor `editor_edit`/`system` event with a custom action/pane_id.
//!
//! Therefore the FULL native-editor → ledger round-trip needs a NEW backend ingestion endpoint (e.g.
//! `POST /flight_recorder/native_editor_event` recording a `FlightRecorderEventType::EditorEdit` with a
//! native `actor_id` + `editor_surface`, queryable by `actor`/`surface`). Backend edits are out of scope
//! (`src/backend/** = reuse-via-API-only`), so this is a TYPED BLOCKER, not a backend edit and not a
//! fake. To keep the producer REAL + swappable, the emitter ships its body through the
//! [`EventLedgerTransport`] seam: [`RuntimeChatLedgerTransport`] targets the verified
//! `runtime_chat_event` endpoint and builds the EXACT verified `RuntimeChatEventV0_1` body, so the
//! emitter is correct + live the moment the backend gap closes; until then the round-trip ACs are
//! recorded as blocked (the emit still runs + logs honestly, never faked).

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use serde_json::{json, Value as JsonValue};

/// The closed schema version every native editor event carries (the MT contract's exact string). It
/// namespaces the native-editor payload so a consumer can distinguish it from other ledger producers.
pub const NATIVE_EDITOR_SCHEMA_VERSION: &str = "hsk.native_editor@0.1";

/// The verified Flight Recorder runtime-chat wire schema version (the body the production transport
/// posts — `src/backend/handshake_core/src/api/flight_recorder.rs` `RuntimeChatEventV0_1.schema_version`).
pub const FR_RUNTIME_CHAT_SCHEMA_VERSION: &str = "hsk.fr.runtime_chat@0.1";

/// The cap on the in-memory error ring (RISK-2 / MC-2): a bounded buffer of the most recent emit
/// failures, surfaced by the [`crate::flight_recorder_pane::FlightRecorderPane`] so a no-context model
/// sees WHY the ledger looks empty rather than silent loss.
pub const ERROR_RING_CAP: usize = 20;

/// The default in-flight emit permit count (RISK-2 / MC-2): at most this many spawned POSTs may be
/// outstanding at once. A saturated emitter DROPS the event into the error ring rather than letting the
/// spawn queue grow unbounded under a rapid edit burst.
pub const EMIT_PERMITS: usize = 20;

/// The default native-editor actor id when no operator/model session is active. A DESCRIPTIVE but valid
/// id (RISK-5 / MC-5): `hsk:native_editor:{pane_id}` is built per-event via [`native_editor_actor_id`];
/// this fallback is used when a pane id is unknown.
pub const DEFAULT_ACTOR_ID: &str = "hsk:native_editor:human";

/// The structured native-editor action kind. Maps 1:1 to the MT contract's action set. `event_type` on
/// the ledger stays the CLOSED backend vocabulary; the action lives INSIDE the native-editor payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeEditorAction {
    /// A rich document was canonically saved (rich-text SAVE path — LIVE wired, MT-020).
    DocumentSaved,
    /// A debounced batch of code edits landed (code pane — DEFERRED to E11/MT-069 live wiring).
    CodeEdit,
    /// An embed (image/slideshow/album/video) atom was inserted into a note (DEFERRED live wiring).
    EmbedCreated,
    /// A node was placed on the canvas (canvas live placement — DEFERRED to E11/MT-069 live wiring).
    CanvasNodePlaced,
    /// A code/note cross-reference was inserted (DEFERRED live wiring).
    CrossRefInserted,
    /// An undo or redo fired (rich-pane undo dispatch — LIVE wired, MT-035).
    UndoFired,
    /// Content was routed to the Stage pane (route-to-stage command — LIVE wired, MT-033).
    RouteToStage,
    /// A review-gated FEMS memory-write PROPOSAL was submitted from the editor (MT-064, E9). This is the
    /// `FR-EVT-MEM-001` (`memory_write_proposed`) marker carried in the payload `action` — NOT a new
    /// ledger `event_type` (the `event_type` stays the CLOSED backend vocabulary per the MT-036 schema).
    /// The editor only ever proposes (review-gated); the commit is downstream and never editor-direct.
    MemoryWriteProposed,
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
            NativeEditorAction::MemoryWriteProposed => "memory_write_proposed",
        }
    }
}

/// One native editor event: the typed melt-together record a surface emits. `payload` carries the
/// action-specific fields (document_id/content_hash, file_path/line_delta, embed_kind/item_id, …) the MT
/// contract names; the common identity (`schema_version` / `action` / `pane_id` / `actor_id` /
/// `workspace_id`) is hoisted to typed fields so a consumer needs no payload re-parse for routing.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NativeEditorEvent {
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
        Self::new(NativeEditorAction::DocumentSaved, pane_id, actor_id, workspace_id, payload)
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
        Self::new(NativeEditorAction::CodeEdit, pane_id, actor_id, workspace_id, payload)
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
        Self::new(NativeEditorAction::EmbedCreated, pane_id, actor_id, workspace_id, payload)
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
        Self::new(NativeEditorAction::CanvasNodePlaced, pane_id, actor_id, workspace_id, payload)
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
        Self::new(NativeEditorAction::CrossRefInserted, pane_id, actor_id, workspace_id, payload)
    }

    /// `undo_fired`: an undo/redo fired. Payload: `{ scope }` where `scope` ∈ {"local","cross_pane"}.
    pub fn undo_fired(
        scope: UndoScope,
        pane_id: impl Into<String>,
        actor_id: impl Into<String>,
        workspace_id: impl Into<String>,
    ) -> Self {
        let payload = json!({ "scope": scope.as_str() });
        Self::new(NativeEditorAction::UndoFired, pane_id, actor_id, workspace_id, payload)
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
        Self::new(NativeEditorAction::RouteToStage, source_pane_id, actor_id, workspace_id, payload)
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
    /// The transport POST failed (backend unreachable / non-2xx). Carries the reason.
    Transport(String),
}

impl std::fmt::Display for EmitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmitError::Backpressure(a) => write!(f, "emit backpressure (permits saturated): {a}"),
            EmitError::NoRuntime(a) => write!(f, "no tokio runtime for emit: {a}"),
            EmitError::Transport(r) => write!(f, "emit transport failure: {r}"),
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
}

impl ErrorRing {
    /// A fresh empty ring.
    pub fn new() -> Self {
        Self { inner: Arc::new(Mutex::new(VecDeque::with_capacity(ERROR_RING_CAP))) }
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

    /// The current entries, oldest-first (a snapshot the pane renders).
    pub fn entries(&self) -> Vec<EmitErrorEntry> {
        self.inner.lock().map(|q| q.iter().cloned().collect()).unwrap_or_default()
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

/// The async transport that ships a native editor event to the ledger. A `Send + Sync` trait so the
/// production HTTP transport and a unit mock are interchangeable (the MT-019/MT-017 transport-trait
/// precedent), and so the destination endpoint is SWAPPABLE the moment the backend ingestion gap closes
/// (the typed blocker) without touching the emitter.
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

/// The production transport: posts to the VERIFIED `POST /api/flight_recorder/runtime_chat_event`
/// endpoint, building the EXACT `RuntimeChatEventV0_1` body the backend accepts
/// (`schema_version='hsk.fr.runtime_chat@0.1'`, a fresh UUID `event_id`, an RFC3339 `ts_utc`, a non-nil
/// UUID `session_id`, a closed `type`, optional `wsid` + `work_packet_id`). The native-editor payload is
/// folded into the fields the closed schema CAN carry (`message_id` = native action, `body_sha256` =
/// content hash when present); the FULL native payload requires the new ingestion endpoint (TYPED
/// BLOCKER) — see the module doc. Reuses the existing reqwest 0.12 stack (no new dependency family).
#[derive(Clone)]
pub struct RuntimeChatLedgerTransport {
    client: reqwest::Client,
    base_url: String,
    /// A stable, valid non-nil UUID used as the `session_id` the backend requires (it rejects a nil or
    /// non-UUID session id with 400). One per emitter session so every native-editor emit is attributable
    /// to the same Flight Recorder trace.
    session_id: String,
}

impl RuntimeChatLedgerTransport {
    /// Build a transport against `base_url` (e.g. [`crate::backend_client::BACKEND_BASE_URL`]) with a
    /// fresh per-session UUID `session_id`.
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            session_id: uuid::Uuid::new_v4().to_string(),
        }
    }

    /// Build a transport with an explicit `session_id` (tests / a shared trace id).
    pub fn with_session_id(base_url: impl Into<String>, session_id: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            session_id: session_id.into(),
        }
    }

    fn url(&self) -> String {
        format!("{}/api/flight_recorder/runtime_chat_event", self.base_url)
    }

    /// The session id this transport stamps on every event (a non-nil UUID string).
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
}

impl EventLedgerTransport for RuntimeChatLedgerTransport {
    fn build_post_body(&self, event: &NativeEditorEvent) -> JsonValue {
        // The verified RuntimeChatEventV0_1 shape (exact snake_case keys; deny_unknown_fields on the
        // backend means ONLY these keys are allowed). `type` is a CLOSED enum — we use the
        // message-appended variant (the only general-purpose runtime-chat type) to carry the native
        // editor event identity in the allowed `message_id` field. `wsid`/`work_packet_id` are optional.
        // body_sha256 carries the content hash when the action provides one (document_saved), so an
        // observer can still correlate by hash.
        let body_sha256 = event
            .payload
            .get("content_hash")
            .and_then(|v| v.as_str())
            .map(|s| s.to_owned());
        let mut body = json!({
            "schema_version": FR_RUNTIME_CHAT_SCHEMA_VERSION,
            "event_id": uuid::Uuid::new_v4().to_string(),
            "ts_utc": chrono::Utc::now().to_rfc3339(),
            "session_id": self.session_id,
            "type": "runtime_chat_message_appended",
            "message_id": format!("native_editor:{}", event.action.as_str()),
            "role": "user",
        });
        if !event.workspace_id.trim().is_empty() {
            body["wsid"] = JsonValue::String(event.workspace_id.clone());
        }
        if let Some(hash) = body_sha256 {
            // The backend requires body_sha256 to be 64 hex chars; only attach when it qualifies so we
            // never trip the validator (a non-conforming hash would 400 the otherwise-valid event).
            if hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit()) {
                body["body_sha256"] = JsonValue::String(hash);
            }
        }
        body
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
/// [`Semaphore`]-style bounded permit count (RISK-2 / MC-2), and the [`ErrorRing`] failures surface.
#[derive(Clone)]
pub struct NativeEditorEventEmitter {
    /// The active workspace id stamped on every event.
    workspace_id: String,
    /// The resolved actor id (e.g. `hsk:native_editor:human`); per-event the pane-scoped id is used.
    actor_id: String,
    /// The ledger transport (production HTTP or a unit mock).
    transport: Arc<dyn EventLedgerTransport>,
    /// The app tokio runtime; `None` headless (an emit then records a `NoRuntime` failure, never faked).
    runtime: Option<tokio::runtime::Handle>,
    /// The bounded in-flight permit pool (RISK-2 / MC-2): a saturated emitter DROPS to the error ring.
    permits: Arc<tokio::sync::Semaphore>,
    /// The bounded failures ring the FlightRecorderPane surfaces.
    error_ring: ErrorRing,
}

impl NativeEditorEventEmitter {
    /// Build an emitter for `workspace_id` over `transport`, dispatching on `runtime` (when present),
    /// bounded by [`EMIT_PERMITS`] concurrent emits. `actor_id` defaults to [`DEFAULT_ACTOR_ID`].
    pub fn new(
        workspace_id: impl Into<String>,
        transport: Arc<dyn EventLedgerTransport>,
        runtime: Option<tokio::runtime::Handle>,
    ) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            actor_id: DEFAULT_ACTOR_ID.to_owned(),
            transport,
            runtime,
            permits: Arc::new(tokio::sync::Semaphore::new(EMIT_PERMITS)),
            error_ring: ErrorRing::new(),
        }
    }

    /// The production emitter: the [`RuntimeChatLedgerTransport`] against `base_url`, on `runtime`.
    pub fn production(
        workspace_id: impl Into<String>,
        base_url: impl Into<String>,
        runtime: tokio::runtime::Handle,
    ) -> Self {
        let transport = Arc::new(RuntimeChatLedgerTransport::new(base_url));
        Self::new(workspace_id, transport, Some(runtime))
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
        self.permits.available_permits()
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
    pub fn emit(&self, event: NativeEditorEvent) -> Result<(), EmitError> {
        let action = event.action.as_str().to_owned();
        // RISK-2 / MC-2: bound concurrency. try_acquire is non-blocking; a saturated pool drops.
        let permit = match self.permits.clone().try_acquire_owned() {
            Ok(p) => p,
            Err(_) => {
                let err = EmitError::Backpressure(action.clone());
                self.error_ring.push(EmitErrorEntry { action, error: err.clone() });
                return Err(err);
            }
        };
        let Some(rt) = &self.runtime else {
            // Headless: no dispatch. Record honestly; the permit drops here (no spawn).
            drop(permit);
            let err = EmitError::NoRuntime(action.clone());
            self.error_ring.push(EmitErrorEntry { action, error: err.clone() });
            return Err(err);
        };
        let transport = Arc::clone(&self.transport);
        let ring = self.error_ring.clone();
        rt.spawn(async move {
            // The permit is held for the lifetime of the POST, then dropped (releasing a slot).
            let _permit = permit;
            if let Err(e) = transport.post(event).await {
                tracing::warn!(action = %action, error = %e, "MT-036 native editor event emit failed");
                ring.push(EmitErrorEntry { action, error: e });
            }
        });
        Ok(())
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

    /// Emit `embed_created` (DEFERRED live wiring).
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

    /// Emit `cross_ref_inserted` (DEFERRED live wiring).
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
            Self { posted: Arc::new(Mutex::new(Vec::new())), fail }
        }
    }
    impl EventLedgerTransport for MockTransport {
        fn build_post_body(&self, event: &NativeEditorEvent) -> JsonValue {
            // The mock mirrors the production shape so the body-shape unit test can run against it too.
            RuntimeChatLedgerTransport::with_session_id("http://test", uuid::Uuid::new_v4().to_string())
                .build_post_body(event)
        }
        fn post(
            &self,
            event: NativeEditorEvent,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), EmitError>> + Send>> {
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
    fn build_post_body_has_every_required_runtime_chat_field_snake_case() {
        // RISK-1 / MC-1: assert every REQUIRED RuntimeChatEventV0_1 field is present with the exact
        // snake_case key the backend's deny_unknown_fields handler demands.
        let transport =
            RuntimeChatLedgerTransport::with_session_id("http://test", uuid::Uuid::new_v4().to_string());
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
        assert_eq!(obj["schema_version"], FR_RUNTIME_CHAT_SCHEMA_VERSION);
        assert!(obj.contains_key("event_id"), "event_id required");
        assert!(uuid::Uuid::parse_str(obj["event_id"].as_str().unwrap()).is_ok(), "event_id is a UUID");
        assert!(obj.contains_key("ts_utc"), "ts_utc required");
        assert!(
            chrono::DateTime::parse_from_rfc3339(obj["ts_utc"].as_str().unwrap()).is_ok(),
            "ts_utc is RFC3339"
        );
        // session_id MUST be a non-nil UUID (the backend 400s otherwise).
        let sid = obj["session_id"].as_str().unwrap();
        let parsed = uuid::Uuid::parse_str(sid).expect("session_id parses as UUID");
        assert_ne!(parsed, uuid::Uuid::nil(), "session_id must be non-nil");
        // type is one of the CLOSED runtime-chat enum values.
        assert_eq!(obj["type"], "runtime_chat_message_appended");
        // optional wsid is attached when present.
        assert_eq!(obj["wsid"], "WS-9");
        // body_sha256 attached (64 hex) so an observer can correlate by content hash.
        assert_eq!(obj["body_sha256"], "f".repeat(64));
        // NO unknown/camelCase keys (deny_unknown_fields would 400). Only the allowed snake_case keys.
        let allowed: std::collections::HashSet<&str> = [
            "schema_version", "event_id", "ts_utc", "session_id", "job_id", "work_packet_id",
            "spec_id", "wsid", "type", "message_id", "role", "model_role", "body_sha256",
            "ans001_sha256", "ans001_compliant", "violation_clauses",
        ]
        .into_iter()
        .collect();
        for k in obj.keys() {
            assert!(allowed.contains(k.as_str()), "unexpected key {k} (would trip deny_unknown_fields)");
        }
    }

    #[test]
    fn non_conforming_content_hash_is_omitted_not_sent() {
        // A non-64-hex content hash must NOT be attached as body_sha256 (it would 400 the event).
        let transport = RuntimeChatLedgerTransport::with_session_id("http://test", uuid::Uuid::new_v4().to_string());
        let ev = NativeEditorEvent::document_saved("DOC-1", "short-hash", "pane-rich", "act", "WS-1");
        let body = transport.build_post_body(&ev);
        assert!(body.as_object().unwrap().get("body_sha256").is_none());
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
        emitter.emit_undo_fired(UndoScope::Local, "pane-rich").expect("dispatched");
        // Let the spawned task run.
        for _ in 0..50 {
            if !emitter.error_ring().is_empty() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        assert_eq!(emitter.error_ring().len(), 1, "forced failure should land in the ring");
        assert_eq!(emitter.error_ring().entries()[0].action, "undo_fired");
        assert!(matches!(emitter.error_ring().entries()[0].error, EmitError::Transport(_)));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn successful_emit_posts_correct_body_through_transport() {
        let mock = Arc::new(MockTransport::new(false));
        let emitter = NativeEditorEventEmitter::new(
            "WS-1",
            mock.clone(),
            Some(tokio::runtime::Handle::current()),
        );
        emitter.emit_route_to_stage("selection", "pane-rich").expect("dispatched");
        for _ in 0..50 {
            if !mock.posted.lock().unwrap().is_empty() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let posted = mock.posted.lock().unwrap();
        assert_eq!(posted.len(), 1);
        assert_eq!(posted[0]["message_id"], "native_editor:route_to_stage");
        assert!(emitter.error_ring().is_empty(), "no failures expected");
    }

    #[test]
    fn backpressure_drops_to_error_ring_when_permits_exhausted() {
        // RISK-2 / MC-2: a saturated permit pool DROPS the event into the error ring rather than
        // spawning unbounded tasks. Build an emitter with ZERO permits to force the saturated path.
        let mut emitter = NativeEditorEventEmitter::new(
            "WS-1",
            Arc::new(MockTransport::new(false)),
            None,
        );
        // Replace the permit pool with an exhausted one (0 permits) to deterministically force drop.
        emitter.permits = Arc::new(tokio::sync::Semaphore::new(0));
        let res = emitter.emit_document_saved("DOC-1", "h".repeat(64), "pane-rich");
        assert_eq!(res, Err(EmitError::Backpressure("document_saved".to_owned())));
        assert_eq!(emitter.error_ring().len(), 1);
        assert!(matches!(emitter.error_ring().entries()[0].error, EmitError::Backpressure(_)));
    }

    #[test]
    fn actor_id_format_is_descriptive_and_valid() {
        // RISK-5 / MC-5: a descriptive, non-empty actor id (the backend only requires non-empty).
        assert_eq!(native_editor_actor_id("pane-code"), "hsk:native_editor:pane-code");
        assert_eq!(native_editor_actor_id(""), DEFAULT_ACTOR_ID);
        assert!(!native_editor_actor_id("pane-code").trim().is_empty());
    }

    #[test]
    fn all_actions_have_distinct_wire_strings() {
        use NativeEditorAction::*;
        let actions = [
            DocumentSaved, CodeEdit, EmbedCreated, CanvasNodePlaced, CrossRefInserted, UndoFired,
            RouteToStage, MemoryWriteProposed,
        ];
        let mut seen = std::collections::HashSet::new();
        for a in actions {
            assert!(seen.insert(a.as_str()), "duplicate action wire string {}", a.as_str());
        }
        assert_eq!(seen.len(), 8);
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
        assert_eq!(ring.len(), ERROR_RING_CAP, "ring must stay bounded at the cap");
        // The oldest entries were evicted; the newest survive.
        let entries = ring.entries();
        assert_eq!(entries.last().unwrap().action, format!("a{}", ERROR_RING_CAP + 9));
    }
}

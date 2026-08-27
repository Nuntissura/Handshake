//! Flight Recorder HTTP surface.
//!
//! # Authorization boundary (WP-KERNEL-012 MT-109, FAIL_V4 remediation)
//!
//! Every route in this group is behind [`authorize_flight_recorder_request`], a fail-closed
//! `axum` middleware that mirrors the boundary already proven in [`crate::api::memory`]:
//!
//! * The caller is authenticated with [`crate::api::stage::capture_context`] (live native-MCP
//!   binding token + process-birth identity). No binding, a forged token, or a stale binding is
//!   `401 HSK-401-FR-SESSION` and performs no recorder or EventLedger mutation.
//! * The route class maps to exactly one capability, resolved through
//!   `capability_registry.profile_can("Operator", ..)`. A denied capability is
//!   `403 HSK-403-FR-CAPABILITY` with no mutation.
//!   - `GET /flight_recorder` / `GET /events` require `fr.read`, and additionally
//!     `fr.read.global` when the caller omits `?wsid=` (unscoped cross-workspace enumeration).
//!   - `POST /workspaces/:workspace_id/flight_recorder/runtime_chat_event` requires
//!     `fr.ingest.runtime_chat`.
//!   - `POST /workspaces/:workspace_id/flight_recorder/native_editor_event` requires
//!     `fr.ingest.native_editor`.
//! * Every allow and every deny emits one redacted `capability_action` Flight Recorder event
//!   (`capability_id` / `actor_id` / `job_id` / `decision_outcome` only — never the session token,
//!   the request body, or the queried event payloads). If the audit write fails the request fails
//!   closed with `500`.
//!
//! # Canonical native-editor envelope contract
//!
//! `actor_id`, `actor_kind` and `workspace_id` are NO LONGER client authority:
//!
//! * `actor_id` is optional. When present it MUST equal the authenticated
//!   [`crate::api::stage::CaptureContext::actor_id`] exactly; otherwise the request is rejected
//!   with `403 HSK-403-FR-ACTOR-SPOOF` before any durable write. When absent the server fills it.
//! * `actor_kind` is optional and is derived from the authenticated context (the native-MCP
//!   binding authenticates an operator, so the lane is always `human`). A client-supplied value
//!   that disagrees is rejected with `403 HSK-403-FR-ACTOR-SPOOF`.
//! * `workspace_id` is optional. When present it MUST equal the `:workspace_id` path segment;
//!   otherwise the request is rejected with `403 HSK-403-FR-WORKSPACE` before any durable write.
//!   When absent the server fills it from the path. The path workspace must resolve to a real
//!   canonical workspace row; an unknown workspace is also `403 HSK-403-FR-WORKSPACE` (the same
//!   status as an unauthorized one, so the route never discloses workspace existence).
//!
//! # Workspace-partitioned idempotency
//!
//! The durable Flight Recorder / EventLedger identity of a native-editor event is
//! [`workspace_scoped_fr_event_id`] — a deterministic digest of
//! `(namespace, authenticated workspace_id, client event_id)`. Two workspaces that submit the same
//! client `event_id` therefore own two disjoint aggregates, so a caller cannot pre-seed another
//! workspace's `event_id`, turn its honest retry into a `409`, read its row back, or complete /
//! reconcile its pending receipt. The ingestion ack returns both `event_id` (the client id, echoed
//! canonicalized) and `fr_event_id` (the durable server-derived id used by `GET /flight_recorder`).

use crate::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use crate::models::ErrorResponse;
use crate::AppState;
use axum::{
    extract::{Path, Query, Request, State},
    http::{Method, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Extension, Json, Router,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value;
use sha2::{Digest, Sha256};
use uuid::Uuid;

type ApiError = (StatusCode, Json<ErrorResponse>);
type ApiResult<T> = Result<T, ApiError>;

/// Recorder read, scoped to one workspace named by `?wsid=`.
const FR_READ_CAPABILITY: &str = "fr.read";
/// Strictly higher capability required to omit `?wsid=` and enumerate every workspace.
const FR_READ_GLOBAL_CAPABILITY: &str = "fr.read.global";
const FR_INGEST_RUNTIME_CHAT_CAPABILITY: &str = "fr.ingest.runtime_chat";
const FR_INGEST_NATIVE_EDITOR_CAPABILITY: &str = "fr.ingest.native_editor";
/// The capability profile the authenticated native-MCP binding resolves to. `capture_context`
/// only ever mints `actor_kind = "operator"`, so this mirrors `api::memory`'s proven boundary.
const FR_CAPABILITY_PROFILE: &str = "Operator";

/// Recorder-relative ingestion paths. Named so the workspace-scoped route, the middleware's
/// capability router, and the native client all share exactly one spelling.
pub const NATIVE_EDITOR_INGEST_PATH: &str = "/flight_recorder/native_editor_event";
pub const RUNTIME_CHAT_INGEST_PATH: &str = "/flight_recorder/runtime_chat_event";

fn invalid_event() -> ApiError {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "HSK-400-INVALID-EVENT",
        }),
    )
}

fn unauthenticated_recorder() -> ApiError {
    (
        StatusCode::UNAUTHORIZED,
        Json(ErrorResponse {
            error: "HSK-401-FR-SESSION",
        }),
    )
}

fn capability_denied() -> ApiError {
    (
        StatusCode::FORBIDDEN,
        Json(ErrorResponse {
            error: "HSK-403-FR-CAPABILITY",
        }),
    )
}

/// One status for "workspace does not exist" and "workspace is not yours", so the route never
/// discloses which workspaces exist.
fn workspace_denied() -> ApiError {
    (
        StatusCode::FORBIDDEN,
        Json(ErrorResponse {
            error: "HSK-403-FR-WORKSPACE",
        }),
    )
}

fn actor_spoof_denied() -> ApiError {
    (
        StatusCode::FORBIDDEN,
        Json(ErrorResponse {
            error: "HSK-403-FR-ACTOR-SPOOF",
        }),
    )
}

fn audit_failed_closed() -> ApiError {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "HSK-500-FR-CAPABILITY-AUDIT",
        }),
    )
}

fn event_conflict() -> ApiError {
    (
        StatusCode::CONFLICT,
        Json(ErrorResponse {
            error: "HSK-409-EVENT-ID-CONFLICT",
        }),
    )
}

fn db_error(err: impl std::fmt::Display) -> ApiError {
    tracing::error!(target: "handshake_core", error = %err, "flight_recorder_db_error");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "HSK-500-DB",
        }),
    )
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FlightEvent {
    pub event_id: String,
    pub trace_id: String,
    pub timestamp: String,
    pub actor: String,
    pub actor_id: String,
    pub event_type: String,
    pub job_id: Option<String>,
    pub workflow_id: Option<String>,
    pub model_id: Option<String>,
    pub model_session_id: Option<String>,
    pub wsids: Vec<String>,
    pub activity_span_id: Option<String>,
    pub session_span_id: Option<String>,
    pub capability_id: Option<String>,
    pub policy_decision_id: Option<String>,
    pub payload: Value,
}

#[derive(Deserialize, Debug, Default)]
pub struct EventFilter {
    pub event_id: Option<Uuid>,
    pub job_id: Option<String>,
    pub trace_id: Option<Uuid>,
    pub model_session_id: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub actor: Option<String>,
    pub actor_id: Option<String>,
    pub surface: Option<String>,
    pub event_type: Option<String>,
    pub wsid: Option<String>,
}

pub fn routes(state: AppState) -> Router {
    spawn_native_editor_reconciler(state.clone());
    let middleware_state = state.clone();
    Router::new()
        .route("/flight_recorder", get(list_events))
        .route("/events", get(list_events)) // backward-compatible path
        // Both ingestion routes are workspace-scoped: the PATH is the workspace authority, so a
        // body field can never widen it. The unscoped `/flight_recorder/{runtime_chat,
        // native_editor}_event` paths are deliberately gone — they had no bindable workspace
        // authority and were the FAIL_V4 unauthenticated write surface.
        .route(
            concat!(
                "/workspaces/:workspace_id",
                "/flight_recorder/runtime_chat_event"
            ),
            post(record_runtime_chat_event),
        )
        .route(
            concat!(
                "/workspaces/:workspace_id",
                "/flight_recorder/native_editor_event"
            ),
            post(record_native_editor_event),
        )
        .layer(middleware::from_fn_with_state(
            middleware_state,
            authorize_flight_recorder_request,
        ))
        .with_state(state)
}

/// The authenticated recorder authority the middleware hands to every handler. Handlers read
/// identity ONLY from here — never from headers or the request body, both of which the caller
/// controls.
#[derive(Clone)]
pub(crate) struct RecorderAuthority {
    pub(crate) ctx: crate::api::stage::CaptureContext,
    /// The `:workspace_id` path segment, present for the ingestion routes.
    pub(crate) workspace_id: Option<String>,
}

impl RecorderAuthority {
    /// The bounded native-editor actor lane the authenticated context maps to. `capture_context`
    /// authenticates the native shell as an operator, so the lane is `human`; agent/system lanes
    /// have no authenticating binding today and must not be reachable from a request body.
    fn native_editor_actor_kind(&self) -> NativeEditorActorKind {
        match self.ctx.actor_kind.as_str() {
            "operator" => NativeEditorActorKind::Human,
            "agent" | "local_model" | "cloud_model" => NativeEditorActorKind::Agent,
            _ => NativeEditorActorKind::System,
        }
    }
}

/// Route class -> required capability. Fail-closed: an unmapped path under this router is denied.
fn flight_recorder_capability_for_request(method: &Method, path: &str) -> Option<&'static str> {
    if method == Method::POST {
        if path.ends_with(NATIVE_EDITOR_INGEST_PATH) {
            return Some(FR_INGEST_NATIVE_EDITOR_CAPABILITY);
        }
        if path.ends_with(RUNTIME_CHAT_INGEST_PATH) {
            return Some(FR_INGEST_RUNTIME_CHAT_CAPABILITY);
        }
        return None;
    }
    if method == Method::GET && matches!(path, "/flight_recorder" | "/events") {
        return Some(FR_READ_CAPABILITY);
    }
    None
}

/// `/workspaces/{workspace_id}/flight_recorder/...` -> the path workspace segment.
fn workspace_id_from_recorder_path(path: &str) -> Option<String> {
    let mut segments = path.trim_start_matches('/').split('/');
    match (segments.next(), segments.next(), segments.next()) {
        (Some("workspaces"), Some(workspace_id), Some("flight_recorder"))
            if !workspace_id.is_empty() =>
        {
            Some(workspace_id.to_owned())
        }
        _ => None,
    }
}

/// The single redacted audit shape for every recorder authorization decision. Matches the
/// `capability_action` FR contract exactly (`capability_id` / `actor_id` / `job_id` /
/// `decision_outcome`), so no token, path, query, or event payload can leak through the audit.
async fn record_flight_recorder_capability_decision(
    state: &AppState,
    ctx: Option<&crate::api::stage::CaptureContext>,
    capability_id: &'static str,
    decision_outcome: &'static str,
    workspace_id: Option<String>,
) -> Result<(), crate::flight_recorder::RecorderError> {
    let trace_id = Uuid::now_v7();
    let policy_decision_id = format!("native-fr-capability:{trace_id}");
    let actor_id = ctx
        .map(|ctx| ctx.actor_id.as_str())
        .unwrap_or("unauthenticated-native-client");
    let actor = if ctx.is_some() {
        crate::flight_recorder::FlightRecorderActor::Human
    } else {
        crate::flight_recorder::FlightRecorderActor::System
    };
    let mut event = crate::flight_recorder::FlightRecorderEvent::new(
        crate::flight_recorder::FlightRecorderEventType::CapabilityAction,
        actor,
        trace_id,
        json!({
            "capability_id": capability_id,
            "actor_id": actor_id,
            "job_id": null,
            "decision_outcome": decision_outcome,
        }),
    )
    .with_actor_id(actor_id)
    .with_capability_id(capability_id)
    .with_policy_decision_id(policy_decision_id);
    if let Some(workspace_id) = workspace_id {
        event = event.with_wsids(vec![workspace_id]);
    }
    state.flight_recorder.record_event(event).await
}

/// Audit an in-handler decision (the unscoped-read escalation) and map an audit failure to a
/// fail-closed `500`.
async fn audit_recorder_decision(
    state: &AppState,
    ctx: Option<&crate::api::stage::CaptureContext>,
    capability_id: &'static str,
    decision_outcome: &'static str,
    workspace_id: Option<String>,
) -> ApiResult<()> {
    record_flight_recorder_capability_decision(
        state,
        ctx,
        capability_id,
        decision_outcome,
        workspace_id,
    )
    .await
    .map_err(|error| {
        tracing::error!(
            target: "handshake_core::flight_recorder_api",
            capability_id,
            error = ?error,
            "flight_recorder_capability_audit_failed"
        );
        audit_failed_closed()
    })
}

/// Fail-closed authorization for the WHOLE flight-recorder route group.
async fn authorize_flight_recorder_request(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path().to_owned();
    let workspace_id = workspace_id_from_recorder_path(&path);
    let Some(capability_id) = flight_recorder_capability_for_request(request.method(), &path)
    else {
        // An unmapped path under this router has no capability contract; deny rather than
        // silently pass an unclassified recorder request through.
        return capability_denied().into_response();
    };

    let ctx = match crate::api::stage::capture_context(request.headers()) {
        Ok(ctx) => ctx,
        Err(_) => {
            if record_flight_recorder_capability_decision(
                &state,
                None,
                capability_id,
                "deny",
                workspace_id,
            )
            .await
            .is_err()
            {
                return audit_failed_closed().into_response();
            }
            return unauthenticated_recorder().into_response();
        }
    };

    let allowed = state
        .capability_registry
        .profile_can(FR_CAPABILITY_PROFILE, capability_id)
        .unwrap_or(false);
    let outcome = if allowed { "allow" } else { "deny" };
    if record_flight_recorder_capability_decision(
        &state,
        Some(&ctx),
        capability_id,
        outcome,
        workspace_id.clone(),
    )
    .await
    .is_err()
    {
        return audit_failed_closed().into_response();
    }
    if !allowed {
        return capability_denied().into_response();
    }

    request
        .extensions_mut()
        .insert(RecorderAuthority { ctx, workspace_id });
    next.run(request).await
}

/// Bind the path workspace to canonical authority BEFORE any durable write. There is no
/// membership table in this product yet, so "authorized workspace" means: the authenticated
/// native binding holds the ingest capability AND the path names a real canonical workspace.
/// A client-asserted workspace that does not resolve is denied, never created implicitly.
async fn authorize_recorder_workspace(state: &AppState, workspace_id: &str) -> ApiResult<()> {
    if workspace_id.trim().is_empty() {
        return Err(workspace_denied());
    }
    match state.storage.get_workspace(workspace_id).await {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err(workspace_denied()),
        Err(error) => Err(db_error(error)),
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeChatEventType {
    RuntimeChatMessageAppended,
    RuntimeChatAns001Validation,
    RuntimeChatSessionClosed,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeChatEventV0_1 {
    pub schema_version: String,
    pub event_id: String,
    pub ts_utc: String,
    pub session_id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_packet_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spec_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wsid: Option<String>,

    #[serde(rename = "type")]
    pub event_type: RuntimeChatEventType,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_role: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ans001_sha256: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ans001_compliant: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub violation_clauses: Option<Vec<String>>,
}

/// Workspace-scoped runtime-chat ingestion.
///
/// The FR actor stays the canonical `system` / `runtime_chat` lane (the recorder contract requires
/// `runtime_chat_*` events to be system-actor, and that identifier was never client-supplied), but
/// the workspace attribution is now taken from the authenticated PATH: a body `wsid` may only
/// confirm it, never widen or redirect it.
async fn record_runtime_chat_event(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Extension(authority): Extension<RecorderAuthority>,
    Json(mut event): Json<RuntimeChatEventV0_1>,
) -> ApiResult<Json<Value>> {
    debug_assert_eq!(
        authority.workspace_id.as_deref(),
        Some(workspace_id.as_str())
    );
    if let Some(claimed) = event.wsid.as_deref() {
        if claimed.trim() != workspace_id {
            return Err(workspace_denied());
        }
    }
    authorize_recorder_workspace(&state, &workspace_id).await?;
    // Server-derived from here on; the client's spelling never reaches the recorder.
    event.wsid = Some(workspace_id.clone());

    let trace_id = match Uuid::parse_str(event.session_id.trim()) {
        Ok(parsed) if parsed != Uuid::nil() => parsed,
        _ => return Err(invalid_event()),
    };

    let event_type = match event.event_type {
        RuntimeChatEventType::RuntimeChatMessageAppended => {
            crate::flight_recorder::FlightRecorderEventType::RuntimeChatMessageAppended
        }
        RuntimeChatEventType::RuntimeChatAns001Validation => {
            crate::flight_recorder::FlightRecorderEventType::RuntimeChatAns001Validation
        }
        RuntimeChatEventType::RuntimeChatSessionClosed => {
            crate::flight_recorder::FlightRecorderEventType::RuntimeChatSessionClosed
        }
    };

    let payload = match serde_json::to_value(&event) {
        Ok(value) => value,
        Err(err) => return Err(db_error(err)),
    };
    let mut fr_event = crate::flight_recorder::FlightRecorderEvent::new(
        event_type,
        crate::flight_recorder::FlightRecorderActor::System,
        trace_id,
        payload,
    )
    .with_actor_id("runtime_chat");

    if let Some(job_id) = event.job_id {
        fr_event = fr_event.with_job_id(job_id);
    }
    if let Some(wsid) = event.wsid {
        fr_event = fr_event.with_wsids(vec![wsid]);
    }

    state
        .flight_recorder
        .record_event(fr_event)
        .await
        .map_err(|e| match e {
            crate::flight_recorder::RecorderError::InvalidEvent(_) => invalid_event(),
            other => db_error(other),
        })?;

    Ok(Json(json!({ "ok": true })))
}

/// The native-editor event schema version this ingestion endpoint accepts. Matches the
/// frontend MT-036 `NATIVE_EDITOR_SCHEMA_VERSION`.
pub const NATIVE_EDITOR_SCHEMA_VERSION: &str = "hsk.native_editor@0.1";

/// The CLOSED native-editor + interop event vocabulary this endpoint accepts. Unknown
/// kinds are rejected at decode (serde), keeping the typed-event discipline. Wire strings
/// are snake_case: the 8 `NativeEditorAction` kinds plus the 5 interop kinds the frontend
/// manifest names (its SCREAMING_SNAKE names map 1:1 to these snake_case wire kinds, e.g.
/// `STAGE_EMBED_BACK` -> `stage_embed_back`).
#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NativeEditorFrEventKind {
    // NativeEditorAction kinds (MT-036 event_emitter.rs).
    DocumentSaved,
    CodeEdit,
    EmbedCreated,
    CanvasNodePlaced,
    CrossRefInserted,
    UndoFired,
    RouteToStage,
    // Interop kinds named by the frontend other-pillar interop manifest.
    StageEmbedBack,
    CalendarEventBound,
    ActivitySpanCorrelated,
    LocusRefResolved,
    LocusReverseLookup,
}

impl NativeEditorFrEventKind {
    /// The stable snake_case wire string carried in the FR payload `kind`.
    pub fn as_str(self) -> &'static str {
        match self {
            NativeEditorFrEventKind::DocumentSaved => "document_saved",
            NativeEditorFrEventKind::CodeEdit => "code_edit",
            NativeEditorFrEventKind::EmbedCreated => "embed_created",
            NativeEditorFrEventKind::CanvasNodePlaced => "canvas_node_placed",
            NativeEditorFrEventKind::CrossRefInserted => "cross_ref_inserted",
            NativeEditorFrEventKind::UndoFired => "undo_fired",
            NativeEditorFrEventKind::RouteToStage => "route_to_stage",
            NativeEditorFrEventKind::StageEmbedBack => "stage_embed_back",
            NativeEditorFrEventKind::CalendarEventBound => "calendar_event_bound",
            NativeEditorFrEventKind::ActivitySpanCorrelated => "activity_span_correlated",
            NativeEditorFrEventKind::LocusRefResolved => "locus_ref_resolved",
            NativeEditorFrEventKind::LocusReverseLookup => "locus_reverse_lookup",
        }
    }
}

/// Which Flight Recorder actor lane a native-editor event belongs to (bounded). Defaults
/// to `human` (native editors are a human-facing surface); a swarm agent driving a native
/// editor passes `agent`.
#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NativeEditorActorKind {
    Human,
    Agent,
    System,
}

/// The versioned native-editor Flight Recorder ingestion envelope. Closed field set
/// (`deny_unknown_fields`) + closed `kind` vocabulary — free text is only allowed inside
/// the bounded, named `payload` object (rejected if it is not a JSON object).
///
/// `actor_id`, `actor_kind` and `workspace_id` are NOT client authority (see the module header):
/// they are optional confirmations of the authenticated request context. A value that disagrees
/// with the context is rejected with `403` before any durable write; an absent value is filled
/// server-side. Once canonicalized every stored envelope carries the server-derived identity, so
/// the durable EventLedger/Flight Recorder attribution can never be caller-forged.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct NativeEditorFrEventV0_1 {
    pub schema_version: String,
    pub event_id: String,
    pub ts_utc: String,
    pub kind: NativeEditorFrEventKind,
    #[serde(default)]
    pub actor_id: Option<String>,

    #[serde(default)]
    pub actor_kind: Option<NativeEditorActorKind>,
    pub pane_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface: Option<String>,
    #[serde(default)]
    pub workspace_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub work_packet_id: Option<String>,

    /// The bounded, per-kind structured payload. When present it MUST be a JSON object
    /// (no top-level free-text/array smuggling).
    #[serde(default)]
    pub payload: Value,
}

impl NativeEditorFrEventV0_1 {
    /// The server-derived actor id. Only meaningful after the handler (or the reconciler reading a
    /// canonicalized stored envelope) has bound it to the authenticated context.
    pub fn canonical_actor_id(&self) -> &str {
        self.actor_id.as_deref().unwrap_or_default()
    }

    /// The server-derived workspace id (the authenticated `:workspace_id` path segment).
    pub fn canonical_workspace_id(&self) -> &str {
        self.workspace_id.as_deref().unwrap_or_default()
    }
}

/// Namespace for the workspace-partitioned durable Flight Recorder event identity.
const NATIVE_EDITOR_EVENT_ID_NAMESPACE: &str = "hsk.native_editor.fr_event_id@1";

/// Derive the durable Flight Recorder / EventLedger identity of a native-editor event from the
/// AUTHENTICATED workspace plus the client-supplied `event_id`.
///
/// This is what partitions idempotency and conflict ownership per workspace. The client `event_id`
/// alone is caller-controlled, so using it directly as the durable key let any caller pre-seed an
/// id another workspace would later submit and convert that workspace's honest retry into a `409`,
/// or read its row back through `GET /flight_recorder?event_id=`. Mixing the authenticated
/// workspace into the digest makes the two aggregates disjoint while staying deterministic, so
/// retries and restart reconciliation still converge on exactly one row.
pub fn workspace_scoped_fr_event_id(workspace_id: &str, client_event_id: Uuid) -> Uuid {
    let mut hasher = Sha256::new();
    hasher.update(NATIVE_EDITOR_EVENT_ID_NAMESPACE.as_bytes());
    hasher.update([0u8]);
    hasher.update(workspace_id.as_bytes());
    hasher.update([0u8]);
    hasher.update(client_event_id.as_bytes());
    let digest = hasher.finalize();
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    // RFC 9562 version 8 (custom / name-based digest) + variant 10x.
    bytes[6] = (bytes[6] & 0x0f) | 0x80;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}

fn non_empty_string(map: &serde_json::Map<String, Value>, key: &str) -> bool {
    map.get(key)
        .and_then(Value::as_str)
        .is_some_and(|value| !value.trim().is_empty())
}

fn optional_non_empty_string(map: &serde_json::Map<String, Value>, key: &str) -> bool {
    map.get(key)
        .is_none_or(|value| value.as_str().is_some_and(|text| !text.trim().is_empty()))
}

fn only_keys(map: &serde_json::Map<String, Value>, allowed: &[&str]) -> bool {
    map.keys().all(|key| allowed.contains(&key.as_str()))
}

fn non_empty_string_array(map: &serde_json::Map<String, Value>, key: &str) -> bool {
    map.get(key)
        .and_then(Value::as_array)
        .is_some_and(|values| {
            !values.is_empty()
                && values
                    .iter()
                    .all(|value| value.as_str().is_some_and(|text| !text.trim().is_empty()))
        })
}

fn sha256_string(map: &serde_json::Map<String, Value>, key: &str) -> bool {
    map.get(key)
        .and_then(Value::as_str)
        .is_some_and(|hash| hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()))
}

/// Validate the per-kind payload contract for [`NATIVE_EDITOR_SCHEMA_VERSION`]. Stage route and
/// embed-back receipts may carry the same `causal_action_id`; legacy uncorrelated 0.1 producers remain
/// valid, while a present correlation must be a non-blank string and every other field stays closed.
fn valid_native_editor_payload(kind: NativeEditorFrEventKind, payload: &Value) -> bool {
    let Some(map) = payload.as_object() else {
        return false;
    };
    match kind {
        NativeEditorFrEventKind::DocumentSaved => {
            only_keys(
                map,
                &[
                    "document_id",
                    "content_hash",
                    "save_receipt_event_id",
                    "actor_kind",
                    "kernel_task_run_id",
                    "session_run_id",
                    "correlation_id",
                ],
            ) && non_empty_string(map, "document_id")
                && sha256_string(map, "content_hash")
                && [
                    "save_receipt_event_id",
                    "actor_kind",
                    "kernel_task_run_id",
                    "session_run_id",
                ]
                .iter()
                .all(|key| non_empty_string(map, key))
                && non_empty_string(map, "correlation_id")
        }
        NativeEditorFrEventKind::CodeEdit => {
            only_keys(map, &["file_path", "line_delta"])
                && non_empty_string(map, "file_path")
                && map.get("line_delta").and_then(Value::as_i64).is_some()
        }
        NativeEditorFrEventKind::EmbedCreated => {
            only_keys(map, &["embed_kind", "item_id", "target_document_id"])
                && non_empty_string(map, "embed_kind")
                && non_empty_string(map, "item_id")
                && non_empty_string(map, "target_document_id")
        }
        NativeEditorFrEventKind::CanvasNodePlaced => {
            only_keys(map, &["canvas_id", "node_id", "node_kind"])
                && non_empty_string(map, "canvas_id")
                && non_empty_string(map, "node_id")
                && non_empty_string(map, "node_kind")
        }
        NativeEditorFrEventKind::CrossRefInserted => {
            only_keys(map, &["ref_kind", "symbol_entity_id", "target_document_id"])
                && non_empty_string(map, "ref_kind")
                && non_empty_string(map, "symbol_entity_id")
                && non_empty_string(map, "target_document_id")
        }
        NativeEditorFrEventKind::UndoFired => {
            only_keys(map, &["scope"])
                && matches!(
                    map.get("scope").and_then(Value::as_str),
                    Some("local" | "cross_pane")
                )
        }
        NativeEditorFrEventKind::RouteToStage => {
            only_keys(map, &["content_kind", "causal_action_id"])
                && non_empty_string(map, "content_kind")
                && optional_non_empty_string(map, "causal_action_id")
        }
        NativeEditorFrEventKind::StageEmbedBack => {
            only_keys(
                map,
                &[
                    "artifact_id",
                    "target_pane_id",
                    "sha256",
                    "manifest_ref",
                    "causal_action_id",
                ],
            ) && non_empty_string(map, "artifact_id")
                && non_empty_string(map, "target_pane_id")
                && sha256_string(map, "sha256")
                && non_empty_string(map, "manifest_ref")
                && optional_non_empty_string(map, "causal_action_id")
        }
        NativeEditorFrEventKind::CalendarEventBound => {
            only_keys(map, &["date", "document_id", "calendar_event_id"])
                && non_empty_string(map, "date")
                && NaiveDate::parse_from_str(
                    map.get("date").and_then(Value::as_str).unwrap_or_default(),
                    "%Y-%m-%d",
                )
                .is_ok()
                && non_empty_string(map, "document_id")
                && non_empty_string(map, "calendar_event_id")
        }
        NativeEditorFrEventKind::ActivitySpanCorrelated => {
            only_keys(
                map,
                &[
                    "calendar_event_id",
                    "activity_span_id",
                    "edited_document_ids",
                ],
            ) && non_empty_string(map, "calendar_event_id")
                && non_empty_string(map, "activity_span_id")
                && non_empty_string_array(map, "edited_document_ids")
        }
        NativeEditorFrEventKind::LocusRefResolved => {
            only_keys(map, &["locus_uri", "target_kind", "target_id"])
                && non_empty_string(map, "locus_uri")
                && matches!(
                    map.get("target_kind").and_then(Value::as_str),
                    Some("work_packet" | "microtask")
                )
                && non_empty_string(map, "target_id")
        }
        NativeEditorFrEventKind::LocusReverseLookup => {
            only_keys(map, &["locus_uri", "document_ids"])
                && non_empty_string(map, "locus_uri")
                && non_empty_string_array(map, "document_ids")
        }
    }
}

fn canonical_receipt_actor_kind(request_actor_kind: &str) -> Option<&'static str> {
    match request_actor_kind {
        "operator" => Some("operator"),
        "system" => Some("system"),
        "validator" => Some("validation_runner"),
        "local_model" | "cloud_model" => Some("model_adapter"),
        _ => None,
    }
}

/// Authenticate a native `document_saved` claim against the immutable EventLedger row minted by the
/// canonical knowledge-document save. A caller-provided UUID or a receipt for another document,
/// workspace, actor, run, correlation, or content hash is rejected before any native ledger/FR write.
async fn validate_document_save_receipt(
    state: &AppState,
    event: &NativeEditorFrEventV0_1,
) -> ApiResult<()> {
    let payload = event.payload.as_object().ok_or_else(invalid_event)?;
    let receipt_id = payload
        .get("save_receipt_event_id")
        .and_then(Value::as_str)
        .ok_or_else(invalid_event)?;
    let document_id = payload
        .get("document_id")
        .and_then(Value::as_str)
        .ok_or_else(invalid_event)?;
    let row = state
        .storage
        .list_kernel_events_for_aggregate("knowledge_rich_document", document_id)
        .await
        .map_err(db_error)?
        .into_iter()
        .find(|row| row.event_id == receipt_id)
        .ok_or_else(invalid_event)?;

    let receipt_payload = row.payload.as_object().ok_or_else(invalid_event)?;
    let request_actor_kind = payload
        .get("actor_kind")
        .and_then(Value::as_str)
        .and_then(canonical_receipt_actor_kind)
        .ok_or_else(invalid_event)?;
    let correlation = row.correlation_id.as_deref();
    let claimed_correlation = payload
        .get("correlation_id")
        .and_then(|value| value.as_str().map(str::to_owned));

    // WP-KERNEL-012 MT-120 — receipt OWNERSHIP anchor.
    //
    // This is a STRENGTHENING, not a relaxation. The comparison used to read the ledger `actor_id`
    // COLUMN, which is the CLIENT-declared per-agent attribution supplied by the save request header,
    // while `event.canonical_actor_id()` is the SERVER-derived `handshake-native:{pid}:{fingerprint}`
    // principal. Those two values can never be equal for a real product save, so the clause rejected
    // every legitimate save: a guard that always fails is an outage wearing a security costume, and it
    // enforces nothing. The comparison now reads a SERVER-WRITTEN receipt-payload field that only an
    // authenticated document-save can produce (`knowledge_documents::save_document` stamps it from
    // `stage::capture_context`, never from the request body). That makes the invariant MT-109 wants —
    // a save receipt is claimable only by the principal that minted it — actually enforceable, while
    // per-agent attribution survives untouched in the `actor_id` column.
    //
    // Fail-closed: an absent, non-string, or blank field is UNCLAIMABLE, and a blank claiming actor
    // can never match a blank minted principal. This stays an UNCONDITIONAL conjunct below.
    let minted_by_principal = receipt_payload
        .get(crate::api::knowledge_documents::SAVE_RECEIPT_MINTED_BY_PRINCIPAL_FIELD)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|principal| !principal.is_empty());
    let claiming_principal = Some(event.canonical_actor_id().trim()).filter(|id| !id.is_empty());
    let receipt_owned_by_claimant = matches!(
        (minted_by_principal, claiming_principal),
        (Some(minted), Some(claiming)) if minted == claiming
    );

    let authentic = row.event_type == KernelEventType::KnowledgeRichDocumentSaved
        && row.aggregate_type == "knowledge_rich_document"
        && row.aggregate_id == document_id
        && row.actor.actor_kind() == request_actor_kind
        // The canonical save receipt must belong to the SAME authenticated principal that is now
        // claiming it: `event.actor_id` is server-derived by this point, and `minted_by_principal`
        // is server-written by the save route, so a caller cannot bind its native event to another
        // principal's document-save receipt. See the MT-120 note above.
        && receipt_owned_by_claimant
        && row.kernel_task_run_id == payload["kernel_task_run_id"].as_str().unwrap_or_default()
        && row.session_run_id == payload["session_run_id"].as_str().unwrap_or_default()
        && correlation == claimed_correlation.as_deref()
        && receipt_payload.get("event").and_then(Value::as_str) == Some("saved")
        && receipt_payload.get("workspace_id").and_then(Value::as_str)
            == Some(event.canonical_workspace_id())
        && receipt_payload.get("content_hash").and_then(Value::as_str)
            == payload.get("content_hash").and_then(Value::as_str);
    if !authentic {
        return Err(invalid_event());
    }
    Ok(())
}

fn map_recorder_err(err: crate::flight_recorder::RecorderError) -> ApiError {
    match err {
        crate::flight_recorder::RecorderError::InvalidEvent(_) => invalid_event(),
        other => db_error(other),
    }
}

fn native_editor_fr_event_from_envelope(
    event: &NativeEditorFrEventV0_1,
) -> ApiResult<crate::flight_recorder::FlightRecorderEvent> {
    let client_event_uuid = Uuid::parse_str(event.event_id.trim()).map_err(|_| invalid_event())?;
    // A canonicalized envelope always carries the server-derived actor/workspace. If either is
    // missing the envelope never passed the authorization boundary and must not become an FR row.
    let workspace_id = event
        .workspace_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(invalid_event)?;
    let actor_id = event
        .actor_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(invalid_event)?;
    let event_uuid = workspace_scoped_fr_event_id(workspace_id, client_event_uuid);
    let timestamp = DateTime::parse_from_rfc3339(event.ts_utc.trim())
        .map_err(|_| invalid_event())?
        .with_timezone(&Utc);
    let trace_id = event
        .session_id
        .as_deref()
        .and_then(|session| Uuid::parse_str(session.trim()).ok())
        .filter(|id| *id != Uuid::nil())
        .unwrap_or(event_uuid);
    let actor = match event.actor_kind.unwrap_or(NativeEditorActorKind::Human) {
        NativeEditorActorKind::Human => crate::flight_recorder::FlightRecorderActor::Human,
        NativeEditorActorKind::Agent => crate::flight_recorder::FlightRecorderActor::Agent,
        NativeEditorActorKind::System => crate::flight_recorder::FlightRecorderActor::System,
    };
    let kind = event.kind.as_str();
    let surface = event
        .surface
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| event.pane_id.clone());
    let payload = json!({
        "event_family": "native_editor",
        "schema": NATIVE_EDITOR_SCHEMA_VERSION,
        "schema_version": NATIVE_EDITOR_SCHEMA_VERSION,
        "action": kind,
        "kind": kind,
        "editor_surface": surface,
        "ops": [ { "kind": kind, "payload": event.payload.clone() } ],
        "pane_id": event.pane_id,
        "workspace_id": workspace_id,
        "actor_id": actor_id,
        // The caller's own correlation id, preserved for client-side reconciliation. It is NOT the
        // durable identity: `fr_event.event_id` is the workspace-partitioned derivation.
        "client_event_id": client_event_uuid.to_string(),
        // Keep the payload spelling identical to the typed TIMESTAMPTZ identity. A partial FR write
        // built from an equivalent RFC3339 spelling (`Z` versus `+00:00`) must converge with the
        // canonical retry instead of conflicting with itself.
        "ts_utc": timestamp.to_rfc3339(),
        "native_payload": event.payload,
    });
    let mut fr_event = crate::flight_recorder::FlightRecorderEvent::new(
        crate::flight_recorder::FlightRecorderEventType::System,
        actor,
        trace_id,
        payload,
    )
    .with_actor_id(actor_id)
    .with_wsids(vec![workspace_id.to_owned()]);
    fr_event.event_id = event_uuid;
    fr_event.timestamp = timestamp;
    if let Some(session_id) = event.session_id.clone() {
        fr_event = fr_event.with_session_span(session_id);
    }
    // The DuckDB authority normalizes string content before insert. Normalize the comparison copy too,
    // otherwise a valid decomposed-Unicode envelope would insert successfully and conflict with its
    // own retry after the stored row had been normalized.
    fr_event.normalize_payload();
    Ok(fr_event)
}

fn native_editor_fr_event_matches(
    stored: &crate::flight_recorder::FlightRecorderEvent,
    expected: &crate::flight_recorder::FlightRecorderEvent,
) -> bool {
    stored.event_id == expected.event_id
        && stored.trace_id == expected.trace_id
        // DuckDB's TIMESTAMPTZ authority stores microseconds, while RFC3339 inputs and chrono can
        // carry nanoseconds. Requiring nanosecond equality makes an event conflict with its own
        // readback whenever the caller supplied sub-microsecond precision because readback uses the
        // recorder's integer microsecond boundary. The immutable payload below still compares the
        // original `ts_utc` exactly, so comparing the typed timestamp at the recorder's actual
        // precision does not weaken envelope identity.
        && stored.timestamp.timestamp_micros() == expected.timestamp.timestamp_micros()
        && stored.actor == expected.actor
        && stored.actor_id == expected.actor_id
        && stored.event_type == expected.event_type
        && stored.job_id == expected.job_id
        && stored.workflow_id == expected.workflow_id
        && stored.model_id == expected.model_id
        && stored.model_session_id == expected.model_session_id
        && stored.wsids == expected.wsids
        && stored.activity_span_id == expected.activity_span_id
        && stored.session_span_id == expected.session_span_id
        && stored.capability_id == expected.capability_id
        && stored.policy_decision_id == expected.policy_decision_id
        && stored.payload == expected.payload
}

fn spawn_native_editor_reconciler(state: AppState) {
    let Ok(runtime) = tokio::runtime::Handle::try_current() else {
        return;
    };
    runtime.spawn(async move {
        loop {
            if let Err(error) = reconcile_native_editor_pending(&state).await {
                tracing::warn!(error = %error, "native-editor Flight Recorder reconciliation pass failed");
            }
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    });
}

async fn reconcile_native_editor_pending(state: &AppState) -> Result<(), String> {
    let mut after_event_sequence = 0_i64;
    loop {
        let pending = state
            .storage
            .list_pending_native_editor_mirrors(after_event_sequence, 100)
            .await
            .map_err(|error| error.to_string())?;
        if pending.is_empty() {
            break;
        }
        let batch_len = pending.len();
        for pending_receipt in pending {
            after_event_sequence = after_event_sequence.max(pending_receipt.event_sequence);
            if let Err(error) =
                reconcile_native_editor_pending_receipt(state, pending_receipt).await
            {
                // A malformed/conflicting poison row remains operator-visible in logs. Keyset
                // pagination advances past it, so even 100+ permanent poison rows cannot starve a
                // newer valid mirror.
                tracing::error!(error = %error, "native-editor pending mirror could not reconcile");
            }
        }
        if batch_len < 100 {
            break;
        }
        tokio::task::yield_now().await;
    }
    Ok(())
}

fn native_editor_completion_matches(
    pending: &crate::kernel::KernelEvent,
    completed: &crate::kernel::KernelEvent,
) -> bool {
    let Some(envelope_value) = pending.payload.get("envelope") else {
        return false;
    };
    let Ok(envelope) = serde_json::from_value::<NativeEditorFrEventV0_1>(envelope_value.clone())
    else {
        return false;
    };
    let Ok(expected) = build_native_editor_completion(pending, &envelope) else {
        return false;
    };
    kernel_event_matches_new(completed, &expected)
}

fn kernel_event_matches_new(
    stored: &crate::kernel::KernelEvent,
    expected: &NewKernelEvent,
) -> bool {
    stored.event_version == expected.event_version
        && stored.kernel_task_run_id == expected.kernel_task_run_id
        && stored.session_run_id == expected.session_run_id
        && stored.aggregate_type == expected.aggregate_type
        && stored.aggregate_id == expected.aggregate_id
        && stored.idempotency_key == expected.idempotency_key
        && stored.event_type == expected.event_type
        && stored.actor == expected.actor
        && stored.causation_id == expected.causation_id
        && stored.correlation_id == expected.correlation_id
        && stored.payload_hash == expected.payload_hash
        && stored.source_component == expected.source_component
        && stored.payload == expected.payload
}

fn native_editor_completion_payload(envelope: &NativeEditorFrEventV0_1) -> Value {
    // `fr_event_id` is the DURABLE workspace-partitioned id actually written to the recorder; the
    // caller's own id is preserved separately so client-side reconciliation still has its handle.
    let fr_event_id = Uuid::parse_str(envelope.event_id.trim())
        .map(|client_event_id| {
            workspace_scoped_fr_event_id(envelope.canonical_workspace_id(), client_event_id)
                .to_string()
        })
        .unwrap_or_default();
    json!({
        "receipt_kind": "native_editor_flight_recorder_recorded",
        "fr_event_id": fr_event_id,
        "client_event_id": envelope.event_id,
        "fr_event_type": "system",
        "envelope": envelope,
    })
}

fn native_editor_completion_payload_hash(envelope: &NativeEditorFrEventV0_1) -> String {
    crate::kernel::context_bundle::sha256_hex(&crate::kernel::context_bundle::canonical_json_bytes(
        &native_editor_completion_payload(envelope),
    ))
}

fn build_native_editor_pending(
    envelope: &NativeEditorFrEventV0_1,
    expected_fr: &crate::flight_recorder::FlightRecorderEvent,
) -> Result<NewKernelEvent, String> {
    let actor_id = envelope.canonical_actor_id().to_owned();
    let actor = match &expected_fr.actor {
        crate::flight_recorder::FlightRecorderActor::Human => KernelActor::Operator(actor_id),
        _ => KernelActor::System(actor_id),
    };
    let workspace_id = envelope.canonical_workspace_id().to_owned();
    let kernel_task_run_id = envelope
        .work_packet_id
        .clone()
        .unwrap_or_else(|| workspace_id.clone());
    let session_run_id = envelope
        .session_id
        .clone()
        .unwrap_or_else(|| envelope.event_id.clone());
    // Aggregate identity and idempotency are BOTH workspace-partitioned. `expected_fr.event_id` is
    // the workspace-scoped derivation, and the idempotency key names the authenticated workspace
    // explicitly so an operator reading the ledger can see the tenant boundary directly.
    let durable_event_id = expected_fr.event_id.to_string();

    NewKernelEvent::builder(
        kernel_task_run_id,
        session_run_id,
        KernelEventType::FlightRecorderMirrorPending,
        actor,
    )
    .aggregate("native_editor_event", durable_event_id)
    .idempotency_key(format!(
        "native-editor-fr-pending:{workspace_id}:{}",
        envelope.event_id
    ))
    .source_component("native_editor_fr_ingestion")
    .correlation_id(expected_fr.trace_id.to_string())
    .payload(json!({
        "receipt_kind": "native_editor_flight_recorder_pending",
        "expected_completion_payload_hash": native_editor_completion_payload_hash(envelope),
        "envelope": envelope,
    }))
    .build()
    .map_err(|error| error.to_string())
}

fn native_editor_pending_receipt_matches(
    pending: &crate::kernel::KernelEvent,
    envelope: &NativeEditorFrEventV0_1,
    expected_fr: &crate::flight_recorder::FlightRecorderEvent,
) -> bool {
    let Ok(mut expected) = build_native_editor_pending(envelope, expected_fr) else {
        return false;
    };
    if kernel_event_matches_new(pending, &expected) {
        return true;
    }
    // Pre-hardening pending receipts did not carry the expected completion hash.
    // Keep them recoverable without rewriting append-only rows: reconstruct their
    // original closed payload and require the same exact immutable-event match.
    if pending
        .payload
        .get("expected_completion_payload_hash")
        .is_none()
    {
        if let Value::Object(payload) = &mut expected.payload {
            payload.remove("expected_completion_payload_hash");
        }
        expected.payload_hash = crate::kernel::context_bundle::sha256_hex(
            &crate::kernel::context_bundle::canonical_json_bytes(&expected.payload),
        );
        return kernel_event_matches_new(pending, &expected);
    }
    false
}

fn build_native_editor_completion(
    pending_receipt: &crate::kernel::KernelEvent,
    envelope: &NativeEditorFrEventV0_1,
) -> Result<NewKernelEvent, String> {
    NewKernelEvent::builder(
        pending_receipt.kernel_task_run_id.clone(),
        pending_receipt.session_run_id.clone(),
        KernelEventType::FlightRecorderMirrorRecorded,
        pending_receipt.actor.clone(),
    )
    // Reuse the pending receipt's already workspace-partitioned aggregate identity so a caller can
    // never complete or reconcile another workspace's pending mirror.
    .aggregate("native_editor_event", pending_receipt.aggregate_id.clone())
    .idempotency_key(format!(
        "native-editor-fr-complete:{}:{}",
        envelope.canonical_workspace_id(),
        envelope.event_id
    ))
    .source_component("native_editor_fr_ingestion")
    .causation_id(pending_receipt.event_id.clone())
    .correlation_id(
        pending_receipt
            .correlation_id
            .clone()
            .unwrap_or_else(|| envelope.event_id.clone()),
    )
    .payload(native_editor_completion_payload(envelope))
    .build()
    .map_err(|error| error.to_string())
}

async fn reconcile_native_editor_pending_receipt(
    state: &AppState,
    pending_receipt: crate::kernel::KernelEvent,
) -> Result<(), String> {
    let aggregate_events = state
        .storage
        .list_kernel_events_for_aggregate(
            &pending_receipt.aggregate_type,
            &pending_receipt.aggregate_id,
        )
        .await
        .map_err(|error| error.to_string())?;
    if aggregate_events
        .iter()
        .any(|event| native_editor_completion_matches(&pending_receipt, event))
    {
        return Ok(());
    }
    let envelope: NativeEditorFrEventV0_1 = serde_json::from_value(
        pending_receipt
            .payload
            .get("envelope")
            .cloned()
            .ok_or_else(|| "pending native-editor receipt lacks envelope".to_owned())?,
    )
    .map_err(|error| error.to_string())?;
    let expected = native_editor_fr_event_from_envelope(&envelope)
        .map_err(|_| "pending native-editor envelope is invalid".to_owned())?;
    if !native_editor_pending_receipt_matches(&pending_receipt, &envelope, &expected) {
        return Err(format!(
            "native-editor pending receipt {} is not authentic",
            pending_receipt.event_id
        ));
    }
    let existing = state
        .flight_recorder
        .list_events(crate::flight_recorder::EventFilter {
            event_id: Some(expected.event_id),
            ..Default::default()
        })
        .await
        .map_err(|error| error.to_string())?;
    if let Some(stored) = existing.first() {
        if !native_editor_fr_event_matches(stored, &expected) {
            return Err(format!(
                "native-editor pending receipt {} conflicts with existing Flight Recorder row",
                envelope.event_id
            ));
        }
    } else {
        state
            .flight_recorder
            .record_event(expected)
            .await
            .map_err(|error| format!("native-editor pending mirror remains queued: {error}"))?;
    }

    let completion = build_native_editor_completion(&pending_receipt, &envelope)?;
    state
        .storage
        .append_kernel_event(completion)
        .await
        .map_err(|error| error.to_string())?;
    Ok(())
}

/// AC-109-1: accept a versioned native-editor event envelope, land it in the FR authority
/// store (readable via the existing GET route), idempotent on the workspace-partitioned durable
/// event id, and durably mirror it into the kernel EventLedger.
///
/// Authorization is enforced twice over: [`authorize_flight_recorder_request`] already proved the
/// caller holds `fr.ingest.native_editor` on a live binding, and this handler then binds actor and
/// workspace attribution to that authenticated context BEFORE the first durable write.
async fn record_native_editor_event(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Extension(authority): Extension<RecorderAuthority>,
    Json(mut event): Json<NativeEditorFrEventV0_1>,
) -> ApiResult<Json<Value>> {
    debug_assert_eq!(
        authority.workspace_id.as_deref(),
        Some(workspace_id.as_str())
    );

    // ---- Attribution binding (before ANY durable mutation) --------------------------------
    // A body-supplied actor/workspace may only CONFIRM the authenticated context; it can never
    // set, widen, or redirect it.
    let authenticated_actor_id = authority.ctx.actor_id.clone();
    let authenticated_actor_kind = authority.native_editor_actor_kind();
    if let Some(claimed_actor_id) = event.actor_id.as_deref() {
        if claimed_actor_id.trim() != authenticated_actor_id {
            return Err(actor_spoof_denied());
        }
    }
    if let Some(claimed_actor_kind) = event.actor_kind {
        if claimed_actor_kind != authenticated_actor_kind {
            return Err(actor_spoof_denied());
        }
    }
    if let Some(claimed_workspace_id) = event.workspace_id.as_deref() {
        if claimed_workspace_id.trim() != workspace_id {
            return Err(workspace_denied());
        }
    }
    authorize_recorder_workspace(&state, &workspace_id).await?;

    if event.schema_version.trim() != NATIVE_EDITOR_SCHEMA_VERSION {
        return Err(invalid_event());
    }
    let event_uuid = match Uuid::parse_str(event.event_id.trim()) {
        Ok(id) if id != Uuid::nil() => id,
        _ => return Err(invalid_event()),
    };
    if event.pane_id.trim().is_empty() {
        return Err(invalid_event());
    }
    let timestamp = DateTime::parse_from_rfc3339(event.ts_utc.trim())
        .map_err(|_| invalid_event())?
        .with_timezone(&Utc);
    let session_id = match event.session_id.as_deref() {
        Some(raw) => match Uuid::parse_str(raw.trim()) {
            Ok(id) if id != Uuid::nil() => Some(id.to_string()),
            _ => return Err(invalid_event()),
        },
        None => None,
    };
    if !valid_native_editor_payload(event.kind, &event.payload) {
        return Err(invalid_event());
    }

    // Canonicalize every common envelope identity before the first durable write. Equivalent lexical
    // UUID/timestamp spellings must converge on one embedded aggregate/idempotency state machine and
    // one Flight Recorder UUID, never create parallel mirrors for the same logical event.
    //
    // Actor and workspace are OVERWRITTEN from the authenticated context here, so everything
    // downstream — the save-receipt authentication, the pending receipt, the FR row, the derived
    // durable id — reads server-derived identity only.
    event.schema_version = NATIVE_EDITOR_SCHEMA_VERSION.to_owned();
    event.event_id = event_uuid.to_string();
    event.ts_utc = timestamp.to_rfc3339();
    event.actor_id = Some(authenticated_actor_id);
    event.actor_kind = Some(authenticated_actor_kind);
    event.workspace_id = Some(workspace_id.clone());
    event.pane_id = event.pane_id.trim().to_owned();
    event.surface = event
        .surface
        .take()
        .and_then(|surface| (!surface.trim().is_empty()).then(|| surface.trim().to_owned()));
    event.session_id = session_id;
    event.work_packet_id = event.work_packet_id.take().and_then(|work_packet| {
        (!work_packet.trim().is_empty()).then(|| work_packet.trim().to_owned())
    });

    // Runs AFTER canonicalization so the receipt is authenticated against the server-derived
    // actor/workspace, not against anything the caller claimed.
    if event.kind == NativeEditorFrEventKind::DocumentSaved {
        validate_document_save_receipt(&state, &event).await?;
    }
    if serde_json::to_vec(&event.payload)
        .map(|bytes| bytes.len() > 64 * 1024)
        .unwrap_or(true)
    {
        return Err(invalid_event());
    }

    let kind_str = event.kind.as_str();
    let fr_event = native_editor_fr_event_from_envelope(&event)?;
    // The durable, workspace-partitioned recorder identity. Every idempotency/conflict probe below
    // uses THIS id, never the caller-controlled `event_uuid`.
    let durable_event_uuid = fr_event.event_id;
    // Durable EventLedger mirror is written FIRST and is idempotent. If the subsequent FR write fails,
    // a retry converges by reusing this receipt and completing the missing FR row. The former FR-first
    // ordering could strand an FR row forever without its required EventLedger mirror.
    let pending_receipt = build_native_editor_pending(&event, &fr_event).map_err(db_error)?;

    let pending_receipt = state
        .storage
        .append_kernel_event(pending_receipt)
        .await
        .map_err(|error| match error {
            crate::storage::StorageError::Validation(_) => event_conflict(),
            other => db_error(other),
        })?;
    if !native_editor_pending_receipt_matches(&pending_receipt, &event, &fr_event) {
        return Err(event_conflict());
    }

    // Idempotency covers BOTH stores. After the ledger receipt exists, either reuse the existing FR row
    // or create it. A concurrent same-id insertion is re-observed and treated as idempotent.
    let existing = state
        .flight_recorder
        .list_events(crate::flight_recorder::EventFilter {
            event_id: Some(durable_event_uuid),
            ..Default::default()
        })
        .await
        .map_err(map_recorder_err)?;
    let mut idempotent = false;
    if let Some(stored) = existing.first() {
        if !native_editor_fr_event_matches(stored, &fr_event) {
            return Err(event_conflict());
        }
        idempotent = true;
    } else {
        if let Err(error) = state.flight_recorder.record_event(fr_event).await {
            let raced = state
                .flight_recorder
                .list_events(crate::flight_recorder::EventFilter {
                    event_id: Some(durable_event_uuid),
                    ..Default::default()
                })
                .await
                .map_err(map_recorder_err)?;
            let Some(stored) = raced.first() else {
                return Err(map_recorder_err(error));
            };
            let expected = native_editor_fr_event_from_envelope(&event)?;
            if !native_editor_fr_event_matches(stored, &expected) {
                return Err(event_conflict());
            }
            idempotent = true;
        }
    }

    // Only the post-mirror completion receipt may claim `MIRROR_RECORDED`. A crash after the pending
    // row or after the FR write is repaired by the startup reconciler, which reuses the full immutable
    // envelope and appends this completion only after an exact FR row exists.
    let completion = build_native_editor_completion(&pending_receipt, &event).map_err(db_error)?;
    state
        .storage
        .append_kernel_event(completion)
        .await
        .map_err(|error| match error {
            crate::storage::StorageError::Validation(_) => event_conflict(),
            other => db_error(other),
        })?;

    Ok(Json(json!({
        "ok": true,
        // The caller's own id, echoed canonicalized...
        "event_id": event.event_id,
        // ...and the durable workspace-partitioned id the recorder actually stores. `GET
        // /flight_recorder?event_id=` reads back on THIS id.
        "fr_event_id": durable_event_uuid.to_string(),
        "workspace_id": workspace_id,
        "actor_id": event.canonical_actor_id(),
        "kind": kind_str,
        "idempotent": idempotent,
    })))
}

fn system_event_matches_surface(
    event: &crate::flight_recorder::FlightRecorderEvent,
    target: &str,
) -> bool {
    target == "system"
        || (event.payload.get("event_family").and_then(Value::as_str) == Some("native_editor")
            && event.payload.get("editor_surface").and_then(Value::as_str) == Some(target))
}

/// Recorder read.
///
/// The middleware already proved `fr.read`. This handler adds the SCOPE decision: a caller that
/// omits `?wsid=` is asking to enumerate every workspace, which requires the strictly higher
/// `fr.read.global`. Otherwise the authenticated workspace scope is applied unconditionally, after
/// every other filter, so no combination of `actor`/`surface`/`event_type`/`event_id` query
/// parameters can reach a row outside it. A cross-workspace `event_id` therefore returns an empty
/// list rather than an error, so the route never discloses that a protected event exists.
async fn list_events(
    State(state): State<AppState>,
    Extension(authority): Extension<RecorderAuthority>,
    Query(filter): Query<EventFilter>,
) -> ApiResult<Json<Vec<FlightEvent>>> {
    let workspace_scope = filter
        .wsid
        .as_deref()
        .map(str::trim)
        .filter(|wsid| !wsid.is_empty())
        .map(str::to_owned);
    if workspace_scope.is_none() {
        let global_allowed = state
            .capability_registry
            .profile_can(FR_CAPABILITY_PROFILE, FR_READ_GLOBAL_CAPABILITY)
            .unwrap_or(false);
        let outcome = if global_allowed { "allow" } else { "deny" };
        audit_recorder_decision(
            &state,
            Some(&authority.ctx),
            FR_READ_GLOBAL_CAPABILITY,
            outcome,
            None,
        )
        .await?;
        if !global_allowed {
            return Err(capability_denied());
        }
    }

    let actor_lane = filter
        .actor
        .as_ref()
        .filter(|actor| matches!(actor.as_str(), "human" | "agent" | "system"))
        .cloned();
    let actor_id_from_legacy_actor = filter
        .actor
        .as_ref()
        .filter(|actor| !matches!(actor.as_str(), "human" | "agent" | "system"))
        .cloned();
    let internal_filter = crate::flight_recorder::EventFilter {
        event_id: filter.event_id,
        job_id: filter.job_id,
        trace_id: filter.trace_id,
        model_session_id: filter.model_session_id,
        from: filter.from,
        to: filter.to,
        actor: actor_lane,
        actor_id: filter.actor_id.clone().or(actor_id_from_legacy_actor),
        surface: filter.surface.clone(),
        event_type: filter.event_type.clone(),
        // Authoritative scope, not the raw query value (a blank `?wsid=` must not read as "all").
        wsid: workspace_scope.clone(),
    };

    let mut events = state
        .flight_recorder
        .list_events(internal_filter)
        .await
        .map_err(db_error)?;

    if let Some(actor) = filter.actor {
        // Backward-compatible query contract: canonical lane tokens filter the actor lane; any other
        // actor token is an actor-id lookup (the MT-036 proof uses `actor=native_editor_human`). New
        // callers may use the explicit `actor_id` parameter below.
        if matches!(actor.as_str(), "human" | "agent" | "system") {
            events.retain(|e| e.actor.to_string() == actor);
        } else {
            events.retain(|e| e.actor_id == actor);
        }
    }
    if let Some(actor_id) = filter.actor_id {
        events.retain(|e| e.actor_id == actor_id);
    }
    if let Some(kind) = filter.event_type {
        events.retain(|e| e.event_type.to_string() == kind);
    }
    if let Some(surface) = filter.surface {
        let target = surface.as_str();
        let mut filtered = Vec::new();
        for event in events {
            let surface_match = match event.event_type {
                crate::flight_recorder::FlightRecorderEventType::Diagnostic => {
                    let diag_id = event
                        .payload
                        .get("diagnostic_id")
                        .and_then(|v| v.as_str())
                        .and_then(|raw| Uuid::parse_str(raw).ok());

                    match diag_id {
                        Some(id) => match state.diagnostics.get_diagnostic(id).await {
                            Ok(diag) => diag.surface.as_str() == target,
                            Err(_) => false,
                        },
                        None => false,
                    }
                }
                crate::flight_recorder::FlightRecorderEventType::TerminalCommand => {
                    target == "system" || target == "terminal"
                }
                crate::flight_recorder::FlightRecorderEventType::EditorEdit => {
                    if target == "system" {
                        true
                    } else {
                        matches!(
                            event.payload.get("editor_surface").and_then(|v| v.as_str()),
                            Some(surface) if surface == target
                        )
                    }
                }
                crate::flight_recorder::FlightRecorderEventType::System => {
                    system_event_matches_surface(&event, target)
                }
                _ => target == "system",
            };

            if surface_match {
                filtered.push(event);
            }
        }
        events = filtered;
    }

    // LAST and unconditional: the authenticated workspace scope. Applying it after every other
    // filter is what makes filter-bypass impossible — a scoped caller cannot widen the result set
    // with any query parameter, and an event carrying no workspace attribution is excluded.
    if let Some(wsid) = workspace_scope.as_ref() {
        events.retain(|event| event.wsids.iter().any(|candidate| candidate == wsid));
    }

    let api_events = events
        .into_iter()
        .map(|e| FlightEvent {
            event_id: e.event_id.to_string(),
            trace_id: e.trace_id.to_string(),
            timestamp: e.timestamp.to_rfc3339(),
            actor: e.actor.to_string(),
            actor_id: e.actor_id,
            event_type: e.event_type.to_string(),
            job_id: e.job_id,
            workflow_id: e.workflow_id,
            model_id: e.model_id,
            model_session_id: e.model_session_id,
            wsids: e.wsids,
            activity_span_id: e.activity_span_id,
            session_span_id: e.session_span_id,
            capability_id: e.capability_id,
            policy_decision_id: e.policy_decision_id,
            payload: e.payload,
        })
        .collect();

    Ok(Json(api_events))
}

#[cfg(all(test, feature = "duckdb-flight-recorder"))]
mod tests {
    use super::*;
    use crate::capabilities::CapabilityRegistry;
    use crate::flight_recorder::duckdb::DuckDbFlightRecorder;
    use crate::flight_recorder::{
        FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType, RecorderError,
    };
    use crate::llm::{
        CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile, TokenUsage,
    };
    use crate::storage::{tests::embedded_test_backend, Database};
    use crate::workflows::{SessionRegistry, SessionSchedulerConfig};
    use crate::AppState;
    use axum::http::{HeaderMap, HeaderValue};
    use std::sync::Arc;
    use surrealdb::types::{RecordId, SurrealValue};
    use uuid::Uuid;

    /// The canonical fixture workspace every unqualified native-editor fixture is bound to.
    const TEST_WORKSPACE_ID: &str = "WS-NE-1";
    /// A second real workspace used to prove the cross-workspace boundary.
    const OTHER_TEST_WORKSPACE_ID: &str = "WS-NE-2";
    /// The server-derived actor identity the authenticated context yields in these tests. Fixtures
    /// never set `actor_id` themselves — that is the point of the MT-109 remediation.
    const TEST_ACTOR_ID: &str = "handshake-native:mt109-fixture:0";

    #[derive(SurrealValue)]
    struct TestWorkspaceSeed {
        name: String,
        last_job_id: Option<String>,
        last_workflow_id: Option<String>,
        last_actor_id: Option<String>,
        edit_event_id: String,
        last_actor_kind: String,
    }

    #[derive(SurrealValue)]
    struct NoBindings {}

    #[derive(SurrealValue)]
    struct CountRow {
        count: i64,
    }

    #[derive(SurrealValue)]
    struct PayloadHashUpdate {
        record: RecordId,
        payload_hash: String,
    }

    #[derive(SurrealValue)]
    struct PayloadUpdate {
        record: RecordId,
        payload: Value,
    }

    /// `HANDSHAKE_STAGE_BINDING_FILE` is process-global, so the router-level authorization tests
    /// that install a real native-MCP binding must not race each other OR `api::memory`'s suite.
    use crate::api::stage::NATIVE_BINDING_ENV_LOCK as FR_AUTH_ENV_LOCK;

    struct BindingEnvGuard {
        previous: Option<std::ffi::OsString>,
        path: std::path::PathBuf,
    }

    impl Drop for BindingEnvGuard {
        fn drop(&mut self) {
            if let Some(previous) = self.previous.take() {
                std::env::set_var("HANDSHAKE_STAGE_BINDING_FILE", previous);
            } else {
                std::env::remove_var("HANDSHAKE_STAGE_BINDING_FILE");
            }
            let _ = std::fs::remove_file(&self.path);
        }
    }

    /// Install a REAL live native-MCP binding for the current process and return the token a
    /// client must present. Used by the router-level authorization proofs so they exercise
    /// `capture_context` itself rather than a hand-built context.
    fn install_native_binding() -> Result<(String, BindingEnvGuard), Box<dyn std::error::Error>> {
        let token = "a".repeat(64);
        let path = std::env::temp_dir().join(format!("hsk-fr-binding-{}.json", Uuid::now_v7()));
        std::fs::write(
            &path,
            serde_json::to_vec(&crate::api::stage::current_process_native_binding(&token))?,
        )?;
        let guard = BindingEnvGuard {
            previous: std::env::var_os("HANDSHAKE_STAGE_BINDING_FILE"),
            path: path.clone(),
        };
        std::env::set_var("HANDSHAKE_STAGE_BINDING_FILE", &path);
        Ok((token, guard))
    }

    /// The authenticated authority the middleware injects. Handler-level tests (which prove
    /// ingestion mechanics, not the auth boundary) construct it directly; the auth boundary itself
    /// is proven end-to-end through the mounted router in the dedicated authorization tests.
    fn test_recorder_authority(workspace_id: &str) -> RecorderAuthority {
        RecorderAuthority {
            ctx: crate::api::stage::CaptureContext {
                actor_kind: "operator".to_owned(),
                actor_id: TEST_ACTOR_ID.to_owned(),
                limiter_principal: "mt109-fixture-principal".to_owned(),
                actor: KernelActor::Operator(TEST_ACTOR_ID.to_owned()),
                kernel_task_run_id: "native-stage-task:mt109-fixture:0".to_owned(),
                session_run_id: "native-mcp-session:mt109-fixture:0".to_owned(),
                binding_token: "0".repeat(64),
            },
            workspace_id: Some(workspace_id.to_owned()),
        }
    }

    /// Handler-level ingestion with the authenticated authority the middleware would inject. The
    /// signature mirrors the real handler's extractor prefix so the tests read the same.
    async fn ingest_native_editor(
        State(state): State<AppState>,
        Json(event): Json<NativeEditorFrEventV0_1>,
    ) -> ApiResult<Json<Value>> {
        let workspace_id = event.canonical_workspace_id().to_owned();
        super::record_native_editor_event(
            State(state),
            Path(workspace_id.clone()),
            Extension(test_recorder_authority(&workspace_id)),
            Json(event),
        )
        .await
    }

    /// Handler-level recorder read with the authenticated authority the middleware would inject.
    async fn list_events_scoped(
        State(state): State<AppState>,
        Query(filter): Query<EventFilter>,
    ) -> ApiResult<Json<Vec<FlightEvent>>> {
        let workspace = filter.wsid.clone().unwrap_or_default();
        super::list_events(
            State(state),
            Extension(test_recorder_authority(&workspace)),
            Query(filter),
        )
        .await
    }

    /// Seed a real canonical workspace row with a chosen id. Ingestion binds the path workspace to
    /// canonical authority, so fixtures must name workspaces that actually exist. Every test runs
    /// in its own isolated embedded store, so the fixed ids never collide.
    async fn ensure_test_workspace(
        state: &AppState,
        workspace_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let workspace_id = workspace_id.to_owned();
        state
            .surreal
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .upsert_one::<surrealdb::types::Value, _>(
                            "workspaces",
                            &workspace_id,
                            TestWorkspaceSeed {
                                name: format!("mt109-fr-{workspace_id}"),
                                last_job_id: None,
                                last_workflow_id: None,
                                last_actor_id: None,
                                edit_event_id: Uuid::nil().to_string(),
                                last_actor_kind: "system".to_owned(),
                            },
                        )
                        .await
                        .map(|_| ())
                })
            })
            .await?;
        Ok(())
    }

    /// Count only the native-editor residue. The capability audit events the authorization
    /// boundary is REQUIRED to emit are evidence, not residue, so a zero-residue assertion must
    /// measure the native-editor family specifically instead of the whole recorder.
    async fn native_editor_fr_row_count(state: &AppState) -> Result<usize, RecorderError> {
        Ok(state
            .flight_recorder
            .list_events(crate::flight_recorder::EventFilter::default())
            .await?
            .into_iter()
            .filter(|event| {
                event.payload.get("event_family").and_then(Value::as_str) == Some("native_editor")
            })
            .count())
    }

    async fn native_editor_ledger_row_count(
        state: &AppState,
    ) -> Result<i64, Box<dyn std::error::Error>> {
        let rows = state
            .surreal
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values::<CountRow, _>(
                            "SELECT count() AS count FROM kernel_event_ledger WHERE aggregate_type = 'native_editor_event' GROUP ALL;",
                            NoBindings {},
                        )
                        .await
                })
            })
            .await?;
        Ok(rows.first().map_or(0, |row| row.count))
    }

    async fn native_editor_ledger_events(
        state: &AppState,
        aggregate_id: impl AsRef<str>,
    ) -> Result<Vec<crate::kernel::KernelEvent>, Box<dyn std::error::Error>> {
        Ok(state
            .storage
            .list_kernel_events_for_aggregate("native_editor_event", aggregate_id.as_ref())
            .await?)
    }

    async fn set_event_payload_hash(
        state: &AppState,
        event_id: String,
        payload_hash: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let affected = state
            .surreal
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .execute_returning(
                            "UPDATE $record SET payload_hash = $payload_hash RETURN AFTER;",
                            PayloadHashUpdate {
                                record: RecordId::new("kernel_event_ledger", event_id),
                                payload_hash,
                            },
                        )
                        .await
                })
            })
            .await?;
        if affected != 1 {
            return Err(format!("payload_hash fault injection matched {affected} rows").into());
        }
        Ok(())
    }

    async fn set_event_payload(
        state: &AppState,
        event_id: String,
        payload: Value,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let affected = state
            .surreal
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .execute_returning(
                            "UPDATE $record SET payload = $payload RETURN AFTER;",
                            PayloadUpdate {
                                record: RecordId::new("kernel_event_ledger", event_id),
                                payload,
                            },
                        )
                        .await
                })
            })
            .await?;
        if affected != 1 {
            return Err(format!("payload fault injection matched {affected} rows").into());
        }
        Ok(())
    }

    /// The exact redacted capability-decision audit rows this route group must emit.
    async fn capability_decisions(
        state: &AppState,
        capability_id: &str,
        outcome: &str,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Ok(state
            .flight_recorder
            .list_events(crate::flight_recorder::EventFilter {
                event_type: Some("capability_action".to_owned()),
                ..Default::default()
            })
            .await?
            .into_iter()
            .filter(|event| {
                event.payload.get("capability_id").and_then(Value::as_str) == Some(capability_id)
                    && event
                        .payload
                        .get("decision_outcome")
                        .and_then(Value::as_str)
                        == Some(outcome)
            })
            .collect())
    }

    struct TestLlmClient {
        profile: ModelProfile,
    }

    impl TestLlmClient {
        fn new() -> Self {
            Self {
                profile: ModelProfile::new("flight-recorder-api-test".to_string(), 4096),
            }
        }
    }

    #[async_trait::async_trait]
    impl LlmClient for TestLlmClient {
        async fn completion(
            &self,
            _req: CompletionRequest,
        ) -> Result<CompletionResponse, LlmError> {
            Ok(CompletionResponse {
                text: "ok".to_string(),
                usage: TokenUsage {
                    prompt_tokens: 1,
                    completion_tokens: 1,
                    total_tokens: 2,
                },
                latency_ms: 0,
            })
        }

        fn profile(&self) -> &ModelProfile {
            &self.profile
        }
    }

    async fn setup_state() -> Result<Option<AppState>, Box<dyn std::error::Error>> {
        let backend = embedded_test_backend().await?;

        let recorder = Arc::new(DuckDbFlightRecorder::new_in_memory(32)?);

        let state = AppState {
            storage: backend.database,
            surreal: backend.storage,
            flight_recorder: recorder.clone(),
            diagnostics: recorder,
            llm_client: Arc::new(TestLlmClient::new()),
            capability_registry: Arc::new(CapabilityRegistry::new()),
            session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
        };
        // Ingestion binds the path workspace to canonical authority, so the fixture workspaces
        // must be real rows in this test's isolated schema.
        ensure_test_workspace(&state, TEST_WORKSPACE_ID)
            .await
            .map_err(|error| error.to_string())?;
        ensure_test_workspace(&state, OTHER_TEST_WORKSPACE_ID)
            .await
            .map_err(|error| error.to_string())?;
        Ok(Some(state))
    }

    async fn serve_test_router(
        app: axum::Router,
    ) -> (String, reqwest::Client, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind native-editor test router");
        let address = listener.local_addr().expect("native-editor test address");
        let server = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("serve native-editor test router");
        });
        (format!("http://{address}"), reqwest::Client::new(), server)
    }

    fn state_with_recorder(state: &AppState, recorder: Arc<DuckDbFlightRecorder>) -> AppState {
        AppState {
            storage: state.storage.clone(),
            surreal: state.surreal.clone(),
            flight_recorder: recorder.clone(),
            diagnostics: recorder,
            llm_client: state.llm_client.clone(),
            capability_registry: state.capability_registry.clone(),
            session_registry: state.session_registry.clone(),
        }
    }

    #[tokio::test]
    async fn list_events_preserves_model_session_id_filter_and_payload(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(state) = setup_state().await? else {
            return Ok(());
        };
        let trace_id = Uuid::now_v7();

        state
            .flight_recorder
            .record_event(
                FlightRecorderEvent::new(
                    FlightRecorderEventType::System,
                    FlightRecorderActor::System,
                    trace_id,
                    json!({
                        "type": "system",
                        "event_id": "FR-EVT-SYS-001",
                    }),
                )
                .with_model_session_id("sess-keep")
                .with_wsids(vec![TEST_WORKSPACE_ID.to_owned()]),
            )
            .await?;

        state
            .flight_recorder
            .record_event(
                FlightRecorderEvent::new(
                    FlightRecorderEventType::System,
                    FlightRecorderActor::System,
                    trace_id,
                    json!({
                        "type": "system",
                        "event_id": "FR-EVT-SYS-000",
                    }),
                )
                .with_wsids(vec![TEST_WORKSPACE_ID.to_owned()]),
            )
            .await?;

        let response = list_events_scoped(
            State(state),
            Query(EventFilter {
                model_session_id: Some("sess-keep".to_string()),
                wsid: Some(TEST_WORKSPACE_ID.to_owned()),
                ..Default::default()
            }),
        )
        .await;
        let Json(events) = match response {
            Ok(payload) => payload,
            Err(_) => panic!("filtered flight recorder api response failed"),
        };

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].model_session_id.as_deref(), Some("sess-keep"));
        assert_eq!(events[0].event_type, "system");

        Ok(())
    }

    fn native_editor_envelope(event_id: &str) -> NativeEditorFrEventV0_1 {
        native_editor_envelope_in(TEST_WORKSPACE_ID, event_id)
    }

    /// `actor_id`/`actor_kind` are deliberately left unset: they are no longer client authority,
    /// and the handler fills them from the authenticated context.
    fn native_editor_envelope_in(workspace_id: &str, event_id: &str) -> NativeEditorFrEventV0_1 {
        NativeEditorFrEventV0_1 {
            schema_version: NATIVE_EDITOR_SCHEMA_VERSION.to_string(),
            event_id: event_id.to_string(),
            ts_utc: "2026-07-02T04:08:05Z".to_string(),
            kind: NativeEditorFrEventKind::CodeEdit,
            actor_id: Some(TEST_ACTOR_ID.to_owned()),
            actor_kind: Some(NativeEditorActorKind::Human),
            pane_id: "pane-rich".to_owned(),
            surface: None,
            workspace_id: Some(workspace_id.to_owned()),
            session_id: None,
            work_packet_id: None,
            payload: json!({"file_path":"src/main.rs","line_delta":1}),
        }
    }

    /// The durable, workspace-partitioned recorder id for a fixture envelope.
    fn durable_id(event: &NativeEditorFrEventV0_1) -> Uuid {
        workspace_scoped_fr_event_id(
            event.canonical_workspace_id(),
            Uuid::parse_str(event.event_id.trim()).expect("uuid fixture event id"),
        )
    }

    /// The durable recorder id for a client event id submitted to the canonical test workspace.
    fn durable_id_for(event_id: &str) -> Uuid {
        workspace_scoped_fr_event_id(
            TEST_WORKSPACE_ID,
            Uuid::parse_str(event_id.trim()).expect("uuid fixture event id"),
        )
    }

    fn native_editor_pending_event(event: &NativeEditorFrEventV0_1) -> NewKernelEvent {
        let expected = native_editor_fr_event_from_envelope(event)
            .unwrap_or_else(|_| panic!("valid native-editor Flight Recorder fixture"));
        build_native_editor_pending(event, &expected).expect("valid native-editor pending fixture")
    }

    async fn authentic_document_saved_envelope(
        state: &AppState,
    ) -> Result<NativeEditorFrEventV0_1, Box<dyn std::error::Error>> {
        authentic_document_saved_envelope_minted_by(state, TEST_ACTOR_ID).await
    }

    /// Mint a canonical save receipt owned by `minted_by_principal` and return the native envelope
    /// that claims it.
    ///
    /// WP-KERNEL-012 MT-120: the ledger `actor_id` column is deliberately a DISTINCT client-declared
    /// per-agent id, exactly as a real save produces (`x-hsk-actor-id` is per-agent attribution, the
    /// native principal is server-derived). Ownership therefore lives in the server-written
    /// `minted_by_principal` payload field, which is what the claim is validated against.
    async fn authentic_document_saved_envelope_minted_by(
        state: &AppState,
        minted_by_principal: &str,
    ) -> Result<NativeEditorFrEventV0_1, Box<dyn std::error::Error>> {
        let document_id = format!("DOC-{}", Uuid::now_v7());
        let workspace_id = format!("WS-{}", Uuid::now_v7());
        ensure_test_workspace(state, &workspace_id).await?;
        let content_hash = "a".repeat(64);
        let task = format!("task-{}", Uuid::now_v7());
        let session = format!("session-{}", Uuid::now_v7());
        let correlation = format!("correlation-{}", Uuid::now_v7());
        // The CLIENT-declared per-agent save actor. It is NOT the native principal, and the ledger
        // column keeps it so swarm attribution survives (AC-120-2 / the MT-043 attribution assert).
        let actor_id = format!("mt120-agent-{}", Uuid::now_v7());
        let mut receipt_payload = json!({
            "event":"saved",
            "doc_version":2,
            "workspace_id":workspace_id,
            "content_hash":content_hash,
        });
        receipt_payload[crate::api::knowledge_documents::SAVE_RECEIPT_MINTED_BY_PRINCIPAL_FIELD] =
            json!(minted_by_principal);
        let receipt = state
            .storage
            .append_kernel_event(
                NewKernelEvent::builder(
                    task.clone(),
                    session.clone(),
                    KernelEventType::KnowledgeRichDocumentSaved,
                    KernelActor::Operator(actor_id),
                )
                .aggregate("knowledge_rich_document", document_id.clone())
                .source_component("knowledge_documents_api")
                .correlation_id(correlation.clone())
                .payload(receipt_payload)
                .build()?,
            )
            .await?;
        Ok(NativeEditorFrEventV0_1 {
            schema_version: NATIVE_EDITOR_SCHEMA_VERSION.to_owned(),
            event_id: Uuid::now_v7().to_string(),
            ts_utc: Utc::now().to_rfc3339(),
            kind: NativeEditorFrEventKind::DocumentSaved,
            actor_id: Some(TEST_ACTOR_ID.to_owned()),
            actor_kind: Some(NativeEditorActorKind::Human),
            pane_id: "pane-rich".to_owned(),
            surface: Some("rich-editor".to_owned()),
            workspace_id: Some(workspace_id),
            session_id: None,
            work_packet_id: Some("WP-KERNEL-012".to_owned()),
            payload: json!({
                "document_id":document_id,
                "content_hash":content_hash,
                "save_receipt_event_id":receipt.event_id,
                "actor_kind":"operator",
                "kernel_task_run_id":task,
                "session_run_id":session,
                "correlation_id":correlation,
            }),
        })
    }

    /// AC-109-1: a native-editor event lands in the FR store (readable via GET),
    /// idempotent on event_id, and durably mirrors into the kernel EventLedger.
    #[tokio::test]
    async fn native_editor_event_round_trips_and_mirrors_to_ledger(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(state) = setup_state().await? else {
            return Ok(());
        };
        let event_id = Uuid::now_v7().to_string();
        let uuid = Uuid::parse_str(&event_id)?;
        // The durable, workspace-partitioned identity the recorder actually stores.
        let durable_uuid = workspace_scoped_fr_event_id(TEST_WORKSPACE_ID, uuid);
        assert_ne!(
            durable_uuid, uuid,
            "the durable id must not be the caller-controlled id"
        );
        let body = json!({
            "schema_version": NATIVE_EDITOR_SCHEMA_VERSION,
            "event_id": event_id,
            "ts_utc": "2026-07-02T04:08:05Z",
            "kind": "stage_embed_back",
            // An envelope MAY confirm the authenticated identity; it may never set it.
            "actor_id": TEST_ACTOR_ID,
            "actor_kind": "human",
            "pane_id": "pane-rich",
            "surface": "stage",
            "workspace_id": TEST_WORKSPACE_ID,
            "payload": {
                "artifact_id": "ART-1",
                "target_pane_id": "pane-rich",
                "sha256": "a".repeat(64),
                "manifest_ref": "manifest-ART-1",
                "causal_action_id": "stage-route-action-1"
            }
        });
        let event: NativeEditorFrEventV0_1 = serde_json::from_value(body.clone())?;

        let Json(ack) = ingest_native_editor(State(state.clone()), Json(event))
            .await
            .map_err(|(code, _body)| format!("ingest failed: {code}"))?;
        assert_eq!(ack["ok"], true);
        assert_eq!(ack["kind"], "stage_embed_back");
        assert_eq!(ack["event_id"], event_id, "the caller id is echoed back");
        assert_eq!(
            ack["fr_event_id"],
            durable_uuid.to_string(),
            "the ack names the durable workspace-partitioned recorder id"
        );
        assert_eq!(
            ack["actor_id"], TEST_ACTOR_ID,
            "attribution is server-derived, never caller-declared"
        );

        // Readable back via the existing GET route, keyed on event_id.
        let Json(events) = list_events_scoped(
            State(state.clone()),
            Query(EventFilter {
                event_id: Some(durable_uuid),
                wsid: Some(TEST_WORKSPACE_ID.to_owned()),
                ..Default::default()
            }),
        )
        .await
        .map_err(|_| "list failed")?;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "system");
        assert_eq!(events[0].event_id, durable_uuid.to_string());
        assert_eq!(events[0].actor_id, TEST_ACTOR_ID);
        assert_eq!(events[0].payload["client_event_id"], event_id);
        assert_eq!(events[0].payload["actor_id"], TEST_ACTOR_ID);
        assert_eq!(events[0].payload["kind"], "stage_embed_back");
        assert_eq!(events[0].payload["action"], "stage_embed_back");
        assert_eq!(events[0].payload["schema"], NATIVE_EDITOR_SCHEMA_VERSION);
        assert_eq!(events[0].payload["event_family"], "native_editor");
        assert_eq!(
            events[0].payload["native_payload"]["causal_action_id"],
            "stage-route-action-1"
        );
        assert!(events[0].wsids.contains(&TEST_WORKSPACE_ID.to_string()));

        // Durable EventLedger mirror in the embedded authority store.
        let ledger_count = native_editor_ledger_events(&state, durable_uuid.to_string())
            .await?
            .into_iter()
            .filter(|row| row.event_type == KernelEventType::FlightRecorderMirrorRecorded)
            .count();
        assert_eq!(
            ledger_count, 1,
            "one durable native-editor FR ledger receipt"
        );

        // Idempotent re-POST of the same event_id.
        let event_again: NativeEditorFrEventV0_1 = serde_json::from_value(body)?;
        let Json(ack2) = ingest_native_editor(State(state.clone()), Json(event_again))
            .await
            .map_err(|(code, _body)| format!("re-ingest failed: {code}"))?;
        assert_eq!(ack2["idempotent"], true);

        let Json(events_after) = list_events_scoped(
            State(state.clone()),
            Query(EventFilter {
                event_id: Some(durable_uuid),
                wsid: Some(TEST_WORKSPACE_ID.to_owned()),
                ..Default::default()
            }),
        )
        .await
        .map_err(|_| "list failed")?;
        assert_eq!(
            events_after.len(),
            1,
            "idempotent: still exactly one FR row"
        );

        let ledger_after = native_editor_ledger_events(&state, durable_uuid.to_string())
            .await?
            .into_iter()
            .filter(|row| row.event_type == KernelEventType::FlightRecorderMirrorRecorded)
            .count();
        assert_eq!(
            ledger_after, 1,
            "idempotent: still exactly one ledger receipt"
        );
        Ok(())
    }

    #[tokio::test]
    async fn native_editor_same_id_mutated_envelope_conflicts(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(state) = setup_state().await? else {
            return Ok(());
        };
        for mutation in ["pane", "surface", "timestamp"] {
            let event_id = Uuid::now_v7().to_string();
            let mut original = native_editor_envelope(&event_id);
            original.surface = Some("pane-rich".to_owned());
            ingest_native_editor(State(state.clone()), Json(original.clone()))
                .await
                .map_err(|(status, _)| format!("initial ingest failed: {status}"))?;
            let mut changed = original;
            match mutation {
                "pane" => changed.pane_id = "pane-other".to_owned(),
                "surface" => changed.surface = Some("surface-other".to_owned()),
                "timestamp" => changed.ts_utc = "2026-07-02T04:08:06Z".to_owned(),
                _ => unreachable!(),
            }
            assert!(
                matches!(
                    ingest_native_editor(State(state.clone()), Json(changed)).await,
                    Err((StatusCode::CONFLICT, _))
                ),
                "same event_id with changed {mutation} must conflict"
            );
        }
        Ok(())
    }

    #[tokio::test]
    async fn native_editor_concurrent_same_id_converges_once_in_both_stores(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(state) = setup_state().await? else {
            return Ok(());
        };
        let event_id = Uuid::now_v7().to_string();
        let event = native_editor_envelope(&event_id);

        let (left, right) = tokio::join!(
            ingest_native_editor(State(state.clone()), Json(event.clone())),
            ingest_native_editor(State(state.clone()), Json(event)),
        );
        assert!(left.is_ok(), "first concurrent ingest failed");
        assert!(right.is_ok(), "second concurrent ingest failed");

        let recorder_rows = state
            .flight_recorder
            .list_events(crate::flight_recorder::EventFilter {
                event_id: Some(durable_id_for(&event_id)),
                ..Default::default()
            })
            .await?;
        assert_eq!(recorder_rows.len(), 1, "exactly one Flight Recorder row");

        let ledger_rows =
            native_editor_ledger_events(&state, durable_id_for(&event_id).to_string())
                .await?
                .into_iter()
                .filter(|row| {
                    matches!(
                        row.event_type,
                        KernelEventType::FlightRecorderMirrorPending
                            | KernelEventType::FlightRecorderMirrorRecorded
                    )
                })
                .count();
        assert_eq!(ledger_rows, 2, "one pending and one completion receipt");
        Ok(())
    }

    #[tokio::test]
    async fn native_editor_canonical_uuid_and_timestamp_spellings_converge(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(state) = setup_state().await? else {
            return Ok(());
        };
        let uuid = Uuid::now_v7();
        let mut first = native_editor_envelope(&format!("  {}  ", uuid.to_string().to_uppercase()));
        first.ts_utc = "2026-07-02T06:08:05.123456789+02:00".to_owned();
        let mut retry = native_editor_envelope(&uuid.to_string());
        retry.ts_utc = "2026-07-02T04:08:05.123456789Z".to_owned();
        let expected_timestamp = DateTime::parse_from_rfc3339(&retry.ts_utc)?.with_timezone(&Utc);

        ingest_native_editor(State(state.clone()), Json(first))
            .await
            .map_err(|(status, _)| format!("canonical initial ingest failed: {status}"))?;
        let Json(ack) = ingest_native_editor(State(state.clone()), Json(retry))
            .await
            .map_err(|(status, _)| format!("canonical retry failed: {status}"))?;
        assert_eq!(ack["idempotent"], true);

        let durable_uuid = workspace_scoped_fr_event_id(TEST_WORKSPACE_ID, uuid);
        let aggregate_id = durable_uuid.to_string();
        let ledger_rows = native_editor_ledger_events(&state, &aggregate_id)
            .await?
            .into_iter()
            .filter(|row| {
                matches!(
                    row.event_type,
                    KernelEventType::FlightRecorderMirrorPending
                        | KernelEventType::FlightRecorderMirrorRecorded
                )
            })
            .count();
        assert_eq!(
            ledger_rows, 2,
            "one canonical pending/completion state machine"
        );
        let recorder_rows = state
            .flight_recorder
            .list_events(crate::flight_recorder::EventFilter {
                event_id: Some(durable_uuid),
                ..Default::default()
            })
            .await?;
        assert_eq!(recorder_rows.len(), 1);
        assert_eq!(
            recorder_rows[0].timestamp.timestamp_micros(),
            expected_timestamp.timestamp_micros(),
            "DuckDB readback must preserve the canonical storage microsecond"
        );
        assert_eq!(
            recorder_rows[0].payload["ts_utc"],
            json!("2026-07-02T04:08:05.123456789+00:00"),
            "the immutable envelope retains the canonical nanosecond spelling"
        );
        Ok(())
    }

    #[tokio::test]
    async fn native_editor_decomposed_unicode_retry_matches_normalized_store(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(state) = setup_state().await? else {
            return Ok(());
        };
        let event_id = Uuid::now_v7().to_string();
        let mut event = native_editor_envelope(&event_id);
        event.kind = NativeEditorFrEventKind::CodeEdit;
        event.payload = json!({"file_path": "Cafe\u{301}.rs", "line_delta": 1});
        ingest_native_editor(State(state.clone()), Json(event.clone()))
            .await
            .map_err(|(status, _)| format!("unicode initial ingest failed: {status}"))?;
        let Json(ack) = ingest_native_editor(State(state), Json(event))
            .await
            .map_err(|(status, _)| format!("unicode retry failed: {status}"))?;
        assert_eq!(ack["idempotent"], true);
        Ok(())
    }

    #[test]
    fn native_editor_exact_fr_matcher_rejects_each_unrelated_row() {
        let event = native_editor_envelope(&Uuid::now_v7().to_string());
        let expected = native_editor_fr_event_from_envelope(&event)
            .unwrap_or_else(|_| panic!("expected FR event"));
        let mut changed = expected.clone();
        changed.actor_id = "other-actor".to_owned();
        assert!(!native_editor_fr_event_matches(&changed, &expected));
        let mut changed = expected.clone();
        changed.wsids = vec!["other-workspace".to_owned()];
        assert!(!native_editor_fr_event_matches(&changed, &expected));
        let mut changed = expected.clone();
        changed.payload["pane_id"] = json!("other-pane");
        assert!(!native_editor_fr_event_matches(&changed, &expected));
    }

    #[test]
    fn native_editor_fr_matcher_uses_recorder_timestamp_precision_without_weakening_payload() {
        let event = native_editor_envelope(&Uuid::now_v7().to_string());
        let expected = native_editor_fr_event_from_envelope(&event)
            .unwrap_or_else(|_| panic!("expected FR event"));

        let mut duckdb_readback = expected.clone();
        duckdb_readback.timestamp += chrono::Duration::nanoseconds(200);
        assert_eq!(
            duckdb_readback.timestamp.timestamp_micros(),
            expected.timestamp.timestamp_micros(),
            "fixture must stay inside one DuckDB microsecond"
        );
        assert!(native_editor_fr_event_matches(&duckdb_readback, &expected));

        let mut next_microsecond = expected.clone();
        next_microsecond.timestamp += chrono::Duration::microseconds(1);
        assert!(!native_editor_fr_event_matches(
            &next_microsecond,
            &expected
        ));

        let mut changed_envelope_timestamp = duckdb_readback;
        changed_envelope_timestamp.payload["ts_utc"] = json!("2026-07-16T00:00:00Z");
        assert!(
            !native_editor_fr_event_matches(&changed_envelope_timestamp, &expected),
            "the immutable envelope timestamp remains exact even when the typed column is compared at storage precision"
        );
    }

    #[test]
    fn native_editor_system_events_filter_by_surface_without_matching_unrelated_system_rows() {
        let mut envelope = native_editor_envelope(&Uuid::now_v7().to_string());
        envelope.surface = Some("pane-rich".to_owned());
        let native = native_editor_fr_event_from_envelope(&envelope)
            .unwrap_or_else(|_| panic!("native system event"));
        assert!(system_event_matches_surface(&native, "pane-rich"));
        assert!(!system_event_matches_surface(&native, "pane-code"));
        assert!(system_event_matches_surface(&native, "system"));

        let unrelated = FlightRecorderEvent::new(
            FlightRecorderEventType::System,
            FlightRecorderActor::System,
            Uuid::now_v7(),
            json!({"editor_surface": "pane-rich", "event_family": "runtime"}),
        )
        .with_actor_id("runtime-system");
        assert!(
            !system_event_matches_surface(&unrelated, "pane-rich"),
            "a non-native System row must not enter an editor-surface projection merely because it carries a similarly named field"
        );
        assert!(system_event_matches_surface(&unrelated, "system"));
    }

    #[tokio::test]
    async fn list_events_surface_filter_returns_only_native_system_events_for_that_surface(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(state) = setup_state().await? else {
            return Ok(());
        };
        let native_id = Uuid::now_v7();
        let mut envelope = native_editor_envelope(&native_id.to_string());
        envelope.surface = Some("pane-rich".to_owned());
        state
            .flight_recorder
            .record_event(
                native_editor_fr_event_from_envelope(&envelope)
                    .map_err(|_| "native fixture rejected")?,
            )
            .await?;

        let unrelated = FlightRecorderEvent::new(
            FlightRecorderEventType::System,
            FlightRecorderActor::System,
            Uuid::now_v7(),
            json!({"editor_surface": "pane-rich", "event_family": "runtime"}),
        )
        .with_actor_id("runtime-system");
        state.flight_recorder.record_event(unrelated).await?;

        let Json(rows) = list_events_scoped(
            State(state),
            Query(EventFilter {
                surface: Some("pane-rich".to_owned()),
                event_type: Some("system".to_owned()),
                wsid: Some(TEST_WORKSPACE_ID.to_owned()),
                ..Default::default()
            }),
        )
        .await
        .map_err(|(status, _)| format!("surface-filter list failed: {status}"))?;
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].event_id,
            workspace_scoped_fr_event_id(TEST_WORKSPACE_ID, native_id).to_string()
        );
        assert_eq!(rows[0].payload["event_family"], "native_editor");
        Ok(())
    }

    #[test]
    fn native_editor_receipt_matchers_reject_minimal_corruption() {
        let event = native_editor_envelope(&Uuid::now_v7().to_string());
        let expected_fr = native_editor_fr_event_from_envelope(&event)
            .unwrap_or_else(|_| panic!("expected FR event"));
        let pending = crate::kernel::KernelEvent::from_new(native_editor_pending_event(&event));
        assert!(native_editor_pending_receipt_matches(
            &pending,
            &event,
            &expected_fr
        ));

        for mutation in [
            "event_version",
            "task",
            "session",
            "aggregate_type",
            "aggregate_id",
            "key",
            "event_type",
            "actor",
            "causation",
            "correlation",
            "payload_hash",
            "source",
            "receipt_kind",
            "extra_payload",
        ] {
            let mut changed = pending.clone();
            match mutation {
                "event_version" => changed.event_version = "kernel_event_v2".to_owned(),
                "task" => changed.kernel_task_run_id = "other-task".to_owned(),
                "session" => changed.session_run_id = "other-session".to_owned(),
                "aggregate_type" => changed.aggregate_type = "other-aggregate".to_owned(),
                "aggregate_id" => changed.aggregate_id = "other-event".to_owned(),
                "key" => changed.idempotency_key = "other-key".to_owned(),
                "event_type" => changed.event_type = KernelEventType::FlightRecorderMirrorRecorded,
                "actor" => changed.actor = KernelActor::System("other".to_owned()),
                "causation" => changed.causation_id = Some(Uuid::now_v7().to_string()),
                "correlation" => changed.correlation_id = Some("other-correlation".to_owned()),
                "payload_hash" => changed.payload_hash = "0".repeat(64),
                "source" => changed.source_component = "other-source".to_owned(),
                "receipt_kind" => {
                    changed.payload["receipt_kind"] = json!("wrong");
                }
                "extra_payload" => {
                    changed.payload["extra"] = json!(true);
                }
                _ => unreachable!(),
            }
            assert!(
                !native_editor_pending_receipt_matches(&changed, &event, &expected_fr),
                "pending receipt accepted minimal {mutation} corruption"
            );
        }

        let completion = crate::kernel::KernelEvent::from_new(
            build_native_editor_completion(&pending, &event).expect("completion fixture"),
        );
        assert!(native_editor_completion_matches(&pending, &completion));
        for mutation in [
            "event_version",
            "receipt_kind",
            "fr_event_type",
            "extra_payload",
            "actor",
            "task",
            "session",
            "aggregate_type",
            "aggregate_id",
            "key",
            "event_type",
            "causation",
            "correlation",
            "payload_hash",
            "source",
        ] {
            let mut changed = completion.clone();
            match mutation {
                "event_version" => changed.event_version = "kernel_event_v2".to_owned(),
                "receipt_kind" => changed.payload["receipt_kind"] = json!("wrong"),
                "fr_event_type" => changed.payload["fr_event_type"] = json!("wrong"),
                "extra_payload" => changed.payload["extra"] = json!(true),
                "actor" => changed.actor = KernelActor::System("other".to_owned()),
                "task" => changed.kernel_task_run_id = "other-task".to_owned(),
                "session" => changed.session_run_id = "other-session".to_owned(),
                "aggregate_type" => changed.aggregate_type = "other-aggregate".to_owned(),
                "aggregate_id" => changed.aggregate_id = "other-event".to_owned(),
                "key" => changed.idempotency_key = "other-key".to_owned(),
                "event_type" => changed.event_type = KernelEventType::FlightRecorderMirrorPending,
                "causation" => changed.causation_id = Some(Uuid::now_v7().to_string()),
                "correlation" => changed.correlation_id = Some("other-correlation".to_owned()),
                "payload_hash" => changed.payload_hash = "0".repeat(64),
                "source" => changed.source_component = "other-source".to_owned(),
                _ => unreachable!(),
            }
            assert!(
                !native_editor_completion_matches(&pending, &changed),
                "completion accepted minimal {mutation} corruption"
            );
        }
    }

    #[tokio::test]
    async fn native_editor_reconciler_repairs_durable_pending_after_restart_window(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(state) = setup_state().await? else {
            return Ok(());
        };
        let event_id = Uuid::now_v7().to_string();
        let event = native_editor_envelope(&event_id);
        let pending = native_editor_pending_event(&event);
        state.storage.append_kernel_event(pending).await?;

        let before = state
            .flight_recorder
            .list_events(crate::flight_recorder::EventFilter {
                event_id: Some(durable_id_for(&event_id)),
                ..Default::default()
            })
            .await?;
        assert!(
            before.is_empty(),
            "fixture represents the post-ledger/pre-FR crash window"
        );

        // This is the same startup pass installed by `routes`: it discovers work from durable
        // embedded EventLedger state rather than relying on an in-memory queue from the failed process.
        reconcile_native_editor_pending(&state)
            .await
            .map_err(std::io::Error::other)?;

        let after = state
            .flight_recorder
            .list_events(crate::flight_recorder::EventFilter {
                event_id: Some(durable_id_for(&event_id)),
                ..Default::default()
            })
            .await?;
        assert_eq!(after.len(), 1, "startup reconciliation restored the FR row");
        let completion_count =
            native_editor_ledger_events(&state, durable_id_for(&event_id).to_string())
                .await?
                .into_iter()
                .filter(|row| row.event_type == KernelEventType::FlightRecorderMirrorRecorded)
                .count();
        assert_eq!(
            completion_count, 1,
            "completion exists only after FR recovery"
        );
        Ok(())
    }

    #[tokio::test]
    async fn native_editor_routes_autonomously_starts_reconciliation(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(state) = setup_state().await? else {
            return Ok(());
        };
        let event_id = Uuid::now_v7().to_string();
        let event = native_editor_envelope(&event_id);
        state
            .storage
            .append_kernel_event(native_editor_pending_event(&event))
            .await?;

        let _mounted_routes = routes(state.clone());
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let rows = state
                .flight_recorder
                .list_events(crate::flight_recorder::EventFilter {
                    event_id: Some(durable_id_for(&event_id)),
                    ..Default::default()
                })
                .await?;
            if rows.len() == 1 {
                break;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "routes() did not autonomously start native-editor reconciliation"
            );
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
        Ok(())
    }

    #[tokio::test]
    async fn native_editor_spurious_completion_does_not_suppress_recovery(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(state) = setup_state().await? else {
            return Ok(());
        };
        let event_id = Uuid::now_v7().to_string();
        let event = native_editor_envelope(&event_id);
        let pending = state
            .storage
            .append_kernel_event(native_editor_pending_event(&event))
            .await?;
        let spurious = NewKernelEvent::builder(
            event.canonical_workspace_id().to_owned(),
            event.event_id.clone(),
            KernelEventType::FlightRecorderMirrorRecorded,
            KernelActor::Operator(event.canonical_actor_id().to_owned()),
        )
        .aggregate("native_editor_event", event.event_id.clone())
        .idempotency_key(format!(
            "spurious-native-editor-completion:{}",
            event.event_id
        ))
        .source_component("unrelated_component")
        .causation_id("unrelated-cause")
        .payload(json!({"fr_event_id": event.event_id, "envelope": {"wrong": true}}))
        .build()?;
        let spurious = state.storage.append_kernel_event(spurious).await?;
        assert!(!native_editor_completion_matches(&pending, &spurious));

        reconcile_native_editor_pending(&state)
            .await
            .map_err(std::io::Error::other)?;
        assert_eq!(
            state
                .flight_recorder
                .list_events(crate::flight_recorder::EventFilter {
                    event_id: Some(durable_id_for(&event_id)),
                    ..Default::default()
                })
                .await?
                .len(),
            1,
            "spurious completion must not suppress the pending mirror"
        );
        let completion_count =
            native_editor_ledger_events(&state, durable_id_for(&event_id).to_string())
                .await?
                .into_iter()
                .filter(|row| row.event_type == KernelEventType::FlightRecorderMirrorRecorded)
                .count();
        assert_eq!(completion_count, 1);
        Ok(())
    }

    #[tokio::test]
    async fn native_editor_completion_with_corrupt_hash_cannot_suppress_recovery(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(state) = setup_state().await? else {
            return Ok(());
        };
        let event = native_editor_envelope(&Uuid::now_v7().to_string());
        let pending = state
            .storage
            .append_kernel_event(native_editor_pending_event(&event))
            .await?;
        state
            .flight_recorder
            .record_event(
                native_editor_fr_event_from_envelope(&event)
                    .map_err(|_| "failed to build corrupt-hash FR fixture")?,
            )
            .await?;
        let completion = build_native_editor_completion(&pending, &event)?;
        let canonical_completion_hash = completion.payload_hash.clone();
        let completion = state.storage.append_kernel_event(completion).await?;
        let completion_event_id = completion.event_id;

        set_event_payload_hash(&state, completion_event_id.clone(), "0".repeat(64)).await?;

        let candidates = state
            .storage
            .list_pending_native_editor_mirrors(0, 100)
            .await?;
        let candidate_found = candidates
            .iter()
            .any(|candidate| candidate.event_id == pending.event_id);
        let corrupt_completion_rejected = reconcile_native_editor_pending_receipt(&state, pending)
            .await
            .is_err();
        // Restore the test-injected fault so the embedded fixture remains internally consistent.
        set_event_payload_hash(&state, completion_event_id, canonical_completion_hash).await?;
        assert!(
            candidate_found,
            "completion with a non-canonical payload_hash suppressed recovery"
        );
        assert!(
            corrupt_completion_rejected,
            "corrupt completion was accepted as authentic"
        );
        Ok(())
    }

    #[tokio::test]
    async fn native_editor_legacy_pending_without_expected_hash_remains_recoverable(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(state) = setup_state().await? else {
            return Ok(());
        };
        let event = native_editor_envelope(&Uuid::now_v7().to_string());
        let mut legacy_pending = native_editor_pending_event(&event);
        legacy_pending
            .payload
            .as_object_mut()
            .expect("pending payload object")
            .remove("expected_completion_payload_hash");
        legacy_pending.payload_hash = crate::kernel::context_bundle::sha256_hex(
            &crate::kernel::context_bundle::canonical_json_bytes(&legacy_pending.payload),
        );
        let pending = state.storage.append_kernel_event(legacy_pending).await?;

        reconcile_native_editor_pending_receipt(&state, pending.clone())
            .await
            .map_err(std::io::Error::other)?;
        let recovered = state
            .flight_recorder
            .list_events(crate::flight_recorder::EventFilter {
                event_id: Some(durable_id(&event)),
                ..Default::default()
            })
            .await?;
        assert_eq!(
            recovered.len(),
            1,
            "legacy pending row recovered its FR row"
        );

        // Legacy rows remain candidates because their immutable payload cannot be
        // backfilled. The exact Rust completion matcher makes reprocessing idempotent.
        let candidates = state
            .storage
            .list_pending_native_editor_mirrors(0, 100)
            .await?;
        assert!(candidates
            .iter()
            .any(|candidate| candidate.event_id == pending.event_id));
        reconcile_native_editor_pending_receipt(&state, pending)
            .await
            .map_err(std::io::Error::other)?;
        Ok(())
    }

    #[tokio::test]
    async fn native_editor_persistent_restart_repairs_both_partial_write_windows(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(base_state) = setup_state().await? else {
            return Ok(());
        };
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("native-editor-restart.duckdb");
        let recorder_before = Arc::new(DuckDbFlightRecorder::new_on_path(&path, 32)?);
        let state_before = state_with_recorder(&base_state, recorder_before.clone());

        let before_fr = native_editor_envelope(&Uuid::now_v7().to_string());
        let after_fr = native_editor_envelope(&Uuid::now_v7().to_string());
        state_before
            .storage
            .append_kernel_events_atomic(vec![
                native_editor_pending_event(&before_fr),
                native_editor_pending_event(&after_fr),
            ])
            .await?;
        state_before
            .flight_recorder
            .record_event(native_editor_fr_event_from_envelope(&after_fr).map_err(|_| "fixture")?)
            .await?;
        drop(state_before);
        drop(recorder_before);

        let recorder_after = Arc::new(DuckDbFlightRecorder::new_on_path(&path, 32)?);
        let state_after = state_with_recorder(&base_state, recorder_after);
        reconcile_native_editor_pending(&state_after)
            .await
            .map_err(std::io::Error::other)?;

        for event in [&before_fr, &after_fr] {
            let uuid = durable_id(event);
            assert_eq!(
                state_after
                    .flight_recorder
                    .list_events(crate::flight_recorder::EventFilter {
                        event_id: Some(uuid),
                        ..Default::default()
                    })
                    .await?
                    .len(),
                1,
                "restart must converge each partial-write window to one FR row"
            );
            let completion_count =
                native_editor_ledger_events(&state_after, durable_id(&event).to_string())
                    .await?
                    .into_iter()
                    .filter(|row| row.event_type == KernelEventType::FlightRecorderMirrorRecorded)
                    .count();
            assert_eq!(completion_count, 1);
        }
        drop(state_after);
        Ok(())
    }

    #[tokio::test]
    async fn native_editor_reconciler_traverses_more_than_one_poison_batch(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(state) = setup_state().await? else {
            return Ok(());
        };
        let prefix = format!("poison-{}", Uuid::now_v7());
        let valid = native_editor_envelope(&Uuid::now_v7().to_string());
        let mut events = Vec::with_capacity(102);
        for index in 0..101 {
            let aggregate_id = format!("{prefix}-{index:03}");
            events.push(
                NewKernelEvent::builder(
                    "WS-NE-POISON",
                    aggregate_id.clone(),
                    KernelEventType::FlightRecorderMirrorPending,
                    KernelActor::System("poison-probe".to_owned()),
                )
                .aggregate("native_editor_event", aggregate_id.clone())
                .idempotency_key(format!("native-editor-fr-pending:{aggregate_id}"))
                .source_component("native_editor_fr_ingestion")
                .payload(json!({"receipt_kind":"native_editor_flight_recorder_pending","envelope":{"invalid":true}}))
                .build()?,
            );
        }
        events.push(native_editor_pending_event(&valid));
        state.storage.append_kernel_events_atomic(events).await?;

        reconcile_native_editor_pending(&state)
            .await
            .map_err(std::io::Error::other)?;
        let valid_rows = state
            .flight_recorder
            .list_events(crate::flight_recorder::EventFilter {
                event_id: Some(durable_id(&valid)),
                ..Default::default()
            })
            .await?
            .len();

        assert_eq!(
            valid_rows, 1,
            "101 poison rows must not starve the newer valid mirror"
        );
        Ok(())
    }

    /// AC-109-1: unknown kinds and unknown fields are rejected at decode (typed rejection,
    /// closed vocabulary — no free-text smuggling).
    #[test]
    fn native_editor_envelope_rejects_unknown_kind_and_field() {
        let bad_kind = json!({
            "schema_version": NATIVE_EDITOR_SCHEMA_VERSION,
            "event_id": Uuid::now_v7().to_string(),
            "ts_utc": "2026-07-02T04:08:05Z",
            "kind": "bogus_kind",
            "actor_id": "a",
            "pane_id": "pane-rich",
            "workspace_id": "WS-NE-1"
        });
        assert!(serde_json::from_value::<NativeEditorFrEventV0_1>(bad_kind).is_err());

        let legacy_memory_proposed = json!({
            "schema_version": NATIVE_EDITOR_SCHEMA_VERSION,
            "event_id": Uuid::now_v7().to_string(),
            "ts_utc": "2026-07-02T04:08:05Z",
            "kind": "memory_write_proposed",
            "actor_id": "a",
            "pane_id": "pane-rich",
            "workspace_id": "WS-NE-1"
        });
        assert!(
            serde_json::from_value::<NativeEditorFrEventV0_1>(legacy_memory_proposed).is_err(),
            "FR-EVT-MEM-001 is backend-owned and must not re-enter through the native-editor envelope"
        );

        let bad_field = json!({
            "schema_version": NATIVE_EDITOR_SCHEMA_VERSION,
            "event_id": Uuid::now_v7().to_string(),
            "ts_utc": "2026-07-02T04:08:05Z",
            "kind": "document_saved",
            "actor_id": "a",
            "pane_id": "pane-rich",
            "workspace_id": "WS-NE-1",
            "smuggled_free_text": "arbitrary"
        });
        assert!(serde_json::from_value::<NativeEditorFrEventV0_1>(bad_field).is_err());
    }

    #[tokio::test]
    async fn native_editor_route_rejects_unknown_kind_and_field_without_durable_residue(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(state) = setup_state().await? else {
            return Ok(());
        };
        let _env_lock = FR_AUTH_ENV_LOCK.lock().expect("fr auth env lock");
        let (token, _binding) = install_native_binding()?;
        let before_fr = native_editor_fr_row_count(&state).await?;
        let before_ledger = native_editor_ledger_row_count(&state).await?;
        let (base, http, server) = serve_test_router(routes(state.clone())).await;
        let endpoint =
            format!("{base}/workspaces/{TEST_WORKSPACE_ID}/flight_recorder/native_editor_event");

        let mut unknown_kind =
            serde_json::to_value(native_editor_envelope(&Uuid::now_v7().to_string()))?;
        unknown_kind["kind"] = json!("smuggled_editor_kind");
        let response = http
            .post(&endpoint)
            .header("x-hsk-session-token", &token)
            .json(&unknown_kind)
            .send()
            .await?;
        assert!(
            response.status().is_client_error(),
            "unknown kind must be rejected by the mounted route, got {}",
            response.status()
        );

        let mut unknown_field =
            serde_json::to_value(native_editor_envelope(&Uuid::now_v7().to_string()))?;
        unknown_field["smuggled"] = json!("free text");
        let response = http
            .post(&endpoint)
            .header("x-hsk-session-token", &token)
            .json(&unknown_field)
            .send()
            .await?;
        assert!(
            response.status().is_client_error(),
            "unknown envelope field must be rejected by the mounted route, got {}",
            response.status()
        );

        let after_fr = native_editor_fr_row_count(&state).await?;
        let after_ledger = native_editor_ledger_row_count(&state).await?;
        assert_eq!(
            after_fr, before_fr,
            "rejected bodies must emit no native-editor FR row"
        );
        assert_eq!(
            after_ledger, before_ledger,
            "rejected bodies must emit no EventLedger receipt"
        );
        server.abort();
        Ok(())
    }

    fn documented_payloads() -> Vec<(NativeEditorFrEventKind, Value)> {
        vec![
            (
                NativeEditorFrEventKind::DocumentSaved,
                json!({"document_id":"doc-1","content_hash":"a".repeat(64),"save_receipt_event_id":"KE-receipt","actor_kind":"operator","kernel_task_run_id":"task-1","session_run_id":"session-1","correlation_id":"correlation-1"}),
            ),
            (
                NativeEditorFrEventKind::CodeEdit,
                json!({"file_path":"src/main.rs","line_delta":1}),
            ),
            (
                NativeEditorFrEventKind::EmbedCreated,
                json!({"embed_kind":"stage_capture","item_id":"artifact-1","target_document_id":"doc-1"}),
            ),
            (
                NativeEditorFrEventKind::CanvasNodePlaced,
                json!({"canvas_id":"canvas-1","node_id":"node-1","node_kind":"note"}),
            ),
            (
                NativeEditorFrEventKind::CrossRefInserted,
                json!({"ref_kind":"symbol","symbol_entity_id":"symbol-1","target_document_id":"doc-1"}),
            ),
            (NativeEditorFrEventKind::UndoFired, json!({"scope":"local"})),
            (
                NativeEditorFrEventKind::RouteToStage,
                json!({"content_kind":"selection"}),
            ),
            (
                NativeEditorFrEventKind::StageEmbedBack,
                json!({"artifact_id":"artifact-1","target_pane_id":"pane-rich","sha256":"c".repeat(64),"manifest_ref":"manifest-1"}),
            ),
            (
                NativeEditorFrEventKind::CalendarEventBound,
                json!({"date":"2026-07-02","document_id":"doc-1","calendar_event_id":"calendar-1"}),
            ),
            (
                NativeEditorFrEventKind::ActivitySpanCorrelated,
                json!({"calendar_event_id":"calendar-1","activity_span_id":"span-1","edited_document_ids":["doc-1"]}),
            ),
            (
                NativeEditorFrEventKind::LocusRefResolved,
                json!({"locus_uri":"locus://wp/WP-KERNEL-012","target_kind":"work_packet","target_id":"WP-KERNEL-012"}),
            ),
            (
                NativeEditorFrEventKind::LocusReverseLookup,
                json!({"locus_uri":"locus://wp/WP-KERNEL-012","document_ids":["doc-1"]}),
            ),
        ]
    }

    /// AC-109-1: every documented kind decodes and satisfies its closed payload contract.
    #[test]
    fn native_editor_envelope_accepts_all_documented_kinds() {
        for (kind, payload) in documented_payloads() {
            let body = json!({
                "schema_version": NATIVE_EDITOR_SCHEMA_VERSION,
                "event_id": Uuid::now_v7().to_string(),
                "ts_utc": "2026-07-02T04:08:05Z",
                "kind": kind.as_str(),
                "actor_id": "a",
                "pane_id": "pane-rich",
                "workspace_id": "WS-NE-1",
                "payload": payload,
            });
            let parsed: NativeEditorFrEventV0_1 = serde_json::from_value(body)
                .unwrap_or_else(|e| panic!("kind {} should parse: {e}", kind.as_str()));
            assert!(
                valid_native_editor_payload(parsed.kind, &parsed.payload),
                "kind {} should satisfy its exact payload contract",
                kind.as_str()
            );
        }
    }

    #[tokio::test]
    async fn native_editor_handler_accepts_all_documented_kinds_and_persists_each(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(state) = setup_state().await? else {
            return Ok(());
        };

        for (kind, payload) in documented_payloads() {
            let event = if kind == NativeEditorFrEventKind::DocumentSaved {
                authentic_document_saved_envelope(&state).await?
            } else {
                let mut event = native_editor_envelope(&Uuid::now_v7().to_string());
                event.kind = kind;
                event.payload = payload;
                event
            };
            let workspace_id = event.canonical_workspace_id().to_owned();
            let event_id = durable_id(&event);
            let aggregate_id = event_id.to_string();

            let Json(ack) = ingest_native_editor(State(state.clone()), Json(event))
                .await
                .map_err(|(status, _)| {
                    format!(
                        "handler rejected documented kind {}: {status}",
                        kind.as_str()
                    )
                })?;
            assert_eq!(ack["ok"], true, "{} acknowledgement", kind.as_str());
            assert_eq!(ack["kind"], kind.as_str());

            let Json(events) = list_events_scoped(
                State(state.clone()),
                Query(EventFilter {
                    event_id: Some(event_id),
                    wsid: Some(workspace_id.clone()),
                    ..Default::default()
                }),
            )
            .await
            .map_err(|_| format!("GET failed for documented kind {}", kind.as_str()))?;
            assert_eq!(events.len(), 1, "{} must persist one FR row", kind.as_str());
            assert_eq!(events[0].payload["kind"], kind.as_str());
            assert_eq!(events[0].payload["workspace_id"], workspace_id);

            let completion_count = native_editor_ledger_events(&state, &aggregate_id)
                .await?
                .into_iter()
                .filter(|row| row.event_type == KernelEventType::FlightRecorderMirrorRecorded)
                .count();
            assert_eq!(
                completion_count,
                1,
                "{} must persist one completion receipt",
                kind.as_str()
            );
        }
        Ok(())
    }

    #[test]
    fn native_editor_stage_correlation_extension_is_closed_and_non_empty() {
        let correlated = [
            (
                NativeEditorFrEventKind::RouteToStage,
                json!({
                    "content_kind": "selection",
                    "causal_action_id": "stage-action-1",
                }),
            ),
            (
                NativeEditorFrEventKind::StageEmbedBack,
                json!({
                    "artifact_id": "artifact-1",
                    "target_pane_id": "pane-rich",
                    "sha256": "c".repeat(64),
                    "manifest_ref": "manifest-1",
                    "causal_action_id": "stage-action-1",
                }),
            ),
        ];

        for (kind, payload) in correlated {
            assert!(
                valid_native_editor_payload(kind, &payload),
                "{} rejects its correlated hsk.native_editor@0.1 payload",
                kind.as_str()
            );

            for invalid in [json!(""), json!("   "), Value::Null] {
                let mut malformed = payload.clone();
                malformed
                    .as_object_mut()
                    .expect("correlated fixture is an object")
                    .insert("causal_action_id".to_owned(), invalid);
                assert!(
                    !valid_native_editor_payload(kind, &malformed),
                    "{} accepts an empty or non-string causal_action_id",
                    kind.as_str()
                );
            }

            let mut unknown = payload;
            unknown
                .as_object_mut()
                .expect("correlated fixture is an object")
                .insert("unknown_correlation".to_owned(), json!("smuggled"));
            assert!(
                !valid_native_editor_payload(kind, &unknown),
                "{} accepts an unknown correlated payload field",
                kind.as_str()
            );
        }
    }

    #[test]
    fn native_editor_documented_payloads_reject_unknown_missing_and_wrong_typed_fields() {
        for (kind, payload) in documented_payloads() {
            let mut unknown = payload.clone();
            unknown
                .as_object_mut()
                .expect("documented fixture is an object")
                .insert("unknown".to_owned(), json!("smuggled"));
            assert!(
                !valid_native_editor_payload(kind, &unknown),
                "{} accepts an unknown field",
                kind.as_str()
            );

            let keys = payload
                .as_object()
                .expect("documented fixture is an object")
                .keys()
                .cloned()
                .collect::<Vec<_>>();
            for key in keys {
                let mut missing = payload.clone();
                missing
                    .as_object_mut()
                    .expect("documented fixture is an object")
                    .remove(&key);
                assert!(
                    !valid_native_editor_payload(kind, &missing),
                    "{} accepts missing documented field {key}",
                    kind.as_str()
                );

                let mut wrong_type = payload.clone();
                wrong_type
                    .as_object_mut()
                    .expect("documented fixture is an object")
                    .insert(key.clone(), Value::Null);
                assert!(
                    !valid_native_editor_payload(kind, &wrong_type),
                    "{} accepts null for documented field {key}",
                    kind.as_str()
                );
            }
        }
    }

    #[tokio::test]
    async fn native_editor_handler_rejects_every_documented_payload_boundary_corruption(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(state) = setup_state().await? else {
            return Ok(());
        };

        for (kind, payload) in documented_payloads() {
            let keys = payload
                .as_object()
                .expect("documented fixture is an object")
                .keys()
                .cloned()
                .collect::<Vec<_>>();
            for key in keys {
                for corruption in ["missing", "null"] {
                    let mut event = native_editor_envelope(&Uuid::now_v7().to_string());
                    event.kind = kind;
                    event.payload = payload.clone();
                    let map = event
                        .payload
                        .as_object_mut()
                        .expect("documented fixture is an object");
                    match corruption {
                        "missing" => {
                            map.remove(&key);
                        }
                        "null" => {
                            map.insert(key.clone(), Value::Null);
                        }
                        _ => unreachable!(),
                    }
                    assert!(
                        matches!(
                            ingest_native_editor(State(state.clone()), Json(event)).await,
                            Err((StatusCode::BAD_REQUEST, _))
                        ),
                        "handler accepted {corruption} {}.{key}",
                        kind.as_str()
                    );
                }
            }

            let mut unknown = native_editor_envelope(&Uuid::now_v7().to_string());
            unknown.kind = kind;
            unknown.payload = payload;
            unknown
                .payload
                .as_object_mut()
                .expect("documented fixture is an object")
                .insert("unknown".to_owned(), json!("smuggled"));
            assert!(
                matches!(
                    ingest_native_editor(State(state.clone()), Json(unknown)).await,
                    Err((StatusCode::BAD_REQUEST, _))
                ),
                "handler accepted an unknown field for {}",
                kind.as_str()
            );
        }
        Ok(())
    }

    #[tokio::test]
    async fn native_editor_handler_accepts_correlated_stage_payloads_and_rejects_bad_correlation(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(state) = setup_state().await? else {
            return Ok(());
        };
        let correlated = [
            (
                NativeEditorFrEventKind::RouteToStage,
                json!({
                    "content_kind": "selection",
                    "causal_action_id": "stage-action-handler-1",
                }),
            ),
            (
                NativeEditorFrEventKind::StageEmbedBack,
                json!({
                    "artifact_id": "artifact-handler-1",
                    "target_pane_id": "pane-rich",
                    "sha256": "d".repeat(64),
                    "manifest_ref": "manifest-handler-1",
                    "causal_action_id": "stage-action-handler-1",
                }),
            ),
        ];

        for (kind, payload) in correlated {
            let mut accepted = native_editor_envelope(&Uuid::now_v7().to_string());
            accepted.kind = kind;
            accepted.payload = payload.clone();
            let Json(ack) = ingest_native_editor(State(state.clone()), Json(accepted))
                .await
                .map_err(|(status, _)| {
                    format!("handler rejected correlated {}: {status}", kind.as_str())
                })?;
            assert_eq!(ack["ok"], true);
            assert_eq!(ack["kind"], kind.as_str());

            for invalid in [json!(""), json!("   "), Value::Null] {
                let mut rejected = native_editor_envelope(&Uuid::now_v7().to_string());
                rejected.kind = kind;
                rejected.payload = payload.clone();
                rejected
                    .payload
                    .as_object_mut()
                    .expect("correlated fixture is an object")
                    .insert("causal_action_id".to_owned(), invalid);
                assert!(matches!(
                    ingest_native_editor(State(state.clone()), Json(rejected)).await,
                    Err((StatusCode::BAD_REQUEST, _))
                ));
            }

            let mut unknown = native_editor_envelope(&Uuid::now_v7().to_string());
            unknown.kind = kind;
            unknown.payload = payload;
            unknown
                .payload
                .as_object_mut()
                .expect("correlated fixture is an object")
                .insert("unknown_correlation".to_owned(), json!("smuggled"));
            assert!(matches!(
                ingest_native_editor(State(state.clone()), Json(unknown)).await,
                Err((StatusCode::BAD_REQUEST, _))
            ));
        }
        Ok(())
    }

    /// AC-109-1: the handler fails closed on a wrong schema version, a non-UUID event_id,
    /// or a non-object payload (no top-level free-text smuggling).
    #[tokio::test]
    async fn native_editor_event_handler_fails_closed() -> Result<(), Box<dyn std::error::Error>> {
        let Some(state) = setup_state().await? else {
            return Ok(());
        };

        let mut wrong_schema = native_editor_envelope(&Uuid::now_v7().to_string());
        wrong_schema.schema_version = "wrong@0.0".to_string();
        assert!(matches!(
            ingest_native_editor(State(state.clone()), Json(wrong_schema)).await,
            Err((StatusCode::BAD_REQUEST, _))
        ));

        let mut bad_id = native_editor_envelope("not-a-uuid");
        bad_id.payload = Value::Null;
        assert!(matches!(
            ingest_native_editor(State(state.clone()), Json(bad_id)).await,
            Err((StatusCode::BAD_REQUEST, _))
        ));

        let mut free_text_payload = native_editor_envelope(&Uuid::now_v7().to_string());
        free_text_payload.payload = json!("free text string");
        assert!(matches!(
            ingest_native_editor(State(state.clone()), Json(free_text_payload)).await,
            Err((StatusCode::BAD_REQUEST, _))
        ));

        let mut invalid_timestamp = native_editor_envelope(&Uuid::now_v7().to_string());
        invalid_timestamp.ts_utc = "not-rfc3339".to_owned();
        assert!(matches!(
            ingest_native_editor(State(state.clone()), Json(invalid_timestamp)).await,
            Err((StatusCode::BAD_REQUEST, _))
        ));

        let mut incomplete_document = native_editor_envelope(&Uuid::now_v7().to_string());
        incomplete_document.kind = NativeEditorFrEventKind::DocumentSaved;
        incomplete_document.payload = json!({"document_id": "DOC-MISSING-HASH"});
        assert!(matches!(
            ingest_native_editor(State(state.clone()), Json(incomplete_document)).await,
            Err((StatusCode::BAD_REQUEST, _))
        ));

        let mut missing_pane = native_editor_envelope(&Uuid::now_v7().to_string());
        missing_pane.pane_id.clear();
        assert!(matches!(
            ingest_native_editor(State(state.clone()), Json(missing_pane)).await,
            Err((StatusCode::BAD_REQUEST, _))
        ));

        // A blank body workspace no longer "falls back" to anything: it disagrees with the
        // authenticated path workspace, so it is a workspace rejection, not a shape error.
        let mut blank_workspace = native_editor_envelope(&Uuid::now_v7().to_string());
        blank_workspace.workspace_id = Some(String::new());
        assert!(matches!(
            record_native_editor_event(
                State(state.clone()),
                Path(TEST_WORKSPACE_ID.to_owned()),
                Extension(test_recorder_authority(TEST_WORKSPACE_ID)),
                Json(blank_workspace),
            )
            .await,
            Err((StatusCode::FORBIDDEN, _))
        ));

        // An unknown path workspace is denied (never implicitly created) and is indistinguishable
        // from an unauthorized one, so the route discloses nothing about workspace existence.
        let unknown_workspace_event =
            native_editor_envelope_in("WS-DOES-NOT-EXIST", &Uuid::now_v7().to_string());
        assert!(matches!(
            ingest_native_editor(State(state.clone()), Json(unknown_workspace_event)).await,
            Err((StatusCode::FORBIDDEN, _))
        ));
        Ok(())
    }

    #[tokio::test]
    async fn document_saved_requires_exact_canonical_save_receipt(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(state) = setup_state().await? else {
            return Ok(());
        };
        let authentic = authentic_document_saved_envelope(&state).await?;

        let mut missing = authentic.clone();
        missing
            .payload
            .as_object_mut()
            .expect("document payload")
            .remove("save_receipt_event_id");
        assert!(matches!(
            ingest_native_editor(State(state.clone()), Json(missing)).await,
            Err((StatusCode::BAD_REQUEST, _))
        ));

        let mut fabricated = authentic.clone();
        fabricated.payload["save_receipt_event_id"] = json!(format!("KE-{}", Uuid::now_v7()));
        assert!(matches!(
            ingest_native_editor(State(state.clone()), Json(fabricated)).await,
            Err((StatusCode::BAD_REQUEST, _))
        ));

        let mut corrupt = authentic.clone();
        corrupt.payload["content_hash"] = json!("b".repeat(64));
        assert!(matches!(
            ingest_native_editor(State(state.clone()), Json(corrupt)).await,
            Err((StatusCode::BAD_REQUEST, _))
        ));

        let Json(ack) = ingest_native_editor(State(state), Json(authentic))
            .await
            .map_err(|(status, _)| format!("authentic receipt rejected: {status}"))?;
        assert_eq!(ack["ok"], true);
        Ok(())
    }

    /// WP-KERNEL-012 MT-120 / AC-120-2 — THE BINDING IS NOT WEAKENED.
    ///
    /// A save receipt minted by principal A is NOT claimable by principal B, and the rejection leaves
    /// ZERO residue (validation runs before the pending EventLedger write and before any FR write).
    /// The SAME envelope is then re-pointed at a receipt minted by the claiming principal and
    /// succeeds — proving the rejection was the PRINCIPAL and not some unrelated conjunct.
    #[tokio::test]
    async fn document_saved_receipt_minted_by_another_principal_is_unclaimable(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(state) = setup_state().await? else {
            return Ok(());
        };
        // Principal A is some OTHER live native process; the ingesting principal is TEST_ACTOR_ID.
        let other_principal = "handshake-native:999999:0f0f0f0f";
        assert_ne!(other_principal, TEST_ACTOR_ID);
        let foreign = authentic_document_saved_envelope_minted_by(&state, other_principal).await?;

        let fr_before = native_editor_fr_row_count(&state).await?;
        let ledger_before = native_editor_ledger_row_count(&state).await?;
        let (status, _) = ingest_native_editor(State(state.clone()), Json(foreign.clone()))
            .await
            .expect_err("a receipt minted by another principal must not be claimable");
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(
            native_editor_fr_row_count(&state).await?,
            fr_before,
            "cross-principal rejection wrote a Flight Recorder row"
        );
        assert_eq!(
            native_editor_ledger_row_count(&state).await?,
            ledger_before,
            "cross-principal rejection wrote an EventLedger pending row"
        );

        // Flip ONLY the receipt to one minted by the claiming principal: the same envelope shape now
        // ingests, so nothing but the principal was ever in question.
        let owned = authentic_document_saved_envelope_minted_by(&state, TEST_ACTOR_ID).await?;
        let mut same_shape = foreign;
        same_shape.workspace_id = owned.workspace_id.clone();
        same_shape.payload = owned.payload.clone();
        let Json(ack) = ingest_native_editor(State(state.clone()), Json(same_shape))
            .await
            .map_err(|(status, _)| format!("own-principal receipt rejected: {status}"))?;
        assert_eq!(ack["ok"], true);
        Ok(())
    }

    /// WP-KERNEL-012 MT-120 — FAIL CLOSED on an absent ownership anchor. A legacy or unauthenticated
    /// save leaves a receipt with no `minted_by_principal`; that receipt is UNCLAIMABLE, and so is a
    /// blank one. The clause is unconditional: absence is never "assume it matches".
    #[tokio::test]
    async fn document_saved_receipt_without_minted_by_principal_is_unclaimable(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(state) = setup_state().await? else {
            return Ok(());
        };
        for mutate in [None, Some(""), Some("   ")] {
            let envelope = authentic_document_saved_envelope(&state).await?;
            let receipt_id = envelope.payload["save_receipt_event_id"]
                .as_str()
                .expect("fixture receipt id")
                .to_owned();
            let document_id = envelope.payload["document_id"]
                .as_str()
                .expect("fixture document id");
            let receipt = state
                .storage
                .list_kernel_events_for_aggregate("knowledge_rich_document", document_id)
                .await?
                .into_iter()
                .find(|event| event.event_id == receipt_id)
                .expect("fixture receipt row");
            let mut receipt_payload = receipt.payload;
            let receipt_object = receipt_payload
                .as_object_mut()
                .expect("receipt payload object");
            let field = crate::api::knowledge_documents::SAVE_RECEIPT_MINTED_BY_PRINCIPAL_FIELD;
            match mutate {
                None => {
                    receipt_object.remove(field);
                }
                Some(value) => {
                    receipt_object.insert(field.to_owned(), Value::String(value.to_owned()));
                }
            }
            set_event_payload(&state, receipt_id, receipt_payload).await?;

            let fr_before = native_editor_fr_row_count(&state).await?;
            let ledger_before = native_editor_ledger_row_count(&state).await?;
            let (status, _) = ingest_native_editor(State(state.clone()), Json(envelope))
                .await
                .expect_err("a receipt without a server-written ownership anchor is unclaimable");
            assert_eq!(
                status,
                StatusCode::BAD_REQUEST,
                "minted_by_principal={mutate:?} must reject"
            );
            assert_eq!(
                native_editor_fr_row_count(&state).await?,
                fr_before,
                "absent-anchor rejection wrote a Flight Recorder row"
            );
            assert_eq!(
                native_editor_ledger_row_count(&state).await?,
                ledger_before,
                "absent-anchor rejection wrote an EventLedger pending row"
            );
        }
        Ok(())
    }

    /// A legacy FR-only partial write is repaired on replay: the handler appends the missing durable
    /// EventLedger receipt before recognizing the existing Flight Recorder row as idempotent.
    #[tokio::test]
    async fn native_editor_replay_repairs_fr_only_partial_write(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(state) = setup_state().await? else {
            return Ok(());
        };
        let event_id = Uuid::now_v7();
        let event = native_editor_envelope(&event_id.to_string());
        let aggregate_id = durable_id(&event).to_string();
        let fr_only = native_editor_fr_event_from_envelope(&event)
            .map_err(|_| "failed to build exact FR-only fixture")?;
        state.flight_recorder.record_event(fr_only).await?;

        let Json(ack) = ingest_native_editor(State(state.clone()), Json(event))
            .await
            .map_err(|(code, _)| format!("repair replay failed: {code}"))?;
        assert_eq!(ack["idempotent"], true);
        let ledger_count = native_editor_ledger_events(&state, aggregate_id)
            .await?
            .into_iter()
            .filter(|row| row.event_type == KernelEventType::FlightRecorderMirrorRecorded)
            .count();
        assert_eq!(
            ledger_count, 1,
            "replay repaired the missing embedded EventLedger mirror"
        );
        Ok(())
    }

    // =====================================================================================
    // MT-109 FAIL_V4 remediation: HTTP-level authorization proofs against the REAL router.
    // =====================================================================================

    /// The `handshake-native:<pid>:<birth>` identity `capture_context` mints for this process.
    fn authenticated_actor_id(token: &str) -> String {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-hsk-session-token",
            HeaderValue::from_str(token).expect("session token header"),
        );
        crate::api::stage::capture_context(&headers)
            .expect("live native binding")
            .actor_id
    }

    fn native_editor_endpoint(base: &str, workspace_id: &str) -> String {
        format!("{base}/workspaces/{workspace_id}/flight_recorder/native_editor_event")
    }

    fn runtime_chat_endpoint(base: &str, workspace_id: &str) -> String {
        format!("{base}/workspaces/{workspace_id}/flight_recorder/runtime_chat_event")
    }

    fn runtime_chat_body(session_id: Uuid, message_id: Uuid, wsid: Option<&str>) -> Value {
        let mut body = json!({
            "schema_version": "hsk.fr.runtime_chat@0.1",
            "event_id": "FR-EVT-RUNTIME-CHAT-101",
            "ts_utc": "2026-07-02T04:08:05Z",
            "session_id": session_id.to_string(),
            "type": "runtime_chat_message_appended",
            "message_id": message_id.to_string(),
            "role": "user",
            "body_sha256": "e".repeat(64),
        });
        if let Some(wsid) = wsid {
            body["wsid"] = json!(wsid);
        }
        body
    }

    /// Missing or forged session binding => 401 on EVERY route in the group, with zero durable
    /// residue and one exact redacted deny audit per attempt.
    #[tokio::test]
    async fn flight_recorder_routes_reject_unauthenticated_callers_with_zero_residue(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(state) = setup_state().await? else {
            return Ok(());
        };
        let _env_lock = FR_AUTH_ENV_LOCK.lock().expect("fr auth env lock");
        let (token, _binding) = install_native_binding()?;
        let before_fr = native_editor_fr_row_count(&state).await?;
        let before_ledger = native_editor_ledger_row_count(&state).await?;
        let (base, http, server) = serve_test_router(routes(state.clone())).await;

        let envelope = serde_json::to_value(native_editor_envelope(&Uuid::now_v7().to_string()))?;
        let chat = runtime_chat_body(Uuid::now_v7(), Uuid::now_v7(), None);

        for (label, response) in [
            (
                "GET /flight_recorder without binding",
                http.get(format!("{base}/flight_recorder?wsid={TEST_WORKSPACE_ID}"))
                    .send()
                    .await?,
            ),
            (
                "GET /events without binding",
                http.get(format!("{base}/events?wsid={TEST_WORKSPACE_ID}"))
                    .send()
                    .await?,
            ),
            (
                "GET /flight_recorder with a forged token",
                http.get(format!("{base}/flight_recorder?wsid={TEST_WORKSPACE_ID}"))
                    .header("x-hsk-session-token", "b".repeat(64))
                    .send()
                    .await?,
            ),
            (
                "POST native_editor_event without binding",
                http.post(native_editor_endpoint(&base, TEST_WORKSPACE_ID))
                    .json(&envelope)
                    .send()
                    .await?,
            ),
            (
                "POST native_editor_event with a forged token",
                http.post(native_editor_endpoint(&base, TEST_WORKSPACE_ID))
                    .header("x-hsk-session-token", "c".repeat(64))
                    .json(&envelope)
                    .send()
                    .await?,
            ),
            (
                "POST runtime_chat_event without binding",
                http.post(runtime_chat_endpoint(&base, TEST_WORKSPACE_ID))
                    .json(&chat)
                    .send()
                    .await?,
            ),
        ] {
            assert_eq!(
                response.status(),
                StatusCode::UNAUTHORIZED,
                "{label} must be 401"
            );
            let body: Value = response.json().await?;
            assert_eq!(body["error"], "HSK-401-FR-SESSION", "{label} error code");
        }

        assert_eq!(
            native_editor_fr_row_count(&state).await?,
            before_fr,
            "unauthenticated calls must leave no native-editor FR row"
        );
        assert_eq!(
            native_editor_ledger_row_count(&state).await?,
            before_ledger,
            "unauthenticated calls must leave no EventLedger receipt"
        );

        // Every denial is attributable and redacted.
        for capability_id in [
            FR_READ_CAPABILITY,
            FR_INGEST_NATIVE_EDITOR_CAPABILITY,
            FR_INGEST_RUNTIME_CHAT_CAPABILITY,
        ] {
            let denies = capability_decisions(&state, capability_id, "deny").await?;
            assert!(!denies.is_empty(), "{capability_id} denial must be audited");
            for deny in &denies {
                let payload = deny.payload.as_object().expect("audit payload object");
                assert_eq!(
                    payload.len(),
                    4,
                    "the capability audit must carry exactly the redacted contract keys"
                );
                assert_eq!(payload["capability_id"], capability_id);
                assert_eq!(payload["decision_outcome"], "deny");
                assert_eq!(payload["actor_id"], "unauthenticated-native-client");
                let rendered = serde_json::to_string(&deny.payload)?;
                assert!(
                    !rendered.contains(&token) && !rendered.contains("body_sha256"),
                    "the audit must never carry the session token or request body"
                );
            }
        }
        server.abort();
        Ok(())
    }

    /// A VALID authenticated context that lacks the required capability => 403 with zero residue
    /// and an exact deny audit. `fr.read.global` is granted to no shipped profile, so an
    /// authenticated caller that omits `?wsid=` cannot enumerate across workspaces.
    #[tokio::test]
    async fn recorder_read_without_scope_is_denied_for_lack_of_global_capability(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(state) = setup_state().await? else {
            return Ok(());
        };
        let _env_lock = FR_AUTH_ENV_LOCK.lock().expect("fr auth env lock");
        let (token, _binding) = install_native_binding()?;
        let actor_id = authenticated_actor_id(&token);
        let before_fr = native_editor_fr_row_count(&state).await?;
        let (base, http, server) = serve_test_router(routes(state.clone())).await;

        for url in [
            format!("{base}/flight_recorder"),
            format!("{base}/events"),
            // A blank scope must not read as "all workspaces".
            format!("{base}/flight_recorder?wsid="),
        ] {
            let response = http
                .get(&url)
                .header("x-hsk-session-token", &token)
                .send()
                .await?;
            assert_eq!(
                response.status(),
                StatusCode::FORBIDDEN,
                "unscoped recorder enumeration must be 403 at {url}"
            );
            let body: Value = response.json().await?;
            assert_eq!(body["error"], "HSK-403-FR-CAPABILITY");
        }

        let denies = capability_decisions(&state, FR_READ_GLOBAL_CAPABILITY, "deny").await?;
        assert_eq!(
            denies.len(),
            3,
            "one exact deny audit per unscoped read attempt"
        );
        for deny in &denies {
            assert_eq!(deny.payload["actor_id"], actor_id);
            assert_eq!(
                deny.capability_id.as_deref(),
                Some(FR_READ_GLOBAL_CAPABILITY)
            );
        }
        assert!(
            capability_decisions(&state, FR_READ_CAPABILITY, "allow")
                .await?
                .len()
                >= 3,
            "the base read capability was allowed before the scope escalation was denied"
        );
        assert_eq!(
            native_editor_fr_row_count(&state).await?,
            before_fr,
            "a denied read must not mutate recorder state"
        );
        server.abort();
        Ok(())
    }

    /// A spoofed actor or a body workspace that disagrees with the authenticated path is rejected
    /// BEFORE any durable write; a clean envelope is accepted with server-derived attribution.
    #[tokio::test]
    async fn native_editor_ingest_rejects_spoofed_identity_and_derives_attribution(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(state) = setup_state().await? else {
            return Ok(());
        };
        let _env_lock = FR_AUTH_ENV_LOCK.lock().expect("fr auth env lock");
        let (token, _binding) = install_native_binding()?;
        let actor_id = authenticated_actor_id(&token);
        assert!(actor_id.starts_with("handshake-native:"));
        let before_fr = native_editor_fr_row_count(&state).await?;
        let before_ledger = native_editor_ledger_row_count(&state).await?;
        let (base, http, server) = serve_test_router(routes(state.clone())).await;
        let endpoint = native_editor_endpoint(&base, TEST_WORKSPACE_ID);

        let mut spoofed_actor =
            serde_json::to_value(native_editor_envelope(&Uuid::now_v7().to_string()))?;
        spoofed_actor["actor_id"] = json!("operator-i-am-not");
        let response = http
            .post(&endpoint)
            .header("x-hsk-session-token", &token)
            .json(&spoofed_actor)
            .send()
            .await?;
        assert_eq!(response.status(), StatusCode::FORBIDDEN, "spoofed actor_id");
        assert_eq!(
            response.json::<Value>().await?["error"],
            "HSK-403-FR-ACTOR-SPOOF"
        );

        let mut spoofed_kind =
            serde_json::to_value(native_editor_envelope(&Uuid::now_v7().to_string()))?;
        spoofed_kind["actor_id"] = json!(actor_id);
        spoofed_kind["actor_kind"] = json!("system");
        let response = http
            .post(&endpoint)
            .header("x-hsk-session-token", &token)
            .json(&spoofed_kind)
            .send()
            .await?;
        assert_eq!(
            response.status(),
            StatusCode::FORBIDDEN,
            "spoofed actor_kind"
        );

        let mut spoofed_workspace =
            serde_json::to_value(native_editor_envelope(&Uuid::now_v7().to_string()))?;
        spoofed_workspace["actor_id"] = json!(actor_id);
        spoofed_workspace["workspace_id"] = json!(OTHER_TEST_WORKSPACE_ID);
        let response = http
            .post(&endpoint)
            .header("x-hsk-session-token", &token)
            .json(&spoofed_workspace)
            .send()
            .await?;
        assert_eq!(
            response.status(),
            StatusCode::FORBIDDEN,
            "body workspace must not contradict the authenticated path"
        );
        assert_eq!(
            response.json::<Value>().await?["error"],
            "HSK-403-FR-WORKSPACE"
        );

        assert_eq!(
            native_editor_fr_row_count(&state).await?,
            before_fr,
            "spoof attempts must leave no native-editor FR row"
        );
        assert_eq!(
            native_editor_ledger_row_count(&state).await?,
            before_ledger,
            "spoof attempts must leave no EventLedger receipt"
        );

        // A clean envelope that names NO identity at all is accepted, and the server fills it.
        let client_event_id = Uuid::now_v7();
        let mut clean = serde_json::to_value(native_editor_envelope(&client_event_id.to_string()))?;
        clean["actor_id"] = Value::Null;
        clean["actor_kind"] = Value::Null;
        clean["workspace_id"] = Value::Null;
        let response = http
            .post(&endpoint)
            .header("x-hsk-session-token", &token)
            .json(&clean)
            .send()
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        let ack: Value = response.json().await?;
        let durable = workspace_scoped_fr_event_id(TEST_WORKSPACE_ID, client_event_id);
        assert_eq!(ack["fr_event_id"], durable.to_string());
        assert_eq!(ack["actor_id"], actor_id);
        assert_eq!(ack["workspace_id"], TEST_WORKSPACE_ID);

        let stored = state
            .flight_recorder
            .list_events(crate::flight_recorder::EventFilter {
                event_id: Some(durable),
                ..Default::default()
            })
            .await?;
        assert_eq!(stored.len(), 1);
        assert_eq!(
            stored[0].actor_id, actor_id,
            "server-derived FR attribution"
        );
        assert!(stored[0].wsids.contains(&TEST_WORKSPACE_ID.to_string()));

        let ledger_actor = native_editor_ledger_events(&state, durable.to_string())
            .await?
            .into_iter()
            .find(|row| row.event_type == KernelEventType::FlightRecorderMirrorRecorded)
            .expect("completion receipt")
            .actor
            .actor_id()
            .to_owned();
        assert_eq!(
            ledger_actor, actor_id,
            "the durable EventLedger attribution is server-derived"
        );

        let allows =
            capability_decisions(&state, FR_INGEST_NATIVE_EDITOR_CAPABILITY, "allow").await?;
        assert!(!allows.is_empty(), "accepted ingest must be audited");
        for allow in &allows {
            assert_eq!(allow.payload["actor_id"], actor_id);
            assert_eq!(
                allow.payload.as_object().expect("audit object").len(),
                4,
                "redacted audit contract"
            );
        }
        server.abort();
        Ok(())
    }

    /// Idempotency and conflict ownership are partitioned per authenticated workspace: one
    /// workspace cannot pre-seed another workspace's `event_id`, turn its retry into a conflict,
    /// read its row, or reconcile its receipt.
    #[tokio::test]
    async fn cross_workspace_event_id_preemption_cannot_conflict_or_leak(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(state) = setup_state().await? else {
            return Ok(());
        };
        let _env_lock = FR_AUTH_ENV_LOCK.lock().expect("fr auth env lock");
        let (token, _binding) = install_native_binding()?;
        let (base, http, server) = serve_test_router(routes(state.clone())).await;

        // The attacker pre-seeds a client event id inside the workspace it DOES hold.
        let contested = Uuid::now_v7();
        let mut attacker = serde_json::to_value(native_editor_envelope_in(
            OTHER_TEST_WORKSPACE_ID,
            &contested.to_string(),
        ))?;
        attacker["pane_id"] = json!("attacker-pane");
        attacker["actor_id"] = Value::Null;
        let response = http
            .post(native_editor_endpoint(&base, OTHER_TEST_WORKSPACE_ID))
            .header("x-hsk-session-token", &token)
            .json(&attacker)
            .send()
            .await?;
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "attacker's own workspace"
        );

        // The victim workspace submits the SAME client id with different content. Before the
        // partition this was a 409 that denied the victim its own event.
        let mut victim = serde_json::to_value(native_editor_envelope_in(
            TEST_WORKSPACE_ID,
            &contested.to_string(),
        ))?;
        victim["pane_id"] = json!("victim-pane");
        victim["actor_id"] = Value::Null;
        let response = http
            .post(native_editor_endpoint(&base, TEST_WORKSPACE_ID))
            .header("x-hsk-session-token", &token)
            .json(&victim)
            .send()
            .await?;
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "a foreign workspace must not be able to convert this into a conflict"
        );
        let victim_ack: Value = response.json().await?;
        assert_eq!(victim_ack["idempotent"], false);

        let attacker_durable = workspace_scoped_fr_event_id(OTHER_TEST_WORKSPACE_ID, contested);
        let victim_durable = workspace_scoped_fr_event_id(TEST_WORKSPACE_ID, contested);
        assert_ne!(attacker_durable, victim_durable);
        assert_eq!(victim_ack["fr_event_id"], victim_durable.to_string());

        // Two disjoint aggregates, two disjoint pending/completion state machines.
        for (workspace_id, durable) in [
            (OTHER_TEST_WORKSPACE_ID, attacker_durable),
            (TEST_WORKSPACE_ID, victim_durable),
        ] {
            let receipts = native_editor_ledger_events(&state, durable.to_string())
                .await?
                .len();
            assert_eq!(
                receipts, 2,
                "{workspace_id} owns its own pending+completion"
            );
        }

        // A scoped read cannot reach the other workspace's row, even by naming its exact id.
        let leaked = http
            .get(format!(
                "{base}/flight_recorder?wsid={TEST_WORKSPACE_ID}&event_id={attacker_durable}"
            ))
            .header("x-hsk-session-token", &token)
            .send()
            .await?;
        assert_eq!(leaked.status(), StatusCode::OK);
        let rows: Vec<FlightEvent> = leaked.json().await?;
        assert!(
            rows.is_empty(),
            "a cross-workspace event id must return an empty list, not the row and not an error"
        );

        // The scoped read returns exactly the caller's own row.
        let own = http
            .get(format!(
                "{base}/flight_recorder?wsid={TEST_WORKSPACE_ID}&event_id={victim_durable}"
            ))
            .header("x-hsk-session-token", &token)
            .send()
            .await?;
        let rows: Vec<FlightEvent> = own.json().await?;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].payload["pane_id"], "victim-pane");
        server.abort();
        Ok(())
    }

    /// Query filters cannot widen the authenticated workspace scope, and both GET aliases behave
    /// identically.
    #[tokio::test]
    async fn recorder_read_scope_cannot_be_widened_by_query_filters(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(state) = setup_state().await? else {
            return Ok(());
        };
        let _env_lock = FR_AUTH_ENV_LOCK.lock().expect("fr auth env lock");
        let (token, _binding) = install_native_binding()?;
        let actor_id = authenticated_actor_id(&token);
        let (base, http, server) = serve_test_router(routes(state.clone())).await;

        for workspace_id in [TEST_WORKSPACE_ID, OTHER_TEST_WORKSPACE_ID] {
            let mut body = serde_json::to_value(native_editor_envelope_in(
                workspace_id,
                &Uuid::now_v7().to_string(),
            ))?;
            body["actor_id"] = Value::Null;
            let response = http
                .post(native_editor_endpoint(&base, workspace_id))
                .header("x-hsk-session-token", &token)
                .json(&body)
                .send()
                .await?;
            assert_eq!(response.status(), StatusCode::OK);
        }

        for alias in ["flight_recorder", "events"] {
            // Both workspaces share one actor and one surface, so neither filter may widen scope.
            for query in [
                format!("wsid={TEST_WORKSPACE_ID}"),
                format!("wsid={TEST_WORKSPACE_ID}&actor_id={actor_id}"),
                format!("wsid={TEST_WORKSPACE_ID}&surface=pane-rich&event_type=system"),
                format!("wsid={TEST_WORKSPACE_ID}&actor=human"),
            ] {
                let response = http
                    .get(format!("{base}/{alias}?{query}"))
                    .header("x-hsk-session-token", &token)
                    .send()
                    .await?;
                assert_eq!(response.status(), StatusCode::OK, "{alias}?{query}");
                let rows: Vec<FlightEvent> = response.json().await?;
                assert!(
                    rows.iter()
                        .all(|row| row.wsids.contains(&TEST_WORKSPACE_ID.to_string())),
                    "{alias}?{query} leaked a row outside the authenticated workspace"
                );
                assert!(
                    rows.iter()
                        .all(|row| !row.wsids.contains(&OTHER_TEST_WORKSPACE_ID.to_string())),
                    "{alias}?{query} leaked the other workspace"
                );
            }
        }
        server.abort();
        Ok(())
    }

    /// Runtime-chat ingestion shares the same boundary: capability, authenticated path workspace,
    /// and a body `wsid` that may only confirm it.
    #[tokio::test]
    async fn runtime_chat_ingest_is_capability_gated_and_workspace_bound(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(state) = setup_state().await? else {
            return Ok(());
        };
        let _env_lock = FR_AUTH_ENV_LOCK.lock().expect("fr auth env lock");
        let (token, _binding) = install_native_binding()?;
        let (base, http, server) = serve_test_router(routes(state.clone())).await;
        let endpoint = runtime_chat_endpoint(&base, TEST_WORKSPACE_ID);

        let mismatched = runtime_chat_body(
            Uuid::now_v7(),
            Uuid::now_v7(),
            Some(OTHER_TEST_WORKSPACE_ID),
        );
        let response = http
            .post(&endpoint)
            .header("x-hsk-session-token", &token)
            .json(&mismatched)
            .send()
            .await?;
        assert_eq!(
            response.status(),
            StatusCode::FORBIDDEN,
            "a runtime-chat body wsid must not contradict the authenticated path"
        );

        let unknown_workspace = runtime_chat_endpoint(&base, "WS-DOES-NOT-EXIST");
        let response = http
            .post(&unknown_workspace)
            .header("x-hsk-session-token", &token)
            .json(&runtime_chat_body(Uuid::now_v7(), Uuid::now_v7(), None))
            .send()
            .await?;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let session_id = Uuid::now_v7();
        let response = http
            .post(&endpoint)
            .header("x-hsk-session-token", &token)
            .json(&runtime_chat_body(session_id, Uuid::now_v7(), None))
            .send()
            .await?;
        assert_eq!(response.status(), StatusCode::OK);

        let rows = state
            .flight_recorder
            .list_events(crate::flight_recorder::EventFilter {
                trace_id: Some(session_id),
                ..Default::default()
            })
            .await?;
        assert_eq!(rows.len(), 1);
        assert!(
            rows[0].wsids.contains(&TEST_WORKSPACE_ID.to_string()),
            "runtime-chat workspace attribution is taken from the authenticated path"
        );
        assert_eq!(
            rows[0].payload["wsid"], TEST_WORKSPACE_ID,
            "the stored envelope carries the server-derived workspace"
        );
        server.abort();
        Ok(())
    }

    /// The capability registry must actually carry the MT-109 recorder capabilities, and the
    /// unscoped-read escalation must stay ungranted.
    #[test]
    fn recorder_capabilities_are_registered_and_global_read_stays_ungranted() {
        let registry = CapabilityRegistry::new();
        for capability_id in [
            FR_READ_CAPABILITY,
            FR_READ_GLOBAL_CAPABILITY,
            FR_INGEST_RUNTIME_CHAT_CAPABILITY,
            FR_INGEST_NATIVE_EDITOR_CAPABILITY,
        ] {
            assert!(
                registry.is_valid(capability_id),
                "{capability_id} must be a canonical capability"
            );
        }
        for capability_id in [
            FR_READ_CAPABILITY,
            FR_INGEST_RUNTIME_CHAT_CAPABILITY,
            FR_INGEST_NATIVE_EDITOR_CAPABILITY,
        ] {
            assert!(
                matches!(
                    registry.profile_can(FR_CAPABILITY_PROFILE, capability_id),
                    Ok(true)
                ),
                "{capability_id} must be granted to the authenticated native profile"
            );
        }
        assert!(
            matches!(
                registry.profile_can(FR_CAPABILITY_PROFILE, FR_READ_GLOBAL_CAPABILITY),
                Ok(false)
            ),
            "unscoped cross-workspace recorder enumeration must stay fail-closed"
        );
    }

    /// The workspace partition is a real partition: same client id, different workspace, different
    /// durable identity — and it is stable across restarts (deterministic, not random).
    #[test]
    fn workspace_scoped_event_ids_are_disjoint_and_deterministic() {
        let client_event_id = Uuid::now_v7();
        let a = workspace_scoped_fr_event_id(TEST_WORKSPACE_ID, client_event_id);
        let b = workspace_scoped_fr_event_id(OTHER_TEST_WORKSPACE_ID, client_event_id);
        assert_ne!(a, b);
        assert_ne!(a, client_event_id);
        assert_eq!(
            a,
            workspace_scoped_fr_event_id(TEST_WORKSPACE_ID, client_event_id),
            "reconciliation and retries depend on determinism"
        );
        assert_eq!(a.get_version_num(), 8);
        assert_eq!(
            a.get_variant(),
            uuid::Variant::RFC4122,
            "the derived id must still be a well-formed UUID"
        );
    }
}

/// MT-109 proof-command guard.
///
/// The MT-109 native-editor proof suite is `cfg(feature = "duckdb-flight-recorder")`, so
/// `cargo test --lib native_editor` under DEFAULT features selects zero tests and reports
/// `0 passed; 0 failed` — a false green that a validator can mistake for a pass. This guard is
/// compiled ONLY when the feature is absent, matches the same `native_editor` filter, and is
/// `#[ignore]` so it never fails an unrelated default-feature run: an ignored result is visibly
/// NOT a pass, and running the MT-109 filter with `--include-ignored` (or reading the `ignored`
/// count) makes the missing feature explicit instead of silent.
///
/// Authoritative MT-109 proof command:
/// `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib
///  --features duckdb-flight-recorder,test-utils api::flight_recorder::tests::native_editor`
#[cfg(all(test, not(feature = "duckdb-flight-recorder")))]
mod native_editor_mt109_proof_guard {
    #[test]
    #[ignore = "MT-109 native_editor proof requires --features duckdb-flight-recorder; a zero-test run is NOT green"]
    fn native_editor_mt109_proof_requires_duckdb_flight_recorder_feature() {
        panic!(
            "MT-109 proof command ran without --features duckdb-flight-recorder: the \
             native-editor ingestion and authorization suite was not compiled, so any \
             '0 passed; 0 failed' result is a false green."
        );
    }
}

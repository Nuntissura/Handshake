use crate::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use crate::models::ErrorResponse;
use crate::AppState;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value;
use sqlx::Row;
use uuid::Uuid;

type ApiError = (StatusCode, Json<ErrorResponse>);
type ApiResult<T> = Result<T, ApiError>;

fn invalid_event() -> ApiError {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "HSK-400-INVALID-EVENT",
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
    Router::new()
        .route("/flight_recorder", get(list_events))
        .route("/events", get(list_events)) // backward-compatible path
        .route(
            "/flight_recorder/runtime_chat_event",
            post(record_runtime_chat_event),
        )
        .route(
            "/flight_recorder/native_editor_event",
            post(record_native_editor_event),
        )
        .with_state(state)
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

async fn record_runtime_chat_event(
    State(state): State<AppState>,
    Json(event): Json<RuntimeChatEventV0_1>,
) -> ApiResult<Json<Value>> {
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
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct NativeEditorFrEventV0_1 {
    pub schema_version: String,
    pub event_id: String,
    pub ts_utc: String,
    pub kind: NativeEditorFrEventKind,
    pub actor_id: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_kind: Option<NativeEditorActorKind>,
    pub pane_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface: Option<String>,
    pub workspace_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub work_packet_id: Option<String>,

    /// The bounded, per-kind structured payload. When present it MUST be a JSON object
    /// (no top-level free-text/array smuggling).
    #[serde(default)]
    pub payload: Value,
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
    let row = sqlx::query(
        r#"
        SELECT event_type, kernel_task_run_id, session_run_id, aggregate_type, aggregate_id,
               actor_kind, actor_id, correlation_id, payload
        FROM kernel_event_ledger
        WHERE event_id = $1
        "#,
    )
    .bind(receipt_id)
    .fetch_optional(&state.postgres_pool)
    .await
    .map_err(db_error)?
    .ok_or_else(invalid_event)?;

    let receipt_payload: Value = row.try_get("payload").map_err(db_error)?;
    let receipt_payload = receipt_payload.as_object().ok_or_else(invalid_event)?;
    let request_actor_kind = payload
        .get("actor_kind")
        .and_then(Value::as_str)
        .and_then(canonical_receipt_actor_kind)
        .ok_or_else(invalid_event)?;
    let correlation: Option<String> = row.try_get("correlation_id").map_err(db_error)?;
    let claimed_correlation = payload
        .get("correlation_id")
        .and_then(|value| value.as_str().map(str::to_owned));

    let authentic = row.try_get::<String, _>("event_type").map_err(db_error)?
        == KernelEventType::KnowledgeRichDocumentSaved.as_str()
        && row
            .try_get::<String, _>("aggregate_type")
            .map_err(db_error)?
            == "knowledge_rich_document"
        && row.try_get::<String, _>("aggregate_id").map_err(db_error)?
            == payload["document_id"].as_str().unwrap_or_default()
        && row.try_get::<String, _>("actor_kind").map_err(db_error)? == request_actor_kind
        && row.try_get::<String, _>("actor_id").map_err(db_error)? == event.actor_id
        && row
            .try_get::<String, _>("kernel_task_run_id")
            .map_err(db_error)?
            == payload["kernel_task_run_id"].as_str().unwrap_or_default()
        && row
            .try_get::<String, _>("session_run_id")
            .map_err(db_error)?
            == payload["session_run_id"].as_str().unwrap_or_default()
        && correlation == claimed_correlation
        && receipt_payload.get("event").and_then(Value::as_str) == Some("saved")
        && receipt_payload.get("workspace_id").and_then(Value::as_str)
            == Some(event.workspace_id.as_str())
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
    let event_uuid = Uuid::parse_str(event.event_id.trim()).map_err(|_| invalid_event())?;
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
        "workspace_id": event.workspace_id,
        "actor_id": event.actor_id,
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
    .with_actor_id(event.actor_id.clone())
    .with_wsids(vec![event.workspace_id.clone()]);
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
    json!({
        "receipt_kind": "native_editor_flight_recorder_recorded",
        "fr_event_id": envelope.event_id,
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
    let actor = match &expected_fr.actor {
        crate::flight_recorder::FlightRecorderActor::Human => {
            KernelActor::Operator(envelope.actor_id.clone())
        }
        _ => KernelActor::System(envelope.actor_id.clone()),
    };
    let kernel_task_run_id = envelope
        .work_packet_id
        .clone()
        .unwrap_or_else(|| envelope.workspace_id.clone());
    let session_run_id = envelope
        .session_id
        .clone()
        .unwrap_or_else(|| envelope.event_id.clone());

    NewKernelEvent::builder(
        kernel_task_run_id,
        session_run_id,
        KernelEventType::FlightRecorderMirrorPending,
        actor,
    )
    .aggregate("native_editor_event", envelope.event_id.clone())
    .idempotency_key(format!("native-editor-fr-pending:{}", envelope.event_id))
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
    .aggregate("native_editor_event", envelope.event_id.clone())
    .idempotency_key(format!("native-editor-fr-complete:{}", envelope.event_id))
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
/// store (readable via the existing GET route), idempotent on `event_id`, and durably
/// mirror it into the PostgreSQL kernel EventLedger.
async fn record_native_editor_event(
    State(state): State<AppState>,
    Json(mut event): Json<NativeEditorFrEventV0_1>,
) -> ApiResult<Json<Value>> {
    if event.schema_version.trim() != NATIVE_EDITOR_SCHEMA_VERSION {
        return Err(invalid_event());
    }
    let event_uuid = match Uuid::parse_str(event.event_id.trim()) {
        Ok(id) if id != Uuid::nil() => id,
        _ => return Err(invalid_event()),
    };
    if event.actor_id.trim().is_empty() {
        return Err(invalid_event());
    }
    if event.pane_id.trim().is_empty() || event.workspace_id.trim().is_empty() {
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
    if event.kind == NativeEditorFrEventKind::DocumentSaved {
        validate_document_save_receipt(&state, &event).await?;
    }
    if serde_json::to_vec(&event.payload)
        .map(|bytes| bytes.len() > 64 * 1024)
        .unwrap_or(true)
    {
        return Err(invalid_event());
    }

    // Canonicalize every common envelope identity before the first durable write. Equivalent lexical
    // UUID/timestamp spellings must converge on one PostgreSQL aggregate/idempotency state machine and
    // one Flight Recorder UUID, never create parallel mirrors for the same logical event.
    event.schema_version = NATIVE_EDITOR_SCHEMA_VERSION.to_owned();
    event.event_id = event_uuid.to_string();
    event.ts_utc = timestamp.to_rfc3339();
    event.actor_id = event.actor_id.trim().to_owned();
    event.pane_id = event.pane_id.trim().to_owned();
    event.workspace_id = event.workspace_id.trim().to_owned();
    event.surface = event
        .surface
        .take()
        .and_then(|surface| (!surface.trim().is_empty()).then(|| surface.trim().to_owned()));
    event.session_id = session_id;
    event.work_packet_id = event.work_packet_id.take().and_then(|work_packet| {
        (!work_packet.trim().is_empty()).then(|| work_packet.trim().to_owned())
    });

    let kind_str = event.kind.as_str();
    let fr_event = native_editor_fr_event_from_envelope(&event)?;
    // Durable EventLedger mirror is written FIRST and is idempotent. If the subsequent FR write fails,
    // a retry converges by reusing this receipt and completing the missing FR row. The former FR-first
    // ordering could strand an FR row forever without its required PostgreSQL mirror.
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
            event_id: Some(event_uuid),
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
                    event_id: Some(event_uuid),
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
        "event_id": event.event_id,
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

async fn list_events(
    State(state): State<AppState>,
    Query(filter): Query<EventFilter>,
) -> ApiResult<Json<Vec<FlightEvent>>> {
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
        wsid: filter.wsid.clone(),
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
    if let Some(wsid) = filter.wsid {
        events.retain(|e| e.wsids.contains(&wsid));
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
        FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
    };
    use crate::llm::{
        CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile, TokenUsage,
    };
    use crate::storage::{tests::optional_postgres_backend_with_pool_from_env, Database};
    use crate::workflows::{SessionRegistry, SessionSchedulerConfig};
    use crate::AppState;
    use std::sync::Arc;
    use uuid::Uuid;

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
        let Some(backend) = optional_postgres_backend_with_pool_from_env().await? else {
            return Ok(None);
        };

        let recorder = Arc::new(DuckDbFlightRecorder::new_in_memory(32)?);

        Ok(Some(AppState {
            storage: backend.database,
            postgres_pool: backend.postgres_pool,
            flight_recorder: recorder.clone(),
            diagnostics: recorder,
            llm_client: Arc::new(TestLlmClient::new()),
            capability_registry: Arc::new(CapabilityRegistry::new()),
            session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
        }))
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
            postgres_pool: state.postgres_pool.clone(),
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
                .with_model_session_id("sess-keep"),
            )
            .await?;

        state
            .flight_recorder
            .record_event(FlightRecorderEvent::new(
                FlightRecorderEventType::System,
                FlightRecorderActor::System,
                trace_id,
                json!({
                    "type": "system",
                    "event_id": "FR-EVT-SYS-000",
                }),
            ))
            .await?;

        let response = list_events(
            State(state),
            Query(EventFilter {
                model_session_id: Some("sess-keep".to_string()),
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
        NativeEditorFrEventV0_1 {
            schema_version: NATIVE_EDITOR_SCHEMA_VERSION.to_string(),
            event_id: event_id.to_string(),
            ts_utc: "2026-07-02T04:08:05Z".to_string(),
            kind: NativeEditorFrEventKind::CodeEdit,
            actor_id: "hsk:native_editor:pane-rich".to_string(),
            actor_kind: None,
            pane_id: "pane-rich".to_owned(),
            surface: None,
            workspace_id: "WS-NE-1".to_owned(),
            session_id: None,
            work_packet_id: None,
            payload: json!({"file_path":"src/main.rs","line_delta":1}),
        }
    }

    fn native_editor_pending_event(event: &NativeEditorFrEventV0_1) -> NewKernelEvent {
        let expected = native_editor_fr_event_from_envelope(event)
            .unwrap_or_else(|_| panic!("valid native-editor Flight Recorder fixture"));
        build_native_editor_pending(event, &expected).expect("valid native-editor pending fixture")
    }

    async fn authentic_document_saved_envelope(
        state: &AppState,
    ) -> Result<NativeEditorFrEventV0_1, Box<dyn std::error::Error>> {
        let document_id = format!("DOC-{}", Uuid::now_v7());
        let workspace_id = format!("WS-{}", Uuid::now_v7());
        let content_hash = "a".repeat(64);
        let task = format!("task-{}", Uuid::now_v7());
        let session = format!("session-{}", Uuid::now_v7());
        let correlation = format!("correlation-{}", Uuid::now_v7());
        let actor_id = "mt043-authentic-actor".to_owned();
        let receipt = state
            .storage
            .append_kernel_event(
                NewKernelEvent::builder(
                    task.clone(),
                    session.clone(),
                    KernelEventType::KnowledgeRichDocumentSaved,
                    KernelActor::Operator(actor_id.clone()),
                )
                .aggregate("knowledge_rich_document", document_id.clone())
                .source_component("knowledge_documents_api")
                .correlation_id(correlation.clone())
                .payload(json!({
                    "event":"saved",
                    "doc_version":2,
                    "workspace_id":workspace_id,
                    "content_hash":content_hash,
                }))
                .build()?,
            )
            .await?;
        Ok(NativeEditorFrEventV0_1 {
            schema_version: NATIVE_EDITOR_SCHEMA_VERSION.to_owned(),
            event_id: Uuid::now_v7().to_string(),
            ts_utc: Utc::now().to_rfc3339(),
            kind: NativeEditorFrEventKind::DocumentSaved,
            actor_id,
            actor_kind: Some(NativeEditorActorKind::Agent),
            pane_id: "pane-rich".to_owned(),
            surface: Some("rich-editor".to_owned()),
            workspace_id,
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
    /// idempotent on event_id, and durably mirrors into the PostgreSQL kernel EventLedger.
    #[tokio::test]
    async fn native_editor_event_round_trips_and_mirrors_to_ledger(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(state) = setup_state().await? else {
            return Ok(());
        };
        let event_id = Uuid::now_v7().to_string();
        let uuid = Uuid::parse_str(&event_id)?;
        let body = json!({
            "schema_version": NATIVE_EDITOR_SCHEMA_VERSION,
            "event_id": event_id,
            "ts_utc": "2026-07-02T04:08:05Z",
            "kind": "stage_embed_back",
            "actor_id": "hsk:native_editor:pane-rich",
            "actor_kind": "human",
            "pane_id": "pane-rich",
            "surface": "stage",
            "workspace_id": "WS-NE-1",
            "payload": {
                "artifact_id": "ART-1",
                "target_pane_id": "pane-rich",
                "sha256": "a".repeat(64),
                "manifest_ref": "manifest-ART-1",
                "causal_action_id": "stage-route-action-1"
            }
        });
        let event: NativeEditorFrEventV0_1 = serde_json::from_value(body.clone())?;

        let Json(ack) = record_native_editor_event(State(state.clone()), Json(event))
            .await
            .map_err(|(code, _body)| format!("ingest failed: {code}"))?;
        assert_eq!(ack["ok"], true);
        assert_eq!(ack["kind"], "stage_embed_back");

        // Readable back via the existing GET route, keyed on event_id.
        let Json(events) = list_events(
            State(state.clone()),
            Query(EventFilter {
                event_id: Some(uuid),
                ..Default::default()
            }),
        )
        .await
        .map_err(|_| "list failed")?;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "system");
        assert_eq!(events[0].actor_id, "hsk:native_editor:pane-rich");
        assert_eq!(events[0].payload["kind"], "stage_embed_back");
        assert_eq!(events[0].payload["action"], "stage_embed_back");
        assert_eq!(events[0].payload["schema"], NATIVE_EDITOR_SCHEMA_VERSION);
        assert_eq!(events[0].payload["event_family"], "native_editor");
        assert_eq!(
            events[0].payload["native_payload"]["causal_action_id"],
            "stage-route-action-1"
        );
        assert!(events[0].wsids.contains(&"WS-NE-1".to_string()));

        // Durable EventLedger mirror in managed PostgreSQL.
        let ledger_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM kernel_event_ledger WHERE idempotency_key = $1 AND event_type = $2",
        )
        .bind(format!("native-editor-fr-complete:{event_id}"))
        .bind(KernelEventType::FlightRecorderMirrorRecorded.as_str())
        .fetch_one(&state.postgres_pool)
        .await?;
        assert_eq!(
            ledger_count, 1,
            "one durable native-editor FR ledger receipt"
        );

        // Idempotent re-POST of the same event_id.
        let event_again: NativeEditorFrEventV0_1 = serde_json::from_value(body)?;
        let Json(ack2) = record_native_editor_event(State(state.clone()), Json(event_again))
            .await
            .map_err(|(code, _body)| format!("re-ingest failed: {code}"))?;
        assert_eq!(ack2["idempotent"], true);

        let Json(events_after) = list_events(
            State(state.clone()),
            Query(EventFilter {
                event_id: Some(uuid),
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

        let ledger_after: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM kernel_event_ledger WHERE idempotency_key = $1",
        )
        .bind(format!("native-editor-fr-complete:{event_id}"))
        .fetch_one(&state.postgres_pool)
        .await?;
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
            record_native_editor_event(State(state.clone()), Json(original.clone()))
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
                    record_native_editor_event(State(state.clone()), Json(changed)).await,
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
            record_native_editor_event(State(state.clone()), Json(event.clone())),
            record_native_editor_event(State(state.clone()), Json(event)),
        );
        assert!(left.is_ok(), "first concurrent ingest failed");
        assert!(right.is_ok(), "second concurrent ingest failed");

        let recorder_rows = state
            .flight_recorder
            .list_events(crate::flight_recorder::EventFilter {
                event_id: Some(Uuid::parse_str(&event_id)?),
                ..Default::default()
            })
            .await?;
        assert_eq!(recorder_rows.len(), 1, "exactly one Flight Recorder row");

        let ledger_rows: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM kernel_event_ledger WHERE aggregate_type = 'native_editor_event' AND aggregate_id = $1 AND event_type IN ($2, $3)",
        )
        .bind(&event_id)
        .bind(KernelEventType::FlightRecorderMirrorPending.as_str())
        .bind(KernelEventType::FlightRecorderMirrorRecorded.as_str())
        .fetch_one(&state.postgres_pool)
        .await?;
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

        record_native_editor_event(State(state.clone()), Json(first))
            .await
            .map_err(|(status, _)| format!("canonical initial ingest failed: {status}"))?;
        let Json(ack) = record_native_editor_event(State(state.clone()), Json(retry))
            .await
            .map_err(|(status, _)| format!("canonical retry failed: {status}"))?;
        assert_eq!(ack["idempotent"], true);

        let aggregate_id = uuid.to_string();
        let ledger_rows: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM kernel_event_ledger WHERE aggregate_type = 'native_editor_event' AND aggregate_id = $1 AND event_type IN ($2, $3)",
        )
        .bind(&aggregate_id)
        .bind(KernelEventType::FlightRecorderMirrorPending.as_str())
        .bind(KernelEventType::FlightRecorderMirrorRecorded.as_str())
        .fetch_one(&state.postgres_pool)
        .await?;
        assert_eq!(
            ledger_rows, 2,
            "one canonical pending/completion state machine"
        );
        let recorder_rows = state
            .flight_recorder
            .list_events(crate::flight_recorder::EventFilter {
                event_id: Some(uuid),
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
        record_native_editor_event(State(state.clone()), Json(event.clone()))
            .await
            .map_err(|(status, _)| format!("unicode initial ingest failed: {status}"))?;
        let Json(ack) = record_native_editor_event(State(state), Json(event))
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

        let Json(rows) = list_events(
            State(state),
            Query(EventFilter {
                surface: Some("pane-rich".to_owned()),
                event_type: Some("system".to_owned()),
                ..Default::default()
            }),
        )
        .await
        .map_err(|(status, _)| format!("surface-filter list failed: {status}"))?;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].event_id, native_id.to_string());
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
                event_id: Some(Uuid::parse_str(&event_id)?),
                ..Default::default()
            })
            .await?;
        assert!(
            before.is_empty(),
            "fixture represents the post-ledger/pre-FR crash window"
        );

        // This is the same startup pass installed by `routes`: it discovers work from durable
        // PostgreSQL state rather than relying on an in-memory queue from the failed process.
        reconcile_native_editor_pending(&state)
            .await
            .map_err(std::io::Error::other)?;

        let after = state
            .flight_recorder
            .list_events(crate::flight_recorder::EventFilter {
                event_id: Some(Uuid::parse_str(&event_id)?),
                ..Default::default()
            })
            .await?;
        assert_eq!(after.len(), 1, "startup reconciliation restored the FR row");
        let completion_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM kernel_event_ledger WHERE idempotency_key = $1 AND event_type = $2",
        )
        .bind(format!("native-editor-fr-complete:{event_id}"))
        .bind(KernelEventType::FlightRecorderMirrorRecorded.as_str())
        .fetch_one(&state.postgres_pool)
        .await?;
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
                    event_id: Some(Uuid::parse_str(&event_id)?),
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
            event.workspace_id.clone(),
            event.event_id.clone(),
            KernelEventType::FlightRecorderMirrorRecorded,
            KernelActor::Operator(event.actor_id.clone()),
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
                    event_id: Some(Uuid::parse_str(&event_id)?),
                    ..Default::default()
                })
                .await?
                .len(),
            1,
            "spurious completion must not suppress the pending mirror"
        );
        let completion_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM kernel_event_ledger WHERE idempotency_key = $1 AND event_type = $2",
        )
        .bind(format!("native-editor-fr-complete:{event_id}"))
        .bind(KernelEventType::FlightRecorderMirrorRecorded.as_str())
        .fetch_one(&state.postgres_pool)
        .await?;
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
        state.storage.append_kernel_event(completion).await?;

        sqlx::query("UPDATE kernel_event_ledger SET payload_hash = $1 WHERE idempotency_key = $2")
            .bind("0".repeat(64))
            .bind(format!("native-editor-fr-complete:{}", event.event_id))
            .execute(&state.postgres_pool)
            .await?;

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
        // Restore the test-injected fault so the shared managed PostgreSQL resource
        // does not retain a poison row after this adversarial probe.
        sqlx::query("UPDATE kernel_event_ledger SET payload_hash = $1 WHERE idempotency_key = $2")
            .bind(canonical_completion_hash)
            .bind(format!("native-editor-fr-complete:{}", event.event_id))
            .execute(&state.postgres_pool)
            .await?;
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
                event_id: Some(Uuid::parse_str(&event.event_id)?),
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
            let uuid = Uuid::parse_str(&event.event_id)?;
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
            let completion_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM kernel_event_ledger WHERE idempotency_key = $1 AND event_type = $2",
            )
            .bind(format!("native-editor-fr-complete:{}", event.event_id))
            .bind(KernelEventType::FlightRecorderMirrorRecorded.as_str())
            .fetch_one(&state_after.postgres_pool)
            .await?;
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
                event_id: Some(Uuid::parse_str(&valid.event_id)?),
                ..Default::default()
            })
            .await?
            .len();

        // Test-only cleanup prevents deliberately malformed append-only fixtures from burdening
        // every later reconciler pass in the shared managed test database.
        sqlx::query(
            "DELETE FROM kernel_event_ledger WHERE aggregate_type = 'native_editor_event' AND (aggregate_id LIKE $1 OR aggregate_id = $2)",
        )
        .bind(format!("{prefix}%"))
        .bind(&valid.event_id)
        .execute(&state.postgres_pool)
        .await?;
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
        let before_fr = state
            .flight_recorder
            .list_events(crate::flight_recorder::EventFilter::default())
            .await?
            .len();
        let before_ledger: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM kernel_event_ledger WHERE aggregate_type = 'native_editor_event'",
        )
        .fetch_one(&state.postgres_pool)
        .await?;
        let (base, http, server) = serve_test_router(routes(state.clone())).await;
        let endpoint = format!("{base}/flight_recorder/native_editor_event");

        let mut unknown_kind =
            serde_json::to_value(native_editor_envelope(&Uuid::now_v7().to_string()))?;
        unknown_kind["kind"] = json!("smuggled_editor_kind");
        let response = http.post(&endpoint).json(&unknown_kind).send().await?;
        assert!(
            response.status().is_client_error(),
            "unknown kind must be rejected by the mounted route, got {}",
            response.status()
        );

        let mut unknown_field =
            serde_json::to_value(native_editor_envelope(&Uuid::now_v7().to_string()))?;
        unknown_field["smuggled"] = json!("free text");
        let response = http.post(&endpoint).json(&unknown_field).send().await?;
        assert!(
            response.status().is_client_error(),
            "unknown envelope field must be rejected by the mounted route, got {}",
            response.status()
        );

        let after_fr = state
            .flight_recorder
            .list_events(crate::flight_recorder::EventFilter::default())
            .await?
            .len();
        let after_ledger: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM kernel_event_ledger WHERE aggregate_type = 'native_editor_event'",
        )
        .fetch_one(&state.postgres_pool)
        .await?;
        assert_eq!(after_fr, before_fr, "rejected bodies must emit no FR row");
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
            let event_id = Uuid::parse_str(&event.event_id)?;
            let aggregate_id = event_id.to_string();
            let workspace_id = event.workspace_id.clone();

            let Json(ack) = record_native_editor_event(State(state.clone()), Json(event))
                .await
                .map_err(|(status, _)| {
                    format!(
                        "handler rejected documented kind {}: {status}",
                        kind.as_str()
                    )
                })?;
            assert_eq!(ack["ok"], true, "{} acknowledgement", kind.as_str());
            assert_eq!(ack["kind"], kind.as_str());

            let Json(events) = list_events(
                State(state.clone()),
                Query(EventFilter {
                    event_id: Some(event_id),
                    ..Default::default()
                }),
            )
            .await
            .map_err(|_| format!("GET failed for documented kind {}", kind.as_str()))?;
            assert_eq!(events.len(), 1, "{} must persist one FR row", kind.as_str());
            assert_eq!(events[0].payload["kind"], kind.as_str());
            assert_eq!(events[0].payload["workspace_id"], workspace_id);

            let completion_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM kernel_event_ledger WHERE aggregate_type = 'native_editor_event' AND aggregate_id = $1 AND event_type = $2",
            )
            .bind(&aggregate_id)
            .bind(KernelEventType::FlightRecorderMirrorRecorded.as_str())
            .fetch_one(&state.postgres_pool)
            .await?;
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
                            record_native_editor_event(State(state.clone()), Json(event)).await,
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
                    record_native_editor_event(State(state.clone()), Json(unknown)).await,
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
            let Json(ack) = record_native_editor_event(State(state.clone()), Json(accepted))
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
                    record_native_editor_event(State(state.clone()), Json(rejected)).await,
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
                record_native_editor_event(State(state.clone()), Json(unknown)).await,
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
            record_native_editor_event(State(state.clone()), Json(wrong_schema)).await,
            Err((StatusCode::BAD_REQUEST, _))
        ));

        let mut bad_id = native_editor_envelope("not-a-uuid");
        bad_id.payload = Value::Null;
        assert!(matches!(
            record_native_editor_event(State(state.clone()), Json(bad_id)).await,
            Err((StatusCode::BAD_REQUEST, _))
        ));

        let mut free_text_payload = native_editor_envelope(&Uuid::now_v7().to_string());
        free_text_payload.payload = json!("free text string");
        assert!(matches!(
            record_native_editor_event(State(state.clone()), Json(free_text_payload)).await,
            Err((StatusCode::BAD_REQUEST, _))
        ));

        let mut invalid_timestamp = native_editor_envelope(&Uuid::now_v7().to_string());
        invalid_timestamp.ts_utc = "not-rfc3339".to_owned();
        assert!(matches!(
            record_native_editor_event(State(state.clone()), Json(invalid_timestamp)).await,
            Err((StatusCode::BAD_REQUEST, _))
        ));

        let mut incomplete_document = native_editor_envelope(&Uuid::now_v7().to_string());
        incomplete_document.kind = NativeEditorFrEventKind::DocumentSaved;
        incomplete_document.payload = json!({"document_id": "DOC-MISSING-HASH"});
        assert!(matches!(
            record_native_editor_event(State(state.clone()), Json(incomplete_document)).await,
            Err((StatusCode::BAD_REQUEST, _))
        ));

        let mut missing_pane = native_editor_envelope(&Uuid::now_v7().to_string());
        missing_pane.pane_id.clear();
        assert!(matches!(
            record_native_editor_event(State(state.clone()), Json(missing_pane)).await,
            Err((StatusCode::BAD_REQUEST, _))
        ));

        let mut missing_workspace = native_editor_envelope(&Uuid::now_v7().to_string());
        missing_workspace.workspace_id.clear();
        assert!(matches!(
            record_native_editor_event(State(state.clone()), Json(missing_workspace)).await,
            Err((StatusCode::BAD_REQUEST, _))
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
            record_native_editor_event(State(state.clone()), Json(missing)).await,
            Err((StatusCode::BAD_REQUEST, _))
        ));

        let mut fabricated = authentic.clone();
        fabricated.payload["save_receipt_event_id"] = json!(format!("KE-{}", Uuid::now_v7()));
        assert!(matches!(
            record_native_editor_event(State(state.clone()), Json(fabricated)).await,
            Err((StatusCode::BAD_REQUEST, _))
        ));

        let mut corrupt = authentic.clone();
        corrupt.payload["content_hash"] = json!("b".repeat(64));
        assert!(matches!(
            record_native_editor_event(State(state.clone()), Json(corrupt)).await,
            Err((StatusCode::BAD_REQUEST, _))
        ));

        let Json(ack) = record_native_editor_event(State(state), Json(authentic))
            .await
            .map_err(|(status, _)| format!("authentic receipt rejected: {status}"))?;
        assert_eq!(ack["ok"], true);
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
        let fr_only = native_editor_fr_event_from_envelope(&event)
            .map_err(|_| "failed to build exact FR-only fixture")?;
        state.flight_recorder.record_event(fr_only).await?;

        let Json(ack) = record_native_editor_event(State(state.clone()), Json(event))
            .await
            .map_err(|(code, _)| format!("repair replay failed: {code}"))?;
        assert_eq!(ack["idempotent"], true);
        let ledger_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM kernel_event_ledger WHERE idempotency_key = $1",
        )
        .bind(format!("native-editor-fr-complete:{event_id}"))
        .fetch_one(&state.postgres_pool)
        .await?;
        assert_eq!(
            ledger_count, 1,
            "replay repaired the missing PostgreSQL mirror"
        );
        Ok(())
    }
}

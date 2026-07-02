use crate::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use crate::models::ErrorResponse;
use crate::AppState;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value;
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
    pub surface: Option<String>,
    pub event_type: Option<String>,
    pub wsid: Option<String>,
}

pub fn routes(state: AppState) -> Router {
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
    MemoryWriteProposed,
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
            NativeEditorFrEventKind::MemoryWriteProposed => "memory_write_proposed",
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
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NativeEditorFrEventV0_1 {
    pub schema_version: String,
    pub event_id: String,
    pub ts_utc: String,
    pub kind: NativeEditorFrEventKind,
    pub actor_id: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_kind: Option<NativeEditorActorKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pane_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
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

fn map_recorder_err(err: crate::flight_recorder::RecorderError) -> ApiError {
    match err {
        crate::flight_recorder::RecorderError::InvalidEvent(_) => invalid_event(),
        other => db_error(other),
    }
}

/// AC-109-1: accept a versioned native-editor event envelope, land it in the FR authority
/// store (readable via the existing GET route), idempotent on `event_id`, and durably
/// mirror it into the PostgreSQL kernel EventLedger.
async fn record_native_editor_event(
    State(state): State<AppState>,
    Json(event): Json<NativeEditorFrEventV0_1>,
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
    // Free text is only allowed inside bounded named fields: the payload, when present,
    // must be a JSON object (not a bare string/array/number).
    if !event.payload.is_null() && !event.payload.is_object() {
        return Err(invalid_event());
    }

    // A valid non-nil UUID session_id groups the event under a shared trace; otherwise the
    // event's own id is the trace id (FR events require a non-nil trace_id).
    let trace_id = event
        .session_id
        .as_deref()
        .and_then(|s| Uuid::parse_str(s.trim()).ok())
        .filter(|id| *id != Uuid::nil())
        .unwrap_or(event_uuid);

    // Idempotency on event_id: if the FR store already holds it, this is a no-op (the FR
    // `events` table keys on event_id, so a re-insert would otherwise be a PK conflict).
    let existing = state
        .flight_recorder
        .list_events(crate::flight_recorder::EventFilter {
            event_id: Some(event_uuid),
            ..Default::default()
        })
        .await
        .map_err(map_recorder_err)?;
    if !existing.is_empty() {
        return Ok(Json(json!({
            "ok": true,
            "event_id": event.event_id,
            "kind": event.kind.as_str(),
            "idempotent": true,
        })));
    }

    let kind_str = event.kind.as_str();
    let surface = event
        .surface
        .clone()
        .or_else(|| event.pane_id.clone())
        .unwrap_or_else(|| "native_editor".to_string());

    // The FR payload carries the native-editor family + the two fields the EditorEdit
    // schema validator requires (`editor_surface` + `ops`), so the event is queryable by
    // surface via the existing GET route.
    let fr_payload = json!({
        "event_family": "native_editor",
        "schema_version": NATIVE_EDITOR_SCHEMA_VERSION,
        "kind": kind_str,
        "editor_surface": surface,
        "ops": [ { "kind": kind_str, "payload": event.payload } ],
        "pane_id": event.pane_id,
        "workspace_id": event.workspace_id,
        "actor_id": event.actor_id,
        "ts_utc": event.ts_utc,
        "native_payload": event.payload,
    });

    let actor = match event.actor_kind.unwrap_or(NativeEditorActorKind::Human) {
        NativeEditorActorKind::Human => crate::flight_recorder::FlightRecorderActor::Human,
        NativeEditorActorKind::Agent => crate::flight_recorder::FlightRecorderActor::Agent,
        NativeEditorActorKind::System => crate::flight_recorder::FlightRecorderActor::System,
    };

    let mut fr_event = crate::flight_recorder::FlightRecorderEvent::new(
        crate::flight_recorder::FlightRecorderEventType::EditorEdit,
        actor.clone(),
        trace_id,
        fr_payload,
    )
    .with_actor_id(event.actor_id.clone());
    // Honor the client's event_id so idempotency + cross-store correlation hold.
    fr_event.event_id = event_uuid;
    if let Some(ws) = event.workspace_id.clone() {
        fr_event = fr_event.with_wsids(vec![ws]);
    }
    if let Some(session_id) = event.session_id.clone() {
        fr_event = fr_event.with_session_span(session_id);
    }

    state
        .flight_recorder
        .record_event(fr_event)
        .await
        .map_err(map_recorder_err)?;

    // Durable EventLedger mirror (PostgreSQL authority path), idempotent on event_id via
    // the ledger's unique idempotency_key.
    let kernel_task_run_id = event
        .work_packet_id
        .clone()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| event.workspace_id.clone().filter(|v| !v.trim().is_empty()))
        .unwrap_or_else(|| "native-editor-fr".to_string());
    let session_run_id = event
        .session_id
        .clone()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| event.event_id.clone());
    let kernel_actor = match actor {
        crate::flight_recorder::FlightRecorderActor::Human => {
            KernelActor::Operator(event.actor_id.clone())
        }
        _ => KernelActor::System(event.actor_id.clone()),
    };

    let receipt = NewKernelEvent::builder(
        kernel_task_run_id,
        session_run_id,
        KernelEventType::FlightRecorderMirrorRecorded,
        kernel_actor,
    )
    .aggregate("native_editor_event", kind_str)
    .idempotency_key(format!("native-editor-fr:{}", event.event_id))
    .source_component("native_editor_fr_ingestion")
    .correlation_id(trace_id.to_string())
    .payload(json!({
        "receipt_kind": "native_editor_flight_recorder",
        "fr_event_id": event.event_id,
        "fr_event_type": "editor_edit",
        "event_family": "native_editor",
        "kind": kind_str,
        "actor_id": event.actor_id,
        "workspace_id": event.workspace_id,
        "native_payload": event.payload,
    }))
    .build()
    .map_err(db_error)?;

    state
        .storage
        .append_kernel_event(receipt)
        .await
        .map_err(db_error)?;

    Ok(Json(json!({
        "ok": true,
        "event_id": event.event_id,
        "kind": kind_str,
    })))
}

async fn list_events(
    State(state): State<AppState>,
    Query(filter): Query<EventFilter>,
) -> ApiResult<Json<Vec<FlightEvent>>> {
    let internal_filter = crate::flight_recorder::EventFilter {
        event_id: filter.event_id,
        job_id: filter.job_id,
        trace_id: filter.trace_id,
        model_session_id: filter.model_session_id,
        from: filter.from,
        to: filter.to,
    };

    let mut events = state
        .flight_recorder
        .list_events(internal_filter)
        .await
        .map_err(db_error)?;

    if let Some(actor) = filter.actor {
        events.retain(|e| e.actor.to_string() == actor);
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
            kind: NativeEditorFrEventKind::DocumentSaved,
            actor_id: "hsk:native_editor:pane-rich".to_string(),
            actor_kind: None,
            pane_id: None,
            surface: None,
            workspace_id: None,
            session_id: None,
            work_packet_id: None,
            payload: Value::Null,
        }
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
            "payload": { "artifact_id": "ART-1", "content_hash": "a".repeat(64) }
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
        assert_eq!(events[0].event_type, "editor_edit");
        assert_eq!(events[0].actor_id, "hsk:native_editor:pane-rich");
        assert_eq!(events[0].payload["kind"], "stage_embed_back");
        assert_eq!(events[0].payload["event_family"], "native_editor");
        assert!(events[0].wsids.contains(&"WS-NE-1".to_string()));

        // Durable EventLedger mirror in managed PostgreSQL.
        let ledger_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM kernel_event_ledger WHERE idempotency_key = $1 AND event_type = $2",
        )
        .bind(format!("native-editor-fr:{event_id}"))
        .bind(KernelEventType::FlightRecorderMirrorRecorded.as_str())
        .fetch_one(&state.postgres_pool)
        .await?;
        assert_eq!(ledger_count, 1, "one durable native-editor FR ledger receipt");

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
        assert_eq!(events_after.len(), 1, "idempotent: still exactly one FR row");

        let ledger_after: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM kernel_event_ledger WHERE idempotency_key = $1",
        )
        .bind(format!("native-editor-fr:{event_id}"))
        .fetch_one(&state.postgres_pool)
        .await?;
        assert_eq!(ledger_after, 1, "idempotent: still exactly one ledger receipt");
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
            "actor_id": "a"
        });
        assert!(serde_json::from_value::<NativeEditorFrEventV0_1>(bad_kind).is_err());

        let bad_field = json!({
            "schema_version": NATIVE_EDITOR_SCHEMA_VERSION,
            "event_id": Uuid::now_v7().to_string(),
            "ts_utc": "2026-07-02T04:08:05Z",
            "kind": "document_saved",
            "actor_id": "a",
            "smuggled_free_text": "arbitrary"
        });
        assert!(serde_json::from_value::<NativeEditorFrEventV0_1>(bad_field).is_err());
    }

    /// AC-109-1: every documented kind (8 native actions + 5 interop kinds) decodes.
    #[test]
    fn native_editor_envelope_accepts_all_documented_kinds() {
        let kinds = [
            "document_saved",
            "code_edit",
            "embed_created",
            "canvas_node_placed",
            "cross_ref_inserted",
            "undo_fired",
            "route_to_stage",
            "memory_write_proposed",
            "stage_embed_back",
            "calendar_event_bound",
            "activity_span_correlated",
            "locus_ref_resolved",
            "locus_reverse_lookup",
        ];
        for kind in kinds {
            let body = json!({
                "schema_version": NATIVE_EDITOR_SCHEMA_VERSION,
                "event_id": Uuid::now_v7().to_string(),
                "ts_utc": "2026-07-02T04:08:05Z",
                "kind": kind,
                "actor_id": "a"
            });
            let parsed: NativeEditorFrEventV0_1 = serde_json::from_value(body)
                .unwrap_or_else(|e| panic!("kind {kind} should parse: {e}"));
            assert_eq!(parsed.kind.as_str(), kind);
        }
    }

    /// AC-109-1: the handler fails closed on a wrong schema version, a non-UUID event_id,
    /// or a non-object payload (no top-level free-text smuggling).
    #[tokio::test]
    async fn native_editor_event_handler_fails_closed(
    ) -> Result<(), Box<dyn std::error::Error>> {
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
        Ok(())
    }
}

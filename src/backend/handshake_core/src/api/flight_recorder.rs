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

async fn list_events(
    State(state): State<AppState>,
    Query(filter): Query<EventFilter>,
) -> ApiResult<Json<Vec<FlightEvent>>> {
    let internal_filter = crate::flight_recorder::EventFilter {
        event_id: filter.event_id,
        job_id: filter.job_id,
        trace_id: filter.trace_id,
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

use crate::AppState;
use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

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
        .with_state(state)
}

async fn list_events(
    State(state): State<AppState>,
    Query(filter): Query<EventFilter>,
) -> Result<Json<Vec<FlightEvent>>, String> {
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
        .map_err(|e| e.to_string())?;

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

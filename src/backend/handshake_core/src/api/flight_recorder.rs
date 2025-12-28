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
    pub wsids: Vec<String>,
    pub payload: Value,
}

#[derive(Deserialize, Debug, Default)]
pub struct EventFilter {
    pub job_id: Option<String>,
    pub trace_id: Option<Uuid>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub actor: Option<String>,
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
            wsids: e.wsids,
            payload: e.payload,
        })
        .collect();

    Ok(Json(api_events))
}

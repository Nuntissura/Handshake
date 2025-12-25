use crate::AppState;
use axum::{extract::State, routing::get, Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug)]
pub struct FlightEvent {
    pub timestamp: String,
    pub event_type: String,
    pub job_id: Option<String>,
    pub workflow_id: Option<String>,
    pub payload: Value,
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/flight_recorder", get(list_events))
        .route("/events", get(list_events)) // backward-compatible path
        .with_state(state)
}

async fn list_events(State(state): State<AppState>) -> Result<Json<Vec<FlightEvent>>, String> {
    let conn = state.fr_pool.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare("SELECT timestamp, event_type, job_id, workflow_id, payload FROM events ORDER BY timestamp DESC LIMIT 100")
        .map_err(|e| e.to_string())?;

    let event_iter = stmt
        .query_map([], |row| {
            let payload_str: String = row.get(4)?;
            let payload: Value = match serde_json::from_str(&payload_str) {
                Ok(val) => val,
                Err(_) => Value::Null,
            };

            Ok(FlightEvent {
                timestamp: row.get(0)?,
                event_type: row.get(1)?,
                job_id: row.get(2)?,
                workflow_id: row.get(3)?,
                payload,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut events = Vec::new();
    for event in event_iter {
        events.push(event.map_err(|e| e.to_string())?);
    }

    Ok(Json(events))
}

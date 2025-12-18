use crate::AppState;
use serde_json::Value as JsonValue;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FlightRecorderError {
    #[error("Database error: {0}")]
    DuckDb(#[from] duckdb::Error),
    #[error("Failed to lock database connection")]
    LockError,
}

pub fn log_event(
    state: &AppState,
    event_type: &str,
    job_id: Option<&str>,
    workflow_id: Option<&str>,
    payload: JsonValue,
) -> Result<(), FlightRecorderError> {
    // We lock the mutex to get exclusive access to the connection, as DuckDB
    // connections are not meant to be used across threads simultaneously.
    let conn = state
        .fr_pool
        .lock()
        .map_err(|_| FlightRecorderError::LockError)?;

    let payload_str = payload.to_string();

    conn.execute(
        "INSERT INTO events (event_type, job_id, workflow_id, payload) VALUES (?, ?, ?, ?)",
        duckdb::params![event_type, job_id, workflow_id, payload_str],
    )?;

    Ok(())
}

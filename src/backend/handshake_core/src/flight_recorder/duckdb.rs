use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use duckdb::Connection as DuckDbConnection;
use serde_json::Value;

use super::{
    FlightRecorder, FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
    RecorderError,
};

pub struct DuckDbFlightRecorder {
    conn: Arc<Mutex<DuckDbConnection>>,
    retention_days: u32,
}

impl DuckDbFlightRecorder {
    pub fn new(
        conn: Arc<Mutex<DuckDbConnection>>,
        retention_days: u32,
    ) -> Result<Self, RecorderError> {
        let recorder = Self {
            conn,
            retention_days,
        };
        recorder.init_schema()?;
        Ok(recorder)
    }

    pub fn new_on_path(path: &std::path::Path, retention_days: u32) -> Result<Self, RecorderError> {
        let conn =
            DuckDbConnection::open(path).map_err(|e| RecorderError::SinkError(e.to_string()))?;
        Self::new(Arc::new(Mutex::new(conn)), retention_days)
    }

    pub fn new_in_memory(retention_days: u32) -> Result<Self, RecorderError> {
        let conn = DuckDbConnection::open_in_memory()
            .map_err(|e| RecorderError::SinkError(e.to_string()))?;
        Self::new(Arc::new(Mutex::new(conn)), retention_days)
    }

    pub fn connection(&self) -> Arc<Mutex<DuckDbConnection>> {
        self.conn.clone()
    }

    fn init_schema(&self) -> Result<(), RecorderError> {
        let conn = self.conn.lock().map_err(|_| RecorderError::LockError)?;

        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS events (
                event_id UUID PRIMARY KEY,
                trace_id UUID NOT NULL,
                timestamp TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
                actor TEXT NOT NULL,
                actor_id TEXT NOT NULL,
                event_type TEXT NOT NULL,
                job_id TEXT,
                workflow_id TEXT,
                model_id TEXT,
                activity_span_id TEXT,
                session_span_id TEXT,
                capability_id TEXT,
                policy_decision_id TEXT,
                wsids JSON,
                payload JSON NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_events_trace_id ON events(trace_id);
            CREATE INDEX IF NOT EXISTS idx_events_job_id ON events(job_id);
            CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp);
        "#,
        )
        .map_err(|e| RecorderError::SinkError(e.to_string()))?;

        conn.execute_batch(
            r#"
            ALTER TABLE events ADD COLUMN IF NOT EXISTS actor_id TEXT;
            ALTER TABLE events ADD COLUMN IF NOT EXISTS actor TEXT;
            ALTER TABLE events ADD COLUMN IF NOT EXISTS trace_id UUID;
            ALTER TABLE events ADD COLUMN IF NOT EXISTS event_id UUID;
            ALTER TABLE events ADD COLUMN IF NOT EXISTS workflow_id TEXT;
            ALTER TABLE events ADD COLUMN IF NOT EXISTS model_id TEXT;
            ALTER TABLE events ADD COLUMN IF NOT EXISTS activity_span_id TEXT;
            ALTER TABLE events ADD COLUMN IF NOT EXISTS session_span_id TEXT;
            ALTER TABLE events ADD COLUMN IF NOT EXISTS capability_id TEXT;
            ALTER TABLE events ADD COLUMN IF NOT EXISTS policy_decision_id TEXT;
            ALTER TABLE events ADD COLUMN IF NOT EXISTS wsids JSON;
            ALTER TABLE events ADD COLUMN IF NOT EXISTS payload JSON;
            ALTER TABLE events ADD COLUMN IF NOT EXISTS timestamp TIMESTAMPTZ;
            ALTER TABLE events ADD COLUMN IF NOT EXISTS event_type TEXT;
            ALTER TABLE events ADD COLUMN IF NOT EXISTS job_id TEXT;
        "#,
        )
        .map_err(|e| RecorderError::SinkError(e.to_string()))?;

        Ok(())
    }

    fn insert_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
        let payload_str = serde_json::to_string(&event.payload)
            .map_err(|e| RecorderError::InvalidEvent(e.to_string()))?;
        let wsids_json = if event.wsids.is_empty() {
            Value::Null
        } else {
            Value::Array(
                event
                    .wsids
                    .iter()
                    .map(|s| Value::String(s.clone()))
                    .collect(),
            )
        };
        let wsids_str = serde_json::to_string(&wsids_json)
            .map_err(|e| RecorderError::InvalidEvent(e.to_string()))?;

        let conn = self.conn.lock().map_err(|_| RecorderError::LockError)?;

        let event_id_str = event.event_id.to_string();
        let trace_id_str = event.trace_id.to_string();
        let timestamp_str = event.timestamp.to_rfc3339();

        conn.execute(
            r#"
            INSERT INTO events (
                event_id,
                trace_id,
                timestamp,
                actor,
                actor_id,
                event_type,
                job_id,
                workflow_id,
                model_id,
                activity_span_id,
                session_span_id,
                capability_id,
                policy_decision_id,
                wsids,
                payload
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
            duckdb::params![
                event_id_str,
                trace_id_str,
                timestamp_str,
                event.actor.to_string(),
                event.actor_id,
                event.event_type.to_string(),
                event.job_id.as_deref(),
                event.workflow_id.as_deref(),
                event.model_id.as_deref(),
                event.activity_span_id.as_deref(),
                event.session_span_id.as_deref(),
                event.capability_id.as_deref(),
                event.policy_decision_id.as_deref(),
                wsids_str,
                payload_str
            ],
        )
        .map_err(|e| RecorderError::SinkError(e.to_string()))?;

        Ok(())
    }

    fn purge_retention(&self) -> Result<u64, RecorderError> {
        let conn = self.conn.lock().map_err(|_| RecorderError::LockError)?;

        let mut stmt = conn
            .prepare("DELETE FROM events WHERE timestamp < (CURRENT_TIMESTAMP - INTERVAL ? DAY)")
            .map_err(|e| RecorderError::SinkError(e.to_string()))?;
        let affected = stmt
            .execute([self.retention_days as i32])
            .map_err(|e| RecorderError::SinkError(e.to_string()))?;
        Ok(affected as u64)
    }

    fn query_events(
        &self,
        filter: super::EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        let conn = self.conn.lock().map_err(|_| RecorderError::LockError)?;

        let mut conditions = Vec::new();
        let mut params: Vec<Box<dyn duckdb::ToSql>> = Vec::new();

        if let Some(job_id) = filter.job_id {
            conditions.push("job_id = ?".to_string());
            params.push(Box::new(job_id));
        }

        if let Some(trace_id) = filter.trace_id {
            conditions.push("trace_id = ?".to_string());
            params.push(Box::new(trace_id.to_string()));
        }

        if let Some(from) = filter.from {
            conditions.push("timestamp >= ?".to_string());
            params.push(Box::new(from.to_rfc3339()));
        }

        if let Some(to) = filter.to {
            conditions.push("timestamp <= ?".to_string());
            params.push(Box::new(to.to_rfc3339()));
        }

        let mut query = String::from("SELECT event_id, trace_id, timestamp, actor, actor_id, event_type, job_id, workflow_id, model_id, wsids, activity_span_id, session_span_id, capability_id, policy_decision_id, payload FROM events");
        if !conditions.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(&conditions.join(" AND "));
        }
        query.push_str(" ORDER BY timestamp DESC LIMIT 200");

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| RecorderError::SinkError(e.to_string()))?;

        // Convert Vec<Box<dyn ToSql>> to something duckdb can use
        let param_refs: Vec<&dyn duckdb::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        struct RawEvent {
            event_id: String,
            trace_id: String,
            timestamp: String,
            actor: String,
            actor_id: String,
            event_type: String,
            job_id: Option<String>,
            workflow_id: Option<String>,
            model_id: Option<String>,
            wsids: Option<String>,
            activity_span_id: Option<String>,
            session_span_id: Option<String>,
            capability_id: Option<String>,
            policy_decision_id: Option<String>,
            payload: String,
        }

        let event_iter = stmt
            .query_map(duckdb::params_from_iter(param_refs), |row| {
                Ok(RawEvent {
                    event_id: row.get(0)?,
                    trace_id: row.get(1)?,
                    timestamp: row.get(2)?,
                    actor: row.get(3)?,
                    actor_id: row.get(4)?,
                    event_type: row.get(5)?,
                    job_id: row.get(6)?,
                    workflow_id: row.get(7)?,
                    model_id: row.get(8)?,
                    wsids: row.get(9)?,
                    activity_span_id: row.get(10)?,
                    session_span_id: row.get(11)?,
                    capability_id: row.get(12)?,
                    policy_decision_id: row.get(13)?,
                    payload: row.get(14)?,
                })
            })
            .map_err(|e| RecorderError::SinkError(e.to_string()))?;

        let mut events = Vec::new();
        for raw_res in event_iter {
            let raw = raw_res.map_err(|e| RecorderError::SinkError(e.to_string()))?;

            let event_id = uuid::Uuid::parse_str(&raw.event_id)
                .map_err(|e| RecorderError::InvalidEvent(format!("invalid event_id: {}", e)))?;
            let trace_id = uuid::Uuid::parse_str(&raw.trace_id)
                .map_err(|e| RecorderError::InvalidEvent(format!("invalid trace_id: {}", e)))?;
            let timestamp = chrono::DateTime::parse_from_rfc3339(&raw.timestamp)
                .map_err(|e| RecorderError::InvalidEvent(format!("invalid timestamp: {}", e)))?
                .with_timezone(&chrono::Utc);

            let actor = match raw.actor.as_str() {
                "human" => super::FlightRecorderActor::Human,
                "agent" => super::FlightRecorderActor::Agent,
                _ => super::FlightRecorderActor::System,
            };

            let event_type = match raw.event_type.as_str() {
                "llm_inference" => super::FlightRecorderEventType::LlmInference,
                "diagnostic" => super::FlightRecorderEventType::Diagnostic,
                "capability_action" => super::FlightRecorderEventType::CapabilityAction,
                _ => super::FlightRecorderEventType::System,
            };

            let wsids: Vec<String> = raw
                .wsids
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();
            let payload: Value = serde_json::from_str(&raw.payload).unwrap_or(Value::Null);

            events.push(super::FlightRecorderEvent {
                event_id,
                trace_id,
                timestamp,
                actor,
                actor_id: raw.actor_id,
                event_type,
                job_id: raw.job_id,
                workflow_id: raw.workflow_id,
                model_id: raw.model_id,
                wsids,
                activity_span_id: raw.activity_span_id,
                session_span_id: raw.session_span_id,
                capability_id: raw.capability_id,
                policy_decision_id: raw.policy_decision_id,
                payload,
            });
        }

        Ok(events)
    }
}

#[async_trait]
impl FlightRecorder for DuckDbFlightRecorder {
    async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
        event.validate()?;
        self.insert_event(event)
    }

    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        self.purge_retention()
    }

    async fn list_events(
        &self,
        filter: super::EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        self.query_events(filter)
    }
}

impl Clone for DuckDbFlightRecorder {
    fn clone(&self) -> Self {
        Self {
            conn: self.conn.clone(),
            retention_days: self.retention_days,
        }
    }
}

/// Helper to generate a trace id with fallback when the provided value is not a UUID.
pub fn parse_or_new_trace_id(input: Option<&str>) -> uuid::Uuid {
    input
        .and_then(|raw| uuid::Uuid::parse_str(raw).ok())
        .unwrap_or_else(uuid::Uuid::new_v4)
}

/// Convenience for system-level events.
pub fn system_event(message: &str, component: &str) -> FlightRecorderEvent {
    FlightRecorderEvent::new(
        FlightRecorderEventType::System,
        FlightRecorderActor::System,
        uuid::Uuid::new_v4(),
        serde_json::json!({
            "component": component,
            "message": message,
            "level": "info"
        }),
    )
}

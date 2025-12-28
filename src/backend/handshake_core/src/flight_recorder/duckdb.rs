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

        // DuckDB doesn't support parameterized INTERVAL, so we construct the query directly.
        // retention_days is a u32 from trusted config, not user input.
        let query = format!(
            "DELETE FROM events WHERE timestamp < (CURRENT_TIMESTAMP - INTERVAL '{}' DAY)",
            self.retention_days
        );
        let affected = conn
            .execute(&query, [])
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

        // NOTE: Avoid provider-specific datetime formatting; use epoch seconds for portability.
        let mut query = String::from("SELECT event_id, trace_id, EXTRACT(EPOCH FROM timestamp), actor, actor_id, event_type, job_id, workflow_id, model_id, wsids, activity_span_id, session_span_id, capability_id, policy_decision_id, payload FROM events");
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
            timestamp_epoch: f64,
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
                    timestamp_epoch: row.get(2)?,
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
            let secs = raw.timestamp_epoch.trunc() as i64;
            let nanos = ((raw.timestamp_epoch.fract()) * 1_000_000_000f64) as u32;
            let timestamp = chrono::DateTime::from_timestamp(secs, nanos)
                .ok_or_else(|| RecorderError::InvalidEvent("invalid timestamp".into()))?;

            let actor = match raw.actor.as_str() {
                "human" => super::FlightRecorderActor::Human,
                "agent" => super::FlightRecorderActor::Agent,
                _ => super::FlightRecorderActor::System,
            };

            let event_type = match raw.event_type.as_str() {
                "llm_inference" => super::FlightRecorderEventType::LlmInference,
                "diagnostic" => super::FlightRecorderEventType::Diagnostic,
                "capability_action" => super::FlightRecorderEventType::CapabilityAction,
                "security_violation" => super::FlightRecorderEventType::SecurityViolation,
                "workflow_recovery" => super::FlightRecorderEventType::WorkflowRecovery,
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
    async fn record_event(&self, mut event: FlightRecorderEvent) -> Result<(), RecorderError> {
        event.validate()?;
        // HARDENED_INVARIANT: Apply NFC normalization to all string content
        // before persistence to prevent Unicode bypass attacks [§11.5].
        event.normalize_payload();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flight_recorder::{EventFilter, FlightRecorderActor, FlightRecorderEventType};
    use serde_json::json;
    use uuid::Uuid;

    /// Helper to create a test event with given trace_id.
    fn test_event(trace_id: Uuid, job_id: Option<&str>) -> FlightRecorderEvent {
        let mut event = FlightRecorderEvent::new(
            FlightRecorderEventType::System,
            FlightRecorderActor::System,
            trace_id,
            json!({
                "component": "test",
                "message": "test event",
                "level": "info"
            }),
        );
        if let Some(jid) = job_id {
            event = event.with_job_id(jid);
        }
        event
    }

    #[tokio::test]
    async fn test_record_and_list_events() {
        let recorder = DuckDbFlightRecorder::new_in_memory(7).expect("failed to create recorder");
        let trace_id = Uuid::new_v4();

        // Record an event
        let event = test_event(trace_id, Some("job-123"));
        recorder
            .record_event(event)
            .await
            .expect("failed to record event");

        // List all events (no filter)
        let events = recorder
            .list_events(EventFilter::default())
            .await
            .expect("failed to list events");

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].trace_id, trace_id);
        assert_eq!(events[0].job_id, Some("job-123".to_string()));
    }

    #[tokio::test]
    async fn test_query_by_trace_id() {
        let recorder = DuckDbFlightRecorder::new_in_memory(7).expect("failed to create recorder");

        let trace_id_a = Uuid::new_v4();
        let trace_id_b = Uuid::new_v4();

        // Record two events with different trace IDs
        recorder
            .record_event(test_event(trace_id_a, Some("job-a")))
            .await
            .expect("failed to record event a");
        recorder
            .record_event(test_event(trace_id_b, Some("job-b")))
            .await
            .expect("failed to record event b");

        // Query by trace_id_a
        let filter = EventFilter {
            trace_id: Some(trace_id_a),
            ..Default::default()
        };
        let events = recorder
            .list_events(filter)
            .await
            .expect("failed to list events");

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].trace_id, trace_id_a);
        assert_eq!(events[0].job_id, Some("job-a".to_string()));
    }

    #[tokio::test]
    async fn test_query_by_job_id() {
        let recorder = DuckDbFlightRecorder::new_in_memory(7).expect("failed to create recorder");

        let trace_id = Uuid::new_v4();
        recorder
            .record_event(test_event(trace_id, Some("target-job")))
            .await
            .expect("failed to record event");
        recorder
            .record_event(test_event(Uuid::new_v4(), Some("other-job")))
            .await
            .expect("failed to record event");

        let filter = EventFilter {
            job_id: Some("target-job".to_string()),
            ..Default::default()
        };
        let events = recorder
            .list_events(filter)
            .await
            .expect("failed to list events");

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].job_id, Some("target-job".to_string()));
    }

    #[tokio::test]
    async fn test_retention_purges_old_events() {
        // Create recorder with 0-day retention (purge everything)
        let recorder = DuckDbFlightRecorder::new_in_memory(0).expect("failed to create recorder");

        // Record an event
        recorder
            .record_event(test_event(Uuid::new_v4(), None))
            .await
            .expect("failed to record event");

        // Verify event exists
        let before = recorder
            .list_events(EventFilter::default())
            .await
            .expect("failed to list events");
        assert_eq!(before.len(), 1);

        // Enforce retention (0 days = purge all)
        let purged = recorder
            .enforce_retention()
            .await
            .expect("failed to enforce retention");
        assert_eq!(purged, 1);

        // Verify event is gone
        let after = recorder
            .list_events(EventFilter::default())
            .await
            .expect("failed to list events");
        assert_eq!(after.len(), 0);
    }

    #[tokio::test]
    async fn test_nfc_normalization_applied() {
        let recorder = DuckDbFlightRecorder::new_in_memory(7).expect("failed to create recorder");
        let trace_id = Uuid::new_v4();

        // Create event with non-NFC string (NFD: é as e + combining acute accent)
        // NFD form: "café" = "cafe\u{0301}" (e followed by combining acute accent)
        // NFC form: "café" = "caf\u{00E9}" (single precomposed é character)
        let nfd_string = "cafe\u{0301}"; // NFD form
        let nfc_string = "caf\u{00E9}"; // NFC form (what we expect after normalization)

        let event = FlightRecorderEvent::new(
            FlightRecorderEventType::System,
            FlightRecorderActor::System,
            trace_id,
            json!({
                "message": nfd_string,
                "nested": {
                    "text": nfd_string
                }
            }),
        );

        recorder
            .record_event(event)
            .await
            .expect("failed to record event");

        let events = recorder
            .list_events(EventFilter {
                trace_id: Some(trace_id),
                ..Default::default()
            })
            .await
            .expect("failed to list events");

        assert_eq!(events.len(), 1);

        // Verify the payload was normalized to NFC
        let payload = &events[0].payload;
        assert_eq!(payload["message"].as_str().unwrap(), nfc_string);
        assert_eq!(payload["nested"]["text"].as_str().unwrap(), nfc_string);
    }

    #[tokio::test]
    async fn test_validation_rejects_nil_uuid() {
        let recorder = DuckDbFlightRecorder::new_in_memory(7).expect("failed to create recorder");

        // Create event with nil event_id (invalid)
        let mut event = test_event(Uuid::new_v4(), None);
        event.event_id = Uuid::nil();

        let result = recorder.record_event(event).await;
        assert!(result.is_err());

        if let Err(RecorderError::InvalidEvent(msg)) = result {
            assert!(msg.contains("event_id"));
        } else {
            panic!("Expected InvalidEvent error");
        }
    }

    #[tokio::test]
    async fn test_validation_rejects_nil_trace_id() {
        let recorder = DuckDbFlightRecorder::new_in_memory(7).expect("failed to create recorder");

        // Create event with nil trace_id (invalid)
        let mut event = test_event(Uuid::new_v4(), None);
        event.trace_id = Uuid::nil();

        let result = recorder.record_event(event).await;
        assert!(result.is_err());

        if let Err(RecorderError::InvalidEvent(msg)) = result {
            assert!(msg.contains("trace_id"));
        } else {
            panic!("Expected InvalidEvent error");
        }
    }

    #[tokio::test]
    async fn test_fr_evt_shapes_persisted() {
        let recorder = DuckDbFlightRecorder::new_in_memory(7).expect("failed to create recorder");
        let trace_id = Uuid::new_v4();

        // Test FR-EVT-001 System shape
        let system_event = FlightRecorderEvent::new(
            FlightRecorderEventType::System,
            FlightRecorderActor::System,
            trace_id,
            json!({
                "component": "workflow_engine",
                "message": "Workflow started",
                "level": "info",
                "details": { "workflow_id": "wf-123" }
            }),
        )
        .with_workflow_id("wf-123");

        recorder
            .record_event(system_event)
            .await
            .expect("failed to record system event");

        // Test FR-EVT-002 LLM Inference shape
        let llm_event = FlightRecorderEvent::new(
            FlightRecorderEventType::LlmInference,
            FlightRecorderActor::Agent,
            trace_id,
            json!({
                "model_id": "llama3",
                "input_tokens": 150,
                "output_tokens": 50,
                "latency_ms": 1200
            }),
        )
        .with_model_id("llama3")
        .with_job_id("job-456");

        recorder
            .record_event(llm_event)
            .await
            .expect("failed to record llm event");

        // Query back by trace_id
        let events = recorder
            .list_events(EventFilter {
                trace_id: Some(trace_id),
                ..Default::default()
            })
            .await
            .expect("failed to list events");

        assert_eq!(events.len(), 2);

        // Verify shapes are preserved
        let system = events
            .iter()
            .find(|e| matches!(e.event_type, FlightRecorderEventType::System))
            .unwrap();
        assert_eq!(system.payload["component"], "workflow_engine");
        assert_eq!(system.workflow_id, Some("wf-123".to_string()));

        let llm = events
            .iter()
            .find(|e| matches!(e.event_type, FlightRecorderEventType::LlmInference))
            .unwrap();
        assert_eq!(llm.payload["model_id"], "llama3");
        assert_eq!(llm.payload["input_tokens"], 150);
        assert_eq!(llm.model_id, Some("llama3".to_string()));
        assert_eq!(llm.job_id, Some("job-456".to_string()));
    }
}

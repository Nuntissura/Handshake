use std::{
    str::FromStr,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use duckdb::{Connection as DuckDbConnection, ToSql};
use serde_json::Value;
use uuid::Uuid;

use crate::ace::ArtifactHandle;
use crate::diagnostics::{
    aggregate_grouped, apply_metadata, diagnostic_metadata, DiagFilter, Diagnostic,
    DiagnosticActor, DiagnosticSeverity, DiagnosticSource, DiagnosticSurface, DiagnosticsStore,
    LinkConfidence, ProblemGroup,
};
use crate::storage::StorageError;

use super::{
    canonical_json_sha256_hex, FlightRecorder, FlightRecorderActor, FlightRecorderEvent,
    FlightRecorderEventType, FrEvt003Diagnostic, RecorderError,
};

pub struct DuckDbFlightRecorder {
    conn: Arc<Mutex<DuckDbConnection>>,
    retention_days: u32,
}

pub(crate) fn store_terminal_output_redacted(
    conn: &DuckDbConnection,
    event_id: Uuid,
    stdout_redacted: &str,
    stderr_redacted: &str,
) -> Result<(), RecorderError> {
    conn.execute(
        r#"
        INSERT INTO terminal_output (
            event_id,
            ts_utc,
            stdout_redacted,
            stderr_redacted
        ) VALUES (?, CURRENT_TIMESTAMP, ?, ?)
    "#,
        duckdb::params![event_id.to_string(), stdout_redacted, stderr_redacted],
    )
    .map_err(|e| RecorderError::SinkError(e.to_string()))?;

    Ok(())
}

pub(crate) fn store_tool_payload_redacted(
    conn: &DuckDbConnection,
    tool_call_id: Uuid,
    payload_kind: &str,
    payload_redacted: &Value,
) -> Result<(ArtifactHandle, String), RecorderError> {
    let payload_sha256 = canonical_json_sha256_hex(payload_redacted);
    let artifact_path =
        format!("data/flight_recorder/tool_payloads/{tool_call_id}/{payload_kind}.json");
    let payload_str = serde_json::to_string(payload_redacted)
        .map_err(|e| RecorderError::InvalidEvent(e.to_string()))?;

    conn.execute(
        r#"
        INSERT INTO tool_payloads (
            tool_call_id,
            payload_kind,
            ts_utc,
            artifact_path,
            payload,
            payload_sha256
        ) VALUES (?, ?, CURRENT_TIMESTAMP, ?, ?, ?)
    "#,
        duckdb::params![
            tool_call_id.to_string(),
            payload_kind,
            artifact_path.clone(),
            payload_str,
            payload_sha256.clone()
        ],
    )
    .map_err(|e| RecorderError::SinkError(e.to_string()))?;

    Ok((
        ArtifactHandle::new(tool_call_id, artifact_path),
        payload_sha256,
    ))
}

fn map_db_error(err: duckdb::Error) -> StorageError {
    StorageError::Database(err.to_string())
}

fn map_recorder_error(err: RecorderError) -> StorageError {
    StorageError::Database(err.to_string())
}

fn parse_timestamp(raw: String) -> Result<DateTime<Utc>, StorageError> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(raw.trim()) {
        return Ok(dt.with_timezone(&Utc));
    }

    let trimmed = raw.trim();
    let formats = [
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%dT%H:%M:%S",
    ];
    for fmt in formats {
        if let Ok(naive) = NaiveDateTime::parse_from_str(trimmed, fmt) {
            return Ok(DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc));
        }
    }

    Err(StorageError::Serialization(format!(
        "unparseable timestamp: {trimmed}"
    )))
}

fn format_duckdb_timestamp(timestamp: DateTime<Utc>) -> String {
    timestamp
        .naive_utc()
        .format("%Y-%m-%d %H:%M:%S%.f")
        .to_string()
}

fn actor_for_diagnostic(actor: Option<DiagnosticActor>) -> FlightRecorderActor {
    match actor {
        Some(DiagnosticActor::Human) => FlightRecorderActor::Human,
        Some(DiagnosticActor::Agent) => FlightRecorderActor::Agent,
        Some(DiagnosticActor::System) | None => FlightRecorderActor::System,
    }
}

fn map_diagnostic_row(row: &duckdb::Row) -> Result<Diagnostic, StorageError> {
    let id_str: String = row.get(0).map_err(map_db_error)?;
    let fingerprint: String = row.get(1).map_err(map_db_error)?;
    let title: String = row.get(2).map_err(map_db_error)?;
    let message: Option<String> = row.get(3).map_err(map_db_error)?;
    let severity_raw: String = row.get(4).map_err(map_db_error)?;
    let source_raw: String = row.get(5).map_err(map_db_error)?;
    let surface_raw: String = row.get(6).map_err(map_db_error)?;
    let tool: Option<String> = row.get(7).map_err(map_db_error)?;
    let code: Option<String> = row.get(8).map_err(map_db_error)?;
    let wsid: Option<String> = row.get(9).map_err(map_db_error)?;
    let job_id: Option<String> = row.get(10).map_err(map_db_error)?;
    let link_conf_raw: Option<String> = row.get(11).map_err(map_db_error)?;
    let timestamp_raw: String = row.get(12).map_err(map_db_error)?;
    let metadata_raw: Option<String> = row.get(13).map_err(map_db_error)?;

    let id = Uuid::parse_str(&id_str)
        .map_err(|e| StorageError::Serialization(format!("invalid diagnostic id: {e}")))?;
    let severity = DiagnosticSeverity::from_str(&severity_raw)
        .map_err(|e| StorageError::Serialization(e.to_string()))?;
    let source = DiagnosticSource::from_str(&source_raw)
        .map_err(|e| StorageError::Serialization(e.to_string()))?;
    let surface = DiagnosticSurface::from_str(&surface_raw)
        .map_err(|e| StorageError::Serialization(e.to_string()))?;
    let link_confidence = link_conf_raw
        .as_deref()
        .map(LinkConfidence::from_str)
        .transpose()
        .map_err(|e| StorageError::Serialization(e.to_string()))?
        .unwrap_or_default();
    let timestamp = parse_timestamp(timestamp_raw)?;

    let mut diagnostic = Diagnostic {
        id,
        fingerprint,
        title,
        message: message.unwrap_or_default(),
        severity,
        source,
        surface,
        tool,
        code,
        tags: None,
        wsid,
        job_id,
        model_id: None,
        actor: None,
        capability_id: None,
        policy_decision_id: None,
        locations: None,
        evidence_refs: None,
        link_confidence,
        status: None,
        count: None,
        first_seen: None,
        last_seen: None,
        timestamp,
        updated_at: None,
    };

    if let Some(meta_str) = metadata_raw {
        if let Ok(meta_val) = serde_json::from_str::<Value>(&meta_str) {
            apply_metadata(&mut diagnostic, &meta_val);
        }
    }

    Ok(diagnostic)
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
                model_session_id TEXT,
                activity_span_id TEXT,
                session_span_id TEXT,
                capability_id TEXT,
                policy_decision_id TEXT,
                wsids JSON,
                payload JSON NOT NULL
            );
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
            ALTER TABLE events ADD COLUMN IF NOT EXISTS model_session_id TEXT;
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

        conn.execute_batch(
            r#"
            CREATE INDEX IF NOT EXISTS idx_events_trace_id ON events(trace_id);
            CREATE INDEX IF NOT EXISTS idx_events_job_id ON events(job_id);
            CREATE INDEX IF NOT EXISTS idx_events_model_session_id ON events(model_session_id);
            CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp);
        "#,
        )
        .map_err(|e| RecorderError::SinkError(e.to_string()))?;

        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS diagnostics (
                id UUID PRIMARY KEY,
                fingerprint VARCHAR NOT NULL,
                title VARCHAR NOT NULL,
                message TEXT,
                severity ENUM('fatal', 'error', 'warning', 'info', 'hint'),
                source VARCHAR NOT NULL,
                surface VARCHAR NOT NULL,
                tool VARCHAR,
                code VARCHAR,
                wsid VARCHAR,
                job_id UUID,
                link_confidence ENUM('direct', 'inferred', 'ambiguous', 'unlinked'),
                timestamp TIMESTAMP NOT NULL,
                metadata JSON
            );
            CREATE INDEX IF NOT EXISTS idx_diag_fingerprint ON diagnostics(fingerprint);
            CREATE INDEX IF NOT EXISTS idx_diag_job_id ON diagnostics(job_id);
        "#,
        )
        .map_err(|e| RecorderError::SinkError(e.to_string()))?;

        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS fr_events (
                event_id        BIGINT PRIMARY KEY,
                ts_utc          TIMESTAMP NOT NULL,
                session_id      TEXT,
                task_id         TEXT,
                job_id          TEXT,
                workflow_run_id TEXT,
                event_kind      TEXT NOT NULL,
                source          TEXT NOT NULL,
                level           TEXT,
                message         TEXT,
                payload         JSON
            );
            CREATE INDEX IF NOT EXISTS idx_fr_events_job_id ON fr_events(job_id);
            CREATE INDEX IF NOT EXISTS idx_fr_events_kind ON fr_events(event_kind);
        "#,
        )
        .map_err(|e| RecorderError::SinkError(e.to_string()))?;

        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS terminal_output (
                event_id UUID PRIMARY KEY,
                ts_utc TIMESTAMP NOT NULL,
                stdout_redacted TEXT,
                stderr_redacted TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_terminal_output_ts ON terminal_output(ts_utc);
        "#,
        )
        .map_err(|e| RecorderError::SinkError(e.to_string()))?;

        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS tool_payloads (
                tool_call_id UUID NOT NULL,
                payload_kind TEXT NOT NULL,
                ts_utc TIMESTAMP NOT NULL,
                artifact_path TEXT NOT NULL,
                payload JSON NOT NULL,
                payload_sha256 TEXT NOT NULL,
                PRIMARY KEY(tool_call_id, payload_kind)
            );
            CREATE INDEX IF NOT EXISTS idx_tool_payloads_ts ON tool_payloads(ts_utc);
        "#,
        )
        .map_err(|e| RecorderError::SinkError(e.to_string()))?;

        Ok(())
    }

    fn insert_diagnostic_row(
        &self,
        diagnostic: &Diagnostic,
        metadata: &Value,
    ) -> Result<(), StorageError> {
        let meta_str = serde_json::to_string(metadata)?;
        let conn = self
            .conn
            .lock()
            .map_err(|_| StorageError::Database("lock error".into()))?;
        let job_uuid = match diagnostic.job_id.as_deref() {
            Some(raw) => Some(
                Uuid::parse_str(raw)
                    .map_err(|_| StorageError::Validation("invalid job_id uuid"))?,
            ),
            None => None,
        };

        conn.execute(
            r#"
            INSERT INTO diagnostics (
                id,
                fingerprint,
                title,
                message,
                severity,
                source,
                surface,
                tool,
                code,
                wsid,
                job_id,
                link_confidence,
                timestamp,
                metadata
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            duckdb::params![
                diagnostic.id.to_string(),
                diagnostic.fingerprint,
                diagnostic.title,
                diagnostic.message,
                diagnostic.severity.as_str(),
                diagnostic.source.as_str(),
                diagnostic.surface.as_str(),
                diagnostic.tool.as_ref(),
                diagnostic.code.as_ref(),
                diagnostic.wsid.as_ref(),
                job_uuid.map(|j| j.to_string()),
                diagnostic.link_confidence.as_str(),
                format_duckdb_timestamp(diagnostic.timestamp),
                meta_str
            ],
        )
        .map_err(map_db_error)?;

        Ok(())
    }

    fn query_diagnostics(&self, filter: DiagFilter) -> Result<Vec<Diagnostic>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| StorageError::Database("lock error".into()))?;

        let mut conditions = Vec::new();
        let mut params: Vec<Box<dyn ToSql>> = Vec::new();

        if let Some(severity) = filter.severity {
            conditions.push("severity = ?".to_string());
            params.push(Box::new(severity.as_str().to_string()));
        }
        if let Some(source) = filter.source {
            conditions.push("source = ?".to_string());
            params.push(Box::new(source));
        }
        if let Some(surface) = filter.surface {
            conditions.push("surface = ?".to_string());
            params.push(Box::new(surface.as_str().to_string()));
        }
        if let Some(wsid) = filter.wsid {
            conditions.push("wsid = ?".to_string());
            params.push(Box::new(wsid));
        }
        if let Some(job_id) = filter.job_id {
            conditions.push("job_id = ?".to_string());
            params.push(Box::new(job_id.to_string()));
        }
        if let Some(from) = filter.from {
            conditions.push("timestamp >= ?".to_string());
            params.push(Box::new(format_duckdb_timestamp(from)));
        }
        if let Some(to) = filter.to {
            conditions.push("timestamp <= ?".to_string());
            params.push(Box::new(format_duckdb_timestamp(to)));
        }
        if let Some(fp) = filter.fingerprint {
            conditions.push("fingerprint = ?".to_string());
            params.push(Box::new(fp));
        }

        let mut query = String::from(
            "SELECT id, fingerprint, title, message, CAST(severity AS VARCHAR) AS severity, source, surface, tool, code, wsid, job_id, CAST(link_confidence AS VARCHAR) AS link_confidence, CAST(timestamp AS VARCHAR) AS timestamp, metadata FROM diagnostics",
        );
        if !conditions.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(&conditions.join(" AND "));
        }
        query.push_str(" ORDER BY diagnostics.timestamp DESC");
        if let Some(limit) = filter.limit {
            query.push_str(" LIMIT ?");
            params.push(Box::new(limit as i64));
        }

        let mut stmt = conn.prepare(&query).map_err(map_db_error)?;
        let param_refs: Vec<&dyn ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let mut rows = stmt
            .query(duckdb::params_from_iter(param_refs))
            .map_err(map_db_error)?;

        let mut diagnostics = Vec::new();
        while let Some(row) = rows.next().map_err(map_db_error)? {
            diagnostics.push(map_diagnostic_row(row)?);
        }

        Ok(diagnostics)
    }

    fn build_diagnostic_event(
        &self,
        diagnostic: &Diagnostic,
    ) -> Result<FlightRecorderEvent, StorageError> {
        let payload = FrEvt003Diagnostic {
            diagnostic_id: diagnostic.id.to_string(),
            wsid: diagnostic.wsid.clone(),
            severity: Some(diagnostic.severity.as_str().to_string()),
            source: Some(diagnostic.source.as_str()),
        };

        let mut event = FlightRecorderEvent::new(
            FlightRecorderEventType::Diagnostic,
            actor_for_diagnostic(diagnostic.actor.clone()),
            diagnostic.id,
            serde_json::to_value(&payload)?,
        )
        .with_actor_id("diagnostics_store");

        if let Some(job_id) = diagnostic.job_id.clone() {
            event = event.with_job_id(job_id);
        }
        if let Some(wsid) = diagnostic.wsid.clone() {
            event = event.with_wsids(vec![wsid]);
        }

        Ok(event)
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
                model_session_id,
                activity_span_id,
                session_span_id,
                capability_id,
                policy_decision_id,
                wsids,
                payload
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
                event.model_session_id.as_deref(),
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
        let affected_events = conn
            .execute(&query, [])
            .map_err(|e| RecorderError::SinkError(e.to_string()))?;

        let fr_query = format!(
            "DELETE FROM fr_events WHERE ts_utc < (CURRENT_TIMESTAMP - INTERVAL '{}' DAY)",
            self.retention_days
        );
        let affected_fr_events = conn
            .execute(&fr_query, [])
            .map_err(|e| RecorderError::SinkError(e.to_string()))?;

        let terminal_query = format!(
            "DELETE FROM terminal_output WHERE ts_utc < (CURRENT_TIMESTAMP - INTERVAL '{}' DAY)",
            self.retention_days
        );
        let affected_terminal_output = conn
            .execute(&terminal_query, [])
            .map_err(|e| RecorderError::SinkError(e.to_string()))?;

        let tool_payload_query = format!(
            "DELETE FROM tool_payloads WHERE ts_utc < (CURRENT_TIMESTAMP - INTERVAL '{}' DAY)",
            self.retention_days
        );
        let affected_tool_payloads = conn
            .execute(&tool_payload_query, [])
            .map_err(|e| RecorderError::SinkError(e.to_string()))?;

        Ok((affected_events
            + affected_fr_events
            + affected_terminal_output
            + affected_tool_payloads) as u64)
    }

    fn query_events(
        &self,
        filter: super::EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        let conn = self.conn.lock().map_err(|_| RecorderError::LockError)?;

        let mut conditions = Vec::new();
        let mut params: Vec<Box<dyn duckdb::ToSql>> = Vec::new();

        if let Some(event_id) = filter.event_id {
            conditions.push("event_id = ?".to_string());
            params.push(Box::new(event_id.to_string()));
        }

        if let Some(job_id) = filter.job_id {
            conditions.push("job_id = ?".to_string());
            params.push(Box::new(job_id));
        }

        if let Some(trace_id) = filter.trace_id {
            conditions.push("trace_id = ?".to_string());
            params.push(Box::new(trace_id.to_string()));
        }

        if let Some(model_session_id) = filter.model_session_id {
            conditions.push("model_session_id = ?".to_string());
            params.push(Box::new(model_session_id));
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
        let mut query = String::from("SELECT event_id, trace_id, EXTRACT(EPOCH FROM timestamp), actor, actor_id, event_type, job_id, workflow_id, model_id, model_session_id, wsids, activity_span_id, session_span_id, capability_id, policy_decision_id, payload FROM events");
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
            model_session_id: Option<String>,
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
                    model_session_id: row.get(9)?,
                    wsids: row.get(10)?,
                    activity_span_id: row.get(11)?,
                    session_span_id: row.get(12)?,
                    capability_id: row.get(13)?,
                    policy_decision_id: row.get(14)?,
                    payload: row.get(15)?,
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

            let wsids: Vec<String> = raw
                .wsids
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();
            let payload: Value = serde_json::from_str(&raw.payload).unwrap_or(Value::Null);
            let payload_type = payload.get("type").and_then(|value| value.as_str());
            // Back-compat mapping table for stored event_type strings -> enum variants.
            let event_type = match raw.event_type.as_str() {
                "terminal_command" => super::FlightRecorderEventType::TerminalCommand,
                "editor_edit" => super::FlightRecorderEventType::EditorEdit,
                "llm_inference" => super::FlightRecorderEventType::LlmInference,
                "tool_call" => super::FlightRecorderEventType::ToolCall,
                "diagnostic" => super::FlightRecorderEventType::Diagnostic,
                "micro_task_loop_started" => super::FlightRecorderEventType::MicroTaskLoopStarted,
                "micro_task_iteration_started" => {
                    super::FlightRecorderEventType::MicroTaskIterationStarted
                }
                "micro_task_iteration_complete" => {
                    super::FlightRecorderEventType::MicroTaskIterationComplete
                }
                "micro_task_complete" => super::FlightRecorderEventType::MicroTaskComplete,
                "micro_task_escalated" => super::FlightRecorderEventType::MicroTaskEscalated,
                "micro_task_hard_gate" => super::FlightRecorderEventType::MicroTaskHardGate,
                "micro_task_pause_requested" => {
                    super::FlightRecorderEventType::MicroTaskPauseRequested
                }
                "micro_task_resumed" => super::FlightRecorderEventType::MicroTaskResumed,
                "micro_task_loop_completed" => {
                    super::FlightRecorderEventType::MicroTaskLoopCompleted
                }
                "micro_task_loop_failed" => super::FlightRecorderEventType::MicroTaskLoopFailed,
                "micro_task_loop_cancelled" => {
                    super::FlightRecorderEventType::MicroTaskLoopCancelled
                }
                "micro_task_validation" => super::FlightRecorderEventType::MicroTaskValidation,
                "micro_task_lora_selection" => {
                    super::FlightRecorderEventType::MicroTaskLoraSelection
                }
                "micro_task_drop_back" => super::FlightRecorderEventType::MicroTaskDropBack,
                "micro_task_distillation_candidate" => {
                    super::FlightRecorderEventType::MicroTaskDistillationCandidate
                }
                "micro_task_skipped" => super::FlightRecorderEventType::MicroTaskSkipped,
                "micro_task_blocked" => super::FlightRecorderEventType::MicroTaskBlocked,
                "work_packet_created" => super::FlightRecorderEventType::LocusWorkPacketCreated,
                "work_packet_updated" => super::FlightRecorderEventType::LocusWorkPacketUpdated,
                "work_packet_gated" => super::FlightRecorderEventType::LocusWorkPacketGated,
                "work_packet_completed" => super::FlightRecorderEventType::LocusWorkPacketCompleted,
                "work_packet_deleted" => super::FlightRecorderEventType::LocusWorkPacketDeleted,
                "micro_tasks_registered" => {
                    super::FlightRecorderEventType::LocusMicroTasksRegistered
                }
                "mt_iteration_completed" => {
                    super::FlightRecorderEventType::LocusMtIterationCompleted
                }
                "mt_started" => super::FlightRecorderEventType::LocusMtStarted,
                "mt_completed" => super::FlightRecorderEventType::LocusMtCompleted,
                "mt_escalated" => super::FlightRecorderEventType::LocusMtEscalated,
                "mt_failed" => super::FlightRecorderEventType::LocusMtFailed,
                "dependency_added" => super::FlightRecorderEventType::LocusDependencyAdded,
                "dependency_removed" => super::FlightRecorderEventType::LocusDependencyRemoved,
                "task_board_entry_added" => {
                    super::FlightRecorderEventType::LocusTaskBoardEntryAdded
                }
                "task_board_synced" => super::FlightRecorderEventType::LocusTaskBoardSynced,
                "task_board_status_changed" => {
                    super::FlightRecorderEventType::LocusTaskBoardStatusChanged
                }
                "sync_started" => super::FlightRecorderEventType::LocusSyncStarted,
                "sync_completed" => super::FlightRecorderEventType::LocusSyncCompleted,
                "sync_failed" => super::FlightRecorderEventType::LocusSyncFailed,
                "work_query_executed" => super::FlightRecorderEventType::LocusWorkQueryExecuted,
                "debug_bundle_export" => super::FlightRecorderEventType::DebugBundleExport,
                "governance_pack_export" => super::FlightRecorderEventType::GovernancePackExport,
                "memory_write_proposed" => super::FlightRecorderEventType::MemoryWriteProposed,
                "memory_write_reviewed" => super::FlightRecorderEventType::MemoryWriteReviewed,
                "memory_write_committed" => super::FlightRecorderEventType::MemoryWriteCommitted,
                "memory_pack_built" => super::FlightRecorderEventType::MemoryPackBuilt,
                "memory_item_status_changed" => {
                    super::FlightRecorderEventType::MemoryItemStatusChanged
                }
                "runtime_chat_message_appended" => {
                    super::FlightRecorderEventType::RuntimeChatMessageAppended
                }
                "runtime_chat_ans001_validation" => {
                    super::FlightRecorderEventType::RuntimeChatAns001Validation
                }
                "runtime_chat_session_closed" => {
                    super::FlightRecorderEventType::RuntimeChatSessionClosed
                }
                "workflow_recovery" => super::FlightRecorderEventType::WorkflowRecovery,
                "gov_mailbox_message_created" => {
                    super::FlightRecorderEventType::GovMailboxMessageCreated
                }
                "gov_mailbox_exported" => super::FlightRecorderEventType::GovMailboxExported,
                "gov_mailbox_transcribed" => super::FlightRecorderEventType::GovMailboxTranscribed,
                "gov_gate_transition" => super::FlightRecorderEventType::GovGateTransition,
                "gov_work_packet_activated" => {
                    super::FlightRecorderEventType::GovWorkPacketActivated
                }
                "gov_decision_created" => super::FlightRecorderEventType::GovDecisionCreated,
                "gov_decision_applied" => super::FlightRecorderEventType::GovDecisionApplied,
                "gov_auto_signature_created" => {
                    super::FlightRecorderEventType::GovAutoSignatureCreated
                }
                "gov_human_intervention_requested" => {
                    super::FlightRecorderEventType::GovHumanInterventionRequested
                }
                "gov_human_intervention_received" => {
                    super::FlightRecorderEventType::GovHumanInterventionReceived
                }
                "governance.check.started" => {
                    super::FlightRecorderEventType::GovernanceCheckStarted
                }
                "governance.check.completed" => {
                    super::FlightRecorderEventType::GovernanceCheckCompleted
                }
                "governance.check.blocked" => {
                    super::FlightRecorderEventType::GovernanceCheckBlocked
                }
                "cloud_escalation_requested" => {
                    super::FlightRecorderEventType::CloudEscalationRequested
                }
                "cloud_escalation_approved" => {
                    super::FlightRecorderEventType::CloudEscalationApproved
                }
                "cloud_escalation_denied" => super::FlightRecorderEventType::CloudEscalationDenied,
                "cloud_escalation_executed" => {
                    super::FlightRecorderEventType::CloudEscalationExecuted
                }
                "security_violation" => super::FlightRecorderEventType::SecurityViolation,
                "data_bronze_created" => super::FlightRecorderEventType::DataBronzeCreated,
                "data_silver_created" => super::FlightRecorderEventType::DataSilverCreated,
                "data_silver_updated" => super::FlightRecorderEventType::DataSilverUpdated,
                "data_embedding_computed" => super::FlightRecorderEventType::DataEmbeddingComputed,
                "data_embedding_model_changed" => {
                    super::FlightRecorderEventType::DataEmbeddingModelChanged
                }
                "data_index_updated" => super::FlightRecorderEventType::DataIndexUpdated,
                "data_index_rebuilt" => super::FlightRecorderEventType::DataIndexRebuilt,
                "data_validation_failed" => super::FlightRecorderEventType::DataValidationFailed,
                "data_retrieval_executed" => super::FlightRecorderEventType::DataRetrievalExecuted,
                "data_context_assembled" => super::FlightRecorderEventType::DataContextAssembled,
                "data_pollution_alert" => super::FlightRecorderEventType::DataPollutionAlert,
                "data_quality_degradation" => {
                    super::FlightRecorderEventType::DataQualityDegradation
                }
                "data_reembedding_triggered" => {
                    super::FlightRecorderEventType::DataReembeddingTriggered
                }
                "data_relationship_extracted" => {
                    super::FlightRecorderEventType::DataRelationshipExtracted
                }
                "data_golden_query_failed" => super::FlightRecorderEventType::DataGoldenQueryFailed,
                "loom_block_created" => super::FlightRecorderEventType::LoomBlockCreated,
                "loom_block_updated" => super::FlightRecorderEventType::LoomBlockUpdated,
                "loom_block_deleted" => super::FlightRecorderEventType::LoomBlockDeleted,
                "loom_edge_created" => super::FlightRecorderEventType::LoomEdgeCreated,
                "loom_edge_deleted" => super::FlightRecorderEventType::LoomEdgeDeleted,
                "loom_dedup_hit" => super::FlightRecorderEventType::LoomDedupHit,
                "loom_preview_generated" => super::FlightRecorderEventType::LoomPreviewGenerated,
                "loom_ai_tag_suggested" => super::FlightRecorderEventType::LoomAiTagSuggested,
                "loom_ai_tag_accepted" => super::FlightRecorderEventType::LoomAiTagAccepted,
                "loom_ai_tag_rejected" => super::FlightRecorderEventType::LoomAiTagRejected,
                "loom_view_queried" => super::FlightRecorderEventType::LoomViewQueried,
                "loom_search_executed" => super::FlightRecorderEventType::LoomSearchExecuted,
                "session_scheduler.enqueue" | "session_scheduler_enqueue" => {
                    super::FlightRecorderEventType::SessionSchedulerEnqueue
                }
                "session_scheduler.dispatch" | "session_scheduler_dispatch" => {
                    super::FlightRecorderEventType::SessionSchedulerDispatch
                }
                "session_scheduler.rate_limited" | "session_scheduler_rate_limited" => {
                    super::FlightRecorderEventType::SessionSchedulerRateLimited
                }
                "session_scheduler.cancelled" | "session_scheduler_cancelled" => {
                    super::FlightRecorderEventType::SessionSchedulerCancelled
                }
                "session.spawn_requested" => super::FlightRecorderEventType::SessionSpawnRequested,
                "session.spawn_accepted" => super::FlightRecorderEventType::SessionSpawnAccepted,
                "session.spawn_rejected" => super::FlightRecorderEventType::SessionSpawnRejected,
                "session.announce_back" => super::FlightRecorderEventType::SessionSpawnAnnounceBack,
                "session.cascade_cancel" => super::FlightRecorderEventType::SessionCascadeCancel,
                "session_checkpoint_created" => {
                    super::FlightRecorderEventType::SessionCheckpointCreated
                }
                "session_recovery_attempted" => {
                    super::FlightRecorderEventType::SessionRecoveryAttempted
                }
                "workspace_isolation.denied" => {
                    super::FlightRecorderEventType::WorkspaceIsolationDenied
                }
                "workspace_cross_session.denied" => {
                    super::FlightRecorderEventType::WorkspaceCrossSessionDenied
                }
                "workspace_cross_session.approved" => {
                    super::FlightRecorderEventType::WorkspaceCrossSessionApproved
                }
                "capability_action" => {
                    if payload_type == Some("terminal_command") {
                        super::FlightRecorderEventType::TerminalCommand
                    } else {
                        super::FlightRecorderEventType::CapabilityAction
                    }
                }
                "system" => super::FlightRecorderEventType::System,
                _ => super::FlightRecorderEventType::System,
            };

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
                model_session_id: raw.model_session_id,
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
        // Validation gates ingestion only; stored events are not re-validated on read.
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

    fn duckdb_connection(&self) -> Option<Arc<Mutex<DuckDbConnection>>> {
        Some(self.conn.clone())
    }
}

#[async_trait]
impl DiagnosticsStore for DuckDbFlightRecorder {
    async fn record_diagnostic(&self, diagnostic: Diagnostic) -> Result<(), StorageError> {
        let metadata = diagnostic_metadata(&diagnostic);
        self.insert_diagnostic_row(&diagnostic, &metadata)?;

        let event = self.build_diagnostic_event(&diagnostic)?;
        FlightRecorder::record_event(self, event)
            .await
            .map_err(map_recorder_error)?;
        Ok(())
    }

    async fn list_problems(&self, filter: DiagFilter) -> Result<Vec<ProblemGroup>, StorageError> {
        let mut filter_all = filter.clone();
        filter_all.limit = None;
        let diagnostics = self.query_diagnostics(filter_all)?;
        let mut grouped = aggregate_grouped(diagnostics);
        if let Some(limit) = filter.limit {
            grouped.truncate(limit as usize);
        }

        let problems = grouped
            .into_iter()
            .map(|diag| ProblemGroup {
                fingerprint: diag.fingerprint.clone(),
                count: diag.count.unwrap_or(1),
                first_seen: diag.first_seen.unwrap_or(diag.timestamp),
                last_seen: diag.last_seen.unwrap_or(diag.timestamp),
                sample: diag,
            })
            .collect();

        Ok(problems)
    }

    async fn get_diagnostic(&self, id: Uuid) -> Result<Diagnostic, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| StorageError::Database("lock error".into()))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, fingerprint, title, message, CAST(severity AS VARCHAR) AS severity, source, surface, tool, code, wsid, job_id, CAST(link_confidence AS VARCHAR) AS link_confidence, CAST(timestamp AS VARCHAR) AS timestamp, metadata FROM diagnostics WHERE id = ?",
            )
            .map_err(map_db_error)?;

        let mut rows = stmt
            .query(duckdb::params![id.to_string()])
            .map_err(map_db_error)?;
        let row = rows.next().map_err(map_db_error)?;
        let row = row.ok_or(StorageError::NotFound("diagnostic"))?;
        map_diagnostic_row(row)
    }

    async fn list_diagnostics(&self, filter: DiagFilter) -> Result<Vec<Diagnostic>, StorageError> {
        self.query_diagnostics(filter)
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
    use crate::diagnostics::{
        DiagFilter, DiagnosticActor, DiagnosticInput, DiagnosticSeverity, DiagnosticSource,
        DiagnosticSurface, LinkConfidence,
    };
    use crate::flight_recorder::{EventFilter, FlightRecorderActor, FlightRecorderEventType};
    use serde_json::json;
    use tempfile::tempdir;
    use uuid::Uuid;

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

    fn list_query_string_cells(
        conn: &DuckDbConnection,
        query: &str,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut stmt = conn.prepare(query)?;
        let mut rows = stmt.query([])?;

        let mut values = Vec::new();
        while let Some(row) = rows.next()? {
            for col in 0..32 {
                if let Ok(value) = row.get::<usize, String>(col) {
                    values.push(value);
                }
            }
        }

        Ok(values)
    }

    fn list_index_introspection_values(
        conn: &DuckDbConnection,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        #[cfg(not(windows))]
        {
            if let Ok(values) = list_query_string_cells(conn, "PRAGMA index_list('events')") {
                if !values.is_empty() {
                    return Ok(values);
                }
            }
        }

        for query in [
            "SELECT * FROM duckdb_indexes()",
            "SELECT * FROM duckdb_indexes",
        ] {
            if let Ok(values) = list_query_string_cells(conn, query) {
                if !values.is_empty() {
                    return Ok(values);
                }
            }
        }

        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "duckdb index introspection not available",
        )
        .into())
    }

    #[tokio::test]
    async fn test_new_on_path_migrates_legacy_db_and_creates_indexes(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempdir()?;
        let db_path = tmp.path().join("legacy_flight_recorder.duckdb");

        {
            let conn = DuckDbConnection::open(&db_path)?;
            conn.execute_batch(
                r#"
                CREATE TABLE events (
                    event_id UUID PRIMARY KEY,
                    timestamp TIMESTAMPTZ,
                    actor TEXT,
                    actor_id TEXT,
                    event_type TEXT,
                    job_id TEXT,
                    payload JSON
                );
            "#,
            )?;
        }

        let recorder = DuckDbFlightRecorder::new_on_path(&db_path, 7)?;

        let conn = recorder.connection();
        let conn = conn.lock().map_err(|_| RecorderError::LockError)?;

        let index_values = list_index_introspection_values(&conn)?;
        assert!(
            index_values
                .iter()
                .any(|value| value == "idx_events_trace_id"),
            "expected idx_events_trace_id in DuckDB index introspection; got: {index_values:?}"
        );
        assert!(
            index_values
                .iter()
                .any(|value| value == "idx_events_job_id"),
            "expected idx_events_job_id in DuckDB index introspection; got: {index_values:?}"
        );
        assert!(
            index_values
                .iter()
                .any(|value| value == "idx_events_model_session_id"),
            "expected idx_events_model_session_id in DuckDB index introspection; got: {index_values:?}"
        );
        assert!(
            index_values
                .iter()
                .any(|value| value == "idx_events_timestamp"),
            "expected idx_events_timestamp in DuckDB index introspection; got: {index_values:?}"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_record_and_list_events() -> Result<(), Box<dyn std::error::Error>> {
        let recorder = DuckDbFlightRecorder::new_in_memory(7)?;
        let trace_id = Uuid::new_v4();

        recorder
            .record_event(test_event(trace_id, Some("job-123")))
            .await?;

        let events = recorder.list_events(EventFilter::default()).await?;

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].trace_id, trace_id);
        assert_eq!(events[0].job_id, Some("job-123".to_string()));
        Ok(())
    }

    #[tokio::test]
    async fn query_by_session() -> Result<(), Box<dyn std::error::Error>> {
        let recorder = DuckDbFlightRecorder::new_in_memory(7)?;
        let trace_id = Uuid::new_v4();

        recorder
            .record_event(test_event(trace_id, Some("job-a")).with_model_session_id("session-a"))
            .await?;
        recorder
            .record_event(test_event(trace_id, Some("job-b")).with_model_session_id("session-b"))
            .await?;

        let events = recorder
            .list_events(EventFilter {
                model_session_id: Some("session-a".to_string()),
                ..Default::default()
            })
            .await?;

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].job_id, Some("job-a".to_string()));
        assert_eq!(events[0].model_session_id, Some("session-a".to_string()));
        Ok(())
    }

    #[tokio::test]
    async fn tripwire_session_scoped_events_require_model_session_id(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let recorder = DuckDbFlightRecorder::new_in_memory(7)?;
        let trace_id = Uuid::new_v4();
        let model_session_id = "session-tripwire";

        let session_events = vec![
            (
                FlightRecorderEventType::SessionSchedulerEnqueue,
                json!({
                    "type": "session_scheduler.enqueue",
                    "event_id": "FR-EVT-SESS-SCHED-001",
                    "session_id": "session-tripwire",
                    "job_id": "job-enqueue",
                    "job_kind": "model_run",
                    "lane": "PRIMARY",
                    "priority": 50,
                    "concurrency_group": "test",
                    "queue_depth": 1,
                    "attempt": 0,
                    "max_retries": 2,
                    "retry_backoff": "exponential",
                    "cancellation_token": "token",
                }),
            ),
            (
                FlightRecorderEventType::SessionSchedulerDispatch,
                json!({
                    "type": "session_scheduler.dispatch",
                    "event_id": "FR-EVT-SESS-SCHED-002",
                    "session_id": "session-tripwire",
                    "job_id": "job-dispatch",
                    "job_kind": "model_run",
                    "lane": "PRIMARY",
                    "priority": 50,
                    "concurrency_group": "test",
                    "queue_wait_ms": 10,
                    "attempt": 1,
                }),
            ),
            (
                FlightRecorderEventType::SessionSchedulerRateLimited,
                json!({
                    "type": "session_scheduler.rate_limited",
                    "event_id": "FR-EVT-SESS-SCHED-003",
                    "session_id": "session-tripwire",
                    "job_id": "job-rate-limited",
                    "provider": "provider",
                    "job_kind": "model_run",
                    "lane": "PRIMARY",
                    "priority": 50,
                    "concurrency_group": "test",
                    "queue_wait_ms": 20,
                    "attempt": 1,
                    "backoff_ms": 10,
                    "reason": "rate limit",
                }),
            ),
            (
                FlightRecorderEventType::SessionSchedulerCancelled,
                json!({
                    "type": "session_scheduler.cancelled",
                    "event_id": "FR-EVT-SESS-SCHED-004",
                    "session_id": "session-tripwire",
                    "job_id": "job-cancelled",
                    "job_kind": "model_run",
                    "lane": "PRIMARY",
                    "priority": 50,
                    "concurrency_group": "test",
                    "attempt": 1,
                    "cancelled_by": "scheduler",
                    "reason": "test path",
                }),
            ),
        ];

        for (event_type, payload) in session_events {
            recorder
                .record_event(
                    FlightRecorderEvent::new(
                        event_type,
                        FlightRecorderActor::System,
                        trace_id,
                        payload,
                    )
                    .with_model_session_id(model_session_id.to_string()),
                )
                .await?;
        }

        let events = recorder
            .list_events(EventFilter {
                model_session_id: Some(model_session_id.to_string()),
                ..Default::default()
            })
            .await?;

        assert_eq!(events.len(), 4);
        for event in events {
            match event.event_type {
                FlightRecorderEventType::SessionSchedulerEnqueue
                | FlightRecorderEventType::SessionSchedulerDispatch
                | FlightRecorderEventType::SessionSchedulerRateLimited
                | FlightRecorderEventType::SessionSchedulerCancelled => {
                    assert_eq!(event.model_session_id, Some(model_session_id.to_string()))
                }
                _ => {}
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_query_by_trace_id() -> Result<(), Box<dyn std::error::Error>> {
        let recorder = DuckDbFlightRecorder::new_in_memory(7)?;

        let trace_id_a = Uuid::new_v4();
        let trace_id_b = Uuid::new_v4();

        recorder
            .record_event(test_event(trace_id_a, Some("job-a")))
            .await?;
        recorder
            .record_event(test_event(trace_id_b, Some("job-b")))
            .await?;

        let events = recorder
            .list_events(EventFilter {
                trace_id: Some(trace_id_a),
                ..Default::default()
            })
            .await?;

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].trace_id, trace_id_a);
        assert_eq!(events[0].job_id, Some("job-a".to_string()));
        Ok(())
    }

    #[tokio::test]
    async fn fr_model_session_id() -> Result<(), Box<dyn std::error::Error>> {
        let recorder = DuckDbFlightRecorder::new_in_memory(7)?;
        let trace_id = Uuid::new_v4();

        recorder
            .record_event(test_event(trace_id, Some("job-123")).with_model_session_id("session-x"))
            .await?;

        let events = recorder
            .list_events(EventFilter {
                model_session_id: Some("session-x".to_string()),
                ..Default::default()
            })
            .await?;

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].model_session_id, Some("session-x".to_string()));
        Ok(())
    }

    #[tokio::test]
    async fn test_query_by_job_id() -> Result<(), Box<dyn std::error::Error>> {
        let recorder = DuckDbFlightRecorder::new_in_memory(7)?;

        let trace_id = Uuid::new_v4();
        recorder
            .record_event(test_event(trace_id, Some("target-job")))
            .await?;
        recorder
            .record_event(test_event(Uuid::new_v4(), Some("other-job")))
            .await?;

        let events = recorder
            .list_events(EventFilter {
                job_id: Some("target-job".to_string()),
                ..Default::default()
            })
            .await?;

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].job_id, Some("target-job".to_string()));
        Ok(())
    }

    #[tokio::test]
    async fn test_retention_purges_old_events() -> Result<(), Box<dyn std::error::Error>> {
        let recorder = DuckDbFlightRecorder::new_in_memory(0)?;

        recorder
            .record_event(test_event(Uuid::new_v4(), None))
            .await?;

        let before = recorder.list_events(EventFilter::default()).await?;
        assert_eq!(before.len(), 1);

        let purged = recorder.enforce_retention().await?;
        assert_eq!(purged, 1);

        let after = recorder.list_events(EventFilter::default()).await?;
        assert_eq!(after.len(), 0);
        Ok(())
    }

    #[tokio::test]
    async fn test_nfc_normalization_applied() -> Result<(), Box<dyn std::error::Error>> {
        let recorder = DuckDbFlightRecorder::new_in_memory(7)?;
        let trace_id = Uuid::new_v4();

        let nfd_string = "cafe\u{0301}";
        let nfc_string = "caf\u{00E9}";

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

        recorder.record_event(event).await?;

        let events = recorder
            .list_events(EventFilter {
                trace_id: Some(trace_id),
                ..Default::default()
            })
            .await?;

        assert_eq!(events.len(), 1);

        let payload = &events[0].payload;
        assert_eq!(payload["message"].as_str().unwrap_or_default(), nfc_string);
        assert_eq!(
            payload["nested"]["text"].as_str().unwrap_or_default(),
            nfc_string
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_validation_rejects_nil_uuid() -> Result<(), Box<dyn std::error::Error>> {
        let recorder = DuckDbFlightRecorder::new_in_memory(7)?;

        let mut event = test_event(Uuid::new_v4(), None);
        event.event_id = Uuid::nil();

        let result = recorder.record_event(event).await;
        assert!(matches!(result, Err(RecorderError::InvalidEvent(_))));
        Ok(())
    }

    #[tokio::test]
    async fn test_validation_rejects_nil_trace_id() -> Result<(), Box<dyn std::error::Error>> {
        let recorder = DuckDbFlightRecorder::new_in_memory(7)?;

        let mut event = test_event(Uuid::new_v4(), None);
        event.trace_id = Uuid::nil();

        let result = recorder.record_event(event).await;
        assert!(matches!(result, Err(RecorderError::InvalidEvent(_))));
        Ok(())
    }

    #[tokio::test]
    async fn test_fr_evt_shapes_persisted() -> Result<(), Box<dyn std::error::Error>> {
        let recorder = DuckDbFlightRecorder::new_in_memory(7)?;
        let trace_id = Uuid::new_v4();

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

        recorder.record_event(system_event).await?;

        let llm_event = FlightRecorderEvent::new(
            FlightRecorderEventType::LlmInference,
            FlightRecorderActor::Agent,
            trace_id,
            json!({
                "type": "llm_inference",
                "trace_id": trace_id.to_string(),
                "model_id": "llama3",
                "token_usage": {
                    "prompt_tokens": 150,
                    "completion_tokens": 50,
                    "total_tokens": 200
                },
                "latency_ms": 1200,
                "prompt_hash": null,
                "response_hash": null
            }),
        )
        .with_model_id("llama3")
        .with_job_id("job-456");

        recorder.record_event(llm_event).await?;

        let events = recorder
            .list_events(EventFilter {
                trace_id: Some(trace_id),
                ..Default::default()
            })
            .await?;

        assert_eq!(events.len(), 2);

        let system = events
            .iter()
            .find(|e| matches!(e.event_type, FlightRecorderEventType::System));
        assert!(system.is_some());
        if let Some(system_event) = system {
            assert_eq!(system_event.payload["component"], "workflow_engine");
            assert_eq!(system_event.workflow_id, Some("wf-123".to_string()));
        }

        let llm = events
            .iter()
            .find(|e| matches!(e.event_type, FlightRecorderEventType::LlmInference));
        assert!(llm.is_some());
        if let Some(llm_event) = llm {
            assert_eq!(llm_event.payload["model_id"], "llama3");
            assert_eq!(llm_event.payload["type"], "llm_inference");
            assert_eq!(llm_event.payload["trace_id"], trace_id.to_string());
            assert_eq!(llm_event.payload["token_usage"]["prompt_tokens"], 150);
            assert_eq!(llm_event.model_id, Some("llama3".to_string()));
            assert_eq!(llm_event.job_id, Some("job-456".to_string()));
        }

        Ok(())
    }

    #[tokio::test]
    async fn flight_recorder_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let recorder = DuckDbFlightRecorder::new_in_memory(7)?;
        let trace_id = Uuid::new_v4();
        let expected_payload = json!({
            "type": "session_scheduler.enqueue",
            "event_id": "FR-EVT-SESS-SCHED-001",
            "session_id": "session-roundtrip",
            "job_id": "job-roundtrip",
            "job_kind": "model_run",
            "lane": "PRIMARY",
            "priority": 10,
            "concurrency_group": null,
            "queue_depth": 0,
            "attempt": 1,
            "max_retries": 3,
            "retry_backoff": "fixed",
            "cancellation_token": null
        });

        let event = FlightRecorderEvent::new(
            FlightRecorderEventType::SessionSchedulerEnqueue,
            FlightRecorderActor::Agent,
            trace_id,
            expected_payload,
        )
        .with_actor_id("agent")
        .with_job_id("job-roundtrip")
        .with_workflow_id("workflow-roundtrip")
        .with_model_id("model-roundtrip")
        .with_model_session_id("session-roundtrip")
        .with_wsids(vec!["ws-1".to_string(), "ws-2".to_string()])
        .with_activity_span("activity-roundtrip")
        .with_session_span("session-span-roundtrip")
        .with_capability("capability-roundtrip")
        .with_policy_decision("policy-roundtrip");

        recorder.record_event(event).await?;

        let events = recorder
            .list_events(EventFilter {
                model_session_id: Some("session-roundtrip".to_string()),
                ..Default::default()
            })
            .await?;

        assert_eq!(events.len(), 1);
        let stored = &events[0];
        assert_eq!(stored.actor, FlightRecorderActor::Agent);
        assert_eq!(stored.actor_id, "agent".to_string());
        assert_eq!(
            stored.event_type,
            FlightRecorderEventType::SessionSchedulerEnqueue
        );
        assert_eq!(stored.trace_id, trace_id);
        assert_eq!(stored.job_id, Some("job-roundtrip".to_string()));
        assert_eq!(stored.workflow_id, Some("workflow-roundtrip".to_string()));
        assert_eq!(stored.model_id, Some("model-roundtrip".to_string()));
        assert_eq!(
            stored.model_session_id,
            Some("session-roundtrip".to_string())
        );
        assert_eq!(
            stored.activity_span_id,
            Some("activity-roundtrip".to_string())
        );
        assert_eq!(
            stored.session_span_id,
            Some("session-span-roundtrip".to_string())
        );
        assert_eq!(
            stored.capability_id,
            Some("capability-roundtrip".to_string())
        );
        assert_eq!(
            stored.policy_decision_id,
            Some("policy-roundtrip".to_string())
        );
        assert_eq!(stored.wsids, vec!["ws-1".to_string(), "ws-2".to_string()]);
        assert_eq!(stored.payload["scheduler"], "unit-test");
        assert_eq!(stored.payload["session_id"], "session-roundtrip");
        Ok(())
    }

    #[tokio::test]
    async fn test_record_diagnostic_and_group() -> Result<(), Box<dyn std::error::Error>> {
        let recorder = DuckDbFlightRecorder::new_in_memory(7)?;
        let job_id = Uuid::new_v4();

        let diagnostic = DiagnosticInput {
            title: "Compiler error".to_string(),
            message: "Missing semicolon".to_string(),
            severity: DiagnosticSeverity::Error,
            source: DiagnosticSource::Validator,
            surface: DiagnosticSurface::Monaco,
            tool: Some("rustc".to_string()),
            code: Some("E001".to_string()),
            tags: None,
            wsid: Some("ws-1".to_string()),
            job_id: Some(job_id.to_string()),
            model_id: None,
            actor: Some(DiagnosticActor::System),
            capability_id: None,
            policy_decision_id: None,
            locations: None,
            evidence_refs: None,
            link_confidence: LinkConfidence::Direct,
            status: None,
            count: None,
            first_seen: None,
            last_seen: None,
            timestamp: None,
            updated_at: None,
        }
        .into_diagnostic()?;

        recorder.record_diagnostic(diagnostic.clone()).await?;

        let fetched = recorder.get_diagnostic(diagnostic.id).await?;
        assert_eq!(fetched.fingerprint, diagnostic.fingerprint);
        assert_eq!(fetched.job_id, diagnostic.job_id);

        let problems = recorder.list_problems(DiagFilter::default()).await?;
        assert_eq!(problems.len(), 1);
        assert_eq!(problems[0].count, 1);

        let events = recorder
            .list_events(EventFilter {
                job_id: diagnostic.job_id.clone(),
                ..Default::default()
            })
            .await?;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, FlightRecorderEventType::Diagnostic);
        Ok(())
    }
}

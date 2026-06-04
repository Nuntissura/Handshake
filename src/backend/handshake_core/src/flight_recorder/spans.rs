//! WP-KERNEL-004 cluster X.4 (MT-196..MT-200) Flight Recorder Span + Aggregate.
//!
//! Submodules-by-section in this file:
//!  - `ModelSessionSpan` + `ActivitySpan` types (MT-196)
//!  - Span Postgres binding (MT-197) — schema lives in
//!    `migrations/0024_session_checkpoint.sql`
//!  - FR-EVT-* event registry (MT-198) — see `fr_event_registry.rs` sibling
//!  - `SpanFrEmitter` lifecycle hooks (MT-199)
//!  - `SessionAggregateQueries` multi-session visibility surface (MT-200)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::future::Future;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use uuid::Uuid;

use super::fr_event_registry::FrEventId;

pub const SPAN_ATTRIBUTE_MAX_COUNT: usize = 16;
pub const SPAN_ATTRIBUTE_MAX_BYTES: usize = 1024;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum AttributeValue {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SpanStatus {
    Active,
    Completed,
    Failed { reason: String },
}

impl SpanStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Completed => "completed",
            Self::Failed { .. } => "failed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpanId(pub Uuid);

impl SpanId {
    pub fn new_v7() -> Self {
        Self(Uuid::now_v7())
    }
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelSessionSpan {
    pub span_id: SpanId,
    pub model_session_id: Uuid,
    pub session_id: Uuid,
    pub started_at_utc: DateTime<Utc>,
    pub ended_at_utc: Option<DateTime<Utc>>,
    pub attributes: BTreeMap<String, AttributeValue>,
    pub status: SpanStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SpanError {
    #[error("activity span requires a task-local parent SpanContext")]
    MissingParentSpanContext,
    #[error("span has too many attributes: {count} > {max}")]
    TooManyAttributes { count: usize, max: usize },
    #[error("span attribute `{key}` is too large: {size} bytes > {max} bytes")]
    AttributeTooLarge {
        key: String,
        size: usize,
        max: usize,
    },
}

pub fn validate_span_attributes(
    attributes: &BTreeMap<String, AttributeValue>,
) -> Result<(), SpanError> {
    if attributes.len() > SPAN_ATTRIBUTE_MAX_COUNT {
        return Err(SpanError::TooManyAttributes {
            count: attributes.len(),
            max: SPAN_ATTRIBUTE_MAX_COUNT,
        });
    }

    for (key, value) in attributes {
        if key.len() > SPAN_ATTRIBUTE_MAX_BYTES {
            return Err(SpanError::AttributeTooLarge {
                key: key.clone(),
                size: key.len(),
                max: SPAN_ATTRIBUTE_MAX_BYTES,
            });
        }

        if let AttributeValue::String(value) = value {
            if value.len() > SPAN_ATTRIBUTE_MAX_BYTES {
                return Err(SpanError::AttributeTooLarge {
                    key: key.clone(),
                    size: value.len(),
                    max: SPAN_ATTRIBUTE_MAX_BYTES,
                });
            }
        }
    }

    Ok(())
}

tokio::task_local! {
    static CURRENT_PARENT_SPAN_ID: SpanId;
}

pub struct SpanContext;

impl SpanContext {
    pub async fn scope<F>(parent_span_id: SpanId, future: F) -> F::Output
    where
        F: Future,
    {
        CURRENT_PARENT_SPAN_ID.scope(parent_span_id, future).await
    }

    pub fn current_parent_span_id() -> Option<SpanId> {
        CURRENT_PARENT_SPAN_ID.try_with(|span_id| *span_id).ok()
    }

    pub fn require_parent_span_id() -> Result<SpanId, SpanError> {
        Self::current_parent_span_id().ok_or(SpanError::MissingParentSpanContext)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "label", rename_all = "snake_case")]
pub enum ActivityKind {
    MtIteration,
    MailboxLease,
    ModelSwap,
    CheckpointWrite,
    StateReplay,
    ToolInvocation,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActivitySpan {
    pub span_id: SpanId,
    pub parent_span_id: SpanId,
    pub activity_kind: ActivityKind,
    pub started_at_utc: DateTime<Utc>,
    pub ended_at_utc: Option<DateTime<Utc>>,
    pub attributes: BTreeMap<String, AttributeValue>,
    pub status: SpanStatus,
}

impl ActivitySpan {
    pub fn start(
        kind: ActivityKind,
        attributes: BTreeMap<String, AttributeValue>,
        recorder: Arc<dyn FrSpanRecorder>,
    ) -> Result<(SpanGuard, Self), SpanError> {
        let parent = SpanContext::require_parent_span_id()?;
        SpanGuard::try_start_activity(parent, kind, attributes, recorder)
    }
}

// ----- FR emitter -----

pub trait FrSpanRecorder: Send + Sync {
    fn record(&self, event_id: FrEventId, payload: serde_json::Value);
}

pub struct StubFrRecorder {
    pub events: Arc<Mutex<Vec<(FrEventId, serde_json::Value)>>>,
}

impl StubFrRecorder {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Default for StubFrRecorder {
    fn default() -> Self {
        Self::new()
    }
}

impl FrSpanRecorder for StubFrRecorder {
    fn record(&self, event_id: FrEventId, payload: serde_json::Value) {
        self.events.lock().unwrap().push((event_id, payload));
    }
}

/// SpanGuard: RAII wrapper. On Drop emits the appropriate SpanEnded/SpanFailed
/// event via the configured `FrSpanRecorder`.
pub struct SpanGuard {
    span_id: SpanId,
    parent_span_id: Option<SpanId>,
    started_at_utc: DateTime<Utc>,
    pub status: SpanStatus,
    recorder: Arc<dyn FrSpanRecorder>,
    is_session_span: bool,
}

impl SpanGuard {
    pub fn start_session(
        model_session_id: Uuid,
        session_id: Uuid,
        attributes: BTreeMap<String, AttributeValue>,
        recorder: Arc<dyn FrSpanRecorder>,
    ) -> (Self, ModelSessionSpan) {
        Self::try_start_session(model_session_id, session_id, attributes, recorder)
            .expect("span attributes must satisfy MT-196 limits")
    }

    pub fn try_start_session(
        model_session_id: Uuid,
        session_id: Uuid,
        attributes: BTreeMap<String, AttributeValue>,
        recorder: Arc<dyn FrSpanRecorder>,
    ) -> Result<(Self, ModelSessionSpan), SpanError> {
        validate_span_attributes(&attributes)?;
        let span_id = SpanId::new_v7();
        let started_at_utc = Utc::now();
        recorder.record(
            FrEventId::SpanStarted,
            serde_json::json!({
                "span_id": span_id.as_uuid(),
                "model_session_id": model_session_id,
                "session_id": session_id,
                "attributes": &attributes,
            }),
        );
        let span = ModelSessionSpan {
            span_id,
            model_session_id,
            session_id,
            started_at_utc,
            ended_at_utc: None,
            attributes,
            status: SpanStatus::Active,
        };
        let guard = Self {
            span_id,
            parent_span_id: None,
            started_at_utc,
            status: SpanStatus::Active,
            recorder,
            is_session_span: true,
        };
        Ok((guard, span))
    }

    pub fn start_activity(
        parent: SpanId,
        kind: ActivityKind,
        attributes: BTreeMap<String, AttributeValue>,
        recorder: Arc<dyn FrSpanRecorder>,
    ) -> (Self, ActivitySpan) {
        Self::try_start_activity(parent, kind, attributes, recorder)
            .expect("span attributes must satisfy MT-196 limits")
    }

    pub fn try_start_activity(
        parent: SpanId,
        kind: ActivityKind,
        attributes: BTreeMap<String, AttributeValue>,
        recorder: Arc<dyn FrSpanRecorder>,
    ) -> Result<(Self, ActivitySpan), SpanError> {
        validate_span_attributes(&attributes)?;
        let span_id = SpanId::new_v7();
        let started_at_utc = Utc::now();
        recorder.record(
            FrEventId::SpanStarted,
            serde_json::json!({
                "span_id": span_id.as_uuid(),
                "parent_span_id": parent.as_uuid(),
                "activity_kind": &kind,
                "attributes": &attributes,
            }),
        );
        let span = ActivitySpan {
            span_id,
            parent_span_id: parent,
            activity_kind: kind,
            started_at_utc,
            ended_at_utc: None,
            attributes,
            status: SpanStatus::Active,
        };
        let guard = Self {
            span_id,
            parent_span_id: Some(parent),
            started_at_utc,
            status: SpanStatus::Active,
            recorder,
            is_session_span: false,
        };
        Ok((guard, span))
    }

    pub fn fail(&mut self, reason: impl Into<String>) {
        self.status = SpanStatus::Failed {
            reason: reason.into(),
        };
    }
}

impl Drop for SpanGuard {
    fn drop(&mut self) {
        let now = Utc::now();
        let duration_ms = (now - self.started_at_utc).num_milliseconds();
        let (event_id, status_str) = match &self.status {
            SpanStatus::Failed { reason } => (FrEventId::SpanFailed, Some(reason.clone())),
            _ => (FrEventId::SpanEnded, None),
        };
        let payload = serde_json::json!({
            "span_id": self.span_id.as_uuid(),
            "parent_span_id": self.parent_span_id.map(|s| s.as_uuid()),
            "duration_ms": duration_ms,
            "is_session_span": self.is_session_span,
            "failure_reason": status_str,
        });
        self.recorder.record(event_id, payload);
    }
}

// ----- Multi-session aggregate (MT-200) query surface -----

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct ActivityRow {
    pub span_id: Uuid,
    pub parent_span_id: Option<Uuid>,
    pub model_session_id: Uuid,
    pub session_id: Uuid,
    pub activity_kind: String,
    pub started_at_utc: DateTime<Utc>,
    pub ended_at_utc: Option<DateTime<Utc>>,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct SessionSummary {
    pub session_id: Uuid,
    pub model_session_id: Uuid,
    pub wp_id: Option<String>,
    pub started_at_utc: DateTime<Utc>,
    pub ended_at_utc: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct SpanLatencyRow {
    pub span_id: Uuid,
    pub session_id: Uuid,
    pub activity_kind: String,
    pub duration_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwarmSnapshot {
    pub active_sessions: u32,
    pub active_leases: u32,
    pub in_flight_micro_tasks: u32,
    pub pending_mailbox_messages: u32,
    pub captured_at_utc: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct SessionTimelineEntry {
    pub kind: String,
    pub at_utc: DateTime<Utc>,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionTimeline {
    pub session_id: Uuid,
    pub entries: Vec<SessionTimelineEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Limit(u16);

impl Limit {
    pub const DEFAULT: u16 = 100;
    pub const MAX: u16 = 1000;

    pub fn new(value: usize) -> Self {
        Self(value.min(Self::MAX as usize) as u16)
    }

    pub fn as_usize(self) -> usize {
        self.0 as usize
    }
}

impl Default for Limit {
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub struct Offset(u32);

impl Offset {
    pub fn new(value: usize) -> Self {
        Self(value.min(u32::MAX as usize) as u32)
    }

    pub fn as_usize(self) -> usize {
        self.0 as usize
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AggregateQueryFixture {
    pub sessions: Vec<SessionSummary>,
    pub activities: Vec<ActivityRow>,
    pub timeline_entries: Vec<(Uuid, SessionTimelineEntry)>,
    pub active_leases: u32,
    pub in_flight_micro_tasks: u32,
    pub pending_mailbox_messages: u32,
}

#[derive(Debug, Error)]
pub enum AggregateQueryError {
    #[error("query time range is invalid: from_utc is after to_utc")]
    InvalidTimeRange,
    #[error("postgres aggregate query failed: {0}")]
    Postgres(#[from] sqlx::Error),
    #[error("aggregate query count overflow: {field}={value}")]
    CountOverflow { field: &'static str, value: i64 },
}

#[derive(Debug, Clone)]
pub struct SessionAggregateQueries {
    backend: AggregateQueryBackend,
}

#[derive(Debug, Clone)]
enum AggregateQueryBackend {
    Fixture(AggregateQueryFixture),
    Postgres(sqlx::PgPool),
}

impl SessionAggregateQueries {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self {
            backend: AggregateQueryBackend::Postgres(pool),
        }
    }

    pub fn from_fixture(fixture: AggregateQueryFixture) -> Self {
        Self {
            backend: AggregateQueryBackend::Fixture(fixture),
        }
    }

    pub async fn activity_for_model_session(
        &self,
        model_session_id: Uuid,
        from_utc: DateTime<Utc>,
        to_utc: DateTime<Utc>,
        limit: Limit,
    ) -> Result<Vec<ActivityRow>, AggregateQueryError> {
        self.activity_for_model_session_page(
            model_session_id,
            from_utc,
            to_utc,
            Offset::default(),
            limit,
        )
        .await
    }

    pub async fn activity_for_model_session_page(
        &self,
        model_session_id: Uuid,
        from_utc: DateTime<Utc>,
        to_utc: DateTime<Utc>,
        offset: Offset,
        limit: Limit,
    ) -> Result<Vec<ActivityRow>, AggregateQueryError> {
        validate_range(from_utc, to_utc)?;
        match &self.backend {
            AggregateQueryBackend::Fixture(fixture) => {
                let mut rows: Vec<_> = fixture
                    .activities
                    .iter()
                    .filter(|row| row.model_session_id == model_session_id)
                    .filter(|row| in_range(row.started_at_utc, from_utc, to_utc))
                    .cloned()
                    .collect();
                rows.sort_by_key(|row| (row.started_at_utc, row.span_id));
                rows = rows
                    .into_iter()
                    .skip(offset.as_usize())
                    .take(limit.as_usize())
                    .collect();
                Ok(rows)
            }
            AggregateQueryBackend::Postgres(pool) => sqlx::query_as::<_, ActivityRow>(
                r#"SELECT
                       a.span_id,
                       a.parent_span_id,
                       s.model_session_id,
                       s.session_id,
                       a.activity_kind,
                       a.started_at_utc,
                       a.ended_at_utc,
                       a.status
                   FROM kernel_activity_span a
                   INNER JOIN kernel_model_session_span s
                       ON s.span_id = a.parent_span_id
                   WHERE s.model_session_id = $1
                     AND a.started_at_utc >= $2
                     AND a.started_at_utc <= $3
                   ORDER BY a.started_at_utc ASC, a.span_id ASC
                   LIMIT $4
                   OFFSET $5"#,
            )
            .bind(model_session_id)
            .bind(from_utc)
            .bind(to_utc)
            .bind(limit.as_usize() as i64)
            .bind(offset.as_usize() as i64)
            .fetch_all(pool)
            .await
            .map_err(AggregateQueryError::from),
        }
    }

    pub async fn sessions_touching_wp(
        &self,
        wp_id: &str,
        from_utc: DateTime<Utc>,
        to_utc: DateTime<Utc>,
        limit: Limit,
    ) -> Result<Vec<SessionSummary>, AggregateQueryError> {
        self.sessions_touching_wp_page(wp_id, from_utc, to_utc, Offset::default(), limit)
            .await
    }

    pub async fn sessions_touching_wp_page(
        &self,
        wp_id: &str,
        from_utc: DateTime<Utc>,
        to_utc: DateTime<Utc>,
        offset: Offset,
        limit: Limit,
    ) -> Result<Vec<SessionSummary>, AggregateQueryError> {
        validate_range(from_utc, to_utc)?;
        match &self.backend {
            AggregateQueryBackend::Fixture(fixture) => {
                let mut rows: Vec<_> = fixture
                    .sessions
                    .iter()
                    .filter(|row| row.wp_id.as_deref() == Some(wp_id))
                    .filter(|row| session_overlaps(row, from_utc, to_utc))
                    .cloned()
                    .collect();
                rows.sort_by_key(|row| (row.started_at_utc, row.session_id));
                rows = rows
                    .into_iter()
                    .skip(offset.as_usize())
                    .take(limit.as_usize())
                    .collect();
                Ok(rows)
            }
            AggregateQueryBackend::Postgres(pool) => sqlx::query_as::<_, SessionSummary>(
                r#"SELECT
                       s.session_id,
                       s.model_session_id,
                       j.wp_id AS wp_id,
                       MIN(s.started_at_utc) AS started_at_utc,
                       MAX(s.ended_at_utc) AS ended_at_utc
                   FROM kernel_micro_task_job j
                   INNER JOIN kernel_model_session_span s
                       ON s.session_id = j.claimed_by_session
                   WHERE j.wp_id = $1
                     AND s.started_at_utc <= $3
                     AND COALESCE(s.ended_at_utc, $3) >= $2
                   GROUP BY s.session_id, s.model_session_id, j.wp_id
                   ORDER BY MIN(s.started_at_utc) ASC, s.session_id ASC
                   LIMIT $4
                   OFFSET $5"#,
            )
            .bind(wp_id)
            .bind(from_utc)
            .bind(to_utc)
            .bind(limit.as_usize() as i64)
            .bind(offset.as_usize() as i64)
            .fetch_all(pool)
            .await
            .map_err(AggregateQueryError::from),
        }
    }

    pub async fn slowest_spans_by_activity_kind(
        &self,
        activity_kind: &str,
        limit: Limit,
    ) -> Result<Vec<SpanLatencyRow>, AggregateQueryError> {
        self.slowest_spans_by_activity_kind_page(activity_kind, Offset::default(), limit)
            .await
    }

    pub async fn slowest_spans_by_activity_kind_page(
        &self,
        activity_kind: &str,
        offset: Offset,
        limit: Limit,
    ) -> Result<Vec<SpanLatencyRow>, AggregateQueryError> {
        match &self.backend {
            AggregateQueryBackend::Fixture(fixture) => {
                let mut rows: Vec<_> = fixture
                    .activities
                    .iter()
                    .filter(|row| row.activity_kind == activity_kind)
                    .filter_map(|row| {
                        let ended_at_utc = row.ended_at_utc?;
                        Some(SpanLatencyRow {
                            span_id: row.span_id,
                            session_id: row.session_id,
                            activity_kind: row.activity_kind.clone(),
                            duration_ms: (ended_at_utc - row.started_at_utc).num_milliseconds(),
                        })
                    })
                    .collect();
                rows.sort_by(|a, b| {
                    b.duration_ms
                        .cmp(&a.duration_ms)
                        .then_with(|| a.span_id.cmp(&b.span_id))
                });
                rows = rows
                    .into_iter()
                    .skip(offset.as_usize())
                    .take(limit.as_usize())
                    .collect();
                Ok(rows)
            }
            AggregateQueryBackend::Postgres(pool) => sqlx::query_as::<_, SpanLatencyRow>(
                r#"SELECT
                       a.span_id,
                       s.session_id,
                       a.activity_kind,
                       ((EXTRACT(EPOCH FROM (a.ended_at_utc - a.started_at_utc)) * 1000)::BIGINT)
                           AS duration_ms
                   FROM kernel_activity_span a
                   INNER JOIN kernel_model_session_span s
                       ON s.span_id = a.parent_span_id
                   WHERE a.activity_kind = $1
                     AND a.ended_at_utc IS NOT NULL
                   ORDER BY duration_ms DESC, a.span_id ASC
                   LIMIT $2
                   OFFSET $3"#,
            )
            .bind(activity_kind)
            .bind(limit.as_usize() as i64)
            .bind(offset.as_usize() as i64)
            .fetch_all(pool)
            .await
            .map_err(AggregateQueryError::from),
        }
    }

    pub async fn session_timeline(
        &self,
        session_id: Uuid,
        from_utc: DateTime<Utc>,
        to_utc: DateTime<Utc>,
        limit: Limit,
    ) -> Result<SessionTimeline, AggregateQueryError> {
        self.session_timeline_page(session_id, from_utc, to_utc, Offset::default(), limit)
            .await
    }

    pub async fn session_timeline_page(
        &self,
        session_id: Uuid,
        from_utc: DateTime<Utc>,
        to_utc: DateTime<Utc>,
        offset: Offset,
        limit: Limit,
    ) -> Result<SessionTimeline, AggregateQueryError> {
        validate_range(from_utc, to_utc)?;
        match &self.backend {
            AggregateQueryBackend::Fixture(fixture) => {
                let mut entries: Vec<_> = fixture
                    .timeline_entries
                    .iter()
                    .filter(|(entry_session_id, _)| *entry_session_id == session_id)
                    .map(|(_, entry)| entry)
                    .filter(|entry| in_range(entry.at_utc, from_utc, to_utc))
                    .cloned()
                    .collect();
                entries.sort_by_key(|entry| entry.at_utc);
                entries = entries
                    .into_iter()
                    .skip(offset.as_usize())
                    .take(limit.as_usize())
                    .collect();
                Ok(SessionTimeline {
                    session_id,
                    entries,
                })
            }
            AggregateQueryBackend::Postgres(pool) => {
                let entries = sqlx::query_as::<_, SessionTimelineEntry>(
                    r#"SELECT kind, at_utc, summary
                       FROM (
                         SELECT
                           'event'::TEXT AS kind,
                           e.created_at AT TIME ZONE 'UTC' AS at_utc,
                           e.event_type::TEXT AS summary
                         FROM kernel_event_ledger e
                         WHERE e.session_run_id = ($1::UUID)::TEXT
                           AND e.created_at AT TIME ZONE 'UTC' >= $2
                           AND e.created_at AT TIME ZONE 'UTC' <= $3
                         UNION ALL
                         SELECT
                           'span'::TEXT AS kind,
                           a.started_at_utc AS at_utc,
                           a.activity_kind::TEXT AS summary
                         FROM kernel_activity_span a
                         INNER JOIN kernel_model_session_span s
                           ON s.span_id = a.parent_span_id
                         WHERE s.session_id = $1
                           AND a.started_at_utc >= $2
                           AND a.started_at_utc <= $3
                         UNION ALL
                         SELECT
                           'checkpoint'::TEXT AS kind,
                           c.created_at_utc AS at_utc,
                           CONCAT(c.state_kind, ':', c.last_event_ledger_seq)::TEXT AS summary
                         FROM kernel_session_checkpoint c
                         WHERE c.session_id = $1
                           AND c.created_at_utc >= $2
                           AND c.created_at_utc <= $3
                         UNION ALL
                         SELECT
                           'mailbox_message'::TEXT AS kind,
                           m.created_at_utc AS at_utc,
                           m.message_type::TEXT AS summary
                         FROM kernel_micro_task_job j
                         INNER JOIN role_mailbox_message m
                           ON m.thread_id = j.mailbox_thread_id
                         WHERE j.claimed_by_session = $1
                           AND m.created_at_utc >= $2
                           AND m.created_at_utc <= $3
                         UNION ALL
                         SELECT
                           'mt_outcome'::TEXT AS kind,
                           o.recorded_at_utc AS at_utc,
                           o.outcome_kind::TEXT AS summary
                         FROM kernel_micro_task_job j
                         INNER JOIN kernel_mt_outcome o
                           ON o.job_id = j.job_id
                         WHERE j.claimed_by_session = $1
                           AND o.recorded_at_utc >= $2
                           AND o.recorded_at_utc <= $3
                       ) timeline
                       ORDER BY at_utc ASC, kind ASC, summary ASC
                       LIMIT $4
                       OFFSET $5"#,
                )
                .bind(session_id)
                .bind(from_utc)
                .bind(to_utc)
                .bind(limit.as_usize() as i64)
                .bind(offset.as_usize() as i64)
                .fetch_all(pool)
                .await?;

                Ok(SessionTimeline {
                    session_id,
                    entries,
                })
            }
        }
    }

    pub async fn swarm_concurrency_snapshot(
        &self,
        now: DateTime<Utc>,
    ) -> Result<SwarmSnapshot, AggregateQueryError> {
        match &self.backend {
            AggregateQueryBackend::Fixture(fixture) => Ok(SwarmSnapshot {
                active_sessions: fixture
                    .sessions
                    .iter()
                    .filter(|session| {
                        session.started_at_utc <= now
                            && session
                                .ended_at_utc
                                .map(|ended| ended > now)
                                .unwrap_or(true)
                    })
                    .count() as u32,
                active_leases: fixture.active_leases,
                in_flight_micro_tasks: fixture.in_flight_micro_tasks,
                pending_mailbox_messages: fixture.pending_mailbox_messages,
                captured_at_utc: now,
            }),
            AggregateQueryBackend::Postgres(pool) => {
                let (active_sessions, active_leases, in_flight_micro_tasks, pending_mailbox_messages) =
                    sqlx::query_as::<_, (i64, i64, i64, i64)>(
                        r#"SELECT
                              (SELECT COUNT(DISTINCT session_id)
                               FROM kernel_model_session_span
                               WHERE started_at_utc <= $1
                                 AND (ended_at_utc IS NULL OR ended_at_utc > $1)) AS active_sessions,
                              (SELECT COUNT(*)
                               FROM role_mailbox_claim_lease
                               WHERE released_at_utc IS NULL
                                 AND expires_at_utc > $1) AS active_leases,
                              (SELECT COUNT(*)
                               FROM kernel_micro_task_job
                               WHERE state IN ('claimed', 'running', 'in_progress', 'blocked')) AS in_flight_micro_tasks,
                              (SELECT COUNT(*)
                               FROM role_mailbox_message
                               WHERE delivery_state IN ('pending', 'queued', 'delivered')) AS pending_mailbox_messages"#,
                    )
                    .bind(now)
                    .fetch_one(pool)
                    .await?;

                Ok(SwarmSnapshot {
                    active_sessions: count_to_u32("active_sessions", active_sessions)?,
                    active_leases: count_to_u32("active_leases", active_leases)?,
                    in_flight_micro_tasks: count_to_u32(
                        "in_flight_micro_tasks",
                        in_flight_micro_tasks,
                    )?,
                    pending_mailbox_messages: count_to_u32(
                        "pending_mailbox_messages",
                        pending_mailbox_messages,
                    )?,
                    captured_at_utc: now,
                })
            }
        }
    }
}

fn count_to_u32(field: &'static str, value: i64) -> Result<u32, AggregateQueryError> {
    u32::try_from(value).map_err(|_| AggregateQueryError::CountOverflow { field, value })
}

fn validate_range(
    from_utc: DateTime<Utc>,
    to_utc: DateTime<Utc>,
) -> Result<(), AggregateQueryError> {
    if from_utc > to_utc {
        return Err(AggregateQueryError::InvalidTimeRange);
    }
    Ok(())
}

fn in_range(at_utc: DateTime<Utc>, from_utc: DateTime<Utc>, to_utc: DateTime<Utc>) -> bool {
    at_utc >= from_utc && at_utc <= to_utc
}

fn session_overlaps(row: &SessionSummary, from_utc: DateTime<Utc>, to_utc: DateTime<Utc>) -> bool {
    row.started_at_utc <= to_utc
        && row
            .ended_at_utc
            .map(|ended| ended >= from_utc)
            .unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_guard_emits_started_and_ended() {
        let recorder = Arc::new(StubFrRecorder::new());
        {
            let (_guard, _span) = SpanGuard::start_session(
                Uuid::now_v7(),
                Uuid::now_v7(),
                BTreeMap::new(),
                Arc::clone(&recorder) as Arc<dyn FrSpanRecorder>,
            );
        }
        let events = recorder.events.lock().unwrap();
        let ids: Vec<FrEventId> = events.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids, vec![FrEventId::SpanStarted, FrEventId::SpanEnded]);
    }

    #[test]
    fn span_guard_failure_emits_failed_event() {
        let recorder = Arc::new(StubFrRecorder::new());
        {
            let (mut guard, _span) = SpanGuard::start_session(
                Uuid::now_v7(),
                Uuid::now_v7(),
                BTreeMap::new(),
                Arc::clone(&recorder) as Arc<dyn FrSpanRecorder>,
            );
            guard.fail("test failure");
        }
        let events = recorder.events.lock().unwrap();
        let ids: Vec<FrEventId> = events.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids, vec![FrEventId::SpanStarted, FrEventId::SpanFailed]);
    }

    #[test]
    fn activity_span_with_parent_emits_correlation() {
        let recorder = Arc::new(StubFrRecorder::new());
        let parent_id = SpanId::new_v7();
        {
            let (_g, _s) = SpanGuard::start_activity(
                parent_id,
                ActivityKind::MtIteration,
                BTreeMap::new(),
                Arc::clone(&recorder) as Arc<dyn FrSpanRecorder>,
            );
        }
        let events = recorder.events.lock().unwrap();
        assert_eq!(events.len(), 2);
        let started_payload = &events[0].1;
        assert_eq!(
            started_payload
                .get("parent_span_id")
                .and_then(|v| v.as_str()),
            Some(parent_id.as_uuid().to_string()).as_deref()
        );
    }

    #[test]
    fn nested_drops_emit_in_correct_order() {
        let recorder = Arc::new(StubFrRecorder::new());
        let session_id = Uuid::now_v7();
        {
            let (session_guard, session_span) = SpanGuard::start_session(
                Uuid::now_v7(),
                session_id,
                BTreeMap::new(),
                Arc::clone(&recorder) as Arc<dyn FrSpanRecorder>,
            );
            {
                let (_a, _) = SpanGuard::start_activity(
                    session_span.span_id,
                    ActivityKind::MtIteration,
                    BTreeMap::new(),
                    Arc::clone(&recorder) as Arc<dyn FrSpanRecorder>,
                );
            }
            drop(session_guard);
        }
        let events = recorder.events.lock().unwrap();
        // Order: SessionSpan Started, ActivitySpan Started, ActivitySpan Ended, SessionSpan Ended.
        assert_eq!(
            events.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
            vec![
                FrEventId::SpanStarted,
                FrEventId::SpanStarted,
                FrEventId::SpanEnded,
                FrEventId::SpanEnded,
            ]
        );
    }
}

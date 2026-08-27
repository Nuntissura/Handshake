//! WP-KERNEL-004 cluster X.4 (MT-196..MT-200) Flight Recorder Span + Aggregate.
//!
//! Submodules-by-section in this file:
//!  - `ModelSessionSpan` + `ActivitySpan` types (MT-196)
//!  - Span store binding (MT-197) — schema lives in
//!    `storage/surreal/schema.surql`
//!  - FR-EVT-* event registry (MT-198) — see `fr_event_registry.rs` sibling
//!  - `SpanFrEmitter` lifecycle hooks (MT-199)
//!  - `SessionAggregateQueries` multi-session visibility surface (MT-200)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::future::Future;
use std::sync::{Arc, Mutex};
use surrealdb::types::{RecordId, SurrealValue};
use thiserror::Error;
use uuid::Uuid;

use super::fr_event_registry::FrEventId;
use crate::storage::surreal::{SurrealStorage, SurrealStorageError};

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SurrealValue)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSummary {
    pub session_id: Uuid,
    pub model_session_id: Uuid,
    pub wp_id: Option<String>,
    pub started_at_utc: DateTime<Utc>,
    pub ended_at_utc: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SurrealValue)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SurrealValue)]
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
    #[error("aggregate query failed: {0}")]
    Storage(#[from] SurrealStorageError),
    #[error("aggregate query count overflow: {field}={value}")]
    CountOverflow { field: &'static str, value: i64 },
}

#[derive(Clone)]
pub struct SessionAggregateQueries {
    backend: AggregateQueryBackend,
}

#[derive(Clone)]
enum AggregateQueryBackend {
    Fixture(AggregateQueryFixture),
    Surreal(SurrealStorage),
}

impl SessionAggregateQueries {
    pub fn new(storage: SurrealStorage) -> Self {
        Self {
            backend: AggregateQueryBackend::Surreal(storage),
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
            AggregateQueryBackend::Surreal(storage) => {
                #[derive(SurrealValue)]
                struct Bindings {
                    model_session_id: Uuid,
                    from_utc: DateTime<Utc>,
                    to_utc: DateTime<Utc>,
                    limit: i64,
                    offset: i64,
                }

                let bindings = Bindings {
                    model_session_id,
                    from_utc,
                    to_utc,
                    limit: limit.as_usize() as i64,
                    offset: offset.as_usize() as i64,
                };
                // The previous SQL join is expressed as record-link traversal
                // through `parent_span_id`.
                storage
                    .with_data_operation(move |database| {
                        Box::pin(async move {
                            database
                                .query_values(
                                    "SELECT span_id, \
                                     record::id(parent_span_id) AS parent_span_id, \
                                     parent_span_id.model_session_id AS model_session_id, \
                                     parent_span_id.session_id AS session_id, \
                                     activity_kind, started_at_utc, ended_at_utc, status \
                                     FROM kernel_activity_span \
                                     WHERE parent_span_id.model_session_id = $model_session_id \
                                       AND started_at_utc >= $from_utc \
                                       AND started_at_utc <= $to_utc \
                                     ORDER BY started_at_utc ASC, span_id ASC \
                                     LIMIT $limit START $offset;",
                                    bindings,
                                )
                                .await
                        })
                    })
                    .await
                    .map_err(AggregateQueryError::from)
            }
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
            AggregateQueryBackend::Surreal(storage) => {
                #[derive(SurrealValue)]
                struct SpanBindings {
                    wp_id: String,
                    from_utc: DateTime<Utc>,
                    to_utc: DateTime<Utc>,
                }

                #[derive(SurrealValue)]
                struct SpanRow {
                    session_id: Uuid,
                    model_session_id: Uuid,
                    started_at_utc: DateTime<Utc>,
                    ended_at_utc: Option<DateTime<Utc>>,
                }

                // The previous single SQL statement joined jobs to spans and
                // grouped server-side. The subquery keeps the wp filter
                // server-side; MIN/MAX grouping with PostgreSQL's
                // NULL-ignoring MAX semantics is reproduced client-side.
                // `COALESCE(ended, $to) >= $from` reduces to
                // `ended = NONE OR ended >= $from` because the validated
                // range guarantees `$to >= $from`.
                let bindings = SpanBindings {
                    wp_id: wp_id.to_string(),
                    from_utc,
                    to_utc,
                };
                let rows: Vec<SpanRow> = storage
                    .with_data_operation(move |database| {
                        Box::pin(async move {
                            database
                                .query_values(
                                    "SELECT session_id, model_session_id, started_at_utc, \
                                     ended_at_utc FROM kernel_model_session_span \
                                     WHERE session_id IN \
                                       (SELECT VALUE claimed_by_session \
                                        FROM kernel_micro_task_job \
                                        WHERE wp_id = $wp_id \
                                          AND claimed_by_session != NONE) \
                                       AND started_at_utc <= $to_utc \
                                       AND (ended_at_utc = NONE OR ended_at_utc >= $from_utc);",
                                    bindings,
                                )
                                .await
                        })
                    })
                    .await?;

                let mut grouped: BTreeMap<(Uuid, Uuid), SessionSummary> = BTreeMap::new();
                for row in rows {
                    let entry = grouped
                        .entry((row.session_id, row.model_session_id))
                        .or_insert_with(|| SessionSummary {
                            session_id: row.session_id,
                            model_session_id: row.model_session_id,
                            wp_id: Some(wp_id.to_string()),
                            started_at_utc: row.started_at_utc,
                            ended_at_utc: None,
                        });
                    entry.started_at_utc = entry.started_at_utc.min(row.started_at_utc);
                    // SQL MAX ignores NULLs: an open span does not clear an
                    // already-observed end time.
                    if let Some(ended) = row.ended_at_utc {
                        entry.ended_at_utc = Some(match entry.ended_at_utc {
                            Some(existing) => existing.max(ended),
                            None => ended,
                        });
                    }
                }
                let mut summaries: Vec<SessionSummary> = grouped.into_values().collect();
                summaries.sort_by_key(|row| (row.started_at_utc, row.session_id));
                Ok(summaries
                    .into_iter()
                    .skip(offset.as_usize())
                    .take(limit.as_usize())
                    .collect())
            }
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
            AggregateQueryBackend::Surreal(storage) => {
                #[derive(SurrealValue)]
                struct Bindings {
                    activity_kind: String,
                    limit: i64,
                    offset: i64,
                }

                let bindings = Bindings {
                    activity_kind: activity_kind.to_string(),
                    limit: limit.as_usize() as i64,
                    offset: offset.as_usize() as i64,
                };
                storage
                    .with_data_operation(move |database| {
                        Box::pin(async move {
                            database
                                .query_values(
                                    "SELECT span_id, \
                                     parent_span_id.session_id AS session_id, \
                                     activity_kind, \
                                     duration::millis(ended_at_utc - started_at_utc) \
                                         AS duration_ms \
                                     FROM kernel_activity_span \
                                     WHERE activity_kind = $activity_kind \
                                       AND ended_at_utc != NONE \
                                     ORDER BY duration_ms DESC, span_id ASC \
                                     LIMIT $limit START $offset;",
                                    bindings,
                                )
                                .await
                        })
                    })
                    .await
                    .map_err(AggregateQueryError::from)
            }
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
            AggregateQueryBackend::Surreal(storage) => {
                // SurrealQL has no UNION, so one explicit transaction reads
                // all five bounded sources from the same database snapshot.
                // The client then merges and pages that snapshot exactly as
                // the historical SQL outer query did. A row past
                // `offset + limit` in any one source cannot reach the final
                // requested page.
                #[derive(SurrealValue)]
                struct SourceBindings {
                    session_id: Uuid,
                    session_run_id: String,
                    from_utc: DateTime<Utc>,
                    to_utc: DateTime<Utc>,
                    cap: i64,
                }

                let bindings = SourceBindings {
                    session_id,
                    session_run_id: session_id.to_string(),
                    from_utc,
                    to_utc,
                    cap: (offset.as_usize() + limit.as_usize()) as i64,
                };

                const TIMELINE_SNAPSHOT: &str = "BEGIN TRANSACTION; \
                    SELECT 'event' AS kind, created_at AS at_utc, event_type AS summary \
                     FROM kernel_event_ledger WHERE session_run_id = $session_run_id \
                       AND created_at >= $from_utc AND created_at <= $to_utc \
                     ORDER BY at_utc ASC, summary ASC LIMIT $cap; \
                    SELECT 'span' AS kind, started_at_utc AS at_utc, activity_kind AS summary \
                     FROM kernel_activity_span \
                     WHERE parent_span_id.session_id = $session_id \
                       AND started_at_utc >= $from_utc AND started_at_utc <= $to_utc \
                     ORDER BY at_utc ASC, summary ASC LIMIT $cap; \
                    SELECT 'checkpoint' AS kind, created_at_utc AS at_utc, \
                     string::concat(state_kind, ':', <string>last_event_ledger_seq) AS summary \
                     FROM kernel_session_checkpoint WHERE session_id = $session_id \
                       AND created_at_utc >= $from_utc AND created_at_utc <= $to_utc \
                     ORDER BY at_utc ASC, summary ASC LIMIT $cap; \
                    SELECT 'mailbox_message' AS kind, created_at_utc AS at_utc, \
                     message_type AS summary FROM role_mailbox_message \
                     WHERE record::id(thread_id) IN \
                       (SELECT VALUE mailbox_thread_id FROM kernel_micro_task_job \
                        WHERE claimed_by_session = $session_id \
                          AND mailbox_thread_id != NONE) \
                       AND created_at_utc >= $from_utc AND created_at_utc <= $to_utc \
                     ORDER BY at_utc ASC, summary ASC LIMIT $cap; \
                    SELECT 'mt_outcome' AS kind, recorded_at_utc AS at_utc, \
                     outcome_kind AS summary FROM kernel_mt_outcome \
                     WHERE job_id.claimed_by_session = $session_id \
                       AND recorded_at_utc >= $from_utc AND recorded_at_utc <= $to_utc \
                     ORDER BY at_utc ASC, summary ASC LIMIT $cap; \
                    COMMIT TRANSACTION;";

                let [events, spans, checkpoints, mailbox_messages, mt_outcomes]: [Vec<
                    SessionTimelineEntry,
                >;
                    5] = storage
                    .with_data_operation(move |database| {
                        Box::pin(async move {
                            database
                                .query_five_values_at(TIMELINE_SNAPSHOT, bindings, [1, 2, 3, 4, 5])
                                .await
                        })
                    })
                    .await?;
                let mut entries = events;
                entries.extend(spans);
                entries.extend(checkpoints);
                entries.extend(mailbox_messages);
                entries.extend(mt_outcomes);
                entries.sort_by(|a, b| {
                    a.at_utc
                        .cmp(&b.at_utc)
                        .then_with(|| a.kind.cmp(&b.kind))
                        .then_with(|| a.summary.cmp(&b.summary))
                });
                let entries = entries
                    .into_iter()
                    .skip(offset.as_usize())
                    .take(limit.as_usize())
                    .collect();

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
            AggregateQueryBackend::Surreal(storage) => {
                #[derive(SurrealValue)]
                struct NowBinding {
                    now: DateTime<Utc>,
                }

                #[derive(SurrealValue)]
                struct SnapshotCounts {
                    active_sessions: i64,
                    active_leases: i64,
                    in_flight_micro_tasks: i64,
                    pending_mailbox_messages: i64,
                }

                // One RETURN statement evaluates all four subqueries in a
                // single statement (one transaction), preserving the
                // single-snapshot property of the previous SQL statement.
                let counts: Option<SnapshotCounts> = storage
                    .with_data_operation(move |database| {
                        Box::pin(async move {
                            database
                                .query_first(
                                    "RETURN { \
                                     active_sessions: array::len(array::distinct(( \
                                         SELECT VALUE session_id FROM kernel_model_session_span \
                                         WHERE started_at_utc <= $now \
                                           AND (ended_at_utc = NONE OR ended_at_utc > $now)))), \
                                     active_leases: array::len(( \
                                         SELECT VALUE lease_id FROM role_mailbox_claim_lease \
                                         WHERE released_at_utc = NONE \
                                           AND expires_at_utc > $now)), \
                                     in_flight_micro_tasks: array::len(( \
                                         SELECT VALUE job_id FROM kernel_micro_task_job \
                                         WHERE state IN \
                                           ['claimed', 'running', 'in_progress', 'blocked'])), \
                                     pending_mailbox_messages: array::len(( \
                                         SELECT VALUE message_id FROM role_mailbox_message \
                                         WHERE delivery_state IN \
                                           ['pending', 'queued', 'delivered'])) \
                                     };",
                                    NowBinding { now },
                                )
                                .await
                        })
                    })
                    .await?;
                let counts = counts.ok_or(AggregateQueryError::CountOverflow {
                    field: "snapshot",
                    value: -1,
                })?;

                Ok(SwarmSnapshot {
                    active_sessions: count_to_u32("active_sessions", counts.active_sessions)?,
                    active_leases: count_to_u32("active_leases", counts.active_leases)?,
                    in_flight_micro_tasks: count_to_u32(
                        "in_flight_micro_tasks",
                        counts.in_flight_micro_tasks,
                    )?,
                    pending_mailbox_messages: count_to_u32(
                        "pending_mailbox_messages",
                        counts.pending_mailbox_messages,
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
    use crate::storage::surreal::{bootstrap_schema, SurrealStorageConfig};

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

    #[tokio::test]
    async fn mt137_timeline_reads_cross_source_pairs_from_one_snapshot() {
        #[derive(SurrealValue)]
        struct PairBindings {
            event_id: String,
            checkpoint_record_id: String,
            checkpoint_id: Uuid,
            session_id: Uuid,
            session_run_id: String,
            model_session_id: Uuid,
            event_type: String,
            state_kind: String,
            at_utc: DateTime<Utc>,
            pair_index: i64,
        }

        #[derive(SurrealValue)]
        struct TieBindings {
            first_event_id: String,
            second_event_id: String,
            session_run_id: String,
            first_summary: String,
            second_summary: String,
            at_utc: DateTime<Utc>,
        }

        fn assert_complete_pairs(timeline: &SessionTimeline) {
            let events: std::collections::BTreeSet<_> = timeline
                .entries
                .iter()
                .filter_map(|entry| entry.summary.strip_prefix("event-").map(str::to_owned))
                .collect();
            let checkpoints: std::collections::BTreeSet<_> = timeline
                .entries
                .iter()
                .filter_map(|entry| {
                    entry
                        .summary
                        .strip_prefix("checkpoint-")
                        .and_then(|value| value.strip_suffix(":0"))
                        .map(str::to_owned)
                })
                .collect();
            assert_eq!(
                events, checkpoints,
                "one timeline read must never mix snapshots across event and checkpoint sources"
            );
        }

        let directory = tempfile::tempdir().expect("temporary timeline snapshot root");
        let storage = SurrealStorage::open(
            SurrealStorageConfig::with_path(&directory.path().join("store"))
                .expect("valid timeline snapshot path"),
        )
        .await
        .expect("open timeline snapshot store");
        bootstrap_schema(&storage)
            .await
            .expect("bootstrap timeline snapshot schema");

        let session_id = Uuid::now_v7();
        let session_run_id = session_id.to_string();
        let model_session_id = Uuid::now_v7();
        let base = Utc::now();
        let queries = SessionAggregateQueries::new(storage.clone());
        let writer_storage = storage.clone();
        let writer_session_run_id = session_run_id.clone();
        let start_barrier = Arc::new(tokio::sync::Barrier::new(2));
        let writer_barrier = Arc::clone(&start_barrier);
        let writer = tokio::spawn(async move {
            writer_barrier.wait().await;
            for pair_index in 0..1_i64 {
                let checkpoint_id = Uuid::now_v7();
                let marker = format!("{pair_index:03}");
                let bindings = PairBindings {
                    event_id: format!("mt137-timeline-{session_id}-{marker}"),
                    checkpoint_record_id: checkpoint_id.to_string(),
                    checkpoint_id,
                    session_id,
                    session_run_id: writer_session_run_id.clone(),
                    model_session_id,
                    event_type: format!("event-{marker}"),
                    state_kind: format!("checkpoint-{marker}"),
                    at_utc: base + chrono::Duration::milliseconds(pair_index),
                    pair_index,
                };
                let applied: Option<bool> = writer_storage
                    .with_data_operation(move |database| {
                        Box::pin(async move {
                            database
                                .query_first(
                                    "RETURN { \
                                     CREATE type::record('kernel_event_ledger', $event_id) CONTENT { \
                                         event_id: $event_id, event_version: 'v1', \
                                         kernel_task_run_id: 'mt137-timeline-proof', \
                                         session_run_id: $session_run_id, aggregate_type: 'session', \
                                         aggregate_id: $session_run_id, idempotency_key: $event_id, \
                                         event_type: $event_type, actor_kind: 'test', actor_id: 'mt137', \
                                         causation_id: NONE, correlation_id: NONE, \
                                         payload_hash: $event_id, source_component: 'mt137-timeline-proof', \
                                         payload: { pair_index: $pair_index }, created_at: $at_utc \
                                     } RETURN NONE; \
                                     CREATE type::record('kernel_session_checkpoint', $checkpoint_record_id) CONTENT { \
                                         checkpoint_id: $checkpoint_id, session_id: $session_id, \
                                         model_session_id: $model_session_id, last_event_ledger_seq: 0, \
                                         compact_state: { pair_index: $pair_index }, state_kind: $state_kind, \
                                         pending_artifacts: [], created_at_utc: $at_utc, \
                                         created_by_process: 1, schema_version: 1 \
                                     } RETURN NONE; \
                                     RETURN true; \
                                     };",
                                    bindings,
                                )
                                .await
                        })
                    })
                    .await
                    .expect("append atomic event/checkpoint pair");
                assert_eq!(applied, Some(true));
                tokio::task::yield_now().await;
            }
        });

        start_barrier.wait().await;
        for _ in 0..1 {
            let timeline = queries
                .session_timeline(
                    session_id,
                    base - chrono::Duration::seconds(1),
                    base + chrono::Duration::minutes(1),
                    Limit::new(1000),
                )
                .await
                .expect("read timeline snapshot while pairs are committed");
            assert_complete_pairs(&timeline);
            if writer.is_finished() {
                break;
            }
            tokio::task::yield_now().await;
        }
        writer.await.expect("timeline pair writer joins");

        let final_timeline = queries
            .session_timeline(
                session_id,
                base - chrono::Duration::seconds(1),
                base + chrono::Duration::minutes(1),
                Limit::new(1000),
            )
            .await
            .expect("read final timeline snapshot");
        assert_complete_pairs(&final_timeline);
        assert_eq!(
            final_timeline
                .entries
                .iter()
                .filter(|entry| entry.kind == "event")
                .count(),
            1
        );
        assert_eq!(
            final_timeline
                .entries
                .iter()
                .filter(|entry| entry.kind == "checkpoint")
                .count(),
            1
        );

        // Per-source caps must preserve the historical global tie-break order
        // before the client applies OFFSET/LIMIT. Otherwise cap=1 could pick
        // either same-timestamp event and make pagination nondeterministic.
        let tie_at = base + chrono::Duration::seconds(10);
        let tie_bindings = TieBindings {
            first_event_id: format!("mt137-timeline-tie-a-{session_id}"),
            second_event_id: format!("mt137-timeline-tie-z-{session_id}"),
            session_run_id: session_run_id.clone(),
            first_summary: "a-tie".to_owned(),
            second_summary: "z-tie".to_owned(),
            at_utc: tie_at,
        };
        let tied_rows_created: Option<bool> = storage
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .query_first(
                            "RETURN { \
                             CREATE type::record('kernel_event_ledger', $first_event_id) CONTENT { \
                                 event_id: $first_event_id, event_version: 'v1', \
                                 kernel_task_run_id: 'mt137-timeline-tie-proof', \
                                 session_run_id: $session_run_id, aggregate_type: 'session', \
                                 aggregate_id: $session_run_id, idempotency_key: $first_event_id, \
                                 event_type: $first_summary, actor_kind: 'test', actor_id: 'mt137', \
                                 causation_id: NONE, correlation_id: NONE, \
                                 payload_hash: $first_event_id, source_component: 'mt137-timeline-tie-proof', \
                                 payload: {}, created_at: $at_utc \
                             } RETURN NONE; \
                             CREATE type::record('kernel_event_ledger', $second_event_id) CONTENT { \
                                 event_id: $second_event_id, event_version: 'v1', \
                                 kernel_task_run_id: 'mt137-timeline-tie-proof', \
                                 session_run_id: $session_run_id, aggregate_type: 'session', \
                                 aggregate_id: $session_run_id, idempotency_key: $second_event_id, \
                                 event_type: $second_summary, actor_kind: 'test', actor_id: 'mt137', \
                                 causation_id: NONE, correlation_id: NONE, \
                                 payload_hash: $second_event_id, source_component: 'mt137-timeline-tie-proof', \
                                 payload: {}, created_at: $at_utc \
                             } RETURN NONE; \
                             RETURN true; \
                             };",
                            tie_bindings,
                        )
                        .await
                })
            })
            .await
            .expect("append same-timestamp tie rows");
        assert_eq!(tied_rows_created, Some(true));

        let first_tie_page = queries
            .session_timeline_page(session_id, tie_at, tie_at, Offset::new(0), Limit::new(1))
            .await
            .expect("read first same-timestamp timeline page");
        assert_eq!(
            first_tie_page
                .entries
                .iter()
                .map(|entry| entry.summary.as_str())
                .collect::<Vec<_>>(),
            vec!["a-tie"]
        );

        let second_tie_page = queries
            .session_timeline_page(session_id, tie_at, tie_at, Offset::new(1), Limit::new(1))
            .await
            .expect("read second same-timestamp timeline page");
        assert_eq!(
            second_tie_page
                .entries
                .iter()
                .map(|entry| entry.summary.as_str())
                .collect::<Vec<_>>(),
            vec!["z-tie"]
        );

        drop(queries);
        storage
            .shutdown()
            .await
            .expect("close timeline snapshot store");
    }
}

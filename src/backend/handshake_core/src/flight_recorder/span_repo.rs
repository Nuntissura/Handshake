//! MT-197 Flight Recorder span repository on the embedded SurrealDB store.
//!
//! Authority: cluster X.4 contract
//! `.GOV/task_packets/WP-KERNEL-004-.../MT-197.json`.
//!
//! Folded WP-1-Session-Observability-Spans-FR-v1 invariant:
//! "session-wide queries via model_session_id work even without spans"
//! — implemented here by filtering `kernel_model_session_span` on
//! `model_session_id` in
//! [`SpanRepo::query_session_spans_for_model_session_id`].
//!
//! Authority-files alignment: MT-197 `expected_diff_shape` referenced
//! `src/backend/handshake_core/src/observability/span_repo.rs`; the
//! cluster already landed observability under `flight_recorder::spans`
//! via MT-196 deviation, so this MT extends the same module per MT-196
//! `contract_deviation_note`. Schema lives in
//! `storage/surreal/schema.surql` (`kernel_model_session_span` +
//! `kernel_activity_span`, including the immutability EVENT guards that
//! replaced the 0025 triggers).
//!
//! Write model:
//!   - CREATE a new session span -> `insert_session_span`.
//!   - CREATE a new activity span (record-link parent) -> `insert_activity_span`.
//!   - End a span (status + ended_at_utc + optional ledger watermark)
//!     -> `update_session_span_end` / `update_activity_span_end`.
//!   - Append an event ledger seq to an activity span's
//!     `related_event_ledger_seqs` -> `attach_event_ledger_seq`.
//!
//! Attributes and other immutable fields are enforced by the schema
//! `DEFINE EVENT` guards in schema.surql; the Rust API has no method to
//! update them on purpose.
//!
//! # Porting notes (PostgreSQL -> embedded SurrealDB)
//!
//! The transactional `SELECT ... FOR UPDATE` + conditional UPDATE pairs are
//! collapsed into single guarded `UPDATE ... WHERE ended_at_utc = NONE ...
//! RETURN AFTER` statements. SurrealDB reports affected rows by returning
//! them, so zero returned rows is the "someone else already ended it" signal
//! the row lock used to give; the NotFound-vs-Conflict distinction is then
//! derived from a diagnostic follow-up read, which correctness never depends
//! on. The concurrent-append safety of `attach_event_ledger_seq` (previously
//! PostgreSQL `jsonb_insert`/`||`) is carried by server-side
//! `array::append` inside the single UPDATE statement.

use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use surrealdb::types::{RecordId, SurrealValue};
use thiserror::Error;
use uuid::Uuid;

use super::spans::{ActivityKind, ActivitySpan, ModelSessionSpan, SpanId, SpanStatus};
use crate::storage::surreal::{SurrealStorage, SurrealStorageError};

const SESSION_SPAN_TABLE: &str = "kernel_model_session_span";
const ACTIVITY_SPAN_TABLE: &str = "kernel_activity_span";

/// Repository error.
#[derive(Debug, Error)]
pub enum SpanRepoError {
    #[error("span not found")]
    NotFound,
    #[error("storage error: {0}")]
    Storage(#[from] SurrealStorageError),
    #[error("serde_json error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("conflict: row was modified concurrently or already ended")]
    Conflict,
}

/// CX-503R: embedded SurrealDB only by construction; no other backend.
pub struct SpanRepo {
    storage: SurrealStorage,
}

// ── record shapes + bindings ────────────────────────────────────────────────

/// Write shape for `kernel_model_session_span`. The table asserts
/// `span_id = record::id($this.id)`, so the record key must be the span's
/// UUID key.
#[derive(SurrealValue)]
struct SessionSpanWriteRow {
    span_id: Uuid,
    model_session_id: Uuid,
    session_id: Uuid,
    started_at_utc: DateTime<Utc>,
    ended_at_utc: Option<DateTime<Utc>>,
    status: String,
    attributes: JsonValue,
    last_event_ledger_seq: Option<i64>,
}

/// Write shape for `kernel_activity_span`. `parent_span_id` is a
/// `record<kernel_model_session_span>` REFERENCE with `ASSERT record::exists`,
/// so an activity span can never be written against a session span that does
/// not exist — the previous foreign key is preserved by the schema.
#[derive(SurrealValue)]
struct ActivitySpanWriteRow {
    span_id: Uuid,
    parent_span_id: RecordId,
    activity_kind: String,
    started_at_utc: DateTime<Utc>,
    ended_at_utc: Option<DateTime<Utc>>,
    status: String,
    attributes: JsonValue,
    related_event_ledger_seqs: Vec<i64>,
}

/// Read shape for session spans. `span_id` doubles as the record key.
#[derive(SurrealValue)]
struct SessionSpanReadRow {
    span_id: Uuid,
    model_session_id: Uuid,
    session_id: Uuid,
    started_at_utc: DateTime<Utc>,
    ended_at_utc: Option<DateTime<Utc>>,
    status: String,
    attributes: JsonValue,
    last_event_ledger_seq: Option<i64>,
}

impl SessionSpanReadRow {
    fn into_public(self) -> SessionSpanRow {
        SessionSpanRow {
            span_id: SpanId(self.span_id),
            model_session_id: self.model_session_id,
            session_id: self.session_id,
            started_at_utc: self.started_at_utc,
            ended_at_utc: self.ended_at_utc,
            status: self.status,
            attributes: self.attributes,
            last_event_ledger_seq: self.last_event_ledger_seq,
        }
    }
}

/// Read shape for activity spans. `parent_span_id` is projected through
/// `record::id(...)`, which the schema guarantees to be the parent's UUID.
#[derive(SurrealValue)]
struct ActivitySpanReadRow {
    span_id: Uuid,
    parent_span_id: Uuid,
    activity_kind: String,
    started_at_utc: DateTime<Utc>,
    ended_at_utc: Option<DateTime<Utc>>,
    status: String,
    attributes: JsonValue,
    related_event_ledger_seqs: Vec<i64>,
}

impl ActivitySpanReadRow {
    fn into_public(self) -> ActivitySpanRow {
        ActivitySpanRow {
            span_id: SpanId(self.span_id),
            parent_span_id: SpanId(self.parent_span_id),
            activity_kind: self.activity_kind,
            started_at_utc: self.started_at_utc,
            ended_at_utc: self.ended_at_utc,
            status: self.status,
            attributes: self.attributes,
            related_event_ledger_seqs: JsonValue::Array(
                self.related_event_ledger_seqs
                    .into_iter()
                    .map(JsonValue::from)
                    .collect(),
            ),
        }
    }
}

#[derive(SurrealValue)]
struct CreateSpanBindings {
    record: RecordId,
    content: surrealdb::types::Value,
}

#[derive(SurrealValue)]
struct SpanIdBinding {
    span_id: Uuid,
}

#[derive(SurrealValue)]
struct EndSessionSpanBindings {
    span_id: Uuid,
    ended_at_utc: DateTime<Utc>,
    status: String,
    last_event_ledger_seq: Option<i64>,
}

#[derive(SurrealValue)]
struct EndActivitySpanBindings {
    span_id: Uuid,
    ended_at_utc: DateTime<Utc>,
    status: String,
}

#[derive(SurrealValue)]
struct AttachSeqBindings {
    span_id: Uuid,
    event_ledger_seq: i64,
}

#[derive(SurrealValue)]
struct ModelSessionBinding {
    model_session_id: Uuid,
}

#[derive(SurrealValue)]
struct ParentSpanBinding {
    parent: RecordId,
}

#[derive(SurrealValue)]
struct ModelSessionRangeBindings {
    model_session_id: Uuid,
    from_utc: DateTime<Utc>,
    to_utc: DateTime<Utc>,
}

fn session_span_record(id: Uuid) -> RecordId {
    RecordId::new(SESSION_SPAN_TABLE, surrealdb::types::Uuid::from(id))
}

fn activity_span_record(id: Uuid) -> RecordId {
    RecordId::new(ACTIVITY_SPAN_TABLE, surrealdb::types::Uuid::from(id))
}

impl SpanRepo {
    pub fn new(storage: SurrealStorage) -> Self {
        Self { storage }
    }

    pub fn storage(&self) -> &SurrealStorage {
        &self.storage
    }

    async fn query<R, B>(
        &self,
        statement: &'static str,
        bindings: B,
    ) -> Result<Vec<R>, SurrealStorageError>
    where
        R: SurrealValue + Send + 'static,
        B: SurrealValue + Send + 'static,
    {
        self.storage
            .with_data_operation(move |database| {
                Box::pin(async move { database.query_values(statement, bindings).await })
            })
            .await
    }

    /// Append-friendly insert of a fresh session span. Attributes,
    /// `started_at_utc`, and the immutable id columns are frozen at this
    /// point by the schema EVENT guard.
    pub async fn insert_session_span(&self, span: &ModelSessionSpan) -> Result<(), SpanRepoError> {
        let row = SessionSpanWriteRow {
            span_id: span.span_id.as_uuid(),
            model_session_id: span.model_session_id,
            session_id: span.session_id,
            started_at_utc: span.started_at_utc,
            ended_at_utc: span.ended_at_utc,
            status: span.status.as_str().to_string(),
            attributes: serde_json::to_value(&span.attributes)?,
            last_event_ledger_seq: None,
        };
        // CREATE, not UPSERT: a duplicate span id must fail exactly as the
        // original INSERT did rather than silently replacing a live span.
        let _: Vec<surrealdb::types::Value> = self
            .query(
                "CREATE $record CONTENT $content RETURN AFTER;",
                CreateSpanBindings {
                    record: session_span_record(span.span_id.as_uuid()),
                    content: row.into_value(),
                },
            )
            .await?;
        Ok(())
    }

    /// Append-friendly insert of an activity span. The record link to the
    /// parent session span is schema-enforced; deleting a session span
    /// cascades to its activity rows.
    pub async fn insert_activity_span(&self, span: &ActivitySpan) -> Result<(), SpanRepoError> {
        let row = ActivitySpanWriteRow {
            span_id: span.span_id.as_uuid(),
            parent_span_id: session_span_record(span.parent_span_id.as_uuid()),
            activity_kind: activity_kind_to_str(&span.activity_kind),
            started_at_utc: span.started_at_utc,
            ended_at_utc: span.ended_at_utc,
            status: span.status.as_str().to_string(),
            attributes: serde_json::to_value(&span.attributes)?,
            related_event_ledger_seqs: Vec::new(),
        };
        let _: Vec<surrealdb::types::Value> = self
            .query(
                "CREATE $record CONTENT $content RETURN AFTER;",
                CreateSpanBindings {
                    record: activity_span_record(span.span_id.as_uuid()),
                    content: row.into_value(),
                },
            )
            .await?;
        Ok(())
    }

    /// End a session span. The `ended_at_utc = NONE` guard inside the UPDATE
    /// gives concurrent end-writes exactly-one-winner semantics: the second
    /// writer matches zero rows and gets [`SpanRepoError::Conflict`].
    pub async fn update_session_span_end(
        &self,
        span_id: SpanId,
        ended_at_utc: DateTime<Utc>,
        status: &SpanStatus,
        last_event_ledger_seq: Option<i64>,
    ) -> Result<(), SpanRepoError> {
        let updated: Vec<surrealdb::types::Value> = self
            .query(
                "UPDATE kernel_model_session_span SET \
                 ended_at_utc = $ended_at_utc, \
                 status = $status, \
                 last_event_ledger_seq = IF $last_event_ledger_seq = NONE \
                   { last_event_ledger_seq } ELSE { $last_event_ledger_seq } \
                 WHERE span_id = $span_id AND ended_at_utc = NONE RETURN AFTER;",
                EndSessionSpanBindings {
                    span_id: span_id.as_uuid(),
                    ended_at_utc,
                    status: status.as_str().to_string(),
                    last_event_ledger_seq,
                },
            )
            .await?;
        if !updated.is_empty() {
            return Ok(());
        }
        // The guard refused. Re-read only to choose the typed error.
        let exists: Vec<Uuid> = self
            .query(
                "SELECT VALUE span_id FROM kernel_model_session_span WHERE span_id = $span_id;",
                SpanIdBinding {
                    span_id: span_id.as_uuid(),
                },
            )
            .await?;
        if exists.is_empty() {
            Err(SpanRepoError::NotFound)
        } else {
            Err(SpanRepoError::Conflict)
        }
    }

    pub async fn update_activity_span_end(
        &self,
        span_id: SpanId,
        ended_at_utc: DateTime<Utc>,
        status: &SpanStatus,
    ) -> Result<(), SpanRepoError> {
        let updated: Vec<surrealdb::types::Value> = self
            .query(
                "UPDATE kernel_activity_span SET \
                 ended_at_utc = $ended_at_utc, status = $status \
                 WHERE span_id = $span_id AND ended_at_utc = NONE RETURN AFTER;",
                EndActivitySpanBindings {
                    span_id: span_id.as_uuid(),
                    ended_at_utc,
                    status: status.as_str().to_string(),
                },
            )
            .await?;
        if !updated.is_empty() {
            return Ok(());
        }
        let exists: Vec<Uuid> = self
            .query(
                "SELECT VALUE span_id FROM kernel_activity_span WHERE span_id = $span_id;",
                SpanIdBinding {
                    span_id: span_id.as_uuid(),
                },
            )
            .await?;
        if exists.is_empty() {
            Err(SpanRepoError::NotFound)
        } else {
            Err(SpanRepoError::Conflict)
        }
    }

    /// Append an EventLedger seq to the activity span's accumulator.
    /// Server-side `array::append` inside the single UPDATE statement means
    /// concurrent appends don't trample each other's writes — every caller
    /// stamps a new array element at the tail.
    pub async fn attach_event_ledger_seq(
        &self,
        activity_span_id: SpanId,
        event_ledger_seq: i64,
    ) -> Result<(), SpanRepoError> {
        let updated: Vec<surrealdb::types::Value> = self
            .query(
                "UPDATE kernel_activity_span SET \
                 related_event_ledger_seqs = \
                   array::append(related_event_ledger_seqs, $event_ledger_seq) \
                 WHERE span_id = $span_id RETURN AFTER;",
                AttachSeqBindings {
                    span_id: activity_span_id.as_uuid(),
                    event_ledger_seq,
                },
            )
            .await?;
        if updated.is_empty() {
            return Err(SpanRepoError::NotFound);
        }
        Ok(())
    }

    /// Read a session span by id. Returns `Ok(None)` if not present.
    pub async fn get_session_span(
        &self,
        span_id: SpanId,
    ) -> Result<Option<SessionSpanRow>, SpanRepoError> {
        let rows: Vec<SessionSpanReadRow> = self
            .query(
                "SELECT span_id, model_session_id, session_id, started_at_utc, \
                 ended_at_utc, status, attributes, last_event_ledger_seq \
                 FROM kernel_model_session_span WHERE span_id = $span_id;",
                SpanIdBinding {
                    span_id: span_id.as_uuid(),
                },
            )
            .await?;
        Ok(rows.into_iter().next().map(SessionSpanReadRow::into_public))
    }

    /// Read an activity span by id. Returns `Ok(None)` if not present.
    pub async fn get_activity_span(
        &self,
        span_id: SpanId,
    ) -> Result<Option<ActivitySpanRow>, SpanRepoError> {
        let rows: Vec<ActivitySpanReadRow> = self
            .query(
                "SELECT span_id, record::id(parent_span_id) AS parent_span_id, \
                 activity_kind, started_at_utc, ended_at_utc, status, attributes, \
                 related_event_ledger_seqs \
                 FROM kernel_activity_span WHERE span_id = $span_id;",
                SpanIdBinding {
                    span_id: span_id.as_uuid(),
                },
            )
            .await?;
        Ok(rows
            .into_iter()
            .next()
            .map(ActivitySpanReadRow::into_public))
    }

    /// Spec-line-1011 cross-link query: session spans for a given
    /// `model_session_id`, ordered most-recent-first. Backs the folded
    /// WP-1-Session-Observability-Spans-FR-v1 invariant that session-wide
    /// queries via `model_session_id` work even when no spans were
    /// emitted (returns an empty Vec in that case).
    pub async fn query_session_spans_for_model_session_id(
        &self,
        model_session_id: Uuid,
    ) -> Result<Vec<SessionSpanRow>, SpanRepoError> {
        let rows: Vec<SessionSpanReadRow> = self
            .query(
                "SELECT span_id, model_session_id, session_id, started_at_utc, \
                 ended_at_utc, status, attributes, last_event_ledger_seq \
                 FROM kernel_model_session_span \
                 WHERE model_session_id = $model_session_id \
                 ORDER BY started_at_utc DESC;",
                ModelSessionBinding { model_session_id },
            )
            .await?;
        Ok(rows
            .into_iter()
            .map(SessionSpanReadRow::into_public)
            .collect())
    }

    /// All activity spans whose parent is the given session span,
    /// ordered by `started_at_utc` ascending (so the diagnostic panel
    /// can render a chronologically correct nested tree).
    pub async fn query_activity_spans_for_session_span(
        &self,
        parent_session_span_id: SpanId,
    ) -> Result<Vec<ActivitySpanRow>, SpanRepoError> {
        let rows: Vec<ActivitySpanReadRow> = self
            .query(
                "SELECT span_id, record::id(parent_span_id) AS parent_span_id, \
                 activity_kind, started_at_utc, ended_at_utc, status, attributes, \
                 related_event_ledger_seqs \
                 FROM kernel_activity_span WHERE parent_span_id = $parent \
                 ORDER BY started_at_utc ASC;",
                ParentSpanBinding {
                    parent: session_span_record(parent_session_span_id.as_uuid()),
                },
            )
            .await?;
        Ok(rows
            .into_iter()
            .map(ActivitySpanReadRow::into_public)
            .collect())
    }

    /// Range query for the diagnostics panel: every activity span whose
    /// owning session span belongs to `model_session_id` and whose start
    /// time falls inside `[from_utc, to_utc]`. The previous SQL join walks
    /// through the `parent_span_id` record link instead.
    pub async fn query_activity_spans_for_model_session_in_range(
        &self,
        model_session_id: Uuid,
        from_utc: DateTime<Utc>,
        to_utc: DateTime<Utc>,
    ) -> Result<Vec<ActivitySpanRow>, SpanRepoError> {
        let rows: Vec<ActivitySpanReadRow> = self
            .query(
                "SELECT span_id, record::id(parent_span_id) AS parent_span_id, \
                 activity_kind, started_at_utc, ended_at_utc, status, attributes, \
                 related_event_ledger_seqs \
                 FROM kernel_activity_span \
                 WHERE parent_span_id.model_session_id = $model_session_id \
                   AND started_at_utc >= $from_utc \
                   AND started_at_utc <= $to_utc \
                 ORDER BY started_at_utc ASC;",
                ModelSessionRangeBindings {
                    model_session_id,
                    from_utc,
                    to_utc,
                },
            )
            .await?;
        Ok(rows
            .into_iter()
            .map(ActivitySpanReadRow::into_public)
            .collect())
    }
}

/// Row shape for `kernel_model_session_span`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSpanRow {
    pub span_id: SpanId,
    pub model_session_id: Uuid,
    pub session_id: Uuid,
    pub started_at_utc: DateTime<Utc>,
    pub ended_at_utc: Option<DateTime<Utc>>,
    pub status: String,
    pub attributes: JsonValue,
    pub last_event_ledger_seq: Option<i64>,
}

/// Row shape for `kernel_activity_span`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivitySpanRow {
    pub span_id: SpanId,
    pub parent_span_id: SpanId,
    pub activity_kind: String,
    pub started_at_utc: DateTime<Utc>,
    pub ended_at_utc: Option<DateTime<Utc>>,
    pub status: String,
    pub attributes: JsonValue,
    pub related_event_ledger_seqs: JsonValue,
}

fn activity_kind_to_str(kind: &ActivityKind) -> String {
    match kind {
        ActivityKind::MtIteration => "mt_iteration".to_string(),
        ActivityKind::MailboxLease => "mailbox_lease".to_string(),
        ActivityKind::ModelSwap => "model_swap".to_string(),
        ActivityKind::CheckpointWrite => "checkpoint_write".to_string(),
        ActivityKind::StateReplay => "state_replay".to_string(),
        ActivityKind::ToolInvocation => "tool_invocation".to_string(),
        ActivityKind::Other(s) => format!("other:{s}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flight_recorder::spans::ActivityKind;

    #[test]
    fn activity_kind_str_canonical() {
        assert_eq!(
            activity_kind_to_str(&ActivityKind::MtIteration),
            "mt_iteration"
        );
        assert_eq!(
            activity_kind_to_str(&ActivityKind::MailboxLease),
            "mailbox_lease"
        );
        assert_eq!(activity_kind_to_str(&ActivityKind::ModelSwap), "model_swap");
        assert_eq!(
            activity_kind_to_str(&ActivityKind::CheckpointWrite),
            "checkpoint_write"
        );
        assert_eq!(
            activity_kind_to_str(&ActivityKind::StateReplay),
            "state_replay"
        );
        assert_eq!(
            activity_kind_to_str(&ActivityKind::ToolInvocation),
            "tool_invocation"
        );
        assert_eq!(
            activity_kind_to_str(&ActivityKind::Other("xyz".to_string())),
            "other:xyz"
        );
    }
}

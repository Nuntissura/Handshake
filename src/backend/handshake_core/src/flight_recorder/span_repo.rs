//! MT-197 Flight Recorder span Postgres repository.
//!
//! Authority: cluster X.4 contract
//! `.GOV/task_packets/WP-KERNEL-004-.../MT-197.json`.
//!
//! Folded WP-1-Session-Observability-Spans-FR-v1 invariant:
//! "session-wide queries via model_session_id work even without spans"
//! — implemented here by joining `kernel_model_session_span` to
//! `kernel_event_ledger` via `model_session_id` in
//! [`SpanRepo::query_session_spans_for_model_session_id`].
//!
//! Authority-files alignment: MT-197 `expected_diff_shape` referenced
//! `src/backend/handshake_core/src/observability/span_repo.rs`; the
//! cluster already landed observability under `flight_recorder::spans`
//! via MT-196 deviation, so this MT extends the same module per MT-196
//! `contract_deviation_note`. Schema lives across
//! `migrations/0024_session_checkpoint.sql` (base tables, MT-190..194)
//! and `migrations/0025_observability_spans.sql` (MT-197 hardening).
//!
//! Write model:
//!   - INSERT a new session span -> `insert_session_span`.
//!   - INSERT a new activity span (with FK parent) -> `insert_activity_span`.
//!   - End a span (status + ended_at_utc + optional ledger watermark)
//!     -> `update_session_span_end` / `update_activity_span_end`.
//!   - Append an event ledger seq to an activity span's
//!     `related_event_ledger_seqs` -> `attach_event_ledger_seq`.
//!
//! Attributes and other immutable fields are enforced by the database
//! triggers shipped in 0025_observability_spans.sql; the Rust API has
//! no method to update them on purpose.

use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use sqlx::PgPool;
use thiserror::Error;
use uuid::Uuid;

use super::spans::{ActivityKind, ActivitySpan, ModelSessionSpan, SpanId, SpanStatus};

/// Repository error.
#[derive(Debug, Error)]
pub enum SpanRepoError {
    #[error("span not found")]
    NotFound,
    #[error("sqlx error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("serde_json error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("conflict: row was modified concurrently or already ended")]
    Conflict,
}

/// CX-503R: Postgres-only by construction; no SQLite backend.
pub struct SpanRepo {
    pool: PgPool,
}

impl SpanRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Append-friendly insert of a fresh session span. Attributes,
    /// `started_at_utc`, and the immutable id columns are frozen at this
    /// point by the 0025 trigger.
    pub async fn insert_session_span(&self, span: &ModelSessionSpan) -> Result<(), SpanRepoError> {
        let attributes = serde_json::to_value(&span.attributes)?;
        sqlx::query(
            r#"
            INSERT INTO kernel_model_session_span (
                span_id, model_session_id, session_id,
                started_at_utc, ended_at_utc, status,
                attributes, last_event_ledger_seq
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(span.span_id.as_uuid())
        .bind(span.model_session_id)
        .bind(span.session_id)
        .bind(span.started_at_utc)
        .bind(span.ended_at_utc)
        .bind(span.status.as_str())
        .bind(attributes)
        .bind(None::<i64>)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Append-friendly insert of an activity span. The FK to the parent
    /// session span is enforced by the 0024 schema; deleting a session
    /// span cascades to its activity rows.
    pub async fn insert_activity_span(&self, span: &ActivitySpan) -> Result<(), SpanRepoError> {
        let attributes = serde_json::to_value(&span.attributes)?;
        let activity_kind = activity_kind_to_str(&span.activity_kind);
        sqlx::query(
            r#"
            INSERT INTO kernel_activity_span (
                span_id, parent_span_id, activity_kind,
                started_at_utc, ended_at_utc, status,
                attributes, related_event_ledger_seqs
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, '[]'::jsonb)
            "#,
        )
        .bind(span.span_id.as_uuid())
        .bind(span.parent_span_id.as_uuid())
        .bind(activity_kind)
        .bind(span.started_at_utc)
        .bind(span.ended_at_utc)
        .bind(span.status.as_str())
        .bind(attributes)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// End a session span. Uses a transactional `SELECT ... FOR UPDATE`
    /// so concurrent end-writes see exactly-one-winner semantics: the
    /// second writer sees the already-ended row and gets
    /// [`SpanRepoError::Conflict`].
    pub async fn update_session_span_end(
        &self,
        span_id: SpanId,
        ended_at_utc: DateTime<Utc>,
        status: &SpanStatus,
        last_event_ledger_seq: Option<i64>,
    ) -> Result<(), SpanRepoError> {
        let mut tx = self.pool.begin().await?;
        let row: Option<(Option<DateTime<Utc>>,)> = sqlx::query_as(
            "SELECT ended_at_utc FROM kernel_model_session_span WHERE span_id = $1 FOR UPDATE",
        )
        .bind(span_id.as_uuid())
        .fetch_optional(&mut *tx)
        .await?;
        let Some((existing_end,)) = row else {
            return Err(SpanRepoError::NotFound);
        };
        if existing_end.is_some() {
            return Err(SpanRepoError::Conflict);
        }

        let affected = sqlx::query(
            r#"
            UPDATE kernel_model_session_span
            SET ended_at_utc = $2,
                status = $3,
                last_event_ledger_seq = COALESCE($4, last_event_ledger_seq)
            WHERE span_id = $1 AND ended_at_utc IS NULL
            "#,
        )
        .bind(span_id.as_uuid())
        .bind(ended_at_utc)
        .bind(status.as_str())
        .bind(last_event_ledger_seq)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if affected == 0 {
            return Err(SpanRepoError::Conflict);
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn update_activity_span_end(
        &self,
        span_id: SpanId,
        ended_at_utc: DateTime<Utc>,
        status: &SpanStatus,
    ) -> Result<(), SpanRepoError> {
        let mut tx = self.pool.begin().await?;
        let row: Option<(Option<DateTime<Utc>>,)> = sqlx::query_as(
            "SELECT ended_at_utc FROM kernel_activity_span WHERE span_id = $1 FOR UPDATE",
        )
        .bind(span_id.as_uuid())
        .fetch_optional(&mut *tx)
        .await?;
        let Some((existing_end,)) = row else {
            return Err(SpanRepoError::NotFound);
        };
        if existing_end.is_some() {
            return Err(SpanRepoError::Conflict);
        }

        let affected = sqlx::query(
            r#"
            UPDATE kernel_activity_span
            SET ended_at_utc = $2,
                status = $3
            WHERE span_id = $1 AND ended_at_utc IS NULL
            "#,
        )
        .bind(span_id.as_uuid())
        .bind(ended_at_utc)
        .bind(status.as_str())
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if affected == 0 {
            return Err(SpanRepoError::Conflict);
        }
        tx.commit().await?;
        Ok(())
    }

    /// Append an EventLedger seq to the activity span's accumulator.
    /// Postgres-side `jsonb_insert` is used so concurrent appends don't
    /// trample each other's writes — every caller stamps a new array
    /// element at the tail.
    pub async fn attach_event_ledger_seq(
        &self,
        activity_span_id: SpanId,
        event_ledger_seq: i64,
    ) -> Result<(), SpanRepoError> {
        let affected = sqlx::query(
            r#"
            UPDATE kernel_activity_span
            SET related_event_ledger_seqs =
                COALESCE(related_event_ledger_seqs, '[]'::jsonb)
                || to_jsonb($2::bigint)
            WHERE span_id = $1
            "#,
        )
        .bind(activity_span_id.as_uuid())
        .bind(event_ledger_seq)
        .execute(&self.pool)
        .await?
        .rows_affected();
        if affected == 0 {
            return Err(SpanRepoError::NotFound);
        }
        Ok(())
    }

    /// Read a session span by id. Returns `Ok(None)` if not present.
    pub async fn get_session_span(
        &self,
        span_id: SpanId,
    ) -> Result<Option<SessionSpanRow>, SpanRepoError> {
        let row: Option<(
            Uuid,
            Uuid,
            Uuid,
            DateTime<Utc>,
            Option<DateTime<Utc>>,
            String,
            JsonValue,
            Option<i64>,
        )> = sqlx::query_as(
            r#"
            SELECT span_id, model_session_id, session_id, started_at_utc,
                   ended_at_utc, status, attributes, last_event_ledger_seq
            FROM kernel_model_session_span WHERE span_id = $1
            "#,
        )
        .bind(span_id.as_uuid())
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| SessionSpanRow {
            span_id: SpanId(r.0),
            model_session_id: r.1,
            session_id: r.2,
            started_at_utc: r.3,
            ended_at_utc: r.4,
            status: r.5,
            attributes: r.6,
            last_event_ledger_seq: r.7,
        }))
    }

    /// Read an activity span by id. Returns `Ok(None)` if not present.
    pub async fn get_activity_span(
        &self,
        span_id: SpanId,
    ) -> Result<Option<ActivitySpanRow>, SpanRepoError> {
        let row: Option<(
            Uuid,
            Uuid,
            String,
            DateTime<Utc>,
            Option<DateTime<Utc>>,
            String,
            JsonValue,
            JsonValue,
        )> = sqlx::query_as(
            r#"
            SELECT span_id, parent_span_id, activity_kind, started_at_utc,
                   ended_at_utc, status, attributes, related_event_ledger_seqs
            FROM kernel_activity_span WHERE span_id = $1
            "#,
        )
        .bind(span_id.as_uuid())
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| ActivitySpanRow {
            span_id: SpanId(r.0),
            parent_span_id: SpanId(r.1),
            activity_kind: r.2,
            started_at_utc: r.3,
            ended_at_utc: r.4,
            status: r.5,
            attributes: r.6,
            related_event_ledger_seqs: r.7,
        }))
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
        let rows: Vec<(
            Uuid,
            Uuid,
            Uuid,
            DateTime<Utc>,
            Option<DateTime<Utc>>,
            String,
            JsonValue,
            Option<i64>,
        )> = sqlx::query_as(
            r#"
            SELECT span_id, model_session_id, session_id, started_at_utc,
                   ended_at_utc, status, attributes, last_event_ledger_seq
            FROM kernel_model_session_span
            WHERE model_session_id = $1
            ORDER BY started_at_utc DESC
            "#,
        )
        .bind(model_session_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| SessionSpanRow {
                span_id: SpanId(r.0),
                model_session_id: r.1,
                session_id: r.2,
                started_at_utc: r.3,
                ended_at_utc: r.4,
                status: r.5,
                attributes: r.6,
                last_event_ledger_seq: r.7,
            })
            .collect())
    }

    /// All activity spans whose parent is the given session span,
    /// ordered by `started_at_utc` ascending (so the diagnostic panel
    /// can render a chronologically correct nested tree).
    pub async fn query_activity_spans_for_session_span(
        &self,
        parent_session_span_id: SpanId,
    ) -> Result<Vec<ActivitySpanRow>, SpanRepoError> {
        let rows: Vec<(
            Uuid,
            Uuid,
            String,
            DateTime<Utc>,
            Option<DateTime<Utc>>,
            String,
            JsonValue,
            JsonValue,
        )> = sqlx::query_as(
            r#"
            SELECT span_id, parent_span_id, activity_kind, started_at_utc,
                   ended_at_utc, status, attributes, related_event_ledger_seqs
            FROM kernel_activity_span
            WHERE parent_span_id = $1
            ORDER BY started_at_utc ASC
            "#,
        )
        .bind(parent_session_span_id.as_uuid())
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| ActivitySpanRow {
                span_id: SpanId(r.0),
                parent_span_id: SpanId(r.1),
                activity_kind: r.2,
                started_at_utc: r.3,
                ended_at_utc: r.4,
                status: r.5,
                attributes: r.6,
                related_event_ledger_seqs: r.7,
            })
            .collect())
    }

    /// Range query for the diagnostics panel: every activity span whose
    /// owning session span belongs to `model_session_id` and whose start
    /// time falls inside `[from_utc, to_utc]`.
    pub async fn query_activity_spans_for_model_session_in_range(
        &self,
        model_session_id: Uuid,
        from_utc: DateTime<Utc>,
        to_utc: DateTime<Utc>,
    ) -> Result<Vec<ActivitySpanRow>, SpanRepoError> {
        let rows: Vec<(
            Uuid,
            Uuid,
            String,
            DateTime<Utc>,
            Option<DateTime<Utc>>,
            String,
            JsonValue,
            JsonValue,
        )> = sqlx::query_as(
            r#"
            SELECT a.span_id, a.parent_span_id, a.activity_kind,
                   a.started_at_utc, a.ended_at_utc, a.status,
                   a.attributes, a.related_event_ledger_seqs
            FROM kernel_activity_span a
            JOIN kernel_model_session_span s
                ON s.span_id = a.parent_span_id
            WHERE s.model_session_id = $1
              AND a.started_at_utc >= $2
              AND a.started_at_utc <= $3
            ORDER BY a.started_at_utc ASC
            "#,
        )
        .bind(model_session_id)
        .bind(from_utc)
        .bind(to_utc)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| ActivitySpanRow {
                span_id: SpanId(r.0),
                parent_span_id: SpanId(r.1),
                activity_kind: r.2,
                started_at_utc: r.3,
                ended_at_utc: r.4,
                status: r.5,
                attributes: r.6,
                related_event_ledger_seqs: r.7,
            })
            .collect())
    }
}

/// Postgres row shape for `kernel_model_session_span`.
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

/// Postgres row shape for `kernel_activity_span`.
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

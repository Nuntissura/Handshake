//! WP-KERNEL-012 MT-067: the calendar activity-span store — the native
//! editor's edit-activity provenance for a calendar block.
//!
//! DISTINCT from `flight_recorder::spans::ActivitySpan` (table
//! `kernel_activity_span`, a swarm / mt-iteration span with no calendar
//! linkage): this store is calendar-specific — every span carries a
//! `calendar_event_id` and the set of documents edited during that event window
//! (`edited_doc_ids`). Backed by `calendar_activity_spans` (migration 0340)
//! over the shared PostgreSQL pool — PostgreSQL authority only, no SQLite.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{postgres::PgRow, PgPool, Row};

use super::StorageError;

/// One stored calendar activity span.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CalendarActivitySpan {
    pub span_id: String,
    pub workspace_id: String,
    pub calendar_event_id: String,
    pub started_utc: DateTime<Utc>,
    /// `None` while the span is still open (an in-progress edit block).
    pub ended_utc: Option<DateTime<Utc>>,
    pub edited_doc_ids: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Upsert input for a calendar activity span.
#[derive(Clone, Debug)]
pub struct NewCalendarActivitySpan {
    pub span_id: String,
    pub workspace_id: String,
    pub calendar_event_id: String,
    pub started_utc: DateTime<Utc>,
    pub ended_utc: Option<DateTime<Utc>>,
    pub edited_doc_ids: Vec<String>,
}

/// Pool-backed store for calendar activity spans + the daily-note linkage read.
/// Cheap to construct per request (wraps a pooled handle; never reconnects).
#[derive(Clone)]
pub struct CalendarActivityStore {
    pool: PgPool,
}

impl CalendarActivityStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Insert-or-update a calendar activity span keyed by `span_id`.
    pub async fn upsert_activity_span(
        &self,
        input: NewCalendarActivitySpan,
    ) -> Result<CalendarActivitySpan, StorageError> {
        if input.span_id.trim().is_empty() {
            return Err(StorageError::Validation("activity span span_id is required"));
        }
        if input.workspace_id.trim().is_empty() {
            return Err(StorageError::Validation(
                "activity span workspace_id is required",
            ));
        }
        if input.calendar_event_id.trim().is_empty() {
            return Err(StorageError::Validation(
                "activity span calendar_event_id is required",
            ));
        }
        if let Some(ended) = input.ended_utc {
            if ended < input.started_utc {
                return Err(StorageError::Validation(
                    "activity span ended_utc must be >= started_utc",
                ));
            }
        }

        // JSON array of doc-id strings for the `edited_doc_ids` jsonb column.
        let edited: Value = Value::Array(
            input
                .edited_doc_ids
                .iter()
                .map(|d| Value::String(d.clone()))
                .collect(),
        );

        let row = sqlx::query(
            r#"
            INSERT INTO calendar_activity_spans
                (span_id, workspace_id, calendar_event_id, started_utc, ended_utc, edited_doc_ids)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (span_id) DO UPDATE SET
                workspace_id = EXCLUDED.workspace_id,
                calendar_event_id = EXCLUDED.calendar_event_id,
                started_utc = EXCLUDED.started_utc,
                ended_utc = EXCLUDED.ended_utc,
                edited_doc_ids = EXCLUDED.edited_doc_ids,
                updated_at = NOW()
            RETURNING span_id, workspace_id, calendar_event_id, started_utc, ended_utc,
                      edited_doc_ids, created_at, updated_at
            "#,
        )
        .bind(&input.span_id)
        .bind(&input.workspace_id)
        .bind(&input.calendar_event_id)
        .bind(input.started_utc)
        .bind(input.ended_utc)
        .bind(edited)
        .fetch_one(&self.pool)
        .await
        .map_err(StorageError::from)?;
        Ok(map_span_row(&row))
    }

    /// All activity spans for a calendar event, newest span-start first.
    pub async fn query_activity_spans_by_event(
        &self,
        workspace_id: &str,
        calendar_event_id: &str,
    ) -> Result<Vec<CalendarActivitySpan>, StorageError> {
        let rows = sqlx::query(
            r#"
            SELECT span_id, workspace_id, calendar_event_id, started_utc, ended_utc,
                   edited_doc_ids, created_at, updated_at
            FROM calendar_activity_spans
            WHERE workspace_id = $1 AND calendar_event_id = $2
            ORDER BY started_utc DESC, span_id ASC
            "#,
        )
        .bind(workspace_id)
        .bind(calendar_event_id)
        .fetch_all(&self.pool)
        .await
        .map_err(StorageError::from)?;
        Ok(rows.iter().map(map_span_row).collect())
    }

    /// Read-only: the daily-note doc id for a workspace + date, if a daily
    /// journal LoomBlock exists for that date (`content_type = 'journal'` and
    /// `journal_date` = the `YYYY-MM-DD` key — the MT-019 / MT-257 daily
    /// journal). Returns the block's linked `document_id` when present, else the
    /// block id (the same date->doc key the native daily-note interop uses in
    /// `calendar_interop::open_or_create_daily_note`). This is a pure LOOKUP: it
    /// never creates a journal block.
    pub async fn find_daily_note_doc_id_for_date(
        &self,
        workspace_id: &str,
        journal_date: &str,
    ) -> Result<Option<String>, StorageError> {
        let row = sqlx::query(
            r#"
            SELECT block_id, document_id
            FROM loom_blocks
            WHERE workspace_id = $1
              AND content_type = 'journal'
              AND journal_date = $2
            LIMIT 1
            "#,
        )
        .bind(workspace_id)
        .bind(journal_date)
        .fetch_optional(&self.pool)
        .await
        .map_err(StorageError::from)?;
        Ok(row.map(|r| {
            let document_id: Option<String> = r.get("document_id");
            let block_id: String = r.get("block_id");
            document_id.unwrap_or(block_id)
        }))
    }
}

/// Map one `calendar_activity_spans` row to the domain type.
fn map_span_row(row: &PgRow) -> CalendarActivitySpan {
    let edited: Value = row.get("edited_doc_ids");
    let edited_doc_ids = edited
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    CalendarActivitySpan {
        span_id: row.get("span_id"),
        workspace_id: row.get("workspace_id"),
        calendar_event_id: row.get("calendar_event_id"),
        started_utc: row.get("started_utc"),
        ended_utc: row.get("ended_utc"),
        edited_doc_ids,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

//! WP-KERNEL-012 MT-067 calendar activity-span authority on embedded SurrealDB.
//!
//! This store is calendar-specific and distinct from the swarm activity-span
//! projection. Every row belongs to one workspace and calendar event, and the
//! span identity is immutable across idempotent retries.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::types::{Datetime, RecordId, RecordIdKey, SurrealValue};

use super::{surreal::SurrealStorage, StorageError};

const ACTIVITY_TABLE: &str = "calendar_activity_spans";
const WORKSPACES_TABLE: &str = "workspaces";
const CALENDAR_EVENTS_TABLE: &str = "calendar_events";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CalendarActivitySpan {
    pub span_id: String,
    pub workspace_id: String,
    pub calendar_event_id: String,
    pub started_utc: DateTime<Utc>,
    pub ended_utc: Option<DateTime<Utc>>,
    pub edited_doc_ids: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct NewCalendarActivitySpan {
    pub span_id: String,
    pub workspace_id: String,
    pub calendar_event_id: String,
    pub started_utc: DateTime<Utc>,
    pub ended_utc: Option<DateTime<Utc>>,
    pub edited_doc_ids: Vec<String>,
}

#[derive(SurrealValue)]
struct ActivitySpanRow {
    span_id: String,
    workspace_id: RecordId,
    calendar_event_id: String,
    started_utc: Datetime,
    ended_utc: Option<Datetime>,
    edited_doc_ids: Vec<String>,
    created_at: Datetime,
    updated_at: Datetime,
}

#[derive(SurrealValue)]
struct UpsertBindings {
    span: RecordId,
    workspace: RecordId,
    event: RecordId,
    span_id: String,
    calendar_event_id: String,
    started_utc: Datetime,
    ended_utc: Option<Datetime>,
    edited_doc_ids: Vec<String>,
}

#[derive(SurrealValue)]
struct EventBindings {
    workspace: RecordId,
    calendar_event_id: String,
}

#[derive(SurrealValue)]
struct DailyNoteBindings {
    workspace: RecordId,
    journal_date: String,
}

#[derive(SurrealValue)]
struct DailyNoteRow {
    block_id: String,
    document_id: Option<RecordId>,
}

#[derive(Clone)]
pub struct CalendarActivityStore {
    storage: SurrealStorage,
}

impl CalendarActivityStore {
    pub fn new(storage: SurrealStorage) -> Self {
        Self { storage }
    }

    pub async fn upsert_activity_span(
        &self,
        input: NewCalendarActivitySpan,
    ) -> Result<CalendarActivitySpan, StorageError> {
        if input.span_id.trim().is_empty() {
            return Err(StorageError::Validation(
                "activity span span_id is required",
            ));
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
        if input
            .ended_utc
            .is_some_and(|ended| ended < input.started_utc)
        {
            return Err(StorageError::Validation(
                "activity span ended_utc must be >= started_utc",
            ));
        }

        let bindings = UpsertBindings {
            span: RecordId::new(ACTIVITY_TABLE, input.span_id.clone()),
            workspace: RecordId::new(WORKSPACES_TABLE, input.workspace_id),
            event: RecordId::new(CALENDAR_EVENTS_TABLE, input.calendar_event_id.clone()),
            span_id: input.span_id,
            calendar_event_id: input.calendar_event_id,
            started_utc: Datetime::from(input.started_utc),
            ended_utc: input.ended_utc.map(Datetime::from),
            edited_doc_ids: input.edited_doc_ids,
        };
        let result: Result<Option<ActivitySpanRow>, _> = self
            .storage
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .query_first(
                            "IF (SELECT VALUE id FROM $event WHERE workspace_id = $workspace LIMIT 1)[0] = NONE { \
                                 THROW 'HSK-CALENDAR-EVENT-NOT-FOUND'; \
                             } ELSE IF (SELECT VALUE workspace_id FROM $span LIMIT 1)[0] != NONE \
                                 AND (SELECT VALUE workspace_id FROM $span LIMIT 1)[0] != $workspace { \
                                 THROW 'HSK-CALENDAR-SPAN-WORKSPACE-CONFLICT'; \
                             } ELSE IF (SELECT VALUE calendar_event_id FROM $span LIMIT 1)[0] != NONE \
                                 AND (SELECT VALUE calendar_event_id FROM $span LIMIT 1)[0] != $calendar_event_id { \
                                 THROW 'HSK-CALENDAR-SPAN-EVENT-CONFLICT'; \
                             } ELSE { \
                                 RETURN UPSERT $span SET span_id = $span_id, workspace_id = $workspace, \
                                     calendar_event_id = $calendar_event_id, started_utc = $started_utc, \
                                     ended_utc = $ended_utc, edited_doc_ids = $edited_doc_ids, \
                                     updated_at = time::now() RETURN AFTER; \
                             };",
                            bindings,
                        )
                        .await
                })
            })
            .await;

        match result {
            Ok(Some(row)) => map_span_row(row),
            Ok(None) => Err(StorageError::Database(
                "calendar activity span upsert returned no row".to_owned(),
            )),
            Err(error) => {
                let rendered = error.to_string();
                if rendered.contains("HSK-CALENDAR-EVENT-NOT-FOUND") {
                    Err(StorageError::NotFound("calendar_event_not_found"))
                } else if rendered.contains("HSK-CALENDAR-SPAN-WORKSPACE-CONFLICT") {
                    Err(StorageError::Conflict(
                        "calendar_activity_span_workspace_conflict",
                    ))
                } else if rendered.contains("HSK-CALENDAR-SPAN-EVENT-CONFLICT") {
                    Err(StorageError::Conflict(
                        "calendar_activity_span_event_conflict",
                    ))
                } else {
                    Err(StorageError::Database(rendered))
                }
            }
        }
    }

    pub async fn query_activity_spans_by_event(
        &self,
        workspace_id: &str,
        calendar_event_id: &str,
    ) -> Result<Vec<CalendarActivitySpan>, StorageError> {
        let bindings = EventBindings {
            workspace: RecordId::new(WORKSPACES_TABLE, workspace_id),
            calendar_event_id: calendar_event_id.to_owned(),
        };
        let rows: Vec<ActivitySpanRow> = self
            .storage
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .query_values(
                            "SELECT span_id, workspace_id, calendar_event_id, started_utc, ended_utc, \
                                 edited_doc_ids, created_at, updated_at FROM calendar_activity_spans \
                             WHERE workspace_id = $workspace AND calendar_event_id = $calendar_event_id \
                             ORDER BY started_utc DESC, span_id ASC;",
                            bindings,
                        )
                        .await
                })
            })
            .await
            .map_err(|error| StorageError::Database(error.to_string()))?;
        rows.into_iter().map(map_span_row).collect()
    }

    pub async fn find_daily_note_doc_id_for_date(
        &self,
        workspace_id: &str,
        journal_date: &str,
    ) -> Result<Option<String>, StorageError> {
        let bindings = DailyNoteBindings {
            workspace: RecordId::new(WORKSPACES_TABLE, workspace_id),
            journal_date: journal_date.to_owned(),
        };
        let row: Option<DailyNoteRow> = self
            .storage
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .query_first(
                            "SELECT block_id, document_id FROM loom_blocks \
                             WHERE workspace_id = $workspace AND content_type = 'journal' \
                             AND journal_date = $journal_date LIMIT 1;",
                            bindings,
                        )
                        .await
                })
            })
            .await
            .map_err(|error| StorageError::Database(error.to_string()))?;
        row.map(|row| match row.document_id {
            Some(record) => record_key(record, "daily-note document"),
            None => Ok(row.block_id),
        })
        .transpose()
    }
}

fn map_span_row(row: ActivitySpanRow) -> Result<CalendarActivitySpan, StorageError> {
    Ok(CalendarActivitySpan {
        span_id: row.span_id,
        workspace_id: record_key(row.workspace_id, "calendar activity workspace")?,
        calendar_event_id: row.calendar_event_id,
        started_utc: row.started_utc.into_inner(),
        ended_utc: row.ended_utc.map(Datetime::into_inner),
        edited_doc_ids: row.edited_doc_ids,
        created_at: row.created_at.into_inner(),
        updated_at: row.updated_at.into_inner(),
    })
}

fn record_key(record: RecordId, field: &str) -> Result<String, StorageError> {
    match record.key {
        RecordIdKey::String(value) => Ok(value),
        _ => Err(StorageError::Serialization(format!(
            "{field} is not a string record key"
        ))),
    }
}

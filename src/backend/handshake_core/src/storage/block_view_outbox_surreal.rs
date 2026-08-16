//! Embedded-SurrealDB persistence for the MT-027 Block Collection View outbox.
//!
//! This is the FIRST module outside `storage/surreal/` to persist through the
//! widened [`SurrealDataContext`] seam, so it is deliberately written to be
//! copied: the record shape, the record-link handling, the affected-row
//! convention and the error mapping here are the pattern the remaining outer
//! modules should follow rather than each inventing their own.
//!
//! Three things differ from the PostgreSQL original and are load-bearing:
//!
//! 1. `workspace_id` and `block_id` are RECORD LINKS (`record<workspaces>` /
//!    `record<loom_blocks>`) with `ASSERT record::exists($value)`, not opaque
//!    strings. The store therefore rejects an outbox row that points at a
//!    workspace or block which does not exist - a foreign-key guarantee the
//!    original got from PostgreSQL and which must not be lost.
//! 2. SurrealDB reports "rows affected" by RETURNING the rows, so every
//!    statement whose affected count matters ends in `RETURN AFTER` and the
//!    count is the returned row count. `mark_published` and `record_failure`
//!    depend on that, because a count of zero is how they detect a lost race.
//! 3. The event body is bound as `serde_json::Value` into an
//!    `object FLEXIBLE` field. The envelope is still hash-verified on read, so
//!    a body that was altered in storage is rejected exactly as before.

use serde_json::Value as JsonValue;
use surrealdb::types::{Datetime, RecordId, RecordIdKey, SurrealValue};
use uuid::Uuid;

use super::surreal::SurrealStorage;
use crate::flight_recorder::FlightRecorderEvent;
use crate::storage::{StorageError, StorageResult};

const OUTBOX_TABLE: &str = "loom_block_view_fr_outbox";
const WORKSPACES_TABLE: &str = "workspaces";
const LOOM_BLOCKS_TABLE: &str = "loom_blocks";

/// The row shape written on insert.
#[derive(SurrealValue)]
struct OutboxCreate {
    event_id: String,
    workspace_id: RecordId,
    block_id: RecordId,
    operation: String,
    event: JsonValue,
    event_hash: String,
    created_at: Datetime,
}

/// The projection read back for decoding and publication decisions.
#[derive(SurrealValue)]
pub(crate) struct OutboxRow {
    pub event_id: String,
    pub workspace_id: RecordId,
    pub event: JsonValue,
    pub event_hash: String,
    pub published_at: Option<Datetime>,
    pub quarantined_at: Option<Datetime>,
}

impl OutboxRow {
    /// The owning workspace as a plain id, for callers that key on strings.
    ///
    /// `RecordIdKey` has no `Display`, and that is useful rather than annoying:
    /// it forces the caller to say which key shape it expects. Workspace ids are
    /// strings throughout Handshake, so a non-string key here means the row was
    /// written by something that does not share that contract, and this reports
    /// it as an invalid record rather than inventing a rendering for it.
    pub(crate) fn workspace_key(&self) -> StorageResult<String> {
        match &self.workspace_id.key {
            RecordIdKey::String(id) => Ok(id.clone()),
            _ => Err(StorageError::Serialization(
                "block-view outbox workspace id is not a string record key".to_owned(),
            )),
        }
    }
}

fn map_err(error: super::surreal::SurrealStorageError) -> StorageError {
    StorageError::Database(error.to_string())
}

// ── bindings ────────────────────────────────────────────────────────────────

#[derive(SurrealValue)]
struct ScopedKey {
    workspace: RecordId,
    event_id: String,
}

#[derive(SurrealValue)]
struct QuarantineBindings {
    workspace: RecordId,
    event_id: String,
    error: String,
    at: Datetime,
}

#[derive(SurrealValue)]
struct FailureBindings {
    workspace: RecordId,
    event_id: String,
    error: String,
    at: Datetime,
}

#[derive(SurrealValue)]
struct PendingBindings {
    workspace: Option<RecordId>,
    event_id: Option<String>,
    limit: i64,
}

// ── operations ──────────────────────────────────────────────────────────────

/// Insert one outbox row.
///
/// The PostgreSQL original ran inside the caller's transaction and failed on a
/// duplicate key. `create_if_absent` preserves that: an id already present
/// yields `Conflict` rather than silently overwriting a row whose event body a
/// publisher may already have read.
pub(crate) async fn store_event(
    storage: &SurrealStorage,
    workspace_id: &str,
    block_id: &str,
    operation: &str,
    event: &FlightRecorderEvent,
    event_hash: String,
) -> StorageResult<()> {
    let body = serde_json::to_value(event)
        .map_err(|error| StorageError::Serialization(error.to_string()))?;
    let content = OutboxCreate {
        event_id: event.event_id.to_string(),
        workspace_id: RecordId::new(WORKSPACES_TABLE, workspace_id),
        block_id: RecordId::new(LOOM_BLOCKS_TABLE, block_id),
        operation: operation.to_owned(),
        event: body,
        event_hash,
        created_at: Datetime::from(event.timestamp),
    };
    let id = event.event_id.to_string();
    let created: Option<OutboxRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move { database.create_if_absent(OUTBOX_TABLE, &id, content).await })
        })
        .await
        .map_err(map_err)?;
    if created.is_none() {
        return Err(StorageError::Conflict(
            "block-view flight-recorder outbox event already exists",
        ));
    }
    Ok(())
}

/// Read one row scoped to its workspace.
pub(crate) async fn load_row(
    storage: &SurrealStorage,
    workspace_id: &str,
    event_id: Uuid,
) -> StorageResult<OutboxRow> {
    let bindings = ScopedKey {
        workspace: RecordId::new(WORKSPACES_TABLE, workspace_id),
        event_id: event_id.to_string(),
    };
    let row: Option<OutboxRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "SELECT event_id, workspace_id, event, event_hash, published_at, \
                         quarantined_at FROM loom_block_view_fr_outbox \
                         WHERE workspace_id = $workspace AND event_id = $event_id;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    row.ok_or(StorageError::NotFound(
        "block-view flight-recorder outbox event",
    ))
}

/// Quarantine a row whose stored envelope failed verification.
pub(crate) async fn quarantine_invalid(
    storage: &SurrealStorage,
    workspace_id: &str,
    event_id: &str,
    error: String,
) -> StorageResult<()> {
    let bindings = QuarantineBindings {
        workspace: RecordId::new(WORKSPACES_TABLE, workspace_id),
        event_id: event_id.to_owned(),
        error,
        at: Datetime::from(chrono::Utc::now()),
    };
    storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .execute_returning(
                        "UPDATE loom_block_view_fr_outbox SET \
                         attempt_count = attempt_count + 1, \
                         last_error = $error, \
                         last_error_at = $at, \
                         quarantined_at = IF quarantined_at = NONE { $at } ELSE { quarantined_at } \
                         WHERE workspace_id = $workspace AND event_id = $event_id \
                         AND published_at = NONE RETURN AFTER;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    Ok(())
}

/// Rows awaiting publication, oldest first.
///
/// `workspace_id` and `event_id` are optional filters; `NONE` means "any",
/// which reproduces the original `$1 IS NULL OR column = $1` predicate.
pub(crate) async fn list_pending_rows(
    storage: &SurrealStorage,
    workspace_id: Option<&str>,
    event_id: Option<Uuid>,
    limit: i64,
) -> StorageResult<Vec<OutboxRow>> {
    let bindings = PendingBindings {
        workspace: workspace_id.map(|id| RecordId::new(WORKSPACES_TABLE, id)),
        event_id: event_id.map(|id| id.to_string()),
        limit: limit.clamp(1, 200),
    };
    storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT event_id, workspace_id, event, event_hash, published_at, \
                         quarantined_at FROM loom_block_view_fr_outbox \
                         WHERE ($workspace = NONE OR workspace_id = $workspace) \
                         AND ($event_id = NONE OR event_id = $event_id) \
                         AND published_at = NONE AND quarantined_at = NONE \
                         ORDER BY created_at ASC, event_id ASC LIMIT $limit;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)
}

/// Mark a row published. Idempotent: an already-published row keeps its
/// original timestamp rather than being pushed forward on a replay.
pub(crate) async fn mark_published(
    storage: &SurrealStorage,
    workspace_id: &str,
    event_id: Uuid,
) -> StorageResult<()> {
    let bindings = QuarantineBindings {
        workspace: RecordId::new(WORKSPACES_TABLE, workspace_id),
        event_id: event_id.to_string(),
        error: String::new(),
        at: Datetime::from(chrono::Utc::now()),
    };
    let affected = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .execute_returning(
                        "UPDATE loom_block_view_fr_outbox SET \
                         published_at = IF published_at = NONE { \
                           IF $at > created_at { $at } ELSE { created_at } \
                         } ELSE { published_at } \
                         WHERE workspace_id = $workspace AND event_id = $event_id RETURN AFTER;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    if affected != 1 {
        return Err(StorageError::NotFound(
            "block-view flight-recorder outbox event",
        ));
    }
    Ok(())
}

/// Record a publication failure against an unpublished row.
///
/// A zero affected count is NOT automatically an error: a concurrent idempotent
/// reconciler may have published the row after this worker observed a recorder
/// error. The original distinguished those two cases with a follow-up EXISTS
/// probe and this keeps that distinction - a row that exists is a benign race,
/// a row that does not is a genuine NotFound.
pub(crate) async fn record_failure(
    storage: &SurrealStorage,
    workspace_id: &str,
    event_id: Uuid,
    error: String,
) -> StorageResult<()> {
    let bindings = FailureBindings {
        workspace: RecordId::new(WORKSPACES_TABLE, workspace_id),
        event_id: event_id.to_string(),
        error,
        at: Datetime::from(chrono::Utc::now()),
    };
    let affected = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .execute_returning(
                        "UPDATE loom_block_view_fr_outbox SET \
                         attempt_count = attempt_count + 1, \
                         last_error = $error, \
                         last_error_at = $at \
                         WHERE workspace_id = $workspace AND event_id = $event_id \
                         AND published_at = NONE RETURN AFTER;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    if affected == 1 {
        return Ok(());
    }
    match load_row(storage, workspace_id, event_id).await {
        Ok(_) => Ok(()),
        Err(StorageError::NotFound(_)) => Err(StorageError::NotFound(
            "block-view flight-recorder outbox event",
        )),
        Err(other) => Err(other),
    }
}

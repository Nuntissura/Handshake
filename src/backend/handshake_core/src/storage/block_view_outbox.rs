//! MT-027 transactional outbox for saved Block Collection View mutations.
//!
//! Durable view state, ProjectKnowledgeIndex/EventLedger authority, search
//! projection, and the exact Flight Recorder event are committed together.
//! Publication into the recorder is an idempotent projection that can resume
//! after a process restart.
//!
//! PENDING SURREALDB PORT (WP-KERNEL-012 MT-136): this module still binds
//! `sqlx` against the deleted relational backend and does not compile today.
//! Handshake's only database is the embedded SurrealDB store.

use crate::flight_recorder::{FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType};
use crate::storage::{MutationMetadata, StorageError, StorageResult, WriteActorKind};
use serde_json::json;
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

const ERROR_MAX_CHARS: usize = 1_000;

pub(crate) enum ScopedPublicationEvent {
    Published,
    Pending(FlightRecorderEvent),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BlockViewMutationOperation {
    Create,
    Update,
}

impl BlockViewMutationOperation {
    fn as_str(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Update => "update",
        }
    }

    fn event_type(self) -> FlightRecorderEventType {
        match self {
            Self::Create => FlightRecorderEventType::LoomBlockCreated,
            Self::Update => FlightRecorderEventType::LoomBlockUpdated,
        }
    }
}

fn actor(metadata: &MutationMetadata) -> (FlightRecorderActor, String) {
    let actor = match metadata.actor_kind {
        WriteActorKind::Human => FlightRecorderActor::Human,
        WriteActorKind::Ai => FlightRecorderActor::Agent,
        WriteActorKind::System => FlightRecorderActor::System,
    };
    let actor_id = metadata
        .actor_id
        .clone()
        .unwrap_or_else(|| actor.to_string());
    (actor, actor_id)
}

pub(crate) fn build_event(
    metadata: &MutationMetadata,
    workspace_id: &str,
    block_id: &str,
    operation: BlockViewMutationOperation,
) -> StorageResult<FlightRecorderEvent> {
    let (actor, actor_id) = actor(metadata);
    let payload = match operation {
        BlockViewMutationOperation::Create => json!({
            "type": "loom_block_created",
            "workspace_id": workspace_id,
            "block_id": block_id,
            "content_type": "view_def",
            "asset_id": null,
            "content_hash": null,
        }),
        BlockViewMutationOperation::Update => json!({
            "type": "loom_block_updated",
            "block_id": block_id,
            "fields_changed": ["view_definition"],
            "updated_by": "user",
        }),
    };
    let mut event = FlightRecorderEvent::new(
        operation.event_type(),
        actor,
        metadata.edit_event_id,
        payload,
    )
    .with_actor_id(actor_id)
    .with_wsids(vec![workspace_id.to_owned()]);
    event.event_id = metadata.edit_event_id;
    event.timestamp = chrono::DateTime::from_timestamp_micros(
        metadata.timestamp.timestamp_micros(),
    )
    .ok_or_else(|| StorageError::Serialization("invalid block-view event time".to_owned()))?;
    // DuckDB normalizes these fields before persistence. Normalize the
    // PostgreSQL authority copy before hashing so a crash after recorder
    // insertion but before publish acknowledgement remains idempotent.
    event.normalize_payload();
    event
        .validate()
        .map_err(|error| StorageError::Serialization(error.to_string()))?;
    Ok(event)
}

fn event_hash(event: &FlightRecorderEvent) -> StorageResult<String> {
    let value = serde_json::to_value(event)
        .map_err(|error| StorageError::Serialization(error.to_string()))?;
    Ok(crate::kernel::context_bundle::sha256_hex(
        &crate::kernel::context_bundle::canonical_json_bytes(&value),
    ))
}

fn same_event(left: &FlightRecorderEvent, right: &FlightRecorderEvent) -> bool {
    left.event_id == right.event_id
        && left.trace_id == right.trace_id
        && left.timestamp.timestamp_micros() == right.timestamp.timestamp_micros()
        && left.actor == right.actor
        && left.actor_id == right.actor_id
        && left.event_type == right.event_type
        && left.job_id == right.job_id
        && left.workflow_id == right.workflow_id
        && left.model_id == right.model_id
        && left.model_session_id == right.model_session_id
        && left.wsids == right.wsids
        && left.activity_span_id == right.activity_span_id
        && left.session_span_id == right.session_span_id
        && left.capability_id == right.capability_id
        && left.policy_decision_id == right.policy_decision_id
        && left.payload == right.payload
}

pub(crate) async fn store_event(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: &str,
    block_id: &str,
    operation: BlockViewMutationOperation,
    event: &FlightRecorderEvent,
) -> StorageResult<()> {
    let serialized = serde_json::to_string(event)
        .map_err(|error| StorageError::Serialization(error.to_string()))?;
    let hash = event_hash(event)?;
    sqlx::query(
        r#"
        INSERT INTO loom_block_view_fr_outbox
            (event_id, workspace_id, block_id, operation, event, event_hash, created_at)
        VALUES ($1, $2, $3, $4, $5::jsonb, $6, $7)
        "#,
    )
    .bind(event.event_id.to_string())
    .bind(workspace_id)
    .bind(block_id)
    .bind(operation.as_str())
    .bind(serialized)
    .bind(hash)
    .bind(event.timestamp)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

fn bounded_error(error: &str) -> String {
    error.chars().take(ERROR_MAX_CHARS).collect()
}

async fn quarantine_invalid(
    pool: &PgPool,
    workspace_id: &str,
    event_id: &str,
    error: &str,
) -> StorageResult<()> {
    sqlx::query(
        r#"
        UPDATE loom_block_view_fr_outbox
        SET attempt_count = attempt_count + 1,
            last_error = $3,
            last_error_at = now(),
            quarantined_at = COALESCE(quarantined_at, now())
        WHERE workspace_id = $1 AND event_id = $2 AND published_at IS NULL
        "#,
    )
    .bind(workspace_id)
    .bind(event_id)
    .bind(bounded_error(error))
    .execute(pool)
    .await?;
    Ok(())
}

fn decode_event_row(row: &sqlx::postgres::PgRow) -> StorageResult<FlightRecorderEvent> {
    let event_id: String = row.try_get("event_id")?;
    let event_text: String = row.try_get("event")?;
    let stored_hash: String = row.try_get("event_hash")?;
    let event: FlightRecorderEvent = serde_json::from_str(&event_text)
        .map_err(|error| StorageError::Serialization(error.to_string()))?;
    let expected_id = Uuid::parse_str(&event_id)
        .map_err(|error| StorageError::Serialization(error.to_string()))?;
    if event.event_id != expected_id || event_hash(&event)? != stored_hash {
        return Err(StorageError::Conflict(
            "block-view outbox event hash or identity does not match its envelope",
        ));
    }
    event
        .validate()
        .map_err(|error| StorageError::Serialization(error.to_string()))?;
    Ok(event)
}

pub(crate) async fn load_scoped_publication(
    pool: &PgPool,
    workspace_id: &str,
    event_id: Uuid,
) -> StorageResult<ScopedPublicationEvent> {
    let event_id = event_id.to_string();
    let row = sqlx::query(
        r#"
        SELECT event_id, event::text AS event, event_hash, published_at, quarantined_at
        FROM loom_block_view_fr_outbox
        WHERE workspace_id = $1 AND event_id = $2
        "#,
    )
    .bind(workspace_id)
    .bind(&event_id)
    .fetch_optional(pool)
    .await?
    .ok_or(StorageError::NotFound(
        "block-view flight-recorder outbox event",
    ))?;

    if row
        .try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("published_at")?
        .is_some()
    {
        return Ok(ScopedPublicationEvent::Published);
    }
    if row
        .try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("quarantined_at")?
        .is_some()
    {
        return Err(StorageError::Conflict(
            "block_view_flight_event_quarantined",
        ));
    }

    match decode_event_row(&row) {
        Ok(event) => Ok(ScopedPublicationEvent::Pending(event)),
        Err(error) => {
            quarantine_invalid(pool, workspace_id, &event_id, &error.to_string()).await?;
            Err(error)
        }
    }
}

pub(crate) async fn list_pending(
    pool: &PgPool,
    workspace_id: Option<&str>,
    event_id: Option<Uuid>,
    limit: i64,
) -> StorageResult<Vec<(String, FlightRecorderEvent)>> {
    let event_id = event_id.map(|value| value.to_string());
    let rows = sqlx::query(
        r#"
        SELECT workspace_id, event_id, event::text AS event, event_hash
        FROM loom_block_view_fr_outbox
        WHERE ($1::text IS NULL OR workspace_id = $1)
          AND ($2::text IS NULL OR event_id = $2)
          AND published_at IS NULL
          AND quarantined_at IS NULL
        ORDER BY created_at ASC, event_id ASC
        LIMIT $3
        "#,
    )
    .bind(workspace_id)
    .bind(event_id)
    .bind(limit.clamp(1, 200))
    .fetch_all(pool)
    .await?;

    let mut events = Vec::with_capacity(rows.len());
    for row in rows {
        let workspace_id: String = row.try_get("workspace_id")?;
        let event_id: String = row.try_get("event_id")?;
        let decoded = decode_event_row(&row);
        match decoded {
            Ok(event) => events.push((workspace_id, event)),
            Err(error) => {
                quarantine_invalid(pool, &workspace_id, &event_id, &error.to_string()).await?;
            }
        }
    }
    Ok(events)
}

pub(crate) async fn mark_published(
    pool: &PgPool,
    workspace_id: &str,
    event_id: Uuid,
) -> StorageResult<()> {
    let result = sqlx::query(
        r#"
        UPDATE loom_block_view_fr_outbox
        SET published_at = COALESCE(published_at, GREATEST(now(), created_at))
        WHERE workspace_id = $1 AND event_id = $2
        "#,
    )
    .bind(workspace_id)
    .bind(event_id.to_string())
    .execute(pool)
    .await?;
    if result.rows_affected() != 1 {
        return Err(StorageError::NotFound(
            "block-view flight-recorder outbox event",
        ));
    }
    Ok(())
}

pub(crate) async fn record_failure(
    pool: &PgPool,
    workspace_id: &str,
    event_id: Uuid,
    error: &str,
) -> StorageResult<()> {
    let result = sqlx::query(
        r#"
        UPDATE loom_block_view_fr_outbox
        SET attempt_count = attempt_count + 1,
            last_error = $3,
            last_error_at = now()
        WHERE workspace_id = $1 AND event_id = $2 AND published_at IS NULL
        "#,
    )
    .bind(workspace_id)
    .bind(event_id.to_string())
    .bind(bounded_error(error))
    .execute(pool)
    .await?;
    if result.rows_affected() != 1 {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(
                SELECT 1 FROM loom_block_view_fr_outbox
                WHERE workspace_id = $1 AND event_id = $2
            )",
        )
        .bind(workspace_id)
        .bind(event_id.to_string())
        .fetch_one(pool)
        .await?;
        if exists {
            // A concurrent idempotent reconciler may have published the row
            // after this worker observed a recorder error.
            return Ok(());
        }
        return Err(StorageError::NotFound(
            "block-view flight-recorder outbox event",
        ));
    }
    Ok(())
}

pub(crate) fn events_equal(left: &FlightRecorderEvent, right: &FlightRecorderEvent) -> bool {
    same_event(left, right)
}

//! MT-027 transactional outbox for saved Block Collection View mutations.
//!
//! Durable view state, ProjectKnowledgeIndex/EventLedger authority, search
//! projection, and the exact Flight Recorder event are committed together.
//! Publication into the recorder is an idempotent projection that can resume
//! after a process restart.
//!
//! Persistence lives in [`super::block_view_outbox_surreal`]; this module owns
//! the parts that are not storage-specific - building the Flight Recorder
//! event, hashing it, and verifying a stored envelope still matches its
//! identity and hash on the way back out.

use super::block_view_outbox_surreal::{self as surreal_outbox, OutboxRow};
use super::surreal::SurrealStorage;
use crate::flight_recorder::{FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType};
use crate::storage::{MutationMetadata, StorageError, StorageResult, WriteActorKind};
use serde_json::json;
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
    // SurrealDB authority copy before hashing so a crash after recorder
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

/// Verify a stored envelope and decode it.
///
/// The hash and the event id are re-checked on every read: a row whose body was
/// altered in storage, or whose id no longer matches the envelope it carries,
/// is a Conflict rather than something to publish. This is the check that makes
/// the outbox safe to resume after a crash.
fn decode_row(row: &OutboxRow) -> StorageResult<FlightRecorderEvent> {
    let event: FlightRecorderEvent = serde_json::from_value(row.event.clone())
        .map_err(|error| StorageError::Serialization(error.to_string()))?;
    let expected_id = Uuid::parse_str(&row.event_id)
        .map_err(|error| StorageError::Serialization(error.to_string()))?;
    if event.event_id != expected_id || event_hash(&event)? != row.event_hash {
        return Err(StorageError::Conflict(
            "block-view outbox event hash or identity does not match its envelope",
        ));
    }
    event
        .validate()
        .map_err(|error| StorageError::Serialization(error.to_string()))?;
    Ok(event)
}

fn bounded_error(error: &str) -> String {
    error.chars().take(ERROR_MAX_CHARS).collect()
}

/// Write one outbox row.
pub(crate) async fn store_event(
    storage: &SurrealStorage,
    workspace_id: &str,
    block_id: &str,
    operation: BlockViewMutationOperation,
    event: &FlightRecorderEvent,
) -> StorageResult<()> {
    let hash = event_hash(event)?;
    surreal_outbox::store_event(
        storage,
        workspace_id,
        block_id,
        operation.as_str(),
        event,
        hash,
    )
    .await
}

/// Read one event, deciding whether it still needs publishing.
///
/// A row that fails verification is quarantined before the error is returned,
/// so a poisoned envelope is taken out of the publication path instead of being
/// retried forever.
pub(crate) async fn load_scoped_publication(
    storage: &SurrealStorage,
    workspace_id: &str,
    event_id: Uuid,
) -> StorageResult<ScopedPublicationEvent> {
    let row = surreal_outbox::load_row(storage, workspace_id, event_id).await?;
    if row.published_at.is_some() {
        return Ok(ScopedPublicationEvent::Published);
    }
    if row.quarantined_at.is_some() {
        return Err(StorageError::Conflict(
            "block_view_flight_event_quarantined",
        ));
    }
    match decode_row(&row) {
        Ok(event) => Ok(ScopedPublicationEvent::Pending(event)),
        Err(error) => {
            surreal_outbox::quarantine_invalid(
                storage,
                workspace_id,
                &row.event_id,
                bounded_error(&error.to_string()),
            )
            .await?;
            Err(error)
        }
    }
}

/// Events still awaiting publication, oldest first.
pub(crate) async fn list_pending(
    storage: &SurrealStorage,
    workspace_id: Option<&str>,
    event_id: Option<Uuid>,
    limit: i64,
) -> StorageResult<Vec<(String, FlightRecorderEvent)>> {
    let rows = surreal_outbox::list_pending_rows(storage, workspace_id, event_id, limit).await?;
    let mut events = Vec::with_capacity(rows.len());
    for row in rows {
        let owner = row.workspace_key()?;
        match decode_row(&row) {
            Ok(event) => events.push((owner, event)),
            Err(error) => {
                surreal_outbox::quarantine_invalid(
                    storage,
                    &owner,
                    &row.event_id,
                    bounded_error(&error.to_string()),
                )
                .await?;
            }
        }
    }
    Ok(events)
}

/// Mark an event published. Idempotent on replay.
pub(crate) async fn mark_published(
    storage: &SurrealStorage,
    workspace_id: &str,
    event_id: Uuid,
) -> StorageResult<()> {
    surreal_outbox::mark_published(storage, workspace_id, event_id).await
}

/// Record a publication failure, tolerating the benign already-published race.
pub(crate) async fn record_failure(
    storage: &SurrealStorage,
    workspace_id: &str,
    event_id: Uuid,
    error: &str,
) -> StorageResult<()> {
    surreal_outbox::record_failure(storage, workspace_id, event_id, bounded_error(error)).await
}

pub(crate) fn events_equal(left: &FlightRecorderEvent, right: &FlightRecorderEvent) -> bool {
    same_event(left, right)
}

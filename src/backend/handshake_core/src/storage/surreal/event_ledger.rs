//! Embedded EventLedger primitives shared by storage-domain transactions.

use surrealdb::types::{Datetime, RecordId, SurrealValue};

use super::SurrealStorage;
use crate::kernel::{KernelActor, KernelEvent, KernelEventType, NewKernelEvent};
use crate::storage::{StorageError, StorageResult};

const EVENT_TABLE: &str = "kernel_event_ledger";

#[derive(Clone, SurrealValue)]
pub(crate) struct LedgerWrite {
    pub(crate) record: RecordId,
    pub(crate) event_id: String,
    pub(crate) event_version: String,
    pub(crate) kernel_task_run_id: String,
    pub(crate) session_run_id: String,
    pub(crate) aggregate_type: String,
    pub(crate) aggregate_id: String,
    pub(crate) idempotency_key: String,
    pub(crate) event_type: String,
    pub(crate) actor_kind: String,
    pub(crate) actor_id: String,
    pub(crate) causation_id: Option<String>,
    pub(crate) correlation_id: Option<String>,
    pub(crate) payload_hash: String,
    pub(crate) source_component: String,
    pub(crate) payload: serde_json::Value,
    pub(crate) created_at: Datetime,
}

#[derive(SurrealValue)]
struct EventBindings {
    event: LedgerWrite,
}

#[derive(SurrealValue)]
struct EventBatchBindings {
    events: Vec<LedgerBulkInsert>,
    idempotency_keys: Vec<String>,
}

#[derive(SurrealValue)]
struct LedgerBulkInsert {
    id: RecordId,
    event_id: String,
    event_version: String,
    kernel_task_run_id: String,
    session_run_id: String,
    aggregate_type: String,
    aggregate_id: String,
    idempotency_key: String,
    event_type: String,
    actor_kind: String,
    actor_id: String,
    causation_id: Option<String>,
    correlation_id: Option<String>,
    payload_hash: String,
    source_component: String,
    payload: serde_json::Value,
    created_at: Datetime,
}

impl From<LedgerWrite> for LedgerBulkInsert {
    fn from(write: LedgerWrite) -> Self {
        Self {
            id: write.record,
            event_id: write.event_id,
            event_version: write.event_version,
            kernel_task_run_id: write.kernel_task_run_id,
            session_run_id: write.session_run_id,
            aggregate_type: write.aggregate_type,
            aggregate_id: write.aggregate_id,
            idempotency_key: write.idempotency_key,
            event_type: write.event_type,
            actor_kind: write.actor_kind,
            actor_id: write.actor_id,
            causation_id: write.causation_id,
            correlation_id: write.correlation_id,
            payload_hash: write.payload_hash,
            source_component: write.source_component,
            payload: write.payload,
            created_at: write.created_at,
        }
    }
}

#[derive(SurrealValue)]
struct EventPairBindings {
    first: LedgerWrite,
    second: LedgerWrite,
    idempotency_keys: Vec<String>,
}

#[derive(SurrealValue)]
struct EventLookupBindings {
    value: String,
}

#[derive(SurrealValue)]
struct PendingMirrorBindings {
    pending_type: String,
    completed_type: String,
    after_sequence: i64,
}

#[derive(Clone, SurrealValue)]
struct LedgerRow {
    event_id: String,
    event_sequence: i64,
    event_version: String,
    kernel_task_run_id: String,
    session_run_id: String,
    aggregate_type: String,
    aggregate_id: String,
    idempotency_key: String,
    event_type: String,
    actor_kind: String,
    actor_id: String,
    causation_id: Option<String>,
    correlation_id: Option<String>,
    payload_hash: String,
    source_component: String,
    payload: serde_json::Value,
    created_at: Datetime,
}

pub(crate) fn prepare_event(event: NewKernelEvent) -> StorageResult<(KernelEvent, LedgerWrite)> {
    event
        .validate()
        .map_err(|_| StorageError::Validation("invalid kernel event"))?;
    let stored = KernelEvent::from_new(event.clone());
    let write = LedgerWrite {
        record: RecordId::new(EVENT_TABLE, stored.event_id.clone()),
        event_id: stored.event_id.clone(),
        event_version: event.event_version,
        kernel_task_run_id: event.kernel_task_run_id,
        session_run_id: event.session_run_id,
        aggregate_type: event.aggregate_type,
        aggregate_id: event.aggregate_id,
        idempotency_key: event.idempotency_key,
        event_type: event.event_type.as_str().to_owned(),
        actor_kind: event.actor.actor_kind().to_owned(),
        actor_id: event.actor.actor_id().to_owned(),
        causation_id: event.causation_id,
        correlation_id: event.correlation_id,
        payload_hash: event.payload_hash,
        source_component: event.source_component,
        payload: event.payload,
        created_at: Datetime::from(stored.created_at),
    };
    Ok((stored, write))
}

pub(crate) async fn append(
    storage: &SurrealStorage,
    event: NewKernelEvent,
) -> StorageResult<KernelEvent> {
    let (candidate, write) = prepare_event(event)?;
    let idempotency_key = candidate.idempotency_key.clone();
    let result: Result<Option<LedgerRow>, _> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "IF (SELECT VALUE id FROM kernel_event_ledger \
                             WHERE idempotency_key = $event.idempotency_key LIMIT 1)[0] != NONE { \
                             RETURN SELECT event_id, event_sequence, event_version, kernel_task_run_id, \
                                 session_run_id, aggregate_type, aggregate_id, idempotency_key, event_type, \
                                 actor_kind, actor_id, causation_id, correlation_id, payload_hash, \
                                 source_component, payload, created_at FROM kernel_event_ledger \
                                 WHERE idempotency_key = $event.idempotency_key LIMIT 1; \
                         } ELSE { \
                             RETURN CREATE $event.record CONTENT { \
                                 event_id: $event.event_id, event_version: $event.event_version, \
                                 kernel_task_run_id: $event.kernel_task_run_id, \
                                 session_run_id: $event.session_run_id, aggregate_type: $event.aggregate_type, \
                                 aggregate_id: $event.aggregate_id, idempotency_key: $event.idempotency_key, \
                                 event_type: $event.event_type, actor_kind: $event.actor_kind, \
                                 actor_id: $event.actor_id, causation_id: $event.causation_id, \
                                 correlation_id: $event.correlation_id, payload_hash: $event.payload_hash, \
                                 source_component: $event.source_component, payload: $event.payload, \
                                 created_at: $event.created_at \
                             }; \
                         };",
                        EventBindings { event: write },
                    )
                    .await
            })
        })
        .await;
    let row = match result {
        Ok(row) => row,
        Err(error) => {
            // A concurrent exact replay can lose the unique-index race after
            // both callers observe the idempotency key as absent. Re-read the
            // winner and accept it only when every immutable event dimension
            // matches; otherwise retain the original database failure.
            if let Some(stored) = get_by_idempotency(storage, &idempotency_key).await? {
                ensure_same_event(&stored, &candidate)?;
                return Ok(stored);
            }
            return Err(StorageError::from(error));
        }
    };
    let stored = row
        .map(row_to_event)
        .transpose()?
        .ok_or_else(|| StorageError::Database("EventLedger append returned no row".to_owned()))?;
    ensure_same_event(&stored, &candidate)?;
    Ok(stored)
}

/// Appends a batch as one canonical SurrealDB EventLedger transaction. An exact
/// idempotent replay returns the original stored event, while any immutable-
/// content mismatch aborts the entire batch.
pub(crate) async fn append_atomic(
    storage: &SurrealStorage,
    events: Vec<NewKernelEvent>,
) -> StorageResult<Vec<KernelEvent>> {
    if events.is_empty() {
        return Ok(Vec::new());
    }

    let mut candidates = Vec::with_capacity(events.len());
    let mut writes = Vec::with_capacity(events.len());
    let mut idempotency_keys = Vec::with_capacity(events.len());
    for event in events {
        let (candidate, write) = prepare_event(event)?;
        idempotency_keys.push(candidate.idempotency_key.clone());
        candidates.push(candidate);
        writes.push(write);
    }

    let existing_rows = read_by_idempotency_keys(storage, idempotency_keys.clone()).await?;
    let existing_by_key = existing_rows
        .iter()
        .map(|row| (row.idempotency_key.as_str(), row))
        .collect::<std::collections::HashMap<_, _>>();
    let mut first_candidate_by_key = std::collections::HashMap::new();
    let mut inserts = Vec::with_capacity(writes.len());
    for (index, (candidate, write)) in candidates.iter().zip(writes).enumerate() {
        if let Some(first_index) = first_candidate_by_key.get(&candidate.idempotency_key) {
            ensure_same_event(&candidates[*first_index], candidate)?;
            continue;
        }
        first_candidate_by_key.insert(candidate.idempotency_key.clone(), index);
        if let Some(row) = existing_by_key.get(candidate.idempotency_key.as_str()) {
            ensure_same_event(&row_to_event((*row).clone())?, candidate)?;
        } else {
            inserts.push(LedgerBulkInsert::from(write));
        }
    }
    drop(existing_by_key);

    if inserts.is_empty() {
        return order_and_validate(existing_rows, &candidates);
    }

    let bindings = EventBatchBindings {
        events: inserts,
        idempotency_keys,
    };
    let result: Result<Vec<LedgerRow>, _> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at(
                        "BEGIN TRANSACTION; \
                         INSERT INTO kernel_event_ledger $events RETURN NONE; \
                         COMMIT TRANSACTION; \
                         SELECT event_id, event_sequence, event_version, kernel_task_run_id, \
                           session_run_id, aggregate_type, aggregate_id, idempotency_key, event_type, \
                           actor_kind, actor_id, causation_id, correlation_id, payload_hash, \
                           source_component, payload, created_at FROM kernel_event_ledger \
                           WHERE idempotency_key IN $idempotency_keys;",
                        bindings,
                        3,
                    )
                    .await
            })
        })
        .await;

    match result {
        Ok(rows) => order_and_validate(rows, &candidates),
        Err(error) if is_idempotency_conflict(&error.to_string()) => Err(idempotency_conflict()),
        Err(error) => {
            // A concurrent exact replay may win a unique-index race. Match the
            // single-append contract by accepting only a complete, exact
            // winner set; a partial or conflicting set retains the failure.
            if let Some(stored) = read_and_validate(storage, &candidates).await? {
                return Ok(stored);
            }
            Err(StorageError::from(error))
        }
    }
}

/// Atomically appends two events and binds the second event's causation to
/// the actual stored first event. This matters when the first event is an
/// idempotent replay whose durable event id differs from the fresh candidate.
pub(crate) async fn append_pair_atomic_with_causation(
    storage: &SurrealStorage,
    first: NewKernelEvent,
    mut second: NewKernelEvent,
) -> StorageResult<Vec<KernelEvent>> {
    let (first_candidate, first_write) = prepare_event(first)?;
    // Validate the second event with a syntactically valid ledger id before
    // the transaction. The transaction replaces this provisional causation
    // value with the actual stored first event id and validates replay content
    // against that same durable id.
    second.causation_id = Some(first_candidate.event_id.clone());
    let (second_candidate, second_write) = prepare_event(second)?;
    let candidates = [first_candidate, second_candidate];
    let bindings = EventPairBindings {
        idempotency_keys: candidates
            .iter()
            .map(|event| event.idempotency_key.clone())
            .collect(),
        first: first_write,
        second: second_write,
    };
    let result: Result<Vec<LedgerRow>, _> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at(
                        "BEGIN TRANSACTION; \
                         LET $first_stored = (SELECT event_id, event_sequence, event_version, \
                           kernel_task_run_id, session_run_id, aggregate_type, aggregate_id, \
                           idempotency_key, event_type, actor_kind, actor_id, causation_id, \
                           correlation_id, payload_hash, source_component, payload, created_at \
                           FROM kernel_event_ledger \
                           WHERE idempotency_key = $first.idempotency_key LIMIT 1)[0]; \
                         IF $first_stored != NONE { \
                           IF $first_stored.event_version != $first.event_version \
                              OR $first_stored.kernel_task_run_id != $first.kernel_task_run_id \
                              OR $first_stored.session_run_id != $first.session_run_id \
                              OR $first_stored.aggregate_type != $first.aggregate_type \
                              OR $first_stored.aggregate_id != $first.aggregate_id \
                              OR $first_stored.event_type != $first.event_type \
                              OR $first_stored.actor_kind != $first.actor_kind \
                              OR $first_stored.actor_id != $first.actor_id \
                              OR $first_stored.causation_id != $first.causation_id \
                              OR $first_stored.correlation_id != $first.correlation_id \
                              OR $first_stored.payload_hash != $first.payload_hash \
                              OR $first_stored.source_component != $first.source_component { \
                             THROW 'HSK-EVENT-LEDGER-IDEMPOTENCY-CONFLICT'; \
                           }; \
                         } ELSE { \
                           CREATE $first.record CONTENT { \
                             event_id: $first.event_id, event_version: $first.event_version, \
                             kernel_task_run_id: $first.kernel_task_run_id, \
                             session_run_id: $first.session_run_id, aggregate_type: $first.aggregate_type, \
                             aggregate_id: $first.aggregate_id, idempotency_key: $first.idempotency_key, \
                             event_type: $first.event_type, actor_kind: $first.actor_kind, \
                             actor_id: $first.actor_id, causation_id: $first.causation_id, \
                             correlation_id: $first.correlation_id, payload_hash: $first.payload_hash, \
                             source_component: $first.source_component, payload: $first.payload, \
                             created_at: $first.created_at \
                           } RETURN NONE; \
                         }; \
                         LET $actual_first = (SELECT event_id FROM kernel_event_ledger \
                           WHERE idempotency_key = $first.idempotency_key LIMIT 1)[0]; \
                         LET $second_stored = (SELECT event_id, event_sequence, event_version, \
                           kernel_task_run_id, session_run_id, aggregate_type, aggregate_id, \
                           idempotency_key, event_type, actor_kind, actor_id, causation_id, \
                           correlation_id, payload_hash, source_component, payload, created_at \
                           FROM kernel_event_ledger \
                           WHERE idempotency_key = $second.idempotency_key LIMIT 1)[0]; \
                         IF $second_stored != NONE { \
                           IF $second_stored.event_version != $second.event_version \
                              OR $second_stored.kernel_task_run_id != $second.kernel_task_run_id \
                              OR $second_stored.session_run_id != $second.session_run_id \
                              OR $second_stored.aggregate_type != $second.aggregate_type \
                              OR $second_stored.aggregate_id != $second.aggregate_id \
                              OR $second_stored.event_type != $second.event_type \
                              OR $second_stored.actor_kind != $second.actor_kind \
                              OR $second_stored.actor_id != $second.actor_id \
                              OR $second_stored.causation_id != $actual_first.event_id \
                              OR $second_stored.correlation_id != $second.correlation_id \
                              OR $second_stored.payload_hash != $second.payload_hash \
                              OR $second_stored.source_component != $second.source_component { \
                             THROW 'HSK-EVENT-LEDGER-IDEMPOTENCY-CONFLICT'; \
                           }; \
                         } ELSE { \
                           CREATE $second.record CONTENT { \
                             event_id: $second.event_id, event_version: $second.event_version, \
                             kernel_task_run_id: $second.kernel_task_run_id, \
                             session_run_id: $second.session_run_id, aggregate_type: $second.aggregate_type, \
                             aggregate_id: $second.aggregate_id, idempotency_key: $second.idempotency_key, \
                             event_type: $second.event_type, actor_kind: $second.actor_kind, \
                             actor_id: $second.actor_id, causation_id: $actual_first.event_id, \
                             correlation_id: $second.correlation_id, payload_hash: $second.payload_hash, \
                             source_component: $second.source_component, payload: $second.payload, \
                             created_at: $second.created_at \
                           } RETURN NONE; \
                         }; \
                         COMMIT TRANSACTION; \
                         SELECT event_id, event_sequence, event_version, kernel_task_run_id, \
                           session_run_id, aggregate_type, aggregate_id, idempotency_key, event_type, \
                           actor_kind, actor_id, causation_id, correlation_id, payload_hash, \
                           source_component, payload, created_at FROM kernel_event_ledger \
                           WHERE idempotency_key IN $idempotency_keys;",
                        bindings,
                        7,
                    )
                    .await
            })
        })
        .await;

    match result {
        Ok(rows) => order_pair_and_validate(rows, &candidates),
        Err(error) if is_idempotency_conflict(&error.to_string()) => Err(idempotency_conflict()),
        Err(error) => {
            if let Some(stored) = read_pair_and_validate(storage, &candidates).await? {
                return Ok(stored);
            }
            Err(StorageError::from(error))
        }
    }
}

async fn read_and_validate(
    storage: &SurrealStorage,
    candidates: &[KernelEvent],
) -> StorageResult<Option<Vec<KernelEvent>>> {
    let keys = candidates
        .iter()
        .map(|event| event.idempotency_key.clone())
        .collect();
    let rows = read_by_idempotency_keys(storage, keys).await?;
    let expected_count = candidates
        .iter()
        .map(|event| event.idempotency_key.as_str())
        .collect::<std::collections::HashSet<_>>()
        .len();
    if rows.len() < expected_count {
        return Ok(None);
    }
    order_and_validate(rows, candidates).map(Some)
}

async fn read_pair_and_validate(
    storage: &SurrealStorage,
    candidates: &[KernelEvent; 2],
) -> StorageResult<Option<Vec<KernelEvent>>> {
    let keys = candidates
        .iter()
        .map(|event| event.idempotency_key.clone())
        .collect();
    let rows = read_by_idempotency_keys(storage, keys).await?;
    let expected_count = candidates
        .iter()
        .map(|event| event.idempotency_key.as_str())
        .collect::<std::collections::HashSet<_>>()
        .len();
    if rows.len() < expected_count {
        return Ok(None);
    }
    order_pair_and_validate(rows, candidates).map(Some)
}

async fn read_by_idempotency_keys(
    storage: &SurrealStorage,
    idempotency_keys: Vec<String>,
) -> StorageResult<Vec<LedgerRow>> {
    #[derive(SurrealValue)]
    struct IdempotencyKeysBindings {
        idempotency_keys: Vec<String>,
    }

    storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT event_id, event_sequence, event_version, kernel_task_run_id, \
                           session_run_id, aggregate_type, aggregate_id, idempotency_key, event_type, \
                           actor_kind, actor_id, causation_id, correlation_id, payload_hash, \
                           source_component, payload, created_at FROM kernel_event_ledger \
                           WHERE idempotency_key IN $idempotency_keys;",
                        IdempotencyKeysBindings { idempotency_keys },
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)
}

fn order_and_validate(
    rows: Vec<LedgerRow>,
    candidates: &[KernelEvent],
) -> StorageResult<Vec<KernelEvent>> {
    let mut stored_by_key = std::collections::HashMap::with_capacity(rows.len());
    for row in rows {
        let event = row_to_event(row)?;
        stored_by_key.insert(event.idempotency_key.clone(), event);
    }
    candidates
        .iter()
        .map(|candidate| {
            let stored = stored_by_key
                .get(&candidate.idempotency_key)
                .ok_or_else(|| {
                    StorageError::Database(
                        "atomic EventLedger append returned an incomplete result set".to_owned(),
                    )
                })?
                .clone();
            ensure_same_event(&stored, candidate)?;
            Ok(stored)
        })
        .collect()
}

fn order_pair_and_validate(
    rows: Vec<LedgerRow>,
    candidates: &[KernelEvent; 2],
) -> StorageResult<Vec<KernelEvent>> {
    let mut stored_by_key = std::collections::HashMap::with_capacity(rows.len());
    for row in rows {
        let event = row_to_event(row)?;
        stored_by_key.insert(event.idempotency_key.clone(), event);
    }
    let first = stored_by_key
        .get(&candidates[0].idempotency_key)
        .ok_or_else(|| {
            StorageError::Database(
                "atomic EventLedger pair append returned no first event".to_owned(),
            )
        })?
        .clone();
    ensure_same_event(&first, &candidates[0])?;

    let second = stored_by_key
        .get(&candidates[1].idempotency_key)
        .ok_or_else(|| {
            StorageError::Database(
                "atomic EventLedger pair append returned no second event".to_owned(),
            )
        })?
        .clone();
    let mut expected_second = candidates[1].clone();
    expected_second.causation_id = Some(first.event_id.clone());
    ensure_same_event(&second, &expected_second)?;
    Ok(vec![first, second])
}

fn is_idempotency_conflict(error: &str) -> bool {
    error.contains("HSK-EVENT-LEDGER-IDEMPOTENCY-CONFLICT")
}

fn idempotency_conflict() -> StorageError {
    StorageError::Conflict("kernel event idempotency key was reused with different event content")
}

pub(crate) async fn list_for_session(
    storage: &SurrealStorage,
    session_run_id: &str,
) -> StorageResult<Vec<KernelEvent>> {
    list(
        storage,
        "SELECT event_id, event_sequence, event_version, kernel_task_run_id, session_run_id, \
         aggregate_type, aggregate_id, idempotency_key, event_type, actor_kind, actor_id, \
         causation_id, correlation_id, payload_hash, source_component, payload, created_at \
         FROM kernel_event_ledger WHERE session_run_id = $value ORDER BY event_sequence ASC;",
        session_run_id,
    )
    .await
}

pub(crate) async fn list_for_aggregate(
    storage: &SurrealStorage,
    aggregate_type: &str,
    aggregate_id: &str,
) -> StorageResult<Vec<KernelEvent>> {
    #[derive(SurrealValue)]
    struct AggregateBindings {
        aggregate_type: String,
        aggregate_id: String,
    }
    let bindings = AggregateBindings {
        aggregate_type: aggregate_type.to_owned(),
        aggregate_id: aggregate_id.to_owned(),
    };
    let rows: Vec<LedgerRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT event_id, event_sequence, event_version, kernel_task_run_id, session_run_id, \
                         aggregate_type, aggregate_id, idempotency_key, event_type, actor_kind, actor_id, \
                         causation_id, correlation_id, payload_hash, source_component, payload, created_at \
                         FROM kernel_event_ledger WHERE aggregate_type = $aggregate_type \
                         AND aggregate_id = $aggregate_id ORDER BY event_sequence ASC;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    rows.into_iter().map(row_to_event).collect()
}

pub(crate) async fn list_pending_native_editor_mirrors(
    storage: &SurrealStorage,
    after_event_sequence: i64,
    limit: i64,
) -> StorageResult<Vec<KernelEvent>> {
    let bindings = PendingMirrorBindings {
        pending_type: KernelEventType::FlightRecorderMirrorPending
            .as_str()
            .to_owned(),
        completed_type: KernelEventType::FlightRecorderMirrorRecorded
            .as_str()
            .to_owned(),
        after_sequence: after_event_sequence.max(0),
    };
    let rows: Vec<LedgerRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT event_id, event_sequence, event_version, kernel_task_run_id, session_run_id, \
                         aggregate_type, aggregate_id, idempotency_key, event_type, actor_kind, actor_id, \
                         causation_id, correlation_id, payload_hash, source_component, payload, created_at \
                         FROM kernel_event_ledger WHERE \
                           (event_type = $pending_type AND aggregate_type = 'native_editor_event' \
                            AND event_sequence > $after_sequence) \
                           OR event_type = $completed_type \
                         ORDER BY event_sequence ASC;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;

    let mut pending = Vec::new();
    let mut completed = Vec::new();
    for row in rows {
        let event = row_to_event(row)?;
        if event.event_type == KernelEventType::FlightRecorderMirrorPending {
            pending.push(event);
        } else if event.event_type == KernelEventType::FlightRecorderMirrorRecorded {
            completed.push(event);
        }
    }

    let limit = usize::try_from(limit.clamp(1, 1_000)).unwrap_or(1_000);
    pending.retain(|candidate| {
        !completed
            .iter()
            .any(|receipt| native_editor_completion_matches(candidate, receipt))
    });
    pending.truncate(limit);
    Ok(pending)
}

fn native_editor_completion_matches(pending: &KernelEvent, completed: &KernelEvent) -> bool {
    let Some(expected_hash) = pending
        .payload
        .get("expected_completion_payload_hash")
        .and_then(serde_json::Value::as_str)
    else {
        // Legacy pending receipts deliberately remain visible so the reconciler
        // can revalidate them without rewriting append-only EventLedger rows.
        return false;
    };
    let expected_payload = serde_json::json!({
        "receipt_kind": "native_editor_flight_recorder_recorded",
        "fr_event_id": pending.aggregate_id,
        "fr_event_type": "system",
        "envelope": pending.payload.get("envelope").cloned().unwrap_or(serde_json::Value::Null),
    });
    completed.aggregate_type == pending.aggregate_type
        && completed.aggregate_id == pending.aggregate_id
        && completed.event_version == pending.event_version
        && completed.kernel_task_run_id == pending.kernel_task_run_id
        && completed.session_run_id == pending.session_run_id
        && completed.idempotency_key
            == format!("native-editor-fr-complete:{}", pending.aggregate_id)
        && completed.source_component == "native_editor_fr_ingestion"
        && completed.actor == pending.actor
        && completed.causation_id.as_deref() == Some(pending.event_id.as_str())
        && completed.correlation_id.as_deref()
            == Some(
                pending
                    .correlation_id
                    .as_deref()
                    .unwrap_or(pending.aggregate_id.as_str()),
            )
        && completed.payload_hash == expected_hash
        && completed.payload == expected_payload
}

pub(crate) async fn get_by_idempotency(
    storage: &SurrealStorage,
    idempotency_key: &str,
) -> StorageResult<Option<KernelEvent>> {
    let bindings = EventLookupBindings {
        value: idempotency_key.to_owned(),
    };
    let row: Option<LedgerRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "SELECT event_id, event_sequence, event_version, kernel_task_run_id, \
                         session_run_id, aggregate_type, aggregate_id, idempotency_key, event_type, \
                         actor_kind, actor_id, causation_id, correlation_id, payload_hash, \
                         source_component, payload, created_at FROM kernel_event_ledger \
                         WHERE idempotency_key = $value LIMIT 1;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    row.map(row_to_event).transpose()
}

async fn list(
    storage: &SurrealStorage,
    statement: &'static str,
    value: &str,
) -> StorageResult<Vec<KernelEvent>> {
    let bindings = EventLookupBindings {
        value: value.to_owned(),
    };
    let rows: Vec<LedgerRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move { database.query_values(statement, bindings).await })
        })
        .await
        .map_err(StorageError::from)?;
    rows.into_iter().map(row_to_event).collect()
}

fn row_to_event(row: LedgerRow) -> StorageResult<KernelEvent> {
    let payload = normalize_self_describing_payload(row.payload)?;
    Ok(KernelEvent {
        event_id: row.event_id,
        event_sequence: row.event_sequence,
        event_version: row.event_version,
        kernel_task_run_id: row.kernel_task_run_id,
        session_run_id: row.session_run_id,
        aggregate_type: row.aggregate_type,
        aggregate_id: row.aggregate_id,
        idempotency_key: row.idempotency_key,
        event_type: KernelEventType::try_from(row.event_type.as_str())
            .map_err(|_| StorageError::Validation("invalid kernel event_type"))?,
        actor: actor_from_parts(&row.actor_kind, row.actor_id)?,
        causation_id: row.causation_id,
        correlation_id: row.correlation_id,
        payload_hash: row.payload_hash,
        source_component: row.source_component,
        payload,
        created_at: row.created_at.into_inner(),
    })
}

fn normalize_self_describing_payload(
    mut payload: serde_json::Value,
) -> StorageResult<serde_json::Value> {
    let is_float_preference_event = payload.get("type").and_then(serde_json::Value::as_str)
        == Some("preference_record_changed")
        && payload
            .get("value_type")
            .and_then(serde_json::Value::as_str)
            == Some("float");
    if !is_float_preference_event {
        return Ok(payload);
    }

    let object = payload.as_object_mut().ok_or(StorageError::Validation(
        "preference event payload is not an object",
    ))?;
    for field in ["old_value_ref", "new_value_ref"] {
        let Some(value) = object.get_mut(field) else {
            continue;
        };
        let Some(number) = value.as_f64() else {
            continue;
        };
        let number = serde_json::Number::from_f64(number).ok_or(StorageError::Validation(
            "preference event float value is not finite",
        ))?;
        *value = serde_json::Value::Number(number);
    }
    Ok(payload)
}

fn actor_from_parts(kind: &str, id: String) -> StorageResult<KernelActor> {
    match kind {
        "operator" => Ok(KernelActor::Operator(id)),
        "system" => Ok(KernelActor::System(id)),
        "session_broker" => Ok(KernelActor::SessionBroker(id)),
        "model_adapter" => Ok(KernelActor::ModelAdapter(id)),
        "toolgate" => Ok(KernelActor::ToolGate(id)),
        "validation_runner" => Ok(KernelActor::ValidationRunner(id)),
        "promotion_gate" => Ok(KernelActor::PromotionGate(id)),
        _ => Err(StorageError::Validation("invalid kernel actor_kind")),
    }
}

fn ensure_same_event(stored: &KernelEvent, candidate: &KernelEvent) -> StorageResult<()> {
    // Match the Handshake canonical event contract: payload_hash is computed
    // from canonical JSON, while SurrealDB may normalize the decoded numeric
    // JSON representation. Comparing payload Value directly would therefore
    // reject a semantically exact replay after harmless storage normalization.
    let same = stored.event_version == candidate.event_version
        && stored.kernel_task_run_id == candidate.kernel_task_run_id
        && stored.session_run_id == candidate.session_run_id
        && stored.aggregate_type == candidate.aggregate_type
        && stored.aggregate_id == candidate.aggregate_id
        && stored.idempotency_key == candidate.idempotency_key
        && stored.event_type == candidate.event_type
        && stored.actor == candidate.actor
        && stored.causation_id == candidate.causation_id
        && stored.correlation_id == candidate.correlation_id
        && stored.payload_hash == candidate.payload_hash
        && stored.source_component == candidate.source_component;
    if same {
        Ok(())
    } else {
        Err(StorageError::Conflict(
            "kernel event idempotency key was reused with different event content",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::super::SurrealStorageConfig;
    use super::*;
    use serde_json::json;
    use std::future::Future;
    use std::time::Duration;

    fn event(idempotency_key: &str, payload: serde_json::Value) -> NewKernelEvent {
        NewKernelEvent::builder(
            "mt-136-proof-task",
            "mt-136-proof-session",
            KernelEventType::ArtifactStored,
            KernelActor::System("mt-136-proof".to_owned()),
        )
        .aggregate("mt_136_storage_proof", "durable-event")
        .idempotency_key(idempotency_key)
        .source_component("storage_mt_136_proof")
        .payload(payload)
        .build()
        .expect("valid MT-136 EventLedger fixture")
    }

    fn row_from_event(event: KernelEvent, event_sequence: i64) -> LedgerRow {
        LedgerRow {
            event_id: event.event_id,
            event_sequence,
            event_version: event.event_version,
            kernel_task_run_id: event.kernel_task_run_id,
            session_run_id: event.session_run_id,
            aggregate_type: event.aggregate_type,
            aggregate_id: event.aggregate_id,
            idempotency_key: event.idempotency_key,
            event_type: event.event_type.as_str().to_owned(),
            actor_kind: event.actor.actor_kind().to_owned(),
            actor_id: event.actor.actor_id().to_owned(),
            causation_id: event.causation_id,
            correlation_id: event.correlation_id,
            payload_hash: event.payload_hash,
            source_component: event.source_component,
            payload: event.payload,
            created_at: Datetime::from(event.created_at),
        }
    }

    #[test]
    fn atomic_batch_results_follow_caller_order() {
        let (first, _) = prepare_event(event("mt-136-batch-first", json!({"ordinal": 1})))
            .expect("prepare first event");
        let (second, _) = prepare_event(event("mt-136-batch-second", json!({"ordinal": 2})))
            .expect("prepare second event");
        let mut stored_first = first.clone();
        stored_first.event_id = "KE-stored-first".to_owned();
        let mut stored_second = second.clone();
        stored_second.event_id = "KE-stored-second".to_owned();

        let ordered = order_and_validate(
            vec![
                row_from_event(stored_second, 42),
                row_from_event(stored_first, 41),
            ],
            &[first, second],
        )
        .expect("order stored events");

        assert_eq!(ordered[0].event_id, "KE-stored-first");
        assert_eq!(ordered[0].event_sequence, 41);
        assert_eq!(ordered[1].event_id, "KE-stored-second");
        assert_eq!(ordered[1].event_sequence, 42);
    }

    #[test]
    fn atomic_pair_uses_actual_replayed_first_event_as_causation() {
        let (first, _) = prepare_event(event("mt-136-pair-first", json!({"ordinal": 1})))
            .expect("prepare first event");
        let mut second_new = event("mt-136-pair-second", json!({"ordinal": 2}));
        second_new.causation_id = Some(first.event_id.clone());
        let (second, _) = prepare_event(second_new).expect("prepare second event");

        let mut stored_first = first.clone();
        stored_first.event_id = "KE-durable-first".to_owned();
        let mut stored_second = second.clone();
        stored_second.event_id = "KE-durable-second".to_owned();
        stored_second.causation_id = Some(stored_first.event_id.clone());

        let ordered = order_pair_and_validate(
            vec![
                row_from_event(stored_second, 12),
                row_from_event(stored_first, 11),
            ],
            &[first, second],
        )
        .expect("order stored pair");

        assert_eq!(ordered[0].event_id, "KE-durable-first");
        assert_eq!(ordered[1].causation_id.as_deref(), Some("KE-durable-first"));
    }

    async fn open(path: &std::path::Path) -> SurrealStorage {
        eprintln!("event-ledger-test stage=storage-open state=start");
        let storage = SurrealStorage::open(
            SurrealStorageConfig::with_path(path).expect("valid embedded test path"),
        )
        .await
        .expect("open embedded SurrealDB");
        eprintln!("event-ledger-test stage=storage-open state=complete");
        eprintln!("event-ledger-test stage=schema-bootstrap state=start");
        let (_, after_start) = include_str!("schema.surql")
            .split_once("-- 0018_kernel_event_ledger")
            .expect("compiled schema contains EventLedger start marker");
        let (ddl, _) = after_start
            .split_once("-- 0019_kernel_session_queue")
            .expect("compiled schema contains EventLedger end marker");
        let ddl = ddl.to_owned();
        storage
            .with_admin_operation(move |database| {
                Box::pin(async move {
                    database.query(ddl).await?;
                    Ok(())
                })
            })
            .await
            .expect("bootstrap authoritative EventLedger schema slice");
        eprintln!("event-ledger-test stage=schema-bootstrap state=complete");
        storage
    }

    async fn within<T>(stage: &str, future: impl Future<Output = T>) -> T {
        eprintln!("event-ledger-test stage={stage} state=start");
        let result = tokio::time::timeout(Duration::from_secs(120), future)
            .await
            .unwrap_or_else(|_| panic!("event-ledger-test stage={stage} timed out after 120s"));
        eprintln!("event-ledger-test stage={stage} state=complete");
        result
    }

    #[tokio::test]
    async fn event_ledger_round_trip_survives_shutdown_and_reopen() {
        let directory = tempfile::tempdir().expect("temporary MT-136 store root");
        let path = directory.path().join("store");
        let storage = open(&path).await;
        let inserted = append(
            &storage,
            event("mt-136-durable-event", json!({"proof": "before-reopen"})),
        )
        .await
        .expect("append EventLedger row");
        assert!(inserted.event_sequence > 0);
        storage.shutdown().await.expect("close embedded store");
        drop(storage);

        let reopened = open(&path).await;
        let persisted = get_by_idempotency(&reopened, "mt-136-durable-event")
            .await
            .expect("read reopened EventLedger")
            .expect("durable EventLedger row");
        assert_eq!(persisted.event_id, inserted.event_id);
        assert_eq!(persisted.event_sequence, inserted.event_sequence);
        assert_eq!(persisted.payload, json!({"proof": "before-reopen"}));
        reopened.shutdown().await.expect("close reopened store");
    }

    #[tokio::test]
    async fn concurrent_exact_replays_converge_and_conflicting_replay_fails_closed() {
        let directory = tempfile::tempdir().expect("temporary MT-136 store root");
        let storage = open(&directory.path().join("store")).await;
        let left_storage = storage.clone();
        let right_storage = storage.clone();
        let left = tokio::spawn(async move {
            append(
                &left_storage,
                event("mt-136-concurrent-event", json!({"value": 1})),
            )
            .await
        });
        let right = tokio::spawn(async move {
            append(
                &right_storage,
                event("mt-136-concurrent-event", json!({"value": 1})),
            )
            .await
        });
        let left = left.await.expect("left append task").expect("left append");
        let right = right
            .await
            .expect("right append task")
            .expect("right append");
        assert_eq!(left.event_id, right.event_id);
        assert_eq!(left.event_sequence, right.event_sequence);

        let conflict = append(
            &storage,
            event("mt-136-concurrent-event", json!({"value": 2})),
        )
        .await;
        assert!(matches!(conflict, Err(StorageError::Conflict(_))));
        storage.shutdown().await.expect("close embedded store");
    }

    #[tokio::test]
    async fn atomic_batch_bulk_insert_preserves_replay_and_rollback_contracts() {
        let directory = tempfile::tempdir().expect("temporary atomic batch store root");
        let storage = within("open", open(&directory.path().join("store"))).await;

        let inserted = within(
            "initial-insert",
            append_atomic(
                &storage,
                vec![
                    event("mt-136-bulk-first", json!({"ordinal": 1})),
                    event("mt-136-bulk-second", json!({"ordinal": 2})),
                ],
            ),
        )
        .await
        .expect("bulk insert events");
        assert_eq!(inserted.len(), 2);
        assert!(inserted.iter().all(|stored| stored.event_sequence > 0));

        let replayed = within(
            "exact-replay",
            append_atomic(
                &storage,
                vec![
                    event("mt-136-bulk-first", json!({"ordinal": 1})),
                    event("mt-136-bulk-second", json!({"ordinal": 2})),
                ],
            ),
        )
        .await
        .expect("exact bulk replay");
        assert_eq!(
            replayed
                .iter()
                .map(|stored| stored.event_id.as_str())
                .collect::<Vec<_>>(),
            inserted
                .iter()
                .map(|stored| stored.event_id.as_str())
                .collect::<Vec<_>>()
        );

        let mixed = within(
            "mixed-replay-insert",
            append_atomic(
                &storage,
                vec![
                    event("mt-136-bulk-first", json!({"ordinal": 1})),
                    event("mt-136-bulk-third", json!({"ordinal": 3})),
                ],
            ),
        )
        .await
        .expect("mixed replay and insert");
        assert_eq!(mixed[0].event_id, inserted[0].event_id);
        assert_ne!(mixed[1].event_id, inserted[0].event_id);

        let duplicate = within(
            "internal-exact-duplicate",
            append_atomic(
                &storage,
                vec![
                    event("mt-136-bulk-duplicate", json!({"ordinal": 4})),
                    event("mt-136-bulk-duplicate", json!({"ordinal": 4})),
                ],
            ),
        )
        .await
        .expect("exact duplicate inside one batch");
        assert_eq!(duplicate[0].event_id, duplicate[1].event_id);

        let conflict = within(
            "stored-conflict",
            append_atomic(
                &storage,
                vec![
                    event("mt-136-bulk-first", json!({"ordinal": 999})),
                    event("mt-136-bulk-must-rollback", json!({"ordinal": 5})),
                ],
            ),
        )
        .await;
        assert!(matches!(conflict, Err(StorageError::Conflict(_))));
        assert!(get_by_idempotency(&storage, "mt-136-bulk-must-rollback")
            .await
            .expect("read rolled-back event")
            .is_none());

        let internal_conflict = within(
            "internal-conflict",
            append_atomic(
                &storage,
                vec![
                    event("mt-136-bulk-internal-conflict", json!({"ordinal": 6})),
                    event("mt-136-bulk-internal-conflict", json!({"ordinal": 7})),
                    event("mt-136-bulk-internal-must-rollback", json!({"ordinal": 8})),
                ],
            ),
        )
        .await;
        assert!(matches!(internal_conflict, Err(StorageError::Conflict(_))));
        assert!(
            get_by_idempotency(&storage, "mt-136-bulk-internal-conflict")
                .await
                .expect("read conflicting event")
                .is_none()
        );
        assert!(
            get_by_idempotency(&storage, "mt-136-bulk-internal-must-rollback")
                .await
                .expect("read internal rollback event")
                .is_none()
        );

        let left_storage = storage.clone();
        let right_storage = storage.clone();
        let (left, right) = within("concurrent-exact-batches", async move {
            tokio::join!(
                append_atomic(
                    &left_storage,
                    vec![
                        event("mt-136-bulk-race-first", json!({"ordinal": 9})),
                        event("mt-136-bulk-race-second", json!({"ordinal": 10})),
                    ],
                ),
                append_atomic(
                    &right_storage,
                    vec![
                        event("mt-136-bulk-race-first", json!({"ordinal": 9})),
                        event("mt-136-bulk-race-second", json!({"ordinal": 10})),
                    ],
                ),
            )
        })
        .await;
        let left = left.expect("left concurrent bulk insert");
        let right = right.expect("right concurrent bulk insert");
        assert_eq!(
            left.iter()
                .map(|stored| stored.event_id.as_str())
                .collect::<Vec<_>>(),
            right
                .iter()
                .map(|stored| stored.event_id.as_str())
                .collect::<Vec<_>>()
        );

        storage.shutdown().await.expect("close embedded store");
    }
}

use surrealdb::types::{Datetime, RecordId, RecordIdKey, SurrealValue};

use super::{event_ledger, SurrealStorage};
use crate::kernel::{
    KernelActor, KernelEvent, KernelEventType, KernelSessionLease, NewKernelEvent, SessionBroker,
    SessionRun, SessionRunState,
};
use crate::storage::{StorageError, StorageResult};

const QUEUE: &str = "kernel_session_queue";

#[derive(SurrealValue)]
struct QueueRow {
    id: RecordId,
    kernel_task_run_id: String,
    adapter_id: String,
    state: String,
    claimed_by: Option<String>,
    lease_expires_at: Option<Datetime>,
    attempt_count: i64,
    available_at: Datetime,
    created_at: Datetime,
    updated_at: Datetime,
}

#[derive(SurrealValue)]
struct EnqueueBindings {
    record: RecordId,
    kernel_task_run_id: String,
    adapter_id: String,
    state: String,
    created_at: Datetime,
    updated_at: Datetime,
}

#[derive(SurrealValue)]
struct AtomicEnqueueBindings {
    queue: EnqueueBindings,
    event: event_ledger::LedgerWrite,
}

#[derive(SurrealValue)]
struct ClaimBindings {
    record: RecordId,
    claimed_by: String,
    now: Datetime,
    lease_expires_at: Datetime,
}

#[derive(SurrealValue)]
struct AtomicClaimBindings {
    claim: ClaimBindings,
    event: event_ledger::LedgerWrite,
}

#[derive(SurrealValue)]
struct RecordBinding {
    record: RecordId,
}

#[derive(SurrealValue)]
struct StateBindings {
    record: RecordId,
    expected_state: String,
    state: String,
    release_claim: bool,
    reset_available: bool,
    now: Datetime,
}

#[derive(SurrealValue)]
struct AtomicStateBindings {
    update: StateBindings,
    event: event_ledger::LedgerWrite,
}

pub(crate) async fn enqueue(
    storage: &SurrealStorage,
    session: SessionRun,
) -> StorageResult<SessionRun> {
    validate_session(&session)?;
    let candidate = session.clone();
    let bindings = enqueue_bindings(session);
    let row: Option<QueueRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "IF (SELECT VALUE id FROM $record)[0] = NONE { \
                            RETURN CREATE $record SET session_run_id = record::id($record), kernel_task_run_id = $kernel_task_run_id, \
                              adapter_id = $adapter_id, state = $state, claimed_by = NONE, \
                              lease_expires_at = NONE, attempt_count = 0, available_at = $created_at, \
                              created_at = $created_at, updated_at = $updated_at; \
                         } ELSE { RETURN SELECT * FROM $record; };",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    let stored = row
        .map(map_session_run)
        .transpose()?
        .ok_or_else(|| StorageError::Database("kernel queue enqueue returned no row".to_owned()))?;
    ensure_same_session(&stored, &candidate)?;
    Ok(stored)
}

pub(crate) async fn enqueue_and_record_event(
    storage: &SurrealStorage,
    session: SessionRun,
    causation_id: Option<String>,
    correlation_id: String,
) -> StorageResult<(SessionRun, KernelEvent)> {
    validate_session(&session)?;
    let event = build_event(
        &session.kernel_task_run_id,
        &session.session_run_id,
        KernelEventType::SessionQueued,
        causation_id,
        correlation_id,
        serde_json::json!({"state": session.state.as_str()}),
    )?;
    let (candidate_event, event_write) = event_ledger::prepare_event(event)?;
    let event_key = candidate_event.idempotency_key.clone();
    let candidate = session.clone();
    let rows: Vec<QueueRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at(
                        "BEGIN TRANSACTION; \
                         IF (SELECT VALUE id FROM $queue.record)[0] = NONE { \
                            CREATE $queue.record SET session_run_id = record::id($queue.record), kernel_task_run_id = $queue.kernel_task_run_id, \
                              adapter_id = $queue.adapter_id, state = $queue.state, claimed_by = NONE, \
                              lease_expires_at = NONE, attempt_count = 0, available_at = $queue.created_at, \
                              created_at = $queue.created_at, updated_at = $queue.updated_at; \
                         }; \
                         IF (SELECT VALUE id FROM kernel_event_ledger \
                             WHERE idempotency_key = $event.idempotency_key LIMIT 1)[0] = NONE { \
                            CREATE $event.record CONTENT { event_id: $event.event_id, event_version: $event.event_version, \
                              kernel_task_run_id: $event.kernel_task_run_id, session_run_id: $event.session_run_id, \
                              aggregate_type: $event.aggregate_type, aggregate_id: $event.aggregate_id, \
                              idempotency_key: $event.idempotency_key, event_type: $event.event_type, \
                              actor_kind: $event.actor_kind, actor_id: $event.actor_id, causation_id: $event.causation_id, \
                              correlation_id: $event.correlation_id, payload_hash: $event.payload_hash, \
                              source_component: $event.source_component, payload: $event.payload, created_at: $event.created_at }; \
                         }; \
                         COMMIT TRANSACTION; \
                         SELECT * FROM $queue.record;",
                        AtomicEnqueueBindings {
                            queue: enqueue_bindings(session),
                            event: event_write,
                        },
                        4,
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    let stored = rows
        .into_iter()
        .next()
        .map(map_session_run)
        .transpose()?
        .ok_or_else(|| StorageError::Database("atomic enqueue returned no row".to_owned()))?;
    ensure_same_session(&stored, &candidate)?;
    let stored_event = event_ledger::get_by_idempotency(storage, &event_key)
        .await?
        .ok_or_else(|| StorageError::Database("atomic enqueue event is missing".to_owned()))?;
    ensure_same_event(&stored_event, &candidate_event)?;
    Ok((stored, stored_event))
}

pub(crate) async fn claim(
    storage: &SurrealStorage,
    session_run_id: &str,
    claimed_by: &str,
    lease_seconds: i64,
) -> StorageResult<Option<KernelSessionLease>> {
    let bindings = claim_bindings(session_run_id, claimed_by, lease_seconds)?;
    let row: Option<QueueRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "UPDATE $record SET state = 'CLAIMED', claimed_by = $claimed_by, \
                         lease_expires_at = $lease_expires_at, attempt_count += 1, updated_at = $now \
                         WHERE available_at <= $now AND (state IN ['QUEUED', 'RETRY_SCHEDULED'] \
                           OR (state IN ['CLAIMED', 'RUNNING'] AND lease_expires_at != NONE \
                               AND lease_expires_at <= $now)) RETURN AFTER;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    row.map(map_lease).transpose()
}

pub(crate) async fn claim_and_record_event(
    storage: &SurrealStorage,
    session_run_id: &str,
    claimed_by: &str,
    lease_seconds: i64,
    causation_id: Option<String>,
    correlation_id: String,
) -> StorageResult<Option<(KernelSessionLease, KernelEvent)>> {
    let Some(current) = get_queue(storage, session_run_id).await? else {
        return Ok(None);
    };
    let event = build_event(
        &current.kernel_task_run_id,
        session_run_id,
        KernelEventType::SessionClaimed,
        causation_id,
        correlation_id,
        serde_json::json!({
            "state": SessionRunState::Claimed.as_str(),
            "claimed_by": claimed_by,
            "attempt_count": current.attempt_count + 1,
        }),
    )?;
    let (candidate_event, event_write) = event_ledger::prepare_event(event)?;
    let event_key = candidate_event.idempotency_key.clone();
    let rows: Vec<QueueRow> = storage
        .with_data_operation({
            let claim = claim_bindings(session_run_id, claimed_by, lease_seconds)?;
            move |database| {
                Box::pin(async move {
                    database
                        .query_values_at(
                            "BEGIN TRANSACTION; \
                             LET $updated = UPDATE $claim.record SET state = 'CLAIMED', \
                               claimed_by = $claim.claimed_by, lease_expires_at = $claim.lease_expires_at, \
                               attempt_count += 1, updated_at = $claim.now \
                               WHERE available_at <= $claim.now AND (state IN ['QUEUED', 'RETRY_SCHEDULED'] \
                                 OR (state IN ['CLAIMED', 'RUNNING'] AND lease_expires_at != NONE \
                                     AND lease_expires_at <= $claim.now)) RETURN AFTER; \
                             IF array::len($updated) > 0 { \
                                CREATE $event.record CONTENT { event_id: $event.event_id, event_version: $event.event_version, \
                                  kernel_task_run_id: $event.kernel_task_run_id, session_run_id: $event.session_run_id, \
                                  aggregate_type: $event.aggregate_type, aggregate_id: $event.aggregate_id, \
                                  idempotency_key: $event.idempotency_key, event_type: $event.event_type, \
                                  actor_kind: $event.actor_kind, actor_id: $event.actor_id, causation_id: $event.causation_id, \
                                  correlation_id: $event.correlation_id, payload_hash: $event.payload_hash, \
                                  source_component: $event.source_component, payload: $event.payload, created_at: $event.created_at }; \
                             }; \
                             COMMIT TRANSACTION; \
                             RETURN $updated;",
                            AtomicClaimBindings {
                                claim,
                                event: event_write,
                            },
                            4,
                        )
                        .await
                })
            }
        })
        .await
        .map_err(StorageError::from)?;
    let Some(row) = rows.into_iter().next() else {
        return Ok(None);
    };
    let lease = map_lease(row)?;
    let stored_event = event_ledger::get_by_idempotency(storage, &event_key)
        .await?
        .ok_or_else(|| StorageError::Database("atomic claim event is missing".to_owned()))?;
    ensure_same_event(&stored_event, &candidate_event)?;
    Ok(Some((lease, stored_event)))
}

pub(crate) async fn update_state(
    storage: &SurrealStorage,
    session_run_id: &str,
    state: SessionRunState,
) -> StorageResult<KernelSessionLease> {
    let current = get_queue(storage, session_run_id)
        .await?
        .ok_or(StorageError::NotFound("kernel_session_run"))?;
    validate_transition(current.state, state)?;
    let row: Option<QueueRow> = storage
        .with_data_operation({
            let bindings = state_bindings(session_run_id, current.state, state);
            move |database| {
                Box::pin(async move {
                    database
                        .query_first(
                            "UPDATE $record SET state = $state, \
                             claimed_by = IF $release_claim { NONE } ELSE { claimed_by }, \
                             lease_expires_at = IF $release_claim { NONE } ELSE { lease_expires_at }, \
                             available_at = IF $reset_available { $now } ELSE { available_at }, updated_at = $now \
                             WHERE state = $expected_state RETURN AFTER;",
                            bindings,
                        )
                        .await
                })
            }
        })
        .await
        .map_err(StorageError::from)?;
    row.map(map_lease)
        .transpose()?
        .ok_or(StorageError::Conflict(
            "kernel session state changed concurrently",
        ))
}

pub(crate) async fn update_state_and_record_event(
    storage: &SurrealStorage,
    session_run_id: &str,
    state: SessionRunState,
    causation_id: Option<String>,
    correlation_id: String,
) -> StorageResult<(KernelSessionLease, KernelEvent)> {
    let current = get_queue(storage, session_run_id)
        .await?
        .ok_or(StorageError::NotFound("kernel_session_run"))?;
    validate_transition(current.state, state)?;
    let event_type = if current.state == state {
        state_event_type(state)
    } else {
        SessionBroker::transition_event_type(current.state, state)
            .map_err(|error| StorageError::Serialization(error.to_string()))?
    };
    let event = build_event(
        &current.kernel_task_run_id,
        session_run_id,
        event_type,
        causation_id,
        correlation_id,
        serde_json::json!({"state": state.as_str()}),
    )?;
    let (candidate_event, event_write) = event_ledger::prepare_event(event)?;
    let event_key = candidate_event.idempotency_key.clone();
    let rows: Vec<QueueRow> = storage
        .with_data_operation({
            let update = state_bindings(session_run_id, current.state, state);
            move |database| {
                Box::pin(async move {
                    database
                        .query_values_at(
                            "BEGIN TRANSACTION; \
                             LET $updated = UPDATE $update.record SET state = $update.state, \
                               claimed_by = IF $update.release_claim { NONE } ELSE { claimed_by }, \
                               lease_expires_at = IF $update.release_claim { NONE } ELSE { lease_expires_at }, \
                               available_at = IF $update.reset_available { $update.now } ELSE { available_at }, \
                               updated_at = $update.now WHERE state = $update.expected_state RETURN AFTER; \
                             IF array::len($updated) = 0 { THROW 'HSK-KERNEL-SESSION-CONCURRENT-STATE'; }; \
                             CREATE $event.record CONTENT { event_id: $event.event_id, event_version: $event.event_version, \
                               kernel_task_run_id: $event.kernel_task_run_id, session_run_id: $event.session_run_id, \
                               aggregate_type: $event.aggregate_type, aggregate_id: $event.aggregate_id, \
                               idempotency_key: $event.idempotency_key, event_type: $event.event_type, \
                               actor_kind: $event.actor_kind, actor_id: $event.actor_id, causation_id: $event.causation_id, \
                               correlation_id: $event.correlation_id, payload_hash: $event.payload_hash, \
                               source_component: $event.source_component, payload: $event.payload, created_at: $event.created_at }; \
                             COMMIT TRANSACTION; \
                             RETURN $updated;",
                            AtomicStateBindings {
                                update,
                                event: event_write,
                            },
                            5,
                        )
                        .await
                })
            }
        })
        .await
        .map_err(map_queue_error)?;
    let lease =
        rows.into_iter()
            .next()
            .map(map_lease)
            .transpose()?
            .ok_or(StorageError::Conflict(
                "kernel session state changed concurrently",
            ))?;
    let stored_event = event_ledger::get_by_idempotency(storage, &event_key)
        .await?
        .ok_or_else(|| StorageError::Database("atomic state event is missing".to_owned()))?;
    ensure_same_event(&stored_event, &candidate_event)?;
    Ok((lease, stored_event))
}

fn enqueue_bindings(session: SessionRun) -> EnqueueBindings {
    EnqueueBindings {
        record: RecordId::new(QUEUE, session.session_run_id),
        kernel_task_run_id: session.kernel_task_run_id,
        adapter_id: session.adapter_id,
        state: session.state.as_str().to_owned(),
        created_at: Datetime::from(session.created_at),
        updated_at: Datetime::from(session.updated_at),
    }
}

fn claim_bindings(
    session_run_id: &str,
    claimed_by: &str,
    lease_seconds: i64,
) -> StorageResult<ClaimBindings> {
    if session_run_id.trim().is_empty() || claimed_by.trim().is_empty() {
        return Err(StorageError::Validation(
            "session_run_id and claimed_by are required",
        ));
    }
    if lease_seconds <= 0 {
        return Err(StorageError::Validation("lease_seconds must be positive"));
    }
    let now = chrono::Utc::now();
    Ok(ClaimBindings {
        record: RecordId::new(QUEUE, session_run_id.to_owned()),
        claimed_by: claimed_by.to_owned(),
        now: Datetime::from(now),
        lease_expires_at: Datetime::from(now + chrono::Duration::seconds(lease_seconds)),
    })
}

fn state_bindings(
    session_run_id: &str,
    expected: SessionRunState,
    state: SessionRunState,
) -> StateBindings {
    let release_claim = state.is_terminal()
        || matches!(
            state,
            SessionRunState::Queued | SessionRunState::RetryScheduled
        );
    StateBindings {
        record: RecordId::new(QUEUE, session_run_id.to_owned()),
        expected_state: expected.as_str().to_owned(),
        state: state.as_str().to_owned(),
        release_claim,
        reset_available: matches!(
            state,
            SessionRunState::RetryScheduled | SessionRunState::BackpressureDelayed
        ),
        now: Datetime::from(chrono::Utc::now()),
    }
}

async fn get_queue(
    storage: &SurrealStorage,
    session_run_id: &str,
) -> StorageResult<Option<KernelSessionLease>> {
    let row: Option<QueueRow> = storage
        .with_data_operation({
            let session_run_id = session_run_id.to_owned();
            move |database| {
                Box::pin(async move { database.select_one(QUEUE, &session_run_id).await })
            }
        })
        .await
        .map_err(StorageError::from)?;
    row.map(map_lease).transpose()
}

fn validate_session(session: &SessionRun) -> StorageResult<()> {
    if session.session_run_id.trim().is_empty()
        || session.kernel_task_run_id.trim().is_empty()
        || session.adapter_id.trim().is_empty()
    {
        return Err(StorageError::Validation(
            "kernel session identifiers are required",
        ));
    }
    if session.updated_at < session.created_at {
        return Err(StorageError::Validation(
            "kernel session updated_at precedes created_at",
        ));
    }
    Ok(())
}

fn validate_transition(from: SessionRunState, to: SessionRunState) -> StorageResult<()> {
    if from != to && !SessionBroker::can_transition(from, to) {
        Err(StorageError::Validation(
            "invalid kernel session transition",
        ))
    } else {
        Ok(())
    }
}

fn build_event(
    kernel_task_run_id: &str,
    session_run_id: &str,
    event_type: KernelEventType,
    causation_id: Option<String>,
    correlation_id: String,
    payload: serde_json::Value,
) -> StorageResult<NewKernelEvent> {
    let mut builder = NewKernelEvent::builder(
        kernel_task_run_id,
        session_run_id,
        event_type,
        KernelActor::SessionBroker("kernel-session-broker".to_owned()),
    )
    .correlation_id(correlation_id)
    .source_component("session_broker")
    .payload(payload);
    if let Some(causation_id) = causation_id {
        builder = builder.causation_id(causation_id);
    }
    builder
        .build()
        .map_err(|error| StorageError::Serialization(error.to_string()))
}

fn state_event_type(state: SessionRunState) -> KernelEventType {
    match state {
        SessionRunState::Queued => KernelEventType::SessionQueued,
        SessionRunState::Claimed => KernelEventType::SessionClaimed,
        SessionRunState::Running => KernelEventType::SessionStarted,
        SessionRunState::Completed => KernelEventType::SessionCompleted,
        SessionRunState::Failed => KernelEventType::SessionFailed,
        SessionRunState::Cancelled => KernelEventType::SessionCancelled,
        SessionRunState::BackpressureDelayed => KernelEventType::SessionBackpressureDelayed,
        SessionRunState::RetryScheduled => KernelEventType::SessionRetryScheduled,
        SessionRunState::DeadLettered => KernelEventType::SessionDeadLettered,
    }
}

fn key(record: RecordId) -> StorageResult<String> {
    match record.key {
        RecordIdKey::String(value) => Ok(value),
        _ => Err(StorageError::Database(
            "kernel queue record has a non-string id".to_owned(),
        )),
    }
}

fn map_state(value: &str) -> StorageResult<SessionRunState> {
    SessionRunState::parse(value).map_err(|_| StorageError::Validation("invalid session state"))
}

fn map_session_run(row: QueueRow) -> StorageResult<SessionRun> {
    Ok(SessionRun {
        session_run_id: key(row.id)?,
        kernel_task_run_id: row.kernel_task_run_id,
        adapter_id: row.adapter_id,
        state: map_state(&row.state)?,
        created_at: row.created_at.into_inner(),
        updated_at: row.updated_at.into_inner(),
    })
}

fn map_lease(row: QueueRow) -> StorageResult<KernelSessionLease> {
    let _ = row.available_at;
    Ok(KernelSessionLease {
        session_run_id: key(row.id)?,
        kernel_task_run_id: row.kernel_task_run_id,
        adapter_id: row.adapter_id,
        state: map_state(&row.state)?,
        claimed_by: row.claimed_by,
        lease_expires_at: row.lease_expires_at.map(|value| value.into_inner()),
        attempt_count: row.attempt_count,
        created_at: row.created_at.into_inner(),
        updated_at: row.updated_at.into_inner(),
    })
}

fn ensure_same_session(stored: &SessionRun, candidate: &SessionRun) -> StorageResult<()> {
    if stored.session_run_id == candidate.session_run_id
        && stored.kernel_task_run_id == candidate.kernel_task_run_id
        && stored.adapter_id == candidate.adapter_id
        && stored.state == candidate.state
        && stored.created_at == candidate.created_at
        && stored.updated_at == candidate.updated_at
    {
        Ok(())
    } else {
        Err(StorageError::Conflict(
            "kernel session id was reused with different content",
        ))
    }
}

fn ensure_same_event(stored: &KernelEvent, candidate: &KernelEvent) -> StorageResult<()> {
    if stored.event_version == candidate.event_version
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
        && stored.source_component == candidate.source_component
        && stored.payload == candidate.payload
    {
        Ok(())
    } else {
        Err(StorageError::Conflict(
            "kernel event idempotency key was reused with different content",
        ))
    }
}

fn map_queue_error(error: super::SurrealStorageError) -> StorageError {
    if error
        .to_string()
        .contains("HSK-KERNEL-SESSION-CONCURRENT-STATE")
    {
        StorageError::Conflict("kernel session state changed concurrently")
    } else {
        StorageError::from(error)
    }
}

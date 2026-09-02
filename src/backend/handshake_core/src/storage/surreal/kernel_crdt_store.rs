//! Durable embedded-SurrealDB persistence for kernel CRDT updates and snapshots.

use sha2::{Digest, Sha256};
use surrealdb::types::{Bytes, RecordId, RecordIdKey, SurrealValue};
use tokio::sync::Mutex;

use super::{event_ledger, SurrealStorage};
use crate::kernel::crdt::{
    persistence::{
        validate_crdt_update_record, CrdtReplayMetadataV1, CrdtStorageAuthorityPosture,
        CrdtUpdateRecordV1,
    },
    snapshot::{validate_crdt_snapshot_record, CrdtSnapshotRecordV1},
};
use crate::kernel::{KernelEventType, NewKernelEvent};
use crate::storage::{
    KernelCrdtAtomicAppendOutcome, KernelCrdtAtomicAppendRequest, StorageError, StorageResult,
};

const UPDATE_TABLE: &str = "kernel_crdt_updates";
const SNAPSHOT_TABLE: &str = "kernel_crdt_snapshots";
const EVENT_TABLE: &str = "kernel_event_ledger";

static CRDT_ATOMIC_APPEND_LOCK: Mutex<()> = Mutex::const_new(());

#[derive(SurrealValue)]
struct UpdateRow {
    schema_id: String,
    workspace_id: String,
    document_id: String,
    crdt_document_id: String,
    update_id: String,
    update_seq: i64,
    update_sha256: String,
    update_bytes_ref: String,
    actor_id: String,
    actor_kind: String,
    session_id: String,
    trace_id: String,
    state_vector_before: String,
    state_vector_after: String,
    replay_metadata_json: serde_json::Value,
    event_ledger_stream_id: String,
    event_ledger_event_id: RecordId,
    storage_authority: String,
}

#[derive(SurrealValue)]
struct SnapshotRow {
    schema_id: String,
    snapshot_id: String,
    workspace_id: String,
    document_id: String,
    crdt_document_id: String,
    covered_update_seq: i64,
    state_vector: String,
    snapshot_sha256: String,
    snapshot_bytes_ref: String,
    actor_id: String,
    actor_kind: String,
    event_ledger_stream_id: String,
    event_ledger_event_id: RecordId,
    promotion_evidence_update_ids: Vec<String>,
    storage_authority: String,
}

#[derive(SurrealValue)]
struct UpdateWriteBindings {
    record: RecordId,
    schema_id: String,
    workspace_id: String,
    document_id: String,
    crdt_document_id: String,
    update_id: String,
    update_seq: i64,
    update_sha256: String,
    update_bytes_ref: String,
    update_bytes: Bytes,
    actor_id: String,
    actor_kind: String,
    session_id: String,
    trace_id: String,
    state_vector_before: String,
    state_vector_after: String,
    replay_metadata_json: serde_json::Value,
    event_ledger_stream_id: String,
    event_ledger_event_id: RecordId,
    storage_authority: String,
}

#[derive(SurrealValue)]
struct SnapshotWriteBindings {
    record: RecordId,
    schema_id: String,
    snapshot_id: String,
    workspace_id: String,
    document_id: String,
    crdt_document_id: String,
    covered_update_seq: i64,
    state_vector: String,
    snapshot_sha256: String,
    snapshot_bytes_ref: String,
    snapshot_bytes: Bytes,
    actor_id: String,
    actor_kind: String,
    event_ledger_stream_id: String,
    event_ledger_event_id: RecordId,
    promotion_evidence_update_ids: Vec<String>,
    storage_authority: String,
}

#[derive(SurrealValue)]
struct IdentityBindings {
    workspace_id: String,
    document_id: String,
    crdt_document_id: String,
}

#[derive(SurrealValue)]
struct BytesRefBinding {
    bytes_ref: String,
}

#[derive(SurrealValue)]
struct BytesRow {
    bytes: Bytes,
}

#[derive(SurrealValue)]
struct AtomicWriteBindings {
    update: UpdateWriteBindings,
    event: event_ledger::LedgerWrite,
}

pub(crate) async fn append_update_with_event_atomic(
    storage: &SurrealStorage,
    request: KernelCrdtAtomicAppendRequest,
) -> StorageResult<KernelCrdtAtomicAppendOutcome> {
    if !request
        .provisional_record
        .event_ledger_event_id
        .trim()
        .is_empty()
    {
        return Err(StorageError::Validation(
            "atomic CRDT append requires an empty provisional EventLedger event id",
        ));
    }
    if sha256_hex(&request.update_bytes) != request.provisional_record.update_sha256 {
        return Err(StorageError::Validation(
            "kernel CRDT update bytes do not match update_sha256",
        ));
    }
    authority_str(request.provisional_record.storage_authority)?;
    validate_atomic_event(&request.event, &request.provisional_record)?;

    let _guard = CRDT_ATOMIC_APPEND_LOCK.lock().await;
    let current = list_updates(
        storage,
        &request.provisional_record.workspace_id,
        &request.provisional_record.document_id,
        &request.provisional_record.crdt_document_id,
    )
    .await?;
    if let Some(existing) = current
        .iter()
        .find(|record| record.update_id == request.provisional_record.update_id)
        .cloned()
    {
        if !existing_matches_provisional(&existing, &request.provisional_record) {
            return Ok(KernelCrdtAtomicAppendOutcome::UpdateIdContentMismatch {
                update_id: request.provisional_record.update_id,
            });
        }
        validate_update_event_link(storage, &existing).await?;
        let head = current.last().ok_or(StorageError::Conflict(
            "kernel CRDT idempotency row exists without a document head",
        ))?;
        return Ok(KernelCrdtAtomicAppendOutcome::AlreadyStored {
            record: existing,
            head_update_seq: head.update_seq,
            head_state_vector: head.state_vector_after.clone(),
        });
    }

    let (head_update_seq, head_state_vector) = current
        .last()
        .map(|record| (record.update_seq, record.state_vector_after.clone()))
        .unwrap_or_else(|| (0, "hsk-sv1:".to_owned()));
    let expected_next_seq = head_update_seq
        .checked_add(1)
        .ok_or(StorageError::Validation(
            "kernel CRDT update sequence overflow",
        ))?;
    if request.expected_head_update_seq != head_update_seq
        || request.expected_head_state_vector != head_state_vector
        || request.provisional_record.update_seq != expected_next_seq
        || request.provisional_record.state_vector_before != head_state_vector
    {
        return Ok(KernelCrdtAtomicAppendOutcome::StaleHead {
            head_update_seq,
            head_state_vector,
        });
    }

    let (stored_event, event) = event_ledger::prepare_event(request.event)?;
    let mut record = request.provisional_record;
    record.event_ledger_event_id = stored_event.event_id;
    validate_crdt_update_record(&record)
        .map_err(|_| StorageError::Validation("invalid kernel CRDT update record"))?;
    let update = update_bindings(&record, request.update_bytes)?;
    let bindings = AtomicWriteBindings { update, event };
    let result: Result<Vec<UpdateRow>, _> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at(
                        "BEGIN TRANSACTION; \
                         CREATE $event.record CONTENT { event_id: $event.event_id, event_version: $event.event_version, kernel_task_run_id: $event.kernel_task_run_id, session_run_id: $event.session_run_id, aggregate_type: $event.aggregate_type, aggregate_id: $event.aggregate_id, idempotency_key: $event.idempotency_key, event_type: $event.event_type, actor_kind: $event.actor_kind, actor_id: $event.actor_id, causation_id: $event.causation_id, correlation_id: $event.correlation_id, payload_hash: $event.payload_hash, source_component: $event.source_component, payload: $event.payload, created_at: $event.created_at }; \
                         CREATE $update.record CONTENT { schema_id: $update.schema_id, workspace_id: $update.workspace_id, document_id: $update.document_id, crdt_document_id: $update.crdt_document_id, update_id: $update.update_id, update_seq: $update.update_seq, update_sha256: $update.update_sha256, update_bytes_ref: $update.update_bytes_ref, update_bytes: $update.update_bytes, actor_id: $update.actor_id, actor_kind: $update.actor_kind, session_id: $update.session_id, trace_id: $update.trace_id, state_vector_before: $update.state_vector_before, state_vector_after: $update.state_vector_after, replay_metadata_json: $update.replay_metadata_json, event_ledger_stream_id: $update.event_ledger_stream_id, event_ledger_event_id: $update.event_ledger_event_id, storage_authority: $update.storage_authority }; \
                         COMMIT TRANSACTION; \
                         SELECT * FROM $update.record;",
                        bindings,
                        4,
                    )
                    .await
            })
        })
        .await;
    let stored = result
        .map_err(map_write_error)?
        .into_iter()
        .next()
        .ok_or_else(|| {
            StorageError::Database("atomic kernel CRDT append returned no row".to_owned())
        })
        .and_then(map_update)?;
    if stored != record {
        return Err(StorageError::Conflict(
            "atomic kernel CRDT append projection mismatch",
        ));
    }
    validate_update_event_link(storage, &stored).await?;
    Ok(KernelCrdtAtomicAppendOutcome::Stored(stored))
}

fn validate_atomic_event(event: &NewKernelEvent, record: &CrdtUpdateRecordV1) -> StorageResult<()> {
    event
        .validate()
        .map_err(|_| StorageError::Validation("invalid kernel CRDT event"))?;
    let expected_idempotency_key = format!(
        "knowledge-crdt-update:{}:{}",
        record.crdt_document_id, record.update_id
    );
    let payload = &event.payload;
    let exact = event.aggregate_type == "knowledge_crdt_document"
        && event.aggregate_id == record.crdt_document_id
        && event.idempotency_key == expected_idempotency_key
        && event.event_type == KernelEventType::KnowledgeCrdtUpdateRecorded
        && event.actor.actor_id() == record.actor_id
        && event.actor.actor_kind() == record.actor_kind
        && event.session_run_id == record.session_id
        && event.correlation_id.as_deref() == Some(record.trace_id.as_str())
        && event.source_component == "knowledge_crdt_yjs_bridge"
        && payload.get("update_id").and_then(serde_json::Value::as_str)
            == Some(record.update_id.as_str())
        && payload
            .get("update_seq")
            .and_then(serde_json::Value::as_u64)
            == Some(record.update_seq)
        && payload
            .get("update_sha256")
            .and_then(serde_json::Value::as_str)
            == Some(record.update_sha256.as_str())
        && payload
            .get("state_vector_before")
            .and_then(serde_json::Value::as_str)
            == Some(record.state_vector_before.as_str())
        && payload
            .get("state_vector_after")
            .and_then(serde_json::Value::as_str)
            == Some(record.state_vector_after.as_str());
    if !exact {
        return Err(StorageError::Conflict(
            "kernel CRDT event does not match its update projection",
        ));
    }
    Ok(())
}

async fn validate_update_event_link(
    storage: &SurrealStorage,
    record: &CrdtUpdateRecordV1,
) -> StorageResult<()> {
    let events = event_ledger::list_for_aggregate(
        storage,
        "knowledge_crdt_document",
        &record.crdt_document_id,
    )
    .await?;
    let event = events
        .into_iter()
        .find(|event| event.event_id == record.event_ledger_event_id)
        .ok_or(StorageError::Conflict(
            "kernel CRDT update is missing its canonical EventLedger receipt",
        ))?;
    let expected_idempotency_key = format!(
        "knowledge-crdt-update:{}:{}",
        record.crdt_document_id, record.update_id
    );
    let exact = event.aggregate_type == "knowledge_crdt_document"
        && event.aggregate_id == record.crdt_document_id
        && event.idempotency_key == expected_idempotency_key
        && event.event_type == KernelEventType::KnowledgeCrdtUpdateRecorded
        && event.actor.actor_id() == record.actor_id
        && event.actor.actor_kind() == record.actor_kind
        && event.session_run_id == record.session_id
        && event.correlation_id.as_deref() == Some(record.trace_id.as_str())
        && event.source_component == "knowledge_crdt_yjs_bridge"
        && event
            .payload
            .get("update_id")
            .and_then(serde_json::Value::as_str)
            == Some(record.update_id.as_str())
        && event
            .payload
            .get("update_seq")
            .and_then(serde_json::Value::as_u64)
            == Some(record.update_seq)
        && event
            .payload
            .get("update_sha256")
            .and_then(serde_json::Value::as_str)
            == Some(record.update_sha256.as_str())
        && event
            .payload
            .get("state_vector_before")
            .and_then(serde_json::Value::as_str)
            == Some(record.state_vector_before.as_str())
        && event
            .payload
            .get("state_vector_after")
            .and_then(serde_json::Value::as_str)
            == Some(record.state_vector_after.as_str());
    if !exact {
        return Err(StorageError::Conflict(
            "kernel CRDT update EventLedger receipt does not match its projection",
        ));
    }
    Ok(())
}

fn update_bindings(
    record: &CrdtUpdateRecordV1,
    update_bytes: Vec<u8>,
) -> StorageResult<UpdateWriteBindings> {
    let update_seq = i64::try_from(record.update_seq)
        .map_err(|_| StorageError::Validation("kernel CRDT update sequence too large"))?;
    let event_id = required_event_id(&record.event_ledger_event_id)?;
    Ok(UpdateWriteBindings {
        record: RecordId::new(
            UPDATE_TABLE,
            composite_id(
                "KCU",
                &[
                    &record.workspace_id,
                    &record.document_id,
                    &record.crdt_document_id,
                    &record.update_id,
                ],
            ),
        ),
        schema_id: record.schema_id.clone(),
        workspace_id: record.workspace_id.clone(),
        document_id: record.document_id.clone(),
        crdt_document_id: record.crdt_document_id.clone(),
        update_id: record.update_id.clone(),
        update_seq,
        update_sha256: record.update_sha256.clone(),
        update_bytes_ref: record.update_bytes_ref.clone(),
        update_bytes: Bytes::from(update_bytes),
        actor_id: record.actor_id.clone(),
        actor_kind: record.actor_kind.clone(),
        session_id: record.session_id.clone(),
        trace_id: record.trace_id.clone(),
        state_vector_before: record.state_vector_before.clone(),
        state_vector_after: record.state_vector_after.clone(),
        replay_metadata_json: serde_json::to_value(&record.replay_metadata)?,
        event_ledger_stream_id: record.event_ledger_stream_id.clone(),
        event_ledger_event_id: RecordId::new(EVENT_TABLE, event_id),
        storage_authority: authority_str(record.storage_authority)?.to_owned(),
    })
}

fn existing_matches_provisional(
    existing: &CrdtUpdateRecordV1,
    provisional: &CrdtUpdateRecordV1,
) -> bool {
    existing.schema_id == provisional.schema_id
        && existing.workspace_id == provisional.workspace_id
        && existing.document_id == provisional.document_id
        && existing.crdt_document_id == provisional.crdt_document_id
        && existing.update_id == provisional.update_id
        && existing.update_sha256 == provisional.update_sha256
        && existing.update_bytes_ref == provisional.update_bytes_ref
        && existing.actor_id == provisional.actor_id
        && existing.actor_kind == provisional.actor_kind
        && existing.session_id == provisional.session_id
        && existing.trace_id == provisional.trace_id
        && existing.state_vector_before == provisional.state_vector_before
        && existing.state_vector_after == provisional.state_vector_after
        && existing.replay_metadata.dependency_update_ids
            == provisional.replay_metadata.dependency_update_ids
        && existing.replay_metadata.encoding == provisional.replay_metadata.encoding
        && existing.replay_metadata.schema_version == provisional.replay_metadata.schema_version
        && existing.event_ledger_stream_id == provisional.event_ledger_stream_id
        && existing.storage_authority == provisional.storage_authority
}

pub(crate) async fn append_update(
    storage: &SurrealStorage,
    record: CrdtUpdateRecordV1,
    update_bytes: Vec<u8>,
) -> StorageResult<CrdtUpdateRecordV1> {
    validate_crdt_update_record(&record)
        .map_err(|_| StorageError::Validation("invalid kernel CRDT update record"))?;
    if sha256_hex(&update_bytes) != record.update_sha256 {
        return Err(StorageError::Validation(
            "kernel CRDT update bytes do not match update_sha256",
        ));
    }
    let bindings = update_bindings(&record, update_bytes)?;
    let result: Result<Vec<UpdateRow>, _> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at(
                        "IF !record::exists($event_ledger_event_id) { THROW 'HSK-CRDT-EVENT-MISSING'; }; \
                         IF (SELECT VALUE id FROM kernel_crdt_updates WHERE workspace_id = $workspace_id AND document_id = $document_id AND crdt_document_id = $crdt_document_id AND update_id = $update_id LIMIT 1)[0] != NONE { \
                             RETURN SELECT * FROM kernel_crdt_updates WHERE workspace_id = $workspace_id AND document_id = $document_id AND crdt_document_id = $crdt_document_id AND update_id = $update_id LIMIT 1; \
                         } ELSE { \
                             RETURN CREATE $record CONTENT { schema_id: $schema_id, workspace_id: $workspace_id, document_id: $document_id, crdt_document_id: $crdt_document_id, update_id: $update_id, update_seq: $update_seq, update_sha256: $update_sha256, update_bytes_ref: $update_bytes_ref, update_bytes: $update_bytes, actor_id: $actor_id, actor_kind: $actor_kind, session_id: $session_id, trace_id: $trace_id, state_vector_before: $state_vector_before, state_vector_after: $state_vector_after, replay_metadata_json: $replay_metadata_json, event_ledger_stream_id: $event_ledger_stream_id, event_ledger_event_id: $event_ledger_event_id, storage_authority: $storage_authority }; \
                         };",
                        bindings,
                        1,
                    )
                    .await
            })
        })
        .await;
    let stored = result
        .map_err(map_write_error)?
        .into_iter()
        .next()
        .ok_or_else(|| StorageError::Database("kernel CRDT update write returned no row".into()))
        .and_then(map_update)?;
    if stored != record {
        return Err(StorageError::Conflict(
            "kernel CRDT update idempotency conflict",
        ));
    }
    Ok(stored)
}

pub(crate) async fn list_updates(
    storage: &SurrealStorage,
    workspace_id: &str,
    document_id: &str,
    crdt_document_id: &str,
) -> StorageResult<Vec<CrdtUpdateRecordV1>> {
    let bindings = IdentityBindings {
        workspace_id: workspace_id.to_owned(),
        document_id: document_id.to_owned(),
        crdt_document_id: crdt_document_id.to_owned(),
    };
    let rows: Vec<UpdateRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT * FROM kernel_crdt_updates WHERE workspace_id = $workspace_id AND document_id = $document_id AND crdt_document_id = $crdt_document_id ORDER BY update_seq ASC;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    let records = rows
        .into_iter()
        .map(map_update)
        .collect::<StorageResult<Vec<_>>>()?;
    for record in &records {
        validate_update_event_link(storage, record).await?;
    }
    Ok(records)
}

pub(crate) async fn read_update_bytes(
    storage: &SurrealStorage,
    update_bytes_ref: &str,
) -> StorageResult<Vec<u8>> {
    read_bytes(storage, UPDATE_TABLE, update_bytes_ref).await
}

pub(crate) async fn append_snapshot(
    storage: &SurrealStorage,
    record: CrdtSnapshotRecordV1,
    snapshot_bytes: Vec<u8>,
) -> StorageResult<CrdtSnapshotRecordV1> {
    validate_crdt_snapshot_record(&record)
        .map_err(|_| StorageError::Validation("invalid kernel CRDT snapshot record"))?;
    if sha256_hex(&snapshot_bytes) != record.snapshot_sha256 {
        return Err(StorageError::Validation(
            "kernel CRDT snapshot bytes do not match snapshot_sha256",
        ));
    }
    let covered_update_seq = i64::try_from(record.covered_update_seq)
        .map_err(|_| StorageError::Validation("kernel CRDT snapshot covered sequence too large"))?;
    let event_id = required_event_id(&record.event_ledger_event_id)?;
    let bindings = SnapshotWriteBindings {
        record: RecordId::new(
            SNAPSHOT_TABLE,
            composite_id(
                "KCS",
                &[
                    &record.workspace_id,
                    &record.document_id,
                    &record.crdt_document_id,
                    &record.snapshot_id,
                ],
            ),
        ),
        schema_id: record.schema_id.clone(),
        snapshot_id: record.snapshot_id.clone(),
        workspace_id: record.workspace_id.clone(),
        document_id: record.document_id.clone(),
        crdt_document_id: record.crdt_document_id.clone(),
        covered_update_seq,
        state_vector: record.state_vector.clone(),
        snapshot_sha256: record.snapshot_sha256.clone(),
        snapshot_bytes_ref: record.snapshot_bytes_ref.clone(),
        snapshot_bytes: Bytes::from(snapshot_bytes),
        actor_id: record.actor_id.clone(),
        actor_kind: record.actor_kind.clone(),
        event_ledger_stream_id: record.event_ledger_stream_id.clone(),
        event_ledger_event_id: RecordId::new(EVENT_TABLE, event_id),
        promotion_evidence_update_ids: record.promotion_evidence_update_ids.clone(),
        storage_authority: authority_str(record.storage_authority)?.to_owned(),
    };
    let result: Result<Vec<SnapshotRow>, _> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at(
                        "IF !record::exists($event_ledger_event_id) { THROW 'HSK-CRDT-EVENT-MISSING'; }; \
                         IF (SELECT VALUE id FROM kernel_crdt_snapshots WHERE workspace_id = $workspace_id AND document_id = $document_id AND crdt_document_id = $crdt_document_id AND snapshot_id = $snapshot_id LIMIT 1)[0] != NONE { \
                             RETURN SELECT * FROM kernel_crdt_snapshots WHERE workspace_id = $workspace_id AND document_id = $document_id AND crdt_document_id = $crdt_document_id AND snapshot_id = $snapshot_id LIMIT 1; \
                         } ELSE { \
                             RETURN CREATE $record CONTENT { schema_id: $schema_id, snapshot_id: $snapshot_id, workspace_id: $workspace_id, document_id: $document_id, crdt_document_id: $crdt_document_id, covered_update_seq: $covered_update_seq, state_vector: $state_vector, snapshot_sha256: $snapshot_sha256, snapshot_bytes_ref: $snapshot_bytes_ref, snapshot_bytes: $snapshot_bytes, actor_id: $actor_id, actor_kind: $actor_kind, event_ledger_stream_id: $event_ledger_stream_id, event_ledger_event_id: $event_ledger_event_id, promotion_evidence_update_ids: $promotion_evidence_update_ids, storage_authority: $storage_authority }; \
                         };",
                        bindings,
                        1,
                    )
                    .await
            })
        })
        .await;
    let stored = result
        .map_err(map_write_error)?
        .into_iter()
        .next()
        .ok_or_else(|| StorageError::Database("kernel CRDT snapshot write returned no row".into()))
        .and_then(map_snapshot)?;
    if stored != record {
        return Err(StorageError::Conflict(
            "kernel CRDT snapshot idempotency conflict",
        ));
    }
    Ok(stored)
}

pub(crate) async fn list_snapshots(
    storage: &SurrealStorage,
    workspace_id: &str,
    document_id: &str,
    crdt_document_id: &str,
) -> StorageResult<Vec<CrdtSnapshotRecordV1>> {
    let bindings = IdentityBindings {
        workspace_id: workspace_id.to_owned(),
        document_id: document_id.to_owned(),
        crdt_document_id: crdt_document_id.to_owned(),
    };
    let rows: Vec<SnapshotRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT * FROM kernel_crdt_snapshots WHERE workspace_id = $workspace_id AND document_id = $document_id AND crdt_document_id = $crdt_document_id ORDER BY covered_update_seq DESC, snapshot_id ASC;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    rows.into_iter().map(map_snapshot).collect()
}

pub(crate) async fn read_snapshot_bytes(
    storage: &SurrealStorage,
    snapshot_bytes_ref: &str,
) -> StorageResult<Vec<u8>> {
    read_bytes(storage, SNAPSHOT_TABLE, snapshot_bytes_ref).await
}

async fn read_bytes(
    storage: &SurrealStorage,
    table: &'static str,
    bytes_ref: &str,
) -> StorageResult<Vec<u8>> {
    let statement = match table {
        UPDATE_TABLE => {
            "SELECT update_bytes AS bytes FROM kernel_crdt_updates WHERE update_bytes_ref = $bytes_ref LIMIT 1;"
        }
        SNAPSHOT_TABLE => {
            "SELECT snapshot_bytes AS bytes FROM kernel_crdt_snapshots WHERE snapshot_bytes_ref = $bytes_ref LIMIT 1;"
        }
        _ => return Err(StorageError::Validation("invalid kernel CRDT byte table")),
    };
    let bytes_ref = bytes_ref.to_owned();
    let row: Option<BytesRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(statement, BytesRefBinding { bytes_ref })
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    row.map(|row| row.bytes.into_inner().to_vec())
        .ok_or(StorageError::NotFound("kernel CRDT bytes"))
}

fn map_update(row: UpdateRow) -> StorageResult<CrdtUpdateRecordV1> {
    Ok(CrdtUpdateRecordV1 {
        schema_id: row.schema_id,
        workspace_id: row.workspace_id,
        document_id: row.document_id,
        crdt_document_id: row.crdt_document_id,
        update_id: row.update_id,
        update_seq: u64::try_from(row.update_seq)
            .map_err(|_| StorageError::Validation("invalid kernel CRDT update sequence"))?,
        update_sha256: row.update_sha256,
        update_bytes_ref: row.update_bytes_ref,
        actor_id: row.actor_id,
        actor_kind: row.actor_kind,
        session_id: row.session_id,
        trace_id: row.trace_id,
        state_vector_before: row.state_vector_before,
        state_vector_after: row.state_vector_after,
        replay_metadata: serde_json::from_value::<CrdtReplayMetadataV1>(row.replay_metadata_json)?,
        event_ledger_stream_id: row.event_ledger_stream_id,
        event_ledger_event_id: record_key(row.event_ledger_event_id, "kernel CRDT update event")?,
        storage_authority: parse_authority(&row.storage_authority)?,
    })
}

fn map_snapshot(row: SnapshotRow) -> StorageResult<CrdtSnapshotRecordV1> {
    Ok(CrdtSnapshotRecordV1 {
        schema_id: row.schema_id,
        snapshot_id: row.snapshot_id,
        workspace_id: row.workspace_id,
        document_id: row.document_id,
        crdt_document_id: row.crdt_document_id,
        covered_update_seq: u64::try_from(row.covered_update_seq)
            .map_err(|_| StorageError::Validation("invalid kernel CRDT snapshot sequence"))?,
        state_vector: row.state_vector,
        snapshot_sha256: row.snapshot_sha256,
        snapshot_bytes_ref: row.snapshot_bytes_ref,
        actor_id: row.actor_id,
        actor_kind: row.actor_kind,
        event_ledger_stream_id: row.event_ledger_stream_id,
        event_ledger_event_id: record_key(row.event_ledger_event_id, "kernel CRDT snapshot event")?,
        promotion_evidence_update_ids: row.promotion_evidence_update_ids,
        storage_authority: parse_authority(&row.storage_authority)?,
    })
}

fn required_event_id(value: &str) -> StorageResult<String> {
    let value = value.trim();
    if value.is_empty() {
        Err(StorageError::Validation(
            "kernel CRDT EventLedger event ref is missing",
        ))
    } else {
        Ok(value.to_owned())
    }
}

fn composite_id(prefix: &str, components: &[&str]) -> String {
    let mut hash = Sha256::new();
    for component in components {
        hash.update(component.as_bytes());
        hash.update([0]);
    }
    format!("{prefix}-{}", &hex::encode(hash.finalize())[..32])
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn authority_str(authority: CrdtStorageAuthorityPosture) -> StorageResult<&'static str> {
    match authority {
        CrdtStorageAuthorityPosture::EmbeddedSurrealDb => Ok("embedded_surreal_db"),
        _ => Err(StorageError::Validation(
            "kernel CRDT writes require embedded SurrealDB plus EventLedger authority",
        )),
    }
}

fn parse_authority(value: &str) -> StorageResult<CrdtStorageAuthorityPosture> {
    match value {
        "embedded_surreal_db" => Ok(CrdtStorageAuthorityPosture::EmbeddedSurrealDb),
        _ => Err(StorageError::Validation("invalid CRDT storage authority")),
    }
}

fn record_key(record: RecordId, context: &'static str) -> StorageResult<String> {
    match record.key {
        RecordIdKey::String(value) => Ok(value),
        _ => Err(StorageError::Database(format!(
            "{context} has a non-string record id"
        ))),
    }
}

fn map_write_error(error: super::SurrealStorageError) -> StorageError {
    let message = error.to_string();
    if message.contains("HSK-CRDT-EVENT-MISSING") {
        StorageError::Validation("kernel CRDT EventLedger event ref is missing")
    } else if message.contains("kernel_crdt_updates")
        || message.contains("kernel_crdt_snapshots")
        || message.contains("idx_kernel_crdt")
        || message.contains("pk_kernel_crdt")
    {
        StorageError::Conflict("kernel CRDT persistence uniqueness conflict")
    } else {
        StorageError::Database(message)
    }
}

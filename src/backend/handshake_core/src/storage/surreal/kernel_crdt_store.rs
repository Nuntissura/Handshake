//! Durable embedded-SurrealDB persistence for kernel CRDT updates and snapshots.

use sha2::{Digest, Sha256};
use surrealdb::types::{Bytes, RecordId, RecordIdKey, SurrealValue};

use super::SurrealStorage;
use crate::kernel::crdt::{
    persistence::{
        validate_crdt_update_record, CrdtReplayMetadataV1, CrdtStorageAuthorityPosture,
        CrdtUpdateRecordV1,
    },
    snapshot::{validate_crdt_snapshot_record, CrdtSnapshotRecordV1},
};
use crate::storage::{StorageError, StorageResult};

const UPDATE_TABLE: &str = "kernel_crdt_updates";
const SNAPSHOT_TABLE: &str = "kernel_crdt_snapshots";
const EVENT_TABLE: &str = "kernel_event_ledger";

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
    let update_seq = i64::try_from(record.update_seq)
        .map_err(|_| StorageError::Validation("kernel CRDT update sequence too large"))?;
    let event_id = required_event_id(&record.event_ledger_event_id)?;
    let bindings = UpdateWriteBindings {
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
        storage_authority: authority_str(record.storage_authority).to_owned(),
    };
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
    rows.into_iter().map(map_update).collect()
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
        storage_authority: authority_str(record.storage_authority).to_owned(),
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

fn authority_str(authority: CrdtStorageAuthorityPosture) -> &'static str {
    match authority {
        CrdtStorageAuthorityPosture::SurrealEventLedger => "surreal_event_ledger",
        CrdtStorageAuthorityPosture::FileSystemAuthority => "filesystem_authority",
        CrdtStorageAuthorityPosture::MemoryOnly => "memory_only",
    }
}

fn parse_authority(value: &str) -> StorageResult<CrdtStorageAuthorityPosture> {
    match value {
        // Read-only compatibility for rows written before the embedded-store
        // authority wire was corrected. `authority_str` never emits the alias.
        "surreal_event_ledger" | "postgres_event_ledger" => {
            Ok(CrdtStorageAuthorityPosture::SurrealEventLedger)
        }
        "filesystem_authority" => Ok(CrdtStorageAuthorityPosture::FileSystemAuthority),
        "memory_only" => Ok(CrdtStorageAuthorityPosture::MemoryOnly),
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

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::identity::CrdtWorkspaceIdentityV1;

pub const CRDT_UPDATE_RECORD_SCHEMA_ID: &str = "hsk.kernel.crdt_update_record@1";
pub const CRDT_REPLAY_PLAN_SCHEMA_ID: &str = "hsk.kernel.crdt_replay_plan@1";
pub const CRDT_POSTGRES_UPDATE_LOG_CONTRACT_SCHEMA_ID: &str =
    "hsk.kernel.crdt_postgres_update_log_contract@1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrdtStorageAuthorityPosture {
    PostgresEventLedger,
    FileSystemAuthority,
    MemoryOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtReplayMetadataV1 {
    pub replay_order_key: String,
    pub dependency_update_ids: Vec<String>,
    pub encoding: String,
    pub schema_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtUpdateRecordV1 {
    pub schema_id: String,
    pub workspace_id: String,
    pub document_id: String,
    pub crdt_document_id: String,
    pub update_id: String,
    pub update_seq: u64,
    pub update_sha256: String,
    pub update_bytes_ref: String,
    pub actor_id: String,
    pub actor_kind: String,
    pub session_id: String,
    pub trace_id: String,
    pub state_vector_before: String,
    pub state_vector_after: String,
    pub replay_metadata: CrdtReplayMetadataV1,
    pub event_ledger_stream_id: String,
    pub event_ledger_event_id: String,
    pub storage_authority: CrdtStorageAuthorityPosture,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrdtUpdateRecordInputV1<'a> {
    pub identity: &'a CrdtWorkspaceIdentityV1,
    pub update_id: &'a str,
    pub update_seq: u64,
    pub update_bytes: &'a [u8],
    pub update_bytes_ref: &'a str,
    pub session_id: &'a str,
    pub trace_id: &'a str,
    pub state_vector_before: &'a str,
    pub state_vector_after: &'a str,
    pub replay_metadata: CrdtReplayMetadataV1,
    pub event_ledger_event_id: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtReplayStepV1 {
    pub update_id: String,
    pub update_seq: u64,
    pub update_sha256: String,
    pub update_bytes_ref: String,
    pub actor_id: String,
    pub actor_kind: String,
    pub session_id: String,
    pub trace_id: String,
    pub state_vector_after: String,
    pub event_ledger_event_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtReplayPlanV1 {
    pub schema_id: String,
    pub workspace_id: String,
    pub document_id: String,
    pub crdt_document_id: String,
    pub source_authority: CrdtStorageAuthorityPosture,
    pub ordered_updates: Vec<CrdtReplayStepV1>,
    pub final_state_vector: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtPostgresUpdateLogContractV1 {
    pub schema_id: String,
    pub table_name: &'static str,
    pub storage_authority: CrdtStorageAuthorityPosture,
    pub required_columns: Vec<&'static str>,
    pub unique_constraints: Vec<&'static str>,
    pub replay_order: &'static str,
    pub denied_authority_refs: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrdtUpdateRecordValidationError {
    pub field: &'static str,
    pub message: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrdtReplayPlanError {
    EmptyLog,
    InvalidRecord {
        update_id: String,
        errors: Vec<CrdtUpdateRecordValidationError>,
    },
    MixedIdentity {
        field: &'static str,
        expected: String,
        found: String,
    },
    DuplicateUpdateId {
        update_id: String,
    },
    DuplicateSequence {
        update_seq: u64,
    },
    SequenceGap {
        expected: u64,
        found: u64,
    },
}

pub fn new_crdt_update_record(input: CrdtUpdateRecordInputV1<'_>) -> CrdtUpdateRecordV1 {
    CrdtUpdateRecordV1 {
        schema_id: CRDT_UPDATE_RECORD_SCHEMA_ID.to_string(),
        workspace_id: input.identity.workspace_id.clone(),
        document_id: input.identity.document_id.clone(),
        crdt_document_id: input.identity.crdt_document_id.clone(),
        update_id: input.update_id.to_string(),
        update_seq: input.update_seq,
        update_sha256: sha256_hex(input.update_bytes),
        update_bytes_ref: input.update_bytes_ref.to_string(),
        actor_id: input.identity.actor_id.clone(),
        actor_kind: input.identity.actor_kind.clone(),
        session_id: input.session_id.to_string(),
        trace_id: input.trace_id.to_string(),
        state_vector_before: input.state_vector_before.to_string(),
        state_vector_after: input.state_vector_after.to_string(),
        replay_metadata: input.replay_metadata,
        event_ledger_stream_id: input
            .identity
            .authority_links
            .event_ledger_stream_id
            .clone(),
        event_ledger_event_id: input.event_ledger_event_id.to_string(),
        storage_authority: CrdtStorageAuthorityPosture::PostgresEventLedger,
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

pub fn kernel_crdt_postgres_update_log_contract() -> CrdtPostgresUpdateLogContractV1 {
    CrdtPostgresUpdateLogContractV1 {
        schema_id: CRDT_POSTGRES_UPDATE_LOG_CONTRACT_SCHEMA_ID.to_string(),
        table_name: "kernel_crdt_updates",
        storage_authority: CrdtStorageAuthorityPosture::PostgresEventLedger,
        required_columns: vec![
            "workspace_id",
            "document_id",
            "crdt_document_id",
            "update_id",
            "update_seq",
            "update_sha256",
            "update_bytes_ref",
            "actor_id",
            "actor_kind",
            "session_id",
            "trace_id",
            "state_vector_before",
            "state_vector_after",
            "replay_metadata_json",
            "event_ledger_stream_id",
            "event_ledger_event_id",
        ],
        unique_constraints: vec![
            "workspace_id,document_id,crdt_document_id,update_seq",
            "workspace_id,document_id,crdt_document_id,update_id",
            "event_ledger_stream_id,event_ledger_event_id",
        ],
        replay_order: "workspace_id,document_id,crdt_document_id,update_seq",
        denied_authority_refs: vec![
            "filesystem_update_bytes",
            "markdown_update_log",
            "browser_local_storage_authority",
        ],
    }
}

pub fn validate_crdt_update_record(
    record: &CrdtUpdateRecordV1,
) -> Result<(), Vec<CrdtUpdateRecordValidationError>> {
    let mut errors = Vec::new();

    require_non_empty(&mut errors, "schema_id", &record.schema_id);
    require_non_empty(&mut errors, "workspace_id", &record.workspace_id);
    require_non_empty(&mut errors, "document_id", &record.document_id);
    require_non_empty(&mut errors, "crdt_document_id", &record.crdt_document_id);
    require_non_empty(&mut errors, "update_id", &record.update_id);
    require_non_empty(&mut errors, "update_sha256", &record.update_sha256);
    require_non_empty(&mut errors, "update_bytes_ref", &record.update_bytes_ref);
    require_non_empty(&mut errors, "actor_id", &record.actor_id);
    require_non_empty(&mut errors, "actor_kind", &record.actor_kind);
    require_non_empty(&mut errors, "session_id", &record.session_id);
    require_non_empty(&mut errors, "trace_id", &record.trace_id);
    require_non_empty(
        &mut errors,
        "state_vector_before",
        &record.state_vector_before,
    );
    require_non_empty(
        &mut errors,
        "state_vector_after",
        &record.state_vector_after,
    );
    require_non_empty(
        &mut errors,
        "replay_metadata.replay_order_key",
        &record.replay_metadata.replay_order_key,
    );
    require_non_empty(
        &mut errors,
        "replay_metadata.encoding",
        &record.replay_metadata.encoding,
    );
    require_non_empty(
        &mut errors,
        "replay_metadata.schema_version",
        &record.replay_metadata.schema_version,
    );
    require_non_empty(
        &mut errors,
        "event_ledger_stream_id",
        &record.event_ledger_stream_id,
    );
    require_non_empty(
        &mut errors,
        "event_ledger_event_id",
        &record.event_ledger_event_id,
    );

    if record.schema_id != CRDT_UPDATE_RECORD_SCHEMA_ID {
        errors.push(CrdtUpdateRecordValidationError {
            field: "schema_id",
            message: "unexpected CRDT update record schema",
        });
    }
    if record.update_seq == 0 {
        errors.push(CrdtUpdateRecordValidationError {
            field: "update_seq",
            message: "update sequence starts at 1",
        });
    }
    if !is_sha256_hex(&record.update_sha256) {
        errors.push(CrdtUpdateRecordValidationError {
            field: "update_sha256",
            message: "value must be a 64-character sha256 hex digest",
        });
    }
    if !record.update_bytes_ref.starts_with("postgres://") {
        errors.push(CrdtUpdateRecordValidationError {
            field: "update_bytes_ref",
            message: "CRDT update bytes must be referenced from Postgres storage",
        });
    }
    if record.storage_authority != CrdtStorageAuthorityPosture::PostgresEventLedger {
        errors.push(CrdtUpdateRecordValidationError {
            field: "storage_authority",
            message: "CRDT update authority must be Postgres plus EventLedger",
        });
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn build_crdt_replay_plan(
    records: &[CrdtUpdateRecordV1],
) -> Result<CrdtReplayPlanV1, CrdtReplayPlanError> {
    if records.is_empty() {
        return Err(CrdtReplayPlanError::EmptyLog);
    }

    for record in records {
        if let Err(errors) = validate_crdt_update_record(record) {
            return Err(CrdtReplayPlanError::InvalidRecord {
                update_id: record.update_id.clone(),
                errors,
            });
        }
    }

    let mut ordered = records.to_vec();
    ordered.sort_by_key(|record| record.update_seq);

    let first = ordered.first().expect("non-empty records checked above");
    let mut seen_update_ids = HashSet::new();
    let mut seen_sequences = HashSet::new();

    for record in &ordered {
        require_same_identity("workspace_id", &first.workspace_id, &record.workspace_id)?;
        require_same_identity("document_id", &first.document_id, &record.document_id)?;
        require_same_identity(
            "crdt_document_id",
            &first.crdt_document_id,
            &record.crdt_document_id,
        )?;

        if !seen_update_ids.insert(record.update_id.clone()) {
            return Err(CrdtReplayPlanError::DuplicateUpdateId {
                update_id: record.update_id.clone(),
            });
        }
        if !seen_sequences.insert(record.update_seq) {
            return Err(CrdtReplayPlanError::DuplicateSequence {
                update_seq: record.update_seq,
            });
        }
    }

    for (index, record) in ordered.iter().enumerate() {
        let expected = index as u64 + 1;
        if record.update_seq != expected {
            return Err(CrdtReplayPlanError::SequenceGap {
                expected,
                found: record.update_seq,
            });
        }
    }

    let final_state_vector = ordered
        .last()
        .expect("non-empty records checked above")
        .state_vector_after
        .clone();

    Ok(CrdtReplayPlanV1 {
        schema_id: CRDT_REPLAY_PLAN_SCHEMA_ID.to_string(),
        workspace_id: first.workspace_id.clone(),
        document_id: first.document_id.clone(),
        crdt_document_id: first.crdt_document_id.clone(),
        source_authority: CrdtStorageAuthorityPosture::PostgresEventLedger,
        ordered_updates: ordered.into_iter().map(replay_step).collect(),
        final_state_vector,
    })
}

fn replay_step(record: CrdtUpdateRecordV1) -> CrdtReplayStepV1 {
    CrdtReplayStepV1 {
        update_id: record.update_id,
        update_seq: record.update_seq,
        update_sha256: record.update_sha256,
        update_bytes_ref: record.update_bytes_ref,
        actor_id: record.actor_id,
        actor_kind: record.actor_kind,
        session_id: record.session_id,
        trace_id: record.trace_id,
        state_vector_after: record.state_vector_after,
        event_ledger_event_id: record.event_ledger_event_id,
    }
}

fn require_same_identity(
    field: &'static str,
    expected: &str,
    found: &str,
) -> Result<(), CrdtReplayPlanError> {
    if expected == found {
        Ok(())
    } else {
        Err(CrdtReplayPlanError::MixedIdentity {
            field,
            expected: expected.to_string(),
            found: found.to_string(),
        })
    }
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn require_non_empty(
    errors: &mut Vec<CrdtUpdateRecordValidationError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(CrdtUpdateRecordValidationError {
            field,
            message: "value must not be empty",
        });
    }
}

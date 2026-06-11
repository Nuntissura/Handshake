//! WP-KERNEL-009 MT-066 CRDTAndConcurrencyCore-066-RichDocumentCrdtSnapshotModel.
//!
//! Master Spec anchor: 02-system-architecture.md section 2.3.13.11 —
//! `RichDocument`: "a versioned ProseMirror/Tiptap document JSON authority
//! record, with schema version, CRDT snapshot refs, EventLedger promotion
//! refs, and projection refs."
//!
//! This module defines what the bytes inside a kernel CRDT snapshot
//! (`kernel_crdt_snapshots.snapshot_bytes`, migration 0020) mean for a
//! ProjectKnowledgeIndex rich document: a schema-version-stamped
//! ProseMirror/Tiptap doc JSON plus the typed state vector and the covered
//! update sequence. The existing `CrdtSnapshotRecordV1` stays the durable
//! envelope (sha256, Postgres byte ref, EventLedger event ref, promotion
//! evidence); the payload here is the restore-path contract: parse, hash
//! check, structural validation, and cross-checks against the envelope.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::identity::CrdtWorkspaceIdentityV1;
use super::persistence::sha256_hex;
use super::snapshot::{
    new_crdt_snapshot_record, validate_crdt_snapshot_record, CrdtSnapshotRecordInputV1,
    CrdtSnapshotRecordV1,
};
use super::state_vector::KnowledgeStateVectorV1;

pub const RICH_DOCUMENT_SNAPSHOT_PAYLOAD_SCHEMA_ID: &str =
    "hsk.kernel.knowledge_rich_document_snapshot_payload@1";
pub const RICH_DOCUMENT_SCHEMA_ID: &str = "hsk.doc.rich_document@1";

/// The decoded content of a rich-document CRDT snapshot. Serialized with
/// canonical serde_json and stored as `kernel_crdt_snapshots.snapshot_bytes`
/// in PostgreSQL; never on the filesystem, never in browser storage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RichDocumentSnapshotPayloadV1 {
    pub schema_id: String,
    /// Document family schema id (e.g. `hsk.doc.rich_document@1`).
    pub document_schema_id: String,
    /// ProseMirror/Tiptap editor schema version stamp, e.g.
    /// `tiptap-starter-kit@3.13.0`. Restores MUST surface this so the
    /// frontend can refuse or migrate incompatible documents.
    pub prosemirror_schema_version: String,
    /// The ProseMirror document JSON (`{"type":"doc","content":[...]}`).
    pub doc_json: Value,
    /// Canonical typed state vector (`hsk-sv1:`) materialized by this
    /// snapshot; must match the envelope's `state_vector` column.
    pub state_vector: String,
    /// Highest update_seq folded into this snapshot; must match the
    /// envelope's `covered_update_seq` column.
    pub covered_update_seq: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RichDocumentSnapshotValidationError {
    pub field: &'static str,
    pub message: String,
}

/// Structural validation for a rich-document snapshot payload.
pub fn validate_rich_document_snapshot_payload(
    payload: &RichDocumentSnapshotPayloadV1,
) -> Result<(), Vec<RichDocumentSnapshotValidationError>> {
    let mut errors = Vec::new();

    if payload.schema_id != RICH_DOCUMENT_SNAPSHOT_PAYLOAD_SCHEMA_ID {
        errors.push(RichDocumentSnapshotValidationError {
            field: "schema_id",
            message: format!(
                "expected {RICH_DOCUMENT_SNAPSHOT_PAYLOAD_SCHEMA_ID}, found '{}'",
                payload.schema_id
            ),
        });
    }
    if payload.document_schema_id.trim().is_empty() {
        errors.push(RichDocumentSnapshotValidationError {
            field: "document_schema_id",
            message: "document schema id must not be empty".to_string(),
        });
    }
    if payload.prosemirror_schema_version.trim().is_empty() {
        errors.push(RichDocumentSnapshotValidationError {
            field: "prosemirror_schema_version",
            message: "editor schema version stamp must not be empty".to_string(),
        });
    }
    match payload.doc_json.as_object() {
        None => errors.push(RichDocumentSnapshotValidationError {
            field: "doc_json",
            message: "doc_json must be a JSON object".to_string(),
        }),
        Some(map) => {
            if map.get("type").and_then(Value::as_str) != Some("doc") {
                errors.push(RichDocumentSnapshotValidationError {
                    field: "doc_json.type",
                    message: "ProseMirror root node type must be 'doc'".to_string(),
                });
            }
            if let Some(content) = map.get("content") {
                if !content.is_array() {
                    errors.push(RichDocumentSnapshotValidationError {
                        field: "doc_json.content",
                        message: "ProseMirror doc content must be an array when present"
                            .to_string(),
                    });
                }
            }
        }
    }
    if let Err(parse_error) = KnowledgeStateVectorV1::parse(&payload.state_vector) {
        errors.push(RichDocumentSnapshotValidationError {
            field: "state_vector",
            message: format!("not a typed knowledge state vector: {parse_error}"),
        });
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Deterministic Postgres byte ref for a rich-document snapshot, matching
/// the `postgres://kernel_crdt_snapshots/...` convention enforced by
/// `validate_crdt_snapshot_record`.
pub fn rich_document_snapshot_bytes_ref(crdt_document_id: &str, snapshot_id: &str) -> String {
    format!("postgres://kernel_crdt_snapshots/{crdt_document_id}/{snapshot_id}/snapshot_bytes")
}

/// Build the durable snapshot envelope + payload bytes for a rich document.
///
/// Returns the `CrdtSnapshotRecordV1` (ready for
/// `Database::append_kernel_crdt_snapshot`) and the exact payload bytes whose
/// sha256 the envelope carries.
pub fn build_rich_document_snapshot_record(
    identity: &CrdtWorkspaceIdentityV1,
    snapshot_id: &str,
    payload: &RichDocumentSnapshotPayloadV1,
    event_ledger_event_id: &str,
    promotion_evidence_update_ids: &[&str],
) -> Result<(CrdtSnapshotRecordV1, Vec<u8>), Vec<RichDocumentSnapshotValidationError>> {
    validate_rich_document_snapshot_payload(payload)?;
    if identity.document_schema_id != payload.document_schema_id {
        return Err(vec![RichDocumentSnapshotValidationError {
            field: "document_schema_id",
            message: format!(
                "identity document schema '{}' does not match payload '{}'",
                identity.document_schema_id, payload.document_schema_id
            ),
        }]);
    }
    let bytes = serde_json::to_vec(payload).map_err(|error| {
        vec![RichDocumentSnapshotValidationError {
            field: "payload",
            message: format!("payload serialization failed: {error}"),
        }]
    })?;
    let bytes_ref = rich_document_snapshot_bytes_ref(&identity.crdt_document_id, snapshot_id);
    let record = new_crdt_snapshot_record(CrdtSnapshotRecordInputV1 {
        identity,
        snapshot_id,
        covered_update_seq: payload.covered_update_seq,
        snapshot_bytes: &bytes,
        snapshot_bytes_ref: &bytes_ref,
        state_vector: &payload.state_vector,
        event_ledger_event_id,
        promotion_evidence_update_ids,
    });
    Ok((record, bytes))
}

/// A fully verified restore of a rich document from its persisted snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RichDocumentRestoreV1 {
    pub snapshot_id: String,
    pub workspace_id: String,
    pub document_id: String,
    pub crdt_document_id: String,
    pub document_schema_id: String,
    pub prosemirror_schema_version: String,
    pub doc_json: Value,
    pub state_vector: String,
    pub covered_update_seq: u64,
    pub snapshot_sha256: String,
    pub event_ledger_event_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RichDocumentRestoreError {
    InvalidSnapshotRecord {
        fields: Vec<&'static str>,
    },
    ByteHashMismatch {
        expected: String,
        found: String,
    },
    PayloadParse {
        message: String,
    },
    PayloadInvalid {
        errors: Vec<RichDocumentSnapshotValidationError>,
    },
    EnvelopePayloadDrift {
        field: &'static str,
        envelope: String,
        payload: String,
    },
}

impl std::fmt::Display for RichDocumentRestoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSnapshotRecord { fields } => {
                write!(f, "snapshot record failed validation on fields {fields:?}")
            }
            Self::ByteHashMismatch { expected, found } => write!(
                f,
                "snapshot bytes hash mismatch: envelope says {expected}, bytes hash to {found}"
            ),
            Self::PayloadParse { message } => {
                write!(f, "snapshot payload is not valid JSON: {message}")
            }
            Self::PayloadInvalid { errors } => {
                write!(f, "snapshot payload failed validation: {errors:?}")
            }
            Self::EnvelopePayloadDrift {
                field,
                envelope,
                payload,
            } => write!(
                f,
                "snapshot envelope/payload drift on {field}: envelope '{envelope}' vs payload '{payload}'"
            ),
        }
    }
}

impl std::error::Error for RichDocumentRestoreError {}

/// Restore path: verify the envelope, the byte hash, the payload structure,
/// and the envelope/payload cross-fields, then hand back the typed document.
pub fn restore_rich_document_snapshot(
    record: &CrdtSnapshotRecordV1,
    snapshot_bytes: &[u8],
) -> Result<RichDocumentRestoreV1, RichDocumentRestoreError> {
    if let Err(errors) = validate_crdt_snapshot_record(record) {
        return Err(RichDocumentRestoreError::InvalidSnapshotRecord {
            fields: errors.into_iter().map(|error| error.field).collect(),
        });
    }
    let found_hash = sha256_hex(snapshot_bytes);
    if found_hash != record.snapshot_sha256 {
        return Err(RichDocumentRestoreError::ByteHashMismatch {
            expected: record.snapshot_sha256.clone(),
            found: found_hash,
        });
    }
    let payload: RichDocumentSnapshotPayloadV1 =
        serde_json::from_slice(snapshot_bytes).map_err(|error| {
            RichDocumentRestoreError::PayloadParse {
                message: error.to_string(),
            }
        })?;
    validate_rich_document_snapshot_payload(&payload)
        .map_err(|errors| RichDocumentRestoreError::PayloadInvalid { errors })?;
    if payload.state_vector != record.state_vector {
        return Err(RichDocumentRestoreError::EnvelopePayloadDrift {
            field: "state_vector",
            envelope: record.state_vector.clone(),
            payload: payload.state_vector,
        });
    }
    if payload.covered_update_seq != record.covered_update_seq {
        return Err(RichDocumentRestoreError::EnvelopePayloadDrift {
            field: "covered_update_seq",
            envelope: record.covered_update_seq.to_string(),
            payload: payload.covered_update_seq.to_string(),
        });
    }

    Ok(RichDocumentRestoreV1 {
        snapshot_id: record.snapshot_id.clone(),
        workspace_id: record.workspace_id.clone(),
        document_id: record.document_id.clone(),
        crdt_document_id: record.crdt_document_id.clone(),
        document_schema_id: payload.document_schema_id,
        prosemirror_schema_version: payload.prosemirror_schema_version,
        doc_json: payload.doc_json,
        state_vector: payload.state_vector,
        covered_update_seq: payload.covered_update_seq,
        snapshot_sha256: record.snapshot_sha256.clone(),
        event_ledger_event_id: record.event_ledger_event_id.clone(),
    })
}

//! WP-KERNEL-009 MT-067 CRDTAndConcurrencyCore-067-TiptapYjsBridgeContract.
//!
//! Master Spec anchor: 02-system-architecture.md section 2.3.13.11 — rich
//! document edits MAY use CRDT state for drafting, but PostgreSQL plus
//! EventLedger is the only durable authority; browser state is never
//! authority. This module is the typed bridge between frontend Yjs binary
//! updates (yjs 13.x / @tiptap/extension-collaboration) and the kernel CRDT
//! update log (`kernel_crdt_updates`, migration 0020).
//!
//! Contract shape:
//!   * [`YjsUpdateEnvelopeV1`] — one Yjs update as it crosses the HTTP
//!     boundary: ids, typed actor/site attribution, base64 update bytes,
//!     sha256, typed state vectors before/after.
//!   * [`push_yjs_update`] — server-side ingest: validates the envelope,
//!     enforces the linear draft log (stale base => typed denial, never a
//!     silent overwrite), appends the EventLedger receipt and the Postgres
//!     update row. Idempotent on `update_id`.
//!   * [`pull_yjs_updates`] — replay feed for reconnecting editors: returns
//!     envelopes (bytes re-encoded from Postgres) strictly ordered by
//!     `update_seq`, plus the head sequence and head state vector.
//!
//! The update bytes are opaque to the backend (no `yrs` dependency): the
//! backend orders, attributes, hashes, persists, and replays them; merging
//! is the Yjs client's job. That keeps the backend free of any external
//! CRDT service while staying byte-compatible with the bundled frontend
//! stack (MT-078 proves the no-external-relay posture).

use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
};

use base64::Engine;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::kernel::{KernelEventType, NewKernelEvent};
use crate::storage::Database;

use super::actor_site::{
    KnowledgeActorIdError, KnowledgeActorIdV1, derive_knowledge_site_id, knowledge_crdt_identity,
};
use super::persistence::{
    CrdtReplayMetadataV1, CrdtUpdateRecordInputV1, CrdtUpdateRecordV1, new_crdt_update_record,
    sha256_hex,
};
use super::state_vector::{
    KnowledgeStateVectorOrdering, KnowledgeStateVectorParseError, KnowledgeStateVectorV1,
};

pub const YJS_UPDATE_ENVELOPE_SCHEMA_ID: &str = "hsk.kernel.knowledge_yjs_update_envelope@1";
pub const YJS_UPDATE_ENCODING_V1: &str = "yjs-update-v1";
pub const YJS_PUSH_DENIAL_SCHEMA_ID: &str = "hsk.kernel.knowledge_yjs_push_denial@1";

fn b64() -> base64::engine::general_purpose::GeneralPurpose {
    base64::engine::general_purpose::STANDARD
}

type DocumentPushLocks = Mutex<HashMap<String, Arc<Mutex<()>>>>;

fn document_push_locks() -> &'static DocumentPushLocks {
    static LOCKS: OnceLock<DocumentPushLocks> = OnceLock::new();
    LOCKS.get_or_init(|| Mutex::new(HashMap::new()))
}

// The Handshake backend is the loopback CRDT ingress point. Serializing one
// document inside the process keeps same-base pushes from producing orphaned
// EventLedger update events before the PostgreSQL sequence index rejects a
// loser. The DB uniqueness constraint remains the cross-process backstop.
async fn push_lock_for_document(envelope: &YjsUpdateEnvelopeV1) -> Arc<Mutex<()>> {
    let key = format!(
        "{}\u{0}{}\u{0}{}",
        envelope.workspace_id, envelope.document_id, envelope.crdt_document_id
    );
    let mut locks = document_push_locks().lock().await;
    locks
        .entry(key)
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone()
}

/// One Yjs update crossing the frontend/backend boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct YjsUpdateEnvelopeV1 {
    pub schema_id: String,
    pub workspace_id: String,
    pub document_id: String,
    pub crdt_document_id: String,
    /// Client-generated stable id for this update (idempotency token).
    pub update_id: String,
    /// Canonical typed actor id (`kind:ident`, MT-065).
    pub actor_id: String,
    /// Stable CRDT site id; MUST equal the MT-065 derivation for
    /// (workspace, crdt document, actor).
    pub site_id: String,
    pub session_id: String,
    pub trace_id: String,
    pub document_schema_id: String,
    /// Yjs binary update, base64 (standard alphabet, padded).
    pub update_b64: String,
    /// sha256 hex of the decoded update bytes.
    pub update_sha256: String,
    /// Typed state vector the client had applied before this update.
    pub state_vector_before: String,
    /// Typed state vector after this update (must strictly dominate before).
    pub state_vector_after: String,
    pub encoding: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum YjsEnvelopeValidationError {
    EmptyField {
        field: &'static str,
    },
    WrongSchemaId {
        found: String,
    },
    WrongEncoding {
        found: String,
    },
    ActorIdInvalid {
        error: KnowledgeActorIdError,
    },
    SiteIdMismatch {
        expected: String,
        found: String,
    },
    UpdateBytesNotBase64 {
        message: String,
    },
    UpdateBytesEmpty,
    UpdateHashMismatch {
        expected: String,
        found: String,
    },
    StateVectorInvalid {
        field: &'static str,
        error: KnowledgeStateVectorParseError,
    },
    AfterDoesNotDominateBefore {
        ordering: KnowledgeStateVectorOrdering,
    },
    AfterDoesNotAdvanceOwnSite {
        site_id: String,
    },
}

impl std::fmt::Display for YjsEnvelopeValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyField { field } => write!(f, "envelope field {field} must not be empty"),
            Self::WrongSchemaId { found } => write!(
                f,
                "envelope schema id '{found}' is not {YJS_UPDATE_ENVELOPE_SCHEMA_ID}"
            ),
            Self::WrongEncoding { found } => {
                write!(
                    f,
                    "envelope encoding '{found}' is not {YJS_UPDATE_ENCODING_V1}"
                )
            }
            Self::ActorIdInvalid { error } => write!(f, "actor id invalid: {error}"),
            Self::SiteIdMismatch { expected, found } => write!(
                f,
                "site id '{found}' does not match the deterministic derivation '{expected}' for this actor/document"
            ),
            Self::UpdateBytesNotBase64 { message } => {
                write!(f, "update_b64 does not decode: {message}")
            }
            Self::UpdateBytesEmpty => write!(f, "update bytes must not be empty"),
            Self::UpdateHashMismatch { expected, found } => write!(
                f,
                "update bytes hash to {found} but envelope claims {expected}"
            ),
            Self::StateVectorInvalid { field, error } => {
                write!(f, "{field} is not a typed state vector: {error}")
            }
            Self::AfterDoesNotDominateBefore { ordering } => write!(
                f,
                "state_vector_after must strictly dominate state_vector_before (got {ordering:?})"
            ),
            Self::AfterDoesNotAdvanceOwnSite { site_id } => write!(
                f,
                "state_vector_after must advance the sender's own site '{site_id}'"
            ),
        }
    }
}

impl std::error::Error for YjsEnvelopeValidationError {}

/// Validated view of an envelope: decoded bytes + typed vectors + actor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedYjsUpdate {
    pub update_bytes: Vec<u8>,
    pub actor: KnowledgeActorIdV1,
    pub before: KnowledgeStateVectorV1,
    pub after: KnowledgeStateVectorV1,
}

/// Full structural validation of an incoming envelope. Returns the decoded
/// update bytes and typed metadata so callers never re-parse.
pub fn validate_yjs_update_envelope(
    envelope: &YjsUpdateEnvelopeV1,
) -> Result<ValidatedYjsUpdate, Vec<YjsEnvelopeValidationError>> {
    let mut errors = Vec::new();

    for (field, value) in [
        ("workspace_id", &envelope.workspace_id),
        ("document_id", &envelope.document_id),
        ("crdt_document_id", &envelope.crdt_document_id),
        ("update_id", &envelope.update_id),
        ("actor_id", &envelope.actor_id),
        ("site_id", &envelope.site_id),
        ("session_id", &envelope.session_id),
        ("trace_id", &envelope.trace_id),
        ("document_schema_id", &envelope.document_schema_id),
        ("update_b64", &envelope.update_b64),
        ("update_sha256", &envelope.update_sha256),
    ] {
        if value.trim().is_empty() {
            errors.push(YjsEnvelopeValidationError::EmptyField { field });
        }
    }
    if envelope.schema_id != YJS_UPDATE_ENVELOPE_SCHEMA_ID {
        errors.push(YjsEnvelopeValidationError::WrongSchemaId {
            found: envelope.schema_id.clone(),
        });
    }
    if envelope.encoding != YJS_UPDATE_ENCODING_V1 {
        errors.push(YjsEnvelopeValidationError::WrongEncoding {
            found: envelope.encoding.clone(),
        });
    }

    let actor = match KnowledgeActorIdV1::parse(&envelope.actor_id) {
        Ok(actor) => Some(actor),
        Err(error) => {
            errors.push(YjsEnvelopeValidationError::ActorIdInvalid { error });
            None
        }
    };
    if let Some(actor) = &actor {
        let derived =
            derive_knowledge_site_id(&envelope.workspace_id, &envelope.crdt_document_id, actor);
        if derived.site_id != envelope.site_id {
            errors.push(YjsEnvelopeValidationError::SiteIdMismatch {
                expected: derived.site_id,
                found: envelope.site_id.clone(),
            });
        }
    }

    let update_bytes = match b64().decode(envelope.update_b64.as_bytes()) {
        Ok(bytes) => {
            if bytes.is_empty() {
                errors.push(YjsEnvelopeValidationError::UpdateBytesEmpty);
                None
            } else {
                let found = sha256_hex(&bytes);
                if found != envelope.update_sha256 {
                    errors.push(YjsEnvelopeValidationError::UpdateHashMismatch {
                        expected: envelope.update_sha256.clone(),
                        found,
                    });
                    None
                } else {
                    Some(bytes)
                }
            }
        }
        Err(error) => {
            errors.push(YjsEnvelopeValidationError::UpdateBytesNotBase64 {
                message: error.to_string(),
            });
            None
        }
    };

    let before = match KnowledgeStateVectorV1::parse(&envelope.state_vector_before) {
        Ok(vector) => Some(vector),
        Err(error) => {
            errors.push(YjsEnvelopeValidationError::StateVectorInvalid {
                field: "state_vector_before",
                error,
            });
            None
        }
    };
    let after = match KnowledgeStateVectorV1::parse(&envelope.state_vector_after) {
        Ok(vector) => Some(vector),
        Err(error) => {
            errors.push(YjsEnvelopeValidationError::StateVectorInvalid {
                field: "state_vector_after",
                error,
            });
            None
        }
    };
    if let (Some(before), Some(after)) = (&before, &after) {
        let ordering = after.compare(before);
        if ordering != KnowledgeStateVectorOrdering::Dominates {
            errors.push(YjsEnvelopeValidationError::AfterDoesNotDominateBefore { ordering });
        } else if after.clock(&envelope.site_id) <= before.clock(&envelope.site_id) {
            errors.push(YjsEnvelopeValidationError::AfterDoesNotAdvanceOwnSite {
                site_id: envelope.site_id.clone(),
            });
        }
    }

    match (update_bytes, actor, before, after) {
        (Some(update_bytes), Some(actor), Some(before), Some(after)) if errors.is_empty() => {
            Ok(ValidatedYjsUpdate {
                update_bytes,
                actor,
                before,
                after,
            })
        }
        _ => Err(errors),
    }
}

/// Build the persistable update record for a validated envelope at the
/// server-assigned `update_seq`.
pub fn envelope_to_update_record(
    envelope: &YjsUpdateEnvelopeV1,
    validated: &ValidatedYjsUpdate,
    update_seq: u64,
    event_ledger_event_id: &str,
) -> CrdtUpdateRecordV1 {
    let identity = knowledge_crdt_identity(
        &envelope.workspace_id,
        &envelope.document_id,
        &envelope.crdt_document_id,
        &envelope.document_schema_id,
        &validated.actor,
        &envelope.trace_id,
    );
    new_crdt_update_record(CrdtUpdateRecordInputV1 {
        identity: &identity,
        update_id: &envelope.update_id,
        update_seq,
        update_bytes: &validated.update_bytes,
        update_bytes_ref: &format!(
            "postgres://kernel_crdt_updates/{}/{}/update_bytes",
            envelope.crdt_document_id, envelope.update_id
        ),
        session_id: &envelope.session_id,
        trace_id: &envelope.trace_id,
        state_vector_before: &envelope.state_vector_before,
        state_vector_after: &envelope.state_vector_after,
        replay_metadata: CrdtReplayMetadataV1 {
            replay_order_key: format!(
                "{}/{}/{update_seq:020}",
                envelope.workspace_id, envelope.document_id
            ),
            dependency_update_ids: Vec::new(),
            encoding: YJS_UPDATE_ENCODING_V1.to_string(),
            schema_version: "kernel-crdt-update-v1".to_string(),
        },
        event_ledger_event_id,
    })
}

/// Reconstruct the wire envelope for a persisted update (pull path).
pub fn update_record_to_envelope(
    record: &CrdtUpdateRecordV1,
    update_bytes: &[u8],
    document_schema_id: &str,
) -> YjsUpdateEnvelopeV1 {
    YjsUpdateEnvelopeV1 {
        schema_id: YJS_UPDATE_ENVELOPE_SCHEMA_ID.to_string(),
        workspace_id: record.workspace_id.clone(),
        document_id: record.document_id.clone(),
        crdt_document_id: record.crdt_document_id.clone(),
        update_id: record.update_id.clone(),
        actor_id: record.actor_id.clone(),
        site_id: site_id_for_record(record),
        session_id: record.session_id.clone(),
        trace_id: record.trace_id.clone(),
        document_schema_id: document_schema_id.to_string(),
        update_b64: b64().encode(update_bytes),
        update_sha256: record.update_sha256.clone(),
        state_vector_before: record.state_vector_before.clone(),
        state_vector_after: record.state_vector_after.clone(),
        encoding: record.replay_metadata.encoding.clone(),
    }
}

fn site_id_for_record(record: &CrdtUpdateRecordV1) -> String {
    match KnowledgeActorIdV1::parse(&record.actor_id) {
        Ok(actor) => {
            derive_knowledge_site_id(&record.workspace_id, &record.crdt_document_id, &actor).site_id
        }
        // Legacy records (pre-MT-065 actor ids) cannot re-derive a site;
        // surface the actor id so attribution is still visible.
        Err(_) => format!("site-legacy-{}", record.actor_id),
    }
}

/// Typed reasons a push is refused. Always a typed result — a stale or
/// conflicting push NEVER silently overwrites the draft log.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum YjsPushDenialReasonV1 {
    /// Envelope failed structural validation.
    EnvelopeInvalid { messages: Vec<String> },
    /// The envelope's `state_vector_before` does not match the current head:
    /// the client must pull, merge locally (Yjs), and resubmit.
    StaleBase {
        head_update_seq: u64,
        head_state_vector: String,
        ordering: String,
    },
    /// Same `update_id` was stored before with different content.
    UpdateIdContentMismatch { update_id: String },
    /// Two writers raced for the same sequence slot; retry after refresh.
    SequenceSlotRace { attempted_seq: u64 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct YjsPushDenialV1 {
    pub schema_id: String,
    pub crdt_document_id: String,
    pub update_id: String,
    pub actor_id: String,
    pub reason: YjsPushDenialReasonV1,
}

/// Outcome of a push: stored, replayed (idempotent), or denied (typed).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum YjsPushOutcomeV1 {
    Stored {
        update_seq: u64,
        update_id: String,
        event_ledger_event_id: String,
        head_state_vector: String,
    },
    AlreadyStored {
        update_seq: u64,
        update_id: String,
        event_ledger_event_id: String,
        head_state_vector: String,
    },
    Denied {
        denial: YjsPushDenialV1,
    },
}

#[derive(Debug)]
pub enum KnowledgeCrdtFlowError {
    Storage(String),
    Event(String),
}

impl std::fmt::Display for KnowledgeCrdtFlowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Storage(message) => write!(f, "knowledge CRDT storage failure: {message}"),
            Self::Event(message) => write!(f, "knowledge CRDT event failure: {message}"),
        }
    }
}

impl std::error::Error for KnowledgeCrdtFlowError {}

/// Current head of a draft log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeDraftHeadV1 {
    pub head_update_seq: u64,
    pub head_state_vector: String,
}

/// Read the draft head from the persisted update log (seq 0 + empty vector
/// for a fresh document).
pub async fn read_draft_head(
    db: &(dyn Database + '_),
    workspace_id: &str,
    document_id: &str,
    crdt_document_id: &str,
) -> Result<KnowledgeDraftHeadV1, KnowledgeCrdtFlowError> {
    let records = db
        .list_kernel_crdt_updates(workspace_id, document_id, crdt_document_id)
        .await
        .map_err(|error| KnowledgeCrdtFlowError::Storage(error.to_string()))?;
    Ok(head_of(&records))
}

fn head_of(records: &[CrdtUpdateRecordV1]) -> KnowledgeDraftHeadV1 {
    records
        .iter()
        .max_by_key(|record| record.update_seq)
        .map(|record| KnowledgeDraftHeadV1 {
            head_update_seq: record.update_seq,
            head_state_vector: record.state_vector_after.clone(),
        })
        .unwrap_or(KnowledgeDraftHeadV1 {
            head_update_seq: 0,
            head_state_vector: KnowledgeStateVectorV1::new().encode(),
        })
}

/// Server-side ingest of one Yjs update envelope.
///
/// Linear draft-log rule: `state_vector_before` must equal the current head
/// state vector. A stale or concurrent base yields a typed
/// [`YjsPushDenialReasonV1::StaleBase`] — the Yjs client pulls, merges
/// locally, and resubmits a rebased envelope. Identical resubmission of an
/// already-stored update returns `AlreadyStored` (idempotent replay).
pub async fn push_yjs_update(
    db: &(dyn Database + '_),
    envelope: &YjsUpdateEnvelopeV1,
) -> Result<YjsPushOutcomeV1, KnowledgeCrdtFlowError> {
    let validated = match validate_yjs_update_envelope(envelope) {
        Ok(validated) => validated,
        Err(errors) => {
            return Ok(YjsPushOutcomeV1::Denied {
                denial: denial(
                    envelope,
                    YjsPushDenialReasonV1::EnvelopeInvalid {
                        messages: errors.iter().map(|error| error.to_string()).collect(),
                    },
                ),
            });
        }
    };
    let document_push_lock = push_lock_for_document(envelope).await;
    let _document_push_guard = document_push_lock.lock().await;

    let records = db
        .list_kernel_crdt_updates(
            &envelope.workspace_id,
            &envelope.document_id,
            &envelope.crdt_document_id,
        )
        .await
        .map_err(|error| KnowledgeCrdtFlowError::Storage(error.to_string()))?;

    // Idempotent replay of an already-stored update.
    if let Some(existing) = records
        .iter()
        .find(|record| record.update_id == envelope.update_id)
    {
        if existing.update_sha256 == envelope.update_sha256
            && existing.state_vector_before == envelope.state_vector_before
            && existing.state_vector_after == envelope.state_vector_after
        {
            let head = head_of(&records);
            return Ok(YjsPushOutcomeV1::AlreadyStored {
                update_seq: existing.update_seq,
                update_id: existing.update_id.clone(),
                event_ledger_event_id: existing.event_ledger_event_id.clone(),
                head_state_vector: head.head_state_vector,
            });
        }
        return Ok(YjsPushOutcomeV1::Denied {
            denial: denial(
                envelope,
                YjsPushDenialReasonV1::UpdateIdContentMismatch {
                    update_id: envelope.update_id.clone(),
                },
            ),
        });
    }

    let head = head_of(&records);
    let head_vector = KnowledgeStateVectorV1::parse(&head.head_state_vector)
        .map_err(|error| KnowledgeCrdtFlowError::Storage(error.to_string()))?;
    if validated.before != head_vector {
        let ordering = head_vector.compare(&validated.before);
        return Ok(YjsPushOutcomeV1::Denied {
            denial: denial(
                envelope,
                YjsPushDenialReasonV1::StaleBase {
                    head_update_seq: head.head_update_seq,
                    head_state_vector: head.head_state_vector,
                    ordering: format!("{ordering:?}"),
                },
            ),
        });
    }

    let attempted_seq = head.head_update_seq + 1;
    let event = NewKernelEvent::builder(
        format!("KTR-KNOWLEDGE-CRDT-{}", envelope.crdt_document_id),
        envelope.session_id.clone(),
        KernelEventType::KnowledgeCrdtUpdateRecorded,
        validated.actor.to_kernel_actor(),
    )
    .aggregate("knowledge_crdt_document", envelope.crdt_document_id.clone())
    .idempotency_key(format!(
        "knowledge-crdt-update:{}:{}",
        envelope.crdt_document_id, envelope.update_id
    ))
    .correlation_id(envelope.trace_id.clone())
    .source_component("knowledge_crdt_yjs_bridge")
    .payload(serde_json::json!({
        "update_id": envelope.update_id,
        "update_seq": attempted_seq,
        "actor_id": envelope.actor_id,
        "site_id": envelope.site_id,
        "update_sha256": envelope.update_sha256,
        "state_vector_before": envelope.state_vector_before,
        "state_vector_after": envelope.state_vector_after,
    }))
    .build()
    .map_err(|error| KnowledgeCrdtFlowError::Event(error.to_string()))?;
    let stored_event = db
        .append_kernel_event(event)
        .await
        .map_err(|error| KnowledgeCrdtFlowError::Event(error.to_string()))?;

    let record =
        envelope_to_update_record(envelope, &validated, attempted_seq, &stored_event.event_id);
    match db
        .append_kernel_crdt_update(record, validated.update_bytes.clone())
        .await
    {
        Ok(stored) => Ok(YjsPushOutcomeV1::Stored {
            update_seq: stored.update_seq,
            update_id: stored.update_id,
            event_ledger_event_id: stored.event_ledger_event_id,
            head_state_vector: envelope.state_vector_after.clone(),
        }),
        Err(error) => {
            let message = error.to_string();
            // Unique sequence-slot index (idx_kernel_crdt_updates_seq) turns
            // a concurrent race into a typed retryable denial.
            if message.contains("idx_kernel_crdt_updates_seq") {
                Ok(YjsPushOutcomeV1::Denied {
                    denial: denial(
                        envelope,
                        YjsPushDenialReasonV1::SequenceSlotRace { attempted_seq },
                    ),
                })
            } else {
                Err(KnowledgeCrdtFlowError::Storage(message))
            }
        }
    }
}

fn denial(envelope: &YjsUpdateEnvelopeV1, reason: YjsPushDenialReasonV1) -> YjsPushDenialV1 {
    YjsPushDenialV1 {
        schema_id: YJS_PUSH_DENIAL_SCHEMA_ID.to_string(),
        crdt_document_id: envelope.crdt_document_id.clone(),
        update_id: envelope.update_id.clone(),
        actor_id: envelope.actor_id.clone(),
        reason,
    }
}

/// Pull response: ordered envelopes after `since_update_seq` plus the head.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct YjsUpdatePullResponseV1 {
    pub workspace_id: String,
    pub document_id: String,
    pub crdt_document_id: String,
    pub since_update_seq: u64,
    pub updates: Vec<YjsUpdateEnvelopeV1>,
    pub head_update_seq: u64,
    pub head_state_vector: String,
}

/// Replay feed for reconnecting editors: every persisted update with
/// `update_seq > since_update_seq`, bytes re-encoded from PostgreSQL,
/// strictly ordered.
pub async fn pull_yjs_updates(
    db: &(dyn Database + '_),
    workspace_id: &str,
    document_id: &str,
    crdt_document_id: &str,
    since_update_seq: u64,
    document_schema_id: &str,
) -> Result<YjsUpdatePullResponseV1, KnowledgeCrdtFlowError> {
    let mut records = db
        .list_kernel_crdt_updates(workspace_id, document_id, crdt_document_id)
        .await
        .map_err(|error| KnowledgeCrdtFlowError::Storage(error.to_string()))?;
    records.sort_by_key(|record| record.update_seq);
    let head = head_of(&records);

    let mut updates = Vec::new();
    for record in records
        .iter()
        .filter(|record| record.update_seq > since_update_seq)
    {
        let bytes = db
            .read_kernel_crdt_update_bytes(&record.update_bytes_ref)
            .await
            .map_err(|error| KnowledgeCrdtFlowError::Storage(error.to_string()))?;
        updates.push(update_record_to_envelope(
            record,
            &bytes,
            document_schema_id,
        ));
    }

    Ok(YjsUpdatePullResponseV1 {
        workspace_id: workspace_id.to_string(),
        document_id: document_id.to_string(),
        crdt_document_id: crdt_document_id.to_string(),
        since_update_seq,
        updates,
        head_update_seq: head.head_update_seq,
        head_state_vector: head.head_state_vector,
    })
}

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
//! The backend decodes each Yjs v1 update at ingress to reject
//! hash-consistent-but-malformed bytes, then treats valid updates as opaque
//! durable payloads: PostgreSQL/EventLedger orders, attributes, and persists
//! them while document materialization remains a Yjs-compatible client concern.
//! This preserves the no-external-relay posture while preventing invalid binary
//! data from becoming authoritative replay state.

use base64::Engine;
use serde::{Deserialize, Serialize};
use yrs::{updates::decoder::Decode, ClientID, Update};

use crate::kernel::{KernelEventType, NewKernelEvent};
use crate::storage::{Database, KernelCrdtAtomicAppendOutcome, KernelCrdtAtomicAppendRequest};

use super::actor_site::{
    derive_knowledge_site_id, knowledge_crdt_identity, KnowledgeActorIdError, KnowledgeActorIdV1,
};
use super::persistence::{
    new_crdt_update_record, sha256_hex, CrdtReplayMetadataV1, CrdtUpdateRecordInputV1,
    CrdtUpdateRecordV1,
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
    /// Client-reported typed state vector before this update. The server
    /// compares it against the durable head and persists only its own
    /// canonical derivation.
    pub state_vector_before: String,
    /// Client-reported typed state vector after this update. It must equal the
    /// server-derived next vector; callers cannot advance arbitrary sites or
    /// clocks by supplying metadata here.
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
    UpdateBytesNotYjsV1 {
        message: String,
    },
    UpdateMissingExpectedClientId {
        expected_client_id: u32,
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
            Self::UpdateBytesNotYjsV1 { message } => {
                write!(f, "update bytes are not a decodable Yjs v1 update: {message}")
            }
            Self::UpdateMissingExpectedClientId { expected_client_id } => write!(
                f,
                "update has no insertion from the actor's deterministic Yjs client id {expected_client_id}"
            ),
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

    let decoded_update = match b64().decode(envelope.update_b64.as_bytes()) {
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
                    match Update::decode_v1(&bytes) {
                        Ok(update) => Some((bytes, update)),
                        Err(error) => {
                            errors.push(YjsEnvelopeValidationError::UpdateBytesNotYjsV1 {
                                message: error.to_string(),
                            });
                            None
                        }
                    }
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

    if let (Some((_, update)), Some(actor)) = (&decoded_update, &actor) {
        let expected =
            derive_knowledge_site_id(&envelope.workspace_id, &envelope.crdt_document_id, actor);
        let expected_client_id = ClientID::new(u64::from(expected.yjs_client_id));
        // A decoded Yjs payload is not enough for actor attribution: an actor
        // could otherwise submit another client's update while claiming its
        // own HSK site.  The bridge currently accepts only updates containing
        // at least one insertion from the deterministic client id bound to
        // the actor/site.  Delete-only payloads deliberately fail closed until
        // a separately durable operation-author attribution exists.
        if !update
            .state_vector_lower()
            .contains_client(&expected_client_id)
        {
            errors.push(YjsEnvelopeValidationError::UpdateMissingExpectedClientId {
                expected_client_id: expected.yjs_client_id,
            });
        }
    }

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

    match (decoded_update, actor, before, after) {
        (Some((update_bytes, _)), Some(actor), Some(before), Some(after)) if errors.is_empty() => {
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
/// Linear draft-log rule: the client-reported `state_vector_before` must equal
/// the current head and `state_vector_after` must equal the server-derived
/// next vector for the deterministic actor site. A stale or concurrent base yields a typed
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
    // This read only constructs the optimistic receipt candidate. The storage
    // operation below reacquires the durable head under a transaction-scoped
    // PostgreSQL advisory lock before it writes anything, so a different
    // process cannot create a receipt/row split or win the same sequence slot.
    let records = db
        .list_kernel_crdt_updates(
            &envelope.workspace_id,
            &envelope.document_id,
            &envelope.crdt_document_id,
        )
        .await
        .map_err(|error| KnowledgeCrdtFlowError::Storage(error.to_string()))?;

    let head = head_of(&records);
    let (server_before, server_after, attempted_seq) = match records
        .iter()
        .find(|record| record.update_id == envelope.update_id)
    {
        // Preserve exact idempotent replay after the durable head has moved.
        // The client must still present the original causal metadata; otherwise
        // the storage layer returns a content-mismatch denial rather than
        // accepting a forged retry.
        Some(existing) => {
            let before = KnowledgeStateVectorV1::parse(&existing.state_vector_before)
                .map_err(|error| KnowledgeCrdtFlowError::Storage(error.to_string()))?;
            let after = KnowledgeStateVectorV1::parse(&existing.state_vector_after)
                .map_err(|error| KnowledgeCrdtFlowError::Storage(error.to_string()))?;
            if validated.before != before || validated.after != after {
                return Ok(YjsPushOutcomeV1::Denied {
                    denial: denial(
                        envelope,
                        YjsPushDenialReasonV1::UpdateIdContentMismatch {
                            update_id: envelope.update_id.clone(),
                        },
                    ),
                });
            }
            (before, after, existing.update_seq)
        }
        None => {
            let before = KnowledgeStateVectorV1::parse(&head.head_state_vector)
                .map_err(|error| KnowledgeCrdtFlowError::Storage(error.to_string()))?;
            if validated.before != before {
                return Ok(YjsPushOutcomeV1::Denied {
                    denial: denial(
                        envelope,
                        YjsPushDenialReasonV1::StaleBase {
                            head_update_seq: head.head_update_seq,
                            head_state_vector: head.head_state_vector.clone(),
                            ordering: format!("{:?}", before.compare(&validated.before)),
                        },
                    ),
                });
            }
            let mut after = before.clone();
            after.increment(&envelope.site_id);
            if validated.after != after {
                return Ok(YjsPushOutcomeV1::Denied {
                    denial: denial(
                        envelope,
                        YjsPushDenialReasonV1::EnvelopeInvalid {
                            messages: vec![format!(
                                "state_vector_after must equal the server-derived next vector '{}'",
                                after.encode()
                            )],
                        },
                    ),
                });
            }
            let sequence = head.head_update_seq.checked_add(1).ok_or_else(|| {
                KnowledgeCrdtFlowError::Storage("CRDT update sequence overflow".into())
            })?;
            (before, after, sequence)
        }
    };

    // Keep client metadata outside the durable authority path.  Once the
    // server has verified it, construct the receipt/event from the derived
    // vectors so the stored causal chain cannot be advanced by a forged
    // envelope field.
    let mut canonical_envelope = envelope.clone();
    canonical_envelope.state_vector_before = server_before.encode();
    canonical_envelope.state_vector_after = server_after.encode();

    let event = NewKernelEvent::builder(
        format!("KTR-KNOWLEDGE-CRDT-{}", canonical_envelope.crdt_document_id),
        canonical_envelope.session_id.clone(),
        KernelEventType::KnowledgeCrdtUpdateRecorded,
        validated.actor.to_kernel_actor(),
    )
    .aggregate(
        "knowledge_crdt_document",
        canonical_envelope.crdt_document_id.clone(),
    )
    .idempotency_key(format!(
        "knowledge-crdt-update:{}:{}",
        canonical_envelope.crdt_document_id, canonical_envelope.update_id
    ))
    .correlation_id(canonical_envelope.trace_id.clone())
    .source_component("knowledge_crdt_yjs_bridge")
    .payload(serde_json::json!({
        "update_id": canonical_envelope.update_id,
        "update_seq": attempted_seq,
        "actor_id": canonical_envelope.actor_id,
        "site_id": canonical_envelope.site_id,
        "update_sha256": canonical_envelope.update_sha256,
        "state_vector_before": canonical_envelope.state_vector_before,
        "state_vector_after": canonical_envelope.state_vector_after,
    }))
    .build()
    .map_err(|error| KnowledgeCrdtFlowError::Event(error.to_string()))?;
    let provisional_record =
        envelope_to_update_record(&canonical_envelope, &validated, attempted_seq, "");
    match db
        .append_kernel_crdt_update_with_event_atomic(KernelCrdtAtomicAppendRequest {
            expected_head_update_seq: head.head_update_seq,
            expected_head_state_vector: head.head_state_vector.clone(),
            provisional_record,
            update_bytes: validated.update_bytes.clone(),
            event,
        })
        .await
    {
        Ok(KernelCrdtAtomicAppendOutcome::Stored(stored)) => Ok(YjsPushOutcomeV1::Stored {
            update_seq: stored.update_seq,
            update_id: stored.update_id,
            event_ledger_event_id: stored.event_ledger_event_id,
            head_state_vector: stored.state_vector_after,
        }),
        Ok(KernelCrdtAtomicAppendOutcome::AlreadyStored {
            record,
            head_state_vector,
            ..
        }) => Ok(YjsPushOutcomeV1::AlreadyStored {
            update_seq: record.update_seq,
            update_id: record.update_id,
            event_ledger_event_id: record.event_ledger_event_id,
            head_state_vector,
        }),
        Ok(KernelCrdtAtomicAppendOutcome::UpdateIdContentMismatch { update_id }) => {
            Ok(YjsPushOutcomeV1::Denied {
                denial: denial(
                    envelope,
                    YjsPushDenialReasonV1::UpdateIdContentMismatch { update_id },
                ),
            })
        }
        Ok(KernelCrdtAtomicAppendOutcome::StaleHead {
            head_update_seq,
            head_state_vector,
        }) => {
            let current = KnowledgeStateVectorV1::parse(&head_state_vector)
                .map_err(|error| KnowledgeCrdtFlowError::Storage(error.to_string()))?;
            Ok(YjsPushOutcomeV1::Denied {
                denial: denial(
                    envelope,
                    YjsPushDenialReasonV1::StaleBase {
                        head_update_seq,
                        head_state_vector,
                        ordering: format!("{:?}", current.compare(&validated.before)),
                    },
                ),
            })
        }
        Err(error) => Err(KnowledgeCrdtFlowError::Storage(error.to_string())),
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

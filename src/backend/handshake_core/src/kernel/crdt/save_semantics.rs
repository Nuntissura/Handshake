//! WP-KERNEL-009 MT-070 CRDTAndConcurrencyCore-070-ConcurrentEditorSaveSemantics.
//!
//! Master Spec anchor: 02-system-architecture.md section 2.3.13.11 — manual
//! edits MUST leave actor/state-vector/denial receipts; a stale or
//! conflicting save is NEVER silently overwritten.
//!
//! Deterministic semantics for simultaneous operator/model rich-document
//! draft saves over the linear update log:
//!
//! | base vs head            | decision        | effect                       |
//! |-------------------------|-----------------|------------------------------|
//! | base == head            | `FastForward`   | update appended              |
//! | head dominates base     | `StaleWrite`    | typed conflict + durable     |
//! |                         |                 | denial receipt + event       |
//! | base dominates head     | `AheadOfHead`   | denial: missing updates must |
//! |                         |                 | be pushed first              |
//! | concurrent              | `ConcurrentFork`| typed conflict + durable     |
//! |                         |                 | denial receipt + event       |
//!
//! The denied writer recovers by pulling (`pull_yjs_updates`), merging
//! locally (Yjs merge is the client-side CRDT job), and resubmitting a
//! rebased envelope whose `state_vector_before` equals the new head.

use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;

use crate::kernel::{KernelEventType, NewKernelEvent};
use crate::storage::knowledge_crdt::{
    insert_denial_receipt, new_denial_receipt_id, KnowledgeCrdtDenialReceiptRow,
    NewKnowledgeCrdtDenialReceipt,
};
use crate::storage::Database;

use super::actor_site::KnowledgeActorIdV1;
use super::agent_lease::{
    guard_lease_for_write, KnowledgeLeaseScopeKind, LeaseWriteDenialV1, LeaseWriteGuardOutcomeV1,
};
use super::state_vector::{KnowledgeStateVectorOrdering, KnowledgeStateVectorV1};
use super::yjs_bridge::{
    push_yjs_update, read_draft_head, KnowledgeCrdtFlowError, YjsPushDenialReasonV1,
    YjsPushOutcomeV1, YjsUpdateEnvelopeV1,
};

pub const KNOWLEDGE_SAVE_DECISION_SCHEMA_ID: &str = "hsk.kernel.knowledge_save_decision@1";

/// Pure decision over (head, incoming base). Deterministic: same vectors,
/// same verdict, regardless of which actor saves first on the wall clock.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum KnowledgeSaveDecisionV1 {
    /// Writer saw the latest head; append.
    FastForward,
    /// Server head moved past the writer's base: the save is stale.
    StaleWrite {
        head_state_vector: String,
        base_state_vector: String,
    },
    /// Writer claims updates the server has not stored yet; those must be
    /// pushed (offline replay path, MT-073) before this save.
    AheadOfHead {
        head_state_vector: String,
        base_state_vector: String,
    },
    /// True concurrent fork: both sides advanced independently.
    ConcurrentFork {
        head_state_vector: String,
        base_state_vector: String,
    },
}

/// Decide what a save attempt with `base` against `head` means.
pub fn decide_concurrent_save(
    head: &KnowledgeStateVectorV1,
    base: &KnowledgeStateVectorV1,
) -> KnowledgeSaveDecisionV1 {
    match base.compare(head) {
        KnowledgeStateVectorOrdering::Equal => KnowledgeSaveDecisionV1::FastForward,
        KnowledgeStateVectorOrdering::DominatedBy => KnowledgeSaveDecisionV1::StaleWrite {
            head_state_vector: head.encode(),
            base_state_vector: base.encode(),
        },
        KnowledgeStateVectorOrdering::Dominates => KnowledgeSaveDecisionV1::AheadOfHead {
            head_state_vector: head.encode(),
            base_state_vector: base.encode(),
        },
        KnowledgeStateVectorOrdering::Concurrent => KnowledgeSaveDecisionV1::ConcurrentFork {
            head_state_vector: head.encode(),
            base_state_vector: base.encode(),
        },
    }
}

impl KnowledgeSaveDecisionV1 {
    /// Receipt kind for the durable denial row; None for FastForward.
    pub fn denial_receipt_kind(&self) -> Option<&'static str> {
        match self {
            Self::FastForward => None,
            Self::StaleWrite { .. } => Some("stale_draft_save"),
            Self::AheadOfHead { .. } => Some("ahead_of_head_save"),
            Self::ConcurrentFork { .. } => Some("concurrent_draft_fork"),
        }
    }
}

/// Outcome of a draft save attempt. A non-accepted save always carries the
/// typed conflict AND the durable receipt id — never a silent overwrite,
/// never a silent drop.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum KnowledgeDraftSaveOutcomeV1 {
    Accepted {
        update_seq: u64,
        update_id: String,
        event_ledger_event_id: String,
        head_state_vector: String,
    },
    AlreadyApplied {
        update_seq: u64,
        update_id: String,
        event_ledger_event_id: String,
        head_state_vector: String,
    },
    Conflict {
        decision: KnowledgeSaveDecisionV1,
        denial_receipt_id: String,
        conflict_event_id: String,
        head_update_seq: u64,
        head_state_vector: String,
    },
    /// Structural rejection (invalid envelope / mismatched update_id reuse /
    /// sequence race). No durable receipt: the writer must fix the request.
    Rejected { reason: YjsPushDenialReasonV1 },
    /// Authority-hardening #4: the save was attempted under a lease that
    /// failed the server-side write guard (expired / foreign / released /
    /// wrong-scope). Durable receipt + KNOWLEDGE_CRDT_LEASE_WRITE_DENIED
    /// event; the draft is NOT written.
    LeaseDenied { denial: LeaseWriteDenialV1 },
}

/// Save a rich-document draft update with deterministic concurrent-save
/// semantics. Wraps [`push_yjs_update`]; on a stale/concurrent base the
/// denial becomes durable: one `knowledge_crdt_denial_receipts` row plus one
/// `KNOWLEDGE_CRDT_CONFLICT_DETECTED` EventLedger event, both idempotent on
/// (document, update_id).
pub async fn save_rich_document_draft(
    db: &(dyn Database + '_),
    pool: &PgPool,
    envelope: &YjsUpdateEnvelopeV1,
) -> Result<KnowledgeDraftSaveOutcomeV1, KnowledgeCrdtFlowError> {
    match push_yjs_update(db, envelope).await? {
        YjsPushOutcomeV1::Stored {
            update_seq,
            update_id,
            event_ledger_event_id,
            head_state_vector,
        } => Ok(KnowledgeDraftSaveOutcomeV1::Accepted {
            update_seq,
            update_id,
            event_ledger_event_id,
            head_state_vector,
        }),
        YjsPushOutcomeV1::AlreadyStored {
            update_seq,
            update_id,
            event_ledger_event_id,
            head_state_vector,
        } => Ok(KnowledgeDraftSaveOutcomeV1::AlreadyApplied {
            update_seq,
            update_id,
            event_ledger_event_id,
            head_state_vector,
        }),
        YjsPushOutcomeV1::Denied { denial } => match &denial.reason {
            YjsPushDenialReasonV1::StaleBase {
                head_update_seq,
                head_state_vector,
                ..
            } => {
                let head = KnowledgeStateVectorV1::parse(head_state_vector)
                    .map_err(|error| KnowledgeCrdtFlowError::Storage(error.to_string()))?;
                let base = KnowledgeStateVectorV1::parse(&envelope.state_vector_before)
                    .map_err(|error| KnowledgeCrdtFlowError::Storage(error.to_string()))?;
                let decision = decide_concurrent_save(&head, &base);
                let receipt_kind = match decision.denial_receipt_kind() {
                    Some(kind) => kind,
                    None => {
                        // StaleBase with equal vectors cannot happen (push
                        // would have accepted); treat as storage drift.
                        return Err(KnowledgeCrdtFlowError::Storage(
                            "stale-base denial with fast-forward decision".to_string(),
                        ));
                    }
                };
                let (receipt, event_id) = record_conflict_receipt(
                    db,
                    pool,
                    envelope,
                    &decision,
                    receipt_kind,
                    *head_update_seq,
                    head_state_vector,
                )
                .await?;
                Ok(KnowledgeDraftSaveOutcomeV1::Conflict {
                    decision,
                    denial_receipt_id: receipt.receipt_id,
                    conflict_event_id: event_id,
                    head_update_seq: *head_update_seq,
                    head_state_vector: head_state_vector.clone(),
                })
            }
            _ => Ok(KnowledgeDraftSaveOutcomeV1::Rejected {
                reason: denial.reason,
            }),
        },
    }
}

/// Authority-hardening #4: save a rich-document draft under a presented lane
/// lease, enforced as a server-side chokepoint. The lease MUST be live,
/// owned by the saving actor, and cover the document scope; otherwise the
/// save is refused with a durable [`KnowledgeDraftSaveOutcomeV1::LeaseDenied`]
/// receipt and NO draft is written. When the guard passes, the save proceeds
/// exactly as [`save_rich_document_draft`]. Callers that hold no lease (e.g.
/// operator direct saves where a lease is not required) use the unguarded
/// entrypoint.
pub async fn save_rich_document_draft_under_lease(
    db: &(dyn Database + '_),
    pool: &PgPool,
    envelope: &YjsUpdateEnvelopeV1,
    lease_id: &str,
) -> Result<KnowledgeDraftSaveOutcomeV1, KnowledgeCrdtFlowError> {
    let actor = KnowledgeActorIdV1::parse(&envelope.actor_id)
        .map_err(|error| KnowledgeCrdtFlowError::Storage(error.to_string()))?;
    match guard_lease_for_write(
        db,
        pool,
        lease_id,
        &actor,
        &envelope.session_id,
        &envelope.trace_id,
        &envelope.workspace_id,
        KnowledgeLeaseScopeKind::Document,
        &envelope.crdt_document_id,
    )
    .await
    .map_err(|error| KnowledgeCrdtFlowError::Storage(error.to_string()))?
    {
        LeaseWriteGuardOutcomeV1::Allowed(_) => save_rich_document_draft(db, pool, envelope).await,
        LeaseWriteGuardOutcomeV1::Denied(denial) => {
            Ok(KnowledgeDraftSaveOutcomeV1::LeaseDenied { denial: *denial })
        }
    }
}

async fn record_conflict_receipt(
    db: &(dyn Database + '_),
    pool: &PgPool,
    envelope: &YjsUpdateEnvelopeV1,
    decision: &KnowledgeSaveDecisionV1,
    receipt_kind: &'static str,
    head_update_seq: u64,
    head_state_vector: &str,
) -> Result<(KnowledgeCrdtDenialReceiptRow, String), KnowledgeCrdtFlowError> {
    let actor = KnowledgeActorIdV1::parse(&envelope.actor_id)
        .map_err(|error| KnowledgeCrdtFlowError::Storage(error.to_string()))?;
    let idempotency_key = format!(
        "knowledge-crdt-conflict:{}:{}",
        envelope.crdt_document_id, envelope.update_id
    );
    let event = NewKernelEvent::builder(
        format!("KTR-KNOWLEDGE-CRDT-{}", envelope.crdt_document_id),
        envelope.session_id.clone(),
        KernelEventType::KnowledgeCrdtConflictDetected,
        actor.to_kernel_actor(),
    )
    .aggregate("knowledge_crdt_document", envelope.crdt_document_id.clone())
    .idempotency_key(idempotency_key.clone())
    .correlation_id(envelope.trace_id.clone())
    .source_component("knowledge_crdt_save_semantics")
    .payload(json!({
        "schema_id": KNOWLEDGE_SAVE_DECISION_SCHEMA_ID,
        "decision": decision,
        "denied_update_id": envelope.update_id,
        "denied_actor_id": envelope.actor_id,
        "denied_site_id": envelope.site_id,
        "head_update_seq": head_update_seq,
        "head_state_vector": head_state_vector,
        "base_state_vector": envelope.state_vector_before,
        "attempted_state_vector": envelope.state_vector_after,
    }))
    .build()
    .map_err(|error| KnowledgeCrdtFlowError::Event(error.to_string()))?;
    let stored_event = db
        .append_kernel_event(event)
        .await
        .map_err(|error| KnowledgeCrdtFlowError::Event(error.to_string()))?;

    let receipt = insert_denial_receipt(
        pool,
        NewKnowledgeCrdtDenialReceipt {
            receipt_id: new_denial_receipt_id(),
            receipt_kind: receipt_kind.to_string(),
            workspace_id: envelope.workspace_id.clone(),
            document_id: Some(envelope.document_id.clone()),
            crdt_document_id: Some(envelope.crdt_document_id.clone()),
            scope_ref: format!("crdt_document:{}", envelope.crdt_document_id),
            actor_id: envelope.actor_id.clone(),
            actor_kind: actor.kind().as_str().to_string(),
            session_id: envelope.session_id.clone(),
            correlation_id: envelope.trace_id.clone(),
            denial_payload: json!({
                "schema_id": KNOWLEDGE_SAVE_DECISION_SCHEMA_ID,
                "decision": decision,
                "denied_update_id": envelope.update_id,
                "denied_update_sha256": envelope.update_sha256,
                "head_update_seq": head_update_seq,
                "attempted_state_vector": envelope.state_vector_after,
            }),
            event_ledger_event_id: stored_event.event_id.clone(),
            idempotency_key,
        },
    )
    .await
    .map_err(|error| KnowledgeCrdtFlowError::Storage(error.to_string()))?;

    Ok((receipt, stored_event.event_id))
}

/// Convenience for tests and the API: current head as typed vector.
pub async fn read_head_vector(
    db: &(dyn Database + '_),
    workspace_id: &str,
    document_id: &str,
    crdt_document_id: &str,
) -> Result<(u64, KnowledgeStateVectorV1), KnowledgeCrdtFlowError> {
    let head = read_draft_head(db, workspace_id, document_id, crdt_document_id).await?;
    let vector = KnowledgeStateVectorV1::parse(&head.head_state_vector)
        .map_err(|error| KnowledgeCrdtFlowError::Storage(error.to_string()))?;
    Ok((head.head_update_seq, vector))
}

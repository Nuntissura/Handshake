//! WP-KERNEL-009 MT-071 CRDTAndConcurrencyCore-071-ConcurrentIndexRunSemantics.
//!
//! Master Spec anchor: 02-system-architecture.md section 2.3.13.11 (index
//! receipts + lease discipline) and the MT-041 AgentLaneLease seed.
//!
//! Concurrency rule: at most ONE active index run per source root. The guard
//! is the MT-076 lane lease with `scope_kind = source_root` — the partial
//! unique index on the lease table is the server-side mutual exclusion, so
//! two indexers racing for the same root resolve in PostgreSQL, not in
//! process memory. Different source roots claim independently (safe parallel
//! partitioning); a second claimant on the same root receives a typed
//! rejection naming the holder, its expiry, and whether takeover is already
//! possible, plus a durable `index_run_slot_rejected` denial receipt.
//!
//! Relationship to `knowledge_index_runs` (migration 0133): that table is
//! owned by the PostgresEventLedgerCore lane (MT-052) and is NOT modified
//! here. When a run row exists, callers thread its id through
//! `index_run_ref` so lease receipts and run rows correlate; the guard
//! itself only depends on committed WP-009 surfaces (leases 0151, denial
//! receipts 0150).

use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;

use crate::kernel::{KernelEventType, NewKernelEvent};
use crate::storage::Database;
use crate::storage::knowledge_crdt::{
    AgentLaneLeaseRow, NewKnowledgeCrdtDenialReceipt, insert_denial_receipt, new_denial_receipt_id,
};

use super::actor_site::KnowledgeActorIdV1;
use super::agent_lease::{
    KnowledgeLeaseScopeKind, LeaseClaimOutcomeV1, LeaseClaimRequestV1, LeaseFlowError, claim_lease,
    release_lease,
};

pub const INDEX_RUN_SLOT_REJECTION_SCHEMA_ID: &str =
    "hsk.kernel.knowledge_index_run_slot_rejection@1";

/// Request to start indexing a source root.
#[derive(Debug, Clone)]
pub struct IndexRunSlotRequestV1 {
    pub lane_id: String,
    pub actor: KnowledgeActorIdV1,
    pub session_id: String,
    pub correlation_id: String,
    pub workspace_id: String,
    /// `knowledge_source_roots.root_id` (committed migration 0131).
    pub source_root_id: String,
    /// Optional `knowledge_index_runs.run_id` once that row exists (0133,
    /// other lane); threaded into receipts for correlation only.
    pub index_run_ref: Option<String>,
    pub ttl_seconds: i64,
}

/// Typed rejection for a second concurrent claim on the same source root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexRunSlotRejectionV1 {
    pub schema_id: String,
    pub source_root_id: String,
    pub holder_lease_id: String,
    pub holder_actor_id: String,
    pub holder_session_id: String,
    pub holder_expires_at_utc: String,
    /// True when the holder lease is already expired on the database clock:
    /// the claimant may immediately use lease takeover (MT-076) instead of
    /// waiting.
    pub holder_expired_takeover_possible: bool,
    pub denial_receipt_id: String,
    pub event_ledger_event_id: String,
}

/// Outcome of claiming the per-root index slot.
#[derive(Debug, Clone, PartialEq)]
pub enum IndexRunSlotOutcomeV1 {
    /// Slot claimed; the returned lease guards the run. Release it with
    /// [`release_index_run_slot`] when the run finishes or fails.
    Claimed(Box<AgentLaneLeaseRow>),
    Rejected(Box<IndexRunSlotRejectionV1>),
}

/// Claim the single active index-run slot for a source root.
pub async fn claim_index_run_slot(
    db: &(dyn Database + '_),
    pool: &PgPool,
    request: IndexRunSlotRequestV1,
) -> Result<IndexRunSlotOutcomeV1, LeaseFlowError> {
    let claim = claim_lease(
        db,
        pool,
        LeaseClaimRequestV1 {
            lane_id: request.lane_id.clone(),
            actor: request.actor.clone(),
            session_id: request.session_id.clone(),
            correlation_id: request.correlation_id.clone(),
            scope_kind: KnowledgeLeaseScopeKind::SourceRoot,
            scope_id: request.source_root_id.clone(),
            ttl_seconds: request.ttl_seconds,
        },
    )
    .await?;

    match claim {
        LeaseClaimOutcomeV1::Claimed(lease) => Ok(IndexRunSlotOutcomeV1::Claimed(lease)),
        LeaseClaimOutcomeV1::ScopeHeld {
            holder,
            holder_expired,
        } => {
            // Durable typed rejection: event + denial receipt.
            let receipt_id = new_denial_receipt_id();
            let event = NewKernelEvent::builder(
                format!("KTR-KNOWLEDGE-INDEX-RUN-{}", request.source_root_id),
                request.session_id.clone(),
                KernelEventType::KnowledgeCrdtLeaseWriteDenied,
                request.actor.to_kernel_actor(),
            )
            .aggregate("knowledge_index_run_slot", request.source_root_id.clone())
            .idempotency_key(format!("knowledge-index-run-denial:{receipt_id}"))
            .correlation_id(request.correlation_id.clone())
            .source_component("knowledge_crdt_index_run_guard")
            .payload(json!({
                "schema_id": INDEX_RUN_SLOT_REJECTION_SCHEMA_ID,
                "source_root_id": request.source_root_id,
                "index_run_ref": request.index_run_ref,
                "holder_lease_id": holder.lease_id,
                "holder_actor_id": holder.actor_id,
                "holder_expired_takeover_possible": holder_expired,
                "claimant_actor_id": request.actor.canonical(),
            }))
            .build()
            .map_err(|error| LeaseFlowError::Event(error.to_string()))?;
            let stored_event = db
                .append_kernel_event(event)
                .await
                .map_err(|error| LeaseFlowError::Event(error.to_string()))?;

            let receipt = insert_denial_receipt(
                pool,
                NewKnowledgeCrdtDenialReceipt {
                    receipt_id: receipt_id.clone(),
                    receipt_kind: "index_run_slot_rejected".to_string(),
                    workspace_id: request.workspace_id.clone(),
                    document_id: None,
                    crdt_document_id: None,
                    scope_ref: format!("lease_scope:source_root:{}", request.source_root_id),
                    actor_id: request.actor.canonical(),
                    actor_kind: request.actor.kind().as_str().to_string(),
                    session_id: request.session_id.clone(),
                    correlation_id: request.correlation_id.clone(),
                    denial_payload: json!({
                        "schema_id": INDEX_RUN_SLOT_REJECTION_SCHEMA_ID,
                        "source_root_id": request.source_root_id,
                        "index_run_ref": request.index_run_ref,
                        "holder_lease_id": holder.lease_id,
                        "holder_actor_id": holder.actor_id,
                        "holder_expires_at_utc": holder.expires_at_utc.to_rfc3339(),
                        "holder_expired_takeover_possible": holder_expired,
                    }),
                    event_ledger_event_id: stored_event.event_id.clone(),
                    idempotency_key: format!("knowledge-index-run-denial:{receipt_id}"),
                },
            )
            .await?;

            Ok(IndexRunSlotOutcomeV1::Rejected(Box::new(
                IndexRunSlotRejectionV1 {
                    schema_id: INDEX_RUN_SLOT_REJECTION_SCHEMA_ID.to_string(),
                    source_root_id: request.source_root_id,
                    holder_lease_id: holder.lease_id.clone(),
                    holder_actor_id: holder.actor_id.clone(),
                    holder_session_id: holder.session_id.clone(),
                    holder_expires_at_utc: holder.expires_at_utc.to_rfc3339(),
                    holder_expired_takeover_possible: holder_expired,
                    denial_receipt_id: receipt.receipt_id,
                    event_ledger_event_id: stored_event.event_id,
                },
            )))
        }
    }
}

/// Release the index-run slot when the run completes or aborts.
pub async fn release_index_run_slot(
    db: &(dyn Database + '_),
    pool: &PgPool,
    lease_id: &str,
    actor: &KnowledgeActorIdV1,
) -> Result<Option<AgentLaneLeaseRow>, LeaseFlowError> {
    release_lease(db, pool, lease_id, actor).await
}

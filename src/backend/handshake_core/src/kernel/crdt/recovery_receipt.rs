//! WP-KERNEL-009 MT-079 CRDTAndConcurrencyCore-079-CrdtRecoveryReceiptFormat.
//!
//! Contract source: MT-041 `swarm_lease_checkpoint_contract_seed`
//! (SwarmCheckpoint semantics). A checkpoint freezes everything another
//! model needs to resume after compaction, crash, or session loss:
//! identity (session/actor/lane/lease), the scope, a TYPED resume pointer,
//! and a hashed payload. Recovery emits a durable recovery receipt linking
//! the NEW session to the recovered checkpoint and the full lease lineage
//! (takeover chain) — reconstructable from PostgreSQL alone, with no chat
//! history dependency.

use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;

use crate::kernel::{KernelEventType, NewKernelEvent};
use crate::storage::knowledge_crdt::{
    self, NewRecoveryReceipt, NewSwarmCheckpoint, RecoveryReceiptRow, SwarmCheckpointRow,
};
use crate::storage::Database;

use super::actor_site::KnowledgeActorIdV1;
use super::agent_lease::{new_ulid, LeaseFlowError};
use super::persistence::sha256_hex;

pub const SWARM_CHECKPOINT_SCHEMA_ID: &str = "hsk.kernel.knowledge_swarm_checkpoint@1";
pub const CRDT_RECOVERY_RECEIPT_SCHEMA_ID: &str = "hsk.kernel.knowledge_crdt_recovery_receipt@1";

/// Typed resume pointer (seed: "MT id, claim id, document revision, or
/// index-run position").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "pointer", rename_all = "snake_case")]
pub enum SwarmResumePointerV1 {
    MicroTask {
        mt_id: String,
    },
    Claim {
        claim_id: String,
    },
    DocumentRevision {
        crdt_document_id: String,
        update_seq: u64,
        state_vector: String,
    },
    IndexRunPosition {
        index_run_ref: String,
        position: String,
    },
}

/// Request to write a swarm checkpoint.
#[derive(Debug, Clone)]
pub struct SwarmCheckpointRequestV1 {
    pub session_id: String,
    pub actor: KnowledgeActorIdV1,
    pub lane_id: String,
    /// The active lane lease the work runs under (FK-verified).
    pub lease_id: String,
    pub scope_ref: String,
    pub resume_pointer: SwarmResumePointerV1,
    /// Work-in-flight payload (frozen as-is; hashed below).
    pub checkpoint_payload: serde_json::Value,
}

/// Write a durable checkpoint (KNOWLEDGE_CRDT_CHECKPOINT_RECORDED + row).
/// `payload_sha256` is computed over the canonical JSON payload bytes.
pub async fn write_swarm_checkpoint(
    db: &(dyn Database + '_),
    pool: &PgPool,
    request: SwarmCheckpointRequestV1,
) -> Result<SwarmCheckpointRow, LeaseFlowError> {
    let checkpoint_id = new_ulid();
    let payload_bytes = serde_json::to_vec(&request.checkpoint_payload)
        .map_err(|error| LeaseFlowError::Event(error.to_string()))?;
    let payload_sha256 = sha256_hex(&payload_bytes);
    let resume_pointer = serde_json::to_value(&request.resume_pointer)
        .map_err(|error| LeaseFlowError::Event(error.to_string()))?;

    let event = NewKernelEvent::builder(
        format!("KTR-KNOWLEDGE-CHECKPOINT-{}", request.lane_id),
        request.session_id.clone(),
        KernelEventType::KnowledgeCrdtCheckpointRecorded,
        request.actor.to_kernel_actor(),
    )
    .aggregate("knowledge_swarm_checkpoint", checkpoint_id.clone())
    .idempotency_key(format!("knowledge-checkpoint:{checkpoint_id}:recorded"))
    .correlation_id(request.scope_ref.clone())
    .source_component("knowledge_crdt_recovery_receipt")
    .payload(json!({
        "schema_id": SWARM_CHECKPOINT_SCHEMA_ID,
        "checkpoint_id": checkpoint_id,
        "session_id": request.session_id,
        "actor_id": request.actor.canonical(),
        "lane_id": request.lane_id,
        "lease_id": request.lease_id,
        "scope_ref": request.scope_ref,
        "resume_pointer": resume_pointer,
        "payload_sha256": payload_sha256,
    }))
    .build()
    .map_err(|error| LeaseFlowError::Event(error.to_string()))?;
    let stored_event = db
        .append_kernel_event(event)
        .await
        .map_err(|error| LeaseFlowError::Event(error.to_string()))?;

    let row = knowledge_crdt::insert_swarm_checkpoint(
        pool,
        NewSwarmCheckpoint {
            checkpoint_id,
            session_id: request.session_id,
            actor_id: request.actor.canonical(),
            lane_id: request.lane_id,
            lease_id: request.lease_id,
            scope_ref: request.scope_ref,
            resume_pointer,
            checkpoint_payload: request.checkpoint_payload,
            payload_sha256,
            recorded_event_id: stored_event.event_id,
        },
    )
    .await?;
    Ok(row)
}

/// Typed recovery failure reasons.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum RecoveryFailureV1 {
    CheckpointNotFound {
        checkpoint_id: String,
    },
    PayloadHashMismatch {
        checkpoint_id: String,
        expected: String,
        found: String,
    },
    NewLeaseNotFound {
        lease_id: String,
    },
}

/// A verified recovery: the durable receipt plus the typed resume pointer
/// and re-verified checkpoint payload, all reconstructed from PostgreSQL.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CrdtRecoveryV1 {
    pub schema_id: String,
    pub receipt: RecoveryReceiptRow,
    pub checkpoint: SwarmCheckpointRow,
    pub resume_pointer: SwarmResumePointerV1,
    /// Ordered lease ids, newest (the recovering session's lease) first,
    /// back through every takeover to the original claim.
    pub lease_lineage_ids: Vec<String>,
}

/// Recover a new session from a checkpoint:
///   1. load + hash-verify the checkpoint payload;
///   2. resolve the new lease and walk its takeover lineage (MT-076);
///   3. append KNOWLEDGE_CRDT_RECOVERY_RECEIPT_RECORDED;
///   4. insert the durable recovery receipt row.
pub async fn recover_from_checkpoint(
    db: &(dyn Database + '_),
    pool: &PgPool,
    checkpoint_id: &str,
    new_session_id: &str,
    new_actor: &KnowledgeActorIdV1,
    new_lease_id: &str,
) -> Result<Result<CrdtRecoveryV1, RecoveryFailureV1>, LeaseFlowError> {
    let checkpoint = match knowledge_crdt::get_swarm_checkpoint(pool, checkpoint_id).await? {
        Some(row) => row,
        None => {
            return Ok(Err(RecoveryFailureV1::CheckpointNotFound {
                checkpoint_id: checkpoint_id.to_string(),
            }));
        }
    };

    // Re-verify the frozen payload against its recorded hash.
    let payload_bytes = serde_json::to_vec(&checkpoint.checkpoint_payload)
        .map_err(|error| LeaseFlowError::Event(error.to_string()))?;
    let found_hash = sha256_hex(&payload_bytes);
    if found_hash != checkpoint.payload_sha256 {
        return Ok(Err(RecoveryFailureV1::PayloadHashMismatch {
            checkpoint_id: checkpoint_id.to_string(),
            expected: checkpoint.payload_sha256.clone(),
            found: found_hash,
        }));
    }

    if knowledge_crdt::get_lease(pool, new_lease_id)
        .await?
        .is_none()
    {
        return Ok(Err(RecoveryFailureV1::NewLeaseNotFound {
            lease_id: new_lease_id.to_string(),
        }));
    }
    let lineage = knowledge_crdt::lease_lineage(pool, new_lease_id).await?;
    let lease_lineage_ids: Vec<String> =
        lineage.iter().map(|lease| lease.lease_id.clone()).collect();

    let resume_pointer: SwarmResumePointerV1 =
        serde_json::from_value(checkpoint.resume_pointer.clone())
            .map_err(|error| LeaseFlowError::Event(error.to_string()))?;

    let receipt_id = new_ulid();
    let event = NewKernelEvent::builder(
        format!("KTR-KNOWLEDGE-CHECKPOINT-{}", checkpoint.lane_id),
        new_session_id.to_string(),
        KernelEventType::KnowledgeCrdtRecoveryReceiptRecorded,
        new_actor.to_kernel_actor(),
    )
    .aggregate(
        "knowledge_swarm_checkpoint",
        checkpoint.checkpoint_id.clone(),
    )
    .idempotency_key(format!("knowledge-recovery:{receipt_id}:recorded"))
    .correlation_id(checkpoint.scope_ref.clone())
    .source_component("knowledge_crdt_recovery_receipt")
    .payload(json!({
        "schema_id": CRDT_RECOVERY_RECEIPT_SCHEMA_ID,
        "receipt_id": receipt_id,
        "checkpoint_id": checkpoint.checkpoint_id,
        "prior_session_id": checkpoint.session_id,
        "new_session_id": new_session_id,
        "new_actor_id": new_actor.canonical(),
        "new_lease_id": new_lease_id,
        "lease_lineage": lease_lineage_ids,
        "resume_pointer": checkpoint.resume_pointer,
    }))
    .build()
    .map_err(|error| LeaseFlowError::Event(error.to_string()))?;
    let stored_event = db
        .append_kernel_event(event)
        .await
        .map_err(|error| LeaseFlowError::Event(error.to_string()))?;

    let receipt = knowledge_crdt::insert_recovery_receipt(
        pool,
        NewRecoveryReceipt {
            receipt_id,
            checkpoint_id: checkpoint.checkpoint_id.clone(),
            prior_session_id: checkpoint.session_id.clone(),
            new_session_id: new_session_id.to_string(),
            new_actor_id: new_actor.canonical(),
            new_lease_id: new_lease_id.to_string(),
            lease_lineage: json!(lease_lineage_ids),
            resume_pointer: checkpoint.resume_pointer.clone(),
            recorded_event_id: stored_event.event_id,
        },
    )
    .await?;

    Ok(Ok(CrdtRecoveryV1 {
        schema_id: CRDT_RECOVERY_RECEIPT_SCHEMA_ID.to_string(),
        receipt,
        checkpoint,
        resume_pointer,
        lease_lineage_ids,
    }))
}

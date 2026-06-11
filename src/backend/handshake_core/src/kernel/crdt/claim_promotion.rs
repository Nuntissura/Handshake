//! WP-KERNEL-009 MT-069 CRDTAndConcurrencyCore-069-ClaimPromotionBridge.
//!
//! Master Spec anchor: 02-system-architecture.md section 2.3.13.11 —
//! "authority changes MUST flow through WriteBoxV1 and EventLedger
//! promotion" and "A direct write to ProjectKnowledgeIndex authority records
//! outside the catalog/write-box/promotion path is invalid."
//!
//! Bridge: an APPROVED graph/claim proposal (MT-068) becomes a stable
//! EventLedger-backed knowledge fact:
//!   1. PROMOTION_REQUESTED + PROMOTION_ACCEPTED appended atomically
//!      (existing kernel promotion event taxonomy, causation-linked);
//!   2. one `knowledge_crdt_promoted_facts` authority row (frozen payload,
//!      span refs, proposer + gate actors, both event ids);
//!   3. proposal marked 'promoted'.
//!
//! Idempotent end to end: the event pair dedups on idempotency keys and the
//! fact row dedups on proposal_id, so a crashed/retried promotion converges
//! on the same fact. Invalid promotions (unknown proposal, not approved)
//! leave a durable `graph_promotion_denied` receipt + PROMOTION_REJECTED.
//!
//! Relationship to `knowledge_claims` (migration 0137, committed by MT-056):
//! promoted facts land in `knowledge_crdt_promoted_facts` (registered
//! authority_class 'authority', record family KnowledgeClaim) rather than
//! double-writing `knowledge_claims` rows, because claim creation requires
//! commit-time `KSP-*` span evidence (FK + trigger) while proposals may cite
//! `pending:<source>:<range>` markers for spans not extracted yet. The fact
//! row freezes the full payload + span refs, so the knowledge-claims lane
//! can map promoted `add_claim` facts into `knowledge_claims` without data
//! loss once their spans exist (see 0153 header).

use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;

use crate::kernel::{KernelEventType, NewKernelEvent};
use crate::storage::knowledge_crdt::{
    self, insert_denial_receipt, new_denial_receipt_id, NewKnowledgeCrdtDenialReceipt,
    NewPromotedFact, PromotedFactRow,
};
use crate::storage::Database;

use super::actor_site::{KnowledgeActorIdV1, KnowledgeActorKind};
use super::agent_lease::{new_ulid, LeaseFlowError};

pub const GRAPH_PROMOTION_DENIAL_SCHEMA_ID: &str = "hsk.kernel.knowledge_graph_promotion_denial@1";

/// Typed reasons a promotion is denied (each leaves a durable receipt).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum GraphPromotionDenialReasonV1 {
    UnknownProposal { proposal_id: String },
    NotApproved { current_state: String },
    GateActorNotAllowed { gate_actor: String },
}

/// Durable promotion denial: receipt row + PROMOTION_REJECTED event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphPromotionDenialV1 {
    pub schema_id: String,
    pub proposal_id: String,
    pub reason: GraphPromotionDenialReasonV1,
    pub denial_receipt_id: String,
    pub event_ledger_event_id: String,
}

/// Outcome of a promotion attempt.
#[derive(Debug, Clone, PartialEq)]
pub enum GraphPromotionOutcomeV1 {
    Promoted(Box<PromotedFactRow>),
    /// The proposal was already promoted; the existing fact is returned
    /// unchanged (idempotent replay).
    AlreadyPromoted(Box<PromotedFactRow>),
    Denied(Box<GraphPromotionDenialV1>),
}

fn promotion_idempotency(proposal_id: &str, leg: &str) -> String {
    format!("knowledge-graph-promotion:{proposal_id}:{leg}")
}

/// Promote an approved graph mutation proposal into an EventLedger-backed
/// knowledge fact. Gate actors are operators, validators, or system lanes.
pub async fn promote_graph_proposal(
    db: &(dyn Database + '_),
    pool: &PgPool,
    proposal_id: &str,
    gate_actor: &KnowledgeActorIdV1,
    gate_session_id: &str,
    correlation_id: &str,
) -> Result<GraphPromotionOutcomeV1, LeaseFlowError> {
    // Idempotent replay first: an existing fact wins regardless of state.
    if let Some(existing) = knowledge_crdt::get_promoted_fact_by_proposal(pool, proposal_id).await?
    {
        return Ok(GraphPromotionOutcomeV1::AlreadyPromoted(Box::new(existing)));
    }

    if !matches!(
        gate_actor.kind(),
        KnowledgeActorKind::Operator | KnowledgeActorKind::Validator | KnowledgeActorKind::System
    ) {
        return deny(
            db,
            pool,
            proposal_id,
            None,
            gate_actor,
            gate_session_id,
            correlation_id,
            GraphPromotionDenialReasonV1::GateActorNotAllowed {
                gate_actor: gate_actor.canonical(),
            },
        )
        .await;
    }

    let proposal = knowledge_crdt::get_graph_proposal(pool, proposal_id).await?;
    let proposal = match proposal {
        Some(row) => row,
        None => {
            return deny(
                db,
                pool,
                proposal_id,
                None,
                gate_actor,
                gate_session_id,
                correlation_id,
                GraphPromotionDenialReasonV1::UnknownProposal {
                    proposal_id: proposal_id.to_string(),
                },
            )
            .await;
        }
    };
    if proposal.review_state != "approved" {
        return deny(
            db,
            pool,
            proposal_id,
            Some(&proposal.workspace_id),
            gate_actor,
            gate_session_id,
            correlation_id,
            GraphPromotionDenialReasonV1::NotApproved {
                current_state: proposal.review_state.clone(),
            },
        )
        .await;
    }

    // EventLedger promotion pair (atomic, causation-linked, idempotent).
    let requested = promotion_event(
        &proposal.workspace_id,
        proposal_id,
        gate_actor,
        gate_session_id,
        correlation_id,
        KernelEventType::PromotionRequested,
        "requested",
        json!({
            "proposal_id": proposal_id,
            "mutation_kind": proposal.mutation_kind,
            "proposed_by": proposal.actor_id,
            "gate_actor": gate_actor.canonical(),
        }),
    )?;
    let accepted = promotion_event(
        &proposal.workspace_id,
        proposal_id,
        gate_actor,
        gate_session_id,
        correlation_id,
        KernelEventType::PromotionAccepted,
        "accepted",
        json!({
            "proposal_id": proposal_id,
            "mutation_kind": proposal.mutation_kind,
            "source_span_refs": proposal.source_span_refs,
            "confidence": proposal.confidence,
        }),
    )?;
    let events = db
        .append_kernel_event_pair_atomic_with_causation(requested, accepted)
        .await
        .map_err(|error| LeaseFlowError::Event(error.to_string()))?;
    let (requested_id, accepted_id) = match events.as_slice() {
        [first, second] => (first.event_id.clone(), second.event_id.clone()),
        _ => {
            return Err(LeaseFlowError::Event(
                "promotion event pair append returned unexpected event count".to_string(),
            ));
        }
    };

    let fact = knowledge_crdt::insert_promoted_fact_idempotent(
        pool,
        NewPromotedFact {
            fact_id: new_ulid(),
            proposal_id: proposal_id.to_string(),
            workspace_id: proposal.workspace_id.clone(),
            mutation_kind: proposal.mutation_kind.clone(),
            fact_payload: proposal.mutation_payload.clone(),
            source_span_refs: proposal.source_span_refs.clone(),
            confidence: proposal.confidence,
            proposed_by: proposal.actor_id.clone(),
            promoted_by: gate_actor.canonical(),
            promotion_requested_event_id: requested_id,
            promotion_accepted_event_id: accepted_id,
        },
    )
    .await?;

    knowledge_crdt::mark_graph_proposal_promoted(pool, proposal_id).await?;

    Ok(GraphPromotionOutcomeV1::Promoted(Box::new(fact)))
}

fn promotion_event(
    workspace_id: &str,
    proposal_id: &str,
    gate_actor: &KnowledgeActorIdV1,
    gate_session_id: &str,
    correlation_id: &str,
    event_type: KernelEventType,
    leg: &str,
    payload: serde_json::Value,
) -> Result<NewKernelEvent, LeaseFlowError> {
    NewKernelEvent::builder(
        format!("KTR-KNOWLEDGE-GRAPH-{workspace_id}"),
        gate_session_id.to_string(),
        event_type,
        gate_actor.to_kernel_actor(),
    )
    .aggregate("knowledge_graph_promotion", proposal_id.to_string())
    .idempotency_key(promotion_idempotency(proposal_id, leg))
    .correlation_id(correlation_id.to_string())
    .source_component("knowledge_crdt_claim_promotion")
    .payload(payload)
    .build()
    .map_err(|error| LeaseFlowError::Event(error.to_string()))
}

#[allow(clippy::too_many_arguments)]
async fn deny(
    db: &(dyn Database + '_),
    pool: &PgPool,
    proposal_id: &str,
    workspace_id: Option<&str>,
    gate_actor: &KnowledgeActorIdV1,
    gate_session_id: &str,
    correlation_id: &str,
    reason: GraphPromotionDenialReasonV1,
) -> Result<GraphPromotionOutcomeV1, LeaseFlowError> {
    let receipt_id = new_denial_receipt_id();
    let event = NewKernelEvent::builder(
        format!(
            "KTR-KNOWLEDGE-GRAPH-{}",
            workspace_id.unwrap_or("unknown-workspace")
        ),
        gate_session_id.to_string(),
        KernelEventType::PromotionRejected,
        gate_actor.to_kernel_actor(),
    )
    .aggregate("knowledge_graph_promotion", proposal_id.to_string())
    .idempotency_key(format!("knowledge-graph-promotion-denial:{receipt_id}"))
    .correlation_id(correlation_id.to_string())
    .source_component("knowledge_crdt_claim_promotion")
    .payload(json!({
        "schema_id": GRAPH_PROMOTION_DENIAL_SCHEMA_ID,
        "proposal_id": proposal_id,
        "reason": reason,
        "gate_actor": gate_actor.canonical(),
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
            receipt_kind: "graph_promotion_denied".to_string(),
            workspace_id: workspace_id.unwrap_or("unknown-workspace").to_string(),
            document_id: None,
            crdt_document_id: None,
            scope_ref: format!("proposal:{proposal_id}"),
            actor_id: gate_actor.canonical(),
            actor_kind: gate_actor.kind().as_str().to_string(),
            session_id: gate_session_id.to_string(),
            correlation_id: correlation_id.to_string(),
            denial_payload: json!({
                "schema_id": GRAPH_PROMOTION_DENIAL_SCHEMA_ID,
                "proposal_id": proposal_id,
                "reason": reason,
            }),
            event_ledger_event_id: stored_event.event_id.clone(),
            idempotency_key: format!("knowledge-graph-promotion-denial:{receipt_id}"),
        },
    )
    .await?;

    Ok(GraphPromotionOutcomeV1::Denied(Box::new(
        GraphPromotionDenialV1 {
            schema_id: GRAPH_PROMOTION_DENIAL_SCHEMA_ID.to_string(),
            proposal_id: proposal_id.to_string(),
            reason,
            denial_receipt_id: receipt.receipt_id,
            event_ledger_event_id: stored_event.event_id,
        },
    )))
}

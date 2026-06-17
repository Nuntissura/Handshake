//! WP-KERNEL-009 MT-074 CRDTAndConcurrencyCore-074-AiEditProposalReviewFlow.
//!
//! Master Spec anchor: 02-system-architecture.md section 2.3.13.11 — "AI
//! edit proposals ... MUST leave actor, source span, state-vector,
//! validation, denial, or promotion receipts."
//!
//! Flow: a MODEL actor proposes a rich-document edit as a typed diff pinned
//! to a document revision (base update_seq + typed state vector), with
//! session provenance and source span citations. Review state machine:
//! proposed -> approved | rejected (operator/validator only; models cannot
//! self-approve). Approved proposals promote through the EventLedger
//! promotion pair; the approved diff is then eligible to be applied as a
//! normal CRDT update by the model actor (push path, MT-067), citing the
//! proposal id. Rejection records the durable decision on the row + event;
//! promoting a rejected/pending proposal leaves a durable
//! `ai_edit_promotion_denied` receipt + PROMOTION_REJECTED event.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::PgPool;

use crate::kernel::{KernelEventType, NewKernelEvent};
use crate::storage::Database;
use crate::storage::knowledge_crdt::{
    self, AiEditProposalRow, NewAiEditProposal, NewKnowledgeCrdtDenialReceipt,
    insert_denial_receipt, new_denial_receipt_id,
};

use super::actor_site::{KnowledgeActorIdV1, KnowledgeActorKind};
use super::agent_lease::{
    KnowledgeLeaseScopeKind, LeaseFlowError, LeaseWriteDenialV1, LeaseWriteGuardOutcomeV1,
    guard_lease_for_write, new_ulid,
};
use super::persistence::sha256_hex;
use super::state_vector::KnowledgeStateVectorV1;

pub const AI_EDIT_PROPOSAL_SCHEMA_ID: &str = "hsk.kernel.knowledge_ai_edit_proposal@1";
pub const AI_EDIT_PROMOTION_DENIAL_SCHEMA_ID: &str =
    "hsk.kernel.knowledge_ai_edit_promotion_denial@1";

/// Request to record an AI edit proposal.
#[derive(Debug, Clone)]
pub struct AiEditProposalRequestV1 {
    pub workspace_id: String,
    pub document_id: String,
    pub crdt_document_id: String,
    /// Revision the diff applies to (head at proposal time).
    pub base_update_seq: u64,
    pub base_state_vector: String,
    /// Typed JSON diff payload (ProseMirror steps / node replacement).
    pub proposed_diff: Value,
    /// Source span citations backing the proposal (>= 1, spec MUST).
    pub source_span_citations: Vec<String>,
    pub actor: KnowledgeActorIdV1,
    pub session_id: String,
    pub correlation_id: String,
    pub lease_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiEditProposalValidationError {
    EmptyField { field: &'static str },
    ActorNotModel { actor_id: String },
    ModelActorWithoutLease { actor_id: String },
    NoCitations,
    BaseVectorInvalid { message: String },
    DiffNotObject,
}

impl std::fmt::Display for AiEditProposalValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyField { field } => write!(f, "proposal field {field} must not be empty"),
            Self::ActorNotModel { actor_id } => write!(
                f,
                "AI edit proposals come from model actors only; '{actor_id}' is not local_model/cloud_model"
            ),
            Self::ModelActorWithoutLease { actor_id } => write!(
                f,
                "model actor '{actor_id}' must hold a lane lease to propose edits (MT-041 seed)"
            ),
            Self::NoCitations => write!(
                f,
                "AI edit proposals require at least one source span citation (spec 2.3.13.11)"
            ),
            Self::BaseVectorInvalid { message } => {
                write!(f, "base state vector invalid: {message}")
            }
            Self::DiffNotObject => write!(f, "proposed diff must be a JSON object"),
        }
    }
}

impl std::error::Error for AiEditProposalValidationError {}

pub fn validate_ai_edit_proposal_request(
    request: &AiEditProposalRequestV1,
) -> Result<(), Vec<AiEditProposalValidationError>> {
    let mut errors = Vec::new();
    for (field, value) in [
        ("workspace_id", &request.workspace_id),
        ("document_id", &request.document_id),
        ("crdt_document_id", &request.crdt_document_id),
        ("session_id", &request.session_id),
        ("correlation_id", &request.correlation_id),
    ] {
        if value.trim().is_empty() {
            errors.push(AiEditProposalValidationError::EmptyField { field });
        }
    }
    if !request.actor.kind().is_model() {
        errors.push(AiEditProposalValidationError::ActorNotModel {
            actor_id: request.actor.canonical(),
        });
    } else if request.lease_id.is_none() {
        errors.push(AiEditProposalValidationError::ModelActorWithoutLease {
            actor_id: request.actor.canonical(),
        });
    }
    if request.source_span_citations.is_empty()
        || request
            .source_span_citations
            .iter()
            .any(|span| span.trim().is_empty())
    {
        errors.push(AiEditProposalValidationError::NoCitations);
    }
    if let Err(error) = KnowledgeStateVectorV1::parse(&request.base_state_vector) {
        errors.push(AiEditProposalValidationError::BaseVectorInvalid {
            message: error.to_string(),
        });
    }
    if !request.proposed_diff.is_object() {
        errors.push(AiEditProposalValidationError::DiffNotObject);
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Outcome of recording an AI edit proposal. A lease denial is a durable
/// runtime denial (receipt + event), distinct from a validation error and
/// from success.
#[derive(Debug, Clone, PartialEq)]
pub enum RecordAiEditProposalOutcomeV1 {
    Recorded(Box<AiEditProposalRow>),
    Invalid(Vec<AiEditProposalValidationError>),
    /// Authority-hardening #4: the presented lease failed the server-side
    /// write guard (expired / foreign / released / wrong-scope). Durable.
    LeaseDenied(Box<LeaseWriteDenialV1>),
}

/// Record an AI edit proposal (AI_EDIT_PROPOSAL_RECORDED + draft row).
///
/// Authority-hardening #4: AI edit proposals always carry a model actor +
/// lease (validation above). The presented lease is routed through
/// [`guard_lease_for_write`] (document scope) BEFORE the draft is written, so
/// an expired/foreign/wrong-scope lease is denied with a durable receipt and
/// NO draft row — presence-only checking let those through.
pub async fn record_ai_edit_proposal(
    db: &(dyn Database + '_),
    pool: &PgPool,
    request: AiEditProposalRequestV1,
) -> Result<RecordAiEditProposalOutcomeV1, LeaseFlowError> {
    if let Err(errors) = validate_ai_edit_proposal_request(&request) {
        return Ok(RecordAiEditProposalOutcomeV1::Invalid(errors));
    }

    // Lease chokepoint over the workspace scope (model lane leases are
    // workspace-scoped per the MT-041 seed; the validation above already
    // guarantees a model actor presented a lease id).
    if let Some(lease_id) = &request.lease_id {
        match guard_lease_for_write(
            db,
            pool,
            lease_id,
            &request.actor,
            &request.session_id,
            &request.correlation_id,
            &request.workspace_id,
            KnowledgeLeaseScopeKind::Workspace,
            &request.workspace_id,
        )
        .await?
        {
            LeaseWriteGuardOutcomeV1::Allowed(_) => {}
            LeaseWriteGuardOutcomeV1::Denied(denial) => {
                return Ok(RecordAiEditProposalOutcomeV1::LeaseDenied(denial));
            }
        }
    }

    let proposal_id = new_ulid();
    let diff_bytes = serde_json::to_vec(&request.proposed_diff)
        .map_err(|error| LeaseFlowError::Event(error.to_string()))?;
    let diff_sha256 = sha256_hex(&diff_bytes);

    let event = NewKernelEvent::builder(
        format!("KTR-KNOWLEDGE-AI-EDIT-{}", request.crdt_document_id),
        request.session_id.clone(),
        KernelEventType::AiEditProposalRecorded,
        request.actor.to_kernel_actor(),
    )
    .aggregate("knowledge_ai_edit_proposal", proposal_id.clone())
    .idempotency_key(format!("knowledge-ai-edit:{proposal_id}:recorded"))
    .correlation_id(request.correlation_id.clone())
    .source_component("knowledge_crdt_ai_edit_proposal")
    .payload(json!({
        "schema_id": AI_EDIT_PROPOSAL_SCHEMA_ID,
        "proposal_id": proposal_id,
        "crdt_document_id": request.crdt_document_id,
        "base_update_seq": request.base_update_seq,
        "base_state_vector": request.base_state_vector,
        "diff_sha256": diff_sha256,
        "actor_id": request.actor.canonical(),
        "source_span_citations": request.source_span_citations,
        "lease_id": request.lease_id,
    }))
    .build()
    .map_err(|error| LeaseFlowError::Event(error.to_string()))?;
    let stored_event = db
        .append_kernel_event(event)
        .await
        .map_err(|error| LeaseFlowError::Event(error.to_string()))?;

    let row = knowledge_crdt::insert_ai_edit_proposal(
        pool,
        NewAiEditProposal {
            proposal_id,
            workspace_id: request.workspace_id,
            document_id: request.document_id,
            crdt_document_id: request.crdt_document_id,
            base_update_seq: i64::try_from(request.base_update_seq)
                .map_err(|_| LeaseFlowError::Event("base_update_seq exceeds i64".to_string()))?,
            base_state_vector: request.base_state_vector,
            proposed_diff: request.proposed_diff,
            diff_sha256,
            source_span_citations: request.source_span_citations,
            actor_id: request.actor.canonical(),
            actor_kind: request.actor.kind().as_str().to_string(),
            session_id: request.session_id,
            correlation_id: request.correlation_id,
            lease_id: request.lease_id,
            recorded_event_id: stored_event.event_id,
        },
    )
    .await?;
    Ok(RecordAiEditProposalOutcomeV1::Recorded(Box::new(row)))
}

/// Typed review errors (mirrors graph proposal semantics).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiEditDecisionError {
    ProposalNotFound { proposal_id: String },
    NotInProposedState { current_state: String },
    ReviewerNotAllowed { reviewer: String },
}

impl std::fmt::Display for AiEditDecisionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProposalNotFound { proposal_id } => {
                write!(f, "AI edit proposal '{proposal_id}' not found")
            }
            Self::NotInProposedState { current_state } => write!(
                f,
                "proposal is '{current_state}', only 'proposed' rows can be decided"
            ),
            Self::ReviewerNotAllowed { reviewer } => write!(
                f,
                "reviewer '{reviewer}' must be an operator or validator actor"
            ),
        }
    }
}

impl std::error::Error for AiEditDecisionError {}

/// Approve or reject (AI_EDIT_PROPOSAL_DECIDED; reviewer = operator/validator).
pub async fn decide_ai_edit_proposal(
    db: &(dyn Database + '_),
    pool: &PgPool,
    proposal_id: &str,
    approve: bool,
    reviewer: &KnowledgeActorIdV1,
    reviewer_session_id: &str,
    decision_reason: &str,
) -> Result<Result<AiEditProposalRow, AiEditDecisionError>, LeaseFlowError> {
    if !matches!(
        reviewer.kind(),
        KnowledgeActorKind::Operator | KnowledgeActorKind::Validator
    ) {
        return Ok(Err(AiEditDecisionError::ReviewerNotAllowed {
            reviewer: reviewer.canonical(),
        }));
    }
    let existing = knowledge_crdt::get_ai_edit_proposal(pool, proposal_id).await?;
    let existing = match existing {
        Some(row) => row,
        None => {
            return Ok(Err(AiEditDecisionError::ProposalNotFound {
                proposal_id: proposal_id.to_string(),
            }));
        }
    };
    if existing.review_state != "proposed" {
        return Ok(Err(AiEditDecisionError::NotInProposedState {
            current_state: existing.review_state,
        }));
    }

    let new_state = if approve { "approved" } else { "rejected" };
    let event = NewKernelEvent::builder(
        format!("KTR-KNOWLEDGE-AI-EDIT-{}", existing.crdt_document_id),
        reviewer_session_id.to_string(),
        KernelEventType::AiEditProposalDecided,
        reviewer.to_kernel_actor(),
    )
    .aggregate("knowledge_ai_edit_proposal", proposal_id.to_string())
    .idempotency_key(format!(
        "knowledge-ai-edit:{proposal_id}:decided:{new_state}"
    ))
    .correlation_id(existing.correlation_id.clone())
    .source_component("knowledge_crdt_ai_edit_proposal")
    .payload(json!({
        "schema_id": AI_EDIT_PROPOSAL_SCHEMA_ID,
        "proposal_id": proposal_id,
        "decision": new_state,
        "decided_by": reviewer.canonical(),
        "decision_reason": decision_reason,
        "diff_sha256": existing.diff_sha256,
    }))
    .build()
    .map_err(|error| LeaseFlowError::Event(error.to_string()))?;
    let stored_event = db
        .append_kernel_event(event)
        .await
        .map_err(|error| LeaseFlowError::Event(error.to_string()))?;

    let decided = knowledge_crdt::decide_ai_edit_proposal(
        pool,
        proposal_id,
        new_state,
        &reviewer.canonical(),
        decision_reason,
        &stored_event.event_id,
    )
    .await?;
    match decided {
        Some(row) => Ok(Ok(row)),
        None => {
            let current = knowledge_crdt::get_ai_edit_proposal(pool, proposal_id)
                .await?
                .map(|row| row.review_state)
                .unwrap_or_else(|| "missing".to_string());
            Ok(Err(AiEditDecisionError::NotInProposedState {
                current_state: current,
            }))
        }
    }
}

/// Typed promotion denial reasons.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum AiEditPromotionDenialReasonV1 {
    UnknownProposal { proposal_id: String },
    NotApproved { current_state: String },
    GateActorNotAllowed { gate_actor: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiEditPromotionDenialV1 {
    pub schema_id: String,
    pub proposal_id: String,
    pub reason: AiEditPromotionDenialReasonV1,
    pub denial_receipt_id: String,
    pub event_ledger_event_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AiEditPromotionOutcomeV1 {
    Promoted(Box<AiEditProposalRow>),
    AlreadyPromoted(Box<AiEditProposalRow>),
    Denied(Box<AiEditPromotionDenialV1>),
}

/// Promote an APPROVED AI edit proposal (EventLedger promotion pair, row
/// stamped 'promoted'). Promotion of unknown/unapproved proposals leaves a
/// durable denial receipt + PROMOTION_REJECTED event. Idempotent.
pub async fn promote_ai_edit_proposal(
    db: &(dyn Database + '_),
    pool: &PgPool,
    proposal_id: &str,
    gate_actor: &KnowledgeActorIdV1,
    gate_session_id: &str,
    correlation_id: &str,
) -> Result<AiEditPromotionOutcomeV1, LeaseFlowError> {
    if !matches!(
        gate_actor.kind(),
        KnowledgeActorKind::Operator | KnowledgeActorKind::Validator | KnowledgeActorKind::System
    ) {
        return deny_promotion(
            db,
            pool,
            proposal_id,
            None,
            gate_actor,
            gate_session_id,
            correlation_id,
            AiEditPromotionDenialReasonV1::GateActorNotAllowed {
                gate_actor: gate_actor.canonical(),
            },
        )
        .await;
    }
    let proposal = knowledge_crdt::get_ai_edit_proposal(pool, proposal_id).await?;
    let proposal = match proposal {
        Some(row) => row,
        None => {
            return deny_promotion(
                db,
                pool,
                proposal_id,
                None,
                gate_actor,
                gate_session_id,
                correlation_id,
                AiEditPromotionDenialReasonV1::UnknownProposal {
                    proposal_id: proposal_id.to_string(),
                },
            )
            .await;
        }
    };
    if proposal.review_state == "promoted" {
        return Ok(AiEditPromotionOutcomeV1::AlreadyPromoted(Box::new(
            proposal,
        )));
    }
    if proposal.review_state != "approved" {
        return deny_promotion(
            db,
            pool,
            proposal_id,
            Some(&proposal),
            gate_actor,
            gate_session_id,
            correlation_id,
            AiEditPromotionDenialReasonV1::NotApproved {
                current_state: proposal.review_state.clone(),
            },
        )
        .await;
    }

    let requested = NewKernelEvent::builder(
        format!("KTR-KNOWLEDGE-AI-EDIT-{}", proposal.crdt_document_id),
        gate_session_id.to_string(),
        KernelEventType::PromotionRequested,
        gate_actor.to_kernel_actor(),
    )
    .aggregate("knowledge_ai_edit_promotion", proposal_id.to_string())
    .idempotency_key(format!(
        "knowledge-ai-edit-promotion:{proposal_id}:requested"
    ))
    .correlation_id(correlation_id.to_string())
    .source_component("knowledge_crdt_ai_edit_proposal")
    .payload(json!({
        "proposal_id": proposal_id,
        "diff_sha256": proposal.diff_sha256,
        "base_update_seq": proposal.base_update_seq,
        "base_state_vector": proposal.base_state_vector,
    }))
    .build()
    .map_err(|error| LeaseFlowError::Event(error.to_string()))?;
    let accepted = NewKernelEvent::builder(
        format!("KTR-KNOWLEDGE-AI-EDIT-{}", proposal.crdt_document_id),
        gate_session_id.to_string(),
        KernelEventType::PromotionAccepted,
        gate_actor.to_kernel_actor(),
    )
    .aggregate("knowledge_ai_edit_promotion", proposal_id.to_string())
    .idempotency_key(format!(
        "knowledge-ai-edit-promotion:{proposal_id}:accepted"
    ))
    .correlation_id(correlation_id.to_string())
    .source_component("knowledge_crdt_ai_edit_proposal")
    .payload(json!({
        "proposal_id": proposal_id,
        "decided_by": proposal.decided_by,
        "source_span_citations": proposal.source_span_citations,
    }))
    .build()
    .map_err(|error| LeaseFlowError::Event(error.to_string()))?;
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

    let promoted = knowledge_crdt::mark_ai_edit_proposal_promoted(
        pool,
        proposal_id,
        &requested_id,
        &accepted_id,
    )
    .await?;
    match promoted {
        Some(row) => Ok(AiEditPromotionOutcomeV1::Promoted(Box::new(row))),
        // Raced with a concurrent promotion that won after our state read.
        None => {
            let current = knowledge_crdt::get_ai_edit_proposal(pool, proposal_id)
                .await?
                .ok_or(LeaseFlowError::Event(
                    "proposal vanished during promotion".to_string(),
                ))?;
            Ok(AiEditPromotionOutcomeV1::AlreadyPromoted(Box::new(current)))
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn deny_promotion(
    db: &(dyn Database + '_),
    pool: &PgPool,
    proposal_id: &str,
    proposal: Option<&AiEditProposalRow>,
    gate_actor: &KnowledgeActorIdV1,
    gate_session_id: &str,
    correlation_id: &str,
    reason: AiEditPromotionDenialReasonV1,
) -> Result<AiEditPromotionOutcomeV1, LeaseFlowError> {
    let receipt_id = new_denial_receipt_id();
    let crdt_document_id = proposal.map(|row| row.crdt_document_id.clone());
    let event = NewKernelEvent::builder(
        format!(
            "KTR-KNOWLEDGE-AI-EDIT-{}",
            crdt_document_id.as_deref().unwrap_or("unknown-document")
        ),
        gate_session_id.to_string(),
        KernelEventType::PromotionRejected,
        gate_actor.to_kernel_actor(),
    )
    .aggregate("knowledge_ai_edit_promotion", proposal_id.to_string())
    .idempotency_key(format!("knowledge-ai-edit-promotion-denial:{receipt_id}"))
    .correlation_id(correlation_id.to_string())
    .source_component("knowledge_crdt_ai_edit_proposal")
    .payload(json!({
        "schema_id": AI_EDIT_PROMOTION_DENIAL_SCHEMA_ID,
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
            receipt_kind: "ai_edit_promotion_denied".to_string(),
            workspace_id: proposal
                .map(|row| row.workspace_id.clone())
                .unwrap_or_else(|| "unknown-workspace".to_string()),
            document_id: proposal.map(|row| row.document_id.clone()),
            crdt_document_id,
            scope_ref: format!("proposal:{proposal_id}"),
            actor_id: gate_actor.canonical(),
            actor_kind: gate_actor.kind().as_str().to_string(),
            session_id: gate_session_id.to_string(),
            correlation_id: correlation_id.to_string(),
            denial_payload: json!({
                "schema_id": AI_EDIT_PROMOTION_DENIAL_SCHEMA_ID,
                "proposal_id": proposal_id,
                "reason": reason,
            }),
            event_ledger_event_id: stored_event.event_id.clone(),
            idempotency_key: format!("knowledge-ai-edit-promotion-denial:{receipt_id}"),
        },
    )
    .await?;

    Ok(AiEditPromotionOutcomeV1::Denied(Box::new(
        AiEditPromotionDenialV1 {
            schema_id: AI_EDIT_PROMOTION_DENIAL_SCHEMA_ID.to_string(),
            proposal_id: proposal_id.to_string(),
            reason,
            denial_receipt_id: receipt.receipt_id,
            event_ledger_event_id: stored_event.event_id,
        },
    )))
}

pub const AI_EDIT_APPLIED_BINDING_SCHEMA_ID: &str = "hsk.kernel.knowledge_ai_edit_applied@1";
pub const AI_EDIT_APPLIED_MISMATCH_SCHEMA_ID: &str =
    "hsk.kernel.knowledge_ai_edit_applied_mismatch@1";
/// MT-074 V1 FAIL remediation: emitted when an applied-binding cites an
/// `applied_update_id` with NO corresponding `kernel_crdt_updates` row, or with
/// a row whose persisted `update_sha256` disagrees with the content presented
/// for binding. A matching real update row is required before authority can be
/// stamped, even when the diff hash matches.
pub const AI_EDIT_APPLIED_UPDATE_MISSING_SCHEMA_ID: &str =
    "hsk.kernel.knowledge_ai_edit_applied_update_missing@1";

/// Outcome of binding an applied update to an approved AI edit proposal.
#[derive(Debug, Clone, PartialEq)]
pub enum AiEditApplyOutcomeV1 {
    /// The applied update hashed to the approved diff; the proposal row now
    /// carries the binding (`applied_update_id` + `applied_update_sha256`).
    Bound(Box<AiEditProposalRow>),
    /// The proposal does not exist.
    UnknownProposal { proposal_id: String },
    /// The proposal is not approved/promoted (only those can be applied).
    NotApplicable { current_state: String },
    /// Authority-hardening #5: the applied update content did NOT hash to the
    /// approved `diff_sha256`. Durable `ai_edit_applied_mismatch` receipt; the
    /// binding is refused so authority cannot claim an unrelated edit as the
    /// approved one.
    HashMismatch {
        expected_diff_sha256: String,
        applied_content_sha256: String,
        denial_receipt_id: String,
        event_ledger_event_id: String,
    },
    /// MT-074 V1 FAIL remediation: there is NO `kernel_crdt_updates` row for the
    /// proposal's (workspace, document, crdt_document, update_id), OR the row
    /// that exists carries a different `update_sha256` than the content
    /// presented for binding. The binding is refused even when the diff hash
    /// matches — an approved edit may only bind to a real, content-consistent
    /// document update. Durable `ai_edit_applied_update_missing` receipt.
    UpdateRowMissing {
        applied_update_id: String,
        /// `Some(stored)` when a row exists but its persisted hash disagrees
        /// with the presented content; `None` when no row exists at all.
        stored_update_sha256: Option<String>,
        applied_content_sha256: String,
        denial_receipt_id: String,
        event_ledger_event_id: String,
    },
}

/// Authority-hardening #5: record that `applied_update_id` (a
/// `kernel_crdt_updates` row already pushed to the document) is the
/// application of an APPROVED AI edit proposal, binding it to the approved
/// `diff_sha256`. `applied_diff` is the diff payload that was actually
/// applied; its canonical hash MUST equal the proposal's approved
/// `diff_sha256` or the binding is refused with a durable receipt. This is
/// what closes the approved-vs-applied gap: nothing else lets an approved
/// proposal's authority trail point at the update that realized it, and a
/// non-matching update can never be recorded as that application.
pub async fn apply_approved_ai_edit(
    db: &(dyn Database + '_),
    pool: &PgPool,
    proposal_id: &str,
    applied_update_id: &str,
    applied_diff: &Value,
    actor: &KnowledgeActorIdV1,
    session_id: &str,
    correlation_id: &str,
) -> Result<AiEditApplyOutcomeV1, LeaseFlowError> {
    let proposal = match knowledge_crdt::get_ai_edit_proposal(pool, proposal_id).await? {
        Some(row) => row,
        None => {
            return Ok(AiEditApplyOutcomeV1::UnknownProposal {
                proposal_id: proposal_id.to_string(),
            });
        }
    };
    if !matches!(proposal.review_state.as_str(), "approved" | "promoted") {
        return Ok(AiEditApplyOutcomeV1::NotApplicable {
            current_state: proposal.review_state,
        });
    }

    let applied_bytes = serde_json::to_vec(applied_diff)
        .map_err(|error| LeaseFlowError::Event(error.to_string()))?;
    let applied_content_sha256 = sha256_hex(&applied_bytes);

    // MT-074 V1 FAIL remediation: before binding, require that a REAL
    // `kernel_crdt_updates` row exists for the proposal's
    // (workspace, document, crdt_document, applied_update_id) AND that the
    // persisted row hash equals the content presented for binding. The diff
    // hash matching the approved diff is NOT sufficient: without a real update
    // row there is no document edit to anchor the approved proposal to, and an
    // approved proposal's authority trail must never point at an update id that
    // was never pushed (or at one whose stored content differs). Refuse the
    // binding with a durable `ai_edit_applied_update_missing` denial.
    let stored_update_sha256 = knowledge_crdt::find_applied_crdt_update_sha256(
        pool,
        &proposal.workspace_id,
        &proposal.document_id,
        &proposal.crdt_document_id,
        applied_update_id,
    )
    .await?;
    if stored_update_sha256.as_deref() != Some(applied_content_sha256.as_str()) {
        return deny_applied_update_missing(
            db,
            pool,
            &proposal,
            applied_update_id,
            stored_update_sha256,
            &applied_content_sha256,
            actor,
            session_id,
            correlation_id,
        )
        .await;
    }

    // The binder only stamps the row when the hash equals the approved diff
    // hash (and the 0192 CHECK is the schema backstop).
    if let Some(bound) = knowledge_crdt::bind_applied_ai_edit_update(
        pool,
        proposal_id,
        applied_update_id,
        &applied_content_sha256,
    )
    .await?
    {
        // Durable applied-binding receipt event (AI_EDIT_PROPOSAL_DECIDED
        // family, applied discriminator).
        let event = NewKernelEvent::builder(
            format!("KTR-KNOWLEDGE-AI-EDIT-{}", proposal.crdt_document_id),
            session_id.to_string(),
            KernelEventType::AiEditProposalDecided,
            actor.to_kernel_actor(),
        )
        .aggregate("knowledge_ai_edit_proposal", proposal_id.to_string())
        .idempotency_key(format!(
            "knowledge-ai-edit:{proposal_id}:applied:{applied_update_id}"
        ))
        .correlation_id(correlation_id.to_string())
        .source_component("knowledge_crdt_ai_edit_proposal")
        .payload(json!({
            "schema_id": AI_EDIT_APPLIED_BINDING_SCHEMA_ID,
            "proposal_id": proposal_id,
            "applied_update_id": applied_update_id,
            "applied_update_sha256": applied_content_sha256,
            "approved_diff_sha256": proposal.diff_sha256,
        }))
        .build()
        .map_err(|error| LeaseFlowError::Event(error.to_string()))?;
        db.append_kernel_event(event)
            .await
            .map_err(|error| LeaseFlowError::Event(error.to_string()))?;
        return Ok(AiEditApplyOutcomeV1::Bound(Box::new(bound)));
    }

    // No row matched -> the applied content did not hash to the approved
    // diff. Emit a durable mismatch denial.
    let receipt_id = new_denial_receipt_id();
    let event = NewKernelEvent::builder(
        format!("KTR-KNOWLEDGE-AI-EDIT-{}", proposal.crdt_document_id),
        session_id.to_string(),
        KernelEventType::AiEditProposalDecided,
        actor.to_kernel_actor(),
    )
    .aggregate("knowledge_ai_edit_proposal", proposal_id.to_string())
    .idempotency_key(format!("knowledge-ai-edit-applied-denial:{receipt_id}"))
    .correlation_id(correlation_id.to_string())
    .source_component("knowledge_crdt_ai_edit_proposal")
    .payload(json!({
        "schema_id": AI_EDIT_APPLIED_MISMATCH_SCHEMA_ID,
        "proposal_id": proposal_id,
        "applied_update_id": applied_update_id,
        "applied_content_sha256": applied_content_sha256,
        "approved_diff_sha256": proposal.diff_sha256,
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
            receipt_kind: "ai_edit_applied_mismatch".to_string(),
            workspace_id: proposal.workspace_id.clone(),
            document_id: Some(proposal.document_id.clone()),
            crdt_document_id: Some(proposal.crdt_document_id.clone()),
            scope_ref: format!("proposal:{proposal_id}"),
            actor_id: actor.canonical(),
            actor_kind: actor.kind().as_str().to_string(),
            session_id: session_id.to_string(),
            correlation_id: correlation_id.to_string(),
            denial_payload: json!({
                "schema_id": AI_EDIT_APPLIED_MISMATCH_SCHEMA_ID,
                "proposal_id": proposal_id,
                "applied_update_id": applied_update_id,
                "applied_content_sha256": applied_content_sha256,
                "approved_diff_sha256": proposal.diff_sha256,
            }),
            event_ledger_event_id: stored_event.event_id.clone(),
            idempotency_key: format!("knowledge-ai-edit-applied-denial:{receipt_id}"),
        },
    )
    .await?;

    Ok(AiEditApplyOutcomeV1::HashMismatch {
        expected_diff_sha256: proposal.diff_sha256,
        applied_content_sha256,
        denial_receipt_id: receipt.receipt_id,
        event_ledger_event_id: stored_event.event_id,
    })
}

/// MT-074 V1 FAIL remediation: emit the durable `ai_edit_applied_update_missing`
/// denial (receipt + EventLedger row) when an applied-binding cannot be anchored
/// to a real, content-consistent `kernel_crdt_updates` row, and refuse the
/// binding. `stored_update_sha256` distinguishes "no update row at all" (`None`)
/// from "update row exists but its persisted content hash disagrees with the
/// presented content" (`Some(stored)`).
#[allow(clippy::too_many_arguments)]
async fn deny_applied_update_missing(
    db: &(dyn Database + '_),
    pool: &PgPool,
    proposal: &AiEditProposalRow,
    applied_update_id: &str,
    stored_update_sha256: Option<String>,
    applied_content_sha256: &str,
    actor: &KnowledgeActorIdV1,
    session_id: &str,
    correlation_id: &str,
) -> Result<AiEditApplyOutcomeV1, LeaseFlowError> {
    let proposal_id = proposal.proposal_id.as_str();
    let receipt_id = new_denial_receipt_id();
    let update_row_absent = stored_update_sha256.is_none();
    let denial_payload = json!({
        "schema_id": AI_EDIT_APPLIED_UPDATE_MISSING_SCHEMA_ID,
        "proposal_id": proposal_id,
        "applied_update_id": applied_update_id,
        "applied_content_sha256": applied_content_sha256,
        "stored_update_sha256": stored_update_sha256.clone(),
        "approved_diff_sha256": proposal.diff_sha256,
        // true => no kernel_crdt_updates row exists; false => row exists but its
        // persisted hash disagrees with the presented content.
        "update_row_absent": update_row_absent,
    });
    let event = NewKernelEvent::builder(
        format!("KTR-KNOWLEDGE-AI-EDIT-{}", proposal.crdt_document_id),
        session_id.to_string(),
        KernelEventType::AiEditProposalDecided,
        actor.to_kernel_actor(),
    )
    .aggregate("knowledge_ai_edit_proposal", proposal_id.to_string())
    .idempotency_key(format!(
        "knowledge-ai-edit-applied-missing-denial:{receipt_id}"
    ))
    .correlation_id(correlation_id.to_string())
    .source_component("knowledge_crdt_ai_edit_proposal")
    .payload(denial_payload.clone())
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
            receipt_kind: "ai_edit_applied_update_missing".to_string(),
            workspace_id: proposal.workspace_id.clone(),
            document_id: Some(proposal.document_id.clone()),
            crdt_document_id: Some(proposal.crdt_document_id.clone()),
            scope_ref: format!("proposal:{proposal_id}"),
            actor_id: actor.canonical(),
            actor_kind: actor.kind().as_str().to_string(),
            session_id: session_id.to_string(),
            correlation_id: correlation_id.to_string(),
            denial_payload,
            event_ledger_event_id: stored_event.event_id.clone(),
            idempotency_key: format!("knowledge-ai-edit-applied-missing-denial:{receipt_id}"),
        },
    )
    .await?;

    Ok(AiEditApplyOutcomeV1::UpdateRowMissing {
        applied_update_id: applied_update_id.to_string(),
        stored_update_sha256,
        applied_content_sha256: applied_content_sha256.to_string(),
        denial_receipt_id: receipt.receipt_id,
        event_ledger_event_id: stored_event.event_id,
    })
}

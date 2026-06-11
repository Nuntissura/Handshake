//! WP-KERNEL-009 MT-068 CRDTAndConcurrencyCore-068-GraphMutationProposalModel.
//!
//! Master Spec anchor: 02-system-architecture.md section 2.3.13.11 —
//! "graph mutation proposals ... MUST leave actor, source span,
//! state-vector, validation, denial, or promotion receipts." Knowledge-graph
//! writes from agents are REPRESENTED AS PROPOSALS: typed draft rows
//! (`knowledge_crdt_graph_proposals`, migration 0152, authority_class
//! 'support') that are reviewed (operator/validator), then rejected or
//! promoted (MT-069 bridge). Drafts are never authority.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::PgPool;

use crate::kernel::{KernelEventType, NewKernelEvent};
use crate::storage::knowledge_crdt::{self, GraphMutationProposalRow, NewGraphMutationProposal};
use crate::storage::{Database, StorageError};

use super::actor_site::{KnowledgeActorIdV1, KnowledgeActorKind};
use super::agent_lease::{
    guard_lease_for_write, KnowledgeLeaseScopeKind, LeaseFlowError, LeaseWriteDenialV1,
    LeaseWriteGuardOutcomeV1,
};

pub const GRAPH_MUTATION_PROPOSAL_SCHEMA_ID: &str =
    "hsk.kernel.knowledge_graph_mutation_proposal@1";

/// Typed knowledge-graph mutations an agent may propose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphMutationKind {
    AddEntity,
    RetireEntity,
    AddEdge,
    RetireEdge,
    AddClaim,
    RetireClaim,
}

impl GraphMutationKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AddEntity => "add_entity",
            Self::RetireEntity => "retire_entity",
            Self::AddEdge => "add_edge",
            Self::RetireEdge => "retire_edge",
            Self::AddClaim => "add_claim",
            Self::RetireClaim => "retire_claim",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "add_entity" => Some(Self::AddEntity),
            "retire_entity" => Some(Self::RetireEntity),
            "add_edge" => Some(Self::AddEdge),
            "retire_edge" => Some(Self::RetireEdge),
            "add_claim" => Some(Self::AddClaim),
            "retire_claim" => Some(Self::RetireClaim),
            _ => None,
        }
    }
}

/// Review states (mirrors the migration CHECK).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalReviewState {
    Proposed,
    Approved,
    Rejected,
    Promoted,
}

impl ProposalReviewState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Promoted => "promoted",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "proposed" => Some(Self::Proposed),
            "approved" => Some(Self::Approved),
            "rejected" => Some(Self::Rejected),
            "promoted" => Some(Self::Promoted),
            _ => None,
        }
    }

    /// The legal state machine: proposed -> approved | rejected;
    /// approved -> promoted. Everything else is refused.
    pub fn can_transition_to(&self, next: ProposalReviewState) -> bool {
        matches!(
            (self, next),
            (Self::Proposed, Self::Approved)
                | (Self::Proposed, Self::Rejected)
                | (Self::Approved, Self::Promoted)
        )
    }
}

/// Request to record a graph mutation proposal.
#[derive(Debug, Clone)]
pub struct GraphMutationProposalRequestV1 {
    pub workspace_id: String,
    pub mutation_kind: GraphMutationKind,
    /// The proposed entity/edge/claim draft payload.
    pub mutation_payload: Value,
    /// Source span refs (spec MUST: at least one; `KSP-...` ids from
    /// knowledge_spans or `pending:<source>:<range>` markers).
    pub source_span_refs: Vec<String>,
    pub confidence: f64,
    pub actor: KnowledgeActorIdV1,
    pub session_id: String,
    pub correlation_id: String,
    /// Lane lease for model actors (MT-041 seed: a lease MUST be claimed
    /// before a model session mutates graph mutation proposals).
    pub lease_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphProposalValidationError {
    EmptyField { field: &'static str },
    NoSourceSpanRefs,
    ConfidenceOutOfRange { found: String },
    ModelActorWithoutLease { actor_id: String },
    PayloadNotObject,
}

impl std::fmt::Display for GraphProposalValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyField { field } => write!(f, "proposal field {field} must not be empty"),
            Self::NoSourceSpanRefs => write!(
                f,
                "graph mutation proposals require at least one source span ref (spec 2.3.13.11)"
            ),
            Self::ConfidenceOutOfRange { found } => {
                write!(f, "confidence {found} is outside 0.0..=1.0")
            }
            Self::ModelActorWithoutLease { actor_id } => write!(
                f,
                "model actor '{actor_id}' must hold a lane lease to write proposals (MT-041 seed)"
            ),
            Self::PayloadNotObject => write!(f, "mutation payload must be a JSON object"),
        }
    }
}

impl std::error::Error for GraphProposalValidationError {}

pub fn validate_graph_proposal_request(
    request: &GraphMutationProposalRequestV1,
) -> Result<(), Vec<GraphProposalValidationError>> {
    let mut errors = Vec::new();
    for (field, value) in [
        ("workspace_id", &request.workspace_id),
        ("session_id", &request.session_id),
        ("correlation_id", &request.correlation_id),
    ] {
        if value.trim().is_empty() {
            errors.push(GraphProposalValidationError::EmptyField { field });
        }
    }
    if request.source_span_refs.is_empty()
        || request
            .source_span_refs
            .iter()
            .any(|span| span.trim().is_empty())
    {
        errors.push(GraphProposalValidationError::NoSourceSpanRefs);
    }
    if !(0.0..=1.0).contains(&request.confidence) || request.confidence.is_nan() {
        errors.push(GraphProposalValidationError::ConfidenceOutOfRange {
            found: request.confidence.to_string(),
        });
    }
    if request.actor.kind().is_model() && request.lease_id.is_none() {
        errors.push(GraphProposalValidationError::ModelActorWithoutLease {
            actor_id: request.actor.canonical(),
        });
    }
    if !request.mutation_payload.is_object() {
        errors.push(GraphProposalValidationError::PayloadNotObject);
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Outcome of recording a graph mutation proposal. A lease denial is a
/// durable runtime denial (receipt + event), distinct from a pre-flight
/// validation error and from success.
#[derive(Debug, Clone, PartialEq)]
pub enum RecordGraphProposalOutcomeV1 {
    Recorded(Box<GraphMutationProposalRow>),
    Invalid(Vec<GraphProposalValidationError>),
    /// Authority-hardening #4: the presented lease failed the server-side
    /// write guard (expired / foreign / released / wrong-scope). Durable.
    LeaseDenied(Box<LeaseWriteDenialV1>),
}

/// Record a graph mutation proposal: GRAPH_MUTATION_PROPOSAL_RECORDED event
/// plus the draft row. Model actors must present a lane lease.
///
/// Authority-hardening #4: when a lease is presented it is routed through
/// [`guard_lease_for_write`] (workspace scope) BEFORE the draft is written —
/// presence-only checking let an expired/foreign/wrong-scope lease through.
/// A guard denial returns [`RecordGraphProposalOutcomeV1::LeaseDenied`] with a
/// durable receipt and NO draft row.
pub async fn record_graph_proposal(
    db: &(dyn Database + '_),
    pool: &PgPool,
    request: GraphMutationProposalRequestV1,
) -> Result<RecordGraphProposalOutcomeV1, LeaseFlowError> {
    if let Err(errors) = validate_graph_proposal_request(&request) {
        return Ok(RecordGraphProposalOutcomeV1::Invalid(errors));
    }

    // Lease chokepoint: a presented lease MUST be live, owned, and cover the
    // workspace scope. (Model actors always present one per the validation
    // above; operator/system writers may omit it.)
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
                return Ok(RecordGraphProposalOutcomeV1::LeaseDenied(denial));
            }
        }
    }

    let proposal_id = super::agent_lease::new_ulid();
    let event = NewKernelEvent::builder(
        format!("KTR-KNOWLEDGE-GRAPH-{}", request.workspace_id),
        request.session_id.clone(),
        KernelEventType::GraphMutationProposalRecorded,
        request.actor.to_kernel_actor(),
    )
    .aggregate("knowledge_graph_proposal", proposal_id.clone())
    .idempotency_key(format!("knowledge-graph-proposal:{proposal_id}:recorded"))
    .correlation_id(request.correlation_id.clone())
    .source_component("knowledge_crdt_graph_proposal")
    .payload(json!({
        "schema_id": GRAPH_MUTATION_PROPOSAL_SCHEMA_ID,
        "proposal_id": proposal_id,
        "workspace_id": request.workspace_id,
        "mutation_kind": request.mutation_kind.as_str(),
        "actor_id": request.actor.canonical(),
        "source_span_refs": request.source_span_refs,
        "confidence": request.confidence,
        "lease_id": request.lease_id,
    }))
    .build()
    .map_err(|error| LeaseFlowError::Event(error.to_string()))?;
    let stored_event = db
        .append_kernel_event(event)
        .await
        .map_err(|error| LeaseFlowError::Event(error.to_string()))?;

    let row = knowledge_crdt::insert_graph_proposal(
        pool,
        NewGraphMutationProposal {
            proposal_id,
            workspace_id: request.workspace_id,
            mutation_kind: request.mutation_kind.as_str().to_string(),
            mutation_payload: request.mutation_payload,
            source_span_refs: request.source_span_refs,
            confidence: request.confidence,
            actor_id: request.actor.canonical(),
            actor_kind: request.actor.kind().as_str().to_string(),
            session_id: request.session_id,
            correlation_id: request.correlation_id,
            lease_id: request.lease_id,
            recorded_event_id: stored_event.event_id,
        },
    )
    .await?;
    Ok(RecordGraphProposalOutcomeV1::Recorded(Box::new(row)))
}

/// Typed review decision errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProposalDecisionError {
    ProposalNotFound { proposal_id: String },
    NotInProposedState { current_state: String },
    ReviewerNotAllowed { reviewer: String },
}

impl std::fmt::Display for ProposalDecisionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProposalNotFound { proposal_id } => {
                write!(f, "graph proposal '{proposal_id}' not found")
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

impl std::error::Error for ProposalDecisionError {}

/// Decide a proposal: approve or reject (GRAPH_MUTATION_PROPOSAL_DECIDED).
/// Reviewers must be operator/validator actors; models cannot self-approve.
pub async fn decide_graph_proposal(
    db: &(dyn Database + '_),
    pool: &PgPool,
    proposal_id: &str,
    approve: bool,
    reviewer: &KnowledgeActorIdV1,
    reviewer_session_id: &str,
    decision_reason: &str,
) -> Result<Result<GraphMutationProposalRow, ProposalDecisionError>, LeaseFlowError> {
    if !matches!(
        reviewer.kind(),
        KnowledgeActorKind::Operator | KnowledgeActorKind::Validator
    ) {
        return Ok(Err(ProposalDecisionError::ReviewerNotAllowed {
            reviewer: reviewer.canonical(),
        }));
    }
    let existing = knowledge_crdt::get_graph_proposal(pool, proposal_id).await?;
    let existing = match existing {
        Some(row) => row,
        None => {
            return Ok(Err(ProposalDecisionError::ProposalNotFound {
                proposal_id: proposal_id.to_string(),
            }));
        }
    };
    if existing.review_state != "proposed" {
        return Ok(Err(ProposalDecisionError::NotInProposedState {
            current_state: existing.review_state,
        }));
    }

    let new_state = if approve { "approved" } else { "rejected" };
    let event = NewKernelEvent::builder(
        format!("KTR-KNOWLEDGE-GRAPH-{}", existing.workspace_id),
        reviewer_session_id.to_string(),
        KernelEventType::GraphMutationProposalDecided,
        reviewer.to_kernel_actor(),
    )
    .aggregate("knowledge_graph_proposal", proposal_id.to_string())
    .idempotency_key(format!(
        "knowledge-graph-proposal:{proposal_id}:decided:{new_state}"
    ))
    .correlation_id(existing.correlation_id.clone())
    .source_component("knowledge_crdt_graph_proposal")
    .payload(json!({
        "schema_id": GRAPH_MUTATION_PROPOSAL_SCHEMA_ID,
        "proposal_id": proposal_id,
        "decision": new_state,
        "decided_by": reviewer.canonical(),
        "decision_reason": decision_reason,
    }))
    .build()
    .map_err(|error| LeaseFlowError::Event(error.to_string()))?;
    let stored_event = db
        .append_kernel_event(event)
        .await
        .map_err(|error| LeaseFlowError::Event(error.to_string()))?;

    let decided = knowledge_crdt::decide_graph_proposal(
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
        // Lost the race: someone decided in between our read and update.
        None => {
            let current = knowledge_crdt::get_graph_proposal(pool, proposal_id)
                .await?
                .map(|row| row.review_state)
                .unwrap_or_else(|| "missing".to_string());
            Ok(Err(ProposalDecisionError::NotInProposedState {
                current_state: current,
            }))
        }
    }
}

/// Convenience: typed view of a stored row's states.
pub fn row_states(
    row: &GraphMutationProposalRow,
) -> (Option<GraphMutationKind>, Option<ProposalReviewState>) {
    (
        GraphMutationKind::parse(&row.mutation_kind),
        ProposalReviewState::parse(&row.review_state),
    )
}

/// Re-exported storage error type for callers matching on conflicts.
pub type GraphProposalStorageError = StorageError;

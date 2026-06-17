//! WP-KERNEL-009 MT-260 — AI Loom suggestion review + promotion flow.
//!
//! Reuses the `ai_edit_proposal` PATTERN: decide (operator/validator only) ->
//! promote (atomic PROMOTION_REQUESTED + PROMOTION_ACCEPTED pair); reject and
//! denied-promotion leave durable receipts. The promotion target depends on the
//! suggestion kind:
//!   - link_suggest -> a real LoomEdge (ai_suggested, created_by=ai)
//!   - auto_tag      -> a tag_hub block (find-or-create) + TAG edge (created_by=ai)
//!   - auto_caption  -> LoomBlockDerived.auto_caption + generated_by provenance
//! Every promoted suggestion's SOURCE block is bridged to knowledge (MT-177) so
//! the result is indistinguishable from operator-authored authority.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::PgPool;

use crate::kernel::crdt::actor_site::{KnowledgeActorIdV1, KnowledgeActorKind};
use crate::kernel::{KernelEventType, NewKernelEvent};
use crate::storage::knowledge_crdt::{
    insert_denial_receipt, new_denial_receipt_id, NewKnowledgeCrdtDenialReceipt,
};
use crate::storage::loom_ai::{
    apply_loom_block_auto_derived, decide_loom_ai_suggestion, get_loom_ai_suggestion,
    mark_loom_ai_suggestion_promoted, LoomAiSuggestionRow,
};
use crate::storage::{
    Database, LoomBlockContentType, LoomBlockDerived, LoomEdgeCreatedBy, LoomEdgeType,
    NewLoomBlock, NewLoomEdge, StorageError, WriteContext,
};

use super::LOOM_AI_SUGGESTION_SCHEMA_ID;

pub const LOOM_AI_PROMOTION_DENIAL_SCHEMA_ID: &str = "hsk.loom.ai_promotion_denial@1";

/// Typed errors for the review/promotion flow.
#[derive(Debug)]
pub enum LoomAiReviewError {
    Storage(StorageError),
    Internal(String),
}

impl std::fmt::Display for LoomAiReviewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Storage(err) => write!(f, "storage error: {err}"),
            Self::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

impl std::error::Error for LoomAiReviewError {}

impl From<StorageError> for LoomAiReviewError {
    fn from(err: StorageError) -> Self {
        Self::Storage(err)
    }
}

/// Typed denial reasons (recorded in the receipt payload + PROMOTION_REJECTED).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum LoomAiPromotionDenialReason {
    UnknownSuggestion { suggestion_id: String },
    NotAccepted { current_state: String },
    GateActorNotAllowed { gate_actor: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoomAiPromotionDenial {
    pub schema_id: String,
    pub suggestion_id: String,
    pub reason: LoomAiPromotionDenialReason,
    pub denial_receipt_id: String,
    pub event_ledger_event_id: String,
}

/// Outcome of accepting (decide+promote) a suggestion.
#[derive(Debug, Clone)]
pub enum LoomAiAcceptOutcome {
    /// The suggestion was accepted and promoted to a real artifact.
    Promoted {
        suggestion: Box<LoomAiSuggestionRow>,
        artifact_ref: String,
    },
    /// Already promoted (idempotent replay).
    AlreadyPromoted(Box<LoomAiSuggestionRow>),
    /// The suggestion does not exist.
    UnknownSuggestion { suggestion_id: String },
    /// The actor is not allowed to confirm/promote (durable denial written).
    Denied(Box<LoomAiPromotionDenial>),
    /// The suggestion was not in 'pending' state (cannot decide).
    NotPending { current_state: String },
}

/// Outcome of rejecting a suggestion.
#[derive(Debug, Clone)]
pub enum LoomAiRejectOutcome {
    Rejected(Box<LoomAiSuggestionRow>),
    UnknownSuggestion { suggestion_id: String },
    Denied(Box<LoomAiPromotionDenial>),
    NotPending { current_state: String },
}

fn reviewer_allowed(actor: &KnowledgeActorIdV1) -> bool {
    matches!(
        actor.kind(),
        KnowledgeActorKind::Operator | KnowledgeActorKind::Validator
    )
}

/// Reject a PENDING suggestion: stamp the decision + write
/// AI_EDIT_PROPOSAL_DECIDED. Authority is untouched (no edge/derived field).
pub async fn reject_loom_ai_suggestion(
    db: &(dyn Database + '_),
    pool: &PgPool,
    suggestion_id: &str,
    reviewer: &KnowledgeActorIdV1,
    reviewer_session_id: &str,
    correlation_id: &str,
    decision_reason: &str,
) -> Result<LoomAiRejectOutcome, LoomAiReviewError> {
    let Some(existing) = get_loom_ai_suggestion(pool, suggestion_id).await? else {
        return Ok(LoomAiRejectOutcome::UnknownSuggestion {
            suggestion_id: suggestion_id.to_string(),
        });
    };
    if !reviewer_allowed(reviewer) {
        let denial = deny(
            db,
            pool,
            &existing,
            reviewer,
            reviewer_session_id,
            correlation_id,
            LoomAiPromotionDenialReason::GateActorNotAllowed {
                gate_actor: reviewer.canonical(),
            },
        )
        .await?;
        return Ok(LoomAiRejectOutcome::Denied(Box::new(denial)));
    }
    if existing.review_state != "pending" {
        return Ok(LoomAiRejectOutcome::NotPending {
            current_state: existing.review_state,
        });
    }

    let event = decided_event(
        db,
        &existing,
        "rejected",
        reviewer,
        reviewer_session_id,
        correlation_id,
        decision_reason,
    )
    .await?;
    let Some(row) =
        decide_loom_ai_suggestion(pool, suggestion_id, "rejected", &reviewer.canonical(), decision_reason, &event)
            .await?
    else {
        let current = get_loom_ai_suggestion(pool, suggestion_id)
            .await?
            .map(|r| r.review_state)
            .unwrap_or_else(|| "missing".to_string());
        return Ok(LoomAiRejectOutcome::NotPending {
            current_state: current,
        });
    };
    Ok(LoomAiRejectOutcome::Rejected(Box::new(row)))
}

/// Accept (decide -> accepted) a PENDING suggestion and PROMOTE it to a real
/// artifact in one flow. Promotion writes the atomic PROMOTION_REQUESTED +
/// PROMOTION_ACCEPTED pair, the real artifact, and bridges the source block to
/// knowledge. Reviewer must be operator/validator.
pub async fn accept_loom_ai_suggestion(
    db: &(dyn Database + '_),
    pool: &PgPool,
    suggestion_id: &str,
    reviewer: &KnowledgeActorIdV1,
    reviewer_session_id: &str,
    correlation_id: &str,
    decision_reason: &str,
) -> Result<LoomAiAcceptOutcome, LoomAiReviewError> {
    let Some(existing) = get_loom_ai_suggestion(pool, suggestion_id).await? else {
        return Ok(LoomAiAcceptOutcome::UnknownSuggestion {
            suggestion_id: suggestion_id.to_string(),
        });
    };
    if existing.review_state == "promoted" {
        return Ok(LoomAiAcceptOutcome::AlreadyPromoted(Box::new(existing)));
    }
    if !reviewer_allowed(reviewer) {
        let denial = deny(
            db,
            pool,
            &existing,
            reviewer,
            reviewer_session_id,
            correlation_id,
            LoomAiPromotionDenialReason::GateActorNotAllowed {
                gate_actor: reviewer.canonical(),
            },
        )
        .await?;
        return Ok(LoomAiAcceptOutcome::Denied(Box::new(denial)));
    }
    if existing.review_state != "pending" {
        return Ok(LoomAiAcceptOutcome::NotPending {
            current_state: existing.review_state,
        });
    }

    // 1) Decide -> accepted (AI_EDIT_PROPOSAL_DECIDED).
    let decided_event = decided_event(
        db,
        &existing,
        "accepted",
        reviewer,
        reviewer_session_id,
        correlation_id,
        decision_reason,
    )
    .await?;
    let Some(accepted) = decide_loom_ai_suggestion(
        pool,
        suggestion_id,
        "accepted",
        &reviewer.canonical(),
        decision_reason,
        &decided_event,
    )
    .await?
    else {
        let current = get_loom_ai_suggestion(pool, suggestion_id)
            .await?
            .map(|r| r.review_state)
            .unwrap_or_else(|| "missing".to_string());
        return Ok(LoomAiAcceptOutcome::NotPending {
            current_state: current,
        });
    };

    // 2) Build the real authority artifact for this kind.
    let artifact_ref = promote_artifact(db, pool, &accepted, reviewer).await?;

    // 3) Atomic promotion event pair.
    let requested = NewKernelEvent::builder(
        format!("KTR-LOOM-AI-{}", accepted.job_id),
        reviewer_session_id.to_string(),
        KernelEventType::PromotionRequested,
        reviewer.to_kernel_actor(),
    )
    .aggregate("loom_ai_promotion", suggestion_id.to_string())
    .idempotency_key(format!("loom-ai-promotion:{suggestion_id}:requested"))
    .correlation_id(correlation_id.to_string())
    .source_component("loom_ai_promotion")
    .payload(json!({
        "schema_id": LOOM_AI_SUGGESTION_SCHEMA_ID,
        "suggestion_id": suggestion_id,
        "kind": accepted.kind,
        "block_id": accepted.block_id,
        "target_block_id": accepted.target_block_id,
        "output_sha256": accepted.output_sha256,
    }))
    .build()
    .map_err(|err| LoomAiReviewError::Internal(err.to_string()))?;
    let promoted_event = NewKernelEvent::builder(
        format!("KTR-LOOM-AI-{}", accepted.job_id),
        reviewer_session_id.to_string(),
        KernelEventType::PromotionAccepted,
        reviewer.to_kernel_actor(),
    )
    .aggregate("loom_ai_promotion", suggestion_id.to_string())
    .idempotency_key(format!("loom-ai-promotion:{suggestion_id}:accepted"))
    .correlation_id(correlation_id.to_string())
    .source_component("loom_ai_promotion")
    .payload(json!({
        "schema_id": LOOM_AI_SUGGESTION_SCHEMA_ID,
        "suggestion_id": suggestion_id,
        "decided_by": reviewer.canonical(),
        "promoted_artifact_ref": artifact_ref,
    }))
    .build()
    .map_err(|err| LoomAiReviewError::Internal(err.to_string()))?;
    let events = db
        .append_kernel_event_pair_atomic_with_causation(requested, promoted_event)
        .await
        .map_err(|err| LoomAiReviewError::Internal(err.to_string()))?;
    let (requested_id, accepted_id) = match events.as_slice() {
        [first, second] => (first.event_id.clone(), second.event_id.clone()),
        _ => {
            return Err(LoomAiReviewError::Internal(
                "promotion event pair returned unexpected event count".to_string(),
            ));
        }
    };

    // 4) Stamp the row promoted.
    let promoted = mark_loom_ai_suggestion_promoted(
        pool,
        suggestion_id,
        &requested_id,
        &accepted_id,
        &artifact_ref,
    )
    .await?
    .ok_or_else(|| LoomAiReviewError::Internal("promotion stamp lost a race".to_string()))?;

    // 5) Bridge the source block to knowledge (MT-177) so the promoted
    // suggestion is indistinguishable from operator-authored authority.
    // Bridge under the reviewer's HUMAN authority for the same reason as the
    // artifact write above (operator/validator-confirmed, not a silent AI edit).
    let bridge_ctx = WriteContext::human(Some(reviewer.canonical()));
    let _ = db
        .bridge_loom_block_to_knowledge(&bridge_ctx, &promoted.workspace_id, &promoted.block_id)
        .await;

    Ok(LoomAiAcceptOutcome::Promoted {
        suggestion: Box::new(promoted),
        artifact_ref,
    })
}

/// Result of an accept-all-of-kind sweep over a job's PENDING suggestions.
#[derive(Debug, Clone, Default)]
pub struct LoomAiAcceptAllOutcome {
    /// suggestion_ids promoted to real authority artifacts.
    pub promoted: Vec<String>,
    /// suggestion_ids that hit a durable denial (e.g. non-operator actor).
    pub denied: Vec<String>,
    /// suggestion_ids skipped (kind filtered out, not pending, or unknown).
    pub skipped: Vec<String>,
}

/// Accept ALL pending suggestions for a job (optionally filtered to one kind).
///
/// This is the canonical accept-all path: it lists the PENDING rows from
/// PostgreSQL (the authority set, NOT a UI-rendered subset) and runs the SAME
/// per-item `accept_loom_ai_suggestion` flow on each. Per-item authority is
/// therefore preserved — a non-operator reviewer promotes NOTHING (every item
/// lands in `denied` with a durable receipt), and an operator/validator
/// promotes every item of the requested kind. The HTTP handler
/// (`api::loom::accept_all_loom_ai_suggestions`) is a thin wrapper over this.
pub async fn accept_all_loom_ai_suggestions(
    db: &(dyn Database + '_),
    pool: &PgPool,
    workspace_id: &str,
    job_id: &str,
    kind_filter: Option<crate::storage::loom_ai::LoomAiJobKind>,
    reviewer: &KnowledgeActorIdV1,
    reviewer_session_id: &str,
    correlation_id: &str,
    decision_reason: &str,
) -> Result<LoomAiAcceptAllOutcome, LoomAiReviewError> {
    let pending = crate::storage::loom_ai::list_loom_ai_suggestions(
        pool,
        workspace_id,
        Some(job_id),
        Some("pending"),
    )
    .await?;

    let mut outcome = LoomAiAcceptAllOutcome::default();
    for row in pending {
        if let Some(kind) = kind_filter {
            if row.kind != kind.as_str() {
                outcome.skipped.push(row.suggestion_id);
                continue;
            }
        }
        let accept = accept_loom_ai_suggestion(
            db,
            pool,
            &row.suggestion_id,
            reviewer,
            reviewer_session_id,
            correlation_id,
            decision_reason,
        )
        .await?;
        match accept {
            LoomAiAcceptOutcome::Promoted { suggestion, .. }
            | LoomAiAcceptOutcome::AlreadyPromoted(suggestion) => {
                outcome.promoted.push(suggestion.suggestion_id.clone());
            }
            LoomAiAcceptOutcome::Denied(_) => outcome.denied.push(row.suggestion_id),
            LoomAiAcceptOutcome::NotPending { .. }
            | LoomAiAcceptOutcome::UnknownSuggestion { .. } => {
                outcome.skipped.push(row.suggestion_id)
            }
        }
    }
    Ok(outcome)
}

/// Build the real authority artifact for an ACCEPTED suggestion and return its
/// reference (edge_id or block_id). AI-authored: edges carry created_by=ai and
/// ai_suggested type; captions carry generated_by provenance.
async fn promote_artifact(
    db: &(dyn Database + '_),
    pool: &PgPool,
    suggestion: &LoomAiSuggestionRow,
    reviewer: &KnowledgeActorIdV1,
) -> Result<String, LoomAiReviewError> {
    // The promotion write is AUTHORIZED by the confirming reviewer (operator or
    // validator — `accept_loom_ai_suggestion` already denied any other actor),
    // so the storage write-context actor is that HUMAN reviewer, not an
    // autonomous AI edit. (The "No Silent Edits" guard rejects an Ai write
    // context that lacks a registered job_id+workflow_id; an operator-confirmed
    // promotion is not a silent AI edit.) The AI ORIGIN of the content is
    // recorded separately and durably: the LoomEdge carries created_by=ai /
    // ai_suggested, and auto_caption carries generated_by provenance.
    let ctx = WriteContext::human(Some(reviewer.canonical()));
    match suggestion.kind.as_str() {
        "link_suggest" => {
            let target = suggestion.target_block_id.clone().ok_or_else(|| {
                LoomAiReviewError::Internal("link_suggest missing target".to_string())
            })?;
            let edge = db
                .create_loom_edge(
                    &ctx,
                    NewLoomEdge {
                        edge_id: None,
                        workspace_id: suggestion.workspace_id.clone(),
                        source_block_id: suggestion.block_id.clone(),
                        target_block_id: target,
                        edge_type: LoomEdgeType::AiSuggested,
                        created_by: LoomEdgeCreatedBy::Ai,
                        crdt_site_id: None,
                        source_anchor: None,
                    },
                )
                .await?;
            Ok(edge.edge_id)
        }
        "auto_tag" => {
            let tag = suggestion
                .suggested_value
                .get("tag")
                .and_then(Value::as_str)
                .ok_or_else(|| LoomAiReviewError::Internal("auto_tag missing tag".to_string()))?
                .to_string();
            // Find-or-create the tag_hub block, then TAG edge (created_by=ai).
            let tag_block = db
                .create_loom_block(
                    &ctx,
                    NewLoomBlock {
                        block_id: None,
                        workspace_id: suggestion.workspace_id.clone(),
                        content_type: LoomBlockContentType::TagHub,
                        document_id: None,
                        asset_id: None,
                        title: Some(tag),
                        original_filename: None,
                        content_hash: None,
                        pinned: false,
                        journal_date: None,
                        imported_at: None,
                        derived: LoomBlockDerived::default(),
                    },
                )
                .await?;
            let edge = db
                .create_loom_edge(
                    &ctx,
                    NewLoomEdge {
                        edge_id: None,
                        workspace_id: suggestion.workspace_id.clone(),
                        source_block_id: suggestion.block_id.clone(),
                        target_block_id: tag_block.block_id,
                        edge_type: LoomEdgeType::Tag,
                        created_by: LoomEdgeCreatedBy::Ai,
                        crdt_site_id: None,
                        source_anchor: None,
                    },
                )
                .await?;
            Ok(edge.edge_id)
        }
        "auto_caption" => {
            let caption = suggestion
                .suggested_value
                .get("caption")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    LoomAiReviewError::Internal("auto_caption missing caption".to_string())
                })?
                .to_string();
            // generated_by.version is typed `String` in LoomBlockDerivedGeneratedBy
            // (storage/loom.rs); the model_attribution `version` may be a JSON
            // number (e.g. max_context_tokens: u32). Coerce to a string so the
            // derived blob deserializes on read-back instead of failing the
            // struct parse and silently falling back to LoomBlockDerived::default().
            let model_str = suggestion
                .model_attribution
                .get("model")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let version_str = match suggestion.model_attribution.get("version") {
                Some(Value::String(s)) => s.clone(),
                Some(Value::Number(n)) => n.to_string(),
                Some(other) => other.to_string(),
                None => String::new(),
            };
            let generated_by = json!({
                "model": model_str,
                "version": version_str,
                "timestamp": chrono::Utc::now(),
            });
            apply_loom_block_auto_derived(
                pool,
                &suggestion.workspace_id,
                &suggestion.block_id,
                Some(&caption),
                None,
                generated_by,
            )
            .await?
            .ok_or_else(|| {
                LoomAiReviewError::Internal("auto_caption target block missing".to_string())
            })
        }
        other => Err(LoomAiReviewError::Internal(format!(
            "unknown suggestion kind {other}"
        ))),
    }
}

/// Build + append the AI_EDIT_PROPOSAL_DECIDED event for a decision, returning
/// its event id.
#[allow(clippy::too_many_arguments)]
async fn decided_event(
    db: &(dyn Database + '_),
    suggestion: &LoomAiSuggestionRow,
    new_state: &str,
    reviewer: &KnowledgeActorIdV1,
    reviewer_session_id: &str,
    correlation_id: &str,
    decision_reason: &str,
) -> Result<String, LoomAiReviewError> {
    let event = NewKernelEvent::builder(
        format!("KTR-LOOM-AI-{}", suggestion.job_id),
        reviewer_session_id.to_string(),
        KernelEventType::AiEditProposalDecided,
        reviewer.to_kernel_actor(),
    )
    .aggregate("loom_ai_suggestion", suggestion.suggestion_id.clone())
    .idempotency_key(format!(
        "loom-ai:{}:decided:{new_state}",
        suggestion.suggestion_id
    ))
    .correlation_id(correlation_id.to_string())
    .source_component("loom_ai_promotion")
    .payload(json!({
        "schema_id": LOOM_AI_SUGGESTION_SCHEMA_ID,
        "suggestion_id": suggestion.suggestion_id,
        "decision": new_state,
        "decided_by": reviewer.canonical(),
        "decision_reason": decision_reason,
        "output_sha256": suggestion.output_sha256,
    }))
    .build()
    .map_err(|err| LoomAiReviewError::Internal(err.to_string()))?;
    let stored = db
        .append_kernel_event(event)
        .await
        .map_err(|err| LoomAiReviewError::Internal(err.to_string()))?;
    Ok(stored.event_id)
}

/// Write a durable `loom_ai_promotion_denied` receipt + PROMOTION_REJECTED
/// event for an unauthorized / wrong-state confirm attempt.
async fn deny(
    db: &(dyn Database + '_),
    pool: &PgPool,
    suggestion: &LoomAiSuggestionRow,
    gate_actor: &KnowledgeActorIdV1,
    gate_session_id: &str,
    correlation_id: &str,
    reason: LoomAiPromotionDenialReason,
) -> Result<LoomAiPromotionDenial, LoomAiReviewError> {
    let receipt_id = new_denial_receipt_id();
    let event = NewKernelEvent::builder(
        format!("KTR-LOOM-AI-{}", suggestion.job_id),
        gate_session_id.to_string(),
        KernelEventType::PromotionRejected,
        gate_actor.to_kernel_actor(),
    )
    .aggregate("loom_ai_promotion", suggestion.suggestion_id.clone())
    .idempotency_key(format!("loom-ai-promotion-denial:{receipt_id}"))
    .correlation_id(correlation_id.to_string())
    .source_component("loom_ai_promotion")
    .payload(json!({
        "schema_id": LOOM_AI_PROMOTION_DENIAL_SCHEMA_ID,
        "suggestion_id": suggestion.suggestion_id,
        "reason": reason,
        "gate_actor": gate_actor.canonical(),
    }))
    .build()
    .map_err(|err| LoomAiReviewError::Internal(err.to_string()))?;
    let stored = db
        .append_kernel_event(event)
        .await
        .map_err(|err| LoomAiReviewError::Internal(err.to_string()))?;

    let receipt = insert_denial_receipt(
        pool,
        NewKnowledgeCrdtDenialReceipt {
            receipt_id: receipt_id.clone(),
            receipt_kind: "loom_ai_promotion_denied".to_string(),
            workspace_id: suggestion.workspace_id.clone(),
            document_id: None,
            crdt_document_id: None,
            scope_ref: format!("loom_ai_suggestion:{}", suggestion.suggestion_id),
            actor_id: gate_actor.canonical(),
            actor_kind: gate_actor.kind().as_str().to_string(),
            session_id: gate_session_id.to_string(),
            correlation_id: correlation_id.to_string(),
            denial_payload: json!({
                "schema_id": LOOM_AI_PROMOTION_DENIAL_SCHEMA_ID,
                "suggestion_id": suggestion.suggestion_id,
                "reason": reason,
            }),
            event_ledger_event_id: stored.event_id.clone(),
            idempotency_key: format!("loom-ai-promotion-denial:{receipt_id}"),
        },
    )
    .await?;

    Ok(LoomAiPromotionDenial {
        schema_id: LOOM_AI_PROMOTION_DENIAL_SCHEMA_ID.to_string(),
        suggestion_id: suggestion.suggestion_id.clone(),
        reason,
        denial_receipt_id: receipt.receipt_id,
        event_ledger_event_id: stored.event_id,
    })
}

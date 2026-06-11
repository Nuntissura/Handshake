//! WP-KERNEL-009 MT-075 CRDTAndConcurrencyCore-075-ConflictUiStateModel.
//!
//! Master Spec anchor: 02-system-architecture.md section 2.3.13.11 — the
//! conflict surface is part of the receipts contract: denied/conflicted
//! writes leave durable receipts, and the UI renders THOSE receipts, never a
//! recomputed guess. This module is the typed payload the frontend conflict
//! UI consumes, computed from CRDT metadata (typed state vectors) plus the
//! durable denial receipts (`knowledge_crdt_denial_receipts`, 0150). Served
//! by GET /knowledge/crdt/conflict_state (api/knowledge_crdt.rs).

use serde::{Deserialize, Serialize};

use crate::storage::knowledge_crdt::KnowledgeCrdtDenialReceiptRow;

use super::state_vector::{KnowledgeStateVectorOrdering, KnowledgeStateVectorV1};
use super::yjs_bridge::KnowledgeDraftHeadV1;

pub const CONFLICT_UI_STATE_SCHEMA_ID: &str = "hsk.kernel.knowledge_conflict_ui_state@1";

/// Conflict kinds the UI distinguishes (superset of the draft-save denial
/// receipt kinds that concern a document).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictUiKindV1 {
    StaleDraftSave,
    ConcurrentDraftFork,
    AheadOfHeadSave,
    LeaseWriteDenied,
    AiEditPromotionDenied,
}

impl ConflictUiKindV1 {
    pub fn from_receipt_kind(receipt_kind: &str) -> Option<Self> {
        match receipt_kind {
            "stale_draft_save" => Some(Self::StaleDraftSave),
            "concurrent_draft_fork" => Some(Self::ConcurrentDraftFork),
            "ahead_of_head_save" => Some(Self::AheadOfHeadSave),
            "lease_write_denied" => Some(Self::LeaseWriteDenied),
            "ai_edit_promotion_denied" => Some(Self::AiEditPromotionDenied),
            _ => None,
        }
    }
}

/// An actor participating in a conflict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflictUiActorV1 {
    pub actor_id: String,
    pub actor_kind: String,
    pub session_id: String,
}

/// A revision reference the UI can render side-by-side.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflictUiRevisionV1 {
    /// `base` (common ancestor the writer saw), `ours` (server head),
    /// `theirs` (the denied writer's attempted state).
    pub label: String,
    pub update_seq: Option<u64>,
    pub update_id: Option<String>,
    pub state_vector: String,
}

/// Resolution options the UI offers; computed from the causal relation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "option", rename_all = "snake_case")]
pub enum ConflictResolutionOptionV1 {
    /// Pull the head, merge locally (Yjs), resubmit rebased update.
    PullMergeResubmit { pull_since_update_seq: u64 },
    /// Discard the local attempt and adopt the server head.
    AdoptServerHead,
    /// Push the missing local updates first (ahead-of-head case).
    PushMissingUpdatesFirst,
    /// Wait for / take over the blocking lease (lease denial case).
    ResolveLeaseFirst { lease_id: String },
    /// Review-flow conflicts route back to the proposal surface.
    ReviewProposal { proposal_id: String },
}

/// One renderable conflict entry (backed 1:1 by a durable denial receipt).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflictUiEntryV1 {
    pub conflict_id: String,
    pub kind: ConflictUiKindV1,
    pub detected_at_utc: String,
    pub conflicting_actors: Vec<ConflictUiActorV1>,
    pub base: Option<ConflictUiRevisionV1>,
    pub ours: Option<ConflictUiRevisionV1>,
    pub theirs: Option<ConflictUiRevisionV1>,
    pub resolution_options: Vec<ConflictResolutionOptionV1>,
    pub denial_receipt_id: String,
    pub event_ledger_event_id: String,
}

/// The typed conflict payload for one document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflictUiStateV1 {
    pub schema_id: String,
    pub workspace_id: String,
    pub document_id: String,
    pub crdt_document_id: String,
    pub head_update_seq: u64,
    pub head_state_vector: String,
    pub conflicts: Vec<ConflictUiEntryV1>,
}

/// Compute the conflict UI state for a document from its draft head and the
/// durable denial receipts. Pure over its inputs (deterministic, testable);
/// the API handler supplies head + receipts from PostgreSQL.
pub fn compute_conflict_ui_state(
    workspace_id: &str,
    document_id: &str,
    crdt_document_id: &str,
    head: &KnowledgeDraftHeadV1,
    receipts: &[KnowledgeCrdtDenialReceiptRow],
) -> ConflictUiStateV1 {
    let mut conflicts = Vec::new();
    for receipt in receipts {
        let Some(kind) = ConflictUiKindV1::from_receipt_kind(&receipt.receipt_kind) else {
            continue;
        };
        let payload = &receipt.denial_payload;
        let decision = payload.get("decision");
        let base_vector = decision
            .and_then(|d| d.get("base_state_vector"))
            .and_then(|v| v.as_str());
        let head_vector_at_denial = decision
            .and_then(|d| d.get("head_state_vector"))
            .and_then(|v| v.as_str());
        let attempted_vector = payload
            .get("attempted_state_vector")
            .and_then(|v| v.as_str());
        let denied_update_id = payload
            .get("denied_update_id")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let head_seq_at_denial = payload
            .get("head_update_seq")
            .and_then(|v| v.as_u64())
            .unwrap_or(head.head_update_seq);

        let base = base_vector.map(|vector| ConflictUiRevisionV1 {
            label: "base".to_string(),
            update_seq: None,
            update_id: None,
            state_vector: vector.to_string(),
        });
        let ours = head_vector_at_denial.map(|vector| ConflictUiRevisionV1 {
            label: "ours".to_string(),
            update_seq: Some(head_seq_at_denial),
            update_id: None,
            state_vector: vector.to_string(),
        });
        let theirs = attempted_vector.map(|vector| ConflictUiRevisionV1 {
            label: "theirs".to_string(),
            update_seq: None,
            update_id: denied_update_id.clone(),
            state_vector: vector.to_string(),
        });

        let resolution_options = resolution_options_for(kind, receipt, head, base_vector);

        conflicts.push(ConflictUiEntryV1 {
            conflict_id: receipt.receipt_id.clone(),
            kind,
            detected_at_utc: receipt.created_at.to_rfc3339(),
            conflicting_actors: vec![ConflictUiActorV1 {
                actor_id: receipt.actor_id.clone(),
                actor_kind: receipt.actor_kind.clone(),
                session_id: receipt.session_id.clone(),
            }],
            base,
            ours,
            theirs,
            resolution_options,
            denial_receipt_id: receipt.receipt_id.clone(),
            event_ledger_event_id: receipt.event_ledger_event_id.clone(),
        });
    }

    ConflictUiStateV1 {
        schema_id: CONFLICT_UI_STATE_SCHEMA_ID.to_string(),
        workspace_id: workspace_id.to_string(),
        document_id: document_id.to_string(),
        crdt_document_id: crdt_document_id.to_string(),
        head_update_seq: head.head_update_seq,
        head_state_vector: head.head_state_vector.clone(),
        conflicts,
    }
}

fn resolution_options_for(
    kind: ConflictUiKindV1,
    receipt: &KnowledgeCrdtDenialReceiptRow,
    head: &KnowledgeDraftHeadV1,
    base_vector: Option<&str>,
) -> Vec<ConflictResolutionOptionV1> {
    match kind {
        ConflictUiKindV1::StaleDraftSave | ConflictUiKindV1::ConcurrentDraftFork => {
            // Pull from the writer's base forward; if the base is parseable
            // we can be precise, otherwise pull everything.
            let pull_since = match (
                base_vector.and_then(|v| KnowledgeStateVectorV1::parse(v).ok()),
                KnowledgeStateVectorV1::parse(&head.head_state_vector).ok(),
            ) {
                (Some(base), Some(head_vector))
                    if head_vector.compare(&base) == KnowledgeStateVectorOrdering::Dominates =>
                {
                    base.lamport_max()
                }
                _ => 0,
            };
            vec![
                ConflictResolutionOptionV1::PullMergeResubmit {
                    pull_since_update_seq: pull_since,
                },
                ConflictResolutionOptionV1::AdoptServerHead,
            ]
        }
        ConflictUiKindV1::AheadOfHeadSave => {
            vec![ConflictResolutionOptionV1::PushMissingUpdatesFirst]
        }
        ConflictUiKindV1::LeaseWriteDenied => {
            let lease_id = receipt
                .denial_payload
                .get("lease_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            vec![ConflictResolutionOptionV1::ResolveLeaseFirst { lease_id }]
        }
        ConflictUiKindV1::AiEditPromotionDenied => {
            let proposal_id = receipt
                .scope_ref
                .strip_prefix("proposal:")
                .unwrap_or(&receipt.scope_ref)
                .to_string();
            vec![ConflictResolutionOptionV1::ReviewProposal { proposal_id }]
        }
    }
}

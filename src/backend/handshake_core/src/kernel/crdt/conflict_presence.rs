use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::identity::{CrdtWorkspaceIdentityV1, validate_crdt_workspace_identity};
use super::persistence::{CrdtUpdateRecordV1, validate_crdt_update_record};

pub const CRDT_CONFLICT_PRESENCE_PROJECTION_SCHEMA_ID: &str =
    "hsk.kernel.crdt_conflict_presence_projection@1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrdtPresenceStatus {
    Active,
    Idle,
    Offline,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtPresenceRecordV1 {
    pub actor_id: String,
    pub actor_kind: String,
    pub session_id: String,
    pub cursor_field_id: String,
    pub cursor_start_byte: usize,
    pub cursor_end_byte: usize,
    pub status: CrdtPresenceStatus,
    pub last_seen_state_vector: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtPendingConflictV1 {
    pub conflict_id: String,
    pub field_id: String,
    pub actor_ids: Vec<String>,
    pub actor_update_ids: Vec<String>,
    pub conflict_summary: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrdtChangePromotionState {
    MergedCrdtOnly,
    PendingPromotion,
    PromotionAccepted,
    PromotionRejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtPromotionStateRefV1 {
    pub update_id: String,
    pub promotion_state: CrdtChangePromotionState,
    pub proposal_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtActorChangeAttributionV1 {
    pub update_id: String,
    pub update_seq: u64,
    pub actor_id: String,
    pub actor_kind: String,
    pub session_id: String,
    pub changed_field_ids: Vec<String>,
    pub promotion_state: CrdtChangePromotionState,
    pub proposal_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtConflictPresenceProjectionV1 {
    pub schema_id: String,
    pub workspace_id: String,
    pub document_id: String,
    pub crdt_document_id: String,
    pub presence: Vec<CrdtPresenceRecordV1>,
    pub pending_conflicts: Vec<CrdtPendingConflictV1>,
    pub actor_attributions: Vec<CrdtActorChangeAttributionV1>,
    pub merged_crdt_update_ids: Vec<String>,
    pub pending_promotion_update_ids: Vec<String>,
    pub accepted_promotion_update_ids: Vec<String>,
    pub rejected_promotion_update_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtConflictPresenceInputV1 {
    pub identity: CrdtWorkspaceIdentityV1,
    pub presence_records: Vec<CrdtPresenceRecordV1>,
    pub pending_conflicts: Vec<CrdtPendingConflictV1>,
    pub updates: Vec<CrdtUpdateRecordV1>,
    pub promotion_states: Vec<CrdtPromotionStateRefV1>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrdtConflictPresenceValidationError {
    pub field: &'static str,
    pub message: &'static str,
}

pub fn build_crdt_conflict_presence_projection(
    input: CrdtConflictPresenceInputV1,
) -> Result<CrdtConflictPresenceProjectionV1, Vec<CrdtConflictPresenceValidationError>> {
    validate_input(&input)?;

    let promotion_states = input
        .promotion_states
        .iter()
        .map(|state| (state.update_id.clone(), state.clone()))
        .collect::<HashMap<_, _>>();
    let mut updates = input.updates.clone();
    updates.sort_by_key(|update| update.update_seq);

    let mut actor_attributions = Vec::new();
    let mut merged_crdt_update_ids = Vec::new();
    let mut pending_promotion_update_ids = Vec::new();
    let mut accepted_promotion_update_ids = Vec::new();
    let mut rejected_promotion_update_ids = Vec::new();

    for update in updates {
        let promotion_ref = promotion_states.get(&update.update_id);
        let promotion_state = promotion_ref
            .map(|state| state.promotion_state)
            .unwrap_or(CrdtChangePromotionState::MergedCrdtOnly);
        match promotion_state {
            CrdtChangePromotionState::MergedCrdtOnly => {
                merged_crdt_update_ids.push(update.update_id.clone())
            }
            CrdtChangePromotionState::PendingPromotion => {
                pending_promotion_update_ids.push(update.update_id.clone())
            }
            CrdtChangePromotionState::PromotionAccepted => {
                accepted_promotion_update_ids.push(update.update_id.clone())
            }
            CrdtChangePromotionState::PromotionRejected => {
                rejected_promotion_update_ids.push(update.update_id.clone())
            }
        }

        actor_attributions.push(CrdtActorChangeAttributionV1 {
            update_id: update.update_id.clone(),
            update_seq: update.update_seq,
            actor_id: update.actor_id,
            actor_kind: update.actor_kind,
            session_id: update.session_id,
            changed_field_ids: changed_field_ids(&input.pending_conflicts, &update.update_id),
            promotion_state,
            proposal_id: promotion_ref.and_then(|state| state.proposal_id.clone()),
        });
    }

    Ok(CrdtConflictPresenceProjectionV1 {
        schema_id: CRDT_CONFLICT_PRESENCE_PROJECTION_SCHEMA_ID.to_string(),
        workspace_id: input.identity.workspace_id,
        document_id: input.identity.document_id,
        crdt_document_id: input.identity.crdt_document_id,
        presence: input.presence_records,
        pending_conflicts: input.pending_conflicts,
        actor_attributions,
        merged_crdt_update_ids,
        pending_promotion_update_ids,
        accepted_promotion_update_ids,
        rejected_promotion_update_ids,
    })
}

fn validate_input(
    input: &CrdtConflictPresenceInputV1,
) -> Result<(), Vec<CrdtConflictPresenceValidationError>> {
    let mut errors = Vec::new();

    if let Err(identity_errors) = validate_crdt_workspace_identity(&input.identity) {
        for identity_error in identity_errors {
            errors.push(CrdtConflictPresenceValidationError {
                field: identity_error.field,
                message: identity_error.message,
            });
        }
    }

    for presence in &input.presence_records {
        require_non_empty(&mut errors, "presence.actor_id", &presence.actor_id);
        require_non_empty(&mut errors, "presence.actor_kind", &presence.actor_kind);
        require_non_empty(&mut errors, "presence.session_id", &presence.session_id);
        require_non_empty(
            &mut errors,
            "presence.cursor_field_id",
            &presence.cursor_field_id,
        );
        require_non_empty(
            &mut errors,
            "presence.last_seen_state_vector",
            &presence.last_seen_state_vector,
        );
        if presence.cursor_start_byte > presence.cursor_end_byte {
            errors.push(CrdtConflictPresenceValidationError {
                field: "presence.cursor_range",
                message: "cursor start must not exceed cursor end",
            });
        }
    }

    for conflict in &input.pending_conflicts {
        require_non_empty(
            &mut errors,
            "pending_conflicts.conflict_id",
            &conflict.conflict_id,
        );
        require_non_empty(
            &mut errors,
            "pending_conflicts.field_id",
            &conflict.field_id,
        );
        require_non_empty(
            &mut errors,
            "pending_conflicts.conflict_summary",
            &conflict.conflict_summary,
        );
        if conflict.actor_ids.is_empty() {
            errors.push(CrdtConflictPresenceValidationError {
                field: "pending_conflicts.actor_ids",
                message: "pending conflict must cite actor ids",
            });
        }
        if conflict.actor_update_ids.is_empty() {
            errors.push(CrdtConflictPresenceValidationError {
                field: "pending_conflicts.actor_update_ids",
                message: "pending conflict must cite actor update ids",
            });
        }
    }

    for update in &input.updates {
        if let Err(update_errors) = validate_crdt_update_record(update) {
            for update_error in update_errors {
                errors.push(CrdtConflictPresenceValidationError {
                    field: update_error.field,
                    message: update_error.message,
                });
            }
        }
        if update.workspace_id != input.identity.workspace_id
            || update.document_id != input.identity.document_id
            || update.crdt_document_id != input.identity.crdt_document_id
        {
            errors.push(CrdtConflictPresenceValidationError {
                field: "updates.identity",
                message: "update identity must match projection identity",
            });
        }
    }

    for promotion_state in &input.promotion_states {
        require_non_empty(
            &mut errors,
            "promotion_states.update_id",
            &promotion_state.update_id,
        );
        if matches!(
            promotion_state.promotion_state,
            CrdtChangePromotionState::PendingPromotion
                | CrdtChangePromotionState::PromotionAccepted
        ) && promotion_state
            .proposal_id
            .as_ref()
            .is_none_or(|proposal_id| proposal_id.trim().is_empty())
        {
            errors.push(CrdtConflictPresenceValidationError {
                field: "promotion_states.proposal_id",
                message: "pending or accepted promotion state requires a proposal id",
            });
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn changed_field_ids(conflicts: &[CrdtPendingConflictV1], update_id: &str) -> Vec<String> {
    conflicts
        .iter()
        .filter(|conflict| {
            conflict
                .actor_update_ids
                .iter()
                .any(|conflict_update_id| conflict_update_id == update_id)
        })
        .map(|conflict| conflict.field_id.clone())
        .collect()
}

fn require_non_empty(
    errors: &mut Vec<CrdtConflictPresenceValidationError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(CrdtConflictPresenceValidationError {
            field,
            message: "value must not be empty",
        });
    }
}

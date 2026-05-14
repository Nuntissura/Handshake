use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::kernel::action_envelope::{AuthorityEffect, EventLedgerMapping};

use super::persistence::sha256_hex;
use super::validity_guard::{
    CrdtMaterializedStateV1, CrdtPromotionValidationDecision, CrdtPromotionValidationReportV1,
    CrdtStateValidationError,
};

pub const CRDT_ARTIFACT_PROPOSAL_SCHEMA_ID: &str = "hsk.kernel.crdt_artifact_proposal@1";
pub const CRDT_PROMOTION_GATE_INPUT_SCHEMA_ID: &str = "hsk.kernel.crdt_promotion_gate_input@1";
pub const CRDT_REJECTED_PROMOTION_EVIDENCE_SCHEMA_ID: &str =
    "hsk.kernel.crdt_rejected_promotion_evidence@1";
pub const CRDT_PROMOTION_BRIDGE_RESULT_SCHEMA_ID: &str =
    "hsk.kernel.crdt_promotion_bridge_result@1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtPromotionBridgeInputV1 {
    pub bridge_id: String,
    pub artifact_proposal_id: String,
    pub promotion_gate_id: String,
    pub promotion_target_ref: String,
    pub state: CrdtMaterializedStateV1,
    pub validation_report: CrdtPromotionValidationReportV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrdtPromotionBridgeStatus {
    Accepted,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtArtifactProposalV1 {
    pub schema_id: String,
    pub artifact_proposal_id: String,
    pub workspace_id: String,
    pub document_id: String,
    pub crdt_document_id: String,
    pub document_schema_id: String,
    pub state_hash: String,
    pub source_update_ids: Vec<String>,
    pub validation_report_ref: String,
    pub authority_effect: AuthorityEffect,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtPromotionGateInputV1 {
    pub schema_id: String,
    pub promotion_gate_id: String,
    pub artifact_proposal_id: String,
    pub promotion_target_ref: String,
    pub validation_report_ref: String,
    pub event_ledger_stream_id: String,
    pub state_hash: String,
    pub authority_effect: AuthorityEffect,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtRejectedPromotionEvidenceV1 {
    pub schema_id: String,
    pub evidence_id: String,
    pub workspace_id: String,
    pub document_id: String,
    pub crdt_document_id: String,
    pub state_hash: String,
    pub validation_errors: Vec<CrdtStateValidationError>,
    pub authority_effect: AuthorityEffect,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtPromotionBridgeResultV1 {
    pub schema_id: String,
    pub bridge_id: String,
    pub status: CrdtPromotionBridgeStatus,
    pub artifact_proposal: Option<CrdtArtifactProposalV1>,
    pub promotion_gate_input: Option<CrdtPromotionGateInputV1>,
    pub event_mapping: Option<EventLedgerMapping>,
    pub rejection_evidence: Option<CrdtRejectedPromotionEvidenceV1>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrdtPromotionBridgeError {
    ValidationReportMisaligned(&'static str),
    Serialization(String),
}

pub fn bridge_crdt_state_to_promotion(
    input: CrdtPromotionBridgeInputV1,
) -> Result<CrdtPromotionBridgeResultV1, CrdtPromotionBridgeError> {
    validate_report_alignment(&input)?;
    let state_hash = state_hash(&input.state)?;

    if input.validation_report.promotion_allowed
        && input.validation_report.decision == CrdtPromotionValidationDecision::Allowed
    {
        accepted_bridge_result(input, state_hash)
    } else {
        rejected_bridge_result(input, state_hash)
    }
}

fn accepted_bridge_result(
    input: CrdtPromotionBridgeInputV1,
    state_hash: String,
) -> Result<CrdtPromotionBridgeResultV1, CrdtPromotionBridgeError> {
    let validation_report_ref = validation_report_ref(&input.bridge_id);
    let event_ledger_stream_id = input
        .state
        .identity
        .authority_links
        .event_ledger_stream_id
        .clone();
    let artifact_proposal = CrdtArtifactProposalV1 {
        schema_id: CRDT_ARTIFACT_PROPOSAL_SCHEMA_ID.to_string(),
        artifact_proposal_id: input.artifact_proposal_id.clone(),
        workspace_id: input.state.identity.workspace_id.clone(),
        document_id: input.state.identity.document_id.clone(),
        crdt_document_id: input.state.identity.crdt_document_id.clone(),
        document_schema_id: input.state.document_schema_id.clone(),
        state_hash: state_hash.clone(),
        source_update_ids: source_update_ids(&input.state),
        validation_report_ref: validation_report_ref.clone(),
        authority_effect: AuthorityEffect::PrePromotionEvidenceOnly,
    };
    let promotion_gate_input = CrdtPromotionGateInputV1 {
        schema_id: CRDT_PROMOTION_GATE_INPUT_SCHEMA_ID.to_string(),
        promotion_gate_id: input.promotion_gate_id.clone(),
        artifact_proposal_id: input.artifact_proposal_id.clone(),
        promotion_target_ref: input.promotion_target_ref.clone(),
        validation_report_ref,
        event_ledger_stream_id,
        state_hash: state_hash.clone(),
        authority_effect: AuthorityEffect::EventLedgerAuthorityWrite,
    };
    let event_mapping = EventLedgerMapping {
        event_kind: "KernelCrdtPromotionAcceptedV1".to_string(),
        event_schema_id: "hsk.event.kernel_crdt_promotion_accepted@1".to_string(),
        idempotency_key: format!(
            "{}:{}:{}",
            input.bridge_id, input.artifact_proposal_id, state_hash
        ),
    };

    Ok(CrdtPromotionBridgeResultV1 {
        schema_id: CRDT_PROMOTION_BRIDGE_RESULT_SCHEMA_ID.to_string(),
        bridge_id: input.bridge_id,
        status: CrdtPromotionBridgeStatus::Accepted,
        artifact_proposal: Some(artifact_proposal),
        promotion_gate_input: Some(promotion_gate_input),
        event_mapping: Some(event_mapping),
        rejection_evidence: None,
    })
}

fn rejected_bridge_result(
    input: CrdtPromotionBridgeInputV1,
    state_hash: String,
) -> Result<CrdtPromotionBridgeResultV1, CrdtPromotionBridgeError> {
    let rejection_evidence = CrdtRejectedPromotionEvidenceV1 {
        schema_id: CRDT_REJECTED_PROMOTION_EVIDENCE_SCHEMA_ID.to_string(),
        evidence_id: format!("{}:rejected", input.bridge_id),
        workspace_id: input.state.identity.workspace_id.clone(),
        document_id: input.state.identity.document_id.clone(),
        crdt_document_id: input.state.identity.crdt_document_id.clone(),
        state_hash,
        validation_errors: input.validation_report.validation_errors.clone(),
        authority_effect: AuthorityEffect::PrePromotionEvidenceOnly,
    };

    Ok(CrdtPromotionBridgeResultV1 {
        schema_id: CRDT_PROMOTION_BRIDGE_RESULT_SCHEMA_ID.to_string(),
        bridge_id: input.bridge_id,
        status: CrdtPromotionBridgeStatus::Rejected,
        artifact_proposal: None,
        promotion_gate_input: None,
        event_mapping: None,
        rejection_evidence: Some(rejection_evidence),
    })
}

fn validate_report_alignment(
    input: &CrdtPromotionBridgeInputV1,
) -> Result<(), CrdtPromotionBridgeError> {
    match (
        input.validation_report.decision,
        input.validation_report.promotion_allowed,
    ) {
        (CrdtPromotionValidationDecision::Allowed, true)
        | (CrdtPromotionValidationDecision::Denied, false) => {}
        _ => {
            return Err(CrdtPromotionBridgeError::ValidationReportMisaligned(
                "validation decision and promotion_allowed disagree",
            ));
        }
    }

    if input.validation_report.document_schema_id != input.state.document_schema_id {
        return Err(CrdtPromotionBridgeError::ValidationReportMisaligned(
            "validation report document schema does not match state",
        ));
    }
    if input.validation_report.state_vector != input.state.state_vector
        || input.validation_report.latest_update_seq != input.state.latest_update_seq
    {
        return Err(CrdtPromotionBridgeError::ValidationReportMisaligned(
            "validation report CRDT version does not match state",
        ));
    }

    Ok(())
}

fn state_hash(state: &CrdtMaterializedStateV1) -> Result<String, CrdtPromotionBridgeError> {
    let bytes = serde_json::to_vec(state)
        .map_err(|error| CrdtPromotionBridgeError::Serialization(error.to_string()))?;
    Ok(sha256_hex(&bytes))
}

fn source_update_ids(state: &CrdtMaterializedStateV1) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut ids = Vec::new();
    for field in &state.fields {
        for update_id in &field.source_update_ids {
            if seen.insert(update_id.clone()) {
                ids.push(update_id.clone());
            }
        }
    }
    ids
}

fn validation_report_ref(bridge_id: &str) -> String {
    format!("validation-report://{bridge_id}")
}

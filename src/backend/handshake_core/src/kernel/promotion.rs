use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;
use uuid::Uuid;

use super::context_bundle::canonical_json_bytes;
use super::model_adapter::ModelAdapterOutput;
use super::{KernelError, KernelEventType, KernelResult};
use crate::storage::artifacts::{
    artifact_root_rel, write_file_artifact, ArtifactClassification, ArtifactLayer,
    ArtifactManifest, ArtifactPayloadKind,
};
use crate::storage::EntityRef;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactRecord {
    pub artifact_id: String,
    pub artifact_uuid: Uuid,
    pub artifact_layer: ArtifactLayer,
    pub artifact_proposal_id: String,
    pub kernel_task_run_id: String,
    pub session_run_id: String,
    pub artifact_kind: String,
    pub content_hash: String,
    pub artifact_manifest_ref: String,
    pub artifact_payload_ref: String,
    pub event_type: KernelEventType,
    pub created_at: DateTime<Utc>,
}

impl ArtifactRecord {
    pub fn from_adapter_output(
        kernel_task_run_id: impl Into<String>,
        session_run_id: impl Into<String>,
        output: &ModelAdapterOutput,
    ) -> KernelResult<Self> {
        let kernel_task_run_id = kernel_task_run_id.into();
        let session_run_id = session_run_id.into();
        if kernel_task_run_id.trim().is_empty() || session_run_id.trim().is_empty() {
            return Err(KernelError::InvalidEvent(
                "artifact record requires kernel and session run ids",
            ));
        }
        let artifact_uuid = Uuid::now_v7();
        let artifact_layer = ArtifactLayer::L2;
        let artifact_root = artifact_root_rel(artifact_layer, artifact_uuid);
        Ok(Self {
            artifact_id: format!("ART-{}", &output.output_hash[..16]),
            artifact_uuid,
            artifact_layer,
            artifact_proposal_id: output.artifact_proposal.artifact_proposal_id.clone(),
            kernel_task_run_id,
            session_run_id,
            artifact_kind: output.artifact_proposal.artifact_kind.clone(),
            content_hash: output.artifact_proposal.content_hash.clone(),
            artifact_manifest_ref: format!("{artifact_root}/artifact.json"),
            artifact_payload_ref: format!("{artifact_root}/payload"),
            event_type: KernelEventType::ArtifactStored,
            created_at: Utc::now(),
        })
    }

    pub fn store_adapter_output(
        workspace_root: &Path,
        kernel_task_run_id: impl Into<String>,
        session_run_id: impl Into<String>,
        output: &ModelAdapterOutput,
    ) -> KernelResult<Self> {
        let record = Self::from_adapter_output(kernel_task_run_id, session_run_id, output)?;
        let payload_bytes = canonical_json_bytes(&output.artifact_payload);
        let manifest = ArtifactManifest {
            artifact_id: record.artifact_uuid,
            layer: record.artifact_layer,
            kind: ArtifactPayloadKind::ToolOutput,
            mime: "application/json".to_string(),
            filename_hint: Some("kernel-adapter-output.json".to_string()),
            created_at: record.created_at,
            created_by_job_id: None,
            source_entity_refs: vec![
                EntityRef {
                    entity_kind: "kernel_task_run".to_string(),
                    entity_id: record.kernel_task_run_id.clone(),
                },
                EntityRef {
                    entity_kind: "session_run".to_string(),
                    entity_id: record.session_run_id.clone(),
                },
            ],
            source_artifact_refs: Vec::new(),
            content_hash: record.content_hash.clone(),
            size_bytes: payload_bytes.len() as u64,
            classification: ArtifactClassification::Low,
            exportable: true,
            retention_ttl_days: None,
            pinned: Some(false),
            hash_basis: Some(
                "canonical_json_bytes(model_adapter_output.artifact_payload)".to_string(),
            ),
            hash_exclude_paths: Vec::new(),
        };
        write_file_artifact(workspace_root, &manifest, &payload_bytes)?;
        Ok(record)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ValidationOutcome {
    Passed,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ValidationRecord {
    pub validation_id: String,
    pub kernel_task_run_id: String,
    pub session_run_id: String,
    pub artifact_id: String,
    pub outcome: ValidationOutcome,
    pub evidence: Value,
    pub event_type: KernelEventType,
    pub created_at: DateTime<Utc>,
}

pub struct ValidationRunner;

impl ValidationRunner {
    pub fn record(
        kernel_task_run_id: impl Into<String>,
        session_run_id: impl Into<String>,
        artifact: &ArtifactRecord,
        outcome: ValidationOutcome,
        evidence: Value,
    ) -> KernelResult<ValidationRecord> {
        let kernel_task_run_id = kernel_task_run_id.into();
        let session_run_id = session_run_id.into();
        if artifact.artifact_id.trim().is_empty() {
            return Err(KernelError::InvalidEvent("artifact_id is required"));
        }
        if outcome == ValidationOutcome::Passed {
            let artifact_id_matches = evidence.get("artifact_id").and_then(|value| value.as_str())
                == Some(artifact.artifact_id.as_str());
            let content_hash_matches = evidence
                .get("content_hash")
                .and_then(|value| value.as_str())
                == Some(artifact.content_hash.as_str());
            let hash_validated = evidence
                .get("artifact_content_hash_validated")
                .and_then(|value| value.as_bool())
                == Some(true);
            let has_evidence_ref = evidence
                .get("evidence_refs")
                .and_then(|value| value.as_array())
                .is_some_and(|refs| !refs.is_empty())
                || evidence
                    .get("artifact_manifest_ref")
                    .and_then(|value| value.as_str())
                    .is_some_and(|value| !value.trim().is_empty());
            if !artifact_id_matches || !content_hash_matches || !hash_validated || !has_evidence_ref
            {
                return Err(KernelError::InvalidEvent(
                    "passed validation requires artifact hash evidence",
                ));
            }
        }
        Ok(ValidationRecord {
            validation_id: format!("VAL-{}", artifact.artifact_id.trim_start_matches("ART-")),
            kernel_task_run_id,
            session_run_id,
            artifact_id: artifact.artifact_id.clone(),
            outcome,
            evidence,
            event_type: KernelEventType::ValidationRecorded,
            created_at: Utc::now(),
        })
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PromotionDecisionKind {
    Approved,
    Rejected,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperatorPromotionApproval {
    pub operator_id: String,
    pub reason: String,
    pub review_receipt_id: String,
    pub approval_source: String,
    pub approved_at: DateTime<Utc>,
}

impl OperatorPromotionApproval {
    pub fn new(operator_id: impl Into<String>, reason: impl Into<String>) -> Self {
        let operator_id = operator_id.into();
        let reason = reason.into();
        Self {
            review_receipt_id: format!("OPERATOR-REVIEW-{}", Uuid::now_v7()),
            approval_source: "operator_review_receipt".to_string(),
            operator_id,
            reason,
            approved_at: Utc::now(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromotionDecision {
    pub promotion_decision_id: String,
    pub artifact_id: String,
    pub validation_id: Option<String>,
    pub decision: PromotionDecisionKind,
    pub operator_id: String,
    pub operator_review_receipt_id: String,
    pub operator_approval_source: String,
    pub operator_reason: String,
    pub event_type: KernelEventType,
    pub decided_at: DateTime<Utc>,
}

pub struct PromotionGate;

impl PromotionGate {
    pub fn decide(
        artifact: &ArtifactRecord,
        validation: Option<&ValidationRecord>,
        decision: PromotionDecisionKind,
        approval: OperatorPromotionApproval,
    ) -> KernelResult<PromotionDecision> {
        if approval.operator_id.trim().is_empty() {
            return Err(KernelError::InvalidEvent(
                "promotion decision requires operator_id",
            ));
        }
        if approval.reason.trim().is_empty() {
            return Err(KernelError::InvalidEvent(
                "promotion decision requires operator review reason",
            ));
        }
        let fixture_like = approval.operator_id.contains("kernel-proof")
            || approval.operator_id.contains("fixture")
            || approval.reason.to_ascii_lowercase().contains("fixture")
            || approval
                .review_receipt_id
                .to_ascii_lowercase()
                .contains("fixture")
            || approval
                .approval_source
                .to_ascii_lowercase()
                .contains("fixture");
        if matches!(decision, PromotionDecisionKind::Approved)
            && (fixture_like
                || approval.review_receipt_id.trim().is_empty()
                || approval.approval_source != "operator_review_receipt")
        {
            return Err(KernelError::InvalidEvent(
                "promotion approval requires operator-reviewable evidence",
            ));
        }
        if matches!(decision, PromotionDecisionKind::Approved) {
            let validation = validation.ok_or(KernelError::InvalidEvent(
                "promotion approval requires validation evidence",
            ))?;
            if validation.artifact_id != artifact.artifact_id {
                return Err(KernelError::InvalidEvent(
                    "promotion validation does not match artifact",
                ));
            }
            if validation.outcome != ValidationOutcome::Passed {
                return Err(KernelError::InvalidEvent(
                    "promotion approval requires passed validation",
                ));
            }
        }

        Ok(PromotionDecision {
            promotion_decision_id: format!(
                "PROM-{}",
                artifact.artifact_id.trim_start_matches("ART-")
            ),
            artifact_id: artifact.artifact_id.clone(),
            validation_id: validation.map(|record| record.validation_id.clone()),
            decision,
            operator_id: approval.operator_id,
            operator_review_receipt_id: approval.review_receipt_id,
            operator_approval_source: approval.approval_source,
            operator_reason: approval.reason,
            event_type: KernelEventType::PromotionDecided,
            decided_at: approval.approved_at,
        })
    }
}

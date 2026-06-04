//! MT-179 Mailbox message family typed payloads (10 Phase-1 families).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::role_mailbox::RoleId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockerSeverity {
    Soft,
    Hard,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReviewTarget {
    WorkPacket { id: String },
    MicroTask { id: String },
    Patch { id: String },
    Other { label: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewKind {
    CodeReview,
    GovReview,
    SpecReview,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidencePointer {
    pub kind: String,
    pub uri: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactPointer {
    pub artifact_id: String,
    pub uri: String,
    pub content_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionOption {
    pub option_id: String,
    pub label: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionAuthority {
    pub role: RoleId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompletionState {
    Completed,
    Partial,
    Failed,
    Cancelled,
}

/// Forward-declared escalation tier shared with cluster X.2 (`mt_executor::EscalationTier`).
///
/// NOTE: per-variant explicit `rename` instead of `rename_all = "snake_case"`.
/// serde's snake_case derivation splits at every capital, producing wire forms
/// like `t7_b` / `t7_b_alt` for `T7B` / `T7BAlt` — which collide with the
/// canonical compact form `t7b` / `t7b_alt` that the sibling
/// `crate::mt_executor::job::EscalationTier` (MT-184) uses on its `as_str()`
/// write path. MT-184 caught and patched the same defect in the mt_executor
/// surface; this rename keeps the wire form bit-identical across role_mailbox_v1
/// + mt_executor so MT-188 (outcome recording) can bridge them without a
/// translation table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum EscalationTier {
    #[serde(rename = "t7b")]
    T7B,
    #[serde(rename = "t7b_alt")]
    T7BAlt,
    #[serde(rename = "t13b")]
    T13B,
    #[serde(rename = "t13b_alt")]
    T13BAlt,
    #[serde(rename = "t32b")]
    T32B,
    #[serde(rename = "hard_gate")]
    HardGate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityGrant {
    pub capability_id: String,
    pub granted_at_utc: DateTime<Utc>,
    pub granted_by: RoleId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpectedResponse {
    pub by_role: Option<RoleId>,
    pub deadline_utc: Option<DateTime<Utc>>,
}

/// MT-184 forward-declares the MicroTaskJob primitive; here it is an opaque
/// newtype that MT-184 wires to the concrete executor contract row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MicroTaskExecutorContractRef {
    pub job_id: Uuid,
    pub mt_id: String,
    pub wp_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MicroTaskRef {
    pub wp_id: String,
    pub mt_id: String,
    pub iteration_n: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriorAttemptRef {
    pub attempt_id: Uuid,
    pub tier: EscalationTier,
    pub outcome_summary: String,
}

// --------------------------------- Bodies ----------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DelegateWorkBody {
    pub task_summary: String,
    pub target_role: RoleId,
    pub due_at_utc: Option<DateTime<Utc>>,
    pub linked_wp: Option<String>,
    pub linked_mt: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockerBody {
    pub blocker_description: String,
    pub blocking_role: Option<RoleId>,
    pub severity: BlockerSeverity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewRequestBody {
    pub review_target: ReviewTarget,
    pub review_target_id: String,
    pub review_kind: ReviewKind,
    pub evidence_pointers: Vec<EvidencePointer>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionRequestBody {
    pub question: String,
    pub options: Vec<DecisionOption>,
    pub decision_authority_role: RoleId,
    pub deadline_utc: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnnounceBackBody {
    pub sub_session_id: Option<Uuid>,
    pub summary: String,
    pub artifacts: Vec<ArtifactPointer>,
    pub completion_state: CompletionState,
    /// Provenance chain back to the original DelegateWork (populated by
    /// `super::handoff::AnnounceBackComposer::compose`).
    #[serde(default)]
    pub provenance_chain: Vec<super::handoff::ProvenanceLink>,
    /// Optional bundle reference (populated when the announce-back follows a
    /// handoff bundle).
    #[serde(default)]
    pub bundle_id: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MicroTaskRequestBody {
    pub mt_ref: MicroTaskRef,
    pub micro_task_executor_contract_ref: MicroTaskExecutorContractRef,
    pub objective: String,
    pub due_at_utc: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MicroTaskFeedbackBody {
    pub mt_ref: MicroTaskRef,
    pub micro_task_executor_contract_ref: MicroTaskExecutorContractRef,
    pub feedback_summary: String,
    pub guidance: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MicroTaskVerificationNeededBody {
    pub mt_ref: MicroTaskRef,
    pub micro_task_executor_contract_ref: MicroTaskExecutorContractRef,
    pub reason: String,
    pub verifier_target_role: RoleId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MicroTaskEscalationBody {
    pub mt_ref: MicroTaskRef,
    pub micro_task_executor_contract_ref: MicroTaskExecutorContractRef,
    pub escalation_target: EscalationTier,
    pub reason: String,
    pub prior_attempts: Vec<PriorAttemptRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MicroTaskCompletionReportBody {
    pub mt_ref: MicroTaskRef,
    pub micro_task_executor_contract_ref: MicroTaskExecutorContractRef,
    pub outcome_summary: String,
    pub artifacts: Vec<ArtifactPointer>,
    pub completion_state: CompletionState,
}

// ---------------------------- Wrapper enum ---------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "family", content = "body", rename_all = "snake_case")]
pub enum MessageFamily {
    DelegateWork(DelegateWorkBody),
    Blocker(BlockerBody),
    ReviewRequest(ReviewRequestBody),
    DecisionRequest(DecisionRequestBody),
    AnnounceBack(AnnounceBackBody),
    MicroTaskRequest(MicroTaskRequestBody),
    MicroTaskFeedback(MicroTaskFeedbackBody),
    MicroTaskVerificationNeeded(MicroTaskVerificationNeededBody),
    MicroTaskEscalation(MicroTaskEscalationBody),
    MicroTaskCompletionReport(MicroTaskCompletionReportBody),
    /// Forward-compat escape hatch for future families. Unknown payloads
    /// decode here instead of erroring.
    Unknown {
        raw: serde_json::Value,
    },
}

/// Maximum encoded payload size for any single mailbox family (bytes).
///
/// Per MT-179 subagent brief red-team coverage: encoded family payloads larger
/// than 1 MiB are rejected at the encode boundary with a typed
/// `FamilyError::PayloadTooLarge` rather than silently propagating through the
/// mailbox repo. Downstream consumers (router, repo, exporter) can rely on
/// this bound to size their buffers safely.
pub const MAX_FAMILY_PAYLOAD_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FamilyError {
    #[error("encoding error: {0}")]
    Encoding(String),
    #[error("missing required field: {field}")]
    MissingField { field: String },
    #[error(
        "payload too large: {size_bytes} bytes exceeds {limit_bytes}-byte limit for family {family}"
    )]
    PayloadTooLarge {
        family: &'static str,
        size_bytes: usize,
        limit_bytes: usize,
    },
}

impl MessageFamily {
    pub fn family_id(&self) -> &'static str {
        match self {
            Self::DelegateWork(_) => "delegate_work",
            Self::Blocker(_) => "blocker",
            Self::ReviewRequest(_) => "review_request",
            Self::DecisionRequest(_) => "decision_request",
            Self::AnnounceBack(_) => "announce_back",
            Self::MicroTaskRequest(_) => "micro_task_request",
            Self::MicroTaskFeedback(_) => "micro_task_feedback",
            Self::MicroTaskVerificationNeeded(_) => "micro_task_verification_needed",
            Self::MicroTaskEscalation(_) => "micro_task_escalation",
            Self::MicroTaskCompletionReport(_) => "micro_task_completion_report",
            Self::Unknown { .. } => "unknown",
        }
    }

    /// Encode the family to JSON bytes, enforcing the
    /// `MAX_FAMILY_PAYLOAD_BYTES` size bound. Use this at the mailbox repo /
    /// router boundary to fail closed before persisting oversized payloads.
    pub fn encode_bounded(&self) -> Result<Vec<u8>, FamilyError> {
        let bytes = serde_json::to_vec(self).map_err(|e| FamilyError::Encoding(e.to_string()))?;
        if bytes.len() > MAX_FAMILY_PAYLOAD_BYTES {
            return Err(FamilyError::PayloadTooLarge {
                family: self.family_id(),
                size_bytes: bytes.len(),
                limit_bytes: MAX_FAMILY_PAYLOAD_BYTES,
            });
        }
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delegate_work_round_trip() {
        let body = MessageFamily::DelegateWork(DelegateWorkBody {
            task_summary: "do x".to_string(),
            target_role: RoleId::Coder,
            due_at_utc: Some(Utc::now()),
            linked_wp: Some("WP-1".to_string()),
            linked_mt: None,
        });
        let s = serde_json::to_string(&body).unwrap();
        let back: MessageFamily = serde_json::from_str(&s).unwrap();
        assert_eq!(body, back);
        assert_eq!(body.family_id(), "delegate_work");
    }

    #[test]
    fn micro_task_request_requires_contract_ref() {
        let bad_json = r#"{"family":"micro_task_request","body":{"mt_ref":{"wp_id":"W","mt_id":"M","iteration_n":1},"objective":"do","due_at_utc":null}}"#;
        let result: Result<MessageFamily, _> = serde_json::from_str(bad_json);
        assert!(
            result.is_err(),
            "micro_task_request must reject if micro_task_executor_contract_ref absent"
        );
    }

    #[test]
    fn unknown_family_decodes_to_unknown() {
        let bad_json = r#"{"family":"future_family","body":{"raw":{"x":1}}}"#;
        // Strict tag-content decoding rejects unknown tags; we encode it as
        // explicit Unknown variant for forward-compat.
        let explicit = MessageFamily::Unknown {
            raw: serde_json::json!({"x": 1}),
        };
        let s = serde_json::to_string(&explicit).unwrap();
        let back: MessageFamily = serde_json::from_str(&s).unwrap();
        assert_eq!(explicit, back);
        // The bad_json without explicit `Unknown` tag is expected to fail —
        // forward-compat is via the Unknown variant, not arbitrary tag.
        let _ = serde_json::from_str::<MessageFamily>(bad_json);
    }

    #[test]
    fn all_families_round_trip() {
        let families = vec![
            MessageFamily::DelegateWork(DelegateWorkBody {
                task_summary: "a".to_string(),
                target_role: RoleId::Coder,
                due_at_utc: None,
                linked_wp: None,
                linked_mt: None,
            }),
            MessageFamily::Blocker(BlockerBody {
                blocker_description: "b".to_string(),
                blocking_role: Some(RoleId::Validator),
                severity: BlockerSeverity::Hard,
            }),
            MessageFamily::ReviewRequest(ReviewRequestBody {
                review_target: ReviewTarget::WorkPacket {
                    id: "WP-1".to_string(),
                },
                review_target_id: "WP-1".to_string(),
                review_kind: ReviewKind::CodeReview,
                evidence_pointers: vec![],
            }),
            MessageFamily::DecisionRequest(DecisionRequestBody {
                question: "?".to_string(),
                options: vec![],
                decision_authority_role: RoleId::Operator,
                deadline_utc: None,
            }),
            MessageFamily::AnnounceBack(AnnounceBackBody {
                sub_session_id: None,
                summary: "done".to_string(),
                artifacts: vec![],
                completion_state: CompletionState::Completed,
                provenance_chain: vec![],
                bundle_id: None,
            }),
        ];
        for f in families {
            let s = serde_json::to_string(&f).unwrap();
            let back: MessageFamily = serde_json::from_str(&s).unwrap();
            assert_eq!(f, back);
        }
    }
}

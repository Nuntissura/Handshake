use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::kernel::{PromotionDecisionKind, PromotionGate};

use super::{
    AdapterCapabilities, IsolationStrength, ProcessHandle, WINDOWS_NATIVE_JAIL_ADAPTER_ID,
    WINDOWS_NATIVE_JAIL_BACKEND_APPROVED,
};

pub const SANDBOX_PROMOTION_VALIDATED_EVENT_FAMILY: &str = "FR-EVT-PROMOTION-SANDBOX-VALIDATED";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxValidationEvidence {
    pub process_handle: ProcessHandle,
    pub adapter_capabilities: AdapterCapabilities,
    pub validation_exit_code: i32,
    pub validation_stdout_artifact_id: String,
    pub validation_stderr_artifact_id: String,
    pub sandbox_runtime_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxPromotionRequest {
    pub evidence: SandboxValidationEvidence,
    pub candidate_artifact_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SandboxPromotionRejectReason {
    InsufficientSandboxIsolation {
        required_min: IsolationStrength,
        observed_filesystem: IsolationStrength,
        observed_network: IsolationStrength,
    },
    ValidationFailed {
        exit_code: i32,
    },
    MissingArtifactEvidence {
        field: String,
    },
    AdapterUnavailable {
        adapter_id: String,
        reason: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SandboxPromotionOutcome {
    Accepted,
    Rejected {
        reason: SandboxPromotionRejectReason,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxPromotionEventRow {
    pub event_family: String,
    pub process_handle_id: Uuid,
    pub sandbox_internal_id: String,
    pub adapter_id: String,
    pub candidate_artifact_id: String,
    pub validation_stdout_artifact_id: String,
    pub validation_stderr_artifact_id: String,
    pub sandbox_runtime_ms: u64,
    pub emitted_at_utc: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxPromotionDecision {
    pub promotion_decision_kind: PromotionDecisionKind,
    pub outcome: SandboxPromotionOutcome,
    pub event_row: Option<SandboxPromotionEventRow>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxPromotionProjectedState {
    pub candidate_artifact_id: String,
    pub promotion_decision_kind: PromotionDecisionKind,
    pub adapter_id: String,
    pub validation_stdout_artifact_id: String,
    pub validation_stderr_artifact_id: String,
}

pub fn build_promotion_request(
    evidence: SandboxValidationEvidence,
    candidate_artifact_id: impl Into<String>,
) -> SandboxPromotionRequest {
    SandboxPromotionRequest {
        evidence,
        candidate_artifact_id: candidate_artifact_id.into(),
    }
}

impl PromotionGate {
    pub fn decide_sandbox_validated(request: SandboxPromotionRequest) -> SandboxPromotionDecision {
        if request.candidate_artifact_id.trim().is_empty() {
            return rejected(SandboxPromotionRejectReason::MissingArtifactEvidence {
                field: "candidate_artifact_id".to_string(),
            });
        }
        if request
            .evidence
            .validation_stdout_artifact_id
            .trim()
            .is_empty()
        {
            return rejected(SandboxPromotionRejectReason::MissingArtifactEvidence {
                field: "validation_stdout_artifact_id".to_string(),
            });
        }
        if request
            .evidence
            .validation_stderr_artifact_id
            .trim()
            .is_empty()
        {
            return rejected(SandboxPromotionRejectReason::MissingArtifactEvidence {
                field: "validation_stderr_artifact_id".to_string(),
            });
        }
        if request.evidence.validation_exit_code != 0 {
            return rejected(SandboxPromotionRejectReason::ValidationFailed {
                exit_code: request.evidence.validation_exit_code,
            });
        }

        let capabilities = &request.evidence.adapter_capabilities;
        if !capabilities.runtime_available {
            return rejected(SandboxPromotionRejectReason::AdapterUnavailable {
                adapter_id: capabilities.adapter_id.as_str().to_string(),
                reason: "sandbox promotion evidence must come from runtime-available adapter capabilities".to_string(),
            });
        }
        if capabilities.adapter_id.as_str() == WINDOWS_NATIVE_JAIL_ADAPTER_ID
            && (!WINDOWS_NATIVE_JAIL_BACKEND_APPROVED || !capabilities.win32_native_fidelity)
        {
            return rejected(SandboxPromotionRejectReason::AdapterUnavailable {
                adapter_id: WINDOWS_NATIVE_JAIL_ADAPTER_ID.to_string(),
                reason: "windows_native_jail promotion evidence requires an approved MT-045 runtime backend, not target capability metadata".to_string(),
            });
        }
        if capabilities.adapter_id.as_str() == WINDOWS_NATIVE_JAIL_ADAPTER_ID
            && capabilities.filesystem_isolation_strength != IsolationStrength::VeryStrong
        {
            return rejected(SandboxPromotionRejectReason::InsufficientSandboxIsolation {
                required_min: IsolationStrength::VeryStrong,
                observed_filesystem: capabilities.filesystem_isolation_strength,
                observed_network: capabilities.network_isolation_strength,
            });
        }
        if capabilities.filesystem_isolation_strength == IsolationStrength::Weak
            || capabilities.network_isolation_strength == IsolationStrength::Weak
        {
            return rejected(SandboxPromotionRejectReason::InsufficientSandboxIsolation {
                required_min: IsolationStrength::Strong,
                observed_filesystem: capabilities.filesystem_isolation_strength,
                observed_network: capabilities.network_isolation_strength,
            });
        }

        SandboxPromotionDecision {
            promotion_decision_kind: PromotionDecisionKind::Approved,
            outcome: SandboxPromotionOutcome::Accepted,
            event_row: Some(SandboxPromotionEventRow {
                event_family: SANDBOX_PROMOTION_VALIDATED_EVENT_FAMILY.to_string(),
                process_handle_id: request.evidence.process_handle.id,
                sandbox_internal_id: request.evidence.process_handle.sandbox_internal_id,
                adapter_id: capabilities.adapter_id.as_str().to_string(),
                candidate_artifact_id: request.candidate_artifact_id,
                validation_stdout_artifact_id: request.evidence.validation_stdout_artifact_id,
                validation_stderr_artifact_id: request.evidence.validation_stderr_artifact_id,
                sandbox_runtime_ms: request.evidence.sandbox_runtime_ms,
                emitted_at_utc: Utc::now(),
            }),
        }
    }
}

pub fn replay_sandbox_promotion_events(
    events: impl IntoIterator<Item = SandboxPromotionEventRow>,
) -> BTreeMap<String, SandboxPromotionProjectedState> {
    let mut projected = BTreeMap::new();
    for event in events {
        if event.event_family != SANDBOX_PROMOTION_VALIDATED_EVENT_FAMILY {
            continue;
        }
        projected.insert(
            event.candidate_artifact_id.clone(),
            SandboxPromotionProjectedState {
                candidate_artifact_id: event.candidate_artifact_id,
                promotion_decision_kind: PromotionDecisionKind::Approved,
                adapter_id: event.adapter_id,
                validation_stdout_artifact_id: event.validation_stdout_artifact_id,
                validation_stderr_artifact_id: event.validation_stderr_artifact_id,
            },
        );
    }
    projected
}

fn rejected(reason: SandboxPromotionRejectReason) -> SandboxPromotionDecision {
    SandboxPromotionDecision {
        promotion_decision_kind: PromotionDecisionKind::Rejected,
        outcome: SandboxPromotionOutcome::Rejected { reason },
        event_row: None,
    }
}

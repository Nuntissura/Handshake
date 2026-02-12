//! Cloud escalation guard (policy + consent enforcement).
//!
//! Spec anchors:
//! - Handshake_Master_Spec_v02.125.md §11.1.7: Cloud escalation requires explicit human consent
//!   and ConsentReceipt/ProjectionPlan binding.
//! - Refinement RT-CLOUD-001: Cloud invocation without ConsentReceipt (when required) MUST hard-block;
//!   GovernanceMode LOCKED MUST deny cloud escalation.

use super::{CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelTier};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;

pub const ENV_GOVERNANCE_MODE: &str = "HANDSHAKE_GOVERNANCE_MODE";
pub const ENV_CLOUD_ESCALATION_ALLOWED: &str = "HANDSHAKE_CLOUD_ESCALATION_ALLOWED";
pub const ENV_CLOUD_PROJECTION_PLAN_JSON: &str = "HANDSHAKE_CLOUD_PROJECTION_PLAN_JSON";
pub const ENV_CLOUD_CONSENT_RECEIPT_JSON: &str = "HANDSHAKE_CLOUD_CONSENT_RECEIPT_JSON";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeGovernanceMode {
    /// Spec GovernanceMode LOCKED => cloud escalation MUST be denied.
    Locked,
    GovStrict,
    GovStandard,
    GovLight,
}

impl RuntimeGovernanceMode {
    fn parse(value: &str) -> Option<Self> {
        match value.trim().to_lowercase().as_str() {
            "locked" => Some(Self::Locked),
            "gov_strict" => Some(Self::GovStrict),
            "gov_standard" => Some(Self::GovStandard),
            "gov_light" => Some(Self::GovLight),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CloudEscalationPolicy {
    pub governance_mode: RuntimeGovernanceMode,
    pub cloud_escalation_allowed: bool,
}

impl CloudEscalationPolicy {
    /// Loads the policy from env vars (default-deny).
    pub fn from_env() -> Self {
        let governance_mode = std::env::var(ENV_GOVERNANCE_MODE)
            .ok()
            .and_then(|v| RuntimeGovernanceMode::parse(&v))
            .unwrap_or(RuntimeGovernanceMode::GovStandard);

        let cloud_escalation_allowed = std::env::var(ENV_CLOUD_ESCALATION_ALLOWED)
            .ok()
            .map(|v| matches!(v.trim().to_lowercase().as_str(), "1" | "true" | "yes" | "y"))
            .unwrap_or(false);

        Self {
            governance_mode,
            cloud_escalation_allowed,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectionPlanV0_4 {
    pub schema_version: String, // "hsk.projection_plan@0.4"
    pub projection_plan_id: String,
    pub include_artifact_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include_fields: Option<Vec<String>>,
    pub redactions_applied: Vec<String>,
    pub max_bytes: u32,
    pub payload_sha256: String,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wp_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mt_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentReceiptV0_4 {
    pub schema_version: String, // "hsk.consent_receipt@0.4"
    pub consent_receipt_id: String,
    pub projection_plan_id: String,
    pub payload_sha256: String,
    pub approved: bool,
    pub approved_at: String,
    pub user_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui_surface: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CloudConsentArtifacts {
    pub projection_plan: ProjectionPlanV0_4,
    pub consent_receipt: ConsentReceiptV0_4,
}

impl CloudConsentArtifacts {
    pub fn from_env() -> Result<Option<Self>, LlmError> {
        let plan_json = std::env::var(ENV_CLOUD_PROJECTION_PLAN_JSON).ok();
        let receipt_json = std::env::var(ENV_CLOUD_CONSENT_RECEIPT_JSON).ok();

        let (Some(plan_json), Some(receipt_json)) = (plan_json, receipt_json) else {
            return Ok(None);
        };

        let projection_plan: ProjectionPlanV0_4 =
            serde_json::from_str(&plan_json).map_err(|e| {
                LlmError::CloudConsentMismatch(format!("invalid ProjectionPlan JSON: {e}"))
            })?;
        let consent_receipt: ConsentReceiptV0_4 = serde_json::from_str(&receipt_json).map_err(|e| {
            LlmError::CloudConsentMismatch(format!("invalid ConsentReceipt JSON: {e}"))
        })?;

        Ok(Some(Self {
            projection_plan,
            consent_receipt,
        }))
    }

    pub fn validate_for_prompt(&self, prompt: &str) -> Result<(), LlmError> {
        if self.projection_plan.schema_version.trim() != "hsk.projection_plan@0.4" {
            return Err(LlmError::CloudConsentMismatch(
                "ProjectionPlan.schema_version must be hsk.projection_plan@0.4".to_string(),
            ));
        }
        if self.consent_receipt.schema_version.trim() != "hsk.consent_receipt@0.4" {
            return Err(LlmError::CloudConsentMismatch(
                "ConsentReceipt.schema_version must be hsk.consent_receipt@0.4".to_string(),
            ));
        }
        if !self.consent_receipt.approved {
            return Err(LlmError::CloudConsentMismatch(
                "ConsentReceipt.approved must be true".to_string(),
            ));
        }
        if self.consent_receipt.projection_plan_id != self.projection_plan.projection_plan_id {
            return Err(LlmError::CloudConsentMismatch(
                "ConsentReceipt.projection_plan_id must match ProjectionPlan.projection_plan_id"
                    .to_string(),
            ));
        }
        if self.consent_receipt.payload_sha256 != self.projection_plan.payload_sha256 {
            return Err(LlmError::CloudConsentMismatch(
                "ConsentReceipt.payload_sha256 must match ProjectionPlan.payload_sha256"
                    .to_string(),
            ));
        }

        // v1 payload model: sha256(prompt UTF-8 bytes)
        let mut h = Sha256::new();
        h.update(prompt.as_bytes());
        let computed = hex::encode(h.finalize());
        if computed != self.projection_plan.payload_sha256 {
            return Err(LlmError::CloudConsentMismatch(
                "payload_sha256 mismatch (v1: sha256(prompt bytes))".to_string(),
            ));
        }

        Ok(())
    }
}

/// Enforces cloud escalation policy + consent before allowing Cloud tier model calls.
pub struct CloudEscalationGuard {
    inner: Arc<dyn LlmClient>,
    policy: CloudEscalationPolicy,
    consent: Option<CloudConsentArtifacts>,
}

impl CloudEscalationGuard {
    pub fn new(
        inner: Arc<dyn LlmClient>,
        policy: CloudEscalationPolicy,
        consent: Option<CloudConsentArtifacts>,
    ) -> Self {
        Self {
            inner,
            policy,
            consent,
        }
    }

    pub fn from_env(inner: Arc<dyn LlmClient>) -> Result<Self, LlmError> {
        let policy = CloudEscalationPolicy::from_env();
        let consent = CloudConsentArtifacts::from_env()?;
        Ok(Self::new(inner, policy, consent))
    }
}

#[async_trait]
impl LlmClient for CloudEscalationGuard {
    async fn completion(&self, req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        if self.inner.profile().model_tier == ModelTier::Local {
            return self.inner.completion(req).await;
        }

        // Cloud tier enforcement (default-deny).
        if self.policy.governance_mode == RuntimeGovernanceMode::Locked {
            return Err(LlmError::GovernanceLocked);
        }
        if !self.policy.cloud_escalation_allowed {
            return Err(LlmError::CloudEscalationDenied);
        }

        let consent = self
            .consent
            .as_ref()
            .ok_or(LlmError::CloudConsentRequired)?;
        consent.validate_for_prompt(&req.prompt)?;

        self.inner.completion(req).await
    }

    async fn swap_model(&self, req: crate::workflows::ModelSwapRequestV0_4) -> Result<(), LlmError> {
        self.inner.swap_model(req).await
    }

    fn profile(&self) -> &super::ModelProfile {
        self.inner.profile()
    }
}


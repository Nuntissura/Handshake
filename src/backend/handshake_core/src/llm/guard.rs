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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::{CompletionResponse, ModelProfile, TokenUsage};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use uuid::Uuid;

    struct CountingClient {
        calls: AtomicUsize,
        profile: ModelProfile,
    }

    impl CountingClient {
        fn new(tier: ModelTier) -> Self {
            Self {
                calls: AtomicUsize::new(0),
                profile: ModelProfile::new("counting".to_string(), 1).with_tier(tier),
            }
        }

        fn calls(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl LlmClient for CountingClient {
        async fn completion(&self, _req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(CompletionResponse {
                text: "ok".to_string(),
                usage: TokenUsage {
                    prompt_tokens: 1,
                    completion_tokens: 1,
                    total_tokens: 2,
                },
                latency_ms: 1,
            })
        }

        fn profile(&self) -> &ModelProfile {
            &self.profile
        }
    }

    fn compute_prompt_sha256(prompt: &str) -> String {
        let mut h = Sha256::new();
        h.update(prompt.as_bytes());
        hex::encode(h.finalize())
    }

    fn valid_consent_for_prompt(prompt: &str) -> CloudConsentArtifacts {
        let payload_sha256 = compute_prompt_sha256(prompt);
        let projection_plan_id = "pp-1".to_string();
        CloudConsentArtifacts {
            projection_plan: ProjectionPlanV0_4 {
                schema_version: "hsk.projection_plan@0.4".to_string(),
                projection_plan_id: projection_plan_id.clone(),
                include_artifact_refs: Vec::new(),
                include_fields: None,
                redactions_applied: vec!["secrets".to_string()],
                max_bytes: 1024,
                payload_sha256: payload_sha256.clone(),
                created_at: "1970-01-01T00:00:00Z".to_string(),
                job_id: None,
                wp_id: None,
                mt_id: None,
            },
            consent_receipt: ConsentReceiptV0_4 {
                schema_version: "hsk.consent_receipt@0.4".to_string(),
                consent_receipt_id: "cr-1".to_string(),
                projection_plan_id,
                payload_sha256,
                approved: true,
                approved_at: "1970-01-01T00:00:00Z".to_string(),
                user_id: "user-1".to_string(),
                ui_surface: None,
                notes: None,
            },
        }
    }

    #[tokio::test]
    async fn locked_governance_denies_cloud_escalation_without_calling_inner() {
        let inner = Arc::new(CountingClient::new(ModelTier::Cloud));
        let guard = CloudEscalationGuard::new(
            inner.clone(),
            CloudEscalationPolicy {
                governance_mode: RuntimeGovernanceMode::Locked,
                cloud_escalation_allowed: true,
            },
            None,
        );

        let req = CompletionRequest::new(
            Uuid::new_v4(),
            "hello".to_string(),
            "cloud-model".to_string(),
        );
        let err = match guard.completion(req).await {
            Ok(_) => {
                assert!(false, "expected GovernanceLocked error");
                return;
            }
            Err(err) => err,
        };

        assert!(matches!(err, LlmError::GovernanceLocked));
        assert_eq!(inner.calls(), 0);
    }

    #[tokio::test]
    async fn policy_disallowed_denies_cloud_escalation_without_calling_inner() {
        let inner = Arc::new(CountingClient::new(ModelTier::Cloud));
        let guard = CloudEscalationGuard::new(
            inner.clone(),
            CloudEscalationPolicy {
                governance_mode: RuntimeGovernanceMode::GovStandard,
                cloud_escalation_allowed: false,
            },
            Some(valid_consent_for_prompt("hello")),
        );

        let req =
            CompletionRequest::new(Uuid::new_v4(), "hello".to_string(), "cloud-model".to_string());
        let err = match guard.completion(req).await {
            Ok(_) => {
                assert!(false, "expected CloudEscalationDenied error");
                return;
            }
            Err(err) => err,
        };

        assert!(matches!(err, LlmError::CloudEscalationDenied));
        assert_eq!(inner.calls(), 0);
    }

    #[tokio::test]
    async fn consent_missing_denies_cloud_escalation_without_calling_inner() {
        let inner = Arc::new(CountingClient::new(ModelTier::Cloud));
        let guard = CloudEscalationGuard::new(
            inner.clone(),
            CloudEscalationPolicy {
                governance_mode: RuntimeGovernanceMode::GovStandard,
                cloud_escalation_allowed: true,
            },
            None,
        );

        let req =
            CompletionRequest::new(Uuid::new_v4(), "hello".to_string(), "cloud-model".to_string());
        let err = match guard.completion(req).await {
            Ok(_) => {
                assert!(false, "expected CloudConsentRequired error");
                return;
            }
            Err(err) => err,
        };

        assert!(matches!(err, LlmError::CloudConsentRequired));
        assert_eq!(inner.calls(), 0);
    }

    #[tokio::test]
    async fn consent_mismatch_denies_cloud_escalation_without_calling_inner() {
        let inner = Arc::new(CountingClient::new(ModelTier::Cloud));
        let mut artifacts = valid_consent_for_prompt("hello");
        artifacts.projection_plan.payload_sha256 = "bad".to_string();

        let guard = CloudEscalationGuard::new(
            inner.clone(),
            CloudEscalationPolicy {
                governance_mode: RuntimeGovernanceMode::GovStandard,
                cloud_escalation_allowed: true,
            },
            Some(artifacts),
        );

        let req =
            CompletionRequest::new(Uuid::new_v4(), "hello".to_string(), "cloud-model".to_string());
        let err = match guard.completion(req).await {
            Ok(_) => {
                assert!(false, "expected CloudConsentMismatch error");
                return;
            }
            Err(err) => err,
        };

        assert!(matches!(err, LlmError::CloudConsentMismatch(_)));
        assert_eq!(inner.calls(), 0);
    }

    #[tokio::test]
    async fn valid_policy_and_consent_allows_cloud_call() {
        let inner = Arc::new(CountingClient::new(ModelTier::Cloud));
        let guard = CloudEscalationGuard::new(
            inner.clone(),
            CloudEscalationPolicy {
                governance_mode: RuntimeGovernanceMode::GovStandard,
                cloud_escalation_allowed: true,
            },
            Some(valid_consent_for_prompt("hello")),
        );

        let req =
            CompletionRequest::new(Uuid::new_v4(), "hello".to_string(), "cloud-model".to_string());
        let resp = match guard.completion(req).await {
            Ok(resp) => resp,
            Err(err) => {
                assert!(false, "expected completion Ok, got err: {err}");
                return;
            }
        };

        assert_eq!(resp.text, "ok");
        assert_eq!(inner.calls(), 1);
    }
}

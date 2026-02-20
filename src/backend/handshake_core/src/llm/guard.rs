//! Cloud escalation guard (policy + consent enforcement).
//!
//! Spec anchors:
//! - Handshake_Master_Spec_v02.133.md §11.1.7: Cloud escalation requires explicit human consent
//!   and ConsentReceipt/ProjectionPlan binding.
//! - Handshake_Master_Spec_v02.133.md CloudEscalationRequest schema.

use super::{
    openai_compat_canonical_request_bytes, sha256_hex, CompletionRequest, CompletionResponse,
    LlmClient, LlmError,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub const ENV_GOVERNANCE_MODE: &str = "HANDSHAKE_GOVERNANCE_MODE";

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
}

impl CloudEscalationPolicy {
    pub fn from_env() -> Self {
        let governance_mode = std::env::var(ENV_GOVERNANCE_MODE)
            .ok()
            .and_then(|v| RuntimeGovernanceMode::parse(&v))
            .unwrap_or(RuntimeGovernanceMode::GovStandard);
        Self { governance_mode }
    }
}

/// Cloud escalation consent artifacts [Spec §11.1.7.1].
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

/// Cloud escalation consent receipt [Spec §11.1.7.2].
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

/// Canonical cloud escalation request [Spec "CloudEscalationRequest Schema"].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudEscalationRequestV0_4 {
    pub schema_version: String, // "hsk.cloud_escalation@0.4"
    pub request_id: String,
    pub wp_id: String,
    pub mt_id: String,
    pub reason: String,
    pub local_attempts: u32,
    pub last_error_summary: String,
    pub requested_model_id: String,
    pub projection_plan_id: String,
    pub consent_receipt_id: String,
}

/// Bundle enforced at the outbound trust boundary (no raw payloads).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudEscalationBundleV0_4 {
    pub request: CloudEscalationRequestV0_4,
    pub projection_plan: ProjectionPlanV0_4,
    pub consent_receipt: ConsentReceiptV0_4,
}

impl CloudEscalationBundleV0_4 {
    pub fn validate_for_payload_sha256(
        &self,
        computed_payload_sha256: &str,
        resolved_model_id: &str,
    ) -> Result<(), LlmError> {
        if self.request.schema_version.trim() != "hsk.cloud_escalation@0.4" {
            return Err(LlmError::CloudConsentMismatch(
                "CloudEscalationRequest.schema_version must be hsk.cloud_escalation@0.4"
                    .to_string(),
            ));
        }
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
        if self.request.requested_model_id.trim() != resolved_model_id {
            return Err(LlmError::CloudConsentMismatch(
                "CloudEscalationRequest.requested_model_id must match resolved request model_id"
                    .to_string(),
            ));
        }

        if self.request.projection_plan_id != self.projection_plan.projection_plan_id {
            return Err(LlmError::CloudConsentMismatch(
                "CloudEscalationRequest.projection_plan_id must match ProjectionPlan.projection_plan_id"
                    .to_string(),
            ));
        }
        if self.request.consent_receipt_id != self.consent_receipt.consent_receipt_id {
            return Err(LlmError::CloudConsentMismatch(
                "CloudEscalationRequest.consent_receipt_id must match ConsentReceipt.consent_receipt_id"
                    .to_string(),
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
        if computed_payload_sha256 != self.projection_plan.payload_sha256 {
            return Err(LlmError::CloudConsentMismatch(
                "payload_sha256 mismatch (computed canonical OpenAI-compatible request bytes)"
                    .to_string(),
            ));
        }

        Ok(())
    }
}

/// Enforces ProjectionPlan + ConsentReceipt binding before allowing an outbound cloud invocation.
pub struct CloudEscalationGuard {
    inner: Arc<dyn LlmClient>,
    policy: CloudEscalationPolicy,
}

impl CloudEscalationGuard {
    pub fn new(inner: Arc<dyn LlmClient>, policy: CloudEscalationPolicy) -> Self {
        Self { inner, policy }
    }

    pub fn from_env(inner: Arc<dyn LlmClient>) -> Result<Self, LlmError> {
        let policy = CloudEscalationPolicy::from_env();
        Ok(Self::new(inner, policy))
    }
}

#[async_trait]
impl LlmClient for CloudEscalationGuard {
    async fn completion(&self, req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        // Enforcement is per-invocation: only requests explicitly marked as cloud escalation
        // require ProjectionPlan + ConsentReceipt binding.
        let Some(bundle) = req.cloud_escalation.as_ref() else {
            return self.inner.completion(req).await;
        };

        if self.policy.governance_mode == RuntimeGovernanceMode::Locked {
            return Err(LlmError::GovernanceLocked);
        }

        let resolved_model_id = if req.model_id.trim().is_empty() {
            self.inner.profile().model_id.clone()
        } else {
            req.model_id.clone()
        };

        let canonical_bytes =
            openai_compat_canonical_request_bytes(&req, resolved_model_id.as_str());
        let computed_sha256 = sha256_hex(&canonical_bytes);

        bundle.validate_for_payload_sha256(computed_sha256.as_str(), resolved_model_id.as_str())?;

        self.inner.completion(req).await
    }

    async fn swap_model(
        &self,
        req: crate::workflows::ModelSwapRequestV0_4,
    ) -> Result<(), LlmError> {
        self.inner.swap_model(req).await
    }

    fn profile(&self) -> &super::ModelProfile {
        self.inner.profile()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::{CompletionResponse, ModelProfile, ModelTier, TokenUsage};
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
        async fn completion(
            &self,
            _req: CompletionRequest,
        ) -> Result<CompletionResponse, LlmError> {
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

    fn valid_bundle_for_req(req: &CompletionRequest) -> CloudEscalationBundleV0_4 {
        let canonical_bytes = openai_compat_canonical_request_bytes(req, req.model_id.as_str());
        let payload_sha256 = sha256_hex(&canonical_bytes);
        let projection_plan_id = "pp-1".to_string();
        let consent_receipt_id = "cr-1".to_string();
        let request_id = "req-1".to_string();

        CloudEscalationBundleV0_4 {
            request: CloudEscalationRequestV0_4 {
                schema_version: "hsk.cloud_escalation@0.4".to_string(),
                request_id,
                wp_id: "WP-TEST".to_string(),
                mt_id: "MT-TEST".to_string(),
                reason: "test".to_string(),
                local_attempts: 2,
                last_error_summary: "local_failed".to_string(),
                requested_model_id: req.model_id.clone(),
                projection_plan_id: projection_plan_id.clone(),
                consent_receipt_id: consent_receipt_id.clone(),
            },
            projection_plan: ProjectionPlanV0_4 {
                schema_version: "hsk.projection_plan@0.4".to_string(),
                projection_plan_id: projection_plan_id.clone(),
                include_artifact_refs: Vec::new(),
                include_fields: None,
                redactions_applied: vec!["none".to_string()],
                max_bytes: canonical_bytes.len().min(u32::MAX as usize) as u32,
                payload_sha256: payload_sha256.clone(),
                created_at: "1970-01-01T00:00:00Z".to_string(),
                job_id: None,
                wp_id: Some("WP-TEST".to_string()),
                mt_id: Some("MT-TEST".to_string()),
            },
            consent_receipt: ConsentReceiptV0_4 {
                schema_version: "hsk.consent_receipt@0.4".to_string(),
                consent_receipt_id,
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
    async fn local_tier_passes_through_without_consent() {
        let inner = Arc::new(CountingClient::new(ModelTier::Local));
        let guard = CloudEscalationGuard::new(
            inner.clone(),
            CloudEscalationPolicy {
                governance_mode: RuntimeGovernanceMode::Locked,
            },
        );

        let req = CompletionRequest::new(
            Uuid::new_v4(),
            "hello".to_string(),
            "local-model".to_string(),
        );
        let resp = match guard.completion(req).await {
            Ok(resp) => resp,
            Err(err) => {
                assert!(false, "expected passthrough, got error: {err:?}");
                return;
            }
        };
        assert_eq!(resp.text, "ok");
        assert_eq!(inner.calls(), 1);
    }

    #[tokio::test]
    async fn locked_governance_denies_cloud_escalation_without_calling_inner() {
        let inner = Arc::new(CountingClient::new(ModelTier::Cloud));
        let guard = CloudEscalationGuard::new(
            inner.clone(),
            CloudEscalationPolicy {
                governance_mode: RuntimeGovernanceMode::Locked,
            },
        );

        let mut req = CompletionRequest::new(
            Uuid::new_v4(),
            "hello".to_string(),
            "cloud-model".to_string(),
        );
        req.cloud_escalation = Some(valid_bundle_for_req(&req));

        let err = guard
            .completion(req)
            .await
            .expect_err("expected locked denial");
        assert!(matches!(err, LlmError::GovernanceLocked));
        assert_eq!(inner.calls(), 0);
    }

    #[tokio::test]
    async fn unmarked_request_passes_through_without_enforcing_consent() {
        let inner = Arc::new(CountingClient::new(ModelTier::Cloud));
        let guard = CloudEscalationGuard::new(
            inner.clone(),
            CloudEscalationPolicy {
                governance_mode: RuntimeGovernanceMode::GovStandard,
            },
        );

        let req = CompletionRequest::new(
            Uuid::new_v4(),
            "hello".to_string(),
            "cloud-model".to_string(),
        );
        let resp = match guard.completion(req).await {
            Ok(resp) => resp,
            Err(err) => {
                assert!(false, "expected passthrough, got error: {err:?}");
                return;
            }
        };
        assert_eq!(resp.text, "ok");
        assert_eq!(inner.calls(), 1);
    }

    #[tokio::test]
    async fn consent_mismatch_denies_cloud_escalation_without_calling_inner() {
        let inner = Arc::new(CountingClient::new(ModelTier::Cloud));
        let guard = CloudEscalationGuard::new(
            inner.clone(),
            CloudEscalationPolicy {
                governance_mode: RuntimeGovernanceMode::GovStandard,
            },
        );

        let mut req = CompletionRequest::new(
            Uuid::new_v4(),
            "hello".to_string(),
            "cloud-model".to_string(),
        );
        let mut bundle = valid_bundle_for_req(&req);
        bundle.projection_plan.payload_sha256 = "bad".to_string();
        req.cloud_escalation = Some(bundle);

        let err = guard
            .completion(req)
            .await
            .expect_err("expected mismatch denial");
        assert!(matches!(err, LlmError::CloudConsentMismatch(_)));
        assert_eq!(inner.calls(), 0);
    }

    #[tokio::test]
    async fn valid_consent_allows_cloud_call() {
        let inner = Arc::new(CountingClient::new(ModelTier::Cloud));
        let guard = CloudEscalationGuard::new(
            inner.clone(),
            CloudEscalationPolicy {
                governance_mode: RuntimeGovernanceMode::GovStandard,
            },
        );

        let mut req = CompletionRequest::new(
            Uuid::new_v4(),
            "hello".to_string(),
            "cloud-model".to_string(),
        );
        req.cloud_escalation = Some(valid_bundle_for_req(&req));

        let resp = match guard.completion(req).await {
            Ok(resp) => resp,
            Err(err) => {
                assert!(false, "expected completion Ok, got error: {err:?}");
                return;
            }
        };
        assert_eq!(resp.text, "ok");
        assert_eq!(inner.calls(), 1);
    }
}

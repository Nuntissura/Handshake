//! Cloud escalation guard (policy + consent enforcement).
//!
//! Spec anchors:
//! - Handshake_Master_Spec_v02.133.md §11.1.7: Cloud escalation requires explicit human consent
//!   and ConsentReceipt/ProjectionPlan binding.
//! - Handshake_Master_Spec_v02.133.md CloudEscalationRequest schema.

use super::{
    openai_compat_canonical_request_bytes, sha256_hex, CompletionRequest, CompletionResponse,
    LlmClient, LlmError, ModelTier,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
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
    // Optional session binding fields (parallel sessions consent gate).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consent_scope: Option<ConsentScopeV0_4>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_ids: Option<Vec<String>>,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConsentScopeV0_4 {
    SingleCall,
    SessionScoped,
    WpScoped,
    BroadcastScoped,
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
    // Optional consent gate invariants (parallel sessions consent gate).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consent_scope: Option<ConsentScopeV0_4>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_ids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_from_utc: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_until_utc: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revoked_at_utc: Option<String>,
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
    // Optional session binding fields (parallel sessions consent gate).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consent_scope: Option<ConsentScopeV0_4>,
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
    fn parse_rfc3339_utc(label: &str, value: &str) -> Result<DateTime<Utc>, LlmError> {
        let parsed = DateTime::parse_from_rfc3339(value).map_err(|_| {
            LlmError::CloudConsentMismatch(format!("{label} must be RFC3339 (UTC preferred)"))
        })?;
        Ok(parsed.with_timezone(&Utc))
    }

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

        // Optional session binding + consent gate invariants (deny-by-default when session_id present).
        if let Some(revoked_at) = self
            .consent_receipt
            .revoked_at_utc
            .as_deref()
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
        {
            let _ = Self::parse_rfc3339_utc("ConsentReceipt.revoked_at_utc", revoked_at)?;
            return Err(LlmError::CloudConsentMismatch(
                "ConsentReceipt is revoked".to_string(),
            ));
        }

        let now = Utc::now();
        if let Some(valid_from) = self
            .consent_receipt
            .valid_from_utc
            .as_deref()
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
        {
            let start = Self::parse_rfc3339_utc("ConsentReceipt.valid_from_utc", valid_from)?;
            if now < start {
                return Err(LlmError::CloudConsentMismatch(
                    "ConsentReceipt not yet valid (valid_from_utc)".to_string(),
                ));
            }
        }
        if let Some(valid_until) = self
            .consent_receipt
            .valid_until_utc
            .as_deref()
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
        {
            let end = Self::parse_rfc3339_utc("ConsentReceipt.valid_until_utc", valid_until)?;
            if now > end {
                return Err(LlmError::CloudConsentMismatch(
                    "ConsentReceipt expired (valid_until_utc)".to_string(),
                ));
            }
        }

        let request_session_id = self
            .request
            .session_id
            .as_deref()
            .map(|v| v.trim())
            .filter(|v| !v.is_empty());
        if let Some(session_id) = request_session_id {
            let scope = self
                .consent_receipt
                .consent_scope
                .or(self.request.consent_scope)
                .ok_or_else(|| {
                    LlmError::CloudConsentMismatch(
                        "ConsentReceipt.consent_scope required when CloudEscalationRequest.session_id is present"
                            .to_string(),
                    )
                })?;
            let session_ids = self
                .consent_receipt
                .session_ids
                .as_ref()
                .ok_or_else(|| {
                    LlmError::CloudConsentMismatch(
                        "ConsentReceipt.session_ids required when CloudEscalationRequest.session_id is present"
                            .to_string(),
                    )
                })?;
            if !session_ids.iter().any(|v| v.trim() == session_id) {
                return Err(LlmError::CloudConsentMismatch(
                    "ConsentReceipt.session_ids must include CloudEscalationRequest.session_id"
                        .to_string(),
                ));
            }
            if matches!(scope, ConsentScopeV0_4::SessionScoped) && session_ids.len() != 1 {
                return Err(LlmError::CloudConsentMismatch(
                    "SESSION_SCOPED receipt must bind exactly one session_id".to_string(),
                ));
            }
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
        // Local models are trusted and not subject to cloud escalation consent enforcement.
        // Cloud tier invocations MUST include explicit consent artifacts.
        if self.inner.profile().model_tier == ModelTier::Local {
            return self.inner.completion(req).await;
        }

        if self.policy.governance_mode == RuntimeGovernanceMode::Locked {
            return Err(LlmError::GovernanceLocked);
        }

        let Some(bundle) = req.cloud_escalation.as_ref() else {
            return Err(LlmError::CloudConsentRequired);
        };

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
                session_id: None,
                consent_scope: None,
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
                consent_scope: None,
                session_ids: None,
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
                consent_scope: None,
                session_ids: None,
                valid_from_utc: None,
                valid_until_utc: None,
                revoked_at_utc: None,
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
    async fn cloud_tier_requires_consent_bundle() {
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
        let err = guard
            .completion(req)
            .await
            .expect_err("expected consent-required denial");
        assert!(matches!(err, LlmError::CloudConsentRequired));
        assert_eq!(inner.calls(), 0);
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

//! JobBoundaryRoutingGuard (§2.6.6.7.11.7)
//!
//! Ensures job routing invariants are maintained:
//! - policy_profile_id MUST NOT change mid-job
//! - model_tier MUST NOT change mid-job
//! - layer_scope MUST NOT change mid-job

use async_trait::async_trait;

use super::AceRuntimeValidator;
use crate::ace::{AceError, QueryPlan, RetrievalTrace};

/// Marker in trace warnings indicating policy changed mid-job
pub const POLICY_CHANGED_WARNING: &str = "boundary:policy_changed";

/// Marker in trace warnings indicating model tier changed mid-job
pub const MODEL_TIER_CHANGED_WARNING: &str = "boundary:model_tier_changed";

/// Marker in trace warnings indicating layer scope changed mid-job
pub const LAYER_SCOPE_CHANGED_WARNING: &str = "boundary:layer_scope_changed";

/// JobBoundaryRoutingGuard ensures routing invariants hold for job duration.
///
/// Per §2.6.6.7.11.7:
/// - policy_profile_id MUST NOT change since initial AIJob creation
/// - model_tier MUST NOT change since initial AIJob creation
/// - layer_scope MUST NOT change since initial AIJob creation
pub struct JobBoundaryRoutingGuard;

impl JobBoundaryRoutingGuard {
    /// Check if trace has policy changed warning
    fn has_policy_changed(trace: &RetrievalTrace) -> Option<(String, String)> {
        trace
            .warnings
            .iter()
            .find(|w| w.starts_with(POLICY_CHANGED_WARNING))
            .and_then(|w| Self::extract_change_info(w, POLICY_CHANGED_WARNING))
    }

    /// Check if trace has model tier changed warning
    fn has_model_tier_changed(trace: &RetrievalTrace) -> Option<(String, String)> {
        trace
            .warnings
            .iter()
            .find(|w| w.starts_with(MODEL_TIER_CHANGED_WARNING))
            .and_then(|w| Self::extract_change_info(w, MODEL_TIER_CHANGED_WARNING))
    }

    /// Check if trace has layer scope changed warning
    fn has_layer_scope_changed(trace: &RetrievalTrace) -> Option<(String, String)> {
        trace
            .warnings
            .iter()
            .find(|w| w.starts_with(LAYER_SCOPE_CHANGED_WARNING))
            .and_then(|w| Self::extract_change_info(w, LAYER_SCOPE_CHANGED_WARNING))
    }

    /// Extract change info (original, current) from warning
    /// Expected format: "prefix:original:current"
    fn extract_change_info(warning: &str, prefix: &str) -> Option<(String, String)> {
        let rest = warning.strip_prefix(prefix)?;
        let parts: Vec<&str> = rest.split(':').filter(|s| !s.is_empty()).collect();
        if parts.len() >= 2 {
            Some((parts[0].to_string(), parts[1].to_string()))
        } else {
            Some(("unknown".to_string(), "unknown".to_string()))
        }
    }
}

#[async_trait]
impl AceRuntimeValidator for JobBoundaryRoutingGuard {
    fn name(&self) -> &str {
        "job_boundary_routing_guard"
    }

    async fn validate_plan(&self, _plan: &QueryPlan) -> Result<(), AceError> {
        // Boundary validation is at trace time when we can compare to initial state
        Ok(())
    }

    async fn validate_trace(&self, trace: &RetrievalTrace) -> Result<(), AceError> {
        // Check for policy change
        if let Some((original, current)) = Self::has_policy_changed(trace) {
            return Err(AceError::JobBoundaryViolation {
                field: "policy_profile_id".to_string(),
                original,
                current,
            });
        }

        // Check for model tier change
        if let Some((original, current)) = Self::has_model_tier_changed(trace) {
            return Err(AceError::JobBoundaryViolation {
                field: "model_tier".to_string(),
                original,
                current,
            });
        }

        // Check for layer scope change
        if let Some((original, current)) = Self::has_layer_scope_changed(trace) {
            return Err(AceError::JobBoundaryViolation {
                field: "layer_scope".to_string(),
                original,
                current,
            });
        }

        // Check for boundary errors
        let boundary_errors: Vec<_> = trace
            .errors
            .iter()
            .filter(|e| {
                e.contains("boundary")
                    || e.contains("policy_profile")
                    || e.contains("model_tier")
                    || e.contains("layer_scope")
            })
            .collect();

        if !boundary_errors.is_empty() {
            return Err(AceError::JobBoundaryViolation {
                field: "unknown".to_string(),
                original: "unknown".to_string(),
                current: format!("errors: {:?}", boundary_errors),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ace::QueryKind;

    /// T-ACE-VAL-007: JobBoundaryRoutingGuard detects policy changes
    #[tokio::test]
    async fn test_boundary_guard_policy_changed() {
        let guard = JobBoundaryRoutingGuard;
        let plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        let mut trace = RetrievalTrace::new(&plan);

        // Add policy changed warning
        trace.warnings.push(format!(
            "{}:safe_default:permissive",
            POLICY_CHANGED_WARNING
        ));

        let result = guard.validate_trace(&trace).await;
        assert!(matches!(
            result,
            Err(AceError::JobBoundaryViolation { field, original, current })
                if field == "policy_profile_id"
                && original == "safe_default"
                && current == "permissive"
        ));
    }

    /// T-ACE-VAL-007: JobBoundaryRoutingGuard detects model tier changes
    #[tokio::test]
    async fn test_boundary_guard_model_tier_changed() {
        let guard = JobBoundaryRoutingGuard;
        let plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        let mut trace = RetrievalTrace::new(&plan);

        // Add model tier changed warning
        trace
            .warnings
            .push(format!("{}:local:cloud", MODEL_TIER_CHANGED_WARNING));

        let result = guard.validate_trace(&trace).await;
        assert!(matches!(
            result,
            Err(AceError::JobBoundaryViolation { field, .. }) if field == "model_tier"
        ));
    }

    /// T-ACE-VAL-007: JobBoundaryRoutingGuard detects layer scope changes
    #[tokio::test]
    async fn test_boundary_guard_layer_scope_changed() {
        let guard = JobBoundaryRoutingGuard;
        let plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        let mut trace = RetrievalTrace::new(&plan);

        // Add layer scope changed warning
        trace
            .warnings
            .push(format!("{}:workspace:global", LAYER_SCOPE_CHANGED_WARNING));

        let result = guard.validate_trace(&trace).await;
        assert!(matches!(
            result,
            Err(AceError::JobBoundaryViolation { field, .. }) if field == "layer_scope"
        ));
    }

    #[tokio::test]
    async fn test_boundary_guard_valid_trace() {
        let guard = JobBoundaryRoutingGuard;
        let plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        let trace = RetrievalTrace::new(&plan);

        // No warnings -> OK
        assert!(guard.validate_trace(&trace).await.is_ok());
    }

    #[tokio::test]
    async fn test_boundary_guard_error_detection() {
        let guard = JobBoundaryRoutingGuard;
        let plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        let mut trace = RetrievalTrace::new(&plan);

        // Add boundary error
        trace
            .errors
            .push("boundary violation: model_tier escalation detected".to_string());

        let result = guard.validate_trace(&trace).await;
        assert!(matches!(result, Err(AceError::JobBoundaryViolation { .. })));
    }
}

//! ArtifactHandleOnlyGuard (§2.6.6.7.11.2)
//!
//! Enforces inline delta size limits:
//! - MUST enforce tool_delta_inline_char_limit (default: 2000)
//! - Reject any tool_delta exceeding limit unless offloaded to ArtifactHandle

use async_trait::async_trait;

use super::AceRuntimeValidator;
use crate::ace::{AceError, QueryPlan, RetrievalTrace};

/// Default inline character limit for tool deltas
pub const DEFAULT_TOOL_DELTA_INLINE_CHAR_LIMIT: u32 = 2000;

/// Marker in trace warnings indicating inline delta exceeded
pub const INLINE_DELTA_EXCEEDED_WARNING: &str = "artifact:inline_delta_exceeded";

/// ArtifactHandleOnlyGuard enforces blob-free prompts.
///
/// Per §2.6.6.7.11.2:
/// - tool_delta_inline_char_limit is enforced (default: 2000)
/// - Oversized deltas MUST be offloaded to ArtifactHandle
pub struct ArtifactHandleOnlyGuard;

impl ArtifactHandleOnlyGuard {
    /// Check if trace contains inline delta exceeded warnings
    fn has_inline_exceeded_warnings(trace: &RetrievalTrace) -> bool {
        trace
            .warnings
            .iter()
            .any(|w| w.starts_with(INLINE_DELTA_EXCEEDED_WARNING))
    }

    /// Extract size info from warning if present
    fn extract_exceeded_size(trace: &RetrievalTrace) -> Option<(u32, u32)> {
        for warning in &trace.warnings {
            if let Some(rest) = warning.strip_prefix(INLINE_DELTA_EXCEEDED_WARNING) {
                // Expected format: ":actual:limit"
                let parts: Vec<&str> = rest.split(':').filter(|s| !s.is_empty()).collect();
                if parts.len() >= 2 {
                    if let (Ok(actual), Ok(limit)) = (parts[0].parse(), parts[1].parse()) {
                        return Some((actual, limit));
                    }
                }
            }
        }
        None
    }
}

#[async_trait]
impl AceRuntimeValidator for ArtifactHandleOnlyGuard {
    fn name(&self) -> &str {
        "artifact_handle_only_guard"
    }

    async fn validate_plan(&self, plan: &QueryPlan) -> Result<(), AceError> {
        // Validate that tool_delta_inline_char_limit is reasonable
        if plan.budgets.tool_delta_inline_char_limit == 0 {
            return Err(AceError::ValidationFailed {
                message: "tool_delta_inline_char_limit must be > 0".to_string(),
            });
        }
        Ok(())
    }

    async fn validate_trace(&self, trace: &RetrievalTrace) -> Result<(), AceError> {
        // Check for inline delta exceeded warnings
        if Self::has_inline_exceeded_warnings(trace) {
            let (actual, limit) = Self::extract_exceeded_size(trace)
                .unwrap_or((0, trace.budgets_applied.tool_delta_inline_char_limit));

            return Err(AceError::InlineDeltaExceeded { actual, limit });
        }

        // Check for artifact errors
        let artifact_errors: Vec<_> = trace
            .errors
            .iter()
            .filter(|e| {
                e.contains("inline_delta")
                    || e.contains("tool_delta")
                    || e.contains("ArtifactHandle")
            })
            .collect();

        if !artifact_errors.is_empty() {
            return Err(AceError::ValidationFailed {
                message: format!("Artifact handling errors: {:?}", artifact_errors),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ace::QueryKind;

    /// T-ACE-VAL-002: ArtifactHandleOnlyGuard enforces inline limits
    #[tokio::test]
    async fn test_artifact_guard_inline_exceeded() {
        let guard = ArtifactHandleOnlyGuard;
        let plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        let mut trace = RetrievalTrace::new(&plan);

        // Add inline exceeded warning with size info
        trace
            .warnings
            .push(format!("{}:3500:2000", INLINE_DELTA_EXCEEDED_WARNING));

        let result = guard.validate_trace(&trace).await;
        assert!(matches!(
            result,
            Err(AceError::InlineDeltaExceeded {
                actual: 3500,
                limit: 2000
            })
        ));
    }

    #[tokio::test]
    async fn test_artifact_guard_valid_trace() {
        let guard = ArtifactHandleOnlyGuard;
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
    async fn test_artifact_guard_plan_validation() {
        let guard = ArtifactHandleOnlyGuard;

        // Valid plan
        let plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        assert!(guard.validate_plan(&plan).await.is_ok());

        // Zero limit should fail
        let mut invalid_plan = plan.clone();
        invalid_plan.budgets.tool_delta_inline_char_limit = 0;
        let result = guard.validate_plan(&invalid_plan).await;
        assert!(matches!(result, Err(AceError::ValidationFailed { .. })));
    }

    #[tokio::test]
    async fn test_artifact_guard_error_detection() {
        let guard = ArtifactHandleOnlyGuard;
        let plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        let mut trace = RetrievalTrace::new(&plan);

        // Add artifact error
        trace
            .errors
            .push("inline_delta: failed to offload to ArtifactHandle".to_string());

        let result = guard.validate_trace(&trace).await;
        assert!(matches!(result, Err(AceError::ValidationFailed { .. })));
    }
}

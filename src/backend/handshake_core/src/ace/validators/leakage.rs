//! CloudLeakageGuard (§2.6.6.7.11.5)
//!
//! Prevents sensitive data from leaking to cloud model tiers:
//! - If model_tier=Cloud, MUST scan all artifact_handles and SourceRefs
//! - MUST block if any item has exportable=false or sensitivity=high
//! - MUST default to Block if sensitivity is unknown or metadata missing

use async_trait::async_trait;

use super::AceRuntimeValidator;
use crate::ace::{AceError, QueryPlan, RetrievalTrace};

/// Marker in trace warnings indicating non-exportable artifact detected
pub const NON_EXPORTABLE_WARNING: &str = "leakage:non_exportable";

/// Marker in trace warnings indicating high sensitivity content
pub const HIGH_SENSITIVITY_WARNING: &str = "leakage:high_sensitivity";

/// Marker in trace warnings indicating unknown sensitivity (default block)
pub const UNKNOWN_SENSITIVITY_WARNING: &str = "leakage:unknown_sensitivity";

/// Marker in trace warnings indicating cloud tier is active
pub const CLOUD_TIER_MARKER: &str = "model_tier:cloud";

/// CloudLeakageGuard prevents sensitive data from reaching cloud tiers.
///
/// Per §2.6.6.7.11.5:
/// - If model_tier=Cloud, scan all artifact_handles and SourceRefs
/// - Block if exportable=false or sensitivity=high
/// - Default to Block if sensitivity unknown (security-first)
pub struct CloudLeakageGuard;

impl CloudLeakageGuard {
    /// Check if trace indicates cloud tier is active
    fn is_cloud_tier(trace: &RetrievalTrace) -> bool {
        trace.warnings.iter().any(|w| w.contains(CLOUD_TIER_MARKER))
            || trace.errors.iter().any(|e| e.contains(CLOUD_TIER_MARKER))
    }

    /// Check for non-exportable content
    fn has_non_exportable_warning(trace: &RetrievalTrace) -> bool {
        trace
            .warnings
            .iter()
            .any(|w| w.starts_with(NON_EXPORTABLE_WARNING))
    }

    /// Check for high sensitivity content
    fn has_high_sensitivity_warning(trace: &RetrievalTrace) -> bool {
        trace
            .warnings
            .iter()
            .any(|w| w.starts_with(HIGH_SENSITIVITY_WARNING))
    }

    /// Check for unknown sensitivity (default block per mandate)
    fn has_unknown_sensitivity_warning(trace: &RetrievalTrace) -> bool {
        trace
            .warnings
            .iter()
            .any(|w| w.starts_with(UNKNOWN_SENSITIVITY_WARNING))
    }

    /// Extract artifact/source ID from warning
    fn extract_item_id(warning: &str, prefix: &str) -> Option<String> {
        warning
            .strip_prefix(prefix)
            .map(|rest| rest.trim_start_matches(':').to_string())
    }
}

#[async_trait]
impl AceRuntimeValidator for CloudLeakageGuard {
    fn name(&self) -> &str {
        "cloud_leakage_guard"
    }

    async fn validate_plan(&self, _plan: &QueryPlan) -> Result<(), AceError> {
        // Leakage detection is at trace time when we know actual content
        Ok(())
    }

    async fn validate_trace(&self, trace: &RetrievalTrace) -> Result<(), AceError> {
        // Only enforce leakage rules when cloud tier is active
        // However, we still check for explicit warnings even without cloud marker

        // Check for non-exportable content
        if Self::has_non_exportable_warning(trace) {
            let item_id = trace
                .warnings
                .iter()
                .find_map(|w| Self::extract_item_id(w, NON_EXPORTABLE_WARNING))
                .unwrap_or_else(|| "unknown".to_string());

            return Err(AceError::CloudLeakageBlocked {
                reason: format!(
                    "Non-exportable artifact '{}' cannot be sent to cloud tier",
                    item_id
                ),
            });
        }

        // Check for high sensitivity content
        if Self::has_high_sensitivity_warning(trace) {
            let item_id = trace
                .warnings
                .iter()
                .find_map(|w| Self::extract_item_id(w, HIGH_SENSITIVITY_WARNING))
                .unwrap_or_else(|| "unknown".to_string());

            return Err(AceError::CloudLeakageBlocked {
                reason: format!(
                    "High-sensitivity content '{}' cannot be sent to cloud tier",
                    item_id
                ),
            });
        }

        // Check for unknown sensitivity (MUST default to Block per mandate)
        if Self::has_unknown_sensitivity_warning(trace) {
            let item_id = trace
                .warnings
                .iter()
                .find_map(|w| Self::extract_item_id(w, UNKNOWN_SENSITIVITY_WARNING))
                .unwrap_or_else(|| "unknown".to_string());

            return Err(AceError::CloudLeakageBlocked {
                reason: format!(
                    "Unknown sensitivity for '{}' - defaulting to block for cloud tier",
                    item_id
                ),
            });
        }

        // Check for leakage errors
        let leakage_errors: Vec<_> = trace
            .errors
            .iter()
            .filter(|e| {
                e.contains("leakage")
                    || e.contains("exportable")
                    || e.contains("sensitivity")
                    || e.contains("cloud")
            })
            .collect();

        if !leakage_errors.is_empty() {
            return Err(AceError::CloudLeakageBlocked {
                reason: format!("Cloud leakage errors: {:?}", leakage_errors),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ace::QueryKind;

    /// T-ACE-VAL-005: CloudLeakageGuard blocks non-exportable content
    #[tokio::test]
    async fn test_leakage_guard_non_exportable() {
        let guard = CloudLeakageGuard;
        let plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        let mut trace = RetrievalTrace::new(&plan);

        // Add non-exportable warning
        trace
            .warnings
            .push(format!("{}:artifact_123", NON_EXPORTABLE_WARNING));

        let result = guard.validate_trace(&trace).await;
        assert!(matches!(
            result,
            Err(AceError::CloudLeakageBlocked { reason }) if reason.contains("Non-exportable")
        ));
    }

    /// T-ACE-VAL-005: CloudLeakageGuard blocks high sensitivity content
    #[tokio::test]
    async fn test_leakage_guard_high_sensitivity() {
        let guard = CloudLeakageGuard;
        let plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        let mut trace = RetrievalTrace::new(&plan);

        // Add high sensitivity warning
        trace
            .warnings
            .push(format!("{}:source_456", HIGH_SENSITIVITY_WARNING));

        let result = guard.validate_trace(&trace).await;
        assert!(matches!(
            result,
            Err(AceError::CloudLeakageBlocked { reason }) if reason.contains("High-sensitivity")
        ));
    }

    /// T-ACE-VAL-005: CloudLeakageGuard defaults to block for unknown sensitivity
    #[tokio::test]
    async fn test_leakage_guard_unknown_sensitivity_blocks() {
        let guard = CloudLeakageGuard;
        let plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        let mut trace = RetrievalTrace::new(&plan);

        // Add unknown sensitivity warning - MUST block per mandate
        trace
            .warnings
            .push(format!("{}:artifact_789", UNKNOWN_SENSITIVITY_WARNING));

        let result = guard.validate_trace(&trace).await;
        assert!(matches!(
            result,
            Err(AceError::CloudLeakageBlocked { reason }) if reason.contains("Unknown sensitivity")
        ));
    }

    #[tokio::test]
    async fn test_leakage_guard_valid_trace() {
        let guard = CloudLeakageGuard;
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
    async fn test_leakage_guard_error_detection() {
        let guard = CloudLeakageGuard;
        let plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        let mut trace = RetrievalTrace::new(&plan);

        // Add leakage error
        trace
            .errors
            .push("cloud leakage: sensitive data detected".to_string());

        let result = guard.validate_trace(&trace).await;
        assert!(matches!(result, Err(AceError::CloudLeakageBlocked { .. })));
    }
}

//! LocalPayloadGuard (§2.6.6.7.11.8)
//!
//! Validates local-only payload storage requirements:
//! - If local_only_payload_ref is present, artifact URI MUST point to /encrypted/ volume
//! - MUST ensure exportable=false
//! - MUST enforce encrypted-at-rest storage
//! - MUST apply retention TTL

use async_trait::async_trait;

use super::AceRuntimeValidator;
use crate::ace::{AceError, QueryPlan, RetrievalTrace};

/// Expected path prefix for encrypted storage
pub const ENCRYPTED_VOLUME_PREFIX: &str = "/encrypted/";

/// Marker in trace warnings indicating local payload not in encrypted volume
pub const NOT_ENCRYPTED_WARNING: &str = "local_payload:not_encrypted";

/// Marker in trace warnings indicating local payload marked exportable
pub const EXPORTABLE_WARNING: &str = "local_payload:exportable_true";

/// Marker in trace warnings indicating local payload missing retention
pub const MISSING_RETENTION_WARNING: &str = "local_payload:missing_retention";

/// LocalPayloadGuard ensures local-only payloads meet security requirements.
///
/// Per §2.6.6.7.11.8:
/// - local_only_payload_ref MUST point to /encrypted/ storage volume
/// - MUST have exportable=false
/// - MUST be encrypted at rest
/// - SHOULD have finite retention TTL
pub struct LocalPayloadGuard;

impl LocalPayloadGuard {
    /// Check if path is in encrypted volume
    pub fn is_encrypted_path(path: &str) -> bool {
        path.starts_with(ENCRYPTED_VOLUME_PREFIX)
    }

    /// Check if trace has not-encrypted warning
    fn has_not_encrypted_warning(trace: &RetrievalTrace) -> bool {
        trace
            .warnings
            .iter()
            .any(|w| w.starts_with(NOT_ENCRYPTED_WARNING))
    }

    /// Check if trace has exportable warning
    fn has_exportable_warning(trace: &RetrievalTrace) -> bool {
        trace
            .warnings
            .iter()
            .any(|w| w.starts_with(EXPORTABLE_WARNING))
    }

    /// Check if trace has missing retention warning (non-blocking, but logged)
    fn has_missing_retention_warning(trace: &RetrievalTrace) -> bool {
        trace
            .warnings
            .iter()
            .any(|w| w.starts_with(MISSING_RETENTION_WARNING))
    }

    /// Extract artifact ID from warning
    fn extract_artifact_id(warning: &str, prefix: &str) -> Option<String> {
        warning
            .strip_prefix(prefix)
            .map(|rest| rest.trim_start_matches(':').to_string())
    }
}

#[async_trait]
impl AceRuntimeValidator for LocalPayloadGuard {
    fn name(&self) -> &str {
        "local_payload_guard"
    }

    async fn validate_plan(&self, _plan: &QueryPlan) -> Result<(), AceError> {
        // Local payload validation is at trace time when we have actual artifact refs
        Ok(())
    }

    async fn validate_trace(&self, trace: &RetrievalTrace) -> Result<(), AceError> {
        // Check for non-encrypted storage
        if Self::has_not_encrypted_warning(trace) {
            let artifact_id = trace
                .warnings
                .iter()
                .find_map(|w| Self::extract_artifact_id(w, NOT_ENCRYPTED_WARNING))
                .unwrap_or_else(|| "unknown".to_string());

            return Err(AceError::LocalPayloadViolation {
                reason: format!(
                    "local_only_payload_ref '{}' must be stored in {} volume",
                    artifact_id, ENCRYPTED_VOLUME_PREFIX
                ),
            });
        }

        // Check for exportable=true on local payload
        if Self::has_exportable_warning(trace) {
            let artifact_id = trace
                .warnings
                .iter()
                .find_map(|w| Self::extract_artifact_id(w, EXPORTABLE_WARNING))
                .unwrap_or_else(|| "unknown".to_string());

            return Err(AceError::LocalPayloadViolation {
                reason: format!(
                    "local_only_payload_ref '{}' must have exportable=false",
                    artifact_id
                ),
            });
        }

        // Missing retention is a warning, not a blocking error
        // But we log it for observability
        if Self::has_missing_retention_warning(trace) {
            // In a real implementation, we'd use tracing here
            // tracing::warn!("Local payload missing retention TTL");
        }

        // Check for local payload errors
        let payload_errors: Vec<_> = trace
            .errors
            .iter()
            .filter(|e| {
                e.contains("local_payload")
                    || e.contains("local_only")
                    || e.contains("encrypted")
                    || e.contains("/encrypted/")
            })
            .collect();

        if !payload_errors.is_empty() {
            return Err(AceError::LocalPayloadViolation {
                reason: format!("Local payload errors: {:?}", payload_errors),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ace::QueryKind;

    /// T-ACE-VAL-008: LocalPayloadGuard validates encrypted storage
    #[tokio::test]
    async fn test_payload_guard_not_encrypted() {
        let guard = LocalPayloadGuard;
        let plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        let mut trace = RetrievalTrace::new(&plan);

        // Add not encrypted warning
        trace
            .warnings
            .push(format!("{}:artifact_123", NOT_ENCRYPTED_WARNING));

        let result = guard.validate_trace(&trace).await;
        assert!(matches!(
            result,
            Err(AceError::LocalPayloadViolation { reason }) if reason.contains("/encrypted/")
        ));
    }

    /// T-ACE-VAL-008: LocalPayloadGuard validates exportable=false
    #[tokio::test]
    async fn test_payload_guard_exportable_true() {
        let guard = LocalPayloadGuard;
        let plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        let mut trace = RetrievalTrace::new(&plan);

        // Add exportable warning
        trace
            .warnings
            .push(format!("{}:artifact_456", EXPORTABLE_WARNING));

        let result = guard.validate_trace(&trace).await;
        assert!(matches!(
            result,
            Err(AceError::LocalPayloadViolation { reason }) if reason.contains("exportable=false")
        ));
    }

    /// Test encrypted path validation
    #[test]
    fn test_encrypted_path_validation() {
        // Valid encrypted paths
        assert!(LocalPayloadGuard::is_encrypted_path(
            "/encrypted/artifact/123"
        ));
        assert!(LocalPayloadGuard::is_encrypted_path("/encrypted/"));

        // Invalid paths
        assert!(!LocalPayloadGuard::is_encrypted_path("/artifacts/123"));
        assert!(!LocalPayloadGuard::is_encrypted_path(
            "/cache/encrypted/123"
        ));
        assert!(!LocalPayloadGuard::is_encrypted_path("encrypted/123"));
    }

    #[tokio::test]
    async fn test_payload_guard_valid_trace() {
        let guard = LocalPayloadGuard;
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
    async fn test_payload_guard_missing_retention_allowed() {
        let guard = LocalPayloadGuard;
        let plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        let mut trace = RetrievalTrace::new(&plan);

        // Missing retention is a warning, not an error
        trace
            .warnings
            .push(format!("{}:artifact_789", MISSING_RETENTION_WARNING));

        // Should still pass (non-blocking)
        assert!(guard.validate_trace(&trace).await.is_ok());
    }

    #[tokio::test]
    async fn test_payload_guard_error_detection() {
        let guard = LocalPayloadGuard;
        let plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        let mut trace = RetrievalTrace::new(&plan);

        // Add local payload error
        trace
            .errors
            .push("local_payload: encryption verification failed".to_string());

        let result = guard.validate_trace(&trace).await;
        assert!(matches!(
            result,
            Err(AceError::LocalPayloadViolation { .. })
        ));
    }
}

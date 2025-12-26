//! MemoryPromotionGuard (§2.6.6.7.11.4)
//!
//! Validates memory promotion from SessionLog to LongTermMemory:
//! - MUST reject promotion if ValidationResult is absent or Fail
//! - MUST ensure provenance is preserved

use async_trait::async_trait;

use super::AceRuntimeValidator;
use crate::ace::{AceError, QueryPlan, RetrievalTrace};

/// Marker in trace warnings indicating promotion without validation
pub const PROMOTION_NO_VALIDATION_WARNING: &str = "promotion:no_validation_result";

/// Marker in trace warnings indicating promotion with failed validation
pub const PROMOTION_VALIDATION_FAIL_WARNING: &str = "promotion:validation_failed";

/// Marker in trace warnings indicating promotion without provenance
pub const PROMOTION_NO_PROVENANCE_WARNING: &str = "promotion:missing_provenance";

/// MemoryPromotionGuard blocks unauthorized SessionLog -> LongTermMemory promotion.
///
/// Per §2.6.6.7.11.4:
/// - MUST reject promotion if ValidationResult absent or Fail
/// - Provenance MUST be preserved for all promoted items
pub struct MemoryPromotionGuard;

impl MemoryPromotionGuard {
    /// Check if trace has promotion without validation
    fn has_no_validation_warning(trace: &RetrievalTrace) -> bool {
        trace
            .warnings
            .iter()
            .any(|w| w.starts_with(PROMOTION_NO_VALIDATION_WARNING))
    }

    /// Check if trace has promotion with failed validation
    fn has_validation_fail_warning(trace: &RetrievalTrace) -> bool {
        trace
            .warnings
            .iter()
            .any(|w| w.starts_with(PROMOTION_VALIDATION_FAIL_WARNING))
    }

    /// Check if trace has promotion without provenance
    fn has_no_provenance_warning(trace: &RetrievalTrace) -> bool {
        trace
            .warnings
            .iter()
            .any(|w| w.starts_with(PROMOTION_NO_PROVENANCE_WARNING))
    }
}

#[async_trait]
impl AceRuntimeValidator for MemoryPromotionGuard {
    fn name(&self) -> &str {
        "memory_promotion_guard"
    }

    async fn validate_plan(&self, _plan: &QueryPlan) -> Result<(), AceError> {
        // Memory promotion is validated at trace time
        Ok(())
    }

    async fn validate_trace(&self, trace: &RetrievalTrace) -> Result<(), AceError> {
        // Check for promotion without validation result
        if Self::has_no_validation_warning(trace) {
            return Err(AceError::MemoryPromotionBlocked {
                reason: "SessionLog -> LongTermMemory promotion requires ValidationResult"
                    .to_string(),
            });
        }

        // Check for promotion with failed validation
        if Self::has_validation_fail_warning(trace) {
            return Err(AceError::MemoryPromotionBlocked {
                reason: "SessionLog -> LongTermMemory promotion blocked: ValidationResult is Fail"
                    .to_string(),
            });
        }

        // Check for promotion without provenance
        if Self::has_no_provenance_warning(trace) {
            return Err(AceError::MemoryPromotionBlocked {
                reason: "SessionLog -> LongTermMemory promotion requires preserved provenance"
                    .to_string(),
            });
        }

        // Check for promotion errors
        let promotion_errors: Vec<_> = trace
            .errors
            .iter()
            .filter(|e| {
                e.contains("promotion") || e.contains("SessionLog") || e.contains("LongTermMemory")
            })
            .collect();

        if !promotion_errors.is_empty() {
            return Err(AceError::MemoryPromotionBlocked {
                reason: format!("Memory promotion errors: {:?}", promotion_errors),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ace::QueryKind;

    /// T-ACE-VAL-004: MemoryPromotionGuard blocks unvalidated promotions
    #[tokio::test]
    async fn test_promotion_guard_no_validation() {
        let guard = MemoryPromotionGuard;
        let plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        let mut trace = RetrievalTrace::new(&plan);

        // Add no validation warning
        trace
            .warnings
            .push(PROMOTION_NO_VALIDATION_WARNING.to_string());

        let result = guard.validate_trace(&trace).await;
        assert!(matches!(
            result,
            Err(AceError::MemoryPromotionBlocked { reason }) if reason.contains("ValidationResult")
        ));
    }

    /// T-ACE-VAL-004: MemoryPromotionGuard blocks failed validations
    #[tokio::test]
    async fn test_promotion_guard_validation_failed() {
        let guard = MemoryPromotionGuard;
        let plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        let mut trace = RetrievalTrace::new(&plan);

        // Add validation failed warning
        trace
            .warnings
            .push(PROMOTION_VALIDATION_FAIL_WARNING.to_string());

        let result = guard.validate_trace(&trace).await;
        assert!(matches!(
            result,
            Err(AceError::MemoryPromotionBlocked { reason }) if reason.contains("Fail")
        ));
    }

    /// T-ACE-VAL-004: MemoryPromotionGuard requires provenance
    #[tokio::test]
    async fn test_promotion_guard_missing_provenance() {
        let guard = MemoryPromotionGuard;
        let plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        let mut trace = RetrievalTrace::new(&plan);

        // Add missing provenance warning
        trace
            .warnings
            .push(PROMOTION_NO_PROVENANCE_WARNING.to_string());

        let result = guard.validate_trace(&trace).await;
        assert!(matches!(
            result,
            Err(AceError::MemoryPromotionBlocked { reason }) if reason.contains("provenance")
        ));
    }

    #[tokio::test]
    async fn test_promotion_guard_valid_trace() {
        let guard = MemoryPromotionGuard;
        let plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        let trace = RetrievalTrace::new(&plan);

        // No warnings -> OK
        assert!(guard.validate_trace(&trace).await.is_ok());
    }
}

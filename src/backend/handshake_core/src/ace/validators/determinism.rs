//! ContextDeterminismGuard (§2.6.6.7.11.1)
//!
//! Validates determinism requirements based on mode:
//! - Strict mode: MUST verify seed presence in ContextSnapshot
//! - Replay mode: MUST verify retrieval_candidates.ids_hash matches persisted list

use async_trait::async_trait;

use super::AceRuntimeValidator;
use crate::ace::{AceError, DeterminismMode, QueryPlan, RetrievalTrace};

/// Marker in trace warnings indicating seed is missing in strict mode
pub const MISSING_SEED_WARNING: &str = "determinism:missing_seed";

/// Marker in trace warnings indicating ids_hash mismatch in replay mode
pub const IDS_HASH_MISMATCH_WARNING: &str = "determinism:ids_hash_mismatch";

/// ContextDeterminismGuard enforces determinism invariants.
///
/// Per §2.6.6.7.11.1:
/// - Strict mode: seed MUST be present in ContextSnapshot
/// - Replay mode: retrieval_candidates.ids_hash MUST match persisted list
pub struct ContextDeterminismGuard;

#[async_trait]
impl AceRuntimeValidator for ContextDeterminismGuard {
    fn name(&self) -> &str {
        "determinism_guard"
    }

    async fn validate_plan(&self, plan: &QueryPlan) -> Result<(), AceError> {
        // Plan-level validation: just check mode is valid
        // Actual seed/hash checks happen at trace time
        match plan.determinism_mode {
            DeterminismMode::Strict | DeterminismMode::Replay => Ok(()),
        }
    }

    async fn validate_trace(&self, trace: &RetrievalTrace) -> Result<(), AceError> {
        // Check for determinism warnings that indicate violations
        for warning in &trace.warnings {
            if warning.starts_with(MISSING_SEED_WARNING) {
                return Err(AceError::DeterminismViolation {
                    reason: "Strict mode requires seed in ContextSnapshot but none was provided"
                        .to_string(),
                });
            }

            if warning.starts_with(IDS_HASH_MISMATCH_WARNING) {
                return Err(AceError::DeterminismViolation {
                    reason: "Replay mode ids_hash does not match persisted candidate list"
                        .to_string(),
                });
            }
        }

        // Check for determinism errors (more severe)
        let determinism_errors: Vec<_> = trace
            .errors
            .iter()
            .filter(|e| e.contains("determinism") || e.contains("seed") || e.contains("ids_hash"))
            .collect();

        if !determinism_errors.is_empty() {
            return Err(AceError::DeterminismViolation {
                reason: format!("Determinism errors: {:?}", determinism_errors),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ace::QueryKind;

    /// T-ACE-VAL-001: Determinism guard validates strict mode seed requirement
    #[tokio::test]
    async fn test_determinism_guard_strict_mode_missing_seed() {
        let guard = ContextDeterminismGuard;
        let mut plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        plan.determinism_mode = DeterminismMode::Strict;

        let mut trace = RetrievalTrace::new(&plan);

        // Add missing seed warning
        trace.warnings.push(MISSING_SEED_WARNING.to_string());

        let result = guard.validate_trace(&trace).await;
        assert!(matches!(
            result,
            Err(AceError::DeterminismViolation { reason }) if reason.contains("seed")
        ));
    }

    /// T-ACE-VAL-001: Determinism guard validates replay mode hash consistency
    #[tokio::test]
    async fn test_determinism_guard_replay_mode_hash_mismatch() {
        let guard = ContextDeterminismGuard;
        let mut plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        plan.determinism_mode = DeterminismMode::Replay;

        let mut trace = RetrievalTrace::new(&plan);

        // Add ids_hash mismatch warning
        trace.warnings.push(IDS_HASH_MISMATCH_WARNING.to_string());

        let result = guard.validate_trace(&trace).await;
        assert!(matches!(
            result,
            Err(AceError::DeterminismViolation { reason }) if reason.contains("ids_hash")
        ));
    }

    #[tokio::test]
    async fn test_determinism_guard_valid_trace() {
        let guard = ContextDeterminismGuard;
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
    async fn test_determinism_guard_plan_validation() {
        let guard = ContextDeterminismGuard;

        let mut plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );

        // Strict mode -> OK
        plan.determinism_mode = DeterminismMode::Strict;
        assert!(guard.validate_plan(&plan).await.is_ok());

        // Replay mode -> OK
        plan.determinism_mode = DeterminismMode::Replay;
        assert!(guard.validate_plan(&plan).await.is_ok());
    }
}

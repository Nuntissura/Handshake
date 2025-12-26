//! CompactionSchemaGuard (§2.6.6.7.11.3)
//!
//! Validates compaction block schemas:
//! - Every Decision block MUST contain at least one SourceRef in evidence_refs
//! - Every Constraint block MUST map to a LAW or RID anchor

use async_trait::async_trait;

use super::AceRuntimeValidator;
use crate::ace::{AceError, QueryPlan, RetrievalTrace};

/// Marker in trace warnings indicating Decision block missing evidence
pub const DECISION_MISSING_EVIDENCE_WARNING: &str = "compaction:decision_missing_evidence";

/// Marker in trace warnings indicating Constraint block missing anchor
pub const CONSTRAINT_MISSING_ANCHOR_WARNING: &str = "compaction:constraint_missing_anchor";

/// CompactionSchemaGuard enforces schema correctness for compaction blocks.
///
/// Per §2.6.6.7.11.3:
/// - Every Decision block MUST have >= 1 SourceRef in evidence_refs
/// - Every Constraint block MUST map to LAW or RID anchor
pub struct CompactionSchemaGuard;

impl CompactionSchemaGuard {
    /// Check if trace has Decision blocks missing evidence
    fn has_decision_missing_evidence(trace: &RetrievalTrace) -> bool {
        trace
            .warnings
            .iter()
            .any(|w| w.starts_with(DECISION_MISSING_EVIDENCE_WARNING))
    }

    /// Check if trace has Constraint blocks missing anchor
    fn has_constraint_missing_anchor(trace: &RetrievalTrace) -> bool {
        trace
            .warnings
            .iter()
            .any(|w| w.starts_with(CONSTRAINT_MISSING_ANCHOR_WARNING))
    }

    /// Extract block ID from warning if present
    fn extract_block_id(warning: &str, prefix: &str) -> Option<String> {
        warning
            .strip_prefix(prefix)
            .map(|rest| rest.trim_start_matches(':').to_string())
    }
}

#[async_trait]
impl AceRuntimeValidator for CompactionSchemaGuard {
    fn name(&self) -> &str {
        "compaction_schema_guard"
    }

    async fn validate_plan(&self, _plan: &QueryPlan) -> Result<(), AceError> {
        // Compaction schema is validated at trace time, not plan time
        Ok(())
    }

    async fn validate_trace(&self, trace: &RetrievalTrace) -> Result<(), AceError> {
        // Check for Decision blocks missing evidence
        if Self::has_decision_missing_evidence(trace) {
            let block_id = trace
                .warnings
                .iter()
                .find_map(|w| Self::extract_block_id(w, DECISION_MISSING_EVIDENCE_WARNING))
                .unwrap_or_else(|| "unknown".to_string());

            return Err(AceError::CompactionSchemaViolation {
                reason: format!(
                    "Decision block '{}' missing required SourceRef in evidence_refs",
                    block_id
                ),
            });
        }

        // Check for Constraint blocks missing anchor
        if Self::has_constraint_missing_anchor(trace) {
            let block_id = trace
                .warnings
                .iter()
                .find_map(|w| Self::extract_block_id(w, CONSTRAINT_MISSING_ANCHOR_WARNING))
                .unwrap_or_else(|| "unknown".to_string());

            return Err(AceError::CompactionSchemaViolation {
                reason: format!(
                    "Constraint block '{}' missing required LAW or RID anchor mapping",
                    block_id
                ),
            });
        }

        // Check for compaction errors
        let compaction_errors: Vec<_> = trace
            .errors
            .iter()
            .filter(|e| {
                e.contains("compaction")
                    || e.contains("Decision")
                    || e.contains("Constraint")
                    || e.contains("evidence_refs")
            })
            .collect();

        if !compaction_errors.is_empty() {
            return Err(AceError::CompactionSchemaViolation {
                reason: format!("Compaction errors: {:?}", compaction_errors),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ace::QueryKind;

    /// T-ACE-VAL-003: CompactionSchemaGuard validates Decision evidence
    #[tokio::test]
    async fn test_compaction_guard_decision_missing_evidence() {
        let guard = CompactionSchemaGuard;
        let plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        let mut trace = RetrievalTrace::new(&plan);

        // Add Decision missing evidence warning
        trace.warnings.push(format!(
            "{}:decision_123",
            DECISION_MISSING_EVIDENCE_WARNING
        ));

        let result = guard.validate_trace(&trace).await;
        assert!(matches!(
            result,
            Err(AceError::CompactionSchemaViolation { reason }) if reason.contains("Decision")
        ));
    }

    /// T-ACE-VAL-003: CompactionSchemaGuard validates Constraint anchors
    #[tokio::test]
    async fn test_compaction_guard_constraint_missing_anchor() {
        let guard = CompactionSchemaGuard;
        let plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        let mut trace = RetrievalTrace::new(&plan);

        // Add Constraint missing anchor warning
        trace.warnings.push(format!(
            "{}:constraint_456",
            CONSTRAINT_MISSING_ANCHOR_WARNING
        ));

        let result = guard.validate_trace(&trace).await;
        assert!(matches!(
            result,
            Err(AceError::CompactionSchemaViolation { reason }) if reason.contains("Constraint")
        ));
    }

    #[tokio::test]
    async fn test_compaction_guard_valid_trace() {
        let guard = CompactionSchemaGuard;
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
    async fn test_compaction_guard_error_detection() {
        let guard = CompactionSchemaGuard;
        let plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        let mut trace = RetrievalTrace::new(&plan);

        // Add compaction error
        trace
            .errors
            .push("compaction: failed to parse Decision block".to_string());

        let result = guard.validate_trace(&trace).await;
        assert!(matches!(
            result,
            Err(AceError::CompactionSchemaViolation { .. })
        ));
    }
}

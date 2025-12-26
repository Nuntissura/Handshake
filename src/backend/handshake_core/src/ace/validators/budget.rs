//! RetrievalBudgetGuard (§2.6.6.7.14.11)
//!
//! Enforces budget constraints on retrieval operations.
//!
//! Fail if:
//! - max_total_evidence_tokens exceeded
//! - max_snippets_total exceeded
//! - any bounded read exceeds max_read_tokens without truncation flag

use async_trait::async_trait;

use super::AceRuntimeValidator;
use crate::ace::{AceError, QueryPlan, RetrievalTrace};

/// RetrievalBudgetGuard enforces token and snippet budgets.
///
/// This guard ensures that:
/// 1. The QueryPlan has valid (non-zero) budget values
/// 2. The RetrievalTrace does not exceed budget limits
/// 3. All oversized spans have proper truncation flags
pub struct RetrievalBudgetGuard;

#[async_trait]
impl AceRuntimeValidator for RetrievalBudgetGuard {
    fn name(&self) -> &str {
        "budget_guard"
    }

    async fn validate_plan(&self, plan: &QueryPlan) -> Result<(), AceError> {
        // Validate that budgets are reasonable (non-zero where required)
        plan.budgets.validate()?;

        // Warn-level checks (don't fail, but log)
        // In a real implementation, we'd use tracing here
        if plan.budgets.max_snippets_total > 100 {
            // This is unusually high, but not an error
        }

        Ok(())
    }

    async fn validate_trace(&self, trace: &RetrievalTrace) -> Result<(), AceError> {
        let budgets = &trace.budgets_applied;

        // Check 1: max_total_evidence_tokens
        let total_tokens = trace.total_span_tokens();
        if total_tokens > budgets.max_total_evidence_tokens {
            return Err(AceError::BudgetExceeded {
                field: "max_total_evidence_tokens".to_string(),
                actual: total_tokens,
                max: budgets.max_total_evidence_tokens,
            });
        }

        // Check 2: max_snippets_total
        let total_snippets = trace.spans.len() as u32;
        if total_snippets > budgets.max_snippets_total {
            return Err(AceError::BudgetExceeded {
                field: "max_snippets_total".to_string(),
                actual: total_snippets,
                max: budgets.max_snippets_total,
            });
        }

        // Check 3: max_snippets_per_source
        let per_source = trace.snippets_per_source();
        for (source_id, count) in per_source {
            if count > budgets.max_snippets_per_source {
                return Err(AceError::BudgetExceeded {
                    field: format!("max_snippets_per_source[{}]", source_id),
                    actual: count,
                    max: budgets.max_snippets_per_source,
                });
            }
        }

        // Check 4: max_read_tokens per span (with truncation flag check)
        let untruncated = trace.find_untruncated_oversized_spans(budgets.max_read_tokens);
        if let Some(span) = untruncated.first() {
            return Err(AceError::TruncationFlagMissing {
                source_id: span.source_ref.source_id.to_string(),
            });
        }

        // Check 5: max_candidates_total
        let total_candidates = trace.candidates.len() as u32;
        if total_candidates > budgets.max_candidates_total {
            return Err(AceError::BudgetExceeded {
                field: "max_candidates_total".to_string(),
                actual: total_candidates,
                max: budgets.max_candidates_total,
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ace::{QueryKind, SourceRef, SpanExtraction};
    use uuid::Uuid;

    /// T-ACE-RAG-005: Budget enforcement
    #[tokio::test]
    async fn test_budget_guard_plan_validation() {
        let guard = RetrievalBudgetGuard;

        // Valid plan should pass
        let plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        assert!(guard.validate_plan(&plan).await.is_ok());

        // Plan with zero max_total_evidence_tokens should fail
        let mut invalid_plan = plan.clone();
        invalid_plan.budgets.max_total_evidence_tokens = 0;
        let result = guard.validate_plan(&invalid_plan).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_budget_guard_trace_token_limit() {
        let guard = RetrievalBudgetGuard;
        let plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        let mut trace = RetrievalTrace::new(&plan);

        // Add spans that exceed budget
        trace.budgets_applied.max_total_evidence_tokens = 100;
        let source = SourceRef::new(Uuid::new_v4(), "hash".to_string());
        trace.spans.push(SpanExtraction {
            source_ref: source.clone(),
            selector: "test".to_string(),
            start: 0,
            end: 500,
            token_estimate: 150, // Exceeds 100
        });

        let result = guard.validate_trace(&trace).await;
        assert!(
            matches!(result, Err(AceError::BudgetExceeded { field, .. }) if field == "max_total_evidence_tokens")
        );
    }

    #[tokio::test]
    async fn test_budget_guard_snippets_per_source() {
        let guard = RetrievalBudgetGuard;
        let plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        let mut trace = RetrievalTrace::new(&plan);

        trace.budgets_applied.max_snippets_per_source = 2;
        let source = SourceRef::new(Uuid::new_v4(), "hash".to_string());

        // Add 3 snippets from same source (exceeds limit of 2)
        for i in 0..3 {
            trace.spans.push(SpanExtraction {
                source_ref: source.clone(),
                selector: format!("test{}", i),
                start: i * 100,
                end: (i + 1) * 100,
                token_estimate: 10,
            });
        }

        let result = guard.validate_trace(&trace).await;
        assert!(
            matches!(result, Err(AceError::BudgetExceeded { field, .. }) if field.starts_with("max_snippets_per_source"))
        );
    }

    #[tokio::test]
    async fn test_budget_guard_truncation_flag() {
        let guard = RetrievalBudgetGuard;
        let plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        let mut trace = RetrievalTrace::new(&plan);

        trace.budgets_applied.max_read_tokens = 100;
        let source = SourceRef::new(Uuid::new_v4(), "hash".to_string());

        // Add oversized span WITHOUT truncation flag
        trace.spans.push(SpanExtraction {
            source_ref: source.clone(),
            selector: "test".to_string(),
            start: 0,
            end: 1000,
            token_estimate: 200, // Exceeds 100, no flag
        });

        let result = guard.validate_trace(&trace).await;
        assert!(matches!(
            result,
            Err(AceError::TruncationFlagMissing { .. })
        ));

        // Now add truncation flag
        trace
            .truncation_flags
            .push(format!("truncated:{}", source.source_id));
        let result = guard.validate_trace(&trace).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_budget_guard_valid_trace() {
        let guard = RetrievalBudgetGuard;
        let plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        let mut trace = RetrievalTrace::new(&plan);

        // Add valid spans within budget
        let source = SourceRef::new(Uuid::new_v4(), "hash".to_string());
        trace.spans.push(SpanExtraction {
            source_ref: source.clone(),
            selector: "test".to_string(),
            start: 0,
            end: 100,
            token_estimate: 50, // Within default 500 limit
        });

        let result = guard.validate_trace(&trace).await;
        assert!(result.is_ok());
    }
}

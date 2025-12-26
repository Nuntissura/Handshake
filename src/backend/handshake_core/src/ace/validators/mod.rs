//! ACE Runtime Validators (§2.6.6.7.11)
//!
//! This module implements the `AceRuntimeValidator` trait and all 12 required guards:
//!
//! **Original 4 guards (§2.6.6.7.14.11):**
//! 1. RetrievalBudgetGuard - Budget enforcement
//! 2. ContextPackFreshnessGuard - Stale pack detection
//! 3. IndexDriftGuard - Source hash mismatch detection
//! 4. CacheKeyGuard - Cache key validation in strict mode
//!
//! **New 8 security guards (§2.6.6.7.11.1-8):**
//! 5. ContextDeterminismGuard - Strict/replay determinism requirements
//! 6. ArtifactHandleOnlyGuard - Inline delta size limits
//! 7. CompactionSchemaGuard - Decision/Constraint schema validation
//! 8. MemoryPromotionGuard - SessionLog->LongTermMemory gating
//! 9. CloudLeakageGuard - Sensitive data protection for cloud tiers
//! 10. PromptInjectionGuard - Injection pattern detection
//! 11. JobBoundaryRoutingGuard - Job routing invariant enforcement
//! 12. LocalPayloadGuard - Encrypted local storage validation

pub mod artifact;
pub mod boundary;
pub mod budget;
pub mod cache;
pub mod compaction;
pub mod determinism;
pub mod drift;
pub mod freshness;
pub mod injection;
pub mod leakage;
pub mod payload;
pub mod promotion;

use async_trait::async_trait;

use crate::ace::{AceError, QueryPlan, RetrievalTrace};

/// HSK-TRAIT-002: ACE Runtime Validator (§2.6.6.7.14.11)
///
/// All retrieval operations MUST be validated by a pipeline of these guards.
/// Each validator can check both the QueryPlan (before execution) and
/// the RetrievalTrace (after execution, before usage).
#[async_trait]
pub trait AceRuntimeValidator: Send + Sync {
    /// Unique identifier for the validator (e.g., "budget_guard")
    fn name(&self) -> &str;

    /// Validates the QueryPlan before execution.
    ///
    /// Returns: Ok(()) if allowed, Err(AceError) if blocked.
    ///
    /// Use this to validate that the plan's budgets, filters, and configuration
    /// are reasonable before any retrieval work is done.
    async fn validate_plan(&self, plan: &QueryPlan) -> Result<(), AceError>;

    /// Validates the RetrievalTrace after execution (before usage).
    ///
    /// Returns: Ok(()) if allowed, Err(AceError) if blocked.
    ///
    /// Use this to validate that the retrieval results meet all constraints
    /// and that no violations occurred during retrieval.
    async fn validate_trace(&self, trace: &RetrievalTrace) -> Result<(), AceError>;
}

/// A pipeline of validators that runs all checks in sequence
pub struct ValidatorPipeline {
    validators: Vec<Box<dyn AceRuntimeValidator>>,
}

impl ValidatorPipeline {
    /// Create an empty pipeline
    pub fn new() -> Self {
        Self {
            validators: Vec::new(),
        }
    }

    /// Create a pipeline with all 12 required guards per §2.6.6.7.11
    pub fn with_default_guards() -> Self {
        Self {
            validators: vec![
                // Original 4 guards (§2.6.6.7.14.11)
                Box::new(budget::RetrievalBudgetGuard),
                Box::new(freshness::ContextPackFreshnessGuard),
                Box::new(drift::IndexDriftGuard),
                Box::new(cache::CacheKeyGuard),
                // New 8 security guards (§2.6.6.7.11.1-8)
                Box::new(determinism::ContextDeterminismGuard),
                Box::new(artifact::ArtifactHandleOnlyGuard),
                Box::new(compaction::CompactionSchemaGuard),
                Box::new(promotion::MemoryPromotionGuard),
                Box::new(leakage::CloudLeakageGuard),
                Box::new(injection::PromptInjectionGuard),
                Box::new(boundary::JobBoundaryRoutingGuard),
                Box::new(payload::LocalPayloadGuard),
            ],
        }
    }

    /// Add a validator to the pipeline
    pub fn add(&mut self, validator: Box<dyn AceRuntimeValidator>) {
        self.validators.push(validator);
    }

    /// Validate a query plan against all validators
    ///
    /// Returns the first error encountered, or Ok(()) if all pass.
    pub async fn validate_plan(&self, plan: &QueryPlan) -> Result<(), AceError> {
        for validator in &self.validators {
            validator.validate_plan(plan).await?;
        }
        Ok(())
    }

    /// Validate a retrieval trace against all validators
    ///
    /// Returns the first error encountered, or Ok(()) if all pass.
    pub async fn validate_trace(&self, trace: &RetrievalTrace) -> Result<(), AceError> {
        for validator in &self.validators {
            validator.validate_trace(trace).await?;
        }
        Ok(())
    }

    /// Validate both plan and trace, returning all errors
    pub async fn validate_all(&self, plan: &QueryPlan, trace: &RetrievalTrace) -> Vec<AceError> {
        let mut errors = Vec::new();

        for validator in &self.validators {
            if let Err(e) = validator.validate_plan(plan).await {
                errors.push(e);
            }
            if let Err(e) = validator.validate_trace(trace).await {
                errors.push(e);
            }
        }

        errors
    }

    /// Get validator names for logging
    pub fn validator_names(&self) -> Vec<&str> {
        self.validators.iter().map(|v| v.name()).collect()
    }
}

impl Default for ValidatorPipeline {
    fn default() -> Self {
        Self::with_default_guards()
    }
}

// Re-exports - Original 4 guards
pub use budget::RetrievalBudgetGuard;
pub use cache::CacheKeyGuard;
pub use drift::IndexDriftGuard;
pub use freshness::ContextPackFreshnessGuard;

// Re-exports - New 8 security guards (§2.6.6.7.11.1-8)
pub use artifact::ArtifactHandleOnlyGuard;
pub use boundary::JobBoundaryRoutingGuard;
pub use compaction::CompactionSchemaGuard;
pub use determinism::ContextDeterminismGuard;
pub use injection::PromptInjectionGuard;
pub use leakage::CloudLeakageGuard;
pub use payload::LocalPayloadGuard;
pub use promotion::MemoryPromotionGuard;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ace::{QueryKind, QueryPlan};

    #[tokio::test]
    async fn test_validator_pipeline_default() {
        let pipeline = ValidatorPipeline::with_default_guards();
        assert_eq!(pipeline.validator_names().len(), 12);

        // Original 4 guards
        assert!(pipeline.validator_names().contains(&"budget_guard"));
        assert!(pipeline.validator_names().contains(&"freshness_guard"));
        assert!(pipeline.validator_names().contains(&"drift_guard"));
        assert!(pipeline.validator_names().contains(&"cache_key_guard"));

        // New 8 security guards (§2.6.6.7.11.1-8)
        assert!(pipeline.validator_names().contains(&"determinism_guard"));
        assert!(pipeline
            .validator_names()
            .contains(&"artifact_handle_only_guard"));
        assert!(pipeline
            .validator_names()
            .contains(&"compaction_schema_guard"));
        assert!(pipeline
            .validator_names()
            .contains(&"memory_promotion_guard"));
        assert!(pipeline.validator_names().contains(&"cloud_leakage_guard"));
        assert!(pipeline
            .validator_names()
            .contains(&"prompt_injection_guard"));
        assert!(pipeline
            .validator_names()
            .contains(&"job_boundary_routing_guard"));
        assert!(pipeline.validator_names().contains(&"local_payload_guard"));
    }

    #[tokio::test]
    async fn test_validator_pipeline_plan_validation() {
        let pipeline = ValidatorPipeline::with_default_guards();
        let plan = QueryPlan::new(
            "test query".to_string(),
            QueryKind::FactLookup,
            "test_policy".to_string(),
        )
        .with_default_route();

        // Default plan should pass all validators
        let result = pipeline.validate_plan(&plan).await;
        assert!(result.is_ok(), "Unexpected error: {:?}", result);
    }
}

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
//!
//! **Hardened Security Enforcement (§2.6.6.7.11.0):**
//! - [HSK-ACE-VAL-100] Content Awareness: Validators MUST resolve raw UTF-8 content
//! - [HSK-ACE-VAL-101] Atomic Poisoning: Injection triggers JobState::Poisoned
//! - [HSK-ACE-VAL-102] NFC Normalization: Scans use NFC-normalized, case-folded text

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
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

use crate::ace::{AceError, QueryPlan, RetrievalTrace, SourceRef};

// ============================================================================
// Content Resolution Types [HSK-ACE-VAL-100]
// ============================================================================

/// Sensitivity level for content classification per §2.6.6.7.11.5
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SensitivityLevel {
    Low,
    #[default]
    Medium,
    High,
    /// Unknown sensitivity MUST default to block per mandate
    Unknown,
}

/// Classification metadata for leakage checks per §2.6.6.7.11.5
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentClassification {
    /// Sensitivity level of the content
    pub sensitivity: SensitivityLevel,
    /// Whether the content can be exported to cloud tiers
    pub exportable: bool,
    /// Whether this is a composite (bundle/dataset_slice) requiring recursive checks
    pub is_composite: bool,
    /// Member refs for recursive classification (bundles, dataset slices)
    pub member_refs: Vec<SourceRef>,
}

impl Default for ContentClassification {
    fn default() -> Self {
        Self {
            // Default to Unknown which MUST block per §2.6.6.7.11.5
            sensitivity: SensitivityLevel::Unknown,
            exportable: false,
            is_composite: false,
            member_refs: Vec::new(),
        }
    }
}

/// Content resolver for validators requiring raw UTF-8 access [HSK-ACE-VAL-100]
///
/// Validators MUST NOT operate on hashes or handles alone. This trait provides
/// the mechanism to resolve actual content for security scanning.
#[async_trait]
pub trait ContentResolver: Send + Sync {
    /// Resolve raw UTF-8 content for a SourceRef
    ///
    /// Returns the full text content that MUST be scanned by security validators.
    async fn resolve_content(&self, source_ref: &SourceRef) -> Result<String, AceError>;

    /// Resolve classification metadata for a SourceRef per §2.6.6.7.11.5
    ///
    /// For composite refs (bundles, dataset_slices), returns member_refs for recursive checks.
    async fn resolve_classification(
        &self,
        source_ref: &SourceRef,
    ) -> Result<ContentClassification, AceError>;

    /// Batch resolve content for multiple SourceRefs (optimization)
    async fn resolve_content_batch(
        &self,
        source_refs: &[SourceRef],
    ) -> Result<Vec<(SourceRef, String)>, AceError> {
        let mut results = Vec::with_capacity(source_refs.len());
        for source_ref in source_refs {
            let content = self.resolve_content(source_ref).await?;
            results.push((source_ref.clone(), content));
        }
        Ok(results)
    }
}

/// Retrieved snippet with resolved content for security scanning
#[derive(Debug, Clone)]
pub struct ResolvedSnippet {
    /// The source reference
    pub source_ref: SourceRef,
    /// Raw UTF-8 content (resolved, not just a hash)
    pub content: String,
    /// Classification metadata
    pub classification: ContentClassification,
}

/// Result of security validation with detailed violation info
#[derive(Debug, Clone)]
pub struct SecurityValidationResult {
    /// Whether validation passed
    pub passed: bool,
    /// Detected violations (empty if passed)
    pub violations: Vec<SecurityViolation>,
}

/// A security violation detected by validators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityViolation {
    /// Violation type identifier
    pub violation_type: SecurityViolationType,
    /// Human-readable description
    pub description: String,
    /// Source ref where violation was detected (if applicable)
    pub source_ref: Option<SourceRef>,
    /// The pattern or content that triggered the violation
    pub trigger: String,
}

/// Types of security violations per §2.6.6.7.11
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityViolationType {
    /// Prompt injection detected [§2.6.6.7.11.6]
    PromptInjection,
    /// Cloud leakage attempted [§2.6.6.7.11.5]
    CloudLeakage,
    /// High sensitivity content exposure
    SensitivityViolation,
    /// Unknown sensitivity defaulted to block
    UnknownSensitivity,
    /// Non-exportable content attempted export
    ExportViolation,
}

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

    /// Validate trace with content resolution [HSK-ACE-VAL-100]
    ///
    /// This method MUST be used for security-critical validation to ensure
    /// content is actually scanned, not just metadata.
    pub async fn validate_trace_with_resolver(
        &self,
        trace: &RetrievalTrace,
        resolver: &dyn ContentResolver,
    ) -> Result<SecurityValidationResult, AceError> {
        let mut violations = Vec::new();

        // First run standard trace validation
        for validator in &self.validators {
            if let Err(e) = validator.validate_trace(trace).await {
                // Convert AceError to SecurityViolation
                let violation = match &e {
                    AceError::PromptInjectionDetected { pattern } => SecurityViolation {
                        violation_type: SecurityViolationType::PromptInjection,
                        description: format!("Prompt injection detected: {}", pattern),
                        source_ref: None,
                        trigger: pattern.to_string(),
                    },
                    AceError::CloudLeakageBlocked { reason } => SecurityViolation {
                        violation_type: SecurityViolationType::CloudLeakage,
                        description: format!("Cloud leakage blocked: {}", reason),
                        source_ref: None,
                        trigger: reason.clone(),
                    },
                    _ => SecurityViolation {
                        violation_type: SecurityViolationType::SensitivityViolation,
                        description: e.to_string(),
                        source_ref: None,
                        trigger: String::new(),
                    },
                };
                violations.push(violation);
            }
        }

        // Then perform content-aware security scans
        let content_violations = self.scan_resolved_content(trace, resolver).await?;
        violations.extend(content_violations);

        Ok(SecurityValidationResult {
            passed: violations.is_empty(),
            violations,
        })
    }

    /// Scan resolved content for security violations [HSK-ACE-VAL-100]
    async fn scan_resolved_content(
        &self,
        trace: &RetrievalTrace,
        resolver: &dyn ContentResolver,
    ) -> Result<Vec<SecurityViolation>, AceError> {
        let mut violations = Vec::new();

        // Extract all SourceRefs from spans
        let source_refs: Vec<SourceRef> =
            trace.spans.iter().map(|s| s.source_ref.clone()).collect();

        // Resolve and scan each source
        for source_ref in &source_refs {
            // Resolve content
            let content = resolver.resolve_content(source_ref).await?;

            // Scan for prompt injection using NFC-normalized text [HSK-ACE-VAL-102]
            if let Some(pattern) = injection::PromptInjectionGuard::scan_for_injection_nfc(&content)
            {
                violations.push(SecurityViolation {
                    violation_type: SecurityViolationType::PromptInjection,
                    description: format!(
                        "Prompt injection pattern '{}' detected in resolved content",
                        pattern
                    ),
                    source_ref: Some(source_ref.clone()),
                    trigger: pattern.to_string(),
                });
            }

            // Check classification for leakage [§2.6.6.7.11.5]
            let classification = resolver.resolve_classification(source_ref).await?;

            // Unknown sensitivity MUST block
            if classification.sensitivity == SensitivityLevel::Unknown {
                violations.push(SecurityViolation {
                    violation_type: SecurityViolationType::UnknownSensitivity,
                    description: "Content has unknown sensitivity level - blocked by default"
                        .to_string(),
                    source_ref: Some(source_ref.clone()),
                    trigger: "unknown_sensitivity".to_string(),
                });
            }

            // High sensitivity content cannot be exported
            if classification.sensitivity == SensitivityLevel::High && !classification.exportable {
                violations.push(SecurityViolation {
                    violation_type: SecurityViolationType::SensitivityViolation,
                    description: "High sensitivity content cannot be included in retrieval"
                        .to_string(),
                    source_ref: Some(source_ref.clone()),
                    trigger: "high_sensitivity".to_string(),
                });
            }

            // Recursive check for composites
            if classification.is_composite {
                let mut visited = HashSet::new();
                visited.insert(source_ref.source_id);
                let composite_violations = self
                    .check_composite_recursive(&classification.member_refs, resolver, &mut visited)
                    .await?;
                violations.extend(composite_violations);
            }
        }

        Ok(violations)
    }

    /// Recursive classification check for composite refs [§2.6.6.7.11.5]
    fn check_composite_recursive<'a>(
        &'a self,
        member_refs: &'a [SourceRef],
        resolver: &'a dyn ContentResolver,
        visited: &'a mut HashSet<Uuid>,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Vec<SecurityViolation>, AceError>> + Send + 'a>,
    > {
        Box::pin(async move {
            let mut violations = Vec::new();

            for member_ref in member_refs {
                // Prevent cycles
                if visited.contains(&member_ref.source_id) {
                    continue;
                }
                visited.insert(member_ref.source_id);

                let classification = resolver.resolve_classification(member_ref).await?;

                // Check member classification
                if classification.sensitivity == SensitivityLevel::Unknown {
                    violations.push(SecurityViolation {
                        violation_type: SecurityViolationType::UnknownSensitivity,
                        description: "Composite member has unknown sensitivity".to_string(),
                        source_ref: Some(member_ref.clone()),
                        trigger: "unknown_sensitivity".to_string(),
                    });
                }

                if classification.sensitivity == SensitivityLevel::High
                    && !classification.exportable
                {
                    violations.push(SecurityViolation {
                        violation_type: SecurityViolationType::SensitivityViolation,
                        description: "Composite member has high sensitivity".to_string(),
                        source_ref: Some(member_ref.clone()),
                        trigger: "high_sensitivity".to_string(),
                    });
                }

                // Recurse into nested composites
                if classification.is_composite {
                    let nested = self
                        .check_composite_recursive(&classification.member_refs, resolver, visited)
                        .await?;
                    violations.extend(nested);
                }
            }

            Ok(violations)
        })
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

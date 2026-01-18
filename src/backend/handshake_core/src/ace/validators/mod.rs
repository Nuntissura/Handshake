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
use crate::llm::ModelTier;
use crate::storage::{Block, Database};

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

// ============================================================================
// Storage-backed Content Resolver [HSK-ACE-VAL-100]
// ============================================================================

use std::sync::Arc;

/// Storage-backed content resolver for ACE validators [HSK-ACE-VAL-100]
///
/// Resolves raw UTF-8 content and classification metadata from the storage layer.
/// Used by validators to scan actual content rather than hashes/handles.
pub struct StorageContentResolver {
    db: Arc<dyn Database>,
}

impl StorageContentResolver {
    /// Create a new StorageContentResolver with the given database
    pub fn new(db: Arc<dyn Database>) -> Self {
        Self { db }
    }

    /// Convert storage Block sensitivity string to SensitivityLevel
    fn parse_sensitivity(sensitivity: Option<&str>) -> SensitivityLevel {
        match sensitivity {
            Some("low") => SensitivityLevel::Low,
            Some("medium") => SensitivityLevel::Medium,
            Some("high") => SensitivityLevel::High,
            // NULL or unknown -> Unknown (which MUST block per mandate)
            _ => SensitivityLevel::Unknown,
        }
    }

    /// Build classification from Block data
    fn build_classification(block: &Block) -> ContentClassification {
        ContentClassification {
            sensitivity: Self::parse_sensitivity(block.sensitivity.as_deref()),
            // NULL or true = exportable; false = non-exportable
            exportable: block.exportable.unwrap_or(true),
            // Blocks are not composite; composite refs (bundles) not yet implemented
            is_composite: false,
            member_refs: Vec::new(),
        }
    }
}

#[async_trait]
impl ContentResolver for StorageContentResolver {
    async fn resolve_content(&self, source_ref: &SourceRef) -> Result<String, AceError> {
        // SourceRef.source_id is the block UUID
        let block_id = source_ref.source_id.to_string();
        let block = self
            .db
            .get_block(&block_id)
            .await
            .map_err(|e| AceError::ValidationFailed {
                message: format!("Failed to resolve content for block {}: {}", block_id, e),
            })?;
        Ok(block.raw_content)
    }

    async fn resolve_classification(
        &self,
        source_ref: &SourceRef,
    ) -> Result<ContentClassification, AceError> {
        let block_id = source_ref.source_id.to_string();
        let block = self
            .db
            .get_block(&block_id)
            .await
            .map_err(|e| AceError::ValidationFailed {
                message: format!(
                    "Failed to resolve classification for block {}: {}",
                    block_id, e
                ),
            })?;
        Ok(Self::build_classification(&block))
    }
}

// ============================================================================
// Security Scan API [HSK-ACE-VAL-100]
// ============================================================================

/// Scan content for security violations before Cloud model calls [HSK-ACE-VAL-100]
///
/// This function performs content-aware security scanning:
/// 1. Resolves raw UTF-8 content from storage
/// 2. Scans for prompt injection using NFC-normalized text [HSK-ACE-VAL-102]
/// 3. Checks classification (sensitivity, exportable) for CloudLeakageGuard
///
/// Returns `Err(AceError::PromptInjectionDetected)` if injection found,
/// `Err(AceError::CloudLeakageBlocked)` if leakage violation detected,
/// or `Ok(())` if content is safe to use.
pub async fn scan_content_for_security(
    source_refs: &[SourceRef],
    resolver: &dyn ContentResolver,
    model_tier: ModelTier,
) -> Result<(), AceError> {
    for source_ref in source_refs {
        // 1. Resolve raw content
        let content = resolver.resolve_content(source_ref).await?;

        // 2. Scan for prompt injection [HSK-ACE-VAL-102]
        if let Some(found) = injection::PromptInjectionGuard::scan_for_injection_nfc(&content) {
            return Err(AceError::PromptInjectionDetected {
                pattern: found.pattern,
                offset: found.offset,
                context: found.context,
            });
        }

        // 3. Check classification for cloud leakage [§2.6.6.7.11.5]
        // Only enforce blocking for Cloud tier; Local tier logs but doesn't block
        if model_tier == ModelTier::Cloud {
            let classification = resolver.resolve_classification(source_ref).await?;

            // Unknown sensitivity MUST block for Cloud
            if classification.sensitivity == SensitivityLevel::Unknown {
                return Err(AceError::CloudLeakageBlocked {
                    reason: format!(
                        "Content has unknown sensitivity - blocked for cloud model (source: {})",
                        source_ref.source_id
                    ),
                });
            }

            // High sensitivity non-exportable content blocked
            if classification.sensitivity == SensitivityLevel::High && !classification.exportable {
                return Err(AceError::CloudLeakageBlocked {
                    reason: format!(
                        "High sensitivity non-exportable content blocked (source: {})",
                        source_ref.source_id
                    ),
                });
            }

            // Non-exportable content blocked for cloud
            if !classification.exportable {
                return Err(AceError::CloudLeakageBlocked {
                    reason: format!(
                        "Non-exportable content blocked for cloud model (source: {})",
                        source_ref.source_id
                    ),
                });
            }

            // Recursive check for composites
            if classification.is_composite {
                let mut visited = HashSet::new();
                visited.insert(source_ref.source_id);
                check_composite_leakage(&classification.member_refs, resolver, &mut visited)
                    .await?;
            }
        }
    }

    Ok(())
}

/// Recursive leakage check for composite refs (bundles, dataset_slices) [§2.6.6.7.11.5]
fn check_composite_leakage<'a>(
    member_refs: &'a [SourceRef],
    resolver: &'a dyn ContentResolver,
    visited: &'a mut HashSet<Uuid>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), AceError>> + Send + 'a>> {
    Box::pin(async move {
        for member_ref in member_refs {
            // Prevent cycles
            if visited.contains(&member_ref.source_id) {
                continue;
            }
            visited.insert(member_ref.source_id);

            let classification = resolver.resolve_classification(member_ref).await?;

            // Check member classification
            if classification.sensitivity == SensitivityLevel::Unknown {
                return Err(AceError::CloudLeakageBlocked {
                    reason: format!(
                        "Composite member has unknown sensitivity (source: {})",
                        member_ref.source_id
                    ),
                });
            }

            if !classification.exportable {
                return Err(AceError::CloudLeakageBlocked {
                    reason: format!(
                        "Composite member is non-exportable (source: {})",
                        member_ref.source_id
                    ),
                });
            }

            // Recurse into nested composites
            if classification.is_composite {
                check_composite_leakage(&classification.member_refs, resolver, visited).await?;
            }
        }

        Ok(())
    })
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
    /// Character offset of the trigger within normalized content (if available)
    pub offset: Option<usize>,
    /// Context snippet (~20 chars) surrounding the trigger (if available)
    pub context: Option<String>,
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
                    AceError::PromptInjectionDetected {
                        pattern,
                        offset,
                        context,
                    } => SecurityViolation {
                        violation_type: SecurityViolationType::PromptInjection,
                        description: format!("Prompt injection detected: {}", pattern),
                        source_ref: None,
                        trigger: pattern.to_string(),
                        offset: Some(*offset),
                        context: Some(context.clone()),
                    },
                    AceError::CloudLeakageBlocked { reason } => SecurityViolation {
                        violation_type: SecurityViolationType::CloudLeakage,
                        description: format!("Cloud leakage blocked: {}", reason),
                        source_ref: None,
                        trigger: reason.clone(),
                        offset: None,
                        context: None,
                    },
                    _ => SecurityViolation {
                        violation_type: SecurityViolationType::SensitivityViolation,
                        description: e.to_string(),
                        source_ref: None,
                        trigger: String::new(),
                        offset: None,
                        context: None,
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
            if let Some(found) = injection::PromptInjectionGuard::scan_for_injection_nfc(&content) {
                let pattern = found.pattern.clone();
                violations.push(SecurityViolation {
                    violation_type: SecurityViolationType::PromptInjection,
                    description: format!(
                        "Prompt injection pattern '{}' detected in resolved content",
                        pattern
                    ),
                    source_ref: Some(source_ref.clone()),
                    trigger: pattern,
                    offset: Some(found.offset),
                    context: Some(found.context.clone()),
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
                    offset: None,
                    context: None,
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
                    offset: None,
                    context: None,
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
    // This async helper recurses on nested composites; keeping `&self` makes the
    // call sites consistent with other pipeline helpers even though it is only
    // used for recursion.
    #[allow(clippy::only_used_in_recursion)]
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
                        offset: None,
                        context: None,
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
                        offset: None,
                        context: None,
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

// ============================================================================
// Flight Recorder ACE Validation Payload [§2.6.6.7.14.12]
// ============================================================================

/// ACE validation payload for Flight Recorder logging per §2.6.6.7.14.12.
///
/// This struct represents the `ace_validation` sub-object that MUST be included
/// in `llm_inference` events for retrieval-backed model calls.
///
/// Required fields per spec:
/// - QueryPlan (id + hash)
/// - normalized_query_hash
/// - RetrievalTrace (id + hash)
/// - cache hits/misses per stage
/// - rerank metadata (method + inputs_hash + outputs_hash)
/// - diversity metadata (method + lambda)
/// - per-source caps enforcement outcomes
/// - drift detection flags and any degraded-mode marker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AceValidationPayload {
    // Scope identification
    pub scope_document_id: String,
    pub scope_inputs_hash: String,
    pub determinism_mode: String,

    // Candidate tracking
    pub candidate_ids: Vec<String>,
    pub candidate_hashes: Vec<String>,
    pub selected_ids: Vec<String>,
    pub selected_hashes: Vec<String>,

    // Truncation/compaction
    pub truncation_applied: bool,
    pub truncation_flags: Vec<String>,
    pub compaction_applied: bool,

    // QueryPlan fields
    pub query_plan_id: String,
    pub query_plan_hash: String,
    pub normalized_query_hash: String,

    // RetrievalTrace fields
    pub retrieval_trace_id: String,
    pub retrieval_trace_hash: String,

    // Rerank metadata (optional)
    pub rerank_method: Option<String>,
    pub rerank_inputs_hash: Option<String>,
    pub rerank_outputs_hash: Option<String>,

    // Diversity metadata (optional)
    pub diversity_method: Option<String>,
    pub diversity_lambda: Option<f64>,

    // Cache markers (per stage)
    pub cache_markers: Vec<CacheMarker>,

    // Drift flags and degraded mode
    pub drift_flags: Vec<String>,
    pub degraded_mode: bool,

    // Artifact handles (Phase 2)
    pub artifact_handles: Vec<String>,

    // Validation results
    pub guards_passed: Vec<String>,
    pub guards_failed: Vec<String>,
    pub violation_codes: Vec<String>,
    pub outcome: String, // "pass" | "fail" | "degraded"

    // Model tier and timing
    pub model_tier: String,
    pub validation_duration_ms: u64,
}

/// Cache hit/miss marker for a retrieval stage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMarker {
    pub stage: String,
    pub cache_hit: bool,
}

impl AceValidationPayload {
    /// Build an ACE validation payload from a QueryPlan and RetrievalTrace.
    ///
    /// This method MUST be called after validation to capture the complete
    /// validation state for Flight Recorder logging per §2.6.6.7.14.12.
    pub fn from_plan_and_trace(
        plan: &QueryPlan,
        trace: &RetrievalTrace,
        scope_document_id: String,
        guards_passed: Vec<String>,
        guards_failed: Vec<String>,
        violation_codes: Vec<String>,
        model_tier: &str,
        validation_duration_ms: u64,
    ) -> Self {
        use sha2::{Digest, Sha256};

        // Compute plan hash
        let plan_json = serde_json::to_string(plan).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(plan_json.as_bytes());
        let query_plan_hash = hex::encode(hasher.finalize());

        // Compute trace hash
        let trace_json = serde_json::to_string(trace).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(trace_json.as_bytes());
        let retrieval_trace_hash = hex::encode(hasher.finalize());

        // Compute scope inputs hash (plan + document scope)
        let scope_inputs = format!("{}:{}", scope_document_id, plan.policy_id);
        let mut hasher = Sha256::new();
        hasher.update(scope_inputs.as_bytes());
        let scope_inputs_hash = hex::encode(hasher.finalize());

        // Extract candidate/selected ids and hashes
        let candidate_ids: Vec<_> = trace
            .candidates
            .iter()
            .map(|c| c.candidate_id.clone())
            .collect();
        let candidate_hashes: Vec<_> = trace
            .candidates
            .iter()
            .map(|c| {
                let json = serde_json::to_string(c).unwrap_or_default();
                let mut hasher = Sha256::new();
                hasher.update(json.as_bytes());
                hex::encode(hasher.finalize())
            })
            .collect();
        let selected_ids: Vec<_> = trace
            .selected
            .iter()
            .map(|s| format!("{:?}", s.candidate_ref))
            .collect();
        let selected_hashes: Vec<_> = trace
            .selected
            .iter()
            .map(|s| {
                let json = serde_json::to_string(s).unwrap_or_default();
                let mut hasher = Sha256::new();
                hasher.update(json.as_bytes());
                hex::encode(hasher.finalize())
            })
            .collect();

        // Build cache markers from route_taken
        let cache_markers: Vec<_> = trace
            .route_taken
            .iter()
            .map(|r| CacheMarker {
                stage: format!("{:?}", r.store),
                cache_hit: r.cache_hit.unwrap_or(false),
            })
            .collect();

        // Extract drift flags from warnings
        let drift_flags: Vec<_> = trace
            .warnings
            .iter()
            .filter(|w| {
                w.contains("drift")
                    || w.contains("hash_mismatch")
                    || w.contains("provenance")
                    || w.contains("stale")
            })
            .cloned()
            .collect();

        // Determine outcome
        let outcome = if guards_failed.is_empty() && violation_codes.is_empty() {
            if drift_flags.is_empty() {
                "pass"
            } else {
                "degraded"
            }
        } else {
            "fail"
        };

        Self {
            scope_document_id,
            scope_inputs_hash,
            determinism_mode: format!("{:?}", plan.determinism_mode).to_lowercase(),
            candidate_ids,
            candidate_hashes,
            selected_ids,
            selected_hashes,
            truncation_applied: !trace.truncation_flags.is_empty(),
            truncation_flags: trace.truncation_flags.clone(),
            compaction_applied: trace
                .candidates
                .iter()
                .any(|c| c.store == crate::ace::StoreKind::ContextPacks),
            query_plan_id: plan.plan_id.to_string(),
            query_plan_hash,
            normalized_query_hash: trace.normalized_query_hash.clone(),
            retrieval_trace_id: trace.trace_id.to_string(),
            retrieval_trace_hash,
            rerank_method: if trace.rerank.used {
                Some(trace.rerank.method.clone())
            } else {
                None
            },
            rerank_inputs_hash: if trace.rerank.used {
                Some(trace.rerank.inputs_hash.clone())
            } else {
                None
            },
            rerank_outputs_hash: if trace.rerank.used {
                Some(trace.rerank.outputs_hash.clone())
            } else {
                None
            },
            diversity_method: if trace.diversity.used {
                Some(trace.diversity.method.clone())
            } else {
                None
            },
            diversity_lambda: trace.diversity.lambda,
            cache_markers,
            drift_flags,
            degraded_mode: outcome == "degraded",
            artifact_handles: Vec::new(), // Phase 2
            guards_passed,
            guards_failed,
            violation_codes,
            outcome: outcome.to_string(),
            model_tier: model_tier.to_string(),
            validation_duration_ms,
        }
    }

    /// Convert to serde_json::Value for inclusion in Flight Recorder events.
    pub fn to_json_value(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

impl ValidatorPipeline {
    /// Validate plan and trace, returning an AceValidationPayload for Flight Recorder.
    ///
    /// This method runs all validators and captures the results in a format
    /// suitable for logging to Flight Recorder per §2.6.6.7.14.12.
    ///
    /// The returned payload should be included in the `ace_validation` field
    /// of `llm_inference` events.
    pub async fn validate_and_log(
        &self,
        plan: &QueryPlan,
        trace: &RetrievalTrace,
        scope_document_id: String,
        model_tier: &str,
    ) -> (Vec<AceError>, AceValidationPayload) {
        use std::time::Instant;

        // WAIVER [CX-573E]: timing-only instrumentation for FR latency metrics; no determinism impact
        let start = Instant::now();
        let mut guards_passed = Vec::new();
        let mut guards_failed = Vec::new();
        let mut violation_codes = Vec::new();
        let mut errors = Vec::new();

        // Run all validators and collect results
        for validator in &self.validators {
            let name = validator.name().to_string();

            // Validate plan
            if let Err(e) = validator.validate_plan(plan).await {
                guards_failed.push(format!("{}:plan", name));
                violation_codes.push(format!("{}", e));
                errors.push(e);
            } else {
                guards_passed.push(format!("{}:plan", name));
            }

            // Validate trace
            if let Err(e) = validator.validate_trace(trace).await {
                guards_failed.push(format!("{}:trace", name));
                violation_codes.push(format!("{}", e));
                errors.push(e);
            } else {
                guards_passed.push(format!("{}:trace", name));
            }
        }

        let validation_duration_ms = start.elapsed().as_millis() as u64;

        let payload = AceValidationPayload::from_plan_and_trace(
            plan,
            trace,
            scope_document_id,
            guards_passed,
            guards_failed,
            violation_codes,
            model_tier,
            validation_duration_ms,
        );

        (errors, payload)
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

// Re-exports - Content Resolution [HSK-ACE-VAL-100]
// StorageContentResolver and scan_content_for_security are exported at module level

// ============================================================================
// QueryPlan/RetrievalTrace Builders [§2.6.6.7.14.5]
// ============================================================================

use sha2::{Digest, Sha256};

use crate::ace::{
    CandidateRef, CandidateScores, QueryKind, RetrievalCandidate, RouteTaken, SelectedEvidence,
    SpanExtraction, StoreKind,
};

/// Build QueryPlan from document blocks [§2.6.6.7.14.5]
///
/// Returns `Err(AceError::ValidationFailed)` if any block has invalid UUID.
/// MUST NOT skip invalid UUIDs per content-awareness invariant.
pub fn build_query_plan_from_blocks(
    blocks: &[Block],
    query_text: &str,
    policy_id: &str,
) -> Result<QueryPlan, AceError> {
    // Validate ALL blocks have valid UUIDs first (fail-fast)
    for block in blocks {
        Uuid::parse_str(&block.id).map_err(|_| AceError::ValidationFailed {
            message: format!("Block has invalid UUID: {}", block.id),
        })?;
    }

    let plan = QueryPlan::new(
        query_text.to_string(),
        QueryKind::Summarize,
        policy_id.to_string(),
    );

    Ok(plan)
}

/// Build RetrievalTrace from blocks with SHA256 hashes [§2.6.6.7.14.5]
///
/// Returns `Err(AceError::ValidationFailed)` if any block has invalid UUID.
/// MUST NOT skip invalid UUIDs per content-awareness invariant.
pub fn build_retrieval_trace_from_blocks(
    blocks: &[Block],
    plan: &QueryPlan,
) -> Result<RetrievalTrace, AceError> {
    let mut trace = RetrievalTrace::new(plan);

    for (i, block) in blocks.iter().enumerate() {
        // MUST fail on invalid UUID - enforced by returning Result
        let source_id = Uuid::parse_str(&block.id).map_err(|_| AceError::ValidationFailed {
            message: format!("Block has invalid UUID: {}", block.id),
        })?;

        // Compute SHA256 hash of raw_content
        let mut hasher = Sha256::new();
        hasher.update(block.raw_content.as_bytes());
        let source_hash = hex::encode(hasher.finalize());

        let source_ref = SourceRef::new(source_id, source_hash);

        // Build candidate
        let candidate = RetrievalCandidate::from_source(
            source_ref.clone(),
            StoreKind::ShadowWsLexical,
            CandidateScores::default(),
        );
        trace.candidates.push(candidate);

        // Build selected evidence (all selected for doc summarization)
        trace.selected.push(SelectedEvidence {
            candidate_ref: CandidateRef::Source(source_ref.clone()),
            final_rank: i as u32,
            final_score: 1.0,
            why: "doc_summarization_full_retrieval".to_string(),
        });

        // Build span
        let token_estimate = (block.raw_content.len() / 4) as u32; // ~4 chars/token
        trace.spans.push(SpanExtraction {
            source_ref,
            selector: format!("block:{}", block.id),
            start: 0,
            end: block.raw_content.len() as u32,
            token_estimate,
        });
    }

    // Route taken
    trace.route_taken.push(RouteTaken {
        store: StoreKind::ShadowWsLexical,
        reason: "doc_summarization_block_retrieval".to_string(),
        cache_hit: Some(false),
    });

    Ok(trace)
}

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

    #[test]
    fn test_build_query_plan_fails_on_invalid_uuid() {
        use crate::storage::Block;

        let blocks = vec![Block {
            id: "not-a-valid-uuid".to_string(),
            document_id: "doc-1".to_string(),
            kind: "paragraph".to_string(),
            sequence: 1,
            raw_content: "Test content".to_string(),
            display_content: "Test content".to_string(),
            derived_content: serde_json::json!({}),
            sensitivity: None,
            exportable: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }];

        let result = build_query_plan_from_blocks(&blocks, "test query", "test_policy");
        assert!(result.is_err(), "Expected error for invalid UUID");
        let err = result.unwrap_err();
        assert!(
            matches!(err, AceError::ValidationFailed { .. }),
            "Expected ValidationFailed error"
        );
    }

    #[test]
    fn test_build_retrieval_trace_computes_sha256() {
        use crate::storage::Block;

        let block_id = Uuid::new_v4().to_string();
        let blocks = vec![Block {
            id: block_id.clone(),
            document_id: "doc-1".to_string(),
            kind: "paragraph".to_string(),
            sequence: 1,
            raw_content: "Test content for hashing".to_string(),
            display_content: "Test content for hashing".to_string(),
            derived_content: serde_json::json!({}),
            sensitivity: None,
            exportable: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }];

        let plan = QueryPlan::new(
            "test query".to_string(),
            QueryKind::Summarize,
            "test_policy".to_string(),
        );

        let result = build_retrieval_trace_from_blocks(&blocks, &plan);
        assert!(result.is_ok(), "Expected successful trace build");

        let trace = result.unwrap();
        assert_eq!(trace.spans.len(), 1);
        assert_eq!(trace.candidates.len(), 1);
        assert_eq!(trace.selected.len(), 1);

        // Verify SHA256 hash is computed (64 hex chars)
        let source_hash = &trace.spans[0].source_ref.source_hash;
        assert_eq!(source_hash.len(), 64, "SHA256 hash should be 64 hex chars");

        // Verify hash is correct for "Test content for hashing"
        let expected_hash = {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(b"Test content for hashing");
            hex::encode(hasher.finalize())
        };
        assert_eq!(source_hash, &expected_hash);
    }

    #[test]
    fn test_build_retrieval_trace_fails_on_invalid_uuid() {
        use crate::storage::Block;

        let blocks = vec![Block {
            id: "invalid-uuid".to_string(),
            document_id: "doc-1".to_string(),
            kind: "paragraph".to_string(),
            sequence: 1,
            raw_content: "Test content".to_string(),
            display_content: "Test content".to_string(),
            derived_content: serde_json::json!({}),
            sensitivity: None,
            exportable: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }];

        let plan = QueryPlan::new(
            "test query".to_string(),
            QueryKind::Summarize,
            "test_policy".to_string(),
        );

        let result = build_retrieval_trace_from_blocks(&blocks, &plan);
        assert!(result.is_err(), "Expected error for invalid UUID");
    }
}

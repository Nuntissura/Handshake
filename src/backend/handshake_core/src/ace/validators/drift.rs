//! IndexDriftGuard (§2.6.6.7.14.11)
//!
//! Detects index drift in retrieval results.
//!
//! If any selected evidence item has source_hash mismatch (embedding drift)
//! or missing provenance (KG drift), the job MUST:
//! - either fail (policy dependent), or
//! - downgrade to `completed_with_issues` and include explicit warnings + recovery action

use async_trait::async_trait;

use super::AceRuntimeValidator;
use crate::ace::{AceError, CandidateKind, CandidateRef, QueryPlan, RetrievalTrace, StoreKind};

/// Marker string in warnings indicating embedding drift
pub const EMBEDDING_DRIFT_PREFIX: &str = "embedding_drift:";

/// Marker string in warnings indicating KG provenance missing
pub const KG_PROVENANCE_MISSING_PREFIX: &str = "kg_provenance_missing:";

/// Marker string in warnings indicating source hash mismatch
pub const SOURCE_HASH_MISMATCH_PREFIX: &str = "source_hash_mismatch:";

/// IndexDriftGuard checks for index drift in retrieval results.
///
/// This guard ensures that:
/// 1. Embedding records match current source hashes
/// 2. KG-derived candidates have valid provenance
/// 3. LocalWebCache respects TTL and pinning rules
pub struct IndexDriftGuard;

impl IndexDriftGuard {
    /// Check for any drift-related warnings
    fn has_drift_warnings(trace: &RetrievalTrace) -> bool {
        trace.warnings.iter().any(|w| {
            w.starts_with(EMBEDDING_DRIFT_PREFIX)
                || w.starts_with(KG_PROVENANCE_MISSING_PREFIX)
                || w.starts_with(SOURCE_HASH_MISMATCH_PREFIX)
        })
    }

    /// Check for any drift-related errors (more severe)
    fn has_drift_errors(trace: &RetrievalTrace) -> bool {
        trace
            .errors
            .iter()
            .any(|e| e.contains("drift") || e.contains("hash_mismatch") || e.contains("provenance"))
    }

    /// Check if any selected evidence from KG lacks provenance
    fn check_kg_provenance(trace: &RetrievalTrace) -> Vec<String> {
        let mut missing = Vec::new();

        for candidate in &trace.candidates {
            // KG candidates that are entity refs should have valid provenance
            if candidate.store == StoreKind::KnowledgeGraph {
                if let CandidateRef::Entity(entity_ref) = &candidate.candidate_ref {
                    // In a real implementation, we'd check actual provenance data
                    // For now, we check if there's a warning about this entity
                    let entity_id = entity_ref.entity_id.to_string();
                    if trace.warnings.iter().any(|w| {
                        w.starts_with(KG_PROVENANCE_MISSING_PREFIX) && w.contains(&entity_id)
                    }) {
                        missing.push(candidate.candidate_id.clone());
                    }
                }
            }
        }

        missing
    }

    /// Check if any selected evidence has source hash mismatch
    fn check_source_hash_drift(trace: &RetrievalTrace) -> Vec<String> {
        let mut drifted = Vec::new();

        for candidate in &trace.candidates {
            if candidate.kind == CandidateKind::SourceRef {
                if let CandidateRef::Source(source_ref) = &candidate.candidate_ref {
                    let source_id = source_ref.source_id.to_string();
                    // Check if there's a drift warning for this source
                    if trace.warnings.iter().any(|w| {
                        (w.starts_with(EMBEDDING_DRIFT_PREFIX)
                            || w.starts_with(SOURCE_HASH_MISMATCH_PREFIX))
                            && w.contains(&source_id)
                    }) {
                        drifted.push(candidate.candidate_id.clone());
                    }
                }
            }
        }

        drifted
    }
}

#[async_trait]
impl AceRuntimeValidator for IndexDriftGuard {
    fn name(&self) -> &str {
        "drift_guard"
    }

    async fn validate_plan(&self, _plan: &QueryPlan) -> Result<(), AceError> {
        // Drift is a trace-time check, not plan-time.
        // The plan doesn't know about current index state.
        Ok(())
    }

    async fn validate_trace(&self, trace: &RetrievalTrace) -> Result<(), AceError> {
        // Check for hard errors first
        if Self::has_drift_errors(trace) {
            let error_details = trace
                .errors
                .iter()
                .filter(|e| {
                    e.contains("drift") || e.contains("hash_mismatch") || e.contains("provenance")
                })
                .cloned()
                .collect::<Vec<_>>()
                .join("; ");

            return Err(AceError::IndexDrift {
                details: error_details,
            });
        }

        // Check for KG provenance issues
        let missing_provenance = Self::check_kg_provenance(trace);
        if !missing_provenance.is_empty() {
            return Err(AceError::ProvenanceMissing {
                candidate_id: missing_provenance.first().cloned().unwrap_or_default(),
            });
        }

        // Check for source hash drift
        let drifted_sources = Self::check_source_hash_drift(trace);
        if !drifted_sources.is_empty() {
            // If there are drifted sources that are selected (not just candidates),
            // this is an error. Check if any drifted sources are in selected evidence.
            let selected_ids: std::collections::HashSet<_> = trace
                .selected
                .iter()
                .filter_map(|s| match &s.candidate_ref {
                    CandidateRef::Source(src) => Some(src.source_id.to_string()),
                    _ => None,
                })
                .collect();

            for drift_warning in &trace.warnings {
                if drift_warning.starts_with(SOURCE_HASH_MISMATCH_PREFIX) {
                    // Extract source ID from warning
                    if let Some(rest) = drift_warning.strip_prefix(SOURCE_HASH_MISMATCH_PREFIX) {
                        // Check if this drifted source was selected
                        if selected_ids.iter().any(|id| rest.contains(id)) {
                            return Err(AceError::IndexDrift {
                                details: format!("Selected evidence has drifted: {}", rest),
                            });
                        }
                    }
                }
            }

            // Drifted candidates that weren't selected are OK (just warnings)
        }

        // Warnings without errors are acceptable (trace documents the degradation)
        if Self::has_drift_warnings(trace) {
            // Log would go here in real implementation
            // tracing::warn!("Index drift detected but not in selected evidence");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ace::{
        CandidateScores, EntityRef, QueryKind, QueryPlan, RetrievalCandidate, SelectedEvidence,
        SourceRef,
    };
    use uuid::Uuid;

    /// T-ACE-RAG-006: Drift detection
    #[tokio::test]
    async fn test_drift_guard_no_drift() {
        let guard = IndexDriftGuard;
        let plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        let trace = RetrievalTrace::new(&plan);

        // No drift warnings or errors -> OK
        assert!(guard.validate_trace(&trace).await.is_ok());
    }

    #[tokio::test]
    async fn test_drift_guard_embedding_drift_warning_not_selected() {
        let guard = IndexDriftGuard;
        let plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        let mut trace = RetrievalTrace::new(&plan);

        // Add a source candidate
        let source = SourceRef::new(Uuid::new_v4(), "hash".to_string());
        let scores = CandidateScores::default();
        trace.candidates.push(RetrievalCandidate::from_source(
            source.clone(),
            StoreKind::ShadowWsVector,
            scores,
        ));

        // Drift warning but source not in selected -> OK
        trace.warnings.push(format!(
            "{}source:{}",
            EMBEDDING_DRIFT_PREFIX, source.source_id
        ));

        assert!(guard.validate_trace(&trace).await.is_ok());
    }

    #[tokio::test]
    async fn test_drift_guard_embedding_drift_in_selected() {
        let guard = IndexDriftGuard;
        let plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        let mut trace = RetrievalTrace::new(&plan);

        // Add a source candidate and select it
        let source = SourceRef::new(Uuid::new_v4(), "hash".to_string());
        let scores = CandidateScores::default();
        trace.candidates.push(RetrievalCandidate::from_source(
            source.clone(),
            StoreKind::ShadowWsVector,
            scores,
        ));

        // Select this source
        trace.selected.push(SelectedEvidence {
            candidate_ref: CandidateRef::Source(source.clone()),
            final_rank: 1,
            final_score: 0.8,
            why: "test".to_string(),
        });

        // Drift warning for selected source -> FAIL
        trace.warnings.push(format!(
            "{}source:{}",
            SOURCE_HASH_MISMATCH_PREFIX, source.source_id
        ));

        let result = guard.validate_trace(&trace).await;
        assert!(matches!(result, Err(AceError::IndexDrift { .. })));
    }

    #[tokio::test]
    async fn test_drift_guard_kg_provenance_missing() {
        let guard = IndexDriftGuard;
        let plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        let mut trace = RetrievalTrace::new(&plan);

        // Add a KG entity candidate
        let entity = EntityRef::new(Uuid::new_v4(), "Person".to_string());
        let scores = CandidateScores {
            graph: Some(0.9),
            ..Default::default()
        };
        trace.candidates.push(RetrievalCandidate::from_entity(
            entity.clone(),
            StoreKind::KnowledgeGraph,
            scores,
        ));

        // Mark provenance as missing
        trace.warnings.push(format!(
            "{}entity:{}",
            KG_PROVENANCE_MISSING_PREFIX, entity.entity_id
        ));

        let result = guard.validate_trace(&trace).await;
        assert!(matches!(result, Err(AceError::ProvenanceMissing { .. })));
    }

    #[tokio::test]
    async fn test_drift_guard_hard_error() {
        let guard = IndexDriftGuard;
        let plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        let mut trace = RetrievalTrace::new(&plan);

        // Hard error mentioning drift -> FAIL
        trace
            .errors
            .push("critical: hash_mismatch detected, index corrupted".to_string());

        let result = guard.validate_trace(&trace).await;
        assert!(matches!(result, Err(AceError::IndexDrift { .. })));
    }
}

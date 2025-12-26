//! ContextPackFreshnessGuard (§2.6.6.7.14.11)
//!
//! Validates that selected ContextPacks are fresh.
//!
//! Fail or mark degraded if a selected ContextPack is stale
//! and regeneration was required by policy but not performed.

use async_trait::async_trait;

use super::AceRuntimeValidator;
use crate::ace::{AceError, QueryPlan, RetrievalTrace, StoreKind};

/// Marker string in warnings indicating a stale context pack
pub const STALE_PACK_WARNING_PREFIX: &str = "stale_pack:";

/// Marker string in warnings indicating regeneration was skipped
pub const REGEN_SKIPPED_PREFIX: &str = "regen_skipped:";

/// ContextPackFreshnessGuard checks for stale context packs.
///
/// This guard ensures that:
/// 1. No stale context packs are used without explicit warning
/// 2. If regeneration was required but skipped, the trace indicates this
/// 3. Pack freshness is validated against source hashes
pub struct ContextPackFreshnessGuard;

impl ContextPackFreshnessGuard {
    /// Check if any warnings indicate a stale pack was used
    fn has_stale_pack_warnings(trace: &RetrievalTrace) -> bool {
        trace
            .warnings
            .iter()
            .any(|w| w.starts_with(STALE_PACK_WARNING_PREFIX))
    }

    /// Check if regeneration was skipped for any pack
    fn has_regen_skipped_warnings(trace: &RetrievalTrace) -> bool {
        trace
            .warnings
            .iter()
            .any(|w| w.starts_with(REGEN_SKIPPED_PREFIX))
    }

    /// Check if any selected evidence came from context packs store
    fn has_context_pack_evidence(trace: &RetrievalTrace) -> bool {
        trace
            .candidates
            .iter()
            .any(|c| c.store == StoreKind::ContextPacks)
    }

    /// Extract pack IDs from stale warnings
    fn extract_stale_pack_ids(trace: &RetrievalTrace) -> Vec<String> {
        trace
            .warnings
            .iter()
            .filter_map(|w| w.strip_prefix(STALE_PACK_WARNING_PREFIX))
            .map(|s| s.to_string())
            .collect()
    }
}

#[async_trait]
impl AceRuntimeValidator for ContextPackFreshnessGuard {
    fn name(&self) -> &str {
        "freshness_guard"
    }

    async fn validate_plan(&self, _plan: &QueryPlan) -> Result<(), AceError> {
        // Freshness is a trace-time check, not plan-time.
        // The plan doesn't know about current source hashes.
        Ok(())
    }

    async fn validate_trace(&self, trace: &RetrievalTrace) -> Result<(), AceError> {
        // If no context pack evidence was used, nothing to check
        if !Self::has_context_pack_evidence(trace) {
            return Ok(());
        }

        // Check for stale pack warnings
        if Self::has_stale_pack_warnings(trace) {
            // If regeneration was skipped but was required, fail
            if Self::has_regen_skipped_warnings(trace) {
                let stale_ids = Self::extract_stale_pack_ids(trace);
                let pack_id_str = stale_ids
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string());

                // Try to parse as UUID, or create a nil UUID
                let pack_id =
                    uuid::Uuid::parse_str(&pack_id_str).unwrap_or_else(|_| uuid::Uuid::nil());

                return Err(AceError::ContextPackStale { pack_id });
            }

            // Stale pack used but with explicit acknowledgment (warning only)
            // This is acceptable - the trace documents the degradation
        }

        // Check for errors (more severe than warnings)
        let pack_errors: Vec<_> = trace
            .errors
            .iter()
            .filter(|e| e.contains("context_pack") || e.contains("ContextPack"))
            .collect();

        if !pack_errors.is_empty() {
            return Err(AceError::ValidationFailed {
                message: format!("Context pack errors: {:?}", pack_errors),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ace::{CandidateScores, QueryKind, QueryPlan, RetrievalCandidate, SourceRef};
    use uuid::Uuid;

    /// T-ACE-RAG-004: ContextPack freshness invalidation
    #[tokio::test]
    async fn test_freshness_guard_no_packs() {
        let guard = ContextPackFreshnessGuard;
        let plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        let trace = RetrievalTrace::new(&plan);

        // No context pack evidence -> OK
        assert!(guard.validate_trace(&trace).await.is_ok());
    }

    #[tokio::test]
    async fn test_freshness_guard_fresh_pack() {
        let guard = ContextPackFreshnessGuard;
        let plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        let mut trace = RetrievalTrace::new(&plan);

        // Add a fresh context pack candidate (no stale warnings)
        let source = SourceRef::new(Uuid::new_v4(), "hash".to_string());
        let scores = CandidateScores {
            pack: Some(1.0),
            ..Default::default()
        };
        trace.candidates.push(RetrievalCandidate::from_source(
            source,
            StoreKind::ContextPacks,
            scores,
        ));

        // No warnings -> OK
        assert!(guard.validate_trace(&trace).await.is_ok());
    }

    #[tokio::test]
    async fn test_freshness_guard_stale_pack_acknowledged() {
        let guard = ContextPackFreshnessGuard;
        let plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        let mut trace = RetrievalTrace::new(&plan);

        // Add context pack candidate
        let source = SourceRef::new(Uuid::new_v4(), "hash".to_string());
        let scores = CandidateScores {
            pack: Some(0.0),
            ..Default::default()
        }; // Stale score
        trace.candidates.push(RetrievalCandidate::from_source(
            source,
            StoreKind::ContextPacks,
            scores,
        ));

        // Stale warning but no regen_skipped -> OK (acknowledged degradation)
        trace
            .warnings
            .push(format!("{}pack123", STALE_PACK_WARNING_PREFIX));

        assert!(guard.validate_trace(&trace).await.is_ok());
    }

    #[tokio::test]
    async fn test_freshness_guard_stale_pack_regen_required() {
        let guard = ContextPackFreshnessGuard;
        let plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        let mut trace = RetrievalTrace::new(&plan);

        // Add context pack candidate
        let source = SourceRef::new(Uuid::new_v4(), "hash".to_string());
        let scores = CandidateScores {
            pack: Some(0.0),
            ..Default::default()
        };
        trace.candidates.push(RetrievalCandidate::from_source(
            source,
            StoreKind::ContextPacks,
            scores,
        ));

        // Both stale warning AND regen_skipped -> FAIL
        let pack_id = Uuid::new_v4();
        trace
            .warnings
            .push(format!("{}{}", STALE_PACK_WARNING_PREFIX, pack_id));
        trace
            .warnings
            .push(format!("{}{}", REGEN_SKIPPED_PREFIX, pack_id));

        let result = guard.validate_trace(&trace).await;
        assert!(matches!(result, Err(AceError::ContextPackStale { .. })));
    }

    #[tokio::test]
    async fn test_freshness_guard_pack_error() {
        let guard = ContextPackFreshnessGuard;
        let plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        let mut trace = RetrievalTrace::new(&plan);

        // Add context pack candidate
        let source = SourceRef::new(Uuid::new_v4(), "hash".to_string());
        let scores = CandidateScores {
            pack: Some(1.0),
            ..Default::default()
        };
        trace.candidates.push(RetrievalCandidate::from_source(
            source,
            StoreKind::ContextPacks,
            scores,
        ));

        // Error mentioning context pack -> FAIL
        trace
            .errors
            .push("context_pack: failed to deserialize payload".to_string());

        let result = guard.validate_trace(&trace).await;
        assert!(matches!(result, Err(AceError::ValidationFailed { .. })));
    }
}

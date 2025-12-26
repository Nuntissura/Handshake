//! CacheKeyGuard (§2.6.6.7.14.11)
//!
//! Ensures cache keys are computed and logged for all cacheable stages.
//! Rejects missing keys in strict mode.

use async_trait::async_trait;

use super::AceRuntimeValidator;
use crate::ace::{AceError, DeterminismMode, QueryPlan, RetrievalTrace};

/// Marker string in route_taken indicating cache was checked
pub const CACHE_CHECKED_MARKER: &str = "cache_checked";

/// CacheKeyGuard validates cache key handling.
///
/// This guard ensures that:
/// 1. In strict mode, cache keys are computable (policy_id present, etc.)
/// 2. Cache hit/miss is logged for each route step
/// 3. No cache stages are skipped without documentation
pub struct CacheKeyGuard;

impl CacheKeyGuard {
    /// Check if a query plan has all required fields for cache key computation
    fn can_compute_cache_key(plan: &QueryPlan) -> Result<(), String> {
        // Required fields for cache key
        if plan.policy_id.is_empty() {
            return Err("policy_id is empty".to_string());
        }

        if plan.query_text.is_empty() {
            return Err("query_text is empty".to_string());
        }

        // Route must be non-empty for meaningful caching
        if plan.route.is_empty() {
            return Err("route is empty".to_string());
        }

        Ok(())
    }

    /// Check if all route steps have cache_hit field populated
    fn all_routes_have_cache_status(trace: &RetrievalTrace) -> bool {
        trace.route_taken.iter().all(|r| r.cache_hit.is_some())
    }

    /// Get route steps missing cache status
    fn routes_missing_cache_status(trace: &RetrievalTrace) -> Vec<String> {
        trace
            .route_taken
            .iter()
            .filter(|r| r.cache_hit.is_none())
            .map(|r| format!("{:?}", r.store))
            .collect()
    }
}

#[async_trait]
impl AceRuntimeValidator for CacheKeyGuard {
    fn name(&self) -> &str {
        "cache_key_guard"
    }

    async fn validate_plan(&self, plan: &QueryPlan) -> Result<(), AceError> {
        // In strict mode, we require all cache key fields to be present
        if plan.determinism_mode == DeterminismMode::Strict {
            if let Err(reason) = Self::can_compute_cache_key(plan) {
                return Err(AceError::CacheKeyMissing {
                    stage: format!("query_plan: {}", reason),
                });
            }
        }

        // In replay mode, we're more lenient but still want basic validation
        if plan.policy_id.is_empty() && plan.determinism_mode == DeterminismMode::Replay {
            // Just a warning in replay mode, not an error
            // In real implementation: tracing::warn!("policy_id empty in replay mode");
        }

        Ok(())
    }

    async fn validate_trace(&self, trace: &RetrievalTrace) -> Result<(), AceError> {
        // For traces with routes taken, verify cache status was logged
        if !trace.route_taken.is_empty() && !Self::all_routes_have_cache_status(trace) {
            let missing = Self::routes_missing_cache_status(trace);

            // In strict mode (determined by budgets presence), this is an error
            // We check if normalized_query_hash is present as a proxy for strict mode
            if !trace.normalized_query_hash.is_empty() {
                // Check if any route was actually executed (not just planned)
                let executed_routes = trace
                    .route_taken
                    .iter()
                    .filter(|r| r.cache_hit.is_some() || !r.reason.is_empty())
                    .count();

                // If routes were executed but cache status missing, that's a problem
                if executed_routes > 0 && !missing.is_empty() {
                    return Err(AceError::CacheKeyMissing {
                        stage: format!("route_taken: missing cache_hit for {:?}", missing),
                    });
                }
            }
        }

        // Verify rerank cache info if reranking was used
        if trace.rerank.used {
            if trace.rerank.inputs_hash.is_empty() || trace.rerank.outputs_hash.is_empty() {
                return Err(AceError::CacheKeyMissing {
                    stage: "rerank: inputs_hash or outputs_hash empty".to_string(),
                });
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ace::{QueryKind, RerankInfo, RouteTaken, StoreKind};

    /// T-ACE-RAG-007: Cache key invalidation
    #[tokio::test]
    async fn test_cache_guard_strict_mode_valid() {
        let guard = CacheKeyGuard;
        let plan = QueryPlan::new(
            "test query".to_string(),
            QueryKind::FactLookup,
            "policy1".to_string(),
        )
        .with_default_route();

        // Valid strict mode plan -> OK
        assert!(guard.validate_plan(&plan).await.is_ok());
    }

    #[tokio::test]
    async fn test_cache_guard_strict_mode_empty_policy() {
        let guard = CacheKeyGuard;
        let mut plan = QueryPlan::new(
            "test query".to_string(),
            QueryKind::FactLookup,
            "".to_string(),
        );
        plan.determinism_mode = DeterminismMode::Strict;
        plan.route = vec![]; // Also empty route

        // Empty policy_id in strict mode -> FAIL
        let result = guard.validate_plan(&plan).await;
        assert!(
            matches!(result, Err(AceError::CacheKeyMissing { stage }) if stage.contains("policy_id"))
        );
    }

    #[tokio::test]
    async fn test_cache_guard_replay_mode_lenient() {
        let guard = CacheKeyGuard;
        let mut plan = QueryPlan::new(
            "test query".to_string(),
            QueryKind::FactLookup,
            "".to_string(),
        );
        plan.determinism_mode = DeterminismMode::Replay;
        plan.route = vec![]; // Empty route

        // Empty policy_id in replay mode -> OK (lenient)
        assert!(guard.validate_plan(&plan).await.is_ok());
    }

    #[tokio::test]
    async fn test_cache_guard_trace_with_cache_status() {
        let guard = CacheKeyGuard;
        let plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        let mut trace = RetrievalTrace::new(&plan);

        // Add routes with cache status
        trace.route_taken.push(RouteTaken {
            store: StoreKind::ContextPacks,
            reason: "primary".to_string(),
            cache_hit: Some(true),
        });
        trace.route_taken.push(RouteTaken {
            store: StoreKind::KnowledgeGraph,
            reason: "secondary".to_string(),
            cache_hit: Some(false),
        });

        // All routes have cache status -> OK
        assert!(guard.validate_trace(&trace).await.is_ok());
    }

    #[tokio::test]
    async fn test_cache_guard_trace_missing_cache_status() {
        let guard = CacheKeyGuard;
        let plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        let mut trace = RetrievalTrace::new(&plan);

        // Add routes, some with cache status, some without
        trace.route_taken.push(RouteTaken {
            store: StoreKind::ContextPacks,
            reason: "primary".to_string(),
            cache_hit: Some(true),
        });
        trace.route_taken.push(RouteTaken {
            store: StoreKind::KnowledgeGraph,
            reason: "secondary".to_string(),
            cache_hit: None, // Missing!
        });

        // Missing cache status for executed route -> FAIL
        let result = guard.validate_trace(&trace).await;
        assert!(matches!(result, Err(AceError::CacheKeyMissing { .. })));
    }

    #[tokio::test]
    async fn test_cache_guard_rerank_hash_required() {
        let guard = CacheKeyGuard;
        let plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        let mut trace = RetrievalTrace::new(&plan);

        // Reranking used but hashes missing
        trace.rerank = RerankInfo {
            used: true,
            method: "cross_encoder".to_string(),
            inputs_hash: "".to_string(), // Empty!
            outputs_hash: "output_hash".to_string(),
        };

        // Missing rerank hash -> FAIL
        let result = guard.validate_trace(&trace).await;
        assert!(
            matches!(result, Err(AceError::CacheKeyMissing { stage }) if stage.contains("rerank"))
        );
    }

    #[tokio::test]
    async fn test_cache_guard_rerank_hash_valid() {
        let guard = CacheKeyGuard;
        let plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        let mut trace = RetrievalTrace::new(&plan);

        // Reranking used with valid hashes
        trace.rerank = RerankInfo {
            used: true,
            method: "cross_encoder".to_string(),
            inputs_hash: "input_hash".to_string(),
            outputs_hash: "output_hash".to_string(),
        };

        // Valid rerank hashes -> OK
        assert!(guard.validate_trace(&trace).await.is_ok());
    }
}

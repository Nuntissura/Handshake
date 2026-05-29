//! MT-161 (part 2): ProgressiveRetriever — Tiered retrieval pipeline with
//! graceful degradation under CPU/GPU load.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::capsule::DegradationTier;

/// Trait to expose runtime load pressure to the retriever. Production
/// wires to actual CPU/GPU metrics; tests inject a fixed value.
pub trait LoadSignal {
    fn current_pressure(&self) -> f64; // [0.0, 1.0]
}

/// Retrieval tier markers used in DegradationReport. The pipeline tries
/// each tier in order, skipping the latest under high pressure.
pub const TIER_FULL_TEXT: &str = "full_text";
pub const TIER_VECTOR: &str = "vector";
pub const TIER_GRAPH_EXPANSION: &str = "graph_expansion";
pub const TIER_RERANK: &str = "re_rank";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetrievedItem {
    pub item_id: String,
    pub score: f64,
    pub tier: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DegradationReport {
    pub tiers_completed: Vec<String>,
    pub tiers_skipped: Vec<String>,
    pub load_signal_at_start: f64,
    pub started_at_utc: DateTime<Utc>,
    pub total_duration_ms: u64,
    pub tier_chosen: DegradationTier,
}

#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum RetrievalError {
    #[error("all tiers failed; no items retrieved")]
    AllTiersFailed,
    #[error("retrieval tier {tier} failed: {message}")]
    TierFailed { tier: String, message: String },
}

/// Hook for one tier of the pipeline. Production wires each tier to a
/// real backend (BM25, vector search, graph traversal, re-ranker); tests
/// inject closures.
pub trait RetrievalTier {
    fn tier_name(&self) -> &'static str;
    fn execute(
        &self,
        query: &str,
        carry: &[RetrievedItem],
    ) -> Result<Vec<RetrievedItem>, RetrievalError>;
}

pub struct ProgressiveRetriever<'a> {
    pub full_text: &'a dyn RetrievalTier,
    pub vector: &'a dyn RetrievalTier,
    pub graph: &'a dyn RetrievalTier,
    pub rerank: &'a dyn RetrievalTier,
}

impl<'a> ProgressiveRetriever<'a> {
    pub fn new(
        full_text: &'a dyn RetrievalTier,
        vector: &'a dyn RetrievalTier,
        graph: &'a dyn RetrievalTier,
        rerank: &'a dyn RetrievalTier,
    ) -> Self {
        Self {
            full_text,
            vector,
            graph,
            rerank,
        }
    }

    pub fn retrieve_progressive(
        &self,
        query: &str,
        top_k: u32,
        tier_request: DegradationTier,
        load: &dyn LoadSignal,
    ) -> Result<(Vec<RetrievedItem>, DegradationReport), RetrievalError> {
        let pressure = load.current_pressure();
        let started_at_utc = Utc::now();
        let started = std::time::Instant::now();
        let mut completed: Vec<String> = Vec::new();
        let mut skipped: Vec<String> = Vec::new();
        let mut carry: Vec<RetrievedItem> = Vec::new();

        // Tier policy:
        // - Strict: run only full_text + vector.
        // - Tiered: run all four if pressure < 0.6.
        // - Aggressive: run only full_text if pressure > 0.8; otherwise
        //   full_text + vector.
        let run_full_text = true; // always
        let (run_vector, run_graph, run_rerank) = match tier_request {
            DegradationTier::Strict => (true, false, false),
            DegradationTier::Tiered => {
                let allow_full = pressure < 0.6;
                (true, allow_full, allow_full)
            }
            DegradationTier::Aggressive => {
                let high_pressure = pressure > 0.8;
                (!high_pressure, false, false)
            }
        };

        if run_full_text {
            match self.full_text.execute(query, &carry) {
                Ok(items) => {
                    completed.push(self.full_text.tier_name().to_string());
                    carry = items;
                }
                Err(e) => {
                    skipped.push(self.full_text.tier_name().to_string());
                    // If even full_text fails we can't continue.
                    let _ = e;
                }
            }
        }
        if run_vector {
            match self.vector.execute(query, &carry) {
                Ok(items) => {
                    completed.push(self.vector.tier_name().to_string());
                    carry = items;
                }
                Err(_) => skipped.push(self.vector.tier_name().to_string()),
            }
        } else {
            skipped.push(self.vector.tier_name().to_string());
        }
        if run_graph {
            match self.graph.execute(query, &carry) {
                Ok(items) => {
                    completed.push(self.graph.tier_name().to_string());
                    carry = items;
                }
                Err(_) => skipped.push(self.graph.tier_name().to_string()),
            }
        } else {
            skipped.push(self.graph.tier_name().to_string());
        }
        if run_rerank {
            match self.rerank.execute(query, &carry) {
                Ok(items) => {
                    completed.push(self.rerank.tier_name().to_string());
                    carry = items;
                }
                Err(_) => skipped.push(self.rerank.tier_name().to_string()),
            }
        } else {
            skipped.push(self.rerank.tier_name().to_string());
        }

        if completed.is_empty() {
            return Err(RetrievalError::AllTiersFailed);
        }

        carry.truncate(top_k as usize);
        Ok((
            carry,
            DegradationReport {
                tiers_completed: completed,
                tiers_skipped: skipped,
                load_signal_at_start: pressure,
                started_at_utc,
                total_duration_ms: started.elapsed().as_millis() as u64,
                tier_chosen: tier_request,
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ConstLoad(f64);
    impl LoadSignal for ConstLoad {
        fn current_pressure(&self) -> f64 {
            self.0
        }
    }

    struct StubTier {
        name: &'static str,
        items: Vec<RetrievedItem>,
    }
    impl RetrievalTier for StubTier {
        fn tier_name(&self) -> &'static str {
            self.name
        }
        fn execute(
            &self,
            _query: &str,
            _carry: &[RetrievedItem],
        ) -> Result<Vec<RetrievedItem>, RetrievalError> {
            Ok(self.items.clone())
        }
    }

    fn tier(name: &'static str, n: u32) -> StubTier {
        StubTier {
            name,
            items: (0..n)
                .map(|i| RetrievedItem {
                    item_id: format!("{name}-{i}"),
                    score: 1.0 - (i as f64) * 0.1,
                    tier: name.to_string(),
                })
                .collect(),
        }
    }

    #[test]
    fn tiered_runs_all_tiers_at_low_pressure() {
        let ft = tier(TIER_FULL_TEXT, 3);
        let vec_t = tier(TIER_VECTOR, 3);
        let gr = tier(TIER_GRAPH_EXPANSION, 3);
        let rr = tier(TIER_RERANK, 3);
        let retriever = ProgressiveRetriever::new(&ft, &vec_t, &gr, &rr);
        let (_items, report) = retriever
            .retrieve_progressive("q", 10, DegradationTier::Tiered, &ConstLoad(0.1))
            .unwrap();
        assert_eq!(report.tiers_completed.len(), 4);
        assert!(report.tiers_skipped.is_empty());
    }

    #[test]
    fn tiered_skips_graph_and_rerank_under_pressure() {
        let ft = tier(TIER_FULL_TEXT, 3);
        let vec_t = tier(TIER_VECTOR, 3);
        let gr = tier(TIER_GRAPH_EXPANSION, 3);
        let rr = tier(TIER_RERANK, 3);
        let retriever = ProgressiveRetriever::new(&ft, &vec_t, &gr, &rr);
        let (_items, report) = retriever
            .retrieve_progressive("q", 10, DegradationTier::Tiered, &ConstLoad(0.9))
            .unwrap();
        assert!(report.tiers_completed.contains(&TIER_FULL_TEXT.to_string()));
        assert!(report.tiers_completed.contains(&TIER_VECTOR.to_string()));
        assert!(
            report
                .tiers_skipped
                .contains(&TIER_GRAPH_EXPANSION.to_string())
        );
        assert!(report.tiers_skipped.contains(&TIER_RERANK.to_string()));
    }

    #[test]
    fn strict_runs_only_full_text_and_vector() {
        let ft = tier(TIER_FULL_TEXT, 3);
        let vec_t = tier(TIER_VECTOR, 3);
        let gr = tier(TIER_GRAPH_EXPANSION, 3);
        let rr = tier(TIER_RERANK, 3);
        let retriever = ProgressiveRetriever::new(&ft, &vec_t, &gr, &rr);
        let (_items, report) = retriever
            .retrieve_progressive("q", 10, DegradationTier::Strict, &ConstLoad(0.1))
            .unwrap();
        assert!(report.tiers_completed.contains(&TIER_FULL_TEXT.to_string()));
        assert!(report.tiers_completed.contains(&TIER_VECTOR.to_string()));
        assert!(
            report
                .tiers_skipped
                .contains(&TIER_GRAPH_EXPANSION.to_string())
        );
        assert!(report.tiers_skipped.contains(&TIER_RERANK.to_string()));
    }

    #[test]
    fn aggressive_skips_vector_at_high_pressure() {
        let ft = tier(TIER_FULL_TEXT, 3);
        let vec_t = tier(TIER_VECTOR, 3);
        let gr = tier(TIER_GRAPH_EXPANSION, 3);
        let rr = tier(TIER_RERANK, 3);
        let retriever = ProgressiveRetriever::new(&ft, &vec_t, &gr, &rr);
        let (_items, report) = retriever
            .retrieve_progressive("q", 10, DegradationTier::Aggressive, &ConstLoad(0.95))
            .unwrap();
        assert!(report.tiers_completed.contains(&TIER_FULL_TEXT.to_string()));
        assert!(report.tiers_skipped.contains(&TIER_VECTOR.to_string()));
    }

    #[test]
    fn top_k_caps_output() {
        let ft = tier(TIER_FULL_TEXT, 10);
        let vec_t = tier(TIER_VECTOR, 10);
        let gr = tier(TIER_GRAPH_EXPANSION, 10);
        let rr = tier(TIER_RERANK, 10);
        let retriever = ProgressiveRetriever::new(&ft, &vec_t, &gr, &rr);
        let (items, _r) = retriever
            .retrieve_progressive("q", 3, DegradationTier::Strict, &ConstLoad(0.0))
            .unwrap();
        assert_eq!(items.len(), 3);
    }
}

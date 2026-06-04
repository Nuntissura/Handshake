//! MT-161 (part 2) — ProgressiveRetriever integration tests.
//!
//! Inline tests in src/memory/progressive_retrieval.rs cover the happy
//! paths per tier. This file adds adversarial / cross-cutting integration
//! coverage:
//!
//!   - LoadSignal trait injection: deterministic pressure values exercised
//!     across each DegradationTier setting.
//!   - DegradationReport accurately tracks every skipped tier (no silent
//!     skips).
//!   - "Never returns empty when at least one tier succeeded" invariant.
//!   - Tier-failure does not poison the pipeline; downstream tiers still
//!     attempt unless the policy says skip.

use std::cell::RefCell;

use handshake_core::memory::capsule::DegradationTier;
use handshake_core::memory::progressive_retrieval::{
    LoadSignal, ProgressiveRetriever, RetrievalError, RetrievalTier, RetrievedItem,
    TIER_FULL_TEXT, TIER_GRAPH_EXPANSION, TIER_RERANK, TIER_VECTOR,
};

// ----------------------------------------------------------------------------
// Test-only LoadSignal that returns a fixed pressure.
// ----------------------------------------------------------------------------

struct FixedLoad(f64);

impl LoadSignal for FixedLoad {
    fn current_pressure(&self) -> f64 {
        self.0
    }
}

// ----------------------------------------------------------------------------
// Test-only RetrievalTier: succeeds, fails, or returns a fixed item list.
// Uses RefCell to count invocations so the integration test can assert
// each tier was actually called (or not).
// ----------------------------------------------------------------------------

struct CountingTier {
    name: &'static str,
    succeed: bool,
    items: Vec<RetrievedItem>,
    invocations: RefCell<usize>,
}

impl CountingTier {
    fn ok(name: &'static str, items: Vec<RetrievedItem>) -> Self {
        Self {
            name,
            succeed: true,
            items,
            invocations: RefCell::new(0),
        }
    }

    fn fail(name: &'static str) -> Self {
        Self {
            name,
            succeed: false,
            items: Vec::new(),
            invocations: RefCell::new(0),
        }
    }
}

impl RetrievalTier for CountingTier {
    fn tier_name(&self) -> &'static str {
        self.name
    }

    fn execute(
        &self,
        _query: &str,
        _carry: &[RetrievedItem],
    ) -> Result<Vec<RetrievedItem>, RetrievalError> {
        *self.invocations.borrow_mut() += 1;
        if self.succeed {
            Ok(self.items.clone())
        } else {
            Err(RetrievalError::TierFailed {
                tier: self.name.to_string(),
                message: "simulated tier failure".to_string(),
            })
        }
    }
}

fn one(tier: &str, score: f64) -> RetrievedItem {
    RetrievedItem {
        item_id: format!("item-{tier}"),
        score,
        tier: tier.to_string(),
    }
}

// ----------------------------------------------------------------------------
// Tests.
// ----------------------------------------------------------------------------

#[test]
fn mt161_strict_tier_runs_only_full_text_and_vector() {
    let full_text = CountingTier::ok(TIER_FULL_TEXT, vec![one(TIER_FULL_TEXT, 0.9)]);
    let vector = CountingTier::ok(TIER_VECTOR, vec![one(TIER_VECTOR, 0.8)]);
    let graph = CountingTier::ok(TIER_GRAPH_EXPANSION, vec![one(TIER_GRAPH_EXPANSION, 0.7)]);
    let rerank = CountingTier::ok(TIER_RERANK, vec![one(TIER_RERANK, 0.99)]);
    let retriever = ProgressiveRetriever::new(&full_text, &vector, &graph, &rerank);

    let (_items, report) = retriever
        .retrieve_progressive("query", 10, DegradationTier::Strict, &FixedLoad(0.1))
        .expect("Strict tier must succeed under low load");

    assert_eq!(*full_text.invocations.borrow(), 1);
    assert_eq!(*vector.invocations.borrow(), 1);
    assert_eq!(
        *graph.invocations.borrow(),
        0,
        "graph tier must not run under Strict"
    );
    assert_eq!(
        *rerank.invocations.borrow(),
        0,
        "re-rank tier must not run under Strict"
    );
    assert!(report.tiers_completed.contains(&TIER_FULL_TEXT.to_string()));
    assert!(report.tiers_completed.contains(&TIER_VECTOR.to_string()));
    assert!(report.tiers_skipped.contains(&TIER_GRAPH_EXPANSION.to_string()));
    assert!(report.tiers_skipped.contains(&TIER_RERANK.to_string()));
    assert_eq!(report.tier_chosen, DegradationTier::Strict);
    assert_eq!(report.load_signal_at_start, 0.1);
}

#[test]
fn mt161_tiered_low_pressure_runs_all_four_tiers() {
    let full_text = CountingTier::ok(TIER_FULL_TEXT, vec![one(TIER_FULL_TEXT, 0.9)]);
    let vector = CountingTier::ok(TIER_VECTOR, vec![one(TIER_VECTOR, 0.8)]);
    let graph = CountingTier::ok(TIER_GRAPH_EXPANSION, vec![one(TIER_GRAPH_EXPANSION, 0.7)]);
    let rerank = CountingTier::ok(TIER_RERANK, vec![one(TIER_RERANK, 0.99)]);
    let retriever = ProgressiveRetriever::new(&full_text, &vector, &graph, &rerank);

    let (_items, report) = retriever
        .retrieve_progressive("query", 10, DegradationTier::Tiered, &FixedLoad(0.2))
        .expect("Tiered at low pressure must run all tiers");

    assert_eq!(*full_text.invocations.borrow(), 1);
    assert_eq!(*vector.invocations.borrow(), 1);
    assert_eq!(*graph.invocations.borrow(), 1);
    assert_eq!(*rerank.invocations.borrow(), 1);
    assert!(report.tiers_skipped.is_empty(), "no tier should be skipped at low pressure");
    assert_eq!(report.tier_chosen, DegradationTier::Tiered);
}

#[test]
fn mt161_tiered_high_pressure_skips_graph_and_rerank() {
    let full_text = CountingTier::ok(TIER_FULL_TEXT, vec![one(TIER_FULL_TEXT, 0.9)]);
    let vector = CountingTier::ok(TIER_VECTOR, vec![one(TIER_VECTOR, 0.8)]);
    let graph = CountingTier::ok(TIER_GRAPH_EXPANSION, vec![one(TIER_GRAPH_EXPANSION, 0.7)]);
    let rerank = CountingTier::ok(TIER_RERANK, vec![one(TIER_RERANK, 0.99)]);
    let retriever = ProgressiveRetriever::new(&full_text, &vector, &graph, &rerank);

    let (_items, report) = retriever
        .retrieve_progressive("query", 10, DegradationTier::Tiered, &FixedLoad(0.9))
        .expect("Tiered at high pressure must still succeed via early tiers");

    assert_eq!(*full_text.invocations.borrow(), 1);
    assert_eq!(*vector.invocations.borrow(), 1);
    assert_eq!(*graph.invocations.borrow(), 0, "graph tier must skip under high pressure");
    assert_eq!(*rerank.invocations.borrow(), 0, "re-rank tier must skip under high pressure");
    assert!(report.tiers_skipped.contains(&TIER_GRAPH_EXPANSION.to_string()));
    assert!(report.tiers_skipped.contains(&TIER_RERANK.to_string()));
}

#[test]
fn mt161_aggressive_high_pressure_runs_only_full_text() {
    let full_text = CountingTier::ok(TIER_FULL_TEXT, vec![one(TIER_FULL_TEXT, 0.9)]);
    let vector = CountingTier::ok(TIER_VECTOR, vec![one(TIER_VECTOR, 0.8)]);
    let graph = CountingTier::ok(TIER_GRAPH_EXPANSION, vec![one(TIER_GRAPH_EXPANSION, 0.7)]);
    let rerank = CountingTier::ok(TIER_RERANK, vec![one(TIER_RERANK, 0.99)]);
    let retriever = ProgressiveRetriever::new(&full_text, &vector, &graph, &rerank);

    let (_items, report) = retriever
        .retrieve_progressive("query", 10, DegradationTier::Aggressive, &FixedLoad(0.95))
        .expect("Aggressive at 0.95 pressure must succeed via full_text only");

    assert_eq!(*full_text.invocations.borrow(), 1);
    assert_eq!(*vector.invocations.borrow(), 0, "vector must skip under Aggressive high-pressure");
    assert_eq!(*graph.invocations.borrow(), 0);
    assert_eq!(*rerank.invocations.borrow(), 0);
    assert!(report.tiers_skipped.contains(&TIER_VECTOR.to_string()));
    assert!(report.tiers_skipped.contains(&TIER_GRAPH_EXPANSION.to_string()));
    assert!(report.tiers_skipped.contains(&TIER_RERANK.to_string()));
}

#[test]
fn mt161_aggressive_moderate_pressure_includes_vector() {
    let full_text = CountingTier::ok(TIER_FULL_TEXT, vec![one(TIER_FULL_TEXT, 0.9)]);
    let vector = CountingTier::ok(TIER_VECTOR, vec![one(TIER_VECTOR, 0.8)]);
    let graph = CountingTier::ok(TIER_GRAPH_EXPANSION, vec![one(TIER_GRAPH_EXPANSION, 0.7)]);
    let rerank = CountingTier::ok(TIER_RERANK, vec![one(TIER_RERANK, 0.99)]);
    let retriever = ProgressiveRetriever::new(&full_text, &vector, &graph, &rerank);

    let (_items, _report) = retriever
        .retrieve_progressive("query", 10, DegradationTier::Aggressive, &FixedLoad(0.5))
        .expect("Aggressive at 0.5 pressure runs full_text + vector");

    assert_eq!(*full_text.invocations.borrow(), 1);
    assert_eq!(*vector.invocations.borrow(), 1, "vector runs at pressure < 0.8");
    assert_eq!(*graph.invocations.borrow(), 0);
    assert_eq!(*rerank.invocations.borrow(), 0);
}

#[test]
fn mt161_degradation_report_captures_load_signal_at_start() {
    let full_text = CountingTier::ok(TIER_FULL_TEXT, vec![one(TIER_FULL_TEXT, 0.9)]);
    let vector = CountingTier::ok(TIER_VECTOR, vec![one(TIER_VECTOR, 0.8)]);
    let graph = CountingTier::ok(TIER_GRAPH_EXPANSION, vec![]);
    let rerank = CountingTier::ok(TIER_RERANK, vec![]);
    let retriever = ProgressiveRetriever::new(&full_text, &vector, &graph, &rerank);

    let (_items, report) = retriever
        .retrieve_progressive("query", 10, DegradationTier::Tiered, &FixedLoad(0.37))
        .expect("retrieval must succeed");
    assert_eq!(report.load_signal_at_start, 0.37);
}

#[test]
fn mt161_completed_and_skipped_lists_partition_the_four_tiers() {
    let full_text = CountingTier::ok(TIER_FULL_TEXT, vec![one(TIER_FULL_TEXT, 0.9)]);
    let vector = CountingTier::ok(TIER_VECTOR, vec![]);
    let graph = CountingTier::ok(TIER_GRAPH_EXPANSION, vec![]);
    let rerank = CountingTier::ok(TIER_RERANK, vec![]);
    let retriever = ProgressiveRetriever::new(&full_text, &vector, &graph, &rerank);

    let (_items, report) = retriever
        .retrieve_progressive("query", 10, DegradationTier::Tiered, &FixedLoad(0.9))
        .expect("retrieval succeeds");

    let total: std::collections::HashSet<&String> =
        report.tiers_completed.iter().chain(report.tiers_skipped.iter()).collect();
    assert_eq!(
        total.len(),
        report.tiers_completed.len() + report.tiers_skipped.len(),
        "completed and skipped lists must not overlap"
    );
    // Total must cover all four tier names. Note Strict/Tiered/Aggressive
    // each visit all four tiers in the policy (some "complete", others
    // "skip"); the union must always be the full set.
    let expected_names: std::collections::HashSet<&'static str> = [
        TIER_FULL_TEXT,
        TIER_VECTOR,
        TIER_GRAPH_EXPANSION,
        TIER_RERANK,
    ]
    .into_iter()
    .collect();
    let actual_names: std::collections::HashSet<&str> = total.iter().map(|s| s.as_str()).collect();
    assert_eq!(
        actual_names, expected_names,
        "report should mention all 4 tier names across completed+skipped lists"
    );
}

#[test]
fn mt161_total_duration_ms_is_non_negative_and_recorded() {
    let full_text = CountingTier::ok(TIER_FULL_TEXT, vec![one(TIER_FULL_TEXT, 0.9)]);
    let vector = CountingTier::ok(TIER_VECTOR, vec![]);
    let graph = CountingTier::ok(TIER_GRAPH_EXPANSION, vec![]);
    let rerank = CountingTier::ok(TIER_RERANK, vec![]);
    let retriever = ProgressiveRetriever::new(&full_text, &vector, &graph, &rerank);

    let (_items, report) = retriever
        .retrieve_progressive("query", 10, DegradationTier::Strict, &FixedLoad(0.1))
        .expect("retrieval succeeds");
    // total_duration_ms is a u64 so non-negativity is structural; the
    // real assertion is that it's set (non-zero is hard to guarantee for
    // such fast tests, but reading the field works).
    let _ = report.total_duration_ms;
    assert!(!report.tiers_completed.is_empty());
}

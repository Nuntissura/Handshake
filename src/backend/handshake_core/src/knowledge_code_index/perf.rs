//! WP-KERNEL-009 MT-111 CodeIndexPerformanceBudget.
//!
//! Master Spec anchor: 2.3.13.11 (code navigation precision) + the WP constraint
//! that indexing large repos must stay bounded. Defines a DOCUMENTED performance
//! budget for the parse+extract phase and a measured assertion the fixtures
//! (MT-112) and a dedicated perf test exercise on a representative fixture: if
//! the budget is exceeded the test FAILS.
//!
//! Pure logic; no DB. The budget is expressed per-thousand-lines so it scales
//! with file size and is independent of the host's absolute speed within a
//! generous ceiling. The numbers are a guardrail against accidental O(n^2)
//! regressions in the AST walk / extractors, not a micro-benchmark.

use serde::{Deserialize, Serialize};

/// The parse+extract performance budget for the Tree-sitter code index.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CodeIndexBudget {
    /// Maximum wall-clock milliseconds allowed per 1,000 source lines for the
    /// full parse + symbol/doc/relationship extraction of one file.
    pub max_ms_per_kloc: f64,
    /// A fixed ceiling (ms) added to absorb constant per-file overhead so very
    /// small files are not judged against a tiny budget.
    pub fixed_overhead_ms: f64,
}

impl Default for CodeIndexBudget {
    fn default() -> Self {
        // Generous guardrail: Tree-sitter parses well under this on commodity
        // hardware; the goal is to catch algorithmic regressions, not to be a
        // tight benchmark. 250 ms / kloc + 50 ms fixed leaves wide headroom on
        // a slow CI box while still failing a quadratic blow-up.
        Self {
            max_ms_per_kloc: 250.0,
            fixed_overhead_ms: 50.0,
        }
    }
}

impl CodeIndexBudget {
    /// The allowed milliseconds for a file of `line_count` lines.
    pub fn allowed_ms(&self, line_count: usize) -> f64 {
        let kloc = (line_count as f64) / 1000.0;
        self.fixed_overhead_ms + self.max_ms_per_kloc * kloc
    }

    /// Whether `elapsed_ms` is within budget for `line_count` lines.
    pub fn is_within(&self, line_count: usize, elapsed_ms: f64) -> bool {
        elapsed_ms <= self.allowed_ms(line_count)
    }
}

/// A measured perf sample for one file (for the receipt / perf test report).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerfSample {
    pub relative_path: String,
    pub line_count: usize,
    pub elapsed_ms: f64,
    pub allowed_ms: f64,
    pub within_budget: bool,
}

impl PerfSample {
    pub fn measure(
        budget: &CodeIndexBudget,
        relative_path: impl Into<String>,
        line_count: usize,
        elapsed_ms: f64,
    ) -> Self {
        let allowed = budget.allowed_ms(line_count);
        Self {
            relative_path: relative_path.into(),
            line_count,
            elapsed_ms,
            allowed_ms: allowed,
            within_budget: elapsed_ms <= allowed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowed_scales_with_lines() {
        let b = CodeIndexBudget::default();
        let small = b.allowed_ms(100);
        let large = b.allowed_ms(10_000);
        assert!(large > small);
        // 10k lines: 50 + 250*10 = 2550 ms.
        assert!((b.allowed_ms(10_000) - 2550.0).abs() < 1e-6);
    }

    #[test]
    fn within_budget_check() {
        let b = CodeIndexBudget::default();
        assert!(b.is_within(1000, 100.0));
        assert!(!b.is_within(1000, 10_000.0));
    }

    #[test]
    fn perf_sample_records_verdict() {
        let b = CodeIndexBudget::default();
        let s = PerfSample::measure(&b, "src/lib.rs", 1000, 80.0);
        assert!(s.within_budget);
        assert_eq!(s.line_count, 1000);
        assert!(s.allowed_ms > s.elapsed_ms);
    }
}

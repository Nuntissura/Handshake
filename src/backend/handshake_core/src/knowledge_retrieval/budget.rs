//! WP-KERNEL-009 MT-137 ContextBudgetPolicy.
//!
//! Spec 2.6.6.7.14.5 `RetrievalBudgets` + 2.3.13.11 ("context bundles MUST be
//! bounded"): define token/size budgets, truncation rules, priority tiers, and
//! WHY each dropped item was dropped. A bundle that exceeds budget MUST drop
//! lower-priority evidence and record the drop (`excluded_budget`) rather than
//! silently truncating or overflowing.
//!
//! This is a pure, deterministic allocator: given ranked items (each with a
//! token cost and a priority tier) and a [`RetrievalBudgets`], it returns the
//! admitted items in order plus a structured drop ledger. The bundle compiler
//! (MT-136) feeds the ledger into the per-item `retrieval_decision`.

use crate::knowledge_retrieval::plan::RetrievalBudgets;

/// Priority tier of a candidate item. Higher tiers are admitted first; within a
/// tier, input order (already ranked by MT-134) is preserved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PriorityTier {
    /// Authoritative direct/exact loads — never dropped for budget unless they
    /// alone exceed the hard token ceiling.
    Authoritative,
    /// High-confidence graph/lexical evidence.
    Primary,
    /// Supplementary recall (vector / lower-confidence).
    Supplementary,
}

impl PriorityTier {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Authoritative => "authoritative",
            Self::Primary => "primary",
            Self::Supplementary => "supplementary",
        }
    }

    /// Admission order key: authoritative (0) before primary (1) before
    /// supplementary (2).
    fn order_key(self) -> u8 {
        match self {
            Self::Authoritative => 0,
            Self::Primary => 1,
            Self::Supplementary => 2,
        }
    }
}

/// One item presented to the budget allocator.
#[derive(Debug, Clone, PartialEq)]
pub struct BudgetItem {
    pub item_id: String,
    pub tier: PriorityTier,
    pub token_count: u32,
    /// Source id this item came from, for the per-source snippet cap.
    pub source_id: String,
}

/// Why an item was dropped (maps onto the spec
/// `knowledge_context_bundle_items.retrieval_decision` excluded reasons +
/// truncation flags).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DropReason {
    /// The total evidence-token budget was exhausted.
    TokenBudgetExhausted,
    /// The total snippet count cap was reached.
    SnippetCountExhausted,
    /// The per-source snippet cap for this item's source was reached.
    PerSourceCapReached,
}

impl DropReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TokenBudgetExhausted => "token_budget_exhausted",
            Self::SnippetCountExhausted => "snippet_count_exhausted",
            Self::PerSourceCapReached => "per_source_cap_reached",
        }
    }
}

/// One dropped item with its reason — the "why-dropped" ledger entry.
#[derive(Debug, Clone, PartialEq)]
pub struct DroppedItem {
    pub item_id: String,
    pub tier: PriorityTier,
    pub reason: DropReason,
}

/// The allocation outcome: admitted item ids (in admission order), the drop
/// ledger, and the totals actually used.
#[derive(Debug, Clone, PartialEq)]
pub struct BudgetAllocation {
    pub admitted: Vec<String>,
    pub dropped: Vec<DroppedItem>,
    pub tokens_used: u32,
    pub snippets_used: u32,
    /// Truncation flags surfaced in the trace (spec `truncation_flags[]`).
    pub truncation_flags: Vec<String>,
}

impl BudgetAllocation {
    pub fn is_admitted(&self, item_id: &str) -> bool {
        self.admitted.iter().any(|id| id == item_id)
    }
}

/// Allocate items against a budget. Deterministic: items are admitted in tier
/// order then input order. Each admission is checked against the total token
/// budget, the total snippet cap, and the per-source snippet cap; the first
/// failing check drops the item with the matching reason.
pub fn allocate(items: &[BudgetItem], budgets: &RetrievalBudgets) -> BudgetAllocation {
    // Stable sort by tier (authoritative first); input order preserved within
    // a tier because sort_by is stable.
    let mut ordered: Vec<&BudgetItem> = items.iter().collect();
    ordered.sort_by_key(|item| item.tier.order_key());

    let mut admitted = Vec::new();
    let mut dropped = Vec::new();
    let mut tokens_used: u32 = 0;
    let mut snippets_used: u32 = 0;
    let mut truncation_flags: Vec<String> = Vec::new();
    let mut per_source_counts: std::collections::HashMap<String, u32> =
        std::collections::HashMap::new();

    for item in ordered {
        let source_count = per_source_counts.get(&item.source_id).copied().unwrap_or(0);

        if snippets_used >= budgets.max_snippets_total {
            dropped.push(DroppedItem {
                item_id: item.item_id.clone(),
                tier: item.tier,
                reason: DropReason::SnippetCountExhausted,
            });
            continue;
        }
        if source_count >= budgets.max_snippets_per_source {
            dropped.push(DroppedItem {
                item_id: item.item_id.clone(),
                tier: item.tier,
                reason: DropReason::PerSourceCapReached,
            });
            continue;
        }
        if tokens_used.saturating_add(item.token_count) > budgets.max_total_evidence_tokens {
            dropped.push(DroppedItem {
                item_id: item.item_id.clone(),
                tier: item.tier,
                reason: DropReason::TokenBudgetExhausted,
            });
            continue;
        }

        admitted.push(item.item_id.clone());
        tokens_used = tokens_used.saturating_add(item.token_count);
        snippets_used += 1;
        *per_source_counts.entry(item.source_id.clone()).or_insert(0) += 1;
    }

    if dropped
        .iter()
        .any(|d| d.reason == DropReason::TokenBudgetExhausted)
    {
        truncation_flags.push("evidence_token_budget_exhausted".to_string());
    }
    if dropped
        .iter()
        .any(|d| d.reason == DropReason::SnippetCountExhausted)
    {
        truncation_flags.push("snippet_count_cap_reached".to_string());
    }

    BudgetAllocation {
        admitted,
        dropped,
        tokens_used,
        snippets_used,
        truncation_flags,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, tier: PriorityTier, tokens: u32, source: &str) -> BudgetItem {
        BudgetItem {
            item_id: id.to_string(),
            tier,
            token_count: tokens,
            source_id: source.to_string(),
        }
    }

    fn budget(tokens: u32, snippets: u32, per_source: u32) -> RetrievalBudgets {
        RetrievalBudgets {
            max_total_evidence_tokens: tokens,
            max_snippets_total: snippets,
            max_snippets_per_source: per_source,
            max_candidates_total: 100,
            max_read_tokens: 1000,
            max_tool_calls: 8,
            max_rerank_candidates: 64,
            tool_delta_inline_char_limit: 2000,
        }
    }

    #[test]
    fn authoritative_admitted_before_supplementary() {
        let items = vec![
            item("supp", PriorityTier::Supplementary, 100, "s1"),
            item("auth", PriorityTier::Authoritative, 100, "s2"),
        ];
        // Budget for only one item.
        let alloc = allocate(&items, &budget(100, 10, 10));
        assert_eq!(alloc.admitted, vec!["auth"]);
        assert_eq!(alloc.dropped.len(), 1);
        assert_eq!(alloc.dropped[0].item_id, "supp");
        assert_eq!(alloc.dropped[0].reason, DropReason::TokenBudgetExhausted);
    }

    #[test]
    fn token_budget_exhaustion_drops_and_flags() {
        let items = vec![
            item("a", PriorityTier::Primary, 60, "s1"),
            item("b", PriorityTier::Primary, 60, "s2"),
        ];
        let alloc = allocate(&items, &budget(100, 10, 10));
        assert_eq!(alloc.admitted, vec!["a"]);
        assert_eq!(alloc.tokens_used, 60);
        assert!(alloc
            .truncation_flags
            .contains(&"evidence_token_budget_exhausted".to_string()));
    }

    #[test]
    fn per_source_cap_drops_extra_from_same_source() {
        let items = vec![
            item("a", PriorityTier::Primary, 10, "s1"),
            item("b", PriorityTier::Primary, 10, "s1"),
            item("c", PriorityTier::Primary, 10, "s2"),
        ];
        // Per-source cap of 1: only one from s1.
        let alloc = allocate(&items, &budget(1000, 10, 1));
        assert!(alloc.is_admitted("a"));
        assert!(!alloc.is_admitted("b"));
        assert!(alloc.is_admitted("c"));
        assert_eq!(
            alloc
                .dropped
                .iter()
                .find(|d| d.item_id == "b")
                .unwrap()
                .reason,
            DropReason::PerSourceCapReached
        );
    }

    #[test]
    fn snippet_count_cap_reached() {
        let items = vec![
            item("a", PriorityTier::Primary, 10, "s1"),
            item("b", PriorityTier::Primary, 10, "s2"),
            item("c", PriorityTier::Primary, 10, "s3"),
        ];
        let alloc = allocate(&items, &budget(1000, 2, 10));
        assert_eq!(alloc.snippets_used, 2);
        assert!(alloc
            .truncation_flags
            .contains(&"snippet_count_cap_reached".to_string()));
    }
}

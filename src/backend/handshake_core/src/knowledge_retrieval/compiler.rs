//! WP-KERNEL-009 MT-136 ContextBundleCompilerV2.
//!
//! Spec 2.3.13.11 ("context bundles MUST be bounded, cited, explainable,
//! restartable, and inspectable") + the kernel ContextBundle V1 shape
//! (`kernel/context_bundle.rs`). Compile a BOUNDED context bundle for a
//! retrieval target — task, file, symbol, WP, MT, spec topic, rich doc, Loom
//! block, media asset, or UserManual page — by running the
//! plan -> rank -> budget -> snippet pipeline and persisting:
//!   1. the kernel ContextBundle V1 (id = `CTX-` + 16 hex of the content hash),
//!      its per-item retrieval decisions, token budget/used, via
//!      `KnowledgeStore::record_knowledge_context_bundle` (table 0141), and
//!   2. the replayable RetrievalTrace bound to that bundle, via
//!      `storage/knowledge_retrieval.rs` (MT-138).
//!
//! "V2" over the V1 builder: V1 (`kernel/context_bundle.rs::ContextBundle`)
//! produces a content-hashed bundle from an `allowed_context` JSON. V2 wraps it
//! with the WP-009 retrieval pipeline so the bundle's items are RANKED + BUDGETED
//! cited memory passages/spans, and every build leaves a QueryPlan +
//! RetrievalTrace. The V1 bundle is persisted exactly (no reshaping), preserving
//! kernel compatibility.
//!
//! Authority: PostgreSQL/EventLedger. The compiled JSON bundle is a projection.

use serde_json::{json, Value};

use crate::kernel::context_bundle::ContextBundle;
use crate::knowledge_retrieval::budget::{allocate, BudgetAllocation, BudgetItem, PriorityTier};
use crate::knowledge_retrieval::plan::{QueryPlan, RetrievalTrace, RouteTaken, SelectedRef};
use crate::knowledge_retrieval::snippet::EvidenceSnippet;
use crate::storage::knowledge::{
    KnowledgeBundleItemDecision, KnowledgeBundleItemRefKind, KnowledgeStore,
    NewKnowledgeContextBundle, NewKnowledgeContextBundleItem,
};
use crate::storage::knowledge_retrieval::record_retrieval_trace;
use crate::storage::postgres::PostgresDatabase;
use crate::storage::{StorageError, StorageResult};

/// What a bundle is being compiled FOR (spec MT-136 target list). The kind is
/// recorded in the bundle's `allowed_context` so a consumer knows the bundle's
/// subject.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BundleTargetKind {
    Task,
    File,
    Symbol,
    WorkPacket,
    MicroTask,
    SpecTopic,
    RichDocument,
    LoomBlock,
    MediaAsset,
    UserManualPage,
}

impl BundleTargetKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Task => "task",
            Self::File => "file",
            Self::Symbol => "symbol",
            Self::WorkPacket => "work_packet",
            Self::MicroTask => "micro_task",
            Self::SpecTopic => "spec_topic",
            Self::RichDocument => "rich_document",
            Self::LoomBlock => "loom_block",
            Self::MediaAsset => "media_asset",
            Self::UserManualPage => "user_manual_page",
        }
    }
}

/// A ranked, budget-classified candidate ready for the bundle. The compiler is
/// fed these (produced upstream by the ranking MT-134 + snippet MT-135 steps)
/// so the compiler itself stays a deterministic assembler.
#[derive(Debug, Clone, PartialEq)]
pub struct BundleCandidate {
    pub ref_kind: KnowledgeBundleItemRefKind,
    pub ref_id: String,
    pub tier: PriorityTier,
    pub token_count: u32,
    pub relevance_score: f64,
    /// The source id used for the per-source snippet cap.
    pub source_id: String,
    /// The assembled evidence snippet (for the citation), if any.
    pub snippet: Option<EvidenceSnippet>,
}

/// The result of a compile: the persisted bundle id, the trace id, and the
/// budget allocation (so the caller can surface the why-dropped ledger).
#[derive(Debug, Clone, PartialEq)]
pub struct CompiledBundle {
    pub bundle_id: String,
    pub trace_id: String,
    pub allocation: BudgetAllocation,
    pub tokens_used: u32,
}

/// The bundle compiler.
pub struct ContextBundleCompilerV2<'a> {
    db: &'a PostgresDatabase,
}

impl<'a> ContextBundleCompilerV2<'a> {
    pub fn new(db: &'a PostgresDatabase) -> Self {
        Self { db }
    }

    /// Compile + persist a bounded bundle and its trace.
    ///
    /// * `workspace_id` — FK target for both tables.
    /// * `kernel_task_run_id` / `session_run_id` — the kernel V1 bundle keys.
    /// * `target_kind` / `target_ref` — what the bundle is about.
    /// * `plan` / `trace` — the QueryPlan + RetrievalTrace from the planners.
    /// * `candidates` — ranked candidates to admit under the plan's budget.
    /// * `build_receipt_event_id` / `trace_receipt_event_id` — EventLedger refs.
    #[allow(clippy::too_many_arguments)]
    pub async fn compile(
        &self,
        workspace_id: &str,
        kernel_task_run_id: &str,
        session_run_id: &str,
        target_kind: BundleTargetKind,
        target_ref: &str,
        plan: &QueryPlan,
        trace: &mut RetrievalTrace,
        candidates: &[BundleCandidate],
        build_receipt_event_id: Option<String>,
        trace_receipt_event_id: Option<String>,
    ) -> StorageResult<CompiledBundle> {
        plan.validate().map_err(StorageError::Validation)?;

        // 1. Budget-allocate the candidates (deterministic, tier-ordered).
        let budget_items: Vec<BudgetItem> = candidates
            .iter()
            .map(|c| BudgetItem {
                item_id: c.ref_id.clone(),
                tier: c.tier,
                token_count: c.token_count,
                source_id: c.source_id.clone(),
            })
            .collect();
        let allocation = allocate(&budget_items, &plan.budgets);

        // 2. Build the kernel V1 allowed_context (the bounded, cited projection).
        let admitted_snippets: Vec<Value> = candidates
            .iter()
            .filter(|c| allocation.is_admitted(&c.ref_id))
            .map(|c| {
                let supported = c.snippet.as_ref().map(|s| s.supported).unwrap_or(true);
                let unsupported_reason = c
                    .snippet
                    .as_ref()
                    .and_then(|s| s.unsupported_reason.clone());
                json!({
                    "ref_kind": c.ref_kind.as_str(),
                    "ref_id": c.ref_id,
                    "relevance_score": c.relevance_score,
                    "citation": c.snippet.as_ref().map(|s| s.citation()),
                    "supported": supported,
                    "unsupported_reason": unsupported_reason,
                })
            })
            .collect();
        let allowed_context = json!({
            "schema": "hsk.context_bundle_v2@1",
            "target": {"kind": target_kind.as_str(), "ref": target_ref},
            "query_plan_id": plan.plan_id,
            "retrieval_mode": plan.retrieval_mode.to_storage_str(),
            "items": admitted_snippets,
            "tokens_used": allocation.tokens_used,
            "token_budget": plan.budgets.max_total_evidence_tokens,
        });

        // 3. Construct the kernel V1 bundle (content-hashed id) — persisted as-is.
        let bundle = ContextBundle::new(kernel_task_run_id, session_run_id, allowed_context)
            .map_err(|err| StorageError::Validation(kernel_err_str(err)))?;

        // 4. Per-item decisions (included / excluded_*), in candidate order.
        let items: Vec<NewKnowledgeContextBundleItem> = candidates
            .iter()
            .map(|c| {
                let decision = item_decision(&allocation, &c.ref_id);
                let supported = c.snippet.as_ref().map(|s| s.supported).unwrap_or(true);
                let unsupported_reason = c
                    .snippet
                    .as_ref()
                    .and_then(|s| s.unsupported_reason.clone());
                NewKnowledgeContextBundleItem {
                    ref_kind: c.ref_kind,
                    ref_id: c.ref_id.clone(),
                    retrieval_decision: decision,
                    relevance_score: Some(c.relevance_score.clamp(0.0, 1.0)),
                    token_count: Some(c.token_count as i32),
                    citation: c.snippet.as_ref().map(|s| s.citation()),
                    supported,
                    unsupported_reason,
                }
            })
            .collect();

        // 5. Persist the bundle + items (committed store).
        let stored = self
            .db
            .record_knowledge_context_bundle(NewKnowledgeContextBundle {
                workspace_id: workspace_id.to_string(),
                bundle,
                query_text: Some(plan.query_text.clone()),
                token_budget: Some(plan.budgets.max_total_evidence_tokens as i32),
                tokens_used: Some(allocation.tokens_used as i32),
                build_receipt_event_id,
                items,
            })
            .await?;

        // 6. Finalize the trace: selected refs + truncation flags from the
        //    allocation, bound to the bundle, then persist (MT-138).
        for (rank, candidate) in candidates
            .iter()
            .filter(|c| allocation.is_admitted(&c.ref_id))
            .enumerate()
        {
            trace.selected.push(SelectedRef {
                candidate_id: candidate.ref_id.clone(),
                final_rank: rank as u32,
                final_score: candidate.relevance_score,
                why: format!(
                    "admitted under {} budget tier {}",
                    plan.retrieval_mode.to_storage_str(),
                    candidate.tier.as_str()
                ),
            });
        }
        for flag in &allocation.truncation_flags {
            if !trace.truncation_flags.contains(flag) {
                trace.truncation_flags.push(flag.clone());
            }
        }
        if trace.route_taken.is_empty() {
            // Reflect the planned route as taken when the caller didn't record
            // execution steps (keeps the trace self-describing).
            for step in &plan.route {
                trace.route_taken.push(RouteTaken {
                    store: step.store,
                    reason: step.purpose.clone(),
                    cache_hit: false,
                });
            }
        }

        let stored_trace = record_retrieval_trace(
            self.db,
            workspace_id,
            plan,
            trace,
            Some(stored.bundle_id.clone()),
            trace_receipt_event_id,
        )
        .await?;

        Ok(CompiledBundle {
            bundle_id: stored.bundle_id,
            trace_id: stored_trace.trace_id,
            tokens_used: allocation.tokens_used,
            allocation,
        })
    }
}

/// Map the budget allocation to a per-item retrieval decision.
fn item_decision(allocation: &BudgetAllocation, ref_id: &str) -> KnowledgeBundleItemDecision {
    if allocation.is_admitted(ref_id) {
        return KnowledgeBundleItemDecision::Included;
    }
    // Find the drop reason; budget vs per-source/snippet caps both map to the
    // spec `excluded_budget` decision (the bundle items enum has no
    // per-source-specific value; the precise reason is in the trace).
    KnowledgeBundleItemDecision::ExcludedBudget
}

fn kernel_err_str(err: crate::kernel::KernelError) -> &'static str {
    match err {
        crate::kernel::KernelError::InvalidEvent(msg) => msg,
        _ => "kernel context bundle construction failed",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_kinds_have_stable_strings() {
        assert_eq!(BundleTargetKind::WorkPacket.as_str(), "work_packet");
        assert_eq!(
            BundleTargetKind::UserManualPage.as_str(),
            "user_manual_page"
        );
        assert_eq!(BundleTargetKind::Symbol.as_str(), "symbol");
    }

    #[test]
    fn admitted_item_decision_is_included() {
        let allocation = BudgetAllocation {
            admitted: vec!["a".to_string()],
            dropped: vec![],
            tokens_used: 10,
            snippets_used: 1,
            truncation_flags: vec![],
        };
        assert_eq!(
            item_decision(&allocation, "a"),
            KnowledgeBundleItemDecision::Included
        );
        assert_eq!(
            item_decision(&allocation, "b"),
            KnowledgeBundleItemDecision::ExcludedBudget
        );
    }
}

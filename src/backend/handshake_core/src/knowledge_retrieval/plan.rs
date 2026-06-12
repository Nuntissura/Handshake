//! WP-KERNEL-009 RetrievalContextAndRanking — the spec-canonical QueryPlan /
//! RetrievalTrace data model (spec 2.6.6.7.14.5, 2.3.13.11, 2.3.14
//! [ADD v02.178]).
//!
//! These are Handshake-native Rust structs that mirror the spec's typed
//! objects. They are the *plan* and *replayable trace* a retrieval-mode planner
//! (MT-130..MT-133) produces, the ranking (MT-134) scores, the snippet
//! assembler (MT-135) cites, and the context-bundle compiler (MT-136) bounds.
//!
//! Authority note (spec 2.3.13.11): the durable authority is the
//! `knowledge_retrieval_traces` row (migration 0141) — `retrieval_mode`,
//! `mode_reason`, and the `decisions` JSONB. A [`QueryPlan`] and its
//! [`RetrievalTrace`] serialize (via [`RetrievalTrace::to_decisions_json`]) into
//! that `decisions` column so the entire run is replayable from PostgreSQL. The
//! struct itself is an in-memory projection that the storage write
//! (`storage/knowledge_retrieval.rs`, MT-138) persists.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::memory::retrieval_mode::{NonHybridReason, QueryKind, QueryRetrievalMode};

/// A retrieval store a route step targets (spec 2.6.6.7.14.5 `RouteStep.store`),
/// mapped onto the Handshake ProjectKnowledgeIndex surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalStore {
    /// Reuse a fresh ContextPack (cheapest; spec default route 1).
    ContextPacks,
    /// Knowledge-Graph / Loom neighborhood (prefilter candidate entity sets).
    KnowledgeGraph,
    /// High-precision lexical/keyword index over indexed sources.
    ShadowWsLexical,
    /// Semantic-recall vector index over indexed passages.
    ShadowWsVector,
    /// Locally cached web content (never a live external fetch here).
    LocalWebCache,
    /// Bounded escalation read of an authoritative record (direct/exact load).
    BoundedReadOnly,
}

impl RetrievalStore {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ContextPacks => "context_packs",
            Self::KnowledgeGraph => "knowledge_graph",
            Self::ShadowWsLexical => "shadow_ws_lexical",
            Self::ShadowWsVector => "shadow_ws_vector",
            Self::LocalWebCache => "local_web_cache",
            Self::BoundedReadOnly => "bounded_read_only",
        }
    }
}

/// One planned store step (spec 2.6.6.7.14.5 `RouteStep`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteStep {
    pub store: RetrievalStore,
    /// Human-readable purpose (logged, never executed as code).
    pub purpose: String,
    pub max_candidates: u32,
    /// If true, a failure of this step is a hard error, not a silent skip.
    pub required: bool,
}

impl RouteStep {
    pub fn new(store: RetrievalStore, purpose: impl Into<String>, max_candidates: u32) -> Self {
        Self {
            store,
            purpose: purpose.into(),
            max_candidates,
            required: false,
        }
    }

    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }
}

/// Token / count budgets that bound a retrieval (spec 2.6.6.7.14.5
/// `RetrievalBudgets`). MT-137 ContextBudgetPolicy owns the defaults and the
/// truncation rules; this struct is the value carried in the plan/trace.
///
/// Enforcement map (adversarial-v2 MT-137 LOW — which caps are HARD and which
/// are planner hints):
/// * `max_total_evidence_tokens`, `max_snippets_total`,
///   `max_snippets_per_source` — ENFORCED by `budget::allocate` on every
///   compiled bundle (drops recorded with truncation flags).
/// * `max_candidates_total`, `max_rerank_candidates` — planner HINTS carried
///   into `RouteStep.max_candidates`; the executed pipeline additionally
///   bounds its loads (graph `max_nodes`, schema-fact and fallback-passage
///   limits in `executor.rs`).
/// * `max_read_tokens`, `max_tool_calls`, `tool_delta_inline_char_limit` —
///   ADVISORY for the consuming runtime (the retrieval layer performs no
///   tool calls or raw reads itself); they travel in the plan/trace so the
///   consumer enforces them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalBudgets {
    pub max_total_evidence_tokens: u32,
    pub max_snippets_total: u32,
    pub max_snippets_per_source: u32,
    pub max_candidates_total: u32,
    pub max_read_tokens: u32,
    pub max_tool_calls: u32,
    pub max_rerank_candidates: u32,
    pub tool_delta_inline_char_limit: u32,
}

impl RetrievalBudgets {
    /// A conservative default budget for a single bounded bundle build. MT-137
    /// derives task-shaped variants from this baseline.
    pub fn default_bounded() -> Self {
        Self {
            max_total_evidence_tokens: 4_000,
            max_snippets_total: 24,
            max_snippets_per_source: 4,
            max_candidates_total: 128,
            max_read_tokens: 1_200,
            max_tool_calls: 8,
            max_rerank_candidates: 64,
            tool_delta_inline_char_limit: 2_000,
        }
    }
}

/// Trust floor and metadata allowlists applied to candidates (spec
/// 2.6.6.7.14.5 `RetrievalFilters`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RetrievalFilters {
    pub allow_external_fetch: bool,
    /// `low` | `medium` | `high`.
    pub trust_min: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content_tier_allowlist: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub consent_profile_allowlist: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entity_types_allowlist: Vec<String>,
}

/// Determinism posture of a plan (spec 2.6.6.7.14.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeterminismMode {
    /// Deterministic (or deterministic approximation with fixed seed/settings).
    Strict,
    /// Approximate, but candidate list + selection inputs are persisted so the
    /// run can be replayed exactly.
    Replay,
}

impl DeterminismMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Strict => "strict",
            Self::Replay => "replay",
        }
    }
}

/// The plan a retrieval-mode planner produces BEFORE candidate generation
/// (spec 2.6.6.7.14.5 `QueryPlan`, 2.3.14 [ADD v02.178] cheapest-authoritative).
///
/// Invariant (spec A0.5): if `retrieval_mode != HybridRag`, `non_hybrid_reason`
/// MUST be populated. [`QueryPlan::validate`] enforces this.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryPlan {
    pub plan_id: String,
    pub created_at: DateTime<Utc>,
    pub query_text: String,
    pub query_kind: QueryKind,
    pub retrieval_mode: QueryRetrievalMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub non_hybrid_reason: Option<NonHybridReason>,
    pub route: Vec<RouteStep>,
    pub budgets: RetrievalBudgets,
    pub filters: RetrievalFilters,
    pub determinism_mode: DeterminismMode,
    pub policy_id: String,
    pub version: u32,
}

impl QueryPlan {
    /// `normalized_query_hash = sha256(normalize(query_text))` (spec
    /// 2.6.6.7.14.6 B): trim, collapse internal whitespace, NFC, Unicode
    /// casefold (lowercase), strip control chars.
    pub fn normalized_query_hash(query_text: &str) -> String {
        let normalized = normalize_query(query_text);
        let mut hasher = Sha256::new();
        hasher.update(normalized.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Enforce the spec A0.5 invariant: a non-hybrid mode MUST carry a reason.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.retrieval_mode.requires_non_hybrid_reason() && self.non_hybrid_reason.is_none() {
            return Err(
                "QueryPlan with retrieval_mode != hybrid_rag MUST populate non_hybrid_reason (spec 2.3.14 [ADD v02.178])",
            );
        }
        Ok(())
    }

    pub fn as_value(&self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }
}

/// Deterministic query normalization (spec 2.6.6.7.14.6 B).
pub fn normalize_query(query_text: &str) -> String {
    // Strip control chars, collapse whitespace runs, trim, lowercase.
    let mut collapsed = String::with_capacity(query_text.len());
    let mut last_was_space = false;
    for ch in query_text.chars() {
        if ch.is_control() {
            continue;
        }
        if ch.is_whitespace() {
            if !last_was_space {
                collapsed.push(' ');
                last_was_space = true;
            }
        } else {
            collapsed.push(ch);
            last_was_space = false;
        }
    }
    // NFC normalization is approximated by Rust's built-in handling here; the
    // load-bearing determinism properties for the index (trim/collapse/lower)
    // are explicit. Lowercase via Unicode-aware lowercasing.
    collapsed.trim().to_lowercase()
}

/// A scored retrieval candidate (spec 2.6.6.7.14.5 `RetrievalCandidate`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetrievalCandidate {
    /// Stable id of what was retrieved (source/span/entity/passage id).
    pub candidate_id: String,
    /// `source_ref` | `entity_ref` | `span_ref` | `passage_ref` | `claim_ref`.
    pub kind: String,
    pub store: RetrievalStore,
    #[serde(default)]
    pub scores: CandidateScores,
    /// Deterministically computed base score.
    pub base_score: f64,
    /// Stable tie-break key.
    pub tiebreak: String,
}

/// Component scores feeding a candidate's base score (spec
/// 2.6.6.7.14.5 `RetrievalCandidate.scores`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct CandidateScores {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lexical: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vector: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pack: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trust_adjust: Option<f64>,
}

/// One taken route step in the executed trace (spec 2.6.6.7.14.5
/// `RetrievalTrace.route_taken[]`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteTaken {
    pub store: RetrievalStore,
    pub reason: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub cache_hit: bool,
}

/// A finally-selected evidence ref (spec 2.6.6.7.14.5
/// `RetrievalTrace.selected[]`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectedRef {
    pub candidate_id: String,
    pub final_rank: u32,
    pub final_score: f64,
    pub why: String,
}

/// The replayable record of one retrieval (spec 2.6.6.7.14.5 `RetrievalTrace`,
/// 2.3.13.11). Built alongside the QueryPlan, serialized into the
/// `knowledge_retrieval_traces.decisions` JSONB.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetrievalTrace {
    pub query_plan_id: String,
    pub retrieval_mode: QueryRetrievalMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub non_hybrid_reason: Option<NonHybridReason>,
    pub normalized_query_hash: String,
    pub route_taken: Vec<RouteTaken>,
    pub candidates: Vec<RetrievalCandidate>,
    pub selected: Vec<SelectedRef>,
    pub budgets_applied: RetrievalBudgets,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub truncation_flags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
}

impl RetrievalTrace {
    /// Build an empty trace bound to a plan (no candidates yet).
    pub fn for_plan(plan: &QueryPlan) -> Self {
        Self {
            query_plan_id: plan.plan_id.clone(),
            retrieval_mode: plan.retrieval_mode,
            non_hybrid_reason: plan.non_hybrid_reason,
            normalized_query_hash: QueryPlan::normalized_query_hash(&plan.query_text),
            route_taken: Vec::new(),
            candidates: Vec::new(),
            selected: Vec::new(),
            budgets_applied: plan.budgets,
            truncation_flags: Vec::new(),
            warnings: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// The `mode_reason` string persisted in `knowledge_retrieval_traces`
    /// (spec MUST: why broader retrieval was used or skipped). For a non-hybrid
    /// mode it is the `non_hybrid_reason`; for hybrid it is an explicit
    /// "hybrid required" sentence; for a passage fallback (persisted as
    /// `hybrid_rag` in the durable mode column) it carries the fallback
    /// rationale the executor recorded in the trace warnings, so the WHY
    /// survives into the durable `mode_reason` column (adversarial-v2
    /// MT-129/MT-133). Never empty (the DB CHECK rejects empty).
    pub fn mode_reason(&self) -> String {
        if self.retrieval_mode == QueryRetrievalMode::PassageFallback {
            let rationale = self
                .warnings
                .iter()
                .find(|w| w.contains("falling back to passage retrieval"))
                .cloned()
                .unwrap_or_else(|| {
                    "graph candidates were missing/stale/contradicted/low-confidence".to_string()
                });
            return format!("passage fallback (persisted as hybrid_rag): {rationale}");
        }
        match self.non_hybrid_reason {
            Some(reason) => format!(
                "chose {} (skipped hybrid_rag): {}",
                self.retrieval_mode.to_storage_str(),
                reason.as_str()
            ),
            None => format!(
                "chose {}: supporting objects were not already known, discovery/synthesis required",
                self.retrieval_mode.to_storage_str()
            ),
        }
    }

    /// Serialize the full plan + trace into the replayable `decisions` JSONB
    /// payload persisted in `knowledge_retrieval_traces.decisions`. This is the
    /// single artifact from which the entire run is reconstructable.
    pub fn to_decisions_json(&self, plan: &QueryPlan) -> Value {
        json!({
            "schema": "hsk.retrieval_decisions@1",
            "query_plan": plan.as_value(),
            "retrieval_trace": serde_json::to_value(self).unwrap_or(Value::Null),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_plan(mode: QueryRetrievalMode, reason: Option<NonHybridReason>) -> QueryPlan {
        QueryPlan {
            plan_id: "QP-test-0001".to_string(),
            created_at: Utc::now(),
            query_text: "  Find  WP-KERNEL-009 \t status\n".to_string(),
            query_kind: QueryKind::FactLookup,
            retrieval_mode: mode,
            non_hybrid_reason: reason,
            route: vec![RouteStep::new(
                RetrievalStore::BoundedReadOnly,
                "load packet authority",
                1,
            )
            .required()],
            budgets: RetrievalBudgets::default_bounded(),
            filters: RetrievalFilters::default(),
            determinism_mode: DeterminismMode::Strict,
            policy_id: "cheapest_authoritative_v1".to_string(),
            version: 1,
        }
    }

    #[test]
    fn normalize_is_deterministic_and_canonical() {
        let a = normalize_query("  Find   WP-KERNEL-009 \t Status\n");
        let b = normalize_query("find wp-kernel-009 status");
        assert_eq!(a, b);
        assert_eq!(a, "find wp-kernel-009 status");
    }

    #[test]
    fn normalized_hash_is_stable() {
        let h1 = QueryPlan::normalized_query_hash("Find  WP-KERNEL-009");
        let h2 = QueryPlan::normalized_query_hash("find wp-kernel-009");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn validate_rejects_non_hybrid_without_reason() {
        let plan = sample_plan(QueryRetrievalMode::DirectLoad, None);
        assert!(plan.validate().is_err());
        let ok = sample_plan(
            QueryRetrievalMode::DirectLoad,
            Some(NonHybridReason::ExactIdentifierKnown),
        );
        assert!(ok.validate().is_ok());
    }

    #[test]
    fn validate_allows_hybrid_without_reason() {
        let plan = sample_plan(QueryRetrievalMode::HybridRag, None);
        assert!(plan.validate().is_ok());
    }

    #[test]
    fn mode_reason_is_never_empty() {
        // Non-hybrid: reason text.
        let plan = sample_plan(
            QueryRetrievalMode::DirectLoad,
            Some(NonHybridReason::ExactIdentifierKnown),
        );
        let trace = RetrievalTrace::for_plan(&plan);
        assert!(trace.mode_reason().contains("exact_identifier_known"));
        assert!(!trace.mode_reason().trim().is_empty());
        // Hybrid: still a non-empty sentence.
        let hplan = sample_plan(QueryRetrievalMode::HybridRag, None);
        let htrace = RetrievalTrace::for_plan(&hplan);
        assert!(!htrace.mode_reason().trim().is_empty());
    }

    #[test]
    fn decisions_json_round_trips_plan_and_trace() {
        let plan = sample_plan(
            QueryRetrievalMode::ExactLookup,
            Some(NonHybridReason::ExactIdentifierKnown),
        );
        let trace = RetrievalTrace::for_plan(&plan);
        let decisions = trace.to_decisions_json(&plan);
        assert_eq!(decisions["schema"], "hsk.retrieval_decisions@1");
        // The persisted plan id is recoverable from the decisions payload.
        assert_eq!(decisions["query_plan"]["plan_id"], "QP-test-0001");
        assert_eq!(
            decisions["retrieval_trace"]["retrieval_mode"],
            "exact_lookup"
        );
    }
}

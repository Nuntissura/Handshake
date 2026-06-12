//! MT-164: RAG retrieval-mode policy.
//!
//! Closed `RetrievalMode` enum + a deterministic router that selects the
//! cheapest authoritative path per operation type.

use regex::Regex;
use serde::{Deserialize, Serialize};

use super::capsule::TaskType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalMode {
    NoRag,
    FullText,
    Vector,
    Hybrid,
    GraphAware,
    AuthoritativeOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalContext {
    pub query: String,
    pub task_type: TaskType,
    pub operation_class: OperationClass,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationClass {
    QueryPlan,
    RetrievalTrace,
    ProjectBrain,
    PromptToSpecRouter,
    WorkPacketLoad,
    MicroTaskContextAssembly,
    GeneralFreeform,
    FreshnessSensitive,
}

/// One routing rule. Predicate is run in order against the context.
pub struct RoutingRule {
    pub id: &'static str,
    pub predicate: Box<dyn Fn(&RetrievalContext) -> bool + Send + Sync>,
    pub mode: RetrievalMode,
    pub rationale: &'static str,
}

impl std::fmt::Debug for RoutingRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RoutingRule")
            .field("id", &self.id)
            .field("mode", &self.mode)
            .field("rationale", &self.rationale)
            .finish()
    }
}

pub struct RetrievalModePolicy {
    pub rules: Vec<RoutingRule>,
    pub default_mode: RetrievalMode,
}

impl RetrievalModePolicy {
    pub fn default_v0() -> Self {
        let uuid_re = Regex::new(
            r"\b[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}\b",
        )
        .unwrap();
        let wp_re = Regex::new(r"\bWP-[A-Z0-9-]+\b").unwrap();
        let hbr_re = Regex::new(r"\bHBR-[A-Z]+-\d{3}\b").unwrap();
        let spec_re = Regex::new(r"(\.GOV/spec/|master-spec-v\d+\.\d+|spec-modules/)").unwrap();

        Self {
            rules: vec![
                RoutingRule {
                    id: "exact_uuid_in_query",
                    predicate: Box::new(move |ctx| uuid_re.is_match(&ctx.query)),
                    mode: RetrievalMode::NoRag,
                    rationale: "exact uuid lookup never benefits from RAG",
                },
                RoutingRule {
                    id: "wp_id_pattern",
                    predicate: Box::new(move |ctx| wp_re.is_match(&ctx.query)),
                    mode: RetrievalMode::AuthoritativeOnly,
                    rationale: "wp_id maps directly to packet.json",
                },
                RoutingRule {
                    id: "hbr_id_pattern",
                    predicate: Box::new(move |ctx| hbr_re.is_match(&ctx.query)),
                    mode: RetrievalMode::AuthoritativeOnly,
                    rationale: "hbr_id maps directly to HANDSHAKE_BUILD_RULES.json",
                },
                RoutingRule {
                    id: "spec_anchor_pattern",
                    predicate: Box::new(move |ctx| spec_re.is_match(&ctx.query)),
                    mode: RetrievalMode::AuthoritativeOnly,
                    rationale: "spec anchor reads directly from spec module file",
                },
                RoutingRule {
                    id: "freshness_sensitive_task_type",
                    predicate: Box::new(|ctx| {
                        matches!(ctx.operation_class, OperationClass::FreshnessSensitive)
                    }),
                    mode: RetrievalMode::Vector,
                    rationale: "freshness-sensitive tasks prefer recency-weighted vector search",
                },
                RoutingRule {
                    id: "general_freeform_query",
                    predicate: Box::new(|ctx| {
                        matches!(ctx.operation_class, OperationClass::GeneralFreeform)
                    }),
                    mode: RetrievalMode::Hybrid,
                    rationale: "freeform queries fall back to hybrid",
                },
            ],
            default_mode: RetrievalMode::Hybrid,
        }
    }
}

pub struct RetrievalModeRouter<'a> {
    pub policy: &'a RetrievalModePolicy,
}

impl<'a> RetrievalModeRouter<'a> {
    pub fn new(policy: &'a RetrievalModePolicy) -> Self {
        Self { policy }
    }

    pub fn route(&self, ctx: &RetrievalContext) -> (RetrievalMode, String) {
        for rule in &self.policy.rules {
            if (rule.predicate)(ctx) {
                return (rule.mode, rule.id.to_string());
            }
        }
        (self.policy.default_mode, "default".to_string())
    }
}

// ===========================================================================
// WP-KERNEL-009 MT-129 RetrievalModeEnumExpansion (ADDITIVE extension).
//
// The MT-164 `RetrievalMode` enum above (NoRag/FullText/Vector/Hybrid/
// GraphAware/AuthoritativeOnly) describes the *RAG search strategy* a router
// picks. It is intentionally left untouched.
//
// WP-KERNEL-009's ProjectKnowledgeIndex retrieval (spec 2.3.13.11 +
// 2.3.14 [ADD v02.178]) reasons over a different, spec-canonical axis: the
// *cheapest authoritative mode* an explainable QueryPlan/RetrievalTrace records
// — `none` -> `direct_load` -> `exact_lookup` -> `graph_traversal` ->
// `hybrid_rag`, plus `passage_fallback` and `blocked`. This axis is what the
// storage layer already persists in `knowledge_retrieval_traces.retrieval_mode`
// (migration 0141, the five spec strings) and is the contract the
// RetrievalContextAndRanking group (MT-129..MT-144) plans against.
//
// `QueryRetrievalMode` is that closed enum. It is a superset of the storage
// `KnowledgeRetrievalMode` (which only persists the five spec-string modes):
// `PassageFallback` is a *planner posture* that resolves to a `hybrid_rag`-class
// passage retrieval when graph candidates are missing/stale/contradicted, and
// `Blocked` is a planner posture that produces no retrieval at all. Both record
// an explicit reason. `to_storage_mode()` maps a planner posture to the durable
// spec string so a trace row is always one of the five spec values.
// ===========================================================================

use std::fmt;

/// The spec-canonical retrieval mode axis (2.3.13.11 / 2.3.14 [ADD v02.178]):
/// the cheapest authoritative mode that satisfies a task. Ordered cheapest →
/// broadest. A planner MUST pick the first mode that satisfies the task and, if
/// it skips `HybridRag`, record a [`NonHybridReason`] in the QueryPlan and
/// RetrievalTrace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryRetrievalMode {
    /// No retrieval is needed — the caller already holds the answer, or the
    /// query is non-retrieval (e.g. a pure transform). Cheapest.
    None,
    /// A stable authoritative handle (entity id, file selector, WP/MT id) is
    /// known; load that record directly. No search.
    DirectLoad,
    /// An exact key (ontology term, alias, relationship_id, content hash) is
    /// known; resolve it with a single exact lookup. No ranking sweep.
    ExactLookup,
    /// A bounded Knowledge-Graph / Loom neighborhood can satisfy the task;
    /// traverse edges within depth/edge-type/hub-suppression caps.
    GraphTraversal,
    /// Discovery / synthesis over passages: the supporting objects are not
    /// already known and graph candidates do not suffice. Broadest authoritative
    /// mode.
    HybridRag,
    /// Planner posture: graph candidates were missing, stale, contradicted, or
    /// low-confidence, so retrieval falls back to passage retrieval. Persisted
    /// as `hybrid_rag` with a passage-fallback reason (MT-133).
    PassageFallback,
    /// Planner posture: retrieval is refused (policy disallows, no authoritative
    /// handle, budget exhausted). Persisted as `none` with a blocked reason.
    Blocked,
}

impl QueryRetrievalMode {
    /// The spec string used in `QueryPlan.retrieval_mode` / the
    /// `knowledge_retrieval_traces.retrieval_mode` column. Planner postures
    /// collapse onto the nearest durable spec value: `PassageFallback` →
    /// `hybrid_rag`, `Blocked` → `none`.
    pub fn to_storage_str(self) -> &'static str {
        match self {
            Self::None | Self::Blocked => "none",
            Self::DirectLoad => "direct_load",
            Self::ExactLookup => "exact_lookup",
            Self::GraphTraversal => "graph_traversal",
            Self::HybridRag | Self::PassageFallback => "hybrid_rag",
        }
    }

    /// Cheapest-first rank used by the planner to compare two satisfying modes
    /// and to assert the chosen mode is not more expensive than necessary.
    pub fn cost_rank(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Blocked => 0,
            Self::DirectLoad => 1,
            Self::ExactLookup => 2,
            Self::GraphTraversal => 3,
            Self::PassageFallback => 4,
            Self::HybridRag => 5,
        }
    }

    /// Whether this mode skips full hybrid retrieval and therefore MUST carry a
    /// [`NonHybridReason`] in the plan/trace (spec 2.3.14 [ADD v02.178]).
    pub fn requires_non_hybrid_reason(self) -> bool {
        !matches!(self, Self::HybridRag)
    }
}

impl fmt::Display for QueryRetrievalMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.to_storage_str())
    }
}

/// Why a consumer skipped `hybrid_rag` (spec 2.6.6.7.14.5 `non_hybrid_reason`
/// vocabulary). Recorded in the QueryPlan + RetrievalTrace whenever
/// `retrieval_mode != hybrid_rag` so operators can tell when "no RAG" was the
/// safer path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NonHybridReason {
    /// An exact id / authoritative handle was supplied (direct_load /
    /// exact_lookup).
    ExactIdentifierKnown,
    /// Authoritative packet/spec/manual state was required, not advisory search.
    AuthoritativeStateRequired,
    /// A bounded executor context (graph neighborhood) satisfied the task.
    BoundedExecutorContext,
    /// Freshness could not be guaranteed for broader retrieval.
    FreshnessUncertain,
    /// Policy disallowed hybrid retrieval for this caller/operation.
    PolicyDisallowsHybridRag,
    /// The operator/caller forced a direct, no-RAG path.
    UserForcedDirect,
}

impl NonHybridReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ExactIdentifierKnown => "exact_identifier_known",
            Self::AuthoritativeStateRequired => "authoritative_state_required",
            Self::BoundedExecutorContext => "bounded_executor_context",
            Self::FreshnessUncertain => "freshness_uncertain",
            Self::PolicyDisallowsHybridRag => "policy_disallows_hybrid_rag",
            Self::UserForcedDirect => "user_forced_direct",
        }
    }
}

impl fmt::Display for NonHybridReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The shape of a retrieval-backed query (spec 2.6.6.7.14.5 `QueryPlan.query_kind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryKind {
    FactLookup,
    Summarize,
    Compare,
    Transform,
    Export,
    Unknown,
}

impl QueryKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FactLookup => "fact_lookup",
            Self::Summarize => "summarize",
            Self::Compare => "compare",
            Self::Transform => "transform",
            Self::Export => "export",
            Self::Unknown => "unknown",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(query: &str, op: OperationClass) -> RetrievalContext {
        RetrievalContext {
            query: query.to_string(),
            task_type: TaskType::GeneralRetrieval,
            operation_class: op,
        }
    }

    #[test]
    fn uuid_query_routes_to_no_rag() {
        let policy = RetrievalModePolicy::default_v0();
        let router = RetrievalModeRouter::new(&policy);
        let (mode, rule) = router.route(&ctx(
            "lookup 01938b67-1234-7abc-89ef-0123456789ab here",
            OperationClass::QueryPlan,
        ));
        assert_eq!(mode, RetrievalMode::NoRag);
        assert_eq!(rule, "exact_uuid_in_query");
    }

    #[test]
    fn wp_id_routes_to_authoritative() {
        let policy = RetrievalModePolicy::default_v0();
        let router = RetrievalModeRouter::new(&policy);
        let (mode, rule) = router.route(&ctx("load WP-KERNEL-004", OperationClass::WorkPacketLoad));
        assert_eq!(mode, RetrievalMode::AuthoritativeOnly);
        assert_eq!(rule, "wp_id_pattern");
    }

    #[test]
    fn hbr_id_routes_to_authoritative() {
        let policy = RetrievalModePolicy::default_v0();
        let router = RetrievalModeRouter::new(&policy);
        let (mode, rule) = router.route(&ctx("HBR-INT-006 enforcement", OperationClass::QueryPlan));
        assert_eq!(mode, RetrievalMode::AuthoritativeOnly);
        assert_eq!(rule, "hbr_id_pattern");
    }

    #[test]
    fn spec_anchor_routes_to_authoritative() {
        let policy = RetrievalModePolicy::default_v0();
        let router = RetrievalModeRouter::new(&policy);
        let (mode, _rule) = router.route(&ctx(
            ".GOV/spec/master-spec-v02.186/spec-modules/04-llm-infrastructure.md",
            OperationClass::QueryPlan,
        ));
        assert_eq!(mode, RetrievalMode::AuthoritativeOnly);
    }

    #[test]
    fn freshness_sensitive_routes_to_vector() {
        let policy = RetrievalModePolicy::default_v0();
        let router = RetrievalModeRouter::new(&policy);
        let (mode, rule) = router.route(&ctx("latest news", OperationClass::FreshnessSensitive));
        assert_eq!(mode, RetrievalMode::Vector);
        assert_eq!(rule, "freshness_sensitive_task_type");
    }

    #[test]
    fn general_freeform_routes_to_hybrid() {
        let policy = RetrievalModePolicy::default_v0();
        let router = RetrievalModeRouter::new(&policy);
        let (mode, rule) = router.route(&ctx(
            "how do we ship this?",
            OperationClass::GeneralFreeform,
        ));
        assert_eq!(mode, RetrievalMode::Hybrid);
        assert_eq!(rule, "general_freeform_query");
    }

    #[test]
    fn router_is_deterministic() {
        let policy = RetrievalModePolicy::default_v0();
        let router = RetrievalModeRouter::new(&policy);
        let c = ctx("review the audit", OperationClass::GeneralFreeform);
        let a = router.route(&c);
        let b = router.route(&c);
        assert_eq!(a, b);
    }

    // --- MT-129 RetrievalModeEnumExpansion (additive) ---

    #[test]
    fn query_retrieval_mode_maps_to_spec_storage_strings() {
        // The five durable spec strings the 0141 trace column accepts.
        assert_eq!(QueryRetrievalMode::None.to_storage_str(), "none");
        assert_eq!(
            QueryRetrievalMode::DirectLoad.to_storage_str(),
            "direct_load"
        );
        assert_eq!(
            QueryRetrievalMode::ExactLookup.to_storage_str(),
            "exact_lookup"
        );
        assert_eq!(
            QueryRetrievalMode::GraphTraversal.to_storage_str(),
            "graph_traversal"
        );
        assert_eq!(QueryRetrievalMode::HybridRag.to_storage_str(), "hybrid_rag");
        // Planner postures collapse onto the nearest durable spec value.
        assert_eq!(
            QueryRetrievalMode::PassageFallback.to_storage_str(),
            "hybrid_rag"
        );
        assert_eq!(QueryRetrievalMode::Blocked.to_storage_str(), "none");
    }

    #[test]
    fn cheapest_authoritative_ordering_is_monotonic() {
        // none < direct_load < exact_lookup < graph_traversal < passage_fallback
        // < hybrid_rag (spec 2.3.14 [ADD v02.178] cheapest-authoritative ladder).
        assert!(QueryRetrievalMode::None.cost_rank() < QueryRetrievalMode::DirectLoad.cost_rank());
        assert!(
            QueryRetrievalMode::DirectLoad.cost_rank()
                < QueryRetrievalMode::ExactLookup.cost_rank()
        );
        assert!(
            QueryRetrievalMode::ExactLookup.cost_rank()
                < QueryRetrievalMode::GraphTraversal.cost_rank()
        );
        assert!(
            QueryRetrievalMode::GraphTraversal.cost_rank()
                < QueryRetrievalMode::HybridRag.cost_rank()
        );
    }

    #[test]
    fn non_hybrid_modes_require_a_reason() {
        for mode in [
            QueryRetrievalMode::None,
            QueryRetrievalMode::DirectLoad,
            QueryRetrievalMode::ExactLookup,
            QueryRetrievalMode::GraphTraversal,
            QueryRetrievalMode::PassageFallback,
            QueryRetrievalMode::Blocked,
        ] {
            assert!(
                mode.requires_non_hybrid_reason(),
                "{mode} must carry a non_hybrid_reason"
            );
        }
        assert!(!QueryRetrievalMode::HybridRag.requires_non_hybrid_reason());
    }

    #[test]
    fn non_hybrid_reason_and_query_kind_round_trip_via_serde() {
        for reason in [
            NonHybridReason::ExactIdentifierKnown,
            NonHybridReason::AuthoritativeStateRequired,
            NonHybridReason::BoundedExecutorContext,
            NonHybridReason::FreshnessUncertain,
            NonHybridReason::PolicyDisallowsHybridRag,
            NonHybridReason::UserForcedDirect,
        ] {
            let json = serde_json::to_string(&reason).unwrap();
            assert_eq!(json, format!("\"{}\"", reason.as_str()));
            let back: NonHybridReason = serde_json::from_str(&json).unwrap();
            assert_eq!(back, reason);
        }
        for kind in [
            QueryKind::FactLookup,
            QueryKind::Summarize,
            QueryKind::Compare,
            QueryKind::Transform,
            QueryKind::Export,
            QueryKind::Unknown,
        ] {
            let json = serde_json::to_string(&kind).unwrap();
            assert_eq!(json, format!("\"{}\"", kind.as_str()));
        }
    }
}

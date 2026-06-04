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
}

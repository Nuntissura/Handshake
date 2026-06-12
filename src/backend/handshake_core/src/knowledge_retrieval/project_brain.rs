//! WP-KERNEL-009 MT-139 ProjectBrainBridge.
//!
//! Folded WP-1-Project-Brain-Runtime-Backfill-v1 intent: "Project Brain remains
//! a governed runtime retrieval/notebook surface, not a vague RAG promise";
//! "Project Brain query/runtime mapping must expose citations, context-
//! compaction visibility, QueryPlan, RetrievalTrace, and job/tool visibility";
//! "Project Brain must distinguish discovery/synthesis from authoritative lookup
//! and prefer fresh ContextPack reuse where appropriate." Spec 2.3.2.x Project
//! Brain ("a governed backend retrieval notebook over AI-Ready Data. QueryPlan,
//! RetrievalTrace, and fresh ContextPack reuse MUST remain explicit backend
//! contracts rather than UI-only notebook affordances").
//!
//! This bridge maps a Project Brain query into the WP-009 retrieval pipeline: it
//! classifies the query as authoritative LOOKUP (a stable handle is present) vs
//! DISCOVERY/synthesis (none is), builds the corresponding [`RetrievalRequest`]
//! for the CheapestAuthoritativePathPlanner, and signals whether a fresh
//! ContextPack should be preferred. It is the deterministic glue that keeps
//! Project Brain a backend contract, not a prompt affordance.

use crate::knowledge_retrieval::plan::{DeterminismMode, RetrievalBudgets, RetrievalFilters};
use crate::knowledge_retrieval::planner::{AuthoritativeHandle, HybridPosture, RetrievalRequest};
use crate::memory::retrieval_mode::QueryKind;

/// How a Project Brain query was classified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectBrainIntent {
    /// A stable authoritative handle is present — authoritative lookup.
    AuthoritativeLookup,
    /// No handle — discovery / synthesis over the index.
    DiscoverySynthesis,
}

impl ProjectBrainIntent {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AuthoritativeLookup => "authoritative_lookup",
            Self::DiscoverySynthesis => "discovery_synthesis",
        }
    }
}

/// A Project Brain query as it enters the bridge.
#[derive(Debug, Clone)]
pub struct ProjectBrainQuery {
    pub workspace_id: String,
    pub query_text: String,
    /// Any authoritative handles the notebook context already holds.
    pub handles: Vec<AuthoritativeHandle>,
    /// True when the caller is willing to reuse a fresh ContextPack rather than
    /// rebuild (the folded "prefer fresh ContextPack reuse" requirement).
    pub prefer_context_pack_reuse: bool,
    /// True when the notebook cannot confirm index freshness.
    pub freshness_uncertain: bool,
}

/// The bridge's plan inputs for the retrieval pipeline.
#[derive(Debug, Clone)]
pub struct ProjectBrainPlanInputs {
    pub intent: ProjectBrainIntent,
    pub request: RetrievalRequest,
    /// Whether the planner should attempt fresh ContextPack reuse first.
    pub prefer_context_pack_reuse: bool,
}

/// Map a Project Brain query into retrieval-pipeline inputs. The mapping is
/// deterministic: presence of any handle ⇒ authoritative lookup (cheapest
/// path); absence ⇒ discovery (hybrid allowed). Either way a QueryPlan +
/// RetrievalTrace will be produced downstream, satisfying the "explicit backend
/// contract" requirement.
pub fn map_query(query: &ProjectBrainQuery) -> ProjectBrainPlanInputs {
    let has_handle = !query.handles.is_empty();
    let intent = if has_handle {
        ProjectBrainIntent::AuthoritativeLookup
    } else {
        ProjectBrainIntent::DiscoverySynthesis
    };

    let request = RetrievalRequest {
        workspace_id: query.workspace_id.clone(),
        query_text: query.query_text.clone(),
        // A lookup with a handle is a fact lookup; open discovery is summarize.
        query_kind: if has_handle {
            QueryKind::FactLookup
        } else {
            QueryKind::Summarize
        },
        handles: query.handles.clone(),
        // Discovery without a handle does not assume a bounded graph; the
        // planner decides hybrid. A lookup ignores this.
        graph_neighborhood_expected: false,
        freshness_uncertain: query.freshness_uncertain,
        hybrid_posture: HybridPosture::Allowed,
        budgets: RetrievalBudgets::default_bounded(),
        filters: RetrievalFilters::default(),
        determinism_mode: DeterminismMode::Strict,
    };

    ProjectBrainPlanInputs {
        intent,
        request,
        prefer_context_pack_reuse: query.prefer_context_pack_reuse,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::knowledge::KnowledgeEntityKind;

    #[test]
    fn query_with_handle_is_authoritative_lookup() {
        let q = ProjectBrainQuery {
            workspace_id: "ws".to_string(),
            query_text: "status of WP-KERNEL-009".to_string(),
            handles: vec![AuthoritativeHandle::WorkPacketId(
                "WP-KERNEL-009".to_string(),
            )],
            prefer_context_pack_reuse: true,
            freshness_uncertain: false,
        };
        let inputs = map_query(&q);
        assert_eq!(inputs.intent, ProjectBrainIntent::AuthoritativeLookup);
        assert_eq!(inputs.request.query_kind, QueryKind::FactLookup);
        assert!(inputs.prefer_context_pack_reuse);
    }

    #[test]
    fn query_without_handle_is_discovery() {
        let q = ProjectBrainQuery {
            workspace_id: "ws".to_string(),
            query_text: "how does retrieval ranking work?".to_string(),
            handles: vec![],
            prefer_context_pack_reuse: false,
            freshness_uncertain: false,
        };
        let inputs = map_query(&q);
        assert_eq!(inputs.intent, ProjectBrainIntent::DiscoverySynthesis);
        assert_eq!(inputs.request.query_kind, QueryKind::Summarize);
    }

    #[test]
    fn entity_identity_handle_drives_lookup() {
        let q = ProjectBrainQuery {
            workspace_id: "ws".to_string(),
            query_text: "the foo symbol".to_string(),
            handles: vec![AuthoritativeHandle::EntityIdentity {
                kind: KnowledgeEntityKind::Symbol,
                key: "foo".to_string(),
            }],
            prefer_context_pack_reuse: false,
            freshness_uncertain: true,
        };
        let inputs = map_query(&q);
        assert_eq!(inputs.intent, ProjectBrainIntent::AuthoritativeLookup);
        assert!(inputs.request.freshness_uncertain);
    }
}

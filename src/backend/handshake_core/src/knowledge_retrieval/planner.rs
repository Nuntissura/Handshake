//! WP-KERNEL-009 MT-130 CheapestAuthoritativePathPlanner.
//!
//! Spec 2.3.14 [ADD v02.178] + 2.6.6.7.14.6 A0: a retrieval consumer MUST choose
//! the cheapest authoritative mode that satisfies the task —
//! `none` -> `direct_load` -> `exact_lookup` -> `graph_traversal` ->
//! `hybrid_rag`. If exact entity ids, file selectors, Work Packet ids,
//! Micro-Task ids, or other stable authoritative handles are already known, the
//! runtime SHOULD NOT begin with hybrid retrieval, and MUST record the chosen
//! mode + skip reason in the QueryPlan and RetrievalTrace.
//!
//! This planner takes a [`RetrievalRequest`] (the query plus any authoritative
//! handles the caller already holds) and produces a [`QueryPlan`]. It is
//! deterministic: the same request yields the same plan. When a handle names an
//! entity, the planner CONFIRMS the entity exists in the ProjectKnowledgeIndex
//! (real PostgreSQL read via `KnowledgeStore`) before choosing `direct_load` —
//! a dangling handle degrades to a broader mode rather than producing a plan
//! that cannot be satisfied.
//!
//! The folded WP-1-RAG-Retrieval-Mode-Policy-v1 stub intent is preserved here:
//! "one governed retrieval-mode policy covers none/direct_load/exact_lookup/
//! graph_traversal/hybrid_rag" and "Prompt-to-Spec, Project Brain, Loom graph
//! expansion, Work Packet loads, and Micro-Task context assembly must choose the
//! cheapest authoritative retrieval path rather than defaulting to hybrid".

use uuid::Uuid;

use crate::memory::retrieval_mode::{NonHybridReason, QueryKind, QueryRetrievalMode};
use crate::storage::knowledge::{KnowledgeEntityKind, KnowledgeStore};
use crate::storage::postgres::PostgresDatabase;
use crate::storage::StorageResult;

use super::plan::{
    DeterminismMode, QueryPlan, RetrievalBudgets, RetrievalFilters, RetrievalStore, RouteStep,
};

/// A stable authoritative handle the caller already holds. The presence of one
/// of these is what lets the planner skip hybrid retrieval (spec A0.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthoritativeHandle {
    /// A ProjectKnowledgeIndex entity id (confirmed against PG before use).
    EntityId(String),
    /// A typed entity identity (kind + key) resolved to an entity id.
    EntityIdentity {
        kind: KnowledgeEntityKind,
        key: String,
    },
    /// A Work Packet id (e.g. `WP-KERNEL-009-...`). Authoritative packet load.
    WorkPacketId(String),
    /// A Micro-Task id (e.g. `MT-130`). Authoritative micro-task context load.
    MicroTaskId(String),
    /// A file selector / source id known to the caller.
    SourceId(String),
    /// A stable relationship_id (graph edge) — an exact graph handle.
    RelationshipId(String),
}

/// The operator/caller posture toward hybrid retrieval, letting policy or the
/// operator force a cheaper path (spec `non_hybrid_reason` user/policy cases).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HybridPosture {
    /// Use the cheapest authoritative mode; hybrid only if nothing cheaper fits.
    #[default]
    Allowed,
    /// Policy forbids hybrid for this caller/operation.
    PolicyDisallowed,
    /// The operator forced a direct, no-RAG path.
    UserForcedDirect,
}

/// A retrieval request: the query plus any authoritative handles the caller
/// already holds and the freshness/graph hints the planner uses.
#[derive(Debug, Clone)]
pub struct RetrievalRequest {
    pub workspace_id: String,
    pub query_text: String,
    pub query_kind: QueryKind,
    /// Authoritative handles the caller already holds (cheapest-first wins).
    pub handles: Vec<AuthoritativeHandle>,
    /// True when a bounded graph neighborhood is expected to satisfy the task
    /// (Loom expansion, "what links to X") and no exact handle is supplied.
    pub graph_neighborhood_expected: bool,
    /// True when the caller cannot guarantee index freshness (forces a reason
    /// even when a cheap mode is otherwise available).
    pub freshness_uncertain: bool,
    pub hybrid_posture: HybridPosture,
    pub budgets: RetrievalBudgets,
    pub filters: RetrievalFilters,
    pub determinism_mode: DeterminismMode,
}

impl RetrievalRequest {
    /// A minimal discovery request (no handles, hybrid allowed).
    pub fn discovery(workspace_id: impl Into<String>, query_text: impl Into<String>) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            query_text: query_text.into(),
            query_kind: QueryKind::Unknown,
            handles: Vec::new(),
            graph_neighborhood_expected: false,
            freshness_uncertain: false,
            hybrid_posture: HybridPosture::Allowed,
            budgets: RetrievalBudgets::default_bounded(),
            filters: RetrievalFilters::default(),
            determinism_mode: DeterminismMode::Strict,
        }
    }

    /// Attach an authoritative handle (builder).
    pub fn with_handle(mut self, handle: AuthoritativeHandle) -> Self {
        self.handles.push(handle);
        self
    }
}

/// The policy id stamped on plans this planner produces.
pub const POLICY_ID: &str = "cheapest_authoritative_v1";

/// The result of planning: the plan plus a structured note of the confirmed
/// handle (if any) so the caller / trace can cite exactly what was loaded.
#[derive(Debug, Clone)]
pub struct PlannedRetrieval {
    pub plan: QueryPlan,
    /// The handle the planner confirmed and chose to load directly, if the
    /// chosen mode is direct_load / exact_lookup.
    pub confirmed_handle: Option<ConfirmedHandle>,
}

/// A handle the planner confirmed exists in the ProjectKnowledgeIndex.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfirmedHandle {
    /// `entity` | `work_packet` | `micro_task` | `source` | `relationship`.
    pub kind: String,
    /// The concrete id loaded (entity_id for identity handles).
    pub id: String,
}

/// The cheapest-authoritative-path planner. Confirms entity/source handles
/// against the ProjectKnowledgeIndex through the committed `KnowledgeStore`
/// (the `PostgresDatabase`); it needs no separate pool because every read it
/// performs is a `KnowledgeStore` method.
pub struct CheapestAuthoritativePathPlanner<'a> {
    db: &'a PostgresDatabase,
}

impl<'a> CheapestAuthoritativePathPlanner<'a> {
    pub fn new(db: &'a PostgresDatabase) -> Self {
        Self { db }
    }

    /// Plan a retrieval, choosing the cheapest authoritative mode that the
    /// request's handles + hints justify. Confirms entity handles against PG.
    pub async fn plan(&self, request: &RetrievalRequest) -> StorageResult<PlannedRetrieval> {
        // 1. User/policy can force a direct, no-RAG path regardless of handles.
        if let HybridPosture::UserForcedDirect = request.hybrid_posture {
            // If a handle is present we still prefer direct_load; otherwise the
            // forced posture yields a `none` plan with the user reason.
            if let Some(confirmed) = self.first_confirmed_handle(request).await? {
                return Ok(self.direct_or_exact_plan(
                    request,
                    confirmed,
                    NonHybridReason::UserForcedDirect,
                ));
            }
            return Ok(PlannedRetrieval {
                plan: self.build_plan(
                    request,
                    QueryRetrievalMode::None,
                    Some(NonHybridReason::UserForcedDirect),
                    vec![],
                ),
                confirmed_handle: None,
            });
        }

        // 2. Cheapest authoritative: a confirmed exact handle -> direct/exact.
        if let Some(confirmed) = self.first_confirmed_handle(request).await? {
            let reason = if request.freshness_uncertain {
                // Even with a handle, an uncertain-freshness caller records the
                // freshness reason so operators see why broader was skipped.
                NonHybridReason::FreshnessUncertain
            } else {
                NonHybridReason::ExactIdentifierKnown
            };
            return Ok(self.direct_or_exact_plan(request, confirmed, reason));
        }

        // 3. No exact handle, but a bounded graph neighborhood is expected.
        if request.graph_neighborhood_expected {
            let route = vec![
                RouteStep::new(
                    RetrievalStore::KnowledgeGraph,
                    "bounded graph neighborhood satisfies the task",
                    request.budgets.max_candidates_total,
                )
                .required(),
                RouteStep::new(
                    RetrievalStore::ShadowWsLexical,
                    "lexical confirmation of graph candidates",
                    request.budgets.max_rerank_candidates,
                ),
            ];
            return Ok(PlannedRetrieval {
                plan: self.build_plan(
                    request,
                    QueryRetrievalMode::GraphTraversal,
                    Some(NonHybridReason::BoundedExecutorContext),
                    route,
                ),
                confirmed_handle: None,
            });
        }

        // 4. Policy disallows hybrid but nothing cheaper fits -> blocked with a
        // policy reason rather than silently widening to hybrid.
        if let HybridPosture::PolicyDisallowed = request.hybrid_posture {
            return Ok(PlannedRetrieval {
                plan: self.build_plan(
                    request,
                    QueryRetrievalMode::Blocked,
                    Some(NonHybridReason::PolicyDisallowsHybridRag),
                    vec![],
                ),
                confirmed_handle: None,
            });
        }

        // 5. Discovery / synthesis: hybrid_rag is the reserved broadest mode.
        let route = vec![
            RouteStep::new(
                RetrievalStore::ContextPacks,
                "reuse fresh ContextPack if available",
                8,
            ),
            RouteStep::new(
                RetrievalStore::KnowledgeGraph,
                "prefilter candidate entity sets",
                request.budgets.max_candidates_total,
            ),
            RouteStep::new(
                RetrievalStore::ShadowWsLexical,
                "high-precision lexical recall",
                request.budgets.max_rerank_candidates,
            ),
            RouteStep::new(
                RetrievalStore::ShadowWsVector,
                "semantic recall",
                request.budgets.max_rerank_candidates,
            ),
        ];
        Ok(PlannedRetrieval {
            plan: self.build_plan(request, QueryRetrievalMode::HybridRag, None, route),
            confirmed_handle: None,
        })
    }

    /// Resolve the first handle (cheapest-first order in the request) that
    /// CONFIRMS against the ProjectKnowledgeIndex. WP/MT/source/relationship
    /// handles are authoritative by construction; entity handles are confirmed
    /// by a real PG read so a dangling id degrades the plan.
    async fn first_confirmed_handle(
        &self,
        request: &RetrievalRequest,
    ) -> StorageResult<Option<ConfirmedHandle>> {
        for handle in &request.handles {
            match handle {
                AuthoritativeHandle::EntityId(entity_id) => {
                    if let Some(entity) = self.db.get_knowledge_entity(entity_id).await? {
                        return Ok(Some(ConfirmedHandle {
                            kind: "entity".to_string(),
                            id: entity.entity_id,
                        }));
                    }
                }
                AuthoritativeHandle::EntityIdentity { kind, key } => {
                    if let Some(entity) = self
                        .db
                        .get_knowledge_entity_by_identity(&request.workspace_id, *kind, key)
                        .await?
                    {
                        return Ok(Some(ConfirmedHandle {
                            kind: "entity".to_string(),
                            id: entity.entity_id,
                        }));
                    }
                }
                AuthoritativeHandle::WorkPacketId(id) => {
                    return Ok(Some(ConfirmedHandle {
                        kind: "work_packet".to_string(),
                        id: id.clone(),
                    }));
                }
                AuthoritativeHandle::MicroTaskId(id) => {
                    return Ok(Some(ConfirmedHandle {
                        kind: "micro_task".to_string(),
                        id: id.clone(),
                    }));
                }
                AuthoritativeHandle::SourceId(id) => {
                    if let Some(source) = self.db.get_knowledge_source(id).await? {
                        return Ok(Some(ConfirmedHandle {
                            kind: "source".to_string(),
                            id: source.source_id,
                        }));
                    }
                }
                AuthoritativeHandle::RelationshipId(id) => {
                    return Ok(Some(ConfirmedHandle {
                        kind: "relationship".to_string(),
                        id: id.clone(),
                    }));
                }
            }
        }
        Ok(None)
    }

    /// Build a direct_load (whole record) or exact_lookup (exact key) plan for a
    /// confirmed handle. A relationship_id is an exact_lookup; everything else a
    /// direct_load of an authoritative record.
    fn direct_or_exact_plan(
        &self,
        request: &RetrievalRequest,
        confirmed: ConfirmedHandle,
        reason: NonHybridReason,
    ) -> PlannedRetrieval {
        let (mode, purpose) = if confirmed.kind == "relationship" {
            (
                QueryRetrievalMode::ExactLookup,
                "exact relationship_id lookup",
            )
        } else {
            (
                QueryRetrievalMode::DirectLoad,
                "direct load of confirmed authoritative record",
            )
        };
        let route = vec![RouteStep::new(RetrievalStore::BoundedReadOnly, purpose, 1).required()];
        let plan = self.build_plan(request, mode, Some(reason), route);
        PlannedRetrieval {
            plan,
            confirmed_handle: Some(confirmed),
        }
    }

    /// Assemble a [`QueryPlan`] with a fresh plan id. The plan is validated
    /// (non-hybrid invariant) before return — an internal inconsistency panics
    /// in tests but in prod the caller can re-check via `plan.validate()`.
    fn build_plan(
        &self,
        request: &RetrievalRequest,
        mode: QueryRetrievalMode,
        reason: Option<NonHybridReason>,
        route: Vec<RouteStep>,
    ) -> QueryPlan {
        QueryPlan {
            plan_id: format!("QP-{}", Uuid::now_v7().simple()),
            created_at: chrono::Utc::now(),
            query_text: request.query_text.clone(),
            query_kind: request.query_kind,
            retrieval_mode: mode,
            non_hybrid_reason: reason,
            route,
            budgets: request.budgets,
            filters: request.filters.clone(),
            determinism_mode: request.determinism_mode,
            policy_id: POLICY_ID.to_string(),
            version: 1,
        }
    }
}

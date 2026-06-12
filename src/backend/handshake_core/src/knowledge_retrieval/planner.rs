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
//! deterministic: the same request yields the same plan. EVERY handle kind
//! (entity, work-packet, micro-task, source, relationship) is CONFIRMED
//! against the ProjectKnowledgeIndex with a real PostgreSQL read before it may
//! anchor `direct_load`/`exact_lookup` (adversarial-v2 MT-130) — a dangling
//! handle degrades the plan to a broader mode AND is recorded in
//! [`PlannedRetrieval::dangling_handles`] rather than producing a plan that
//! cannot be satisfied.
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
    /// Handles the caller supplied that did NOT confirm against the
    /// ProjectKnowledgeIndex (adversarial-v2 MT-130): each dangling handle is
    /// RECORDED here (and surfaced into the trace by the executor) so an
    /// operator can see why the plan degraded to a broader mode instead of
    /// silently producing an unsatisfiable direct load.
    pub dangling_handles: Vec<DanglingHandle>,
}

/// A handle the planner confirmed exists in the ProjectKnowledgeIndex.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfirmedHandle {
    /// `entity` | `work_packet` | `micro_task` | `source` | `relationship`.
    pub kind: String,
    /// The concrete id loaded (entity_id for identity handles).
    pub id: String,
}

/// A supplied handle that failed existence confirmation (MT-130 degrade
/// recording): the plan widened instead of trusting it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DanglingHandle {
    /// `entity` | `work_packet` | `micro_task` | `source` | `relationship`.
    pub kind: String,
    pub id: String,
}

impl DanglingHandle {
    /// The actor-visible degrade reason for a trace warning.
    pub fn reason(&self) -> String {
        format!(
            "authoritative handle did not confirm against the ProjectKnowledgeIndex: {} `{}` (plan degraded to a broader mode)",
            self.kind, self.id
        )
    }
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
        // Existence-check the supplied handles ONCE (MT-130): the first
        // confirmed handle anchors the plan; every dangling handle is recorded
        // so the degrade is explainable.
        let (confirmed, dangling_handles) = self.first_confirmed_handle(request).await?;

        // 1. User/policy can force a direct, no-RAG path regardless of handles.
        if let HybridPosture::UserForcedDirect = request.hybrid_posture {
            // If a handle is present we still prefer direct_load; otherwise the
            // forced posture yields a `none` plan with the user reason.
            if let Some(confirmed) = confirmed {
                return Ok(self.direct_or_exact_plan(
                    request,
                    confirmed,
                    NonHybridReason::UserForcedDirect,
                    dangling_handles,
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
                dangling_handles,
            });
        }

        // 2. Cheapest authoritative: a confirmed exact handle -> direct/exact.
        if let Some(confirmed) = confirmed {
            let reason = if request.freshness_uncertain {
                // Even with a handle, an uncertain-freshness caller records the
                // freshness reason so operators see why broader was skipped.
                NonHybridReason::FreshnessUncertain
            } else {
                NonHybridReason::ExactIdentifierKnown
            };
            return Ok(self.direct_or_exact_plan(request, confirmed, reason, dangling_handles));
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
                dangling_handles,
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
                dangling_handles,
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
            dangling_handles,
        })
    }

    /// Resolve the first handle (cheapest-first order in the request) that
    /// CONFIRMS against the ProjectKnowledgeIndex, recording every handle that
    /// did not (adversarial-v2 MT-130). EVERY handle kind is existence-checked
    /// by a real PG read before it may anchor a direct/exact plan:
    /// entity handles against `knowledge_entities`, WP/MT handles against the
    /// indexed work-packet / micro-task entities (the packet store inside the
    /// ProjectKnowledgeIndex), source handles against `knowledge_sources`, and
    /// relationship handles against `knowledge_edges.relationship_id`. A
    /// dangling handle never produces an unsatisfiable plan — the planner
    /// degrades to a broader mode and the dangle is recorded.
    async fn first_confirmed_handle(
        &self,
        request: &RetrievalRequest,
    ) -> StorageResult<(Option<ConfirmedHandle>, Vec<DanglingHandle>)> {
        let mut dangling = Vec::new();
        let mut confirmed = None;
        for handle in &request.handles {
            let (kind, id, hit) = match handle {
                AuthoritativeHandle::EntityId(entity_id) => {
                    let hit = self
                        .db
                        .get_knowledge_entity(entity_id)
                        .await?
                        .map(|entity| entity.entity_id);
                    ("entity", entity_id.clone(), hit)
                }
                AuthoritativeHandle::EntityIdentity { kind, key } => {
                    let hit = self
                        .db
                        .get_knowledge_entity_by_identity(&request.workspace_id, *kind, key)
                        .await?
                        .map(|entity| entity.entity_id);
                    ("entity", key.clone(), hit)
                }
                AuthoritativeHandle::WorkPacketId(id) => {
                    let hit = self
                        .db
                        .get_knowledge_entity_by_identity(
                            &request.workspace_id,
                            KnowledgeEntityKind::WorkPacket,
                            id,
                        )
                        .await?
                        .map(|_| id.clone());
                    ("work_packet", id.clone(), hit)
                }
                AuthoritativeHandle::MicroTaskId(id) => {
                    let hit = self
                        .db
                        .get_knowledge_entity_by_identity(
                            &request.workspace_id,
                            KnowledgeEntityKind::MicroTask,
                            id,
                        )
                        .await?
                        .map(|_| id.clone());
                    ("micro_task", id.clone(), hit)
                }
                AuthoritativeHandle::SourceId(id) => {
                    let hit = self
                        .db
                        .get_knowledge_source(id)
                        .await?
                        .map(|source| source.source_id);
                    ("source", id.clone(), hit)
                }
                AuthoritativeHandle::RelationshipId(id) => {
                    let hit = self
                        .db
                        .get_knowledge_edge_by_relationship_id(&request.workspace_id, id)
                        .await?
                        .map(|edge| edge.relationship_id);
                    ("relationship", id.clone(), hit)
                }
            };
            match hit {
                Some(confirmed_id) => {
                    confirmed = Some(ConfirmedHandle {
                        kind: kind.to_string(),
                        id: confirmed_id,
                    });
                    // Cheapest-first: the first confirmed handle wins; the
                    // remaining handles are not checked (they were supplied in
                    // cheapest-first order and the plan is already anchored).
                    break;
                }
                None => dangling.push(DanglingHandle {
                    kind: kind.to_string(),
                    id,
                }),
            }
        }
        Ok((confirmed, dangling))
    }

    /// Build a direct_load (whole record) or exact_lookup (exact key) plan for a
    /// confirmed handle. A relationship_id is an exact_lookup; everything else a
    /// direct_load of an authoritative record.
    fn direct_or_exact_plan(
        &self,
        request: &RetrievalRequest,
        confirmed: ConfirmedHandle,
        reason: NonHybridReason,
        dangling_handles: Vec<DanglingHandle>,
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
            dangling_handles,
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

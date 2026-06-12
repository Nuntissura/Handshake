//! WP-KERNEL-009 executed retrieval pipeline (adversarial-v2 MT-133/MT-134
//! closure).
//!
//! The review found every pipeline primitive existed and was unit-tested but
//! the INTEGRATED path was never executed: the passage fallback was a pure
//! decision nothing called, ranking was fed hand-built candidates, and the
//! compiler received pre-built inputs. This module is the one executed path:
//!
//! ```text
//! RetrievalRequest -> plan (MT-130, catalog routes MT-140)
//!                  -> schema_first_filter (MT-131 seeds)
//!                  -> graph traversal (MT-132)
//!                  -> passage fallback decision (MT-133)
//!                  -> rank_candidates (MT-134)
//!                  -> evidence snippets (MT-135)
//!                  -> compile + persist bundle + replayable trace (MT-136/138)
//! ```
//!
//! Everything reads/writes the committed PostgreSQL substrate; the produced
//! bundle and trace rows are the durable authority (spec 2.3.13.11). When the
//! graph result is missing/stale/low-confidence the fallback fires: the plan
//! is revised to the `PassageFallback` posture (persisted as `hybrid_rag` in
//! the durable mode column per the MT-129 mode law) and the durable
//! `mode_reason` + trace warnings/decisions record WHY.

use std::collections::BTreeSet;

use sqlx::PgPool;

use crate::knowledge_memory::passage::load_passages_for_workspace;
use crate::knowledge_retrieval::budget::PriorityTier;
use crate::knowledge_retrieval::compiler::{
    BundleCandidate, BundleTargetKind, CompiledBundle, ContextBundleCompilerV2,
};
use crate::knowledge_retrieval::graph_planner::{GraphTraversalPlanner, GraphTraversalPolicy};
use crate::knowledge_retrieval::passage_fallback::{
    decide_passage_fallback, GraphCandidateSignals,
};
use crate::knowledge_retrieval::plan::{RetrievalCandidate, RetrievalStore, RetrievalTrace};
use crate::knowledge_retrieval::planner::{
    CheapestAuthoritativePathPlanner, PlannedRetrieval, RetrievalRequest,
};
use crate::knowledge_retrieval::ranking::{rank_candidates, CandidateFeatures, RankingWeights};
use crate::knowledge_retrieval::snippet::{assemble_span_snippet, EvidenceSnippet};
use crate::memory::retrieval_mode::{NonHybridReason, QueryRetrievalMode};
use crate::storage::knowledge::{KnowledgePassageEvidenceRef, KnowledgeStore};
use crate::storage::postgres::PostgresDatabase;
use crate::storage::StorageResult;

/// Documented feature defaults for axes the substrate does not surface
/// per-candidate yet: graph edges come from the committed authoritative index
/// (higher authority than free passages); recency is neutral until a
/// per-record freshness signal is wired through.
const GRAPH_SOURCE_AUTHORITY: f64 = 0.7;
const PASSAGE_SOURCE_AUTHORITY: f64 = 0.5;
const NEUTRAL_RECENCY: f64 = 0.5;

/// Bounded loads inside one execution.
const SCHEMA_FACT_LIMIT: i64 = 256;
const FALLBACK_PASSAGE_LIMIT: i64 = 32;

/// The outcome of one executed retrieval: the plan that drove it, what the
/// schema filter and graph produced, whether the passage fallback fired (and
/// why), the deterministic ranking, and the persisted bundle + trace ids.
#[derive(Debug, Clone)]
pub struct ExecutedRetrieval {
    pub planned: PlannedRetrieval,
    /// Ontology terms the query resolved to (MT-131 schema scope).
    pub schema_terms_matched: usize,
    /// Facts dropped as off-topic by the schema filter.
    pub schema_off_topic_dropped: usize,
    /// Graph candidate edges the bounded traversal produced (MT-132).
    pub graph_edge_count: usize,
    /// `Some(reason)` when the passage fallback fired (MT-133).
    pub fallback_reason: Option<String>,
    /// Deterministically ranked candidates (MT-134), highest first.
    pub ranked: Vec<RetrievalCandidate>,
    /// The persisted bundle + trace (MT-136/138).
    pub compiled: CompiledBundle,
}

/// Execute one full retrieval against real PostgreSQL: plan, narrow, traverse,
/// fall back when the graph cannot answer, rank, cite, compile, persist.
///
/// `graph_seeds` are the caller's known traversal anchors (e.g. the Loom block
/// or entity a "what links to X" neighborhood query is about); the schema
/// filter's in-scope entities and a confirmed entity handle are merged in.
#[allow(clippy::too_many_arguments)]
pub async fn execute_retrieval(
    db: &PostgresDatabase,
    pool: &PgPool,
    kernel_task_run_id: &str,
    session_run_id: &str,
    target_kind: BundleTargetKind,
    target_ref: &str,
    request: &RetrievalRequest,
    graph_seeds: &BTreeSet<String>,
    graph_policy: GraphTraversalPolicy,
) -> StorageResult<ExecutedRetrieval> {
    // 1. Plan (cheapest authoritative; existence-checked handles; catalog
    //    routes when present).
    let planner = CheapestAuthoritativePathPlanner::new(db);
    let mut planned = planner.plan(request).await?;
    let mut trace = RetrievalTrace::for_plan(&planned.plan);
    for dangle in &planned.dangling_handles {
        // MT-130: the degrade is recorded in the replayable trace.
        trace.warnings.push(dangle.reason());
    }

    // 2. Schema-first filtering (MT-131): narrow the seed set to the query's
    //    ontology scope before the graph widens.
    let schema = crate::knowledge_retrieval::schema_filter::schema_first_filter(
        pool,
        &request.workspace_id,
        &request.query_text,
        SCHEMA_FACT_LIMIT,
    )
    .await?;
    let mut seeds: BTreeSet<String> = if schema.has_schema_scope() {
        schema.seed_entity_ids()
    } else {
        BTreeSet::new()
    };
    seeds.extend(graph_seeds.iter().cloned());
    if let Some(confirmed) = &planned.confirmed_handle {
        if confirmed.kind == "entity" {
            seeds.insert(confirmed.id.clone());
        }
    }

    // 3. Bounded graph traversal (MT-132) over the committed edge graph.
    let graph = GraphTraversalPlanner::new(db, pool, graph_policy)
        .traverse(&seeds)
        .await?;

    // 4. Passage-fallback decision (MT-133): missing/stale/low-confidence
    //    graphs fall back to the workspace's committed passages.
    let available_passages =
        load_passages_for_workspace(pool, &request.workspace_id, FALLBACK_PASSAGE_LIMIT).await?;
    let signals = GraphCandidateSignals {
        any_contradicted: false,
        freshness_uncertain: request.freshness_uncertain,
        max_confidence: graph
            .edges
            .iter()
            .map(|edge| edge.confidence)
            .fold(0.0_f64, f64::max),
    };
    let decision = decide_passage_fallback(&graph, signals, available_passages);

    // 5. Build ranking features from the EXECUTED result (graph candidates, or
    //    the fallback passages when the fallback fired) and rank (MT-134).
    let mut features: Vec<CandidateFeatures> = Vec::new();
    let mut snippet_span_by_candidate: Vec<(String, Option<String>, Option<i32>)> = Vec::new();
    if decision.fallback {
        trace.warnings.push(decision.rationale.clone());
        // The plan is REVISED to the PassageFallback posture: the durable mode
        // column persists it as hybrid_rag (the MT-129 mode law) and
        // `RetrievalTrace::mode_reason` carries the fallback rationale, so the
        // WHY survives into the durable row, not just the decisions JSONB.
        // PassageFallback is a non-hybrid posture and MUST carry a reason
        // (spec A0.5): a hybrid-origin plan (reason None) gets the bounded
        // passage-context reason on revision.
        planned.plan.retrieval_mode = QueryRetrievalMode::PassageFallback;
        if planned.plan.non_hybrid_reason.is_none() {
            planned.plan.non_hybrid_reason = Some(NonHybridReason::BoundedExecutorContext);
        }
        trace.retrieval_mode = QueryRetrievalMode::PassageFallback;
        trace.non_hybrid_reason = planned.plan.non_hybrid_reason;
        for passage in &decision.fallback_passages {
            features.push(CandidateFeatures {
                candidate_id: passage.passage_id.clone(),
                kind: "passage_ref".to_string(),
                store: RetrievalStore::ShadowWsVector,
                evidence_quality: passage.extraction_confidence.clamp(0.0, 1.0),
                graph_proximity: 0.0,
                relationship_type_weight: CandidateFeatures::relationship_weight_for(""),
                source_authority: PASSAGE_SOURCE_AUTHORITY,
                recency: NEUTRAL_RECENCY,
                via_hub: false,
                lexical: None,
                vector: None,
            });
            // The passage's first span evidence backs its citation (MT-135).
            let span = db
                .list_knowledge_passage_evidence(&passage.passage_id)
                .await?
                .into_iter()
                .find_map(|evidence| match evidence {
                    KnowledgePassageEvidenceRef::Span { span_id } => Some(span_id),
                    _ => None,
                });
            snippet_span_by_candidate.push((
                passage.passage_id.clone(),
                span,
                passage.token_count,
            ));
        }
    } else {
        for edge in &graph.edges {
            let via_hub = graph.suppressed_hubs.contains(&edge.source_entity_id)
                || graph.suppressed_hubs.contains(&edge.target_entity_id);
            features.push(CandidateFeatures {
                candidate_id: edge.relationship_id.clone(),
                kind: "entity_ref".to_string(),
                store: RetrievalStore::KnowledgeGraph,
                evidence_quality: edge.confidence.clamp(0.0, 1.0),
                graph_proximity: 1.0 / (1.0 + f64::from(edge.depth)),
                relationship_type_weight: CandidateFeatures::relationship_weight_for(
                    edge.edge_type.as_str(),
                ),
                source_authority: GRAPH_SOURCE_AUTHORITY,
                recency: NEUTRAL_RECENCY,
                via_hub,
                lexical: None,
                vector: None,
            });
            // The edge's first evidence span (junction table
            // knowledge_edge_spans) backs its citation (MT-135).
            let span: Option<String> = sqlx::query_scalar(
                "SELECT span_id FROM knowledge_edge_spans
                 WHERE edge_id = $1 ORDER BY span_id LIMIT 1",
            )
            .bind(&edge.edge_id)
            .fetch_optional(pool)
            .await?;
            snippet_span_by_candidate.push((edge.relationship_id.clone(), span, None));
        }
    }
    let ranked = rank_candidates(features, &RankingWeights::default());
    trace.candidates = ranked.clone();

    // 6. Evidence snippets (MT-135) + bundle candidates in RANKED order, then
    //    compile + persist the bounded bundle and its replayable trace
    //    (MT-136/138).
    let mut bundle_candidates: Vec<BundleCandidate> = Vec::with_capacity(ranked.len());
    for candidate in &ranked {
        let (_, span_id, token_count) = snippet_span_by_candidate
            .iter()
            .find(|(id, _, _)| id == &candidate.candidate_id)
            .cloned()
            .unwrap_or((candidate.candidate_id.clone(), None, None));
        let snippet: Option<EvidenceSnippet> = match span_id {
            Some(span_id) => Some(assemble_span_snippet(db, &span_id).await?),
            None => None,
        };
        let token_count = token_count
            .map(|t| t.max(1) as u32)
            .or_else(|| {
                snippet
                    .as_ref()
                    .and_then(|s| s.excerpt.as_ref())
                    .map(|e| (e.len() as u32 / 4).max(1))
            })
            .unwrap_or(8);
        let tier = if candidate.kind == "passage_ref" {
            PriorityTier::Supplementary
        } else {
            PriorityTier::Authoritative
        };
        bundle_candidates.push(BundleCandidate {
            ref_kind: bundle_ref_kind(&candidate.kind),
            ref_id: candidate.candidate_id.clone(),
            tier,
            token_count,
            relevance_score: candidate.base_score,
            source_id: snippet
                .as_ref()
                .map(|s| s.source_id.clone())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| candidate.candidate_id.clone()),
            snippet,
        });
    }

    let compiled = ContextBundleCompilerV2::new(db)
        .compile(
            &request.workspace_id,
            kernel_task_run_id,
            session_run_id,
            target_kind,
            target_ref,
            &planned.plan,
            &mut trace,
            &bundle_candidates,
            None,
            None,
        )
        .await?;

    Ok(ExecutedRetrieval {
        schema_terms_matched: schema.matched_term_ids.len(),
        schema_off_topic_dropped: schema.off_topic_dropped,
        graph_edge_count: graph.edges.len(),
        fallback_reason: decision.reason.map(|r| r.as_str().to_string()),
        ranked,
        compiled,
        planned,
    })
}

/// Map a ranked candidate kind to the bundle-item ref vocabulary.
fn bundle_ref_kind(kind: &str) -> crate::storage::knowledge::KnowledgeBundleItemRefKind {
    use crate::storage::knowledge::KnowledgeBundleItemRefKind as K;
    match kind {
        "passage_ref" => K::Passage,
        "span_ref" => K::Span,
        "source_ref" => K::Source,
        "claim_ref" => K::Claim,
        _ => K::Entity,
    }
}

/// Re-exported for callers that only need the decision surface.
pub use crate::knowledge_retrieval::passage_fallback::PassageFallbackReason;

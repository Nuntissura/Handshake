//! WP-KERNEL-009 RetrievalContextAndRanking (MT-129..MT-144) — real-PostgreSQL
//! proof for the planner / trace / graph-traversal / compiler / bridges.
//!
//! Proof-path contract (spec 2.3.13.11): PostgreSQL + EventLedger authority
//! only. Reuses the committed `MemoryFixture` (workspace -> source -> span,
//! entity/claim helpers) and the managed-PG auto-discovery in
//! `knowledge_pg_support`. NO SQLite, NO mocks: when PostgreSQL binaries are
//! absent the fixture returns `None` and each test SKIPs loudly.
//!
//! Central proofs:
//!   * MT-130/MT-138: a query with a KNOWN entity id plans `direct_load` and
//!     records WHY it skipped hybrid, persisted as a replayable RetrievalTrace
//!     whose `decisions` JSONB round-trips the QueryPlan + RetrievalTrace.
//!   * MT-131: schema-first filtering narrows facts to the query's ontology
//!     scope.
//!   * MT-132: a bounded graph traversal over a real edge yields non-empty
//!     candidates citing the stable relationship_id.
//!   * MT-136/MT-137: the compiler persists a bounded kernel ContextBundle V1 +
//!     its trace, dropping over-budget items with a recorded reason.
//!   * MT-140: the SemanticCatalog routing contract is backend-queryable.
//!   * MT-144: end-to-end — index -> plan in a mode -> assert trace + ranked
//!     evidence + context bundle.

#[path = "knowledge_memory_fixtures.rs"]
mod knowledge_memory_fixtures;

use handshake_core::knowledge_retrieval::ai_ready_export::build_evidence_manifest;
use handshake_core::knowledge_retrieval::budget::PriorityTier;
use handshake_core::knowledge_retrieval::compiler::{
    BundleCandidate, BundleTargetKind, ContextBundleCompilerV2,
};
use handshake_core::knowledge_retrieval::executor::execute_retrieval;
use handshake_core::knowledge_retrieval::graph_planner::{
    GraphTraversalPlanner, GraphTraversalPolicy,
};
use handshake_core::knowledge_retrieval::plan::{RetrievalBudgets, RetrievalTrace};
use handshake_core::knowledge_retrieval::planner::{
    AuthoritativeHandle, CheapestAuthoritativePathPlanner, RetrievalRequest,
};
use handshake_core::knowledge_retrieval::snippet::assemble_span_snippet;
use handshake_core::memory::retrieval_mode::{NonHybridReason, QueryRetrievalMode};
use handshake_core::storage::knowledge::{
    KnowledgeBundleItemRefKind, KnowledgeCompactionPolicy, KnowledgeEdgeType,
    KnowledgePassageEvidenceRef, KnowledgeRetrievalMode, KnowledgeStore, NewKnowledgeEdge,
    NewKnowledgeMemoryPassage,
};
use handshake_core::storage::knowledge_retrieval::{
    record_retrieval_trace, traces_for_bundle, upsert_semantic_catalog_entry,
    NewSemanticCatalogEntry, SemanticCatalogKind,
};
use std::collections::BTreeSet;

use knowledge_memory_fixtures::{pool_for, MemoryFixture};

macro_rules! skip_if_no_pg {
    ($opt:expr, $name:literal) => {
        match $opt {
            Some(value) => value,
            None => {
                eprintln!(concat!("SKIP ", $name, ": PostgreSQL unavailable"));
                return;
            }
        }
    };
}

/// MT-130 + MT-138: a known entity id => direct_load, skip reason recorded, and
/// the trace persists replayably.
#[tokio::test]
async fn known_entity_plans_direct_load_and_records_skip_reason() {
    let fx = skip_if_no_pg!(
        MemoryFixture::setup().await,
        "known_entity_plans_direct_load"
    );

    // Seed a real entity the caller "already holds".
    let entity_id = fx
        .entity("symbol", "retrieval_planner", "RetrievalPlanner")
        .await;

    let planner = CheapestAuthoritativePathPlanner::new(&fx.pg.db);
    let request = RetrievalRequest::discovery(&fx.workspace_id, "load RetrievalPlanner")
        .with_handle(AuthoritativeHandle::EntityId(entity_id.clone()));
    let planned = planner.plan(&request).await.expect("plan");

    // Cheapest authoritative mode: direct_load, skipping hybrid with a reason.
    assert_eq!(planned.plan.retrieval_mode, QueryRetrievalMode::DirectLoad);
    assert_eq!(
        planned.plan.non_hybrid_reason,
        Some(NonHybridReason::ExactIdentifierKnown)
    );
    assert!(planned.plan.validate().is_ok());
    assert_eq!(
        planned.confirmed_handle.as_ref().map(|c| c.id.as_str()),
        Some(entity_id.as_str())
    );

    // Persist the trace and prove it is replayable from the decisions JSONB.
    let trace = RetrievalTrace::for_plan(&planned.plan);
    let stored = record_retrieval_trace(
        &fx.pg.db,
        &fx.workspace_id,
        &planned.plan,
        &trace,
        None,
        None,
    )
    .await
    .expect("record trace");
    assert_eq!(stored.retrieval_mode.as_str(), "direct_load");
    assert!(stored.mode_reason.contains("exact_identifier_known"));
    // decisions round-trips the plan id + mode.
    assert_eq!(
        stored.decisions["query_plan"]["plan_id"],
        planned.plan.plan_id
    );
    assert_eq!(
        stored.decisions["retrieval_trace"]["retrieval_mode"],
        "direct_load"
    );
}

/// MT-130: a DANGLING entity handle does NOT degrade into a false direct_load —
/// the planner widens (here, to hybrid discovery) because the handle could not
/// be confirmed against the index.
#[tokio::test]
async fn dangling_handle_degrades_to_discovery() {
    let fx = skip_if_no_pg!(MemoryFixture::setup().await, "dangling_handle_degrades");

    let planner = CheapestAuthoritativePathPlanner::new(&fx.pg.db);
    let request = RetrievalRequest::discovery(&fx.workspace_id, "load ghost").with_handle(
        AuthoritativeHandle::EntityId("ENT-does-not-exist".to_string()),
    );
    let planned = planner.plan(&request).await.expect("plan");

    assert_eq!(planned.plan.retrieval_mode, QueryRetrievalMode::HybridRag);
    assert!(planned.confirmed_handle.is_none());
    // Adversarial-v2: the dangle is RECORDED, not silent.
    assert_eq!(planned.dangling_handles.len(), 1);
    assert_eq!(planned.dangling_handles[0].kind, "entity");
    assert_eq!(planned.dangling_handles[0].id, "ENT-does-not-exist");
}

/// Adversarial-v2 MT-130: WP / MT / relationship handles are existence-checked
/// exactly like entity/source handles — a dangling one degrades the plan to a
/// broader mode AND is recorded, never a false direct/exact plan.
#[tokio::test]
async fn dangling_wp_mt_relationship_handles_degrade_and_record() {
    let fx = skip_if_no_pg!(
        MemoryFixture::setup().await,
        "dangling_wp_mt_relationship_handles"
    );

    let planner = CheapestAuthoritativePathPlanner::new(&fx.pg.db);
    let request = RetrievalRequest::discovery(&fx.workspace_id, "load ghosts")
        .with_handle(AuthoritativeHandle::WorkPacketId("WP-GHOST-001".to_string()))
        .with_handle(AuthoritativeHandle::MicroTaskId("MT-999".to_string()))
        .with_handle(AuthoritativeHandle::RelationshipId(
            "KREL-0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        ));
    let planned = planner.plan(&request).await.expect("plan");

    // Before the hardening this produced an unsatisfiable direct_load on the
    // ghost WP id; now it degrades to discovery with all three dangles named.
    assert_eq!(planned.plan.retrieval_mode, QueryRetrievalMode::HybridRag);
    assert!(planned.confirmed_handle.is_none());
    let kinds: Vec<&str> = planned
        .dangling_handles
        .iter()
        .map(|d| d.kind.as_str())
        .collect();
    assert_eq!(kinds, vec!["work_packet", "micro_task", "relationship"]);
    assert!(planned.dangling_handles[0].reason().contains("WP-GHOST-001"));
}

/// Adversarial-v2 MT-130: an INDEXED work-packet / micro-task / relationship
/// handle confirms against the ProjectKnowledgeIndex and anchors the cheapest
/// authoritative mode (direct_load / exact_lookup), with no dangles recorded.
#[tokio::test]
async fn indexed_wp_mt_relationship_handles_confirm() {
    let fx = skip_if_no_pg!(
        MemoryFixture::setup().await,
        "indexed_wp_mt_relationship_handles"
    );

    // Index the WP + MT as ProjectKnowledgeIndex entities (the packet store)
    // and a real edge whose relationship_id is the exact graph handle.
    fx.entity("work_packet", "WP-KERNEL-009", "WP-KERNEL-009").await;
    fx.entity("micro_task", "MT-130", "MT-130").await;
    let a = fx.entity("symbol", "edge_a", "EdgeA").await;
    let b = fx.entity("symbol", "edge_b", "EdgeB").await;
    let edge = fx
        .pg
        .db
        .upsert_knowledge_edge(NewKnowledgeEdge {
            workspace_id: fx.workspace_id.clone(),
            edge_type: KnowledgeEdgeType::DependsOn,
            source_entity_id: a,
            target_entity_id: b,
            extractor_version: "test_v1".to_string(),
            confidence: 0.9,
            detected_in_run: None,
            evidence_span_ids: vec![fx.span_id.clone()],
        })
        .await
        .expect("edge");

    let planner = CheapestAuthoritativePathPlanner::new(&fx.pg.db);

    // Confirmed WP handle -> direct_load.
    let planned = planner
        .plan(
            &RetrievalRequest::discovery(&fx.workspace_id, "load WP").with_handle(
                AuthoritativeHandle::WorkPacketId("WP-KERNEL-009".to_string()),
            ),
        )
        .await
        .expect("plan wp");
    assert_eq!(planned.plan.retrieval_mode, QueryRetrievalMode::DirectLoad);
    assert_eq!(
        planned.confirmed_handle.as_ref().map(|c| c.kind.as_str()),
        Some("work_packet")
    );
    assert!(planned.dangling_handles.is_empty());

    // Confirmed MT handle -> direct_load.
    let planned = planner
        .plan(
            &RetrievalRequest::discovery(&fx.workspace_id, "load MT")
                .with_handle(AuthoritativeHandle::MicroTaskId("MT-130".to_string())),
        )
        .await
        .expect("plan mt");
    assert_eq!(planned.plan.retrieval_mode, QueryRetrievalMode::DirectLoad);
    assert_eq!(
        planned.confirmed_handle.as_ref().map(|c| c.kind.as_str()),
        Some("micro_task")
    );

    // Confirmed relationship handle -> exact_lookup citing the stable id.
    let planned = planner
        .plan(
            &RetrievalRequest::discovery(&fx.workspace_id, "load edge").with_handle(
                AuthoritativeHandle::RelationshipId(edge.relationship_id.clone()),
            ),
        )
        .await
        .expect("plan rel");
    assert_eq!(planned.plan.retrieval_mode, QueryRetrievalMode::ExactLookup);
    assert_eq!(
        planned.confirmed_handle.as_ref().map(|c| c.id.as_str()),
        Some(edge.relationship_id.as_str())
    );

    // A dangling WP followed by a REAL entity: the plan anchors on the entity
    // and records the WP dangle (degrade, not fail).
    let entity_id = fx.entity("symbol", "fallback_anchor", "Anchor").await;
    let planned = planner
        .plan(
            &RetrievalRequest::discovery(&fx.workspace_id, "load mixed")
                .with_handle(AuthoritativeHandle::WorkPacketId("WP-GHOST-XYZ".to_string()))
                .with_handle(AuthoritativeHandle::EntityId(entity_id.clone())),
        )
        .await
        .expect("plan mixed");
    assert_eq!(planned.plan.retrieval_mode, QueryRetrievalMode::DirectLoad);
    assert_eq!(
        planned.confirmed_handle.as_ref().map(|c| c.id.as_str()),
        Some(entity_id.as_str())
    );
    assert_eq!(planned.dangling_handles.len(), 1);
    assert_eq!(planned.dangling_handles[0].kind, "work_packet");
}

/// MT-132: a bounded graph traversal over a real edge yields non-empty
/// candidates and cites the stable relationship_id (folded RelationshipIds
/// intent).
#[tokio::test]
async fn graph_traversal_yields_candidates_with_relationship_ids() {
    let fx = skip_if_no_pg!(
        MemoryFixture::setup().await,
        "graph_traversal_yields_candidates"
    );
    let pool = pool_for(&fx.pg).await;

    let a = fx.entity("symbol", "module_a", "ModuleA").await;
    let b = fx.entity("symbol", "module_b", "ModuleB").await;
    let edge = fx
        .pg
        .db
        .upsert_knowledge_edge(NewKnowledgeEdge {
            workspace_id: fx.workspace_id.clone(),
            edge_type: KnowledgeEdgeType::DependsOn,
            source_entity_id: a.clone(),
            target_entity_id: b.clone(),
            extractor_version: "test_v1".to_string(),
            confidence: 0.9,
            detected_in_run: None,
            evidence_span_ids: vec![fx.span_id.clone()],
        })
        .await
        .expect("edge");

    let planner = GraphTraversalPlanner::new(&fx.pg.db, &pool, GraphTraversalPolicy::default());
    let seeds = BTreeSet::from([a.clone()]);
    let result = planner.traverse(&seeds).await.expect("traverse");

    assert!(
        result.has_candidates(),
        "graph traversal must yield candidates"
    );
    assert!(result
        .cited_relationship_ids()
        .contains(&edge.relationship_id));
    // b was reachable from a.
    assert!(result.visited.iter().any(|n| n.entity_id == b));
}

/// MT-132: edge-type allowlist excludes non-allowlisted edges from expansion.
#[tokio::test]
async fn graph_traversal_allowlist_excludes_other_edge_types() {
    let fx = skip_if_no_pg!(MemoryFixture::setup().await, "graph_traversal_allowlist");
    let pool = pool_for(&fx.pg).await;

    let a = fx.entity("symbol", "a_node", "A").await;
    let b = fx.entity("symbol", "b_node", "B").await;
    fx.pg
        .db
        .upsert_knowledge_edge(NewKnowledgeEdge {
            workspace_id: fx.workspace_id.clone(),
            edge_type: KnowledgeEdgeType::Mentions,
            source_entity_id: a.clone(),
            target_entity_id: b.clone(),
            extractor_version: "test_v1".to_string(),
            confidence: 0.9,
            detected_in_run: None,
            evidence_span_ids: vec![fx.span_id.clone()],
        })
        .await
        .expect("edge");

    // Allow only DependsOn — the Mentions edge must not be followed.
    let policy = GraphTraversalPolicy::default().with_edge_types([KnowledgeEdgeType::DependsOn]);
    let planner = GraphTraversalPlanner::new(&fx.pg.db, &pool, policy);
    let result = planner
        .traverse(&BTreeSet::from([a.clone()]))
        .await
        .expect("traverse");
    assert!(!result.has_candidates());
}

/// MT-135: an evidence snippet for a real span carries the source path, range,
/// content hash, and a supported citation.
#[tokio::test]
async fn evidence_snippet_carries_span_citation() {
    let fx = skip_if_no_pg!(MemoryFixture::setup().await, "evidence_snippet");
    let snippet = assemble_span_snippet(&fx.pg.db, &fx.span_id)
        .await
        .expect("snippet");
    assert!(snippet.supported);
    assert_eq!(snippet.source_id, fx.source_id);
    assert!(snippet.content_sha256.starts_with("bbbb"));
    assert!(snippet.citation().contains("memory/graph.rs"));
}

/// MT-136 + MT-137: the compiler persists a bounded kernel ContextBundle V1 +
/// its trace; an over-budget item is dropped with a recorded decision.
#[tokio::test]
async fn compiler_persists_bounded_bundle_and_drops_over_budget() {
    let fx = skip_if_no_pg!(
        MemoryFixture::setup().await,
        "compiler_persists_bounded_bundle"
    );

    // Plan a direct_load for a known entity (so a non-hybrid reason is set).
    let entity_id = fx.entity("symbol", "bundle_target", "BundleTarget").await;
    let planner = CheapestAuthoritativePathPlanner::new(&fx.pg.db);
    let mut request = RetrievalRequest::discovery(&fx.workspace_id, "compile bundle")
        .with_handle(AuthoritativeHandle::EntityId(entity_id.clone()));
    // Tight token budget: room for exactly one of the two 30-token items.
    request.budgets = RetrievalBudgets {
        max_total_evidence_tokens: 30,
        ..RetrievalBudgets::default_bounded()
    };
    let planned = planner.plan(&request).await.expect("plan");
    let mut trace = RetrievalTrace::for_plan(&planned.plan);

    let snippet = assemble_span_snippet(&fx.pg.db, &fx.span_id)
        .await
        .expect("snippet");
    let candidates = vec![
        BundleCandidate {
            ref_kind: KnowledgeBundleItemRefKind::Span,
            ref_id: fx.span_id.clone(),
            tier: PriorityTier::Authoritative,
            token_count: 30,
            relevance_score: 0.9,
            source_id: fx.source_id.clone(),
            snippet: Some(snippet.clone()),
        },
        BundleCandidate {
            ref_kind: KnowledgeBundleItemRefKind::Span,
            ref_id: format!("{}-extra", fx.span_id),
            tier: PriorityTier::Supplementary,
            token_count: 30,
            relevance_score: 0.5,
            source_id: format!("{}-other", fx.source_id),
            snippet: Some(snippet),
        },
    ];

    let compiler = ContextBundleCompilerV2::new(&fx.pg.db);
    let compiled = compiler
        .compile(
            &fx.workspace_id,
            "ktr-1",
            "sr-1",
            BundleTargetKind::Symbol,
            &entity_id,
            &planned.plan,
            &mut trace,
            &candidates,
            None,
            None,
        )
        .await
        .expect("compile");

    // Bounded: only the authoritative item fit the 30-token budget.
    assert_eq!(compiled.tokens_used, 30);
    assert!(compiled.allocation.is_admitted(&fx.span_id));
    assert!(!compiled
        .allocation
        .is_admitted(&format!("{}-extra", fx.span_id)));

    // The bundle + its trace are persisted and bound together (replayable).
    let (bundle, items) = fx
        .pg
        .db
        .get_knowledge_context_bundle(&compiled.bundle_id)
        .await
        .expect("get bundle")
        .expect("bundle exists");
    assert_eq!(bundle.bundle_id, compiled.bundle_id);
    assert_eq!(items.len(), 2);
    let traces = traces_for_bundle(&fx.pg.db, &compiled.bundle_id)
        .await
        .expect("traces");
    assert_eq!(traces.len(), 1);
    assert_eq!(
        traces[0].bundle_id.as_deref(),
        Some(compiled.bundle_id.as_str())
    );

    // MT-141: the AI-ready export manifest is reconstructable from these rows.
    let manifest = build_evidence_manifest(&bundle, &traces);
    assert!(manifest.reconstructable);
    assert_eq!(manifest.dialect, "ai_ready_evidence_export@1");
}

/// Adversarial-v2 MT-133 + MT-134: ONE EXECUTED
/// request -> plan -> schema_filter -> graph -> passage_fallback -> rank ->
/// snippet -> compile path against real PostgreSQL, driving the PASSAGE
/// FALLBACK end-to-end: an empty graph falls back to committed passages, the
/// passages are ranked deterministically, cited, compiled into a persisted
/// bundle, and the trace records WHY.
#[tokio::test]
async fn executed_pipeline_falls_back_to_passages_ranks_and_compiles() {
    let fx = skip_if_no_pg!(
        MemoryFixture::setup().await,
        "executed_pipeline_falls_back"
    );
    let pool = pool_for(&fx.pg).await;

    // Two committed passages with DIFFERENT extraction confidence, both
    // span-backed (the citation evidence).
    let mut passage_ids = Vec::new();
    for (text, confidence) in [
        ("managed postgres listens on port 5544", 0.9_f64),
        ("an older note about postgres ports", 0.4_f64),
    ] {
        let passage = fx
            .pg
            .db
            .create_knowledge_memory_passage(NewKnowledgeMemoryPassage {
                workspace_id: fx.workspace_id.clone(),
                passage_text: text.to_string(),
                token_count: Some(12),
                ocr_transcript_metadata: None,
                extraction_confidence: confidence,
                ranking_features: serde_json::json!({}),
                retrieval_mode: KnowledgeRetrievalMode::HybridRag,
                compaction_policy: KnowledgeCompactionPolicy::Keep,
                failure_receipt_event_id: None,
                derived_in_run: None,
                evidence: vec![KnowledgePassageEvidenceRef::Span {
                    span_id: fx.span_id.clone(),
                }],
            })
            .await
            .expect("passage");
        passage_ids.push(passage.passage_id);
    }

    // A graph-neighborhood request with NO edges in the workspace: the graph
    // step yields nothing, so the executed pipeline MUST fall back.
    let mut request =
        RetrievalRequest::discovery(&fx.workspace_id, "what port does managed postgres use");
    request.graph_neighborhood_expected = true;
    let executed = execute_retrieval(
        &fx.pg.db,
        &pool,
        "ktr-exec-fb",
        "sr-exec-fb",
        BundleTargetKind::Task,
        "port-question",
        &request,
        &BTreeSet::new(),
        GraphTraversalPolicy::default(),
    )
    .await
    .expect("execute");

    // The fallback FIRED with the recorded reason (MT-133 executed).
    assert_eq!(executed.graph_edge_count, 0);
    assert_eq!(
        executed.fallback_reason.as_deref(),
        Some("graph_candidates_missing")
    );

    // Ranking ran over the REAL fallback passages (MT-134 executed): the
    // higher-confidence passage outranks the lower one deterministically.
    assert_eq!(executed.ranked.len(), 2);
    assert_eq!(executed.ranked[0].candidate_id, passage_ids[0]);
    assert_eq!(executed.ranked[1].candidate_id, passage_ids[1]);
    assert!(executed.ranked[0].base_score > executed.ranked[1].base_score);
    assert_eq!(executed.ranked[0].kind, "passage_ref");

    // The bundle + trace are PERSISTED and bound together (MT-136/138).
    let (bundle, items) = fx
        .pg
        .db
        .get_knowledge_context_bundle(&executed.compiled.bundle_id)
        .await
        .expect("get bundle")
        .expect("bundle exists");
    assert_eq!(bundle.bundle_id, executed.compiled.bundle_id);
    assert_eq!(items.len(), 2);
    assert!(items.iter().all(|i| i.ref_kind.as_str() == "passage"));
    assert!(
        items
            .iter()
            .any(|i| i.citation.as_deref().is_some_and(|c| c.contains("memory/graph.rs"))),
        "passage items carry span-backed citations: {items:?}"
    );

    let traces = traces_for_bundle(&fx.pg.db, &executed.compiled.bundle_id)
        .await
        .expect("traces");
    assert_eq!(traces.len(), 1);
    // The durable mode column is hybrid_rag (the MT-129 mode law), the durable
    // mode_reason carries the fallback rationale, and the decisions JSONB
    // keeps the precise passage_fallback posture + ranked candidates
    // replayably.
    assert_eq!(traces[0].retrieval_mode.as_str(), "hybrid_rag");
    assert!(
        traces[0].mode_reason.contains("passage fallback")
            && traces[0].mode_reason.contains("graph_candidates_missing"),
        "mode_reason must carry the fallback WHY: {}",
        traces[0].mode_reason
    );
    assert_eq!(
        traces[0].decisions["retrieval_trace"]["retrieval_mode"],
        "passage_fallback"
    );
    let warnings = traces[0].decisions["retrieval_trace"]["warnings"]
        .as_array()
        .expect("warnings")
        .iter()
        .map(|w| w.as_str().unwrap_or_default().to_string())
        .collect::<Vec<_>>();
    assert!(
        warnings.iter().any(|w| w.contains("graph_candidates_missing")),
        "the trace records WHY the fallback fired: {warnings:?}"
    );
    let candidates = traces[0].decisions["retrieval_trace"]["candidates"]
        .as_array()
        .expect("candidates");
    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0]["candidate_id"], passage_ids[0].as_str());
}

/// Adversarial-v2 MT-134: the SAME executed path with a REAL graph: the
/// traversal's edges become ranked entity candidates (graph features, no
/// fallback) and compile into a persisted bundle.
#[tokio::test]
async fn executed_pipeline_graph_path_ranks_and_compiles() {
    let fx = skip_if_no_pg!(
        MemoryFixture::setup().await,
        "executed_pipeline_graph_path"
    );
    let pool = pool_for(&fx.pg).await;

    // A real 2-edge neighborhood around a seed entity.
    let hub = fx.entity("symbol", "managed_postgres", "ManagedPostgres").await;
    let port = fx.entity("symbol", "pg_port", "PgPort").await;
    let config = fx.entity("symbol", "pg_config", "PgConfig").await;
    for (target, edge_type, confidence) in [
        (&port, KnowledgeEdgeType::Defines, 0.95_f64),
        (&config, KnowledgeEdgeType::Mentions, 0.5_f64),
    ] {
        fx.pg
            .db
            .upsert_knowledge_edge(NewKnowledgeEdge {
                workspace_id: fx.workspace_id.clone(),
                edge_type,
                source_entity_id: hub.clone(),
                target_entity_id: (*target).clone(),
                extractor_version: "test_v1".to_string(),
                confidence,
                detected_in_run: None,
                evidence_span_ids: vec![fx.span_id.clone()],
            })
            .await
            .expect("edge");
    }

    // A neighborhood query anchored on the hub as the traversal seed (the
    // Loom "what links to X" shape: the anchor is known, the answer is not).
    let mut request =
        RetrievalRequest::discovery(&fx.workspace_id, "managed postgres neighborhood");
    request.graph_neighborhood_expected = true;
    let executed = execute_retrieval(
        &fx.pg.db,
        &pool,
        "ktr-exec-graph",
        "sr-exec-graph",
        BundleTargetKind::Symbol,
        &hub,
        &request,
        &BTreeSet::from([hub.clone()]),
        GraphTraversalPolicy::default(),
    )
    .await
    .expect("execute");

    // No fallback: the graph satisfied the task (MT-133 negative half).
    assert!(executed.fallback_reason.is_none());
    assert_eq!(executed.graph_edge_count, 2);

    // MT-134 executed: real traversed edges ranked with structure-aware
    // features — the high-confidence `defines` edge outranks the low
    // `mentions` edge.
    assert_eq!(executed.ranked.len(), 2);
    assert!(executed.ranked[0].base_score > executed.ranked[1].base_score);
    assert_eq!(executed.ranked[0].kind, "entity_ref");
    assert!(executed.ranked[0].scores.graph.is_some());

    // Persisted bundle with span-cited items + bound trace.
    let (bundle, items) = fx
        .pg
        .db
        .get_knowledge_context_bundle(&executed.compiled.bundle_id)
        .await
        .expect("get bundle")
        .expect("bundle exists");
    assert_eq!(bundle.bundle_id, executed.compiled.bundle_id);
    assert_eq!(items.len(), 2);
    assert!(items
        .iter()
        .all(|i| i.citation.as_deref().is_some_and(|c| c.contains("memory/graph.rs"))));
    let traces = traces_for_bundle(&fx.pg.db, &executed.compiled.bundle_id)
        .await
        .expect("traces");
    assert_eq!(traces.len(), 1);
    assert_eq!(traces[0].retrieval_mode.as_str(), "graph_traversal");
}

/// Adversarial-v2 MT-140: when a request names a SemanticCatalog entry, the
/// PLANNER derives plan.route[] from the catalog's declared routing contract
/// (spec 2.6.6.7.14.6 A) — and falls back to defaults when the entry is
/// absent.
#[tokio::test]
async fn planner_derives_route_from_semantic_catalog_entry() {
    let fx = skip_if_no_pg!(
        MemoryFixture::setup().await,
        "planner_derives_route_from_catalog"
    );
    let pool = pool_for(&fx.pg).await;

    upsert_semantic_catalog_entry(
        &pool,
        NewSemanticCatalogEntry {
            workspace_id: fx.workspace_id.clone(),
            entry_kind: SemanticCatalogKind::Index,
            name: "loom_neighborhood".to_string(),
            version: 1,
            description: "Loom graph neighborhood index".to_string(),
            query_routes: vec![
                "knowledge_graph".to_string(),
                "shadow_ws_vector".to_string(),
            ],
            supported_selectors: vec!["loom_block".to_string()],
            default_budgets: None,
            examples: serde_json::json!([]),
        },
    )
    .await
    .expect("upsert catalog");

    let planner = CheapestAuthoritativePathPlanner::new(&fx.pg.db);

    // Hybrid discovery governed by the catalog entry: route IS the contract.
    let planned = planner
        .plan(
            &RetrievalRequest::discovery(&fx.workspace_id, "find related loom blocks")
                .with_catalog_entry("loom_neighborhood"),
        )
        .await
        .expect("plan hybrid");
    assert_eq!(planned.plan.retrieval_mode, QueryRetrievalMode::HybridRag);
    assert_eq!(planned.plan.route.len(), 2, "route derived from the catalog");
    assert!(
        planned.plan.route[0]
            .purpose
            .contains("semantic catalog 'loom_neighborhood'"),
        "route steps cite the catalog contract: {:?}",
        planned.plan.route
    );
    assert!(planned.plan.route[1]
        .purpose
        .contains("route 'shadow_ws_vector'"));

    // Graph-neighborhood mode is governed by the catalog too.
    let mut graph_request = RetrievalRequest::discovery(&fx.workspace_id, "expand neighborhood")
        .with_catalog_entry("loom_neighborhood");
    graph_request.graph_neighborhood_expected = true;
    let planned = planner.plan(&graph_request).await.expect("plan graph");
    assert_eq!(
        planned.plan.retrieval_mode,
        QueryRetrievalMode::GraphTraversal
    );
    assert!(planned.plan.route[0].purpose.contains("semantic catalog"));

    // An UNKNOWN entry name falls back to the default routing (spec C).
    let planned = planner
        .plan(
            &RetrievalRequest::discovery(&fx.workspace_id, "discover")
                .with_catalog_entry("no_such_contract"),
        )
        .await
        .expect("plan fallback");
    assert_eq!(planned.plan.retrieval_mode, QueryRetrievalMode::HybridRag);
    assert_eq!(planned.plan.route.len(), 4, "default hybrid route");
    assert!(planned.plan.route[0].purpose.contains("ContextPack"));
}

/// Adversarial-v2 MT-144: the three previously label-only scenarios are now
/// DRIVEN against real PostgreSQL (the other four — exact lookup, graph
/// traversal, hybrid, passage fallback — are driven by the tests above):
///   * stale_graph — a REAL edge + uncertain freshness falls back with the
///     `graph_candidates_stale` reason;
///   * no_result_recovery — nothing to retrieve yields a persisted, bounded,
///     explainable EMPTY bundle (recovery without hallucination);
///   * bad_citation — an unsupported citation is surfaced with the explicit
///     `supported=false` marker in the persisted bundle, and the FK law makes
///     a dangling citation unconstructable through storage.
#[tokio::test]
async fn retrieval_scenarios_stale_graph_no_result_and_bad_citation_are_driven() {
    let fx = skip_if_no_pg!(
        MemoryFixture::setup().await,
        "retrieval_scenarios_stale_graph"
    );
    let pool = pool_for(&fx.pg).await;

    // ---- stale_graph ------------------------------------------------------
    let a = fx.entity("symbol", "stale_a", "StaleA").await;
    let b = fx.entity("symbol", "stale_b", "StaleB").await;
    fx.pg
        .db
        .upsert_knowledge_edge(NewKnowledgeEdge {
            workspace_id: fx.workspace_id.clone(),
            edge_type: KnowledgeEdgeType::DependsOn,
            source_entity_id: a.clone(),
            target_entity_id: b.clone(),
            extractor_version: "test_v1".to_string(),
            confidence: 0.9,
            detected_in_run: None,
            evidence_span_ids: vec![fx.span_id.clone()],
        })
        .await
        .expect("edge");
    fx.pg
        .db
        .create_knowledge_memory_passage(NewKnowledgeMemoryPassage {
            workspace_id: fx.workspace_id.clone(),
            passage_text: "fallback passage for the stale graph".to_string(),
            token_count: Some(8),
            ocr_transcript_metadata: None,
            extraction_confidence: 0.8,
            ranking_features: serde_json::json!({}),
            retrieval_mode: KnowledgeRetrievalMode::HybridRag,
            compaction_policy: KnowledgeCompactionPolicy::Keep,
            failure_receipt_event_id: None,
            derived_in_run: None,
            evidence: vec![KnowledgePassageEvidenceRef::Span {
                span_id: fx.span_id.clone(),
            }],
        })
        .await
        .expect("passage");

    let mut request = RetrievalRequest::discovery(&fx.workspace_id, "stale neighborhood");
    request.graph_neighborhood_expected = true;
    request.freshness_uncertain = true; // the staleness signal
    let executed = execute_retrieval(
        &fx.pg.db,
        &pool,
        "ktr-stale",
        "sr-stale",
        BundleTargetKind::Symbol,
        &a,
        &request,
        &BTreeSet::from([a.clone()]),
        GraphTraversalPolicy::default(),
    )
    .await
    .expect("execute stale");
    assert!(executed.graph_edge_count > 0, "the graph HAS candidates");
    assert_eq!(
        executed.fallback_reason.as_deref(),
        Some("graph_candidates_stale"),
        "uncertain freshness over a present graph falls back as STALE"
    );
    assert!(!executed.ranked.is_empty(), "fallback passages ranked");

    // ---- no_result_recovery ------------------------------------------------
    // A separate empty workspace: no edges, no passages, nothing to retrieve.
    let empty_ws = fx.pg.create_workspace().await;
    let mut request = RetrievalRequest::discovery(&empty_ws, "anything at all");
    request.graph_neighborhood_expected = true;
    let executed = execute_retrieval(
        &fx.pg.db,
        &pool,
        "ktr-noresult",
        "sr-noresult",
        BundleTargetKind::Task,
        "no-result",
        &request,
        &BTreeSet::new(),
        GraphTraversalPolicy::default(),
    )
    .await
    .expect("execute no-result");
    assert_eq!(
        executed.fallback_reason.as_deref(),
        Some("graph_candidates_missing")
    );
    assert!(executed.ranked.is_empty(), "nothing to rank");
    // Recovery = a PERSISTED, bounded, explainable empty bundle.
    let (bundle, items) = fx
        .pg
        .db
        .get_knowledge_context_bundle(&executed.compiled.bundle_id)
        .await
        .expect("get bundle")
        .expect("empty bundle persists");
    assert_eq!(items.len(), 0, "no fabricated items");
    assert_eq!(bundle.tokens_used, Some(0));
    let traces = traces_for_bundle(&fx.pg.db, &executed.compiled.bundle_id)
        .await
        .expect("traces");
    assert_eq!(traces.len(), 1, "the empty result is explainable");

    // ---- bad_citation -------------------------------------------------------
    // (1) A citation the index cannot back is SURFACED, never silent: the
    // snippet assembler marks it supported=false and the compiled bundle's
    // allowed_context records the marker.
    let ghost = assemble_span_snippet(&fx.pg.db, "KSP-00000000000000000000000000000000")
        .await
        .expect("assemble ghost snippet");
    assert!(!ghost.supported);
    let planner = CheapestAuthoritativePathPlanner::new(&fx.pg.db);
    let planned = planner
        .plan(
            &RetrievalRequest::discovery(&fx.workspace_id, "bad citation scenario").with_handle(
                AuthoritativeHandle::EntityId(a.clone()),
            ),
        )
        .await
        .expect("plan");
    let mut trace = RetrievalTrace::for_plan(&planned.plan);
    let compiled = ContextBundleCompilerV2::new(&fx.pg.db)
        .compile(
            &fx.workspace_id,
            "ktr-badcite",
            "sr-badcite",
            BundleTargetKind::Symbol,
            &a,
            &planned.plan,
            &mut trace,
            &[BundleCandidate {
                ref_kind: KnowledgeBundleItemRefKind::Span,
                ref_id: "KSP-00000000000000000000000000000000".to_string(),
                tier: PriorityTier::Authoritative,
                token_count: 8,
                relevance_score: 0.9,
                source_id: "ghost-source".to_string(),
                snippet: Some(ghost),
            }],
            None,
            None,
        )
        .await
        .expect("compile bad citation");
    let (bundle, _items) = fx
        .pg
        .db
        .get_knowledge_context_bundle(&compiled.bundle_id)
        .await
        .expect("get bundle")
        .expect("bundle exists");
    assert_eq!(
        bundle.allowed_context["items"][0]["supported"],
        serde_json::json!(false),
        "the persisted bundle carries the unsupported-citation marker: {}",
        bundle.allowed_context
    );

    // (2) The FK law makes a DANGLING citation unconstructable via storage:
    // deleting a span that passages/edges cite is RESTRICTed.
    let mut conn = fx.pg.raw_connection().await;
    let err = sqlx::query("DELETE FROM knowledge_spans WHERE span_id = $1")
        .bind(&fx.span_id)
        .execute(&mut conn)
        .await
        .expect_err("deleting a cited span must violate ON DELETE RESTRICT");
    assert!(
        err.to_string().contains("violates foreign key constraint"),
        "unexpected: {err}"
    );
}

/// MT-140: SemanticCatalog routing contracts are backend-queryable (not
/// prompt-only).
#[tokio::test]
async fn semantic_catalog_entry_is_backend_queryable() {
    let fx = skip_if_no_pg!(MemoryFixture::setup().await, "semantic_catalog_queryable");
    let pool = pool_for(&fx.pg).await;

    let entry = upsert_semantic_catalog_entry(
        &pool,
        NewSemanticCatalogEntry {
            workspace_id: fx.workspace_id.clone(),
            entry_kind: SemanticCatalogKind::Index,
            name: "code_symbols".to_string(),
            version: 1,
            description: "symbol index".to_string(),
            query_routes: vec![
                "knowledge_graph".to_string(),
                "shadow_ws_lexical".to_string(),
            ],
            supported_selectors: vec!["symbol".to_string()],
            default_budgets: None,
            examples: serde_json::json!([]),
        },
    )
    .await
    .expect("upsert catalog");
    assert_eq!(entry.query_routes.len(), 2);

    let resolved = handshake_core::knowledge_retrieval::semantic_catalog::routing_for(
        &pool,
        &fx.workspace_id,
        "code_symbols",
        16,
    )
    .await
    .expect("resolve")
    .expect("entry present");
    assert_eq!(resolved.entry_name, "code_symbols");
    assert_eq!(resolved.route.len(), 2);
}

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
    KnowledgeBundleItemRefKind, KnowledgeEdgeType, KnowledgeStore, NewKnowledgeEdge,
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

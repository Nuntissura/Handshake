//! WP-KERNEL-009 MemoryGraphAndClaims MT-128 (MemoryGraphFixtures) integration
//! tests against REAL Handshake-managed PostgreSQL.
//!
//! Proves each of the six fixture scenarios the contract enumerates produces the
//! intended authority state: contradictions (both claims conflicted + a recorded
//! conflict), stale facts (claim retired stale), fragmented subgraphs (two
//! entities, no edge), false bridge edges (a relates_to edge created then
//! retired), unsupported claims (a fact labelled unsupported, excluded from the
//! trusted fact graph), and a successful promotion (an ontology term promoted to
//! stable with a receipt). Also exercises the conflict-resolution helper.

mod knowledge_memory_fixtures;

use handshake_core::knowledge_memory::fixtures::{
    contradiction, false_bridge_edge, fragmented_subgraph, resolve_contradiction, stale_fact,
    successful_promotion, unsupported_claim, FixtureContext,
};
use handshake_core::knowledge_memory::projection::build_fact_graph;
use handshake_core::storage::knowledge::{
    KnowledgeClaimState, KnowledgeEdgeLifecycle, KnowledgeStore,
};
use handshake_core::storage::knowledge_memory::MemoryOntologyLifecycle;
use handshake_core::storage::postgres::PostgresDatabase;
use knowledge_memory_fixtures::{pool_for, MemoryFixture};

fn ctx(fx: &MemoryFixture, seed: &str) -> FixtureContext {
    FixtureContext {
        workspace_id: fx.workspace_id.clone(),
        evidence_span_id: fx.span_id.clone(),
        seed: seed.to_string(),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn contradiction_and_resolution_fixtures() {
    let Some(fx) = MemoryFixture::setup().await else {
        eprintln!("SKIP contradiction_and_resolution_fixtures: no PostgreSQL");
        return;
    };
    let pool = pool_for(&fx.pg).await;
    let db = PostgresDatabase::new(pool.clone());
    let subject = fx.entity("api", "pg", "ManagedPostgres").await;
    let c = ctx(&fx, "contradiction");

    let fixture = contradiction(&db, &pool, &c, &subject)
        .await
        .expect("contradiction fixture");
    assert_eq!(fixture.conflict_ids.len(), 1, "one recorded conflict");
    // Both backing claims are conflicted.
    for fact in [&fixture.fact_a, &fixture.fact_b] {
        let state = db
            .get_knowledge_claim(&fact.claim_id)
            .await
            .expect("get")
            .expect("claim")
            .lifecycle_state;
        assert_eq!(state, KnowledgeClaimState::Conflicted);
    }

    // Resolve: discard fact_b's claim, keep fact_a's. Conflict resolved.
    let job_id = resolve_contradiction(
        &db,
        &pool,
        &c,
        &fixture.conflict_ids[0],
        &fixture.fact_a.claim_id,
        &fixture.fact_b.claim_id,
    )
    .await
    .expect("resolve contradiction");
    assert!(job_id.starts_with("KCRJ-"));
    let conflicts = db
        .list_knowledge_claim_conflicts(&fixture.fact_a.claim_id)
        .await
        .expect("conflicts");
    assert!(
        conflicts[0].resolved_at.is_some(),
        "the conflict is now resolved"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stale_fact_fixture_retires_claim_as_stale() {
    let Some(fx) = MemoryFixture::setup().await else {
        eprintln!("SKIP stale_fact_fixture_retires_claim_as_stale: no PostgreSQL");
        return;
    };
    let pool = pool_for(&fx.pg).await;
    let db = PostgresDatabase::new(pool.clone());
    let subject = fx.entity("api", "pg", "ManagedPostgres").await;

    let (_fact, claim_id) = stale_fact(&db, &pool, &ctx(&fx, "stale"), &subject)
        .await
        .expect("stale fixture");
    let claim = db
        .get_knowledge_claim(&claim_id)
        .await
        .expect("get")
        .expect("claim");
    assert_eq!(claim.lifecycle_state, KnowledgeClaimState::Retired);
    assert_eq!(
        claim.retirement_reason,
        Some(handshake_core::storage::knowledge::KnowledgeClaimRetirementReason::Stale)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fragmented_subgraph_and_false_bridge_fixtures() {
    let Some(fx) = MemoryFixture::setup().await else {
        eprintln!("SKIP fragmented_subgraph_and_false_bridge_fixtures: no PostgreSQL");
        return;
    };
    let pool = pool_for(&fx.pg).await;
    let db = PostgresDatabase::new(pool.clone());
    let a = fx.entity("symbol", "crate::a::A", "A").await;
    let b = fx.entity("symbol", "crate::b::B", "B").await;
    let c = ctx(&fx, "frag");

    // Fragmented: two facts, two entities, no edge between them.
    let (fact_a, fact_b) = fragmented_subgraph(&db, &pool, &c, &a, &b)
        .await
        .expect("fragmented fixture");
    assert_ne!(fact_a.subject_entity_id, fact_b.subject_entity_id);
    assert!(
        db.list_knowledge_edges_for_entity(&a)
            .await
            .expect("edges a")
            .is_empty(),
        "fragmented entities have no edges yet"
    );

    // False bridge: a relates_to edge created then retired (rejected).
    let edge_id = false_bridge_edge(&db, &pool, &c, &a, &b)
        .await
        .expect("false bridge fixture");
    let edge = db
        .get_knowledge_edge(&edge_id)
        .await
        .expect("get edge")
        .expect("edge");
    assert_eq!(
        edge.lifecycle_state,
        KnowledgeEdgeLifecycle::Retired,
        "a false bridge ends retired (rejected)"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unsupported_claim_and_promotion_fixtures() {
    let Some(fx) = MemoryFixture::setup().await else {
        eprintln!("SKIP unsupported_claim_and_promotion_fixtures: no PostgreSQL");
        return;
    };
    let pool = pool_for(&fx.pg).await;
    let db = PostgresDatabase::new(pool.clone());
    let subject = fx.entity("symbol", "crate::x::X", "X").await;
    let c = ctx(&fx, "unsupported");

    // Unsupported fact: present in the full graph, excluded from trusted graph.
    let fact = unsupported_claim(&db, &pool, &c, &subject)
        .await
        .expect("unsupported fixture");
    let all = build_fact_graph(&db, &pool, &fx.workspace_id, false, 50)
        .await
        .expect("all graph");
    assert!(all.edges.iter().any(|e| e.fact_id == fact.fact_id));
    let trusted = build_fact_graph(&db, &pool, &fx.workspace_id, true, 50)
        .await
        .expect("trusted graph");
    assert!(
        !trusted.edges.iter().any(|e| e.fact_id == fact.fact_id),
        "an unsupported fact must be excluded from the trusted fact graph"
    );

    // Successful promotion: operator-approved term -> stable with receipt.
    let term = successful_promotion(&db, &pool, &c)
        .await
        .expect("promotion fixture");
    assert_eq!(term.lifecycle_state, MemoryOntologyLifecycle::Stable);
    assert!(term.promotion_receipt_event_id.is_some());
}

//! WP-KERNEL-009 MemoryGraphAndClaims MT-124 (BridgeEdgeGenerator) integration
//! tests against REAL Handshake-managed PostgreSQL.
//!
//! Proof: two entities that co-occur in a shared span but sit in different edge
//! components get a PROPOSED `relates_to` bridge edge backed by that span; a
//! pair that is already connected is suppressed (`suppressed_connected`); a pair
//! whose endpoint is a hub (degree >= threshold) is suppressed
//! (`suppressed_hub`). Every decision is logged. Bridges are born `proposed`
//! (suggestions), never auto-accepted.

mod knowledge_memory_fixtures;

use handshake_core::knowledge_memory::bridge::{generate_bridge_edges, list_bridge_decisions};
use handshake_core::storage::knowledge::{
    KnowledgeEdgeLifecycle, KnowledgeEdgeType, KnowledgeStore, NewKnowledgeEdge,
};
use handshake_core::storage::knowledge_memory::BridgeDecision;
use handshake_core::storage::postgres::PostgresDatabase;
use knowledge_memory_fixtures::{pool_for, MemoryFixture};

/// All fixture entities are detected from the single fixture span, so any two
/// of them co-occur. That is exactly the bridge-candidate condition.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bridges_disconnected_cooccurring_entities_as_proposed() {
    let Some(fx) = MemoryFixture::setup().await else {
        eprintln!("SKIP bridges_disconnected_cooccurring_entities_as_proposed: no PostgreSQL");
        return;
    };
    let pool = pool_for(&fx.pg).await;
    let db = PostgresDatabase::new(pool.clone());

    // Two entities, co-occurring (shared fixture span), no edge between them.
    let _a = fx.entity("symbol", "crate::mod_a::Alpha", "Alpha").await;
    let _b = fx.entity("symbol", "crate::mod_b::Beta", "Beta").await;

    let result = generate_bridge_edges(&db, &pool, &fx.workspace_id, 5, 0.4, "bridge_v1", 50)
        .await
        .expect("bridge pass");

    assert_eq!(result.bridged_edge_ids.len(), 1, "one bridge created");
    let edge = db
        .get_knowledge_edge(&result.bridged_edge_ids[0])
        .await
        .expect("get edge")
        .expect("edge exists");
    assert_eq!(edge.edge_type, KnowledgeEdgeType::RelatesTo);
    assert_eq!(
        edge.lifecycle_state,
        KnowledgeEdgeLifecycle::Proposed,
        "a bridge is a proposed suggestion, not auto-accepted"
    );
    // The bridge carries the shared span as evidence.
    let span_ids = db
        .list_knowledge_edge_span_ids(&edge.edge_id)
        .await
        .expect("edge spans");
    assert_eq!(span_ids, vec![fx.span_id.clone()]);

    // The decision log records the bridge.
    let decisions = list_bridge_decisions(&pool, &fx.workspace_id, 50)
        .await
        .expect("decisions");
    assert_eq!(decisions.len(), 1);
    assert_eq!(decisions[0].decision, BridgeDecision::Bridged);
    assert_eq!(
        decisions[0].bridge_edge_id.as_deref(),
        Some(edge.edge_id.as_str())
    );

    // Idempotent-ish: a second pass now sees the pair as connected (the bridge
    // we just made unions them), so it suppresses rather than double-bridging.
    let again = generate_bridge_edges(&db, &pool, &fx.workspace_id, 5, 0.4, "bridge_v1", 50)
        .await
        .expect("second bridge pass");
    assert_eq!(again.bridged_edge_ids.len(), 0, "no second bridge");
    assert_eq!(again.decisions.len(), 1);
    assert_eq!(
        again.decisions[0].decision,
        BridgeDecision::SuppressedConnected
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn hub_endpoint_suppresses_the_bridge() {
    let Some(fx) = MemoryFixture::setup().await else {
        eprintln!("SKIP hub_endpoint_suppresses_the_bridge: no PostgreSQL");
        return;
    };
    let pool = pool_for(&fx.pg).await;
    let db = PostgresDatabase::new(pool.clone());

    // `hub` will be given a high degree by connecting it to several leaves.
    let hub = fx.entity("concept", "crate::core::Hub", "Hub").await;
    let other = fx.entity("symbol", "crate::edge::Leaf", "Leaf").await;
    // Build degree on `hub` by linking it to 3 helper entities (these helper
    // entities also co-occur on the fixture span, but the hub pair is the focus).
    for n in 0..3 {
        let leaf = fx
            .entity("symbol", &format!("crate::h::L{n}"), &format!("L{n}"))
            .await;
        db.upsert_knowledge_edge(NewKnowledgeEdge {
            workspace_id: fx.workspace_id.clone(),
            edge_type: KnowledgeEdgeType::References,
            source_entity_id: hub.clone(),
            target_entity_id: leaf,
            extractor_version: "edge_v1".to_string(),
            confidence: 0.9,
            detected_in_run: None,
            evidence_span_ids: vec![fx.span_id.clone()],
        })
        .await
        .expect("hub edge");
    }

    // Threshold 3: hub degree is 3 (>= threshold) so the hub pair is suppressed.
    let result = generate_bridge_edges(&db, &pool, &fx.workspace_id, 3, 0.4, "bridge_v1", 100)
        .await
        .expect("bridge pass");

    // The (hub, other) pair must be a suppressed_hub decision, never bridged.
    let hub_other = result.decisions.iter().find(|d| {
        (d.entity_id_a == hub && d.entity_id_b == other)
            || (d.entity_id_a == other && d.entity_id_b == hub)
    });
    let hub_other = hub_other.expect("hub/other pair was evaluated");
    assert_eq!(
        hub_other.decision,
        BridgeDecision::SuppressedHub,
        "a bridge through a hub endpoint must be suppressed"
    );
    assert!(hub_other.bridge_edge_id.is_none());

    // No bridge edge touches the hub (its only edges are the 3 reference edges).
    let hub_edges = db
        .list_knowledge_edges_for_entity(&hub)
        .await
        .expect("hub edges");
    assert!(
        hub_edges
            .iter()
            .all(|e| e.edge_type == KnowledgeEdgeType::References),
        "no relates_to bridge should touch the hub"
    );
}

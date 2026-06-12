//! WP-KERNEL-009 MT-179 LocalGraphApi + MT-180 GlobalGraphApi — REAL PostgreSQL
//! authority proof.
//!
//! §10.12 §9.4 / [LM-GRAPH-001] (local neighborhood: undirected BFS, filters,
//! depth, stale markers, ProjectKnowledgeIndex citations) and [LM-GRAPH-002] /
//! §7.1.4.3 (global project graph: performance limits + hub suppression), over
//! loom_edges + loom_blocks (+ the MT-177 bridge for citations). No parallel
//! store: all reads resolve to the isolated migrated schema.

mod knowledge_pg_support;

use handshake_core::storage::{
    Database, LoomBlockContentType, LoomBlockDerived, LoomEdgeCreatedBy, LoomEdgeType, NewLoomBlock,
    NewLoomEdge, WriteContext,
};
use knowledge_pg_support::knowledge_pg;

macro_rules! pg_or_skip {
    () => {{
        match knowledge_pg().await {
            Some(pg) => pg,
            None => {
                eprintln!("SKIP MT-179/180 loom graph proof: PostgreSQL unavailable");
                return;
            }
        }
    }};
}

async fn block(
    db: &handshake_core::storage::postgres::PostgresDatabase,
    ws: &str,
    title: &str,
    content_type: LoomBlockContentType,
) -> String {
    let ctx = WriteContext::human(None);
    db.create_loom_block(
        &ctx,
        NewLoomBlock {
            block_id: None,
            workspace_id: ws.to_string(),
            content_type,
            document_id: None,
            asset_id: None,
            title: Some(title.to_string()),
            original_filename: None,
            content_hash: None,
            pinned: false,
            journal_date: None,
            imported_at: None,
            derived: LoomBlockDerived::default(),
        },
    )
    .await
    .expect("block")
    .block_id
}

async fn bridge(db: &handshake_core::storage::postgres::PostgresDatabase, ws: &str, block_id: &str) {
    let ctx = WriteContext::human(None);
    db.bridge_loom_block_to_knowledge(&ctx, ws, block_id)
        .await
        .expect("bridge");
}

async fn edge(
    db: &handshake_core::storage::postgres::PostgresDatabase,
    ws: &str,
    src: &str,
    tgt: &str,
    edge_type: LoomEdgeType,
    created_by: LoomEdgeCreatedBy,
) {
    let ctx = WriteContext::human(None);
    db.create_loom_edge(
        &ctx,
        NewLoomEdge {
            edge_id: None,
            workspace_id: ws.to_string(),
            source_block_id: src.to_string(),
            target_block_id: tgt.to_string(),
            edge_type,
            created_by,
            crdt_site_id: None,
            source_anchor: None,
        },
    )
    .await
    .expect("edge");
}

#[tokio::test]
async fn local_graph_is_undirected_with_stale_markers_and_citations() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;

    // A --mention--> B --tag--> C(tag_hub) ;  A --ai_suggested--> D
    let a = block(&pg.db, &ws, "Alpha", LoomBlockContentType::Note).await;
    let b = block(&pg.db, &ws, "Beta", LoomBlockContentType::Note).await;
    let c = block(&pg.db, &ws, "Gamma", LoomBlockContentType::TagHub).await;
    let d = block(&pg.db, &ws, "Delta", LoomBlockContentType::Note).await;

    // Bridge A and B (indexed); leave C and D un-bridged so they are STALE.
    bridge(&pg.db, &ws, &a).await;
    bridge(&pg.db, &ws, &b).await;

    edge(&pg.db, &ws, &a, &b, LoomEdgeType::Mention, LoomEdgeCreatedBy::User).await;
    edge(&pg.db, &ws, &b, &c, LoomEdgeType::Tag, LoomEdgeCreatedBy::User).await;
    edge(&pg.db, &ws, &a, &d, LoomEdgeType::AiSuggested, LoomEdgeCreatedBy::Ai).await;

    // Local graph around B (depth 2). Undirected: should reach A (incoming) and
    // C (outgoing). At depth 2, D is reachable B->A->D.
    let graph = pg
        .db
        .local_graph(&ws, &b, 2, &[], 200)
        .await
        .expect("local graph");

    let node_ids: Vec<&str> = graph.nodes.iter().map(|n| n.block.block_id.as_str()).collect();
    assert!(node_ids.contains(&b.as_str()), "start node present");
    assert!(node_ids.contains(&a.as_str()), "incoming neighbor A present (undirected)");
    assert!(node_ids.contains(&c.as_str()), "outgoing neighbor C present");

    // Start node is depth 0.
    let start_node = graph.nodes.iter().find(|n| n.block.block_id == b).unwrap();
    assert_eq!(start_node.depth, 0);
    assert!(!start_node.stale, "bridged node B is not stale");
    assert!(start_node.entity_id.is_some(), "B carries a ProjectKnowledgeIndex citation");

    // C is un-bridged -> STALE, no citation.
    let c_node = graph.nodes.iter().find(|n| n.block.block_id == c).unwrap();
    assert!(c_node.stale, "un-bridged node C is stale");
    assert!(c_node.entity_id.is_none());

    // The ai_suggested edge (A->D) is marked stale when present in the graph.
    if let Some(ai_edge) = graph
        .edges
        .iter()
        .find(|e| e.edge.edge_type == LoomEdgeType::AiSuggested)
    {
        assert!(ai_edge.stale, "ai_suggested edge is stale");
    }

    // Edge-type filter: restricting to mention should drop the tag edge B->C,
    // so C is no longer reachable from B.
    let mention_only = pg
        .db
        .local_graph(&ws, &b, 2, &[LoomEdgeType::Mention], 200)
        .await
        .expect("mention-only local graph");
    let m_ids: Vec<&str> = mention_only
        .nodes
        .iter()
        .map(|n| n.block.block_id.as_str())
        .collect();
    assert!(m_ids.contains(&a.as_str()), "A still reachable via mention");
    assert!(!m_ids.contains(&c.as_str()), "C not reachable when only mention edges are followed");
}

#[tokio::test]
async fn local_graph_depth_one_is_immediate_neighborhood() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;

    let a = block(&pg.db, &ws, "A1", LoomBlockContentType::Note).await;
    let b = block(&pg.db, &ws, "B1", LoomBlockContentType::Note).await;
    let c = block(&pg.db, &ws, "C1", LoomBlockContentType::Note).await;
    edge(&pg.db, &ws, &a, &b, LoomEdgeType::Mention, LoomEdgeCreatedBy::User).await;
    edge(&pg.db, &ws, &b, &c, LoomEdgeType::Mention, LoomEdgeCreatedBy::User).await;

    // depth 1 around B reaches A and C but the induced edges only include those
    // among {A,B,C}.
    let graph = pg.db.local_graph(&ws, &b, 1, &[], 200).await.expect("graph");
    let ids: Vec<&str> = graph.nodes.iter().map(|n| n.block.block_id.as_str()).collect();
    assert!(ids.contains(&a.as_str()) && ids.contains(&b.as_str()) && ids.contains(&c.as_str()));
    // Both edges are within the node set -> 2 induced edges.
    assert_eq!(graph.edges.len(), 2);
}

#[tokio::test]
async fn global_graph_returns_all_and_suppresses_hubs() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;

    // A hub H connected to many leaves; plus a small unrelated pair X-Y.
    let hub = block(&pg.db, &ws, "Hub", LoomBlockContentType::Note).await;
    let mut leaves = Vec::new();
    for i in 0..6 {
        let leaf = block(&pg.db, &ws, &format!("Leaf{i}"), LoomBlockContentType::Note).await;
        edge(&pg.db, &ws, &hub, &leaf, LoomEdgeType::Mention, LoomEdgeCreatedBy::User).await;
        leaves.push(leaf);
    }
    let x = block(&pg.db, &ws, "X", LoomBlockContentType::Note).await;
    let y = block(&pg.db, &ws, "Y", LoomBlockContentType::Note).await;
    edge(&pg.db, &ws, &x, &y, LoomEdgeType::Mention, LoomEdgeCreatedBy::User).await;

    // No hub suppression (threshold 0 disables it): all blocks present.
    let full = pg.db.global_graph(&ws, &[], 500, 0).await.expect("full global");
    let full_ids: Vec<&str> = full.nodes.iter().map(|n| n.block.block_id.as_str()).collect();
    assert!(full_ids.contains(&hub.as_str()), "hub present when suppression disabled");
    assert!(full.suppressed_hub_ids.is_empty());
    // Hub degree is 6 (6 mention edges).
    let hub_node = full.nodes.iter().find(|n| n.block.block_id == hub).unwrap();
    assert_eq!(hub_node.degree, 6, "hub degree counted");

    // Suppress hubs with degree > 5: the hub (degree 6) is dropped + listed.
    let suppressed = pg.db.global_graph(&ws, &[], 500, 5).await.expect("suppressed global");
    let s_ids: Vec<&str> = suppressed.nodes.iter().map(|n| n.block.block_id.as_str()).collect();
    assert!(!s_ids.contains(&hub.as_str()), "hub suppressed");
    assert!(
        suppressed.suppressed_hub_ids.contains(&hub),
        "hub id reported in suppressed_hub_ids"
    );
    // The unrelated X-Y pair is unaffected.
    assert!(s_ids.contains(&x.as_str()) && s_ids.contains(&y.as_str()));
    // Edges touching the suppressed hub are not in the induced subgraph.
    assert!(
        !suppressed
            .edges
            .iter()
            .any(|e| e.edge.source_block_id == hub || e.edge.target_block_id == hub),
        "no edges to a suppressed hub"
    );
}

#[tokio::test]
async fn global_graph_node_limit_sets_truncated() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    for i in 0..5 {
        block(&pg.db, &ws, &format!("N{i}"), LoomBlockContentType::Note).await;
    }
    // node_limit 3 over 5 blocks -> truncated, exactly 3 nodes.
    let graph = pg.db.global_graph(&ws, &[], 3, 0).await.expect("limited global");
    assert_eq!(graph.nodes.len(), 3);
    assert!(graph.truncated, "result is marked truncated");
}

#[tokio::test]
async fn local_graph_fails_closed_on_missing_start() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let err = pg
        .db
        .local_graph(&ws, "loom-missing-start", 2, &[], 200)
        .await
        .expect_err("missing start fails closed");
    let msg = format!("{err}");
    assert!(msg.contains("not found") || msg.contains("not_found"), "{msg}");
}

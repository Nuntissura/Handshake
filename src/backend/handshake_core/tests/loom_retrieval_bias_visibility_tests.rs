//! WP-KERNEL-009 MT-189 LoomRetrievalBiasVisibility -- real PostgreSQL proof.
//!
//! Graph-search hits must expose why Loom-native graph signals changed
//! retrieval order, so no-context models can see tag, backlink, and pin
//! influence instead of guessing from opaque ranking.

mod knowledge_pg_support;

use handshake_core::storage::{
    Database, LoomBlockContentType, LoomBlockDerived, LoomEdgeCreatedBy, LoomEdgeType,
    LoomSearchFilters, LoomSearchSourceKind, NewLoomBlock, NewLoomEdge, WriteContext,
};
use knowledge_pg_support::knowledge_pg;
use serde_json::Value;

macro_rules! pg_or_fail {
    () => {{
        match knowledge_pg().await {
            Some(pg) => pg,
            None => {
                panic!("MT-189 loom retrieval-bias proof requires real PostgreSQL");
            }
        }
    }};
}

async fn insert_loom_block(
    db: &handshake_core::storage::postgres::PostgresDatabase,
    ctx: &WriteContext,
    ws: &str,
    content_type: LoomBlockContentType,
    title: &str,
    full_text: Option<&str>,
    pinned: bool,
) -> String {
    db.create_loom_block(
        ctx,
        NewLoomBlock {
            block_id: None,
            workspace_id: ws.to_string(),
            content_type,
            document_id: None,
            asset_id: None,
            title: Some(title.to_string()),
            original_filename: None,
            content_hash: None,
            pinned,
            journal_date: None,
            imported_at: None,
            derived: LoomBlockDerived {
                full_text_index: full_text.map(str::to_string),
                ..Default::default()
            },
        },
    )
    .await
    .expect("insert Loom block")
    .block_id
}

async fn create_edge(
    db: &handshake_core::storage::postgres::PostgresDatabase,
    ctx: &WriteContext,
    ws: &str,
    source_block_id: &str,
    target_block_id: &str,
    edge_type: LoomEdgeType,
) {
    db.create_loom_edge(
        ctx,
        NewLoomEdge {
            edge_id: None,
            workspace_id: ws.to_string(),
            source_block_id: source_block_id.to_string(),
            target_block_id: target_block_id.to_string(),
            edge_type,
            created_by: LoomEdgeCreatedBy::User,
            crdt_site_id: None,
            source_anchor: None,
        },
    )
    .await
    .expect("create Loom edge");
}

fn reason_codes(metadata: &Value) -> Vec<String> {
    metadata
        .get("retrieval_bias_reasons")
        .and_then(Value::as_array)
        .expect("graph-search block hit must expose retrieval_bias_reasons")
        .iter()
        .map(|reason| {
            reason
                .get("code")
                .and_then(Value::as_str)
                .expect("retrieval bias reason must carry a code")
                .to_string()
        })
        .collect()
}

#[tokio::test]
async fn mt189_graph_search_exposes_loom_retrieval_bias_reasons() {
    let pg = pg_or_fail!();
    let ws = pg.create_workspace().await;
    let ctx = WriteContext::human(None);

    let biased = insert_loom_block(
        &pg.db,
        &ctx,
        &ws,
        LoomBlockContentType::Note,
        "BiasBeacon older pinned tagged note",
        Some("BiasBeacon appears in the older block with graph support."),
        true,
    )
    .await;
    let plain = insert_loom_block(
        &pg.db,
        &ctx,
        &ws,
        LoomBlockContentType::Note,
        "BiasBeacon newer plain note",
        Some("BiasBeacon appears in the newer block without graph support."),
        false,
    )
    .await;
    let tag = insert_loom_block(
        &pg.db,
        &ctx,
        &ws,
        LoomBlockContentType::TagHub,
        "Bias visibility tag hub",
        None,
        false,
    )
    .await;
    let backlink_source = insert_loom_block(
        &pg.db,
        &ctx,
        &ws,
        LoomBlockContentType::Note,
        "Backlink source",
        Some("A note that links to the biased candidate."),
        false,
    )
    .await;

    create_edge(&pg.db, &ctx, &ws, &biased, &tag, LoomEdgeType::Tag).await;
    create_edge(
        &pg.db,
        &ctx,
        &ws,
        &backlink_source,
        &biased,
        LoomEdgeType::Mention,
    )
    .await;

    let hits = pg
        .db
        .search_loom_graph(
            &ws,
            "BiasBeacon",
            LoomSearchFilters {
                source_kinds: vec![LoomSearchSourceKind::LoomBlock],
                ..Default::default()
            },
            20,
            0,
        )
        .await
        .expect("graph search");
    let hit_ids: Vec<_> = hits.iter().map(|hit| hit.ref_id.as_str()).collect();
    assert!(
        hit_ids.len() >= 2,
        "fixture must produce both plain and biased hits: {hit_ids:?}"
    );
    assert_eq!(
        hit_ids[0], biased,
        "graph-biased hit must outrank the newer plain hit: {hit_ids:?}"
    );
    assert!(
        hits[0].score > hits[1].score,
        "bias score must be exposed on graph-search hits: {:?}",
        hits.iter().map(|hit| hit.score).collect::<Vec<_>>()
    );
    assert_eq!(
        hits[0].metadata["retrieval_bias_schema_id"],
        "hsk.loom_retrieval_bias@1"
    );
    assert!(
        hits[0]
            .metadata
            .get("retrieval_bias_score")
            .and_then(Value::as_f64)
            .unwrap_or_default()
            > 0.0,
        "biased hit metadata must include a positive retrieval_bias_score"
    );

    let biased_codes = reason_codes(&hits[0].metadata);
    for expected_code in ["pinned", "tagged", "backlinked"] {
        assert!(
            biased_codes.iter().any(|code| code == expected_code),
            "biased hit must include {expected_code} reason, got {biased_codes:?}"
        );
    }

    let plain_hit = hits
        .iter()
        .find(|hit| hit.ref_id == plain)
        .expect("plain hit should still be returned");
    assert!(
        reason_codes(&plain_hit.metadata).is_empty(),
        "plain hit should expose an empty reason list, not hidden or stale bias"
    );

    let snapshot = pg
        .db
        .loom_visual_debug_snapshot(&ws, &biased, "BiasBeacon", 20)
        .await
        .expect("visual-debug snapshot");
    let debug_hit = snapshot
        .search
        .results
        .iter()
        .find(|hit| hit.ref_id == biased)
        .expect("visual-debug search summary should include biased hit");
    assert_eq!(
        debug_hit.retrieval_bias_schema_id.as_deref(),
        Some("hsk.loom_retrieval_bias@1")
    );
    assert!(
        debug_hit.retrieval_bias_score.unwrap_or_default() > 0.0,
        "visual-debug search summary must expose retrieval bias score"
    );
    let debug_codes: Vec<_> = debug_hit
        .retrieval_bias_reasons
        .iter()
        .filter_map(|reason| reason.get("code").and_then(Value::as_str))
        .collect();
    for expected_code in ["pinned", "tagged", "backlinked"] {
        assert!(
            debug_codes.iter().any(|code| *code == expected_code),
            "visual-debug search summary must include {expected_code} reason, got {debug_codes:?}"
        );
    }
}

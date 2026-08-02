//! WP-KERNEL-009 MT-182 TagsAndTagHubs — REAL PostgreSQL authority proof.
//!
//! §10.12 [LM-TAG-001..005] / §7.1.4.3: tags are first-class LoomBlocks
//! (content_type=tag_hub) with their own content, sub-tags (SUB_TAG nested-tag
//! hierarchy: child SOURCE -> parent TARGET), and search filtering (blocks
//! tagged with a tag, optionally including descendant sub-tags). Authority =
//! loom_blocks + loom_edges. No parallel store.

mod knowledge_pg_support;

use handshake_core::storage::{
    Database, LoomBlockContentType, LoomBlockDerived, LoomEdgeCreatedBy, LoomEdgeType,
    NewLoomBlock, NewLoomEdge, WriteContext,
};
use knowledge_pg_support::knowledge_pg;

macro_rules! pg_or_skip {
    () => {{
        match knowledge_pg().await {
            Some(pg) => pg,
            None => {
                eprintln!("SKIP MT-182 loom tag hub proof: PostgreSQL unavailable");
                return;
            }
        }
    }};
}

async fn blk(
    db: &handshake_core::storage::postgres::PostgresDatabase,
    ws: &str,
    title: &str,
    ct: LoomBlockContentType,
) -> String {
    let ctx = WriteContext::human(None);
    db.create_loom_block(
        &ctx,
        NewLoomBlock {
            block_id: None,
            workspace_id: ws.to_string(),
            content_type: ct,
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

async fn edge(
    db: &handshake_core::storage::postgres::PostgresDatabase,
    ws: &str,
    src: &str,
    tgt: &str,
    et: LoomEdgeType,
) {
    let ctx = WriteContext::human(None);
    db.create_loom_edge(
        &ctx,
        NewLoomEdge {
            edge_id: None,
            workspace_id: ws.to_string(),
            source_block_id: src.to_string(),
            target_block_id: tgt.to_string(),
            edge_type: et,
            created_by: LoomEdgeCreatedBy::User,
            crdt_site_id: None,
            source_anchor: None,
        },
    )
    .await
    .expect("edge");
}

/// Build: tag #project; sub-tag #alpha (SUB_TAG alpha->project);
/// note N1 TAG #project; note N2 TAG #alpha.
async fn fixture(
    db: &handshake_core::storage::postgres::PostgresDatabase,
    ws: &str,
) -> (String, String, String, String) {
    let project = blk(db, ws, "project", LoomBlockContentType::TagHub).await;
    let alpha = blk(db, ws, "alpha", LoomBlockContentType::TagHub).await;
    edge(db, ws, &alpha, &project, LoomEdgeType::SubTag).await; // alpha is child of project
    let n1 = blk(db, ws, "Note One", LoomBlockContentType::Note).await;
    let n2 = blk(db, ws, "Note Two", LoomBlockContentType::Note).await;
    edge(db, ws, &n1, &project, LoomEdgeType::Tag).await;
    edge(db, ws, &n2, &alpha, LoomEdgeType::Tag).await;
    (project, alpha, n1, n2)
}

#[tokio::test]
async fn list_tag_hubs_returns_only_tag_blocks() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let (project, alpha, _n1, _n2) = fixture(&pg.db, &ws).await;

    let tags = pg.db.list_tag_hubs(&ws, 100, 0).await.expect("list tags");
    let ids: Vec<&str> = tags.iter().map(|b| b.block_id.as_str()).collect();
    assert!(ids.contains(&project.as_str()) && ids.contains(&alpha.as_str()));
    // Only tag_hub blocks (the notes are excluded).
    assert!(tags
        .iter()
        .all(|b| b.content_type == LoomBlockContentType::TagHub));
}

#[tokio::test]
async fn get_tag_hub_exposes_subtags_tagged_blocks_and_backlinks() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let (project, alpha, n1, _n2) = fixture(&pg.db, &ws).await;

    let hub = pg.db.get_tag_hub(&ws, &project).await.expect("tag hub");
    assert_eq!(hub.block.block_id, project);

    // Exact separation: alpha is the only direct sub-tag; N1 is the only direct tag member.
    let sub_tag_ids: Vec<&str> = hub.sub_tags.iter().map(|b| b.block_id.as_str()).collect();
    let tagged_ids: Vec<&str> = hub
        .tagged_blocks
        .iter()
        .map(|b| b.block_id.as_str())
        .collect();
    assert_eq!(sub_tag_ids, vec![alpha.as_str()]);
    assert_eq!(tagged_ids, vec![n1.as_str()]);
    assert!(
        hub.sub_tags.iter().all(|block| block.block_id != n1),
        "tag members must never bleed into sub_tags"
    );
    assert!(
        hub.tagged_blocks
            .iter()
            .all(|block| block.block_id != alpha),
        "sub-tags must never bleed into tagged_blocks"
    );
    // backlink_count = SUB_TAG(alpha->project) + TAG(N1->project) = 2 incoming.
    assert_eq!(hub.backlink_count, 2);
}

#[tokio::test]
async fn get_tag_hub_deduplicates_members_without_hiding_duplicate_edge_backlinks() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let project = blk(&pg.db, &ws, "project", LoomBlockContentType::TagHub).await;
    let note = blk(
        &pg.db,
        &ws,
        "duplicate-edge-note",
        LoomBlockContentType::Note,
    )
    .await;

    // Semantic-edge uniqueness is not a schema invariant: two physical edge rows may name the same
    // source/target/type tuple. The tag-hub contract intentionally de-duplicates the complete member
    // row while backlink_count continues to report every incoming physical edge.
    edge(&pg.db, &ws, &note, &project, LoomEdgeType::Tag).await;
    edge(&pg.db, &ws, &note, &project, LoomEdgeType::Tag).await;

    let hub = pg.db.get_tag_hub(&ws, &project).await.expect("tag hub");
    assert_eq!(
        hub.tagged_blocks
            .iter()
            .filter(|block| block.block_id == note)
            .count(),
        1,
        "duplicate semantic tag edges must not duplicate the returned LoomBlock"
    );
    assert_eq!(
        hub.backlink_count, 2,
        "backlink_count must retain both physical incoming edges"
    );
}

#[tokio::test]
async fn get_tag_hub_is_workspace_isolated_and_deterministically_ordered() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let other_ws = pg.create_workspace().await;
    let hub = blk(&pg.db, &ws, "ordered-hub", LoomBlockContentType::TagHub).await;
    let other_hub = blk(
        &pg.db,
        &other_ws,
        "other-workspace-hub",
        LoomBlockContentType::TagHub,
    )
    .await;
    let mut members = Vec::new();
    for title in ["member-c", "member-a", "member-b"] {
        let member = blk(&pg.db, &ws, title, LoomBlockContentType::Note).await;
        edge(&pg.db, &ws, &member, &hub, LoomEdgeType::Tag).await;
        members.push(member);
    }
    let other_member = blk(
        &pg.db,
        &other_ws,
        "other-workspace-member",
        LoomBlockContentType::Note,
    )
    .await;
    edge(
        &pg.db,
        &other_ws,
        &other_member,
        &other_hub,
        LoomEdgeType::Tag,
    )
    .await;

    // Force the primary ordering key equal so the required block_id ASC total tie-breaker is proven.
    let mut connection = pg.raw_connection().await;
    sqlx::query(
        "UPDATE loom_blocks SET updated_at = TIMESTAMPTZ '2026-01-01 00:00:00+00' \
         WHERE workspace_id = $1 AND block_id = ANY($2)",
    )
    .bind(&ws)
    .bind(&members)
    .execute(&mut connection)
    .await
    .expect("set equal updated_at values");
    drop(connection);

    let first = pg.db.get_tag_hub(&ws, &hub).await.expect("first tag hub");
    let second = pg.db.get_tag_hub(&ws, &hub).await.expect("repeat tag hub");
    let first_ids: Vec<String> = first
        .tagged_blocks
        .iter()
        .map(|block| block.block_id.clone())
        .collect();
    let second_ids: Vec<String> = second
        .tagged_blocks
        .iter()
        .map(|block| block.block_id.clone())
        .collect();
    let mut expected = members;
    expected.sort();
    assert_eq!(first_ids, expected, "equal timestamps use block_id ASC");
    assert_eq!(second_ids, first_ids, "repeat reads preserve total order");
    assert!(
        first
            .tagged_blocks
            .iter()
            .all(|block| block.workspace_id == ws && block.block_id != other_member),
        "tag-hub membership must not cross workspace authority"
    );
}

#[tokio::test]
async fn list_blocks_for_tag_resolves_nested_membership() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let (project, _alpha, n1, n2) = fixture(&pg.db, &ws).await;

    // Direct only: #project has just N1.
    let direct = pg
        .db
        .list_blocks_for_tag(&ws, &project, false, 100, 0)
        .await
        .expect("direct blocks");
    let direct_ids: Vec<&str> = direct.iter().map(|b| b.block_id.as_str()).collect();
    assert!(direct_ids.contains(&n1.as_str()));
    assert!(
        !direct_ids.contains(&n2.as_str()),
        "N2 (tagged alpha) is not a DIRECT project block"
    );

    // Nested: #project includes descendants (#alpha) -> N1 + N2.
    let nested = pg
        .db
        .list_blocks_for_tag(&ws, &project, true, 100, 0)
        .await
        .expect("nested blocks");
    let nested_ids: Vec<&str> = nested.iter().map(|b| b.block_id.as_str()).collect();
    assert!(
        nested_ids.contains(&n1.as_str()) && nested_ids.contains(&n2.as_str()),
        "nested membership pulls N2 via the alpha sub-tag"
    );
}

#[tokio::test]
async fn tag_hub_apis_fail_closed_on_non_tag_block() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let note = blk(&pg.db, &ws, "Just A Note", LoomBlockContentType::Note).await;

    let err = pg
        .db
        .get_tag_hub(&ws, &note)
        .await
        .expect_err("not a tag_hub");
    assert!(format!("{err}").contains("not a tag_hub"), "{err}");

    let err2 = pg
        .db
        .list_blocks_for_tag(&ws, &note, true, 100, 0)
        .await
        .expect_err("not a tag_hub");
    assert!(format!("{err2}").contains("not a tag_hub"), "{err2}");
}

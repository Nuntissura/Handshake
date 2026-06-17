//! WP-KERNEL-009 MT-191 LoomVisualDebugViews -- real PostgreSQL proof.
//!
//! The Loom visual-debug payload is a bounded backend projection over the
//! existing Loom authority tables. It exposes graph/backlink/folder/search
//! navigation state without becoming a parallel store or a full-content export.

mod knowledge_pg_support;

use handshake_core::storage::{
    Database, LoomBlockContentType, LoomBlockDerived, LoomEdgeCreatedBy, LoomEdgeType,
    LoomFolderSortMode, NewLoomBlock, NewLoomEdge, NewLoomFolder, WriteContext,
    LOOM_VISUAL_DEBUG_SCHEMA_ID,
};
use handshake_core::user_manual::registry::wp009_surface_registry;
use knowledge_pg_support::knowledge_pg;

macro_rules! pg_or_skip {
    () => {{
        match knowledge_pg().await {
            Some(pg) => pg,
            None => {
                panic!("MT-191 Loom visual-debug proof requires real PostgreSQL");
            }
        }
    }};
}

async fn insert_loom_block(
    db: &handshake_core::storage::postgres::PostgresDatabase,
    ctx: &WriteContext,
    workspace_id: &str,
    content_type: LoomBlockContentType,
    title: &str,
    full_text: Option<&str>,
    pinned: bool,
) -> String {
    db.create_loom_block(
        ctx,
        NewLoomBlock {
            block_id: None,
            workspace_id: workspace_id.to_string(),
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

#[tokio::test]
async fn mt191_loom_visual_debug_snapshot_exposes_navigation_state_from_real_postgres() {
    let pg = pg_or_skip!();
    let workspace_id = pg.create_workspace().await;
    let ctx = WriteContext::human(None);

    let start_block_id = insert_loom_block(
        &pg.db,
        &ctx,
        &workspace_id,
        LoomBlockContentType::Note,
        "VisualDebugAlpha start",
        Some("VisualDebugAlpha anchors the local graph and search debug state."),
        true,
    )
    .await;
    let backlink_source_id = insert_loom_block(
        &pg.db,
        &ctx,
        &workspace_id,
        LoomBlockContentType::Note,
        "VisualDebugAlpha backlink source",
        Some("This note mentions VisualDebugAlpha start and must appear as a backlink snippet."),
        false,
    )
    .await;
    let tag_id = insert_loom_block(
        &pg.db,
        &ctx,
        &workspace_id,
        LoomBlockContentType::TagHub,
        "VisualDebugAlpha tag",
        None,
        false,
    )
    .await;

    for block_id in [&start_block_id, &backlink_source_id, &tag_id] {
        pg.db
            .bridge_loom_block_to_knowledge(&ctx, &workspace_id, block_id)
            .await
            .expect("bridge Loom block to ProjectKnowledgeIndex");
    }

    pg.db
        .create_loom_edge(
            &ctx,
            NewLoomEdge {
                edge_id: None,
                workspace_id: workspace_id.clone(),
                source_block_id: backlink_source_id.clone(),
                target_block_id: start_block_id.clone(),
                edge_type: LoomEdgeType::Mention,
                created_by: LoomEdgeCreatedBy::User,
                crdt_site_id: None,
                source_anchor: None,
            },
        )
        .await
        .expect("mention edge");
    pg.db
        .create_loom_edge(
            &ctx,
            NewLoomEdge {
                edge_id: None,
                workspace_id: workspace_id.clone(),
                source_block_id: start_block_id.clone(),
                target_block_id: tag_id.clone(),
                edge_type: LoomEdgeType::Tag,
                created_by: LoomEdgeCreatedBy::User,
                crdt_site_id: None,
                source_anchor: None,
            },
        )
        .await
        .expect("tag edge");

    let folder = pg
        .db
        .create_loom_folder(
            &workspace_id,
            NewLoomFolder {
                folder_id: None,
                workspace_id: workspace_id.clone(),
                parent_folder_id: None,
                name: "Visual Debug Folder".to_string(),
                color: Some("#2f6fed".to_string()),
                sort_mode: LoomFolderSortMode::Manual,
                sort_order: Some(1),
                project_ref: Some("WP-KERNEL-009".to_string()),
            },
        )
        .await
        .expect("create Loom folder");
    pg.db
        .add_block_to_loom_folder(&workspace_id, &folder.folder_id, &start_block_id, Some(1))
        .await
        .expect("add block to folder");

    let snapshot = pg
        .db
        .loom_visual_debug_snapshot(&workspace_id, &start_block_id, "VisualDebugAlpha", 25)
        .await
        .expect("build Loom visual-debug snapshot");

    assert_eq!(snapshot.schema_id, LOOM_VISUAL_DEBUG_SCHEMA_ID);
    assert_eq!(snapshot.authority_class, "projection");
    assert_eq!(snapshot.authority_backend.as_str(), "postgres_event_ledger");
    assert_eq!(snapshot.start_block_id, start_block_id);
    assert_eq!(snapshot.search.query, "VisualDebugAlpha");

    assert!(snapshot.counts.blocks >= 3);
    assert!(snapshot.counts.edges >= 2);
    assert!(snapshot.counts.folders >= 1);
    assert!(snapshot.counts.folder_members >= 1);
    assert!(snapshot.counts.tag_hubs >= 1);
    assert!(snapshot.counts.pinned_blocks >= 1);
    assert!(snapshot.counts.indexed_bridges >= 3);

    assert!(
        snapshot
            .graph
            .nodes
            .iter()
            .any(|node| node.block_id == backlink_source_id && node.depth == 1),
        "local graph summaries must expose backlink source nodes: {:?}",
        snapshot.graph.nodes
    );
    assert!(
        snapshot
            .graph
            .edges
            .iter()
            .any(|edge| edge.source_block_id == backlink_source_id
                && edge.target_block_id == snapshot.start_block_id
                && edge.edge_type.as_str() == "mention"),
        "local graph summaries must expose mention edges: {:?}",
        snapshot.graph.edges
    );

    assert!(
        snapshot
            .backlinks
            .incoming
            .iter()
            .any(|backlink| backlink.source_block_id == backlink_source_id
                && backlink
                    .context_snippet
                    .as_deref()
                    .is_some_and(|snippet| snippet.contains("VisualDebugAlpha"))),
        "backlink summaries must include bounded context snippets: {:?}",
        snapshot.backlinks.incoming
    );

    assert!(
        snapshot.folders.iter().any(|folder_summary| {
            folder_summary.folder_id == folder.folder_id
                && folder_summary
                    .sample_block_ids
                    .contains(&snapshot.start_block_id)
                && folder_summary.member_count >= 1
        }),
        "folder summaries must expose membership samples: {:?}",
        snapshot.folders
    );
    assert!(
        snapshot
            .search
            .results
            .iter()
            .any(|result| result.ref_id == snapshot.start_block_id
                && result.source_kind.as_str() == "loom_block"),
        "search summaries must expose graph-search hits: {:?}",
        snapshot.search.results
    );
    assert!(
        snapshot
            .route_ids
            .iter()
            .any(|route| route == "loom.visual_debug")
            && snapshot
                .route_ids
                .iter()
                .any(|route| route == "loom.graph_search"),
        "route ids must include the visual-debug and graph-search surfaces"
    );
}

#[test]
fn mt191_usermanual_registry_covers_loom_visual_debug_and_graph_search_routes() {
    let surfaces = wp009_surface_registry();

    assert!(
        surfaces.iter().any(|surface| {
            surface.surface_id == "loom.visual_debug"
                && surface.method == "GET"
                && surface.route == "/workspaces/:workspace_id/loom/visual-debug"
        }),
        "MT-191 must add UserManual registry coverage for the Loom visual-debug route"
    );
    assert!(
        surfaces.iter().any(|surface| {
            surface.surface_id == "loom.graph_search"
                && surface.method == "GET"
                && surface.route == "/workspaces/:workspace_id/loom/graph-search"
        }),
        "MT-191 must keep UserManual registry coverage for graph-search"
    );
}

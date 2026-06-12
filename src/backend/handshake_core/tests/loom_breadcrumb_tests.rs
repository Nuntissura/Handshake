//! WP-KERNEL-009 MT-188 NavigationBreadcrumbs — REAL PostgreSQL proof.
//!
//! MT-188: a breadcrumb trail across the entity spine (workspace -> project ->
//! folder ancestry -> block -> ProjectKnowledgeIndex entity), reusing the
//! MT-181 folder tree + MT-177 bridge. A read projection; no parallel store.

mod knowledge_pg_support;

use handshake_core::storage::{
    Database, LoomBlockContentType, LoomBlockDerived, LoomFolderSortMode, NewLoomBlock,
    NewLoomFolder, WriteContext,
};
use knowledge_pg_support::knowledge_pg;

macro_rules! pg_or_skip {
    () => {{
        match knowledge_pg().await {
            Some(pg) => pg,
            None => {
                eprintln!("SKIP MT-188 loom breadcrumb proof: PostgreSQL unavailable");
                return;
            }
        }
    }};
}

async fn blk(db: &handshake_core::storage::postgres::PostgresDatabase, ws: &str, title: &str) -> String {
    let ctx = WriteContext::human(None);
    db.create_loom_block(
        &ctx,
        NewLoomBlock {
            block_id: None,
            workspace_id: ws.to_string(),
            content_type: LoomBlockContentType::Note,
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

#[tokio::test]
async fn breadcrumbs_span_workspace_folder_ancestry_block_and_entity() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let ctx = WriteContext::human(None);

    // Folder tree: Work (project_ref=proj-x) -> Projects.
    let work = pg
        .db
        .create_loom_folder(
            &ws,
            NewLoomFolder {
                folder_id: None,
                workspace_id: ws.clone(),
                parent_folder_id: None,
                name: "Work".into(),
                color: None,
                sort_mode: LoomFolderSortMode::UpdatedDesc,
                sort_order: None,
                project_ref: Some("proj-x".into()),
            },
        )
        .await
        .expect("work folder")
        .folder_id;
    let projects = pg
        .db
        .create_loom_folder(
            &ws,
            NewLoomFolder {
                folder_id: None,
                workspace_id: ws.clone(),
                parent_folder_id: Some(work.clone()),
                name: "Projects".into(),
                color: None,
                sort_mode: LoomFolderSortMode::UpdatedDesc,
                sort_order: None,
                project_ref: None,
            },
        )
        .await
        .expect("projects folder")
        .folder_id;

    let note = blk(&pg.db, &ws, "Deep Note").await;
    pg.db.add_block_to_loom_folder(&ws, &projects, &note, None).await.expect("member");
    // Bridge the block so the entity crumb appears.
    pg.db.bridge_loom_block_to_knowledge(&ctx, &ws, &note).await.expect("bridge");

    let trail = pg.db.loom_block_breadcrumbs(&ws, &note).await.expect("breadcrumbs");
    assert_eq!(trail.block_id, note);

    let kinds: Vec<&str> = trail.crumbs.iter().map(|c| c.kind.as_str()).collect();
    // Root-first: workspace, project, folder(Work), folder(Projects), block, entity.
    assert_eq!(kinds.first(), Some(&"workspace"), "starts at workspace");
    assert!(kinds.contains(&"project"), "project crumb from folder project_ref");
    assert_eq!(kinds.last(), Some(&"entity"), "ends at the knowledge entity");

    // Folder crumbs are root-first: Work before Projects.
    let folder_labels: Vec<&str> = trail
        .crumbs
        .iter()
        .filter(|c| c.kind == "folder")
        .map(|c| c.label.as_str())
        .collect();
    assert_eq!(folder_labels, vec!["Work", "Projects"], "folder ancestry is root-first");

    // The block crumb is present with its title.
    let block_crumb = trail.crumbs.iter().find(|c| c.kind == "block").unwrap();
    assert_eq!(block_crumb.label, "Deep Note");
    assert_eq!(block_crumb.id, note);
}

#[tokio::test]
async fn breadcrumbs_for_unfiled_unbridged_block_are_minimal() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    // A block in no folder and not bridged: just workspace + block.
    let note = blk(&pg.db, &ws, "Loose Note").await;
    let trail = pg.db.loom_block_breadcrumbs(&ws, &note).await.expect("breadcrumbs");
    let kinds: Vec<&str> = trail.crumbs.iter().map(|c| c.kind.as_str()).collect();
    assert_eq!(kinds, vec!["workspace", "block"], "minimal trail when unfiled + unbridged");
}

#[tokio::test]
async fn breadcrumbs_fail_closed_on_missing_block() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let err = pg.db.loom_block_breadcrumbs(&ws, "loom-missing").await.expect_err("missing");
    assert!(format!("{err}").contains("loom_block") || format!("{err}").contains("not"), "{err}");
}

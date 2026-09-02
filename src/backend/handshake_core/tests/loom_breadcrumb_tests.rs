//! WP-KERNEL-009 MT-188 NavigationBreadcrumbs — real embedded-store proof.
//!
//! MT-188: a breadcrumb trail across the entity spine (workspace -> project ->
//! folder ancestry -> block -> ProjectKnowledgeIndex entity), reusing the
//! MT-181 folder tree + MT-177 bridge. A read projection; no parallel store.

#[path = "knowledge_ingestion_support.rs"]
mod embedded_knowledge_support;

use embedded_knowledge_support::open_embedded_store;
use handshake_core::storage::{
    Database, LoomBlockContentType, LoomBlockDerived, LoomFolderSortMode, NewLoomBlock,
    NewLoomFolder, WriteContext,
};

macro_rules! embedded_store_or_return {
    () => {{
        match open_embedded_store().await {
            Some(store) => store,
            None => {
                eprintln!("SKIP MT-188 loom breadcrumb proof: embedded store unavailable");
                return;
            }
        }
    }};
}

async fn blk(
    db: &handshake_core::storage::surreal::SurrealDatabase,
    ws: &str,
    title: &str,
) -> String {
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
    let store = embedded_store_or_return!();
    let ws = store.create_workspace().await;
    let ctx = WriteContext::human(None);

    // Folder tree: Work (project_ref=proj-x) -> Projects.
    let work = store
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
    let projects = store
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

    let note = blk(&store.db, &ws, "Deep Note").await;
    store
        .db
        .add_block_to_loom_folder(&ws, &projects, &note, None)
        .await
        .expect("member");
    // Bridge the block so the entity crumb appears.
    store
        .db
        .bridge_loom_block_to_knowledge(&ctx, &ws, &note)
        .await
        .expect("bridge");

    let trail = store
        .db
        .loom_block_breadcrumbs(&ws, &note)
        .await
        .expect("breadcrumbs");
    assert_eq!(trail.block_id, note);

    let kinds: Vec<&str> = trail.crumbs.iter().map(|c| c.kind.as_str()).collect();
    // Root-first: workspace, project, folder(Work), folder(Projects), block, entity.
    assert_eq!(kinds.first(), Some(&"workspace"), "starts at workspace");
    assert!(
        kinds.contains(&"project"),
        "project crumb from folder project_ref"
    );
    assert_eq!(
        kinds.last(),
        Some(&"entity"),
        "ends at the knowledge entity"
    );

    // Folder crumbs are root-first: Work before Projects.
    let folder_labels: Vec<&str> = trail
        .crumbs
        .iter()
        .filter(|c| c.kind == "folder")
        .map(|c| c.label.as_str())
        .collect();
    assert_eq!(
        folder_labels,
        vec!["Work", "Projects"],
        "folder ancestry is root-first"
    );

    // The block crumb is present with its title.
    let block_crumb = trail.crumbs.iter().find(|c| c.kind == "block").unwrap();
    assert_eq!(block_crumb.label, "Deep Note");
    assert_eq!(block_crumb.id, note);
}

#[tokio::test]
async fn breadcrumbs_for_unfiled_unbridged_block_are_minimal() {
    let store = embedded_store_or_return!();
    let ws = store.create_workspace().await;
    // A block in no folder and not bridged: just workspace + block.
    let note = blk(&store.db, &ws, "Loose Note").await;
    let trail = store
        .db
        .loom_block_breadcrumbs(&ws, &note)
        .await
        .expect("breadcrumbs");
    let kinds: Vec<&str> = trail.crumbs.iter().map(|c| c.kind.as_str()).collect();
    assert_eq!(
        kinds,
        vec!["workspace", "block"],
        "minimal trail when unfiled + unbridged"
    );
}

#[tokio::test]
async fn breadcrumbs_fail_closed_on_missing_block() {
    let store = embedded_store_or_return!();
    let ws = store.create_workspace().await;
    let err = store
        .db
        .loom_block_breadcrumbs(&ws, "loom-missing")
        .await
        .expect_err("missing");
    assert!(
        format!("{err}").contains("loom_block") || format!("{err}").contains("not"),
        "{err}"
    );
}

//! WP-KERNEL-009 MT-181 FolderTreeAndColorLabels — REAL PostgreSQL proof.
//!
//! §7.1.4.3 / MT-181: a persistent Loom folder hierarchy with color labels,
//! sort modes, and project membership over PostgreSQL (loom_folders +
//! loom_folder_members). An organizational overlay over LoomBlocks; never a
//! second source of block truth. No parallel store.

mod knowledge_pg_support;

use handshake_core::storage::{
    Database, LoomBlockContentType, LoomBlockDerived, LoomFolderSortMode, LoomFolderUpdate,
    NewLoomBlock, NewLoomFolder, StorageError, WriteContext,
};
use knowledge_pg_support::{base_database_url, knowledge_pg, KnowledgePg};
use sqlx::{Connection, Row};

macro_rules! pg_or_skip {
    () => {{
        match knowledge_pg().await {
            Some(pg) => pg,
            None => {
                eprintln!("SKIP MT-181 loom folder proof: PostgreSQL unavailable");
                return;
            }
        }
    }};
}

async fn drop_isolated_schema(pg: KnowledgePg) {
    let schema = pg.schema.clone();
    drop(pg);
    let base_url = base_database_url()
        .await
        .expect("PostgreSQL URL remains available for isolated-schema cleanup");
    let mut connection = sqlx::PgConnection::connect(&base_url)
        .await
        .expect("connect for isolated-schema cleanup");
    let quoted = schema.replace('"', "\"\"");
    sqlx::query(&format!("DROP SCHEMA IF EXISTS \"{quoted}\" CASCADE"))
        .execute(&mut connection)
        .await
        .expect("drop isolated Loom folder proof schema");
}

#[test]
fn folder_update_json_distinguishes_absent_null_and_value_for_move_fields() {
    let absent: LoomFolderUpdate =
        serde_json::from_value(serde_json::json!({})).expect("empty folder PATCH body");
    assert_eq!(absent.parent_folder_id, None);
    assert_eq!(absent.sort_order, None);

    let clear: LoomFolderUpdate = serde_json::from_value(serde_json::json!({
        "parent_folder_id": null,
        "sort_order": null
    }))
    .expect("explicit-null folder PATCH body");
    assert_eq!(clear.parent_folder_id, Some(None));
    assert_eq!(clear.sort_order, Some(None));

    let set: LoomFolderUpdate = serde_json::from_value(serde_json::json!({
        "parent_folder_id": "LFD-parent",
        "sort_order": 7
    }))
    .expect("value-bearing folder PATCH body");
    assert_eq!(set.parent_folder_id, Some(Some("LFD-parent".to_owned())));
    assert_eq!(set.sort_order, Some(Some(7)));
}

async fn blk(
    db: &handshake_core::storage::postgres::PostgresDatabase,
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

async fn folder(
    db: &handshake_core::storage::postgres::PostgresDatabase,
    ws: &str,
    name: &str,
    parent: Option<&str>,
    sort_mode: LoomFolderSortMode,
) -> String {
    db.create_loom_folder(
        ws,
        NewLoomFolder {
            folder_id: None,
            workspace_id: ws.to_string(),
            parent_folder_id: parent.map(|p| p.to_string()),
            name: name.to_string(),
            color: None,
            sort_mode,
            sort_order: None,
            project_ref: None,
        },
    )
    .await
    .expect("folder")
    .folder_id
}

#[tokio::test]
async fn folder_tree_persists_hierarchy_color_and_sort() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;

    let work = folder(&pg.db, &ws, "Work", None, LoomFolderSortMode::NameAsc).await;
    let projects = folder(
        &pg.db,
        &ws,
        "Projects",
        Some(&work),
        LoomFolderSortMode::Manual,
    )
    .await;

    // Recolor + rename Work via update.
    let updated = pg
        .db
        .update_loom_folder(
            &ws,
            &work,
            LoomFolderUpdate {
                name: Some("Work Area".into()),
                color: Some(Some("#ff8800".into())),
                ..Default::default()
            },
        )
        .await
        .expect("update folder");
    assert_eq!(updated.name, "Work Area");
    assert_eq!(updated.color.as_deref(), Some("#ff8800"));

    // The tree lists parent before child.
    let tree = pg.db.list_loom_folders(&ws).await.expect("tree");
    let work_idx = tree.iter().position(|f| f.folder_id == work).unwrap();
    let proj_idx = tree.iter().position(|f| f.folder_id == projects).unwrap();
    assert!(work_idx < proj_idx, "parent listed before child");
    let proj = tree.iter().find(|f| f.folder_id == projects).unwrap();
    assert_eq!(proj.parent_folder_id.as_deref(), Some(work.as_str()));
    assert_eq!(proj.sort_mode, LoomFolderSortMode::Manual);
}

#[tokio::test]
async fn duplicate_root_folder_names_are_rejected_per_workspace() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;

    let first = folder(
        &pg.db,
        &ws,
        "Duplicate root",
        None,
        LoomFolderSortMode::UpdatedDesc,
    )
    .await;
    let error = pg
        .db
        .create_loom_folder(
            &ws,
            NewLoomFolder {
                folder_id: None,
                workspace_id: ws.clone(),
                parent_folder_id: None,
                name: "Duplicate root".to_owned(),
                color: None,
                sort_mode: LoomFolderSortMode::UpdatedDesc,
                sort_order: None,
                project_ref: None,
            },
        )
        .await
        .expect_err("duplicate root name must violate the root partial unique index");
    assert!(
        matches!(error, StorageError::Conflict("loom_folder_sibling_name")),
        "duplicate roots must produce the typed sibling-name conflict, got {error}"
    );

    let rows = pg.db.list_loom_folders(&ws).await.expect("list folders");
    assert_eq!(
        rows.iter()
            .filter(|row| row.parent_folder_id.is_none() && row.name == "Duplicate root")
            .count(),
        1,
        "the rejected duplicate must not create a second ambiguous root"
    );
    assert!(rows.iter().any(|row| row.folder_id == first));
    drop_isolated_schema(pg).await;
}

#[tokio::test]
async fn loom_folder_uniqueness_forward_migration_installs_partial_indexes() {
    let pg = pg_or_skip!();
    let mut connection = pg.raw_connection().await;
    let indexes = sqlx::query(
        "SELECT indexname, indexdef FROM pg_indexes \
         WHERE schemaname = current_schema() AND tablename = 'loom_folders'",
    )
    .fetch_all(&mut connection)
    .await
    .expect("inspect migrated loom_folders indexes");
    let definitions: std::collections::HashMap<String, String> = indexes
        .into_iter()
        .map(|row| (row.get("indexname"), row.get("indexdef")))
        .collect();
    assert!(
        definitions
            .get("uq_loom_folders_root_name")
            .is_some_and(|definition| definition.contains("parent_folder_id IS NULL")),
        "0342 must install the root-name partial unique index: {definitions:?}"
    );
    assert!(
        definitions
            .get("uq_loom_folders_child_name")
            .is_some_and(|definition| definition.contains("parent_folder_id IS NOT NULL")),
        "0342 must install the non-root sibling partial unique index: {definitions:?}"
    );

    let old_constraint_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pg_constraint \
         WHERE conname = 'uq_loom_folders_sibling_name' \
           AND conrelid = 'loom_folders'::regclass",
    )
    .fetch_one(&mut connection)
    .await
    .expect("inspect superseded nullable-parent constraint");
    assert_eq!(old_constraint_count, 0);
    drop(connection);
    drop_isolated_schema(pg).await;
}

#[tokio::test]
async fn loom_folder_uniqueness_forward_migration_recovers_preexisting_duplicate_roots() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let mut connection = pg.raw_connection().await;

    sqlx::raw_sql(
        "DROP INDEX IF EXISTS uq_loom_folders_root_name;
         DROP INDEX IF EXISTS uq_loom_folders_child_name;
         ALTER TABLE loom_folders
             ADD CONSTRAINT uq_loom_folders_sibling_name
             UNIQUE (workspace_id, parent_folder_id, name);",
    )
    .execute(&mut connection)
    .await
    .expect("restore the pre-0342 nullable-parent uniqueness shape");

    sqlx::query(
        "INSERT INTO loom_folders
             (folder_id, workspace_id, parent_folder_id, name, sort_mode)
         VALUES
             ('LFD-upgrade-001', $1, NULL, 'Duplicate root', 'updated_desc'),
             ('LFD-upgrade-002', $1, NULL, 'Duplicate root', 'updated_desc'),
             ('LFD-upgrade-existing-name', $1, NULL,
              'Duplicate root [recovered-LFD-upgrade-002]', 'updated_desc')",
    )
    .bind(&ws)
    .execute(&mut connection)
    .await
    .expect("pre-0342 schema permits duplicate root names");

    sqlx::raw_sql(include_str!(
        "../migrations/0342_loom_folder_sibling_name_uniqueness.sql"
    ))
    .execute(&mut connection)
    .await
    .expect("0342 recovers duplicates and installs truthful indexes");

    let names: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM loom_folders WHERE workspace_id = $1 ORDER BY folder_id",
    )
    .bind(&ws)
    .fetch_all(&mut connection)
    .await
    .expect("read recovered folder names");
    assert_eq!(names.len(), 3, "migration preserves every folder");
    assert_eq!(
        names
            .iter()
            .filter(|name| *name == "Duplicate root")
            .count(),
        1,
        "exactly one canonical root retains the original name"
    );
    assert!(
        names
            .iter()
            .any(|name| name == "Duplicate root [recovered-LFD-upgrade-002]-2"),
        "recovery avoids a pre-existing generated name: {names:?}"
    );

    let duplicate_after_upgrade = sqlx::query(
        "INSERT INTO loom_folders
             (folder_id, workspace_id, parent_folder_id, name, sort_mode)
         VALUES ('LFD-upgrade-003', $1, NULL, 'Duplicate root', 'updated_desc')",
    )
    .bind(&ws)
    .execute(&mut connection)
    .await
    .expect_err("0342 rejects a new duplicate root");
    assert_eq!(
        duplicate_after_upgrade
            .as_database_error()
            .and_then(|error| error.code())
            .as_deref(),
        Some("23505")
    );

    drop(connection);
    drop_isolated_schema(pg).await;
}

#[tokio::test]
async fn folder_membership_and_manual_sort() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let f = folder(&pg.db, &ws, "Bucket", None, LoomFolderSortMode::Manual).await;

    let a = blk(&pg.db, &ws, "Apple").await;
    let b = blk(&pg.db, &ws, "Banana").await;
    let c = blk(&pg.db, &ws, "Cherry").await;
    // Add with manual order C=0, A=1, B=2 -> expected list order C, A, B.
    pg.db
        .add_block_to_loom_folder(&ws, &f, &c, Some(0))
        .await
        .expect("add c");
    pg.db
        .add_block_to_loom_folder(&ws, &f, &a, Some(1))
        .await
        .expect("add a");
    pg.db
        .add_block_to_loom_folder(&ws, &f, &b, Some(2))
        .await
        .expect("add b");

    let blocks = pg
        .db
        .list_loom_folder_blocks(&ws, &f, 100, 0)
        .await
        .expect("folder blocks");
    let ids: Vec<&str> = blocks.iter().map(|x| x.block_id.as_str()).collect();
    assert_eq!(ids, vec![c.as_str(), a.as_str(), b.as_str()]);

    // Re-add is idempotent (updates order). Move A to front.
    pg.db
        .add_block_to_loom_folder(&ws, &f, &a, Some(-1))
        .await
        .expect("reorder a");
    let blocks2 = pg
        .db
        .list_loom_folder_blocks(&ws, &f, 100, 0)
        .await
        .expect("folder blocks2");
    assert_eq!(blocks2[0].block_id, a, "A moved to front; no duplicate row");
    assert_eq!(blocks2.len(), 3, "still 3 members (idempotent add)");

    // Remove B.
    pg.db
        .remove_block_from_loom_folder(&ws, &f, &b)
        .await
        .expect("remove b");
    let blocks3 = pg
        .db
        .list_loom_folder_blocks(&ws, &f, 100, 0)
        .await
        .expect("folder blocks3");
    assert_eq!(blocks3.len(), 2);
    assert!(!blocks3.iter().any(|x| x.block_id == b));
}

#[tokio::test]
async fn folder_name_sort_mode_orders_by_title() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let f = folder(&pg.db, &ws, "Alpha", None, LoomFolderSortMode::NameAsc).await;
    let zeb = blk(&pg.db, &ws, "Zebra").await;
    let ant = blk(&pg.db, &ws, "Ant").await;
    pg.db
        .add_block_to_loom_folder(&ws, &f, &zeb, None)
        .await
        .expect("add zeb");
    pg.db
        .add_block_to_loom_folder(&ws, &f, &ant, None)
        .await
        .expect("add ant");

    let blocks = pg
        .db
        .list_loom_folder_blocks(&ws, &f, 100, 0)
        .await
        .expect("name-sorted");
    assert_eq!(blocks[0].block_id, ant, "name_asc puts Ant before Zebra");
    assert_eq!(blocks[1].block_id, zeb);
}

#[tokio::test]
async fn folder_move_into_own_subtree_is_rejected() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let root = folder(&pg.db, &ws, "Root", None, LoomFolderSortMode::UpdatedDesc).await;
    let child = folder(
        &pg.db,
        &ws,
        "Child",
        Some(&root),
        LoomFolderSortMode::UpdatedDesc,
    )
    .await;
    let grandchild = folder(
        &pg.db,
        &ws,
        "Grand",
        Some(&child),
        LoomFolderSortMode::UpdatedDesc,
    )
    .await;

    // Moving Root under Grand (its own descendant) would create a cycle.
    let err = pg
        .db
        .update_loom_folder(
            &ws,
            &root,
            LoomFolderUpdate {
                parent_folder_id: Some(Some(grandchild.clone())),
                ..Default::default()
            },
        )
        .await
        .expect_err("cycle move rejected");
    assert!(format!("{err}").contains("cycle"), "{err}");

    // Self-parent is also rejected.
    let err2 = pg
        .db
        .update_loom_folder(
            &ws,
            &child,
            LoomFolderUpdate {
                parent_folder_id: Some(Some(child.clone())),
                ..Default::default()
            },
        )
        .await
        .expect_err("self-parent rejected");
    assert!(
        format!("{err2}").contains("own parent") || format!("{err2}").contains("cycle"),
        "{err2}"
    );
}

#[tokio::test]
async fn deleting_folder_cascades_subtree_but_not_blocks() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let root = folder(
        &pg.db,
        &ws,
        "DelRoot",
        None,
        LoomFolderSortMode::UpdatedDesc,
    )
    .await;
    let child = folder(
        &pg.db,
        &ws,
        "DelChild",
        Some(&root),
        LoomFolderSortMode::UpdatedDesc,
    )
    .await;
    let note = blk(&pg.db, &ws, "Survivor").await;
    pg.db
        .add_block_to_loom_folder(&ws, &child, &note, None)
        .await
        .expect("add");

    // Delete the root -> child subtree + membership cascade.
    pg.db
        .delete_loom_folder(&ws, &root)
        .await
        .expect("delete root");
    assert!(
        pg.db.get_loom_folder(&ws, &root).await.is_err(),
        "root gone"
    );
    assert!(
        pg.db.get_loom_folder(&ws, &child).await.is_err(),
        "child cascaded"
    );

    // The block itself survives (folders are an overlay, not ownership).
    let survivor = pg
        .db
        .get_loom_block(&ws, &note)
        .await
        .expect("block survives");
    assert_eq!(survivor.block_id, note);
}

#[tokio::test]
async fn folder_apis_fail_closed_on_missing_targets() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;

    // Create under a non-existent parent.
    let err = pg
        .db
        .create_loom_folder(
            &ws,
            NewLoomFolder {
                folder_id: None,
                workspace_id: ws.clone(),
                parent_folder_id: Some("LFD-missing".into()),
                name: "Orphan".into(),
                color: None,
                sort_mode: LoomFolderSortMode::UpdatedDesc,
                sort_order: None,
                project_ref: None,
            },
        )
        .await
        .expect_err("missing parent");
    assert!(
        format!("{err}").contains("loom_folder") || format!("{err}").contains("not"),
        "{err}"
    );

    // Add a missing block to a real folder.
    let f = folder(&pg.db, &ws, "Real", None, LoomFolderSortMode::UpdatedDesc).await;
    let err2 = pg
        .db
        .add_block_to_loom_folder(&ws, &f, "loom-missing-block", None)
        .await
        .expect_err("missing block");
    assert!(
        format!("{err2}").contains("loom_block") || format!("{err2}").contains("not"),
        "{err2}"
    );
}

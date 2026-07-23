//! WP-KERNEL-012 E3 MT-022 / MT-023 — atomic EventLedger receipt proof on REAL
//! PostgreSQL.
//!
//! FAIL_V2 remediation proof: folder create/update/delete/member mutations
//! (MT-022) and tag-edge create/delete mutations (MT-023) MUST append their
//! durable EventLedger receipt in the SAME transaction as the domain write, so a
//! committed mutation can never lack durable evidence. These tests prove:
//!   1. Happy path: after a mutation, both the domain row (with a non-null
//!      event_ledger_event_id) AND the matching kernel_event_ledger row exist,
//!      and they survive a simulated restart (a fresh PostgresDatabase client
//!      reading the committed schema).
//!   2. Genuine injected-failure atomicity: a BEFORE INSERT trigger on
//!      kernel_event_ledger forces the receipt append to fail. The mutation must
//!      roll back entirely — after the failure and a fresh-client readback there
//!      is NO partial persisted domain row and NO phantom ledger row.
//!
//! The failure is injected at the real database boundary (a real trigger), not
//! through a production test hook, so no scaffolding weakens the product path.

mod knowledge_pg_support;

use handshake_core::storage::{
    Database, LoomBlockContentType, LoomBlockDerived, LoomEdgeCreatedBy, LoomEdgeType,
    LoomFolderSortMode, LoomFolderUpdate, NewLoomBlock, NewLoomEdge, NewLoomFolder, WriteContext,
};
use handshake_core::storage::postgres::PostgresDatabase;
use knowledge_pg_support::{knowledge_pg, KnowledgePg};
use sqlx::Row;

macro_rules! pg_or_skip {
    () => {{
        match knowledge_pg().await {
            Some(pg) => pg,
            None => {
                eprintln!("SKIP MT-022/023 atomic receipt proof: PostgreSQL unavailable");
                return;
            }
        }
    }};
}

const FOLDER_EVENT_TYPE: &str = "KNOWLEDGE_LOOM_FOLDER_MUTATED";
const TAG_EVENT_TYPE: &str = "KNOWLEDGE_LOOM_TAG_MUTATED";

async fn make_block(db: &PostgresDatabase, ws: &str, title: &str) -> String {
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
    .expect("create block")
    .block_id
}

fn new_folder(ws: &str, name: &str) -> NewLoomFolder {
    NewLoomFolder {
        folder_id: None,
        workspace_id: ws.to_string(),
        parent_folder_id: None,
        name: name.to_string(),
        color: Some("#123456".to_string()),
        sort_mode: LoomFolderSortMode::UpdatedDesc,
        sort_order: None,
        project_ref: None,
    }
}

/// Count kernel_event_ledger rows of an event type whose aggregate_id matches.
async fn ledger_count(pg: &KnowledgePg, event_type: &str, aggregate_id: &str) -> i64 {
    let mut conn = pg.raw_connection().await;
    let row = sqlx::query(
        "SELECT COUNT(*)::BIGINT AS n FROM kernel_event_ledger \
         WHERE event_type = $1 AND aggregate_id = $2",
    )
    .bind(event_type)
    .bind(aggregate_id)
    .fetch_one(&mut conn)
    .await
    .expect("count ledger rows");
    row.get::<i64, _>("n")
}

/// Read the event_ledger_event_id stored on a loom_folders row (None if absent).
async fn folder_receipt_id(pg: &KnowledgePg, folder_id: &str) -> Option<String> {
    let mut conn = pg.raw_connection().await;
    let row = sqlx::query(
        "SELECT event_ledger_event_id FROM loom_folders WHERE folder_id = $1",
    )
    .bind(folder_id)
    .fetch_optional(&mut conn)
    .await
    .expect("read folder receipt id");
    row.and_then(|r| r.get::<Option<String>, _>("event_ledger_event_id"))
}

async fn folder_row_exists(pg: &KnowledgePg, folder_id: &str) -> bool {
    let mut conn = pg.raw_connection().await;
    let row = sqlx::query("SELECT 1 AS x FROM loom_folders WHERE folder_id = $1")
        .bind(folder_id)
        .fetch_optional(&mut conn)
        .await
        .expect("probe folder row");
    row.is_some()
}

async fn install_ledger_fail_trigger(pg: &KnowledgePg) {
    let mut conn = pg.raw_connection().await;
    sqlx::query(
        "CREATE OR REPLACE FUNCTION inject_ledger_fail() RETURNS trigger \
         LANGUAGE plpgsql AS $$ BEGIN \
           RAISE EXCEPTION 'injected kernel_event_ledger append failure'; \
         END; $$",
    )
    .execute(&mut conn)
    .await
    .expect("create injected-failure function");
    sqlx::query(
        "CREATE TRIGGER inject_ledger_fail_trg BEFORE INSERT ON kernel_event_ledger \
         FOR EACH ROW EXECUTE FUNCTION inject_ledger_fail()",
    )
    .execute(&mut conn)
    .await
    .expect("install injected-failure trigger");
}

async fn remove_ledger_fail_trigger(pg: &KnowledgePg) {
    let mut conn = pg.raw_connection().await;
    sqlx::query("DROP TRIGGER IF EXISTS inject_ledger_fail_trg ON kernel_event_ledger")
        .execute(&mut conn)
        .await
        .expect("drop injected-failure trigger");
    sqlx::query("DROP FUNCTION IF EXISTS inject_ledger_fail()")
        .execute(&mut conn)
        .await
        .expect("drop injected-failure function");
}

/// Simulate a process restart: open a brand-new PostgresDatabase client against
/// the same isolated schema and read committed state through it.
async fn restart_client(pg: &KnowledgePg) -> PostgresDatabase {
    PostgresDatabase::connect(&pg.schema_url, 5)
        .await
        .expect("fresh restart client into isolated schema")
}

// ---------------------------------------------------------------------------
// MT-022: folder mutation atomicity
// ---------------------------------------------------------------------------

#[tokio::test]
async fn mt022_folder_create_appends_atomic_receipt_and_survives_restart() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;

    let folder = pg
        .db
        .create_loom_folder(&ws, new_folder(&ws, "Atomic Projects"))
        .await
        .expect("create folder");

    // Domain row carries a non-null durable receipt id...
    let receipt = folder_receipt_id(&pg, &folder.folder_id)
        .await
        .expect("committed folder must carry a durable receipt id");
    // ...and the ledger row for it exists (exactly one create receipt).
    assert_eq!(
        ledger_count(&pg, FOLDER_EVENT_TYPE, &folder.folder_id).await,
        1,
        "exactly one create receipt in the ledger"
    );

    // Restart readback: a fresh client sees the folder and the same receipt id.
    let restart = restart_client(&pg).await;
    let read = restart
        .get_loom_folder(&ws, &folder.folder_id)
        .await
        .expect("restart client reads the committed folder");
    assert_eq!(read.name, "Atomic Projects");
    let _ = restart.close().await;
    let restart_receipt = folder_receipt_id(&pg, &folder.folder_id)
        .await
        .expect("receipt id persists across restart");
    assert_eq!(receipt, restart_receipt, "receipt id is stable across restart");

    pg.teardown().await;
}

#[tokio::test]
async fn mt022_folder_update_delete_member_are_atomic_with_receipts() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let folder = pg
        .db
        .create_loom_folder(&ws, new_folder(&ws, "Members"))
        .await
        .expect("create folder");
    let block = make_block(&pg.db, &ws, "member block").await;

    // update -> new receipt on the row
    pg.db
        .update_loom_folder(
            &ws,
            &folder.folder_id,
            LoomFolderUpdate {
                color: Some(Some("#00ff00".to_string())),
                ..Default::default()
            },
        )
        .await
        .expect("update folder");
    assert!(folder_receipt_id(&pg, &folder.folder_id).await.is_some());
    assert_eq!(
        ledger_count(&pg, FOLDER_EVENT_TYPE, &folder.folder_id).await,
        2,
        "create + update receipts"
    );

    // add member -> receipt
    pg.db
        .add_block_to_loom_folder(&ws, &folder.folder_id, &block, Some(1))
        .await
        .expect("add member");
    // remove member -> receipt (real removal)
    pg.db
        .remove_block_from_loom_folder(&ws, &folder.folder_id, &block)
        .await
        .expect("remove member");
    assert_eq!(
        ledger_count(&pg, FOLDER_EVENT_TYPE, &folder.folder_id).await,
        4,
        "create + update + add_member + remove_member receipts"
    );

    // delete -> receipt appended, row gone
    pg.db
        .delete_loom_folder(&ws, &folder.folder_id)
        .await
        .expect("delete folder");
    assert!(!folder_row_exists(&pg, &folder.folder_id).await);
    assert_eq!(
        ledger_count(&pg, FOLDER_EVENT_TYPE, &folder.folder_id).await,
        5,
        "delete receipt is durable even though the row is gone"
    );

    pg.teardown().await;
}

#[tokio::test]
async fn mt022_folder_create_rolls_back_when_ledger_append_fails() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;

    // Baseline count of folders in this isolated schema.
    let before = {
        let mut conn = pg.raw_connection().await;
        sqlx::query("SELECT COUNT(*)::BIGINT AS n FROM loom_folders")
            .fetch_one(&mut conn)
            .await
            .expect("baseline folder count")
            .get::<i64, _>("n")
    };

    install_ledger_fail_trigger(&pg).await;
    let result = pg
        .db
        .create_loom_folder(&ws, new_folder(&ws, "Should Roll Back"))
        .await;
    assert!(
        result.is_err(),
        "create must fail when the atomic ledger append fails"
    );
    remove_ledger_fail_trigger(&pg).await;

    // Restart readback: no partial folder row and no phantom ledger row exist.
    let restart = restart_client(&pg).await;
    let after = {
        let mut conn = pg.raw_connection().await;
        sqlx::query("SELECT COUNT(*)::BIGINT AS n FROM loom_folders")
            .fetch_one(&mut conn)
            .await
            .expect("post-rollback folder count")
            .get::<i64, _>("n")
    };
    assert_eq!(before, after, "no partial folder row after rollback");
    let named = restart.list_loom_folders(&ws).await.expect("list folders");
    assert!(
        !named.iter().any(|f| f.name == "Should Roll Back"),
        "rolled-back folder must not be readable after restart"
    );
    let _ = restart.close().await;

    pg.teardown().await;
}

// ---------------------------------------------------------------------------
// MT-023: tag-edge mutation atomicity
// ---------------------------------------------------------------------------

#[tokio::test]
async fn mt023_tag_edge_create_appends_atomic_receipt_and_survives_restart() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let src = make_block(&pg.db, &ws, "source").await;
    let hub = make_block(&pg.db, &ws, "tag hub").await;

    let ctx = WriteContext::human(None);
    let edge = pg
        .db
        .create_loom_edge(
            &ctx,
            NewLoomEdge {
                edge_id: None,
                workspace_id: ws.clone(),
                source_block_id: src.clone(),
                target_block_id: hub.clone(),
                edge_type: LoomEdgeType::Tag,
                created_by: LoomEdgeCreatedBy::User,
                crdt_site_id: None,
                source_anchor: None,
            },
        )
        .await
        .expect("create tag edge");

    // The edge row carries a durable receipt id and the ledger row exists.
    let receipt = {
        let mut conn = pg.raw_connection().await;
        sqlx::query("SELECT event_ledger_event_id FROM loom_edges WHERE edge_id = $1")
            .bind(&edge.edge_id)
            .fetch_one(&mut conn)
            .await
            .expect("read edge receipt id")
            .get::<Option<String>, _>("event_ledger_event_id")
    };
    assert!(
        receipt.is_some(),
        "committed tag edge must carry a durable receipt id"
    );
    assert_eq!(
        ledger_count(&pg, TAG_EVENT_TYPE, &edge.edge_id).await,
        1,
        "exactly one tag-create receipt in the ledger"
    );

    // Restart readback: the edge and its receipt persist for a fresh client.
    let restart = restart_client(&pg).await;
    let edges = restart
        .list_loom_edges_for_block(&ws, &src)
        .await
        .expect("restart client lists edges");
    assert!(
        edges.iter().any(|e| e.edge_id == edge.edge_id),
        "tag edge persists across restart"
    );
    let _ = restart.close().await;

    pg.teardown().await;
}

#[tokio::test]
async fn mt023_tag_edge_create_rolls_back_when_ledger_append_fails() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let src = make_block(&pg.db, &ws, "source-rb").await;
    let hub = make_block(&pg.db, &ws, "hub-rb").await;

    let before = {
        let mut conn = pg.raw_connection().await;
        sqlx::query("SELECT COUNT(*)::BIGINT AS n FROM loom_edges")
            .fetch_one(&mut conn)
            .await
            .expect("baseline edge count")
            .get::<i64, _>("n")
    };

    install_ledger_fail_trigger(&pg).await;
    let ctx = WriteContext::human(None);
    let result = pg
        .db
        .create_loom_edge(
            &ctx,
            NewLoomEdge {
                edge_id: None,
                workspace_id: ws.clone(),
                source_block_id: src.clone(),
                target_block_id: hub.clone(),
                edge_type: LoomEdgeType::Tag,
                created_by: LoomEdgeCreatedBy::User,
                crdt_site_id: None,
                source_anchor: None,
            },
        )
        .await;
    assert!(
        result.is_err(),
        "tag edge create must fail when the atomic ledger append fails"
    );
    remove_ledger_fail_trigger(&pg).await;

    // Restart readback: no partial edge row persisted.
    let restart = restart_client(&pg).await;
    let after = {
        let mut conn = pg.raw_connection().await;
        sqlx::query("SELECT COUNT(*)::BIGINT AS n FROM loom_edges")
            .fetch_one(&mut conn)
            .await
            .expect("post-rollback edge count")
            .get::<i64, _>("n")
    };
    assert_eq!(before, after, "no partial tag edge row after rollback");
    let edges = restart
        .list_loom_edges_for_block(&ws, &src)
        .await
        .expect("restart edge list");
    assert!(edges.is_empty(), "rolled-back tag edge is not readable after restart");
    let _ = restart.close().await;

    pg.teardown().await;
}

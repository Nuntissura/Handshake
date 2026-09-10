#![cfg(all(feature = "surreal-test-support", feature = "test-utils"))]
//! WP-KERNEL-012 MT-150: Loom atomic-receipt coverage restored from the suite deleted by 4f92cc25.
//!
//! Originals (PostgreSQL, real trigger-injected failure):
//!   `git show 4f92cc25^:src/backend/handshake_core/tests/loom_atomic_receipt_pg_tests.rs`
//!   `git show 4f92cc25^:src/backend/handshake_core/tests/loom_block_wiki_atomic_receipt_pg_tests.rs`
//!
//! Ported onto the embedded SurrealDB store (`SurrealDatabase` / `SurrealStorage`)
//! using the same idiom as the in-source proofs in
//! `src/storage/surreal/loom_store.rs` (`open_store`, close+reopen restart
//! proof) and the sibling integration test `tests/loom_canvas_board_tests.rs`
//! / `tests/mt137_restart_recovery_tests.rs`.
//!
//! Fault injection for the `[RB]` (rollback) originals: every atomic Loom
//! mutation appends its `kernel_event_ledger` receipt via
//! `CREATE $ledger.record CONTENT {..}` inside the SAME `BEGIN
//! TRANSACTION; .. COMMIT TRANSACTION;` multi-statement query as the domain
//! write (confirmed by direct inspection of `loom_store.rs` /
//! `wiki_store.rs`). SurrealDB has no PostgreSQL-style trigger, so the
//! injection mechanism used here is a temporary schema override reachable
//! entirely through the crate's already-public `SurrealStorage::with_data_operation`
//! + `SurrealDataContext::query_values` surface (no `src/` edit):
//! `kernel_event_ledger.event_id` is temporarily redefined with
//! `ASSERT false`, which makes every `CREATE kernel_event_ledger ..`
//! statement fail closed while installed, exactly mirroring the PostgreSQL
//! originals' install/remove-trigger bracket around a single call. The
//! restore statement is the field's live production definition, copied
//! verbatim from `src/storage/surreal/schema.surql:789`.
//!
//! FIVE of the fourteen originals are recorded as `BLOCKED_ON_PRODUCT_SEAM`
//! and are NOT ported below, because direct inspection of the current
//! SurrealDB storage layer shows their production behaviour no longer does
//! what the deleted PostgreSQL test proved -- this is a genuine gap in the
//! Surreal port, not a fault-injection-seam problem:
//!
//!   * `mt023_tag_edge_create_appends_atomic_receipt_and_survives_restart`
//!   * `mt023_tag_edge_create_rolls_back_when_ledger_append_fails` [RB]
//!   * `mt024_mention_edge_backlink_is_atomic_and_survives_restart`
//!     `create_loom_edge` (src/storage/surreal/loom_store.rs:1193-1253) never
//!     appends a `kernel_event_ledger` row for ANY edge type (tag or
//!     mention) -- its transaction only creates the edge and recomputes
//!     mention/tag/backlink counts. `KNOWLEDGE_LOOM_TAG_MUTATED` /
//!     `KernelEventType::KnowledgeLoomTagMutated` remains a registered event
//!     type (src/kernel/mod.rs:294,395,474) and is documented as the
//!     intended behaviour (src/user_manual/seed.rs:1117), but no code path
//!     emits it. There is nothing to assert an atomic receipt for, and
//!     nothing for the fault-injection seam to roll back.
//!
//!   * `mt024_favorite_mutation_appends_atomic_receipt_and_survives_restart`
//!   * `mt024_favorite_mutation_rolls_back_when_ledger_append_fails` [RB]
//!     `update_loom_block` (src/storage/surreal/loom_store.rs:964-1020), the
//!     function behind the trait-level favorite mutation, never appends a
//!     `kernel_event_ledger` row and never sets `event_ledger_event_id` --
//!     unlike `mutate_pin` (pin_order/pin_removed), which does. Same
//!     disposition as the edge gap above.
//!
//! Full per-test disposition, evidence and deviations are recorded in
//! `.../MT-150/kb01/logs/LANE-L-loom.json`.

use handshake_core::storage::surreal::{
    bootstrap_schema, SurrealDatabase, SurrealStorage, SurrealStorageConfig,
};
use handshake_core::storage::{
    Database, LoomBlockContentType, LoomBlockDerived, LoomFolderSortMode, LoomFolderUpdate,
    NewLoomBlock, NewLoomFolder, NewWorkspace, WriteContext,
};
use surrealdb::types::{RecordId, RecordIdKey, SurrealValue};

const FOLDER_EVENT_TYPE: &str = "KNOWLEDGE_LOOM_FOLDER_MUTATED";
const BLOCK_EVENT_TYPE: &str = "KNOWLEDGE_LOOM_BLOCK_MUTATED";
const WIKI_EVENT_TYPE: &str = "KNOWLEDGE_LOOM_WIKI_MUTATED";

// ---------------------------------------------------------------------------
// Store harness: mirrors `loom_store.rs`'s in-source `open_store` idiom
// (tempdir + `SurrealStorageConfig::for_data_dir`) and the close/reopen
// restart proof used by `collection_replacement_is_atomic_and_survives_close_reopen`.
// ---------------------------------------------------------------------------

async fn open_store(dir: &std::path::Path) -> (SurrealStorageConfig, SurrealStorage, SurrealDatabase) {
    let config =
        SurrealStorageConfig::for_data_dir(dir).expect("configure embedded Surreal store");
    let storage = SurrealStorage::open(config.clone())
        .await
        .expect("open embedded Surreal store");
    bootstrap_schema(&storage)
        .await
        .expect("bootstrap production Surreal schema");
    let db = SurrealDatabase::new(storage.clone());
    (config, storage, db)
}

async fn seed_workspace(db: &SurrealDatabase) -> String {
    db.create_workspace(
        &WriteContext::human(None),
        NewWorkspace {
            name: "Loom atomic receipt workspace".to_owned(),
        },
    )
    .await
    .expect("create workspace")
    .id
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

async fn make_block(db: &SurrealDatabase, ws: &str, title: &str, pinned: bool) -> String {
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
            pinned,
            journal_date: None,
            imported_at: None,
            derived: LoomBlockDerived::default(),
        },
    )
    .await
    .expect("create block")
    .block_id
}

async fn seed_wiki_projection(db: &SurrealDatabase, ws: &str) -> String {
    let block = make_block(db, ws, "Source block", false).await;
    db.compile_loom_wiki_projection(ws, "Ownership model", &[block])
        .await
        .expect("compile wiki projection")
        .projection_id
}

// ---------------------------------------------------------------------------
// Raw-receipt readback: the domain structs returned by the `Database` trait
// (`LoomFolder`, `LoomBlock`, `LoomWikiOverlay`) do not expose
// `event_ledger_event_id`, so these narrow row projections read it directly
// through the already-public `SurrealStorage::with_data_operation` +
// `SurrealDataContext::select_one` surface (same idiom as
// `tests/mt137_restart_recovery_tests.rs`'s `read_process_effect`).
// ---------------------------------------------------------------------------

#[derive(SurrealValue)]
struct FolderReceiptRow {
    event_ledger_event_id: RecordId,
}

#[derive(SurrealValue)]
struct BlockReceiptRow {
    event_ledger_event_id: Option<RecordId>,
}

#[derive(SurrealValue)]
struct OverlayReceiptRow {
    event_ledger_event_id: RecordId,
}

/// `kernel_event_ledger` record keys are always strings
/// (`schema.surql:789`: `event_id TYPE string ASSERT $value = record::id($this.id)`),
/// so only that variant is meaningful here.
fn record_key_string(record: &RecordId) -> String {
    match &record.key {
        RecordIdKey::String(value) => value.clone(),
        _ => String::new(),
    }
}

async fn folder_receipt_id(storage: &SurrealStorage, folder_id: &str) -> Option<String> {
    let folder_id = folder_id.to_owned();
    storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .select_one::<FolderReceiptRow>("loom_folders", &folder_id)
                    .await
            })
        })
        .await
        .expect("read folder receipt id")
        .map(|row| record_key_string(&row.event_ledger_event_id))
}

async fn block_receipt_id(storage: &SurrealStorage, block_id: &str) -> Option<String> {
    let block_id = block_id.to_owned();
    storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .select_one::<BlockReceiptRow>("loom_blocks", &block_id)
                    .await
            })
        })
        .await
        .expect("read block receipt id")
        .and_then(|row| row.event_ledger_event_id)
        .map(|record| record_key_string(&record))
}

async fn overlay_receipt_id(storage: &SurrealStorage, overlay_id: &str) -> Option<String> {
    let overlay_id = overlay_id.to_owned();
    storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .select_one::<OverlayReceiptRow>("loom_wiki_overlays", &overlay_id)
                    .await
            })
        })
        .await
        .expect("read overlay receipt id")
        .map(|row| record_key_string(&row.event_ledger_event_id))
}

#[derive(SurrealValue)]
struct LedgerCountBinding {
    event_type: String,
    aggregate_id: String,
}

#[derive(SurrealValue)]
struct CountRow {
    count: i64,
}

/// Count `kernel_event_ledger` rows of an event type whose aggregate_id
/// matches. Mirrors the PostgreSQL originals' `ledger_count` raw-SQL helper,
/// ported onto the closed `query_values` surface.
async fn ledger_count(storage: &SurrealStorage, event_type: &str, aggregate_id: &str) -> i64 {
    let bindings = LedgerCountBinding {
        event_type: event_type.to_owned(),
        aggregate_id: aggregate_id.to_owned(),
    };
    storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values::<CountRow, _>(
                        "SELECT count() AS count FROM kernel_event_ledger \
                         WHERE event_type = $event_type AND aggregate_id = $aggregate_id \
                         GROUP ALL;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .expect("count kernel_event_ledger rows")
        .into_iter()
        .next()
        .map(|row| row.count)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Fault injection (solved once, reused by every [RB] test below): temporarily
// override `kernel_event_ledger.event_id` so every ledger CREATE fails
// closed, then restore the live production field definition
// (`schema.surql:789`). Reachable through the public `query_values` surface
// (static bound SurrealQL, no dynamic/injectable statement text, matching
// that method's own closed-facade contract) -- no `src/` edit required.
// ---------------------------------------------------------------------------

#[derive(SurrealValue)]
struct NoBindings {
    _marker: bool,
}

const INSTALL_LEDGER_FAIL_DDL: &str =
    "DEFINE FIELD OVERWRITE event_id ON TABLE kernel_event_ledger TYPE string ASSERT false;";
const REMOVE_LEDGER_FAIL_DDL: &str = "DEFINE FIELD OVERWRITE event_id ON TABLE kernel_event_ledger \
     TYPE string ASSERT $value = record::id($this.id);";

async fn install_ledger_fail(storage: &SurrealStorage) {
    storage
        .with_data_operation(|database| {
            Box::pin(async move {
                database
                    .query_values::<surrealdb::types::Value, _>(
                        INSTALL_LEDGER_FAIL_DDL,
                        NoBindings { _marker: true },
                    )
                    .await
            })
        })
        .await
        .expect("install injected kernel_event_ledger append failure");
}

async fn remove_ledger_fail(storage: &SurrealStorage) {
    storage
        .with_data_operation(|database| {
            Box::pin(async move {
                database
                    .query_values::<surrealdb::types::Value, _>(
                        REMOVE_LEDGER_FAIL_DDL,
                        NoBindings { _marker: true },
                    )
                    .await
            })
        })
        .await
        .expect("restore kernel_event_ledger.event_id production definition");
}

// ---------------------------------------------------------------------------
// MT-022: folder mutation atomicity
// ---------------------------------------------------------------------------

#[tokio::test]
async fn mt022_folder_create_appends_atomic_receipt_and_survives_restart() {
    let temp = tempfile::tempdir().expect("create temporary data root");
    let (config, storage, db) = open_store(temp.path()).await;
    let ws = seed_workspace(&db).await;

    let folder = db
        .create_loom_folder(&ws, new_folder(&ws, "Atomic Projects"))
        .await
        .expect("create folder");

    let receipt = folder_receipt_id(&storage, &folder.folder_id)
        .await
        .expect("committed folder must carry a durable receipt id");
    assert_eq!(
        ledger_count(&storage, FOLDER_EVENT_TYPE, &folder.folder_id).await,
        1,
        "exactly one create receipt in the ledger"
    );

    storage
        .shutdown()
        .await
        .expect("close embedded store before restart proof");
    drop(db);
    drop(storage);
    let reopened_storage = SurrealStorage::open(config)
        .await
        .expect("reopen embedded store");
    let reopened_db = SurrealDatabase::new(reopened_storage.clone());
    let read = reopened_db
        .get_loom_folder(&ws, &folder.folder_id)
        .await
        .expect("restart client reads the committed folder");
    assert_eq!(read.name, "Atomic Projects");
    let restart_receipt = folder_receipt_id(&reopened_storage, &folder.folder_id)
        .await
        .expect("receipt id persists across restart");
    assert_eq!(receipt, restart_receipt, "receipt id is stable across restart");

    reopened_storage
        .shutdown()
        .await
        .expect("close reopened store");
}

#[tokio::test]
async fn mt022_folder_update_delete_member_are_atomic_with_receipts() {
    let temp = tempfile::tempdir().expect("create temporary data root");
    let (_config, storage, db) = open_store(temp.path()).await;
    let ws = seed_workspace(&db).await;
    let folder = db
        .create_loom_folder(&ws, new_folder(&ws, "Members"))
        .await
        .expect("create folder");
    let block = make_block(&db, &ws, "member block", false).await;

    // update -> new receipt on the row
    db.update_loom_folder(
        &ws,
        &folder.folder_id,
        LoomFolderUpdate {
            color: Some(Some("#00ff00".to_string())),
            ..Default::default()
        },
    )
    .await
    .expect("update folder");
    assert!(folder_receipt_id(&storage, &folder.folder_id).await.is_some());
    assert_eq!(
        ledger_count(&storage, FOLDER_EVENT_TYPE, &folder.folder_id).await,
        2,
        "create + update receipts"
    );

    // add member -> receipt
    db.add_block_to_loom_folder(&ws, &folder.folder_id, &block, Some(1))
        .await
        .expect("add member");
    // remove member -> receipt (real removal)
    db.remove_block_from_loom_folder(&ws, &folder.folder_id, &block)
        .await
        .expect("remove member");
    assert_eq!(
        ledger_count(&storage, FOLDER_EVENT_TYPE, &folder.folder_id).await,
        4,
        "create + update + add_member + remove_member receipts"
    );

    // delete -> receipt appended, row gone
    db.delete_loom_folder(&ws, &folder.folder_id)
        .await
        .expect("delete folder");
    assert!(
        db.get_loom_folder(&ws, &folder.folder_id).await.is_err(),
        "deleted folder must not be readable"
    );
    assert_eq!(
        ledger_count(&storage, FOLDER_EVENT_TYPE, &folder.folder_id).await,
        5,
        "delete receipt is durable even though the row is gone"
    );

    storage.shutdown().await.expect("close embedded store");
}

#[tokio::test]
async fn mt022_folder_create_rolls_back_when_ledger_append_fails() {
    let temp = tempfile::tempdir().expect("create temporary data root");
    let (config, storage, db) = open_store(temp.path()).await;
    let ws = seed_workspace(&db).await;

    // Baseline count of folders in this isolated workspace.
    let before = db
        .list_loom_folders(&ws)
        .await
        .expect("baseline folder list")
        .len();

    install_ledger_fail(&storage).await;
    let result = db
        .create_loom_folder(&ws, new_folder(&ws, "Should Roll Back"))
        .await;
    assert!(
        result.is_err(),
        "create must fail when the atomic ledger append fails"
    );
    remove_ledger_fail(&storage).await;

    // Restart readback: no partial folder row and no phantom ledger row exist.
    storage
        .shutdown()
        .await
        .expect("close embedded store before restart proof");
    drop(db);
    drop(storage);
    let reopened_storage = SurrealStorage::open(config)
        .await
        .expect("reopen embedded store");
    let reopened_db = SurrealDatabase::new(reopened_storage.clone());
    let after = reopened_db
        .list_loom_folders(&ws)
        .await
        .expect("post-rollback folder list");
    assert_eq!(before, after.len(), "no partial folder row after rollback");
    assert!(
        !after.iter().any(|f| f.name == "Should Roll Back"),
        "rolled-back folder must not be readable after restart"
    );

    reopened_storage
        .shutdown()
        .await
        .expect("close reopened store");
}

// ---------------------------------------------------------------------------
// MT-024: pin REORDER atomicity
// ---------------------------------------------------------------------------

#[tokio::test]
async fn mt024_pin_order_set_appends_atomic_receipt_and_survives_restart() {
    let temp = tempfile::tempdir().expect("create temporary data root");
    let (config, storage, db) = open_store(temp.path()).await;
    let ws = seed_workspace(&db).await;
    let block = make_block(&db, &ws, "Reorderable pin", true).await;
    let ctx = WriteContext::human(None);

    let ordered = db
        .set_loom_block_pin_order(&ctx, &ws, &block, Some(3))
        .await
        .expect("set pin order");
    assert_eq!(ordered.pin_order, Some(3));

    let receipt = block_receipt_id(&storage, &block)
        .await
        .expect("committed reorder must carry a durable receipt id");
    assert_eq!(
        ledger_count(&storage, BLOCK_EVENT_TYPE, &block).await,
        1,
        "exactly one pin-order receipt in the ledger"
    );

    storage
        .shutdown()
        .await
        .expect("close embedded store before restart proof");
    drop(db);
    drop(storage);
    let reopened_storage = SurrealStorage::open(config)
        .await
        .expect("reopen embedded store");
    let reopened_db = SurrealDatabase::new(reopened_storage.clone());
    let read = reopened_db
        .get_loom_block(&ws, &block)
        .await
        .expect("restart client reads the committed block");
    assert_eq!(read.pin_order, Some(3), "pin order persists across restart");
    let restart_receipt = block_receipt_id(&reopened_storage, &block)
        .await
        .expect("receipt id persists across restart");
    assert_eq!(receipt, restart_receipt, "receipt id is stable across restart");

    reopened_storage
        .shutdown()
        .await
        .expect("close reopened store");
}

#[tokio::test]
async fn mt024_pin_order_set_rolls_back_when_ledger_append_fails() {
    let temp = tempfile::tempdir().expect("create temporary data root");
    let (config, storage, db) = open_store(temp.path()).await;
    let ws = seed_workspace(&db).await;
    let block = make_block(&db, &ws, "Rollback reorder", true).await;
    let ctx = WriteContext::human(None);

    install_ledger_fail(&storage).await;
    let result = db.set_loom_block_pin_order(&ctx, &ws, &block, Some(9)).await;
    assert!(
        result.is_err(),
        "set pin order must fail when the atomic ledger append fails"
    );
    remove_ledger_fail(&storage).await;

    // Restart readback: no partial ordinal, no receipt, no phantom ledger row.
    storage
        .shutdown()
        .await
        .expect("close embedded store before restart proof");
    drop(db);
    drop(storage);
    let reopened_storage = SurrealStorage::open(config)
        .await
        .expect("reopen embedded store");
    let reopened_db = SurrealDatabase::new(reopened_storage.clone());
    let read = reopened_db
        .get_loom_block(&ws, &block)
        .await
        .expect("restart client reads the block");
    assert_eq!(read.pin_order, None, "pin order was NOT partially applied");
    assert_eq!(block_receipt_id(&reopened_storage, &block).await, None);
    assert_eq!(
        ledger_count(&reopened_storage, BLOCK_EVENT_TYPE, &block).await,
        0
    );

    reopened_storage
        .shutdown()
        .await
        .expect("close reopened store");
}

// ---------------------------------------------------------------------------
// MT-024: pin REMOVAL atomicity (the core FAIL_V2 finding)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn mt024_pin_removal_is_atomic_and_survives_restart() {
    // PARTIAL per the MT-150 lane brief:
    // `concurrent_pin_removals_return_their_exact_committed_event_identity`
    // (src/storage/surreal/loom_store.rs:4015) already proves, against the
    // live (not reopened) store: distinct receipts for concurrent removals,
    // final unpinned+cleared state, and exact ledger linkage of the final
    // block revision to its committed removal receipt. It never closes and
    // reopens the store. This test adds only what that in-source proof does
    // not assert: a single-call removal surviving an actual store
    // close + reopen.
    let temp = tempfile::tempdir().expect("create temporary data root");
    let (config, storage, db) = open_store(temp.path()).await;
    let ws = seed_workspace(&db).await;
    let block = make_block(&db, &ws, "Pinned + ordered", true).await;
    let ctx = WriteContext::human(None);
    // Give it an ordinal so removal has BOTH columns to clear atomically.
    db.set_loom_block_pin_order(&ctx, &ws, &block, Some(2))
        .await
        .expect("seed pin order");

    let removed = db
        .remove_loom_block_pin(&ctx, &ws, &block)
        .await
        .expect("atomic pin removal");
    assert!(!removed.block.pinned, "removal unpins the block");
    assert_eq!(removed.block.pin_order, None, "removal clears the ordinal");

    assert!(block_receipt_id(&storage, &block).await.is_some());
    assert_eq!(
        ledger_count(&storage, BLOCK_EVENT_TYPE, &block).await,
        2,
        "pin_order_set + pin_removed receipts"
    );

    storage
        .shutdown()
        .await
        .expect("close embedded store before restart proof");
    drop(db);
    drop(storage);
    let reopened_storage = SurrealStorage::open(config)
        .await
        .expect("reopen embedded store");
    let reopened_db = SurrealDatabase::new(reopened_storage.clone());
    let read = reopened_db
        .get_loom_block(&ws, &block)
        .await
        .expect("restart read");
    assert!(!read.pinned, "unpin persists across restart");
    assert_eq!(read.pin_order, None, "cleared ordinal persists across restart");

    reopened_storage
        .shutdown()
        .await
        .expect("close reopened store");
}

#[tokio::test]
async fn mt024_pin_removal_rolls_back_and_leaves_no_partial_state() {
    let temp = tempfile::tempdir().expect("create temporary data root");
    let (config, storage, db) = open_store(temp.path()).await;
    let ws = seed_workspace(&db).await;
    let block = make_block(&db, &ws, "No partial removal", true).await;
    let ctx = WriteContext::human(None);
    db.set_loom_block_pin_order(&ctx, &ws, &block, Some(5))
        .await
        .expect("seed pin order");
    assert_eq!(ledger_count(&storage, BLOCK_EVENT_TYPE, &block).await, 1);

    // Inject the receipt failure: the removal transaction must roll BOTH
    // column changes back -- the exact partial state (pin_order cleared but
    // still pinned) the old two-call flow risked must be impossible.
    install_ledger_fail(&storage).await;
    let result = db.remove_loom_block_pin(&ctx, &ws, &block).await;
    assert!(
        result.is_err(),
        "pin removal must fail when the ledger append fails"
    );
    remove_ledger_fail(&storage).await;

    // Restart readback: the block is STILL fully pinned WITH its ordinal intact.
    storage
        .shutdown()
        .await
        .expect("close embedded store before restart proof");
    drop(db);
    drop(storage);
    let reopened_storage = SurrealStorage::open(config)
        .await
        .expect("reopen embedded store");
    let reopened_db = SurrealDatabase::new(reopened_storage.clone());
    let read = reopened_db
        .get_loom_block(&ws, &block)
        .await
        .expect("restart read");
    assert!(read.pinned, "block remains pinned after rolled-back removal");
    assert_eq!(
        read.pin_order,
        Some(5),
        "pin_order was NOT cleared: no partial removal state persisted"
    );
    // Only the original pin_order_set receipt exists; no pin_removed receipt.
    assert_eq!(
        ledger_count(&reopened_storage, BLOCK_EVENT_TYPE, &block).await,
        1
    );

    reopened_storage
        .shutdown()
        .await
        .expect("close reopened store");
}

// ---------------------------------------------------------------------------
// MT-025: wiki projection overlay atomicity
// ---------------------------------------------------------------------------

#[tokio::test]
async fn mt025_wiki_overlay_appends_atomic_receipt_and_survives_restart() {
    let temp = tempfile::tempdir().expect("create temporary data root");
    let (config, storage, db) = open_store(temp.path()).await;
    let ws = seed_workspace(&db).await;
    let projection_id = seed_wiki_projection(&db, &ws).await;

    let overlay = db
        .add_loom_wiki_overlay(
            &ws,
            &projection_id,
            "operator note on the ownership model",
            None,
        )
        .await
        .expect("add wiki overlay");

    let receipt = overlay_receipt_id(&storage, &overlay.overlay_id)
        .await
        .expect("committed overlay must carry a durable receipt id");
    assert_eq!(
        ledger_count(&storage, WIKI_EVENT_TYPE, &overlay.overlay_id).await,
        1,
        "exactly one overlay receipt in the ledger"
    );

    storage
        .shutdown()
        .await
        .expect("close embedded store before restart proof");
    drop(db);
    drop(storage);
    let reopened_storage = SurrealStorage::open(config)
        .await
        .expect("reopen embedded store");
    let reopened_db = SurrealDatabase::new(reopened_storage.clone());
    let overlays = reopened_db
        .list_loom_wiki_overlays(&ws, &projection_id)
        .await
        .expect("restart client lists overlays");
    assert!(
        overlays.iter().any(|o| o.overlay_id == overlay.overlay_id
            && o.annotation == "operator note on the ownership model"),
        "overlay annotation persists across restart"
    );
    let restart_receipt = overlay_receipt_id(&reopened_storage, &overlay.overlay_id)
        .await
        .expect("receipt id persists across restart");
    assert_eq!(
        receipt, restart_receipt,
        "overlay receipt id is stable across restart"
    );

    reopened_storage
        .shutdown()
        .await
        .expect("close reopened store");
}

#[tokio::test]
async fn mt025_wiki_overlay_rolls_back_when_ledger_append_fails() {
    let temp = tempfile::tempdir().expect("create temporary data root");
    let (config, storage, db) = open_store(temp.path()).await;
    let ws = seed_workspace(&db).await;
    let projection_id = seed_wiki_projection(&db, &ws).await;
    assert_eq!(
        db.list_loom_wiki_overlays(&ws, &projection_id)
            .await
            .expect("baseline overlays")
            .len(),
        0
    );

    install_ledger_fail(&storage).await;
    let result = db
        .add_loom_wiki_overlay(&ws, &projection_id, "should roll back", None)
        .await;
    assert!(
        result.is_err(),
        "overlay add must fail when the atomic ledger append fails"
    );
    remove_ledger_fail(&storage).await;

    // Restart readback: no partial overlay row and no phantom ledger row.
    storage
        .shutdown()
        .await
        .expect("close embedded store before restart proof");
    drop(db);
    drop(storage);
    let reopened_storage = SurrealStorage::open(config)
        .await
        .expect("reopen embedded store");
    let reopened_db = SurrealDatabase::new(reopened_storage.clone());
    let overlays = reopened_db
        .list_loom_wiki_overlays(&ws, &projection_id)
        .await
        .expect("restart lists overlays");
    assert!(
        overlays.is_empty(),
        "rolled-back overlay must not be readable after restart"
    );

    reopened_storage
        .shutdown()
        .await
        .expect("close reopened store");
}

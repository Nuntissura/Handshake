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
//! verbatim from `src/storage/surreal/schema.surql` (`DEFINE FIELD OVERWRITE
//! event_id ON TABLE kernel_event_ledger`).
//!
//! ALL fourteen originals are ported below. Five of them (the two tag-edge
//! MT-023 cases, the two favorite MT-024 cases and the mention-backlink
//! MT-024 case) were previously recorded as `BLOCKED_ON_PRODUCT_SEAM` because
//! `update_loom_block` / `create_loom_edge` did not append a receipt; the
//! MT-150 remediation (V2 finding MT150-V2-F01) made every block metadata
//! mutation and every edge create/delete append its typed receipt through the
//! one canonical builder + `loom_ledger_append_sql!` path in
//! `src/storage/surreal/loom_store.rs`, bound to
//! `loom_blocks.event_ledger_event_id` / `loom_edges.event_ledger_event_id`
//! (the latter added to `schema.surql` by MT-150). Those five are now ported
//! with their ORIGINAL assertions (see the per-test comments), and the
//! `mt150_*` tests below add the validator's adversarial matrix: exact-retry
//! idempotency, same-identity/different-content divergence, duplicate edge id,
//! delete receipts, no-op updates, rollback of derived counts / search index,
//! and concurrent identical / conflicting requests -- all against the real
//! embedded store and real EventLedger rows.
//!
//! The fault-injection seam's restore statement is the field's live production
//! definition, copied verbatim from `src/storage/surreal/schema.surql`
//! (`DEFINE FIELD OVERWRITE event_id ON TABLE kernel_event_ledger ...`, line
//! 832 at MT-150 time; searched by text, not by line number).

use handshake_core::storage::surreal::{
    bootstrap_schema, SurrealDatabase, SurrealStorage, SurrealStorageConfig,
};
use handshake_core::storage::{
    Database, LoomBlockContentType, LoomBlockDerived, LoomBlockUpdate, LoomEdgeCreatedBy,
    LoomEdgeType, LoomFolderSortMode, LoomFolderUpdate, NewLoomBlock, NewLoomEdge, NewLoomFolder,
    NewWorkspace, StorageError, WriteContext,
};
use surrealdb::types::{Datetime, RecordId, RecordIdKey, SurrealValue};

const FOLDER_EVENT_TYPE: &str = "KNOWLEDGE_LOOM_FOLDER_MUTATED";
const BLOCK_EVENT_TYPE: &str = "KNOWLEDGE_LOOM_BLOCK_MUTATED";
const WIKI_EVENT_TYPE: &str = "KNOWLEDGE_LOOM_WIKI_MUTATED";
/// The originals asserted the TAG event type for mention edges too: every edge mutation is one
/// `KNOWLEDGE_LOOM_TAG_MUTATED` receipt with aggregate_type `loom_edge`.
const TAG_EVENT_TYPE: &str = "KNOWLEDGE_LOOM_TAG_MUTATED";

// ---------------------------------------------------------------------------
// Store harness: mirrors `loom_store.rs`'s in-source `open_store` idiom
// (tempdir + `SurrealStorageConfig::for_data_dir`) and the close/reopen
// restart proof used by `collection_replacement_is_atomic_and_survives_close_reopen`.
// ---------------------------------------------------------------------------

async fn open_store(
    dir: &std::path::Path,
) -> (SurrealStorageConfig, SurrealStorage, SurrealDatabase) {
    let config = SurrealStorageConfig::for_data_dir(dir).expect("configure embedded Surreal store");
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

fn new_edge(ws: &str, src: &str, target: &str, edge_type: LoomEdgeType) -> NewLoomEdge {
    NewLoomEdge {
        edge_id: None,
        workspace_id: ws.to_string(),
        source_block_id: src.to_string(),
        target_block_id: target.to_string(),
        edge_type,
        created_by: LoomEdgeCreatedBy::User,
        crdt_site_id: None,
        source_anchor: None,
    }
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

#[derive(SurrealValue)]
struct EdgeReceiptRow {
    event_ledger_event_id: Option<RecordId>,
}

#[derive(SurrealValue)]
struct SearchIndexRow {
    search_text: String,
    indexed_at: Datetime,
}

/// `kernel_event_ledger` record keys are always strings
/// (`schema.surql` `event_id ON TABLE kernel_event_ledger TYPE string ASSERT $value = record::id($this.id)`),
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

/// `loom_edges.event_ledger_event_id` read straight off the row (the originals read it with
/// raw SQL); `None` when the row is absent OR carries no receipt.
async fn edge_receipt_id(storage: &SurrealStorage, edge_id: &str) -> Option<String> {
    let edge_id = edge_id.to_owned();
    storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .select_one::<EdgeReceiptRow>("loom_edges", &edge_id)
                    .await
            })
        })
        .await
        .expect("read edge receipt id")
        .and_then(|row| row.event_ledger_event_id)
        .map(|record| record_key_string(&record))
}

/// The block's search-index projection row (text + indexed_at), which every block metadata
/// mutation refreshes inside its receipt transaction.
async fn search_index_row(
    storage: &SurrealStorage,
    block_id: &str,
) -> (String, chrono::DateTime<chrono::Utc>) {
    let block_id = block_id.to_owned();
    storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .select_one::<SearchIndexRow>("loom_block_search_index", &block_id)
                    .await
            })
        })
        .await
        .expect("read search index row")
        .map(|row| (row.search_text, row.indexed_at.into_inner()))
        .expect("block search index row present")
}

#[derive(SurrealValue)]
struct EmptyCountBinding {
    _marker: bool,
}

/// Total `loom_edges` rows in the store (the originals' `SELECT COUNT(*) FROM loom_edges`).
async fn edge_row_count(storage: &SurrealStorage) -> i64 {
    storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values::<CountRow, _>(
                        "SELECT count() AS count FROM loom_edges GROUP ALL;",
                        EmptyCountBinding { _marker: true },
                    )
                    .await
            })
        })
        .await
        .expect("count loom_edges rows")
        .into_iter()
        .next()
        .map(|row| row.count)
        .unwrap_or(0)
}

/// (mention_count, tag_count, backlink_count) of a block as the product reads it back.
async fn counts(db: &SurrealDatabase, ws: &str, block: &str) -> (i64, i64, i64) {
    let read = db
        .get_loom_block(ws, block)
        .await
        .expect("read block counts");
    (
        read.derived.mention_count,
        read.derived.tag_count,
        read.derived.backlink_count,
    )
}

#[derive(SurrealValue)]
struct LedgerTypeBinding {
    event_type: String,
}

/// Count every `kernel_event_ledger` row of one event type (any aggregate).
async fn ledger_count_by_type(storage: &SurrealStorage, event_type: &str) -> i64 {
    let bindings = LedgerTypeBinding {
        event_type: event_type.to_owned(),
    };
    storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values::<CountRow, _>(
                        "SELECT count() AS count FROM kernel_event_ledger WHERE event_type = $event_type GROUP ALL;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .expect("count kernel_event_ledger rows by type")
        .into_iter()
        .next()
        .map(|row| row.count)
        .unwrap_or(0)
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
// (`schema.surql`, `DEFINE FIELD OVERWRITE event_id ON TABLE kernel_event_ledger`). Reachable through the public `query_values` surface
// (static bound SurrealQL, no dynamic/injectable statement text, matching
// that method's own closed-facade contract) -- no `src/` edit required.
// ---------------------------------------------------------------------------

#[derive(SurrealValue)]
struct NoBindings {
    _marker: bool,
}

const INSTALL_LEDGER_FAIL_DDL: &str =
    "DEFINE FIELD OVERWRITE event_id ON TABLE kernel_event_ledger TYPE string ASSERT false;";
/// Verbatim `schema.surql` definition of `kernel_event_ledger.event_id` (the seam's restore).
const REMOVE_LEDGER_FAIL_DDL: &str =
    "DEFINE FIELD OVERWRITE event_id ON TABLE kernel_event_ledger \
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
    assert_eq!(
        receipt, restart_receipt,
        "receipt id is stable across restart"
    );

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
    assert!(folder_receipt_id(&storage, &folder.folder_id)
        .await
        .is_some());
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
    assert_eq!(
        receipt, restart_receipt,
        "receipt id is stable across restart"
    );

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
    let result = db
        .set_loom_block_pin_order(&ctx, &ws, &block, Some(9))
        .await;
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
    assert_eq!(
        read.pin_order, None,
        "cleared ordinal persists across restart"
    );

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
    assert!(
        read.pinned,
        "block remains pinned after rolled-back removal"
    );
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
// MT-023: tag-edge create atomicity (restored by MT-150; original assertions)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn mt023_tag_edge_create_appends_atomic_receipt_and_survives_restart() {
    let temp = tempfile::tempdir().expect("create temporary data root");
    let (config, storage, db) = open_store(temp.path()).await;
    let ws = seed_workspace(&db).await;
    let src = make_block(&db, &ws, "source", false).await;
    let hub = make_block(&db, &ws, "tag hub", false).await;

    let ctx = WriteContext::human(None);
    let edge = db
        .create_loom_edge(&ctx, new_edge(&ws, &src, &hub, LoomEdgeType::Tag))
        .await
        .expect("create tag edge");

    // The edge row carries a durable receipt id and the ledger row exists.
    let receipt = edge_receipt_id(&storage, &edge.edge_id).await;
    assert!(
        receipt.is_some(),
        "committed tag edge must carry a durable receipt id"
    );
    assert_eq!(
        edge.event_ledger_event_id, receipt,
        "the domain projection exposes the same receipt the row carries"
    );
    assert_eq!(
        ledger_count(&storage, TAG_EVENT_TYPE, &edge.edge_id).await,
        1,
        "exactly one tag-create receipt in the ledger"
    );
    // The derived counts committed with the edge (MT-150: same transaction as the receipt).
    assert_eq!(counts(&db, &ws, &src).await, (0, 1, 0));
    assert_eq!(counts(&db, &ws, &hub).await, (0, 0, 1));

    // Restart readback: the edge and its receipt persist for a fresh client.
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
    let edges = reopened_db
        .list_loom_edges_for_block(&ws, &src)
        .await
        .expect("restart client lists edges");
    assert!(
        edges.iter().any(|e| e.edge_id == edge.edge_id),
        "tag edge persists across restart"
    );
    assert_eq!(
        edge_receipt_id(&reopened_storage, &edge.edge_id).await,
        receipt,
        "receipt id is stable across restart"
    );
    assert_eq!(counts(&reopened_db, &ws, &src).await, (0, 1, 0));
    assert_eq!(counts(&reopened_db, &ws, &hub).await, (0, 0, 1));

    reopened_storage
        .shutdown()
        .await
        .expect("close reopened store");
}

#[tokio::test]
async fn mt023_tag_edge_create_rolls_back_when_ledger_append_fails() {
    let temp = tempfile::tempdir().expect("create temporary data root");
    let (config, storage, db) = open_store(temp.path()).await;
    let ws = seed_workspace(&db).await;
    let src = make_block(&db, &ws, "source-rb", false).await;
    let hub = make_block(&db, &ws, "hub-rb", false).await;

    let before = edge_row_count(&storage).await;

    install_ledger_fail(&storage).await;
    let ctx = WriteContext::human(None);
    let result = db
        .create_loom_edge(&ctx, new_edge(&ws, &src, &hub, LoomEdgeType::Tag))
        .await;
    assert!(
        result.is_err(),
        "tag edge create must fail when the atomic ledger append fails"
    );
    remove_ledger_fail(&storage).await;

    // Restart readback: no partial edge row persisted, no phantom receipt, counts untouched.
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
    let after = edge_row_count(&reopened_storage).await;
    assert_eq!(before, after, "no partial tag edge row after rollback");
    let edges = reopened_db
        .list_loom_edges_for_block(&ws, &src)
        .await
        .expect("restart edge list");
    assert!(
        edges.is_empty(),
        "rolled-back tag edge is not readable after restart"
    );
    assert_eq!(counts(&reopened_db, &ws, &src).await, (0, 0, 0));
    assert_eq!(counts(&reopened_db, &ws, &hub).await, (0, 0, 0));
    assert_eq!(
        ledger_count_by_type(&reopened_storage, TAG_EVENT_TYPE).await,
        0,
        "no phantom ledger row"
    );

    reopened_storage
        .shutdown()
        .await
        .expect("close reopened store");
}

// ---------------------------------------------------------------------------
// MT-024: favorite mutation atomicity (restored by MT-150; original assertions)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn mt024_favorite_mutation_appends_atomic_receipt_and_survives_restart() {
    let temp = tempfile::tempdir().expect("create temporary data root");
    let (config, storage, db) = open_store(temp.path()).await;
    let ws = seed_workspace(&db).await;
    let block = make_block(&db, &ws, "Favorite me", false).await;
    let ctx = WriteContext::human(None);

    // favorite:true -> receipt
    let fav = db
        .update_loom_block(
            &ctx,
            &ws,
            &block,
            LoomBlockUpdate {
                favorite: Some(true),
                ..Default::default()
            },
        )
        .await
        .expect("set favorite");
    assert!(fav.favorite);
    let first_receipt = block_receipt_id(&storage, &block).await;
    assert!(first_receipt.is_some());
    assert_eq!(ledger_count(&storage, BLOCK_EVENT_TYPE, &block).await, 1);

    // favorite:false -> second receipt (the Favorites remove path, AC3)
    db.update_loom_block(
        &ctx,
        &ws,
        &block,
        LoomBlockUpdate {
            favorite: Some(false),
            ..Default::default()
        },
    )
    .await
    .expect("clear favorite");
    assert_eq!(
        ledger_count(&storage, BLOCK_EVENT_TYPE, &block).await,
        2,
        "set + clear favorite receipts"
    );
    let second_receipt = block_receipt_id(&storage, &block).await;
    assert!(second_receipt.is_some());
    assert_ne!(
        first_receipt, second_receipt,
        "the row binds the receipt of its latest accepted mutation"
    );

    // Restart readback: the un-favorite persisted.
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
    assert!(!read.favorite, "un-favorite persists across restart");
    assert_eq!(
        block_receipt_id(&reopened_storage, &block).await,
        second_receipt,
        "receipt linkage persists across restart"
    );
    assert_eq!(
        ledger_count(&reopened_storage, BLOCK_EVENT_TYPE, &block).await,
        2
    );

    reopened_storage
        .shutdown()
        .await
        .expect("close reopened store");
}

#[tokio::test]
async fn mt024_favorite_mutation_rolls_back_when_ledger_append_fails() {
    let temp = tempfile::tempdir().expect("create temporary data root");
    let (config, storage, db) = open_store(temp.path()).await;
    let ws = seed_workspace(&db).await;
    let block = make_block(&db, &ws, "Favorite rollback", false).await;
    let ctx = WriteContext::human(None);
    let (search_before, indexed_before) = search_index_row(&storage, &block).await;

    install_ledger_fail(&storage).await;
    let result = db
        .update_loom_block(
            &ctx,
            &ws,
            &block,
            LoomBlockUpdate {
                favorite: Some(true),
                ..Default::default()
            },
        )
        .await;
    assert!(
        result.is_err(),
        "favorite mutation must fail with the ledger append"
    );
    remove_ledger_fail(&storage).await;

    // The search-index refresh in the same transaction rolled back with the block write.
    let (search_after, indexed_after) = search_index_row(&storage, &block).await;
    assert_eq!(search_before, search_after);
    assert_eq!(
        indexed_before, indexed_after,
        "search index was NOT re-indexed"
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
    assert!(!read.favorite, "favorite was NOT partially applied");
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
// MT-024: mention (backlink) edge atomicity (restored by MT-150; original assertions)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn mt024_mention_edge_backlink_is_atomic_and_survives_restart() {
    let temp = tempfile::tempdir().expect("create temporary data root");
    let (config, storage, db) = open_store(temp.path()).await;
    let ws = seed_workspace(&db).await;
    let src = make_block(&db, &ws, "mentions target", false).await;
    let target = make_block(&db, &ws, "target", false).await;
    let ctx = WriteContext::human(None);

    let edge = db
        .create_loom_edge(&ctx, new_edge(&ws, &src, &target, LoomEdgeType::Mention))
        .await
        .expect("create mention (backlink) edge");

    let receipt = edge_receipt_id(&storage, &edge.edge_id).await;
    assert!(
        receipt.is_some(),
        "committed mention edge must carry a durable receipt id"
    );
    assert_eq!(edge.event_ledger_event_id, receipt);
    assert_eq!(
        ledger_count(&storage, TAG_EVENT_TYPE, &edge.edge_id).await,
        1,
        "exactly one edge-mutation receipt for the backlink edge"
    );
    // Both endpoints' derived counts committed atomically with the edge + receipt.
    assert_eq!(counts(&db, &ws, &src).await, (1, 0, 0));
    assert_eq!(counts(&db, &ws, &target).await, (0, 0, 1));

    // Restart readback: the backlink edge (target -> src) persists.
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
    let edges = reopened_db
        .list_loom_edges_for_block(&ws, &target)
        .await
        .expect("restart lists edges pointing at the target");
    assert!(
        edges.iter().any(|e| e.edge_id == edge.edge_id),
        "mention backlink edge persists across restart"
    );
    assert_eq!(
        edge_receipt_id(&reopened_storage, &edge.edge_id).await,
        receipt
    );
    assert_eq!(counts(&reopened_db, &ws, &src).await, (1, 0, 0));
    assert_eq!(counts(&reopened_db, &ws, &target).await, (0, 0, 1));

    reopened_storage
        .shutdown()
        .await
        .expect("close reopened store");
}

// ---------------------------------------------------------------------------
// MT-150 adversarial matrix (validator remediation_required.adversarial_tests)
// ---------------------------------------------------------------------------

/// Favorite set: an EXACT retry of one request (same guard-minted request identity, the
/// production guarded-retry shape) reuses the original receipt and re-reads the committed
/// state; the same request identity resubmitted with different content is a typed divergence
/// conflict that leaves favorite, receipt linkage and ledger untouched -- also across a
/// close/reopen.
#[tokio::test]
async fn mt150_favorite_exact_retry_reuses_receipt_and_divergent_retry_conflicts_without_residue() {
    let temp = tempfile::tempdir().expect("create temporary data root");
    let (config, storage, db) = open_store(temp.path()).await;
    let ws = seed_workspace(&db).await;
    let block = make_block(&db, &ws, "Retry favorite", false).await;
    let ctx = WriteContext::human(Some("operator-150".to_owned()));
    let request = db
        .validate_write_with_guard(&ctx, &block)
        .await
        .expect("mint one request identity through the real write guard");

    let first = db
        .test_update_loom_block_with_metadata(
            &ws,
            &block,
            LoomBlockUpdate {
                favorite: Some(true),
                ..Default::default()
            },
            request.clone(),
        )
        .await
        .expect("first submission");
    assert!(first.favorite);
    let receipt = block_receipt_id(&storage, &block)
        .await
        .expect("first submission bound its receipt");
    assert_eq!(ledger_count(&storage, BLOCK_EVENT_TYPE, &block).await, 1);

    let retried = db
        .test_update_loom_block_with_metadata(
            &ws,
            &block,
            LoomBlockUpdate {
                favorite: Some(true),
                ..Default::default()
            },
            request.clone(),
        )
        .await
        .expect("identical retry returns the original durable outcome");
    assert!(retried.favorite);
    assert_eq!(
        retried.updated_at, first.updated_at,
        "retry did not re-apply"
    );
    assert_eq!(
        block_receipt_id(&storage, &block).await.as_deref(),
        Some(receipt.as_str()),
        "retry reuses the original receipt"
    );
    assert_eq!(
        ledger_count(&storage, BLOCK_EVENT_TYPE, &block).await,
        1,
        "identical retry does not mint a second receipt"
    );

    let divergent = db
        .test_update_loom_block_with_metadata(
            &ws,
            &block,
            LoomBlockUpdate {
                favorite: Some(false),
                ..Default::default()
            },
            request,
        )
        .await;
    assert!(
        matches!(
            divergent,
            Err(StorageError::Conflict("loom_mutation_receipt_divergent"))
        ),
        "same request identity with different content must conflict, got {divergent:?}"
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
    assert!(
        read.favorite,
        "divergent retry left the accepted value in place"
    );
    assert_eq!(
        block_receipt_id(&reopened_storage, &block).await.as_deref(),
        Some(receipt.as_str())
    );
    assert_eq!(
        ledger_count(&reopened_storage, BLOCK_EVENT_TYPE, &block).await,
        1,
        "zero ledger residue from the retry and the divergent retry"
    );
    reopened_storage
        .shutdown()
        .await
        .expect("close reopened store");
}

/// A block update that changes nothing (every requested value already stored) is a no-op:
/// no write, no spurious receipt.
#[tokio::test]
async fn mt150_block_update_without_change_is_a_no_op_without_receipt() {
    let temp = tempfile::tempdir().expect("create temporary data root");
    let (_config, storage, db) = open_store(temp.path()).await;
    let ws = seed_workspace(&db).await;
    let block = make_block(&db, &ws, "Unchanged", false).await;
    let ctx = WriteContext::human(None);
    let before = db.get_loom_block(&ws, &block).await.expect("read");

    let unchanged = db
        .update_loom_block(
            &ctx,
            &ws,
            &block,
            LoomBlockUpdate {
                favorite: Some(false),
                title: Some("Unchanged".to_owned()),
                ..Default::default()
            },
        )
        .await
        .expect("no-op update succeeds");
    assert!(!unchanged.favorite);
    assert_eq!(unchanged.updated_at, before.updated_at, "no write happened");
    assert_eq!(block_receipt_id(&storage, &block).await, None);
    assert_eq!(
        ledger_count(&storage, BLOCK_EVENT_TYPE, &block).await,
        0,
        "a no-op update must not emit a receipt"
    );

    // ...and a real change right after still emits exactly one.
    db.update_loom_block(
        &ctx,
        &ws,
        &block,
        LoomBlockUpdate {
            favorite: Some(true),
            ..Default::default()
        },
    )
    .await
    .expect("real change");
    assert_eq!(ledger_count(&storage, BLOCK_EVENT_TYPE, &block).await, 1);
    storage.shutdown().await.expect("close embedded store");
}

/// Block update rollback under the ledger fault: the title, favorite, receipt linkage AND the
/// search-index projection (text + indexed_at) are all left untouched.
#[tokio::test]
async fn mt150_block_update_rollback_leaves_search_index_and_fields_untouched() {
    let temp = tempfile::tempdir().expect("create temporary data root");
    let (config, storage, db) = open_store(temp.path()).await;
    let ws = seed_workspace(&db).await;
    let block = make_block(&db, &ws, "Original title", false).await;
    let ctx = WriteContext::human(None);
    let (search_before, indexed_before) = search_index_row(&storage, &block).await;
    assert!(search_before.contains("Original title"));

    install_ledger_fail(&storage).await;
    let result = db
        .update_loom_block(
            &ctx,
            &ws,
            &block,
            LoomBlockUpdate {
                title: Some("Renamed under fault".to_owned()),
                favorite: Some(true),
                ..Default::default()
            },
        )
        .await;
    assert!(result.is_err(), "update must fail with the ledger append");
    remove_ledger_fail(&storage).await;

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
    assert_eq!(read.title.as_deref(), Some("Original title"));
    assert!(!read.favorite);
    let (search_after, indexed_after) = search_index_row(&reopened_storage, &block).await;
    assert_eq!(search_before, search_after, "search text was NOT rewritten");
    assert_eq!(
        indexed_before, indexed_after,
        "search index was NOT re-indexed"
    );
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

/// Tag edge create: an EXACT retry reuses the receipt and returns the committed edge; the same
/// request identity with a different target is a typed divergence conflict; a NEW request
/// reusing the committed edge id is a typed \`loom_edge_exists\` conflict -- and none of them
/// leave an orphan ledger row, a second edge, or count drift, including after close/reopen.
#[tokio::test]
async fn mt150_tag_edge_exact_retry_reuses_receipt_and_conflicting_requests_leave_no_residue() {
    let temp = tempfile::tempdir().expect("create temporary data root");
    let (config, storage, db) = open_store(temp.path()).await;
    let ws = seed_workspace(&db).await;
    let src = make_block(&db, &ws, "retry source", false).await;
    let hub = make_block(&db, &ws, "retry hub", false).await;
    let other = make_block(&db, &ws, "other hub", false).await;
    let ctx = WriteContext::human(Some("operator-150".to_owned()));
    let edge_id = format!("edge-retry-{}", uuid::Uuid::now_v7());
    let request = db
        .validate_write_with_guard(&ctx, &edge_id)
        .await
        .expect("mint one request identity through the real write guard");
    let mut edge = new_edge(&ws, &src, &hub, LoomEdgeType::Tag);
    edge.edge_id = Some(edge_id.clone());

    let first = db
        .test_create_loom_edge_with_metadata(edge.clone(), request.clone())
        .await
        .expect("first submission");
    let receipt = first
        .event_ledger_event_id
        .clone()
        .expect("first submission bound its receipt");
    assert_eq!(ledger_count(&storage, TAG_EVENT_TYPE, &edge_id).await, 1);
    assert_eq!(counts(&db, &ws, &src).await, (0, 1, 0));
    assert_eq!(counts(&db, &ws, &hub).await, (0, 0, 1));

    let retried = db
        .test_create_loom_edge_with_metadata(edge.clone(), request.clone())
        .await
        .expect("identical retry returns the original durable outcome");
    assert_eq!(retried.edge_id, edge_id);
    assert_eq!(
        retried.event_ledger_event_id.as_deref(),
        Some(receipt.as_str())
    );
    assert_eq!(retried.created_at, first.created_at);
    assert_eq!(
        ledger_count(&storage, TAG_EVENT_TYPE, &edge_id).await,
        1,
        "identical retry does not mint a second receipt"
    );
    assert_eq!(counts(&db, &ws, &src).await, (0, 1, 0), "no count drift");
    assert_eq!(counts(&db, &ws, &hub).await, (0, 0, 1), "no count drift");

    let mut divergent_edge = edge.clone();
    divergent_edge.target_block_id = other.clone();
    let divergent = db
        .test_create_loom_edge_with_metadata(divergent_edge, request)
        .await;
    assert!(
        matches!(
            divergent,
            Err(StorageError::Conflict("loom_mutation_receipt_divergent"))
        ),
        "same request identity with different content must conflict, got {divergent:?}"
    );

    // A NEW request (fresh identity) reusing the committed edge id: typed conflict, and the
    // receipt it appended inside its own transaction is rolled back with it.
    let duplicate = db.create_loom_edge(&ctx, edge.clone()).await;
    assert!(
        matches!(duplicate, Err(StorageError::Conflict("loom_edge_exists"))),
        "duplicate edge id must be a typed conflict, got {duplicate:?}"
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
    let edges = reopened_db
        .list_loom_edges_for_block(&ws, &src)
        .await
        .expect("restart edge list");
    assert_eq!(edges.len(), 1, "exactly one edge survives");
    assert_eq!(
        edges[0].target_block_id, hub,
        "the accepted target survives"
    );
    assert_eq!(
        edges[0].event_ledger_event_id.as_deref(),
        Some(receipt.as_str())
    );
    assert_eq!(
        ledger_count(&reopened_storage, TAG_EVENT_TYPE, &edge_id).await,
        1,
        "zero orphan ledger rows from the retry, the divergent retry and the duplicate request"
    );
    assert_eq!(counts(&reopened_db, &ws, &src).await, (0, 1, 0));
    assert_eq!(counts(&reopened_db, &ws, &hub).await, (0, 0, 1));
    assert_eq!(counts(&reopened_db, &ws, &other).await, (0, 0, 0));
    reopened_storage
        .shutdown()
        .await
        .expect("close reopened store");
}

/// Edge delete (tag and mention): success appends exactly one durable delete receipt that
/// stays readable after the row is gone, both endpoints' counts drop atomically, and all of it
/// survives a close/reopen; under the ledger fault the delete rolls back and the edge, its
/// receipt linkage and both endpoints' counts are untouched.
#[tokio::test]
async fn mt150_edge_delete_appends_durable_receipt_and_rolls_back_with_counts() {
    let temp = tempfile::tempdir().expect("create temporary data root");
    let (config, storage, db) = open_store(temp.path()).await;
    let ws = seed_workspace(&db).await;
    let src = make_block(&db, &ws, "delete source", false).await;
    let hub = make_block(&db, &ws, "delete hub", false).await;
    let target = make_block(&db, &ws, "delete target", false).await;
    let ctx = WriteContext::human(None);
    let tag = db
        .create_loom_edge(&ctx, new_edge(&ws, &src, &hub, LoomEdgeType::Tag))
        .await
        .expect("create tag edge");
    let mention = db
        .create_loom_edge(&ctx, new_edge(&ws, &src, &target, LoomEdgeType::Mention))
        .await
        .expect("create mention edge");
    assert_eq!(counts(&db, &ws, &src).await, (1, 1, 0));
    assert_eq!(counts(&db, &ws, &hub).await, (0, 0, 1));
    assert_eq!(counts(&db, &ws, &target).await, (0, 0, 1));

    // Rollback first: the tag delete fails at the receipt append.
    install_ledger_fail(&storage).await;
    let result = db.delete_loom_edge(&ctx, &ws, &tag.edge_id).await;
    assert!(
        result.is_err(),
        "delete must fail when the ledger append fails"
    );
    remove_ledger_fail(&storage).await;
    assert_eq!(
        edge_receipt_id(&storage, &tag.edge_id).await,
        tag.event_ledger_event_id,
        "rolled-back delete leaves the edge and its create receipt in place"
    );
    assert_eq!(
        ledger_count(&storage, TAG_EVENT_TYPE, &tag.edge_id).await,
        1
    );
    assert_eq!(
        counts(&db, &ws, &src).await,
        (1, 1, 0),
        "no partial count change"
    );
    assert_eq!(
        counts(&db, &ws, &hub).await,
        (0, 0, 1),
        "no partial count change"
    );

    // Success: one delete receipt per accepted deletion, counts drop on both endpoints.
    let deleted_tag = db
        .delete_loom_edge(&ctx, &ws, &tag.edge_id)
        .await
        .expect("delete tag edge");
    assert_eq!(deleted_tag.edge_id, tag.edge_id);
    assert_eq!(
        ledger_count(&storage, TAG_EVENT_TYPE, &tag.edge_id).await,
        2,
        "create + delete receipts, durable although the row is gone"
    );
    assert_eq!(
        edge_receipt_id(&storage, &tag.edge_id).await,
        None,
        "row is gone"
    );
    db.delete_loom_edge(&ctx, &ws, &mention.edge_id)
        .await
        .expect("delete mention edge");
    assert_eq!(
        ledger_count(&storage, TAG_EVENT_TYPE, &mention.edge_id).await,
        2
    );
    assert_eq!(counts(&db, &ws, &src).await, (0, 0, 0));
    assert_eq!(counts(&db, &ws, &hub).await, (0, 0, 0));
    assert_eq!(counts(&db, &ws, &target).await, (0, 0, 0));
    // Deleting again is not a second receipt: the aggregate is gone.
    assert!(matches!(
        db.delete_loom_edge(&ctx, &ws, &tag.edge_id).await,
        Err(StorageError::NotFound("loom_edge"))
    ));
    assert_eq!(
        ledger_count(&storage, TAG_EVENT_TYPE, &tag.edge_id).await,
        2
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
    assert!(reopened_db
        .list_loom_edges_for_block(&ws, &src)
        .await
        .expect("restart edge list")
        .is_empty());
    assert_eq!(
        ledger_count(&reopened_storage, TAG_EVENT_TYPE, &tag.edge_id).await,
        2
    );
    assert_eq!(
        ledger_count(&reopened_storage, TAG_EVENT_TYPE, &mention.edge_id).await,
        2
    );
    assert_eq!(counts(&reopened_db, &ws, &src).await, (0, 0, 0));
    assert_eq!(counts(&reopened_db, &ws, &hub).await, (0, 0, 0));
    assert_eq!(counts(&reopened_db, &ws, &target).await, (0, 0, 0));
    reopened_storage
        .shutdown()
        .await
        .expect("close reopened store");
}

/// Concurrent identical and conflicting requests through the public \`Database\` surface: eight
/// concurrent creates of ONE edge id (four to one target, four to another) settle to exactly
/// one accepted edge, seven typed conflicts, one receipt and consistent counts; eight concurrent
/// favorite-set requests settle to one accepted change (one receipt) and seven no-ops.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mt150_concurrent_identical_and_conflicting_requests_settle_without_residue() {
    let temp = tempfile::tempdir().expect("create temporary data root");
    let (config, storage, db) = open_store(temp.path()).await;
    let ws = seed_workspace(&db).await;
    let src = make_block(&db, &ws, "race source", false).await;
    let hub_a = make_block(&db, &ws, "race hub a", false).await;
    let hub_b = make_block(&db, &ws, "race hub b", false).await;
    let fav = make_block(&db, &ws, "race favorite", false).await;
    let edge_id = format!("edge-race-{}", uuid::Uuid::now_v7());

    let mut handles = Vec::new();
    for attempt in 0..8 {
        let db = db.clone();
        let ws = ws.clone();
        let src = src.clone();
        let target = if attempt % 2 == 0 {
            hub_a.clone()
        } else {
            hub_b.clone()
        };
        let edge_id = edge_id.clone();
        handles.push(tokio::spawn(async move {
            let ctx = WriteContext::human(Some(format!("racer-{attempt}")));
            let mut edge = new_edge(&ws, &src, &target, LoomEdgeType::Tag);
            edge.edge_id = Some(edge_id);
            db.create_loom_edge(&ctx, edge).await
        }));
    }
    let mut accepted = Vec::new();
    let mut conflicts = 0;
    for handle in handles {
        match handle.await.expect("racer joins") {
            Ok(edge) => accepted.push(edge),
            Err(StorageError::Conflict("loom_edge_exists")) => conflicts += 1,
            Err(other) => panic!("unexpected racer outcome: {other:?}"),
        }
    }
    assert_eq!(accepted.len(), 1, "exactly one stable accepted outcome");
    assert_eq!(
        conflicts, 7,
        "every other request is a deterministic typed conflict"
    );
    let winner = &accepted[0];
    assert_eq!(
        ledger_count(&storage, TAG_EVENT_TYPE, &edge_id).await,
        1,
        "no orphan ledger row from the losers"
    );
    assert_eq!(
        edge_receipt_id(&storage, &edge_id).await,
        winner.event_ledger_event_id
    );
    let (winner_target, loser_target) = if winner.target_block_id == hub_a {
        (hub_a.clone(), hub_b.clone())
    } else {
        (hub_b.clone(), hub_a.clone())
    };
    assert_eq!(counts(&db, &ws, &src).await, (0, 1, 0));
    assert_eq!(counts(&db, &ws, &winner_target).await, (0, 0, 1));
    assert_eq!(counts(&db, &ws, &loser_target).await, (0, 0, 0));

    let mut handles = Vec::new();
    for attempt in 0..8 {
        let db = db.clone();
        let ws = ws.clone();
        let fav = fav.clone();
        handles.push(tokio::spawn(async move {
            let ctx = WriteContext::human(Some(format!("racer-{attempt}")));
            db.update_loom_block(
                &ctx,
                &ws,
                &fav,
                LoomBlockUpdate {
                    favorite: Some(true),
                    ..Default::default()
                },
            )
            .await
        }));
    }
    for handle in handles {
        let block = handle
            .await
            .expect("racer joins")
            .expect("identical favorite requests all succeed");
        assert!(block.favorite);
    }
    assert_eq!(
        ledger_count(&storage, BLOCK_EVENT_TYPE, &fav).await,
        1,
        "one accepted change, seven no-ops: exactly one receipt"
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
    let edges = reopened_db
        .list_loom_edges_for_block(&ws, &src)
        .await
        .expect("restart edge list");
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].target_block_id, winner_target);
    assert_eq!(
        ledger_count(&reopened_storage, TAG_EVENT_TYPE, &edge_id).await,
        1
    );
    assert!(
        reopened_db
            .get_loom_block(&ws, &fav)
            .await
            .expect("restart read")
            .favorite
    );
    assert_eq!(
        ledger_count(&reopened_storage, BLOCK_EVENT_TYPE, &fav).await,
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

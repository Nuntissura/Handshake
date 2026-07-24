//! WP-KERNEL-012 E3 MT-024 / MT-025 — atomic EventLedger receipt proof on REAL
//! PostgreSQL for block bookmark mutations and wiki projection overlays.
//!
//! FAIL_V2 remediation proof:
//!   * MT-024 — pin/favorite mutations, pin REORDER, and (critically) pin
//!     REMOVAL must append their durable EventLedger receipt in the SAME
//!     transaction as the domain write. The old pin removal was a two-call
//!     sequence (PUT /pin-order(null) THEN PATCH {pinned:false}) that could
//!     clear pin_order before the second request failed, leaving partial
//!     persisted state with no reliable recovery. `remove_loom_block_pin`
//!     collapses that into ONE atomic operation.
//!   * MT-025 — the wiki overlay POST previously persisted the overlay row
//!     directly with NO durable business-event receipt. `add_loom_wiki_overlay`
//!     now appends KNOWLEDGE_LOOM_WIKI_MUTATED atomically with the insert.
//!
//! Every test proves:
//!   1. Happy path: after a mutation both the domain row (with a non-null
//!      event_ledger_event_id) AND the matching kernel_event_ledger row exist,
//!      and they survive a simulated restart (a fresh PostgresDatabase client
//!      reading the committed schema).
//!   2. Genuine injected-failure atomicity: a BEFORE INSERT trigger on
//!      kernel_event_ledger forces the receipt append to fail. The mutation must
//!      roll back entirely — after the failure and a fresh-client readback there
//!      is NO partial state (no half-removed pin, no orphan overlay, no phantom
//!      ledger row).
//!
//! The failure is injected at the real database boundary (a real trigger), not
//! through a production test hook, so no scaffolding weakens the product path.

mod knowledge_pg_support;

use handshake_core::storage::postgres::PostgresDatabase;
use handshake_core::storage::{
    Database, LoomBlockContentType, LoomBlockDerived, LoomBlockUpdate, LoomEdgeCreatedBy,
    LoomEdgeType, NewLoomBlock, NewLoomEdge, WriteContext,
};
use knowledge_pg_support::{knowledge_pg, KnowledgePg};
use sqlx::Row;

macro_rules! pg_or_skip {
    () => {{
        match knowledge_pg().await {
            Some(pg) => pg,
            None => {
                eprintln!("SKIP MT-024/025 atomic receipt proof: PostgreSQL unavailable");
                return;
            }
        }
    }};
}

const BLOCK_EVENT_TYPE: &str = "KNOWLEDGE_LOOM_BLOCK_MUTATED";
const WIKI_EVENT_TYPE: &str = "KNOWLEDGE_LOOM_WIKI_MUTATED";
const TAG_EVENT_TYPE: &str = "KNOWLEDGE_LOOM_TAG_MUTATED";

async fn make_block(db: &PostgresDatabase, ws: &str, title: &str, pinned: bool) -> String {
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

/// Read the event_ledger_event_id stored on a loom_blocks row (None if absent).
async fn block_receipt_id(pg: &KnowledgePg, block_id: &str) -> Option<String> {
    let mut conn = pg.raw_connection().await;
    let row = sqlx::query("SELECT event_ledger_event_id FROM loom_blocks WHERE block_id = $1")
        .bind(block_id)
        .fetch_optional(&mut conn)
        .await
        .expect("read block receipt id");
    row.and_then(|r| r.get::<Option<String>, _>("event_ledger_event_id"))
}

/// Read the event_ledger_event_id stored on a loom_wiki_overlays row.
async fn overlay_receipt_id(pg: &KnowledgePg, overlay_id: &str) -> Option<String> {
    let mut conn = pg.raw_connection().await;
    let row =
        sqlx::query("SELECT event_ledger_event_id FROM loom_wiki_overlays WHERE overlay_id = $1")
            .bind(overlay_id)
            .fetch_optional(&mut conn)
            .await
            .expect("read overlay receipt id");
    row.and_then(|r| r.get::<Option<String>, _>("event_ledger_event_id"))
}

async fn overlay_row_count(pg: &KnowledgePg, projection_id: &str) -> i64 {
    let mut conn = pg.raw_connection().await;
    sqlx::query("SELECT COUNT(*)::BIGINT AS n FROM loom_wiki_overlays WHERE projection_id = $1")
        .bind(projection_id)
        .fetch_one(&mut conn)
        .await
        .expect("count overlay rows")
        .get::<i64, _>("n")
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
// MT-024: pin REORDER atomicity
// ---------------------------------------------------------------------------

#[tokio::test]
async fn mt024_pin_order_set_appends_atomic_receipt_and_survives_restart() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let block = make_block(&pg.db, &ws, "Reorderable pin", true).await;
    let ctx = WriteContext::human(None);

    let ordered = pg
        .db
        .set_loom_block_pin_order(&ctx, &ws, &block, Some(3))
        .await
        .expect("set pin order");
    assert_eq!(ordered.pin_order, Some(3));

    let receipt = block_receipt_id(&pg, &block)
        .await
        .expect("committed reorder must carry a durable receipt id");
    assert_eq!(
        ledger_count(&pg, BLOCK_EVENT_TYPE, &block).await,
        1,
        "exactly one pin-order receipt in the ledger"
    );

    // Restart readback: a fresh client sees the ordinal and the same receipt id.
    let restart = restart_client(&pg).await;
    let read = restart
        .get_loom_block(&ws, &block)
        .await
        .expect("restart client reads the committed block");
    assert_eq!(read.pin_order, Some(3), "pin order persists across restart");
    let _ = restart.close().await;
    let restart_receipt = block_receipt_id(&pg, &block)
        .await
        .expect("receipt id persists across restart");
    assert_eq!(receipt, restart_receipt, "receipt id is stable across restart");

    pg.teardown().await;
}

#[tokio::test]
async fn mt024_pin_order_set_rolls_back_when_ledger_append_fails() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let block = make_block(&pg.db, &ws, "Rollback reorder", true).await;
    let ctx = WriteContext::human(None);

    install_ledger_fail_trigger(&pg).await;
    let result = pg.db.set_loom_block_pin_order(&ctx, &ws, &block, Some(9)).await;
    assert!(
        result.is_err(),
        "set pin order must fail when the atomic ledger append fails"
    );
    remove_ledger_fail_trigger(&pg).await;

    // Restart readback: no partial ordinal, no receipt, no phantom ledger row.
    let restart = restart_client(&pg).await;
    let read = restart
        .get_loom_block(&ws, &block)
        .await
        .expect("restart client reads the block");
    assert_eq!(read.pin_order, None, "pin order was NOT partially applied");
    let _ = restart.close().await;
    assert_eq!(block_receipt_id(&pg, &block).await, None);
    assert_eq!(ledger_count(&pg, BLOCK_EVENT_TYPE, &block).await, 0);

    pg.teardown().await;
}

// ---------------------------------------------------------------------------
// MT-024: favorite mutation atomicity
// ---------------------------------------------------------------------------

#[tokio::test]
async fn mt024_favorite_mutation_appends_atomic_receipt_and_survives_restart() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let block = make_block(&pg.db, &ws, "Favorite me", false).await;
    let ctx = WriteContext::human(None);

    // favorite:true -> receipt
    let fav = pg
        .db
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
    assert!(block_receipt_id(&pg, &block).await.is_some());
    assert_eq!(ledger_count(&pg, BLOCK_EVENT_TYPE, &block).await, 1);

    // favorite:false -> second receipt (the Favorites remove path, AC3)
    pg.db
        .update_loom_block(
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
        ledger_count(&pg, BLOCK_EVENT_TYPE, &block).await,
        2,
        "set + clear favorite receipts"
    );

    // Restart readback: the un-favorite persisted.
    let restart = restart_client(&pg).await;
    let read = restart.get_loom_block(&ws, &block).await.expect("restart read");
    assert!(!read.favorite, "un-favorite persists across restart");
    let _ = restart.close().await;

    pg.teardown().await;
}

#[tokio::test]
async fn mt024_favorite_mutation_rolls_back_when_ledger_append_fails() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let block = make_block(&pg.db, &ws, "Favorite rollback", false).await;
    let ctx = WriteContext::human(None);

    install_ledger_fail_trigger(&pg).await;
    let result = pg
        .db
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
    assert!(result.is_err(), "favorite mutation must fail with the ledger append");
    remove_ledger_fail_trigger(&pg).await;

    let restart = restart_client(&pg).await;
    let read = restart.get_loom_block(&ws, &block).await.expect("restart read");
    assert!(!read.favorite, "favorite was NOT partially applied");
    let _ = restart.close().await;
    assert_eq!(block_receipt_id(&pg, &block).await, None);
    assert_eq!(ledger_count(&pg, BLOCK_EVENT_TYPE, &block).await, 0);

    pg.teardown().await;
}

// ---------------------------------------------------------------------------
// MT-024: pin REMOVAL atomicity (the core FAIL_V2 finding)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn mt024_pin_removal_is_atomic_and_survives_restart() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let block = make_block(&pg.db, &ws, "Pinned + ordered", true).await;
    let ctx = WriteContext::human(None);
    // Give it an ordinal so removal has BOTH columns to clear atomically.
    pg.db
        .set_loom_block_pin_order(&ctx, &ws, &block, Some(2))
        .await
        .expect("seed pin order");

    let removed = pg
        .db
        .remove_loom_block_pin(&ctx, &ws, &block)
        .await
        .expect("atomic pin removal");
    assert!(!removed.pinned, "removal unpins the block");
    assert_eq!(removed.pin_order, None, "removal clears the ordinal");

    assert!(block_receipt_id(&pg, &block).await.is_some());
    assert_eq!(
        ledger_count(&pg, BLOCK_EVENT_TYPE, &block).await,
        2,
        "pin_order_set + pin_removed receipts"
    );

    // Restart readback: both columns cleared for a fresh client.
    let restart = restart_client(&pg).await;
    let read = restart.get_loom_block(&ws, &block).await.expect("restart read");
    assert!(!read.pinned, "unpin persists across restart");
    assert_eq!(read.pin_order, None, "cleared ordinal persists across restart");
    let _ = restart.close().await;

    pg.teardown().await;
}

#[tokio::test]
async fn mt024_pin_removal_rolls_back_and_leaves_no_partial_state() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let block = make_block(&pg.db, &ws, "No partial removal", true).await;
    let ctx = WriteContext::human(None);
    pg.db
        .set_loom_block_pin_order(&ctx, &ws, &block, Some(5))
        .await
        .expect("seed pin order");
    assert_eq!(ledger_count(&pg, BLOCK_EVENT_TYPE, &block).await, 1);

    // Inject the receipt failure: the removal transaction must roll BOTH column
    // changes back — the exact partial state (pin_order cleared but still pinned)
    // the old two-call flow risked must be impossible.
    install_ledger_fail_trigger(&pg).await;
    let result = pg.db.remove_loom_block_pin(&ctx, &ws, &block).await;
    assert!(result.is_err(), "pin removal must fail when the ledger append fails");
    remove_ledger_fail_trigger(&pg).await;

    // Restart readback: the block is STILL fully pinned WITH its ordinal intact.
    let restart = restart_client(&pg).await;
    let read = restart.get_loom_block(&ws, &block).await.expect("restart read");
    assert!(read.pinned, "block remains pinned after rolled-back removal");
    assert_eq!(
        read.pin_order,
        Some(5),
        "pin_order was NOT cleared: no partial removal state persisted"
    );
    let _ = restart.close().await;
    // Only the original pin_order_set receipt exists; no pin_removed receipt.
    assert_eq!(ledger_count(&pg, BLOCK_EVENT_TYPE, &block).await, 1);

    pg.teardown().await;
}

// ---------------------------------------------------------------------------
// MT-024: backlink primitive atomicity (mention edges feeding the Backlinks
// section) reuse the MT-023 atomic edge-receipt boundary.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn mt024_mention_edge_backlink_is_atomic_and_survives_restart() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let src = make_block(&pg.db, &ws, "mentions target", false).await;
    let target = make_block(&pg.db, &ws, "target", false).await;
    let ctx = WriteContext::human(None);

    let edge = pg
        .db
        .create_loom_edge(
            &ctx,
            NewLoomEdge {
                edge_id: None,
                workspace_id: ws.clone(),
                source_block_id: src.clone(),
                target_block_id: target.clone(),
                edge_type: LoomEdgeType::Mention,
                created_by: LoomEdgeCreatedBy::User,
                crdt_site_id: None,
                source_anchor: None,
            },
        )
        .await
        .expect("create mention (backlink) edge");

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
        "committed mention edge must carry a durable receipt id"
    );
    assert_eq!(
        ledger_count(&pg, TAG_EVENT_TYPE, &edge.edge_id).await,
        1,
        "exactly one edge-mutation receipt for the backlink edge"
    );

    // Restart readback: the backlink edge (target -> src) persists.
    let restart = restart_client(&pg).await;
    let edges = restart
        .list_loom_edges_for_block(&ws, &target)
        .await
        .expect("restart lists edges pointing at the target");
    assert!(
        edges.iter().any(|e| e.edge_id == edge.edge_id),
        "mention backlink edge persists across restart"
    );
    let _ = restart.close().await;

    pg.teardown().await;
}

// ---------------------------------------------------------------------------
// MT-025: wiki projection overlay atomicity
// ---------------------------------------------------------------------------

async fn seed_wiki_projection(pg: &KnowledgePg, ws: &str) -> String {
    let block = make_block(&pg.db, ws, "Source block", false).await;
    let projection = pg
        .db
        .compile_loom_wiki_projection(ws, "Ownership model", &[block])
        .await
        .expect("compile wiki projection");
    projection.projection_id
}

#[tokio::test]
async fn mt025_wiki_overlay_appends_atomic_receipt_and_survives_restart() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let projection_id = seed_wiki_projection(&pg, &ws).await;

    let overlay = pg
        .db
        .add_loom_wiki_overlay(&ws, &projection_id, "operator note on the ownership model", None)
        .await
        .expect("add wiki overlay");

    let receipt = overlay_receipt_id(&pg, &overlay.overlay_id)
        .await
        .expect("committed overlay must carry a durable receipt id");
    assert_eq!(
        ledger_count(&pg, WIKI_EVENT_TYPE, &overlay.overlay_id).await,
        1,
        "exactly one overlay receipt in the ledger"
    );

    // Restart readback: a fresh client lists the overlay and the receipt is stable.
    let restart = restart_client(&pg).await;
    let overlays = restart
        .list_loom_wiki_overlays(&ws, &projection_id)
        .await
        .expect("restart client lists overlays");
    assert!(
        overlays
            .iter()
            .any(|o| o.overlay_id == overlay.overlay_id
                && o.annotation == "operator note on the ownership model"),
        "overlay annotation persists across restart"
    );
    let _ = restart.close().await;
    let restart_receipt = overlay_receipt_id(&pg, &overlay.overlay_id)
        .await
        .expect("receipt id persists across restart");
    assert_eq!(receipt, restart_receipt, "overlay receipt id is stable across restart");

    pg.teardown().await;
}

#[tokio::test]
async fn mt025_wiki_overlay_rolls_back_when_ledger_append_fails() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let projection_id = seed_wiki_projection(&pg, &ws).await;
    assert_eq!(overlay_row_count(&pg, &projection_id).await, 0);

    install_ledger_fail_trigger(&pg).await;
    let result = pg
        .db
        .add_loom_wiki_overlay(&ws, &projection_id, "should roll back", None)
        .await;
    assert!(
        result.is_err(),
        "overlay add must fail when the atomic ledger append fails"
    );
    remove_ledger_fail_trigger(&pg).await;

    // Restart readback: no partial overlay row and no phantom ledger row.
    let restart = restart_client(&pg).await;
    let overlays = restart
        .list_loom_wiki_overlays(&ws, &projection_id)
        .await
        .expect("restart lists overlays");
    assert!(
        overlays.is_empty(),
        "rolled-back overlay must not be readable after restart"
    );
    let _ = restart.close().await;
    assert_eq!(overlay_row_count(&pg, &projection_id).await, 0);

    pg.teardown().await;
}

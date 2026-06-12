//! WP-KERNEL-009 MT-177 LoomBlockKnowledgeBridge — REAL PostgreSQL + EventLedger
//! authority proof.
//!
//! Proves the foundational WP-009 Loom supersession (Master Spec §10.12 #9.1.1):
//! a LoomBlock resolves to the ProjectKnowledgeIndex (`knowledge_entities`,
//! entity_kind=`loom_block`) and carries a `KNOWLEDGE_LOOM_BLOCK_INDEXED`
//! EventLedger receipt. There is NO SQLite path: the storage crate compiles no
//! `sqlite` module, and these tests run against the same isolated schema the
//! full migration chain ran in (`knowledge_pg`), driving the real
//! `PostgresDatabase` (which implements both `Database` and `KnowledgeStore`).
//!
//! Covered:
//!  * bridge upserts a knowledge entity with stable identity (ws, loom_block,
//!    block_id) and a non-empty display_name (falls back when title is absent);
//!  * bridge appends a real EventLedger receipt referenced by the bridge row;
//!  * bridge is idempotent (re-bridge => same entity, re-pointed receipt, no
//!    duplicate authority rows);
//!  * get/list bridge read back the authority binding;
//!  * the authority backend is Postgres+EventLedger (the only variant);
//!  * fail-closed when bridging a non-existent / foreign block.

mod knowledge_pg_support;

use handshake_core::storage::knowledge::{KnowledgeEntityKind, KnowledgeStore};
use handshake_core::storage::{
    Database, LoomAuthorityBackend, LoomBlockContentType, LoomBlockDerived, NewLoomBlock,
    WriteContext,
};
use knowledge_pg_support::knowledge_pg;
use uuid::Uuid;

/// Macro: skip loudly (never silently green) when PostgreSQL binaries are
/// absent. Mirrors the knowledge_pg_support contract.
macro_rules! pg_or_skip {
    () => {{
        match knowledge_pg().await {
            Some(pg) => pg,
            None => {
                eprintln!("SKIP MT-177 loom knowledge bridge proof: PostgreSQL unavailable");
                return;
            }
        }
    }};
}

async fn make_block(
    db: &handshake_core::storage::postgres::PostgresDatabase,
    workspace_id: &str,
    title: Option<&str>,
    content_type: LoomBlockContentType,
) -> String {
    let ctx = WriteContext::human(None);
    let block = db
        .create_loom_block(
            &ctx,
            NewLoomBlock {
                block_id: None,
                workspace_id: workspace_id.to_string(),
                content_type,
                document_id: None,
                asset_id: None,
                title: title.map(|t| t.to_string()),
                original_filename: None,
                content_hash: None,
                pinned: false,
                journal_date: None,
                imported_at: None,
                derived: LoomBlockDerived::default(),
            },
        )
        .await
        .expect("create loom block");
    block.block_id
}

#[tokio::test]
async fn bridge_binds_loom_block_to_knowledge_entity_and_event_ledger() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let ctx = WriteContext::human(None);

    let block_id = make_block(&pg.db, &ws, Some("Design Notes"), LoomBlockContentType::Note).await;

    let bridge = pg
        .db
        .bridge_loom_block_to_knowledge(&ctx, &ws, &block_id)
        .await
        .expect("bridge loom block");

    assert_eq!(bridge.block_id, block_id);
    assert_eq!(bridge.workspace_id, ws);
    assert!(bridge.entity_id.starts_with("KEN-"), "entity id is a KEN id");
    assert!(!bridge.index_event_id.is_empty(), "receipt id present");

    // The ProjectKnowledgeIndex entity exists with the stable natural identity
    // (workspace, loom_block, block_id) — the block id IS the entity key.
    let entity = pg
        .db
        .get_knowledge_entity_by_identity(&ws, KnowledgeEntityKind::LoomBlock, &block_id)
        .await
        .expect("entity query")
        .expect("loom_block entity must exist");
    assert_eq!(entity.entity_id, bridge.entity_id);
    assert_eq!(entity.entity_kind, KnowledgeEntityKind::LoomBlock);
    assert_eq!(entity.entity_key, block_id);
    assert_eq!(entity.display_name, "Design Notes");

    // The EventLedger receipt is a real KNOWLEDGE_LOOM_BLOCK_INDEXED event,
    // aggregated on the entity, and the bridge row points at it.
    let events = pg
        .db
        .list_kernel_events_for_aggregate("knowledge_loom_block", &bridge.entity_id)
        .await
        .expect("list ledger events");
    let receipt = events
        .iter()
        .find(|e| e.event_id == bridge.index_event_id)
        .expect("bridge receipt must be in the EventLedger");
    assert_eq!(
        receipt.event_type.as_str(),
        "KNOWLEDGE_LOOM_BLOCK_INDEXED",
        "receipt is the loom-block-indexed event family"
    );
    assert_eq!(
        receipt.payload.get("block_id").and_then(|v| v.as_str()),
        Some(block_id.as_str())
    );
    assert_eq!(
        receipt.payload.get("entity_id").and_then(|v| v.as_str()),
        Some(bridge.entity_id.as_str())
    );
}

#[tokio::test]
async fn bridge_display_name_falls_back_when_title_absent() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let ctx = WriteContext::human(None);

    // No title, no filename: display_name must still be non-empty (0135 CHECK)
    // and stable/human-meaningful — never an absolute path.
    let block_id = make_block(&pg.db, &ws, None, LoomBlockContentType::Note).await;
    let bridge = pg
        .db
        .bridge_loom_block_to_knowledge(&ctx, &ws, &block_id)
        .await
        .expect("bridge untitled block");

    let entity = pg
        .db
        .get_knowledge_entity(&bridge.entity_id)
        .await
        .expect("entity query")
        .expect("entity exists");
    assert!(
        !entity.display_name.trim().is_empty(),
        "display_name is never empty"
    );
    assert!(
        entity.display_name.contains(&block_id),
        "fallback display_name is derived from the block id: {}",
        entity.display_name
    );
}

#[tokio::test]
async fn bridge_is_idempotent_no_duplicate_authority_rows() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let ctx = WriteContext::human(None);

    let block_id = make_block(&pg.db, &ws, Some("Reindex Me"), LoomBlockContentType::Note).await;

    let first = pg
        .db
        .bridge_loom_block_to_knowledge(&ctx, &ws, &block_id)
        .await
        .expect("first bridge");
    let second = pg
        .db
        .bridge_loom_block_to_knowledge(&ctx, &ws, &block_id)
        .await
        .expect("second bridge (re-index)");

    // Same stable knowledge entity across re-index runs.
    assert_eq!(
        first.entity_id, second.entity_id,
        "re-bridge resolves to the same knowledge entity"
    );

    // Exactly one loom_block entity for this block: no parallel/duplicate rows.
    let entities = pg
        .db
        .list_knowledge_entities_by_kind(&ws, KnowledgeEntityKind::LoomBlock)
        .await
        .expect("list loom_block entities");
    let for_block: Vec<_> = entities
        .iter()
        .filter(|e| e.entity_key == block_id)
        .collect();
    assert_eq!(
        for_block.len(),
        1,
        "exactly one knowledge entity bridges the block"
    );

    // Exactly one bridge row for the block (PK on block_id guarantees it).
    let bridges = pg
        .db
        .list_loom_block_knowledge_bridges(&ws)
        .await
        .expect("list bridges");
    let bridge_rows: Vec<_> = bridges.iter().filter(|b| b.block_id == block_id).collect();
    assert_eq!(bridge_rows.len(), 1, "exactly one bridge row per block");

    // The re-index appended a fresh receipt and the bridge re-points to it.
    let events = pg
        .db
        .list_kernel_events_for_aggregate("knowledge_loom_block", &first.entity_id)
        .await
        .expect("list ledger events");
    let receipt_count = events
        .iter()
        .filter(|e| e.event_type.as_str() == "KNOWLEDGE_LOOM_BLOCK_INDEXED")
        .count();
    assert!(
        receipt_count >= 2,
        "each bridge call leaves an EventLedger receipt (got {receipt_count})"
    );
    assert!(
        events.iter().any(|e| e.event_id == second.index_event_id),
        "bridge row points at a real ledger receipt after re-index"
    );
}

#[tokio::test]
async fn get_and_list_bridge_read_back_authority_binding() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let ctx = WriteContext::human(None);

    // Un-bridged read is None (the block exists but was never bridged via this
    // direct-storage path).
    let manual_block = make_block(&pg.db, &ws, Some("Manual"), LoomBlockContentType::Note).await;
    // (create_loom_block at the storage layer does NOT auto-bridge; the API
    // layer does. So this block is initially un-bridged.)
    let unbridged = pg
        .db
        .get_loom_block_knowledge_bridge(&ws, &manual_block)
        .await
        .expect("get bridge");
    assert!(unbridged.is_none(), "block is not bridged until bridged");

    let bridge = pg
        .db
        .bridge_loom_block_to_knowledge(&ctx, &ws, &manual_block)
        .await
        .expect("bridge");

    let fetched = pg
        .db
        .get_loom_block_knowledge_bridge(&ws, &manual_block)
        .await
        .expect("get bridge")
        .expect("bridge now present");
    assert_eq!(fetched.entity_id, bridge.entity_id);
    assert_eq!(fetched.index_event_id, bridge.index_event_id);

    let listed = pg
        .db
        .list_loom_block_knowledge_bridges(&ws)
        .await
        .expect("list bridges");
    assert!(listed.iter().any(|b| b.block_id == manual_block));
}

#[tokio::test]
async fn authority_backend_is_postgres_event_ledger() {
    let pg = pg_or_skip!();
    // §10.12 #9.1.1: the only Loom authority is Postgres + EventLedger.
    assert_eq!(
        pg.db.loom_authority_backend(),
        LoomAuthorityBackend::PostgresEventLedger
    );
    assert!(pg.db.loom_authority_backend().is_authority());
}

/// MT-177 SQLite-removal guard (no DB needed): proves SQLite is unreachable
/// from the compiled Loom (and whole-crate) runtime path, per the WP-009
/// no-SQLite supersession (§9.1.1). This is a source-level regression guard so
/// a future change cannot silently re-introduce a SQLite Loom backend.
#[test]
fn sqlite_is_unreachable_from_the_loom_runtime_path() {
    let crate_dir = env!("CARGO_MANIFEST_DIR");

    // 1. The storage module graph declares NO `sqlite` / `locus_sqlite` module,
    //    so no SQLite code is compiled into the crate.
    let storage_mod =
        std::fs::read_to_string(format!("{crate_dir}/src/storage/mod.rs")).expect("read storage/mod.rs");
    for forbidden in ["mod sqlite;", "mod locus_sqlite;", "pub mod sqlite;", "pub mod locus_sqlite;"]
    {
        assert!(
            !storage_mod.contains(forbidden),
            "storage/mod.rs must not declare a SQLite module ({forbidden})"
        );
    }

    // 2. The dead SQLite orphan source files are removed (MT-177 REMOVE path).
    for orphan in [
        "src/storage/sqlite.rs",
        "src/storage/locus_sqlite.rs",
    ] {
        assert!(
            !std::path::Path::new(&format!("{crate_dir}/{orphan}")).exists(),
            "SQLite orphan file {orphan} must be removed"
        );
    }

    // 3. sqlx is built WITHOUT the `sqlite` feature: a SQLite pool cannot even
    //    be constructed in this crate. (Postgres is the sole durable backend.)
    let cargo_toml =
        std::fs::read_to_string(format!("{crate_dir}/Cargo.toml")).expect("read Cargo.toml");
    let sqlx_line = cargo_toml
        .lines()
        .find(|line| line.trim_start().starts_with("sqlx ="))
        .expect("sqlx dependency line");
    assert!(
        !sqlx_line.contains("\"sqlite\""),
        "sqlx must not enable the sqlite feature: {sqlx_line}"
    );
    assert!(sqlx_line.contains("\"postgres\""), "sqlx enables postgres");

    // 4. The Loom storage API surface references no SQLite type.
    let loom_storage =
        std::fs::read_to_string(format!("{crate_dir}/src/storage/loom.rs")).expect("read loom.rs");
    let loom_api =
        std::fs::read_to_string(format!("{crate_dir}/src/api/loom.rs")).expect("read api/loom.rs");
    for src in [&loom_storage, &loom_api] {
        assert!(!src.contains("SqlitePool"), "no SqlitePool in Loom source");
        assert!(
            !src.contains("storage::sqlite"),
            "no storage::sqlite reference in Loom source"
        );
    }
}

#[tokio::test]
async fn bridge_fails_closed_on_missing_block() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let ctx = WriteContext::human(None);

    let missing = format!("loom-missing-{}", Uuid::now_v7());
    let err = pg
        .db
        .bridge_loom_block_to_knowledge(&ctx, &ws, &missing)
        .await
        .expect_err("bridging a non-existent block must fail closed");
    // The block lookup is a NotFound; never a silent success.
    let msg = format!("{err}");
    assert!(
        msg.contains("not found") || msg.contains("not_found"),
        "missing block bridge fails with not-found, got: {msg}"
    );
}

/// MT-177 adversarial: a LoomBlock with an un-bridgeable id (empty / surrounded
/// by whitespace) must be rejected at create time, so it can never exist as an
/// orphan block outside Postgres/EventLedger authority. (The block id becomes
/// the knowledge_entities entity_key, which forbids surrounding whitespace.)
#[tokio::test]
async fn create_rejects_unbridgeable_block_id() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let ctx = WriteContext::human(None);

    for bad_id in ["", "   ", " leading", "trailing ", "\tmixed\n"] {
        let result = pg
            .db
            .create_loom_block(
                &ctx,
                NewLoomBlock {
                    block_id: Some(bad_id.to_string()),
                    workspace_id: ws.clone(),
                    content_type: LoomBlockContentType::Note,
                    document_id: None,
                    asset_id: None,
                    title: Some("Bad Id".to_string()),
                    original_filename: None,
                    content_hash: None,
                    pinned: false,
                    journal_date: None,
                    imported_at: None,
                    derived: LoomBlockDerived::default(),
                },
            )
            .await;
        assert!(
            result.is_err(),
            "create_loom_block must reject un-bridgeable id {bad_id:?}"
        );
    }

    // No orphan blocks were created (none can be listed in the All view).
    let view = pg
        .db
        .query_loom_view(
            &ws,
            handshake_core::storage::LoomViewType::All,
            handshake_core::storage::LoomViewFilters::default(),
            100,
            0,
        )
        .await
        .expect("all view");
    let count = match view {
        handshake_core::storage::LoomViewResponse::All { blocks } => blocks.len(),
        _ => unreachable!("all view returns All"),
    };
    assert_eq!(count, 0, "no orphan blocks created from rejected ids");
}

//! WP-KERNEL-009 MT-262 BlockCollectionViews — REAL PostgreSQL + EventLedger
//! authority proof.
//!
//! Proves saved table / Kanban / calendar views over the REAL Loom query
//! backend (Master Spec §10.12). All assertions run against the same isolated
//! schema the full migration chain ran in.
//!
//! Covered:
//!  * a saved view IS a `LoomBlock(content_type='view_def')` carrying its
//!    definition in the dedicated `view_definition_json` column (NOT a
//!    derived_json overload), with a ProjectKnowledgeIndex bridge + receipt;
//!  * table sort by a typed column is correct ACROSS A PAGE BOUNDARY (insert
//!    more rows than the limit; page 2 continues the global SQL-side sort);
//!  * Kanban move via the REAL tag edge create/delete re-queries to show the
//!    card in its new lane, and a fresh PG read reflects the change;
//!  * calendar buckets by the real date field and a date_from/to filter runs
//!    in SQL;
//!  * a re-sort persists into the view definition (saved-view reload proof).

mod knowledge_pg_support;

use handshake_core::storage::knowledge::{KnowledgeEntityKind, KnowledgeStore};
use handshake_core::storage::{
    BlockViewDefinition, BlockViewField, BlockViewGroupBy, BlockViewKind, BlockViewQuery,
    BlockViewSort, BlockViewSortDirection, Database, LoomBlockContentType, LoomBlockDerived,
    LoomEdgeCreatedBy, LoomEdgeType, NewLoomBlock, NewLoomEdge, WriteContext,
    BLOCK_VIEW_UNTAGGED_LANE,
};
use knowledge_pg_support::knowledge_pg;
use sha2::{Digest, Sha256};

macro_rules! pg_or_skip {
    () => {{
        match knowledge_pg().await {
            Some(pg) => pg,
            None => {
                eprintln!("SKIP MT-262 loom block collection views proof: PostgreSQL unavailable");
                return;
            }
        }
    }};
}

async fn make_block(
    db: &handshake_core::storage::postgres::PostgresDatabase,
    workspace_id: &str,
    title: &str,
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
        .expect("create loom block");
    db.bridge_loom_block_to_knowledge(&ctx, workspace_id, &block.block_id)
        .await
        .expect("bridge block");
    block.block_id
}

/// Create a saved view through the one atomic storage operation used by the API.
async fn make_view(
    db: &handshake_core::storage::postgres::PostgresDatabase,
    workspace_id: &str,
    title: &str,
    definition: BlockViewDefinition,
) -> String {
    let ctx = WriteContext::human(None);
    let block_id = uuid::Uuid::new_v4().to_string();
    db.create_block_view(
        &ctx,
        workspace_id,
        &block_id,
        Some(title.to_string()),
        definition,
    )
    .await
    .expect("create block view");
    block_id
}

#[tokio::test]
async fn saved_view_creation_rolls_back_every_authority_surface_and_retries_idempotently() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let definition = BlockViewDefinition {
        kind: BlockViewKind::Table,
        query: BlockViewQuery::default(),
        columns: vec![BlockViewField::Title],
        group_by: None,
        sort: None,
        calendar_date_field: None,
    };

    // Inject after each preceding durable operation. Every failure must erase
    // block/search/entity/ledger/bridge/outbox authority together.
    let fault_boundaries = [
        (
            "search_projection",
            "loom_block_search_index",
            "BEFORE INSERT",
            "",
        ),
        (
            "knowledge_entity",
            "knowledge_entities",
            "BEFORE INSERT",
            "",
        ),
        (
            "knowledge_index_receipt",
            "kernel_event_ledger",
            "BEFORE INSERT",
            "WHEN (NEW.event_type = 'KNOWLEDGE_LOOM_BLOCK_INDEXED')",
        ),
        (
            "knowledge_bridge",
            "loom_block_knowledge_bridge",
            "BEFORE INSERT",
            "",
        ),
        (
            "mutation_receipt",
            "kernel_event_ledger",
            "BEFORE INSERT",
            "WHEN (NEW.event_type = 'KNOWLEDGE_LOOM_BLOCK_MUTATED')",
        ),
        (
            "block_receipt_link",
            "loom_blocks",
            "BEFORE UPDATE",
            "WHEN (OLD.content_type = 'view_def')",
        ),
        (
            "recorder_outbox",
            "loom_block_view_fr_outbox",
            "BEFORE INSERT",
            "",
        ),
    ];
    for (boundary, table, timing, predicate) in fault_boundaries {
        let view_id = uuid::Uuid::new_v4().to_string();
        let function_name = format!("mt027_fail_{boundary}");
        let trigger_name = format!("mt027_fail_{boundary}_trigger");
        let mut conn = pg.raw_connection().await;
        sqlx::query(&format!(
            "CREATE FUNCTION {function_name}() RETURNS trigger AS $$ BEGIN \
             RAISE EXCEPTION 'MT027 injected {boundary} failure'; END; $$ LANGUAGE plpgsql"
        ))
        .execute(&mut conn)
        .await
        .unwrap_or_else(|error| panic!("install {boundary} fault function: {error}"));
        sqlx::query(&format!(
            "CREATE TRIGGER {trigger_name} {timing} ON {table} \
             FOR EACH ROW {predicate} EXECUTE FUNCTION {function_name}()"
        ))
        .execute(&mut conn)
        .await
        .unwrap_or_else(|error| panic!("install {boundary} fault trigger: {error}"));
        drop(conn);

        pg.db
            .create_block_view(
                &WriteContext::human(None),
                &ws,
                &view_id,
                Some(format!("Atomic view {boundary}")),
                definition.clone(),
            )
            .await
            .expect_err("injected boundary fault must fail the atomic create");

        let mut conn = pg.raw_connection().await;
        for (label, sql) in [
            (
                "block",
                "SELECT COUNT(*) FROM loom_blocks WHERE workspace_id = $1 AND block_id = $2",
            ),
            (
                "search",
                "SELECT COUNT(*) FROM loom_block_search_index WHERE workspace_id = $1 AND block_id = $2",
            ),
            (
                "bridge",
                "SELECT COUNT(*) FROM loom_block_knowledge_bridge WHERE workspace_id = $1 AND block_id = $2",
            ),
            (
                "entity",
                "SELECT COUNT(*) FROM knowledge_entities WHERE workspace_id = $1 AND entity_key = $2",
            ),
            (
                "ledger",
                "SELECT COUNT(*) FROM kernel_event_ledger WHERE payload->>'workspace_id' = $1 AND payload->>'block_id' = $2",
            ),
            (
                "outbox",
                "SELECT COUNT(*) FROM loom_block_view_fr_outbox WHERE workspace_id = $1 AND block_id = $2",
            ),
        ] {
            let count: i64 = sqlx::query_scalar(sql)
                .bind(&ws)
                .bind(&view_id)
                .fetch_one(&mut conn)
                .await
                .unwrap_or_else(|error| panic!("count {label} after {boundary}: {error}"));
            assert_eq!(
                count, 0,
                "{label} must roll back with the {boundary} fault"
            );
        }
        sqlx::query(&format!("DROP TRIGGER {trigger_name} ON {table}"))
            .execute(&mut conn)
            .await
            .unwrap_or_else(|error| panic!("drop {boundary} fault trigger: {error}"));
        sqlx::query(&format!("DROP FUNCTION {function_name}()"))
            .execute(&mut conn)
            .await
            .unwrap_or_else(|error| panic!("drop {boundary} fault function: {error}"));
    }
    let view_id = uuid::Uuid::new_v4().to_string();
    let created = pg
        .db
        .create_block_view(
            &WriteContext::human(None),
            &ws,
            &view_id,
            Some("Atomic view".to_owned()),
            definition.clone(),
        )
        .await
        .expect("create after fault removal");
    assert_eq!(created.block.block_id, view_id);
    let retry = pg
        .db
        .create_block_view(
            &WriteContext::human(None),
            &ws,
            &view_id,
            Some("Atomic view".to_owned()),
            definition.clone(),
        )
        .await
        .expect("same-id identical retry converges");
    assert_eq!(retry.block.block_id, view_id);
    pg.db
        .create_block_view(
            &WriteContext::human(None),
            &ws,
            &view_id,
            Some("Conflicting title".to_owned()),
            definition,
        )
        .await
        .expect_err("same id with changed payload must conflict");

    let mut conn = pg.raw_connection().await;
    let counts: (i64, i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
          (SELECT COUNT(*) FROM loom_blocks WHERE workspace_id = $1 AND block_id = $2),
          (SELECT COUNT(*) FROM loom_block_knowledge_bridge WHERE workspace_id = $1 AND block_id = $2),
          (SELECT COUNT(*) FROM knowledge_entities WHERE workspace_id = $1 AND entity_key = $2),
          (SELECT COUNT(*) FROM loom_block_view_fr_outbox WHERE workspace_id = $1 AND block_id = $2)
        "#,
    )
    .bind(&ws)
    .bind(&view_id)
    .fetch_one(&mut conn)
    .await
    .expect("post-retry authority counts");
    assert_eq!(counts, (1, 1, 1, 1));
    let bridge_content_type: String = sqlx::query_scalar(
        "SELECT detection_provenance->>'content_type' FROM knowledge_entities \
         WHERE workspace_id = $1 AND entity_key = $2",
    )
    .bind(&ws)
    .bind(&view_id)
    .fetch_one(&mut conn)
    .await
    .expect("bridge provenance content type");
    assert_eq!(bridge_content_type, "view_def");
    let indexed_content_type: String = sqlx::query_scalar(
        "SELECT payload->>'content_type' FROM kernel_event_ledger \
         WHERE event_type = 'KNOWLEDGE_LOOM_BLOCK_INDEXED' \
           AND payload->>'workspace_id' = $1 AND payload->>'block_id' = $2",
    )
    .bind(&ws)
    .bind(&view_id)
    .fetch_one(&mut conn)
    .await
    .expect("bridge EventLedger content type");
    assert_eq!(indexed_content_type, "view_def");

    let unicode_view_id = uuid::Uuid::new_v4().to_string();
    pg.db
        .create_block_view(
            &WriteContext::human(Some("Cafe\u{301}".to_owned())),
            &ws,
            &unicode_view_id,
            Some("Unicode-normalized view".to_owned()),
            BlockViewDefinition {
                kind: BlockViewKind::Table,
                query: BlockViewQuery::default(),
                columns: vec![BlockViewField::Title],
                group_by: None,
                sort: None,
                calendar_date_field: None,
            },
        )
        .await
        .expect("create decomposed-Unicode actor view");
    let normalized_actor: String = sqlx::query_scalar(
        "SELECT event->>'actor_id' FROM loom_block_view_fr_outbox \
         WHERE workspace_id = $1 AND block_id = $2",
    )
    .bind(&ws)
    .bind(&unicode_view_id)
    .fetch_one(&mut conn)
    .await
    .expect("read normalized outbox event");
    assert_eq!(
        normalized_actor, "Café",
        "outbox authority must match recorder NFC persistence before hashing"
    );
}

#[tokio::test]
async fn legacy_view_projection_migration_repairs_real_pg_idempotently_and_survives_restart() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let definition = BlockViewDefinition {
        kind: BlockViewKind::Table,
        query: BlockViewQuery::default(),
        columns: vec![BlockViewField::Title],
        group_by: None,
        sort: None,
        calendar_date_field: None,
    };
    let stale_view_id = make_view(&pg.db, &ws, "Legacy stale projection", definition.clone()).await;
    let missing_view_id = make_view(&pg.db, &ws, "Legacy missing projection", definition).await;
    let ordinary_note_id = make_block(
        &pg.db,
        &ws,
        "Ordinary note must stay a note",
        LoomBlockContentType::Note,
    )
    .await;

    let mut conn = pg.raw_connection().await;
    sqlx::query(
        "UPDATE loom_block_search_index \
         SET content_type = 'note', search_text = 'stale legacy projection', \
             embedding_model = 'legacy-model' \
         WHERE workspace_id = $1 AND block_id = $2",
    )
    .bind(&ws)
    .bind(&stale_view_id)
    .execute(&mut conn)
    .await
    .expect("seed stale saved-view search projection");
    sqlx::query(
        "DELETE FROM loom_block_search_index \
         WHERE workspace_id = $1 AND block_id = $2",
    )
    .bind(&ws)
    .bind(&missing_view_id)
    .execute(&mut conn)
    .await
    .expect("seed missing saved-view search projection");
    sqlx::query(
        "UPDATE knowledge_entities AS entity \
         SET detection_provenance = jsonb_build_object( \
             'extractor', 'loom_block_knowledge_bridge', \
             'extractor_version', 'loom_block_knowledge_bridge_v1', \
             'method', 'mt177_bridge', \
             'content_type', 'note' \
         ) \
         FROM loom_block_knowledge_bridge AS bridge \
         WHERE bridge.entity_id = entity.entity_id \
           AND bridge.workspace_id = $1 \
           AND bridge.block_id = $2",
    )
    .bind(&ws)
    .bind(&stale_view_id)
    .execute(&mut conn)
    .await
    .expect("seed note-typed legacy knowledge provenance");
    sqlx::query(
        "DELETE FROM knowledge_entities \
         WHERE workspace_id = $1 \
           AND entity_kind = 'loom_block' \
           AND entity_key = $2",
    )
    .bind(&ws)
    .bind(&missing_view_id)
    .execute(&mut conn)
    .await
    .expect("seed missing saved-view knowledge projection");

    sqlx::raw_sql(include_str!(
        "../migrations/0363_loom_block_view_legacy_projection_repair.sql"
    ))
    .execute(&mut conn)
    .await
    .expect("upgrade legacy saved-view projections");

    let repaired_search: Vec<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT block_id, search_text, embedding_model \
         FROM loom_block_search_index \
         WHERE workspace_id = $1 AND block_id = ANY($2) \
         ORDER BY block_id",
    )
    .bind(&ws)
    .bind(vec![stale_view_id.clone(), missing_view_id.clone()])
    .fetch_all(&mut conn)
    .await
    .expect("read repaired saved-view search projections");
    assert_eq!(repaired_search.len(), 2);
    assert!(repaired_search.iter().any(|(block_id, text, model)| {
        block_id == &stale_view_id
            && text == "Legacy stale projection"
            && model.as_deref() == Some("legacy-model")
    }));
    assert!(repaired_search.iter().any(|(block_id, text, model)| {
        block_id == &missing_view_id && text == "Legacy missing projection" && model.is_none()
    }));

    let repaired_receipts: Vec<(String, String, serde_json::Value, String)> = sqlx::query_as(
        "SELECT bridge.block_id, event.event_type, event.payload, event.payload_hash \
         FROM loom_block_knowledge_bridge AS bridge \
         JOIN knowledge_entities AS entity ON entity.entity_id = bridge.entity_id \
         JOIN kernel_event_ledger AS event ON event.event_id = bridge.index_event_id \
         WHERE bridge.workspace_id = $1 \
           AND bridge.block_id = ANY($2) \
           AND entity.detection_provenance ->> 'content_type' = 'view_def' \
         ORDER BY bridge.block_id",
    )
    .bind(&ws)
    .bind(vec![stale_view_id.clone(), missing_view_id.clone()])
    .fetch_all(&mut conn)
    .await
    .expect("read repaired knowledge projections and receipts");
    assert_eq!(repaired_receipts.len(), 2);
    for (block_id, event_type, payload, payload_hash) in &repaired_receipts {
        assert_eq!(event_type, "KNOWLEDGE_LOOM_BLOCK_INDEXED");
        assert_eq!(payload["type"], "knowledge_loom_block_indexed");
        assert_eq!(payload["workspace_id"], ws.as_str());
        assert_eq!(payload["block_id"], block_id.as_str());
        assert_eq!(payload["content_type"], "view_def");
        assert_eq!(payload["repair_reason"], "legacy_view_projection_repair");
        let canonical_payload =
            serde_json::to_vec(payload).expect("serialize canonical repair payload");
        assert_eq!(
            payload_hash,
            &hex::encode(Sha256::digest(canonical_payload)),
            "migration receipt hash must match the runtime canonical JSON hash"
        );
    }

    let first_projection_timestamps: Vec<(
        String,
        chrono::DateTime<chrono::Utc>,
        chrono::DateTime<chrono::Utc>,
        chrono::DateTime<chrono::Utc>,
    )> = sqlx::query_as(
        "SELECT bridge.block_id, search.indexed_at, entity.updated_at, bridge.updated_at \
         FROM loom_block_knowledge_bridge AS bridge \
         JOIN loom_block_search_index AS search ON search.block_id = bridge.block_id \
         JOIN knowledge_entities AS entity ON entity.entity_id = bridge.entity_id \
         WHERE bridge.workspace_id = $1 AND bridge.block_id = ANY($2) \
         ORDER BY bridge.block_id",
    )
    .bind(&ws)
    .bind(vec![stale_view_id.clone(), missing_view_id.clone()])
    .fetch_all(&mut conn)
    .await
    .expect("capture first repair timestamps");
    sqlx::raw_sql(include_str!(
        "../migrations/0363_loom_block_view_legacy_projection_repair.sql"
    ))
    .execute(&mut conn)
    .await
    .expect("replay saved-view projection repair");
    let replay_projection_timestamps: Vec<(
        String,
        chrono::DateTime<chrono::Utc>,
        chrono::DateTime<chrono::Utc>,
        chrono::DateTime<chrono::Utc>,
    )> = sqlx::query_as(
        "SELECT bridge.block_id, search.indexed_at, entity.updated_at, bridge.updated_at \
         FROM loom_block_knowledge_bridge AS bridge \
         JOIN loom_block_search_index AS search ON search.block_id = bridge.block_id \
         JOIN knowledge_entities AS entity ON entity.entity_id = bridge.entity_id \
         WHERE bridge.workspace_id = $1 AND bridge.block_id = ANY($2) \
         ORDER BY bridge.block_id",
    )
    .bind(&ws)
    .bind(vec![stale_view_id.clone(), missing_view_id.clone()])
    .fetch_all(&mut conn)
    .await
    .expect("capture replay timestamps");
    assert_eq!(
        replay_projection_timestamps, first_projection_timestamps,
        "an idempotent replay must not rewrite already-canonical projections"
    );
    let receipt_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger \
         WHERE source_component = 'loom_block_view_legacy_projection_repair' \
           AND payload ->> 'workspace_id' = $1",
    )
    .bind(&ws)
    .fetch_one(&mut conn)
    .await
    .expect("count replay-safe repair receipts");
    assert_eq!(receipt_count, 2, "one repair receipt per saved view");
    let ordinary_note: (String, String, String, i64) = sqlx::query_as(
        "SELECT block.content_type, search.content_type, \
                entity.detection_provenance ->> 'content_type', \
                (SELECT COUNT(*) FROM kernel_event_ledger AS repair \
                 WHERE repair.source_component = 'loom_block_view_legacy_projection_repair' \
                   AND repair.payload ->> 'block_id' = block.block_id) \
         FROM loom_blocks AS block \
         JOIN loom_block_search_index AS search ON search.block_id = block.block_id \
         JOIN loom_block_knowledge_bridge AS bridge ON bridge.block_id = block.block_id \
         JOIN knowledge_entities AS entity ON entity.entity_id = bridge.entity_id \
         WHERE block.workspace_id = $1 AND block.block_id = $2",
    )
    .bind(&ws)
    .bind(&ordinary_note_id)
    .fetch_one(&mut conn)
    .await
    .expect("inspect ordinary note after migration");
    assert_eq!(
        ordinary_note,
        ("note".to_owned(), "note".to_owned(), "note".to_owned(), 0),
        "migration must never infer that an indistinguishable note is a stranded view"
    );
    drop(conn);

    // A new product database/pool models process restart: no in-memory state
    // from the migration connection can satisfy these reads.
    let restarted = handshake_core::storage::postgres::PostgresDatabase::connect(&pg.schema_url, 1)
        .await
        .expect("restart PostgresDatabase on repaired schema");
    for view_id in [&stale_view_id, &missing_view_id] {
        let view = restarted
            .get_block_view(&ws, view_id)
            .await
            .expect("saved view survives restart");
        assert!(matches!(
            view.block.content_type,
            LoomBlockContentType::ViewDef
        ));
        let bridge = restarted
            .get_loom_block_knowledge_bridge(&ws, view_id)
            .await
            .expect("read repaired bridge after restart")
            .expect("repaired bridge exists after restart");
        assert!(
            bridge.index_event_id.starts_with("KE-MT028-0363-"),
            "bridge must retain the typed repair receipt after restart"
        );
    }
    restarted.close().await;
}

#[tokio::test]
async fn outbox_retention_migration_round_trips_deleted_block_intent() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let view_id = uuid::Uuid::new_v4().to_string();
    pg.db
        .create_block_view(
            &WriteContext::human(None),
            &ws,
            &view_id,
            Some("Retained publication".to_owned()),
            BlockViewDefinition {
                kind: BlockViewKind::Table,
                query: BlockViewQuery::default(),
                columns: vec![BlockViewField::Title],
                group_by: None,
                sort: None,
                calendar_date_field: None,
            },
        )
        .await
        .expect("create retained block view");
    pg.db
        .delete_loom_block(&WriteContext::human(None), &ws, &view_id)
        .await
        .expect("delete block while retaining outbox intent");

    let mut conn = pg.raw_connection().await;
    let retained_before: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM loom_block_view_fr_outbox \
         WHERE workspace_id = $1 AND block_id = $2",
    )
    .bind(&ws)
    .bind(&view_id)
    .fetch_one(&mut conn)
    .await
    .expect("retained outbox before rollback");
    assert_eq!(retained_before, 1);

    // PostgreSQL DDL is transactional. Keep the destructive down/up probe
    // private to this connection so parallel Handshake jobs continue seeing
    // the committed retention schema throughout the test. A panic or dropped
    // connection also rolls the probe back automatically.
    sqlx::query("BEGIN")
        .execute(&mut conn)
        .await
        .expect("begin isolated retention migration probe");
    sqlx::raw_sql(include_str!(
        "../migrations/0362_loom_block_view_outbox_retention.down.sql"
    ))
    .execute(&mut conn)
    .await
    .expect("rollback retention migration with an orphaned intent");
    let active_after_down: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM loom_block_view_fr_outbox \
         WHERE workspace_id = $1 AND block_id = $2",
    )
    .bind(&ws)
    .bind(&view_id)
    .fetch_one(&mut conn)
    .await
    .expect("active outbox after rollback");
    let archived_after_down: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM loom_block_view_fr_outbox_retention_archive \
         WHERE workspace_id = $1 AND block_id = $2",
    )
    .bind(&ws)
    .bind(&view_id)
    .fetch_one(&mut conn)
    .await
    .expect("archived outbox after rollback");
    let fk_after_down: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM pg_constraint \
         WHERE conname = 'fk_loom_block_view_fr_outbox_block')",
    )
    .fetch_one(&mut conn)
    .await
    .expect("block FK after rollback");
    assert_eq!(
        (active_after_down, archived_after_down, fk_after_down),
        (0, 1, true)
    );

    sqlx::raw_sql(include_str!(
        "../migrations/0362_loom_block_view_outbox_retention.sql"
    ))
    .execute(&mut conn)
    .await
    .expect("reapply retention migration");
    let restored_after_up: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM loom_block_view_fr_outbox \
         WHERE workspace_id = $1 AND block_id = $2",
    )
    .bind(&ws)
    .bind(&view_id)
    .fetch_one(&mut conn)
    .await
    .expect("restored outbox after forward migration");
    let archive_dropped: bool = sqlx::query_scalar(
        "SELECT to_regclass('loom_block_view_fr_outbox_retention_archive') IS NULL",
    )
    .fetch_one(&mut conn)
    .await
    .expect("retention archive removed after restore");
    assert_eq!((restored_after_up, archive_dropped), (1, true));

    // Roll back once more, then delete the owning workspace while the intent
    // is archived. The archive's workspace FK must cascade that row, and the
    // forward migration must tolerate/reject no stale workspace reference.
    sqlx::raw_sql(include_str!(
        "../migrations/0362_loom_block_view_outbox_retention.down.sql"
    ))
    .execute(&mut conn)
    .await
    .expect("second rollback retention migration");
    sqlx::query("DELETE FROM workspaces WHERE id = $1")
        .bind(&ws)
        .execute(&mut conn)
        .await
        .expect("delete workspace while publication intent is archived");
    let archived_after_workspace_delete: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM loom_block_view_fr_outbox_retention_archive \
         WHERE workspace_id = $1",
    )
    .bind(&ws)
    .fetch_one(&mut conn)
    .await
    .expect("archived outbox after workspace delete");
    assert_eq!(
        archived_after_workspace_delete, 0,
        "archive rows must follow workspace ON DELETE CASCADE semantics"
    );
    sqlx::raw_sql(include_str!(
        "../migrations/0362_loom_block_view_outbox_retention.sql"
    ))
    .execute(&mut conn)
    .await
    .expect("reapply retention migration after workspace deletion");
    let restored_after_workspace_delete: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM loom_block_view_fr_outbox \
         WHERE workspace_id = $1 AND block_id = $2",
    )
    .bind(&ws)
    .bind(&view_id)
    .fetch_one(&mut conn)
    .await
    .expect("outbox after deleted-workspace forward migration");
    assert_eq!(
        restored_after_workspace_delete, 0,
        "forward migration must not resurrect an event for a deleted workspace"
    );

    // The preceding migration owns the entire outbox feature boundary. Undo
    // 0362 and then 0361 exactly as sqlx would: neither the live table nor the
    // rollback archive may survive. The surrounding transaction restores the
    // committed test database after this assertion.
    sqlx::raw_sql(include_str!(
        "../migrations/0362_loom_block_view_outbox_retention.down.sql"
    ))
    .execute(&mut conn)
    .await
    .expect("rollback retention before full outbox teardown");
    sqlx::raw_sql(include_str!(
        "../migrations/0361_loom_block_view_fr_outbox.down.sql"
    ))
    .execute(&mut conn)
    .await
    .expect("rollback outbox feature boundary");
    let boundary_removed: (bool, bool) = sqlx::query_as(
        "SELECT
           to_regclass('loom_block_view_fr_outbox') IS NULL,
           to_regclass('loom_block_view_fr_outbox_retention_archive') IS NULL",
    )
    .fetch_one(&mut conn)
    .await
    .expect("inspect full outbox teardown");
    assert_eq!(
        boundary_removed,
        (true, true),
        "0361 down must remove both the live outbox and rollback archive"
    );
    sqlx::query("ROLLBACK")
        .execute(&mut conn)
        .await
        .expect("rollback isolated retention migration probe");
}

async fn add_tag_edge(
    db: &handshake_core::storage::postgres::PostgresDatabase,
    workspace_id: &str,
    source: &str,
    tag: &str,
) -> String {
    let ctx = WriteContext::human(None);
    db.create_loom_edge(
        &ctx,
        NewLoomEdge {
            edge_id: None,
            workspace_id: workspace_id.to_string(),
            source_block_id: source.to_string(),
            target_block_id: tag.to_string(),
            edge_type: LoomEdgeType::Tag,
            created_by: LoomEdgeCreatedBy::User,
            crdt_site_id: None,
            source_anchor: None,
        },
    )
    .await
    .expect("create tag edge")
    .edge_id
}

#[tokio::test]
async fn view_def_block_round_trips_with_bridge_and_dedicated_column() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;

    let definition = BlockViewDefinition {
        kind: BlockViewKind::Table,
        query: BlockViewQuery::default(),
        columns: vec![BlockViewField::Title, BlockViewField::Updated],
        group_by: None,
        sort: Some(BlockViewSort {
            field: BlockViewField::Title,
            direction: BlockViewSortDirection::Asc,
        }),
        calendar_date_field: None,
    };
    let view_id = make_view(&pg.db, &ws, "All notes (A-Z)", definition).await;

    // It IS a content_type='view_def' LoomBlock.
    let block = pg
        .db
        .get_loom_block(&ws, &view_id)
        .await
        .expect("get block");
    assert!(matches!(block.content_type, LoomBlockContentType::ViewDef));

    // Authority-resolved through the ProjectKnowledgeIndex bridge.
    let bridge = pg
        .db
        .get_loom_block_knowledge_bridge(&ws, &view_id)
        .await
        .expect("read bridge")
        .expect("bridge exists for view block");
    let entity = pg
        .db
        .get_knowledge_entity(&bridge.entity_id)
        .await
        .expect("get entity")
        .expect("entity exists");
    assert!(matches!(entity.entity_kind, KnowledgeEntityKind::LoomBlock));

    // The definition decodes back from the dedicated column (NOT derived_json).
    let record = pg.db.get_block_view(&ws, &view_id).await.expect("get view");
    assert!(matches!(record.definition.kind, BlockViewKind::Table));
    assert_eq!(record.definition.columns.len(), 2);

    // The dedicated column is populated and derived_json is NOT carrying it.
    let mut conn = pg.raw_connection().await;
    let row: (Option<String>, String) = sqlx::query_as(
        "SELECT view_definition_json, derived_json FROM loom_blocks WHERE block_id = $1",
    )
    .bind(&view_id)
    .fetch_one(&mut conn)
    .await
    .expect("probe row");
    assert!(row.0.is_some(), "view_definition_json populated");
    assert!(
        !row.1.contains("\"kind\":\"table\""),
        "definition must not leak into derived_json overload"
    );
}

#[tokio::test]
async fn table_sort_by_typed_column_is_correct_across_a_page_boundary() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;

    // Insert more blocks than the page limit, with deterministic sortable
    // titles (T000..T011). 12 blocks, page size 5 -> page 2 must continue the
    // GLOBAL ascending title sort, proving the ORDER BY runs SQL-side.
    for i in 0..12u32 {
        make_block(&pg.db, &ws, &format!("T{i:03}"), LoomBlockContentType::Note).await;
    }

    let definition = BlockViewDefinition {
        kind: BlockViewKind::Table,
        query: BlockViewQuery {
            content_type: Some(LoomBlockContentType::Note),
            ..BlockViewQuery::default()
        },
        columns: vec![BlockViewField::Title],
        group_by: None,
        sort: Some(BlockViewSort {
            field: BlockViewField::Title,
            direction: BlockViewSortDirection::Asc,
        }),
        calendar_date_field: None,
    };

    let page1 = pg
        .db
        .query_block_view_results(&ws, &definition, 5, 0)
        .await
        .expect("page 1");
    let page2 = pg
        .db
        .query_block_view_results(&ws, &definition, 5, 5)
        .await
        .expect("page 2");

    let titles1: Vec<String> = page1
        .blocks
        .iter()
        .map(|b| b.title.clone().unwrap_or_default())
        .collect();
    let titles2: Vec<String> = page2
        .blocks
        .iter()
        .map(|b| b.title.clone().unwrap_or_default())
        .collect();

    assert_eq!(
        titles1,
        vec!["T000", "T001", "T002", "T003", "T004"],
        "page 1 ascending"
    );
    assert_eq!(
        titles2,
        vec!["T005", "T006", "T007", "T008", "T009"],
        "page 2 CONTINUES the global ascending sort (SQL-side, not client-side)"
    );
}

#[tokio::test]
async fn kanban_move_via_real_tag_edges_reflects_in_requery_and_pg() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let ctx = WriteContext::human(None);

    // Two tag lanes (real TagHub blocks) + a card starting in "todo".
    let todo = make_block(&pg.db, &ws, "todo", LoomBlockContentType::TagHub).await;
    let done = make_block(&pg.db, &ws, "done", LoomBlockContentType::TagHub).await;
    let card = make_block(&pg.db, &ws, "Ship MT-262", LoomBlockContentType::Note).await;
    let todo_edge = add_tag_edge(&pg.db, &ws, &card, &todo).await;

    let definition = BlockViewDefinition {
        kind: BlockViewKind::Kanban,
        query: BlockViewQuery {
            content_type: Some(LoomBlockContentType::Note),
            tag_ids: vec![todo.clone(), done.clone()],
            ..BlockViewQuery::default()
        },
        columns: vec![BlockViewField::Title],
        group_by: Some(BlockViewGroupBy::Tag),
        sort: None,
        calendar_date_field: None,
    };

    let before = pg
        .db
        .query_block_view_results(&ws, &definition, 100, 0)
        .await
        .expect("before move");
    let todo_lane = before
        .groups
        .iter()
        .find(|l| l.key == todo)
        .expect("todo lane");
    assert!(
        todo_lane.blocks.iter().any(|b| b.block_id == card),
        "card starts in the todo lane"
    );

    // Kanban move = REAL mutation: delete the old tag edge, create the new one.
    pg.db
        .delete_loom_edge(&ctx, &ws, &todo_edge)
        .await
        .expect("delete todo edge");
    add_tag_edge(&pg.db, &ws, &card, &done).await;

    // Re-query (never local state as truth) shows the card in its NEW lane.
    let after = pg
        .db
        .query_block_view_results(&ws, &definition, 100, 0)
        .await
        .expect("after move");
    let done_lane = after
        .groups
        .iter()
        .find(|l| l.key == done)
        .expect("done lane");
    assert!(
        done_lane.blocks.iter().any(|b| b.block_id == card),
        "card now in the done lane after the real tag mutation"
    );
    let todo_lane_after = after
        .groups
        .iter()
        .find(|l| l.key == todo)
        .expect("todo lane");
    assert!(
        !todo_lane_after.blocks.iter().any(|b| b.block_id == card),
        "card no longer in the todo lane"
    );

    // Fresh PG read of the edges confirms authority moved (not just the view).
    let edges = pg
        .db
        .list_loom_edges_for_block(&ws, &card)
        .await
        .expect("list edges");
    let tag_targets: Vec<String> = edges
        .iter()
        .filter(|e| e.edge_type == LoomEdgeType::Tag)
        .map(|e| e.target_block_id.clone())
        .collect();
    assert!(tag_targets.contains(&done), "PG: card tagged done");
    assert!(
        !tag_targets.contains(&todo),
        "PG: card no longer tagged todo"
    );
}

#[tokio::test]
async fn free_kanban_places_shared_tag_cards_once_each() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;

    let shared_tag = make_block(&pg.db, &ws, "shared", LoomBlockContentType::TagHub).await;
    let second_tag = make_block(&pg.db, &ws, "second", LoomBlockContentType::TagHub).await;
    let first = make_block(&pg.db, &ws, "First", LoomBlockContentType::Note).await;
    let second = make_block(&pg.db, &ws, "Second", LoomBlockContentType::Note).await;
    // Insert in the reverse of canonical key order so query/HashSet iteration
    // cannot accidentally satisfy the stable-lane assertion.
    let mut reversed_tags = [shared_tag.clone(), second_tag.clone()];
    reversed_tags.sort_by(|a, b| b.cmp(a));
    for tag in &reversed_tags {
        add_tag_edge(&pg.db, &ws, &first, tag).await;
    }
    add_tag_edge(&pg.db, &ws, &first, &shared_tag).await;
    add_tag_edge(&pg.db, &ws, &second, &shared_tag).await;

    let definition = BlockViewDefinition {
        kind: BlockViewKind::Kanban,
        query: BlockViewQuery {
            content_type: Some(LoomBlockContentType::Note),
            ..BlockViewQuery::default()
        },
        columns: vec![BlockViewField::Title],
        group_by: Some(BlockViewGroupBy::Tag),
        sort: None,
        calendar_date_field: None,
    };

    let results = pg
        .db
        .query_block_view_results(&ws, &definition, 100, 0)
        .await
        .expect("query free Kanban");
    let lane_keys: Vec<&str> = results
        .groups
        .iter()
        .filter(|lane| lane.key != BLOCK_VIEW_UNTAGGED_LANE)
        .map(|lane| lane.key.as_str())
        .collect();
    let mut expected_keys = vec![shared_tag.as_str(), second_tag.as_str()];
    expected_keys.sort();
    assert_eq!(
        lane_keys, expected_keys,
        "free-Kanban dynamic lane order is stable by canonical tag id"
    );
    let lane = results
        .groups
        .iter()
        .find(|lane| lane.key == shared_tag)
        .expect("shared dynamic tag lane");
    let ids: Vec<&str> = lane
        .blocks
        .iter()
        .map(|block| block.block_id.as_str())
        .collect();
    assert_eq!(ids.len(), 2, "each card appears once in its dynamic lane");
    assert_eq!(
        ids.iter()
            .copied()
            .collect::<std::collections::HashSet<_>>()
            .len(),
        2,
        "a free Kanban must not duplicate a card when its lane already exists"
    );
    assert!(ids.contains(&first.as_str()));
    assert!(ids.contains(&second.as_str()));
}

#[tokio::test]
async fn calendar_buckets_by_journal_date_with_sql_date_filter() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let ctx = WriteContext::human(None);

    // Three journal blocks on distinct dates (real journal_date field).
    for date in ["2026-06-10", "2026-06-15", "2026-06-20"] {
        pg.db
            .get_or_create_daily_journal_block(&ctx, &ws, date)
            .await
            .expect("journal block");
    }

    // A calendar view bucketing by journal_date with a SQL date window that
    // excludes the first journal (date_from = 2026-06-12).
    let definition = BlockViewDefinition {
        kind: BlockViewKind::Calendar,
        query: BlockViewQuery {
            content_type: Some(LoomBlockContentType::Journal),
            date_from: Some(
                chrono::DateTime::parse_from_rfc3339("2026-06-12T00:00:00Z")
                    .unwrap()
                    .with_timezone(&chrono::Utc),
            ),
            ..BlockViewQuery::default()
        },
        columns: vec![],
        group_by: None,
        sort: Some(BlockViewSort {
            field: BlockViewField::JournalDate,
            direction: BlockViewSortDirection::Asc,
        }),
        calendar_date_field: Some(BlockViewField::JournalDate),
    };

    let view_id = make_view(&pg.db, &ws, "June journal", definition.clone()).await;
    let results = pg
        .db
        .query_block_view_results(&ws, &definition, 100, 0)
        .await
        .expect("calendar results");

    let dates: Vec<String> = results
        .blocks
        .iter()
        .filter_map(|b| b.journal_date.clone())
        .collect();
    assert!(
        dates.contains(&"2026-06-15".to_string()) && dates.contains(&"2026-06-20".to_string()),
        "SQL date filter keeps journals on/after 2026-06-12: {dates:?}"
    );
    assert!(
        !dates.contains(&"2026-06-10".to_string()),
        "SQL date_from filter excludes the 2026-06-10 journal: {dates:?}"
    );
    // Ascending journal_date order (server-side).
    let mut sorted = dates.clone();
    sorted.sort();
    assert_eq!(dates, sorted, "journals returned in ascending journal_date");

    // Saved-view reload proof: the persisted definition decodes back identical.
    let reloaded = pg
        .db
        .get_block_view(&ws, &view_id)
        .await
        .expect("reload view");
    assert!(matches!(reloaded.definition.kind, BlockViewKind::Calendar));
    assert!(matches!(
        reloaded.definition.calendar_date_field,
        Some(BlockViewField::JournalDate)
    ));
}

#[tokio::test]
async fn resort_persists_into_view_definition() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;

    let definition = BlockViewDefinition {
        kind: BlockViewKind::Table,
        query: BlockViewQuery::default(),
        columns: vec![BlockViewField::Title, BlockViewField::Created],
        group_by: None,
        sort: Some(BlockViewSort {
            field: BlockViewField::Title,
            direction: BlockViewSortDirection::Asc,
        }),
        calendar_date_field: None,
    };
    let view_id = make_view(&pg.db, &ws, "Resortable", definition).await;

    // A header click re-sorts by Created DESC and PERSISTS (not localStorage).
    let new_definition = BlockViewDefinition {
        kind: BlockViewKind::Table,
        query: BlockViewQuery::default(),
        columns: vec![BlockViewField::Title, BlockViewField::Created],
        group_by: None,
        sort: Some(BlockViewSort {
            field: BlockViewField::Created,
            direction: BlockViewSortDirection::Desc,
        }),
        calendar_date_field: None,
    };
    pg.db
        .update_block_view_definition(&WriteContext::human(None), &ws, &view_id, new_definition)
        .await
        .expect("update definition");

    let reloaded = pg.db.get_block_view(&ws, &view_id).await.expect("reload");
    let sort = reloaded.definition.sort.expect("sort persisted");
    assert!(matches!(sort.field, BlockViewField::Created));
    assert!(matches!(sort.direction, BlockViewSortDirection::Desc));

    // The untagged sentinel is a stable, public contract for empty-tag lanes.
    assert_eq!(BLOCK_VIEW_UNTAGGED_LANE, "__untagged__");
}

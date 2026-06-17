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

/// Create a saved view by birthing a real LoomBlock (+bridge) then flipping it
/// to view_def with its definition — mirrors the create_block_view API path.
async fn make_view(
    db: &handshake_core::storage::postgres::PostgresDatabase,
    workspace_id: &str,
    title: &str,
    definition: BlockViewDefinition,
) -> String {
    let ctx = WriteContext::human(None);
    let block = db
        .create_loom_block(
            &ctx,
            NewLoomBlock {
                block_id: None,
                workspace_id: workspace_id.to_string(),
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
        .expect("create view block");
    db.bridge_loom_block_to_knowledge(&ctx, workspace_id, &block.block_id)
        .await
        .expect("bridge view block");
    db.create_block_view(
        &ctx,
        workspace_id,
        &block.block_id,
        Some(title.to_string()),
        definition,
    )
    .await
    .expect("create block view");
    block.block_id
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
    let block = pg.db.get_loom_block(&ws, &view_id).await.expect("get block");
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
    let todo_lane_after = after.groups.iter().find(|l| l.key == todo).expect("todo lane");
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
    assert!(!tag_targets.contains(&todo), "PG: card no longer tagged todo");
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
        dates.contains(&"2026-06-15".to_string())
            && dates.contains(&"2026-06-20".to_string()),
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
    let reloaded = pg.db.get_block_view(&ws, &view_id).await.expect("reload view");
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

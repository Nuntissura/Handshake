//! WP-KERNEL-009 MT-262 BlockCollectionViews — embedded SurrealDB + EventLedger
//! authority proof.
//!
//! Proves saved table / Kanban / calendar views over the real Loom query
//! backend (Master Spec §10.12). Every executable assertion uses an isolated
//! embedded store opened by the shared test support.
//!
//! Covered:
//!  * a saved view is a `LoomBlock(content_type='view_def')` carrying its
//!    definition in the dedicated `view_definition_json` field (not a
//!    `derived` overload), with a ProjectKnowledgeIndex bridge + receipt;
//!  * table sort by a typed column is correct across a page boundary;
//!  * Kanban move via the real tag edge create/delete re-queries to show the
//!    card in its new lane, and a fresh embedded read reflects the change;
//!  * calendar buckets by the real date field and a date filter;
//!  * a re-sort persists into the view definition (saved-view reload proof).

#[path = "knowledge_ingestion_support.rs"]
mod knowledge_ingestion_support;

use handshake_core::storage::knowledge::{KnowledgeEntityKind, KnowledgeStore};
use handshake_core::storage::surreal::RowFilter;
use handshake_core::storage::{
    BlockViewDefinition, BlockViewField, BlockViewGroupBy, BlockViewKind, BlockViewQuery,
    BlockViewSort, BlockViewSortDirection, Database, LoomBlockContentType, LoomBlockDerived,
    LoomEdgeCreatedBy, LoomEdgeType, NewLoomBlock, NewLoomEdge, WriteContext,
    BLOCK_VIEW_UNTAGGED_LANE,
};
use knowledge_ingestion_support::{open_embedded_store, EmbeddedKnowledgeStore};

macro_rules! embedded_or_skip {
    () => {{
        match open_embedded_store().await {
            Some(store) => store,
            None => {
                eprintln!(
                    "SKIP MT-141 loom block collection views proof: embedded store unavailable"
                );
                return;
            }
        }
    }};
}

async fn make_block(
    db: &handshake_core::storage::surreal::SurrealDatabase,
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
    db: &handshake_core::storage::surreal::SurrealDatabase,
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

async fn embedded_row_count_by_id(
    store: &EmbeddedKnowledgeStore,
    table_name: &str,
    record_id: &str,
) -> u64 {
    let inspector = store.storage.test_inspector();
    let table = inspector
        .table_selector(table_name)
        .await
        .expect("select embedded table");
    inspector
        .row_count(&table, RowFilter::IdEquals(record_id.to_owned()))
        .await
        .expect("count embedded row")
}

async fn embedded_row_count(store: &EmbeddedKnowledgeStore, table_name: &str) -> u64 {
    let inspector = store.storage.test_inspector();
    let table = inspector
        .table_selector(table_name)
        .await
        .expect("select embedded table");
    inspector
        .row_count(&table, RowFilter::All)
        .await
        .expect("count embedded rows")
}

#[derive(Clone, Copy, Debug)]
enum SavedViewCreateFailpoint {
    BlockCreate,
    Search,
    BridgeReceipt,
    MutationReceipt,
    Entity,
    Bridge,
    Outbox,
    ReceiptLink,
}

async fn set_saved_view_create_failpoint(
    store: &EmbeddedKnowledgeStore,
    point: SavedViewCreateFailpoint,
    enabled: bool,
) {
    let result = match point {
        SavedViewCreateFailpoint::BlockCreate => {
            store
                .storage
                .test_set_block_view_block_create_failpoint(enabled)
                .await
        }
        SavedViewCreateFailpoint::Search => {
            store
                .storage
                .test_set_block_view_search_failpoint(enabled)
                .await
        }
        SavedViewCreateFailpoint::BridgeReceipt => {
            store
                .storage
                .test_set_block_view_bridge_receipt_failpoint(enabled)
                .await
        }
        SavedViewCreateFailpoint::MutationReceipt => {
            store
                .storage
                .test_set_block_view_mutation_receipt_failpoint(enabled)
                .await
        }
        SavedViewCreateFailpoint::Entity => {
            store
                .storage
                .test_set_block_view_entity_failpoint(enabled)
                .await
        }
        SavedViewCreateFailpoint::Bridge => {
            store
                .storage
                .test_set_block_view_bridge_failpoint(enabled)
                .await
        }
        SavedViewCreateFailpoint::Outbox => {
            store
                .storage
                .test_set_block_view_outbox_failpoint(enabled)
                .await
        }
        SavedViewCreateFailpoint::ReceiptLink => {
            store
                .storage
                .test_set_block_view_receipt_link_failpoint(enabled)
                .await
        }
    };
    result.unwrap_or_else(|error| panic!("set {point:?}={enabled}: {error}"));
}

#[tokio::test]
async fn saved_view_creation_is_idempotent_and_authority_backed() {
    let store = embedded_or_skip!();
    let workspace_id = store.create_workspace().await;
    let definition = BlockViewDefinition {
        kind: BlockViewKind::Table,
        query: BlockViewQuery::default(),
        columns: vec![BlockViewField::Title],
        group_by: None,
        sort: None,
        calendar_date_field: None,
    };
    let view_id = uuid::Uuid::new_v4().to_string();

    let created = store
        .db
        .create_block_view(
            &WriteContext::human(None),
            &workspace_id,
            &view_id,
            Some("Atomic view".to_owned()),
            definition.clone(),
        )
        .await
        .expect("create view");
    assert_eq!(created.block.block_id, view_id);
    assert!(matches!(
        created.block.content_type,
        LoomBlockContentType::ViewDef
    ));
    let publication_event_id = created
        .publication_event_id
        .expect("create returns its publication event id");

    let retry = store
        .db
        .create_block_view(
            &WriteContext::human(None),
            &workspace_id,
            &view_id,
            Some("Atomic view".to_owned()),
            definition.clone(),
        )
        .await
        .expect("same-id identical retry converges");
    assert_eq!(retry.block.block_id, view_id);
    assert!(matches!(retry.definition.kind, BlockViewKind::Table));
    assert_eq!(retry.publication_event_id, Some(publication_event_id));

    store
        .db
        .create_block_view(
            &WriteContext::human(None),
            &workspace_id,
            &view_id,
            Some("Conflicting title".to_owned()),
            definition,
        )
        .await
        .expect_err("same id with changed payload must conflict");

    let block = store
        .db
        .get_loom_block(&workspace_id, &view_id)
        .await
        .expect("read created view block");
    assert!(matches!(block.content_type, LoomBlockContentType::ViewDef));
    let bridge = store
        .db
        .get_loom_block_knowledge_bridge(&workspace_id, &view_id)
        .await
        .expect("read view bridge")
        .expect("view bridge exists");
    let entity = store
        .db
        .get_knowledge_entity(&bridge.entity_id)
        .await
        .expect("read bridged entity")
        .expect("bridged entity exists");
    assert!(matches!(entity.entity_kind, KnowledgeEntityKind::LoomBlock));
    assert_eq!(entity.entity_key, view_id);
    assert_eq!(
        entity
            .detection_provenance
            .get("content_type")
            .and_then(|value| value.as_str()),
        Some("view_def")
    );

    assert_eq!(
        (
            embedded_row_count_by_id(&store, "loom_blocks", &view_id).await,
            embedded_row_count_by_id(&store, "loom_block_search_index", &view_id).await,
            embedded_row_count_by_id(&store, "loom_block_knowledge_bridge", &view_id).await,
            embedded_row_count_by_id(&store, "knowledge_entities", &bridge.entity_id).await,
            embedded_row_count_by_id(
                &store,
                "loom_block_view_fr_outbox",
                &publication_event_id.to_string(),
            )
            .await,
        ),
        (1, 1, 1, 1, 1),
        "same-id retries and conflicts must leave exactly one row on every authority surface"
    );

    let events = store
        .db
        .list_kernel_events_for_aggregate("knowledge_loom_block", &bridge.entity_id)
        .await
        .expect("read view EventLedger receipts");
    let bridge_event = events
        .iter()
        .find(|event| event.event_id == bridge.index_event_id)
        .expect("bridge points to a real EventLedger receipt");
    assert_eq!(
        bridge_event.event_type.as_str(),
        "KNOWLEDGE_LOOM_BLOCK_INDEXED"
    );
    assert_eq!(
        bridge_event
            .payload
            .get("content_type")
            .and_then(|value| value.as_str()),
        Some("view_def")
    );
}

#[tokio::test]
async fn saved_view_create_failure_rolls_back_every_authority_surface() {
    let store = embedded_or_skip!();
    let workspace_id = store.create_workspace().await;
    let definition = BlockViewDefinition {
        kind: BlockViewKind::Table,
        query: BlockViewQuery::default(),
        columns: vec![BlockViewField::Title],
        group_by: None,
        sort: None,
        calendar_date_field: None,
    };
    let baseline_entities = embedded_row_count(&store, "knowledge_entities").await;
    let baseline_ledger = embedded_row_count(&store, "kernel_event_ledger").await;
    let baseline_outbox = embedded_row_count(&store, "loom_block_view_fr_outbox").await;
    for point in [
        SavedViewCreateFailpoint::BlockCreate,
        SavedViewCreateFailpoint::Search,
        SavedViewCreateFailpoint::BridgeReceipt,
        SavedViewCreateFailpoint::MutationReceipt,
        SavedViewCreateFailpoint::Entity,
        SavedViewCreateFailpoint::Bridge,
        SavedViewCreateFailpoint::Outbox,
        SavedViewCreateFailpoint::ReceiptLink,
    ] {
        let view_id = uuid::Uuid::new_v4().to_string();
        set_saved_view_create_failpoint(&store, point, true).await;
        store
            .db
            .create_block_view(
                &WriteContext::human(None),
                &workspace_id,
                &view_id,
                Some(format!("Atomic view {point:?}")),
                definition.clone(),
            )
            .await
            .expect_err("injected boundary failure must reject the create");
        set_saved_view_create_failpoint(&store, point, false).await;

        for table in [
            "loom_blocks",
            "loom_block_search_index",
            "loom_block_knowledge_bridge",
        ] {
            assert_eq!(
                embedded_row_count_by_id(&store, table, &view_id).await,
                0,
                "{table} must roll back at {point:?}"
            );
        }
        assert_eq!(
            embedded_row_count(&store, "knowledge_entities").await,
            baseline_entities,
            "entity creation must roll back at {point:?}"
        );
        assert_eq!(
            embedded_row_count(&store, "kernel_event_ledger").await,
            baseline_ledger,
            "both EventLedger writes must roll back at {point:?}"
        );
        assert_eq!(
            embedded_row_count(&store, "loom_block_view_fr_outbox").await,
            baseline_outbox,
            "outbox creation must roll back at {point:?}"
        );
    }

    let view_id = uuid::Uuid::new_v4().to_string();
    let title = "Atomic view after reset".to_owned();
    store
        .db
        .create_block_view(
            &WriteContext::human(None),
            &workspace_id,
            &view_id,
            Some(title.clone()),
            definition.clone(),
        )
        .await
        .expect("create succeeds after every failpoint resets");
    store.shutdown().await.expect("close embedded view store");
    let reopened = store.reopen_database().await.expect("reopen view store");
    let durable = reopened
        .get_block_view(&workspace_id, &view_id)
        .await
        .expect("created view survives reopen");
    assert_eq!(durable.block.title.as_deref(), Some(title.as_str()));
    let retry = reopened
        .create_block_view(
            &WriteContext::human(None),
            &workspace_id,
            &view_id,
            Some(title),
            definition,
        )
        .await
        .expect("same-id retry converges after reopen");
    assert_eq!(retry.block.block_id, view_id);
}

#[tokio::test]
async fn saved_view_creation_persists_normalized_outbox_actor() {
    let store = embedded_or_skip!();
    let workspace_id = store.create_workspace().await;
    let view_id = uuid::Uuid::new_v4().to_string();
    let created = store
        .db
        .create_block_view(
            &WriteContext::human(Some("Cafe\u{301}".to_owned())),
            &workspace_id,
            &view_id,
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
    let publication_event_id = created
        .publication_event_id
        .expect("create returns its publication event id")
        .to_string();

    let inspector = store.storage.test_inspector();
    let outbox = inspector
        .table_selector("loom_block_view_fr_outbox")
        .await
        .expect("select view outbox table");
    let rows = inspector
        .project(
            &outbox,
            &[
                outbox.field("operation").expect("select operation field"),
                outbox.field("event").expect("select event field"),
            ],
            RowFilter::IdEquals(publication_event_id.clone()),
        )
        .await
        .expect("read persisted view outbox event");
    assert_eq!(rows.len(), 1, "create must persist one exact outbox event");
    assert_eq!(
        rows[0].record_id.key_string(),
        Some(publication_event_id.as_str())
    );
    assert_eq!(rows[0].values["operation"].as_str(), Some("create"));
    assert_eq!(
        rows[0].values["event"]
            .get("actor_id")
            .and_then(serde_json::Value::as_str),
        Some("Caf\u{e9}"),
        "outbox authority must persist the normalized actor before hashing"
    );
}

#[allow(dead_code)]
const MT141_LEGACY_VIEW_PROJECTION_REPAIR_DISPOSITION: &str =
    "RETIRED migration 0363: schema.surql bootstraps only the latest empty schema; \
     block_view_store::CREATE_TRANSACTION is the current proof that each new view atomically creates \
     loom_block_search_index, knowledge_entities, loom_block_knowledge_bridge, both ledger receipts, \
     and the outbox. No pre-upgrade saved-view rows exist to repair.";

#[allow(dead_code)]
const MT141_DELETED_VIEW_OUTBOX_RETENTION_DISPOSITION: &str =
    "RETIRED migration 0362: schema.surql defines loom_block_view_fr_outbox.block_id as \
     record<loom_blocks> REFERENCE ON DELETE CASCADE. Deleted-block publication intent retention and \
     its archive/down-migration lifecycle are not current Surreal schema behavior.";

#[tokio::test]
async fn view_def_block_persists_definition_only_in_dedicated_field() {
    let store = embedded_or_skip!();
    let workspace_id = store.create_workspace().await;
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
    let view_id = make_view(
        &store.db,
        &workspace_id,
        "Dedicated definition field",
        definition.clone(),
    )
    .await;

    let inspector = store.storage.test_inspector();
    let blocks = inspector
        .table_selector("loom_blocks")
        .await
        .expect("select Loom block table");
    let rows = inspector
        .project(
            &blocks,
            &[
                blocks
                    .field("view_definition_json")
                    .expect("select dedicated definition field"),
                blocks.field("derived_json").expect("select derived field"),
            ],
            RowFilter::IdEquals(view_id),
        )
        .await
        .expect("read persisted view fields");
    assert_eq!(rows.len(), 1, "saved view must persist one Loom block");

    let persisted_definition = rows[0].values["view_definition_json"]
        .as_str()
        .expect("dedicated definition field is populated");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(persisted_definition)
            .expect("decode persisted view definition"),
        serde_json::to_value(&definition).expect("encode expected view definition")
    );
    assert_eq!(
        rows[0].values["derived_json"],
        serde_json::to_value(LoomBlockDerived::default()).expect("encode default derived payload"),
        "the view definition must not be overloaded into the derived payload"
    );
}

#[tokio::test]
async fn view_def_block_round_trips_with_bridge_and_dedicated_column() {
    let store = embedded_or_skip!();
    let workspace_id = store.create_workspace().await;
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
    let view_id = make_view(&store.db, &workspace_id, "All notes (A-Z)", definition).await;

    let block = store
        .db
        .get_loom_block(&workspace_id, &view_id)
        .await
        .expect("get view block");
    assert!(matches!(block.content_type, LoomBlockContentType::ViewDef));
    let bridge = store
        .db
        .get_loom_block_knowledge_bridge(&workspace_id, &view_id)
        .await
        .expect("read bridge")
        .expect("bridge exists for view block");
    let entity = store
        .db
        .get_knowledge_entity(&bridge.entity_id)
        .await
        .expect("get entity")
        .expect("entity exists");
    assert!(matches!(entity.entity_kind, KnowledgeEntityKind::LoomBlock));

    // get_block_view decodes the dedicated view field, while the typed block
    // still carries the default derived payload rather than the definition.
    let record = store
        .db
        .get_block_view(&workspace_id, &view_id)
        .await
        .expect("get saved view");
    assert!(matches!(record.definition.kind, BlockViewKind::Table));
    assert_eq!(record.definition.columns.len(), 2);
    let derived = serde_json::to_value(&record.block.derived).expect("serialize derived payload");
    assert!(
        !derived.to_string().contains("\"kind\":\"table\""),
        "view definition must not be carried by the derived payload"
    );
}

#[tokio::test]
async fn table_sort_by_typed_column_is_correct_across_a_page_boundary() {
    let store = embedded_or_skip!();
    let workspace_id = store.create_workspace().await;

    // Insert more blocks than the page limit, with deterministic sortable
    // titles (T000..T011). Page 2 must continue the global server-side sort.
    for i in 0..12u32 {
        make_block(
            &store.db,
            &workspace_id,
            &format!("T{i:03}"),
            LoomBlockContentType::Note,
        )
        .await;
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

    let page1 = store
        .db
        .query_block_view_results(&workspace_id, &definition, 5, 0)
        .await
        .expect("page 1");
    let page2 = store
        .db
        .query_block_view_results(&workspace_id, &definition, 5, 5)
        .await
        .expect("page 2");
    let titles1: Vec<String> = page1
        .blocks
        .iter()
        .map(|block| block.title.clone().unwrap_or_default())
        .collect();
    let titles2: Vec<String> = page2
        .blocks
        .iter()
        .map(|block| block.title.clone().unwrap_or_default())
        .collect();
    assert_eq!(
        titles1,
        vec!["T000", "T001", "T002", "T003", "T004"],
        "page 1 ascending"
    );
    assert_eq!(
        titles2,
        vec!["T005", "T006", "T007", "T008", "T009"],
        "page 2 continues the global ascending sort"
    );
}

async fn add_tag_edge(
    db: &handshake_core::storage::surreal::SurrealDatabase,
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
async fn kanban_move_via_real_tag_edges_reflects_in_requery_and_embedded_store() {
    let store = embedded_or_skip!();
    let workspace_id = store.create_workspace().await;
    let ctx = WriteContext::human(None);

    // Two tag lanes (real TagHub blocks) + a card starting in "todo".
    let todo = make_block(
        &store.db,
        &workspace_id,
        "todo",
        LoomBlockContentType::TagHub,
    )
    .await;
    let done = make_block(
        &store.db,
        &workspace_id,
        "done",
        LoomBlockContentType::TagHub,
    )
    .await;
    let card = make_block(
        &store.db,
        &workspace_id,
        "Ship MT-262",
        LoomBlockContentType::Note,
    )
    .await;
    let todo_edge = add_tag_edge(&store.db, &workspace_id, &card, &todo).await;
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

    let before = store
        .db
        .query_block_view_results(&workspace_id, &definition, 100, 0)
        .await
        .expect("before move");
    let todo_lane = before
        .groups
        .iter()
        .find(|lane| lane.key == todo)
        .expect("todo lane");
    assert!(
        todo_lane.blocks.iter().any(|block| block.block_id == card),
        "card starts in the todo lane"
    );

    // Kanban move = real mutation: delete the old tag edge, create the new one.
    store
        .db
        .delete_loom_edge(&ctx, &workspace_id, &todo_edge)
        .await
        .expect("delete todo edge");
    add_tag_edge(&store.db, &workspace_id, &card, &done).await;

    // Re-query (never local state as truth) shows the card in its new lane.
    let after = store
        .db
        .query_block_view_results(&workspace_id, &definition, 100, 0)
        .await
        .expect("after move");
    let done_lane = after
        .groups
        .iter()
        .find(|lane| lane.key == done)
        .expect("done lane");
    assert!(
        done_lane.blocks.iter().any(|block| block.block_id == card),
        "card now in the done lane after the real tag mutation"
    );
    let todo_lane_after = after
        .groups
        .iter()
        .find(|lane| lane.key == todo)
        .expect("todo lane after move");
    assert!(
        !todo_lane_after
            .blocks
            .iter()
            .any(|block| block.block_id == card),
        "card no longer in the todo lane"
    );

    // Fresh embedded read of the real edges confirms authority moved.
    let edges = store
        .db
        .list_loom_edges_for_block(&workspace_id, &card)
        .await
        .expect("list edges");
    let tag_targets: Vec<String> = edges
        .iter()
        .filter(|edge| edge.edge_type == LoomEdgeType::Tag)
        .map(|edge| edge.target_block_id.clone())
        .collect();
    assert!(tag_targets.contains(&done), "card is tagged done");
    assert!(
        !tag_targets.contains(&todo),
        "card is no longer tagged todo"
    );
}

#[tokio::test]
async fn free_kanban_places_shared_tag_cards_once_each() {
    let store = embedded_or_skip!();
    let workspace_id = store.create_workspace().await;
    let shared_tag = make_block(
        &store.db,
        &workspace_id,
        "shared",
        LoomBlockContentType::TagHub,
    )
    .await;
    let second_tag = make_block(
        &store.db,
        &workspace_id,
        "second",
        LoomBlockContentType::TagHub,
    )
    .await;
    let first = make_block(
        &store.db,
        &workspace_id,
        "First",
        LoomBlockContentType::Note,
    )
    .await;
    let second = make_block(
        &store.db,
        &workspace_id,
        "Second",
        LoomBlockContentType::Note,
    )
    .await;

    // Insert in reverse canonical key order so iteration cannot accidentally
    // satisfy the stable-lane assertion.
    let mut reversed_tags = [shared_tag.clone(), second_tag.clone()];
    reversed_tags.sort_by(|left, right| right.cmp(left));
    for tag in &reversed_tags {
        add_tag_edge(&store.db, &workspace_id, &first, tag).await;
    }
    add_tag_edge(&store.db, &workspace_id, &first, &shared_tag).await;
    add_tag_edge(&store.db, &workspace_id, &second, &shared_tag).await;

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
    let results = store
        .db
        .query_block_view_results(&workspace_id, &definition, 100, 0)
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
async fn calendar_buckets_by_journal_date_with_date_filter() {
    let store = embedded_or_skip!();
    let workspace_id = store.create_workspace().await;
    let ctx = WriteContext::human(None);

    // Three journal blocks on distinct dates (real journal_date field).
    for date in ["2026-06-10", "2026-06-15", "2026-06-20"] {
        store
            .db
            .get_or_create_daily_journal_block(&ctx, &workspace_id, date)
            .await
            .expect("journal block");
    }

    // A calendar view bucketing by journal_date with a date window that
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
    let view_id = make_view(&store.db, &workspace_id, "June journal", definition.clone()).await;
    let results = store
        .db
        .query_block_view_results(&workspace_id, &definition, 100, 0)
        .await
        .expect("calendar results");
    let dates: Vec<String> = results
        .blocks
        .iter()
        .filter_map(|block| block.journal_date.clone())
        .collect();
    assert!(
        dates.contains(&"2026-06-15".to_string()) && dates.contains(&"2026-06-20".to_string()),
        "date filter keeps journals on/after 2026-06-12: {dates:?}"
    );
    assert!(
        !dates.contains(&"2026-06-10".to_string()),
        "date_from filter excludes the 2026-06-10 journal: {dates:?}"
    );
    let mut sorted = dates.clone();
    sorted.sort();
    assert_eq!(dates, sorted, "journals returned in ascending journal_date");

    // Saved-view reload proof: the persisted definition decodes back identical.
    let reloaded = store
        .db
        .get_block_view(&workspace_id, &view_id)
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
    let store = embedded_or_skip!();
    let workspace_id = store.create_workspace().await;
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
    let view_id = make_view(&store.db, &workspace_id, "Resortable", definition).await;

    // A header click re-sorts by Created DESC and persists the definition.
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
    store
        .db
        .update_block_view_definition(
            &WriteContext::human(None),
            &workspace_id,
            &view_id,
            new_definition,
        )
        .await
        .expect("update definition");

    let reloaded = store
        .db
        .get_block_view(&workspace_id, &view_id)
        .await
        .expect("reload");
    let sort = reloaded.definition.sort.expect("sort persisted");
    assert!(matches!(sort.field, BlockViewField::Created));
    assert!(matches!(sort.direction, BlockViewSortDirection::Desc));

    // The untagged sentinel is a stable, public contract for empty-tag lanes.
    assert_eq!(BLOCK_VIEW_UNTAGGED_LANE, "__untagged__");
}

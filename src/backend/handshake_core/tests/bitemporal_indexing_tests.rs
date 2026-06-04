//! MT-157 Postgres-backed bitemporal indexing over the real kernel event ledger.
//!
//! The MT-157 contract originally named a nonexistent `memory_item` table. The
//! production persistence surface for FEMS memory is `kernel_event_ledger`, so
//! this test proves the bitemporal index writes typed JSONB events there and
//! keeps replay semantics without introducing a second table.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use handshake_core::{
    memory::bitemporal::{
        AsOfQuery, BitemporalError, BitemporalItem, BitemporalStamps,
        MEMORY_BITEMPORAL_EVENT_SCHEMA_ID, MEMORY_BITEMPORAL_ITEM_AGGREGATE_TYPE,
        MEMORY_BITEMPORAL_SOURCE_COMPONENT, PostgresBitemporalMemoryIndex,
    },
    storage::{Database, StorageError, StorageResult, postgres::PostgresDatabase},
};
use serde_json::json;
use sqlx::{Connection, PgPool, Row};
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires POSTGRES_TEST_URL; run with `cargo test --test bitemporal_indexing_tests -- --ignored`"]
async fn bitemporal_items_persist_to_kernel_event_ledger_jsonb_without_memory_item_table() {
    let (db, pool) = isolated_postgres().await.expect("isolated postgres");
    let index = PostgresBitemporalMemoryIndex::with_db(Arc::clone(&db));

    let stable = item(
        1,
        BitemporalStamps {
            valid_from: at(100),
            valid_until: None,
            recorded_at: at(50),
            invalidated_at: None,
        },
    );
    let invalidated = item(
        2,
        BitemporalStamps {
            valid_from: at(100),
            valid_until: None,
            recorded_at: at(50),
            invalidated_at: None,
        },
    );

    index
        .record_item(stable.clone())
        .await
        .expect("record stable");
    index
        .record_item(invalidated.clone())
        .await
        .expect("record invalidated");
    assert!(
        index
            .invalidate_item(invalidated.item_id, at(200))
            .await
            .expect("invalidate item")
    );

    let before_invalidation = index
        .items_visible_at(&AsOfQuery {
            as_of_world_time: at(150),
            as_of_recorded_time: at(150),
        })
        .await
        .expect("query before invalidation");
    assert_eq!(
        item_ids(before_invalidation),
        vec![stable.item_id, invalidated.item_id]
    );

    let after_invalidation = index
        .items_visible_at(&AsOfQuery {
            as_of_world_time: at(150),
            as_of_recorded_time: at(250),
        })
        .await
        .expect("query after invalidation");
    assert_eq!(item_ids(after_invalidation), vec![stable.item_id]);

    let table_exists: Option<String> =
        sqlx::query_scalar("SELECT to_regclass('memory_item')::text")
            .fetch_one(&pool)
            .await
            .expect("to_regclass");
    assert!(
        table_exists.is_none(),
        "MT-157 must not invent memory_item; got {table_exists:?}"
    );

    let bitemporal_index_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM pg_indexes
        WHERE schemaname = current_schema()
          AND indexname IN (
            'idx_kernel_event_ledger_memory_bitemporal_world',
            'idx_kernel_event_ledger_memory_bitemporal_recorded'
          )
        "#,
    )
    .fetch_one(&pool)
    .await
    .expect("index query");
    assert_eq!(
        bitemporal_index_count, 2,
        "migration must add JSONB expression indexes for both temporal axes"
    );

    let rows = sqlx::query(
        r#"
        SELECT aggregate_type, source_component, payload
        FROM kernel_event_ledger
        WHERE aggregate_type = $1
        ORDER BY event_sequence
        "#,
    )
    .bind(MEMORY_BITEMPORAL_ITEM_AGGREGATE_TYPE)
    .fetch_all(&pool)
    .await
    .expect("ledger rows");
    assert!(
        rows.len() >= 3,
        "record, record, invalidate should create at least three item events"
    );
    for row in rows {
        assert_eq!(
            row.get::<String, _>("aggregate_type"),
            MEMORY_BITEMPORAL_ITEM_AGGREGATE_TYPE
        );
        assert_eq!(
            row.get::<String, _>("source_component"),
            MEMORY_BITEMPORAL_SOURCE_COMPONENT
        );
        let payload: serde_json::Value = row.get("payload");
        assert_eq!(
            payload["schema_id"].as_str(),
            Some(MEMORY_BITEMPORAL_EVENT_SCHEMA_ID)
        );
        assert!(
            payload["item"]["stamps"]["valid_from"].is_string(),
            "payload must carry world-time bitemporal stamps"
        );
        assert!(
            payload["item"]["stamps"]["recorded_at"].is_string(),
            "payload must carry system-time bitemporal stamps"
        );
    }
}

#[tokio::test]
#[ignore = "requires POSTGRES_TEST_URL; run with `cargo test --test bitemporal_indexing_tests -- --ignored`"]
async fn duplicate_records_are_idempotent_and_manifest_replay_deduplicates_visible_items() {
    let (db, pool) = isolated_postgres().await.expect("isolated postgres");
    let index = PostgresBitemporalMemoryIndex::with_db(Arc::clone(&db));
    let item = item(
        10,
        BitemporalStamps {
            valid_from: at(100),
            valid_until: None,
            recorded_at: at(50),
            invalidated_at: None,
        },
    );

    let first = index.record_item(item.clone()).await.expect("first record");
    let duplicate = index
        .record_item(item.clone())
        .await
        .expect("duplicate record");
    assert_eq!(
        first.event_id, duplicate.event_id,
        "duplicate record must resolve to the existing idempotent ledger row"
    );

    let visible = index
        .items_visible_at(&AsOfQuery {
            as_of_world_time: at(150),
            as_of_recorded_time: at(150),
        })
        .await
        .expect("visible items");
    assert_eq!(item_ids(visible), vec![item.item_id]);

    let item_event_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM kernel_event_ledger
        WHERE aggregate_type = $1
          AND aggregate_id = $2
        "#,
    )
    .bind(MEMORY_BITEMPORAL_ITEM_AGGREGATE_TYPE)
    .bind(item.item_id.to_string())
    .fetch_one(&pool)
    .await
    .expect("item event count");
    assert_eq!(
        item_event_count, 1,
        "idempotent duplicate record must not append a second item event"
    );
}

#[tokio::test]
#[ignore = "requires POSTGRES_TEST_URL; run with `cargo test --test bitemporal_indexing_tests -- --ignored`"]
async fn invalid_temporal_windows_are_rejected_before_ledger_append() {
    let (db, pool) = isolated_postgres().await.expect("isolated postgres");
    let index = PostgresBitemporalMemoryIndex::with_db(Arc::clone(&db));
    let invalid_world = item(
        20,
        BitemporalStamps {
            valid_from: at(100),
            valid_until: Some(at(100)),
            recorded_at: at(50),
            invalidated_at: None,
        },
    );
    assert!(matches!(
        index.record_item(invalid_world.clone()).await,
        Err(BitemporalError::InvalidWorldWindow { .. })
    ));

    let valid = item(
        21,
        BitemporalStamps {
            valid_from: at(100),
            valid_until: None,
            recorded_at: at(50),
            invalidated_at: None,
        },
    );
    index
        .record_item(valid.clone())
        .await
        .expect("record valid");
    assert!(matches!(
        index.invalidate_item(valid.item_id, at(50)).await,
        Err(BitemporalError::InvalidSystemWindow { .. })
    ));

    let invalid_world_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM kernel_event_ledger
        WHERE aggregate_type = $1
          AND aggregate_id = $2
        "#,
    )
    .bind(MEMORY_BITEMPORAL_ITEM_AGGREGATE_TYPE)
    .bind(invalid_world.item_id.to_string())
    .fetch_one(&pool)
    .await
    .expect("invalid world count");
    assert_eq!(
        invalid_world_count, 0,
        "invalid world window must not append an item event"
    );

    let valid_item_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM kernel_event_ledger
        WHERE aggregate_type = $1
          AND aggregate_id = $2
        "#,
    )
    .bind(MEMORY_BITEMPORAL_ITEM_AGGREGATE_TYPE)
    .bind(valid.item_id.to_string())
    .fetch_one(&pool)
    .await
    .expect("valid item count");
    assert_eq!(
        valid_item_count, 1,
        "invalid system-window invalidation must not append an invalidation event"
    );
}

#[tokio::test]
#[ignore = "requires POSTGRES_TEST_URL; run with `cargo test --test bitemporal_indexing_tests -- --ignored`"]
async fn as_of_replay_uses_recorded_time_not_latest_event_sequence() {
    let (db, _pool) = isolated_postgres().await.expect("isolated postgres");
    let index = PostgresBitemporalMemoryIndex::with_db(Arc::clone(&db));
    let item_id = Uuid::from_u128(30);

    index
        .record_item(BitemporalItem {
            item_id,
            stamps: BitemporalStamps {
                valid_from: at(100),
                valid_until: None,
                recorded_at: at(50),
                invalidated_at: None,
            },
            payload: json!({"version": "original"}),
        })
        .await
        .expect("record original");
    index
        .record_item(BitemporalItem {
            item_id,
            stamps: BitemporalStamps {
                valid_from: at(100),
                valid_until: None,
                recorded_at: at(300),
                invalidated_at: None,
            },
            payload: json!({"version": "later"}),
        })
        .await
        .expect("record later");

    let before_later_record = index
        .items_visible_at(&AsOfQuery {
            as_of_world_time: at(150),
            as_of_recorded_time: at(150),
        })
        .await
        .expect("query before later record");
    assert_eq!(before_later_record.len(), 1);
    assert_eq!(before_later_record[0].payload["version"], "original");

    let after_later_record = index
        .items_visible_at(&AsOfQuery {
            as_of_world_time: at(150),
            as_of_recorded_time: at(350),
        })
        .await
        .expect("query after later record");
    assert_eq!(after_later_record.len(), 1);
    assert_eq!(after_later_record[0].payload["version"], "later");
}

#[tokio::test]
#[ignore = "requires POSTGRES_TEST_URL; run with `cargo test --test bitemporal_indexing_tests -- --ignored`"]
async fn repeated_invalidation_replay_uses_earliest_effective_invalidation_for_as_of_query() {
    let (db, _pool) = isolated_postgres().await.expect("isolated postgres");
    let index = PostgresBitemporalMemoryIndex::with_db(Arc::clone(&db));
    let item_id = Uuid::from_u128(31);

    index
        .record_item(BitemporalItem {
            item_id,
            stamps: BitemporalStamps {
                valid_from: at(100),
                valid_until: None,
                recorded_at: at(50),
                invalidated_at: None,
            },
            payload: json!({"version": "recorded"}),
        })
        .await
        .expect("record item");

    assert!(
        index
            .invalidate_item(item_id, at(200))
            .await
            .expect("first invalidation")
    );
    assert!(
        index
            .invalidate_item(item_id, at(350))
            .await
            .expect("second invalidation")
    );

    let after_first_before_second_invalidation = index
        .items_visible_at(&AsOfQuery {
            as_of_world_time: at(150),
            as_of_recorded_time: at(250),
        })
        .await
        .expect("query after first invalidation");
    assert!(
        after_first_before_second_invalidation.is_empty(),
        "the first invalidation at recorded time 200 must hide the item even though a later event exists"
    );
}

async fn isolated_postgres() -> StorageResult<(Arc<dyn Database>, PgPool)> {
    let url = std::env::var("POSTGRES_TEST_URL")
        .map_err(|_| StorageError::Validation("POSTGRES_TEST_URL not set for postgres tests"))?;
    let mut conn = sqlx::PgConnection::connect(&url).await?;
    let schema = format!("mt157_bitemporal_{}", Uuid::now_v7().simple());
    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(&mut conn)
        .await?;
    drop(conn);

    let sep = if url.contains('?') { "&" } else { "?" };
    let schema_url = format!("{url}{sep}options=-csearch_path%3D{schema}");
    let pool = PgPool::connect(&schema_url).await?;
    for statement in [
        include_str!("../migrations/0018_kernel_event_ledger.sql"),
        include_str!("../migrations/0029_bitemporal_event_ledger_indexes.sql"),
    ] {
        sqlx::raw_sql(statement).execute(&pool).await?;
    }
    let db = PostgresDatabase::new(pool.clone());
    Ok((db.into_arc(), pool))
}

fn at(secs: i64) -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(secs, 0).unwrap()
}

fn item(id: u128, stamps: BitemporalStamps) -> BitemporalItem {
    BitemporalItem {
        item_id: Uuid::from_u128(id),
        stamps,
        payload: json!({"id": id}),
    }
}

fn item_ids(mut items: Vec<BitemporalItem>) -> Vec<Uuid> {
    items.sort_by_key(|item| item.item_id);
    items.into_iter().map(|item| item.item_id).collect()
}

//! MT-141 embedded SurrealDB successors for the retired bitemporal corpus.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use handshake_core::{
    memory::bitemporal::{
        AsOfQuery, BitemporalError, BitemporalItem, BitemporalStamps, SurrealBitemporalMemoryIndex,
        MEMORY_BITEMPORAL_EVENT_SCHEMA_ID, MEMORY_BITEMPORAL_ITEM_AGGREGATE_TYPE,
        MEMORY_BITEMPORAL_SOURCE_COMPONENT,
    },
    storage::{
        surreal::{bootstrap_schema, SurrealDatabase, SurrealStorage, SurrealStorageConfig},
        tests::{embedded_test_backend, EmbeddedTestBackend},
        Database,
    },
};
use serde_json::json;
use uuid::Uuid;

const RETIRED_BEHAVIOR_MAPPINGS: &[(&str, &str)] = &[
    (
        "bitemporal_items_persist_to_kernel_event_ledger_jsonb_without_memory_item_table",
        "superseded by bitemporal_items_persist_to_embedded_event_ledger_and_replay_after_reopen; the retired backend-only expression indexes are replaced by MT-139 PT-139-2 catalog/no-parallel-table proof",
    ),
    (
        "duplicate_records_are_idempotent_and_manifest_replay_deduplicates_visible_items",
        "superseded by the same-named embedded close/reopen idempotency and manifest-replay test",
    ),
    (
        "invalid_temporal_windows_are_rejected_before_ledger_append",
        "superseded by the same-named typed-error and reopened-ledger row-count test",
    ),
    (
        "as_of_replay_uses_recorded_time_not_latest_event_sequence",
        "superseded by the same-named embedded close/reopen recorded-time ordering test",
    ),
    (
        "repeated_invalidation_replay_uses_earliest_effective_invalidation_for_as_of_query",
        "superseded by the same-named embedded close/reopen repeated-invalidation test",
    ),
];

async fn reopen_backend(backend: &EmbeddedTestBackend) -> (SurrealStorage, Arc<dyn Database>) {
    backend
        .storage
        .shutdown()
        .await
        .expect("close original embedded bitemporal store");
    let storage = SurrealStorage::open(
        SurrealStorageConfig::for_data_dir(&backend.data_dir)
            .expect("configure reopened bitemporal store"),
    )
    .await
    .expect("reopen embedded bitemporal store");
    bootstrap_schema(&storage)
        .await
        .expect("bootstrap reopened bitemporal schema");
    let database: Arc<dyn Database> = Arc::new(SurrealDatabase::new(storage.clone()));
    (storage, database)
}

async fn close_reopened_and_remove(
    storage: SurrealStorage,
    database: Arc<dyn Database>,
    backend: EmbeddedTestBackend,
) {
    drop(database);
    storage
        .shutdown()
        .await
        .expect("close reopened bitemporal store");
    drop(storage);
    backend
        .close_and_remove()
        .await
        .expect("remove embedded bitemporal store");
}

#[test]
fn mt141_bitemporal_retirement_mappings_are_explicit() {
    assert_eq!(RETIRED_BEHAVIOR_MAPPINGS.len(), 5);
    for (retired, successor) in RETIRED_BEHAVIOR_MAPPINGS {
        assert!(!retired.is_empty());
        assert!(successor.starts_with("superseded by"));
    }
}

#[tokio::test]
async fn bitemporal_items_persist_to_embedded_event_ledger_and_replay_after_reopen() {
    let backend = embedded_test_backend()
        .await
        .expect("isolated embedded store");
    let index = SurrealBitemporalMemoryIndex::with_db(Arc::clone(&backend.database));
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
    assert!(index
        .invalidate_item(invalidated.item_id, at(200))
        .await
        .expect("invalidate item"));
    drop(index);

    let (storage, database) = reopen_backend(&backend).await;
    let reopened = SurrealBitemporalMemoryIndex::with_db(Arc::clone(&database));
    assert_eq!(
        item_ids(
            reopened
                .items_visible_at(&AsOfQuery {
                    as_of_world_time: at(150),
                    as_of_recorded_time: at(150),
                })
                .await
                .expect("query before invalidation")
        ),
        vec![stable.item_id, invalidated.item_id]
    );
    assert_eq!(
        item_ids(
            reopened
                .items_visible_at(&AsOfQuery {
                    as_of_world_time: at(150),
                    as_of_recorded_time: at(250),
                })
                .await
                .expect("query after invalidation")
        ),
        vec![stable.item_id]
    );

    let rows = database
        .list_kernel_events_for_aggregate(
            MEMORY_BITEMPORAL_ITEM_AGGREGATE_TYPE,
            &invalidated.item_id.to_string(),
        )
        .await
        .expect("reopened item events");
    assert_eq!(
        rows.len(),
        2,
        "record plus invalidation must survive reopen"
    );
    for row in rows {
        assert_eq!(row.aggregate_type, MEMORY_BITEMPORAL_ITEM_AGGREGATE_TYPE);
        assert_eq!(row.source_component, MEMORY_BITEMPORAL_SOURCE_COMPONENT);
        assert_eq!(
            row.payload["schema_id"].as_str(),
            Some(MEMORY_BITEMPORAL_EVENT_SCHEMA_ID)
        );
        assert!(row.payload["item"]["stamps"]["valid_from"].is_string());
        assert!(row.payload["item"]["stamps"]["recorded_at"].is_string());
    }
    let inspector = storage.test_inspector();
    let table_names = inspector.table_names().await.expect("inspect live catalog");
    assert!(
        !table_names.iter().any(|name| name == "memory_item"),
        "bitemporal replay must not introduce the retired memory_item table"
    );
    let event_catalog = inspector
        .table_catalog("kernel_event_ledger")
        .await
        .expect("inspect EventLedger catalog");
    for required in [
        "idx_kernel_event_ledger_memory_bitemporal_world",
        "idx_kernel_event_ledger_memory_bitemporal_recorded",
        "idx_kernel_event_ledger_memory_bitemporal_manifest",
    ] {
        assert!(
            event_catalog
                .indexes
                .iter()
                .any(|index| index.name == required),
            "missing embedded bitemporal EventLedger index {required}"
        );
    }
    close_reopened_and_remove(storage, database, backend).await;
}

#[tokio::test]
async fn duplicate_records_are_idempotent_and_manifest_replay_deduplicates_visible_items() {
    let backend = embedded_test_backend()
        .await
        .expect("isolated embedded store");
    let index = SurrealBitemporalMemoryIndex::with_db(Arc::clone(&backend.database));
    let recorded = item(
        10,
        BitemporalStamps {
            valid_from: at(100),
            valid_until: None,
            recorded_at: at(50),
            invalidated_at: None,
        },
    );
    let first = index
        .record_item(recorded.clone())
        .await
        .expect("first record");
    let duplicate = index
        .record_item(recorded.clone())
        .await
        .expect("duplicate record");
    assert_eq!(first.event_id, duplicate.event_id);
    drop(index);

    let (storage, database) = reopen_backend(&backend).await;
    let reopened = SurrealBitemporalMemoryIndex::with_db(Arc::clone(&database));
    assert_eq!(
        item_ids(
            reopened
                .items_visible_at(&AsOfQuery {
                    as_of_world_time: at(150),
                    as_of_recorded_time: at(150),
                })
                .await
                .expect("reopened manifest replay")
        ),
        vec![recorded.item_id]
    );
    let events = database
        .list_kernel_events_for_aggregate(
            MEMORY_BITEMPORAL_ITEM_AGGREGATE_TYPE,
            &recorded.item_id.to_string(),
        )
        .await
        .expect("reopened item events");
    assert_eq!(events.len(), 1, "duplicate record must not append twice");
    close_reopened_and_remove(storage, database, backend).await;
}

#[tokio::test]
async fn invalid_temporal_windows_are_rejected_before_ledger_append() {
    let backend = embedded_test_backend()
        .await
        .expect("isolated embedded store");
    let index = SurrealBitemporalMemoryIndex::with_db(Arc::clone(&backend.database));
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
    drop(index);

    let (storage, database) = reopen_backend(&backend).await;
    assert!(database
        .list_kernel_events_for_aggregate(
            MEMORY_BITEMPORAL_ITEM_AGGREGATE_TYPE,
            &invalid_world.item_id.to_string(),
        )
        .await
        .expect("invalid-world event count")
        .is_empty());
    assert_eq!(
        database
            .list_kernel_events_for_aggregate(
                MEMORY_BITEMPORAL_ITEM_AGGREGATE_TYPE,
                &valid.item_id.to_string(),
            )
            .await
            .expect("valid item event count")
            .len(),
        1,
        "invalid invalidation must append no second event"
    );
    close_reopened_and_remove(storage, database, backend).await;
}

#[tokio::test]
async fn as_of_replay_uses_recorded_time_not_latest_event_sequence() {
    let backend = embedded_test_backend()
        .await
        .expect("isolated embedded store");
    let index = SurrealBitemporalMemoryIndex::with_db(Arc::clone(&backend.database));
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
    drop(index);

    let (storage, database) = reopen_backend(&backend).await;
    let reopened = SurrealBitemporalMemoryIndex::with_db(Arc::clone(&database));
    let before = reopened
        .items_visible_at(&AsOfQuery {
            as_of_world_time: at(150),
            as_of_recorded_time: at(150),
        })
        .await
        .expect("query before later record");
    assert_eq!(before.len(), 1);
    assert_eq!(before[0].payload["version"], "original");
    let after = reopened
        .items_visible_at(&AsOfQuery {
            as_of_world_time: at(150),
            as_of_recorded_time: at(350),
        })
        .await
        .expect("query after later record");
    assert_eq!(after.len(), 1);
    assert_eq!(after[0].payload["version"], "later");
    close_reopened_and_remove(storage, database, backend).await;
}

#[tokio::test]
async fn repeated_invalidation_replay_uses_earliest_effective_invalidation_for_as_of_query() {
    let backend = embedded_test_backend()
        .await
        .expect("isolated embedded store");
    let index = SurrealBitemporalMemoryIndex::with_db(Arc::clone(&backend.database));
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
    assert!(index
        .invalidate_item(item_id, at(200))
        .await
        .expect("first invalidation"));
    assert!(index
        .invalidate_item(item_id, at(350))
        .await
        .expect("second invalidation"));
    drop(index);

    let (storage, database) = reopen_backend(&backend).await;
    let reopened = SurrealBitemporalMemoryIndex::with_db(Arc::clone(&database));
    assert!(reopened
        .items_visible_at(&AsOfQuery {
            as_of_world_time: at(150),
            as_of_recorded_time: at(250),
        })
        .await
        .expect("query after first invalidation")
        .is_empty());
    close_reopened_and_remove(storage, database, backend).await;
}

fn at(secs: i64) -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(secs, 0).expect("valid timestamp")
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

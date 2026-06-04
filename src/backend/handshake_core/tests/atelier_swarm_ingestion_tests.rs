//! WP-KERNEL-005 backend readiness proof: parallel swarm agents + mass file
//! ingestion against live PostgreSQL. These tests assert the atelier store is
//! safe under the concurrency a fleet of governed swarm sessions would create:
//! concurrent schema bootstrap (advisory-locked, no CREATE TABLE race), mass
//! concurrent intake at scale, content-hash dedup under contention, and
//! per-(batch, source_path) idempotency under contention.
//!
//! Concurrency is driven with `join_all` so many queries are in flight against
//! the shared pool at once (genuine DB-level contention on the ON CONFLICT
//! paths). Gated on DATABASE_URL; a no-op skip when unset. Storage authority is
//! PostgreSQL only (no SQLite, no Docker, no localhost).

use std::collections::HashSet;

use futures::future::join_all;
use handshake_core::atelier::intake::{NewIntakeBatch, NewIntakeItem};
use handshake_core::atelier::{AtelierStore, NewMediaAsset};
use uuid::Uuid;

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL").ok().filter(|s| !s.is_empty())
}

async fn connected_store(url: &str) -> AtelierStore {
    let store = AtelierStore::connect(url)
        .await
        .expect("connect to PostgreSQL");
    store.ensure_schema().await.expect("ensure atelier schema");
    store
}

/// A swarm of agents racing to bootstrap the schema must never collide on
/// CREATE TABLE; ensure_schema is serialized by a transaction-scoped advisory
/// lock. Every concurrent connect+ensure_schema must succeed.
#[tokio::test]
async fn swarm_concurrent_schema_bootstrap_is_race_free() {
    let Some(url) = database_url() else {
        eprintln!("SKIP swarm_concurrent_schema_bootstrap_is_race_free: DATABASE_URL not set");
        return;
    };

    let futs = (0..16).map(|_| {
        let url = url.clone();
        async move {
            // Independent connection per "agent", as a real swarm would have.
            let store = AtelierStore::connect(&url).await.expect("connect");
            store.ensure_schema().await
        }
    });
    let results = join_all(futs).await;
    assert_eq!(results.len(), 16);
    for r in results {
        r.expect("ensure_schema must succeed under concurrency");
    }
}

/// Mass file ingestion: many concurrent workers add many items into one batch.
/// All items must persist with no duplicate-key panic, and the canonical
/// lane-count must equal the total ingested (no lost or phantom rows).
#[tokio::test]
async fn mass_concurrent_intake_persists_every_item() {
    let Some(url) = database_url() else {
        eprintln!("SKIP mass_concurrent_intake_persists_every_item: DATABASE_URL not set");
        return;
    };
    let store = connected_store(&url).await;

    let run = Uuid::new_v4();
    let batch = store
        .open_intake_batch(&NewIntakeBatch {
            idempotency_key: format!("swarm-mass-{run}"),
            source_label: "swarm mass ingestion".to_string(),
            character_internal_id: None,
        })
        .await
        .expect("open intake batch");

    const WORKERS: usize = 24;
    const PER_WORKER: usize = 50;
    let total = (WORKERS * PER_WORKER) as i64;

    let workers = (0..WORKERS).map(|w| {
        let store = store.clone();
        let batch_id = batch.batch_id;
        async move {
            for i in 0..PER_WORKER {
                // Globally-unique source_path so every add is a distinct item.
                let path = format!("/inbox/{run}/w{w}/i{i}-{}.png", Uuid::new_v4());
                store
                    .add_intake_item(
                        batch_id,
                        &NewIntakeItem {
                            source_path: path.clone(),
                            file_name: format!("i{i}.png"),
                            byte_len: 2048,
                            content_hash: Some(format!("sha256-{}", Uuid::new_v4())),
                        },
                    )
                    .await
                    .expect("add intake item under concurrency");
            }
        }
    });
    join_all(workers).await;

    let counts = store
        .intake_lane_counts(batch.batch_id)
        .await
        .expect("lane counts");
    assert_eq!(
        counts.new, total,
        "every concurrently-ingested item must land in the New lane exactly once"
    );
}

/// Content-hash dedup under contention: many workers materialize the SAME
/// content hash at once. The store must collapse them to a single asset (ON
/// CONFLICT), returning one stable asset_id and never raising a duplicate-key
/// error. This is the mass-ingestion dedup guarantee for a swarm pulling the
/// same bytes from many sources.
#[tokio::test]
async fn concurrent_identical_hash_dedups_to_one_asset() {
    let Some(url) = database_url() else {
        eprintln!("SKIP concurrent_identical_hash_dedups_to_one_asset: DATABASE_URL not set");
        return;
    };
    let store = connected_store(&url).await;

    let shared_hash = format!("sha256-{}", Uuid::new_v4());

    let futs = (0..20).map(|_| {
        let store = store.clone();
        let hash = shared_hash.clone();
        async move {
            store
                .materialize_media_asset(&NewMediaAsset {
                    content_hash: hash.clone(),
                    mime: "image/png".to_string(),
                    byte_len: 4096,
                    source_provenance: Some("swarm-dedup".to_string()),
                    artifact_ref: format!("artifact://atelier/media/{}", Uuid::new_v4()),
                })
                .await
                .expect("materialize under concurrency must not raise duplicate-key")
                .asset_id
        }
    });

    let ids: HashSet<Uuid> = join_all(futs).await.into_iter().collect();
    assert_eq!(
        ids.len(),
        1,
        "20 concurrent identical-hash materializations must collapse to ONE asset_id, got {ids:?}"
    );

    let fetched = store
        .get_media_asset_by_hash(&shared_hash)
        .await
        .expect("query asset by content hash")
        .expect("asset must be retrievable by its content hash");
    assert_eq!(
        fetched.asset_id,
        *ids.iter().next().unwrap(),
        "the deduped asset_id must match the one queryable by content hash"
    );
}

/// Idempotency under contention: many workers add the SAME (batch, source_path)
/// at once. The store must return one stable item_id with no duplicate row,
/// proving a swarm re-scanning the same source path cannot fork the item.
#[tokio::test]
async fn concurrent_same_source_path_is_idempotent() {
    let Some(url) = database_url() else {
        eprintln!("SKIP concurrent_same_source_path_is_idempotent: DATABASE_URL not set");
        return;
    };
    let store = connected_store(&url).await;

    let run = Uuid::new_v4();
    let batch = store
        .open_intake_batch(&NewIntakeBatch {
            idempotency_key: format!("swarm-idem-{run}"),
            source_label: "swarm idempotency".to_string(),
            character_internal_id: None,
        })
        .await
        .expect("open intake batch");
    let shared_path = format!("/inbox/{run}/contended.png");

    let futs = (0..16).map(|_| {
        let store = store.clone();
        let batch_id = batch.batch_id;
        let path = shared_path.clone();
        async move {
            store
                .add_intake_item(
                    batch_id,
                    &NewIntakeItem {
                        source_path: path.clone(),
                        file_name: "contended.png".to_string(),
                        byte_len: 1000,
                        content_hash: None,
                    },
                )
                .await
                .expect("contended add must not raise duplicate-key")
                .item_id
        }
    });

    let ids: HashSet<Uuid> = join_all(futs).await.into_iter().collect();
    assert_eq!(
        ids.len(),
        1,
        "16 concurrent same-(batch, source_path) adds must yield ONE item_id, got {ids:?}"
    );

    let counts = store
        .intake_lane_counts(batch.batch_id)
        .await
        .expect("lane counts");
    assert_eq!(
        counts.new, 1,
        "only one item may exist for the contended source_path"
    );
}

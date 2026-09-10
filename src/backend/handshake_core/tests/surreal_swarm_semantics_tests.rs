#![cfg(all(feature = "surreal-test-support", feature = "test-utils"))]
//! MT-142 "Harden SurrealDB swarm concurrency and load" - same-record,
//! disjoint-record, independent-client, deadlock and lock-registry semantics
//! against the real embedded RocksDB store.
//!
//! Proves AC-142-4, AC-142-8, AC-142-10 (PT-142-4, PT-142-6, PT-142-9):
//! * `same_record_expected_version_race_has_one_winner`
//! * `identical_idempotency_key_replays_converge_to_one_effect`
//! * `disjoint_records_commit_concurrently_without_unrelated_waiting`
//! * `independent_clients_without_shared_lock_registry_stay_correct`
//! * `opposite_order_multi_record_operations_do_not_deadlock`
//! * `keyed_lock_registry_reclaims_after_high_cardinality_churn`
//!
//! Every task is joined, every wait is bounded, no sleep is used for
//! synchronisation (tokio `Barrier` + `timeout` only), and no test accepts
//! last-writer-wins where the API promises optimistic concurrency.

#[path = "swarm_support/mod.rs"]
mod swarm_support;

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use handshake_core::storage::knowledge::{
    KnowledgeIdempotentWrite, KnowledgeRichDocument, KnowledgeStore,
    UpsertKnowledgeDocumentBacklink,
};
use handshake_core::storage::surreal::keyed_lock::{KeyedLockRegistry, LockKey, LockMode};
use handshake_core::storage::surreal::SurrealDatabase;
use handshake_core::storage::StorageError;
use serde_json::json;
use swarm_support::*;
use tokio::sync::Barrier;
use tokio::time::timeout;

/// Racers per same-record proof (contract: N >= 8).
const RACERS: usize = 12;
/// Disjoint documents per overlap proof (contract: 16 workers).
const DISJOINT_DOCUMENTS: usize = 16;
/// An engine-side statement must start at least this long before another
/// operation returns to count as overlapping inside the engine.
const OVERLAP_MARGIN: Duration = Duration::from_millis(2);
const RACE_BOUND: Duration = Duration::from_secs(30);

fn utc_after(instant: DateTime<Utc>, margin: Duration) -> DateTime<Utc> {
    instant + chrono::Duration::from_std(margin).expect("margin fits chrono")
}

/// Barrier-aligned `save_knowledge_rich_document_version` calls with the SAME
/// `expected_version`; client `i` is `clients[i % clients.len()]`.
async fn race_same_record_saves(
    clients: &[SurrealDatabase],
    rich_document_id: &str,
    expected_version: i64,
    racers: usize,
) -> Vec<Result<KnowledgeRichDocument, StorageError>> {
    let barrier = Arc::new(Barrier::new(racers));
    let mut tasks = Vec::with_capacity(racers);
    for racer in 0..racers {
        let db = clients[racer % clients.len()].clone();
        let barrier = Arc::clone(&barrier);
        let rich_document_id = rich_document_id.to_owned();
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;
            timeout(
                PER_OPERATION_TIMEOUT,
                db.save_knowledge_rich_document_version(
                    &rich_document_id,
                    expected_version,
                    document_content(&format!("racer {racer} expected {expected_version}")),
                    None,
                    None,
                    None,
                ),
            )
            .await
            .unwrap_or_else(|_| {
                panic!("racer {racer}: save exceeded the {} ms per-operation bound (hang)", PER_OPERATION_TIMEOUT.as_millis())
            })
        }));
    }
    let mut outcomes = Vec::with_capacity(racers);
    for task in tasks {
        outcomes.push(task.await.expect("racer task joined"));
    }
    outcomes
}

/// Exactly one winner at `expected_version + 1`; every loser is a typed
/// stale/conflict outcome (never last-writer-wins, never a raw engine error).
fn assert_one_winner(
    outcomes: &[Result<KnowledgeRichDocument, StorageError>],
    expected_version: i64,
    context: &str,
) -> KnowledgeRichDocument {
    let winners: Vec<&KnowledgeRichDocument> = outcomes.iter().filter_map(|o| o.as_ref().ok()).collect();
    let losers: Vec<&StorageError> = outcomes.iter().filter_map(|o| o.as_ref().err()).collect();
    assert_eq!(
        winners.len(),
        1,
        "{context}: exactly one same-record save with expected_version {expected_version} may commit; winners={} losers={} outcomes={outcomes:?}",
        winners.len(),
        losers.len()
    );
    assert_eq!(losers.len(), outcomes.len() - 1, "{context}: every non-winner is a loser");
    for loser in &losers {
        assert!(
            !is_untyped_engine_conflict(loser),
            "{context}: a loser surfaced a raw engine conflict instead of the typed stale outcome: {loser}"
        );
        assert!(
            is_typed_conflict(loser),
            "{context}: a loser must be StorageError::Conflict/ConflictDetails (HSK-KRD-SAVE-STALE), got: {loser}"
        );
        let text = loser.to_string().to_ascii_lowercase();
        assert!(
            text.contains("stale") || text.contains("conflict"),
            "{context}: loser conflict must name the stale expected_version, got: {loser}"
        );
    }
    let winner = winners[0].clone();
    assert_eq!(
        winner.doc_version,
        expected_version + 1,
        "{context}: the winner's doc_version must be expected_version + 1"
    );
    winner
}

async fn assert_single_new_version(
    db: &SurrealDatabase,
    rich_document_id: &str,
    expected_versions: &[i64],
    winner_sha256: &str,
    context: &str,
) {
    let versions = db
        .list_knowledge_rich_document_versions(rich_document_id)
        .await
        .expect("list versions");
    let observed: Vec<i64> = versions.iter().map(|v| v.doc_version).collect();
    assert_eq!(
        observed, expected_versions,
        "{context}: exactly one new version row must exist after the race"
    );
    let head = versions.last().expect("head version row");
    assert_eq!(
        head.content_sha256, winner_sha256,
        "{context}: the head version row must carry the winner's content"
    );
    let live = db
        .get_knowledge_rich_document(rich_document_id)
        .await
        .expect("read live document")
        .expect("document is live");
    assert_eq!(
        live.doc_version,
        *expected_versions.last().expect("non-empty"),
        "{context}: the document head must equal the highest version row"
    );
    assert_eq!(
        live.content_sha256, winner_sha256,
        "{context}: the live document must carry the winner's content"
    );
}

/// Barrier-aligned idempotent saves with an identical key and payload.
async fn race_idempotent_saves(
    clients: &[SurrealDatabase],
    idempotency_key: &str,
    rich_document_id: &str,
    expected_version: i64,
    racers: usize,
) -> Vec<Result<KnowledgeIdempotentWrite<KnowledgeRichDocument>, StorageError>> {
    let barrier = Arc::new(Barrier::new(racers));
    let payload = document_content(&format!("idempotent payload for {idempotency_key}"));
    let mut tasks = Vec::with_capacity(racers);
    for racer in 0..racers {
        let db = clients[racer % clients.len()].clone();
        let barrier = Arc::clone(&barrier);
        let key = idempotency_key.to_owned();
        let rich_document_id = rich_document_id.to_owned();
        let payload = payload.clone();
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;
            timeout(
                PER_OPERATION_TIMEOUT,
                db.save_knowledge_rich_document_version_idempotent(
                    &key,
                    &rich_document_id,
                    expected_version,
                    payload,
                    None,
                    None,
                    None,
                ),
            )
            .await
            .unwrap_or_else(|_| {
                panic!("idempotent racer {racer}: save exceeded the {} ms per-operation bound (hang)", PER_OPERATION_TIMEOUT.as_millis())
            })
        }));
    }
    let mut outcomes = Vec::with_capacity(racers);
    for task in tasks {
        outcomes.push(task.await.expect("idempotent racer task joined"));
    }
    outcomes
}

/// Every replay returns the same result identity and exactly one call
/// performed the durable effect.
fn assert_one_effect(
    outcomes: &[Result<KnowledgeIdempotentWrite<KnowledgeRichDocument>, StorageError>],
    expected_version: i64,
    context: &str,
) -> EffectIdentity {
    for (index, outcome) in outcomes.iter().enumerate() {
        assert!(
            outcome.is_ok(),
            "{context}: identical-key replay {index} must converge to the first effect, got error: {:?}",
            outcome.as_ref().err()
        );
    }
    let identities: Vec<EffectIdentity> = outcomes
        .iter()
        .map(|o| EffectIdentity::of(&o.as_ref().expect("checked").value))
        .collect();
    let first = identities[0].clone();
    for (index, identity) in identities.iter().enumerate() {
        assert_eq!(
            identity, &first,
            "{context}: replay {index} returned a different result identity than the first effect"
        );
    }
    assert_eq!(
        first.doc_version,
        expected_version + 1,
        "{context}: the single effect must land at expected_version + 1"
    );
    let effects = outcomes
        .iter()
        .filter(|o| !o.as_ref().expect("checked").replayed)
        .count();
    assert_eq!(
        effects, 1,
        "{context}: exactly one call may report replayed=false (the durable effect); {effects} did"
    );
    first
}

async fn assert_one_receipt_row(store: &EmbeddedKnowledgeStore, idempotency_key: &str, context: &str) {
    let inspector = store.storage.test_inspector();
    let keys = inspector
        .table_selector("knowledge_idempotency_keys")
        .await
        .expect("idempotency table selector");
    let rows = inspector
        .row_count(
            &keys,
            handshake_core::storage::surreal::RowFilter::IdEquals(idempotency_key.to_owned()),
        )
        .await
        .expect("count receipt rows");
    assert_eq!(
        rows, 1,
        "{context}: exactly one idempotency receipt row may exist for key {idempotency_key}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn same_record_expected_version_race_has_one_winner() {
    let seed = workload_seed();
    println!("SWARM_SEED={seed}");
    let store = open_embedded_store()
        .await
        .expect("MT-142 requires the embedded SurrealDB test store");
    let workspace_id = store.create_workspace().await;
    let created = store
        .db
        .create_knowledge_rich_document(new_document(&workspace_id, "cas race", "base"))
        .await
        .expect("create document");

    let outcomes = timeout(
        RACE_BOUND,
        race_same_record_saves(
            &[store.db.clone()],
            &created.rich_document_id,
            created.doc_version,
            RACERS,
        ),
    )
    .await
    .expect("same-record race must finish inside its bound (no hang)");
    let winner = assert_one_winner(&outcomes, created.doc_version, "same-record CAS race");
    assert_single_new_version(
        &store.db,
        &created.rich_document_id,
        &[1, 2],
        &winner.content_sha256,
        "same-record CAS race",
    )
    .await;

    // A second round from the new head proves the semantics hold after a
    // race, not only on a fresh document.
    let outcomes = timeout(
        RACE_BOUND,
        race_same_record_saves(&[store.db.clone()], &created.rich_document_id, 2, RACERS),
    )
    .await
    .expect("second same-record race must finish inside its bound");
    let winner = assert_one_winner(&outcomes, 2, "same-record CAS race round 2");
    assert_single_new_version(
        &store.db,
        &created.rich_document_id,
        &[1, 2, 3],
        &winner.content_sha256,
        "same-record CAS race round 2",
    )
    .await;

    store.close_and_remove().await.expect("close store");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn identical_idempotency_key_replays_converge_to_one_effect() {
    let seed = workload_seed();
    println!("SWARM_SEED={seed}");
    let store = open_embedded_store()
        .await
        .expect("MT-142 requires the embedded SurrealDB test store");
    let workspace_id = store.create_workspace().await;
    let created = store
        .db
        .create_knowledge_rich_document(new_document(&workspace_id, "idempotent replay", "base"))
        .await
        .expect("create document");
    let key = format!("mt142-idem-{}", uuid::Uuid::now_v7().simple());

    let outcomes = timeout(
        RACE_BOUND,
        race_idempotent_saves(
            &[store.db.clone()],
            &key,
            &created.rich_document_id,
            created.doc_version,
            RACERS,
        ),
    )
    .await
    .expect("idempotent race must finish inside its bound (no hang)");
    let effect = assert_one_effect(&outcomes, created.doc_version, "identical-key replay race");
    assert_single_new_version(
        &store.db,
        &created.rich_document_id,
        &[1, 2],
        &effect.content_sha256,
        "identical-key replay race",
    )
    .await;
    assert_one_receipt_row(&store, &key, "identical-key replay race").await;

    // A later sequential replay of the same key still converges without a
    // second effect, even though the document head moved on.
    let replay = store
        .db
        .save_knowledge_rich_document_version_idempotent(
            &key,
            &created.rich_document_id,
            created.doc_version,
            document_content(&format!("idempotent payload for {key}")),
            None,
            None,
            None,
        )
        .await
        .expect("sequential replay converges");
    assert!(replay.replayed, "sequential replay must report replayed=true");
    assert_eq!(EffectIdentity::of(&replay.value), effect, "sequential replay identity");
    assert_single_new_version(
        &store.db,
        &created.rich_document_id,
        &[1, 2],
        &effect.content_sha256,
        "identical-key sequential replay",
    )
    .await;
    assert_one_receipt_row(&store, &key, "identical-key sequential replay").await;

    store.close_and_remove().await.expect("close store");
}

/// One barrier-aligned save with its call window and the engine-side
/// statement time (`updated_at = time::now()` evaluated inside the
/// transaction).
#[derive(Clone, Debug)]
struct SaveWindow {
    rich_document_id: String,
    start: DateTime<Utc>,
    engine_at: DateTime<Utc>,
    end: DateTime<Utc>,
    latency: Duration,
}

/// Maximum number of operations provably inside the engine at once: an
/// operation `b` was executing at `b.engine_at`; every other operation `a`
/// whose window still had at least `OVERLAP_MARGIN` to run at that instant
/// was in flight with it. A process-global lock makes every `engine_at`
/// follow the previous operation's return, so this stays at 1.
fn engine_concurrency(windows: &[SaveWindow]) -> (usize, usize) {
    let mut max_concurrent = 0usize;
    let mut overlapping_pairs = 0usize;
    for b in windows {
        let mut concurrent = 1usize;
        for a in windows {
            if std::ptr::eq(a, b) {
                continue;
            }
            if a.start <= b.engine_at && utc_after(b.engine_at, OVERLAP_MARGIN) <= a.end {
                concurrent += 1;
                overlapping_pairs += 1;
            }
        }
        max_concurrent = max_concurrent.max(concurrent);
    }
    (max_concurrent, overlapping_pairs)
}

async fn barrier_aligned_saves(
    db: &SurrealDatabase,
    documents: &[KnowledgeRichDocument],
    gauge: &Arc<InFlightGauge>,
) -> Vec<SaveWindow> {
    let barrier = Arc::new(Barrier::new(documents.len()));
    let mut tasks = Vec::with_capacity(documents.len());
    for (index, document) in documents.iter().enumerate() {
        let db = db.clone();
        let barrier = Arc::clone(&barrier);
        let gauge = Arc::clone(gauge);
        let rich_document_id = document.rich_document_id.clone();
        let expected_version = document.doc_version;
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;
            let _in_flight = gauge.enter();
            let start = Utc::now();
            let started = Instant::now();
            let saved = timeout(
                PER_OPERATION_TIMEOUT,
                db.save_knowledge_rich_document_version(
                    &rich_document_id,
                    expected_version,
                    document_content(&format!("disjoint save {index}")),
                    None,
                    None,
                    None,
                ),
            )
            .await
            .unwrap_or_else(|_| panic!("disjoint save {index} exceeded its per-operation bound (hang)"))
            .unwrap_or_else(|error| {
                panic!("disjoint save {index} on {rich_document_id} must commit (no unrelated waiting or conflict), got: {error}")
            });
            let end = Utc::now();
            SaveWindow {
                rich_document_id,
                start,
                engine_at: saved.updated_at,
                end,
                latency: started.elapsed(),
            }
        }));
    }
    let mut windows = Vec::with_capacity(documents.len());
    for task in tasks {
        windows.push(task.await.expect("disjoint save task joined"));
    }
    windows
}

async fn create_documents(
    db: &SurrealDatabase,
    workspace_id: &str,
    prefix: &str,
    count: usize,
) -> Vec<KnowledgeRichDocument> {
    let mut documents = Vec::with_capacity(count);
    for index in 0..count {
        documents.push(
            db.create_knowledge_rich_document(new_document(
                workspace_id,
                &format!("{prefix} {index}"),
                &format!("{prefix} base {index}"),
            ))
            .await
            .expect("create disjoint document"),
        );
    }
    documents
}

fn assert_overlap(windows: &[SaveWindow], context: &str) -> (usize, usize) {
    let (max_concurrent, overlapping_pairs) = engine_concurrency(windows);
    let latencies: Vec<u128> = windows.iter().map(|w| w.latency.as_millis()).collect();
    println!(
        "SWARM_OVERLAP context={context} max_concurrent_in_engine={max_concurrent} overlapping_pairs={overlapping_pairs} latencies_ms={latencies:?}"
    );
    assert!(
        max_concurrent >= 2,
        "{context}: disjoint-record writes must be observably concurrent inside the engine (maximum concurrent in-flight writes >= 2), but every engine-side statement started only after the previous operation returned (max_concurrent={max_concurrent}, overlapping_pairs={overlapping_pairs}); this is the global-serialisation signature of RICH_DOCUMENT_MUTATION_LOCK. windows={windows:?}"
    );
    (max_concurrent, overlapping_pairs)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn disjoint_records_commit_concurrently_without_unrelated_waiting() {
    let seed = workload_seed();
    println!("SWARM_SEED={seed}");
    let store = open_embedded_store()
        .await
        .expect("MT-142 requires the embedded SurrealDB test store");
    let workspace_a = store.create_workspace().await;
    let workspace_b = store.create_workspace().await;
    let gauge = Arc::new(InFlightGauge::default());

    // Same workspace, 16 different documents.
    let same_workspace = create_documents(&store.db, &workspace_a, "same-ws", DISJOINT_DOCUMENTS).await;
    let windows = timeout(
        RACE_BOUND,
        barrier_aligned_saves(&store.db, &same_workspace, &gauge),
    )
    .await
    .expect("same-workspace disjoint saves must finish inside their bound");
    assert_eq!(windows.len(), DISJOINT_DOCUMENTS, "every disjoint save committed");
    assert!(
        gauge.high_water() >= 2,
        "call-boundary in-flight high-water mark must be >= 2, got {}",
        gauge.high_water()
    );
    assert_overlap(&windows, "same workspace, 16 disjoint documents");
    assert_eq!(
        store.db.lock_registry().entry_count(),
        0,
        "no disjoint-record write may leave a keyed-lock entry behind"
    );
    for (index, document) in same_workspace.iter().enumerate() {
        let versions = store
            .db
            .list_knowledge_rich_document_versions(&document.rich_document_id)
            .await
            .expect("list versions");
        assert_eq!(
            versions.iter().map(|v| v.doc_version).collect::<Vec<_>>(),
            vec![1, 2],
            "disjoint document {index} must have exactly its acknowledged version chain"
        );
    }

    // Two workspaces, 8 + 8 different documents, driven through a wrapper
    // whose keyed-lock registry is DISABLED: no operation can wait on a
    // process-local lock, so the observed overlap is the engine's own.
    let unlocked = SurrealDatabase::with_lock_registry(
        store.storage.clone(),
        KeyedLockRegistry::disabled(),
    );
    let mut cross_workspace = create_documents(&unlocked, &workspace_a, "ws-a", DISJOINT_DOCUMENTS / 2).await;
    cross_workspace.extend(create_documents(&unlocked, &workspace_b, "ws-b", DISJOINT_DOCUMENTS / 2).await);
    let windows = timeout(
        RACE_BOUND,
        barrier_aligned_saves(&unlocked, &cross_workspace, &gauge),
    )
    .await
    .expect("cross-workspace disjoint saves must finish inside their bound");
    assert_eq!(windows.len(), DISJOINT_DOCUMENTS, "every cross-workspace save committed");
    assert_overlap(&windows, "two workspaces, 8 + 8 disjoint documents, registry disabled");
    let ids: std::collections::BTreeSet<&str> = windows.iter().map(|w| w.rich_document_id.as_str()).collect();
    assert_eq!(ids.len(), DISJOINT_DOCUMENTS, "every save targeted a distinct record");
    assert_eq!(
        unlocked.lock_registry().entry_count(),
        0,
        "a disabled registry never holds entries (lock_wait == 0 for every operation)"
    );

    // The same cross-workspace set again (at its new heads) through the keyed
    // wrapper: disjoint keys never wait on each other, so the overlap must be
    // identical in kind.
    let mut cross_workspace_heads = Vec::with_capacity(cross_workspace.len());
    for document in &cross_workspace {
        cross_workspace_heads.push(
            store
                .db
                .get_knowledge_rich_document(&document.rich_document_id)
                .await
                .expect("re-read cross-workspace document")
                .expect("cross-workspace document is live"),
        );
    }
    let windows = timeout(
        RACE_BOUND,
        barrier_aligned_saves(&store.db, &cross_workspace_heads, &gauge),
    )
    .await
    .expect("keyed cross-workspace disjoint saves must finish inside their bound");
    assert_overlap(&windows, "two workspaces, 8 + 8 disjoint documents, registry keyed");
    assert_eq!(
        store.db.lock_registry().entry_count(),
        0,
        "keyed registry must be idle once the disjoint saves returned"
    );

    store.close_and_remove().await.expect("close store");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn independent_clients_without_shared_lock_registry_stay_correct() {
    let seed = workload_seed();
    println!("SWARM_SEED={seed}");
    let store = open_embedded_store()
        .await
        .expect("MT-142 requires the embedded SurrealDB test store");
    let workspace_id = store.create_workspace().await;

    // Two wrappers over ONE embedded engine that do NOT share a keyed-lock
    // registry: client A owns a fresh keyed registry, client B runs with the
    // registry DISABLED (no process-local lock at all), so only the database
    // transactions and guards can keep the pair correct.
    let client_a = SurrealDatabase::new(store.storage.clone());
    let client_b = SurrealDatabase::with_lock_registry(
        store.storage.clone(),
        KeyedLockRegistry::disabled(),
    );
    assert_eq!(client_b.lock_registry().mode(), LockMode::Disabled);
    let clients = [client_a.clone(), client_b.clone()];

    let created = client_a
        .create_knowledge_rich_document(new_document(&workspace_id, "independent clients", "base"))
        .await
        .expect("create through client A");
    let outcomes = timeout(
        RACE_BOUND,
        race_same_record_saves(&clients, &created.rich_document_id, created.doc_version, RACERS),
    )
    .await
    .expect("independent-client CAS race must finish inside its bound");
    let winner = assert_one_winner(&outcomes, created.doc_version, "independent-client CAS race");
    assert_single_new_version(
        &client_b,
        &created.rich_document_id,
        &[1, 2],
        &winner.content_sha256,
        "independent-client CAS race (read through client B)",
    )
    .await;
    let cas_losers = outcomes.iter().filter(|o| o.is_err()).count();

    let replay_doc = client_b
        .create_knowledge_rich_document(new_document(&workspace_id, "independent replay", "base"))
        .await
        .expect("create through client B");
    let key = format!("mt142-independent-{}", uuid::Uuid::now_v7().simple());
    let outcomes = timeout(
        RACE_BOUND,
        race_idempotent_saves(&clients, &key, &replay_doc.rich_document_id, replay_doc.doc_version, RACERS),
    )
    .await
    .expect("independent-client idempotent race must finish inside its bound");
    let effect = assert_one_effect(&outcomes, replay_doc.doc_version, "independent-client replay race");
    assert_single_new_version(
        &client_a,
        &replay_doc.rich_document_id,
        &[1, 2],
        &effect.content_sha256,
        "independent-client replay race (read through client A)",
    )
    .await;
    assert_one_receipt_row(&store, &key, "independent-client replay race").await;
    assert_eq!(
        client_a.lock_registry().entry_count(),
        0,
        "client A's registry must be idle after the races"
    );
    assert_eq!(
        client_b.lock_registry().entry_count(),
        0,
        "client B runs without locks and never holds entries"
    );

    let run_id = new_run_id("mt142-independent-clients");
    let fragment = json!({
        "schema_id": REPORT_FRAGMENT_SCHEMA_ID,
        "fragment": "independent_clients",
        "run_id": run_id,
        "source_commit": source_commit(),
        "surrealdb_version": SURREALDB_VERSION,
        "engine_mode": "embedded_rocks_db",
        "workload_seed": seed,
        "clients": 2,
        "client_construction": "SurrealDatabase::new (own keyed registry) and SurrealDatabase::with_lock_registry(storage.clone(), KeyedLockRegistry::disabled()) over one embedded engine",
        "client_lock_modes": ["keyed", "disabled"],
        "shared_keyed_lock_registry": false,
        "same_record_cas_race": {
            "racers": RACERS,
            "winners": 1,
            "typed_stale_losers": cas_losers,
            "final_doc_version": winner.doc_version,
        },
        "idempotency_replay_race": {
            "racers": RACERS,
            "durable_effects": 1,
            "effect": {
                "rich_document_id": effect.rich_document_id,
                "doc_version": effect.doc_version,
                "content_sha256": effect.content_sha256,
            },
        },
        "remote_proof_status": "not_run_unconfigured",
        "remote_proof_note": "no managed remote SurrealDB endpoint is configured; embedded proof only, never reported as remote PASS",
    });
    let path = write_report_json(&format!("swarm-independent-clients-{run_id}.json"), &fragment);
    println!("SWARM_INDEPENDENT_CLIENTS_REPORT={}", path.display());
    assert_eq!(
        fragment["remote_proof_status"], "not_run_unconfigured",
        "an unconfigured remote proof is NOT_RUN_UNCONFIGURED, never PASS"
    );

    store.close_and_remove().await.expect("close store");
}

fn backlink(
    workspace_id: &str,
    source: &KnowledgeRichDocument,
    target: &KnowledgeRichDocument,
    label: &str,
) -> UpsertKnowledgeDocumentBacklink {
    UpsertKnowledgeDocumentBacklink {
        workspace_id: workspace_id.to_owned(),
        relationship_id: format!("KDLNK-mt142-{label}"),
        source_document_id: source.rich_document_id.clone(),
        link_kind: "wikilink".to_owned(),
        target: target.rich_document_id.clone(),
        block_id: format!("blk-mt142-{label}"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn opposite_order_multi_record_operations_do_not_deadlock() {
    const ITERATIONS: usize = 10;
    const DEADLOCK_BOUND: Duration = Duration::from_secs(60);

    // (1) The registry primitive: two tasks take the same two keys in
    // opposite input order through `acquire_many` (sorted acquisition).
    let registry = KeyedLockRegistry::keyed();
    let key_a = LockKey::record("knowledge_rich_documents", "KRD-A");
    let key_b = LockKey::record("knowledge_rich_documents", "KRD-B");
    let forward = {
        let registry = registry.clone();
        let (a, b) = (key_a.clone(), key_b.clone());
        tokio::spawn(async move {
            for _ in 0..500 {
                let guards = registry.acquire_many(vec![a.clone(), b.clone()]).await;
                assert_eq!(guards.len(), 2, "forward order acquired both keys");
                tokio::task::yield_now().await;
            }
        })
    };
    let reverse = {
        let registry = registry.clone();
        let (a, b) = (key_a.clone(), key_b.clone());
        tokio::spawn(async move {
            for _ in 0..500 {
                let guards = registry.acquire_many(vec![b.clone(), a.clone()]).await;
                assert_eq!(guards.len(), 2, "reverse order acquired both keys");
                tokio::task::yield_now().await;
            }
        })
    };
    timeout(DEADLOCK_BOUND, async {
        forward.await.expect("forward registry task");
        reverse.await.expect("reverse registry task");
    })
    .await
    .expect("opposite-order acquire_many deadlocked: neither task completed inside the bound");
    assert_eq!(registry.entry_count(), 0, "registry must be idle after the opposite-order churn");

    // (2) The public multi-record product operation: rebuilding A's backlinks
    // touches loom_blocks A and B (edge + count recompute) and B's rebuild
    // touches the same two records in the opposite order.
    let store = open_embedded_store()
        .await
        .expect("MT-142 requires the embedded SurrealDB test store");
    let workspace_id = store.create_workspace().await;
    let doc_a = store
        .db
        .create_knowledge_rich_document(new_document(&workspace_id, "deadlock A", "a"))
        .await
        .expect("create A");
    let doc_b = store
        .db
        .create_knowledge_rich_document(new_document(&workspace_id, "deadlock B", "b"))
        .await
        .expect("create B");

    let barrier = Arc::new(Barrier::new(2));
    let a_to_b = {
        let db = store.db.clone();
        let barrier = Arc::clone(&barrier);
        let (workspace_id, doc_a, doc_b) = (workspace_id.clone(), doc_a.clone(), doc_b.clone());
        tokio::spawn(async move {
            barrier.wait().await;
            for iteration in 0..ITERATIONS {
                let result = timeout(
                    PER_OPERATION_TIMEOUT,
                    db.replace_knowledge_document_backlinks(
                        &doc_a.rich_document_id,
                        vec![backlink(&workspace_id, &doc_a, &doc_b, "a-to-b")],
                    ),
                )
                .await
                .unwrap_or_else(|_| panic!("A->B rebuild {iteration} exceeded its per-operation bound (hang)"));
                result.unwrap_or_else(|error| {
                    panic!("A->B backlink rebuild {iteration} must converge (idempotent multi-record rebuild), got: {error}")
                });
            }
        })
    };
    let b_to_a = {
        let db = store.db.clone();
        let barrier = Arc::clone(&barrier);
        let (workspace_id, doc_a, doc_b) = (workspace_id.clone(), doc_a.clone(), doc_b.clone());
        tokio::spawn(async move {
            barrier.wait().await;
            for iteration in 0..ITERATIONS {
                let result = timeout(
                    PER_OPERATION_TIMEOUT,
                    db.replace_knowledge_document_backlinks(
                        &doc_b.rich_document_id,
                        vec![backlink(&workspace_id, &doc_b, &doc_a, "b-to-a")],
                    ),
                )
                .await
                .unwrap_or_else(|_| panic!("B->A rebuild {iteration} exceeded its per-operation bound (hang)"));
                result.unwrap_or_else(|error| {
                    panic!("B->A backlink rebuild {iteration} must converge (idempotent multi-record rebuild), got: {error}")
                });
            }
        })
    };
    timeout(DEADLOCK_BOUND, async {
        a_to_b.await.expect("A->B task");
        b_to_a.await.expect("B->A task");
    })
    .await
    .expect("opposite-order backlink rebuilds deadlocked: a task did not complete inside the bound");

    let from_a = store
        .db
        .list_knowledge_document_backlinks_from(&doc_a.rich_document_id)
        .await
        .expect("backlinks from A");
    let from_b = store
        .db
        .list_knowledge_document_backlinks_from(&doc_b.rich_document_id)
        .await
        .expect("backlinks from B");
    assert_eq!(from_a.len(), 1, "A keeps exactly its one rebuilt backlink");
    assert_eq!(from_b.len(), 1, "B keeps exactly its one rebuilt backlink");
    assert_eq!(from_a[0].target, doc_b.rich_document_id, "A links to B");
    assert_eq!(from_b[0].target, doc_a.rich_document_id, "B links to A");

    store.close_and_remove().await.expect("close store");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn keyed_lock_registry_reclaims_after_high_cardinality_churn() {
    const DISTINCT_KEYS: u32 = 10_000;
    const CHURN_BOUND: Duration = Duration::from_secs(60);

    // The registry the product actually uses: the one attached to this
    // store's SurrealDatabase wrapper (a clone shares the same map), so the
    // churn below and the real product writes afterwards hit one registry.
    let store = open_embedded_store()
        .await
        .expect("MT-142 requires the embedded SurrealDB test store");
    let registry = store.db.lock_registry().clone();
    assert_eq!(registry.mode(), LockMode::Keyed);

    // Sequential churn through 10_000 distinct record keys.
    for index in 0..DISTINCT_KEYS {
        let guard = registry
            .acquire(LockKey::record("knowledge_rich_documents", format!("KRD-{index}")))
            .await;
        assert!(!guard.is_noop(), "keyed mode hands out real guards");
        drop(guard);
        if index % 1_000 == 999 {
            assert_eq!(
                registry.idle_entry_count(),
                0,
                "no idle entry may linger mid-churn (after {} keys)",
                index + 1
            );
        }
    }
    assert_eq!(
        registry.entry_count(),
        0,
        "entry_count must return to the documented idle bound (0) after {DISTINCT_KEYS} distinct keys"
    );

    // Concurrent churn: 8 workers over natural keys and record keys with
    // overlapping key sets plus multi-key acquisitions.
    let mut workers = Vec::new();
    for worker in 0..8u32 {
        let registry = registry.clone();
        workers.push(tokio::spawn(async move {
            for index in 0..(DISTINCT_KEYS / 8) {
                let natural = LockKey::natural_key(
                    format!("ws-{}", index % 5),
                    "title",
                    format!("title-{}", (index + worker) % 700),
                );
                let record = LockKey::record("loom_blocks", format!("BLK-{}", (index * 7 + worker) % 900));
                if index % 3 == 0 {
                    let guards = registry.acquire_many(vec![record.clone(), natural.clone()]).await;
                    assert_eq!(guards.len(), 2);
                } else {
                    let guard = registry.acquire(natural).await;
                    if index % 97 == 0 {
                        tokio::task::yield_now().await;
                    }
                    drop(guard);
                }
            }
        }));
    }
    timeout(CHURN_BOUND, async {
        for worker in workers {
            worker.await.expect("churn worker joined");
        }
    })
    .await
    .expect("high-cardinality churn workers did not finish inside their bound");
    assert_eq!(registry.entry_count(), 0, "entry_count must be 0 after concurrent churn");
    assert_eq!(registry.idle_entry_count(), 0, "idle_entry_count must be 0 after concurrent churn");

    // Real product writes through the same registry: 32 concurrent saves on
    // 8 documents (4 writers per record key) leave it at the idle bound.
    let workspace_id = store.create_workspace().await;
    let documents = create_documents(&store.db, &workspace_id, "churn", 8).await;
    let barrier = Arc::new(Barrier::new(32));
    let mut writers = Vec::new();
    for writer in 0..32usize {
        let db = store.db.clone();
        let barrier = Arc::clone(&barrier);
        let document = documents[writer % documents.len()].clone();
        writers.push(tokio::spawn(async move {
            barrier.wait().await;
            timeout(
                PER_OPERATION_TIMEOUT,
                db.save_knowledge_rich_document_version(
                    &document.rich_document_id,
                    document.doc_version,
                    document_content(&format!("churn writer {writer}")),
                    None,
                    None,
                    None,
                ),
            )
            .await
            .unwrap_or_else(|_| panic!("churn writer {writer} exceeded its per-operation bound (hang)"))
        }));
    }
    let mut product_ok = 0usize;
    for task in writers {
        match task.await.expect("churn writer joined") {
            Ok(_) => product_ok += 1,
            Err(error) => assert!(
                is_typed_conflict(&error),
                "a churn writer must win or lose with the typed stale outcome, got: {error}"
            ),
        }
    }
    assert_eq!(product_ok, documents.len(), "exactly one writer per record commits");
    assert_eq!(
        store.db.lock_registry().entry_count(),
        0,
        "entry_count must return to 0 after real product writes through the registry"
    );
    assert_eq!(store.db.lock_registry().idle_entry_count(), 0);
    store.close_and_remove().await.expect("close store");

    // Disabled mode (the independent-client configuration) never tracks.
    let disabled = KeyedLockRegistry::disabled();
    for index in 0..1_000u32 {
        let guard = disabled
            .acquire(LockKey::record("knowledge_rich_documents", format!("KRD-{index}")))
            .await;
        assert!(guard.is_noop(), "disabled mode hands out no-op guards");
        assert_eq!(guard.lock_wait(), Duration::ZERO);
    }
    assert_eq!(disabled.entry_count(), 0, "disabled registry never holds entries");

    let mut summary = BTreeMap::new();
    summary.insert("distinct_keys", DISTINCT_KEYS);
    summary.insert("entry_count_after", registry.entry_count() as u32);
    summary.insert("idle_entry_count_after", registry.idle_entry_count() as u32);
    println!("SWARM_LOCK_REGISTRY {summary:?}");
}

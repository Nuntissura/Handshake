#![cfg(all(feature = "surreal-test-support", feature = "test-utils"))]
//! MT-142 adversarial review probes, LENS 1 (concurrency correctness and
//! atomicity), round 1. Written by the independent reviewer; retained as
//! regression coverage for the defects they demonstrated.
//!
//! Probe A - `probe_keyed_lock_reclaim_race_leaves_idle_entries`
//! Regression test for the confirmed `KeyedLockRegistry` reclamation race
//! (finding R1-1-1). The original `reclaim` took an `Arc::strong_count`
//! snapshot under the entries mutex but released its own strong reference
//! only after dropping that mutex, so two guards of one key could each see
//! the other's reference and neither would remove the entry. The map then
//! kept a dead `Weak` for a key nobody revisits - unbounded growth under
//! high-cardinality churn (AC-142-10 requires the documented idle bound).
//! The probe churns a fresh key window per round so a leaked entry can never
//! be papered over by a later acquire of the same key, and it uses many
//! releasers per key so the racing-drop interleaving is hit.
//!
//! Probe B - `probe_independent_clients_natural_key_upserts_stay_correct`
//! AC-142-8 attack on the natural-key upsert families that the lane's own
//! semantics target never races across independent clients: the entity
//! identity upsert (single IF/ELSE statement, implicit per-statement
//! transaction, `uq_knowledge_entities_identity`) and create-if-title-absent
//! (explicit transaction plus the MT-142 title write anchor). One client owns
//! a keyed registry, the other runs with `LockMode::Disabled`, so half the
//! racers hold no process-local lock at all and only the database transaction
//! and its guards can keep the pair correct. The probe also counts the retry
//! module's own structured events, so the run states whether a real embedded
//! RocksDB commit conflict was actually classified retryable and retried
//! end-to-end (the load profiles report `retry_count = 0`).

#[path = "knowledge_ingestion_support/mod.rs"]
mod knowledge_ingestion_support;

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use handshake_core::storage::knowledge::{
    KnowledgeEntityKind, KnowledgeStore, NewKnowledgeEntity, NewKnowledgeRichDocument,
};
use handshake_core::storage::surreal::keyed_lock::{KeyedLockRegistry, LockKey};
use handshake_core::storage::surreal::retry::{classify_storage_error, RetryClass};
use handshake_core::storage::surreal::SurrealDatabase;
use handshake_core::storage::StorageError;
use knowledge_ingestion_support::open_embedded_store;
use serde_json::json;
use tokio::sync::Barrier;
use tokio::time::timeout;
use tracing::field::{Field, Visit};
use tracing::subscriber::Interest;
use tracing::{Event, Metadata, Subscriber};
use tracing_subscriber::layer::{Context, Layer, SubscriberExt};
use tracing_subscriber::Registry;

const PER_OPERATION_TIMEOUT: Duration = Duration::from_secs(5);
const RACE_BOUND: Duration = Duration::from_secs(120);

// ---------------------------------------------------------------------------
// Retry-event counters over the product's own structured diagnostics
// (`handshake_core::storage::surreal::retry`: "surreal retry scheduled" at
// debug, "surreal retry exhausted" at warn).
// ---------------------------------------------------------------------------

const RETRY_TARGET: &str = "handshake_core::storage::surreal::retry";
static RETRY_SCHEDULED: AtomicU64 = AtomicU64::new(0);
static RETRY_EXHAUSTED: AtomicU64 = AtomicU64::new(0);
static PROBE_DIAGNOSTICS: OnceLock<bool> = OnceLock::new();

struct RetryProbeLayer;

#[derive(Default)]
struct MessageVisitor {
    message: String,
}

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{value:?}");
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_owned();
        }
    }
}

impl<S: Subscriber> Layer<S> for RetryProbeLayer {
    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        if metadata.target().starts_with(RETRY_TARGET) {
            Interest::always()
        } else {
            Interest::never()
        }
    }

    fn enabled(&self, metadata: &Metadata<'_>, _ctx: Context<'_, S>) -> bool {
        metadata.target().starts_with(RETRY_TARGET)
    }

    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);
        if visitor.message.contains("retry scheduled") {
            RETRY_SCHEDULED.fetch_add(1, Ordering::SeqCst);
        } else if visitor.message.contains("retry exhausted") {
            RETRY_EXHAUSTED.fetch_add(1, Ordering::SeqCst);
        }
    }
}

fn install_probe_diagnostics() -> bool {
    *PROBE_DIAGNOSTICS.get_or_init(|| {
        tracing::subscriber::set_global_default(Registry::default().with(RetryProbeLayer)).is_ok()
    })
}

fn retry_counts() -> (u64, u64) {
    (
        RETRY_SCHEDULED.load(Ordering::SeqCst),
        RETRY_EXHAUSTED.load(Ordering::SeqCst),
    )
}

// ---------------------------------------------------------------------------
// Probe A: keyed-lock reclamation.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn probe_keyed_lock_reclaim_race_leaves_idle_entries() {
    const TASKS: usize = 16;
    const ROUNDS: u32 = 1_500;
    const KEYS_PER_ROUND: u32 = 4;
    const CYCLES_PER_ROUND: u32 = 40;

    let registry = KeyedLockRegistry::keyed();
    let barrier = Arc::new(Barrier::new(TASKS));
    let mut tasks = Vec::with_capacity(TASKS);
    for task in 0..TASKS {
        let registry = registry.clone();
        let barrier = Arc::clone(&barrier);
        tasks.push(tokio::spawn(async move {
            let mut max_idle_seen = 0usize;
            for round in 0..ROUNDS {
                barrier.wait().await;
                let key = LockKey::record(
                    "knowledge_rich_documents",
                    format!(
                        "KRD-probe-{}",
                        round * KEYS_PER_ROUND + (task as u32 % KEYS_PER_ROUND)
                    ),
                );
                for _ in 0..CYCLES_PER_ROUND {
                    let guard = registry.acquire(key.clone()).await;
                    drop(guard);
                }
                if task == 0 && round % 100 == 99 {
                    max_idle_seen = max_idle_seen.max(registry.idle_entry_count());
                }
            }
            max_idle_seen
        }));
    }
    let mut max_idle_seen = 0usize;
    for task in tasks {
        max_idle_seen = max_idle_seen.max(task.await.expect("churn task joined"));
    }
    let entry_count = registry.entry_count();
    let idle_entry_count = registry.idle_entry_count();
    println!(
        "PROBE_LOCK_RECLAIM rounds={ROUNDS} keys={} cycles={} entry_count_after={entry_count} idle_entry_count_after={idle_entry_count} max_idle_seen_mid_run={max_idle_seen}",
        ROUNDS * KEYS_PER_ROUND,
        ROUNDS * KEYS_PER_ROUND * CYCLES_PER_ROUND
    );
    assert_eq!(
        idle_entry_count, 0,
        "keyed-lock reclamation leaked {idle_entry_count} dead Weak entries for keys no task will revisit (documented idle bound: 0; AC-142-10)"
    );
    assert_eq!(
        entry_count, 0,
        "entry_count must return to the documented idle bound after high-cardinality churn"
    );
}

// ---------------------------------------------------------------------------
// Probe B: independent clients over one engine, natural-key upserts.
// ---------------------------------------------------------------------------

fn document_content(text: &str) -> serde_json::Value {
    json!({
        "type": "doc",
        "content": [{
            "type": "paragraph",
            "content": [{ "type": "text", "text": format!("probe {text}") }]
        }]
    })
}

fn new_document(workspace_id: &str, title: &str, text: &str) -> NewKnowledgeRichDocument {
    NewKnowledgeRichDocument {
        workspace_id: workspace_id.to_owned(),
        document_id: None,
        title: title.to_owned(),
        schema_version: "hsk_richdoc_v1".to_owned(),
        content_json: document_content(text),
        crdt_document_id: None,
        crdt_snapshot_id: None,
        promotion_receipt_event_id: None,
        ..Default::default()
    }
}

fn describe(error: &StorageError) -> String {
    let class = classify_storage_error(error);
    format!("{error} [class={class:?}]")
}

async fn join_all<T: Send + 'static>(tasks: Vec<tokio::task::JoinHandle<T>>) -> Vec<T> {
    let mut out = Vec::with_capacity(tasks.len());
    for task in tasks {
        out.push(task.await.expect("racer task joined"));
    }
    out
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn probe_independent_clients_natural_key_upserts_stay_correct() {
    const RACERS: usize = 12;
    const ROUNDS: usize = 4;

    let diagnostics_owned = install_probe_diagnostics();
    let store = open_embedded_store()
        .await
        .expect("MT-142 requires the embedded SurrealDB test store");
    let workspace_id = store.create_workspace().await;
    // Two wrappers over ONE embedded engine with NO shared keyed-lock
    // registry: A keyed, B disabled (AC-142-8).
    let keyed = SurrealDatabase::new(store.storage.clone());
    let unlocked =
        SurrealDatabase::with_lock_registry(store.storage.clone(), KeyedLockRegistry::disabled());
    let clients = [keyed.clone(), unlocked.clone()];
    let before = retry_counts();

    // (1) Entity identity natural key: one statement, implicit transaction.
    let mut untyped = Vec::new();
    for round in 0..ROUNDS {
        let key = format!("probe-entity-{round}");
        let barrier = Arc::new(Barrier::new(RACERS));
        let mut tasks = Vec::with_capacity(RACERS);
        for racer in 0..RACERS {
            let db = clients[racer % clients.len()].clone();
            let barrier = Arc::clone(&barrier);
            let workspace_id = workspace_id.clone();
            let key = key.clone();
            tasks.push(tokio::spawn(async move {
                barrier.wait().await;
                timeout(
                    PER_OPERATION_TIMEOUT,
                    db.upsert_knowledge_entity(NewKnowledgeEntity {
                        workspace_id,
                        entity_kind: KnowledgeEntityKind::Concept,
                        entity_key: key.clone(),
                        display_name: format!("probe entity {key} racer {racer}"),
                        detection_provenance: json!({ "probe": "lens1", "racer": racer }),
                        primary_source_id: None,
                        detected_in_run: None,
                        evidence_span_ids: Vec::new(),
                    }),
                )
                .await
                .unwrap_or_else(|_| {
                    panic!("entity racer {racer} round {round} exceeded its per-operation bound")
                })
            }));
        }
        let mut ids = BTreeSet::new();
        let mut errors = Vec::new();
        for outcome in timeout(RACE_BOUND, join_all(tasks))
            .await
            .expect("entity race must finish inside its bound")
        {
            match outcome {
                Ok(entity) => {
                    ids.insert(entity.entity_id);
                }
                Err(error) => {
                    if classify_storage_error(&error) == RetryClass::RetryableTransient {
                        untyped.push(describe(&error));
                    }
                    errors.push(describe(&error));
                }
            }
        }
        let resolved = keyed
            .get_knowledge_entity_by_identity(&workspace_id, KnowledgeEntityKind::Concept, &key)
            .await
            .expect("resolve entity identity")
            .map(|entity| entity.entity_id);
        println!(
            "PROBE_ENTITY_RACE round={round} ok={} errors={} distinct_ids={} resolved={resolved:?} errors_detail={errors:?}",
            RACERS - errors.len(),
            errors.len(),
            ids.len()
        );
        assert!(
            errors.is_empty(),
            "independent-client entity upsert race round {round}: every racer must converge without a process-local lock (AC-142-8); errors: {errors:?}"
        );
        assert_eq!(
            ids.len(),
            1,
            "exactly one durable entity may exist per natural key, got {ids:?}"
        );
        assert_eq!(
            resolved.as_ref(),
            ids.iter().next(),
            "the resolved identity must be the converged id"
        );
    }
    assert!(
        untyped.is_empty(),
        "a raw engine conflict leaked to the caller instead of a typed outcome: {untyped:?}"
    );

    // Phase split so the title rounds below - whose `guarded_mutation` passes
    // `own_index: None`, making `RetryClass::RetryableSnapshotChange`
    // unreachable - attribute their retries to real engine commit conflicts
    // only.
    let after_entities = retry_counts();
    println!(
        "PROBE_RETRY_ENTITY scheduled={} exhausted={}",
        after_entities.0 - before.0,
        after_entities.1 - before.1
    );

    // (2) Title natural key: explicit transaction plus the MT-142 write anchor.
    for round in 0..ROUNDS {
        let title = format!("Probe Shared Title {round}");
        let barrier = Arc::new(Barrier::new(RACERS));
        let mut tasks = Vec::with_capacity(RACERS);
        for racer in 0..RACERS {
            let db = clients[racer % clients.len()].clone();
            let barrier = Arc::clone(&barrier);
            let document = new_document(&workspace_id, &title, &format!("racer {racer}"));
            tasks.push(tokio::spawn(async move {
                barrier.wait().await;
                timeout(
                    PER_OPERATION_TIMEOUT,
                    db.create_knowledge_rich_document_if_title_absent(document),
                )
                .await
                .unwrap_or_else(|_| {
                    panic!("title racer {racer} round {round} exceeded its per-operation bound")
                })
            }));
        }
        let mut ids = BTreeSet::new();
        let mut created = 0usize;
        let mut errors = Vec::new();
        for outcome in timeout(RACE_BOUND, join_all(tasks))
            .await
            .expect("title race must finish inside its bound")
        {
            match outcome {
                Ok((document, was_created)) => {
                    ids.insert(document.rich_document_id);
                    created += usize::from(was_created);
                }
                Err(error) => errors.push(describe(&error)),
            }
        }
        let live = keyed
            .list_knowledge_rich_documents(&workspace_id, None, None)
            .await
            .expect("list documents")
            .into_iter()
            .filter(|document| document.title == title)
            .count();
        println!(
            "PROBE_TITLE_RACE round={round} ok={} errors={} created={created} distinct_ids={} live_with_title={live} errors_detail={errors:?}",
            RACERS - errors.len(),
            errors.len(),
            ids.len()
        );
        assert!(
            errors.is_empty(),
            "independent-client title race round {round}: every racer must converge; errors: {errors:?}"
        );
        assert_eq!(created, 1, "exactly one racer may report creating the title");
        assert_eq!(
            ids.len(),
            1,
            "exactly one durable document may back the title, got {ids:?}"
        );
        assert_eq!(live, 1, "exactly one live document may carry the title");
    }

    let after = retry_counts();
    let title_scheduled = after.0 - after_entities.0;
    println!(
        "PROBE_RETRY diagnostics_owned={diagnostics_owned} scheduled_total={} scheduled_entity={} scheduled_title_engine_conflict_only={title_scheduled} exhausted={} (over {} entity + {} title racing calls)",
        after.0 - before.0,
        after_entities.0 - before.0,
        after.1 - before.1,
        ROUNDS * RACERS,
        ROUNDS * RACERS
    );
    assert_eq!(
        after.1 - before.1,
        0,
        "no natural-key race may exhaust the retry budget"
    );
    // The whole point of AC-142-5 end-to-end: a real embedded RocksDB commit
    // conflict must be classified retryable and replayed to success. The title
    // path cannot report a snapshot-change retry (own_index is None), so a
    // non-zero count here is proof the engine conflict path works, and a zero
    // count means this run never contended hard enough to prove it.
    assert!(
        title_scheduled > 0,
        "the title-anchor race must exercise the real engine-conflict retry path at least once (scheduled_title={title_scheduled})"
    );
    assert_eq!(keyed.lock_registry().entry_count(), 0);
    assert_eq!(unlocked.lock_registry().entry_count(), 0);
    store.close_and_remove().await.expect("close store");
}

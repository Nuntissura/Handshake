#![cfg(all(feature = "surreal-test-support", feature = "test-utils"))]
//! MT-142 "Harden SurrealDB swarm concurrency and load" - lifecycle stress:
//! graceful shutdown under load, typed closed outcomes afterwards, then a real
//! close-and-reopen durability proof against the same embedded RocksDB
//! `data_dir`.
//!
//! Proves AC-142-7 (PT-142-8):
//! * `shutdown_under_load_then_reopen_is_durable`
//!
//! Every worker and every operation is bounded; no sleep is used for
//! synchronisation (tokio `Notify` + `timeout` only); every task is joined.

#[path = "swarm_support/mod.rs"]
mod swarm_support;

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use handshake_core::storage::knowledge::{KnowledgeRichDocument, KnowledgeStore};
use handshake_core::storage::surreal::swarm_load_report::IntegrityVerdict;
use serde_json::json;
use swarm_support::*;
use tokio::sync::Notify;
use tokio::time::timeout;

const WORKERS: u32 = 16;
/// Acknowledged writes before shutdown is triggered mid-flight.
const ACKS_BEFORE_SHUTDOWN: u64 = 200;
/// Operations a worker may still complete successfully after shutdown was
/// requested before admission must have stopped (bounded drain).
const MAX_SUCCESSES_AFTER_SHUTDOWN_REQUEST: u64 = 64;
const LIFECYCLE_BOUND: Duration = Duration::from_millis(180_000);

#[derive(Debug, Default)]
struct WorkerSummary {
    worker: u32,
    acknowledged: u64,
    reads: u64,
    started_before_shutdown_finished_after: u64,
    drained_ok_in_flight: u64,
    rejected_in_flight: u64,
    successes_after_shutdown_request: u64,
    closed_error: Option<String>,
    untyped_post_shutdown_errors: Vec<String>,
    timed_out: Option<String>,
}

struct LifecycleShared {
    db: handshake_core::storage::surreal::SurrealDatabase,
    oracle: Mutex<Oracle>,
    acknowledged: AtomicU64,
    threshold_reached: Notify,
    shutdown_requested: AtomicBool,
    shutdown_started_at: Mutex<Option<Instant>>,
}

impl LifecycleShared {
    fn shutdown_requested(&self) -> bool {
        self.shutdown_requested.load(Ordering::SeqCst)
    }

    fn shutdown_started_at(&self) -> Option<Instant> {
        *self.shutdown_started_at.lock().expect("shutdown instant")
    }
}

async fn run_worker(
    shared: Arc<LifecycleShared>,
    worker: u32,
    documents: Vec<KnowledgeRichDocument>,
) -> WorkerSummary {
    let mut summary = WorkerSummary {
        worker,
        ..Default::default()
    };
    let mut heads: Vec<(String, i64)> = documents
        .iter()
        .map(|document| (document.rich_document_id.clone(), document.doc_version))
        .collect();
    let mut operation = 0u64;
    loop {
        let slot = (operation % heads.len() as u64) as usize;
        let (rich_document_id, expected_version) = heads[slot].clone();
        let requested_before = shared.shutdown_requested();
        let started = Instant::now();
        if operation % 3 == 2 {
            // Read: must either see a committed head or the typed closed error.
            let read = timeout(
                PER_OPERATION_TIMEOUT,
                shared.db.get_knowledge_rich_document(&rich_document_id),
            )
            .await;
            match read {
                Err(_) => {
                    summary.timed_out = Some(format!(
                        "worker {worker} read of {rich_document_id} exceeded the per-operation bound after shutdown_requested={requested_before}"
                    ));
                    break;
                }
                Ok(Ok(Some(document))) => {
                    summary.reads += 1;
                    let acknowledged_head = shared
                        .oracle
                        .lock()
                        .expect("oracle")
                        .docs
                        .get(&rich_document_id)
                        .and_then(|entry| entry.head_version().map(|(version, _)| version));
                    if let Some(head) = acknowledged_head {
                        assert!(
                            document.doc_version >= head,
                            "worker {worker} read {rich_document_id} at version {} below its acknowledged head {head} (lost write)",
                            document.doc_version
                        );
                    }
                    if requested_before {
                        summary.successes_after_shutdown_request += 1;
                    }
                }
                Ok(Ok(None)) => panic!("worker {worker}: document {rich_document_id} vanished"),
                Ok(Err(error)) => {
                    if is_closed_error(&error) {
                        summary.closed_error = Some(error.to_string());
                    } else {
                        summary
                            .untyped_post_shutdown_errors
                            .push(format!("read after shutdown_requested={requested_before}: {error}"));
                    }
                    break;
                }
            }
        } else {
            let saved = timeout(
                PER_OPERATION_TIMEOUT,
                shared.db.save_knowledge_rich_document_version(
                    &rich_document_id,
                    expected_version,
                    document_content(&format!("lifecycle w{worker} op{operation}")),
                    None,
                    None,
                    None,
                ),
            )
            .await;
            let finished_after_shutdown_started = shared
                .shutdown_started_at()
                .is_some_and(|at| started < at && Instant::now() > at);
            match saved {
                Err(_) => {
                    summary.timed_out = Some(format!(
                        "worker {worker} save of {rich_document_id} exceeded the per-operation bound after shutdown_requested={requested_before}"
                    ));
                    break;
                }
                Ok(Ok(document)) => {
                    heads[slot].1 = document.doc_version;
                    shared.oracle.lock().expect("oracle").ack_save(&document);
                    summary.acknowledged += 1;
                    let total = shared.acknowledged.fetch_add(1, Ordering::SeqCst) + 1;
                    if total == ACKS_BEFORE_SHUTDOWN {
                        shared.threshold_reached.notify_one();
                    }
                    if finished_after_shutdown_started {
                        summary.started_before_shutdown_finished_after += 1;
                        summary.drained_ok_in_flight += 1;
                    }
                    if requested_before {
                        summary.successes_after_shutdown_request += 1;
                    }
                }
                Ok(Err(error)) => {
                    if is_closed_error(&error) {
                        summary.closed_error = Some(error.to_string());
                        if finished_after_shutdown_started {
                            summary.started_before_shutdown_finished_after += 1;
                            summary.rejected_in_flight += 1;
                        }
                    } else {
                        // Before shutdown every save on a worker-private
                        // document must succeed; afterwards only the typed
                        // closed/cancelled error is acceptable.
                        summary.untyped_post_shutdown_errors.push(format!(
                            "save after shutdown_requested={requested_before}: {error}"
                        ));
                    }
                    break;
                }
            }
        }
        operation += 1;
        if summary.successes_after_shutdown_request > MAX_SUCCESSES_AFTER_SHUTDOWN_REQUEST {
            summary.untyped_post_shutdown_errors.push(format!(
                "worker {worker} completed {} operations after shutdown was requested; admission never stopped",
                summary.successes_after_shutdown_request
            ));
            break;
        }
    }
    summary
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn shutdown_under_load_then_reopen_is_durable() {
    let seed = workload_seed();
    println!("SWARM_SEED={seed}");
    timeout(LIFECYCLE_BOUND, lifecycle_proof(seed))
        .await
        .unwrap_or_else(|_| {
            panic!(
                "lifecycle proof exceeded its whole-test bound of {} ms",
                LIFECYCLE_BOUND.as_millis()
            )
        });
}

async fn lifecycle_proof(seed: u64) {
    let run_id = new_run_id("mt142-lifecycle");
    let store = open_embedded_store()
        .await
        .expect("MT-142 requires the embedded SurrealDB test store");
    let workspace_id = store.create_workspace().await;
    let inspector = store.storage.test_inspector();
    let baseline = table_counts(&inspector).await;

    // Two private documents per worker: writes never conflict, so every
    // pre-shutdown save must be acknowledged.
    let mut oracle = Oracle::default();
    let mut per_worker: Vec<Vec<KnowledgeRichDocument>> = Vec::with_capacity(WORKERS as usize);
    for worker in 0..WORKERS {
        let mut documents = Vec::with_capacity(2);
        for slot in 0..2 {
            let document = store
                .db
                .create_knowledge_rich_document(new_document(
                    &workspace_id,
                    &format!("lifecycle w{worker} d{slot}"),
                    &format!("lifecycle base w{worker} d{slot}"),
                ))
                .await
                .expect("create lifecycle document");
            oracle.ack_create(&document);
            documents.push(document);
        }
        per_worker.push(documents);
    }

    let shared = Arc::new(LifecycleShared {
        db: store.db.clone(),
        oracle: Mutex::new(oracle),
        acknowledged: AtomicU64::new(0),
        threshold_reached: Notify::new(),
        shutdown_requested: AtomicBool::new(false),
        shutdown_started_at: Mutex::new(None),
    });

    let mut tasks = Vec::with_capacity(WORKERS as usize);
    for (worker, documents) in per_worker.into_iter().enumerate() {
        let shared = Arc::clone(&shared);
        let worker = worker as u32;
        tasks.push(tokio::spawn(async move {
            match timeout(PER_WORKER_TIMEOUT, run_worker(shared, worker, documents)).await {
                Ok(summary) => summary,
                Err(_) => WorkerSummary {
                    worker,
                    timed_out: Some(format!(
                        "worker {worker} exceeded its per-worker bound of {} ms",
                        PER_WORKER_TIMEOUT.as_millis()
                    )),
                    ..Default::default()
                },
            }
        }));
    }

    // Wait (bounded, no sleep) until enough writes were acknowledged, then
    // shut down while workers are mid-flight.
    timeout(PER_WORKER_TIMEOUT, shared.threshold_reached.notified())
        .await
        .expect("workers must acknowledge the pre-shutdown write threshold inside the bound");
    let acknowledged_before = shared.acknowledged.load(Ordering::SeqCst);
    assert!(
        acknowledged_before >= ACKS_BEFORE_SHUTDOWN,
        "threshold notified early: {acknowledged_before} < {ACKS_BEFORE_SHUTDOWN}"
    );
    let shutdown_started = Instant::now();
    *shared.shutdown_started_at.lock().expect("shutdown instant") = Some(shutdown_started);
    shared.shutdown_requested.store(true, Ordering::SeqCst);
    let in_flight_hint = shared.acknowledged.load(Ordering::SeqCst);
    let shutdown_wait = store.storage.config().shutdown_wait();
    let drain_grace = store.storage.config().drain_grace();
    let shutdown = timeout(
        shutdown_wait + Duration::from_secs(5),
        store.storage.shutdown_with_report(),
    )
    .await;
    let shutdown_elapsed = shutdown_started.elapsed();
    let shutdown_result = match shutdown {
        Ok(Ok(report)) => Ok(report),
        Ok(Err(error)) => Err(error.to_string()),
        Err(_) => Err(format!(
            "shutdown did not return inside its explicit bound of {} ms",
            shutdown_wait.as_millis()
        )),
    };
    println!(
        "SWARM_SHUTDOWN elapsed_ms={} report={shutdown_result:?} acknowledged_at_request={in_flight_hint} drain_grace_ms={} shutdown_wait_ms={}",
        shutdown_elapsed.as_millis(),
        drain_grace.as_millis(),
        shutdown_wait.as_millis()
    );
    let shutdown_report = shutdown_result
        .expect("graceful shutdown under load must succeed inside its explicit bound");
    assert!(
        shutdown_elapsed <= shutdown_wait,
        "shutdown took {} ms, over its explicit bound of {} ms",
        shutdown_elapsed.as_millis(),
        shutdown_wait.as_millis()
    );
    assert!(
        shutdown_report.elapsed <= shutdown_wait,
        "engine-reported close {} ms exceeds the shutdown bound",
        shutdown_report.elapsed.as_millis()
    );
    assert!(
        shutdown_report.drained || shutdown_report.cancelled,
        "the report must say whether in-flight work drained or was cancelled: {shutdown_report:?}"
    );
    if shutdown_report.cancelled {
        assert!(
            shutdown_elapsed >= drain_grace,
            "cancellation may only fire after the drain grace ({} ms) expired",
            drain_grace.as_millis()
        );
    }
    assert!(
        !store.storage.is_accepting_operations(),
        "shutdown must stop admission"
    );
    assert!(store.storage.is_closed().await, "the engine must be closed after shutdown");

    let mut summaries = Vec::with_capacity(WORKERS as usize);
    for task in tasks {
        summaries.push(task.await.expect("worker task joined"));
    }
    let acknowledged_total = shared.acknowledged.load(Ordering::SeqCst);
    let drained: u64 = summaries.iter().map(|s| s.drained_ok_in_flight).sum();
    let rejected_in_flight: u64 = summaries.iter().map(|s| s.rejected_in_flight).sum();
    let in_flight_at_shutdown: u64 = summaries
        .iter()
        .map(|s| s.started_before_shutdown_finished_after)
        .sum();
    let successes_after_request: u64 = summaries
        .iter()
        .map(|s| s.successes_after_shutdown_request)
        .sum();
    let reads_total: u64 = summaries.iter().map(|s| s.reads).sum();
    println!(
        "SWARM_LIFECYCLE acknowledged_total={acknowledged_total} reads={reads_total} in_flight_at_shutdown={in_flight_at_shutdown} drained_ok={drained} rejected_in_flight={rejected_in_flight} successes_after_request={successes_after_request}"
    );
    for summary in &summaries {
        assert!(
            summary.timed_out.is_none(),
            "worker {} hung: {:?}",
            summary.worker,
            summary.timed_out
        );
        assert!(
            summary.untyped_post_shutdown_errors.is_empty(),
            "worker {} saw errors that are neither acknowledged writes nor the typed closed outcome: {:?}",
            summary.worker,
            summary.untyped_post_shutdown_errors
        );
        assert!(
            summary.closed_error.is_some(),
            "worker {} never observed the typed closed/cancelled error after shutdown (admission did not stop or the worker exited early)",
            summary.worker
        );
    }
    assert!(
        in_flight_at_shutdown <= WORKERS as u64,
        "at most one in-flight operation per worker can straddle the shutdown instant"
    );

    // Real reopen of the same data_dir.
    let reopen_started = Instant::now();
    let reopened = store
        .reopen_database()
        .await
        .expect("reopen the same data_dir after shutdown");
    let reopen_elapsed = reopen_started.elapsed();
    let reopened_inspector = reopened.storage().test_inspector();
    let oracle = Arc::try_unwrap(shared)
        .unwrap_or_else(|_| panic!("every worker released the shared state"))
        .oracle
        .into_inner()
        .expect("oracle");
    assert_eq!(
        oracle.acknowledged_writes,
        acknowledged_total + 2 * u64::from(WORKERS),
        "oracle must hold every acknowledged create and save"
    );
    let integrity = reconcile(&reopened, &reopened_inspector, &oracle, &baseline).await;
    println!(
        "SWARM_REOPEN elapsed_ms={} verdict={:?} violations={} documents_checked={} versions_checked={}",
        reopen_elapsed.as_millis(),
        integrity.verdict,
        integrity.violations.len(),
        integrity.documents_checked,
        integrity.versions_checked
    );

    // Every acknowledged write is present with its version row and no
    // unacknowledged partial transaction exists (doc_version == highest
    // version row, no orphan version/block/index rows).
    let mut acknowledged_versions = 0u64;
    for (rich_document_id, entry) in &oracle.docs {
        let live = reopened
            .get_knowledge_rich_document(rich_document_id)
            .await
            .expect("read after reopen")
            .unwrap_or_else(|| panic!("document {rich_document_id} lost across reopen"));
        let (head, sha) = entry.head_version().expect("acknowledged head");
        assert_eq!(
            live.doc_version, head,
            "document {rich_document_id} head must equal its highest acknowledged version after reopen"
        );
        assert_eq!(
            live.content_sha256, sha,
            "document {rich_document_id} must carry the acknowledged head content after reopen"
        );
        let versions = reopened
            .list_knowledge_rich_document_versions(rich_document_id)
            .await
            .expect("versions after reopen");
        let stored: BTreeMap<i64, String> = versions
            .iter()
            .map(|row| (row.doc_version, row.content_sha256.clone()))
            .collect();
        assert_eq!(
            stored, entry.versions,
            "document {rich_document_id} version chain must equal exactly the acknowledged writes (no lost, no unacknowledged partial rows)"
        );
        acknowledged_versions += stored.len() as u64;
    }
    assert_eq!(
        integrity.verdict,
        IntegrityVerdict::Pass,
        "reopen reconciliation must PASS; first violations:\n{}",
        integrity.rendered_violations(25)
    );

    let store_path_drive = machine_context(reopened.storage().config().path()).store_drive_kind;
    reopened
        .storage()
        .shutdown()
        .await
        .expect("shutdown the reopened engine");
    drop(reopened_inspector);
    drop(reopened);

    let fragment = json!({
        "schema_id": REPORT_FRAGMENT_SCHEMA_ID,
        "fragment": "lifecycle_shutdown_reopen",
        "run_id": run_id,
        "source_commit": source_commit(),
        "surrealdb_version": SURREALDB_VERSION,
        "engine_mode": "embedded_rocks_db",
        "workload_seed": seed,
        "worker_count": WORKERS,
        "acknowledged_writes_before_shutdown_request": acknowledged_before,
        "acknowledged_writes_total": acknowledged_total,
        "in_flight_at_shutdown": in_flight_at_shutdown,
        "drained_ok": drained,
        "rejected_in_flight": rejected_in_flight,
        "successes_after_shutdown_request": successes_after_request,
        "shutdown_elapsed_ms": shutdown_elapsed.as_millis() as u64,
        "shutdown_bound_ms": shutdown_wait.as_millis() as u64,
        "drain_grace_ms": drain_grace.as_millis() as u64,
        "shutdown_report": {
            "drained": shutdown_report.drained,
            "cancelled": shutdown_report.cancelled,
            "engine_elapsed_ms": shutdown_report.elapsed.as_millis() as u64,
        },
        "drained_vs_rejected_note": "drained_ok / rejected_in_flight count worker operations that straddled the shutdown instant (acknowledged vs typed closed); the ShutdownReport says whether the lease drain finished inside drain_grace or the cooperative cancellation fired",
        "reopen_elapsed_ms": reopen_elapsed.as_millis() as u64,
        "reopen_integrity_counts_and_hashes": integrity.counts_and_hashes,
        "acknowledged_version_rows_verified": acknowledged_versions,
        "integrity_verdict": format!("{:?}", integrity.verdict),
        "store_drive_kind": format!("{store_path_drive:?}"),
        "remote_proof_status": "not_run_unconfigured",
    });
    let path = write_report_json(&format!("swarm-lifecycle-{run_id}.json"), &fragment);
    println!("SWARM_LIFECYCLE_REPORT={}", path.display());

    store.close_and_remove().await.expect("close and remove the store");
}

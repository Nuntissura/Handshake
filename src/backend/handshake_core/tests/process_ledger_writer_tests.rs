use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use bytes::Bytes;
use serde_json::json;

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

use handshake_core::{
    process_ledger::{
        cap_metadata_jsonb, flush_failed_row_count, LedgerBatcher, LedgerBatcherConfig,
        LedgerDrainJoinOutcome, LedgerEvent, LedgerEventKind, LedgerOverflowEvent,
        ProcessEngineKind, ProcessLedgerError, ProcessLedgerOverflowSink, ProcessLedgerStore,
        ProcessStart, ProcessStop, RetainedLedgerBatcher, StopRecordOutcome,
        PROCESS_LEDGER_BATCH_SIZE, PROCESS_LEDGER_FLUSH_INTERVAL_MS,
        PROCESS_LEDGER_METADATA_CAP_BYTES, PROCESS_LEDGER_RING_CAPACITY,
    },
    sandbox::{
        build_registry_from_adapters_with_ledger, default_no_op_capabilities, AdapterCapabilities,
        AdapterId, BindMode, Command, ExecResult, ImageRef, LedgerDecorator, NetPolicy,
        ProcessHandle, ProcessSpec, ProcessStatus, ResourceLimits, SandboxAdapter,
        SandboxAdapterError, Signal, TrustClass,
    },
};

/// `flush_failed_row_count()` is a PROCESS-WIDE monotonic counter, so any two
/// tests that assert on its delta would race when the binary runs its tests
/// concurrently (the default). Tests that read the global counter take this
/// guard for the full duration of their measurement window so their deltas stay
/// deterministic. Poison is recovered (a panicking sibling test must not wedge
/// the rest of the suite on this lock).
static FLUSH_COUNTER_GUARD: Mutex<()> = Mutex::new(());

fn lock_flush_counter() -> std::sync::MutexGuard<'static, ()> {
    FLUSH_COUNTER_GUARD
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[test]
fn ledger_batcher_uses_mt053_batch_flush_and_ring_defaults() {
    assert_eq!(PROCESS_LEDGER_RING_CAPACITY, 10_000);
    assert_eq!(PROCESS_LEDGER_BATCH_SIZE, 100);
    assert_eq!(PROCESS_LEDGER_FLUSH_INTERVAL_MS, 250);
    assert_eq!(PROCESS_LEDGER_METADATA_CAP_BYTES, 16 * 1024);

    let config = LedgerBatcherConfig::default();
    assert_eq!(config.capacity, PROCESS_LEDGER_RING_CAPACITY);
    assert_eq!(config.batch_size, PROCESS_LEDGER_BATCH_SIZE);
    assert_eq!(config.flush_interval.as_millis(), 250);
}

#[test]
fn ledger_batcher_rejects_batch_size_larger_than_capacity() {
    let result = LedgerBatcher::manual_for_tests(
        LedgerBatcherConfig {
            capacity: 2,
            batch_size: 3,
            flush_interval: std::time::Duration::from_millis(250),
        },
        Arc::new(InMemoryOverflowSink::default()),
    );

    let error = match result {
        Ok(_) => panic!("batch_size larger than capacity must be rejected"),
        Err(error) => error,
    };
    assert!(
        matches!(
            error,
            ProcessLedgerError::InvalidConfig(ref message)
                if message == "batch_size 3 must not exceed capacity 2"
        ),
        "unexpected invalid-config error: {error}"
    );
}

#[test]
fn metadata_jsonb_over_16kb_is_capped_with_original_size_marker() {
    let mut metadata = BTreeMap::new();
    metadata.insert("oversized".to_string(), "x".repeat(20 * 1024));

    let capped = cap_metadata_jsonb(&metadata);
    assert!(capped.was_capped);
    assert_eq!(capped.original_bytes, Some(20 * 1024 + 16));
    assert_eq!(capped.value["capped"], true);
    assert_eq!(capped.value["original_bytes"], 20 * 1024 + 16);
    assert!(serde_json::to_vec(&capped.value).unwrap().len() <= PROCESS_LEDGER_METADATA_CAP_BYTES);
}

#[tokio::test]
async fn overflow_10001st_event_emits_fr_evt_ledger_overflow_without_blocking_spawn_path() {
    let overflow = InMemoryOverflowSink::default();
    let (batcher, drain) = LedgerBatcher::manual_for_tests(
        LedgerBatcherConfig {
            capacity: PROCESS_LEDGER_RING_CAPACITY,
            batch_size: PROCESS_LEDGER_BATCH_SIZE,
            flush_interval: std::time::Duration::from_millis(PROCESS_LEDGER_FLUSH_INTERVAL_MS),
        },
        Arc::new(overflow.clone()),
    )
    .expect("manual batcher");

    for index in 0..=PROCESS_LEDGER_RING_CAPACITY {
        batcher
            .record_start(ProcessStart::new(
                ProcessEngineKind::SandboxContainer,
                format!("role-{index}"),
                Some("WP-KERNEL-004".to_string()),
            ))
            .expect("nonblocking enqueue");
    }

    let overflow_events = overflow.events();
    assert_eq!(overflow_events.len(), 1);
    assert_eq!(overflow_events[0].event_type, "FR_EVT_LEDGER_OVERFLOW");
    assert_eq!(overflow_events[0].overflow_count, 1);
    assert_eq!(overflow_events[0].capacity, PROCESS_LEDGER_RING_CAPACITY);
    assert_eq!(
        overflow_events[0].dropped_event_kind,
        LedgerEventKind::Start
    );

    let store = InMemoryProcessLedgerStore::default();
    drain
        .drain_available_to(Arc::new(store.clone()))
        .await
        .expect("drain retained events");
    assert_eq!(store.events().len(), PROCESS_LEDGER_RING_CAPACITY);
}

#[tokio::test]
async fn dual_lifecycle_reservation_is_all_or_none_when_capacity_is_insufficient() {
    let (batcher, drain) = LedgerBatcher::manual_for_tests(
        LedgerBatcherConfig {
            capacity: 2,
            batch_size: 2,
            flush_interval: std::time::Duration::from_millis(250),
        },
        Arc::new(InMemoryOverflowSink::default()),
    )
    .expect("manual lifecycle reservation batcher");

    batcher
        .record_start(ProcessStart::new(
            ProcessEngineKind::SandboxContainer,
            "unrelated-prefill",
            Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".to_string()),
        ))
        .expect("prefill one of two writer slots");

    let error = match batcher.try_reserve_lifecycles(1) {
        Ok(_) => panic!("one remaining slot cannot reserve a complete two-row lifecycle"),
        Err(error) => error,
    };
    assert!(matches!(
        error,
        ProcessLedgerError::EnqueueDropped(ref message)
            if message.contains("writer is full or undersized")
    ));

    // The dynamic reservation acquired one permit before discovering the
    // second slot was unavailable. Failure must emit no partial lifecycle and
    // must release that temporary permit.
    let store = Arc::new(InMemoryProcessLedgerStore::default());
    drain
        .drain_available_to(store.clone())
        .await
        .expect("drain after failed reservation");
    assert_eq!(
        store.events().len(),
        1,
        "only the unrelated prefill row may survive the failed reservation"
    );

    let one = batcher
        .try_reserve_lifecycles(1)
        .expect("the partially acquired permit was returned after failure");
    assert_eq!(one.len(), 1);
}

#[test]
fn lifecycle_reservation_rejects_huge_counts_before_allocation() {
    let (batcher, _drain) = LedgerBatcher::manual_for_tests(
        LedgerBatcherConfig {
            capacity: 2,
            batch_size: 1,
            flush_interval: std::time::Duration::from_millis(250),
        },
        Arc::new(InMemoryOverflowSink::default()),
    )
    .expect("small manual ledger");

    let undersized = match batcher.try_reserve_lifecycles(usize::MAX / 2) {
        Ok(_) => panic!("huge non-overflowing reservation must fail before allocation"),
        Err(error) => error,
    };
    assert!(matches!(
        undersized,
        ProcessLedgerError::EnqueueDropped(ref message)
            if message.contains("writer is undersized")
    ));

    let overflow = match batcher.try_reserve_lifecycles(usize::MAX) {
        Ok(_) => panic!("overflowing reservation count must fail before allocation"),
        Err(error) => error,
    };
    assert!(matches!(
        overflow,
        ProcessLedgerError::InvalidConfig(ref message)
            if message.contains("overflowed usize")
    ));
}

#[tokio::test]
async fn reserved_stop_is_accepted_after_writer_close_begins() {
    let store = Arc::new(InMemoryProcessLedgerStore::default());
    let (batcher, join) = LedgerBatcher::spawn(
        store.clone(),
        Arc::new(InMemoryOverflowSink::default()),
        LedgerBatcherConfig {
            capacity: 2,
            batch_size: 2,
            flush_interval: std::time::Duration::from_millis(10),
        },
    );
    let reservation = batcher
        .try_reserve_lifecycles(1)
        .expect("reserve START and future STOP")
        .pop()
        .expect("one reservation");
    let active = reservation
        .begin(ProcessStart::new(
            ProcessEngineKind::Candle,
            "reserved-close-proof",
            None,
        ))
        .expect("begin reserved lifecycle");

    batcher.begin_close();
    tokio::task::yield_now().await;
    assert_eq!(
        active
            .stop(Some(0), "reserved-stop-after-close")
            .expect("owned STOP permit remains valid"),
        StopRecordOutcome::Recorded
    );
    drop(active);
    tokio::time::timeout(std::time::Duration::from_secs(2), join)
        .await
        .expect("writer close waits for reserved STOP")
        .expect("writer task joins")
        .expect("writer drains successfully");

    let events = store.events();
    assert_eq!(events.len(), 2);
    assert!(matches!(events[0], LedgerEvent::Start(_)));
    assert!(matches!(events[1], LedgerEvent::Stop(_)));
}

#[tokio::test]
async fn durable_start_ack_resolves_only_after_store_recovery_and_commit() {
    let _counter_guard = lock_flush_counter();
    let store = Arc::new(RecoverableProcessLedgerStore::default());
    let (batcher, join) = LedgerBatcher::spawn(
        store.clone(),
        Arc::new(InMemoryOverflowSink::default()),
        LedgerBatcherConfig {
            capacity: 2,
            batch_size: 1,
            flush_interval: std::time::Duration::from_millis(10),
        },
    );
    let reservation = batcher
        .try_reserve_lifecycles(1)
        .expect("reserve acknowledged START and STOP")
        .pop()
        .expect("one lifecycle reservation");
    let process_uuid = uuid::Uuid::now_v7();
    let (active, start_ack) = reservation
        .begin_with_durable_ack_for_test(
            ProcessStart::new(ProcessEngineKind::Candle, "durable-start-proof", None)
                .with_process_uuid(process_uuid),
        )
        .expect("begin acknowledged lifecycle");
    let ack_wait =
        tokio::spawn(async move { start_ack.wait(std::time::Duration::from_secs(2)).await });

    wait_for_store_attempts(&store, 1).await;
    assert!(
        !ack_wait.is_finished(),
        "queue acceptance or a failed store call must not acknowledge durability"
    );
    assert!(store.events().is_empty());

    store.recover();
    ack_wait
        .await
        .expect("ack waiter joins")
        .expect("START is acknowledged after the retained batch commits");
    let events = store.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind(), LedgerEventKind::Start);
    assert_eq!(events[0].process_uuid(), process_uuid);

    assert_eq!(
        active
            .stop(Some(0), "durable-start-proof-complete")
            .expect("reserved STOP after acknowledged START"),
        StopRecordOutcome::Recorded
    );
    drop(active);
    batcher.begin_close();
    tokio::time::timeout(std::time::Duration::from_secs(2), join)
        .await
        .expect("writer closes")
        .expect("writer joins")
        .expect("writer flushes STOP");
    let events = store.events();
    assert_eq!(events.len(), 2);
    assert_eq!(events[1].kind(), LedgerEventKind::Stop);
    assert_eq!(events[1].process_uuid(), process_uuid);
}

#[tokio::test]
async fn acknowledged_start_forces_flush_without_waiting_for_batch_or_long_tick() {
    let store = Arc::new(InMemoryProcessLedgerStore::default());
    let (batcher, join) = LedgerBatcher::spawn(
        store.clone(),
        Arc::new(InMemoryOverflowSink::default()),
        LedgerBatcherConfig {
            capacity: 4,
            batch_size: 4,
            flush_interval: std::time::Duration::from_secs(60),
        },
    );
    let reservation = batcher
        .try_reserve_lifecycles(1)
        .expect("reserve acknowledged lifecycle")
        .pop()
        .expect("one lifecycle reservation");
    let process_uuid = uuid::Uuid::now_v7();
    let (active, ack) = reservation
        .begin_with_durable_ack_for_test(
            ProcessStart::new(ProcessEngineKind::Candle, "forced-ack-flush", None)
                .with_process_uuid(process_uuid),
        )
        .expect("begin acknowledged lifecycle");

    ack.wait(std::time::Duration::from_millis(250))
        .await
        .expect("acknowledged row bypasses the 60-second tick and partial batch");
    let events = store.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].process_uuid(), process_uuid);

    active
        .stop(Some(0), "forced-ack-flush-complete")
        .expect("reserved STOP");
    drop(active);
    batcher.begin_close();
    tokio::time::timeout(std::time::Duration::from_secs(2), join)
        .await
        .expect("writer closes")
        .expect("writer joins")
        .expect("writer flushes STOP");
}

#[tokio::test]
async fn durable_start_ack_timeout_is_typed_and_suppresses_unproven_stop() {
    let _counter_guard = lock_flush_counter();
    let store = Arc::new(RecoverableProcessLedgerStore::default());
    let (batcher, join) = LedgerBatcher::spawn(
        store.clone(),
        Arc::new(InMemoryOverflowSink::default()),
        LedgerBatcherConfig {
            capacity: 2,
            batch_size: 1,
            flush_interval: std::time::Duration::from_secs(60),
        },
    );
    let reservation = batcher
        .try_reserve_lifecycles(1)
        .expect("reserve acknowledged lifecycle")
        .pop()
        .expect("one lifecycle reservation");
    let process_uuid = uuid::Uuid::now_v7();
    let (active, start_ack) = reservation
        .begin_with_durable_ack_for_test(
            ProcessStart::new(ProcessEngineKind::Candle, "durable-timeout-proof", None)
                .with_process_uuid(process_uuid),
        )
        .expect("begin acknowledged lifecycle");
    let error = start_ack
        .wait(std::time::Duration::from_millis(20))
        .await
        .expect_err("permanently failing store must not false-acknowledge START");
    assert!(
        matches!(error, ProcessLedgerError::DurabilityAckTimeout { .. }),
        "{error}"
    );
    assert!(store.events().is_empty());

    // A timed-out acknowledgement cannot distinguish a late commit from a
    // rejected/missing START. The reserved STOP is consumed without emission;
    // eventual START authority stays open for liveness reconciliation.
    assert_eq!(
        active
            .stop(Some(-1), "durable-start-timeout-rollback")
            .expect("unproven STOP is suppressed"),
        StopRecordOutcome::LeftOpenForReconciliation
    );
    drop(active);
    store.recover();
    batcher.begin_close();
    tokio::time::timeout(std::time::Duration::from_secs(2), join)
        .await
        .expect("writer closes after recovery")
        .expect("writer joins")
        .expect("writer commits retained START");
    let events = store.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind(), LedgerEventKind::Start);
    assert!(events
        .iter()
        .all(|event| event.process_uuid() == process_uuid));
}

#[tokio::test]
async fn durable_stop_ack_timeout_is_unconfirmed_and_late_flush_is_not_called_open() {
    let _counter_guard = lock_flush_counter();
    let store = Arc::new(RecoverableProcessLedgerStore::default());
    store.recover();
    let (batcher, join) = LedgerBatcher::spawn(
        store.clone(),
        Arc::new(InMemoryOverflowSink::default()),
        LedgerBatcherConfig {
            capacity: 2,
            batch_size: 1,
            flush_interval: std::time::Duration::from_secs(60),
        },
    );
    let reservation = batcher
        .try_reserve_lifecycles(1)
        .expect("reserve lifecycle")
        .pop()
        .expect("one lifecycle reservation");
    let process_uuid = uuid::Uuid::now_v7();
    let (active, start_ack) = reservation
        .begin_with_durable_ack_for_test(
            ProcessStart::new(ProcessEngineKind::Candle, "late-stop-proof", None)
                .with_process_uuid(process_uuid),
        )
        .expect("begin lifecycle");
    start_ack
        .wait(std::time::Duration::from_millis(250))
        .await
        .expect("START is durable before resource shutdown");

    store.fail();
    let error = active
        .stop_with_durable_ack(
            Some(0),
            "late-stop-after-timeout",
            std::time::Duration::from_millis(20),
        )
        .await
        .expect_err("failed store must not false-acknowledge STOP");
    assert!(matches!(
        error,
        ProcessLedgerError::DurabilityAckTimeout { .. }
    ));
    assert_eq!(
        active
            .stop_with_durable_ack(
                Some(0),
                "second-observer",
                std::time::Duration::from_millis(20),
            )
            .await
            .expect("second observer gets typed lifecycle outcome"),
        StopRecordOutcome::DurabilityUnconfirmed,
        "a queued STOP with unknown durability must not be mislabeled as an open START"
    );

    store.recover();
    batcher.begin_close();
    tokio::time::timeout(std::time::Duration::from_secs(2), join)
        .await
        .expect("writer closes after recovery")
        .expect("writer joins")
        .expect("retained STOP commits after recovery");
    let events = store.events();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].kind(), LedgerEventKind::Start);
    assert_eq!(events[1].kind(), LedgerEventKind::Stop);
    assert!(events
        .iter()
        .all(|event| event.process_uuid() == process_uuid));
}

#[tokio::test]
async fn dropping_pending_durable_start_handle_never_fabricates_stop() {
    let _counter_guard = lock_flush_counter();
    let store = Arc::new(RecoverableProcessLedgerStore::default());
    let (batcher, join) = LedgerBatcher::spawn(
        store.clone(),
        Arc::new(InMemoryOverflowSink::default()),
        LedgerBatcherConfig {
            capacity: 2,
            batch_size: 1,
            flush_interval: std::time::Duration::from_secs(60),
        },
    );
    let reservation = batcher
        .try_reserve_lifecycles(1)
        .expect("reserve cancellable acknowledged lifecycle")
        .pop()
        .expect("one lifecycle reservation");
    let process_uuid = uuid::Uuid::now_v7();
    let (active, ack) = reservation
        .begin_with_durable_ack_for_test(
            ProcessStart::new(ProcessEngineKind::Candle, "cancelled-ack-proof", None)
                .with_process_uuid(process_uuid),
        )
        .expect("begin cancellable acknowledged lifecycle");
    wait_for_store_attempts(&store, 1).await;
    drop(ack);
    drop(active);

    store.recover();
    batcher.begin_close();
    tokio::time::timeout(std::time::Duration::from_secs(2), join)
        .await
        .expect("writer closes after cancelled acknowledged handle")
        .expect("writer task joins")
        .expect("writer commits retained START without fabricated STOP");
    let events = store.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind(), LedgerEventKind::Start);
    assert_eq!(events[0].process_uuid(), process_uuid);
}

#[tokio::test]
async fn explicitly_unproven_lifecycle_leaves_start_open_for_reconciliation() {
    let (batcher, drain) = LedgerBatcher::manual_for_tests(
        LedgerBatcherConfig {
            capacity: 2,
            batch_size: 2,
            flush_interval: std::time::Duration::from_millis(250),
        },
        Arc::new(InMemoryOverflowSink::default()),
    )
    .expect("manual reconciliation lifecycle batcher");
    let active = batcher
        .try_reserve_lifecycles(1)
        .expect("reserve lifecycle")
        .pop()
        .expect("one reservation")
        .begin(ProcessStart::new(
            ProcessEngineKind::Candle,
            "unproven-stop-proof",
            None,
        ))
        .expect("begin lifecycle");

    assert!(active.leave_open_for_reconciliation());
    assert!(
        !active.leave_open_for_reconciliation(),
        "only the first reconciliation transition may consume the STOP permit"
    );
    assert_eq!(
        active
            .stop(Some(0), "must-not-convert-left-open-to-success")
            .expect("left-open state remains inspectable"),
        StopRecordOutcome::LeftOpenForReconciliation,
        "a later caller must not mistake an abandoned STOP permit for an already-recorded STOP"
    );
    drop(active);

    let store = Arc::new(InMemoryProcessLedgerStore::default());
    drain
        .drain_available_to(store.clone())
        .await
        .expect("drain open lifecycle");
    let events = store.events();
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0], LedgerEvent::Start(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_reserved_shutdown_reports_one_recorded_stop_after_serialization() {
    let (batcher, drain) = LedgerBatcher::manual_for_tests(
        LedgerBatcherConfig {
            capacity: 2,
            batch_size: 2,
            flush_interval: std::time::Duration::from_millis(250),
        },
        Arc::new(InMemoryOverflowSink::default()),
    )
    .expect("manual concurrency batcher");
    let reservation = batcher
        .try_reserve_lifecycles(1)
        .expect("reserve lifecycle")
        .pop()
        .expect("one reservation");
    let active = Arc::new(
        reservation
            .begin(ProcessStart::new(
                ProcessEngineKind::Candle,
                "concurrent-stop-proof",
                None,
            ))
            .expect("begin lifecycle"),
    );

    let barrier = Arc::new(tokio::sync::Barrier::new(9));
    let mut tasks = Vec::new();
    for _ in 0..8 {
        let active = active.clone();
        let barrier = barrier.clone();
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;
            active.stop(Some(0), "concurrent-stop")
        }));
    }
    barrier.wait().await;
    let mut recorded = 0;
    let mut already_stopped = 0;
    for task in tasks {
        match task
            .await
            .expect("stop task joins")
            .expect("stop is infallible")
        {
            StopRecordOutcome::Recorded => recorded += 1,
            StopRecordOutcome::AlreadyStopped => already_stopped += 1,
            StopRecordOutcome::LeftOpenForReconciliation => {
                panic!("concurrent graceful STOP was not abandoned")
            }
            StopRecordOutcome::DurabilityUnconfirmed => {
                panic!("unacknowledged STOP is impossible on the synchronous stop path")
            }
        }
    }
    assert_eq!(recorded, 1);
    assert_eq!(already_stopped, 7);
    drop(active);

    let store = Arc::new(InMemoryProcessLedgerStore::default());
    drain
        .drain_available_to(store.clone())
        .await
        .expect("drain concurrent lifecycle");
    assert_eq!(
        store
            .events()
            .iter()
            .filter(|event| matches!(event, LedgerEvent::Stop(_)))
            .count(),
        1
    );
}

#[tokio::test]
async fn replay_equivalent_spawn_stop_sequences_produce_identical_row_sets() {
    let first = run_sequence_for_replay_equivalence().await;
    let second = run_sequence_for_replay_equivalence().await;

    assert_eq!(first, second);
}

#[tokio::test]
async fn ledger_decorator_tests_spawn_records_start_with_handle_metadata_and_capabilities() {
    let fixture = DecoratorFixture::new(vec![ProcessStatus::Running]);
    let spec = fixture.process_spec("model-process:llama");

    let handle = fixture
        .decorator
        .spawn(spec)
        .await
        .expect("decorator spawn succeeds");
    fixture.drain().await;

    let events = fixture.store.events();
    assert_eq!(events.len(), 1);
    match &events[0] {
        LedgerEvent::Start(start) => {
            assert_eq!(start.process_uuid, handle.id);
            assert_eq!(start.os_pid, Some(4242));
            assert_eq!(start.sandbox_adapter_id.as_deref(), Some("stub"));
            assert_eq!(start.sandbox_internal_id.as_deref(), Some("stub-internal"));
            assert_eq!(start.engine_kind, ProcessEngineKind::LlamaCpp);
            assert_eq!(start.owner_role, "KERNEL_BUILDER");
            assert_eq!(start.owner_wp.as_deref(), Some("WP-KERNEL-004"));
            assert_eq!(start.mt_id.as_deref(), Some("MT-053"));
            assert_eq!(start.metadata_jsonb["model_id"], "llama");
            assert_eq!(start.sandbox_capabilities_snapshot["adapter_id"], "stub");
        }
        other => panic!("expected START event, got {other:?}"),
    }
}

#[tokio::test]
async fn ledger_decorator_tests_kill_records_one_stop_with_stop_reason() {
    let fixture = DecoratorFixture::new(vec![ProcessStatus::Running]);
    let handle = fixture
        .decorator
        .spawn(fixture.process_spec("validation-job:compile"))
        .await
        .expect("spawn");

    fixture
        .decorator
        .kill(&handle, Signal::Kill)
        .await
        .expect("kill");
    fixture.drain().await;

    let events = fixture.store.events();
    assert_eq!(events.len(), 2);
    match &events[1] {
        LedgerEvent::Stop(stop) => {
            assert_eq!(stop.process_uuid, handle.id);
            assert_eq!(stop.stop_reason.as_deref(), Some("kill:kill"));
            assert_eq!(stop.exit_code, None);
        }
        other => panic!("expected STOP event, got {other:?}"),
    }
}

#[tokio::test]
async fn ledger_decorator_tests_terminal_status_records_stop_once() {
    let fixture = DecoratorFixture::new(vec![
        ProcessStatus::Exited { code: 7 },
        ProcessStatus::Exited { code: 7 },
    ]);
    let handle = fixture
        .decorator
        .spawn(fixture.process_spec("validation-job:test"))
        .await
        .expect("spawn");

    assert_eq!(
        fixture.decorator.status(&handle).await.unwrap(),
        ProcessStatus::Exited { code: 7 }
    );
    assert_eq!(
        fixture.decorator.status(&handle).await.unwrap(),
        ProcessStatus::Exited { code: 7 }
    );
    fixture.drain().await;

    let stops = fixture
        .store
        .events()
        .into_iter()
        .filter(|event| matches!(event, LedgerEvent::Stop(_)))
        .collect::<Vec<_>>();
    assert_eq!(stops.len(), 1);
    match &stops[0] {
        LedgerEvent::Stop(stop) => {
            assert_eq!(stop.process_uuid, handle.id);
            assert_eq!(stop.exit_code, Some(7));
            assert_eq!(stop.stop_reason.as_deref(), Some("status:exited"));
        }
        _ => unreachable!(),
    }
}

#[tokio::test]
async fn ledger_decorator_tests_spawn_path_p95_overhead_stays_under_5ms() {
    let fixture = DecoratorFixture::new(vec![ProcessStatus::Running]);
    let mut elapsed = Vec::new();

    for index in 0..100 {
        let started = std::time::Instant::now();
        fixture
            .decorator
            .spawn(fixture.process_spec(&format!("model-process:latency-{index}")))
            .await
            .expect("spawn");
        elapsed.push(started.elapsed());
    }

    elapsed.sort();
    let p95 = elapsed[(elapsed.len() * 95 / 100).min(elapsed.len() - 1)];
    assert!(
        p95 < std::time::Duration::from_millis(5),
        "LedgerDecorator spawn p95 exceeded MT-053 budget: {p95:?}"
    );
}

#[tokio::test]
async fn bootstrap_with_ledger_wraps_registered_adapters_without_changing_selection() {
    let store = InMemoryProcessLedgerStore::default();
    let overflow = InMemoryOverflowSink::default();
    let (batcher, drain) = LedgerBatcher::manual_for_tests(
        LedgerBatcherConfig {
            capacity: 16,
            batch_size: 4,
            flush_interval: std::time::Duration::from_millis(250),
        },
        Arc::new(overflow),
    )
    .expect("manual batcher");
    let registry = build_registry_from_adapters_with_ledger(
        AdapterId::new("stub"),
        vec![Arc::new(RecordingAdapter::new(vec![
            ProcessStatus::Running,
        ]))],
        true,
        Some(batcher),
    )
    .expect("registry with ledger");

    assert_eq!(registry.default_adapter_id(), &AdapterId::new("stub"));
    assert!(registry.docker_explicit_opt_in());
    let handle = registry
        .default()
        .spawn(DecoratorFixture::new(vec![]).process_spec("model-process:bootstrap"))
        .await
        .expect("decorated default spawn");
    drain
        .drain_available_to(Arc::new(store.clone()))
        .await
        .expect("drain");

    let events = store.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].process_uuid(), handle.id);
}

// MT-007: a ledger flush/store failure must be OBSERVABLE (counted + logged
// loud) instead of being silently swallowed. Before the fix, the drain path's
// flush errors were the only observable ones and the background writer used
// `let _ = flush_batch(...)`, so a dropped ProcessStart/ProcessStop row produced
// no signal at all. These tests prove the failure is now surfaced.

#[tokio::test]
async fn drain_flush_store_failure_is_observable_and_counted() {
    let failing = FailingProcessLedgerStore::default();
    let overflow = InMemoryOverflowSink::default();
    let (batcher, drain) = LedgerBatcher::manual_for_tests(
        LedgerBatcherConfig {
            capacity: 16,
            batch_size: 4,
            flush_interval: std::time::Duration::from_millis(250),
        },
        Arc::new(overflow),
    )
    .expect("manual batcher");

    // Serialize against other global-counter tests so the exact `== 2` delta
    // below is not perturbed by a concurrent flush-failure in a sibling test.
    let _counter_guard = lock_flush_counter();
    let before = flush_failed_row_count();

    // Enqueue two rows whose identity we can later confirm was counted.
    batcher
        .record_start(
            ProcessStart::new(
                ProcessEngineKind::SandboxContainer,
                "KERNEL_BUILDER",
                Some("WP-KERNEL-004".to_string()),
            )
            .with_process_uuid(uuid::Uuid::nil()),
        )
        .expect("enqueue start");
    batcher
        .record_stop(ProcessStop::from_start(
            &ProcessStart::new(
                ProcessEngineKind::SandboxContainer,
                "KERNEL_BUILDER",
                Some("WP-KERNEL-004".to_string()),
            )
            .with_process_uuid(uuid::Uuid::nil()),
            Some(0),
        ))
        .expect("enqueue stop");

    // The drain path propagates the store error (manual-drain contract) AND
    // records the loss observably before returning it.
    let result = drain.drain_available_to(Arc::new(failing.clone())).await;
    assert!(
        result.is_err(),
        "failing store must surface an error to the drain caller"
    );

    // Observable signal 1: per-drain counter incremented by the number of rows.
    assert_eq!(
        drain.flush_failed_rows(),
        2,
        "drain must count both dropped rows"
    );
    // Observable signal 2: process-wide counter incremented.
    assert_eq!(
        flush_failed_row_count() - before,
        2,
        "process-wide flush-failed counter must reflect the dropped rows"
    );
    // The store actually rejected the write (no rows persisted).
    assert_eq!(failing.attempts(), 1);
}

#[tokio::test]
async fn background_writer_flush_failure_is_counted_not_silently_dropped() {
    let failing = Arc::new(FailingProcessLedgerStore::default());
    let overflow = Arc::new(InMemoryOverflowSink::default());
    // Serialize against the exact-delta test so neither perturbs the other's
    // process-wide counter window.
    let _counter_guard = lock_flush_counter();
    let before = flush_failed_row_count();

    // batch_size = 1 forces an immediate flush attempt per enqueued row through
    // the background `run_writer` select-loop path (the one that previously used
    // `let _ = flush_batch(...)`).
    let (batcher, mut join) = LedgerBatcher::spawn(
        failing.clone(),
        overflow,
        LedgerBatcherConfig {
            capacity: 16,
            batch_size: 1,
            flush_interval: std::time::Duration::from_millis(10),
        },
    );

    batcher
        .record_start(ProcessStart::new(
            ProcessEngineKind::SandboxContainer,
            "KERNEL_BUILDER",
            Some("WP-KERNEL-004".to_string()),
        ))
        .expect("enqueue start");

    // Wait until the background writer has observed and counted the failure.
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            if flush_failed_row_count() > before {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("background writer must count the flush failure, not silently drop it");

    assert!(
        flush_failed_row_count() > before,
        "background writer flush failure must be observable via the global counter"
    );

    // Drop the batcher to close the channel. A permanently failing store keeps
    // its accepted row for retry, so explicitly cancel and await the test task
    // instead of letting a timed-out JoinHandle detach.
    drop(batcher);
    if tokio::time::timeout(std::time::Duration::from_millis(50), &mut join)
        .await
        .is_err()
    {
        join.abort();
        let _ = join.await;
    }
}

#[tokio::test]
async fn retained_failed_batch_backpressures_then_persists_reserved_stop_after_recovery() {
    // This test deliberately creates background flush failures, so serialize it
    // with every exact process-wide failure-counter assertion.
    let _counter_guard = lock_flush_counter();
    let store = Arc::new(RecoverableProcessLedgerStore::default());
    let overflow = Arc::new(InMemoryOverflowSink::default());
    let (batcher, join) = LedgerBatcher::spawn(
        store.clone(),
        overflow.clone(),
        LedgerBatcherConfig {
            capacity: 2,
            batch_size: 1,
            // Event arrival and close drive every attempt in this proof; a long
            // tick prevents an unrelated retry from changing the exact setup.
            flush_interval: std::time::Duration::from_secs(60),
        },
    );

    let reservation = batcher
        .try_reserve_lifecycles(1)
        .expect("reserve START and STOP at capacity two")
        .pop()
        .expect("one lifecycle reservation");
    let reserved_start = ProcessStart::new(
        ProcessEngineKind::Candle,
        "reserved-stop-recovery-proof",
        None,
    );
    let reserved_uuid = reserved_start.process_uuid;
    let active = reservation
        .begin(reserved_start)
        .expect("enqueue reserved START");

    wait_for_store_attempts(&store, 1).await;

    let unrelated_start = ProcessStart::new(
        ProcessEngineKind::OfficialCliBridge,
        "unrelated-failed-row",
        None,
    );
    let unrelated_uuid = unrelated_start.process_uuid;
    batcher
        .record_start(unrelated_start)
        .expect("writer receive freed capacity for unrelated row");

    wait_for_store_attempts(&store, 2).await;
    assert!(
        store.events().is_empty(),
        "both failed attempts must remain retained, not appear persisted"
    );

    assert_eq!(
        active
            .stop(Some(0), "store-recovered-during-close")
            .expect("reserved STOP permit remains enqueueable"),
        StopRecordOutcome::Recorded
    );
    drop(active);

    store.recover();
    batcher.begin_close();
    tokio::time::timeout(std::time::Duration::from_secs(2), join)
        .await
        .expect("writer must drain after store recovery")
        .expect("writer task must join")
        .expect("recovered store must flush every accepted row");

    let events = store.events();
    assert_eq!(
        events.len(),
        3,
        "START, unrelated row, and STOP all persist"
    );
    assert_eq!(events[0].kind(), LedgerEventKind::Start);
    assert_eq!(events[0].process_uuid(), reserved_uuid);
    assert_eq!(events[1].kind(), LedgerEventKind::Start);
    assert_eq!(events[1].process_uuid(), unrelated_uuid);
    assert_eq!(events[2].kind(), LedgerEventKind::Stop);
    assert_eq!(events[2].process_uuid(), reserved_uuid);
    assert_eq!(
        store.attempts(),
        4,
        "two retained failures followed by one recovered batch and one STOP flush"
    );
    assert!(
        overflow.events().is_empty(),
        "the writer must never overflow a row it already accepted"
    );
}

#[tokio::test]
async fn retained_ledger_batcher_drains_spawned_writer_once() {
    let store = Arc::new(InMemoryProcessLedgerStore::default());
    let retained = RetainedLedgerBatcher::spawn(
        store.clone(),
        Arc::new(InMemoryOverflowSink::default()),
        LedgerBatcherConfig {
            capacity: 16,
            batch_size: 1,
            flush_interval: std::time::Duration::from_millis(10),
        },
    );
    let ledger = retained.ledger();

    ledger
        .record_start(ProcessStart::new(
            ProcessEngineKind::OfficialCliBridge,
            "OFFICIAL_CLI_BRIDGE",
            Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".to_string()),
        ))
        .expect("enqueue retained writer start");

    let outcome = retained
        .drain_and_join(std::time::Duration::from_secs(2))
        .await;
    assert!(
        matches!(outcome, LedgerDrainJoinOutcome::Flushed),
        "retained writer must flush and join cleanly, got {outcome:?}"
    );
    assert_eq!(store.events().len(), 1);

    let second = retained
        .drain_and_join(std::time::Duration::from_secs(2))
        .await;
    assert!(
        matches!(second, LedgerDrainJoinOutcome::AlreadyDrained),
        "retained writer drain must be single-consumer, got {second:?}"
    );
}

#[tokio::test]
async fn timed_out_retained_writer_is_aborted_and_awaited_not_detached() {
    let store = Arc::new(PendingProcessLedgerStore::default());
    let retained = RetainedLedgerBatcher::spawn(
        store.clone(),
        Arc::new(InMemoryOverflowSink::default()),
        LedgerBatcherConfig {
            capacity: 2,
            batch_size: 1,
            flush_interval: std::time::Duration::from_secs(60),
        },
    );
    retained
        .ledger()
        .record_start(ProcessStart::new(
            ProcessEngineKind::Candle,
            "pending-store-timeout-proof",
            None,
        ))
        .expect("enqueue row into pending store");

    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while store.active_writes() == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("writer must enter pending store call");

    let outcome = retained
        .drain_and_join(std::time::Duration::from_millis(50))
        .await;
    assert!(
        matches!(outcome, LedgerDrainJoinOutcome::TimedOut),
        "pending store must force bounded drain timeout, got {outcome:?}"
    );
    assert_eq!(
        store.active_writes(),
        0,
        "timeout must abort and await the task instead of detaching it"
    );
}

#[test]
fn no_direct_lifecycle_insert_outside_process_ledger_module() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let src = manifest_dir.join("src");
    let process_ledger_dir = src.join("process_ledger");
    let mut offenders = Vec::new();
    collect_direct_lifecycle_inserts(&src, &process_ledger_dir, &mut offenders);

    assert!(
        offenders.is_empty(),
        "kernel_process_lifecycle INSERT must stay inside process_ledger: {offenders:?}"
    );
}

#[test]
fn direct_lifecycle_insert_guard_recognizes_only_and_whitespace_variants() {
    assert!(contains_direct_lifecycle_insert(
        "INSERT INTO kernel_process_lifecycle (process_uuid) VALUES ($1)"
    ));
    assert!(contains_direct_lifecycle_insert(
        "insert\ninto\nonly\tkernel_process_lifecycle (process_uuid) values ($1)"
    ));
    assert!(!contains_direct_lifecycle_insert(
        "SELECT * FROM ONLY kernel_process_lifecycle"
    ));
}

#[test]
fn production_process_ledger_writers_are_retained_until_shutdown() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let production_sources = [
        "src/api/mod.rs",
        "src/swarm_orchestration/production_factory.rs",
    ];
    let mut detached_spawns = Vec::new();

    for relative_path in production_sources {
        let path = manifest_dir.join(relative_path);
        let source = std::fs::read_to_string(&path).expect("read production source");
        for forbidden in [
            "let (ledger, _writer) = LedgerBatcher::spawn(",
            "let (ledger, _writer) = crate::process_ledger::LedgerBatcher::spawn(",
            "let (process_ledger, _writer) = LedgerBatcher::spawn(",
            "let (process_ledger, _writer) = crate::process_ledger::LedgerBatcher::spawn(",
        ] {
            if source.contains(forbidden) {
                detached_spawns.push(format!("{relative_path}: {forbidden}"));
            }
        }
    }

    let api_source = std::fs::read_to_string(manifest_dir.join("src/api/mod.rs"))
        .expect("read api routes source");
    assert!(
        api_source.contains("RetainedLedgerBatcher::spawn("),
        "production API routes must create a retained process-ledger writer for operator-chat lanes"
    );
    assert!(
        !api_source.contains("routes_with_runtime(state).router"),
        "api::routes(state) must not discard ApiRouteRuntime after extracting the router"
    );
    assert!(
        api_source.contains("Extension(runtime)"),
        "api::routes(state) must retain ApiRouteRuntime on the returned router for fixture/server callers"
    );
    assert!(
        detached_spawns.is_empty(),
        "production process-ledger writers must retain and drain their JoinHandle at shutdown: {detached_spawns:?}"
    );
}

async fn run_sequence_for_replay_equivalence() -> Vec<serde_json::Value> {
    let store = InMemoryProcessLedgerStore::default();
    let overflow = InMemoryOverflowSink::default();
    let (batcher, drain) = LedgerBatcher::manual_for_tests(
        LedgerBatcherConfig {
            capacity: 16,
            batch_size: 4,
            flush_interval: std::time::Duration::from_millis(250),
        },
        Arc::new(overflow),
    )
    .expect("manual batcher");

    let start = ProcessStart::new(
        ProcessEngineKind::SandboxContainer,
        "KERNEL_BUILDER",
        Some("WP-KERNEL-004".to_string()),
    )
    .with_process_uuid(uuid::Uuid::nil())
    .with_sandbox_adapter_id("stub")
    .with_sandbox_internal_id("stable-internal")
    .with_metadata_jsonb(json!({"stable": true}));
    let stop = ProcessStop::from_start(&start, Some(0)).with_stop_reason("status:exited");

    batcher.record_start(start).expect("start");
    batcher.record_stop(stop).expect("stop");
    drain
        .drain_available_to(Arc::new(store.clone()))
        .await
        .expect("drain");

    store
        .events()
        .into_iter()
        .map(|event| match event {
            LedgerEvent::Start(start) => json!({
                "kind": "START",
                "process_uuid": start.process_uuid,
                "sandbox_adapter_id": start.sandbox_adapter_id,
                "sandbox_internal_id": start.sandbox_internal_id,
                "engine_kind": start.engine_kind,
                "owner_role": start.owner_role,
                "owner_wp": start.owner_wp,
                "metadata_jsonb": start.metadata_jsonb,
            }),
            LedgerEvent::Stop(stop) => json!({
                "kind": "STOP",
                "process_uuid": stop.process_uuid,
                "sandbox_adapter_id": stop.sandbox_adapter_id,
                "sandbox_internal_id": stop.sandbox_internal_id,
                "engine_kind": stop.engine_kind,
                "owner_role": stop.owner_role,
                "owner_wp": stop.owner_wp,
                "exit_code": stop.exit_code,
                "stop_reason": stop.stop_reason,
                "metadata_jsonb": stop.metadata_jsonb,
            }),
        })
        .collect()
}

#[derive(Clone, Default)]
struct InMemoryProcessLedgerStore {
    events: Arc<Mutex<Vec<LedgerEvent>>>,
}

impl InMemoryProcessLedgerStore {
    fn events(&self) -> Vec<LedgerEvent> {
        self.events.lock().unwrap().clone()
    }
}

#[async_trait]
impl ProcessLedgerStore for InMemoryProcessLedgerStore {
    async fn write_batch(&self, events: Vec<LedgerEvent>) -> Result<(), ProcessLedgerError> {
        self.events.lock().unwrap().extend(events);
        Ok(())
    }
}

/// Store that always rejects `write_batch`, used to prove flush/store failures
/// are observable (counted + logged) rather than silently swallowed (MT-007).
#[derive(Clone, Default)]
struct FailingProcessLedgerStore {
    attempts: Arc<AtomicU64>,
}

impl FailingProcessLedgerStore {
    fn attempts(&self) -> u64 {
        self.attempts.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl ProcessLedgerStore for FailingProcessLedgerStore {
    async fn write_batch(&self, _events: Vec<LedgerEvent>) -> Result<(), ProcessLedgerError> {
        self.attempts.fetch_add(1, Ordering::SeqCst);
        Err(ProcessLedgerError::Store(
            "simulated ledger store write failure".to_string(),
        ))
    }
}

#[derive(Clone)]
struct RecoverableProcessLedgerStore {
    fail_writes: Arc<AtomicBool>,
    attempts: Arc<AtomicU64>,
    events: Arc<Mutex<Vec<LedgerEvent>>>,
}

impl Default for RecoverableProcessLedgerStore {
    fn default() -> Self {
        Self {
            fail_writes: Arc::new(AtomicBool::new(true)),
            attempts: Arc::new(AtomicU64::new(0)),
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl RecoverableProcessLedgerStore {
    fn fail(&self) {
        self.fail_writes.store(true, Ordering::SeqCst);
    }

    fn recover(&self) {
        self.fail_writes.store(false, Ordering::SeqCst);
    }

    fn attempts(&self) -> u64 {
        self.attempts.load(Ordering::SeqCst)
    }

    fn events(&self) -> Vec<LedgerEvent> {
        self.events.lock().unwrap().clone()
    }
}

#[async_trait]
impl ProcessLedgerStore for RecoverableProcessLedgerStore {
    async fn write_batch(&self, events: Vec<LedgerEvent>) -> Result<(), ProcessLedgerError> {
        self.attempts.fetch_add(1, Ordering::SeqCst);
        if self.fail_writes.load(Ordering::SeqCst) {
            return Err(ProcessLedgerError::Store(
                "simulated transient ledger store failure".to_string(),
            ));
        }
        self.events.lock().unwrap().extend(events);
        Ok(())
    }
}

async fn wait_for_store_attempts(store: &RecoverableProcessLedgerStore, expected: u64) {
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while store.attempts() < expected {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("store did not reach {expected} attempts"));
}

#[derive(Clone, Default)]
struct PendingProcessLedgerStore {
    active_writes: Arc<AtomicUsize>,
}

impl PendingProcessLedgerStore {
    fn active_writes(&self) -> usize {
        self.active_writes.load(Ordering::SeqCst)
    }
}

struct ActiveWriteGuard {
    active_writes: Arc<AtomicUsize>,
}

impl Drop for ActiveWriteGuard {
    fn drop(&mut self) {
        self.active_writes.fetch_sub(1, Ordering::SeqCst);
    }
}

#[async_trait]
impl ProcessLedgerStore for PendingProcessLedgerStore {
    async fn write_batch(&self, _events: Vec<LedgerEvent>) -> Result<(), ProcessLedgerError> {
        self.active_writes.fetch_add(1, Ordering::SeqCst);
        let _guard = ActiveWriteGuard {
            active_writes: Arc::clone(&self.active_writes),
        };
        std::future::pending::<()>().await;
        Err(ProcessLedgerError::Store(
            "pending store unexpectedly resumed".to_string(),
        ))
    }
}

#[derive(Clone, Default)]
struct InMemoryOverflowSink {
    events: Arc<Mutex<Vec<LedgerOverflowEvent>>>,
}

impl InMemoryOverflowSink {
    fn events(&self) -> Vec<LedgerOverflowEvent> {
        self.events.lock().unwrap().clone()
    }
}

impl ProcessLedgerOverflowSink for InMemoryOverflowSink {
    fn emit_overflow(&self, event: LedgerOverflowEvent) -> Result<(), ProcessLedgerError> {
        self.events.lock().unwrap().push(event);
        Ok(())
    }
}

struct DecoratorFixture {
    decorator: LedgerDecorator,
    store: InMemoryProcessLedgerStore,
    drain: handshake_core::process_ledger::ProcessLedgerDrain,
}

impl DecoratorFixture {
    fn new(statuses: Vec<ProcessStatus>) -> Self {
        let store = InMemoryProcessLedgerStore::default();
        let overflow = InMemoryOverflowSink::default();
        let (batcher, drain) = LedgerBatcher::manual_for_tests(
            LedgerBatcherConfig {
                capacity: 16,
                batch_size: 4,
                flush_interval: std::time::Duration::from_millis(250),
            },
            Arc::new(overflow),
        )
        .expect("manual batcher");
        let decorator = LedgerDecorator::new(Arc::new(RecordingAdapter::new(statuses)), batcher);
        Self {
            decorator,
            store,
            drain,
        }
    }

    async fn drain(&self) {
        self.drain
            .drain_available_to(Arc::new(self.store.clone()))
            .await
            .expect("drain decorator events");
    }

    fn process_spec(&self, id: &str) -> ProcessSpec {
        ProcessSpec {
            id: AdapterId::new(id),
            image_or_root: ImageRef::new("test-image"),
            cmd: vec!["run".to_string()],
            env: BTreeMap::new(),
            cwd: None,
            binds: Vec::new(),
            net_policy: NetPolicy::DenyAll,
            resource_limits: ResourceLimits::default(),
            idle_timeout_ms: None,
            required_capabilities: BTreeSet::new(),
            trust_class: TrustClass::default(),
            metadata: BTreeMap::from([
                ("engine_kind".to_string(), "llama_cpp".to_string()),
                ("model_id".to_string(), "llama".to_string()),
                ("role_id".to_string(), "KERNEL_BUILDER".to_string()),
                ("wp_id".to_string(), "WP-KERNEL-004".to_string()),
                ("mt_id".to_string(), "MT-053".to_string()),
            ]),
        }
    }
}

struct RecordingAdapter {
    statuses: Mutex<Vec<ProcessStatus>>,
    capabilities: AdapterCapabilities,
}

impl RecordingAdapter {
    fn new(statuses: Vec<ProcessStatus>) -> Self {
        let mut capabilities = default_no_op_capabilities();
        capabilities.adapter_id = AdapterId::new("stub");
        Self {
            statuses: Mutex::new(statuses.into_iter().rev().collect()),
            capabilities,
        }
    }
}

#[async_trait]
impl SandboxAdapter for RecordingAdapter {
    async fn spawn(&self, _spec: ProcessSpec) -> Result<ProcessHandle, SandboxAdapterError> {
        Ok(ProcessHandle::new(
            AdapterId::new("stub"),
            Some(4242),
            "stub-internal",
        ))
    }

    async fn exec(
        &self,
        _handle: &ProcessHandle,
        _cmd: Command,
    ) -> Result<ExecResult, SandboxAdapterError> {
        Ok(ExecResult {
            exit_code: 0,
            stdout: Bytes::new(),
            stderr: Bytes::new(),
            duration_ms: 1,
        })
    }

    async fn fs_bind(
        &self,
        _handle: &ProcessHandle,
        _host_path: PathBuf,
        _guest_path: PathBuf,
        _mode: BindMode,
    ) -> Result<(), SandboxAdapterError> {
        Ok(())
    }

    async fn net_policy(
        &self,
        _handle: &ProcessHandle,
        _policy: NetPolicy,
    ) -> Result<(), SandboxAdapterError> {
        Ok(())
    }

    async fn kill(
        &self,
        _handle: &ProcessHandle,
        _signal: Signal,
    ) -> Result<(), SandboxAdapterError> {
        Ok(())
    }

    async fn status(&self, _handle: &ProcessHandle) -> Result<ProcessStatus, SandboxAdapterError> {
        Ok(self
            .statuses
            .lock()
            .unwrap()
            .pop()
            .unwrap_or(ProcessStatus::Running))
    }

    async fn exit_code(&self, handle: &ProcessHandle) -> Result<Option<i32>, SandboxAdapterError> {
        match self.status(handle).await? {
            ProcessStatus::Exited { code } => Ok(Some(code)),
            _ => Ok(None),
        }
    }

    fn capabilities(&self) -> AdapterCapabilities {
        self.capabilities.clone()
    }
}

fn collect_direct_lifecycle_inserts(
    dir: &std::path::Path,
    allowed_dir: &std::path::Path,
    offenders: &mut Vec<String>,
) {
    for entry in std::fs::read_dir(dir).expect("read source dir") {
        let entry = entry.expect("source entry");
        let path = entry.path();
        if path == allowed_dir || path.starts_with(allowed_dir) {
            continue;
        }
        if path.is_dir() {
            collect_direct_lifecycle_inserts(&path, allowed_dir, offenders);
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("read source file");
        if contains_direct_lifecycle_insert(&source) {
            offenders.push(path.display().to_string());
        }
    }
}

fn contains_direct_lifecycle_insert(source: &str) -> bool {
    let normalized = source
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    normalized.contains("insert into kernel_process_lifecycle")
        || normalized.contains("insert into only kernel_process_lifecycle")
}

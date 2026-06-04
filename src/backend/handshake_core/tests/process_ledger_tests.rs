use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use async_trait::async_trait;
use sqlx::{postgres::PgPoolOptions, Row};
use tokio::sync::Notify;

use handshake_core::{
    kernel::KernelEventType,
    process_ledger::{
        is_degraded, LedgerEvent, LedgerEventKind, LedgerOverflowEvent, PostgresProcessLedgerStore,
        ProcessEngineKind, ProcessLedgerError, ProcessLedgerOverflowSink, ProcessLedgerStore,
        ProcessLedgerWriter, ProcessStart, ProcessStop, WriterConfig,
        PROCESS_LEDGER_DEFAULT_CHANNEL_CAPACITY, PROCESS_LEDGER_MIGRATION_SQL,
        PROCESS_LEDGER_RING_CAPACITY, PROCESS_LEDGER_TABLE_NAME, PROCESS_START_INSERT_SQL,
        PROCESS_STOP_UPSERT_SQL,
    },
};

const PROCESS_LEDGER_SOURCE_FILES: &[&str] = &[
    "src/process_ledger/mod.rs",
    "src/process_ledger/table.rs",
    "src/process_ledger/writer.rs",
];

#[test]
fn postgres_table_contract_declares_process_lifecycle_primitive() {
    assert_eq!(PROCESS_LEDGER_TABLE_NAME, "kernel_process_lifecycle");
    assert_eq!(
        PROCESS_LEDGER_DEFAULT_CHANNEL_CAPACITY,
        PROCESS_LEDGER_RING_CAPACITY
    );
    assert_eq!(PROCESS_LEDGER_DEFAULT_CHANNEL_CAPACITY, 10_000);

    let ddl = PROCESS_LEDGER_MIGRATION_SQL;
    assert!(ddl.contains("CREATE TABLE IF NOT EXISTS kernel_process_lifecycle"));
    assert!(ddl.contains("process_uuid UUID PRIMARY KEY"));
    assert!(ddl.contains("process_id UUID GENERATED ALWAYS AS (process_uuid) STORED"));
    assert!(ddl.contains("os_pid BIGINT"));
    assert!(ddl.contains("adapter_id TEXT GENERATED ALWAYS AS (sandbox_adapter_id) STORED"));
    assert!(ddl.contains("sandbox_internal_id TEXT"));
    assert!(ddl.contains("started_at TIMESTAMPTZ NOT NULL"));
    assert!(ddl.contains("spawned_at_utc TIMESTAMPTZ GENERATED ALWAYS AS (started_at) STORED"));
    assert!(ddl.contains("stopped_at TIMESTAMPTZ"));
    assert!(ddl.contains("stop_reason TEXT"));
    assert!(ddl.contains("sandbox_capabilities_snapshot JSONB NOT NULL DEFAULT '{}'::jsonb"));
    assert!(ddl.contains("metadata_jsonb JSONB NOT NULL DEFAULT '{}'::jsonb"));
    assert!(ddl.contains("idx_kernel_process_lifecycle_parent_session_started"));
    assert!(ddl.contains("idx_kernel_process_lifecycle_engine_started"));
    assert!(ddl.contains("idx_kernel_process_lifecycle_os_pid"));
    assert!(ddl.contains("idx_kernel_process_lifecycle_adapter_spawned"));
    assert!(ddl.contains("idx_kernel_process_lifecycle_wp_spawned"));

    for engine in [
        "llamacpp",
        "candle",
        "abliteration_tool",
        "sandbox_container",
        "mechanical_job",
        "asr_worker",
        "comfyui_worker",
        "plugin_process",
        "helper_subprocess",
        "external_compat",
        "webview2_cdp",
        "official_cli_bridge",
    ] {
        assert!(ddl.contains(engine), "missing engine kind {engine}");
    }

    assert!(!ddl.to_ascii_lowercase().contains("sqlite"));
}

#[test]
fn overflow_event_is_registered_as_event_ledger_type() {
    assert_eq!(
        KernelEventType::FrEvtLedgerOverflow.as_str(),
        "FR_EVT_LEDGER_OVERFLOW"
    );
    assert_eq!(
        KernelEventType::try_from("FR_EVT_LEDGER_OVERFLOW").unwrap(),
        KernelEventType::FrEvtLedgerOverflow
    );
}

#[test]
fn process_uuid_uses_uuid_v7_and_stop_upsert_recovers_missing_start() {
    let start = start_event("owner", "WP-KERNEL-004");
    assert_eq!(start.process_uuid.get_version_num(), 7);

    let stop = ProcessStop::from_start(&start, Some(0));
    assert_eq!(stop.process_uuid, start.process_uuid);
    assert_eq!(stop.os_pid, start.os_pid);
    assert_eq!(stop.engine_kind, start.engine_kind);
    assert_eq!(stop.owner_role, start.owner_role);

    assert!(PROCESS_START_INSERT_SQL.contains("ON CONFLICT (process_uuid) DO UPDATE"));
    assert!(PROCESS_STOP_UPSERT_SQL.contains("ON CONFLICT (process_uuid) DO UPDATE"));
    assert!(PROCESS_STOP_UPSERT_SQL.contains("started_at"));
}

#[tokio::test]
async fn writer_batches_start_and_stop_without_overflow() {
    let store = InMemoryProcessLedgerStore::default();
    let overflow = InMemoryOverflowSink::default();
    let config = WriterConfig {
        capacity: 8,
        batch_size: 2,
        flush_interval: Duration::from_millis(10),
    };
    let (writer, join) =
        ProcessLedgerWriter::spawn(Arc::new(store.clone()), Arc::new(overflow.clone()), config);

    let start = start_event("kernel_builder", "WP-KERNEL-004");
    let stop = ProcessStop::from_start(&start, Some(0));

    writer.append_start(start.clone()).expect("append start");
    writer.append_stop(stop.clone()).expect("append stop");

    store.wait_for_event_count(2).await;
    assert!(!writer.is_degraded());
    drop(writer);
    join.await
        .expect("process ledger writer task joins")
        .expect("process ledger writer drains cleanly");

    let events = store.events();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].kind(), LedgerEventKind::Start);
    assert_eq!(events[1].kind(), LedgerEventKind::Stop);
    assert_eq!(events[0].process_uuid(), start.process_uuid);
    assert_eq!(events[1].process_uuid(), stop.process_uuid);
    assert!(overflow.events().is_empty());
}

#[tokio::test]
async fn saturation_emits_overflow_receipts_and_clears_degraded_after_drain() {
    let store = InMemoryProcessLedgerStore::default();
    let overflow = InMemoryOverflowSink::default();
    let (writer, drain) =
        ProcessLedgerWriter::new_manual(128, Arc::new(overflow.clone())).expect("manual writer");

    let mut worst_append = Duration::ZERO;
    for index in 0..10_000 {
        let start = ProcessStart::new(
            ProcessEngineKind::MechanicalJob,
            format!("owner-{index}"),
            Some("WP-KERNEL-004".to_string()),
        );
        let started = Instant::now();
        writer
            .append_start(start)
            .expect("append never blocks on store");
        worst_append = worst_append.max(started.elapsed());
    }

    assert!(
        worst_append < Duration::from_millis(10),
        "append path waited too long: {worst_append:?}"
    );
    assert!(writer.is_degraded());
    assert!(is_degraded());

    let overflow_events = overflow.events();
    assert_eq!(overflow_events.len(), 10_000 - 128);
    assert_eq!(
        overflow_events.last().unwrap().overflow_count,
        (10_000 - 128) as u64
    );
    assert_eq!(
        overflow_events.last().unwrap().dropped_event_kind,
        LedgerEventKind::Start
    );

    drain
        .drain_available_to(Arc::new(store.clone()))
        .await
        .unwrap();
    assert_eq!(store.events().len(), 128);
    assert!(!writer.is_degraded());
    assert!(!is_degraded());
}

#[test]
fn overflow_payload_converts_to_typed_kernel_event() {
    let start = start_event("kernel_builder", "WP-KERNEL-004");
    let overflow = LedgerOverflowEvent::new(1, 128, LedgerEvent::Start(start.clone()));
    let event = overflow
        .to_kernel_event()
        .expect("overflow event should convert to EventLedger row");

    assert_eq!(event.event_type, KernelEventType::FrEvtLedgerOverflow);
    assert_eq!(event.payload["event_type"], "FR_EVT_LEDGER_OVERFLOW");
    assert_eq!(event.payload["overflow_count"], 1);
    assert_eq!(event.payload["capacity"], 128);
    assert_eq!(event.payload["dropped_event_kind"], "START");
    assert_eq!(
        event.payload["sampled_event_payload"]["process_uuid"],
        start.process_uuid.to_string()
    );
}

#[test]
fn new_process_ledger_sources_do_not_add_sqlite_paths() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    for relative_path in PROCESS_LEDGER_SOURCE_FILES {
        let path = std::path::Path::new(manifest_dir).join(relative_path);
        let source = std::fs::read_to_string(&path).expect("read process ledger source");
        assert!(
            !source.to_ascii_lowercase().contains("sqlite"),
            "{} introduced a SQLite token",
            path.display()
        );
    }
}

#[tokio::test]
#[ignore = "requires POSTGRES_TEST_URL; run with `cargo test --test process_ledger_tests -- --ignored`"]
async fn process_ledger_persists_start_and_stop_in_postgres(
) -> Result<(), Box<dyn std::error::Error>> {
    let postgres_url = std::env::var("POSTGRES_TEST_URL")?;
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&postgres_url)
        .await?;
    let store = PostgresProcessLedgerStore::new(pool.clone());
    store.apply_migration().await?;

    let start = start_event("kernel_builder", "WP-KERNEL-004");
    let stop = ProcessStop::from_start(&start, Some(0));
    store
        .write_batch(vec![
            LedgerEvent::Start(start.clone()),
            LedgerEvent::Stop(stop),
        ])
        .await?;

    let row = sqlx::query(
        r#"
        SELECT engine_kind, stopped_at IS NOT NULL AS has_stop
        FROM kernel_process_lifecycle
        WHERE process_uuid = $1::uuid
        "#,
    )
    .bind(start.process_uuid.to_string())
    .fetch_one(&pool)
    .await?;

    let engine_kind: String = row.get("engine_kind");
    let has_stop: bool = row.get("has_stop");
    assert_eq!(engine_kind, ProcessEngineKind::HelperSubprocess.as_str());
    assert!(has_stop);
    Ok(())
}

fn start_event(owner_role: &str, owner_wp: &str) -> ProcessStart {
    ProcessStart::new(
        ProcessEngineKind::HelperSubprocess,
        owner_role.to_string(),
        Some(owner_wp.to_string()),
    )
    .with_parent_session_id("SR-PROCESS-LEDGER-TEST")
    .with_work_profile_id("work-profile-test")
}

#[derive(Clone, Default)]
struct InMemoryProcessLedgerStore {
    events: Arc<Mutex<Vec<LedgerEvent>>>,
    notify: Arc<Notify>,
}

impl InMemoryProcessLedgerStore {
    fn events(&self) -> Vec<LedgerEvent> {
        self.events.lock().unwrap().clone()
    }

    async fn wait_for_event_count(&self, expected: usize) {
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            if self.events.lock().unwrap().len() >= expected {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "timed out waiting for ledger events"
            );
            self.notify.notified().await;
        }
    }
}

#[async_trait]
impl ProcessLedgerStore for InMemoryProcessLedgerStore {
    async fn write_batch(&self, events: Vec<LedgerEvent>) -> Result<(), ProcessLedgerError> {
        self.events.lock().unwrap().extend(events);
        self.notify.notify_waiters();
        Ok(())
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

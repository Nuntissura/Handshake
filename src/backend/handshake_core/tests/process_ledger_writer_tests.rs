use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use bytes::Bytes;
use serde_json::json;

use handshake_core::{
    process_ledger::{
        cap_metadata_jsonb, LedgerBatcher, LedgerBatcherConfig, LedgerEvent, LedgerEventKind,
        LedgerOverflowEvent, ProcessEngineKind, ProcessLedgerError, ProcessLedgerOverflowSink,
        ProcessLedgerStore, ProcessStart, ProcessStop, PROCESS_LEDGER_BATCH_SIZE,
        PROCESS_LEDGER_FLUSH_INTERVAL_MS, PROCESS_LEDGER_METADATA_CAP_BYTES,
        PROCESS_LEDGER_RING_CAPACITY,
    },
    sandbox::{
        build_registry_from_adapters_with_ledger, default_no_op_capabilities, AdapterCapabilities,
        AdapterId, BindMode, Command, ExecResult, ImageRef, LedgerDecorator, NetPolicy,
        ProcessHandle, ProcessSpec, ProcessStatus, ResourceLimits, SandboxAdapter,
        SandboxAdapterError, Signal, TrustClass,
    },
};

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
        if source.contains("INSERT INTO kernel_process_lifecycle") {
            offenders.push(path.display().to_string());
        }
    }
}

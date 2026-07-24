#![cfg(feature = "test-utils")]

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
use std::time::Duration;

use async_trait::async_trait;
use handshake_core::model_runtime::{
    CancellationToken, Embedding, GenerateRequest, KvCacheHandle, LoraStackHandle,
    ModelCapabilities, ModelId, ModelRuntime, ModelRuntimeError, ProviderKind, RuntimeBinding,
    Score, SteeringHookHandle, TokenStream,
};
use handshake_core::process_ledger::{
    LedgerBatcher, LedgerBatcherConfig, LedgerEvent, LedgerEventKind, NoopOverflowSink,
    ProcessEngineKind, ProcessLedgerError, ProcessLedgerStore, ProcessStart,
};
use handshake_core::swarm_orchestration::{
    CloudLaneFactoryConfig, CloudLiveRuntime, CloudRuntimeBuilder, ModelInstanceId,
    ModelSessionFactory, ProductionModelSessionFactory, RecordingSwarmSink, RunBudget,
    SpawnRequest, SwarmConfig, SwarmCoordinator, SwarmError,
};

#[derive(Default)]
struct CapturingStore {
    events: Mutex<Vec<LedgerEvent>>,
}

struct DelayedFirstWriteStore {
    events: Mutex<Vec<LedgerEvent>>,
    writes: AtomicUsize,
    first_write_delay: Duration,
}

impl DelayedFirstWriteStore {
    fn new(first_write_delay: Duration) -> Self {
        Self {
            events: Mutex::new(Vec::new()),
            writes: AtomicUsize::new(0),
            first_write_delay,
        }
    }

    fn events(&self) -> Vec<LedgerEvent> {
        self.events.lock().expect("delayed store poisoned").clone()
    }
}

#[async_trait]
impl ProcessLedgerStore for DelayedFirstWriteStore {
    async fn write_batch(&self, events: Vec<LedgerEvent>) -> Result<(), ProcessLedgerError> {
        if self.writes.fetch_add(1, Ordering::SeqCst) == 0 {
            tokio::time::sleep(self.first_write_delay).await;
        }
        self.events
            .lock()
            .expect("delayed store poisoned")
            .extend(events);
        Ok(())
    }
}

impl CapturingStore {
    fn events(&self) -> Vec<LedgerEvent> {
        self.events
            .lock()
            .expect("capturing store poisoned")
            .clone()
    }
}

#[async_trait]
impl ProcessLedgerStore for CapturingStore {
    async fn write_batch(&self, events: Vec<LedgerEvent>) -> Result<(), ProcessLedgerError> {
        self.events
            .lock()
            .expect("capturing store poisoned")
            .extend(events);
        Ok(())
    }
}

struct IdleRuntime;

#[async_trait]
impl ModelRuntime for IdleRuntime {
    async fn load(
        &mut self,
        _spec: handshake_core::model_runtime::LoadSpec,
    ) -> Result<ModelId, ModelRuntimeError> {
        Ok(ModelId::new_v7())
    }

    async fn unload(&mut self, _id: ModelId) -> Result<(), ModelRuntimeError> {
        Ok(())
    }

    fn generate(&self, _req: GenerateRequest) -> TokenStream {
        Box::pin(futures::stream::empty())
    }

    async fn score(&self, _id: ModelId, _sequence: Vec<u32>) -> Result<Score, ModelRuntimeError> {
        Ok(Score {
            token_logprobs: Vec::new(),
            mean_logprob: 0.0,
        })
    }

    async fn embed(&self, _id: ModelId, _text: &str) -> Result<Embedding, ModelRuntimeError> {
        Ok(Embedding { vector: Vec::new() })
    }

    fn capabilities(&self, _id: ModelId) -> Result<&ModelCapabilities, ModelRuntimeError> {
        Err(ModelRuntimeError::CapabilityNotSupported {
            capability: "test-only".to_string(),
            adapter: "idle-runtime".to_string(),
        })
    }

    fn kv_cache(&self, _id: ModelId) -> Result<KvCacheHandle, ModelRuntimeError> {
        Err(ModelRuntimeError::KvCacheError("test-only".to_string()))
    }

    fn lora_stack(&self, _id: ModelId) -> Result<LoraStackHandle, ModelRuntimeError> {
        Err(ModelRuntimeError::LoraStackError("test-only".to_string()))
    }

    fn steering_hooks(&self, _id: ModelId) -> Result<SteeringHookHandle, ModelRuntimeError> {
        Err(ModelRuntimeError::SteeringHookError(
            "test-only".to_string(),
        ))
    }

    fn cancel(&self, token: CancellationToken) {
        token.cancel();
    }
}

struct CountingCloudBuilder {
    calls: Arc<AtomicUsize>,
}

#[async_trait]
impl CloudRuntimeBuilder for CountingCloudBuilder {
    fn provider(&self) -> ProviderKind {
        ProviderKind::ByokCloud
    }

    async fn build_loaded(
        &self,
        _model_name: &str,
        _invocation_context: Option<handshake_core::model_runtime::cloud::CliInvocationContext>,
        _working_dir: Option<&str>,
    ) -> Result<CloudLiveRuntime, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(CloudLiveRuntime {
            runtime: Arc::new(IdleRuntime),
            model_id: ModelId::new_v7(),
        })
    }
}

fn cloud_request(instance: u32) -> SpawnRequest {
    SpawnRequest::new(
        ModelInstanceId::new(ModelId::new_v7(), instance),
        RuntimeBinding::Candle,
        "pidless-lifecycle-test",
        "pidless-parent",
    )
    .with_cloud_provider(ProviderKind::ByokCloud, "test-cloud-model")
}

fn cloud_config(builder: Arc<dyn CloudRuntimeBuilder>) -> CloudLaneFactoryConfig {
    CloudLaneFactoryConfig {
        anthropic: None,
        openai: Some(builder),
        official_cli: None,
        official_cli_by_provider: Default::default(),
    }
}

#[tokio::test]
async fn pidless_cloud_reserves_complete_lifecycle_before_builder_side_effects() {
    let (ledger, _drain) = LedgerBatcher::manual_for_tests(
        LedgerBatcherConfig {
            capacity: 2,
            batch_size: 1,
            flush_interval: Duration::from_millis(1),
        },
        Arc::new(NoopOverflowSink),
    )
    .expect("two-row manual ledger");
    ledger
        .record_start_lossless(ProcessStart::new(
            ProcessEngineKind::ExternalCompat,
            "preexisting-row",
            None,
        ))
        .expect("occupy one of two queue slots");

    let calls = Arc::new(AtomicUsize::new(0));
    let factory = ProductionModelSessionFactory::new(
        ledger,
        cloud_config(Arc::new(CountingCloudBuilder {
            calls: calls.clone(),
        })),
        None,
    );

    let result = factory.create(&cloud_request(1)).await;
    assert!(
        matches!(result, Err(SwarmError::LedgerFailed(_))),
        "a partially full lifecycle ring must reject before cloud startup"
    );
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "cloud builder must not run when complete START+STOP capacity is unavailable"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pidless_cloud_spawn_acknowledges_durable_start_and_cancel_records_matching_stop() {
    let store = Arc::new(CapturingStore::default());
    let (ledger, writer_join) = LedgerBatcher::spawn(
        store.clone(),
        Arc::new(NoopOverflowSink),
        LedgerBatcherConfig {
            capacity: 8,
            batch_size: 1,
            flush_interval: Duration::from_millis(1),
        },
    );
    let calls = Arc::new(AtomicUsize::new(0));
    let factory = Arc::new(ProductionModelSessionFactory::new(
        ledger.clone(),
        cloud_config(Arc::new(CountingCloudBuilder {
            calls: calls.clone(),
        })),
        None,
    ));
    let coordinator = SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        SwarmConfig::new(RunBudget::defaulted(2).with_concurrency(2)),
        factory,
        Arc::new(RecordingSwarmSink::new()),
        ledger.clone(),
    );
    let request = cloud_request(2);
    let instance_id = request.instance_id;

    coordinator
        .spawn_session(request)
        .await
        .expect("pidless cloud spawn");
    let after_spawn = store.events();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        after_spawn.len(),
        1,
        "spawn success must wait until the authoritative store has the START"
    );
    let start = match &after_spawn[0] {
        LedgerEvent::Start(start) => start.clone(),
        other => panic!("first durable row must be START, got {:?}", other.kind()),
    };
    assert_eq!(start.os_pid, None, "cloud lifecycle must remain pidless");

    coordinator
        .cancel_session(instance_id, "pidless-lifecycle-test-cancel")
        .await
        .expect("cancel pidless cloud session");
    ledger.begin_close();
    tokio::time::timeout(Duration::from_secs(2), writer_join)
        .await
        .expect("ledger writer must drain within the bound")
        .expect("ledger writer task must not panic")
        .expect("ledger writer must flush successfully");

    let rows = store.events();
    assert_eq!(rows.len(), 2, "exactly one START and one STOP are required");
    assert_eq!(rows[0].kind(), LedgerEventKind::Start);
    assert_eq!(rows[1].kind(), LedgerEventKind::Stop);
    assert_eq!(rows[0].process_uuid(), rows[1].process_uuid());
    match &rows[1] {
        LedgerEvent::Stop(stop) => assert_eq!(stop.os_pid, None),
        LedgerEvent::Start(_) => unreachable!(),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pidless_cloud_late_start_after_timeout_records_aborted_stop() {
    let store = Arc::new(DelayedFirstWriteStore::new(Duration::from_secs(6)));
    let (ledger, writer_join) = LedgerBatcher::spawn(
        store.clone(),
        Arc::new(NoopOverflowSink),
        LedgerBatcherConfig {
            capacity: 8,
            batch_size: 1,
            flush_interval: Duration::from_millis(1),
        },
    );
    let calls = Arc::new(AtomicUsize::new(0));
    let factory = ProductionModelSessionFactory::new(
        ledger.clone(),
        cloud_config(Arc::new(CountingCloudBuilder {
            calls: calls.clone(),
        })),
        None,
    );

    let error = match factory.create(&cloud_request(3)).await {
        Ok(_) => panic!("caller must time out before the delayed START commits"),
        Err(error) => error,
    };
    assert!(matches!(error, SwarmError::LedgerFailed(_)));
    assert_eq!(calls.load(Ordering::SeqCst), 1);

    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if store.events().len() == 2 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("late durable START must be paired with an aborted-session STOP");
    ledger.begin_close();
    writer_join
        .await
        .expect("writer task")
        .expect("writer flush");

    let rows = store.events();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].kind(), LedgerEventKind::Start);
    assert_eq!(rows[1].kind(), LedgerEventKind::Stop);
    assert_eq!(rows[0].process_uuid(), rows[1].process_uuid());
    match &rows[1] {
        LedgerEvent::Stop(stop) => assert_eq!(
            stop.stop_reason.as_deref(),
            Some("pidless-session-start-durability-timeout-aborted")
        ),
        LedgerEvent::Start(_) => unreachable!(),
    }
}

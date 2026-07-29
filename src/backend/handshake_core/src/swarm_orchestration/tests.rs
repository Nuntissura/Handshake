//! Proof tests for the swarm coordinator, exercised against a REAL controllable
//! worker adapter (genuine async work + state) — never a result-faking mock.
//!
//! The adapter implements the real [`ModelRuntime`] trait. The factory drives a
//! genuine `tokio` load (an awaitable gate + a real counter of created/loaded
//! sessions) so the orchestration logic under test is exercised end to end:
//! concurrency cap, lifetime ceiling, lease reaper, failure-fingerprint breaker,
//! budget exhaustion, cancel teardown, and no-orphan ledger reconciliation.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use futures::stream;
use tokio::task::JoinSet;

use crate::llm::{CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile};
use crate::model_runtime::error::ModelRuntimeError;
use crate::model_runtime::registry::RuntimeBinding;
use crate::model_runtime::{
    CancellationToken, Embedding, GenerateRequest, KvCacheHandle, KvCachePolicy, KvQuantSupport,
    LoadSpec, LoraStackHandle, ModelCapabilities, ModelId, ModelRuntime, ProviderKind, RuntimeKind,
    SamplingParams, Score, SteeringHookHandle, TokenStream,
};
use crate::process_ledger::{
    LedgerBatcher, LedgerBatcherConfig, LedgerEvent, LedgerOverflowEvent, NoopOverflowSink,
    ProcessEngineKind, ProcessLedgerDrain, ProcessLedgerError, ProcessLedgerOverflowSink,
    ProcessLedgerStore, ProcessOwnershipRecordId, ProcessStart,
};

use super::breaker::BreakerConfig;
use super::coordinator::{SwarmConfig, SwarmCoordinator};
use super::error::SwarmError;
use super::events::{RecordingSwarmSink, SwarmEvent, SwarmFrEventId};
use super::factory::{LiveSession, ModelSessionFactory};
use super::ids::{ModelInstanceId, RunBudget, SpawnRequest};
use super::state::ModelSessionState;

// ---------------------------------------------------------------------------
// Real controllable worker adapter (implements the real ModelRuntime trait).
// ---------------------------------------------------------------------------

/// A genuine model-runtime adapter the tests can drive deterministically. It
/// holds real state (cancel flag, capability + handle values for the trait's
/// reference-returning methods) and produces a real (bounded) token stream that
/// respects cancellation. Nothing here fakes a *result* — it is a controllable
/// worker, exactly the kind of real adapter the task permits for exercising
/// orchestration.
struct ControllableWorker {
    capabilities: ModelCapabilities,
    kv: KvCacheHandle,
    lora: LoraStackHandle,
    steering: SteeringHookHandle,
    cancelled: Arc<AtomicBool>,
    /// Shared counter bumped by the real `unload` so the test can prove the
    /// teardown seam actually freed the model (D1).
    unloaded: Arc<AtomicUsize>,
    /// Shared resource-liveness bit for the owned teardown handle and the
    /// session-serving wrapper. This makes the proof exercise one underlying
    /// loaded resource instead of two unrelated worker instances.
    loaded: Arc<AtomicBool>,
}

struct CountingLlmClient {
    inner: Arc<dyn LlmClient>,
    stream_calls: Arc<AtomicUsize>,
}

struct ProviderErrorLlmClient {
    profile: ModelProfile,
    message: String,
}

#[async_trait]
impl LlmClient for ProviderErrorLlmClient {
    async fn completion(&self, _req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        Err(LlmError::ProviderError(self.message.clone()))
    }

    fn stream_completion(self: Arc<Self>, _req: GenerateRequest) -> TokenStream {
        let message = self.message.clone();
        Box::pin(stream::once(async move {
            Err(ModelRuntimeError::GenerateError(message))
        }))
    }

    fn profile(&self) -> &ModelProfile {
        &self.profile
    }
}

#[async_trait]
impl LlmClient for CountingLlmClient {
    async fn completion(&self, req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        self.inner.completion(req).await
    }

    fn stream_completion(self: Arc<Self>, req: GenerateRequest) -> TokenStream {
        self.stream_calls.fetch_add(1, Ordering::SeqCst);
        Arc::clone(&self.inner).stream_completion(req)
    }

    fn profile(&self) -> &ModelProfile {
        self.inner.profile()
    }
}

impl ControllableWorker {
    fn new(unloaded: Arc<AtomicUsize>, loaded: Arc<AtomicBool>) -> Self {
        Self {
            capabilities: ModelCapabilities::default(),
            kv: KvCacheHandle::new("swarm-test-kv"),
            lora: LoraStackHandle::new("swarm-test-lora"),
            steering: SteeringHookHandle::new("swarm-test-steering"),
            cancelled: Arc::new(AtomicBool::new(false)),
            unloaded,
            loaded,
        }
    }
}

#[async_trait]
impl ModelRuntime for ControllableWorker {
    async fn load(&mut self, _spec: LoadSpec) -> Result<ModelId, ModelRuntimeError> {
        self.loaded.store(true, Ordering::SeqCst);
        Ok(ModelId::new_v7())
    }

    async fn unload(&mut self, _id: ModelId) -> Result<(), ModelRuntimeError> {
        self.cancelled.store(true, Ordering::SeqCst);
        self.loaded.store(false, Ordering::SeqCst);
        self.unloaded.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn generate(&self, req: GenerateRequest) -> TokenStream {
        if !self.loaded.load(Ordering::SeqCst) {
            return Box::pin(stream::iter(vec![Err(ModelRuntimeError::GenerateError(
                "shared model resource is unloaded".to_string(),
            ))]));
        }
        // Real bounded stream that stops early if the request's cancel token
        // fires — genuine generation semantics, not a canned result.
        let cancel = req.cancel.clone();
        let max = req.max_tokens.min(8) as usize;
        let items = (0..max).map(move |i| {
            if cancel.is_cancelled() {
                Err(ModelRuntimeError::Cancelled)
            } else {
                Ok(crate::model_runtime::GeneratedToken {
                    token_id: i as u32,
                    text: format!("t{i}"),
                    logprob: None,
                    finish_reason: None,
                })
            }
        });
        Box::pin(stream::iter(items.collect::<Vec<_>>()))
    }

    async fn score(&self, _id: ModelId, _sequence: Vec<u32>) -> Result<Score, ModelRuntimeError> {
        if !self.loaded.load(Ordering::SeqCst) {
            return Err(ModelRuntimeError::ScoreError(
                "shared model resource is unloaded".to_string(),
            ));
        }
        Ok(Score {
            token_logprobs: vec![],
            mean_logprob: 0.0,
        })
    }

    async fn embed(&self, _id: ModelId, _text: &str) -> Result<Embedding, ModelRuntimeError> {
        if !self.loaded.load(Ordering::SeqCst) {
            return Err(ModelRuntimeError::EmbedError(
                "shared model resource is unloaded".to_string(),
            ));
        }
        Ok(Embedding { vector: vec![] })
    }

    fn capabilities(&self, _id: ModelId) -> Result<&ModelCapabilities, ModelRuntimeError> {
        Ok(&self.capabilities)
    }

    fn kv_cache(&self, _id: ModelId) -> Result<KvCacheHandle, ModelRuntimeError> {
        Ok(self.kv.clone())
    }

    fn lora_stack(&self, _id: ModelId) -> Result<LoraStackHandle, ModelRuntimeError> {
        Ok(self.lora.clone())
    }

    fn steering_hooks(&self, _id: ModelId) -> Result<SteeringHookHandle, ModelRuntimeError> {
        Ok(self.steering.clone())
    }

    fn cancel(&self, token: CancellationToken) {
        token.cancel();
        self.cancelled.store(true, Ordering::SeqCst);
    }
}

// ---------------------------------------------------------------------------
// Controllable factory: drives a REAL async load and records ledger starts.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum FactoryBehavior {
    /// Succeed after a real `load_delay` await.
    Succeed,
    /// Fail with a stable factory error message (same fingerprint every time).
    AlwaysFail,
}

struct ControllableFactory {
    behavior: FactoryBehavior,
    load_delay: Duration,
    ledger: LedgerBatcher,
    /// Observable counters so tests can assert on real factory activity.
    created: Arc<AtomicUsize>,
    /// Total times `create()` was ENTERED (regardless of success/failure). The
    /// D3 admission-gate proof asserts this does NOT increment while the breaker
    /// suppresses admission.
    create_calls: Arc<AtomicUsize>,
    /// Peak number of factory loads in flight at once (the concurrency probe).
    in_flight: Arc<AtomicUsize>,
    peak_in_flight: Arc<AtomicUsize>,
    /// Number of times a session's teardown was actually invoked (D1 proof):
    /// the teardown closure increments this AND calls the worker's `unload`.
    teardown_invocations: Arc<AtomicUsize>,
    fail_teardown_remaining: Arc<AtomicUsize>,
    hold_teardown: Arc<AtomicBool>,
    /// Number of times the worker's real `unload` ran (D1 proof): proves the
    /// teardown frees the model, not just cancels.
    unload_invocations: Arc<AtomicUsize>,
    handed_out:
        Arc<Mutex<std::collections::HashMap<ModelInstanceId, (Arc<dyn ModelRuntime>, ModelId)>>>,
    fail_message: String,
    llm_stream_calls: Arc<AtomicUsize>,
    provider_stream_error: Option<String>,
}

impl ControllableFactory {
    fn new(behavior: FactoryBehavior, load_delay: Duration, ledger: LedgerBatcher) -> Self {
        Self {
            behavior,
            load_delay,
            ledger,
            created: Arc::new(AtomicUsize::new(0)),
            create_calls: Arc::new(AtomicUsize::new(0)),
            in_flight: Arc::new(AtomicUsize::new(0)),
            peak_in_flight: Arc::new(AtomicUsize::new(0)),
            teardown_invocations: Arc::new(AtomicUsize::new(0)),
            fail_teardown_remaining: Arc::new(AtomicUsize::new(0)),
            hold_teardown: Arc::new(AtomicBool::new(false)),
            unload_invocations: Arc::new(AtomicUsize::new(0)),
            handed_out: Arc::new(Mutex::new(std::collections::HashMap::new())),
            fail_message: "controllable factory deterministic failure".to_string(),
            llm_stream_calls: Arc::new(AtomicUsize::new(0)),
            provider_stream_error: None,
        }
    }

    fn with_provider_stream_error(mut self, message: impl Into<String>) -> Self {
        self.provider_stream_error = Some(message.into());
        self
    }
}

#[async_trait]
impl ModelSessionFactory for ControllableFactory {
    async fn create(&self, request: &SpawnRequest) -> Result<LiveSession, SwarmError> {
        // Count every entry into create() (D3 admission-gate proof).
        self.create_calls.fetch_add(1, Ordering::SeqCst);
        // Track real in-flight concurrency at the factory boundary.
        let now = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak_in_flight.fetch_max(now, Ordering::SeqCst);

        // Real async work: yield + sleep so multiple loads genuinely overlap.
        tokio::time::sleep(self.load_delay).await;

        let result = match self.behavior {
            FactoryBehavior::Succeed => {
                // Record a real process-ledger start row, mirroring production.
                let record_id = ProcessOwnershipRecordId::new_v7();
                let os_pid = 40000 + request.instance_id.instance;
                let start = ProcessStart::new(
                    ProcessEngineKind::LlamaCpp,
                    request.owner_role.clone(),
                    request.owner_wp.clone(),
                )
                .with_process_uuid(record_id.as_uuid())
                .with_os_pid(os_pid)
                .with_parent_session_id(request.parent_session_id.clone());
                self.ledger
                    .record_start(start)
                    .map_err(|e| SwarmError::LedgerFailed(e.to_string()))?;

                // Real load: drive the runtime's `load` to obtain a genuine
                // ModelId (no longer discarded — D1) and keep an OWNED worker
                // the teardown closure frees via `unload`, mirroring an owned
                // candle runtime whose Drop/unload releases the engine.
                let loaded = Arc::new(AtomicBool::new(false));
                let mut owned =
                    ControllableWorker::new(self.unload_invocations.clone(), Arc::clone(&loaded));
                let model_id = owned
                    .load(test_load_spec())
                    .await
                    .map_err(|e| SwarmError::FactoryFailed(e.to_string()))?;
                let owned = Arc::new(tokio::sync::Mutex::new(owned));
                let shared: Arc<dyn ModelRuntime> = Arc::new(ControllableWorker::new(
                    self.unload_invocations.clone(),
                    loaded,
                ));
                self.handed_out
                    .lock()
                    .unwrap()
                    .insert(request.instance_id, (shared.clone(), model_id));
                let cancel = CancellationToken::new();
                self.created.fetch_add(1, Ordering::SeqCst);

                let teardown_invocations = self.teardown_invocations.clone();
                let fail_teardown_remaining = self.fail_teardown_remaining.clone();
                let hold_teardown = self.hold_teardown.clone();
                let teardown: super::factory::SessionTeardown = Arc::new(move || {
                    let teardown_invocations = teardown_invocations.clone();
                    let fail_teardown_remaining = fail_teardown_remaining.clone();
                    let hold_teardown = hold_teardown.clone();
                    let owned = Arc::clone(&owned);
                    Box::pin(async move {
                        teardown_invocations.fetch_add(1, Ordering::SeqCst);
                        while hold_teardown.load(Ordering::SeqCst) {
                            tokio::time::sleep(Duration::from_millis(5)).await;
                        }
                        if fail_teardown_remaining
                            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |value| {
                                (value > 0).then(|| value - 1)
                            })
                            .is_ok()
                        {
                            return Err(SwarmError::Internal(
                                "injected retryable teardown failure".into(),
                            ));
                        }
                        // Free the model on the owned runtime — the real
                        // teardown contract (D1). This bumps `unloaded`.
                        owned
                            .lock()
                            .await
                            .unload(model_id)
                            .await
                            .map_err(|e| SwarmError::Internal(e.to_string()))
                    })
                });

                let live = LiveSession::new(shared, model_id, cancel, teardown, record_id, os_pid);
                let inner_client: Arc<dyn LlmClient> = match self.provider_stream_error.as_ref() {
                    Some(message) => Arc::new(ProviderErrorLlmClient {
                        profile: ModelProfile::new(model_id.to_string(), 4_096),
                        message: message.clone(),
                    }),
                    None => Arc::clone(&live.llm_client),
                };
                let counted_client: Arc<dyn LlmClient> = Arc::new(CountingLlmClient {
                    inner: inner_client,
                    stream_calls: Arc::clone(&self.llm_stream_calls),
                });
                Ok(live.with_llm_client(counted_client))
            }
            FactoryBehavior::AlwaysFail => {
                Err(SwarmError::FactoryFailed(self.fail_message.clone()))
            }
        };

        self.in_flight.fetch_sub(1, Ordering::SeqCst);
        result
    }
}

#[tokio::test]
async fn application_generation_crosses_llm_client_once_and_preserves_streaming() {
    use futures::StreamExt;

    let (ledger, _drain) = ledger_pair();
    let factory = Arc::new(ControllableFactory::new(
        FactoryBehavior::Succeed,
        Duration::from_millis(1),
        ledger.clone(),
    ));
    let coordinator = SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        SwarmConfig::new(
            RunBudget::defaulted(1)
                .with_concurrency(1)
                .with_lifetime_spawns(1),
        ),
        factory.clone(),
        Arc::new(RecordingSwarmSink::new()),
        ledger,
    );
    let instance_id = instance(400);
    coordinator
        .spawn_session(spawn_req(instance_id))
        .await
        .expect("spawn mediated session");
    let model_id = coordinator
        .session_model_id(instance_id)
        .expect("session model id");
    let request_cancel = CancellationToken::new();
    let mut stream = coordinator
        .generate_session(
            instance_id,
            GenerateRequest {
                id: ModelId::new_v7(),
                prompt: "mediated".into(),
                sampling: SamplingParams::default(),
                lora_overrides: Vec::new(),
                steering_overrides: Vec::new(),
                kv_prefix_handle: None,
                cancel: request_cancel,
                max_tokens: 3,
                stop_sequences: Vec::new(),
                speculative_mode: None,
                structured_decoding: None,
            },
        )
        .expect("start mediated generation");
    let tokens = stream
        .by_ref()
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .expect("stream tokens");

    assert_eq!(factory.llm_stream_calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        tokens.len(),
        3,
        "runtime token streaming remains unbuffered"
    );
    assert_eq!(
        coordinator.session_model_id(instance_id),
        Some(model_id),
        "coordinator-owned runtime model identity remains authoritative"
    );
}

fn managed_generate_request(max_tokens: u32) -> GenerateRequest {
    GenerateRequest {
        id: ModelId::new_v7(),
        prompt: "managed generation proof".into(),
        sampling: SamplingParams::default(),
        lora_overrides: Vec::new(),
        steering_overrides: Vec::new(),
        kv_prefix_handle: None,
        cancel: CancellationToken::new(),
        max_tokens,
        stop_sequences: Vec::new(),
        speculative_mode: None,
        structured_decoding: None,
    }
}

#[tokio::test]
async fn managed_generation_eof_records_one_correlated_invocation_and_usage() {
    use futures::StreamExt;

    let (ledger, _drain) = ledger_pair();
    let factory = Arc::new(ControllableFactory::new(
        FactoryBehavior::Succeed,
        Duration::from_millis(1),
        ledger.clone(),
    ));
    let sink = Arc::new(RecordingSwarmSink::new());
    let coordinator = SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        SwarmConfig::new(
            RunBudget::defaulted(1)
                .with_concurrency(1)
                .with_lifetime_spawns(2)
                .with_token_ceiling(10),
        ),
        factory,
        sink.clone(),
        ledger,
    );
    let instance_id = instance(401);
    coordinator
        .spawn_session(spawn_req(instance_id))
        .await
        .expect("spawn managed session");

    let tokens = coordinator
        .generate_session_managed(instance_id, managed_generate_request(3))
        .expect("start managed generation")
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .expect("managed stream completes");
    assert_eq!(tokens.len(), 3);
    assert_eq!(
        coordinator.session_state(instance_id),
        Some(ModelSessionState::Ready)
    );
    assert_eq!(coordinator.remaining().tokens_remaining, Some(7));

    let invocation_events = sink
        .events()
        .into_iter()
        .filter(|event| {
            matches!(
                event,
                SwarmEvent::ModelInvocationStarted { instance_id: id, .. }
                    | SwarmEvent::ModelInvocationFinished { instance_id: id, .. }
                    if *id == instance_id
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        invocation_events.len(),
        2,
        "one START plus one terminal receipt"
    );
    match (&invocation_events[0], &invocation_events[1]) {
        (
            SwarmEvent::ModelInvocationStarted {
                trace_id: start_trace,
                run_id: start_run,
                session_id: start_session,
                ..
            },
            SwarmEvent::ModelInvocationFinished {
                trace_id: finish_trace,
                run_id: finish_run,
                session_id: finish_session,
                outcome,
                generated_tokens,
                error,
                ..
            },
        ) => {
            assert_eq!(start_trace, finish_trace);
            assert_eq!(start_run, finish_run);
            assert_eq!(start_session, finish_session);
            assert_eq!(outcome, "completed");
            assert_eq!(*generated_tokens, 3);
            assert_eq!(error, &None);
        }
        events => panic!("unexpected invocation event order: {events:?}"),
    }
}

#[tokio::test]
async fn managed_generation_provider_error_records_failed_terminal_without_ready_resurrection() {
    use futures::StreamExt;

    let (ledger, _drain) = ledger_pair();
    let factory = Arc::new(
        ControllableFactory::new(
            FactoryBehavior::Succeed,
            Duration::from_millis(1),
            ledger.clone(),
        )
        .with_provider_stream_error("injected provider failure"),
    );
    let sink = Arc::new(RecordingSwarmSink::new());
    let coordinator = SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        SwarmConfig::new(
            RunBudget::defaulted(1)
                .with_concurrency(1)
                .with_lifetime_spawns(2)
                .with_token_ceiling(10),
        ),
        factory,
        sink.clone(),
        ledger,
    );
    let instance_id = instance(402);
    coordinator
        .spawn_session(spawn_req(instance_id))
        .await
        .expect("spawn failing managed session");

    let mut stream = coordinator
        .generate_session_managed(instance_id, managed_generate_request(3))
        .expect("start failing managed generation");
    let error = stream
        .next()
        .await
        .expect("provider returns one terminal error")
        .expect_err("provider error must remain an error");
    assert!(error.to_string().contains("injected provider failure"));
    assert_eq!(coordinator.session_state(instance_id), None);
    assert_eq!(coordinator.remaining().tokens_remaining, Some(10));

    let terminal = sink.events().into_iter().filter(|event| {
        matches!(
            event,
            SwarmEvent::ModelInvocationFinished {
                instance_id: id,
                outcome,
                generated_tokens: 0,
                error: Some(_),
                ..
            } if *id == instance_id && outcome == "failed"
        )
    });
    assert_eq!(terminal.count(), 1);
    assert!(!sink.events().iter().any(|event| matches!(
        event,
        SwarmEvent::SessionStateChanged {
            instance_id: id,
            from: ModelSessionState::Generating,
            to: ModelSessionState::Ready,
        } if *id == instance_id
    )));
}

#[tokio::test]
async fn managed_generation_drop_records_terminal_and_cancels_session() {
    let (ledger, _drain) = ledger_pair();
    let factory = Arc::new(ControllableFactory::new(
        FactoryBehavior::Succeed,
        Duration::from_millis(1),
        ledger.clone(),
    ));
    let sink = Arc::new(RecordingSwarmSink::new());
    let coordinator = SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        SwarmConfig::new(
            RunBudget::defaulted(1)
                .with_concurrency(1)
                .with_lifetime_spawns(2)
                .with_token_ceiling(10),
        ),
        factory,
        sink.clone(),
        ledger,
    );
    let instance_id = instance(403);
    coordinator
        .spawn_session(spawn_req(instance_id))
        .await
        .expect("spawn dropped managed session");

    let stream = coordinator
        .generate_session_managed(instance_id, managed_generate_request(3))
        .expect("start dropped managed generation");
    drop(stream);
    tokio::time::timeout(Duration::from_secs(2), async {
        while coordinator.session_state(instance_id).is_some() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("drop cleanup reaches terminal state");

    assert_eq!(coordinator.remaining().tokens_remaining, Some(10));
    assert_eq!(
        sink.events()
            .iter()
            .filter(|event| matches!(
                event,
                SwarmEvent::ModelInvocationFinished {
                    instance_id: id,
                    outcome,
                    generated_tokens: 0,
                    ..
                } if *id == instance_id && outcome == "dropped"
            ))
            .count(),
        1
    );
    assert!(!sink.events().iter().any(|event| matches!(
        event,
        SwarmEvent::SessionStateChanged {
            instance_id: id,
            from: ModelSessionState::Generating,
            to: ModelSessionState::Ready,
        } if *id == instance_id
    )));
}

#[tokio::test]
async fn managed_generation_drop_without_tokio_handle_fences_and_durably_cleans_session() {
    let (ledger, _drain) = ledger_pair();
    let factory = Arc::new(ControllableFactory::new(
        FactoryBehavior::Succeed,
        Duration::from_millis(1),
        ledger.clone(),
    ));
    let sink = Arc::new(RecordingSwarmSink::new());
    let coordinator = SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        SwarmConfig::new(
            RunBudget::defaulted(1)
                .with_concurrency(1)
                .with_lifetime_spawns(2)
                .with_token_ceiling(10),
        ),
        factory,
        sink.clone(),
        ledger,
    );
    let instance_id = instance(404);
    coordinator
        .spawn_session(spawn_req(instance_id))
        .await
        .expect("spawn no-Tokio-drop managed session");

    let stream = coordinator
        .generate_session_managed(instance_id, managed_generate_request(3))
        .expect("start no-Tokio-drop managed generation");
    std::thread::spawn(move || {
        assert!(
            tokio::runtime::Handle::try_current().is_err(),
            "negative path must drop outside any Tokio runtime"
        );
        drop(stream);
    })
    .join()
    .expect("plain drop thread");

    assert!(
        !matches!(
            coordinator.session_state(instance_id),
            Some(ModelSessionState::Ready | ModelSessionState::Generating)
        ),
        "drop must synchronously fence the registry before background cleanup"
    );
    assert!(sink.events().iter().any(|event| matches!(
        event,
        SwarmEvent::SessionStateChanged {
            instance_id: id,
            from: ModelSessionState::Generating,
            to: ModelSessionState::Cancelling,
        } if *id == instance_id
    )));

    tokio::time::timeout(Duration::from_secs(2), async {
        while coordinator.session_state(instance_id).is_some() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("no-Tokio drop cleanup reaches durable terminal eviction");
    assert_eq!(coordinator.remaining().tokens_remaining, Some(10));
    assert_eq!(
        sink.events()
            .iter()
            .filter(|event| matches!(
                event,
                SwarmEvent::ModelInvocationFinished {
                    instance_id: id,
                    outcome,
                    generated_tokens: 0,
                    ..
                } if *id == instance_id && outcome == "dropped"
            ))
            .count(),
        1
    );
}

#[test]
fn coordinator_source_forbids_direct_application_runtime_generate() {
    let source = include_str!("coordinator.rs");
    let distillation_source = include_str!("../distillation/parallel_distill.rs");
    assert!(source.contains("llm_client.stream_completion(request)"));
    assert!(
        !source.contains("runtime.generate(request)"),
        "SwarmCoordinator application generation must not bypass LlmClient"
    );
    // Parallel distillation drives application generation through the
    // coordinator's LlmClient-mediated `generate_session_managed`, which
    // internally dispatches `llm_client.stream_completion_with_context`. It must
    // never reach a raw `ModelRuntime::generate` on the application path.
    assert!(distillation_source.contains("generate_session_managed("));
    assert!(
        !distillation_source.contains("runtime.generate(req)"),
        "parallel distillation application generation must not bypass LlmClient"
    );
}

struct ReadyHookFactory {
    inner: ControllableFactory,
    ready_commits: Arc<AtomicUsize>,
}

struct BlockingReadyHookFactory {
    inner: ControllableFactory,
    hook_entered: Arc<AtomicBool>,
    release_hook: Arc<AtomicBool>,
    ready_commits: Arc<AtomicUsize>,
}

#[async_trait]
impl ModelSessionFactory for BlockingReadyHookFactory {
    async fn create(&self, request: &SpawnRequest) -> Result<LiveSession, SwarmError> {
        let live = self.inner.create(request).await?;
        let hook_entered = Arc::clone(&self.hook_entered);
        let release_hook = Arc::clone(&self.release_hook);
        let ready_commits = Arc::clone(&self.ready_commits);
        Ok(live.with_ready_hook(Arc::new(move || {
            hook_entered.store(true, Ordering::SeqCst);
            while !release_hook.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(1));
            }
            ready_commits.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })))
    }
}

#[async_trait]
impl ModelSessionFactory for ReadyHookFactory {
    async fn create(&self, request: &SpawnRequest) -> Result<LiveSession, SwarmError> {
        let live = self.inner.create(request).await?;
        let ready_commits = Arc::clone(&self.ready_commits);
        Ok(live.with_ready_hook(Arc::new(move || {
            ready_commits.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })))
    }
}

// ---------------------------------------------------------------------------
// In-memory process ledger store (real drain of real rows).
// ---------------------------------------------------------------------------

#[derive(Clone, Default)]
struct InMemoryStore {
    events: Arc<Mutex<Vec<LedgerEvent>>>,
}

struct FailingOverflowSink;

impl ProcessLedgerOverflowSink for FailingOverflowSink {
    fn emit_overflow(&self, _event: LedgerOverflowEvent) -> Result<(), ProcessLedgerError> {
        Err(ProcessLedgerError::OverflowEmit(
            "injected STOP overflow failure".into(),
        ))
    }
}

#[async_trait]
impl ProcessLedgerStore for InMemoryStore {
    async fn write_batch(&self, events: Vec<LedgerEvent>) -> Result<(), ProcessLedgerError> {
        self.events.lock().unwrap().extend(events);
        Ok(())
    }
}

fn ledger_pair() -> (LedgerBatcher, ProcessLedgerDrain) {
    LedgerBatcher::manual_for_tests(
        LedgerBatcherConfig {
            capacity: 4096,
            ..LedgerBatcherConfig::default()
        },
        Arc::new(NoopOverflowSink),
    )
    .expect("manual ledger")
}

async fn drain(drain: &ProcessLedgerDrain, store: Arc<InMemoryStore>) -> Vec<LedgerEvent> {
    drain.drain_available_to(store.clone()).await.unwrap();
    store.events.lock().unwrap().clone()
}

fn instance(i: u32) -> ModelInstanceId {
    ModelInstanceId::new(ModelId::new_v7(), i)
}

fn spawn_req(iid: ModelInstanceId) -> SpawnRequest {
    SpawnRequest::new(
        iid,
        RuntimeBinding::LlamaCpp,
        "swarm_test",
        "parent-session-1",
    )
}

/// Minimal LoadSpec for the controllable worker (which ignores the spec body —
/// it exists only so the real `load` seam runs and returns a genuine ModelId).
fn test_load_spec() -> LoadSpec {
    LoadSpec {
        artifact_path: std::path::PathBuf::from("swarm-test-artifact"),
        sha256_expected: "swarm-test-sha".to_string(),
        runtime_kind: RuntimeKind::LlamaCpp,
        sampling_defaults: SamplingParams::default(),
        kv_cache_policy: KvCachePolicy::Default {
            quant: KvQuantSupport::Q4,
            prefix_cache_ttl_seconds: 0,
            max_bytes: None,
        },
        declared_capabilities: ModelCapabilities::default(),
        provider: ProviderKind::Local,
        engine_origin: None,
        external_engine_import: None,
    }
}

// ===========================================================================
// RANK-2: board/lineage grouping (swarm_id / worktree_id) flows request -> handle.
// ===========================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rank2_spawned_session_carries_swarm_and_worktree_grouping() {
    let (ledger, _drain) = ledger_pair();
    let factory = Arc::new(ControllableFactory::new(
        FactoryBehavior::Succeed,
        Duration::from_millis(1),
        ledger.clone(),
    ));
    let sink = Arc::new(RecordingSwarmSink::new());
    let budget = RunBudget::defaulted(8)
        .with_concurrency(8)
        .with_lifetime_spawns(8);
    let coordinator = SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        SwarmConfig::new(budget),
        factory,
        sink.clone(),
        ledger,
    );

    // A grouped spawn: the swarm + worktree grouping copied from the SpawnRequest
    // is readable per session (board swimlane / Flight-Recorder drill-down join).
    let iid = instance(1);
    let req = spawn_req(iid)
        .with_swarm("swarm-alpha")
        .with_worktree("wt-7");
    coordinator.spawn_session(req).await.unwrap();
    assert_eq!(
        coordinator.session_grouping(iid),
        Some((Some("swarm-alpha".to_string()), Some("wt-7".to_string())))
    );
    let spawned = sink
        .events()
        .into_iter()
        .find_map(|event| match event {
            SwarmEvent::SessionSpawned {
                instance_id,
                swarm_id,
                worktree_id,
                ..
            } if instance_id == iid => Some((swarm_id, worktree_id)),
            _ => None,
        })
        .expect("SessionSpawned event recorded");
    assert_eq!(
        spawned,
        (Some("swarm-alpha".to_string()), Some("wt-7".to_string())),
        "durable swarm events carry the same grouping as the live registry"
    );

    // An ungrouped spawn carries (None, None) — source-compatible default.
    let iid2 = instance(2);
    coordinator.spawn_session(spawn_req(iid2)).await.unwrap();
    assert_eq!(coordinator.session_grouping(iid2), Some((None, None)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn checkout_lease_blocks_factory_until_terminal_teardown_and_stop_release() {
    let checkout = std::env::temp_dir().join(format!(
        "handshake-coordinator-checkout-{}",
        uuid::Uuid::now_v7()
    ));
    std::fs::create_dir_all(checkout.join(".git")).expect("create checkout marker");

    let (ledger, ledger_drain) = ledger_pair();
    let factory = Arc::new(ControllableFactory::new(
        FactoryBehavior::Succeed,
        Duration::from_millis(1),
        ledger.clone(),
    ));
    let sink = Arc::new(RecordingSwarmSink::new());
    let coordinator = SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        SwarmConfig::new(
            RunBudget::defaulted(8)
                .with_concurrency(8)
                .with_lifetime_spawns(8),
        ),
        factory.clone(),
        sink,
        ledger,
    );

    let first = instance(201);
    coordinator
        .spawn_session(
            spawn_req(first)
                .with_worktree("wt-checkout-owner")
                .with_working_dir(checkout.display().to_string()),
        )
        .await
        .expect("first checkout owner starts");

    let second = instance(202);
    let conflict = coordinator
        .spawn_session(
            spawn_req(second)
                .with_worktree("wt-checkout-spoof")
                .with_working_dir(checkout.display().to_string()),
        )
        .await
        .expect_err("same canonical checkout must be exclusive");
    assert!(matches!(
        conflict,
        SwarmError::CheckoutLeaseConflict { key_kind, .. } if key_kind == "canonical_path"
    ));
    assert_eq!(
        factory.create_calls.load(Ordering::SeqCst),
        1,
        "lease conflict must reject before factory.create"
    );

    coordinator
        .cancel_session(first, "checkout-lease-release-proof")
        .await
        .expect("terminal teardown and STOP release first checkout lease");
    coordinator
        .spawn_session(
            spawn_req(second)
                .with_worktree("wt-checkout-spoof")
                .with_working_dir(checkout.display().to_string()),
        )
        .await
        .expect("checkout reacquires after terminal cleanup");
    assert_eq!(factory.create_calls.load(Ordering::SeqCst), 2);
    coordinator
        .cancel_session(second, "checkout-lease-final-cleanup")
        .await
        .expect("clean up second owner");

    let store = Arc::new(InMemoryStore::default());
    let events = drain(&ledger_drain, store).await;
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, LedgerEvent::Start(_)))
            .count(),
        2
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, LedgerEvent::Stop(_)))
            .count(),
        2
    );
    std::fs::remove_dir_all(checkout).expect("remove checkout fixture");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn failed_orphan_cleanup_retains_checkout_lease_until_retry_teardown_and_stop() {
    let checkout = std::env::temp_dir().join(format!(
        "handshake-orphan-checkout-{}",
        uuid::Uuid::now_v7()
    ));
    std::fs::create_dir_all(checkout.join(".git")).expect("create orphan checkout marker");

    let (ledger, ledger_drain) = ledger_pair();
    let factory = Arc::new(ControllableFactory::new(
        FactoryBehavior::Succeed,
        Duration::from_millis(200),
        ledger.clone(),
    ));
    factory.fail_teardown_remaining.store(1, Ordering::SeqCst);
    let coordinator = Arc::new(SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        SwarmConfig::new(
            RunBudget::defaulted(8)
                .with_concurrency(8)
                .with_lifetime_spawns(8)
                .with_committed_memory_ceiling(1024),
        ),
        factory.clone(),
        Arc::new(RecordingSwarmSink::new()),
        ledger,
    ));

    let pending_id = instance(204);
    let pending_request = spawn_req(pending_id)
        .with_worktree("wt-orphan-owner")
        .with_committed_memory_bytes(512)
        .with_working_dir(checkout.display().to_string());
    let spawn = {
        let coordinator = Arc::clone(&coordinator);
        tokio::spawn(async move { coordinator.spawn_session(pending_request).await })
    };
    for _ in 0..100 {
        if factory.in_flight.load(Ordering::SeqCst) > 0 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert_eq!(
        factory.in_flight.load(Ordering::SeqCst),
        1,
        "factory create must be in flight before pending-spawn cancellation"
    );
    coordinator
        .cancel_session(
            pending_id,
            "cancel pending spawn for orphan retention proof",
        )
        .await
        .expect("pending spawn cancellation token is delivered");
    let spawn_error = spawn
        .await
        .expect("pending spawn task joins")
        .expect_err("injected first orphan teardown attempt must fail");
    assert!(
        spawn_error
            .to_string()
            .contains("injected retryable teardown failure"),
        "spawn must surface the retained orphan cleanup failure: {spawn_error}"
    );
    let retained_capacity = coordinator.remaining();
    assert_eq!(retained_capacity.concurrency_permits_available, 7);
    assert_eq!(
        retained_capacity.committed_memory_bytes_remaining,
        Some(512),
        "cancelled post-factory orphan must retain its permit and memory charge"
    );

    let contender_id = instance(205);
    let contender_request = spawn_req(contender_id)
        .with_worktree("wt-orphan-contender")
        .with_committed_memory_bytes(0)
        .with_working_dir(checkout.display().to_string());
    let conflict = coordinator
        .spawn_session(contender_request.clone())
        .await
        .expect_err("failed orphan cleanup must keep the canonical checkout locked");
    assert!(matches!(
        conflict,
        SwarmError::CheckoutLeaseConflict { key_kind, .. } if key_kind == "canonical_path"
    ));
    assert_eq!(
        factory.create_calls.load(Ordering::SeqCst),
        1,
        "retained orphan checkout lock rejects before a second factory create"
    );

    coordinator
        .retry_pending_orphan_cleanups()
        .await
        .expect("retry completes teardown and matching STOP before lease release");
    assert_eq!(factory.unload_invocations.load(Ordering::SeqCst), 1);
    let released_capacity = coordinator.remaining();
    assert_eq!(released_capacity.concurrency_permits_available, 8);
    assert_eq!(
        released_capacity.committed_memory_bytes_remaining,
        Some(1024)
    );
    coordinator
        .spawn_session(contender_request)
        .await
        .expect("checkout becomes available only after orphan teardown and STOP succeed");
    coordinator
        .cancel_session(contender_id, "orphan retention proof cleanup")
        .await
        .expect("clean up checkout contender");

    let store = Arc::new(InMemoryStore::default());
    let events = drain(&ledger_drain, store).await;
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, LedgerEvent::Start(_)))
            .count(),
        2
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, LedgerEvent::Stop(_)))
            .count(),
        2
    );
    std::fs::remove_dir_all(checkout).expect("remove orphan checkout fixture");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn duplicate_spawn_compensation_never_commits_ready_hook_for_loser() {
    let (ledger, ledger_drain) = ledger_pair();
    let ready_commits = Arc::new(AtomicUsize::new(0));
    let factory = Arc::new(ReadyHookFactory {
        inner: ControllableFactory::new(
            FactoryBehavior::Succeed,
            Duration::from_millis(1),
            ledger.clone(),
        ),
        ready_commits: Arc::clone(&ready_commits),
    });
    let create_calls = Arc::clone(&factory.inner.create_calls);
    let teardown_invocations = Arc::clone(&factory.inner.teardown_invocations);
    let unload_invocations = Arc::clone(&factory.inner.unload_invocations);
    let coordinator = SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        SwarmConfig::new(
            RunBudget::defaulted(8)
                .with_concurrency(8)
                .with_lifetime_spawns(8),
        ),
        factory.clone(),
        Arc::new(RecordingSwarmSink::new()),
        ledger,
    );

    let instance_id = instance(203);
    coordinator
        .spawn_session(spawn_req(instance_id))
        .await
        .expect("first spawn commits ready hook");
    assert_eq!(ready_commits.load(Ordering::SeqCst), 1);

    // Create a genuine second live session and inject it at the exact
    // post-factory/pre-registry boundary. A normal same-id call is rejected by
    // the earlier pending/live admission check and cannot prove this rollback.
    let duplicate_request = spawn_req(instance_id);
    let loser = factory
        .create(&duplicate_request)
        .await
        .expect("second factory create crosses the external side-effect boundary");
    factory.inner.hold_teardown.store(true, Ordering::SeqCst);
    let mut duplicate_future = Box::pin(
        coordinator.duplicate_insert_after_factory_create_for_test(duplicate_request, loser),
    );
    tokio::select! {
        result = &mut duplicate_future => panic!("duplicate rollback returned before blocked teardown: {result:?}"),
        _ = tokio::time::sleep(Duration::from_millis(30)) => {}
    }
    assert_eq!(teardown_invocations.load(Ordering::SeqCst), 1);
    assert_eq!(
        coordinator.remaining().concurrency_permits_available,
        6,
        "winner and duplicate loser must both retain permits until loser teardown completes"
    );
    factory.inner.hold_teardown.store(false, Ordering::SeqCst);
    let duplicate = duplicate_future
        .await
        .expect_err("factory-created loser is rejected by atomic registry insertion");
    assert!(matches!(duplicate, SwarmError::DuplicateInstance(id) if id == instance_id));
    assert_eq!(create_calls.load(Ordering::SeqCst), 2);
    assert_eq!(
        ready_commits.load(Ordering::SeqCst),
        1,
        "duplicate loser must be compensated without publishing its ready hook"
    );
    assert_eq!(teardown_invocations.load(Ordering::SeqCst), 1);
    assert_eq!(unload_invocations.load(Ordering::SeqCst), 1);

    coordinator
        .cancel_session(instance_id, "ready-hook-test-cleanup")
        .await
        .expect("clean up winning session");
    assert_eq!(teardown_invocations.load(Ordering::SeqCst), 2);
    assert_eq!(unload_invocations.load(Ordering::SeqCst), 2);
    let store = Arc::new(InMemoryStore::default());
    let events = drain(&ledger_drain, store).await;
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, LedgerEvent::Start(_)))
            .count(),
        2
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, LedgerEvent::Stop(_)))
            .count(),
        2
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn failed_duplicate_cleanup_retains_permit_and_memory_until_retry_succeeds() {
    let (ledger, _ledger_drain) = ledger_pair();
    let factory = Arc::new(ControllableFactory::new(
        FactoryBehavior::Succeed,
        Duration::from_millis(1),
        ledger.clone(),
    ));
    let coordinator = SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        SwarmConfig::new(
            RunBudget::defaulted(8)
                .with_concurrency(2)
                .with_lifetime_spawns(8)
                .with_committed_memory_ceiling(512),
        ),
        factory.clone(),
        Arc::new(RecordingSwarmSink::new()),
        ledger,
    );

    let instance_id = instance(206);
    coordinator
        .spawn_session(spawn_req(instance_id).with_committed_memory_bytes(0))
        .await
        .expect("winner occupies one concurrency permit");

    let duplicate_request = spawn_req(instance_id).with_committed_memory_bytes(512);
    let loser = factory
        .create(&duplicate_request)
        .await
        .expect("duplicate loser crosses the factory side-effect boundary");
    factory.fail_teardown_remaining.store(1, Ordering::SeqCst);
    let cleanup_error = coordinator
        .duplicate_insert_after_factory_create_for_test(duplicate_request, loser)
        .await
        .expect_err("first duplicate teardown attempt is injected to fail");
    assert!(
        cleanup_error
            .to_string()
            .contains("injected retryable teardown failure"),
        "duplicate rollback must expose the retryable cleanup error: {cleanup_error}"
    );

    let pending = coordinator.remaining();
    assert_eq!(
        pending.concurrency_permits_available, 0,
        "winner and pending duplicate cleanup must retain both permits"
    );
    assert_eq!(
        pending.committed_memory_bytes_remaining,
        Some(0),
        "pending duplicate cleanup must retain its committed-memory reservation"
    );

    coordinator
        .retry_pending_orphan_cleanups()
        .await
        .expect("retry completes duplicate teardown and matching STOP");
    let released = coordinator.remaining();
    assert_eq!(released.concurrency_permits_available, 1);
    assert_eq!(released.committed_memory_bytes_remaining, Some(512));

    coordinator
        .cancel_session(instance_id, "duplicate capacity retention test cleanup")
        .await
        .expect("clean up winning session");
    assert_eq!(coordinator.remaining().concurrency_permits_available, 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn successful_insert_transfers_capacity_before_spawn_future_cancellation() {
    let (ledger, _ledger_drain) = ledger_pair();
    let factory = Arc::new(ControllableFactory::new(
        FactoryBehavior::Succeed,
        Duration::from_millis(1),
        ledger.clone(),
    ));
    let coordinator = Arc::new(SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        SwarmConfig::new(
            RunBudget::defaulted(1)
                .with_concurrency(1)
                .with_lifetime_spawns(1)
                .with_committed_memory_ceiling(512),
        ),
        factory.clone(),
        Arc::new(RecordingSwarmSink::new()),
        ledger,
    ));

    let instance_id = instance(207);
    let request = spawn_req(instance_id).with_committed_memory_bytes(512);
    let live = factory
        .create(&request)
        .await
        .expect("factory creates the session for the insertion handoff seam");
    let insertion = {
        let coordinator = Arc::clone(&coordinator);
        tokio::spawn(async move {
            coordinator
                .successful_insert_ownership_handoff_for_test(request, live)
                .await
        })
    };
    for _ in 0..100 {
        if coordinator.session_state(instance_id).is_some() {
            break;
        }
        tokio::task::yield_now().await;
    }
    assert!(
        coordinator.session_state(instance_id).is_some(),
        "test seam must reach successful registry insertion before cancellation"
    );
    let inserted = coordinator.remaining();
    assert_eq!(inserted.concurrency_permits_available, 0);
    assert_eq!(inserted.committed_memory_bytes_remaining, Some(0));

    insertion.abort();
    let _ = insertion.await;
    let after_abort = coordinator.remaining();
    assert_eq!(
        after_abort.concurrency_permits_available, 0,
        "registry handle remains the sole permit owner after spawn future abort"
    );
    assert_eq!(
        after_abort.committed_memory_bytes_remaining,
        Some(0),
        "disarmed outer guard cannot release registry-owned memory on abort"
    );

    coordinator
        .cancel_session(
            instance_id,
            "successful insertion ownership handoff cleanup",
        )
        .await
        .expect("terminal cleanup releases registry-owned capacity");
    let released = coordinator.remaining();
    assert_eq!(released.concurrency_permits_available, 1);
    assert_eq!(released.committed_memory_bytes_remaining, Some(512));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn terminal_cleanup_waits_for_in_progress_ready_hook_publication_fence() {
    let (ledger, _drain) = ledger_pair();
    let hook_entered = Arc::new(AtomicBool::new(false));
    let release_hook = Arc::new(AtomicBool::new(false));
    let ready_commits = Arc::new(AtomicUsize::new(0));
    let factory = Arc::new(BlockingReadyHookFactory {
        inner: ControllableFactory::new(
            FactoryBehavior::Succeed,
            Duration::from_millis(1),
            ledger.clone(),
        ),
        hook_entered: Arc::clone(&hook_entered),
        release_hook: Arc::clone(&release_hook),
        ready_commits: Arc::clone(&ready_commits),
    });
    let coordinator = Arc::new(SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        SwarmConfig::new(
            RunBudget::defaulted(8)
                .with_concurrency(8)
                .with_lifetime_spawns(8),
        ),
        factory,
        Arc::new(RecordingSwarmSink::new()),
        ledger,
    ));
    let instance_id = instance(206);
    let spawn = {
        let coordinator = Arc::clone(&coordinator);
        let runtime = tokio::runtime::Handle::current();
        tokio::task::spawn_blocking(move || {
            runtime.block_on(coordinator.spawn_session(spawn_req(instance_id)))
        })
    };
    for _ in 0..100 {
        if hook_entered.load(Ordering::SeqCst) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    let hook_was_entered = hook_entered.load(Ordering::SeqCst);
    if !hook_was_entered {
        release_hook.store(true, Ordering::SeqCst);
        let _ = spawn.await;
        panic!("ready hook must enter before the concurrent terminal probe");
    }
    let complete = {
        let coordinator = Arc::clone(&coordinator);
        let runtime = tokio::runtime::Handle::current();
        tokio::task::spawn_blocking(move || {
            runtime.block_on(coordinator.complete_session(instance_id))
        })
    };
    tokio::time::sleep(Duration::from_millis(25)).await;
    let terminal_waited_for_publication = !complete.is_finished();
    release_hook.store(true, Ordering::SeqCst);
    spawn
        .await
        .expect("spawn task joins")
        .expect("Ready publication commits before terminal cleanup acquires the fence");
    complete
        .await
        .expect("completion task joins")
        .expect("terminal cleanup succeeds after Ready publication releases the fence");
    assert!(
        terminal_waited_for_publication,
        "terminal cleanup must wait on the same registry fence as Ready publication"
    );
    assert_eq!(ready_commits.load(Ordering::SeqCst), 1);
    assert_eq!(
        coordinator.session_state(instance_id),
        None,
        "terminal cleanup removes the published session exactly once"
    );
}

/// The read-only `live_instances_in_swarm` accessor enumerates EXACTLY the live
/// instances registered under a swarm — including manually-spawned ones — and
/// excludes other swarms, ungrouped sessions, terminal sessions, blank queries,
/// and unknown swarms. This is the authoritative source the calendar teardown
/// uses to cancel ALL sessions in a swarm.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn live_instances_in_swarm_enumerates_all_live_and_excludes_terminal_and_other_swarms() {
    use std::collections::HashSet;

    let (ledger, _drain) = ledger_pair();
    let factory = Arc::new(ControllableFactory::new(
        FactoryBehavior::Succeed,
        Duration::from_millis(1),
        ledger.clone(),
    ));
    let sink = Arc::new(RecordingSwarmSink::new());
    let budget = RunBudget::defaulted(16)
        .with_concurrency(16)
        .with_lifetime_spawns(16);
    let coordinator = SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        SwarmConfig::new(budget),
        factory,
        sink,
        ledger,
    );

    // Three sessions in alpha, one in beta, one ungrouped.
    let a1 = instance(1);
    let a2 = instance(2);
    let a3 = instance(3);
    let b1 = instance(4);
    let u1 = instance(5);
    for (iid, swarm) in [
        (a1, Some("swarm-alpha")),
        (a2, Some("swarm-alpha")),
        (a3, Some("swarm-alpha")),
        (b1, Some("swarm-beta")),
    ] {
        let mut req = spawn_req(iid);
        if let Some(s) = swarm {
            req = req.with_swarm(s);
        }
        coordinator.spawn_session(req).await.unwrap();
    }
    coordinator.spawn_session(spawn_req(u1)).await.unwrap(); // ungrouped

    let alpha: HashSet<_> = coordinator
        .live_instances_in_swarm("swarm-alpha")
        .into_iter()
        .collect();
    assert_eq!(
        alpha,
        HashSet::from([a1, a2, a3]),
        "all three alpha sessions"
    );
    assert_eq!(
        coordinator.live_instances_in_swarm("swarm-beta"),
        vec![b1],
        "exactly the one beta session"
    );

    // Blank query and unknown swarm both return empty — a teardown must never
    // fan out to ungrouped or unrelated sessions.
    assert!(coordinator.live_instances_in_swarm("").is_empty());
    assert!(coordinator.live_instances_in_swarm("   ").is_empty());
    assert!(coordinator
        .live_instances_in_swarm("swarm-unknown")
        .is_empty());

    // Cancelling a2 makes it terminal -> the accessor drops it (no dead id).
    coordinator
        .cancel_session(a2, "test_teardown")
        .await
        .unwrap();
    let alpha_after: HashSet<_> = coordinator
        .live_instances_in_swarm("swarm-alpha")
        .into_iter()
        .collect();
    assert_eq!(
        alpha_after,
        HashSet::from([a1, a3]),
        "terminal session excluded after cancel"
    );
}

// ===========================================================================
// RANK-6: cold-start admission bounds SIMULTANEOUS boots below run-concurrency.
// ===========================================================================

#[tokio::test(flavor = "multi_thread")]
async fn rank6_committed_memory_budget_rejects_no_overcommit_before_factory_create() {
    let (ledger, _drain) = ledger_pair();
    let factory = Arc::new(ControllableFactory::new(
        FactoryBehavior::Succeed,
        Duration::from_millis(1),
        ledger.clone(),
    ));
    let sink = Arc::new(RecordingSwarmSink::new());

    let budget = RunBudget::defaulted(4)
        .with_concurrency(4)
        .with_committed_memory_ceiling(1024);
    let coordinator = SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        SwarmConfig::new(budget),
        factory.clone(),
        sink.clone(),
        ledger,
    );

    coordinator
        .spawn_session(spawn_req(instance(1)).with_committed_memory_bytes(768))
        .await
        .unwrap();

    let calls_before = factory.create_calls.load(Ordering::SeqCst);
    let err = coordinator
        .spawn_session(spawn_req(instance(2)).with_committed_memory_bytes(512))
        .await
        .unwrap_err();
    match err {
        SwarmError::BudgetExhausted { dimension } => {
            assert_eq!(dimension, "committed_memory");
        }
        other => panic!("expected committed-memory BudgetExhausted, got {other}"),
    }
    assert_eq!(
        factory.create_calls.load(Ordering::SeqCst),
        calls_before,
        "memory admission must reject before factory.create so no model/VM boot starts"
    );
    assert!(sink.events().iter().any(|event| matches!(
        event,
        super::events::SwarmEvent::SpawnRejected { reason, .. }
            if reason == "budget:committed_memory"
    )));
}

#[tokio::test(flavor = "multi_thread")]
async fn rank6_committed_memory_reservation_releases_on_terminal_teardown() {
    let (ledger, _drain) = ledger_pair();
    let factory = Arc::new(ControllableFactory::new(
        FactoryBehavior::Succeed,
        Duration::from_millis(1),
        ledger.clone(),
    ));
    let sink = Arc::new(RecordingSwarmSink::new());

    let budget = RunBudget::defaulted(2)
        .with_concurrency(2)
        .with_committed_memory_ceiling(1024);
    let coordinator = SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        SwarmConfig::new(budget),
        factory,
        sink,
        ledger,
    );

    let first = instance(10);
    coordinator
        .spawn_session(spawn_req(first).with_committed_memory_bytes(1024))
        .await
        .unwrap();
    assert_eq!(
        coordinator.remaining().committed_memory_bytes_remaining,
        Some(0)
    );

    coordinator
        .cancel_session(first, "memory-release-test")
        .await
        .unwrap();
    assert_eq!(
        coordinator.remaining().committed_memory_bytes_remaining,
        Some(1024),
        "terminal teardown must return the committed-memory reservation"
    );

    coordinator
        .spawn_session(spawn_req(instance(11)).with_committed_memory_bytes(1024))
        .await
        .unwrap();
}

#[test]
fn rank6_committed_memory_budget_serde_accepts_legacy_snapshots() {
    let budget: RunBudget = serde_json::from_value(serde_json::json!({
        "max_concurrent": 4,
        "max_concurrent_cold_starts": 2,
        "max_lifetime_spawns": 99,
        "max_total_tokens": null,
        "max_total_cost_micros": null
    }))
    .expect("legacy RunBudget without committed-memory field must decode");
    assert_eq!(budget.max_committed_memory_bytes, None);

    let remaining: super::ids::BudgetRemaining = serde_json::from_value(serde_json::json!({
        "concurrency_permits_available": 1,
        "lifetime_spawns_remaining": 2,
        "tokens_remaining": null,
        "cost_micros_remaining": null,
        "exhausted": false
    }))
    .expect("legacy BudgetRemaining without committed-memory field must decode");
    assert_eq!(remaining.committed_memory_bytes_remaining, None);
}

#[tokio::test(flavor = "multi_thread")]
async fn rank6_committed_memory_ceiling_rejects_unestimated_spawn_before_factory_create() {
    let (ledger, _drain) = ledger_pair();
    let factory = Arc::new(ControllableFactory::new(
        FactoryBehavior::Succeed,
        Duration::from_millis(1),
        ledger.clone(),
    ));
    let budget = RunBudget::defaulted(1)
        .with_concurrency(1)
        .with_committed_memory_ceiling(1024);
    let coordinator = SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        SwarmConfig::new(budget),
        factory.clone(),
        Arc::new(RecordingSwarmSink::new()),
        ledger,
    );

    let err = coordinator
        .spawn_session(spawn_req(instance(20)))
        .await
        .unwrap_err();
    match err {
        SwarmError::BudgetExhausted { dimension } => {
            assert_eq!(dimension, "committed_memory_unestimated");
        }
        other => panic!("expected unestimated committed-memory refusal, got {other}"),
    }
    assert_eq!(factory.create_calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        coordinator.remaining().committed_memory_bytes_remaining,
        Some(1024)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn rank6_committed_memory_ceiling_allows_cloud_without_host_estimate() {
    let (ledger, _drain) = ledger_pair();
    let factory = Arc::new(ControllableFactory::new(
        FactoryBehavior::Succeed,
        Duration::from_millis(1),
        ledger.clone(),
    ));
    let budget = RunBudget::defaulted(1)
        .with_concurrency(1)
        .with_committed_memory_ceiling(1);
    let coordinator = SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        SwarmConfig::new(budget),
        factory.clone(),
        Arc::new(RecordingSwarmSink::new()),
        ledger,
    );

    coordinator
        .spawn_session(
            spawn_req(instance(21)).with_cloud_provider(ProviderKind::ByokCloud, "gpt-4o"),
        )
        .await
        .expect("cloud request does not reserve host committed memory");

    assert_eq!(
        factory.create_calls.load(Ordering::SeqCst),
        1,
        "cloud admission should reach the factory even without a host-memory estimate"
    );
    assert_eq!(
        coordinator.remaining().committed_memory_bytes_remaining,
        Some(1),
        "cloud session leaves the local committed-memory budget untouched"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn rank6_committed_memory_saturated_local_budget_still_allows_cloud() {
    let (ledger, _drain) = ledger_pair();
    let factory = Arc::new(ControllableFactory::new(
        FactoryBehavior::Succeed,
        Duration::from_millis(1),
        ledger.clone(),
    ));
    let budget = RunBudget::defaulted(2)
        .with_concurrency(2)
        .with_committed_memory_ceiling(1024);
    let coordinator = SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        SwarmConfig::new(budget),
        factory.clone(),
        Arc::new(RecordingSwarmSink::new()),
        ledger,
    );

    coordinator
        .spawn_session(spawn_req(instance(30)).with_committed_memory_bytes(1024))
        .await
        .expect("local session saturates committed-memory budget");
    assert_eq!(
        coordinator.remaining().committed_memory_bytes_remaining,
        Some(0)
    );

    coordinator
        .spawn_session(
            spawn_req(instance(31)).with_cloud_provider(ProviderKind::ByokCloud, "gpt-4o"),
        )
        .await
        .expect("cloud request must bypass local committed-memory saturation");

    assert_eq!(
        factory.create_calls.load(Ordering::SeqCst),
        2,
        "both local and cloud spawns should reach the factory"
    );
    assert_eq!(
        coordinator.remaining().committed_memory_bytes_remaining,
        Some(0),
        "cloud spawn does not consume additional committed memory"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn rank6_committed_memory_provider_split_still_honors_global_token_budget_for_cloud() {
    let (ledger, _drain) = ledger_pair();
    let factory = Arc::new(ControllableFactory::new(
        FactoryBehavior::Succeed,
        Duration::from_millis(1),
        ledger.clone(),
    ));
    let budget = RunBudget::defaulted(1)
        .with_concurrency(1)
        .with_token_ceiling(1)
        .with_committed_memory_ceiling(1024);
    let coordinator = SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        SwarmConfig::new(budget),
        factory.clone(),
        Arc::new(RecordingSwarmSink::new()),
        ledger,
    );

    coordinator.record_usage(1, 0);
    let err = coordinator
        .spawn_session(
            spawn_req(instance(32)).with_cloud_provider(ProviderKind::ByokCloud, "gpt-4o"),
        )
        .await
        .expect_err("global token exhaustion still blocks cloud");

    match err {
        SwarmError::BudgetExhausted { dimension } => assert_eq!(dimension, "tokens"),
        other => panic!("expected token BudgetExhausted, got {other}"),
    }
    assert_eq!(
        factory.create_calls.load(Ordering::SeqCst),
        0,
        "global token exhaustion rejects before factory.create"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn rank6_committed_memory_reservation_releases_on_factory_failure() {
    let (ledger, _drain) = ledger_pair();
    let factory = Arc::new(ControllableFactory::new(
        FactoryBehavior::AlwaysFail,
        Duration::from_millis(1),
        ledger.clone(),
    ));
    let budget = RunBudget::defaulted(1)
        .with_concurrency(1)
        .with_committed_memory_ceiling(1024);
    let coordinator = SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        SwarmConfig::new(budget),
        factory,
        Arc::new(RecordingSwarmSink::new()),
        ledger,
    );

    let err = coordinator
        .spawn_session(spawn_req(instance(21)).with_committed_memory_bytes(512))
        .await
        .unwrap_err();
    assert!(matches!(err, SwarmError::FactoryFailed(_)));
    assert_eq!(
        coordinator.remaining().committed_memory_bytes_remaining,
        Some(1024),
        "factory failure must roll back the pre-factory memory reservation"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rank6_committed_memory_and_lifetime_release_when_spawn_future_is_aborted_before_insert() {
    let (ledger, _drain) = ledger_pair();
    let factory = Arc::new(ControllableFactory::new(
        FactoryBehavior::Succeed,
        Duration::from_secs(30),
        ledger.clone(),
    ));
    let budget = RunBudget::defaulted(4)
        .with_concurrency(4)
        .with_lifetime_spawns(4)
        .with_committed_memory_ceiling(1024);
    let coordinator = Arc::new(SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        SwarmConfig::new(budget),
        factory.clone(),
        Arc::new(RecordingSwarmSink::new()),
        ledger,
    ));

    let coordinator_for_task = coordinator.clone();
    let task = tokio::spawn(async move {
        coordinator_for_task
            .spawn_session(spawn_req(instance(28)).with_committed_memory_bytes(1024))
            .await
    });

    for _ in 0..100 {
        if factory.create_calls.load(Ordering::SeqCst) > 0 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(
        factory.create_calls.load(Ordering::SeqCst),
        1,
        "spawn must have crossed reservation and entered factory.create before abort"
    );
    assert_eq!(
        coordinator.remaining().committed_memory_bytes_remaining,
        Some(0),
        "the in-flight spawn owns the full committed-memory reservation before abort"
    );

    task.abort();
    let join_error = task.await.expect_err("spawn task should be cancelled");
    assert!(
        join_error.is_cancelled(),
        "expected cancellation join error, got {join_error}"
    );

    let remaining = coordinator.remaining();
    assert_eq!(coordinator.live_session_count(), 0);
    assert_eq!(
        remaining.committed_memory_bytes_remaining,
        Some(1024),
        "aborting before registry insertion must release committed memory"
    );
    assert_eq!(
        remaining.lifetime_spawns_remaining, 4,
        "aborting before registry insertion must roll back lifetime admission"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn rank6_committed_memory_reservation_rolls_back_on_lifetime_and_concurrency_refusals() {
    let (ledger_a, _drain_a) = ledger_pair();
    let factory_a = Arc::new(ControllableFactory::new(
        FactoryBehavior::Succeed,
        Duration::from_millis(1),
        ledger_a.clone(),
    ));
    let lifetime_budget = RunBudget::defaulted(1)
        .with_concurrency(1)
        .with_lifetime_spawns(0)
        .with_committed_memory_ceiling(1024);
    let lifetime_coordinator = SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        SwarmConfig::new(lifetime_budget),
        factory_a.clone(),
        Arc::new(RecordingSwarmSink::new()),
        ledger_a,
    );
    let lifetime_err = lifetime_coordinator
        .spawn_session(spawn_req(instance(22)).with_committed_memory_bytes(512))
        .await
        .unwrap_err();
    assert!(matches!(
        lifetime_err,
        SwarmError::LifetimeSpawnCeilingReached { .. }
    ));
    assert_eq!(factory_a.create_calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        lifetime_coordinator
            .remaining()
            .committed_memory_bytes_remaining,
        Some(1024)
    );

    let (ledger_b, _drain_b) = ledger_pair();
    let factory_b = Arc::new(ControllableFactory::new(
        FactoryBehavior::Succeed,
        Duration::from_millis(1),
        ledger_b.clone(),
    ));
    let concurrency_budget = RunBudget::defaulted(2)
        .with_concurrency(1)
        .with_committed_memory_ceiling(1024);
    let concurrency_coordinator = SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        SwarmConfig::new(concurrency_budget),
        factory_b.clone(),
        Arc::new(RecordingSwarmSink::new()),
        ledger_b,
    );
    concurrency_coordinator
        .spawn_session(spawn_req(instance(23)).with_committed_memory_bytes(512))
        .await
        .unwrap();
    let calls_before = factory_b.create_calls.load(Ordering::SeqCst);
    let concurrency_err = concurrency_coordinator
        .spawn_session(spawn_req(instance(24)).with_committed_memory_bytes(256))
        .await
        .unwrap_err();
    assert!(matches!(
        concurrency_err,
        SwarmError::ConcurrencyCapReached { .. }
    ));
    assert_eq!(factory_b.create_calls.load(Ordering::SeqCst), calls_before);
    assert_eq!(
        concurrency_coordinator
            .remaining()
            .committed_memory_bytes_remaining,
        Some(512),
        "concurrency refusal must release the second request's reservation"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rank6_committed_memory_duplicate_loser_releases_reservation() {
    let (ledger, _drain) = ledger_pair();
    let factory = Arc::new(ControllableFactory::new(
        FactoryBehavior::Succeed,
        Duration::from_millis(40),
        ledger.clone(),
    ));
    let budget = RunBudget::defaulted(2)
        .with_concurrency(2)
        .with_committed_memory_ceiling(1024);
    let coordinator = Arc::new(SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        SwarmConfig::new(budget),
        factory,
        Arc::new(RecordingSwarmSink::new()),
        ledger,
    ));
    let iid = instance(25);
    let mut js = JoinSet::new();
    for _ in 0..2 {
        let coordinator = coordinator.clone();
        js.spawn(async move {
            coordinator
                .spawn_session(spawn_req(iid).with_committed_memory_bytes(512))
                .await
        });
    }
    let mut ok = 0;
    let mut duplicate = 0;
    while let Some(joined) = js.join_next().await {
        match joined.expect("join duplicate spawn task") {
            Ok(_) => ok += 1,
            Err(SwarmError::DuplicateInstance(_)) => duplicate += 1,
            Err(other) => panic!("unexpected duplicate-race result: {other}"),
        }
    }

    assert_eq!(ok, 1);
    assert_eq!(duplicate, 1);
    assert_eq!(
        coordinator.remaining().committed_memory_bytes_remaining,
        Some(512),
        "only the winning live session may retain a memory reservation"
    );
    coordinator.drain_all().await.unwrap();
    assert_eq!(
        coordinator.remaining().committed_memory_bytes_remaining,
        Some(1024)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn rank6_committed_memory_reaper_releases_reservation() {
    let (ledger, _drain) = ledger_pair();
    let factory = Arc::new(ControllableFactory::new(
        FactoryBehavior::Succeed,
        Duration::from_millis(1),
        ledger.clone(),
    ));
    let budget = RunBudget::defaulted(1)
        .with_concurrency(1)
        .with_committed_memory_ceiling(1024);
    let config = SwarmConfig::new(budget)
        .with_lease_ttl(Duration::from_millis(40))
        .with_reaper_scan_interval(Duration::from_millis(20));
    let coordinator = SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        config,
        factory,
        Arc::new(RecordingSwarmSink::new()),
        ledger,
    );
    coordinator.start_reaper();
    coordinator
        .spawn_session(spawn_req(instance(26)).with_committed_memory_bytes(1024))
        .await
        .unwrap();
    assert_eq!(
        coordinator.remaining().committed_memory_bytes_remaining,
        Some(0)
    );

    tokio::time::sleep(Duration::from_millis(180)).await;

    coordinator.stop_reaper();
    assert_eq!(coordinator.live_session_count(), 0);
    assert_eq!(
        coordinator.remaining().committed_memory_bytes_remaining,
        Some(1024),
        "lease reaper must return the live session's memory reservation"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rank6_committed_memory_concurrent_reservations_never_overcommit() {
    let (ledger, _drain) = ledger_pair();
    let factory = Arc::new(ControllableFactory::new(
        FactoryBehavior::Succeed,
        Duration::from_millis(40),
        ledger.clone(),
    ));
    let budget = RunBudget::defaulted(16)
        .with_concurrency(16)
        .with_committed_memory_ceiling(1024);
    let coordinator = Arc::new(SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        SwarmConfig::new(budget),
        factory,
        Arc::new(RecordingSwarmSink::new()),
        ledger,
    ));

    let n = 8usize;
    let mut js = JoinSet::new();
    for i in 0..n {
        let coordinator = coordinator.clone();
        js.spawn(async move {
            coordinator
                .spawn_session(spawn_req(instance(100 + i as u32)).with_committed_memory_bytes(512))
                .await
        });
    }
    let mut accepted = 0;
    let mut memory_refused = 0;
    while let Some(joined) = js.join_next().await {
        match joined.expect("join concurrent reservation task") {
            Ok(_) => accepted += 1,
            Err(SwarmError::BudgetExhausted { dimension }) if dimension == "committed_memory" => {
                memory_refused += 1;
            }
            Err(other) => panic!("unexpected concurrent reservation result: {other}"),
        }
    }
    assert!(
        accepted <= 2,
        "accepted {accepted} would exceed 1024/512 cap"
    );
    assert_eq!(accepted + memory_refused, n);
    assert_eq!(
        coordinator.remaining().committed_memory_bytes_remaining,
        Some(1024 - (accepted as u64 * 512))
    );
    coordinator.drain_all().await.unwrap();
    assert_eq!(
        coordinator.remaining().committed_memory_bytes_remaining,
        Some(1024)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rank6_cold_start_bound_throttles_concurrent_boots_below_run_concurrency() {
    let (ledger, _drain) = ledger_pair();
    let factory = Arc::new(ControllableFactory::new(
        FactoryBehavior::Succeed,
        Duration::from_millis(40),
        ledger.clone(),
    ));
    let peak = factory.peak_in_flight.clone();
    let sink = Arc::new(RecordingSwarmSink::new());

    // Run-concurrency is WIDE (16) so it never bottlenecks; the cold-start bound
    // is 2, so at most 2 boots run simultaneously even under a burst of admitted
    // spawns (the boot/networking layer is the scale wall, not the running count).
    let budget = RunBudget::defaulted(16)
        .with_concurrency(16)
        .with_lifetime_spawns(64)
        .with_cold_start_concurrency(2);
    let coordinator = Arc::new(SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        SwarmConfig::new(budget),
        factory.clone(),
        sink.clone(),
        ledger,
    ));

    // Fire 8 parallel spawns. All are ADMITTED by run-concurrency, but they QUEUE
    // on the cold-start bound, so boots happen at most 2 at a time -- none rejected
    // (this is what distinguishes cold-start admission from the concurrency cap).
    let n = 8usize;
    let mut js = JoinSet::new();
    let accepted = Arc::new(AtomicUsize::new(0));
    for i in 0..n {
        let c = coordinator.clone();
        let acc = accepted.clone();
        js.spawn(async move {
            if c.spawn_session(spawn_req(instance(i as u32))).await.is_ok() {
                acc.fetch_add(1, Ordering::SeqCst);
            }
        });
    }
    while js.join_next().await.is_some() {}

    assert!(
        peak.load(Ordering::SeqCst) <= 2,
        "cold-start bound must cap simultaneous boots at 2; saw peak {}",
        peak.load(Ordering::SeqCst)
    );
    assert_eq!(
        accepted.load(Ordering::SeqCst),
        n,
        "all spawns are admitted (queued on the boot bound, not rejected)"
    );
}

// ===========================================================================
// PROOF 1: concurrency cap holds under N>cap parallel spawns.
// ===========================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn proof_1_concurrency_cap_holds_under_parallel_spawns() {
    let (ledger, _drain) = ledger_pair();
    let factory = Arc::new(ControllableFactory::new(
        FactoryBehavior::Succeed,
        Duration::from_millis(40),
        ledger.clone(),
    ));
    let peak = factory.peak_in_flight.clone();
    let sink = Arc::new(RecordingSwarmSink::new());

    let cap = 3usize;
    let budget = RunBudget::defaulted(16).with_concurrency(cap);
    let coordinator = Arc::new(SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        SwarmConfig::new(budget),
        factory.clone(),
        sink.clone(),
        ledger,
    ));

    // Fire N=16 parallel spawns. Only `cap` may load at once; the rest get a
    // typed ConcurrencyCapReached (we hold each accepted session live by not
    // completing it until the JoinSet drains).
    let n = 16usize;
    let mut js = JoinSet::new();
    let accepted = Arc::new(AtomicUsize::new(0));
    let rejected = Arc::new(AtomicUsize::new(0));
    for i in 0..n {
        let c = coordinator.clone();
        let acc = accepted.clone();
        let rej = rejected.clone();
        js.spawn(async move {
            match c.spawn_session(spawn_req(instance(i as u32))).await {
                Ok(_) => {
                    acc.fetch_add(1, Ordering::SeqCst);
                }
                Err(SwarmError::ConcurrencyCapReached { .. }) => {
                    rej.fetch_add(1, Ordering::SeqCst);
                }
                Err(other) => panic!("unexpected error: {other}"),
            }
        });
    }
    while js.join_next().await.is_some() {}

    // The factory NEVER had more than `cap` loads in flight simultaneously.
    assert!(
        peak.load(Ordering::SeqCst) <= cap,
        "peak factory in-flight {} exceeded cap {cap}",
        peak.load(Ordering::SeqCst)
    );
    // Slot occupancy in the registry never exceeds cap right now either.
    assert!(coordinator.slot_occupancy() <= cap);
    // Some were accepted (<= cap live) and the remainder were cap-rejected.
    assert!(accepted.load(Ordering::SeqCst) <= cap);
    assert_eq!(
        accepted.load(Ordering::SeqCst) + rejected.load(Ordering::SeqCst),
        n
    );
    assert!(rejected.load(Ordering::SeqCst) >= n - cap);

    coordinator.drain_all().await.unwrap();
}

// ===========================================================================
// PROOF 2: lifetime spawn ceiling rejects past the cap with a typed error.
// ===========================================================================

#[tokio::test(flavor = "multi_thread")]
async fn proof_2_lifetime_spawn_ceiling_rejects_with_typed_error() {
    let (ledger, _drain) = ledger_pair();
    let factory = Arc::new(ControllableFactory::new(
        FactoryBehavior::Succeed,
        Duration::from_millis(1),
        ledger.clone(),
    ));
    let sink = Arc::new(RecordingSwarmSink::new());

    // Concurrency wide enough; lifetime ceiling = 4.
    let budget = RunBudget::defaulted(8)
        .with_concurrency(8)
        .with_lifetime_spawns(4);
    let coordinator = SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        SwarmConfig::new(budget),
        factory,
        sink,
        ledger,
    );

    // Spawn + immediately complete so a slot is always free; the lifetime
    // counter is monotonic and never replenished.
    for i in 0..4u32 {
        let iid = instance(i);
        coordinator.spawn_session(spawn_req(iid)).await.unwrap();
        coordinator.complete_session(iid).await.unwrap();
    }
    // The 5th spawn must be rejected by the lifetime ceiling, typed.
    let err = coordinator
        .spawn_session(spawn_req(instance(99)))
        .await
        .unwrap_err();
    match err {
        SwarmError::LifetimeSpawnCeilingReached { spawned, ceiling } => {
            assert_eq!(ceiling, 4);
            assert_eq!(spawned, 4);
        }
        other => panic!("expected LifetimeSpawnCeilingReached, got {other}"),
    }
    assert_eq!(coordinator.remaining().lifetime_spawns_remaining, 0);
}

// ===========================================================================
// PROOF 3: lease expiry -> reaper reclaims + cancels + records reclaim.
// ===========================================================================

#[tokio::test(flavor = "multi_thread")]
async fn proof_3_lease_expiry_reaper_reclaims_cancels_records() {
    let (ledger, drain_h) = ledger_pair();
    let store = Arc::new(InMemoryStore::default());
    let factory = Arc::new(ControllableFactory::new(
        FactoryBehavior::Succeed,
        Duration::from_millis(1),
        ledger.clone(),
    ));
    let sink = Arc::new(RecordingSwarmSink::new());

    let budget = RunBudget::defaulted(4).with_concurrency(4);
    let config = SwarmConfig::new(budget)
        .with_lease_ttl(Duration::from_millis(50))
        .with_reaper_scan_interval(Duration::from_millis(20));
    let coordinator = SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        config,
        factory,
        sink.clone(),
        ledger,
    );
    coordinator.start_reaper();

    let iid = instance(0);
    coordinator.spawn_session(spawn_req(iid)).await.unwrap();
    assert_eq!(coordinator.live_session_count(), 1);

    // Do NOT renew the lease; wait past TTL + a scan interval.
    tokio::time::sleep(Duration::from_millis(200)).await;

    // The reaper reclaimed it: gone from the registry, lease-expired event,
    // and evicted event emitted.
    assert_eq!(coordinator.live_session_count(), 0);
    assert!(sink.contains(SwarmFrEventId::LeaseExpired));
    assert!(sink.contains(SwarmFrEventId::ResourceEvicted));

    coordinator.stop_reaper();

    // The ledger has a matching stop row for the reclaimed process.
    let rows = drain(&drain_h, store).await;
    let stops = rows
        .iter()
        .filter(|e| matches!(e, LedgerEvent::Stop(_)))
        .count();
    assert_eq!(stops, 1, "reaper must record exactly one reclaim stop");
}

// ===========================================================================
// RANK-7: a per-spawn time_box drives reaping independently of the config TTL.
// ===========================================================================

#[tokio::test(flavor = "multi_thread")]
async fn rank7_time_boxed_session_is_reaped_at_its_box_while_untimed_survives() {
    let (ledger, _drain) = ledger_pair();
    let factory = Arc::new(ControllableFactory::new(
        FactoryBehavior::Succeed,
        Duration::from_millis(1),
        ledger.clone(),
    ));
    let sink = Arc::new(RecordingSwarmSink::new());

    // Configured lease_ttl is LONG (60s) so an untimed session never expires
    // during the test; the reaper scans frequently.
    let budget = RunBudget::defaulted(4).with_concurrency(4);
    let config = SwarmConfig::new(budget)
        .with_lease_ttl(Duration::from_secs(60))
        .with_reaper_scan_interval(Duration::from_millis(20));
    let coordinator = SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        config,
        factory,
        sink.clone(),
        ledger,
    );
    coordinator.start_reaper();

    // A is time-boxed to 30ms; B uses the 60s default (no time_box).
    let a = instance(0);
    let b = instance(1);
    coordinator
        .spawn_session(spawn_req(a).with_time_box(Duration::from_millis(30)))
        .await
        .unwrap();
    coordinator.spawn_session(spawn_req(b)).await.unwrap();
    assert_eq!(coordinator.live_session_count(), 2);

    // Wait past A's box + a scan interval; B's 60s lease is nowhere near expiry.
    tokio::time::sleep(Duration::from_millis(200)).await;

    assert!(
        coordinator.session_state(a).is_none(),
        "the time-boxed session must be reaped at its box (no new teardown code)"
    );
    assert!(
        coordinator.session_state(b).is_some(),
        "the untimed session (60s lease) must survive"
    );
    assert_eq!(coordinator.live_session_count(), 1);
    assert!(sink.contains(SwarmFrEventId::LeaseExpired));

    coordinator.stop_reaper();
}

// ===========================================================================
// PROOF 4: failure-fingerprint breaker trips after threshold + suppresses.
// ===========================================================================

#[tokio::test(flavor = "multi_thread")]
async fn proof_4_failure_fingerprint_breaker_trips_and_suppresses() {
    let (ledger, _drain) = ledger_pair();
    let factory = Arc::new(ControllableFactory::new(
        FactoryBehavior::AlwaysFail,
        Duration::from_millis(1),
        ledger.clone(),
    ));
    let sink = Arc::new(RecordingSwarmSink::new());

    let budget = RunBudget::defaulted(8)
        .with_concurrency(8)
        .with_lifetime_spawns(1000);
    let config = SwarmConfig::new(budget).with_breaker(BreakerConfig {
        failure_threshold: 3,
        cooldown: Duration::from_secs(60),
    });
    let coordinator = SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        config,
        factory,
        sink.clone(),
        ledger,
    );

    // Each spawn fails with the SAME fingerprint (same class + message).
    // First `threshold` calls return FactoryFailed; once tripped, the breaker
    // suppresses with BreakerOpen.
    let mut factory_failed = 0;
    let mut breaker_open = 0;
    for i in 0..8u32 {
        match coordinator
            .spawn_session(spawn_req(instance(i)))
            .await
            .unwrap_err()
        {
            SwarmError::FactoryFailed(_) => factory_failed += 1,
            SwarmError::BreakerOpen { .. } => breaker_open += 1,
            other => panic!("unexpected: {other}"),
        }
    }

    assert!(sink.contains(SwarmFrEventId::BreakerTripped));
    assert_eq!(
        sink.count_of(SwarmFrEventId::BreakerTripped),
        1,
        "trip once"
    );
    assert!(
        breaker_open >= 1,
        "breaker must suppress at least one later spawn"
    );
    // The lifetime budget was NOT drained by the retries (rolled back on the
    // suppressed/failed path): far fewer than 8 lifetime spawns consumed.
    assert!(factory_failed <= 4);
}

// ===========================================================================
// PROOF 5: budget exhaustion stops spawning.
// ===========================================================================

#[tokio::test(flavor = "multi_thread")]
async fn proof_5_budget_exhaustion_stops_spawning() {
    let (ledger, _drain) = ledger_pair();
    let factory = Arc::new(ControllableFactory::new(
        FactoryBehavior::Succeed,
        Duration::from_millis(1),
        ledger.clone(),
    ));
    let sink = Arc::new(RecordingSwarmSink::new());

    let budget = RunBudget::defaulted(8)
        .with_concurrency(8)
        .with_token_ceiling(100);
    let coordinator = SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        SwarmConfig::new(budget),
        factory,
        sink,
        ledger,
    );

    // First spawn ok.
    let iid = instance(0);
    coordinator.spawn_session(spawn_req(iid)).await.unwrap();
    // Report usage that exhausts the token ceiling.
    coordinator.record_usage(100, 0);
    assert!(coordinator.remaining().exhausted);

    // Next spawn must be refused with a typed BudgetExhausted.
    let err = coordinator
        .spawn_session(spawn_req(instance(1)))
        .await
        .unwrap_err();
    match err {
        SwarmError::BudgetExhausted { dimension } => assert_eq!(dimension, "tokens"),
        other => panic!("expected BudgetExhausted, got {other}"),
    }
    coordinator.complete_session(iid).await.unwrap();
}

// ===========================================================================
// PROOF 6: cancel_session cancels + unloads + evicts + emits the event.
// ===========================================================================

#[tokio::test(flavor = "multi_thread")]
async fn proof_6_cancel_session_cancels_unloads_evicts_emits() {
    let (ledger, drain_h) = ledger_pair();
    let store = Arc::new(InMemoryStore::default());
    let factory = Arc::new(ControllableFactory::new(
        FactoryBehavior::Succeed,
        Duration::from_millis(1),
        ledger.clone(),
    ));
    let sink = Arc::new(RecordingSwarmSink::new());

    let budget = RunBudget::defaulted(4).with_concurrency(4);
    let coordinator = SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        SwarmConfig::new(budget),
        factory,
        sink.clone(),
        ledger,
    );

    let iid = instance(0);
    coordinator.spawn_session(spawn_req(iid)).await.unwrap();
    assert_eq!(
        coordinator.session_state(iid),
        Some(ModelSessionState::Ready)
    );

    coordinator
        .cancel_session(iid, "operator_cancel")
        .await
        .unwrap();

    // Evicted from registry, cancelled + evicted events emitted.
    assert_eq!(coordinator.session_state(iid), None);
    assert!(sink.contains(SwarmFrEventId::SessionCancelled));
    assert!(sink.contains(SwarmFrEventId::ResourceEvicted));
    // Permit returned: a fresh spawn can take the slot.
    assert_eq!(coordinator.remaining().concurrency_permits_available, 4);

    // Ledger has a matching stop row.
    let rows = drain(&drain_h, store).await;
    let stops = rows
        .iter()
        .filter(|e| matches!(e, LedgerEvent::Stop(_)))
        .count();
    assert_eq!(stops, 1);
}

// ===========================================================================
// PROOF 7: no orphan — after a run all sessions are terminal and every
// started process has a matching stop row.
// ===========================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn proof_7_no_orphan_after_run() {
    let (ledger, drain_h) = ledger_pair();
    let store = Arc::new(InMemoryStore::default());
    let factory = Arc::new(ControllableFactory::new(
        FactoryBehavior::Succeed,
        Duration::from_millis(5),
        ledger.clone(),
    ));
    let sink = Arc::new(RecordingSwarmSink::new());

    let budget = RunBudget::defaulted(4)
        .with_concurrency(4)
        .with_lifetime_spawns(1000);
    let coordinator = Arc::new(SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        SwarmConfig::new(budget),
        factory.clone(),
        sink,
        ledger,
    ));

    // Drive a real run: spawn many (respecting cap via retry), complete each.
    let n = 12u32;
    let completed = Arc::new(AtomicU32::new(0));
    for i in 0..n {
        let iid = instance(i);
        // Retry on concurrency cap until admitted (real backpressure loop).
        loop {
            match coordinator.spawn_session(spawn_req(iid)).await {
                Ok(_) => break,
                Err(SwarmError::ConcurrencyCapReached { .. }) => {
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
                Err(e) => panic!("unexpected: {e}"),
            }
        }
        coordinator.complete_session(iid).await.unwrap();
        completed.fetch_add(1, Ordering::SeqCst);
    }

    // No live sessions remain.
    assert_eq!(coordinator.live_session_count(), 0);
    assert_eq!(completed.load(Ordering::SeqCst), n);

    // Ledger reconciliation: equal number of START and STOP rows, and every
    // start process_uuid has a matching stop.
    let rows = drain(&drain_h, store).await;
    let starts: Vec<_> = rows
        .iter()
        .filter_map(|e| match e {
            LedgerEvent::Start(s) => Some(s.process_uuid),
            _ => None,
        })
        .collect();
    let stops: Vec<_> = rows
        .iter()
        .filter_map(|e| match e {
            LedgerEvent::Stop(s) => Some(s.process_uuid),
            _ => None,
        })
        .collect();
    assert_eq!(starts.len() as u32, n, "one start per spawn");
    assert_eq!(stops.len(), starts.len(), "one stop per start: no orphan");
    for uuid in &starts {
        assert!(stops.contains(uuid), "orphan start with no stop: {uuid}");
    }
}

// ===========================================================================
// D1: teardown actually frees the model — BOTH explicit terminate (cancel/
// complete) AND lease-expiry reclaim invoke the teardown seam, which runs the
// runtime's real `unload`. Without the fix, terminate only called
// runtime.cancel() and the model leaked forever.
// ===========================================================================

#[tokio::test(flavor = "multi_thread")]
async fn proof_d1_teardown_frees_model_on_terminate_and_lease_expiry() {
    let (ledger, _drain) = ledger_pair();
    let factory = Arc::new(ControllableFactory::new(
        FactoryBehavior::Succeed,
        Duration::from_millis(1),
        ledger.clone(),
    ));
    let teardown_calls = factory.teardown_invocations.clone();
    let unload_calls = factory.unload_invocations.clone();
    let sink = Arc::new(RecordingSwarmSink::new());

    // Short lease so the reaper reclaims the second instance.
    let budget = RunBudget::defaulted(4).with_concurrency(4);
    let config = SwarmConfig::new(budget)
        .with_lease_ttl(Duration::from_millis(40))
        .with_reaper_scan_interval(Duration::from_millis(15));
    let coordinator = SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        config,
        factory.clone(),
        sink,
        ledger,
    );

    // (a) Explicit cancel path invokes teardown -> unload exactly once.
    let iid0 = instance(0);
    coordinator.spawn_session(spawn_req(iid0)).await.unwrap();
    let (served_runtime0, served_model0) = factory
        .handed_out
        .lock()
        .unwrap()
        .get(&iid0)
        .cloned()
        .expect("session-serving runtime");
    served_runtime0
        .score(served_model0, Vec::new())
        .await
        .expect("serving runtime uses the loaded resource before teardown");
    assert_eq!(teardown_calls.load(Ordering::SeqCst), 0);
    assert_eq!(unload_calls.load(Ordering::SeqCst), 0);
    coordinator
        .cancel_session(iid0, "operator_cancel")
        .await
        .unwrap();
    assert_eq!(
        teardown_calls.load(Ordering::SeqCst),
        1,
        "terminate must invoke the teardown seam"
    );
    assert_eq!(
        unload_calls.load(Ordering::SeqCst),
        1,
        "teardown must actually free the model (real unload), not just cancel"
    );
    assert!(
        matches!(
            served_runtime0.score(served_model0, Vec::new()).await,
            Err(ModelRuntimeError::ScoreError(message)) if message.contains("unloaded")
        ),
        "the exact session-serving runtime must lose access to the unloaded resource"
    );

    // (b) Lease-expiry reclaim path also invokes teardown -> unload.
    coordinator.start_reaper();
    let iid1 = instance(1);
    coordinator.spawn_session(spawn_req(iid1)).await.unwrap();
    let (served_runtime1, served_model1) = factory
        .handed_out
        .lock()
        .unwrap()
        .get(&iid1)
        .cloned()
        .expect("lease-reaped session-serving runtime");
    // Do not renew; wait past TTL + a scan.
    tokio::time::sleep(Duration::from_millis(160)).await;
    coordinator.stop_reaper();
    assert_eq!(coordinator.live_session_count(), 0);
    assert_eq!(
        teardown_calls.load(Ordering::SeqCst),
        2,
        "reaper reclaim must invoke the teardown seam"
    );
    assert_eq!(
        unload_calls.load(Ordering::SeqCst),
        2,
        "reaper teardown must actually free the model"
    );
    assert!(
        matches!(
            served_runtime1.score(served_model1, Vec::new()).await,
            Err(ModelRuntimeError::ScoreError(message)) if message.contains("unloaded")
        ),
        "lease reclaim must free the resource used by the exact serving runtime"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn teardown_failure_retains_live_cleanup_handle_until_retry_succeeds() {
    let (ledger, _drain) = ledger_pair();
    let factory = Arc::new(ControllableFactory::new(
        FactoryBehavior::Succeed,
        Duration::from_millis(1),
        ledger.clone(),
    ));
    factory.fail_teardown_remaining.store(1, Ordering::SeqCst);
    let teardown_calls = factory.teardown_invocations.clone();
    let unload_calls = factory.unload_invocations.clone();
    let coordinator = SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        SwarmConfig::new(RunBudget::defaulted(1)),
        factory,
        Arc::new(RecordingSwarmSink::new()),
        ledger,
    );
    let instance_id = instance(90);
    coordinator
        .spawn_session(spawn_req(instance_id))
        .await
        .unwrap();

    let error = coordinator
        .cancel_session(instance_id, "teardown_failure_probe")
        .await
        .expect_err("first teardown must fail");
    assert!(error
        .to_string()
        .contains("injected retryable teardown failure"));
    assert_eq!(coordinator.live_session_count(), 1);
    assert_eq!(
        coordinator.session_state(instance_id),
        Some(ModelSessionState::Cancelling)
    );
    assert_eq!(unload_calls.load(Ordering::SeqCst), 0);

    coordinator.retry_pending_session_cleanups().await.unwrap();
    assert_eq!(coordinator.live_session_count(), 0);
    assert_eq!(teardown_calls.load(Ordering::SeqCst), 2);
    assert_eq!(unload_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn stop_queue_failure_retains_teardown_receipt_and_retries_without_double_unload() {
    let (ledger, drain_handle) = LedgerBatcher::manual_for_tests(
        LedgerBatcherConfig {
            capacity: 1,
            batch_size: 1,
            ..LedgerBatcherConfig::default()
        },
        Arc::new(FailingOverflowSink),
    )
    .expect("manual ledger");
    let factory = Arc::new(ControllableFactory::new(
        FactoryBehavior::Succeed,
        Duration::from_millis(1),
        ledger.clone(),
    ));
    let teardown_calls = factory.teardown_invocations.clone();
    let unload_calls = factory.unload_invocations.clone();
    let coordinator = SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        SwarmConfig::new(RunBudget::defaulted(1)),
        factory,
        Arc::new(RecordingSwarmSink::new()),
        ledger,
    );
    let instance_id = instance(91);
    coordinator
        .spawn_session(spawn_req(instance_id))
        .await
        .unwrap();

    coordinator
        .cancel_session(instance_id, "stop_queue_failure_probe")
        .await
        .expect_err("full queue must reject STOP");
    assert_eq!(coordinator.live_session_count(), 1);
    assert_eq!(
        coordinator.session_state(instance_id),
        Some(ModelSessionState::Cancelling)
    );
    assert_eq!(teardown_calls.load(Ordering::SeqCst), 1);
    assert_eq!(unload_calls.load(Ordering::SeqCst), 1);

    let store = Arc::new(InMemoryStore::default());
    drain_handle.drain_available_to(store).await.unwrap();
    coordinator.retry_pending_session_cleanups().await.unwrap();
    assert_eq!(coordinator.live_session_count(), 0);
    assert_eq!(teardown_calls.load(Ordering::SeqCst), 1);
    assert_eq!(unload_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_cancel_and_cleanup_retry_have_one_owner_one_teardown_and_one_stop() {
    let (ledger, drain_handle) = ledger_pair();
    let factory = Arc::new(ControllableFactory::new(
        FactoryBehavior::Succeed,
        Duration::from_millis(1),
        ledger.clone(),
    ));
    factory.hold_teardown.store(true, Ordering::SeqCst);
    let teardown_calls = factory.teardown_invocations.clone();
    let unload_calls = factory.unload_invocations.clone();
    let sink = Arc::new(RecordingSwarmSink::new());
    let coordinator = Arc::new(SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        SwarmConfig::new(RunBudget::defaulted(1)),
        factory.clone(),
        sink.clone(),
        ledger,
    ));
    let instance_id = instance(92);
    coordinator
        .spawn_session(spawn_req(instance_id))
        .await
        .unwrap();

    let cancelling = {
        let coordinator = coordinator.clone();
        tokio::spawn(async move {
            coordinator
                .cancel_session(instance_id, "overlap_cleanup_probe")
                .await
        })
    };
    for _ in 0..200 {
        if teardown_calls.load(Ordering::SeqCst) == 1 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert_eq!(teardown_calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        coordinator.session_state(instance_id),
        Some(ModelSessionState::Cancelling)
    );
    assert!(!sink.events().iter().any(|event| matches!(
        event,
        SwarmEvent::SessionCancelled { instance_id: observed, .. } if *observed == instance_id
    )));

    let overlap_error = coordinator
        .retry_pending_session_cleanups()
        .await
        .expect_err("concurrent retry must not acquire the active cleanup generation");
    assert!(overlap_error
        .to_string()
        .contains("cleanup is already in progress"));
    assert_eq!(teardown_calls.load(Ordering::SeqCst), 1);

    factory.hold_teardown.store(false, Ordering::SeqCst);
    cancelling
        .await
        .expect("join owning cancellation")
        .expect("owning cleanup reaches durable receipt and terminalization");
    assert_eq!(coordinator.live_session_count(), 0);
    assert_eq!(teardown_calls.load(Ordering::SeqCst), 1);
    assert_eq!(unload_calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        sink.events().iter().filter(|event| matches!(
            event,
            SwarmEvent::SessionCancelled { instance_id: observed, .. } if *observed == instance_id
        )).count(),
        1
    );

    let rows = drain(&drain_handle, Arc::new(InMemoryStore::default())).await;
    assert_eq!(
        rows.iter()
            .filter(|event| matches!(event, LedgerEvent::Stop(_)))
            .count(),
        1,
        "cleanup ownership fence must emit exactly one lossless STOP"
    );
}

// ===========================================================================
// D2: concurrent same-instance spawn — exactly one Ready, one
// DuplicateInstance, and ledger START count == STOP count (no orphan START).
// Without the fix the non-atomic check+insert recorded two STARTs and dropped
// the first handle (orphan START, no STOP).
// ===========================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn proof_d2_concurrent_same_instance_no_orphan_start() {
    let (ledger, drain_h) = ledger_pair();
    let store = Arc::new(InMemoryStore::default());
    // Non-trivial load delay so both spawns are genuinely in the factory at the
    // same time, maximising the race window.
    let factory = Arc::new(ControllableFactory::new(
        FactoryBehavior::Succeed,
        Duration::from_millis(30),
        ledger.clone(),
    ));
    let sink = Arc::new(RecordingSwarmSink::new());

    let budget = RunBudget::defaulted(8)
        .with_concurrency(8)
        .with_lifetime_spawns(1000);
    let coordinator = Arc::new(SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        SwarmConfig::new(budget),
        factory,
        sink,
        ledger,
    ));

    // Fire the SAME instance_id concurrently many times.
    let iid = instance(0);
    let racers = 6usize;
    let mut js = JoinSet::new();
    let ready = Arc::new(AtomicUsize::new(0));
    let dup = Arc::new(AtomicUsize::new(0));
    for _ in 0..racers {
        let c = coordinator.clone();
        let r = ready.clone();
        let d = dup.clone();
        js.spawn(async move {
            match c.spawn_session(spawn_req(iid)).await {
                Ok(_) => {
                    r.fetch_add(1, Ordering::SeqCst);
                }
                Err(SwarmError::DuplicateInstance(_)) => {
                    d.fetch_add(1, Ordering::SeqCst);
                }
                Err(other) => panic!("unexpected error: {other}"),
            }
        });
    }
    while js.join_next().await.is_some() {}

    assert_eq!(ready.load(Ordering::SeqCst), 1, "exactly one spawn wins");
    assert_eq!(
        dup.load(Ordering::SeqCst),
        racers - 1,
        "all other concurrent spawns get DuplicateInstance"
    );
    assert_eq!(coordinator.live_session_count(), 1);

    // Tear the winner down so its START also gets a STOP.
    coordinator.cancel_session(iid, "cleanup").await.unwrap();

    // Ledger reconciliation: START count == STOP count, no orphan START. Every
    // session that recorded a START (winner + each loser that created before
    // losing the race) has a matching STOP.
    let rows = drain(&drain_h, store).await;
    let (starts, stops) = count_start_stop(&rows);
    assert_eq!(
        starts, stops,
        "START count must equal STOP count: no orphan START (got {starts} starts, {stops} stops)"
    );
}

// ===========================================================================
// D3: an Open breaker GATES the factory — once tripped, subsequent spawns
// return BreakerOpen WITHOUT the factory's create being called (the create
// counter does not increment while suppressed).
// ===========================================================================

#[tokio::test(flavor = "multi_thread")]
async fn proof_d3_open_breaker_gates_factory_admission() {
    let (ledger, _drain) = ledger_pair();
    let factory = Arc::new(ControllableFactory::new(
        FactoryBehavior::AlwaysFail,
        Duration::from_millis(1),
        ledger.clone(),
    ));
    let create_calls = factory.create_calls.clone();
    let sink = Arc::new(RecordingSwarmSink::new());

    let budget = RunBudget::defaulted(4)
        .with_concurrency(4)
        .with_lifetime_spawns(1000);
    let config = SwarmConfig::new(budget).with_breaker(BreakerConfig {
        failure_threshold: 3,
        cooldown: Duration::from_secs(60),
    });
    let coordinator = SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        config,
        factory.clone(),
        sink,
        ledger,
    );

    // Trip the breaker on a SINGLE instance with the same fingerprint. The
    // admission gate keys on the instance's last-seen signature, so we drive
    // the same instance_id to threshold.
    let iid = instance(7);
    for _ in 0..3u32 {
        match coordinator.spawn_session(spawn_req(iid)).await {
            Err(SwarmError::FactoryFailed(_)) | Err(SwarmError::BreakerOpen { .. }) => {}
            other => panic!("unexpected: {other:?}"),
        }
    }
    // The factory was genuinely entered while tripping (real failures).
    let calls_at_trip = create_calls.load(Ordering::SeqCst);
    assert!(calls_at_trip >= 1, "real factory create() calls happened");

    // Now attempt more spawns of the SAME instance. They must be suppressed
    // BEFORE the factory is called: the create-call counter must NOT increment.
    let mut breaker_open = 0usize;
    for _ in 0..5u32 {
        match coordinator.spawn_session(spawn_req(iid)).await {
            Err(SwarmError::BreakerOpen { .. }) => breaker_open += 1,
            Err(SwarmError::FactoryFailed(_)) => {
                panic!("factory was called while breaker should suppress admission")
            }
            other => panic!("unexpected: {other:?}"),
        }
    }
    assert!(breaker_open >= 1, "open breaker must suppress later spawns");
    assert_eq!(
        create_calls.load(Ordering::SeqCst),
        calls_at_trip,
        "factory create() must NOT be entered while the breaker gates admission \
         (D3): expected {calls_at_trip}, got {}",
        create_calls.load(Ordering::SeqCst)
    );
}

// ===========================================================================
// C4: a signature that tripped the breaker recovers after a REAL success, not
// only via cooldown. We trip on a failing factory, then swap in a succeeding
// factory and prove the next spawn of that instance heals the breaker.
// ===========================================================================

#[tokio::test(flavor = "multi_thread")]
async fn proof_c4_breaker_heals_via_real_success() {
    use super::breaker::{BreakerState, FailureFingerprint};
    use super::error::SwarmErrorClass;

    let (ledger, _drain) = ledger_pair();
    // A factory that fails its first 2 creates (same fingerprint) then succeeds
    // once `allow_success` is set. Same instance is re-spawned throughout.
    let factory = Arc::new(HealableFactory::new(ledger.clone(), 2));
    let sink = Arc::new(RecordingSwarmSink::new());

    let budget = RunBudget::defaulted(4)
        .with_concurrency(4)
        .with_lifetime_spawns(1000);
    // Short cooldown so the breaker half-opens and admits a probe; the REAL
    // success on that probe is what must heal it. The probe alone (half-open)
    // does NOT reset consecutive_failures to 0 — only record_success does, which
    // is exactly the wiring C4 fixes (the old breaker_success_for_instance was a
    // no-op).
    let config = SwarmConfig::new(budget).with_breaker(BreakerConfig {
        failure_threshold: 2,
        cooldown: Duration::from_millis(60),
    });
    let coordinator = SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        config,
        factory.clone(),
        sink.clone(),
        ledger,
    );

    let fp = FailureFingerprint::compute(SwarmErrorClass::FactoryFailed, &factory.fail_message);

    // Drive the SAME instance to trip the breaker for this signature.
    let iid = instance(3);
    for _ in 0..2 {
        match coordinator.spawn_session(spawn_req(iid)).await {
            Err(SwarmError::FactoryFailed(_)) | Err(SwarmError::BreakerOpen { .. }) => {}
            other => panic!("unexpected: {other:?}"),
        }
    }
    assert_eq!(
        coordinator.breaker_state_for_test(&fp),
        BreakerState::Open,
        "breaker Open after threshold failures"
    );
    assert!(coordinator.breaker_consecutive_failures_for_test(&fp) >= 2);

    // Let the factory succeed from now on, and wait past the cooldown so the
    // breaker half-opens and admits the next probe.
    factory.allow_success();
    tokio::time::sleep(Duration::from_millis(90)).await;

    // The next spawn of this instance is admitted (half-open probe) and the
    // factory now SUCCEEDS. The Ok path heals the tracked signature via a real
    // record_success — fully closing the breaker and resetting the failure
    // count. Without the C4 fix this success would NOT have healed `fp`.
    coordinator.spawn_session(spawn_req(iid)).await.unwrap();

    assert_eq!(
        coordinator.breaker_state_for_test(&fp),
        BreakerState::Closed,
        "a real success must heal the tripped signature (not only cooldown)"
    );
    assert_eq!(
        coordinator.breaker_consecutive_failures_for_test(&fp),
        0,
        "real success resets the consecutive-failure count to 0"
    );
    assert!(
        coordinator.breaker_admits(&fp),
        "healed signature admitted again"
    );

    coordinator.cancel_session(iid, "cleanup").await.unwrap();
}

// ===========================================================================
// C6: cancelling an already-reaped instance does NOT emit a spurious
// SessionCancelled. The event is folded into terminate, after the handle was
// actually removed.
// ===========================================================================

#[tokio::test(flavor = "multi_thread")]
async fn proof_c6_no_spurious_cancel_event_for_reaped_instance() {
    let (ledger, _drain) = ledger_pair();
    let factory = Arc::new(ControllableFactory::new(
        FactoryBehavior::Succeed,
        Duration::from_millis(1),
        ledger.clone(),
    ));
    let sink = Arc::new(RecordingSwarmSink::new());

    let budget = RunBudget::defaulted(4).with_concurrency(4);
    let config = SwarmConfig::new(budget)
        .with_lease_ttl(Duration::from_millis(30))
        .with_reaper_scan_interval(Duration::from_millis(12));
    let coordinator = SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        config,
        factory,
        sink.clone(),
        ledger,
    );
    coordinator.start_reaper();

    let iid = instance(0);
    coordinator.spawn_session(spawn_req(iid)).await.unwrap();
    // Let the reaper reclaim it.
    tokio::time::sleep(Duration::from_millis(140)).await;
    coordinator.stop_reaper();
    assert_eq!(coordinator.live_session_count(), 0);
    let cancels_before = sink.count_of(SwarmFrEventId::SessionCancelled);

    // Cancelling the already-reaped instance must return UnknownInstance and
    // emit NO SessionCancelled event.
    let err = coordinator
        .cancel_session(iid, "late_cancel")
        .await
        .unwrap_err();
    assert!(
        matches!(err, SwarmError::UnknownInstance(_)),
        "cancel of reaped instance is UnknownInstance, got {err}"
    );
    assert_eq!(
        sink.count_of(SwarmFrEventId::SessionCancelled),
        cancels_before,
        "no spurious SessionCancelled for an already-reaped instance"
    );
}

// ===========================================================================
// C5: after many terminal sessions the per-instance accounting maps and the
// breaker signature map do NOT grow without bound (pruned on terminal/evict).
// ===========================================================================

#[tokio::test(flavor = "multi_thread")]
async fn proof_c5_maps_do_not_grow_unbounded_after_many_terminals() {
    let (ledger, _drain) = ledger_pair();
    // Each instance: one real factory FAILURE (records a per-instance signature
    // AND a breaker signature) followed by a successful spawn + complete. The
    // failure populates the maps; the terminal eviction + heal must prune them
    // so they stay bounded regardless of how many instances churn through.
    let factory = Arc::new(FailThenSucceedFactory::new(ledger.clone()));
    let sink = Arc::new(RecordingSwarmSink::new());

    let budget = RunBudget::defaulted(4)
        .with_concurrency(4)
        .with_lifetime_spawns(10_000);
    // High breaker threshold so a single failure per instance never trips it
    // (we are testing map growth, not tripping).
    let config = SwarmConfig::new(budget).with_breaker(BreakerConfig {
        failure_threshold: 1000,
        cooldown: Duration::from_secs(1),
    });
    let coordinator = SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        config,
        factory.clone(),
        sink,
        ledger,
    );

    let n = 200u32;
    let mut peak_signatures = 0usize;
    for i in 0..n {
        let iid = instance(i);
        // First attempt fails for this instance (populates the maps).
        match coordinator.spawn_session(spawn_req(iid)).await {
            Err(SwarmError::FactoryFailed(_)) => {}
            other => panic!("expected first attempt to fail: {other:?}"),
        }
        // Track how big the per-instance signature map ever gets mid-run.
        let (_r, sig) = coordinator.accounting_map_sizes();
        peak_signatures = peak_signatures.max(sig);
        // Second attempt succeeds (heals the signature) then completes.
        coordinator.spawn_session(spawn_req(iid)).await.unwrap();
        coordinator.complete_session(iid).await.unwrap();
    }

    // The maps were populated mid-run (proving the test is not vacuous) ...
    assert!(
        peak_signatures >= 1,
        "the per-instance signature map must have been populated by failures"
    );
    // ... but after each instance terminates they are pruned, so the final size
    // is bounded (here: empty) regardless of n=200.
    assert_eq!(coordinator.live_session_count(), 0);
    let (respawns, signatures) = coordinator.accounting_map_sizes();
    assert_eq!(respawns, 0, "respawn map must be pruned on terminal");
    assert_eq!(
        signatures, 0,
        "per-instance signature map must be pruned (heal-on-success + terminal evict), not grow to n"
    );
    // The breaker signature map is also bounded well below n: healed/settled
    // signatures are pruned by the reaper and closed by success.
    assert!(
        coordinator.breaker_signature_count() < n as usize,
        "breaker signature map must not grow to one-per-instance (got {}, n={n})",
        coordinator.breaker_signature_count()
    );
}

// ---------------------------------------------------------------------------
// Helpers + extra factories used by the new proofs.
// ---------------------------------------------------------------------------

fn count_start_stop(rows: &[LedgerEvent]) -> (usize, usize) {
    let starts = rows
        .iter()
        .filter(|e| matches!(e, LedgerEvent::Start(_)))
        .count();
    let stops = rows
        .iter()
        .filter(|e| matches!(e, LedgerEvent::Stop(_)))
        .count();
    (starts, stops)
}

// ===========================================================================
// FAIL-SCENARIO (2): FAILURE-FINGERPRINT STORM CONTAINMENT. MANY sessions fail
// concurrently with the SAME signature. The class-keyed breaker must trip ONCE
// and then suppress the whole class — NOT spin up N independent breakers. We
// fire a concurrent storm of distinct-instance spawns through an always-failing
// factory (one stable fingerprint), and assert: (a) exactly one BreakerTripped
// event, (b) the breaker tracks exactly ONE signature for the whole storm, and
// (c) the suppression actually bites (later same-signature spawns return
// BreakerOpen), proving the storm was contained by a single breaker.
// ===========================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn fail_scenario_failure_fingerprint_storm_trips_one_breaker_for_the_class() {
    let (ledger, _drain) = ledger_pair();
    // Every create fails with the SAME message -> one stable fingerprint.
    let factory = Arc::new(ControllableFactory::new(
        FactoryBehavior::AlwaysFail,
        Duration::from_millis(2),
        ledger.clone(),
    ));
    let sink = Arc::new(RecordingSwarmSink::new());

    // Wide concurrency + huge lifetime budget so neither cap (not the breaker)
    // is what stops the storm — the breaker must be the thing that contains it.
    let budget = RunBudget::defaulted(32)
        .with_concurrency(32)
        .with_lifetime_spawns(10_000);
    let config = SwarmConfig::new(budget).with_breaker(BreakerConfig {
        failure_threshold: 5,
        cooldown: Duration::from_secs(60),
    });
    let coordinator = Arc::new(SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        config,
        factory,
        sink.clone(),
        ledger,
    ));

    // Storm: 40 DISTINCT instances all fail with the same signature, concurrently.
    let storm = 40u32;
    let mut js = JoinSet::new();
    for i in 0..storm {
        let c = coordinator.clone();
        js.spawn(async move {
            // Drive each instance to its terminal failure; ignore the typed error.
            let _ = c.spawn_session(spawn_req(instance(i))).await;
        });
    }
    while js.join_next().await.is_some() {}

    // (a) The breaker tripped EXACTLY ONCE for the whole storm — one trip event,
    // not one-per-failure and not one-per-instance.
    assert_eq!(
        sink.count_of(SwarmFrEventId::BreakerTripped),
        1,
        "a same-signature storm must trip the class breaker exactly once"
    );
    // (b) ONE breaker for the class: the signature map holds exactly one entry
    // regardless of how many distinct instances stormed (NOT N breakers).
    assert_eq!(
        coordinator.breaker_signature_count(),
        1,
        "the storm must be absorbed by a single class-keyed breaker, not N breakers"
    );
    // (c) The suppression bites: a fresh distinct instance carrying the same
    // signature is gated. We need its last-failure signature recorded first, so
    // spawn once (records the signature, returns BreakerOpen since the class is
    // already open), then confirm a follow-up is also BreakerOpen.
    let probe = instance(9_999);
    let first = coordinator.spawn_session(spawn_req(probe)).await;
    let second = coordinator.spawn_session(spawn_req(probe)).await;
    assert!(
        matches!(second, Err(SwarmError::BreakerOpen { .. })),
        "while the class breaker is open the same-signature spawn is suppressed, got {second:?} \
         (first probe: {first:?})"
    );
}

// ===========================================================================
// FAIL-SCENARIO (3): LEASE-REAPER MASS-RECLAIM. MANY time-boxed sessions all
// expire at (about) the same time. The single reaper must reclaim ALL of them
// with NO orphan: every START the factory recorded gets a matching STOP, so the
// ledger START count == STOP count and the registry is fully drained. This is
// the mass version of proof_3 (which reaped a single session).
// ===========================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn fail_scenario_lease_reaper_mass_reclaim_start_count_equals_stop_count() {
    let (ledger, drain_h) = ledger_pair();
    let store = Arc::new(InMemoryStore::default());
    let factory = Arc::new(ControllableFactory::new(
        FactoryBehavior::Succeed,
        Duration::from_millis(1),
        ledger.clone(),
    ));
    let sink = Arc::new(RecordingSwarmSink::new());

    // Wide concurrency so all sessions can be live at once; a long configured
    // lease so ONLY the per-spawn time_box drives expiry; a tight scan interval.
    let n = 24u32;
    let budget = RunBudget::defaulted(n as usize)
        .with_concurrency(n as usize)
        .with_lifetime_spawns(10_000);
    let config = SwarmConfig::new(budget)
        .with_lease_ttl(Duration::from_secs(60))
        .with_reaper_scan_interval(Duration::from_millis(20));
    let coordinator = SwarmCoordinator::new_legacy_without_dexterity_for_tests(
        config,
        factory,
        sink.clone(),
        ledger,
    );

    // Spawn ALL N time-boxed sessions FIRST (reaper not yet running) so every
    // session is genuinely live and has recorded its START before any reclaim —
    // a deterministic mass-expiry, not a spawn/reap race. Each is boxed to
    // ~120ms (comfortably longer than the spawn loop) so none expires mid-spawn.
    for i in 0..n {
        coordinator
            .spawn_session(spawn_req(instance(i)).with_time_box(Duration::from_millis(120)))
            .await
            .unwrap();
    }
    assert_eq!(coordinator.live_session_count(), n as usize);

    // NOW start the reaper: every session's box expires (about) together and the
    // single reaper must mass-reclaim them all.
    coordinator.start_reaper();
    // Wait past the box + several scan intervals so the reaper reclaims ALL.
    tokio::time::sleep(Duration::from_millis(500)).await;
    coordinator.stop_reaper();

    // The reaper reclaimed EVERY session — none orphaned in the registry.
    assert_eq!(
        coordinator.live_session_count(),
        0,
        "the reaper must mass-reclaim every expired time-boxed session"
    );
    assert!(sink.contains(SwarmFrEventId::LeaseExpired));
    // One LeaseExpired + one ResourceEvicted per reclaimed session.
    assert_eq!(
        sink.count_of(SwarmFrEventId::LeaseExpired),
        n as usize,
        "exactly one lease-expired event per reclaimed session"
    );

    // Ledger reconciliation: START count == STOP count, no orphan START.
    let rows = drain(&drain_h, store).await;
    let (starts, stops) = count_start_stop(&rows);
    assert_eq!(starts, n as usize, "one START per spawned session");
    assert_eq!(
        starts, stops,
        "mass reclaim must produce START==STOP (no orphan): {starts} starts, {stops} stops"
    );
}

// ===========================================================================
// FAIL-SCENARIO (4): OVERLOAD ESCALATION under saturation. When the local lane
// reports LocalOutcome::Overloaded (concurrency saturated / memory pressure),
// the routing policy escalates the task to CLOUD to relieve load — WITHOUT
// tripping the local breaker (overload is transient capacity, not a lane
// fault), so the local lane stays admissible afterward. A ForceLocal task under
// the same saturation must NOT escalate (data-residency blocks egress). This
// drives the real RoutingPolicy through a saturation-driven Overloaded storm.
// ===========================================================================
#[test]
fn fail_scenario_overload_escalates_to_cloud_without_tripping_local_breaker() {
    use super::routing::{
        CloudProvider, LocalOutcome, RoutingDecision, RoutingPolicy, RoutingRequest, TaskClass,
        TaskTier,
    };
    use std::time::Instant;

    let mut policy = RoutingPolicy::with_default();
    let now = Instant::now();

    let base = |class: TaskClass| {
        RoutingRequest::new(class, 100, 100)
            .with_local_model("tinyllama.safetensors")
            .with_cloud_model("claude-sonnet-4")
    };

    // Saturation storm: 16 routine tasks each report Overloaded from the local
    // lane. Every one must escalate to CLOUD (relieve load) AND must NOT charge
    // the local breaker (healthy lane, just saturated).
    let saturation = 16;
    for _ in 0..saturation {
        let decision = policy
            .route(
                &base(TaskClass::Routine).with_local_outcome(LocalOutcome::Overloaded),
                now,
            )
            .expect("overloaded routine escalates to cloud");
        assert_eq!(
            decision.tier(),
            TaskTier::Cloud,
            "an overloaded local lane must escalate the task to cloud to relieve load"
        );
        assert!(
            matches!(
                decision,
                RoutingDecision::Cloud {
                    provider: CloudProvider::Anthropic,
                    ..
                }
            ),
            "escalation targets the first-preference cloud provider"
        );
    }

    // The local breaker was NOT tripped by the overload storm: a fresh routine
    // task with NO prior outcome still routes LOCAL (the healthy lane is intact).
    // Had overload (wrongly) charged the breaker, 16 > threshold 5 would have
    // suppressed local and forced this to cloud.
    let healthy = policy
        .route(&base(TaskClass::Routine), now)
        .expect("local lane still admissible after overload storm");
    assert_eq!(
        healthy.tier(),
        TaskTier::Local,
        "overload must NOT trip the local breaker; the healthy local lane stays admissible"
    );

    // Data-residency: a ForceLocal task under the SAME saturation must NOT
    // escalate — ForceLocal blocks egress to cloud even when overloaded.
    let force_local = policy
        .route(
            &base(TaskClass::ForceLocal).with_local_outcome(LocalOutcome::Overloaded),
            now,
        )
        .expect("force-local task routes locally even under overload");
    assert_eq!(
        force_local.tier(),
        TaskTier::Local,
        "ForceLocal blocks overload escalation: no data egress to cloud under load"
    );
}

/// A factory that fails the FIRST create attempt for each distinct instance
/// (same fingerprint) and succeeds on every subsequent attempt. Used to
/// populate then prune the per-instance accounting maps (C5).
struct FailThenSucceedFactory {
    ledger: LedgerBatcher,
    seen: Mutex<std::collections::HashSet<ModelInstanceId>>,
    unloaded: Arc<AtomicUsize>,
    fail_message: String,
}

impl FailThenSucceedFactory {
    fn new(ledger: LedgerBatcher) -> Self {
        Self {
            ledger,
            seen: Mutex::new(std::collections::HashSet::new()),
            unloaded: Arc::new(AtomicUsize::new(0)),
            fail_message: "fail-then-succeed deterministic first-attempt failure".to_string(),
        }
    }
}

#[async_trait]
impl ModelSessionFactory for FailThenSucceedFactory {
    async fn create(&self, request: &SpawnRequest) -> Result<LiveSession, SwarmError> {
        tokio::time::sleep(Duration::from_millis(1)).await;
        let first_time = {
            let mut seen = self.seen.lock().unwrap();
            seen.insert(request.instance_id)
        };
        if first_time {
            return Err(SwarmError::FactoryFailed(self.fail_message.clone()));
        }
        let record_id = ProcessOwnershipRecordId::new_v7();
        let os_pid = 60000 + request.instance_id.instance;
        let start = ProcessStart::new(
            ProcessEngineKind::LlamaCpp,
            request.owner_role.clone(),
            request.owner_wp.clone(),
        )
        .with_process_uuid(record_id.as_uuid())
        .with_os_pid(os_pid)
        .with_parent_session_id(request.parent_session_id.clone());
        self.ledger
            .record_start(start)
            .map_err(|e| SwarmError::LedgerFailed(e.to_string()))?;

        let loaded = Arc::new(AtomicBool::new(false));
        let mut owned = ControllableWorker::new(self.unloaded.clone(), Arc::clone(&loaded));
        let model_id = owned
            .load(test_load_spec())
            .await
            .map_err(|e| SwarmError::FactoryFailed(e.to_string()))?;
        let owned = Arc::new(tokio::sync::Mutex::new(owned));
        let shared = ControllableWorker::new(self.unloaded.clone(), loaded);
        let teardown: super::factory::SessionTeardown = Arc::new(move || {
            let owned = Arc::clone(&owned);
            Box::pin(async move {
                owned
                    .lock()
                    .await
                    .unload(model_id)
                    .await
                    .map_err(|e| SwarmError::Internal(e.to_string()))
            })
        });
        Ok(LiveSession::new(
            Arc::new(shared),
            model_id,
            CancellationToken::new(),
            teardown,
            record_id,
            os_pid,
        ))
    }
}

/// A factory that fails its first `fail_first` creates (same fingerprint) then
/// succeeds once `allow_success` is set — used to prove the breaker heals via a
/// real success (C4).
struct HealableFactory {
    ledger: LedgerBatcher,
    fail_remaining: AtomicUsize,
    allow_success: AtomicBool,
    unloaded: Arc<AtomicUsize>,
    fail_message: String,
}

impl HealableFactory {
    fn new(ledger: LedgerBatcher, fail_first: usize) -> Self {
        Self {
            ledger,
            fail_remaining: AtomicUsize::new(fail_first),
            allow_success: AtomicBool::new(false),
            unloaded: Arc::new(AtomicUsize::new(0)),
            fail_message: "healable factory deterministic failure".to_string(),
        }
    }

    fn allow_success(&self) {
        self.allow_success.store(true, Ordering::SeqCst);
    }
}

#[async_trait]
impl ModelSessionFactory for HealableFactory {
    async fn create(&self, request: &SpawnRequest) -> Result<LiveSession, SwarmError> {
        tokio::time::sleep(Duration::from_millis(1)).await;
        let still_failing = self.fail_remaining.load(Ordering::SeqCst) > 0;
        if still_failing && !self.allow_success.load(Ordering::SeqCst) {
            self.fail_remaining.fetch_sub(1, Ordering::SeqCst);
            return Err(SwarmError::FactoryFailed(self.fail_message.clone()));
        }
        let record_id = ProcessOwnershipRecordId::new_v7();
        let os_pid = 50000 + request.instance_id.instance;
        let start = ProcessStart::new(
            ProcessEngineKind::LlamaCpp,
            request.owner_role.clone(),
            request.owner_wp.clone(),
        )
        .with_process_uuid(record_id.as_uuid())
        .with_os_pid(os_pid)
        .with_parent_session_id(request.parent_session_id.clone());
        self.ledger
            .record_start(start)
            .map_err(|e| SwarmError::LedgerFailed(e.to_string()))?;

        let loaded = Arc::new(AtomicBool::new(false));
        let mut owned = ControllableWorker::new(self.unloaded.clone(), Arc::clone(&loaded));
        let model_id = owned
            .load(test_load_spec())
            .await
            .map_err(|e| SwarmError::FactoryFailed(e.to_string()))?;
        let owned = Arc::new(tokio::sync::Mutex::new(owned));
        let shared = ControllableWorker::new(self.unloaded.clone(), loaded);
        let teardown: super::factory::SessionTeardown = Arc::new(move || {
            let owned = Arc::clone(&owned);
            Box::pin(async move {
                owned
                    .lock()
                    .await
                    .unload(model_id)
                    .await
                    .map_err(|e| SwarmError::Internal(e.to_string()))
            })
        });
        Ok(LiveSession::new(
            Arc::new(shared),
            model_id,
            CancellationToken::new(),
            teardown,
            record_id,
            os_pid,
        ))
    }
}

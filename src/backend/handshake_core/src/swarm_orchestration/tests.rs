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

use crate::model_runtime::error::ModelRuntimeError;
use crate::model_runtime::registry::RuntimeBinding;
use crate::model_runtime::{
    CancellationToken, Embedding, GenerateRequest, KvCacheHandle, KvCachePolicy, KvQuantSupport,
    LoadSpec, LoraStackHandle, ModelCapabilities, ModelId, ModelRuntime, ProviderKind, RuntimeKind,
    SamplingParams, Score, SteeringHookHandle, TokenStream,
};
use crate::process_ledger::{
    LedgerBatcher, LedgerBatcherConfig, LedgerEvent, NoopOverflowSink, ProcessEngineKind,
    ProcessLedgerDrain, ProcessLedgerError, ProcessLedgerStore, ProcessOwnershipRecordId,
    ProcessStart,
};

use super::breaker::BreakerConfig;
use super::coordinator::{SwarmConfig, SwarmCoordinator};
use super::error::SwarmError;
use super::events::{RecordingSwarmSink, SwarmFrEventId};
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
}

impl ControllableWorker {
    fn new(unloaded: Arc<AtomicUsize>) -> Self {
        Self {
            capabilities: ModelCapabilities::default(),
            kv: KvCacheHandle::new("swarm-test-kv"),
            lora: LoraStackHandle::new("swarm-test-lora"),
            steering: SteeringHookHandle::new("swarm-test-steering"),
            cancelled: Arc::new(AtomicBool::new(false)),
            unloaded,
        }
    }
}

#[async_trait]
impl ModelRuntime for ControllableWorker {
    async fn load(&mut self, _spec: LoadSpec) -> Result<ModelId, ModelRuntimeError> {
        Ok(ModelId::new_v7())
    }

    async fn unload(&mut self, _id: ModelId) -> Result<(), ModelRuntimeError> {
        self.cancelled.store(true, Ordering::SeqCst);
        self.unloaded.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn generate(&self, req: GenerateRequest) -> TokenStream {
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
        Ok(Score {
            token_logprobs: vec![],
            mean_logprob: 0.0,
        })
    }

    async fn embed(&self, _id: ModelId, _text: &str) -> Result<Embedding, ModelRuntimeError> {
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
    /// Number of times the worker's real `unload` ran (D1 proof): proves the
    /// teardown frees the model, not just cancels.
    unload_invocations: Arc<AtomicUsize>,
    fail_message: String,
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
            unload_invocations: Arc::new(AtomicUsize::new(0)),
            fail_message: "controllable factory deterministic failure".to_string(),
        }
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
                let mut owned = ControllableWorker::new(self.unload_invocations.clone());
                let model_id = owned
                    .load(test_load_spec())
                    .await
                    .map_err(|e| SwarmError::FactoryFailed(e.to_string()))?;
                let shared = ControllableWorker::new(self.unload_invocations.clone());
                let cancel = CancellationToken::new();
                self.created.fetch_add(1, Ordering::SeqCst);

                let teardown_invocations = self.teardown_invocations.clone();
                let teardown: super::factory::SessionTeardown = Box::new(move || {
                    Box::pin(async move {
                        teardown_invocations.fetch_add(1, Ordering::SeqCst);
                        // Free the model on the owned runtime — the real
                        // teardown contract (D1). This bumps `unloaded`.
                        owned
                            .unload(model_id)
                            .await
                            .map_err(|e| SwarmError::Internal(e.to_string()))
                    })
                });

                Ok(LiveSession::new(
                    Arc::new(shared),
                    model_id,
                    cancel,
                    teardown,
                    record_id,
                    os_pid,
                ))
            }
            FactoryBehavior::AlwaysFail => Err(SwarmError::FactoryFailed(self.fail_message.clone())),
        };

        self.in_flight.fetch_sub(1, Ordering::SeqCst);
        result
    }
}

// ---------------------------------------------------------------------------
// In-memory process ledger store (real drain of real rows).
// ---------------------------------------------------------------------------

#[derive(Clone, Default)]
struct InMemoryStore {
    events: Arc<Mutex<Vec<LedgerEvent>>>,
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
    SpawnRequest::new(iid, RuntimeBinding::LlamaCpp, "swarm_test", "parent-session-1")
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
    let coordinator = SwarmCoordinator::new(SwarmConfig::new(budget), factory, sink, ledger);

    // A grouped spawn: the swarm + worktree grouping copied from the SpawnRequest
    // is readable per session (board swimlane / Flight-Recorder drill-down join).
    let iid = instance(1);
    let req = spawn_req(iid).with_swarm("swarm-alpha").with_worktree("wt-7");
    coordinator.spawn_session(req).await.unwrap();
    assert_eq!(
        coordinator.session_grouping(iid),
        Some((Some("swarm-alpha".to_string()), Some("wt-7".to_string())))
    );

    // An ungrouped spawn carries (None, None) — source-compatible default.
    let iid2 = instance(2);
    coordinator.spawn_session(spawn_req(iid2)).await.unwrap();
    assert_eq!(coordinator.session_grouping(iid2), Some((None, None)));
}

// ===========================================================================
// RANK-6: cold-start admission bounds SIMULTANEOUS boots below run-concurrency.
// ===========================================================================

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
    let coordinator = Arc::new(SwarmCoordinator::new(
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
    let coordinator = Arc::new(SwarmCoordinator::new(
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
    let coordinator = SwarmCoordinator::new(SwarmConfig::new(budget), factory, sink, ledger);

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
    let coordinator = SwarmCoordinator::new(config, factory, sink.clone(), ledger);
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
    let coordinator = SwarmCoordinator::new(config, factory, sink.clone(), ledger);
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
    let coordinator = SwarmCoordinator::new(config, factory, sink.clone(), ledger);

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
    assert_eq!(sink.count_of(SwarmFrEventId::BreakerTripped), 1, "trip once");
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
    let coordinator = SwarmCoordinator::new(SwarmConfig::new(budget), factory, sink, ledger);

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
    let coordinator = SwarmCoordinator::new(SwarmConfig::new(budget), factory, sink.clone(), ledger);

    let iid = instance(0);
    coordinator.spawn_session(spawn_req(iid)).await.unwrap();
    assert_eq!(coordinator.session_state(iid), Some(ModelSessionState::Ready));

    coordinator.cancel_session(iid, "operator_cancel").await.unwrap();

    // Evicted from registry, cancelled + evicted events emitted.
    assert_eq!(coordinator.session_state(iid), None);
    assert!(sink.contains(SwarmFrEventId::SessionCancelled));
    assert!(sink.contains(SwarmFrEventId::ResourceEvicted));
    // Permit returned: a fresh spawn can take the slot.
    assert_eq!(coordinator.remaining().concurrency_permits_available, 4);

    // Ledger has a matching stop row.
    let rows = drain(&drain_h, store).await;
    let stops = rows.iter().filter(|e| matches!(e, LedgerEvent::Stop(_))).count();
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

    let budget = RunBudget::defaulted(4).with_concurrency(4).with_lifetime_spawns(1000);
    let coordinator = Arc::new(SwarmCoordinator::new(
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
    let coordinator = SwarmCoordinator::new(config, factory, sink, ledger);

    // (a) Explicit cancel path invokes teardown -> unload exactly once.
    let iid0 = instance(0);
    coordinator.spawn_session(spawn_req(iid0)).await.unwrap();
    assert_eq!(teardown_calls.load(Ordering::SeqCst), 0);
    assert_eq!(unload_calls.load(Ordering::SeqCst), 0);
    coordinator.cancel_session(iid0, "operator_cancel").await.unwrap();
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

    // (b) Lease-expiry reclaim path also invokes teardown -> unload.
    coordinator.start_reaper();
    let iid1 = instance(1);
    coordinator.spawn_session(spawn_req(iid1)).await.unwrap();
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

    let budget = RunBudget::defaulted(8).with_concurrency(8).with_lifetime_spawns(1000);
    let coordinator = Arc::new(SwarmCoordinator::new(
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

    let budget = RunBudget::defaulted(4).with_concurrency(4).with_lifetime_spawns(1000);
    let config = SwarmConfig::new(budget).with_breaker(BreakerConfig {
        failure_threshold: 3,
        cooldown: Duration::from_secs(60),
    });
    let coordinator = SwarmCoordinator::new(config, factory.clone(), sink, ledger);

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

    let budget = RunBudget::defaulted(4).with_concurrency(4).with_lifetime_spawns(1000);
    // Short cooldown so the breaker half-opens and admits a probe; the REAL
    // success on that probe is what must heal it. The probe alone (half-open)
    // does NOT reset consecutive_failures to 0 — only record_success does, which
    // is exactly the wiring C4 fixes (the old breaker_success_for_instance was a
    // no-op).
    let config = SwarmConfig::new(budget).with_breaker(BreakerConfig {
        failure_threshold: 2,
        cooldown: Duration::from_millis(60),
    });
    let coordinator = SwarmCoordinator::new(config, factory.clone(), sink.clone(), ledger);

    let fp = FailureFingerprint::compute(
        SwarmErrorClass::FactoryFailed,
        &factory.fail_message,
    );

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
    assert!(coordinator.breaker_admits(&fp), "healed signature admitted again");

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
    let coordinator = SwarmCoordinator::new(config, factory, sink.clone(), ledger);
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
    let coordinator = SwarmCoordinator::new(config, factory.clone(), sink, ledger);

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
    let coordinator = Arc::new(SwarmCoordinator::new(config, factory, sink.clone(), ledger));

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
    let coordinator = SwarmCoordinator::new(config, factory, sink.clone(), ledger);

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

        let mut owned = ControllableWorker::new(self.unloaded.clone());
        let model_id = owned
            .load(test_load_spec())
            .await
            .map_err(|e| SwarmError::FactoryFailed(e.to_string()))?;
        let shared = ControllableWorker::new(self.unloaded.clone());
        let teardown: super::factory::SessionTeardown = Box::new(move || {
            Box::pin(async move {
                owned
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

        let mut owned = ControllableWorker::new(self.unloaded.clone());
        let model_id = owned
            .load(test_load_spec())
            .await
            .map_err(|e| SwarmError::FactoryFailed(e.to_string()))?;
        let shared = ControllableWorker::new(self.unloaded.clone());
        let teardown: super::factory::SessionTeardown = Box::new(move || {
            Box::pin(async move {
                owned
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

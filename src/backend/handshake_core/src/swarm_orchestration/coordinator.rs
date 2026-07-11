//! The load-bearing [`SwarmCoordinator`].
//!
//! Owns the single spawn path, the session registry, the two-level fan-out
//! bound (concurrency semaphore + monotonic lifetime ceiling), the
//! claim-lease/TTL/reaper machinery, the failure-fingerprint circuit breaker,
//! and the budget-as-data ledger. It is generic over an injected
//! [`ModelSessionFactory`] and a [`SwarmEventSink`] so it carries no
//! candle/llama specifics and is fully exercisable with a real controllable
//! worker adapter in tests.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{DateTime, Utc};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::task::JoinHandle;

use crate::model_runtime::{
    CancellationToken, GenerateRequest, ModelId, ProviderKind, TokenStream,
};
use crate::process_ledger::{
    LedgerBatcher, ProcessEngineKind, ProcessOwnershipRecordId, ProcessStop, ReclaimTrigger,
};

use super::breaker::{AdmitDecision, BreakerConfig, FailureFingerprint, FailureFingerprintBreaker};
use super::error::{SwarmError, SwarmResult};
use super::events::{SwarmEvent, SwarmEventSink};
use super::factory::{LiveSession, ModelSessionFactory, SessionTeardown};
use super::ids::{BudgetRemaining, ModelInstanceId, RunBudget, SpawnRequest};
use super::model_lane::{
    build_failed_launch_records, build_successful_launch_records, DexterityLaunchAdapterKind,
    DexterityLaunchAdapterRegistry, DexterityLaunchAdapterRequest,
    ModelLaneDownstreamContextBundle, ModelLaneRecord, ModelLaneRunRecord, ModelLaneStatus,
    ModelLaneStore, RuntimeBinding as ModelLaneRuntimeBinding,
};
use super::state::ModelSessionState;
use crate::kernel::{KernelActor, ModelAdapter, ModelAdapterOutput, ModelAdapterRequest};

/// Max number of times a single instance may be respawned by the reaper before
/// it is given up on, so a flapping session cannot storm respawns.
pub const DEFAULT_MAX_RESPAWNS_PER_INSTANCE: u32 = 3;

/// Configuration for a coordinator run.
#[derive(Clone, Debug)]
pub struct SwarmConfig {
    pub budget: RunBudget,
    /// Default claim-lease TTL. The reaper reclaims sessions whose lease has
    /// expired past this without a renewal.
    pub lease_ttl: Duration,
    /// How often the reaper scans for expired leases.
    pub reaper_scan_interval: Duration,
    pub breaker: BreakerConfig,
    pub max_respawns_per_instance: u32,
    /// owner_role stamped on process-ledger rows the coordinator writes.
    pub owner_role: String,
}

impl SwarmConfig {
    pub fn new(budget: RunBudget) -> Self {
        Self {
            budget,
            lease_ttl: Duration::from_secs(300),
            reaper_scan_interval: Duration::from_secs(5),
            breaker: BreakerConfig::default(),
            max_respawns_per_instance: DEFAULT_MAX_RESPAWNS_PER_INSTANCE,
            owner_role: "swarm_coordinator".to_string(),
        }
    }

    pub fn with_lease_ttl(mut self, ttl: Duration) -> Self {
        self.lease_ttl = ttl;
        self
    }

    pub fn with_reaper_scan_interval(mut self, interval: Duration) -> Self {
        self.reaper_scan_interval = interval;
        self
    }

    pub fn with_breaker(mut self, breaker: BreakerConfig) -> Self {
        self.breaker = breaker;
        self
    }

    pub fn with_max_respawns(mut self, max: u32) -> Self {
        self.max_respawns_per_instance = max;
        self
    }
}

/// A claim lease held by a live session. The reaper reclaims a session whose
/// `expires_at` is in the past.
#[derive(Clone, Debug)]
pub struct ClaimLease {
    pub instance_id: ModelInstanceId,
    pub owner: String,
    pub expires_at: DateTime<Utc>,
}

impl ClaimLease {
    fn is_expired(&self, now: DateTime<Utc>) -> bool {
        now >= self.expires_at
    }
}

/// One entry in the session registry. Holds everything needed to drive,
/// observe, cancel, and ledger-stop a live session.
pub struct SessionHandle {
    pub instance_id: ModelInstanceId,
    pub state: ModelSessionState,
    pub lease: ClaimLease,
    pub cancel: CancellationToken,
    /// The `ModelId` the factory's load returned. Retained so teardown can free
    /// exactly this model from a shared runtime (D1).
    pub model_id: ModelId,
    pub process_record_id: ProcessOwnershipRecordId,
    pub os_pid: u32,
    pub process_engine_kind: ProcessEngineKind,
    pub parent_session_id: String,
    pub runtime: Arc<dyn crate::model_runtime::ModelRuntime>,
    /// Single-shot async teardown that frees the engine resource. Taken
    /// (`Option::take`) on the terminal path so it runs exactly once (D1).
    teardown: Option<SessionTeardown>,
    /// Held for the lifetime of the slot; dropping it returns the permit.
    permit: Option<OwnedSemaphorePermit>,
    started_at: DateTime<Utc>,
    /// Board/lineage grouping (rank-2): the swarm + VM/sandbox worktree this
    /// session belongs to, copied from the SpawnRequest. Lets the operator board
    /// group sessions into swimlanes, the ledger STOP metadata record the
    /// grouping, and the Flight Recorder drill down by swarm/worktree. `None`
    /// when the session is ungrouped/ad-hoc.
    swarm_id: Option<String>,
    worktree_id: Option<String>,
    /// Rank-6 committed-memory bytes reserved for this live session. Released
    /// only after terminal teardown, so new sessions are not admitted while a
    /// cancelling VM/model still owns memory.
    committed_memory_bytes: u64,
    /// Dexterity sessions may become Ready only after ModelLane + EventLedger
    /// persistence commits. Non-Dexterity sessions do not use this gate.
    dexterity_model_lane_persisted: bool,
    dexterity_lane_id: Option<String>,
}

impl SessionHandle {
    pub fn state(&self) -> ModelSessionState {
        self.state
    }

    /// The swarm this session is grouped under (board swimlane / per-swarm scope).
    pub fn swarm_id(&self) -> Option<&str> {
        self.swarm_id.as_deref()
    }

    /// The VM/sandbox worktree this session runs inside (per-worktree recovery).
    pub fn worktree_id(&self) -> Option<&str> {
        self.worktree_id.as_deref()
    }
}

/// Shared, lock-guarded inner state. Split out so the reaper task can hold an
/// `Arc` to exactly this without owning the whole coordinator.
struct Inner {
    registry: Mutex<HashMap<ModelInstanceId, SessionHandle>>,
    breaker: Mutex<FailureFingerprintBreaker>,
    /// Per-instance respawn counters (anti-storm) + budget accounting.
    accounting: Mutex<Accounting>,
    semaphore: Arc<Semaphore>,
    /// rank-6 admission: bounds SIMULTANEOUS cold starts (factory.create / boot)
    /// separately from `semaphore` (run-concurrency). Held only during boot, so a
    /// burst of admitted spawns does not stampede the boot/networking layer.
    cold_start_semaphore: Arc<Semaphore>,
    /// Monotonic lifetime spawn counter — never decremented. The hard ceiling
    /// is `budget.max_lifetime_spawns` (HBR-SWARM-002 loop-cap semantics).
    lifetime_spawns: AtomicU64,
    config: SwarmConfig,
    factory: Arc<dyn ModelSessionFactory>,
    sink: Arc<dyn SwarmEventSink>,
    ledger: LedgerBatcher,
    model_lane_store: Option<ModelLaneStore>,
    dexterity_launch_required: bool,
}

#[derive(Default)]
struct Accounting {
    respawns: HashMap<ModelInstanceId, u32>,
    /// Last failure signature observed for an instance. Drives the breaker
    /// ADMISSION gate (D3): before paying the factory cost on a respawn we ask
    /// the breaker whether this instance's most recent signature is suppressed.
    /// Also the key the success path heals on (C4). Pruned on terminal eviction
    /// so it cannot grow unbounded (C5).
    last_failure_signature: HashMap<ModelInstanceId, FailureFingerprint>,
    tokens_used: u64,
    cost_micros_used: u64,
    committed_memory_bytes_used: u64,
}

/// Cancellation-safe committed-memory reservation. Async spawn admission crosses
/// `await` points before a `SessionHandle` exists; dropping this guard releases
/// the reservation unless ownership was transferred into the registry.
struct CommittedMemoryReservation {
    inner: Arc<Inner>,
    bytes: u64,
    armed: bool,
}

impl CommittedMemoryReservation {
    fn reserve(inner: Arc<Inner>, bytes: u64) -> Result<Self, String> {
        inner.try_reserve_committed_memory(bytes)?;
        Ok(Self {
            inner,
            bytes,
            armed: true,
        })
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for CommittedMemoryReservation {
    fn drop(&mut self) {
        if self.armed {
            self.inner.release_committed_memory(self.bytes);
            self.armed = false;
        }
    }
}

/// Cancellation-safe lifetime-spawn reservation. Successful registry insertion
/// consumes the lifetime budget; every rollback path, including task abort,
/// returns it.
struct LifetimeSpawnReservation {
    inner: Arc<Inner>,
    reserved: u64,
    armed: bool,
}

impl LifetimeSpawnReservation {
    fn reserve(inner: Arc<Inner>) -> Self {
        let reserved = inner.lifetime_spawns.fetch_add(1, Ordering::SeqCst) + 1;
        Self {
            inner,
            reserved,
            armed: true,
        }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for LifetimeSpawnReservation {
    fn drop(&mut self) {
        if self.armed {
            self.inner.lifetime_spawns.fetch_sub(1, Ordering::SeqCst);
            self.armed = false;
        }
    }
}

/// The coordinator handle the operator/scheduler holds.
pub struct SwarmCoordinator {
    inner: Arc<Inner>,
    reaper: Mutex<Option<JoinHandle<()>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DexterityNoOsLaunchCaller {
    caller_session: String,
    adapter_kind: DexterityLaunchAdapterKind,
    run_id: String,
    lane_id: String,
    authority_instance_id: ModelInstanceId,
    capability_receipt_ref: String,
}

fn reserves_host_committed_memory(request: &SpawnRequest) -> bool {
    matches!(request.provider, None | Some(ProviderKind::Local))
}

impl SwarmCoordinator {
    /// Build a fail-closed coordinator. `factory`, `sink`, and `ledger` are
    /// injected so the same coordinator code runs in production and tests, but
    /// runtime creation still requires a Dexterity launch contract. With no
    /// ModelLaneStore wired, any Dexterity launch also fails before factory
    /// creation; callers that need runnable model launches should use
    /// [`Self::new_with_model_lane_store`].
    pub fn new(
        config: SwarmConfig,
        factory: Arc<dyn ModelSessionFactory>,
        sink: Arc<dyn SwarmEventSink>,
        ledger: LedgerBatcher,
    ) -> Self {
        Self::new_with_dexterity_launch_requirement(config, factory, sink, ledger, None, true)
    }

    #[cfg(any(test, feature = "test-utils"))]
    pub fn new_legacy_without_dexterity_for_tests(
        config: SwarmConfig,
        factory: Arc<dyn ModelSessionFactory>,
        sink: Arc<dyn SwarmEventSink>,
        ledger: LedgerBatcher,
    ) -> Self {
        Self::new_with_dexterity_launch_requirement(config, factory, sink, ledger, None, false)
    }

    pub fn new_with_model_lane_store(
        config: SwarmConfig,
        factory: Arc<dyn ModelSessionFactory>,
        sink: Arc<dyn SwarmEventSink>,
        ledger: LedgerBatcher,
        model_lane_store: ModelLaneStore,
    ) -> Self {
        Self::new_with_dexterity_launch_requirement(
            config,
            factory,
            sink,
            ledger,
            Some(model_lane_store),
            true,
        )
    }

    fn new_with_dexterity_launch_requirement(
        config: SwarmConfig,
        factory: Arc<dyn ModelSessionFactory>,
        sink: Arc<dyn SwarmEventSink>,
        ledger: LedgerBatcher,
        model_lane_store: Option<ModelLaneStore>,
        dexterity_launch_required: bool,
    ) -> Self {
        let semaphore = Arc::new(Semaphore::new(config.budget.max_concurrent.max(1)));
        let cold_start_semaphore = Arc::new(Semaphore::new(
            config.budget.max_concurrent_cold_starts.max(1),
        ));
        let breaker = FailureFingerprintBreaker::new(config.breaker);
        let inner = Arc::new(Inner {
            registry: Mutex::new(HashMap::new()),
            breaker: Mutex::new(breaker),
            accounting: Mutex::new(Accounting::default()),
            semaphore,
            cold_start_semaphore,
            lifetime_spawns: AtomicU64::new(0),
            config,
            factory,
            sink,
            ledger,
            model_lane_store,
            dexterity_launch_required,
        });
        Self {
            inner,
            reaper: Mutex::new(None),
        }
    }

    /// Start the single background reaper task. Idempotent: a second call is a
    /// no-op while a reaper is already running.
    pub fn start_reaper(&self) {
        let mut guard = self.reaper.lock().expect("reaper lock poisoned");
        if guard.is_some() {
            return;
        }
        let inner = self.inner.clone();
        let interval = inner.config.reaper_scan_interval;
        let handle = tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval.max(Duration::from_millis(1)));
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                ticker.tick().await;
                reap_expired(&inner).await;
            }
        });
        *guard = Some(handle);
    }

    /// Stop the reaper (used on shutdown / in tests).
    pub fn stop_reaper(&self) {
        if let Some(handle) = self.reaper.lock().expect("reaper lock poisoned").take() {
            handle.abort();
        }
    }

    /// THE single spawn entrypoint. Enforces, in order: breaker ADMISSION gate
    /// -> budget (token/cost) -> lifetime spawn ceiling -> concurrency permit ->
    /// factory create -> atomic dedup + registry insert + lease + events. Any
    /// bound failure returns a typed error and the partially-acquired resources
    /// are released.
    pub async fn spawn_session(&self, request: SpawnRequest) -> SwarmResult<ModelInstanceId> {
        let inner = Arc::clone(&self.inner);
        let instance_id = request.instance_id;

        // (0a) Duplicate pre-check (best-effort, fast path). The authoritative
        // dedup is the atomic check-and-insert under the registry lock after a
        // successful create (D2); this early check just avoids paying the
        // factory cost in the common, uncontended case.
        {
            let registry = inner.registry.lock().expect("registry poisoned");
            if let Some(existing) = registry.get(&instance_id) {
                if !existing.state.is_terminal() {
                    return Err(SwarmError::DuplicateInstance(instance_id));
                }
            }
        }

        // (0b) Breaker ADMISSION gate (D3). BEFORE any expensive work (permit +
        // factory load) we ask the breaker whether this instance's most recent
        // failure signature is currently suppressed. If so, return BreakerOpen
        // immediately — an Open breaker must not pay the full load every time.
        if let Some(fp) = inner.last_failure_signature_for(instance_id) {
            let admit = {
                let mut breaker = inner.breaker.lock().expect("breaker poisoned");
                breaker.admit(&fp, std::time::Instant::now())
            };
            if let AdmitDecision::Suppress {
                cooldown_remaining_ms,
            } = admit
            {
                self.emit_spawn_rejected(instance_id, "breaker_open");
                return Err(SwarmError::BreakerOpen {
                    signature: fp.to_string(),
                    cooldown_remaining_ms,
                });
            }
        }

        // (1) Budget: token/cost ceilings exhausted? These are global run
        // ceilings. Committed memory is provider-aware and checked below so
        // cloud lanes can still escape saturated local host memory.
        if let Some(dimension) = inner.exhausted_global_budget_dimension() {
            self.emit_spawn_rejected(instance_id, &format!("budget:{dimension}"));
            return Err(SwarmError::BudgetExhausted { dimension });
        }
        let committed_memory_bytes = if !reserves_host_committed_memory(&request) {
            0
        } else if inner.config.budget.max_committed_memory_bytes.is_some() {
            match request.committed_memory_bytes {
                Some(bytes) => bytes,
                None => {
                    let dimension = "committed_memory_unestimated".to_string();
                    self.emit_spawn_rejected(instance_id, &format!("budget:{dimension}"));
                    return Err(SwarmError::BudgetExhausted { dimension });
                }
            }
        } else {
            request.committed_memory_bytes.unwrap_or(0)
        };
        let mut committed_memory_reservation =
            match CommittedMemoryReservation::reserve(Arc::clone(&inner), committed_memory_bytes) {
                Ok(reservation) => reservation,
                Err(dimension) => {
                    self.emit_spawn_rejected(instance_id, &format!("budget:{dimension}"));
                    return Err(SwarmError::BudgetExhausted { dimension });
                }
            };

        // (2) Lifetime spawn ceiling (monotonic, HBR-SWARM-002 semantics).
        // Reserve a slot atomically; roll back if anything downstream fails so
        // a rejected spawn does not permanently consume budget.
        let mut lifetime_reservation = LifetimeSpawnReservation::reserve(Arc::clone(&inner));
        let reserved = lifetime_reservation.reserved;
        let ceiling = inner.config.budget.max_lifetime_spawns;
        if reserved > ceiling {
            self.emit_spawn_rejected(instance_id, "lifetime_ceiling");
            return Err(SwarmError::LifetimeSpawnCeilingReached {
                spawned: reserved.saturating_sub(1),
                ceiling,
            });
        }

        // (3) Concurrency permit. try_acquire so an over-cap spawn returns a
        // typed error immediately rather than blocking forever.
        let permit = match inner.semaphore.clone().try_acquire_owned() {
            Ok(p) => p,
            Err(_) => {
                let cap = inner.config.budget.max_concurrent;
                self.emit_spawn_rejected(instance_id, "concurrency_cap");
                return Err(SwarmError::ConcurrencyCapReached {
                    in_flight: cap.saturating_sub(inner.semaphore.available_permits()),
                    cap,
                });
            }
        };

        // (3b) Cold-start admission: bound SIMULTANEOUS boots separately from
        // run-concurrency. An admitted spawn waits here for a boot slot, then
        // releases it the instant boot completes (the permit is scoped to the
        // create() call), so a RUNNING session never holds a boot slot. The
        // boot/networking layer (TAP/CNI) is the scale wall, not the running count.
        // (4) Factory create — the real (or controllable) load. The factory
        // records the process-ledger START (C7): it owns the START so that any
        // factory failure stops its own START before returning, leaving no
        // orphan. The coordinator owns the STOP symmetrically on every terminal
        // path. (See `terminate` / `reap_expired`.)
        match (
            inner.dexterity_launch_required,
            inner.model_lane_store.is_some(),
            request.dexterity_launch.is_some(),
        ) {
            (true, _, false) => {
                drop(permit);
                return Err(SwarmError::LedgerFailed(
                    "ModelLaneStore-backed coordinators require \
                     SpawnRequest::with_dexterity_launch before runtime creation"
                        .into(),
                ));
            }
            (_, false, true) => {
                drop(permit);
                return Err(SwarmError::LedgerFailed(
                    "Dexterity launch contract requires ModelLaneStore before runtime creation"
                        .into(),
                ));
            }
            _ => {}
        }

        if request.dexterity_launch.is_some() {
            if request.provider == Some(crate::model_runtime::ProviderKind::ByokCloud) {
                let Some(store) = inner.model_lane_store.as_ref() else {
                    drop(permit);
                    return Err(SwarmError::LedgerFailed(
                        "Dexterity cloud launch consent preflight requires ModelLaneStore".into(),
                    ));
                };
                if let Err(err) = store.preflight_cloud_spawn_request(&request).await {
                    drop(permit);
                    return Err(SwarmError::LedgerFailed(format!(
                        "Dexterity cloud consent preflight failed: {err}"
                    )));
                }
            }
            if let Err(err) =
                DexterityLaunchAdapterRegistry::standard().preflight_spawn_request(&request)
            {
                drop(permit);
                return Err(SwarmError::LedgerFailed(format!(
                    "Dexterity launch preflight failed: {err}"
                )));
            }
        }
        let create_result = {
            let _boot_permit = inner
                .cold_start_semaphore
                .clone()
                .acquire_owned()
                .await
                .expect("cold-start semaphore is never closed");
            inner.factory.create(&request).await
        };
        match create_result {
            Ok(live) => {
                let dexterity_launch_records = if request.dexterity_launch.is_some() {
                    match build_successful_launch_records(&request, &live) {
                        Ok(records) => Some(records),
                        Err(err) => {
                            drop(permit);
                            self.teardown_orphan(&request, live).await;
                            return Err(SwarmError::LedgerFailed(format!(
                                "Dexterity launch record preparation failed: {err}"
                            )));
                        }
                    }
                } else {
                    None
                };
                // (4a) ATOMIC dedup + insert (D2). Hold the registry lock across
                // BOTH the duplicate check and the insert so two concurrent
                // spawns of the same instance_id cannot both record a START and
                // silently drop the first handle. If a live instance already
                // exists, roll back this spawn's permit + lifetime reservation
                // AND fully tear down the just-created session (cancel +
                // teardown + ledger STOP) so the loser leaves no orphan START.
                let inserted =
                    self.try_insert_loading(&request, live, permit, committed_memory_bytes);
                match inserted {
                    Ok(()) => {
                        if let Some(records) = dexterity_launch_records {
                            let Some(store) = inner.model_lane_store.as_ref() else {
                                let _ = self
                                    .terminate(
                                        instance_id,
                                        ModelSessionState::Cancelled,
                                        "dexterity_model_lane_store_missing",
                                        -1,
                                    )
                                    .await;
                                return Err(SwarmError::LedgerFailed(
                                    "Dexterity launch contract requires ModelLaneStore".into(),
                                ));
                            };
                            if let Err(err) = store.record_prepared_launch(records).await {
                                let _ = self
                                    .terminate(
                                        instance_id,
                                        ModelSessionState::Cancelled,
                                        "dexterity_model_lane_record_failed",
                                        -1,
                                    )
                                    .await;
                                return Err(SwarmError::LedgerFailed(format!(
                                    "Dexterity launch record failed: {err}"
                                )));
                            }
                            if let Err(err) = self.mark_dexterity_model_lane_persisted(instance_id)
                            {
                                let _ = self
                                    .terminate(
                                        instance_id,
                                        ModelSessionState::Cancelled,
                                        "dexterity_model_lane_persist_marker_failed",
                                        -1,
                                    )
                                    .await;
                                return Err(err);
                            }
                        }
                        if let Err(err) = self.transition(instance_id, ModelSessionState::Ready) {
                            let _ = self
                                .terminate(
                                    instance_id,
                                    ModelSessionState::Cancelled,
                                    "dexterity_ready_transition_failed",
                                    -1,
                                )
                                .await;
                            return Err(err);
                        }
                        lifetime_reservation.disarm();
                        committed_memory_reservation.disarm();
                        // Heal the breaker for this instance's tracked signature
                        // on a real success (C4). A signature that tripped
                        // recovers after a genuine success, not only cooldown.
                        inner.heal_breaker_for_instance(instance_id);
                        let permits_cap = inner.config.budget.max_concurrent;
                        let permits_in_use =
                            permits_cap.saturating_sub(inner.semaphore.available_permits());
                        inner.sink.emit(SwarmEvent::ResourceAllocated {
                            instance_id,
                            permits_in_use,
                            permits_cap,
                        });
                        Ok(instance_id)
                    }
                    Err((live, permit)) => {
                        // Lost the race: an instance is already live. Release
                        // permit + lifetime reservation and tear down the loser.
                        drop(permit);
                        self.teardown_orphan(&request, live).await;
                        Err(SwarmError::DuplicateInstance(instance_id))
                    }
                }
            }
            Err(err) => {
                // Release the permit + roll back the lifetime reservation: a
                // failed spawn must not leak a slot. The factory is contracted
                // (C7) to have stopped any START it recorded before failing, so
                // there is no orphan START to clean up here.
                drop(permit);

                if request.dexterity_launch.is_some() {
                    if let Some(store) = inner.model_lane_store.as_ref() {
                        let records = build_failed_launch_records(&request, &err).map_err(
                            |record_err| {
                                SwarmError::LedgerFailed(format!(
                                    "Dexterity failed launch record preparation failed after {err}: {record_err}"
                                ))
                            },
                        )?;
                        if let Err(record_err) = store.record_prepared_launch(records).await {
                            return Err(SwarmError::LedgerFailed(format!(
                                "Dexterity failed launch record failed after {err}: {record_err}"
                            )));
                        }
                    }
                }

                // Feed the failure-fingerprint breaker. Only genuine failures
                // (not capacity refusals) accrue. Track the signature per
                // instance so the ADMISSION gate (D3) can suppress the next
                // respawn before it pays the load.
                if !err.is_capacity_refusal() {
                    let fp = FailureFingerprint::compute(err.class(), err.detail());
                    inner.record_instance_signature(instance_id, fp.clone());
                    let now = std::time::Instant::now();
                    let (tripped, consecutive) = {
                        let mut breaker = inner.breaker.lock().expect("breaker poisoned");
                        let tripped = breaker.record_failure(&fp, now);
                        let consecutive = breaker.consecutive_failures(&fp);
                        (tripped, consecutive)
                    };
                    if tripped {
                        inner.sink.emit(SwarmEvent::BreakerTripped {
                            signature: fp.to_string(),
                            consecutive_failures: consecutive,
                        });
                    }
                    inner.sink.emit(SwarmEvent::SessionFailed {
                        instance_id,
                        error: err.to_string(),
                    });
                    // If this failure pushed the breaker Open, surface the
                    // suppression so the caller stops retrying this signature.
                    let suppressed = {
                        let mut breaker = inner.breaker.lock().expect("breaker poisoned");
                        breaker.admit(&fp, now)
                    };
                    if let AdmitDecision::Suppress {
                        cooldown_remaining_ms,
                    } = suppressed
                    {
                        return Err(SwarmError::BreakerOpen {
                            signature: fp.to_string(),
                            cooldown_remaining_ms,
                        });
                    }
                }
                Err(err)
            }
        }
    }

    /// Launch a Dexterity lane that is runtime-owned by a Handshake manager but
    /// has no OS process to spawn: human/operator, subagent, or validator. The
    /// Rust coordinator still owns the launch path by normalizing through the
    /// Dexterity registry and committing ModelLane + EventLedger rows in the
    /// same PostgreSQL authority path as process-backed launches.
    pub async fn launch_no_os_model_lane(
        &self,
        request: DexterityLaunchAdapterRequest,
        caller: DexterityNoOsLaunchCaller,
    ) -> SwarmResult<(ModelLaneRunRecord, ModelLaneRecord)> {
        if !matches!(
            request.adapter_kind,
            DexterityLaunchAdapterKind::HumanOperator
                | DexterityLaunchAdapterKind::Subagent
                | DexterityLaunchAdapterKind::Validator
        ) {
            return Err(SwarmError::LedgerFailed(format!(
                "Dexterity no-OS launch requires human/operator, subagent, or validator adapter; got {}",
                request.adapter_kind.as_str()
            )));
        }
        validate_no_os_launch_caller(&request, &caller)?;
        self.validate_no_os_authority_session(&caller)?;
        let registry = DexterityLaunchAdapterRegistry::standard();
        let descriptor = registry
            .descriptor(&request.adapter_kind)
            .map_err(|err| {
                SwarmError::LedgerFailed(format!("Dexterity no-OS descriptor failed: {err}"))
            })?
            .clone();
        let launch = registry.normalize(request).map_err(|err| {
            SwarmError::LedgerFailed(format!("Dexterity no-OS preflight failed: {err}"))
        })?;
        if !matches!(
            &descriptor.runtime_binding,
            ModelLaneRuntimeBinding::Human
                | ModelLaneRuntimeBinding::Subagent
                | ModelLaneRuntimeBinding::Validator
        ) || launch.process_ownership_ref.is_some()
            || launch.no_os_process_reason_ref.is_none()
        {
            return Err(SwarmError::LedgerFailed(
                "Dexterity no-OS launch normalized to an invalid runtime ownership contract".into(),
            ));
        }
        let Some(store) = self.inner.model_lane_store.as_ref() else {
            return Err(SwarmError::LedgerFailed(
                "Dexterity no-OS launch requires ModelLaneStore".into(),
            ));
        };
        let records = launch.to_records().map_err(|err| {
            SwarmError::LedgerFailed(format!("Dexterity no-OS record preparation failed: {err}"))
        })?;
        store.record_prepared_launch(records).await.map_err(|err| {
            SwarmError::LedgerFailed(format!("Dexterity no-OS launch record failed: {err}"))
        })
    }

    /// Launch an operator-requested subagent lane that is owned by a Handshake
    /// manager and has no OS process to spawn. This is intentionally narrower
    /// than [`Self::launch_no_os_model_lane`]: it only accepts the subagent
    /// adapter and still normalizes through the Dexterity registry before the
    /// PostgreSQL ModelLane/EventLedger authority write.
    pub async fn launch_operator_subagent_model_lane(
        &self,
        request: DexterityLaunchAdapterRequest,
    ) -> SwarmResult<(ModelLaneRunRecord, ModelLaneRecord)> {
        if request.adapter_kind != DexterityLaunchAdapterKind::Subagent {
            return Err(SwarmError::LedgerFailed(format!(
                "operator subagent launch requires subagent adapter; got {}",
                request.adapter_kind.as_str()
            )));
        }
        let registry = DexterityLaunchAdapterRegistry::standard();
        let descriptor = registry
            .descriptor(&request.adapter_kind)
            .map_err(|err| {
                SwarmError::LedgerFailed(format!("Dexterity subagent descriptor failed: {err}"))
            })?
            .clone();
        let launch = registry.normalize(request).map_err(|err| {
            SwarmError::LedgerFailed(format!("Dexterity subagent preflight failed: {err}"))
        })?;
        if descriptor.runtime_binding != ModelLaneRuntimeBinding::Subagent
            || launch.process_ownership_ref.is_some()
            || launch.no_os_process_reason_ref.is_none()
        {
            return Err(SwarmError::LedgerFailed(
                "operator subagent launch normalized to an invalid no-OS contract".into(),
            ));
        }
        let Some(store) = self.inner.model_lane_store.as_ref() else {
            return Err(SwarmError::LedgerFailed(
                "operator subagent launch requires ModelLaneStore".into(),
            ));
        };
        let records = launch.to_records().map_err(|err| {
            SwarmError::LedgerFailed(format!(
                "Dexterity subagent record preparation failed: {err}"
            ))
        })?;
        store.record_prepared_launch(records).await.map_err(|err| {
            SwarmError::LedgerFailed(format!("Dexterity subagent launch record failed: {err}"))
        })
    }

    pub fn authorize_no_os_model_lane(
        &self,
        request: &DexterityLaunchAdapterRequest,
        authority_instance_id: ModelInstanceId,
    ) -> SwarmResult<DexterityNoOsLaunchCaller> {
        if !matches!(
            request.adapter_kind,
            DexterityLaunchAdapterKind::HumanOperator
                | DexterityLaunchAdapterKind::Subagent
                | DexterityLaunchAdapterKind::Validator
        ) {
            return Err(SwarmError::LedgerFailed(format!(
                "Dexterity no-OS authority can only authorize human/operator, subagent, or validator lanes; got {}",
                request.adapter_kind.as_str()
            )));
        }
        let registry = self.inner.registry.lock().expect("registry poisoned");
        let handle = registry
            .get(&authority_instance_id)
            .ok_or(SwarmError::UnknownInstance(authority_instance_id))?;
        if !matches!(
            handle.state,
            ModelSessionState::Ready | ModelSessionState::Generating
        ) {
            return Err(SwarmError::LedgerFailed(format!(
                "Dexterity no-OS authority session {authority_instance_id} must be Ready or Generating; got {:?}",
                handle.state
            )));
        }
        if handle.lease.is_expired(Utc::now()) {
            return Err(SwarmError::LedgerFailed(format!(
                "Dexterity no-OS authority session {authority_instance_id} lease is expired"
            )));
        }
        if handle.lease.owner != request.owner_session {
            return Err(SwarmError::LedgerFailed(format!(
                "Dexterity no-OS authority owner {} does not match lane owner {}",
                handle.lease.owner, request.owner_session
            )));
        }
        let capability_receipt_ref = expected_no_os_capability_receipt_ref(
            authority_instance_id,
            &request.owner_session,
            &request.adapter_kind,
            &request.run_id,
            &request.lane_id,
        );
        Ok(DexterityNoOsLaunchCaller {
            caller_session: request.owner_session.clone(),
            adapter_kind: request.adapter_kind.clone(),
            run_id: request.run_id.clone(),
            lane_id: request.lane_id.clone(),
            authority_instance_id,
            capability_receipt_ref,
        })
    }

    fn validate_no_os_authority_session(
        &self,
        caller: &DexterityNoOsLaunchCaller,
    ) -> SwarmResult<()> {
        let registry = self.inner.registry.lock().expect("registry poisoned");
        let handle = registry
            .get(&caller.authority_instance_id)
            .ok_or(SwarmError::UnknownInstance(caller.authority_instance_id))?;
        if !matches!(
            handle.state,
            ModelSessionState::Ready | ModelSessionState::Generating
        ) {
            return Err(SwarmError::LedgerFailed(format!(
                "Dexterity no-OS authority session {} must still be Ready or Generating; got {:?}",
                caller.authority_instance_id, handle.state
            )));
        }
        if handle.lease.is_expired(Utc::now()) {
            return Err(SwarmError::LedgerFailed(format!(
                "Dexterity no-OS authority session {} lease is expired",
                caller.authority_instance_id
            )));
        }
        if handle.lease.owner != caller.caller_session {
            return Err(SwarmError::LedgerFailed(format!(
                "Dexterity no-OS authority owner {} no longer matches caller_session {}",
                handle.lease.owner, caller.caller_session
            )));
        }
        Ok(())
    }

    /// Ask the breaker whether a spawn carrying `signature` would be admitted
    /// right now. Lets a caller pre-empt a doomed retry without paying the
    /// factory cost. `signature` is computed from a prior error via
    /// [`FailureFingerprint::compute`].
    pub fn breaker_admits(&self, fp: &FailureFingerprint) -> bool {
        let mut breaker = self.inner.breaker.lock().expect("breaker poisoned");
        matches!(
            breaker.admit(fp, std::time::Instant::now()),
            AdmitDecision::Admit
        )
    }

    /// Transition a live session's state, validating the transition centrally.
    pub fn transition(
        &self,
        instance_id: ModelInstanceId,
        to: ModelSessionState,
    ) -> SwarmResult<()> {
        let from = {
            let mut registry = self.inner.registry.lock().expect("registry poisoned");
            let handle = registry
                .get_mut(&instance_id)
                .ok_or(SwarmError::UnknownInstance(instance_id))?;
            let from = handle.state;
            if !from.can_transition(to) {
                return Err(SwarmError::InvalidStateTransition { from, to });
            }
            if from == ModelSessionState::Loading
                && to == ModelSessionState::Ready
                && !handle.dexterity_model_lane_persisted
            {
                return Err(SwarmError::LedgerFailed(
                    "Dexterity session cannot become Ready before ModelLane persistence commits"
                        .into(),
                ));
            }
            handle.state = to;
            from
        };
        self.inner.sink.emit(SwarmEvent::SessionStateChanged {
            instance_id,
            from,
            to,
        });
        if to == ModelSessionState::Ready {
            self.inner
                .sink
                .emit(SwarmEvent::SessionReady { instance_id });
        }
        Ok(())
    }

    /// Renew a session's claim lease (extends `expires_at` by the configured
    /// TTL from now). A live session calls this to keep the reaper from
    /// reclaiming it.
    pub fn renew_lease(&self, instance_id: ModelInstanceId) -> SwarmResult<()> {
        let mut registry = self.inner.registry.lock().expect("registry poisoned");
        let handle = registry
            .get_mut(&instance_id)
            .ok_or(SwarmError::UnknownInstance(instance_id))?;
        handle.lease.expires_at = Utc::now()
            + chrono::Duration::from_std(self.inner.config.lease_ttl)
                .unwrap_or_else(|_| chrono::Duration::seconds(300));
        Ok(())
    }

    /// Mark a session completed: drives it to a terminal Completed state,
    /// writes the ledger stop row, evicts it, and emits the events.
    pub async fn complete_session(&self, instance_id: ModelInstanceId) -> SwarmResult<()> {
        self.terminate(instance_id, ModelSessionState::Completed, "completed", 0)
            .await
    }

    /// Cancel a session: cancels its token, tears down (frees) the runtime,
    /// writes the ledger stop, evicts it, emits the cancelled event.
    ///
    /// The `SessionCancelled` event is emitted ONLY after `terminate` actually
    /// removed a live handle (C6): cancelling an already-reaped / unknown
    /// instance returns `UnknownInstance` and emits NO spurious cancel event.
    pub async fn cancel_session(
        &self,
        instance_id: ModelInstanceId,
        reason: impl Into<String>,
    ) -> SwarmResult<()> {
        let reason = reason.into();
        self.terminate(instance_id, ModelSessionState::Cancelled, &reason, -1)
            .await
    }

    /// Record token/cost usage against the run budget.
    pub fn record_usage(&self, tokens: u64, cost_micros: u64) {
        let mut acc = self.inner.accounting.lock().expect("accounting poisoned");
        acc.tokens_used = acc.tokens_used.saturating_add(tokens);
        acc.cost_micros_used = acc.cost_micros_used.saturating_add(cost_micros);
    }

    /// Snapshot of remaining budget across every dimension.
    pub fn remaining(&self) -> BudgetRemaining {
        let budget = &self.inner.config.budget;
        let lifetime = self.inner.lifetime_spawns.load(Ordering::SeqCst);
        let acc = self.inner.accounting.lock().expect("accounting poisoned");
        let tokens_remaining = budget
            .max_total_tokens
            .map(|max| max.saturating_sub(acc.tokens_used));
        let cost_remaining = budget
            .max_total_cost_micros
            .map(|max| max.saturating_sub(acc.cost_micros_used));
        let committed_memory_bytes_remaining = budget
            .max_committed_memory_bytes
            .map(|max| max.saturating_sub(acc.committed_memory_bytes_used));
        let lifetime_remaining = budget.max_lifetime_spawns.saturating_sub(lifetime);
        let exhausted = lifetime_remaining == 0
            || tokens_remaining == Some(0)
            || cost_remaining == Some(0)
            || committed_memory_bytes_remaining == Some(0);
        BudgetRemaining {
            concurrency_permits_available: self.inner.semaphore.available_permits(),
            lifetime_spawns_remaining: lifetime_remaining,
            tokens_remaining,
            cost_micros_remaining: cost_remaining,
            committed_memory_bytes_remaining,
            exhausted,
        }
    }

    /// Test-only: breaker state for a signature (C4 healing proof).
    #[cfg(test)]
    pub(crate) fn breaker_state_for_test(
        &self,
        fp: &FailureFingerprint,
    ) -> super::breaker::BreakerState {
        self.inner
            .breaker
            .lock()
            .expect("breaker poisoned")
            .state_of(fp)
    }

    /// Test-only: number of consecutive failures the breaker has recorded for a
    /// signature (C4 healing proof — a real success must reset this to 0).
    #[cfg(test)]
    pub(crate) fn breaker_consecutive_failures_for_test(&self, fp: &FailureFingerprint) -> u32 {
        self.inner
            .breaker
            .lock()
            .expect("breaker poisoned")
            .consecutive_failures(fp)
    }

    /// The live `Arc<dyn ModelRuntime>` registered for a spawned instance, or
    /// `None` if the instance is not (or no longer) in the registry.
    ///
    /// This is the production seam the distillation
    /// [`crate::distillation::parallel_distill::SessionRuntimeResolver`] resolves
    /// against: it lets non-test app/orchestration code drive a real
    /// generate/score on exactly the session the coordinator spawned and owns,
    /// without reaching into the private registry. The returned `Arc` is a clone
    /// of the coordinator-owned handle, so the caller borrows the live engine for
    /// the duration of its work; the coordinator retains ownership for teardown.
    pub fn session_runtime(
        &self,
        instance_id: ModelInstanceId,
    ) -> Option<Arc<dyn crate::model_runtime::ModelRuntime>> {
        self.inner
            .registry
            .lock()
            .expect("registry poisoned")
            .get(&instance_id)
            .filter(|h| {
                matches!(
                    h.state,
                    ModelSessionState::Ready | ModelSessionState::Generating
                )
            })
            .map(|h| h.runtime.clone())
    }

    /// Start one generation through the coordinator-owned session handle.
    ///
    /// The request's `id` and cancellation token are replaced with the live
    /// session values while the registry lock is held. Callers therefore cannot
    /// accidentally create an unrelated cancellation token that bypasses
    /// [`Self::cancel_session`]'s terminal ModelLane/EventLedger transition.
    /// A session may have only one active generation: a second start while it
    /// is already `Generating` fails closed.
    pub fn generate_session(
        &self,
        instance_id: ModelInstanceId,
        mut request: GenerateRequest,
    ) -> SwarmResult<TokenStream> {
        let (runtime, model_id, cancel, from) = {
            let mut registry = self.inner.registry.lock().expect("registry poisoned");
            let handle = registry
                .get_mut(&instance_id)
                .ok_or(SwarmError::UnknownInstance(instance_id))?;
            if handle.state != ModelSessionState::Ready {
                return Err(SwarmError::LedgerFailed(format!(
                    "session {instance_id} must be Ready before generation; got {:?}",
                    handle.state
                )));
            }
            let from = handle.state;
            handle.state = ModelSessionState::Generating;
            (
                handle.runtime.clone(),
                handle.model_id,
                handle.cancel.clone(),
                from,
            )
        };
        request.id = model_id;
        request.cancel = cancel;
        self.inner.sink.emit(SwarmEvent::SessionStateChanged {
            instance_id,
            from,
            to: ModelSessionState::Generating,
        });
        Ok(runtime.generate(request))
    }

    /// The runtime-minted `ModelId` registered for a spawned instance (the id the
    /// factory's load returned, which the generate/score path needs), or `None`
    /// if the instance is not in the registry. Production counterpart to
    /// [`Self::session_runtime`].
    pub fn session_model_id(&self, instance_id: ModelInstanceId) -> Option<ModelId> {
        self.inner
            .registry
            .lock()
            .expect("registry poisoned")
            .get(&instance_id)
            .filter(|h| {
                matches!(
                    h.state,
                    ModelSessionState::Ready | ModelSessionState::Generating
                )
            })
            .map(|h| h.model_id)
    }

    /// The coordinator's wired [`ModelLaneStore`] (a cheap `PgPool`-backed clone),
    /// or `None` when the coordinator was built without one. WP-1 MT-012: the
    /// operator-chat launch service uses this to replay the just-spawned run/lane
    /// so it can persist the launched CLI runtime's real stdout as
    /// ModelLaneMessage rows, and the transcript route uses it to read them back.
    pub fn model_lane_store(&self) -> Option<ModelLaneStore> {
        self.inner.model_lane_store.clone()
    }

    /// Materialize a Dexterity ContextBundle for the named downstream lane from
    /// PostgreSQL/EventLedger authority. Callers pass the returned
    /// `ModelLaneDownstreamContextBundle::to_kernel_context_bundle()` into the
    /// model adapter instead of rebuilding context from prompt memory.
    pub async fn context_bundle_for_downstream_lane(
        &self,
        run_id: &str,
        context_bundle_id: &str,
        downstream_lane_id: &str,
    ) -> SwarmResult<ModelLaneDownstreamContextBundle> {
        let Some(store) = self.inner.model_lane_store.as_ref() else {
            return Err(SwarmError::LedgerFailed(
                "Dexterity ContextBundle consumption requires ModelLaneStore".into(),
            ));
        };
        store
            .consume_context_bundle_for_downstream(run_id, context_bundle_id, downstream_lane_id)
            .await
            .map_err(|err| {
                SwarmError::LedgerFailed(format!(
                    "Dexterity ContextBundle downstream consumption failed: {err}"
                ))
            })
    }

    /// Invoke a model adapter with the downstream lane's replayed Dexterity
    /// ContextBundle. This is the runtime boundary that prevents downstream
    /// models from reconstructing context out of prompt text or provider memory.
    pub async fn invoke_downstream_context_bundle(
        &self,
        run_id: &str,
        context_bundle_id: &str,
        downstream_lane_id: &str,
        adapter: &(dyn ModelAdapter + Send + Sync),
        actor: KernelActor,
    ) -> SwarmResult<ModelAdapterOutput> {
        let downstream = self
            .context_bundle_for_downstream_lane(run_id, context_bundle_id, downstream_lane_id)
            .await?;
        let context_bundle = downstream.to_kernel_context_bundle().map_err(|err| {
            SwarmError::LedgerFailed(format!(
                "Dexterity downstream ContextBundle kernel conversion failed: {err}"
            ))
        })?;
        adapter
            .invoke(ModelAdapterRequest::new(context_bundle, actor))
            .await
            .map_err(|err| {
                SwarmError::LedgerFailed(format!(
                    "Dexterity downstream ContextBundle adapter invocation failed: {err}"
                ))
            })
    }

    /// Board/lineage grouping for a live session: `(swarm_id, worktree_id)` as
    /// copied from its SpawnRequest. Drives the operator board swimlanes and the
    /// Flight-Recorder drill-down join. `None` if the instance is not live.
    pub fn session_grouping(
        &self,
        instance_id: ModelInstanceId,
    ) -> Option<(Option<String>, Option<String>)> {
        self.inner
            .registry
            .lock()
            .expect("registry poisoned")
            .get(&instance_id)
            .map(|h| (h.swarm_id.clone(), h.worktree_id.clone()))
    }

    /// Read-only snapshot of every **live** (non-terminal) instance currently
    /// registered under `swarm_id`. This is the authoritative per-swarm
    /// enumeration the operator-board and the calendar teardown need to cancel
    /// ALL sessions in a swarm — including ones spawned manually, not just by a
    /// scheduler — without reaching into the private registry.
    ///
    /// Semantics + hardening:
    /// * **Exact-match, non-empty:** only handles whose `swarm_id == Some(q)` for
    ///   the queried `q` are returned. Sessions with no swarm (`None`) never
    ///   match, and a blank/whitespace query returns empty rather than matching
    ///   sessions that happen to carry an empty swarm id — a teardown must never
    ///   fan out to unrelated sessions.
    /// * **Terminal excluded:** already-stopped/failed/cancelled handles are
    ///   filtered out, so the caller never tries to re-cancel a dead id.
    /// * **Snapshot, not a live view:** the returned `Vec` is a point-in-time
    ///   copy taken under the registry lock and released before return — NO lock
    ///   is held across the caller's subsequent async `cancel_session` calls. The
    ///   caller MUST re-check `session_state` before cancelling each id, because
    ///   an instance can become terminal (reaped by its time-box, completed, or
    ///   cancelled concurrently) between this snapshot and the cancel; that
    ///   re-check makes the benign TOCTOU a no-op.
    pub fn live_instances_in_swarm(&self, swarm_id: &str) -> Vec<ModelInstanceId> {
        let swarm_id = swarm_id.trim();
        if swarm_id.is_empty() {
            return Vec::new();
        }
        self.inner
            .registry
            .lock()
            .expect("registry poisoned")
            .values()
            .filter(|h| !h.state.is_terminal())
            .filter(|h| h.swarm_id.as_deref() == Some(swarm_id))
            .map(|h| h.instance_id)
            .collect()
    }

    /// Test-only: the live `Arc<dyn ModelRuntime>` registered for an instance,
    /// so the env-gated real parallel test can drive a genuine generate against
    /// exactly the session the coordinator spawned.
    #[cfg(test)]
    pub(crate) fn session_runtime_for_test(
        &self,
        instance_id: ModelInstanceId,
    ) -> Option<Arc<dyn crate::model_runtime::ModelRuntime>> {
        self.session_runtime(instance_id)
    }

    /// Test-only: the runtime-minted `ModelId` registered for an instance (the
    /// id the factory's load returned, which the candle generate path needs).
    #[cfg(test)]
    pub(crate) fn session_model_id_for_test(
        &self,
        instance_id: ModelInstanceId,
    ) -> Option<ModelId> {
        self.session_model_id(instance_id)
    }

    /// Number of breaker signatures currently tracked (observability + the
    /// unbounded-growth guard test, C5).
    pub fn breaker_signature_count(&self) -> usize {
        self.inner
            .breaker
            .lock()
            .expect("breaker poisoned")
            .tracked_signatures()
    }

    /// Sizes of the per-instance accounting maps `(respawns, signatures)`
    /// (observability + the unbounded-growth guard test, C5).
    pub fn accounting_map_sizes(&self) -> (usize, usize) {
        let acc = self.inner.accounting.lock().expect("accounting poisoned");
        (acc.respawns.len(), acc.last_failure_signature.len())
    }

    /// Number of live (non-terminal) sessions currently in the registry.
    pub fn live_session_count(&self) -> usize {
        self.inner
            .registry
            .lock()
            .expect("registry poisoned")
            .values()
            .filter(|h| !h.state.is_terminal())
            .count()
    }

    /// Current state of an instance, if present in the registry.
    pub fn session_state(&self, instance_id: ModelInstanceId) -> Option<ModelSessionState> {
        self.inner
            .registry
            .lock()
            .expect("registry poisoned")
            .get(&instance_id)
            .map(|h| h.state)
    }

    /// Count of sessions currently occupying a concurrency slot (Loading..
    /// Cancelling). Used by the concurrency-cap invariant assertion.
    pub fn slot_occupancy(&self) -> usize {
        self.inner
            .registry
            .lock()
            .expect("registry poisoned")
            .values()
            .filter(|h| h.state.occupies_slot())
            .count()
    }

    /// Cancel + drain every live session (orderly shutdown). After this, no
    /// session remains live and every started process has a matching stop row.
    pub async fn drain_all(&self) -> SwarmResult<()> {
        let live: Vec<ModelInstanceId> = self
            .inner
            .registry
            .lock()
            .expect("registry poisoned")
            .values()
            .filter(|h| !h.state.is_terminal())
            .map(|h| h.instance_id)
            .collect();
        for instance_id in live {
            self.cancel_session(instance_id, "drain_all").await?;
        }
        Ok(())
    }

    // ---- internals ----

    /// Atomic check-and-insert (D2). Holds the registry lock across the
    /// duplicate check AND the insert so two concurrent same-instance spawns
    /// cannot both insert. On success the handle is registered as Loading and
    /// only the spawned event is emitted; the caller commits Dexterity records
    /// first and then transitions to Ready. On a duplicate (a live instance
    /// already present) the `live` session and `permit` are returned to the
    /// caller UNCONSUMED so the caller can roll them back without an orphan
    /// START.
    ///
    /// No `.await` is taken while the registry lock is held, preserving the
    /// no-lock-across-await property.
    #[allow(clippy::result_large_err)]
    fn try_insert_loading(
        &self,
        request: &SpawnRequest,
        live: LiveSession,
        permit: OwnedSemaphorePermit,
        committed_memory_bytes: u64,
    ) -> Result<(), (LiveSession, OwnedSemaphorePermit)> {
        let now = Utc::now();
        // rank-7: a per-spawn time_box overrides the configured lease_ttl, so a
        // time-boxed (e.g. calendar-scheduled) session expires after its box and
        // the existing reaper reclaims it -- no new teardown path.
        let lease_lifetime = request.time_box.unwrap_or(self.inner.config.lease_ttl);
        let expires_at = now
            + chrono::Duration::from_std(lease_lifetime)
                .unwrap_or_else(|_| chrono::Duration::seconds(300));
        let instance_id = request.instance_id;
        let process_uuid = live.process_record_id.as_uuid();
        let process_engine_kind = process_engine_kind_for_request(request);

        {
            let mut registry = self.inner.registry.lock().expect("registry poisoned");
            if let Some(existing) = registry.get(&instance_id) {
                if !existing.state.is_terminal() {
                    // Lost the race — hand the session + permit back untouched.
                    return Err((live, permit));
                }
            }
            let handle = SessionHandle {
                instance_id,
                state: ModelSessionState::Loading,
                lease: ClaimLease {
                    instance_id,
                    owner: request.owner_role.clone(),
                    expires_at,
                },
                cancel: live.cancel,
                model_id: live.model_id,
                process_record_id: live.process_record_id,
                os_pid: live.os_pid,
                process_engine_kind,
                parent_session_id: request.parent_session_id.clone(),
                runtime: live.runtime,
                teardown: Some(live.teardown),
                permit: Some(permit),
                started_at: now,
                swarm_id: request.swarm_id.clone(),
                worktree_id: request.worktree_id.clone(),
                committed_memory_bytes,
                dexterity_model_lane_persisted: request.dexterity_launch.is_none(),
                dexterity_lane_id: request
                    .dexterity_launch
                    .as_ref()
                    .map(|launch| launch.lane_id.clone()),
            };
            registry.insert(instance_id, handle);
        }

        self.inner.sink.emit(SwarmEvent::SessionSpawned {
            instance_id,
            parent_session_id: request.parent_session_id.clone(),
            process_uuid,
            swarm_id: request.swarm_id.clone(),
            worktree_id: request.worktree_id.clone(),
        });
        Ok(())
    }

    fn mark_dexterity_model_lane_persisted(&self, instance_id: ModelInstanceId) -> SwarmResult<()> {
        let mut registry = self.inner.registry.lock().expect("registry poisoned");
        let handle = registry
            .get_mut(&instance_id)
            .ok_or(SwarmError::UnknownInstance(instance_id))?;
        handle.dexterity_model_lane_persisted = true;
        Ok(())
    }

    /// Tear down a session that never made it into the registry (D2 duplicate
    /// loser). Cancels the token, runs the engine teardown so the loaded model
    /// is freed, and writes the matching ledger STOP so the START the factory
    /// recorded for this losing session has no orphan.
    async fn teardown_orphan(&self, request: &SpawnRequest, live: LiveSession) {
        let instance_id = request.instance_id;
        live.cancel.cancel();
        live.runtime.cancel(live.cancel.clone());
        // Free the engine resource (D1 contract).
        let _ = (live.teardown)().await;

        let stop = self.build_stop(
            live.process_record_id,
            live.os_pid,
            process_engine_kind_for_request(request),
            None,
            Utc::now(),
            ModelSessionState::Cancelled,
            "duplicate_instance_rollback",
            -1,
            &instance_id,
        );
        let _ = self.inner.ledger.record_stop(stop);
        self.inner.sink.emit(SwarmEvent::ResourceEvicted {
            instance_id,
            terminal_state: ModelSessionState::Cancelled,
        });
    }

    /// Terminal teardown shared by complete/cancel/reap: cancel token, run the
    /// engine teardown (free the model — D1), write the ledger stop, evict,
    /// prune accounting maps (C5), emit terminal + evicted events.
    ///
    /// `SessionCancelled` for the Cancelled terminal is emitted here, AFTER the
    /// handle was actually removed, so an already-reaped instance produces NO
    /// spurious cancel event (C6).
    async fn terminate(
        &self,
        instance_id: ModelInstanceId,
        terminal: ModelSessionState,
        reason: &str,
        exit_code: i32,
    ) -> SwarmResult<()> {
        // Fence the live handle before awaiting the durable terminal write.
        // Without this state transition, a concurrent `generate_session` can
        // observe Ready and begin provider work while cancellation is already
        // underway but blocked on PostgreSQL/EventLedger.  The terminal lane
        // receipt remains the authoritative gate; `Cancelling` is the local
        // coordinator fence that prevents a new runtime start in that gap.
        let (previous_state, terminal_record) = {
            let mut registry = self.inner.registry.lock().expect("registry poisoned");
            let handle = registry
                .get_mut(&instance_id)
                .ok_or(SwarmError::UnknownInstance(instance_id))?;
            if handle.state == ModelSessionState::Cancelling {
                return Err(SwarmError::LedgerFailed(format!(
                    "session {instance_id} termination is already in progress"
                )));
            }
            if handle.state.is_terminal() {
                return Err(SwarmError::UnknownInstance(instance_id));
            }
            let previous_state = handle.state;
            handle.state = ModelSessionState::Cancelling;
            let terminal_record = if handle.dexterity_model_lane_persisted {
                self.inner
                    .model_lane_store
                    .as_ref()
                    .cloned()
                    .zip(handle.dexterity_lane_id.clone())
            } else {
                None
            };
            (previous_state, terminal_record)
        };

        if let Some(status) = model_lane_terminal_status(terminal) {
            if let Some((store, lane_id)) = terminal_record {
                if let Err(err) = store
                    .record_lane_terminal_status(&lane_id, status, reason)
                    .await
                {
                    // A failed durable terminal write must not strand the
                    // session in a local-only terminal fence. Restore exactly
                    // the prior live state so a retry can take the same path.
                    let mut registry = self.inner.registry.lock().expect("registry poisoned");
                    if let Some(handle) = registry.get_mut(&instance_id) {
                        if handle.state == ModelSessionState::Cancelling {
                            handle.state = previous_state;
                        }
                    }
                    return Err(SwarmError::LedgerFailed(format!(
                        "Dexterity terminal lane state record failed: {err}"
                    )));
                }
            }
        }

        // Remove the handle from the registry (releases the permit via Drop).
        let mut handle = {
            let mut registry = self.inner.registry.lock().expect("registry poisoned");
            match registry.remove(&instance_id) {
                Some(h) => h,
                None => return Err(SwarmError::UnknownInstance(instance_id)),
            }
        };

        // Cancel the session's token so any in-flight generation aborts.
        handle.cancel.cancel();
        handle.runtime.cancel(handle.cancel.clone());

        // D1: actually FREE the loaded model. `cancel` only aborts in-flight
        // generation; teardown is the only thing that releases the engine
        // resource (drop owned Arc / unload on a shared runtime). Runs exactly
        // once via Option::take. The model_id is passed through the closure the
        // factory captured at load time.
        let _ = handle.model_id; // retained for shared-runtime teardown identity
        if let Some(teardown) = handle.teardown.take() {
            let _ = teardown().await;
        }
        self.inner
            .release_committed_memory(handle.committed_memory_bytes);

        // Write the matching process-ledger stop row so there is no orphan.
        let stop = self.build_stop(
            handle.process_record_id,
            handle.os_pid,
            handle.process_engine_kind,
            Some(handle.parent_session_id.clone()),
            handle.started_at,
            terminal,
            reason,
            exit_code,
            &instance_id,
        );
        self.inner
            .ledger
            .record_stop(stop)
            .map_err(|e| SwarmError::LedgerFailed(e.to_string()))?;

        // Prune per-instance accounting now that the instance is terminal so the
        // respawn + signature maps cannot grow without bound (C5).
        {
            let mut acc = self.inner.accounting.lock().expect("accounting poisoned");
            acc.respawns.remove(&instance_id);
            acc.last_failure_signature.remove(&instance_id);
        }

        // Emit terminal + evicted events. The permit is released as `handle`
        // (and its `permit` field) drops at end of scope.
        match terminal {
            ModelSessionState::Completed => self
                .inner
                .sink
                .emit(SwarmEvent::SessionCompleted { instance_id }),
            ModelSessionState::Failed => self.inner.sink.emit(SwarmEvent::SessionFailed {
                instance_id,
                error: reason.to_string(),
            }),
            ModelSessionState::Cancelled => self.inner.sink.emit(SwarmEvent::SessionCancelled {
                instance_id,
                reason: reason.to_string(),
            }),
            _ => {}
        }
        self.inner.sink.emit(SwarmEvent::ResourceEvicted {
            instance_id,
            terminal_state: terminal,
        });
        Ok(())
    }

    /// Build a process-ledger STOP row for a terminating session. Shared by
    /// `terminate`, `teardown_orphan`, and the reaper so START/STOP rows are
    /// symmetric in one place (C7).
    #[allow(clippy::too_many_arguments)]
    fn build_stop(
        &self,
        process_record_id: ProcessOwnershipRecordId,
        os_pid: u32,
        engine_kind: ProcessEngineKind,
        parent_session_id: Option<String>,
        started_at: DateTime<Utc>,
        terminal: ModelSessionState,
        reason: &str,
        exit_code: i32,
        instance_id: &ModelInstanceId,
    ) -> ProcessStop {
        ProcessStop {
            process_uuid: process_record_id.as_uuid(),
            os_pid: Some(os_pid),
            parent_session_id,
            parent_process_id: None,
            sandbox_adapter_id: None,
            sandbox_internal_id: None,
            engine_kind,
            started_at,
            stopped_at: Utc::now(),
            exit_code: Some(exit_code),
            stop_reason: Some(reason.to_string()),
            model_artifact_sha256: None,
            work_profile_id: None,
            owner_role: self.inner.config.owner_role.clone(),
            owner_wp: None,
            role_id: None,
            wp_id: None,
            mt_id: None,
            sandbox_capabilities_snapshot: serde_json::json!({}),
            metadata_jsonb: serde_json::json!({
                "instance_id": instance_id.to_string(),
                "reclaim_trigger": match terminal {
                    ModelSessionState::Cancelled => ReclaimTrigger::OperatorCancel.as_str(),
                    ModelSessionState::Failed => ReclaimTrigger::Failure.as_str(),
                    _ => ReclaimTrigger::Close.as_str(),
                },
            }),
        }
    }

    fn emit_spawn_rejected(&self, instance_id: ModelInstanceId, reason: &str) {
        self.inner.sink.emit(SwarmEvent::SpawnRejected {
            instance_id,
            reason: reason.to_string(),
        });
    }
}

impl Drop for SwarmCoordinator {
    fn drop(&mut self) {
        if let Some(handle) = self.reaper.lock().ok().and_then(|mut g| g.take()) {
            handle.abort();
        }
    }
}

fn process_engine_kind_for_request(request: &SpawnRequest) -> ProcessEngineKind {
    match request.provider {
        Some(ProviderKind::OfficialCli) => ProcessEngineKind::OfficialCliBridge,
        Some(ProviderKind::ByokCloud) => ProcessEngineKind::HelperSubprocess,
        Some(ProviderKind::ExternalCompat) => ProcessEngineKind::ExternalCompat,
        Some(ProviderKind::Local) | None => match request.runtime_binding.adapter_id() {
            "candle" => ProcessEngineKind::Candle,
            _ => ProcessEngineKind::LlamaCpp,
        },
    }
}

fn model_lane_terminal_status(terminal: ModelSessionState) -> Option<ModelLaneStatus> {
    match terminal {
        ModelSessionState::Completed => Some(ModelLaneStatus::Completed),
        ModelSessionState::Failed => Some(ModelLaneStatus::Failed),
        ModelSessionState::Cancelled => Some(ModelLaneStatus::Cancelled),
        _ => None,
    }
}

fn expected_no_os_capability_receipt_ref(
    authority_instance_id: ModelInstanceId,
    caller_session: &str,
    adapter_kind: &DexterityLaunchAdapterKind,
    run_id: &str,
    lane_id: &str,
) -> String {
    format!(
        "dexterity-no-os-launch://{}/{}/{}/{}/{}",
        authority_instance_id,
        caller_session,
        adapter_kind.as_str(),
        run_id,
        lane_id
    )
}

fn validate_no_os_launch_caller(
    request: &DexterityLaunchAdapterRequest,
    caller: &DexterityNoOsLaunchCaller,
) -> SwarmResult<()> {
    if caller.caller_session.trim().is_empty() {
        return Err(SwarmError::LedgerFailed(
            "Dexterity no-OS launch requires caller_session".into(),
        ));
    }
    if caller.caller_session != request.owner_session {
        return Err(SwarmError::LedgerFailed(format!(
            "Dexterity no-OS caller_session {} does not match owner_session {}",
            caller.caller_session, request.owner_session
        )));
    }
    if caller.adapter_kind != request.adapter_kind {
        return Err(SwarmError::LedgerFailed(format!(
            "Dexterity no-OS caller adapter {} does not match request adapter {}",
            caller.adapter_kind.as_str(),
            request.adapter_kind.as_str()
        )));
    }
    if caller.run_id != request.run_id || caller.lane_id != request.lane_id {
        return Err(SwarmError::LedgerFailed(format!(
            "Dexterity no-OS caller receipt is bound to run_id {} lane_id {}, not run_id {} lane_id {}",
            caller.run_id, caller.lane_id, request.run_id, request.lane_id
        )));
    }
    let expected = expected_no_os_capability_receipt_ref(
        caller.authority_instance_id,
        &caller.caller_session,
        &caller.adapter_kind,
        &caller.run_id,
        &caller.lane_id,
    );
    if caller.capability_receipt_ref != expected {
        return Err(SwarmError::LedgerFailed(format!(
            "Dexterity no-OS launch requires capability receipt {expected}"
        )));
    }
    Ok(())
}

impl Inner {
    fn exhausted_global_budget_dimension(&self) -> Option<String> {
        let budget = &self.config.budget;
        let acc = self.accounting.lock().expect("accounting poisoned");
        if let Some(max) = budget.max_total_tokens {
            if acc.tokens_used >= max {
                return Some("tokens".to_string());
            }
        }
        if let Some(max) = budget.max_total_cost_micros {
            if acc.cost_micros_used >= max {
                return Some("cost".to_string());
            }
        }
        None
    }

    /// Reserve committed memory before factory/model/VM creation. The caller
    /// must release the same amount on every later rollback or teardown path.
    /// A zero reservation is a no-op so requests without estimates stay
    /// backward-compatible.
    fn try_reserve_committed_memory(&self, bytes: u64) -> Result<(), String> {
        if bytes == 0 {
            return Ok(());
        }
        let Some(max) = self.config.budget.max_committed_memory_bytes else {
            return Ok(());
        };
        let mut acc = self.accounting.lock().expect("accounting poisoned");
        let next = acc.committed_memory_bytes_used.saturating_add(bytes);
        if next > max {
            return Err("committed_memory".to_string());
        }
        acc.committed_memory_bytes_used = next;
        Ok(())
    }

    /// Release a previous committed-memory reservation. Saturating subtraction
    /// keeps teardown idempotent under defensive double-release bugs while still
    /// preserving no-overcommit on the normal single-release path.
    fn release_committed_memory(&self, bytes: u64) {
        if bytes == 0 {
            return;
        }
        let mut acc = self.accounting.lock().expect("accounting poisoned");
        acc.committed_memory_bytes_used = acc.committed_memory_bytes_used.saturating_sub(bytes);
    }

    /// Heal the breaker for an instance's tracked failure signature after a
    /// real success (C4). If the instance previously failed, its signature is
    /// recorded; a genuine success closes that signature's breaker (resets the
    /// consecutive-failure count and forces Closed) so recovery does not depend
    /// on cooldown alone. The instance's signature record is cleared.
    fn heal_breaker_for_instance(&self, instance_id: ModelInstanceId) {
        let fp = {
            let mut acc = self.accounting.lock().expect("accounting poisoned");
            acc.last_failure_signature.remove(&instance_id)
        };
        if let Some(fp) = fp {
            let mut breaker = self.breaker.lock().expect("breaker poisoned");
            breaker.record_success(&fp);
        }
    }

    /// The most recent failure signature observed for `instance_id`, if any.
    /// Drives the breaker ADMISSION gate (D3).
    fn last_failure_signature_for(
        &self,
        instance_id: ModelInstanceId,
    ) -> Option<FailureFingerprint> {
        self.accounting
            .lock()
            .expect("accounting poisoned")
            .last_failure_signature
            .get(&instance_id)
            .cloned()
    }

    /// Record the failure signature observed for an instance so a later respawn
    /// can be gated by the breaker before paying the factory cost (D3) and so a
    /// later success knows which signature to heal (C4).
    fn record_instance_signature(&self, instance_id: ModelInstanceId, fp: FailureFingerprint) {
        self.accounting
            .lock()
            .expect("accounting poisoned")
            .last_failure_signature
            .insert(instance_id, fp);
    }
}

/// One reaper pass: reclaim every session whose lease has expired. Reclamation
/// cancels + unloads the session, writes the ledger stop, evicts it, and emits
/// LeaseExpired. A per-instance respawn counter (anti-storm) is incremented so
/// a flapping session is eventually abandoned rather than respawned forever.
async fn reap_expired(inner: &Arc<Inner>) {
    let now = Utc::now();
    let expired: Vec<(ModelInstanceId, String)> = {
        let registry = inner.registry.lock().expect("registry poisoned");
        registry
            .values()
            .filter(|h| !h.state.is_terminal() && h.lease.is_expired(now))
            .map(|h| (h.instance_id, h.lease.owner.clone()))
            .collect()
    };

    for (instance_id, owner) in expired {
        inner
            .sink
            .emit(SwarmEvent::LeaseExpired { instance_id, owner });

        // Mirror `terminate`'s cancelling fence. A lease reaper must not leave
        // a Ready handle startable while it awaits the durable terminal lane
        // receipt, and a concurrent explicit cancel must not race a second
        // terminal write against it.
        let fenced = {
            let mut registry = inner.registry.lock().expect("registry poisoned");
            match registry.get_mut(&instance_id) {
                None => None,
                Some(handle)
                    if handle.state.is_terminal()
                        || handle.state == ModelSessionState::Cancelling =>
                {
                    None
                }
                Some(handle) => {
                    let previous_state = handle.state;
                    handle.state = ModelSessionState::Cancelling;
                    let terminal_record = if handle.dexterity_model_lane_persisted {
                        inner
                            .model_lane_store
                            .as_ref()
                            .cloned()
                            .zip(handle.dexterity_lane_id.clone())
                    } else {
                        None
                    };
                    Some((previous_state, terminal_record))
                }
            }
        };
        let Some((previous_state, terminal_record)) = fenced else {
            continue;
        };
        if let Some((store, lane_id)) = terminal_record {
            if let Err(err) = store
                .record_lane_terminal_status(
                    &lane_id,
                    ModelLaneStatus::Cancelled,
                    "lease_expired_reclaim",
                )
                .await
            {
                let mut registry = inner.registry.lock().expect("registry poisoned");
                if let Some(handle) = registry.get_mut(&instance_id) {
                    if handle.state == ModelSessionState::Cancelling {
                        handle.state = previous_state;
                    }
                }
                inner.sink.emit(SwarmEvent::SessionFailed {
                    instance_id,
                    error: format!(
                        "Dexterity terminal lane state record failed before lease reaper teardown: {err}"
                    ),
                });
                continue;
            }
        }

        // Anti-storm: bump and check the respawn counter after terminal
        // persistence succeeds. If terminal persistence fails, the live handle
        // stays in the registry and the next reaper pass can retry.
        let over_storm_cap = {
            let mut acc = inner.accounting.lock().expect("accounting poisoned");
            let count = acc.respawns.entry(instance_id).or_insert(0);
            *count = count.saturating_add(1);
            *count > inner.config.max_respawns_per_instance
        };

        // Reclaim: remove from registry, cancel, tear down (free model),
        // ledger-stop.
        let mut handle = {
            let mut registry = inner.registry.lock().expect("registry poisoned");
            registry.remove(&instance_id)
        };
        let Some(handle) = handle.as_mut() else {
            continue;
        };
        handle.cancel.cancel();
        handle.runtime.cancel(handle.cancel.clone());

        // D1: actually FREE the loaded model on lease-expiry reclaim, mirroring
        // the explicit terminate path. cancel() alone does not release engine
        // memory; teardown does.
        let _ = handle.model_id;
        if let Some(teardown) = handle.teardown.take() {
            let _ = teardown().await;
        }
        inner.release_committed_memory(handle.committed_memory_bytes);

        let stop = ProcessStop {
            process_uuid: handle.process_record_id.as_uuid(),
            os_pid: Some(handle.os_pid),
            parent_session_id: Some(handle.parent_session_id.clone()),
            parent_process_id: None,
            sandbox_adapter_id: None,
            sandbox_internal_id: None,
            engine_kind: handle.process_engine_kind,
            started_at: handle.started_at,
            stopped_at: Utc::now(),
            exit_code: Some(-1),
            stop_reason: Some("lease_expired_reclaim".to_string()),
            model_artifact_sha256: None,
            work_profile_id: None,
            owner_role: inner.config.owner_role.clone(),
            owner_wp: None,
            role_id: None,
            wp_id: None,
            mt_id: None,
            sandbox_capabilities_snapshot: serde_json::json!({}),
            metadata_jsonb: serde_json::json!({
                "instance_id": instance_id.to_string(),
                "reclaim_trigger": ReclaimTrigger::Stale.as_str(),
                "respawn_storm_capped": over_storm_cap,
                "swarm_id": handle.swarm_id,
                "worktree_id": handle.worktree_id,
            }),
        };
        let _ = inner.ledger.record_stop(stop);

        inner.sink.emit(SwarmEvent::ResourceEvicted {
            instance_id,
            terminal_state: ModelSessionState::Cancelled,
        });

        // C5: if this instance is abandoned (past the respawn storm cap) it
        // will never respawn, so drop its accounting entries (respawn counter +
        // tracked signature) to bound map growth. Otherwise keep the respawn
        // counter so the anti-storm bound stays meaningful across reclaims.
        if over_storm_cap {
            let mut acc = inner.accounting.lock().expect("accounting poisoned");
            acc.respawns.remove(&instance_id);
            acc.last_failure_signature.remove(&instance_id);
        }
        // Permit released as `handle` drops here.
    }

    // C5: prune fully-settled breaker signatures (Closed-no-failures or
    // cooled-down Open) so the signature map cannot grow without bound across a
    // long run of heterogeneous failures.
    {
        let mut breaker = inner.breaker.lock().expect("breaker poisoned");
        breaker.prune_settled(std::time::Instant::now());
    }
}

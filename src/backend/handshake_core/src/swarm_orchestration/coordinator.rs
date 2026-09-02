//! The load-bearing [`SwarmCoordinator`].
//!
//! Owns the single spawn path, the session registry, the two-level fan-out
//! bound (concurrency semaphore + monotonic lifetime ceiling), the
//! claim-lease/TTL/reaper machinery, the failure-fingerprint circuit breaker,
//! and the budget-as-data ledger. It is generic over an injected
//! [`ModelSessionFactory`] and a [`SwarmEventSink`] so it carries no
//! candle/llama specifics and is fully exercisable with a real controllable
//! worker adapter in tests.

use std::collections::{hash_map::Entry, HashMap};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{DateTime, Utc};
use tokio::sync::{watch, OwnedSemaphorePermit, Semaphore};
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::llm::{LlmClient, LlmInvocationContext, LlmInvocationEvidenceOwner};
use crate::model_runtime::{
    CancellationToken, GenerateRequest, ModelId, ProviderKind, TokenStream,
};
use crate::process_ledger::{
    ActiveProcessLifecycle, KillOutcome, LedgerBatcher, LedgerEventKind, ProcessEngineKind,
    ProcessOwnershipRecordId, ProcessStart, ProcessStop, Reclaim, ReclaimTrigger,
    StopRecordOutcome,
};

use super::breaker::{AdmitDecision, BreakerConfig, FailureFingerprint, FailureFingerprintBreaker};
use super::checkout_lease::CheckoutLeaseGuard;
use super::error::{SwarmError, SwarmResult};
use super::events::{SwarmEvent, SwarmEventSink};
use super::factory::{LiveSession, ModelSessionFactory, SessionReadyHook, SessionTeardown};
use super::ids::{
    BudgetRemaining, ModelInstanceId, RoutingAttemptIdentity, RunBudget, SpawnRequest,
};
use super::model_lane::{
    build_failed_launch_records, build_successful_launch_records, DexterityLaunchAdapterKind,
    DexterityLaunchAdapterRegistry, DexterityLaunchAdapterRequest, LaunchAuthority,
    ModelLaneDownstreamContextBundle, ModelLaneKind, ModelLaneMessageRecord, ModelLaneRecord,
    ModelLaneRunRecord, ModelLaneStatus, ModelLaneStore, NewModelLaneContextBundleArtifactBinding,
    NewModelLaneMessage, RuntimeBinding as ModelLaneRuntimeBinding,
};
use super::state::ModelSessionState;
use crate::kernel::{KernelActor, ModelAdapter, ModelAdapterOutput, ModelAdapterRequest};

/// Max number of times a single instance may be respawned by the reaper before
/// it is given up on, so a flapping session cannot storm respawns.
pub const DEFAULT_MAX_RESPAWNS_PER_INSTANCE: u32 = 3;

fn parse_routing_model_instance_id(value: &str) -> SwarmResult<ModelInstanceId> {
    let (model_id, instance) = value.rsplit_once('#').ok_or_else(|| {
        SwarmError::LedgerFailed(format!("invalid routing model instance id {value}"))
    })?;
    let model_id = uuid::Uuid::parse_str(model_id)
        .map(ModelId::from)
        .map_err(|err| {
            SwarmError::LedgerFailed(format!("invalid routing model id in {value}: {err}"))
        })?;
    let instance = instance.parse::<u32>().map_err(|err| {
        SwarmError::LedgerFailed(format!(
            "invalid routing instance ordinal in {value}: {err}"
        ))
    })?;
    Ok(ModelInstanceId::new(model_id, instance))
}

/// Reconcile a worker-side stale claim only when cancellation is the canonical
/// terminal authority for this exact stage attempt and runtime instance.
async fn canonical_cancelled_routing_dispatch(
    store: &super::routing_execution::ModelLaneRoutingExecutionStore,
    claim: &super::routing_execution::ModelLaneRoutingStageClaim,
    expected_instance_id: Option<&str>,
) -> Result<Option<super::routing_execution::ModelLaneRoutingStageDispatch>, String> {
    let Some(execution) = store.snapshot(&claim.execution_id).await? else {
        return Ok(None);
    };
    let Some(stage) = execution.stages.get(&claim.stage_id) else {
        return Ok(None);
    };
    if stage.state != super::routing_execution::ModelLaneRoutingStageStateKind::Cancelled
        || stage.attempt != claim.attempt
        || expected_instance_id
            .is_some_and(|expected| stage.instance_id.as_deref() != Some(expected))
    {
        return Ok(None);
    }
    Ok(Some(
        super::routing_execution::ModelLaneRoutingStageDispatch {
            stage_id: claim.stage_id.clone(),
            dispatch_target: claim.dispatch_target,
            state: super::routing_execution::ModelLaneRoutingStageStateKind::Cancelled,
            instance_id: stage.instance_id.clone(),
            detail: stage.detail.clone(),
        },
    ))
}

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
    /// Hard wall-clock bound for one provider generation invocation.
    pub provider_invocation_timeout: Duration,
    /// Maximum silence between provider stream items.
    pub provider_idle_timeout: Duration,
    /// Bound for one owned teardown attempt before it is aborted, joined, and
    /// left in cleanup-pending state for a recoverable retry.
    pub teardown_timeout: Duration,
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
            provider_invocation_timeout: Duration::from_secs(300),
            provider_idle_timeout: Duration::from_secs(30),
            teardown_timeout: Duration::from_secs(5),
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

    pub fn with_provider_deadlines(mut self, invocation: Duration, idle: Duration) -> Self {
        self.provider_invocation_timeout = invocation;
        self.provider_idle_timeout = idle;
        self
    }

    pub fn with_teardown_timeout(mut self, timeout: Duration) -> Self {
        self.teardown_timeout = timeout;
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
    /// Monotonic registry incarnation. A stale generation future may retain the
    /// same stable ModelInstanceId after recovery respawns it, so terminal
    /// cleanup must match this incarnation before touching the current handle.
    spawn_generation: u64,
    routing_attempt: Option<super::ids::RoutingAttemptIdentity>,
    pub state: ModelSessionState,
    pub lease: ClaimLease,
    pub cancel: CancellationToken,
    /// The `ModelId` the factory's load returned. Retained so teardown can free
    /// exactly this model from a shared runtime (D1).
    pub model_id: ModelId,
    pub process_record_id: ProcessOwnershipRecordId,
    pub os_pid: u32,
    pub ledger_os_pid: Option<u32>,
    pub ledger_start_override: Option<ProcessStart>,
    pub ledger_lifecycle: Option<Arc<ActiveProcessLifecycle>>,
    pub process_engine_kind: ProcessEngineKind,
    pub parent_session_id: String,
    pub runtime: Arc<dyn crate::model_runtime::ModelRuntime>,
    /// Application generation authority. The runtime field above is retained
    /// only for lifecycle/control operations such as cancel, score, and teardown.
    pub llm_client: Arc<dyn LlmClient>,
    /// Retryable async teardown that frees the engine resource. Retained until
    /// teardown and the matching process-ledger STOP have both succeeded.
    teardown: Option<SessionTeardown>,
    /// Secondary publication committed only after all launch persistence and
    /// the Ready transition succeed.
    ready_hook: Option<SessionReadyHook>,
    /// Cross-process checkout locks retained through teardown + durable STOP.
    _checkout_lease: Option<CheckoutLeaseGuard>,
    cleanup: Option<PendingSessionCleanup>,
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
    dexterity_consent_receipt_id: Option<String>,
}

struct PendingSpawn {
    cancel: CancellationToken,
    spawn_generation: u64,
    routing_attempt: Option<super::ids::RoutingAttemptIdentity>,
    dexterity_lane_id: Option<String>,
    dexterity_consent_receipt_id: Option<String>,
    checkout_lease: Option<CheckoutLeaseGuard>,
    /// Set by `revoke_cloud_consent_receipt` when this in-flight spawn is
    /// cancelled specifically because its lane-bound cloud consent was revoked
    /// (and durably fenced) mid-flight. The fence commit happens before this
    /// flag is set, so when it is observed the post-factory durable lane insert
    /// would fail closed under CX-MM-007. The spawn path reads it during
    /// factory-create compensation to surface the consent-revoked (CX-MM-007)
    /// error instead of the generic factory-cancellation error.
    revoke_fence: Arc<AtomicBool>,
}

type RoutingSpawnAdmissionKey = (RoutingAttemptIdentity, ModelInstanceId);

/// In-memory half of the durable routing-cancellation fence. A routing worker
/// registers this owner before its final canonical cancellation read and keeps
/// it until `PendingSpawn` is atomically published. Cancellation marks any
/// registered owner before scanning pending/live maps, so it cannot return in
/// the post-read/pre-publication window and let a runtime appear behind it.
struct RoutingSpawnAdmissionGuard {
    inner: Arc<Inner>,
    key: RoutingSpawnAdmissionKey,
}

impl Drop for RoutingSpawnAdmissionGuard {
    fn drop(&mut self) {
        self.inner
            .routing_spawn_admissions
            .lock()
            .expect("routing spawn admissions poisoned")
            .remove(&self.key);
    }
}

#[allow(clippy::result_large_err)]
enum TryInsertLoadingError {
    Duplicate {
        live: LiveSession,
        permit: OwnedSemaphorePermit,
        checkout_lease: Option<CheckoutLeaseGuard>,
    },
    EventSink {
        live: LiveSession,
        permit: OwnedSemaphorePermit,
        checkout_lease: Option<CheckoutLeaseGuard>,
        error: SwarmError,
    },
}

struct PendingOrphanCleanup {
    instance_id: ModelInstanceId,
    cancel: CancellationToken,
    runtime: Arc<dyn crate::model_runtime::ModelRuntime>,
    teardown: SessionTeardown,
    stop: ProcessStop,
    ledger_lifecycle: Option<Arc<ActiveProcessLifecycle>>,
    _permit: OwnedSemaphorePermit,
    committed_memory_bytes: u64,
    _checkout_lease: Option<CheckoutLeaseGuard>,
    teardown_succeeded: bool,
    stop_succeeded: bool,
    owner_generation: u64,
    in_progress: bool,
}

#[derive(Clone)]
struct PendingSessionCleanup {
    terminal: ModelSessionState,
    reason: String,
    exit_code: i32,
    teardown_succeeded: bool,
    stop_succeeded: bool,
    owner_generation: u64,
    in_progress: bool,
    owner_outcome_tx: watch::Sender<CleanupOwnerOutcome>,
    terminal_event_id: Uuid,
    resource_evicted_event_id: Uuid,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum CleanupOwnerOutcome {
    Idle,
    InProgress,
    Succeeded,
    Failed(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CleanupContentionPolicy {
    WaitForOwner,
    SkipIfOwned,
}

struct CleanupOwnerClaim {
    terminal: ModelSessionState,
    reason: String,
    exit_code: i32,
    terminal_record: Option<(ModelLaneStore, String)>,
    spawn_generation: u64,
    generation: u64,
    outcome_tx: watch::Sender<CleanupOwnerOutcome>,
}

enum CleanupClaim {
    Owner(CleanupOwnerClaim),
    Wait(watch::Receiver<CleanupOwnerOutcome>),
    StaleGeneration,
}

struct CleanupOwnershipGuard {
    inner: Arc<Inner>,
    instance_id: ModelInstanceId,
    spawn_generation: u64,
    generation: u64,
}

/// Diagnostic-only record of one decision point inside
/// [`SwarmCoordinator::terminate`]. Exists to make a missing session teardown
/// name its own branch instead of being reconstructed from code reading: the
/// teardowns==1 orphan defect (MT-009) survived three refuted hypotheses
/// precisely because the failure state could not say which exit path skipped
/// `run_teardown_bounded`. Compiled only for tests/test-utils; production
/// builds carry no trace state and pay no allocation.
#[cfg(any(test, feature = "test-utils"))]
#[derive(Debug, Clone)]
pub struct TerminateTraceEvent {
    pub instance_id: ModelInstanceId,
    pub step: String,
}

struct OrphanCleanupOwnershipGuard {
    inner: Arc<Inner>,
    process_record_id: ProcessOwnershipRecordId,
    generation: u64,
}

impl Drop for CleanupOwnershipGuard {
    fn drop(&mut self) {
        let mut registry = self.inner.registry.lock().expect("registry poisoned");
        let Some(handle) = registry.get_mut(&self.instance_id) else {
            return;
        };
        if handle.spawn_generation != self.spawn_generation {
            return;
        }
        let Some(cleanup) = handle.cleanup.as_mut() else {
            return;
        };
        if cleanup.owner_generation == self.generation {
            cleanup.in_progress = false;
            if *cleanup.owner_outcome_tx.borrow() == CleanupOwnerOutcome::InProgress {
                cleanup
                    .owner_outcome_tx
                    .send_replace(CleanupOwnerOutcome::Idle);
            }
        }
    }
}

impl Drop for OrphanCleanupOwnershipGuard {
    fn drop(&mut self) {
        let mut cleanups = self
            .inner
            .orphan_cleanups
            .lock()
            .expect("orphan cleanups poisoned");
        let Some(cleanup) = cleanups.get_mut(&self.process_record_id) else {
            return;
        };
        if cleanup.owner_generation == self.generation {
            cleanup.in_progress = false;
        }
    }
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

fn ensure_reserved_stop_recorded(
    result: Result<StopRecordOutcome, crate::process_ledger::ProcessLedgerError>,
) -> SwarmResult<()> {
    match result {
        Ok(StopRecordOutcome::Recorded | StopRecordOutcome::AlreadyStopped) => Ok(()),
        Ok(
            StopRecordOutcome::LeftOpenForReconciliation | StopRecordOutcome::DurabilityUnconfirmed,
        ) => Err(SwarmError::LedgerFailed(
            "reserved pidless lifecycle left START open for reconciliation".to_string(),
        )),
        Err(error) => Err(SwarmError::LedgerFailed(error.to_string())),
    }
}

/// Shared, lock-guarded inner state. Split out so the reaper task can hold an
/// `Arc` to exactly this without owning the whole coordinator.
struct Inner {
    /// Two-phase shutdown admission fence. A spawn increments the pre-register
    /// counter before doing synchronous admission work and decrements it only
    /// after PendingSpawn is visible. drain_all closes admission, waits for the
    /// counter to reach zero, then scans the published maps.
    spawn_admission_closed: AtomicBool,
    spawn_pre_registration: AtomicUsize,
    registry: Mutex<HashMap<ModelInstanceId, SessionHandle>>,
    pending_spawns: Mutex<HashMap<ModelInstanceId, PendingSpawn>>,
    routing_spawn_admissions: Mutex<HashMap<RoutingSpawnAdmissionKey, bool>>,
    managed_generations: Mutex<HashMap<(ModelInstanceId, u64), ManagedGenerationInvocation>>,
    orphan_cleanups: Mutex<HashMap<ProcessOwnershipRecordId, PendingOrphanCleanup>>,
    breaker: Mutex<FailureFingerprintBreaker>,
    /// Per-instance respawn counters (anti-storm) + budget accounting.
    accounting: Mutex<Accounting>,
    semaphore: Arc<Semaphore>,
    /// The LIVE model-session concurrency cap.
    ///
    /// `config.budget.max_concurrent` is the value the coordinator was BUILT
    /// with and never changes; this is the value currently in force. WP-1 MT-021
    /// AC-3 requires an operator-facing concurrency control bound to actual
    /// runtime behaviour, so the cap has to be adjustable without rebuilding the
    /// coordinator (which would drop every live session). Every place that
    /// reports or enforces "the cap" reads THIS, not the frozen config value, so
    /// the number the operator sees is the number the semaphore is honouring.
    effective_max_concurrent: AtomicUsize,
    /// The cap the operator last ASKED for, which may not be in force yet.
    ///
    /// Lowering is cooperative (running sessions are never killed), so a
    /// requested decrease can only be applied as fast as permits come free. The
    /// deficit has to be remembered somewhere or it is silently abandoned: the
    /// permits handed back by finishing sessions would return to the semaphore
    /// and the cap would settle at whatever intermediate value happened to be
    /// reachable at request time, NOT at what the operator asked for. This is
    /// that memory. `reconcile_concurrency_cap` drains it toward the target.
    desired_max_concurrent: AtomicUsize,
    /// rank-6 admission: bounds SIMULTANEOUS cold starts (factory.create / boot)
    /// separately from `semaphore` (run-concurrency). Held only during boot, so a
    /// burst of admitted spawns does not stampede the boot/networking layer.
    cold_start_semaphore: Arc<Semaphore>,
    /// Monotonic lifetime spawn counter — never decremented. The hard ceiling
    /// is `budget.max_lifetime_spawns` (HBR-SWARM-002 loop-cap semantics).
    lifetime_spawns: AtomicU64,
    checkout_lease_generation: AtomicU64,
    config: SwarmConfig,
    factory: Arc<dyn ModelSessionFactory>,
    sink: Arc<dyn SwarmEventSink>,
    ledger: LedgerBatcher,
    model_lane_store: Option<ModelLaneStore>,
    routing_execution_store: Option<super::routing_execution::ModelLaneRoutingExecutionStore>,
    /// Production fallback for externally owned processes whose normal
    /// teardown fails. Exact-process reclaim prevents a terminal lane from
    /// killing healthy siblings sharing the coordinator session.
    process_reclaimer: Mutex<Option<Arc<Reclaim>>>,
    dexterity_launch_required: bool,
    /// Diagnostic-only terminate() decision trace; see [`TerminateTraceEvent`].
    #[cfg(any(test, feature = "test-utils"))]
    terminate_trace_enabled: AtomicBool,
    #[cfg(any(test, feature = "test-utils"))]
    terminate_trace: Mutex<Vec<TerminateTraceEvent>>,
    #[cfg(any(test, feature = "test-utils"))]
    routing_after_launch_intent_pause:
        Mutex<Option<(Arc<tokio::sync::Notify>, Arc<tokio::sync::Notify>)>>,
    #[cfg(any(test, feature = "test-utils"))]
    routing_before_pending_registration_pause:
        Mutex<Option<(Arc<tokio::sync::Notify>, Arc<tokio::sync::Notify>)>>,
    #[cfg(any(test, feature = "test-utils"))]
    routing_before_dispatch_error_cleanup_pause:
        Mutex<Option<(Arc<tokio::sync::Notify>, Arc<tokio::sync::Notify>)>>,
    #[cfg(any(test, feature = "test-utils"))]
    routing_before_authority_request_commit_pause:
        Mutex<Option<(Arc<tokio::sync::Notify>, Arc<tokio::sync::Notify>)>>,
    #[cfg(any(test, feature = "test-utils"))]
    reaper_after_snapshot_pause:
        Mutex<Option<(Arc<tokio::sync::Notify>, Arc<tokio::sync::Notify>)>>,
    #[cfg(any(test, feature = "test-utils"))]
    cleanup_retry_after_snapshot_pause:
        Mutex<Option<(Arc<tokio::sync::Notify>, Arc<tokio::sync::Notify>)>>,
    #[cfg(any(test, feature = "test-utils"))]
    fail_next_routing_output_persistence: AtomicBool,
    #[cfg(any(test, feature = "test-utils"))]
    fail_next_sibling_cancellation_persistence: AtomicBool,
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

struct SpawnPreRegistrationGuard {
    inner: Arc<Inner>,
}

impl Drop for SpawnPreRegistrationGuard {
    fn drop(&mut self) {
        self.inner
            .spawn_pre_registration
            .fetch_sub(1, Ordering::SeqCst);
    }
}

struct PendingSpawnRegistration {
    inner: Arc<Inner>,
    instance_id: ModelInstanceId,
}

impl Drop for PendingSpawnRegistration {
    fn drop(&mut self) {
        self.inner
            .pending_spawns
            .lock()
            .expect("pending spawns poisoned")
            .remove(&self.instance_id);
    }
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
#[derive(Clone)]
pub struct SwarmCoordinator {
    inner: Arc<Inner>,
    reaper: Arc<ReaperHandle>,
}

struct ReaperHandle {
    task: Mutex<Option<JoinHandle<()>>>,
}

impl ReaperHandle {
    fn new() -> Self {
        Self {
            task: Mutex::new(None),
        }
    }
}

impl Drop for ReaperHandle {
    fn drop(&mut self) {
        if let Some(handle) = self.task.get_mut().ok().and_then(|task| task.take()) {
            handle.abort();
        }
    }
}

struct ManagedGenerationFinalizer {
    coordinator: Arc<SwarmCoordinator>,
    instance_id: ModelInstanceId,
    spawn_generation: u64,
    finalized: bool,
}

#[derive(Clone, Copy)]
enum ManagedGenerationOutcome {
    Completed,
    Failed,
    Cancelled,
    Dropped,
}

#[derive(Clone)]
struct ManagedGenerationDisposition {
    outcome: ManagedGenerationOutcome,
    error: Option<String>,
}

impl ManagedGenerationDisposition {
    fn completed() -> Self {
        Self {
            outcome: ManagedGenerationOutcome::Completed,
            error: None,
        }
    }

    fn failed(error: String) -> Self {
        Self {
            outcome: ManagedGenerationOutcome::Failed,
            error: Some(error),
        }
    }

    fn cancelled(reason: String) -> Self {
        Self {
            outcome: ManagedGenerationOutcome::Cancelled,
            error: Some(reason),
        }
    }

    fn dropped(reason: String) -> Self {
        Self {
            outcome: ManagedGenerationOutcome::Dropped,
            error: Some(reason),
        }
    }

    fn from_session_terminal(terminal: ModelSessionState, reason: String) -> Self {
        match terminal {
            ModelSessionState::Completed => Self::completed(),
            ModelSessionState::Failed => Self::failed(reason),
            ModelSessionState::Cancelled => Self::cancelled(reason),
            unexpected => Self::cancelled(format!(
                "terminal cleanup held unexpected {unexpected:?} disposition: {reason}"
            )),
        }
    }

    fn outcome_str(&self) -> &'static str {
        match self.outcome {
            ManagedGenerationOutcome::Completed => "completed",
            ManagedGenerationOutcome::Failed => "failed",
            ManagedGenerationOutcome::Cancelled => "cancelled",
            ManagedGenerationOutcome::Dropped => "dropped",
        }
    }
}

struct ManagedGenerationInvocation {
    trace_id: Uuid,
    run_id: String,
    session_id: String,
    generated_tokens: u64,
    disposition: Option<ManagedGenerationDisposition>,
    usage_committed: bool,
    terminal_emitted: bool,
}

const MANAGED_GENERATION_CALLER_CLEANUP_DEADLINE: Duration = Duration::from_millis(750);
const MANAGED_GENERATION_RECONCILIATION_DEADLINE: Duration = Duration::from_secs(5);

impl ManagedGenerationFinalizer {
    fn finalize_terminal(
        &self,
        fallback: ManagedGenerationDisposition,
    ) -> (
        Option<ModelSessionState>,
        Result<Option<ManagedGenerationDisposition>, String>,
    ) {
        self.coordinator.emit_managed_generation_terminal(
            self.instance_id,
            self.spawn_generation,
            fallback,
        )
    }

    fn release_terminal_authority(&self) {
        self.coordinator
            .release_managed_generation_authority(self.instance_id, self.spawn_generation);
    }

    fn record_token(&mut self) {
        self.coordinator
            .record_managed_generation_token(self.instance_id, self.spawn_generation);
    }

    async fn finish_ready(&mut self) -> Result<(), String> {
        if self.finalized {
            return Ok(());
        }
        self.finalized = true;
        let (state, terminal_result) =
            self.finalize_terminal(ManagedGenerationDisposition::completed());
        if let Err(error) = terminal_result {
            let reason = format!("terminal event persistence rejected: {error}");
            if state == Some(ModelSessionState::Generating) {
                let _ = self
                    .coordinator
                    .fail_session_generation(
                        self.instance_id,
                        self.spawn_generation,
                        reason.clone(),
                    )
                    .await;
            }
            return Err(reason);
        }
        self.release_terminal_authority();
        if state == Some(ModelSessionState::Generating) {
            let _ = self.coordinator.transition_generation(
                self.instance_id,
                self.spawn_generation,
                ModelSessionState::Ready,
            );
        }
        Ok(())
    }

    async fn finish_failure(&mut self, error: String) -> Result<(), String> {
        if self.finalized {
            return Ok(());
        }
        self.finalized = true;
        let (state, terminal_result) =
            self.finalize_terminal(ManagedGenerationDisposition::failed(error.clone()));
        if terminal_result.is_ok() {
            self.release_terminal_authority();
        }
        if state == Some(ModelSessionState::Generating) {
            let cleanup = self.coordinator.fail_session_generation(
                self.instance_id,
                self.spawn_generation,
                error.clone(),
            );
            if !matches!(
                tokio::time::timeout(MANAGED_GENERATION_CALLER_CLEANUP_DEADLINE, cleanup).await,
                Ok(Ok(()))
            ) {
                self.spawn_reconciliation(ModelSessionState::Failed, error);
            }
        }
        terminal_result.map(|_| ())
    }

    async fn finish_cancelled(&mut self, reason: String) -> Result<(), String> {
        if self.finalized {
            return Ok(());
        }
        self.finalized = true;
        let (state, terminal_result) =
            self.finalize_terminal(ManagedGenerationDisposition::cancelled(reason.clone()));
        if terminal_result.is_ok() {
            self.release_terminal_authority();
        }
        if state == Some(ModelSessionState::Generating) {
            let cleanup = self.coordinator.cancel_session_generation(
                self.instance_id,
                self.spawn_generation,
                reason.clone(),
            );
            if !matches!(
                tokio::time::timeout(MANAGED_GENERATION_CALLER_CLEANUP_DEADLINE, cleanup).await,
                Ok(Ok(()))
            ) {
                self.spawn_reconciliation(ModelSessionState::Cancelled, reason);
            }
        }
        terminal_result.map(|_| ())
    }

    fn spawn_reconciliation(&self, terminal: ModelSessionState, reason: String) {
        let coordinator = Arc::clone(&self.coordinator);
        let instance_id = self.instance_id;
        let spawn_generation = self.spawn_generation;
        let reconcile = async move {
            let result = match terminal {
                ModelSessionState::Failed => {
                    coordinator
                        .fail_session_generation(instance_id, spawn_generation, reason)
                        .await
                }
                ModelSessionState::Cancelled => {
                    coordinator
                        .cancel_session_generation(instance_id, spawn_generation, reason)
                        .await
                }
                _ => unreachable!("managed generation reconciles only failure/cancellation"),
            };
            match result {
                Ok(()) | Err(SwarmError::UnknownInstance(_)) => {}
                Err(error) => tracing::warn!(
                    target: "handshake_core::swarm_orchestration",
                    %instance_id,
                    %error,
                    "managed-generation cleanup remains pending after bounded reconciliation"
                ),
            }
        };
        tokio::spawn(async move {
            if tokio::time::timeout(MANAGED_GENERATION_RECONCILIATION_DEADLINE, reconcile)
                .await
                .is_err()
            {
                tracing::warn!(
                    target: "handshake_core::swarm_orchestration",
                    %instance_id,
                    "managed-generation cleanup reconciliation deadline elapsed; durable/local cleanup_pending state is retained for retry or restart recovery"
                );
            }
        });
    }
}

impl Drop for ManagedGenerationFinalizer {
    fn drop(&mut self) {
        if self.finalized {
            return;
        }
        self.finalized = true;
        let (state, terminal_result) = self.finalize_terminal(
            ManagedGenerationDisposition::dropped("managed generation stream dropped".to_string()),
        );
        if terminal_result.is_ok() {
            self.release_terminal_authority();
        }
        if let Err(error) = terminal_result {
            tracing::error!(
                target: "handshake_core::swarm_orchestration",
                %error,
                instance_id = %self.instance_id,
                "dropped generation terminal event was rejected; synchronous cancellation fence remains active"
            );
        }
        if state.is_none() {
            return;
        }
        let reason = "managed generation stream dropped";
        let fenced = {
            let mut registry = self
                .coordinator
                .inner
                .registry
                .lock()
                .expect("registry poisoned");
            let Some(handle) = registry.get_mut(&self.instance_id).filter(|handle| {
                handle.spawn_generation == self.spawn_generation
                    && handle.state == ModelSessionState::Generating
            }) else {
                return;
            };
            handle.state = ModelSessionState::Cancelling;
            handle.cleanup = Some(PendingSessionCleanup {
                terminal: ModelSessionState::Cancelled,
                reason: reason.to_string(),
                exit_code: -1,
                teardown_succeeded: false,
                stop_succeeded: false,
                owner_generation: 0,
                in_progress: false,
                owner_outcome_tx: watch::channel(CleanupOwnerOutcome::Idle).0,
                terminal_event_id: Uuid::now_v7(),
                resource_evicted_event_id: Uuid::now_v7(),
            });
            (handle.cancel.clone(), Arc::clone(&handle.runtime))
        };
        let _ = self
            .coordinator
            .inner
            .sink
            .emit(SwarmEvent::SessionStateChanged {
                instance_id: self.instance_id,
                from: ModelSessionState::Generating,
                to: ModelSessionState::Cancelling,
            });
        let (cancel, runtime_adapter) = fenced;
        cancel.cancel();
        runtime_adapter.cancel(cancel.clone());
        let coordinator = Arc::clone(&self.coordinator);
        let instance_id = self.instance_id;
        let spawn_generation = self.spawn_generation;
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            runtime.spawn(async move {
                if tokio::time::timeout(
                    MANAGED_GENERATION_RECONCILIATION_DEADLINE,
                    coordinator.cancel_session_generation(
                        instance_id,
                        spawn_generation,
                        reason,
                    ),
                )
                .await
                .is_err()
                {
                    tracing::warn!(
                        target: "handshake_core::swarm_orchestration",
                        %instance_id,
                        "managed-generation Drop cleanup deadline elapsed; cleanup_pending state is retained for restart recovery"
                    );
                }
            });
        } else {
            // A managed stream can be moved to and dropped on a plain worker
            // thread. The synchronous Cancelling + cleanup-intent fence above
            // prevents Ready resurrection immediately; this owned runtime
            // drives the existing durable terminate path without depending on
            // ambient Tokio state on the dropping thread.
            std::thread::spawn(move || {
                let cleanup_runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build();
                match cleanup_runtime {
                    Ok(cleanup_runtime) => {
                        let result = cleanup_runtime.block_on(tokio::time::timeout(
                            MANAGED_GENERATION_RECONCILIATION_DEADLINE,
                            coordinator.cancel_session_generation(
                                instance_id,
                                spawn_generation,
                                reason,
                            ),
                        ));
                        if result.is_err() {
                            tracing::warn!(
                                target: "handshake_core::swarm_orchestration",
                                %instance_id,
                                "no-Tokio managed-generation cleanup deadline elapsed; cleanup_pending state is retained for restart recovery"
                            );
                        }
                    }
                    Err(error) => {
                        tracing::error!(
                            target: "handshake_core::swarm_orchestration",
                            %error,
                            %instance_id,
                            "failed to construct no-Tokio managed-generation cleanup runtime; synchronous Cancelling fence remains for reconciliation"
                        );
                    }
                }
            });
        }
    }
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

/// Unforgeable in-process receipt for the manager-owned half of an
/// operator-requested subagent lane. The coordinator mints this only after the
/// no-OS lane has committed to ModelLaneStore, and requires it again when the
/// external SubagentManager submits a typed output payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorSubagentManagerLane {
    run_id: String,
    lane_id: String,
    owner_session: String,
    manager_receipt_ref: String,
}

fn reserves_host_committed_memory(request: &SpawnRequest) -> bool {
    matches!(request.provider, None | Some(ProviderKind::Local))
}

impl SwarmCoordinator {
    fn emit_event(&self, event: SwarmEvent) -> SwarmResult<()> {
        self.inner
            .sink
            .emit(event)
            .map_err(SwarmError::EventSinkFailed)
    }

    /// Record one terminate() decision point. The closure defers the step
    /// string so production builds (where this is a no-op) never format it.
    #[cfg(any(test, feature = "test-utils"))]
    fn trace_terminate(&self, instance_id: ModelInstanceId, step: impl FnOnce() -> String) {
        if !self.inner.terminate_trace_enabled.load(Ordering::Relaxed) {
            return;
        }
        self.inner
            .terminate_trace
            .lock()
            .expect("terminate trace poisoned")
            .push(TerminateTraceEvent {
                instance_id,
                step: step(),
            });
    }

    #[cfg(not(any(test, feature = "test-utils")))]
    #[inline(always)]
    fn trace_terminate(&self, _instance_id: ModelInstanceId, _step: impl FnOnce() -> String) {}

    /// Diagnostic accessor for the terminate() decision trace, in global
    /// interleaving order across all instances. Test-only.
    #[cfg(any(test, feature = "test-utils"))]
    pub fn terminate_trace_events(&self) -> Vec<TerminateTraceEvent> {
        self.inner
            .terminate_trace
            .lock()
            .expect("terminate trace poisoned")
            .clone()
    }

    /// Enable the MT-009 terminate decision trace only for an explicit
    /// diagnostic run. Default proof runs leave it disabled so its shared mutex
    /// cannot perturb cleanup-owner interleavings.
    #[cfg(any(test, feature = "test-utils"))]
    pub fn set_terminate_trace_enabled_for_test(&self, enabled: bool) {
        if enabled {
            self.inner
                .terminate_trace
                .lock()
                .expect("terminate trace poisoned")
                .clear();
        }
        self.inner
            .terminate_trace_enabled
            .store(enabled, Ordering::SeqCst);
    }

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
        // Inherit the lane store's account scope so routing-produced rows carry
        // the same ownership as directly recorded ones (HBR-PRIV-001).
        let routing_execution_store = model_lane_store
            .as_ref()
            .cloned()
            .map(super::routing_execution::ModelLaneRoutingExecutionStore::new);
        let inner = Arc::new(Inner {
            spawn_admission_closed: AtomicBool::new(false),
            spawn_pre_registration: AtomicUsize::new(0),
            registry: Mutex::new(HashMap::new()),
            pending_spawns: Mutex::new(HashMap::new()),
            routing_spawn_admissions: Mutex::new(HashMap::new()),
            managed_generations: Mutex::new(HashMap::new()),
            orphan_cleanups: Mutex::new(HashMap::new()),
            breaker: Mutex::new(breaker),
            accounting: Mutex::new(Accounting::default()),
            effective_max_concurrent: AtomicUsize::new(config.budget.max_concurrent.max(1)),
            desired_max_concurrent: AtomicUsize::new(config.budget.max_concurrent.max(1)),
            semaphore,
            cold_start_semaphore,
            lifetime_spawns: AtomicU64::new(0),
            checkout_lease_generation: AtomicU64::new(0),
            config,
            factory,
            sink,
            ledger,
            model_lane_store,
            routing_execution_store,
            process_reclaimer: Mutex::new(None),
            dexterity_launch_required,
            #[cfg(any(test, feature = "test-utils"))]
            terminate_trace_enabled: AtomicBool::new(false),
            #[cfg(any(test, feature = "test-utils"))]
            terminate_trace: Mutex::new(Vec::new()),
            #[cfg(any(test, feature = "test-utils"))]
            routing_after_launch_intent_pause: Mutex::new(None),
            #[cfg(any(test, feature = "test-utils"))]
            routing_before_pending_registration_pause: Mutex::new(None),
            #[cfg(any(test, feature = "test-utils"))]
            routing_before_dispatch_error_cleanup_pause: Mutex::new(None),
            #[cfg(any(test, feature = "test-utils"))]
            routing_before_authority_request_commit_pause: Mutex::new(None),
            #[cfg(any(test, feature = "test-utils"))]
            reaper_after_snapshot_pause: Mutex::new(None),
            #[cfg(any(test, feature = "test-utils"))]
            cleanup_retry_after_snapshot_pause: Mutex::new(None),
            #[cfg(any(test, feature = "test-utils"))]
            fail_next_routing_output_persistence: AtomicBool::new(false),
            #[cfg(any(test, feature = "test-utils"))]
            fail_next_sibling_cancellation_persistence: AtomicBool::new(false),
        });
        Self {
            inner,
            reaper: Arc::new(ReaperHandle::new()),
        }
    }

    pub fn with_process_reclaimer(self, reclaimer: Arc<Reclaim>) -> Self {
        *self
            .inner
            .process_reclaimer
            .lock()
            .expect("process reclaimer lock poisoned") = Some(reclaimer);
        self
    }

    /// Start the single background reaper task. Idempotent: a second call is a
    /// no-op while a reaper is already running.
    pub fn start_reaper(&self) {
        let mut guard = self.reaper.task.lock().expect("reaper lock poisoned");
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
        if let Some(handle) = self
            .reaper
            .task
            .lock()
            .expect("reaper lock poisoned")
            .take()
        {
            handle.abort();
        }
    }

    /// THE single spawn entrypoint. Enforces, in order: breaker ADMISSION gate
    /// -> budget (token/cost) -> lifetime spawn ceiling -> concurrency permit ->
    /// factory create -> atomic dedup + registry insert + lease + events. Any
    /// bound failure returns a typed error and the partially-acquired resources
    /// are released.
    pub async fn spawn_session(&self, request: SpawnRequest) -> SwarmResult<ModelInstanceId> {
        self.spawn_session_with_generation(request)
            .await
            .map(|(instance_id, _)| instance_id)
    }

    async fn spawn_session_with_generation(
        &self,
        mut request: SpawnRequest,
    ) -> SwarmResult<(ModelInstanceId, u64)> {
        let inner = Arc::clone(&self.inner);
        let instance_id = request.instance_id;
        if inner.spawn_admission_closed.load(Ordering::SeqCst) {
            return Err(SwarmError::LedgerFailed(
                "spawn rejected: coordinator is draining".to_string(),
            ));
        }
        inner.spawn_pre_registration.fetch_add(1, Ordering::SeqCst);
        let pre_registration = SpawnPreRegistrationGuard {
            inner: Arc::clone(&inner),
        };
        // Close the read->increment race: if drain closed admission after the
        // first read, this spawn must retire its counter and never publish.
        if inner.spawn_admission_closed.load(Ordering::SeqCst) {
            return Err(SwarmError::LedgerFailed(
                "spawn rejected: coordinator is draining".to_string(),
            ));
        }

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
                self.emit_spawn_rejected(instance_id, "breaker_open")?;
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
            self.emit_spawn_rejected(instance_id, &format!("budget:{dimension}"))?;
            return Err(SwarmError::BudgetExhausted { dimension });
        }
        let committed_memory_bytes = if !reserves_host_committed_memory(&request) {
            0
        } else if inner.config.budget.max_committed_memory_bytes.is_some() {
            match request.committed_memory_bytes {
                Some(bytes) => bytes,
                None => {
                    let dimension = "committed_memory_unestimated".to_string();
                    self.emit_spawn_rejected(instance_id, &format!("budget:{dimension}"))?;
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
                    self.emit_spawn_rejected(instance_id, &format!("budget:{dimension}"))?;
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
            self.emit_spawn_rejected(instance_id, "lifetime_ceiling")?;
            return Err(SwarmError::LifetimeSpawnCeilingReached {
                spawned: reserved.saturating_sub(1),
                ceiling,
            });
        }

        // (3) Concurrency permit. try_acquire so an over-cap spawn returns a
        // typed error immediately rather than blocking forever.
        //
        // Reconcile FIRST: a pending lowering can only take permits that are
        // free, so permits released since the request must be retired here,
        // before this spawn could be admitted on one of them.
        // Reconcile FIRST: a pending lowering can only take permits that are
        // free, so permits released since the request must be retired here,
        // before this spawn could be admitted on one of them.
        inner.reconcile_concurrency_cap();
        let permit = match inner.semaphore.clone().try_acquire_owned() {
            Ok(p) => p,
            Err(_) => {
                let cap = inner.effective_max_concurrent.load(Ordering::SeqCst);
                self.emit_spawn_rejected(instance_id, "concurrency_cap")?;
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

        // Register cancellation authority before the first await in this
        // spawn. Routing commits its durable launch intent immediately before
        // calling this function; publishing PendingSpawn synchronously here
        // closes the post-intent/pre-preflight window in which cancellation
        // previously had no local token to fence.
        let checkout_generation = inner
            .checkout_lease_generation
            .fetch_add(1, Ordering::SeqCst)
            + 1;
        let create_cancel = CancellationToken::new();
        let revoke_fence = Arc::new(AtomicBool::new(false));
        #[cfg(any(test, feature = "test-utils"))]
        let routing_pause = {
            inner
                .routing_before_pending_registration_pause
                .lock()
                .expect("routing pre-registration pause poisoned")
                .take()
        };
        #[cfg(any(test, feature = "test-utils"))]
        if let Some((arrived, release)) = routing_pause {
            arrived.notify_one();
            release.notified().await;
        }
        {
            // Lock order is routing admission -> pending spawn. Cancellation
            // marks admissions without taking the pending lock, releases it,
            // then scans pending/live. Therefore either it marks this owner
            // before publication (and this spawn fails closed), or publication
            // wins and its exact token is visible to the cancellation scan.
            let routing_key = request
                .routing_attempt
                .clone()
                .map(|identity| (identity, instance_id));
            let mut routing_admissions = inner
                .routing_spawn_admissions
                .lock()
                .expect("routing spawn admissions poisoned");
            if let Some(key) = routing_key.as_ref() {
                match routing_admissions.get(key) {
                    Some(false) => {}
                    Some(true) => {
                        return Err(SwarmError::LedgerFailed(format!(
                            "routing attempt was cancelled before pending spawn registration for {instance_id}"
                        )));
                    }
                    None => {
                        return Err(SwarmError::LedgerFailed(format!(
                            "routing spawn admission disappeared before pending registration for {instance_id}"
                        )));
                    }
                }
            }
            let mut pending = inner
                .pending_spawns
                .lock()
                .expect("pending spawns poisoned");
            let dexterity_lane_id = request
                .dexterity_launch
                .as_ref()
                .map(|launch| launch.lane_id.clone());
            let dexterity_consent_receipt_id = request
                .dexterity_launch
                .as_ref()
                .and_then(|launch| launch.consent_receipt_ref.clone());
            match pending.entry(instance_id) {
                Entry::Occupied(_) => {
                    drop(permit);
                    return Err(SwarmError::DuplicateInstance(instance_id));
                }
                Entry::Vacant(entry) => {
                    entry.insert(PendingSpawn {
                        cancel: create_cancel.clone(),
                        spawn_generation: checkout_generation,
                        routing_attempt: request.routing_attempt.clone(),
                        dexterity_lane_id,
                        dexterity_consent_receipt_id,
                        checkout_lease: None,
                        revoke_fence: revoke_fence.clone(),
                    });
                }
            }
            if let Some(key) = routing_key.as_ref() {
                routing_admissions.remove(key);
            }
        }
        let _pending_registration = PendingSpawnRegistration {
            inner: Arc::clone(&inner),
            instance_id,
        };
        drop(pre_registration);

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
        // Exclusive checkout ownership is acquired after every pure/durable
        // preflight but before factory.create crosses a runtime side-effect
        // boundary. The guard is first owned by pending-spawn state, then moves
        // into the live SessionHandle and survives until teardown + STOP.
        let checkout_lease = CheckoutLeaseGuard::acquire(&request, checkout_generation)?;
        request.checkout_lease = checkout_lease
            .as_ref()
            .map(|guard| guard.lease_ref().clone());
        {
            let mut pending = inner
                .pending_spawns
                .lock()
                .expect("pending spawns poisoned");
            let spawn = pending
                .get_mut(&instance_id)
                .ok_or(SwarmError::UnknownInstance(instance_id))?;
            spawn.checkout_lease = checkout_lease;
        }
        let (create_result, cancelled_during_create, create_abandoned) = {
            let _boot_permit = inner
                .cold_start_semaphore
                .clone()
                .acquire_owned()
                .await
                .expect("cold-start semaphore is never closed");
            let create = inner.factory.create(&request);
            tokio::pin!(create);
            tokio::select! {
                biased;
                _ = create_cancel.cancelled() => {
                    // `create` may already have crossed an external side-effect
                    // boundary. Do not drop it IMMEDIATELY: give it a bounded
                    // grace period to resolve so any resulting live session can
                    // be compensated before we acknowledge cancellation.
                    //
                    // The grace is BOUNDED (teardown_timeout, the same budget
                    // that bounds every other unwind) because an unbounded
                    // `create.await` here means a factory that never returns
                    // hangs the cancellation itself - and cancel_routing_execution
                    // awaits cancel_session per live instance, so ONE stuck
                    // create wedges the operator's cancel forever. That is worse
                    // than a possible orphan: an orphan is recoverable through
                    // the process-ledger reclaim path, a hung cancel is not
                    // recoverable by the operator at all.
                    match tokio::time::timeout(inner.config.teardown_timeout, &mut create).await {
                        Ok(result) => (result, true, false),
                        Err(_) => (
                            Err(SwarmError::FactoryFailed(format!(
                                "factory create did not resolve within teardown_timeout after cancellation for {instance_id}; create abandoned, any external side effect is left to process-ledger reclaim"
                            ))),
                            true,
                            true,
                        ),
                    }
                },
                result = &mut create => (result, false, false),
            }
        };
        if create_abandoned {
            inner
                .factory
                .cancel_pending_create(&request)
                .await
                .map_err(|cleanup_error| {
                    SwarmError::FactoryFailed(format!(
                        "spawn cancelled while factory create was pending for {instance_id}; pending-create compensation failed: {cleanup_error}"
                    ))
                })?;
        }
        let create_result = match (cancelled_during_create, create_result) {
            (false, result) => result,
            (true, Err(_)) => {
                if !create_abandoned {
                    inner
                        .factory
                        .cancel_pending_create(&request)
                        .await
                        .map_err(|cleanup_error| {
                            SwarmError::FactoryFailed(format!(
                                "spawn cancelled while factory create was pending for {instance_id}; pending-create compensation failed: {cleanup_error}"
                            ))
                        })?;
                }
                if revoke_fence.load(Ordering::SeqCst) {
                    return Err(SwarmError::LedgerFailed(format!(
                        "CX-MM-007 cloud consent revoked mid-flight; durable lane insert fenced closed for {instance_id}"
                    )));
                }
                return Err(SwarmError::FactoryFailed(format!(
                    "spawn cancelled while factory create was pending for {instance_id}"
                )));
            }
            (true, Ok(live)) => {
                let checkout_lease = match self.take_pending_checkout_lease(instance_id) {
                    Ok(checkout_lease) => checkout_lease,
                    Err(transfer_error) => {
                        return self
                            .rollback_unregistered_after_factory_create(
                                &request,
                                live,
                                None,
                                permit,
                                committed_memory_reservation,
                                "spawn_cancelled_checkout_lease_transfer_failed",
                                transfer_error,
                            )
                            .await
                            .map(|instance_id| (instance_id, checkout_generation));
                    }
                };
                // If the cancellation was caused by a mid-flight cloud consent
                // revocation, the durable lane insert this spawn was about to
                // perform would fail closed under CX-MM-007 (the consent receipt
                // is durably fenced/revoked). Surface that consent-revoked error
                // rather than the generic factory-cancellation error, so callers
                // see the true lane-bound-consent-no-longer-valid cause. The
                // compensation itself (factory unload + token cancellation + no
                // lane row) is identical either way.
                let (cleanup_reason, primary_error) = if revoke_fence.load(Ordering::SeqCst) {
                    (
                        "spawn_cancelled_after_factory_create_consent_revoked",
                        SwarmError::LedgerFailed(format!(
                            "CX-MM-007 cloud consent revoked mid-flight; durable lane insert fenced closed for {instance_id}"
                        )),
                    )
                } else {
                    (
                        "spawn_cancelled_after_factory_create",
                        SwarmError::FactoryFailed(format!(
                            "spawn cancelled after factory create compensation for {instance_id}"
                        )),
                    )
                };
                return self
                    .rollback_unregistered_after_factory_create(
                        &request,
                        live,
                        checkout_lease,
                        permit,
                        committed_memory_reservation,
                        cleanup_reason,
                        primary_error,
                    )
                    .await
                    .map(|instance_id| (instance_id, checkout_generation));
            }
        };
        match create_result {
            Ok(live) => {
                let checkout_lease = match self.take_pending_checkout_lease(instance_id) {
                    Ok(checkout_lease) => checkout_lease,
                    Err(transfer_error) => {
                        return self
                            .rollback_unregistered_after_factory_create(
                                &request,
                                live,
                                None,
                                permit,
                                committed_memory_reservation,
                                "checkout_lease_transfer_failed_after_factory_create",
                                transfer_error,
                            )
                            .await
                            .map(|instance_id| (instance_id, checkout_generation));
                    }
                };
                let dexterity_launch_records = if request.dexterity_launch.is_some() {
                    match build_successful_launch_records(&request, &live) {
                        Ok(records) => Some(records),
                        Err(err) => {
                            return self
                                .rollback_unregistered_after_factory_create(
                                    &request,
                                    live,
                                    checkout_lease,
                                    permit,
                                    committed_memory_reservation,
                                    "dexterity_launch_record_preparation_failed",
                                    SwarmError::LedgerFailed(format!(
                                        "Dexterity launch record preparation failed: {err}"
                                    )),
                                )
                                .await
                                .map(|instance_id| (instance_id, checkout_generation));
                        }
                    }
                } else {
                    None
                };
                // (4a) ATOMIC dedup + insert (D2). Hold the registry lock across
                // BOTH the duplicate check and the insert so two concurrent
                // spawns of the same instance_id cannot both record a START and
                // silently drop the first handle. If a live instance already
                // exists, roll back this spawn's permit + committed-memory and
                // lifetime reservations
                // AND fully tear down the just-created session (cancel +
                // teardown + ledger STOP) so the loser leaves no orphan START.
                let inserted = self.try_insert_loading_with_memory_handoff(
                    &request,
                    live,
                    permit,
                    checkout_lease,
                    committed_memory_reservation,
                    checkout_generation,
                );
                match inserted {
                    Ok(()) => {
                        if let Some(records) = dexterity_launch_records {
                            let Some(store) = inner.model_lane_store.as_ref() else {
                                if let Err(cleanup_err) = self
                                    .terminate(
                                        instance_id,
                                        ModelSessionState::Cancelled,
                                        "dexterity_model_lane_store_missing",
                                        -1,
                                    )
                                    .await
                                {
                                    return Err(SwarmError::LedgerFailed(format!(
                                        "Dexterity ModelLaneStore missing and spawned-session cleanup failed: {cleanup_err}"
                                    )));
                                }
                                return Err(SwarmError::LedgerFailed(
                                    "Dexterity launch contract requires ModelLaneStore".into(),
                                ));
                            };
                            if let Err(err) = store.record_prepared_launch(records).await {
                                if let Err(cleanup_err) = self
                                    .terminate(
                                        instance_id,
                                        ModelSessionState::Cancelled,
                                        "dexterity_model_lane_record_failed",
                                        -1,
                                    )
                                    .await
                                {
                                    return Err(SwarmError::LedgerFailed(format!(
                                        "Dexterity launch persistence failed ({err}); spawned-session cleanup also failed: {cleanup_err}"
                                    )));
                                }
                                return Err(SwarmError::LedgerFailed(format!(
                                    "Dexterity launch record failed: {err}"
                                )));
                            }
                            if let Err(err) = self.mark_dexterity_model_lane_persisted(instance_id)
                            {
                                if let Err(cleanup_err) = self
                                    .terminate(
                                        instance_id,
                                        ModelSessionState::Cancelled,
                                        "dexterity_model_lane_persist_marker_failed",
                                        -1,
                                    )
                                    .await
                                {
                                    return Err(SwarmError::LedgerFailed(format!(
                                        "Dexterity persistence marker failed ({err}); spawned-session cleanup also failed: {cleanup_err}"
                                    )));
                                }
                                return Err(err);
                            }
                        }
                        if let Err(err) = self.transition(instance_id, ModelSessionState::Ready) {
                            if let Err(cleanup_err) = self
                                .terminate(
                                    instance_id,
                                    ModelSessionState::Cancelled,
                                    "dexterity_ready_transition_failed",
                                    -1,
                                )
                                .await
                            {
                                return Err(SwarmError::LedgerFailed(format!(
                                    "ready transition failed ({err}); spawned-session cleanup also failed: {cleanup_err}"
                                )));
                            }
                            return Err(err);
                        }
                        if let Err(err) = self.commit_ready_hook(instance_id) {
                            if let Err(cleanup_err) = self
                                .terminate(
                                    instance_id,
                                    ModelSessionState::Cancelled,
                                    "ready_hook_commit_failed",
                                    -1,
                                )
                                .await
                            {
                                return Err(SwarmError::LedgerFailed(format!(
                                    "ready hook failed ({err}); spawned-session cleanup also failed: {cleanup_err}"
                                )));
                            }
                            return Err(err);
                        }
                        lifetime_reservation.disarm();
                        // Heal the breaker for this instance's tracked signature
                        // on a real success (C4). A signature that tripped
                        // recovers after a genuine success, not only cooldown.
                        inner.heal_breaker_for_instance(instance_id);
                        let permits_cap = inner.effective_max_concurrent.load(Ordering::SeqCst);
                        let permits_in_use =
                            permits_cap.saturating_sub(inner.semaphore.available_permits());
                        if let Err(event_error) = inner.sink.emit(SwarmEvent::ResourceAllocated {
                            instance_id,
                            permits_in_use,
                            permits_cap,
                        }) {
                            let cleanup = self
                                .cancel_session_generation(
                                    instance_id,
                                    checkout_generation,
                                    "resource allocation event persistence failed",
                                )
                                .await;
                            return match cleanup {
                                Ok(()) => Err(SwarmError::EventSinkFailed(event_error)),
                                Err(cleanup_error) => Err(SwarmError::LedgerFailed(format!(
                                    "resource allocation event persistence failed ({event_error}); exact spawned-session cleanup also failed: {cleanup_error}"
                                ))),
                            };
                        }
                        Ok((instance_id, checkout_generation))
                    }
                    Err((
                        TryInsertLoadingError::Duplicate {
                            live,
                            permit,
                            checkout_lease,
                        },
                        committed_memory_reservation,
                    )) => self
                        .rollback_duplicate_after_factory_create(
                            &request,
                            live,
                            permit,
                            checkout_lease,
                            committed_memory_reservation,
                        )
                        .await
                        .map(|instance_id| (instance_id, checkout_generation)),
                    Err((
                        TryInsertLoadingError::EventSink {
                            live,
                            permit,
                            checkout_lease,
                            error,
                        },
                        committed_memory_reservation,
                    )) => self
                        .rollback_spawn_event_failure_after_factory_create(
                            &request,
                            live,
                            permit,
                            checkout_lease,
                            committed_memory_reservation,
                            error,
                        )
                        .await
                        .map(|instance_id| (instance_id, checkout_generation)),
                }
            }
            Err(err) => {
                let err = match inner.factory.cancel_pending_create(&request).await {
                    Ok(()) => err,
                    Err(cleanup_error) => SwarmError::FactoryFailed(format!(
                        "factory create failed for {instance_id} ({err}); pending-create compensation also failed: {cleanup_error}"
                    )),
                };
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
                        inner
                            .sink
                            .emit(SwarmEvent::BreakerTripped {
                                signature: fp.to_string(),
                                consecutive_failures: consecutive,
                            })
                            .map_err(SwarmError::EventSinkFailed)?;
                    }
                    inner
                        .sink
                        .emit(SwarmEvent::SessionFailed {
                            instance_id,
                            error: err.to_string(),
                            event_id: None,
                        })
                        .map_err(SwarmError::EventSinkFailed)?;
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
    /// same embedded-Surreal authority path as process-backed launches.
    /// Launch a cloud batch under one run-scoped consent receipt. Every request
    /// must share the run and receipt, and every request is preflighted before
    /// the first factory call.
    pub async fn spawn_cloud_consent_batch(
        &self,
        requests: Vec<SpawnRequest>,
    ) -> SwarmResult<Vec<ModelInstanceId>> {
        if requests.len() < 2 {
            return Err(SwarmError::LedgerFailed(
                "CX-MM-007 SingleRun cloud batch requires at least two targets".into(),
            ));
        }
        let store = self.inner.model_lane_store.as_ref().ok_or_else(|| {
            SwarmError::LedgerFailed(
                "CX-MM-007 SingleRun cloud batch requires ModelLaneStore".into(),
            )
        })?;
        let first_contract = requests[0].dexterity_launch.as_ref().ok_or_else(|| {
            SwarmError::LedgerFailed(
                "CX-MM-007 SingleRun cloud batch request lacks Dexterity contract".into(),
            )
        })?;
        let consent_receipt_id = first_contract.consent_receipt_ref.clone().ok_or_else(|| {
            SwarmError::LedgerFailed("CX-MM-007 SingleRun cloud batch lacks consent receipt".into())
        })?;
        let run_id = first_contract.run_id.clone();
        for request in &requests {
            if request.provider != Some(ProviderKind::ByokCloud) {
                return Err(SwarmError::LedgerFailed(
                    "CX-MM-007 SingleRun batch contains a non-cloud request".into(),
                ));
            }
            let contract = request.dexterity_launch.as_ref().ok_or_else(|| {
                SwarmError::LedgerFailed(
                    "CX-MM-007 SingleRun cloud batch request lacks Dexterity contract".into(),
                )
            })?;
            if contract.run_id != run_id
                || contract.consent_receipt_ref.as_deref() != Some(consent_receipt_id.as_str())
            {
                return Err(SwarmError::LedgerFailed(
                    "CX-MM-007 SingleRun batch requests must share run and receipt authority"
                        .into(),
                ));
            }
            store
                .preflight_cloud_spawn_request(request)
                .await
                .map_err(|error| {
                    SwarmError::LedgerFailed(format!(
                        "CX-MM-007 SingleRun batch cloud preflight failed: {error}"
                    ))
                })?;
            DexterityLaunchAdapterRegistry::standard()
                .preflight_spawn_request(request)
                .map_err(|error| {
                    SwarmError::LedgerFailed(format!(
                        "CX-MM-007 SingleRun batch adapter preflight failed: {error}"
                    ))
                })?;
        }
        let mut launched = Vec::with_capacity(requests.len());
        for request in requests {
            match self.spawn_session(request).await {
                Ok(instance_id) => launched.push(instance_id),
                Err(error) => {
                    for instance_id in launched.iter().copied() {
                        let _ = self
                            .cancel_session(instance_id, "SingleRun batch compensation")
                            .await;
                    }
                    return Err(error);
                }
            }
        }
        Ok(launched)
    }

    /// Revoke cloud consent under coordinator ownership so durable cancellation
    /// evidence corresponds to real token cancellation, teardown, and eviction.
    pub async fn revoke_cloud_consent_receipt(
        &self,
        consent_receipt_id: &str,
        revoked_by_ref: &str,
        reason: &str,
    ) -> SwarmResult<Vec<ModelLaneRecord>> {
        let store = self.inner.model_lane_store.as_ref().ok_or_else(|| {
            SwarmError::LedgerFailed(
                "CX-MM-007 cloud consent revoke requires ModelLaneStore".into(),
            )
        })?;
        // Fence under the same receipt advisory authority used by launch
        // persistence before taking an ephemeral runtime snapshot. A launch that
        // commits first is included; a launch that loses the fence re-preflights
        // as revoked. Identical retries rediscover the same canonical lane set.
        let covered_lanes = store
            .fence_cloud_consent_revocation(consent_receipt_id, revoked_by_ref, reason)
            .await
            .map_err(|error| SwarmError::LedgerFailed(error.to_string()))?;
        let target_lane_ids = covered_lanes
            .iter()
            .map(|lane| lane.lane_id.clone())
            .collect::<std::collections::BTreeSet<_>>();
        let cancel_reason = format!("CX-MM-007 cloud consent revoked: {reason}");
        let pending = {
            self.inner
                .pending_spawns
                .lock()
                .expect("pending spawns poisoned")
                .values()
                .filter(|pending| {
                    pending.dexterity_consent_receipt_id.as_deref() == Some(consent_receipt_id)
                        || pending
                            .dexterity_lane_id
                            .as_ref()
                            .is_some_and(|lane_id| target_lane_ids.contains(lane_id))
                })
                .map(|pending| (pending.cancel.clone(), pending.revoke_fence.clone()))
                .collect::<Vec<_>>()
        };
        for (cancel, revoke_fence) in pending {
            // Raise the fence before cancelling so the in-flight spawn observes
            // the consent-revoked cause during factory-create compensation and
            // surfaces CX-MM-007. The durable consent fence is already committed
            // above (fence_cloud_consent_revocation), so this flag truthfully
            // reflects that the post-factory durable lane insert would fail
            // closed under CX-MM-007.
            revoke_fence.store(true, Ordering::SeqCst);
            cancel.cancel();
        }
        let live = {
            let mut registry = self.inner.registry.lock().expect("registry poisoned");
            registry
                .values_mut()
                .filter_map(|handle| {
                    let covered = handle.dexterity_consent_receipt_id.as_deref()
                        == Some(consent_receipt_id)
                        || handle
                            .dexterity_lane_id
                            .as_ref()
                            .is_some_and(|lane_id| target_lane_ids.contains(lane_id));
                    if !covered || handle.state.is_terminal() {
                        return None;
                    }
                    if handle.state != ModelSessionState::Cancelling {
                        handle.state = ModelSessionState::Cancelling;
                        handle.cleanup = Some(PendingSessionCleanup {
                            terminal: ModelSessionState::Cancelled,
                            reason: cancel_reason.clone(),
                            exit_code: -1,
                            teardown_succeeded: false,
                            stop_succeeded: false,
                            owner_generation: 0,
                            in_progress: false,
                            owner_outcome_tx: watch::channel(CleanupOwnerOutcome::Idle).0,
                            terminal_event_id: Uuid::now_v7(),
                            resource_evicted_event_id: Uuid::now_v7(),
                        });
                    }
                    // Specialized finalization below owns the terminal lane row.
                    handle.dexterity_model_lane_persisted = false;
                    Some((handle.instance_id, handle.dexterity_lane_id.clone()))
                })
                .collect::<Vec<_>>()
        };
        let mut provider_cancelled_lane_ids = std::collections::BTreeSet::new();
        for (instance_id, lane_id) in live {
            match self
                .cancel_session(instance_id, cancel_reason.clone())
                .await
            {
                Ok(()) => {
                    if let Some(lane_id) = lane_id {
                        provider_cancelled_lane_ids.insert(lane_id);
                    }
                }
                Err(SwarmError::UnknownInstance(_)) => {}
                Err(error) => return Err(error),
            }
        }

        // If runtime cleanup completed but terminal finalization failed, the
        // live handle is already evicted. Recover the truthful cancellation
        // outcome from the existing durable cleanup receipt on identical retry.
        for lane in &covered_lanes {
            if provider_cancelled_lane_ids.contains(&lane.lane_id) {
                continue;
            }
            if let Some(instance_id) = lane.model_session_id.strip_prefix("swarm-session:") {
                let completed = store
                    .session_cleanup_completed(instance_id, "Cancelled", &cancel_reason)
                    .await
                    .map_err(|error| SwarmError::LedgerFailed(error.to_string()))?;
                if completed {
                    provider_cancelled_lane_ids.insert(lane.lane_id.clone());
                }
            }
        }

        store
            .finalize_cloud_consent_revocation(
                consent_receipt_id,
                revoked_by_ref,
                reason,
                &provider_cancelled_lane_ids,
            )
            .await
            .map_err(|error| SwarmError::LedgerFailed(error.to_string()))
    }

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
    /// atomic embedded-Surreal ModelLane/EventLedger authority write.
    pub async fn launch_operator_subagent_model_lane(
        &self,
        request: DexterityLaunchAdapterRequest,
    ) -> SwarmResult<(
        ModelLaneRunRecord,
        ModelLaneRecord,
        OperatorSubagentManagerLane,
    )> {
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
        let (run, lane) = store.record_prepared_launch(records).await.map_err(|err| {
            SwarmError::LedgerFailed(format!("Dexterity subagent launch record failed: {err}"))
        })?;
        let manager = OperatorSubagentManagerLane {
            run_id: run.run_id.clone(),
            lane_id: lane.lane_id.clone(),
            owner_session: lane.owner_session.clone(),
            manager_receipt_ref: format!(
                "subagent-manager-receipt://{}/{}/{}",
                run.run_id,
                lane.lane_id,
                uuid::Uuid::now_v7()
            ),
        };
        Ok((run, lane, manager))
    }

    /// Persist one SubagentManager-produced output and its ArtifactStore
    /// binding atomically. Direct ModelLaneStore insertion is deliberately not
    /// the manager ingress: the receipt, lane kind, no-OS ownership contract,
    /// run/owner identity, and terminal fence are checked here first.
    pub async fn record_operator_subagent_manager_output(
        &self,
        manager: &OperatorSubagentManagerLane,
        mut message: NewModelLaneMessage,
        binding: NewModelLaneContextBundleArtifactBinding,
    ) -> SwarmResult<ModelLaneMessageRecord> {
        if message.run_id != manager.run_id
            || message.from_lane_id != manager.lane_id
            || message.owner_session != manager.owner_session
            || binding.run_id != manager.run_id
            || binding.owner_session != manager.owner_session
        {
            return Err(SwarmError::LedgerFailed(
                "SubagentManager output does not match its coordinator-issued lane receipt".into(),
            ));
        }
        let Some(store) = self.inner.model_lane_store.as_ref() else {
            return Err(SwarmError::LedgerFailed(
                "SubagentManager output requires ModelLaneStore".into(),
            ));
        };
        let projection = store
            .navigation_by_lane(&manager.lane_id)
            .await
            .map_err(|err| {
                SwarmError::LedgerFailed(format!(
                    "SubagentManager lane {} lookup failed: {err}",
                    manager.lane_id
                ))
            })?;
        let lane = projection.lanes.first().ok_or_else(|| {
            SwarmError::LedgerFailed(format!(
                "SubagentManager lane {} is missing",
                manager.lane_id
            ))
        })?;
        if lane.run_id != manager.run_id
            || lane.kind != ModelLaneKind::Subagent
            || lane.runtime_binding != ModelLaneRuntimeBinding::Subagent
            || lane.launch_authority != LaunchAuthority::SubagentManager
            || lane.process_ownership_ref.is_some()
            || lane.no_os_process_reason_ref.is_none()
        {
            return Err(SwarmError::LedgerFailed(format!(
                "lane {} is not a coordinator-owned no-OS SubagentManager lane",
                manager.lane_id
            )));
        }
        let diagnostic = message.diagnostic_payload.as_object_mut().ok_or_else(|| {
            SwarmError::LedgerFailed(
                "SubagentManager output diagnostic_payload must be an object".into(),
            )
        })?;
        diagnostic.insert(
            "subagent_manager_receipt_ref".into(),
            serde_json::Value::String(manager.manager_receipt_ref.clone()),
        );
        store
            .record_message_with_payload_binding(message, binding)
            .await
            .map_err(|err| {
                SwarmError::LedgerFailed(format!(
                    "SubagentManager output persistence failed: {err}"
                ))
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
        self.emit_event(SwarmEvent::SessionStateChanged {
            instance_id,
            from,
            to,
        })?;
        if to == ModelSessionState::Ready {
            self.emit_event(SwarmEvent::SessionReady { instance_id })?;
        }
        Ok(())
    }

    fn transition_generation(
        &self,
        instance_id: ModelInstanceId,
        spawn_generation: u64,
        to: ModelSessionState,
    ) -> SwarmResult<()> {
        let from = {
            let mut registry = self.inner.registry.lock().expect("registry poisoned");
            let Some(handle) = registry.get_mut(&instance_id) else {
                return Ok(());
            };
            if handle.spawn_generation != spawn_generation {
                return Ok(());
            }
            let from = handle.state;
            if !from.can_transition(to) {
                return Err(SwarmError::InvalidStateTransition { from, to });
            }
            handle.state = to;
            from
        };
        self.emit_event(SwarmEvent::SessionStateChanged {
            instance_id,
            from,
            to,
        })?;
        if to == ModelSessionState::Ready {
            self.emit_event(SwarmEvent::SessionReady { instance_id })?;
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

    async fn complete_session_generation(
        &self,
        instance_id: ModelInstanceId,
        spawn_generation: u64,
    ) -> SwarmResult<()> {
        self.terminate_generation(
            instance_id,
            spawn_generation,
            ModelSessionState::Completed,
            "completed",
            0,
        )
        .await
    }

    pub async fn fail_session(
        &self,
        instance_id: ModelInstanceId,
        error: impl Into<String>,
    ) -> SwarmResult<()> {
        let error = error.into();
        self.terminate(instance_id, ModelSessionState::Failed, &error, 1)
            .await
    }

    async fn fail_session_generation(
        &self,
        instance_id: ModelInstanceId,
        spawn_generation: u64,
        error: impl Into<String>,
    ) -> SwarmResult<()> {
        let error = error.into();
        self.terminate_generation(
            instance_id,
            spawn_generation,
            ModelSessionState::Failed,
            &error,
            1,
        )
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
        let is_live = self
            .inner
            .registry
            .lock()
            .expect("registry poisoned")
            .contains_key(&instance_id);
        if !is_live {
            if let Some(cancel) = self
                .inner
                .pending_spawns
                .lock()
                .expect("pending spawns poisoned")
                .get(&instance_id)
                .map(|pending| pending.cancel.clone())
            {
                cancel.cancel();
                return Ok(());
            }
        }
        self.terminate(instance_id, ModelSessionState::Cancelled, &reason, -1)
            .await
    }

    async fn cancel_session_generation(
        &self,
        instance_id: ModelInstanceId,
        spawn_generation: u64,
        reason: impl Into<String>,
    ) -> SwarmResult<()> {
        let reason = reason.into();
        self.terminate_generation(
            instance_id,
            spawn_generation,
            ModelSessionState::Cancelled,
            &reason,
            -1,
        )
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

    #[cfg(test)]
    pub(crate) fn session_spawn_generation_for_test(
        &self,
        instance_id: ModelInstanceId,
    ) -> Option<u64> {
        self.inner
            .registry
            .lock()
            .expect("registry poisoned")
            .get(&instance_id)
            .map(|handle| handle.spawn_generation)
    }

    #[cfg(test)]
    pub(crate) fn drop_cleanup_owner_guard_for_test(
        &self,
        instance_id: ModelInstanceId,
        stale_spawn_generation: u64,
        owner_generation: u64,
    ) {
        drop(CleanupOwnershipGuard {
            inner: Arc::clone(&self.inner),
            instance_id,
            spawn_generation: stale_spawn_generation,
            generation: owner_generation,
        });
    }

    #[cfg(test)]
    pub(crate) fn cleanup_owner_state_for_test(
        &self,
        instance_id: ModelInstanceId,
    ) -> Option<(bool, bool)> {
        self.inner
            .registry
            .lock()
            .expect("registry poisoned")
            .get(&instance_id)
            .and_then(|handle| handle.cleanup.as_ref())
            .map(|cleanup| {
                (
                    cleanup.in_progress,
                    *cleanup.owner_outcome_tx.borrow() == CleanupOwnerOutcome::InProgress,
                )
            })
    }

    #[cfg(test)]
    pub(crate) async fn cancel_session_generation_for_test(
        &self,
        instance_id: ModelInstanceId,
        spawn_generation: u64,
        reason: impl Into<String>,
    ) -> SwarmResult<()> {
        self.cancel_session_generation(instance_id, spawn_generation, reason)
            .await
    }

    #[cfg(any(test, feature = "test-utils"))]
    pub fn set_routing_after_launch_intent_pause_for_test(
        &self,
        arrived: Arc<tokio::sync::Notify>,
        release: Arc<tokio::sync::Notify>,
    ) {
        *self
            .inner
            .routing_after_launch_intent_pause
            .lock()
            .expect("routing launch-intent pause poisoned") = Some((arrived, release));
    }

    #[cfg(any(test, feature = "test-utils"))]
    pub fn set_routing_before_pending_registration_pause_for_test(
        &self,
        arrived: Arc<tokio::sync::Notify>,
        release: Arc<tokio::sync::Notify>,
    ) {
        *self
            .inner
            .routing_before_pending_registration_pause
            .lock()
            .expect("routing pre-registration pause poisoned") = Some((arrived, release));
    }

    #[cfg(any(test, feature = "test-utils"))]
    pub fn set_routing_before_dispatch_error_cleanup_pause_for_test(
        &self,
        arrived: Arc<tokio::sync::Notify>,
        release: Arc<tokio::sync::Notify>,
    ) {
        *self
            .inner
            .routing_before_dispatch_error_cleanup_pause
            .lock()
            .expect("routing dispatch-error pause poisoned") = Some((arrived, release));
    }

    #[cfg(any(test, feature = "test-utils"))]
    pub fn set_routing_before_authority_request_commit_pause_for_test(
        &self,
        arrived: Arc<tokio::sync::Notify>,
        release: Arc<tokio::sync::Notify>,
    ) {
        *self
            .inner
            .routing_before_authority_request_commit_pause
            .lock()
            .expect("routing authority-request pause poisoned") = Some((arrived, release));
    }

    #[cfg(any(test, feature = "test-utils"))]
    pub fn pending_spawn_count_for_test(&self) -> usize {
        self.inner
            .pending_spawns
            .lock()
            .expect("pending spawns poisoned")
            .len()
    }

    fn begin_routing_spawn_admission(
        &self,
        identity: RoutingAttemptIdentity,
        instance_id: ModelInstanceId,
    ) -> SwarmResult<RoutingSpawnAdmissionGuard> {
        let key = (identity, instance_id);
        let mut admissions = self
            .inner
            .routing_spawn_admissions
            .lock()
            .expect("routing spawn admissions poisoned");
        match admissions.entry(key.clone()) {
            Entry::Occupied(_) => Err(SwarmError::LedgerFailed(format!(
                "routing attempt already owns spawn admission for {instance_id}"
            ))),
            Entry::Vacant(entry) => {
                entry.insert(false);
                Ok(RoutingSpawnAdmissionGuard {
                    inner: Arc::clone(&self.inner),
                    key,
                })
            }
        }
    }

    #[cfg(any(test, feature = "test-utils"))]
    pub fn set_reaper_after_snapshot_pause_for_test(
        &self,
        arrived: Arc<tokio::sync::Notify>,
        release: Arc<tokio::sync::Notify>,
    ) {
        *self
            .inner
            .reaper_after_snapshot_pause
            .lock()
            .expect("reaper snapshot pause poisoned") = Some((arrived, release));
    }

    #[cfg(any(test, feature = "test-utils"))]
    pub fn set_cleanup_retry_after_snapshot_pause_for_test(
        &self,
        arrived: Arc<tokio::sync::Notify>,
        release: Arc<tokio::sync::Notify>,
    ) {
        *self
            .inner
            .cleanup_retry_after_snapshot_pause
            .lock()
            .expect("cleanup retry snapshot pause poisoned") = Some((arrived, release));
    }

    #[cfg(any(test, feature = "test-utils"))]
    pub fn fail_next_routing_output_persistence_for_test(&self) {
        self.inner
            .fail_next_routing_output_persistence
            .store(true, Ordering::SeqCst);
    }

    #[cfg(any(test, feature = "test-utils"))]
    pub fn fail_next_sibling_cancellation_persistence_for_test(&self) {
        self.inner
            .fail_next_sibling_cancellation_persistence
            .store(true, Ordering::SeqCst);
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
    #[cfg(any(test, feature = "test-utils"))]
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

    /// The central generation facade attached to a live coordinator session.
    /// Application callers that need repeated/batch generation use this
    /// accessor instead of reaching through to `ModelRuntime::generate`.
    #[cfg(any(test, feature = "test-utils"))]
    pub fn session_llm_client(
        &self,
        instance_id: ModelInstanceId,
    ) -> Option<Arc<dyn crate::llm::LlmClient>> {
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
            .map(|h| h.llm_client.clone())
    }

    /// Start one generation through the coordinator-owned session handle.
    ///
    /// The request's `id` and cancellation token are replaced with the live
    /// session values while the registry lock is held. Callers therefore cannot
    /// accidentally create an unrelated cancellation token that bypasses
    /// [`Self::cancel_session`]'s terminal ModelLane/EventLedger transition.
    /// A session may have only one active generation: a second start while it
    /// is already `Generating` fails closed.
    fn generate_session_raw(
        &self,
        instance_id: ModelInstanceId,
        request: GenerateRequest,
    ) -> SwarmResult<TokenStream> {
        self.generate_session_raw_with_context(instance_id, None, request, None)
    }

    fn generate_session_raw_with_context(
        &self,
        instance_id: ModelInstanceId,
        expected_spawn_generation: Option<u64>,
        mut request: GenerateRequest,
        invocation_context: Option<LlmInvocationContext>,
    ) -> SwarmResult<TokenStream> {
        let remaining = self.remaining();
        if remaining.tokens_remaining == Some(0) {
            return Err(SwarmError::LedgerFailed(
                "generation rejected: run token budget is exhausted".to_string(),
            ));
        }
        if remaining.cost_micros_remaining == Some(0) {
            return Err(SwarmError::LedgerFailed(
                "generation rejected: run cost budget is exhausted".to_string(),
            ));
        }
        if remaining
            .tokens_remaining
            .is_some_and(|available| u64::from(request.max_tokens) > available)
        {
            return Err(SwarmError::LedgerFailed(format!(
                "generation max_tokens {} exceeds remaining run token budget {}",
                request.max_tokens,
                remaining.tokens_remaining.unwrap_or_default()
            )));
        }
        let (llm_client, model_id, cancel) = {
            let mut registry = self.inner.registry.lock().expect("registry poisoned");
            let handle = registry
                .get_mut(&instance_id)
                .ok_or(SwarmError::UnknownInstance(instance_id))?;
            if expected_spawn_generation.is_some_and(|expected| handle.spawn_generation != expected)
            {
                return Err(SwarmError::LedgerFailed(format!(
                    "stale routing generation cannot start replacement session {instance_id}"
                )));
            }
            if handle.state != ModelSessionState::Ready {
                return Err(SwarmError::LedgerFailed(format!(
                    "session {instance_id} must be Ready before generation; got {:?}",
                    handle.state
                )));
            }
            let from = handle.state;
            if let Some(context) = invocation_context.as_ref() {
                let mut generations = self
                    .inner
                    .managed_generations
                    .lock()
                    .expect("managed generations poisoned");
                match generations.entry((instance_id, handle.spawn_generation)) {
                    Entry::Occupied(_) => {
                        return Err(SwarmError::InvalidStateTransition {
                            from: ModelSessionState::Generating,
                            to: ModelSessionState::Generating,
                        });
                    }
                    Entry::Vacant(entry) => {
                        entry.insert(ManagedGenerationInvocation {
                            trace_id: context.trace_id,
                            run_id: context.run_id.clone(),
                            session_id: context.session_id.clone(),
                            generated_tokens: 0,
                            disposition: None,
                            usage_committed: false,
                            terminal_emitted: false,
                        });
                    }
                }
            }
            handle.state = ModelSessionState::Generating;
            let event_result = self.emit_event(SwarmEvent::SessionStateChanged {
                instance_id,
                from,
                to: ModelSessionState::Generating,
            });
            let event_result = event_result.and_then(|()| {
                if let Some(context) = invocation_context.as_ref() {
                    self.emit_event(SwarmEvent::ModelInvocationStarted {
                        instance_id,
                        trace_id: context.trace_id,
                        run_id: context.run_id.clone(),
                        session_id: context.session_id.clone(),
                        max_tokens: request.max_tokens,
                    })
                } else {
                    Ok(())
                }
            });
            if let Err(error) = event_result {
                handle.state = from;
                if invocation_context.is_some() {
                    self.inner
                        .managed_generations
                        .lock()
                        .expect("managed generations poisoned")
                        .remove(&(instance_id, handle.spawn_generation));
                }
                return Err(error);
            }
            (
                handle.llm_client.clone(),
                handle.model_id,
                handle.cancel.clone(),
            )
        };
        request.id = model_id;
        request.cancel = cancel;
        if let Some(context) = invocation_context {
            Ok(llm_client.stream_completion_with_context(request, context))
        } else {
            Ok(llm_client.stream_completion(request))
        }
    }

    fn record_managed_generation_token(&self, instance_id: ModelInstanceId, spawn_generation: u64) {
        if let Some(invocation) = self
            .inner
            .managed_generations
            .lock()
            .expect("managed generations poisoned")
            .get_mut(&(instance_id, spawn_generation))
        {
            if invocation.disposition.is_none()
                && !invocation.usage_committed
                && !invocation.terminal_emitted
            {
                invocation.generated_tokens = invocation.generated_tokens.saturating_add(1);
            }
        }
    }

    /// Serialize terminal authority for one active invocation. Cleanup claims
    /// use the same registry -> invocation-map lock order, so cancellation
    /// cannot lose to a late EOF after it has fenced the exact generation.
    /// The record is retained on sink failure and is marked emitted only after
    /// the synchronous durable sink accepts the terminal event.
    fn emit_managed_generation_terminal(
        &self,
        instance_id: ModelInstanceId,
        spawn_generation: u64,
        fallback: ManagedGenerationDisposition,
    ) -> (
        Option<ModelSessionState>,
        Result<Option<ManagedGenerationDisposition>, String>,
    ) {
        let registry = self.inner.registry.lock().expect("registry poisoned");
        let handle = registry
            .get(&instance_id)
            .filter(|handle| handle.spawn_generation == spawn_generation);
        let state = handle.map(|handle| handle.state);
        let cleanup_disposition = handle.and_then(|handle| {
            handle.cleanup.as_ref().map(|cleanup| {
                ManagedGenerationDisposition::from_session_terminal(
                    cleanup.terminal,
                    cleanup.reason.clone(),
                )
            })
        });
        let mut generations = self
            .inner
            .managed_generations
            .lock()
            .expect("managed generations poisoned");
        let Some(invocation) = generations.get_mut(&(instance_id, spawn_generation)) else {
            // Only the caller-owned finalizer removes a record, and only after
            // successful emission. Missing authority therefore means another
            // path already emitted and released this invocation.
            return (state, Ok(None));
        };
        if invocation.disposition.is_none() {
            invocation.disposition = Some(cleanup_disposition.unwrap_or(fallback));
        }
        let disposition = invocation
            .disposition
            .clone()
            .expect("managed generation disposition just initialized");
        if !invocation.usage_committed {
            self.record_usage(invocation.generated_tokens, 0);
            invocation.usage_committed = true;
        }
        if invocation.terminal_emitted {
            return (state, Ok(Some(disposition)));
        }
        let event = SwarmEvent::ModelInvocationFinished {
            instance_id,
            trace_id: invocation.trace_id,
            run_id: invocation.run_id.clone(),
            session_id: invocation.session_id.clone(),
            outcome: disposition.outcome_str().to_string(),
            generated_tokens: invocation.generated_tokens,
            error: disposition.error.clone(),
        };
        match self.inner.sink.emit(event) {
            Ok(()) => {
                invocation.terminal_emitted = true;
                (state, Ok(Some(disposition)))
            }
            Err(error) => (state, Err(error)),
        }
    }

    fn release_managed_generation_authority(
        &self,
        instance_id: ModelInstanceId,
        spawn_generation: u64,
    ) {
        let mut generations = self
            .inner
            .managed_generations
            .lock()
            .expect("managed generations poisoned");
        let may_release = generations
            .get(&(instance_id, spawn_generation))
            .is_some_and(|invocation| invocation.terminal_emitted);
        if may_release {
            generations.remove(&(instance_id, spawn_generation));
        }
    }

    fn emit_active_managed_generation_terminals(&self, reason: &str) -> SwarmResult<()> {
        let active = self
            .inner
            .managed_generations
            .lock()
            .expect("managed generations poisoned")
            .iter()
            .filter_map(|(key, invocation)| (!invocation.terminal_emitted).then_some(*key))
            .collect::<Vec<_>>();
        let mut errors = Vec::new();
        for (instance_id, spawn_generation) in active {
            let (_, result) = self.emit_managed_generation_terminal(
                instance_id,
                spawn_generation,
                ManagedGenerationDisposition::cancelled(reason.to_string()),
            );
            if let Err(error) = result {
                errors.push(format!("{instance_id}: {error}"));
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(SwarmError::EventSinkFailed(format!(
                "managed-generation terminal emission failed: {}",
                errors.join(" | ")
            )))
        }
    }

    #[cfg(any(test, feature = "test-utils"))]
    pub fn generate_session(
        &self,
        instance_id: ModelInstanceId,
        request: GenerateRequest,
    ) -> SwarmResult<TokenStream> {
        self.generate_session_raw(instance_id, request)
    }

    /// Coordinator-owned generation stream. The session's traced/budgeted
    /// [`LlmClient`] remains the only generation authority, and Ready-state
    /// finalization runs on EOF, provider error, cancellation, or caller drop.
    pub fn generate_session_managed(
        &self,
        instance_id: ModelInstanceId,
        request: GenerateRequest,
    ) -> SwarmResult<TokenStream> {
        let (run_id, session_id, spawn_generation) = {
            let registry = self.inner.registry.lock().expect("registry poisoned");
            let handle = registry
                .get(&instance_id)
                .ok_or(SwarmError::UnknownInstance(instance_id))?;
            (
                handle.parent_session_id.clone(),
                instance_id.to_string(),
                handle.spawn_generation,
            )
        };
        self.generate_session_managed_generation(
            instance_id,
            spawn_generation,
            request,
            run_id,
            session_id,
        )
    }

    fn generate_session_managed_generation(
        &self,
        instance_id: ModelInstanceId,
        spawn_generation: u64,
        request: GenerateRequest,
        run_id: String,
        session_id: String,
    ) -> SwarmResult<TokenStream> {
        use futures::StreamExt as _;

        let trace_id = Uuid::now_v7();
        let stream = self.generate_session_raw_with_context(
            instance_id,
            Some(spawn_generation),
            request,
            Some(LlmInvocationContext {
                trace_id,
                run_id: run_id.clone(),
                session_id: session_id.clone(),
                evidence_owner: LlmInvocationEvidenceOwner::Coordinator,
            }),
        )?;
        let finalizer = ManagedGenerationFinalizer {
            coordinator: Arc::new(self.clone()),
            instance_id,
            spawn_generation,
            finalized: false,
        };
        let invocation_deadline =
            tokio::time::Instant::now() + self.inner.config.provider_invocation_timeout;
        let idle_timeout = self.inner.config.provider_idle_timeout;
        Ok(Box::pin(futures::stream::unfold(
            (stream, finalizer, invocation_deadline, false),
            move |(mut stream, mut finalizer, invocation_deadline, terminated)| async move {
                if terminated {
                    return None;
                }
                let now = tokio::time::Instant::now();
                let invocation_remaining = invocation_deadline.saturating_duration_since(now);
                let wait_budget = invocation_remaining.min(idle_timeout);
                let next = tokio::time::timeout(wait_budget, stream.next()).await;
                match next {
                    Err(_) => {
                        let reason = if tokio::time::Instant::now() >= invocation_deadline {
                            "provider generation invocation deadline elapsed"
                        } else {
                            "provider generation idle deadline elapsed"
                        };
                        // `finish_cancelled` enters Cancelling, installs durable
                        // cleanup intent, and invokes both cancellation seams
                        // synchronously inside `terminate` before its first
                        // persistence await.
                        let terminal_result = finalizer.finish_cancelled(reason.to_string()).await;
                        let stream_error = terminal_result
                            .err()
                            .map(crate::model_runtime::ModelRuntimeError::GenerateError)
                            .unwrap_or(crate::model_runtime::ModelRuntimeError::Cancelled);
                        Some((
                            Err(stream_error),
                            (stream, finalizer, invocation_deadline, true),
                        ))
                    }
                    Ok(Some(Ok(token))) => {
                        finalizer.record_token();
                        Some((Ok(token), (stream, finalizer, invocation_deadline, false)))
                    }
                    Ok(Some(Err(error))) => {
                        let reason = error.to_string();
                        let terminal_result =
                            if matches!(&error, crate::model_runtime::ModelRuntimeError::Cancelled)
                            {
                                finalizer.finish_cancelled(reason).await
                            } else {
                                finalizer.finish_failure(reason).await
                            };
                        let stream_error = terminal_result
                            .err()
                            .map(|terminal_error| {
                                crate::model_runtime::ModelRuntimeError::GenerateError(format!(
                                    "terminal event persistence rejected: {terminal_error}; provider result: {error}"
                                ))
                            })
                            .unwrap_or(error);
                        Some((
                            Err(stream_error),
                            (stream, finalizer, invocation_deadline, true),
                        ))
                    }
                    Ok(None) => match finalizer.finish_ready().await {
                        Ok(()) => None,
                        Err(error) => Some((
                            Err(crate::model_runtime::ModelRuntimeError::GenerateError(
                                error,
                            )),
                            (stream, finalizer, invocation_deadline, true),
                        )),
                    },
                }
            },
        )))
    }

    /// Score through the coordinator-owned LlmClient without exposing the
    /// runtime or client. Usage and lifecycle state are finalized exactly once.
    pub async fn score_session(
        &self,
        instance_id: ModelInstanceId,
        sequence: Vec<u32>,
    ) -> SwarmResult<crate::model_runtime::Score> {
        let token_cost = u64::try_from(sequence.len()).unwrap_or(u64::MAX);
        if self
            .remaining()
            .tokens_remaining
            .is_some_and(|remaining| token_cost > remaining)
        {
            return Err(SwarmError::LedgerFailed(
                "score rejected: sequence exceeds remaining run token budget".to_string(),
            ));
        }
        let (client, model_id) = {
            let mut registry = self.inner.registry.lock().expect("registry poisoned");
            let handle = registry
                .get_mut(&instance_id)
                .ok_or(SwarmError::UnknownInstance(instance_id))?;
            if handle.state != ModelSessionState::Ready {
                return Err(SwarmError::LedgerFailed(format!(
                    "session {instance_id} must be Ready before scoring; got {:?}",
                    handle.state
                )));
            }
            handle.state = ModelSessionState::Generating;
            (handle.llm_client.clone(), handle.model_id)
        };
        self.emit_event(SwarmEvent::SessionStateChanged {
            instance_id,
            from: ModelSessionState::Ready,
            to: ModelSessionState::Generating,
        })?;
        let result = client.score(model_id, sequence).await;
        self.record_usage(token_cost, 0);
        match result {
            Ok(score) => {
                if self.session_state(instance_id) == Some(ModelSessionState::Generating) {
                    self.transition(instance_id, ModelSessionState::Ready)?;
                }
                Ok(score)
            }
            Err(error) => {
                let detail = error.to_string();
                if self.session_state(instance_id) == Some(ModelSessionState::Generating) {
                    self.fail_session(instance_id, detail.clone()).await?;
                }
                Err(SwarmError::LedgerFailed(format!(
                    "session scoring failed: {detail}"
                )))
            }
        }
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

    /// The model-session concurrency cap currently in force.
    ///
    /// This is the live value, not `config.budget.max_concurrent`, which is only
    /// what the coordinator was constructed with.
    pub fn max_concurrent(&self) -> usize {
        // Reconciles first so a GET after sessions drained reports the cap that
        // is actually in force, not the stale value from request time.
        self.inner.reconcile_concurrency_cap()
    }

    /// The cap the operator last requested, whether or not it is fully applied.
    ///
    /// Differs from [`Self::max_concurrent`] only while a lowering is still
    /// draining; the pair is what lets the UI say "3 in force, 1 requested"
    /// instead of picking one number and being wrong half the time.
    pub fn requested_max_concurrent(&self) -> usize {
        self.inner.desired_max_concurrent.load(Ordering::SeqCst)
    }

    /// Change how many model sessions may run concurrently, WITHOUT rebuilding
    /// the coordinator or disturbing sessions that are already running.
    ///
    /// WP-1 MT-021 AC-3: the operator-facing swarm concurrency control must bind
    /// to real runtime behaviour. That means moving the actual admission
    /// semaphore, because a setting that only changes a displayed number is the
    /// misleading control the acceptance row exists to remove.
    ///
    /// Semantics, and why they are the safe ones:
    /// * RAISING adds permits immediately, so waiting spawns are admitted at
    ///   once.
    /// * LOWERING is COOPERATIVE, never preemptive: it removes permits that are
    ///   free right now and marks the remainder to be retired as running
    ///   sessions finish (`Semaphore::forget_permits` returns how many it could
    ///   actually take). Sessions already admitted keep running to completion.
    ///   Killing live model sessions to satisfy a settings change would destroy
    ///   operator work and orphan processes, which HBR-QUIET-003 forbids.
    /// * The returned value is the cap now IN FORCE. When lowering could not
    ///   fully take effect yet, the caller learns the real number instead of a
    ///   optimistic one, so the UI can report the truth rather than the request.
    ///
    /// A cap below 1 is clamped to 1: zero would wedge the coordinator with no
    /// way to admit the spawn that would release a permit.
    pub fn set_max_concurrent(&self, requested: usize) -> usize {
        let requested = requested.max(1);
        // Record the target FIRST. This is what makes a partially-applied
        // lowering converge instead of being abandoned at whatever value was
        // reachable at request time: every later admission and read drains the
        // remaining deficit through `reconcile_concurrency_cap`. Storing it also
        // cancels any earlier pending lowering when the operator raises again.
        self.inner
            .desired_max_concurrent
            .store(requested, Ordering::SeqCst);

        // Raising takes effect at once so waiting spawns are admitted now.
        loop {
            let effective = self.inner.effective_max_concurrent.load(Ordering::SeqCst);
            if requested <= effective {
                break;
            }
            if self
                .inner
                .effective_max_concurrent
                .compare_exchange(effective, requested, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                self.inner.semaphore.add_permits(requested - effective);
                break;
            }
        }

        self.inner.reconcile_concurrency_cap()
    }

    /// The coordinator-owned cancellation token for a spawned instance, for
    /// OBSERVATION only.
    ///
    /// `generate_session_managed` deliberately replaces a caller-supplied token
    /// with this one so no caller can bypass the coordinator's terminal ledger
    /// transition. Consumers of the generated stream therefore have no way to
    /// ask "has this session been cancelled?" except through the in-band
    /// `FinishReason::Cancelled` token — and that marker races with any output
    /// already buffered ahead of it in the chunk channel. A capture path that
    /// only watches the in-band marker will durably persist an activity block
    /// that arrived AFTER cancellation, purely depending on which the executor
    /// polls first.
    ///
    /// Exposing the token lets a consumer fence its own durable writes on the
    /// same signal the runtime observes, making "only pre-cancel output is
    /// durable" deterministic instead of timing-dependent.
    ///
    /// Unlike [`Self::session_model_id`] this deliberately does NOT filter on
    /// `ModelSessionState`: the whole point is to keep observing the token while
    /// the session is being torn down, which is exactly when the state has
    /// already moved past `Ready`/`Generating`.
    pub fn session_cancel_token(&self, instance_id: ModelInstanceId) -> Option<CancellationToken> {
        self.inner
            .registry
            .lock()
            .expect("registry poisoned")
            .get(&instance_id)
            .map(|h| h.cancel.clone())
    }

    /// The coordinator's wired [`ModelLaneStore`] (a cheap `SurrealStorage`-backed clone),
    /// or `None` when the coordinator was built without one. WP-1 MT-012: the
    /// operator-chat launch service uses this to replay the just-spawned run/lane
    /// so it can persist the launched CLI runtime's real stdout as
    /// ModelLaneMessage rows, and the transcript route uses it to read them back.
    pub fn model_lane_store(&self) -> Option<ModelLaneStore> {
        self.inner.model_lane_store.clone()
    }

    pub(crate) fn routing_execution_store(
        &self,
    ) -> Option<super::routing_execution::ModelLaneRoutingExecutionStore> {
        self.inner.routing_execution_store.clone()
    }

    /// Materialize a Dexterity ContextBundle for the named downstream lane from
    /// embedded-Surreal/EventLedger authority. Callers pass the returned
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
        // Close admission under the same synchronous fence every spawn holds
        // until its PendingSpawn is visible. Once this returns, no invisible
        // pre-registration spawn can materialize behind the drain.
        self.inner
            .spawn_admission_closed
            .store(true, Ordering::SeqCst);
        while self.inner.spawn_pre_registration.load(Ordering::SeqCst) != 0 {
            tokio::task::yield_now().await;
        }

        let pending_tokens: Vec<CancellationToken> = self
            .inner
            .pending_spawns
            .lock()
            .expect("pending spawns poisoned")
            .values()
            .map(|pending| pending.cancel.clone())
            .collect();
        for cancel in pending_tokens {
            cancel.cancel();
        }
        let mut errors = Vec::new();
        if let Err(error) = self.cancel_live_snapshot("drain_all").await {
            // The late-materialization pass below is also the bounded retry for
            // a transient first-pass cleanup failure. Do not report a recovered
            // failure as the final shutdown outcome.
            tracing::warn!(
                target: "handshake_core::swarm_orchestration",
                %error,
                "initial orderly-drain cleanup pass left retryable sessions"
            );
        }

        // A pending factory may finish its cancellation compensation and move
        // through the live registry after the first snapshot. Wait for every
        // published pending owner to retire, then drain the now-closed live set
        // one final time. The existing teardown budget bounds this wait.
        if tokio::time::timeout(self.inner.config.teardown_timeout, async {
            while !self
                .inner
                .pending_spawns
                .lock()
                .expect("pending spawns poisoned")
                .is_empty()
            {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .is_err()
        {
            errors.push(format!(
                "drain_all exceeded {:?} waiting for pending spawn compensation",
                self.inner.config.teardown_timeout
            ));
        }
        if let Err(error) = self
            .cancel_live_snapshot("drain_all late materialization")
            .await
        {
            errors.push(error.to_string());
        }
        if let Err(error) = self.retry_pending_orphan_cleanups().await {
            errors.push(error.to_string());
        }
        if let Err(error) = self.emit_active_managed_generation_terminals("drain_all") {
            errors.push(error.to_string());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(SwarmError::LedgerFailed(format!(
                "orderly coordinator drain left failures after attempting every cleanup class: {}",
                errors.join(" | ")
            )))
        }
    }

    async fn cancel_live_snapshot(&self, reason: &str) -> SwarmResult<()> {
        let live: Vec<(
            ModelInstanceId,
            u64,
            CancellationToken,
            Arc<dyn crate::model_runtime::ModelRuntime>,
        )> = self
            .inner
            .registry
            .lock()
            .expect("registry poisoned")
            .values()
            .filter(|handle| !handle.state.is_terminal())
            .map(|handle| {
                (
                    handle.instance_id,
                    handle.spawn_generation,
                    handle.cancel.clone(),
                    Arc::clone(&handle.runtime),
                )
            })
            .collect();
        for (_, _, cancel, runtime) in &live {
            cancel.cancel();
            runtime.cancel(cancel.clone());
        }
        let mut errors = Vec::new();
        for (instance_id, generation, _, _) in live {
            match self
                .cancel_session_generation(instance_id, generation, reason)
                .await
            {
                Ok(()) | Err(SwarmError::UnknownInstance(_)) => {}
                Err(error) => errors.push(format!("{instance_id}: {error}")),
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(SwarmError::LedgerFailed(format!(
                "live-session drain left cleanup failures: {}",
                errors.join(" | ")
            )))
        }
    }

    /// Execute every currently-ready stage through coordinator-owned production seams.
    async fn dispatch_ready_routing_stages(
        &self,
        execution_id: &str,
        launches: Vec<super::routing_execution::ModelLaneRoutingStageLaunch>,
    ) -> SwarmResult<super::routing_execution::ModelLaneRoutingDispatchBatch> {
        use super::routing::ModelLaneRoutingDispatchTarget;
        use super::routing_execution::{
            ModelLaneRoutingStageDispatch, ModelLaneRoutingStageStateKind,
        };
        use futures::StreamExt as _;
        let execution_store = self.inner.routing_execution_store.as_ref().ok_or_else(|| {
            SwarmError::LedgerFailed(
                "production routing requires a scoped embedded-Surreal ModelLaneStore".into(),
            )
        })?;
        let execution = execution_store
            .snapshot(execution_id)
            .await
            .map_err(|err| SwarmError::LedgerFailed(format!("routing snapshot failed: {err}")))?
            .ok_or_else(|| SwarmError::LedgerFailed(format!(
                "routing execution {execution_id} must be initialized from a persisted promotion decision"
            )))?;
        let authority = &execution.authority;

        for launch in &launches {
            let Some(request) = launch.request.as_ref() else {
                continue;
            };
            if !matches!(
                request.provider,
                Some(ProviderKind::ByokCloud | ProviderKind::OfficialCli)
            ) {
                continue;
            }
            let contract = request.dexterity_launch.as_ref().ok_or_else(|| {
                SwarmError::LedgerFailed(format!(
                    "cloud routing stage {} requires Dexterity consent authority",
                    launch.stage_id
                ))
            })?;
            if contract.consent_receipt_ref.as_deref()
                != authority.cloud_consent_receipt_ref.as_deref()
            {
                return Err(SwarmError::LedgerFailed(format!(
                    "cloud routing stage {} consent receipt does not match graph authority",
                    launch.stage_id
                )));
            }
        }
        let claims = execution_store
            .claim_ready(execution_id, &launches)
            .await
            .map_err(|err| SwarmError::LedgerFailed(format!("routing claim failed: {err}")))?;
        let mut launches_by_stage: std::collections::HashMap<_, _> = launches
            .into_iter()
            .map(|launch| (launch.stage_id.clone(), launch))
            .collect();
        let jobs: Vec<_> = claims
            .into_iter()
            .map(|claim| {
                let launch = launches_by_stage.remove(&claim.stage_id);
                (claim, launch)
            })
            .collect();
        let mut pending: futures::stream::FuturesUnordered<_> = jobs
            .into_iter()
            .map(|(claim, launch)| {
                let origin_claim = claim.clone();
                async move {
                    let result = async {
            if claim.dispatch_target == ModelLaneRoutingDispatchTarget::CoordinatorJoin {
                execution_store
                    .record_transition(
                        &claim,
                        ModelLaneRoutingStageStateKind::InFlight,
                        None,
                        Some("coordinator join materializing canonical predecessor outputs".into()),
                    )
                    .await
                    .map_err(|err| {
                        SwarmError::LedgerFailed(format!(
                            "routing join launch intent failed: {err}"
                        ))
                    })?;
                let input = execution_store
                    .stage_input_envelope(execution_id, &claim.stage_id)
                    .await
                    .map_err(|err| SwarmError::LedgerFailed(format!("routing join input failed: {err}")))?;
                execution_store
                    .record_generated_output(
                        &claim,
                        ModelLaneRoutingStageStateKind::Joined,
                        None,
                        None,
                        input,
                        None,
                    )
                    .await
                    .map_err(|err| SwarmError::LedgerFailed(format!("routing join failed: {err}")))?;
                return Ok(ModelLaneRoutingStageDispatch {
                    stage_id: claim.stage_id,
                    dispatch_target: claim.dispatch_target,
                    state: ModelLaneRoutingStageStateKind::Joined,
                    instance_id: None,
                    detail: None,
                });
            }

            if matches!(claim.dispatch_target, ModelLaneRoutingDispatchTarget::Validator | ModelLaneRoutingDispatchTarget::Operator) {
                let authority_lane_id = match launch
                    .as_ref()
                    .and_then(|launch| launch.authority_lane_id.clone())
                {
                    Some(lane_id) => lane_id,
                    None => {
                        let detail = format!("authority stage {} requires a durable ModelLane id", claim.stage_id);
                        execution_store
                            .record_transition(&claim, ModelLaneRoutingStageStateKind::Failed, None, Some(detail.clone()))
                            .await
                            .map_err(|err| SwarmError::LedgerFailed(format!("routing authority failure persistence failed: {err}")))?;
                        return Ok(ModelLaneRoutingStageDispatch {
                            stage_id: claim.stage_id,
                            dispatch_target: claim.dispatch_target,
                            state: ModelLaneRoutingStageStateKind::Failed,
                            instance_id: None,
                            detail: Some(detail),
                        });
                    }
                };
                let model_lane_store = self.inner.model_lane_store.as_ref().ok_or_else(|| {
                    SwarmError::LedgerFailed("routing authority dispatch requires ModelLaneStore".into())
                })?;
                let projection = match model_lane_store.navigation_by_lane(&authority_lane_id).await {
                    Ok(projection) => projection,
                    Err(err) => {
                        let detail = format!("routing authority lane lookup failed: {err}");
                        execution_store
                            .record_transition(&claim, ModelLaneRoutingStageStateKind::Failed, None, Some(detail.clone()))
                            .await
                            .map_err(|persist_err| SwarmError::LedgerFailed(format!("{detail}; persistence failed: {persist_err}")))?;
                        return Ok(ModelLaneRoutingStageDispatch {
                            stage_id: claim.stage_id,
                            dispatch_target: claim.dispatch_target,
                            state: ModelLaneRoutingStageStateKind::Failed,
                            instance_id: None,
                            detail: Some(detail),
                        });
                    }
                };
                let expected_kind = match claim.dispatch_target {
                    ModelLaneRoutingDispatchTarget::Validator => super::model_lane::ModelLaneKind::Validator,
                    ModelLaneRoutingDispatchTarget::Operator => super::model_lane::ModelLaneKind::HumanOperator,
                    _ => unreachable!(),
                };
                if projection.lanes.first().map(|lane| &lane.kind) != Some(&expected_kind) {
                    let detail = format!("authority lane {authority_lane_id} does not match {:?}", claim.dispatch_target);
                    execution_store
                        .record_transition(&claim, ModelLaneRoutingStageStateKind::Failed, None, Some(detail.clone()))
                        .await
                        .map_err(|err| SwarmError::LedgerFailed(format!("routing authority-kind persistence failed: {err}")))?;
                    return Ok(ModelLaneRoutingStageDispatch {
                        stage_id: claim.stage_id,
                        dispatch_target: claim.dispatch_target,
                        state: ModelLaneRoutingStageStateKind::Failed,
                        instance_id: None,
                        detail: Some(detail),
                    });
                }
                let current_execution = execution_store
                    .snapshot(execution_id)
                    .await
                    .map_err(|err| SwarmError::LedgerFailed(format!("authority request snapshot failed: {err}")))?
                    .ok_or_else(|| SwarmError::LedgerFailed(format!("routing execution {execution_id} disappeared")))?;
                let authority_stage = current_execution.stages.get(&claim.stage_id).ok_or_else(|| {
                    SwarmError::LedgerFailed(format!("authority stage {} disappeared", claim.stage_id))
                })?;
                let source_stage = current_execution
                    .stages
                    .values()
                    .find(|candidate| {
                        candidate.output_ref.as_ref().is_some_and(|output_ref| {
                            authority_stage.input_refs.iter().any(|input_ref| input_ref == output_ref)
                        }) && candidate.lane_id.is_some()
                    })
                    .ok_or_else(|| SwarmError::LedgerFailed(format!(
                        "authority stage {} has no causal predecessor ModelLane output",
                        claim.stage_id
                    )))?;
                let source_lane_id = source_stage.lane_id.clone().expect("checked source lane");
                let source_projection = model_lane_store
                    .navigation_by_lane(&source_lane_id)
                    .await
                    .map_err(|err| SwarmError::LedgerFailed(format!("authority source lane lookup failed: {err}")))?;
                let source_lane = source_projection.lanes.first().ok_or_else(|| {
                    SwarmError::LedgerFailed(format!("authority source lane {source_lane_id} missing"))
                })?;
                let source_message_ref = source_stage.output_message_ref.as_deref().ok_or_else(|| {
                    SwarmError::LedgerFailed(format!(
                        "authority source stage {} has no causal ModelLaneMessage ref",
                        source_stage.stage_id
                    ))
                })?;
                let source_message_projection = model_lane_store
                    .navigation_by_message(source_message_ref)
                    .await
                    .map_err(|err| SwarmError::LedgerFailed(format!(
                        "authority source message {source_message_ref} lookup failed: {err}"
                    )))?;
                let source_message = source_message_projection.messages.first().ok_or_else(|| {
                    SwarmError::LedgerFailed(format!(
                        "authority source message {source_message_ref} missing"
                    ))
                })?;
                let source_instance_id = parse_routing_model_instance_id(
                    source_stage.instance_id.as_deref().ok_or_else(|| {
                        SwarmError::LedgerFailed(format!(
                            "authority source stage {} has no runtime instance",
                            source_stage.stage_id
                        ))
                    })?,
                )?;
                let source_spawn_generation = self
                    .inner
                    .registry
                    .lock()
                    .expect("registry poisoned")
                    .get(&source_instance_id)
                    .filter(|handle| {
                        handle.routing_attempt.as_ref().is_some_and(|identity| {
                            identity.execution_id == execution_id
                                && identity.stage_id == source_stage.stage_id
                                && identity.attempt == source_stage.attempt
                        })
                    })
                    .map(|handle| handle.spawn_generation);
                let request_message_id = format!(
                    "routing-authority-request:{execution_id}:{}:{}",
                    claim.stage_id, claim.attempt
                );
                let request_message = super::model_lane::NewModelLaneMessage {
                        message_id: request_message_id.clone(),
                        run_id: current_execution.run_id.clone(),
                        trace_id: current_execution.trace_id.clone(),
                        message_span_id: uuid::Uuid::now_v7().to_string(),
                        parent_span_id: Some(current_execution.run_span_id.clone()),
                        linked_span_contexts: vec![source_message.message_span_id.clone()],
                        from_lane_id: source_lane_id,
                        to_lane: super::model_lane::ModelLaneTarget::Lane(authority_lane_id.clone()),
                        routing: Some(super::model_lane::ModelLaneRoutingMetadata {
                            target_role: format!("{:?}", claim.dispatch_target).to_ascii_lowercase(),
                            target_session: authority_lane_id.clone(),
                            correlation_id: format!("routing:{execution_id}:{}", claim.stage_id),
                            requires_ack: true,
                            ack_for: None,
                        }),
                        kind: super::model_lane::ModelLaneMessageKind::Status,
                        payload_ref: source_stage.output_ref.clone().expect("checked predecessor output"),
                        payload_sha256: source_stage.output_sha256.clone().ok_or_else(|| {
                            SwarmError::LedgerFailed("authority predecessor output has no hash".into())
                        })?,
                        event_ledger_stream_id: source_lane.event_ledger_stream_id.clone(),
                        summary: format!("authority request for routing stage {}", claim.stage_id),
                        authority: super::model_lane::ModelLaneAuthority::Advisory,
                        promotion_decision_id: Some(current_execution.selecting_decision_id.clone()),
                        promotion_gate_ref: None,
                        promotion_receipt_ref: None,
                        validator_verdict_ref: None,
                        operator_decision_ref: None,
                        promoted_artifact_ref: None,
                        promoted_artifact_sha256: None,
                        promoted_artifact_version: None,
                        tool_gate_decision_refs: Vec::new(),
                        coordinator_session_id: current_execution.coordinator_session_id.clone(),
                        work_packet_id: Some(current_execution.work_packet_id.clone()),
                        micro_task_id: current_execution.micro_task_id.clone(),
                        task_board_id: Some(current_execution.task_board_id.clone()),
                        owner_session: current_execution.owner_session.clone(),
                        locus_binding: Some(super::model_lane::ModelLaneLocusBinding {
                            work_packet_id: current_execution.work_packet_id.clone(),
                            micro_task_id: current_execution.micro_task_id.clone().ok_or_else(|| {
                                SwarmError::LedgerFailed("authority routing requires micro_task_id".into())
                            })?,
                            task_board_id: Some(current_execution.task_board_id.clone()),
                            coordinator_session_id: current_execution.coordinator_session_id.clone(),
                            session_id: source_lane.session_id.clone(),
                            model_session_id: source_lane.model_session_id.clone(),
                            owner_session: current_execution.owner_session.clone(),
                            locus_binding_ref: current_execution.locus_ref.clone(),
                        }),
                        idempotency_key: format!("routing-authority-request:{execution_id}:{}:{}", claim.stage_id, claim.attempt),
                        replay_order_key: format!("routing/{execution_id}/{}/authority-request/{:04}", claim.stage_id, claim.attempt),
                        replay_after_event_ledger_seq: Some(source_stage.event_ledger_seq),
                        proposal_ref: None,
                        crdt_update_ref: None,
                        crdt_base_snapshot_ref: None,
                        crdt_state_vector: None,
                        crdt_proposal_ref: None,
                        crdt_stale_base_ref: None,
                        failstate_code: None,
                        reason_ref: None,
                        recovery_hint_ref: None,
                        created_at_utc: chrono::Utc::now().to_rfc3339(),
                        diagnostic_payload: serde_json::json!({
                            "schema_id": "hsk.model_lane_routing_authority_request@1",
                            "execution_id": execution_id,
                            "stage_id": claim.stage_id,
                            "attempt": claim.attempt,
                            "fencing_token": claim.fencing_token,
                            "predecessor_message_ref": source_stage.output_message_ref,
                        }),
                    };
                #[cfg(any(test, feature = "test-utils"))]
                let authority_pause = {
                    self.inner
                        .routing_before_authority_request_commit_pause
                        .lock()
                        .expect("routing authority-request pause poisoned")
                        .take()
                };
                #[cfg(any(test, feature = "test-utils"))]
                if let Some((arrived, release)) = authority_pause {
                    arrived.notify_one();
                    release.notified().await;
                }
                let request_message = match execution_store
                    .record_authority_request(&claim, authority_lane_id.clone(), request_message)
                    .await
                {
                    Ok((_execution, message)) => message,
                    Err(err) => {
                        if let Some(cancelled) = canonical_cancelled_routing_dispatch(
                            execution_store,
                            &claim,
                            None,
                        )
                        .await
                        .map_err(|snapshot_err| {
                            SwarmError::LedgerFailed(format!(
                                "authority request commit failed ({err}); canonical cancellation reconciliation also failed: {snapshot_err}"
                            ))
                        })? {
                            return Ok(cancelled);
                        }
                        let cleanup = match source_spawn_generation {
                            Some(generation) => {
                                self.cancel_session_generation(
                                    source_instance_id,
                                    generation,
                                    "authority request persistence failed",
                                )
                                .await
                            }
                            None => Ok(()),
                        };
                        return Err(SwarmError::LedgerFailed(match cleanup {
                            Ok(()) | Err(SwarmError::UnknownInstance(_)) => {
                                format!("authority request persistence failed: {err}")
                            }
                            Err(cleanup_err) => format!(
                                "authority request persistence failed: {err}; source cleanup failed: {cleanup_err}"
                            ),
                        }));
                    }
                };
                let completion = match source_spawn_generation {
                    Some(generation) => {
                        self.complete_session_generation(source_instance_id, generation)
                            .await
                    }
                    None => Ok(()),
                };
                match completion {
                    Ok(()) | Err(SwarmError::UnknownInstance(_)) => {}
                    Err(err) => return Err(err),
                }
                let heartbeat_store = (*execution_store).clone();
                let heartbeat_claim = claim.clone();
                let heartbeat_lane_id = authority_lane_id.clone();
                let heartbeat_request_message_ref = request_message.message_id.clone();
                tokio::spawn(async move {
                    let mut heartbeat =
                        tokio::time::interval(std::time::Duration::from_secs(10));
                    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
                    loop {
                        heartbeat.tick().await;
                        if heartbeat_store
                            .heartbeat_claim(
                                &heartbeat_claim,
                                ModelLaneRoutingStageStateKind::AwaitingAuthority,
                                None,
                                Some(heartbeat_lane_id.clone()),
                                Some(heartbeat_request_message_ref.clone()),
                            )
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                });
                return Ok(ModelLaneRoutingStageDispatch {
                    stage_id: claim.stage_id,
                    dispatch_target: claim.dispatch_target,
                    state: ModelLaneRoutingStageStateKind::AwaitingAuthority,
                    instance_id: None,
                    detail: Some("awaiting typed authority ModelLaneMessage".into()),
                });
            }

            let launch = match launch {
                Some(launch) => launch,
                None => {
                    let detail = format!("ready model stage {} has no launch contract", claim.stage_id);
                    execution_store
                        .record_transition(&claim, ModelLaneRoutingStageStateKind::Failed, None, Some(detail.clone()))
                        .await
                        .map_err(|err| SwarmError::LedgerFailed(format!("routing missing-launch persistence failed: {err}")))?;
                    return Ok(ModelLaneRoutingStageDispatch {
                        stage_id: claim.stage_id,
                        dispatch_target: claim.dispatch_target,
                        state: ModelLaneRoutingStageStateKind::Failed,
                        instance_id: None,
                        detail: Some(detail),
                    });
                }
            };
            let mut request = match launch.request {
                Some(request) => request,
                None => {
                    let detail = format!("ready model stage {} has no SpawnRequest", claim.stage_id);
                    execution_store
                        .record_transition(&claim, ModelLaneRoutingStageStateKind::Failed, None, Some(detail.clone()))
                        .await
                        .map_err(|err| SwarmError::LedgerFailed(format!("routing missing-spawn persistence failed: {err}")))?;
                    return Ok(ModelLaneRoutingStageDispatch {
                        stage_id: claim.stage_id,
                        dispatch_target: claim.dispatch_target,
                        state: ModelLaneRoutingStageStateKind::Failed,
                        instance_id: None,
                        detail: Some(detail),
                    });
                }
            };
            let mut generate_request = match launch.generate_request {
                Some(request) => request,
                None => {
                    let detail = format!("ready model stage {} has no GenerateRequest", claim.stage_id);
                    execution_store
                        .record_transition(&claim, ModelLaneRoutingStageStateKind::Failed, None, Some(detail.clone()))
                        .await
                        .map_err(|err| SwarmError::LedgerFailed(format!("routing missing-generation persistence failed: {err}")))?;
                    return Ok(ModelLaneRoutingStageDispatch {
                        stage_id: claim.stage_id,
                        dispatch_target: claim.dispatch_target,
                        state: ModelLaneRoutingStageStateKind::Failed,
                        instance_id: None,
                        detail: Some(detail),
                    });
                }
            };
            let provider_matches = match claim.dispatch_target {
                ModelLaneRoutingDispatchTarget::LocalModel => {
                    request.provider.is_none() || request.provider == Some(ProviderKind::Local)
                }
                ModelLaneRoutingDispatchTarget::CloudModel => matches!(
                    request.provider,
                    Some(ProviderKind::ByokCloud | ProviderKind::OfficialCli)
                ),
                _ => false,
            };
            if !provider_matches {
                let detail = format!(
                    "routing stage {} target {:?} does not match provider {:?}",
                    claim.stage_id, claim.dispatch_target, request.provider
                );
                execution_store
                    .record_transition(&claim, ModelLaneRoutingStageStateKind::Failed, None, Some(detail.clone()))
                    .await
                    .map_err(|err| SwarmError::LedgerFailed(format!("routing provider-failure persistence failed: {err}")))?;
                return Ok(ModelLaneRoutingStageDispatch {
                    stage_id: claim.stage_id,
                    dispatch_target: claim.dispatch_target,
                    state: ModelLaneRoutingStageStateKind::Failed,
                    instance_id: None,
                    detail: Some(detail),
                });
            }
            let contract = request.dexterity_launch.as_ref().ok_or_else(|| {
                SwarmError::LedgerFailed(format!(
                    "routing stage {} requires a canonical Dexterity launch contract",
                    claim.stage_id
                ))
            })?;
            if contract.run_id != claim.expected_run_id
                || launch.expected_run_id != claim.expected_run_id
                || contract.lane_id != claim.expected_lane_id
                || launch.expected_lane_id != claim.expected_lane_id
                || launch.expected_provider != claim.expected_provider
                || request.provider != claim.expected_provider
                || launch.expected_model_id != claim.expected_model_id
                || request.instance_id.model_id.to_string() != claim.expected_model_id
                || generate_request.id != request.instance_id.model_id
            {
                let detail = format!(
                    "routing stage {} provider/model/run binding differs from its canonical launch contract",
                    claim.stage_id
                );
                execution_store
                    .record_transition(&claim, ModelLaneRoutingStageStateKind::Failed, None, Some(detail.clone()))
                    .await
                    .map_err(|err| SwarmError::LedgerFailed(format!("routing launch-binding persistence failed: {err}")))?;
                return Ok(ModelLaneRoutingStageDispatch {
                    stage_id: claim.stage_id,
                    dispatch_target: claim.dispatch_target,
                    state: ModelLaneRoutingStageStateKind::Failed,
                    instance_id: None,
                    detail: Some(detail),
                });
            }
            let lane_id = Some(contract.lane_id.clone());
            let contract_run_id = contract.run_id.clone();
            request
                .dexterity_launch
                .as_mut()
                .expect("routing launch contract validated above")
                .restart_generation = i64::from(claim.attempt.saturating_sub(1));
            request.routing_attempt = Some(RoutingAttemptIdentity {
                execution_id: claim.execution_id.clone(),
                stage_id: claim.stage_id.clone(),
                attempt: claim.attempt,
            });
            let input_envelope = execution_store
                .stage_input_envelope(execution_id, &claim.stage_id)
                .await
                .map_err(|err| SwarmError::LedgerFailed(format!("routing input materialization failed: {err}")))?;
            let output_contract = if claim.stage_id == "cloud-review" {
                "\n\noutput_contract:\nReturn JSON only with verdict set to accept, reject, or promotion_recommended and a review string."
            } else {
                ""
            };
            generate_request.prompt.text = format!(
                "{input_envelope}\n\ncanonical_stage_instruction:\nExecute routing stage {} using only the authoritative payloads above.{output_contract}",
                claim.stage_id
            );
            if let Err(error) = execution_store
                .record_transition(
                    &claim,
                    ModelLaneRoutingStageStateKind::InFlight,
                    Some(request.instance_id.to_string()),
                    Some(format!(
                        "launch_intent provider={:?} model={} run={}",
                        request.provider, request.instance_id.model_id, contract_run_id
                    )),
                )
                .await
            {
                if let Some(cancelled) = canonical_cancelled_routing_dispatch(
                    execution_store,
                    &claim,
                    None,
                )
                .await
                .map_err(|snapshot_error| {
                    SwarmError::LedgerFailed(format!(
                        "routing launch intent failed ({error}); cancellation reconciliation also failed: {snapshot_error}"
                    ))
                })? {
                    return Ok(cancelled);
                }
                return Err(SwarmError::LedgerFailed(format!(
                    "routing launch intent failed: {error}"
                )));
            }
            #[cfg(any(test, feature = "test-utils"))]
            let routing_pause = {
                self.inner
                    .routing_after_launch_intent_pause
                    .lock()
                    .expect("routing launch-intent pause poisoned")
                    .take()
            };
            #[cfg(any(test, feature = "test-utils"))]
            if let Some((arrived, release)) = routing_pause {
                arrived.notify_one();
                release.notified().await;
            }
            let expected_instance_id = request.instance_id.to_string();
            let routing_parent_session_id = request.parent_session_id.clone();
            // Register the in-memory half of the cancellation fence BEFORE the
            // final canonical read. Cancellation marks this exact
            // execution/stage/attempt/instance owner before scanning
            // pending/live maps. The spawn publication below checks that mark
            // under the same mutex, closing the post-read/pre-publication gap.
            let _routing_spawn_admission = self.begin_routing_spawn_admission(
                RoutingAttemptIdentity {
                    execution_id: claim.execution_id.clone(),
                    stage_id: claim.stage_id.clone(),
                    attempt: claim.attempt,
                },
                request.instance_id,
            )?;
            if let Some(cancelled) = canonical_cancelled_routing_dispatch(
                execution_store,
                &claim,
                Some(&expected_instance_id),
            )
            .await
            .map_err(|err| {
                SwarmError::LedgerFailed(format!(
                    "routing pre-spawn cancellation reconciliation failed: {err}"
                ))
            })? {
                return Ok(cancelled);
            }
            let (instance_id, spawn_generation) = match self
                .spawn_session_with_generation(request)
                .await
            {
                Ok(spawned) => spawned,
                Err(err) => {
                    let detail = err.to_string();
                    let failed_transition = execution_store
                        .record_transition(&claim, ModelLaneRoutingStageStateKind::Failed, None, Some(detail.clone()))
                        .await;
                    if let Err(persist_err) = failed_transition {
                        let cancelled = canonical_cancelled_routing_dispatch(
                            execution_store,
                            &claim,
                            Some(&expected_instance_id),
                        )
                        .await
                        .map_err(|snapshot_err| {
                            SwarmError::LedgerFailed(format!(
                                "routing spawn failed ({detail}) and persistence failed: {persist_err}; canonical cancellation reconciliation also failed: {snapshot_err}"
                            ))
                        })?;
                        if let Some(cancelled) = cancelled {
                            return Ok(cancelled);
                        }
                        return Err(SwarmError::LedgerFailed(format!(
                            "routing spawn failed ({detail}) and persistence failed: {persist_err}"
                        )));
                    }
                    return Ok(ModelLaneRoutingStageDispatch {
                        stage_id: claim.stage_id,
                        dispatch_target: claim.dispatch_target,
                        state: ModelLaneRoutingStageStateKind::Failed,
                        instance_id: None,
                        detail: Some(detail),
                    });
                }
            };
            let persisted_instance_id = instance_id.to_string();
            if let Err(err) = execution_store
                .record_transition(
                    &claim,
                    ModelLaneRoutingStageStateKind::InFlight,
                    Some(persisted_instance_id.clone()),
                    None,
                )
                .await
            {
                let cleanup = self
                    .cancel_session_generation(
                        instance_id,
                        spawn_generation,
                        "routing in-flight persistence rejected after spawn",
                    )
                    .await;
                match cleanup {
                    Ok(()) => {
                        let cancelled = canonical_cancelled_routing_dispatch(
                            execution_store,
                            &claim,
                            Some(&persisted_instance_id),
                        )
                        .await
                        .map_err(|snapshot_err| {
                            SwarmError::LedgerFailed(format!(
                                "routing in-flight transition failed after spawned session cleanup ({err}); canonical cancellation reconciliation also failed: {snapshot_err}"
                            ))
                        })?;
                        if let Some(cancelled) = cancelled {
                            return Ok(cancelled);
                        }
                        return Err(SwarmError::LedgerFailed(format!(
                            "routing in-flight transition failed after spawned session cleanup: {err}"
                        )));
                    }
                    Err(cleanup_err) => {
                        return Err(SwarmError::LedgerFailed(format!(
                            "routing in-flight transition failed ({err}); spawned session cleanup also failed: {cleanup_err}"
                        )));
                    }
                }
            }
            let mut stream = self.generate_session_managed_generation(
                instance_id,
                spawn_generation,
                generate_request,
                routing_parent_session_id,
                persisted_instance_id.clone(),
            )?;
            let mut output = String::new();
            let mut heartbeat = tokio::time::interval(std::time::Duration::from_secs(10));
            heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                let token = tokio::select! {
                    _ = heartbeat.tick() => {
                        if let Err(err) = execution_store
                            .heartbeat_claim(
                                &claim,
                                ModelLaneRoutingStageStateKind::InFlight,
                                Some(persisted_instance_id.clone()),
                                None,
                                None,
                            )
                            .await
                        {
                            let cancelled = canonical_cancelled_routing_dispatch(
                                execution_store,
                                &claim,
                                Some(&persisted_instance_id),
                            )
                            .await
                            .map_err(|snapshot_err| {
                                SwarmError::LedgerFailed(format!(
                                    "routing heartbeat failed ({err}); canonical cancellation reconciliation also failed: {snapshot_err}"
                                ))
                            })?;
                            if let Some(cancelled) = cancelled {
                                return Ok(cancelled);
                            }
                            execution_store
                                .validate_active_claim(&claim)
                                .await
                                .map_err(|validation_err| {
                                    SwarmError::LedgerFailed(format!(
                                        "routing heartbeat failed ({err}); worker claim is no longer current and its stable instance id was not cancelled: {validation_err}"
                                    ))
                                })?;
                            self.cancel_session_generation(
                                instance_id,
                                spawn_generation,
                                "routing claim heartbeat rejected for current worker",
                            )
                            .await?;
                            return Err(SwarmError::LedgerFailed(format!(
                                "routing heartbeat failed: {err}"
                            )));
                        }
                        continue;
                    }
                    token = stream.next() => token,
                };
                let Some(token) = token else { break };
                match token {
                    Ok(token) => output.push_str(&token.text),
                    Err(err) => {
                        let detail = err.to_string();
                        let failed_transition = execution_store
                            .record_transition(
                                &claim,
                                ModelLaneRoutingStageStateKind::Failed,
                                Some(persisted_instance_id.clone()),
                                Some(detail.clone()),
                            )
                            .await;
                        if let Err(persist_err) = failed_transition {
                            // Authoritative cancellation clears the stage
                            // lease/fence before it signals the live stream. The
                            // resulting cancellation error reaches this worker
                            // with an intentionally stale claim. Accept only the
                            // exact cancelled stage attempt and instance; every
                            // other stale-claim or persistence error remains hard.
                            let cancelled = canonical_cancelled_routing_dispatch(
                                execution_store,
                                &claim,
                                Some(&persisted_instance_id),
                            )
                                .await
                                .map_err(|snapshot_err| {
                                    SwarmError::LedgerFailed(format!(
                                        "routing generation failed ({detail}) and persistence failed: {persist_err}; canonical cancellation reconciliation also failed: {snapshot_err}"
                                    ))
                                })?;
                            if let Some(cancelled) = cancelled {
                                return Ok(cancelled);
                            }
                            return Err(SwarmError::LedgerFailed(format!(
                                "routing generation failed ({detail}) and persistence failed: {persist_err}"
                            )));
                        }
                        self.cancel_session_generation(instance_id, spawn_generation, &detail)
                            .await
                            .map_err(|cleanup_err| {
                                SwarmError::LedgerFailed(format!(
                                    "routing generation failed ({detail}); stage failure persisted but session cleanup failed: {cleanup_err}"
                                ))
                            })?;
                        return Ok(ModelLaneRoutingStageDispatch {
                            stage_id: claim.stage_id,
                            dispatch_target: claim.dispatch_target,
                            state: ModelLaneRoutingStageStateKind::Failed,
                            instance_id: Some(persisted_instance_id),
                            detail: Some(detail),
                        });
                    }
                }
            }
            #[cfg(any(test, feature = "test-utils"))]
            let inject_output_failure = self
                .inner
                .fail_next_routing_output_persistence
                .swap(false, Ordering::SeqCst);
            #[cfg(not(any(test, feature = "test-utils")))]
            let inject_output_failure = false;
            let output_persistence = if inject_output_failure {
                Err("injected routing output persistence failure".to_string())
            } else {
                execution_store
                    .record_generated_output(
                        &claim,
                        ModelLaneRoutingStageStateKind::Succeeded,
                        Some(persisted_instance_id.clone()),
                        lane_id,
                        output,
                        None,
                    )
                    .await
            };
            let completed_execution = match output_persistence {
                Ok(execution) => execution,
                Err(persist_err) => {
                    let cleanup = self
                        .cancel_session_generation(
                            instance_id,
                            spawn_generation,
                            "routing output persistence rejected",
                        )
                        .await;
                    let cancelled = canonical_cancelled_routing_dispatch(
                        execution_store,
                        &claim,
                        Some(&persisted_instance_id),
                    )
                    .await
                    .map_err(|snapshot_err| {
                        SwarmError::LedgerFailed(format!(
                            "routing output persistence failed ({persist_err}); canonical cancellation reconciliation also failed: {snapshot_err}"
                        ))
                    })?;
                    if let Some(cancelled) = cancelled {
                        cleanup.map_err(|cleanup_err| {
                            SwarmError::LedgerFailed(format!(
                                "routing output persistence was cancelled canonically, but exact spawned-session cleanup failed: {cleanup_err}"
                            ))
                        })?;
                        return Ok(cancelled);
                    }
                    cleanup.map_err(|cleanup_err| {
                        SwarmError::LedgerFailed(format!(
                            "routing output persistence failed ({persist_err}); exact spawned-session cleanup also failed: {cleanup_err}"
                        ))
                    })?;
                    return Err(SwarmError::LedgerFailed(format!(
                        "routing output persistence failed: {persist_err}"
                    )));
                }
            };
            let has_authority_successor = completed_execution
                .canonical_graph
                .get("stages")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|stages| {
                    stages.iter().any(|candidate| {
                        matches!(
                            candidate.get("target").and_then(serde_json::Value::as_str),
                            Some("validator" | "operator")
                        ) && candidate
                            .get("depends_on")
                            .and_then(serde_json::Value::as_array)
                            .is_some_and(|dependencies| {
                                dependencies.iter().any(|dependency| {
                                    dependency.as_str() == Some(claim.stage_id.as_str())
                                })
                            })
                    })
                });
            if !has_authority_successor {
                self.complete_session_generation(instance_id, spawn_generation)
                    .await?;
            }
            Ok(ModelLaneRoutingStageDispatch {
                stage_id: claim.stage_id,
                dispatch_target: claim.dispatch_target,
                state: ModelLaneRoutingStageStateKind::Succeeded,
                instance_id: Some(persisted_instance_id),
                detail: None,
            })
                    }
                    .await;
                    (origin_claim, result)
                }
            })
            .collect();

        let mut dispatched = Vec::new();
        // Results that completed WHILE we were awaiting sibling cancellation.
        // They must be queued rather than dropped: a completed dispatch is
        // durable state the caller still has to see.
        let mut queued: std::collections::VecDeque<(
            super::routing_execution::ModelLaneRoutingStageClaim,
            SwarmResult<super::routing_execution::ModelLaneRoutingStageDispatch>,
        )> = std::collections::VecDeque::new();
        loop {
            let (origin_claim, result) = match queued.pop_front() {
                Some(result) => result,
                None => match pending.next().await {
                    Some(result) => result,
                    None => break,
                },
            };
            let dispatch = match result {
                Ok(dispatch) => dispatch,
                Err(err) => {
                    #[cfg(any(test, feature = "test-utils"))]
                    let routing_pause = {
                        self.inner
                            .routing_before_dispatch_error_cleanup_pause
                            .lock()
                            .expect("routing dispatch-error pause poisoned")
                            .take()
                    };
                    #[cfg(any(test, feature = "test-utils"))]
                    if let Some((arrived, release)) = routing_pause {
                        arrived.notify_one();
                        release.notified().await;
                    }
                    let origin_still_current = execution_store
                        .snapshot(execution_id)
                        .await
                        .map_err(|snapshot_error| {
                            SwarmError::LedgerFailed(format!(
                                "routing dispatch failed ({err}); originating-claim verification failed: {snapshot_error}"
                            ))
                        })?
                        .and_then(|execution| {
                            execution.stages.get(&origin_claim.stage_id).cloned()
                        })
                        .is_some_and(|stage| {
                            stage.attempt == origin_claim.attempt
                                && stage.fencing_token.as_deref()
                                    == Some(origin_claim.fencing_token.as_str())
                        });
                    if origin_still_current {
                        self.cancel_siblings_while_driving(
                            &mut pending,
                            &mut queued,
                            execution_id,
                            None,
                            "routing dispatch error; immediate terminal cleanup",
                        )
                        .await?;
                    }
                    return Err(err);
                }
            };
            if dispatch.state == ModelLaneRoutingStageStateKind::Failed {
                self.cancel_siblings_while_driving(
                    &mut pending,
                    &mut queued,
                    execution_id,
                    Some(&dispatch.stage_id),
                    "routing sibling failed; immediate terminal cleanup",
                )
                .await?;
            }
            dispatched.push(dispatch);
        }
        let execution = execution_store
            .snapshot(execution_id)
            .await
            .map_err(|err| SwarmError::LedgerFailed(format!("routing snapshot failed: {err}")))?
            .ok_or_else(|| {
                SwarmError::LedgerFailed(format!(
                    "routing execution {execution_id} disappeared after dispatch"
                ))
            })?;
        Ok(super::routing_execution::ModelLaneRoutingDispatchBatch {
            execution,
            dispatched,
        })
    }

    /// Cancel active routing siblings WITHOUT freezing the in-flight stage
    /// futures, by continuing to poll `pending` for the whole cancellation.
    ///
    /// This helper retains polling while cancellation is awaited because a
    /// suspended stage future may still own an open transaction. It is a
    /// defensive race closure, not the measured fix for MT-009's cancellation
    /// hang; the proven hang mechanism was the re-entrant advisory-lock wait and
    /// the effective bound is the transaction-local lock timeout.
    /// `FuturesUnordered` only advances while it is polled, and a suspended
    /// stage future can still own an OPEN transaction holding the
    /// execution-keyed `pg_advisory_xact_lock`. Awaiting cancellation without
    /// polling `pending` therefore deadlocks the worker task against itself:
    /// `cancel_execution` requests the very lock the frozen future holds, and
    /// that future can only resume once this await returns.
    ///
    /// Any dispatch results that land during cancellation are pushed onto
    /// `queued` so the caller still processes them; dropping them would lose
    /// durable stage outcomes.
    async fn cancel_siblings_while_driving<S>(
        &self,
        pending: &mut S,
        queued: &mut std::collections::VecDeque<(
            super::routing_execution::ModelLaneRoutingStageClaim,
            SwarmResult<super::routing_execution::ModelLaneRoutingStageDispatch>,
        )>,
        execution_id: &str,
        except_stage_id: Option<&str>,
        reason: &str,
    ) -> SwarmResult<()>
    where
        S: futures::Stream<
                Item = (
                    super::routing_execution::ModelLaneRoutingStageClaim,
                    SwarmResult<super::routing_execution::ModelLaneRoutingStageDispatch>,
                ),
            > + Unpin,
    {
        use futures::StreamExt as _;
        let cancel = self.cancel_active_routing_siblings(execution_id, except_stage_id, reason);
        futures::pin_mut!(cancel);
        loop {
            tokio::select! {
                // Biased so a completed cancellation wins immediately instead of
                // waiting on another stage poll.
                biased;
                outcome = &mut cancel => return outcome,
                item = pending.next() => match item {
                    Some(item) => queued.push_back(item),
                    // Nothing left to drive; a plain await cannot deadlock now.
                    None => return cancel.await,
                },
            }
        }
    }

    async fn cancel_active_routing_siblings(
        &self,
        execution_id: &str,
        except_stage_id: Option<&str>,
        reason: &str,
    ) -> SwarmResult<()> {
        use super::routing_execution::{
            ModelLaneRoutingStageClaim, ModelLaneRoutingStageStateKind,
        };
        let store = self.inner.routing_execution_store.as_ref().ok_or_else(|| {
            SwarmError::LedgerFailed(
                "production routing requires a scoped ModelLane routing executor".into(),
            )
        })?;
        // Discover local authority independently of the durable Surreal path. If the
        // projection read or a stage transition fails, every exact routing
        // incarnation still has to receive its local cancellation fence.
        let live_targets: Vec<(
            ModelInstanceId,
            u64,
            CancellationToken,
            Arc<dyn crate::model_runtime::ModelRuntime>,
        )> = self
            .inner
            .registry
            .lock()
            .expect("registry poisoned")
            .values()
            .filter(|handle| {
                handle.routing_attempt.as_ref().is_some_and(|identity| {
                    identity.execution_id == execution_id
                        && except_stage_id != Some(identity.stage_id.as_str())
                })
            })
            .map(|handle| {
                (
                    handle.instance_id,
                    handle.spawn_generation,
                    handle.cancel.clone(),
                    Arc::clone(&handle.runtime),
                )
            })
            .collect();
        let pending_targets: Vec<CancellationToken> = self
            .inner
            .pending_spawns
            .lock()
            .expect("pending spawns poisoned")
            .values()
            .filter(|pending| {
                pending.routing_attempt.as_ref().is_some_and(|identity| {
                    identity.execution_id == execution_id
                        && except_stage_id != Some(identity.stage_id.as_str())
                })
            })
            .map(|pending| pending.cancel.clone())
            .collect();

        let mut errors = Vec::new();
        let active: Vec<_> = match store.snapshot(execution_id).await {
            Ok(Some(execution)) => execution
                .stages
                .values()
                .filter(|stage| except_stage_id != Some(stage.stage_id.as_str()))
                .filter(|stage| {
                    matches!(
                        stage.state,
                        ModelLaneRoutingStageStateKind::Claimed
                            | ModelLaneRoutingStageStateKind::InFlight
                            | ModelLaneRoutingStageStateKind::AwaitingAuthority
                    )
                })
                .cloned()
                .collect(),
            Ok(None) => {
                errors.push(format!(
                    "routing execution {execution_id} disappeared during sibling cancellation"
                ));
                Vec::new()
            }
            Err(error) => {
                errors.push(format!(
                    "routing sibling-cancellation snapshot failed: {error}"
                ));
                Vec::new()
            }
        };

        // Make the durable stage cancellation authoritative before its stream
        // observes a cancellation error. Otherwise the worker can win a
        // Cancelled-vs-Failed race with the same claim. No runtime teardown is
        // awaited until every target has crossed this durable fence.
        for stage in active {
            let (Some(fencing_token), Some(lease_owner), Some(lease_expires_at_unix_ms)) = (
                stage.fencing_token.clone(),
                stage.lease_owner.clone(),
                stage.lease_expires_at_unix_ms,
            ) else {
                errors.push(format!(
                    "active routing stage {} has incomplete claim authority",
                    stage.stage_id
                ));
                continue;
            };
            let claim = ModelLaneRoutingStageClaim {
                execution_id: execution_id.to_string(),
                stage_id: stage.stage_id.clone(),
                attempt: stage.attempt,
                fencing_token,
                lease_owner,
                lease_expires_at_unix_ms,
                dispatch_target: stage.dispatch_target,
                expected_run_id: stage.expected_run_id.clone(),
                expected_lane_id: stage.expected_lane_id.clone(),
                expected_model_id: stage.expected_model_id.clone(),
                expected_provider: stage.expected_provider,
            };
            #[cfg(any(test, feature = "test-utils"))]
            let inject_persistence_failure = self
                .inner
                .fail_next_sibling_cancellation_persistence
                .swap(false, Ordering::SeqCst);
            #[cfg(not(any(test, feature = "test-utils")))]
            let inject_persistence_failure = false;
            let transition = if inject_persistence_failure {
                Err("injected sibling cancellation persistence failure".to_string())
            } else {
                store
                    .record_transition(
                        &claim,
                        ModelLaneRoutingStageStateKind::Cancelled,
                        stage.instance_id.clone(),
                        Some(reason.to_string()),
                    )
                    .await
                    .map(|_| ())
            };
            if let Err(error) = transition {
                errors.push(format!(
                    "routing sibling cancellation persistence failed for {} attempt {}: {error}",
                    stage.stage_id, stage.attempt
                ));
            }
        }

        // Fence all live and pending siblings before awaiting the first one's
        // teardown, so a slow cleanup owner cannot leave later siblings running.
        for (_, _, cancel, runtime) in &live_targets {
            cancel.cancel();
            runtime.cancel(cancel.clone());
        }
        for cancel in pending_targets {
            cancel.cancel();
        }
        for (instance_id, generation, _, _) in live_targets {
            match self
                .cancel_session_generation(instance_id, generation, reason)
                .await
            {
                Ok(()) => {}
                Err(SwarmError::UnknownInstance(missing)) if missing == instance_id => {}
                Err(error) => errors.push(format!(
                    "routing sibling exact cleanup failed for {instance_id} generation {generation}: {error}"
                )),
            }
        }
        if !errors.is_empty() {
            return Err(SwarmError::LedgerFailed(errors.join(" | ")));
        }
        Ok(())
    }

    pub async fn execute_routing_wave(
        &self,
        execution_id: &str,
        selecting_decision_id: &str,
        authority: &super::routing::ModelLaneRoutingAuthority,
        context: super::routing_execution::ModelLaneRoutingExecutionContext,
        launches: Vec<super::routing_execution::ModelLaneRoutingStageLaunch>,
    ) -> SwarmResult<super::routing_execution::ModelLaneRoutingDispatchBatch> {
        let store = self.inner.routing_execution_store.as_ref().ok_or_else(|| {
            SwarmError::LedgerFailed(
                "production routing requires a scoped embedded-Surreal ModelLaneStore".into(),
            )
        })?;
        store
            .begin_execution(execution_id, selecting_decision_id, authority, context)
            .await
            .map_err(|err| SwarmError::LedgerFailed(format!("routing start failed: {err}")))?;
        self.dispatch_ready_routing_stages(execution_id, launches)
            .await
    }

    pub async fn execute_routing_lifecycle(
        &self,
        execution_id: &str,
        selecting_decision_id: &str,
        authority: &super::routing::ModelLaneRoutingAuthority,
        context: super::routing_execution::ModelLaneRoutingExecutionContext,
        launches: Vec<super::routing_execution::ModelLaneRoutingStageLaunch>,
    ) -> SwarmResult<super::routing_execution::ModelLaneRoutingDispatchBatch> {
        let store = self.inner.routing_execution_store.as_ref().ok_or_else(|| {
            SwarmError::LedgerFailed(
                "production routing requires a scoped embedded-Surreal ModelLaneStore".into(),
            )
        })?;
        store
            .begin_execution(execution_id, selecting_decision_id, authority, context)
            .await
            .map_err(|err| SwarmError::LedgerFailed(format!("routing start failed: {err}")))?;
        self.drive_routing_lifecycle(execution_id, launches).await
    }

    async fn drive_routing_lifecycle(
        &self,
        execution_id: &str,
        launches: Vec<super::routing_execution::ModelLaneRoutingStageLaunch>,
    ) -> SwarmResult<super::routing_execution::ModelLaneRoutingDispatchBatch> {
        use super::routing_execution::ModelLaneRoutingExecutionStatus;
        self.retry_pending_session_cleanups().await?;
        let mut dispatched = Vec::new();
        loop {
            let batch = self
                .dispatch_ready_routing_stages(execution_id, launches.clone())
                .await?;
            let made_progress = !batch.dispatched.is_empty();
            dispatched.extend(batch.dispatched);
            if matches!(
                batch.execution.status,
                ModelLaneRoutingExecutionStatus::AwaitingAuthority
                    | ModelLaneRoutingExecutionStatus::Succeeded
                    | ModelLaneRoutingExecutionStatus::Failed
                    | ModelLaneRoutingExecutionStatus::Cancelled
            ) || !made_progress
            {
                return Ok(super::routing_execution::ModelLaneRoutingDispatchBatch {
                    execution: batch.execution,
                    dispatched,
                });
            }
        }
    }

    pub async fn complete_authority_and_resume_routing_lifecycle(
        &self,
        execution_id: &str,
        stage_id: &str,
        message_id: &str,
        launches: Vec<super::routing_execution::ModelLaneRoutingStageLaunch>,
    ) -> SwarmResult<super::routing_execution::ModelLaneRoutingDispatchBatch> {
        self.complete_authority_routing_stage(execution_id, stage_id, message_id)
            .await?;
        self.drive_routing_lifecycle(execution_id, launches).await
    }

    pub async fn complete_authority_routing_stage(
        &self,
        execution_id: &str,
        stage_id: &str,
        message_id: &str,
    ) -> SwarmResult<super::routing_execution::ModelLaneRoutingExecutionState> {
        use super::routing::ModelLaneRoutingDispatchTarget;
        let execution_store = self.inner.routing_execution_store.as_ref().ok_or_else(|| {
            SwarmError::LedgerFailed(
                "production routing requires a scoped ModelLane routing executor".into(),
            )
        })?;
        let model_lane_store = self.inner.model_lane_store.as_ref().ok_or_else(|| {
            SwarmError::LedgerFailed("authority completion requires ModelLaneStore".into())
        })?;
        let execution = execution_store
            .snapshot(execution_id)
            .await
            .map_err(|err| SwarmError::LedgerFailed(format!("routing snapshot failed: {err}")))?
            .ok_or_else(|| {
                SwarmError::LedgerFailed(format!("unknown routing execution {execution_id}"))
            })?;
        let stage = execution
            .stages
            .get(stage_id)
            .ok_or_else(|| SwarmError::LedgerFailed(format!("unknown routing stage {stage_id}")))?;
        let projection = model_lane_store
            .navigation_by_message(message_id)
            .await
            .map_err(|err| {
                SwarmError::LedgerFailed(format!("authority message lookup failed: {err}"))
            })?;
        let message = projection
            .messages
            .iter()
            .find(|message| message.message_id == message_id)
            .ok_or_else(|| {
                SwarmError::LedgerFailed(format!("ModelLaneMessage {message_id} missing"))
            })?;
        if stage.lane_id.as_deref() != Some(message.from_lane_id.as_str()) {
            return Err(SwarmError::LedgerFailed(format!(
                "authority message {message_id} is not from stage lane"
            )));
        }
        let authority_request_ref =
            stage
                .authority_request_message_ref
                .as_deref()
                .ok_or_else(|| {
                    SwarmError::LedgerFailed(format!(
                        "authority stage {stage_id} has no causal request message"
                    ))
                })?;
        if message.run_id != execution.run_id
            || message.trace_id != execution.trace_id
            || message.coordinator_session_id != execution.coordinator_session_id
            || message.work_packet_id.as_deref() != Some(execution.work_packet_id.as_str())
            || message
                .routing
                .as_ref()
                .and_then(|routing| routing.ack_for.as_deref())
                != Some(authority_request_ref)
            || message
                .diagnostic_payload
                .get("execution_id")
                .and_then(serde_json::Value::as_str)
                != Some(execution_id)
            || message
                .diagnostic_payload
                .get("stage_id")
                .and_then(serde_json::Value::as_str)
                != Some(stage_id)
        {
            return Err(SwarmError::LedgerFailed(format!(
                "authority message {message_id} is not causally bound to execution/run/trace/predecessor/request"
            )));
        }
        let authority_ref = stage.authority_ref.as_deref().ok_or_else(|| {
            SwarmError::LedgerFailed(format!("authority stage {stage_id} has no authority ref"))
        })?;
        let valid_authority = match stage.dispatch_target {
            ModelLaneRoutingDispatchTarget::Validator => {
                message.authority == super::model_lane::ModelLaneAuthority::ValidatorVerdict
                    && message.validator_verdict_ref.as_deref() == Some(authority_ref)
            }
            ModelLaneRoutingDispatchTarget::Operator => {
                message.authority == super::model_lane::ModelLaneAuthority::OperatorDecision
                    && message.operator_decision_ref.as_deref() == Some(authority_ref)
            }
            _ => false,
        };
        if !valid_authority {
            return Err(SwarmError::LedgerFailed(format!(
                "ModelLaneMessage {message_id} does not satisfy stage authority"
            )));
        }
        let artifact = projection
            .artifacts
            .iter()
            .find(|artifact| artifact.artifact_ref == message.payload_ref)
            .ok_or_else(|| {
                SwarmError::LedgerFailed(format!(
                    "authority message {message_id} has no durable payload binding"
                ))
            })?;
        let payload = artifact.payload_json.clone();
        let hash = super::routing_execution::canonical_sha256(&payload)
            .map_err(SwarmError::LedgerFailed)?;
        if hash != message.payload_sha256 || artifact.artifact_sha256 != message.payload_sha256 {
            return Err(SwarmError::LedgerFailed(format!(
                "authority message {message_id} payload hash does not match its durable artifact binding"
            )));
        }
        let claim = super::routing_execution::ModelLaneRoutingStageClaim {
            execution_id: execution_id.to_string(),
            stage_id: stage_id.to_string(),
            attempt: stage.attempt,
            fencing_token: stage.fencing_token.clone().ok_or_else(|| {
                SwarmError::LedgerFailed(format!("authority stage {stage_id} has no fencing token"))
            })?,
            lease_owner: stage.lease_owner.clone().ok_or_else(|| {
                SwarmError::LedgerFailed(format!("authority stage {stage_id} has no lease owner"))
            })?,
            lease_expires_at_unix_ms: stage.lease_expires_at_unix_ms.ok_or_else(|| {
                SwarmError::LedgerFailed(format!("authority stage {stage_id} has no lease expiry"))
            })?,
            dispatch_target: stage.dispatch_target,
            expected_run_id: stage.expected_run_id.clone(),
            expected_lane_id: stage.expected_lane_id.clone(),
            expected_model_id: stage.expected_model_id.clone(),
            expected_provider: stage.expected_provider,
        };
        execution_store
            .complete_authority_stage(
                &claim,
                authority_ref,
                message.payload_ref.clone(),
                message.message_id.clone(),
                hash,
                payload,
            )
            .await
            .map_err(|err| SwarmError::LedgerFailed(format!("authority completion failed: {err}")))
    }

    pub async fn recover_routing_execution(
        &self,
        execution_id: &str,
        launches: Vec<super::routing_execution::ModelLaneRoutingStageLaunch>,
    ) -> SwarmResult<super::routing_execution::ModelLaneRoutingDispatchBatch> {
        self.retry_pending_session_cleanups().await?;
        let store = self.inner.routing_execution_store.as_ref().ok_or_else(|| {
            SwarmError::LedgerFailed(
                "production routing requires a scoped ModelLane routing executor".into(),
            )
        })?;
        let expired = store
            .expired_stage_attempts(execution_id)
            .await
            .map_err(|err| {
                SwarmError::LedgerFailed(format!("routing recovery scan failed: {err}"))
            })?;
        let expired: Vec<(String, u32, ModelInstanceId)> = expired
            .into_iter()
            .map(|(stage_id, attempt, instance_id)| {
                parse_routing_model_instance_id(&instance_id)
                    .map(|instance_id| (stage_id, attempt, instance_id))
            })
            .collect::<SwarmResult<Vec<_>>>()?;
        let live_targets: Vec<(
            ModelInstanceId,
            u64,
            CancellationToken,
            Arc<dyn crate::model_runtime::ModelRuntime>,
        )> = self
            .inner
            .registry
            .lock()
            .expect("registry poisoned")
            .values()
            .filter(|handle| {
                handle.routing_attempt.as_ref().is_some_and(|identity| {
                    identity.execution_id == execution_id
                        && expired.iter().any(|(stage_id, attempt, instance_id)| {
                            stage_id == &identity.stage_id
                                && *attempt == identity.attempt
                                && instance_id == &handle.instance_id
                        })
                })
            })
            .map(|handle| {
                (
                    handle.instance_id,
                    handle.spawn_generation,
                    handle.cancel.clone(),
                    Arc::clone(&handle.runtime),
                )
            })
            .collect();
        let pending_targets: Vec<CancellationToken> = self
            .inner
            .pending_spawns
            .lock()
            .expect("pending spawns poisoned")
            .iter()
            .filter(|(instance_id, pending)| {
                pending.routing_attempt.as_ref().is_some_and(|identity| {
                    identity.execution_id == execution_id
                        && expired.iter().any(|(stage_id, attempt, expected_id)| {
                            stage_id == &identity.stage_id
                                && *attempt == identity.attempt
                                && expected_id == *instance_id
                        })
                })
            })
            .map(|(_, pending)| pending.cancel.clone())
            .collect();
        for (_, _, cancel, runtime) in &live_targets {
            cancel.cancel();
            runtime.cancel(cancel.clone());
        }
        for cancel in pending_targets {
            cancel.cancel();
        }
        for (instance_id, generation, _, _) in live_targets {
            match self
                .cancel_session_generation(
                    instance_id,
                    generation,
                    "routing stage lease expired; compensate before retry",
                )
                .await
            {
                Ok(()) | Err(SwarmError::UnknownInstance(_)) => {}
                Err(error) => return Err(error),
            }
        }
        store
            .recover_expired_claims(execution_id)
            .await
            .map_err(|err| SwarmError::LedgerFailed(format!("routing recovery failed: {err}")))?;
        self.dispatch_ready_routing_stages(execution_id, launches)
            .await
    }

    /// How many times cancellation re-reads the execution projection looking for
    /// sessions that were spawned after the previous pass. Small on purpose: it
    /// exists to close a narrow spawn/transition race, not to poll indefinitely.
    const CANCELLATION_SETTLE_PASSES: usize = 3;

    pub async fn cancel_routing_execution(
        &self,
        execution_id: &str,
        reason: impl Into<String>,
    ) -> SwarmResult<super::routing_execution::ModelLaneRoutingExecutionState> {
        let store = self.inner.routing_execution_store.as_ref().ok_or_else(|| {
            SwarmError::LedgerFailed(
                "production routing requires a scoped ModelLane routing executor".into(),
            )
        })?;
        // Persist the cancellation BEFORE tearing down the runtime sessions.
        //
        // Tearing down first makes every in-flight stage fail, the worker then
        // terminalizes the execution as Failed, and this persistence step is
        // refused by cancel_execution's terminal guard with "terminal routing
        // execution cannot be cancelled" - so the operator's cancel never
        // becomes the recorded outcome. AC-9 requires the run to end Cancelled,
        // which means the durable transition has to win that race.
        //
        // Ordering the other way is not merely cosmetic: a teardown failure
        // after this point leaves a recoverable orphan that process-ledger
        // reclaim already owns, whereas a persistence failure after teardown
        // leaves a run whose recorded outcome contradicts the operator action
        // and cannot be repaired from state.
        let cancelled = store
            .cancel_execution(execution_id, reason)
            .await
            .map_err(|err| {
                SwarmError::LedgerFailed(format!("routing cancellation persistence failed: {err}"))
            })?;
        let cancelled_attempts: Vec<(String, u32, ModelInstanceId)> = cancelled
            .stages
            .values()
            .filter_map(|stage| {
                stage.instance_id.as_deref().map(|instance_id| {
                    parse_routing_model_instance_id(instance_id)
                        .map(|parsed| (stage.stage_id.clone(), stage.attempt, parsed))
                })
            })
            .collect::<SwarmResult<Vec<_>>>()?;

        // Atomically fence routing workers that have completed (or are about
        // to complete) their final canonical read but have not yet published a
        // PendingSpawn. If publication already won, the admission entry is gone
        // and the exact pending token is visible to the scans below.
        {
            let mut admissions = self
                .inner
                .routing_spawn_admissions
                .lock()
                .expect("routing spawn admissions poisoned");
            for (stage_id, attempt, instance_id) in &cancelled_attempts {
                let key = (
                    RoutingAttemptIdentity {
                        execution_id: execution_id.to_string(),
                        stage_id: stage_id.clone(),
                        attempt: *attempt,
                    },
                    *instance_id,
                );
                if let Some(cancelled) = admissions.get_mut(&key) {
                    *cancelled = true;
                }
            }
        }

        // Fence only the exact routing attempts terminalized above. Stable
        // ModelInstanceId values are reusable, so a stale worker must never
        // cancel a replacement incarnation. Pending registration occurs before
        // spawn's first await; fixed-attempt rescans catch a pending->live handoff
        // without relying on the now-terminal projection's "active" filter.
        let mut cleaned_live: std::collections::BTreeSet<(String, u64)> =
            std::collections::BTreeSet::new();
        let mut fenced_pending: std::collections::BTreeSet<(String, u64)> =
            std::collections::BTreeSet::new();
        let mut cleanup_errors = Vec::new();
        for pass in 0..Self::CANCELLATION_SETTLE_PASSES {
            let live_targets: Vec<(
                ModelInstanceId,
                u64,
                CancellationToken,
                Arc<dyn crate::model_runtime::ModelRuntime>,
            )> = self
                .inner
                .registry
                .lock()
                .expect("registry poisoned")
                .values()
                .filter(|handle| {
                    handle.routing_attempt.as_ref().is_some_and(|identity| {
                        identity.execution_id == execution_id
                            && cancelled_attempts
                                .iter()
                                .any(|(stage_id, attempt, instance_id)| {
                                    stage_id == &identity.stage_id
                                        && *attempt == identity.attempt
                                        && instance_id == &handle.instance_id
                                })
                    })
                })
                .map(|handle| {
                    (
                        handle.instance_id,
                        handle.spawn_generation,
                        handle.cancel.clone(),
                        Arc::clone(&handle.runtime),
                    )
                })
                .collect();
            let pending_targets: Vec<(ModelInstanceId, u64, CancellationToken)> = self
                .inner
                .pending_spawns
                .lock()
                .expect("pending spawns poisoned")
                .iter()
                .filter(|(instance_id, pending)| {
                    pending.routing_attempt.as_ref().is_some_and(|identity| {
                        identity.execution_id == execution_id
                            && cancelled_attempts
                                .iter()
                                .any(|(stage_id, attempt, expected_id)| {
                                    stage_id == &identity.stage_id
                                        && *attempt == identity.attempt
                                        && expected_id == *instance_id
                                })
                    })
                })
                .map(|(instance_id, pending)| {
                    (
                        *instance_id,
                        pending.spawn_generation,
                        pending.cancel.clone(),
                    )
                })
                .collect();

            // Fence every discovered sibling before awaiting any one cleanup.
            for (_, _, cancel, runtime) in &live_targets {
                cancel.cancel();
                runtime.cancel(cancel.clone());
            }
            for (instance_id, generation, cancel) in pending_targets {
                if fenced_pending.insert((instance_id.to_string(), generation)) {
                    cancel.cancel();
                }
            }
            for (instance_id, generation, _, _) in live_targets {
                if !cleaned_live.insert((instance_id.to_string(), generation)) {
                    continue;
                }
                match self
                    .cancel_session_generation(
                        instance_id,
                        generation,
                        "routing execution cancelled",
                    )
                    .await
                {
                    Ok(()) => {}
                    Err(SwarmError::UnknownInstance(missing)) if missing == instance_id => {}
                    Err(error) => cleanup_errors.push(format!(
                        "routing execution cancellation cleanup failed for {instance_id} generation {generation}: {error}"
                    )),
                }
            }
            if pass + 1 < Self::CANCELLATION_SETTLE_PASSES {
                tokio::task::yield_now().await;
            }
        }
        if !cleanup_errors.is_empty() {
            return Err(SwarmError::LedgerFailed(cleanup_errors.join(" | ")));
        }
        Ok(cancelled)
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
        checkout_lease: Option<CheckoutLeaseGuard>,
        spawn_generation: u64,
    ) -> Result<(), TryInsertLoadingError> {
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
        let process_engine_kind = live
            .ledger_engine_kind_override
            .unwrap_or_else(|| process_engine_kind_for_request(request));

        {
            let mut registry = self.inner.registry.lock().expect("registry poisoned");
            if let Some(existing) = registry.get(&instance_id) {
                if !existing.state.is_terminal() {
                    // Lost the race — hand the session + permit back untouched.
                    return Err(TryInsertLoadingError::Duplicate {
                        live,
                        permit,
                        checkout_lease,
                    });
                }
            }
            if let Err(error) = self.emit_event(SwarmEvent::SessionSpawned {
                instance_id,
                parent_session_id: request.parent_session_id.clone(),
                process_uuid,
                swarm_id: request.swarm_id.clone(),
                worktree_id: request.worktree_id.clone(),
            }) {
                return Err(TryInsertLoadingError::EventSink {
                    live,
                    permit,
                    checkout_lease,
                    error,
                });
            }
            let handle = SessionHandle {
                instance_id,
                spawn_generation,
                routing_attempt: request.routing_attempt.clone(),
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
                ledger_os_pid: live.ledger_os_pid,
                ledger_start_override: live.ledger_start_override,
                ledger_lifecycle: live.ledger_lifecycle,
                process_engine_kind,
                parent_session_id: request.parent_session_id.clone(),
                runtime: live.runtime,
                llm_client: live.llm_client,
                teardown: Some(live.teardown),
                ready_hook: live.ready_hook,
                _checkout_lease: checkout_lease,
                cleanup: None,
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
                dexterity_consent_receipt_id: request
                    .dexterity_launch
                    .as_ref()
                    .and_then(|launch| launch.consent_receipt_ref.clone()),
            };
            registry.insert(instance_id, handle);
        }
        Ok(())
    }

    /// Atomically hand the committed-memory charge to the newly inserted
    /// registry handle. On rejection, the still-armed guard is returned beside
    /// the live resources so the compensating orphan owner can adopt it.
    #[allow(clippy::result_large_err)]
    fn try_insert_loading_with_memory_handoff(
        &self,
        request: &SpawnRequest,
        live: LiveSession,
        permit: OwnedSemaphorePermit,
        checkout_lease: Option<CheckoutLeaseGuard>,
        mut committed_memory_reservation: CommittedMemoryReservation,
        spawn_generation: u64,
    ) -> Result<(), (TryInsertLoadingError, CommittedMemoryReservation)> {
        match self.try_insert_loading(
            request,
            live,
            permit,
            committed_memory_reservation.bytes,
            checkout_lease,
            spawn_generation,
        ) {
            Ok(()) => {
                // `SessionHandle::committed_memory_bytes` and its owned permit
                // are now the sole capacity owner. Disarm before any caller can
                // cross an await or be cancelled.
                committed_memory_reservation.disarm();
                Ok(())
            }
            Err(error) => Err((error, committed_memory_reservation)),
        }
    }

    async fn rollback_unregistered_after_factory_create(
        &self,
        request: &SpawnRequest,
        live: LiveSession,
        checkout_lease: Option<CheckoutLeaseGuard>,
        permit: OwnedSemaphorePermit,
        committed_memory_reservation: CommittedMemoryReservation,
        cleanup_reason: &str,
        primary_error: SwarmError,
    ) -> SwarmResult<ModelInstanceId> {
        match self
            .teardown_orphan_with_capacity(
                request,
                live,
                checkout_lease,
                permit,
                committed_memory_reservation,
                cleanup_reason,
            )
            .await
        {
            Ok(()) => Err(primary_error),
            Err(cleanup_error) => Err(SwarmError::LedgerFailed(format!(
                "{primary_error}; live-session rollback also failed: {cleanup_error}"
            ))),
        }
    }

    /// Compensate a factory-created session that lost the atomic registry
    /// insertion race. The ready hook remains uncommitted, the concurrency
    /// permit remains owned until teardown plus the matching ledger STOP
    /// complete, and only then is the typed duplicate result returned. Holding
    /// the permit prevents a replacement spawn from exceeding the configured
    /// live-resource cap while the duplicate loser is still alive.
    async fn rollback_duplicate_after_factory_create(
        &self,
        request: &SpawnRequest,
        live: LiveSession,
        permit: OwnedSemaphorePermit,
        checkout_lease: Option<CheckoutLeaseGuard>,
        committed_memory_reservation: CommittedMemoryReservation,
    ) -> SwarmResult<ModelInstanceId> {
        let instance_id = request.instance_id;
        self.teardown_orphan_with_capacity(
            request,
            live,
            checkout_lease,
            permit,
            committed_memory_reservation,
            "duplicate_instance_rollback",
        )
        .await?;
        Err(SwarmError::DuplicateInstance(instance_id))
    }

    /// A rejected durable `SessionSpawned` write must not publish a live
    /// registry handle. Roll back the still-owned factory session through the
    /// same orphan teardown/STOP path as a duplicate while retaining the
    /// original event-sink failure as the externally visible error.
    async fn rollback_spawn_event_failure_after_factory_create(
        &self,
        request: &SpawnRequest,
        live: LiveSession,
        permit: OwnedSemaphorePermit,
        checkout_lease: Option<CheckoutLeaseGuard>,
        committed_memory_reservation: CommittedMemoryReservation,
        event_error: SwarmError,
    ) -> SwarmResult<ModelInstanceId> {
        let teardown_result = self
            .teardown_orphan_with_capacity(
                request,
                live,
                checkout_lease,
                permit,
                committed_memory_reservation,
                "session_spawned_event_persistence_failed",
            )
            .await;
        match teardown_result {
            Ok(()) => Err(event_error),
            Err(cleanup_error) => Err(SwarmError::EventSinkFailed(format!(
                "{event_error}; live-session rollback also failed: {cleanup_error}"
            ))),
        }
    }

    /// Test-only deterministic seam for the narrow window after factory create
    /// and before atomic registry insertion. A live winner must already occupy
    /// the requested instance id; this exercises the production duplicate
    /// rollback without relying on scheduler timing or bypassing teardown.
    #[cfg(test)]
    pub(crate) async fn duplicate_insert_after_factory_create_for_test(
        &self,
        request: SpawnRequest,
        live: LiveSession,
    ) -> SwarmResult<ModelInstanceId> {
        let committed_memory_bytes = request.committed_memory_bytes.unwrap_or(0);
        let committed_memory_reservation =
            CommittedMemoryReservation::reserve(Arc::clone(&self.inner), committed_memory_bytes)
                .map_err(|dimension| SwarmError::BudgetExhausted { dimension })?;
        let cap = self.inner.effective_max_concurrent.load(Ordering::SeqCst);
        let permit = self
            .inner
            .semaphore
            .clone()
            .try_acquire_owned()
            .map_err(|_| SwarmError::ConcurrencyCapReached {
                in_flight: cap.saturating_sub(self.inner.semaphore.available_permits()),
                cap,
            })?;
        let spawn_generation = self
            .inner
            .checkout_lease_generation
            .fetch_add(1, Ordering::SeqCst)
            + 1;
        match self.try_insert_loading_with_memory_handoff(
            &request,
            live,
            permit,
            None,
            committed_memory_reservation,
            spawn_generation,
        ) {
            Ok(()) => Err(SwarmError::Internal(format!(
                "duplicate insertion test seam expected a live winner for {}",
                request.instance_id
            ))),
            Err((
                TryInsertLoadingError::Duplicate {
                    live,
                    permit,
                    checkout_lease,
                },
                committed_memory_reservation,
            )) => {
                self.rollback_duplicate_after_factory_create(
                    &request,
                    live,
                    permit,
                    checkout_lease,
                    committed_memory_reservation,
                )
                .await
            }
            Err((
                TryInsertLoadingError::EventSink {
                    live,
                    permit,
                    checkout_lease,
                    error,
                },
                committed_memory_reservation,
            )) => {
                self.rollback_spawn_event_failure_after_factory_create(
                    &request,
                    live,
                    permit,
                    checkout_lease,
                    committed_memory_reservation,
                    error,
                )
                .await
            }
        }
    }

    /// Test-only cancellation seam that parks immediately after the production
    /// insertion + memory-ownership handoff. Aborting this future must not drop
    /// the registry-owned committed-memory charge or concurrency permit.
    #[cfg(test)]
    pub(crate) async fn successful_insert_ownership_handoff_for_test(
        &self,
        request: SpawnRequest,
        live: LiveSession,
    ) -> SwarmResult<ModelInstanceId> {
        let committed_memory_bytes = request.committed_memory_bytes.unwrap_or(0);
        let committed_memory_reservation =
            CommittedMemoryReservation::reserve(Arc::clone(&self.inner), committed_memory_bytes)
                .map_err(|dimension| SwarmError::BudgetExhausted { dimension })?;
        let cap = self.inner.effective_max_concurrent.load(Ordering::SeqCst);
        let permit = self
            .inner
            .semaphore
            .clone()
            .try_acquire_owned()
            .map_err(|_| SwarmError::ConcurrencyCapReached {
                in_flight: cap.saturating_sub(self.inner.semaphore.available_permits()),
                cap,
            })?;
        let spawn_generation = self
            .inner
            .checkout_lease_generation
            .fetch_add(1, Ordering::SeqCst)
            + 1;
        match self.try_insert_loading_with_memory_handoff(
            &request,
            live,
            permit,
            None,
            committed_memory_reservation,
            spawn_generation,
        ) {
            Ok(()) => {
                std::future::pending::<()>().await;
                Ok(request.instance_id)
            }
            Err((TryInsertLoadingError::Duplicate { .. }, _reservation)) => {
                Err(SwarmError::DuplicateInstance(request.instance_id))
            }
            Err((TryInsertLoadingError::EventSink { error, .. }, _reservation)) => Err(error),
        }
    }

    fn mark_dexterity_model_lane_persisted(&self, instance_id: ModelInstanceId) -> SwarmResult<()> {
        let mut registry = self.inner.registry.lock().expect("registry poisoned");
        let handle = registry
            .get_mut(&instance_id)
            .ok_or(SwarmError::UnknownInstance(instance_id))?;
        handle.dexterity_model_lane_persisted = true;
        Ok(())
    }

    fn take_pending_checkout_lease(
        &self,
        instance_id: ModelInstanceId,
    ) -> SwarmResult<Option<CheckoutLeaseGuard>> {
        let mut pending = self
            .inner
            .pending_spawns
            .lock()
            .expect("pending spawns poisoned");
        let spawn = pending
            .get_mut(&instance_id)
            .ok_or(SwarmError::UnknownInstance(instance_id))?;
        Ok(spawn.checkout_lease.take())
    }

    fn commit_ready_hook(&self, instance_id: ModelInstanceId) -> SwarmResult<()> {
        // Publication and the Ready-state check share the registry fence. A
        // concurrent terminal path therefore cannot remove the handle between
        // hook selection and publication and then let the hook republish a dead
        // runtime. SessionReadyHook is synchronous and must not re-enter the
        // coordinator; it may only publish the already-prepared secondary handle.
        let mut registry = self.inner.registry.lock().expect("registry poisoned");
        let handle = registry
            .get_mut(&instance_id)
            .ok_or(SwarmError::UnknownInstance(instance_id))?;
        if handle.state != ModelSessionState::Ready {
            return Err(SwarmError::LedgerFailed(format!(
                "ready hook for {instance_id} requires Ready state; got {:?}",
                handle.state
            )));
        }
        if let Some(hook) = handle.ready_hook.clone() {
            hook()?;
            handle.ready_hook = None;
        }
        Ok(())
    }

    /// Register and attempt cleanup for a session that never made it into the
    /// live registry. The orphan owner retains the permit, committed-memory
    /// charge, checkout lease, and teardown authority across failures; only
    /// successful teardown plus matching STOP removes the owner and releases
    /// capacity and cross-process locks.
    /// Transfer all still-live capacity ownership into the retryable orphan
    /// record before awaiting teardown. The committed-memory guard cannot be
    /// stored directly because it owns `Arc<Inner>` and would form an ownership
    /// cycle through `Inner::orphan_cleanups`; retain its byte liability instead.
    async fn teardown_orphan_with_capacity(
        &self,
        request: &SpawnRequest,
        live: LiveSession,
        checkout_lease: Option<CheckoutLeaseGuard>,
        permit: OwnedSemaphorePermit,
        mut committed_memory_reservation: CommittedMemoryReservation,
        reason: &str,
    ) -> SwarmResult<()> {
        let instance_id = request.instance_id;
        let process_record_id = live.process_record_id;
        let committed_memory_bytes = committed_memory_reservation.bytes;
        let process_engine_kind = live
            .ledger_engine_kind_override
            .unwrap_or_else(|| process_engine_kind_for_request(request));
        let stop = self.build_stop(
            process_record_id,
            live.ledger_os_pid,
            live.ledger_start_override.clone(),
            process_engine_kind,
            None,
            Utc::now(),
            ModelSessionState::Cancelled,
            reason,
            -1,
            &instance_id,
        );
        let cleanup = PendingOrphanCleanup {
            instance_id,
            cancel: live.cancel,
            runtime: live.runtime,
            teardown: live.teardown,
            stop,
            ledger_lifecycle: live.ledger_lifecycle,
            _permit: permit,
            committed_memory_bytes,
            _checkout_lease: checkout_lease,
            teardown_succeeded: false,
            stop_succeeded: false,
            owner_generation: 0,
            in_progress: false,
        };
        {
            let mut orphan_cleanups = self
                .inner
                .orphan_cleanups
                .lock()
                .expect("orphan cleanups poisoned");
            if orphan_cleanups.contains_key(&process_record_id) {
                return Err(SwarmError::LedgerFailed(format!(
                    "orphan cleanup owner already exists for process {}",
                    process_record_id.as_uuid()
                )));
            }
            orphan_cleanups.insert(process_record_id, cleanup);
        }
        committed_memory_reservation.disarm();
        self.retry_orphan_cleanup(process_record_id).await
    }

    async fn retry_orphan_cleanup(
        &self,
        process_record_id: ProcessOwnershipRecordId,
    ) -> SwarmResult<()> {
        let (
            instance_id,
            cancel,
            runtime,
            teardown,
            stop,
            ledger_lifecycle,
            teardown_succeeded,
            stop_succeeded,
            generation,
        ) = {
            let mut cleanups = self
                .inner
                .orphan_cleanups
                .lock()
                .expect("orphan cleanups poisoned");
            let cleanup = cleanups.get_mut(&process_record_id).ok_or_else(|| {
                SwarmError::LedgerFailed(format!(
                    "orphan cleanup {} is missing",
                    process_record_id.as_uuid()
                ))
            })?;
            if cleanup.in_progress {
                return Err(SwarmError::LedgerFailed(format!(
                    "orphan cleanup {} is already in progress at generation {}",
                    process_record_id.as_uuid(),
                    cleanup.owner_generation
                )));
            }
            cleanup.owner_generation = cleanup.owner_generation.saturating_add(1);
            cleanup.in_progress = true;
            (
                cleanup.instance_id,
                cleanup.cancel.clone(),
                Arc::clone(&cleanup.runtime),
                cleanup.teardown.clone(),
                cleanup.stop.clone(),
                cleanup.ledger_lifecycle.clone(),
                cleanup.teardown_succeeded,
                cleanup.stop_succeeded,
                cleanup.owner_generation,
            )
        };
        let _cleanup_owner = OrphanCleanupOwnershipGuard {
            inner: Arc::clone(&self.inner),
            process_record_id,
            generation,
        };

        cancel.cancel();
        runtime.cancel(cancel.clone());
        if !teardown_succeeded {
            self.run_teardown_bounded(instance_id, teardown).await?;
            let mut cleanups = self
                .inner
                .orphan_cleanups
                .lock()
                .expect("orphan cleanups poisoned");
            cleanups
                .get_mut(&process_record_id)
                .ok_or_else(|| {
                    SwarmError::LedgerFailed(format!(
                        "orphan cleanup {} disappeared after teardown",
                        process_record_id.as_uuid()
                    ))
                })?
                .teardown_succeeded = true;
        }

        if !stop_succeeded {
            if let Some(lifecycle) = ledger_lifecycle {
                ensure_reserved_stop_recorded(lifecycle.stop(
                    stop.exit_code,
                    stop.stop_reason.as_deref().unwrap_or("orphan_cleanup"),
                ))?;
            } else {
                self.inner
                    .ledger
                    .record_stop_lossless(stop)
                    .map_err(|err| SwarmError::LedgerFailed(err.to_string()))?;
            }
            let mut cleanups = self
                .inner
                .orphan_cleanups
                .lock()
                .expect("orphan cleanups poisoned");
            cleanups
                .get_mut(&process_record_id)
                .ok_or_else(|| {
                    SwarmError::LedgerFailed(format!(
                        "orphan cleanup {} disappeared after STOP",
                        process_record_id.as_uuid()
                    ))
                })?
                .stop_succeeded = true;
        }

        {
            let cleanups = self
                .inner
                .orphan_cleanups
                .lock()
                .expect("orphan cleanups poisoned");
            let cleanup = cleanups.get(&process_record_id).ok_or_else(|| {
                SwarmError::LedgerFailed(format!(
                    "orphan cleanup {} disappeared before completion",
                    process_record_id.as_uuid()
                ))
            })?;
            if !cleanup.teardown_succeeded || !cleanup.stop_succeeded {
                return Err(SwarmError::LedgerFailed(format!(
                    "orphan cleanup {} cannot release capacity before teardown and STOP completed",
                    process_record_id.as_uuid()
                )));
            }
        }
        self.emit_event(SwarmEvent::ResourceEvicted {
            instance_id,
            terminal_state: ModelSessionState::Cancelled,
            event_id: None,
        })?;
        let cleanup = self
            .inner
            .orphan_cleanups
            .lock()
            .expect("orphan cleanups poisoned")
            .remove(&process_record_id)
            .ok_or_else(|| {
                SwarmError::LedgerFailed(format!(
                    "orphan cleanup {} disappeared after completion event",
                    process_record_id.as_uuid()
                ))
            })?;
        self.inner
            .release_committed_memory(cleanup.committed_memory_bytes);
        // Dropping the completed owner releases its semaphore permit and
        // checkout lease only after teardown and STOP have both succeeded.
        drop(cleanup);
        Ok(())
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
        self.terminate_with_contention(
            instance_id,
            terminal,
            reason,
            exit_code,
            None,
            CleanupContentionPolicy::WaitForOwner,
        )
        .await
    }

    async fn terminate_generation(
        &self,
        instance_id: ModelInstanceId,
        spawn_generation: u64,
        terminal: ModelSessionState,
        reason: &str,
        exit_code: i32,
    ) -> SwarmResult<()> {
        self.terminate_with_contention(
            instance_id,
            terminal,
            reason,
            exit_code,
            Some(spawn_generation),
            CleanupContentionPolicy::WaitForOwner,
        )
        .await
    }

    async fn terminate_with_contention(
        &self,
        instance_id: ModelInstanceId,
        terminal: ModelSessionState,
        reason: &str,
        exit_code: i32,
        expected_spawn_generation: Option<u64>,
        contention: CleanupContentionPolicy,
    ) -> SwarmResult<()> {
        self.trace_terminate(instance_id, || {
            format!("entered terminal={terminal:?} reason={reason:?} exit_code={exit_code}")
        });
        loop {
            match self.claim_cleanup(
                instance_id,
                terminal,
                reason,
                exit_code,
                expected_spawn_generation,
            )? {
                CleanupClaim::Owner(claim) => {
                    // Only the single owner is detached from its caller. Dropping a
                    // JoinHandle is documented by Tokio to keep that task running;
                    // waiters remain ordinary caller-owned futures, so retries cannot
                    // accumulate detached tasks behind one slow owner.
                    let coordinator = self.clone();
                    let outcome_tx = claim.outcome_tx.clone();
                    let cleanup_owner = CleanupOwnershipGuard {
                        inner: Arc::clone(&self.inner),
                        instance_id,
                        spawn_generation: claim.spawn_generation,
                        generation: claim.generation,
                    };
                    return tokio::spawn(async move {
                        let _cleanup_owner = cleanup_owner;
                        let result = coordinator
                            .terminate_claimed(
                                instance_id,
                                claim.spawn_generation,
                                claim.terminal,
                                &claim.reason,
                                claim.exit_code,
                                claim.terminal_record,
                            )
                            .await;
                        let outcome = match &result {
                            Ok(()) => CleanupOwnerOutcome::Succeeded,
                            Err(error) => CleanupOwnerOutcome::Failed(error.to_string()),
                        };
                        outcome_tx.send_replace(outcome);
                        result
                    })
                    .await
                    .map_err(|join_error| {
                        SwarmError::LedgerFailed(format!(
                            "session {instance_id} owned terminal cleanup task failed to join: {join_error}"
                        ))
                    })?;
                }
                CleanupClaim::Wait(mut owner_outcome) => {
                    if contention == CleanupContentionPolicy::SkipIfOwned {
                        self.trace_terminate(instance_id, || "skip_owner_in_progress".to_string());
                        return Ok(());
                    }
                    self.trace_terminate(instance_id, || "wait_owner_in_progress".to_string());
                    // Each owner phase has its own product bound (durable writes,
                    // teardown, and reclaim). A single teardown_timeout here is
                    // shorter than the valid sum of those sequential phases and
                    // can make a healthy waiter report failure while its owner
                    // later succeeds. Wait for the owned task's actual published
                    // result; runtime shutdown closes the watch channel.
                    let observed = owner_outcome
                        .wait_for(|outcome| *outcome != CleanupOwnerOutcome::InProgress)
                        .await
                        .map_err(|_| {
                        SwarmError::LedgerFailed(format!(
                            "session {instance_id} cleanup owner ended without publishing an outcome"
                        ))
                        })?;
                    match (*observed).clone() {
                        CleanupOwnerOutcome::Succeeded => {
                            self.trace_terminate(instance_id, || {
                                "completed_by_concurrent_owner".to_string()
                            });
                            return Ok(());
                        }
                        CleanupOwnerOutcome::Failed(error) => {
                            self.trace_terminate(instance_id, || {
                                format!("concurrent_owner_failed {error}")
                            });
                            return Err(SwarmError::LedgerFailed(format!(
                                "session {instance_id} concurrent cleanup owner failed: {error}"
                            )));
                        }
                        CleanupOwnerOutcome::Idle => {
                            self.trace_terminate(instance_id, || {
                                "cleanup_owner_released_retrying".to_string()
                            });
                        }
                        CleanupOwnerOutcome::InProgress => unreachable!(
                            "wait_for only returns after terminal cleanup ownership changes"
                        ),
                    }
                }
                CleanupClaim::StaleGeneration => {
                    self.trace_terminate(instance_id, || {
                        format!("stale_spawn_generation expected={expected_spawn_generation:?}")
                    });
                    return Ok(());
                }
            }
        }
    }

    fn claim_cleanup(
        &self,
        instance_id: ModelInstanceId,
        terminal: ModelSessionState,
        reason: &str,
        exit_code: i32,
        expected_spawn_generation: Option<u64>,
    ) -> SwarmResult<CleanupClaim> {
        let mut registry = self.inner.registry.lock().expect("registry poisoned");
        let Some(handle) = registry.get_mut(&instance_id) else {
            self.trace_terminate(instance_id, || "err_unknown_instance_at_entry".to_string());
            return Err(SwarmError::UnknownInstance(instance_id));
        };
        if expected_spawn_generation.is_some_and(|expected| handle.spawn_generation != expected) {
            return Ok(CleanupClaim::StaleGeneration);
        }
        let cleanup_spawn_generation = handle.spawn_generation;
        if handle.state == ModelSessionState::Cancelling {
            let pending = handle.cleanup.as_mut().ok_or_else(|| {
                SwarmError::LedgerFailed(format!(
                    "session {instance_id} has a cancelling state without a cleanup receipt"
                ))
            })?;
            if let Some(invocation) = self
                .inner
                .managed_generations
                .lock()
                .expect("managed generations poisoned")
                .get_mut(&(instance_id, cleanup_spawn_generation))
            {
                invocation.disposition.get_or_insert_with(|| {
                    ManagedGenerationDisposition::from_session_terminal(
                        pending.terminal,
                        pending.reason.clone(),
                    )
                });
            }
            // First writer wins the terminal intent. A second operator waits for
            // that owner's published result; a background retry rejects quickly.
            if pending.in_progress {
                let step = format!(
                    "owner_in_progress pending_terminal={:?} pending_reason={:?} pending_gen={} pending_teardown_succeeded={} pending_stop_succeeded={}",
                    pending.terminal,
                    pending.reason,
                    pending.owner_generation,
                    pending.teardown_succeeded,
                    pending.stop_succeeded
                );
                self.trace_terminate(instance_id, || step);
                return Ok(CleanupClaim::Wait(pending.owner_outcome_tx.subscribe()));
            }
            let step = format!(
                "adopt_pending_intent authoritative_terminal={:?} authoritative_reason={:?} requested_terminal={terminal:?} requested_reason={reason:?} prev_gen={} prev_teardown_succeeded={} prev_stop_succeeded={}",
                pending.terminal,
                pending.reason,
                pending.owner_generation,
                pending.teardown_succeeded,
                pending.stop_succeeded
            );
            self.trace_terminate(instance_id, || step);
            pending.owner_generation = pending.owner_generation.saturating_add(1);
            pending.in_progress = true;
            // Each ownership generation gets a distinct outcome channel. Reusing
            // one watch sender lets a newer retry overwrite the prior owner's
            // Failed/Succeeded value before a delayed waiter observes it.
            let (outcome_tx, _outcome_rx) = watch::channel(CleanupOwnerOutcome::InProgress);
            pending.owner_outcome_tx = outcome_tx.clone();
            let generation = pending.owner_generation;
            let terminal_record = if handle.dexterity_model_lane_persisted {
                self.inner
                    .model_lane_store
                    .as_ref()
                    .cloned()
                    .zip(handle.dexterity_lane_id.clone())
            } else {
                None
            };
            return Ok(CleanupClaim::Owner(CleanupOwnerClaim {
                terminal: pending.terminal,
                reason: pending.reason.clone(),
                exit_code: pending.exit_code,
                terminal_record,
                spawn_generation: cleanup_spawn_generation,
                generation,
                outcome_tx,
            }));
        }
        if handle.state.is_terminal() {
            let step = format!("err_already_terminal state={:?}", handle.state);
            self.trace_terminate(instance_id, || step);
            return Err(SwarmError::UnknownInstance(instance_id));
        }
        let step = format!("fresh_cleanup from_state={:?}", handle.state);
        self.trace_terminate(instance_id, || step);
        if let Some(invocation) = self
            .inner
            .managed_generations
            .lock()
            .expect("managed generations poisoned")
            .get_mut(&(instance_id, cleanup_spawn_generation))
        {
            invocation.disposition.get_or_insert_with(|| {
                ManagedGenerationDisposition::from_session_terminal(terminal, reason.to_string())
            });
        }
        handle.state = ModelSessionState::Cancelling;
        let (outcome_tx, _outcome_rx) = watch::channel(CleanupOwnerOutcome::InProgress);
        handle.cleanup = Some(PendingSessionCleanup {
            terminal,
            reason: reason.to_string(),
            exit_code,
            teardown_succeeded: false,
            stop_succeeded: false,
            owner_generation: 1,
            in_progress: true,
            owner_outcome_tx: outcome_tx.clone(),
            terminal_event_id: Uuid::now_v7(),
            resource_evicted_event_id: Uuid::now_v7(),
        });
        let terminal_record = if handle.dexterity_model_lane_persisted {
            self.inner
                .model_lane_store
                .as_ref()
                .cloned()
                .zip(handle.dexterity_lane_id.clone())
        } else {
            None
        };
        Ok(CleanupClaim::Owner(CleanupOwnerClaim {
            terminal,
            reason: reason.to_string(),
            exit_code,
            terminal_record,
            spawn_generation: cleanup_spawn_generation,
            generation: 1,
            outcome_tx,
        }))
    }

    async fn terminate_claimed(
        &self,
        instance_id: ModelInstanceId,
        spawn_generation: u64,
        terminal: ModelSessionState,
        reason: &str,
        exit_code: i32,
        terminal_record: Option<(ModelLaneStore, String)>,
    ) -> SwarmResult<()> {
        // Fence provider work before any durable Surreal receipt can block. The
        // typed in-memory cleanup intent above survives a timed-out durable
        // write and is retried by the background/restart reconciliation path.
        {
            let registry = self.inner.registry.lock().expect("registry poisoned");
            let Some(handle) = registry
                .get(&instance_id)
                .filter(|handle| handle.spawn_generation == spawn_generation)
            else {
                self.trace_terminate(instance_id, || {
                    "err_unknown_or_stale_instance_at_cancel_fence".to_string()
                });
                return Err(SwarmError::UnknownInstance(instance_id));
            };
            handle.cancel.cancel();
            handle.runtime.cancel(handle.cancel.clone());
        }

        // A failed DURABLE WRITE must not abort the RUNTIME TEARDOWN.
        //
        // Both writes below precede the only step that actually tears the
        // session down (`run_teardown_bounded`). Returning early at either write
        // can therefore orphan a live model session under a bookkeeping failure,
        // even though measurement showed that this was not the intermittent
        // teardowns==1 mechanism fixed by the single-flight owner path above.
        //
        // This coordinator already argues the same way for the bounded factory
        // create: an orphan is recoverable through process-ledger reclaim, an
        // unrecoverable state is not. So the error is DEFERRED, not dropped -
        // teardown proceeds, and the first durable-write failure is still
        // returned from the function's normal exit, so nothing is silenced.
        let mut deferred_durable_error: Option<SwarmError> = None;
        if let Err(err) = self
            .record_cleanup_receipt(
                instance_id,
                "cleanup_pending",
                terminal,
                reason,
                exit_code,
                None,
            )
            .await
        {
            deferred_durable_error = Some(err);
        }

        if let Some(status) = model_lane_terminal_status(terminal) {
            if let Some((store, lane_id)) = terminal_record {
                let terminal_write = tokio::time::timeout(
                    self.inner.config.teardown_timeout,
                    store.record_lane_terminal_status(&lane_id, status, reason),
                )
                .await;
                let terminal_write_error = match terminal_write {
                    Ok(Ok(_record)) => None,
                    Ok(Err(error)) => Some(format!(
                        "Dexterity terminal lane state record failed: {error}"
                    )),
                    Err(_) => Some(format!(
                        "Dexterity terminal lane state record exceeded {:?}",
                        self.inner.config.teardown_timeout
                    )),
                };
                if let Some(error) = terminal_write_error {
                    let err = SwarmError::LedgerFailed(error);
                    // Keep the FIRST failure: it is the one closest to the
                    // original cause, and later writes may fail as a consequence.
                    deferred_durable_error.get_or_insert(err);
                }
            }
        }

        let (
            cancel,
            runtime,
            teardown,
            teardown_succeeded,
            mut stop_succeeded,
            stop,
            ledger_lifecycle,
            process_record_id,
            parent_session_id,
        ) = {
            let registry = self.inner.registry.lock().expect("registry poisoned");
            let Some(handle) = registry
                .get(&instance_id)
                .filter(|handle| handle.spawn_generation == spawn_generation)
            else {
                self.trace_terminate(instance_id, || {
                    "err_unknown_or_stale_instance_at_cleanup_read".to_string()
                });
                return Err(SwarmError::UnknownInstance(instance_id));
            };
            let Some(cleanup) = handle.cleanup.as_ref() else {
                self.trace_terminate(instance_id, || {
                    "err_cleanup_metadata_disappeared".to_string()
                });
                return Err(SwarmError::LedgerFailed(format!(
                    "session {instance_id} cleanup metadata disappeared"
                )));
            };
            let step = format!(
                "cleanup_read teardown_succeeded={} stop_succeeded={} gen={} deferred_durable_err={:?}",
                cleanup.teardown_succeeded,
                cleanup.stop_succeeded,
                cleanup.owner_generation,
                deferred_durable_error.as_ref().map(|err| err.to_string())
            );
            self.trace_terminate(instance_id, || step);
            (
                handle.cancel.clone(),
                Arc::clone(&handle.runtime),
                handle.teardown.clone(),
                cleanup.teardown_succeeded,
                cleanup.stop_succeeded,
                self.build_stop(
                    handle.process_record_id,
                    handle.ledger_os_pid,
                    handle.ledger_start_override.clone(),
                    handle.process_engine_kind,
                    Some(handle.parent_session_id.clone()),
                    handle.started_at,
                    terminal,
                    reason,
                    exit_code,
                    &instance_id,
                ),
                handle.ledger_lifecycle.clone(),
                handle.process_record_id,
                handle.parent_session_id.clone(),
            )
        };

        cancel.cancel();
        runtime.cancel(cancel.clone());

        if !teardown_succeeded {
            let Some(teardown) = teardown else {
                self.trace_terminate(instance_id, || "err_no_teardown_handle".to_string());
                return Err(SwarmError::LedgerFailed(format!(
                    "session {instance_id} cleanup has no teardown handle"
                )));
            };
            self.trace_terminate(instance_id, || "run_teardown_bounded_invoked".to_string());
            if let Err(err) = self.run_teardown_bounded(instance_id, teardown).await {
                let teardown_error = err.to_string();
                let step = format!("teardown_err {teardown_error}");
                self.trace_terminate(instance_id, || step);
                let pending_receipt_error = self
                    .record_cleanup_receipt(
                        instance_id,
                        "cleanup_pending",
                        terminal,
                        reason,
                        exit_code,
                        Some(&teardown_error),
                    )
                    .await
                    .err();
                let pending_receipt_context = pending_receipt_error
                    .as_ref()
                    .map(|receipt_error| {
                        format!(
                            "; cleanup_pending receipt also failed before reclaim: {receipt_error}"
                        )
                    })
                    .unwrap_or_default();
                let reclaimer = self
                    .inner
                    .process_reclaimer
                    .lock()
                    .expect("process reclaimer lock poisoned")
                    .clone();
                let Some(reclaimer) = reclaimer else {
                    self.trace_terminate(instance_id, || {
                        "err_teardown_failed_no_reclaimer".to_string()
                    });
                    return Err(match pending_receipt_error {
                        Some(receipt_error) => SwarmError::LedgerFailed(format!(
                            "{err}; cleanup_pending receipt also failed: {receipt_error}"
                        )),
                        None => err,
                    });
                };
                let trigger = match terminal {
                    ModelSessionState::Cancelled => ReclaimTrigger::OperatorCancel,
                    ModelSessionState::Failed => ReclaimTrigger::Failure,
                    _ => ReclaimTrigger::Close,
                };
                let report = tokio::time::timeout(
                    self.inner.config.teardown_timeout,
                    reclaimer.run_process(
                        &parent_session_id,
                        process_record_id.as_uuid(),
                        trigger,
                    ),
                )
                .await
                .map_err(|_| {
                    SwarmError::LedgerFailed(format!(
                        "session {instance_id} teardown failed ({teardown_error}); exact-process reclaim exceeded {:?}{pending_receipt_context}",
                        self.inner.config.teardown_timeout,
                    ))
                })?
                .map_err(|reclaim_error| {
                        SwarmError::LedgerFailed(format!(
                            "session {instance_id} teardown failed ({teardown_error}); exact-process reclaim failed: {reclaim_error}{pending_receipt_context}"
                        ))
                    })?;
                let reclaimed = report.processes_reclaimed.as_slice();
                let reclaimed_durably = matches!(
                    reclaimed,
                    [process]
                        if process.process_uuid == process_record_id.as_uuid()
                            && process.kill_result == KillOutcome::Killed
                            && process.stop_event_kind == Some(LedgerEventKind::Stop)
                );
                if !reclaimed_durably {
                    self.trace_terminate(instance_id, || "err_reclaim_not_durable".to_string());
                    return Err(SwarmError::LedgerFailed(format!(
                        "session {instance_id} teardown failed ({teardown_error}); exact-process reclaim did not prove kill plus durable STOP: {reclaimed:?}{pending_receipt_context}"
                    )));
                }
                self.trace_terminate(instance_id, || "reclaim_ok_durable".to_string());
                {
                    let mut registry = self.inner.registry.lock().expect("registry poisoned");
                    let handle = registry
                        .get_mut(&instance_id)
                        .filter(|handle| handle.spawn_generation == spawn_generation)
                        .ok_or(SwarmError::UnknownInstance(instance_id))?;
                    let cleanup = handle.cleanup.as_mut().expect("cleanup initialized");
                    cleanup.teardown_succeeded = true;
                    cleanup.stop_succeeded = true;
                }
                stop_succeeded = true;
                self.record_cleanup_receipt(
                    instance_id,
                    "exact_process_reclaim_succeeded",
                    terminal,
                    reason,
                    exit_code,
                    Some(&teardown_error),
                )
                .await?;
            } else {
                self.trace_terminate(instance_id, || "teardown_ok".to_string());
                {
                    let mut registry = self.inner.registry.lock().expect("registry poisoned");
                    let handle = registry
                        .get_mut(&instance_id)
                        .filter(|handle| handle.spawn_generation == spawn_generation)
                        .ok_or(SwarmError::UnknownInstance(instance_id))?;
                    handle
                        .cleanup
                        .as_mut()
                        .expect("cleanup initialized")
                        .teardown_succeeded = true;
                }
                self.record_cleanup_receipt(
                    instance_id,
                    "teardown_succeeded",
                    terminal,
                    reason,
                    exit_code,
                    None,
                )
                .await?;
            }
        } else {
            self.trace_terminate(instance_id, || {
                "teardown_skipped_already_succeeded".to_string()
            });
        }

        if !stop_succeeded {
            let stop_result = if let Some(lifecycle) = ledger_lifecycle {
                ensure_reserved_stop_recorded(lifecycle.stop(Some(exit_code), reason))
            } else {
                self.inner
                    .ledger
                    .record_stop_lossless(stop)
                    .map_err(|err| SwarmError::LedgerFailed(err.to_string()))
            };
            if let Err(err) = stop_result {
                let detail = err.to_string();
                self.record_cleanup_receipt(
                    instance_id,
                    "teardown_succeeded",
                    terminal,
                    reason,
                    exit_code,
                    Some(&detail),
                )
                .await?;
                return Err(SwarmError::LedgerFailed(detail));
            }
            let mut registry = self.inner.registry.lock().expect("registry poisoned");
            let handle = registry
                .get_mut(&instance_id)
                .filter(|handle| handle.spawn_generation == spawn_generation)
                .ok_or(SwarmError::UnknownInstance(instance_id))?;
            handle
                .cleanup
                .as_mut()
                .expect("cleanup initialized")
                .stop_succeeded = true;
        }

        // Runtime teardown and the process-ledger STOP are complete, but a
        // failed durable cleanup/lane write means terminalization is not. Keep
        // the handle fenced in Cancelling with its original cleanup intent so
        // coordinator-owned retry can re-run only the missing persistence and
        // then commit completion/eviction. Removing the handle here destroys
        // the sole in-memory retry state while the durable lane can still be
        // Ready.
        if let Some(err) = deferred_durable_error {
            let detail = err.to_string();
            if let Err(receipt_err) = self
                .record_cleanup_receipt(
                    instance_id,
                    "cleanup_pending",
                    terminal,
                    reason,
                    exit_code,
                    Some(&detail),
                )
                .await
            {
                self.trace_terminate(instance_id, || {
                    format!(
                        "retained_after_deferred_durable_err receipt_update_failed={receipt_err} original={detail}"
                    )
                });
                return Err(SwarmError::LedgerFailed(format!(
                    "{detail}; retaining cleanup-pending receipt also failed: {receipt_err}"
                )));
            }
            self.trace_terminate(instance_id, || {
                format!("retained_after_deferred_durable_err {detail}")
            });
            return Err(err);
        }

        let (terminal_event_id, resource_evicted_event_id) = {
            let registry = self.inner.registry.lock().expect("registry poisoned");
            let cleanup = registry
                .get(&instance_id)
                .filter(|handle| handle.spawn_generation == spawn_generation)
                .and_then(|handle| handle.cleanup.as_ref())
                .ok_or_else(|| {
                    SwarmError::LedgerFailed(format!(
                        "session {instance_id} lost cleanup event identity before terminal emission"
                    ))
                })?;
            (cleanup.terminal_event_id, cleanup.resource_evicted_event_id)
        };

        // Commit terminal + eviction events before removing the retryable
        // session handle. If the durable outbox rejects either event, the
        // producer observes EventSinkFailed and a later cleanup pass can retry.
        match terminal {
            ModelSessionState::Completed => self.emit_event(SwarmEvent::SessionCompleted {
                instance_id,
                event_id: Some(terminal_event_id),
            })?,
            ModelSessionState::Failed => self.emit_event(SwarmEvent::SessionFailed {
                instance_id,
                error: reason.to_string(),
                event_id: Some(terminal_event_id),
            })?,
            ModelSessionState::Cancelled => self.emit_event(SwarmEvent::SessionCancelled {
                instance_id,
                reason: reason.to_string(),
                event_id: Some(terminal_event_id),
            })?,
            _ => {}
        }
        self.emit_event(SwarmEvent::ResourceEvicted {
            instance_id,
            terminal_state: terminal,
            event_id: Some(resource_evicted_event_id),
        })?;

        // `completed` is the restart scan's exclusion boundary. Persist it only
        // after BOTH stable-ID terminal outbox rows have committed; otherwise a
        // crash or closed bridge between this receipt and event emission would
        // make restart reconciliation permanently skip missing telemetry.
        self.record_cleanup_receipt(instance_id, "completed", terminal, reason, exit_code, None)
            .await?;

        // Only terminalize and release capacity after teardown, STOP, and the
        // durable completed receipt and terminal outbox commits have succeeded.
        let handle = {
            let mut registry = self.inner.registry.lock().expect("registry poisoned");
            let Some(current) = registry.get(&instance_id) else {
                return Err(SwarmError::UnknownInstance(instance_id));
            };
            if current.spawn_generation != spawn_generation {
                return Err(SwarmError::UnknownInstance(instance_id));
            }
            registry
                .remove(&instance_id)
                .ok_or(SwarmError::UnknownInstance(instance_id))?
        };
        self.inner
            .release_committed_memory(handle.committed_memory_bytes);

        // Prune per-instance accounting now that the instance is terminal so the
        // respawn + signature maps cannot grow without bound (C5).
        {
            let mut acc = self.inner.accounting.lock().expect("accounting poisoned");
            acc.respawns.remove(&instance_id);
            acc.last_failure_signature.remove(&instance_id);
        }

        self.trace_terminate(instance_id, || "completed_ok".to_string());
        // The permit is released as `handle` drops at end of scope.
        Ok(())
    }

    async fn run_teardown_bounded(
        &self,
        instance_id: ModelInstanceId,
        teardown: SessionTeardown,
    ) -> SwarmResult<()> {
        let mut owned = tokio::spawn(async move { teardown().await });
        match tokio::time::timeout(self.inner.config.teardown_timeout, &mut owned).await {
            Ok(Ok(result)) => result,
            Ok(Err(join_error)) => Err(SwarmError::LedgerFailed(format!(
                "session {instance_id} owned teardown task failed to join: {join_error}"
            ))),
            Err(_) => {
                owned.abort();
                let _ = owned.await;
                Err(SwarmError::LedgerFailed(format!(
                    "session {instance_id} teardown exceeded {:?}; owned task was aborted and joined, cleanup remains retryable",
                    self.inner.config.teardown_timeout
                )))
            }
        }
    }

    async fn record_cleanup_receipt(
        &self,
        instance_id: ModelInstanceId,
        status: &str,
        terminal: ModelSessionState,
        reason: &str,
        exit_code: i32,
        last_error: Option<&str>,
    ) -> SwarmResult<()> {
        if let Some(store) = self.inner.model_lane_store.as_ref() {
            let (lane_id, process_uuid, terminal_event_id, resource_evicted_event_id) = {
                let registry = self.inner.registry.lock().expect("registry poisoned");
                let handle = registry
                    .get(&instance_id)
                    .ok_or(SwarmError::UnknownInstance(instance_id))?;
                let cleanup = handle.cleanup.as_ref().ok_or_else(|| {
                    SwarmError::LedgerFailed(format!(
                        "session {instance_id} cleanup receipt requested without cleanup state"
                    ))
                })?;
                (
                    handle.dexterity_lane_id.clone(),
                    handle.process_record_id.as_uuid(),
                    cleanup.terminal_event_id,
                    cleanup.resource_evicted_event_id,
                )
            };
            tokio::time::timeout(
                self.inner.config.teardown_timeout,
                store.record_session_cleanup_receipt(
                    &instance_id.to_string(),
                    lane_id.as_deref(),
                    process_uuid,
                    terminal_event_id,
                    resource_evicted_event_id,
                    status,
                    &format!("{terminal:?}"),
                    reason,
                    exit_code,
                    last_error,
                ),
            )
            .await
            .map_err(|_| {
                SwarmError::LedgerFailed(format!(
                    "session cleanup receipt persistence exceeded {:?}",
                    self.inner.config.teardown_timeout
                ))
            })?
            .map_err(|err| {
                SwarmError::LedgerFailed(format!(
                    "session cleanup receipt persistence failed: {err}"
                ))
            })?;
        }
        Ok(())
    }

    pub async fn retry_pending_orphan_cleanups(&self) -> SwarmResult<()> {
        let pending: Vec<_> = self
            .inner
            .orphan_cleanups
            .lock()
            .expect("orphan cleanups poisoned")
            .keys()
            .copied()
            .collect();
        let mut errors = Vec::new();
        for process_record_id in pending {
            if let Err(error) = self.retry_orphan_cleanup(process_record_id).await {
                let still_pending = self
                    .inner
                    .orphan_cleanups
                    .lock()
                    .expect("orphan cleanups poisoned")
                    .contains_key(&process_record_id);
                if still_pending {
                    errors.push(format!("{}: {error}", process_record_id.as_uuid()));
                }
            }
        }
        if !errors.is_empty() {
            return Err(SwarmError::LedgerFailed(format!(
                "orphan cleanup retry batch left failures: {}",
                errors.join(" | ")
            )));
        }
        Ok(())
    }

    pub async fn retry_pending_session_cleanups(&self) -> SwarmResult<()> {
        let mut errors = Vec::new();
        if let Err(error) = self.retry_pending_orphan_cleanups().await {
            errors.push(format!("orphan cleanup class: {error}"));
        }
        let pending: Vec<_> = self
            .inner
            .registry
            .lock()
            .expect("registry poisoned")
            .iter()
            .filter_map(|(instance_id, handle)| {
                handle.cleanup.as_ref().map(|cleanup| {
                    (
                        *instance_id,
                        handle.spawn_generation,
                        cleanup.terminal,
                        cleanup.reason.clone(),
                        cleanup.exit_code,
                    )
                })
            })
            .collect();

        #[cfg(any(test, feature = "test-utils"))]
        let cleanup_retry_pause = {
            self.inner
                .cleanup_retry_after_snapshot_pause
                .lock()
                .expect("cleanup retry snapshot pause poisoned")
                .take()
        };
        #[cfg(any(test, feature = "test-utils"))]
        if let Some((arrived, release)) = cleanup_retry_pause {
            arrived.notify_one();
            release.notified().await;
        }

        for (instance_id, spawn_generation, terminal, reason, exit_code) in pending {
            match self
                .terminate_with_contention(
                    instance_id,
                    terminal,
                    &reason,
                    exit_code,
                    Some(spawn_generation),
                    CleanupContentionPolicy::SkipIfOwned,
                )
                .await
            {
                Ok(()) | Err(SwarmError::UnknownInstance(_)) => {}
                Err(error) => errors.push(format!("{instance_id}: {error}")),
            }
        }
        if !errors.is_empty() {
            return Err(SwarmError::LedgerFailed(format!(
                "session cleanup retry batch left failures: {}",
                errors.join(" | ")
            )));
        }
        Ok(())
    }

    /// Finish durable terminal intent after process restart. Product boot runs
    /// process-ledger reclaim first; this pass independently verifies the exact
    /// process has a durable STOP before repairing the ModelLane terminal row,
    /// stable terminal outbox events, and cleanup receipt.
    pub async fn reconcile_durable_cleanup_receipts_after_boot(&self) -> SwarmResult<()> {
        let Some(store) = self.inner.model_lane_store.as_ref() else {
            return Ok(());
        };
        let receipts = store
            .pending_session_cleanup_receipts()
            .await
            .map_err(|error| {
                SwarmError::LedgerFailed(format!("durable cleanup restart scan failed: {error}"))
            })?;
        let mut errors = Vec::new();
        for receipt in receipts {
            let process_closed = match store
                .cleanup_process_is_durably_closed(receipt.process_uuid)
                .await
            {
                Ok(closed) => closed,
                Err(error) => {
                    errors.push(format!(
                        "{} process closure verification failed: {error}",
                        receipt.instance_id
                    ));
                    continue;
                }
            };
            if !process_closed {
                errors.push(format!(
                    "{} remains cleanup_pending because process {} has no durable STOP (prior status={}, prior error={:?})",
                    receipt.instance_id, receipt.process_uuid, receipt.status, receipt.last_error
                ));
                continue;
            }
            let terminal = match receipt.terminal_state.as_str() {
                "Completed" => ModelSessionState::Completed,
                "Failed" => ModelSessionState::Failed,
                "Cancelled" => ModelSessionState::Cancelled,
                other => {
                    errors.push(format!(
                        "{} has unsupported durable terminal state {other}",
                        receipt.instance_id
                    ));
                    continue;
                }
            };
            let Some(lane_id) = receipt.lane_id.as_deref() else {
                errors.push(format!(
                    "{} cleanup receipt has no durable lane identity",
                    receipt.instance_id
                ));
                continue;
            };
            let Some(lane_status) = model_lane_terminal_status(terminal) else {
                unreachable!("durable terminal state validated above")
            };
            if let Err(error) = tokio::time::timeout(
                self.inner.config.teardown_timeout,
                store.record_lane_terminal_status(lane_id, lane_status, &receipt.reason),
            )
            .await
            .map_err(|_| {
                SwarmError::LedgerFailed(format!(
                    "{} restart lane terminalization exceeded {:?}",
                    receipt.instance_id, self.inner.config.teardown_timeout
                ))
            })
            .and_then(|result| {
                result.map_err(|error| {
                    SwarmError::LedgerFailed(format!(
                        "{} restart lane terminalization failed: {error}",
                        receipt.instance_id
                    ))
                })
            }) {
                errors.push(error.to_string());
                continue;
            }
            let instance_id = match parse_routing_model_instance_id(&receipt.instance_id) {
                Ok(instance_id) => instance_id,
                Err(error) => {
                    errors.push(error.to_string());
                    continue;
                }
            };
            let terminal_event = match terminal {
                ModelSessionState::Completed => SwarmEvent::SessionCompleted {
                    instance_id,
                    event_id: Some(receipt.terminal_event_id),
                },
                ModelSessionState::Failed => SwarmEvent::SessionFailed {
                    instance_id,
                    error: receipt.reason.clone(),
                    event_id: Some(receipt.terminal_event_id),
                },
                ModelSessionState::Cancelled => SwarmEvent::SessionCancelled {
                    instance_id,
                    reason: receipt.reason.clone(),
                    event_id: Some(receipt.terminal_event_id),
                },
                _ => unreachable!("durable terminal state validated above"),
            };
            if let Err(error) = self.emit_event(terminal_event).and_then(|_| {
                self.emit_event(SwarmEvent::ResourceEvicted {
                    instance_id,
                    terminal_state: terminal,
                    event_id: Some(receipt.resource_evicted_event_id),
                })
            }) {
                errors.push(format!(
                    "{} restart terminal event persistence failed: {error}",
                    receipt.instance_id
                ));
                continue;
            }
            if let Err(error) = store
                .record_session_cleanup_receipt(
                    &receipt.instance_id,
                    Some(lane_id),
                    receipt.process_uuid,
                    receipt.terminal_event_id,
                    receipt.resource_evicted_event_id,
                    "completed",
                    &receipt.terminal_state,
                    &receipt.reason,
                    receipt.exit_code,
                    None,
                )
                .await
            {
                errors.push(format!(
                    "{} restart cleanup completion receipt failed: {error}",
                    receipt.instance_id
                ));
            }
        }
        if !errors.is_empty() {
            return Err(SwarmError::LedgerFailed(format!(
                "durable cleanup restart reconciliation left failures: {}",
                errors.join(" | ")
            )));
        }
        Ok(())
    }

    /// Build a process-ledger STOP row for a terminating session. Shared by
    /// `terminate`, `teardown_orphan`, and the reaper so START/STOP rows are
    /// symmetric in one place (C7).
    ///
    /// START/STOP symmetry is an authority requirement, not a convention: the
    /// authoritative STOP upsert only closes a lifecycle row when every
    /// immutable identity column of the STOP matches the persisted START. When
    /// the session factory recorded the START, the coordinator therefore uses
    /// that exact record — supplied by the factory as `ledger_start_override`,
    /// or otherwise recovered from the ledger's in-flight START index. Falling
    /// back to coordinator defaults here would emit a STOP whose `started_at`,
    /// WP/MT lineage, and `metadata_jsonb` diverge from the START, which
    /// canonical ProcessLedger authority rejects as `PROCESS_LEDGER_STOP_IDENTITY_CONFLICT` and which
    /// leaves the process permanently open in the ledger. The synthesized row
    /// below is reserved for sessions whose START was never recorded, where the
    /// STOP inserts a fresh lifecycle row instead of updating one.
    #[allow(clippy::too_many_arguments)]
    fn build_stop(
        &self,
        process_record_id: ProcessOwnershipRecordId,
        ledger_os_pid: Option<u32>,
        ledger_start_override: Option<ProcessStart>,
        engine_kind: ProcessEngineKind,
        parent_session_id: Option<String>,
        started_at: DateTime<Utc>,
        terminal: ModelSessionState,
        reason: &str,
        exit_code: i32,
        instance_id: &ModelInstanceId,
    ) -> ProcessStop {
        let recorded_start = ledger_start_override.or_else(|| {
            self.inner
                .ledger
                .recorded_start(process_record_id.as_uuid())
        });
        if let Some(start) = recorded_start {
            return ProcessStop::from_start(&start, Some(exit_code)).with_stop_reason(reason);
        }
        ProcessStop {
            process_uuid: process_record_id.as_uuid(),
            os_pid: ledger_os_pid,
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
            runtime_owner: None,
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

    fn emit_spawn_rejected(&self, instance_id: ModelInstanceId, reason: &str) -> SwarmResult<()> {
        self.emit_event(SwarmEvent::SpawnRejected {
            instance_id,
            reason: reason.to_string(),
        })
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
    /// Drain the requested concurrency decrease as far as free permits allow,
    /// and report the cap now genuinely in force.
    ///
    /// Idempotent and cheap, so it is safe to call on every admission and every
    /// read. It MUST be called before admission: a session that ended after a
    /// lowering request handed its permit back to the semaphore, and without
    /// this the next spawn would be admitted on that permit — over the cap the
    /// operator asked for and above the number the UI is showing.
    fn reconcile_concurrency_cap(&self) -> usize {
        loop {
            let effective = self.effective_max_concurrent.load(Ordering::SeqCst);
            let desired = self.desired_max_concurrent.load(Ordering::SeqCst);
            if desired >= effective {
                return effective;
            }
            // Only permits that are free right now can be taken; the rest stay
            // with their running sessions and are retired on a later pass.
            let removed = self.semaphore.forget_permits(effective - desired);
            if removed == 0 {
                // Nothing was free. Re-load rather than returning the value read
                // at the top: a concurrent reconcile may have committed a lower
                // cap in between, and reporting the pre-read number would hand a
                // caller a cap the semaphore is no longer honouring.
                return self.effective_max_concurrent.load(Ordering::SeqCst);
            }
            let in_force = effective - removed;
            if self
                .effective_max_concurrent
                .compare_exchange(effective, in_force, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                return in_force;
            }
            // Lost a race with a concurrent raise; give the permits back and
            // re-read rather than leaving the semaphore short.
            self.semaphore.add_permits(removed);
        }
    }

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
    let expired: Vec<(ModelInstanceId, u64, String, ModelSessionState, String, i32)> = {
        let registry = inner.registry.lock().expect("registry poisoned");
        registry
            .values()
            .filter(|h| !h.state.is_terminal() && h.lease.is_expired(now))
            .map(|h| {
                let (terminal, reason, exit_code) = h
                    .cleanup
                    .as_ref()
                    .map(|cleanup| (cleanup.terminal, cleanup.reason.clone(), cleanup.exit_code))
                    .unwrap_or((
                        ModelSessionState::Cancelled,
                        "lease_expired_reclaim".to_string(),
                        -1,
                    ));
                (
                    h.instance_id,
                    h.spawn_generation,
                    h.lease.owner.clone(),
                    terminal,
                    reason,
                    exit_code,
                )
            })
            .collect()
    };

    #[cfg(any(test, feature = "test-utils"))]
    let reaper_pause = {
        inner
            .reaper_after_snapshot_pause
            .lock()
            .expect("reaper snapshot pause poisoned")
            .take()
    };
    #[cfg(any(test, feature = "test-utils"))]
    if let Some((arrived, release)) = reaper_pause {
        arrived.notify_one();
        release.notified().await;
    }

    for (instance_id, spawn_generation, owner, terminal, reason, exit_code) in expired {
        let _ = inner
            .sink
            .emit(SwarmEvent::LeaseExpired { instance_id, owner });

        // Lease expiry uses the same durable cleanup state machine as explicit
        // completion/cancellation. A teardown or STOP failure therefore keeps
        // the session fenced and retryable for the next reaper pass.
        let coordinator = SwarmCoordinator {
            inner: Arc::clone(inner),
            reaper: Arc::new(ReaperHandle::new()),
        };
        if let Err(err) = coordinator
            .terminate_generation(instance_id, spawn_generation, terminal, &reason, exit_code)
            .await
        {
            if let Err(event_error) = inner.sink.emit(SwarmEvent::SessionFailed {
                instance_id,
                error: format!("lease reaper cleanup remains pending: {err}"),
                event_id: None,
            }) {
                tracing::error!(
                    target: "handshake_core::swarm_orchestration",
                    %instance_id,
                    %event_error,
                    "lease reaper terminal failure event rejected; cleanup remains retryable"
                );
            }
        }
    }

    // C5: prune fully-settled breaker signatures (Closed-no-failures or
    // cooled-down Open) so the signature map cannot grow without bound across a
    // long run of heterogeneous failures.
    {
        let mut breaker = inner.breaker.lock().expect("breaker poisoned");
        breaker.prune_settled(std::time::Instant::now());
    }
}

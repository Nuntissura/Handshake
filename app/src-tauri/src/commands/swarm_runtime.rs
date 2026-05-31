//! MT-204/MT-205 app-side wiring for the multi-model SWARM coordinator
//! (Master Spec §4.3.9 MULTI_MODEL_PARALLEL).
//!
//! This module constructs and `.manage`s a production [`SwarmCoordinator`] and
//! exposes the OPERATOR SURFACE Tauri commands (MT-205) that let the operator:
//!
//! - spawn a local model session from a real on-disk artifact (candle / llama)
//!   OR a cloud model (returns a typed `ProviderNotConfigured` until BYOK creds
//!   are wired — a genuine runtime condition, never a placeholder),
//! - cancel a session,
//! - list the live sessions (model id, instance, lifecycle state, provider),
//! - snapshot the resource budget (concurrency cap / used / remaining + the
//!   monotonic lifetime ceiling), and
//! - run a REAL generate against a spawned local-model session and stream the
//!   produced tokens back (the operator <-> local-model chat box).
//!
//! ## Why a session side-table lives here
//!
//! The `SwarmCoordinator` (in `handshake_core`, off-limits to this layer) is the
//! multi-instance authority: it owns the session registry, the bounds, the
//! lease/TTL reaper, and the ledger. It deliberately exposes only
//! `spawn_session` / `cancel_session` / `session_state` / `live_session_count` /
//! `remaining` — there is NO public accessor that hands an
//! `Arc<dyn ModelRuntime>` (or the runtime-minted `ModelId`) back out of a live
//! session. The operator chat box needs exactly that to run a real generate.
//!
//! Rather than double-loading the model, the app wraps the production
//! [`ProductionModelSessionFactory`] in a [`TrackingFactory`]. The coordinator
//! still owns the canonical session; the wrapper, on the SAME successful load,
//! clones the session's runtime `Arc` + `ModelId` + descriptor into an app-side
//! side-table keyed by `ModelInstanceId`. There is exactly ONE load and ONE
//! engine: the `Arc<dyn ModelRuntime>` in the side-table is the very handle the
//! coordinator registered (the factory's `SharedRuntimeHandle`), so a generate
//! through it drives the real, live session. The side-table is pruned lazily —
//! every list/generate/cancel reconciles it against
//! `coordinator.session_state(iid)`, dropping entries the coordinator has
//! already evicted (terminal / reaped) so the table cannot leak.
//!
//! The cloud lane is wired (MT-206) to the SAME `OsKeychainSecretsVault` the
//! cloud-lane control panel writes BYOK keys into, via
//! `CloudLaneFactoryConfig::from_vault`. A cloud swarm spawn then constructs a
//! REAL anthropic/openai BYOK `ModelRuntime` when the operator has stored a key
//! under the configured lane id, and returns a typed `ProviderNotConfigured`
//! (genuine runtime condition) when no key is present — never a placeholder.
//! Local and cloud sessions therefore run in parallel through one coordinator:
//! local candle/llama sessions are real on-host engines; cloud sessions are the
//! real provider HTTP/SSE path when credentialed.
//!
//! ## Cloud-lane vault wiring
//!
//! `production()` builds an `OsKeychainSecretsVault` under
//! `HANDSHAKE_KEYCHAIN_SERVICE` (the same namespace `lib.rs` builds the
//! cloud-lane panel's vault with) and binds the anthropic lane id from
//! `HANDSHAKE_SWARM_ANTHROPIC_LANE` (default `"anthropic"`) and the openai lane
//! id from `HANDSHAKE_SWARM_OPENAI_LANE` (default `"openai"`). Store a key under
//! that lane id via the cloud-lane control panel to enable the lane. Use
//! `production_with_cloud_vault` to inject a specific vault + lane ids (tests,
//! or a shared vault handle).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use futures_util::StreamExt;
use handshake_core::model_runtime::registry::RuntimeBinding;
use handshake_core::model_runtime::{
    CancellationToken, GenPrompt, GenerateRequest, ModelId, ModelRuntime, ProviderKind,
    SamplingParams,
};
use handshake_core::process_ledger::{
    LedgerBatcher, LedgerBatcherConfig, LedgerEvent, LedgerOverflowEvent, ProcessLedgerError,
    ProcessLedgerOverflowSink, ProcessLedgerStore,
};
use handshake_core::model_runtime::cloud::SecretsVault;
use handshake_core::flight_recorder::FlightRecorder;
use handshake_core::swarm_orchestration::{
    BroadcastSwarmSink, CloudLaneFactoryConfig, DurableSwarmFrBridge, FanoutSwarmSink,
    FlightRecorderSwarmSink, LiveSession, ModelInstanceId, ModelSessionFactory, ModelSessionState,
    ProductionModelSessionFactory, RunBudget, SpawnRequest, SwarmConfig, SwarmCoordinator,
    SwarmError, SwarmEvent, SwarmEventSink, SwarmResult,
};
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

/// FR-EVT tag stamped on swarm lifecycle events forwarded to structured stderr
/// until the durable flight-recorder DB sink is wired by a swarm command (MT-205).
pub const FR_EVT_SWARM_LIFECYCLE: &str = "FR-EVT-SWARM-LIFECYCLE";

pub const KERNEL_SWARM_SPAWN_SESSION_IPC_CHANNEL: &str = "kernel_swarm_spawn_session";
pub const KERNEL_SWARM_CANCEL_SESSION_IPC_CHANNEL: &str = "kernel_swarm_cancel_session";
pub const KERNEL_SWARM_LIST_ACTIVE_SESSIONS_IPC_CHANNEL: &str =
    "kernel_swarm_list_active_sessions";
pub const KERNEL_SWARM_RESOURCE_SNAPSHOT_IPC_CHANNEL: &str = "kernel_swarm_resource_snapshot";
pub const KERNEL_SWARM_CHAT_GENERATE_IPC_CHANNEL: &str = "kernel_swarm_chat_generate";

/// Max tokens a single chat-box generate will produce. Bounded so an operator
/// chat turn cannot run away (HBR loop-cap spirit) — the UI runs short turns.
const CHAT_MAX_TOKENS: u32 = 256;

/// In-process [`ProcessLedgerStore`] the app owns for the swarm coordinator. Not
/// a fake: it durably accumulates every START/STOP row for the running session
/// so the orchestration ledger is real and inspectable. A Postgres-backed store
/// replaces this when the app gains a configured pool (out of MT-204 scope).
#[derive(Clone, Default)]
pub struct InProcessLedgerStore {
    events: Arc<Mutex<Vec<LedgerEvent>>>,
}

impl InProcessLedgerStore {
    pub fn rows(&self) -> Vec<LedgerEvent> {
        self.events.lock().expect("swarm ledger store poisoned").clone()
    }
}

#[async_trait]
impl ProcessLedgerStore for InProcessLedgerStore {
    async fn write_batch(&self, events: Vec<LedgerEvent>) -> Result<(), ProcessLedgerError> {
        self.events
            .lock()
            .map_err(|_| ProcessLedgerError::Store("swarm ledger store poisoned".to_string()))?
            .extend(events);
        Ok(())
    }
}

/// Overflow sink for the swarm ledger: counts dropped rows so an overflow is
/// observable rather than silent.
#[derive(Clone, Default)]
struct CountingOverflowSink {
    last_overflow: Arc<Mutex<u64>>,
}

impl ProcessLedgerOverflowSink for CountingOverflowSink {
    fn emit_overflow(&self, event: LedgerOverflowEvent) -> Result<(), ProcessLedgerError> {
        if let Ok(mut last) = self.last_overflow.lock() {
            *last = event.overflow_count;
        }
        eprintln!(
            "{FR_EVT_SWARM_LIFECYCLE}: ledger overflow count={}",
            event.overflow_count
        );
        Ok(())
    }
}

/// A descriptor + live handle for a session the app tracks alongside the
/// coordinator's canonical registry. The `runtime` is the SAME `Arc` the
/// coordinator registered (cloned, not re-loaded), so a generate through it
/// drives the real live session.
#[derive(Clone)]
struct TrackedSession {
    model_id: ModelId,
    runtime: Arc<dyn ModelRuntime>,
    runtime_binding: RuntimeBinding,
    provider: ProviderKind,
    /// For local sessions, the artifact path the operator spawned from (for the
    /// control-room display). `None` for cloud sessions.
    artifact_path: Option<String>,
    /// For cloud sessions, the allowlisted cloud model name. `None` for local.
    cloud_model_name: Option<String>,
}

/// Shared app-side side-table of tracked sessions keyed by instance id.
type SessionTable = Arc<Mutex<HashMap<ModelInstanceId, TrackedSession>>>;

/// Factory wrapper that records every successfully-created session's runtime +
/// model id into the app-side side-table while delegating the real load to the
/// inner production factory. This is the seam that lets the operator chat box
/// reach the live runtime WITHOUT a second load and WITHOUT touching the
/// off-limits coordinator internals.
struct TrackingFactory {
    inner: ProductionModelSessionFactory,
    table: SessionTable,
}

#[async_trait]
impl ModelSessionFactory for TrackingFactory {
    async fn create(&self, request: &SpawnRequest) -> SwarmResult<LiveSession> {
        let live = self.inner.create(request).await?;
        // Clone the SAME runtime Arc + model id the coordinator will own. One
        // load, one engine; the side-table just holds a second strong reference
        // for the chat/generate path. Pruned lazily against session_state.
        let tracked = TrackedSession {
            model_id: live.model_id,
            runtime: live.runtime.clone(),
            runtime_binding: request.runtime_binding,
            provider: request.provider.unwrap_or(ProviderKind::Local),
            artifact_path: request.model_artifact_path().map(|s| s.to_string()),
            cloud_model_name: request.cloud_model_name.clone(),
        };
        self.table
            .lock()
            .expect("swarm session table poisoned")
            .insert(request.instance_id, tracked);
        Ok(live)
    }
}

/// Managed app state holding the production swarm coordinator + its ledger
/// store, the app-side session side-table, plus the writer-task join handle so
/// it is not dropped (which would stop the background ledger writer).
pub struct SwarmRuntimeState {
    coordinator: Arc<SwarmCoordinator>,
    ledger_store: InProcessLedgerStore,
    sessions: SessionTable,
    /// The concurrency cap the coordinator's semaphore was built with. The
    /// coordinator does not expose its `max_concurrent` publicly, so the app
    /// records the value it constructed the budget with for the resource bar.
    concurrency_cap: usize,
    _writer_task: tokio::task::JoinHandle<Result<(), ProcessLedgerError>>,
    /// rank-4: the live board broadcast source. The coordinator's sink fans out to
    /// this (alongside the FR sink); the Tauri board forwarder subscribes here and
    /// re-emits typed deltas to the React board, replacing the 1500ms poll.
    board_events: Arc<BroadcastSwarmSink>,
    /// rank-3: the durable FR bridge drain task (when a recorder is wired). Held
    /// so it lives for the app's lifetime; dropping it stops persisting swarm
    /// events. `None` when no recorder was supplied (the eprintln fallback).
    _fr_drain_task: Option<tokio::task::JoinHandle<()>>,
    /// Integrated Terminal capture seam (spec §10.1). When wired (by lib.rs from
    /// the managed `TerminalRuntimeState`), a swarm spawn ALSO opens a read-only
    /// AiJob capture session bound to the swarm_id swimlane, so the operator can
    /// inspect that swarm's background work in the Terminal panel and the spawn
    /// lifecycle is trace-linked into the Flight Recorder. `None` keeps the
    /// existing behavior unchanged (capture is opt-in, never faked).
    terminal_capture: Option<handshake_core::terminal::TerminalRuntime>,
    /// Live capture sinks keyed by swarm instance_id. When the capture seam is
    /// wired, `attach_spawn_capture` opens a read-only AiJob capture session per
    /// spawn and stores its [`CaptureSink`] here; `kernel_swarm_chat_generate`
    /// then feeds the REAL generated token stream into the matching sink, so the
    /// "inspect all background work" panel carries a live stream of the model's
    /// actual output (not a synthetic marker). Closed + removed on session
    /// cancel / reconcile so the sink's leak guard never has to fire for the
    /// normal path.
    capture_sinks: Arc<Mutex<HashMap<String, Arc<handshake_core::terminal::CaptureSink>>>>,
}

impl std::fmt::Debug for SwarmRuntimeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SwarmRuntimeState")
            .field("coordinator", &"<SwarmCoordinator>")
            .field("ledger_store", &"<InProcessLedgerStore>")
            .field("sessions", &"<SessionTable>")
            .finish()
    }
}

impl SwarmRuntimeState {
    /// Build the production swarm runtime with the cloud lane wired to the
    /// host OS keychain vault (the same `OsKeychainSecretsVault` the cloud-lane
    /// control panel uses). A cloud spawn constructs a real BYOK runtime when
    /// the operator has stored a key under the configured lane id, and returns a
    /// typed `ProviderNotConfigured` otherwise. Lane ids default to
    /// `"anthropic"` / `"openai"` and can be overridden via
    /// `HANDSHAKE_SWARM_ANTHROPIC_LANE` / `HANDSHAKE_SWARM_OPENAI_LANE`.
    pub fn production() -> Self {
        let vault: Arc<dyn SecretsVault> = Arc::new(
            handshake_core::model_runtime::cloud::secrets_vault::OsKeychainSecretsVault::new(
                handshake_core::model_runtime::cloud::secrets_vault::HANDSHAKE_KEYCHAIN_SERVICE,
            ),
        );
        let anthropic_lane = std::env::var("HANDSHAKE_SWARM_ANTHROPIC_LANE")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "anthropic".to_string());
        let openai_lane = std::env::var("HANDSHAKE_SWARM_OPENAI_LANE")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "openai".to_string());
        let cloud = CloudLaneFactoryConfig::from_vault(
            vault,
            Some(anthropic_lane),
            Some(openai_lane),
        );
        Self::production_with_cloud(cloud)
    }

    /// Build the production swarm runtime with an explicit cloud-lane config
    /// built from a caller-supplied vault + lane ids. This is the seam tests
    /// (and alternate wirings) use to inject an `InMemorySecretsVault` so the
    /// cloud dispatch path is provable without touching the OS keychain.
    pub fn production_with_cloud_vault(
        vault: Arc<dyn SecretsVault>,
        anthropic_lane: Option<String>,
        openai_lane: Option<String>,
    ) -> Self {
        let cloud = CloudLaneFactoryConfig::from_vault(vault, anthropic_lane, openai_lane);
        Self::production_with_cloud(cloud)
    }

    /// Core constructor: real ledger batcher + in-process store, the production
    /// factory wrapped in a [`TrackingFactory`], a structured flight-recorder
    /// sink, with the supplied [`CloudLaneFactoryConfig`]. Starts the lease/TTL
    /// reaper so stale sessions are reclaimed for the app's lifetime.
    pub fn production_with_cloud(cloud: CloudLaneFactoryConfig) -> Self {
        Self::production_with_cloud_and_recorder(cloud, None)
    }

    /// rank-3: production swarm runtime that PERSISTS every SwarmEvent into
    /// `recorder` (the deployed app passes the app-data-root DuckDB Flight
    /// Recorder) via the DurableSwarmFrBridge, so swarm/VM lifecycle events are
    /// durable + replayable for the board drill-down + audit -- not just stderr.
    /// Mirrors `production()`'s cloud-lane wiring.
    pub fn production_with_fr_recorder(recorder: Arc<dyn FlightRecorder>) -> Self {
        let vault: Arc<dyn SecretsVault> = Arc::new(
            handshake_core::model_runtime::cloud::secrets_vault::OsKeychainSecretsVault::new(
                handshake_core::model_runtime::cloud::secrets_vault::HANDSHAKE_KEYCHAIN_SERVICE,
            ),
        );
        let anthropic_lane = std::env::var("HANDSHAKE_SWARM_ANTHROPIC_LANE")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "anthropic".to_string());
        let openai_lane = std::env::var("HANDSHAKE_SWARM_OPENAI_LANE")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "openai".to_string());
        let cloud =
            CloudLaneFactoryConfig::from_vault(vault, Some(anthropic_lane), Some(openai_lane));
        Self::production_with_cloud_and_recorder(cloud, Some(recorder))
    }

    fn production_with_cloud_and_recorder(
        cloud: CloudLaneFactoryConfig,
        recorder: Option<Arc<dyn FlightRecorder>>,
    ) -> Self {
        let store = InProcessLedgerStore::default();
        let overflow = Arc::new(CountingOverflowSink::default());
        let (ledger, writer_task) = LedgerBatcher::spawn(
            Arc::new(store.clone()),
            overflow,
            LedgerBatcherConfig::default(),
        );

        let sessions: SessionTable = Arc::new(Mutex::new(HashMap::new()));

        // Build the coordinator manually (rather than via
        // `build_production_swarm_coordinator`) so the production factory can be
        // wrapped with the TrackingFactory that feeds the app-side side-table.
        // Concurrency cap from CPU parallelism; the lifetime ceiling is
        // HBR-SWARM-002's loop cap (RunBudget default). The cloud lane is wired
        // from the supplied `CloudLaneFactoryConfig` (MT-206): a cloud spawn
        // builds a real BYOK runtime when the operator has stored a key, and
        // returns a typed ProviderNotConfigured otherwise.
        let concurrency =
            handshake_core::swarm_orchestration::default_swarm_concurrency().max(1);
        let budget = RunBudget::defaulted(concurrency).with_concurrency(concurrency);
        let config = SwarmConfig::new(budget);

        let production_factory =
            ProductionModelSessionFactory::new(ledger.clone(), cloud);
        let factory = Arc::new(TrackingFactory {
            inner: production_factory,
            table: sessions.clone(),
        });

        let trace_id = Uuid::now_v7();
        // rank-3: when a recorder is supplied, persist every SwarmEvent durably
        // via the bridge (sync sink emit -> bounded channel -> async record_event);
        // otherwise fall back to structured stderr. Hold the bridge drain task.
        let (fr_sink, fr_drain): (Arc<dyn SwarmEventSink>, Option<tokio::task::JoinHandle<()>>) =
            match recorder {
                Some(rec) => {
                    let (bridge, drain) = DurableSwarmFrBridge::spawn(rec, 1024);
                    let sink: Arc<dyn SwarmEventSink> =
                        Arc::new(FlightRecorderSwarmSink::new(trace_id, move |fr_event| {
                            bridge.emit(fr_event);
                        }));
                    (sink, Some(drain))
                }
                None => {
                    let sink: Arc<dyn SwarmEventSink> =
                        Arc::new(FlightRecorderSwarmSink::new(trace_id, |event| {
                            eprintln!(
                                "{FR_EVT_SWARM_LIFECYCLE}: {}",
                                serde_json::to_string(&event.payload).unwrap_or_default()
                            );
                        }));
                    (sink, None)
                }
            };
        // rank-4: fan out to BOTH the FR sink and the live board broadcast, so one
        // coordinator drives durable capture + the live operator board.
        let board_events = Arc::new(BroadcastSwarmSink::new(1024));
        let sink: Arc<dyn SwarmEventSink> = Arc::new(FanoutSwarmSink::new(vec![
            fr_sink,
            board_events.clone() as Arc<dyn SwarmEventSink>,
        ]));

        let coordinator = SwarmCoordinator::new(config, factory, sink, ledger);
        coordinator.start_reaper();

        Self {
            coordinator: Arc::new(coordinator),
            ledger_store: store,
            sessions,
            concurrency_cap: concurrency,
            _writer_task: writer_task,
            board_events,
            _fr_drain_task: fr_drain,
            terminal_capture: None,
            capture_sinks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Wire the Integrated Terminal capture seam so each swarm spawn opens a
    /// read-only capture session bound to the swarm_id swimlane. Called by
    /// lib.rs with the managed `TerminalRuntimeState`'s runtime handle. Builder
    /// style so existing constructors stay unchanged.
    pub fn with_terminal_capture(
        mut self,
        runtime: handshake_core::terminal::TerminalRuntime,
    ) -> Self {
        self.terminal_capture = Some(runtime);
        self
    }

    /// The capture-seam runtime handle, if wired.
    pub fn terminal_capture(&self) -> Option<handshake_core::terminal::TerminalRuntime> {
        self.terminal_capture.clone()
    }

    /// Open a read-only capture session bound to a swarm spawn (the first real
    /// swarm producer of the §10.1 capture seam) and RETAIN its [`CaptureSink`]
    /// keyed by instance_id, so the live generated-token producer
    /// (`kernel_swarm_chat_generate`) can fan the model's REAL output into it.
    /// Returns the capture session id when the seam is wired, else `None`. The
    /// session is tagged with the swarm_id / instance_id swimlane so the board +
    /// Terminal panel correlate.
    ///
    /// The seeded line is an explicit lifecycle banner (not faked model output):
    /// the live model output is appended to this SAME session by the generate
    /// path as tokens are produced.
    pub async fn attach_spawn_capture(
        &self,
        swarm_id: Option<String>,
        instance_id: String,
    ) -> Option<String> {
        let runtime = self.terminal_capture.clone()?;
        let binding = handshake_core::terminal::SessionBinding {
            swarm_id,
            worktree_id: None,
            instance_id: Some(instance_id.clone()),
        };
        let (info, sink) = runtime
            .create_capture_session(binding, Some(format!("swarm:{instance_id}")))
            .await;
        // Lifecycle banner so the panel shows the session opened + when. This is
        // an honest banner, clearly bracketed, NOT a stand-in for model output —
        // the real producer (the generated token stream) appends below it.
        sink.feed(
            format!("[handshake] capture session opened for swarm instance {instance_id}\r\n")
                .as_bytes(),
        )
        .await;
        // Retain the sink so the generate path can feed the live token stream
        // into this same capture session.
        self.capture_sinks
            .lock()
            .expect("capture sink table poisoned")
            .insert(instance_id, Arc::new(sink));
        Some(info.session_id)
    }

    /// The retained capture sink for a swarm instance, if the capture seam is
    /// wired and a session was opened for it. Used by the generate path to fan
    /// the live token stream into the "inspect all background work" panel.
    pub fn capture_sink_for(
        &self,
        instance_id: &str,
    ) -> Option<Arc<handshake_core::terminal::CaptureSink>> {
        self.capture_sinks
            .lock()
            .expect("capture sink table poisoned")
            .get(instance_id)
            .cloned()
    }

    /// Close + remove the retained capture sink for an instance (on session
    /// cancel / reconcile). The sink's own leak guard would eventually reap an
    /// orphaned session, but closing here is the clean, FR-recorded path.
    pub async fn close_capture_sink(&self, instance_id: &str, exit_code: i32) {
        let sink = self
            .capture_sinks
            .lock()
            .expect("capture sink table poisoned")
            .remove(instance_id);
        if let Some(sink) = sink {
            // Only close if we hold the last reference; otherwise a concurrent
            // generate is still feeding and will be the one to drop it. We drop
            // our Arc; if it was the last, Drop's leak guard reaps it. To get a
            // clean FR close we try to unwrap to the owned sink.
            if let Ok(owned) = Arc::try_unwrap(sink) {
                owned.close(exit_code).await;
            }
        }
    }

    /// The concurrency cap the coordinator's semaphore was built with.
    pub fn concurrency_cap(&self) -> usize {
        self.concurrency_cap
    }

    /// The production coordinator. Swarm commands call `spawn_session` /
    /// `cancel_session` / `session_state` / `remaining` on this.
    pub fn coordinator(&self) -> Arc<SwarmCoordinator> {
        self.coordinator.clone()
    }

    /// rank-4: the live board broadcast source. The Tauri board forwarder
    /// subscribes here (`board_events().subscribe()`) and re-emits typed deltas.
    pub fn board_events(&self) -> Arc<BroadcastSwarmSink> {
        self.board_events.clone()
    }

    /// The durable swarm ledger rows recorded so far (observability + the
    /// no-orphan invariant: START count must equal STOP count once drained).
    pub fn ledger_rows(&self) -> Vec<LedgerEvent> {
        self.ledger_store.rows()
    }

    /// Reconcile the app-side side-table against the coordinator's registry,
    /// dropping any entry the coordinator no longer has live (terminal/reaped),
    /// and return the still-live tracked sessions paired with their current
    /// lifecycle state. This is the single source for `list_active_sessions`.
    fn live_tracked_sessions(&self) -> Vec<(ModelInstanceId, TrackedSession, ModelSessionState)> {
        let mut table = self.sessions.lock().expect("swarm session table poisoned");
        let mut out = Vec::new();
        table.retain(|iid, tracked| match self.coordinator.session_state(*iid) {
            Some(state) if !state.is_terminal() => {
                out.push((*iid, tracked.clone(), state));
                true
            }
            // Terminal or unknown (evicted/reaped): drop from the side-table.
            _ => false,
        });
        out
    }

    /// Resolve the live runtime + model id for a tracked instance, reconciling
    /// the side-table first. Returns `None` if the session is gone (so the chat
    /// command can surface a typed "session no longer live" error).
    fn resolve_runtime(
        &self,
        iid: ModelInstanceId,
    ) -> Option<(Arc<dyn ModelRuntime>, ModelId)> {
        // Reconcile first so a stale entry does not hand back a freed runtime.
        let live = match self.coordinator.session_state(iid) {
            Some(state) if !state.is_terminal() => true,
            _ => false,
        };
        let mut table = self.sessions.lock().expect("swarm session table poisoned");
        if !live {
            table.remove(&iid);
            return None;
        }
        table.get(&iid).map(|t| (t.runtime.clone(), t.model_id))
    }
}

// ---------------------------------------------------------------------------
// IPC DTOs
// ---------------------------------------------------------------------------

/// Provider the operator selects in the spawn form. Mirrors the local/cloud
/// dispatch the production factory supports.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SwarmProviderIpc {
    Local,
    ByokCloud,
    OfficialCli,
}

impl SwarmProviderIpc {
    fn to_provider_kind(self) -> ProviderKind {
        match self {
            Self::Local => ProviderKind::Local,
            Self::ByokCloud => ProviderKind::ByokCloud,
            Self::OfficialCli => ProviderKind::OfficialCli,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SwarmRuntimeBindingIpc {
    LlamaCpp,
    Candle,
}

impl SwarmRuntimeBindingIpc {
    fn to_runtime_binding(self) -> RuntimeBinding {
        match self {
            Self::LlamaCpp => RuntimeBinding::LlamaCpp,
            Self::Candle => RuntimeBinding::Candle,
        }
    }
}

/// Spawn request from the control-room form. For a local spawn the operator
/// supplies the artifact path + its expected sha256 (the integrity gate) and a
/// runtime binding (candle / llama.cpp). For a cloud spawn the operator picks a
/// provider + a cloud model name.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmSpawnRequestIpc {
    pub provider: SwarmProviderIpc,
    /// Local: required. The on-disk model artifact (safetensors / GGUF).
    #[serde(default)]
    pub artifact_path: Option<String>,
    /// Local: required. Expected sha256 hex of the artifact (integrity gate).
    #[serde(default)]
    pub sha256_expected: Option<String>,
    /// Local: required. candle | llama_cpp.
    #[serde(default)]
    pub runtime_binding: Option<SwarmRuntimeBindingIpc>,
    /// Cloud: required. The allowlisted cloud model name (e.g. `gpt-4o`).
    #[serde(default)]
    pub cloud_model_name: Option<String>,
    /// Which concurrent instance index of this model to spawn (default 0). The
    /// same artifact can run as several instances for throughput.
    #[serde(default)]
    pub instance: u32,
    /// Parent session id for ledger lineage (defaults to an operator tag).
    #[serde(default)]
    pub parent_session_id: Option<String>,
}

/// Identifier for a spawned session returned to the UI. The composite string
/// (`<model_id>#<instance>`) round-trips back through the cancel/chat commands.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmInstanceIdIpc {
    pub model_id: String,
    pub instance: u32,
    /// Canonical `<model_id>#<instance>` string the UI passes back verbatim.
    pub composite: String,
}

impl From<ModelInstanceId> for SwarmInstanceIdIpc {
    fn from(value: ModelInstanceId) -> Self {
        Self {
            model_id: value.model_id.to_string(),
            instance: value.instance,
            composite: value.to_string(),
        }
    }
}

/// One live session row for the control-room table.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmSessionIpc {
    pub instance_id: SwarmInstanceIdIpc,
    /// Lifecycle state (QUEUED / LOADING / READY / GENERATING / ...).
    pub state: String,
    pub provider: String,
    pub runtime_binding: String,
    pub artifact_path: Option<String>,
    pub cloud_model_name: Option<String>,
}

/// Resource budget snapshot for the control-room resource bar.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmResourceSnapshotIpc {
    /// Hard concurrency cap (semaphore permits).
    pub concurrency_cap: usize,
    /// Permits currently in use (cap - available).
    pub concurrency_in_use: usize,
    /// Permits currently available.
    pub concurrency_available: usize,
    /// Live (non-terminal) session count from the coordinator registry.
    pub live_sessions: usize,
    /// Remaining lifetime spawns before the monotonic ceiling (HBR-SWARM-002).
    pub lifetime_spawns_remaining: u64,
    /// Optional token / cost ceilings (None = uncapped this run).
    pub tokens_remaining: Option<u64>,
    pub cost_micros_remaining: Option<u64>,
    pub budget_exhausted: bool,
}

/// Result of a chat generate: the full produced text + the token count. Honest
/// real output from the spawned local model — not faked text.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmChatResponseIpc {
    pub text: String,
    pub token_count: usize,
    /// Reason the generation finished (stop / length / cancelled / error), as
    /// reported by the runtime on the final token, if any.
    pub finish_reason: Option<String>,
}

// ---------------------------------------------------------------------------
// Tauri commands (thin; delegate to the managed coordinator + side-table)
// ---------------------------------------------------------------------------

/// Spawn a swarm session: a local model from an artifact path, or a cloud
/// model. Enforces the coordinator bounds (concurrency / lifetime ceiling /
/// budget / breaker) and records a real process-ledger START. Returns the
/// minted `ModelInstanceId`. Honest typed errors propagate verbatim (e.g. a
/// cloud spawn with no configured lane surfaces `ProviderNotConfigured`).
#[tauri::command]
pub async fn kernel_swarm_spawn_session(
    request: SwarmSpawnRequestIpc,
    state: State<'_, SwarmRuntimeState>,
) -> Result<SwarmInstanceIdIpc, String> {
    let _ = KERNEL_SWARM_SPAWN_SESSION_IPC_CHANNEL;
    let coordinator = state.coordinator();
    let spawn = build_spawn_request(&request)?;
    let instance_id = spawn.instance_id;
    match coordinator.spawn_session(spawn).await {
        Ok(iid) => {
            // §10.1 capture seam (first real swarm producer): when wired, open a
            // read-only capture session bound to this swarm's swimlane so the
            // operator can inspect its background work in the Terminal panel.
            let _ = state
                .attach_spawn_capture(request.parent_session_id.clone(), iid.to_string())
                .await;
            Ok(iid.into())
        }
        Err(error) => {
            // Reconcile the side-table: a failed spawn must not leave a tracked
            // entry (the TrackingFactory only inserts on success, but a
            // duplicate-rollback can race — reconcile to be safe).
            let _ = state.resolve_runtime(instance_id);
            eprintln!("{FR_EVT_SWARM_LIFECYCLE}: spawn failed: {error}");
            Err(error.to_string())
        }
    }
}

/// Cancel a live session: cancels its token, frees the model (teardown), writes
/// the ledger STOP, evicts it. The app-side side-table entry is reconciled away.
#[tauri::command]
pub async fn kernel_swarm_cancel_session(
    instance_id: String,
    state: State<'_, SwarmRuntimeState>,
) -> Result<(), String> {
    let _ = KERNEL_SWARM_CANCEL_SESSION_IPC_CHANNEL;
    let iid = parse_instance_id(&instance_id)?;
    let coordinator = state.coordinator();
    let result = coordinator
        .cancel_session(iid, "operator_cancel")
        .await
        .map_err(|error| error.to_string());
    // Close the §10.1 capture session bound to this instance so the Terminal
    // panel marks it exited (and does not leak a ghost capture tab).
    state.close_capture_sink(&iid.to_string(), 0).await;
    // Reconcile regardless of outcome so a now-terminal session leaves the table.
    let _ = state.resolve_runtime(iid);
    result
}

/// List the live (non-terminal) swarm sessions with their model id, instance,
/// lifecycle state, and provider/runtime descriptor. Reconciles the side-table
/// against the coordinator registry first (drops reaped/terminal entries).
#[tauri::command]
pub async fn kernel_swarm_list_active_sessions(
    state: State<'_, SwarmRuntimeState>,
) -> Result<Vec<SwarmSessionIpc>, String> {
    let _ = KERNEL_SWARM_LIST_ACTIVE_SESSIONS_IPC_CHANNEL;
    let mut rows: Vec<SwarmSessionIpc> = state
        .live_tracked_sessions()
        .into_iter()
        .map(|(iid, tracked, st)| SwarmSessionIpc {
            instance_id: iid.into(),
            state: st.as_str().to_string(),
            provider: provider_str(tracked.provider).to_string(),
            runtime_binding: tracked.runtime_binding.adapter_id().to_string(),
            artifact_path: tracked.artifact_path,
            cloud_model_name: tracked.cloud_model_name,
        })
        .collect();
    // Stable ordering for the table (by model id then instance).
    rows.sort_by(|a, b| {
        a.instance_id
            .model_id
            .cmp(&b.instance_id.model_id)
            .then(a.instance_id.instance.cmp(&b.instance_id.instance))
    });
    Ok(rows)
}

// ---------------------------------------------------------------------------
// rank-4: live operator board (Jira-style). Board = a READ-MODEL projection of
// SwarmEvents: columns = ModelSessionState, swimlanes = the rank-2 grouping key
// (swarm_id / worktree_id), cards = sessions. The React board calls
// `kernel_swarm_board_snapshot` on mount + on resync, then applies pushed
// `swarm://event` deltas in place; a `seq` gap or `swarm://resync` triggers a
// full reconcile. UI writes are COMMANDS (spawn/cancel/escalate), never direct
// column mutations.
// ---------------------------------------------------------------------------

/// One board card = one live session, with its lifecycle state (the column) and
/// the swarm/worktree grouping (the swimlane).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmBoardCardIpc {
    pub instance_id: SwarmInstanceIdIpc,
    pub state: String,
    pub provider: String,
    pub runtime_binding: String,
    pub swarm_id: Option<String>,
    pub worktree_id: Option<String>,
}

/// Reconcilable board snapshot the React hook fetches on mount + on resync.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmBoardSnapshotIpc {
    pub cards: Vec<SwarmBoardCardIpc>,
    pub live_sessions: usize,
}

/// A pushed live delta: a monotonic `seq` + the typed SwarmEvent. The board
/// applies it in place; a gap in `seq` means a missed event -> full reconcile.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmBoardDeltaIpc {
    pub seq: u64,
    pub event: SwarmEvent,
}

/// Emitted when the broadcast receiver lagged (dropped events) so the board does
/// a full snapshot reconcile instead of applying a partial stream.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmBoardResyncIpc {
    pub dropped: u64,
}

/// Board snapshot: the live sessions as cards with state + grouping, for the
/// initial render and every reconcile.
#[tauri::command]
pub async fn kernel_swarm_board_snapshot(
    state: State<'_, SwarmRuntimeState>,
) -> Result<SwarmBoardSnapshotIpc, String> {
    let coordinator = state.coordinator();
    let mut cards: Vec<SwarmBoardCardIpc> = state
        .live_tracked_sessions()
        .into_iter()
        .map(|(iid, tracked, st)| {
            let (swarm_id, worktree_id) = coordinator.session_grouping(iid).unwrap_or((None, None));
            SwarmBoardCardIpc {
                instance_id: iid.into(),
                state: st.as_str().to_string(),
                provider: provider_str(tracked.provider).to_string(),
                runtime_binding: tracked.runtime_binding.adapter_id().to_string(),
                swarm_id,
                worktree_id,
            }
        })
        .collect();
    cards.sort_by(|a, b| {
        a.instance_id
            .model_id
            .cmp(&b.instance_id.model_id)
            .then(a.instance_id.instance.cmp(&b.instance_id.instance))
    });
    Ok(SwarmBoardSnapshotIpc {
        live_sessions: cards.len(),
        cards,
    })
}

/// rank-4: forward live SwarmEvents to the React board. ONE forwarder task (a
/// single serializer -> ordered deltas) subscribes to the broadcast and re-emits
/// each as a typed `swarm://event` with a monotonic seq; a lagged (dropped)
/// receiver emits `swarm://resync` so the board reconciles rather than drifting.
/// Called once from the Tauri setup with the managed state's `board_events()`.
pub fn spawn_swarm_board_forwarder(app: tauri::AppHandle, board: Arc<BroadcastSwarmSink>) {
    use tauri::Emitter;
    let mut rx = board.subscribe();
    tauri::async_runtime::spawn(async move {
        let mut seq: u64 = 0;
        loop {
            match rx.recv().await {
                Ok(event) => {
                    seq = seq.saturating_add(1);
                    let _ = app.emit("swarm://event", SwarmBoardDeltaIpc { seq, event });
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(dropped)) => {
                    let _ = app.emit("swarm://resync", SwarmBoardResyncIpc { dropped });
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

/// Snapshot the swarm resource budget for the control-room resource bar.
#[tauri::command]
pub async fn kernel_swarm_resource_snapshot(
    state: State<'_, SwarmRuntimeState>,
) -> Result<SwarmResourceSnapshotIpc, String> {
    let _ = KERNEL_SWARM_RESOURCE_SNAPSHOT_IPC_CHANNEL;
    let coordinator = state.coordinator();
    let remaining = coordinator.remaining();
    let cap = state.concurrency_cap();
    let available = remaining.concurrency_permits_available;
    Ok(SwarmResourceSnapshotIpc {
        concurrency_cap: cap,
        concurrency_in_use: cap.saturating_sub(available),
        concurrency_available: available,
        live_sessions: coordinator.live_session_count(),
        lifetime_spawns_remaining: remaining.lifetime_spawns_remaining,
        tokens_remaining: remaining.tokens_remaining,
        cost_micros_remaining: remaining.cost_micros_remaining,
        budget_exhausted: remaining.exhausted,
    })
}

/// Run a REAL generate against a spawned LOCAL-model session and return the
/// produced text. This is the operator <-> local-model chat box backend: it
/// resolves the live `Arc<dyn ModelRuntime>` the coordinator registered for the
/// instance (NOT a second load, NOT faked text), drives the runtime's real
/// `generate` stream, and concatenates the produced token text.
#[tauri::command]
pub async fn kernel_swarm_chat_generate(
    instance_id: String,
    prompt: String,
    state: State<'_, SwarmRuntimeState>,
) -> Result<SwarmChatResponseIpc, String> {
    let _ = KERNEL_SWARM_CHAT_GENERATE_IPC_CHANNEL;
    let trimmed = prompt.trim();
    if trimmed.is_empty() {
        return Err("prompt must not be empty".to_string());
    }
    let iid = parse_instance_id(&instance_id)?;
    let (runtime, model_id) = state
        .resolve_runtime(iid)
        .ok_or_else(|| format!("swarm session {iid} is no longer live (cancelled or reaped)"))?;

    let coordinator = state.coordinator();
    // Drive the lifecycle: READY -> GENERATING for the turn, then back to READY.
    // A failed transition is non-fatal to the generate (the session may already
    // be Generating from a concurrent turn); the real bound is the runtime's own
    // busy guard, which returns a typed error stream rather than deadlocking.
    let _ = coordinator.transition(iid, ModelSessionState::Generating);

    let request = GenerateRequest {
        id: model_id,
        prompt: GenPrompt::new(trimmed),
        sampling: SamplingParams::default(),
        lora_overrides: vec![],
        steering_overrides: vec![],
        kv_prefix_handle: None,
        cancel: CancellationToken::new(),
        max_tokens: CHAT_MAX_TOKENS,
        stop_sequences: vec![],
        speculative_mode: None,
        structured_decoding: None,
    };

    // §10.1 capture seam — REAL live producer: if a capture session was opened
    // for this instance at spawn, fan the model's generated token stream into it
    // as tokens are produced, so the "inspect all background work" panel shows
    // the live output of this background model run (redacted by the sink). This
    // is the actual model output, not a synthetic marker.
    let capture_sink = state.capture_sink_for(&iid.to_string());
    if let Some(sink) = &capture_sink {
        // Echo the prompt turn so the captured transcript is self-describing.
        sink.feed(format!("\r\n$ generate: {trimmed}\r\n").as_bytes())
            .await;
    }

    let mut stream = runtime.generate(request);
    let mut text = String::new();
    let mut token_count = 0usize;
    let mut finish_reason: Option<String> = None;
    let mut error: Option<String> = None;
    while let Some(item) = stream.next().await {
        match item {
            Ok(token) => {
                text.push_str(&token.text);
                token_count += 1;
                // Live fan-out: each produced token's text streams into the
                // capture session immediately (the live background-work stream).
                if let Some(sink) = &capture_sink {
                    if !token.text.is_empty() {
                        sink.feed(token.text.as_bytes()).await;
                    }
                }
                if let Some(reason) = token.finish_reason {
                    finish_reason = Some(format!("{reason:?}").to_lowercase());
                }
            }
            Err(e) => {
                error = Some(e.to_string());
                break;
            }
        }
    }
    drop(stream);
    if let Some(sink) = &capture_sink {
        // Terminate the captured turn with a newline so the next turn is framed.
        sink.feed(b"\r\n").await;
    }

    // Return the session to READY for the next turn (best-effort).
    let _ = coordinator.transition(iid, ModelSessionState::Ready);

    if let Some(error) = error {
        eprintln!("{FR_EVT_SWARM_LIFECYCLE}: chat generate failed for {iid}: {error}");
        return Err(format!("generate failed: {error}"));
    }

    Ok(SwarmChatResponseIpc {
        text,
        token_count,
        finish_reason,
    })
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn provider_str(provider: ProviderKind) -> &'static str {
    match provider {
        ProviderKind::Local => "local",
        ProviderKind::ExternalCompat => "external_compat",
        ProviderKind::ByokCloud => "byok_cloud",
        ProviderKind::OfficialCli => "official_cli",
    }
}

/// Build the typed [`SpawnRequest`] from the IPC DTO, validating that the
/// provider-appropriate fields are present. Local requires artifact path +
/// sha256 + runtime binding; cloud requires a cloud model name.
fn build_spawn_request(request: &SwarmSpawnRequestIpc) -> Result<SpawnRequest, String> {
    let model_id = ModelId::new_v7();
    let instance_id = ModelInstanceId::new(model_id, request.instance);
    let parent = request
        .parent_session_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("operator-swarm-control-room")
        .to_string();

    match request.provider {
        SwarmProviderIpc::Local => {
            let binding = request
                .runtime_binding
                .ok_or_else(|| "local spawn requires a runtime_binding (candle | llama_cpp)".to_string())?
                .to_runtime_binding();
            let path = request
                .artifact_path
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| "local spawn requires a non-empty artifact_path".to_string())?;
            let sha = request
                .sha256_expected
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| {
                    "local spawn requires sha256_expected (the integrity gate)".to_string()
                })?;
            Ok(
                SpawnRequest::new(instance_id, binding, "operator", parent)
                    .with_local_artifact(path.to_string(), sha.to_string()),
            )
        }
        SwarmProviderIpc::ByokCloud | SwarmProviderIpc::OfficialCli => {
            let model_name = request
                .cloud_model_name
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| "cloud spawn requires a non-empty cloud_model_name".to_string())?;
            // runtime_binding is ignored for cloud runtime selection but the
            // SpawnRequest constructor needs a value; Candle is a harmless
            // placeholder the cloud dispatch path never reads for engine choice.
            Ok(SpawnRequest::new(
                instance_id,
                RuntimeBinding::Candle,
                "operator",
                parent,
            )
            .with_cloud_provider(request.provider.to_provider_kind(), model_name.to_string()))
        }
    }
}

/// Parse a `<model_id>#<instance>` composite back into a [`ModelInstanceId`].
fn parse_instance_id(value: &str) -> Result<ModelInstanceId, String> {
    let trimmed = value.trim();
    let (model_part, instance_part) = trimmed
        .split_once('#')
        .ok_or_else(|| format!("instance_id must be '<model_id>#<instance>': {trimmed}"))?;
    let uuid = Uuid::parse_str(model_part.trim())
        .map_err(|error| format!("invalid model_id in instance_id: {error}"))?;
    let model_id = ModelId::from(uuid);
    if model_id.as_uuid().get_version_num() != 7 {
        return Err(format!("model_id must be UUID v7: {model_part}"));
    }
    let instance: u32 = instance_part
        .trim()
        .parse()
        .map_err(|error| format!("invalid instance index in instance_id: {error}"))?;
    Ok(ModelInstanceId::new(model_id, instance))
}

/// Map a [`SwarmError`] kind check helper used by tests to assert the cloud
/// not-configured path surfaces honestly through the command layer.
#[allow(dead_code)]
fn is_provider_not_configured(error: &SwarmError) -> bool {
    matches!(error, SwarmError::ProviderNotConfigured { .. })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn swarm_runtime_state_constructs_and_exposes_a_live_coordinator() {
        let state = SwarmRuntimeState::production();
        let coordinator = state.coordinator();
        // Fresh coordinator: no live sessions, budget headroom present.
        assert_eq!(coordinator.live_session_count(), 0);
        let remaining = coordinator.remaining();
        assert!(remaining.lifetime_spawns_remaining > 0);
        assert!(remaining.concurrency_permits_available >= 1);
        assert!(state.ledger_rows().is_empty());
        // The side-table starts empty.
        assert!(state.live_tracked_sessions().is_empty());
    }

    fn local_request(path: &str) -> SwarmSpawnRequestIpc {
        SwarmSpawnRequestIpc {
            provider: SwarmProviderIpc::Local,
            artifact_path: Some(path.to_string()),
            sha256_expected: Some("ab".repeat(32)),
            runtime_binding: Some(SwarmRuntimeBindingIpc::Candle),
            cloud_model_name: None,
            instance: 0,
            parent_session_id: None,
        }
    }

    #[test]
    fn build_spawn_request_local_requires_artifact_and_sha_and_binding() {
        // Missing binding.
        let mut req = local_request("some/model.safetensors");
        req.runtime_binding = None;
        let err = build_spawn_request(&req).expect_err("missing binding rejected");
        assert!(err.contains("runtime_binding"), "{err}");

        // Missing path.
        let mut req = local_request("");
        req.artifact_path = Some("   ".to_string());
        let err = build_spawn_request(&req).expect_err("empty path rejected");
        assert!(err.contains("artifact_path"), "{err}");

        // Missing sha.
        let mut req = local_request("some/model.safetensors");
        req.sha256_expected = None;
        let err = build_spawn_request(&req).expect_err("missing sha rejected");
        assert!(err.contains("sha256_expected"), "{err}");

        // Happy path produces a local candle SpawnRequest with the artifact set.
        let req = local_request("some/model.safetensors");
        let spawn = build_spawn_request(&req).expect("valid local request");
        assert_eq!(spawn.runtime_binding, RuntimeBinding::Candle);
        assert_eq!(spawn.model_artifact_path(), Some("some/model.safetensors"));
        assert_eq!(spawn.provider, None);
    }

    #[test]
    fn build_spawn_request_cloud_requires_model_name() {
        let req = SwarmSpawnRequestIpc {
            provider: SwarmProviderIpc::ByokCloud,
            artifact_path: None,
            sha256_expected: None,
            runtime_binding: None,
            cloud_model_name: None,
            instance: 0,
            parent_session_id: None,
        };
        let err = build_spawn_request(&req).expect_err("cloud requires model name");
        assert!(err.contains("cloud_model_name"), "{err}");

        let req = SwarmSpawnRequestIpc {
            provider: SwarmProviderIpc::ByokCloud,
            artifact_path: None,
            sha256_expected: None,
            runtime_binding: None,
            cloud_model_name: Some("gpt-4o".to_string()),
            instance: 0,
            parent_session_id: None,
        };
        let spawn = build_spawn_request(&req).expect("valid cloud request");
        assert_eq!(spawn.provider, Some(ProviderKind::ByokCloud));
        assert_eq!(spawn.cloud_model_name.as_deref(), Some("gpt-4o"));
    }

    #[test]
    fn parse_instance_id_round_trips_and_rejects_bad_input() {
        let model_id = ModelId::new_v7();
        let iid = ModelInstanceId::new(model_id, 3);
        let composite = iid.to_string();
        let parsed = parse_instance_id(&composite).expect("round trips");
        assert_eq!(parsed, iid);

        assert!(parse_instance_id("no-hash").is_err());
        assert!(parse_instance_id("not-a-uuid#0").is_err());
        assert!(parse_instance_id(&format!("{model_id}#notnum")).is_err());
        // nil uuid is not v7.
        assert!(parse_instance_id(&format!("{}#0", Uuid::nil())).is_err());
    }

    /// Cloud spawn through the real managed coordinator + production factory
    /// (cloud unconfigured) must surface `ProviderNotConfigured` honestly all
    /// the way out of the command as a string error — no placeholder success.
    #[tokio::test]
    async fn cloud_spawn_without_lane_surfaces_provider_not_configured_through_command_layer() {
        let state = SwarmRuntimeState::production();
        let coordinator = state.coordinator();
        let req = SwarmSpawnRequestIpc {
            provider: SwarmProviderIpc::ByokCloud,
            artifact_path: None,
            sha256_expected: None,
            runtime_binding: None,
            cloud_model_name: Some("gpt-4o".to_string()),
            instance: 0,
            parent_session_id: None,
        };
        let spawn = build_spawn_request(&req).expect("valid cloud request");
        let err = coordinator
            .spawn_session(spawn)
            .await
            .expect_err("cloud spawn with no configured lane must fail");
        assert!(is_provider_not_configured(&err), "got {err}");
        assert!(
            err.to_string().contains("PROVIDER_NOT_CONFIGURED"),
            "{err}"
        );
        // No live session, no tracked entry, no orphan ledger START.
        assert_eq!(coordinator.live_session_count(), 0);
        assert!(state.live_tracked_sessions().is_empty());
    }

    /// A local spawn whose artifact does not exist must fail the real load
    /// (sha/file gate) with a typed FactoryFailed, leave no live session, and
    /// leave NO tracked side-table entry (the TrackingFactory inserts only on a
    /// successful create).
    #[tokio::test]
    async fn local_spawn_nonexistent_artifact_fails_and_leaves_no_tracked_session() {
        let state = SwarmRuntimeState::production();
        let coordinator = state.coordinator();
        let req = local_request("D:/__handshake_no_such_swarm_app_model__/model.safetensors");
        let spawn = build_spawn_request(&req).expect("valid local request");
        let err = coordinator
            .spawn_session(spawn)
            .await
            .expect_err("nonexistent artifact must fail the real load");
        assert!(matches!(err, SwarmError::FactoryFailed(_)), "got {err}");
        assert_eq!(coordinator.live_session_count(), 0);
        assert!(state.live_tracked_sessions().is_empty());
    }

    /// `resolve_runtime` on an instance that was never spawned returns None
    /// (the chat command then surfaces a typed "no longer live" error).
    #[tokio::test]
    async fn resolve_runtime_unknown_instance_is_none() {
        let state = SwarmRuntimeState::production();
        let iid = ModelInstanceId::new(ModelId::new_v7(), 0);
        assert!(state.resolve_runtime(iid).is_none());
    }

    // ---- MT-206: vault-backed cloud lane dispatch (no live network) ----

    use handshake_core::model_runtime::cloud::InMemorySecretsVault;

    /// With a vault holding an anthropic key under the configured lane, a cloud
    /// ByokCloud spawn for an allowlisted model BUILDS a real session: the
    /// anthropic BYOK runtime is constructed, its allowlist-gated `load` mints a
    /// ModelId, the coordinator registers a live session, and the TrackingFactory
    /// records it. This proves the cloud builder dispatch SUCCESS shape without a
    /// live HTTP call (the runtime is real; no generate is issued here).
    #[tokio::test]
    async fn configured_cloud_lane_builds_a_live_cloud_session() {
        let vault = Arc::new(InMemorySecretsVault::default());
        vault
            .put("anthropic", "sk-ant-test-key-do-not-log".to_string())
            .expect("store anthropic key");
        let state = SwarmRuntimeState::production_with_cloud_vault(
            vault,
            Some("anthropic".to_string()),
            // No openai lane so ByokCloud resolves to the anthropic builder.
            None,
        );
        let coordinator = state.coordinator();
        let req = SwarmSpawnRequestIpc {
            provider: SwarmProviderIpc::ByokCloud,
            artifact_path: None,
            sha256_expected: None,
            runtime_binding: None,
            // An allowlisted Claude family model name (anthropic allowlist).
            cloud_model_name: Some("claude-sonnet-4".to_string()),
            instance: 0,
            parent_session_id: None,
        };
        let spawn = build_spawn_request(&req).expect("valid cloud request");
        let iid = coordinator
            .spawn_session(spawn)
            .await
            .expect("configured cloud spawn builds a live session");
        assert_eq!(coordinator.live_session_count(), 1);
        // The session is tracked and resolvable (real Arc<dyn ModelRuntime>).
        assert!(state.resolve_runtime(iid).is_some());
        // Clean teardown: cancel evicts the session with no orphan.
        coordinator
            .cancel_session(iid, "test_cancel")
            .await
            .expect("cancel");
        assert_eq!(coordinator.live_session_count(), 0);
    }

    /// A configured lane whose vault has NO key still surfaces the honest
    /// `ProviderNotConfigured` (the credential preflight in the builder fails).
    #[tokio::test]
    async fn configured_lane_without_key_surfaces_provider_not_configured() {
        let vault = Arc::new(InMemorySecretsVault::default());
        let state = SwarmRuntimeState::production_with_cloud_vault(
            vault,
            Some("anthropic".to_string()),
            None,
        );
        let coordinator = state.coordinator();
        let req = SwarmSpawnRequestIpc {
            provider: SwarmProviderIpc::ByokCloud,
            artifact_path: None,
            sha256_expected: None,
            runtime_binding: None,
            cloud_model_name: Some("claude-sonnet-4".to_string()),
            instance: 0,
            parent_session_id: None,
        };
        let spawn = build_spawn_request(&req).expect("valid cloud request");
        let err = coordinator
            .spawn_session(spawn)
            .await
            .expect_err("no key -> not configured");
        assert!(is_provider_not_configured(&err), "got {err}");
        assert_eq!(coordinator.live_session_count(), 0);
    }

    /// A non-allowlisted cloud model name fails the adapter's real allowlist
    /// gate inside `load`, surfaced honestly as ProviderNotConfigured (genuine
    /// runtime condition), leaving no live session / no orphan ledger START.
    #[tokio::test]
    async fn configured_lane_rejects_non_allowlisted_model_honestly() {
        let vault = Arc::new(InMemorySecretsVault::default());
        vault
            .put("anthropic", "sk-ant-test-key".to_string())
            .expect("store key");
        let state = SwarmRuntimeState::production_with_cloud_vault(
            vault,
            Some("anthropic".to_string()),
            None,
        );
        let coordinator = state.coordinator();
        let req = SwarmSpawnRequestIpc {
            provider: SwarmProviderIpc::ByokCloud,
            artifact_path: None,
            sha256_expected: None,
            runtime_binding: None,
            cloud_model_name: Some("definitely-not-a-claude-model".to_string()),
            instance: 0,
            parent_session_id: None,
        };
        let spawn = build_spawn_request(&req).expect("valid cloud request");
        let err = coordinator
            .spawn_session(spawn)
            .await
            .expect_err("non-allowlisted model rejected");
        assert!(is_provider_not_configured(&err), "got {err}");
        assert_eq!(coordinator.live_session_count(), 0);
    }

    /// MIXED local + cloud parallel spawn through ONE coordinator: a cloud
    /// session (real anthropic BYOK runtime, credentialed via the vault) and a
    /// local candle session spawned concurrently. The local artifact does not
    /// exist so the local spawn fails the real load honestly (FactoryFailed)
    /// while the cloud session is live — proving both lanes route through the
    /// same coordinator independently. (A real local model is exercised by the
    /// env-gated candle proof in `production_factory.rs`.)
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn mixed_local_and_cloud_spawn_route_through_one_coordinator() {
        let vault = Arc::new(InMemorySecretsVault::default());
        vault
            .put("anthropic", "sk-ant-test-key".to_string())
            .expect("store key");
        let state = SwarmRuntimeState::production_with_cloud_vault(
            vault,
            Some("anthropic".to_string()),
            None,
        );
        let coordinator = state.coordinator();

        // Cloud spawn (instance 0) — credentialed, builds a live session.
        let cloud_req = build_spawn_request(&SwarmSpawnRequestIpc {
            provider: SwarmProviderIpc::ByokCloud,
            artifact_path: None,
            sha256_expected: None,
            runtime_binding: None,
            cloud_model_name: Some("claude-sonnet-4".to_string()),
            instance: 0,
            parent_session_id: None,
        })
        .expect("valid cloud request");

        // Local spawn (instance 1) — nonexistent artifact fails the real load.
        let local_req = build_spawn_request(&local_request(
            "D:/__handshake_no_such_mixed_model__/model.safetensors",
        ))
        .expect("valid local request");
        // Distinct instance index so the two sessions are independent.
        let local_req = SpawnRequest {
            instance_id: ModelInstanceId::new(local_req.instance_id.model_id, 1),
            ..local_req
        };

        let c1 = coordinator.clone();
        let c2 = coordinator.clone();
        let (cloud_res, local_res) = tokio::join!(
            async move { c1.spawn_session(cloud_req).await },
            async move { c2.spawn_session(local_req).await },
        );
        let cloud_iid = cloud_res.expect("cloud session live");
        assert!(
            matches!(local_res, Err(SwarmError::FactoryFailed(_))),
            "local spawn fails the real load honestly: {local_res:?}"
        );
        // Exactly the cloud session is live through the shared coordinator.
        assert_eq!(coordinator.live_session_count(), 1);
        coordinator
            .cancel_session(cloud_iid, "test_cancel")
            .await
            .expect("cancel cloud");
        assert_eq!(coordinator.live_session_count(), 0);
    }
}

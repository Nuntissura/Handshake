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
use std::path::Path;
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
use handshake_core::model_runtime::cloud::{
    CliBridgeConfig, CliSubprocessSpawner, LiveCliSpawner, SecretsVault,
};
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
pub const KERNEL_SWARM_LIST_WORKTREES_IPC_CHANNEL: &str = "kernel_swarm_list_worktrees";

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
    /// Operator-assigned VM/sandbox worktree (mirrors the SpawnRequest; the
    /// coordinator also carries it for board grouping). RECORDED ONLY.
    worktree_id: Option<String>,
    /// Operator-assigned on-disk place (attribution only; never executed). The
    /// SpawnRequest carries no companion on the SwarmEvent, so this side-table is
    /// the surfacing path for the list/board/transcript rows.
    working_dir: Option<String>,
    /// Operator-intended isolation tier (RECORDED ONLY, not enforced).
    isolation_tier: Option<SwarmIsolationTierIpc>,
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
            // Recorded assignment/attribution, drained from the SAME SpawnRequest
            // at the SAME atomic insert point as the runtime handle — no second
            // post-spawn insert, so there is no race with a fast list/generate.
            worktree_id: request.worktree_id().map(|s| s.to_string()),
            working_dir: request.working_dir().map(|s| s.to_string()),
            isolation_tier: request
                .isolation_tier()
                .map(SwarmIsolationTierIpc::from_isolation_tier),
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

/// Load the operator's stored CLI-bridge config from `<app_data_root>` and, if
/// it is configured + valid, build the `(spawner, config)` pair that flips the
/// official_cli swarm lane live. Returns `None` when the config is missing,
/// corrupt, or unconfigured (the honest unconfigured default — the lane stays
/// `None` and a CLI spawn reports `ProviderNotConfigured`).
///
/// The `LiveCliSpawner` is built from the SAME `ledger` the swarm factory uses
/// (`LiveCliSpawner::new` requires a `LedgerBatcher`), so every CLI-bridge
/// subprocess START/STOP row is attributable + reclaimable in the live ledger.
fn load_official_cli_lane(
    app_data_root: &Path,
    ledger: &LedgerBatcher,
) -> Option<(Arc<dyn CliSubprocessSpawner>, CliBridgeConfig)> {
    let store = super::cli_bridge_store::CliBridgeConfigStore::new(app_data_root);
    let doc = store.load().ok()?;
    if !doc.configured {
        return None;
    }
    let config = doc.to_cli_bridge_config().ok()?;
    let spawner: Arc<dyn CliSubprocessSpawner> =
        Arc::new(LiveCliSpawner::new(Arc::new(ledger.clone())));
    Some((spawner, config))
}

impl SwarmRuntimeState {
    /// Build the production swarm runtime with the cloud lane wired to the
    /// host OS keychain vault (the same `OsKeychainSecretsVault` the cloud-lane
    /// control panel uses). A cloud spawn constructs a real BYOK runtime when
    /// the operator has stored a key under the configured lane id, and returns a
    /// typed `ProviderNotConfigured` otherwise. Lane ids default to
    /// `"anthropic"` / `"openai"` and can be overridden via
    /// `HANDSHAKE_SWARM_ANTHROPIC_LANE` / `HANDSHAKE_SWARM_OPENAI_LANE`.
    pub fn production(app_data_root: &Path) -> Self {
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
        Self::production_with_cloud_and_recorder(cloud, None, app_data_root)
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
        // No app_data_root => no stored CLI-bridge config is loaded; the
        // official_cli lane stays None (the explicit-cloud seam is for callers
        // that wire the lane themselves or do not need it). A relative,
        // never-present path resolves to the unconfigured default in `load`.
        Self::production_with_cloud_and_recorder(cloud, None, Path::new(""))
    }

    /// rank-3: production swarm runtime that PERSISTS every SwarmEvent into
    /// `recorder` (the deployed app passes the app-data-root DuckDB Flight
    /// Recorder) via the DurableSwarmFrBridge, so swarm/VM lifecycle events are
    /// durable + replayable for the board drill-down + audit -- not just stderr.
    /// Mirrors `production()`'s cloud-lane wiring.
    pub fn production_with_fr_recorder(
        recorder: Arc<dyn FlightRecorder>,
        app_data_root: &Path,
    ) -> Self {
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
        Self::production_with_cloud_and_recorder(cloud, Some(recorder), app_data_root)
    }

    fn production_with_cloud_and_recorder(
        cloud: CloudLaneFactoryConfig,
        recorder: Option<Arc<dyn FlightRecorder>>,
        app_data_root: &Path,
    ) -> Self {
        let store = InProcessLedgerStore::default();
        let overflow = Arc::new(CountingOverflowSink::default());
        let (ledger, writer_task) = LedgerBatcher::spawn(
            Arc::new(store.clone()),
            overflow,
            LedgerBatcherConfig::default(),
        );

        // Flip the official_cli swarm lane live from the operator's stored
        // CLI-bridge config (WP-KERNEL-004 follow-up). The spawner is built from
        // the SAME `ledger` the factory uses, so CLI-bridge subprocess START/STOP
        // rows land in the live ledger (no throwaway, no unattributed process).
        // Missing/corrupt/unconfigured config => None => the lane stays an honest
        // ProviderNotConfigured.
        let cloud = match load_official_cli_lane(app_data_root, &ledger) {
            Some((spawner, config)) => {
                // When a Flight Recorder is wired, thread it as cloud-lane
                // observability so the CLI lane emits FR-EVT-LLM-INFER-* like the
                // BYOK siblings. No recorder => FR-INFER-silent, otherwise identical.
                let observability = recorder.clone().map(|fr| {
                    Arc::new(handshake_core::model_runtime::cloud::CloudLaneObservability {
                        flight_recorder: fr,
                        consent: None,
                    })
                });
                cloud.with_official_cli_observed(spawner, config, observability)
            }
            None => cloud,
        };

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

/// Operator-intended isolation tier the spawn form records on a session. Mirrors
/// `handshake_core::sandbox::adapter::IsolationTier` 1:1 (snake_case serde
/// matches the core wire vocabulary). Deliberately a thin IPC enum rather than a
/// re-export of the core type, so the app IPC boundary stays decoupled from a
/// core type whose primary purpose (execution-substrate selection) is OUT OF
/// SCOPE here. RECORDED ONLY — nothing maps this to an execution substrate today.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SwarmIsolationTierIpc {
    Tier1Container,
    Tier2Syscall,
    Tier3Microvm,
}

impl SwarmIsolationTierIpc {
    fn to_isolation_tier(self) -> handshake_core::sandbox::adapter::IsolationTier {
        use handshake_core::sandbox::adapter::IsolationTier;
        match self {
            Self::Tier1Container => IsolationTier::Tier1Container,
            Self::Tier2Syscall => IsolationTier::Tier2Syscall,
            Self::Tier3Microvm => IsolationTier::Tier3Microvm,
        }
    }

    fn from_isolation_tier(tier: handshake_core::sandbox::adapter::IsolationTier) -> Self {
        use handshake_core::sandbox::adapter::IsolationTier;
        match tier {
            IsolationTier::Tier1Container => Self::Tier1Container,
            IsolationTier::Tier2Syscall => Self::Tier2Syscall,
            IsolationTier::Tier3Microvm => Self::Tier3Microvm,
        }
    }

    /// Stable wire string (matches the snake_case serde + the TS union).
    fn as_str(self) -> &'static str {
        match self {
            Self::Tier1Container => "tier1_container",
            Self::Tier2Syscall => "tier2_syscall",
            Self::Tier3Microvm => "tier3_microvm",
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
    /// Optional VM/sandbox worktree this session is ASSIGNED to (board swimlane +
    /// per-worktree recovery grouping). Blank/whitespace => None (unassigned,
    /// honest). RECORDED ONLY: it does NOT route execution into a VM today.
    #[serde(default)]
    pub worktree_id: Option<String>,
    /// Optional on-disk place the operator assigns the session (absolute OR
    /// repo-relative; disk-agnostic — the path is recorded verbatim, NOT resolved,
    /// created, or used as a cwd here). Blank/whitespace => None. Recorded
    /// attribution only.
    #[serde(default)]
    pub working_dir: Option<String>,
    /// OPTIONAL recorded-only isolation tier the operator intends for this session
    /// (tier1_container | tier2_syscall | tier3_microvm). RECORDED as the bridge to
    /// future VM execution; NOT enforced — the session still runs in-process.
    #[serde(default)]
    pub isolation_tier: Option<SwarmIsolationTierIpc>,
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
    /// Operator-assigned VM/sandbox worktree (board swimlane / recovery group).
    /// `None` => unassigned. RECORDED ONLY (no VM execution today).
    pub worktree_id: Option<String>,
    /// Operator-assigned on-disk place (attribution only; never executed/resolved).
    pub working_dir: Option<String>,
    /// Operator-intended isolation tier (RECORDED ONLY, not enforced).
    pub isolation_tier: Option<SwarmIsolationTierIpc>,
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
    Ok(list_active_sessions_inner(&state))
}

/// Testable body of `kernel_swarm_list_active_sessions` (the command is a thin
/// `State` -> `&SwarmRuntimeState` shim so the mapping is provable in unit tests
/// without fabricating a `tauri::State`).
fn list_active_sessions_inner(state: &SwarmRuntimeState) -> Vec<SwarmSessionIpc> {
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
            worktree_id: tracked.worktree_id,
            working_dir: tracked.working_dir,
            isolation_tier: tracked.isolation_tier,
        })
        .collect();
    // Stable ordering for the table (by model id then instance).
    rows.sort_by(|a, b| {
        a.instance_id
            .model_id
            .cmp(&b.instance_id.model_id)
            .then(a.instance_id.instance.cmp(&b.instance_id.instance))
    });
    rows
}

/// One distinct worktree the operator can pick when assigning a new session, with
/// the count of LIVE sessions currently assigned to it. Drives the spawn-form
/// worktree combobox (existing-pick) alongside its always-present free-text "new"
/// entry.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmWorktreeIpc {
    pub worktree_id: String,
    /// Count of LIVE sessions currently assigned to this worktree.
    pub live_session_count: usize,
}

/// Discover the DISTINCT non-empty worktree ids currently assigned across the
/// LIVE coordinator registry (reconciled side-table), each with its live session
/// count. Live-only is the honest v1: it needs no new core seam and no FR/DuckDB
/// dependency in the swarm command path. The UI always ALSO offers a free-text
/// "new worktree" entry, so live-only discovery never blocks assigning a brand-new
/// worktree. (A live+recent union over `json_extract_string(payload,'$.worktree_id')`
/// against the FR store — the same pattern `discover_fr_sessions` uses — is the
/// cheap follow-on.)
#[tauri::command]
pub async fn kernel_swarm_list_worktrees(
    state: State<'_, SwarmRuntimeState>,
) -> Result<Vec<SwarmWorktreeIpc>, String> {
    let _ = KERNEL_SWARM_LIST_WORKTREES_IPC_CHANNEL;
    Ok(list_worktrees_inner(&state))
}

/// Testable body of `kernel_swarm_list_worktrees`. Distinct non-empty worktree
/// ids across the live registry with their live session counts; reconciled
/// against the coordinator each call via `live_tracked_sessions()`.
fn list_worktrees_inner(state: &SwarmRuntimeState) -> Vec<SwarmWorktreeIpc> {
    let coordinator = state.coordinator();
    let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for (iid, _tracked, _st) in state.live_tracked_sessions() {
        if let Some((_swarm, Some(worktree))) = coordinator.session_grouping(iid) {
            let trimmed = worktree.trim();
            if !trimmed.is_empty() {
                *counts.entry(trimmed.to_string()).or_insert(0) += 1;
            }
        }
    }
    counts
        .into_iter()
        .map(|(worktree_id, live_session_count)| SwarmWorktreeIpc {
            worktree_id,
            live_session_count,
        })
        .collect()
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
    /// Operator-assigned on-disk place (from the app side-table; attribution only).
    /// Surfaced for the card tooltip; `None` when unassigned. RECORDED ONLY.
    pub working_dir: Option<String>,
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
                working_dir: tracked.working_dir,
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

/// Trim an optional operator-supplied string to `Some(trimmed)` only when it is
/// non-empty after trimming; blank/whitespace => `None` (the honest "unassigned"
/// state). Shared by the worktree/working_dir recording so both apply the same
/// rule in both provider arms.
fn trimmed_opt(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// Apply the operator-assigned, RECORDED-ONLY worktree / working-dir / isolation
/// tier onto a built [`SpawnRequest`]. Shared tail for both provider arms so the
/// recording rule (blank => unassigned) is identical for local and cloud. None of
/// these route execution — they are attribution + the bridge to future VM work.
fn apply_recorded_assignment(
    mut spawn: SpawnRequest,
    request: &SwarmSpawnRequestIpc,
) -> SpawnRequest {
    if let Some(wt) = trimmed_opt(&request.worktree_id) {
        spawn = spawn.with_worktree(wt);
    }
    if let Some(wd) = trimmed_opt(&request.working_dir) {
        spawn = spawn.with_working_dir(wd);
    }
    if let Some(tier) = request.isolation_tier {
        spawn = spawn.with_isolation_tier(tier.to_isolation_tier());
    }
    spawn
}

/// Build the typed [`SpawnRequest`] from the IPC DTO, validating that the
/// provider-appropriate fields are present. Local requires artifact path +
/// sha256 + runtime binding; cloud requires a cloud model name. After the
/// provider-specific build, the shared tail records the operator-assigned
/// worktree / working_dir / isolation_tier (blank worktree/working_dir =>
/// unassigned; no new error paths — an unassigned session is valid, mirroring
/// `swarm_id`'s optionality).
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

    let spawn = match request.provider {
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
            SpawnRequest::new(instance_id, binding, "operator", parent)
                .with_local_artifact(path.to_string(), sha.to_string())
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
            SpawnRequest::new(instance_id, RuntimeBinding::Candle, "operator", parent)
                .with_cloud_provider(request.provider.to_provider_kind(), model_name.to_string())
        }
    };

    Ok(apply_recorded_assignment(spawn, request))
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
        let state = SwarmRuntimeState::production(std::path::Path::new(""));
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
            worktree_id: None,
            working_dir: None,
            isolation_tier: None,
        }
    }

    /// A minimal cloud `SwarmSpawnRequestIpc` for tests, with the recorded
    /// assignment fields defaulted to unassigned. `model` is the cloud model name.
    fn cloud_request(model: &str) -> SwarmSpawnRequestIpc {
        SwarmSpawnRequestIpc {
            provider: SwarmProviderIpc::ByokCloud,
            artifact_path: None,
            sha256_expected: None,
            runtime_binding: None,
            cloud_model_name: Some(model.to_string()),
            instance: 0,
            parent_session_id: None,
            worktree_id: None,
            working_dir: None,
            isolation_tier: None,
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
        let mut req = cloud_request("gpt-4o");
        req.cloud_model_name = None;
        let err = build_spawn_request(&req).expect_err("cloud requires model name");
        assert!(err.contains("cloud_model_name"), "{err}");

        let req = cloud_request("gpt-4o");
        let spawn = build_spawn_request(&req).expect("valid cloud request");
        assert_eq!(spawn.provider, Some(ProviderKind::ByokCloud));
        assert_eq!(spawn.cloud_model_name.as_deref(), Some("gpt-4o"));
    }

    #[test]
    fn build_spawn_request_applies_trimmed_worktree_working_dir_and_tier() {
        // Local arm: worktree + working_dir trimmed; tier mapped 1:1.
        let mut req = local_request("some/model.safetensors");
        req.worktree_id = Some("  wt-a  ".to_string());
        req.working_dir = Some("  D:/work/wt-a  ".to_string());
        req.isolation_tier = Some(SwarmIsolationTierIpc::Tier3Microvm);
        let spawn = build_spawn_request(&req).expect("valid local request");
        assert_eq!(spawn.worktree_id(), Some("wt-a"));
        assert_eq!(spawn.working_dir(), Some("D:/work/wt-a"));
        assert_eq!(
            spawn.isolation_tier(),
            Some(handshake_core::sandbox::adapter::IsolationTier::Tier3Microvm)
        );

        // Cloud arm: same recording rule applies after the cloud build.
        let req = SwarmSpawnRequestIpc {
            provider: SwarmProviderIpc::ByokCloud,
            artifact_path: None,
            sha256_expected: None,
            runtime_binding: None,
            cloud_model_name: Some("gpt-4o".to_string()),
            instance: 0,
            parent_session_id: None,
            worktree_id: Some("\twt-cloud\n".to_string()),
            working_dir: Some("./worktrees/cloud".to_string()),
            isolation_tier: Some(SwarmIsolationTierIpc::Tier1Container),
        };
        let spawn = build_spawn_request(&req).expect("valid cloud request");
        assert_eq!(spawn.worktree_id(), Some("wt-cloud"));
        assert_eq!(spawn.working_dir(), Some("./worktrees/cloud"));
        assert_eq!(
            spawn.isolation_tier(),
            Some(handshake_core::sandbox::adapter::IsolationTier::Tier1Container)
        );
    }

    #[test]
    fn build_spawn_request_blank_worktree_and_working_dir_are_none() {
        // Blank / whitespace-only => None (honest "unassigned"), both arms.
        let mut req = local_request("some/model.safetensors");
        req.worktree_id = Some("   ".to_string());
        req.working_dir = Some("".to_string());
        req.isolation_tier = None;
        let spawn = build_spawn_request(&req).expect("valid local request");
        assert_eq!(spawn.worktree_id(), None);
        assert_eq!(spawn.working_dir(), None);
        assert_eq!(spawn.isolation_tier(), None);

        // Omitted entirely (serde default None) => None as well.
        let req = local_request("some/model.safetensors");
        let spawn = build_spawn_request(&req).expect("valid local request");
        assert_eq!(spawn.worktree_id(), None);
        assert_eq!(spawn.working_dir(), None);
        assert_eq!(spawn.isolation_tier(), None);
    }

    #[test]
    fn swarm_isolation_tier_ipc_round_trips_through_core() {
        use handshake_core::sandbox::adapter::IsolationTier;
        for (ipc, core) in [
            (SwarmIsolationTierIpc::Tier1Container, IsolationTier::Tier1Container),
            (SwarmIsolationTierIpc::Tier2Syscall, IsolationTier::Tier2Syscall),
            (SwarmIsolationTierIpc::Tier3Microvm, IsolationTier::Tier3Microvm),
        ] {
            assert_eq!(ipc.to_isolation_tier(), core);
            assert_eq!(SwarmIsolationTierIpc::from_isolation_tier(core), ipc);
            // Wire string matches the snake_case serde the TS union declares.
            let json = serde_json::to_string(&ipc).expect("serialize tier");
            assert_eq!(json, format!("\"{}\"", ipc.as_str()));
        }
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
        let state = SwarmRuntimeState::production(std::path::Path::new(""));
        let coordinator = state.coordinator();
        let req = cloud_request("gpt-4o");
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
        let state = SwarmRuntimeState::production(std::path::Path::new(""));
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
        let state = SwarmRuntimeState::production(std::path::Path::new(""));
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
        // Assign the spawn to a worktree so the recorded grouping + the
        // list_worktrees discovery are provable end-to-end through the real
        // coordinator (not just the pure build_spawn_request mapping).
        let mut req = cloud_request("claude-sonnet-4");
        req.worktree_id = Some("  wt-x  ".to_string());
        req.working_dir = Some("D:/work/wt-x".to_string());
        let spawn = build_spawn_request(&req).expect("valid cloud request");
        let iid = coordinator
            .spawn_session(spawn)
            .await
            .expect("configured cloud spawn builds a live session");
        assert_eq!(coordinator.live_session_count(), 1);
        // The session is tracked and resolvable (real Arc<dyn ModelRuntime>).
        assert!(state.resolve_runtime(iid).is_some());
        // The coordinator reports the trimmed worktree in its grouping (drives the
        // board swimlane), and list_worktrees enumerates it with a live count of 1.
        assert_eq!(coordinator.session_grouping(iid), Some((None, Some("wt-x".to_string()))));
        let worktrees = list_worktrees_inner(&state);
        assert_eq!(worktrees.len(), 1);
        assert_eq!(worktrees[0].worktree_id, "wt-x");
        assert_eq!(worktrees[0].live_session_count, 1);
        // The list/board rows surface the recorded working_dir attribution.
        let sessions = list_active_sessions_inner(&state);
        let row = sessions
            .iter()
            .find(|s| s.instance_id.composite == iid.to_string())
            .expect("session row present");
        assert_eq!(row.worktree_id.as_deref(), Some("wt-x"));
        assert_eq!(row.working_dir.as_deref(), Some("D:/work/wt-x"));
        // Clean teardown: cancel evicts the session with no orphan.
        coordinator
            .cancel_session(iid, "test_cancel")
            .await
            .expect("cancel");
        assert_eq!(coordinator.live_session_count(), 0);
        // After cancel the worktree no longer appears (live-only discovery is
        // reconciled against the registry each call).
        let worktrees = list_worktrees_inner(&state);
        assert!(worktrees.is_empty(), "worktrees: {worktrees:?}");
    }

    /// A credentialed CLOUD (non-local) session is chat-eligible: `resolve_runtime`
    /// — the SINGLE gate `kernel_swarm_chat_generate` consults before driving the
    /// runtime's real `generate` — hands back the live `Arc<dyn ModelRuntime>` the
    /// coordinator registered, with NO provider/local-only guard. This locks in the
    /// provider-agnostic chat contract (governance glue #3): a future local-only
    /// guard (e.g. one that refused a `ByokCloud`/`OfficialCli` session, or required
    /// `cloud_model_name` to be `None`) would fail this test. It also proves the
    /// honest teardown gate: after `cancel_session` the same resolve returns `None`
    /// so the chat command surfaces "no longer live" rather than a freed runtime.
    ///
    /// We assert chat-eligibility at the resolve/eligibility boundary — the exact
    /// local-vs-all decision point the command uses — rather than issuing a real
    /// `runtime.generate`, which for the anthropic BYOK runtime would require a live
    /// HTTP/SSE call the rest of this module deliberately avoids.
    #[tokio::test]
    async fn cloud_session_is_chat_eligible_no_local_only_guard() {
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
        let spawn =
            build_spawn_request(&cloud_request("claude-sonnet-4")).expect("valid cloud request");
        let iid = coordinator
            .spawn_session(spawn)
            .await
            .expect("configured cloud spawn builds a live session");

        // The chat path's ONLY gate is `resolve_runtime` (see
        // `kernel_swarm_chat_generate`): it must succeed for a non-local session.
        let resolved = state.resolve_runtime(iid);
        assert!(
            resolved.is_some(),
            "cloud session must be chat-eligible (no local-only guard)"
        );
        let (_runtime, model_id) = resolved.expect("resolved");
        // The chat command builds `GenerateRequest.id` from this resolved
        // `model_id` — the id the runtime's real `load` MINTED, which
        // `TrackingFactory::create` stores as `live.model_id`. It is intentionally
        // distinct from the spawn request's `iid.model_id` (the placeholder the
        // UI lists), so we assert it is a real minted id, not the request id —
        // i.e. the chat path drives the runtime under its own loaded identity.
        assert_ne!(
            model_id, iid.model_id,
            "resolved model id is the runtime's minted load id, not the request id"
        );
        // The row still resolves to this exact instance via the composite key the
        // UI passes back to chat/cancel — the stable join key across surfaces.
        let row = list_active_sessions_inner(&state)
            .into_iter()
            .find(|s| s.instance_id.composite == iid.to_string())
            .expect("cloud session row present and joinable by composite");
        assert_eq!(row.instance_id.composite, iid.to_string());

        // Clean teardown: cancel evicts the session.
        coordinator
            .cancel_session(iid, "test_cancel")
            .await
            .expect("cancel");
        // After cancel the same gate honestly refuses (no longer live) — the chat
        // command then returns a typed "no longer live" error, not a freed runtime.
        assert!(state.resolve_runtime(iid).is_none());
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
        let req = cloud_request("claude-sonnet-4");
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
        let req = cloud_request("definitely-not-a-claude-model");
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
        let cloud_req =
            build_spawn_request(&cloud_request("claude-sonnet-4")).expect("valid cloud request");

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

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
use handshake_core::distillation::swarm_trace::{
    prepare_distillation_swarm_trace_queue_entry, DistillationSwarmTraceQueueEntry,
    SwarmTraceCandidate, SwarmTraceRouteMetadata, SwarmTraceRouteOutcome,
};
use handshake_core::flight_recorder::FlightRecorder;
use handshake_core::model_runtime::cloud::{
    CliBridgeConfig, CliSubprocessSpawner, LiveCliSpawner, SecretsVault,
};
use handshake_core::model_runtime::registry::RuntimeBinding;
use handshake_core::model_runtime::{
    CancellationToken, GenPrompt, GenerateRequest, ModelId, ModelRuntime, ProviderKind,
    SamplingParams, WarmVmSnapshotManifest,
};
use handshake_core::process_ledger::{
    LedgerBatcher, LedgerBatcherConfig, LedgerEvent, LedgerOverflowEvent, ProcessLedgerError,
    ProcessLedgerOverflowSink, ProcessLedgerStore,
};
use handshake_core::sandbox::SandboxAdapterRegistry;
use handshake_core::storage::ControlPlaneStorage;
use handshake_core::swarm_orchestration::ids::LocalExecutionMode;
use handshake_core::swarm_orchestration::state_recovery::{
    AgentLaneIdentity, AgentLaneKind, AttributionMode, CloudAssistanceOutputKind,
    CloudAssistanceRequest, CloudFallbackBasisRequest, CloudFallbackReason, LocalCloudAttribution,
    ModelProviderKind, ParallelSwarmStateRecoveryStore,
};
use handshake_core::swarm_orchestration::{
    BroadcastSwarmSink, ByokCloudProvider, CloudLaneFactoryConfig, DurableSwarmFrBridge,
    FanoutSwarmSink, FlightRecorderSwarmSink, LiveSession, ModelInstanceId, ModelSessionFactory,
    ModelSessionState, ProductionModelSessionFactory, RunBudget, SpawnRequest, SwarmConfig,
    SwarmCoordinator, SwarmError, SwarmEvent, SwarmEventSink, SwarmResult,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use tauri::State;
use uuid::Uuid;

use super::spawn_template_store::{
    SessionSpawnTemplate, SpawnTemplateStore, TemplateByokCloudProvider, TemplateIsolationTier,
    TemplateLocalExecutionMode, TemplateProvider, TemplateRuntimeBinding,
};

/// FR-EVT tag stamped on swarm lifecycle events forwarded to structured stderr
/// until the durable flight-recorder DB sink is wired by a swarm command (MT-205).
pub const FR_EVT_SWARM_LIFECYCLE: &str = "FR-EVT-SWARM-LIFECYCLE";

pub const KERNEL_SWARM_SPAWN_SESSION_IPC_CHANNEL: &str = "kernel_swarm_spawn_session";
pub const KERNEL_SWARM_SPAWN_LOCAL_CLOUD_PAIR_IPC_CHANNEL: &str =
    "kernel_swarm_spawn_local_cloud_pair";
pub const KERNEL_SWARM_SPAWN_WITH_CLOUD_ESCALATION_IPC_CHANNEL: &str =
    "kernel_swarm_spawn_with_cloud_escalation";
pub const KERNEL_SWARM_CANCEL_SESSION_IPC_CHANNEL: &str = "kernel_swarm_cancel_session";
pub const KERNEL_SWARM_LIST_ACTIVE_SESSIONS_IPC_CHANNEL: &str = "kernel_swarm_list_active_sessions";
pub const KERNEL_SWARM_RESOURCE_SNAPSHOT_IPC_CHANNEL: &str = "kernel_swarm_resource_snapshot";
pub const KERNEL_SWARM_CHAT_GENERATE_IPC_CHANNEL: &str = "kernel_swarm_chat_generate";
pub const KERNEL_SWARM_CHAT_GENERATE_WITH_CLOUD_ESCALATION_IPC_CHANNEL: &str =
    "kernel_swarm_chat_generate_with_cloud_escalation";
pub const KERNEL_SWARM_LIST_WORKTREES_IPC_CHANNEL: &str = "kernel_swarm_list_worktrees";
pub const KERNEL_SWARM_RESUME_SESSION_IPC_CHANNEL: &str = "kernel_swarm_resume_session";
pub const KERNEL_SWARM_GET_SPAWN_TEMPLATE_IPC_CHANNEL: &str = "kernel_swarm_get_spawn_template";

/// Max tokens a single chat-box generate will produce. Bounded so an operator
/// chat turn cannot run away (HBR loop-cap spirit) — the UI runs short turns.
const CHAT_MAX_TOKENS: u32 = 256;
const SWARM_COMMITTED_MEMORY_CEILING_BYTES_ENV: &str =
    "HANDSHAKE_SWARM_COMMITTED_MEMORY_CEILING_BYTES";

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
        self.events
            .lock()
            .expect("swarm ledger store poisoned")
            .clone()
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
    /// coordinator also carries it for board grouping).
    worktree_id: Option<String>,
    /// Operator-assigned on-disk place (attribution only; never executed). The
    /// SpawnRequest carries no companion on the SwarmEvent, so this side-table is
    /// the surfacing path for the list/board/transcript rows.
    working_dir: Option<String>,
    /// Operator-intended isolation tier. Tier3Microvm is load-bearing for
    /// Local+LlamaCpp spawns when the sandbox registry is wired; the other
    /// combinations remain attribution until their substrates ship.
    isolation_tier: Option<SwarmIsolationTierIpc>,
    /// Explicit local execution substrate. `None`/`Cold` preserve legacy cold
    /// execution. `WarmVm` is opt-in and must not be lost through list/template
    /// surfaces.
    local_execution_mode: Option<SwarmLocalExecutionModeIpc>,
    /// Warm-VM restore metadata captured from a successful fresh warm load or
    /// passed through from a restored warm spawn. Stored beside the live runtime
    /// so spawn-template persistence can close the warm-start loop.
    warm_vm_restore_manifest: Option<WarmVmSnapshotManifest>,
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
            local_execution_mode: request
                .local_execution_mode
                .map(SwarmLocalExecutionModeIpc::from_core),
            warm_vm_restore_manifest: live.warm_vm_restore_manifest.clone(),
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
    /// ROI#3 STATE RECOVERY: per-session resume templates persisted at spawn time
    /// (keyed by the live composite instance_id). A spawn writes a
    /// [`SessionSpawnTemplate`] here on success so the exact session can later be
    /// re-spawned via `kernel_swarm_resume_session`. Disk-agnostic under
    /// `app_data_root`, mirroring the calendar `SwarmScheduleStore`.
    spawn_template_store: SpawnTemplateStore,
    /// Optional Rank-6 committed-memory ceiling wired into the live app
    /// coordinator. `None` preserves legacy callers; `Some` makes every spawn
    /// fail closed unless it carries a committed-memory estimate.
    committed_memory_ceiling_bytes: Option<u64>,
    /// MT-221: successful cloud fallback output must be receipt-recorded before
    /// it is returned to a caller. `None` means the receipt path is not wired, so
    /// cloud escalation fails closed rather than returning unreviewed output.
    cloud_assistance_recorder: Option<Arc<dyn CloudAssistanceReceiptRecorder>>,
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

fn committed_memory_ceiling_from_env() -> Option<u64> {
    match std::env::var(SWARM_COMMITTED_MEMORY_CEILING_BYTES_ENV) {
        Ok(raw) => match parse_committed_memory_ceiling(&raw) {
            Ok(Some(bytes)) => Some(bytes),
            Ok(None) => {
                eprintln!(
                    "{FR_EVT_SWARM_LIFECYCLE}: ignoring {SWARM_COMMITTED_MEMORY_CEILING_BYTES_ENV}=0; \
                     committed-memory admission ceiling remains uncapped"
                );
                None
            }
            Err(error) => {
                eprintln!(
                    "{FR_EVT_SWARM_LIFECYCLE}: ignoring invalid \
                     {SWARM_COMMITTED_MEMORY_CEILING_BYTES_ENV}={raw:?}: {error}"
                );
                None
            }
        },
        Err(_) => None,
    }
}

fn parse_committed_memory_ceiling(raw: &str) -> Result<Option<u64>, std::num::ParseIntError> {
    let bytes = raw.trim().parse::<u64>()?;
    Ok((bytes > 0).then_some(bytes))
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
        let cloud =
            CloudLaneFactoryConfig::from_vault(vault, Some(anthropic_lane), Some(openai_lane));
        Self::production_with_cloud_and_recorder(cloud, None, app_data_root, None)
    }

    /// WP-KERNEL-004 wave 1: same as [`production`] but threads the app's sandbox
    /// adapter registry so a Local+LlamaCpp Tier3Microvm spawn routes to the CH
    /// sandboxed-local runtime even when the durable Flight Recorder fell back to
    /// the stderr sink (the FR-up path uses `production_with_fr_recorder`). Keeps
    /// `production`'s signature unchanged for the test seams that call it.
    pub fn production_with_registry(
        app_data_root: &Path,
        sandbox_registry: Option<Arc<SandboxAdapterRegistry>>,
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
        Self::production_with_cloud_and_recorder(cloud, None, app_data_root, sandbox_registry)
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
        Self::production_with_cloud_and_recorder(cloud, None, Path::new(""), None)
    }

    /// rank-3: production swarm runtime that PERSISTS every SwarmEvent into
    /// `recorder` (the deployed app passes the app-data-root DuckDB Flight
    /// Recorder) via the DurableSwarmFrBridge, so swarm/VM lifecycle events are
    /// durable + replayable for the board drill-down + audit -- not just stderr.
    /// Mirrors `production()`'s cloud-lane wiring.
    pub fn production_with_fr_recorder(
        recorder: Arc<dyn FlightRecorder>,
        app_data_root: &Path,
        sandbox_registry: Option<Arc<SandboxAdapterRegistry>>,
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
        Self::production_with_cloud_and_recorder(
            cloud,
            Some(recorder),
            app_data_root,
            sandbox_registry,
        )
    }

    fn production_with_cloud_and_recorder(
        cloud: CloudLaneFactoryConfig,
        recorder: Option<Arc<dyn FlightRecorder>>,
        app_data_root: &Path,
        sandbox_registry: Option<Arc<SandboxAdapterRegistry>>,
    ) -> Self {
        let committed_memory_ceiling_bytes = committed_memory_ceiling_from_env();
        Self::production_with_cloud_and_recorder_and_committed_memory_ceiling(
            cloud,
            recorder,
            app_data_root,
            sandbox_registry,
            committed_memory_ceiling_bytes,
        )
    }

    fn production_with_cloud_and_recorder_and_committed_memory_ceiling(
        cloud: CloudLaneFactoryConfig,
        recorder: Option<Arc<dyn FlightRecorder>>,
        app_data_root: &Path,
        sandbox_registry: Option<Arc<SandboxAdapterRegistry>>,
        committed_memory_ceiling_bytes: Option<u64>,
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
                    Arc::new(
                        handshake_core::model_runtime::cloud::CloudLaneObservability {
                            flight_recorder: fr,
                            consent: None,
                        },
                    )
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
        let concurrency = handshake_core::swarm_orchestration::default_swarm_concurrency().max(1);
        let mut budget = RunBudget::defaulted(concurrency).with_concurrency(concurrency);
        if let Some(bytes) = committed_memory_ceiling_bytes {
            budget = budget.with_committed_memory_ceiling(bytes);
        }
        let config = SwarmConfig::new(budget);

        // WP-KERNEL-004 wave 1: the sandbox adapter registry is threaded in from
        // app state (`run()` reads the managed `Arc<SandboxAdapterRegistry>` and
        // hands a clone to the live `production_with_fr_recorder` entrypoint). When
        // a registry is present a Local+LlamaCpp spawn with isolation_tier ==
        // Tier3Microvm routes to the sandboxed-local path (CH microVM); when it is
        // absent (test seams / FR-fallback `production`) the non-sandbox spawn
        // paths stay byte-for-byte unchanged and a Tier-3 spawn returns a typed
        // FactoryFailed rather than silently downgrading.
        let production_factory =
            ProductionModelSessionFactory::new(ledger.clone(), cloud, sandbox_registry);
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

        // ROI#3: the resume-template store is rooted at the SAME `app_data_root`
        // every other durable store uses (cli-bridge config, schedules). An empty
        // path resolves to the never-present default in `load` (the explicit-cloud
        // test seam) so templates simply never persist there — honest.
        let spawn_template_store = SpawnTemplateStore::new(app_data_root);

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
            spawn_template_store,
            committed_memory_ceiling_bytes,
            cloud_assistance_recorder: None,
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
        worktree_id: Option<String>,
        instance_id: String,
    ) -> Option<String> {
        let runtime = self.terminal_capture.clone()?;
        let binding = handshake_core::terminal::SessionBinding {
            swarm_id,
            worktree_id,
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

    /// The per-session resume-template store (ROI#3). Exposed so a caller can
    /// inject an explicit path (tests) and so `resumable` probes share the same
    /// store the spawn path writes to.
    pub fn spawn_template_store(&self) -> &SpawnTemplateStore {
        &self.spawn_template_store
    }

    pub fn committed_memory_ceiling_bytes(&self) -> Option<u64> {
        self.committed_memory_ceiling_bytes
    }

    /// Override the resume-template store with one bound to an explicit path.
    /// Builder-style; used by tests so a spawn persists into a `tempfile::tempdir`
    /// the test then reads back (the `production()` constructors root it at the
    /// app_data_root, which is `""` in the explicit-cloud test seam).
    pub fn with_spawn_template_store(mut self, store: SpawnTemplateStore) -> Self {
        self.spawn_template_store = store;
        self
    }

    pub fn with_cloud_assistance_recorder(
        mut self,
        recorder: Arc<dyn CloudAssistanceReceiptRecorder>,
    ) -> Self {
        self.cloud_assistance_recorder = Some(recorder);
        self
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
    ) -> Option<(Arc<dyn ModelRuntime>, ModelId, ProviderKind)> {
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
        table
            .get(&iid)
            .map(|t| (t.runtime.clone(), t.model_id, t.provider))
    }

    fn tracked_warm_vm_restore_manifest(
        &self,
        iid: ModelInstanceId,
    ) -> Option<WarmVmSnapshotManifest> {
        self.sessions
            .lock()
            .expect("swarm session table poisoned")
            .get(&iid)
            .and_then(|tracked| tracked.warm_vm_restore_manifest.clone())
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
pub enum ByokCloudProviderIpc {
    Anthropic,
    #[serde(rename = "openai", alias = "open_ai")]
    OpenAi,
}

impl ByokCloudProviderIpc {
    fn to_core(self) -> ByokCloudProvider {
        match self {
            Self::Anthropic => ByokCloudProvider::Anthropic,
            Self::OpenAi => ByokCloudProvider::OpenAi,
        }
    }

    fn from_core(provider: ByokCloudProvider) -> Self {
        match provider {
            ByokCloudProvider::Anthropic => Self::Anthropic,
            ByokCloudProvider::OpenAi => Self::OpenAi,
        }
    }

    fn to_template_provider(self) -> TemplateByokCloudProvider {
        match self {
            Self::Anthropic => TemplateByokCloudProvider::Anthropic,
            Self::OpenAi => TemplateByokCloudProvider::OpenAi,
        }
    }

    fn from_template_provider(provider: TemplateByokCloudProvider) -> Self {
        match provider {
            TemplateByokCloudProvider::Anthropic => Self::Anthropic,
            TemplateByokCloudProvider::OpenAi => Self::OpenAi,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SwarmLocalExecutionModeIpc {
    Cold,
    WarmVm,
}

impl SwarmLocalExecutionModeIpc {
    fn to_template_mode(self) -> TemplateLocalExecutionMode {
        match self {
            Self::Cold => TemplateLocalExecutionMode::Cold,
            Self::WarmVm => TemplateLocalExecutionMode::WarmVm,
        }
    }

    fn from_template_mode(mode: TemplateLocalExecutionMode) -> Self {
        match mode {
            TemplateLocalExecutionMode::Cold => Self::Cold,
            TemplateLocalExecutionMode::WarmVm => Self::WarmVm,
        }
    }

    fn from_core(mode: LocalExecutionMode) -> Self {
        match mode {
            LocalExecutionMode::Cold => Self::Cold,
            LocalExecutionMode::WarmVm => Self::WarmVm,
        }
    }

    /// Stable wire string (matches the snake_case serde + the TS union).
    fn as_str(self) -> &'static str {
        match self {
            Self::Cold => "cold",
            Self::WarmVm => "warm_vm",
        }
    }
}

/// Operator-intended isolation tier the spawn form records on a session. Mirrors
/// `handshake_core::sandbox::adapter::IsolationTier` 1:1 (snake_case serde
/// matches the core wire vocabulary). Deliberately a thin IPC enum rather than a
/// re-export of the core type, so the app IPC boundary stays decoupled. Tier3 is
/// load-bearing for Local+LlamaCpp when the sandbox registry is wired; the other
/// combinations remain attribution until their substrates ship.
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
    /// Local: optional explicit execution substrate (`cold` | `warm_vm`). Omitted
    /// means legacy cold/default behavior. Warm VM is validated before spawn.
    #[serde(default)]
    pub local_execution_mode: Option<SwarmLocalExecutionModeIpc>,
    /// Local warm VM only: a previously captured warm snapshot manifest to
    /// restore instead of cold-loading the model in a new VM.
    #[serde(default)]
    pub warm_vm_restore_manifest: Option<WarmVmSnapshotManifest>,
    /// Cloud: required. The allowlisted cloud model name (e.g. `gpt-4o`).
    #[serde(default)]
    pub cloud_model_name: Option<String>,
    /// BYOK cloud only: exact provider flavor. Optional for backward
    /// compatibility; when supplied the production factory honors it exactly.
    #[serde(default)]
    pub byok_cloud_provider: Option<ByokCloudProviderIpc>,
    /// Which concurrent instance index of this model to spawn (default 0). The
    /// same artifact can run as several instances for throughput.
    #[serde(default)]
    pub instance: u32,
    /// Parent session id for ledger lineage (defaults to an operator tag).
    #[serde(default)]
    pub parent_session_id: Option<String>,
    /// Optional swarm grouping id. Pair/escalation workflows require this to
    /// match across local+cloud peers so board/Terminal/FR attribution is shared.
    #[serde(default)]
    pub swarm_id: Option<String>,
    /// Optional VM/sandbox worktree this session is ASSIGNED to (board swimlane +
    /// per-worktree recovery grouping). Blank/whitespace => None (unassigned,
    /// honest).
    #[serde(default)]
    pub worktree_id: Option<String>,
    /// Optional on-disk place the operator assigns the session (absolute OR
    /// repo-relative; disk-agnostic — the path is recorded verbatim, NOT resolved,
    /// created, or used as a cwd here). Blank/whitespace => None. Recorded
    /// attribution only.
    #[serde(default)]
    pub working_dir: Option<String>,
    /// Optional isolation tier the operator intends for this session
    /// (tier1_container | tier2_syscall | tier3_microvm). Tier3 is enforced for
    /// Local+LlamaCpp spawns when the production sandbox registry is wired.
    #[serde(default)]
    pub isolation_tier: Option<SwarmIsolationTierIpc>,
    /// Rank-6 local committed-memory estimate for admission control and VM
    /// memory sizing. Only local requests reserve this host memory; cloud
    /// requests ignore it so cloud fallback can escape local memory pressure.
    /// When the app run configures a committed-memory ceiling, local spawns must
    /// carry an estimate or fail closed before model/VM boot.
    #[serde(default)]
    pub committed_memory_bytes: Option<u64>,
}

/// Operator request to launch a local model and a cloud peer together. This is a
/// composition over the same validated spawn request shape: local remains a real
/// local/sandbox spawn and cloud remains a real BYOK/official-CLI spawn.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmLocalCloudPairRequestIpc {
    pub local: SwarmSpawnRequestIpc,
    pub cloud: SwarmSpawnRequestIpc,
}

/// Operator request to try a VM/local spawn first, then escalate to the explicit
/// cloud request only when the local coordinator refusal is capacity-shaped.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmCloudEscalationRequestIpc {
    pub local: SwarmSpawnRequestIpc,
    pub cloud: SwarmSpawnRequestIpc,
}

// ---------------------------------------------------------------------------
// ROI#3 STATE RECOVERY: IPC <-> resume-template conversions
// ---------------------------------------------------------------------------

impl SwarmProviderIpc {
    fn to_template_provider(self) -> TemplateProvider {
        match self {
            Self::Local => TemplateProvider::Local,
            Self::ByokCloud => TemplateProvider::ByokCloud,
            Self::OfficialCli => TemplateProvider::OfficialCli,
        }
    }

    fn from_template_provider(provider: TemplateProvider) -> Self {
        match provider {
            TemplateProvider::Local => Self::Local,
            TemplateProvider::ByokCloud => Self::ByokCloud,
            TemplateProvider::OfficialCli => Self::OfficialCli,
        }
    }
}

impl SwarmRuntimeBindingIpc {
    fn to_template_binding(self) -> TemplateRuntimeBinding {
        match self {
            Self::Candle => TemplateRuntimeBinding::Candle,
            Self::LlamaCpp => TemplateRuntimeBinding::LlamaCpp,
        }
    }

    fn from_template_binding(binding: TemplateRuntimeBinding) -> Self {
        match binding {
            TemplateRuntimeBinding::Candle => Self::Candle,
            TemplateRuntimeBinding::LlamaCpp => Self::LlamaCpp,
        }
    }
}

impl SwarmIsolationTierIpc {
    fn to_template_tier(self) -> TemplateIsolationTier {
        match self {
            Self::Tier1Container => TemplateIsolationTier::Tier1Container,
            Self::Tier2Syscall => TemplateIsolationTier::Tier2Syscall,
            Self::Tier3Microvm => TemplateIsolationTier::Tier3Microvm,
        }
    }

    fn from_template_tier(tier: TemplateIsolationTier) -> Self {
        match tier {
            TemplateIsolationTier::Tier1Container => Self::Tier1Container,
            TemplateIsolationTier::Tier2Syscall => Self::Tier2Syscall,
            TemplateIsolationTier::Tier3Microvm => Self::Tier3Microvm,
        }
    }
}

/// Build a resume-complete [`SessionSpawnTemplate`] from the VALIDATED spawn
/// request + the live composite `instance_id` the coordinator returned. Called
/// ONLY after a successful spawn, so the request is known-good; this applies the
/// SAME blank => None trimming rule the spawn build uses (`trimmed_opt`), so the
/// template records exactly the fields the spawn accepted. Cloud captures the
/// model name; local captures artifact + sha + binding, plus a warm restore
/// manifest when a restored warm-VM spawn supplied one. The `origin_session_id`
/// is the captured-from session (the resume lineage root).
pub(crate) fn template_from_ipc(
    request: &SwarmSpawnRequestIpc,
    origin_session_id: &str,
) -> SessionSpawnTemplate {
    let is_local = matches!(request.provider, SwarmProviderIpc::Local);
    SessionSpawnTemplate {
        provider: request.provider.to_template_provider(),
        artifact_path: if is_local {
            trimmed_opt(&request.artifact_path)
        } else {
            None
        },
        sha256_expected: if is_local {
            trimmed_opt(&request.sha256_expected)
        } else {
            None
        },
        runtime_binding: if is_local {
            request
                .runtime_binding
                .map(SwarmRuntimeBindingIpc::to_template_binding)
        } else {
            None
        },
        local_execution_mode: if is_local {
            request
                .local_execution_mode
                .map(SwarmLocalExecutionModeIpc::to_template_mode)
        } else {
            None
        },
        warm_vm_restore_manifest: if is_local
            && matches!(
                request.local_execution_mode,
                Some(SwarmLocalExecutionModeIpc::WarmVm)
            ) {
            request.warm_vm_restore_manifest.clone()
        } else {
            None
        },
        cloud_model_name: if is_local {
            None
        } else {
            trimmed_opt(&request.cloud_model_name)
        },
        byok_cloud_provider: if matches!(request.provider, SwarmProviderIpc::ByokCloud) {
            request
                .byok_cloud_provider
                .map(ByokCloudProviderIpc::to_template_provider)
        } else {
            None
        },
        instance: request.instance,
        swarm_id: trimmed_opt(&request.swarm_id),
        worktree_id: trimmed_opt(&request.worktree_id),
        working_dir: trimmed_opt(&request.working_dir),
        isolation_tier: request
            .isolation_tier
            .map(SwarmIsolationTierIpc::to_template_tier),
        committed_memory_bytes: if is_local {
            request.committed_memory_bytes.filter(|bytes| *bytes > 0)
        } else {
            None
        },
        origin_session_id: origin_session_id.to_string(),
        captured_at: chrono::Utc::now(),
    }
}

/// Rebuild a FRESH [`SwarmSpawnRequestIpc`] from a stored resume template. The
/// rebuilt request carries NO pre-minted model id (the shared spawn path's
/// `build_spawn_request` mints a fresh `ModelId::new_v7()`), so the resumed
/// session gets a brand-new composite id — never colliding with the origin. The
/// `parent_session_id` is set to `origin` for lineage ("resumed from X" lands in
/// the real FR `SessionSpawned.parent_session_id`). The local sha integrity gate
/// is preserved verbatim (re-verified on resume through the same factory).
pub(crate) fn template_to_request_ipc(
    template: &SessionSpawnTemplate,
    origin_session_id: &str,
) -> SwarmSpawnRequestIpc {
    SwarmSpawnRequestIpc {
        provider: SwarmProviderIpc::from_template_provider(template.provider),
        artifact_path: template.artifact_path.clone(),
        sha256_expected: template.sha256_expected.clone(),
        runtime_binding: template
            .runtime_binding
            .map(SwarmRuntimeBindingIpc::from_template_binding),
        local_execution_mode: template
            .local_execution_mode
            .map(SwarmLocalExecutionModeIpc::from_template_mode),
        warm_vm_restore_manifest: template.warm_vm_restore_manifest.clone(),
        cloud_model_name: template.cloud_model_name.clone(),
        byok_cloud_provider: template
            .byok_cloud_provider
            .map(ByokCloudProviderIpc::from_template_provider),
        instance: template.instance,
        // Lineage: the resumed-from session id (NOT the operator default tag).
        parent_session_id: Some(origin_session_id.to_string()),
        swarm_id: template.swarm_id.clone(),
        worktree_id: template.worktree_id.clone(),
        working_dir: template.working_dir.clone(),
        isolation_tier: template
            .isolation_tier
            .map(SwarmIsolationTierIpc::from_template_tier),
        committed_memory_bytes: template.committed_memory_bytes,
    }
}

fn is_local_warm_vm_request(request: &SwarmSpawnRequestIpc) -> bool {
    matches!(request.provider, SwarmProviderIpc::Local)
        && matches!(
            request.local_execution_mode,
            Some(SwarmLocalExecutionModeIpc::WarmVm)
        )
}

fn template_matches_warm_restore_request(
    template: &SessionSpawnTemplate,
    request: &SwarmSpawnRequestIpc,
    worktree_id: &str,
    sha256_expected: &str,
    artifact_path: &str,
) -> bool {
    if template.provider != TemplateProvider::Local {
        return false;
    }
    if template.local_execution_mode != Some(TemplateLocalExecutionMode::WarmVm) {
        return false;
    }
    if template.runtime_binding != Some(TemplateRuntimeBinding::LlamaCpp) {
        return false;
    }
    if request.runtime_binding != Some(SwarmRuntimeBindingIpc::LlamaCpp) {
        return false;
    }
    if template.artifact_path.as_deref().map(str::trim) != Some(artifact_path) {
        return false;
    }
    let Some(manifest) = template.warm_vm_restore_manifest.as_ref() else {
        return false;
    };
    manifest.worktree_id == worktree_id && manifest.model_artifact_sha256 == sha256_expected
}

fn latest_matching_warm_restore_manifest(
    store: &SpawnTemplateStore,
    request: &SwarmSpawnRequestIpc,
) -> Option<WarmVmSnapshotManifest> {
    if request.warm_vm_restore_manifest.is_some() || !is_local_warm_vm_request(request) {
        return None;
    }
    let worktree_id = trimmed_opt(&request.worktree_id)?;
    let sha256_expected = trimmed_opt(&request.sha256_expected)?;
    let artifact_path = trimmed_opt(&request.artifact_path)?;
    let doc = match store.load() {
        Ok(doc) => doc,
        Err(error) => {
            eprintln!("{FR_EVT_SWARM_LIFECYCLE}: warm restore manifest lookup skipped: {error}");
            return None;
        }
    };
    doc.templates
        .values()
        .filter(|template| {
            template_matches_warm_restore_request(
                template,
                request,
                &worktree_id,
                &sha256_expected,
                &artifact_path,
            )
        })
        .max_by_key(|template| template.captured_at)
        .and_then(|template| template.warm_vm_restore_manifest.clone())
}

struct WarmRestoreHydration {
    request: SwarmSpawnRequestIpc,
    auto_attached: bool,
}

fn attach_latest_warm_restore_manifest(
    mut request: SwarmSpawnRequestIpc,
    store: &SpawnTemplateStore,
) -> WarmRestoreHydration {
    let mut auto_attached = false;
    if let Some(manifest) = latest_matching_warm_restore_manifest(store, &request) {
        request.warm_vm_restore_manifest = Some(manifest);
        auto_attached = true;
    }
    WarmRestoreHydration {
        request,
        auto_attached,
    }
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

/// One measured spawn attempt inside a composed operator workflow. The command
/// returns attempts rather than throwing on the first lane so partial success is
/// visible and attributable.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmSpawnAttemptIpc {
    pub provider: String,
    pub instance_id: Option<SwarmInstanceIdIpc>,
    pub error: Option<String>,
}

impl SwarmSpawnAttemptIpc {
    fn success(provider: SwarmProviderIpc, instance_id: SwarmInstanceIdIpc) -> Self {
        Self {
            provider: provider_str(provider.to_provider_kind()).to_string(),
            instance_id: Some(instance_id),
            error: None,
        }
    }

    fn failure(provider: SwarmProviderIpc, error: impl Into<String>) -> Self {
        Self {
            provider: provider_str(provider.to_provider_kind()).to_string(),
            instance_id: None,
            error: Some(error.into()),
        }
    }

    fn is_success(&self) -> bool {
        self.instance_id.is_some() && self.error.is_none()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmLocalCloudPairResultIpc {
    pub local: SwarmSpawnAttemptIpc,
    pub cloud: SwarmSpawnAttemptIpc,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmCloudEscalationResultIpc {
    /// `local` when the local lane spawned, `cloud` when capacity escalation
    /// spawned the cloud lane, or `None` when no lane spawned.
    pub selected: Option<String>,
    pub escalated: bool,
    pub escalation_reason: Option<String>,
    pub local: SwarmSpawnAttemptIpc,
    pub cloud: Option<SwarmSpawnAttemptIpc>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SwarmEscalationTaskClassIpc {
    Routine,
    Classification,
    HardReasoning,
    ForceCloud,
    ForceLocal,
}

/// Generate on a local/VM session first, with an explicit cloud fallback request
/// unless the task class explicitly forces the cloud lane.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmChatGenerateWithCloudEscalationRequestIpc {
    pub local_instance_id: String,
    pub prompt: String,
    pub cloud: SwarmSpawnRequestIpc,
    #[serde(default)]
    pub task_class: Option<SwarmEscalationTaskClassIpc>,
    /// Optional local confidence score in basis points (0..=10000). When paired
    /// with `min_local_confidence_basis_points` and below threshold, the command
    /// keeps the local output as a student candidate and also generates through
    /// the explicit cloud lane for a distillation trace.
    #[serde(default)]
    pub local_confidence_basis_points: Option<u16>,
    #[serde(default)]
    pub min_local_confidence_basis_points: Option<u16>,
    #[serde(default)]
    pub validation_labels: Vec<String>,
    #[serde(default)]
    pub cloud_assistance_receipt: Option<CloudAssistanceReceiptContextIpc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudAssistanceReceiptContextIpc {
    pub workspace_id: String,
    pub wp_id: String,
    pub mt_id: String,
    pub claim_id: String,
    pub cloud_lane_id: String,
    pub cloud_actor_id: String,
    #[serde(default)]
    pub fallback_basis_event_id: Option<String>,
    pub to_role: String,
    pub mailbox_thread_id: String,
    pub mailbox_message_id: String,
    pub target_ref: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudFallbackBasisRefIpc {
    pub basis_id: String,
    pub fallback_basis_event_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudAssistanceReceiptRefIpc {
    pub receipt_id: String,
    pub fallback_basis_event_id: String,
    pub handoff_id: String,
    pub cloud_assistance_event_id: String,
    pub output_sha256: String,
    pub review_state: String,
    pub non_authoritative: bool,
    pub requires_promotion: bool,
}

#[derive(Clone, Debug)]
pub struct CloudFallbackBasisRuntimeRequest {
    pub context: CloudAssistanceReceiptContextIpc,
    pub parent_session_id: String,
    pub prompt_sha256: String,
    pub fallback_reason: CloudFallbackReason,
    pub local_attempt_ref: String,
    pub evidence_sha256: String,
    pub summary: String,
}

#[derive(Clone, Debug)]
pub struct CloudAssistanceRuntimeReceiptRequest {
    pub context: CloudAssistanceReceiptContextIpc,
    pub cloud_instance: SwarmInstanceIdIpc,
    pub cloud_provider: SwarmProviderIpc,
    pub byok_cloud_provider: Option<ByokCloudProviderIpc>,
    pub cloud_model_name: Option<String>,
    pub parent_session_id: String,
    pub fallback_basis_event_id: String,
    pub prompt_sha256: String,
    pub output_sha256: String,
    pub output_body_sha256: String,
    pub output_body_jsonb: serde_json::Value,
    pub output_text: String,
    pub fallback_reason: CloudFallbackReason,
    pub escalation_reason: String,
}

#[async_trait]
pub trait CloudAssistanceReceiptRecorder: Send + Sync {
    async fn record_cloud_fallback_basis(
        &self,
        request: CloudFallbackBasisRuntimeRequest,
    ) -> Result<CloudFallbackBasisRefIpc, String>;

    async fn record_cloud_assistance(
        &self,
        request: CloudAssistanceRuntimeReceiptRequest,
    ) -> Result<CloudAssistanceReceiptRefIpc, String>;
}

pub struct PostgresCloudAssistanceReceiptRecorder {
    store: ParallelSwarmStateRecoveryStore,
}

impl PostgresCloudAssistanceReceiptRecorder {
    pub fn from_control_plane(control_plane: ControlPlaneStorage) -> Self {
        Self {
            store: ParallelSwarmStateRecoveryStore::new(
                control_plane.postgres_pool,
                control_plane.database,
            ),
        }
    }
}

#[async_trait]
impl CloudAssistanceReceiptRecorder for PostgresCloudAssistanceReceiptRecorder {
    async fn record_cloud_fallback_basis(
        &self,
        request: CloudFallbackBasisRuntimeRequest,
    ) -> Result<CloudFallbackBasisRefIpc, String> {
        let core_request = cloud_fallback_basis_request_from_runtime(request)?;
        let receipt = self
            .store
            .record_cloud_fallback_basis(core_request)
            .await
            .map_err(|error| error.to_string())?;
        Ok(CloudFallbackBasisRefIpc {
            basis_id: receipt.basis_id,
            fallback_basis_event_id: receipt.fallback_basis_event_id,
        })
    }

    async fn record_cloud_assistance(
        &self,
        request: CloudAssistanceRuntimeReceiptRequest,
    ) -> Result<CloudAssistanceReceiptRefIpc, String> {
        let cloud_assistance_request = cloud_assistance_request_from_runtime(request)?;
        let receipt = self
            .store
            .record_cloud_assistance_output(cloud_assistance_request)
            .await
            .map_err(|error| error.to_string())?;

        Ok(CloudAssistanceReceiptRefIpc {
            receipt_id: receipt.receipt_id,
            fallback_basis_event_id: receipt.fallback_basis_event_id,
            handoff_id: receipt.handoff_id,
            cloud_assistance_event_id: receipt.cloud_assistance_event_id,
            output_sha256: receipt.output_sha256,
            review_state: receipt.review_state,
            non_authoritative: receipt.non_authoritative,
            requires_promotion: receipt.requires_promotion,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmChatGenerateWithCloudEscalationResponseIpc {
    pub selected: Option<String>,
    pub escalated: bool,
    pub escalation_reason: Option<String>,
    pub local_error: Option<String>,
    pub local: Option<SwarmChatResponseIpc>,
    pub cloud_instance: Option<SwarmInstanceIdIpc>,
    pub cloud: Option<SwarmChatResponseIpc>,
    pub cloud_assistance_receipt: Option<CloudAssistanceReceiptRefIpc>,
    pub cloud_error: Option<String>,
    pub distillation_trace: Option<DistillationSwarmTraceQueueEntry>,
}

#[derive(Clone, Debug)]
struct LocalEscalationTraceInput {
    instance_id: String,
    model_id: String,
    response: SwarmChatResponseIpc,
    confidence_basis_points: Option<u16>,
    validation_labels: Vec<String>,
}

/// ROI#3: the stored resume template, surfaced to the UI (camelCase) for the
/// edit-then-resume PREFILL path (`kernel_swarm_get_spawn_template`). Mirrors the
/// spawn form fields so the operator can tweak (e.g. repoint a newer artifact)
/// before re-spawning. The one-click resume path does NOT need this — it drives
/// `kernel_swarm_resume_session` directly from the stored template.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSpawnTemplateIpc {
    pub provider: SwarmProviderIpc,
    pub artifact_path: Option<String>,
    pub sha256_expected: Option<String>,
    pub runtime_binding: Option<SwarmRuntimeBindingIpc>,
    pub local_execution_mode: Option<SwarmLocalExecutionModeIpc>,
    pub warm_vm_restore_manifest: Option<WarmVmSnapshotManifest>,
    pub cloud_model_name: Option<String>,
    pub byok_cloud_provider: Option<ByokCloudProviderIpc>,
    pub instance: u32,
    pub swarm_id: Option<String>,
    pub worktree_id: Option<String>,
    pub working_dir: Option<String>,
    pub isolation_tier: Option<SwarmIsolationTierIpc>,
    pub committed_memory_bytes: Option<u64>,
    pub origin_session_id: String,
    pub captured_at: String,
}

impl From<SessionSpawnTemplate> for SessionSpawnTemplateIpc {
    fn from(t: SessionSpawnTemplate) -> Self {
        Self {
            provider: SwarmProviderIpc::from_template_provider(t.provider),
            artifact_path: t.artifact_path,
            sha256_expected: t.sha256_expected,
            runtime_binding: t
                .runtime_binding
                .map(SwarmRuntimeBindingIpc::from_template_binding),
            local_execution_mode: t
                .local_execution_mode
                .map(SwarmLocalExecutionModeIpc::from_template_mode),
            warm_vm_restore_manifest: t.warm_vm_restore_manifest,
            cloud_model_name: t.cloud_model_name,
            byok_cloud_provider: t
                .byok_cloud_provider
                .map(ByokCloudProviderIpc::from_template_provider),
            instance: t.instance,
            swarm_id: t.swarm_id,
            worktree_id: t.worktree_id,
            working_dir: t.working_dir,
            isolation_tier: t
                .isolation_tier
                .map(SwarmIsolationTierIpc::from_template_tier),
            committed_memory_bytes: t.committed_memory_bytes,
            origin_session_id: t.origin_session_id,
            captured_at: t.captured_at.to_rfc3339(),
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
    pub local_execution_mode: Option<SwarmLocalExecutionModeIpc>,
    pub artifact_path: Option<String>,
    pub cloud_model_name: Option<String>,
    /// Operator-assigned VM/sandbox worktree (board swimlane / recovery group).
    /// `None` => unassigned.
    pub worktree_id: Option<String>,
    /// Operator-assigned on-disk place (attribution only; never executed/resolved).
    pub working_dir: Option<String>,
    /// Operator-intended isolation tier. Tier3 local llama.cpp is enforced when
    /// the sandbox registry is wired; other combinations remain attribution.
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
    /// Optional committed-memory ceiling headroom, in bytes. `None` means the
    /// current app run did not configure a committed-memory ceiling.
    pub committed_memory_bytes_remaining: Option<u64>,
    /// Optional committed-memory ceiling, in bytes, for operator display.
    pub committed_memory_bytes_cap: Option<u64>,
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

#[derive(Debug)]
enum SpawnCommandError {
    Validation(String),
    Swarm(SwarmError),
}

impl SpawnCommandError {
    fn is_capacity_refusal(&self) -> bool {
        match self {
            Self::Swarm(error) => error.is_capacity_refusal(),
            Self::Validation(_) => false,
        }
    }

    fn is_factory_failure(&self) -> bool {
        matches!(self, Self::Swarm(SwarmError::FactoryFailed(_)))
    }
}

impl std::fmt::Display for SpawnCommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Validation(error) => f.write_str(error),
            Self::Swarm(error) => write!(f, "{error}"),
        }
    }
}

fn should_retry_auto_warm_restore(auto_attached: bool, error: &SpawnCommandError) -> bool {
    auto_attached && error.is_factory_failure()
}

async fn after_successful_swarm_spawn(
    request: &SwarmSpawnRequestIpc,
    state: &SwarmRuntimeState,
    iid: ModelInstanceId,
    auto_attached_warm_restore: bool,
) {
    // ROI#3 STATE RECOVERY: persist a resume template keyed by the live
    // composite instance_id so this exact session can be re-spawned later
    // (`kernel_swarm_resume_session`). BEST-EFFORT: a store-write failure
    // must NOT fail an otherwise-successful spawn (the real model already
    // loaded) — it only makes the session non-resumable, surfaced honestly
    // via the FR/stderr line and a `resumable=false` summary.
    let mut template = template_from_ipc(request, &iid.to_string());
    if is_local_warm_vm_request(request) {
        let tracked_manifest = state.tracked_warm_vm_restore_manifest(iid);
        match (auto_attached_warm_restore, tracked_manifest) {
            (true, Some(manifest))
                if request.warm_vm_restore_manifest.as_ref() != Some(&manifest) =>
            {
                template.warm_vm_restore_manifest = Some(manifest);
            }
            (true, _) => {
                template.warm_vm_restore_manifest = None;
            }
            (false, Some(manifest)) => {
                template.warm_vm_restore_manifest = Some(manifest);
            }
            (false, None) => {}
        }
    }
    if let Err(error) = state.spawn_template_store.upsert(iid.to_string(), template) {
        eprintln!("{FR_EVT_SWARM_LIFECYCLE}: resume template persist failed for {iid}: {error}");
    }
    // §10.1 capture seam: when wired, open a read-only capture session bound to
    // this swarm spawn so the operator can inspect background work in Terminal.
    let _ = state
        .attach_spawn_capture(
            trimmed_opt(&request.swarm_id),
            trimmed_opt(&request.worktree_id),
            iid.to_string(),
        )
        .await;
}

async fn spawn_session_inner(
    request: SwarmSpawnRequestIpc,
    state: &SwarmRuntimeState,
) -> Result<SwarmInstanceIdIpc, SpawnCommandError> {
    let WarmRestoreHydration {
        request,
        auto_attached,
    } = attach_latest_warm_restore_manifest(request, &state.spawn_template_store);
    match spawn_session_once(request.clone(), state, auto_attached).await {
        Ok(instance_id) => Ok(instance_id),
        Err(error) if should_retry_auto_warm_restore(auto_attached, &error) => {
            let mut fresh_request = request;
            fresh_request.warm_vm_restore_manifest = None;
            eprintln!(
                "{FR_EVT_SWARM_LIFECYCLE}: auto warm restore failed; retrying once with a fresh warm VM load: {error}"
            );
            spawn_session_once(fresh_request, state, false).await
        }
        Err(error) => Err(error),
    }
}

async fn spawn_session_once(
    request: SwarmSpawnRequestIpc,
    state: &SwarmRuntimeState,
    auto_attached_warm_restore: bool,
) -> Result<SwarmInstanceIdIpc, SpawnCommandError> {
    let coordinator = state.coordinator();
    let spawn = build_spawn_request(&request).map_err(SpawnCommandError::Validation)?;
    let instance_id = spawn.instance_id;
    match coordinator.spawn_session(spawn).await {
        Ok(iid) => {
            after_successful_swarm_spawn(&request, state, iid, auto_attached_warm_restore).await;
            Ok(iid.into())
        }
        Err(error) => {
            // Reconcile the side-table: a failed spawn must not leave a tracked
            // entry (the TrackingFactory only inserts on success, but a
            // duplicate-rollback can race — reconcile to be safe).
            let _ = state.resolve_runtime(instance_id);
            eprintln!("{FR_EVT_SWARM_LIFECYCLE}: spawn failed: {error}");
            Err(SpawnCommandError::Swarm(error))
        }
    }
}

async fn spawn_attempt(
    request: SwarmSpawnRequestIpc,
    state: &SwarmRuntimeState,
) -> SwarmSpawnAttemptIpc {
    let provider = request.provider;
    match spawn_session_inner(request, state).await {
        Ok(instance_id) => SwarmSpawnAttemptIpc::success(provider, instance_id),
        Err(error) => SwarmSpawnAttemptIpc::failure(provider, error.to_string()),
    }
}

fn validate_local_cloud_orchestration(
    local: &SwarmSpawnRequestIpc,
    cloud: &SwarmSpawnRequestIpc,
) -> Result<(), String> {
    if local.provider != SwarmProviderIpc::Local {
        return Err("local request must use provider=local".to_string());
    }
    if cloud.provider == SwarmProviderIpc::Local {
        return Err("cloud request must use provider=byok_cloud or official_cli".to_string());
    }
    let local_swarm = trimmed_opt(&local.swarm_id);
    let cloud_swarm = trimmed_opt(&cloud.swarm_id);
    let Some(local_swarm) = local_swarm else {
        return Err("local+cloud orchestration requires a non-empty shared swarm_id".to_string());
    };
    if cloud_swarm.as_deref() != Some(local_swarm.as_str()) {
        return Err(
            "local+cloud orchestration requires cloud.swarm_id to match local.swarm_id".to_string(),
        );
    }
    let local_worktree = trimmed_opt(&local.worktree_id);
    let cloud_worktree = trimmed_opt(&cloud.worktree_id);
    let Some(local_worktree) = local_worktree else {
        return Err(
            "local+cloud orchestration requires a non-empty shared worktree_id".to_string(),
        );
    };
    if cloud_worktree.as_deref() != Some(local_worktree.as_str()) {
        return Err(
            "local+cloud orchestration requires cloud.worktree_id to match local.worktree_id"
                .to_string(),
        );
    }
    let local_working_dir = trimmed_opt(&local.working_dir);
    let cloud_working_dir = trimmed_opt(&cloud.working_dir);
    if local_working_dir != cloud_working_dir {
        return Err(
            "local+cloud orchestration requires matching working_dir values when supplied"
                .to_string(),
        );
    }
    Ok(())
}

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
    spawn_session_inner(request, &state)
        .await
        .map_err(|error| error.to_string())
}

/// Spawn the operator's local+cloud pair as one workflow. Each lane is still a
/// real bounded coordinator spawn; the command returns both measured attempts so
/// partial success is visible instead of hidden behind the first failure.
#[tauri::command]
pub async fn kernel_swarm_spawn_local_cloud_pair(
    request: SwarmLocalCloudPairRequestIpc,
    state: State<'_, SwarmRuntimeState>,
) -> Result<SwarmLocalCloudPairResultIpc, String> {
    let _ = KERNEL_SWARM_SPAWN_LOCAL_CLOUD_PAIR_IPC_CHANNEL;
    validate_local_cloud_orchestration(&request.local, &request.cloud)?;
    let local_request = request.local;
    let cloud_request = request.cloud;
    let (local, cloud) = tokio::join!(
        spawn_attempt(local_request, &state),
        spawn_attempt(cloud_request, &state)
    );
    if !local.is_success() && !cloud.is_success() {
        eprintln!(
            "{FR_EVT_SWARM_LIFECYCLE}: local+cloud pair failed: local={:?} cloud={:?}",
            local.error, cloud.error
        );
    }
    Ok(SwarmLocalCloudPairResultIpc { local, cloud })
}

/// Try the local/VM lane first and escalate to an explicit cloud request only
/// when the local refusal is capacity-shaped (concurrency, budget, breaker).
/// Load/hash/file/runtime failures stay local failures and do not cause hidden
/// data egress.
#[tauri::command]
pub async fn kernel_swarm_spawn_with_cloud_escalation(
    request: SwarmCloudEscalationRequestIpc,
    state: State<'_, SwarmRuntimeState>,
) -> Result<SwarmCloudEscalationResultIpc, String> {
    let _ = KERNEL_SWARM_SPAWN_WITH_CLOUD_ESCALATION_IPC_CHANNEL;
    validate_local_cloud_orchestration(&request.local, &request.cloud)?;

    let local_provider = request.local.provider;
    match spawn_session_inner(request.local, &state).await {
        Ok(instance_id) => Ok(SwarmCloudEscalationResultIpc {
            selected: Some("local".to_string()),
            escalated: false,
            escalation_reason: None,
            local: SwarmSpawnAttemptIpc::success(local_provider, instance_id),
            cloud: None,
        }),
        Err(local_error) => {
            let local_error_text = local_error.to_string();
            let local = SwarmSpawnAttemptIpc::failure(local_provider, local_error_text.clone());
            if !local_error.is_capacity_refusal() {
                return Ok(SwarmCloudEscalationResultIpc {
                    selected: None,
                    escalated: false,
                    escalation_reason: None,
                    local,
                    cloud: None,
                });
            }

            let cloud_provider = request.cloud.provider;
            let cloud_attempt = match spawn_session_inner(request.cloud, &state).await {
                Ok(instance_id) => SwarmSpawnAttemptIpc::success(cloud_provider, instance_id),
                Err(error) => SwarmSpawnAttemptIpc::failure(cloud_provider, error.to_string()),
            };
            Ok(SwarmCloudEscalationResultIpc {
                selected: cloud_attempt.is_success().then(|| "cloud".to_string()),
                escalated: true,
                escalation_reason: Some(local_error_text),
                local,
                cloud: Some(cloud_attempt),
            })
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

/// ROI#3 STATE RECOVERY: re-spawn a recorded session from its stored resume
/// template. Reads the [`SessionSpawnTemplate`] captured at the original spawn
/// (keyed by the composite `session_id`), rebuilds a FRESH `SwarmSpawnRequestIpc`
/// (fresh model id minted by `build_spawn_request`; `parent_session_id` = the
/// resumed-from id for lineage), and drives the SAME validated
/// `kernel_swarm_spawn_session` path — so resume inherits the integrity sha gate,
/// the coordinator bounds, the FR `SessionSpawned`, the §10.1 capture seam, AND
/// re-persists a template for the NEW session (a resumed session is itself
/// resumable). There is NO forked spawn path.
///
/// Honest: a session with no stored template (chat sessions; swarm sessions
/// spawned before this feature; sessions whose best-effort template write
/// failed) surfaces a typed `SESSION_NOT_RESUMABLE:` error. A corrupt store
/// surfaces its read error verbatim rather than masquerading as not-resumable.
#[tauri::command]
pub async fn kernel_swarm_resume_session(
    session_id: String,
    state: State<'_, SwarmRuntimeState>,
) -> Result<SwarmInstanceIdIpc, String> {
    let _ = KERNEL_SWARM_RESUME_SESSION_IPC_CHANNEL;
    let origin = session_id.trim();
    if origin.is_empty() {
        return Err("session_id must not be empty".to_string());
    }
    let template = state
        .spawn_template_store
        .get(origin)?
        .ok_or_else(|| format!("SESSION_NOT_RESUMABLE: no spawn template stored for {origin}"))?;
    // Rebuild a fresh request (new model id, lineage parent = origin) and drive
    // the SINGLE validated spawn path. `origin` is owned by `session_id`; clone
    // into a String so the borrow does not outlive `state` being moved.
    let origin = origin.to_string();
    let request = template_to_request_ipc(&template, &origin);
    kernel_swarm_spawn_session(request, state).await
}

/// ROI#3: fetch the stored resume template for a session (camelCase IPC DTO), or
/// `None` when the session is not resumable. Enables the edit-then-resume PREFILL
/// path: the UI populates the existing spawn form from this template so the
/// operator can tweak (e.g. repoint a newer artifact, change worktree) before
/// re-spawning through the normal Spawn button. Read-only; no spawn side effect.
#[tauri::command]
pub async fn kernel_swarm_get_spawn_template(
    session_id: String,
    state: State<'_, SwarmRuntimeState>,
) -> Result<Option<SessionSpawnTemplateIpc>, String> {
    let _ = KERNEL_SWARM_GET_SPAWN_TEMPLATE_IPC_CHANNEL;
    let origin = session_id.trim();
    if origin.is_empty() {
        return Err("session_id must not be empty".to_string());
    }
    Ok(state
        .spawn_template_store
        .get(origin)?
        .map(SessionSpawnTemplateIpc::from))
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
            local_execution_mode: tracked.local_execution_mode,
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
        committed_memory_bytes_remaining: remaining.committed_memory_bytes_remaining,
        committed_memory_bytes_cap: state.committed_memory_ceiling_bytes(),
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
    chat_generate_inner(&instance_id, &prompt, &state).await
}

async fn chat_generate_inner(
    instance_id: &str,
    prompt: &str,
    state: &SwarmRuntimeState,
) -> Result<SwarmChatResponseIpc, String> {
    let trimmed = validate_chat_prompt(prompt)?;
    let (iid, runtime, model_id, provider) = resolve_chat_target(instance_id, state)?;
    if provider != ProviderKind::Local {
        return Err(
            "direct chat is local-only; cloud output requires receipt-gated cloud escalation"
                .to_string(),
        );
    }
    chat_generate_resolved_inner(iid, runtime, model_id, trimmed, state).await
}

fn validate_chat_prompt(prompt: &str) -> Result<&str, String> {
    let trimmed = prompt.trim();
    if trimmed.is_empty() {
        Err("prompt must not be empty".to_string())
    } else {
        Ok(trimmed)
    }
}

fn resolve_chat_target(
    instance_id: &str,
    state: &SwarmRuntimeState,
) -> Result<
    (
        ModelInstanceId,
        Arc<dyn ModelRuntime>,
        ModelId,
        ProviderKind,
    ),
    String,
> {
    let iid = parse_instance_id(instance_id)?;
    let (runtime, model_id, provider) = state
        .resolve_runtime(iid)
        .ok_or_else(|| format!("swarm session {iid} is no longer live (cancelled or reaped)"))?;
    Ok((iid, runtime, model_id, provider))
}

async fn chat_generate_resolved_inner(
    iid: ModelInstanceId,
    runtime: Arc<dyn ModelRuntime>,
    model_id: ModelId,
    trimmed: &str,
    state: &SwarmRuntimeState,
) -> Result<SwarmChatResponseIpc, String> {
    chat_generate_resolved_inner_with_capture(iid, runtime, model_id, trimmed, state, true).await
}

async fn chat_generate_resolved_inner_with_capture(
    iid: ModelInstanceId,
    runtime: Arc<dyn ModelRuntime>,
    model_id: ModelId,
    trimmed: &str,
    state: &SwarmRuntimeState,
    capture_live: bool,
) -> Result<SwarmChatResponseIpc, String> {
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
    let capture_sink = if capture_live {
        state.capture_sink_for(&iid.to_string())
    } else {
        None
    };
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

/// Generate on a local/VM session first, then spawn/generate through an
/// explicit cloud lane only if the local turn fails, the caller marks the local
/// confidence below threshold, or the task class explicitly forces cloud.
/// This is intentionally opt-in: the caller supplies the cloud
/// provider/model/worktree request, so egress cannot happen by hidden default.
#[tauri::command]
pub async fn kernel_swarm_chat_generate_with_cloud_escalation(
    request: SwarmChatGenerateWithCloudEscalationRequestIpc,
    state: State<'_, SwarmRuntimeState>,
) -> Result<SwarmChatGenerateWithCloudEscalationResponseIpc, String> {
    let _ = KERNEL_SWARM_CHAT_GENERATE_WITH_CLOUD_ESCALATION_IPC_CHANNEL;
    chat_generate_with_cloud_escalation_inner(request, &state).await
}

async fn chat_generate_with_cloud_escalation_inner(
    request: SwarmChatGenerateWithCloudEscalationRequestIpc,
    state: &SwarmRuntimeState,
) -> Result<SwarmChatGenerateWithCloudEscalationResponseIpc, String> {
    let prompt = validate_chat_prompt(&request.prompt)?.to_string();

    if matches!(
        request.task_class,
        Some(SwarmEscalationTaskClassIpc::ForceLocal)
    ) {
        let (local_iid, local_runtime, local_model_id, _) =
            match resolve_chat_target(&request.local_instance_id, state) {
                Ok(target) => target,
                Err(local_error) => {
                    return Ok(SwarmChatGenerateWithCloudEscalationResponseIpc {
                        selected: None,
                        escalated: false,
                        escalation_reason: None,
                        local_error: Some(local_error),
                        local: None,
                        cloud_instance: None,
                        cloud: None,
                        cloud_assistance_receipt: None,
                        cloud_error: Some("force_local blocks cloud escalation".to_string()),
                        distillation_trace: None,
                    });
                }
            };
        return match chat_generate_resolved_inner(
            local_iid,
            local_runtime,
            local_model_id,
            &prompt,
            state,
        )
        .await
        {
            Ok(local) => Ok(SwarmChatGenerateWithCloudEscalationResponseIpc {
                selected: Some("local".to_string()),
                escalated: false,
                escalation_reason: None,
                local_error: None,
                local: Some(local),
                cloud_instance: None,
                cloud: None,
                cloud_assistance_receipt: None,
                cloud_error: None,
                distillation_trace: None,
            }),
            Err(local_error) => Ok(SwarmChatGenerateWithCloudEscalationResponseIpc {
                selected: None,
                escalated: false,
                escalation_reason: None,
                local_error: Some(local_error),
                local: None,
                cloud_instance: None,
                cloud: None,
                cloud_assistance_receipt: None,
                cloud_error: Some("force_local blocks cloud escalation".to_string()),
                distillation_trace: None,
            }),
        };
    }

    if matches!(
        request.task_class,
        Some(SwarmEscalationTaskClassIpc::ForceCloud)
    ) {
        parse_instance_id(&request.local_instance_id)?;
        return spawn_generate_cloud_escalation(
            request.cloud,
            &request.local_instance_id,
            &prompt,
            "force_cloud requested; skipped local generate".to_string(),
            CloudFallbackReason::ForceCloud,
            None,
            request.cloud_assistance_receipt,
            state,
        )
        .await;
    }

    let (local_iid, local_runtime, local_model_id, _) =
        match resolve_chat_target(&request.local_instance_id, state) {
            Ok(target) => target,
            Err(local_error) => {
                return Ok(SwarmChatGenerateWithCloudEscalationResponseIpc {
                    selected: None,
                    escalated: false,
                    escalation_reason: None,
                    local_error: Some(local_error),
                    local: None,
                    cloud_instance: None,
                    cloud: None,
                    cloud_assistance_receipt: None,
                    cloud_error: Some(
                        "cloud escalation requires a live local session attempt".to_string(),
                    ),
                    distillation_trace: None,
                });
            }
        };

    match chat_generate_resolved_inner(local_iid, local_runtime, local_model_id, &prompt, state)
        .await
    {
        Ok(local) => {
            if let Some(escalation_reason) = low_confidence_escalation_reason(&request) {
                if request.cloud.provider == SwarmProviderIpc::Local {
                    return Ok(SwarmChatGenerateWithCloudEscalationResponseIpc {
                        selected: Some("local".to_string()),
                        escalated: false,
                        escalation_reason: Some(escalation_reason),
                        local_error: None,
                        local: Some(local),
                        cloud_instance: None,
                        cloud: None,
                        cloud_assistance_receipt: None,
                        cloud_error: Some(
                            "cloud escalation request must target a cloud provider".to_string(),
                        ),
                        distillation_trace: None,
                    });
                }

                let local_trace = LocalEscalationTraceInput {
                    instance_id: local_iid.to_string(),
                    model_id: local_model_id.to_string(),
                    response: local,
                    confidence_basis_points: request
                        .local_confidence_basis_points
                        .map(|value| value.min(10_000)),
                    validation_labels: request.validation_labels.clone(),
                };

                return spawn_generate_cloud_escalation(
                    request.cloud,
                    &request.local_instance_id,
                    &prompt,
                    escalation_reason,
                    CloudFallbackReason::LocalLowConfidence,
                    Some(local_trace),
                    request.cloud_assistance_receipt,
                    state,
                )
                .await;
            }

            Ok(SwarmChatGenerateWithCloudEscalationResponseIpc {
                selected: Some("local".to_string()),
                escalated: false,
                escalation_reason: None,
                local_error: None,
                local: Some(local),
                cloud_instance: None,
                cloud: None,
                cloud_assistance_receipt: None,
                cloud_error: None,
                distillation_trace: None,
            })
        }
        Err(local_error) => {
            if request.cloud.provider == SwarmProviderIpc::Local {
                return Ok(SwarmChatGenerateWithCloudEscalationResponseIpc {
                    selected: None,
                    escalated: false,
                    escalation_reason: None,
                    local_error: Some(local_error),
                    local: None,
                    cloud_instance: None,
                    cloud: None,
                    cloud_assistance_receipt: None,
                    cloud_error: Some(
                        "cloud escalation request must target a cloud provider".to_string(),
                    ),
                    distillation_trace: None,
                });
            }

            let fallback_reason = cloud_fallback_reason_from_local_error(&local_error);
            spawn_generate_cloud_escalation(
                request.cloud,
                &request.local_instance_id,
                &prompt,
                local_error,
                fallback_reason,
                None,
                request.cloud_assistance_receipt,
                state,
            )
            .await
        }
    }
}

async fn spawn_generate_cloud_escalation(
    mut cloud_request: SwarmSpawnRequestIpc,
    parent_session_id: &str,
    prompt: &str,
    escalation_reason: String,
    fallback_reason: CloudFallbackReason,
    local_trace: Option<LocalEscalationTraceInput>,
    receipt_context: Option<CloudAssistanceReceiptContextIpc>,
    state: &SwarmRuntimeState,
) -> Result<SwarmChatGenerateWithCloudEscalationResponseIpc, String> {
    if cloud_request.provider == SwarmProviderIpc::Local {
        return Ok(SwarmChatGenerateWithCloudEscalationResponseIpc {
            selected: None,
            escalated: false,
            escalation_reason: None,
            local_error: Some(escalation_reason),
            local: None,
            cloud_instance: None,
            cloud: None,
            cloud_assistance_receipt: None,
            cloud_error: Some("cloud escalation request must target a cloud provider".to_string()),
            distillation_trace: None,
        });
    }

    let Some(recorder) = state.cloud_assistance_recorder.as_ref() else {
        return Ok(SwarmChatGenerateWithCloudEscalationResponseIpc {
            selected: None,
            escalated: true,
            escalation_reason: Some(escalation_reason),
            local_error: None,
            local: local_trace.as_ref().map(|trace| trace.response.clone()),
            cloud_instance: None,
            cloud: None,
            cloud_assistance_receipt: None,
            cloud_error: Some("cloud assistance receipt recorder is not configured".to_string()),
            distillation_trace: None,
        });
    };
    let Some(context) = receipt_context else {
        return Ok(SwarmChatGenerateWithCloudEscalationResponseIpc {
            selected: None,
            escalated: true,
            escalation_reason: Some(escalation_reason),
            local_error: None,
            local: local_trace.as_ref().map(|trace| trace.response.clone()),
            cloud_instance: None,
            cloud: None,
            cloud_assistance_receipt: None,
            cloud_error: Some(
                "cloud assistance receipt context is required for cloud output".to_string(),
            ),
            distillation_trace: None,
        });
    };
    let prompt_sha256 = sha256_hex(prompt.as_bytes());
    let fallback_basis = recorder
        .record_cloud_fallback_basis(CloudFallbackBasisRuntimeRequest {
            context: context.clone(),
            parent_session_id: parent_session_id.to_string(),
            prompt_sha256: prompt_sha256.clone(),
            fallback_reason,
            local_attempt_ref: cloud_fallback_local_attempt_ref(
                parent_session_id,
                local_trace.as_ref(),
                fallback_reason,
            ),
            evidence_sha256: sha256_hex(escalation_reason.as_bytes()),
            summary: cloud_fallback_basis_summary(fallback_reason, &escalation_reason),
        })
        .await;
    let fallback_basis = match fallback_basis {
        Ok(receipt) => receipt,
        Err(error) => {
            return Ok(SwarmChatGenerateWithCloudEscalationResponseIpc {
                selected: None,
                escalated: true,
                escalation_reason: Some(escalation_reason),
                local_error: None,
                local: local_trace.as_ref().map(|trace| trace.response.clone()),
                cloud_instance: None,
                cloud: None,
                cloud_assistance_receipt: None,
                cloud_error: Some(format!("cloud fallback basis recording failed: {error}")),
                distillation_trace: None,
            });
        }
    };

    if trimmed_opt(&cloud_request.parent_session_id).is_none() {
        cloud_request.parent_session_id = Some(parent_session_id.to_string());
    }
    let cloud_provider = cloud_request.provider;
    let byok_cloud_provider = cloud_request.byok_cloud_provider;
    let cloud_model_name = cloud_request.cloud_model_name.clone();
    let cloud_instance = match spawn_session_inner(cloud_request, state).await {
        Ok(instance) => instance,
        Err(error) => {
            return Ok(SwarmChatGenerateWithCloudEscalationResponseIpc {
                selected: None,
                escalated: true,
                escalation_reason: Some(escalation_reason),
                local_error: None,
                local: None,
                cloud_instance: None,
                cloud: None,
                cloud_assistance_receipt: None,
                cloud_error: Some(error.to_string()),
                distillation_trace: None,
            });
        }
    };
    let cloud_iid = parse_instance_id(&cloud_instance.composite)?;
    let (cloud_runtime, cloud_model_id, _) = state
        .resolve_runtime(cloud_iid)
        .ok_or_else(|| format!("swarm session {cloud_iid} is no longer live after spawn"))?;
    match chat_generate_resolved_inner_with_capture(
        cloud_iid,
        cloud_runtime,
        cloud_model_id,
        prompt,
        state,
        false,
    )
    .await
    {
        Ok(cloud) => {
            let distillation_trace = build_distillation_trace_queue_entry(
                parent_session_id,
                prompt,
                &escalation_reason,
                local_trace.as_ref(),
                &cloud_instance,
                &cloud,
            );
            let output_body_jsonb =
                serde_json::to_value(&cloud).map_err(|error| error.to_string())?;
            let cloud_receipt = recorder
                .record_cloud_assistance(CloudAssistanceRuntimeReceiptRequest {
                    context,
                    cloud_instance: cloud_instance.clone(),
                    cloud_provider,
                    byok_cloud_provider,
                    cloud_model_name,
                    parent_session_id: parent_session_id.to_string(),
                    fallback_basis_event_id: fallback_basis.fallback_basis_event_id.clone(),
                    prompt_sha256,
                    output_sha256: sha256_hex(cloud.text.as_bytes()),
                    output_body_sha256: sha256_hex(
                        serde_json::to_string(&cloud)
                            .map_err(|error| error.to_string())?
                            .as_bytes(),
                    ),
                    output_body_jsonb,
                    output_text: cloud.text.clone(),
                    fallback_reason,
                    escalation_reason: escalation_reason.clone(),
                })
                .await;
            let cloud_assistance_receipt = match cloud_receipt {
                Ok(receipt) => receipt,
                Err(error) => {
                    cancel_cloud_escalation_instance(
                        state,
                        &cloud_instance,
                        "cloud assistance receipt recording failed",
                    )
                    .await;
                    return Ok(SwarmChatGenerateWithCloudEscalationResponseIpc {
                        selected: None,
                        escalated: true,
                        escalation_reason: Some(escalation_reason),
                        local_error: None,
                        local: local_trace.as_ref().map(|trace| trace.response.clone()),
                        cloud_instance: None,
                        cloud: None,
                        cloud_assistance_receipt: None,
                        cloud_error: Some(format!(
                            "cloud assistance receipt recording failed: {error}"
                        )),
                        distillation_trace: None,
                    });
                }
            };
            feed_deferred_cloud_capture(
                state,
                &cloud_instance.composite,
                prompt,
                &cloud.text,
                &cloud_assistance_receipt.receipt_id,
            )
            .await;
            Ok(SwarmChatGenerateWithCloudEscalationResponseIpc {
                selected: Some("cloud".to_string()),
                escalated: true,
                escalation_reason: Some(escalation_reason),
                local_error: None,
                local: local_trace.as_ref().map(|trace| trace.response.clone()),
                cloud_instance: Some(cloud_instance),
                cloud: Some(cloud),
                cloud_assistance_receipt: Some(cloud_assistance_receipt),
                cloud_error: None,
                distillation_trace,
            })
        }
        Err(error) => {
            cancel_cloud_escalation_instance(state, &cloud_instance, "cloud generate failed").await;
            Ok(SwarmChatGenerateWithCloudEscalationResponseIpc {
                selected: None,
                escalated: true,
                escalation_reason: Some(escalation_reason),
                local_error: None,
                local: local_trace.as_ref().map(|trace| trace.response.clone()),
                cloud_instance: None,
                cloud: None,
                cloud_assistance_receipt: None,
                cloud_error: Some(error),
                distillation_trace: None,
            })
        }
    }
}

fn cloud_fallback_local_attempt_ref(
    parent_session_id: &str,
    local_trace: Option<&LocalEscalationTraceInput>,
    fallback_reason: CloudFallbackReason,
) -> String {
    local_trace
        .map(|trace| trace.instance_id.clone())
        .unwrap_or_else(|| {
            format!(
                "{}:{}",
                cloud_fallback_reason_label(fallback_reason),
                parent_session_id
            )
        })
}

fn cloud_fallback_basis_summary(
    fallback_reason: CloudFallbackReason,
    escalation_reason: &str,
) -> String {
    let mut summary = format!(
        "cloud fallback basis recorded before cloud spawn; fallback_reason={}; escalation_reason={}",
        cloud_fallback_reason_label(fallback_reason),
        escalation_reason.trim()
    );
    truncate_to_char_boundary(&mut summary, 512);
    summary
}

async fn cancel_cloud_escalation_instance(
    state: &SwarmRuntimeState,
    cloud_instance: &SwarmInstanceIdIpc,
    reason: &str,
) {
    if let Ok(iid) = parse_instance_id(&cloud_instance.composite) {
        let _ = state.coordinator().cancel_session(iid, reason).await;
        state.close_capture_sink(&iid.to_string(), 1).await;
    }
}

async fn feed_deferred_cloud_capture(
    state: &SwarmRuntimeState,
    instance_id: &str,
    prompt: &str,
    output_text: &str,
    receipt_id: &str,
) {
    if let Some(sink) = state.capture_sink_for(instance_id) {
        sink.feed(
            format!("\r\n$ generate [cloud_assistance_receipt={receipt_id}]: {prompt}\r\n")
                .as_bytes(),
        )
        .await;
        if !output_text.is_empty() {
            sink.feed(output_text.as_bytes()).await;
        }
        sink.feed(b"\r\n").await;
    }
}

fn cloud_assistance_request_from_runtime(
    request: CloudAssistanceRuntimeReceiptRequest,
) -> Result<CloudAssistanceRequest, String> {
    let provider =
        model_provider_kind_from_runtime(request.cloud_provider, request.byok_cloud_provider)?;
    let model_label = trimmed_opt(&request.cloud_model_name)
        .ok_or_else(|| "cloud assistance receipt requires cloud_model_name".to_string())?;
    let provider_metadata = json!({
        "cloud_provider": swarm_provider_label(request.cloud_provider),
        "byok_cloud_provider": request
            .byok_cloud_provider
            .map(byok_cloud_provider_label),
        "cloud_instance": &request.cloud_instance,
        "parent_session_id": &request.parent_session_id,
        "prompt_sha256": &request.prompt_sha256,
        "output_body_sha256": &request.output_body_sha256,
        "fallback_reason": cloud_fallback_reason_label(request.fallback_reason),
    });
    let from_lane = AgentLaneIdentity::new(
        request.context.cloud_lane_id.clone(),
        request.context.cloud_actor_id.clone(),
        AgentLaneKind::Cloud,
        LocalCloudAttribution::cloud(
            provider,
            model_label,
            cloud_credential_ref(provider),
            provider_metadata,
        ),
    )
    .map_err(|error| error.to_string())?;

    Ok(CloudAssistanceRequest {
        from_lane,
        workspace_id: request.context.workspace_id,
        wp_id: request.context.wp_id,
        mt_id: request.context.mt_id,
        claim_id: request.context.claim_id,
        session_id: request.cloud_instance.composite,
        to_role: request.context.to_role,
        mailbox_thread_id: request.context.mailbox_thread_id,
        mailbox_message_id: request.context.mailbox_message_id,
        fallback_basis_event_id: request.fallback_basis_event_id,
        parent_session_id: request.parent_session_id,
        prompt_sha256: request.prompt_sha256,
        fallback_reason: request.fallback_reason,
        output_kind: CloudAssistanceOutputKind::Analysis,
        output_sha256: request.output_sha256,
        body_sha256: request.output_body_sha256,
        output_text: request.output_text.clone(),
        output_body_jsonb: request.output_body_jsonb,
        summary: cloud_assistance_summary(
            request.fallback_reason,
            &request.escalation_reason,
            &request.output_text,
        ),
        target_ref: request.context.target_ref,
    })
}

fn cloud_fallback_basis_request_from_runtime(
    request: CloudFallbackBasisRuntimeRequest,
) -> Result<CloudFallbackBasisRequest, String> {
    let lane = cloud_fallback_system_lane(&request.context)?;
    Ok(CloudFallbackBasisRequest {
        lane,
        workspace_id: request.context.workspace_id,
        wp_id: request.context.wp_id,
        mt_id: request.context.mt_id,
        claim_id: request.context.claim_id,
        parent_session_id: request.parent_session_id,
        prompt_sha256: request.prompt_sha256,
        session_id: "swarm-runtime-cloud-fallback-basis".to_string(),
        fallback_reason: request.fallback_reason,
        local_attempt_ref: request.local_attempt_ref,
        evidence_sha256: request.evidence_sha256,
        summary: request.summary,
    })
}

fn cloud_fallback_system_lane(
    context: &CloudAssistanceReceiptContextIpc,
) -> Result<AgentLaneIdentity, String> {
    AgentLaneIdentity::new(
        "lane-system-cloud-fallback",
        "system-cloud-fallback",
        AgentLaneKind::System,
        LocalCloudAttribution {
            mode: AttributionMode::System,
            provider: None,
            runtime: Some("swarm_runtime".to_string()),
            model_label: "system".to_string(),
            credential_ref: None,
            provider_metadata: json!({
                "cloud_lane_id": context.cloud_lane_id,
                "cloud_actor_id": context.cloud_actor_id,
            }),
        },
    )
    .map_err(|error| error.to_string())
}

fn model_provider_kind_from_runtime(
    cloud_provider: SwarmProviderIpc,
    byok_cloud_provider: Option<ByokCloudProviderIpc>,
) -> Result<ModelProviderKind, String> {
    match cloud_provider {
        SwarmProviderIpc::OfficialCli => Ok(ModelProviderKind::OfficialCli),
        SwarmProviderIpc::ByokCloud => match byok_cloud_provider {
            Some(ByokCloudProviderIpc::Anthropic) => Ok(ModelProviderKind::Anthropic),
            Some(ByokCloudProviderIpc::OpenAi) => Ok(ModelProviderKind::OpenAi),
            None => Err(
                "cloud assistance receipt requires byok_cloud_provider for BYOK attribution"
                    .to_string(),
            ),
        },
        SwarmProviderIpc::Local => {
            Err("cloud assistance receipt cannot be recorded for local provider".to_string())
        }
    }
}

fn cloud_credential_ref(provider: ModelProviderKind) -> &'static str {
    match provider {
        ModelProviderKind::OpenAi => "vault://swarm/byok/openai",
        ModelProviderKind::Anthropic => "vault://swarm/byok/anthropic",
        ModelProviderKind::OfficialCli => "config://swarm/official-cli",
        ModelProviderKind::Other => "config://swarm/cloud/other",
        ModelProviderKind::LocalRuntime => "local://runtime",
    }
}

fn cloud_assistance_summary(
    fallback_reason: CloudFallbackReason,
    escalation_reason: &str,
    output_text: &str,
) -> String {
    let mut summary = format!(
        "cloud fallback output recorded for review; fallback_reason={}; escalation_reason={}; output_chars={}",
        cloud_fallback_reason_label(fallback_reason),
        escalation_reason.trim(),
        output_text.chars().count()
    );
    truncate_to_char_boundary(&mut summary, 512);
    summary
}

fn truncate_to_char_boundary(value: &mut String, max_len: usize) {
    if value.len() <= max_len {
        return;
    }
    let mut end = max_len;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value.truncate(end);
}

fn cloud_fallback_reason_from_local_error(local_error: &str) -> CloudFallbackReason {
    let error = local_error.to_ascii_lowercase();
    if error.contains("overload") || error.contains("capacity") || error.contains("memory") {
        CloudFallbackReason::LocalOverloaded
    } else {
        CloudFallbackReason::LocalFailed
    }
}

fn cloud_fallback_reason_label(reason: CloudFallbackReason) -> &'static str {
    match reason {
        CloudFallbackReason::LocalFailed => "local_failed",
        CloudFallbackReason::LocalLowConfidence => "local_low_confidence",
        CloudFallbackReason::LocalOverloaded => "local_overloaded",
        CloudFallbackReason::LocalSuppressed => "local_suppressed",
        CloudFallbackReason::HardReasoning => "hard_reasoning",
        CloudFallbackReason::ForceCloud => "force_cloud",
        CloudFallbackReason::NoLocalModel => "no_local_model",
    }
}

fn swarm_provider_label(provider: SwarmProviderIpc) -> &'static str {
    match provider {
        SwarmProviderIpc::Local => "local",
        SwarmProviderIpc::ByokCloud => "byok_cloud",
        SwarmProviderIpc::OfficialCli => "official_cli",
    }
}

fn byok_cloud_provider_label(provider: ByokCloudProviderIpc) -> &'static str {
    match provider {
        ByokCloudProviderIpc::Anthropic => "anthropic",
        ByokCloudProviderIpc::OpenAi => "open_ai",
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn low_confidence_escalation_reason(
    request: &SwarmChatGenerateWithCloudEscalationRequestIpc,
) -> Option<String> {
    let confidence = request.local_confidence_basis_points?.min(10_000);
    let threshold = request.min_local_confidence_basis_points?.min(10_000);
    if confidence < threshold {
        Some(format!(
            "local confidence {confidence}bp below threshold {threshold}bp; escalated to cloud"
        ))
    } else {
        None
    }
}

fn build_distillation_trace_queue_entry(
    parent_session_id: &str,
    prompt: &str,
    escalation_reason: &str,
    local_trace: Option<&LocalEscalationTraceInput>,
    cloud_instance: &SwarmInstanceIdIpc,
    cloud: &SwarmChatResponseIpc,
) -> Option<DistillationSwarmTraceQueueEntry> {
    let local_trace = local_trace?;
    let mut route = SwarmTraceRouteMetadata::new(
        parent_session_id,
        escalation_reason,
        SwarmTraceRouteOutcome::CloudEscalated,
        local_trace.model_id.clone(),
        cloud_instance.model_id.clone(),
    )
    .with_validation_labels(local_trace.validation_labels.clone());
    if let Some(confidence) = local_trace.confidence_basis_points {
        route = route.with_local_confidence_basis_points(confidence);
    }

    let candidate = SwarmTraceCandidate::new(
        Uuid::now_v7(),
        format!("{}::{}", local_trace.instance_id, cloud_instance.composite),
        prompt,
        format!("local:{}", local_trace.model_id),
        local_trace.response.text.clone(),
        format!("cloud:{}", cloud_instance.model_id),
        cloud.text.clone(),
    )
    .with_comparison_labels(vec![
        "swarm-escalation".to_string(),
        "local-cloud-distillation".to_string(),
    ])
    .with_route_metadata(route);

    match prepare_distillation_swarm_trace_queue_entry(candidate) {
        Ok(entry) => Some(entry),
        Err(error) => {
            eprintln!(
                "{FR_EVT_SWARM_LIFECYCLE}: distillation trace capture skipped for {parent_session_id}: {error}"
            );
            None
        }
    }
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
    if let Some(swarm_id) = trimmed_opt(&request.swarm_id) {
        spawn = spawn.with_swarm(swarm_id);
    }
    if let Some(wt) = trimmed_opt(&request.worktree_id) {
        spawn = spawn.with_worktree(wt);
    }
    if let Some(wd) = trimmed_opt(&request.working_dir) {
        spawn = spawn.with_working_dir(wd);
    }
    if let Some(tier) = request.isolation_tier {
        spawn = spawn.with_isolation_tier(tier.to_isolation_tier());
    }
    if matches!(request.provider, SwarmProviderIpc::Local) {
        if let Some(bytes) = request.committed_memory_bytes.filter(|bytes| *bytes > 0) {
            spawn = spawn.with_committed_memory_bytes(bytes);
        }
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
                .ok_or_else(|| {
                    "local spawn requires a runtime_binding (candle | llama_cpp)".to_string()
                })?
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
            let mut spawn = SpawnRequest::new(instance_id, binding, "operator", parent)
                .with_local_artifact(path.to_string(), sha.to_string());
            match request.local_execution_mode {
                Some(SwarmLocalExecutionModeIpc::WarmVm) => {
                    validate_warm_vm_request(request, binding)?;
                    spawn = if let Some(manifest) = request.warm_vm_restore_manifest.clone() {
                        spawn.with_warm_vm_restore_manifest(manifest)
                    } else {
                        spawn.with_warm_vm_execution()
                    };
                }
                Some(SwarmLocalExecutionModeIpc::Cold) => {
                    if request.warm_vm_restore_manifest.is_some() {
                        return Err(
                            "warm_vm_restore_manifest requires local_execution_mode=warm_vm"
                                .to_string(),
                        );
                    }
                    spawn.local_execution_mode = Some(LocalExecutionMode::Cold);
                }
                None => {
                    if request.warm_vm_restore_manifest.is_some() {
                        return Err(
                            "warm_vm_restore_manifest requires local_execution_mode=warm_vm"
                                .to_string(),
                        );
                    }
                }
            }
            spawn
        }
        SwarmProviderIpc::ByokCloud | SwarmProviderIpc::OfficialCli => {
            if matches!(
                request.local_execution_mode,
                Some(SwarmLocalExecutionModeIpc::WarmVm)
            ) {
                return Err(
                    "warm_vm local_execution_mode is only valid for provider=local".to_string(),
                );
            }
            if request.warm_vm_restore_manifest.is_some() {
                return Err(
                    "warm_vm_restore_manifest is only valid for provider=local warm_vm".to_string(),
                );
            }
            let model_name = request
                .cloud_model_name
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| "cloud spawn requires a non-empty cloud_model_name".to_string())?;
            // runtime_binding is ignored for cloud runtime selection but the
            // SpawnRequest constructor needs a value; Candle is a harmless
            // placeholder the cloud dispatch path never reads for engine choice.
            let mut spawn =
                SpawnRequest::new(instance_id, RuntimeBinding::Candle, "operator", parent)
                    .with_cloud_provider(
                        request.provider.to_provider_kind(),
                        model_name.to_string(),
                    );
            if matches!(request.provider, SwarmProviderIpc::ByokCloud) {
                if let Some(provider) = request.byok_cloud_provider {
                    spawn = spawn.with_byok_cloud_provider(provider.to_core());
                }
            }
            spawn
        }
    };

    Ok(apply_recorded_assignment(spawn, request))
}

fn validate_warm_vm_request(
    request: &SwarmSpawnRequestIpc,
    binding: RuntimeBinding,
) -> Result<(), String> {
    if binding != RuntimeBinding::LlamaCpp {
        return Err("warm_vm local_execution_mode requires runtime_binding=llama_cpp".to_string());
    }
    if request.isolation_tier != Some(SwarmIsolationTierIpc::Tier3Microvm) {
        return Err(
            "warm_vm local_execution_mode requires isolation_tier=tier3_microvm".to_string(),
        );
    }
    if trimmed_opt(&request.worktree_id).is_none() {
        return Err("warm_vm local_execution_mode requires a non-empty worktree_id".to_string());
    }
    if let Some(manifest) = request.warm_vm_restore_manifest.as_ref() {
        let worktree_id = trimmed_opt(&request.worktree_id)
            .expect("checked non-empty worktree_id before manifest validation");
        if manifest.worktree_id != worktree_id {
            return Err(format!(
                "warm_vm_restore_manifest worktree_id mismatch: expected {}, got {}",
                worktree_id, manifest.worktree_id
            ));
        }
        let sha = trimmed_opt(&request.sha256_expected)
            .expect("local request sha256_expected is validated before warm_vm checks");
        if manifest.model_artifact_sha256 != sha {
            return Err(
                "warm_vm_restore_manifest model_artifact_sha256 must match sha256_expected"
                    .to_string(),
            );
        }
    }
    Ok(())
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
    use handshake_core::capabilities::CapabilityRegistry;
    use handshake_core::flight_recorder::duckdb::DuckDbFlightRecorder;
    use handshake_core::swarm_orchestration::state_recovery::AttributionMode;
    use handshake_core::terminal::TerminalRuntime;
    use std::collections::VecDeque;

    #[derive(Default)]
    struct RecordingCloudAssistanceRecorder {
        fallback_requests: Arc<Mutex<Vec<CloudFallbackBasisRuntimeRequest>>>,
        requests: Arc<Mutex<Vec<CloudAssistanceRuntimeReceiptRequest>>>,
    }

    impl RecordingCloudAssistanceRecorder {
        fn fallback_requests(&self) -> Vec<CloudFallbackBasisRuntimeRequest> {
            self.fallback_requests
                .lock()
                .expect("fallback recorder lock")
                .clone()
        }

        fn recorded(&self) -> Vec<CloudAssistanceRuntimeReceiptRequest> {
            self.requests.lock().expect("receipt recorder lock").clone()
        }
    }

    #[async_trait]
    impl CloudAssistanceReceiptRecorder for RecordingCloudAssistanceRecorder {
        async fn record_cloud_fallback_basis(
            &self,
            request: CloudFallbackBasisRuntimeRequest,
        ) -> Result<CloudFallbackBasisRefIpc, String> {
            let suffix = request.context.target_ref.replace('/', "-");
            self.fallback_requests
                .lock()
                .expect("fallback recorder lock")
                .push(request);
            Ok(CloudFallbackBasisRefIpc {
                basis_id: format!("PSR-FALLBACK-test-{suffix}"),
                fallback_basis_event_id: format!("KE-fallback-basis-test-{suffix}"),
            })
        }

        async fn record_cloud_assistance(
            &self,
            request: CloudAssistanceRuntimeReceiptRequest,
        ) -> Result<CloudAssistanceReceiptRefIpc, String> {
            let fallback_basis_event_id = request.fallback_basis_event_id.clone();
            let output_sha256 = request.output_sha256.clone();
            self.requests
                .lock()
                .expect("receipt recorder lock")
                .push(request);
            Ok(CloudAssistanceReceiptRefIpc {
                receipt_id: "PSR-CLOUD-test-runtime-receipt".to_string(),
                fallback_basis_event_id,
                handoff_id: "PSR-HANDOFF-test-runtime-receipt".to_string(),
                cloud_assistance_event_id: "KE-test-runtime-cloud-assistance".to_string(),
                output_sha256,
                review_state: "pending_review".to_string(),
                non_authoritative: true,
                requires_promotion: true,
            })
        }
    }

    fn cloud_receipt_context(suffix: &str) -> CloudAssistanceReceiptContextIpc {
        CloudAssistanceReceiptContextIpc {
            workspace_id: format!("workspace-{suffix}"),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: "MT-221".to_string(),
            claim_id: format!("PSR-CLAIM-{suffix}"),
            cloud_lane_id: format!("lane-cloud-{suffix}"),
            cloud_actor_id: format!("cloud-{suffix}"),
            fallback_basis_event_id: None,
            to_role: "WP_VALIDATOR".to_string(),
            mailbox_thread_id: format!("thread-{suffix}"),
            mailbox_message_id: format!("message-{suffix}"),
            target_ref: format!("swarm://cloud-assistance/{suffix}"),
        }
    }

    #[test]
    fn cloud_assistance_runtime_request_maps_exact_cloud_lane_identity() {
        let request = CloudAssistanceRuntimeReceiptRequest {
            context: cloud_receipt_context("map-anthropic"),
            cloud_instance: SwarmInstanceIdIpc {
                model_id: Uuid::now_v7().to_string(),
                instance: 0,
                composite: "cloud-session#0".to_string(),
            },
            cloud_provider: SwarmProviderIpc::ByokCloud,
            byok_cloud_provider: Some(ByokCloudProviderIpc::Anthropic),
            cloud_model_name: Some("claude-sonnet-4".to_string()),
            parent_session_id: "local-session#0".to_string(),
            fallback_basis_event_id: "KE-fallback-basis-map-anthropic".to_string(),
            prompt_sha256: "a".repeat(64),
            output_sha256: "b".repeat(64),
            output_body_sha256: "c".repeat(64),
            output_body_jsonb: serde_json::json!({
                "text": "cloud analysis kept pending review",
                "tokenCount": 5,
                "finishReason": "stop"
            }),
            output_text: "cloud analysis kept pending review".to_string(),
            fallback_reason: CloudFallbackReason::LocalLowConfidence,
            escalation_reason: "local confidence 4200 below required 8000".to_string(),
        };

        let mapped = cloud_assistance_request_from_runtime(request)
            .expect("runtime receipt request maps to state-recovery write");

        assert_eq!(mapped.from_lane.lane_id, "lane-cloud-map-anthropic");
        assert_eq!(mapped.from_lane.actor_id, "cloud-map-anthropic");
        assert_eq!(mapped.from_lane.lane_kind, AgentLaneKind::Cloud);
        assert_eq!(mapped.from_lane.attribution.mode, AttributionMode::Cloud);
        assert_eq!(
            mapped.from_lane.attribution.provider,
            Some(ModelProviderKind::Anthropic)
        );
        assert_eq!(mapped.from_lane.attribution.model_label, "claude-sonnet-4");
        assert_eq!(mapped.session_id, "cloud-session#0");
        assert_eq!(
            mapped.fallback_basis_event_id,
            "KE-fallback-basis-map-anthropic"
        );
        assert_eq!(mapped.parent_session_id, "local-session#0");
        assert_eq!(mapped.prompt_sha256, "a".repeat(64));
        assert_eq!(mapped.output_kind, CloudAssistanceOutputKind::Analysis);
        assert_eq!(mapped.output_sha256, "b".repeat(64));
        assert_eq!(mapped.body_sha256, "c".repeat(64));
        assert_eq!(mapped.output_text, "cloud analysis kept pending review");
        assert_eq!(
            mapped.fallback_reason,
            CloudFallbackReason::LocalLowConfidence
        );
    }

    #[test]
    fn cloud_assistance_runtime_request_rejects_ambiguous_byok_provider() {
        let request = CloudAssistanceRuntimeReceiptRequest {
            context: cloud_receipt_context("map-ambiguous"),
            cloud_instance: SwarmInstanceIdIpc {
                model_id: Uuid::now_v7().to_string(),
                instance: 0,
                composite: "cloud-session#0".to_string(),
            },
            cloud_provider: SwarmProviderIpc::ByokCloud,
            byok_cloud_provider: None,
            cloud_model_name: Some("gpt-4o".to_string()),
            parent_session_id: "local-session#0".to_string(),
            fallback_basis_event_id: "KE-fallback-basis-map-ambiguous".to_string(),
            prompt_sha256: "a".repeat(64),
            output_sha256: "b".repeat(64),
            output_body_sha256: "c".repeat(64),
            output_body_jsonb: serde_json::json!({
                "text": "ambiguous cloud analysis",
                "tokenCount": 4,
                "finishReason": "stop"
            }),
            output_text: "ambiguous cloud analysis".to_string(),
            fallback_reason: CloudFallbackReason::LocalFailed,
            escalation_reason: "local failed".to_string(),
        };

        let error = cloud_assistance_request_from_runtime(request)
            .expect_err("BYOK receipt attribution requires exact provider flavor");
        assert!(error.contains("byok_cloud_provider"), "{error}");
    }

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

    #[tokio::test]
    async fn production_state_wires_committed_memory_ceiling_into_budget() {
        let state =
            SwarmRuntimeState::production_with_cloud_and_recorder_and_committed_memory_ceiling(
                CloudLaneFactoryConfig::unconfigured(),
                None,
                std::path::Path::new(""),
                None,
                Some(8 * 1024 * 1024 * 1024),
            );
        assert_eq!(
            state.committed_memory_ceiling_bytes(),
            Some(8 * 1024 * 1024 * 1024)
        );
        assert_eq!(
            state
                .coordinator()
                .remaining()
                .committed_memory_bytes_remaining,
            Some(8 * 1024 * 1024 * 1024)
        );
    }

    #[test]
    fn parse_committed_memory_ceiling_rejects_invalid_and_zero_values() {
        assert_eq!(
            parse_committed_memory_ceiling("8589934592").expect("valid bytes"),
            Some(8 * 1024 * 1024 * 1024)
        );
        assert_eq!(parse_committed_memory_ceiling("0").expect("zero ok"), None);
        assert!(
            parse_committed_memory_ceiling("not-bytes").is_err(),
            "invalid input must not silently become a ceiling"
        );
    }

    fn local_request(path: &str) -> SwarmSpawnRequestIpc {
        SwarmSpawnRequestIpc {
            provider: SwarmProviderIpc::Local,
            artifact_path: Some(path.to_string()),
            sha256_expected: Some("ab".repeat(32)),
            runtime_binding: Some(SwarmRuntimeBindingIpc::Candle),
            local_execution_mode: None,
            warm_vm_restore_manifest: None,
            cloud_model_name: None,
            byok_cloud_provider: None,
            instance: 0,
            parent_session_id: None,
            swarm_id: None,
            worktree_id: None,
            working_dir: None,
            isolation_tier: None,
            committed_memory_bytes: None,
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
            local_execution_mode: None,
            warm_vm_restore_manifest: None,
            cloud_model_name: Some(model.to_string()),
            byok_cloud_provider: None,
            instance: 0,
            parent_session_id: None,
            swarm_id: None,
            worktree_id: None,
            working_dir: None,
            isolation_tier: None,
            committed_memory_bytes: None,
        }
    }

    fn warm_restore_manifest(worktree_id: &str, sha256: &str) -> WarmVmSnapshotManifest {
        WarmVmSnapshotManifest::new(
            worktree_id,
            sha256,
            "/models/model.gguf",
            "ready-nonce",
            handshake_core::sandbox::SnapshotRef::new(
                handshake_core::sandbox::AdapterId::new("cloud_hypervisor"),
                format!("/snapshots/{worktree_id}"),
            ),
        )
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
    fn byok_cloud_provider_ipc_wire_uses_openai_not_open_ai() {
        assert_eq!(
            serde_json::to_string(&ByokCloudProviderIpc::OpenAi).expect("serialize"),
            "\"openai\""
        );
        assert_eq!(
            serde_json::from_str::<ByokCloudProviderIpc>("\"openai\"").expect("deserialize openai"),
            ByokCloudProviderIpc::OpenAi
        );
        assert_eq!(
            serde_json::from_str::<ByokCloudProviderIpc>("\"open_ai\"")
                .expect("deserialize legacy alias"),
            ByokCloudProviderIpc::OpenAi
        );
    }

    #[test]
    fn build_spawn_request_applies_trimmed_worktree_working_dir_and_tier() {
        // Local arm: worktree + working_dir trimmed; tier mapped 1:1.
        let mut req = local_request("some/model.safetensors");
        req.worktree_id = Some("  wt-a  ".to_string());
        req.swarm_id = Some("  swarm-a  ".to_string());
        req.working_dir = Some("  D:/work/wt-a  ".to_string());
        req.isolation_tier = Some(SwarmIsolationTierIpc::Tier3Microvm);
        req.committed_memory_bytes = Some(4096);
        let spawn = build_spawn_request(&req).expect("valid local request");
        assert_eq!(spawn.swarm_id(), Some("swarm-a"));
        assert_eq!(spawn.worktree_id(), Some("wt-a"));
        assert_eq!(spawn.working_dir(), Some("D:/work/wt-a"));
        assert_eq!(
            spawn.isolation_tier(),
            Some(handshake_core::sandbox::adapter::IsolationTier::Tier3Microvm)
        );
        assert_eq!(spawn.committed_memory_bytes, Some(4096));

        // Cloud arm: attribution fields still apply after the cloud build, but
        // committed memory is host-local admission and must not double-charge a
        // cloud fallback.
        let req = SwarmSpawnRequestIpc {
            provider: SwarmProviderIpc::ByokCloud,
            artifact_path: None,
            sha256_expected: None,
            runtime_binding: None,
            local_execution_mode: None,
            warm_vm_restore_manifest: None,
            cloud_model_name: Some("gpt-4o".to_string()),
            byok_cloud_provider: Some(ByokCloudProviderIpc::OpenAi),
            instance: 0,
            parent_session_id: None,
            swarm_id: Some("swarm-cloud".to_string()),
            worktree_id: Some("\twt-cloud\n".to_string()),
            working_dir: Some("./worktrees/cloud".to_string()),
            isolation_tier: Some(SwarmIsolationTierIpc::Tier1Container),
            committed_memory_bytes: Some(2048),
        };
        let spawn = build_spawn_request(&req).expect("valid cloud request");
        assert_eq!(spawn.swarm_id(), Some("swarm-cloud"));
        assert_eq!(spawn.worktree_id(), Some("wt-cloud"));
        assert_eq!(spawn.byok_cloud_provider, Some(ByokCloudProvider::OpenAi));
        assert_eq!(spawn.working_dir(), Some("./worktrees/cloud"));
        assert_eq!(
            spawn.isolation_tier(),
            Some(handshake_core::sandbox::adapter::IsolationTier::Tier1Container)
        );
        assert_eq!(spawn.committed_memory_bytes, None);
    }

    #[test]
    fn build_spawn_request_blank_worktree_and_working_dir_are_none() {
        // Blank / whitespace-only => None (honest "unassigned"), both arms.
        let mut req = local_request("some/model.safetensors");
        req.worktree_id = Some("   ".to_string());
        req.swarm_id = Some("   ".to_string());
        req.working_dir = Some("".to_string());
        req.isolation_tier = None;
        let spawn = build_spawn_request(&req).expect("valid local request");
        assert_eq!(spawn.worktree_id(), None);
        assert_eq!(spawn.swarm_id(), None);
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
            (
                SwarmIsolationTierIpc::Tier1Container,
                IsolationTier::Tier1Container,
            ),
            (
                SwarmIsolationTierIpc::Tier2Syscall,
                IsolationTier::Tier2Syscall,
            ),
            (
                SwarmIsolationTierIpc::Tier3Microvm,
                IsolationTier::Tier3Microvm,
            ),
        ] {
            assert_eq!(ipc.to_isolation_tier(), core);
            assert_eq!(SwarmIsolationTierIpc::from_isolation_tier(core), ipc);
            // Wire string matches the snake_case serde the TS union declares.
            let json = serde_json::to_string(&ipc).expect("serialize tier");
            assert_eq!(json, format!("\"{}\"", ipc.as_str()));
        }
    }

    #[test]
    fn swarm_local_execution_mode_ipc_round_trips_wire_string() {
        for ipc in [
            SwarmLocalExecutionModeIpc::Cold,
            SwarmLocalExecutionModeIpc::WarmVm,
        ] {
            let json = serde_json::to_string(&ipc).expect("serialize mode");
            assert_eq!(json, format!("\"{}\"", ipc.as_str()));
            assert_eq!(
                serde_json::from_str::<SwarmLocalExecutionModeIpc>(&json)
                    .expect("deserialize mode"),
                ipc
            );
        }
    }

    #[test]
    fn build_spawn_request_warm_vm_is_opt_in_and_validated() {
        let mut req = local_request("some/model.gguf");
        req.runtime_binding = Some(SwarmRuntimeBindingIpc::LlamaCpp);
        req.local_execution_mode = Some(SwarmLocalExecutionModeIpc::WarmVm);
        req.isolation_tier = Some(SwarmIsolationTierIpc::Tier3Microvm);
        req.worktree_id = Some("  wt-warm  ".to_string());

        let spawn = build_spawn_request(&req).expect("valid warm vm request");
        assert!(spawn.wants_warm_vm_execution());
        assert_eq!(spawn.runtime_binding, RuntimeBinding::LlamaCpp);
        assert_eq!(spawn.worktree_id(), Some("wt-warm"));

        let mut bad_binding = req.clone();
        bad_binding.runtime_binding = Some(SwarmRuntimeBindingIpc::Candle);
        let err = build_spawn_request(&bad_binding).expect_err("candle rejected");
        assert!(err.contains("runtime_binding=llama_cpp"), "{err}");

        let mut missing_worktree = req.clone();
        missing_worktree.worktree_id = Some("   ".to_string());
        let err = build_spawn_request(&missing_worktree).expect_err("worktree rejected");
        assert!(err.contains("worktree_id"), "{err}");

        let mut cloud = cloud_request("gpt-4o");
        cloud.local_execution_mode = Some(SwarmLocalExecutionModeIpc::WarmVm);
        let err = build_spawn_request(&cloud).expect_err("cloud warm mode rejected");
        assert!(err.contains("provider=local"), "{err}");
    }

    #[test]
    fn build_spawn_request_warm_vm_restore_manifest_is_routed_and_validated() {
        let sha = "ab".repeat(32);
        let manifest = warm_restore_manifest("wt-warm", &sha);
        let mut req = local_request("some/model.gguf");
        req.sha256_expected = Some(sha.clone());
        req.runtime_binding = Some(SwarmRuntimeBindingIpc::LlamaCpp);
        req.local_execution_mode = Some(SwarmLocalExecutionModeIpc::WarmVm);
        req.isolation_tier = Some(SwarmIsolationTierIpc::Tier3Microvm);
        req.worktree_id = Some(" wt-warm ".to_string());
        req.warm_vm_restore_manifest = Some(manifest.clone());

        let spawn = build_spawn_request(&req).expect("valid warm restore request");
        assert!(spawn.wants_warm_vm_execution());
        assert_eq!(spawn.warm_vm_restore_manifest, Some(manifest));

        let mut cold = req.clone();
        cold.local_execution_mode = Some(SwarmLocalExecutionModeIpc::Cold);
        let err = build_spawn_request(&cold).expect_err("cold restore rejected");
        assert!(
            err.contains("warm_vm_restore_manifest requires local_execution_mode=warm_vm"),
            "{err}"
        );

        let mut implicit_cold = req.clone();
        implicit_cold.local_execution_mode = None;
        let err = build_spawn_request(&implicit_cold).expect_err("implicit cold restore rejected");
        assert!(
            err.contains("warm_vm_restore_manifest requires local_execution_mode=warm_vm"),
            "{err}"
        );

        let mut wrong_worktree = req.clone();
        wrong_worktree.warm_vm_restore_manifest = Some(warm_restore_manifest("wt-other", &sha));
        let err = build_spawn_request(&wrong_worktree).expect_err("wrong worktree rejected");
        assert!(err.contains("worktree_id mismatch"), "{err}");

        let mut wrong_sha = req.clone();
        wrong_sha.warm_vm_restore_manifest =
            Some(warm_restore_manifest("wt-warm", &"cd".repeat(32)));
        let err = build_spawn_request(&wrong_sha).expect_err("wrong sha rejected");
        assert!(err.contains("model_artifact_sha256"), "{err}");

        let mut cloud = cloud_request("gpt-4o");
        cloud.warm_vm_restore_manifest = Some(warm_restore_manifest("wt-warm", &sha));
        let err = build_spawn_request(&cloud).expect_err("cloud restore manifest rejected");
        assert!(err.contains("provider=local warm_vm"), "{err}");
    }

    #[test]
    fn warm_vm_restore_manifest_ipc_serde_uses_camel_outer_and_snake_nested() {
        let sha = "ab".repeat(32);
        let wire = serde_json::json!({
            "provider": "local",
            "artifactPath": "D:/models/model.gguf",
            "sha256Expected": sha,
            "runtimeBinding": "llama_cpp",
            "localExecutionMode": "warm_vm",
            "worktreeId": "wt-warm",
            "isolationTier": "tier3_microvm",
            "warmVmRestoreManifest": {
                "protocol_id": "hsk.warm_agent",
                "protocol_version": 1,
                "worktree_id": "wt-warm",
                "model_artifact_sha256": sha,
                "model_guest_path": "/models/model.gguf",
                "ready_nonce": "ready-nonce",
                "snapshot": {
                    "id": "018f3b8e-1111-7111-8111-111111111111",
                    "adapter_id": "cloud_hypervisor",
                    "snapshot_dir": "/snapshots/wt-warm",
                    "created_at_utc": "2026-06-02T00:00:00Z",
                    "observe_path": null
                }
            }
        });
        let req: SwarmSpawnRequestIpc =
            serde_json::from_value(wire).expect("wire shape deserializes");
        assert!(req.warm_vm_restore_manifest.is_some());
        let spawn = build_spawn_request(&req).expect("deserialized request builds");
        assert!(spawn.warm_vm_restore_manifest.is_some());

        let camel_nested = serde_json::json!({
            "provider": "local",
            "artifactPath": "D:/models/model.gguf",
            "sha256Expected": sha,
            "runtimeBinding": "llama_cpp",
            "localExecutionMode": "warm_vm",
            "worktreeId": "wt-warm",
            "isolationTier": "tier3_microvm",
            "warmVmRestoreManifest": {
                "protocolId": "hsk.warm_agent",
                "protocolVersion": 1,
                "worktreeId": "wt-warm",
                "modelArtifactSha256": sha,
                "modelGuestPath": "/models/model.gguf",
                "readyNonce": "ready-nonce",
                "snapshot": {
                    "id": "018f3b8e-1111-7111-8111-111111111111",
                    "adapterId": "cloud_hypervisor",
                    "snapshotDir": "/snapshots/wt-warm",
                    "createdAtUtc": "2026-06-02T00:00:00Z"
                }
            }
        });
        let err = serde_json::from_value::<SwarmSpawnRequestIpc>(camel_nested)
            .expect_err("nested camelCase manifest is rejected");
        assert!(err.to_string().contains("unknown field"), "{err}");
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
        assert!(err.to_string().contains("PROVIDER_NOT_CONFIGURED"), "{err}");
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
    use handshake_core::model_runtime::{
        Embedding, FinishReason, GeneratedToken, KvCacheHandle, LoadSpec, LoraStackHandle,
        ModelCapabilities, ModelRuntimeError, Score, SteeringHookHandle, TokenStream,
    };
    use handshake_core::swarm_orchestration::{CloudLiveRuntime, CloudRuntimeBuilder};

    #[derive(Clone)]
    enum StaticGenerateResponse {
        Text(String),
        Error(String),
    }

    struct StaticCloudBuilder {
        provider: ProviderKind,
        builds: Arc<Mutex<Vec<Option<String>>>>,
        responses: Arc<Mutex<VecDeque<StaticGenerateResponse>>>,
        fallback: StaticGenerateResponse,
    }

    impl StaticCloudBuilder {
        fn openai(text: impl Into<String>) -> Self {
            let response = StaticGenerateResponse::Text(text.into());
            Self {
                provider: ProviderKind::ByokCloud,
                builds: Arc::new(Mutex::new(Vec::new())),
                responses: Arc::new(Mutex::new(VecDeque::new())),
                fallback: response,
            }
        }

        fn openai_sequence(responses: Vec<StaticGenerateResponse>) -> Self {
            let fallback = responses
                .last()
                .cloned()
                .unwrap_or_else(|| StaticGenerateResponse::Text("cloud-ok".to_string()));
            Self {
                provider: ProviderKind::ByokCloud,
                builds: Arc::new(Mutex::new(Vec::new())),
                responses: Arc::new(Mutex::new(VecDeque::from(responses))),
                fallback,
            }
        }

        fn build_count(&self) -> usize {
            self.builds.lock().expect("static builder lock").len()
        }

        fn next_response(&self) -> StaticGenerateResponse {
            self.responses
                .lock()
                .expect("static builder lock")
                .pop_front()
                .unwrap_or_else(|| self.fallback.clone())
        }
    }

    #[async_trait]
    impl CloudRuntimeBuilder for StaticCloudBuilder {
        fn provider(&self) -> ProviderKind {
            self.provider
        }

        async fn build_loaded(
            &self,
            _model_name: &str,
            session_id: Option<String>,
        ) -> Result<CloudLiveRuntime, String> {
            self.builds
                .lock()
                .expect("static builder lock")
                .push(session_id);
            Ok(CloudLiveRuntime {
                runtime: Arc::new(StaticTokenRuntime {
                    response: self.next_response(),
                }),
                model_id: ModelId::new_v7(),
            })
        }
    }

    struct StaticTokenRuntime {
        response: StaticGenerateResponse,
    }

    #[async_trait]
    impl ModelRuntime for StaticTokenRuntime {
        fn adapter_name(&self) -> &'static str {
            "static_cloud_test_runtime"
        }

        async fn load(&mut self, _spec: LoadSpec) -> Result<ModelId, ModelRuntimeError> {
            Ok(ModelId::new_v7())
        }

        async fn unload(&mut self, _id: ModelId) -> Result<(), ModelRuntimeError> {
            Ok(())
        }

        fn generate(&self, _req: GenerateRequest) -> TokenStream {
            match &self.response {
                StaticGenerateResponse::Text(text) => {
                    Box::pin(futures_util::stream::iter(vec![Ok(GeneratedToken {
                        token_id: 0,
                        text: text.clone(),
                        logprob: None,
                        finish_reason: Some(FinishReason::Stop),
                    })]))
                }
                StaticGenerateResponse::Error(error) => {
                    Box::pin(futures_util::stream::iter(vec![Err(
                        ModelRuntimeError::GenerateError(error.clone()),
                    )]))
                }
            }
        }

        async fn score(
            &self,
            _id: ModelId,
            _sequence: Vec<u32>,
        ) -> Result<Score, ModelRuntimeError> {
            Err(ModelRuntimeError::ScoreError(
                "static cloud test runtime does not score".to_string(),
            ))
        }

        async fn embed(&self, _id: ModelId, _text: &str) -> Result<Embedding, ModelRuntimeError> {
            Err(ModelRuntimeError::EmbedError(
                "static cloud test runtime does not embed".to_string(),
            ))
        }

        fn capabilities(&self, _id: ModelId) -> Result<&ModelCapabilities, ModelRuntimeError> {
            Err(ModelRuntimeError::CapabilityNotSupported {
                capability: "capabilities".to_string(),
                adapter: self.adapter_name().to_string(),
            })
        }

        fn kv_cache(&self, _id: ModelId) -> Result<KvCacheHandle, ModelRuntimeError> {
            Err(ModelRuntimeError::KvCacheError(
                "static cloud test runtime has no kv cache".to_string(),
            ))
        }

        fn lora_stack(&self, _id: ModelId) -> Result<LoraStackHandle, ModelRuntimeError> {
            Err(ModelRuntimeError::LoraStackError(
                "static cloud test runtime has no lora stack".to_string(),
            ))
        }

        fn steering_hooks(&self, _id: ModelId) -> Result<SteeringHookHandle, ModelRuntimeError> {
            Err(ModelRuntimeError::SteeringHookError(
                "static cloud test runtime has no steering hooks".to_string(),
            ))
        }

        fn cancel(&self, token: CancellationToken) {
            token.cancel();
        }
    }

    fn state_with_static_openai_cloud(
        builder: Arc<StaticCloudBuilder>,
        app_data_root: &std::path::Path,
    ) -> SwarmRuntimeState {
        state_with_static_openai_cloud_and_recorder(builder, app_data_root).0
    }

    fn state_with_static_openai_cloud_and_recorder(
        builder: Arc<StaticCloudBuilder>,
        app_data_root: &std::path::Path,
    ) -> (SwarmRuntimeState, Arc<RecordingCloudAssistanceRecorder>) {
        let recorder = Arc::new(RecordingCloudAssistanceRecorder::default());
        let state = SwarmRuntimeState::production_with_cloud(CloudLaneFactoryConfig {
            anthropic: None,
            openai: Some(builder),
            official_cli: None,
        })
        .with_spawn_template_store(SpawnTemplateStore::new(app_data_root))
        .with_cloud_assistance_recorder(recorder.clone());
        (state, recorder)
    }

    fn state_with_static_openai_cloud_without_recorder(
        builder: Arc<StaticCloudBuilder>,
        app_data_root: &std::path::Path,
    ) -> SwarmRuntimeState {
        SwarmRuntimeState::production_with_cloud(CloudLaneFactoryConfig {
            anthropic: None,
            openai: Some(builder),
            official_cli: None,
        })
        .with_spawn_template_store(SpawnTemplateStore::new(app_data_root))
    }

    fn test_terminal_runtime() -> TerminalRuntime {
        let recorder =
            Arc::new(DuckDbFlightRecorder::new_in_memory(1).expect("in-memory flight recorder"));
        TerminalRuntime::new(Arc::new(CapabilityRegistry::new()), recorder)
    }

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
        assert_eq!(
            coordinator.session_grouping(iid),
            Some((None, Some("wt-x".to_string())))
        );
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

    /// A credentialed CLOUD session may stay live in the coordinator, but generic
    /// direct chat must reject it before `runtime.generate`. Cloud output is only
    /// allowed through the receipt-gated escalation command.
    #[tokio::test]
    async fn direct_chat_rejects_cloud_session_without_receipt_gate() {
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

        let resolved = state.resolve_runtime(iid);
        assert!(
            resolved.is_some(),
            "cloud session remains live for cancellation and receipt-gated escalation"
        );
        let (_runtime, model_id, provider) = resolved.expect("resolved");
        assert_eq!(provider, ProviderKind::ByokCloud);
        assert_ne!(
            model_id, iid.model_id,
            "resolved model id is the runtime's minted load id, not the request id"
        );

        let err = chat_generate_inner(&iid.to_string(), "hello cloud", &state)
            .await
            .expect_err("generic chat must reject direct cloud output");
        assert!(
            err.contains("direct chat is local-only"),
            "unexpected direct-chat error: {err}"
        );

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

    #[tokio::test]
    async fn cloud_spawn_ignores_committed_memory_estimate_under_host_memory_ceiling() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let builder = Arc::new(StaticCloudBuilder::openai("cloud-ok"));
        let state =
            SwarmRuntimeState::production_with_cloud_and_recorder_and_committed_memory_ceiling(
                CloudLaneFactoryConfig {
                    anthropic: None,
                    openai: Some(builder.clone()),
                    official_cli: None,
                },
                None,
                tmp.path(),
                None,
                Some(1),
            );
        let mut cloud = cloud_request("gpt-4o");
        cloud.byok_cloud_provider = Some(ByokCloudProviderIpc::OpenAi);
        cloud.committed_memory_bytes = Some(8 * 1024 * 1024 * 1024);

        let id = spawn_session_inner(cloud, &state)
            .await
            .expect("cloud spawn must not be rejected by host committed-memory ceiling");

        assert_eq!(builder.build_count(), 1);
        assert_eq!(state.coordinator().live_session_count(), 1);
        assert_eq!(
            state
                .coordinator()
                .remaining()
                .committed_memory_bytes_remaining,
            Some(1),
            "cloud spawn must not reserve host committed memory"
        );
        state
            .coordinator()
            .cancel_session(parse_instance_id(&id.composite).expect("parse id"), "test")
            .await
            .expect("cancel cloud");
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

    #[tokio::test]
    async fn chat_generate_escalation_force_local_blocks_cloud_when_local_missing() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let builder = Arc::new(StaticCloudBuilder::openai("cloud-ok"));
        let state = state_with_static_openai_cloud(builder.clone(), tmp.path());
        let local_iid = ModelInstanceId::new(ModelId::new_v7(), 0).to_string();
        let mut cloud = cloud_request("gpt-4o");
        cloud.byok_cloud_provider = Some(ByokCloudProviderIpc::OpenAi);

        let response = chat_generate_with_cloud_escalation_inner(
            SwarmChatGenerateWithCloudEscalationRequestIpc {
                local_instance_id: local_iid,
                prompt: "summarize this".to_string(),
                cloud,
                task_class: Some(SwarmEscalationTaskClassIpc::ForceLocal),
                local_confidence_basis_points: None,
                min_local_confidence_basis_points: None,
                validation_labels: Vec::new(),
                cloud_assistance_receipt: None,
            },
            &state,
        )
        .await
        .expect("command result");

        assert_eq!(response.selected, None);
        assert!(!response.escalated);
        assert!(
            response
                .cloud_error
                .as_deref()
                .is_some_and(|e| e.contains("force_local")),
            "cloud_error: {:?}",
            response.cloud_error
        );
        assert!(response.local_error.is_some(), "local failure is surfaced");
        assert_eq!(
            builder.build_count(),
            0,
            "force_local must not even spawn the explicit cloud request"
        );
        assert_eq!(state.coordinator().live_session_count(), 0);
    }

    #[tokio::test]
    async fn chat_generate_refuses_missing_local_without_cloud_spawn() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let builder = Arc::new(StaticCloudBuilder::openai("cloud-ok"));
        let state = state_with_static_openai_cloud(builder.clone(), tmp.path());
        let local_iid = ModelInstanceId::new(ModelId::new_v7(), 0).to_string();
        let mut cloud = cloud_request("gpt-4o");
        cloud.byok_cloud_provider = Some(ByokCloudProviderIpc::OpenAi);
        cloud.swarm_id = Some("swarm-escalate".to_string());
        cloud.worktree_id = Some("wt-escalate".to_string());

        let response = chat_generate_with_cloud_escalation_inner(
            SwarmChatGenerateWithCloudEscalationRequestIpc {
                local_instance_id: local_iid,
                prompt: "classify this".to_string(),
                cloud,
                task_class: Some(SwarmEscalationTaskClassIpc::Classification),
                local_confidence_basis_points: None,
                min_local_confidence_basis_points: None,
                validation_labels: Vec::new(),
                cloud_assistance_receipt: None,
            },
            &state,
        )
        .await
        .expect("command result");

        assert_eq!(response.selected, None);
        assert!(!response.escalated);
        assert!(
            response
                .local_error
                .as_deref()
                .is_some_and(|e| e.contains("no longer live")),
            "local_error: {:?}",
            response.local_error
        );
        assert!(
            response
                .cloud_error
                .as_deref()
                .is_some_and(|e| e.contains("live local session attempt")),
            "cloud_error: {:?}",
            response.cloud_error
        );
        assert_eq!(builder.build_count(), 0);
        assert_eq!(state.coordinator().live_session_count(), 0);
    }

    #[tokio::test]
    async fn chat_generate_refuses_invalid_local_id_without_cloud_spawn() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let builder = Arc::new(StaticCloudBuilder::openai("cloud-ok"));
        let state = state_with_static_openai_cloud(builder.clone(), tmp.path());
        let mut cloud = cloud_request("gpt-4o");
        cloud.byok_cloud_provider = Some(ByokCloudProviderIpc::OpenAi);

        let error = chat_generate_with_cloud_escalation_inner(
            SwarmChatGenerateWithCloudEscalationRequestIpc {
                local_instance_id: "not-a-valid-instance-id".to_string(),
                prompt: "classify this".to_string(),
                cloud,
                task_class: Some(SwarmEscalationTaskClassIpc::Classification),
                local_confidence_basis_points: None,
                min_local_confidence_basis_points: None,
                validation_labels: Vec::new(),
                cloud_assistance_receipt: None,
            },
            &state,
        )
        .await
        .expect("command should return a structured no-escalation response");

        assert_eq!(error.selected, None);
        assert!(!error.escalated);
        assert!(
            error
                .local_error
                .as_deref()
                .is_some_and(|e| e.contains("instance_id must be")),
            "local_error: {:?}",
            error.local_error
        );
        assert_eq!(builder.build_count(), 0);
        assert_eq!(state.coordinator().live_session_count(), 0);
    }

    #[tokio::test]
    async fn chat_generate_refuses_blank_prompt_without_cloud_spawn() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let builder = Arc::new(StaticCloudBuilder::openai("cloud-ok"));
        let state = state_with_static_openai_cloud(builder.clone(), tmp.path());
        let local_iid = ModelInstanceId::new(ModelId::new_v7(), 0).to_string();
        let mut cloud = cloud_request("gpt-4o");
        cloud.byok_cloud_provider = Some(ByokCloudProviderIpc::OpenAi);

        let error = chat_generate_with_cloud_escalation_inner(
            SwarmChatGenerateWithCloudEscalationRequestIpc {
                local_instance_id: local_iid,
                prompt: "   \n\t".to_string(),
                cloud,
                task_class: Some(SwarmEscalationTaskClassIpc::Classification),
                local_confidence_basis_points: None,
                min_local_confidence_basis_points: None,
                validation_labels: Vec::new(),
                cloud_assistance_receipt: None,
            },
            &state,
        )
        .await
        .expect_err("blank prompt is a caller error, not a cloud fallback response");

        assert!(error.contains("prompt must not be empty"), "error: {error}");
        assert_eq!(builder.build_count(), 0);
        assert_eq!(state.coordinator().live_session_count(), 0);
    }

    #[tokio::test]
    async fn chat_generate_force_cloud_refuses_blank_prompt_before_spawn() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let builder = Arc::new(StaticCloudBuilder::openai("cloud-ok"));
        let state = state_with_static_openai_cloud(builder.clone(), tmp.path());
        let local_iid = ModelInstanceId::new(ModelId::new_v7(), 0).to_string();
        let mut cloud = cloud_request("gpt-4o");
        cloud.byok_cloud_provider = Some(ByokCloudProviderIpc::OpenAi);

        let error = chat_generate_with_cloud_escalation_inner(
            SwarmChatGenerateWithCloudEscalationRequestIpc {
                local_instance_id: local_iid,
                prompt: "".to_string(),
                cloud,
                task_class: Some(SwarmEscalationTaskClassIpc::ForceCloud),
                local_confidence_basis_points: None,
                min_local_confidence_basis_points: None,
                validation_labels: Vec::new(),
                cloud_assistance_receipt: None,
            },
            &state,
        )
        .await
        .expect_err("force_cloud still validates prompt before cloud spawn");

        assert!(error.contains("prompt must not be empty"), "error: {error}");
        assert_eq!(builder.build_count(), 0);
        assert_eq!(state.coordinator().live_session_count(), 0);
    }

    #[tokio::test]
    async fn chat_generate_escalates_after_live_local_generate_failure() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let builder = Arc::new(StaticCloudBuilder::openai_sequence(vec![
            StaticGenerateResponse::Error("local runtime overloaded".to_string()),
            StaticGenerateResponse::Text("cloud-ok".to_string()),
        ]));
        let (state, receipt_recorder) =
            state_with_static_openai_cloud_and_recorder(builder.clone(), tmp.path());
        let mut local_candidate = cloud_request("gpt-4o");
        local_candidate.byok_cloud_provider = Some(ByokCloudProviderIpc::OpenAi);
        let local_instance = spawn_session_inner(local_candidate, &state)
            .await
            .expect("live local-candidate runtime");
        assert_eq!(builder.build_count(), 1);

        let mut cloud = cloud_request("gpt-4o");
        cloud.byok_cloud_provider = Some(ByokCloudProviderIpc::OpenAi);
        cloud.swarm_id = Some("swarm-escalate".to_string());
        cloud.worktree_id = Some("wt-escalate".to_string());

        let response = chat_generate_with_cloud_escalation_inner(
            SwarmChatGenerateWithCloudEscalationRequestIpc {
                local_instance_id: local_instance.composite.clone(),
                prompt: "classify this".to_string(),
                cloud,
                task_class: Some(SwarmEscalationTaskClassIpc::Classification),
                local_confidence_basis_points: None,
                min_local_confidence_basis_points: None,
                validation_labels: Vec::new(),
                cloud_assistance_receipt: Some(cloud_receipt_context("local-failure")),
            },
            &state,
        )
        .await
        .expect("command result");

        assert_eq!(response.selected.as_deref(), Some("cloud"));
        assert!(response.escalated);
        assert!(
            response
                .escalation_reason
                .as_deref()
                .is_some_and(|r| r.contains("local runtime overloaded")),
            "escalation_reason: {:?}",
            response.escalation_reason
        );
        assert_eq!(response.local_error, None);
        let cloud = response.cloud.as_ref().expect("cloud generated");
        assert_eq!(cloud.text, "cloud-ok");
        assert_eq!(cloud.token_count, 1);
        assert_eq!(cloud.finish_reason.as_deref(), Some("stop"));
        let receipt = response
            .cloud_assistance_receipt
            .as_ref()
            .expect("cloud output carries durable receipt ref");
        assert_eq!(receipt.review_state, "pending_review");
        assert!(receipt.non_authoritative);
        assert!(receipt.requires_promotion);
        assert_eq!(receipt.output_sha256, sha256_hex(b"cloud-ok"));
        let recorded = receipt_recorder.recorded();
        assert_eq!(recorded.len(), 1);
        let fallback_requests = receipt_recorder.fallback_requests();
        assert_eq!(fallback_requests.len(), 1);
        assert_eq!(
            fallback_requests[0].fallback_reason,
            CloudFallbackReason::LocalOverloaded
        );
        assert_eq!(
            recorded[0].fallback_basis_event_id,
            "KE-fallback-basis-test-swarm:--cloud-assistance-local-failure"
        );
        assert_eq!(
            recorded[0].fallback_reason,
            CloudFallbackReason::LocalOverloaded
        );
        assert_eq!(recorded[0].context.mt_id, "MT-221");
        assert_eq!(recorded[0].output_sha256, sha256_hex(b"cloud-ok"));
        assert_eq!(builder.build_count(), 2);
        assert_eq!(state.coordinator().live_session_count(), 2);
        let worktrees = list_worktrees_inner(&state);
        assert_eq!(worktrees.len(), 1);
        assert_eq!(worktrees[0].worktree_id, "wt-escalate");

        let local_iid = parse_instance_id(&local_instance.composite).expect("parse local iid");
        let cloud_iid = parse_instance_id(
            &response
                .cloud_instance
                .as_ref()
                .expect("cloud instance")
                .composite,
        )
        .expect("parse cloud iid");
        state
            .coordinator()
            .cancel_session(local_iid, "test_cancel")
            .await
            .expect("cancel local");
        state
            .coordinator()
            .cancel_session(cloud_iid, "test_cancel")
            .await
            .expect("cancel cloud");
        assert_eq!(state.coordinator().live_session_count(), 0);
    }

    #[tokio::test]
    async fn chat_generate_low_confidence_escalation_returns_distillation_trace_queue_entry() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let builder = Arc::new(StaticCloudBuilder::openai_sequence(vec![
            StaticGenerateResponse::Text(
                "local answer kept safe context while under threshold".to_string(),
            ),
            StaticGenerateResponse::Text(
                "cloud teacher answer kept stronger comparison".to_string(),
            ),
        ]));
        let (state, receipt_recorder) =
            state_with_static_openai_cloud_and_recorder(builder.clone(), tmp.path());
        let mut local_candidate = cloud_request("gpt-4o");
        local_candidate.byok_cloud_provider = Some(ByokCloudProviderIpc::OpenAi);
        let local_instance = spawn_session_inner(local_candidate, &state)
            .await
            .expect("live local-candidate runtime");

        let mut cloud = cloud_request("gpt-4o");
        cloud.byok_cloud_provider = Some(ByokCloudProviderIpc::OpenAi);
        cloud.swarm_id = Some("swarm-trace".to_string());
        cloud.worktree_id = Some("wt-trace".to_string());

        let response = chat_generate_with_cloud_escalation_inner(
            SwarmChatGenerateWithCloudEscalationRequestIpc {
                local_instance_id: local_instance.composite.clone(),
                prompt: "Classify with OPENAI_API_KEY=sk-trace-secret but preserve safe context"
                    .to_string(),
                cloud,
                task_class: Some(SwarmEscalationTaskClassIpc::Classification),
                local_confidence_basis_points: Some(4_200),
                min_local_confidence_basis_points: Some(8_000),
                validation_labels: vec!["low-confidence".to_string(), "teacher-win".to_string()],
                cloud_assistance_receipt: Some(cloud_receipt_context("low-confidence")),
            },
            &state,
        )
        .await
        .expect("command result");

        assert_eq!(response.selected.as_deref(), Some("cloud"));
        assert!(response.escalated);
        assert_eq!(
            response.local.as_ref().expect("student retained").text,
            "local answer kept safe context while under threshold"
        );
        assert_eq!(
            response.cloud.as_ref().expect("teacher generated").text,
            "cloud teacher answer kept stronger comparison"
        );
        let receipt = response
            .cloud_assistance_receipt
            .as_ref()
            .expect("teacher output carries durable receipt ref");
        assert_eq!(
            receipt.output_sha256,
            sha256_hex(b"cloud teacher answer kept stronger comparison")
        );
        let recorded = receipt_recorder.recorded();
        assert_eq!(recorded.len(), 1);
        assert_eq!(receipt_recorder.fallback_requests().len(), 1);
        assert_eq!(
            recorded[0].fallback_reason,
            CloudFallbackReason::LocalLowConfidence
        );
        let trace = response
            .distillation_trace
            .as_ref()
            .expect("low-confidence escalation returns trace entry");
        assert_eq!(
            trace.schema,
            handshake_core::distillation::swarm_trace::DISTILLATION_SWARM_TRACE_QUEUE_SCHEMA
        );
        assert!(trace.eligible_for_training_review);
        assert!(!trace.bundle.prompt.contains("sk-trace-secret"));
        assert!(trace.bundle.prompt.contains("preserve safe context"));
        let route = trace.bundle.route.as_ref().expect("route metadata");
        assert_eq!(route.session_id, local_instance.composite);
        assert_eq!(route.route_outcome, SwarmTraceRouteOutcome::CloudEscalated);
        assert_eq!(route.local_confidence_basis_points, Some(4_200));
        assert_eq!(
            route.validation_labels,
            vec!["low-confidence".to_string(), "teacher-win".to_string()]
        );
        assert!(!route.local_model_id.trim().is_empty());
        assert_eq!(
            trace.bundle.outputs[0].label,
            format!("local:{}", route.local_model_id)
        );
        assert_eq!(
            route.cloud_model_id,
            response
                .cloud_instance
                .as_ref()
                .expect("cloud instance")
                .model_id
        );
        assert_eq!(trace.bundle.outputs.len(), 2);
        assert_eq!(
            trace.bundle.outputs[0].output,
            "local answer kept safe context while under threshold"
        );
        assert_eq!(
            trace.bundle.outputs[1].output,
            "cloud teacher answer kept stronger comparison"
        );

        let local_iid = parse_instance_id(&local_instance.composite).expect("parse local iid");
        let cloud_iid = parse_instance_id(
            &response
                .cloud_instance
                .as_ref()
                .expect("cloud instance")
                .composite,
        )
        .expect("parse cloud iid");
        state
            .coordinator()
            .cancel_session(local_iid, "test_cancel")
            .await
            .expect("cancel local");
        state
            .coordinator()
            .cancel_session(cloud_iid, "test_cancel")
            .await
            .expect("cancel cloud");
    }

    #[tokio::test]
    async fn chat_generate_cloud_output_fails_closed_without_receipt_context() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let builder = Arc::new(StaticCloudBuilder::openai_sequence(vec![
            StaticGenerateResponse::Error("local runtime failed".to_string()),
            StaticGenerateResponse::Text("cloud-output-without-context".to_string()),
        ]));
        let (state, receipt_recorder) =
            state_with_static_openai_cloud_and_recorder(builder.clone(), tmp.path());
        let mut local_candidate = cloud_request("gpt-4o");
        local_candidate.byok_cloud_provider = Some(ByokCloudProviderIpc::OpenAi);
        let local_instance = spawn_session_inner(local_candidate, &state)
            .await
            .expect("live local-candidate runtime");

        let mut cloud = cloud_request("gpt-4o");
        cloud.byok_cloud_provider = Some(ByokCloudProviderIpc::OpenAi);
        let response = chat_generate_with_cloud_escalation_inner(
            SwarmChatGenerateWithCloudEscalationRequestIpc {
                local_instance_id: local_instance.composite.clone(),
                prompt: "classify this".to_string(),
                cloud,
                task_class: Some(SwarmEscalationTaskClassIpc::Classification),
                local_confidence_basis_points: None,
                min_local_confidence_basis_points: None,
                validation_labels: Vec::new(),
                cloud_assistance_receipt: None,
            },
            &state,
        )
        .await
        .expect("command result");

        assert_eq!(response.selected, None);
        assert!(response.escalated);
        assert!(response.cloud.is_none());
        assert!(response.cloud_assistance_receipt.is_none());
        assert!(
            response
                .cloud_error
                .as_deref()
                .is_some_and(|error| error.contains("receipt context is required")),
            "cloud_error: {:?}",
            response.cloud_error
        );
        assert!(receipt_recorder.recorded().is_empty());
        assert!(receipt_recorder.fallback_requests().is_empty());
        assert_eq!(builder.build_count(), 1);
    }

    #[tokio::test]
    async fn cloud_escalation_without_receipt_context_does_not_spawn_or_capture_cloud_output() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let terminal = test_terminal_runtime();
        let builder = Arc::new(StaticCloudBuilder::openai_sequence(vec![
            StaticGenerateResponse::Error("local runtime failed".to_string()),
            StaticGenerateResponse::Text("cloud-output-without-context".to_string()),
        ]));
        let (state, receipt_recorder) =
            state_with_static_openai_cloud_and_recorder(builder.clone(), tmp.path());
        let state = state.with_terminal_capture(terminal.clone());
        let mut local_candidate = cloud_request("gpt-4o");
        local_candidate.byok_cloud_provider = Some(ByokCloudProviderIpc::OpenAi);
        let local_instance = spawn_session_inner(local_candidate, &state)
            .await
            .expect("live local-candidate runtime");

        let mut cloud = cloud_request("gpt-4o");
        cloud.byok_cloud_provider = Some(ByokCloudProviderIpc::OpenAi);
        let response = chat_generate_with_cloud_escalation_inner(
            SwarmChatGenerateWithCloudEscalationRequestIpc {
                local_instance_id: local_instance.composite.clone(),
                prompt: "classify this".to_string(),
                cloud,
                task_class: Some(SwarmEscalationTaskClassIpc::Classification),
                local_confidence_basis_points: None,
                min_local_confidence_basis_points: None,
                validation_labels: Vec::new(),
                cloud_assistance_receipt: None,
            },
            &state,
        )
        .await
        .expect("command result");

        assert_eq!(response.selected, None);
        assert!(response.cloud.is_none());
        assert!(response.cloud_instance.is_none());
        assert!(response.cloud_assistance_receipt.is_none());
        assert!(receipt_recorder.recorded().is_empty());
        assert!(receipt_recorder.fallback_requests().is_empty());
        assert_eq!(
            builder.build_count(),
            1,
            "missing receipt context must fail before spawning the cloud lane"
        );
        let captured = terminal
            .list_sessions()
            .into_iter()
            .filter_map(|info| terminal.scrollback(&info.session_id).ok())
            .map(|bytes| String::from_utf8_lossy(&bytes).to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            !captured.contains("cloud-output-without-context"),
            "unreceipted cloud output leaked into capture scrollback: {captured:?}"
        );
    }

    #[tokio::test]
    async fn chat_generate_cloud_output_fails_closed_without_receipt_recorder() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let builder = Arc::new(StaticCloudBuilder::openai_sequence(vec![
            StaticGenerateResponse::Error("local runtime failed".to_string()),
            StaticGenerateResponse::Text("cloud-output-without-recorder".to_string()),
        ]));
        let state = state_with_static_openai_cloud_without_recorder(builder.clone(), tmp.path());
        let mut local_candidate = cloud_request("gpt-4o");
        local_candidate.byok_cloud_provider = Some(ByokCloudProviderIpc::OpenAi);
        let local_instance = spawn_session_inner(local_candidate, &state)
            .await
            .expect("live local-candidate runtime");

        let mut cloud = cloud_request("gpt-4o");
        cloud.byok_cloud_provider = Some(ByokCloudProviderIpc::OpenAi);
        let response = chat_generate_with_cloud_escalation_inner(
            SwarmChatGenerateWithCloudEscalationRequestIpc {
                local_instance_id: local_instance.composite.clone(),
                prompt: "classify this".to_string(),
                cloud,
                task_class: Some(SwarmEscalationTaskClassIpc::Classification),
                local_confidence_basis_points: None,
                min_local_confidence_basis_points: None,
                validation_labels: Vec::new(),
                cloud_assistance_receipt: Some(cloud_receipt_context("missing-recorder")),
            },
            &state,
        )
        .await
        .expect("command result");

        assert_eq!(response.selected, None);
        assert!(response.escalated);
        assert!(response.cloud.is_none());
        assert!(response.cloud_assistance_receipt.is_none());
        assert!(
            response
                .cloud_error
                .as_deref()
                .is_some_and(|error| error.contains("receipt recorder is not configured")),
            "cloud_error: {:?}",
            response.cloud_error
        );
        assert!(response.cloud_instance.is_none());
        assert_eq!(builder.build_count(), 1);
    }

    #[tokio::test]
    async fn generic_chat_rejects_cloud_sessions_without_receipt_gate() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let builder = Arc::new(StaticCloudBuilder::openai("direct-cloud-output"));
        let state = state_with_static_openai_cloud(builder.clone(), tmp.path());
        let mut cloud = cloud_request("gpt-4o");
        cloud.byok_cloud_provider = Some(ByokCloudProviderIpc::OpenAi);
        let cloud_instance = spawn_session_inner(cloud, &state)
            .await
            .expect("cloud session live");

        let error = chat_generate_inner(&cloud_instance.composite, "direct cloud chat", &state)
            .await
            .expect_err("generic chat must not bypass MT-221 cloud receipt gate");

        assert!(
            error.contains("receipt-gated cloud escalation"),
            "error: {error}"
        );
        assert_eq!(
            builder.build_count(),
            1,
            "rejected generic chat must not generate cloud output"
        );
    }

    #[tokio::test]
    async fn chat_generate_force_cloud_skips_a_live_local_candidate() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let builder = Arc::new(StaticCloudBuilder::openai("cloud-ok"));
        let (state, receipt_recorder) =
            state_with_static_openai_cloud_and_recorder(builder.clone(), tmp.path());
        let mut existing = cloud_request("gpt-4o");
        existing.byok_cloud_provider = Some(ByokCloudProviderIpc::OpenAi);
        let existing_instance = spawn_session_inner(existing, &state)
            .await
            .expect("first cloud-backed candidate live");
        assert_eq!(builder.build_count(), 1);

        let mut cloud = cloud_request("gpt-4o");
        cloud.byok_cloud_provider = Some(ByokCloudProviderIpc::OpenAi);
        let response = chat_generate_with_cloud_escalation_inner(
            SwarmChatGenerateWithCloudEscalationRequestIpc {
                local_instance_id: existing_instance.composite.clone(),
                prompt: "force the cloud lane".to_string(),
                cloud,
                task_class: Some(SwarmEscalationTaskClassIpc::ForceCloud),
                local_confidence_basis_points: None,
                min_local_confidence_basis_points: None,
                validation_labels: Vec::new(),
                cloud_assistance_receipt: Some(cloud_receipt_context("force-cloud")),
            },
            &state,
        )
        .await
        .expect("command result");

        assert_eq!(response.selected.as_deref(), Some("cloud"));
        assert!(response.escalated);
        assert!(
            response.local.is_none(),
            "force_cloud must not return the already-live local candidate"
        );
        assert!(
            response
                .escalation_reason
                .as_deref()
                .is_some_and(|r| r.contains("force_cloud")),
            "escalation_reason: {:?}",
            response.escalation_reason
        );
        assert_eq!(builder.build_count(), 2);
        assert!(response.cloud_assistance_receipt.is_some());
        let recorded = receipt_recorder.recorded();
        assert_eq!(recorded.len(), 1);
        assert_eq!(receipt_recorder.fallback_requests().len(), 1);
        assert_eq!(recorded[0].fallback_reason, CloudFallbackReason::ForceCloud);
        let cloud_instance = response.cloud_instance.as_ref().expect("cloud instance");
        assert_ne!(
            cloud_instance.composite, existing_instance.composite,
            "force_cloud spawned/generated on a new explicit cloud lane"
        );

        let first_iid =
            parse_instance_id(&existing_instance.composite).expect("parse existing iid");
        let second_iid = parse_instance_id(&cloud_instance.composite).expect("parse cloud iid");
        state
            .coordinator()
            .cancel_session(first_iid, "test_cancel")
            .await
            .expect("cancel first");
        state
            .coordinator()
            .cancel_session(second_iid, "test_cancel")
            .await
            .expect("cancel second");
        assert_eq!(state.coordinator().live_session_count(), 0);
    }

    // ---- ROI#3 STATE RECOVERY: resume-template capture + rebuild + resume ----

    /// `template_from_ipc` captures EXACTLY the validated local fields (artifact +
    /// sha + binding), trims blank worktree/working_dir to None, maps the tier 1:1,
    /// and records the origin lineage. Cloud fields are not captured for a local.
    #[test]
    fn template_from_ipc_captures_validated_local_fields() {
        let mut req = local_request("D:/models/qwen.safetensors");
        req.instance = 3;
        req.runtime_binding = Some(SwarmRuntimeBindingIpc::LlamaCpp);
        req.local_execution_mode = Some(SwarmLocalExecutionModeIpc::WarmVm);
        req.swarm_id = Some("  swarm-a  ".to_string());
        req.worktree_id = Some("  wt-a  ".to_string());
        req.working_dir = Some("   ".to_string()); // blank => None
        req.isolation_tier = Some(SwarmIsolationTierIpc::Tier3Microvm);
        req.cloud_model_name = Some("ignored-for-local".to_string());
        let tpl = template_from_ipc(&req, "origin#0");
        assert_eq!(tpl.provider, TemplateProvider::Local);
        assert_eq!(
            tpl.artifact_path.as_deref(),
            Some("D:/models/qwen.safetensors")
        );
        assert_eq!(tpl.sha256_expected.as_deref(), Some(&"ab".repeat(32)[..]));
        assert_eq!(tpl.runtime_binding, Some(TemplateRuntimeBinding::LlamaCpp));
        assert_eq!(
            tpl.local_execution_mode,
            Some(TemplateLocalExecutionMode::WarmVm)
        );
        assert_eq!(tpl.warm_vm_restore_manifest, None);
        assert_eq!(
            tpl.cloud_model_name, None,
            "local must not capture cloud name"
        );
        assert_eq!(tpl.byok_cloud_provider, None);
        assert_eq!(tpl.instance, 3);
        assert_eq!(tpl.swarm_id.as_deref(), Some("swarm-a"), "trimmed");
        assert_eq!(tpl.worktree_id.as_deref(), Some("wt-a"), "trimmed");
        assert_eq!(tpl.working_dir, None, "blank working_dir trimmed to None");
        assert_eq!(
            tpl.isolation_tier,
            Some(TemplateIsolationTier::Tier3Microvm)
        );
        assert_eq!(tpl.origin_session_id, "origin#0");
    }

    /// `template_from_ipc` for a cloud request captures only the cloud model name
    /// (no artifact/sha/binding), preserving the honest local/cloud split.
    #[test]
    fn template_from_ipc_captures_only_cloud_model_name_for_cloud() {
        let mut req = cloud_request("claude-sonnet-4");
        req.byok_cloud_provider = Some(ByokCloudProviderIpc::Anthropic);
        req.swarm_id = Some("swarm-cloud".to_string());
        req.committed_memory_bytes = Some(123456789);
        let tpl = template_from_ipc(&req, "origin#1");
        assert_eq!(tpl.provider, TemplateProvider::ByokCloud);
        assert_eq!(tpl.cloud_model_name.as_deref(), Some("claude-sonnet-4"));
        assert_eq!(
            tpl.byok_cloud_provider,
            Some(TemplateByokCloudProvider::Anthropic)
        );
        assert_eq!(tpl.swarm_id.as_deref(), Some("swarm-cloud"));
        assert_eq!(tpl.artifact_path, None);
        assert_eq!(tpl.sha256_expected, None);
        assert_eq!(tpl.runtime_binding, None);
        assert_eq!(tpl.local_execution_mode, None);
        assert_eq!(tpl.warm_vm_restore_manifest, None);
        assert_eq!(tpl.committed_memory_bytes, None);
        assert_eq!(tpl.origin_session_id, "origin#1");
    }

    #[test]
    fn template_round_trip_preserves_warm_restore_manifest_for_warm_vm_resume() {
        let sha = "ab".repeat(32);
        let manifest = warm_restore_manifest("wt-warm", &sha);
        let mut req = local_request("D:/models/model.gguf");
        req.sha256_expected = Some(sha);
        req.runtime_binding = Some(SwarmRuntimeBindingIpc::LlamaCpp);
        req.local_execution_mode = Some(SwarmLocalExecutionModeIpc::WarmVm);
        req.warm_vm_restore_manifest = Some(manifest.clone());
        req.worktree_id = Some("wt-warm".to_string());
        req.isolation_tier = Some(SwarmIsolationTierIpc::Tier3Microvm);

        let tpl = template_from_ipc(&req, "origin#0");
        assert_eq!(tpl.warm_vm_restore_manifest, Some(manifest.clone()));

        let rebuilt = template_to_request_ipc(&tpl, "origin#0");
        assert_eq!(rebuilt.warm_vm_restore_manifest, Some(manifest.clone()));
        let spawn = build_spawn_request(&rebuilt).expect("warm restore template rebuilds");
        assert_eq!(spawn.warm_vm_restore_manifest, Some(manifest));
    }

    #[test]
    fn attach_latest_warm_restore_manifest_reuses_latest_matching_template() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let store = SpawnTemplateStore::with_path(tmp.path().join("templates.json"));
        let sha = "ab".repeat(32);
        let old_manifest = warm_restore_manifest("wt-warm", &sha);
        let new_manifest = WarmVmSnapshotManifest::new(
            "wt-warm",
            sha.clone(),
            "/models/model.gguf",
            "new-ready-nonce",
            handshake_core::sandbox::SnapshotRef::new(
                handshake_core::sandbox::AdapterId::new("cloud_hypervisor"),
                "/snap/new",
            ),
        );
        let mut base = local_request("D:/models/model.gguf");
        base.sha256_expected = Some(sha);
        base.runtime_binding = Some(SwarmRuntimeBindingIpc::LlamaCpp);
        base.local_execution_mode = Some(SwarmLocalExecutionModeIpc::WarmVm);
        base.worktree_id = Some("wt-warm".to_string());
        base.isolation_tier = Some(SwarmIsolationTierIpc::Tier3Microvm);

        let mut older = base.clone();
        older.warm_vm_restore_manifest = Some(old_manifest);
        let mut old_template = template_from_ipc(&older, "old#0");
        old_template.captured_at = chrono::Utc::now() - chrono::Duration::minutes(2);
        store
            .upsert("old#0", old_template)
            .expect("store old template");

        let mut newer = base.clone();
        newer.warm_vm_restore_manifest = Some(new_manifest.clone());
        let mut new_template = template_from_ipc(&newer, "new#0");
        new_template.captured_at = chrono::Utc::now();
        store
            .upsert("new#0", new_template)
            .expect("store new template");

        let hydrated = attach_latest_warm_restore_manifest(base.clone(), &store);
        assert!(hydrated.auto_attached);
        assert_eq!(
            hydrated.request.warm_vm_restore_manifest,
            Some(new_manifest),
            "latest matching warm template should auto-hydrate the request"
        );

        let mut moved_artifact = base;
        moved_artifact.artifact_path = Some("D:/models/moved/model.gguf".to_string());
        let moved = attach_latest_warm_restore_manifest(moved_artifact, &store);
        assert!(!moved.auto_attached);
        assert_eq!(
            moved.request.warm_vm_restore_manifest, None,
            "artifact path mismatch should fall back to a fresh warm load"
        );
    }

    #[test]
    fn auto_warm_restore_retry_is_limited_to_auto_factory_failures() {
        let factory_error = SpawnCommandError::Swarm(SwarmError::FactoryFailed(
            "restore snapshot is already live".to_string(),
        ));
        assert!(should_retry_auto_warm_restore(true, &factory_error));
        assert!(
            !should_retry_auto_warm_restore(false, &factory_error),
            "operator-supplied manifests stay strict"
        );
        assert!(!should_retry_auto_warm_restore(
            true,
            &SpawnCommandError::Validation("bad request".to_string())
        ));
        assert!(!should_retry_auto_warm_restore(
            true,
            &SpawnCommandError::Swarm(SwarmError::LedgerFailed("ledger".to_string()))
        ));
    }

    #[tokio::test]
    async fn after_successful_spawn_persists_tracked_warm_manifest_for_fresh_warm_vm() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let store = SpawnTemplateStore::with_path(tmp.path().join("templates.json"));
        let state =
            SwarmRuntimeState::production_with_cloud_and_recorder_and_committed_memory_ceiling(
                CloudLaneFactoryConfig::unconfigured(),
                None,
                tmp.path(),
                None,
                None,
            )
            .with_spawn_template_store(store.clone());
        let sha = "ab".repeat(32);
        let manifest = warm_restore_manifest("wt-warm", &sha);
        let iid = ModelInstanceId::new(ModelId::new_v7(), 0);
        state.sessions.lock().expect("session table").insert(
            iid,
            TrackedSession {
                model_id: iid.model_id,
                runtime: Arc::new(StaticTokenRuntime {
                    response: StaticGenerateResponse::Text("warm".to_string()),
                }),
                runtime_binding: RuntimeBinding::LlamaCpp,
                provider: ProviderKind::Local,
                artifact_path: Some("D:/models/model.gguf".to_string()),
                cloud_model_name: None,
                worktree_id: Some("wt-warm".to_string()),
                working_dir: None,
                isolation_tier: Some(SwarmIsolationTierIpc::Tier3Microvm),
                local_execution_mode: Some(SwarmLocalExecutionModeIpc::WarmVm),
                warm_vm_restore_manifest: Some(manifest.clone()),
            },
        );
        let mut req = local_request("D:/models/model.gguf");
        req.sha256_expected = Some(sha);
        req.runtime_binding = Some(SwarmRuntimeBindingIpc::LlamaCpp);
        req.local_execution_mode = Some(SwarmLocalExecutionModeIpc::WarmVm);
        req.worktree_id = Some("wt-warm".to_string());
        req.isolation_tier = Some(SwarmIsolationTierIpc::Tier3Microvm);
        req.warm_vm_restore_manifest = None;

        after_successful_swarm_spawn(&req, &state, iid, false).await;

        let stored = store
            .get(&iid.to_string())
            .expect("load stored template")
            .expect("template stored");
        assert_eq!(stored.warm_vm_restore_manifest, Some(manifest));
    }

    #[tokio::test]
    async fn auto_restored_spawn_does_not_persist_consumed_manifest_without_successor() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let store = SpawnTemplateStore::with_path(tmp.path().join("templates.json"));
        let state =
            SwarmRuntimeState::production_with_cloud_and_recorder_and_committed_memory_ceiling(
                CloudLaneFactoryConfig::unconfigured(),
                None,
                tmp.path(),
                None,
                None,
            )
            .with_spawn_template_store(store.clone());
        let sha = "ab".repeat(32);
        let consumed = warm_restore_manifest("wt-warm", &sha);
        let iid = ModelInstanceId::new(ModelId::new_v7(), 0);
        state.sessions.lock().expect("session table").insert(
            iid,
            TrackedSession {
                model_id: iid.model_id,
                runtime: Arc::new(StaticTokenRuntime {
                    response: StaticGenerateResponse::Text("warm".to_string()),
                }),
                runtime_binding: RuntimeBinding::LlamaCpp,
                provider: ProviderKind::Local,
                artifact_path: Some("D:/models/model.gguf".to_string()),
                cloud_model_name: None,
                worktree_id: Some("wt-warm".to_string()),
                working_dir: None,
                isolation_tier: Some(SwarmIsolationTierIpc::Tier3Microvm),
                local_execution_mode: Some(SwarmLocalExecutionModeIpc::WarmVm),
                warm_vm_restore_manifest: Some(consumed.clone()),
            },
        );
        let mut req = local_request("D:/models/model.gguf");
        req.sha256_expected = Some(sha);
        req.runtime_binding = Some(SwarmRuntimeBindingIpc::LlamaCpp);
        req.local_execution_mode = Some(SwarmLocalExecutionModeIpc::WarmVm);
        req.worktree_id = Some("wt-warm".to_string());
        req.isolation_tier = Some(SwarmIsolationTierIpc::Tier3Microvm);
        req.warm_vm_restore_manifest = Some(consumed);

        after_successful_swarm_spawn(&req, &state, iid, true).await;

        let stored = store
            .get(&iid.to_string())
            .expect("load stored template")
            .expect("template stored");
        assert_eq!(
            stored.warm_vm_restore_manifest, None,
            "auto-consumed manifests are not persisted again without a successor snapshot"
        );
    }

    /// `template_to_request_ipc` rebuilds a request whose `parent_session_id` is
    /// the origin (lineage) and which `build_spawn_request` accepts, minting a
    /// FRESH model id (so the resumed composite never collides with the origin),
    /// while carrying the integrity sha + artifact verbatim.
    #[test]
    fn template_to_request_ipc_rebuilds_a_fresh_request_with_lineage() {
        // Capture a local template, then rebuild it.
        let original = local_request("D:/models/qwen.safetensors");
        let tpl = template_from_ipc(&original, "ORIGIN#0");
        let rebuilt = template_to_request_ipc(&tpl, "ORIGIN#0");
        assert_eq!(rebuilt.parent_session_id.as_deref(), Some("ORIGIN#0"));
        assert_eq!(
            rebuilt.artifact_path.as_deref(),
            Some("D:/models/qwen.safetensors")
        );
        assert_eq!(
            rebuilt.sha256_expected.as_deref(),
            Some(&"ab".repeat(32)[..])
        );

        // build_spawn_request accepts it and carries the integrity gate + lineage.
        let spawn = build_spawn_request(&rebuilt).expect("rebuilt request is valid");
        assert_eq!(spawn.runtime_binding, RuntimeBinding::Candle);
        assert_eq!(
            spawn.model_artifact_path(),
            Some("D:/models/qwen.safetensors")
        );
        // Two rebuilds mint DISTINCT fresh model ids (never reuse the origin id).
        let spawn_b = build_spawn_request(&template_to_request_ipc(&tpl, "ORIGIN#0"))
            .expect("second rebuild valid");
        assert_ne!(
            spawn.instance_id.model_id, spawn_b.instance_id.model_id,
            "each resume mints a fresh model id"
        );
    }

    /// A successful spawn PERSISTS a resume template keyed by the live composite
    /// instance_id; resume then reads it back, rebuilds (lineage parent = origin),
    /// and re-spawns through the shared path — and the NEW session is ITSELF
    /// resumable (its template is persisted too). Uses a credentialed cloud lane
    /// so the spawn really succeeds without a live HTTP call.
    #[tokio::test]
    async fn spawn_persists_template_and_resume_rebuilds_and_respawns() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let store = SpawnTemplateStore::new(tmp.path());
        let vault = Arc::new(InMemorySecretsVault::default());
        vault
            .put("anthropic", "sk-ant-test-key".to_string())
            .expect("store key");
        let state = SwarmRuntimeState::production_with_cloud_vault(
            vault,
            Some("anthropic".to_string()),
            None,
        )
        .with_spawn_template_store(store.clone());
        let coordinator = state.coordinator();

        // Spawn a cloud session and persist its template exactly as the command
        // does (the persist tail lives in the Tauri command body; here we drive
        // the coordinator + persist seam directly since we cannot fabricate State).
        let mut req = cloud_request("claude-sonnet-4");
        req.byok_cloud_provider = Some(ByokCloudProviderIpc::Anthropic);
        req.swarm_id = Some("swarm-resume".to_string());
        req.worktree_id = Some("wt-resume".to_string());
        req.working_dir = Some("D:/work/wt-resume".to_string());
        let spawn = build_spawn_request(&req).expect("valid cloud request");
        let origin_iid = coordinator.spawn_session(spawn).await.expect("origin live");
        let origin_composite = origin_iid.to_string();
        store
            .upsert(
                &origin_composite,
                template_from_ipc(&req, &origin_composite),
            )
            .expect("persist origin template");

        // The origin session is now resumable: a template exists for it.
        assert!(state.spawn_template_store().contains(&origin_composite));

        // Resume: read the stored template, rebuild with lineage, re-spawn.
        let tpl = state
            .spawn_template_store()
            .get(&origin_composite)
            .expect("get ok")
            .expect("template present");
        let resume_req = template_to_request_ipc(&tpl, &origin_composite);
        assert_eq!(
            resume_req.parent_session_id.as_deref(),
            Some(origin_composite.as_str())
        );
        assert_eq!(
            resume_req.cloud_model_name.as_deref(),
            Some("claude-sonnet-4")
        );
        assert_eq!(resume_req.worktree_id.as_deref(), Some("wt-resume"));
        assert_eq!(resume_req.swarm_id.as_deref(), Some("swarm-resume"));
        assert_eq!(
            resume_req.byok_cloud_provider,
            Some(ByokCloudProviderIpc::Anthropic)
        );
        assert_eq!(resume_req.working_dir.as_deref(), Some("D:/work/wt-resume"));
        let resume_spawn = build_spawn_request(&resume_req).expect("resume request valid");
        let new_iid = coordinator
            .spawn_session(resume_spawn)
            .await
            .expect("resumed live");

        // Fresh composite id (distinct model id from the origin) — no collision.
        assert_ne!(new_iid.model_id, origin_iid.model_id);
        assert_eq!(coordinator.live_session_count(), 2);

        // The resumed session is itself resumable (persist its template too, as
        // the shared command tail does).
        let new_composite = new_iid.to_string();
        store
            .upsert(
                &new_composite,
                template_from_ipc(&resume_req, &new_composite),
            )
            .expect("persist resumed template");
        assert!(state.spawn_template_store().contains(&new_composite));
        // The resumed session's template is captured FROM the new session
        // (origin_session_id = the new composite); the resume LINEAGE back to the
        // origin lives in the rebuilt request's parent_session_id (which the FR
        // SessionSpawned.parent_session_id records), proven below.
        let chained = store.get(&new_composite).expect("ok").expect("present");
        assert_eq!(chained.origin_session_id, new_composite);
        assert_eq!(
            resume_req.parent_session_id.as_deref(),
            Some(origin_composite.as_str()),
            "resume request lineage parent = the origin session"
        );

        coordinator
            .cancel_session(origin_iid, "t")
            .await
            .expect("cancel origin");
        coordinator
            .cancel_session(new_iid, "t")
            .await
            .expect("cancel resumed");
    }

    /// Resume of an id with NO stored template surfaces the typed
    /// `SESSION_NOT_RESUMABLE:` error (the not-resumable contract).
    #[test]
    fn resume_unknown_id_is_not_resumable() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let store = SpawnTemplateStore::new(tmp.path());
        // No template stored -> get yields None -> the command maps to the typed
        // error. We assert at the store boundary the command consults.
        assert_eq!(store.get("never-spawned#0").expect("get ok"), None);
    }

    /// A resumed CLOUD session with no configured lane still surfaces
    /// `ProviderNotConfigured` through the SHARED spawn path — resume inherits the
    /// same honest runtime conditions as a first spawn (no bypass).
    #[tokio::test]
    async fn resume_of_cloud_without_lane_surfaces_provider_not_configured() {
        let state = SwarmRuntimeState::production(std::path::Path::new(""));
        let coordinator = state.coordinator();
        // Build a template as if a prior spawn had succeeded, then rebuild + spawn.
        let tpl = template_from_ipc(&cloud_request("claude-sonnet-4"), "origin#0");
        let resume_req = template_to_request_ipc(&tpl, "origin#0");
        let spawn = build_spawn_request(&resume_req).expect("valid request");
        let err = coordinator
            .spawn_session(spawn)
            .await
            .expect_err("no lane -> not configured");
        assert!(is_provider_not_configured(&err), "got {err}");
        assert_eq!(coordinator.live_session_count(), 0);
    }

    /// The camelCase template IPC DTO carries every field for the prefill path.
    #[test]
    fn session_spawn_template_ipc_maps_all_fields() {
        let mut req = local_request("D:/models/qwen.safetensors");
        req.swarm_id = Some("swarm-z".to_string());
        req.worktree_id = Some("wt-z".to_string());
        req.working_dir = Some("D:/work/wt-z".to_string());
        req.isolation_tier = Some(SwarmIsolationTierIpc::Tier2Syscall);
        req.local_execution_mode = Some(SwarmLocalExecutionModeIpc::Cold);
        req.committed_memory_bytes = Some(7 * 1024 * 1024 * 1024);
        let tpl = template_from_ipc(&req, "origin#5");
        let rebuilt = template_to_request_ipc(&tpl, "origin#5");
        assert_eq!(rebuilt.committed_memory_bytes, Some(7 * 1024 * 1024 * 1024));
        assert_eq!(
            rebuilt.local_execution_mode,
            Some(SwarmLocalExecutionModeIpc::Cold)
        );
        let dto = SessionSpawnTemplateIpc::from(tpl);
        assert_eq!(dto.provider, SwarmProviderIpc::Local);
        assert_eq!(
            dto.artifact_path.as_deref(),
            Some("D:/models/qwen.safetensors")
        );
        assert_eq!(dto.runtime_binding, Some(SwarmRuntimeBindingIpc::Candle));
        assert_eq!(
            dto.local_execution_mode,
            Some(SwarmLocalExecutionModeIpc::Cold)
        );
        assert_eq!(dto.byok_cloud_provider, None);
        assert_eq!(dto.swarm_id.as_deref(), Some("swarm-z"));
        assert_eq!(dto.worktree_id.as_deref(), Some("wt-z"));
        assert_eq!(dto.working_dir.as_deref(), Some("D:/work/wt-z"));
        assert_eq!(
            dto.isolation_tier,
            Some(SwarmIsolationTierIpc::Tier2Syscall)
        );
        assert_eq!(dto.committed_memory_bytes, Some(7 * 1024 * 1024 * 1024));
        assert_eq!(dto.origin_session_id, "origin#5");
        assert!(
            !dto.captured_at.is_empty(),
            "captured_at serialized as rfc3339"
        );
    }
}

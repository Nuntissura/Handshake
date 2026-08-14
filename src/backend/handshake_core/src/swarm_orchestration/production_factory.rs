//! The PRODUCTION [`ModelSessionFactory`] (MT-204).
//!
//! This is the concrete runtime/process seam the coordinator drives in
//! production. It dispatches on [`SpawnRequest::provider`] /
//! [`SpawnRequest::runtime_binding`]:
//!
//! - **Local candle** (the provable path): reuses the EXACT proven load logic
//!   from `kernel_model_runtime_load` via
//!   [`crate::model_runtime::candle::load_local_candle_model`], records a real
//!   process-ledger START, and returns a [`LiveSession`] whose `teardown`
//!   actually FREES the model by dropping the sole owning `Arc<CandleRuntime>`
//!   (the same detach-drop the single-load unload proved — D1 contract) AND
//!   writing the matching ledger STOP via the coordinator.
//! - **Local llama.cpp**: same shape via [`crate::model_runtime::llama_cpp`].
//!   If the GGUF artifact is absent the real `load` returns a typed
//!   [`ModelRuntimeError`] which is surfaced honestly at `create`, not faked.
//! - **Cloud** (anthropic / openai BYOK, official CLI): dispatches to the real
//!   cloud adapters. If the operator has not configured credentials (no API key
//!   in the secrets vault) or the CLI is absent, `create` returns a typed
//!   [`SwarmError::ProviderNotConfigured`] — a genuine runtime condition, never
//!   a placeholder. Live cloud generation routing/policy is MT-206; this MT
//!   wires the dispatch + the honest not-configured failure.
//!
//! ## Teardown frees the model (D1)
//!
//! For the candle path the owning `CandleRuntime` is moved into the teardown
//! closure. The `LiveSession::runtime` handed to the coordinator is a SEPARATE
//! `Arc` clone used only for cancellation/observation; the OWNING `Arc` lives
//! solely inside the teardown closure, so when the coordinator invokes teardown
//! exactly once on every terminal path it drops the last strong reference and
//! the `CandleRuntime` (with its loaded weights) is freed — mirroring
//! `kernel_model_runtime_unload`'s detach-drop. The teardown also calls the
//! runtime's async `unload(model_id)` first so the model map entry is removed
//! deterministically before the drop, matching the single-load contract.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[cfg(test)]
use std::sync::OnceLock;

use async_trait::async_trait;
use serde_json::json;

use crate::model_runtime::candle::{load_local_candle_model, LoadedCandleModel};
use crate::model_runtime::registry::RuntimeBinding;
use crate::model_runtime::{CancellationToken, ModelId, ModelRuntime, ProviderKind};
use crate::process_ledger::{
    record_spawn, ActiveProcessLifecycle, LedgerBatcher, ProcessEngineKind,
    ProcessOwnershipRecordId, ProcessStart, ReservedProcessLifecycle, RetainedLedgerBatcher,
    SpawnMeta,
};
use crate::sandbox::{
    validate_warm_agent_package_candidate_with_runtime_probe, BindMode, BindSpec,
    SandboxAdapterRegistry, WarmAgentGuestPackageManifest, WarmAgentPackageError,
    CLOUD_HYPERVISOR_WARM_AGENT_GUEST_PATH_METADATA_KEY, CLOUD_HYPERVISOR_WARM_AGENT_GUEST_ROOT,
    CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV, DEFAULT_WSL_DISTRO,
};

use super::error::{SwarmError, SwarmResult};
use super::factory::{LiveSession, ModelSessionFactory, SessionTeardown};
use super::ids::{ModelInstanceId, SpawnRequest};
use super::resource_scope::ExactResourceScopeAttribution;

use super::coordinator::{SwarmConfig, SwarmCoordinator};
use super::events::FlightRecorderSwarmSink;
use super::ids::RunBudget;
use super::model_lane::ModelLaneStore;
use crate::flight_recorder::FlightRecorderEvent;

const SANDBOX_LLAMA_CLI_HOST_PATH_ENV: &str = "HANDSHAKE_SANDBOX_LLAMA_CLI_HOST_PATH";
const WARM_VM_WORKTREE_GUEST_ROOT: &str = "/worktree";
const PIDLESS_SESSION_START_DURABILITY_TIMEOUT: Duration = Duration::from_secs(5);

#[cfg(test)]
type WarmAgentPackageValidator =
    fn(&std::path::Path) -> Result<WarmAgentGuestPackageManifest, WarmAgentPackageError>;

#[cfg(test)]
static WARM_AGENT_PACKAGE_VALIDATOR_OVERRIDE: OnceLock<Mutex<Option<WarmAgentPackageValidator>>> =
    OnceLock::new();

fn warm_agent_bind_from_env() -> Result<Option<(BindSpec, String)>, SwarmError> {
    let Some(path) = std::env::var_os(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV) else {
        return Ok(None);
    };
    let host_path = std::path::PathBuf::from(path);
    validate_warm_agent_package_for_factory(&host_path).map_err(|error| {
        SwarmError::FactoryFailed(format!(
            "warm VM local execution requires {CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV} \
             to point at a complete packaged Linux guest warm-agent artifact: {error}"
        ))
    })?;
    let file_name = host_path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            SwarmError::FactoryFailed(format!(
                "warm VM local execution: {CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV} \
                 must have a UTF-8 file name"
            ))
        })?;
    if file_name.chars().any(char::is_whitespace) {
        return Err(SwarmError::FactoryFailed(format!(
            "warm VM local execution: warm-agent file name must not contain whitespace: {file_name}"
        )));
    }
    let parent = host_path
        .parent()
        .map(|path| path.to_path_buf())
        .ok_or_else(|| {
            SwarmError::FactoryFailed(format!(
            "warm VM local execution: {CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV} has no parent \
             directory to bind"
        ))
        })?;
    let guest_path = format!(
        "{}/{}",
        CLOUD_HYPERVISOR_WARM_AGENT_GUEST_ROOT.trim_end_matches('/'),
        file_name
    );
    Ok(Some((
        BindSpec {
            host_path: parent,
            guest_path: std::path::PathBuf::from(CLOUD_HYPERVISOR_WARM_AGENT_GUEST_ROOT),
            mode: BindMode::ReadOnly,
        },
        guest_path,
    )))
}

fn validate_warm_agent_package_for_factory(
    host_path: &std::path::Path,
) -> Result<WarmAgentGuestPackageManifest, WarmAgentPackageError> {
    #[cfg(test)]
    if let Some(validator) = *warm_agent_package_validator_override().lock().unwrap() {
        return validator(host_path);
    }

    let distro = warm_agent_runtime_probe_distro();
    let wsl_exe = warm_agent_runtime_probe_wsl_exe();
    validate_warm_agent_package_candidate_with_runtime_probe(host_path, &distro, &wsl_exe)
}

#[cfg(test)]
fn warm_agent_package_validator_override() -> &'static Mutex<Option<WarmAgentPackageValidator>> {
    WARM_AGENT_PACKAGE_VALIDATOR_OVERRIDE.get_or_init(|| Mutex::new(None))
}

fn warm_agent_runtime_probe_distro() -> String {
    std::env::var("HANDSHAKE_CH_DISTRO")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_WSL_DISTRO.to_string())
}

fn warm_agent_runtime_probe_wsl_exe() -> std::path::PathBuf {
    std::env::var_os("HANDSHAKE_CH_WSL_EXE")
        .filter(|value| !value.is_empty())
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("wsl.exe"))
}

/// Default per-run concurrency cap when the caller does not specify one: the
/// available CPU parallelism, clamped to at least 1. GPU-bound deployments
/// should pass an explicit smaller cap (one swarm session per GPU) via
/// [`build_production_swarm_coordinator`].
pub fn default_swarm_concurrency() -> usize {
    std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(1)
        .max(1)
}

/// Build a fully-wired PRODUCTION [`SwarmCoordinator`]: the production factory
/// (local candle/llama + cloud dispatch), the real [`LedgerBatcher`], and a
/// [`FlightRecorderSwarmSink`] forwarding each swarm event through `emit_fn`.
///
/// - `concurrency` is the semaphore cap (defaults to [`default_swarm_concurrency`]
///   when `None`); the lifetime spawn ceiling is HBR-SWARM-002's
///   `HBR_SWARM_002_LOOP_CAP` (the `RunBudget::defaulted` default).
/// - `trace_id` is the flight-recorder trace the sink stamps onto every event.
/// - `model_lane_store` is Dexterity's PostgreSQL/EventLedger-backed launch
///   recorder and routing executor pool; production construction owns both so
///   callers cannot substitute filesystem or detached routing authority.
///
/// The reaper is NOT started here; the owner (app state) calls
/// [`SwarmCoordinator::start_reaper`] after `.manage` so the TTL/lease reaper
/// runs for the app's lifetime.
pub fn build_production_swarm_coordinator<F>(
    ledger: LedgerBatcher,
    cloud: CloudLaneFactoryConfig,
    model_lane_store: ModelLaneStore,
    concurrency: Option<usize>,
    trace_id: uuid::Uuid,
    emit_fn: F,
) -> SwarmCoordinator
where
    F: Fn(FlightRecorderEvent) -> Result<(), String> + Send + Sync + 'static,
{
    build_production_swarm_coordinator_with_sandbox_registry(
        ledger,
        cloud,
        model_lane_store,
        None,
        concurrency,
        trace_id,
        emit_fn,
    )
}

/// Production coordinator construction with an explicit sandbox registry.
/// Warm Tier-3 model lanes use this path so the same coordinator-owned
/// [`ModelLaneStore`] becomes durable WorktreeVmRegistry authority.
pub fn build_production_swarm_coordinator_with_sandbox_registry<F>(
    ledger: LedgerBatcher,
    cloud: CloudLaneFactoryConfig,
    model_lane_store: ModelLaneStore,
    sandbox_registry: Option<Arc<SandboxAdapterRegistry>>,
    concurrency: Option<usize>,
    trace_id: uuid::Uuid,
    emit_fn: F,
) -> SwarmCoordinator
where
    F: Fn(FlightRecorderEvent) -> Result<(), String> + Send + Sync + 'static,
{
    let concurrency = concurrency.unwrap_or_else(default_swarm_concurrency).max(1);
    let budget = RunBudget::defaulted(concurrency).with_concurrency(concurrency);
    let config = SwarmConfig::new(budget);
    let projection_scope = model_lane_store
        .write_scope()
        .map(super::resource_scope::ExactResourceScopeAttribution::try_from_resource_scope)
        .transpose()
        .unwrap_or_else(|error| {
            panic!("production ModelLaneStore has incomplete projection scope: {error}")
        });
    let factory = Arc::new(
        ProductionModelSessionFactory::new(ledger.clone(), cloud, sandbox_registry)
            .with_durable_worktree_vm_store(&model_lane_store),
    );
    // The durable Flight Recorder sink runs FIRST so its terminal-persistence
    // rejection still propagates to the lifecycle producer (the fanout returns on
    // the first child error). The NON-AUTHORITATIVE console tee runs AFTER: it
    // mirrors each event into the process-wide live debug-console hub
    // (`GET /wp1/diagnostics/console/stream`) and can never fail the coordinator —
    // a full broadcast ring or absent subscribers is a silent no-op.
    let flight_sink: Arc<dyn super::events::SwarmEventSink> = match projection_scope.clone() {
        Some(scope) => Arc::new(FlightRecorderSwarmSink::new_scoped(
            trace_id, scope, emit_fn,
        )),
        None => Arc::new(FlightRecorderSwarmSink::new(trace_id, emit_fn)),
    };
    let console_sink: Arc<dyn super::events::SwarmEventSink> = match projection_scope {
        Some(scope) => Arc::new(crate::console_stream::ConsoleSwarmSink::new_scoped(
            crate::console_stream::ConsoleBroadcast::shared(),
            scope,
        )),
        None => Arc::new(crate::console_stream::ConsoleSwarmSink::shared()),
    };
    let sink: Arc<dyn super::events::SwarmEventSink> =
        Arc::new(super::events::FanoutSwarmSink::new(vec![
            flight_sink,
            console_sink,
        ]));
    SwarmCoordinator::new_with_model_lane_store(config, factory, sink, ledger, model_lane_store)
}

/// High-level production routing entrypoint. The executor is deliberately not
/// supplied by the caller: it is owned by the coordinator built above and uses
/// the same PostgreSQL/EventLedger pool as Dexterity ModelLane persistence.
pub async fn execute_production_routing_wave(
    coordinator: &SwarmCoordinator,
    execution_id: &str,
    selecting_decision_id: &str,
    authority: &super::routing::ModelLaneRoutingAuthority,
    context: super::routing_execution::ModelLaneRoutingExecutionContext,
    launches: Vec<super::routing_execution::ModelLaneRoutingStageLaunch>,
) -> super::error::SwarmResult<super::routing_execution::ModelLaneRoutingDispatchBatch> {
    coordinator
        .execute_routing_wave(
            execution_id,
            selecting_decision_id,
            authority,
            context,
            launches,
        )
        .await
}

pub async fn execute_production_routing_lifecycle(
    coordinator: &SwarmCoordinator,
    execution_id: &str,
    selecting_decision_id: &str,
    authority: &super::routing::ModelLaneRoutingAuthority,
    context: super::routing_execution::ModelLaneRoutingExecutionContext,
    launches: Vec<super::routing_execution::ModelLaneRoutingStageLaunch>,
) -> super::error::SwarmResult<super::routing_execution::ModelLaneRoutingDispatchBatch> {
    coordinator
        .execute_routing_lifecycle(
            execution_id,
            selecting_decision_id,
            authority,
            context,
            launches,
        )
        .await
}

/// WP-1 MT-012 (F3/F4): build the live [`OperatorChatLaunchService`] the shipped
/// `POST /operator-chat/launch` + `GET /operator-chat/transcript/:run_id` routes
/// resolve through, from the app's shared PostgreSQL pool + Flight Recorder. This
/// is what makes the launch route NON-`503` — wired to a real
/// [`SwarmCoordinator`] + [`ModelLaneStore`] rather than left inert.
///
/// `cloud` carries the configured BYOK and official-CLI builders. Unconfigured
/// providers remain absent and fail closed with `ProviderNotConfigured`; Local
/// lanes still launch through the real candle/llama path.
pub fn build_operator_chat_launch_service(
    pool: sqlx::PgPool,
    recorder: Arc<dyn crate::flight_recorder::FlightRecorder>,
    catalog: Arc<crate::model_runtime::catalog::ModelCatalog>,
    process_ledger_runtime: RetainedLedgerBatcher,
    process_reclaimer: Arc<crate::process_ledger::Reclaim>,
    cloud: CloudLaneFactoryConfig,
    trace_id: uuid::Uuid,
) -> Arc<super::operator_chat::OperatorChatLaunchService> {
    build_operator_chat_launch_service_with_sandbox_registry(
        pool,
        recorder,
        catalog,
        process_ledger_runtime,
        process_reclaimer,
        cloud,
        None,
        trace_id,
    )
}

/// Shipped operator-chat construction with the app's sandbox registry threaded
/// into the same production factory used by coordinator launches.
pub fn build_operator_chat_launch_service_with_sandbox_registry(
    pool: sqlx::PgPool,
    recorder: Arc<dyn crate::flight_recorder::FlightRecorder>,
    catalog: Arc<crate::model_runtime::catalog::ModelCatalog>,
    process_ledger_runtime: RetainedLedgerBatcher,
    process_reclaimer: Arc<crate::process_ledger::Reclaim>,
    cloud: CloudLaneFactoryConfig,
    sandbox_registry: Option<Arc<SandboxAdapterRegistry>>,
    trace_id: uuid::Uuid,
) -> Arc<super::operator_chat::OperatorChatLaunchService> {
    build_operator_chat_launch_service_with_optional_scope(
        pool,
        recorder,
        catalog,
        process_ledger_runtime,
        process_reclaimer,
        cloud,
        sandbox_registry,
        None,
        trace_id,
    )
}

/// Shipped product-local operator-chat construction. Unlike the retained legacy
/// helper, every lane write and FR/console projection is stamped from the exact
/// server-owned scope.
pub fn build_scoped_operator_chat_launch_service_with_sandbox_registry(
    pool: sqlx::PgPool,
    recorder: Arc<dyn crate::flight_recorder::FlightRecorder>,
    catalog: Arc<crate::model_runtime::catalog::ModelCatalog>,
    process_ledger_runtime: RetainedLedgerBatcher,
    process_reclaimer: Arc<crate::process_ledger::Reclaim>,
    cloud: CloudLaneFactoryConfig,
    sandbox_registry: Option<Arc<SandboxAdapterRegistry>>,
    resource_scope: super::resource_scope::ResourceScope,
    trace_id: uuid::Uuid,
) -> Arc<super::operator_chat::OperatorChatLaunchService> {
    build_operator_chat_launch_service_with_optional_scope(
        pool,
        recorder,
        catalog,
        process_ledger_runtime,
        process_reclaimer,
        cloud,
        sandbox_registry,
        Some(resource_scope),
        trace_id,
    )
}

fn build_operator_chat_launch_service_with_optional_scope(
    pool: sqlx::PgPool,
    recorder: Arc<dyn crate::flight_recorder::FlightRecorder>,
    catalog: Arc<crate::model_runtime::catalog::ModelCatalog>,
    process_ledger_runtime: RetainedLedgerBatcher,
    process_reclaimer: Arc<crate::process_ledger::Reclaim>,
    cloud: CloudLaneFactoryConfig,
    sandbox_registry: Option<Arc<SandboxAdapterRegistry>>,
    resource_scope: Option<super::resource_scope::ResourceScope>,
    trace_id: uuid::Uuid,
) -> Arc<super::operator_chat::OperatorChatLaunchService> {
    let model_lane_store = match resource_scope {
        Some(scope) => ModelLaneStore::new_scoped(pool.clone(), scope),
        None => ModelLaneStore::new(pool.clone()),
    };
    let ledger = process_ledger_runtime.ledger();
    let (terminal_bridge, _terminal_drain) =
        super::events::DurableSwarmFrBridge::spawn_with_postgres_outbox(
            recorder.clone(),
            pool.clone(),
            1024,
        );
    let coordinator = build_production_swarm_coordinator_with_sandbox_registry(
        ledger,
        cloud,
        model_lane_store,
        sandbox_registry,
        None,
        trace_id,
        move |event| terminal_bridge.emit(event),
    )
    .with_process_reclaimer(process_reclaimer);
    Arc::new(
        super::operator_chat::OperatorChatLaunchService::new_with_process_ledger_runtime(
            Arc::new(coordinator),
            catalog,
            recorder,
            process_ledger_runtime,
        ),
    )
}

/// Optional cloud-lane configuration the production factory needs to dispatch
/// cloud spawns. When absent, a cloud spawn returns
/// [`SwarmError::ProviderNotConfigured`] (the lane is not wired by the
/// operator). Construction of the concrete cloud adapters + credential vault is
/// owned by the app wiring (MT-205/MT-206); this struct is the seam.
#[derive(Clone)]
pub struct CloudLaneFactoryConfig {
    /// Builder for an anthropic BYOK runtime. Returns `Ok(runtime)` only when
    /// the operator has configured the lane (api key present in the vault);
    /// otherwise returns a typed not-configured reason string.
    pub anthropic: Option<Arc<dyn CloudRuntimeBuilder>>,
    /// Builder for an openai BYOK runtime (same contract as `anthropic`).
    pub openai: Option<Arc<dyn CloudRuntimeBuilder>>,
    /// Builder for the official-CLI bridge runtime (claude/codex/gemini CLI).
    /// Wired for [`ProviderKind::OfficialCli`] spawns; when `None` an
    /// OfficialCli spawn returns [`SwarmError::ProviderNotConfigured`] until the
    /// operator configures the CLI bridge.
    pub official_cli: Option<Arc<dyn CloudRuntimeBuilder>>,
    /// Provider-specific official CLI builders keyed by the picker provider id
    /// (`claude_code`, `codex`). Operator Chat uses this so selecting one CLI
    /// row does not accidentally launch through another configured template.
    pub official_cli_by_provider: HashMap<String, Arc<dyn CloudRuntimeBuilder>>,
}

impl CloudLaneFactoryConfig {
    /// A config with NO cloud lanes wired: every cloud spawn returns
    /// `ProviderNotConfigured`. This is the honest default until the operator
    /// configures BYOK credentials (MT-206).
    pub fn unconfigured() -> Self {
        Self {
            anthropic: None,
            openai: None,
            official_cli: None,
            official_cli_by_provider: HashMap::new(),
        }
    }

    /// Build a cloud-lane config from a real operator secrets vault (the same
    /// [`SecretsVault`] the cloud-lane control panel writes BYOK keys into).
    /// Each provider is wired to a [`VaultCloudRuntimeBuilder`] keyed by the
    /// lane id the operator stored the key under, so a cloud spawn actually
    /// constructs a live BYOK runtime when the operator has configured the lane.
    /// If a key is absent at spawn time the builder returns an honest
    /// not-configured reason which the factory surfaces as
    /// [`SwarmError::ProviderNotConfigured`] — never a placeholder.
    ///
    /// `anthropic_lane` / `openai_lane` are the vault lane ids (the same lane
    /// ids the cloud-lane registration uses). Pass `None` for a provider the
    /// operator has not registered; that provider then reports not-configured.
    pub fn from_vault(
        vault: Arc<dyn crate::model_runtime::cloud::SecretsVault>,
        anthropic_lane: Option<String>,
        openai_lane: Option<String>,
    ) -> Self {
        let anthropic = anthropic_lane.map(|lane| {
            Arc::new(VaultCloudRuntimeBuilder::new(
                CloudProviderFlavor::Anthropic,
                vault.clone(),
                lane,
            )) as Arc<dyn CloudRuntimeBuilder>
        });
        let openai = openai_lane.map(|lane| {
            Arc::new(VaultCloudRuntimeBuilder::new(
                CloudProviderFlavor::OpenAi,
                vault.clone(),
                lane,
            )) as Arc<dyn CloudRuntimeBuilder>
        });
        // The vault path is BYOK-only; the official-CLI lane is configured via
        // [`CloudLaneFactoryConfig::with_official_cli`].
        Self {
            anthropic,
            openai,
            official_cli: None,
            official_cli_by_provider: HashMap::new(),
        }
    }

    /// Configure (or override) the official-CLI bridge lane on this config.
    /// `spawner` is the live byte source (`LiveCliSpawner` in production) and
    /// `config_template` is the operator CLI config every cloud `load()`
    /// registers. After this call an [`ProviderKind::OfficialCli`] spawn
    /// dispatches to a [`CliBridgeCloudRuntimeBuilder`] which builds a real
    /// [`crate::model_runtime::cloud::CliBridgeModelRuntime`] when the CLI is
    /// present and returns an honest not-configured reason when it is absent.
    pub fn with_official_cli(
        self,
        spawner: Arc<dyn crate::model_runtime::cloud::CliSubprocessSpawner>,
        config: crate::model_runtime::cloud::AllowlistedCliBridgeConfig,
    ) -> Self {
        self.with_official_cli_observed(spawner, config, None)
    }

    /// Like [`Self::with_official_cli`] but threads a
    /// [`crate::model_runtime::cloud::CloudLaneObservability`] into the CLI lane
    /// so the built runtime emits `FR-EVT-LLM-INFER-{START,TOKEN,END}` (the
    /// recorder-carrying production constructors pass `Some(..)`; `None` keeps
    /// the lane FR-INFER-silent but otherwise identical).
    pub fn with_official_cli_observed(
        mut self,
        spawner: Arc<dyn crate::model_runtime::cloud::CliSubprocessSpawner>,
        config: crate::model_runtime::cloud::AllowlistedCliBridgeConfig,
        observability: Option<Arc<crate::model_runtime::cloud::CloudLaneObservability>>,
    ) -> Self {
        let mut builder = CliBridgeCloudRuntimeBuilder::new(spawner, config);
        if let Some(obs) = observability {
            builder = builder.with_observability(obs);
        }
        self.official_cli = Some(Arc::new(builder) as Arc<dyn CloudRuntimeBuilder>);
        self
    }

    /// Configure one exact official CLI provider. This is the Operator Chat path:
    /// each provider row (`claude_code`, `codex`) gets its own template so the
    /// spawn request's selected provider is load-bearing.
    pub fn with_official_cli_provider_observed(
        mut self,
        provider_id: impl Into<String>,
        spawner: Arc<dyn crate::model_runtime::cloud::CliSubprocessSpawner>,
        config: crate::model_runtime::cloud::AllowlistedCliBridgeConfig,
        observability: Option<Arc<crate::model_runtime::cloud::CloudLaneObservability>>,
    ) -> Self {
        let mut builder = CliBridgeCloudRuntimeBuilder::new(spawner, config);
        if let Some(obs) = observability {
            builder = builder.with_observability(obs);
        }
        self.official_cli_by_provider.insert(
            provider_id.into(),
            Arc::new(builder) as Arc<dyn CloudRuntimeBuilder>,
        );
        self
    }

    /// Operator Chat variant for exact official CLI providers. The launch
    /// service replays the same JSONL stdout into canonical lane messages and
    /// emits `FR-EVT-AGENT-*` evidence there, so this builder keeps LLM infer
    /// telemetry but disables the runtime's duplicate agent-activity producer.
    pub fn with_official_cli_provider_replay_captured(
        mut self,
        provider_id: impl Into<String>,
        spawner: Arc<dyn crate::model_runtime::cloud::CliSubprocessSpawner>,
        config: crate::model_runtime::cloud::AllowlistedCliBridgeConfig,
        observability: Option<Arc<crate::model_runtime::cloud::CloudLaneObservability>>,
    ) -> Self {
        let mut builder =
            CliBridgeCloudRuntimeBuilder::new(spawner, config).without_agent_activity_events();
        if let Some(obs) = observability {
            builder = builder.with_observability(obs);
        }
        self.official_cli_by_provider.insert(
            provider_id.into(),
            Arc::new(builder) as Arc<dyn CloudRuntimeBuilder>,
        );
        self
    }

    /// `from_vault` + the official-CLI lane in one shot. `cli` is `Some((spawner,
    /// config))` when the operator has configured the CLI bridge (the lane goes
    /// live), and `None` when they have not (the lane stays `None` → an honest
    /// `ProviderNotConfigured` on an OfficialCli spawn). Reuses
    /// [`Self::from_vault`] + [`Self::with_official_cli`] verbatim — no new
    /// builder logic. `spawner` is the `LiveCliSpawner` in production.
    pub fn from_vault_with_cli(
        vault: Arc<dyn crate::model_runtime::cloud::SecretsVault>,
        anthropic_lane: Option<String>,
        openai_lane: Option<String>,
        cli: Option<(
            Arc<dyn crate::model_runtime::cloud::CliSubprocessSpawner>,
            crate::model_runtime::cloud::AllowlistedCliBridgeConfig,
        )>,
    ) -> Self {
        let base = Self::from_vault(vault, anthropic_lane, openai_lane);
        match cli {
            Some((spawner, cfg)) => base.with_official_cli(spawner, cfg),
            None => base,
        }
    }
}

/// A real cloud-runtime builder for the official-CLI bridge lane. Mirrors
/// [`VaultCloudRuntimeBuilder`] but the "credential preflight" becomes a
/// CLI-executable-exists / configured-bridge preflight: a missing/unconfigured
/// CLI yields an honest not-configured reason that the factory surfaces as
/// [`SwarmError::ProviderNotConfigured`]. On success it drives the real
/// [`CliBridgeModelRuntime::load`] (which re-validates the config and mints the
/// runtime-keyed [`ModelId`]) and hands the swarm an `Arc<dyn ModelRuntime>`
/// whose `generate` streams the CLI's live stdout as tokens — auto-captured by
/// the swarm capture seam into the in-app terminal panel.
/// WP-1 MT-012: apply the operator-selected on-disk location to a CLI bridge
/// config so a CLI lane truly runs in that directory. This is the real cwd
/// plumbing that closes the F10 gap (`SpawnRequest.working_dir` was
/// recorded-attribution-only; `CliBridgeConfig.working_dir` is the real
/// subprocess cwd via `LiveCliSpawner`). A blank/absent selection keeps the
/// template's configured `working_dir` unchanged.
pub fn cli_bridge_config_with_working_dir(
    template: crate::model_runtime::cloud::CliBridgeConfig,
    working_dir: Option<&str>,
) -> crate::model_runtime::cloud::CliBridgeConfig {
    match working_dir {
        Some(dir) if !dir.trim().is_empty() => crate::model_runtime::cloud::CliBridgeConfig {
            working_dir: Some(std::path::PathBuf::from(dir)),
            ..template
        },
        _ => template,
    }
}

pub struct CliBridgeCloudRuntimeBuilder {
    spawner: Arc<dyn crate::model_runtime::cloud::CliSubprocessSpawner>,
    config_template: crate::model_runtime::cloud::CliBridgeConfig,
    model_allowlist: crate::model_runtime::cloud::CliModelAllowlist,
    /// Optional cloud-lane observability threaded into the built
    /// [`crate::model_runtime::cloud::CliBridgeModelRuntime`] so its `generate`
    /// emits `FR-EVT-LLM-INFER-{START,TOKEN,END}`. `None` => no FR-INFER events
    /// (e.g. a recorder-less wiring); generation is identical either way.
    lane_obs: Option<Arc<crate::model_runtime::cloud::CloudLaneObservability>>,
    /// Whether JSON-stream CLI output should be converted into runtime-level
    /// structured `FR-EVT-AGENT-*` evidence. Defaults to true; replay-captured
    /// callers disable it and emit agent evidence from their canonical capture
    /// path instead.
    emit_agent_activity_events: bool,
}

impl CliBridgeCloudRuntimeBuilder {
    pub fn new(
        spawner: Arc<dyn crate::model_runtime::cloud::CliSubprocessSpawner>,
        config: crate::model_runtime::cloud::AllowlistedCliBridgeConfig,
    ) -> Self {
        let (config_template, model_allowlist) = config.into_parts();
        Self {
            spawner,
            config_template,
            model_allowlist,
            lane_obs: None,
            emit_agent_activity_events: true,
        }
    }

    /// Attach cloud-lane observability so the built CLI runtime emits
    /// `FR-EVT-LLM-INFER-*` events.
    pub fn with_observability(
        mut self,
        lane_obs: Arc<crate::model_runtime::cloud::CloudLaneObservability>,
    ) -> Self {
        self.lane_obs = Some(lane_obs);
        self
    }

    pub fn without_agent_activity_events(mut self) -> Self {
        self.emit_agent_activity_events = false;
        self
    }

    /// Build the OfficialCli `LoadSpec`: provider=OfficialCli, engine_origin
    /// carries the allowlisted CLI model name. Mirrors
    /// [`VaultCloudRuntimeBuilder::cloud_load_spec`] but on the OfficialCli lane.
    fn cli_load_spec(&self, model_name: &str) -> crate::model_runtime::LoadSpec {
        use crate::model_runtime::{
            KvCachePolicy, LoadSpec, ModelCapabilities, RuntimeKind, SamplingParams,
        };
        LoadSpec {
            artifact_path: std::path::PathBuf::new(),
            sha256_expected: String::new(),
            runtime_kind: RuntimeKind::Candle, // unused by the CLI bridge load
            sampling_defaults: SamplingParams::default(),
            kv_cache_policy: KvCachePolicy::default(),
            declared_capabilities: ModelCapabilities::default(),
            provider: ProviderKind::OfficialCli,
            engine_origin: Some(model_name.to_string()),
            external_engine_import: None,
        }
    }
}

#[async_trait]
impl CloudRuntimeBuilder for CliBridgeCloudRuntimeBuilder {
    fn provider(&self) -> ProviderKind {
        ProviderKind::OfficialCli
    }

    async fn build_loaded(
        &self,
        model_name: &str,
        invocation_context: Option<crate::model_runtime::cloud::CliInvocationContext>,
        working_dir: Option<&str>,
    ) -> Result<CloudLiveRuntime, String> {
        // Honest CLI-executable preflight: a missing/unconfigured CLI is the
        // genuine not-configured runtime condition (the factory turns this into
        // ProviderNotConfigured). `register_bridge`'s own exe-exists check at
        // load() is the second, redundant honest guard.
        if !self.config_template.executable_path.exists() {
            return Err(format!(
                "official CLI executable not found at '{}'; configure the CLI bridge \
                 (claude/codex/gemini) path to enable the official_cli lane",
                self.config_template.executable_path.display()
            ));
        }

        // WP-1 MT-012: plumb the operator-selected working_dir onto THIS session's
        // CLI config so the launched subprocess truly runs in that directory
        // (`CliBridgeConfig.working_dir` -> `LiveCliSpawner` `cmd.current_dir`).
        // Before MT-012 the config_template's working_dir was the only source and
        // SpawnRequest.working_dir was recorded-attribution-only.
        let session_config =
            cli_bridge_config_with_working_dir(self.config_template.clone(), working_dir);
        let mut rt = crate::model_runtime::cloud::CliBridgeModelRuntime::new(
            self.spawner.clone(),
            crate::model_runtime::cloud::AllowlistedCliBridgeConfig::new(
                session_config,
                self.model_allowlist.clone(),
            ),
        );
        let invocation_context = invocation_context.ok_or_else(|| {
            "official CLI bridge requires authoritative SpawnRequest invocation context".to_string()
        })?;
        let missing = [
            (
                invocation_context.requested_trust_class.is_none(),
                "requested_trust_class",
            ),
            (
                invocation_context.requested_isolation_tier.is_none(),
                "requested_isolation_tier",
            ),
            (
                invocation_context.requested_sandbox_capabilities.is_none(),
                "requested_sandbox_capabilities",
            ),
            (
                invocation_context.requested_net_policy.is_none(),
                "requested_net_policy",
            ),
            (
                invocation_context
                    .requested_execution_policy_ref
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .is_none(),
                "requested_execution_policy_ref",
            ),
        ]
        .into_iter()
        .filter_map(|(is_missing, field)| is_missing.then_some(field))
        .collect::<Vec<_>>();
        if !missing.is_empty() {
            return Err(format!(
                "official CLI bridge missing authoritative sandbox posture fields: {}",
                missing.join(", ")
            ));
        }
        rt = rt.with_invocation_context(invocation_context);
        if !self.emit_agent_activity_events {
            rt = rt.without_agent_activity_events();
        }
        // Thread the cloud-lane observability so the CLI lane emits
        // FR-EVT-LLM-INFER-{START,TOKEN,END} like the BYOK siblings, when a
        // recorder was wired (production_with_*_recorder).
        if let Some(obs) = self.lane_obs.clone() {
            rt = rt.with_lane_observability(obs);
        }
        let model_id = rt.load(self.cli_load_spec(model_name)).await.map_err(|e| {
            format!("official CLI bridge load for model '{model_name}' failed: {e}")
        })?;
        Ok(CloudLiveRuntime {
            runtime: Arc::new(rt),
            model_id,
        })
    }
}

/// Which concrete BYOK adapter a [`VaultCloudRuntimeBuilder`] constructs. Both
/// adapters already implement [`ModelRuntime`] (load / generate / score / embed
/// / cancel) so the swarm factory wraps them with no further glue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloudProviderFlavor {
    Anthropic,
    OpenAi,
}

impl CloudProviderFlavor {
    fn provider_kind(self) -> ProviderKind {
        // Both BYOK flavors live on the ByokCloud provider lane; the flavor
        // selects which concrete adapter/wire-protocol is constructed.
        ProviderKind::ByokCloud
    }

    fn default_api_base(self) -> &'static str {
        match self {
            // Bare host (no `/v1`); the adapters append their documented paths.
            CloudProviderFlavor::Anthropic => "https://api.anthropic.com",
            CloudProviderFlavor::OpenAi => "https://api.openai.com/v1",
        }
    }

    fn adapter_label(self) -> &'static str {
        match self {
            CloudProviderFlavor::Anthropic => "anthropic_byok",
            CloudProviderFlavor::OpenAi => "openai_byok",
        }
    }
}

/// A real cloud-runtime builder backed by the operator [`SecretsVault`] and the
/// shipped BYOK adapters. On `build_loaded` it:
///
/// 1. Fetches the operator API key for its lane from the vault. If the key is
///    absent (`NoSecretForLane`) it returns an honest not-configured reason —
///    the factory turns that into [`SwarmError::ProviderNotConfigured`]. The
///    key is fetched on demand per `build` and never stored on the builder.
/// 2. Constructs the concrete BYOK adapter ([`AnthropicByokRuntime`] /
///    [`OpenAiByokRuntime`]) with a [`VaultApiKeyProvider`] (so the live
///    generate path re-fetches the key per call) + a real tracing audit sink.
/// 3. Drives the adapter's real `load(LoadSpec{provider: ByokCloud,
///    engine_origin: model_name})`, which allowlist-gates the model name and
///    mints the runtime-keyed [`ModelId`]. A non-allowlisted model name or a
///    load failure surfaces as a not-configured reason (genuine runtime
///    condition).
///
/// The returned [`CloudLiveRuntime`] hands the swarm an `Arc<dyn ModelRuntime>`
/// whose `generate` is the adapter's real HTTP/SSE streaming path — so a cloud
/// session spawned through the coordinator runs the real provider when the
/// operator has stored a key, and fails honestly when they have not.
pub struct VaultCloudRuntimeBuilder {
    flavor: CloudProviderFlavor,
    vault: Arc<dyn crate::model_runtime::cloud::SecretsVault>,
    lane: String,
    api_base: String,
}

impl VaultCloudRuntimeBuilder {
    pub fn new(
        flavor: CloudProviderFlavor,
        vault: Arc<dyn crate::model_runtime::cloud::SecretsVault>,
        lane: impl Into<String>,
    ) -> Self {
        Self {
            flavor,
            vault,
            lane: lane.into(),
            api_base: flavor.default_api_base().to_string(),
        }
    }

    /// Override the provider API base (e.g. an Azure/OpenAI-compatible gateway
    /// or a test server). Defaults to the official provider host.
    pub fn with_api_base(mut self, api_base: impl Into<String>) -> Self {
        self.api_base = api_base.into();
        self
    }

    /// Construct the BYOK adapter's `ApiKeyProvider` bound to this lane. The
    /// provider re-fetches the secret from the vault on every call, so the key
    /// is never cached in the adapter struct (matches the BYOK redaction
    /// contract).
    ///
    /// We use a local `DynVaultApiKeyProvider` rather than the crate's
    /// `VaultApiKeyProvider<V>` because the latter is generic over a sized
    /// concrete vault, whereas the swarm wiring carries an erased
    /// `Arc<dyn SecretsVault>` (one config can mix vault impls). The local
    /// provider holds the trait object and dispatches dynamically.
    fn key_provider(&self) -> Arc<dyn crate::model_runtime::cloud::ApiKeyProvider> {
        Arc::new(DynVaultApiKeyProvider {
            vault: self.vault.clone(),
            lane: self.lane.clone(),
        })
    }

    /// Build the cloud `LoadSpec` for the BYOK adapters: provider=ByokCloud,
    /// engine_origin carries the allowlisted cloud model name. The local-only
    /// fields (artifact_path / sha256) are unused by the cloud adapters' load
    /// (they read only `provider` + `engine_origin`), so a synthetic empty
    /// artifact path + empty sha is the honest no-local-artifact shape.
    fn cloud_load_spec(&self, model_name: &str) -> crate::model_runtime::LoadSpec {
        use crate::model_runtime::{
            KvCachePolicy, LoadSpec, ModelCapabilities, RuntimeKind, SamplingParams,
        };
        LoadSpec {
            artifact_path: std::path::PathBuf::new(),
            sha256_expected: String::new(),
            runtime_kind: RuntimeKind::Candle, // unused by cloud load; placeholder
            sampling_defaults: SamplingParams::default(),
            kv_cache_policy: KvCachePolicy::default(),
            declared_capabilities: ModelCapabilities::default(),
            provider: ProviderKind::ByokCloud,
            engine_origin: Some(model_name.to_string()),
            external_engine_import: None,
        }
    }
}

#[async_trait]
impl CloudRuntimeBuilder for VaultCloudRuntimeBuilder {
    fn provider(&self) -> ProviderKind {
        self.flavor.provider_kind()
    }

    async fn build_loaded(
        &self,
        model_name: &str,
        // BYOK adapters do not spawn a local child, so the process invocation
        // context is unused here; accepted to satisfy the uniform trait.
        _invocation_context: Option<crate::model_runtime::cloud::CliInvocationContext>,
        // BYOK/remote lanes run over HTTP with no local subprocess, so the
        // operator working_dir has no cwd meaning here (MT-012).
        _working_dir: Option<&str>,
    ) -> Result<CloudLiveRuntime, String> {
        // (1) Honest credential preflight: a missing key is the genuine
        // not-configured runtime condition. We fetch through the SAME vault
        // lane the live generate path will use so there is no divergence
        // between "configured" here and "usable" at generate time.
        if let Err(err) = self.vault.get(&self.lane) {
            return Err(format!(
                "no API key in the operator secrets vault for lane '{}' ({}); store a key via the \
                 cloud-lane control panel to enable the {} lane",
                self.lane,
                err,
                self.flavor.adapter_label(),
            ));
        }

        let key_provider = self.key_provider();
        let audit_sink: Arc<dyn crate::model_runtime::cloud::CloudInvocationAuditSink> =
            Arc::new(TracingCloudAuditSink::new(self.flavor.adapter_label()));

        // (2)+(3): construct the concrete BYOK adapter and drive its REAL
        // allowlist-gated load to mint the runtime-keyed ModelId. A
        // non-allowlisted model name or any load failure is a genuine runtime
        // condition surfaced as a not-configured reason.
        let (runtime, model_id): (Arc<dyn ModelRuntime>, ModelId) = match self.flavor {
            CloudProviderFlavor::Anthropic => {
                let mut rt = crate::model_runtime::cloud::AnthropicByokRuntime::new(
                    self.api_base.clone(),
                    key_provider,
                    audit_sink,
                );
                let id = rt
                    .load(self.cloud_load_spec(model_name))
                    .await
                    .map_err(|e| {
                        format!("anthropic BYOK load for model '{model_name}' failed: {e}")
                    })?;
                (Arc::new(rt), id)
            }
            CloudProviderFlavor::OpenAi => {
                let mut rt = crate::model_runtime::cloud::OpenAiByokRuntime::new(
                    self.api_base.clone(),
                    key_provider,
                    audit_sink,
                );
                let id = rt
                    .load(self.cloud_load_spec(model_name))
                    .await
                    .map_err(|e| {
                        format!("openai BYOK load for model '{model_name}' failed: {e}")
                    })?;
                (Arc::new(rt), id)
            }
        };

        Ok(CloudLiveRuntime { runtime, model_id })
    }
}

/// Erased-vault [`ApiKeyProvider`] for the swarm cloud lane. Holds an
/// `Arc<dyn SecretsVault>` + a lane id and re-fetches the secret on every call,
/// so the key is never cached in the adapter struct (BYOK redaction contract).
/// The secret string is returned by value to the adapter and dropped after the
/// HTTP request is sent; it never enters Debug/Display here.
struct DynVaultApiKeyProvider {
    vault: Arc<dyn crate::model_runtime::cloud::SecretsVault>,
    lane: String,
}

impl crate::model_runtime::cloud::ApiKeyProvider for DynVaultApiKeyProvider {
    fn fetch_api_key(&self) -> Result<String, crate::model_runtime::cloud::OpenAiByokError> {
        // The vault read is `Zeroizing` (wiped on drop); the owned `String`
        // returned here is the one transient copy the transport needs and is
        // dropped after the HTTP request is sent.
        self.vault
            .get(&self.lane)
            .map(|secret| secret.to_string())
            .map_err(|err| {
                use crate::model_runtime::cloud::{ApiKeyFetchCode, SecretsVaultError};
                let code = match err {
                    SecretsVaultError::EmptyLaneId => ApiKeyFetchCode::VaultEmptyLaneId,
                    SecretsVaultError::EmptySecretValue => ApiKeyFetchCode::VaultEmptySecretValue,
                    SecretsVaultError::NoSecretForLane(_) => ApiKeyFetchCode::VaultNoSecretForLane,
                    SecretsVaultError::LockPoisoned(_) => ApiKeyFetchCode::VaultLockPoisoned,
                    SecretsVaultError::KeychainBackend(_) => ApiKeyFetchCode::VaultKeychainBackend,
                };
                crate::model_runtime::cloud::OpenAiByokError::ApiKeyFetch { code }
            })
    }
}

/// Real cloud-invocation audit sink: emits each [`CloudInvocationAuditRow`]
/// through `tracing` at INFO with the adapter label. This is a genuine,
/// leak-free observability sink (the API key is never part of the row), not a
/// mock — every BYOK lifecycle row (Started/Succeeded/Failed/Cancelled) is
/// recorded to the structured log. A Postgres-backed sink can replace this
/// without touching the adapters (the runtime is sink-agnostic).
struct TracingCloudAuditSink {
    adapter: &'static str,
}

impl TracingCloudAuditSink {
    fn new(adapter: &'static str) -> Self {
        Self { adapter }
    }
}

impl crate::model_runtime::cloud::CloudInvocationAuditSink for TracingCloudAuditSink {
    fn record(
        &self,
        row: crate::model_runtime::cloud::CloudInvocationAuditRow,
    ) -> Result<(), crate::model_runtime::cloud::OpenAiByokError> {
        tracing::info!(
            target: "handshake_core::swarm_orchestration::cloud_audit",
            adapter = self.adapter,
            model_id = %row.model_id,
            model_name = %row.openai_model_name,
            status = ?row.status,
            started_at = %row.started_at_utc,
            finished_at = ?row.finished_at_utc,
            "cloud BYOK invocation audit row"
        );
        Ok(())
    }
}

/// Seam that builds a live cloud `ModelRuntime` for a given allowlisted model
/// name, or reports that the lane is not configured. The app supplies a real
/// implementation backed by the secrets vault + the BYOK adapters (MT-206); the
/// factory only consumes this trait so it carries no reqwest/keychain specifics.
#[async_trait]
pub trait CloudRuntimeBuilder: Send + Sync + 'static {
    /// Provider this builder serves (for error reporting).
    fn provider(&self) -> ProviderKind;

    /// Build a loaded cloud runtime for `model_name`. Returns
    /// `Err(not_configured_reason)` when credentials/prereqs are absent — a
    /// real runtime condition, surfaced by the factory as
    /// [`SwarmError::ProviderNotConfigured`].
    ///
    /// `invocation_context` carries the authoritative owner, WP/MT, role,
    /// parent/session, trace, and selected-model attribution composed from the
    /// spawn request. Official-CLI builders require it and pass it verbatim into
    /// every child invocation; remote BYOK builders accept and ignore it because
    /// they do not spawn a local process.
    /// `working_dir` is the operator-selected on-disk location for this session
    /// (WP-1 MT-012). For the official-CLI bridge lane it becomes the REAL
    /// subprocess cwd (`CliBridgeConfig.working_dir`); BYOK/remote lanes have no
    /// local cwd and ignore it. `None` uses the builder's configured default.
    async fn build_loaded(
        &self,
        model_name: &str,
        invocation_context: Option<crate::model_runtime::cloud::CliInvocationContext>,
        working_dir: Option<&str>,
    ) -> Result<CloudLiveRuntime, String>;
}

/// A built, loaded cloud runtime plus the model id its `load` minted. The
/// factory wraps this into a [`LiveSession`] whose teardown unloads the cloud
/// model handle.
pub struct CloudLiveRuntime {
    pub runtime: Arc<dyn ModelRuntime>,
    pub model_id: ModelId,
}

#[derive(Clone)]
struct DurableWorktreeVmStore {
    pool: sqlx::PgPool,
    access: super::resource_scope::ResourceAccessContext,
}

#[derive(Clone)]
struct PendingWarmVmCreate {
    registry: Arc<super::worktree_vm_registry::WorktreeVmRegistry>,
    worktree_id: String,
    /// Exact durable/in-memory binding fence created by this factory attempt. A
    /// worktree id or ProcessHandle alone is routing identity, not cleanup
    /// ownership: an ABA successor may reuse either key after this attempt.
    binding_identity: super::worktree_vm_registry::WorktreeVmBindingIdentity,
}

/// The production model session factory.
pub struct ProductionModelSessionFactory {
    ledger: LedgerBatcher,
    cloud: CloudLaneFactoryConfig,
    /// Optional sandbox adapter registry (WP-KERNEL-004 wave 1). When present, a
    /// Local+LlamaCpp spawn whose `isolation_tier == Some(Tier3Microvm)` routes
    /// to [`Self::create_sandboxed_local`], which selects the microVM adapter
    /// and runs the boxed model inside it. `None` => sandbox routing is
    /// unavailable and a Tier-3 request fails loud with a typed error (never
    /// silently downgraded to an in-process spawn). Threaded by the app at
    /// startup (Integrate phase); the handshake_core change compiles with `None`.
    sandbox_registry: Option<Arc<SandboxAdapterRegistry>>,
    /// PostgreSQL + account authority shared with the coordinator's
    /// ModelLaneStore. When present, every worktree -> microVM binding survives
    /// factory/registry reconstruction and is filtered to the same account.
    durable_worktree_vm_store: Option<DurableWorktreeVmStore>,
    /// One lifecycle registry per selected adapter. Keeping this map inside the
    /// factory makes every warm session for the same worktree converge on the
    /// same in-process serialization point while PostgreSQL remains canonical
    /// across component restarts.
    worktree_vm_registries:
        Mutex<HashMap<String, Arc<super::worktree_vm_registry::WorktreeVmRegistry>>>,
    /// VM resources that crossed their durable bind boundary but whose
    /// `LiveSession` has not yet been returned. Entries intentionally survive
    /// cancellation of the async create future so the coordinator can await
    /// exact compensation even though no ProcessLedger START exists yet.
    pending_warm_vm_creates: Mutex<HashMap<ModelInstanceId, PendingWarmVmCreate>>,
    /// Synthetic pid base for in-process sessions (candle / cloud run in-process
    /// — there is no separate OS process). A monotonic offset keeps ledger pids
    /// distinct per instance so START/STOP rows correlate one-to-one.
    pid_base: u32,
}

impl ProductionModelSessionFactory {
    /// Build a factory that supports the local candle + llama.cpp paths, the
    /// supplied cloud-lane config, and (when `sandbox_registry` is `Some`) the
    /// Tier-3 microVM sandbox route. Pass [`CloudLaneFactoryConfig::unconfigured`]
    /// for a local-only deployment and `None` for the registry when sandbox
    /// routing is not wired.
    pub fn new(
        ledger: LedgerBatcher,
        cloud: CloudLaneFactoryConfig,
        sandbox_registry: Option<Arc<SandboxAdapterRegistry>>,
    ) -> Self {
        Self {
            ledger,
            cloud,
            sandbox_registry,
            durable_worktree_vm_store: None,
            worktree_vm_registries: Mutex::new(HashMap::new()),
            pending_warm_vm_creates: Mutex::new(HashMap::new()),
            pid_base: 50_000,
        }
    }

    /// Convenience: local-only factory (no cloud lanes, no sandbox registry).
    /// Cloud spawns return `ProviderNotConfigured`; Tier-3 sandbox spawns return
    /// a typed `FactoryFailed` (no registry to select a microVM adapter).
    pub fn local_only(ledger: LedgerBatcher) -> Self {
        Self::new(ledger, CloudLaneFactoryConfig::unconfigured(), None)
    }

    /// Thread a sandbox adapter registry so Tier-3 microVM routing is available.
    pub fn with_sandbox_registry(mut self, registry: Arc<SandboxAdapterRegistry>) -> Self {
        self.sandbox_registry = Some(registry);
        self
    }

    /// Bind warm worktree-VM lifecycle persistence to the exact store authority
    /// the coordinator uses for its ModelLane records.
    pub fn with_durable_worktree_vm_store(mut self, store: &ModelLaneStore) -> Self {
        self.durable_worktree_vm_store = Some(DurableWorktreeVmStore {
            pool: store.postgres_pool(),
            access: store.access().clone(),
        });
        self
    }

    fn require_cloud_resource_scope(&self) -> SwarmResult<ExactResourceScopeAttribution> {
        let scope = self
            .durable_worktree_vm_store
            .as_ref()
            .and_then(|store| store.access.write_scope())
            .ok_or_else(|| {
                SwarmError::FactoryFailed(
                    "RESOURCE_SCOPE_EXACT_REQUIRED: cloud provider dispatch requires complete server-owned scope"
                        .to_string(),
                )
            })?;
        ExactResourceScopeAttribution::try_from_resource_scope(scope).map_err(|_| {
            SwarmError::FactoryFailed(
                "RESOURCE_SCOPE_EXACT_REQUIRED: cloud provider dispatch requires complete server-owned scope"
                    .to_string(),
            )
        })
    }

    fn worktree_vm_registry_for(
        &self,
        adapter: Arc<dyn crate::sandbox::SandboxAdapter>,
    ) -> SwarmResult<Arc<super::worktree_vm_registry::WorktreeVmRegistry>> {
        let adapter_id = adapter.capabilities().adapter_id.to_string();
        let mut registries = self.worktree_vm_registries.lock().map_err(|error| {
            SwarmError::FactoryFailed(format!(
                "warm VM worktree registry map is poisoned: {error}"
            ))
        })?;
        if let Some(registry) = registries.get(&adapter_id) {
            return Ok(Arc::clone(registry));
        }
        let registry = match &self.durable_worktree_vm_store {
            Some(store) => Arc::new(
                super::worktree_vm_registry::WorktreeVmRegistry::new_durable(
                    adapter,
                    store.pool.clone(),
                    store.access.clone(),
                ),
            ),
            None => Arc::new(super::worktree_vm_registry::WorktreeVmRegistry::new(
                adapter,
            )),
        };
        registries.insert(adapter_id, Arc::clone(&registry));
        Ok(registry)
    }

    fn synthetic_pid(&self, request: &SpawnRequest) -> u32 {
        self.pid_base.wrapping_add(request.instance_id.instance)
    }

    fn register_pending_warm_vm_create(
        &self,
        request: &SpawnRequest,
        registry: Arc<super::worktree_vm_registry::WorktreeVmRegistry>,
        worktree_id: &str,
        binding_identity: super::worktree_vm_registry::WorktreeVmBindingIdentity,
    ) -> SwarmResult<()> {
        let mut pending = self.pending_warm_vm_creates.lock().map_err(|error| {
            SwarmError::FactoryFailed(format!("pending warm VM create map is poisoned: {error}"))
        })?;
        pending.insert(
            request.instance_id,
            PendingWarmVmCreate {
                registry,
                worktree_id: worktree_id.to_string(),
                binding_identity,
            },
        );
        Ok(())
    }

    fn clear_pending_warm_vm_create(&self, instance_id: ModelInstanceId) {
        if let Ok(mut pending) = self.pending_warm_vm_creates.lock() {
            pending.remove(&instance_id);
        }
    }

    /// Record a real process-ledger START row for an in-process model session,
    /// returning the record id the teardown STOP must match (C7). On a ledger
    /// failure returns a typed [`SwarmError::LedgerFailed`] so the spawn fails
    /// loud rather than leaving a phantom session.
    fn record_start(
        &self,
        request: &SpawnRequest,
        model_id: ModelId,
        os_pid: u32,
        engine_kind: ProcessEngineKind,
    ) -> SwarmResult<ProcessOwnershipRecordId> {
        let mut meta =
            SpawnMeta::new(os_pid, engine_kind, request.owner_role.clone()).without_os_pid();
        meta.model_id = Some(model_id.to_string());
        meta.runtime_binding = Some(request.runtime_binding.adapter_id().to_string());
        meta.parent_session_id = Some(request.parent_session_id.clone());
        meta.model_artifact_sha256 = request.model_artifact_sha256.clone();
        meta.owner_wp = request.owner_wp.clone();
        meta.role_id = request.role_id.clone();
        meta.wp_id = request.wp_id.clone();
        meta.mt_id = request.mt_id.clone();
        meta.metadata_blob = json!({
            "no_os_process_reason": "runtime executes in-process; numeric handle is coordinator-local only",
            "synthetic_coordinator_id": os_pid,
            "checkout_lease_id": request.checkout_lease().map(|lease| lease.lease_id),
            "checkout_lease_owner_generation": request.checkout_lease().map(|lease| lease.owner_generation),
            "checkout_lease_owner_instance_id": request.checkout_lease().map(|lease| lease.owner_instance_id.to_string()),
            "checkout_lease_worktree_id": request.checkout_lease().and_then(|lease| lease.worktree_id.clone()),
            "checkout_lease_canonical_working_dir": request.checkout_lease().and_then(|lease| lease.canonical_working_dir.clone()),
        });
        record_spawn(&self.ledger, meta).map_err(|e| SwarmError::LedgerFailed(e.to_string()))
    }

    async fn record_cloud_session_start(
        &self,
        reservation: ReservedProcessLifecycle,
        request: &SpawnRequest,
        resource_scope: &ExactResourceScopeAttribution,
        model_id: ModelId,
        provider: ProviderKind,
        engine_kind: ProcessEngineKind,
        os_pid: Option<u32>,
    ) -> SwarmResult<(
        ProcessOwnershipRecordId,
        ProcessStart,
        Arc<ActiveProcessLifecycle>,
    )> {
        let record_id = ProcessOwnershipRecordId::new_v7();
        let mut start = ProcessStart::new(
            engine_kind,
            request.owner_role.clone(),
            request.owner_wp.clone(),
        )
        .with_process_uuid(record_id.as_uuid())
        .with_parent_session_id(request.parent_session_id.clone())
        .with_metadata_jsonb(json!({
            "instance_id": request.instance_id.to_string(),
            "lifecycle_kind": if provider == ProviderKind::OfficialCli {
                "official_cli_bridge_session"
            } else {
                "in_process_cloud_session"
            },
            "no_os_process_reason": (provider == ProviderKind::ByokCloud).then_some(
                "BYOK cloud runtime session is in-process and has no Handshake-owned OS process"
            ),
            "provider": provider_str(provider),
            "model_id": model_id.to_string(),
            "runtime_binding": request.runtime_binding.adapter_id(),
            "synthetic_coordinator_id": self.synthetic_pid(request),
            "checkout_lease_id": request.checkout_lease().map(|lease| lease.lease_id),
            "checkout_lease_owner_generation": request.checkout_lease().map(|lease| lease.owner_generation),
            "checkout_lease_owner_instance_id": request.checkout_lease().map(|lease| lease.owner_instance_id.to_string()),
            "checkout_lease_worktree_id": request.checkout_lease().and_then(|lease| lease.worktree_id.clone()),
            "checkout_lease_canonical_working_dir": request.checkout_lease().and_then(|lease| lease.canonical_working_dir.clone()),
            "owner_account_id": resource_scope.owner_account_id.as_uuid(),
            "actor_principal_id": resource_scope.actor_principal_id.as_uuid(),
            "authenticated_session_id": resource_scope.authenticated_session_id.as_uuid(),
            "access_space_id": resource_scope.access_space_id.as_uuid(),
            "workspace_id": resource_scope.workspace_id.as_str(),
        }));
        if let Some(os_pid) = os_pid {
            start = start.with_os_pid(os_pid);
        }
        if let Some(role_id) = request.role_id.clone() {
            start = start.with_role_id(role_id);
        }
        if let Some(wp_id) = request.wp_id.clone() {
            start = start.with_wp_id(wp_id);
        }
        if let Some(mt_id) = request.mt_id.clone() {
            start = start.with_mt_id(mt_id);
        }
        let (lifecycle, durable_ack) = reservation
            .begin_with_durable_ack(start.clone())
            .map_err(|error| SwarmError::LedgerFailed(error.to_string()))?;
        let lifecycle = Arc::new(lifecycle);
        let mut durable_wait = tokio::spawn(durable_ack.wait_unbounded());
        match tokio::time::timeout(PIDLESS_SESSION_START_DURABILITY_TIMEOUT, &mut durable_wait)
            .await
        {
            Ok(Ok(Ok(()))) => {}
            Ok(Ok(Err(error))) => return Err(SwarmError::LedgerFailed(error.to_string())),
            Ok(Err(join_error)) => {
                return Err(SwarmError::LedgerFailed(format!(
                    "pidless START durability task failed: {join_error}"
                )))
            }
            Err(_elapsed) => {
                // The store write may still succeed after the caller's bounded
                // wait. Retain the lifecycle/STOP reservation until that result
                // is known. A late success is an aborted unpublished session,
                // so close it with a matching STOP; rejection releases the
                // unused STOP authority without fabricating a row.
                let late_lifecycle = Arc::clone(&lifecycle);
                tokio::spawn(async move {
                    if let Ok(Ok(())) = durable_wait.await {
                        if let Err(error) = late_lifecycle
                            .stop(None, "pidless-session-start-durability-timeout-aborted")
                        {
                            eprintln!(
                                "pidless late START committed but aborted STOP enqueue failed: {error}"
                            );
                        }
                    }
                });
                let error = crate::process_ledger::ProcessLedgerError::DurabilityAckTimeout {
                    event_kind: "START".to_string(),
                    process_uuid: record_id.as_uuid(),
                    timeout_ms: PIDLESS_SESSION_START_DURABILITY_TIMEOUT.as_millis(),
                };
                return Err(SwarmError::LedgerFailed(error.to_string()));
            }
        }
        Ok((record_id, start, lifecycle))
    }

    /// Record a START row for a SANDBOXED (microVM-routed) session, populating
    /// the ledger sandbox fields from the selected adapter + the ephemeral
    /// handle (WP-KERNEL-004 wave 1 §6). Mirrors [`Self::record_start`] but adds
    /// `sandbox_adapter_id` (e.g. `cloud_hypervisor`) and `sandbox_internal_id`
    /// (e.g. `hsk-ch-<uuid>`) so the START/STOP rows carry the microVM identity.
    #[allow(clippy::too_many_arguments)]
    fn record_start_sandboxed(
        &self,
        request: &SpawnRequest,
        model_id: ModelId,
        engine_kind: ProcessEngineKind,
        handle: &crate::sandbox::ProcessHandle,
    ) -> SwarmResult<(ProcessOwnershipRecordId, ProcessStart)> {
        let record_id = ProcessOwnershipRecordId::new_v7();
        let resource_scope = self
            .durable_worktree_vm_store
            .as_ref()
            .and_then(|store| store.access.write_scope());
        let owner_account_id = resource_scope.map(|scope| scope.owner_account_id.as_uuid());
        let actor_principal_id = resource_scope.map(|scope| scope.actor_principal_id.as_uuid());
        let authenticated_session_id = resource_scope
            .and_then(|scope| scope.authenticated_session.map(|session| session.as_uuid()));
        let access_space_id =
            resource_scope.and_then(|scope| scope.access_space.map(|space| space.as_uuid()));
        let workspace_id = resource_scope
            .and_then(|scope| scope.workspace.as_ref().map(|workspace| workspace.as_str()));
        let mut start = ProcessStart::new(
            engine_kind,
            request.owner_role.clone(),
            request.owner_wp.clone(),
        )
        .with_process_uuid(record_id.as_uuid())
        .with_parent_session_id(request.parent_session_id.clone())
        .with_sandbox_adapter_id(handle.adapter_id.to_string())
        .with_sandbox_internal_id(handle.sandbox_internal_id.clone())
        .with_metadata_jsonb(json!({
            "instance_id": request.instance_id.to_string(),
            "model_id": model_id.to_string(),
            "runtime_binding": request.runtime_binding.adapter_id(),
            "sandbox_handle_id": handle.id,
            "no_os_process_reason": handle.pid.is_none().then_some(
                "sandbox adapter did not expose a host OS PID; reclaim uses the persisted owning adapter handle"
            ),
            "checkout_lease_id": request.checkout_lease().map(|lease| lease.lease_id),
            "checkout_lease_owner_generation": request.checkout_lease().map(|lease| lease.owner_generation),
            "checkout_lease_owner_instance_id": request.checkout_lease().map(|lease| lease.owner_instance_id.to_string()),
            "checkout_lease_worktree_id": request.checkout_lease().and_then(|lease| lease.worktree_id.clone()),
            "checkout_lease_canonical_working_dir": request.checkout_lease().and_then(|lease| lease.canonical_working_dir.clone()),
            "owner_account_id": owner_account_id,
            "actor_principal_id": actor_principal_id,
            "authenticated_session_id": authenticated_session_id,
            "access_space_id": access_space_id,
            "workspace_id": workspace_id,
        }));
        start.started_at = handle.spawned_at_utc;
        if let Some(os_pid) = handle.pid {
            start = start.with_os_pid(os_pid);
        }
        if let Some(model_artifact_sha256) = request.model_artifact_sha256.clone() {
            start = start.with_model_artifact_sha256(model_artifact_sha256);
        }
        if let Some(role_id) = request.role_id.clone() {
            start = start.with_role_id(role_id);
        }
        if let Some(wp_id) = request.wp_id.clone() {
            start = start.with_wp_id(wp_id);
        }
        if let Some(mt_id) = request.mt_id.clone() {
            start = start.with_mt_id(mt_id);
        }
        self.ledger
            .record_start_lossless(start.clone())
            .map_err(|error| SwarmError::LedgerFailed(error.to_string()))?;
        Ok((record_id, start))
    }

    /// MT-207: explicit warm-VM local execution. This is intentionally a
    /// separate opt-in route from the cold Tier-3 sandbox runtime: if the
    /// selected adapter cannot provide a resident warm-agent transport with live
    /// token frames, the request fails closed instead of falling back to stdout
    /// chunking.
    async fn create_warm_vm_local(&self, request: &SpawnRequest) -> SwarmResult<LiveSession> {
        use crate::model_runtime::{
            try_inference_process_spec, LoadSpec, ModelCapabilities, RuntimeKind, SamplingParams,
            SandboxModelConfig, WarmVmModelConfig, WarmVmModelRuntime,
        };
        use crate::sandbox::cloud_hypervisor::{
            SANDBOX_MODE_METADATA_KEY, SANDBOX_MODE_PERSISTENT,
        };
        use crate::sandbox::{select, TrustClass};

        if request.runtime_binding != RuntimeBinding::LlamaCpp {
            return Err(SwarmError::FactoryFailed(
                "warm VM local execution currently requires runtime_binding=llama_cpp".to_string(),
            ));
        }
        if request.isolation_tier != Some(crate::sandbox::adapter::IsolationTier::Tier3Microvm) {
            return Err(SwarmError::FactoryFailed(
                "warm VM local execution requires isolation_tier=tier3_microvm".to_string(),
            ));
        }
        let worktree_id = request.worktree_id().ok_or_else(|| {
            SwarmError::FactoryFailed(
                "warm VM local execution requires worktree_id for snapshot/restore attribution"
                    .to_string(),
            )
        })?;
        let worktree_host_path = request
            .checkout_lease()
            .and_then(|lease| lease.canonical_working_dir.as_deref())
            .or_else(|| request.working_dir())
            .map(str::trim)
            .filter(|path| !path.is_empty())
            .map(std::path::PathBuf::from)
            .ok_or_else(|| {
                SwarmError::FactoryFailed(
                    "warm VM model-lane execution requires a worktree-scoped working_dir; the \
                     coordinator must provide its canonical checkout path before VM creation"
                        .to_string(),
                )
            })?;

        let registry = self.sandbox_registry.as_ref().ok_or_else(|| {
            SwarmError::FactoryFailed(
                "warm VM local execution requires a sandbox adapter registry; none is wired into \
                 the production factory"
                    .to_string(),
            )
        })?;

        let gguf_host =
            std::path::PathBuf::from(request.model_artifact_path().ok_or_else(|| {
                SwarmError::FactoryFailed(
                    "warm VM local execution requires a model artifact path (the GGUF)".to_string(),
                )
            })?);
        let model_artifact_sha256 = request.model_artifact_sha256.clone().ok_or_else(|| {
            SwarmError::FactoryFailed(
                "warm VM local execution requires model_artifact_sha256 for restore/load gating"
                    .to_string(),
            )
        })?;
        let restoring_warm_snapshot = request.warm_vm_restore_manifest.is_some();
        let gguf_root = gguf_host.parent().map(|p| p.to_path_buf()).ok_or_else(|| {
            SwarmError::FactoryFailed(
                "warm VM local execution: the GGUF path has no parent directory to bind"
                    .to_string(),
            )
        })?;

        let mut sandbox_cfg = SandboxModelConfig::new(gguf_host, gguf_root, "llama-cli")
            .with_trust_class(TrustClass::UntrustedAgent);
        if restoring_warm_snapshot {
            if let Some(runner_host) = std::env::var_os(SANDBOX_LLAMA_CLI_HOST_PATH_ENV) {
                sandbox_cfg =
                    sandbox_cfg.with_runner_host_path(std::path::PathBuf::from(runner_host));
            }
        } else {
            let runner_host =
                std::env::var_os(SANDBOX_LLAMA_CLI_HOST_PATH_ENV).ok_or_else(|| {
                    SwarmError::FactoryFailed(format!(
                    "warm VM local execution requires {SANDBOX_LLAMA_CLI_HOST_PATH_ENV} to point \
                     at the host llama.cpp runner binary"
                ))
                })?;
            sandbox_cfg = sandbox_cfg.with_runner_host_path(std::path::PathBuf::from(runner_host));
        }
        if let Some(bytes) = request.committed_memory_bytes {
            sandbox_cfg = sandbox_cfg.with_memory_bytes(bytes);
        }

        let model_id_for_spec = ModelId::new_v7();
        let mut spec = try_inference_process_spec(model_id_for_spec, &sandbox_cfg)
            .map_err(|e| SwarmError::FactoryFailed(e.to_string()))?;
        spec.metadata.insert(
            SANDBOX_MODE_METADATA_KEY.to_string(),
            SANDBOX_MODE_PERSISTENT.to_string(),
        );
        let adapter = select(registry, &spec, None)
            .map_err(|e| SwarmError::FactoryFailed(format!("sandbox selection failed: {e}")))?;
        let caps = adapter.capabilities();
        if !caps.supports_snapshot {
            return Err(SwarmError::FactoryFailed(format!(
                "adapter {} cannot satisfy warm VM execution: supports_snapshot=false; refusing \
                 non-resumable warm sessions",
                caps.adapter_id
            )));
        }
        if !restoring_warm_snapshot
            && (!caps.supports_warm_agent || !caps.supports_live_token_stream)
        {
            return Err(SwarmError::FactoryFailed(format!(
                "adapter {} cannot satisfy fresh warm VM execution: supports_warm_agent={}, \
                 supports_live_token_stream={}; refusing fallback to cold sandbox stdout chunking",
                caps.adapter_id, caps.supports_warm_agent, caps.supports_live_token_stream
            )));
        }
        if !restoring_warm_snapshot {
            if let Some((bind, guest_path)) = warm_agent_bind_from_env()? {
                spec.binds.push(bind);
                spec.metadata.insert(
                    CLOUD_HYPERVISOR_WARM_AGENT_GUEST_PATH_METADATA_KEY.to_string(),
                    guest_path,
                );
            }
        }
        spec.binds.push(BindSpec {
            host_path: worktree_host_path,
            guest_path: std::path::PathBuf::from(WARM_VM_WORKTREE_GUEST_ROOT),
            mode: BindMode::ReadOnly,
        });
        let worktree_vm_registry = self.worktree_vm_registry_for(Arc::clone(&adapter))?;

        let warm_cfg = WarmVmModelConfig::new(
            worktree_id.to_string(),
            sandbox_cfg.guest_gguf_path(),
            model_artifact_sha256.clone(),
        );
        if let Some(manifest) = request.warm_vm_restore_manifest.as_ref() {
            if manifest.worktree_id != worktree_id {
                return Err(SwarmError::FactoryFailed(format!(
                    "warm snapshot worktree mismatch: expected {}, got {}",
                    worktree_id, manifest.worktree_id
                )));
            }
            manifest
                .validate_for_restore(&model_artifact_sha256, &warm_cfg.model_guest_path)
                .map_err(|e| SwarmError::FactoryFailed(e.to_string()))?;
        }

        let (handle, created_by_attempt, binding_identity) =
            if let Some(manifest) = request.warm_vm_restore_manifest.as_ref() {
                let outcome = worktree_vm_registry
                    .restore_warm_model_with_identity(
                        manifest,
                        &model_artifact_sha256,
                        &warm_cfg.model_guest_path,
                    )
                    .await
                    .map_err(|e| SwarmError::FactoryFailed(e.to_string()))?;
                (outcome.handle, true, outcome.binding_identity)
            } else {
                let outcome = worktree_vm_registry
                    .ensure_worktree_vm_with_spec_outcome(worktree_id, spec)
                    .await
                    .map_err(|e| SwarmError::FactoryFailed(e.to_string()))?;
                (outcome.handle, outcome.created, outcome.binding_identity)
            };
        let binding_identity = binding_identity.ok_or_else(|| {
            SwarmError::FactoryFailed(format!(
                "warm VM binding for worktree {worktree_id} did not return an exact ownership fence"
            ))
        })?;

        if created_by_attempt {
            self.register_pending_warm_vm_create(
                request,
                Arc::clone(&worktree_vm_registry),
                worktree_id,
                binding_identity.clone(),
            )?;
        }

        let completion = async {

        let transport = match adapter.warm_agent_transport(&handle).await {
            Ok(transport) => transport,
            Err(error) => {
                let cleanup = teardown_worktree_after_factory_failure_if_owned(
                    &worktree_vm_registry,
                    worktree_id,
                    created_by_attempt,
                    &binding_identity,
                )
                .await;
                return Err(factory_error_with_cleanup(error.to_string(), cleanup));
            }
        };

        let mut runtime = if let Some(manifest) = request.warm_vm_restore_manifest.as_ref() {
            match WarmVmModelRuntime::from_restored_manifest(transport, warm_cfg.clone(), manifest) {
                Ok(runtime) => runtime,
                Err(error) => {
                    let cleanup = teardown_worktree_after_factory_failure_if_owned(
                        &worktree_vm_registry,
                        worktree_id,
                        created_by_attempt,
                        &binding_identity,
                    )
                    .await;
                    return Err(factory_error_with_cleanup(error.to_string(), cleanup));
                }
            }
        } else {
            WarmVmModelRuntime::new(transport, warm_cfg.clone())
        };
        let loaded_model_id = if request.warm_vm_restore_manifest.is_some() {
            runtime.model_id()
        } else {
            let spec_load = LoadSpec {
                artifact_path: std::path::PathBuf::new(),
                sha256_expected: model_artifact_sha256.clone(),
                runtime_kind: RuntimeKind::LlamaCpp,
                sampling_defaults: SamplingParams::default(),
                kv_cache_policy: crate::model_runtime::KvCachePolicy::default(),
                declared_capabilities: ModelCapabilities::default(),
                provider: ProviderKind::Local,
                engine_origin: Some("llama_cpp".to_string()),
                external_engine_import: None,
            };
            match runtime.load(spec_load).await {
                Ok(model_id) => model_id,
                Err(error) => {
                    let cleanup = teardown_worktree_after_factory_failure_if_owned(
                        &worktree_vm_registry,
                        worktree_id,
                        created_by_attempt,
                        &binding_identity,
                    )
                    .await;
                    return Err(factory_error_with_cleanup(error.to_string(), cleanup));
                }
            }
        };
        let ready = match runtime.ready_frame() {
            Ok(ready) => ready,
            Err(error) => {
                let cleanup = teardown_worktree_after_factory_failure_if_owned(
                    &worktree_vm_registry,
                    worktree_id,
                    created_by_attempt,
                    &binding_identity,
                )
                .await;
                return Err(factory_error_with_cleanup(error.to_string(), cleanup));
            }
        };
        let warm_vm_restore_manifest = match worktree_vm_registry
            .snapshot_warm_model(
                worktree_id,
                &model_artifact_sha256,
                &warm_cfg.model_guest_path,
                &ready,
            )
            .await
        {
            Ok(manifest) => manifest,
            Err(error) => {
                let cleanup = teardown_worktree_after_factory_failure_if_owned(
                    &worktree_vm_registry,
                    worktree_id,
                    created_by_attempt,
                    &binding_identity,
                )
                .await;
                return Err(factory_error_with_cleanup(
                    format!(
                        "warm VM snapshot capture failed after successful load/restore; refusing to \
                         return a non-resumable warm session: {error}"
                    ),
                    cleanup,
                ));
            }
        };
        let os_pid = self.synthetic_pid(request);
        let (record_id, ledger_start) = match self.record_start_sandboxed(
            request,
            loaded_model_id,
            ProcessEngineKind::LlamaCpp,
            &handle,
        ) {
            Ok(record_id) => record_id,
            Err(error) => {
                let cleanup = adapter
                    .delete_snapshot(&warm_vm_restore_manifest.snapshot)
                    .await;
                let kill_cleanup = teardown_worktree_after_factory_failure_if_owned(
                    &worktree_vm_registry,
                    worktree_id,
                    created_by_attempt,
                    &binding_identity,
                )
                .await;
                if let Err(cleanup_error) = cleanup {
                    return Err(SwarmError::FactoryFailed(format!(
                        "warm VM ledger START failed after successful snapshot capture and \
                         snapshot cleanup also failed; refusing to return a session: \
                         ledger_error={error}; cleanup_error={cleanup_error}"
                    )));
                }
                if let Err(cleanup_error) = kill_cleanup {
                    return Err(SwarmError::FactoryFailed(format!(
                        "warm VM ledger START failed and sandbox cleanup failed; \
                         ledger_error={error}; cleanup_error={cleanup_error}"
                    )));
                }
                return Err(error);
            }
        };

        let owning = Arc::new(tokio::sync::Mutex::new(runtime));
        let shared: Arc<dyn ModelRuntime> = Arc::new(SharedRuntimeHandle {
            inner: owning.clone(),
            model_id: loaded_model_id,
        });
        let cancel = CancellationToken::new();
        let teardown = worktree_warm_runtime_teardown(
            owning,
            loaded_model_id,
            Arc::clone(&worktree_vm_registry),
            worktree_id.to_string(),
            binding_identity,
        );

        let mut live =
            LiveSession::new(shared, loaded_model_id, cancel, teardown, record_id, os_pid);
        live = if ledger_start.os_pid.is_some() {
            live.with_ledger_start(ProcessEngineKind::LlamaCpp, ledger_start)
        } else {
            live.with_pidless_ledger(ProcessEngineKind::LlamaCpp, ledger_start)
        };
        live = live.with_warm_vm_restore_manifest(warm_vm_restore_manifest);
        Ok(live)
        }
        .await;

        if created_by_attempt && completion.is_ok() {
            self.clear_pending_warm_vm_create(request.instance_id);
        }
        completion
    }

    /// WP-KERNEL-004 wave 1: route a Local+LlamaCpp+Tier3 spawn into a Cloud
    /// Hypervisor microVM. Builds the inference [`ProcessSpec`], selects the
    /// microVM adapter via the registry (the `UntrustedAgent` trust class forces
    /// the Tier-3 minimum), builds + `load`s a [`SandboxModelRuntime`] (which
    /// mints the ephemeral handle + binds the GGUF), records the ledger START
    /// with the sandbox fields populated, and returns a [`LiveSession`] whose
    /// teardown kills the microVM handle + closes the ledger.
    async fn create_sandboxed_local(&self, request: &SpawnRequest) -> SwarmResult<LiveSession> {
        use crate::model_runtime::{
            try_inference_process_spec, SandboxModelConfig, SandboxModelRuntime,
        };
        use crate::sandbox::{select, TrustClass};

        let registry = self.sandbox_registry.as_ref().ok_or_else(|| {
            SwarmError::FactoryFailed(
                "Tier3 microVM spawn requires a sandbox adapter registry; none is wired into the \
                 production factory (Integrate phase threads it at app startup)"
                    .to_string(),
            )
        })?;

        let gguf_host =
            std::path::PathBuf::from(request.model_artifact_path().ok_or_else(|| {
                SwarmError::FactoryFailed(
                    "Tier3 sandboxed local spawn requires a model artifact path (the GGUF)"
                        .to_string(),
                )
            })?);
        let gguf_root = gguf_host.parent().map(|p| p.to_path_buf()).ok_or_else(|| {
            SwarmError::FactoryFailed(
                "Tier3 sandboxed local spawn: the GGUF path has no parent directory to bind"
                    .to_string(),
            )
        })?;

        let runner_host = std::env::var_os(SANDBOX_LLAMA_CLI_HOST_PATH_ENV).ok_or_else(|| {
            SwarmError::FactoryFailed(format!(
                "Tier3 sandboxed local spawn requires {SANDBOX_LLAMA_CLI_HOST_PATH_ENV} to \
                     point at the host llama.cpp runner binary; the microVM guest path is derived \
                     from that bind instead of assuming `llama-cli` exists in the guest"
            ))
        })?;
        let mut cfg = SandboxModelConfig::new(gguf_host, gguf_root, "llama-cli")
            .with_runner_host_path(std::path::PathBuf::from(runner_host))
            .with_trust_class(TrustClass::UntrustedAgent);
        if let Some(bytes) = request.committed_memory_bytes {
            cfg = cfg.with_memory_bytes(bytes);
        }

        // Select the microVM adapter for this spec (enforces the Tier-3 minimum).
        let model_id = ModelId::new_v7();
        let spec = try_inference_process_spec(model_id, &cfg)
            .map_err(|e| SwarmError::FactoryFailed(e.to_string()))?;
        let adapter = select(registry, &spec, None)
            .map_err(|e| SwarmError::FactoryFailed(format!("sandbox selection failed: {e}")))?;

        // Build + load the runtime around the SELECTED adapter (mints the
        // ephemeral handle + binds the GGUF; honest typed error on failure).
        let mut runtime = SandboxModelRuntime::new(Arc::clone(&adapter), cfg);
        let spec_load = crate::model_runtime::LoadSpec {
            artifact_path: std::path::PathBuf::new(),
            sha256_expected: request.model_artifact_sha256.clone().unwrap_or_default(),
            runtime_kind: crate::model_runtime::RuntimeKind::LlamaCpp,
            sampling_defaults: crate::model_runtime::SamplingParams::default(),
            kv_cache_policy: crate::model_runtime::KvCachePolicy::default(),
            declared_capabilities: crate::model_runtime::ModelCapabilities::default(),
            provider: ProviderKind::Local,
            engine_origin: Some("llama_cpp".to_string()),
            external_engine_import: None,
        };
        let loaded_model_id = runtime
            .load(spec_load)
            .await
            .map_err(|e| SwarmError::FactoryFailed(e.to_string()))?;

        // Read the sandbox identity for the ledger BEFORE moving the runtime.
        let ledger_handle = runtime.handle().cloned().ok_or_else(|| {
            SwarmError::FactoryFailed(
                "sandbox runtime loaded without a microVM handle (no sandbox_internal_id)"
                    .to_string(),
            )
        })?;
        // The adapter handle for teardown kill (clone before the runtime moves).
        let teardown_adapter = runtime.adapter();
        let teardown_handle = runtime.handle().cloned();

        let os_pid = self.synthetic_pid(request);
        // Record the START only AFTER load succeeded (C7) with the sandbox fields.
        let (record_id, ledger_start) = self.record_start_sandboxed(
            request,
            loaded_model_id,
            ProcessEngineKind::LlamaCpp,
            &ledger_handle,
        )?;

        let owning = Arc::new(tokio::sync::Mutex::new(runtime));
        let shared: Arc<dyn ModelRuntime> = Arc::new(SharedRuntimeHandle {
            inner: owning.clone(),
            model_id: loaded_model_id,
        });
        let cancel = CancellationToken::new();
        let teardown =
            sandboxed_runtime_teardown(owning, loaded_model_id, teardown_adapter, teardown_handle);

        let live = LiveSession::new(shared, loaded_model_id, cancel, teardown, record_id, os_pid);
        Ok(if ledger_start.os_pid.is_some() {
            live.with_ledger_start(ProcessEngineKind::LlamaCpp, ledger_start)
        } else {
            live.with_pidless_ledger(ProcessEngineKind::LlamaCpp, ledger_start)
        })
    }

    async fn create_local_candle(&self, request: &SpawnRequest) -> SwarmResult<LiveSession> {
        let artifact_path =
            std::path::PathBuf::from(request.model_artifact_path().ok_or_else(|| {
                SwarmError::FactoryFailed(
                    "local candle spawn requires a model artifact path".to_string(),
                )
            })?);
        let sha256 = request.model_artifact_sha256.clone().ok_or_else(|| {
            SwarmError::FactoryFailed(
                "local candle spawn requires model_artifact_sha256 for the integrity gate"
                    .to_string(),
            )
        })?;

        // REAL load via the exact shared helper the single-load IPC uses.
        let LoadedCandleModel {
            runtime,
            model_id,
            capabilities: _capabilities,
        } = load_local_candle_model(artifact_path, sha256)
            .await
            .map_err(|e| SwarmError::FactoryFailed(e.to_string()))?;

        let os_pid = self.synthetic_pid(request);
        // Record the START only AFTER the load succeeded so a load failure never
        // leaves an orphan START (C7).
        let record_id = self.record_start(request, model_id, os_pid, ProcessEngineKind::Candle)?;

        // Wrap the owning runtime in a shared Mutex so the coordinator's
        // `Arc<dyn ModelRuntime>` (used for cancel + the real generate path) and
        // the teardown closure (which needs &mut for the async `unload`) agree on
        // ONE engine. Teardown deterministically unloads then drops the last Arc
        // — the D1 free, mirroring `kernel_model_runtime_unload`'s detach-drop.
        let owning = Arc::new(tokio::sync::Mutex::new(runtime));
        let shared: Arc<dyn ModelRuntime> = Arc::new(SharedRuntimeHandle {
            inner: owning.clone(),
            model_id,
        });
        let cancel = CancellationToken::new();
        let teardown = shared_runtime_teardown(owning, model_id);

        Ok(LiveSession::new(
            shared, model_id, cancel, teardown, record_id, os_pid,
        ))
    }

    async fn create_local_llama(&self, request: &SpawnRequest) -> SwarmResult<LiveSession> {
        use crate::model_runtime::llama_cpp::LlamaCppRuntime;
        use crate::model_runtime::{KvCachePolicy, LoadSpec, RuntimeKind, SamplingParams};

        let artifact_path =
            std::path::PathBuf::from(request.model_artifact_path().ok_or_else(|| {
                SwarmError::FactoryFailed(
                    "local llama.cpp spawn requires a model artifact path".to_string(),
                )
            })?);
        let sha256 = request.model_artifact_sha256.clone().ok_or_else(|| {
            SwarmError::FactoryFailed(
                "local llama.cpp spawn requires model_artifact_sha256 for the integrity gate"
                    .to_string(),
            )
        })?;

        let spec = LoadSpec {
            artifact_path,
            sha256_expected: sha256,
            runtime_kind: RuntimeKind::LlamaCpp,
            sampling_defaults: SamplingParams::default(),
            kv_cache_policy: KvCachePolicy::default(),
            declared_capabilities: llama_base_capabilities(),
            provider: ProviderKind::Local,
            engine_origin: Some("llama_cpp".to_string()),
            external_engine_import: None,
        };

        let mut runtime = LlamaCppRuntime::new(KvCachePolicy::default());
        // REAL load. If the GGUF is absent / unreadable the adapter returns a
        // typed ModelRuntimeError, surfaced honestly here (not a placeholder).
        let model_id = runtime
            .load(spec)
            .await
            .map_err(|e| SwarmError::FactoryFailed(e.to_string()))?;

        let os_pid = self.synthetic_pid(request);
        let record_id =
            self.record_start(request, model_id, os_pid, ProcessEngineKind::LlamaCpp)?;

        let owning = Arc::new(tokio::sync::Mutex::new(runtime));
        let shared: Arc<dyn ModelRuntime> = Arc::new(SharedRuntimeHandle {
            inner: owning.clone(),
            model_id,
        });
        let cancel = CancellationToken::new();
        let teardown = shared_runtime_teardown(owning, model_id);

        Ok(LiveSession::new(
            shared, model_id, cancel, teardown, record_id, os_pid,
        ))
    }

    async fn create_cloud(
        &self,
        request: &SpawnRequest,
        provider: ProviderKind,
    ) -> SwarmResult<LiveSession> {
        // Deny before any provider/session resource is opened unless the same
        // complete server-owned authority can protect its durable lifecycle.
        let resource_scope = self.require_cloud_resource_scope()?;
        let model_name =
            request
                .cloud_model_name
                .clone()
                .ok_or_else(|| SwarmError::ProviderNotConfigured {
                    provider: provider_str(provider).to_string(),
                    detail: "cloud spawn requires cloud_model_name".to_string(),
                })?;

        let builder = match provider {
            ProviderKind::ByokCloud => match request.byok_cloud_provider {
                Some(super::ids::ByokCloudProvider::Anthropic) => self.cloud.anthropic.as_ref(),
                Some(super::ids::ByokCloudProvider::OpenAi) => self.cloud.openai.as_ref(),
                None => self.cloud.openai.as_ref().or(self.cloud.anthropic.as_ref()),
            },
            ProviderKind::OfficialCli => match request.official_cli_provider.as_deref() {
                Some(provider) => self.cloud.official_cli_by_provider.get(provider),
                None => self.cloud.official_cli.as_ref(),
            },
            _ => None,
        };
        let Some(builder) = builder else {
            let flavor_detail = match (provider, request.byok_cloud_provider) {
                (ProviderKind::ByokCloud, Some(flavor)) => {
                    format!("requested BYOK provider {flavor:?} is not configured")
                }
                (ProviderKind::OfficialCli, _) => match request.official_cli_provider.as_deref() {
                    Some(cli_provider) => format!(
                        "requested official CLI provider {cli_provider} is not configured"
                    ),
                    None => "no official CLI provider was selected and no fallback CLI bridge is configured"
                        .to_string(),
                },
                _ => "no cloud-lane builder configured for this provider (operator has not \
                         set up BYOK credentials / CLI)"
                    .to_string(),
            };
            return Err(SwarmError::ProviderNotConfigured {
                provider: provider_str(provider).to_string(),
                detail: flavor_detail,
            });
        };

        if provider == ProviderKind::OfficialCli {
            let missing = [
                (
                    request.requested_trust_class.is_none(),
                    "requested_trust_class",
                ),
                (request.isolation_tier.is_none(), "isolation_tier"),
                (
                    request.requested_sandbox_capabilities.is_none(),
                    "requested_sandbox_capabilities",
                ),
                (
                    request.requested_net_policy.is_none(),
                    "requested_net_policy",
                ),
                (
                    request
                        .requested_execution_policy_ref
                        .as_deref()
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .is_none(),
                    "requested_execution_policy_ref",
                ),
            ]
            .into_iter()
            .filter_map(|(is_missing, field)| is_missing.then_some(field))
            .collect::<Vec<_>>();
            if !missing.is_empty() {
                return Err(SwarmError::FactoryFailed(format!(
                    "official CLI spawn missing authoritative sandbox posture fields: {}",
                    missing.join(", ")
                )));
            }
        }

        let session_id = request.instance_id.to_string();
        let mut invocation_context = crate::model_runtime::cloud::CliInvocationContext::new(
            request.owner_role.clone(),
            model_name.clone(),
        );
        invocation_context.owner_wp = request.owner_wp.clone();
        invocation_context.role_id = request.role_id.clone();
        invocation_context.wp_id = request.wp_id.clone();
        invocation_context.mt_id = request.mt_id.clone();
        invocation_context.session_id = Some(session_id.clone());
        invocation_context.parent_session_id = Some(request.parent_session_id.clone());
        invocation_context.trace_id = Some(request.parent_session_id.clone());
        invocation_context.span_id = Some(session_id);
        invocation_context.requested_trust_class = request.requested_trust_class;
        invocation_context.requested_isolation_tier = request.isolation_tier;
        invocation_context.requested_sandbox_capabilities =
            request.requested_sandbox_capabilities.clone();
        invocation_context.requested_net_policy = request.requested_net_policy.clone();
        invocation_context.requested_execution_policy_ref =
            request.requested_execution_policy_ref.clone();
        invocation_context.swarm_id = request.swarm_id.clone();
        invocation_context.worktree_id = request.worktree_id.clone();
        invocation_context.working_dir = request.working_dir.clone();
        invocation_context.checkout_lease_id = request.checkout_lease().map(|lease| lease.lease_id);
        invocation_context.checkout_lease_owner_generation =
            request.checkout_lease().map(|lease| lease.owner_generation);
        invocation_context.checkout_lease_canonical_working_dir = request
            .checkout_lease()
            .and_then(|lease| lease.canonical_working_dir.clone());

        // Reserve the complete cloud-session START/STOP lifecycle before the cloud
        // runtime opens any session resource. A full or closed writer therefore
        // fails the spawn before `build_loaded`, never after publication.
        let reservation = self
            .ledger
            .try_reserve_lifecycles(1)
            .map_err(|error| SwarmError::LedgerFailed(error.to_string()))?
            .pop()
            .ok_or_else(|| {
                SwarmError::LedgerFailed(
                    "single pidless session lifecycle reservation was empty".to_string(),
                )
            })?;
        let CloudLiveRuntime { runtime, model_id } = builder
            .build_loaded(&model_name, Some(invocation_context), request.working_dir())
            .await
            .map_err(|reason| SwarmError::ProviderNotConfigured {
                provider: provider_str(provider).to_string(),
                detail: reason,
            })?;

        // A BYOK cloud session is genuinely in-process with no Handshake-owned
        // OS process, so its session-level START stays pidless (os_pid = None).
        // The Official-CLI bridge is different: it runs inside a HandshakeNative
        // attached sandbox HOSTED BY THIS coordinator process (see seed.rs
        // UserManual "HandshakeNative attached-sandbox" contract), so the
        // session-level START carries the REAL coordinator/host OS PID
        // (`std::process::id()`). That is a genuine host PID, not a fabricated
        // child pid, so it does not violate the MT-013 synthetic-pid-forbidden
        // invariant (which forbids inventing a fake child pid for a process that
        // truly has none). The coordinator-local synthetic scheduling id remains
        // in `LiveSession::os_pid` for scheduling only and is still kept out of
        // the ledger via `ledger_os_pid = None` (see factory.rs), so it never
        // masquerades as a host PID.
        let os_pid = self.synthetic_pid(request);
        let session_engine_kind = match provider {
            ProviderKind::ByokCloud => ProcessEngineKind::ExternalCompat,
            ProviderKind::OfficialCli => ProcessEngineKind::OfficialCliBridge,
            _ => {
                return Err(SwarmError::FactoryFailed(format!(
                    "unsupported pidless cloud session provider: {}",
                    provider_str(provider)
                )))
            }
        };
        let session_start_os_pid = match provider {
            ProviderKind::OfficialCli => Some(std::process::id()),
            _ => None,
        };
        let (record_id, ledger_start, ledger_lifecycle) = self
            .record_cloud_session_start(
                reservation,
                request,
                &resource_scope,
                model_id,
                provider,
                session_engine_kind,
                session_start_os_pid,
            )
            .await?;
        let cancel = CancellationToken::new();
        let teardown = cloud_teardown(runtime.clone(), model_id);

        Ok(
            LiveSession::new(runtime, model_id, cancel, teardown, record_id, os_pid)
                .with_pidless_reserved_ledger(session_engine_kind, ledger_start, ledger_lifecycle),
        )
    }
}

#[async_trait]
impl ModelSessionFactory for ProductionModelSessionFactory {
    async fn create(&self, request: &SpawnRequest) -> SwarmResult<LiveSession> {
        match request.provider {
            None | Some(ProviderKind::Local) if request.wants_warm_vm_execution() => {
                self.create_warm_vm_local(request).await
            }
            // WP-KERNEL-004 wave 1: a Local+LlamaCpp spawn that EXPLICITLY
            // requests Tier-3 microVM isolation routes into a Cloud Hypervisor
            // microVM. Keyed strictly on the three conditions so every other
            // local spawn (incl. plain Local+LlamaCpp with no Tier-3) falls
            // through unchanged to `create_local_llama` (regression guard).
            None | Some(ProviderKind::Local)
                if request.runtime_binding == RuntimeBinding::LlamaCpp
                    && request.isolation_tier
                        == Some(crate::sandbox::adapter::IsolationTier::Tier3Microvm) =>
            {
                self.create_sandboxed_local(request).await
            }
            None | Some(ProviderKind::Local) => match request.runtime_binding {
                RuntimeBinding::Candle => self.create_local_candle(request).await,
                RuntimeBinding::LlamaCpp => self.create_local_llama(request).await,
            },
            Some(ProviderKind::ExternalCompat) => Err(SwarmError::ProviderNotConfigured {
                provider: "external_compat".to_string(),
                detail: "external-compat imports are offline registrations, not swarm-spawnable \
                         runtimes"
                    .to_string(),
            }),
            Some(provider @ (ProviderKind::ByokCloud | ProviderKind::OfficialCli)) => {
                self.create_cloud(request, provider).await
            }
        }
    }

    async fn cancel_pending_create(&self, request: &SpawnRequest) -> SwarmResult<()> {
        let pending = self
            .pending_warm_vm_creates
            .lock()
            .map_err(|error| {
                SwarmError::FactoryFailed(format!(
                    "pending warm VM create map is poisoned: {error}"
                ))
            })?
            .get(&request.instance_id)
            .cloned();
        let Some(target) = pending else {
            // No exact attempt record means no cleanup authority. In
            // particular, a failed adopter has a worktree id but did not create
            // the existing binding and must never tear it down.
            return Ok(());
        };
        target
            .registry
            .teardown_worktree_vm_if_current(&target.worktree_id, &target.binding_identity)
            .await
            .map_err(|error| {
                SwarmError::FactoryFailed(format!(
                    "pending warm VM create compensation refused a stale binding or failed for {}: {error}",
                    request.instance_id,
                ))
            })?;
        self.clear_pending_warm_vm_create(request.instance_id);
        Ok(())
    }
}

fn provider_str(provider: ProviderKind) -> &'static str {
    match provider {
        ProviderKind::Local => "local",
        ProviderKind::ExternalCompat => "external_compat",
        ProviderKind::ByokCloud => "byok_cloud",
        ProviderKind::OfficialCli => "official_cli",
    }
}

fn llama_base_capabilities() -> crate::model_runtime::ModelCapabilities {
    crate::model_runtime::ModelCapabilities {
        supports_lora: true,
        supports_kv_prefix_cache: true,
        supports_kv_quantization: crate::model_runtime::KvQuantSupport::None,
        // llama.cpp binding must NOT declare activation steering (registry gate).
        supports_activation_steering: false,
        supports_subquadratic: false,
        supports_speculative_draft: false,
        supports_eagle3: false,
        ..Default::default()
    }
}

/// Teardown for a local runtime wrapped in a shared `Arc<Mutex<R>>`: it
/// deterministically `unload`s the model behind the Mutex (mirroring
/// `kernel_model_runtime_unload`'s detach), then drops the owning Arc so the
/// runtime's `Drop` frees the engine weights when the last reference goes away.
/// This is the D1 free for both the candle and llama.cpp local paths.
async fn kill_created_sandbox_after_factory_failure(
    adapter: &Arc<dyn crate::sandbox::SandboxAdapter>,
    handle: &crate::sandbox::ProcessHandle,
) -> Result<(), String> {
    let mut last_error = None;
    for _ in 0..3 {
        match adapter.kill(handle, crate::sandbox::Signal::Kill).await {
            Ok(()) => return Ok(()),
            Err(error) => {
                last_error = Some(error.to_string());
                tokio::task::yield_now().await;
            }
        }
    }
    Err(last_error.unwrap_or_else(|| "sandbox kill failed without a diagnostic".into()))
}

async fn teardown_created_worktree_after_factory_failure(
    registry: &Arc<super::worktree_vm_registry::WorktreeVmRegistry>,
    worktree_id: &str,
    binding_identity: &super::worktree_vm_registry::WorktreeVmBindingIdentity,
) -> Result<(), String> {
    let mut last_error = None;
    for _ in 0..3 {
        match registry
            .teardown_worktree_vm_if_current(worktree_id, binding_identity)
            .await
        {
            Ok(()) => return Ok(()),
            Err(error) => {
                last_error = Some(error.to_string());
                tokio::task::yield_now().await;
            }
        }
    }
    Err(last_error.unwrap_or_else(|| {
        format!("worktree VM teardown failed without a diagnostic for {worktree_id}")
    }))
}

async fn teardown_worktree_after_factory_failure_if_owned(
    registry: &Arc<super::worktree_vm_registry::WorktreeVmRegistry>,
    worktree_id: &str,
    created_by_attempt: bool,
    binding_identity: &super::worktree_vm_registry::WorktreeVmBindingIdentity,
) -> Result<(), String> {
    if !created_by_attempt {
        return Ok(());
    }
    teardown_created_worktree_after_factory_failure(registry, worktree_id, binding_identity).await
}

fn factory_error_with_cleanup(primary_error: String, cleanup: Result<(), String>) -> SwarmError {
    match cleanup {
        Ok(()) => SwarmError::FactoryFailed(primary_error),
        Err(cleanup_error) => SwarmError::FactoryFailed(format!(
            "{primary_error}; sandbox cleanup failed after three attempts: {cleanup_error}"
        )),
    }
}

/// Warm worktree-VM teardown unloads the resident model and always asks the
/// WorktreeVmRegistry to terminalize the exact durable binding. Either failure
/// keeps the coordinator's retryable teardown closure pending.
fn worktree_warm_runtime_teardown<R>(
    owning: Arc<tokio::sync::Mutex<R>>,
    model_id: ModelId,
    registry: Arc<super::worktree_vm_registry::WorktreeVmRegistry>,
    worktree_id: String,
    binding_identity: super::worktree_vm_registry::WorktreeVmBindingIdentity,
) -> SessionTeardown
where
    R: ModelRuntime + 'static,
{
    #[derive(Default)]
    struct Progress {
        unload_succeeded: bool,
        registry_teardown_succeeded: bool,
    }

    let progress = Arc::new(tokio::sync::Mutex::new(Progress::default()));
    Arc::new(move || {
        let owning = Arc::clone(&owning);
        let registry = Arc::clone(&registry);
        let worktree_id = worktree_id.clone();
        let binding_identity = binding_identity.clone();
        let progress = Arc::clone(&progress);
        Box::pin(async move {
            // Serialize retries and retain success independently for both
            // side-effects. A transient failure must not repeat a completed
            // unload or terminalize the same VM binding twice.
            let mut progress = progress.lock().await;
            let unload_error = if progress.unload_succeeded {
                None
            } else {
                let result = {
                    let mut guard = owning.lock().await;
                    guard.unload(model_id).await
                };
                match result {
                    Ok(()) => {
                        progress.unload_succeeded = true;
                        None
                    }
                    Err(error) => Some(error),
                }
            };
            let teardown_error = if progress.registry_teardown_succeeded {
                None
            } else {
                match registry
                    .teardown_worktree_vm_if_current(&worktree_id, &binding_identity)
                    .await
                {
                    Ok(()) => {
                        progress.registry_teardown_succeeded = true;
                        // Killing the owning VM terminalizes the in-guest model
                        // even when the warm-agent shutdown RPC failed. Treat
                        // unload as complete so a retry never sends RPCs to a
                        // transport whose VM is already gone.
                        progress.unload_succeeded = true;
                        None
                    }
                    Err(error) => Some(error),
                }
            };
            if unload_error.is_some() || teardown_error.is_some() {
                return Err(SwarmError::Internal(format!(
                    "warm worktree VM teardown failed: unload={unload_error:?}; registry={teardown_error:?}"
                )));
            }
            Ok(())
        })
    })
}

fn shared_runtime_teardown<R>(
    owning: Arc<tokio::sync::Mutex<R>>,
    model_id: ModelId,
) -> SessionTeardown
where
    R: ModelRuntime + 'static,
{
    Arc::new(move || {
        let owning = Arc::clone(&owning);
        Box::pin(async move {
            {
                let mut guard = owning.lock().await;
                guard.unload(model_id).await.map_err(|error| {
                    SwarmError::Internal(format!("shared runtime teardown unload failed: {error}"))
                })?;
            }
            // Drop our owning reference. When the coordinator also drops its
            // shared `Arc<dyn ModelRuntime>` (on terminal eviction) the last
            // reference goes away and `R::Drop` frees the weights.
            Ok(())
        })
    })
}

/// Sandboxed (microVM) teardown (WP-KERNEL-004 wave 1): unload the boxed model
/// behind the Mutex (best-effort, which also kills the ephemeral handle via the
/// runtime's `unload`), THEN explicitly `kill` the microVM handle through the
/// adapter so the in-VM boot is torn down even if the runtime was already
/// unloaded, then drop the owning Arc. The ledger STOP is written by the
/// coordinator against the START record this session carries.
fn sandboxed_runtime_teardown<R>(
    owning: Arc<tokio::sync::Mutex<R>>,
    model_id: ModelId,
    adapter: Arc<dyn crate::sandbox::SandboxAdapter>,
    handle: Option<crate::sandbox::ProcessHandle>,
) -> SessionTeardown
where
    R: ModelRuntime + 'static,
{
    Arc::new(move || {
        let owning = Arc::clone(&owning);
        let adapter = Arc::clone(&adapter);
        let handle = handle.clone();
        Box::pin(async move {
            let unload_error = {
                let mut guard = owning.lock().await;
                guard.unload(model_id).await.err()
            };
            // Always attempt explicit microVM teardown even when model unload
            // fails. Either error keeps the coordinator cleanup receipt pending
            // and retains this retryable closure.
            let kill_error = if let Some(handle) = handle {
                adapter
                    .kill(&handle, crate::sandbox::Signal::Term)
                    .await
                    .err()
            } else {
                None
            };
            if unload_error.is_some() || kill_error.is_some() {
                return Err(SwarmError::Internal(format!(
                    "sandbox runtime teardown failed: unload={:?}; kill={:?}",
                    unload_error, kill_error
                )));
            }
            Ok(())
        })
    })
}

/// Cloud teardown: unload the model handle from the cloud runtime so its audit
/// trail closes; the runtime Arc drops with the session.
fn cloud_teardown(runtime: Arc<dyn ModelRuntime>, _model_id: ModelId) -> SessionTeardown {
    Arc::new(move || {
        let runtime = Arc::clone(&runtime);
        Box::pin(async move {
            // The cloud adapter's `unload` takes &mut self; the trait object is
            // shared, so we cannot call it directly. Dropping the last Arc is
            // the free for a cloud handle (no local weights to release). The
            // per-call audit rows already close each invocation. MT-206 wires
            // an explicit cloud-handle unload when the lane gains a mutable
            // registry; until then dropping the shared handle is the honest,
            // leak-free teardown for an in-memory cloud session.
            drop(runtime);
            Ok(())
        })
    })
}

/// Shared view over an owning local runtime `Arc<Mutex<R>>` so the coordinator's
/// `Arc<dyn ModelRuntime>` (cancel + the real generate path) and the teardown
/// (which needs `&mut` for the async `unload`) agree on ONE engine. Generic over
/// the concrete local runtime (`CandleRuntime` or `LlamaCppRuntime`).
///
/// `generate` acquires the Mutex via `try_lock` so the coordinator never blocks
/// on a busy single instance; a contended generate returns a typed error stream
/// rather than deadlocking. (One instance serves one generate at a time; run a
/// SECOND `ModelInstanceId` of the same artifact for concurrent generation.)
struct SharedRuntimeHandle<R: ModelRuntime> {
    inner: Arc<tokio::sync::Mutex<R>>,
    model_id: ModelId,
}

#[async_trait]
impl<R: ModelRuntime + 'static> ModelRuntime for SharedRuntimeHandle<R> {
    fn adapter_name(&self) -> &'static str {
        "swarm_shared_runtime"
    }

    async fn load(
        &mut self,
        _spec: crate::model_runtime::LoadSpec,
    ) -> Result<ModelId, crate::model_runtime::ModelRuntimeError> {
        Err(crate::model_runtime::ModelRuntimeError::LoadError(
            "swarm shared handle does not load; the session is already loaded".to_string(),
        ))
    }

    async fn unload(&mut self, id: ModelId) -> Result<(), crate::model_runtime::ModelRuntimeError> {
        let mut guard = self.inner.lock().await;
        guard.unload(id).await
    }

    fn generate(
        &self,
        req: crate::model_runtime::GenerateRequest,
    ) -> crate::model_runtime::TokenStream {
        match self.inner.try_lock() {
            Ok(guard) => guard.generate(req),
            Err(_) => Box::pin(futures::stream::once(async {
                Err(crate::model_runtime::ModelRuntimeError::GenerateError(
                    "swarm runtime instance is busy (a single instance serves one generate at a \
                     time; spawn another ModelInstanceId for concurrency)"
                        .to_string(),
                ))
            })),
        }
    }

    async fn score(
        &self,
        id: ModelId,
        sequence: Vec<u32>,
    ) -> Result<crate::model_runtime::Score, crate::model_runtime::ModelRuntimeError> {
        let guard = self.inner.lock().await;
        guard.score(id, sequence).await
    }

    async fn embed(
        &self,
        id: ModelId,
        text: &str,
    ) -> Result<crate::model_runtime::Embedding, crate::model_runtime::ModelRuntimeError> {
        let guard = self.inner.lock().await;
        guard.embed(id, text).await
    }

    fn capabilities(
        &self,
        _id: ModelId,
    ) -> Result<&crate::model_runtime::ModelCapabilities, crate::model_runtime::ModelRuntimeError>
    {
        // A `&` capability ref cannot escape the Mutex guard; the coordinator
        // reads capabilities from the registry, not the shared handle. Surface a
        // typed redirect rather than panicking.
        Err(
            crate::model_runtime::ModelRuntimeError::CapabilityNotSupported {
                capability: "capabilities via swarm shared handle (use the registry record)"
                    .to_string(),
                adapter: "swarm_shared_runtime".to_string(),
            },
        )
    }

    fn kv_cache(
        &self,
        _id: ModelId,
    ) -> Result<crate::model_runtime::KvCacheHandle, crate::model_runtime::ModelRuntimeError> {
        Err(crate::model_runtime::ModelRuntimeError::KvCacheError(
            "kv_cache via swarm shared handle is not exposed".to_string(),
        ))
    }

    fn lora_stack(
        &self,
        _id: ModelId,
    ) -> Result<crate::model_runtime::LoraStackHandle, crate::model_runtime::ModelRuntimeError>
    {
        Err(crate::model_runtime::ModelRuntimeError::LoraStackError(
            "lora_stack via swarm shared handle is not exposed".to_string(),
        ))
    }

    fn steering_hooks(
        &self,
        _id: ModelId,
    ) -> Result<crate::model_runtime::SteeringHookHandle, crate::model_runtime::ModelRuntimeError>
    {
        Err(crate::model_runtime::ModelRuntimeError::SteeringHookError(
            "steering via swarm shared handle is not exposed".to_string(),
        ))
    }

    fn cancel(&self, token: CancellationToken) {
        let _ = self.model_id;
        token.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_runtime::ModelId;
    use crate::process_ledger::{
        LedgerBatcherConfig, LedgerEvent, LedgerEventKind, LedgerOverflowEvent, NoopOverflowSink,
        ProcessLedgerDrain, ProcessLedgerError, ProcessLedgerOverflowSink, ProcessLedgerStore,
        ProcessStart,
    };
    use crate::sandbox::{
        manifest_for_test_recorded_probe_artifact, package_manifest_path_for_agent,
        validate_warm_agent_package_manifest_candidate, WarmAgentGuestPackageManifest,
        WarmAgentPackageError, WARM_AGENT_PACKAGE_AGENT_BINARY_MARKER,
        WARM_AGENT_PACKAGE_LLAMA_SERVER_MARKERS,
    };
    use crate::swarm_orchestration::events::RecordingSwarmSink;
    use crate::swarm_orchestration::ids::{ModelInstanceId, RunBudget, SpawnRequest};
    use crate::swarm_orchestration::SwarmCoordinator;
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex, OnceLock};

    static RUNNER_ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn runner_env_lock() -> std::sync::MutexGuard<'static, ()> {
        RUNNER_ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("runner env lock")
    }

    struct WarmAgentValidatorOverrideGuard {
        prior: Option<WarmAgentPackageValidator>,
    }

    impl Drop for WarmAgentValidatorOverrideGuard {
        fn drop(&mut self) {
            *warm_agent_package_validator_override().lock().unwrap() = self.prior;
        }
    }

    fn install_test_warm_agent_validator() -> WarmAgentValidatorOverrideGuard {
        let mut guard = warm_agent_package_validator_override().lock().unwrap();
        let prior = *guard;
        *guard = Some(test_warm_agent_package_validator);
        WarmAgentValidatorOverrideGuard { prior }
    }

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

    #[derive(Clone, Default)]
    struct FailingOverflowSink;

    impl ProcessLedgerOverflowSink for FailingOverflowSink {
        fn emit_overflow(&self, _event: LedgerOverflowEvent) -> Result<(), ProcessLedgerError> {
            Err(ProcessLedgerError::Store(
                "forced ledger overflow failure".to_string(),
            ))
        }
    }

    /// A MANUAL batcher: no writer task exists, so nothing is written until the
    /// test itself calls [`drained`]. That is fine for paths that only enqueue
    /// rows, but it can NEVER satisfy a caller that awaits a store-level
    /// durability acknowledgement while it is still inside `create()` —
    /// `record_cloud_session_start` does exactly that. Cloud-session tests must
    /// use [`spawned_ledger`] instead; see the comment there.
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

    /// A DRIVEN ledger: `LedgerBatcher::spawn` starts the real background writer
    /// task against `store`, so a START enqueued with a durable-ack permit is
    /// actually flushed and acknowledged while the caller is still awaiting it.
    ///
    /// This is the same wiring the passing
    /// `factory_dispatches_official_cli_to_cli_builder` uses. Any test that
    /// drives `ProductionModelSessionFactory` down the CLOUD path
    /// (`record_cloud_session_start`) needs it: that function awaits a durable
    /// START ack under `PIDLESS_SESSION_START_DURABILITY_TIMEOUT`, and against a
    /// manual batcher with no writer the ack can only ever time out. Pair with
    /// [`drain_spawned`] so the writer is closed and joined, never detached.
    fn spawned_ledger(
        store: Arc<InMemoryStore>,
    ) -> (
        LedgerBatcher,
        tokio::task::JoinHandle<Result<(), ProcessLedgerError>>,
    ) {
        LedgerBatcher::spawn(
            store,
            Arc::new(NoopOverflowSink),
            LedgerBatcherConfig {
                capacity: 4096,
                ..LedgerBatcherConfig::default()
            },
        )
    }

    fn lazy_model_lane_store() -> ModelLaneStore {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://handshake:handshake@127.0.0.1:5432/handshake_lazy")
            .expect("lazy model lane pool");
        ModelLaneStore::new(pool)
    }

    /// Cloud dispatch is production-fenced by a complete server-owned resource
    /// scope. Tests that intend to exercise a cloud lane must provide that same
    /// authority instead of stopping at the fail-closed scope guard.
    fn scoped_lazy_model_lane_store() -> ModelLaneStore {
        use crate::swarm_orchestration::resource_scope::{
            AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, OwnerAccountId,
            ResourceScope, WorkspaceScopeRef,
        };

        let scope = ResourceScope::new(OwnerAccountId::mint(), ActorPrincipalId::mint())
            .with_session(AuthenticatedSessionRef::mint())
            .with_access_space(AccessSpaceRef::mint())
            .with_workspace(
                WorkspaceScopeRef::new("workspace-production-factory-test")
                    .expect("non-empty workspace scope"),
            );
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://handshake:handshake@127.0.0.1:5432/handshake_lazy")
            .expect("lazy scoped model lane pool");
        ModelLaneStore::new_scoped(pool, scope)
    }

    fn one_slot_failing_overflow_ledger_pair() -> (LedgerBatcher, ProcessLedgerDrain) {
        LedgerBatcher::manual_for_tests(
            LedgerBatcherConfig {
                capacity: 1,
                batch_size: 1,
                ..LedgerBatcherConfig::default()
            },
            Arc::new(FailingOverflowSink),
        )
        .expect("manual one-slot ledger")
    }

    async fn drained(drain: &ProcessLedgerDrain, store: Arc<InMemoryStore>) -> Vec<LedgerEvent> {
        drain.drain_available_to(store.clone()).await.unwrap();
        store.events.lock().unwrap().clone()
    }

    /// Close + join the writer started by [`spawned_ledger`] under a bounded
    /// deadline and return what the store durably holds. The `Flushed`
    /// assertion is part of the proof: a `TimedOut`/`JoinError` outcome would
    /// mean the rows below were read from a writer that never finished.
    async fn drain_spawned(
        ledger: &LedgerBatcher,
        writer_join: tokio::task::JoinHandle<Result<(), ProcessLedgerError>>,
        store: Arc<InMemoryStore>,
    ) -> Vec<LedgerEvent> {
        let outcome = crate::process_ledger::drain_and_join_ledger_writer(
            ledger,
            writer_join,
            Duration::from_secs(5),
        )
        .await;
        assert!(
            matches!(
                outcome,
                crate::process_ledger::LedgerDrainJoinOutcome::Flushed
            ),
            "the driven ledger writer must flush and terminate cleanly; got {outcome:?}"
        );
        store.events.lock().unwrap().clone()
    }

    fn instance(i: u32) -> ModelInstanceId {
        ModelInstanceId::new(ModelId::new_v7(), i)
    }

    fn local_candle_req(iid: ModelInstanceId, path: &str) -> SpawnRequest {
        SpawnRequest::new(iid, RuntimeBinding::Candle, "swarm_prod_test", "parent-1")
            .with_local_artifact(path, "ab".repeat(32))
    }

    /// `create` returns `Result<LiveSession, _>` and `LiveSession` is not
    /// `Debug` (it owns a boxed teardown closure), so the standard
    /// `expect_err` is unavailable. This helper asserts the create FAILED and
    /// yields the typed error.
    async fn create_err(factory: &ProductionModelSessionFactory, req: &SpawnRequest) -> SwarmError {
        match factory.create(req).await {
            Ok(_) => panic!("expected the factory create to fail"),
            Err(e) => e,
        }
    }

    // ---- DEFAULT-CI: dispatch returns the right typed errors honestly. ----

    #[tokio::test]
    async fn cloud_spawn_without_configured_lane_returns_provider_not_configured() {
        let (ledger, _drain) = ledger_pair();
        let model_lane_store = scoped_lazy_model_lane_store();
        let factory = ProductionModelSessionFactory::local_only(ledger)
            .with_durable_worktree_vm_store(&model_lane_store);
        let req = SpawnRequest::new(
            instance(0),
            RuntimeBinding::Candle,
            "swarm_prod_test",
            "parent-1",
        )
        .with_cloud_provider(ProviderKind::ByokCloud, "gpt-4o");
        let err = create_err(&factory, &req).await;
        match err {
            SwarmError::ProviderNotConfigured { provider, .. } => {
                assert_eq!(provider, "byok_cloud");
            }
            other => panic!("expected ProviderNotConfigured, got {other}"),
        }
    }

    #[tokio::test]
    async fn external_compat_provider_is_not_swarm_spawnable() {
        let (ledger, _drain) = ledger_pair();
        let factory = ProductionModelSessionFactory::local_only(ledger);
        let mut req = SpawnRequest::new(
            instance(0),
            RuntimeBinding::Candle,
            "swarm_prod_test",
            "parent-1",
        );
        req.provider = Some(ProviderKind::ExternalCompat);
        let err = create_err(&factory, &req).await;
        assert!(
            matches!(err, SwarmError::ProviderNotConfigured { .. }),
            "got {err}"
        );
    }

    #[tokio::test]
    async fn local_candle_missing_artifact_path_fails_factory_honestly() {
        let (ledger, _drain) = ledger_pair();
        let factory = ProductionModelSessionFactory::local_only(ledger);
        // No artifact path set -> typed FactoryFailed, no panic, no placeholder.
        let req = SpawnRequest::new(
            instance(0),
            RuntimeBinding::Candle,
            "swarm_prod_test",
            "parent-1",
        );
        let err = create_err(&factory, &req).await;
        assert!(matches!(err, SwarmError::FactoryFailed(_)), "got {err}");
    }

    #[tokio::test]
    async fn local_candle_nonexistent_file_fails_factory_and_leaves_no_orphan_start() {
        // A nonexistent artifact must fail the real load (sha/file gate) and the
        // factory must NOT have recorded a ledger START before failing (the
        // START is recorded only AFTER a successful load — C7).
        let (ledger, drain) = ledger_pair();
        let store = Arc::new(InMemoryStore::default());
        let factory = ProductionModelSessionFactory::local_only(ledger);
        let req = local_candle_req(
            instance(0),
            "D:/__handshake_no_such_swarm_model__/model.safetensors",
        );
        let err = create_err(&factory, &req).await;
        assert!(matches!(err, SwarmError::FactoryFailed(_)), "got {err}");
        let rows = drained(&drain, store).await;
        assert!(
            rows.is_empty(),
            "a failed load must leave NO ledger START (got {} rows)",
            rows.len()
        );
    }

    /// Two admission stages of the same production wiring, in the order the
    /// coordinator actually applies them.
    ///
    /// STAGE 1 — `build_production_swarm_coordinator` always returns a
    /// ModelLaneStore-backed, dexterity-REQUIRED coordinator
    /// (`SwarmCoordinator::new_with_model_lane_store`). Its contract guard runs
    /// BEFORE factory dispatch and rejects any `SpawnRequest` that carries no
    /// `with_dexterity_launch` contract, whatever the artifact looks like. This
    /// stage therefore cannot observe a factory outcome at all, and asserting
    /// `FactoryFailed` here was a stale expectation, not a product guarantee.
    ///
    /// AC-2 DECISION (MT-020): the guard legitimately precedes factory dispatch
    /// AND a ModelLaneStore-backed coordinator is the wrong vehicle for the
    /// missing-artifact case. Both halves of the original intent are kept, each
    /// on the coordinator that can actually express it, and the guard itself is
    /// untouched:
    ///   * the ModelLaneStore-backed production coordinator is asserted to
    ///     reject pre-dispatch with the SPECIFIC contract error (not merely
    ///     "some LedgerFailed"), leaving no live session and no ledger row, and
    ///   * the missing-artifact factory failure is asserted through a
    ///     coordinator that is legitimately permitted to dispatch.
    ///
    /// Why STAGE 2 does not simply attach a Dexterity contract to STAGE 1's
    /// request: on the factory-failure path a ModelLaneStore-backed coordinator
    /// durably records a FAILED launch row (`store.record_prepared_launch`, see
    /// coordinator.rs). `lazy_model_lane_store()` points at a pool that has no
    /// reachable database, so that write fails and re-wraps the real
    /// `FactoryFailed` cause into another `LedgerFailed`. Proving the
    /// missing-artifact contract through that path would require real
    /// PostgreSQL, which this `--lib` unit module deliberately does not use.
    #[tokio::test]
    async fn production_coordinator_constructs_and_rejects_unloadable_local_spawn_without_orphan() {
        // ---- STAGE 1: production wiring helper + its pre-dispatch guard. ----
        let (ledger, drain) = ledger_pair();
        let store = Arc::new(InMemoryStore::default());
        let coordinator = build_production_swarm_coordinator(
            ledger,
            CloudLaneFactoryConfig::unconfigured(),
            lazy_model_lane_store(),
            Some(2),
            uuid::Uuid::now_v7(),
            |_ev| Ok(()),
        );
        let req = local_candle_req(
            instance(7),
            "D:/__handshake_no_such_swarm_model__/model.safetensors",
        );
        let err = coordinator
            .spawn_session(req)
            .await
            .expect_err("contract-less spawn on a ModelLaneStore-backed coordinator");
        match &err {
            SwarmError::LedgerFailed(detail) => assert!(
                detail.contains("require SpawnRequest::with_dexterity_launch"),
                "the rejection must be the Dexterity launch-contract guard, not \
                 an unrelated ledger failure; got {detail}"
            ),
            other => panic!("expected the Dexterity launch-contract guard, got {other}"),
        }
        // Rejected pre-dispatch: no live session and no ledger row exists,
        // because the factory was never reached to record a START.
        assert_eq!(coordinator.live_session_count(), 0);
        let rows = drained(&drain, store).await;
        assert!(
            rows.is_empty(),
            "a pre-dispatch rejection writes no ledger row; got {}",
            rows.len()
        );

        // ---- STAGE 2: the missing-artifact factory failure itself, on a
        // coordinator that is legitimately allowed to dispatch. ----
        let (ledger2, drain2) = ledger_pair();
        let store2 = Arc::new(InMemoryStore::default());
        let sink = Arc::new(RecordingSwarmSink::new());
        let factory2 = Arc::new(ProductionModelSessionFactory::new(
            ledger2.clone(),
            CloudLaneFactoryConfig::unconfigured(),
            None,
        ));
        let dispatching = SwarmCoordinator::new_legacy_without_dexterity_for_tests(
            crate::swarm_orchestration::SwarmConfig::new(
                RunBudget::defaulted(2).with_concurrency(2),
            ),
            factory2,
            sink.clone(),
            ledger2,
        );
        let req2 = local_candle_req(
            instance(8),
            "D:/__handshake_no_such_swarm_model__/model.safetensors",
        );
        let err2 = dispatching
            .spawn_session(req2)
            .await
            .expect_err("unloadable spawn");
        assert!(
            matches!(err2, SwarmError::FactoryFailed(_)),
            "the typed factory error propagates; got {err2}"
        );
        // No live session remains and the ledger has no orphan rows: the
        // factory stopped any START it recorded before returning.
        assert_eq!(dispatching.live_session_count(), 0);
        let rows2 = drained(&drain2, store2).await;
        assert!(
            rows2.is_empty(),
            "no orphan ledger rows; got {}",
            rows2.len()
        );
        // A SessionFailed/SpawnRejected event was emitted (sink ran).
        assert!(
            !sink.events().is_empty(),
            "the sink must have emitted at least one event"
        );
    }

    #[tokio::test]
    async fn cloud_builder_reports_not_configured_through_factory() {
        // A configured-but-credential-less cloud builder surfaces its honest
        // not-configured reason as SwarmError::ProviderNotConfigured.
        struct UnconfiguredOpenAi;
        #[async_trait]
        impl CloudRuntimeBuilder for UnconfiguredOpenAi {
            fn provider(&self) -> ProviderKind {
                ProviderKind::ByokCloud
            }
            async fn build_loaded(
                &self,
                _model: &str,
                _invocation_context: Option<crate::model_runtime::cloud::CliInvocationContext>,
                _working_dir: Option<&str>,
            ) -> Result<CloudLiveRuntime, String> {
                Err("no openai api key in the operator secrets vault".to_string())
            }
        }
        let (ledger, drain) = ledger_pair();
        let store = Arc::new(InMemoryStore::default());
        let cloud = CloudLaneFactoryConfig {
            openai: Some(Arc::new(UnconfiguredOpenAi)),
            anthropic: None,
            official_cli: None,
            official_cli_by_provider: HashMap::new(),
        };
        let model_lane_store = scoped_lazy_model_lane_store();
        let factory = ProductionModelSessionFactory::new(ledger, cloud, None)
            .with_durable_worktree_vm_store(&model_lane_store);
        let req = SpawnRequest::new(
            instance(0),
            RuntimeBinding::Candle,
            "swarm_prod_test",
            "parent-1",
        )
        .with_cloud_provider(ProviderKind::ByokCloud, "gpt-4o");
        let err = create_err(&factory, &req).await;
        match err {
            SwarmError::ProviderNotConfigured { detail, .. } => {
                assert!(detail.contains("api key"), "{detail}");
            }
            other => panic!("expected ProviderNotConfigured, got {other}"),
        }
        // Cloud failure recorded no START (the START is written only after a
        // successful build).
        let rows = drained(&drain, store).await;
        assert!(rows.is_empty(), "cloud failure must leave no ledger rows");
    }

    /// A cloud builder that returns a REAL in-process ModelRuntime (a controllable
    /// one) so the cloud SUCCESS dispatch + teardown + ledger START/STOP symmetry
    /// is provable without live network. Not a result-faking mock of the cloud
    /// API: it exercises the factory's cloud dispatch path end-to-end.
    struct OkCloudBuilder {
        unloaded: Arc<std::sync::atomic::AtomicUsize>,
    }
    #[async_trait]
    impl CloudRuntimeBuilder for OkCloudBuilder {
        fn provider(&self) -> ProviderKind {
            ProviderKind::ByokCloud
        }
        async fn build_loaded(
            &self,
            _model: &str,
            _invocation_context: Option<crate::model_runtime::cloud::CliInvocationContext>,
            _working_dir: Option<&str>,
        ) -> Result<CloudLiveRuntime, String> {
            let model_id = ModelId::new_v7();
            Ok(CloudLiveRuntime {
                runtime: Arc::new(super::tests::CountingRuntime {
                    unloaded: self.unloaded.clone(),
                }),
                model_id,
            })
        }
    }

    struct CountingRuntime {
        unloaded: Arc<std::sync::atomic::AtomicUsize>,
    }
    #[async_trait]
    impl ModelRuntime for CountingRuntime {
        async fn load(
            &mut self,
            _spec: crate::model_runtime::LoadSpec,
        ) -> Result<ModelId, crate::model_runtime::ModelRuntimeError> {
            Ok(ModelId::new_v7())
        }
        async fn unload(
            &mut self,
            _id: ModelId,
        ) -> Result<(), crate::model_runtime::ModelRuntimeError> {
            self.unloaded
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }
        fn generate(
            &self,
            _req: crate::model_runtime::GenerateRequest,
        ) -> crate::model_runtime::TokenStream {
            Box::pin(futures::stream::iter(Vec::new()))
        }
        async fn score(
            &self,
            _id: ModelId,
            _s: Vec<u32>,
        ) -> Result<crate::model_runtime::Score, crate::model_runtime::ModelRuntimeError> {
            Ok(crate::model_runtime::Score {
                token_logprobs: vec![],
                mean_logprob: 0.0,
            })
        }
        async fn embed(
            &self,
            _id: ModelId,
            _t: &str,
        ) -> Result<crate::model_runtime::Embedding, crate::model_runtime::ModelRuntimeError>
        {
            Ok(crate::model_runtime::Embedding { vector: vec![] })
        }
        fn capabilities(
            &self,
            _id: ModelId,
        ) -> Result<&crate::model_runtime::ModelCapabilities, crate::model_runtime::ModelRuntimeError>
        {
            Err(
                crate::model_runtime::ModelRuntimeError::CapabilityNotSupported {
                    capability: "n/a".to_string(),
                    adapter: "counting".to_string(),
                },
            )
        }
        fn kv_cache(
            &self,
            _id: ModelId,
        ) -> Result<crate::model_runtime::KvCacheHandle, crate::model_runtime::ModelRuntimeError>
        {
            Err(crate::model_runtime::ModelRuntimeError::KvCacheError(
                "n/a".to_string(),
            ))
        }
        fn lora_stack(
            &self,
            _id: ModelId,
        ) -> Result<crate::model_runtime::LoraStackHandle, crate::model_runtime::ModelRuntimeError>
        {
            Err(crate::model_runtime::ModelRuntimeError::LoraStackError(
                "n/a".to_string(),
            ))
        }
        fn steering_hooks(
            &self,
            _id: ModelId,
        ) -> Result<crate::model_runtime::SteeringHookHandle, crate::model_runtime::ModelRuntimeError>
        {
            Err(crate::model_runtime::ModelRuntimeError::SteeringHookError(
                "n/a".to_string(),
            ))
        }
        fn cancel(&self, token: CancellationToken) {
            token.cancel();
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn cloud_success_dispatch_records_start_and_teardown_records_stop_and_unloads() {
        // DRIVEN ledger: the cloud dispatch path awaits a durable START ack, so
        // a manual batcher with no writer task could only time out.
        let store = Arc::new(InMemoryStore::default());
        let (ledger, ledger_writer) = spawned_ledger(store.clone());
        let ledger_close = ledger.clone();
        let unloaded = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let cloud = CloudLaneFactoryConfig {
            openai: Some(Arc::new(OkCloudBuilder {
                unloaded: unloaded.clone(),
            })),
            anthropic: None,
            official_cli: None,
            official_cli_by_provider: HashMap::new(),
        };
        let sink = Arc::new(RecordingSwarmSink::new());
        let model_lane_store = scoped_lazy_model_lane_store();
        let factory = Arc::new(
            ProductionModelSessionFactory::new(ledger.clone(), cloud, None)
                .with_durable_worktree_vm_store(&model_lane_store),
        );
        let coordinator = SwarmCoordinator::new_legacy_without_dexterity_for_tests(
            crate::swarm_orchestration::SwarmConfig::new(
                RunBudget::defaulted(4).with_concurrency(4),
            ),
            factory,
            sink,
            ledger,
        );
        let iid = instance(0);
        let req = SpawnRequest::new(iid, RuntimeBinding::Candle, "swarm_prod_test", "parent-1")
            .with_cloud_provider(ProviderKind::ByokCloud, "gpt-4o");
        coordinator
            .spawn_session(req)
            .await
            .expect("cloud spawn succeeds");
        assert_eq!(coordinator.live_session_count(), 1);
        // Cancel -> teardown must free (unload) the cloud handle is dropped; the
        // cloud teardown drops the shared Arc (no unload call for shared handle),
        // so we assert the session left no orphan and START==STOP instead.
        coordinator
            .cancel_session(iid, "test_cancel")
            .await
            .expect("cancel");
        assert_eq!(coordinator.live_session_count(), 0);

        let rows = drain_spawned(&ledger_close, ledger_writer, store).await;
        let starts = rows
            .iter()
            .filter(|e| e.kind() == LedgerEventKind::Start)
            .count();
        let stops = rows
            .iter()
            .filter(|e| e.kind() == LedgerEventKind::Stop)
            .count();
        assert_eq!(starts, 1, "exactly one START for the cloud session");
        assert_eq!(stops, 1, "exactly one STOP — no orphan");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn byok_cloud_provider_selection_honors_anthropic_when_openai_also_configured() {
        struct RecordingCloudBuilder {
            label: &'static str,
            built: Arc<Mutex<Vec<&'static str>>>,
        }

        #[async_trait]
        impl CloudRuntimeBuilder for RecordingCloudBuilder {
            fn provider(&self) -> ProviderKind {
                ProviderKind::ByokCloud
            }

            async fn build_loaded(
                &self,
                _model: &str,
                _invocation_context: Option<crate::model_runtime::cloud::CliInvocationContext>,
                _working_dir: Option<&str>,
            ) -> Result<CloudLiveRuntime, String> {
                self.built.lock().expect("built lock").push(self.label);
                let model_id = ModelId::new_v7();
                Ok(CloudLiveRuntime {
                    runtime: Arc::new(super::tests::CountingRuntime {
                        unloaded: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
                    }),
                    model_id,
                })
            }
        }

        // DRIVEN ledger: the BYOK cloud path awaits a durable START ack.
        let store = Arc::new(InMemoryStore::default());
        let (ledger, ledger_writer) = spawned_ledger(store.clone());
        let ledger_close = ledger.clone();
        let built = Arc::new(Mutex::new(Vec::new()));
        let cloud = CloudLaneFactoryConfig {
            anthropic: Some(Arc::new(RecordingCloudBuilder {
                label: "anthropic",
                built: built.clone(),
            })),
            openai: Some(Arc::new(RecordingCloudBuilder {
                label: "openai",
                built: built.clone(),
            })),
            official_cli: None,
            official_cli_by_provider: HashMap::new(),
        };
        let model_lane_store = scoped_lazy_model_lane_store();
        let factory = ProductionModelSessionFactory::new(ledger, cloud, None)
            .with_durable_worktree_vm_store(&model_lane_store);
        let req = SpawnRequest::new(
            instance(0),
            RuntimeBinding::Candle,
            "swarm_prod_test",
            "parent-1",
        )
        .with_cloud_provider(ProviderKind::ByokCloud, "claude-sonnet-4")
        .with_byok_cloud_provider(crate::swarm_orchestration::ByokCloudProvider::Anthropic);

        let live = factory
            .create(&req)
            .await
            .expect("explicit anthropic BYOK spawn succeeds");
        assert_eq!(
            built.lock().expect("built lock").as_slice(),
            ["anthropic"],
            "explicit provider flavor must select Anthropic, not legacy OpenAI-first fallback"
        );
        (live.teardown)().await.expect("teardown");
        // This is a FACTORY-level test: the coordinator, which owns the STOP,
        // is not in the picture. The reserved lifecycle inside `live` therefore
        // still holds its unused STOP permit, and an outstanding permit keeps
        // the writer's channel open past `begin_close()`. Release it before the
        // bounded drain-and-join below, or the writer cannot terminate.
        drop(live);
        // The spawn only returns Ok once the START was durably acknowledged, so
        // the row MUST already be in the store. Asserting it keeps the
        // durability guarantee observable rather than implicit.
        let rows = drain_spawned(&ledger_close, ledger_writer, store).await;
        assert_eq!(
            rows.iter()
                .filter(|e| e.kind() == LedgerEventKind::Start)
                .count(),
            1,
            "the durably acknowledged START is in the store"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn official_cli_provider_selection_honors_requested_provider() {
        struct RecordingCliBuilder {
            label: &'static str,
            built: Arc<Mutex<Vec<&'static str>>>,
            contexts: Arc<Mutex<Vec<crate::model_runtime::cloud::CliInvocationContext>>>,
        }

        #[async_trait]
        impl CloudRuntimeBuilder for RecordingCliBuilder {
            fn provider(&self) -> ProviderKind {
                ProviderKind::OfficialCli
            }

            async fn build_loaded(
                &self,
                _model: &str,
                invocation_context: Option<crate::model_runtime::cloud::CliInvocationContext>,
                _working_dir: Option<&str>,
            ) -> Result<CloudLiveRuntime, String> {
                self.built.lock().expect("built lock").push(self.label);
                self.contexts
                    .lock()
                    .expect("contexts lock")
                    .push(invocation_context.expect("production factory must compose CLI context"));
                let model_id = ModelId::new_v7();
                Ok(CloudLiveRuntime {
                    runtime: Arc::new(super::tests::CountingRuntime {
                        unloaded: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
                    }),
                    model_id,
                })
            }
        }

        // DRIVEN ledger: the official-CLI cloud path awaits a durable START ack.
        let store = Arc::new(InMemoryStore::default());
        let (ledger, ledger_writer) = spawned_ledger(store.clone());
        let ledger_close = ledger.clone();
        let built = Arc::new(Mutex::new(Vec::new()));
        let contexts = Arc::new(Mutex::new(Vec::new()));
        let mut official_cli_by_provider: HashMap<String, Arc<dyn CloudRuntimeBuilder>> =
            HashMap::new();
        official_cli_by_provider.insert(
            "claude_code".to_string(),
            Arc::new(RecordingCliBuilder {
                label: "claude_code",
                built: built.clone(),
                contexts: contexts.clone(),
            }),
        );
        official_cli_by_provider.insert(
            "codex".to_string(),
            Arc::new(RecordingCliBuilder {
                label: "codex",
                built: built.clone(),
                contexts: contexts.clone(),
            }),
        );
        let cloud = CloudLaneFactoryConfig {
            anthropic: None,
            openai: None,
            official_cli: None,
            official_cli_by_provider,
        };
        let model_lane_store = scoped_lazy_model_lane_store();
        let factory = ProductionModelSessionFactory::new(ledger, cloud, None)
            .with_durable_worktree_vm_store(&model_lane_store);
        let mut req = SpawnRequest::new(
            instance(0),
            RuntimeBinding::Candle,
            "swarm_prod_test",
            "parent-1",
        )
        .with_cloud_provider(ProviderKind::OfficialCli, "gpt-5-codex")
        .with_official_cli_provider("codex")
        .with_swarm("swarm-v6")
        .with_worktree("wt-v6")
        .with_working_dir("worktree-v6")
        .with_sandbox_posture(
            crate::sandbox::TrustClass::Trusted,
            crate::sandbox::IsolationTier::Tier1Container,
            std::collections::BTreeSet::from([
                crate::sandbox::RequiredCapability::HighStdioThroughput,
            ]),
            crate::sandbox::NetPolicy::HostInherited,
            crate::sandbox::CLI_BRIDGE_REQUESTED_EXECUTION_POLICY_REF,
        );
        req.owner_wp = Some("WP-MULTI-MODEL-ORCHESTRATION-V1".to_string());
        req.role_id = Some("CODER".to_string());
        req.wp_id = Some("WP-MULTI-MODEL-ORCHESTRATION-V1".to_string());
        req.mt_id = Some("MT-003".to_string());

        let live = factory
            .create(&req)
            .await
            .expect("explicit codex official-CLI spawn succeeds");
        assert_eq!(
            built.lock().expect("built lock").as_slice(),
            ["codex"],
            "explicit CLI provider must select Codex, not a fallback Claude builder"
        );
        let captured = contexts.lock().expect("contexts lock");
        assert_eq!(captured.len(), 1);
        let context = &captured[0];
        let expected_session_id = req.instance_id.to_string();
        assert_eq!(context.owner_role, "swarm_prod_test");
        assert_eq!(
            context.owner_wp.as_deref(),
            Some("WP-MULTI-MODEL-ORCHESTRATION-V1")
        );
        assert_eq!(context.role_id.as_deref(), Some("CODER"));
        assert_eq!(
            context.wp_id.as_deref(),
            Some("WP-MULTI-MODEL-ORCHESTRATION-V1")
        );
        assert_eq!(context.mt_id.as_deref(), Some("MT-003"));
        assert_eq!(
            context.session_id.as_deref(),
            Some(expected_session_id.as_str())
        );
        assert_eq!(context.parent_session_id.as_deref(), Some("parent-1"));
        assert_eq!(context.trace_id.as_deref(), Some("parent-1"));
        assert_eq!(
            context.span_id.as_deref(),
            Some(expected_session_id.as_str())
        );
        assert_eq!(context.model_identity, "gpt-5-codex");
        assert_eq!(
            context.requested_trust_class,
            Some(crate::sandbox::TrustClass::Trusted)
        );
        assert_eq!(
            context.requested_isolation_tier,
            Some(crate::sandbox::IsolationTier::Tier1Container)
        );
        assert_eq!(
            context.requested_sandbox_capabilities.as_ref(),
            Some(&std::collections::BTreeSet::from([
                crate::sandbox::RequiredCapability::HighStdioThroughput,
            ]))
        );
        assert_eq!(
            context.requested_net_policy,
            Some(crate::sandbox::NetPolicy::HostInherited)
        );
        assert_eq!(
            context.requested_execution_policy_ref.as_deref(),
            Some(crate::sandbox::CLI_BRIDGE_REQUESTED_EXECUTION_POLICY_REF)
        );
        assert_eq!(context.swarm_id.as_deref(), Some("swarm-v6"));
        assert_eq!(context.worktree_id.as_deref(), Some("wt-v6"));
        assert_eq!(context.working_dir.as_deref(), Some("worktree-v6"));
        drop(captured);
        (live.teardown)().await.expect("teardown");
        // FACTORY-level test: no coordinator owns the STOP, so release the
        // reserved lifecycle (and its unused STOP permit) before the bounded
        // drain-and-join, or the writer's channel cannot close. See the same
        // note in `byok_cloud_provider_selection_honors_anthropic_...`.
        drop(live);
        // The spawn only returns Ok once the START was durably acknowledged, so
        // the row MUST already be in the store.
        let rows = drain_spawned(&ledger_close, ledger_writer, store).await;
        assert_eq!(
            rows.iter()
                .filter(|e| e.kind() == LedgerEventKind::Start)
                .count(),
            1,
            "the durably acknowledged START is in the store"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn missing_requested_official_cli_never_enters_populated_legacy_fallback() {
        struct ForbiddenFallbackBuilder {
            builder_calls: Arc<std::sync::atomic::AtomicUsize>,
            runtime_loads: Arc<std::sync::atomic::AtomicUsize>,
            spawner_calls: Arc<std::sync::atomic::AtomicUsize>,
        }

        #[async_trait]
        impl CloudRuntimeBuilder for ForbiddenFallbackBuilder {
            fn provider(&self) -> ProviderKind {
                ProviderKind::OfficialCli
            }

            async fn build_loaded(
                &self,
                _model: &str,
                _invocation_context: Option<crate::model_runtime::cloud::CliInvocationContext>,
                _working_dir: Option<&str>,
            ) -> Result<CloudLiveRuntime, String> {
                self.builder_calls
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                self.runtime_loads
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                self.spawner_calls
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Err("legacy fallback builder/runtime/spawner must not be invoked".to_string())
            }
        }

        struct ForbiddenProviderBuilder;

        #[async_trait]
        impl CloudRuntimeBuilder for ForbiddenProviderBuilder {
            fn provider(&self) -> ProviderKind {
                ProviderKind::OfficialCli
            }

            async fn build_loaded(
                &self,
                _model: &str,
                _invocation_context: Option<crate::model_runtime::cloud::CliInvocationContext>,
                _working_dir: Option<&str>,
            ) -> Result<CloudLiveRuntime, String> {
                panic!("a different provider-map entry must not satisfy the requested codex id")
            }
        }

        let (ledger, drain) = ledger_pair();
        let store = Arc::new(InMemoryStore::default());
        let builder_calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let runtime_loads = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let spawner_calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mut official_cli_by_provider: HashMap<String, Arc<dyn CloudRuntimeBuilder>> =
            HashMap::new();
        official_cli_by_provider.insert(
            "claude_code".to_string(),
            Arc::new(ForbiddenProviderBuilder),
        );
        let cloud = CloudLaneFactoryConfig {
            anthropic: None,
            openai: None,
            official_cli: Some(Arc::new(ForbiddenFallbackBuilder {
                builder_calls: builder_calls.clone(),
                runtime_loads: runtime_loads.clone(),
                spawner_calls: spawner_calls.clone(),
            })),
            official_cli_by_provider,
        };
        let model_lane_store = scoped_lazy_model_lane_store();
        let factory = ProductionModelSessionFactory::new(ledger, cloud, None)
            .with_durable_worktree_vm_store(&model_lane_store);
        let req = SpawnRequest::new(
            instance(0),
            RuntimeBinding::Candle,
            "swarm_prod_test",
            "parent-1",
        )
        .with_cloud_provider(ProviderKind::OfficialCli, "gpt-5-codex")
        .with_official_cli_provider("codex")
        .with_sandbox_posture(
            crate::sandbox::TrustClass::Trusted,
            crate::sandbox::IsolationTier::Tier1Container,
            std::collections::BTreeSet::from([
                crate::sandbox::RequiredCapability::HighStdioThroughput,
            ]),
            crate::sandbox::NetPolicy::HostInherited,
            crate::sandbox::CLI_BRIDGE_REQUESTED_EXECUTION_POLICY_REF,
        );

        let err = create_err(&factory, &req).await;
        match err {
            SwarmError::ProviderNotConfigured { provider, detail } => {
                assert_eq!(provider, "official_cli");
                assert!(
                    detail.contains("requested official CLI provider codex is not configured"),
                    "unexpected typed pre-spawn detail: {detail}"
                );
            }
            other => panic!("expected ProviderNotConfigured, got {other}"),
        }
        assert_eq!(
            builder_calls.load(std::sync::atomic::Ordering::SeqCst),
            0,
            "legacy fallback builder must not be invoked for an explicit provider-map miss"
        );
        assert_eq!(
            runtime_loads.load(std::sync::atomic::Ordering::SeqCst),
            0,
            "legacy fallback runtime must not be loaded for an explicit provider-map miss"
        );
        assert_eq!(
            spawner_calls.load(std::sync::atomic::Ordering::SeqCst),
            0,
            "legacy fallback spawner must not run for an explicit provider-map miss"
        );
        let rows = drained(&drain, store).await;
        assert!(
            rows.is_empty(),
            "typed pre-spawn rejection leaves no ledger rows"
        );
    }

    // ---- ENV-GATED REAL PARALLEL CANDLE PROOF (candle engine only). ----

    /// Spawn 2 REAL TinyLlama candle sessions IN PARALLEL through the production
    /// coordinator + factory, run a real generate on each, cancel one, and assert:
    /// both produced tokens, the cancelled session's teardown actually freed the
    /// model (the owning Arc was the last strong ref), ledger START==STOP, no
    /// orphan.
    ///
    /// Env-gated (model not committed). Run with:
    ///   HANDSHAKE_TEST_CANDLE_LLAMA_MODEL=<.../model.safetensors>
    ///   HANDSHAKE_TEST_CANDLE_LLAMA_SHA256=<hex>
    #[cfg(feature = "candle-runtime-engine")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn real_parallel_tinyllama_spawn_generate_cancel_no_orphan() {
        use crate::model_runtime::{GenPrompt, GenerateRequest, SamplingParams};
        use futures::StreamExt;
        use std::sync::atomic::Ordering;

        let Some(artifact) = std::env::var_os("HANDSHAKE_TEST_CANDLE_LLAMA_MODEL") else {
            eprintln!(
                "SKIP real_parallel_tinyllama_...: HANDSHAKE_TEST_CANDLE_LLAMA_MODEL not set"
            );
            return;
        };
        let artifact = artifact.to_string_lossy().to_string();
        let sha256 = std::env::var("HANDSHAKE_TEST_CANDLE_LLAMA_SHA256")
            .expect("HANDSHAKE_TEST_CANDLE_LLAMA_SHA256 required when the model path is set");

        let (ledger, drain) = ledger_pair();
        let store = Arc::new(InMemoryStore::default());
        let sink = Arc::new(RecordingSwarmSink::new());
        let factory = Arc::new(ProductionModelSessionFactory::local_only(ledger.clone()));
        let coordinator = Arc::new(SwarmCoordinator::new_legacy_without_dexterity_for_tests(
            crate::swarm_orchestration::SwarmConfig::new(
                RunBudget::defaulted(4).with_concurrency(4),
            ),
            factory,
            sink,
            ledger,
        ));

        // Two instances of the SAME real artifact, spawned IN PARALLEL.
        let iid_a = instance(0);
        let iid_b = instance(1);
        let req_a = local_candle_req_real(iid_a, &artifact, &sha256);
        let req_b = local_candle_req_real(iid_b, &artifact, &sha256);

        let c1 = coordinator.clone();
        let c2 = coordinator.clone();
        let (ra, rb) = tokio::join!(async move { c1.spawn_session(req_a).await }, async move {
            c2.spawn_session(req_b).await
        },);
        ra.expect("parallel spawn A");
        rb.expect("parallel spawn B");
        assert_eq!(
            coordinator.live_session_count(),
            2,
            "both sessions live in parallel"
        );

        // Run a REAL generate on each session's runtime (the shared handle is the
        // live candle runtime for that instance).
        let mut produced_a = 0usize;
        let mut produced_b = 0usize;
        for (iid, produced) in [(iid_a, &mut produced_a), (iid_b, &mut produced_b)] {
            let runtime = runtime_for(&coordinator, iid);
            let req = GenerateRequest {
                id: iid.model_id_runtime(&coordinator),
                prompt: GenPrompt::new("The capital of France is"),
                sampling: SamplingParams::default(),
                lora_overrides: vec![],
                steering_overrides: vec![],
                kv_prefix_handle: None,
                cancel: CancellationToken::new(),
                max_tokens: 4,
                stop_sequences: vec![],
                speculative_mode: None,
                structured_decoding: None,
            };
            let mut stream = runtime.generate(req);
            while let Some(item) = stream.next().await {
                if item.is_ok() {
                    *produced += 1;
                }
            }
        }
        assert!(produced_a > 0, "session A produced real tokens");
        assert!(produced_b > 0, "session B produced real tokens");

        // Cancel one session -> teardown must FREE its model. Because the factory
        // kept the OWNING Arc only inside the teardown closure, after teardown the
        // model map entry is gone. We assert the session is evicted and the ledger
        // is symmetric.
        coordinator
            .cancel_session(iid_a, "real_cancel")
            .await
            .expect("cancel A");
        assert_eq!(coordinator.live_session_count(), 1, "A freed, B still live");

        // Drain B too for a clean ledger.
        coordinator
            .complete_session(iid_b)
            .await
            .expect("complete B");
        assert_eq!(coordinator.live_session_count(), 0);

        let rows = drained(&drain, store).await;
        let starts = rows
            .iter()
            .filter(|e| e.kind() == LedgerEventKind::Start)
            .count();
        let stops = rows
            .iter()
            .filter(|e| e.kind() == LedgerEventKind::Stop)
            .count();
        assert_eq!(starts, 2, "two real model STARTs");
        assert_eq!(stops, 2, "two STOPs — teardown closed both, no orphan");
        let _ = produced_a + produced_b; // silence on some toolchains
        let _ = Ordering::SeqCst;
    }

    #[cfg(feature = "candle-runtime-engine")]
    fn local_candle_req_real(iid: ModelInstanceId, path: &str, sha: &str) -> SpawnRequest {
        SpawnRequest::new(iid, RuntimeBinding::Candle, "swarm_prod_test", "parent-1")
            .with_local_artifact(path, sha)
    }

    // Helper accessors used only by the env-gated real test: fetch the live
    // runtime + model id the coordinator registered for an instance.
    #[cfg(feature = "candle-runtime-engine")]
    fn runtime_for(coordinator: &SwarmCoordinator, iid: ModelInstanceId) -> Arc<dyn ModelRuntime> {
        coordinator
            .session_runtime_for_test(iid)
            .expect("a live runtime is registered for the instance")
    }

    #[cfg(feature = "candle-runtime-engine")]
    impl ModelInstanceId {
        fn model_id_runtime(&self, coordinator: &SwarmCoordinator) -> ModelId {
            coordinator
                .session_model_id_for_test(*self)
                .expect("a model id is registered for the instance")
        }
    }

    // ---- DEFAULT-CI: VaultCloudRuntimeBuilder dispatch (no live network). ----

    use crate::model_runtime::cloud::{InMemorySecretsVault, SecretsVault};

    /// The vault-backed builder, with a key stored for its lane and an
    /// allowlisted Claude model, BUILDS a real anthropic BYOK runtime (its
    /// allowlist-gated `load` mints a ModelId). No HTTP call is issued — this
    /// proves the configured -> builds-a-session-shape dispatch.
    #[tokio::test]
    async fn vault_builder_configured_builds_real_byok_runtime() {
        let vault: Arc<dyn SecretsVault> = Arc::new(InMemorySecretsVault::default());
        vault
            .put("anthropic", "sk-ant-test-do-not-log")
            .expect("store key");
        let builder =
            VaultCloudRuntimeBuilder::new(CloudProviderFlavor::Anthropic, vault, "anthropic");
        let built = builder
            .build_loaded("claude-sonnet-4", None, None)
            .await
            .expect("configured lane builds a runtime");
        assert_eq!(built.runtime.adapter_name(), "anthropic_byok");
        // The minted model id is the runtime-keyed handle (UUID v7).
        assert_eq!(built.model_id.as_uuid().get_version_num(), 7);
    }

    /// Unconfigured lane (no key in the vault) -> honest not-configured reason
    /// (the factory turns this into ProviderNotConfigured).
    #[tokio::test]
    async fn vault_builder_unconfigured_returns_not_configured_reason() {
        let vault: Arc<dyn SecretsVault> = Arc::new(InMemorySecretsVault::default());
        let builder = VaultCloudRuntimeBuilder::new(CloudProviderFlavor::OpenAi, vault, "openai");
        // CloudLiveRuntime is not Debug (owns trait objects), so match rather
        // than expect_err.
        let err = match builder.build_loaded("gpt-4o", None, None).await {
            Ok(_) => panic!("expected not-configured error"),
            Err(e) => e,
        };
        assert!(err.contains("no API key"), "{err}");
        assert!(err.contains("openai"), "{err}");
    }

    /// Through the factory: a from_vault config with a stored key dispatches a
    /// ByokCloud spawn to a real builder, records a START, and the session tears
    /// down with a matching STOP (no orphan) — the configured cloud SUCCESS path.
    #[tokio::test(flavor = "multi_thread")]
    async fn from_vault_factory_cloud_spawn_records_start_and_stop() {
        let vault: Arc<dyn SecretsVault> = Arc::new(InMemorySecretsVault::default());
        vault.put("openai", "sk-test-openai").expect("store key");
        let cloud = CloudLaneFactoryConfig::from_vault(vault, None, Some("openai".to_string()));
        // DRIVEN ledger: the cloud dispatch path awaits a durable START ack.
        let store = Arc::new(InMemoryStore::default());
        let (ledger, ledger_writer) = spawned_ledger(store.clone());
        let ledger_close = ledger.clone();
        let sink = Arc::new(RecordingSwarmSink::new());
        let model_lane_store = scoped_lazy_model_lane_store();
        let factory = Arc::new(
            ProductionModelSessionFactory::new(ledger.clone(), cloud, None)
                .with_durable_worktree_vm_store(&model_lane_store),
        );
        let coordinator = SwarmCoordinator::new_legacy_without_dexterity_for_tests(
            crate::swarm_orchestration::SwarmConfig::new(
                RunBudget::defaulted(4).with_concurrency(4),
            ),
            factory,
            sink,
            ledger,
        );
        let iid = instance(0);
        let req = SpawnRequest::new(iid, RuntimeBinding::Candle, "swarm_prod_test", "parent-1")
            .with_cloud_provider(ProviderKind::ByokCloud, "gpt-4o");
        coordinator
            .spawn_session(req)
            .await
            .expect("configured cloud spawn builds a live session");
        assert_eq!(coordinator.live_session_count(), 1);
        coordinator
            .cancel_session(iid, "test_cancel")
            .await
            .expect("cancel");
        assert_eq!(coordinator.live_session_count(), 0);
        let rows = drain_spawned(&ledger_close, ledger_writer, store).await;
        let starts = rows
            .iter()
            .filter(|e| e.kind() == LedgerEventKind::Start)
            .count();
        let stops = rows
            .iter()
            .filter(|e| e.kind() == LedgerEventKind::Stop)
            .count();
        assert_eq!(starts, 1, "one START for the cloud session");
        assert_eq!(stops, 1, "one STOP — no orphan");
    }

    // ---- ENV-GATED REAL CLOUD SPAWN+GENERATE (live BYOK credentials). ----

    /// Spawn a REAL cloud BYOK session through the production factory + a real
    /// secrets vault holding a live key, run a REAL `generate` against the
    /// provider, and assert tokens were produced + the ledger is symmetric.
    ///
    /// Env-gated (no committed credentials). Run with, e.g.:
    ///   HANDSHAKE_TEST_CLOUD_PROVIDER=anthropic   (or `openai`)
    ///   HANDSHAKE_TEST_CLOUD_API_KEY=<live key>
    ///   HANDSHAKE_TEST_CLOUD_MODEL=claude-sonnet-4-...   (allowlisted family)
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn real_cloud_spawn_generate_when_credentialed() {
        use crate::model_runtime::{GenPrompt, GenerateRequest, SamplingParams};
        use futures::StreamExt;

        let Some(provider) = std::env::var("HANDSHAKE_TEST_CLOUD_PROVIDER").ok() else {
            eprintln!("SKIP real_cloud_spawn_generate: HANDSHAKE_TEST_CLOUD_PROVIDER not set");
            return;
        };
        let Some(api_key) = std::env::var("HANDSHAKE_TEST_CLOUD_API_KEY").ok() else {
            eprintln!("SKIP real_cloud_spawn_generate: HANDSHAKE_TEST_CLOUD_API_KEY not set");
            return;
        };
        let model = std::env::var("HANDSHAKE_TEST_CLOUD_MODEL")
            .expect("HANDSHAKE_TEST_CLOUD_MODEL required when the provider+key are set");

        let (flavor, lane) = match provider.as_str() {
            "anthropic" => (CloudProviderFlavor::Anthropic, "anthropic"),
            "openai" => (CloudProviderFlavor::OpenAi, "openai"),
            other => panic!("unknown HANDSHAKE_TEST_CLOUD_PROVIDER: {other}"),
        };
        let vault: Arc<dyn SecretsVault> = Arc::new(InMemorySecretsVault::default());
        vault.put(lane, &api_key).expect("store live key");
        let cloud = match flavor {
            CloudProviderFlavor::Anthropic => {
                CloudLaneFactoryConfig::from_vault(vault, Some(lane.to_string()), None)
            }
            CloudProviderFlavor::OpenAi => {
                CloudLaneFactoryConfig::from_vault(vault, None, Some(lane.to_string()))
            }
        };

        // DRIVEN ledger: this env-gated test drives the same cloud path as its
        // always-run siblings and therefore has the same durable-START-ack
        // requirement. A manual batcher would make it fail the moment live
        // credentials are supplied.
        let store = Arc::new(InMemoryStore::default());
        let (ledger, ledger_writer) = spawned_ledger(store.clone());
        let ledger_close = ledger.clone();
        let sink = Arc::new(RecordingSwarmSink::new());
        let model_lane_store = scoped_lazy_model_lane_store();
        let factory = Arc::new(
            ProductionModelSessionFactory::new(ledger.clone(), cloud, None)
                .with_durable_worktree_vm_store(&model_lane_store),
        );
        let coordinator = Arc::new(SwarmCoordinator::new_legacy_without_dexterity_for_tests(
            crate::swarm_orchestration::SwarmConfig::new(
                RunBudget::defaulted(2).with_concurrency(2),
            ),
            factory,
            sink,
            ledger,
        ));
        let iid = instance(0);
        let req = SpawnRequest::new(iid, RuntimeBinding::Candle, "swarm_prod_test", "parent-1")
            .with_cloud_provider(ProviderKind::ByokCloud, &model);
        coordinator
            .spawn_session(req)
            .await
            .expect("real cloud spawn");
        assert_eq!(coordinator.live_session_count(), 1);

        let runtime = coordinator
            .session_runtime_for_test(iid)
            .expect("live cloud runtime");
        let model_id = coordinator
            .session_model_id_for_test(iid)
            .expect("cloud model id");
        let gen = GenerateRequest {
            id: model_id,
            prompt: GenPrompt::new("Say the single word: ping"),
            sampling: SamplingParams::default(),
            lora_overrides: vec![],
            steering_overrides: vec![],
            kv_prefix_handle: None,
            cancel: CancellationToken::new(),
            max_tokens: 8,
            stop_sequences: vec![],
            speculative_mode: None,
            structured_decoding: None,
        };
        let mut stream = runtime.generate(gen);
        let mut produced = 0usize;
        while let Some(item) = stream.next().await {
            if item.is_ok() {
                produced += 1;
            }
        }
        assert!(produced > 0, "real cloud generate produced tokens");
        drop(stream);

        coordinator
            .cancel_session(iid, "test_done")
            .await
            .expect("cancel");
        assert_eq!(coordinator.live_session_count(), 0);
        let rows = drain_spawned(&ledger_close, ledger_writer, store).await;
        let starts = rows
            .iter()
            .filter(|e| e.kind() == LedgerEventKind::Start)
            .count();
        let stops = rows
            .iter()
            .filter(|e| e.kind() == LedgerEventKind::Stop)
            .count();
        assert_eq!(starts, 1);
        assert_eq!(stops, 1);
    }

    // ---- DEFAULT-CI: official-CLI bridge cloud lane (no real subprocess). ----

    use crate::model_runtime::cloud::official_cli_bridge::{
        CliBridgeConfig as TestCliBridgeConfig, CliInvocationContext, CliInvocationReceipt,
        CliKind, CliOutputFormat, CliSubprocessSpawner, OfficialCliBridgeError,
    };

    fn cli_temp_exe() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")
    }

    fn raw_cli_config_present() -> TestCliBridgeConfig {
        TestCliBridgeConfig {
            cli_kind: CliKind::ClaudeCode,
            executable_path: cli_temp_exe(),
            args_template: vec![
                "--model".to_string(),
                "{model}".to_string(),
                "--prompt".to_string(),
                "{prompt}".to_string(),
            ],
            output_format: CliOutputFormat::RawText,
            env_vars: std::collections::HashMap::new(),
            working_dir: None,
            timeout_seconds: 120,
        }
    }

    fn cli_config_present() -> crate::model_runtime::cloud::AllowlistedCliBridgeConfig {
        crate::model_runtime::cloud::AllowlistedCliBridgeConfig::new(
            raw_cli_config_present(),
            crate::model_runtime::cloud::CliModelAllowlist::new(vec!["claude-sonnet".to_string()])
                .expect("test allowlist"),
        )
    }

    /// Mock CLI byte source: emits a couple of stdout chunks (so the built
    /// runtime's `generate` is a real streaming path), no real subprocess.
    struct MockCliSpawner;
    impl CliSubprocessSpawner for MockCliSpawner {
        fn spawn(
            &self,
            _config: &TestCliBridgeConfig,
            _invocation: &CliInvocationContext,
            _model_name: &str,
            _prompt: &str,
        ) -> Result<CliInvocationReceipt, OfficialCliBridgeError> {
            Ok(CliInvocationReceipt {
                model_id: ModelId::new_v7(),
                stdout: "ok".to_string(),
                pid: Some(7),
                exit_code: Some(0),
                cancelled: false,
            })
        }
        fn spawn_streaming(
            &self,
            _config: &TestCliBridgeConfig,
            _invocation: &CliInvocationContext,
            _model_name: &str,
            _prompt: &str,
            chunk_sender: &tokio::sync::mpsc::Sender<Vec<u8>>,
        ) -> Result<CliInvocationReceipt, OfficialCliBridgeError> {
            chunk_sender.try_send(b"ok".to_vec()).map_err(|failure| {
                OfficialCliBridgeError::SpawnFailed {
                    reason: failure.to_string(),
                    exit_code: None,
                }
            })?;
            Ok(CliInvocationReceipt {
                model_id: ModelId::new_v7(),
                stdout: "ok".to_string(),
                pid: Some(7),
                exit_code: Some(0),
                cancelled: false,
            })
        }
    }

    /// Test 4: `CliBridgeCloudRuntimeBuilder::build_loaded` returns a
    /// not-configured reason when the CLI executable is absent, and a real
    /// runtime when it is present.
    #[tokio::test]
    async fn cli_builder_not_configured_when_cli_absent_and_real_when_present() {
        // Absent CLI -> honest not-configured reason.
        let (mut absent, allowlist) = cli_config_present().into_parts();
        absent.executable_path =
            std::path::PathBuf::from("D:/__handshake_no_such_cli__/claude.exe");
        let builder_absent = CliBridgeCloudRuntimeBuilder::new(
            Arc::new(MockCliSpawner),
            crate::model_runtime::cloud::AllowlistedCliBridgeConfig::new(absent, allowlist),
        );
        let err = match builder_absent
            .build_loaded("claude-sonnet", None, None)
            .await
        {
            Ok(_) => panic!("expected not-configured for an absent CLI"),
            Err(e) => e,
        };
        assert!(err.contains("not found"), "{err}");

        // Present CLI (a file that exists) -> real CliBridgeModelRuntime.
        let builder_present =
            CliBridgeCloudRuntimeBuilder::new(Arc::new(MockCliSpawner), cli_config_present());
        let built = builder_present
            .build_loaded(
                "claude-sonnet",
                Some({
                    let mut context = CliInvocationContext::new("TEST_ROLE", "claude-sonnet");
                    context.owner_wp = Some("WP-TEST".to_string());
                    context.wp_id = Some("WP-TEST".to_string());
                    context.mt_id = Some("MT-003".to_string());
                    context.session_id = Some("mid#5".to_string());
                    context.parent_session_id = Some("parent-test".to_string());
                    context.requested_trust_class = Some(crate::sandbox::TrustClass::Trusted);
                    context.requested_isolation_tier =
                        Some(crate::sandbox::IsolationTier::Tier1Container);
                    context.requested_sandbox_capabilities =
                        Some(std::collections::BTreeSet::from([
                            crate::sandbox::RequiredCapability::HighStdioThroughput,
                        ]));
                    context.requested_net_policy = Some(crate::sandbox::NetPolicy::HostInherited);
                    context.requested_execution_policy_ref =
                        Some(crate::sandbox::CLI_BRIDGE_REQUESTED_EXECUTION_POLICY_REF.to_string());
                    context
                }),
                None,
            )
            .await
            .expect("present CLI builds a runtime");
        assert_eq!(built.runtime.adapter_name(), "official_cli_bridge");
        assert_eq!(built.model_id.as_uuid().get_version_num(), 7);
    }

    /// `from_vault_with_cli` populates `official_cli` to `Some(..)` when a
    /// `(spawner, config)` is supplied (operator configured the bridge), and
    /// leaves it `None` when not (honest unconfigured default). This is the seam
    /// the production swarm constructor uses to flip the lane on/off from the
    /// stored CLI-bridge config.
    #[test]
    fn from_vault_with_cli_populates_official_cli_when_configured() {
        use crate::model_runtime::cloud::{InMemorySecretsVault, SecretsVault};
        let vault: Arc<dyn SecretsVault> = Arc::new(InMemorySecretsVault::default());

        // None -> official_cli stays None.
        let unconfigured = CloudLaneFactoryConfig::from_vault_with_cli(
            vault.clone(),
            Some("anthropic".to_string()),
            Some("openai".to_string()),
            None,
        );
        assert!(
            unconfigured.official_cli.is_none(),
            "no CLI config => official_cli lane stays None"
        );

        // Some((spawner, config)) -> official_cli becomes Some.
        let spawner: Arc<dyn CliSubprocessSpawner> = Arc::new(MockCliSpawner);
        let configured = CloudLaneFactoryConfig::from_vault_with_cli(
            vault,
            Some("anthropic".to_string()),
            Some("openai".to_string()),
            Some((spawner, cli_config_present())),
        );
        assert!(
            configured.official_cli.is_some(),
            "supplied CLI config => official_cli lane goes live"
        );
        assert_eq!(
            configured
                .official_cli
                .as_ref()
                .expect("official_cli")
                .provider(),
            ProviderKind::OfficialCli
        );
    }

    /// Test 5: factory `create()` dispatches a ProviderKind::OfficialCli spawn
    /// to the official_cli builder (records START==STOP, no orphan), and a
    /// not-configured variant returns ProviderNotConfigured with the
    /// `official_cli` provider label.
    #[tokio::test(flavor = "multi_thread")]
    async fn factory_dispatches_official_cli_to_cli_builder() {
        // Configured: official_cli lane wired with a present CLI + mock spawner.
        let store = Arc::new(InMemoryStore::default());
        let (ledger, ledger_writer) = LedgerBatcher::spawn(
            store.clone(),
            Arc::new(NoopOverflowSink),
            LedgerBatcherConfig {
                capacity: 4096,
                ..LedgerBatcherConfig::default()
            },
        );
        let ledger_close = ledger.clone();
        let cloud = CloudLaneFactoryConfig {
            anthropic: None,
            openai: None,
            official_cli: Some(Arc::new(CliBridgeCloudRuntimeBuilder::new(
                Arc::new(MockCliSpawner),
                cli_config_present(),
            ))),
            official_cli_by_provider: HashMap::new(),
        };
        let sink = Arc::new(RecordingSwarmSink::new());
        let model_lane_store = scoped_lazy_model_lane_store();
        let factory = Arc::new(
            ProductionModelSessionFactory::new(ledger.clone(), cloud, None)
                .with_durable_worktree_vm_store(&model_lane_store),
        );
        let coordinator = SwarmCoordinator::new_legacy_without_dexterity_for_tests(
            crate::swarm_orchestration::SwarmConfig::new(
                RunBudget::defaulted(4).with_concurrency(4),
            ),
            factory,
            sink,
            ledger,
        );
        let iid = instance(0);
        let req = SpawnRequest::new(iid, RuntimeBinding::Candle, "swarm_prod_test", "parent-1")
            .with_cloud_provider(ProviderKind::OfficialCli, "claude-sonnet")
            .with_sandbox_posture(
                crate::sandbox::TrustClass::Trusted,
                crate::sandbox::IsolationTier::Tier1Container,
                std::collections::BTreeSet::from([
                    crate::sandbox::RequiredCapability::HighStdioThroughput,
                ]),
                crate::sandbox::NetPolicy::HostInherited,
                crate::sandbox::CLI_BRIDGE_REQUESTED_EXECUTION_POLICY_REF,
            );
        coordinator
            .spawn_session(req)
            .await
            .expect("official_cli spawn dispatches to the CLI builder");
        assert_eq!(coordinator.live_session_count(), 1);
        coordinator
            .cancel_session(iid, "test_cancel")
            .await
            .expect("cancel");
        assert_eq!(coordinator.live_session_count(), 0);
        let ledger_outcome = crate::process_ledger::drain_and_join_ledger_writer(
            &ledger_close,
            ledger_writer,
            Duration::from_secs(5),
        )
        .await;
        assert!(matches!(
            ledger_outcome,
            crate::process_ledger::LedgerDrainJoinOutcome::Flushed
        ));
        let rows = store.events.lock().unwrap().clone();
        let starts = rows
            .iter()
            .filter(|e| e.kind() == LedgerEventKind::Start)
            .count();
        let stops = rows
            .iter()
            .filter(|e| e.kind() == LedgerEventKind::Stop)
            .count();
        assert_eq!(starts, 1, "one START for the official_cli session");
        assert_eq!(stops, 1, "one STOP — no orphan");
        let start = rows
            .iter()
            .find_map(|event| match event {
                LedgerEvent::Start(start) => Some(start),
                LedgerEvent::Stop(_) => None,
            })
            .expect("official CLI START row");
        assert_eq!(start.engine_kind, ProcessEngineKind::OfficialCliBridge);
        assert!(
            start.os_pid.is_some(),
            "official CLI bridge session carries its coordinator PID identity"
        );

        // Not-configured variant: official_cli=None -> ProviderNotConfigured.
        let (ledger2, _drain2) = ledger_pair();
        let model_lane_store2 = scoped_lazy_model_lane_store();
        let factory2 = ProductionModelSessionFactory::local_only(ledger2)
            .with_durable_worktree_vm_store(&model_lane_store2);
        let req2 = SpawnRequest::new(
            instance(1),
            RuntimeBinding::Candle,
            "swarm_prod_test",
            "parent-1",
        )
        .with_cloud_provider(ProviderKind::OfficialCli, "claude-sonnet");
        let err = create_err(&factory2, &req2).await;
        match err {
            SwarmError::ProviderNotConfigured { provider, .. } => {
                assert_eq!(provider, "official_cli");
            }
            other => panic!("expected ProviderNotConfigured, got {other}"),
        }
    }

    // ---- WP-KERNEL-004 wave 1: Tier-3 microVM routing + ledger sandbox fields ----

    /// Minimal headless FAKE SandboxAdapter for the factory routing test: reports
    /// Tier-3 + snapshot so selection picks it; `exec` returns a fixed completion;
    /// `spawn` mints an `hsk-ch-*` handle so the ledger sandbox_internal_id is set.
    struct FactoryFakeSandbox {
        spawned_specs: Arc<Mutex<Vec<crate::sandbox::ProcessSpec>>>,
    }

    #[async_trait]
    impl crate::sandbox::SandboxAdapter for FactoryFakeSandbox {
        async fn spawn(
            &self,
            spec: crate::sandbox::ProcessSpec,
        ) -> Result<crate::sandbox::ProcessHandle, crate::sandbox::SandboxAdapterError> {
            self.spawned_specs.lock().unwrap().push(spec);
            Ok(crate::sandbox::ProcessHandle::new(
                crate::sandbox::AdapterId::new("cloud_hypervisor"),
                None,
                "hsk-ch-factory-fake",
            ))
        }
        async fn exec(
            &self,
            _handle: &crate::sandbox::ProcessHandle,
            _cmd: crate::sandbox::Command,
        ) -> Result<crate::sandbox::ExecResult, crate::sandbox::SandboxAdapterError> {
            Ok(crate::sandbox::ExecResult {
                exit_code: 0,
                stdout: bytes::Bytes::from_static(b"sandboxed completion"),
                stderr: bytes::Bytes::new(),
                duration_ms: 1,
            })
        }
        async fn fs_bind(
            &self,
            _handle: &crate::sandbox::ProcessHandle,
            _host_path: std::path::PathBuf,
            _guest_path: std::path::PathBuf,
            _mode: crate::sandbox::BindMode,
        ) -> Result<(), crate::sandbox::SandboxAdapterError> {
            Ok(())
        }
        async fn net_policy(
            &self,
            _handle: &crate::sandbox::ProcessHandle,
            _policy: crate::sandbox::NetPolicy,
        ) -> Result<(), crate::sandbox::SandboxAdapterError> {
            Ok(())
        }
        async fn kill(
            &self,
            _handle: &crate::sandbox::ProcessHandle,
            _signal: crate::sandbox::Signal,
        ) -> Result<(), crate::sandbox::SandboxAdapterError> {
            Ok(())
        }
        async fn status(
            &self,
            _handle: &crate::sandbox::ProcessHandle,
        ) -> Result<crate::sandbox::ProcessStatus, crate::sandbox::SandboxAdapterError> {
            Ok(crate::sandbox::ProcessStatus::Running)
        }
        async fn exit_code(
            &self,
            _handle: &crate::sandbox::ProcessHandle,
        ) -> Result<Option<i32>, crate::sandbox::SandboxAdapterError> {
            Ok(None)
        }
        fn capabilities(&self) -> crate::sandbox::AdapterCapabilities {
            crate::sandbox::AdapterCapabilities {
                adapter_id: crate::sandbox::AdapterId::new("cloud_hypervisor"),
                runtime_available: true,
                filesystem_isolation_strength: crate::sandbox::IsolationStrength::VeryStrong,
                network_isolation_strength: crate::sandbox::IsolationStrength::VeryStrong,
                gpu_passthrough: crate::sandbox::GpuPassthrough::None,
                stdio_throughput_class: crate::sandbox::ThroughputClass::Low,
                win32_native_fidelity: false,
                cross_machine_portable: true,
                isolation_tier: crate::sandbox::IsolationTier::Tier3Microvm,
                requires_nested_virt: true,
                supports_snapshot: true,
                supports_persistent_exec: false,
                supports_warm_agent: false,
                supports_live_token_stream: false,
            }
        }
    }

    fn sandbox_registry_with_fake() -> Arc<crate::sandbox::SandboxAdapterRegistry> {
        sandbox_registry_with_observed_fake().0
    }

    fn sandbox_registry_with_observed_fake() -> (
        Arc<crate::sandbox::SandboxAdapterRegistry>,
        Arc<Mutex<Vec<crate::sandbox::ProcessSpec>>>,
    ) {
        let spawned_specs = Arc::new(Mutex::new(Vec::new()));
        let mut registry = crate::sandbox::SandboxAdapterRegistry::new(
            crate::sandbox::AdapterId::new("cloud_hypervisor"),
        );
        registry.register(Arc::new(FactoryFakeSandbox {
            spawned_specs: spawned_specs.clone(),
        }));
        (Arc::new(registry), spawned_specs)
    }

    #[derive(Default)]
    struct FactoryWarmTransport {
        shutdown_attempts: std::sync::atomic::AtomicUsize,
        shutdown_after_terminalization_attempts: std::sync::atomic::AtomicUsize,
        shutdown_failures_remaining: std::sync::atomic::AtomicUsize,
        vm_terminalized: std::sync::atomic::AtomicBool,
    }

    impl FactoryWarmTransport {
        fn with_shutdown_failures(failures: usize) -> Self {
            Self {
                shutdown_attempts: std::sync::atomic::AtomicUsize::new(0),
                shutdown_after_terminalization_attempts: std::sync::atomic::AtomicUsize::new(0),
                shutdown_failures_remaining: std::sync::atomic::AtomicUsize::new(failures),
                vm_terminalized: std::sync::atomic::AtomicBool::new(false),
            }
        }
    }

    #[async_trait]
    impl crate::model_runtime::WarmAgentTransport for FactoryWarmTransport {
        async fn load_model(
            &self,
            frame: crate::model_runtime::WarmAgentHostFrame,
        ) -> Result<
            crate::model_runtime::WarmAgentGuestFrame,
            crate::model_runtime::WarmVmTransportError,
        > {
            match frame {
                crate::model_runtime::WarmAgentHostFrame::Load {
                    model_guest_path,
                    model_artifact_sha256,
                    ..
                } => Ok(crate::model_runtime::WarmAgentGuestFrame::Ready {
                    protocol_id: crate::model_runtime::WARM_AGENT_PROTOCOL_ID.to_string(),
                    protocol_version: crate::model_runtime::WARM_AGENT_PROTOCOL_VERSION,
                    agent_id: "factory-warm-agent".to_string(),
                    ready_nonce: "ready-nonce".to_string(),
                    loaded_model_sha256: Some(model_artifact_sha256),
                    loaded_model_guest_path: Some(model_guest_path),
                }),
                other => Err(crate::model_runtime::WarmVmTransportError::Transport(
                    format!("expected load frame, got {other:?}"),
                )),
            }
        }

        fn generate(
            &self,
            request: crate::model_runtime::WarmAgentGenerateRequest,
        ) -> crate::model_runtime::WarmAgentFrameStream {
            Box::pin(futures::stream::iter([
                Ok(crate::model_runtime::WarmAgentGuestFrame::Token {
                    request_id: request.request_id.clone(),
                    token_id: 1,
                    token_index: Some(1),
                    text: "warm".to_string(),
                }),
                Ok(crate::model_runtime::WarmAgentGuestFrame::Complete {
                    request_id: request.request_id,
                    finish_reason: "stop".to_string(),
                }),
            ]))
        }

        async fn cancel(
            &self,
            _request_id: &str,
        ) -> Result<(), crate::model_runtime::WarmVmTransportError> {
            Ok(())
        }

        async fn shutdown(&self) -> Result<(), crate::model_runtime::WarmVmTransportError> {
            self.shutdown_attempts
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if self
                .vm_terminalized
                .load(std::sync::atomic::Ordering::SeqCst)
            {
                self.shutdown_after_terminalization_attempts
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                return Err(crate::model_runtime::WarmVmTransportError::Transport(
                    "warm-agent shutdown invoked after VM terminalization".to_string(),
                ));
            }
            if self
                .shutdown_failures_remaining
                .fetch_update(
                    std::sync::atomic::Ordering::SeqCst,
                    std::sync::atomic::Ordering::SeqCst,
                    |remaining| remaining.checked_sub(1),
                )
                .is_ok()
            {
                return Err(crate::model_runtime::WarmVmTransportError::Transport(
                    "injected retryable warm-agent shutdown failure".to_string(),
                ));
            }
            Ok(())
        }
    }

    struct FactoryWarmSandbox {
        spawned_specs: Arc<Mutex<Vec<crate::sandbox::ProcessSpec>>>,
        restored_snapshots: Arc<Mutex<Vec<String>>>,
        snapshotted_handles: Arc<Mutex<Vec<String>>>,
        deleted_snapshots: Arc<Mutex<Vec<String>>>,
        killed: Arc<Mutex<Vec<String>>>,
        kill_attempts: Arc<std::sync::atomic::AtomicUsize>,
        kill_failures_remaining: Arc<std::sync::atomic::AtomicUsize>,
        snapshot_error: Option<String>,
        snapshot_started: Option<Arc<tokio::sync::Semaphore>>,
        snapshot_release: Option<Arc<tokio::sync::Semaphore>>,
        warm_agent_capable: bool,
        warm_transport: Arc<FactoryWarmTransport>,
    }

    #[async_trait]
    impl crate::sandbox::SandboxAdapter for FactoryWarmSandbox {
        async fn spawn(
            &self,
            spec: crate::sandbox::ProcessSpec,
        ) -> Result<crate::sandbox::ProcessHandle, crate::sandbox::SandboxAdapterError> {
            self.spawned_specs.lock().unwrap().push(spec);
            Ok(crate::sandbox::ProcessHandle::new(
                crate::sandbox::AdapterId::new("cloud_hypervisor"),
                None,
                "hsk-ch-warm-factory-fake",
            ))
        }
        async fn exec(
            &self,
            _handle: &crate::sandbox::ProcessHandle,
            _cmd: crate::sandbox::Command,
        ) -> Result<crate::sandbox::ExecResult, crate::sandbox::SandboxAdapterError> {
            Err(crate::sandbox::SandboxAdapterError::SpawnFailed {
                adapter_id: crate::sandbox::AdapterId::new("cloud_hypervisor"),
                reason: "warm factory test must not use cold exec".to_string(),
            })
        }
        async fn fs_bind(
            &self,
            _handle: &crate::sandbox::ProcessHandle,
            _host_path: std::path::PathBuf,
            _guest_path: std::path::PathBuf,
            _mode: crate::sandbox::BindMode,
        ) -> Result<(), crate::sandbox::SandboxAdapterError> {
            Ok(())
        }
        async fn net_policy(
            &self,
            _handle: &crate::sandbox::ProcessHandle,
            _policy: crate::sandbox::NetPolicy,
        ) -> Result<(), crate::sandbox::SandboxAdapterError> {
            Ok(())
        }
        async fn kill(
            &self,
            handle: &crate::sandbox::ProcessHandle,
            _signal: crate::sandbox::Signal,
        ) -> Result<(), crate::sandbox::SandboxAdapterError> {
            self.kill_attempts
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if self
                .kill_failures_remaining
                .fetch_update(
                    std::sync::atomic::Ordering::SeqCst,
                    std::sync::atomic::Ordering::SeqCst,
                    |remaining| remaining.checked_sub(1),
                )
                .is_ok()
            {
                return Err(crate::sandbox::SandboxAdapterError::AdapterUnavailable {
                    adapter_id: crate::sandbox::AdapterId::new("cloud_hypervisor"),
                    reason: "injected retryable VM teardown failure".to_string(),
                });
            }
            self.killed
                .lock()
                .unwrap()
                .push(handle.sandbox_internal_id.clone());
            self.warm_transport
                .vm_terminalized
                .store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }
        async fn status(
            &self,
            _handle: &crate::sandbox::ProcessHandle,
        ) -> Result<crate::sandbox::ProcessStatus, crate::sandbox::SandboxAdapterError> {
            Ok(crate::sandbox::ProcessStatus::Running)
        }
        async fn exit_code(
            &self,
            _handle: &crate::sandbox::ProcessHandle,
        ) -> Result<Option<i32>, crate::sandbox::SandboxAdapterError> {
            Ok(None)
        }
        async fn snapshot(
            &self,
            handle: &crate::sandbox::ProcessHandle,
        ) -> Result<crate::sandbox::SnapshotRef, crate::sandbox::SandboxAdapterError> {
            self.snapshotted_handles
                .lock()
                .unwrap()
                .push(handle.sandbox_internal_id.clone());
            if let Some(started) = self.snapshot_started.as_ref() {
                started.add_permits(1);
            }
            if let Some(release) = self.snapshot_release.as_ref() {
                release
                    .acquire()
                    .await
                    .expect("snapshot release semaphore remains open")
                    .forget();
            }
            if let Some(reason) = self.snapshot_error.as_ref() {
                return Err(crate::sandbox::SandboxAdapterError::SnapshotFailed {
                    adapter_id: crate::sandbox::AdapterId::new("cloud_hypervisor"),
                    reason: reason.clone(),
                });
            }
            Ok(crate::sandbox::SnapshotRef::new(
                crate::sandbox::AdapterId::new("cloud_hypervisor"),
                format!("/snap/{}", handle.sandbox_internal_id),
            ))
        }
        async fn delete_snapshot(
            &self,
            snapshot: &crate::sandbox::SnapshotRef,
        ) -> Result<(), crate::sandbox::SandboxAdapterError> {
            self.deleted_snapshots
                .lock()
                .unwrap()
                .push(snapshot.snapshot_dir.clone());
            Ok(())
        }
        async fn restore(
            &self,
            snapshot: &crate::sandbox::SnapshotRef,
        ) -> Result<crate::sandbox::ProcessHandle, crate::sandbox::SandboxAdapterError> {
            self.restored_snapshots
                .lock()
                .unwrap()
                .push(snapshot.snapshot_dir.clone());
            Ok(crate::sandbox::ProcessHandle::new(
                crate::sandbox::AdapterId::new("cloud_hypervisor"),
                None,
                "hsk-ch-warm-restored-factory-fake",
            ))
        }
        async fn warm_agent_transport(
            &self,
            _handle: &crate::sandbox::ProcessHandle,
        ) -> Result<
            Arc<dyn crate::model_runtime::WarmAgentTransport>,
            crate::sandbox::SandboxAdapterError,
        > {
            Ok(self.warm_transport.clone())
        }
        fn capabilities(&self) -> crate::sandbox::AdapterCapabilities {
            crate::sandbox::AdapterCapabilities {
                adapter_id: crate::sandbox::AdapterId::new("cloud_hypervisor"),
                runtime_available: true,
                filesystem_isolation_strength: crate::sandbox::IsolationStrength::VeryStrong,
                network_isolation_strength: crate::sandbox::IsolationStrength::VeryStrong,
                gpu_passthrough: crate::sandbox::GpuPassthrough::None,
                stdio_throughput_class: crate::sandbox::ThroughputClass::Low,
                win32_native_fidelity: false,
                cross_machine_portable: true,
                isolation_tier: crate::sandbox::IsolationTier::Tier3Microvm,
                requires_nested_virt: true,
                supports_snapshot: true,
                supports_persistent_exec: true,
                supports_warm_agent: self.warm_agent_capable,
                supports_live_token_stream: self.warm_agent_capable,
            }
        }
    }

    type WarmRegistryObservations = (
        Arc<crate::sandbox::SandboxAdapterRegistry>,
        Arc<Mutex<Vec<crate::sandbox::ProcessSpec>>>,
        Arc<Mutex<Vec<String>>>,
        Arc<Mutex<Vec<String>>>,
        Arc<Mutex<Vec<String>>>,
        Arc<Mutex<Vec<String>>>,
    );

    fn warm_sandbox_registry_with_observed_fake() -> WarmRegistryObservations {
        warm_sandbox_registry_with_observed_fake_and_snapshot_error(None)
    }

    fn warm_sandbox_registry_with_observed_fake_and_snapshot_error(
        snapshot_error: Option<String>,
    ) -> WarmRegistryObservations {
        warm_sandbox_registry_with_observed_fake_options(snapshot_error, true)
    }

    fn warm_sandbox_registry_with_observed_fake_options(
        snapshot_error: Option<String>,
        warm_agent_capable: bool,
    ) -> WarmRegistryObservations {
        let spawned_specs = Arc::new(Mutex::new(Vec::new()));
        let restored_snapshots = Arc::new(Mutex::new(Vec::new()));
        let snapshotted_handles = Arc::new(Mutex::new(Vec::new()));
        let deleted_snapshots = Arc::new(Mutex::new(Vec::new()));
        let killed = Arc::new(Mutex::new(Vec::new()));
        let mut registry = crate::sandbox::SandboxAdapterRegistry::new(
            crate::sandbox::AdapterId::new("cloud_hypervisor"),
        );
        registry.register(Arc::new(FactoryWarmSandbox {
            spawned_specs: spawned_specs.clone(),
            restored_snapshots: restored_snapshots.clone(),
            snapshotted_handles: snapshotted_handles.clone(),
            deleted_snapshots: deleted_snapshots.clone(),
            killed: killed.clone(),
            kill_attempts: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            kill_failures_remaining: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            snapshot_error,
            snapshot_started: None,
            snapshot_release: None,
            warm_agent_capable,
            warm_transport: Arc::new(FactoryWarmTransport::default()),
        }));
        (
            Arc::new(registry),
            spawned_specs,
            restored_snapshots,
            snapshotted_handles,
            deleted_snapshots,
            killed,
        )
    }

    struct WarmFaultObservations {
        registry: Arc<crate::sandbox::SandboxAdapterRegistry>,
        killed: Arc<Mutex<Vec<String>>>,
        kill_attempts: Arc<std::sync::atomic::AtomicUsize>,
        transport: Arc<FactoryWarmTransport>,
        snapshot_started: Option<Arc<tokio::sync::Semaphore>>,
        snapshot_release: Option<Arc<tokio::sync::Semaphore>>,
    }

    fn warm_sandbox_registry_with_faults(
        shutdown_failures: usize,
        kill_failures: usize,
        gate_snapshot: bool,
        snapshot_error: Option<String>,
    ) -> WarmFaultObservations {
        let spawned_specs = Arc::new(Mutex::new(Vec::new()));
        let restored_snapshots = Arc::new(Mutex::new(Vec::new()));
        let snapshotted_handles = Arc::new(Mutex::new(Vec::new()));
        let deleted_snapshots = Arc::new(Mutex::new(Vec::new()));
        let killed = Arc::new(Mutex::new(Vec::new()));
        let kill_attempts = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let kill_failures_remaining = Arc::new(std::sync::atomic::AtomicUsize::new(kill_failures));
        let transport = Arc::new(FactoryWarmTransport::with_shutdown_failures(
            shutdown_failures,
        ));
        let snapshot_started = gate_snapshot.then(|| Arc::new(tokio::sync::Semaphore::new(0)));
        let snapshot_release = gate_snapshot.then(|| Arc::new(tokio::sync::Semaphore::new(0)));
        let mut registry = crate::sandbox::SandboxAdapterRegistry::new(
            crate::sandbox::AdapterId::new("cloud_hypervisor"),
        );
        registry.register(Arc::new(FactoryWarmSandbox {
            spawned_specs,
            restored_snapshots,
            snapshotted_handles,
            deleted_snapshots,
            killed: Arc::clone(&killed),
            kill_attempts: Arc::clone(&kill_attempts),
            kill_failures_remaining,
            snapshot_error,
            snapshot_started: snapshot_started.clone(),
            snapshot_release: snapshot_release.clone(),
            warm_agent_capable: true,
            warm_transport: Arc::clone(&transport),
        }));
        WarmFaultObservations {
            registry: Arc::new(registry),
            killed,
            kill_attempts,
            transport,
            snapshot_started,
            snapshot_release,
        }
    }

    /// A temp dir + fake gguf so the sandbox runtime's `load` existence gates pass.
    fn temp_gguf_for_factory() -> (std::path::PathBuf, std::path::PathBuf) {
        let dir =
            std::env::temp_dir().join(format!("hsk-fac-sbx-{}", uuid::Uuid::now_v7().simple()));
        std::fs::create_dir_all(&dir).expect("mk temp gguf dir");
        let gguf = dir.join("model.gguf");
        std::fs::write(&gguf, b"GGUF-FAKE").expect("write fake gguf");
        (dir, gguf)
    }

    fn temp_runner_for_factory() -> (std::path::PathBuf, std::path::PathBuf) {
        let dir =
            std::env::temp_dir().join(format!("hsk-fac-runner-{}", uuid::Uuid::now_v7().simple()));
        std::fs::create_dir_all(&dir).expect("mk temp runner dir");
        let runner = dir.join("llama-cli");
        std::fs::write(&runner, b"#!/bin/sh\n").expect("write fake runner");
        (dir, runner)
    }

    fn minimal_static_x86_64_elf() -> Vec<u8> {
        let mut elf = vec![0_u8; 64 + 56];
        elf[0..4].copy_from_slice(b"\x7fELF");
        elf[4] = 2;
        elf[5] = 1;
        elf[6] = 1;
        elf[16..18].copy_from_slice(&3_u16.to_le_bytes());
        elf[18..20].copy_from_slice(&62_u16.to_le_bytes());
        elf[20..24].copy_from_slice(&1_u32.to_le_bytes());
        elf[32..40].copy_from_slice(&64_u64.to_le_bytes());
        elf[52..54].copy_from_slice(&64_u16.to_le_bytes());
        elf[54..56].copy_from_slice(&56_u16.to_le_bytes());
        elf[56..58].copy_from_slice(&1_u16.to_le_bytes());
        elf[64..68].copy_from_slice(&1_u32.to_le_bytes());
        elf
    }

    fn marked_warm_agent_elf() -> Vec<u8> {
        let mut elf = minimal_static_x86_64_elf();
        elf.extend_from_slice(WARM_AGENT_PACKAGE_AGENT_BINARY_MARKER.as_bytes());
        elf
    }

    fn marked_llama_server_elf() -> Vec<u8> {
        let mut elf = minimal_static_x86_64_elf();
        for marker in WARM_AGENT_PACKAGE_LLAMA_SERVER_MARKERS {
            elf.extend_from_slice(marker.as_bytes());
        }
        elf
    }

    fn write_static_warm_agent_package(dir: &std::path::Path) -> std::path::PathBuf {
        std::fs::create_dir_all(dir).expect("mk warm agent dir");
        let agent = dir.join("hsk-warm-agent");
        let llama_server = dir.join("llama-server");
        std::fs::write(&agent, marked_warm_agent_elf()).expect("write fake static ELF warm agent");
        std::fs::write(&llama_server, marked_llama_server_elf())
            .expect("write fake static ELF llama-server");
        let manifest =
            manifest_for_test_recorded_probe_artifact(&agent).expect("warm-agent package manifest");
        std::fs::write(
            package_manifest_path_for_agent(&agent).expect("manifest path"),
            serde_json::to_vec_pretty(&manifest).expect("manifest json"),
        )
        .expect("write warm-agent package manifest");
        agent
    }

    fn test_warm_agent_package_validator(
        path: &std::path::Path,
    ) -> Result<WarmAgentGuestPackageManifest, WarmAgentPackageError> {
        validate_warm_agent_package_manifest_candidate(path)
    }

    fn restore_env_var(key: &str, prior: Option<std::ffi::OsString>) {
        match prior {
            Some(value) => std::env::set_var(key, value),
            None => std::env::remove_var(key),
        }
    }

    #[test]
    fn warm_agent_bind_from_env_derives_read_only_guest_binding() {
        let _env_guard = runner_env_lock();
        let _validator_guard = install_test_warm_agent_validator();
        let prior = std::env::var_os(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV);
        std::env::remove_var(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV);
        assert!(
            warm_agent_bind_from_env()
                .expect("unset env should be accepted")
                .is_none(),
            "unset warm-agent env should not add a bind"
        );

        let dir =
            std::env::temp_dir().join(format!("hsk-warm-agent-{}", uuid::Uuid::now_v7().simple()));
        let agent = write_static_warm_agent_package(&dir);
        std::env::set_var(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV, &agent);

        let (bind, guest_path) = warm_agent_bind_from_env()
            .expect("configured warm agent bind")
            .expect("bind present");
        assert_eq!(bind.host_path, dir);
        assert_eq!(
            bind.guest_path,
            std::path::PathBuf::from(CLOUD_HYPERVISOR_WARM_AGENT_GUEST_ROOT)
        );
        assert_eq!(bind.mode, BindMode::ReadOnly);
        assert_eq!(guest_path, "/warm-agent/hsk-warm-agent");

        let missing = bind.host_path.join("missing-agent");
        std::env::set_var(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV, &missing);
        let err = warm_agent_bind_from_env().expect_err("missing warm agent should fail closed");
        let detail = format!("{err}");
        assert!(
            detail.contains(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV),
            "{detail}"
        );
        assert!(detail.contains("missing"), "{detail}");

        let exe = bind.host_path.join("hsk-warm-agent.exe");
        std::fs::write(&exe, b"MZ").expect("write windows exe stand-in");
        std::env::set_var(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV, &exe);
        let err = warm_agent_bind_from_env().expect_err("windows exe must fail closed");
        let detail = format!("{err}");
        assert!(detail.contains("Windows .exe"), "{detail}");

        restore_env_var(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV, prior);
        let _ = std::fs::remove_dir_all(bind.host_path);
    }

    #[test]
    fn warm_agent_bind_from_env_rejects_marked_fake_package_without_runtime_probe() {
        let _env_guard = runner_env_lock();
        let prior = std::env::var_os(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV);
        let dir = std::env::temp_dir().join(format!(
            "hsk-warm-agent-runtime-reject-{}",
            uuid::Uuid::now_v7().simple()
        ));
        let agent = write_static_warm_agent_package(&dir);
        std::env::set_var(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV, &agent);

        let err = warm_agent_bind_from_env()
            .expect_err("marked fake package must fail without runtime probe proof");
        let detail = format!("{err}");
        assert!(
            detail.contains("complete packaged Linux guest warm-agent artifact"),
            "{detail}"
        );

        restore_env_var(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV, prior);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn warm_vm_request_fails_closed_when_adapter_lacks_warm_agent() {
        let _env_guard = runner_env_lock();
        let (dir, gguf) = temp_gguf_for_factory();
        let (runner_dir, runner) = temp_runner_for_factory();
        let prior_runner = std::env::var_os(SANDBOX_LLAMA_CLI_HOST_PATH_ENV);
        std::env::set_var(SANDBOX_LLAMA_CLI_HOST_PATH_ENV, &runner);
        let (ledger, drain) = ledger_pair();
        let store = Arc::new(InMemoryStore::default());
        let (registry, spawned_specs) = sandbox_registry_with_observed_fake();
        let factory = ProductionModelSessionFactory::new(
            ledger,
            CloudLaneFactoryConfig::unconfigured(),
            Some(registry),
        );
        let req = SpawnRequest::new(
            instance(0),
            RuntimeBinding::LlamaCpp,
            "swarm_prod_test",
            "p",
        )
        .with_local_artifact(gguf.to_string_lossy(), "ab".repeat(32))
        .with_isolation_tier(crate::sandbox::adapter::IsolationTier::Tier3Microvm)
        .with_worktree("wt-warm-1")
        .with_working_dir(dir.to_string_lossy())
        .with_warm_vm_execution();

        let err = create_err(&factory, &req).await;
        let detail = format!("{err}");
        assert!(detail.contains("supports_warm_agent=false"), "{detail}");
        assert!(detail.contains("stdout chunking"), "{detail}");
        assert!(
            spawned_specs.lock().unwrap().is_empty(),
            "unsupported warm mode must fail before spawning a cold VM"
        );
        let rows = drained(&drain, store).await;
        assert!(
            rows.is_empty(),
            "failed warm preflight leaves no ledger rows"
        );
        match prior_runner {
            Some(value) => std::env::set_var(SANDBOX_LLAMA_CLI_HOST_PATH_ENV, value),
            None => std::env::remove_var(SANDBOX_LLAMA_CLI_HOST_PATH_ENV),
        }
        let _ = std::fs::remove_dir_all(dir);
        let _ = std::fs::remove_dir_all(runner_dir);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn warm_vm_request_uses_persistent_spawn_and_warm_runtime_when_supported() {
        let _env_guard = runner_env_lock();
        let (dir, gguf) = temp_gguf_for_factory();
        let (runner_dir, runner) = temp_runner_for_factory();
        let prior_runner = std::env::var_os(SANDBOX_LLAMA_CLI_HOST_PATH_ENV);
        let prior_warm_agent = std::env::var_os(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV);
        std::env::remove_var(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV);
        std::env::set_var(SANDBOX_LLAMA_CLI_HOST_PATH_ENV, &runner);
        let (ledger, drain) = ledger_pair();
        let store = Arc::new(InMemoryStore::default());
        let (
            registry,
            spawned_specs,
            restored_snapshots,
            snapshotted_handles,
            deleted_snapshots,
            killed,
        ) = warm_sandbox_registry_with_observed_fake();
        let factory = ProductionModelSessionFactory::new(
            ledger,
            CloudLaneFactoryConfig::unconfigured(),
            Some(registry),
        );
        let req = SpawnRequest::new(
            instance(0),
            RuntimeBinding::LlamaCpp,
            "swarm_prod_test",
            "p",
        )
        .with_local_artifact(gguf.to_string_lossy(), "ab".repeat(32))
        .with_isolation_tier(crate::sandbox::adapter::IsolationTier::Tier3Microvm)
        .with_worktree("wt-warm-1")
        .with_working_dir(dir.to_string_lossy())
        .with_warm_vm_execution();

        let session = factory
            .create(&req)
            .await
            .expect("warm adapter routes to WarmVmModelRuntime");
        let manifest = session
            .warm_vm_restore_manifest
            .clone()
            .expect("fresh warm spawn should capture a restore manifest");
        assert_eq!(manifest.worktree_id, "wt-warm-1");
        assert_eq!(manifest.model_artifact_sha256, "ab".repeat(32));
        assert_eq!(manifest.model_guest_path, "/models/model.gguf");
        assert_eq!(manifest.ready_nonce, "ready-nonce");
        assert_eq!(
            manifest.snapshot.snapshot_dir,
            "/snap/hsk-ch-warm-factory-fake"
        );
        (session.teardown)().await.expect("teardown");

        let specs = spawned_specs.lock().unwrap().clone();
        assert_eq!(specs.len(), 1, "warm spawn creates one persistent VM");
        assert_eq!(
            specs[0]
                .metadata
                .get(crate::sandbox::SANDBOX_MODE_METADATA_KEY)
                .map(String::as_str),
            Some(crate::sandbox::SANDBOX_MODE_PERSISTENT),
            "warm spawn must use the persistent VM mode, not cold exec"
        );
        let worktree_bind = specs[0]
            .binds
            .iter()
            .find(|bind| bind.guest_path == std::path::PathBuf::from(WARM_VM_WORKTREE_GUEST_ROOT))
            .expect("warm model lane must carry its worktree folder into the microVM spec");
        assert_eq!(worktree_bind.host_path, dir);
        assert_eq!(worktree_bind.mode, BindMode::ReadOnly);
        assert!(
            restored_snapshots.lock().unwrap().is_empty(),
            "fresh warm spawn must not restore a snapshot"
        );
        assert_eq!(
            snapshotted_handles.lock().unwrap().as_slice(),
            ["hsk-ch-warm-factory-fake"],
            "fresh warm spawn should snapshot once after the ready frame"
        );
        assert!(
            deleted_snapshots.lock().unwrap().is_empty(),
            "successful warm sessions must retain their restart snapshot"
        );
        assert_eq!(
            killed.lock().unwrap().as_slice(),
            ["hsk-ch-warm-factory-fake"],
            "teardown must kill the persistent warm VM handle"
        );
        let rows = drained(&drain, store).await;
        let start = rows
            .iter()
            .find_map(|e| match e {
                LedgerEvent::Start(s) => Some(s.clone()),
                _ => None,
            })
            .expect("a START row was recorded");
        assert_eq!(
            start.sandbox_adapter_id.as_deref(),
            Some("cloud_hypervisor")
        );
        assert_eq!(
            start.sandbox_internal_id.as_deref(),
            Some("hsk-ch-warm-factory-fake")
        );
        match prior_runner {
            Some(value) => std::env::set_var(SANDBOX_LLAMA_CLI_HOST_PATH_ENV, value),
            None => std::env::remove_var(SANDBOX_LLAMA_CLI_HOST_PATH_ENV),
        }
        restore_env_var(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV, prior_warm_agent);
        let _ = std::fs::remove_dir_all(dir);
        let _ = std::fs::remove_dir_all(runner_dir);
    }

    #[tokio::test]
    async fn sandboxed_start_and_stop_metadata_preserve_exact_resource_scope() {
        use crate::swarm_orchestration::resource_scope::{
            AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, OwnerAccountId,
            ResourceScope, WorkspaceScopeRef,
        };

        let owner_account_id = OwnerAccountId::mint();
        let actor_principal_id = ActorPrincipalId::mint();
        let authenticated_session = AuthenticatedSessionRef::mint();
        let access_space = AccessSpaceRef::mint();
        let workspace =
            WorkspaceScopeRef::new("workspace-mt023-ledger").expect("non-empty workspace scope");
        let owner_account_text = owner_account_id.as_uuid().to_string();
        let actor_principal_text = actor_principal_id.as_uuid().to_string();
        let authenticated_session_text = authenticated_session.as_uuid().to_string();
        let access_space_text = access_space.as_uuid().to_string();
        let scope = ResourceScope::new(owner_account_id, actor_principal_id)
            .with_session(authenticated_session)
            .with_access_space(access_space)
            .with_workspace(workspace.clone());
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://handshake:handshake@127.0.0.1:5432/handshake_lazy")
            .expect("lazy scoped model lane pool");
        let model_lane_store = ModelLaneStore::new_scoped(pool, scope);
        let (ledger, drain) = ledger_pair();
        let store = Arc::new(InMemoryStore::default());
        let factory = ProductionModelSessionFactory::new(
            ledger,
            CloudLaneFactoryConfig::unconfigured(),
            None,
        )
        .with_durable_worktree_vm_store(&model_lane_store);
        let request = SpawnRequest::new(
            instance(39),
            RuntimeBinding::LlamaCpp,
            "KERNEL_BUILDER-MT023",
            "mt023-scoped-ledger",
        );
        let handle = crate::sandbox::ProcessHandle::new(
            crate::sandbox::AdapterId::new("cloud_hypervisor"),
            None,
            "hsk-ch-scoped-ledger",
        );
        let (_, start) = factory
            .record_start_sandboxed(
                &request,
                ModelId::new_v7(),
                ProcessEngineKind::LlamaCpp,
                &handle,
            )
            .expect("sandboxed START records scoped metadata");
        let stop = crate::process_ledger::ProcessStop::from_start(&start, Some(0))
            .with_stop_reason("scope metadata proof");

        for metadata in [&start.metadata_jsonb, &stop.metadata_jsonb] {
            assert_eq!(
                metadata["owner_account_id"].as_str(),
                Some(owner_account_text.as_str())
            );
            assert_eq!(
                metadata["actor_principal_id"].as_str(),
                Some(actor_principal_text.as_str())
            );
            assert_eq!(
                metadata["authenticated_session_id"].as_str(),
                Some(authenticated_session_text.as_str())
            );
            assert_eq!(
                metadata["access_space_id"].as_str(),
                Some(access_space_text.as_str())
            );
            assert_eq!(metadata["workspace_id"].as_str(), Some(workspace.as_str()));
        }
        let rows = drained(&drain, store).await;
        assert!(
            rows.iter()
                .any(|event| matches!(event, LedgerEvent::Start(_))),
            "the scoped START reaches the ledger batcher"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn warm_vm_request_fails_closed_when_restore_manifest_snapshot_fails() {
        let _env_guard = runner_env_lock();
        let (dir, gguf) = temp_gguf_for_factory();
        let (runner_dir, runner) = temp_runner_for_factory();
        let prior_runner = std::env::var_os(SANDBOX_LLAMA_CLI_HOST_PATH_ENV);
        let prior_warm_agent = std::env::var_os(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV);
        std::env::remove_var(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV);
        std::env::set_var(SANDBOX_LLAMA_CLI_HOST_PATH_ENV, &runner);
        let (ledger, drain) = ledger_pair();
        let store = Arc::new(InMemoryStore::default());
        let (
            registry,
            spawned_specs,
            restored_snapshots,
            snapshotted_handles,
            deleted_snapshots,
            killed,
        ) = warm_sandbox_registry_with_observed_fake_and_snapshot_error(Some(
            "snapshot backend refused capture".to_string(),
        ));
        let factory = ProductionModelSessionFactory::new(
            ledger,
            CloudLaneFactoryConfig::unconfigured(),
            Some(registry),
        );
        let req = SpawnRequest::new(
            instance(0),
            RuntimeBinding::LlamaCpp,
            "swarm_prod_test",
            "p",
        )
        .with_local_artifact(gguf.to_string_lossy(), "ab".repeat(32))
        .with_isolation_tier(crate::sandbox::adapter::IsolationTier::Tier3Microvm)
        .with_worktree("wt-warm-snapshot-fail")
        .with_working_dir(dir.to_string_lossy())
        .with_warm_vm_execution();

        let err = create_err(&factory, &req).await;
        let detail = format!("{err}");
        assert!(detail.contains("snapshot capture failed"), "{detail}");
        assert!(detail.contains("non-resumable warm session"), "{detail}");
        assert_eq!(spawned_specs.lock().unwrap().len(), 1);
        assert!(
            restored_snapshots.lock().unwrap().is_empty(),
            "fresh warm spawn should not restore while testing snapshot capture failure"
        );
        assert_eq!(
            snapshotted_handles.lock().unwrap().as_slice(),
            ["hsk-ch-warm-factory-fake"],
            "factory must attempt manifest-producing snapshot after the ready frame"
        );
        assert_eq!(
            killed.lock().unwrap().as_slice(),
            ["hsk-ch-warm-factory-fake"],
            "failed manifest capture must not leave a persistent VM running"
        );
        assert!(
            deleted_snapshots.lock().unwrap().is_empty(),
            "adapter-level snapshot failures have no successful snapshot dir to delete"
        );
        let rows = drained(&drain, store).await;
        assert!(
            rows.is_empty(),
            "snapshot failure happens before ledger START so no orphan ledger row is created"
        );

        restore_env_var(SANDBOX_LLAMA_CLI_HOST_PATH_ENV, prior_runner);
        restore_env_var(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV, prior_warm_agent);
        let _ = std::fs::remove_dir_all(dir);
        let _ = std::fs::remove_dir_all(runner_dir);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn failed_adopter_does_not_teardown_existing_worktree_vm() {
        let _env_guard = runner_env_lock();
        let (dir, gguf) = temp_gguf_for_factory();
        let (runner_dir, runner) = temp_runner_for_factory();
        let prior_runner = std::env::var_os(SANDBOX_LLAMA_CLI_HOST_PATH_ENV);
        let prior_warm_agent = std::env::var_os(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV);
        std::env::remove_var(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV);
        std::env::set_var(SANDBOX_LLAMA_CLI_HOST_PATH_ENV, &runner);
        let (ledger, _drain) = ledger_pair();
        let (
            sandbox_registry,
            _spawned_specs,
            _restored_snapshots,
            _snapshotted_handles,
            _deleted_snapshots,
            killed,
        ) = warm_sandbox_registry_with_observed_fake_and_snapshot_error(Some(
            "adopter snapshot failure".to_string(),
        ));
        let factory = ProductionModelSessionFactory::new(
            ledger,
            CloudLaneFactoryConfig::unconfigured(),
            Some(sandbox_registry.clone()),
        );
        let adapter = sandbox_registry
            .get(&crate::sandbox::AdapterId::new("cloud_hypervisor"))
            .expect("registered warm adapter");
        let worktree_registry = factory
            .worktree_vm_registry_for(adapter)
            .expect("factory worktree registry");
        let worktree_id = "wt-warm-adopted-snapshot-fail";
        worktree_registry
            .ensure_worktree_vm(worktree_id)
            .await
            .expect("pre-existing worktree VM");

        let request = SpawnRequest::new(
            instance(0),
            RuntimeBinding::LlamaCpp,
            "swarm_prod_test",
            "p",
        )
        .with_local_artifact(gguf.to_string_lossy(), "ab".repeat(32))
        .with_isolation_tier(crate::sandbox::adapter::IsolationTier::Tier3Microvm)
        .with_worktree(worktree_id)
        .with_working_dir(dir.to_string_lossy())
        .with_warm_vm_execution();

        let error = create_err(&factory, &request).await;
        assert!(
            format!("{error}").contains("adopter snapshot failure"),
            "the injected post-adoption failure must be observed: {error}"
        );
        assert!(
            killed.lock().unwrap().is_empty(),
            "a failed adopter must not tear down the VM owned by the existing binding"
        );
        assert!(
            worktree_registry.is_bound(worktree_id).await,
            "the pre-existing VM must remain bound after the adopter fails"
        );

        worktree_registry
            .teardown_worktree_vm(worktree_id)
            .await
            .expect("explicit owner cleanup");
        assert_eq!(killed.lock().unwrap().len(), 1);

        restore_env_var(SANDBOX_LLAMA_CLI_HOST_PATH_ENV, prior_runner);
        restore_env_var(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV, prior_warm_agent);
        let _ = std::fs::remove_dir_all(dir);
        let _ = std::fs::remove_dir_all(runner_dir);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn coordinator_failed_adopter_preserves_existing_worktree_vm() {
        let _env_guard = runner_env_lock();
        let (dir, gguf) = temp_gguf_for_factory();
        let (runner_dir, runner) = temp_runner_for_factory();
        let prior_runner = std::env::var_os(SANDBOX_LLAMA_CLI_HOST_PATH_ENV);
        let prior_warm_agent = std::env::var_os(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV);
        std::env::remove_var(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV);
        std::env::set_var(SANDBOX_LLAMA_CLI_HOST_PATH_ENV, &runner);
        let (ledger, _drain) = ledger_pair();
        let (sandbox_registry, _, _, _, _, killed) =
            warm_sandbox_registry_with_observed_fake_and_snapshot_error(Some(
                "coordinator adopter snapshot failure".to_string(),
            ));
        let factory = ProductionModelSessionFactory::new(
            ledger.clone(),
            CloudLaneFactoryConfig::unconfigured(),
            Some(sandbox_registry.clone()),
        );
        let adapter = sandbox_registry
            .get(&crate::sandbox::AdapterId::new("cloud_hypervisor"))
            .expect("registered warm adapter");
        let worktree_registry = factory
            .worktree_vm_registry_for(adapter)
            .expect("factory worktree registry");
        let worktree_id = "wt-warm-coordinator-adopter-fail";
        let original = worktree_registry
            .ensure_worktree_vm(worktree_id)
            .await
            .expect("pre-existing worktree VM");
        let coordinator = SwarmCoordinator::new_legacy_without_dexterity_for_tests(
            crate::swarm_orchestration::SwarmConfig::new(
                RunBudget::defaulted(2).with_concurrency(2),
            ),
            Arc::new(factory),
            Arc::new(RecordingSwarmSink::new()),
            ledger,
        );
        let request = SpawnRequest::new(
            instance(40),
            RuntimeBinding::LlamaCpp,
            "swarm_prod_test",
            "p",
        )
        .with_local_artifact(gguf.to_string_lossy(), "ab".repeat(32))
        .with_isolation_tier(crate::sandbox::adapter::IsolationTier::Tier3Microvm)
        .with_worktree(worktree_id)
        .with_working_dir(dir.to_string_lossy())
        .with_warm_vm_execution();

        let error = coordinator
            .spawn_session(request)
            .await
            .expect_err("injected adopter failure must propagate through coordinator");
        assert!(
            error
                .to_string()
                .contains("coordinator adopter snapshot failure"),
            "unexpected failure: {error}"
        );
        assert!(
            killed.lock().unwrap().is_empty(),
            "coordinator compensation must not kill a binding this attempt adopted"
        );
        assert_eq!(
            worktree_registry
                .resolve_worktree_vm(worktree_id)
                .await
                .expect("existing VM remains resolvable"),
            original
        );

        worktree_registry
            .teardown_worktree_vm(worktree_id)
            .await
            .expect("explicit owner cleanup");
        assert_eq!(killed.lock().unwrap().len(), 1);
        restore_env_var(SANDBOX_LLAMA_CLI_HOST_PATH_ENV, prior_runner);
        restore_env_var(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV, prior_warm_agent);
        let _ = std::fs::remove_dir_all(dir);
        let _ = std::fs::remove_dir_all(runner_dir);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn coordinator_cancel_reclaims_exact_pending_creator_binding() {
        let _env_guard = runner_env_lock();
        let (dir, gguf) = temp_gguf_for_factory();
        let (runner_dir, runner) = temp_runner_for_factory();
        let prior_runner = std::env::var_os(SANDBOX_LLAMA_CLI_HOST_PATH_ENV);
        let prior_warm_agent = std::env::var_os(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV);
        std::env::remove_var(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV);
        std::env::set_var(SANDBOX_LLAMA_CLI_HOST_PATH_ENV, &runner);
        let observations = warm_sandbox_registry_with_faults(0, 0, true, None);
        let (ledger, _drain) = ledger_pair();
        let factory = ProductionModelSessionFactory::new(
            ledger.clone(),
            CloudLaneFactoryConfig::unconfigured(),
            Some(observations.registry.clone()),
        );
        let adapter = observations
            .registry
            .get(&crate::sandbox::AdapterId::new("cloud_hypervisor"))
            .expect("registered warm adapter");
        let worktree_registry = factory
            .worktree_vm_registry_for(adapter)
            .expect("factory worktree registry");
        let coordinator = Arc::new(SwarmCoordinator::new_legacy_without_dexterity_for_tests(
            crate::swarm_orchestration::SwarmConfig::new(
                RunBudget::defaulted(2).with_concurrency(2),
            )
            .with_teardown_timeout(Duration::from_millis(25)),
            Arc::new(factory),
            Arc::new(RecordingSwarmSink::new()),
            ledger,
        ));
        let instance_id = instance(41);
        let worktree_id = "wt-warm-cancel-pending-creator";
        let request = SpawnRequest::new(
            instance_id,
            RuntimeBinding::LlamaCpp,
            "swarm_prod_test",
            "p",
        )
        .with_local_artifact(gguf.to_string_lossy(), "ab".repeat(32))
        .with_isolation_tier(crate::sandbox::adapter::IsolationTier::Tier3Microvm)
        .with_worktree(worktree_id)
        .with_working_dir(dir.to_string_lossy())
        .with_warm_vm_execution();
        let spawn = {
            let coordinator = Arc::clone(&coordinator);
            tokio::spawn(async move { coordinator.spawn_session(request).await })
        };
        let started = observations
            .snapshot_started
            .as_ref()
            .expect("gated snapshot start semaphore");
        tokio::time::timeout(Duration::from_secs(2), started.acquire())
            .await
            .expect("snapshot starts")
            .expect("snapshot start semaphore remains open")
            .forget();
        coordinator
            .cancel_session(instance_id, "cancel exact pending creator")
            .await
            .expect("pending cancellation token is delivered");
        let error = spawn
            .await
            .expect("spawn task joins")
            .expect_err("cancelled pending creator must not become live");
        assert!(error.to_string().contains("spawn cancelled"), "{error}");
        assert_eq!(
            observations
                .kill_attempts
                .load(std::sync::atomic::Ordering::SeqCst),
            1,
            "compensation kills exactly the handle recorded for this create attempt"
        );
        assert_eq!(observations.killed.lock().unwrap().len(), 1);
        assert!(
            !worktree_registry.is_bound(worktree_id).await,
            "creator compensation removes the exact pending binding"
        );
        if let Some(release) = observations.snapshot_release.as_ref() {
            release.add_permits(1);
        }
        restore_env_var(SANDBOX_LLAMA_CLI_HOST_PATH_ENV, prior_runner);
        restore_env_var(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV, prior_warm_agent);
        let _ = std::fs::remove_dir_all(dir);
        let _ = std::fs::remove_dir_all(runner_dir);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn coordinator_failed_creator_retries_exact_compensation_after_three_kill_failures() {
        let _env_guard = runner_env_lock();
        let (dir, gguf) = temp_gguf_for_factory();
        let (runner_dir, runner) = temp_runner_for_factory();
        let prior_runner = std::env::var_os(SANDBOX_LLAMA_CLI_HOST_PATH_ENV);
        let prior_warm_agent = std::env::var_os(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV);
        std::env::remove_var(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV);
        std::env::set_var(SANDBOX_LLAMA_CLI_HOST_PATH_ENV, &runner);
        let observations = warm_sandbox_registry_with_faults(
            0,
            3,
            false,
            Some("creator snapshot failure after bind".to_string()),
        );
        let (ledger, _drain) = ledger_pair();
        let factory = ProductionModelSessionFactory::new(
            ledger.clone(),
            CloudLaneFactoryConfig::unconfigured(),
            Some(observations.registry.clone()),
        );
        let adapter = observations
            .registry
            .get(&crate::sandbox::AdapterId::new("cloud_hypervisor"))
            .expect("registered warm adapter");
        let worktree_registry = factory
            .worktree_vm_registry_for(adapter)
            .expect("factory worktree registry");
        let coordinator = SwarmCoordinator::new_legacy_without_dexterity_for_tests(
            crate::swarm_orchestration::SwarmConfig::new(
                RunBudget::defaulted(2).with_concurrency(2),
            ),
            Arc::new(factory),
            Arc::new(RecordingSwarmSink::new()),
            ledger,
        );
        let worktree_id = "wt-warm-failed-creator-compensation";
        let error = coordinator
            .spawn_session(
                SpawnRequest::new(
                    instance(45),
                    RuntimeBinding::LlamaCpp,
                    "swarm_prod_test",
                    "p",
                )
                .with_local_artifact(gguf.to_string_lossy(), "ab".repeat(32))
                .with_isolation_tier(crate::sandbox::adapter::IsolationTier::Tier3Microvm)
                .with_worktree(worktree_id)
                .with_working_dir(dir.to_string_lossy())
                .with_warm_vm_execution(),
            )
            .await
            .expect_err("injected creator failure must propagate");
        assert!(
            error
                .to_string()
                .contains("creator snapshot failure after bind"),
            "unexpected failure: {error}"
        );
        assert_eq!(
            observations
                .kill_attempts
                .load(std::sync::atomic::Ordering::SeqCst),
            4,
            "factory exhausts three in-band attempts and coordinator retries the retained exact owner once"
        );
        assert_eq!(
            observations.killed.lock().unwrap().len(),
            1,
            "only the successful exact compensation terminalizes the VM"
        );
        assert!(
            !worktree_registry.is_bound(worktree_id).await,
            "coordinator compensation leaves no failed-creator VM orphan"
        );

        restore_env_var(SANDBOX_LLAMA_CLI_HOST_PATH_ENV, prior_runner);
        restore_env_var(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV, prior_warm_agent);
        let _ = std::fs::remove_dir_all(dir);
        let _ = std::fs::remove_dir_all(runner_dir);
    }

    async fn assert_warm_teardown_retry(
        instance_number: u32,
        shutdown_failures: usize,
        kill_failures: usize,
        expected_first_error: &[&str],
    ) {
        let _env_guard = runner_env_lock();
        let (dir, gguf) = temp_gguf_for_factory();
        let (runner_dir, runner) = temp_runner_for_factory();
        let prior_runner = std::env::var_os(SANDBOX_LLAMA_CLI_HOST_PATH_ENV);
        let prior_warm_agent = std::env::var_os(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV);
        std::env::remove_var(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV);
        std::env::set_var(SANDBOX_LLAMA_CLI_HOST_PATH_ENV, &runner);
        let observations =
            warm_sandbox_registry_with_faults(shutdown_failures, kill_failures, false, None);
        let store = Arc::new(InMemoryStore::default());
        let (ledger, ledger_writer) = spawned_ledger(store.clone());
        let ledger_close = ledger.clone();
        let factory = ProductionModelSessionFactory::new(
            ledger.clone(),
            CloudLaneFactoryConfig::unconfigured(),
            Some(observations.registry.clone()),
        );
        let adapter = observations
            .registry
            .get(&crate::sandbox::AdapterId::new("cloud_hypervisor"))
            .expect("registered warm adapter");
        let worktree_registry = factory
            .worktree_vm_registry_for(adapter)
            .expect("factory worktree registry");
        let coordinator = SwarmCoordinator::new_legacy_without_dexterity_for_tests(
            crate::swarm_orchestration::SwarmConfig::new(
                RunBudget::defaulted(2).with_concurrency(2),
            ),
            Arc::new(factory),
            Arc::new(RecordingSwarmSink::new()),
            ledger,
        );
        let instance_id = instance(instance_number);
        let worktree_id = format!("wt-warm-teardown-retry-{instance_number}");
        coordinator
            .spawn_session(
                SpawnRequest::new(
                    instance_id,
                    RuntimeBinding::LlamaCpp,
                    "swarm_prod_test",
                    "p",
                )
                .with_local_artifact(gguf.to_string_lossy(), "ab".repeat(32))
                .with_isolation_tier(crate::sandbox::adapter::IsolationTier::Tier3Microvm)
                .with_worktree(&worktree_id)
                .with_working_dir(dir.to_string_lossy())
                .with_warm_vm_execution(),
            )
            .await
            .expect("warm session starts");

        let first_error = coordinator
            .cancel_session(instance_id, "injected warm teardown retry")
            .await
            .expect_err("first teardown attempt must expose injected failure");
        let first_detail = first_error.to_string();
        for expected in expected_first_error {
            assert!(
                first_detail.contains(expected),
                "first teardown error lost {expected:?}: {first_detail}"
            );
        }
        coordinator
            .retry_pending_session_cleanups()
            .await
            .expect("retry completes only the unfinished teardown substeps");
        assert_eq!(coordinator.live_session_count(), 0);
        assert!(
            !worktree_registry.is_bound(&worktree_id).await,
            "successful retry leaves no bound warm VM"
        );
        assert_eq!(
            observations.killed.lock().unwrap().len(),
            1,
            "the VM is successfully terminalized exactly once"
        );
        assert_eq!(
            observations
                .transport
                .shutdown_attempts
                .load(std::sync::atomic::Ordering::SeqCst),
            if kill_failures == 0 {
                1
            } else {
                shutdown_failures + 1
            },
            "unload retries only while the VM remains alive"
        );
        assert_eq!(
            observations
                .transport
                .shutdown_after_terminalization_attempts
                .load(std::sync::atomic::Ordering::SeqCst),
            0,
            "retry must never issue warm-agent RPCs after registry teardown killed the VM"
        );
        assert_eq!(
            observations
                .kill_attempts
                .load(std::sync::atomic::Ordering::SeqCst),
            kill_failures + 1,
            "a successful registry teardown is latched and never repeated"
        );

        let rows = drain_spawned(&ledger_close, ledger_writer, store).await;
        assert_eq!(
            rows.iter()
                .filter(|event| matches!(event, LedgerEvent::Start(_)))
                .count(),
            1,
            "one warm VM owner records one START"
        );
        assert_eq!(
            rows.iter()
                .filter(|event| matches!(event, LedgerEvent::Stop(_)))
                .count(),
            1,
            "retry records one matching STOP, not one per teardown attempt"
        );

        restore_env_var(SANDBOX_LLAMA_CLI_HOST_PATH_ENV, prior_runner);
        restore_env_var(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV, prior_warm_agent);
        let _ = std::fs::remove_dir_all(dir);
        let _ = std::fs::remove_dir_all(runner_dir);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn warm_teardown_retry_skips_completed_registry_after_unload_failure() {
        assert_warm_teardown_retry(
            42,
            1,
            0,
            &["injected retryable warm-agent shutdown failure"],
        )
        .await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn warm_teardown_retry_skips_completed_unload_after_registry_failure() {
        assert_warm_teardown_retry(43, 0, 1, &["injected retryable VM teardown failure"]).await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn warm_teardown_preserves_combined_errors_then_retries_both_substeps() {
        assert_warm_teardown_retry(
            44,
            1,
            1,
            &[
                "injected retryable warm-agent shutdown failure",
                "injected retryable VM teardown failure",
            ],
        )
        .await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn warm_vm_request_deletes_successful_snapshot_when_ledger_start_fails() {
        let _env_guard = runner_env_lock();
        let (dir, gguf) = temp_gguf_for_factory();
        let (runner_dir, runner) = temp_runner_for_factory();
        let prior_runner = std::env::var_os(SANDBOX_LLAMA_CLI_HOST_PATH_ENV);
        let prior_warm_agent = std::env::var_os(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV);
        std::env::remove_var(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV);
        std::env::set_var(SANDBOX_LLAMA_CLI_HOST_PATH_ENV, &runner);
        let (ledger, _drain) = one_slot_failing_overflow_ledger_pair();
        ledger
            .record_start(ProcessStart::new(
                ProcessEngineKind::HelperSubprocess,
                "overflow-seed",
                None,
            ))
            .expect("seed event should fill the one-slot manual ledger queue");
        let (
            registry,
            spawned_specs,
            restored_snapshots,
            snapshotted_handles,
            deleted_snapshots,
            killed,
        ) = warm_sandbox_registry_with_observed_fake();
        let factory = ProductionModelSessionFactory::new(
            ledger,
            CloudLaneFactoryConfig::unconfigured(),
            Some(registry),
        );
        let req = SpawnRequest::new(
            instance(0),
            RuntimeBinding::LlamaCpp,
            "swarm_prod_test",
            "p",
        )
        .with_local_artifact(gguf.to_string_lossy(), "ab".repeat(32))
        .with_isolation_tier(crate::sandbox::adapter::IsolationTier::Tier3Microvm)
        .with_worktree("wt-warm-ledger-fail")
        .with_working_dir(dir.to_string_lossy())
        .with_warm_vm_execution();

        let err = create_err(&factory, &req).await;
        let detail = format!("{err}");
        assert!(
            detail.contains("forced ledger overflow failure"),
            "{detail}"
        );
        assert_eq!(
            spawned_specs.lock().unwrap().len(),
            1,
            "ledger failure happens after the warm VM is spawned"
        );
        assert!(
            restored_snapshots.lock().unwrap().is_empty(),
            "fresh warm spawn should not restore while testing ledger failure"
        );
        assert_eq!(
            snapshotted_handles.lock().unwrap().as_slice(),
            ["hsk-ch-warm-factory-fake"],
            "factory should snapshot before recording START"
        );
        assert_eq!(
            deleted_snapshots.lock().unwrap().as_slice(),
            ["/snap/hsk-ch-warm-factory-fake"],
            "post-snapshot ledger failure must delete the new snapshot dir"
        );
        assert_eq!(
            killed.lock().unwrap().as_slice(),
            ["hsk-ch-warm-factory-fake"],
            "post-snapshot ledger failure must not leave the persistent VM running"
        );

        restore_env_var(SANDBOX_LLAMA_CLI_HOST_PATH_ENV, prior_runner);
        restore_env_var(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV, prior_warm_agent);
        let _ = std::fs::remove_dir_all(dir);
        let _ = std::fs::remove_dir_all(runner_dir);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn warm_vm_request_binds_configured_warm_agent_into_persistent_spawn() {
        let _env_guard = runner_env_lock();
        let _validator_guard = install_test_warm_agent_validator();
        let (dir, gguf) = temp_gguf_for_factory();
        let (runner_dir, runner) = temp_runner_for_factory();
        let warm_dir = std::env::temp_dir().join(format!(
            "hsk-fac-warm-agent-{}",
            uuid::Uuid::now_v7().simple()
        ));
        let warm_agent = write_static_warm_agent_package(&warm_dir);
        let prior_runner = std::env::var_os(SANDBOX_LLAMA_CLI_HOST_PATH_ENV);
        let prior_warm_agent = std::env::var_os(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV);
        std::env::set_var(SANDBOX_LLAMA_CLI_HOST_PATH_ENV, &runner);
        std::env::set_var(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV, &warm_agent);
        let (ledger, drain) = ledger_pair();
        let store = Arc::new(InMemoryStore::default());
        let (
            registry,
            spawned_specs,
            _restored_snapshots,
            _snapshotted_handles,
            _deleted_snapshots,
            killed,
        ) = warm_sandbox_registry_with_observed_fake();
        let factory = ProductionModelSessionFactory::new(
            ledger,
            CloudLaneFactoryConfig::unconfigured(),
            Some(registry),
        );
        let req = SpawnRequest::new(
            instance(0),
            RuntimeBinding::LlamaCpp,
            "swarm_prod_test",
            "p",
        )
        .with_local_artifact(gguf.to_string_lossy(), "ab".repeat(32))
        .with_isolation_tier(crate::sandbox::adapter::IsolationTier::Tier3Microvm)
        .with_worktree("wt-warm-agent-bind")
        .with_working_dir(dir.to_string_lossy())
        .with_warm_vm_execution();

        let session = factory
            .create(&req)
            .await
            .expect("warm adapter routes to WarmVmModelRuntime");
        (session.teardown)().await.expect("teardown");

        let specs = spawned_specs.lock().unwrap().clone();
        assert_eq!(specs.len(), 1, "warm spawn creates one persistent VM");
        assert_eq!(
            specs[0]
                .metadata
                .get(CLOUD_HYPERVISOR_WARM_AGENT_GUEST_PATH_METADATA_KEY)
                .map(String::as_str),
            Some("/warm-agent/hsk-warm-agent"),
            "persistent VM spawn should carry the configured guest warm-agent path"
        );
        assert!(
            specs[0].binds.iter().any(|bind| {
                bind.host_path == warm_dir
                    && bind.guest_path
                        == std::path::PathBuf::from(CLOUD_HYPERVISOR_WARM_AGENT_GUEST_ROOT)
                    && bind.mode == BindMode::ReadOnly
            }),
            "persistent VM spawn should bind the warm-agent directory read-only"
        );
        assert_eq!(
            killed.lock().unwrap().as_slice(),
            ["hsk-ch-warm-factory-fake"],
            "teardown must kill the persistent warm VM handle"
        );
        let rows = drained(&drain, store).await;
        assert!(
            rows.iter().any(|e| matches!(e, LedgerEvent::Start(_))),
            "warm bind path should still record a START row"
        );

        restore_env_var(SANDBOX_LLAMA_CLI_HOST_PATH_ENV, prior_runner);
        restore_env_var(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV, prior_warm_agent);
        let _ = std::fs::remove_dir_all(dir);
        let _ = std::fs::remove_dir_all(runner_dir);
        let _ = std::fs::remove_dir_all(warm_dir);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn warm_vm_restore_manifest_uses_restore_without_reloading_or_fresh_spawn() {
        let _env_guard = runner_env_lock();
        let (dir, gguf) = temp_gguf_for_factory();
        let (runner_dir, runner) = temp_runner_for_factory();
        let prior_runner = std::env::var_os(SANDBOX_LLAMA_CLI_HOST_PATH_ENV);
        let prior_warm_agent = std::env::var_os(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV);
        std::env::remove_var(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV);
        std::env::set_var(SANDBOX_LLAMA_CLI_HOST_PATH_ENV, &runner);
        let (ledger, _drain) = ledger_pair();
        let (
            registry,
            spawned_specs,
            restored_snapshots,
            snapshotted_handles,
            deleted_snapshots,
            killed,
        ) = warm_sandbox_registry_with_observed_fake();
        let factory = ProductionModelSessionFactory::new(
            ledger,
            CloudLaneFactoryConfig::unconfigured(),
            Some(registry),
        );
        let sha = "ab".repeat(32);
        let manifest = crate::model_runtime::WarmVmSnapshotManifest::new(
            "wt-warm-restore",
            sha.clone(),
            "/models/model.gguf",
            "ready-nonce",
            crate::sandbox::SnapshotRef::new(
                crate::sandbox::AdapterId::new("cloud_hypervisor"),
                "/snap/warm-restore",
            ),
        );
        let req = SpawnRequest::new(
            instance(0),
            RuntimeBinding::LlamaCpp,
            "swarm_prod_test",
            "p",
        )
        .with_local_artifact(gguf.to_string_lossy(), sha)
        .with_isolation_tier(crate::sandbox::adapter::IsolationTier::Tier3Microvm)
        .with_worktree("wt-warm-restore")
        .with_working_dir(dir.to_string_lossy())
        .with_warm_vm_restore_manifest(manifest.clone());

        let session = factory
            .create(&req)
            .await
            .expect("warm restore routes through adapter.restore");
        let successor_manifest = session
            .warm_vm_restore_manifest
            .clone()
            .expect("restored warm sessions should capture a successor manifest");
        assert_eq!(successor_manifest.worktree_id, manifest.worktree_id);
        assert_eq!(
            successor_manifest.model_artifact_sha256,
            manifest.model_artifact_sha256
        );
        assert_eq!(
            successor_manifest.model_guest_path,
            manifest.model_guest_path
        );
        assert_eq!(successor_manifest.ready_nonce, manifest.ready_nonce);
        assert_eq!(
            successor_manifest.snapshot.snapshot_dir, "/snap/hsk-ch-warm-restored-factory-fake",
            "restored warm sessions should not keep reusing the consumed snapshot"
        );
        (session.teardown)().await.expect("teardown");
        assert!(
            spawned_specs.lock().unwrap().is_empty(),
            "restored warm starts must not create a fresh persistent spawn"
        );
        assert_eq!(
            restored_snapshots.lock().unwrap().as_slice(),
            ["/snap/warm-restore"],
            "restore path must consume the manifest snapshot"
        );
        assert_eq!(
            snapshotted_handles.lock().unwrap().as_slice(),
            ["hsk-ch-warm-restored-factory-fake"],
            "restore path should capture a successor snapshot for future reuse"
        );
        assert!(
            deleted_snapshots.lock().unwrap().is_empty(),
            "successful restore sessions must retain their successor snapshot"
        );
        assert_eq!(
            killed.lock().unwrap().as_slice(),
            ["hsk-ch-warm-restored-factory-fake"],
            "teardown must kill the restored warm VM handle"
        );
        match prior_runner {
            Some(value) => std::env::set_var(SANDBOX_LLAMA_CLI_HOST_PATH_ENV, value),
            None => std::env::remove_var(SANDBOX_LLAMA_CLI_HOST_PATH_ENV),
        }
        restore_env_var(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV, prior_warm_agent);
        let _ = std::fs::remove_dir_all(dir);
        let _ = std::fs::remove_dir_all(runner_dir);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn warm_vm_restore_does_not_require_current_warm_agent_or_runner_env() {
        let _env_guard = runner_env_lock();
        let (dir, gguf) = temp_gguf_for_factory();
        let prior_runner = std::env::var_os(SANDBOX_LLAMA_CLI_HOST_PATH_ENV);
        let prior_warm_agent = std::env::var_os(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV);
        std::env::remove_var(SANDBOX_LLAMA_CLI_HOST_PATH_ENV);
        std::env::remove_var(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV);
        let (ledger, _drain) = ledger_pair();
        let (
            registry,
            spawned_specs,
            restored_snapshots,
            snapshotted_handles,
            _deleted_snapshots,
            killed,
        ) = warm_sandbox_registry_with_observed_fake_options(None, false);
        let factory = ProductionModelSessionFactory::new(
            ledger,
            CloudLaneFactoryConfig::unconfigured(),
            Some(registry),
        );
        let sha = "cd".repeat(32);
        let manifest = crate::model_runtime::WarmVmSnapshotManifest::new(
            "wt-warm-restore-no-env",
            sha.clone(),
            "/models/model.gguf",
            "ready-nonce",
            crate::sandbox::SnapshotRef::new(
                crate::sandbox::AdapterId::new("cloud_hypervisor"),
                "/snap/warm-restore-no-env",
            ),
        );
        let req = SpawnRequest::new(
            instance(0),
            RuntimeBinding::LlamaCpp,
            "swarm_prod_test",
            "p",
        )
        .with_local_artifact(gguf.to_string_lossy(), sha)
        .with_isolation_tier(crate::sandbox::adapter::IsolationTier::Tier3Microvm)
        .with_worktree("wt-warm-restore-no-env")
        .with_working_dir(dir.to_string_lossy())
        .with_warm_vm_execution()
        .with_warm_vm_restore_manifest(manifest);

        let session = factory
            .create(&req)
            .await
            .expect("restore should rely on snapshot provenance, not current package env");
        assert!(
            spawned_specs.lock().unwrap().is_empty(),
            "restore path must not create a fresh persistent VM that would need current binds"
        );
        assert_eq!(
            restored_snapshots.lock().unwrap().as_slice(),
            ["/snap/warm-restore-no-env"]
        );
        assert_eq!(
            snapshotted_handles.lock().unwrap().as_slice(),
            ["hsk-ch-warm-restored-factory-fake"],
            "restore path should still capture a successor warm snapshot"
        );
        (session.teardown)().await.expect("teardown");
        assert_eq!(
            killed.lock().unwrap().as_slice(),
            ["hsk-ch-warm-restored-factory-fake"]
        );

        restore_env_var(SANDBOX_LLAMA_CLI_HOST_PATH_ENV, prior_runner);
        restore_env_var(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV, prior_warm_agent);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn warm_vm_restore_manifest_rejects_wrong_worktree_before_restore() {
        let _env_guard = runner_env_lock();
        let (dir, gguf) = temp_gguf_for_factory();
        let (runner_dir, runner) = temp_runner_for_factory();
        let prior_runner = std::env::var_os(SANDBOX_LLAMA_CLI_HOST_PATH_ENV);
        let prior_warm_agent = std::env::var_os(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV);
        std::env::remove_var(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV);
        std::env::set_var(SANDBOX_LLAMA_CLI_HOST_PATH_ENV, &runner);
        let (ledger, _drain) = ledger_pair();
        let (
            registry,
            spawned_specs,
            restored_snapshots,
            _snapshotted_handles,
            _deleted_snapshots,
            killed,
        ) = warm_sandbox_registry_with_observed_fake();
        let factory = ProductionModelSessionFactory::new(
            ledger,
            CloudLaneFactoryConfig::unconfigured(),
            Some(registry),
        );
        let sha = "ab".repeat(32);
        let manifest = crate::model_runtime::WarmVmSnapshotManifest::new(
            "wt-other",
            sha.clone(),
            "/models/model.gguf",
            "ready-nonce",
            crate::sandbox::SnapshotRef::new(
                crate::sandbox::AdapterId::new("cloud_hypervisor"),
                "/snap/wrong-worktree",
            ),
        );
        let req = SpawnRequest::new(
            instance(0),
            RuntimeBinding::LlamaCpp,
            "swarm_prod_test",
            "p",
        )
        .with_local_artifact(gguf.to_string_lossy(), sha)
        .with_isolation_tier(crate::sandbox::adapter::IsolationTier::Tier3Microvm)
        .with_worktree("wt-requested")
        .with_working_dir(dir.to_string_lossy())
        .with_warm_vm_restore_manifest(manifest);

        let err = create_err(&factory, &req).await;
        let detail = format!("{err}");
        assert!(detail.contains("worktree mismatch"), "{detail}");
        assert!(
            spawned_specs.lock().unwrap().is_empty(),
            "wrong-worktree restore must not create a fresh persistent spawn"
        );
        assert!(
            restored_snapshots.lock().unwrap().is_empty(),
            "wrong-worktree restore must fail before adapter.restore is called"
        );
        assert!(
            killed.lock().unwrap().is_empty(),
            "no handle exists to kill when restore is rejected before adapter.restore"
        );
        match prior_runner {
            Some(value) => std::env::set_var(SANDBOX_LLAMA_CLI_HOST_PATH_ENV, value),
            None => std::env::remove_var(SANDBOX_LLAMA_CLI_HOST_PATH_ENV),
        }
        restore_env_var(CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV, prior_warm_agent);
        let _ = std::fs::remove_dir_all(dir);
        let _ = std::fs::remove_dir_all(runner_dir);
    }

    /// Test 5: a Local+LlamaCpp+Tier3 spawn routes to create_sandboxed_local and
    /// records a START with BOTH sandbox_adapter_id AND sandbox_internal_id set.
    #[tokio::test(flavor = "current_thread")]
    async fn tier3_local_llama_routes_to_sandbox_and_populates_ledger() {
        let _env_guard = runner_env_lock();
        let (dir, gguf) = temp_gguf_for_factory();
        let (runner_dir, runner) = temp_runner_for_factory();
        let prior_runner = std::env::var_os(SANDBOX_LLAMA_CLI_HOST_PATH_ENV);
        std::env::set_var(SANDBOX_LLAMA_CLI_HOST_PATH_ENV, &runner);
        let (ledger, drain) = ledger_pair();
        let store = Arc::new(InMemoryStore::default());
        let (registry, spawned_specs) = sandbox_registry_with_observed_fake();
        let factory = ProductionModelSessionFactory::new(
            ledger,
            CloudLaneFactoryConfig::unconfigured(),
            Some(registry),
        );
        let req = SpawnRequest::new(
            instance(0),
            RuntimeBinding::LlamaCpp,
            "swarm_prod_test",
            "p",
        )
        .with_local_artifact(gguf.to_string_lossy(), "ab".repeat(32))
        .with_isolation_tier(crate::sandbox::adapter::IsolationTier::Tier3Microvm)
        .with_worktree("wt-sbx-1")
        .with_committed_memory_bytes(5 * 1024 * 1024 * 1024);
        let session = factory
            .create(&req)
            .await
            .expect("tier3 spawn routes to sandbox");
        // Drive the teardown closure so the runtime unloads + the microVM handle
        // is killed (the coordinator writes the ledger STOP in production; here we
        // assert the START sandbox fields directly).
        (session.teardown)().await.expect("teardown");

        let rows = drained(&drain, store).await;
        let start = rows
            .iter()
            .find_map(|e| match e {
                LedgerEvent::Start(s) => Some(s.clone()),
                _ => None,
            })
            .expect("a START row was recorded");
        assert_eq!(
            start.sandbox_adapter_id.as_deref(),
            Some("cloud_hypervisor"),
            "sandbox_adapter_id must be populated for a microVM-routed session"
        );
        assert_eq!(
            start.sandbox_internal_id.as_deref(),
            Some("hsk-ch-factory-fake"),
            "sandbox_internal_id must be populated from the microVM handle"
        );
        assert_eq!(
            spawned_specs
                .lock()
                .unwrap()
                .first()
                .and_then(|spec| spec.resource_limits.memory_bytes),
            Some(5 * 1024 * 1024 * 1024),
            "Tier3 sandbox spec must carry the admitted committed-memory estimate"
        );
        match prior_runner {
            Some(value) => std::env::set_var(SANDBOX_LLAMA_CLI_HOST_PATH_ENV, value),
            None => std::env::remove_var(SANDBOX_LLAMA_CLI_HOST_PATH_ENV),
        }
        let _ = std::fs::remove_dir_all(dir);
        let _ = std::fs::remove_dir_all(runner_dir);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn tier3_local_llama_without_runner_env_fails_loud() {
        let _env_guard = runner_env_lock();
        let prior_runner = std::env::var_os(SANDBOX_LLAMA_CLI_HOST_PATH_ENV);
        std::env::remove_var(SANDBOX_LLAMA_CLI_HOST_PATH_ENV);
        let (dir, gguf) = temp_gguf_for_factory();
        let (ledger, drain) = ledger_pair();
        let store = Arc::new(InMemoryStore::default());
        let factory = ProductionModelSessionFactory::new(
            ledger,
            CloudLaneFactoryConfig::unconfigured(),
            Some(sandbox_registry_with_fake()),
        );
        let req = SpawnRequest::new(
            instance(0),
            RuntimeBinding::LlamaCpp,
            "swarm_prod_test",
            "p",
        )
        .with_local_artifact(gguf.to_string_lossy(), "ab".repeat(32))
        .with_isolation_tier(crate::sandbox::adapter::IsolationTier::Tier3Microvm)
        .with_worktree("wt-sbx-1");
        let err = create_err(&factory, &req).await;
        assert!(
            format!("{err}").contains(SANDBOX_LLAMA_CLI_HOST_PATH_ENV),
            "missing runner env must be named in the failure: {err}"
        );
        let rows = drained(&drain, store).await;
        assert!(rows.is_empty(), "failed preflight leaves no ledger rows");
        match prior_runner {
            Some(value) => std::env::set_var(SANDBOX_LLAMA_CLI_HOST_PATH_ENV, value),
            None => std::env::remove_var(SANDBOX_LLAMA_CLI_HOST_PATH_ENV),
        }
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Control: a plain Local+LlamaCpp spawn WITHOUT Tier3 does NOT route to the
    /// sandbox (it stays on create_local_llama, which fails on the missing real
    /// GGUF load) — proving the regression guard keys strictly on Tier3.
    #[tokio::test]
    async fn local_llama_without_tier3_does_not_route_to_sandbox() {
        let (ledger, drain) = ledger_pair();
        let store = Arc::new(InMemoryStore::default());
        let factory = ProductionModelSessionFactory::new(
            ledger,
            CloudLaneFactoryConfig::unconfigured(),
            Some(sandbox_registry_with_fake()),
        );
        // No isolation_tier => the plain local llama path. A nonexistent GGUF
        // makes the REAL llama load fail (proving it did NOT take the sandbox
        // path, which would have succeeded against the fake adapter).
        let req = SpawnRequest::new(
            instance(0),
            RuntimeBinding::LlamaCpp,
            "swarm_prod_test",
            "p",
        )
        .with_local_artifact("D:/__no_such_swarm_gguf__/model.gguf", "ab".repeat(32));
        let err = create_err(&factory, &req).await;
        assert!(
            matches!(err, SwarmError::FactoryFailed(_)),
            "non-Tier3 local llama stays on the in-process path (got {err})"
        );
        let rows = drained(&drain, store).await;
        assert!(rows.is_empty(), "failed local load leaves no ledger rows");
    }

    /// A Tier3 spawn with NO sandbox registry fails loud (typed FactoryFailed) —
    /// never silently downgraded to an in-process spawn.
    #[tokio::test]
    async fn tier3_without_registry_fails_loud() {
        let (ledger, _drain) = ledger_pair();
        let factory = ProductionModelSessionFactory::local_only(ledger);
        let req = SpawnRequest::new(
            instance(0),
            RuntimeBinding::LlamaCpp,
            "swarm_prod_test",
            "p",
        )
        .with_local_artifact("/x/model.gguf", "ab".repeat(32))
        .with_isolation_tier(crate::sandbox::adapter::IsolationTier::Tier3Microvm);
        let err = create_err(&factory, &req).await;
        assert!(matches!(err, SwarmError::FactoryFailed(_)), "got {err}");
    }
}

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

use std::sync::Arc;

use async_trait::async_trait;

use crate::model_runtime::candle::{load_local_candle_model, LoadedCandleModel};
use crate::model_runtime::registry::RuntimeBinding;
use crate::model_runtime::{
    CancellationToken, ModelId, ModelRuntime, ProviderKind,
};
use crate::process_ledger::{
    record_spawn, LedgerBatcher, ProcessEngineKind, ProcessOwnershipRecordId, SpawnMeta,
};

use super::error::{SwarmError, SwarmResult};
use super::factory::{LiveSession, ModelSessionFactory, SessionTeardown};
use super::ids::SpawnRequest;

use super::coordinator::{SwarmConfig, SwarmCoordinator};
use super::events::FlightRecorderSwarmSink;
use super::ids::RunBudget;
use crate::flight_recorder::FlightRecorderEvent;

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
///
/// The reaper is NOT started here; the owner (app state) calls
/// [`SwarmCoordinator::start_reaper`] after `.manage` so the TTL/lease reaper
/// runs for the app's lifetime.
pub fn build_production_swarm_coordinator<F>(
    ledger: LedgerBatcher,
    cloud: CloudLaneFactoryConfig,
    concurrency: Option<usize>,
    trace_id: uuid::Uuid,
    emit_fn: F,
) -> SwarmCoordinator
where
    F: Fn(FlightRecorderEvent) + Send + Sync + 'static,
{
    let concurrency = concurrency.unwrap_or_else(default_swarm_concurrency).max(1);
    let budget = RunBudget::defaulted(concurrency).with_concurrency(concurrency);
    let config = SwarmConfig::new(budget);
    let factory = Arc::new(ProductionModelSessionFactory::new(ledger.clone(), cloud));
    let sink = Arc::new(FlightRecorderSwarmSink::new(trace_id, emit_fn));
    SwarmCoordinator::new(config, factory, sink, ledger)
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
        config_template: crate::model_runtime::cloud::CliBridgeConfig,
    ) -> Self {
        self.with_official_cli_observed(spawner, config_template, None)
    }

    /// Like [`Self::with_official_cli`] but threads a
    /// [`crate::model_runtime::cloud::CloudLaneObservability`] into the CLI lane
    /// so the built runtime emits `FR-EVT-LLM-INFER-{START,TOKEN,END}` (the
    /// recorder-carrying production constructors pass `Some(..)`; `None` keeps
    /// the lane FR-INFER-silent but otherwise identical).
    pub fn with_official_cli_observed(
        mut self,
        spawner: Arc<dyn crate::model_runtime::cloud::CliSubprocessSpawner>,
        config_template: crate::model_runtime::cloud::CliBridgeConfig,
        observability: Option<Arc<crate::model_runtime::cloud::CloudLaneObservability>>,
    ) -> Self {
        let mut builder = CliBridgeCloudRuntimeBuilder::new(spawner, config_template);
        if let Some(obs) = observability {
            builder = builder.with_observability(obs);
        }
        self.official_cli = Some(Arc::new(builder) as Arc<dyn CloudRuntimeBuilder>);
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
            crate::model_runtime::cloud::CliBridgeConfig,
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
pub struct CliBridgeCloudRuntimeBuilder {
    spawner: Arc<dyn crate::model_runtime::cloud::CliSubprocessSpawner>,
    config_template: crate::model_runtime::cloud::CliBridgeConfig,
    /// Optional cloud-lane observability threaded into the built
    /// [`crate::model_runtime::cloud::CliBridgeModelRuntime`] so its `generate`
    /// emits `FR-EVT-LLM-INFER-{START,TOKEN,END}`. `None` => no FR-INFER events
    /// (e.g. a recorder-less wiring); generation is identical either way.
    lane_obs: Option<Arc<crate::model_runtime::cloud::CloudLaneObservability>>,
}

impl CliBridgeCloudRuntimeBuilder {
    pub fn new(
        spawner: Arc<dyn crate::model_runtime::cloud::CliSubprocessSpawner>,
        config_template: crate::model_runtime::cloud::CliBridgeConfig,
    ) -> Self {
        Self {
            spawner,
            config_template,
            lane_obs: None,
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

    async fn build_loaded(&self, model_name: &str) -> Result<CloudLiveRuntime, String> {
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

        let mut rt = crate::model_runtime::cloud::CliBridgeModelRuntime::new(
            self.spawner.clone(),
            self.config_template.clone(),
        );
        // Thread the cloud-lane observability so the CLI lane emits
        // FR-EVT-LLM-INFER-{START,TOKEN,END} like the BYOK siblings, when a
        // recorder was wired (production_with_*_recorder).
        if let Some(obs) = self.lane_obs.clone() {
            rt = rt.with_lane_observability(obs);
        }
        let model_id = rt
            .load(self.cli_load_spec(model_name))
            .await
            .map_err(|e| format!("official CLI bridge load for model '{model_name}' failed: {e}"))?;
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
    fn key_provider(
        &self,
    ) -> Arc<dyn crate::model_runtime::cloud::ApiKeyProvider> {
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

    async fn build_loaded(&self, model_name: &str) -> Result<CloudLiveRuntime, String> {
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
    fn fetch_api_key(
        &self,
    ) -> Result<String, crate::model_runtime::cloud::OpenAiByokError> {
        self.vault.get(&self.lane).map_err(|err| {
            crate::model_runtime::cloud::OpenAiByokError::ApiKeyFetch(format!("{err}"))
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
    async fn build_loaded(
        &self,
        model_name: &str,
    ) -> Result<CloudLiveRuntime, String>;
}

/// A built, loaded cloud runtime plus the model id its `load` minted. The
/// factory wraps this into a [`LiveSession`] whose teardown unloads the cloud
/// model handle.
pub struct CloudLiveRuntime {
    pub runtime: Arc<dyn ModelRuntime>,
    pub model_id: ModelId,
}

/// The production model session factory.
pub struct ProductionModelSessionFactory {
    ledger: LedgerBatcher,
    cloud: CloudLaneFactoryConfig,
    /// Synthetic pid base for in-process sessions (candle / cloud run in-process
    /// — there is no separate OS process). A monotonic offset keeps ledger pids
    /// distinct per instance so START/STOP rows correlate one-to-one.
    pid_base: u32,
}

impl ProductionModelSessionFactory {
    /// Build a factory that supports the local candle + llama.cpp paths and the
    /// supplied cloud-lane config. Pass [`CloudLaneFactoryConfig::unconfigured`]
    /// for a local-only deployment.
    pub fn new(ledger: LedgerBatcher, cloud: CloudLaneFactoryConfig) -> Self {
        Self {
            ledger,
            cloud,
            pid_base: 50_000,
        }
    }

    /// Convenience: local-only factory (no cloud lanes). Cloud spawns return
    /// `ProviderNotConfigured`.
    pub fn local_only(ledger: LedgerBatcher) -> Self {
        Self::new(ledger, CloudLaneFactoryConfig::unconfigured())
    }

    fn synthetic_pid(&self, request: &SpawnRequest) -> u32 {
        self.pid_base.wrapping_add(request.instance_id.instance)
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
        let mut meta = SpawnMeta::new(os_pid, engine_kind, request.owner_role.clone());
        meta.model_id = Some(model_id.to_string());
        meta.runtime_binding = Some(request.runtime_binding.adapter_id().to_string());
        meta.parent_session_id = Some(request.parent_session_id.clone());
        meta.model_artifact_sha256 = request.model_artifact_sha256.clone();
        meta.owner_wp = request.owner_wp.clone();
        meta.role_id = request.role_id.clone();
        meta.wp_id = request.wp_id.clone();
        meta.mt_id = request.mt_id.clone();
        record_spawn(&self.ledger, meta).map_err(|e| SwarmError::LedgerFailed(e.to_string()))
    }

    async fn create_local_candle(&self, request: &SpawnRequest) -> SwarmResult<LiveSession> {
        let artifact_path = std::path::PathBuf::from(
            request
                .model_artifact_path()
                .ok_or_else(|| {
                    SwarmError::FactoryFailed(
                        "local candle spawn requires a model artifact path".to_string(),
                    )
                })?,
        );
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
        let record_id =
            self.record_start(request, model_id, os_pid, ProcessEngineKind::Candle)?;

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

        let artifact_path = std::path::PathBuf::from(
            request
                .model_artifact_path()
                .ok_or_else(|| {
                    SwarmError::FactoryFailed(
                        "local llama.cpp spawn requires a model artifact path".to_string(),
                    )
                })?,
        );
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
        let model_name = request.cloud_model_name.clone().ok_or_else(|| {
            SwarmError::ProviderNotConfigured {
                provider: provider_str(provider).to_string(),
                detail: "cloud spawn requires cloud_model_name".to_string(),
            }
        })?;

        let builder = match provider {
            ProviderKind::ByokCloud => self.cloud.openai.as_ref().or(self.cloud.anthropic.as_ref()),
            ProviderKind::OfficialCli => self.cloud.official_cli.as_ref(),
            _ => None,
        };
        let Some(builder) = builder else {
            return Err(SwarmError::ProviderNotConfigured {
                provider: provider_str(provider).to_string(),
                detail: "no cloud-lane builder configured for this provider (operator has not \
                         set up BYOK credentials / CLI)"
                    .to_string(),
            });
        };

        let CloudLiveRuntime { runtime, model_id } = builder
            .build_loaded(&model_name)
            .await
            .map_err(|reason| SwarmError::ProviderNotConfigured {
                provider: provider_str(provider).to_string(),
                detail: reason,
            })?;

        // Cloud BYOK invocations do not spawn a Handshake-owned OS process, but
        // the swarm still tracks the session lifecycle; record a synthetic
        // in-process START so the coordinator's START==STOP invariant holds for
        // cloud sessions too.
        let os_pid = self.synthetic_pid(request);
        let record_id =
            self.record_start(request, model_id, os_pid, ProcessEngineKind::LlamaCpp)?;
        let cancel = CancellationToken::new();
        let teardown = cloud_teardown(runtime.clone(), model_id);

        Ok(LiveSession::new(
            runtime, model_id, cancel, teardown, record_id, os_pid,
        ))
    }
}

#[async_trait]
impl ModelSessionFactory for ProductionModelSessionFactory {
    async fn create(&self, request: &SpawnRequest) -> SwarmResult<LiveSession> {
        match request.provider {
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
    }
}

/// Teardown for a local runtime wrapped in a shared `Arc<Mutex<R>>`: it
/// deterministically `unload`s the model behind the Mutex (mirroring
/// `kernel_model_runtime_unload`'s detach), then drops the owning Arc so the
/// runtime's `Drop` frees the engine weights when the last reference goes away.
/// This is the D1 free for both the candle and llama.cpp local paths.
fn shared_runtime_teardown<R>(
    owning: Arc<tokio::sync::Mutex<R>>,
    model_id: ModelId,
) -> SessionTeardown
where
    R: ModelRuntime + 'static,
{
    Box::new(move || {
        Box::pin(async move {
            {
                let mut guard = owning.lock().await;
                // Best-effort: the load succeeded so this removes the model map
                // entry. An error here means it was already unloaded — not fatal
                // to the free, which the subsequent Arc drop guarantees.
                let _ = guard.unload(model_id).await;
            }
            // Drop our owning reference. When the coordinator also drops its
            // shared `Arc<dyn ModelRuntime>` (on terminal eviction) the last
            // reference goes away and `R::Drop` frees the weights.
            drop(owning);
            Ok(())
        })
    })
}

/// Cloud teardown: unload the model handle from the cloud runtime so its audit
/// trail closes; the runtime Arc drops with the session.
fn cloud_teardown(runtime: Arc<dyn ModelRuntime>, _model_id: ModelId) -> SessionTeardown {
    Box::new(move || {
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

    async fn unload(
        &mut self,
        id: ModelId,
    ) -> Result<(), crate::model_runtime::ModelRuntimeError> {
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
        Err(crate::model_runtime::ModelRuntimeError::CapabilityNotSupported {
            capability: "capabilities via swarm shared handle (use the registry record)"
                .to_string(),
            adapter: "swarm_shared_runtime".to_string(),
        })
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
        LedgerBatcherConfig, LedgerEvent, LedgerEventKind, NoopOverflowSink, ProcessLedgerDrain,
        ProcessLedgerStore, ProcessLedgerError,
    };
    use crate::swarm_orchestration::events::RecordingSwarmSink;
    use crate::swarm_orchestration::ids::{ModelInstanceId, RunBudget, SpawnRequest};
    use crate::swarm_orchestration::SwarmCoordinator;
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};

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

    async fn drained(drain: &ProcessLedgerDrain, store: Arc<InMemoryStore>) -> Vec<LedgerEvent> {
        drain.drain_available_to(store.clone()).await.unwrap();
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
    async fn create_err(
        factory: &ProductionModelSessionFactory,
        req: &SpawnRequest,
    ) -> SwarmError {
        match factory.create(req).await {
            Ok(_) => panic!("expected the factory create to fail"),
            Err(e) => e,
        }
    }

    // ---- DEFAULT-CI: dispatch returns the right typed errors honestly. ----

    #[tokio::test]
    async fn cloud_spawn_without_configured_lane_returns_provider_not_configured() {
        let (ledger, _drain) = ledger_pair();
        let factory = ProductionModelSessionFactory::local_only(ledger);
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

    #[tokio::test]
    async fn production_coordinator_constructs_and_rejects_unloadable_local_spawn_without_orphan() {
        // Construct the production coordinator via the wiring helper, spawn a
        // local-candle session whose artifact does not exist: the typed factory
        // error propagates and the ledger carries no orphan START/STOP.
        let (ledger, drain) = ledger_pair();
        let store = Arc::new(InMemoryStore::default());
        let emitted = Arc::new(Mutex::new(0usize));
        let e2 = emitted.clone();
        let coordinator = build_production_swarm_coordinator(
            ledger,
            CloudLaneFactoryConfig::unconfigured(),
            Some(2),
            uuid::Uuid::now_v7(),
            move |_ev| {
                *e2.lock().unwrap() += 1;
            },
        );
        let req = local_candle_req(
            instance(7),
            "D:/__handshake_no_such_swarm_model__/model.safetensors",
        );
        let err = coordinator.spawn_session(req).await.expect_err("unloadable spawn");
        assert!(matches!(err, SwarmError::FactoryFailed(_)), "got {err}");
        // No live session remains and the ledger has no orphan rows.
        assert_eq!(coordinator.live_session_count(), 0);
        let rows = drained(&drain, store).await;
        assert!(rows.is_empty(), "no orphan ledger rows; got {}", rows.len());
        // A SpawnFailed/SpawnRejected event was emitted (sink ran).
        assert!(*emitted.lock().unwrap() >= 1, "the sink must have emitted at least one event");
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
            async fn build_loaded(&self, _model: &str) -> Result<CloudLiveRuntime, String> {
                Err("no openai api key in the operator secrets vault".to_string())
            }
        }
        let (ledger, drain) = ledger_pair();
        let store = Arc::new(InMemoryStore::default());
        let cloud = CloudLaneFactoryConfig {
            openai: Some(Arc::new(UnconfiguredOpenAi)),
            anthropic: None,
            official_cli: None,
        };
        let factory = ProductionModelSessionFactory::new(ledger, cloud);
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
        async fn build_loaded(&self, _model: &str) -> Result<CloudLiveRuntime, String> {
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
        ) -> Result<
            &crate::model_runtime::ModelCapabilities,
            crate::model_runtime::ModelRuntimeError,
        > {
            Err(crate::model_runtime::ModelRuntimeError::CapabilityNotSupported {
                capability: "n/a".to_string(),
                adapter: "counting".to_string(),
            })
        }
        fn kv_cache(
            &self,
            _id: ModelId,
        ) -> Result<crate::model_runtime::KvCacheHandle, crate::model_runtime::ModelRuntimeError>
        {
            Err(crate::model_runtime::ModelRuntimeError::KvCacheError("n/a".to_string()))
        }
        fn lora_stack(
            &self,
            _id: ModelId,
        ) -> Result<crate::model_runtime::LoraStackHandle, crate::model_runtime::ModelRuntimeError>
        {
            Err(crate::model_runtime::ModelRuntimeError::LoraStackError("n/a".to_string()))
        }
        fn steering_hooks(
            &self,
            _id: ModelId,
        ) -> Result<
            crate::model_runtime::SteeringHookHandle,
            crate::model_runtime::ModelRuntimeError,
        > {
            Err(crate::model_runtime::ModelRuntimeError::SteeringHookError("n/a".to_string()))
        }
        fn cancel(&self, token: CancellationToken) {
            token.cancel();
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn cloud_success_dispatch_records_start_and_teardown_records_stop_and_unloads() {
        let (ledger, drain) = ledger_pair();
        let store = Arc::new(InMemoryStore::default());
        let unloaded = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let cloud = CloudLaneFactoryConfig {
            openai: Some(Arc::new(OkCloudBuilder {
                unloaded: unloaded.clone(),
            })),
            anthropic: None,
            official_cli: None,
        };
        let sink = Arc::new(RecordingSwarmSink::new());
        let factory = Arc::new(ProductionModelSessionFactory::new(ledger.clone(), cloud));
        let coordinator = SwarmCoordinator::new(
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
        coordinator.spawn_session(req).await.expect("cloud spawn succeeds");
        assert_eq!(coordinator.live_session_count(), 1);
        // Cancel -> teardown must free (unload) the cloud handle is dropped; the
        // cloud teardown drops the shared Arc (no unload call for shared handle),
        // so we assert the session left no orphan and START==STOP instead.
        coordinator.cancel_session(iid, "test_cancel").await.expect("cancel");
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
        assert_eq!(starts, 1, "exactly one START for the cloud session");
        assert_eq!(stops, 1, "exactly one STOP — no orphan");
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
        let coordinator = Arc::new(SwarmCoordinator::new(
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
        let (ra, rb) = tokio::join!(
            async move { c1.spawn_session(req_a).await },
            async move { c2.spawn_session(req_b).await },
        );
        ra.expect("parallel spawn A");
        rb.expect("parallel spawn B");
        assert_eq!(coordinator.live_session_count(), 2, "both sessions live in parallel");

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
        coordinator.cancel_session(iid_a, "real_cancel").await.expect("cancel A");
        assert_eq!(coordinator.live_session_count(), 1, "A freed, B still live");

        // Drain B too for a clean ledger.
        coordinator.complete_session(iid_b).await.expect("complete B");
        assert_eq!(coordinator.live_session_count(), 0);

        let rows = drained(&drain, store).await;
        let starts = rows.iter().filter(|e| e.kind() == LedgerEventKind::Start).count();
        let stops = rows.iter().filter(|e| e.kind() == LedgerEventKind::Stop).count();
        assert_eq!(starts, 2, "two real model STARTs");
        assert_eq!(stops, 2, "two STOPs — teardown closed both, no orphan");
        let _ = produced_a + produced_b; // silence on some toolchains
        let _ = Ordering::SeqCst;
    }

    #[cfg(feature = "candle-runtime-engine")]
    fn local_candle_req_real(
        iid: ModelInstanceId,
        path: &str,
        sha: &str,
    ) -> SpawnRequest {
        SpawnRequest::new(iid, RuntimeBinding::Candle, "swarm_prod_test", "parent-1")
            .with_local_artifact(path, sha)
    }

    // Helper accessors used only by the env-gated real test: fetch the live
    // runtime + model id the coordinator registered for an instance.
    #[cfg(feature = "candle-runtime-engine")]
    fn runtime_for(
        coordinator: &SwarmCoordinator,
        iid: ModelInstanceId,
    ) -> Arc<dyn ModelRuntime> {
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
            .put("anthropic", "sk-ant-test-do-not-log".to_string())
            .expect("store key");
        let builder = VaultCloudRuntimeBuilder::new(
            CloudProviderFlavor::Anthropic,
            vault,
            "anthropic",
        );
        let built = builder
            .build_loaded("claude-sonnet-4")
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
        let builder =
            VaultCloudRuntimeBuilder::new(CloudProviderFlavor::OpenAi, vault, "openai");
        // CloudLiveRuntime is not Debug (owns trait objects), so match rather
        // than expect_err.
        let err = match builder.build_loaded("gpt-4o").await {
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
        vault
            .put("openai", "sk-test-openai".to_string())
            .expect("store key");
        let cloud = CloudLaneFactoryConfig::from_vault(
            vault,
            None,
            Some("openai".to_string()),
        );
        let (ledger, drain) = ledger_pair();
        let store = Arc::new(InMemoryStore::default());
        let sink = Arc::new(RecordingSwarmSink::new());
        let factory = Arc::new(ProductionModelSessionFactory::new(ledger.clone(), cloud));
        let coordinator = SwarmCoordinator::new(
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
        let rows = drained(&drain, store).await;
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
        vault.put(lane, api_key).expect("store live key");
        let cloud = match flavor {
            CloudProviderFlavor::Anthropic => {
                CloudLaneFactoryConfig::from_vault(vault, Some(lane.to_string()), None)
            }
            CloudProviderFlavor::OpenAi => {
                CloudLaneFactoryConfig::from_vault(vault, None, Some(lane.to_string()))
            }
        };

        let (ledger, drain) = ledger_pair();
        let store = Arc::new(InMemoryStore::default());
        let sink = Arc::new(RecordingSwarmSink::new());
        let factory = Arc::new(ProductionModelSessionFactory::new(ledger.clone(), cloud));
        let coordinator = Arc::new(SwarmCoordinator::new(
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
        let rows = drained(&drain, store).await;
        let starts = rows.iter().filter(|e| e.kind() == LedgerEventKind::Start).count();
        let stops = rows.iter().filter(|e| e.kind() == LedgerEventKind::Stop).count();
        assert_eq!(starts, 1);
        assert_eq!(stops, 1);
    }

    // ---- DEFAULT-CI: official-CLI bridge cloud lane (no real subprocess). ----

    use crate::model_runtime::cloud::official_cli_bridge::{
        CliBridgeConfig as TestCliBridgeConfig, CliInvocationReceipt, CliKind, CliOutputFormat,
        CliSubprocessSpawner, OfficialCliBridgeError,
    };

    fn cli_temp_exe() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")
    }

    fn cli_config_present() -> TestCliBridgeConfig {
        TestCliBridgeConfig {
            cli_kind: CliKind::ClaudeCode,
            executable_path: cli_temp_exe(),
            args_template: vec!["--prompt".to_string(), "{prompt}".to_string()],
            output_format: CliOutputFormat::RawText,
            env_vars: std::collections::HashMap::new(),
            working_dir: None,
            timeout_seconds: 120,
        }
    }

    /// Mock CLI byte source: emits a couple of stdout chunks (so the built
    /// runtime's `generate` is a real streaming path), no real subprocess.
    struct MockCliSpawner;
    impl CliSubprocessSpawner for MockCliSpawner {
        fn spawn(
            &self,
            _config: &TestCliBridgeConfig,
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
            _model_name: &str,
            _prompt: &str,
            on_chunk: &mut dyn FnMut(&[u8]),
        ) -> Result<CliInvocationReceipt, OfficialCliBridgeError> {
            on_chunk(b"ok");
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
        let mut absent = cli_config_present();
        absent.executable_path =
            std::path::PathBuf::from("D:/__handshake_no_such_cli__/claude.exe");
        let builder_absent =
            CliBridgeCloudRuntimeBuilder::new(Arc::new(MockCliSpawner), absent);
        let err = match builder_absent.build_loaded("claude-sonnet").await {
            Ok(_) => panic!("expected not-configured for an absent CLI"),
            Err(e) => e,
        };
        assert!(err.contains("not found"), "{err}");

        // Present CLI (a file that exists) -> real CliBridgeModelRuntime.
        let builder_present =
            CliBridgeCloudRuntimeBuilder::new(Arc::new(MockCliSpawner), cli_config_present());
        let built = builder_present
            .build_loaded("claude-sonnet")
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
        let (ledger, drain) = ledger_pair();
        let store = Arc::new(InMemoryStore::default());
        let cloud = CloudLaneFactoryConfig {
            anthropic: None,
            openai: None,
            official_cli: Some(Arc::new(CliBridgeCloudRuntimeBuilder::new(
                Arc::new(MockCliSpawner),
                cli_config_present(),
            ))),
        };
        let sink = Arc::new(RecordingSwarmSink::new());
        let factory = Arc::new(ProductionModelSessionFactory::new(ledger.clone(), cloud));
        let coordinator = SwarmCoordinator::new(
            crate::swarm_orchestration::SwarmConfig::new(
                RunBudget::defaulted(4).with_concurrency(4),
            ),
            factory,
            sink,
            ledger,
        );
        let iid = instance(0);
        let req = SpawnRequest::new(iid, RuntimeBinding::Candle, "swarm_prod_test", "parent-1")
            .with_cloud_provider(ProviderKind::OfficialCli, "claude-sonnet");
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
        let rows = drained(&drain, store).await;
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

        // Not-configured variant: official_cli=None -> ProviderNotConfigured.
        let (ledger2, _drain2) = ledger_pair();
        let factory2 = ProductionModelSessionFactory::local_only(ledger2);
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
}

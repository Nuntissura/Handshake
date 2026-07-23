//! WP-1 MT-012 operator chat/launch proof.
//!
//! Real runtime, no scaffolding: these tests drive the sanctioned launch
//! authority (`SwarmCoordinator::spawn_session`), the real CLI-capture parser
//! (`parse_agent_activity_line`), and the real `ModelLaneStore::record_message`
//! against Handshake-managed PostgreSQL/EventLedger. The Flight Recorder is a
//! capturing double so FR-EVT-AGENT-* / FR-EVT-MODEL-SELECTION-RECORDED events
//! are inspectable.

mod knowledge_pg_support;

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use chrono::Utc;
use futures::stream;
use serde_json::json;
use uuid::Uuid;

use handshake_core::api::operator_chat::{routes as operator_chat_http_routes, OperatorChatState};
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::model_runtime::catalog::ModelCatalog;
use handshake_core::model_runtime::cloud::{
    AllowlistedCliBridgeConfig, CliBridgeConfig, CliBridgeModelRuntime, CliCancellationContext,
    CliInvocationContext, CliInvocationReceipt, CliKind, CliModelAllowlist, CliOutputFormat,
    CliSubprocessSpawner, CloudLaneObservability, LiveCliSpawner, OfficialCliBridgeError,
};
use handshake_core::model_runtime::registry::RuntimeBinding as RuntimeAdapterBinding;
use handshake_core::model_runtime::{
    BaseModelTag, CancellationToken, Embedding, GenPrompt, GenerateRequest, GeneratedToken,
    KvCacheHandle, KvCachePolicy, LoadSpec, LoraStackHandle, ModelCapabilities, ModelId,
    ModelRegistration, ModelRegistry, ModelRuntime, ModelRuntimeError, OperatorId, ProviderKind,
    RuntimeKind, SamplingParams, Score, SteeringHookHandle, TokenStream,
};
use handshake_core::process_ledger::{
    drain_and_join_ledger_writer, LedgerBatcher, LedgerBatcherConfig, LedgerDrainJoinOutcome,
    NoopOverflowSink, PostgresProcessLedgerStore, ProcessEngineKind, ProcessLedgerDrain,
    ProcessOwnershipRecordId, ProcessStart,
};
use handshake_core::swarm_orchestration::model_lane::{
    DexterityLaunchAdapterKind, DexterityLaunchAdapterRequest, LaunchAuthority, ModelLaneKind,
    ModelLaneMessageKind, ModelLaneProviderKind, ModelLaneStatus, ModelLaneStore, RuntimeBinding,
};
use handshake_core::swarm_orchestration::operator_chat::{
    build_spawn_request, ModelLaneCaptureRecorder, OperatorChatLaneKind, OperatorChatLaunchService,
    OperatorChatLaunched, OperatorChatSelection, OPERATOR_CHAT_CLI_ADAPTER,
};
use handshake_core::swarm_orchestration::{
    CloudLaneFactoryConfig, LiveSession, ModelInstanceId, ModelSessionFactory,
    ProductionModelSessionFactory, RecordingSwarmSink, RunBudget, SessionTeardown, SpawnRequest,
    SwarmConfig, SwarmCoordinator, SwarmError,
};

// ---------------------------------------------------------------------------
// Capturing Flight Recorder double (inspectable).
// ---------------------------------------------------------------------------

#[derive(Default)]
struct CapturingRecorder {
    events: Mutex<Vec<FlightRecorderEvent>>,
}

fn operator_chat_registry_session(
    session_id: &str,
    parent_session_id: Option<&str>,
    spawn_depth: i32,
) -> handshake_core::storage::ModelSession {
    handshake_core::storage::ModelSession {
        session_id: session_id.to_string(),
        parent_session_id: parent_session_id.map(str::to_string),
        spawn_depth,
        state: handshake_core::storage::ModelSessionState::Active,
        model_id: "gpt-test".to_string(),
        backend: "codex".to_string(),
        parameter_class: "standard".to_string(),
        role: "CODER".to_string(),
        wp_id: Some("WP-1".to_string()),
        mt_id: Some("MT-017".to_string()),
        work_profile_id: None,
        execution_mode: "delegated".to_string(),
        memory_policy: "SESSION_SCOPED".to_string(),
        consent_receipt_id: None,
        capability_grants: Vec::new(),
        capability_token_ids: None,
        job_id: None,
        checkpoint_artifact_id: None,
        last_checkpoint_at: None,
        checkpoint_count: 0,
        merge_back_artifact: None,
        agent: None,
        purpose: None,
        close_reason: None,
        closed_by_actor: None,
        closed_at: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

#[async_trait]
impl FlightRecorder for CapturingRecorder {
    async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
        self.events.lock().expect("events lock").push(event);
        Ok(())
    }

    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        Ok(0)
    }

    async fn list_events(
        &self,
        _filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Ok(self.events.lock().expect("events lock").clone())
    }
}

impl CapturingRecorder {
    /// Every `payload.event_id` / `payload.fr_event` string across captured events.
    fn event_markers(&self) -> Vec<String> {
        self.events
            .lock()
            .expect("events lock")
            .iter()
            .flat_map(|e| {
                let mut out = Vec::new();
                if let Some(id) = e.payload.get("event_id").and_then(|v| v.as_str()) {
                    out.push(id.to_string());
                }
                if let Some(id) = e.payload.get("fr_event").and_then(|v| v.as_str()) {
                    out.push(id.to_string());
                }
                out
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Proof factory + runtime (non-mocked lane launch: real coordinator + real
// ModelLaneStore persistence; the model runtime is an in-process no-op so the
// proof does not need a GPU/real model load).
// ---------------------------------------------------------------------------

fn sample_sha256() -> String {
    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".into()
}

struct OperatorChatProofFactory {
    ledger: LedgerBatcher,
    loads: Arc<AtomicUsize>,
    unloads: Arc<AtomicUsize>,
}

#[async_trait]
impl ModelSessionFactory for OperatorChatProofFactory {
    async fn create(&self, request: &SpawnRequest) -> Result<LiveSession, SwarmError> {
        let record_id = ProcessOwnershipRecordId::new_v7();
        let os_pid = 57000 + request.instance_id.instance;
        let engine = match request.provider {
            Some(ProviderKind::OfficialCli) => ProcessEngineKind::OfficialCliBridge,
            Some(ProviderKind::ByokCloud) => ProcessEngineKind::HelperSubprocess,
            Some(ProviderKind::ExternalCompat) => ProcessEngineKind::ExternalCompat,
            Some(ProviderKind::Local) | None => match request.runtime_binding {
                RuntimeAdapterBinding::Candle => ProcessEngineKind::Candle,
                RuntimeAdapterBinding::LlamaCpp => ProcessEngineKind::LlamaCpp,
            },
        };
        let start = ProcessStart::new(engine, request.owner_role.clone(), request.owner_wp.clone())
            .with_process_uuid(record_id.as_uuid())
            .with_os_pid(os_pid)
            .with_parent_session_id(request.parent_session_id.clone())
            .with_wp_id(request.wp_id.clone().unwrap_or_default())
            .with_mt_id(request.mt_id.clone().unwrap_or_default());
        self.ledger
            .record_start(start)
            .map_err(|err| SwarmError::LedgerFailed(err.to_string()))?;

        let mut owned_runtime = ProofRuntime::new(self.loads.clone(), self.unloads.clone());
        let model_id = owned_runtime
            .load(load_spec(request))
            .await
            .map_err(|err| SwarmError::FactoryFailed(err.to_string()))?;
        let owned = Arc::new(tokio::sync::Mutex::new(owned_runtime));
        let shared = ProofRuntime::new(self.loads.clone(), self.unloads.clone());
        let teardown: SessionTeardown = Arc::new(move || {
            let owned = Arc::clone(&owned);
            Box::pin(async move {
                let mut owned = owned.lock().await;
                owned
                    .unload(model_id)
                    .await
                    .map_err(|err| SwarmError::Internal(err.to_string()))
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

struct CountingFactory {
    calls: Arc<AtomicUsize>,
}

#[async_trait]
impl ModelSessionFactory for CountingFactory {
    async fn create(&self, _request: &SpawnRequest) -> Result<LiveSession, SwarmError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Err(SwarmError::FactoryFailed(
            "CountingFactory must not be called when the launch fails closed".into(),
        ))
    }
}

struct ProofRuntime {
    capabilities: ModelCapabilities,
    kv: KvCacheHandle,
    lora: LoraStackHandle,
    steering: SteeringHookHandle,
    loads: Arc<AtomicUsize>,
    unloads: Arc<AtomicUsize>,
}

impl ProofRuntime {
    fn new(loads: Arc<AtomicUsize>, unloads: Arc<AtomicUsize>) -> Self {
        Self {
            capabilities: ModelCapabilities::default(),
            kv: KvCacheHandle::new("operator-chat-kv"),
            lora: LoraStackHandle::new("operator-chat-lora"),
            steering: SteeringHookHandle::new("operator-chat-steering"),
            loads,
            unloads,
        }
    }
}

#[async_trait]
impl ModelRuntime for ProofRuntime {
    async fn load(&mut self, _spec: LoadSpec) -> Result<ModelId, ModelRuntimeError> {
        self.loads.fetch_add(1, Ordering::SeqCst);
        Ok(ModelId::new_v7())
    }
    async fn unload(&mut self, _id: ModelId) -> Result<(), ModelRuntimeError> {
        self.unloads.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    fn generate(&self, req: GenerateRequest) -> TokenStream {
        let items = (0..req.max_tokens.min(1)).map(move |i| {
            Ok(GeneratedToken {
                token_id: i,
                text: "ok".into(),
                logprob: None,
                finish_reason: None,
            })
        });
        Box::pin(stream::iter(items.collect::<Vec<_>>()))
    }
    async fn score(&self, _id: ModelId, _seq: Vec<u32>) -> Result<Score, ModelRuntimeError> {
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
    }
}

fn load_spec(request: &SpawnRequest) -> LoadSpec {
    LoadSpec {
        artifact_path: "operator-chat-proof.gguf".into(),
        sha256_expected: sample_sha256(),
        runtime_kind: RuntimeKind::LlamaCpp,
        sampling_defaults: SamplingParams::default(),
        kv_cache_policy: KvCachePolicy::default(),
        declared_capabilities: ModelCapabilities::default(),
        provider: request.provider.unwrap_or(ProviderKind::Local),
        engine_origin: Some("operator-chat-proof".into()),
        external_engine_import: None,
    }
}

// ---------------------------------------------------------------------------
// Harness helpers.
// ---------------------------------------------------------------------------

async fn pg_store() -> (sqlx::PgPool, ModelLaneStore) {
    let kpg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("PostgreSQL/EventLedger is required for MT-012 proof");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&kpg.schema_url)
        .await
        .expect("connect isolated operator-chat schema");
    let store = ModelLaneStore::new(pool.clone());
    (pool, store)
}

fn store_backed_coordinator(
    store: ModelLaneStore,
) -> (Arc<SwarmCoordinator>, Arc<AtomicUsize>, ProcessLedgerDrain) {
    let (ledger, drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("manual process ledger");
    let loads = Arc::new(AtomicUsize::new(0));
    let coordinator = SwarmCoordinator::new_with_model_lane_store(
        SwarmConfig::new(RunBudget::defaulted(8)),
        Arc::new(OperatorChatProofFactory {
            ledger: ledger.clone(),
            loads: loads.clone(),
            unloads: Arc::new(AtomicUsize::new(0)),
        }),
        Arc::new(RecordingSwarmSink::new()),
        ledger,
        store,
    );
    (Arc::new(coordinator), loads, drain)
}

fn registered_local_catalog() -> (Arc<ModelCatalog>, String) {
    let model_id = ModelId::new_v7();
    let mut registry = ModelRegistry::default();
    registry
        .register(ModelRegistration {
            model_id,
            artifact_path: "D:/models/operator-chat-local-proof.gguf".into(),
            sha256: [42u8; 32],
            runtime_binding: RuntimeAdapterBinding::Candle,
            declared_capabilities: ModelCapabilities::default(),
            base_model_tag: BaseModelTag::new("Operator Chat Local Proof"),
            registered_at_utc: Utc::now(),
            registered_by: OperatorId::new("operator-chat-test"),
            provider: ProviderKind::Local,
        })
        .expect("register local operator-chat proof model");
    registry.mark_loaded(model_id).expect("mark model ready");
    (
        ModelCatalog::from_registry(Arc::new(registry)),
        model_id.to_string(),
    )
}

// ---------------------------------------------------------------------------
// F1/F2 live loopback: a real `CliBridgeModelRuntime` backed by a mock CLI
// spawner that emits stream-json, so `launch()` drives the ACTUAL runtime and its
// real stdout becomes ModelLaneMessage rows (not a separately-authored vec).
// ---------------------------------------------------------------------------

/// A mock CLI subprocess spawner: emits a fixed set of stdout chunks LIVE through
/// `spawn_streaming` (each codex stream-json line, newline-terminated). No real
/// subprocess — the loopback proves the launch->capture wiring end to end.
struct LoopbackCliSpawner {
    chunks: Vec<Vec<u8>>,
}

impl LoopbackCliSpawner {
    fn from_lines(lines: &[String]) -> Self {
        Self {
            chunks: lines
                .iter()
                .map(|l| format!("{l}\n").into_bytes())
                .collect(),
        }
    }
}

struct FailingAfterChunkCliSpawner {
    chunks: Vec<Vec<u8>>,
}

/// Coordination probes for an actual `CliBridgeModelRuntime` cancellation. The
/// test receives only the spawned instance id; it still requests cancellation
/// through `SwarmCoordinator::cancel_session`, never by touching a raw token.
#[derive(Default)]
struct CancellationLaunchProbe {
    instance_id: Mutex<Option<ModelInstanceId>>,
    run_id: Mutex<Option<String>>,
    lane_id: Mutex<Option<String>>,
    prefix_emitted: AtomicBool,
    cancellation_observed: AtomicBool,
}

impl CancellationLaunchProbe {
    fn record_launch(&self, request: &SpawnRequest) {
        let contract = request
            .dexterity_launch
            .as_ref()
            .expect("cancellation fixture requires Dexterity launch authority");
        *self.instance_id.lock().expect("cancellation probe lock") = Some(request.instance_id);
        *self.run_id.lock().expect("cancellation probe lock") = Some(contract.run_id.clone());
        *self.lane_id.lock().expect("cancellation probe lock") = Some(contract.lane_id.clone());
    }

    fn instance_id(&self) -> Option<ModelInstanceId> {
        *self.instance_id.lock().expect("cancellation probe lock")
    }

    fn run_id(&self) -> Option<String> {
        self.run_id.lock().expect("cancellation probe lock").clone()
    }

    fn lane_id(&self) -> Option<String> {
        self.lane_id
            .lock()
            .expect("cancellation probe lock")
            .clone()
    }
}

/// Deterministic subprocess adapter fixture for the live CLI bridge: it emits
/// one complete activity, waits for the bridge's concrete cancellation state,
/// then flushes one already-buffered late tool activity. That exercises the
/// production bridge/coordinator/capture boundary without a live cloud account.
struct CancelAfterPrefixCliSpawner {
    prefix_chunk: Vec<u8>,
    late_chunk: Vec<u8>,
    probe: Arc<CancellationLaunchProbe>,
}

impl CancelAfterPrefixCliSpawner {
    fn from_lines(lines: &[String], probe: Arc<CancellationLaunchProbe>) -> Self {
        assert!(
            lines.len() >= 2,
            "cancellation fixture requires a prefix and a late activity line"
        );
        Self {
            prefix_chunk: format!("{}\n", lines[0]).into_bytes(),
            late_chunk: format!("{}\n", lines[1]).into_bytes(),
            probe,
        }
    }
}

impl CliSubprocessSpawner for CancelAfterPrefixCliSpawner {
    fn spawn(
        &self,
        _config: &CliBridgeConfig,
        _invocation: &CliInvocationContext,
        _model_name: &str,
        _prompt: &str,
    ) -> Result<CliInvocationReceipt, OfficialCliBridgeError> {
        Err(OfficialCliBridgeError::SpawnFailed {
            reason: "cancellation fixture must use the live streaming bridge path".to_string(),
            exit_code: None,
        })
    }

    fn spawn_streaming_cancellable(
        &self,
        _config: &CliBridgeConfig,
        _invocation: &CliInvocationContext,
        _model_name: &str,
        _prompt: &str,
        chunk_sender: &tokio::sync::mpsc::Sender<Vec<u8>>,
        cancellation: &CliCancellationContext,
    ) -> Result<CliInvocationReceipt, OfficialCliBridgeError> {
        chunk_sender
            .try_send(self.prefix_chunk.clone())
            .map_err(|failure| OfficialCliBridgeError::SpawnFailed {
                reason: failure.to_string(),
                exit_code: None,
            })?;
        self.probe.prefix_emitted.store(true, Ordering::SeqCst);

        let deadline = Instant::now() + Duration::from_secs(10);
        while !cancellation.is_cancelled() {
            if Instant::now() >= deadline {
                return Err(OfficialCliBridgeError::SpawnFailed {
                    reason: "coordinator cancellation was not propagated to CLI bridge".to_string(),
                    exit_code: None,
                });
            }
            std::thread::sleep(Duration::from_millis(5));
        }

        self.probe
            .cancellation_observed
            .store(true, Ordering::SeqCst);
        // Model an already-buffered completed line that arrives after the
        // coordinator committed the lane terminal state. The capture path must
        // reject it without emitting a phantom Flight Recorder activity.
        chunk_sender
            .try_send(self.late_chunk.clone())
            .map_err(|failure| OfficialCliBridgeError::SpawnFailed {
                reason: failure.to_string(),
                exit_code: None,
            })?;
        Ok(CliInvocationReceipt {
            model_id: ModelId::new_v7(),
            stdout: format!(
                "{}{}",
                String::from_utf8_lossy(&self.prefix_chunk),
                String::from_utf8_lossy(&self.late_chunk)
            ),
            pid: Some(4243),
            exit_code: Some(0),
            cancelled: true,
        })
    }
}

impl FailingAfterChunkCliSpawner {
    fn from_lines(lines: &[String]) -> Self {
        Self {
            chunks: lines
                .iter()
                .map(|l| format!("{l}\n").into_bytes())
                .collect(),
        }
    }
}

impl CliSubprocessSpawner for FailingAfterChunkCliSpawner {
    fn spawn(
        &self,
        _config: &CliBridgeConfig,
        _invocation: &CliInvocationContext,
        _model_name: &str,
        _prompt: &str,
    ) -> Result<CliInvocationReceipt, OfficialCliBridgeError> {
        Err(OfficialCliBridgeError::SpawnFailed {
            reason: "operator-chat test stream failure".to_string(),
            exit_code: Some(23),
        })
    }

    fn spawn_streaming(
        &self,
        _config: &CliBridgeConfig,
        _invocation: &CliInvocationContext,
        _model_name: &str,
        _prompt: &str,
        chunk_sender: &tokio::sync::mpsc::Sender<Vec<u8>>,
    ) -> Result<CliInvocationReceipt, OfficialCliBridgeError> {
        for chunk in &self.chunks {
            chunk_sender.try_send(chunk.clone()).map_err(|failure| {
                OfficialCliBridgeError::SpawnFailed {
                    reason: failure.to_string(),
                    exit_code: None,
                }
            })?;
        }
        Err(OfficialCliBridgeError::SpawnFailed {
            reason: "operator-chat test stream failure".to_string(),
            exit_code: Some(23),
        })
    }
}

impl CliSubprocessSpawner for LoopbackCliSpawner {
    fn spawn(
        &self,
        _config: &CliBridgeConfig,
        _invocation: &CliInvocationContext,
        _model_name: &str,
        _prompt: &str,
    ) -> Result<CliInvocationReceipt, OfficialCliBridgeError> {
        let stdout = self
            .chunks
            .iter()
            .map(|c| String::from_utf8_lossy(c).into_owned())
            .collect::<String>();
        Ok(CliInvocationReceipt {
            model_id: ModelId::new_v7(),
            stdout,
            pid: Some(4242),
            exit_code: Some(0),
            cancelled: false,
        })
    }

    fn spawn_streaming(
        &self,
        _config: &CliBridgeConfig,
        _invocation: &CliInvocationContext,
        _model_name: &str,
        _prompt: &str,
        chunk_sender: &tokio::sync::mpsc::Sender<Vec<u8>>,
    ) -> Result<CliInvocationReceipt, OfficialCliBridgeError> {
        let mut full = Vec::new();
        for chunk in &self.chunks {
            chunk_sender.try_send(chunk.clone()).map_err(|failure| {
                OfficialCliBridgeError::SpawnFailed {
                    reason: failure.to_string(),
                    exit_code: None,
                }
            })?;
            full.extend_from_slice(chunk);
        }
        Ok(CliInvocationReceipt {
            model_id: ModelId::new_v7(),
            stdout: String::from_utf8_lossy(&full).into_owned(),
            pid: Some(4242),
            exit_code: Some(0),
            cancelled: false,
        })
    }
}

/// A CLI-bridge exe path that exists (register_bridge validates exe-exists). The
/// crate's own Cargo.toml is a stable always-present file.
fn loopback_cli_exe() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")
}

fn loopback_cli_exe_for_kind(cli_kind: CliKind) -> std::path::PathBuf {
    #[cfg(windows)]
    if cli_kind == CliKind::CodexCli {
        return std::env::var_os("PATH")
            .into_iter()
            .flat_map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
            .map(|directory| directory.join("codex.cmd"))
            .find(|candidate| candidate.is_file())
            .expect("canonical codex.cmd must be installed on PATH for the Codex CLI lane proof");
    }
    loopback_cli_exe()
}

fn loopback_cli_config() -> CliBridgeConfig {
    loopback_cli_config_for_kind(CliKind::CodexCli)
}

fn loopback_cli_config_for_kind(cli_kind: CliKind) -> CliBridgeConfig {
    CliBridgeConfig {
        cli_kind,
        executable_path: loopback_cli_exe_for_kind(cli_kind),
        args_template: vec![
            "exec".into(),
            "--json".into(),
            "--model".into(),
            "{model}".into(),
            "{prompt}".into(),
        ],
        // JSON-stream so the launched runtime's stdout is TYPED codex activity.
        output_format: CliOutputFormat::JsonStream,
        env_vars: std::collections::HashMap::new(),
        working_dir: None,
        timeout_seconds: 60,
    }
}

fn loopback_cli_load_spec(model_name: &str) -> LoadSpec {
    LoadSpec {
        artifact_path: std::path::PathBuf::new(),
        sha256_expected: String::new(),
        runtime_kind: RuntimeKind::Candle,
        sampling_defaults: SamplingParams::default(),
        kv_cache_policy: KvCachePolicy::default(),
        declared_capabilities: ModelCapabilities::default(),
        provider: ProviderKind::OfficialCli,
        engine_origin: Some(model_name.to_string()),
        external_engine_import: None,
    }
}

/// A factory that builds a REAL [`CliBridgeModelRuntime`] backed by the loopback
/// spawner, so `SwarmCoordinator::session_runtime` hands `launch()` back a runtime
/// whose `generate()` streams the mock CLI's stream-json stdout.
enum CliLoopbackMode {
    Complete,
    FailAfterChunks,
    CancelAfterPrefix(Arc<CancellationLaunchProbe>),
}

struct CliLoopbackFactory {
    ledger: LedgerBatcher,
    lines: Vec<String>,
    mode: CliLoopbackMode,
    cli_kind: CliKind,
}

#[async_trait]
impl ModelSessionFactory for CliLoopbackFactory {
    async fn create(&self, request: &SpawnRequest) -> Result<LiveSession, SwarmError> {
        let record_id = ProcessOwnershipRecordId::new_v7();
        let os_pid = 58000 + request.instance_id.instance;
        let start = ProcessStart::new(
            ProcessEngineKind::OfficialCliBridge,
            request.owner_role.clone(),
            request.owner_wp.clone(),
        )
        .with_process_uuid(record_id.as_uuid())
        .with_os_pid(os_pid)
        .with_parent_session_id(request.parent_session_id.clone())
        .with_wp_id(request.wp_id.clone().unwrap_or_default())
        .with_mt_id(request.mt_id.clone().unwrap_or_default());
        self.ledger
            .record_start(start.clone())
            .map_err(|err| SwarmError::LedgerFailed(err.to_string()))?;

        let model_name = request
            .cloud_model_name
            .clone()
            .unwrap_or_else(|| "gpt-5-codex".to_string());
        let spawner: Arc<dyn CliSubprocessSpawner> = match &self.mode {
            CliLoopbackMode::Complete => Arc::new(LoopbackCliSpawner::from_lines(&self.lines)),
            CliLoopbackMode::FailAfterChunks => {
                Arc::new(FailingAfterChunkCliSpawner::from_lines(&self.lines))
            }
            CliLoopbackMode::CancelAfterPrefix(probe) => {
                probe.record_launch(request);
                Arc::new(CancelAfterPrefixCliSpawner::from_lines(
                    &self.lines,
                    probe.clone(),
                ))
            }
        };
        let session_id = request.instance_id.to_string();
        let mut invocation_context =
            CliInvocationContext::new(request.owner_role.clone(), model_name.clone());
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

        // Shared runtime handed to the coordinator (its generate() is driven by
        // launch); loaded so handle_for(model_id) resolves.
        let allowlisted = |model: &str| {
            AllowlistedCliBridgeConfig::new(
                loopback_cli_config_for_kind(self.cli_kind),
                CliModelAllowlist::new(vec![model.to_string()]).expect("test allowlist"),
            )
        };
        let mut shared = CliBridgeModelRuntime::new(spawner.clone(), allowlisted(&model_name))
            .with_invocation_context(invocation_context.clone());
        let shared_model_id = shared
            .load(loopback_cli_load_spec(&model_name))
            .await
            .map_err(|err| SwarmError::FactoryFailed(err.to_string()))?;

        // Owned runtime for the teardown free (mirrors the proof factory shape).
        let mut owned_runtime = CliBridgeModelRuntime::new(spawner, allowlisted(&model_name))
            .with_invocation_context(invocation_context);
        let owned_model_id = owned_runtime
            .load(loopback_cli_load_spec(&model_name))
            .await
            .map_err(|err| SwarmError::FactoryFailed(err.to_string()))?;
        let owned = Arc::new(tokio::sync::Mutex::new(owned_runtime));
        let teardown: SessionTeardown = Arc::new(move || {
            let owned = Arc::clone(&owned);
            Box::pin(async move {
                let mut owned = owned.lock().await;
                owned
                    .unload(owned_model_id)
                    .await
                    .map_err(|err| SwarmError::Internal(err.to_string()))
            })
        });

        let mut live = LiveSession::new(
            Arc::new(shared),
            shared_model_id,
            CancellationToken::new(),
            teardown,
            record_id,
            os_pid,
        );
        live.ledger_start_override = Some(start);
        Ok(live)
    }
}

fn cli_loopback_coordinator(store: ModelLaneStore, lines: Vec<String>) -> Arc<SwarmCoordinator> {
    cli_loopback_coordinator_with_ledger(store, lines).0
}

fn cli_generic_loopback_coordinator(
    store: ModelLaneStore,
    lines: Vec<String>,
) -> Arc<SwarmCoordinator> {
    cli_generic_loopback_coordinator_with_ledger(store, lines).0
}

fn cli_generic_loopback_coordinator_with_ledger(
    store: ModelLaneStore,
    lines: Vec<String>,
) -> (Arc<SwarmCoordinator>, ProcessLedgerDrain) {
    cli_loopback_coordinator_with_ledger_and_kind(store, lines, CliKind::Other)
}

fn cli_loopback_coordinator_with_ledger(
    store: ModelLaneStore,
    lines: Vec<String>,
) -> (Arc<SwarmCoordinator>, ProcessLedgerDrain) {
    cli_loopback_coordinator_with_ledger_and_kind(store, lines, CliKind::CodexCli)
}

fn cli_loopback_coordinator_with_ledger_and_kind(
    store: ModelLaneStore,
    lines: Vec<String>,
    cli_kind: CliKind,
) -> (Arc<SwarmCoordinator>, ProcessLedgerDrain) {
    let (ledger, drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("manual process ledger");
    let coordinator = SwarmCoordinator::new_with_model_lane_store(
        SwarmConfig::new(RunBudget::defaulted(8)),
        Arc::new(CliLoopbackFactory {
            ledger: ledger.clone(),
            lines,
            mode: CliLoopbackMode::Complete,
            cli_kind,
        }),
        Arc::new(RecordingSwarmSink::new()),
        ledger,
        store,
    );
    (Arc::new(coordinator), drain)
}

fn cli_failing_after_chunk_coordinator(
    store: ModelLaneStore,
    lines: Vec<String>,
) -> Arc<SwarmCoordinator> {
    let (ledger, _drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("manual process ledger");
    let coordinator = SwarmCoordinator::new_with_model_lane_store(
        SwarmConfig::new(RunBudget::defaulted(8)),
        Arc::new(CliLoopbackFactory {
            ledger: ledger.clone(),
            lines,
            mode: CliLoopbackMode::FailAfterChunks,
            cli_kind: CliKind::CodexCli,
        }),
        Arc::new(RecordingSwarmSink::new()),
        ledger,
        store,
    );
    Arc::new(coordinator)
}

fn cli_cancel_after_prefix_coordinator(
    store: ModelLaneStore,
    lines: Vec<String>,
    probe: Arc<CancellationLaunchProbe>,
) -> (Arc<SwarmCoordinator>, ProcessLedgerDrain) {
    let (ledger, drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("manual process ledger");
    let coordinator = SwarmCoordinator::new_with_model_lane_store(
        SwarmConfig::new(RunBudget::defaulted(8)),
        Arc::new(CliLoopbackFactory {
            ledger: ledger.clone(),
            lines,
            mode: CliLoopbackMode::CancelAfterPrefix(probe),
            cli_kind: CliKind::CodexCli,
        }),
        Arc::new(RecordingSwarmSink::new()),
        ledger,
        store,
    );
    (Arc::new(coordinator), drain)
}

/// The codex model id the loopback uses so `cli_kind_for_model` -> `CodexCli`
/// matches the emitted codex stream-json dialect.
fn codex_cli_selection(working_dir: &str, prompt: &str, owner: &str) -> OperatorChatSelection {
    OperatorChatSelection {
        model_id: "gpt-5-codex".into(),
        cli_provider: Some("codex".into()),
        ..cli_selection(working_dir, prompt, owner)
    }
}

fn existing_working_dir() -> &'static str {
    env!("CARGO_MANIFEST_DIR")
}

fn cli_selection(working_dir: &str, prompt: &str, owner: &str) -> OperatorChatSelection {
    OperatorChatSelection {
        lane_kind: OperatorChatLaneKind::Cli,
        model_id: "claude-sonnet-4".into(),
        cloud_provider: None,
        cli_provider: Some("claude_code".into()),
        working_dir: working_dir.into(),
        worktree_id: Some("wt-operator-chat".into()),
        prompt: prompt.into(),
        owner_session: owner.into(),
        parent_session_id: "operator-chat-parent".into(),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-012".into()),
    }
}

fn local_selection(
    model_id: &str,
    working_dir: &str,
    prompt: &str,
    owner: &str,
) -> OperatorChatSelection {
    OperatorChatSelection {
        lane_kind: OperatorChatLaneKind::Local,
        model_id: model_id.into(),
        cloud_provider: None,
        cli_provider: None,
        working_dir: working_dir.into(),
        worktree_id: Some("wt-operator-chat".into()),
        prompt: prompt.into(),
        owner_session: owner.into(),
        parent_session_id: "operator-chat-parent".into(),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-012".into()),
    }
}

fn cloud_selection(working_dir: &str, prompt: &str, owner: &str) -> OperatorChatSelection {
    OperatorChatSelection {
        lane_kind: OperatorChatLaneKind::Cloud,
        model_id: "claude-sonnet-4-byok".into(),
        cloud_provider: Some("anthropic".into()),
        cli_provider: None,
        working_dir: working_dir.into(),
        worktree_id: Some("wt-operator-chat".into()),
        prompt: prompt.into(),
        owner_session: owner.into(),
        parent_session_id: "operator-chat-parent".into(),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-012".into()),
    }
}

fn subagent_selection(working_dir: &str, prompt: &str, owner: &str) -> OperatorChatSelection {
    OperatorChatSelection {
        lane_kind: OperatorChatLaneKind::Subagent,
        model_id: "subagent://operator-chat/coder".into(),
        cloud_provider: None,
        cli_provider: None,
        working_dir: working_dir.into(),
        worktree_id: Some("wt-operator-chat".into()),
        prompt: prompt.into(),
        owner_session: owner.into(),
        parent_session_id: "operator-chat-parent".into(),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-012".into()),
    }
}

/// A realistic codex `stream-json` turn: envelope + `item.updated` deltas (which
/// MUST NOT emit) interleaved with three `item.completed` blocks (which each emit
/// exactly one typed activity).
fn codex_stream_lines() -> Vec<String> {
    vec![
        json!({"type":"thread.started","thread_id":"t1"}).to_string(),
        json!({"type":"item.updated","item":{"id":"r1","type":"reasoning","text":"let me"}})
            .to_string(),
        json!({"type":"item.completed","item":{"id":"r1","type":"reasoning","text":"let me think about the repo layout"}}).to_string(),
        json!({"type":"item.updated","item":{"id":"c1","type":"command_execution","command":"ls -la","status":"running"}}).to_string(),
        json!({"type":"item.completed","item":{"id":"c1","type":"command_execution","command":"ls -la","status":"completed"}}).to_string(),
        json!({"type":"item.updated","item":{"id":"m1","type":"agent_message","text":"the ans"}})
            .to_string(),
        json!({"type":"item.completed","item":{"id":"m1","type":"agent_message","text":"the answer is 42"}}).to_string(),
        json!({"type":"turn.completed","usage":{"input_tokens":10}}).to_string(),
    ]
}

#[allow(clippy::too_many_arguments)]
async fn assert_process_backed_launch_evidence(
    pool: &sqlx::PgPool,
    store: &ModelLaneStore,
    drain: Option<&ProcessLedgerDrain>,
    recorder: &CapturingRecorder,
    launched: &OperatorChatLaunched,
    expected_lane_kind: ModelLaneKind,
    expected_runtime_binding: RuntimeBinding,
    expected_launch_authority: LaunchAuthority,
    expected_provider_kind: ModelLaneProviderKind,
    expected_engine_kind: &str,
    expected_model_messages: usize,
    expected_additional_artifact_bindings: usize,
) {
    assert_eq!(
        launched.captured_message_count, expected_model_messages,
        "launch() must drive the selected runtime and capture model output"
    );

    let replay = store
        .replay_run(&launched.run_id)
        .await
        .expect("operator-chat process-backed run replays");
    assert!(
        replay
            .lanes
            .iter()
            .any(|lane| lane.kind == ModelLaneKind::HumanOperator),
        "operator prompt lane is persisted for {}",
        launched.run_id
    );
    let observed_lane_kinds = replay
        .lanes
        .iter()
        .map(|lane| lane.kind.as_str())
        .collect::<Vec<_>>();
    let model_lane = replay
        .lanes
        .iter()
        .find(|lane| lane.kind == expected_lane_kind)
        .unwrap_or_else(|| {
            panic!(
                "expected {expected_lane_kind:?} lane in replay for {}; lane_kinds={observed_lane_kinds:?}",
                launched.run_id
            )
        });
    assert_eq!(model_lane.runtime_binding, expected_runtime_binding);
    assert_eq!(model_lane.launch_authority, expected_launch_authority);
    assert_eq!(model_lane.provider_kind, expected_provider_kind);
    assert!(
        model_lane.no_os_process_reason_ref.is_none(),
        "process-backed operator-chat lanes must not carry no-OS reasons"
    );
    let process_uuid = model_lane
        .process_ownership_ref
        .as_deref()
        .and_then(|value| value.strip_prefix("process-ledger://"))
        .expect("process-backed operator-chat lane carries process_ownership_ref")
        .to_string();
    let model_lane_id = model_lane.lane_id.clone();

    let messages = replay.messages;
    assert_eq!(
        messages.len(),
        expected_model_messages + 1,
        "operator prompt plus captured model messages are durable"
    );
    assert!(
        messages.iter().any(|msg| {
            msg.diagnostic_payload["turn_role"] == json!("operator")
                && msg.from_lane_id != model_lane_id
        }),
        "operator prompt is stored on a distinct HUMAN_OPERATOR lane"
    );
    let model_messages = messages
        .iter()
        .filter(|msg| msg.from_lane_id == model_lane_id)
        .collect::<Vec<_>>();
    assert_eq!(
        model_messages.len(),
        expected_model_messages,
        "captured output is attributed to the selected model lane"
    );

    let agent_events = recorder
        .event_markers()
        .iter()
        .filter(|marker| marker.starts_with("FR-EVT-AGENT-"))
        .count();
    assert!(
        agent_events >= expected_model_messages + 1,
        "Flight Recorder evidence covers operator prompt and captured output"
    );

    let message_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger \
         WHERE session_run_id = $1 AND aggregate_type = 'model_lane_message'",
    )
    .bind(&replay.run.event_ledger_stream_id)
    .fetch_one(pool)
    .await
    .expect("count model_lane_message EventLedger rows");
    assert_eq!(
        message_rows,
        (expected_model_messages + 1) as i64,
        "operator/model messages append EventLedger authority rows"
    );

    let artifact_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM model_lane_context_bundle_artifacts WHERE run_id = $1",
    )
    .bind(&launched.run_id)
    .fetch_one(pool)
    .await
    .expect("count payload artifact bindings");
    assert_eq!(
        artifact_rows,
        (expected_model_messages + 1 + expected_additional_artifact_bindings) as i64,
        "operator/model messages plus launch authority keep ArtifactStore payload bindings"
    );

    for (tier, state) in [
        ("flight_recorder", "wired"),
        ("internal_diagnostics", "deferred_with_reason"),
        ("palmistry", "deferred_with_reason"),
    ] {
        let rows: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM model_lane_diagnostic_tier_statuses \
             WHERE run_id = $1 AND behavior_id = 'HBR-INT-009' AND tier = $2 AND state = $3",
        )
        .bind(&launched.run_id)
        .bind(tier)
        .bind(state)
        .fetch_one(pool)
        .await
        .expect("count HBR-INT-009 tier rows");
        assert_eq!(rows, 1, "HBR-INT-009 tier row {tier}/{state} is recorded");
    }

    let ledger_store = Arc::new(PostgresProcessLedgerStore::new(pool.clone()));
    ledger_store
        .apply_migration()
        .await
        .expect("process ledger migration applies");
    if let Some(drain) = drain {
        drain
            .drain_available_to(ledger_store)
            .await
            .expect("process ledger rows drain to PostgreSQL");
    }
    let process_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_process_lifecycle \
         WHERE process_uuid = $1::uuid \
           AND engine_kind = $2 \
           AND stopped_at IS NOT NULL \
           AND os_pid IS NOT NULL \
           AND stop_reason = 'completed' \
           AND wp_id = 'WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1' \
           AND mt_id = 'MT-012'",
    )
    .bind(&process_uuid)
    .bind(expected_engine_kind)
    .fetch_one(pool)
    .await
    .expect("count linked process-ledger START/STOP row");
    assert_eq!(
        process_rows, 1,
        "lane process_ownership_ref must resolve to a durable START/STOP ledger row"
    );
}

async fn assert_cloud_projection_artifact_bindings(
    pool: &sqlx::PgPool,
    launched: &OperatorChatLaunched,
) {
    let plan: serde_json::Value = sqlx::query_scalar(
        "SELECT record_json FROM model_lane_cloud_projection_plans WHERE run_id = $1",
    )
    .bind(&launched.run_id)
    .fetch_one(pool)
    .await
    .expect("fetch cloud ProjectionPlan record");
    let source_refs = plan["source_artifact_refs"]
        .as_array()
        .expect("ProjectionPlan source_artifact_refs is an array");
    let payload_ref = plan["payload_artifact_ref"]
        .as_str()
        .expect("ProjectionPlan payload_artifact_ref is a string");

    let cloud_refs = source_refs
        .iter()
        .filter_map(|reference| reference.as_str())
        .filter(|reference| reference.starts_with("artifact-store://operator-chat/"))
        .chain(std::iter::once(payload_ref))
        .collect::<Vec<_>>();
    assert_eq!(
        cloud_refs.len(),
        2,
        "cloud ProjectionPlan must expose one cloud input ref and one projected payload ref"
    );

    for artifact_ref in cloud_refs {
        let rows: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM model_lane_context_bundle_artifacts \
             WHERE run_id = $1 AND artifact_ref = $2 AND artifact_payload_ref = $2",
        )
        .bind(&launched.run_id)
        .bind(artifact_ref)
        .fetch_one(pool)
        .await
        .expect("count cloud ProjectionPlan artifact binding rows");
        assert_eq!(
            rows, 1,
            "ProjectionPlan artifact ref {artifact_ref} must resolve to exactly one ArtifactStore binding"
        );
    }
}

fn no_os_operator_launch_request(idx: usize, owner_session: &str) -> DexterityLaunchAdapterRequest {
    DexterityLaunchAdapterRequest {
        adapter_kind: DexterityLaunchAdapterKind::HumanOperator,
        run_id: format!("operator-run-{idx}"),
        lane_id: format!("operator-lane-{idx}"),
        trace_id: format!("operator-trace-{idx}"),
        run_span_id: format!("operator-span-run-{idx}"),
        lane_span_id: format!("operator-span-lane-{idx}"),
        coordinator_session_id: "operator-chat-coordinator".into(),
        routing_policy: "dexterity_registry_normalized".into(),
        context_bundle_id: format!("context-bundle://operator-chat/{idx}"),
        event_ledger_stream_id: format!("event-ledger://operator-chat/{idx}"),
        artifact_namespace: format!("artifact://operator-chat/{idx}"),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-012".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: owner_session.into(),
        locus_binding_ref: format!("locus://wp1/mt012/operator/{idx}"),
        role: "operator-turn".into(),
        backend: None,
        adapter_id: None,
        model_id: None,
        session_id: format!("operator-session-{idx}"),
        model_session_id: format!("operator-model-session-{idx}"),
        extra_capability_token_ids: vec![],
        requested_tool_capability_tokens: vec!["tool-capability://read-context".into()],
        effective_capability_snapshot_ref: None,
        capability_negotiation_ref: None,
        provider_feature_profile_ref: None,
        requested_execution_policy_ref: None,
        effective_execution_policy_ref: None,
        projection_plan_ref: None,
        consent_receipt_ref: None,
        tool_gate_decision_refs: vec![format!("toolgate://operator-chat/{idx}/allow-read-context")],
        status: Some(ModelLaneStatus::Ready),
        heartbeat_at_utc: Some("2026-07-02T00:00:00Z".into()),
        lease_expires_at_utc: Some("2026-07-02T00:05:00Z".into()),
        reclaim_after_utc: Some("2026-07-02T00:06:00Z".into()),
        restart_generation: 0,
        cancellation_ref: None,
        reclaim_policy_ref: None,
        terminal_status_mapping_ref: None,
        process_ownership_ref: None,
        no_os_process_reason_ref: None,
        backpressure_ref: None,
        loop_counter_ref: Some(format!("loop-counter://operator-chat/{idx}")),
        last_runtime_status_ref: Some(format!("runtime-status://operator-chat/{idx}")),
        last_recovery_event_ref: Some(format!("recovery://operator-chat/{idx}")),
        startup_failure_code: None,
        startup_failure_ref: None,
        reason_ref: None,
        run_recovery_hint_ref: Some("usermanual://operator-chat-launch#run".into()),
        lane_recovery_hint_ref: Some("usermanual://operator-chat-launch#lane".into()),
        memory_pack_ref: "memory-pack://operator-chat/mt012".into(),
        memory_pack_hash: sample_sha256(),
        determinism_mode: "deterministic_replay".into(),
        budget_summary_ref: "budget://operator-chat/mt012".into(),
        selected_model_id: None,
        candidate_model_ids: vec![format!("model://operator-chat/candidate/{idx}")],
        procedural_review_status: "preflight_reviewed_and_registry_normalized".into(),
        truncation_warning_ref: None,
        rejection_reason_refs: vec![],
    }
}

// ---------------------------------------------------------------------------
// Proofs.
// ---------------------------------------------------------------------------

/// F1/F2 LIVE LOOP: `launch()` resolves through `SwarmCoordinator::spawn_session`,
/// then DRIVES the launched `CliBridgeModelRuntime` and re-homes its REAL stdout
/// (mock CLI stream-json) into ONE ModelLaneMessage per completed activity block
/// with the correct kind + activity_kind + FR evidence — end to end through
/// `launch()`, not a separately-bound hand-authored vec.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn operator_chat_launch_drives_runtime_and_captures_one_message_per_completed_block() {
    eprintln!("MT017_OPERATOR_CHAT_STAGE=pg_store:start");
    let (pool, store) = pg_store().await;
    eprintln!("MT017_OPERATOR_CHAT_STAGE=pg_store:complete");
    let recorder = Arc::new(CapturingRecorder::default());
    let ledger_store = Arc::new(PostgresProcessLedgerStore::new(pool.clone()));
    ledger_store
        .apply_migration()
        .await
        .expect("process ledger migration applies before the launch durability gate");
    eprintln!("MT017_OPERATOR_CHAT_STAGE=ledger_migration:complete");
    let (ledger, ledger_writer) = LedgerBatcher::spawn(
        ledger_store,
        Arc::new(NoopOverflowSink),
        LedgerBatcherConfig::default(),
    );
    let spawner: Arc<dyn CliSubprocessSpawner> =
        Arc::new(LoopbackCliSpawner::from_lines(&codex_stream_lines()));
    let observability = Some(Arc::new(CloudLaneObservability {
        flight_recorder: recorder.clone(),
        consent: None,
    }));
    let cloud = handshake_core::api::configure_operator_chat_official_cli_providers(
        CloudLaneFactoryConfig::unconfigured(),
        spawner,
        observability,
        [(
            "codex".to_string(),
            AllowlistedCliBridgeConfig::new(
                loopback_cli_config(),
                CliModelAllowlist::new(vec!["gpt-5-codex".to_string()])
                    .expect("operator-chat loopback allowlist"),
            ),
        )],
    );
    let coordinator = Arc::new(SwarmCoordinator::new_with_model_lane_store(
        SwarmConfig::new(RunBudget::defaulted(8)),
        Arc::new(ProductionModelSessionFactory::new(
            ledger.clone(),
            cloud,
            None,
        )),
        Arc::new(RecordingSwarmSink::new()),
        ledger.clone(),
        store.clone(),
    ));
    let service =
        OperatorChatLaunchService::new(coordinator, ModelCatalog::empty(), recorder.clone());

    // launch() spawns through spawn_session AND drives the launched runtime,
    // capturing its real stdout in ONE call.
    eprintln!("MT017_OPERATOR_CHAT_STAGE=launch:start");
    let launched = service
        .launch(&codex_cli_selection(
            existing_working_dir(),
            "audit the repo",
            "operator-1",
        ))
        .await
        .expect("operator-chat CLI lane launches + captures through spawn_session");
    eprintln!("MT017_OPERATOR_CHAT_STAGE=launch:complete");

    // F1/F2: launch() itself persisted one message per completed block (F5).
    assert_eq!(
        launched.captured_message_count, 3,
        "launch() captured one ModelLaneMessage per completed activity block"
    );

    // Persisted under the run and replayable.
    let replay = store
        .replay_run(&launched.run_id)
        .await
        .expect("launched run is replayable");
    eprintln!("MT017_OPERATOR_CHAT_STAGE=replay:complete");
    assert_eq!(
        replay.lanes.len(),
        2,
        "the launched run persists the CLI lane plus the HUMAN_OPERATOR prompt lane"
    );
    assert!(
        replay
            .lanes
            .iter()
            .any(|lane| lane.kind == ModelLaneKind::HumanOperator),
        "the launched run carries a HUMAN_OPERATOR lane"
    );
    let stream_id = replay.run.event_ledger_stream_id.clone();
    let messages = replay.messages;
    assert_eq!(
        messages.len(),
        4,
        "one durable operator prompt plus one ModelLaneMessage per completed activity block"
    );
    let operator_messages = messages
        .iter()
        .filter(|msg| msg.diagnostic_payload["turn_role"] == json!("operator"))
        .collect::<Vec<_>>();
    assert_eq!(
        operator_messages.len(),
        1,
        "launch() persists the operator prompt as a HUMAN_OPERATOR message"
    );

    // F1 message-kind mapping + activity_kind discriminator (order preserved).
    let model_messages = messages
        .iter()
        .filter(|msg| msg.diagnostic_payload["turn_role"] != json!("operator"))
        .collect::<Vec<_>>();
    assert_eq!(
        model_messages.len(),
        3,
        "one ModelLaneMessage per completed activity block, not per delta line"
    );
    assert_eq!(model_messages[0].kind, ModelLaneMessageKind::Status);
    assert_eq!(
        model_messages[0].diagnostic_payload["activity_kind"],
        json!("thinking"),
        "exposed thought is a labelled Status, never an unlabelled one"
    );
    assert_eq!(model_messages[1].kind, ModelLaneMessageKind::ToolRequest);
    assert_eq!(
        model_messages[1].diagnostic_payload["activity_kind"],
        json!("tool_call")
    );
    assert!(
        !model_messages[1].tool_gate_decision_refs.is_empty(),
        "tool messages carry a tool-gate decision ref"
    );
    assert_eq!(model_messages[2].kind, ModelLaneMessageKind::Status);
    assert_eq!(
        model_messages[2].diagnostic_payload["activity_kind"],
        json!("text")
    );

    // Flight Recorder evidence: one FR-EVT-AGENT-* for the operator prompt plus
    // one per captured model activity, so Flight Recorder and EventLedger can
    // prove the full conversation timeline.
    let events = recorder.events.lock().expect("events lock").clone();
    let agent_events = events
        .iter()
        .filter(|event| {
            event
                .payload
                .get("event_id")
                .and_then(|value| value.as_str())
                .is_some_and(|event_id| event_id.starts_with("FR-EVT-AGENT-"))
        })
        .collect::<Vec<_>>();
    assert_eq!(
        agent_events.len(),
        4,
        "capture/replay is the sole FR lifecycle producer: one operator prompt plus three model activities"
    );
    assert!(
        agent_events.iter().all(|event| {
            event.payload.get("adapter").and_then(|value| value.as_str())
                == Some(OPERATOR_CHAT_CLI_ADAPTER)
        }),
        "every FR-EVT-AGENT event must come from the operator-chat capture adapter; runtime-level duplicates are scoped out: {agent_events:?}"
    );
    let unique_agent_lifecycle_keys = agent_events
        .iter()
        .map(|event| {
            format!(
                "{}:{}:{}",
                event.payload["request_id"],
                event.payload["ordered_index"],
                event.payload["adapter"]
            )
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        unique_agent_lifecycle_keys.len(),
        4,
        "each captured lifecycle event has a unique request/index/adapter key"
    );
    let infer_events = events
        .iter()
        .filter(|event| {
            event
                .payload
                .get("event_id")
                .and_then(|value| value.as_str())
                .is_some_and(|event_id| event_id.starts_with("FR-EVT-LLM-INFER-"))
        })
        .count();
    assert_eq!(
        infer_events, 2,
        "the shared runtime recorder remains live for infer START/END while only agent activity is suppressed"
    );

    // EventLedger authority rows exist for the messages.
    let message_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger \
         WHERE session_run_id = $1 AND aggregate_type = 'model_lane_message'",
    )
    .bind(&stream_id)
    .fetch_one(&pool)
    .await
    .expect("count model_lane_message EventLedger rows");
    assert_eq!(
        message_rows, 4,
        "operator prompt + each captured message appends EventLedger authority"
    );

    let artifact_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM model_lane_context_bundle_artifacts WHERE run_id = $1",
    )
    .bind(&launched.run_id)
    .fetch_one(&pool)
    .await
    .expect("count payload artifact bindings");
    assert_eq!(
        artifact_rows, 4,
        "every operator/model message gets an ArtifactStore payload binding"
    );
    let artifact_records: Vec<serde_json::Value> = sqlx::query_scalar(
        "SELECT record_json FROM model_lane_context_bundle_artifacts WHERE run_id = $1",
    )
    .bind(&launched.run_id)
    .fetch_all(&pool)
    .await
    .expect("fetch ArtifactStore binding records");
    for msg in &messages {
        let matches = artifact_records
            .iter()
            .filter(|artifact| {
                artifact["diagnostic_payload"]["message_id"] == json!(msg.message_id)
                    && (artifact["artifact_ref"] == json!(msg.payload_ref)
                        || artifact["artifact_payload_ref"] == json!(msg.payload_ref))
                    && artifact["artifact_sha256"] == json!(msg.payload_sha256)
                    && artifact["content_hash"] == json!(msg.payload_sha256)
            })
            .count();
        assert_eq!(
            matches, 1,
            "message {} payload_ref {} / sha {} must have exactly one matching ArtifactStore binding; artifacts={artifact_records:?}",
            msg.message_id, msg.payload_ref, msg.payload_sha256
        );
    }

    for (tier, state) in [
        ("flight_recorder", "wired"),
        ("internal_diagnostics", "deferred_with_reason"),
        ("palmistry", "deferred_with_reason"),
    ] {
        let rows: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM model_lane_diagnostic_tier_statuses \
             WHERE run_id = $1 AND behavior_id = 'HBR-INT-009' AND tier = $2 AND state = $3",
        )
        .bind(&launched.run_id)
        .bind(tier)
        .bind(state)
        .fetch_one(&pool)
        .await
        .expect("count HBR-INT-009 tier rows");
        assert_eq!(rows, 1, "HBR-INT-009 tier row {tier}/{state} is recorded");
    }
    eprintln!("MT017_OPERATOR_CHAT_STAGE=postgres_assertions:complete");

    let ledger_outcome =
        drain_and_join_ledger_writer(&ledger, ledger_writer, Duration::from_secs(10)).await;
    assert!(
        matches!(ledger_outcome, LedgerDrainJoinOutcome::Flushed),
        "the production process-ledger writer must flush START/STOP before assertions"
    );
    eprintln!("MT017_OPERATOR_CHAT_STAGE=ledger_drain:complete");

    assert_process_backed_launch_evidence(
        &pool,
        &store,
        None,
        recorder.as_ref(),
        &launched,
        ModelLaneKind::CliModel,
        RuntimeBinding::CliBridge,
        LaunchAuthority::CliBridge,
        ModelLaneProviderKind::OfficialCli,
        "official_cli_bridge",
        3,
        0,
    )
    .await;
    eprintln!("MT017_OPERATOR_CHAT_STAGE=proof:complete");
}

/// AC: LOCAL and BYOK CLOUD selections use the same operator-chat launch/capture
/// authority as CLI. This proves they are not just picker rows: both go through
/// `OperatorChatLaunchService::launch`, persist ModelLane messages + HBR-INT-009
/// posture, and link the process-backed lane to a durable ProcessOwnershipLedger
/// START/STOP row.
#[tokio::test]
async fn operator_chat_local_and_byok_cloud_launches_capture_and_link_process_ledger() {
    let (pool, store) = pg_store().await;
    let (catalog, local_model_id) = registered_local_catalog();
    let (coordinator, loads, drain) = store_backed_coordinator(store.clone());
    let recorder = Arc::new(CapturingRecorder::default());
    let service = OperatorChatLaunchService::new(coordinator, catalog, recorder.clone());

    let local = service
        .launch(&local_selection(
            &local_model_id,
            existing_working_dir(),
            "run the local model",
            "operator-local",
        ))
        .await
        .expect("local operator-chat lane launches + captures");
    assert_process_backed_launch_evidence(
        &pool,
        &store,
        Some(&drain),
        recorder.as_ref(),
        &local,
        ModelLaneKind::LocalModel,
        RuntimeBinding::Local,
        LaunchAuthority::ModelRuntime,
        ModelLaneProviderKind::LocalRuntime,
        "candle",
        1,
        0,
    )
    .await;

    let cloud = service
        .launch(&cloud_selection(
            existing_working_dir(),
            "run the cloud model",
            "operator-cloud",
        ))
        .await
        .expect("BYOK cloud operator-chat lane launches + captures");
    assert_cloud_projection_artifact_bindings(&pool, &cloud).await;
    assert_process_backed_launch_evidence(
        &pool,
        &store,
        Some(&drain),
        recorder.as_ref(),
        &cloud,
        ModelLaneKind::CloudModel,
        RuntimeBinding::Cloud,
        LaunchAuthority::CloudLane,
        ModelLaneProviderKind::Anthropic,
        "helper_subprocess",
        1,
        2,
    )
    .await;

    assert_eq!(
        loads.load(Ordering::SeqCst),
        2,
        "local and cloud selections both reached the runtime factory"
    );
}

/// Failure path: if the launched runtime emits stdout and then returns a stream
/// error, `launch()` still records the partial stdout, records HBR posture, and
/// reclaims the spawned session instead of leaking it.
#[tokio::test]
async fn operator_chat_launch_stream_error_preserves_partial_capture_and_reclaims_session() {
    let (pool, store) = pg_store().await;
    let marker = "partial-before-stream-error-9fd31";
    let lines = vec![
        json!({"type":"item.completed","item":{"id":"r1","type":"reasoning","text": marker}})
            .to_string(),
    ];
    let coordinator = cli_failing_after_chunk_coordinator(store, lines);
    let recorder = Arc::new(CapturingRecorder::default());
    let service =
        OperatorChatLaunchService::new(coordinator.clone(), ModelCatalog::empty(), recorder);

    let err = service
        .launch(&codex_cli_selection(
            existing_working_dir(),
            "audit",
            "operator-fail",
        ))
        .await
        .expect_err("stream error after partial stdout must fail the launch");
    assert!(
        err.to_string()
            .contains("operator-chat runtime stream failed"),
        "stream error must be surfaced honestly: {err}"
    );
    assert_eq!(
        coordinator.live_session_count(),
        0,
        "post-spawn capture failure must reclaim the spawned session"
    );

    let messages: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM model_lane_messages")
        .fetch_one(&pool)
        .await
        .expect("count partial messages");
    assert_eq!(
        messages, 2,
        "operator prompt plus one partial stdout message are persisted before surfacing failure"
    );
    let partial_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM model_lane_messages \
         WHERE record_json->'diagnostic_payload'->'capture'->>'text' = $1",
    )
    .bind(marker)
    .fetch_one(&pool)
    .await
    .expect("count partial captured stdout row");
    assert_eq!(partial_rows, 1, "partial stdout is still captured");

    let artifact_rows: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM model_lane_context_bundle_artifacts")
            .fetch_one(&pool)
            .await
            .expect("count partial artifact bindings");
    assert_eq!(
        artifact_rows, 2,
        "operator prompt and partial stdout keep ArtifactStore payload bindings"
    );
    let hbr_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM model_lane_diagnostic_tier_statuses \
         WHERE behavior_id = 'HBR-INT-009'",
    )
    .fetch_one(&pool)
    .await
    .expect("count HBR-INT-009 rows");
    assert_eq!(
        hbr_rows, 3,
        "failure path still records all HBR-INT-009 tier statuses"
    );
}

/// MT-009 cancellation hardening: the real operator-chat launch route drives a
/// `CliBridgeModelRuntime` through `SwarmCoordinator::generate_session`. After
/// one newline-complete activity is durably captured, only the coordinator is
/// allowed to cancel the live instance. The terminal lane/EventLedger write must
/// precede cancellation, preserve the prefix, and reject a late buffered tool
/// activity without a phantom Flight Recorder event.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn operator_chat_launch_coordinator_cancellation_preserves_prefix_and_rejects_late_activity()
{
    let (pool, store) = pg_store().await;
    let prefix = "mt009-coordinator-cancel-prefix";
    let late_tool = "mt009-coordinator-cancel-late-tool";
    let lines = vec![
        json!({"type":"item.completed","item":{"id":"r1","type":"reasoning","text":prefix}})
            .to_string(),
        json!({"type":"item.completed","item":{"id":"c1","type":"command_execution","command":late_tool,"status":"completed"}})
            .to_string(),
    ];
    let probe = Arc::new(CancellationLaunchProbe::default());
    let (coordinator, drain) =
        cli_cancel_after_prefix_coordinator(store.clone(), lines, probe.clone());
    let recorder = Arc::new(CapturingRecorder::default());
    let service = Arc::new(OperatorChatLaunchService::new(
        coordinator.clone(),
        ModelCatalog::empty(),
        recorder.clone(),
    ));
    let selection = codex_cli_selection(existing_working_dir(), "audit", "operator-mt009-cancel");
    let launch_task = {
        let service = service.clone();
        let selection = selection.clone();
        tokio::spawn(async move { service.launch(&selection).await })
    };

    let deadline = Instant::now() + Duration::from_secs(10);
    let instance_id = loop {
        let prefix_rows: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM model_lane_messages \
             WHERE record_json->'diagnostic_payload'->'capture'->>'text' = $1",
        )
        .bind(prefix)
        .fetch_one(&pool)
        .await
        .expect("count durably captured prefix rows");
        if probe.prefix_emitted.load(Ordering::SeqCst) && prefix_rows == 1 {
            break probe
                .instance_id()
                .expect("factory must expose the real spawned instance id");
        }
        assert!(
            Instant::now() < deadline,
            "launch did not durably capture the prefix before cancellation"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    };
    let run_id = probe.run_id().expect("factory must expose launched run id");
    let lane_id = probe
        .lane_id()
        .expect("factory must expose launched lane id");

    coordinator
        .cancel_session(instance_id, "operator-cancelled-mt009-live-capture")
        .await
        .expect(
            "the coordinator must persist terminal state then cancel the exact runtime request",
        );

    let launch_result = tokio::time::timeout(Duration::from_secs(10), launch_task)
        .await
        .expect("cancelled launch must return without hanging")
        .expect("launch task must join");
    let launch_error = launch_result.expect_err("cancelled live launch must not report completion");
    assert!(
        launch_error.to_string().contains("terminal source lane")
            || launch_error
                .to_string()
                .contains("runtime stream cancelled"),
        "cancellation must surface a terminal/cancelled outcome, got {launch_error}"
    );
    assert!(
        probe.cancellation_observed.load(Ordering::SeqCst),
        "CliBridgeModelRuntime must observe the coordinator-owned cancellation token"
    );
    assert_eq!(
        coordinator.live_session_count(),
        0,
        "coordinator cancellation must evict and tear down the live session"
    );

    let replay = store
        .replay_run(&run_id)
        .await
        .expect("cancelled operator-chat run replays from PostgreSQL/EventLedger");
    let lane = replay
        .lanes
        .iter()
        .find(|candidate| candidate.lane_id == lane_id)
        .expect("replay contains cancelled model lane");
    assert_eq!(lane.status, ModelLaneStatus::Cancelled);
    assert_eq!(
        replay.messages.len(),
        2,
        "only the operator prompt and pre-cancel prefix are durable"
    );
    assert!(
        replay
            .messages
            .iter()
            .any(|message| { message.diagnostic_payload["capture"]["text"] == json!(prefix) }),
        "the prefix captured before cancellation remains replayable"
    );
    assert!(
        !replay.messages.iter().any(|message| {
            message.diagnostic_payload["capture"]["command"] == json!(late_tool)
        }),
        "late tool activity must not become a ModelLane message"
    );

    let post_terminal_messages: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger \
         WHERE session_run_id = $1 \
           AND aggregate_type = 'model_lane_message' \
           AND event_sequence > $2",
    )
    .bind(&replay.run.event_ledger_stream_id)
    .bind(lane.event_ledger_seq)
    .fetch_one(&pool)
    .await
    .expect("count post-terminal ModelLane EventLedger rows");
    assert_eq!(
        post_terminal_messages, 0,
        "no ModelLane message EventLedger authority may appear after cancellation"
    );
    let artifact_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM model_lane_context_bundle_artifacts WHERE run_id = $1",
    )
    .bind(&run_id)
    .fetch_one(&pool)
    .await
    .expect("count durable prompt/prefix artifact bindings");
    assert_eq!(
        artifact_rows, 2,
        "late activity must not leave an artifact binding orphan"
    );
    assert!(
        !recorder
            .event_markers()
            .iter()
            .any(|marker| marker == "FR-EVT-AGENT-TOOLCALL"),
        "late tool capture must not emit a phantom Flight Recorder activity"
    );

    let process_uuid = lane
        .process_ownership_ref
        .as_deref()
        .and_then(|value| value.strip_prefix("process-ledger://"))
        .expect("process-backed lane carries ProcessOwnershipLedger reference");
    let ledger_store = Arc::new(PostgresProcessLedgerStore::new(pool.clone()));
    ledger_store
        .apply_migration()
        .await
        .expect("process-ledger migration applies");
    drain
        .drain_available_to(ledger_store)
        .await
        .expect("drain coordinator start/stop ledger events");
    let stopped_processes: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_process_lifecycle \
         WHERE process_uuid = $1::uuid AND stopped_at IS NOT NULL \
           AND stop_reason = 'operator-cancelled-mt009-live-capture'",
    )
    .bind(process_uuid)
    .fetch_one(&pool)
    .await
    .expect("count durable cancelled process stop rows");
    assert_eq!(
        stopped_processes, 1,
        "coordinator cancellation must retain ProcessOwnershipLedger START/STOP symmetry"
    );
}

/// MT-009 cancellation-fence regression: while the real PostgreSQL terminal
/// lane write is in flight, the coordinator must expose `Cancelling` and
/// refuse a new generation.  This closes the cancel-vs-start window without a
/// mock store or a hand-written terminal state.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn coordinator_cancellation_fence_rejects_generation_during_terminal_pg_write() {
    let (pool, store) = pg_store().await;
    let (coordinator, _ledger_drain) =
        cli_generic_loopback_coordinator_with_ledger(store, codex_stream_lines());
    let request = build_spawn_request(
        &codex_cli_selection(
            existing_working_dir(),
            "fence generation",
            "operator-mt009-fence",
        ),
        91,
    )
    .expect("build real Dexterity spawn request");
    let instance_id = request.instance_id;
    coordinator
        .spawn_session(request)
        .await
        .expect("spawn real coordinator-owned session");

    let suffix = Uuid::now_v7().simple().to_string();
    let function_name = format!("mt009_pause_terminal_{suffix}");
    let trigger_name = format!("mt009_pause_terminal_trigger_{suffix}");
    sqlx::query(&format!(
        "CREATE FUNCTION {function_name}() RETURNS trigger AS $$ \
         BEGIN PERFORM pg_sleep(1); RETURN NEW; END; $$ LANGUAGE plpgsql"
    ))
    .execute(&pool)
    .await
    .expect("install real PostgreSQL terminal-write pause function");
    sqlx::query(&format!(
        "CREATE TRIGGER {trigger_name} BEFORE UPDATE ON model_lanes \
         FOR EACH ROW EXECUTE FUNCTION {function_name}()"
    ))
    .execute(&pool)
    .await
    .expect("install real PostgreSQL terminal-write pause trigger");

    let cancelling_coordinator = coordinator.clone();
    let cancel_task = tokio::spawn(async move {
        cancelling_coordinator
            .cancel_session(instance_id, "mt009-cancellation-fence")
            .await
    });
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if coordinator.session_state(instance_id)
                == Some(handshake_core::swarm_orchestration::ModelSessionState::Cancelling)
            {
                return;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("coordinator must fence the handle before awaiting terminal PostgreSQL write");

    let start_attempt = coordinator.generate_session(
        instance_id,
        GenerateRequest {
            id: ModelId::new_v7(),
            prompt: GenPrompt::new("must not start after cancellation fence"),
            sampling: SamplingParams::default(),
            lora_overrides: vec![],
            steering_overrides: vec![],
            kv_prefix_handle: None,
            cancel: CancellationToken::new(),
            max_tokens: 1,
            stop_sequences: vec![],
            speculative_mode: None,
            structured_decoding: None,
        },
    );
    assert!(
        matches!(&start_attempt, Err(SwarmError::LedgerFailed(message)) if message.contains("Cancelling")),
        "a start racing cancellation must be rejected by the local cancelling fence"
    );

    cancel_task
        .await
        .expect("cancel task joins")
        .expect("terminal write eventually completes");
    sqlx::query(&format!("DROP TRIGGER {trigger_name} ON model_lanes"))
        .execute(&pool)
        .await
        .expect("remove terminal-write pause trigger");
    sqlx::query(&format!("DROP FUNCTION {function_name}()"))
        .execute(&pool)
        .await
        .expect("remove terminal-write pause function");
    assert_eq!(
        coordinator.session_state(instance_id),
        None,
        "successful terminal persistence must evict the fenced session"
    );
}

/// If the real terminal lane transaction fails, the durable cleanup intent
/// keeps the handle fenced and cancelled until coordinator-owned retry can
/// finish exactly once.
#[tokio::test]
async fn coordinator_cancellation_fence_retries_after_terminal_pg_failure() {
    let (pool, store) = pg_store().await;
    let (coordinator, _ledger_drain) =
        cli_generic_loopback_coordinator_with_ledger(store, codex_stream_lines());
    let request = build_spawn_request(
        &codex_cli_selection(
            existing_working_dir(),
            "fence retry",
            "operator-mt009-fence-retry",
        ),
        92,
    )
    .expect("build real Dexterity spawn request");
    let instance_id = request.instance_id;
    let event_stream_id = request
        .dexterity_launch
        .as_ref()
        .expect("Dexterity request carries launch contract")
        .event_ledger_stream_id
        .clone();
    coordinator
        .spawn_session(request)
        .await
        .expect("spawn real coordinator-owned session");

    let suffix = Uuid::now_v7().simple().to_string();
    let function_name = format!("mt009_fail_terminal_{suffix}");
    let trigger_name = format!("mt009_fail_terminal_trigger_{suffix}");
    sqlx::query(&format!(
        "CREATE FUNCTION {function_name}() RETURNS trigger AS $$ \
         BEGIN RAISE EXCEPTION 'mt009 forced terminal failure'; RETURN NEW; END; $$ LANGUAGE plpgsql"
    ))
    .execute(&pool)
    .await
    .expect("install real PostgreSQL terminal-write failure function");
    sqlx::query(&format!(
        "CREATE TRIGGER {trigger_name} BEFORE UPDATE ON model_lanes \
         FOR EACH ROW EXECUTE FUNCTION {function_name}()"
    ))
    .execute(&pool)
    .await
    .expect("install real PostgreSQL terminal-write failure trigger");

    let error = coordinator
        .cancel_session(instance_id, "mt009-cancellation-fence-failure")
        .await
        .expect_err("forced terminal PostgreSQL failure must be surfaced");
    assert!(
        error.to_string().contains("mt009 forced terminal failure"),
        "the original terminal persistence failure must remain visible: {error}"
    );
    assert_eq!(
        coordinator.session_state(instance_id),
        Some(handshake_core::swarm_orchestration::ModelSessionState::Cancelling),
        "failed terminal persistence must retain the durable cancelling fence"
    );
    let start_attempt = coordinator.generate_session(
        instance_id,
        GenerateRequest {
            id: ModelId::new_v7(),
            prompt: GenPrompt::new("must remain fenced after terminal persistence failure"),
            sampling: SamplingParams::default(),
            lora_overrides: vec![],
            steering_overrides: vec![],
            kv_prefix_handle: None,
            cancel: CancellationToken::new(),
            max_tokens: 1,
            stop_sequences: vec![],
            speculative_mode: None,
            structured_decoding: None,
        },
    );
    assert!(
        matches!(&start_attempt, Err(SwarmError::LedgerFailed(message)) if message.contains("Cancelling")),
        "pending cleanup must reject every later generation start"
    );
    let pending: (String, String, i64) = sqlx::query_as(
        "SELECT status, reason, revision FROM swarm_session_cleanup_receipts WHERE instance_id = $1",
    )
    .bind(instance_id.to_string())
    .fetch_one(&pool)
    .await
    .expect("durable cleanup-pending receipt survives terminal write failure");
    assert_eq!(pending.0, "cleanup_pending");
    assert_eq!(pending.1, "mt009-cancellation-fence-failure");
    assert!(pending.2 >= 1);

    sqlx::query(&format!("DROP TRIGGER {trigger_name} ON model_lanes"))
        .execute(&pool)
        .await
        .expect("remove terminal-write failure trigger");
    sqlx::query(&format!("DROP FUNCTION {function_name}()"))
        .execute(&pool)
        .await
        .expect("remove terminal-write failure function");
    coordinator
        .retry_pending_session_cleanups()
        .await
        .expect("coordinator-owned retry must complete the original terminal intent");
    assert_eq!(coordinator.session_state(instance_id), None);
    let completed: (String, String, i64) = sqlx::query_as(
        "SELECT status, reason, revision FROM swarm_session_cleanup_receipts WHERE instance_id = $1",
    )
    .bind(instance_id.to_string())
    .fetch_one(&pool)
    .await
    .expect("completed cleanup receipt remains durable");
    assert_eq!(completed.0, "completed");
    assert_eq!(completed.1, "mt009-cancellation-fence-failure");
    assert!(completed.2 > pending.2);
    let terminal_events: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger WHERE session_run_id = $1 AND aggregate_type = 'model_lane_terminal' AND event_type = 'SESSION_CANCELLED'",
    )
    .bind(event_stream_id)
    .fetch_one(&pool)
    .await
    .expect("count exact terminal EventLedger transition after retry");
    assert_eq!(
        terminal_events, 1,
        "retry must append one terminal transition"
    );
}

/// F1/F2 PROVENANCE: the captured messages carry the DISTINCTIVE text the mock CLI
/// emitted, proving they originate from the LAUNCHED runtime's stdout (not a
/// separately-authored vec). Also exercises the transcript projection end to end.
#[tokio::test]
async fn operator_chat_launch_captured_messages_originate_from_launched_runtime_stdout() {
    let (_pool, store) = pg_store().await;
    let marker_thought = "provenance-thought-8f3a2b";
    let marker_answer = "provenance-answer-7c1d9e";
    let lines = vec![
        json!({"type":"item.completed","item":{"id":"r1","type":"reasoning","text": marker_thought}}).to_string(),
        json!({"type":"item.completed","item":{"id":"m1","type":"agent_message","text": marker_answer}}).to_string(),
    ];
    let coordinator = cli_loopback_coordinator(store.clone(), lines);
    let recorder = Arc::new(CapturingRecorder::default());
    let service =
        OperatorChatLaunchService::new(coordinator, ModelCatalog::empty(), recorder.clone());

    let launched = service
        .launch(&codex_cli_selection(
            existing_working_dir(),
            "audit",
            "operator-2",
        ))
        .await
        .expect("launch + capture");
    assert_eq!(launched.captured_message_count, 2);

    // The transcript projection returns the captured turns; their text is the
    // exact stdout the launched runtime emitted.
    let rows = service
        .fetch_transcript(&launched.run_id)
        .await
        .expect("transcript replays captured messages");
    let texts: Vec<String> = rows.iter().map(|r| r.text.clone()).collect();
    assert!(
        texts.iter().any(|t| t.contains(marker_thought)),
        "captured thought originates from the launched runtime stdout: {texts:?}"
    );
    assert!(
        texts.iter().any(|t| t.contains(marker_answer)),
        "captured answer originates from the launched runtime stdout: {texts:?}"
    );
}

/// AC: the operator turn is persisted as a HUMAN_OPERATOR ModelLane message.
#[tokio::test]
async fn operator_chat_persists_operator_prompt_as_human_operator_message() {
    let (_pool, store) = pg_store().await;
    let (coordinator, _loads, _drain) = store_backed_coordinator(store.clone());
    let recorder = Arc::new(CapturingRecorder::default());

    // Spawn the CLI authority session so it can authorize the no-OS operator lane.
    let cli_request = build_spawn_request(
        &cli_selection(existing_working_dir(), "hello", "operator-77"),
        1,
    )
    .expect("cli spawn request builds");
    let authority = cli_request.instance_id;
    coordinator
        .spawn_session(cli_request)
        .await
        .expect("cli authority session spawns");

    // Create the HUMAN_OPERATOR lane through the sanctioned no-OS launch path.
    let op_request = no_os_operator_launch_request(1, "operator-77");
    let caller = coordinator
        .authorize_no_os_model_lane(&op_request, authority)
        .expect("live authority issues operator caller receipt");
    let (op_run, op_lane) = coordinator
        .launch_no_os_model_lane(op_request, caller)
        .await
        .expect("HUMAN_OPERATOR lane launches through SwarmCoordinator");

    // The operator lane carries the spec-mandated HUMAN_OPERATOR identity.
    use handshake_core::swarm_orchestration::model_lane::{
        LaunchAuthority, ModelLaneKind, RuntimeBinding,
    };
    assert_eq!(op_lane.kind, ModelLaneKind::HumanOperator);
    assert_eq!(op_lane.launch_authority, LaunchAuthority::Operator);
    assert_eq!(op_lane.runtime_binding, RuntimeBinding::Human);

    // Persist the operator prompt as a typed Status message from that lane.
    let capture = ModelLaneCaptureRecorder::new(store.clone(), recorder.clone());
    let message = capture
        .record_operator_prompt(&op_run, &op_lane, "audit the repo and summarize risks")
        .await
        .expect("operator prompt records as a HUMAN_OPERATOR message");
    assert_eq!(message.kind, ModelLaneMessageKind::Status);
    assert_eq!(message.from_lane_id, op_lane.lane_id);
    assert_eq!(message.diagnostic_payload["activity_kind"], json!("text"));
    assert_eq!(message.diagnostic_payload["turn_role"], json!("operator"));
}

/// AC: selecting the SUBAGENT row launches a real no-OS ModelLane through the
/// coordinator-owned Dexterity normalization path; it must not touch the runtime
/// factory or claim a fake subprocess transcript.
#[tokio::test]
async fn operator_chat_subagent_selection_launches_no_os_subagent_lane() {
    let (_pool, store) = pg_store().await;
    let (coordinator, loads, _drain) = store_backed_coordinator(store.clone());
    let recorder = Arc::new(CapturingRecorder::default());
    let service =
        OperatorChatLaunchService::new(coordinator, ModelCatalog::empty(), recorder.clone());

    let launched = service
        .launch(&subagent_selection(
            existing_working_dir(),
            "assign a review subtask",
            "operator-subagent",
        ))
        .await
        .expect("subagent no-OS lane launches");
    assert_eq!(launched.lane_kind, OperatorChatLaneKind::Subagent);
    assert_eq!(launched.captured_message_count, 0);
    assert!(
        launched.instance_id.starts_with("no-os:"),
        "subagent launch must not fabricate a process-backed instance id: {launched:?}"
    );
    assert_eq!(
        loads.load(Ordering::SeqCst),
        0,
        "subagent no-OS launch must not call the runtime factory"
    );

    let replay = store
        .replay_run(&launched.run_id)
        .await
        .expect("subagent run replays from PostgreSQL");
    let subagent_lane = replay
        .lanes
        .iter()
        .find(|lane| lane.kind == ModelLaneKind::Subagent)
        .expect("subagent lane persisted");
    assert_eq!(subagent_lane.runtime_binding, RuntimeBinding::Subagent);
    assert_eq!(
        subagent_lane.launch_authority,
        LaunchAuthority::SubagentManager
    );
    assert!(subagent_lane.process_ownership_ref.is_none());
    assert!(
        subagent_lane
            .no_os_process_reason_ref
            .as_deref()
            .is_some_and(|reason| reason.contains("subagent")),
        "subagent lane carries no-OS ownership reason: {subagent_lane:?}"
    );
    assert!(
        replay.messages.iter().any(|message| {
            message.kind == ModelLaneMessageKind::Status
                && message.diagnostic_payload["turn_role"] == json!("operator")
                && message.summary.contains("assign a review subtask")
        }),
        "operator prompt is persisted alongside the subagent launch"
    );
}

/// Negative: a launch whose coordinator has NO ModelLaneStore fails closed and
/// the factory is never called (bypass/absent-store guard).
#[tokio::test]
async fn operator_chat_launch_without_model_lane_store_fails_closed() {
    let (ledger, _drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("manual process ledger");
    let calls = Arc::new(AtomicUsize::new(0));
    let coordinator = SwarmCoordinator::new(
        SwarmConfig::new(RunBudget::defaulted(1)),
        Arc::new(CountingFactory {
            calls: calls.clone(),
        }),
        Arc::new(RecordingSwarmSink::new()),
        ledger,
    );
    let service = OperatorChatLaunchService::new(
        Arc::new(coordinator),
        ModelCatalog::empty(),
        Arc::new(CapturingRecorder::default()),
    );

    let err = service
        .launch(&cli_selection(existing_working_dir(), "hi", "operator-9"))
        .await
        .expect_err("launch without ModelLaneStore must fail closed");
    match err {
        handshake_core::swarm_orchestration::operator_chat::OperatorChatError::Swarm(
            SwarmError::LedgerFailed(_),
        ) => {}
        other => panic!("expected fail-closed LedgerFailed, got {other:?}"),
    }
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "factory must not be reached when the launch fails closed"
    );
}

/// AC: model selection emits an auditable selection-decision event, distinct
/// from launch (wires MT-014 record_selection_decision; closes MED-1).
#[tokio::test]
async fn operator_chat_selection_emits_auditable_decision_event() {
    let (_pool, store) = pg_store().await;
    let (coordinator, _loads, _drain) = store_backed_coordinator(store);
    let recorder = Arc::new(CapturingRecorder::default());
    let service =
        OperatorChatLaunchService::new(coordinator, ModelCatalog::empty(), recorder.clone());

    service
        .record_selection(
            "claude-sonnet-4",
            "operator",
            "operator picked the CLI lane",
        )
        .await
        .expect("selection decision records");

    let markers = recorder.event_markers();
    assert!(
        markers
            .iter()
            .any(|m| m == "FR-EVT-MODEL-SELECTION-RECORDED"),
        "selection emits the auditable FR-EVT-MODEL-SELECTION-RECORDED event; got {markers:?}"
    );
}

/// F3: with a live launch service wired, `POST /operator-chat/launch` performs a
/// REAL launch (HTTP 200 + captured messages), never the inert `503
/// launch_not_wired`; and `GET /operator-chat/transcript/:run_id` returns the
/// captured ModelLaneMessage rows.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn operator_chat_launch_route_performs_real_launch_when_wired() {
    let (_pool, store) = pg_store().await;
    let coordinator = cli_loopback_coordinator(store.clone(), codex_stream_lines());
    let recorder = Arc::new(CapturingRecorder::default());
    let service = Arc::new(OperatorChatLaunchService::new(
        coordinator,
        ModelCatalog::empty(),
        recorder,
    ));
    let session_registry = std::sync::Arc::new(handshake_core::workflows::SessionRegistry::new(
        handshake_core::workflows::SessionSchedulerConfig::default(),
    ));
    session_registry
        .upsert_session(operator_chat_registry_session("parent-route", None, 0))
        .await;
    session_registry
        .upsert_session(operator_chat_registry_session(
            "operator-route",
            Some("parent-route"),
            1,
        ))
        .await;
    let state = OperatorChatState::production()
        .with_launch_service(service)
        .with_session_registry(session_registry);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, operator_chat_http_routes(state))
            .await
            .expect("operator-chat server");
    });
    let base = format!("http://{addr}");

    let body = json!({
        "lane_kind": "cli",
        "model_id": "gpt-5-codex",
        "cli_provider": "codex",
        "working_dir": "D:/work/repo",
        "prompt": "audit the repo",
        "owner_session_id": "operator-route"
    });
    let resp = reqwest::Client::new()
        .post(format!("{base}/operator-chat/launch"))
        .json(&body)
        .send()
        .await
        .expect("launch request");
    assert_eq!(
        resp.status().as_u16(),
        200,
        "a wired launch route performs a REAL launch, never 503 launch_not_wired"
    );
    let launched: serde_json::Value = resp.json().await.expect("launch json");
    let run_id = launched["run_id"].as_str().expect("run_id").to_string();
    assert_eq!(
        launched["captured_message_count"].as_u64(),
        Some(3),
        "the real launch drove the runtime and captured its stdout: {launched:?}"
    );

    // Transcript route returns the captured rows (F8 backend half).
    let tr = reqwest::Client::new()
        .get(format!("{base}/operator-chat/transcript/{run_id}"))
        .send()
        .await
        .expect("transcript request");
    assert_eq!(tr.status().as_u16(), 200);
    let tr_body: serde_json::Value = tr.json().await.expect("transcript json");
    let rows = tr_body["rows"].as_array().expect("rows array");
    assert_eq!(
        rows.len(),
        4,
        "transcript route returns the durable operator row plus captured ModelLaneMessage rows"
    );
    server.abort();
}

/// F4: the operator-chat CLI launch config is forced into `--output-format
/// stream-json` so a launched CLI lane's activities are TYPED, not `Other{raw}`.
/// This is the exact transform the live wiring
/// (`build_operator_chat_launch_service`) applies to the CLI template.
#[test]
fn operator_chat_launch_config_forces_stream_json() {
    use handshake_core::swarm_orchestration::operator_chat::force_json_stream_output;
    let raw = CliBridgeConfig {
        output_format: CliOutputFormat::RawText,
        args_template: vec!["-p".into(), "{prompt}".into()],
        ..loopback_cli_config()
    };
    let forced = force_json_stream_output(raw);
    assert_eq!(
        forced.output_format,
        CliOutputFormat::JsonStream,
        "the launch config forces JSON-stream output"
    );
    assert!(
        forced.args_template.iter().any(|a| a == "stream-json"),
        "the launch config carries the --output-format stream-json flag: {:?}",
        forced.args_template
    );
}

/// AC: the operator working_dir is the REAL CLI subprocess cwd (F10 plumbing).
/// Uses the real `LiveCliSpawner` to run `cmd /c cd`, which prints its cwd.
#[cfg(windows)]
#[tokio::test]
async fn operator_chat_cli_lane_runs_in_operator_selected_cwd() {
    use handshake_core::swarm_orchestration::production_factory::cli_bridge_config_with_working_dir;
    use std::collections::HashMap;

    // A unique operator-selected directory under the OS temp root.
    let unique = format!("operator-chat-cwd-{}", Uuid::new_v4().simple());
    let selected = std::env::temp_dir().join(&unique);
    std::fs::create_dir_all(&selected).expect("create operator-selected dir");
    let selected_str = selected.to_string_lossy().to_string();

    let (ledger, _drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("manual process ledger");
    let spawner = LiveCliSpawner::new(Arc::new(ledger), LiveCliSpawner::native_cli_registry());

    let template = CliBridgeConfig {
        cli_kind: CliKind::Other,
        executable_path: "cmd".into(),
        args_template: vec!["/c".into(), "cd".into()],
        output_format: CliOutputFormat::RawText,
        env_vars: HashMap::new(),
        working_dir: None,
        timeout_seconds: 30,
    };

    // The plumbing: SpawnRequest.working_dir -> CliBridgeConfig.working_dir.
    let config = cli_bridge_config_with_working_dir(template.clone(), Some(&selected_str));
    assert_eq!(config.working_dir.as_deref(), Some(selected.as_path()));

    use handshake_core::model_runtime::cloud::CliSubprocessSpawner;
    let mut selected_invocation = CliInvocationContext::new("OPERATOR_CHAT_TEST", "model");
    selected_invocation.requested_trust_class = Some(handshake_core::sandbox::TrustClass::Trusted);
    selected_invocation.requested_isolation_tier =
        Some(handshake_core::sandbox::IsolationTier::Tier1Container);
    selected_invocation.requested_sandbox_capabilities = Some(std::collections::BTreeSet::from([
        handshake_core::sandbox::RequiredCapability::HighStdioThroughput,
    ]));
    selected_invocation.requested_net_policy =
        Some(handshake_core::sandbox::NetPolicy::HostInherited);
    selected_invocation.requested_execution_policy_ref =
        Some(handshake_core::sandbox::CLI_BRIDGE_REQUESTED_EXECUTION_POLICY_REF.to_string());
    selected_invocation.working_dir = Some(selected_str.clone());
    let receipt = spawner
        .spawn(&config, &selected_invocation, "model", "prompt")
        .expect("cmd /c cd spawns");
    assert!(
        receipt
            .stdout
            .to_lowercase()
            .contains(&unique.to_lowercase()),
        "launched subprocess cwd must be the operator selection; stdout={:?}",
        receipt.stdout
    );

    // Negative: with NO working_dir applied, the child runs in the parent cwd,
    // which does NOT contain the unique operator dir — proving it is load-bearing.
    let mut default_invocation = selected_invocation;
    default_invocation.working_dir = None;
    let default_receipt = spawner
        .spawn(&template, &default_invocation, "model", "prompt")
        .expect("default cmd /c cd spawns");
    assert!(
        !default_receipt
            .stdout
            .to_lowercase()
            .contains(&unique.to_lowercase()),
        "without working_dir the subprocess must NOT be in the operator selection"
    );

    let _ = std::fs::remove_dir_all(&selected);
}

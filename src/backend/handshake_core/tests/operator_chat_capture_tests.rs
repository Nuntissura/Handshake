//! WP-1 MT-012 operator chat/launch proof.
//!
//! Real runtime, no scaffolding: these tests drive the sanctioned launch
//! authority (`SwarmCoordinator::spawn_session`), the real CLI-capture parser
//! (`parse_agent_activity_line`), and the real `ModelLaneStore::record_message`
//! against Handshake-managed PostgreSQL/EventLedger. The Flight Recorder is a
//! capturing double so FR-EVT-AGENT-* / FR-EVT-MODEL-SELECTION-RECORDED events
//! are inspectable.

mod knowledge_pg_support;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use futures::stream;
use serde_json::json;
use uuid::Uuid;

use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::model_runtime::catalog::ModelCatalog;
use handshake_core::model_runtime::cloud::{
    CliBridgeConfig, CliKind, CliOutputFormat, LiveCliSpawner,
};
use handshake_core::model_runtime::registry::RuntimeBinding as RuntimeAdapterBinding;
use handshake_core::model_runtime::{
    CancellationToken, Embedding, GenerateRequest, GeneratedToken, KvCacheHandle, KvCachePolicy,
    LoadSpec, LoraStackHandle, ModelCapabilities, ModelId, ModelRuntime, ModelRuntimeError,
    ProviderKind, RuntimeKind, SamplingParams, Score, SteeringHookHandle, TokenStream,
};
use handshake_core::process_ledger::{
    LedgerBatcher, LedgerBatcherConfig, NoopOverflowSink, ProcessEngineKind,
    ProcessOwnershipRecordId, ProcessStart,
};
use handshake_core::swarm_orchestration::model_lane::{
    DexterityLaunchAdapterKind, DexterityLaunchAdapterRequest, ModelLaneMessageKind,
    ModelLaneStatus, ModelLaneStore,
};
use handshake_core::swarm_orchestration::operator_chat::{
    build_spawn_request, ModelLaneCaptureRecorder, OperatorChatLaneKind, OperatorChatLaunchService,
    OperatorChatSelection,
};
use handshake_core::swarm_orchestration::{
    LiveSession, ModelSessionFactory, RecordingSwarmSink, RunBudget, SessionTeardown, SpawnRequest,
    SwarmConfig, SwarmCoordinator, SwarmError,
};

// ---------------------------------------------------------------------------
// Capturing Flight Recorder double (inspectable).
// ---------------------------------------------------------------------------

#[derive(Default)]
struct CapturingRecorder {
    events: Mutex<Vec<FlightRecorderEvent>>,
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

        let mut owned = ProofRuntime::new(self.loads.clone(), self.unloads.clone());
        let model_id = owned
            .load(load_spec(request))
            .await
            .map_err(|err| SwarmError::FactoryFailed(err.to_string()))?;
        let shared = ProofRuntime::new(self.loads.clone(), self.unloads.clone());
        let teardown: SessionTeardown = Box::new(move || {
            Box::pin(async move {
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

fn store_backed_coordinator(store: ModelLaneStore) -> (Arc<SwarmCoordinator>, Arc<AtomicUsize>) {
    let (ledger, _drain) =
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
    (Arc::new(coordinator), loads)
}

fn cli_selection(working_dir: &str, prompt: &str, owner: &str) -> OperatorChatSelection {
    OperatorChatSelection {
        lane_kind: OperatorChatLaneKind::Cli,
        model_id: "claude-sonnet-4".into(),
        cloud_provider: None,
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

fn no_os_operator_launch_request(
    idx: usize,
    owner_session: &str,
) -> DexterityLaunchAdapterRequest {
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

/// AC: launch resolves through `SwarmCoordinator::spawn_session` and persists
/// ModelLaneRun + ModelLane; the CLI-capture becomes ONE ModelLaneMessage per
/// completed activity block with the correct kind + activity_kind + FR evidence.
#[tokio::test]
async fn operator_chat_launch_and_capture_persist_one_message_per_completed_block_with_fr() {
    let (pool, store) = pg_store().await;
    let (coordinator, _loads) = store_backed_coordinator(store.clone());
    let recorder = Arc::new(CapturingRecorder::default());
    let service = OperatorChatLaunchService::new(
        coordinator,
        ModelCatalog::empty(),
        recorder.clone(),
    );

    // Launch through spawn_session (real persistence).
    let launched = service
        .launch(&cli_selection("D:/work/repo", "audit the repo", "operator-1"))
        .await
        .expect("operator-chat CLI lane launches through spawn_session");

    // The run + lane are persisted and replayable.
    let replay = store
        .replay_run(&launched.run_id)
        .await
        .expect("launched run is replayable");
    assert_eq!(replay.lanes.len(), 1, "one CLI lane persisted");
    let run = replay.run;
    let lane = replay.lanes.into_iter().next().unwrap();

    // Capture the realistic multi-line codex stream.
    let capture = ModelLaneCaptureRecorder::new(store.clone(), recorder.clone());
    let messages = capture
        .capture_cli_stream(
            &run,
            &lane,
            ModelId::new_v7(),
            Uuid::new_v4(),
            CliKind::CodexCli,
            0,
            codex_stream_lines(),
        )
        .await
        .expect("codex stream capture records messages");

    // F5: exactly ONE message per completed block; the 3 item.updated deltas +
    // 2 envelopes emit nothing.
    assert_eq!(
        messages.len(),
        3,
        "one ModelLaneMessage per completed activity block, not per delta line"
    );

    // F1 message-kind mapping + activity_kind discriminator.
    assert_eq!(messages[0].kind, ModelLaneMessageKind::Status);
    assert_eq!(
        messages[0].diagnostic_payload["activity_kind"],
        json!("thinking"),
        "exposed thought is a labelled Status, never an unlabelled one"
    );
    assert_eq!(messages[1].kind, ModelLaneMessageKind::ToolRequest);
    assert_eq!(
        messages[1].diagnostic_payload["activity_kind"],
        json!("tool_call")
    );
    assert!(
        !messages[1].tool_gate_decision_refs.is_empty(),
        "tool messages carry a tool-gate decision ref"
    );
    assert_eq!(messages[2].kind, ModelLaneMessageKind::Status);
    assert_eq!(
        messages[2].diagnostic_payload["activity_kind"],
        json!("text")
    );

    // Persisted under the run and replayable.
    let after = store
        .replay_run(&launched.run_id)
        .await
        .expect("run replays after capture");
    assert_eq!(after.messages.len(), 3, "messages persisted under the run");

    // Flight Recorder evidence: one FR-EVT-AGENT-* per captured activity.
    let markers = recorder.event_markers();
    let agent_events = markers
        .iter()
        .filter(|m| m.starts_with("FR-EVT-AGENT-"))
        .count();
    assert_eq!(agent_events, 3, "one FR-EVT-AGENT-* per captured activity");

    // EventLedger authority rows exist for the messages.
    let message_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger \
         WHERE session_run_id = $1 AND aggregate_type = 'model_lane_message'",
    )
    .bind(&run.event_ledger_stream_id)
    .fetch_one(&pool)
    .await
    .expect("count model_lane_message EventLedger rows");
    assert_eq!(message_rows, 3, "each captured message appends EventLedger authority");
}

/// AC: the operator turn is persisted as a HUMAN_OPERATOR ModelLane message.
#[tokio::test]
async fn operator_chat_persists_operator_prompt_as_human_operator_message() {
    let (_pool, store) = pg_store().await;
    let (coordinator, _loads) = store_backed_coordinator(store.clone());
    let recorder = Arc::new(CapturingRecorder::default());

    // Spawn the CLI authority session so it can authorize the no-OS operator lane.
    let cli_request = build_spawn_request(
        &cli_selection("D:/work/repo", "hello", "operator-77"),
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
        .launch(&cli_selection("D:/work/repo", "hi", "operator-9"))
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
    let (coordinator, _loads) = store_backed_coordinator(store);
    let recorder = Arc::new(CapturingRecorder::default());
    let service =
        OperatorChatLaunchService::new(coordinator, ModelCatalog::empty(), recorder.clone());

    service
        .record_selection("claude-sonnet-4", "operator", "operator picked the CLI lane")
        .await
        .expect("selection decision records");

    let markers = recorder.event_markers();
    assert!(
        markers.iter().any(|m| m == "FR-EVT-MODEL-SELECTION-RECORDED"),
        "selection emits the auditable FR-EVT-MODEL-SELECTION-RECORDED event; got {markers:?}"
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
    let spawner = LiveCliSpawner::new(Arc::new(ledger));

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
    let receipt = spawner
        .spawn(&config, "model", "prompt")
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
    let default_receipt = spawner
        .spawn(&template, "model", "prompt")
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

//! WP-1 MT-012 operator chat: fail-closed launch, stream JSON, and real embedded-Surreal CLI cwd proof.

mod process_ledger_surreal_support;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::model_runtime::catalog::ModelCatalog;
use handshake_core::model_runtime::cloud::{
    CliBridgeConfig, CliInvocationContext, CliKind, CliOutputFormat, CliSubprocessSpawner,
    LiveCliSpawner,
};
use handshake_core::process_ledger::{
    drain_and_join_ledger_writer, LedgerBatcher, LedgerBatcherConfig, LedgerDrainJoinOutcome,
    LedgerEvent, NoopOverflowSink, ProcessLedgerError, ProcessLedgerStore, ReclaimResourceScope,
};
use handshake_core::swarm_orchestration::operator_chat::{
    OperatorChatLaneKind, OperatorChatLaunchService, OperatorChatSelection,
};
use handshake_core::swarm_orchestration::{
    LiveSession, ModelSessionFactory, RecordingSwarmSink, RunBudget, SpawnRequest, SwarmConfig,
    SwarmCoordinator, SwarmError,
};
use process_ledger_surreal_support::{scope_metadata, ProcessLedgerSurrealHarness};

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

fn operator_chat_process_scope() -> ReclaimResourceScope {
    ReclaimResourceScope {
        account_uuid: Uuid::from_u128(0x11),
        actor_uuid: Uuid::from_u128(0x12),
        session_uuid: Uuid::from_u128(0x13),
        workspace_id: "operator-chat-process-ledger-tests".to_string(),
        access_space_uuid: Uuid::from_u128(0x14),
    }
}

fn operator_chat_process_metadata() -> Value {
    scope_metadata(&operator_chat_process_scope())
}

struct ExplicitTestScopeStore {
    inner: Arc<dyn ProcessLedgerStore>,
}

#[async_trait]
impl ProcessLedgerStore for ExplicitTestScopeStore {
    async fn write_batch(&self, mut events: Vec<LedgerEvent>) -> Result<(), ProcessLedgerError> {
        let exact = operator_chat_process_metadata();
        let exact = exact.as_object().expect("exact test scope is an object");
        for event in &mut events {
            let metadata = match event {
                LedgerEvent::Start(start) => &mut start.metadata_jsonb,
                LedgerEvent::Stop(stop) => &mut stop.metadata_jsonb,
            };
            let object = metadata.as_object_mut().ok_or_else(|| {
                ProcessLedgerError::InvalidConfig(
                    "CLI test lifecycle metadata must be an object".to_string(),
                )
            })?;
            object.extend(exact.clone());
        }
        self.inner.write_batch(events).await
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
// Multi-threaded runtime is REQUIRED, not cosmetic. `LiveCliSpawner::spawn` is a
// blocking call, and it will not return until the process ledger durably
// acknowledges the START row. On the default current-thread runtime the blocking
// spawn owns the only worker, so the spawned ledger writer task can never be
// polled, the ack never lands, and the spawner correctly terminates and reaps the
// child with PROCESS_LEDGER_DURABILITY_ACK_TIMEOUT. The sibling launch proofs use
// the same flavor for the same reason.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn operator_chat_cli_lane_runs_in_operator_selected_cwd() {
    use handshake_core::swarm_orchestration::production_factory::cli_bridge_config_with_working_dir;
    use std::collections::HashMap;

    // A unique operator-selected directory under the OS temp root.
    let unique = format!("operator-chat-cwd-{}", Uuid::new_v4().simple());
    let selected = std::env::temp_dir().join(&unique);
    std::fs::create_dir_all(&selected).expect("create operator-selected dir");
    let selected_str = selected.to_string_lossy().to_string();

    // A real driven embedded-Surreal ledger, not a manual batcher. This focused
    // cwd fixture supplies its explicit test scope at the store boundary because
    // CliInvocationContext does not yet carry the five production scope fields.
    //
    // `LiveCliSpawner` is the production spawner: it refuses to leave a child
    // running unless the ledger durably acknowledges the START row, and
    // terminates and reaps the child if that ack does not land
    // (PROCESS_LEDGER_DURABILITY_ACK_TIMEOUT). That is the HBR-QUIET-003
    // process-ownership guarantee, so it must not be dodged. A
    // `manual_for_tests` batcher is never driven, so the ack can never arrive
    // and the child is always killed before it can report its cwd.
    let process_ledger =
        ProcessLedgerSurrealHarness::open_with_scope(operator_chat_process_scope()).await;
    let ledger_store: Arc<dyn ProcessLedgerStore> = Arc::new(ExplicitTestScopeStore {
        inner: process_ledger.store(),
    });
    let (ledger, ledger_writer) = LedgerBatcher::spawn(
        ledger_store,
        Arc::new(NoopOverflowSink),
        LedgerBatcherConfig::default(),
    );
    let spawner = LiveCliSpawner::new(
        Arc::new(ledger.clone()),
        LiveCliSpawner::native_cli_registry(),
    );

    // A real, non-interpreter executable that reports its own cwd.
    //
    // This previously spawned `cmd /c cd`, which can never succeed: the product
    // denylists generic command interpreters as CLI entrypoints
    // (`reject_command_interpreter` in model_runtime/cloud/official_cli_bridge.rs
    // lists `cmd` / `cmd.exe` among others, matched on file name, so PATH
    // resolution does not get past it either). Making the old form pass would
    // have required deleting `cmd` from that denylist — removing a security
    // control to satisfy a fixture. The behaviour under test (an operator-selected
    // working directory really becomes the spawned child's cwd) is legitimate, so
    // it is proven here against `cwd-probe`, a genuine spawned process.
    let template = CliBridgeConfig {
        cli_kind: CliKind::Other,
        executable_path: env!("CARGO_BIN_EXE_cwd-probe").into(),
        args_template: Vec::new(),
        output_format: CliOutputFormat::RawText,
        env_vars: HashMap::new(),
        working_dir: None,
        timeout_seconds: 30,
    };

    // The plumbing: SpawnRequest.working_dir -> CliBridgeConfig.working_dir.
    let config = cli_bridge_config_with_working_dir(template.clone(), Some(&selected_str));
    assert_eq!(config.working_dir.as_deref(), Some(selected.as_path()));

    use handshake_core::model_runtime::cloud::CliSubprocessSpawner;

    // Pin the executable graph exactly as production does. `register_bridge`
    // pins the config when a CLI bridge is registered, and `LiveCliSpawner`
    // revalidates that same pinned identity immediately before every launch, so
    // an unpinned entrypoint is refused. This test hand-builds its config
    // instead of going through `register_bridge`, so it must pin explicitly —
    // that keeps the identity control in force rather than routing around it.
    spawner
        .pin_config(&config)
        .expect("cwd-probe executable graph pins for the operator-selected launch");

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
        .expect("cwd-probe spawns");
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
        .expect("default cwd-probe spawns");
    assert!(
        !default_receipt
            .stdout
            .to_lowercase()
            .contains(&unique.to_lowercase()),
        "without working_dir the subprocess must NOT be in the operator selection"
    );

    // Drain and join the ledger writer so both spawns' START/STOP rows are
    // durably settled before the test ends. Leaving the writer task running
    // would leak it into the rest of the suite and hide unacknowledged rows.
    let ledger_outcome =
        drain_and_join_ledger_writer(&ledger, ledger_writer, Duration::from_secs(10)).await;
    assert!(
        matches!(ledger_outcome, LedgerDrainJoinOutcome::Flushed),
        "the production process-ledger writer must flush both spawns' START/STOP rows; got {ledger_outcome:?}"
    );
    assert_eq!(process_ledger.lifecycle_count().await, 2);
    assert_eq!(process_ledger.open_lifecycle_count().await, 0);
    process_ledger.close().await;

    let _ = std::fs::remove_dir_all(&selected);
}


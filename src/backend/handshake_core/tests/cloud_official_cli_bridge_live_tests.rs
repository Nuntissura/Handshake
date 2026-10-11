//! MT-003 LIVE integration test for the Official CLI bridge runtime.
//!
//! Spec-Realism Gate Sub-rule 2 ("real external-resource touch") is
//! satisfied by spawning the actually-installed Codex CLI on the host,
//! capturing real stdout, asserting a real
//! PID and a real exit code. The test resolves the binary path at
//! run time using the OS PATH probe (`where` on Windows, `which`
//! elsewhere) so it is portable across kernel-builder hosts. If Codex
//! is not installed, the test fails with a
//! clear environmental error — that is the honest BLOCKED_ON_DEPENDENCY
//! signal the brief asks for, not silent env-gated skipping.
//!
//! The probe uses the `--version` invocation, which exits cleanly,
//! does not hit any vendor API, and produces deterministic stdout
//! containing the CLI name. This pins the live subprocess path
//! without consuming operator API credits.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use handshake_core::model_runtime::cloud::{
    CliBridgeConfig, CliInvocationContext, CliKind, CliOutputFormat, LiveCliSpawner,
    OfficialCliBridgeRuntime,
};
use handshake_core::process_ledger::{
    LedgerBatcher, LedgerBatcherConfig, LedgerEvent, NoopOverflowSink, ProcessLedgerDrain,
    ProcessLedgerError, ProcessLedgerStore,
};

#[derive(Clone, Default)]
struct CapturingLedgerStore {
    events: Arc<Mutex<Vec<LedgerEvent>>>,
}

#[async_trait::async_trait]
impl ProcessLedgerStore for CapturingLedgerStore {
    async fn write_batch(&self, events: Vec<LedgerEvent>) -> Result<(), ProcessLedgerError> {
        self.events
            .lock()
            .expect("ledger store lock")
            .extend(events);
        Ok(())
    }
}

/// Build a real, manually-drained `LedgerBatcher` for the live tests. The
/// ledger is mandatory on `LiveCliSpawner`; these
/// live proof drains and asserts the real child's lifecycle rows.
fn test_ledger() -> (Arc<LedgerBatcher>, ProcessLedgerDrain) {
    let (batcher, drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("manual ledger batcher for live tests");
    (Arc::new(batcher), drain)
}

fn invocation(model: &str) -> CliInvocationContext {
    let mut context = CliInvocationContext::new("LIVE_TEST", model);
    context.owner_wp = Some("WP-MULTI-MODEL-ORCHESTRATION-V1".to_string());
    context.role_id = Some("LIVE_TEST".to_string());
    context.wp_id = context.owner_wp.clone();
    context.mt_id = Some("MT-003".to_string());
    context.session_id = Some("live-cli-test".to_string());
    context.parent_session_id = Some("live-cli-parent".to_string());
    context.trace_id = Some("live-cli-trace".to_string());
    context.span_id = Some("live-cli-span".to_string());
    context.cancellation_id = Some("live-cli-cancel".to_string());
    context.reclaim_key = Some("live-cli-test-reclaim".to_string());
    context.requested_trust_class = Some(handshake_core::sandbox::TrustClass::Trusted);
    context.requested_isolation_tier = Some(handshake_core::sandbox::IsolationTier::Tier1Container);
    context.requested_sandbox_capabilities = Some(std::collections::BTreeSet::from([
        handshake_core::sandbox::RequiredCapability::HighStdioThroughput,
    ]));
    context.requested_net_policy = Some(handshake_core::sandbox::NetPolicy::HostInherited);
    context.requested_execution_policy_ref =
        Some(handshake_core::sandbox::CLI_BRIDGE_REQUESTED_EXECUTION_POLICY_REF.to_string());
    context.swarm_id = Some("live-cli-swarm".to_string());
    context.worktree_id = Some("live-cli-worktree".to_string());
    context
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn live_cli_spawner_spawns_real_binary_and_returns_real_pid_and_stdout() {
    let (mut config, _allowlist) = handshake_core::api::resolve_official_cli_config_from_path(
        handshake_core::model_runtime::cloud::CliBridgeProvider::Codex,
    )
    .expect("installed Codex must resolve through the production validated PATH/config resolver")
    .into_parts();
    let exe = config.executable_path.clone();
    // This is deliberately a generic native-process lifecycle probe, not a
    // Codex inference request. Keep production Codex lanes pinned to the
    // validated `exec --json --model {model} {prompt}` preset while invoking
    // the resolved executable's local-only `--version` command here.
    config.cli_kind = CliKind::Other;
    config.args_template = vec![
        "--model".to_string(),
        "{model}".to_string(),
        "{prompt}".to_string(),
    ];
    config.output_format = CliOutputFormat::RawText;
    config.timeout_seconds = 45;
    let live_working_dir = config
        .working_dir
        .as_ref()
        .map(|path| path.to_string_lossy().to_string());
    let name = "codex";
    let mut failures = Vec::new();
    let store = CapturingLedgerStore::default();
    let (ledger, ledger_writer) = LedgerBatcher::spawn(
        Arc::new(store.clone()),
        Arc::new(NoopOverflowSink),
        LedgerBatcherConfig::default(),
    );
    let ledger = Arc::new(ledger);
    let runtime = OfficialCliBridgeRuntime::new(Arc::new(LiveCliSpawner::new(
        ledger.clone(),
        LiveCliSpawner::native_cli_registry(),
    )));
    let handle = runtime
        .register_bridge(config, "version-probe-model", "2026-05-20T18:10:00Z")
        .expect("production-resolved Codex config must register");
    let mut live_invocation = invocation(name);
    live_invocation.working_dir = live_working_dir;

    let mut successful_receipt = None;
    for attempt in 1..=2 {
        match runtime.invoke(handle.model_id, "--version", &live_invocation) {
            Ok(receipt) => {
                assert_live_cli_receipt(&receipt, name);
                successful_receipt = Some(receipt);
                break;
            }
            Err(err) => {
                failures.push(format!(
                    "{} at {} attempt {attempt} failed: {err:?}",
                    name,
                    exe.display()
                ));
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
        }
    }

    let receipt = successful_receipt.unwrap_or_else(|| {
        panic!(
            "MT-003 live Codex proof: installed Codex candidates were found, \
             but none completed the live Codex --version probe:\n{}",
            failures.join("\n")
        )
    });
    let pid = receipt.pid.expect("successful live Codex receipt has PID");
    ledger.begin_close();
    ledger_writer
        .await
        .expect("join real Codex ledger writer")
        .expect("flush real Codex child ledger");
    let events = store.events.lock().expect("ledger store lock").clone();

    let matching: Vec<(usize, &LedgerEvent)> = events
        .iter()
        .enumerate()
        .filter(|(_, event)| match event {
            LedgerEvent::Start(row) => row.os_pid == Some(pid),
            LedgerEvent::Stop(row) => row.os_pid == Some(pid),
        })
        .collect();
    assert_eq!(
        matching.len(),
        2,
        "actual Codex PID must have exactly one START and one STOP: {events:?}"
    );
    let (start_index, LedgerEvent::Start(start)) = matching[0] else {
        panic!("actual Codex PID lifecycle must begin with START: {matching:?}");
    };
    let (stop_index, LedgerEvent::Stop(stop)) = matching[1] else {
        panic!("actual Codex PID lifecycle must end with STOP: {matching:?}");
    };
    assert!(start_index < stop_index, "Codex START must precede STOP");
    assert_eq!(stop.process_uuid, start.process_uuid);
    assert_eq!(start.owner_role, "LIVE_TEST");
    assert_eq!(
        start.owner_wp.as_deref(),
        Some("WP-MULTI-MODEL-ORCHESTRATION-V1")
    );
    assert_eq!(
        start.wp_id.as_deref(),
        Some("WP-MULTI-MODEL-ORCHESTRATION-V1")
    );
    assert_eq!(start.mt_id.as_deref(), Some("MT-003"));
    assert_eq!(
        start.metadata_jsonb["selected_model_name"],
        "version-probe-model"
    );
    assert_eq!(start.metadata_jsonb["requested_model_identity"], "codex");
    assert_eq!(start.metadata_jsonb["session_id"], "live-cli-test");
    assert_eq!(start.metadata_jsonb["trace_id"], "live-cli-trace");
    assert_eq!(start.metadata_jsonb["requested_trust_class"], "trusted");
    assert_eq!(
        start.metadata_jsonb["requested_isolation_tier"],
        "tier1_container"
    );
    assert_eq!(
        start.metadata_jsonb["requested_net_policy"],
        "host_inherited"
    );
    assert_eq!(
        start.metadata_jsonb["requested_execution_policy_ref"],
        handshake_core::sandbox::CLI_BRIDGE_REQUESTED_EXECUTION_POLICY_REF
    );
    assert_eq!(start.metadata_jsonb["swarm_id"], "live-cli-swarm");
    assert_eq!(start.metadata_jsonb["worktree_id"], "live-cli-worktree");
    assert_eq!(start.metadata_jsonb["requested_swarm_id"], "live-cli-swarm");
    assert_eq!(
        start.metadata_jsonb["requested_worktree_id"],
        "live-cli-worktree"
    );
    assert_eq!(start.metadata_jsonb["effective_trust_class"], "trusted");
    assert_eq!(
        start.metadata_jsonb["effective_isolation_tier"],
        "tier1_container"
    );
    assert_eq!(
        start.metadata_jsonb["effective_net_policy"],
        "outbound_internet_client"
    );
    assert_eq!(start.metadata_jsonb["effective_swarm_id"], "live-cli-swarm");
    assert_eq!(
        start.metadata_jsonb["effective_worktree_id"],
        "live-cli-worktree"
    );
    assert_eq!(stop.owner_role, "LIVE_TEST");
    assert_eq!(
        stop.owner_wp.as_deref(),
        Some("WP-MULTI-MODEL-ORCHESTRATION-V1")
    );
    assert_eq!(
        stop.wp_id.as_deref(),
        Some("WP-MULTI-MODEL-ORCHESTRATION-V1")
    );
    assert_eq!(stop.mt_id.as_deref(), Some("MT-003"));
    assert_eq!(stop.exit_code, Some(0));

    for event in events
        .iter()
        .filter(|event| matches!(event, LedgerEvent::Start(_)))
    {
        let LedgerEvent::Start(start) = event else {
            unreachable!()
        };
        assert!(
            events.iter().any(|candidate| matches!(
                candidate,
                LedgerEvent::Stop(stop) if stop.process_uuid == start.process_uuid
            )),
            "live proof must leave no open child lifecycle: {start:?}"
        );
    }
}

/// Current-source, authenticated real-model proof. Unlike the deterministic
/// `--version` probe above, this drives the production Codex preset all the way
/// through an actual model turn and therefore remains ignored in ordinary test
/// runs. The exact proof command deliberately fails when the installed CLI is
/// unauthenticated or its production allowlisted model is unavailable; it never
/// substitutes a fixture or silently skips.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires an authenticated installed Codex CLI and live model access"]
async fn live_cli_spawner_runs_authenticated_real_codex_model_turn() {
    const MODEL: &str = "gpt-5-codex";
    const MARKER: &str = "HSK_WP1_REAL_CODEX_OK";

    let allowlisted = handshake_core::api::resolve_official_cli_config_from_path(
        handshake_core::model_runtime::cloud::CliBridgeProvider::Codex,
    )
    .expect("installed Codex must resolve through the production validated PATH/config resolver");
    let (mut config, allowlist) = allowlisted.into_parts();
    assert!(
        allowlist.contains(MODEL),
        "the real-model proof must use the production allowlisted Codex model"
    );
    config.timeout_seconds = 180;
    let live_working_dir = config
        .working_dir
        .as_ref()
        .map(|path| path.to_string_lossy().to_string());

    let store = CapturingLedgerStore::default();
    let (ledger, ledger_writer) = LedgerBatcher::spawn(
        Arc::new(store.clone()),
        Arc::new(NoopOverflowSink),
        LedgerBatcherConfig::default(),
    );
    let ledger = Arc::new(ledger);
    let runtime = OfficialCliBridgeRuntime::new(Arc::new(LiveCliSpawner::new(
        ledger.clone(),
        LiveCliSpawner::native_cli_registry(),
    )));
    let handle = runtime
        .register_bridge(config, MODEL, "2026-07-22T00:00:00Z")
        .expect("production Codex preset and executable graph must register");
    let mut context = invocation("codex");
    context.working_dir = live_working_dir;

    let receipt = runtime
        .invoke(
            handle.model_id,
            &format!("Reply with exactly {MARKER} and no other text."),
            &context,
        )
        .expect("authenticated production Codex model turn must complete");
    assert_eq!(receipt.exit_code, Some(0));
    assert!(!receipt.cancelled);
    let pid = receipt
        .pid
        .expect("real Codex model turn carries an OS PID");
    assert!(pid > 0);
    assert!(
        receipt.stdout.contains(MARKER),
        "real Codex output must contain the nonce marker; stdout={:?}",
        receipt.stdout
    );

    ledger.begin_close();
    ledger_writer
        .await
        .expect("join real-model Codex ledger writer")
        .expect("flush real-model Codex lifecycle");
    let events = store.events.lock().expect("ledger store lock").clone();
    let matching = events
        .iter()
        .filter(|event| match event {
            LedgerEvent::Start(row) => row.os_pid == Some(pid),
            LedgerEvent::Stop(row) => row.os_pid == Some(pid),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        matching.len(),
        2,
        "real model process must have exactly one START and one STOP: {events:?}"
    );
    let LedgerEvent::Start(start) = matching[0] else {
        panic!("real model lifecycle must begin with START: {matching:?}");
    };
    let LedgerEvent::Stop(stop) = matching[1] else {
        panic!("real model lifecycle must end with STOP: {matching:?}");
    };
    assert_eq!(start.process_uuid, stop.process_uuid);
    assert_eq!(start.metadata_jsonb["selected_model_name"], MODEL);
    assert_eq!(
        start.metadata_jsonb["execution_policy_resolution"]["effective_ref"],
        handshake_core::sandbox::CLI_BRIDGE_EFFECTIVE_EXECUTION_POLICY_REF
    );
}

fn assert_live_cli_receipt(
    receipt: &handshake_core::model_runtime::cloud::CliInvocationReceipt,
    name: &str,
) {
    // PID is populated by the production spawner (mock spawners set
    // it to None or a literal sentinel). A real Some(pid) here proves
    // std::process::Command actually launched a child process.
    let pid = receipt
        .pid
        .unwrap_or_else(|| panic!("LiveCliSpawner did not record a PID for binary {name}"));
    assert!(pid > 0, "PID must be > 0 for a real subprocess");

    // Exit code 0 for `--version`.
    assert_eq!(
        receipt.exit_code,
        Some(0),
        "{} --version should exit cleanly; got exit_code={:?}",
        name,
        receipt.exit_code
    );

    // Cancellation flag must be false for a clean run.
    assert!(
        !receipt.cancelled,
        "clean --version run must not be cancelled"
    );

    // Stdout must contain the CLI name in some form. Each of the
    // three target CLIs prints a banner that references its name.
    let stdout_lower = receipt.stdout.to_lowercase();
    assert!(
        stdout_lower.contains(name) || stdout_lower.contains("code"),
        "LiveCliSpawner stdout for {} --version must include CLI name; got {:?}",
        name,
        receipt.stdout
    );
    assert!(
        !receipt.stdout.trim().is_empty(),
        "LiveCliSpawner must capture non-empty stdout from real subprocess"
    );
}

#[test]
fn live_cli_spawner_surfaces_spawn_failure_for_missing_binary() {
    // Real-resource negative-path proof: when the configured
    // executable does not exist on the host, LiveCliSpawner must
    // return SpawnFailed (no silent fallback to mock-shaped success).
    // The register_bridge guard catches non-existent paths via
    // ExecutableNotFound; this test exercises the path where the
    // executable existed at register time but is unspawnable.
    use std::fs;
    use tempfile::tempdir;

    let dir = tempdir().expect("tempdir");
    let bogus = dir.path().join("not-a-real-binary.exe");
    // Create a zero-byte file so register_bridge.executable_path
    // existence check passes, but spawn() will fail because the
    // file is not an executable image.
    fs::write(&bogus, b"").expect("create bogus exe");

    let config = CliBridgeConfig {
        cli_kind: CliKind::Other,
        executable_path: bogus,
        args_template: vec![
            "--model".to_string(),
            "{model}".to_string(),
            "{prompt}".to_string(),
        ],
        output_format: CliOutputFormat::RawText,
        env_vars: HashMap::new(),
        working_dir: None,
        timeout_seconds: 5,
    };

    let (ledger, _ledger_drain) = test_ledger();
    let runtime = OfficialCliBridgeRuntime::new(Arc::new(LiveCliSpawner::new(
        ledger,
        LiveCliSpawner::native_cli_registry(),
    )));
    let handle = runtime
        .register_bridge(config, "bogus-model", "2026-05-20T18:10:00Z")
        .expect("register_bridge passes because the file exists, even though it is not executable");

    let err = runtime
        .invoke(handle.model_id, "anything", &invocation("bogus-model"))
        .expect_err("LiveCliSpawner.invoke must surface a real spawn failure");

    use handshake_core::model_runtime::cloud::OfficialCliBridgeError;
    assert!(
        matches!(err, OfficialCliBridgeError::SpawnFailed { .. }),
        "expected SpawnFailed, got {err:?}"
    );
}

//! MT-127 cross-crate integration smoke for the Official CLI bridge
//! runtime scaffold. Exhaustive coverage lives in the inline tests
//! in `model_runtime::cloud::official_cli_bridge::tests`; this file
//! pins the cross-crate API surface + the red_team minimum_controls.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use handshake_core::model_runtime::cloud::{
    CliBridgeConfig, CliInvocationReceipt, CliKind, CliOutputFormat, CliSubprocessSpawner,
    LiveCliSpawner, OfficialCliBridgeError, OfficialCliBridgeRuntime,
};
use handshake_core::model_runtime::ModelId;
use handshake_core::process_ledger::{
    LedgerBatcher, LedgerBatcherConfig, LedgerEvent, NoopOverflowSink, ProcessEngineKind,
    ProcessLedgerError, ProcessLedgerStore,
};

/// Build a real, manually-drained `LedgerBatcher` for tests that construct a
/// `LiveCliSpawner` but do not inspect the recorded rows. The ledger is
/// mandatory on the spawner (MT-127, MT-122-class).
fn test_ledger() -> Arc<LedgerBatcher> {
    let (batcher, _drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("manual ledger batcher for tests");
    Arc::new(batcher)
}

struct EchoSpawner {
    cancel_reported: Mutex<bool>,
}
impl CliSubprocessSpawner for EchoSpawner {
    fn spawn(
        &self,
        _config: &CliBridgeConfig,
        model_name: &str,
        prompt: &str,
    ) -> Result<CliInvocationReceipt, OfficialCliBridgeError> {
        Ok(CliInvocationReceipt {
            model_id: ModelId::new_v7(),
            stdout: format!("echo model={model_name} prompt={prompt}"),
            pid: Some(42),
            exit_code: Some(0),
            cancelled: *self.cancel_reported.lock().unwrap(),
        })
    }
}

fn fixture_config() -> CliBridgeConfig {
    CliBridgeConfig {
        cli_kind: CliKind::ClaudeCode,
        executable_path: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
        args_template: vec!["--prompt".to_string(), "{prompt}".to_string()],
        output_format: CliOutputFormat::RawText,
        env_vars: HashMap::new(),
        working_dir: None,
        timeout_seconds: 30,
    }
}

#[test]
fn cli_bridge_capabilities_are_all_false() {
    // MT-127 red_team minimum_controls[1]: no false advertising on
    // the CLI bridge. None of the inference techniques work through
    // a CLI subprocess.
    let caps = OfficialCliBridgeRuntime::cli_bridge_capabilities();
    assert!(!caps.supports_lora);
    assert!(!caps.supports_kv_prefix_cache);
    assert!(!caps.supports_activation_steering);
    assert!(!caps.supports_subquadratic);
    assert!(!caps.supports_speculative_draft);
    assert!(!caps.supports_eagle3);
}

#[test]
fn cli_bridge_invoke_routes_through_spawner() {
    let spawner = Arc::new(EchoSpawner {
        cancel_reported: Mutex::new(false),
    });
    let runtime = OfficialCliBridgeRuntime::new(spawner);
    let handle = runtime
        .register_bridge(
            fixture_config(),
            "claude-3.5-sonnet",
            "2026-05-20T06:30:00Z",
        )
        .expect("register");
    let receipt = runtime
        .invoke(handle.model_id, "hello world")
        .expect("invoke");
    assert!(receipt.stdout.contains("claude-3.5-sonnet"));
    assert!(receipt.stdout.contains("hello world"));
    assert_eq!(receipt.exit_code, Some(0));
}

#[test]
fn cli_bridge_register_validates_placeholders_and_timeout() {
    let spawner = Arc::new(EchoSpawner {
        cancel_reported: Mutex::new(false),
    });
    let runtime = OfficialCliBridgeRuntime::new(spawner);
    let mut bad = fixture_config();
    bad.args_template = vec!["no-placeholder".to_string()];
    let err = runtime
        .register_bridge(bad, "claude-3.5-sonnet", "2026-05-20T06:30:00Z")
        .expect_err("missing placeholder");
    assert!(matches!(
        err,
        OfficialCliBridgeError::MissingPromptPlaceholder
    ));

    let mut bad = fixture_config();
    bad.timeout_seconds = 0;
    let err = runtime
        .register_bridge(bad, "claude-3.5-sonnet", "2026-05-20T06:30:00Z")
        .expect_err("zero timeout");
    assert!(matches!(err, OfficialCliBridgeError::InvalidTimeout));
}

#[test]
fn cli_bridge_render_args_substitutes_placeholders() {
    let rendered = OfficialCliBridgeRuntime::render_args(
        &vec![
            "--model".to_string(),
            "{model}".to_string(),
            "--text".to_string(),
            "<<{prompt}>>".to_string(),
        ],
        "claude-3.5-sonnet",
        "hello",
    );
    assert_eq!(rendered[1], "claude-3.5-sonnet");
    assert_eq!(rendered[3], "<<hello>>");
}

#[test]
fn cli_bridge_invoke_unregistered_model_errors() {
    let spawner = Arc::new(EchoSpawner {
        cancel_reported: Mutex::new(false),
    });
    let runtime = OfficialCliBridgeRuntime::new(spawner);
    let err = runtime
        .invoke(ModelId::new_v7(), "x")
        .expect_err("unknown model");
    assert!(matches!(err, OfficialCliBridgeError::ModelNotRegistered(_)));
}

/// A trivially fast, host-native config: a command that prints + exits
/// immediately. Used to prove the ledger row is registered the moment
/// the child pid is known, without paying any timeout.
#[cfg(windows)]
fn fast_exit_config() -> CliBridgeConfig {
    CliBridgeConfig {
        cli_kind: CliKind::Other,
        executable_path: PathBuf::from(
            std::env::var("ComSpec").unwrap_or_else(|_| "C:\\Windows\\System32\\cmd.exe".to_string()),
        ),
        args_template: vec!["/C".to_string(), "echo {model}-{prompt}".to_string()],
        output_format: CliOutputFormat::RawText,
        env_vars: HashMap::new(),
        working_dir: None,
        timeout_seconds: 30,
    }
}

#[cfg(not(windows))]
fn fast_exit_config() -> CliBridgeConfig {
    CliBridgeConfig {
        cli_kind: CliKind::Other,
        executable_path: PathBuf::from("/bin/sh"),
        args_template: vec![
            "-c".to_string(),
            "printf '%s-%s\\n' '{model}' '{prompt}'".to_string(),
        ],
        output_format: CliOutputFormat::RawText,
        env_vars: HashMap::new(),
        working_dir: None,
        timeout_seconds: 30,
    }
}

/// In-memory ProcessLedgerStore that captures recorded events, mirroring
/// the established pattern in `process_ledger_tests.rs`.
#[derive(Clone, Default)]
struct CapturingLedgerStore {
    events: Arc<Mutex<Vec<LedgerEvent>>>,
}

impl CapturingLedgerStore {
    fn events(&self) -> Vec<LedgerEvent> {
        self.events.lock().unwrap().clone()
    }
}

#[async_trait]
impl ProcessLedgerStore for CapturingLedgerStore {
    async fn write_batch(&self, events: Vec<LedgerEvent>) -> Result<(), ProcessLedgerError> {
        self.events.lock().unwrap().extend(events);
        Ok(())
    }
}

/// MT-127 remediation (MT-122-class), end-to-end: the LiveCliSpawner's
/// ProcessOwnershipLedger registration is UNCONDITIONAL. The ledger is
/// mandatory at construction, so every CLI-bridge subprocess spawn records
/// an attributable START row (engine_kind=OfficialCliBridge) the moment the
/// child pid is known AND a matching STOP row after the child exits. Proves
/// the spawned CLI subprocess is always attributable + reclaimable across
/// its full lifecycle, closing the optional-ledger (MT-122-class) gap.
#[tokio::test]
async fn live_cli_spawner_records_official_cli_bridge_start_and_stop_rows() {
    let config = fast_exit_config();
    if !config.executable_path.exists() {
        eprintln!(
            "skipping ledger-row test; executable missing: {}",
            config.executable_path.display()
        );
        return;
    }

    let store = CapturingLedgerStore::default();
    let (batcher, drain) = LedgerBatcher::manual_for_tests(
        LedgerBatcherConfig::default(),
        Arc::new(NoopOverflowSink),
    )
    .expect("manual ledger batcher");

    // The ledger is MANDATORY: there is no ledger-less / optional builder.
    let spawner = LiveCliSpawner::new(Arc::new(batcher));
    let receipt = spawner
        .spawn(&config, "claude-3.5-sonnet", "hello world")
        .expect("spawn + ledger registration must succeed");
    assert!(receipt.pid.is_some(), "live spawn must capture a pid");

    drain
        .drain_available_to(Arc::new(store.clone()))
        .await
        .expect("drain ledger to store");

    let events = store.events();
    assert_eq!(
        events.len(),
        2,
        "exactly one START and one matching STOP ProcessOwnershipLedger row, got {events:?}"
    );

    let LedgerEvent::Start(start) = &events[0] else {
        panic!("expected the first event to be a Start, got {:?}", events[0]);
    };
    assert_eq!(
        start.engine_kind,
        ProcessEngineKind::OfficialCliBridge,
        "START row must be attributed to engine_kind=OfficialCliBridge"
    );
    assert_eq!(start.owner_role, "OFFICIAL_CLI_BRIDGE");
    assert_eq!(start.os_pid, receipt.pid);
    assert_eq!(start.mt_id.as_deref(), Some("MT-127"));
    assert_eq!(
        start.sandbox_adapter_id.as_deref(),
        Some("official_cli_bridge")
    );
    assert_eq!(
        start.metadata_jsonb["subprocess_kind"].as_str(),
        Some("official_cli_bridge")
    );
    assert_eq!(start.metadata_jsonb["mt"].as_str(), Some("MT-127"));
    assert_eq!(
        start.metadata_jsonb["model_name"].as_str(),
        Some("claude-3.5-sonnet")
    );

    let LedgerEvent::Stop(stop) = &events[1] else {
        panic!("expected the second event to be a Stop, got {:?}", events[1]);
    };
    // The STOP row must reconcile to the SAME process: same uuid, same pid,
    // same engine_kind/owner_role, so the row is attributable + reclaimable
    // across its full lifecycle.
    assert_eq!(
        stop.process_uuid, start.process_uuid,
        "STOP must reference the same ProcessOwnership row uuid as START"
    );
    assert_eq!(stop.os_pid, receipt.pid);
    assert_eq!(stop.engine_kind, ProcessEngineKind::OfficialCliBridge);
    assert_eq!(stop.owner_role, "OFFICIAL_CLI_BRIDGE");
    assert_eq!(stop.exit_code, receipt.exit_code);
    assert_eq!(
        stop.stop_reason.as_deref(),
        Some("official_cli_bridge_exit"),
        "clean exit must record the canonical stop reason"
    );
}

/// Host-native config that echoes two inherited env vars so a test can
/// observe which ones reached the spawned child process.
#[cfg(windows)]
fn env_echo_config() -> CliBridgeConfig {
    CliBridgeConfig {
        cli_kind: CliKind::Other,
        executable_path: PathBuf::from(
            std::env::var("ComSpec").unwrap_or_else(|_| "C:\\Windows\\System32\\cmd.exe".to_string()),
        ),
        args_template: vec![
            "/C".to_string(),
            "echo SECRET=[%SCRUB_PROBE_API_KEY%] PUBLIC=[%SCRUB_PROBE_PUBLIC_DIR%]".to_string(),
        ],
        output_format: CliOutputFormat::RawText,
        env_vars: HashMap::new(),
        working_dir: None,
        timeout_seconds: 30,
    }
}

#[cfg(not(windows))]
fn env_echo_config() -> CliBridgeConfig {
    CliBridgeConfig {
        cli_kind: CliKind::Other,
        executable_path: PathBuf::from("/bin/sh"),
        args_template: vec![
            "-c".to_string(),
            "echo \"SECRET=[$SCRUB_PROBE_API_KEY] PUBLIC=[$SCRUB_PROBE_PUBLIC_DIR]\"".to_string(),
        ],
        output_format: CliOutputFormat::RawText,
        env_vars: HashMap::new(),
        working_dir: None,
        timeout_seconds: 30,
    }
}

/// MT-127 HIGH remediation, end-to-end: a secret-named env var exported in
/// the parent (operator shell) MUST NOT leak into the spawned CLI
/// subprocess, while ordinary runtime vars still pass through. Closes the
/// "operator's BYOK keys leak into every spawned CLI subprocess" finding.
#[test]
fn live_cli_spawner_strips_secret_env_but_keeps_public_env() {
    let config = env_echo_config();
    if !config.executable_path.exists() {
        eprintln!(
            "skipping env-scrub test; executable missing: {}",
            config.executable_path.display()
        );
        return;
    }

    // Export a credential-named var (must be scrubbed) and a benign var
    // (must survive) into this process's env, which the child inherits.
    std::env::set_var("SCRUB_PROBE_API_KEY", "leaked-NEVER-LOG-xyz");
    std::env::set_var("SCRUB_PROBE_PUBLIC_DIR", "public-ok-value");

    let receipt = LiveCliSpawner::new(test_ledger())
        .spawn(&config, "model", "prompt")
        .expect("env-echo spawn must succeed");

    std::env::remove_var("SCRUB_PROBE_API_KEY");
    std::env::remove_var("SCRUB_PROBE_PUBLIC_DIR");

    assert!(
        !receipt.stdout.contains("leaked-NEVER-LOG-xyz"),
        "secret-named env var leaked into the spawned subprocess: {}",
        receipt.stdout
    );
    assert!(
        receipt.stdout.contains("public-ok-value"),
        "benign runtime env var was wrongly scrubbed: {}",
        receipt.stdout
    );
}

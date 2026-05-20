//! MT-127 cross-crate integration smoke for the Official CLI bridge
//! runtime scaffold. Exhaustive coverage lives in the inline tests
//! in `model_runtime::cloud::official_cli_bridge::tests`; this file
//! pins the cross-crate API surface + the red_team minimum_controls.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use handshake_core::model_runtime::cloud::{
    CliBridgeConfig, CliInvocationReceipt, CliKind, CliOutputFormat, CliSubprocessSpawner,
    OfficialCliBridgeError, OfficialCliBridgeRuntime,
};
use handshake_core::model_runtime::ModelId;

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
        .register_bridge(fixture_config(), "claude-3.5-sonnet", "2026-05-20T06:30:00Z")
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
    assert!(matches!(
        err,
        OfficialCliBridgeError::ModelNotRegistered(_)
    ));
}

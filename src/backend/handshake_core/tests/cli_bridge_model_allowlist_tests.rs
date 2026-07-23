use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use handshake_core::model_runtime::cloud::cli_bridge_runtime::{
    AllowlistedCliBridgeConfig, CliBridgeModelRuntime, CLI_BRIDGE_MODEL_ALLOWLIST_METADATA_ENV,
};
use handshake_core::model_runtime::cloud::{
    CliBridgeConfig, CliInvocationContext, CliInvocationReceipt, CliKind, CliOutputFormat,
    CliSubprocessSpawner, OfficialCliBridgeError,
};
use handshake_core::model_runtime::{
    KvCachePolicy, LoadSpec, ModelCapabilities, ModelRuntime, ProviderKind, RuntimeKind,
    SamplingParams,
};

#[derive(Default)]
struct PinCapturingSpawner {
    pinned: Mutex<Vec<CliBridgeConfig>>,
}

impl CliSubprocessSpawner for PinCapturingSpawner {
    fn pin_config(&self, config: &CliBridgeConfig) -> Result<(), OfficialCliBridgeError> {
        self.pinned
            .lock()
            .expect("pin capture")
            .push(config.clone());
        Ok(())
    }

    fn spawn(
        &self,
        _config: &CliBridgeConfig,
        _invocation: &CliInvocationContext,
        _model_name: &str,
        _prompt: &str,
    ) -> Result<CliInvocationReceipt, OfficialCliBridgeError> {
        unreachable!("model allowlist tests do not spawn")
    }
}

fn raw_config_with_allowlist(encoded_allowlist: &str) -> CliBridgeConfig {
    let mut env_vars = HashMap::new();
    env_vars.insert(
        CLI_BRIDGE_MODEL_ALLOWLIST_METADATA_ENV.to_string(),
        encoded_allowlist.to_string(),
    );
    CliBridgeConfig {
        cli_kind: CliKind::ClaudeCode,
        executable_path: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
        args_template: vec![
            "--model".to_string(),
            "{model}".to_string(),
            "--prompt".to_string(),
            "{prompt}".to_string(),
        ],
        output_format: CliOutputFormat::RawText,
        env_vars,
        working_dir: None,
        timeout_seconds: 120,
    }
}

#[test]
fn model_allowlist_is_bound_to_exactly_one_provider_supported_cli_argument() {
    let spawner = Arc::new(PinCapturingSpawner::default());
    let runtime = handshake_core::model_runtime::cloud::OfficialCliBridgeRuntime::new(spawner);

    for (label, args_template) in [
        (
            "missing",
            vec!["--prompt".to_string(), "{prompt}".to_string()],
        ),
        (
            "embedded",
            vec![
                "--model".to_string(),
                "model={model}".to_string(),
                "{prompt}".to_string(),
            ],
        ),
        (
            "duplicate",
            vec![
                "--model".to_string(),
                "{model}".to_string(),
                "--model".to_string(),
                "{model}".to_string(),
                "{prompt}".to_string(),
            ],
        ),
        (
            "unsupported-flag",
            vec![
                "--engine".to_string(),
                "{model}".to_string(),
                "{prompt}".to_string(),
            ],
        ),
    ] {
        let mut config = raw_config_with_allowlist(r#"["gpt-5.4"]"#);
        config
            .env_vars
            .remove(CLI_BRIDGE_MODEL_ALLOWLIST_METADATA_ENV);
        config.args_template = args_template;
        let error = runtime
            .register_bridge(config, "gpt-5.4", "2026-07-16T00:00:00Z")
            .expect_err(label);
        assert!(
            matches!(error, OfficialCliBridgeError::InvalidModelBinding(_)),
            "{label} model binding must fail closed, got {error:?}"
        );
    }
}

fn config_with_allowlist(encoded_allowlist: &str) -> AllowlistedCliBridgeConfig {
    AllowlistedCliBridgeConfig::from_config_metadata(raw_config_with_allowlist(encoded_allowlist))
        .expect("valid allowlist metadata")
}

fn load_spec(model_name: &str) -> LoadSpec {
    LoadSpec {
        artifact_path: PathBuf::new(),
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

#[tokio::test]
async fn stored_allowlist_accepts_declared_model_and_is_not_projected_to_child_env() {
    let spawner = Arc::new(PinCapturingSpawner::default());
    let mut runtime = CliBridgeModelRuntime::new(
        spawner.clone(),
        config_with_allowlist(r#"["gpt-5.4","gpt-5.3-codex"]"#),
    );

    runtime
        .load(load_spec("gpt-5.4"))
        .await
        .expect("allowlisted official CLI model loads");

    let pinned = spawner.pinned.lock().expect("pin capture");
    assert_eq!(pinned.len(), 1);
    assert!(!pinned[0]
        .env_vars
        .contains_key(CLI_BRIDGE_MODEL_ALLOWLIST_METADATA_ENV));
}

#[tokio::test]
async fn stored_allowlist_rejects_undeclared_model_before_registration() {
    let spawner = Arc::new(PinCapturingSpawner::default());
    let mut runtime =
        CliBridgeModelRuntime::new(spawner.clone(), config_with_allowlist(r#"["gpt-5.4"]"#));

    let error = runtime
        .load(load_spec("gpt-5.3-codex"))
        .await
        .expect_err("undeclared official CLI model must fail closed");

    assert!(error.to_string().contains("not in the operator allowlist"));
    assert!(spawner.pinned.lock().expect("pin capture").is_empty());
}

#[tokio::test]
async fn malformed_stored_allowlist_fails_closed_before_registration() {
    let spawner = Arc::new(PinCapturingSpawner::default());
    let error =
        AllowlistedCliBridgeConfig::from_config_metadata(raw_config_with_allowlist("not-json"))
            .expect_err("malformed stored allowlist must fail closed at construction");

    assert!(error.contains("allowlist metadata is invalid"));
    assert!(spawner.pinned.lock().expect("pin capture").is_empty());
}

#[tokio::test]
async fn missing_stored_allowlist_fails_closed_before_runtime_construction() {
    let spawner = Arc::new(PinCapturingSpawner::default());
    let mut config = raw_config_with_allowlist(r#"["gpt-5.4"]"#);
    config
        .env_vars
        .remove(CLI_BRIDGE_MODEL_ALLOWLIST_METADATA_ENV);

    let error = AllowlistedCliBridgeConfig::from_config_metadata(config)
        .expect_err("missing stored allowlist must fail closed at construction");

    assert!(error.contains("allowlist metadata is required"));
    assert!(spawner.pinned.lock().expect("pin capture").is_empty());
}

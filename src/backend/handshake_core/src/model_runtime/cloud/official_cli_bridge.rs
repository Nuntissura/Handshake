//! MT-127: Cloud lane Official-CLI bridge runtime (scoped scaffold).
//!
//! Different posture from MT-125 / MT-126 HTTP BYOK runtimes: this
//! adapter transports invocations through an OFFICIAL CLI subprocess
//! (Claude Code, Codex CLI, gemini-cli, ...). Operator auth is
//! handled by the CLI itself - the kernel does NOT store an API
//! key. The scaffold lands the config shape + capability declaration
//! + subprocess-spawn abstraction + LiveSpawnUnavailable for the
//! live path; concrete sandbox + ProcessOwnershipLedger
//! engine_kind=OfficialCliBridge wiring is the follow-on alongside
//! the cluster-B sandbox adapter + MT-069 ledger row plumbing.
//!
//! Per MT-127 implementation_notes: NONE of the inference techniques
//! (LoRA / KV / steering / subquadratic / speculative) work through
//! a CLI bridge - all capability flags MUST be false. The bridge is
//! a usability-not-feature lane for operator workflow flexibility.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use thiserror::Error;

use crate::model_runtime::{ModelCapabilities, ModelId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CliKind {
    ClaudeCode,
    CodexCli,
    GeminiCli,
    Other,
}

impl CliKind {
    pub fn label(self) -> &'static str {
        match self {
            CliKind::ClaudeCode => "claude_code",
            CliKind::CodexCli => "codex_cli",
            CliKind::GeminiCli => "gemini_cli",
            CliKind::Other => "other",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CliOutputFormat {
    Json,
    RawText,
    JsonStream,
}

/// Operator-supplied configuration for a CLI bridge instance.
/// `args_template` may contain `{prompt}` and `{model}` placeholders
/// which the bridge substitutes per request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CliBridgeConfig {
    pub cli_kind: CliKind,
    pub executable_path: PathBuf,
    pub args_template: Vec<String>,
    pub output_format: CliOutputFormat,
    pub env_vars: HashMap<String, String>,
    pub working_dir: Option<PathBuf>,
    pub timeout_seconds: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CliBridgeHandle {
    pub model_id: ModelId,
    pub cli_kind: CliKind,
    pub model_name: String,
    pub registered_at_utc: String,
}

#[derive(Debug, Error)]
pub enum OfficialCliBridgeError {
    #[error("executable_path must exist; got {0}")]
    ExecutableNotFound(PathBuf),
    #[error("model_name must not be empty")]
    EmptyModelName,
    #[error("args_template must contain {{prompt}} placeholder for prompt substitution")]
    MissingPromptPlaceholder,
    #[error("timeout_seconds must be > 0")]
    InvalidTimeout,
    #[error("model_id {0} is not registered with the CLI bridge runtime")]
    ModelNotRegistered(ModelId),
    #[error("internal lock poisoned: {0}")]
    LockPoisoned(String),
    #[error(
        "live subprocess spawn not attached: {0}; wire the SandboxAdapter (cluster B) + the \
         MT-069 ProcessOwnershipLedger row with engine_kind=OfficialCliBridge before invoking \
         the live CLI"
    )]
    LiveSpawnUnavailable(String),
}

/// Abstraction over the sandboxed subprocess spawn. The production
/// impl wraps the cluster-B SandboxAdapter; the mock impl backs
/// unit tests. The MT-069 ProcessOwnershipLedger row with
/// engine_kind=OfficialCliBridge is the trait impl's responsibility.
pub trait CliSubprocessSpawner: Send + Sync {
    fn spawn(
        &self,
        config: &CliBridgeConfig,
        model_name: &str,
        prompt: &str,
    ) -> Result<CliInvocationReceipt, OfficialCliBridgeError>;
}

/// Result of one spawn attempt. The live impl populates pid; the
/// mock impl populates `mock_pid = None`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CliInvocationReceipt {
    pub model_id: ModelId,
    pub stdout: String,
    pub pid: Option<u32>,
    pub exit_code: Option<i32>,
    pub cancelled: bool,
}

pub struct OfficialCliBridgeRuntime {
    spawner: Arc<dyn CliSubprocessSpawner>,
    bridges: RwLock<HashMap<ModelId, (CliBridgeConfig, CliBridgeHandle)>>,
}

impl std::fmt::Debug for OfficialCliBridgeRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OfficialCliBridgeRuntime")
            .field("spawner", &"<Arc<dyn CliSubprocessSpawner>>")
            .field(
                "bridges",
                &self.bridges.read().map(|b| b.len()).unwrap_or(0),
            )
            .finish()
    }
}

impl OfficialCliBridgeRuntime {
    pub fn new(spawner: Arc<dyn CliSubprocessSpawner>) -> Self {
        Self {
            spawner,
            bridges: RwLock::new(HashMap::new()),
        }
    }

    /// Register a CLI bridge configuration as a model handle.
    /// Validates the config fields then mints a ModelId v7.
    pub fn register_bridge(
        &self,
        config: CliBridgeConfig,
        model_name: &str,
        now_utc: &str,
    ) -> Result<CliBridgeHandle, OfficialCliBridgeError> {
        if model_name.trim().is_empty() {
            return Err(OfficialCliBridgeError::EmptyModelName);
        }
        if !config.executable_path.exists() {
            return Err(OfficialCliBridgeError::ExecutableNotFound(
                config.executable_path.clone(),
            ));
        }
        if !config
            .args_template
            .iter()
            .any(|arg| arg.contains("{prompt}"))
        {
            return Err(OfficialCliBridgeError::MissingPromptPlaceholder);
        }
        if config.timeout_seconds == 0 {
            return Err(OfficialCliBridgeError::InvalidTimeout);
        }
        let model_id = ModelId::new_v7();
        let handle = CliBridgeHandle {
            model_id,
            cli_kind: config.cli_kind,
            model_name: model_name.to_string(),
            registered_at_utc: now_utc.to_string(),
        };
        let mut bridges = self
            .bridges
            .write()
            .map_err(|err| OfficialCliBridgeError::LockPoisoned(err.to_string()))?;
        bridges.insert(model_id, (config, handle.clone()));
        Ok(handle)
    }

    /// Cluster-B realities: NONE of the inference techniques work
    /// through a CLI subprocess. Every capability flag is false per
    /// MT-127 red_team minimum_controls[1]. The bridge is a
    /// usability-not-feature lane.
    pub fn cli_bridge_capabilities() -> ModelCapabilities {
        ModelCapabilities {
            supports_lora: false,
            supports_kv_prefix_cache: false,
            supports_kv_quantization: crate::model_runtime::KvQuantSupport::None,
            supports_activation_steering: false,
            supports_subquadratic: false,
            supports_speculative_draft: false,
            supports_eagle3: false,
        }
    }

    pub fn handle_for(
        &self,
        model_id: ModelId,
    ) -> Result<CliBridgeHandle, OfficialCliBridgeError> {
        let bridges = self
            .bridges
            .read()
            .map_err(|err| OfficialCliBridgeError::LockPoisoned(err.to_string()))?;
        bridges
            .get(&model_id)
            .map(|(_, handle)| handle.clone())
            .ok_or(OfficialCliBridgeError::ModelNotRegistered(model_id))
    }

    /// Substitutes `{prompt}` and `{model}` placeholders in
    /// args_template. Pure helper exposed publicly so tests can pin
    /// the substitution rule without spawning a subprocess.
    pub fn render_args(args_template: &[String], model_name: &str, prompt: &str) -> Vec<String> {
        args_template
            .iter()
            .map(|arg| {
                arg.replace("{prompt}", prompt)
                    .replace("{model}", model_name)
            })
            .collect()
    }

    /// Invoke the bridge: looks up the registered config, asks the
    /// spawner to run the CLI with the rendered args, returns the
    /// receipt. The spawner is responsible for the sandbox boundary
    /// + ProcessOwnershipLedger registration; the runtime here is
    /// the contract surface + validation gate.
    pub fn invoke(
        &self,
        model_id: ModelId,
        prompt: &str,
    ) -> Result<CliInvocationReceipt, OfficialCliBridgeError> {
        let (config, handle) = {
            let bridges = self
                .bridges
                .read()
                .map_err(|err| OfficialCliBridgeError::LockPoisoned(err.to_string()))?;
            bridges
                .get(&model_id)
                .cloned()
                .ok_or(OfficialCliBridgeError::ModelNotRegistered(model_id))?
        };
        self.spawner.spawn(&config, &handle.model_name, prompt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// Mock spawner that records the last invocation and returns a
    /// configurable canned response.
    struct CapturingSpawner {
        canned_stdout: String,
        last_invocation: Mutex<Option<(CliBridgeConfig, String, String)>>,
    }
    impl CliSubprocessSpawner for CapturingSpawner {
        fn spawn(
            &self,
            config: &CliBridgeConfig,
            model_name: &str,
            prompt: &str,
        ) -> Result<CliInvocationReceipt, OfficialCliBridgeError> {
            *self.last_invocation.lock().unwrap() =
                Some((config.clone(), model_name.to_string(), prompt.to_string()));
            Ok(CliInvocationReceipt {
                model_id: ModelId::new_v7(),
                stdout: self.canned_stdout.clone(),
                pid: None,
                exit_code: Some(0),
                cancelled: false,
            })
        }
    }

    struct UnavailableSpawner;
    impl CliSubprocessSpawner for UnavailableSpawner {
        fn spawn(
            &self,
            _config: &CliBridgeConfig,
            _model_name: &str,
            _prompt: &str,
        ) -> Result<CliInvocationReceipt, OfficialCliBridgeError> {
            Err(OfficialCliBridgeError::LiveSpawnUnavailable(
                "production spawner not wired".to_string(),
            ))
        }
    }

    fn temp_exe() -> PathBuf {
        // Use a file that definitely exists on every host the test
        // runs on. cargo's manifest dir always exists.
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")
    }

    fn good_config() -> CliBridgeConfig {
        CliBridgeConfig {
            cli_kind: CliKind::ClaudeCode,
            executable_path: temp_exe(),
            args_template: vec!["--model".to_string(), "{model}".to_string(), "--prompt".to_string(), "{prompt}".to_string()],
            output_format: CliOutputFormat::Json,
            env_vars: HashMap::new(),
            working_dir: None,
            timeout_seconds: 120,
        }
    }

    #[test]
    fn capabilities_are_all_false_per_red_team_minimum_controls() {
        let caps = OfficialCliBridgeRuntime::cli_bridge_capabilities();
        assert!(!caps.supports_lora);
        assert!(!caps.supports_kv_prefix_cache);
        assert!(!caps.supports_activation_steering);
        assert!(!caps.supports_subquadratic);
        assert!(!caps.supports_speculative_draft);
        assert!(!caps.supports_eagle3);
    }

    #[test]
    fn register_bridge_validates_executable_path_and_placeholder_and_timeout() {
        let runtime = OfficialCliBridgeRuntime::new(Arc::new(UnavailableSpawner));
        // Missing prompt placeholder.
        let mut bad = good_config();
        bad.args_template = vec!["--model".to_string(), "{model}".to_string()];
        let err = runtime
            .register_bridge(bad, "claude-3.5-sonnet", "2026-05-20T06:00:00Z")
            .expect_err("missing placeholder");
        assert!(matches!(
            err,
            OfficialCliBridgeError::MissingPromptPlaceholder
        ));

        // Bad executable path.
        let mut bad = good_config();
        bad.executable_path = PathBuf::from("/this/path/definitely/does/not/exist/nope");
        let err = runtime
            .register_bridge(bad, "claude-3.5-sonnet", "2026-05-20T06:00:00Z")
            .expect_err("missing exe");
        assert!(matches!(
            err,
            OfficialCliBridgeError::ExecutableNotFound(_)
        ));

        // Empty model name.
        let err = runtime
            .register_bridge(good_config(), "  ", "2026-05-20T06:00:00Z")
            .expect_err("empty model name");
        assert!(matches!(err, OfficialCliBridgeError::EmptyModelName));

        // Zero timeout.
        let mut bad = good_config();
        bad.timeout_seconds = 0;
        let err = runtime
            .register_bridge(bad, "claude-3.5-sonnet", "2026-05-20T06:00:00Z")
            .expect_err("invalid timeout");
        assert!(matches!(err, OfficialCliBridgeError::InvalidTimeout));
    }

    #[test]
    fn render_args_substitutes_prompt_and_model_placeholders() {
        let args = vec![
            "--model".to_string(),
            "{model}".to_string(),
            "--prompt".to_string(),
            "Hello {prompt}".to_string(),
        ];
        let rendered = OfficialCliBridgeRuntime::render_args(&args, "claude-3.5", "world");
        assert_eq!(
            rendered,
            vec![
                "--model".to_string(),
                "claude-3.5".to_string(),
                "--prompt".to_string(),
                "Hello world".to_string(),
            ]
        );
    }

    #[test]
    fn invoke_routes_through_spawner_with_registered_config() {
        let spawner = Arc::new(CapturingSpawner {
            canned_stdout: r#"{"completion":"hi"}"#.to_string(),
            last_invocation: Mutex::new(None),
        });
        let runtime = OfficialCliBridgeRuntime::new(spawner.clone());
        let handle = runtime
            .register_bridge(good_config(), "claude-3.5-sonnet", "2026-05-20T06:00:00Z")
            .expect("register");

        let receipt = runtime
            .invoke(handle.model_id, "hello world")
            .expect("invoke");
        assert_eq!(receipt.stdout, r#"{"completion":"hi"}"#);
        let captured = spawner.last_invocation.lock().unwrap().clone().unwrap();
        assert_eq!(captured.1, "claude-3.5-sonnet");
        assert_eq!(captured.2, "hello world");
    }

    #[test]
    fn invoke_on_unregistered_model_errors() {
        let runtime = OfficialCliBridgeRuntime::new(Arc::new(UnavailableSpawner));
        let unknown = ModelId::new_v7();
        let err = runtime.invoke(unknown, "x").expect_err("unknown model");
        assert!(matches!(
            err,
            OfficialCliBridgeError::ModelNotRegistered(_)
        ));
    }

    #[test]
    fn live_spawn_unavailable_until_sandbox_wired() {
        let runtime = OfficialCliBridgeRuntime::new(Arc::new(UnavailableSpawner));
        let handle = runtime
            .register_bridge(good_config(), "claude-3.5-sonnet", "2026-05-20T06:00:00Z")
            .expect("register");
        let err = runtime
            .invoke(handle.model_id, "hello")
            .expect_err("not yet wired");
        assert!(matches!(
            err,
            OfficialCliBridgeError::LiveSpawnUnavailable(_)
        ));
    }

    #[test]
    fn cli_kind_label_is_stable() {
        assert_eq!(CliKind::ClaudeCode.label(), "claude_code");
        assert_eq!(CliKind::CodexCli.label(), "codex_cli");
        assert_eq!(CliKind::GeminiCli.label(), "gemini_cli");
        assert_eq!(CliKind::Other.label(), "other");
    }
}

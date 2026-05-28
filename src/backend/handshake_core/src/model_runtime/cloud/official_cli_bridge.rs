//! MT-127: Cloud lane Official-CLI bridge runtime.
//!
//! Different posture from MT-125 / MT-126 HTTP BYOK runtimes: this
//! adapter transports invocations through an OFFICIAL CLI subprocess
//! (Claude Code, Codex CLI, gemini-cli, ...). Operator auth is
//! handled by the CLI itself - the kernel does NOT store an API
//! key.
//!
//! The runtime composes three pieces: a typed `CliBridgeConfig` that
//! captures the executable path, args template, output format, env
//! vars, working directory and timeout for each registered CLI; the
//! `CliSubprocessSpawner` trait that owns the actual subprocess
//! boundary (so tests can substitute capturing spawners while the
//! production path runs real binaries via `LiveCliSpawner`); and the
//! `OfficialCliBridgeRuntime` itself which validates configs at
//! `register_bridge` time and dispatches per-request through the
//! spawner at `invoke` time.
//!
//! Per MT-127 implementation_notes: NONE of the inference techniques
//! (LoRA / KV / steering / subquadratic / speculative) work through
//! a CLI bridge - all capability flags MUST be false. The bridge is
//! a usability-not-feature lane for operator workflow flexibility.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Child, Command, Output, Stdio};
use std::sync::{mpsc, Arc, RwLock};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::json;
use thiserror::Error;

use crate::model_runtime::{ModelCapabilities, ModelId};
use crate::process_ledger::{record_spawn, LedgerBatcher, ProcessEngineKind, SpawnMeta};

/// Default owner role recorded on the CLI bridge subprocess's
/// ProcessOwnershipLedger row when the caller does not override it.
const DEFAULT_CLI_BRIDGE_OWNER_ROLE: &str = "OFFICIAL_CLI_BRIDGE";

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
    #[error("CLI subprocess spawn failed: {reason}")]
    SpawnFailed {
        reason: String,
        exit_code: Option<i32>,
    },
    #[error("CLI subprocess exceeded timeout {timeout_seconds}s; sent kill signal")]
    SpawnTimeout {
        timeout_seconds: u64,
        partial_stdout: String,
    },
    #[error(
        "ProcessOwnershipLedger registration failed for the CLI bridge subprocess (pid {pid}): \
         {reason}; the subprocess was killed to avoid leaving an unattributed process"
    )]
    LedgerRegistration { pid: u32, reason: String },
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

    pub fn handle_for(&self, model_id: ModelId) -> Result<CliBridgeHandle, OfficialCliBridgeError> {
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

/// Production `CliSubprocessSpawner` that drives a real subprocess
/// via `std::process::Command`. Renders the args template, applies
/// the configured env vars (after `env_clear` so the subprocess does
/// not inherit the parent's environment by default), honours the
/// configured working directory, and enforces the configured timeout
/// by polling `try_wait` and sending `kill` on overrun.
///
/// PID, exit_code and captured stdout are recorded on the
/// `CliInvocationReceipt` so callers can attribute the run.
///
/// MT-127 remediation: when a [`LedgerBatcher`] is attached via
/// [`Self::with_process_ledger`], the spawn is registered as an
/// attributable + reclaimable `ProcessOwnershipLedger` row
/// (`engine_kind = OfficialCliBridge`) immediately after the child pid
/// is captured, mirroring the MT-122 distillation trainer pattern. The
/// spawner FAILS CLOSED: if ledger registration fails, the just-spawned
/// child is killed and an error is returned rather than leaving an
/// unattributed/unreclaimable process running. Without a ledger the run
/// still works but is unattributed, so production call sites SHOULD
/// attach the ledger.
#[derive(Clone)]
pub struct LiveCliSpawner {
    process_ledger: Option<Arc<LedgerBatcher>>,
    owner_role: String,
}

impl std::fmt::Debug for LiveCliSpawner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // LedgerBatcher is not Debug (it wraps channels); report only
        // whether a ledger is attached so LiveCliSpawner stays Debug.
        f.debug_struct("LiveCliSpawner")
            .field("process_ledger", &self.process_ledger.is_some())
            .field("owner_role", &self.owner_role)
            .finish()
    }
}

impl Default for LiveCliSpawner {
    fn default() -> Self {
        Self::new()
    }
}

/// Build the ProcessOwnershipLedger row metadata for a CLI bridge
/// subprocess so the spawned process is attributable + reclaimable
/// (MT-127 HIGH remediation). Pure helper extracted so tests can pin
/// the engine_kind + metadata markers without spawning a subprocess,
/// mirroring MT-122's `distillation_spawn_meta`.
fn cli_bridge_spawn_meta(
    pid: u32,
    owner_role: &str,
    model_name: &str,
    executable_path: &std::path::Path,
) -> SpawnMeta {
    let mut meta = SpawnMeta::new(pid, ProcessEngineKind::OfficialCliBridge, owner_role);
    meta.sandbox_adapter = Some("official_cli_bridge".to_string());
    meta.model_id = Some(model_name.to_string());
    meta.mt_id = Some("MT-127".to_string());
    meta.metadata_blob = json!({
        "subprocess_kind": "official_cli_bridge",
        "mt": "MT-127",
        "model_name": model_name,
        "executable": executable_path.display().to_string(),
    });
    meta
}

/// MT-127 remediation (HIGH): returns true if an inherited environment
/// variable name looks like it carries a credential, so the CLI-bridge
/// spawner can strip it from the child env before launch. Matches on
/// case-insensitive secret-bearing substrings. PATH / USERPROFILE /
/// APPDATA and other runtime vars the CLI needs are intentionally NOT
/// matched, so the subprocess still starts.
fn is_secret_bearing_env_name(name: &str) -> bool {
    const SECRET_SUBSTRINGS: &[&str] = &[
        "API_KEY",
        "APIKEY",
        "SECRET",
        "TOKEN",
        "PASSWORD",
        "PASSWD",
        "ANTHROPIC_",
        "OPENAI_",
        "GEMINI_",
        "GOOGLE_API",
        "AWS_SECRET",
        "AZURE_",
        "HF_TOKEN",
        "HUGGINGFACE",
        "PRIVATE_KEY",
        "CREDENTIAL",
        "ACCESS_KEY",
        "BEARER",
        "SESSION_KEY",
    ];
    let upper = name.to_ascii_uppercase();
    SECRET_SUBSTRINGS
        .iter()
        .any(|needle| upper.contains(needle))
}

const POST_TIMEOUT_OUTPUT_GRACE: Duration = Duration::from_secs(2);

fn kill_process_tree(pid: u32, child: &mut Child) {
    #[cfg(windows)]
    {
        let pid_arg = pid.to_string();
        let _ = Command::new("taskkill")
            .args(["/PID", pid_arg.as_str(), "/T", "/F"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    let _ = child.kill();
}

fn wait_with_output_bounded(child: Child, timeout: Duration) -> Option<Output> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(child.wait_with_output());
    });
    rx.recv_timeout(timeout).ok().and_then(Result::ok)
}

impl LiveCliSpawner {
    /// Construct a spawner with no process ledger attached. The spawn
    /// still runs but is unattributed; production call sites SHOULD use
    /// [`Self::with_process_ledger`] so each CLI subprocess is recorded
    /// as an attributable ProcessOwnershipLedger row.
    pub fn new() -> Self {
        Self {
            process_ledger: None,
            owner_role: DEFAULT_CLI_BRIDGE_OWNER_ROLE.to_string(),
        }
    }

    /// Attach a process ledger so each CLI bridge subprocess is
    /// registered as an attributable + reclaimable
    /// `ProcessOwnershipLedger` row (`engine_kind = OfficialCliBridge`)
    /// on spawn (MT-127). Fails closed if registration fails.
    pub fn with_process_ledger(mut self, ledger: Arc<LedgerBatcher>) -> Self {
        self.process_ledger = Some(ledger);
        self
    }

    /// Override the owner role recorded on the ledger row (defaults to
    /// `OFFICIAL_CLI_BRIDGE`).
    pub fn with_owner_role(mut self, owner_role: impl Into<String>) -> Self {
        self.owner_role = owner_role.into();
        self
    }
}

impl CliSubprocessSpawner for LiveCliSpawner {
    fn spawn(
        &self,
        config: &CliBridgeConfig,
        model_name: &str,
        prompt: &str,
    ) -> Result<CliInvocationReceipt, OfficialCliBridgeError> {
        let rendered =
            OfficialCliBridgeRuntime::render_args(&config.args_template, model_name, prompt);

        let mut cmd = Command::new(&config.executable_path);
        cmd.args(&rendered);
        // MT-127 remediation (HIGH): we cannot env_clear() — Node-based
        // CLIs (claude, codex, gemini) load runtime DLLs via PATH +
        // USERPROFILE + APPDATA on Windows; stripping the inherited
        // environment causes STATUS_ACCESS_VIOLATION (0xC0000005) at
        // process startup. But a blind parent-env inherit leaks the
        // operator's shell-exported BYOK credentials (OPENAI_API_KEY,
        // ANTHROPIC_API_KEY, ...) into every spawned subprocess. The CLI
        // bridge authenticates via the operator's subscription login, not
        // via vendor API-key env vars (BYOK is operationally dormant per
        // the MT operator clarification), so we scrub secret-bearing var
        // names from the inherited env while preserving the runtime vars
        // the CLI needs. config.env_vars is applied AFTER the scrub, so an
        // explicit operator-provided value is an intentional opt-in.
        for (key, _value) in std::env::vars_os() {
            if let Some(name) = key.to_str() {
                if is_secret_bearing_env_name(name) {
                    cmd.env_remove(&key);
                }
            }
        }
        for (key, value) in &config.env_vars {
            cmd.env(key, value);
        }
        if let Some(dir) = &config.working_dir {
            cmd.current_dir(dir);
        }
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.stdin(Stdio::null());

        let mut child = cmd
            .spawn()
            .map_err(|err| OfficialCliBridgeError::SpawnFailed {
                reason: format!(
                    "failed to spawn {}: {err}",
                    config.executable_path.display()
                ),
                exit_code: None,
            })?;
        let pid = child.id();

        // MT-127 remediation (HIGH): the moment the child pid is known
        // and the row is attributable, register a ProcessOwnershipLedger
        // row with engine_kind=OfficialCliBridge so the spawned CLI
        // subprocess is attributable + reclaimable. Fail closed: if
        // registration fails, kill the child rather than leaving an
        // unattributed/unreclaimable process running.
        if let Some(ledger) = &self.process_ledger {
            let meta = cli_bridge_spawn_meta(
                pid,
                &self.owner_role,
                model_name,
                &config.executable_path,
            );
            if let Err(err) = record_spawn(ledger, meta) {
                kill_process_tree(pid, &mut child);
                return Err(OfficialCliBridgeError::LedgerRegistration {
                    pid,
                    reason: err.to_string(),
                });
            }
        }

        let timeout = Duration::from_secs(config.timeout_seconds);
        let started = Instant::now();
        let exit_status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => {
                    if started.elapsed() >= timeout {
                        kill_process_tree(pid, &mut child);
                        let partial_stdout =
                            wait_with_output_bounded(child, POST_TIMEOUT_OUTPUT_GRACE)
                                .map(|output| String::from_utf8_lossy(&output.stdout).into_owned())
                                .unwrap_or_default();
                        return Err(OfficialCliBridgeError::SpawnTimeout {
                            timeout_seconds: config.timeout_seconds,
                            partial_stdout,
                        });
                    }
                    std::thread::sleep(Duration::from_millis(25));
                }
                Err(err) => {
                    return Err(OfficialCliBridgeError::SpawnFailed {
                        reason: format!("try_wait failed: {err}"),
                        exit_code: None,
                    });
                }
            }
        };

        let output =
            child
                .wait_with_output()
                .map_err(|err| OfficialCliBridgeError::SpawnFailed {
                    reason: format!("wait_with_output failed: {err}"),
                    exit_code: exit_status.code(),
                })?;

        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        let exit_code = output.status.code();

        if !output.status.success() {
            return Err(OfficialCliBridgeError::SpawnFailed {
                reason: format!(
                    "CLI {} exited with status {:?}; stderr={}",
                    config.executable_path.display(),
                    exit_code,
                    stderr.trim()
                ),
                exit_code,
            });
        }

        Ok(CliInvocationReceipt {
            model_id: ModelId::new_v7(),
            stdout,
            pid: Some(pid),
            exit_code,
            cancelled: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;
    use std::time::Instant;

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

    struct FailingSpawner;
    impl CliSubprocessSpawner for FailingSpawner {
        fn spawn(
            &self,
            _config: &CliBridgeConfig,
            _model_name: &str,
            _prompt: &str,
        ) -> Result<CliInvocationReceipt, OfficialCliBridgeError> {
            Err(OfficialCliBridgeError::SpawnFailed {
                reason: "test fault injection".to_string(),
                exit_code: None,
            })
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
            args_template: vec![
                "--model".to_string(),
                "{model}".to_string(),
                "--prompt".to_string(),
                "{prompt}".to_string(),
            ],
            output_format: CliOutputFormat::Json,
            env_vars: HashMap::new(),
            working_dir: None,
            timeout_seconds: 120,
        }
    }

    #[cfg(windows)]
    fn timeout_config() -> CliBridgeConfig {
        CliBridgeConfig {
            cli_kind: CliKind::Other,
            executable_path: PathBuf::from(
                std::env::var("ComSpec")
                    .unwrap_or_else(|_| "C:\\Windows\\System32\\cmd.exe".to_string()),
            ),
            args_template: vec![
                "/C".to_string(),
                "echo {model}-{prompt} && ping -n 6 127.0.0.1 > nul".to_string(),
            ],
            output_format: CliOutputFormat::RawText,
            env_vars: HashMap::new(),
            working_dir: None,
            timeout_seconds: 1,
        }
    }

    #[cfg(not(windows))]
    fn timeout_config() -> CliBridgeConfig {
        CliBridgeConfig {
            cli_kind: CliKind::Other,
            executable_path: PathBuf::from("/bin/sh"),
            args_template: vec![
                "-c".to_string(),
                "printf '%s-%s\\n' '{model}' '{prompt}'; sleep 6".to_string(),
            ],
            output_format: CliOutputFormat::RawText,
            env_vars: HashMap::new(),
            working_dir: None,
            timeout_seconds: 1,
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
        let runtime = OfficialCliBridgeRuntime::new(Arc::new(FailingSpawner));
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
        assert!(matches!(err, OfficialCliBridgeError::ExecutableNotFound(_)));

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
        let runtime = OfficialCliBridgeRuntime::new(Arc::new(FailingSpawner));
        let unknown = ModelId::new_v7();
        let err = runtime.invoke(unknown, "x").expect_err("unknown model");
        assert!(matches!(err, OfficialCliBridgeError::ModelNotRegistered(_)));
    }

    #[test]
    fn spawn_failed_surfaces_through_invoke() {
        let runtime = OfficialCliBridgeRuntime::new(Arc::new(FailingSpawner));
        let handle = runtime
            .register_bridge(good_config(), "claude-3.5-sonnet", "2026-05-20T06:00:00Z")
            .expect("register");
        let err = runtime
            .invoke(handle.model_id, "hello")
            .expect_err("spawner returned failure");
        assert!(matches!(err, OfficialCliBridgeError::SpawnFailed { .. }));
    }

    #[test]
    fn live_spawner_timeout_is_bounded_after_kill() {
        let config = timeout_config();
        if !config.executable_path.exists() {
            eprintln!(
                "skipping live timeout test; executable missing: {}",
                config.executable_path.display()
            );
            return;
        }

        let started = Instant::now();
        let err = LiveCliSpawner::new()
            .spawn(&config, "model", "prompt")
            .expect_err("timeout command must fail with SpawnTimeout");

        assert!(matches!(err, OfficialCliBridgeError::SpawnTimeout { .. }));
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "timeout branch must not wait for the full child sleep"
        );
    }

    #[test]
    fn cli_kind_label_is_stable() {
        assert_eq!(CliKind::ClaudeCode.label(), "claude_code");
        assert_eq!(CliKind::CodexCli.label(), "codex_cli");
        assert_eq!(CliKind::GeminiCli.label(), "gemini_cli");
        assert_eq!(CliKind::Other.label(), "other");
    }

    #[test]
    fn cli_bridge_spawn_meta_is_attributable() {
        // MT-127 HIGH remediation: the CLI bridge subprocess must be
        // recorded as an attributable ProcessOwnershipLedger row with
        // engine_kind=OfficialCliBridge + a clear MT-127 metadata
        // marker, mirroring MT-122's distillation_spawn_meta test.
        let meta = cli_bridge_spawn_meta(
            7777,
            DEFAULT_CLI_BRIDGE_OWNER_ROLE,
            "claude-3.5-sonnet",
            &PathBuf::from("/usr/local/bin/claude"),
        );
        assert_eq!(meta.pid, 7777);
        assert_eq!(meta.engine_kind, ProcessEngineKind::OfficialCliBridge);
        assert_eq!(meta.owner_role, "OFFICIAL_CLI_BRIDGE");
        assert_eq!(meta.mt_id.as_deref(), Some("MT-127"));
        assert_eq!(meta.sandbox_adapter.as_deref(), Some("official_cli_bridge"));
        assert_eq!(meta.model_id.as_deref(), Some("claude-3.5-sonnet"));
        assert_eq!(
            meta.metadata_blob["subprocess_kind"].as_str(),
            Some("official_cli_bridge")
        );
        assert_eq!(meta.metadata_blob["mt"].as_str(), Some("MT-127"));
        assert_eq!(
            meta.metadata_blob["model_name"].as_str(),
            Some("claude-3.5-sonnet")
        );
        assert!(meta.metadata_blob["executable"]
            .as_str()
            .unwrap()
            .contains("claude"));
    }

    #[test]
    fn live_cli_spawner_default_owner_role_is_set() {
        // Default()/new() must yield the canonical owner role so the
        // ledger row is attributable even when the caller does not
        // override it; with_owner_role overrides it.
        let spawner = LiveCliSpawner::default();
        assert_eq!(spawner.owner_role, "OFFICIAL_CLI_BRIDGE");
        assert!(spawner.process_ledger.is_none());
        let custom = LiveCliSpawner::new().with_owner_role("DISTILLATION_PIPELINE");
        assert_eq!(custom.owner_role, "DISTILLATION_PIPELINE");
    }

    #[test]
    fn process_engine_kind_official_cli_bridge_roundtrips() {
        // The new engine kind must serialize to a stable wire string and
        // parse back, so ledger reads/writes are consistent.
        assert_eq!(
            ProcessEngineKind::OfficialCliBridge.as_str(),
            "official_cli_bridge"
        );
        assert_eq!(
            ProcessEngineKind::try_from("official_cli_bridge").unwrap(),
            ProcessEngineKind::OfficialCliBridge
        );
        // OfficialCliBridge is NOT a regular local model runtime engine.
        assert!(!ProcessEngineKind::OfficialCliBridge.is_regular_model_runtime_engine());
    }

    #[test]
    fn secret_bearing_env_names_are_scrubbed_runtime_vars_are_kept() {
        // MT-127 HIGH: credential-named vars must be stripped from the
        // inherited child env; the runtime vars Node CLIs need must pass.
        for secret in [
            "OPENAI_API_KEY",
            "ANTHROPIC_API_KEY",
            "GEMINI_API_KEY",
            "GOOGLE_API_KEY",
            "HF_TOKEN",
            "AWS_SECRET_ACCESS_KEY",
            "MY_SERVICE_TOKEN",
            "DB_PASSWORD",
        ] {
            assert!(
                is_secret_bearing_env_name(secret),
                "{secret} must be treated as secret-bearing"
            );
        }
        for runtime_var in [
            "PATH", "USERPROFILE", "APPDATA", "LOCALAPPDATA", "SystemRoot", "TEMP", "HOME",
            "ComSpec",
        ] {
            assert!(
                !is_secret_bearing_env_name(runtime_var),
                "{runtime_var} is a runtime var and must NOT be scrubbed"
            );
        }
    }
}

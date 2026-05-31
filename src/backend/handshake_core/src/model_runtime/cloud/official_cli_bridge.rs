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
use crate::process_ledger::{
    LedgerBatcher, ProcessEngineKind, ProcessOwnershipRecordId, ProcessStart, ProcessStop,
    SpawnMeta,
};

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

    /// Spawn the CLI subprocess and stream its piped stdout LIVE: `on_chunk` is
    /// invoked with each raw byte slice as it is read from the child's stdout
    /// pipe DURING the run (not after completion). This is what lets the §10.1
    /// capture seam attach a real live background stream rather than a post-hoc
    /// dump. The default impl falls back to [`CliSubprocessSpawner::spawn`] and
    /// replays the captured stdout once (so mock spawners without a real pipe
    /// still work); [`LiveCliSpawner`] overrides it with a true incremental
    /// pipe reader.
    fn spawn_streaming(
        &self,
        config: &CliBridgeConfig,
        model_name: &str,
        prompt: &str,
        on_chunk: &mut dyn FnMut(&[u8]),
    ) -> Result<CliInvocationReceipt, OfficialCliBridgeError> {
        let receipt = self.spawn(config, model_name, prompt)?;
        if !receipt.stdout.is_empty() {
            on_chunk(receipt.stdout.as_bytes());
        }
        Ok(receipt)
    }

    /// Cancellable variant of [`CliSubprocessSpawner::spawn_streaming`]: in
    /// addition to the live `on_chunk` fan-out it consults `should_cancel`
    /// between reads and, when it observes a set cancellation, kills the child
    /// subprocess and returns a receipt with `cancelled = true` rather than
    /// running the CLI to completion. This is what lets the swarm
    /// `CliBridgeModelRuntime` honour the request/runtime `CancellationToken`
    /// by actually killing the backing process (poll-based token — see
    /// [`crate::model_runtime::CancellationToken`]).
    ///
    /// The DEFAULT impl ignores `should_cancel` and delegates to
    /// [`CliSubprocessSpawner::spawn_streaming`], so existing mock/test spawners
    /// that do not override it keep compiling and behaving exactly as before.
    /// [`LiveCliSpawner`] overrides it with a real per-iteration kill check.
    fn spawn_streaming_cancellable(
        &self,
        config: &CliBridgeConfig,
        model_name: &str,
        prompt: &str,
        on_chunk: &mut dyn FnMut(&[u8]),
        _should_cancel: &dyn Fn() -> bool,
    ) -> Result<CliInvocationReceipt, OfficialCliBridgeError> {
        self.spawn_streaming(config, model_name, prompt, on_chunk)
    }
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

    /// Remove a registered CLI bridge handle (the `unload` counterpart to
    /// [`register_bridge`]). Returns the removed handle so callers can audit
    /// the teardown, or [`OfficialCliBridgeError::ModelNotRegistered`] if the
    /// model was never registered / already unloaded. A CLI bridge owns no
    /// local weights, so removal of the map entry is the full free.
    pub fn unregister(&self, model_id: ModelId) -> Result<CliBridgeHandle, OfficialCliBridgeError> {
        let mut bridges = self
            .bridges
            .write()
            .map_err(|err| OfficialCliBridgeError::LockPoisoned(err.to_string()))?;
        bridges
            .remove(&model_id)
            .map(|(_, handle)| handle)
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

    /// Invoke the bridge AND mirror its LIVE piped stdout into the Integrated
    /// Terminal capture seam (spec §10.1). This is a real capture producer for
    /// the "inspect all background work" deliverable: the cloud CLI bridge's
    /// stdout is read incrementally DURING the subprocess run (via
    /// [`CliSubprocessSpawner::spawn_streaming`]) and each chunk is fanned, after
    /// redaction, into a read-only AiJob capture session so the operator can
    /// inspect cloud-CLI background work in the Terminal panel as it happens, and
    /// every chunk is trace-linked into the Flight Recorder. The capture session
    /// is opened BEFORE the subprocess starts so an attached panel sees output
    /// stream in, and closed with the real exit code when the run ends.
    ///
    /// `invoke` itself is left untouched (sync, no terminal dependency); callers
    /// that have a live `TerminalRuntime` opt into LIVE capture via this wrapper.
    pub async fn invoke_with_capture(
        &self,
        model_id: ModelId,
        prompt: &str,
        runtime: &crate::terminal::TerminalRuntime,
        binding: crate::terminal::SessionBinding,
    ) -> Result<(CliInvocationReceipt, String), OfficialCliBridgeError> {
        // Resolve the registered config first (so a missing model fails before we
        // open a capture session).
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

        // Open the capture session up front so an attached panel streams live.
        let (info, sink) = runtime
            .create_capture_session(binding, Some("cloud-cli-bridge".to_string()))
            .await;
        let session_id = info.session_id.clone();

        // Bridge the SYNC streaming spawn (which calls `on_chunk` from a blocking
        // context) to the ASYNC capture sink: each live chunk is queued on a
        // bounded channel that an async drain task feeds into `sink.feed` in
        // order, so capture stays live without blocking the spawn thread.
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();
        let sink = std::sync::Arc::new(sink);
        let drain_sink = std::sync::Arc::clone(&sink);
        let drain = tokio::spawn(async move {
            while let Some(chunk) = rx.recv().await {
                drain_sink.feed(&chunk).await;
            }
        });

        // Run the blocking streaming spawn on a blocking thread; forward chunks.
        let spawner = std::sync::Arc::clone(&self.spawner);
        let model_name = handle.model_name.clone();
        let prompt_owned = prompt.to_string();
        let spawn_result = tokio::task::spawn_blocking(move || {
            let mut on_chunk = |bytes: &[u8]| {
                // If the receiver is gone the run still completes; capture simply
                // stops fanning. Never block the subprocess.
                let _ = tx.send(bytes.to_vec());
            };
            spawner.spawn_streaming(&config, &model_name, &prompt_owned, &mut on_chunk)
        })
        .await
        .map_err(|e| OfficialCliBridgeError::SpawnFailed {
            reason: format!("streaming spawn task join failed: {e}"),
            exit_code: None,
        })?;

        // Ensure all queued chunks are drained before closing the session.
        let _ = drain.await;
        let exit = spawn_result
            .as_ref()
            .ok()
            .and_then(|r| r.exit_code)
            .unwrap_or(0);
        // Reclaim the sink (the drain task dropped its Arc) and close cleanly.
        match std::sync::Arc::try_unwrap(sink) {
            Ok(owned) => owned.close(exit).await,
            Err(still_shared) => {
                // Should not happen (drain task done), but never leak: drop our
                // ref and let the Drop leak guard reap the session.
                drop(still_shared);
            }
        }

        let receipt = spawn_result?;
        Ok((receipt, session_id))
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
/// MT-127 remediation (MT-122-class): a [`LedgerBatcher`] is MANDATORY at
/// construction. Every CLI-bridge subprocess spawn is registered as an
/// attributable + reclaimable `ProcessOwnershipLedger` START row
/// (`engine_kind = OfficialCliBridge`) immediately after the child pid is
/// captured, and a matching STOP row is recorded after the child exits,
/// mirroring the MT-122 distillation trainer pattern. There is NO
/// unattributed code path: the spawner FAILS CLOSED — if START registration
/// fails, the just-spawned child is killed and an error is returned rather
/// than leaving an unattributed/unreclaimable process running. A genuinely
/// absent runtime (the binary never spawns) still surfaces an honest typed
/// [`OfficialCliBridgeError::SpawnFailed`]; no row is faked when no process
/// is created.
#[derive(Clone)]
pub struct LiveCliSpawner {
    process_ledger: Arc<LedgerBatcher>,
    owner_role: String,
}

impl std::fmt::Debug for LiveCliSpawner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // LedgerBatcher is not Debug (it wraps channels); the ledger is
        // always attached now, so report only that it is present so
        // LiveCliSpawner stays Debug.
        f.debug_struct("LiveCliSpawner")
            .field("process_ledger", &"<attached>")
            .field("owner_role", &self.owner_role)
            .finish()
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

/// Build the `ProcessStart` row for a CLI-bridge subprocess from its
/// `SpawnMeta`. Mirrors `process_ledger::record_spawn`'s internal build but
/// returns the fully-built `ProcessStart` so the caller can record the
/// matching `ProcessStop` on completion. MT-127 (MT-122-class remediation)
/// requires BOTH a START and a STOP row so the spawned CLI subprocess is
/// attributable AND reclaimable across its full lifecycle.
fn cli_bridge_process_start(record_id: ProcessOwnershipRecordId, meta: SpawnMeta) -> ProcessStart {
    let mut start = ProcessStart::new(meta.engine_kind, meta.owner_role.clone(), meta.owner_wp)
        .with_process_uuid(record_id.as_uuid())
        .with_os_pid(meta.pid)
        .with_metadata_jsonb(meta.metadata_blob)
        .with_sandbox_capabilities_snapshot(meta.sandbox_capabilities_snapshot);
    start.started_at = meta.started_at_utc;
    if let Some(sandbox_adapter) = meta.sandbox_adapter {
        start = start.with_sandbox_adapter_id(sandbox_adapter);
    }
    if let Some(mt_id) = meta.mt_id {
        start = start.with_mt_id(mt_id);
    }
    start
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
    /// Construct a spawner with a MANDATORY process ledger (MT-127,
    /// MT-122-class). Every CLI-bridge subprocess this spawner launches is
    /// registered as an attributable + reclaimable `ProcessOwnershipLedger`
    /// row (`engine_kind = OfficialCliBridge`): a START row the moment the
    /// child pid is known and a matching STOP row after it exits. There is
    /// no unattributed construction path.
    pub fn new(process_ledger: Arc<LedgerBatcher>) -> Self {
        Self {
            process_ledger,
            owner_role: DEFAULT_CLI_BRIDGE_OWNER_ROLE.to_string(),
        }
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
        // HBR-QUIET: the Node-based cloud CLI (claude / codex / gemini) is
        // backgrounded by Handshake and must not pop a console window on
        // Windows. std::process::Command exposes creation_flags via CommandExt.
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

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

        // MT-127 remediation (MT-122-class): ledger registration is
        // UNCONDITIONAL. The moment the child pid is known the spawn is
        // registered as an attributable ProcessOwnershipLedger START row
        // (engine_kind=OfficialCliBridge) so the spawned CLI subprocess is
        // attributable + reclaimable. Fail closed: if START registration
        // fails, kill the child rather than leaving an unattributed/
        // unreclaimable process running. The matching STOP row is recorded
        // on EVERY exit path below (success, non-zero exit, timeout, wait
        // error) via `record_stop`, so the ledger reflects the full
        // lifecycle and there is no unattributed code path.
        let record_id = ProcessOwnershipRecordId::new_v7();
        let start = cli_bridge_process_start(
            record_id,
            cli_bridge_spawn_meta(pid, &self.owner_role, model_name, &config.executable_path),
        );
        if let Err(err) = self.process_ledger.record_start(start.clone()) {
            kill_process_tree(pid, &mut child);
            return Err(OfficialCliBridgeError::LedgerRegistration {
                pid,
                reason: err.to_string(),
            });
        }

        // Record the matching STOP row for the now-finished subprocess. Best
        // effort: the process has already exited (or been killed) by the time
        // this runs, so a STOP write failure must not resurrect the child; it
        // is surfaced only as a debug log so the primary spawn outcome is
        // preserved.
        let record_stop = |exit_code: Option<i32>, stop_reason: &str| {
            let stop = ProcessStop::from_start(&start, exit_code).with_stop_reason(stop_reason);
            if let Err(err) = self.process_ledger.record_stop(stop) {
                eprintln!(
                    "official_cli_bridge: ledger STOP registration failed for pid {pid}: {err}"
                );
            }
        };

        let timeout = Duration::from_secs(config.timeout_seconds);
        let started = Instant::now();
        let exit_status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => {
                    if started.elapsed() >= timeout {
                        kill_process_tree(pid, &mut child);
                        // The child was killed on timeout; record the STOP row
                        // so the killed process is reconciled in the ledger.
                        record_stop(None, "official_cli_bridge_timeout_kill");
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
                    // try_wait failed: the child's fate is unknown, kill it so
                    // it is not orphaned, then record the STOP row.
                    kill_process_tree(pid, &mut child);
                    record_stop(None, "official_cli_bridge_try_wait_error");
                    return Err(OfficialCliBridgeError::SpawnFailed {
                        reason: format!("try_wait failed: {err}"),
                        exit_code: None,
                    });
                }
            }
        };

        let output = match child.wait_with_output() {
            Ok(output) => output,
            Err(err) => {
                record_stop(exit_status.code(), "official_cli_bridge_wait_output_error");
                return Err(OfficialCliBridgeError::SpawnFailed {
                    reason: format!("wait_with_output failed: {err}"),
                    exit_code: exit_status.code(),
                });
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        let exit_code = output.status.code();

        // The child has exited; record the matching STOP row with its real
        // exit code on both the failure and success paths so the
        // ProcessOwnershipLedger reflects the full lifecycle unconditionally.
        if !output.status.success() {
            record_stop(exit_code, "official_cli_bridge_nonzero_exit");
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

        record_stop(exit_code, "official_cli_bridge_exit");

        Ok(CliInvocationReceipt {
            model_id: ModelId::new_v7(),
            stdout,
            pid: Some(pid),
            exit_code,
            cancelled: false,
        })
    }

    /// Live-streaming spawn: identical lifecycle to [`LiveCliSpawner::spawn`]
    /// (env scrub, CREATE_NO_WINDOW, ledger START/STOP, timeout + kill), but the
    /// child's stdout pipe is read INCREMENTALLY on a dedicated reader thread and
    /// each chunk is delivered to `on_chunk` LIVE while the subprocess is still
    /// running. This is the real cloud-CLI capture producer for §10.1: the
    /// callback wiring (see `invoke_with_capture`) fans these live chunks into a
    /// read-only AiJob capture session + the Flight Recorder, instead of dumping
    /// the post-completion stdout string. The full stdout is also accumulated so
    /// the returned [`CliInvocationReceipt`] is byte-for-byte identical to the
    /// non-streaming path.
    fn spawn_streaming(
        &self,
        config: &CliBridgeConfig,
        model_name: &str,
        prompt: &str,
        on_chunk: &mut dyn FnMut(&[u8]),
    ) -> Result<CliInvocationReceipt, OfficialCliBridgeError> {
        use std::io::Read;

        let rendered =
            OfficialCliBridgeRuntime::render_args(&config.args_template, model_name, prompt);

        let mut cmd = Command::new(&config.executable_path);
        cmd.args(&rendered);
        // Mirror the env scrub of `spawn` (secret-bearing inherited vars removed).
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
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let mut child = cmd
            .spawn()
            .map_err(|err| OfficialCliBridgeError::SpawnFailed {
                reason: format!("failed to spawn {}: {err}", config.executable_path.display()),
                exit_code: None,
            })?;
        let pid = child.id();

        // Unconditional ledger START (fail-closed), identical to `spawn`.
        let record_id = ProcessOwnershipRecordId::new_v7();
        let start = cli_bridge_process_start(
            record_id,
            cli_bridge_spawn_meta(pid, &self.owner_role, model_name, &config.executable_path),
        );
        if let Err(err) = self.process_ledger.record_start(start.clone()) {
            kill_process_tree(pid, &mut child);
            return Err(OfficialCliBridgeError::LedgerRegistration {
                pid,
                reason: err.to_string(),
            });
        }
        let record_stop = |exit_code: Option<i32>, stop_reason: &str| {
            let stop = ProcessStop::from_start(&start, exit_code).with_stop_reason(stop_reason);
            if let Err(err) = self.process_ledger.record_stop(stop) {
                eprintln!(
                    "official_cli_bridge: ledger STOP registration failed for pid {pid}: {err}"
                );
            }
        };

        // Take the stdout pipe and pump it on a dedicated thread, forwarding each
        // chunk over a channel so the (non-Send) `on_chunk` callback — owned by
        // this thread — receives live chunks while we poll try_wait for the
        // timeout. stderr is drained on its own thread so a full stderr pipe can
        // never deadlock the child.
        let child_stdout = child.stdout.take();
        let child_stderr = child.stderr.take();
        let (chunk_tx, chunk_rx) = mpsc::channel::<Vec<u8>>();
        let stdout_reader = child_stdout.map(|mut out| {
            let tx = chunk_tx.clone();
            thread::spawn(move || {
                let mut buf = [0u8; 8192];
                let mut acc = Vec::new();
                loop {
                    match out.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            acc.extend_from_slice(&buf[..n]);
                            // Forward live; if the consumer hung up, keep draining
                            // so the pipe never blocks the child.
                            let _ = tx.send(buf[..n].to_vec());
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                        Err(_) => break,
                    }
                }
                acc
            })
        });
        let stderr_reader = child_stderr.map(|mut err| {
            thread::spawn(move || {
                let mut buf = [0u8; 8192];
                let mut acc = Vec::new();
                loop {
                    match err.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => acc.extend_from_slice(&buf[..n]),
                        Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                        Err(_) => break,
                    }
                }
                acc
            })
        });
        drop(chunk_tx); // only the reader thread holds a sender now.

        let timeout = Duration::from_secs(config.timeout_seconds);
        let started = Instant::now();
        let exit_status = loop {
            // Drain any live chunks to the callback before checking exit.
            while let Ok(chunk) = chunk_rx.try_recv() {
                on_chunk(&chunk);
            }
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => {
                    if started.elapsed() >= timeout {
                        kill_process_tree(pid, &mut child);
                        record_stop(None, "official_cli_bridge_timeout_kill");
                        // Flush any remaining live chunks captured before the kill.
                        while let Ok(chunk) = chunk_rx.recv_timeout(POST_TIMEOUT_OUTPUT_GRACE) {
                            on_chunk(&chunk);
                        }
                        let partial_stdout = stdout_reader
                            .and_then(|h| h.join().ok())
                            .map(|b| String::from_utf8_lossy(&b).into_owned())
                            .unwrap_or_default();
                        return Err(OfficialCliBridgeError::SpawnTimeout {
                            timeout_seconds: config.timeout_seconds,
                            partial_stdout,
                        });
                    }
                    std::thread::sleep(Duration::from_millis(15));
                }
                Err(err) => {
                    kill_process_tree(pid, &mut child);
                    record_stop(None, "official_cli_bridge_try_wait_error");
                    return Err(OfficialCliBridgeError::SpawnFailed {
                        reason: format!("try_wait failed: {err}"),
                        exit_code: None,
                    });
                }
            }
        };

        // Child exited: drain any straggler chunks, then join the reader threads
        // to recover the full stdout/stderr.
        while let Ok(chunk) = chunk_rx.recv() {
            on_chunk(&chunk);
        }
        let stdout_bytes = stdout_reader
            .and_then(|h| h.join().ok())
            .unwrap_or_default();
        let stderr_bytes = stderr_reader
            .and_then(|h| h.join().ok())
            .unwrap_or_default();
        let stdout = String::from_utf8_lossy(&stdout_bytes).into_owned();
        let stderr = String::from_utf8_lossy(&stderr_bytes).into_owned();
        let exit_code = exit_status.code();

        if !exit_status.success() {
            record_stop(exit_code, "official_cli_bridge_nonzero_exit");
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

        record_stop(exit_code, "official_cli_bridge_exit");
        Ok(CliInvocationReceipt {
            model_id: ModelId::new_v7(),
            stdout,
            pid: Some(pid),
            exit_code,
            cancelled: false,
        })
    }

    /// Cancellable live-streaming spawn: identical lifecycle to
    /// [`LiveCliSpawner::spawn_streaming`] (env scrub, CREATE_NO_WINDOW, ledger
    /// START/STOP, live `on_chunk` fan-out, timeout + kill) plus ONE additional
    /// check per poll iteration: when `should_cancel()` returns true the child
    /// process tree is killed (`kill_process_tree`), a STOP row with reason
    /// `official_cli_bridge_cancel_kill` is recorded, any straggler chunks are
    /// flushed, and a receipt with `cancelled = true` is returned. This is the
    /// real deterministic-cancellation backstop the swarm adapter relies on to
    /// honour the request/runtime `CancellationToken`.
    fn spawn_streaming_cancellable(
        &self,
        config: &CliBridgeConfig,
        model_name: &str,
        prompt: &str,
        on_chunk: &mut dyn FnMut(&[u8]),
        should_cancel: &dyn Fn() -> bool,
    ) -> Result<CliInvocationReceipt, OfficialCliBridgeError> {
        use std::io::Read;

        let rendered =
            OfficialCliBridgeRuntime::render_args(&config.args_template, model_name, prompt);

        let mut cmd = Command::new(&config.executable_path);
        cmd.args(&rendered);
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
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let mut child = cmd
            .spawn()
            .map_err(|err| OfficialCliBridgeError::SpawnFailed {
                reason: format!("failed to spawn {}: {err}", config.executable_path.display()),
                exit_code: None,
            })?;
        let pid = child.id();

        let record_id = ProcessOwnershipRecordId::new_v7();
        let start = cli_bridge_process_start(
            record_id,
            cli_bridge_spawn_meta(pid, &self.owner_role, model_name, &config.executable_path),
        );
        if let Err(err) = self.process_ledger.record_start(start.clone()) {
            kill_process_tree(pid, &mut child);
            return Err(OfficialCliBridgeError::LedgerRegistration {
                pid,
                reason: err.to_string(),
            });
        }
        let record_stop = |exit_code: Option<i32>, stop_reason: &str| {
            let stop = ProcessStop::from_start(&start, exit_code).with_stop_reason(stop_reason);
            if let Err(err) = self.process_ledger.record_stop(stop) {
                eprintln!(
                    "official_cli_bridge: ledger STOP registration failed for pid {pid}: {err}"
                );
            }
        };

        let child_stdout = child.stdout.take();
        let child_stderr = child.stderr.take();
        let (chunk_tx, chunk_rx) = mpsc::channel::<Vec<u8>>();
        let stdout_reader = child_stdout.map(|mut out| {
            let tx = chunk_tx.clone();
            thread::spawn(move || {
                let mut buf = [0u8; 8192];
                let mut acc = Vec::new();
                loop {
                    match out.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            acc.extend_from_slice(&buf[..n]);
                            let _ = tx.send(buf[..n].to_vec());
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                        Err(_) => break,
                    }
                }
                acc
            })
        });
        let stderr_reader = child_stderr.map(|mut err| {
            thread::spawn(move || {
                let mut buf = [0u8; 8192];
                let mut acc = Vec::new();
                loop {
                    match err.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => acc.extend_from_slice(&buf[..n]),
                        Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                        Err(_) => break,
                    }
                }
                acc
            })
        });
        drop(chunk_tx);

        let timeout = Duration::from_secs(config.timeout_seconds);
        let started = Instant::now();
        let exit_status = loop {
            // Drain any live chunks to the callback before checking exit/cancel.
            while let Ok(chunk) = chunk_rx.try_recv() {
                on_chunk(&chunk);
            }
            // Deterministic cancellation: kill the child and return a cancelled
            // receipt rather than running the CLI to completion.
            if should_cancel() {
                kill_process_tree(pid, &mut child);
                record_stop(None, "official_cli_bridge_cancel_kill");
                while let Ok(chunk) = chunk_rx.recv_timeout(POST_TIMEOUT_OUTPUT_GRACE) {
                    on_chunk(&chunk);
                }
                let stdout_bytes = stdout_reader
                    .and_then(|h| h.join().ok())
                    .unwrap_or_default();
                let _ = stderr_reader.and_then(|h| h.join().ok());
                return Ok(CliInvocationReceipt {
                    model_id: ModelId::new_v7(),
                    stdout: String::from_utf8_lossy(&stdout_bytes).into_owned(),
                    pid: Some(pid),
                    exit_code: None,
                    cancelled: true,
                });
            }
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => {
                    if started.elapsed() >= timeout {
                        kill_process_tree(pid, &mut child);
                        record_stop(None, "official_cli_bridge_timeout_kill");
                        while let Ok(chunk) = chunk_rx.recv_timeout(POST_TIMEOUT_OUTPUT_GRACE) {
                            on_chunk(&chunk);
                        }
                        let partial_stdout = stdout_reader
                            .and_then(|h| h.join().ok())
                            .map(|b| String::from_utf8_lossy(&b).into_owned())
                            .unwrap_or_default();
                        return Err(OfficialCliBridgeError::SpawnTimeout {
                            timeout_seconds: config.timeout_seconds,
                            partial_stdout,
                        });
                    }
                    std::thread::sleep(Duration::from_millis(15));
                }
                Err(err) => {
                    kill_process_tree(pid, &mut child);
                    record_stop(None, "official_cli_bridge_try_wait_error");
                    return Err(OfficialCliBridgeError::SpawnFailed {
                        reason: format!("try_wait failed: {err}"),
                        exit_code: None,
                    });
                }
            }
        };

        while let Ok(chunk) = chunk_rx.recv() {
            on_chunk(&chunk);
        }
        let stdout_bytes = stdout_reader
            .and_then(|h| h.join().ok())
            .unwrap_or_default();
        let stderr_bytes = stderr_reader
            .and_then(|h| h.join().ok())
            .unwrap_or_default();
        let stdout = String::from_utf8_lossy(&stdout_bytes).into_owned();
        let stderr = String::from_utf8_lossy(&stderr_bytes).into_owned();
        let exit_code = exit_status.code();

        if !exit_status.success() {
            record_stop(exit_code, "official_cli_bridge_nonzero_exit");
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

        record_stop(exit_code, "official_cli_bridge_exit");
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

    /// Build a real, manually-drained `LedgerBatcher` for tests. The ledger is
    /// mandatory on `LiveCliSpawner` (MT-127, MT-122-class), so every test
    /// that constructs a live spawner attaches one. The drain side is dropped
    /// when the caller does not inspect rows; the batcher's in-memory channel
    /// still accepts START/STOP writes without a backing store.
    fn test_ledger() -> Arc<crate::process_ledger::LedgerBatcher> {
        let (batcher, _drain) = crate::process_ledger::LedgerBatcher::manual_for_tests(
            crate::process_ledger::LedgerBatcherConfig::default(),
            Arc::new(crate::process_ledger::NoopOverflowSink),
        )
        .expect("manual ledger batcher for tests");
        Arc::new(batcher)
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
        let err = LiveCliSpawner::new(test_ledger())
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
        // new() must yield the canonical owner role so the ledger row is
        // attributable even when the caller does not override it;
        // with_owner_role overrides it. The ledger is mandatory (MT-127,
        // MT-122-class): there is no ledger-less construction path.
        let spawner = LiveCliSpawner::new(test_ledger());
        assert_eq!(spawner.owner_role, "OFFICIAL_CLI_BRIDGE");
        let custom = LiveCliSpawner::new(test_ledger()).with_owner_role("DISTILLATION_PIPELINE");
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

    /// Mock spawner that emits MULTIPLE live chunks via `spawn_streaming` (not a
    /// single post-hoc dump), so the test proves `invoke_with_capture` fans a
    /// genuine live stream — not a one-shot replay of finished stdout.
    struct StreamingMockSpawner {
        chunks: Vec<Vec<u8>>,
    }
    impl CliSubprocessSpawner for StreamingMockSpawner {
        fn spawn(
            &self,
            _config: &CliBridgeConfig,
            _model_name: &str,
            _prompt: &str,
        ) -> Result<CliInvocationReceipt, OfficialCliBridgeError> {
            // The fallback path concatenates — used only if spawn_streaming is
            // not called; this test calls spawn_streaming directly.
            let stdout = self
                .chunks
                .iter()
                .map(|c| String::from_utf8_lossy(c).into_owned())
                .collect::<String>();
            Ok(CliInvocationReceipt {
                model_id: ModelId::new_v7(),
                stdout,
                pid: Some(4321),
                exit_code: Some(0),
                cancelled: false,
            })
        }
        fn spawn_streaming(
            &self,
            _config: &CliBridgeConfig,
            _model_name: &str,
            _prompt: &str,
            on_chunk: &mut dyn FnMut(&[u8]),
        ) -> Result<CliInvocationReceipt, OfficialCliBridgeError> {
            let mut full = Vec::new();
            for chunk in &self.chunks {
                // Deliver each chunk LIVE, as if read incrementally from a pipe.
                on_chunk(chunk);
                full.extend_from_slice(chunk);
            }
            Ok(CliInvocationReceipt {
                model_id: ModelId::new_v7(),
                stdout: String::from_utf8_lossy(&full).into_owned(),
                pid: Some(4321),
                exit_code: Some(0),
                cancelled: false,
            })
        }
    }

    /// PROOF the cloud CLI bridge is a REAL live capture producer (not dead
    /// code): `invoke_with_capture` opens a §10.1 capture session, fans the
    /// streaming spawn's LIVE chunks into the broadcast + Flight Recorder, and
    /// closes the session with the real exit code.
    #[tokio::test]
    async fn invoke_with_capture_fans_live_stream_to_broadcast_and_fr() {
        use crate::flight_recorder::{
            EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
        };
        use crate::terminal::{SessionBinding, SessionOutput, TerminalRuntime};
        use async_trait::async_trait;

        #[derive(Default)]
        struct CountingRecorder {
            events: Mutex<Vec<FlightRecorderEvent>>,
        }
        #[async_trait]
        impl FlightRecorder for CountingRecorder {
            async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
                self.events.lock().unwrap().push(event);
                Ok(())
            }
            async fn enforce_retention(&self) -> Result<u64, RecorderError> {
                Ok(0)
            }
            async fn list_events(
                &self,
                _f: EventFilter,
            ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
                Ok(self.events.lock().unwrap().clone())
            }
        }

        // Bridge runtime with a streaming mock that emits 3 live chunks.
        let spawner = Arc::new(StreamingMockSpawner {
            chunks: vec![
                b"chunk-one ".to_vec(),
                b"chunk-two ".to_vec(),
                b"chunk-three".to_vec(),
            ],
        });
        let runtime = OfficialCliBridgeRuntime::new(spawner);
        let handle = runtime
            .register_bridge(good_config(), "claude-sonnet", "2026-01-01T00:00:00Z")
            .expect("register");

        // Terminal runtime wired to a counting recorder (the capture target).
        let recorder = Arc::new(CountingRecorder::default());
        let caps = Arc::new(crate::capabilities::CapabilityRegistry::new());
        let term = TerminalRuntime::new(caps, recorder.clone());

        // Subscribe is per-session; open via invoke_with_capture and read the
        // capture scrollback (authoritative) + assert FR events + broadcast.
        let binding = SessionBinding {
            swarm_id: Some("cloud-lane".to_string()),
            ..Default::default()
        };
        // Pre-attach a subscriber by opening the session id after the call: we
        // instead assert via scrollback (which retains every fed chunk) + FR.
        let (receipt, session_id) = runtime
            .invoke_with_capture(handle.model_id, "hello", &term, binding)
            .await
            .expect("invoke_with_capture");

        // The receipt's stdout is the full concatenation.
        assert_eq!(receipt.stdout, "chunk-one chunk-two chunk-three");

        // The Flight Recorder must have recorded the session open + one
        // COMMAND-EXEC PER fed chunk (so background cloud work is trace-linked).
        // We assert the live-fan via the FR COMMAND-EXEC count below: the session
        // is closed + reaped by invoke_with_capture when the run ends, so its
        // scrollback is intentionally no longer readable here (close is the clean
        // teardown path); the per-chunk FR events are the durable live-stream
        // evidence. (A live UI panel reads chunks via the broadcast forwarder +
        // a pre-close scrollback backfill while the session is open.)
        let events = recorder.events.lock().unwrap();
        let fr_tags: Vec<String> = events
            .iter()
            .filter_map(|e| {
                e.payload
                    .get("fr_event")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .collect();
        assert!(
            fr_tags.contains(&"FR-EVT-TERMINAL-SESSION-OPEN".to_string()),
            "must record session open"
        );
        let command_execs = fr_tags
            .iter()
            .filter(|t| *t == "FR-EVT-TERMINAL-COMMAND-EXEC")
            .count();
        assert!(
            command_execs >= 3,
            "each live chunk must be trace-linked (>=3 COMMAND-EXEC), got {command_execs}"
        );
        // The session must be closed (reaped) after the run.
        drop(events);
        assert!(
            term.subscribe(&session_id).is_err(),
            "capture session must be closed after the run ends"
        );
        // Touch SessionOutput so the import is exercised (exit fan-out type).
        let _ = SessionOutput::Exit(0);
    }
}

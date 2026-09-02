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

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitStatus;
use std::sync::{mpsc, Arc, RwLock};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(windows)]
use std::os::windows::fs::OpenOptionsExt;

use serde_json::json;
use sha2::{Digest, Sha256};
use thiserror::Error;
use zeroize::{Zeroize, Zeroizing};

use crate::model_runtime::{ModelCapabilities, ModelId};
use crate::process_ledger::{
    ActiveProcessLifecycle, LedgerBatcher, ProcessEngineKind, ProcessOwnershipRecordId,
    ProcessStart, SpawnMeta, StopRecordOutcome,
};
#[cfg(target_os = "windows")]
use crate::process_ledger::{
    ProcessLedgerDurabilityAck, ProcessLedgerError, ReservedProcessLifecycle,
};
use crate::sandbox::{
    select, AdapterCapabilities, AdapterId, AttachedNetworkMode, AttachedProcessSpec,
    AttachedSandboxProcess, AttachedStdioContract, BindMode, BindSpec,
    HandshakeNativeSandboxAdapter, ImageRef, IsolationTier, NetPolicy, ProcessSpec,
    RequiredCapability, ResourceLimits, SandboxAdapter, SandboxAdapterError,
    SandboxAdapterRegistry, TrustClass, HANDSHAKE_NATIVE_SANDBOX_ADAPTER_ID,
};

fn paths_match_checkout_identity(left: &Path, right: &Path) -> bool {
    if cfg!(windows) {
        left.to_string_lossy()
            .eq_ignore_ascii_case(right.to_string_lossy().as_ref())
    } else {
        left == right
    }
}

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
    #[error("official CLI command script is not a validated Codex npm shim: {0}")]
    UnsupportedCommandScript(PathBuf),
    #[error("official CLI executable identity could not be pinned or revalidated: {0}")]
    ExecutableIdentity(String),
    #[error("Codex CLI bridge configuration is not the canonical exec JSONL preset: {0}")]
    InvalidCodexPreset(String),
    #[error("official CLI model binding is invalid: {0}")]
    InvalidModelBinding(String),
    #[error("Codex CLI state root is unavailable to the attached sandbox: {0}")]
    CodexHomeUnavailable(String),
    #[error("official CLI environment variable is not allowed at the process boundary: {0}")]
    UnsafeEnvironmentVariable(String),
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
    #[error(
        "ProcessOwnershipLedger authority unavailable before CLI bridge subprocess spawn: {reason}; no subprocess was started"
    )]
    LedgerPreflight { reason: String },
}

/// Attribution and recovery identity for one concrete CLI invocation. This is
/// deliberately supplied at the call boundary: a reusable spawner must never
/// invent ownership from construction-time defaults.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CliInvocationContext {
    /// Canonical registered runtime identity, injected by
    /// `OfficialCliBridgeRuntime` immediately before dispatch.
    pub registered_model_id: Option<ModelId>,
    pub owner_role: String,
    pub owner_wp: Option<String>,
    pub role_id: Option<String>,
    pub wp_id: Option<String>,
    pub mt_id: Option<String>,
    pub session_id: Option<String>,
    pub parent_session_id: Option<String>,
    pub parent_process_id: Option<uuid::Uuid>,
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
    pub cancellation_id: Option<String>,
    pub reclaim_key: Option<String>,
    pub model_identity: String,
    pub requested_trust_class: Option<TrustClass>,
    pub requested_isolation_tier: Option<IsolationTier>,
    pub requested_sandbox_capabilities: Option<BTreeSet<RequiredCapability>>,
    pub requested_net_policy: Option<NetPolicy>,
    pub requested_execution_policy_ref: Option<String>,
    pub swarm_id: Option<String>,
    pub worktree_id: Option<String>,
    pub working_dir: Option<String>,
    pub checkout_lease_id: Option<uuid::Uuid>,
    pub checkout_lease_owner_generation: Option<u64>,
    pub checkout_lease_canonical_working_dir: Option<String>,
}

impl CliInvocationContext {
    pub fn new(owner_role: impl Into<String>, model_identity: impl Into<String>) -> Self {
        Self {
            registered_model_id: None,
            owner_role: owner_role.into(),
            owner_wp: None,
            role_id: None,
            wp_id: None,
            mt_id: None,
            session_id: None,
            parent_session_id: None,
            parent_process_id: None,
            trace_id: None,
            span_id: None,
            cancellation_id: None,
            reclaim_key: None,
            model_identity: model_identity.into(),
            requested_trust_class: None,
            requested_isolation_tier: None,
            requested_sandbox_capabilities: None,
            requested_net_policy: None,
            requested_execution_policy_ref: None,
            swarm_id: None,
            worktree_id: None,
            working_dir: None,
            checkout_lease_id: None,
            checkout_lease_owner_generation: None,
            checkout_lease_canonical_working_dir: None,
        }
    }
}

/// Concrete cancellation inputs for one invocation. Polling these project-owned
/// tokens is bounded and cannot execute caller callbacks on the process loop.
#[derive(Clone, Default)]
pub struct CliCancellationContext {
    tokens: Vec<crate::model_runtime::CancellationToken>,
}

impl CliCancellationContext {
    pub fn new(tokens: Vec<crate::model_runtime::CancellationToken>) -> Self {
        Self { tokens }
    }

    pub fn is_cancelled(&self) -> bool {
        self.tokens.iter().any(|token| token.is_cancelled())
    }
}

/// Abstraction over the CLI subprocess lifecycle. Test implementations can
/// substitute deterministic spawners; the production implementation owns the
/// ProcessOwnershipLedger row with `engine_kind=OfficialCliBridge`.
pub trait CliSubprocessSpawner: Send + Sync {
    /// Pin the executable graph while a bridge is registered. Production
    /// spawners revalidate the same graph immediately before every launch;
    /// deterministic test spawners may keep the no-op default.
    fn pin_config(&self, _config: &CliBridgeConfig) -> Result<(), OfficialCliBridgeError> {
        Ok(())
    }

    fn spawn(
        &self,
        config: &CliBridgeConfig,
        invocation: &CliInvocationContext,
        model_name: &str,
        prompt: &str,
    ) -> Result<CliInvocationReceipt, OfficialCliBridgeError>;

    /// Spawn the CLI subprocess and stream its piped stdout LIVE through a
    /// bounded, nonblocking sender. The process polling path never invokes
    /// caller code and therefore remains responsive to timeout/cancellation.
    /// This is what lets the §10.1
    /// capture seam attach a real live background stream rather than a post-hoc
    /// dump. The default impl falls back to [`CliSubprocessSpawner::spawn`] and
    /// replays the captured stdout once (so mock spawners without a real pipe
    /// still work); [`LiveCliSpawner`] overrides it with a true incremental
    /// pipe reader.
    fn spawn_streaming(
        &self,
        config: &CliBridgeConfig,
        invocation: &CliInvocationContext,
        model_name: &str,
        prompt: &str,
        chunk_sender: &tokio::sync::mpsc::Sender<Vec<u8>>,
    ) -> Result<CliInvocationReceipt, OfficialCliBridgeError> {
        let receipt = self.spawn(config, invocation, model_name, prompt)?;
        if !receipt.stdout.is_empty() {
            deliver_cli_chunk(chunk_sender, receipt.stdout.as_bytes())?;
        }
        Ok(receipt)
    }

    /// Cancellable variant of [`CliSubprocessSpawner::spawn_streaming`]: in
    /// addition to live chunk fan-out it consults the concrete cancellation set
    /// between reads and, when it observes a set cancellation, kills the child
    /// subprocess and returns a receipt with `cancelled = true` rather than
    /// running the CLI to completion. This is what lets the swarm
    /// `CliBridgeModelRuntime` honour the request/runtime `CancellationToken`
    /// by actually killing the backing process (poll-based token — see
    /// [`crate::model_runtime::CancellationToken`]).
    ///
    /// The DEFAULT impl ignores cancellation and delegates to
    /// [`CliSubprocessSpawner::spawn_streaming`], so existing mock/test spawners
    /// that do not override it keep compiling and behaving exactly as before.
    /// [`LiveCliSpawner`] overrides it with a real per-iteration kill check.
    fn spawn_streaming_cancellable(
        &self,
        config: &CliBridgeConfig,
        invocation: &CliInvocationContext,
        model_name: &str,
        prompt: &str,
        chunk_sender: &tokio::sync::mpsc::Sender<Vec<u8>>,
        _cancellation: &CliCancellationContext,
    ) -> Result<CliInvocationReceipt, OfficialCliBridgeError> {
        self.spawn_streaming(config, invocation, model_name, prompt, chunk_sender)
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

/// Reduced output from an auxiliary provider-owned CLI command. Stderr never
/// crosses this boundary because provider diagnostics may contain account or
/// credential metadata. The caller must reduce stdout to a typed status and
/// drop it immediately.
pub(crate) struct AuxiliaryCliCommandOutput {
    pub(crate) success: bool,
    pub(crate) stdout: Zeroizing<Vec<u8>>,
}

/// Live, operator-drivable transport for one provider-owned interactive login.
///
/// The operator-facing surface (the native Settings cloud-access panel, through
/// the `model-access` routes) only ever sees this trait: a pid receipt, the
/// bounded transcript the provider has printed so far, a way to type an answer,
/// and the terminal exit state. The executable path, argv, and environment
/// deliberately never cross the backend boundary, exactly as before.
///
/// The production implementation is [`InteractiveLoginPty`], which runs the
/// provider's own login command under a Handshake-hosted pseudo-terminal so the
/// interactive device/OAuth flow is completable WITHOUT an OS console window
/// (HBR-QUIET-001). Route-level tests substitute an in-memory transport, so
/// they exercise the HTTP contract without invoking an installed CLI.
pub trait InteractiveLoginTransport: Send + Sync + std::fmt::Debug {
    /// OS pid of the provider's login process (non-secret receipt).
    fn pid(&self) -> u32;
    /// Raw bytes the provider has written to the terminal so far, bounded by
    /// the session's scrollback cap.
    fn transcript(&self) -> Vec<u8>;
    /// Send one operator response (already newline-terminated by the caller)
    /// to the login process's stdin.
    fn write_input(&self, bytes: &[u8]) -> Result<(), String>;
    /// Terminal exit code, or `None` while the login is still running.
    fn exit_code(&self) -> Option<i32>;
    /// Terminate the login process. Idempotent.
    fn cancel(&self);
}

/// PTY-backed interactive login process.
///
/// Holds a shared handle to the [`crate::terminal::PtySession`] that owns the
/// provider's login child. The session's watcher thread owns the process-ledger
/// STOP record; this handle is read/write access only, so dropping it never
/// bypasses ledger ownership.
#[cfg(target_os = "windows")]
pub struct InteractiveLoginPty {
    pid: u32,
    session: Arc<crate::terminal::PtySession>,
}

#[cfg(target_os = "windows")]
impl std::fmt::Debug for InteractiveLoginPty {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The transcript is deliberately absent: provider login output may carry
        // one-time codes and must never reach a Debug/log surface.
        formatter
            .debug_struct("InteractiveLoginPty")
            .field("pid", &self.pid)
            .finish()
    }
}

#[cfg(target_os = "windows")]
impl InteractiveLoginTransport for InteractiveLoginPty {
    fn pid(&self) -> u32 {
        self.pid
    }

    fn transcript(&self) -> Vec<u8> {
        self.session.scrollback()
    }

    fn write_input(&self, bytes: &[u8]) -> Result<(), String> {
        self.session
            .write_stdin(bytes)
            .map_err(|error| error.to_string())
    }

    fn exit_code(&self) -> Option<i32> {
        self.session.exit_code()
    }

    fn cancel(&self) {
        self.session.kill();
    }
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
        validate_cli_executable_path(&config.executable_path)?;
        validate_config_environment(&config.env_vars)?;
        validate_cli_bridge_config_contract(&config)?;
        self.spawner.pin_config(&config)?;
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
            ..Default::default()
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
        invocation: &CliInvocationContext,
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
        let mut effective_invocation = invocation.clone();
        effective_invocation.registered_model_id = Some(model_id);
        let mut receipt =
            self.spawner
                .spawn(&config, &effective_invocation, &handle.model_name, prompt)?;
        receipt.model_id = model_id;
        Ok(receipt)
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
        invocation: CliInvocationContext,
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

        // Bridge the SYNC streaming spawn to the ASYNC capture sink: each live chunk is queued on a
        // bounded channel that an async drain task feeds into `sink.feed` in
        // order, so capture stays live without blocking the spawn thread.
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(MAX_PENDING_STREAM_CHUNKS);
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
        let mut effective_invocation = invocation;
        effective_invocation.registered_model_id = Some(model_id);
        let spawn_result = tokio::task::spawn_blocking(move || {
            spawner.spawn_streaming(
                &config,
                &effective_invocation,
                &model_name,
                &prompt_owned,
                &tx,
            )
        })
        .await
        .map_err(|e| OfficialCliBridgeError::SpawnFailed {
            reason: format!("streaming spawn task join failed: {e}"),
            exit_code: None,
        })?;

        // Ensure all queued chunks are drained before closing the session.
        let _ = drain.await;
        let exit = terminal_capture_exit_code(&spawn_result);
        // Reclaim the sink (the drain task dropped its Arc) and close cleanly.
        match std::sync::Arc::try_unwrap(sink) {
            Ok(owned) => owned.close(exit).await,
            Err(still_shared) => {
                // Should not happen (drain task done), but never leak: drop our
                // ref and let the Drop leak guard reap the session.
                drop(still_shared);
            }
        }

        let mut receipt = spawn_result?;
        receipt.model_id = model_id;
        Ok((receipt, session_id))
    }
}

fn terminal_capture_exit_code(
    spawn_result: &Result<CliInvocationReceipt, OfficialCliBridgeError>,
) -> i32 {
    match spawn_result {
        Ok(receipt) => receipt.exit_code.unwrap_or(0),
        Err(OfficialCliBridgeError::SpawnFailed { exit_code, .. }) => exit_code.unwrap_or(-1),
        Err(_) => -1,
    }
}

/// Production `CliSubprocessSpawner` that drives a real subprocess through an
/// attached [`SandboxAdapter`]. The adapter owns process-tree containment,
/// termination, and reaping; this layer owns bounded output delivery and the
/// durable ProcessOwnershipLedger lifecycle.
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
    sandbox_registry: Arc<SandboxAdapterRegistry>,
    /// MT-019 F1: the RUNNING app's own reaper for CLI children whose STOP could
    /// not be proven. Optional because several composition roots build a spawner
    /// without a reclaim runtime; when absent the open row is simply left for the
    /// boot/periodic restart pass, exactly as before.
    reclaim: Option<Arc<crate::process_ledger::Reclaim>>,
    pinned_identities: Arc<RwLock<HashMap<PathBuf, CliLaunchIdentity>>>,
    /// Backend-owned data root used only by non-authenticating version probes.
    /// Normal model invocations always use the operator's persisted CLI home.
    preflight_codex_home: Option<Arc<PreflightDataRoot>>,
}

#[derive(Debug)]
struct PreflightDataRoot {
    path: PathBuf,
    transferred_to_startup: std::sync::atomic::AtomicBool,
}

impl Drop for PreflightDataRoot {
    fn drop(&mut self) {
        if self
            .transferred_to_startup
            .load(std::sync::atomic::Ordering::Acquire)
        {
            return;
        }
        if let Err(error) = std::fs::remove_dir_all(&self.path) {
            if error.kind() != std::io::ErrorKind::NotFound {
                tracing::error!(
                    target: "handshake_core::official_cli_bridge",
                    path = %self.path.display(),
                    error = %error,
                    "isolated preflight data-root cleanup failed"
                );
            }
        }
    }
}

impl std::fmt::Debug for LiveCliSpawner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // LedgerBatcher is not Debug (it wraps channels); the ledger is
        // always attached now, so report only that it is present so
        // LiveCliSpawner stays Debug.
        f.debug_struct("LiveCliSpawner")
            .field("process_ledger", &"<attached>")
            .field("sandbox_governance_attached", &true)
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
    invocation: &CliInvocationContext,
    selected_model_name: &str,
    identity: &CliLaunchIdentity,
) -> SpawnMeta {
    let mut meta = SpawnMeta::new(
        pid,
        ProcessEngineKind::OfficialCliBridge,
        invocation.owner_role.clone(),
    );
    meta.model_id = invocation
        .registered_model_id
        .map(|value| value.to_string());
    meta.model_identity = Some(selected_model_name.to_string());
    meta.owner_wp = invocation.owner_wp.clone();
    meta.role_id = invocation.role_id.clone();
    meta.wp_id = invocation.wp_id.clone();
    meta.mt_id = invocation.mt_id.clone();
    meta.session_id = invocation.session_id.clone();
    meta.parent_session_id = invocation.parent_session_id.clone();
    meta.parent_process_id = invocation.parent_process_id;
    meta.trace_id = invocation.trace_id.clone();
    meta.span_id = invocation.span_id.clone();
    meta.cancellation_id = invocation.cancellation_id.clone();
    meta.reclaim_key = invocation.reclaim_key.clone();
    meta.metadata_blob = json!({
        "subprocess_kind": "official_cli_bridge",
        "selected_model_name": selected_model_name,
        "registered_model_id": invocation.registered_model_id,
        "model_identity": selected_model_name,
        "requested_model_identity": invocation.model_identity,
        "owner_role": invocation.owner_role,
        "owner_wp": invocation.owner_wp,
        "role_id": invocation.role_id,
        "wp_id": invocation.wp_id,
        "mt_id": invocation.mt_id,
        "session_id": invocation.session_id,
        "trace_id": invocation.trace_id,
        "span_id": invocation.span_id,
        "cancellation_id": invocation.cancellation_id,
        "reclaim_key": invocation.reclaim_key,
        "requested_trust_class": invocation.requested_trust_class,
        "requested_isolation_tier": invocation.requested_isolation_tier,
        "requested_sandbox_capabilities": invocation.requested_sandbox_capabilities,
        "requested_net_policy": invocation.requested_net_policy,
        "requested_execution_policy_ref": invocation.requested_execution_policy_ref,
        "swarm_id": invocation.swarm_id,
        "worktree_id": invocation.worktree_id,
        "working_dir": invocation.working_dir,
        "requested_swarm_id": invocation.swarm_id,
        "requested_worktree_id": invocation.worktree_id,
        "requested_working_dir": invocation.working_dir,
        "checkout_lease_id": invocation.checkout_lease_id,
        "checkout_lease_owner_generation": invocation.checkout_lease_owner_generation,
        "checkout_lease_canonical_working_dir": invocation.checkout_lease_canonical_working_dir,
        "requested_entrypoint": identity.requested_entrypoint.canonical_path.display().to_string(),
        "requested_entrypoint_sha256": identity.requested_entrypoint.sha256,
        "effective_executable": identity.effective_executable.canonical_path.display().to_string(),
        "effective_executable_sha256": identity.effective_executable.sha256,
        "effective_script": identity.effective_script.as_ref().map(|value| value.canonical_path.display().to_string()),
        "effective_script_sha256": identity.effective_script.as_ref().map(|value| value.sha256.clone()),
        "launcher_package_manifest": identity.launcher_package_manifest.as_ref().map(|value| value.canonical_path.display().to_string()),
        "launcher_package_manifest_sha256": identity.launcher_package_manifest.as_ref().map(|value| value.sha256.clone()),
        "platform_package_manifest": identity.platform_package_manifest.as_ref().map(|value| value.canonical_path.display().to_string()),
        "platform_package_manifest_sha256": identity.platform_package_manifest.as_ref().map(|value| value.sha256.clone()),
        "final_native_executable": identity.final_native_executable.as_ref().map(|value| value.canonical_path.display().to_string()),
        "final_native_executable_sha256": identity.final_native_executable.as_ref().map(|value| value.sha256.clone()),
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
    if let Some(parent_session_id) = meta.parent_session_id {
        start = start.with_parent_session_id(parent_session_id);
    }
    if let Some(parent_process_id) = meta.parent_process_id {
        start = start.with_parent_process_id(parent_process_id);
    }
    if let Some(role_id) = meta.role_id {
        start = start.with_role_id(role_id);
    }
    if let Some(wp_id) = meta.wp_id {
        start = start.with_wp_id(wp_id);
    }
    if let Some(mt_id) = meta.mt_id {
        start = start.with_mt_id(mt_id);
    }
    start
}

/// Parent environment is projected through an explicit runtime allowlist.
/// Provider credentials are never inherited implicitly; an operator-approved
/// `CliBridgeConfig.env_vars` entry is the only path for additional values.
fn is_inherited_runtime_env_name(name: &str) -> bool {
    const ALLOWED: &[&str] = &[
        "PATH",
        "PATHEXT",
        "SYSTEMROOT",
        "WINDIR",
        "COMSPEC",
        "USERPROFILE",
        "HOME",
        "APPDATA",
        "LOCALAPPDATA",
        "TEMP",
        "TMP",
        "LANG",
        "LC_ALL",
        "LC_CTYPE",
        "TERM",
        "COLORTERM",
        "NO_COLOR",
        "XDG_CONFIG_HOME",
        "XDG_CACHE_HOME",
        "XDG_DATA_HOME",
        "CODEX_HOME",
    ];
    ALLOWED
        .iter()
        .any(|allowed| name.eq_ignore_ascii_case(allowed))
}

/// The credential-free parent environment shared by official-CLI inference and
/// auxiliary provider-owned commands such as auth status. API keys, auth
/// tokens, endpoints, proxies, and interpreter controls are excluded.
pub(crate) fn inherited_official_cli_environment() -> BTreeMap<String, String> {
    std::env::vars()
        .filter(|(name, _)| is_inherited_runtime_env_name(name))
        .collect()
}

/// Even explicit config must not be able to alter interpreter startup or route
/// credentials through executable ask-pass hooks. Provider API tokens remain
/// allowed when deliberately supplied because they are data, not code-loading
/// controls.
fn is_execution_control_env_name(name: &str) -> bool {
    const FORBIDDEN: &[&str] = &[
        "PATH",
        "PATHEXT",
        "COMSPEC",
        "SYSTEMROOT",
        "WINDIR",
        "NODE_OPTIONS",
        "NODE_PATH",
        "DOTNET_STARTUP_HOOKS",
        "DOTNET_ADDITIONAL_DEPS",
        "DOTNET_SHARED_STORE",
        "COREHOST_TRACEFILE",
        "LD_PRELOAD",
        "LD_LIBRARY_PATH",
        "DYLD_INSERT_LIBRARIES",
        "DYLD_LIBRARY_PATH",
        "PYTHONPATH",
        "PYTHONSTARTUP",
        "RUBYOPT",
        "PERL5OPT",
        "BASH_ENV",
        "ENV",
        "PROMPT_COMMAND",
        "GIT_ASKPASS",
        "SSH_ASKPASS",
        "SUDO_ASKPASS",
    ];
    FORBIDDEN
        .iter()
        .any(|forbidden| name.eq_ignore_ascii_case(forbidden))
}

/// Explicit bridge environment is presentation-only. Official subscription
/// authentication remains owned by the installed CLI profile; credentials,
/// endpoints, proxies, executable resolution, loaders, and startup controls
/// are never accepted through persisted bridge configuration.
fn is_explicit_cli_data_env_name(name: &str) -> bool {
    const EXACT: &[&str] = &[
        "LANG",
        "LC_ALL",
        "LC_CTYPE",
        "TERM",
        "COLORTERM",
        "NO_COLOR",
    ];
    let upper = name.to_ascii_uppercase();
    EXACT.contains(&upper.as_str())
}

fn validate_config_environment(
    env: &HashMap<String, String>,
) -> Result<(), OfficialCliBridgeError> {
    if let Some(name) = env.keys().find(|name| is_execution_control_env_name(name)) {
        return Err(OfficialCliBridgeError::UnsafeEnvironmentVariable(
            name.clone(),
        ));
    }
    if let Some(name) = env.keys().find(|name| !is_explicit_cli_data_env_name(name)) {
        return Err(OfficialCliBridgeError::UnsafeEnvironmentVariable(
            name.clone(),
        ));
    }
    Ok(())
}

fn is_windows_command_script(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            extension.eq_ignore_ascii_case("cmd") || extension.eq_ignore_ascii_case("bat")
        })
        .unwrap_or(false)
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FileIdentity {
    canonical_path: PathBuf,
    sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CliLaunchIdentity {
    requested_entrypoint: FileIdentity,
    effective_executable: FileIdentity,
    effective_script: Option<FileIdentity>,
    launcher_package_manifest: Option<FileIdentity>,
    platform_package_manifest: Option<FileIdentity>,
    final_native_executable: Option<FileIdentity>,
}

#[derive(Debug)]
struct CliLaunchPlan {
    executable_path: PathBuf,
    args: Vec<String>,
    identity: CliLaunchIdentity,
    read_only_roots: Vec<PathBuf>,
    /// Windows files opened without write/delete sharing. Keeping these
    /// handles alive through the child lifecycle prevents path replacement
    /// after identity verification and before the image/script is consumed.
    identity_locks: Vec<File>,
}

fn attached_process_binds(
    launch: &CliLaunchPlan,
    config: &CliBridgeConfig,
    env: &BTreeMap<String, String>,
    requested_cwd: Option<&PathBuf>,
) -> Result<Vec<BindSpec>, OfficialCliBridgeError> {
    let mut grants = Vec::new();
    let mut push_unique = |path: PathBuf, mode: BindMode| {
        if !grants
            .iter()
            .any(|grant: &BindSpec| grant.host_path == path)
        {
            grants.push(BindSpec {
                host_path: path.clone(),
                guest_path: path,
                mode,
            });
        }
    };
    push_unique(
        launch.identity.effective_executable.canonical_path.clone(),
        BindMode::ReadOnly,
    );
    for root in &launch.read_only_roots {
        push_unique(root.clone(), BindMode::ReadOnly);
    }
    if config.cli_kind == CliKind::CodexCli {
        push_unique(resolve_codex_home(env)?, BindMode::ReadWrite);
    }
    if let Some(cwd) = requested_cwd {
        push_unique(cwd.clone(), BindMode::ReadWrite);
    }
    Ok(grants)
}

fn resolve_codex_home(env: &BTreeMap<String, String>) -> Result<PathBuf, OfficialCliBridgeError> {
    let configured = env
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("CODEX_HOME"))
        .map(|(_, value)| value.trim())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);
    let default_home = env
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("USERPROFILE"))
        .or_else(|| {
            env.iter()
                .find(|(name, _)| name.eq_ignore_ascii_case("HOME"))
        })
        .map(|(_, value)| PathBuf::from(value).join(".codex"));
    let path = configured.or(default_home).ok_or_else(|| {
        OfficialCliBridgeError::CodexHomeUnavailable(
            "neither CODEX_HOME nor USERPROFILE/HOME is available".to_string(),
        )
    })?;
    if !path.is_absolute() {
        return Err(OfficialCliBridgeError::CodexHomeUnavailable(format!(
            "{} is not absolute",
            path.display()
        )));
    }
    if !path.is_dir() {
        return Err(OfficialCliBridgeError::CodexHomeUnavailable(format!(
            "{} is not an existing directory; authenticate the official CLI before enabling the lane",
            path.display()
        )));
    }
    std::fs::canonicalize(&path).map_err(|error| {
        OfficialCliBridgeError::CodexHomeUnavailable(format!(
            "canonicalize {}: {error}",
            path.display()
        ))
    })
}

fn validate_cli_preset(config: &CliBridgeConfig) -> Result<(), OfficialCliBridgeError> {
    if config.cli_kind != CliKind::CodexCli {
        return Ok(());
    }
    #[cfg(windows)]
    if config
        .executable_path
        .file_name()
        .and_then(|name| name.to_str())
        .is_none_or(|name| !name.eq_ignore_ascii_case("codex.cmd"))
    {
        return Err(OfficialCliBridgeError::InvalidCodexPreset(
            "Windows Codex lanes must use the validated official `codex.cmd` npm entrypoint"
                .to_string(),
        ));
    }
    if config.output_format != CliOutputFormat::JsonStream {
        return Err(OfficialCliBridgeError::InvalidCodexPreset(
            "output_format must be JsonStream for `codex exec --json`".to_string(),
        ));
    }
    if config.args_template.first().map(String::as_str) != Some("exec") {
        return Err(OfficialCliBridgeError::InvalidCodexPreset(
            "args_template must start with `exec`".to_string(),
        ));
    }
    if !config.args_template.iter().any(|arg| arg == "--json") {
        return Err(OfficialCliBridgeError::InvalidCodexPreset(
            "args_template must include `--json`".to_string(),
        ));
    }
    let prompt_positions = config
        .args_template
        .iter()
        .enumerate()
        .filter_map(|(index, arg)| (arg == "{prompt}").then_some(index))
        .collect::<Vec<_>>();
    if prompt_positions.as_slice() != [config.args_template.len().saturating_sub(1)] {
        return Err(OfficialCliBridgeError::InvalidCodexPreset(
            "a single standalone `{prompt}` must be the final argument".to_string(),
        ));
    }
    if config
        .args_template
        .iter()
        .any(|arg| arg.contains("{prompt}") && arg != "{prompt}")
    {
        return Err(OfficialCliBridgeError::InvalidCodexPreset(
            "`{prompt}` must not be embedded into another argument".to_string(),
        ));
    }
    Ok(())
}

fn validate_model_binding(config: &CliBridgeConfig) -> Result<(), OfficialCliBridgeError> {
    let model_positions = config
        .args_template
        .iter()
        .enumerate()
        .filter_map(|(index, arg)| (arg == "{model}").then_some(index))
        .collect::<Vec<_>>();
    if model_positions.len() != 1 {
        return Err(OfficialCliBridgeError::InvalidModelBinding(
            "args_template must contain exactly one standalone `{model}` placeholder".to_string(),
        ));
    }
    if config
        .args_template
        .iter()
        .any(|arg| arg.contains("{model}") && arg != "{model}")
    {
        return Err(OfficialCliBridgeError::InvalidModelBinding(
            "`{model}` must not be embedded into another argument".to_string(),
        ));
    }
    let position = model_positions[0];
    let flag = position
        .checked_sub(1)
        .and_then(|index| config.args_template.get(index))
        .map(String::as_str);
    let valid_flag = match config.cli_kind {
        CliKind::ClaudeCode | CliKind::GeminiCli | CliKind::Other => flag == Some("--model"),
        CliKind::CodexCli => matches!(flag, Some("--model" | "-m")),
    };
    if !valid_flag {
        return Err(OfficialCliBridgeError::InvalidModelBinding(format!(
            "{} args_template must bind the standalone placeholder through {}",
            config.cli_kind.label(),
            if config.cli_kind == CliKind::CodexCli {
                "`--model {model}` or `-m {model}`"
            } else {
                "`--model {model}`"
            }
        )));
    }
    Ok(())
}

/// Validate the portable, persisted portion of an Official-CLI bridge config.
///
/// This is deliberately shared by configuration producers and runtime
/// registration so the settings UI cannot persist a document that is known to
/// be rejected later. Executable existence, canonical identity, environment,
/// and sandbox pinning remain runtime checks because they depend on the host.
pub fn validate_cli_bridge_config_contract(
    config: &CliBridgeConfig,
) -> Result<(), OfficialCliBridgeError> {
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
    validate_model_binding(config)?;
    validate_cli_preset(config)
}

fn locked_file_identity(
    path: &std::path::Path,
) -> Result<(FileIdentity, File), OfficialCliBridgeError> {
    let canonical_path = std::fs::canonicalize(path).map_err(|error| {
        OfficialCliBridgeError::ExecutableIdentity(format!(
            "canonicalize {}: {error}",
            path.display()
        ))
    })?;
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    options.share_mode(windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ);
    let mut file = options.open(&canonical_path).map_err(|error| {
        OfficialCliBridgeError::ExecutableIdentity(format!(
            "open identity lock for {}: {error}",
            canonical_path.display()
        ))
    })?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).map_err(|error| {
        OfficialCliBridgeError::ExecutableIdentity(format!(
            "read locked identity {}: {error}",
            canonical_path.display()
        ))
    })?;
    Ok((
        FileIdentity {
            canonical_path,
            sha256: hex::encode(Sha256::digest(bytes)),
        },
        file,
    ))
}

fn file_identity(path: &std::path::Path) -> Result<FileIdentity, OfficialCliBridgeError> {
    locked_file_identity(path).map(|(identity, _lock)| identity)
}

fn reject_command_interpreter(path: &std::path::Path) -> Result<(), OfficialCliBridgeError> {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    const INTERPRETERS: &[&str] = &[
        "cmd",
        "cmd.exe",
        "powershell",
        "powershell.exe",
        "pwsh",
        "pwsh.exe",
        "sh",
        "bash",
        "zsh",
        "fish",
        "node",
        "node.exe",
        "python",
        "python.exe",
        "python3",
        "perl",
        "ruby",
    ];
    if INTERPRETERS.contains(&name.as_str()) {
        return Err(OfficialCliBridgeError::ExecutableIdentity(format!(
            "generic command interpreter {} is not an allowed CLI entrypoint",
            path.display()
        )));
    }
    Ok(())
}

/// Validate discovery/registration against the same executable contract used
/// at launch. A Windows `.cmd` is accepted only when it is the installed Codex
/// npm shim that can be reduced to a direct interpreter argv plan.
pub fn validate_cli_executable_path(path: &std::path::Path) -> Result<(), OfficialCliBridgeError> {
    if !is_windows_command_script(path) {
        return reject_command_interpreter(path);
    }
    #[cfg(windows)]
    {
        resolve_codex_npm_shim(path).map(|_| ())
    }
    #[cfg(not(windows))]
    {
        Err(OfficialCliBridgeError::UnsupportedCommandScript(
            path.to_path_buf(),
        ))
    }
}

fn cli_launch_plan(
    path: &std::path::Path,
    rendered: Vec<String>,
) -> Result<CliLaunchPlan, OfficialCliBridgeError> {
    let (requested_entrypoint, requested_entrypoint_lock) = locked_file_identity(path)?;
    if !is_windows_command_script(path) {
        reject_command_interpreter(path)?;
        return Ok(CliLaunchPlan {
            executable_path: requested_entrypoint.canonical_path.clone(),
            args: rendered,
            identity: CliLaunchIdentity {
                effective_executable: requested_entrypoint.clone(),
                requested_entrypoint,
                effective_script: None,
                launcher_package_manifest: None,
                platform_package_manifest: None,
                final_native_executable: None,
            },
            read_only_roots: Vec::new(),
            identity_locks: vec![requested_entrypoint_lock],
        });
    }
    #[cfg(windows)]
    {
        let resolved = resolve_codex_npm_shim(path)?;
        let (effective_script, effective_script_lock) = locked_file_identity(&resolved.script)?;
        let (launcher_package_manifest, launcher_package_manifest_lock) =
            locked_file_identity(&resolved.launcher_package_manifest)?;
        let (platform_package_manifest, platform_package_manifest_lock) =
            match resolved.platform_package_manifest.as_ref() {
                Some(path) => {
                    let (identity, lock) = locked_file_identity(path)?;
                    (Some(identity), Some(lock))
                }
                None => (None, None),
            };
        let (final_native_executable, final_native_executable_lock) =
            locked_file_identity(&resolved.final_native_executable)?;
        let mut identity_locks = vec![
            requested_entrypoint_lock,
            effective_script_lock,
            launcher_package_manifest_lock,
            final_native_executable_lock,
        ];
        if let Some(lock) = platform_package_manifest_lock {
            identity_locks.push(lock);
        }
        Ok(CliLaunchPlan {
            // Execute the pinned native Codex image directly. The npm shim and
            // JS launcher remain verified provenance, but Node is not placed
            // inside the runtime process tree and cannot create an untracked
            // native descendant.
            executable_path: final_native_executable.canonical_path.clone(),
            args: rendered,
            identity: CliLaunchIdentity {
                requested_entrypoint,
                effective_executable: final_native_executable.clone(),
                effective_script: Some(effective_script),
                launcher_package_manifest: Some(launcher_package_manifest),
                platform_package_manifest,
                final_native_executable: Some(final_native_executable),
            },
            read_only_roots: Vec::new(),
            identity_locks,
        })
    }
    #[cfg(not(windows))]
    {
        let _ = rendered;
        Err(OfficialCliBridgeError::UnsupportedCommandScript(
            path.to_path_buf(),
        ))
    }
}

#[cfg(windows)]
#[derive(Debug)]
struct ResolvedCodexNpmShim {
    node: PathBuf,
    script: PathBuf,
    launcher_package_manifest: PathBuf,
    platform_package_manifest: Option<PathBuf>,
    final_native_executable: PathBuf,
    read_only_roots: Vec<PathBuf>,
}

#[cfg(windows)]
fn resolve_codex_npm_shim(
    shim: &std::path::Path,
) -> Result<ResolvedCodexNpmShim, OfficialCliBridgeError> {
    let is_codex_cmd = shim
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("codex.cmd"));
    if !is_codex_cmd {
        return Err(OfficialCliBridgeError::UnsupportedCommandScript(
            shim.to_path_buf(),
        ));
    }
    let metadata = std::fs::metadata(shim)
        .map_err(|_| OfficialCliBridgeError::UnsupportedCommandScript(shim.to_path_buf()))?;
    if metadata.len() > 128 * 1024 {
        return Err(OfficialCliBridgeError::UnsupportedCommandScript(
            shim.to_path_buf(),
        ));
    }
    let contents = std::fs::read_to_string(shim)
        .map_err(|_| OfficialCliBridgeError::UnsupportedCommandScript(shim.to_path_buf()))?;
    let normalized = contents.replace('/', "\\").to_ascii_lowercase();
    if !normalized.contains("node_modules\\@openai\\codex\\bin\\codex.js")
        || !normalized.contains("%*")
    {
        return Err(OfficialCliBridgeError::UnsupportedCommandScript(
            shim.to_path_buf(),
        ));
    }
    let npm_root = shim
        .parent()
        .ok_or_else(|| OfficialCliBridgeError::UnsupportedCommandScript(shim.to_path_buf()))?;
    let script = npm_root
        .join("node_modules")
        .join("@openai")
        .join("codex")
        .join("bin")
        .join("codex.js");
    if !script.is_file() {
        return Err(OfficialCliBridgeError::UnsupportedCommandScript(
            shim.to_path_buf(),
        ));
    }
    let local_node = npm_root.join("node.exe");
    let node = if local_node.is_file() {
        local_node
    } else {
        find_windows_executable_on_path("node.exe")
            .ok_or_else(|| OfficialCliBridgeError::UnsupportedCommandScript(shim.to_path_buf()))?
    };
    let codex_root = script
        .parent()
        .and_then(std::path::Path::parent)
        .ok_or_else(|| OfficialCliBridgeError::UnsupportedCommandScript(shim.to_path_buf()))?;
    let launcher_package_manifest = codex_root.join("package.json");
    let launcher_manifest = read_json_manifest(&launcher_package_manifest)?;
    if launcher_manifest
        .get("name")
        .and_then(serde_json::Value::as_str)
        != Some("@openai/codex")
        || launcher_manifest
            .pointer("/bin/codex")
            .and_then(serde_json::Value::as_str)
            != Some("bin/codex.js")
    {
        return Err(OfficialCliBridgeError::ExecutableIdentity(format!(
            "{} is not an official @openai/codex launcher manifest",
            launcher_package_manifest.display()
        )));
    }
    let launcher_version = launcher_manifest
        .get("version")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            OfficialCliBridgeError::ExecutableIdentity(format!(
                "{} has no package version",
                launcher_package_manifest.display()
            ))
        })?;
    let (platform_suffix, target_triple, cpu) = windows_codex_target()?;
    let platform_dependency = format!("@openai/codex-{platform_suffix}");
    let expected_alias = format!("npm:@openai/codex@{launcher_version}-{platform_suffix}");
    if launcher_manifest
        .get("optionalDependencies")
        .and_then(|value| value.get(&platform_dependency))
        .and_then(serde_json::Value::as_str)
        != Some(expected_alias.as_str())
    {
        return Err(OfficialCliBridgeError::ExecutableIdentity(format!(
            "{} does not bind the expected platform package alias {expected_alias}",
            launcher_package_manifest.display()
        )));
    }
    let package_directory_name = format!("codex-{platform_suffix}");
    let candidate_roots = [
        codex_root
            .join("node_modules")
            .join("@openai")
            .join(&package_directory_name),
        npm_root
            .join("node_modules")
            .join("@openai")
            .join(&package_directory_name),
    ];
    let existing_package_root = candidate_roots.iter().find(|root| root.exists());
    let (platform_package_manifest, final_native_executable, mut read_only_roots) =
        if let Some(platform_root) = existing_package_root {
            let manifest_path = platform_root.join("package.json");
            let manifest = read_json_manifest(&manifest_path)?;
            let expected_platform_version = format!("{launcher_version}-{platform_suffix}");
            let valid_name =
                manifest.get("name").and_then(serde_json::Value::as_str) == Some("@openai/codex");
            let valid_version = manifest.get("version").and_then(serde_json::Value::as_str)
                == Some(expected_platform_version.as_str());
            let valid_os = manifest
                .get("os")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|values| values.iter().any(|value| value.as_str() == Some("win32")));
            let valid_cpu = manifest
                .get("cpu")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|values| values.iter().any(|value| value.as_str() == Some(cpu)));
            if !(valid_name && valid_version && valid_os && valid_cpu) {
                return Err(OfficialCliBridgeError::ExecutableIdentity(format!(
                "{} does not match the official Codex platform package tuple name/version/os/cpu",
                manifest_path.display()
            )));
            }
            (
                Some(manifest_path),
                platform_root
                    .join("vendor")
                    .join(target_triple)
                    .join("bin")
                    .join("codex.exe"),
                vec![codex_root.to_path_buf(), platform_root.to_path_buf()],
            )
        } else {
            (
                None,
                codex_root
                    .join("vendor")
                    .join(target_triple)
                    .join("bin")
                    .join("codex.exe"),
                vec![codex_root.to_path_buf()],
            )
        };
    if !final_native_executable.is_file() {
        return Err(OfficialCliBridgeError::ExecutableIdentity(format!(
            "official Codex native executable is missing at {}",
            final_native_executable.display()
        )));
    }
    read_only_roots.sort();
    read_only_roots.dedup();
    Ok(ResolvedCodexNpmShim {
        node,
        script,
        launcher_package_manifest,
        platform_package_manifest,
        final_native_executable,
        read_only_roots,
    })
}

#[cfg(windows)]
fn read_json_manifest(path: &std::path::Path) -> Result<serde_json::Value, OfficialCliBridgeError> {
    let bytes = std::fs::read(path).map_err(|error| {
        OfficialCliBridgeError::ExecutableIdentity(format!(
            "read package manifest {}: {error}",
            path.display()
        ))
    })?;
    serde_json::from_slice(&bytes).map_err(|error| {
        OfficialCliBridgeError::ExecutableIdentity(format!(
            "parse package manifest {}: {error}",
            path.display()
        ))
    })
}

#[cfg(all(windows, target_arch = "x86_64"))]
fn windows_codex_target(
) -> Result<(&'static str, &'static str, &'static str), OfficialCliBridgeError> {
    Ok(("win32-x64", "x86_64-pc-windows-msvc", "x64"))
}

#[cfg(all(windows, target_arch = "aarch64"))]
fn windows_codex_target(
) -> Result<(&'static str, &'static str, &'static str), OfficialCliBridgeError> {
    Ok(("win32-arm64", "aarch64-pc-windows-msvc", "arm64"))
}

#[cfg(all(windows, not(any(target_arch = "x86_64", target_arch = "aarch64"))))]
fn windows_codex_target(
) -> Result<(&'static str, &'static str, &'static str), OfficialCliBridgeError> {
    Err(OfficialCliBridgeError::ExecutableIdentity(format!(
        "unsupported Windows architecture {} for Codex CLI",
        std::env::consts::ARCH
    )))
}

#[cfg(windows)]
fn find_windows_executable_on_path(name: &str) -> Option<PathBuf> {
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|paths| std::env::split_paths(&paths).collect::<Vec<_>>())
        .map(|directory| directory.join(name))
        .find(|candidate| candidate.is_file())
}

const CLI_START_DURABILITY_TIMEOUT: Duration = Duration::from_secs(5);
const CLI_STOP_DURABILITY_TIMEOUT: Duration = Duration::from_secs(5);
const CLI_ATTACHED_STARTUP_TIMEOUT_MS: u64 = 60_000;
const CLI_PIPE_READER_CLEANUP_TIMEOUT: Duration = Duration::from_millis(500);
const CLI_ATTACHED_RUNTIME_SHUTDOWN_TIMEOUT: Duration = Duration::from_millis(100);
const MAX_CLI_CAPTURE_BYTES: usize = 8 * 1024 * 1024;
const MAX_PENDING_STREAM_CHUNKS: usize = 32;
const MAX_STREAM_CHUNKS_PER_POLL_CYCLE: usize = 8;

fn attached_child_env(config: &CliBridgeConfig) -> BTreeMap<String, String> {
    let mut env = inherited_official_cli_environment();
    env.extend(
        config
            .env_vars
            .iter()
            .filter(|(name, _)| is_explicit_cli_data_env_name(name))
            .map(|(name, value)| (name.clone(), value.clone())),
    );
    env
}

fn spawn_attached_blocking(
    adapter: Arc<dyn SandboxAdapter>,
    spec: AttachedProcessSpec,
    stdio: AttachedStdioContract,
    preflight_root: Option<Arc<PreflightDataRoot>>,
) -> Result<Box<dyn AttachedSandboxProcess>, SandboxAdapterError> {
    let panic_adapter_id = adapter.capabilities().adapter_id;
    let runtime_thread = thread::Builder::new()
        .name("handshake-cli-attached".to_string())
        .spawn(move || {
            let adapter_id = adapter.capabilities().adapter_id;
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|err| SandboxAdapterError::SpawnFailed {
                    adapter_id,
                    reason: format!("failed to build attached-process runtime: {err}"),
                })?;
            if let Some(root) = preflight_root.as_ref() {
                root.transferred_to_startup
                    .store(true, std::sync::atomic::Ordering::Release);
            }
            let result = runtime.block_on(adapter.spawn_attached_with_stdio(spec, stdio));
            // `spawn_handshake_native_attached` uses a cancellation-aware
            // `spawn_blocking` worker for Win32/AppContainer creation. After its
            // async startup deadline fires, dropping a Tokio runtime would normally
            // wait indefinitely for that blocking worker and erase the bound. A
            // bounded runtime shutdown lets the caller return immediately; the
            // worker retains its cancellation flag and reclaims any late child
            // before it exits.
            runtime.shutdown_timeout(CLI_ATTACHED_RUNTIME_SHUTDOWN_TIMEOUT);
            result
        })
        .map_err(|err| SandboxAdapterError::SpawnFailed {
            adapter_id: panic_adapter_id.clone(),
            reason: format!("failed to create attached-process runtime thread: {err}"),
        })?;
    runtime_thread
        .join()
        .map_err(|_| SandboxAdapterError::SpawnFailed {
            adapter_id: panic_adapter_id,
            reason: "attached-process runtime thread panicked".to_string(),
        })?
}

fn wait_start_durability_blocking(
    acknowledgement: crate::process_ledger::ProcessLedgerDurabilityAck,
) -> Result<(), crate::process_ledger::ProcessLedgerError> {
    thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| {
                crate::process_ledger::ProcessLedgerError::InvalidConfig(format!(
                    "build CLI START durability runtime: {error}"
                ))
            })?;
        runtime.block_on(acknowledgement.wait(CLI_START_DURABILITY_TIMEOUT))
    })
    .join()
    .map_err(|_| {
        crate::process_ledger::ProcessLedgerError::InvalidConfig(
            "CLI START durability thread panicked".to_string(),
        )
    })?
}

fn wait_stop_durability_blocking(
    lifecycle: Arc<ActiveProcessLifecycle>,
    exit_code: Option<i32>,
    reason: String,
) -> Result<StopRecordOutcome, crate::process_ledger::ProcessLedgerError> {
    thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| {
                crate::process_ledger::ProcessLedgerError::InvalidConfig(format!(
                    "build CLI STOP durability runtime: {error}"
                ))
            })?;
        runtime.block_on(lifecycle.stop_with_durable_ack(
            exit_code,
            &reason,
            CLI_STOP_DURABILITY_TIMEOUT,
        ))
    })
    .join()
    .map_err(|_| {
        crate::process_ledger::ProcessLedgerError::InvalidConfig(
            "CLI STOP durability thread panicked".to_string(),
        )
    })?
}

#[cfg(target_os = "windows")]
struct ForegroundWatchPayload {
    owner: ForegroundChildOwner,
}

#[cfg(target_os = "windows")]
fn record_foreground_stop(
    lifecycle: Arc<ActiveProcessLifecycle>,
    pid: u32,
    exit_code: Option<i32>,
    reason: &str,
) -> Result<(), OfficialCliBridgeError> {
    match wait_stop_durability_blocking(lifecycle, exit_code, reason.to_string()) {
        Ok(StopRecordOutcome::Recorded | StopRecordOutcome::AlreadyStopped) => Ok(()),
        Ok(
            StopRecordOutcome::LeftOpenForReconciliation
            | StopRecordOutcome::DurabilityUnconfirmed,
        ) => Err(OfficialCliBridgeError::LedgerRegistration {
            pid,
            reason: format!(
                "foreground login child was reaped but STOP authority remains open or durability-unconfirmed ({reason})"
            ),
        }),
        Err(error) => Err(OfficialCliBridgeError::LedgerRegistration {
            pid,
            reason: format!(
                "foreground login child was reaped but STOP durability failed ({reason}): {error}"
            ),
        }),
    }
}

/// How long a reap attempt waits for the PTY child to be observed dead.
///
/// `PtySession::wait_for_exit` only returns once the waiter thread has reaped
/// the child AND the reader has drained the pseudo-console, so observing it is
/// positive proof the exact process is gone — the same guarantee the previous
/// `Child::wait()` call provided.
#[cfg(target_os = "windows")]
const INTERACTIVE_LOGIN_REAP_TIMEOUT: Duration = Duration::from_secs(15);

/// Bounded lifetime for one interactive login (HBR-QUIET-004 has NOT been
/// declared for this MT, so the login must be time-bounded rather than an
/// open-ended attached process). The watcher stops waiting after this window
/// and falls through to its kill-and-record recovery path, so a login the
/// operator abandoned can never linger as an unbounded child.
#[cfg(target_os = "windows")]
pub(crate) const INTERACTIVE_LOGIN_MAX_LIFETIME: Duration = Duration::from_secs(15 * 60);

/// Terminal geometry for the login PTY. Wide enough that provider login URLs
/// and one-time codes are not hard-wrapped mid-token in the transcript.
#[cfg(target_os = "windows")]
const INTERACTIVE_LOGIN_PTY_ROWS: u16 = 30;
#[cfg(target_os = "windows")]
const INTERACTIVE_LOGIN_PTY_COLS: u16 = 160;

/// Transcript cap for one login session. Bounded so a provider that floods the
/// terminal cannot grow backend memory without limit.
#[cfg(target_os = "windows")]
const INTERACTIVE_LOGIN_TRANSCRIPT_BYTES: usize = 256 * 1024;

/// Immediate owner installed on the first instruction after the interactive
/// login child is spawned. Until ownership transfers into
/// `ForegroundWatchPayload`, every error and panic path kills and synchronously
/// reaps the exact child.
///
/// Identity locks remain in this owner through reap and the matching STOP
/// attempt. A failed wait restores the session into the owner so Drop gets one
/// final recovery attempt instead of silently dropping a potentially-live
/// child.
#[cfg(target_os = "windows")]
struct ForegroundChildOwner {
    session: Option<Arc<crate::terminal::PtySession>>,
    pid: u32,
    lifecycle: Option<Arc<ActiveProcessLifecycle>>,
    _identity_locks: Vec<File>,
}

#[cfg(target_os = "windows")]
impl ForegroundChildOwner {
    fn new(session: Arc<crate::terminal::PtySession>, pid: u32, identity_locks: Vec<File>) -> Self {
        Self {
            session: Some(session),
            pid,
            lifecycle: None,
            _identity_locks: identity_locks,
        }
    }

    fn pid(&self) -> u32 {
        self.pid
    }

    fn attach_lifecycle(&mut self, lifecycle: ActiveProcessLifecycle) {
        self.lifecycle = Some(Arc::new(lifecycle));
    }

    fn terminate_reap_and_record(&mut self, reason: &str) -> Result<(), OfficialCliBridgeError> {
        let pid = self.pid;
        let session = self
            .session
            .take()
            .expect("interactive login owner retains exact child");
        session.kill();
        let Some(exit_code) = session.wait_for_exit(INTERACTIVE_LOGIN_REAP_TIMEOUT) else {
            self.session = Some(session);
            if let Some(lifecycle) = self.lifecycle.as_ref() {
                let _ = lifecycle.leave_open_for_reconciliation();
            }
            return Err(OfficialCliBridgeError::SpawnFailed {
                reason: format!(
                    "interactive login exact child {pid} could not be killed/reaped during {reason}"
                ),
                exit_code: None,
            });
        };
        // The exact child is proven dead. Releasing our session handle here also
        // joins the pty reader/waiter threads when this was the last holder.
        drop(session);
        if let Some(lifecycle) = self.lifecycle.take() {
            record_foreground_stop(lifecycle, pid, Some(exit_code), reason)?;
        }
        Ok(())
    }

    fn wait_and_record(&mut self, reason: &str) -> Result<(), OfficialCliBridgeError> {
        let pid = self.pid;
        let session = self
            .session
            .take()
            .expect("interactive login owner retains exact child");
        let Some(exit_code) = session.wait_for_exit(INTERACTIVE_LOGIN_MAX_LIFETIME) else {
            self.session = Some(session);
            return Err(OfficialCliBridgeError::SpawnFailed {
                reason: format!(
                    "interactive login exact child {pid} exceeded its bounded lifetime during {reason}"
                ),
                exit_code: None,
            });
        };
        drop(session);
        if let Some(lifecycle) = self.lifecycle.take() {
            record_foreground_stop(lifecycle, pid, Some(exit_code), reason)?;
        }
        Ok(())
    }

    fn fail(
        mut self,
        primary: OfficialCliBridgeError,
        cleanup_reason: &str,
    ) -> OfficialCliBridgeError {
        match self.terminate_reap_and_record(cleanup_reason) {
            Ok(()) => primary,
            Err(cleanup_error) => cleanup_error,
        }
    }

    fn into_watch_payload(self) -> Result<ForegroundWatchPayload, OfficialCliBridgeError> {
        let pid = self.pid();
        if self.lifecycle.is_none() {
            return Err(OfficialCliBridgeError::LedgerRegistration {
                pid,
                reason: "interactive login watcher handoff attempted without lifecycle authority"
                    .to_string(),
            });
        }
        Ok(ForegroundWatchPayload { owner: self })
    }
}

#[cfg(target_os = "windows")]
impl Drop for ForegroundChildOwner {
    fn drop(&mut self) {
        if self.session.is_none() {
            return;
        }
        if let Err(error) =
            self.terminate_reap_and_record("official_cli_foreground_login_owner_drop")
        {
            tracing::error!(
                target: "handshake_core::official_cli_bridge",
                error = %error,
                "interactive login owner Drop could not prove exact child cleanup"
            );
            // Dropping the session is not a process-lifecycle operation. If two
            // exact reap attempts both fail, permanently retain the complete
            // ownership bundle. This deliberately leaks the session handle,
            // identity locks, and open lifecycle authority rather than dropping
            // any of them while the process may still be live.
            if let Some(session) = self.session.take() {
                let lifecycle = self.lifecycle.take();
                let identity_locks = std::mem::take(&mut self._identity_locks);
                std::mem::forget((session, lifecycle, identity_locks));
            }
        }
    }
}

#[cfg(target_os = "windows")]
impl ForegroundWatchPayload {
    fn pid(&self) -> u32 {
        self.owner.pid()
    }

    fn wait_and_record_stop(mut self) {
        let pid = self.pid();
        if let Err(initial_error) = self
            .owner
            .wait_and_record("official_cli_foreground_login_exit")
        {
            if self.owner.session.is_some() {
                if let Err(recovery_error) = self
                    .owner
                    .terminate_reap_and_record("official_cli_foreground_login_wait_failed_kill")
                {
                    tracing::error!(
                        target: "handshake_core::official_cli_bridge",
                        pid,
                        initial_error = %initial_error,
                        recovery_error = %recovery_error,
                        "foreground login wait recovery failed; exact owner retained through Drop"
                    );
                }
            } else {
                tracing::error!(
                    target: "handshake_core::official_cli_bridge",
                    pid,
                    error = %initial_error,
                    "foreground login exact child was reaped but STOP durability was not confirmed"
                );
            }
        }
    }

    /// Recover ownership after a failed watcher handoff. STOP is attempted
    /// only after `wait` proves that the exact child handle has terminated.
    fn terminate_after_handoff_failure(mut self) -> Result<(), OfficialCliBridgeError> {
        self.owner
            .terminate_reap_and_record("official_cli_foreground_login_watcher_handoff_failed")
    }
}

#[cfg(target_os = "windows")]
type ForegroundWatcherSender = mpsc::SyncSender<ForegroundWatchPayload>;

#[cfg(target_os = "windows")]
fn spawn_foreground_watcher() -> Result<ForegroundWatcherSender, OfficialCliBridgeError> {
    let (sender, receiver) = mpsc::sync_channel::<ForegroundWatchPayload>(1);
    thread::Builder::new()
        .name("handshake-cli-foreground-watch".to_string())
        .spawn(move || {
            if let Ok(payload) = receiver.recv() {
                payload.wait_and_record_stop();
            }
        })
        .map_err(|error| OfficialCliBridgeError::SpawnFailed {
            reason: format!(
                "foreground login watcher could not be established before process creation: {error}"
            ),
            exit_code: None,
        })?;
    Ok(sender)
}

#[cfg(target_os = "windows")]
fn handoff_foreground_watch(
    sender: ForegroundWatcherSender,
    payload: ForegroundWatchPayload,
) -> Result<(), OfficialCliBridgeError> {
    match sender.send(payload) {
        Ok(()) => Ok(()),
        Err(error) => {
            let pid = error.0.pid();
            match error.0.terminate_after_handoff_failure() {
                Ok(()) => Err(OfficialCliBridgeError::SpawnFailed {
                    reason: format!(
                        "foreground login watcher rejected exact child {pid}; child was killed and reaped"
                    ),
                    exit_code: None,
                }),
                Err(cleanup_error) => Err(cleanup_error),
            }
        }
    }
}

/// Cancellation-aware owner for a streaming stdout/stderr reader.
///
/// A pipe `Read` can remain blocked when a sandbox adapter fails to terminate
/// its child. Cleanup therefore first requests cooperative cancellation and
/// then waits only until a shared deadline. A reader still blocked in the OS
/// pipe is detached pending EOF instead of holding the launch caller forever;
/// the child lifecycle remains open for authoritative reconciliation.
struct StreamingPipeReader {
    cancel: Arc<std::sync::atomic::AtomicBool>,
    worker: Option<thread::JoinHandle<Zeroizing<Vec<u8>>>>,
}

impl StreamingPipeReader {
    fn spawn(
        mut stream: Box<dyn std::io::Read + Send>,
        chunk_sender: Option<mpsc::SyncSender<Zeroizing<Vec<u8>>>>,
    ) -> Self {
        let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let worker_cancel = Arc::clone(&cancel);
        let worker = thread::spawn(move || {
            // Wipe both worker-owned buffers on every exit path. If bounded
            // cleanup detaches this JoinHandle, the thread retains these
            // wrappers and zeroizes them when the blocked read eventually
            // returns or reaches EOF.
            let mut buffer = Zeroizing::new([0u8; 8192]);
            let mut bytes = Zeroizing::new(Vec::new());
            loop {
                if worker_cancel.load(std::sync::atomic::Ordering::Acquire) {
                    break;
                }
                match stream.read(&mut *buffer) {
                    Ok(0) => break,
                    Ok(count) => {
                        let forwarded =
                            count.min(MAX_CLI_CAPTURE_BYTES.saturating_sub(bytes.len()));
                        append_capped(&mut bytes, &buffer[..count]);
                        if forwarded > 0 {
                            if let Some(sender) = chunk_sender.as_ref() {
                                if sender
                                    .send(Zeroizing::new(buffer[..forwarded].to_vec()))
                                    .is_err()
                                {
                                    break;
                                }
                            }
                        }
                    }
                    Err(ref error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(_) => break,
                }
            }
            bytes
        });
        Self {
            cancel,
            worker: Some(worker),
        }
    }

    fn cancel(&self) {
        self.cancel
            .store(true, std::sync::atomic::Ordering::Release);
    }

    fn finish_until(mut self, deadline: Instant) -> PipeReaderFinish {
        let Some(worker) = self.worker.take() else {
            return PipeReaderFinish::default();
        };
        while !worker.is_finished() {
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                // Dropping a JoinHandle detaches only the pipe reader. Its
                // cancellation flag remains set and it exits as soon as the
                // failed child closes the pipe or the blocked read returns.
                drop(worker);
                return PipeReaderFinish {
                    bytes: Zeroizing::new(Vec::new()),
                    completed: false,
                };
            };
            thread::sleep(remaining.min(Duration::from_millis(5)));
        }
        match worker.join() {
            Ok(bytes) => PipeReaderFinish {
                bytes,
                completed: true,
            },
            Err(_) => PipeReaderFinish {
                bytes: Zeroizing::new(Vec::new()),
                completed: false,
            },
        }
    }
}

impl Drop for StreamingPipeReader {
    fn drop(&mut self) {
        self.cancel();
    }
}

struct PipeReaderFinish {
    bytes: Zeroizing<Vec<u8>>,
    completed: bool,
}

impl Default for PipeReaderFinish {
    fn default() -> Self {
        Self {
            bytes: Zeroizing::new(Vec::new()),
            completed: false,
        }
    }
}

struct StreamingPipeReaders {
    stdout: Option<StreamingPipeReader>,
    stderr: Option<StreamingPipeReader>,
}

struct StreamingPipeReadersFinish {
    stdout: Zeroizing<Vec<u8>>,
    stderr: Zeroizing<Vec<u8>>,
    completed: bool,
}

impl StreamingPipeReaders {
    fn new(stdout: Option<StreamingPipeReader>, stderr: Option<StreamingPipeReader>) -> Self {
        Self { stdout, stderr }
    }

    fn cancel(&self) {
        if let Some(reader) = self.stdout.as_ref() {
            reader.cancel();
        }
        if let Some(reader) = self.stderr.as_ref() {
            reader.cancel();
        }
    }

    fn collect_until(&mut self, deadline: Instant) -> StreamingPipeReadersFinish {
        let stdout = self
            .stdout
            .take()
            .map(|reader| reader.finish_until(deadline))
            .unwrap_or(PipeReaderFinish {
                bytes: Zeroizing::new(Vec::new()),
                completed: true,
            });
        let stderr = self
            .stderr
            .take()
            .map(|reader| reader.finish_until(deadline))
            .unwrap_or(PipeReaderFinish {
                bytes: Zeroizing::new(Vec::new()),
                completed: true,
            });
        StreamingPipeReadersFinish {
            stdout: stdout.bytes,
            stderr: stderr.bytes,
            completed: stdout.completed && stderr.completed,
        }
    }
}

impl Drop for StreamingPipeReaders {
    fn drop(&mut self) {
        self.cancel();
        let _ = self.collect_until(Instant::now() + CLI_PIPE_READER_CLEANUP_TIMEOUT);
    }
}

#[cfg(feature = "test-utils")]
pub fn hostile_never_eof_reader_cleanup_probe() -> (bool, Duration) {
    struct NeverEofReader;

    impl std::io::Read for NeverEofReader {
        fn read(&mut self, _buffer: &mut [u8]) -> std::io::Result<usize> {
            loop {
                thread::sleep(Duration::from_secs(60));
            }
        }
    }

    let mut readers = StreamingPipeReaders::new(
        Some(StreamingPipeReader::spawn(Box::new(NeverEofReader), None)),
        None,
    );
    let started = Instant::now();
    let outcome = readers.collect_until(started + CLI_PIPE_READER_CLEANUP_TIMEOUT);
    (outcome.completed, started.elapsed())
}

fn drain_streaming_pipe(
    stream: Option<Box<dyn std::io::Read + Send>>,
    chunk_sender: Option<mpsc::SyncSender<Zeroizing<Vec<u8>>>>,
) -> Option<StreamingPipeReader> {
    stream.map(|stream| StreamingPipeReader::spawn(stream, chunk_sender))
}

fn append_capped(target: &mut Vec<u8>, bytes: &[u8]) {
    let remaining = MAX_CLI_CAPTURE_BYTES.saturating_sub(target.len());
    target.extend_from_slice(&bytes[..bytes.len().min(remaining)]);
}

fn deliver_cli_chunk(
    sender: &tokio::sync::mpsc::Sender<Vec<u8>>,
    chunk: &[u8],
) -> Result<(), OfficialCliBridgeError> {
    sender
        .try_send(chunk.to_vec())
        .map_err(|failure| OfficialCliBridgeError::SpawnFailed {
            reason: format!("official CLI bounded chunk queue rejected output: {failure}"),
            exit_code: None,
        })
}

fn drain_chunks_until(
    receiver: &mpsc::Receiver<Zeroizing<Vec<u8>>>,
    chunk_sender: &tokio::sync::mpsc::Sender<Vec<u8>>,
    deadline: Instant,
) -> Result<bool, OfficialCliBridgeError> {
    while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
        match receiver.recv_timeout(remaining) {
            Ok(chunk) => deliver_cli_chunk(chunk_sender, &chunk)?,
            Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(true),
            Err(mpsc::RecvTimeoutError::Timeout) => return Ok(false),
        }
    }
    Ok(false)
}

fn cleanup_chunk_delivery_failure(
    child: &mut GuardedCliChild,
    receiver: mpsc::Receiver<Zeroizing<Vec<u8>>>,
    mut readers: StreamingPipeReaders,
    error: OfficialCliBridgeError,
) -> OfficialCliBridgeError {
    let cleanup_failed = child.child.is_some()
        && child
            .terminate_and_collect("official_cli_bridge_chunk_delivery_failure")
            .is_none();
    drop(receiver);
    readers.cancel();
    let reader_deadline = Instant::now() + CLI_PIPE_READER_CLEANUP_TIMEOUT;
    let reader_cleanup_timed_out = !readers.collect_until(reader_deadline).completed;
    if cleanup_failed {
        OfficialCliBridgeError::SpawnFailed {
            reason: format!(
                "{error}; sandbox adapter could not prove process-tree termination/reap{}",
                if reader_cleanup_timed_out {
                    "; pipe-reader cleanup exceeded its bounded deadline and remains cancellation-pending until EOF"
                } else {
                    ""
                }
            ),
            exit_code: None,
        }
    } else if reader_cleanup_timed_out {
        OfficialCliBridgeError::SpawnFailed {
            reason: format!(
                "{error}; pipe-reader cleanup exceeded its bounded deadline and remains cancellation-pending until EOF"
            ),
            exit_code: None,
        }
    } else {
        error
    }
}

/// Owns the adapter-attached child and its ledger lifecycle as one drop unit.
///
/// Any live output delivery/cancellation failure must terminate and reap the
/// process tree through the adapter before the reserved STOP permit is consumed. This guard covers
/// the small post-spawn/pre-START interval: if ledger START cannot begin, the
/// unattributed child is killed without fabricating a STOP row.
struct GuardedCliChild {
    pid: u32,
    child: Option<Box<dyn AttachedSandboxProcess>>,
    adapter_capabilities: AdapterCapabilities,
    launch_identity: CliLaunchIdentity,
    resolved_execution_policy: crate::sandbox::ResolvedExecutionPolicy,
    _identity_locks: Vec<File>,
    lifecycle: Option<Arc<ActiveProcessLifecycle>>,
    stop_recorded: bool,
    /// MT-019 F1: running-app reaper for the exact process this guard owns.
    reclaim: Option<Arc<crate::process_ledger::Reclaim>>,
}

/// MT-019 P-5: the running-app reap must finish far inside `RECLAIM_KILL_TIMEOUT`
/// (30s). It blocks a caller thread, and `auth_status` is a sync trait method
/// whose axum caller would otherwise stall a worker for the full kill budget plus
/// the STOP ack. A timeout here is not a failure: the claim lease expires, the row
/// stays truthfully open, and the periodic restart pass retries it.
const CLI_RECLAIM_HOOK_TIMEOUT: Duration = Duration::from_secs(8);

impl GuardedCliChild {
    fn new(
        child: Box<dyn AttachedSandboxProcess>,
        adapter_capabilities: AdapterCapabilities,
        launch_identity: CliLaunchIdentity,
        resolved_execution_policy: crate::sandbox::ResolvedExecutionPolicy,
        identity_locks: Vec<File>,
        reclaim: Option<Arc<crate::process_ledger::Reclaim>>,
    ) -> Self {
        Self {
            pid: child.pid(),
            child: Some(child),
            adapter_capabilities,
            launch_identity,
            resolved_execution_policy,
            _identity_locks: identity_locks,
            lifecycle: None,
            stop_recorded: false,
            reclaim,
        }
    }

    fn pid(&self) -> u32 {
        self.pid
    }

    fn child_mut(
        &mut self,
    ) -> Result<&mut (dyn AttachedSandboxProcess + '_), OfficialCliBridgeError> {
        match self.child.as_deref_mut() {
            Some(child) => Ok(child),
            None => Err(OfficialCliBridgeError::SpawnFailed {
                reason: format!(
                    "guarded CLI child {} no longer owns an attached process",
                    self.pid
                ),
                exit_code: None,
            }),
        }
    }

    fn take_stdout(
        &mut self,
    ) -> Result<Option<Box<dyn std::io::Read + Send>>, OfficialCliBridgeError> {
        Ok(self.child_mut()?.take_stdout())
    }

    fn take_stderr(
        &mut self,
    ) -> Result<Option<Box<dyn std::io::Read + Send>>, OfficialCliBridgeError> {
        Ok(self.child_mut()?.take_stderr())
    }

    fn attach_lifecycle(&mut self, lifecycle: ActiveProcessLifecycle) {
        self.lifecycle = Some(Arc::new(lifecycle));
    }

    fn record_stop(&mut self, exit_code: Option<i32>, reason: &str) {
        if self.stop_recorded {
            return;
        }
        let Some(lifecycle) = self.lifecycle.as_ref() else {
            return;
        };
        let result =
            wait_stop_durability_blocking(Arc::clone(lifecycle), exit_code, reason.to_string());
        match result {
            Ok(StopRecordOutcome::Recorded | StopRecordOutcome::AlreadyStopped) => {
                self.stop_recorded = true
            }
            Ok(
                StopRecordOutcome::LeftOpenForReconciliation
                | StopRecordOutcome::DurabilityUnconfirmed,
            ) => {
                self.leave_open_and_reclaim(
                    "ledger STOP authority was left open for reconciliation",
                );
            }
            Err(err) => {
                tracing::error!(
                    target: "handshake_core::official_cli_bridge",
                    pid = self.pid,
                    error = %err,
                    "ledger STOP registration failed"
                );
                self.leave_open_and_reclaim("ledger STOP registration failed");
            }
        }
    }

    fn leave_open_for_reconciliation(&mut self, reason: &str) {
        if self.stop_recorded {
            return;
        }
        if let Some(lifecycle) = self.lifecycle.as_ref() {
            let _ = lifecycle.leave_open_for_reconciliation();
        }
        self.stop_recorded = true;
        tracing::warn!(
            target: "handshake_core::official_cli_bridge",
            pid = self.pid,
            reason,
            "leaving ProcessOwnershipLedger START open for reconciliation"
        );
    }

    /// MT-019 F1 + P-5: leave the START open, then immediately hand the exact
    /// process to the running app's reaper.
    ///
    /// Ordering is load-bearing. `leave_open_for_reconciliation` releases the
    /// reserved STOP permit; calling the reclaim before that would leave the guard
    /// still holding the permit and `Reclaim::run_claimed` would abort on a
    /// saturated writer, reclaiming nothing.
    ///
    /// This is deliberately NOT called from `Drop`. A `Handle::current().block_on`
    /// during unwind is a double panic (abort), and `futures::executor::block_on`
    /// on a tokio worker stalls it, so Drop's leave-open cases are left to the
    /// periodic pass.
    fn leave_open_and_reclaim(&mut self, reason: &str) {
        self.leave_open_for_reconciliation(reason);
        self.reclaim_open_lifecycle(reason);
    }

    fn reclaim_open_lifecycle(&self, reason: &str) {
        let (Some(reclaim), Some(lifecycle)) = (self.reclaim.as_ref(), self.lifecycle.as_ref())
        else {
            return;
        };
        // `Drop`'s "guard dropped without child ownership" branch is reachable only
        // when no START row was ever written, so there is no process_uuid to
        // reclaim; the explicit lifecycle guard above covers it.
        let process_uuid = lifecycle.process_uuid();
        // The owner descriptor is stamped onto the START by the ledger writer. No
        // owner means we cannot prove THIS instance owns the row, and the
        // owner-scoped claim would (correctly) match nothing, so do not pretend.
        let Some(owner_runtime_instance_id) = lifecycle
            .start()
            .runtime_owner
            .as_ref()
            .map(|owner| owner.runtime_instance_id)
        else {
            tracing::warn!(
                target: "handshake_core::official_cli_bridge",
                pid = self.pid,
                %process_uuid,
                reason,
                "open CLI lifecycle carries no runtime-owner descriptor; leaving it to the boot restart pass instead of reclaiming without ownership proof"
            );
            return;
        };
        let reclaim = Arc::clone(reclaim);
        // Same shape as `wait_stop_durability_blocking`: a dedicated thread with
        // its own current-thread runtime. `Reclaim` needs a real runtime (it
        // spawns claim-renewal tasks and uses `spawn_blocking`), and this call
        // site is a sync `fn` that may be reached from a tokio worker.
        let outcome = thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| format!("build CLI reclaim runtime: {error}"))?;
            runtime.block_on(async move {
                match tokio::time::timeout(
                    CLI_RECLAIM_HOOK_TIMEOUT,
                    reclaim.run_owned_process(
                        process_uuid,
                        owner_runtime_instance_id,
                        crate::process_ledger::ReclaimTrigger::Failure,
                    ),
                )
                .await
                {
                    Ok(Ok(report)) => Ok(report),
                    Ok(Err(error)) => Err(error.to_string()),
                    Err(_) => Err(format!(
                        "running-app reclaim exceeded {} ms",
                        CLI_RECLAIM_HOOK_TIMEOUT.as_millis()
                    )),
                }
            })
        })
        .join()
        .unwrap_or_else(|_| Err("CLI reclaim thread panicked".to_string()));
        match outcome {
            Ok(report) => tracing::info!(
                target: "handshake_core::official_cli_bridge",
                pid = self.pid,
                %process_uuid,
                reason,
                processes_reclaimed = report.processes_reclaimed.len(),
                "running-app reclaim ran for a CLI child whose STOP could not be proven"
            ),
            Err(error) => tracing::warn!(
                target: "handshake_core::official_cli_bridge",
                pid = self.pid,
                %process_uuid,
                reason,
                error,
                "running-app reclaim did not complete; the START row remains truthfully open for the periodic restart pass"
            ),
        }
    }

    /// Terminate, synchronously reap, then emit STOP. The child is killed before
    /// the ledger can describe it as stopped.
    fn terminate_and_collect(&mut self, reason: &str) -> Option<ExitStatus> {
        let mut child = self.child.take()?;
        match child.terminate_tree_and_wait() {
            Ok(status) => {
                self.record_stop(status.code(), reason);
                Some(status)
            }
            Err(err) => {
                self.leave_open_and_reclaim(reason);
                tracing::error!(
                    target: "handshake_core::official_cli_bridge",
                    pid = self.pid,
                    error = %err,
                    "terminated attached child could not be reaped"
                );
                None
            }
        }
    }
}

impl Drop for GuardedCliChild {
    fn drop(&mut self) {
        if self.stop_recorded {
            return;
        }

        let reaped = if let Some(mut child) = self.child.take() {
            match child.try_wait() {
                Ok(Some(_)) => child.terminate_tree_and_wait().map(|status| {
                    (
                        status.code(),
                        "official_cli_bridge_unwind_guard_observed_exit",
                    )
                }),
                Ok(None) => child
                    .terminate_tree_and_wait()
                    .map(|status| (status.code(), "official_cli_bridge_unwind_guard_kill")),
                Err(_) => child.terminate_tree_and_wait().map(|status| {
                    (
                        status.code(),
                        "official_cli_bridge_unwind_guard_try_wait_error",
                    )
                }),
            }
        } else {
            self.leave_open_for_reconciliation("guard dropped without child ownership");
            return;
        };
        match reaped {
            Ok((exit_code, reason)) => self.record_stop(exit_code, reason),
            Err(error) => self.leave_open_for_reconciliation(&format!(
                "guard could not terminate and reap process tree: {error}"
            )),
        }
    }
}

fn resolve_official_cli_execution_policy(
    requested_ref: Option<&str>,
) -> Result<(&str, &'static str), OfficialCliBridgeError> {
    let requested_ref = requested_ref
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| OfficialCliBridgeError::SpawnFailed {
            reason: "official CLI invocation missing execution-policy authority".to_string(),
            exit_code: None,
        })?;
    let effective_ref = crate::sandbox::resolve_execution_policy_ref(requested_ref).ok_or_else(|| {
        OfficialCliBridgeError::SpawnFailed {
            reason: format!(
                "official CLI invocation rejected unknown or stale execution-policy authority {requested_ref}"
            ),
            exit_code: None,
        }
    })?;
    if requested_ref != crate::sandbox::CLI_BRIDGE_REQUESTED_EXECUTION_POLICY_REF
        || effective_ref != crate::sandbox::CLI_BRIDGE_EFFECTIVE_EXECUTION_POLICY_REF
    {
        return Err(OfficialCliBridgeError::SpawnFailed {
            reason: format!(
                "official CLI invocation execution-policy mismatch: requested {requested_ref}, resolved {effective_ref}"
            ),
            exit_code: None,
        });
    }
    Ok((requested_ref, effective_ref))
}

impl LiveCliSpawner {
    /// Construct a production spawner with both mandatory authorities. Making
    /// the sandbox registry a constructor argument prevents a Tauri or backend
    /// composition path from compiling a spawner that can only fail at launch.
    pub fn new(
        process_ledger: Arc<LedgerBatcher>,
        sandbox_registry: Arc<SandboxAdapterRegistry>,
    ) -> Self {
        Self {
            process_ledger,
            sandbox_registry,
            reclaim: None,
            pinned_identities: Arc::new(RwLock::new(HashMap::new())),
            preflight_codex_home: None,
        }
    }

    /// MT-019 F1: attach the running app's reclaimer.
    ///
    /// Without it, a CLI child whose STOP could not be proven leaves an OPEN
    /// START row that only the NEXT boot's restart pass can close — and for the
    /// auth-status/preflight probe class (no `parent_session_id`) not even that,
    /// because `restart_sessions` requires a non-NULL session id. With it, the
    /// bridge reaps that exact process through the owner-scoped claim path as
    /// soon as the failure is observed.
    pub fn with_reclaim(mut self, reclaim: Arc<crate::process_ledger::Reclaim>) -> Self {
        self.reclaim = Some(reclaim);
        self
    }

    /// Product-owned default adapter availability. Per-invocation trust, tier,
    /// capability, network, and workspace authority remain request-scoped.
    pub fn native_cli_registry() -> Arc<SandboxAdapterRegistry> {
        let native_id = AdapterId::new(HANDSHAKE_NATIVE_SANDBOX_ADAPTER_ID);
        let mut registry = SandboxAdapterRegistry::new(native_id.clone());
        registry.register(Arc::new(HandshakeNativeSandboxAdapter::new()));
        Arc::new(registry)
    }

    /// Run a version preflight through the exact production attached-process
    /// boundary. The caller supplies the already validated, persisted bridge
    /// config and the backend-owned invocation posture; raw webview executable
    /// paths never reach this launch seam.
    ///
    /// Reusing [`CliSubprocessSpawner::spawn`] gives the probe the same pinned
    /// executable graph, scrubbed environment, sandbox selection, durable
    /// ProcessOwnershipLedger START/STOP lifecycle, bounded timeout, and
    /// process-tree termination/reap behavior as a real Official-CLI request.
    pub fn preflight_version(
        &self,
        config: &CliBridgeConfig,
        version_arg: &str,
        timeout: Duration,
        invocation: &CliInvocationContext,
    ) -> Result<CliInvocationReceipt, OfficialCliBridgeError> {
        let version_arg = version_arg.trim();
        if version_arg.is_empty() {
            return Err(OfficialCliBridgeError::SpawnFailed {
                reason: "Official-CLI preflight version argument must not be empty".to_string(),
                exit_code: None,
            });
        }
        let mut preflight_config = config.clone();
        preflight_config.args_template = vec![version_arg.to_string()];
        preflight_config.timeout_seconds = timeout.as_secs().max(1);
        let preflight_home = std::env::temp_dir().join(format!(
            "handshake-official-cli-preflight-{}",
            uuid::Uuid::now_v7().simple()
        ));
        std::fs::create_dir(&preflight_home).map_err(|error| {
            OfficialCliBridgeError::SpawnFailed {
                reason: format!(
                    "create isolated Official-CLI preflight data root {}: {error}",
                    preflight_home.display()
                ),
                exit_code: None,
            }
        })?;
        let preflight_root = Arc::new(PreflightDataRoot {
            path: preflight_home,
            transferred_to_startup: std::sync::atomic::AtomicBool::new(false),
        });
        let mut preflight_spawner = self.clone();
        preflight_spawner.preflight_codex_home = Some(preflight_root.clone());
        let result = preflight_spawner
            .pin_config(&preflight_config)
            .and_then(|_| {
                preflight_spawner.spawn(
                    &preflight_config,
                    invocation,
                    "official-cli-configuration-preflight",
                    "",
                )
            });
        drop(preflight_spawner);
        drop(preflight_root);
        result
    }

    /// Run a fixed auxiliary command against an already configured CLI through
    /// the same pinned executable graph, attached sandbox, Job Object/process
    /// tree, bounded pipe readers, and durable START/STOP lifecycle as a normal
    /// official-CLI invocation.
    ///
    /// This is intentionally not a PATH-discovery API. The supplied config must
    /// be the exact launch config already accepted by the provider builder.
    /// Stderr is always zeroized and discarded; only capped stdout plus the
    /// exact exit-success bit cross the boundary for typed status reduction.
    pub(crate) fn run_auxiliary_fixed_command(
        &self,
        config: &CliBridgeConfig,
        args: &[&str],
        timeout: Duration,
        invocation: &CliInvocationContext,
        output_limit: usize,
    ) -> Result<AuxiliaryCliCommandOutput, OfficialCliBridgeError> {
        let mut auxiliary_config = config.clone();
        auxiliary_config.args_template = args.iter().map(|arg| (*arg).to_string()).collect();
        auxiliary_config.timeout_seconds = timeout.as_secs().max(1);
        self.require_previously_pinned_config(&auxiliary_config)?;

        let lifecycle_reservation = self.reserve_process_lifecycle()?;
        let rendered = OfficialCliBridgeRuntime::render_args(
            &auxiliary_config.args_template,
            "official-cli-auth-status",
            "",
        );
        let mut child = self.spawn_attached_child(&auxiliary_config, rendered, invocation)?;
        let record_id = ProcessOwnershipRecordId::new_v7();
        let start = cli_bridge_process_start(
            record_id,
            self.attributed_spawn_meta(&child, invocation, "official-cli-auth-status")?,
        );
        self.attach_durable_lifecycle(&mut child, lifecycle_reservation, start)?;

        let mut readers = StreamingPipeReaders::new(
            drain_streaming_pipe(child.take_stdout()?, None),
            drain_streaming_pipe(child.take_stderr()?, None),
        );
        let started = Instant::now();
        let exit_status = loop {
            match child.child_mut()?.try_wait() {
                Ok(Some(status)) => {
                    break child
                        .terminate_and_collect("official_cli_auth_status_exit")
                        .ok_or_else(|| OfficialCliBridgeError::SpawnFailed {
                            reason: "sandbox adapter failed to reap auth-status process tree after leader exit"
                                .to_string(),
                            exit_code: status.code(),
                        })?;
                }
                Ok(None) if started.elapsed() < timeout => {
                    thread::sleep(Duration::from_millis(25));
                }
                Ok(None) => {
                    let terminated = child
                        .terminate_and_collect("official_cli_auth_status_timeout_kill")
                        .is_some();
                    let mut output =
                        readers.collect_until(Instant::now() + CLI_PIPE_READER_CLEANUP_TIMEOUT);
                    output.stdout.zeroize();
                    output.stderr.zeroize();
                    return Err(OfficialCliBridgeError::SpawnFailed {
                        reason: if terminated {
                            "official CLI auth-status command timed out; process tree terminated and output discarded"
                                .to_string()
                        } else {
                            "official CLI auth-status command timed out and process-tree termination/reap could not be proven"
                                .to_string()
                        },
                        exit_code: None,
                    });
                }
                Err(_) => {
                    let terminated = child
                        .terminate_and_collect("official_cli_auth_status_try_wait_error")
                        .is_some();
                    let mut output =
                        readers.collect_until(Instant::now() + CLI_PIPE_READER_CLEANUP_TIMEOUT);
                    output.stdout.zeroize();
                    output.stderr.zeroize();
                    return Err(OfficialCliBridgeError::SpawnFailed {
                        reason: if terminated {
                            "official CLI auth-status wait failed; process tree terminated and output discarded"
                                .to_string()
                        } else {
                            "official CLI auth-status wait failed and process-tree termination/reap could not be proven"
                                .to_string()
                        },
                        exit_code: None,
                    });
                }
            }
        };

        let mut output = readers.collect_until(Instant::now() + CLI_PIPE_READER_CLEANUP_TIMEOUT);
        if !output.completed {
            output.stdout.zeroize();
            output.stderr.zeroize();
            return Err(OfficialCliBridgeError::SpawnFailed {
                reason:
                    "official CLI auth-status pipe cleanup exceeded its bounded deadline; output discarded"
                        .to_string(),
                exit_code: exit_status.code(),
            });
        }
        child.record_stop(
            exit_status.code(),
            if exit_status.success() {
                "official_cli_auth_status_exit"
            } else {
                "official_cli_auth_status_nonzero_exit"
            },
        );
        output.stderr.zeroize();
        output.stdout.truncate(output_limit);
        Ok(AuxiliaryCliCommandOutput {
            success: exit_status.success(),
            stdout: output.stdout,
        })
    }

    fn require_previously_pinned_config(
        &self,
        config: &CliBridgeConfig,
    ) -> Result<(), OfficialCliBridgeError> {
        validate_config_environment(&config.env_vars)?;
        let identity = cli_launch_plan(&config.executable_path, Vec::new())?.identity;
        let key = identity.requested_entrypoint.canonical_path.clone();
        let pinned = self
            .pinned_identities
            .read()
            .map_err(|error| OfficialCliBridgeError::LockPoisoned(error.to_string()))?;
        match pinned.get(&key) {
            Some(existing) if existing == &identity => Ok(()),
            Some(_) => Err(OfficialCliBridgeError::ExecutableIdentity(format!(
                "registered executable graph changed for {}",
                key.display()
            ))),
            None => Err(OfficialCliBridgeError::ExecutableIdentity(format!(
                "auxiliary command target was not registered by the canonical launch builder: {}",
                key.display()
            ))),
        }
    }

    fn foreground_fixed_launch_plan(
        &self,
        config: &CliBridgeConfig,
        args: &[&str],
    ) -> Result<CliLaunchPlan, OfficialCliBridgeError> {
        self.require_previously_pinned_config(config)?;
        let launch = cli_launch_plan(
            &config.executable_path,
            args.iter().map(|arg| (*arg).to_string()).collect(),
        )?;
        let key = launch.identity.requested_entrypoint.canonical_path.clone();
        let pinned = self
            .pinned_identities
            .read()
            .map_err(|error| OfficialCliBridgeError::LockPoisoned(error.to_string()))?;
        if pinned.get(&key) != Some(&launch.identity) {
            return Err(OfficialCliBridgeError::ExecutableIdentity(format!(
                "foreground login executable graph no longer matches its canonical pin: {}",
                key.display()
            )));
        }
        drop(pinned);
        Ok(launch)
    }

    /// Launch a provider-owned interactive login inside a Handshake-hosted
    /// pseudo-terminal.
    ///
    /// QUIET CONTRACT (HBR-QUIET-001). The child is attached to a ConPTY
    /// pseudo-console opened by `portable-pty`, which calls `CreateProcess`
    /// with `EXTENDED_STARTUPINFO_PRESENT | CREATE_UNICODE_ENVIRONMENT` and
    /// deliberately WITHOUT `CREATE_NEW_CONSOLE`. No console window is created,
    /// nothing is raised to the foreground, and the operator's Z-order and
    /// keyboard focus are untouched. This replaces the previous
    /// `CREATE_NEW_CONSOLE` launch, which was the only spawn site in the tree
    /// that popped an OS window; the rest of the codebase already spawns with
    /// `CREATE_NO_WINDOW`.
    ///
    /// The flow stays COMPLETABLE, which a bare `CREATE_NO_WINDOW` flip would
    /// have destroyed: `claude auth login` and `codex login` are interactive
    /// device/OAuth flows, so the provider's prompt is streamed out of the PTY
    /// into the in-app Settings login panel and the operator's typed answer is
    /// streamed back over [`InteractiveLoginTransport::write_input`].
    ///
    /// Every identity control the console launch had is retained: the
    /// executable graph must already be pinned by the canonical launch builder
    /// (`pin_config` / `require_previously_pinned_config`), `reject_command_interpreter`
    /// still refuses generic shells, the identity locks stay held across the
    /// child's life, the environment is cleared and rebuilt from
    /// `attached_child_env`, and the process ledger still owns matching
    /// START/STOP records. The GUI receives only the resulting pid receipt and
    /// terminal transcript — never a path or argv to re-resolve.
    #[cfg(target_os = "windows")]
    pub(crate) fn launch_foreground_fixed_command(
        &self,
        config: &CliBridgeConfig,
        args: &[&str],
    ) -> Result<InteractiveLoginPty, OfficialCliBridgeError> {
        self.launch_foreground_fixed_command_with_watcher(config, args, spawn_foreground_watcher)
    }

    /// Test-only entry to the REAL interactive-login launch.
    ///
    /// An integration test needs the production path end to end — canonical pin
    /// check, `env_clear` + `attached_child_env`, ConPTY spawn, OS attestation,
    /// process-ledger START/STOP, bounded watcher — while supplying its own
    /// long-lived fixture argv instead of a provider's fixed login argv. This
    /// wrapper adds no behaviour; it only widens visibility under `test-utils`
    /// so the quiet-mode negative proof observes the same code the operator's
    /// `Log in…` button reaches.
    #[cfg(all(target_os = "windows", feature = "test-utils"))]
    pub fn launch_interactive_login_for_tests(
        &self,
        config: &CliBridgeConfig,
        args: &[&str],
    ) -> Result<InteractiveLoginPty, OfficialCliBridgeError> {
        self.launch_foreground_fixed_command(config, args)
    }

    #[cfg(target_os = "windows")]
    fn launch_foreground_fixed_command_with_watcher(
        &self,
        config: &CliBridgeConfig,
        args: &[&str],
        establish_watcher: impl FnOnce() -> Result<ForegroundWatcherSender, OfficialCliBridgeError>,
    ) -> Result<InteractiveLoginPty, OfficialCliBridgeError> {
        self.launch_foreground_fixed_command_with_hooks(
            config,
            args,
            Vec::new(),
            establish_watcher,
            |pid| {
                crate::sandbox::handshake_native::process_creation_time_100ns(pid)
                    .map_err(|error| error.to_string())
            },
            |reservation, start| reservation.begin_with_durable_ack(start),
            wait_start_durability_blocking,
        )
    }

    #[cfg(target_os = "windows")]
    fn launch_foreground_fixed_command_with_hooks(
        &self,
        config: &CliBridgeConfig,
        args: &[&str],
        mut additional_identity_locks: Vec<File>,
        establish_watcher: impl FnOnce() -> Result<ForegroundWatcherSender, OfficialCliBridgeError>,
        attest_process: impl FnOnce(u32) -> Result<u64, String>,
        begin_lifecycle: impl FnOnce(
            ReservedProcessLifecycle,
            ProcessStart,
        ) -> Result<
            (ActiveProcessLifecycle, ProcessLedgerDurabilityAck),
            ProcessLedgerError,
        >,
        await_start: impl FnOnce(ProcessLedgerDurabilityAck) -> Result<(), ProcessLedgerError>,
    ) -> Result<InteractiveLoginPty, OfficialCliBridgeError> {
        let lifecycle_reservation = self.reserve_process_lifecycle()?;
        let launch = self.foreground_fixed_launch_plan(config, args)?;
        // Establish watcher capacity before process creation. No login child can
        // exist if the OS cannot create its ownership thread.
        let watcher = establish_watcher()?;

        // HBR-QUIET-001: ConPTY, not CREATE_NEW_CONSOLE. `portable-pty` attaches
        // the child to a headless pseudo-console, so no window is created and no
        // Z-order/foreground change occurs. `env_clear` reproduces the console
        // launch's `Command::env_clear()`, so the PTY route does not widen the
        // child environment.
        // FAIL CLOSED rather than lossily converting the PINNED entrypoint.
        //
        // `launch.executable_path` is the exact binary this bridge canonicalized,
        // identity-checked, and pinned via `require_previously_pinned_config`.
        // `to_string_lossy()` would substitute U+FFFD for any non-UTF-8 byte and
        // hand the PTY a path that is no longer the pinned binary - silently
        // defeating the identity guarantee the pinning exists to provide, on the
        // one code path that launches a credential-bearing login. A path we
        // cannot represent exactly is a refusal, not a best-effort spawn.
        let shell = launch
            .executable_path
            .to_str()
            .ok_or_else(|| {
                OfficialCliBridgeError::ExecutableIdentity(format!(
                    "pinned CLI entrypoint is not representable as UTF-8 and cannot be launched \
                 without losing executable identity: {}",
                    launch.executable_path.display()
                ))
            })?
            .to_string();
        let session = crate::terminal::PtySession::spawn(crate::terminal::PtySpawnConfig {
            shell: Some(shell),
            args: launch.args.clone(),
            cwd: config.working_dir.clone(),
            env: attached_child_env(config).into_iter().collect(),
            env_clear: true,
            rows: INTERACTIVE_LOGIN_PTY_ROWS,
            cols: INTERACTIVE_LOGIN_PTY_COLS,
            scrollback_bytes: INTERACTIVE_LOGIN_TRANSCRIPT_BYTES,
            broadcast_capacity: crate::terminal::pty::DEFAULT_BROADCAST_CAPACITY,
        })
        .map_err(|error| OfficialCliBridgeError::SpawnFailed {
            reason: format!("interactive official CLI login pty spawn failed: {error}"),
            exit_code: None,
        })?;
        // Install exact-child ownership immediately after spawn, before any
        // attestation, allocation, ledger, or watcher-handoff work can fail.
        // `PtySession` is kill-on-drop, so the child is owned from this point on
        // even if the pid could not be read.
        let session = Arc::new(session);
        let Some(pid) = session.child_pid() else {
            return Err(OfficialCliBridgeError::SpawnFailed {
                reason: "interactive official CLI login pty reported no child pid".to_string(),
                exit_code: None,
            });
        };
        let mut identity_locks = launch.identity_locks;
        identity_locks.append(&mut additional_identity_locks);
        let mut child_owner = ForegroundChildOwner::new(Arc::clone(&session), pid, identity_locks);
        let creation_time_100ns = match attest_process(pid) {
            Ok(value) => value,
            Err(error) => {
                return Err(child_owner.fail(
                    OfficialCliBridgeError::SpawnFailed {
                        reason: format!(
                            "foreground login process-generation attestation failed: {error}"
                        ),
                        exit_code: None,
                    },
                    "official_cli_foreground_login_attestation_failed",
                ));
            }
        };
        let record_id = ProcessOwnershipRecordId::new_v7();
        let mut meta = SpawnMeta::new(
            pid,
            ProcessEngineKind::OfficialCliBridge,
            "MODEL_ACCESS_CLI_LOGIN",
        );
        meta.owner_wp = Some("WP-1".to_string());
        meta.role_id = Some("MODEL_ACCESS_CLI_LOGIN".to_string());
        meta.wp_id = Some("WP-1".to_string());
        meta.mt_id = Some("MT-015".to_string());
        meta.reclaim_key = Some(format!("model-access-cli-login-{pid}"));
        meta.model_identity = Some(config.cli_kind.label().to_string());
        meta.metadata_blob = json!({
            "launch_kind": "operator_confirmed_in_app_pty_login",
            "requested_entrypoint_sha256": launch.identity.requested_entrypoint.sha256,
            "effective_executable_sha256": launch.identity.effective_executable.sha256,
            "os_creation_time_100ns": creation_time_100ns,
        });
        let start = cli_bridge_process_start(record_id, meta);
        let (lifecycle, acknowledgement) = match begin_lifecycle(lifecycle_reservation, start) {
            Ok(value) => value,
            Err(error) => {
                return Err(child_owner.fail(
                    OfficialCliBridgeError::LedgerRegistration {
                        pid,
                        reason: error.to_string(),
                    },
                    "official_cli_foreground_login_start_begin_failed",
                ));
            }
        };
        child_owner.attach_lifecycle(lifecycle);
        if let Err(error) = await_start(acknowledgement) {
            return Err(child_owner.fail(
                OfficialCliBridgeError::LedgerRegistration {
                    pid,
                    reason: error.to_string(),
                },
                "official_cli_foreground_login_start_not_durable",
            ));
        }
        handoff_foreground_watch(watcher, child_owner.into_watch_payload()?)?;
        Ok(InteractiveLoginPty { pid, session })
    }

    fn spawn_attached_child(
        &self,
        config: &CliBridgeConfig,
        rendered: Vec<String>,
        invocation: &CliInvocationContext,
    ) -> Result<GuardedCliChild, OfficialCliBridgeError> {
        // Resolve the complete typed policy before executable graph inspection,
        // pin comparison, sandbox selection, ledger START, or process spawn.
        let requested_trust_class = invocation.requested_trust_class.ok_or_else(|| {
            OfficialCliBridgeError::SpawnFailed {
                reason: "official CLI invocation missing requested trust class".to_string(),
                exit_code: None,
            }
        })?;
        let requested_isolation_tier = invocation.requested_isolation_tier.ok_or_else(|| {
            OfficialCliBridgeError::SpawnFailed {
                reason: "official CLI invocation missing requested isolation tier".to_string(),
                exit_code: None,
            }
        })?;
        let requested_capabilities = invocation
            .requested_sandbox_capabilities
            .as_ref()
            .ok_or_else(|| OfficialCliBridgeError::SpawnFailed {
                reason: "official CLI invocation missing requested sandbox capabilities"
                    .to_string(),
                exit_code: None,
            })?;
        let requested_net_policy = invocation.requested_net_policy.as_ref().ok_or_else(|| {
            OfficialCliBridgeError::SpawnFailed {
                reason: "official CLI invocation missing requested network policy".to_string(),
                exit_code: None,
            }
        })?;
        let resource_limits = ResourceLimits {
            timeout_ms: Some(config.timeout_seconds.saturating_mul(1_000)),
            ..ResourceLimits::default()
        };
        let requested_execution_policy_ref = invocation
            .requested_execution_policy_ref
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| OfficialCliBridgeError::SpawnFailed {
                reason: "official CLI invocation missing execution-policy authority".to_string(),
                exit_code: None,
            })?;
        let resolved_execution_policy =
            crate::sandbox::ResolvedExecutionPolicy::resolve_official_cli(
                crate::sandbox::ExecutionPolicyRequest {
                    requested_ref: requested_execution_policy_ref.to_string(),
                    trust_class: requested_trust_class,
                    isolation_tier: requested_isolation_tier,
                    required_capabilities: requested_capabilities.clone(),
                    requested_net_policy: requested_net_policy.clone(),
                    effective_attached_network_mode: AttachedNetworkMode::OutboundInternetClient,
                    resource_limits: resource_limits.clone(),
                    startup_timeout_ms: CLI_ATTACHED_STARTUP_TIMEOUT_MS,
                },
            )
            .map_err(|error| OfficialCliBridgeError::SpawnFailed {
                reason: format!("official CLI execution-policy resolution failed: {error}"),
                exit_code: None,
            })?;
        let execution_policy_ref = resolved_execution_policy.requested_ref.as_str();
        let effective_execution_policy_ref = resolved_execution_policy.effective_ref.as_str();

        let launch = cli_launch_plan(&config.executable_path, rendered)?;
        let pinned = self
            .pinned_identities
            .read()
            .map_err(|error| OfficialCliBridgeError::LockPoisoned(error.to_string()))?;
        let pinned_identity = pinned
            .get(&launch.identity.requested_entrypoint.canonical_path)
            .ok_or_else(|| {
                OfficialCliBridgeError::ExecutableIdentity(format!(
                    "{} was not pinned by bridge registration",
                    config.executable_path.display()
                ))
            })?;
        if pinned_identity != &launch.identity {
            return Err(OfficialCliBridgeError::ExecutableIdentity(format!(
                "executable graph changed after bridge registration for {}",
                config.executable_path.display()
            )));
        }
        drop(pinned);
        let registry = &self.sandbox_registry;
        let requested_cwd = invocation
            .working_dir
            .as_ref()
            .map(std::path::PathBuf::from);
        if let Some(expected_root) = invocation.checkout_lease_canonical_working_dir.as_deref() {
            let requested =
                requested_cwd
                    .as_ref()
                    .ok_or_else(|| OfficialCliBridgeError::SpawnFailed {
                        reason: "official CLI checkout lease has no working-directory bind"
                            .to_string(),
                        exit_code: None,
                    })?;
            let effective_root =
                crate::swarm_orchestration::checkout_lease::canonical_checkout_root(
                    requested.to_string_lossy().as_ref(),
                )
                .map_err(|error| OfficialCliBridgeError::SpawnFailed {
                    reason: format!("official CLI checkout lease validation failed: {error}"),
                    exit_code: None,
                })?;
            if !paths_match_checkout_identity(&effective_root, Path::new(expected_root)) {
                return Err(OfficialCliBridgeError::SpawnFailed {
                    reason: format!(
                        "official CLI checkout lease root mismatch: lease={} effective={}",
                        expected_root,
                        effective_root.display()
                    ),
                    exit_code: None,
                });
            }
        }
        if requested_cwd != config.working_dir {
            return Err(OfficialCliBridgeError::SpawnFailed {
                reason: format!(
                    "official CLI working-directory authority mismatch: request={:?}, config={:?}",
                    requested_cwd, config.working_dir
                ),
                exit_code: None,
            });
        }
        let mut env = attached_child_env(config);
        if let Some(preflight_codex_home) = self.preflight_codex_home.as_ref() {
            env.insert(
                "CODEX_HOME".to_string(),
                preflight_codex_home.path.display().to_string(),
            );
        }
        if config.cli_kind == CliKind::CodexCli {
            if let Some(package_manifest) = launch.identity.launcher_package_manifest.as_ref() {
                if let Some(package_root) = package_manifest.canonical_path.parent() {
                    env.insert(
                        "CODEX_MANAGED_PACKAGE_ROOT".to_string(),
                        package_root.display().to_string(),
                    );
                    env.insert("CODEX_MANAGED_BY_NPM".to_string(), "1".to_string());
                }
            }
        }
        let binds = attached_process_binds(&launch, config, &env, requested_cwd.as_ref())?;
        let selection_spec = ProcessSpec {
            id: AdapterId::new("official_cli_bridge"),
            image_or_root: ImageRef::new(launch.executable_path.display().to_string()),
            cmd: std::iter::once(launch.executable_path.display().to_string())
                .chain(launch.args.iter().cloned())
                .collect(),
            env: env.clone(),
            cwd: requested_cwd.clone(),
            binds: binds.clone(),
            net_policy: requested_net_policy.clone(),
            resource_limits: resource_limits.clone(),
            idle_timeout_ms: None,
            required_capabilities: requested_capabilities.clone(),
            trust_class: requested_trust_class,
            metadata: BTreeMap::from([
                (
                    "invocation_kind".to_string(),
                    "official_cli_bridge".to_string(),
                ),
                (
                    "execution_policy_ref".to_string(),
                    execution_policy_ref.to_string(),
                ),
                (
                    "effective_execution_policy_ref".to_string(),
                    effective_execution_policy_ref.to_string(),
                ),
                (
                    "swarm_id".to_string(),
                    invocation.swarm_id.clone().unwrap_or_default(),
                ),
                (
                    "worktree_id".to_string(),
                    invocation.worktree_id.clone().unwrap_or_default(),
                ),
                (
                    "checkout_lease_id".to_string(),
                    invocation
                        .checkout_lease_id
                        .map(|value| value.to_string())
                        .unwrap_or_default(),
                ),
                (
                    "checkout_lease_owner_generation".to_string(),
                    invocation
                        .checkout_lease_owner_generation
                        .map(|value| value.to_string())
                        .unwrap_or_default(),
                ),
            ]),
        };
        let adapter_id = AdapterId::new(HANDSHAKE_NATIVE_SANDBOX_ADAPTER_ID);
        let adapter = select(registry, &selection_spec, Some(&adapter_id)).map_err(|failure| {
            OfficialCliBridgeError::SpawnFailed {
                reason: format!("sandbox selection rejected official CLI launch: {failure}"),
                exit_code: None,
            }
        })?;
        if requested_net_policy != &NetPolicy::HostInherited {
            return Err(OfficialCliBridgeError::SpawnFailed {
                reason: format!(
                    "selected attached sandbox cannot satisfy requested network policy {:?}",
                    requested_net_policy
                ),
                exit_code: None,
            });
        }
        adapter
            .validate_attached_network_mode(AttachedNetworkMode::OutboundInternetClient)
            .map_err(|failure| OfficialCliBridgeError::SpawnFailed {
                reason: format!(
                    "selected sandbox cannot enforce official CLI attached networking: {failure}"
                ),
                exit_code: None,
            })?;
        let capabilities = adapter.capabilities();
        resolved_execution_policy
            .validate_adapter_capabilities(&capabilities)
            .map_err(|error| OfficialCliBridgeError::SpawnFailed {
                reason: format!(
                    "selected sandbox does not satisfy resolved official CLI policy: {error}"
                ),
                exit_code: None,
            })?;
        let launch_identity = launch.identity.clone();
        let identity_locks = launch.identity_locks;
        let spec = AttachedProcessSpec {
            executable_path: launch.executable_path,
            args: launch.args,
            env,
            cwd: requested_cwd,
            binds,
            network_mode: AttachedNetworkMode::OutboundInternetClient,
            trust_class: requested_trust_class,
            required_capabilities: requested_capabilities.clone(),
            requested_isolation_tier,
            requested_net_policy: requested_net_policy.clone(),
            resource_limits,
            // AppContainer profile/token/job creation has its own bounded
            // startup budget. Invocation timeout starts after an attached
            // child exists and must not collapse this boundary to one second.
            startup_timeout_ms: CLI_ATTACHED_STARTUP_TIMEOUT_MS,
            ephemeral_cleanup_paths: self
                .preflight_codex_home
                .as_ref()
                .map(|root| vec![root.path.clone()])
                .unwrap_or_default(),
            execution_policy_ref: effective_execution_policy_ref.to_string(),
            resolved_execution_policy: Some(resolved_execution_policy.clone()),
            swarm_id: invocation.swarm_id.clone(),
            worktree_id: invocation.worktree_id.clone(),
            checkout_lease_id: invocation.checkout_lease_id,
            checkout_lease_owner_generation: invocation.checkout_lease_owner_generation,
            checkout_lease_canonical_working_dir: invocation
                .checkout_lease_canonical_working_dir
                .clone(),
        };
        resolved_execution_policy
            .validate_attached_spec(&spec)
            .map_err(|error| OfficialCliBridgeError::SpawnFailed {
                reason: format!(
                    "attached process spec drifted from resolved official CLI policy: {error}"
                ),
                exit_code: None,
            })?;
        spawn_attached_blocking(
            adapter,
            spec,
            AttachedStdioContract::null_stdin_piped_output(),
            self.preflight_codex_home.clone(),
        )
        .map(|child| {
            GuardedCliChild::new(
                child,
                capabilities,
                launch_identity,
                resolved_execution_policy,
                identity_locks,
                self.reclaim.clone(),
            )
        })
        .map_err(|err| OfficialCliBridgeError::SpawnFailed {
            reason: format!(
                "sandbox adapter rejected {}: {err}",
                config.executable_path.display()
            ),
            exit_code: None,
        })
    }

    fn attributed_spawn_meta(
        &self,
        child: &GuardedCliChild,
        invocation: &CliInvocationContext,
        selected_model_name: &str,
    ) -> Result<SpawnMeta, OfficialCliBridgeError> {
        let capabilities = child.adapter_capabilities.clone();
        let mut meta = cli_bridge_spawn_meta(
            child.pid(),
            invocation,
            selected_model_name,
            &child.launch_identity,
        );
        meta.sandbox_adapter = serde_json::to_value(&capabilities.adapter_id)
            .ok()
            .and_then(|value| value.as_str().map(ToOwned::to_owned));
        meta.sandbox_capabilities_snapshot = serde_json::to_value(&capabilities)
            .unwrap_or_else(|_| json!({"serialization_error": true}));
        meta.metadata_blob["sandbox_adapter"] = json!(capabilities.adapter_id.as_str());
        meta.metadata_blob["effective_trust_class"] =
            json!(child.resolved_execution_policy.trust_class);
        meta.metadata_blob["effective_isolation_tier"] = json!(capabilities.isolation_tier);
        meta.metadata_blob["effective_sandbox_capabilities"] = json!(capabilities);
        meta.metadata_blob["effective_net_policy"] = json!("outbound_internet_client");
        meta.metadata_blob["execution_policy_resolution"] = json!({
            "schema_id": child.resolved_execution_policy.schema_id,
            "schema_version": child.resolved_execution_policy.schema_version,
            "policy_revision": child.resolved_execution_policy.policy_revision,
            "requested_ref": child.resolved_execution_policy.requested_ref,
            "effective_ref": child.resolved_execution_policy.effective_ref,
            "resolution_status": "resolved",
            "sandbox_boundary_adapter": capabilities.adapter_id.as_str(),
            "effective_isolation_tier": capabilities.isolation_tier,
            "effective_net_policy": child.resolved_execution_policy.effective_attached_network_mode,
            "required_capabilities": child.resolved_execution_policy.required_capabilities,
            "resource_limits": child.resolved_execution_policy.resource_limits,
            "startup_timeout_ms": child.resolved_execution_policy.startup_timeout_ms
        });
        meta.metadata_blob["effective_swarm_id"] = json!(invocation.swarm_id);
        meta.metadata_blob["effective_worktree_id"] = json!(invocation.worktree_id);
        meta.metadata_blob["effective_working_dir"] = json!(invocation.working_dir);
        meta.metadata_blob["effective_checkout_lease_id"] = json!(invocation.checkout_lease_id);
        meta.metadata_blob["effective_checkout_lease_owner_generation"] =
            json!(invocation.checkout_lease_owner_generation);
        meta.metadata_blob["effective_checkout_lease_canonical_working_dir"] =
            json!(invocation.checkout_lease_canonical_working_dir);
        meta.metadata_blob["os_creation_time_100ns"] = json!(
            crate::sandbox::handshake_native::process_creation_time_100ns(child.pid()).map_err(
                |error| OfficialCliBridgeError::SpawnFailed {
                    reason: format!(
                        "official CLI process-generation attestation failed for pid {}: {error}",
                        child.pid()
                    ),
                    exit_code: None,
                }
            )?
        );
        Ok(meta)
    }

    fn reserve_process_lifecycle(
        &self,
    ) -> Result<crate::process_ledger::ReservedProcessLifecycle, OfficialCliBridgeError> {
        self.process_ledger
            .try_reserve_lifecycles(1)
            .and_then(|mut reservations| {
                reservations.pop().ok_or_else(|| {
                    crate::process_ledger::ProcessLedgerError::InvalidConfig(
                        "single CLI lifecycle reservation was empty".to_string(),
                    )
                })
            })
            .map_err(|err| OfficialCliBridgeError::LedgerPreflight {
                reason: err.to_string(),
            })
    }

    fn attach_durable_lifecycle(
        &self,
        child: &mut GuardedCliChild,
        reservation: crate::process_ledger::ReservedProcessLifecycle,
        start: ProcessStart,
    ) -> Result<(), OfficialCliBridgeError> {
        let pid = child.pid();
        let (lifecycle, acknowledgement) =
            reservation.begin_with_durable_ack(start).map_err(|error| {
                OfficialCliBridgeError::LedgerRegistration {
                    pid,
                    reason: error.to_string(),
                }
            })?;
        child.attach_lifecycle(lifecycle);
        if let Err(error) = wait_start_durability_blocking(acknowledgement) {
            let cleanup = child
                .terminate_and_collect("official_cli_bridge_start_not_durable")
                .map(|_| "child terminated and reaped".to_string())
                .unwrap_or_else(|| "child termination/reap could not be proven".to_string());
            return Err(OfficialCliBridgeError::LedgerRegistration {
                pid,
                reason: format!("{error}; {cleanup}"),
            });
        }
        Ok(())
    }
}

impl CliSubprocessSpawner for LiveCliSpawner {
    fn pin_config(&self, config: &CliBridgeConfig) -> Result<(), OfficialCliBridgeError> {
        validate_config_environment(&config.env_vars)?;
        let identity = cli_launch_plan(&config.executable_path, Vec::new())?.identity;
        let key = identity.requested_entrypoint.canonical_path.clone();
        let mut pinned = self
            .pinned_identities
            .write()
            .map_err(|error| OfficialCliBridgeError::LockPoisoned(error.to_string()))?;
        if let Some(existing) = pinned.get(&key) {
            if existing != &identity {
                return Err(OfficialCliBridgeError::ExecutableIdentity(format!(
                    "registered executable graph changed for {}",
                    key.display()
                )));
            }
        } else {
            pinned.insert(key, identity);
        }
        Ok(())
    }

    fn spawn(
        &self,
        config: &CliBridgeConfig,
        invocation: &CliInvocationContext,
        model_name: &str,
        prompt: &str,
    ) -> Result<CliInvocationReceipt, OfficialCliBridgeError> {
        let lifecycle_reservation = self.reserve_process_lifecycle()?;
        let rendered =
            OfficialCliBridgeRuntime::render_args(&config.args_template, model_name, prompt);

        let mut child = self.spawn_attached_child(config, rendered, invocation)?;
        let pid = child.pid();

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
            self.attributed_spawn_meta(&child, invocation, model_name)?,
        );
        self.attach_durable_lifecycle(&mut child, lifecycle_reservation, start)?;

        let mut readers = StreamingPipeReaders::new(
            drain_streaming_pipe(child.take_stdout()?, None),
            drain_streaming_pipe(child.take_stderr()?, None),
        );
        let timeout = Duration::from_secs(config.timeout_seconds);
        let started = Instant::now();
        let exit_status = loop {
            match child.child_mut()?.try_wait() {
                Ok(Some(status)) => {
                    break child
                        .terminate_and_collect(if status.success() {
                            "official_cli_bridge_exit"
                        } else {
                            "official_cli_bridge_nonzero_exit"
                        })
                        .ok_or_else(|| OfficialCliBridgeError::SpawnFailed {
                            reason:
                                "sandbox adapter failed to reap CLI process tree after leader exit"
                                    .to_string(),
                            exit_code: status.code(),
                        })?;
                }
                Ok(None) => {
                    if started.elapsed() >= timeout {
                        // The child was killed on timeout; record the STOP row
                        // so the killed process is reconciled in the ledger.
                        let terminated = child
                            .terminate_and_collect("official_cli_bridge_timeout_kill")
                            .is_some();
                        if !terminated {
                            return Err(OfficialCliBridgeError::SpawnFailed {
                                reason: "CLI timeout occurred and sandbox adapter could not prove process-tree termination/reap".to_string(),
                                exit_code: None,
                            });
                        }
                        let output =
                            readers.collect_until(Instant::now() + CLI_PIPE_READER_CLEANUP_TIMEOUT);
                        let partial_stdout = String::from_utf8_lossy(&output.stdout).into_owned();
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
                    if child
                        .terminate_and_collect("official_cli_bridge_try_wait_error")
                        .is_none()
                    {
                        return Err(OfficialCliBridgeError::SpawnFailed {
                            reason: format!(
                                "try_wait failed: {err}; sandbox adapter could not prove process-tree termination/reap"
                            ),
                            exit_code: None,
                        });
                    }
                    return Err(OfficialCliBridgeError::SpawnFailed {
                        reason: format!("try_wait failed: {err}"),
                        exit_code: None,
                    });
                }
            }
        };

        let output = readers.collect_until(Instant::now() + CLI_PIPE_READER_CLEANUP_TIMEOUT);
        if !output.completed {
            return Err(OfficialCliBridgeError::SpawnFailed {
                reason: "CLI exited but pipe-reader cleanup exceeded its bounded deadline"
                    .to_string(),
                exit_code: exit_status.code(),
            });
        }
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let mut stderr = output.stderr;
        let exit_code = exit_status.code();
        child.record_stop(
            exit_code,
            if exit_status.success() {
                "official_cli_bridge_exit"
            } else {
                "official_cli_bridge_nonzero_exit"
            },
        );

        if !exit_status.success() {
            stderr.zeroize();
            return Err(OfficialCliBridgeError::SpawnFailed {
                reason: format!(
                    "CLI {} exited with status {:?}; provider stderr was discarded",
                    config.executable_path.display(),
                    exit_code
                ),
                exit_code,
            });
        }
        stderr.zeroize();

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
    /// each chunk is delivered through the bounded sender while the subprocess is still
    /// running. This is the real cloud-CLI capture producer for §10.1: the
    /// bounded sender wiring (see `invoke_with_capture`) fans these live chunks into a
    /// read-only AiJob capture session + the Flight Recorder, instead of dumping
    /// the post-completion stdout string. The full stdout is also accumulated so
    /// the returned [`CliInvocationReceipt`] is byte-for-byte identical to the
    /// non-streaming path.
    fn spawn_streaming(
        &self,
        config: &CliBridgeConfig,
        invocation: &CliInvocationContext,
        model_name: &str,
        prompt: &str,
        chunk_sender: &tokio::sync::mpsc::Sender<Vec<u8>>,
    ) -> Result<CliInvocationReceipt, OfficialCliBridgeError> {
        let lifecycle_reservation = self.reserve_process_lifecycle()?;
        let rendered =
            OfficialCliBridgeRuntime::render_args(&config.args_template, model_name, prompt);

        let mut child = self.spawn_attached_child(config, rendered, invocation)?;
        let pid = child.pid();

        // Unconditional ledger START (fail-closed), identical to `spawn`.
        let record_id = ProcessOwnershipRecordId::new_v7();
        let start = cli_bridge_process_start(
            record_id,
            self.attributed_spawn_meta(&child, invocation, model_name)?,
        );
        self.attach_durable_lifecycle(&mut child, lifecycle_reservation, start)?;

        // Take the stdout pipe and pump it on a dedicated thread, forwarding each
        // chunk through bounded channels while this thread polls try_wait for the
        // timeout. stderr is drained on its own thread so a full stderr pipe can
        // never deadlock the child.
        let child_stdout = child.take_stdout()?;
        let child_stderr = child.take_stderr()?;
        let (chunk_tx, chunk_rx) =
            mpsc::sync_channel::<Zeroizing<Vec<u8>>>(MAX_PENDING_STREAM_CHUNKS);
        let mut readers = StreamingPipeReaders::new(
            drain_streaming_pipe(child_stdout, Some(chunk_tx.clone())),
            drain_streaming_pipe(child_stderr, None),
        );
        drop(chunk_tx); // only the reader thread holds a sender now.

        let timeout = Duration::from_secs(config.timeout_seconds);
        let started = Instant::now();
        let exit_status = loop {
            // Forward live chunks without executing caller code before checking exit.
            for _ in 0..MAX_STREAM_CHUNKS_PER_POLL_CYCLE {
                if started.elapsed() >= timeout {
                    break;
                }
                let chunk = match chunk_rx.try_recv() {
                    Ok(chunk) => chunk,
                    Err(mpsc::TryRecvError::Empty | mpsc::TryRecvError::Disconnected) => break,
                };
                if let Err(error) = deliver_cli_chunk(chunk_sender, &chunk) {
                    return Err(cleanup_chunk_delivery_failure(
                        &mut child, chunk_rx, readers, error,
                    ));
                }
            }
            match child.child_mut()?.try_wait() {
                Ok(Some(status)) => {
                    break child
                        .terminate_and_collect(if status.success() {
                            "official_cli_bridge_exit"
                        } else {
                            "official_cli_bridge_nonzero_exit"
                        })
                        .ok_or_else(|| OfficialCliBridgeError::SpawnFailed {
                            reason:
                                "sandbox adapter failed to reap CLI process tree after leader exit"
                                    .to_string(),
                            exit_code: status.code(),
                        })?;
                }
                Ok(None) => {
                    if started.elapsed() >= timeout {
                        if child
                            .terminate_and_collect("official_cli_bridge_timeout_kill")
                            .is_none()
                        {
                            return Err(OfficialCliBridgeError::SpawnFailed {
                                reason: "sandbox adapter failed to reap timed-out CLI process tree"
                                    .to_string(),
                                exit_code: None,
                            });
                        }
                        // Flush any remaining live chunks captured before the kill.
                        let cleanup_deadline = Instant::now() + CLI_PIPE_READER_CLEANUP_TIMEOUT;
                        if let Err(error) =
                            drain_chunks_until(&chunk_rx, chunk_sender, cleanup_deadline)
                        {
                            return Err(cleanup_chunk_delivery_failure(
                                &mut child, chunk_rx, readers, error,
                            ));
                        }
                        drop(chunk_rx);
                        let output = readers.collect_until(cleanup_deadline);
                        let partial_stdout = String::from_utf8_lossy(&output.stdout).into_owned();
                        return Err(OfficialCliBridgeError::SpawnTimeout {
                            timeout_seconds: config.timeout_seconds,
                            partial_stdout,
                        });
                    }
                    std::thread::sleep(Duration::from_millis(15));
                }
                Err(err) => {
                    if child
                        .terminate_and_collect("official_cli_bridge_try_wait_error")
                        .is_none()
                    {
                        return Err(OfficialCliBridgeError::SpawnFailed {
                            reason: format!(
                                "try_wait failed: {err}; sandbox adapter could not prove process-tree termination/reap"
                            ),
                            exit_code: None,
                        });
                    }
                    return Err(OfficialCliBridgeError::SpawnFailed {
                        reason: format!("try_wait failed: {err}"),
                        exit_code: None,
                    });
                }
            }
        };

        // Child exited: drain any straggler chunks, then join the reader threads
        // to recover the full stdout/stderr.
        let cleanup_deadline = Instant::now() + CLI_PIPE_READER_CLEANUP_TIMEOUT;
        let chunks_drained = match drain_chunks_until(&chunk_rx, chunk_sender, cleanup_deadline) {
            Ok(completed) => completed,
            Err(error) => {
                return Err(cleanup_chunk_delivery_failure(
                    &mut child, chunk_rx, readers, error,
                ));
            }
        };
        drop(chunk_rx);
        let output = readers.collect_until(cleanup_deadline);
        if !chunks_drained || !output.completed {
            return Err(OfficialCliBridgeError::SpawnFailed {
                reason: "CLI exited but pipe-reader cleanup exceeded its bounded deadline"
                    .to_string(),
                exit_code: exit_status.code(),
            });
        }
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let mut stderr = output.stderr;
        let exit_code = exit_status.code();

        if !exit_status.success() {
            child.record_stop(exit_code, "official_cli_bridge_nonzero_exit");
            stderr.zeroize();
            return Err(OfficialCliBridgeError::SpawnFailed {
                reason: format!(
                    "CLI {} exited with status {:?}; provider stderr was discarded",
                    config.executable_path.display(),
                    exit_code
                ),
                exit_code,
            });
        }

        stderr.zeroize();
        child.record_stop(exit_code, "official_cli_bridge_exit");
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
    /// START/STOP, bounded live chunk fan-out, timeout + kill) plus ONE additional
    /// check per poll iteration: when the cancellation set is marked the child
    /// process tree is killed through the attached sandbox adapter, a STOP row with reason
    /// `official_cli_bridge_cancel_kill` is recorded, any straggler chunks are
    /// flushed, and a receipt with `cancelled = true` is returned. This is the
    /// real deterministic-cancellation backstop the swarm adapter relies on to
    /// honour the request/runtime `CancellationToken`.
    fn spawn_streaming_cancellable(
        &self,
        config: &CliBridgeConfig,
        invocation: &CliInvocationContext,
        model_name: &str,
        prompt: &str,
        chunk_sender: &tokio::sync::mpsc::Sender<Vec<u8>>,
        cancellation: &CliCancellationContext,
    ) -> Result<CliInvocationReceipt, OfficialCliBridgeError> {
        let lifecycle_reservation = self.reserve_process_lifecycle()?;
        let rendered =
            OfficialCliBridgeRuntime::render_args(&config.args_template, model_name, prompt);

        let mut child = self.spawn_attached_child(config, rendered, invocation)?;
        let pid = child.pid();

        let record_id = ProcessOwnershipRecordId::new_v7();
        let start = cli_bridge_process_start(
            record_id,
            self.attributed_spawn_meta(&child, invocation, model_name)?,
        );
        self.attach_durable_lifecycle(&mut child, lifecycle_reservation, start)?;

        let child_stdout = child.take_stdout()?;
        let child_stderr = child.take_stderr()?;
        let (chunk_tx, chunk_rx) =
            mpsc::sync_channel::<Zeroizing<Vec<u8>>>(MAX_PENDING_STREAM_CHUNKS);
        let mut readers = StreamingPipeReaders::new(
            drain_streaming_pipe(child_stdout, Some(chunk_tx.clone())),
            drain_streaming_pipe(child_stderr, None),
        );
        drop(chunk_tx);

        let timeout = Duration::from_secs(config.timeout_seconds);
        let started = Instant::now();
        let exit_status = loop {
            // Forward live chunks without executing caller code before checking exit/cancel.
            for _ in 0..MAX_STREAM_CHUNKS_PER_POLL_CYCLE {
                if cancellation.is_cancelled() || started.elapsed() >= timeout {
                    break;
                }
                let chunk = match chunk_rx.try_recv() {
                    Ok(chunk) => chunk,
                    Err(mpsc::TryRecvError::Empty | mpsc::TryRecvError::Disconnected) => break,
                };
                if let Err(error) = deliver_cli_chunk(chunk_sender, &chunk) {
                    return Err(cleanup_chunk_delivery_failure(
                        &mut child, chunk_rx, readers, error,
                    ));
                }
            }
            // Deterministic cancellation: kill the child and return a cancelled
            // receipt rather than running the CLI to completion.
            if cancellation.is_cancelled() {
                if child
                    .terminate_and_collect("official_cli_bridge_cancel_kill")
                    .is_none()
                {
                    return Err(OfficialCliBridgeError::SpawnFailed {
                        reason: "sandbox adapter failed to reap cancelled CLI process tree"
                            .to_string(),
                        exit_code: None,
                    });
                }
                let cleanup_deadline = Instant::now() + CLI_PIPE_READER_CLEANUP_TIMEOUT;
                if let Err(error) = drain_chunks_until(&chunk_rx, chunk_sender, cleanup_deadline) {
                    return Err(cleanup_chunk_delivery_failure(
                        &mut child, chunk_rx, readers, error,
                    ));
                }
                drop(chunk_rx);
                let output = readers.collect_until(cleanup_deadline);
                if !output.completed {
                    return Err(OfficialCliBridgeError::SpawnFailed {
                        reason: "cancelled CLI pipe-reader cleanup exceeded its bounded deadline"
                            .to_string(),
                        exit_code: None,
                    });
                }
                return Ok(CliInvocationReceipt {
                    model_id: ModelId::new_v7(),
                    stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                    pid: Some(pid),
                    exit_code: None,
                    cancelled: true,
                });
            }
            match child.child_mut()?.try_wait() {
                Ok(Some(status)) => {
                    break child
                        .terminate_and_collect(if status.success() {
                            "official_cli_bridge_exit"
                        } else {
                            "official_cli_bridge_nonzero_exit"
                        })
                        .ok_or_else(|| OfficialCliBridgeError::SpawnFailed {
                            reason:
                                "sandbox adapter failed to reap CLI process tree after leader exit"
                                    .to_string(),
                            exit_code: status.code(),
                        })?;
                }
                Ok(None) => {
                    if started.elapsed() >= timeout {
                        if child
                            .terminate_and_collect("official_cli_bridge_timeout_kill")
                            .is_none()
                        {
                            return Err(OfficialCliBridgeError::SpawnFailed {
                                reason: "sandbox adapter failed to reap timed-out CLI process tree"
                                    .to_string(),
                                exit_code: None,
                            });
                        }
                        let cleanup_deadline = Instant::now() + CLI_PIPE_READER_CLEANUP_TIMEOUT;
                        if let Err(error) =
                            drain_chunks_until(&chunk_rx, chunk_sender, cleanup_deadline)
                        {
                            return Err(cleanup_chunk_delivery_failure(
                                &mut child, chunk_rx, readers, error,
                            ));
                        }
                        drop(chunk_rx);
                        let output = readers.collect_until(cleanup_deadline);
                        let partial_stdout = String::from_utf8_lossy(&output.stdout).into_owned();
                        return Err(OfficialCliBridgeError::SpawnTimeout {
                            timeout_seconds: config.timeout_seconds,
                            partial_stdout,
                        });
                    }
                    std::thread::sleep(Duration::from_millis(15));
                }
                Err(err) => {
                    if child
                        .terminate_and_collect("official_cli_bridge_try_wait_error")
                        .is_none()
                    {
                        return Err(OfficialCliBridgeError::SpawnFailed {
                            reason: format!(
                                "try_wait failed: {err}; sandbox adapter could not prove process-tree termination/reap"
                            ),
                            exit_code: None,
                        });
                    }
                    return Err(OfficialCliBridgeError::SpawnFailed {
                        reason: format!("try_wait failed: {err}"),
                        exit_code: None,
                    });
                }
            }
        };

        let cleanup_deadline = Instant::now() + CLI_PIPE_READER_CLEANUP_TIMEOUT;
        let chunks_drained = match drain_chunks_until(&chunk_rx, chunk_sender, cleanup_deadline) {
            Ok(completed) => completed,
            Err(error) => {
                return Err(cleanup_chunk_delivery_failure(
                    &mut child, chunk_rx, readers, error,
                ));
            }
        };
        drop(chunk_rx);
        let output = readers.collect_until(cleanup_deadline);
        if !chunks_drained || !output.completed {
            return Err(OfficialCliBridgeError::SpawnFailed {
                reason: "CLI exited but pipe-reader cleanup exceeded its bounded deadline"
                    .to_string(),
                exit_code: exit_status.code(),
            });
        }
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let mut stderr = output.stderr;
        let exit_code = exit_status.code();

        if !exit_status.success() {
            child.record_stop(exit_code, "official_cli_bridge_nonzero_exit");
            stderr.zeroize();
            return Err(OfficialCliBridgeError::SpawnFailed {
                reason: format!(
                    "CLI {} exited with status {:?}; provider stderr was discarded",
                    config.executable_path.display(),
                    exit_code
                ),
                exit_code,
            });
        }

        stderr.zeroize();
        child.record_stop(exit_code, "official_cli_bridge_exit");
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
mod sandbox_composition_regression_tests {
    use super::*;

    #[test]
    fn production_launches_are_adapter_owned_and_have_no_native_fallback() {
        let source = include_str!("official_cli_bridge.rs");
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("production source section");
        let windows_attached_adapter = include_str!("../../sandbox/windows_native_jail/adapter.rs");

        assert_eq!(
            production
                .matches("self.spawn_attached_child(config, rendered, invocation)?")
                .count(),
            3,
            "all live launch variants must invoke the attached sandbox adapter"
        );
        assert!(production.contains("select(registry, &selection_spec"));
        assert!(production.contains("trust_class: requested_trust_class"));
        assert!(production.contains("required_capabilities: requested_capabilities.clone()"));
        assert!(production.contains("AttachedStdioContract::null_stdin_piped_output()"));
        assert!(production.contains("net_policy: requested_net_policy.clone()"));
        assert!(production.contains("requested_isolation_tier"));
        assert!(production.contains("requested_execution_policy_ref"));
        assert!(!production.contains("sandbox_context"));
        assert!(production.contains("validate_attached_network_mode"));
        assert!(production.contains("tokio::sync::mpsc::Sender<Vec<u8>>"));
        assert!(production.contains("sandbox adapter rejected"));
        assert!(production.contains("terminate_tree_and_wait"));
        // Tightened by MT-015 v5 (was 1). The operator-confirmed login was the last
        // direct OS spawn in this module and the only site in the tree that opened
        // a console window; it now runs under the Handshake-hosted ConPTY, so ZERO
        // direct spawns remain. This is strictly stronger than the previous bound:
        // reintroducing any `std::process::Command::new` here fails immediately.
        assert_eq!(
            production.matches("std::process::Command::new").count(),
            0,
            "no direct OS spawn may remain: model and status execution stay adapter-owned, and the \
             operator-confirmed login runs under the Handshake-hosted pty (HBR-QUIET-001)"
        );
        assert!(
            production.contains("crate::terminal::PtySession::spawn"),
            "the operator-confirmed login must run under the Handshake-hosted pty"
        );
        assert!(production.contains("launch_foreground_fixed_command"));
        assert!(!production.contains("Command::new(\"cmd.exe\")"));
        assert!(!production.contains("Command::new(\"cmd\")"));
        assert!(production.contains("args: rendered"));
        assert!(!production.contains("taskkill"));
        assert!(!production.contains("kill_process_tree"));
        assert!(production.contains("JoinHandle<Zeroizing<Vec<u8>>>"));
        assert!(production.contains("Zeroizing::new([0u8; 8192])"));
        assert!(production.contains("SyncSender<Zeroizing<Vec<u8>>>"));
        assert!(production.contains("stdout: Zeroizing<Vec<u8>>"));
        assert!(production.contains("stderr: Zeroizing<Vec<u8>>"));
        assert!(windows_attached_adapter
            .contains("wait_with_timeout(Some(ATTACHED_TERMINATION_REAP_TIMEOUT))"));
        assert!(windows_attached_adapter.contains(
            "const ATTACHED_TERMINATION_REAP_TIMEOUT: Duration = Duration::from_secs(5)"
        ));
        assert!(
            !windows_attached_adapter.contains("let wait_result = self.wait();"),
            "timeout/cancellation/unwind cleanup must not reintroduce an infinite Windows reap"
        );
    }

    #[test]
    fn capture_and_chunk_backpressure_failures_are_bounded() {
        let mut capture = vec![0u8; MAX_CLI_CAPTURE_BYTES - 1];
        append_capped(&mut capture, &[1, 2, 3, 4]);
        assert_eq!(capture.len(), MAX_CLI_CAPTURE_BYTES);

        let (sender, _receiver) = tokio::sync::mpsc::channel(1);
        sender.try_send(b"first".to_vec()).expect("seed queue");
        assert!(matches!(
            deliver_cli_chunk(&sender, b"overflow"),
            Err(OfficialCliBridgeError::SpawnFailed { .. })
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;
    use std::time::Instant;

    #[test]
    fn terminal_capture_never_reports_spawn_failure_as_exit_zero() {
        let missing_exit = Err(OfficialCliBridgeError::SpawnFailed {
            reason: "spawn never started".to_owned(),
            exit_code: None,
        });
        assert_eq!(terminal_capture_exit_code(&missing_exit), -1);
        let explicit_exit = Err(OfficialCliBridgeError::SpawnFailed {
            reason: "process rejected".to_owned(),
            exit_code: Some(127),
        });
        assert_eq!(terminal_capture_exit_code(&explicit_exit), 127);
    }

    struct ReapFailingAttachedProcess {
        pid: u32,
    }

    impl AttachedSandboxProcess for ReapFailingAttachedProcess {
        fn pid(&self) -> u32 {
            self.pid
        }

        fn take_stdout(&mut self) -> Option<Box<dyn std::io::Read + Send>> {
            None
        }

        fn take_stderr(&mut self) -> Option<Box<dyn std::io::Read + Send>> {
            None
        }

        fn try_wait(&mut self) -> Result<Option<ExitStatus>, SandboxAdapterError> {
            Ok(None)
        }

        fn wait(&mut self) -> Result<ExitStatus, SandboxAdapterError> {
            Err(fake_reap_failure())
        }

        fn terminate_tree_and_wait(&mut self) -> Result<ExitStatus, SandboxAdapterError> {
            Err(fake_reap_failure())
        }
    }

    fn fake_reap_failure() -> SandboxAdapterError {
        SandboxAdapterError::SpawnFailed {
            adapter_id: AdapterId::new("reap_failure_test"),
            reason: "intentional terminate/reap failure".to_string(),
        }
    }

    #[derive(Clone, Default)]
    struct ReconciliationLedgerStore {
        events: Arc<Mutex<Vec<crate::process_ledger::LedgerEvent>>>,
    }

    #[async_trait::async_trait]
    impl crate::process_ledger::ProcessLedgerStore for ReconciliationLedgerStore {
        async fn write_batch(
            &self,
            events: Vec<crate::process_ledger::LedgerEvent>,
        ) -> Result<(), crate::process_ledger::ProcessLedgerError> {
            self.events.lock().unwrap().extend(events);
            Ok(())
        }
    }

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
            _invocation: &CliInvocationContext,
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
            _invocation: &CliInvocationContext,
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

    fn test_invocation() -> CliInvocationContext {
        let mut context = CliInvocationContext::new("TEST_ROLE", "test-model");
        context.owner_wp = Some("WP-TEST".to_string());
        context.role_id = Some("TEST_ROLE".to_string());
        context.wp_id = Some("WP-TEST".to_string());
        context.mt_id = Some("MT-003".to_string());
        context.session_id = Some("session-test".to_string());
        context.parent_session_id = Some("session-parent".to_string());
        context.trace_id = Some("trace-test".to_string());
        context.span_id = Some("span-test".to_string());
        context.cancellation_id = Some("cancel-test".to_string());
        context.reclaim_key = Some("reclaim-test".to_string());
        context.requested_trust_class = Some(TrustClass::Trusted);
        context.requested_isolation_tier = Some(IsolationTier::Tier1Container);
        context.requested_sandbox_capabilities =
            Some(BTreeSet::from([RequiredCapability::HighStdioThroughput]));
        context.requested_net_policy = Some(NetPolicy::HostInherited);
        context.requested_execution_policy_ref =
            Some(crate::sandbox::CLI_BRIDGE_REQUESTED_EXECUTION_POLICY_REF.to_string());
        context.swarm_id = Some("test-swarm".to_string());
        context.worktree_id = Some("test-worktree".to_string());
        context
    }

    fn test_resolved_execution_policy() -> crate::sandbox::ResolvedExecutionPolicy {
        crate::sandbox::ResolvedExecutionPolicy::resolve_official_cli(
            crate::sandbox::ExecutionPolicyRequest {
                requested_ref: crate::sandbox::CLI_BRIDGE_REQUESTED_EXECUTION_POLICY_REF
                    .to_string(),
                trust_class: TrustClass::Trusted,
                isolation_tier: IsolationTier::Tier1Container,
                required_capabilities: BTreeSet::from([RequiredCapability::HighStdioThroughput]),
                requested_net_policy: NetPolicy::HostInherited,
                effective_attached_network_mode: AttachedNetworkMode::OutboundInternetClient,
                resource_limits: ResourceLimits {
                    timeout_ms: Some(1_000),
                    ..ResourceLimits::default()
                },
                startup_timeout_ms: CLI_ATTACHED_STARTUP_TIMEOUT_MS,
            },
        )
        .expect("canonical test execution policy resolves")
    }

    #[test]
    fn official_cli_execution_policy_resolution_rejects_missing_unknown_and_stale_refs() {
        for candidate in [
            None,
            Some(""),
            Some("execution-policy://test/official-cli"),
            Some("execution-policy://requested/retired-cli-v0"),
            Some(crate::sandbox::LOCAL_REQUESTED_EXECUTION_POLICY_REF),
        ] {
            let error = resolve_official_cli_execution_policy(candidate)
                .expect_err("non-authoritative policy must fail before process creation");
            assert!(matches!(error, OfficialCliBridgeError::SpawnFailed { .. }));
        }
        assert_eq!(
            resolve_official_cli_execution_policy(Some(
                crate::sandbox::CLI_BRIDGE_REQUESTED_EXECUTION_POLICY_REF,
            ))
            .expect("registered CLI policy resolves"),
            (
                crate::sandbox::CLI_BRIDGE_REQUESTED_EXECUTION_POLICY_REF,
                crate::sandbox::CLI_BRIDGE_EFFECTIVE_EXECUTION_POLICY_REF,
            )
        );
    }

    #[test]
    fn official_cli_posture_mismatch_fails_before_executable_inspection() {
        let (ledger, _drain) = test_ledger();
        let spawner = LiveCliSpawner::new(ledger, LiveCliSpawner::native_cli_registry());
        let mut invocation = test_invocation();
        invocation.requested_isolation_tier = Some(IsolationTier::Tier2Syscall);
        let config = CliBridgeConfig {
            executable_path: PathBuf::from("definitely-missing-official-cli-executable"),
            ..good_config()
        };

        let error = match spawner.spawn_attached_child(&config, Vec::new(), &invocation) {
            Ok(_) => panic!("policy posture must reject before executable inspection"),
            Err(error) => error,
        };
        assert!(
            error
                .to_string()
                .contains("execution-policy resolution failed"),
            "unexpected preflight ordering: {error}"
        );
        assert!(!error.to_string().contains("executable identity"));
    }

    /// Build a real, manually-drained `LedgerBatcher` for tests. The ledger is
    /// mandatory on `LiveCliSpawner` (MT-127, MT-122-class), so every test
    /// that constructs a live spawner attaches one. The receiver stays alive
    /// for the full subprocess run; dropping it is a closed-authority condition
    /// that preflight rejects before spawn.
    fn test_ledger() -> (
        Arc<crate::process_ledger::LedgerBatcher>,
        crate::process_ledger::ProcessLedgerDrain,
    ) {
        let (batcher, drain) = crate::process_ledger::LedgerBatcher::manual_for_tests(
            crate::process_ledger::LedgerBatcherConfig::default(),
            Arc::new(crate::process_ledger::NoopOverflowSink),
        )
        .expect("manual ledger batcher for tests");
        (Arc::new(batcher), drain)
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
                std::env::var_os("SystemRoot").unwrap_or_else(|| r"C:\Windows".into()),
            )
            .join(r"System32\ping.exe"),
            args_template: vec![
                "-t".to_string(),
                "-w".to_string(),
                "{prompt}".to_string(),
                "127.0.0.1".to_string(),
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
            executable_path: PathBuf::from("/usr/bin/yes"),
            args_template: vec!["{prompt}".to_string()],
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

        let script_path = std::env::temp_dir().join(format!(
            "handshake-official-cli-{}.cmd",
            uuid::Uuid::now_v7()
        ));
        std::fs::write(&script_path, "@echo off\r\n").expect("write command-script fixture");
        let mut bad = good_config();
        bad.executable_path = script_path.clone();
        let err = runtime
            .register_bridge(bad, "claude-3.5-sonnet", "2026-05-20T06:00:00Z")
            .expect_err("command scripts require an injection-safe typed launcher");
        let _ = std::fs::remove_file(script_path);
        assert!(matches!(
            err,
            OfficialCliBridgeError::UnsupportedCommandScript(_)
        ));
    }

    #[cfg(windows)]
    #[test]
    fn codex_npm_shim_builds_direct_injection_safe_argv_for_metacharacters() {
        let root =
            std::env::temp_dir().join(format!("handshake-codex-shim-{}", uuid::Uuid::now_v7()));
        let script = root
            .join("node_modules")
            .join("@openai")
            .join("codex")
            .join("bin")
            .join("codex.js");
        std::fs::create_dir_all(script.parent().expect("script parent")).expect("fixture dirs");
        let node = root.join("node.exe");
        std::fs::write(&node, b"fixture").expect("fixture node");
        std::fs::write(&script, b"// fixture").expect("fixture script");
        let codex_root = script.parent().unwrap().parent().unwrap();
        let (platform_suffix, target_triple, cpu) = windows_codex_target().expect("target");
        std::fs::write(
            codex_root.join("package.json"),
            serde_json::to_vec(&serde_json::json!({
                "name": "@openai/codex",
                "version": "1.2.3",
                "bin": { "codex": "bin/codex.js" },
                "optionalDependencies": {
                    format!("@openai/codex-{platform_suffix}"):
                        format!("npm:@openai/codex@1.2.3-{platform_suffix}")
                }
            }))
            .expect("launcher manifest"),
        )
        .expect("write launcher manifest");
        let platform_root = codex_root
            .join("node_modules")
            .join("@openai")
            .join(format!("codex-{platform_suffix}"));
        let native = platform_root
            .join("vendor")
            .join(target_triple)
            .join("bin")
            .join("codex.exe");
        std::fs::create_dir_all(native.parent().expect("native parent"))
            .expect("platform fixture dirs");
        std::fs::write(
            platform_root.join("package.json"),
            serde_json::to_vec(&serde_json::json!({
                "name": "@openai/codex",
                "version": format!("1.2.3-{platform_suffix}"),
                "os": ["win32"],
                "cpu": [cpu]
            }))
            .expect("platform manifest"),
        )
        .expect("write platform manifest");
        std::fs::write(&native, b"native fixture").expect("write native fixture");
        let shim = root.join("codex.cmd");
        std::fs::write(
            &shim,
            b"@echo off\r\n\"%dp0%\\node.exe\" \"%dp0%\\node_modules\\@openai\\codex\\bin\\codex.js\" %*\r\n",
        )
        .expect("fixture shim");
        let rendered = vec![
            "exec".to_string(),
            "--json".to_string(),
            "space separated prompt".to_string(),
            "& whoami".to_string(),
            "| more".to_string(),
            "<input >output".to_string(),
            "^caret".to_string(),
            "%PATH%".to_string(),
            "!delayed!".to_string(),
            "(parenthesized)".to_string(),
            "quote\"inside".to_string(),
            "semi;colon && second".to_string(),
            "line-one\r\nline-two".to_string(),
            "backslash\\ending\\".to_string(),
            "equals=value".to_string(),
        ];
        let plan = cli_launch_plan(&shim, rendered.clone()).expect("validated direct plan");
        let canonical_native = std::fs::canonicalize(&native).expect("canonical native");
        assert_eq!(plan.executable_path, canonical_native);
        assert_eq!(plan.args, rendered);
        assert_ne!(plan.executable_path, node);
        assert!(
            !plan
                .args
                .iter()
                .any(|arg| arg == &script.display().to_string()),
            "verified JavaScript launcher is provenance, not runtime argv"
        );
        assert_eq!(
            plan.identity
                .final_native_executable
                .as_ref()
                .expect("native identity")
                .canonical_path,
            canonical_native
        );
        assert_eq!(plan.identity_locks.len(), 5);
        let _ = std::fs::remove_dir_all(root);
    }

    #[cfg(windows)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn installed_codex_preflight_uses_attached_sandbox_and_balanced_ledger_lifecycle() {
        let shim = find_windows_executable_on_path("codex.cmd").expect(
            "installed Codex npm shim is required for the Windows production preflight proof",
        );
        let config = CliBridgeConfig {
            cli_kind: CliKind::CodexCli,
            executable_path: shim,
            args_template: vec![
                "exec".to_string(),
                "--json".to_string(),
                "--skip-git-repo-check".to_string(),
                "--model".to_string(),
                "{model}".to_string(),
                "{prompt}".to_string(),
            ],
            output_format: CliOutputFormat::JsonStream,
            env_vars: HashMap::new(),
            working_dir: None,
            timeout_seconds: 30,
        };
        let store = ReconciliationLedgerStore::default();
        let retained = crate::process_ledger::RetainedLedgerBatcher::spawn(
            Arc::new(store.clone()),
            Arc::new(crate::process_ledger::NoopOverflowSink),
            crate::process_ledger::LedgerBatcherConfig::default(),
        );
        let spawner = Arc::new(LiveCliSpawner::new(
            Arc::new(retained.ledger()),
            LiveCliSpawner::native_cli_registry(),
        ));
        let invocation = test_invocation();
        let receipt = tokio::task::spawn_blocking(move || {
            spawner.preflight_version(&config, "--version", Duration::from_secs(10), &invocation)
        })
        .await
        .expect("join real preflight")
        .expect("installed Codex preflight succeeds");
        assert_eq!(receipt.exit_code, Some(0));
        assert!(receipt.pid.is_some());
        assert!(
            receipt.stdout.to_ascii_lowercase().contains("codex"),
            "real version output must identify Codex: {:?}",
            receipt.stdout
        );

        assert!(matches!(
            retained.drain_and_join(Duration::from_secs(5)).await,
            crate::process_ledger::LedgerDrainJoinOutcome::Flushed
        ));
        let events = store.events.lock().unwrap().clone();
        assert_eq!(events.len(), 2, "one preflight must emit START then STOP");
        let (start, stop) = match events.as_slice() {
            [crate::process_ledger::LedgerEvent::Start(start), crate::process_ledger::LedgerEvent::Stop(stop)] => {
                (start, stop)
            }
            other => panic!("unexpected preflight lifecycle: {other:?}"),
        };
        assert_eq!(start.process_uuid, stop.process_uuid);
        assert_eq!(start.engine_kind, ProcessEngineKind::OfficialCliBridge);
        assert_eq!(start.mt_id.as_deref(), Some("MT-003"));
        assert_eq!(
            start.sandbox_adapter_id.as_deref(),
            Some(HANDSHAKE_NATIVE_SANDBOX_ADAPTER_ID)
        );
    }

    #[test]
    fn codex_exec_json_preset_is_fail_closed() {
        let mut config = good_config();
        config.cli_kind = CliKind::CodexCli;
        #[cfg(windows)]
        {
            config.executable_path = PathBuf::from("codex.cmd");
        }
        config.output_format = CliOutputFormat::JsonStream;
        config.args_template = vec![
            "exec".to_string(),
            "--json".to_string(),
            "--model".to_string(),
            "{model}".to_string(),
            "{prompt}".to_string(),
        ];
        validate_cli_preset(&config).expect("canonical Codex exec JSONL preset");

        config.args_template = vec!["exec".to_string(), "{prompt}".to_string()];
        assert!(matches!(
            validate_cli_preset(&config),
            Err(OfficialCliBridgeError::InvalidCodexPreset(_))
        ));
    }

    #[test]
    fn codex_home_bind_is_canonical_and_read_write() {
        let root =
            std::env::temp_dir().join(format!("handshake-codex-home-{}", uuid::Uuid::now_v7()));
        std::fs::create_dir_all(&root).expect("codex home fixture");
        let env = BTreeMap::from([("CODEX_HOME".to_string(), root.display().to_string())]);
        let resolved = resolve_codex_home(&env).expect("resolve Codex home");
        assert_eq!(
            resolved,
            std::fs::canonicalize(&root).expect("canonical fixture")
        );
        let _ = std::fs::remove_dir_all(root);
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
            .invoke(handle.model_id, "hello world", &test_invocation())
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
        let err = runtime
            .invoke(unknown, "x", &test_invocation())
            .expect_err("unknown model");
        assert!(matches!(err, OfficialCliBridgeError::ModelNotRegistered(_)));
    }

    #[test]
    fn spawn_failed_surfaces_through_invoke() {
        let runtime = OfficialCliBridgeRuntime::new(Arc::new(FailingSpawner));
        let handle = runtime
            .register_bridge(good_config(), "claude-3.5-sonnet", "2026-05-20T06:00:00Z")
            .expect("register");
        let err = runtime
            .invoke(handle.model_id, "hello", &test_invocation())
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
        let (ledger, _drain) = test_ledger();
        let spawner = LiveCliSpawner::new(ledger, LiveCliSpawner::native_cli_registry());
        spawner.pin_config(&config).expect("pin timeout fixture");
        let err = spawner
            .spawn(&config, &test_invocation(), "model", "100")
            .expect_err("timeout command must fail with SpawnTimeout");

        assert!(matches!(err, OfficialCliBridgeError::SpawnTimeout { .. }));
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "timeout branch must not wait for the full child sleep"
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    #[ignore]
    fn auxiliary_auth_status_canary_child() {
        println!(
            r#"{{"loggedIn":false,"email":"operator@example.invalid","refresh_token":"oauth-refresh-token-NEVER-RETURN"}}"#
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    #[ignore]
    fn foreground_watcher_sleep_child() {
        let ready_path = std::env::var_os("NO_COLOR").expect("foreground watcher readiness path");
        std::fs::write(&ready_path, std::process::id().to_string())
            .expect("publish foreground watcher child pid");
        std::thread::sleep(Duration::from_secs(60));
    }

    #[cfg(target_os = "windows")]
    const FOREGROUND_SLEEP_ARGS: &[&str] = &[
        "--ignored",
        "--exact",
        "model_runtime::cloud::official_cli_bridge::tests::foreground_watcher_sleep_child",
        "--nocapture",
    ];

    #[cfg(target_os = "windows")]
    fn foreground_failure_fixture(
        label: &str,
    ) -> (
        tempfile::TempDir,
        PathBuf,
        PathBuf,
        PathBuf,
        File,
        CliBridgeConfig,
    ) {
        let temp = tempfile::Builder::new()
            .prefix(&format!("foreground-{label}-"))
            .tempdir()
            .expect("foreground failure tempdir");
        let executable = temp.path().join("foreground-fixture.exe");
        std::fs::copy(
            std::env::current_exe().expect("current test executable"),
            &executable,
        )
        .expect("copy foreground fixture executable");
        let ready_path = temp.path().join("child.ready");
        let lock_canary_path = temp.path().join("identity-lock-canary.bin");
        std::fs::write(&lock_canary_path, b"foreground-identity-lock-canary")
            .expect("write foreground identity-lock canary");
        let (_lock_canary_identity, lock_canary) =
            locked_file_identity(&lock_canary_path).expect("lock foreground identity canary");
        let mut config = CliBridgeConfig {
            cli_kind: CliKind::Other,
            executable_path: executable.clone(),
            args_template: vec!["{prompt}".to_string()],
            output_format: CliOutputFormat::RawText,
            env_vars: HashMap::new(),
            working_dir: None,
            timeout_seconds: 5,
        };
        config.env_vars.insert(
            "NO_COLOR".to_string(),
            ready_path.to_string_lossy().into_owned(),
        );
        (
            temp,
            executable,
            ready_path,
            lock_canary_path,
            lock_canary,
            config,
        )
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn foreground_attestation_failure_reaps_exact_child_before_lock_release() {
        use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

        let (_temp, executable, _ready_path, lock_canary_path, lock_canary, config) =
            foreground_failure_fixture("attestation");
        let (ledger, _drain) = test_ledger();
        let spawner = LiveCliSpawner::new(ledger, LiveCliSpawner::native_cli_registry());
        spawner
            .pin_config(&config)
            .expect("pin attestation fixture");
        let observed_pid = Arc::new(AtomicU32::new(0));
        let lock_was_held = Arc::new(AtomicBool::new(false));
        let pid_slot = Arc::clone(&observed_pid);
        let lock_slot = Arc::clone(&lock_was_held);
        let locked_canary = lock_canary_path.clone();

        let error = spawner
            .launch_foreground_fixed_command_with_hooks(
                &config,
                FOREGROUND_SLEEP_ARGS,
                vec![lock_canary],
                spawn_foreground_watcher,
                move |pid| {
                    pid_slot.store(pid, Ordering::SeqCst);
                    lock_slot.store(
                        std::fs::remove_file(&locked_canary).is_err(),
                        Ordering::SeqCst,
                    );
                    Err("injected generation-attestation failure".to_string())
                },
                |reservation, start| reservation.begin_with_durable_ack(start),
                wait_start_durability_blocking,
            )
            .expect_err("attestation failure must fail closed");
        assert!(error.to_string().contains("attestation failed"));
        let pid = observed_pid.load(Ordering::SeqCst);
        assert_ne!(pid, 0);
        assert!(lock_was_held.load(Ordering::SeqCst));
        assert!(
            !process_is_still_active(pid),
            "attestation-failed foreground child {pid} survived owner cleanup"
        );
        std::fs::remove_file(&lock_canary_path)
            .expect("identity lock canary releases only after attestation-failed child reap");
        std::fs::remove_file(&executable).expect("remove attestation fixture executable");
    }

    #[cfg(target_os = "windows")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn foreground_ledger_begin_failure_reaps_exact_child_without_false_stop() {
        use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

        let (_temp, executable, _ready_path, lock_canary_path, lock_canary, config) =
            foreground_failure_fixture("begin");
        let (ledger, drain) = test_ledger();
        let spawner = LiveCliSpawner::new(ledger, LiveCliSpawner::native_cli_registry());
        spawner.pin_config(&config).expect("pin begin fixture");
        let observed_pid = Arc::new(AtomicU32::new(0));
        let lock_was_held = Arc::new(AtomicBool::new(false));
        let pid_slot = Arc::clone(&observed_pid);
        let lock_slot = Arc::clone(&lock_was_held);
        let locked_canary = lock_canary_path.clone();

        let error = spawner
            .launch_foreground_fixed_command_with_hooks(
                &config,
                FOREGROUND_SLEEP_ARGS,
                vec![lock_canary],
                spawn_foreground_watcher,
                move |pid| {
                    pid_slot.store(pid, Ordering::SeqCst);
                    crate::sandbox::handshake_native::process_creation_time_100ns(pid)
                        .map_err(|error| error.to_string())
                },
                move |_reservation, _start| {
                    lock_slot.store(
                        std::fs::remove_file(&locked_canary).is_err(),
                        Ordering::SeqCst,
                    );
                    Err(ProcessLedgerError::InvalidConfig(
                        "injected foreground START begin failure".to_string(),
                    ))
                },
                wait_start_durability_blocking,
            )
            .expect_err("ledger begin failure must fail closed");
        assert!(matches!(
            error,
            OfficialCliBridgeError::LedgerRegistration { .. }
        ));
        let pid = observed_pid.load(Ordering::SeqCst);
        assert_ne!(pid, 0);
        assert!(lock_was_held.load(Ordering::SeqCst));
        assert!(
            !process_is_still_active(pid),
            "ledger-begin-failed foreground child {pid} survived owner cleanup"
        );
        std::fs::remove_file(&lock_canary_path)
            .expect("identity lock canary releases only after ledger-begin-failed child reap");
        std::fs::remove_file(&executable).expect("remove ledger-begin fixture executable");
        let store = ReconciliationLedgerStore::default();
        drain
            .drain_available_to(Arc::new(store.clone()))
            .await
            .expect("drain begin-failure ledger");
        assert!(
            store.events.lock().unwrap().is_empty(),
            "failed START begin must emit neither START nor false STOP"
        );
    }

    #[cfg(target_os = "windows")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn foreground_start_ack_failure_reaps_before_matching_stop_and_lock_release() {
        use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

        #[derive(Clone)]
        struct StopOrderingStore {
            events: Arc<Mutex<Vec<crate::process_ledger::LedgerEvent>>>,
            pid: Arc<AtomicU32>,
            stop_observed_live: Arc<AtomicBool>,
            lock_canary_path: PathBuf,
            stop_observed_lock_held: Arc<AtomicBool>,
        }

        #[async_trait::async_trait]
        impl crate::process_ledger::ProcessLedgerStore for StopOrderingStore {
            async fn write_batch(
                &self,
                events: Vec<crate::process_ledger::LedgerEvent>,
            ) -> Result<(), ProcessLedgerError> {
                if events
                    .iter()
                    .any(|event| matches!(event, crate::process_ledger::LedgerEvent::Stop(_)))
                {
                    self.stop_observed_live.store(
                        process_is_still_active(self.pid.load(Ordering::SeqCst)),
                        Ordering::SeqCst,
                    );
                    self.stop_observed_lock_held.store(
                        std::fs::remove_file(&self.lock_canary_path).is_err(),
                        Ordering::SeqCst,
                    );
                }
                self.events.lock().unwrap().extend(events);
                Ok(())
            }
        }

        let (_temp, executable, _ready_path, lock_canary_path, lock_canary, config) =
            foreground_failure_fixture("start-ack");
        let observed_pid = Arc::new(AtomicU32::new(0));
        let stop_observed_live = Arc::new(AtomicBool::new(false));
        let stop_observed_lock_held = Arc::new(AtomicBool::new(false));
        let store = StopOrderingStore {
            events: Arc::new(Mutex::new(Vec::new())),
            pid: Arc::clone(&observed_pid),
            stop_observed_live: Arc::clone(&stop_observed_live),
            lock_canary_path: lock_canary_path.clone(),
            stop_observed_lock_held: Arc::clone(&stop_observed_lock_held),
        };
        let retained = crate::process_ledger::RetainedLedgerBatcher::spawn(
            Arc::new(store.clone()),
            Arc::new(crate::process_ledger::NoopOverflowSink),
            crate::process_ledger::LedgerBatcherConfig::default(),
        );
        let spawner = LiveCliSpawner::new(
            Arc::new(retained.ledger()),
            LiveCliSpawner::native_cli_registry(),
        );
        spawner.pin_config(&config).expect("pin START-ack fixture");
        let pid_slot = Arc::clone(&observed_pid);
        let locked_canary = lock_canary_path.clone();

        let error = spawner
            .launch_foreground_fixed_command_with_hooks(
                &config,
                FOREGROUND_SLEEP_ARGS,
                vec![lock_canary],
                spawn_foreground_watcher,
                move |pid| {
                    pid_slot.store(pid, Ordering::SeqCst);
                    crate::sandbox::handshake_native::process_creation_time_100ns(pid)
                        .map_err(|error| error.to_string())
                },
                |reservation, start| reservation.begin_with_durable_ack(start),
                move |acknowledgement| {
                    wait_start_durability_blocking(acknowledgement)?;
                    assert!(
                        std::fs::remove_file(&locked_canary).is_err(),
                        "identity lock released before injected START-ack cleanup"
                    );
                    Err(ProcessLedgerError::InvalidConfig(
                        "injected post-ack foreground failure".to_string(),
                    ))
                },
            )
            .expect_err("injected START-ack failure must fail closed");
        assert!(matches!(
            error,
            OfficialCliBridgeError::LedgerRegistration { .. }
        ));
        let pid = observed_pid.load(Ordering::SeqCst);
        assert_ne!(pid, 0);
        assert!(
            !process_is_still_active(pid),
            "START-ack-failed foreground child {pid} survived owner cleanup"
        );
        assert!(
            !stop_observed_live.load(Ordering::SeqCst),
            "matching STOP reached the store before exact-child reap"
        );
        assert!(
            stop_observed_lock_held.load(Ordering::SeqCst),
            "identity lock canary was not held through matching STOP durability"
        );
        std::fs::remove_file(&lock_canary_path).expect(
            "identity lock canary releases only after START-ack child reap and STOP attempt",
        );
        std::fs::remove_file(&executable).expect("remove START-ack fixture executable");
        assert!(matches!(
            retained.drain_and_join(Duration::from_secs(5)).await,
            crate::process_ledger::LedgerDrainJoinOutcome::Flushed
        ));
        let events = store.events.lock().unwrap().clone();
        let (start, stop) = match events.as_slice() {
            [crate::process_ledger::LedgerEvent::Start(start), crate::process_ledger::LedgerEvent::Stop(stop)] => {
                (start, stop)
            }
            other => panic!("unexpected START-ack failure lifecycle: {other:?}"),
        };
        assert_eq!(start.process_uuid, stop.process_uuid);
        assert_eq!(start.os_pid, Some(pid));
        assert_eq!(
            stop.stop_reason.as_deref(),
            Some("official_cli_foreground_login_start_not_durable")
        );
    }

    #[cfg(target_os = "windows")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn foreground_watcher_spawn_failure_prevents_process_and_ledger_start() {
        let ready_path = std::env::temp_dir().join(format!(
            "handshake-cli-foreground-watch-spawn-{}.ready",
            uuid::Uuid::now_v7()
        ));
        let _ = std::fs::remove_file(&ready_path);
        let (ledger, drain) = test_ledger();
        let spawner = LiveCliSpawner::new(ledger, LiveCliSpawner::native_cli_registry());
        let mut config = CliBridgeConfig {
            cli_kind: CliKind::Other,
            executable_path: std::env::current_exe().expect("current test executable"),
            args_template: vec!["{prompt}".to_string()],
            output_format: CliOutputFormat::RawText,
            env_vars: HashMap::new(),
            working_dir: None,
            timeout_seconds: 5,
        };
        config.env_vars.insert(
            "NO_COLOR".to_string(),
            ready_path.to_string_lossy().into_owned(),
        );
        spawner
            .pin_config(&config)
            .expect("pin watcher-spawn fixture");

        let error = spawner
            .launch_foreground_fixed_command_with_watcher(
                &config,
                &[
                    "--ignored",
                    "--exact",
                    "model_runtime::cloud::official_cli_bridge::tests::foreground_watcher_sleep_child",
                    "--nocapture",
                ],
                || {
                    Err(OfficialCliBridgeError::SpawnFailed {
                        reason: "injected watcher creation failure".to_string(),
                        exit_code: None,
                    })
                },
            )
            .expect_err("watcher failure must prevent foreground process creation");
        assert!(error
            .to_string()
            .contains("injected watcher creation failure"));
        std::thread::sleep(Duration::from_millis(100));
        assert!(
            !ready_path.exists(),
            "foreground child ran even though watcher capacity was unavailable"
        );

        let store = ReconciliationLedgerStore::default();
        drain
            .drain_available_to(Arc::new(store.clone()))
            .await
            .expect("drain watcher-spawn ledger");
        assert!(
            store.events.lock().unwrap().is_empty(),
            "watcher creation failure before process spawn must emit neither START nor STOP"
        );
    }

    #[cfg(target_os = "windows")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn foreground_watcher_send_failure_reaps_child_before_stop_and_lock_release() {
        let ready_path = std::env::temp_dir().join(format!(
            "handshake-cli-foreground-watch-send-{}.ready",
            uuid::Uuid::now_v7()
        ));
        let lock_path = std::env::temp_dir().join(format!(
            "handshake-cli-foreground-watch-lock-{}.bin",
            uuid::Uuid::now_v7()
        ));
        let _ = std::fs::remove_file(&ready_path);
        let _ = std::fs::remove_file(&lock_path);
        std::fs::write(&lock_path, b"identity-lock-canary").expect("write identity lock fixture");
        let (_identity, identity_lock) =
            locked_file_identity(&lock_path).expect("hold executable identity lock");

        // The interactive login now runs its exact child under a Handshake-hosted
        // ConPTY (HBR-QUIET-001: no console window, no foreground change), so the
        // ownership fixture builds the same kind of child the production path
        // hands to `ForegroundChildOwner`. Every assertion below is unchanged.
        let session = Arc::new(
            crate::terminal::PtySession::spawn(crate::terminal::PtySpawnConfig {
                shell: Some(
                    std::env::current_exe()
                        .expect("current test executable")
                        .to_string_lossy()
                        .into_owned(),
                ),
                args: vec![
                    "--ignored".to_string(),
                    "--exact".to_string(),
                    "model_runtime::cloud::official_cli_bridge::tests::foreground_watcher_sleep_child"
                        .to_string(),
                    "--nocapture".to_string(),
                ],
                cwd: None,
                env: vec![(
                    "NO_COLOR".to_string(),
                    ready_path.to_string_lossy().into_owned(),
                )],
                env_clear: false,
                rows: 24,
                cols: 120,
                scrollback_bytes: 64 * 1024,
                broadcast_capacity: 16,
            })
            .expect("spawn watcher-send fixture child under a pty"),
        );
        let pid = session
            .child_pid()
            .expect("the pty fixture child reports its pid");
        let ready_deadline = Instant::now() + Duration::from_secs(10);
        while !ready_path.exists() && Instant::now() < ready_deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        if !ready_path.exists() {
            drop(session);
            panic!("fixture child {pid} did not start");
        }
        assert!(process_is_still_active(pid), "fixture child must be live");

        let store = ReconciliationLedgerStore::default();
        let retained = crate::process_ledger::RetainedLedgerBatcher::spawn(
            Arc::new(store.clone()),
            Arc::new(crate::process_ledger::NoopOverflowSink),
            crate::process_ledger::LedgerBatcherConfig::default(),
        );
        let reservation = retained
            .ledger()
            .try_reserve_lifecycles(1)
            .expect("reserve foreground lifecycle")
            .pop()
            .expect("one foreground lifecycle");
        let start = ProcessStart::new(
            ProcessEngineKind::OfficialCliBridge,
            "MODEL_ACCESS_CLI_LOGIN",
            Some("WP-1".to_string()),
        )
        .with_os_pid(pid)
        .with_mt_id("MT-015");
        let (lifecycle, acknowledgement) = reservation
            .begin_with_durable_ack(start)
            .expect("begin foreground START");
        wait_start_durability_blocking(acknowledgement).expect("foreground START durable");
        {
            let events = store.events.lock().unwrap();
            assert!(
                matches!(
                    events.as_slice(),
                    [crate::process_ledger::LedgerEvent::Start(start)]
                        if start.os_pid == Some(pid)
                ),
                "live child must have exactly one durable START and no false STOP before watcher handoff: {events:?}"
            );
        }

        let (sender, receiver) = mpsc::sync_channel::<ForegroundWatchPayload>(1);
        drop(receiver);
        let mut owner = ForegroundChildOwner::new(session, pid, vec![identity_lock]);
        owner.attach_lifecycle(lifecycle);
        let payload = ForegroundWatchPayload { owner };
        assert!(
            std::fs::remove_file(&lock_path).is_err(),
            "identity lock was released while the exact child was still live"
        );
        let error = handoff_foreground_watch(sender, payload)
            .expect_err("closed watcher receiver must fail the handoff");
        assert!(error.to_string().contains("killed and reaped"), "{error}");
        assert!(
            !process_is_still_active(pid),
            "exact child {pid} survived watcher handoff recovery"
        );
        std::fs::remove_file(&lock_path)
            .expect("identity lock releases only after exact-child reap and STOP attempt");
        std::fs::remove_file(&ready_path).expect("remove watcher readiness file");

        assert!(matches!(
            retained.drain_and_join(Duration::from_secs(5)).await,
            crate::process_ledger::LedgerDrainJoinOutcome::Flushed
        ));
        let events = store.events.lock().unwrap().clone();
        assert_eq!(events.len(), 2, "reaped child must emit START then STOP");
        let (start, stop) = match events.as_slice() {
            [crate::process_ledger::LedgerEvent::Start(start), crate::process_ledger::LedgerEvent::Stop(stop)] => {
                (start, stop)
            }
            other => panic!("unexpected foreground lifecycle: {other:?}"),
        };
        assert_eq!(start.process_uuid, stop.process_uuid);
        assert_eq!(start.os_pid, Some(pid));
        assert_eq!(
            stop.stop_reason.as_deref(),
            Some("official_cli_foreground_login_watcher_handoff_failed")
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    #[ignore]
    fn auxiliary_auth_status_pipe_holding_grandchild() {
        use std::io::Write;

        println!("GRANDCHILD_STDOUT_CREDENTIAL_CANARY");
        eprintln!("GRANDCHILD_STDERR_CREDENTIAL_CANARY");
        std::io::stdout().flush().expect("flush grandchild stdout");
        std::io::stderr().flush().expect("flush grandchild stderr");
        let ready_path = std::env::var_os("NO_COLOR").expect("readiness path in NO_COLOR");
        std::fs::write(&ready_path, std::process::id().to_string())
            .expect("record pipe-holding grandchild pid");
        std::thread::sleep(Duration::from_secs(60));
    }

    #[cfg(target_os = "windows")]
    #[test]
    #[ignore]
    fn auxiliary_auth_status_nonzero_tree_child() {
        use std::io::Write;

        let grandchild = std::process::Command::new(
            std::env::current_exe().expect("current test executable"),
        )
        .args([
            "--ignored",
            "--exact",
            "model_runtime::cloud::official_cli_bridge::tests::auxiliary_auth_status_pipe_holding_grandchild",
            "--nocapture",
        ])
        .spawn()
        .expect("spawn pipe-holding grandchild");
        let ready_path = std::env::var_os("NO_COLOR").expect("readiness path in NO_COLOR");
        let deadline = Instant::now() + Duration::from_secs(3);
        while !std::path::Path::new(&ready_path).exists() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(
            std::path::Path::new(&ready_path).exists(),
            "grandchild {} did not confirm inherited stdout/stderr pipes",
            grandchild.id()
        );
        println!("CHILD_STDOUT_CREDENTIAL_CANARY");
        eprintln!("CHILD_STDERR_CREDENTIAL_CANARY");
        std::io::stdout().flush().expect("flush child stdout");
        std::io::stderr().flush().expect("flush child stderr");
        std::process::exit(23);
    }

    #[cfg(target_os = "windows")]
    fn process_is_still_active(pid: u32) -> bool {
        use windows_sys::Win32::{
            Foundation::{CloseHandle, STILL_ACTIVE},
            System::Threading::{
                GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
            },
        };

        let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if process.is_null() {
            return false;
        }
        let mut exit_code = 0u32;
        let ok = unsafe { GetExitCodeProcess(process, &mut exit_code) };
        unsafe {
            let _ = CloseHandle(process);
        }
        ok != 0 && exit_code == STILL_ACTIVE as u32
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn auxiliary_auth_status_nonzero_tree_is_reaped_and_pipe_canaries_are_bounded() {
        let ready_path = std::env::temp_dir().join(format!(
            "handshake-cli-auth-tree-{}.ready",
            uuid::Uuid::now_v7()
        ));
        let _ = std::fs::remove_file(&ready_path);
        let (ledger, _drain) = test_ledger();
        let spawner = LiveCliSpawner::new(ledger, LiveCliSpawner::native_cli_registry());
        let mut config = CliBridgeConfig {
            cli_kind: CliKind::Other,
            executable_path: std::env::current_exe().expect("current test executable"),
            args_template: vec!["{prompt}".to_string()],
            output_format: CliOutputFormat::RawText,
            env_vars: HashMap::new(),
            working_dir: None,
            timeout_seconds: 5,
        };
        config.env_vars.insert(
            "NO_COLOR".to_string(),
            ready_path.to_string_lossy().into_owned(),
        );
        spawner
            .pin_config(&config)
            .expect("pin child-grandchild executable graph");

        let started = Instant::now();
        let mut output = spawner
            .run_auxiliary_fixed_command(
                &config,
                &[
                    "--ignored",
                    "--exact",
                    "model_runtime::cloud::official_cli_bridge::tests::auxiliary_auth_status_nonzero_tree_child",
                    "--nocapture",
                ],
                Duration::from_secs(5),
                &test_invocation(),
                64 * 1024,
            )
            .expect("nonzero provider status remains typed runner output");
        assert!(!output.success, "child must preserve its nonzero exit");
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "grandchild pipe hold must be ended by bounded Job-tree reap"
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("CHILD_STDOUT_CREDENTIAL_CANARY"),
            "{stdout}"
        );
        assert!(
            stdout.contains("GRANDCHILD_STDOUT_CREDENTIAL_CANARY"),
            "{stdout}"
        );
        assert!(
            !stdout.contains("CHILD_STDERR_CREDENTIAL_CANARY"),
            "{stdout}"
        );
        assert!(
            !stdout.contains("GRANDCHILD_STDERR_CREDENTIAL_CANARY"),
            "{stdout}"
        );
        drop(stdout);
        output.stdout.zeroize();

        let grandchild_pid = std::fs::read_to_string(&ready_path)
            .expect("read recorded grandchild pid")
            .parse::<u32>()
            .expect("parse recorded grandchild pid");
        let inactive_deadline = Instant::now() + Duration::from_secs(1);
        while process_is_still_active(grandchild_pid) && Instant::now() < inactive_deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(
            !process_is_still_active(grandchild_pid),
            "grandchild {grandchild_pid} survived successful Job-tree reap"
        );
        std::fs::remove_file(&ready_path).expect("remove grandchild readiness file");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn foreground_login_plan_uses_pinned_absolute_graph_not_shadowed_cwd_or_bare_path() {
        let shadow_dir = std::env::temp_dir().join(format!(
            "handshake-cli-login-shadow-{}",
            uuid::Uuid::now_v7()
        ));
        std::fs::create_dir(&shadow_dir).expect("create shadow cwd");
        let executable = std::env::current_exe().expect("current test executable");
        let shadow_executable =
            shadow_dir.join(executable.file_name().expect("test executable filename"));
        std::fs::write(
            &shadow_executable,
            b"shadow executable must never be selected",
        )
        .expect("write cwd shadow");
        let (ledger, _drain) = test_ledger();
        let spawner = LiveCliSpawner::new(ledger, LiveCliSpawner::native_cli_registry());
        let config = CliBridgeConfig {
            cli_kind: CliKind::Other,
            executable_path: executable.clone(),
            args_template: vec!["{prompt}".to_string()],
            output_format: CliOutputFormat::RawText,
            env_vars: HashMap::new(),
            working_dir: Some(shadow_dir.clone()),
            timeout_seconds: 5,
        };
        spawner
            .pin_config(&config)
            .expect("pin canonical absolute executable");
        let plan = spawner
            .foreground_fixed_launch_plan(&config, &["--version"])
            .expect("build foreground plan from canonical pin");
        assert_eq!(
            plan.identity.requested_entrypoint.canonical_path,
            executable
                .canonicalize()
                .expect("canonical test executable")
        );
        assert_ne!(
            plan.identity.requested_entrypoint.canonical_path,
            shadow_executable
                .canonicalize()
                .expect("canonical cwd shadow executable")
        );

        let shadow_codex = shadow_dir.join("codex.exe");
        std::fs::write(&shadow_codex, b"PATH shadow must never be selected")
            .expect("write PATH shadow");
        let bare_cwd_config = CliBridgeConfig {
            executable_path: PathBuf::from("codex"),
            working_dir: Some(shadow_dir.clone()),
            ..config
        };
        assert!(
            spawner
                .foreground_fixed_launch_plan(&bare_cwd_config, &["login"])
                .is_err(),
            "bare program names must not resolve against a shadowed working directory"
        );
        let mut path_shadow_config = bare_cwd_config;
        path_shadow_config.env_vars.insert(
            "PATH".to_string(),
            shadow_dir.to_string_lossy().into_owned(),
        );
        assert!(matches!(
            spawner.foreground_fixed_launch_plan(&path_shadow_config, &["login"]),
            Err(OfficialCliBridgeError::UnsafeEnvironmentVariable(name)) if name == "PATH"
        ));
        std::fs::remove_file(&shadow_codex).expect("remove PATH shadow");
        std::fs::remove_file(&shadow_executable).expect("remove cwd shadow executable");
        std::fs::remove_dir(&shadow_dir).expect("remove shadow cwd");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn auxiliary_auth_status_runner_is_job_contained_bounded_and_zeroizes_canary_output() {
        let (ledger, _drain) = test_ledger();
        let spawner = LiveCliSpawner::new(ledger, LiveCliSpawner::native_cli_registry());
        let config = CliBridgeConfig {
            cli_kind: CliKind::Other,
            executable_path: std::env::current_exe().expect("current test executable"),
            args_template: vec!["{prompt}".to_string()],
            output_format: CliOutputFormat::RawText,
            env_vars: HashMap::new(),
            working_dir: None,
            timeout_seconds: 5,
        };
        let unpinned_error = spawner
            .run_auxiliary_fixed_command(
                &config,
                &["--version"],
                Duration::from_secs(1),
                &test_invocation(),
                64 * 1024,
            )
            .err()
            .expect("auxiliary runner must not register or execute an unpinned target");
        assert!(
            unpinned_error
                .to_string()
                .contains("was not registered by the canonical launch builder"),
            "{unpinned_error}"
        );
        spawner
            .pin_config(&config)
            .expect("canonical builder pins canary executable graph");
        let mut output = spawner
            .run_auxiliary_fixed_command(
                &config,
                &[
                    "--ignored",
                    "--exact",
                    "model_runtime::cloud::official_cli_bridge::tests::auxiliary_auth_status_canary_child",
                    "--nocapture",
                ],
                Duration::from_secs(5),
                &test_invocation(),
                64 * 1024,
            )
            .expect("attached auxiliary canary command");
        assert!(output.success);
        assert!(
            String::from_utf8_lossy(&output.stdout).contains("oauth-refresh-token-NEVER-RETURN"),
            "test precondition: provider output contains the credential canary"
        );
        output.stdout.zeroize();
        assert!(
            output.stdout.is_empty() || output.stdout.iter().all(|byte| *byte == 0),
            "zeroize may clear or overwrite the private output buffer"
        );

        let timeout = timeout_config();
        spawner
            .pin_config(&timeout)
            .expect("canonical builder pins timeout executable graph");
        let started = Instant::now();
        let error = spawner
            .run_auxiliary_fixed_command(
                &timeout,
                &["-t", "127.0.0.1"],
                Duration::from_secs(1),
                &test_invocation(),
                64 * 1024,
            )
            .err()
            .expect("non-terminating auxiliary command must time out");
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "auxiliary timeout must include tree termination and bounded pipe cleanup"
        );
        let rendered = error.to_string();
        assert!(rendered.contains("timed out"), "{rendered}");
        assert!(!rendered.contains("oauth-refresh-token"));
        assert!(!rendered.contains("operator@example.invalid"));
    }

    #[test]
    fn live_spawner_registry_rejects_insufficient_trust_tier_without_fallback() {
        let config = timeout_config();
        let (ledger, _drain) = test_ledger();
        let mut invocation = test_invocation();
        invocation.requested_trust_class = Some(TrustClass::Reviewed);
        invocation.requested_isolation_tier = Some(IsolationTier::Tier1Container);
        let spawner = LiveCliSpawner::new(ledger, LiveCliSpawner::native_cli_registry());
        spawner.pin_config(&config).expect("pin tier fixture");
        let err = spawner
            .spawn(&config, &invocation, "model", "prompt")
            .expect_err("reviewed workload must not downgrade to Tier0 native execution");
        assert!(matches!(err, OfficialCliBridgeError::SpawnFailed { .. }));
    }

    #[test]
    fn live_spawner_registry_rejects_unsatisfied_capability_without_fallback() {
        let config = timeout_config();
        let (ledger, _drain) = test_ledger();
        let mut invocation = test_invocation();
        invocation.requested_sandbox_capabilities = Some(BTreeSet::from([
            RequiredCapability::VeryStrongFilesystemIsolation,
        ]));
        let spawner = LiveCliSpawner::new(ledger, LiveCliSpawner::native_cli_registry());
        spawner.pin_config(&config).expect("pin capability fixture");
        let err = spawner
            .spawn(&config, &invocation, "model", "prompt")
            .expect_err("native adapter must reject unavailable isolation capability");
        assert!(matches!(err, OfficialCliBridgeError::SpawnFailed { .. }));
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
        let mut invocation = test_invocation();
        let registered_model_id = ModelId::new_v7();
        invocation.registered_model_id = Some(registered_model_id);
        let executable = file_identity(&good_config().executable_path).expect("fixture identity");
        let identity = CliLaunchIdentity {
            requested_entrypoint: executable.clone(),
            effective_executable: executable,
            effective_script: None,
            launcher_package_manifest: None,
            platform_package_manifest: None,
            final_native_executable: None,
        };
        let meta = cli_bridge_spawn_meta(7777, &invocation, "claude-3.5-sonnet", &identity);
        assert_eq!(meta.pid, 7777);
        assert_eq!(meta.engine_kind, ProcessEngineKind::OfficialCliBridge);
        assert_eq!(meta.owner_role, "TEST_ROLE");
        assert_eq!(meta.owner_wp.as_deref(), Some("WP-TEST"));
        assert_eq!(meta.role_id.as_deref(), Some("TEST_ROLE"));
        assert_eq!(meta.wp_id.as_deref(), Some("WP-TEST"));
        assert_eq!(meta.mt_id.as_deref(), Some("MT-003"));
        assert_eq!(meta.session_id.as_deref(), Some("session-test"));
        assert_eq!(meta.parent_session_id.as_deref(), Some("session-parent"));
        assert_eq!(meta.trace_id.as_deref(), Some("trace-test"));
        assert_eq!(meta.span_id.as_deref(), Some("span-test"));
        assert_eq!(meta.cancellation_id.as_deref(), Some("cancel-test"));
        assert_eq!(meta.reclaim_key.as_deref(), Some("reclaim-test"));
        assert_eq!(meta.model_identity.as_deref(), Some("claude-3.5-sonnet"));
        assert_eq!(
            meta.model_id.as_deref(),
            Some(registered_model_id.to_string().as_str())
        );
        assert_eq!(
            meta.metadata_blob["subprocess_kind"].as_str(),
            Some("official_cli_bridge")
        );
        assert_eq!(
            meta.metadata_blob["selected_model_name"].as_str(),
            Some("claude-3.5-sonnet")
        );
        assert_eq!(
            meta.metadata_blob["requested_model_identity"].as_str(),
            Some("test-model")
        );
        assert_eq!(meta.metadata_blob["owner_wp"].as_str(), Some("WP-TEST"));
        assert_eq!(meta.metadata_blob["mt_id"].as_str(), Some("MT-003"));
        assert!(meta.metadata_blob["effective_executable"]
            .as_str()
            .unwrap()
            .ends_with("Cargo.toml"));
    }

    #[tokio::test]
    async fn failed_terminate_and_reap_leaves_start_open_without_stop() {
        let (ledger, drain) = test_ledger();
        let reservation = ledger
            .try_reserve_lifecycles(1)
            .expect("reserve lifecycle")
            .pop()
            .expect("one lifecycle reservation");
        let start = ProcessStart::new(
            ProcessEngineKind::OfficialCliBridge,
            "REAP_FAILURE_TEST",
            Some("WP-TEST".to_string()),
        )
        .with_os_pid(9090)
        .with_mt_id("MT-003");
        let lifecycle = reservation.begin(start).expect("record START");
        let capabilities = HandshakeNativeSandboxAdapter::new().capabilities();
        let executable = file_identity(&good_config().executable_path).expect("fixture identity");
        let mut child = GuardedCliChild::new(
            Box::new(ReapFailingAttachedProcess { pid: 9090 }),
            capabilities,
            CliLaunchIdentity {
                requested_entrypoint: executable.clone(),
                effective_executable: executable,
                effective_script: None,
                launcher_package_manifest: None,
                platform_package_manifest: None,
                final_native_executable: None,
            },
            test_resolved_execution_policy(),
            Vec::new(),
            None,
        );
        child.attach_lifecycle(lifecycle);
        drop(child);

        let store = ReconciliationLedgerStore::default();
        drain
            .drain_available_to(Arc::new(store.clone()))
            .await
            .expect("drain ledger");
        let events = store.events.lock().unwrap().clone();
        assert_eq!(events.len(), 1, "failed reap must not fabricate STOP");
        assert!(matches!(
            events.as_slice(),
            [crate::process_ledger::LedgerEvent::Start(_)]
        ));
    }

    #[tokio::test]
    async fn explicit_failed_terminate_leaves_start_open_without_stop() {
        let (ledger, drain) = test_ledger();
        let reservation = ledger
            .try_reserve_lifecycles(1)
            .expect("reserve lifecycle")
            .pop()
            .expect("one lifecycle reservation");
        let start = ProcessStart::new(
            ProcessEngineKind::OfficialCliBridge,
            "REAP_FAILURE_TEST",
            Some("WP-TEST".to_string()),
        )
        .with_os_pid(9091)
        .with_mt_id("MT-003");
        let lifecycle = reservation.begin(start).expect("record START");
        let capabilities = HandshakeNativeSandboxAdapter::new().capabilities();
        let executable = file_identity(&good_config().executable_path).expect("fixture identity");
        let mut child = GuardedCliChild::new(
            Box::new(ReapFailingAttachedProcess { pid: 9091 }),
            capabilities,
            CliLaunchIdentity {
                requested_entrypoint: executable.clone(),
                effective_executable: executable,
                effective_script: None,
                launcher_package_manifest: None,
                platform_package_manifest: None,
                final_native_executable: None,
            },
            test_resolved_execution_policy(),
            Vec::new(),
            None,
        );
        child.attach_lifecycle(lifecycle);
        assert!(child
            .terminate_and_collect("explicit_reap_failure_test")
            .is_none());
        drop(child);

        let store = ReconciliationLedgerStore::default();
        drain
            .drain_available_to(Arc::new(store.clone()))
            .await
            .expect("drain ledger");
        let events = store.events.lock().unwrap().clone();
        assert_eq!(events.len(), 1, "failed reap must not fabricate STOP");
        assert!(matches!(
            events.as_slice(),
            [crate::process_ledger::LedgerEvent::Start(_)]
        ));
    }

    // -----------------------------------------------------------------------
    // MT-019 F1 + P-5: the running-app reclaim hook.
    //
    // `GuardedCliChild` is private, so this wiring is exercised by the in-crate
    // `official_cli_bridge::tests::mt019_*` cases. The current MT-015 integration
    // target is `cli_bridge_login_quiet_tests`, backed by embedded SurrealDB.
    // -----------------------------------------------------------------------

    #[derive(Default)]
    struct RecordingOwnedProcessClaimStore {
        owned_claims: std::sync::Mutex<Vec<(uuid::Uuid, uuid::Uuid)>>,
    }

    #[async_trait::async_trait]
    impl crate::process_ledger::ReclaimProcessStore for RecordingOwnedProcessClaimStore {
        async fn active_processes_for_session(
            &self,
            _session_id: &str,
        ) -> Result<
            Vec<crate::process_ledger::ReclaimableProcess>,
            crate::process_ledger::ProcessLedgerError,
        > {
            panic!("the running-app CLI reap must never use the session-wide claim");
        }

        async fn active_owned_process(
            &self,
            process_uuid: uuid::Uuid,
            owner_runtime_instance_id: uuid::Uuid,
        ) -> Result<
            Option<crate::process_ledger::ReclaimableProcess>,
            crate::process_ledger::ProcessLedgerError,
        > {
            self.owned_claims
                .lock()
                .unwrap()
                .push((process_uuid, owner_runtime_instance_id));
            Ok(None)
        }

        async fn renew_reclaim_claim(
            &self,
            _process_uuid: uuid::Uuid,
            claim: &crate::process_ledger::ReclaimClaim,
        ) -> Result<crate::process_ledger::ReclaimClaim, crate::process_ledger::ProcessLedgerError>
        {
            Ok(claim.clone())
        }

        async fn mark_reclaim_kill_succeeded(
            &self,
            _stop: &crate::process_ledger::ProcessStop,
            _claim: &crate::process_ledger::ReclaimClaim,
        ) -> Result<(), crate::process_ledger::ProcessLedgerError> {
            Ok(())
        }

        async fn mark_reclaim_kill_started(
            &self,
            _process_uuid: uuid::Uuid,
            _claim: &crate::process_ledger::ReclaimClaim,
        ) -> Result<(), crate::process_ledger::ProcessLedgerError> {
            Ok(())
        }

        async fn release_reclaim_claim(
            &self,
            _process_uuid: uuid::Uuid,
            _claim: &crate::process_ledger::ReclaimClaim,
        ) -> Result<(), crate::process_ledger::ProcessLedgerError> {
            Ok(())
        }

        async fn resolve_reclaim_kill_operation(
            &self,
            _process_uuid: uuid::Uuid,
            _kill_operation_uuid: uuid::Uuid,
            _status: crate::process_ledger::ReclaimKillOperationStatus,
        ) -> Result<(), crate::process_ledger::ProcessLedgerError> {
            Ok(())
        }

        async fn in_progress_kill_operations_for_session(
            &self,
            _session_id: &str,
            _limit: usize,
        ) -> Result<
            Vec<crate::process_ledger::ReclaimKillOperationCandidate>,
            crate::process_ledger::ProcessLedgerError,
        > {
            Ok(Vec::new())
        }
    }

    struct NeverCalledKill;

    #[async_trait::async_trait]
    impl crate::process_ledger::SandboxKill for NeverCalledKill {
        async fn kill(
            &self,
            _process_uuid: uuid::Uuid,
            _kill_operation_uuid: uuid::Uuid,
        ) -> Result<(), crate::process_ledger::KillError> {
            panic!("no row was claimed, so no kill may be attempted");
        }

        async fn kill_operation_status(
            &self,
            _process_uuid: uuid::Uuid,
            _kill_operation_uuid: uuid::Uuid,
        ) -> Result<
            crate::process_ledger::ReclaimKillOperationStatus,
            crate::process_ledger::KillError,
        > {
            Ok(crate::process_ledger::ReclaimKillOperationStatus::NotStarted)
        }
    }

    fn hook_test_runtime_owner(
        instance_id: uuid::Uuid,
    ) -> crate::process_ledger::ProcessRuntimeOwner {
        crate::process_ledger::ProcessRuntimeOwner {
            runtime_instance_id: instance_id,
            host_scope_id: "mt019-hook-test-host".to_string(),
            lease_schema_id: crate::process_ledger::EMBEDDED_RUNTIME_INSTANCE_SCHEMA_ID.to_string(),
            lease_protocol: crate::process_ledger::EMBEDDED_RUNTIME_LOOPBACK_UDP_PROTOCOL
                .to_string(),
            lease_address: "127.0.0.1".to_string(),
            lease_port: 51_234,
        }
    }

    fn hook_test_guard(
        ledger: &LedgerBatcher,
        pid: u32,
        instance_id: uuid::Uuid,
        reclaim: Option<Arc<crate::process_ledger::Reclaim>>,
    ) -> (GuardedCliChild, uuid::Uuid) {
        let reservation = ledger
            .try_reserve_lifecycles(1)
            .expect("reserve lifecycle")
            .pop()
            .expect("one lifecycle reservation");
        let start = ProcessStart::new(
            ProcessEngineKind::OfficialCliBridge,
            "MT019_HOOK_TEST",
            Some("WP-1".to_string()),
        )
        .with_os_pid(pid)
        .with_mt_id("MT-019")
        .with_runtime_owner(hook_test_runtime_owner(instance_id));
        let process_uuid = start.process_uuid;
        let lifecycle = reservation.begin(start).expect("record START");
        let executable = file_identity(&good_config().executable_path).expect("fixture identity");
        let mut child = GuardedCliChild::new(
            Box::new(ReapFailingAttachedProcess { pid }),
            HandshakeNativeSandboxAdapter::new().capabilities(),
            CliLaunchIdentity {
                requested_entrypoint: executable.clone(),
                effective_executable: executable,
                effective_script: None,
                launcher_package_manifest: None,
                platform_package_manifest: None,
                final_native_executable: None,
            },
            test_resolved_execution_policy(),
            Vec::new(),
            reclaim,
        );
        child.attach_lifecycle(lifecycle);
        (child, process_uuid)
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn mt019_reap_failure_invokes_the_owner_scoped_running_app_reclaim() {
        let (ledger, _drain) = test_ledger();
        let store = Arc::new(RecordingOwnedProcessClaimStore::default());
        let reclaim = Arc::new(crate::process_ledger::Reclaim::new(
            Arc::clone(&store),
            Arc::new(NeverCalledKill),
            ledger.clone(),
        ));
        let instance_id = uuid::Uuid::now_v7();
        let (mut child, process_uuid) = hook_test_guard(&ledger, 9092, instance_id, Some(reclaim));

        assert!(child
            .terminate_and_collect("mt019_reap_failure_hook_test")
            .is_none());

        let claims = store.owned_claims.lock().unwrap().clone();
        assert_eq!(
            claims,
            vec![(process_uuid, instance_id)],
            "the reap-failure path must reclaim EXACTLY this process through the owner-scoped claim"
        );
        drop(child);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn mt019_drop_path_never_invokes_the_running_app_reclaim() {
        let (ledger, _drain) = test_ledger();
        let store = Arc::new(RecordingOwnedProcessClaimStore::default());
        let reclaim = Arc::new(crate::process_ledger::Reclaim::new(
            Arc::clone(&store),
            Arc::new(NeverCalledKill),
            ledger.clone(),
        ));
        let instance_id = uuid::Uuid::now_v7();
        let (child, _process_uuid) = hook_test_guard(&ledger, 9093, instance_id, Some(reclaim));

        // Drop unwinds through `leave_open_for_reconciliation`, never through the
        // reclaiming variant: blocking on a runtime from Drop during unwind is a
        // double panic (abort). Those rows are the periodic pass's job.
        drop(child);

        assert!(
            store.owned_claims.lock().unwrap().is_empty(),
            "Drop must not invoke the async running-app reclaim"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn mt019_reclaim_hook_is_skipped_without_a_runtime_owner_descriptor() {
        let (ledger, _drain) = test_ledger();
        let store = Arc::new(RecordingOwnedProcessClaimStore::default());
        let reclaim = Arc::new(crate::process_ledger::Reclaim::new(
            Arc::clone(&store),
            Arc::new(NeverCalledKill),
            ledger.clone(),
        ));
        let reservation = ledger
            .try_reserve_lifecycles(1)
            .expect("reserve lifecycle")
            .pop()
            .expect("one lifecycle reservation");
        let start = ProcessStart::new(
            ProcessEngineKind::OfficialCliBridge,
            "MT019_HOOK_TEST",
            Some("WP-1".to_string()),
        )
        .with_os_pid(9094)
        .with_mt_id("MT-019");
        let lifecycle = reservation.begin(start).expect("record START");
        let executable = file_identity(&good_config().executable_path).expect("fixture identity");
        let mut child = GuardedCliChild::new(
            Box::new(ReapFailingAttachedProcess { pid: 9094 }),
            HandshakeNativeSandboxAdapter::new().capabilities(),
            CliLaunchIdentity {
                requested_entrypoint: executable.clone(),
                effective_executable: executable,
                effective_script: None,
                launcher_package_manifest: None,
                platform_package_manifest: None,
                final_native_executable: None,
            },
            test_resolved_execution_policy(),
            Vec::new(),
            Some(reclaim),
        );
        child.attach_lifecycle(lifecycle);

        assert!(child
            .terminate_and_collect("mt019_no_owner_hook_test")
            .is_none());

        assert!(
            store.owned_claims.lock().unwrap().is_empty(),
            "without a runtime-owner descriptor there is no ownership proof, so no claim may be attempted"
        );
        drop(child);
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
    fn inherited_environment_is_an_explicit_runtime_allowlist() {
        for runtime_var in [
            "PATH",
            "USERPROFILE",
            "APPDATA",
            "LOCALAPPDATA",
            "SystemRoot",
            "TEMP",
            "HOME",
            "ComSpec",
            "CODEX_HOME",
        ] {
            assert!(
                is_inherited_runtime_env_name(runtime_var),
                "{runtime_var} is required runtime state and must be inherited"
            );
        }
        for untrusted in [
            "OPENAI_API_KEY",
            "CODEX_ACCESS_TOKEN",
            "ANTHROPIC_API_KEY",
            "CLAUDE_CODE_OAUTH_TOKEN",
            "GITHUB_PAT",
            "DATABASE_URL",
            "KUBECONFIG",
            "SENTRY_DSN",
            "NODE_OPTIONS",
            "SCRUB_PROBE_PUBLIC_DIR",
        ] {
            assert!(
                !is_inherited_runtime_env_name(untrusted),
                "{untrusted} must not cross the boundary through parent inheritance"
            );
        }
    }

    #[test]
    fn explicit_environment_rejects_execution_controls() {
        for name in [
            "NODE_OPTIONS",
            "LD_PRELOAD",
            "PYTHONPATH",
            "GIT_ASKPASS",
            "BASH_ENV",
        ] {
            let env = HashMap::from([(name.to_string(), "attacker-controlled".to_string())]);
            assert!(matches!(
                validate_config_environment(&env),
                Err(OfficialCliBridgeError::UnsafeEnvironmentVariable(rejected))
                    if rejected == name
            ));
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
            _invocation: &CliInvocationContext,
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
            _invocation: &CliInvocationContext,
            _model_name: &str,
            _prompt: &str,
            chunk_sender: &tokio::sync::mpsc::Sender<Vec<u8>>,
        ) -> Result<CliInvocationReceipt, OfficialCliBridgeError> {
            let mut full = Vec::new();
            for chunk in &self.chunks {
                // Deliver each chunk LIVE, as if read incrementally from a pipe.
                deliver_cli_chunk(chunk_sender, chunk)?;
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
            .invoke_with_capture(handle.model_id, "hello", test_invocation(), &term, binding)
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

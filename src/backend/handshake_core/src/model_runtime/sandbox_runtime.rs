//! WP-KERNEL-004 Wave 1: [`SandboxModelRuntime`] — a swarm `ModelRuntime` that
//! drives a REAL local model (a llama.cpp runner + a GGUF) INSIDE a Tier-3
//! Cloud Hypervisor microVM through the injected [`SandboxAdapter`] seam.
//!
//! # Design: ephemeral-exec-per-`generate`
//!
//! `generate()` does ONE `adapter.exec()` that boots a fresh CH microVM, runs
//! the llama runner one-shot on the prompt, captures the framed serial
//! completion, and chunks it into [`GeneratedToken`]s. This is forced, not
//! preferred:
//!
//! - The persistent-VM `exec` path FAILS CLOSED today
//!   (`cloud_hypervisor/adapter.rs`): exec into a running snapshot-capable VM
//!   needs a vsock guest agent (out of scope). The idle initramfs only loops
//!   printing `TICK`; there is no command channel into a live VM. So a
//!   persistent serial-daemon inference path is NOT buildable in wave 1.
//! - The ephemeral `exec` path is real and complete: it bakes the declared
//!   binds into a per-exec initramfs, base64-encodes argv onto the kernel
//!   cmdline, boots CH, parses the `---HSK-BEGIN .. ---HSK-END rc=N---` framing,
//!   and returns `ExecResult{ exit_code, stdout, stderr: <empty by design>,
//!   duration_ms }`.
//!
//! HONESTY BOUNDARY (stated in code + reports): the model RELOADS every
//! `generate()` (cold start per call), and the per-token stream is post-hoc
//! chunking of a captured completion, NOT live per-token decode. A persistent
//! in-VM serial daemon with real per-token streaming is a FLAGGED FOLLOW-ON,
//! blocked on the same vsock-guest-agent gap.
//!
//! The live end-to-end (a model actually inferring inside a CH microVM) requires
//! the operator KVM/WSL/Cloud-Hypervisor desktop environment and CANNOT run in
//! the headless agent/CI. The plumbing is built for real and unit-tested against
//! a FAKE [`SandboxAdapter`] (injected); the live CH end-to-end is an
//! `#[ignore]`-d operator-desktop test (see `tests`). The live path is NEVER
//! faked as passing.
//!
//! # Bridge shape
//!
//! `generate()` mirrors `model_runtime/cloud/cli_bridge_runtime.rs` exactly: a
//! dedicated OS thread owns the `Arc<dyn SandboxAdapter>` + the built `Command`
//! and `block_on(adapter.exec(..))` (the blocking boot must not occupy a tokio
//! worker), sending each decoded chunk onto a `tokio::sync::mpsc` channel; the
//! async side drains the channel through `stream::unfold` and appends the single
//! terminal token. Exec failure / non-zero exit surface as an HONEST stream
//! error item — never a silently empty stream.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use futures::stream;

use crate::sandbox::{
    AdapterId, BindMode, Command, ImageRef, NetPolicy, ProcessHandle, ProcessSpec, ResourceLimits,
    SandboxAdapter, Signal, TrustClass,
};

use super::{
    error::ModelRuntimeError, CancellationToken, Embedding, FinishReason, GenerateRequest,
    GeneratedToken, KvCacheHandle, LoadSpec, LoraStackHandle, ModelCapabilities, ModelId,
    ModelRuntime, Score, SteeringHookHandle, TokenStream,
};

/// Adapter name surfaced through [`ModelRuntime::adapter_name`] and used as the
/// `adapter` field on `CapabilityNotSupported` errors.
pub const SANDBOX_RUNTIME_ADAPTER: &str = "sandbox_model_runtime";

/// Guest mount point for the directory containing the GGUF. The runner reads the
/// model at `<GGUF_GUEST_ROOT>/<filename>`. Must match the bind root (asserted
/// in the unit test) so a guest-path mismatch (research risk #4) cannot ship.
pub const GGUF_GUEST_ROOT: &str = "/models";

/// Configuration for one boxed local model: the host GGUF, the dir bound
/// read-only into the guest, the runner's guest path, and the sizing/trust
/// inputs that flow onto the [`ProcessSpec`]/[`Command`].
#[derive(Clone, Debug)]
pub struct SandboxModelConfig {
    /// Host path to the `.gguf` weights file (validated to exist at `load`).
    pub gguf_host_path: PathBuf,
    /// Host directory bound ReadOnly under [`GGUF_GUEST_ROOT`]. The GGUF must
    /// live under this dir; the guest sees it at
    /// `<GGUF_GUEST_ROOT>/<gguf filename>`.
    pub gguf_root_bind: PathBuf,
    /// Guest path / PATH name of the llama runner (e.g. `"llama-cli"` if on the
    /// guest rootfs PATH, or an absolute `/usr/bin/llama-cli`).
    pub runner_guest_path: String,
    /// Trust class for the boxed workload. Defaults to the conservative
    /// [`TrustClass::UntrustedAgent`], whose `min_isolation_tier()` is
    /// `Tier3Microvm`, so registry selection enforces the microVM minimum.
    pub trust_class: TrustClass,
    /// Optional guest memory cap (research risk #2: a 3B-Q4 model is ~2-4 GB).
    pub memory_bytes: Option<u64>,
    /// Optional per-exec timeout; falls back to the adapter's configured default.
    pub timeout_ms: Option<u64>,
}

impl SandboxModelConfig {
    /// Construct a config with the conservative `UntrustedAgent` trust class
    /// (forces the Tier-3 microVM minimum at selection time).
    pub fn new(
        gguf_host_path: impl Into<PathBuf>,
        gguf_root_bind: impl Into<PathBuf>,
        runner_guest_path: impl Into<String>,
    ) -> Self {
        Self {
            gguf_host_path: gguf_host_path.into(),
            gguf_root_bind: gguf_root_bind.into(),
            runner_guest_path: runner_guest_path.into(),
            trust_class: TrustClass::UntrustedAgent,
            memory_bytes: None,
            timeout_ms: None,
        }
    }

    pub fn with_memory_bytes(mut self, bytes: u64) -> Self {
        self.memory_bytes = Some(bytes);
        self
    }

    pub fn with_timeout_ms(mut self, ms: u64) -> Self {
        self.timeout_ms = Some(ms);
        self
    }

    pub fn with_trust_class(mut self, trust_class: TrustClass) -> Self {
        self.trust_class = trust_class;
        self
    }

    /// The GGUF filename (last path component) the guest sees under
    /// [`GGUF_GUEST_ROOT`]. Used both to build the guest `--model` path and to
    /// assert the bind/guest-path agreement.
    pub fn gguf_filename(&self) -> String {
        self.gguf_host_path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    /// Absolute guest path the runner reads the model from.
    pub fn guest_gguf_path(&self) -> String {
        format!("{}/{}", GGUF_GUEST_ROOT.trim_end_matches('/'), self.gguf_filename())
    }
}

/// Build the inference [`ProcessSpec`] for a boxed local model. Lifts the
/// builder SHAPE from `model_runtime/sandbox_binding.rs::process_spec_from_model_spec`
/// but carries the runner argv on the per-exec [`Command`] (the ephemeral CH
/// path takes argv from the command, not `ProcessSpec.cmd`). The GGUF dir is
/// declared as the ReadOnly bind under [`GGUF_GUEST_ROOT`]; `net_policy` is
/// `DenyAll` (CH microVMs boot with no network device); `trust_class` flows from
/// the config so an `UntrustedAgent` workload forces the Tier-3 minimum.
pub fn inference_process_spec(model_id: ModelId, cfg: &SandboxModelConfig) -> ProcessSpec {
    ProcessSpec {
        id: AdapterId::new(format!("sandbox-model:{model_id}")),
        image_or_root: ImageRef::new("llama_cpp"),
        // argv is carried on the per-exec Command (ephemeral exec path), not here.
        cmd: vec![],
        env: BTreeMap::new(),
        cwd: None,
        binds: vec![crate::sandbox::BindSpec {
            host_path: cfg.gguf_root_bind.clone(),
            guest_path: PathBuf::from(GGUF_GUEST_ROOT),
            mode: BindMode::ReadOnly,
        }],
        net_policy: NetPolicy::DenyAll,
        resource_limits: ResourceLimits {
            memory_bytes: cfg.memory_bytes,
            timeout_ms: cfg.timeout_ms,
            ..Default::default()
        },
        required_capabilities: std::collections::BTreeSet::new(),
        trust_class: cfg.trust_class,
        // No `hsk.sandbox.mode=persistent` marker => the proven ephemeral
        // per-exec boot path. Persistent mode is the snapshot/restore seam only.
        metadata: BTreeMap::new(),
    }
}

/// Build the per-exec inference [`Command`]: the runner reads the bound GGUF and
/// runs the prompt one-shot, with stdout = completion only (research risk #5:
/// `--no-display-prompt` + `--log-disable` keep prompt echo / log noise off the
/// captured serial stream; `stderr` is empty by adapter design).
pub fn inference_command(cfg: &SandboxModelConfig, req: &GenerateRequest) -> Command {
    Command {
        argv: vec![
            cfg.runner_guest_path.clone(),
            "--model".to_string(),
            cfg.guest_gguf_path(),
            "-p".to_string(),
            req.prompt.text.clone(),
            "-n".to_string(),
            req.max_tokens.to_string(),
            // one-shot, no interactive REPL
            "-no-cnv".to_string(),
            "--single-turn".to_string(),
            // stdout = completion only
            "--no-display-prompt".to_string(),
            // keep llama.cpp's own logs off the captured stream
            "--log-disable".to_string(),
        ],
        env_overlay: BTreeMap::new(),
        stdin: None,
        timeout_ms: cfg.timeout_ms,
    }
}

/// One-shot stream that yields a single error item (preflight failure surfaced
/// inside the [`TokenStream`] contract). Mirrors the CLI bridge sibling.
fn single_error_stream(err: ModelRuntimeError) -> TokenStream {
    Box::pin(stream::iter([Err(err)]))
}

/// Terminal token carrying only a finish reason (mirrors the CLI bridge sibling).
fn terminal_token(reason: FinishReason) -> GeneratedToken {
    GeneratedToken {
        token_id: 0,
        text: String::new(),
        logprob: None,
        finish_reason: Some(reason),
    }
}

/// Chunk size (bytes of completion) per emitted [`GeneratedToken`]. The captured
/// serial completion is post-hoc chunked into pseudo-tokens (HONEST: not real
/// per-token decode). A modest size keeps the swarm capture seam's terminal
/// panel responsive without flooding it.
const COMPLETION_CHUNK_BYTES: usize = 24;

/// Split a UTF-8 completion into `COMPLETION_CHUNK_BYTES`-ish pieces on CHAR
/// boundaries (never mid-codepoint, so each `GeneratedToken::text` is valid
/// UTF-8 — the trait contract).
fn chunk_completion(text: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        if current.len() + ch.len_utf8() > COMPLETION_CHUNK_BYTES && !current.is_empty() {
            out.push(std::mem::take(&mut current));
        }
        current.push(ch);
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

/// Async stream state carried through `stream::unfold`. The blocking exec thread
/// produces tokens onto the channel; the async side drains them and appends the
/// terminal token exactly once when the producer closes.
struct StreamState {
    rx: tokio::sync::mpsc::UnboundedReceiver<Result<GeneratedToken, ModelRuntimeError>>,
}

/// First-class swarm `ModelRuntime` that proxies `generate()` into a Tier-3
/// Cloud Hypervisor microVM via the injected [`SandboxAdapter`].
pub struct SandboxModelRuntime {
    /// Injected sandbox adapter (the real `CloudHypervisorAdapter` in
    /// production, a fake in tests). Shared so the OS thread in `generate()`
    /// can own a clone for the blocking `exec`.
    adapter: Arc<dyn SandboxAdapter>,
    /// Boxed-model config (GGUF host path/root, runner guest path, sizing).
    cfg: SandboxModelConfig,
    /// The ephemeral handle minted at `load()` via `adapter.spawn`, with the
    /// GGUF dir bound via `adapter.fs_bind` so the per-exec initramfs bakes it.
    handle: Option<ProcessHandle>,
    /// The `ModelId` minted at `load()`.
    model_id: ModelId,
    /// All-false declared capabilities (mirror the CLI bridge: a sandbox runner
    /// exposes no local inference techniques through the serial exec channel).
    declared_capabilities: ModelCapabilities,
    /// Runtime-wide cancellation flag, set by `cancel()`; mirrors the CLI
    /// bridge `runtime_cancel`.
    runtime_cancel: CancellationToken,
}

impl std::fmt::Debug for SandboxModelRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SandboxModelRuntime")
            .field("adapter", &"<Arc<dyn SandboxAdapter>>")
            .field("cfg", &self.cfg)
            .field("model_id", &self.model_id)
            .finish()
    }
}

impl SandboxModelRuntime {
    /// Construct an unloaded runtime around an injected adapter + config. Call
    /// [`ModelRuntime::load`] to validate the GGUF, mint the ephemeral handle,
    /// and bind the model dir before `generate()`.
    pub fn new(adapter: Arc<dyn SandboxAdapter>, cfg: SandboxModelConfig) -> Self {
        Self {
            adapter,
            cfg,
            handle: None,
            model_id: ModelId::new_v7(),
            declared_capabilities: ModelCapabilities::default(),
            runtime_cancel: CancellationToken::new(),
        }
    }

    /// The ephemeral sandbox handle minted at `load()`, if loaded. Exposed so
    /// the factory can read `sandbox_internal_id` for the process-ledger START
    /// and so teardown can `kill` it.
    pub fn handle(&self) -> Option<&ProcessHandle> {
        self.handle.as_ref()
    }

    /// The adapter id of the selected sandbox (e.g. `cloud_hypervisor`), read
    /// from the adapter capabilities. Used by the factory for the ledger
    /// `sandbox_adapter_id` field.
    pub fn adapter_id(&self) -> AdapterId {
        self.adapter.capabilities().adapter_id
    }

    pub fn model_id(&self) -> ModelId {
        self.model_id
    }

    /// Shared clone of the injected adapter (so the factory teardown can `kill`
    /// the handle without holding the runtime).
    pub fn adapter(&self) -> Arc<dyn SandboxAdapter> {
        Arc::clone(&self.adapter)
    }

    fn streaming_generate(&self, req: GenerateRequest) -> TokenStream {
        let Some(handle) = self.handle.clone() else {
            return single_error_stream(ModelRuntimeError::GenerateError(
                "sandbox model runtime generate before load: call load() to mint the microVM \
                 handle and bind the GGUF first"
                    .to_string(),
            ));
        };

        let (tx, rx) =
            tokio::sync::mpsc::unbounded_channel::<Result<GeneratedToken, ModelRuntimeError>>();

        let adapter = Arc::clone(&self.adapter);
        let command = inference_command(&self.cfg, &req);
        let req_cancel = req.cancel.clone();
        let runtime_cancel = self.runtime_cancel.clone();

        // The blocking microVM boot must NOT run on a tokio worker thread: run it
        // on a dedicated OS thread that owns the adapter + command and drives the
        // async `exec` to completion on a tiny current-thread runtime, then sends
        // the decoded completion onto the tokio channel (mirror the CLI bridge).
        std::thread::spawn(move || {
            // Pre-exec cancellation: if cancelled before the boot starts, end the
            // stream as Cancelled without booting a VM.
            if req_cancel.is_cancelled() || runtime_cancel.is_cancelled() {
                let _ = tx.send(Ok(terminal_token(FinishReason::Cancelled)));
                return;
            }

            let exec_result = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt.block_on(adapter.exec(&handle, command)),
                Err(err) => {
                    let _ = tx.send(Err(ModelRuntimeError::GenerateError(format!(
                        "sandbox exec runtime build failed: {err}"
                    ))));
                    return;
                }
            };

            match exec_result {
                Ok(result) => {
                    if result.exit_code != 0 {
                        // HONEST error item — non-zero runner exit (e.g. OOM,
                        // research risk #2) is surfaced, NOT a silent empty
                        // stream. stderr is empty by adapter design, so the exit
                        // code carries the signal.
                        let _ = tx.send(Err(ModelRuntimeError::GenerateError(format!(
                            "sandbox runner exited non-zero (exit_code={}); the microVM serial \
                             capture returned {} stdout bytes",
                            result.exit_code,
                            result.stdout.len()
                        ))));
                        return;
                    }
                    let completion = String::from_utf8_lossy(&result.stdout).into_owned();
                    let mut token_index: u32 = 0;
                    for chunk in chunk_completion(&completion) {
                        token_index = token_index.saturating_add(1);
                        if tx
                            .send(Ok(GeneratedToken {
                                token_id: token_index,
                                text: chunk,
                                logprob: None,
                                finish_reason: None,
                            }))
                            .is_err()
                        {
                            // Receiver dropped: caller no longer cares.
                            return;
                        }
                    }
                    // Terminal token reflects late cancellation vs normal stop.
                    let finish = if req_cancel.is_cancelled() || runtime_cancel.is_cancelled() {
                        FinishReason::Cancelled
                    } else {
                        FinishReason::Stop
                    };
                    let _ = tx.send(Ok(terminal_token(finish)));
                }
                Err(err) => {
                    // HONEST error item — exec/boot failure is never a silent
                    // empty stream.
                    let _ = tx.send(Err(ModelRuntimeError::GenerateError(format!(
                        "sandbox exec failed: {err}"
                    ))));
                }
            }
            // `tx` drops here -> the receiver stream ends.
        });

        Box::pin(stream::unfold(StreamState { rx }, |mut st| async move {
            match st.rx.recv().await {
                Some(item) => Some((item, st)),
                None => None,
            }
        }))
    }
}

#[async_trait]
impl ModelRuntime for SandboxModelRuntime {
    fn adapter_name(&self) -> &'static str {
        SANDBOX_RUNTIME_ADAPTER
    }

    /// Validate the GGUF exists, `spawn` the ephemeral microVM handle, and bind
    /// the GGUF dir into the guest (so the per-exec initramfs bakes it). No model
    /// weights load here in ephemeral mode — they load per-exec inside the VM
    /// (documented honesty cost).
    async fn load(&mut self, _spec: LoadSpec) -> Result<ModelId, ModelRuntimeError> {
        if !self.cfg.gguf_host_path.is_file() {
            return Err(ModelRuntimeError::LoadError(format!(
                "sandbox model gguf not found: {}",
                self.cfg.gguf_host_path.display()
            )));
        }
        if !self.cfg.gguf_root_bind.is_dir() {
            return Err(ModelRuntimeError::LoadError(format!(
                "sandbox model gguf_root_bind must be an existing directory: {}",
                self.cfg.gguf_root_bind.display()
            )));
        }
        // The GGUF must live under the bound dir so the guest path
        // `<GGUF_GUEST_ROOT>/<file>` actually resolves (research risk #4).
        if self.cfg.gguf_host_path.parent() != Some(self.cfg.gguf_root_bind.as_path()) {
            // Allow nested layouts only when the gguf is under the bind root.
            if !self
                .cfg
                .gguf_host_path
                .starts_with(&self.cfg.gguf_root_bind)
            {
                return Err(ModelRuntimeError::LoadError(format!(
                    "sandbox model gguf {} must live under gguf_root_bind {}",
                    self.cfg.gguf_host_path.display(),
                    self.cfg.gguf_root_bind.display()
                )));
            }
        }

        let spec = inference_process_spec(self.model_id, &self.cfg);
        let handle = self
            .adapter
            .spawn(spec)
            .await
            .map_err(|err| ModelRuntimeError::LoadError(format!("sandbox spawn failed: {err}")))?;
        // Register the GGUF dir bind so the ephemeral per-exec initramfs bakes
        // it (the CH adapter reads binds from handle state, not ProcessSpec.binds).
        self.adapter
            .fs_bind(
                &handle,
                self.cfg.gguf_root_bind.clone(),
                PathBuf::from(GGUF_GUEST_ROOT),
                BindMode::ReadOnly,
            )
            .await
            .map_err(|err| {
                ModelRuntimeError::LoadError(format!("sandbox gguf fs_bind failed: {err}"))
            })?;

        self.handle = Some(handle);
        Ok(self.model_id)
    }

    async fn unload(&mut self, _id: ModelId) -> Result<(), ModelRuntimeError> {
        if let Some(handle) = self.handle.take() {
            // Best-effort terminate the ephemeral handle; an error here is not
            // fatal (any in-flight child is reaped by the adapter on drop).
            let _ = self.adapter.kill(&handle, Signal::Term).await;
        }
        Ok(())
    }

    fn generate(&self, req: GenerateRequest) -> TokenStream {
        self.streaming_generate(req)
    }

    async fn score(&self, _id: ModelId, _sequence: Vec<u32>) -> Result<Score, ModelRuntimeError> {
        Err(ModelRuntimeError::CapabilityNotSupported {
            capability: "score (sandbox serial exec transports text only; no per-token logprobs)"
                .to_string(),
            adapter: SANDBOX_RUNTIME_ADAPTER.to_string(),
        })
    }

    async fn embed(&self, _id: ModelId, _text: &str) -> Result<Embedding, ModelRuntimeError> {
        Err(ModelRuntimeError::CapabilityNotSupported {
            capability: "embed (sandbox serial exec has no embeddings surface)".to_string(),
            adapter: SANDBOX_RUNTIME_ADAPTER.to_string(),
        })
    }

    fn capabilities(&self, _id: ModelId) -> Result<&ModelCapabilities, ModelRuntimeError> {
        Ok(&self.declared_capabilities)
    }

    fn kv_cache(&self, _id: ModelId) -> Result<KvCacheHandle, ModelRuntimeError> {
        Err(ModelRuntimeError::CapabilityNotSupported {
            capability: "kv_cache (sandbox serial exec exposes no local KV cache)".to_string(),
            adapter: SANDBOX_RUNTIME_ADAPTER.to_string(),
        })
    }

    fn lora_stack(&self, _id: ModelId) -> Result<LoraStackHandle, ModelRuntimeError> {
        Err(ModelRuntimeError::CapabilityNotSupported {
            capability: "lora_stack (sandbox serial exec mounts no LoRAs)".to_string(),
            adapter: SANDBOX_RUNTIME_ADAPTER.to_string(),
        })
    }

    fn steering_hooks(&self, _id: ModelId) -> Result<SteeringHookHandle, ModelRuntimeError> {
        Err(ModelRuntimeError::CapabilityNotSupported {
            capability: "steering_hooks (sandbox serial exec has no residual stream to hook)"
                .to_string(),
            adapter: SANDBOX_RUNTIME_ADAPTER.to_string(),
        })
    }

    fn cancel(&self, token: CancellationToken) {
        token.cancel();
        self.runtime_cancel.cancel();
        // Tear down the in-flight microVM via the adapter so a cancel actually
        // kills the boxed boot (ephemeral children are also reaped by the
        // adapter's kill_on_drop). Best-effort, fire-and-forget on a detached
        // task because `cancel` is a sync trait method.
        if let Some(handle) = self.handle.clone() {
            let adapter = Arc::clone(&self.adapter);
            tokio::spawn(async move {
                let _ = adapter.kill(&handle, Signal::Kill).await;
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_runtime::{GenPrompt, SamplingParams};
    use crate::sandbox::{
        AdapterCapabilities, BindSpec, ExecResult, GpuPassthrough, IsolationStrength, IsolationTier,
        ProcessStatus, SandboxAdapterError, SnapshotRef, ThroughputClass,
    };
    use bytes::Bytes;
    use futures::StreamExt;
    use std::sync::Mutex;

    /// What the fake `exec` should do this run.
    #[derive(Clone)]
    enum ExecScript {
        /// Succeed, returning this completion on stdout with exit_code 0.
        Completion(String),
        /// Succeed at the boot but the runner exits non-zero (e.g. OOM).
        NonZeroExit(i32),
        /// The boot/exec itself fails (adapter error).
        ExecError,
    }

    /// Records of what the fake adapter observed, so tests assert the exact
    /// ProcessSpec/Command the runtime built.
    #[derive(Default)]
    struct FakeObservations {
        last_spec: Option<ProcessSpec>,
        last_binds: Vec<BindSpec>,
        last_command: Option<Command>,
        kill_called: bool,
        snapshot_called: bool,
        restore_called: bool,
    }

    /// A headless FAKE [`SandboxAdapter`] reporting Tier-3 + snapshot so
    /// `selection::select` picks it; scriptable exec/snapshot/restore.
    struct FakeSandboxAdapter {
        script: ExecScript,
        obs: Arc<Mutex<FakeObservations>>,
    }

    impl FakeSandboxAdapter {
        fn new(script: ExecScript) -> (Arc<Self>, Arc<Mutex<FakeObservations>>) {
            let obs = Arc::new(Mutex::new(FakeObservations::default()));
            let adapter = Arc::new(Self {
                script,
                obs: obs.clone(),
            });
            (adapter, obs)
        }
    }

    #[async_trait]
    impl SandboxAdapter for FakeSandboxAdapter {
        async fn spawn(&self, spec: ProcessSpec) -> Result<ProcessHandle, SandboxAdapterError> {
            self.obs.lock().unwrap().last_spec = Some(spec);
            Ok(ProcessHandle::new(
                AdapterId::new("cloud_hypervisor"),
                None,
                "hsk-ch-fake-0001",
            ))
        }

        async fn exec(
            &self,
            _handle: &ProcessHandle,
            cmd: Command,
        ) -> Result<ExecResult, SandboxAdapterError> {
            self.obs.lock().unwrap().last_command = Some(cmd);
            match &self.script {
                ExecScript::Completion(text) => Ok(ExecResult {
                    exit_code: 0,
                    stdout: Bytes::from(text.clone().into_bytes()),
                    stderr: Bytes::new(),
                    duration_ms: 7,
                }),
                ExecScript::NonZeroExit(code) => Ok(ExecResult {
                    exit_code: *code,
                    stdout: Bytes::new(),
                    stderr: Bytes::new(),
                    duration_ms: 3,
                }),
                ExecScript::ExecError => Err(SandboxAdapterError::SpawnFailed {
                    adapter_id: AdapterId::new("cloud_hypervisor"),
                    reason: "fake exec fault injection".to_string(),
                }),
            }
        }

        async fn fs_bind(
            &self,
            _handle: &ProcessHandle,
            host_path: PathBuf,
            guest_path: PathBuf,
            mode: BindMode,
        ) -> Result<(), SandboxAdapterError> {
            self.obs.lock().unwrap().last_binds.push(BindSpec {
                host_path,
                guest_path,
                mode,
            });
            Ok(())
        }

        async fn net_policy(
            &self,
            _handle: &ProcessHandle,
            _policy: NetPolicy,
        ) -> Result<(), SandboxAdapterError> {
            Ok(())
        }

        async fn kill(
            &self,
            _handle: &ProcessHandle,
            _signal: Signal,
        ) -> Result<(), SandboxAdapterError> {
            self.obs.lock().unwrap().kill_called = true;
            Ok(())
        }

        async fn status(
            &self,
            _handle: &ProcessHandle,
        ) -> Result<ProcessStatus, SandboxAdapterError> {
            Ok(ProcessStatus::Running)
        }

        async fn exit_code(
            &self,
            _handle: &ProcessHandle,
        ) -> Result<Option<i32>, SandboxAdapterError> {
            Ok(None)
        }

        async fn snapshot(
            &self,
            _handle: &ProcessHandle,
        ) -> Result<SnapshotRef, SandboxAdapterError> {
            self.obs.lock().unwrap().snapshot_called = true;
            Ok(SnapshotRef::new(
                AdapterId::new("cloud_hypervisor"),
                "/fake/snap/dir",
            ))
        }

        async fn restore(
            &self,
            _snapshot: &SnapshotRef,
        ) -> Result<ProcessHandle, SandboxAdapterError> {
            self.obs.lock().unwrap().restore_called = true;
            Ok(ProcessHandle::new(
                AdapterId::new("cloud_hypervisor"),
                None,
                "hsk-ch-restored-fake",
            ))
        }

        fn capabilities(&self) -> AdapterCapabilities {
            AdapterCapabilities {
                adapter_id: AdapterId::new("cloud_hypervisor"),
                runtime_available: true,
                filesystem_isolation_strength: IsolationStrength::VeryStrong,
                network_isolation_strength: IsolationStrength::VeryStrong,
                gpu_passthrough: GpuPassthrough::None,
                stdio_throughput_class: ThroughputClass::Low,
                win32_native_fidelity: false,
                cross_machine_portable: true,
                isolation_tier: IsolationTier::Tier3Microvm,
                requires_nested_virt: true,
                supports_snapshot: true,
            }
        }
    }

    /// Create a temp dir holding a fake GGUF file so `load()`'s existence gates
    /// pass headlessly. Returns (dir, gguf_path).
    fn temp_gguf() -> (PathBuf, PathBuf) {
        let dir = std::env::temp_dir().join(format!("hsk-sbx-{}", uuid::Uuid::now_v7().simple()));
        std::fs::create_dir_all(&dir).expect("mk temp gguf dir");
        let gguf = dir.join("model.gguf");
        std::fs::write(&gguf, b"GGUF-FAKE").expect("write fake gguf");
        (dir, gguf)
    }

    fn cfg_for(dir: &PathBuf, gguf: &PathBuf) -> SandboxModelConfig {
        SandboxModelConfig::new(gguf.clone(), dir.clone(), "llama-cli")
            .with_memory_bytes(4 * 1024 * 1024 * 1024)
            .with_timeout_ms(60_000)
    }

    fn gen_req(id: ModelId, prompt: &str, max_tokens: u32) -> GenerateRequest {
        GenerateRequest {
            id,
            prompt: GenPrompt::new(prompt),
            sampling: SamplingParams::default(),
            lora_overrides: vec![],
            steering_overrides: vec![],
            kv_prefix_handle: None,
            cancel: CancellationToken::new(),
            max_tokens,
            stop_sequences: vec![],
            speculative_mode: None,
            structured_decoding: None,
        }
    }

    async fn loaded(
        script: ExecScript,
    ) -> (SandboxModelRuntime, Arc<Mutex<FakeObservations>>, PathBuf) {
        let (dir, gguf) = temp_gguf();
        let (adapter, obs) = FakeSandboxAdapter::new(script);
        let mut rt = SandboxModelRuntime::new(adapter, cfg_for(&dir, &gguf));
        let spec = LoadSpec {
            artifact_path: gguf.clone(),
            sha256_expected: String::new(),
            runtime_kind: crate::model_runtime::RuntimeKind::LlamaCpp,
            sampling_defaults: SamplingParams::default(),
            kv_cache_policy: crate::model_runtime::KvCachePolicy::default(),
            declared_capabilities: ModelCapabilities::default(),
            provider: ProviderKind::Local,
            engine_origin: Some("llama_cpp".to_string()),
            external_engine_import: None,
        };
        let _ = rt.load(spec).await.expect("load mints handle + binds gguf");
        (rt, obs, dir)
    }

    use crate::model_runtime::ProviderKind;

    /// Test 1: load builds the correct ProcessSpec (GGUF bind ReadOnly under
    /// /models, DenyAll, UntrustedAgent -> Tier3 minimum) and generate builds
    /// the correct Command (runner + --model /models/<gguf> + -p prompt + -n).
    #[tokio::test]
    async fn generate_builds_correct_spec_and_command() {
        let (rt, obs, dir) = loaded(ExecScript::Completion("hello world".to_string())).await;
        let mut stream = rt.generate(gen_req(rt.model_id(), "say hi", 32));
        // Drain so the exec thread runs and records the command.
        while stream.next().await.is_some() {}

        let o = obs.lock().unwrap();
        let spec = o.last_spec.as_ref().expect("spawn recorded a spec");
        // GGUF bind is ReadOnly under /models.
        assert_eq!(spec.binds.len(), 1, "exactly one declared bind");
        assert_eq!(spec.binds[0].guest_path, PathBuf::from("/models"));
        assert_eq!(spec.binds[0].mode, BindMode::ReadOnly);
        // net policy denies all; trust class forces Tier3 minimum.
        assert_eq!(spec.net_policy, NetPolicy::DenyAll);
        assert_eq!(spec.trust_class, TrustClass::UntrustedAgent);
        assert_eq!(
            spec.trust_class.min_isolation_tier(),
            IsolationTier::Tier3Microvm
        );
        // The GGUF dir was also bound via fs_bind so the per-exec initramfs bakes it.
        assert!(
            o.last_binds
                .iter()
                .any(|b| b.guest_path == PathBuf::from("/models") && b.mode == BindMode::ReadOnly),
            "fs_bind registered the gguf dir ReadOnly under /models"
        );

        let cmd = o.last_command.as_ref().expect("exec recorded a command");
        assert_eq!(cmd.argv[0], "llama-cli");
        // --model /models/model.gguf must match the bind root (risk #4).
        let model_flag = cmd.argv.iter().position(|a| a == "--model").unwrap();
        assert_eq!(cmd.argv[model_flag + 1], "/models/model.gguf");
        let p_flag = cmd.argv.iter().position(|a| a == "-p").unwrap();
        assert_eq!(cmd.argv[p_flag + 1], "say hi");
        let n_flag = cmd.argv.iter().position(|a| a == "-n").unwrap();
        assert_eq!(cmd.argv[n_flag + 1], "32");
        assert!(cmd.argv.iter().any(|a| a == "--no-display-prompt"));
        assert!(cmd.argv.iter().any(|a| a == "--log-disable"));
        drop(o);
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Test 2: the fake completion's chunks concatenate IN ORDER and the
    /// terminal token is Stop.
    #[tokio::test]
    async fn generate_streams_completion_chunks_in_order_terminal_stop() {
        let completion = "the quick brown fox jumps over the lazy dog repeatedly and well";
        let (rt, _obs, dir) = loaded(ExecScript::Completion(completion.to_string())).await;
        let mut stream = rt.generate(gen_req(rt.model_id(), "p", 64));

        let mut texts: Vec<String> = Vec::new();
        let mut terminal: Option<FinishReason> = None;
        while let Some(item) = stream.next().await {
            let token = item.expect("no error item");
            if let Some(fr) = token.finish_reason {
                terminal = Some(fr);
            } else {
                texts.push(token.text);
            }
        }
        assert!(texts.len() >= 2, "expected multiple chunks, got {texts:?}");
        assert_eq!(texts.join(""), completion, "chunks concatenate to completion");
        assert_eq!(terminal, Some(FinishReason::Stop));
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Test 3a: a non-zero runner exit surfaces as a stream ERROR item, not an
    /// empty stream.
    #[tokio::test]
    async fn generate_nonzero_exit_surfaces_stream_error() {
        let (rt, _obs, dir) = loaded(ExecScript::NonZeroExit(137)).await;
        let mut stream = rt.generate(gen_req(rt.model_id(), "p", 8));
        let mut saw_error = false;
        let mut saw_any = false;
        while let Some(item) = stream.next().await {
            saw_any = true;
            if let Err(ModelRuntimeError::GenerateError(_)) = item {
                saw_error = true;
            }
        }
        assert!(saw_any, "stream must not be silently empty");
        assert!(saw_error, "non-zero exit must surface as a GenerateError item");
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Test 3b: an exec/boot failure surfaces as a stream ERROR item.
    #[tokio::test]
    async fn generate_exec_error_surfaces_stream_error() {
        let (rt, _obs, dir) = loaded(ExecScript::ExecError).await;
        let mut stream = rt.generate(gen_req(rt.model_id(), "p", 8));
        let mut saw_error = false;
        while let Some(item) = stream.next().await {
            if let Err(ModelRuntimeError::GenerateError(_)) = item {
                saw_error = true;
            }
        }
        assert!(saw_error, "exec failure must surface as a GenerateError item");
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Test 4: cancel() calls adapter.kill; a pre-cancelled request ends the
    /// stream with Cancelled without an error item.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn cancel_kills_vm_and_terminal_is_cancelled() {
        let (rt, obs, dir) = loaded(ExecScript::Completion("x".to_string())).await;
        let mut req = gen_req(rt.model_id(), "p", 8);
        // Pre-cancel so the exec thread takes the Cancelled terminal path
        // deterministically (no VM boot, no race).
        req.cancel.cancel();
        let mut stream = rt.generate(req.clone());
        let mut terminal: Option<FinishReason> = None;
        while let Some(item) = stream.next().await {
            if let Ok(token) = item {
                if let Some(fr) = token.finish_reason {
                    terminal = Some(fr);
                }
            }
        }
        assert_eq!(terminal, Some(FinishReason::Cancelled));

        // cancel() must drive adapter.kill on the handle.
        rt.cancel(req.cancel.clone());
        // Give the detached kill task a moment to run.
        for _ in 0..50 {
            if obs.lock().unwrap().kill_called {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        assert!(obs.lock().unwrap().kill_called, "cancel() must call adapter.kill");
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Test: generate before load surfaces an honest error item (never empty).
    #[tokio::test]
    async fn generate_before_load_surfaces_error() {
        let (dir, gguf) = temp_gguf();
        let (adapter, _obs) = FakeSandboxAdapter::new(ExecScript::Completion("x".to_string()));
        let rt = SandboxModelRuntime::new(adapter, cfg_for(&dir, &gguf));
        let mut stream = rt.generate(gen_req(rt.model_id(), "p", 8));
        let mut saw_error = false;
        while let Some(item) = stream.next().await {
            if let Err(ModelRuntimeError::GenerateError(_)) = item {
                saw_error = true;
            }
        }
        assert!(saw_error, "generate before load must surface a GenerateError");
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Test: load fails honestly when the GGUF does not exist.
    #[tokio::test]
    async fn load_missing_gguf_fails_typed() {
        let (adapter, _obs) = FakeSandboxAdapter::new(ExecScript::Completion("x".to_string()));
        let cfg = SandboxModelConfig::new(
            PathBuf::from("D:/__no_such_sandbox_model__/model.gguf"),
            PathBuf::from("D:/__no_such_sandbox_model__"),
            "llama-cli",
        );
        let mut rt = SandboxModelRuntime::new(adapter, cfg);
        let spec = LoadSpec {
            artifact_path: PathBuf::new(),
            sha256_expected: String::new(),
            runtime_kind: crate::model_runtime::RuntimeKind::LlamaCpp,
            sampling_defaults: SamplingParams::default(),
            kv_cache_policy: crate::model_runtime::KvCachePolicy::default(),
            declared_capabilities: ModelCapabilities::default(),
            provider: ProviderKind::Local,
            engine_origin: None,
            external_engine_import: None,
        };
        assert!(matches!(
            rt.load(spec).await,
            Err(ModelRuntimeError::LoadError(_))
        ));
    }

    /// Test: selection::select picks the Tier3 fake for an UntrustedAgent spec
    /// (proves the spec the runtime builds routes to a microVM adapter).
    #[test]
    fn selection_picks_tier3_for_untrusted_agent_spec() {
        use crate::sandbox::{select, SandboxAdapterRegistry};
        let (adapter, _obs) = FakeSandboxAdapter::new(ExecScript::Completion(String::new()));
        let mut registry = SandboxAdapterRegistry::new(AdapterId::new("cloud_hypervisor"));
        registry.register(adapter);
        let cfg = SandboxModelConfig::new(
            PathBuf::from("/x/model.gguf"),
            PathBuf::from("/x"),
            "llama-cli",
        );
        let spec = inference_process_spec(ModelId::new_v7(), &cfg);
        let chosen = select(&registry, &spec, None).expect("selection picks the tier3 fake");
        assert_eq!(
            chosen.capabilities().adapter_id,
            AdapterId::new("cloud_hypervisor")
        );
        assert_eq!(
            chosen.capabilities().isolation_tier,
            IsolationTier::Tier3Microvm
        );
    }

    // ---- LIVE operator-desktop test (NEVER passes headless) ----

    /// Live Cloud Hypervisor end-to-end. Gated on `HANDSHAKE_CH_LIVE=1` plus the
    /// real CH env (`HANDSHAKE_CH_BIN/_REMOTE_BIN/_KERNEL/_INITRAMFS/_BUSYBOX/
    /// _WORK_DIR`) and an operator GGUF + runner under the sandbox env (e.g.
    /// `/home/ilja_smets/handshake-sandbox/`). It builds the REAL
    /// `CloudHypervisorAdapter`, runs `generate` on a tiny prompt, and asserts a
    /// non-empty completion. This requires a KVM/WSL/Cloud-Hypervisor desktop and
    /// CANNOT run in the headless agent/CI — mirroring the `#[ignore]` rationale
    /// of the real-ConPTY tests in `terminal/pty.rs` and the env-gated real
    /// parallel-swarm test. Run with:
    ///
    /// ```text
    /// HANDSHAKE_CH_LIVE=1 \
    ///   HANDSHAKE_SBX_GGUF=/home/ilja_smets/handshake-sandbox/model.gguf \
    ///   HANDSHAKE_SBX_GGUF_ROOT=/home/ilja_smets/handshake-sandbox \
    ///   HANDSHAKE_SBX_RUNNER=llama-cli \
    ///   cargo test -p handshake_core sandbox_runtime -- --ignored
    /// ```
    #[tokio::test]
    #[ignore = "requires operator KVM/WSL/Cloud-Hypervisor desktop + GGUF/runner; never runs headless/CI"]
    async fn live_ch_generate_real_model_in_microvm() {
        if std::env::var("HANDSHAKE_CH_LIVE").ok().as_deref() != Some("1") {
            eprintln!("HANDSHAKE_CH_LIVE!=1; skipping live CH end-to-end");
            return;
        }
        use crate::sandbox::cloud_hypervisor::{CloudHypervisorAdapter, CloudHypervisorConfig};

        let gguf = std::env::var("HANDSHAKE_SBX_GGUF").expect("HANDSHAKE_SBX_GGUF");
        let gguf_root = std::env::var("HANDSHAKE_SBX_GGUF_ROOT").expect("HANDSHAKE_SBX_GGUF_ROOT");
        let runner = std::env::var("HANDSHAKE_SBX_RUNNER").unwrap_or_else(|_| "llama-cli".into());

        // CloudHypervisorConfig::default() reads the HANDSHAKE_CH_* env.
        let config = CloudHypervisorConfig::default();
        let adapter = Arc::new(
            CloudHypervisorAdapter::try_new(config)
                .await
                .expect("real CloudHypervisorAdapter available on this desktop"),
        );

        let cfg = SandboxModelConfig::new(gguf, gguf_root, runner).with_timeout_ms(120_000);
        let mut rt = SandboxModelRuntime::new(adapter, cfg);
        let spec = LoadSpec {
            artifact_path: PathBuf::new(),
            sha256_expected: String::new(),
            runtime_kind: crate::model_runtime::RuntimeKind::LlamaCpp,
            sampling_defaults: SamplingParams::default(),
            kv_cache_policy: crate::model_runtime::KvCachePolicy::default(),
            declared_capabilities: ModelCapabilities::default(),
            provider: ProviderKind::Local,
            engine_origin: Some("llama_cpp".to_string()),
            external_engine_import: None,
        };
        let id = rt.load(spec).await.expect("live load");
        let mut stream = rt.generate(gen_req(id, "Reply with one word: hello", 16));
        let mut out = String::new();
        while let Some(item) = stream.next().await {
            let token = item.expect("no error item from a live run");
            if token.finish_reason.is_none() {
                out.push_str(&token.text);
            }
        }
        assert!(
            !out.trim().is_empty(),
            "a live microVM run must produce a non-empty completion"
        );
    }
}

//! WP-KERNEL-004 (follow-up #2): Official-CLI bridge as a FIRST-CLASS swarm
//! cloud `ModelRuntime`.
//!
//! [`CliBridgeModelRuntime`] makes the operator's official CLI subscription
//! (Claude Code / Codex CLI / gemini-cli) a real swarm cloud runtime. Its
//! `generate()` streams the CLI subprocess's raw stdout LIVE as
//! [`GeneratedToken`]s, so the existing swarm capture seam
//! (`app/src-tauri/src/commands/swarm_runtime.rs`) — which fans EVERY runtime's
//! generated token stream into the bound `CaptureSink` → terminal panel +
//! Flight Recorder — automatically mirrors the CLI's stdout into the in-app
//! terminal with no new capture wiring.
//!
//! Posture (mirrors the BYOK siblings [`super::AnthropicByokRuntime`] /
//! `OpenAiByokRuntime`):
//!
//! - `load(LoadSpec{ provider: OfficialCli, engine_origin = CLI model name })`
//!   registers the operator's [`CliBridgeConfig`] through the existing
//!   [`OfficialCliBridgeRuntime::register_bridge`] (which validates
//!   exe-exists / `{prompt}`-placeholder / timeout and mints a runtime-keyed
//!   `ModelId` v7) — the same tested code path the CLI bridge already uses.
//! - `generate()` bridges the SYNC, blocking
//!   [`CliSubprocessSpawner::spawn_streaming_cancellable`] to the async
//!   [`TokenStream`] via a dedicated OS thread + a tokio mpsc channel +
//!   `stream::unfold` (the same channel→unfold shape the BYOK adapters use,
//!   but fed by an OS thread because `generate` is a sync trait method and the
//!   spawner's poll loop must not occupy a tokio worker).
//! - Spawn/exit failures surface as an HONEST stream error item
//!   ([`ModelRuntimeError::GenerateError`]) — never a silently empty stream.
//! - Cancellation (request or runtime [`CancellationToken`]) kills the child
//!   via the spawner's cancel-kill path and ends the stream with
//!   [`FinishReason::Cancelled`].
//! - score / embed / kv_cache / lora_stack / steering_hooks all return
//!   [`ModelRuntimeError::CapabilityNotSupported`]: none of the inference
//!   techniques work through a CLI subprocess (the bridge is a
//!   usability-not-feature lane; capabilities are all-false).

use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use futures::stream;
use uuid::Uuid;

use super::agent_activity::parse_line as parse_agent_line;
use super::official_cli_bridge::{
    CliBridgeConfig, CliCancellationContext, CliInvocationContext, CliKind, CliOutputFormat,
    CliSubprocessSpawner, OfficialCliBridgeRuntime,
};
use super::CloudLaneObservability;
use crate::flight_recorder::events_agent_activity::agent_activity_event;
use crate::flight_recorder::events_llm_infer::{
    infer_end_event, infer_start_event, infer_token_event, new_llm_infer_request_id,
    should_emit_token_event,
};
use crate::flight_recorder::{FlightRecorder, FlightRecorderEvent};
use crate::model_runtime::{
    error::ModelRuntimeError, CancellationToken, Embedding, FinishReason, GenerateRequest,
    GeneratedToken, KvCacheHandle, LoadSpec, LoraStackHandle, ModelCapabilities, ModelId,
    ModelRuntime, ProviderKind, Score, SteeringHookHandle, TokenStream,
};
use crate::terminal::redaction::{PatternRedactor, SecretRedactor};

/// Hard cap on a single un-terminated stdout line buffered for structured
/// parsing. A pathological CLI that never emits `\n` would otherwise grow the
/// buffer unbounded; at the cap the accumulated bytes are flushed as ONE line
/// (which the defensive parser turns into an `Other` activity — never dropped,
/// never OOM). 1 MiB comfortably exceeds any real JSONL event line.
const AGENT_ACTIVITY_MAX_LINE_BYTES: usize = 1024 * 1024;

/// Reserved composition metadata used by the durable Tauri CLI-bridge store to
/// carry its operator-approved model allowlist through the legacy
/// `CliBridgeConfig` boundary. `CliBridgeModelRuntime::new` consumes and removes
/// this entry before the config reaches executable registration or child env
/// projection, so it is never exposed to the spawned CLI process.
pub const CLI_BRIDGE_MODEL_ALLOWLIST_METADATA_ENV: &str =
    "HANDSHAKE_INTERNAL_CLI_MODEL_ALLOWLIST_V1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CliModelAllowlist {
    allowed: BTreeSet<String>,
}

impl CliModelAllowlist {
    pub fn new(models: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let allowed = models
            .into_iter()
            .map(|model| model.trim().to_string())
            .filter(|model| !model.is_empty())
            .collect::<BTreeSet<_>>();
        if allowed.is_empty() {
            Err("official CLI model allowlist must not be empty".to_string())
        } else {
            Ok(Self { allowed })
        }
    }

    pub fn contains(&self, model_name: &str) -> bool {
        self.allowed.contains(model_name)
    }

    fn validate(&self, model_name: &str) -> Result<(), ModelRuntimeError> {
        if self.contains(model_name) {
            Ok(())
        } else {
            Err(ModelRuntimeError::LoadError(format!(
                "official CLI model '{model_name}' is not in the operator allowlist [{}]",
                self.allowed.iter().cloned().collect::<Vec<_>>().join(", ")
            )))
        }
    }
}

#[derive(Clone, Debug)]
pub struct AllowlistedCliBridgeConfig {
    config: CliBridgeConfig,
    model_allowlist: CliModelAllowlist,
}

impl AllowlistedCliBridgeConfig {
    pub fn new(mut config: CliBridgeConfig, model_allowlist: CliModelAllowlist) -> Self {
        config
            .env_vars
            .remove(CLI_BRIDGE_MODEL_ALLOWLIST_METADATA_ENV);
        Self {
            config,
            model_allowlist,
        }
    }

    pub fn from_config_metadata(mut config: CliBridgeConfig) -> Result<Self, String> {
        let encoded = config
            .env_vars
            .remove(CLI_BRIDGE_MODEL_ALLOWLIST_METADATA_ENV)
            .ok_or_else(|| {
                "official CLI model allowlist metadata is required; refusing unrestricted runtime construction"
                    .to_string()
            })?;
        let decoded = serde_json::from_str::<Vec<String>>(&encoded).map_err(|error| {
            format!("stored official CLI model allowlist metadata is invalid: {error}")
        })?;
        let model_allowlist = CliModelAllowlist::new(decoded)
            .map_err(|_| "stored official CLI model allowlist is empty".to_string())?;
        Ok(Self::new(config, model_allowlist))
    }

    pub fn into_parts(self) -> (CliBridgeConfig, CliModelAllowlist) {
        (self.config, self.model_allowlist)
    }

    /// Borrow the exact launch configuration that was paired with the model
    /// allowlist. Auxiliary provider-owned probes use this accessor so they
    /// cannot rediscover and execute an independent PATH candidate.
    pub fn launch_config(&self) -> &CliBridgeConfig {
        &self.config
    }
}

/// Accumulates decoded stdout text and splits it into complete lines on `\n`,
/// so the structured agent-activity parser sees one JSON event per line even
/// though the raw pipe arrives in arbitrary byte chunks. A trailing partial line
/// is retained until the next chunk completes it (or [`Self::flush`] at stream
/// end). Only constructed in JSON-stream output modes — in `RawText` mode the
/// buffer is never built and there is zero behaviour change.
struct AgentActivityLineBuffer {
    pending: String,
}

impl AgentActivityLineBuffer {
    fn new() -> Self {
        Self {
            pending: String::new(),
        }
    }

    /// Append decoded `text` and return every COMPLETE line (without the `\n`).
    /// A partial trailing line is retained. If the retained buffer exceeds the
    /// max-line cap, it is force-flushed as one line so memory stays bounded.
    fn push(&mut self, text: &str) -> Vec<String> {
        self.pending.push_str(text);
        let mut lines = Vec::new();
        while let Some(idx) = self.pending.find('\n') {
            let mut line: String = self.pending.drain(..=idx).collect();
            // Strip the trailing '\n' (and a preceding '\r' for CRLF streams).
            if line.ends_with('\n') {
                line.pop();
            }
            if line.ends_with('\r') {
                line.pop();
            }
            lines.push(line);
        }
        if self.pending.len() > AGENT_ACTIVITY_MAX_LINE_BYTES {
            let forced = std::mem::take(&mut self.pending);
            lines.push(forced);
        }
        lines
    }

    /// Flush any retained partial final line at end-of-stream (a JSONL stream may
    /// not end with `\n`). Returns `None` when nothing is pending.
    fn flush(&mut self) -> Option<String> {
        if self.pending.is_empty() {
            None
        } else {
            Some(std::mem::take(&mut self.pending))
        }
    }
}

/// Best-effort emit of an `FR-EVT-LLM-INFER-*` event through the cloud-lane
/// recorder. Mirrors the BYOK siblings' posture (`anthropic_byok.rs`): a
/// recorder error is logged and ignored — observability NEVER affects
/// generation. A `None` recorder is a no-op (the lane was built without
/// observability).
async fn emit_infer(recorder: &Option<Arc<dyn FlightRecorder>>, event: FlightRecorderEvent) {
    if let Some(recorder) = recorder.as_ref() {
        if let Err(err) = recorder.record_event(event).await {
            tracing::debug!(
                target: "handshake_core::model_runtime::cloud::cli_bridge",
                error = %err,
                "official CLI bridge FR-EVT-LLM-INFER emit failed; generation unaffected"
            );
        }
    }
}

type InferEmitMessage = (
    FlightRecorderEvent,
    Option<tokio::sync::oneshot::Sender<()>>,
);

struct InferEventEmitter {
    tx: Option<tokio::sync::mpsc::UnboundedSender<InferEmitMessage>>,
}

impl InferEventEmitter {
    fn new(recorder: Option<Arc<dyn FlightRecorder>>) -> Self {
        let Some(recorder) = recorder else {
            return Self { tx: None };
        };
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<InferEmitMessage>();
        let worker = async move {
            while let Some((event, acknowledgement)) = rx.recv().await {
                emit_infer(&Some(recorder.clone()), event).await;
                if let Some(acknowledgement) = acknowledgement {
                    let _ = acknowledgement.send(());
                }
            }
        };
        match tokio::runtime::Handle::try_current() {
            Ok(runtime) => {
                runtime.spawn(worker);
            }
            Err(_) => {
                std::thread::spawn(move || futures::executor::block_on(worker));
            }
        }
        Self { tx: Some(tx) }
    }

    fn emit(&self, event: FlightRecorderEvent) -> Option<tokio::sync::oneshot::Receiver<()>> {
        let tx = self.tx.as_ref()?;
        let (acknowledgement, received) = tokio::sync::oneshot::channel();
        tx.send((event, Some(acknowledgement))).ok()?;
        Some(received)
    }

    async fn emit_and_wait(&self, event: FlightRecorderEvent) {
        if let Some(received) = self.emit(event) {
            let _ = received.await;
        }
    }
}

/// Best-effort emit of an `FR-EVT-AGENT-*` event. Same posture as
/// [`emit_infer`]: a `None` recorder is a no-op, a recorder error is logged and
/// ignored — structured agent-activity capture NEVER affects generation.
async fn emit_agent(recorder: &Option<Arc<dyn FlightRecorder>>, event: FlightRecorderEvent) {
    if let Some(recorder) = recorder.as_ref() {
        if let Err(err) = recorder.record_event(event).await {
            tracing::debug!(
                target: "handshake_core::model_runtime::cloud::cli_bridge",
                error = %err,
                "official CLI bridge FR-EVT-AGENT emit failed; generation unaffected"
            );
        }
    }
}

/// Whether an output format is a JSON-stream mode the structured parser should
/// run on. `RawText` => no structured capture (unchanged behaviour).
fn is_json_stream_mode(format: CliOutputFormat) -> bool {
    matches!(format, CliOutputFormat::Json | CliOutputFormat::JsonStream)
}

/// Parse a complete stdout line and emit one `FR-EVT-AGENT-*` event per parsed
/// activity, advancing `ordered_index`. Best-effort; the parser never panics and
/// never drops a line (a malformed line yields an `Other` activity).
#[allow(clippy::too_many_arguments)]
async fn emit_agent_line(
    recorder: &Option<Arc<dyn FlightRecorder>>,
    cli_kind: CliKind,
    model_id: ModelId,
    request_id: Uuid,
    instance_id: Option<&str>,
    redactor: &dyn SecretRedactor,
    ordered_index: &mut u64,
    line: &str,
) {
    if recorder.is_none() {
        return;
    }
    for activity in parse_agent_line(cli_kind, line) {
        let event = agent_activity_event(
            model_id,
            request_id,
            *ordered_index,
            instance_id,
            CLI_BRIDGE_ADAPTER,
            &activity,
            redactor,
        );
        *ordered_index = ordered_index.saturating_add(1);
        emit_agent(recorder, event).await;
    }
}

/// Async stream state carried through `stream::unfold` so the CLI bridge can
/// emit the lane-normalised `FR-EVT-LLM-INFER-{START,TOKEN,END}` events on the
/// ASYNC side of the channel (where `record_event().await` is legal) while the
/// blocking spawn thread only ever produces tokens. START is emitted lazily on
/// the first poll, TOKEN is emitted on the sampled indices
/// (`should_emit_token_event`), and END is emitted exactly once when the
/// producer channel closes — so START/END are always paired even for an empty
/// or immediately-failing generation.
struct InferEmitStreamState {
    rx: tokio::sync::mpsc::UnboundedReceiver<Result<GeneratedToken, ModelRuntimeError>>,
    recorder: Option<Arc<dyn FlightRecorder>>,
    infer_emitter: InferEventEmitter,
    model_id: ModelId,
    request_id: Uuid,
    start: Instant,
    prompt_tokens: u64,
    started: bool,
    generated: u32,
    finish: FinishReason,
    ended: bool,
    /// Per-generation cancellation owned by the returned stream. Dropping an
    /// unpolled or silent stream cancels the blocking CLI invocation instead of
    /// leaving it alive until its configured timeout.
    stream_cancel: CancellationToken,
    /// Structured agent-activity capture (JSON-stream modes only). When
    /// `agent_capture` is `Some`, each streamed token's text is fed through the
    /// line buffer and complete lines are parsed into `FR-EVT-AGENT-*` events.
    /// `None` in `RawText` mode => zero structured behaviour, unchanged output.
    agent_capture: Option<AgentCaptureState>,
}

impl Drop for InferEmitStreamState {
    fn drop(&mut self) {
        if !self.ended {
            self.stream_cancel.cancel();
            let _ = self.claim_terminal_emit(FinishReason::Cancelled);
        }
    }
}

impl InferEmitStreamState {
    fn claim_terminal_emit(
        &mut self,
        finish: FinishReason,
    ) -> Option<tokio::sync::oneshot::Receiver<()>> {
        if !self.started || self.ended {
            return None;
        }
        self.ended = true;
        let total = self.start.elapsed().as_millis() as u64;
        self.infer_emitter.emit(infer_end_event(
            self.model_id,
            self.request_id,
            self.prompt_tokens,
            self.generated,
            total,
            0,
            total,
            finish,
            CLI_BRIDGE_ADAPTER,
        ))
    }
}

/// Per-request structured agent-activity capture state, carried on the async
/// side of the stream so parsing/emit happens where `record_event().await` is
/// legal. Only present in JSON-stream output modes.
struct AgentCaptureState {
    cli_kind: CliKind,
    instance_id: Option<String>,
    redactor: Arc<dyn SecretRedactor>,
    line_buf: AgentActivityLineBuffer,
    ordered_index: u64,
}

/// Adapter name surfaced through [`ModelRuntime::adapter_name`] and used as the
/// `adapter` field on `CapabilityNotSupported` errors. Matches the CLI bridge
/// label used elsewhere (process ledger, sandbox adapter id).
const CLI_BRIDGE_ADAPTER: &str = "official_cli_bridge";

/// Incremental UTF-8 decoder for raw stdout chunks read off the CLI subprocess
/// pipe. The pipe is read in fixed-size byte blocks, so a multibyte UTF-8
/// codepoint can split across two chunks. Per-chunk `from_utf8_lossy` would
/// corrupt such a boundary (emit replacement chars mid-codepoint), so the
/// decoder carries the trailing incomplete-sequence bytes forward to the next
/// `push` and decodes them once the rest arrives. Each returned `String` is
/// guaranteed valid UTF-8 (the contract for `GeneratedToken::text`).
struct Utf8ChunkDecoder {
    pending: Vec<u8>,
}

impl Utf8ChunkDecoder {
    fn new() -> Self {
        Self {
            pending: Vec::new(),
        }
    }

    /// Append `bytes`, decode the longest valid UTF-8 prefix, and return it.
    /// Any trailing bytes that form an INCOMPLETE (but potentially valid)
    /// multibyte sequence are retained for the next call; genuinely invalid
    /// bytes are emitted lossily (replacement char) and consumed so the decoder
    /// makes forward progress.
    fn push(&mut self, bytes: &[u8]) -> String {
        self.pending.extend_from_slice(bytes);
        match std::str::from_utf8(&self.pending) {
            Ok(s) => {
                let out = s.to_string();
                self.pending.clear();
                out
            }
            Err(e) => {
                let valid = e.valid_up_to();
                // SAFETY-equivalent: `valid_up_to` is a guaranteed char boundary.
                let good = String::from_utf8_lossy(&self.pending[..valid]).into_owned();
                match e.error_len() {
                    // `None` => the bytes after `valid` are an INCOMPLETE trailing
                    // sequence that may complete on the next chunk: carry them.
                    None => {
                        let remainder = self.pending.split_off(valid);
                        self.pending = remainder;
                        good
                    }
                    // `Some(n)` => genuinely invalid bytes at `valid`: emit the
                    // valid prefix plus a lossy decode of the invalid run, then
                    // keep only what follows so the decoder never stalls.
                    Some(n) => {
                        let bad_end = valid + n;
                        let bad =
                            String::from_utf8_lossy(&self.pending[valid..bad_end]).into_owned();
                        let remainder = self.pending.split_off(bad_end);
                        self.pending = remainder;
                        format!("{good}{bad}")
                    }
                }
            }
        }
    }

    /// Flush any retained trailing bytes at end-of-stream. A genuinely truncated
    /// tail is decoded lossily (non-fatal). Returns an empty string when nothing
    /// is pending.
    fn finish(&mut self) -> String {
        if self.pending.is_empty() {
            return String::new();
        }
        let s = String::from_utf8_lossy(&self.pending).into_owned();
        self.pending.clear();
        s
    }
}

/// First-class swarm cloud `ModelRuntime` backed by the official CLI bridge.
/// Reuses [`OfficialCliBridgeRuntime`] for registration + validation +
/// handle lookup, and a shared [`CliSubprocessSpawner`] as the live byte source.
pub struct CliBridgeModelRuntime {
    /// Reused CLI bridge runtime: owns `register_bridge` / `handle_for` /
    /// `unregister` and the model-id → config map.
    inner: OfficialCliBridgeRuntime,
    /// The live byte source. Shared (`Arc`) so the OS thread in `generate()`
    /// can own a clone for the blocking spawn.
    spawner: Arc<dyn CliSubprocessSpawner>,
    /// The operator CLI config registered on every `load()` (exe path, args
    /// template, env, timeout). One runtime serves one configured CLI.
    config_template: CliBridgeConfig,
    model_allowlist: CliModelAllowlist,
    /// Governance and correlation captured at the production spawn boundary.
    /// Generation clones this template and adds only request-scoped identifiers;
    /// it never invents an owner, role, work packet, or parent session.
    invocation_context: Option<CliInvocationContext>,
    /// Runtime-wide cancellation flag, set by `cancel()`; mirrors the BYOK
    /// `runtime_cancel`. Polled by `generate()`'s cancel hook so a runtime-level
    /// cancel kills in-flight CLI subprocesses.
    runtime_cancel: CancellationToken,
    /// Declared capabilities: all-false (the CLI bridge is a usability lane).
    declared_capabilities: ModelCapabilities,
    /// Optional cloud-lane observability. When present, `generate()` emits
    /// `FR-EVT-LLM-INFER-{START,TOKEN,END}` through the FlightRecorder for
    /// HBR-INT-005 lane normalisation, mirroring the BYOK siblings
    /// ([`super::AnthropicByokRuntime::with_lane_observability`]). Emission is
    /// best-effort and never affects generation.
    lane_obs: Option<Arc<CloudLaneObservability>>,
    /// Optional swarm composite `instance_id` (`<model_id>#<instance>`) used as
    /// the session correlation tag stamped onto the `FR-EVT-AGENT-*` events
    /// (`session_span_id` + `payload.instance_id`) so the session-transcript raw
    /// seam scopes them. On the production swarm path the factory threads this
    /// (see `CliBridgeCloudRuntimeBuilder::build_loaded`). `None` => agent events
    /// still emit (model_id only) and are durably recorded, but are NOT
    /// transcript-retrievable (there is no bare-model_id seam in the transcript
    /// fetch). Never dropped, but invisible to the per-session timeline. Set via
    /// [`Self::with_session_correlation`].
    session_instance_id: Option<String>,
    /// Redactor applied to structured agent-activity surfaces (tool input,
    /// thinking/text/raw bodies) at emit time, so the structured surface is no
    /// leakier than the raw terminal capture. Defaults to [`PatternRedactor`]
    /// (the same default the capture lane uses).
    redactor: Arc<dyn SecretRedactor>,
    /// Whether JSON-stream output should also be converted into structured
    /// `FR-EVT-AGENT-*` events by this runtime. Defaults to true for direct
    /// CLI-lane usage. Operator Chat disables this because it already replays
    /// the same stdout into canonical lane messages and emits its own
    /// `FR-EVT-AGENT-*` evidence there; infer telemetry remains separately
    /// controlled by `lane_obs`.
    emit_agent_activity_events: bool,
}

impl std::fmt::Debug for CliBridgeModelRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CliBridgeModelRuntime")
            .field("inner", &self.inner)
            .field("spawner", &"<Arc<dyn CliSubprocessSpawner>>")
            .field("config_template", &self.config_template)
            .finish()
    }
}

impl CliBridgeModelRuntime {
    /// Construct a CLI-bridge swarm runtime from a live byte-source spawner
    /// (`LiveCliSpawner` in production, a mock in tests) and the operator CLI
    /// config that every `load()` registers.
    pub fn new(spawner: Arc<dyn CliSubprocessSpawner>, config: AllowlistedCliBridgeConfig) -> Self {
        let (config_template, model_allowlist) = config.into_parts();
        Self {
            inner: OfficialCliBridgeRuntime::new(spawner.clone()),
            spawner,
            config_template,
            model_allowlist,
            invocation_context: None,
            runtime_cancel: CancellationToken::new(),
            declared_capabilities: OfficialCliBridgeRuntime::cli_bridge_capabilities(),
            lane_obs: None,
            session_instance_id: None,
            redactor: Arc::new(PatternRedactor),
            emit_agent_activity_events: true,
        }
    }

    /// Attach the authoritative invocation context composed from the swarm
    /// [`SpawnRequest`](crate::swarm_orchestration::SpawnRequest). A runtime
    /// without this context fails generation closed before spawning a child.
    pub fn with_invocation_context(mut self, context: CliInvocationContext) -> Self {
        self.session_instance_id = context.session_id.clone();
        self.invocation_context = Some(context);
        self
    }

    /// Thread a [`CloudLaneObservability`] bundle so `generate()` emits
    /// `FR-EVT-LLM-INFER-{START,TOKEN,END}` through the FlightRecorder
    /// (HBR-INT-005 lane normalisation), mirroring
    /// [`super::AnthropicByokRuntime::with_lane_observability`]. Best-effort: a
    /// recorder error is logged and ignored; generation is unaffected.
    pub fn with_lane_observability(mut self, lane_obs: Arc<CloudLaneObservability>) -> Self {
        self.lane_obs = Some(lane_obs);
        self
    }

    /// Thread the swarm composite `instance_id` (`<model_id>#<instance>`) used as
    /// the session correlation tag on the structured `FR-EVT-AGENT-*` events, so
    /// the session-transcript raw seam scopes agent-activity rows to the session.
    /// Set at lane-bind / spawn-capture time where the composite is known (the
    /// production factory threads it in `build_loaded`). When absent, agent
    /// events still emit (carrying `model_id`) and are durably recorded, but they
    /// are NOT transcript-retrievable — the transcript fetch has no bare-model_id
    /// seam, only the composite session-id seams. Never dropped, but invisible to
    /// the per-session timeline until correlation is threaded.
    pub fn with_session_correlation(mut self, instance_id: impl Into<String>) -> Self {
        let instance_id = instance_id.into();
        self.session_instance_id = Some(instance_id.clone());
        if let Some(context) = self.invocation_context.as_mut() {
            context.session_id = Some(instance_id);
        }
        self
    }

    /// Override the structured agent-activity redactor (defaults to
    /// [`PatternRedactor`], the same default the terminal capture lane uses).
    pub fn with_redactor(mut self, redactor: Arc<dyn SecretRedactor>) -> Self {
        self.redactor = redactor;
        self
    }

    /// Suppress runtime-level structured `FR-EVT-AGENT-*` emission while
    /// leaving raw token streaming and `FR-EVT-LLM-INFER-*` observability
    /// unchanged. Used by replay-captured lanes such as Operator Chat to avoid
    /// duplicate agent evidence for the same official-CLI JSONL stream.
    pub fn without_agent_activity_events(mut self) -> Self {
        self.emit_agent_activity_events = false;
        self
    }

    /// The CLI model name (allowlisted by the operator's config + spec) is
    /// carried on `LoadSpec::engine_origin`, mirroring how the BYOK adapters
    /// carry the cloud model name. Looks up the registered handle to resolve
    /// the per-request model name for the spawn.
    fn streaming_generate(&self, req: GenerateRequest) -> TokenStream {
        // Resolve the registered handle first; a missing model fails as a single
        // error item rather than an empty stream (mirror BYOK `messages_stream`).
        let handle = match self.inner.handle_for(req.id) {
            Ok(handle) => handle,
            Err(err) => {
                return single_error_stream(ModelRuntimeError::GenerateError(format!(
                    "official CLI bridge generate: {err}"
                )));
            }
        };

        let (tx, rx) =
            tokio::sync::mpsc::unbounded_channel::<Result<GeneratedToken, ModelRuntimeError>>();

        // Observability is read on the ASYNC side (the unfold below) where
        // `record_event().await` is legal; the blocking spawn thread only ever
        // produces tokens. A coarse prompt-token proxy (whitespace word count)
        // is used because the CLI does not expose tokenisation — it is for the
        // observability payload only, never billing/control.
        let recorder = self.lane_obs.as_ref().map(|o| o.flight_recorder.clone());
        let request_id = new_llm_infer_request_id();
        let prompt_tokens = req.prompt.text.split_whitespace().count() as u64;
        let infer_start = Instant::now();
        let infer_model_id = req.id;

        let spawner = Arc::clone(&self.spawner);
        let config = self.config_template.clone();
        let model_name = handle.model_name.clone();
        let prompt = req.prompt.text.clone();
        let req_cancel = req.cancel.clone();
        let runtime_cancel = self.runtime_cancel.clone();
        let stream_cancel = CancellationToken::new();
        let worker_stream_cancel = stream_cancel.clone();
        let request_correlation = request_id.to_string();
        let mut invocation = match self.invocation_context.clone() {
            Some(context) => context,
            None => {
                return single_error_stream(ModelRuntimeError::GenerateError(
                    "official CLI bridge generate: missing authoritative invocation context"
                        .to_string(),
                ));
            }
        };
        invocation.registered_model_id = Some(req.id);
        invocation.model_identity = model_name.clone();
        let session_id = invocation.session_id.clone();
        if invocation.trace_id.is_none() {
            invocation.trace_id = Some(
                invocation
                    .parent_session_id
                    .clone()
                    .or_else(|| session_id.clone())
                    .unwrap_or_else(|| request_correlation.clone()),
            );
        }
        invocation.span_id = Some(request_correlation.clone());
        invocation.cancellation_id = Some(format!("cli-cancel-{request_correlation}"));
        invocation.reclaim_key = Some(format!("cli-reclaim-{request_correlation}"));

        // The blocking, poll-based spawn must NOT run on a tokio worker thread.
        // A separate owned decoder worker consumes a bounded chunk queue; the
        // subprocess timeout/cancel loop never executes decoding or callbacks.
        std::thread::spawn(move || {
            let (chunk_tx, mut chunk_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(32);
            let token_tx = tx.clone();
            let decoder_worker = std::thread::spawn(move || {
                let mut token_index: u32 = 0;
                let mut decoder = Utf8ChunkDecoder::new();
                let mut send_failed = false;
                while let Some(bytes) = chunk_rx.blocking_recv() {
                    let text = decoder.push(&bytes);
                    if !text.is_empty() {
                        token_index = token_index.saturating_add(1);
                        if token_tx
                            .send(Ok(GeneratedToken {
                                token_id: token_index,
                                text,
                                logprob: None,
                                finish_reason: None,
                            }))
                            .is_err()
                        {
                            send_failed = true;
                            break;
                        }
                    }
                }
                let tail = decoder.finish();
                if !send_failed && !tail.is_empty() {
                    token_index = token_index.saturating_add(1);
                    send_failed = token_tx
                        .send(Ok(GeneratedToken {
                            token_id: token_index,
                            text: tail,
                            logprob: None,
                            finish_reason: None,
                        }))
                        .is_err();
                }
                send_failed
            });

            let cancellation =
                CliCancellationContext::new(vec![req_cancel, runtime_cancel, worker_stream_cancel]);
            let result = spawner.spawn_streaming_cancellable(
                &config,
                &invocation,
                &model_name,
                &prompt,
                &chunk_tx,
                &cancellation,
            );
            drop(chunk_tx);
            let send_failed = decoder_worker.join().unwrap_or(true);

            if !send_failed {
                match result {
                    Ok(receipt) => {
                        let finish = if receipt.cancelled {
                            FinishReason::Cancelled
                        } else {
                            FinishReason::Stop
                        };
                        let _ = tx.send(Ok(terminal_token(finish)));
                    }
                    Err(err) => {
                        let _ = tx.send(Err(ModelRuntimeError::GenerateError(format!(
                            "official CLI bridge generate failed: {err}"
                        ))));
                    }
                }
            }
            // `tx` drops here -> the receiver stream ends.
        });

        // Structured agent-activity capture is built ONLY in a JSON-stream output
        // mode. In RawText mode the line buffer is never constructed and there is
        // zero behaviour change (the raw GeneratedToken text streams as before).
        let agent_capture = if self.emit_agent_activity_events
            && is_json_stream_mode(self.config_template.output_format)
        {
            Some(AgentCaptureState {
                cli_kind: self.config_template.cli_kind,
                instance_id: self.session_instance_id.clone(),
                redactor: self.redactor.clone(),
                line_buf: AgentActivityLineBuffer::new(),
                ordered_index: 0,
            })
        } else {
            None
        };

        let state = InferEmitStreamState {
            rx,
            infer_emitter: InferEventEmitter::new(recorder.clone()),
            recorder,
            model_id: infer_model_id,
            request_id,
            start: infer_start,
            prompt_tokens,
            started: false,
            generated: 0,
            finish: FinishReason::Stop,
            ended: false,
            stream_cancel,
            agent_capture,
        };

        Box::pin(stream::unfold(state, |mut st| async move {
            // START: emitted lazily on the first poll, before the first token,
            // so START/END are paired even for an empty/immediately-failing run.
            if !st.started {
                st.started = true;
                st.infer_emitter
                    .emit_and_wait(infer_start_event(
                        st.model_id,
                        st.request_id,
                        st.prompt_tokens,
                        "",
                        CLI_BRIDGE_ADAPTER,
                    ))
                    .await;
            }
            match st.rx.recv().await {
                Some(item) => {
                    match &item {
                        Ok(token) => {
                            if let Some(reason) = token.finish_reason {
                                // The terminal token carries the real outcome.
                                st.finish = reason;
                            } else if !token.text.is_empty() {
                                st.generated = st.generated.saturating_add(1);
                                // TOKEN: sampled (matches the BYOK siblings) so a
                                // long generation does not flood the recorder.
                                if should_emit_token_event(st.generated) {
                                    let latency = st.start.elapsed().as_millis() as u64;
                                    st.infer_emitter
                                        .emit_and_wait(infer_token_event(
                                            st.model_id,
                                            st.request_id,
                                            st.generated,
                                            token.token_id,
                                            &token.text,
                                            latency,
                                            CLI_BRIDGE_ADAPTER,
                                        ))
                                        .await;
                                }
                                // STRUCTURED: in a JSON-stream mode, feed the raw
                                // token text through the line buffer and emit an
                                // `FR-EVT-AGENT-*` event per parsed activity on
                                // each COMPLETE line. Best-effort; never affects
                                // the raw token (which has already streamed). The
                                // raw `item` is returned unchanged below.
                                let InferEmitStreamState {
                                    recorder,
                                    model_id,
                                    request_id,
                                    agent_capture,
                                    ..
                                } = &mut st;
                                if let Some(cap) = agent_capture.as_mut() {
                                    let lines = cap.line_buf.push(&token.text);
                                    for line in lines {
                                        emit_agent_line(
                                            recorder,
                                            cap.cli_kind,
                                            *model_id,
                                            *request_id,
                                            cap.instance_id.as_deref(),
                                            cap.redactor.as_ref(),
                                            &mut cap.ordered_index,
                                            &line,
                                        )
                                        .await;
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            // An honest error item ends the run as Error.
                            st.finish = FinishReason::Error;
                        }
                    }
                    Some((item, st))
                }
                None => {
                    // Producer channel closed -> flush the final partial agent
                    // line (a JSONL stream may not end with '\n'), then emit END
                    // exactly once.
                    {
                        let InferEmitStreamState {
                            recorder,
                            model_id,
                            request_id,
                            agent_capture,
                            ..
                        } = &mut st;
                        if let Some(cap) = agent_capture.as_mut() {
                            if let Some(line) = cap.line_buf.flush() {
                                emit_agent_line(
                                    recorder,
                                    cap.cli_kind,
                                    *model_id,
                                    *request_id,
                                    cap.instance_id.as_deref(),
                                    cap.redactor.as_ref(),
                                    &mut cap.ordered_index,
                                    &line,
                                )
                                .await;
                            }
                        }
                    }
                    let finish = st.finish;
                    if let Some(received) = st.claim_terminal_emit(finish) {
                        let _ = received.await;
                    }
                    None
                }
            }
        }))
    }
}

/// One-shot stream that yields a single error item (preflight failure surfaced
/// inside the [`TokenStream`] contract). Mirrors the BYOK sibling helper.
fn single_error_stream(err: ModelRuntimeError) -> TokenStream {
    Box::pin(stream::iter([Err(err)]))
}

/// Terminal token carrying only a finish reason (mirrors the BYOK sibling).
fn terminal_token(reason: FinishReason) -> GeneratedToken {
    GeneratedToken {
        token_id: 0,
        text: String::new(),
        logprob: None,
        finish_reason: Some(reason),
    }
}

#[async_trait]
impl ModelRuntime for CliBridgeModelRuntime {
    fn adapter_name(&self) -> &'static str {
        CLI_BRIDGE_ADAPTER
    }

    async fn load(&mut self, spec: LoadSpec) -> Result<ModelId, ModelRuntimeError> {
        if spec.provider != ProviderKind::OfficialCli {
            return Err(ModelRuntimeError::LoadError(format!(
                "CliBridgeModelRuntime requires provider=OfficialCli (got {:?})",
                spec.provider
            )));
        }
        let model_name = spec.engine_origin.as_deref().ok_or_else(|| {
            ModelRuntimeError::LoadError(
                "CliBridgeModelRuntime.load requires LoadSpec::engine_origin = the allowlisted \
                 CLI model name"
                    .to_string(),
            )
        })?;
        self.model_allowlist.validate(model_name)?;
        let now_utc = chrono::Utc::now().to_rfc3339();
        // register_bridge validates exe-exists / {prompt}-placeholder / timeout
        // and mints the runtime-keyed ModelId v7 — the existing tested path.
        let handle = self
            .inner
            .register_bridge(self.config_template.clone(), model_name, &now_utc)
            .map_err(|err| ModelRuntimeError::LoadError(format!("{err}")))?;
        Ok(handle.model_id)
    }

    async fn unload(&mut self, id: ModelId) -> Result<(), ModelRuntimeError> {
        self.inner
            .unregister(id)
            .map(|_| ())
            .map_err(|err| ModelRuntimeError::UnloadError(format!("{err}")))
    }

    fn generate(&self, req: GenerateRequest) -> TokenStream {
        self.streaming_generate(req)
    }

    async fn score(&self, _id: ModelId, _sequence: Vec<u32>) -> Result<Score, ModelRuntimeError> {
        Err(ModelRuntimeError::CapabilityNotSupported {
            capability: "score (CLI bridge transports text only; no per-token logprobs)"
                .to_string(),
            adapter: CLI_BRIDGE_ADAPTER.to_string(),
        })
    }

    async fn embed(&self, _id: ModelId, _text: &str) -> Result<Embedding, ModelRuntimeError> {
        Err(ModelRuntimeError::CapabilityNotSupported {
            capability: "embed (CLI bridge has no embeddings surface)".to_string(),
            adapter: CLI_BRIDGE_ADAPTER.to_string(),
        })
    }

    fn capabilities(&self, id: ModelId) -> Result<&ModelCapabilities, ModelRuntimeError> {
        // Validate the model is registered, then return the all-false declared
        // capabilities (mirror the BYOK sibling's handle-gate).
        let _ = self
            .inner
            .handle_for(id)
            .map_err(|err| ModelRuntimeError::LoadError(format!("{err}")))?;
        Ok(&self.declared_capabilities)
    }

    fn kv_cache(&self, _id: ModelId) -> Result<KvCacheHandle, ModelRuntimeError> {
        Err(ModelRuntimeError::CapabilityNotSupported {
            capability: "kv_cache (CLI bridge has no local KV cache to expose)".to_string(),
            adapter: CLI_BRIDGE_ADAPTER.to_string(),
        })
    }

    fn lora_stack(&self, _id: ModelId) -> Result<LoraStackHandle, ModelRuntimeError> {
        Err(ModelRuntimeError::CapabilityNotSupported {
            capability: "lora_stack (CLI bridge has no local weights to mount LoRAs onto)"
                .to_string(),
            adapter: CLI_BRIDGE_ADAPTER.to_string(),
        })
    }

    fn steering_hooks(&self, _id: ModelId) -> Result<SteeringHookHandle, ModelRuntimeError> {
        Err(ModelRuntimeError::CapabilityNotSupported {
            capability: "steering_hooks (CLI bridge has no residual stream to hook)".to_string(),
            adapter: CLI_BRIDGE_ADAPTER.to_string(),
        })
    }

    fn cancel(&self, token: CancellationToken) {
        token.cancel();
        self.runtime_cancel.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flight_recorder::events_agent_activity::{
        FR_EVT_AGENT_OTHER, FR_EVT_AGENT_TEXT, FR_EVT_AGENT_THINKING, FR_EVT_AGENT_TOOLCALL,
    };
    use crate::model_runtime::cloud::official_cli_bridge::{
        CliInvocationReceipt, CliKind, CliOutputFormat, OfficialCliBridgeError,
    };
    use crate::model_runtime::{GenPrompt, SamplingParams};
    use futures::StreamExt;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Mutex;

    fn temp_exe() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")
    }

    fn raw_good_config() -> CliBridgeConfig {
        CliBridgeConfig {
            cli_kind: CliKind::ClaudeCode,
            executable_path: temp_exe(),
            args_template: vec!["--prompt".to_string(), "{prompt}".to_string()],
            output_format: CliOutputFormat::RawText,
            env_vars: HashMap::new(),
            working_dir: None,
            timeout_seconds: 120,
        }
    }

    fn allowlisted_config(config: CliBridgeConfig, models: &[&str]) -> AllowlistedCliBridgeConfig {
        AllowlistedCliBridgeConfig::new(
            config,
            CliModelAllowlist::new(models.iter().map(|model| (*model).to_string()))
                .expect("test allowlist"),
        )
    }

    fn good_config() -> AllowlistedCliBridgeConfig {
        allowlisted_config(raw_good_config(), &["claude-sonnet"])
    }

    /// A Claude-dialect config in JSON-stream mode (structured agent-activity
    /// capture ON).
    fn good_config_json() -> AllowlistedCliBridgeConfig {
        let mut config = raw_good_config();
        config.output_format = CliOutputFormat::JsonStream;
        allowlisted_config(config, &["claude-sonnet"])
    }

    fn config_with_model_allowlist(models: &[&str]) -> AllowlistedCliBridgeConfig {
        let mut config = raw_good_config();
        config.env_vars.insert(
            CLI_BRIDGE_MODEL_ALLOWLIST_METADATA_ENV.to_string(),
            serde_json::to_string(models).expect("encode model allowlist fixture"),
        );
        AllowlistedCliBridgeConfig::from_config_metadata(config)
            .expect("decode model allowlist fixture")
    }

    fn official_cli_load_spec(model_name: &str) -> LoadSpec {
        LoadSpec {
            artifact_path: PathBuf::new(),
            sha256_expected: String::new(),
            runtime_kind: crate::model_runtime::RuntimeKind::Candle,
            sampling_defaults: SamplingParams::default(),
            kv_cache_policy: crate::model_runtime::KvCachePolicy::default(),
            declared_capabilities: ModelCapabilities::default(),
            provider: ProviderKind::OfficialCli,
            engine_origin: Some(model_name.to_string()),
            external_engine_import: None,
        }
    }

    fn gen_req(id: ModelId) -> GenerateRequest {
        GenerateRequest {
            id,
            prompt: GenPrompt::new("hello"),
            sampling: SamplingParams::default(),
            lora_overrides: vec![],
            steering_overrides: vec![],
            kv_prefix_handle: None,
            cancel: CancellationToken::new(),
            max_tokens: 64,
            stop_sequences: vec![],
            speculative_mode: None,
            structured_decoding: None,
        }
    }

    /// Mock spawner that emits a fixed list of stdout chunks LIVE via
    /// `spawn_streaming` (and therefore via the default
    /// `spawn_streaming_cancellable` which delegates to it).
    struct ChunkSpawner {
        chunks: Vec<Vec<u8>>,
    }

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
    impl CliSubprocessSpawner for ChunkSpawner {
        fn spawn(
            &self,
            _config: &CliBridgeConfig,
            _invocation: &CliInvocationContext,
            _model_name: &str,
            _prompt: &str,
        ) -> Result<CliInvocationReceipt, OfficialCliBridgeError> {
            let stdout = self
                .chunks
                .iter()
                .map(|c| String::from_utf8_lossy(c).into_owned())
                .collect::<String>();
            Ok(CliInvocationReceipt {
                model_id: ModelId::new_v7(),
                stdout,
                pid: Some(1234),
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
                chunk_sender.try_send(chunk.clone()).map_err(|failure| {
                    OfficialCliBridgeError::SpawnFailed {
                        reason: failure.to_string(),
                        exit_code: None,
                    }
                })?;
                full.extend_from_slice(chunk);
            }
            Ok(CliInvocationReceipt {
                model_id: ModelId::new_v7(),
                stdout: String::from_utf8_lossy(&full).into_owned(),
                pid: Some(1234),
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

    /// Spawner that loops emitting chunks until cancellation is observed,
    /// then kills (simulated) and returns a cancelled receipt. Records that the
    /// cancel hook fired so the test can assert the kill path was driven.
    struct CancelAwareSpawner {
        observed_cancel: Arc<AtomicBool>,
        finished: Arc<AtomicBool>,
        emit_initial_chunk: bool,
    }
    impl CliSubprocessSpawner for CancelAwareSpawner {
        fn spawn(
            &self,
            _config: &CliBridgeConfig,
            _invocation: &CliInvocationContext,
            _model_name: &str,
            _prompt: &str,
        ) -> Result<CliInvocationReceipt, OfficialCliBridgeError> {
            Ok(CliInvocationReceipt {
                model_id: ModelId::new_v7(),
                stdout: String::new(),
                pid: Some(9),
                exit_code: Some(0),
                cancelled: false,
            })
        }
        fn spawn_streaming_cancellable(
            &self,
            _config: &CliBridgeConfig,
            _invocation: &CliInvocationContext,
            _model_name: &str,
            _prompt: &str,
            chunk_sender: &tokio::sync::mpsc::Sender<Vec<u8>>,
            cancellation: &CliCancellationContext,
        ) -> Result<CliInvocationReceipt, OfficialCliBridgeError> {
            // Optionally remain completely silent so drop-cancellation does not
            // depend on receiver-loss being discovered through token delivery.
            if self.emit_initial_chunk {
                chunk_sender
                    .try_send(b"first ".to_vec())
                    .map_err(|failure| OfficialCliBridgeError::SpawnFailed {
                        reason: failure.to_string(),
                        exit_code: None,
                    })?;
            }
            let mut spins = 0u32;
            loop {
                if cancellation.is_cancelled() {
                    self.observed_cancel.store(true, Ordering::SeqCst);
                    self.finished.store(true, Ordering::SeqCst);
                    return Ok(CliInvocationReceipt {
                        model_id: ModelId::new_v7(),
                        stdout: "first ".to_string(),
                        pid: Some(9),
                        exit_code: None,
                        cancelled: true,
                    });
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
                spins += 1;
                if spins > 2000 {
                    // Safety valve so a broken test never hangs CI.
                    self.finished.store(true, Ordering::SeqCst);
                    return Ok(CliInvocationReceipt {
                        model_id: ModelId::new_v7(),
                        stdout: "first ".to_string(),
                        pid: Some(9),
                        exit_code: Some(0),
                        cancelled: false,
                    });
                }
            }
        }
    }

    fn test_invocation_context() -> CliInvocationContext {
        let mut context = CliInvocationContext::new("TEST_ROLE", "claude-sonnet");
        context.owner_wp = Some("WP-TEST".to_string());
        context.role_id = Some("TEST_ROLE".to_string());
        context.wp_id = Some("WP-TEST".to_string());
        context.mt_id = Some("MT-003".to_string());
        context.session_id = Some("mid#0".to_string());
        context.parent_session_id = Some("parent-test".to_string());
        context.trace_id = Some("trace-test".to_string());
        context.requested_trust_class = Some(crate::sandbox::TrustClass::Trusted);
        context.requested_isolation_tier = Some(crate::sandbox::IsolationTier::Tier1Container);
        context.requested_sandbox_capabilities = Some(std::collections::BTreeSet::from([
            crate::sandbox::RequiredCapability::HighStdioThroughput,
        ]));
        context.requested_net_policy = Some(crate::sandbox::NetPolicy::HostInherited);
        context.requested_execution_policy_ref =
            Some("execution-policy://test/cli-runtime".to_string());
        context.swarm_id = Some("runtime-test-swarm".to_string());
        context.worktree_id = Some("runtime-test-worktree".to_string());
        context
    }

    async fn loaded_runtime(
        spawner: Arc<dyn CliSubprocessSpawner>,
    ) -> (CliBridgeModelRuntime, ModelId) {
        let mut rt = CliBridgeModelRuntime::new(spawner, good_config())
            .with_invocation_context(test_invocation_context());
        let spec = crate::model_runtime::LoadSpec {
            artifact_path: PathBuf::new(),
            sha256_expected: String::new(),
            runtime_kind: crate::model_runtime::RuntimeKind::Candle,
            sampling_defaults: SamplingParams::default(),
            kv_cache_policy: crate::model_runtime::KvCachePolicy::default(),
            declared_capabilities: ModelCapabilities::default(),
            provider: ProviderKind::OfficialCli,
            engine_origin: Some("claude-sonnet".to_string()),
            external_engine_import: None,
        };
        let id = rt.load(spec).await.expect("load registers the bridge");
        (rt, id)
    }

    #[tokio::test]
    async fn stored_model_allowlist_accepts_only_declared_model_and_is_not_child_env() {
        let spawner = Arc::new(PinCapturingSpawner::default());
        let mut runtime = CliBridgeModelRuntime::new(
            spawner.clone(),
            config_with_model_allowlist(&["gpt-5.4", "gpt-5.3-codex"]),
        );

        runtime
            .load(official_cli_load_spec("gpt-5.4"))
            .await
            .expect("allowlisted official CLI model loads");

        let pinned = spawner.pinned.lock().expect("pin capture");
        assert_eq!(pinned.len(), 1);
        assert!(
            !pinned[0]
                .env_vars
                .contains_key(CLI_BRIDGE_MODEL_ALLOWLIST_METADATA_ENV),
            "internal allowlist metadata must be consumed before executable config pinning or child env projection"
        );
    }

    #[tokio::test]
    async fn stored_model_allowlist_rejects_undeclared_model_before_registration() {
        let spawner = Arc::new(PinCapturingSpawner::default());
        let mut runtime =
            CliBridgeModelRuntime::new(spawner.clone(), config_with_model_allowlist(&["gpt-5.4"]));

        let error = runtime
            .load(official_cli_load_spec("gpt-5.3-codex"))
            .await
            .expect_err("undeclared official CLI model must fail closed");

        assert!(error.to_string().contains("not in the operator allowlist"));
        assert!(
            spawner.pinned.lock().expect("pin capture").is_empty(),
            "rejected model must fail before executable registration"
        );
    }

    #[tokio::test]
    async fn malformed_stored_model_allowlist_fails_closed_before_registration() {
        let spawner = Arc::new(PinCapturingSpawner::default());
        let mut config = raw_good_config();
        config.env_vars.insert(
            CLI_BRIDGE_MODEL_ALLOWLIST_METADATA_ENV.to_string(),
            "not-json".to_string(),
        );
        let error = AllowlistedCliBridgeConfig::from_config_metadata(config)
            .expect_err("malformed stored allowlist must fail closed at construction");

        assert!(error.contains("allowlist metadata is invalid"));
        assert!(spawner.pinned.lock().expect("pin capture").is_empty());
    }

    /// Test 1: generate streams stdout AS TOKENS live (>=3 tokens, in order,
    /// terminal Stop).
    #[tokio::test]
    async fn generate_streams_stdout_as_tokens_live() {
        let spawner = Arc::new(ChunkSpawner {
            chunks: vec![
                b"chunk-one ".to_vec(),
                b"chunk-two ".to_vec(),
                b"chunk-three".to_vec(),
            ],
        });
        let (rt, id) = loaded_runtime(spawner).await;
        let mut stream = rt.generate(gen_req(id));

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
        assert!(
            texts.len() >= 3,
            "expected >=3 streamed tokens, got {texts:?}"
        );
        assert_eq!(
            texts.join(""),
            "chunk-one chunk-two chunk-three",
            "tokens concatenate to the CLI stdout, in order"
        );
        assert_eq!(terminal, Some(FinishReason::Stop));
    }

    /// Test 2: a spawn failure surfaces as a stream ERROR item, not an empty
    /// stream.
    #[tokio::test]
    async fn generate_surfaces_spawn_failure_as_stream_error() {
        let (rt, id) = loaded_runtime(Arc::new(FailingSpawner)).await;
        let mut stream = rt.generate(gen_req(id));
        let mut saw_error = false;
        let mut saw_any = false;
        while let Some(item) = stream.next().await {
            saw_any = true;
            if let Err(ModelRuntimeError::GenerateError(_)) = item {
                saw_error = true;
            }
        }
        assert!(saw_any, "stream must not be silently empty");
        assert!(
            saw_error,
            "spawn failure must surface as a GenerateError item"
        );
    }

    /// Test 3: cancellation kills the child and ends the stream with
    /// FinishReason::Cancelled; the spawner observed the cancel hook.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn generate_cancellation_kills_child_and_ends_stream() {
        let observed = Arc::new(AtomicBool::new(false));
        let finished = Arc::new(AtomicBool::new(false));
        let spawner = Arc::new(CancelAwareSpawner {
            observed_cancel: observed.clone(),
            finished: finished.clone(),
            emit_initial_chunk: true,
        });
        let (rt, id) = loaded_runtime(spawner).await;
        let mut req = gen_req(id);
        let cancel = req.cancel.clone();
        req = GenerateRequest {
            cancel: cancel.clone(),
            ..req
        };

        let mut stream = rt.generate(req);
        // Pull the first real token, then cancel.
        let first = stream.next().await.expect("first item").expect("ok");
        assert_eq!(first.text, "first ");
        cancel.cancel();

        // Drain to the terminal; it must be Cancelled.
        let mut terminal: Option<FinishReason> = None;
        while let Some(item) = stream.next().await {
            if let Ok(token) = item {
                if let Some(fr) = token.finish_reason {
                    terminal = Some(fr);
                }
            }
        }
        assert_eq!(terminal, Some(FinishReason::Cancelled));
        assert!(
            observed.load(Ordering::SeqCst),
            "the spawner must have observed the cancel hook (kill path driven)"
        );
        assert!(finished.load(Ordering::SeqCst));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn dropping_unpolled_silent_stream_cancels_and_finishes_spawner() {
        let observed = Arc::new(AtomicBool::new(false));
        let finished = Arc::new(AtomicBool::new(false));
        let spawner = Arc::new(CancelAwareSpawner {
            observed_cancel: observed.clone(),
            finished: finished.clone(),
            emit_initial_chunk: false,
        });
        let (rt, id) = loaded_runtime(spawner).await;

        let stream = rt.generate(gen_req(id));
        drop(stream);

        tokio::time::timeout(std::time::Duration::from_secs(1), async {
            while !finished.load(Ordering::SeqCst) {
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("silent spawner must finish after its unpolled stream is dropped");
        assert!(
            observed.load(Ordering::SeqCst),
            "stream drop must reach the spawner cancellation context"
        );
    }

    /// Test 6: drained token stream equals the CLI stdout byte-for-byte, even
    /// when a multibyte UTF-8 char is split across a chunk boundary. Proves the
    /// swarm capture seam would receive the real CLI output and the UTF-8
    /// boundary carry is correct.
    #[tokio::test]
    async fn drained_token_stream_equals_cli_stdout_across_utf8_boundary() {
        // "héllo wörld 🎉" — split the 'é' (2 bytes: 0xC3 0xA9) and the '🎉'
        // (4 bytes) across chunk boundaries.
        let full = "héllo wörld 🎉";
        let bytes = full.as_bytes().to_vec();
        // Find the byte index of 'é' (after 'h').
        // Build chunks that deliberately split multibyte sequences.
        let e_start = "h".len(); // 1
        let chunk1 = bytes[..e_start + 1].to_vec(); // 'h' + first byte of 'é'
                                                    // emoji is the last 4 bytes; split it 2/2.
        let emoji_len = "🎉".len(); // 4
        let emoji_start = bytes.len() - emoji_len;
        let chunk2 = bytes[e_start + 1..emoji_start + 2].to_vec(); // rest of é .. first half of emoji
        let chunk3 = bytes[emoji_start + 2..].to_vec(); // second half of emoji

        let spawner = Arc::new(ChunkSpawner {
            chunks: vec![chunk1, chunk2, chunk3],
        });
        let (rt, id) = loaded_runtime(spawner).await;
        let mut stream = rt.generate(gen_req(id));

        let mut out = String::new();
        while let Some(item) = stream.next().await {
            let token = item.expect("ok");
            if token.finish_reason.is_none() {
                out.push_str(&token.text);
            }
        }
        assert_eq!(
            out, full,
            "reassembled token text must equal the CLI stdout"
        );
    }

    /// Sanity: every CLI-bridge capability is false and the unsupported methods
    /// return CapabilityNotSupported with the right adapter label.
    #[tokio::test]
    async fn unsupported_capabilities_are_typed_not_faked() {
        let (rt, id) = loaded_runtime(Arc::new(ChunkSpawner { chunks: vec![] })).await;
        let caps = rt.capabilities(id).expect("registered model has caps");
        assert!(!caps.supports_lora);
        assert!(!caps.supports_kv_prefix_cache);
        assert!(matches!(
            rt.score(id, vec![]).await,
            Err(ModelRuntimeError::CapabilityNotSupported { .. })
        ));
        assert!(matches!(
            rt.embed(id, "x").await,
            Err(ModelRuntimeError::CapabilityNotSupported { .. })
        ));
        assert!(matches!(
            rt.kv_cache(id),
            Err(ModelRuntimeError::CapabilityNotSupported { .. })
        ));
        assert!(matches!(
            rt.lora_stack(id),
            Err(ModelRuntimeError::CapabilityNotSupported { .. })
        ));
        assert!(matches!(
            rt.steering_hooks(id),
            Err(ModelRuntimeError::CapabilityNotSupported { .. })
        ));
    }

    /// Capturing FlightRecorder: collects each event's payload so the test can
    /// assert the FR-EVT-LLM-INFER phases were emitted.
    #[derive(Default)]
    struct CollectingRecorder {
        payloads: Mutex<Vec<serde_json::Value>>,
    }
    #[async_trait]
    impl FlightRecorder for CollectingRecorder {
        async fn record_event(
            &self,
            event: FlightRecorderEvent,
        ) -> Result<(), crate::flight_recorder::RecorderError> {
            self.payloads.lock().unwrap().push(event.payload);
            Ok(())
        }
        async fn enforce_retention(&self) -> Result<u64, crate::flight_recorder::RecorderError> {
            Ok(0)
        }
        async fn list_events(
            &self,
            _filter: crate::flight_recorder::EventFilter,
        ) -> Result<Vec<FlightRecorderEvent>, crate::flight_recorder::RecorderError> {
            Ok(Vec::new())
        }
    }

    /// When a `CloudLaneObservability` is threaded in, `generate()` emits
    /// FR-EVT-LLM-INFER-{START,TOKEN,END} through the recorder — START once
    /// before tokens, END once after, a sampled TOKEN at index 16, all carrying
    /// the same request_id correlation. Generation output is unchanged.
    #[tokio::test]
    async fn generate_emits_fr_evt_llm_infer_start_token_end() {
        use crate::flight_recorder::events_llm_infer::{
            FR_EVT_LLM_INFER_END, FR_EVT_LLM_INFER_START, FR_EVT_LLM_INFER_TOKEN,
        };

        let recorder = Arc::new(CollectingRecorder::default());
        let obs = Arc::new(CloudLaneObservability {
            flight_recorder: recorder.clone() as Arc<dyn FlightRecorder>,
            consent: None,
        });
        // 20 single-byte chunks => 20 generated tokens => token index 16 fires a
        // sampled TOKEN event (LLM_INFER_TOKEN_SAMPLE_INTERVAL = 16).
        let chunks: Vec<Vec<u8>> = (0..20).map(|_| b"x".to_vec()).collect();
        let mut rt = CliBridgeModelRuntime::new(Arc::new(ChunkSpawner { chunks }), good_config())
            .with_invocation_context(test_invocation_context())
            .with_lane_observability(obs);
        let spec = crate::model_runtime::LoadSpec {
            artifact_path: PathBuf::new(),
            sha256_expected: String::new(),
            runtime_kind: crate::model_runtime::RuntimeKind::Candle,
            sampling_defaults: SamplingParams::default(),
            kv_cache_policy: crate::model_runtime::KvCachePolicy::default(),
            declared_capabilities: ModelCapabilities::default(),
            provider: ProviderKind::OfficialCli,
            engine_origin: Some("claude-sonnet".to_string()),
            external_engine_import: None,
        };
        let id = rt.load(spec).await.expect("load");

        // Drain the stream fully so START/TOKEN/END all flush.
        let mut stream = rt.generate(gen_req(id));
        let mut generated = 0usize;
        while let Some(item) = stream.next().await {
            let token = item.expect("no error item");
            if token.finish_reason.is_none() && !token.text.is_empty() {
                generated += 1;
            }
        }
        assert_eq!(generated, 20, "all 20 stdout tokens stream through");

        let payloads = recorder.payloads.lock().unwrap().clone();
        let ids: Vec<String> = payloads
            .iter()
            .filter_map(|p| p.get("event_id").and_then(|v| v.as_str()).map(String::from))
            .collect();
        let count = |needle: &str| ids.iter().filter(|e| e.as_str() == needle).count();
        assert_eq!(
            count(FR_EVT_LLM_INFER_START),
            1,
            "exactly one START: {ids:?}"
        );
        assert_eq!(count(FR_EVT_LLM_INFER_END), 1, "exactly one END: {ids:?}");
        assert!(
            count(FR_EVT_LLM_INFER_TOKEN) >= 1,
            "at least one sampled TOKEN: {ids:?}"
        );
        // START precedes END; all share one request_id correlation.
        let start_idx = ids
            .iter()
            .position(|e| e == FR_EVT_LLM_INFER_START)
            .unwrap();
        let end_idx = ids.iter().rposition(|e| e == FR_EVT_LLM_INFER_END).unwrap();
        assert!(start_idx < end_idx, "START must precede END");
        let req_ids: std::collections::HashSet<String> = payloads
            .iter()
            .filter_map(|p| {
                p.get("request_id")
                    .and_then(|v| v.as_str())
                    .map(String::from)
            })
            .collect();
        assert_eq!(
            req_ids.len(),
            1,
            "all infer events share one request_id: {req_ids:?}"
        );

        // The END event reports the real generated-token count.
        let end = payloads
            .iter()
            .find(|p| p.get("event_id").and_then(|v| v.as_str()) == Some(FR_EVT_LLM_INFER_END))
            .unwrap();
        assert_eq!(
            end.get("tokens_generated").and_then(|v| v.as_u64()),
            Some(20)
        );
    }

    /// Without observability, generate() emits NO FR events but produces the
    /// same tokens — observability is purely additive.
    #[tokio::test]
    async fn generate_without_observability_emits_no_fr_events_same_output() {
        let (rt, id) = loaded_runtime(Arc::new(ChunkSpawner {
            chunks: vec![b"a".to_vec(), b"b".to_vec()],
        }))
        .await;
        let mut stream = rt.generate(gen_req(id));
        let mut out = String::new();
        while let Some(item) = stream.next().await {
            let token = item.expect("ok");
            if token.finish_reason.is_none() {
                out.push_str(&token.text);
            }
        }
        assert_eq!(out, "ab");
    }

    fn cli_spec() -> crate::model_runtime::LoadSpec {
        crate::model_runtime::LoadSpec {
            artifact_path: PathBuf::new(),
            sha256_expected: String::new(),
            runtime_kind: crate::model_runtime::RuntimeKind::Candle,
            sampling_defaults: SamplingParams::default(),
            kv_cache_policy: crate::model_runtime::KvCachePolicy::default(),
            declared_capabilities: ModelCapabilities::default(),
            provider: ProviderKind::OfficialCli,
            engine_origin: Some("claude-sonnet".to_string()),
            external_engine_import: None,
        }
    }

    fn obs_with(recorder: Arc<CollectingRecorder>) -> Arc<CloudLaneObservability> {
        Arc::new(CloudLaneObservability {
            flight_recorder: recorder as Arc<dyn FlightRecorder>,
            consent: None,
        })
    }

    /// Drain a stream fully, returning the concatenated non-terminal token text.
    async fn drain_text(mut stream: TokenStream) -> String {
        let mut out = String::new();
        while let Some(item) = stream.next().await {
            if let Ok(token) = item {
                if token.finish_reason.is_none() {
                    out.push_str(&token.text);
                }
            }
        }
        out
    }

    fn agent_event_ids(recorder: &CollectingRecorder) -> Vec<String> {
        recorder
            .payloads
            .lock()
            .unwrap()
            .iter()
            .filter_map(|p| p.get("event_id").and_then(|v| v.as_str()).map(String::from))
            .filter(|e| e.starts_with("FR-EVT-AGENT-"))
            .collect()
    }

    /// JSON-stream mode: a fixture claude stream yields typed FR-EVT-AGENT-*
    /// events with the right event_id, instance_id (session_span), and a
    /// request_id shared with the INFER events. Raw token output is unchanged.
    #[tokio::test]
    async fn json_stream_mode_emits_typed_agent_events_with_correlation() {
        let recorder = Arc::new(CollectingRecorder::default());
        // One claude assistant line (text + thinking + tool_use), newline-terminated.
        let line = serde_json::json!({
            "type":"assistant",
            "message":{"content":[
                {"type":"text","text":"hi"},
                {"type":"thinking","thinking":"hmm"},
                {"type":"tool_use","id":"toolu_1","name":"Bash","input":{"command":"ls"}}
            ]}
        })
        .to_string();
        let stdout = format!("{line}\n");
        let chunks: Vec<Vec<u8>> = vec![stdout.into_bytes()];

        let mut rt =
            CliBridgeModelRuntime::new(Arc::new(ChunkSpawner { chunks }), good_config_json())
                .with_invocation_context(test_invocation_context())
                .with_lane_observability(obs_with(recorder.clone()))
                .with_session_correlation("mid#0");
        let id = rt.load(cli_spec()).await.expect("load");

        let _ = drain_text(rt.generate(gen_req(id))).await;

        let ids = agent_event_ids(&recorder);
        assert!(ids.contains(&FR_EVT_AGENT_TEXT.to_string()), "{ids:?}");
        assert!(ids.contains(&FR_EVT_AGENT_THINKING.to_string()), "{ids:?}");
        assert!(ids.contains(&FR_EVT_AGENT_TOOLCALL.to_string()), "{ids:?}");

        let payloads = recorder.payloads.lock().unwrap().clone();
        let agent: Vec<&serde_json::Value> = payloads
            .iter()
            .filter(|p| {
                p.get("event_id")
                    .and_then(|v| v.as_str())
                    .map(|e| e.starts_with("FR-EVT-AGENT-"))
                    .unwrap_or(false)
            })
            .collect();
        // instance_id stamped on payload of every agent event.
        for p in &agent {
            assert_eq!(p.get("instance_id").and_then(|v| v.as_str()), Some("mid#0"));
        }
        // The toolcall carries the redacted detail + name.
        let toolcall = agent
            .iter()
            .find(|p| p.get("event_id").and_then(|v| v.as_str()) == Some(FR_EVT_AGENT_TOOLCALL))
            .unwrap();
        assert_eq!(toolcall.get("name").and_then(|v| v.as_str()), Some("Bash"));
        assert_eq!(
            toolcall
                .get("detail")
                .and_then(|d| d.get("command"))
                .unwrap(),
            "ls"
        );
        // Agent + INFER events share the one request_id correlation.
        let req_ids: std::collections::HashSet<String> = payloads
            .iter()
            .filter_map(|p| {
                p.get("request_id")
                    .and_then(|v| v.as_str())
                    .map(String::from)
            })
            .collect();
        assert_eq!(
            req_ids.len(),
            1,
            "one request_id across infer+agent: {req_ids:?}"
        );
    }

    /// Operator Chat replays launched stdout into ModelLaneMessage rows and emits
    /// its own FR-EVT-AGENT-* evidence there. The runtime can keep infer
    /// observability while suppressing its duplicate agent-activity producer.
    #[tokio::test]
    async fn json_stream_mode_can_suppress_agent_events_without_suppressing_infer_events() {
        use crate::flight_recorder::events_llm_infer::{
            FR_EVT_LLM_INFER_END, FR_EVT_LLM_INFER_START,
        };

        let recorder = Arc::new(CollectingRecorder::default());
        let line = serde_json::json!({
            "type":"assistant",
            "message":{"content":[{"type":"text","text":"hi"}]}
        })
        .to_string();
        let chunks: Vec<Vec<u8>> = vec![format!("{line}\n").into_bytes()];

        let mut rt =
            CliBridgeModelRuntime::new(Arc::new(ChunkSpawner { chunks }), good_config_json())
                .with_invocation_context(test_invocation_context())
                .with_lane_observability(obs_with(recorder.clone()))
                .without_agent_activity_events();
        let id = rt.load(cli_spec()).await.expect("load");

        let _ = drain_text(rt.generate(gen_req(id))).await;

        let payloads = recorder.payloads.lock().unwrap().clone();
        let ids: Vec<String> = payloads
            .iter()
            .filter_map(|p| p.get("event_id").and_then(|v| v.as_str()).map(String::from))
            .collect();
        assert!(
            ids.contains(&FR_EVT_LLM_INFER_START.to_string())
                && ids.contains(&FR_EVT_LLM_INFER_END.to_string()),
            "infer observability remains wired: {ids:?}"
        );
        let agent_ids: Vec<String> = ids
            .into_iter()
            .filter(|id| id.starts_with("FR-EVT-AGENT-"))
            .collect();
        assert!(
            agent_ids.is_empty(),
            "runtime-level FR-EVT-AGENT-* events are suppressed for replay-captured lanes: {agent_ids:?}"
        );
    }

    /// RawText mode: NO agent events, and byte-identical token output (the
    /// honesty / no-regression gate).
    #[tokio::test]
    async fn raw_text_mode_emits_no_agent_events_same_output() {
        let recorder = Arc::new(CollectingRecorder::default());
        let line = "{\"type\":\"assistant\"}\n";
        let chunks: Vec<Vec<u8>> = vec![line.as_bytes().to_vec()];
        let mut rt = CliBridgeModelRuntime::new(
            Arc::new(ChunkSpawner { chunks }),
            good_config(), // RawText
        )
        .with_invocation_context(test_invocation_context())
        .with_lane_observability(obs_with(recorder.clone()))
        .with_session_correlation("mid#0");
        let id = rt.load(cli_spec()).await.expect("load");

        let out = drain_text(rt.generate(gen_req(id))).await;
        assert_eq!(
            out, line,
            "raw token output must be byte-identical in RawText mode"
        );
        assert!(
            agent_event_ids(&recorder).is_empty(),
            "RawText mode must emit ZERO agent events"
        );
    }

    /// A malformed JSON line in JSON-stream mode still streams its raw token AND
    /// yields exactly one FR-EVT-AGENT-OTHER event (no data loss, generation
    /// unaffected).
    #[tokio::test]
    async fn json_stream_malformed_line_streams_raw_and_yields_other() {
        let recorder = Arc::new(CollectingRecorder::default());
        let line = "{not valid json\n";
        let chunks: Vec<Vec<u8>> = vec![line.as_bytes().to_vec()];
        let mut rt =
            CliBridgeModelRuntime::new(Arc::new(ChunkSpawner { chunks }), good_config_json())
                .with_invocation_context(test_invocation_context())
                .with_lane_observability(obs_with(recorder.clone()))
                .with_session_correlation("mid#0");
        let id = rt.load(cli_spec()).await.expect("load");

        let out = drain_text(rt.generate(gen_req(id))).await;
        assert_eq!(out, line, "raw token still streams unchanged");
        let ids = agent_event_ids(&recorder);
        assert_eq!(
            ids,
            vec![FR_EVT_AGENT_OTHER.to_string()],
            "malformed -> one OTHER"
        );
    }

    /// A JSONL stream NOT ending with '\n' flushes its final partial line at
    /// stream end (never dropped).
    #[tokio::test]
    async fn json_stream_flushes_final_partial_line() {
        let recorder = Arc::new(CollectingRecorder::default());
        // No trailing newline.
        let line = serde_json::json!({
            "type":"assistant",
            "message":{"content":[{"type":"text","text":"tail"}]}
        })
        .to_string();
        let chunks: Vec<Vec<u8>> = vec![line.into_bytes()];
        let mut rt =
            CliBridgeModelRuntime::new(Arc::new(ChunkSpawner { chunks }), good_config_json())
                .with_invocation_context(test_invocation_context())
                .with_lane_observability(obs_with(recorder.clone()));
        let id = rt.load(cli_spec()).await.expect("load");
        let _ = drain_text(rt.generate(gen_req(id))).await;
        assert!(
            agent_event_ids(&recorder).contains(&FR_EVT_AGENT_TEXT.to_string()),
            "final partial line must flush as a TEXT event"
        );
    }

    /// AgentActivityLineBuffer unit coverage: splits on '\n', retains a partial
    /// tail, strips CRLF, and flushes the remainder.
    #[test]
    fn agent_line_buffer_splits_retains_and_flushes() {
        let mut buf = AgentActivityLineBuffer::new();
        // Partial line, no newline yet.
        assert!(buf.push("hel").is_empty());
        // Completes the first line + starts a second.
        assert_eq!(buf.push("lo\r\nwor"), vec!["hello".to_string()]);
        // Flush the retained partial tail.
        assert_eq!(buf.flush(), Some("wor".to_string()));
        assert_eq!(buf.flush(), None);
    }

    /// Utf8ChunkDecoder unit coverage: split codepoint carries forward.
    #[test]
    fn utf8_decoder_carries_split_codepoint() {
        let mut d = Utf8ChunkDecoder::new();
        let e = "é".as_bytes(); // [0xC3, 0xA9]
        let a = d.push(&[b'h', e[0]]); // 'h' + incomplete 'é'
        assert_eq!(a, "h");
        let b = d.push(&[e[1], b'i']); // completes 'é' + 'i'
        assert_eq!(b, "éi");
        assert_eq!(d.finish(), "");
    }

    /// load() rejects a non-OfficialCli provider with a typed LoadError.
    #[tokio::test]
    async fn load_rejects_wrong_provider() {
        let mut rt =
            CliBridgeModelRuntime::new(Arc::new(ChunkSpawner { chunks: vec![] }), good_config());
        let mut spec = crate::model_runtime::LoadSpec {
            artifact_path: PathBuf::new(),
            sha256_expected: String::new(),
            runtime_kind: crate::model_runtime::RuntimeKind::Candle,
            sampling_defaults: SamplingParams::default(),
            kv_cache_policy: crate::model_runtime::KvCachePolicy::default(),
            declared_capabilities: ModelCapabilities::default(),
            provider: ProviderKind::ByokCloud,
            engine_origin: Some("claude-sonnet".to_string()),
            external_engine_import: None,
        };
        assert!(matches!(
            rt.load(spec).await,
            Err(ModelRuntimeError::LoadError(_))
        ));
        // Wrong because engine_origin is missing.
        spec = crate::model_runtime::LoadSpec {
            artifact_path: PathBuf::new(),
            sha256_expected: String::new(),
            runtime_kind: crate::model_runtime::RuntimeKind::Candle,
            sampling_defaults: SamplingParams::default(),
            kv_cache_policy: crate::model_runtime::KvCachePolicy::default(),
            declared_capabilities: ModelCapabilities::default(),
            provider: ProviderKind::OfficialCli,
            engine_origin: None,
            external_engine_import: None,
        };
        assert!(matches!(
            rt.load(spec).await,
            Err(ModelRuntimeError::LoadError(_))
        ));
    }

    // Keep the unused Mutex import meaningful for future capturing spawners.
    #[allow(dead_code)]
    fn _mutex_marker() -> Mutex<()> {
        Mutex::new(())
    }

    /// A FlightRecorder that captures FULL events (not just `event.payload`), so
    /// a test can assert `session_span_id` — the field the transcript's raw seam
    /// (`session_span_id == <session>`) keys on, which `CollectingRecorder` drops.
    #[derive(Default)]
    struct FullEventRecorder {
        events: Mutex<Vec<FlightRecorderEvent>>,
    }
    #[async_trait]
    impl FlightRecorder for FullEventRecorder {
        async fn record_event(
            &self,
            event: FlightRecorderEvent,
        ) -> Result<(), crate::flight_recorder::RecorderError> {
            self.events.lock().unwrap().push(event);
            Ok(())
        }
        async fn enforce_retention(&self) -> Result<u64, crate::flight_recorder::RecorderError> {
            Ok(0)
        }
        async fn list_events(
            &self,
            _filter: crate::flight_recorder::EventFilter,
        ) -> Result<Vec<FlightRecorderEvent>, crate::flight_recorder::RecorderError> {
            Ok(Vec::new())
        }
    }

    /// PRODUCTION-WIRING integration: build the CLI runtime through the REAL
    /// production seam (`CliBridgeCloudRuntimeBuilder::build_loaded`, the same
    /// path `ProductionModelSessionFactory::create_cloud` drives) WITHOUT any
    /// manual `with_session_correlation`, then assert the emitted
    /// `FR-EVT-AGENT-*` events are scoped to the swarm composite session id on
    /// BOTH transcript seams: `session_span_id == <model_id>#<instance>` (seam 2a)
    /// AND `payload.instance_id == <model_id>#<instance>` (seam 2b). This is the
    /// regression guard for the HIGH defect where production built the runtime
    /// without threading the composite, making the feature transcript-invisible
    /// end-to-end. `fetch_fr_events` lives in the app crate; this test asserts the
    /// exact fields those two seams query rather than calling the seam directly.
    #[tokio::test]
    async fn production_builder_threads_session_correlation_for_transcript() {
        use crate::swarm_orchestration::production_factory::{
            CliBridgeCloudRuntimeBuilder, CloudRuntimeBuilder,
        };

        let recorder = Arc::new(FullEventRecorder::default());
        let obs = Arc::new(CloudLaneObservability {
            flight_recorder: recorder.clone() as Arc<dyn FlightRecorder>,
            consent: None,
        });

        // One claude assistant line (text + thinking + tool_use), newline-terminated.
        let line = serde_json::json!({
            "type":"assistant",
            "message":{"content":[
                {"type":"text","text":"hi"},
                {"type":"thinking","thinking":"hmm"},
                {"type":"tool_use","id":"toolu_1","name":"Bash","input":{"command":"ls"}}
            ]}
        })
        .to_string();
        let chunks: Vec<Vec<u8>> = vec![format!("{line}\n").into_bytes()];

        // Build through the production builder. NOTE: no with_session_correlation
        // call anywhere — the builder must thread the composite itself.
        let builder = CliBridgeCloudRuntimeBuilder::new(
            Arc::new(ChunkSpawner { chunks }),
            good_config_json(),
        )
        .with_observability(obs);

        // Simulate the coordinator's session key: the request's instance_id is
        // ModelInstanceId::new(<placeholder model_id>, instance) whose model id is
        // DISTINCT from the one the runtime's load() mints. The production caller
        // (`create_cloud`) passes `request.instance_id.to_string()`; agent events
        // must carry THAT verbatim — NOT a composite re-formed from the loaded
        // model id — or they never match the transcript's session scope.
        const INSTANCE: u32 = 3;
        let coordinator_session = format!("{}#{INSTANCE}", ModelId::new_v7());
        let mut invocation_context = test_invocation_context();
        invocation_context.session_id = Some(coordinator_session.clone());
        invocation_context.parent_session_id = Some("parent-production-test".to_string());
        let live = builder
            .build_loaded("claude-sonnet", Some(invocation_context), None)
            .await
            .expect("production builder build_loaded");
        let model_id = live.model_id;
        // The fix's whole point: the coordinator key is NOT the loaded-model
        // composite. Prove they differ so the seam assertions below are meaningful
        // (this is the exact divergence the HIGH-defect remediation missed).
        assert_ne!(
            coordinator_session,
            format!("{model_id}#{INSTANCE}"),
            "test integrity: coordinator session id must differ from the loaded-model composite"
        );
        let expected_session = coordinator_session;

        // Drive a real generate through the Arc<dyn ModelRuntime> the factory hands
        // the swarm.
        let mut stream = live.runtime.generate(gen_req(model_id));
        while stream.next().await.is_some() {}

        let events = recorder.events.lock().unwrap().clone();
        let agent: Vec<&FlightRecorderEvent> = events
            .iter()
            .filter(|e| {
                e.payload
                    .get("event_id")
                    .and_then(|v| v.as_str())
                    .map(|id| id.starts_with("FR-EVT-AGENT-"))
                    .unwrap_or(false)
            })
            .collect();
        assert!(
            !agent.is_empty(),
            "production-built JSON-stream runtime must emit FR-EVT-AGENT-* events"
        );

        // Every agent event is retrievable through BOTH transcript seams for the
        // swarm composite session id — the end-to-end visibility the HIGH defect
        // broke.
        for ev in &agent {
            assert_eq!(
                ev.session_span_id.as_deref(),
                Some(expected_session.as_str()),
                "seam 2a (session_span_id) must carry the swarm composite session id"
            );
            assert_eq!(
                ev.payload.get("instance_id").and_then(|v| v.as_str()),
                Some(expected_session.as_str()),
                "seam 2b (payload.instance_id) must carry the swarm composite session id"
            );
        }
    }
}

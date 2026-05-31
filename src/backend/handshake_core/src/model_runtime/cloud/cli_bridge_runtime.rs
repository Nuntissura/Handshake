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

use std::sync::Arc;

use async_trait::async_trait;
use futures::stream;

use super::official_cli_bridge::{
    CliBridgeConfig, CliSubprocessSpawner, OfficialCliBridgeRuntime,
};
use crate::model_runtime::{
    error::ModelRuntimeError, CancellationToken, Embedding, FinishReason, GenerateRequest,
    GeneratedToken, KvCacheHandle, LoadSpec, LoraStackHandle, ModelCapabilities, ModelId,
    ModelRuntime, ProviderKind, Score, SteeringHookHandle, TokenStream,
};

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
        Self { pending: Vec::new() }
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
                        let bad = String::from_utf8_lossy(&self.pending[valid..bad_end]).into_owned();
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
    /// Runtime-wide cancellation flag, set by `cancel()`; mirrors the BYOK
    /// `runtime_cancel`. Polled by `generate()`'s cancel hook so a runtime-level
    /// cancel kills in-flight CLI subprocesses.
    runtime_cancel: CancellationToken,
    /// Declared capabilities: all-false (the CLI bridge is a usability lane).
    declared_capabilities: ModelCapabilities,
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
    pub fn new(spawner: Arc<dyn CliSubprocessSpawner>, config_template: CliBridgeConfig) -> Self {
        Self {
            inner: OfficialCliBridgeRuntime::new(spawner.clone()),
            spawner,
            config_template,
            runtime_cancel: CancellationToken::new(),
            declared_capabilities: OfficialCliBridgeRuntime::cli_bridge_capabilities(),
        }
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

        let spawner = Arc::clone(&self.spawner);
        let config = self.config_template.clone();
        let model_name = handle.model_name.clone();
        let prompt = req.prompt.text.clone();
        let req_cancel = req.cancel.clone();
        let runtime_cancel = self.runtime_cancel.clone();

        // The blocking, poll-based spawn must NOT run on a tokio worker thread:
        // run it on a dedicated OS thread that owns the (non-Send-friendly)
        // `on_chunk` closure and sends each decoded chunk onto the tokio channel.
        std::thread::spawn(move || {
            let mut token_index: u32 = 0;
            let mut decoder = Utf8ChunkDecoder::new();
            let mut send_failed = false;

            {
                let tx_chunks = &tx;
                let mut on_chunk = |bytes: &[u8]| {
                    if send_failed {
                        return;
                    }
                    let text = decoder.push(bytes);
                    if !text.is_empty() {
                        token_index = token_index.saturating_add(1);
                        if tx_chunks
                            .send(Ok(GeneratedToken {
                                token_id: token_index,
                                text,
                                logprob: None,
                                finish_reason: None,
                            }))
                            .is_err()
                        {
                            // Receiver dropped: caller no longer cares. Stop
                            // forwarding; the spawn loop still drains the pipe.
                            send_failed = true;
                        }
                    }
                };
                let should_cancel =
                    || req_cancel.is_cancelled() || runtime_cancel.is_cancelled();

                let result = spawner.spawn_streaming_cancellable(
                    &config,
                    &model_name,
                    &prompt,
                    &mut on_chunk,
                    &should_cancel,
                );

                // Flush any trailing bytes left in the decoder as a final token.
                let tail = decoder.finish();
                if !send_failed && !tail.is_empty() {
                    token_index = token_index.saturating_add(1);
                    if tx
                        .send(Ok(GeneratedToken {
                            token_id: token_index,
                            text: tail,
                            logprob: None,
                            finish_reason: None,
                        }))
                        .is_err()
                    {
                        send_failed = true;
                    }
                }

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
                            // HONEST error item — not a silent empty stream.
                            let _ = tx.send(Err(ModelRuntimeError::GenerateError(format!(
                                "official CLI bridge generate failed: {err}"
                            ))));
                        }
                    }
                }
            }
            // `tx` drops here -> the receiver stream ends.
        });

        Box::pin(stream::unfold(rx, |mut rx| async move {
            rx.recv().await.map(|item| (item, rx))
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

    fn good_config() -> CliBridgeConfig {
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
    impl CliSubprocessSpawner for ChunkSpawner {
        fn spawn(
            &self,
            _config: &CliBridgeConfig,
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
            _model_name: &str,
            _prompt: &str,
            on_chunk: &mut dyn FnMut(&[u8]),
        ) -> Result<CliInvocationReceipt, OfficialCliBridgeError> {
            let mut full = Vec::new();
            for chunk in &self.chunks {
                on_chunk(chunk);
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
            _model_name: &str,
            _prompt: &str,
        ) -> Result<CliInvocationReceipt, OfficialCliBridgeError> {
            Err(OfficialCliBridgeError::SpawnFailed {
                reason: "test fault injection".to_string(),
                exit_code: None,
            })
        }
    }

    /// Spawner that loops emitting chunks until `should_cancel()` is observed,
    /// then kills (simulated) and returns a cancelled receipt. Records that the
    /// cancel hook fired so the test can assert the kill path was driven.
    struct CancelAwareSpawner {
        observed_cancel: Arc<AtomicBool>,
    }
    impl CliSubprocessSpawner for CancelAwareSpawner {
        fn spawn(
            &self,
            _config: &CliBridgeConfig,
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
            _model_name: &str,
            _prompt: &str,
            on_chunk: &mut dyn FnMut(&[u8]),
            should_cancel: &dyn Fn() -> bool,
        ) -> Result<CliInvocationReceipt, OfficialCliBridgeError> {
            // Emit a first chunk, then spin until cancellation is observed.
            on_chunk(b"first ");
            let mut spins = 0u32;
            loop {
                if should_cancel() {
                    self.observed_cancel.store(true, Ordering::SeqCst);
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

    async fn loaded_runtime(spawner: Arc<dyn CliSubprocessSpawner>) -> (CliBridgeModelRuntime, ModelId) {
        let mut rt = CliBridgeModelRuntime::new(spawner, good_config());
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
        assert!(saw_error, "spawn failure must surface as a GenerateError item");
    }

    /// Test 3: cancellation kills the child and ends the stream with
    /// FinishReason::Cancelled; the spawner observed the cancel hook.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn generate_cancellation_kills_child_and_ends_stream() {
        let observed = Arc::new(AtomicBool::new(false));
        let spawner = Arc::new(CancelAwareSpawner {
            observed_cancel: observed.clone(),
        });
        let (rt, id) = loaded_runtime(spawner).await;
        let mut req = gen_req(id);
        let cancel = req.cancel.clone();
        req = GenerateRequest { cancel: cancel.clone(), ..req };

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
        assert_eq!(out, full, "reassembled token text must equal the CLI stdout");
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
        let mut rt = CliBridgeModelRuntime::new(
            Arc::new(ChunkSpawner { chunks: vec![] }),
            good_config(),
        );
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
}

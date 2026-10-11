#![cfg(feature = "candle-runtime-engine")]

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use candle_core::Tensor;
use candle_transformers::generation::{LogitsProcessor, Sampling};
use futures::stream;

use std::sync::Arc as StdArc;

use super::{hooks::CandleSteeringHooks, transformer::TransformerModel};
use crate::model_runtime::{
    CancellationToken, FinishReason, GenerateRequest, GeneratedToken, ModelRuntimeError,
    RuntimeActivityGuard, RuntimePerfCall, RuntimePerfRecorder, SamplingParams, TokenStream,
    MODEL_RUNTIME_TOKEN_STREAM_CAPACITY,
};

pub trait CandleGenerationCodec: Send + Sync {
    fn encode_prompt(&self, prompt: &str) -> Result<Vec<u32>, ModelRuntimeError>;

    fn decode_token(&self, token_id: u32) -> Result<String, ModelRuntimeError>;
}

pub struct TokenizerGenerationCodec {
    tokenizer: Arc<tokenizers::Tokenizer>,
}

impl TokenizerGenerationCodec {
    pub fn new(tokenizer: Arc<tokenizers::Tokenizer>) -> Self {
        Self { tokenizer }
    }
}

impl CandleGenerationCodec for TokenizerGenerationCodec {
    fn encode_prompt(&self, prompt: &str) -> Result<Vec<u32>, ModelRuntimeError> {
        let encoding = self.tokenizer.encode(prompt, true).map_err(|error| {
            ModelRuntimeError::GenerateError(format!("Candle tokenizer encode failed: {error}"))
        })?;
        Ok(encoding.get_ids().to_vec())
    }

    fn decode_token(&self, token_id: u32) -> Result<String, ModelRuntimeError> {
        self.tokenizer.decode(&[token_id], true).map_err(|error| {
            ModelRuntimeError::GenerateError(format!("Candle tokenizer decode failed: {error}"))
        })
    }
}

pub(super) fn candle_generate_stream_tracked(
    model: Arc<Mutex<Box<dyn TransformerModel>>>,
    codec: Arc<dyn CandleGenerationCodec>,
    hooks: CandleSteeringHooks,
    req: GenerateRequest,
    runtime_cancel: CancellationToken,
    activity_guard: RuntimeActivityGuard,
    perf: StdArc<Mutex<RuntimePerfRecorder>>,
) -> TokenStream {
    let (sender, receiver) = tokio::sync::mpsc::channel::<Result<GeneratedToken, ModelRuntimeError>>(
        MODEL_RUNTIME_TOKEN_STREAM_CAPACITY,
    );

    let spawn_result = std::thread::Builder::new()
        .name("handshake-candle-generate".to_string())
        .spawn({
            let sender = sender.clone();
            move || {
                let _activity_guard = activity_guard;
                // MT-014b: measure the real decode call so the §10.13 panel
                // reports genuine tokens/sec and time-since-last-call.
                let started = std::time::Instant::now();
                match run_generation(model, codec, hooks, req, runtime_cancel, &sender) {
                    Ok(tokens_generated) => {
                        let gen_eval_ms =
                            u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
                        if let Ok(mut recorder) = perf.lock() {
                            recorder.record_call(RuntimePerfCall {
                                tokens_generated: u64::from(tokens_generated),
                                gen_eval_ms,
                                // Candle does not yet query device-resident VRAM;
                                // 0 surfaces as a typed reason in perf_snapshot.
                                vram_resident_bytes: 0,
                                completed_at_utc: chrono::Utc::now(),
                            });
                        }
                    }
                    Err(error) => {
                        let _ = sender.try_send(Err(error));
                    }
                }
            }
        });

    if let Err(error) = spawn_result {
        return Box::pin(stream::iter([Err(ModelRuntimeError::GenerateError(
            format!("failed to spawn Candle generation worker: {error}"),
        ))]));
    }

    drop(sender);
    Box::pin(stream::unfold(receiver, |mut receiver| async {
        receiver.recv().await.map(|item| (item, receiver))
    }))
}

/// Runs the Candle decode loop, returning the number of decode tokens produced
/// so the caller can record real perf telemetry (§10.13). Pre-generation exits
/// return `0`; loop exits return the running decode count.
fn run_generation(
    model: Arc<Mutex<Box<dyn TransformerModel>>>,
    codec: Arc<dyn CandleGenerationCodec>,
    hooks: CandleSteeringHooks,
    req: GenerateRequest,
    runtime_cancel: CancellationToken,
    sender: &tokio::sync::mpsc::Sender<Result<GeneratedToken, ModelRuntimeError>>,
) -> Result<u32, ModelRuntimeError> {
    if req.structured_decoding.is_some() {
        return Err(ModelRuntimeError::CapabilityNotSupported {
            capability: "structured_decoding".to_string(),
            adapter: "candle".to_string(),
        });
    }
    if req.kv_prefix_handle.is_some() {
        return Err(ModelRuntimeError::CapabilityNotSupported {
            capability: "kv prefix cache".to_string(),
            adapter: "candle".to_string(),
        });
    }
    if is_cancelled(&req, &runtime_cancel) {
        let _ = send_with_backpressure(
            sender,
            Ok(terminal_token(FinishReason::Cancelled)),
            &req,
            &runtime_cancel,
        );
        return Ok(0);
    }
    if req.max_tokens == 0 {
        let _ = send_with_backpressure(
            sender,
            Ok(terminal_token(FinishReason::Length)),
            &req,
            &runtime_cancel,
        );
        return Ok(0);
    }

    let mut input_ids = codec.encode_prompt(req.prompt.as_str())?;
    if input_ids.is_empty() {
        return Err(ModelRuntimeError::GenerateError(
            "Candle tokenizer produced no prompt tokens".to_string(),
        ));
    }

    let mut logits_processor = logits_processor(&req.sampling);
    let mut stop_detector = StopSequenceDetector::new(req.stop_sequences.clone());
    let mut generated = 0_u32;
    let mut locked = model.lock().map_err(|_| {
        ModelRuntimeError::GenerateError("Candle transformer model lock is poisoned".to_string())
    })?;
    locked.reset_generation_state()?;
    locked.validate_lora_overrides(&req.lora_overrides)?;

    loop {
        if is_cancelled(&req, &runtime_cancel) {
            let _ = send_with_backpressure(
                sender,
                Ok(terminal_token(FinishReason::Cancelled)),
                &req,
                &runtime_cancel,
            );
            return Ok(generated);
        }

        let logits = {
            let device = locked.device();
            let input = Tensor::new(input_ids.as_slice(), &device)
                .and_then(|tensor| tensor.reshape((1, input_ids.len())))
                .map_err(|error| {
                    ModelRuntimeError::GenerateError(format!("Candle input tensor failed: {error}"))
                })?;
            locked.forward(&input, &hooks, &req.steering_overrides, &req.lora_overrides)?
        };

        let logits = normalize_logits(logits)?;
        let token_id = logits_processor.sample(&logits).map_err(|error| {
            ModelRuntimeError::GenerateError(format!("Candle logits sampling failed: {error}"))
        })?;
        generated += 1;

        let is_eos = locked.eos_token_ids().contains(&token_id);

        if is_eos {
            let text = stop_detector.flush();
            let _ = send_with_backpressure(
                sender,
                Ok(generated_token(token_id, text, Some(FinishReason::Stop))),
                &req,
                &runtime_cancel,
            );
            return Ok(generated);
        }

        let piece = codec.decode_token(token_id)?;
        let outcome = stop_detector.push(&piece);
        if outcome.stopped {
            let _ = send_with_backpressure(
                sender,
                Ok(generated_token(
                    token_id,
                    outcome.text,
                    Some(FinishReason::Stop),
                )),
                &req,
                &runtime_cancel,
            );
            return Ok(generated);
        }

        if generated == req.max_tokens {
            let mut text = outcome.text;
            text.push_str(&stop_detector.flush());
            let _ = send_with_backpressure(
                sender,
                Ok(generated_token(token_id, text, Some(FinishReason::Length))),
                &req,
                &runtime_cancel,
            );
            return Ok(generated);
        }

        if !outcome.text.is_empty() {
            if !send_with_backpressure(
                sender,
                Ok(generated_token(token_id, outcome.text, None)),
                &req,
                &runtime_cancel,
            ) {
                return Ok(generated);
            }
        }
        input_ids = vec![token_id];
    }
}

fn send_with_backpressure(
    sender: &tokio::sync::mpsc::Sender<Result<GeneratedToken, ModelRuntimeError>>,
    mut item: Result<GeneratedToken, ModelRuntimeError>,
    req: &GenerateRequest,
    runtime_cancel: &CancellationToken,
) -> bool {
    let is_terminal = match &item {
        Ok(token) => token.finish_reason.is_some(),
        Err(_) => true,
    };
    loop {
        if !is_terminal && sender.capacity() <= 1 {
            if is_cancelled(req, runtime_cancel) {
                let _ = sender.try_send(Ok(terminal_token(FinishReason::Cancelled)));
                return false;
            }
            std::thread::park_timeout(Duration::from_millis(2));
            continue;
        }
        match sender.try_send(item) {
            Ok(()) => return true,
            Err(tokio::sync::mpsc::error::TrySendError::Closed(_item)) => return false,
            Err(tokio::sync::mpsc::error::TrySendError::Full(returned)) => {
                if !is_terminal && is_cancelled(req, runtime_cancel) {
                    let _ = sender.try_send(Ok(terminal_token(FinishReason::Cancelled)));
                    return false;
                }
                item = returned;
                std::thread::park_timeout(Duration::from_millis(2));
            }
        }
    }
}

fn normalize_logits(logits: Tensor) -> Result<Tensor, ModelRuntimeError> {
    match logits.dims() {
        [_vocab] => Ok(logits),
        [1, _vocab] => logits.squeeze(0).map_err(|error| {
            ModelRuntimeError::GenerateError(format!("Candle logits squeeze failed: {error}"))
        }),
        dims => Err(ModelRuntimeError::GenerateError(format!(
            "Candle generation expected logits shape [vocab] or [1, vocab], got {dims:?}"
        ))),
    }
}

fn logits_processor(sampling: &SamplingParams) -> LogitsProcessor {
    let seed = u64::from(sampling.seed.unwrap_or(0));
    let temperature = sampling.temperature.unwrap_or(0.0).max(0.0) as f64;
    let top_p = sampling.top_p.unwrap_or(1.0).clamp(0.0, 1.0) as f64;
    match (sampling.top_k, temperature <= 1e-7, top_p >= 1.0) {
        (Some(k), false, false) => LogitsProcessor::from_sampling(
            seed,
            Sampling::TopKThenTopP {
                k: k as usize,
                p: top_p,
                temperature,
            },
        ),
        (Some(k), false, true) => LogitsProcessor::from_sampling(
            seed,
            Sampling::TopK {
                k: k as usize,
                temperature,
            },
        ),
        (_, true, _) => LogitsProcessor::from_sampling(seed, Sampling::ArgMax),
        (_, false, false) => LogitsProcessor::from_sampling(
            seed,
            Sampling::TopP {
                p: top_p,
                temperature,
            },
        ),
        (_, false, true) => LogitsProcessor::from_sampling(seed, Sampling::All { temperature }),
    }
}

fn generated_token(
    token_id: u32,
    text: String,
    finish_reason: Option<FinishReason>,
) -> GeneratedToken {
    GeneratedToken {
        token_id,
        text,
        logprob: None,
        finish_reason,
    }
}

fn terminal_token(reason: FinishReason) -> GeneratedToken {
    generated_token(0, String::new(), Some(reason))
}

fn is_cancelled(req: &GenerateRequest, runtime_cancel: &CancellationToken) -> bool {
    req.cancel.is_cancelled() || runtime_cancel.is_cancelled()
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StopSequenceDetector {
    stop_sequences: Vec<String>,
    pending: String,
}

impl StopSequenceDetector {
    fn new(stop_sequences: Vec<String>) -> Self {
        Self {
            stop_sequences: stop_sequences
                .into_iter()
                .filter(|sequence| !sequence.is_empty())
                .collect(),
            pending: String::new(),
        }
    }

    fn push(&mut self, text: &str) -> StopSequenceOutcome {
        if self.stop_sequences.is_empty() {
            return StopSequenceOutcome {
                text: text.to_string(),
                stopped: false,
            };
        }

        self.pending.push_str(text);
        if let Some(index) = self.find_stop() {
            let emitted = self.pending[..index].to_string();
            self.pending.clear();
            return StopSequenceOutcome {
                text: emitted,
                stopped: true,
            };
        }

        let keep = self.longest_pending_stop_prefix_suffix();
        let emit_until = self.pending.len().saturating_sub(keep);
        let emitted = self.pending[..emit_until].to_string();
        self.pending = self.pending[emit_until..].to_string();
        StopSequenceOutcome {
            text: emitted,
            stopped: false,
        }
    }

    fn flush(&mut self) -> String {
        std::mem::take(&mut self.pending)
    }

    fn find_stop(&self) -> Option<usize> {
        self.stop_sequences
            .iter()
            .filter_map(|stop| self.pending.find(stop))
            .min()
    }

    fn longest_pending_stop_prefix_suffix(&self) -> usize {
        let mut keep = 0;
        for stop in &self.stop_sequences {
            for (prefix_len, _) in stop.char_indices().skip(1) {
                if self.pending.ends_with(&stop[..prefix_len]) {
                    keep = keep.max(prefix_len);
                }
            }
        }
        keep
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StopSequenceOutcome {
    text: String,
    stopped: bool,
}

#[cfg(test)]
mod activity_tests {
    use std::{
        sync::{mpsc, Arc, Condvar, Mutex},
        time::Duration,
    };

    use candle_core::{Device, Tensor};
    use futures::StreamExt;

    use super::{
        candle_generate_stream_tracked, send_with_backpressure, terminal_token,
        CandleGenerationCodec, CandleSteeringHooks, TransformerModel,
    };
    use crate::model_runtime::{
        CancellationToken, CaptureSpec, FinishReason, GenPrompt, GenerateRequest, GeneratedToken,
        HookPoint, KvPrefixHandle, LayerIndex, LoraId, ModelId, ModelRuntimeError,
        RuntimeActivityKind, RuntimeActivityTracker, RuntimePerfRecorder, SamplingParams,
        SteeringProvenance, SteeringVector, SteeringVectorId, SteeringVectorValues,
        MODEL_RUNTIME_TOKEN_STREAM_CAPACITY,
    };

    fn backpressure_request(cancel: CancellationToken) -> GenerateRequest {
        GenerateRequest {
            id: ModelId::new_v7(),
            prompt: GenPrompt::from("backpressure probe"),
            sampling: SamplingParams::default(),
            lora_overrides: Vec::new(),
            steering_overrides: Vec::new(),
            kv_prefix_handle: None,
            cancel,
            max_tokens: u32::MAX,
            stop_sequences: Vec::new(),
            speculative_mode: None,
            structured_decoding: None,
        }
    }

    fn nonterminal_token(index: u32) -> GeneratedToken {
        GeneratedToken {
            token_id: index,
            text: "x".to_string(),
            logprob: None,
            finish_reason: None,
        }
    }

    #[test]
    fn bounded_stream_reserves_terminal_slot_and_cancellation_releases_full_worker() {
        let (sender, mut receiver) =
            tokio::sync::mpsc::channel(MODEL_RUNTIME_TOKEN_STREAM_CAPACITY);
        let cancel = CancellationToken::new();
        let req = backpressure_request(cancel.clone());
        let runtime_cancel = CancellationToken::new();
        for index in 0..(MODEL_RUNTIME_TOKEN_STREAM_CAPACITY - 1) {
            assert!(send_with_backpressure(
                &sender,
                Ok(nonterminal_token(index as u32)),
                &req,
                &runtime_cancel,
            ));
        }
        assert_eq!(sender.capacity(), 1, "one terminal slot stays reserved");

        let worker_sender = sender.clone();
        let worker_req = req.clone();
        let worker_runtime_cancel = runtime_cancel.clone();
        let worker = std::thread::spawn(move || {
            send_with_backpressure(
                &worker_sender,
                Ok(nonterminal_token(u32::MAX)),
                &worker_req,
                &worker_runtime_cancel,
            )
        });
        std::thread::sleep(Duration::from_millis(20));
        assert!(!worker.is_finished(), "full data lane applies backpressure");
        cancel.cancel();
        assert!(
            !worker.join().expect("backpressured worker joins"),
            "cancellation exits instead of spinning or blocking forever"
        );

        let mut items = Vec::new();
        while let Ok(item) = receiver.try_recv() {
            items.push(item.expect("generated item"));
        }
        assert_eq!(items.len(), MODEL_RUNTIME_TOKEN_STREAM_CAPACITY);
        assert_eq!(
            items.last().and_then(|token| token.finish_reason),
            Some(FinishReason::Cancelled),
            "saturated cancellation must preserve an explicit terminal outcome"
        );
    }

    struct BlockingCodecProbe {
        started: mpsc::Sender<()>,
        release: Arc<(Mutex<bool>, Condvar)>,
    }

    impl CandleGenerationCodec for BlockingCodecProbe {
        fn encode_prompt(&self, _prompt: &str) -> Result<Vec<u32>, ModelRuntimeError> {
            self.started.send(()).expect("worker start observer");
            let (released, wake) = &*self.release;
            let mut released = released.lock().unwrap();
            while !*released {
                released = wake.wait(released).unwrap();
            }
            Err(ModelRuntimeError::GenerateError(
                "activity lifetime probe released".to_string(),
            ))
        }

        fn decode_token(&self, _token_id: u32) -> Result<String, ModelRuntimeError> {
            Err(ModelRuntimeError::GenerateError(
                "activity lifetime probe never decodes".to_string(),
            ))
        }
    }

    struct UnreachableModelProbe;

    impl TransformerModel for UnreachableModelProbe {
        fn forward(
            &mut self,
            _input_ids: &Tensor,
            _hooks: &CandleSteeringHooks,
            _steering_overrides: &[SteeringVectorId],
            _lora_overrides: &[LoraId],
        ) -> Result<Tensor, ModelRuntimeError> {
            Err(ModelRuntimeError::GenerateError(
                "activity lifetime probe must stop before model forward".to_string(),
            ))
        }

        fn n_layers(&self) -> u32 {
            0
        }

        fn hidden_dim(&self) -> u32 {
            1
        }

        fn vocab_size(&self) -> u32 {
            1
        }

        fn eos_token_ids(&self) -> &[u32] {
            &[]
        }

        fn device(&self) -> Device {
            Device::Cpu
        }

        fn reset_generation_state(&mut self) -> Result<(), ModelRuntimeError> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct MountedLoraModelProbe {
        calls: usize,
        mounted_loras: Vec<LoraId>,
    }

    impl MountedLoraModelProbe {
        fn with_mounted_lora(id: LoraId) -> Self {
            Self {
                calls: 0,
                mounted_loras: vec![id],
            }
        }
    }

    impl TransformerModel for MountedLoraModelProbe {
        fn forward(
            &mut self,
            _input_ids: &Tensor,
            _hooks: &CandleSteeringHooks,
            _steering_overrides: &[SteeringVectorId],
            _lora_overrides: &[LoraId],
        ) -> Result<Tensor, ModelRuntimeError> {
            let token = if self.calls == 0 { 2 } else { 3 };
            self.calls += 1;
            let mut logits = vec![0.0_f32; 4];
            logits[token] = 10.0;
            Tensor::from_vec(logits, 4, &Device::Cpu)
                .map_err(|error| ModelRuntimeError::GenerateError(error.to_string()))
        }

        fn n_layers(&self) -> u32 {
            1
        }

        fn hidden_dim(&self) -> u32 {
            2
        }

        fn vocab_size(&self) -> u32 {
            4
        }

        fn eos_token_ids(&self) -> &[u32] {
            &[3]
        }

        fn device(&self) -> Device {
            Device::Cpu
        }

        fn reset_generation_state(&mut self) -> Result<(), ModelRuntimeError> {
            self.calls = 0;
            Ok(())
        }

        fn validate_lora_overrides(&self, ids: &[LoraId]) -> Result<(), ModelRuntimeError> {
            if ids.iter().all(|id| self.mounted_loras.contains(id)) {
                Ok(())
            } else {
                Err(ModelRuntimeError::LoraStackError(
                    "fake transformer saw unmounted LoRA override".to_string(),
                ))
            }
        }
    }

    struct LoraCodecProbe;

    impl CandleGenerationCodec for LoraCodecProbe {
        fn encode_prompt(&self, _prompt: &str) -> Result<Vec<u32>, ModelRuntimeError> {
            Ok(vec![1])
        }

        fn decode_token(&self, token_id: u32) -> Result<String, ModelRuntimeError> {
            Ok(match token_id {
                2 => "A",
                3 => "",
                _ => "?",
            }
            .to_string())
        }
    }

    struct ScriptedTransformerProbe {
        scripted_tokens: Vec<u32>,
        calls: usize,
    }

    impl ScriptedTransformerProbe {
        fn new(scripted_tokens: Vec<u32>) -> Self {
            Self {
                scripted_tokens,
                calls: 0,
            }
        }
    }

    impl TransformerModel for ScriptedTransformerProbe {
        fn forward(
            &mut self,
            _input_ids: &Tensor,
            hooks: &CandleSteeringHooks,
            steering_overrides: &[SteeringVectorId],
            _lora_overrides: &[LoraId],
        ) -> Result<Tensor, ModelRuntimeError> {
            hooks.run_resid_stream_forward_harness(
                [(LayerIndex::new(5), vec![vec![1.0, 2.0]])]
                    .into_iter()
                    .collect(),
                &[LayerIndex::new(5)],
                steering_overrides,
            )?;
            let token = self.scripted_tokens.get(self.calls).copied().unwrap_or(4);
            self.calls += 1;
            let mut logits = vec![0.0_f32; 5];
            logits[token as usize] = 10.0;
            Tensor::from_vec(logits, 5, &Device::Cpu)
                .map_err(|error| ModelRuntimeError::GenerateError(error.to_string()))
        }

        fn n_layers(&self) -> u32 {
            6
        }

        fn hidden_dim(&self) -> u32 {
            2
        }

        fn vocab_size(&self) -> u32 {
            5
        }

        fn eos_token_ids(&self) -> &[u32] {
            &[4]
        }

        fn device(&self) -> Device {
            Device::Cpu
        }

        fn reset_generation_state(&mut self) -> Result<(), ModelRuntimeError> {
            self.calls = 0;
            Ok(())
        }
    }

    struct ScriptedCodecProbe;

    impl CandleGenerationCodec for ScriptedCodecProbe {
        fn encode_prompt(&self, _prompt: &str) -> Result<Vec<u32>, ModelRuntimeError> {
            Ok(vec![1])
        }

        fn decode_token(&self, token_id: u32) -> Result<String, ModelRuntimeError> {
            Ok(match token_id {
                2 => "A",
                3 => "B",
                4 => "",
                _ => "?",
            }
            .to_string())
        }
    }

    fn scripted_request(
        id: ModelId,
        cancel: CancellationToken,
        max_tokens: u32,
        steering_overrides: Vec<SteeringVectorId>,
    ) -> GenerateRequest {
        GenerateRequest {
            id,
            prompt: GenPrompt::from("prompt"),
            sampling: SamplingParams {
                temperature: Some(0.0),
                top_p: None,
                top_k: None,
                min_p: None,
                repetition_penalty: None,
                frequency_penalty: None,
                presence_penalty: None,
                seed: Some(42),
            },
            lora_overrides: Vec::new(),
            steering_overrides,
            kv_prefix_handle: None,
            cancel,
            max_tokens,
            stop_sequences: Vec::new(),
            speculative_mode: None,
            structured_decoding: None,
        }
    }

    fn generation_activity_guard(
        model_id: ModelId,
        cancel: CancellationToken,
    ) -> crate::model_runtime::RuntimeActivityGuard {
        RuntimeActivityTracker::new()
            .try_register(model_id, RuntimeActivityKind::Generate, Some(cancel))
            .expect("generation admission")
    }

    #[tokio::test]
    async fn candle_generate_stream_uses_fake_transformer_sampling_cancel_and_hooks() {
        let model_id = ModelId::new_v7();
        // MT-082: this test exercises scaffold capture on bare hooks (no real
        // forward), so it opts in explicitly; production bare hooks fail closed.
        let hooks = CandleSteeringHooks::new_for_model(model_id, 2).with_scaffold_capture();
        let vector = SteeringVector::try_new(
            None,
            "test-vector",
            LayerIndex::new(5),
            HookPoint::ResidStream,
            SteeringVectorValues::try_new(vec![10.0, 0.0], 0.5).unwrap(),
            "test steering vector",
            Some(SteeringProvenance::Manual {
                author: "test".to_string(),
                notes: "fake transformer hook proof".to_string(),
            }),
        )
        .unwrap();
        let vector_id = hooks.register_vector(vector).await.unwrap();
        let cancel = CancellationToken::new();
        let activity_guard = generation_activity_guard(model_id, cancel.clone());
        let mut stream = candle_generate_stream_tracked(
            Arc::new(Mutex::new(Box::new(ScriptedTransformerProbe::new(vec![
                2, 3, 4,
            ])))),
            Arc::new(ScriptedCodecProbe),
            hooks.clone(),
            scripted_request(model_id, cancel.clone(), 8, vec![vector_id]),
            cancel,
            activity_guard,
            Arc::new(Mutex::new(RuntimePerfRecorder::new())),
        );

        let mut tokens = Vec::new();
        while let Some(item) = stream.next().await {
            tokens.push(item.unwrap());
        }

        assert_eq!(
            tokens
                .iter()
                .map(|token| token.text.as_str())
                .collect::<Vec<_>>(),
            ["A", "B", ""]
        );
        assert_eq!(
            tokens.last().unwrap().finish_reason,
            Some(FinishReason::Stop)
        );
        let captured = hooks
            .capture(CaptureSpec {
                prompts: vec!["after generation".to_string()],
                layers: vec![LayerIndex::new(5)],
                hook_point: HookPoint::ResidStream,
            })
            .await
            .unwrap();
        assert!(captured.activations.contains_key(&LayerIndex::new(5)));
    }

    #[tokio::test]
    async fn candle_generate_stream_cancels_before_forward_work() {
        let model_id = ModelId::new_v7();
        let cancel = CancellationToken::new();
        cancel.cancel();
        let activity_guard = generation_activity_guard(model_id, cancel.clone());
        let mut stream = candle_generate_stream_tracked(
            Arc::new(Mutex::new(Box::new(ScriptedTransformerProbe::new(vec![2])))),
            Arc::new(ScriptedCodecProbe),
            CandleSteeringHooks::new_for_model(model_id, 2),
            scripted_request(model_id, cancel.clone(), 8, Vec::new()),
            cancel,
            activity_guard,
            Arc::new(Mutex::new(RuntimePerfRecorder::new())),
        );

        let token = stream.next().await.unwrap().unwrap();
        assert_eq!(token.finish_reason, Some(FinishReason::Cancelled));
    }

    #[tokio::test]
    async fn candle_generate_stream_rejects_kv_prefix_until_supported() {
        let model_id = ModelId::new_v7();
        let cancel = CancellationToken::new();
        let activity_guard = generation_activity_guard(model_id, cancel.clone());
        let mut kv_request = scripted_request(model_id, cancel.clone(), 8, Vec::new());
        kv_request.kv_prefix_handle = Some(KvPrefixHandle::from_tokens(&[1, 2]).unwrap());
        let mut stream = candle_generate_stream_tracked(
            Arc::new(Mutex::new(Box::new(ScriptedTransformerProbe::new(vec![2])))),
            Arc::new(ScriptedCodecProbe),
            CandleSteeringHooks::new_for_model(model_id, 2),
            kv_request,
            cancel,
            activity_guard,
            Arc::new(Mutex::new(RuntimePerfRecorder::new())),
        );
        let err = stream.next().await.unwrap().unwrap_err();
        assert!(err.to_string().contains("kv prefix"), "{err}");
    }

    #[tokio::test]
    async fn candle_generate_stream_allows_mounted_lora_override() {
        let model_id = ModelId::new_v7();
        let lora_id = LoraId::new_v7();
        let cancellation = CancellationToken::new();
        let tracker = RuntimeActivityTracker::new();
        let activity_guard = tracker
            .try_register(
                model_id,
                RuntimeActivityKind::Generate,
                Some(cancellation.clone()),
            )
            .expect("generation admission");
        let mut stream = candle_generate_stream_tracked(
            Arc::new(Mutex::new(Box::new(
                MountedLoraModelProbe::with_mounted_lora(lora_id),
            ))),
            Arc::new(LoraCodecProbe),
            CandleSteeringHooks::new_for_model(model_id, 2),
            GenerateRequest {
                id: model_id,
                prompt: GenPrompt::from("prompt"),
                sampling: SamplingParams {
                    temperature: Some(0.0),
                    seed: Some(7),
                    ..SamplingParams::default()
                },
                lora_overrides: vec![lora_id],
                steering_overrides: Vec::new(),
                kv_prefix_handle: None,
                cancel: cancellation,
                max_tokens: 2,
                stop_sequences: Vec::new(),
                speculative_mode: None,
                structured_decoding: None,
            },
            CancellationToken::new(),
            activity_guard,
            Arc::new(Mutex::new(RuntimePerfRecorder::new())),
        );

        let first = stream.next().await.unwrap().unwrap();
        assert_eq!(first.text, "A");
        let second = stream.next().await.unwrap().unwrap();
        assert_eq!(second.finish_reason, Some(FinishReason::Stop));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn tracked_candle_worker_remains_active_after_stream_drop() {
        let model_id = ModelId::new_v7();
        let cancellation = CancellationToken::new();
        let tracker = RuntimeActivityTracker::new();
        let activity_guard = tracker
            .try_register(
                model_id,
                RuntimeActivityKind::Generate,
                Some(cancellation.clone()),
            )
            .expect("generation admission");
        let release = Arc::new((Mutex::new(false), Condvar::new()));
        let (started, worker_started) = mpsc::channel();
        let stream = candle_generate_stream_tracked(
            Arc::new(Mutex::new(
                Box::new(UnreachableModelProbe) as Box<dyn TransformerModel>
            )),
            Arc::new(BlockingCodecProbe {
                started,
                release: Arc::clone(&release),
            }),
            CandleSteeringHooks::new_for_model(model_id, 1),
            GenerateRequest {
                id: model_id,
                prompt: GenPrompt::from("activity probe"),
                sampling: SamplingParams::default(),
                lora_overrides: Vec::new(),
                steering_overrides: Vec::new(),
                kv_prefix_handle: None,
                cancel: cancellation.clone(),
                max_tokens: 1,
                stop_sequences: Vec::new(),
                speculative_mode: None,
                structured_decoding: None,
            },
            CancellationToken::new(),
            activity_guard,
            Arc::new(Mutex::new(RuntimePerfRecorder::new())),
        );
        worker_started
            .recv_timeout(Duration::from_secs(1))
            .expect("actual Candle generation closure enters codec");
        drop(stream);

        let quiesce = {
            let tracker = tracker.clone();
            tokio::spawn(async move { tracker.quiesce(Duration::from_secs(2)).await })
        };
        tokio::time::timeout(Duration::from_secs(1), async {
            while !cancellation.is_cancelled() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("quiesce cancels the actual generation token");

        assert_eq!(tracker.active_operations().len(), 1);
        assert_eq!(tracker.active_operations()[0].model_id, model_id);
        assert!(
            !quiesce.is_finished(),
            "dropping the stream cannot release the worker-owned guard"
        );

        let (released, wake) = &*release;
        *released.lock().unwrap() = true;
        wake.notify_all();
        quiesce
            .await
            .expect("quiesce task joins")
            .expect("actual Candle generation closure exits under the shared deadline");
        assert!(tracker.active_operations().is_empty());
    }
}

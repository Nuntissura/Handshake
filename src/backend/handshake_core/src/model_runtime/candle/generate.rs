#![cfg(feature = "candle-runtime-engine")]

use std::sync::{Arc, Mutex};

use candle_core::Tensor;
use candle_transformers::generation::{LogitsProcessor, Sampling};
use futures::stream;

use super::{hooks::CandleSteeringHooks, transformer::TransformerModel};
use crate::model_runtime::{
    CancellationToken, FinishReason, GenerateRequest, GeneratedToken, ModelRuntimeError,
    SamplingParams, TokenStream,
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

pub fn candle_generate_stream(
    model: Arc<Mutex<Box<dyn TransformerModel>>>,
    codec: Arc<dyn CandleGenerationCodec>,
    hooks: CandleSteeringHooks,
    req: GenerateRequest,
    runtime_cancel: CancellationToken,
) -> TokenStream {
    let (sender, receiver) =
        tokio::sync::mpsc::unbounded_channel::<Result<GeneratedToken, ModelRuntimeError>>();

    let spawn_result = std::thread::Builder::new()
        .name("handshake-candle-generate".to_string())
        .spawn({
            let sender = sender.clone();
            move || {
                if let Err(error) =
                    run_generation(model, codec, hooks, req, runtime_cancel, &sender)
                {
                    let _ = sender.send(Err(error));
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

fn run_generation(
    model: Arc<Mutex<Box<dyn TransformerModel>>>,
    codec: Arc<dyn CandleGenerationCodec>,
    hooks: CandleSteeringHooks,
    req: GenerateRequest,
    runtime_cancel: CancellationToken,
    sender: &tokio::sync::mpsc::UnboundedSender<Result<GeneratedToken, ModelRuntimeError>>,
) -> Result<(), ModelRuntimeError> {
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
        let _ = sender.send(Ok(terminal_token(FinishReason::Cancelled)));
        return Ok(());
    }
    if req.max_tokens == 0 {
        let _ = sender.send(Ok(terminal_token(FinishReason::Length)));
        return Ok(());
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
            let _ = sender.send(Ok(terminal_token(FinishReason::Cancelled)));
            return Ok(());
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
            let _ = sender.send(Ok(generated_token(
                token_id,
                text,
                Some(FinishReason::Stop),
            )));
            return Ok(());
        }

        let piece = codec.decode_token(token_id)?;
        let outcome = stop_detector.push(&piece);
        if outcome.stopped {
            let _ = sender.send(Ok(generated_token(
                token_id,
                outcome.text,
                Some(FinishReason::Stop),
            )));
            return Ok(());
        }

        if generated == req.max_tokens {
            let mut text = outcome.text;
            text.push_str(&stop_detector.flush());
            let _ = sender.send(Ok(generated_token(
                token_id,
                text,
                Some(FinishReason::Length),
            )));
            return Ok(());
        }

        if !outcome.text.is_empty() {
            let _ = sender.send(Ok(generated_token(token_id, outcome.text, None)));
        }
        input_ids = vec![token_id];
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

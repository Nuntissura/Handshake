use futures::stream;

#[cfg(feature = "llama-cpp-runtime-engine")]
use crate::model_runtime::CancellationToken;

#[cfg(feature = "llama-cpp-runtime-engine")]
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Instant,
};

#[cfg(feature = "llama-cpp-runtime-engine")]
use crate::flight_recorder::{
    events_llm_infer::{
        infer_end_event, infer_start_event, infer_token_event, new_llm_infer_request_id,
        should_emit_token_event,
    },
    FlightRecorder, FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
};
#[cfg(feature = "llama-cpp-runtime-engine")]
use crate::model_runtime::SpeculativeMode;
use crate::model_runtime::{
    FinishReason, GenerateRequest, GeneratedToken, ModelRuntimeError, TokenStream,
};

#[cfg(feature = "llama-cpp-runtime-engine")]
use crate::model_runtime::KvCacheOps;

#[cfg(feature = "llama-cpp-runtime-engine")]
use super::perf_stats::{LlamaCppPerfStats, LlamaCppPerfStatsUpdate};
#[cfg(feature = "llama-cpp-runtime-engine")]
use super::sampler::sampler_plan;

pub use super::speculative::{
    LLAMA_CPP_EAGLE3_UNSUPPORTED, LLAMA_CPP_SPECULATIVE_DECODE_UNSUPPORTED,
};

pub const LLAMA_CPP_STRUCTURED_DECODING_UNSUPPORTED: &str =
    "llama_cpp_structured_decoding_not_implemented";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeneratePreflight {
    Ready,
    AlreadyCancelled,
    LengthCapped,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StopSequenceOutcome {
    pub text: String,
    pub stopped: bool,
    pub matched_stop: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StopSequenceDetector {
    stop_sequences: Vec<String>,
    pending: String,
}

impl StopSequenceDetector {
    pub fn new(stop_sequences: Vec<String>) -> Self {
        Self {
            stop_sequences: stop_sequences
                .into_iter()
                .filter(|sequence| !sequence.is_empty())
                .collect(),
            pending: String::new(),
        }
    }

    pub fn push(&mut self, text: &str) -> StopSequenceOutcome {
        if self.stop_sequences.is_empty() {
            return StopSequenceOutcome {
                text: text.to_string(),
                stopped: false,
                matched_stop: None,
            };
        }

        self.pending.push_str(text);
        if let Some((index, stop)) = self.find_stop() {
            let emitted = self.pending[..index].to_string();
            self.pending.clear();
            return StopSequenceOutcome {
                text: emitted,
                stopped: true,
                matched_stop: Some(stop),
            };
        }

        let keep = self.longest_pending_stop_prefix_suffix();
        let emit_until = self.pending.len().saturating_sub(keep);
        let emitted = self.pending[..emit_until].to_string();
        self.pending = self.pending[emit_until..].to_string();

        StopSequenceOutcome {
            text: emitted,
            stopped: false,
            matched_stop: None,
        }
    }

    pub fn flush(&mut self) -> String {
        std::mem::take(&mut self.pending)
    }

    fn find_stop(&self) -> Option<(usize, String)> {
        self.stop_sequences
            .iter()
            .filter_map(|stop| self.pending.find(stop).map(|index| (index, stop.clone())))
            .min_by_key(|(index, _)| *index)
    }

    fn longest_pending_stop_prefix_suffix(&self) -> usize {
        let mut keep = 0;
        for stop in &self.stop_sequences {
            for (prefix_len, _) in stop.char_indices().skip(1) {
                let prefix = &stop[..prefix_len];
                if self.pending.ends_with(prefix) {
                    keep = keep.max(prefix_len);
                }
            }
        }
        keep
    }
}

pub fn generation_preflight(req: &GenerateRequest) -> Result<GeneratePreflight, ModelRuntimeError> {
    if req.structured_decoding.is_some() {
        return Err(ModelRuntimeError::CapabilityNotSupported {
            capability: LLAMA_CPP_STRUCTURED_DECODING_UNSUPPORTED.to_string(),
            adapter: "llama_cpp".to_string(),
        });
    }

    super::speculative::validate_speculative_request(req)?;

    if req.cancel.is_cancelled() {
        return Ok(GeneratePreflight::AlreadyCancelled);
    }

    if req.max_tokens == 0 {
        return Ok(GeneratePreflight::LengthCapped);
    }

    Ok(GeneratePreflight::Ready)
}

pub fn terminal_token(reason: FinishReason) -> GeneratedToken {
    GeneratedToken {
        token_id: 0,
        text: String::new(),
        logprob: None,
        finish_reason: Some(reason),
    }
}

pub(crate) fn single_error_stream(error: ModelRuntimeError) -> TokenStream {
    Box::pin(stream::iter([Err(error)]))
}

pub(crate) fn single_token_stream(token: GeneratedToken) -> TokenStream {
    Box::pin(stream::iter([Ok(token)]))
}

#[cfg(feature = "llama-cpp-runtime-engine")]
pub(crate) fn cancellation_requested(
    req: &GenerateRequest,
    runtime_cancel: &CancellationToken,
) -> bool {
    req.cancel.is_cancelled() || runtime_cancel.is_cancelled()
}

#[cfg(feature = "llama-cpp-runtime-engine")]
pub(super) fn native_generate_stream(
    native: Arc<super::context::NativeLlamaCppBackend>,
    req: GenerateRequest,
    runtime_cancel: CancellationToken,
    kv_cache: Arc<super::kv_cache_impl::LlamaCppKvCache>,
    lora_stack: Arc<super::lora_impl::LlamaCppLoraStack>,
    draft_native: Option<Arc<super::context::NativeLlamaCppBackend>>,
    stats_sink: Arc<std::sync::Mutex<Option<super::speculative::SpeculativeStats>>>,
    current_generation_epoch: Arc<AtomicU64>,
    generation_epoch: u64,
    perf_stats: Arc<std::sync::Mutex<LlamaCppPerfStats>>,
    flight_recorder: Option<Arc<dyn FlightRecorder>>,
) -> TokenStream {
    let (sender, receiver) =
        tokio::sync::mpsc::unbounded_channel::<Result<GeneratedToken, ModelRuntimeError>>();
    let recorder_runtime = tokio::runtime::Handle::try_current().ok();

    let spawn_result = std::thread::Builder::new()
        .name("handshake-llama-cpp-generate".to_string())
        .spawn({
            let sender = sender.clone();
            move || {
                if let Err(error) = run_native_generation(
                    native,
                    req,
                    runtime_cancel,
                    kv_cache,
                    lora_stack,
                    draft_native,
                    stats_sink,
                    current_generation_epoch,
                    generation_epoch,
                    perf_stats,
                    flight_recorder,
                    recorder_runtime,
                    &sender,
                ) {
                    let _ = sender.send(Err(error));
                }
            }
        });

    if let Err(error) = spawn_result {
        return single_error_stream(ModelRuntimeError::GenerateError(format!(
            "failed to spawn llama.cpp generation worker: {error}"
        )));
    }

    drop(sender);
    Box::pin(stream::unfold(receiver, |mut receiver| async {
        receiver.recv().await.map(|item| (item, receiver))
    }))
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn run_native_generation(
    native: Arc<super::context::NativeLlamaCppBackend>,
    req: GenerateRequest,
    runtime_cancel: CancellationToken,
    kv_cache: Arc<super::kv_cache_impl::LlamaCppKvCache>,
    lora_stack: Arc<super::lora_impl::LlamaCppLoraStack>,
    draft_native: Option<Arc<super::context::NativeLlamaCppBackend>>,
    stats_sink: Arc<std::sync::Mutex<Option<super::speculative::SpeculativeStats>>>,
    current_generation_epoch: Arc<AtomicU64>,
    generation_epoch: u64,
    perf_stats: Arc<std::sync::Mutex<LlamaCppPerfStats>>,
    flight_recorder: Option<Arc<dyn FlightRecorder>>,
    recorder_runtime: Option<tokio::runtime::Handle>,
    sender: &tokio::sync::mpsc::UnboundedSender<Result<GeneratedToken, ModelRuntimeError>>,
) -> Result<(), ModelRuntimeError> {
    use llama_cpp_2::{llama_batch::LlamaBatch, model::AddBos};

    if cancellation_requested(&req, &runtime_cancel) {
        let _ = sender.send(Ok(terminal_token(FinishReason::Cancelled)));
        return Ok(());
    }

    lora_stack.validate_request(&req.lora_overrides, req.kv_prefix_handle.is_some())?;

    let prompt_tokens = native
        .model
        .str_to_token(req.prompt.as_str(), AddBos::Always)
        .map_err(|error| {
            ModelRuntimeError::GenerateError(format!(
                "llama.cpp prompt tokenization failed: {error}"
            ))
        })?;
    if prompt_tokens.is_empty() {
        return Err(ModelRuntimeError::GenerateError(
            "llama.cpp prompt tokenization produced no tokens".to_string(),
        ));
    }

    let prefix_token_count = if let Some(handle) = req.kv_prefix_handle.as_ref() {
        kv_cache.validate_prompt_prefix(handle, &prompt_tokens)?
    } else {
        0
    };

    let _applied_lora;
    let mut context = native.new_context(kv_cache.quantization())?;
    _applied_lora = lora_stack.apply_to_context(
        &context,
        &req.lora_overrides,
        req.kv_prefix_handle.is_some(),
    )?;
    if let Some(handle) = req.kv_prefix_handle.as_ref() {
        kv_cache.restore_into_context(handle, &mut context)?;
    }

    let required_context = prompt_tokens
        .len()
        .saturating_add(usize::try_from(req.max_tokens).unwrap_or(usize::MAX));
    if required_context > context.n_ctx() as usize {
        return Err(ModelRuntimeError::GenerateError(format!(
            "llama.cpp context too small: requires {required_context} tokens, n_ctx is {}",
            context.n_ctx()
        )));
    }

    let request_id = new_llm_infer_request_id();
    let generation_started = Instant::now();
    let suffix_tokens = &prompt_tokens[prefix_token_count..];
    if suffix_tokens.is_empty() {
        return Err(ModelRuntimeError::GenerateError(
            "llama.cpp KV prefix handle covers the full prompt; include at least one suffix token"
                .to_string(),
        ));
    }

    context.reset_timings();
    record_llm_infer_event(
        flight_recorder.as_ref(),
        recorder_runtime.as_ref(),
        infer_start_event(
            req.id,
            request_id,
            u64::try_from(prompt_tokens.len()).unwrap_or(u64::MAX),
            req.prompt.as_str(),
            "llama_cpp",
        ),
    );

    let mut batch = LlamaBatch::new(suffix_tokens.len(), 1);
    let last_prompt_index =
        i32::try_from(prompt_tokens.len().saturating_sub(1)).map_err(|error| {
            ModelRuntimeError::GenerateError(format!(
                "prompt token count does not fit i32: {error}"
            ))
        })?;
    for (offset, token) in suffix_tokens.iter().copied().enumerate() {
        let absolute_position = prefix_token_count.saturating_add(offset);
        let position = i32::try_from(absolute_position).map_err(|error| {
            ModelRuntimeError::GenerateError(format!("prompt position does not fit i32: {error}"))
        })?;
        batch
            .add(token, position, &[0], position == last_prompt_index)
            .map_err(|error| {
                ModelRuntimeError::GenerateError(format!(
                    "failed to add prompt token to llama.cpp batch: {error}"
                ))
            })?;
    }

    context.decode(&mut batch).map_err(|error| {
        ModelRuntimeError::GenerateError(format!("llama.cpp prompt decode failed: {error}"))
    })?;

    let mut last_sample_index = batch.n_tokens().saturating_sub(1);
    let current_position_start = i32::try_from(prompt_tokens.len()).map_err(|error| {
        ModelRuntimeError::GenerateError(format!("prompt token count does not fit i32: {error}"))
    })?;

    let mut sampler = sampler_plan(&req.sampling).build_llama_sampler();
    sampler.accept_many(&prompt_tokens);
    let speculative_plan = super::speculative::speculative_plan(&req)?;
    let mut speculative_decoder = match super::speculative::prepare_speculative_decoder(
        speculative_plan,
        native.as_ref(),
        draft_native.as_deref(),
        &req,
        &prompt_tokens,
        kv_cache.quantization(),
    ) {
        Ok(decoder) => decoder,
        Err(error) => {
            record_speculative_stats(
                &stats_sink,
                current_generation_epoch.as_ref(),
                generation_epoch,
                super::speculative::SpeculativeStats::default(),
            )?;
            return Err(error);
        }
    };

    let mut stop_detector = StopSequenceDetector::new(req.stop_sequences.clone());
    let mut current_position = current_position_start;
    let mut generated = 0_u32;
    let generation_trace_id = uuid::Uuid::now_v7();
    let mut speculative_round_index = 0_u64;

    while generated < req.max_tokens {
        if cancellation_requested(&req, &runtime_cancel) {
            complete_generation(
                &req,
                request_id,
                &mut context,
                prompt_tokens.len(),
                generated,
                generation_started,
                FinishReason::Cancelled,
                &perf_stats,
                flight_recorder.as_ref(),
                recorder_runtime.as_ref(),
            )?;
            let _ = sender.send(Ok(terminal_token(FinishReason::Cancelled)));
            record_speculative_stats(
                &stats_sink,
                current_generation_epoch.as_ref(),
                generation_epoch,
                speculative_decoder.stats(),
            )?;
            return Ok(());
        }

        let remaining =
            usize::try_from(req.max_tokens.saturating_sub(generated)).map_err(|error| {
                ModelRuntimeError::GenerateError(format!(
                    "llama.cpp remaining token count does not fit usize: {error}"
                ))
            })?;
        let round = match speculative_decoder.sample_verified_round(
            &mut context,
            &mut sampler,
            last_sample_index,
            current_position,
            remaining,
            &req.cancel,
            &runtime_cancel,
        ) {
            Ok(Some(round)) => round,
            Ok(None) => {
                complete_generation(
                    &req,
                    request_id,
                    &mut context,
                    prompt_tokens.len(),
                    generated,
                    generation_started,
                    FinishReason::Cancelled,
                    &perf_stats,
                    flight_recorder.as_ref(),
                    recorder_runtime.as_ref(),
                )?;
                let _ = sender.send(Ok(terminal_token(FinishReason::Cancelled)));
                record_speculative_stats(
                    &stats_sink,
                    current_generation_epoch.as_ref(),
                    generation_epoch,
                    speculative_decoder.stats(),
                )?;
                return Ok(());
            }
            Err(error) => {
                record_speculative_stats(
                    &stats_sink,
                    current_generation_epoch.as_ref(),
                    generation_epoch,
                    speculative_decoder.stats(),
                )?;
                return Err(error);
            }
        };
        if cancellation_requested(&req, &runtime_cancel) {
            super::speculative::clear_round_decoded_suffix(&mut context, current_position, &round)?;
            complete_generation(
                &req,
                request_id,
                &mut context,
                prompt_tokens.len(),
                generated,
                generation_started,
                FinishReason::Cancelled,
                &perf_stats,
                flight_recorder.as_ref(),
                recorder_runtime.as_ref(),
            )?;
            let _ = sender.send(Ok(terminal_token(FinishReason::Cancelled)));
            record_speculative_stats(
                &stats_sink,
                current_generation_epoch.as_ref(),
                generation_epoch,
                speculative_decoder.stats(),
            )?;
            return Ok(());
        }
        if round.tokens.is_empty() {
            return Err(ModelRuntimeError::GenerateError(
                "llama.cpp speculative verifier returned an empty round".to_string(),
            ));
        }
        record_speculative_stats(
            &stats_sink,
            current_generation_epoch.as_ref(),
            generation_epoch,
            speculative_decoder.stats(),
        )?;
        if round.proposed_token_count() > 0 {
            speculative_round_index = speculative_round_index.saturating_add(1);
            emit_speculative_round_events(
                &req,
                &round,
                generation_trace_id,
                speculative_round_index,
                flight_recorder.as_ref(),
                recorder_runtime.as_ref(),
            );
        }

        let round_last_sample_index = round.last_sample_index;
        let round_final_already_decoded = round
            .tokens
            .last()
            .map(|item| item.target_decode == super::speculative::TargetDecodeState::AlreadyDecoded)
            .unwrap_or(false);

        for item in round.tokens {
            generated += 1;
            let token_id = u32::try_from(item.token.0).map_err(|error| {
                ModelRuntimeError::GenerateError(format!(
                    "llama.cpp token id does not fit u32: {error}"
                ))
            })?;

            let finish_reason = emit_generated_token(
                &native.model,
                item.token,
                generated,
                req.max_tokens,
                &mut stop_detector,
                sender,
            )?;
            if should_emit_token_event(generated) {
                record_llm_infer_event(
                    flight_recorder.as_ref(),
                    recorder_runtime.as_ref(),
                    infer_token_event(
                        req.id,
                        request_id,
                        generated,
                        token_id,
                        "",
                        elapsed_millis(generation_started),
                        "llama_cpp",
                    ),
                );
            }

            match item.target_decode {
                super::speculative::TargetDecodeState::AlreadyDecoded => {
                    current_position += 1;
                }
                super::speculative::TargetDecodeState::Required => {
                    if let Some(reason) = finish_reason {
                        complete_generation(
                            &req,
                            request_id,
                            &mut context,
                            prompt_tokens.len(),
                            generated,
                            generation_started,
                            reason,
                            &perf_stats,
                            flight_recorder.as_ref(),
                            recorder_runtime.as_ref(),
                        )?;
                        record_speculative_stats(
                            &stats_sink,
                            current_generation_epoch.as_ref(),
                            generation_epoch,
                            speculative_decoder.stats(),
                        )?;
                        return Ok(());
                    }
                    if cancellation_requested(&req, &runtime_cancel) {
                        complete_generation(
                            &req,
                            request_id,
                            &mut context,
                            prompt_tokens.len(),
                            generated,
                            generation_started,
                            FinishReason::Cancelled,
                            &perf_stats,
                            flight_recorder.as_ref(),
                            recorder_runtime.as_ref(),
                        )?;
                        let _ = sender.send(Ok(terminal_token(FinishReason::Cancelled)));
                        record_speculative_stats(
                            &stats_sink,
                            current_generation_epoch.as_ref(),
                            generation_epoch,
                            speculative_decoder.stats(),
                        )?;
                        return Ok(());
                    }
                    batch.clear();
                    batch
                        .add(item.token, current_position, &[0], true)
                        .map_err(|error| {
                            ModelRuntimeError::GenerateError(format!(
                                "failed to add generated token to llama.cpp batch: {error}"
                            ))
                        })?;
                    current_position += 1;
                    context.decode(&mut batch).map_err(|error| {
                        ModelRuntimeError::GenerateError(format!(
                            "llama.cpp token decode failed: {error}"
                        ))
                    })?;
                    last_sample_index = batch.n_tokens().saturating_sub(1);
                }
            }

            if let Some(reason) = finish_reason {
                complete_generation(
                    &req,
                    request_id,
                    &mut context,
                    prompt_tokens.len(),
                    generated,
                    generation_started,
                    reason,
                    &perf_stats,
                    flight_recorder.as_ref(),
                    recorder_runtime.as_ref(),
                )?;
                record_speculative_stats(
                    &stats_sink,
                    current_generation_epoch.as_ref(),
                    generation_epoch,
                    speculative_decoder.stats(),
                )?;
                return Ok(());
            }

            if cancellation_requested(&req, &runtime_cancel) {
                complete_generation(
                    &req,
                    request_id,
                    &mut context,
                    prompt_tokens.len(),
                    generated,
                    generation_started,
                    FinishReason::Cancelled,
                    &perf_stats,
                    flight_recorder.as_ref(),
                    recorder_runtime.as_ref(),
                )?;
                let _ = sender.send(Ok(terminal_token(FinishReason::Cancelled)));
                record_speculative_stats(
                    &stats_sink,
                    current_generation_epoch.as_ref(),
                    generation_epoch,
                    speculative_decoder.stats(),
                )?;
                return Ok(());
            }
        }

        if round_final_already_decoded {
            last_sample_index = round_last_sample_index;
        }
    }

    complete_generation(
        &req,
        request_id,
        &mut context,
        prompt_tokens.len(),
        generated,
        generation_started,
        FinishReason::Length,
        &perf_stats,
        flight_recorder.as_ref(),
        recorder_runtime.as_ref(),
    )?;
    record_speculative_stats(
        &stats_sink,
        current_generation_epoch.as_ref(),
        generation_epoch,
        speculative_decoder.stats(),
    )?;
    let _ = sender.send(Ok(terminal_token(FinishReason::Length)));
    Ok(())
}

#[cfg(feature = "llama-cpp-runtime-engine")]
#[allow(clippy::too_many_arguments)]
fn complete_generation(
    req: &GenerateRequest,
    request_id: uuid::Uuid,
    context: &mut llama_cpp_2::context::LlamaContext<'_>,
    prompt_token_count: usize,
    tokens_generated: u32,
    started: Instant,
    finish_reason: FinishReason,
    perf_stats: &std::sync::Mutex<LlamaCppPerfStats>,
    flight_recorder: Option<&Arc<dyn FlightRecorder>>,
    runtime: Option<&tokio::runtime::Handle>,
) -> Result<(), ModelRuntimeError> {
    let timings = context.timings();
    let prompt_eval_ms = timing_ms(timings.t_p_eval_ms(), prompt_token_count > 0);
    let gen_eval_ms = timing_ms(timings.t_eval_ms(), tokens_generated > 0);
    let total_ms = elapsed_millis(started);

    {
        let mut guard = perf_stats.lock().map_err(|error| {
            ModelRuntimeError::GenerateError(format!("llama.cpp perf stats lock poisoned: {error}"))
        })?;
        guard.record_call(LlamaCppPerfStatsUpdate {
            prompt_eval_ms,
            gen_eval_ms,
            tokens_generated: u64::from(tokens_generated),
            vram_resident_bytes: 0,
            completed_at_utc: chrono::Utc::now(),
        });
    }

    record_llm_infer_event(
        flight_recorder,
        runtime,
        infer_end_event(
            req.id,
            request_id,
            u64::try_from(prompt_token_count).unwrap_or(u64::MAX),
            tokens_generated,
            total_ms,
            prompt_eval_ms,
            gen_eval_ms,
            finish_reason,
            "llama_cpp",
        ),
    );
    Ok(())
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn timing_ms(raw_ms: f64, has_work: bool) -> u64 {
    if !raw_ms.is_finite() || raw_ms <= 0.0 {
        return if has_work { 1 } else { 0 };
    }
    let rounded = raw_ms.round().max(1.0);
    if rounded >= u64::MAX as f64 {
        u64::MAX
    } else {
        rounded as u64
    }
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn elapsed_millis(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn record_llm_infer_event(
    flight_recorder: Option<&Arc<dyn FlightRecorder>>,
    runtime: Option<&tokio::runtime::Handle>,
    event: FlightRecorderEvent,
) {
    let Some(recorder) = flight_recorder.cloned() else {
        return;
    };
    let Some(runtime) = runtime.cloned() else {
        return;
    };

    runtime.spawn(async move {
        let _ = recorder.record_event(event).await;
    });
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn emit_speculative_round_events(
    req: &GenerateRequest,
    round: &super::speculative::SpeculativeRound,
    trace_id: uuid::Uuid,
    round_index: u64,
    flight_recorder: Option<&Arc<dyn FlightRecorder>>,
    runtime: Option<&tokio::runtime::Handle>,
) {
    let Some(recorder) = flight_recorder.cloned() else {
        return;
    };
    let Some(runtime) = runtime.cloned() else {
        return;
    };
    let Some((mode, draft_model_id)) = speculative_event_mode(req) else {
        return;
    };

    let accepted_tokens = round.accepted_token_count();
    let rejected_tokens = round.rejected_token_count();
    if accepted_tokens > 0 {
        spawn_speculative_event_record(
            recorder.clone(),
            runtime.clone(),
            req.id.to_string(),
            draft_model_id.clone(),
            trace_id,
            round_index,
            mode,
            FlightRecorderEventType::LlmInferenceSpecAccept,
            "FR-EVT-LLM-INFER-SPEC-ACCEPT",
            "llm_infer.spec_accept",
            accepted_tokens,
            0,
        );
    }
    if rejected_tokens > 0 {
        spawn_speculative_event_record(
            recorder,
            runtime,
            req.id.to_string(),
            draft_model_id,
            trace_id,
            round_index,
            mode,
            FlightRecorderEventType::LlmInferenceSpecReject,
            "FR-EVT-LLM-INFER-SPEC-REJECT",
            "llm_infer.spec_reject",
            0,
            rejected_tokens,
        );
    }
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn speculative_event_mode(req: &GenerateRequest) -> Option<(&'static str, Option<String>)> {
    match req.speculative_mode.as_ref()? {
        SpeculativeMode::Ngram { .. } => Some(("ngram", None)),
        SpeculativeMode::DraftModel { draft_id, .. } => {
            Some(("draft_model", Some(draft_id.to_string())))
        }
        SpeculativeMode::Eagle3 { .. } => None,
    }
}

#[cfg(feature = "llama-cpp-runtime-engine")]
#[allow(clippy::too_many_arguments)]
fn spawn_speculative_event_record(
    recorder: Arc<dyn FlightRecorder>,
    runtime: tokio::runtime::Handle,
    model_id: String,
    draft_model_id: Option<String>,
    trace_id: uuid::Uuid,
    round_index: u64,
    mode: &'static str,
    event_type: FlightRecorderEventType,
    event_id: &'static str,
    payload_type: &'static str,
    accepted_tokens: usize,
    rejected_tokens: usize,
) {
    let accepted_tokens = u64::try_from(accepted_tokens).unwrap_or(u64::MAX);
    let rejected_tokens = u64::try_from(rejected_tokens).unwrap_or(u64::MAX);
    let generated_tokens = accepted_tokens.saturating_add(rejected_tokens);
    let mut event = FlightRecorderEvent::new(
        event_type,
        FlightRecorderActor::System,
        trace_id,
        serde_json::json!({
            "schema_version": "hsk.fr.llm_infer_spec@0.1",
            "event_id": event_id,
            "type": payload_type,
            "trace_id": trace_id.to_string(),
            "model_id": model_id.clone(),
            "draft_model_id": draft_model_id.clone(),
            "adapter": "llama_cpp",
            "mode": mode,
            "round_index": round_index,
            "accepted_tokens": accepted_tokens,
            "rejected_tokens": rejected_tokens,
            "generated_tokens": generated_tokens,
            "draft_calls": 1_u64
        }),
    );
    event.model_id = Some(model_id);

    runtime.spawn(async move {
        let _ = recorder.record_event(event).await;
    });
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn emit_generated_token(
    model: &llama_cpp_2::model::LlamaModel,
    token: llama_cpp_2::token::LlamaToken,
    generated: u32,
    max_tokens: u32,
    stop_detector: &mut StopSequenceDetector,
    sender: &tokio::sync::mpsc::UnboundedSender<Result<GeneratedToken, ModelRuntimeError>>,
) -> Result<Option<FinishReason>, ModelRuntimeError> {
    if model.is_eog_token(token) {
        let text = stop_detector.flush();
        let _ = sender.send(Ok(generated_token(token, text, Some(FinishReason::Stop))?));
        return Ok(Some(FinishReason::Stop));
    }

    let piece = token_to_string_lossy(model, token)?;
    let outcome = stop_detector.push(&piece);
    if outcome.stopped {
        let _ = sender.send(Ok(generated_token(
            token,
            outcome.text,
            Some(FinishReason::Stop),
        )?));
        return Ok(Some(FinishReason::Stop));
    }

    if generated == max_tokens {
        let mut text = outcome.text;
        text.push_str(&stop_detector.flush());
        let _ = sender.send(Ok(generated_token(
            token,
            text,
            Some(FinishReason::Length),
        )?));
        return Ok(Some(FinishReason::Length));
    }

    if !outcome.text.is_empty() {
        let _ = sender.send(Ok(generated_token(token, outcome.text, None)?));
    }

    Ok(None)
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn generated_token(
    token: llama_cpp_2::token::LlamaToken,
    text: String,
    finish_reason: Option<FinishReason>,
) -> Result<GeneratedToken, ModelRuntimeError> {
    let token_id = u32::try_from(token.0).map_err(|error| {
        ModelRuntimeError::GenerateError(format!("llama.cpp token id does not fit u32: {error}"))
    })?;
    Ok(GeneratedToken {
        token_id,
        text,
        logprob: None,
        finish_reason,
    })
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn token_to_string_lossy(
    model: &llama_cpp_2::model::LlamaModel,
    token: llama_cpp_2::token::LlamaToken,
) -> Result<String, ModelRuntimeError> {
    match model.token_to_piece_bytes(token, 32, true, None) {
        Ok(bytes) => Ok(String::from_utf8_lossy(&bytes).into_owned()),
        Err(llama_cpp_2::TokenToStringError::InsufficientBufferSpace(size)) if size < 0 => {
            let size = usize::try_from(-size).map_err(|error| {
                ModelRuntimeError::GenerateError(format!(
                    "llama.cpp token piece size does not fit usize: {error}"
                ))
            })?;
            let bytes = model
                .token_to_piece_bytes(token, size, true, None)
                .map_err(|error| {
                    ModelRuntimeError::GenerateError(format!(
                        "llama.cpp token decoding failed: {error}"
                    ))
                })?;
            Ok(String::from_utf8_lossy(&bytes).into_owned())
        }
        Err(error) => Err(ModelRuntimeError::GenerateError(format!(
            "llama.cpp token decoding failed: {error}"
        ))),
    }
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn record_speculative_stats(
    stats_sink: &std::sync::Mutex<Option<super::speculative::SpeculativeStats>>,
    current_generation_epoch: &AtomicU64,
    generation_epoch: u64,
    stats: super::speculative::SpeculativeStats,
) -> Result<(), ModelRuntimeError> {
    if current_generation_epoch.load(Ordering::SeqCst) != generation_epoch {
        return Ok(());
    }

    let mut guard = stats_sink.lock().map_err(|error| {
        ModelRuntimeError::GenerateError(format!(
            "llama.cpp speculative stats lock poisoned: {error}"
        ))
    })?;
    if current_generation_epoch.load(Ordering::SeqCst) == generation_epoch {
        *guard = Some(stats);
    }
    Ok(())
}

#[cfg(all(test, feature = "llama-cpp-runtime-engine"))]
mod tests {
    use super::super::speculative::SpeculativeStats;
    use super::*;

    #[test]
    fn speculative_stats_ignore_stale_generation_epoch() {
        let stats_sink = std::sync::Mutex::new(Some(SpeculativeStats {
            draft_calls: 9,
            generated_tokens: 9,
            ..SpeculativeStats::default()
        }));
        let current_generation_epoch = AtomicU64::new(2);

        record_speculative_stats(
            &stats_sink,
            &current_generation_epoch,
            1,
            SpeculativeStats::default(),
        )
        .expect("stale stats write is ignored without error");
        assert_eq!(
            stats_sink.lock().expect("stats lock").unwrap().draft_calls,
            9
        );

        record_speculative_stats(
            &stats_sink,
            &current_generation_epoch,
            2,
            SpeculativeStats {
                draft_calls: 4,
                generated_tokens: 4,
                ..SpeculativeStats::default()
            },
        )
        .expect("current epoch stats write succeeds");
        assert_eq!(
            stats_sink.lock().expect("stats lock").unwrap().draft_calls,
            4
        );

        current_generation_epoch.store(3, Ordering::SeqCst);
        record_speculative_stats(
            &stats_sink,
            &current_generation_epoch,
            2,
            SpeculativeStats::default(),
        )
        .expect("stale post-lock stats write is ignored without error");
        assert_eq!(
            stats_sink.lock().expect("stats lock").unwrap().draft_calls,
            4
        );
    }
}

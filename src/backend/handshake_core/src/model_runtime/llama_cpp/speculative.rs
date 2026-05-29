#[cfg(feature = "llama-cpp-runtime-engine")]
use super::sampler::sampler_plan;
use crate::model_runtime::{GenerateRequest, ModelRuntimeError, SpeculativeMode};

#[cfg(feature = "llama-cpp-runtime-engine")]
use crate::model_runtime::CancellationToken;

pub const LLAMA_CPP_SPECULATIVE_DECODE_UNSUPPORTED: &str =
    "llama_cpp_speculative_decode_not_implemented";
pub const LLAMA_CPP_EAGLE3_UNSUPPORTED: &str = "llama_cpp_eagle3_not_supported";
const MAX_DRAFT_LIMIT: u32 = 256;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SpeculativeStats {
    pub draft_calls: u64,
    pub generated_tokens: u64,
    pub accepted_tokens: u64,
    pub rejected_tokens: u64,
    pub accepted_drafts: u64,
    pub rejected_drafts: u64,
}

impl SpeculativeStats {
    #[cfg(feature = "llama-cpp-runtime-engine")]
    fn record_draft(&mut self, proposed: usize) {
        self.draft_calls = self.draft_calls.saturating_add(1);
        self.generated_tokens = self.generated_tokens.saturating_add(proposed as u64);
    }

    #[cfg(feature = "llama-cpp-runtime-engine")]
    fn record_verification(&mut self, proposed: usize, accepted: usize) {
        self.accepted_tokens = self.accepted_tokens.saturating_add(accepted as u64);
        self.accepted_drafts = self.accepted_drafts.saturating_add(accepted as u64);
        let rejected = proposed.saturating_sub(accepted);
        self.rejected_tokens = self.rejected_tokens.saturating_add(rejected as u64);
        self.rejected_drafts = self.rejected_drafts.saturating_add(rejected as u64);
    }
}

pub fn validate_speculative_request(req: &GenerateRequest) -> Result<(), ModelRuntimeError> {
    match req.speculative_mode.as_ref() {
        None => Ok(()),
        Some(SpeculativeMode::Eagle3 { .. }) => Err(ModelRuntimeError::CapabilityNotSupported {
            capability: LLAMA_CPP_EAGLE3_UNSUPPORTED.to_string(),
            adapter: "llama_cpp".to_string(),
        }),
        Some(SpeculativeMode::Ngram {
            lookback,
            max_draft,
        }) => validate_ngram_params(*lookback, *max_draft),
        Some(SpeculativeMode::DraftModel {
            draft_id,
            max_draft,
        }) => validate_draft_model_params(req, *draft_id, *max_draft),
    }
}

fn validate_ngram_params(lookback: u32, max_draft: u32) -> Result<(), ModelRuntimeError> {
    if lookback == 0 {
        return Err(ModelRuntimeError::GenerateError(
            "llama.cpp ngram speculation lookback must be greater than zero".to_string(),
        ));
    }

    validate_max_draft("llama.cpp ngram speculation", max_draft)
}

fn validate_draft_model_params(
    req: &GenerateRequest,
    draft_id: crate::model_runtime::ModelId,
    max_draft: u32,
) -> Result<(), ModelRuntimeError> {
    if draft_id == req.id {
        return Err(ModelRuntimeError::GenerateError(
            "llama.cpp draft model must differ from the target model".to_string(),
        ));
    }

    validate_max_draft("llama.cpp draft-model speculation", max_draft)
}

fn validate_max_draft(context: &str, max_draft: u32) -> Result<(), ModelRuntimeError> {
    if max_draft == 0 {
        return Err(ModelRuntimeError::GenerateError(format!(
            "{context} max_draft must be greater than zero"
        )));
    }

    if max_draft > MAX_DRAFT_LIMIT {
        return Err(ModelRuntimeError::GenerateError(format!(
            "{context} max_draft must be at most {MAX_DRAFT_LIMIT}"
        )));
    }

    Ok(())
}

#[cfg(feature = "llama-cpp-runtime-engine")]
#[derive(Clone, Debug)]
pub(crate) enum SpeculativePlan {
    None,
    Ngram { lookback: usize, max_draft: usize },
    DraftModel { max_draft: usize },
}

#[cfg(feature = "llama-cpp-runtime-engine")]
pub(crate) fn speculative_plan(
    req: &GenerateRequest,
) -> Result<SpeculativePlan, ModelRuntimeError> {
    validate_speculative_request(req)?;

    match req.speculative_mode.as_ref() {
        None => Ok(SpeculativePlan::None),
        Some(SpeculativeMode::Ngram {
            lookback,
            max_draft,
        }) => Ok(SpeculativePlan::Ngram {
            lookback: usize::try_from(*lookback).map_err(|error| {
                ModelRuntimeError::GenerateError(format!(
                    "llama.cpp ngram lookback does not fit usize: {error}"
                ))
            })?,
            max_draft: usize::try_from(*max_draft).map_err(|error| {
                ModelRuntimeError::GenerateError(format!(
                    "llama.cpp ngram max_draft does not fit usize: {error}"
                ))
            })?,
        }),
        Some(SpeculativeMode::DraftModel { max_draft, .. }) => Ok(SpeculativePlan::DraftModel {
            max_draft: usize::try_from(*max_draft).map_err(|error| {
                ModelRuntimeError::GenerateError(format!(
                    "llama.cpp draft max_draft does not fit usize: {error}"
                ))
            })?,
        }),
        Some(SpeculativeMode::Eagle3 { .. }) => unreachable!("validated above"),
    }
}

#[cfg(feature = "llama-cpp-runtime-engine")]
pub(super) fn prepare_speculative_decoder<'a>(
    plan: SpeculativePlan,
    target_native: &super::context::NativeLlamaCppBackend,
    draft_native: Option<&'a super::context::NativeLlamaCppBackend>,
    req: &GenerateRequest,
    prompt_tokens: &[llama_cpp_2::token::LlamaToken],
    quantization: crate::model_runtime::KvQuantSupport,
) -> Result<SpeculativeDecoder<'a>, ModelRuntimeError> {
    match plan {
        SpeculativePlan::None => Ok(SpeculativeDecoder::None),
        SpeculativePlan::Ngram {
            lookback,
            max_draft,
        } => Ok(SpeculativeDecoder::Ngram(NgramDraftDecoder::new(
            prompt_tokens.to_vec(),
            lookback,
            max_draft,
        ))),
        SpeculativePlan::DraftModel { max_draft } => {
            let draft_native = draft_native.ok_or_else(|| {
                ModelRuntimeError::GenerateError(
                    "llama.cpp draft-model speculation requires a loaded draft model".to_string(),
                )
            })?;
            Ok(SpeculativeDecoder::Draft(DraftModelSpeculator::new(
                target_native,
                draft_native,
                req,
                prompt_tokens,
                quantization,
                max_draft,
            )?))
        }
    }
}

#[cfg(feature = "llama-cpp-runtime-engine")]
pub(super) enum SpeculativeDecoder<'a> {
    None,
    Ngram(NgramDraftDecoder),
    Draft(DraftModelSpeculator<'a>),
}

#[cfg(feature = "llama-cpp-runtime-engine")]
impl SpeculativeDecoder<'_> {
    pub(super) fn sample_verified_round(
        &mut self,
        target_context: &mut llama_cpp_2::context::LlamaContext<'_>,
        target_sampler: &mut llama_cpp_2::sampling::LlamaSampler,
        target_logits_index: i32,
        current_position: i32,
        max_tokens: usize,
        request_cancel: &CancellationToken,
        runtime_cancel: &CancellationToken,
    ) -> Result<Option<SpeculativeRound>, ModelRuntimeError> {
        if speculative_cancelled(request_cancel, runtime_cancel) {
            return Ok(None);
        }

        match self {
            Self::None => Ok(Some(sample_target_round(
                target_context,
                target_sampler,
                target_logits_index,
            ))),
            Self::Ngram(decoder) => decoder.sample_verified_round(
                target_context,
                target_sampler,
                target_logits_index,
                current_position,
                max_tokens,
                request_cancel,
                runtime_cancel,
            ),
            Self::Draft(decoder) => decoder.sample_verified_round(
                target_context,
                target_sampler,
                target_logits_index,
                current_position,
                max_tokens,
                request_cancel,
                runtime_cancel,
            ),
        }
    }

    pub(super) fn stats(&self) -> SpeculativeStats {
        match self {
            Self::None => SpeculativeStats::default(),
            Self::Ngram(decoder) => decoder.stats,
            Self::Draft(decoder) => decoder.stats,
        }
    }
}

#[cfg(feature = "llama-cpp-runtime-engine")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TargetDecodeState {
    AlreadyDecoded,
    Required,
}

#[cfg(feature = "llama-cpp-runtime-engine")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct VerifiedToken {
    pub(super) token: llama_cpp_2::token::LlamaToken,
    pub(super) target_decode: TargetDecodeState,
}

#[cfg(feature = "llama-cpp-runtime-engine")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SpeculativeRound {
    pub(super) tokens: Vec<VerifiedToken>,
    pub(super) last_sample_index: i32,
    proposed_count: usize,
}

#[cfg(feature = "llama-cpp-runtime-engine")]
impl SpeculativeRound {
    pub(super) fn proposed_token_count(&self) -> usize {
        self.proposed_count
    }

    pub(super) fn accepted_token_count(&self) -> usize {
        count_already_decoded(self)
    }

    pub(super) fn rejected_token_count(&self) -> usize {
        self.proposed_count
            .saturating_sub(self.accepted_token_count())
    }
}

#[cfg(feature = "llama-cpp-runtime-engine")]
pub(super) struct NgramDraftDecoder {
    history: Vec<llama_cpp_2::token::LlamaToken>,
    lookback: usize,
    max_draft: usize,
    stats: SpeculativeStats,
}

#[cfg(feature = "llama-cpp-runtime-engine")]
impl NgramDraftDecoder {
    fn new(
        history: Vec<llama_cpp_2::token::LlamaToken>,
        lookback: usize,
        max_draft: usize,
    ) -> Self {
        Self {
            history,
            lookback,
            max_draft,
            stats: SpeculativeStats::default(),
        }
    }

    fn sample_verified_round(
        &mut self,
        target_context: &mut llama_cpp_2::context::LlamaContext<'_>,
        target_sampler: &mut llama_cpp_2::sampling::LlamaSampler,
        target_logits_index: i32,
        current_position: i32,
        max_tokens: usize,
        request_cancel: &CancellationToken,
        runtime_cancel: &CancellationToken,
    ) -> Result<Option<SpeculativeRound>, ModelRuntimeError> {
        if speculative_cancelled(request_cancel, runtime_cancel) {
            return Ok(None);
        }

        let draft_limit = round_draft_limit(target_context, self.max_draft, max_tokens);
        let proposed = self.next_draft(draft_limit);
        if proposed.is_empty() {
            let round = sample_target_round(target_context, target_sampler, target_logits_index);
            self.history
                .extend(round.tokens.iter().map(|item| item.token));
            return Ok(Some(round));
        }

        self.stats.record_draft(proposed.len());
        let Some(round) = verify_proposed_tokens(
            target_context,
            target_sampler,
            target_logits_index,
            current_position,
            &proposed,
            request_cancel,
            runtime_cancel,
        )?
        else {
            return Ok(None);
        };
        let accepted = count_already_decoded(&round);
        self.stats.record_verification(proposed.len(), accepted);
        self.history
            .extend(round.tokens.iter().map(|item| item.token));
        Ok(Some(round))
    }

    fn next_draft(&self, draft_limit: usize) -> Vec<llama_cpp_2::token::LlamaToken> {
        if self.history.len() <= self.lookback || draft_limit == 0 {
            return Vec::new();
        }

        let key_start = self.history.len() - self.lookback;
        let key = &self.history[key_start..];
        for candidate_start in (0..key_start).rev() {
            let candidate_end = candidate_start + self.lookback;
            if &self.history[candidate_start..candidate_end] != key {
                continue;
            }

            let draft_start = candidate_end;
            let draft_end = draft_start
                .saturating_add(draft_limit)
                .min(self.history.len());
            if draft_start < draft_end {
                return self.history[draft_start..draft_end].to_vec();
            }
        }

        Vec::new()
    }
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn validate_draft_model_compatibility(
    target_model: &llama_cpp_2::model::LlamaModel,
    draft_model: &llama_cpp_2::model::LlamaModel,
) -> Result<(), ModelRuntimeError> {
    if target_model.vocab_type() != draft_model.vocab_type() {
        return Err(ModelRuntimeError::GenerateError(format!(
            "llama.cpp draft-model speculation requires matching vocab types: target={:?}, draft={:?}",
            target_model.vocab_type(),
            draft_model.vocab_type()
        )));
    }

    let target_vocab = target_model.n_vocab();
    let draft_vocab = draft_model.n_vocab();
    if target_vocab != draft_vocab {
        return Err(ModelRuntimeError::GenerateError(format!(
            "llama.cpp draft-model speculation requires matching vocab sizes: target={target_vocab}, draft={draft_vocab}"
        )));
    }

    for (label, target_token, draft_token) in [
        ("BOS", target_model.token_bos(), draft_model.token_bos()),
        ("EOS", target_model.token_eos(), draft_model.token_eos()),
        ("NL", target_model.token_nl(), draft_model.token_nl()),
        ("SEP", target_model.token_sep(), draft_model.token_sep()),
    ] {
        if target_token != draft_token {
            return Err(ModelRuntimeError::GenerateError(format!(
                "llama.cpp draft-model speculation requires matching {label} token: target={target_token}, draft={draft_token}"
            )));
        }
    }

    let vocab_len = usize::try_from(target_vocab).map_err(|error| {
        ModelRuntimeError::GenerateError(format!(
            "llama.cpp target vocab size does not fit usize: {error}"
        ))
    })?;
    for token_id in 0..vocab_len {
        let token =
            llama_cpp_2::token::LlamaToken::new(i32::try_from(token_id).map_err(|error| {
                ModelRuntimeError::GenerateError(format!(
                    "llama.cpp vocab token id does not fit i32: {error}"
                ))
            })?);

        let target_eog = target_model.is_eog_token(token);
        let draft_eog = draft_model.is_eog_token(token);
        if target_eog != draft_eog {
            return Err(ModelRuntimeError::GenerateError(format!(
                "llama.cpp draft-model speculation requires matching EOG flags for token {token_id}: target={target_eog}, draft={draft_eog}"
            )));
        }

        let target_attr = target_model.token_attr(token);
        let draft_attr = draft_model.token_attr(token);
        if target_attr != draft_attr {
            return Err(ModelRuntimeError::GenerateError(format!(
                "llama.cpp draft-model speculation requires matching token attributes for token {token_id}: target={target_attr:?}, draft={draft_attr:?}"
            )));
        }

        let target_piece = token_piece_bytes_result(target_model, token)?;
        let draft_piece = token_piece_bytes_result(draft_model, token)?;
        if target_piece != draft_piece {
            return Err(ModelRuntimeError::GenerateError(format!(
                "llama.cpp draft-model speculation requires matching token pieces for token {token_id}"
            )));
        }
    }

    Ok(())
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn token_piece_bytes_result(
    model: &llama_cpp_2::model::LlamaModel,
    token: llama_cpp_2::token::LlamaToken,
) -> Result<Result<Vec<u8>, String>, ModelRuntimeError> {
    match token_piece_bytes_with_buffer(model, token, 8) {
        Ok(bytes) => Ok(Ok(bytes)),
        Err(error) => {
            if let llama_cpp_2::TokenToStringError::InsufficientBufferSpace(size) = &error {
                if *size < 0 {
                    let needed = usize::try_from(-i64::from(*size)).map_err(|conversion_error| {
                        ModelRuntimeError::GenerateError(format!(
                            "llama.cpp token piece buffer size does not fit usize: {conversion_error}"
                        ))
                    })?;
                    return Ok(token_piece_bytes_with_buffer(model, token, needed)
                        .map_err(|retry_error| retry_error.to_string()));
                }
            }

            Ok(Err(error.to_string()))
        }
    }
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn token_piece_bytes_with_buffer(
    model: &llama_cpp_2::model::LlamaModel,
    token: llama_cpp_2::token::LlamaToken,
    buffer_size: usize,
) -> Result<Vec<u8>, llama_cpp_2::TokenToStringError> {
    model.token_to_piece_bytes(token, buffer_size, true, None)
}

#[cfg(feature = "llama-cpp-runtime-engine")]
pub(super) struct DraftModelSpeculator<'a> {
    context: llama_cpp_2::context::LlamaContext<'a>,
    sampler: llama_cpp_2::sampling::LlamaSampler,
    history: Vec<llama_cpp_2::token::LlamaToken>,
    current_position: i32,
    last_sample_index: i32,
    max_draft: usize,
    generated_in_round: usize,
    stats: SpeculativeStats,
}

#[cfg(feature = "llama-cpp-runtime-engine")]
impl<'a> DraftModelSpeculator<'a> {
    fn new(
        target_native: &super::context::NativeLlamaCppBackend,
        draft_native: &'a super::context::NativeLlamaCppBackend,
        req: &GenerateRequest,
        target_prompt_tokens: &[llama_cpp_2::token::LlamaToken],
        quantization: crate::model_runtime::KvQuantSupport,
        max_draft: usize,
    ) -> Result<Self, ModelRuntimeError> {
        use llama_cpp_2::{llama_batch::LlamaBatch, model::AddBos};

        validate_draft_model_compatibility(&target_native.model, &draft_native.model)?;

        let draft_prompt_tokens = draft_native
            .model
            .str_to_token(req.prompt.as_str(), AddBos::Always)
            .map_err(|error| {
                ModelRuntimeError::GenerateError(format!(
                    "llama.cpp draft prompt tokenization failed: {error}"
                ))
            })?;
        if draft_prompt_tokens != target_prompt_tokens {
            return Err(ModelRuntimeError::GenerateError(
                "llama.cpp draft-model speculation requires target and draft tokenizers to produce identical prompt tokens"
                    .to_string(),
            ));
        }

        let mut context = draft_native.new_context(quantization)?;
        let mut batch = LlamaBatch::new(draft_prompt_tokens.len(), 1);
        let last_prompt_index = i32::try_from(draft_prompt_tokens.len().saturating_sub(1))
            .map_err(|error| {
                ModelRuntimeError::GenerateError(format!(
                    "draft prompt token count does not fit i32: {error}"
                ))
            })?;
        for (position, token) in draft_prompt_tokens.iter().copied().enumerate() {
            let position = i32::try_from(position).map_err(|error| {
                ModelRuntimeError::GenerateError(format!(
                    "draft prompt position does not fit i32: {error}"
                ))
            })?;
            batch
                .add(token, position, &[0], position == last_prompt_index)
                .map_err(|error| {
                    ModelRuntimeError::GenerateError(format!(
                        "failed to add draft prompt token to llama.cpp batch: {error}"
                    ))
                })?;
        }
        context.decode(&mut batch).map_err(|error| {
            ModelRuntimeError::GenerateError(format!(
                "llama.cpp draft prompt decode failed: {error}"
            ))
        })?;

        let mut sampler = sampler_plan(&req.sampling).build_llama_sampler();
        sampler.accept_many(&draft_prompt_tokens);
        let current_position = i32::try_from(draft_prompt_tokens.len()).map_err(|error| {
            ModelRuntimeError::GenerateError(format!(
                "draft prompt token count does not fit i32: {error}"
            ))
        })?;

        Ok(Self {
            context,
            sampler,
            history: draft_prompt_tokens,
            current_position,
            last_sample_index: batch.n_tokens().saturating_sub(1),
            max_draft,
            generated_in_round: 0,
            stats: SpeculativeStats::default(),
        })
    }

    fn sample_verified_round(
        &mut self,
        target_context: &mut llama_cpp_2::context::LlamaContext<'_>,
        target_sampler: &mut llama_cpp_2::sampling::LlamaSampler,
        target_logits_index: i32,
        current_position: i32,
        max_tokens: usize,
        request_cancel: &CancellationToken,
        runtime_cancel: &CancellationToken,
    ) -> Result<Option<SpeculativeRound>, ModelRuntimeError> {
        if speculative_cancelled(request_cancel, runtime_cancel) {
            return Ok(None);
        }

        let draft_limit = round_draft_limit(target_context, self.max_draft, max_tokens)
            .min(usize::try_from(self.context.n_batch()).unwrap_or(usize::MAX));
        if draft_limit == 0 {
            return Ok(Some(sample_target_round(
                target_context,
                target_sampler,
                target_logits_index,
            )));
        }
        let snapshot = DraftContextSnapshot::capture(&self.context)?;
        let previous_position = self.current_position;
        let previous_logits_index = self.last_sample_index;
        let previous_history_len = self.history.len();
        let Some(proposed) =
            self.generate_draft_tokens(draft_limit, request_cancel, runtime_cancel)?
        else {
            self.restore_to_snapshot(
                &snapshot,
                previous_position,
                previous_logits_index,
                previous_history_len,
            )?;
            return Ok(None);
        };

        self.stats.record_draft(proposed.len());
        let Some(round) = verify_proposed_tokens(
            target_context,
            target_sampler,
            target_logits_index,
            current_position,
            &proposed,
            request_cancel,
            runtime_cancel,
        )?
        else {
            self.restore_to_snapshot(
                &snapshot,
                previous_position,
                previous_logits_index,
                previous_history_len,
            )?;
            return Ok(None);
        };
        let accepted = count_already_decoded(&round);
        self.stats.record_verification(proposed.len(), accepted);

        if speculative_cancelled(request_cancel, runtime_cancel) {
            clear_round_decoded_suffix(target_context, current_position, &round)?;
            self.restore_to_snapshot(
                &snapshot,
                previous_position,
                previous_logits_index,
                previous_history_len,
            )?;
            return Ok(None);
        }

        if accepted < proposed.len() {
            snapshot.restore(&mut self.context)?;
            self.current_position = previous_position;
            self.last_sample_index = previous_logits_index;
            self.history.truncate(previous_history_len);
            self.rebuild_sampler_from_history();
            for item in &round.tokens {
                if speculative_cancelled(request_cancel, runtime_cancel) {
                    clear_round_decoded_suffix(target_context, current_position, &round)?;
                    self.restore_to_snapshot(
                        &snapshot,
                        previous_position,
                        previous_logits_index,
                        previous_history_len,
                    )?;
                    return Ok(None);
                }
                self.sampler.accept(item.token);
                decode_single_token(
                    &mut self.context,
                    item.token,
                    self.current_position,
                    "llama.cpp draft target-resync token",
                )?;
                self.current_position += 1;
                self.last_sample_index = 0;
                self.history.push(item.token);
            }
        } else {
            self.history.extend(proposed);
        }

        if speculative_cancelled(request_cancel, runtime_cancel) {
            clear_round_decoded_suffix(target_context, current_position, &round)?;
            self.restore_to_snapshot(
                &snapshot,
                previous_position,
                previous_logits_index,
                previous_history_len,
            )?;
            return Ok(None);
        }

        Ok(Some(round))
    }

    fn generate_draft_tokens(
        &mut self,
        draft_limit: usize,
        request_cancel: &CancellationToken,
        runtime_cancel: &CancellationToken,
    ) -> Result<Option<Vec<llama_cpp_2::token::LlamaToken>>, ModelRuntimeError> {
        let mut tokens = Vec::with_capacity(draft_limit);
        for _ in 0..draft_limit {
            if speculative_cancelled(request_cancel, runtime_cancel) {
                return Ok(None);
            }

            if self.generated_in_round >= self.max_draft {
                self.generated_in_round = 0;
            }

            let token = self.sampler.sample(&self.context, self.last_sample_index);
            self.sampler.accept(token);
            if speculative_cancelled(request_cancel, runtime_cancel) {
                return Ok(None);
            }

            decode_single_token(
                &mut self.context,
                token,
                self.current_position,
                "llama.cpp draft token",
            )?;
            if speculative_cancelled(request_cancel, runtime_cancel) {
                return Ok(None);
            }

            self.current_position += 1;
            self.last_sample_index = 0;
            self.generated_in_round += 1;
            tokens.push(token);
        }
        Ok(Some(tokens))
    }

    fn rebuild_sampler_from_history(&mut self) {
        self.sampler.reset();
        self.sampler.accept_many(&self.history);
    }

    fn restore_to_snapshot(
        &mut self,
        snapshot: &DraftContextSnapshot,
        previous_position: i32,
        previous_logits_index: i32,
        previous_history_len: usize,
    ) -> Result<(), ModelRuntimeError> {
        snapshot.restore(&mut self.context)?;
        self.current_position = previous_position;
        self.last_sample_index = previous_logits_index;
        self.history.truncate(previous_history_len);
        self.rebuild_sampler_from_history();
        Ok(())
    }
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn sample_target_round(
    target_context: &llama_cpp_2::context::LlamaContext<'_>,
    target_sampler: &mut llama_cpp_2::sampling::LlamaSampler,
    target_logits_index: i32,
) -> SpeculativeRound {
    let token = target_sampler.sample(target_context, target_logits_index);
    target_sampler.accept(token);
    SpeculativeRound {
        tokens: vec![VerifiedToken {
            token,
            target_decode: TargetDecodeState::Required,
        }],
        last_sample_index: target_logits_index,
        proposed_count: 0,
    }
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn verify_proposed_tokens(
    target_context: &mut llama_cpp_2::context::LlamaContext<'_>,
    target_sampler: &mut llama_cpp_2::sampling::LlamaSampler,
    target_logits_index: i32,
    current_position: i32,
    proposed: &[llama_cpp_2::token::LlamaToken],
    request_cancel: &CancellationToken,
    runtime_cancel: &CancellationToken,
) -> Result<Option<SpeculativeRound>, ModelRuntimeError> {
    debug_assert!(!proposed.is_empty());
    if speculative_cancelled(request_cancel, runtime_cancel) {
        return Ok(None);
    }

    let first_target = target_sampler.sample(target_context, target_logits_index);
    if first_target != proposed[0] {
        target_sampler.accept(first_target);
        return Ok(Some(SpeculativeRound {
            tokens: vec![VerifiedToken {
                token: first_target,
                target_decode: TargetDecodeState::Required,
            }],
            last_sample_index: target_logits_index,
            proposed_count: proposed.len(),
        }));
    }

    target_sampler.accept(proposed[0]);
    let mut batch = llama_cpp_2::llama_batch::LlamaBatch::new(proposed.len(), 1);
    for (offset, token) in proposed.iter().copied().enumerate() {
        let position = checked_position(current_position, offset)?;
        batch.add(token, position, &[0], true).map_err(|error| {
            ModelRuntimeError::GenerateError(format!(
                "failed to add llama.cpp speculative target token to batch: {error}"
            ))
        })?;
    }
    if speculative_cancelled(request_cancel, runtime_cancel) {
        return Ok(None);
    }

    target_context.decode(&mut batch).map_err(|error| {
        ModelRuntimeError::GenerateError(format!(
            "llama.cpp speculative target batch decode failed: {error}"
        ))
    })?;
    if speculative_cancelled(request_cancel, runtime_cancel) {
        clear_target_suffix(target_context, current_position, proposed.len())?;
        return Ok(None);
    }

    let mut tokens = vec![VerifiedToken {
        token: proposed[0],
        target_decode: TargetDecodeState::AlreadyDecoded,
    }];

    for (offset, proposed_token) in proposed.iter().copied().enumerate().skip(1) {
        if speculative_cancelled(request_cancel, runtime_cancel) {
            clear_target_suffix(target_context, current_position, proposed.len())?;
            return Ok(None);
        }

        let logits_index = i32::try_from(offset.saturating_sub(1)).map_err(|error| {
            ModelRuntimeError::GenerateError(format!(
                "llama.cpp speculative logits index does not fit i32: {error}"
            ))
        })?;
        let target_token = target_sampler.sample(target_context, logits_index);
        if target_token == proposed_token {
            target_sampler.accept(target_token);
            tokens.push(VerifiedToken {
                token: proposed_token,
                target_decode: TargetDecodeState::AlreadyDecoded,
            });
            if speculative_cancelled(request_cancel, runtime_cancel) {
                clear_target_suffix(target_context, current_position, proposed.len())?;
                return Ok(None);
            }
            continue;
        }

        target_sampler.accept(target_token);
        clear_target_suffix(target_context, current_position, tokens.len())?;
        tokens.push(VerifiedToken {
            token: target_token,
            target_decode: TargetDecodeState::Required,
        });
        return Ok(Some(SpeculativeRound {
            tokens,
            last_sample_index: logits_index,
            proposed_count: proposed.len(),
        }));
    }

    Ok(Some(SpeculativeRound {
        tokens,
        last_sample_index: batch.n_tokens().saturating_sub(1),
        proposed_count: proposed.len(),
    }))
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn count_already_decoded(round: &SpeculativeRound) -> usize {
    round
        .tokens
        .iter()
        .filter(|item| item.target_decode == TargetDecodeState::AlreadyDecoded)
        .count()
}

#[cfg(feature = "llama-cpp-runtime-engine")]
pub(super) fn clear_round_decoded_suffix(
    context: &mut llama_cpp_2::context::LlamaContext<'_>,
    current_position: i32,
    round: &SpeculativeRound,
) -> Result<(), ModelRuntimeError> {
    let decoded_count = count_already_decoded(round);
    if decoded_count == 0 {
        return Ok(());
    }

    clear_target_suffix(context, current_position, decoded_count)
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn speculative_cancelled(
    request_cancel: &CancellationToken,
    runtime_cancel: &CancellationToken,
) -> bool {
    request_cancel.is_cancelled() || runtime_cancel.is_cancelled()
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn round_draft_limit(
    target_context: &llama_cpp_2::context::LlamaContext<'_>,
    configured_max_draft: usize,
    max_tokens: usize,
) -> usize {
    let context_batch = usize::try_from(target_context.n_batch())
        .ok()
        .filter(|value| *value > 0)
        .unwrap_or(usize::MAX);

    configured_max_draft.min(max_tokens).min(context_batch)
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn checked_position(current_position: i32, offset: usize) -> Result<i32, ModelRuntimeError> {
    let offset = i32::try_from(offset).map_err(|error| {
        ModelRuntimeError::GenerateError(format!(
            "llama.cpp speculative token offset does not fit i32: {error}"
        ))
    })?;
    current_position.checked_add(offset).ok_or_else(|| {
        ModelRuntimeError::GenerateError(
            "llama.cpp speculative token position overflowed i32".to_string(),
        )
    })
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn clear_target_suffix(
    context: &mut llama_cpp_2::context::LlamaContext<'_>,
    current_position: i32,
    accepted_count: usize,
) -> Result<(), ModelRuntimeError> {
    let accepted_count = i32::try_from(accepted_count).map_err(|error| {
        ModelRuntimeError::GenerateError(format!(
            "llama.cpp speculative accepted token count does not fit i32: {error}"
        ))
    })?;
    let start = current_position
        .checked_add(accepted_count)
        .ok_or_else(|| {
            ModelRuntimeError::GenerateError(
                "llama.cpp speculative KV clear position overflowed i32".to_string(),
            )
        })?;
    let start = u32::try_from(start).map_err(|error| {
        ModelRuntimeError::GenerateError(format!(
            "llama.cpp speculative KV clear start is negative or too large: {error}"
        ))
    })?;
    let removed = context
        .clear_kv_cache_seq(Some(0), Some(start), None)
        .map_err(|error| {
            ModelRuntimeError::GenerateError(format!(
                "llama.cpp speculative KV suffix cleanup failed: {error}"
            ))
        })?;
    if !removed {
        return Err(ModelRuntimeError::GenerateError(
            "llama.cpp speculative KV suffix cleanup returned false".to_string(),
        ));
    }
    Ok(())
}

#[cfg(feature = "llama-cpp-runtime-engine")]
struct DraftContextSnapshot {
    data: Vec<u8>,
}

#[cfg(feature = "llama-cpp-runtime-engine")]
impl DraftContextSnapshot {
    fn capture(
        context: &llama_cpp_2::context::LlamaContext<'_>,
    ) -> Result<Self, ModelRuntimeError> {
        let mut data = vec![0_u8; context.get_state_size()];
        let copied = unsafe { context.copy_state_data(data.as_mut_ptr()) };
        if copied == 0 || copied > data.len() {
            return Err(ModelRuntimeError::GenerateError(format!(
                "llama.cpp draft context state copy returned invalid byte count {copied}"
            )));
        }
        data.truncate(copied);
        Ok(Self { data })
    }

    fn restore(
        &self,
        context: &mut llama_cpp_2::context::LlamaContext<'_>,
    ) -> Result<(), ModelRuntimeError> {
        let restored = unsafe { context.set_state_data(&self.data) };
        if restored == 0 {
            return Err(ModelRuntimeError::GenerateError(
                "llama.cpp draft context state restore failed after rejected draft token"
                    .to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn decode_single_token(
    context: &mut llama_cpp_2::context::LlamaContext<'_>,
    token: llama_cpp_2::token::LlamaToken,
    position: i32,
    label: &str,
) -> Result<(), ModelRuntimeError> {
    let mut batch = llama_cpp_2::llama_batch::LlamaBatch::new(1, 1);
    batch.add(token, position, &[0], true).map_err(|error| {
        ModelRuntimeError::GenerateError(format!("failed to add {label} to batch: {error}"))
    })?;
    context.decode(&mut batch).map_err(|error| {
        ModelRuntimeError::GenerateError(format!("{label} decode failed: {error}"))
    })
}

#[cfg(all(test, feature = "llama-cpp-runtime-engine"))]
mod tests {
    use super::*;

    #[test]
    fn ngram_decoder_drafts_from_repeated_suffix_fixture_tokens() {
        let history = [
            0, 29871, 15595, 21762, 330, 2735, 19471, 15595, 21762, 330, 2735, 19471, 15595, 21762,
            330, 2735, 19471,
        ]
        .into_iter()
        .map(llama_cpp_2::token::LlamaToken::new)
        .collect();
        let decoder = NgramDraftDecoder::new(history, 4, 4);

        let draft_ids = decoder
            .next_draft(4)
            .into_iter()
            .map(|token| token.0)
            .collect::<Vec<_>>();

        assert_eq!(draft_ids, vec![15595, 21762, 330, 2735]);
    }
}

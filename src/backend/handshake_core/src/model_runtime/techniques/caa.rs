//! MT-103: INF-5 Contrastive Activation Addition (CAA).
//!
//! Implements CAA per Rimsky et al. 2024 (arXiv:2312.06681,
//! "Steering Llama 2 via Contrastive Activation Addition") on top of the
//! MT-082..MT-083 + MT-096 activation steering plumbing. No parallel hook
//! layer - CAA reuses `techniques::activation_steering::{capture,
//! contrastive_difference_vector, register_steering_vector}`.
//!
//! CAA differs from RepE (MT-098/099) in vector derivation semantics:
//!
//! * **Paired prompts**: each pair shares context but differs in
//!   completion direction (e.g.
//!   `("Q: Are you a robot? A: Yes", "Q: Are you a robot? A: No")`).
//! * **Completion-token position**: the activation difference is computed
//!   at the COMPLETION token, not as a whole-sequence mean. The kernel
//!   encodes this by treating the supplied prompt strings as full
//!   prefixes-up-to-and-including-the-completion-token; the runtime
//!   capture path is expected to read the residual at the final token
//!   when MT-074 (LlamaCppRuntime streaming) lands. This module's
//!   contract is that the prompt text itself carries the semantic - the
//!   runtime hook attaches at the last token of the input.
//!
//! Per AC-INFER-LAB-8-TECHNIQUES the technique is flagged experimental;
//! the Work Profile knob `caa_enabled` defaults false. The kernel API
//! here is gate-agnostic - opt-in is enforced by callers (UI / Work
//! Profile).

use crate::model_runtime::{
    techniques::activation_steering::{
        capture, contrastive_difference_vector, register_steering_vector,
    },
    ContrastiveTechnique, HookPoint, LayerIndex, ModelId, ModelRuntime, ModelRuntimeError,
    SteeringProvenance, SteeringVector, SteeringVectorId, SteeringVectorValues,
};

/// Default additive intensity for a freshly-extracted CAA vector. Operators
/// can override via the steering UI intensity slider; this is the value the
/// extraction helper stamps on the resulting [`SteeringVector`].
pub const CAA_DEFAULT_INTENSITY: f32 = 1.0;

/// A paired-prompt input to CAA extraction. The two strings should share
/// context and differ only in completion direction.
#[derive(Clone, Debug, PartialEq)]
pub struct CaaPromptPair {
    pub positive: String,
    pub negative: String,
}

/// Extracts a CAA steering vector from `pairs` at the supplied layer.
///
/// The function flattens pairs into `[positive_0, positive_1, ..., negative_0,
/// negative_1, ...]` and calls the shared activation_steering capture path
/// once. The contrastive difference (mean(positives) - mean(negatives)) at
/// the requested layer becomes the steering values. Construction returns a
/// fully-formed [`SteeringVector`] with provenance =
/// `Contrastive { technique: CAA }`; pass to
/// [`register_caa_vector`] to commit through the steering hook ops.
pub async fn extract_caa_vector(
    runtime: &dyn ModelRuntime,
    model_id: ModelId,
    pairs: Vec<CaaPromptPair>,
    layer: LayerIndex,
    name: impl Into<String>,
    description: impl Into<String>,
) -> Result<SteeringVector, ModelRuntimeError> {
    if pairs.is_empty() {
        return Err(ModelRuntimeError::SteeringHookError(
            "CAA extraction requires at least one prompt pair".to_string(),
        ));
    }
    for (idx, pair) in pairs.iter().enumerate() {
        if pair.positive.trim().is_empty() {
            return Err(ModelRuntimeError::SteeringHookError(format!(
                "CAA prompt pair {idx} has empty positive completion"
            )));
        }
        if pair.negative.trim().is_empty() {
            return Err(ModelRuntimeError::SteeringHookError(format!(
                "CAA prompt pair {idx} has empty negative completion"
            )));
        }
    }

    let positive_count = pairs.len();
    let mut prompts = Vec::with_capacity(positive_count * 2);
    for pair in &pairs {
        prompts.push(pair.positive.clone());
    }
    for pair in &pairs {
        prompts.push(pair.negative.clone());
    }

    let capture_result = capture(runtime, model_id, prompts, vec![layer]).await?;
    let direction = contrastive_difference_vector(&capture_result, layer, positive_count)?;

    let positive_prompts: Vec<String> = pairs.iter().map(|p| p.positive.clone()).collect();
    let negative_prompts: Vec<String> = pairs.iter().map(|p| p.negative.clone()).collect();
    let provenance = SteeringProvenance::Contrastive {
        positive_prompts,
        negative_prompts,
        technique: ContrastiveTechnique::CAA,
    };

    let values = SteeringVectorValues::try_new(direction, CAA_DEFAULT_INTENSITY)?;
    SteeringVector::try_new(
        None,
        name,
        layer,
        HookPoint::ResidStream,
        values,
        description,
        Some(provenance),
    )
}

/// Convenience wrapper that extracts a CAA vector and registers it through
/// the shared activation_steering hook ops. Returns the new
/// [`SteeringVectorId`] for downstream activation by `set_active_steering_vectors`.
pub async fn register_caa_vector(
    runtime: &dyn ModelRuntime,
    model_id: ModelId,
    pairs: Vec<CaaPromptPair>,
    layer: LayerIndex,
    name: impl Into<String>,
    description: impl Into<String>,
) -> Result<SteeringVectorId, ModelRuntimeError> {
    let vector = extract_caa_vector(runtime, model_id, pairs, layer, name, description).await?;
    register_steering_vector(runtime, model_id, vector).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caa_default_intensity_is_one() {
        // Pin the contract: the additive default for a freshly-extracted
        // CAA vector is 1.0 (per Rimsky 2024 baseline). Any change must
        // edit this assertion in the same commit as the contract revision.
        assert!((CAA_DEFAULT_INTENSITY - 1.0).abs() < f32::EPSILON);
    }
}

use crate::model_runtime::{
    CaptureResult, CaptureSpec, HookPoint, LayerIndex, ModelId, ModelRuntime, ModelRuntimeError,
    SteeringHookHandle, SteeringProvenance, SteeringVector, SteeringVectorId, SteeringVectorMeta,
};

pub const FR_EVT_LLM_INFER_STEER_CAPTURE: &str = "FR-EVT-LLM-INFER-STEER-CAPTURE";
pub const FR_EVT_LLM_INFER_STEER_REGISTER: &str = "FR-EVT-LLM-INFER-STEER-REGISTER";
pub const FR_EVT_LLM_INFER_STEER_ACTIVE: &str = "FR-EVT-LLM-INFER-STEER-ACTIVE";
pub const FR_EVT_LLM_INFER_STEER_APPLY: &str = "FR-EVT-LLM-INFER-STEER-APPLY";
// MT-096: completes the steering-vector lifecycle taxonomy. REGISTER has no
// observable counterpart for the unregister side, so a withdrawn vector cannot
// be correlated against its registration in the flight recorder. WITHDRAW is
// the unregister-side lifecycle event id (declaration parity with its siblings,
// which are emitted by the downstream recorder-wired layer, not these wrappers).
pub const FR_EVT_LLM_INFER_STEER_WITHDRAW: &str = "FR-EVT-LLM-INFER-STEER-WITHDRAW";

pub async fn capture(
    runtime: &dyn ModelRuntime,
    model_id: ModelId,
    prompts: Vec<String>,
    layers: Vec<LayerIndex>,
) -> Result<CaptureResult, ModelRuntimeError> {
    validate_capture_request(&prompts, &layers)?;
    let hooks = activation_steering_hooks(runtime, model_id)?;
    hooks
        .capture(CaptureSpec {
            prompts,
            layers,
            hook_point: HookPoint::ResidStream,
        })
        .await
}

pub async fn register_steering_vector(
    runtime: &dyn ModelRuntime,
    model_id: ModelId,
    vector: SteeringVector,
) -> Result<SteeringVectorId, ModelRuntimeError> {
    validate_steering_provenance(&vector.derivation_provenance)?;
    let hooks = activation_steering_hooks(runtime, model_id)?;
    hooks.register_vector(vector).await
}

pub async fn set_active_steering_vectors(
    runtime: &dyn ModelRuntime,
    model_id: ModelId,
    ids: Vec<SteeringVectorId>,
) -> Result<(), ModelRuntimeError> {
    let hooks = activation_steering_hooks(runtime, model_id)?;
    hooks.set_active(ids).await
}

/// Unregister (withdraw) a steering vector. The withdrawal corresponds to the
/// [`FR_EVT_LLM_INFER_STEER_WITHDRAW`] lifecycle event id, the unregister-side
/// counterpart of [`FR_EVT_LLM_INFER_STEER_REGISTER`].
pub async fn unregister(
    runtime: &dyn ModelRuntime,
    model_id: ModelId,
    id: SteeringVectorId,
) -> Result<(), ModelRuntimeError> {
    let hooks = activation_steering_hooks(runtime, model_id)?;
    hooks.unregister(id).await
}

pub fn list_vectors(
    runtime: &dyn ModelRuntime,
    model_id: ModelId,
) -> Result<Vec<SteeringVectorMeta>, ModelRuntimeError> {
    let hooks = activation_steering_hooks(runtime, model_id)?;
    Ok(hooks.list_vectors())
}

pub fn contrastive_difference_vector(
    capture_result: &CaptureResult,
    layer: LayerIndex,
    positive_count: usize,
) -> Result<Vec<f32>, ModelRuntimeError> {
    let rows = capture_result.activations.get(&layer).ok_or_else(|| {
        ModelRuntimeError::SteeringHookError(format!(
            "capture result does not include requested layer {}",
            layer.as_u32()
        ))
    })?;
    if positive_count == 0 || positive_count >= rows.len() {
        return Err(ModelRuntimeError::SteeringHookError(format!(
            "positive_count must be in range 1..{} for contrastive direction derivation; got {positive_count}",
            rows.len().saturating_sub(1)
        )));
    }

    let width = rows
        .first()
        .map(Vec::len)
        .filter(|width| *width > 0)
        .ok_or_else(|| {
            ModelRuntimeError::SteeringHookError(
                "capture rows must contain non-empty activation vectors".to_string(),
            )
        })?;
    for row in rows {
        if row.len() != width {
            return Err(ModelRuntimeError::SteeringHookError(format!(
                "capture row width {} does not match expected width {width}",
                row.len()
            )));
        }
        if row.iter().any(|value| !value.is_finite()) {
            return Err(ModelRuntimeError::SteeringHookError(
                "capture rows must contain only finite activation values".to_string(),
            ));
        }
    }

    let positive_mean = mean_rows(&rows[..positive_count], width);
    let negative_mean = mean_rows(&rows[positive_count..], width);
    Ok(positive_mean
        .into_iter()
        .zip(negative_mean)
        .map(|(positive, negative)| positive - negative)
        .collect())
}

fn activation_steering_hooks(
    runtime: &dyn ModelRuntime,
    model_id: ModelId,
) -> Result<SteeringHookHandle, ModelRuntimeError> {
    let capabilities = runtime.capabilities(model_id)?;
    if !capabilities.supports_activation_steering {
        return Err(ModelRuntimeError::CapabilityNotSupported {
            capability: "activation_steering".to_string(),
            adapter: runtime.adapter_name().to_string(),
        });
    }
    runtime.steering_hooks(model_id)
}

fn validate_capture_request(
    prompts: &[String],
    layers: &[LayerIndex],
) -> Result<(), ModelRuntimeError> {
    if prompts.is_empty() {
        return Err(ModelRuntimeError::SteeringHookError(
            "activation steering capture requires at least one prompt".to_string(),
        ));
    }
    if prompts.iter().any(|prompt| prompt.trim().is_empty()) {
        return Err(ModelRuntimeError::SteeringHookError(
            "activation steering capture prompts must not be blank".to_string(),
        ));
    }
    if layers.is_empty() {
        return Err(ModelRuntimeError::SteeringHookError(
            "activation steering capture requires at least one layer".to_string(),
        ));
    }
    Ok(())
}

fn validate_steering_provenance(provenance: &SteeringProvenance) -> Result<(), ModelRuntimeError> {
    match provenance {
        SteeringProvenance::Manual { author, .. } => {
            if author.trim().is_empty() {
                return Err(ModelRuntimeError::SteeringHookError(
                    "manual steering provenance author is required".to_string(),
                ));
            }
        }
        SteeringProvenance::Contrastive {
            positive_prompts,
            negative_prompts,
            ..
        } => {
            if positive_prompts.is_empty()
                || positive_prompts
                    .iter()
                    .any(|prompt| prompt.trim().is_empty())
            {
                return Err(ModelRuntimeError::SteeringHookError(
                    "contrastive steering provenance positive_prompts must be non-empty"
                        .to_string(),
                ));
            }
            if negative_prompts.is_empty()
                || negative_prompts
                    .iter()
                    .any(|prompt| prompt.trim().is_empty())
            {
                return Err(ModelRuntimeError::SteeringHookError(
                    "contrastive steering provenance negative_prompts must be non-empty"
                        .to_string(),
                ));
            }
        }
    }
    Ok(())
}

fn mean_rows(rows: &[Vec<f32>], width: usize) -> Vec<f32> {
    let mut sum = vec![0.0; width];
    for row in rows {
        for (index, value) in row.iter().enumerate() {
            sum[index] += *value;
        }
    }
    let denominator = rows.len() as f32;
    sum.into_iter().map(|value| value / denominator).collect()
}

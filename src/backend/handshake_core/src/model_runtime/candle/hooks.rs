use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;

use crate::model_runtime::{
    CaptureResult, CaptureSpec, HookPoint, LayerIndex, ModelId, ModelRuntimeError, SteeringHookOps,
    SteeringVector, SteeringVectorId, SteeringVectorMeta, SteeringVectorValues,
};

pub const CANDLE_DEFAULT_RESIDUAL_WIDTH: usize = 4096;

#[derive(Clone)]
pub struct CandleSteeringHooks {
    model_id: ModelId,
    residual_width: usize,
    registry: Arc<Mutex<HookRegistry>>,
}

#[derive(Default)]
struct HookRegistry {
    vectors: BTreeMap<SteeringVectorId, SteeringVector>,
    active: BTreeSet<SteeringVectorId>,
    forward_counts: BTreeMap<LayerIndex, u64>,
    capture_layers: Option<HashSet<LayerIndex>>,
    captured_activations: BTreeMap<LayerIndex, Vec<Vec<f32>>>,
    capture_tokens_seen: u32,
}

impl CandleSteeringHooks {
    pub fn new_for_model(model_id: ModelId, residual_width: usize) -> Self {
        Self {
            model_id,
            residual_width: residual_width.max(1),
            registry: Arc::new(Mutex::new(HookRegistry::default())),
        }
    }

    pub fn model_id(&self) -> ModelId {
        self.model_id
    }

    pub fn residual_width(&self) -> usize {
        self.residual_width
    }

    pub async fn capture(&self, spec: CaptureSpec) -> Result<CaptureResult, ModelRuntimeError> {
        <Self as SteeringHookOps>::capture(self, spec).await
    }

    pub async fn register_vector(
        &self,
        vector: SteeringVector,
    ) -> Result<SteeringVectorId, ModelRuntimeError> {
        <Self as SteeringHookOps>::register_vector(self, vector).await
    }

    pub fn list_vectors(&self) -> Vec<SteeringVectorMeta> {
        <Self as SteeringHookOps>::list_vectors(self)
    }

    pub async fn set_active(&self, ids: Vec<SteeringVectorId>) -> Result<(), ModelRuntimeError> {
        <Self as SteeringHookOps>::set_active(self, ids).await
    }

    pub async fn unregister(&self, id: SteeringVectorId) -> Result<(), ModelRuntimeError> {
        <Self as SteeringHookOps>::unregister(self, id).await
    }

    pub fn active_vector_ids(&self) -> Vec<SteeringVectorId> {
        self.try_active_vector_ids().unwrap_or_default()
    }

    pub fn try_active_vector_ids(&self) -> Result<Vec<SteeringVectorId>, ModelRuntimeError> {
        self.with_registry(|registry| registry.active.iter().copied().collect())
    }

    pub fn try_list_vectors(&self) -> Result<Vec<SteeringVectorMeta>, ModelRuntimeError> {
        self.with_registry(|registry| {
            registry
                .vectors
                .values()
                .map(SteeringVectorMeta::from)
                .collect()
        })
    }

    pub fn snapshot_vectors_for_request(
        &self,
        steering_overrides: &[SteeringVectorId],
    ) -> Result<Vec<SteeringVector>, ModelRuntimeError> {
        self.with_registry(|registry| {
            let mut ids = registry.active.iter().copied().collect::<BTreeSet<_>>();
            ids.extend(steering_overrides.iter().copied());
            ids.into_iter()
                .map(|id| {
                    registry.vectors.get(&id).cloned().ok_or_else(|| {
                        ModelRuntimeError::SteeringHookError(format!(
                            "cannot snapshot unknown steering vector {id}"
                        ))
                    })
                })
                .collect()
        })?
    }

    pub fn apply_registered_vectors(
        &self,
        layer: LayerIndex,
        hook_point: HookPoint,
        activation: Vec<f32>,
    ) -> Result<Vec<f32>, ModelRuntimeError> {
        let vectors = self.snapshot_vectors_for_request(&[])?;
        Self::apply_vector_snapshot_to_activation(layer, hook_point, activation, &vectors)
    }

    pub fn apply_vector_snapshot_to_activation(
        layer: LayerIndex,
        hook_point: HookPoint,
        activation: Vec<f32>,
        vectors: &[SteeringVector],
    ) -> Result<Vec<f32>, ModelRuntimeError> {
        Self::validate_resid_stream_only(hook_point)?;
        let mut current = activation;
        for vector in vectors
            .iter()
            .filter(|vector| vector.layer == layer && vector.hook_point == hook_point)
        {
            current = if vector.is_refusal_ablation() {
                // Arditi 2024 directional ablation: project the residual out of
                // the refusal direction rather than additively steering it.
                Self::ablate_activation_against_direction(current, vector.values.values())?
            } else {
                Self::apply_vector_to_activation(current, &vector.values)?
            };
        }
        Ok(current)
    }

    #[cfg(feature = "candle-runtime-engine")]
    pub fn apply_vector_snapshot_to_tensor(
        layer: LayerIndex,
        hook_point: HookPoint,
        activation: &candle_core::Tensor,
        vectors: &[SteeringVector],
    ) -> Result<candle_core::Tensor, ModelRuntimeError> {
        Self::validate_resid_stream_only(hook_point)?;
        let Some(hidden_width) = activation.dims().last().copied() else {
            return Err(ModelRuntimeError::SteeringHookError(
                "cannot apply steering vector to scalar activation".to_string(),
            ));
        };

        let mut current = activation.clone();
        for vector in vectors
            .iter()
            .filter(|vector| vector.layer == layer && vector.hook_point == hook_point)
        {
            if vector.values.values().len() != hidden_width {
                return Err(ModelRuntimeError::SteeringHookError(format!(
                    "activation width {hidden_width} does not match steering vector width {}",
                    vector.values.values().len()
                )));
            }
            let mut vector_shape = vec![1_usize; activation.dims().len()];
            let last = vector_shape.len().saturating_sub(1);
            vector_shape[last] = hidden_width;
            let direction = candle_core::Tensor::from_slice(
                vector.values.values(),
                vector.values.values().len(),
                activation.device(),
            )
            .and_then(|tensor| tensor.to_dtype(activation.dtype()))
            .and_then(|tensor| tensor.reshape(vector_shape.as_slice()))
            .and_then(|tensor| tensor.broadcast_as(activation.shape()))
            .map_err(|error| {
                ModelRuntimeError::SteeringHookError(format!(
                    "failed to build Candle steering tensor: {error}"
                ))
            })?;

            current = if vector.is_refusal_ablation() {
                // Arditi 2024 directional ablation on the live tensor path:
                // current - (current·dir / dir·dir) * dir, broadcast across the
                // token/batch dims. dir·dir is taken from the raw f32 slice
                // (cheap, and guards the degenerate zero-direction case).
                let dir_norm_sq: f64 = vector
                    .values
                    .values()
                    .iter()
                    .map(|d| (*d as f64) * (*d as f64))
                    .sum();
                if !dir_norm_sq.is_finite() || dir_norm_sq == 0.0 {
                    return Err(ModelRuntimeError::SteeringHookError(
                        "refusal ablation direction has zero or non-finite L2 norm".to_string(),
                    ));
                }
                let dot = (&current * &direction)
                    .and_then(|prod| prod.sum_keepdim(last))
                    .map_err(|error| {
                        ModelRuntimeError::SteeringHookError(format!(
                            "failed to compute refusal ablation projection: {error}"
                        ))
                    })?;
                let projection = (dot * (1.0 / dir_norm_sq))
                    .and_then(|coeff| coeff.broadcast_as(activation.shape()))
                    .and_then(|coeff| (coeff * &direction))
                    .map_err(|error| {
                        ModelRuntimeError::SteeringHookError(format!(
                            "failed to build refusal ablation projection tensor: {error}"
                        ))
                    })?;
                (&current - projection).map_err(|error| {
                    ModelRuntimeError::SteeringHookError(format!(
                        "failed to apply refusal ablation tensor: {error}"
                    ))
                })?
            } else {
                let steering =
                    (&direction * vector.values.intensity() as f64).map_err(|error| {
                        ModelRuntimeError::SteeringHookError(format!(
                            "failed to scale Candle steering tensor: {error}"
                        ))
                    })?;
                (&current + steering).map_err(|error| {
                    ModelRuntimeError::SteeringHookError(format!(
                        "failed to apply Candle steering tensor: {error}"
                    ))
                })?
            };
        }
        Ok(current)
    }

    #[cfg(feature = "candle-runtime-engine")]
    pub fn apply_record_and_capture_tensor(
        &self,
        layer: LayerIndex,
        hook_point: HookPoint,
        activation: &candle_core::Tensor,
        vectors: &[SteeringVector],
    ) -> Result<candle_core::Tensor, ModelRuntimeError> {
        self.record_forward_layer(layer)?;
        let adjusted =
            Self::apply_vector_snapshot_to_tensor(layer, hook_point, activation, vectors)?;
        self.capture_tensor_if_requested(layer, &adjusted)?;
        Ok(adjusted)
    }

    pub fn begin_real_capture(&self, layers: &[LayerIndex]) -> Result<(), ModelRuntimeError> {
        if layers.is_empty() {
            return Err(ModelRuntimeError::SteeringHookError(
                "capture spec requires at least one layer".to_string(),
            ));
        }
        let capture_layers = layers.iter().copied().collect::<HashSet<_>>();
        self.with_registry_mut(|registry| {
            registry.capture_layers = Some(capture_layers);
            registry.captured_activations.clear();
            registry.capture_tokens_seen = 0;
            Ok(())
        })
    }

    pub fn finish_real_capture(
        &self,
        layers: &[LayerIndex],
    ) -> Result<CaptureResult, ModelRuntimeError> {
        self.with_registry_mut(|registry| {
            registry.capture_layers = None;
            for layer in layers {
                registry
                    .captured_activations
                    .entry(*layer)
                    .or_insert_with(Vec::new);
            }
            Ok(CaptureResult {
                activations: std::mem::take(&mut registry.captured_activations),
                tokens_seen: std::mem::take(&mut registry.capture_tokens_seen),
            })
        })
    }

    pub fn record_forward_layer(&self, layer: LayerIndex) -> Result<(), ModelRuntimeError> {
        self.with_registry_mut(|registry| {
            *registry.forward_counts.entry(layer).or_default() += 1;
            Ok(())
        })
    }

    pub fn forward_layer_count(&self, layer: LayerIndex) -> Result<u64, ModelRuntimeError> {
        self.with_registry(|registry| registry.forward_counts.get(&layer).copied().unwrap_or(0))
    }

    pub fn run_resid_stream_forward_harness(
        &self,
        layers: BTreeMap<LayerIndex, Vec<Vec<f32>>>,
        capture_layers: &[LayerIndex],
        steering_overrides: &[SteeringVectorId],
    ) -> Result<CaptureResult, ModelRuntimeError> {
        if layers.is_empty() {
            return Err(ModelRuntimeError::SteeringHookError(
                "forward harness requires at least one residual stream layer".to_string(),
            ));
        }
        if capture_layers.is_empty() {
            return Err(ModelRuntimeError::SteeringHookError(
                "forward harness requires at least one capture layer".to_string(),
            ));
        }

        let capture_set = capture_layers.iter().copied().collect::<HashSet<_>>();
        let vectors = self.snapshot_vectors_for_request(steering_overrides)?;
        let mut activations = BTreeMap::new();
        let mut tokens_seen = 0_u32;

        for (layer, rows) in layers {
            self.record_forward_layer(layer)?;
            tokens_seen = tokens_seen.max(rows.len() as u32);
            let mut captured_rows = Vec::new();
            for row in rows {
                if row.len() != self.residual_width {
                    return Err(ModelRuntimeError::SteeringHookError(format!(
                        "residual row width {} does not match Candle residual width {}",
                        row.len(),
                        self.residual_width
                    )));
                }
                let adjusted = Self::apply_vector_snapshot_to_activation(
                    layer,
                    HookPoint::ResidStream,
                    row,
                    &vectors,
                )?;
                if capture_set.contains(&layer) {
                    captured_rows.push(adjusted);
                }
            }
            if capture_set.contains(&layer) {
                activations.insert(layer, captured_rows);
            }
        }

        for layer in capture_layers {
            activations.entry(*layer).or_insert_with(Vec::new);
        }

        Ok(CaptureResult {
            activations,
            tokens_seen,
        })
    }

    pub fn apply_vector_to_activation(
        activation: Vec<f32>,
        vector: &SteeringVectorValues,
    ) -> Result<Vec<f32>, ModelRuntimeError> {
        if activation.len() != vector.values().len() {
            return Err(ModelRuntimeError::SteeringHookError(format!(
                "activation width {} does not match steering vector width {}",
                activation.len(),
                vector.values().len()
            )));
        }
        activation
            .into_iter()
            .zip(vector.values())
            .map(|(base, steering)| {
                let value = base + (*steering * vector.intensity());
                if value.is_finite() {
                    Ok(value)
                } else {
                    Err(ModelRuntimeError::SteeringHookError(
                        "steering vector application produced a non-finite activation".to_string(),
                    ))
                }
            })
            .collect()
    }

    /// Directional ablation per Arditi et al. 2024 ("Refusal in LLMs is
    /// Mediated by a Single Direction"): remove the component of `activation`
    /// that lies along `direction` by subtracting its projection,
    /// `new = activation - (activation·dir / dir·dir) * dir`.
    ///
    /// This is the operation a [`crate::model_runtime::SteeringVector`] flagged
    /// via [`crate::model_runtime::SteeringVector::is_refusal_ablation`]
    /// resolves to, instead of the additive [`Self::apply_vector_to_activation`].
    /// `direction` need not be unit length — normalising by `dir·dir` makes the
    /// result the true orthogonal component either way — but refusal directions
    /// are unit-normalised at extraction, so the projection coefficient reduces
    /// to the plain dot product. The vector's intensity marker
    /// (`REFUSAL_ABLATION_INTENSITY`) is deliberately NOT applied here: ablation
    /// fully removes the projection rather than scaling it.
    pub fn ablate_activation_against_direction(
        activation: Vec<f32>,
        direction: &[f32],
    ) -> Result<Vec<f32>, ModelRuntimeError> {
        if activation.len() != direction.len() {
            return Err(ModelRuntimeError::SteeringHookError(format!(
                "activation width {} does not match refusal direction width {}",
                activation.len(),
                direction.len()
            )));
        }
        let dir_norm_sq: f32 = direction.iter().map(|d| d * d).sum();
        if !dir_norm_sq.is_finite() || dir_norm_sq == 0.0 {
            return Err(ModelRuntimeError::SteeringHookError(
                "refusal ablation direction has zero or non-finite L2 norm".to_string(),
            ));
        }
        let dot: f32 = activation
            .iter()
            .zip(direction)
            .map(|(a, d)| a * d)
            .sum();
        let coeff = dot / dir_norm_sq;
        activation
            .into_iter()
            .zip(direction)
            .map(|(base, dir)| {
                let value = base - coeff * dir;
                if value.is_finite() {
                    Ok(value)
                } else {
                    Err(ModelRuntimeError::SteeringHookError(
                        "refusal ablation produced a non-finite activation".to_string(),
                    ))
                }
            })
            .collect()
    }

    fn validate_resid_stream_only(hook_point: HookPoint) -> Result<(), ModelRuntimeError> {
        if hook_point == HookPoint::ResidStream {
            return Ok(());
        }

        let capability = match hook_point {
            HookPoint::ResidStream => "resid_stream",
            HookPoint::MlpOut => "mlp_out",
            HookPoint::AttnOut => "attn_out",
        };
        Err(ModelRuntimeError::CapabilityNotSupported {
            capability: format!("{capability} hook point"),
            adapter: "candle_hooks".to_string(),
        })
    }

    fn with_registry<T>(
        &self,
        read: impl FnOnce(&HookRegistry) -> T,
    ) -> Result<T, ModelRuntimeError> {
        let registry = self.registry.lock().map_err(|_| {
            ModelRuntimeError::SteeringHookError(
                "candle hook registry lock is poisoned".to_string(),
            )
        })?;
        Ok(read(&registry))
    }

    fn with_registry_mut<T>(
        &self,
        write: impl FnOnce(&mut HookRegistry) -> Result<T, ModelRuntimeError>,
    ) -> Result<T, ModelRuntimeError> {
        let mut registry = self.registry.lock().map_err(|_| {
            ModelRuntimeError::SteeringHookError(
                "candle hook registry lock is poisoned".to_string(),
            )
        })?;
        write(&mut registry)
    }

    #[cfg(feature = "candle-runtime-engine")]
    fn capture_tensor_if_requested(
        &self,
        layer: LayerIndex,
        activation: &candle_core::Tensor,
    ) -> Result<(), ModelRuntimeError> {
        let should_capture = self.with_registry(|registry| {
            registry
                .capture_layers
                .as_ref()
                .is_some_and(|layers| layers.contains(&layer))
        })?;
        if !should_capture {
            return Ok(());
        }

        let rows = tensor_rows(activation)?;
        self.with_registry_mut(|registry| {
            if registry
                .capture_layers
                .as_ref()
                .is_some_and(|layers| layers.contains(&layer))
            {
                registry.capture_tokens_seen = registry.capture_tokens_seen.max(rows.len() as u32);
                registry
                    .captured_activations
                    .entry(layer)
                    .or_default()
                    .extend(rows);
            }
            Ok(())
        })
    }

    fn scaffold_resid_stream_row(&self, layer: LayerIndex, prompt: &str) -> Vec<f32> {
        let seed = prompt.bytes().fold(layer.as_u32(), |acc, byte| {
            acc.wrapping_mul(31).wrapping_add(u32::from(byte))
        });
        (0..self.residual_width)
            .map(|index| ((seed.wrapping_add(index as u32) % 997) as f32) / 997.0)
            .collect()
    }
}

#[cfg(feature = "candle-runtime-engine")]
fn tensor_rows(activation: &candle_core::Tensor) -> Result<Vec<Vec<f32>>, ModelRuntimeError> {
    let activation = activation
        .to_dtype(candle_core::DType::F32)
        .map_err(|error| {
            ModelRuntimeError::SteeringHookError(format!(
                "failed to convert Candle activation to f32 rows: {error}"
            ))
        })?;
    match activation.dims() {
        [_hidden] => Ok(vec![activation
            .to_vec1::<f32>()
            .map_err(tensor_row_error)?]),
        [_tokens, _hidden] => activation.to_vec2::<f32>().map_err(tensor_row_error),
        [_batch, _tokens, _hidden] => Ok(activation
            .to_vec3::<f32>()
            .map_err(tensor_row_error)?
            .into_iter()
            .flatten()
            .collect()),
        dims => Err(ModelRuntimeError::SteeringHookError(format!(
            "unsupported Candle activation rank for capture: {dims:?}"
        ))),
    }
}

#[cfg(feature = "candle-runtime-engine")]
fn tensor_row_error(error: candle_core::Error) -> ModelRuntimeError {
    ModelRuntimeError::SteeringHookError(format!(
        "failed to read Candle activation rows for capture: {error}"
    ))
}

impl std::fmt::Debug for CandleSteeringHooks {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CandleSteeringHooks")
            .field("model_id", &self.model_id)
            .field("residual_width", &self.residual_width)
            .field("active_vector_ids", &self.active_vector_ids())
            .finish()
    }
}

#[async_trait]
impl SteeringHookOps for CandleSteeringHooks {
    async fn capture(&self, spec: CaptureSpec) -> Result<CaptureResult, ModelRuntimeError> {
        Self::validate_resid_stream_only(spec.hook_point)?;
        if spec.prompts.is_empty() {
            return Err(ModelRuntimeError::SteeringHookError(
                "capture spec requires at least one prompt".to_string(),
            ));
        }
        if spec.layers.is_empty() {
            return Err(ModelRuntimeError::SteeringHookError(
                "capture spec requires at least one layer".to_string(),
            ));
        }

        let layers = spec
            .layers
            .iter()
            .map(|layer| {
                let rows = spec
                    .prompts
                    .iter()
                    .map(|prompt| self.scaffold_resid_stream_row(*layer, prompt))
                    .collect::<Vec<_>>();
                (*layer, rows)
            })
            .collect::<BTreeMap<_, _>>();

        self.run_resid_stream_forward_harness(layers, &spec.layers, &[])
    }

    async fn register_vector(
        &self,
        vector: SteeringVector,
    ) -> Result<SteeringVectorId, ModelRuntimeError> {
        Self::validate_resid_stream_only(vector.hook_point)?;
        if vector.values.values().len() != self.residual_width {
            return Err(ModelRuntimeError::SteeringHookError(format!(
                "steering vector width {} does not match Candle residual width {}",
                vector.values.values().len(),
                self.residual_width
            )));
        }
        if vector
            .values
            .values()
            .iter()
            .any(|value| !value.is_finite())
        {
            return Err(ModelRuntimeError::SteeringHookError(
                "steering vector values must be finite".to_string(),
            ));
        }

        let id = vector.id;
        self.with_registry_mut(|registry| {
            registry.vectors.insert(id, vector);
            Ok(id)
        })
    }

    fn list_vectors(&self) -> Vec<SteeringVectorMeta> {
        self.try_list_vectors().unwrap_or_default()
    }

    async fn set_active(&self, ids: Vec<SteeringVectorId>) -> Result<(), ModelRuntimeError> {
        self.with_registry_mut(|registry| {
            for id in &ids {
                if !registry.vectors.contains_key(id) {
                    return Err(ModelRuntimeError::SteeringHookError(format!(
                        "cannot activate unknown steering vector {id}"
                    )));
                }
            }
            registry.active = ids.into_iter().collect();
            Ok(())
        })
    }

    async fn unregister(&self, id: SteeringVectorId) -> Result<(), ModelRuntimeError> {
        self.with_registry_mut(|registry| {
            registry.vectors.remove(&id).ok_or_else(|| {
                ModelRuntimeError::SteeringHookError(format!(
                    "cannot unregister unknown steering vector {id}"
                ))
            })?;
            registry.active.remove(&id);
            Ok(())
        })
    }
}

// ----------------------------------------------------------------------------
// MT-116 — SSM-variant hook mapping surface.
//
// Per refinement INF-9 feature_parity_detail ("activation steering for
// SSMs (steering-for-SSM work)") + operator E-2 FULL FEATURE PARITY.
//
// Scope discipline: this MT lands the per-architecture HookPoint mapping +
// token-by-token semantics documentation + the typed deferral marker.
// Threading the hook callback through the per-variant candle forward
// passes (mamba2.rs / rwkv_v5.rs / rwkv_v6.rs / rwkv_v7.rs) lands in the
// follow-on weight-application MT alongside MT-115's SSM-LoRA actual
// wiring — both face the same candle-transformers extensibility blocker
// (no hookable weight/activation slots in the upstream wrappers today).
//
// capabilities.supportsActivationSteering for SSM variants stays false
// until the follow-on flips the flag and proves identity-test correctness
// (zero vector -> output unchanged) on a fixture model. Same scaffold-
// then-flip pattern as MT-111 EAGLE-3 + MT-115 LoRA-for-SSM.
// ----------------------------------------------------------------------------

use crate::model_runtime::candle::ssm_lora::SSMArchitectureTag;

/// Per-architecture mapping site that names WHERE in the SSM forward path
/// a generic `HookPoint` lands. The follow-on weight-application MT
/// consumes this enum to thread the callback through each variant's
/// per-layer forward.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SSMHookSite {
    pub arch: SSMArchitectureTag,
    pub layer: LayerIndex,
    pub point: HookPoint,
    /// Stable site label for telemetry + the operator manual. The
    /// follow-on weight-application MT must emit this label in
    /// FR-EVT-LLM-INFER-STEER-CAPTURE so dashboards can join across
    /// architectures.
    pub site_label: &'static str,
}

/// Token-by-token semantics doc (single source of truth). The follow-on
/// MT must quote this in the operator manual + propagate it to
/// CaptureSpec.prompts handling so the contrastive-pair capture
/// behaviour matches CAA semantics on transformers.
pub const SSM_HOOK_TOKEN_BY_TOKEN_SEMANTICS: &str =
    "SSM hooks fire once per token (vs once per prompt-batch for transformers). \
     CaptureSpec.prompts is processed sequentially: the hook callback receives \
     the per-token hidden state, and the steering-vector derivation captures \
     the activation at the LAST token of each prompt to match CAA semantics on \
     the transformer path.";

/// Deferred-to-follow-on marker for the SSM steering registration path.
/// Returned by the actual register/capture/set_active calls on SSM
/// variants until the follow-on weight-application MT lands the per-
/// variant forward-pass wiring.
pub const SSM_STEERING_DEFERRED_MARKER: &str = "ssm_activation_steering_disabled_pending_followon";

/// Map a generic `HookPoint` to the SSM-variant-specific site per the
/// MT-116 contract narrative:
/// - Mamba2:   ResidStream = layer-block output;
///             MlpOut       = out_proj output;
///             AttnOut      = x_proj branch output (approximate; Mamba2
///                            has no attention proper).
/// - RWKV v5/v6/v7: ResidStream = time-mix + channel-mix combined output;
///                  MlpOut       = channel-mix output;
///                  AttnOut      = time-mix output.
pub fn ssm_hook_site_for(
    arch: SSMArchitectureTag,
    layer: LayerIndex,
    point: HookPoint,
) -> SSMHookSite {
    let site_label = match (arch, point) {
        (SSMArchitectureTag::Mamba2, HookPoint::ResidStream) => "mamba2.layer_block.output",
        (SSMArchitectureTag::Mamba2, HookPoint::MlpOut) => "mamba2.out_proj.output",
        (SSMArchitectureTag::Mamba2, HookPoint::AttnOut) => "mamba2.x_proj.output",
        (
            SSMArchitectureTag::RwkvV5 | SSMArchitectureTag::RwkvV6 | SSMArchitectureTag::RwkvV7,
            HookPoint::ResidStream,
        ) => "rwkv.layer_block.output",
        (
            SSMArchitectureTag::RwkvV5 | SSMArchitectureTag::RwkvV6 | SSMArchitectureTag::RwkvV7,
            HookPoint::MlpOut,
        ) => "rwkv.channel_mix.output",
        (
            SSMArchitectureTag::RwkvV5 | SSMArchitectureTag::RwkvV6 | SSMArchitectureTag::RwkvV7,
            HookPoint::AttnOut,
        ) => "rwkv.time_mix.output",
    };
    SSMHookSite {
        arch,
        layer,
        point,
        site_label,
    }
}

/// Convenience: return the deferral-marker error for the SSM steering
/// register path until the follow-on weight-application MT lands.
pub fn ssm_steering_register_deferred_error(arch: SSMArchitectureTag) -> ModelRuntimeError {
    ModelRuntimeError::CapabilityNotSupported {
        capability: SSM_STEERING_DEFERRED_MARKER.to_string(),
        adapter: format!("candle_{}", arch.as_str()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_runtime::{
        ContrastiveTechnique, SteeringProvenance, SteeringVector, SteeringVectorValues,
    };

    fn refusal_vector(layer: LayerIndex, direction: Vec<f32>) -> SteeringVector {
        let values = SteeringVectorValues::try_new(direction, -1.0).expect("values");
        SteeringVector::try_new(
            None,
            "refusal-test",
            layer,
            HookPoint::ResidStream,
            values,
            "refusal ablation test vector",
            Some(SteeringProvenance::Contrastive {
                positive_prompts: vec!["harmful".to_string()],
                negative_prompts: vec!["harmless".to_string()],
                technique: ContrastiveTechnique::RefusalVector,
            }),
        )
        .expect("vector")
    }

    fn additive_vector(layer: LayerIndex, values: Vec<f32>, intensity: f32) -> SteeringVector {
        let values = SteeringVectorValues::try_new(values, intensity).expect("values");
        SteeringVector::try_new(
            None,
            "additive-test",
            layer,
            HookPoint::ResidStream,
            values,
            "additive steering test vector",
            Some(SteeringProvenance::Manual {
                author: "tester".to_string(),
                notes: "n".to_string(),
            }),
        )
        .expect("vector")
    }

    fn dot(a: &[f32], b: &[f32]) -> f32 {
        a.iter().zip(b).map(|(x, y)| x * y).sum()
    }

    #[test]
    fn ablate_against_unit_direction_removes_projection() {
        // direction (1,0) unit; activation (3,4); projection coeff = 3 -> (0,4).
        let out =
            CandleSteeringHooks::ablate_activation_against_direction(vec![3.0, 4.0], &[1.0, 0.0])
                .expect("ablate");
        assert!(out[0].abs() < 1e-6, "x={}", out[0]);
        assert!((out[1] - 4.0).abs() < 1e-6, "y={}", out[1]);
        assert!(dot(&out, &[1.0, 0.0]).abs() < 1e-6);
    }

    #[test]
    fn ablate_against_non_unit_direction_normalises_by_dir_dot_dir() {
        // direction (0,2) non-unit; activation (5,7); coeff = (7*2)/4 = 3.5
        // -> (5,7) - 3.5*(0,2) = (5,0); result orthogonal to direction.
        let out =
            CandleSteeringHooks::ablate_activation_against_direction(vec![5.0, 7.0], &[0.0, 2.0])
                .expect("ablate");
        assert!((out[0] - 5.0).abs() < 1e-6, "x={}", out[0]);
        assert!(out[1].abs() < 1e-6, "y={}", out[1]);
        assert!(dot(&out, &[0.0, 2.0]).abs() < 1e-5);
    }

    #[test]
    fn ablate_is_idempotent_after_first_projection_removal() {
        let once = CandleSteeringHooks::ablate_activation_against_direction(
            vec![3.0, 4.0, 5.0],
            &[1.0, 1.0, 0.0],
        )
        .expect("ablate once");
        let twice = CandleSteeringHooks::ablate_activation_against_direction(
            once.clone(),
            &[1.0, 1.0, 0.0],
        )
        .expect("ablate twice");
        for (a, b) in once.iter().zip(&twice) {
            assert!((a - b).abs() < 1e-5, "{a} != {b}");
        }
    }

    #[test]
    fn ablate_rejects_zero_direction() {
        let err =
            CandleSteeringHooks::ablate_activation_against_direction(vec![1.0, 2.0], &[0.0, 0.0])
                .expect_err("zero direction must error");
        assert!(format!("{err:?}").contains("zero"), "{err:?}");
    }

    #[test]
    fn ablate_rejects_width_mismatch() {
        let err = CandleSteeringHooks::ablate_activation_against_direction(
            vec![1.0, 2.0, 3.0],
            &[1.0, 0.0],
        )
        .expect_err("width mismatch must error");
        assert!(format!("{err:?}").contains("width"), "{err:?}");
    }

    #[test]
    fn refusal_vector_application_orthogonalises_not_adds() {
        let layer = LayerIndex::new(14);
        let vector = refusal_vector(layer, vec![1.0, 0.0, 0.0]);
        let out = CandleSteeringHooks::apply_vector_snapshot_to_activation(
            layer,
            HookPoint::ResidStream,
            vec![2.0, 5.0, 9.0],
            std::slice::from_ref(&vector),
        )
        .expect("apply");
        // Orthogonalisation removes the x-component entirely: (0,5,9).
        assert!(out[0].abs() < 1e-6, "x not projected out: {}", out[0]);
        assert!((out[1] - 5.0).abs() < 1e-6);
        assert!((out[2] - 9.0).abs() < 1e-6);
        assert!(dot(&out, &[1.0, 0.0, 0.0]).abs() < 1e-6);
        // The additive path with intensity -1 would have produced
        // (2-1, 5, 9) = (1,5,9); orthogonalisation is provably different.
        assert!(
            (out[0] - 1.0).abs() > 1e-3,
            "refusal vector took the additive path instead of orthogonalising"
        );
    }

    #[test]
    fn non_refusal_vector_still_applies_additively() {
        let layer = LayerIndex::new(14);
        let vector = additive_vector(layer, vec![1.0, 1.0, 1.0], 2.0);
        let out = CandleSteeringHooks::apply_vector_snapshot_to_activation(
            layer,
            HookPoint::ResidStream,
            vec![10.0, 20.0, 30.0],
            std::slice::from_ref(&vector),
        )
        .expect("apply");
        // base + steering * intensity = base + 1 * 2.
        assert_eq!(out, vec![12.0, 22.0, 32.0]);
    }
}

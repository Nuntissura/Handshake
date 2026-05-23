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
            current = Self::apply_vector_to_activation(current, &vector.values)?;
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
            let steering = candle_core::Tensor::from_slice(
                vector.values.values(),
                vector.values.values().len(),
                activation.device(),
            )
            .and_then(|tensor| tensor.to_dtype(activation.dtype()))
            .and_then(|tensor| tensor.reshape(vector_shape.as_slice()))
            .and_then(|tensor| tensor.broadcast_as(activation.shape()))
            .and_then(|tensor| (tensor * vector.values.intensity() as f64))
            .map_err(|error| {
                ModelRuntimeError::SteeringHookError(format!(
                    "failed to build Candle steering tensor: {error}"
                ))
            })?;
            current = (&current + steering).map_err(|error| {
                ModelRuntimeError::SteeringHookError(format!(
                    "failed to apply Candle steering tensor: {error}"
                ))
            })?;
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

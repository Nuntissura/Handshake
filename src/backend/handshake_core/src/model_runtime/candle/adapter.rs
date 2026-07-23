use std::{
    collections::HashMap,
    fs::File,
    io::Read,
    path::Path,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use async_trait::async_trait;
use futures::stream;
use sha2::{Digest, Sha256};

#[cfg(feature = "candle-runtime-engine")]
use super::{
    artifact_snapshot::{capture_candle_artifact, CapturedCandleArtifact},
    generate::{candle_generate_stream_tracked, CandleGenerationCodec, TokenizerGenerationCodec},
    mamba2::{config_value_declares_mamba2, decode_mamba2_config_value, CandleMamba2Model},
    rwkv_v5::{
        config_value_declares_rwkv_v5, config_value_declares_unversioned_rwkv,
        decode_rwkv_config_value, CandleRwkvV5Model,
    },
    rwkv_v6::{config_value_declares_rwkv_v6, decode_rwkv_v6_config_value, CandleRwkvV6Model},
    rwkv_v7::{config_value_declares_rwkv_v7, decode_rwkv_v7_config_value, CandleRwkvV7Model},
    score_embed::{candle_embed_tokens, candle_score_sequence},
    ssm_state::{LockedSsmStateSource, SsmStateSource},
    transformer::{decode_llama_config_value, CandleLlamaModel, TransformerModel},
};
use super::{
    device::{select_candle_device, CandleDevicePreference, CandleDeviceSelection},
    hooks::CandleSteeringHooks,
    state_vector::{SSMStateVariant, StateVectorHandle},
    tokenizer::CandleTokenizerCache,
};
use crate::model_runtime::{
    CancellationToken, Embedding, GenerateRequest, HookPoint, KvCacheHandle, KvQuantSupport,
    LoadSpec, LoraStackHandle, ModelArtifactIntegrityReceipt, ModelCapabilities, ModelId,
    ModelRuntime, ModelRuntimeError, ProviderKind, RuntimeActivityKind, RuntimeActivityTracker,
    RuntimeArtifactIntegrityReceipt, RuntimeKind, RuntimeQuiesceError, Score, SteeringHookHandle,
    SteeringHookOps, SteeringVector, SteeringVectorId, SteeringVectorMeta, TokenStream,
};
#[cfg(feature = "candle-runtime-engine")]
use crate::model_runtime::{CaptureResult, CaptureSpec};
#[cfg(feature = "candle-runtime-engine")]
use candle_core::DType;
#[cfg(feature = "candle-runtime-engine")]
use candle_nn::VarBuilder;

pub const CANDLE_NATIVE_FEATURE_DISABLED: &str =
    "Candle native engine feature disabled; enable candle-runtime-engine";

pub struct CandleRuntime {
    models: HashMap<ModelId, CandleModelHandle>,
    activity: RuntimeActivityTracker,
    device_selection: CandleDeviceSelection,
    tokenizer_cache: CandleTokenizerCache,
    #[cfg(feature = "candle-runtime-engine")]
    native_device: candle_core::Device,
}

/// Fully validated result of the official Candle boot-load path. The model is
/// published in the runtime cache only after this receipt, architecture-derived
/// capability set, configured hash, required-capability contract, and UUIDv7
/// identity have all been validated together.
#[derive(Clone, Debug)]
pub struct AttestedCandleLoad {
    pub model_id: ModelId,
    pub artifact_integrity: ModelArtifactIntegrityReceipt,
    pub capabilities: ModelCapabilities,
}

impl CandleRuntime {
    pub fn with_device_preference(preference: CandleDevicePreference) -> Self {
        let device_selection = select_candle_device(preference);
        Self {
            models: HashMap::new(),
            activity: RuntimeActivityTracker::new(),
            #[cfg(feature = "candle-runtime-engine")]
            native_device: super::device::native_device_for_selection(&device_selection),
            device_selection,
            tokenizer_cache: CandleTokenizerCache::new(),
        }
    }

    pub fn device_selection(&self) -> &CandleDeviceSelection {
        &self.device_selection
    }

    pub fn tokenizer_cache_len(&self) -> usize {
        self.tokenizer_cache.len()
    }

    pub fn load_duration_ms(&self, id: ModelId) -> Result<u128, ModelRuntimeError> {
        self.models
            .get(&id)
            .map(|handle| handle.load_duration_ms)
            .ok_or_else(|| ModelRuntimeError::LoadError(Self::not_loaded_message(id)))
    }

    fn not_loaded_message(id: ModelId) -> String {
        format!("candle model is not loaded: {id}")
    }

    fn not_implemented(operation: &str) -> ModelRuntimeError {
        ModelRuntimeError::CapabilityNotSupported {
            capability: format!("{operation} not implemented"),
            adapter: "candle_mt081_scaffold_not_implemented".to_string(),
        }
    }

    #[cfg(feature = "candle-runtime-engine")]
    fn native_binding_marker(&self) -> &'static str {
        let _ = &self.native_device;
        std::any::type_name::<candle_transformers::generation::LogitsProcessor>()
    }

    pub fn state_vector(&self, id: ModelId) -> Result<StateVectorHandle, ModelRuntimeError> {
        let handle = self
            .models
            .get(&id)
            .ok_or_else(|| ModelRuntimeError::KvCacheError(Self::not_loaded_message(id)))?;
        handle
            .state_vector
            .clone()
            .ok_or_else(|| ModelRuntimeError::CapabilityNotSupported {
                capability: "state_vector_cache".to_string(),
                adapter: "candle_transformer".to_string(),
            })
    }

    /// Official production load boundary used by default boot. No model/cache
    /// publication occurs on an attestation or requirement failure.
    pub async fn load_attested(
        &mut self,
        spec: LoadSpec,
        required_capabilities: &ModelCapabilities,
    ) -> Result<AttestedCandleLoad, ModelRuntimeError> {
        validate_candle_load_spec_fields(&spec)?;

        #[cfg(not(feature = "candle-runtime-engine"))]
        {
            let _ = required_capabilities;
            Err(ModelRuntimeError::LoadError(
                CANDLE_NATIVE_FEATURE_DISABLED.to_string(),
            ))
        }

        #[cfg(feature = "candle-runtime-engine")]
        {
            let started = Instant::now();
            let captured = capture_candle_artifact(&spec)?;
            let prepared = self.prepare_captured_artifact(&spec, captured, started)?;
            self.attest_and_publish(&spec, prepared, required_capabilities)
        }
    }

    #[cfg(feature = "candle-runtime-engine")]
    fn prepare_captured_artifact(
        &self,
        spec: &LoadSpec,
        captured: CapturedCandleArtifact,
        started: Instant,
    ) -> Result<PreparedCandleLoad, ModelRuntimeError> {
        let id = ModelId::new_v7();
        let artifact_sha256 = captured.receipt.weights.sha256.clone();
        let tokenizer_present = captured.tokenizer.is_some();
        let architecture = detect_captured_architecture(&captured.config)?;
        let _ = self.native_binding_marker();
        let vb = VarBuilder::from_slice_safetensors(
            captured.weights.as_ref(),
            DType::F32,
            &self.native_device,
        )
        .map_err(|error| {
            ModelRuntimeError::LoadError(format!(
                "failed to load captured Candle safetensors bytes: {error}"
            ))
        })?;

        let (backend, capabilities, state_vector, residual_width) = match architecture {
            CapturedCandleArchitecture::Mamba2 => {
                let (config, eos_token_ids) = decode_mamba2_config_value(&captured.config)?;
                let model = CandleMamba2Model::from_varbuilder_for_model(
                    id,
                    config,
                    eos_token_ids,
                    vb,
                    &self.native_device,
                )?;
                let residual_width = model.hidden_dim() as usize;
                let model_arc: Arc<Mutex<Box<dyn TransformerModel>>> =
                    Arc::new(Mutex::new(Box::new(model)));
                let state_source: Arc<dyn SsmStateSource> =
                    Arc::new(LockedSsmStateSource::new(Arc::clone(&model_arc)));
                let state_vector = state_vector_handle_with_live_source(
                    id,
                    &artifact_sha256,
                    SSMStateVariant::Mamba2,
                    state_source,
                )?;
                (
                    CandleModelBackend::Mamba2 { model: model_arc },
                    candle_mamba2_capabilities(&spec.declared_capabilities),
                    Some(state_vector),
                    residual_width,
                )
            }
            CapturedCandleArchitecture::RwkvV7 => {
                let (config, eos_token_ids) = decode_rwkv_v7_config_value(&captured.config)?;
                let model = CandleRwkvV7Model::from_varbuilder_for_model(
                    id,
                    config,
                    eos_token_ids,
                    vb,
                    &self.native_device,
                )?;
                let residual_width = model.hidden_dim() as usize;
                let model_arc: Arc<Mutex<Box<dyn TransformerModel>>> =
                    Arc::new(Mutex::new(Box::new(model)));
                let state_source: Arc<dyn SsmStateSource> =
                    Arc::new(LockedSsmStateSource::new(Arc::clone(&model_arc)));
                let state_vector = state_vector_handle_with_live_source(
                    id,
                    &artifact_sha256,
                    SSMStateVariant::RwkvV7,
                    state_source,
                )?;
                (
                    CandleModelBackend::RwkvV7 { model: model_arc },
                    candle_rwkv_capabilities(&spec.declared_capabilities),
                    Some(state_vector),
                    residual_width,
                )
            }
            CapturedCandleArchitecture::RwkvV6 => {
                let (config, eos_token_ids) = decode_rwkv_v6_config_value(&captured.config)?;
                let model = CandleRwkvV6Model::from_varbuilder_for_model(
                    id,
                    config,
                    eos_token_ids,
                    vb,
                    &self.native_device,
                )?;
                let residual_width = model.hidden_dim() as usize;
                let model_arc: Arc<Mutex<Box<dyn TransformerModel>>> =
                    Arc::new(Mutex::new(Box::new(model)));
                let state_source: Arc<dyn SsmStateSource> =
                    Arc::new(LockedSsmStateSource::new(Arc::clone(&model_arc)));
                let state_vector = state_vector_handle_with_live_source(
                    id,
                    &artifact_sha256,
                    SSMStateVariant::RwkvV6,
                    state_source,
                )?;
                (
                    CandleModelBackend::RwkvV6 { model: model_arc },
                    candle_rwkv_capabilities(&spec.declared_capabilities),
                    Some(state_vector),
                    residual_width,
                )
            }
            CapturedCandleArchitecture::RwkvV5 => {
                let (config, eos_token_ids) = decode_rwkv_config_value(&captured.config)?;
                let model = CandleRwkvV5Model::from_varbuilder_for_model(
                    id,
                    config,
                    eos_token_ids,
                    vb,
                    &self.native_device,
                )?;
                let residual_width = model.hidden_dim() as usize;
                let model_arc: Arc<Mutex<Box<dyn TransformerModel>>> =
                    Arc::new(Mutex::new(Box::new(model)));
                let state_source: Arc<dyn SsmStateSource> =
                    Arc::new(LockedSsmStateSource::new(Arc::clone(&model_arc)));
                let state_vector = state_vector_handle_with_live_source(
                    id,
                    &artifact_sha256,
                    SSMStateVariant::RwkvV5,
                    state_source,
                )?;
                (
                    CandleModelBackend::RwkvV5 { model: model_arc },
                    candle_rwkv_capabilities(&spec.declared_capabilities),
                    Some(state_vector),
                    residual_width,
                )
            }
            CapturedCandleArchitecture::Llama => {
                let (config, eos_token_ids) = decode_llama_config_value(&captured.config)?;
                let model = CandleLlamaModel::from_varbuilder_for_model_with_eos(
                    id,
                    config,
                    eos_token_ids,
                    vb,
                    &self.native_device,
                )?;
                let residual_width = model.hidden_dim() as usize;
                let mut actual_capabilities =
                    candle_transformer_capabilities(&spec.declared_capabilities);
                actual_capabilities.supports_embedding &= tokenizer_present;
                actual_capabilities.embedding_dimension = actual_capabilities
                    .supports_embedding
                    .then_some(residual_width);
                (
                    CandleModelBackend::Transformer {
                        model: Arc::new(Mutex::new(Box::new(model))),
                    },
                    actual_capabilities,
                    None,
                    residual_width,
                )
            }
        };

        let handle = CandleModelHandle {
            backend,
            capabilities,
            cancel: CancellationToken::new(),
            load_duration_ms: started.elapsed().as_millis().max(1),
            device_selection: self.device_selection.clone(),
            steering_hooks: CandleSteeringHooks::new_for_model(id, residual_width),
            state_vector,
            artifact_integrity: captured.receipt,
        };

        Ok(PreparedCandleLoad {
            model_id: id,
            handle,
            tokenizer: captured.tokenizer,
        })
    }

    #[cfg(feature = "candle-runtime-engine")]
    fn attest_and_publish(
        &mut self,
        spec: &LoadSpec,
        prepared: PreparedCandleLoad,
        required_capabilities: &ModelCapabilities,
    ) -> Result<AttestedCandleLoad, ModelRuntimeError> {
        let expected_weights = decode_expected_sha256(&spec.sha256_expected)?;
        prepared
            .handle
            .artifact_integrity
            .validate_for_expected_weights(expected_weights)?;
        ensure_candle_capabilities_satisfy(required_capabilities, &prepared.handle.capabilities)?;
        if prepared.model_id.as_uuid().get_version_num() != 7 {
            return Err(ModelRuntimeError::LoadError(format!(
                "Candle attested load returned non-UUIDv7 model id {}",
                prepared.model_id
            )));
        }
        if self.models.contains_key(&prepared.model_id)
            || self.tokenizer_cache.contains_key(&prepared.model_id)
        {
            return Err(ModelRuntimeError::LoadError(format!(
                "Candle attested load minted duplicate model id {}",
                prepared.model_id
            )));
        }

        let attested = AttestedCandleLoad {
            model_id: prepared.model_id,
            artifact_integrity: prepared.handle.artifact_integrity.clone(),
            capabilities: prepared.handle.capabilities.clone(),
        };
        // Every fallible step completed above. Publication is a no-await
        // critical section, so callers cannot interleave with these updates.
        if let Some(tokenizer) = prepared.tokenizer {
            self.tokenizer_cache.insert(prepared.model_id, tokenizer);
        }
        self.models.insert(prepared.model_id, prepared.handle);
        Ok(attested)
    }
}

impl Default for CandleRuntime {
    fn default() -> Self {
        Self::with_device_preference(CandleDevicePreference::Auto)
    }
}

#[allow(dead_code)]
struct CandleModelHandle {
    backend: CandleModelBackend,
    capabilities: ModelCapabilities,
    cancel: CancellationToken,
    load_duration_ms: u128,
    device_selection: CandleDeviceSelection,
    steering_hooks: CandleSteeringHooks,
    state_vector: Option<StateVectorHandle>,
    artifact_integrity: ModelArtifactIntegrityReceipt,
}

#[cfg(feature = "candle-runtime-engine")]
struct PreparedCandleLoad {
    model_id: ModelId,
    handle: CandleModelHandle,
    tokenizer: Option<Arc<tokenizers::Tokenizer>>,
}

enum CandleModelBackend {
    #[cfg(feature = "candle-runtime-engine")]
    Transformer {
        model: Arc<Mutex<Box<dyn TransformerModel>>>,
    },
    #[cfg(feature = "candle-runtime-engine")]
    Mamba2 {
        model: Arc<Mutex<Box<dyn TransformerModel>>>,
    },
    #[cfg(feature = "candle-runtime-engine")]
    RwkvV5 {
        model: Arc<Mutex<Box<dyn TransformerModel>>>,
    },
    #[cfg(feature = "candle-runtime-engine")]
    RwkvV6 {
        model: Arc<Mutex<Box<dyn TransformerModel>>>,
    },
    #[cfg(feature = "candle-runtime-engine")]
    RwkvV7 {
        model: Arc<Mutex<Box<dyn TransformerModel>>>,
    },
    #[cfg(not(feature = "candle-runtime-engine"))]
    TransformerScaffold,
}

#[cfg(feature = "candle-runtime-engine")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CapturedCandleArchitecture {
    Llama,
    Mamba2,
    RwkvV5,
    RwkvV6,
    RwkvV7,
}

#[cfg(feature = "candle-runtime-engine")]
fn detect_captured_architecture(
    config: &serde_json::Value,
) -> Result<CapturedCandleArchitecture, ModelRuntimeError> {
    if config_value_declares_mamba2(config) {
        return Ok(CapturedCandleArchitecture::Mamba2);
    }
    if config_value_declares_rwkv_v7(config) {
        return Ok(CapturedCandleArchitecture::RwkvV7);
    }
    if config_value_declares_rwkv_v6(config) {
        return Ok(CapturedCandleArchitecture::RwkvV6);
    }
    if config_value_declares_rwkv_v5(config) {
        return Ok(CapturedCandleArchitecture::RwkvV5);
    }
    if config_value_declares_unversioned_rwkv(config) {
        return Err(ModelRuntimeError::LoadError(
            "Candle RWKV config declares generic RWKV without a v5, v6, or v7 marker; use model_type rwkv5/rwkv6/rwkv7 or a versioned architecture marker"
                .to_string(),
        ));
    }
    Ok(CapturedCandleArchitecture::Llama)
}

#[async_trait]
impl ModelRuntime for CandleRuntime {
    async fn load(&mut self, spec: LoadSpec) -> Result<ModelId, ModelRuntimeError> {
        let required_capabilities = ModelCapabilities::default();
        self.load_attested(spec, &required_capabilities)
            .await
            .map(|attested| attested.model_id)
    }

    async fn unload(&mut self, id: ModelId) -> Result<(), ModelRuntimeError> {
        self.models.remove(&id).ok_or_else(|| {
            ModelRuntimeError::UnloadError(format!("candle model is not loaded: {id}"))
        })?;
        self.tokenizer_cache.remove(&id);
        Ok(())
    }

    async fn quiesce_model(
        &self,
        id: ModelId,
        timeout: std::time::Duration,
    ) -> Result<(), crate::model_runtime::activity::RuntimeQuiesceError> {
        self.activity.quiesce_model(id, timeout).await
    }

    fn resume_model_admission(
        &self,
        id: ModelId,
    ) -> Result<(), crate::model_runtime::activity::RuntimeQuiesceError> {
        self.activity.resume_model(id);
        Ok(())
    }

    fn generate(&self, req: GenerateRequest) -> TokenStream {
        let Some(handle) = self.models.get(&req.id) else {
            return single_error_stream(ModelRuntimeError::GenerateError(
                Self::not_loaded_message(req.id),
            ));
        };

        if req.cancel.is_cancelled() || handle.cancel.is_cancelled() {
            return single_error_stream(ModelRuntimeError::Cancelled);
        }

        let activity_guard = match self.activity.try_register(
            req.id,
            RuntimeActivityKind::Generate,
            Some(req.cancel.clone()),
        ) {
            Ok(guard) => guard,
            Err(error) => {
                return single_error_stream(ModelRuntimeError::GenerateError(error.to_string()));
            }
        };

        #[cfg(feature = "candle-runtime-engine")]
        {
            match &handle.backend {
                CandleModelBackend::Transformer { model }
                | CandleModelBackend::Mamba2 { model }
                | CandleModelBackend::RwkvV5 { model }
                | CandleModelBackend::RwkvV6 { model }
                | CandleModelBackend::RwkvV7 { model } => {
                    let Some(tokenizer) = self.tokenizer_cache.get(&req.id).cloned() else {
                        return single_error_stream(ModelRuntimeError::GenerateError(format!(
                            "candle tokenizer is not loaded for model {}",
                            req.id
                        )));
                    };
                    candle_generate_stream_tracked(
                        model.clone(),
                        Arc::new(TokenizerGenerationCodec::new(tokenizer)),
                        handle.steering_hooks.clone(),
                        req,
                        handle.cancel.clone(),
                        activity_guard,
                    )
                }
                _ => single_error_stream(Self::not_implemented("candle_generate")),
            }
        }

        #[cfg(not(feature = "candle-runtime-engine"))]
        {
            drop(activity_guard);
            single_error_stream(Self::not_implemented("candle_generate"))
        }
    }

    async fn score(&self, id: ModelId, sequence: Vec<u32>) -> Result<Score, ModelRuntimeError> {
        let handle = self
            .models
            .get(&id)
            .ok_or_else(|| ModelRuntimeError::ScoreError(Self::not_loaded_message(id)))?;

        #[cfg(feature = "candle-runtime-engine")]
        {
            let activity_guard = self
                .activity
                .try_register(id, RuntimeActivityKind::Score, None)
                .map_err(|error| ModelRuntimeError::ScoreError(error.to_string()))?;
            // Teacher-forcing scoring works for any backend whose model exposes
            // the per-position logits seam. The Transformer backend implements
            // it; SSM backends fall through to the trait default (typed
            // CapabilityNotSupported) honestly rather than faking a score.
            match &handle.backend {
                CandleModelBackend::Transformer { model }
                | CandleModelBackend::Mamba2 { model }
                | CandleModelBackend::RwkvV5 { model }
                | CandleModelBackend::RwkvV6 { model }
                | CandleModelBackend::RwkvV7 { model } => {
                    let model = model.clone();
                    let hooks = handle.steering_hooks.clone();
                    // The forward is CPU-bound and synchronous; run it on a
                    // blocking worker so the async scheduler is not stalled.
                    if tokio::runtime::Handle::try_current().is_ok() {
                        tokio::task::spawn_blocking(move || {
                            let _activity_guard = activity_guard;
                            candle_score_sequence(&model, &hooks, sequence)
                        })
                        .await
                        .map_err(|error| {
                            ModelRuntimeError::ScoreError(format!(
                                "Candle score worker failed to join: {error}"
                            ))
                        })?
                    } else {
                        let _activity_guard = activity_guard;
                        candle_score_sequence(&model, &hooks, sequence)
                    }
                }
            }
        }

        #[cfg(not(feature = "candle-runtime-engine"))]
        {
            let _ = (handle, sequence);
            Err(Self::not_implemented("candle_score"))
        }
    }

    async fn embed(&self, id: ModelId, text: &str) -> Result<Embedding, ModelRuntimeError> {
        let handle = self
            .models
            .get(&id)
            .ok_or_else(|| ModelRuntimeError::EmbedError(Self::not_loaded_message(id)))?;

        #[cfg(feature = "candle-runtime-engine")]
        {
            let activity_guard = self
                .activity
                .try_register(id, RuntimeActivityKind::Embed, None)
                .map_err(|error| ModelRuntimeError::EmbedError(error.to_string()))?;
            let Some(tokenizer) = self.tokenizer_cache.get(&id).cloned() else {
                return Err(ModelRuntimeError::EmbedError(format!(
                    "candle tokenizer is not loaded for model {id}"
                )));
            };
            let encoding = tokenizer.encode(text, true).map_err(|error| {
                ModelRuntimeError::EmbedError(format!(
                    "Candle embed tokenizer encode failed: {error}"
                ))
            })?;
            let token_ids = encoding.get_ids().to_vec();

            match &handle.backend {
                CandleModelBackend::Transformer { model }
                | CandleModelBackend::Mamba2 { model }
                | CandleModelBackend::RwkvV5 { model }
                | CandleModelBackend::RwkvV6 { model }
                | CandleModelBackend::RwkvV7 { model } => {
                    let model = model.clone();
                    let hooks = handle.steering_hooks.clone();
                    if tokio::runtime::Handle::try_current().is_ok() {
                        tokio::task::spawn_blocking(move || {
                            let _activity_guard = activity_guard;
                            candle_embed_tokens(&model, &hooks, token_ids)
                        })
                        .await
                        .map_err(|error| {
                            ModelRuntimeError::EmbedError(format!(
                                "Candle embed worker failed to join: {error}"
                            ))
                        })?
                    } else {
                        let _activity_guard = activity_guard;
                        candle_embed_tokens(&model, &hooks, token_ids)
                    }
                }
            }
        }

        #[cfg(not(feature = "candle-runtime-engine"))]
        {
            let _ = (handle, text);
            Err(Self::not_implemented("candle_embed"))
        }
    }

    async fn quiesce(&self, timeout: Duration) -> Result<(), RuntimeQuiesceError> {
        self.activity.quiesce(timeout).await
    }

    fn artifact_integrity(
        &self,
        id: ModelId,
    ) -> Result<RuntimeArtifactIntegrityReceipt, ModelRuntimeError> {
        self.models
            .get(&id)
            .map(|handle| handle.artifact_integrity.clone().into())
            .ok_or_else(|| ModelRuntimeError::LoadError(Self::not_loaded_message(id)))
    }

    fn capabilities(&self, id: ModelId) -> Result<&ModelCapabilities, ModelRuntimeError> {
        self.models
            .get(&id)
            .map(|handle| &handle.capabilities)
            .ok_or_else(|| ModelRuntimeError::LoadError(Self::not_loaded_message(id)))
    }

    fn kv_cache(&self, id: ModelId) -> Result<KvCacheHandle, ModelRuntimeError> {
        let handle = self
            .models
            .get(&id)
            .ok_or_else(|| ModelRuntimeError::KvCacheError(Self::not_loaded_message(id)))?;
        if let Some(state_vector) = &handle.state_vector {
            return Ok(state_vector.as_kv_cache_handle());
        }
        Err(Self::not_implemented("candle_kv_cache"))
    }

    fn lora_stack(&self, id: ModelId) -> Result<LoraStackHandle, ModelRuntimeError> {
        let handle = self
            .models
            .get(&id)
            .ok_or_else(|| ModelRuntimeError::LoraStackError(Self::not_loaded_message(id)))?;
        #[cfg(feature = "candle-runtime-engine")]
        {
            if let CandleModelBackend::Transformer { model } = &handle.backend {
                let model = model.lock().map_err(|_| {
                    ModelRuntimeError::LoraStackError(
                        "Candle transformer model lock is poisoned".to_string(),
                    )
                })?;
                return Ok(model.lora_stack());
            }
        }
        Err(Self::not_implemented("candle_lora_stack"))
    }

    fn steering_hooks(&self, id: ModelId) -> Result<SteeringHookHandle, ModelRuntimeError> {
        let handle = self
            .models
            .get(&id)
            .ok_or_else(|| ModelRuntimeError::SteeringHookError(Self::not_loaded_message(id)))?;
        if !handle.capabilities.supports_activation_steering {
            return Err(ModelRuntimeError::CapabilityNotSupported {
                capability: "activation_steering".to_string(),
                adapter: "candle".to_string(),
            });
        }
        #[cfg(feature = "candle-runtime-engine")]
        {
            if let CandleModelBackend::Transformer { model } = &handle.backend {
                let tokenizer = self.tokenizer_cache.get(&id).cloned().ok_or_else(|| {
                    ModelRuntimeError::SteeringHookError(format!(
                        "candle tokenizer is not loaded for model {id}"
                    ))
                })?;
                return Ok(SteeringHookHandle::with_ops(
                    format!("candle:{id}:activation_hooks"),
                    Arc::new(CandleRuntimeSteeringHookOps {
                        model: model.clone(),
                        codec: Arc::new(TokenizerGenerationCodec::new(tokenizer)),
                        hooks: handle.steering_hooks.clone(),
                    }),
                ));
            }
        }

        Ok(SteeringHookHandle::with_ops(
            format!("candle:{id}:activation_hooks"),
            Arc::new(handle.steering_hooks.clone()),
        ))
    }

    fn cancel(&self, token: CancellationToken) {
        token.cancel();
    }
}

/// Outcome of [`load_local_candle_model`]: an owned, loaded [`CandleRuntime`]
/// plus the runtime-minted [`ModelId`] and the capabilities the runtime
/// actually reports for the loaded model.
///
/// The runtime is returned BY VALUE (owned) so the caller decides ownership:
/// the app's single-load IPC moves it into one `Arc<dyn ModelRuntime>`; the
/// swarm production factory keeps the owning `Arc` inside its teardown closure
/// so dropping that `Arc` (the only strong reference) runs `Drop` and frees the
/// model — the D1 teardown contract.
pub struct LoadedCandleModel {
    pub runtime: CandleRuntime,
    pub model_id: ModelId,
    pub capabilities: ModelCapabilities,
}

/// Build + load a local candle model the same way the production single-load
/// IPC (`kernel_model_runtime_load`) does, factored so the swarm production
/// factory reuses the EXACT proven path instead of duplicating it.
///
/// Builds the permissive base [`LoadSpec`] (the candle arch-detection finalises
/// the real capability set), constructs a real [`CandleRuntime`], drives
/// `load()` (which verifies the artifact sha256 and fails loud on mismatch /
/// missing file), then reads back the capabilities the runtime reports for the
/// loaded model. No fakes, no placeholders: a genuine load or a typed
/// [`ModelRuntimeError`].
pub async fn load_local_candle_model(
    artifact_path: std::path::PathBuf,
    sha256_expected: String,
) -> Result<LoadedCandleModel, ModelRuntimeError> {
    // Permissive base capabilities; the candle arch-detection path
    // (transformer/mamba2/rwkv) finalises the real capability set, which we read
    // back from the runtime and surface as the authoritative record.
    let base_capabilities = ModelCapabilities {
        supports_lora: true,
        supports_kv_prefix_cache: true,
        supports_kv_quantization: KvQuantSupport::None,
        supports_activation_steering: true,
        supports_subquadratic: false,
        supports_speculative_draft: false,
        supports_eagle3: false,
        ..Default::default()
    };

    let spec = LoadSpec {
        artifact_path,
        sha256_expected,
        runtime_kind: RuntimeKind::Candle,
        sampling_defaults: crate::model_runtime::SamplingParams::default(),
        kv_cache_policy: crate::model_runtime::KvCachePolicy::default(),
        declared_capabilities: base_capabilities,
        provider: ProviderKind::Local,
        engine_origin: Some("candle".to_string()),
        external_engine_import: None,
    };

    let mut runtime = CandleRuntime::default();
    let model_id = runtime.load(spec).await?;
    let capabilities = runtime.capabilities(model_id)?.clone();
    Ok(LoadedCandleModel {
        runtime,
        model_id,
        capabilities,
    })
}

fn decode_expected_sha256(expected: &str) -> Result<[u8; 32], ModelRuntimeError> {
    let trimmed = expected.trim();
    let decoded = hex::decode(trimmed).map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "Candle expected artifact sha256 is not valid hex: {error}"
        ))
    })?;
    decoded.try_into().map_err(|bytes: Vec<u8>| {
        ModelRuntimeError::LoadError(format!(
            "Candle expected artifact sha256 decoded to {} bytes, expected 32",
            bytes.len()
        ))
    })
}

fn ensure_candle_capabilities_satisfy(
    required: &ModelCapabilities,
    actual: &ModelCapabilities,
) -> Result<(), ModelRuntimeError> {
    let required_flags = [
        (
            "supports_lora",
            required.supports_lora,
            actual.supports_lora,
        ),
        (
            "supports_kv_prefix_cache",
            required.supports_kv_prefix_cache,
            actual.supports_kv_prefix_cache,
        ),
        (
            "supports_activation_steering",
            required.supports_activation_steering,
            actual.supports_activation_steering,
        ),
        (
            "supports_subquadratic",
            required.supports_subquadratic,
            actual.supports_subquadratic,
        ),
        (
            "supports_speculative_draft",
            required.supports_speculative_draft,
            actual.supports_speculative_draft,
        ),
        (
            "supports_eagle3",
            required.supports_eagle3,
            actual.supports_eagle3,
        ),
        (
            "supports_embedding",
            required.supports_embedding,
            actual.supports_embedding,
        ),
    ];
    if let Some((name, _, _)) = required_flags
        .into_iter()
        .find(|(_, required, actual)| *required && !*actual)
    {
        return Err(ModelRuntimeError::LoadError(format!(
            "Candle attested load is missing required capability {name}"
        )));
    }
    if required.supports_kv_quantization != KvQuantSupport::None
        && actual.supports_kv_quantization != required.supports_kv_quantization
    {
        return Err(ModelRuntimeError::LoadError(format!(
            "Candle attested load KV quantization {:?} differs from required {:?}",
            actual.supports_kv_quantization, required.supports_kv_quantization
        )));
    }
    if let Some(required_dimension) = required.embedding_dimension {
        if actual.embedding_dimension != Some(required_dimension) {
            return Err(ModelRuntimeError::LoadError(format!(
                "Candle attested load embedding dimension {:?} differs from required {required_dimension}",
                actual.embedding_dimension
            )));
        }
    }
    Ok(())
}

pub fn validate_candle_load_spec(spec: &LoadSpec) -> Result<(), ModelRuntimeError> {
    validate_candle_load_spec_fields(spec)?;

    if !spec.artifact_path.is_file() {
        return Err(ModelRuntimeError::LoadError(format!(
            "CandleRuntime requires a regular model artifact file, got {}",
            spec.artifact_path.display()
        )));
    }

    let actual = sha256_file(&spec.artifact_path)?;
    if !actual.eq_ignore_ascii_case(spec.sha256_expected.trim()) {
        return Err(ModelRuntimeError::LoadError(format!(
            "candle artifact sha256 mismatch: expected {}, got {actual}",
            spec.sha256_expected
        )));
    }

    Ok(())
}

fn validate_candle_load_spec_fields(spec: &LoadSpec) -> Result<(), ModelRuntimeError> {
    if spec.runtime_kind != RuntimeKind::Candle {
        return Err(ModelRuntimeError::LoadError(format!(
            "CandleRuntime requires RuntimeKind::Candle, got {:?}",
            spec.runtime_kind
        )));
    }

    if spec.provider != ProviderKind::Local {
        return Err(ModelRuntimeError::LoadError(format!(
            "CandleRuntime accepts only local provider specs, got {:?}",
            spec.provider
        )));
    }

    Ok(())
}

pub fn candle_transformer_capabilities(declared: &ModelCapabilities) -> ModelCapabilities {
    ModelCapabilities {
        supports_lora: true,
        supports_kv_prefix_cache: false,
        supports_kv_quantization: KvQuantSupport::None,
        supports_activation_steering: declared.supports_activation_steering,
        supports_subquadratic: false,
        supports_speculative_draft: false,
        supports_eagle3: false,
        supports_embedding: declared.supports_embedding,
        embedding_dimension: declared.embedding_dimension,
    }
}

pub fn candle_mamba2_capabilities(_declared: &ModelCapabilities) -> ModelCapabilities {
    ModelCapabilities {
        // MT-115: the owned Mamba2 forward (mamba2.rs) routes in_proj/out_proj
        // through the LoRA delta engine, so LoRA is genuinely wired.
        supports_lora: true,
        supports_kv_prefix_cache: false,
        supports_kv_quantization: KvQuantSupport::None,
        // MT-089 / cross-cluster steering-ssm (honest declaration): activation
        // steering is NOT usable end-to-end for SSM. The forward exposes an
        // apply seam, but CandleRuntime::steering_hooks wires real-forward
        // CAPTURE (CandleRuntimeSteeringHookOps) only for the Transformer
        // backend; SSM falls through to the bare hooks whose capture() fails
        // closed (hooks.rs). Declaring true here was a lie that let the steering
        // capability gate pass for SSM and then fail closed at capture. Stays
        // false per the MT-116 deferral until SSM real-forward capture is wired
        // and identity-test correctness is proven.
        supports_activation_steering: false,
        supports_subquadratic: true,
        supports_speculative_draft: false,
        supports_eagle3: false,
        supports_embedding: false,
        embedding_dimension: None,
    }
}

pub fn candle_rwkv_capabilities(_declared: &ModelCapabilities) -> ModelCapabilities {
    ModelCapabilities {
        // MT-115: the owned RWKV v5/v6/v7 forwards route time-mix/channel-mix
        // projections through the LoRA delta engine, so LoRA is genuinely wired.
        supports_lora: true,
        supports_kv_prefix_cache: false,
        supports_kv_quantization: KvQuantSupport::None,
        // MT-089 / cross-cluster steering-ssm (honest): same as Mamba2 — SSM
        // activation-steering CAPTURE fails closed via the adapter (real-forward
        // hooks are Transformer-only), so steering is not usable end-to-end.
        // False until SSM capture is wired (MT-116 deferral).
        supports_activation_steering: false,
        supports_subquadratic: true,
        supports_speculative_draft: false,
        supports_eagle3: false,
        supports_embedding: false,
        embedding_dimension: None,
    }
}

/// CRIT-1 / MT-088: bind the live SSM model behind the state-vector ops.
/// The `state_source` is cloned from the same
/// `Arc<Mutex<Box<dyn TransformerModel>>>` the backend holds, so
/// `prefix_commit` extracts the current live state and `prefix_restore`
/// writes back into the same model the generate path mutates.
#[cfg(feature = "candle-runtime-engine")]
fn state_vector_handle_with_live_source(
    model_id: ModelId,
    artifact_sha256: &str,
    variant: SSMStateVariant,
    state_source: Arc<dyn SsmStateSource>,
) -> Result<StateVectorHandle, ModelRuntimeError> {
    let handle_id = format!("candle:{model_id}:state_vector:{variant}");
    StateVectorHandle::new_in_memory_with_source(
        handle_id,
        model_id,
        artifact_sha256,
        variant,
        state_source,
    )
}

pub fn sha256_file(path: &Path) -> Result<String, ModelRuntimeError> {
    let mut file = File::open(path).map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "failed to open Candle artifact {}: {error}",
            path.display()
        ))
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|error| {
            ModelRuntimeError::LoadError(format!(
                "failed to read Candle artifact {}: {error}",
                path.display()
            ))
        })?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn single_error_stream(error: ModelRuntimeError) -> TokenStream {
    Box::pin(stream::once(async move { Err(error) }))
}

#[cfg(feature = "candle-runtime-engine")]
struct CandleRuntimeSteeringHookOps {
    model: Arc<Mutex<Box<dyn TransformerModel>>>,
    codec: Arc<dyn CandleGenerationCodec>,
    hooks: CandleSteeringHooks,
}

#[cfg(feature = "candle-runtime-engine")]
#[async_trait]
impl SteeringHookOps for CandleRuntimeSteeringHookOps {
    async fn capture(&self, spec: CaptureSpec) -> Result<CaptureResult, ModelRuntimeError> {
        if spec.hook_point != HookPoint::ResidStream {
            let capability = match spec.hook_point {
                HookPoint::ResidStream => "resid_stream",
                HookPoint::MlpOut => "mlp_out",
                HookPoint::AttnOut => "attn_out",
            };
            return Err(ModelRuntimeError::CapabilityNotSupported {
                capability: format!("{capability} hook point"),
                adapter: "candle_hooks".to_string(),
            });
        }
        if spec.prompts.is_empty() {
            return Err(ModelRuntimeError::SteeringHookError(
                "capture spec requires at least one prompt".to_string(),
            ));
        }
        self.hooks.begin_real_capture(&spec.layers)?;
        let run_result = self.run_capture_for_prompts(&spec.prompts);
        let capture_result = self.hooks.finish_real_capture(&spec.layers);
        run_result?;
        capture_result
    }

    async fn register_vector(
        &self,
        vector: SteeringVector,
    ) -> Result<SteeringVectorId, ModelRuntimeError> {
        self.hooks.register_vector(vector).await
    }

    fn list_vectors(&self) -> Vec<SteeringVectorMeta> {
        self.hooks.list_vectors()
    }

    async fn set_active(&self, ids: Vec<SteeringVectorId>) -> Result<(), ModelRuntimeError> {
        self.hooks.set_active(ids).await
    }

    async fn unregister(&self, id: SteeringVectorId) -> Result<(), ModelRuntimeError> {
        self.hooks.unregister(id).await
    }
}

#[cfg(feature = "candle-runtime-engine")]
impl CandleRuntimeSteeringHookOps {
    fn run_capture_for_prompts(&self, prompts: &[String]) -> Result<(), ModelRuntimeError> {
        for prompt in prompts {
            let input_ids = self.codec.encode_prompt(prompt)?;
            if input_ids.is_empty() {
                return Err(ModelRuntimeError::SteeringHookError(
                    "Candle tokenizer produced no prompt tokens for capture".to_string(),
                ));
            }
            let mut model = self.model.lock().map_err(|_| {
                ModelRuntimeError::SteeringHookError(
                    "Candle transformer model lock is poisoned".to_string(),
                )
            })?;
            model.reset_generation_state()?;
            let device = model.device();
            let input = candle_core::Tensor::new(input_ids.as_slice(), &device)
                .and_then(|tensor| tensor.reshape((1, input_ids.len())))
                .map_err(|error| {
                    ModelRuntimeError::SteeringHookError(format!(
                        "Candle capture input tensor failed: {error}"
                    ))
                })?;
            let _ = model.forward(&input, &self.hooks, &[], &[])?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_runtime::{KvCachePolicy, KvQuantSupport, SamplingParams};
    use std::fs;

    #[test]
    fn candle_adapter_default_constructs_with_cpu_selection() {
        let runtime = CandleRuntime::default();

        assert_eq!(
            runtime.device_selection().selected(),
            super::super::device::CandleDeviceKind::Cpu
        );
        assert_eq!(runtime.tokenizer_cache_len(), 0);
    }

    #[test]
    fn candle_adapter_load_spec_validation_preserves_uuid_v7_mint_contract() {
        let id = ModelId::new_v7();
        assert_eq!(id.as_uuid().get_version_num(), 7);
    }

    #[test]
    fn mt013_transformer_and_ssm_capability_finalizers_are_architecture_truthful() {
        let declared = ModelCapabilities {
            supports_activation_steering: true,
            supports_embedding: true,
            embedding_dimension: Some(999),
            ..Default::default()
        };

        let transformer = candle_transformer_capabilities(&declared);
        assert!(transformer.supports_activation_steering);
        assert!(transformer.supports_embedding);
        assert_eq!(transformer.embedding_dimension, Some(999));

        for ssm in [
            candle_mamba2_capabilities(&declared),
            candle_rwkv_capabilities(&declared),
        ] {
            assert!(!ssm.supports_activation_steering);
            assert!(ssm.supports_subquadratic);
            assert!(!ssm.supports_embedding);
            assert_eq!(ssm.embedding_dimension, None);
        }
    }

    #[test]
    fn candle_adapter_validation_rejects_wrong_runtime() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("model.safetensors");
        fs::write(&path, b"weights").expect("write weights");
        let spec = LoadSpec {
            artifact_path: path,
            sha256_expected: "abc".to_string(),
            runtime_kind: RuntimeKind::LlamaCpp,
            sampling_defaults: SamplingParams::default(),
            kv_cache_policy: KvCachePolicy::Default {
                quant: KvQuantSupport::Q4,
                prefix_cache_ttl_seconds: 0,
                max_bytes: None,
            },
            declared_capabilities: ModelCapabilities::default(),
            provider: ProviderKind::Local,
            engine_origin: None,
            external_engine_import: None,
        };

        let error = validate_candle_load_spec(&spec).expect_err("wrong runtime rejected");
        assert!(error.to_string().contains("CandleRuntime"), "{error}");
    }

    #[cfg(feature = "candle-runtime-engine")]
    #[tokio::test]
    async fn mt013_wrong_weights_hash_fails_before_any_cache_publication() {
        let fixture = TinyCandleBundle::new(true);
        let mut runtime = CandleRuntime::default();
        let error = runtime
            .load(fixture.load_spec("00".repeat(32)))
            .await
            .expect_err("wrong expected digest must fail closed");

        assert!(error.to_string().contains("sha256 mismatch"), "{error}");
        assert!(runtime.models.is_empty());
        assert_eq!(runtime.tokenizer_cache_len(), 0);
    }

    #[test]
    fn mt013_production_attested_load_has_no_path_reopen_or_raw_loader_bypass() {
        let source = include_str!("adapter.rs");
        let start = source
            .find("pub async fn load_attested(")
            .expect("official attested Candle load boundary");
        let end = source[start..]
            .find("impl Default for CandleRuntime")
            .map(|offset| start + offset)
            .expect("attested Candle load boundary end");
        let production_load = &source[start..end];

        for forbidden in [
            "from_mmaped_safetensors",
            "Tokenizer::from_file",
            "artifact_config_declares",
            "sha256_file",
        ] {
            assert!(
                !production_load.contains(forbidden),
                "CandleRuntime::load_attested must consume the immutable captured bundle, not {forbidden}"
            );
        }
        assert!(production_load.contains("capture_candle_artifact"));
        assert!(production_load.contains("prepare_captured_artifact"));
        assert!(production_load.contains("attest_and_publish"));

        let trait_load_start = source
            .find("async fn load(&mut self, spec: LoadSpec)")
            .expect("ModelRuntime load implementation");
        let trait_load_end = source[trait_load_start..]
            .find("async fn unload")
            .map(|offset| trait_load_start + offset)
            .expect("ModelRuntime unload boundary");
        assert!(source[trait_load_start..trait_load_end].contains("load_attested"));
    }

    #[cfg(feature = "candle-runtime-engine")]
    #[tokio::test]
    async fn mt013_source_replacement_after_capture_cannot_change_loaded_bytes() {
        let fixture = TinyCandleBundle::new(true);
        let mut spec =
            fixture.load_spec(sha256_file(&fixture.weights).expect("hash fixture weights"));
        spec.declared_capabilities.supports_embedding = true;
        spec.declared_capabilities.embedding_dimension = Some(999);
        let captured = capture_candle_artifact(&spec).expect("capture exact bundle");
        let expected_receipt = captured.receipt.clone();

        fs::write(&fixture.weights, b"replacement weights are not safetensors")
            .expect("replace source weights after capture");
        fs::write(&fixture.config, br#"{"model_type":"replaced"}"#)
            .expect("replace source config after capture");
        fs::write(&fixture.tokenizer, b"not a tokenizer")
            .expect("replace source tokenizer after capture");

        let mut runtime = CandleRuntime::default();
        let prepared = runtime
            .prepare_captured_artifact(&spec, captured, Instant::now())
            .expect("captured bytes still construct the real Candle model");
        let attested = runtime
            .attest_and_publish(&spec, prepared, &ModelCapabilities::default())
            .expect("captured bytes pass attestation and publish atomically");
        let id = attested.model_id;
        assert_eq!(attested.artifact_integrity, expected_receipt);
        assert_eq!(
            runtime.artifact_integrity(id).unwrap(),
            RuntimeArtifactIntegrityReceipt::from(expected_receipt)
        );
        assert_eq!(runtime.tokenizer_cache_len(), 1);

        let score = runtime
            .score(id, vec![1, 2])
            .await
            .expect("real forward uses captured weights after source replacement");
        assert_eq!(score.token_logprobs.len(), 1);
    }

    #[cfg(feature = "candle-runtime-engine")]
    #[test]
    fn mt013_config_and_tokenizer_changes_each_change_bundle_digest() {
        let fixture = TinyCandleBundle::new(true);
        let weights_sha256 = sha256_file(&fixture.weights).expect("hash fixture weights");
        let spec = fixture.load_spec(weights_sha256);
        let original_config = fs::read(&fixture.config).expect("read original config");
        let original_tokenizer = fs::read(&fixture.tokenizer).expect("read original tokenizer");
        let original = capture_candle_artifact(&spec)
            .expect("capture original")
            .receipt;

        let mut changed_config: serde_json::Value =
            serde_json::from_slice(&original_config).expect("decode config fixture");
        changed_config["bos_token_id"] = serde_json::json!(1);
        fs::write(
            &fixture.config,
            serde_json::to_vec(&changed_config).expect("encode changed config"),
        )
        .expect("write changed config");
        let config_changed = capture_candle_artifact(&spec)
            .expect("capture config change")
            .receipt;

        fs::write(&fixture.config, &original_config).expect("restore exact config bytes");
        let mut changed_tokenizer = original_tokenizer.clone();
        changed_tokenizer.push(b'\n');
        fs::write(&fixture.tokenizer, changed_tokenizer).expect("write changed tokenizer");
        let tokenizer_changed = capture_candle_artifact(&spec)
            .expect("capture tokenizer change")
            .receipt;

        assert_eq!(original.weights, config_changed.weights);
        assert_ne!(original.config, config_changed.config);
        assert_eq!(original.tokenizer, config_changed.tokenizer);
        assert_ne!(original.bundle_sha256, config_changed.bundle_sha256);

        assert_eq!(original.weights, tokenizer_changed.weights);
        assert_eq!(original.config, tokenizer_changed.config);
        assert_ne!(original.tokenizer, tokenizer_changed.tokenizer);
        assert_ne!(original.bundle_sha256, tokenizer_changed.bundle_sha256);
    }

    #[cfg(feature = "candle-runtime-engine")]
    #[tokio::test]
    async fn mt013_missing_tokenizer_fails_required_embedding_without_cache_publication() {
        let fixture = TinyCandleBundle::new(false);
        let mut spec =
            fixture.load_spec(sha256_file(&fixture.weights).expect("hash fixture weights"));
        spec.declared_capabilities.supports_embedding = true;
        spec.declared_capabilities.embedding_dimension = Some(999);
        let missing_receipt = capture_candle_artifact(&spec)
            .expect("capture bundle without tokenizer")
            .receipt;
        assert!(missing_receipt.tokenizer.is_none());

        let mut runtime = CandleRuntime::default();
        let required_embedding = ModelCapabilities {
            supports_embedding: true,
            embedding_dimension: Some(4),
            ..Default::default()
        };
        let error = runtime
            .load_attested(spec, &required_embedding)
            .await
            .expect_err("required embedding must fail without a captured tokenizer");
        assert!(error.to_string().contains("supports_embedding"), "{error}");
        assert!(runtime.models.is_empty());
        assert_eq!(runtime.tokenizer_cache_len(), 0);

        write_test_tokenizer(&fixture.tokenizer);
        let mut present_spec =
            fixture.load_spec(sha256_file(&fixture.weights).expect("hash fixture weights"));
        present_spec.declared_capabilities.supports_embedding = true;
        present_spec.declared_capabilities.embedding_dimension = Some(999);
        let present_receipt = capture_candle_artifact(&present_spec)
            .expect("capture bundle with tokenizer")
            .receipt;
        assert!(present_receipt.tokenizer.is_some());
        assert_ne!(missing_receipt.bundle_sha256, present_receipt.bundle_sha256);

        let mut present_runtime = CandleRuntime::default();
        let present_attested = present_runtime
            .load_attested(present_spec, &required_embedding)
            .await
            .expect("tokenizer-backed embedding model loads");
        let actual = &present_attested.capabilities;
        assert!(actual.supports_embedding);
        assert_eq!(actual.embedding_dimension, Some(4));
        assert_eq!(present_runtime.models.len(), 1);
        assert_eq!(present_runtime.tokenizer_cache_len(), 1);
    }

    #[cfg(feature = "candle-runtime-engine")]
    #[tokio::test]
    async fn mt013_model_construction_failure_leaves_model_and_tokenizer_caches_empty() {
        let fixture = TinyCandleBundle::new(true);
        fs::write(
            &fixture.weights,
            b"validly captured but invalid safetensors",
        )
        .expect("write invalid weights");
        let spec = fixture.load_spec(sha256_file(&fixture.weights).expect("hash fixture weights"));
        let mut runtime = CandleRuntime::default();

        let error = runtime
            .load(spec)
            .await
            .expect_err("invalid safetensors must fail model construction");
        assert!(error.to_string().contains("safetensors"), "{error}");
        assert!(runtime.models.is_empty());
        assert_eq!(runtime.tokenizer_cache_len(), 0);
    }

    #[cfg(feature = "candle-runtime-engine")]
    struct TinyCandleBundle {
        _root: tempfile::TempDir,
        weights: std::path::PathBuf,
        config: std::path::PathBuf,
        tokenizer: std::path::PathBuf,
    }

    #[cfg(feature = "candle-runtime-engine")]
    impl TinyCandleBundle {
        fn new(with_tokenizer: bool) -> Self {
            let root = tempfile::tempdir().expect("tiny Candle bundle tempdir");
            let weights = root.path().join("model.safetensors");
            let config = root.path().join("config.json");
            let tokenizer = root.path().join("tokenizer.json");
            let config_value = serde_json::json!({
                "hidden_size": 4,
                "intermediate_size": 8,
                "vocab_size": 8,
                "num_hidden_layers": 1,
                "num_attention_heads": 1,
                "num_key_value_heads": 1,
                "rms_norm_eps": 0.00001,
                "rope_theta": 10000.0,
                "bos_token_id": null,
                "eos_token_id": 2,
                "rope_scaling": null,
                "max_position_embeddings": 16,
                "tie_word_embeddings": false
            });
            fs::write(
                &config,
                serde_json::to_vec(&config_value).expect("encode tiny config"),
            )
            .expect("write tiny config");

            let (runtime_config, _) =
                decode_llama_config_value(&config_value).expect("decode tiny config");
            let varmap = candle_nn::VarMap::new();
            let vb = candle_nn::VarBuilder::from_varmap(
                &varmap,
                candle_core::DType::F32,
                &candle_core::Device::Cpu,
            );
            let _model = CandleLlamaModel::from_varbuilder_for_model(
                ModelId::new_v7(),
                runtime_config,
                vb,
                &candle_core::Device::Cpu,
            )
            .expect("construct tiny Candle weights");
            varmap.save(&weights).expect("save tiny safetensors");
            if with_tokenizer {
                write_test_tokenizer(&tokenizer);
            }

            Self {
                _root: root,
                weights,
                config,
                tokenizer,
            }
        }

        fn load_spec(&self, sha256_expected: String) -> LoadSpec {
            LoadSpec {
                artifact_path: self.weights.clone(),
                sha256_expected,
                runtime_kind: RuntimeKind::Candle,
                sampling_defaults: SamplingParams::default(),
                kv_cache_policy: KvCachePolicy::Default {
                    quant: KvQuantSupport::None,
                    prefix_cache_ttl_seconds: 0,
                    max_bytes: None,
                },
                declared_capabilities: ModelCapabilities::default(),
                provider: ProviderKind::Local,
                engine_origin: Some("candle".to_string()),
                external_engine_import: None,
            }
        }
    }

    #[cfg(feature = "candle-runtime-engine")]
    fn write_test_tokenizer(path: &Path) {
        fs::write(
            path,
            br#"{"version":"1.0","truncation":null,"padding":null,"added_tokens":[],"normalizer":null,"pre_tokenizer":null,"post_processor":null,"decoder":null,"model":{"type":"WordLevel","vocab":{"[UNK]":0,"hello":1},"unk_token":"[UNK]"}}"#,
        )
        .expect("write valid tokenizer fixture");
    }
}

use std::{
    collections::HashMap,
    fs::File,
    io::Read,
    path::Path,
    sync::{Arc, Mutex},
    time::Instant,
};

use async_trait::async_trait;
use futures::stream;
use sha2::{Digest, Sha256};

use super::{
    device::{select_candle_device, CandleDevicePreference, CandleDeviceSelection},
    hooks::CandleSteeringHooks,
    state_vector::{SSMStateVariant, StateVectorHandle},
    tokenizer::{
        cache_tokenizer_if_present, tokenizer_json_path_for_artifact, CandleTokenizerCache,
    },
};
#[cfg(feature = "candle-runtime-engine")]
use super::{
    generate::{candle_generate_stream, CandleGenerationCodec, TokenizerGenerationCodec},
    mamba2::{artifact_config_declares_mamba2, CandleMamba2Model},
    rwkv_v5::{
        artifact_config_declares_rwkv_v5, artifact_config_declares_unversioned_rwkv,
        CandleRwkvV5Model,
    },
    rwkv_v6::{artifact_config_declares_rwkv_v6, CandleRwkvV6Model},
    rwkv_v7::{artifact_config_declares_rwkv_v7, CandleRwkvV7Model},
    ssm_state::{LockedSsmStateSource, SsmStateSource},
    transformer::{CandleLlamaModel, TransformerModel},
};
use crate::model_runtime::{
    CancellationToken, Embedding, GenerateRequest, HookPoint, KvCacheHandle, KvQuantSupport,
    LoadSpec, LoraStackHandle, ModelCapabilities, ModelId, ModelRuntime, ModelRuntimeError,
    ProviderKind, RuntimeKind, Score, SteeringHookHandle, SteeringHookOps, SteeringVector,
    SteeringVectorId, SteeringVectorMeta, TokenStream,
};
#[cfg(feature = "candle-runtime-engine")]
use crate::model_runtime::{CaptureResult, CaptureSpec};

pub const CANDLE_NATIVE_FEATURE_DISABLED: &str =
    "Candle native engine feature disabled; enable candle-runtime-engine";

pub struct CandleRuntime {
    models: HashMap<ModelId, CandleModelHandle>,
    device_selection: CandleDeviceSelection,
    tokenizer_cache: CandleTokenizerCache,
    #[cfg(feature = "candle-runtime-engine")]
    native_device: candle_core::Device,
}

impl CandleRuntime {
    pub fn with_device_preference(preference: CandleDevicePreference) -> Self {
        let device_selection = select_candle_device(preference);
        Self {
            models: HashMap::new(),
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
}

impl Default for CandleRuntime {
    fn default() -> Self {
        Self::with_device_preference(CandleDevicePreference::Auto)
    }
}

#[allow(dead_code)]
struct CandleModelHandle {
    backend: CandleModelBackend,
    declared_capabilities: ModelCapabilities,
    cancel: CancellationToken,
    load_duration_ms: u128,
    tokenizer_path: std::path::PathBuf,
    device_selection: CandleDeviceSelection,
    steering_hooks: CandleSteeringHooks,
    state_vector: Option<StateVectorHandle>,
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

#[async_trait]
impl ModelRuntime for CandleRuntime {
    async fn load(&mut self, spec: LoadSpec) -> Result<ModelId, ModelRuntimeError> {
        validate_candle_load_spec(&spec)?;

        #[cfg(not(feature = "candle-runtime-engine"))]
        {
            Err(ModelRuntimeError::LoadError(
                CANDLE_NATIVE_FEATURE_DISABLED.to_string(),
            ))
        }

        #[cfg(feature = "candle-runtime-engine")]
        {
            let started = Instant::now();
            let id = ModelId::new_v7();
            cache_tokenizer_if_present(&mut self.tokenizer_cache, id, &spec.artifact_path)?;
            let tokenizer_path = tokenizer_json_path_for_artifact(&spec.artifact_path);
            let artifact_sha256 = spec.sha256_expected.trim().to_ascii_lowercase();
            let _ = self.native_binding_marker();
            let is_mamba2 = artifact_config_declares_mamba2(&spec.artifact_path)?;
            let is_rwkv_v7 = artifact_config_declares_rwkv_v7(&spec.artifact_path)?;
            let is_rwkv_v6 = artifact_config_declares_rwkv_v6(&spec.artifact_path)?;
            let is_rwkv_v5 = artifact_config_declares_rwkv_v5(&spec.artifact_path)?;
            let is_unversioned_rwkv =
                artifact_config_declares_unversioned_rwkv(&spec.artifact_path)?;
            let load_duration_ms = started.elapsed().as_millis().max(1);
            if is_mamba2 {
                let model = CandleMamba2Model::load_safetensors_for_model(
                    id,
                    &spec.artifact_path,
                    &self.native_device,
                )?;
                let residual_width = model.hidden_dim() as usize;
                let boxed: Box<dyn TransformerModel> = Box::new(model);
                let model_arc = Arc::new(Mutex::new(boxed));
                let state_source: Arc<dyn SsmStateSource> =
                    Arc::new(LockedSsmStateSource::new(Arc::clone(&model_arc)));
                let state_vector = state_vector_handle_with_live_source(
                    id,
                    &artifact_sha256,
                    SSMStateVariant::Mamba2,
                    state_source,
                )?;
                self.models.insert(
                    id,
                    CandleModelHandle {
                        backend: CandleModelBackend::Mamba2 { model: model_arc },
                        declared_capabilities: candle_mamba2_capabilities(
                            &spec.declared_capabilities,
                        ),
                        cancel: CancellationToken::new(),
                        load_duration_ms,
                        tokenizer_path,
                        device_selection: self.device_selection.clone(),
                        steering_hooks: CandleSteeringHooks::new_for_model(id, residual_width),
                        state_vector: Some(state_vector),
                    },
                );
            } else if is_rwkv_v7 {
                let model = CandleRwkvV7Model::load_safetensors_for_model(
                    id,
                    &spec.artifact_path,
                    &self.native_device,
                )?;
                let residual_width = model.hidden_dim() as usize;
                let boxed: Box<dyn TransformerModel> = Box::new(model);
                let model_arc = Arc::new(Mutex::new(boxed));
                let state_source: Arc<dyn SsmStateSource> =
                    Arc::new(LockedSsmStateSource::new(Arc::clone(&model_arc)));
                let state_vector = state_vector_handle_with_live_source(
                    id,
                    &artifact_sha256,
                    SSMStateVariant::RwkvV7,
                    state_source,
                )?;
                self.models.insert(
                    id,
                    CandleModelHandle {
                        backend: CandleModelBackend::RwkvV7 { model: model_arc },
                        declared_capabilities: candle_rwkv_capabilities(
                            &spec.declared_capabilities,
                        ),
                        cancel: CancellationToken::new(),
                        load_duration_ms,
                        tokenizer_path,
                        device_selection: self.device_selection.clone(),
                        steering_hooks: CandleSteeringHooks::new_for_model(id, residual_width),
                        state_vector: Some(state_vector),
                    },
                );
            } else if is_rwkv_v6 {
                let model = CandleRwkvV6Model::load_safetensors_for_model(
                    id,
                    &spec.artifact_path,
                    &self.native_device,
                )?;
                let residual_width = model.hidden_dim() as usize;
                let boxed: Box<dyn TransformerModel> = Box::new(model);
                let model_arc = Arc::new(Mutex::new(boxed));
                let state_source: Arc<dyn SsmStateSource> =
                    Arc::new(LockedSsmStateSource::new(Arc::clone(&model_arc)));
                let state_vector = state_vector_handle_with_live_source(
                    id,
                    &artifact_sha256,
                    SSMStateVariant::RwkvV6,
                    state_source,
                )?;
                self.models.insert(
                    id,
                    CandleModelHandle {
                        backend: CandleModelBackend::RwkvV6 { model: model_arc },
                        declared_capabilities: candle_rwkv_capabilities(
                            &spec.declared_capabilities,
                        ),
                        cancel: CancellationToken::new(),
                        load_duration_ms,
                        tokenizer_path,
                        device_selection: self.device_selection.clone(),
                        steering_hooks: CandleSteeringHooks::new_for_model(id, residual_width),
                        state_vector: Some(state_vector),
                    },
                );
            } else if is_rwkv_v5 {
                let model = CandleRwkvV5Model::load_safetensors_for_model(
                    id,
                    &spec.artifact_path,
                    &self.native_device,
                )?;
                let residual_width = model.hidden_dim() as usize;
                let boxed: Box<dyn TransformerModel> = Box::new(model);
                let model_arc = Arc::new(Mutex::new(boxed));
                let state_source: Arc<dyn SsmStateSource> =
                    Arc::new(LockedSsmStateSource::new(Arc::clone(&model_arc)));
                let state_vector = state_vector_handle_with_live_source(
                    id,
                    &artifact_sha256,
                    SSMStateVariant::RwkvV5,
                    state_source,
                )?;
                self.models.insert(
                    id,
                    CandleModelHandle {
                        backend: CandleModelBackend::RwkvV5 { model: model_arc },
                        declared_capabilities: candle_rwkv_capabilities(
                            &spec.declared_capabilities,
                        ),
                        cancel: CancellationToken::new(),
                        load_duration_ms,
                        tokenizer_path,
                        device_selection: self.device_selection.clone(),
                        steering_hooks: CandleSteeringHooks::new_for_model(id, residual_width),
                        state_vector: Some(state_vector),
                    },
                );
            } else if is_unversioned_rwkv {
                return Err(ModelRuntimeError::LoadError(
                    "Candle RWKV config declares generic RWKV without a v5, v6, or v7 marker; use model_type rwkv5/rwkv6/rwkv7 or a versioned architecture marker"
                        .to_string(),
                ));
            } else {
                let model = CandleLlamaModel::load_safetensors_for_model(
                    id,
                    &spec.artifact_path,
                    &self.native_device,
                )?;
                let residual_width = model.hidden_dim() as usize;
                self.models.insert(
                    id,
                    CandleModelHandle {
                        backend: CandleModelBackend::Transformer {
                            model: Arc::new(Mutex::new(Box::new(model))),
                        },
                        declared_capabilities: candle_transformer_capabilities(
                            &spec.declared_capabilities,
                        ),
                        cancel: CancellationToken::new(),
                        load_duration_ms,
                        tokenizer_path,
                        device_selection: self.device_selection.clone(),
                        steering_hooks: CandleSteeringHooks::new_for_model(id, residual_width),
                        state_vector: None,
                    },
                );
            }
            Ok(id)
        }
    }

    async fn unload(&mut self, id: ModelId) -> Result<(), ModelRuntimeError> {
        self.models.remove(&id).map(|_| ()).ok_or_else(|| {
            ModelRuntimeError::UnloadError(format!("candle model is not loaded: {id}"))
        })
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
                    candle_generate_stream(
                        model.clone(),
                        Arc::new(TokenizerGenerationCodec::new(tokenizer)),
                        handle.steering_hooks.clone(),
                        req,
                        handle.cancel.clone(),
                    )
                }
                _ => single_error_stream(Self::not_implemented("candle_generate")),
            }
        }

        #[cfg(not(feature = "candle-runtime-engine"))]
        {
            single_error_stream(Self::not_implemented("candle_generate"))
        }
    }

    async fn score(&self, id: ModelId, _sequence: Vec<u32>) -> Result<Score, ModelRuntimeError> {
        if !self.models.contains_key(&id) {
            return Err(ModelRuntimeError::ScoreError(Self::not_loaded_message(id)));
        }
        Err(Self::not_implemented("candle_score"))
    }

    async fn embed(&self, id: ModelId, _text: &str) -> Result<Embedding, ModelRuntimeError> {
        if !self.models.contains_key(&id) {
            return Err(ModelRuntimeError::EmbedError(Self::not_loaded_message(id)));
        }
        Err(Self::not_implemented("candle_embed"))
    }

    fn capabilities(&self, id: ModelId) -> Result<&ModelCapabilities, ModelRuntimeError> {
        self.models
            .get(&id)
            .map(|handle| &handle.declared_capabilities)
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
        if !handle.declared_capabilities.supports_activation_steering {
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

pub fn validate_candle_load_spec(spec: &LoadSpec) -> Result<(), ModelRuntimeError> {
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

pub fn candle_transformer_capabilities(declared: &ModelCapabilities) -> ModelCapabilities {
    ModelCapabilities {
        supports_lora: true,
        supports_kv_prefix_cache: false,
        supports_kv_quantization: KvQuantSupport::None,
        supports_activation_steering: declared.supports_activation_steering,
        supports_subquadratic: false,
        supports_speculative_draft: false,
        supports_eagle3: false,
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
}

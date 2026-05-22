use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Instant,
};

use async_trait::async_trait;

use super::{
    context::LlamaCppContext,
    generate::{
        generation_preflight, single_error_stream, single_token_stream, terminal_token,
        GeneratePreflight,
    },
    gguf_loader::{self, LlamaCppLoadConfig},
    kv_cache_impl::LlamaCppKvCache,
    lora_impl::LlamaCppLoraStack,
    perf_stats::LlamaCppPerfStats,
    speculative::SpeculativeStats,
    tokenizer_cache::TokenizerCache,
};
use crate::{
    flight_recorder::FlightRecorder,
    model_runtime::{
        CancellationToken, Embedding, FinishReason, GenerateRequest, KvCacheHandle, KvCacheOps,
        KvCachePolicy, KvQuantSupport, LoadSpec, LoraStackHandle, ModelCapabilities, ModelId,
        ModelRuntime, ModelRuntimeError, Score, SteeringHookHandle, TokenStream,
    },
};

pub struct LlamaCppRuntime {
    models: HashMap<ModelId, LlamaModelHandle>,
    _default_kv_policy: KvCachePolicy,
    tokenizer_cache: TokenizerCache,
    load_config: LlamaCppLoadConfig,
    flight_recorder: Option<Arc<dyn FlightRecorder>>,
}

impl LlamaCppRuntime {
    pub fn new(default_kv_policy: KvCachePolicy) -> Self {
        Self::with_load_config(default_kv_policy, LlamaCppLoadConfig::default())
    }

    pub fn with_load_config(
        default_kv_policy: KvCachePolicy,
        load_config: LlamaCppLoadConfig,
    ) -> Self {
        Self {
            models: HashMap::new(),
            _default_kv_policy: default_kv_policy,
            tokenizer_cache: TokenizerCache::default(),
            load_config,
            flight_recorder: None,
        }
    }

    pub fn with_flight_recorder(
        default_kv_policy: KvCachePolicy,
        flight_recorder: Arc<dyn FlightRecorder>,
    ) -> Self {
        Self {
            flight_recorder: Some(flight_recorder),
            ..Self::new(default_kv_policy)
        }
    }

    pub fn tokenizer_cache(&self) -> &TokenizerCache {
        &self.tokenizer_cache
    }

    pub fn load_config(&self) -> &LlamaCppLoadConfig {
        &self.load_config
    }

    pub fn load_duration_ms(&self, id: ModelId) -> Result<u128, ModelRuntimeError> {
        self.models
            .get(&id)
            .map(|handle| handle.load_duration_ms)
            .ok_or_else(|| {
                ModelRuntimeError::LoadError(format!("llama.cpp model is not loaded: {id}"))
            })
    }

    pub fn llama_cpp_kv_cache(
        &self,
        id: ModelId,
    ) -> Result<Arc<LlamaCppKvCache>, ModelRuntimeError> {
        self.models
            .get(&id)
            .map(|handle| handle.kv_cache.clone())
            .ok_or_else(|| {
                ModelRuntimeError::LoadError(format!("llama.cpp model is not loaded: {id}"))
            })
    }

    pub fn llama_cpp_lora_stack(
        &self,
        id: ModelId,
    ) -> Result<Arc<LlamaCppLoraStack>, ModelRuntimeError> {
        self.models
            .get(&id)
            .map(|handle| handle.lora_stack.clone())
            .ok_or_else(|| {
                ModelRuntimeError::LoadError(format!("llama.cpp model is not loaded: {id}"))
            })
    }

    pub fn last_speculative_stats(
        &self,
        id: ModelId,
    ) -> Result<Option<super::speculative::SpeculativeStats>, ModelRuntimeError> {
        let handle = self.models.get(&id).ok_or_else(|| {
            ModelRuntimeError::LoadError(format!("llama.cpp model is not loaded: {id}"))
        })?;
        let guard = handle.speculative_stats.lock().map_err(|error| {
            ModelRuntimeError::GenerateError(format!(
                "llama.cpp speculative stats lock poisoned: {error}"
            ))
        })?;
        Ok(*guard)
    }

    pub fn perf_stats(&self, id: ModelId) -> Result<LlamaCppPerfStats, ModelRuntimeError> {
        let handle = self.models.get(&id).ok_or_else(|| {
            ModelRuntimeError::LoadError(format!("llama.cpp model is not loaded: {id}"))
        })?;
        let guard = handle.perf_stats.lock().map_err(|error| {
            ModelRuntimeError::GenerateError(format!("llama.cpp perf stats lock poisoned: {error}"))
        })?;
        Ok(guard.clone())
    }

    pub fn tokenize_prompt(
        &self,
        id: ModelId,
        prompt: &str,
    ) -> Result<Vec<u32>, ModelRuntimeError> {
        let handle = self.models.get(&id).ok_or_else(|| {
            ModelRuntimeError::LoadError(format!("llama.cpp model is not loaded: {id}"))
        })?;
        handle.context.tokenize_prompt(prompt)
    }

    fn not_implemented(operation: &str) -> ModelRuntimeError {
        ModelRuntimeError::CapabilityNotSupported {
            capability: format!("{operation} not implemented"),
            adapter: "llama_cpp_mt072_scaffold_not_implemented".to_string(),
        }
    }
}

impl Default for LlamaCppRuntime {
    fn default() -> Self {
        Self::new(KvCachePolicy::default())
    }
}

struct LlamaModelHandle {
    context: LlamaCppContext,
    declared_capabilities: ModelCapabilities,
    cancel: CancellationToken,
    kv_cache: Arc<LlamaCppKvCache>,
    lora_stack: Arc<LlamaCppLoraStack>,
    load_duration_ms: u128,
    speculative_stats: Arc<Mutex<Option<super::speculative::SpeculativeStats>>>,
    generation_epoch: Arc<AtomicU64>,
    perf_stats: Arc<Mutex<LlamaCppPerfStats>>,
}

#[async_trait]
impl ModelRuntime for LlamaCppRuntime {
    async fn load(&mut self, spec: LoadSpec) -> Result<ModelId, ModelRuntimeError> {
        let started = Instant::now();
        let mut declared_capabilities = spec.declared_capabilities.clone();
        declared_capabilities.supports_eagle3 = false;
        let (initial_quantization, prefix_cache_ttl_seconds, max_bytes) = kv_policy_defaults(
            &spec.kv_cache_policy,
            declared_capabilities.supports_kv_quantization,
        );
        let sha256_scope = spec.sha256_expected.trim().to_ascii_lowercase();
        let id = ModelId::new_v7();
        let context = gguf_loader::load_gguf_context(&spec, &self.load_config)?;
        self.tokenizer_cache.get_or_parse(id, &spec.artifact_path)?;
        let base_model_tag = llama_cpp_base_model_tag(&spec);
        let kv_cache_handle = KvCacheHandle::new(format!("llama_cpp:{id}"));
        let kv_cache = context.kv_cache_ops(
            kv_cache_handle,
            initial_quantization,
            declared_capabilities.supports_kv_quantization,
            prefix_cache_ttl_seconds,
            max_bytes,
            LlamaCppKvCache::scope_for_model(id, &sha256_scope),
        );
        let lora_stack = context.lora_stack_ops(id, base_model_tag);
        let load_duration_ms = started.elapsed().as_millis().max(1);
        self.models.insert(
            id,
            LlamaModelHandle {
                context,
                declared_capabilities,
                cancel: CancellationToken::new(),
                kv_cache,
                lora_stack,
                load_duration_ms,
                speculative_stats: Arc::new(Mutex::new(None)),
                generation_epoch: Arc::new(AtomicU64::new(0)),
                perf_stats: Arc::new(Mutex::new(LlamaCppPerfStats::default())),
            },
        );
        Ok(id)
    }

    async fn unload(&mut self, id: ModelId) -> Result<(), ModelRuntimeError> {
        self.models.remove(&id).map(|_| ()).ok_or_else(|| {
            ModelRuntimeError::UnloadError(format!("llama.cpp model is not loaded: {id}"))
        })
    }

    fn generate(&self, req: GenerateRequest) -> TokenStream {
        let Some(handle) = self.models.get(&req.id) else {
            return single_error_stream(ModelRuntimeError::GenerateError(format!(
                "llama.cpp model is not loaded: {}",
                req.id
            )));
        };

        match generation_preflight(&req) {
            Ok(GeneratePreflight::Ready) => {
                let generation_epoch = handle
                    .generation_epoch
                    .fetch_add(1, Ordering::SeqCst)
                    .saturating_add(1);
                if let Ok(mut guard) = handle.speculative_stats.lock() {
                    *guard = None;
                }
                let draft_native = match draft_native_for_request(&self.models, &req) {
                    Ok(native) => native,
                    Err(error) => {
                        if let Ok(mut guard) = handle.speculative_stats.lock() {
                            *guard = Some(SpeculativeStats::default());
                        }
                        return single_error_stream(error);
                    }
                };
                handle.context.generate(
                    req,
                    handle.cancel.clone(),
                    handle.kv_cache.clone(),
                    handle.lora_stack.clone(),
                    draft_native,
                    handle.speculative_stats.clone(),
                    handle.generation_epoch.clone(),
                    generation_epoch,
                    handle.perf_stats.clone(),
                    self.flight_recorder.clone(),
                )
            }
            Ok(GeneratePreflight::AlreadyCancelled) => {
                single_token_stream(terminal_token(FinishReason::Cancelled))
            }
            Ok(GeneratePreflight::LengthCapped) => {
                single_token_stream(terminal_token(FinishReason::Length))
            }
            Err(error) => single_error_stream(error),
        }
    }

    async fn score(&self, id: ModelId, sequence: Vec<u32>) -> Result<Score, ModelRuntimeError> {
        let handle = self.models.get(&id).ok_or_else(|| {
            ModelRuntimeError::ScoreError(format!("llama.cpp model is not loaded: {id}"))
        })?;
        super::score_embed::score(&handle.context, handle.kv_cache.quantization(), sequence).await
    }

    async fn embed(&self, id: ModelId, text: &str) -> Result<Embedding, ModelRuntimeError> {
        let handle = self.models.get(&id).ok_or_else(|| {
            ModelRuntimeError::EmbedError(format!("llama.cpp model is not loaded: {id}"))
        })?;
        super::score_embed::embed(&handle.context, handle.kv_cache.quantization(), text).await
    }

    fn capabilities(&self, id: ModelId) -> Result<&ModelCapabilities, ModelRuntimeError> {
        self.models
            .get(&id)
            .map(|handle| &handle.declared_capabilities)
            .ok_or_else(|| {
                ModelRuntimeError::LoadError(format!("llama.cpp model is not loaded: {id}"))
            })
    }

    fn kv_cache(&self, id: ModelId) -> Result<KvCacheHandle, ModelRuntimeError> {
        // Wire the existing Arc<LlamaCppKvCache> (which already impls
        // KvCacheOps) into the KvCacheHandle so the public
        // kv_cache_technique::* surface can dispatch through the handle
        // (mirrors LlamaCppLoraStack::handle()).
        self.models
            .get(&id)
            .map(|handle| {
                KvCacheHandle::with_ops(format!("llama_cpp:{id}:kv_cache"), handle.kv_cache.clone())
            })
            .ok_or_else(|| {
                ModelRuntimeError::LoadError(format!("llama.cpp model is not loaded: {id}"))
            })
    }

    fn lora_stack(&self, id: ModelId) -> Result<LoraStackHandle, ModelRuntimeError> {
        self.models
            .get(&id)
            .map(|handle| handle.lora_stack.handle())
            .ok_or_else(|| {
                ModelRuntimeError::LoraStackError(format!("llama.cpp model is not loaded: {id}"))
            })
    }

    fn steering_hooks(&self, _id: ModelId) -> Result<SteeringHookHandle, ModelRuntimeError> {
        Err(Self::not_implemented(
            "llama_cpp_steering_hooks_not_supported",
        ))
    }

    fn cancel(&self, token: CancellationToken) {
        token.cancel();
        for handle in self.models.values() {
            handle.cancel.cancel();
        }
    }
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn draft_native_for_request(
    models: &HashMap<ModelId, LlamaModelHandle>,
    req: &GenerateRequest,
) -> Result<Option<Arc<super::context::NativeLlamaCppBackend>>, ModelRuntimeError> {
    let Some(crate::model_runtime::SpeculativeMode::DraftModel { draft_id, .. }) =
        req.speculative_mode.as_ref()
    else {
        return Ok(None);
    };

    models
        .get(draft_id)
        .map(|handle| Some(handle.context.native_backend()))
        .ok_or_else(|| {
            ModelRuntimeError::GenerateError(format!(
                "llama.cpp draft model is not loaded: {draft_id}"
            ))
        })
}

#[cfg(not(feature = "llama-cpp-runtime-engine"))]
fn draft_native_for_request(
    _models: &HashMap<ModelId, LlamaModelHandle>,
    _req: &GenerateRequest,
) -> Result<Option<()>, ModelRuntimeError> {
    Ok(None)
}

fn kv_policy_defaults(
    policy: &KvCachePolicy,
    supported_quantization: KvQuantSupport,
) -> (KvQuantSupport, u64, Option<u64>) {
    match policy {
        KvCachePolicy::Default {
            quant,
            prefix_cache_ttl_seconds,
            max_bytes,
        } => (
            if quantization_supported(*quant, supported_quantization) {
                *quant
            } else {
                KvQuantSupport::None
            },
            *prefix_cache_ttl_seconds,
            *max_bytes,
        ),
        KvCachePolicy::Disabled | KvCachePolicy::Custom(_) => (KvQuantSupport::None, 0, None),
    }
}

fn quantization_supported(requested: KvQuantSupport, supported: KvQuantSupport) -> bool {
    match (requested, supported) {
        (KvQuantSupport::None, _) => true,
        (KvQuantSupport::Q4, KvQuantSupport::Q4 | KvQuantSupport::Q4Q8Mix) => true,
        (KvQuantSupport::Q8, KvQuantSupport::Q8 | KvQuantSupport::Q4Q8Mix) => true,
        (KvQuantSupport::Q4Q8Mix, KvQuantSupport::Q4Q8Mix) => true,
        _ => false,
    }
}

fn llama_cpp_base_model_tag(spec: &LoadSpec) -> String {
    spec.engine_origin
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .or_else(|| {
            spec.artifact_path
                .file_stem()
                .and_then(|value| value.to_str())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
        })
        .unwrap_or_else(|| "unknown".to_string())
}

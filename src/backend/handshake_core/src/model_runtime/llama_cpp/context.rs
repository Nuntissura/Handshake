use std::{path::Path, sync::Arc};

#[cfg(feature = "llama-cpp-runtime-engine")]
use std::sync::OnceLock;

#[cfg(feature = "llama-cpp-runtime-engine")]
use std::num::NonZeroU32;

#[cfg(feature = "llama-cpp-runtime-engine")]
use llama_cpp_2::context::params::{
    KvCacheType, LlamaAttentionType, LlamaContextParams, LlamaPoolingType,
};
#[cfg(feature = "llama-cpp-runtime-engine")]
use llama_cpp_2::model::params::LlamaModelParams;

use super::super::{
    CancellationToken, GenerateRequest, KvCacheHandle, KvQuantSupport, ModelRuntimeError,
    TokenStream,
};
use super::generate;
use super::gguf_loader::LlamaCppLoadConfig;
#[cfg(feature = "llama-cpp-runtime-engine")]
use super::gguf_loader::{
    GpuLayerOffload, LlamaCppContextLoadConfig, LlamaCppEmbeddingPooling, LlamaCppModelLoadConfig,
};
use super::kv_cache_impl::LlamaCppKvCache;
use super::lora_impl::LlamaCppLoraStack;
use super::perf_stats::LlamaCppPerfStats;
use crate::flight_recorder::FlightRecorder;

pub const LLAMA_CPP_NATIVE_FEATURE_DISABLED: &str = "llama.cpp native engine feature disabled";

#[derive(Clone, Debug)]
pub struct LlamaCppContext {
    backend: LlamaCppBackend,
}

impl LlamaCppContext {
    pub fn load_from_file(
        path: &Path,
        config: &LlamaCppLoadConfig,
    ) -> Result<Self, ModelRuntimeError> {
        load_backend(path, config).map(|backend| Self { backend })
    }

    pub(super) fn generate(
        &self,
        req: GenerateRequest,
        runtime_cancel: CancellationToken,
        kv_cache: Arc<LlamaCppKvCache>,
        lora_stack: Arc<LlamaCppLoraStack>,
        #[cfg(feature = "llama-cpp-runtime-engine")] draft_native: Option<
            Arc<NativeLlamaCppBackend>,
        >,
        #[cfg(not(feature = "llama-cpp-runtime-engine"))] draft_native: Option<()>,
        stats_sink: Arc<std::sync::Mutex<Option<super::speculative::SpeculativeStats>>>,
        current_generation_epoch: Arc<std::sync::atomic::AtomicU64>,
        generation_epoch: u64,
        perf_stats: Arc<std::sync::Mutex<LlamaCppPerfStats>>,
        flight_recorder: Option<Arc<dyn FlightRecorder>>,
    ) -> TokenStream {
        generate_backend(
            &self.backend,
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
        )
    }

    pub fn kv_cache_ops(
        &self,
        handle: KvCacheHandle,
        initial_quantization: KvQuantSupport,
        supported_quantization: KvQuantSupport,
        prefix_cache_ttl_seconds: u64,
        max_bytes: Option<u64>,
        scope: Vec<u8>,
    ) -> Arc<LlamaCppKvCache> {
        kv_cache_ops_backend(
            &self.backend,
            handle,
            initial_quantization,
            supported_quantization,
            prefix_cache_ttl_seconds,
            max_bytes,
            scope,
        )
    }

    pub fn lora_stack_ops(
        &self,
        model_id: crate::model_runtime::ModelId,
        base_model_tag: impl Into<String>,
    ) -> Arc<LlamaCppLoraStack> {
        lora_stack_ops_backend(&self.backend, model_id, base_model_tag.into())
    }

    pub fn tokenize_prompt(&self, prompt: &str) -> Result<Vec<u32>, ModelRuntimeError> {
        tokenize_prompt_backend(&self.backend, prompt)
    }

    #[cfg(feature = "llama-cpp-runtime-engine")]
    pub(super) fn native_backend(&self) -> Arc<NativeLlamaCppBackend> {
        match &self.backend {
            LlamaCppBackend::Native(native) => native.clone(),
        }
    }
}

#[derive(Clone, Debug)]
enum LlamaCppBackend {
    #[cfg(feature = "llama-cpp-runtime-engine")]
    Native(Arc<NativeLlamaCppBackend>),
}

#[cfg(feature = "llama-cpp-runtime-engine")]
#[derive(Debug)]
pub(super) struct NativeLlamaCppBackend {
    pub(super) model: llama_cpp_2::model::LlamaModel,
    pub(super) backend: Arc<llama_cpp_2::llama_backend::LlamaBackend>,
    context_config: LlamaCppContextLoadConfig,
}

#[cfg(feature = "llama-cpp-runtime-engine")]
impl NativeLlamaCppBackend {
    pub(super) fn new_context(
        &self,
        quantization: KvQuantSupport,
    ) -> Result<llama_cpp_2::context::LlamaContext<'_>, ModelRuntimeError> {
        self.model
            .new_context(
                self.backend.as_ref(),
                context_params_from_config(&self.context_config, quantization),
            )
            .map_err(|error| {
                ModelRuntimeError::GenerateError(format!(
                    "llama.cpp context creation failed: {error}"
                ))
            })
    }

    pub(super) fn estimated_kv_capacity_bytes(&self, quantization: KvQuantSupport) -> u64 {
        let n_ctx = u64::from(self.context_config.n_ctx.max(1));
        let n_layer = u64::from(self.model.n_layer().max(1));
        let n_head = u64::from(self.model.n_head().max(1));
        let n_head_kv = u64::from(self.model.n_head_kv().max(1));
        let n_embd = u64::try_from(self.model.n_embd()).unwrap_or(0);
        let head_dim = (n_embd / n_head).max(1);
        let values_per_cache = n_ctx
            .saturating_mul(n_layer)
            .saturating_mul(n_head_kv)
            .saturating_mul(head_dim);
        let (type_k, type_v) = kv_cache_types_for_quantization(quantization);
        scaled_kv_type_bytes(values_per_cache, type_k)
            .saturating_add(scaled_kv_type_bytes(values_per_cache, type_v))
    }
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn load_backend(
    path: &Path,
    config: &LlamaCppLoadConfig,
) -> Result<LlamaCppBackend, ModelRuntimeError> {
    let backend = shared_native_backend()?;
    let model_params = model_params_from_config(&config.model);
    let model =
        llama_cpp_2::model::LlamaModel::load_from_file(backend.as_ref(), path, &model_params)
            .map_err(|error| {
                ModelRuntimeError::LoadError(format!("llama.cpp model load failed: {error}"))
            })?;

    Ok(LlamaCppBackend::Native(Arc::new(NativeLlamaCppBackend {
        model,
        backend,
        context_config: config.context.clone(),
    })))
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn shared_native_backend(
) -> Result<Arc<llama_cpp_2::llama_backend::LlamaBackend>, ModelRuntimeError> {
    static BACKEND: OnceLock<Result<Arc<llama_cpp_2::llama_backend::LlamaBackend>, String>> =
        OnceLock::new();

    match BACKEND.get_or_init(|| {
        llama_cpp_2::llama_backend::LlamaBackend::init()
            .map(Arc::new)
            .map_err(|error| error.to_string())
    }) {
        Ok(backend) => Ok(backend.clone()),
        Err(error) => Err(ModelRuntimeError::LoadError(format!(
            "llama.cpp backend init failed: {error}"
        ))),
    }
}

#[cfg(not(feature = "llama-cpp-runtime-engine"))]
fn load_backend(
    _path: &Path,
    _config: &LlamaCppLoadConfig,
) -> Result<LlamaCppBackend, ModelRuntimeError> {
    Err(ModelRuntimeError::LoadError(
        LLAMA_CPP_NATIVE_FEATURE_DISABLED.to_string(),
    ))
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn tokenize_prompt_backend(
    backend: &LlamaCppBackend,
    prompt: &str,
) -> Result<Vec<u32>, ModelRuntimeError> {
    use llama_cpp_2::model::AddBos;

    let tokens = match backend {
        LlamaCppBackend::Native(native) => native.model.str_to_token(prompt, AddBos::Always),
    }
    .map_err(|error| {
        ModelRuntimeError::GenerateError(format!("llama.cpp prompt tokenization failed: {error}"))
    })?;

    tokens
        .into_iter()
        .map(|token| {
            u32::try_from(token.0).map_err(|error| {
                ModelRuntimeError::GenerateError(format!(
                    "llama.cpp prompt token id does not fit u32: {error}"
                ))
            })
        })
        .collect()
}

#[cfg(not(feature = "llama-cpp-runtime-engine"))]
fn tokenize_prompt_backend(
    _backend: &LlamaCppBackend,
    _prompt: &str,
) -> Result<Vec<u32>, ModelRuntimeError> {
    Err(ModelRuntimeError::LoadError(
        LLAMA_CPP_NATIVE_FEATURE_DISABLED.to_string(),
    ))
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn generate_backend(
    backend: &LlamaCppBackend,
    req: GenerateRequest,
    runtime_cancel: CancellationToken,
    kv_cache: Arc<LlamaCppKvCache>,
    lora_stack: Arc<LlamaCppLoraStack>,
    draft_native: Option<Arc<NativeLlamaCppBackend>>,
    stats_sink: Arc<std::sync::Mutex<Option<super::speculative::SpeculativeStats>>>,
    current_generation_epoch: Arc<std::sync::atomic::AtomicU64>,
    generation_epoch: u64,
    perf_stats: Arc<std::sync::Mutex<LlamaCppPerfStats>>,
    flight_recorder: Option<Arc<dyn FlightRecorder>>,
) -> TokenStream {
    match backend {
        LlamaCppBackend::Native(native) => generate::native_generate_stream(
            native.clone(),
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
        ),
    }
}

#[cfg(not(feature = "llama-cpp-runtime-engine"))]
fn generate_backend(
    _backend: &LlamaCppBackend,
    _req: GenerateRequest,
    _runtime_cancel: CancellationToken,
    _kv_cache: Arc<LlamaCppKvCache>,
    _lora_stack: Arc<LlamaCppLoraStack>,
    _draft_native: Option<()>,
    _stats_sink: Arc<std::sync::Mutex<Option<super::speculative::SpeculativeStats>>>,
    _current_generation_epoch: Arc<std::sync::atomic::AtomicU64>,
    _generation_epoch: u64,
    _perf_stats: Arc<std::sync::Mutex<LlamaCppPerfStats>>,
    _flight_recorder: Option<Arc<dyn FlightRecorder>>,
) -> TokenStream {
    generate::single_error_stream(ModelRuntimeError::LoadError(
        LLAMA_CPP_NATIVE_FEATURE_DISABLED.to_string(),
    ))
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn kv_cache_ops_backend(
    backend: &LlamaCppBackend,
    handle: KvCacheHandle,
    initial_quantization: KvQuantSupport,
    supported_quantization: KvQuantSupport,
    prefix_cache_ttl_seconds: u64,
    max_bytes: Option<u64>,
    scope: Vec<u8>,
) -> Arc<LlamaCppKvCache> {
    match backend {
        LlamaCppBackend::Native(native) => Arc::new(LlamaCppKvCache::new(
            handle,
            native.clone(),
            initial_quantization,
            supported_quantization,
            prefix_cache_ttl_seconds,
            max_bytes,
            scope,
        )),
    }
}

#[cfg(not(feature = "llama-cpp-runtime-engine"))]
fn kv_cache_ops_backend(
    _backend: &LlamaCppBackend,
    handle: KvCacheHandle,
    _initial_quantization: KvQuantSupport,
    _supported_quantization: KvQuantSupport,
    _prefix_cache_ttl_seconds: u64,
    _max_bytes: Option<u64>,
    _scope: Vec<u8>,
) -> Arc<LlamaCppKvCache> {
    Arc::new(LlamaCppKvCache::new(handle))
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn lora_stack_ops_backend(
    backend: &LlamaCppBackend,
    model_id: crate::model_runtime::ModelId,
    base_model_tag: String,
) -> Arc<LlamaCppLoraStack> {
    match backend {
        LlamaCppBackend::Native(native) => Arc::new(LlamaCppLoraStack::new(
            model_id,
            base_model_tag,
            native.clone(),
        )),
    }
}

#[cfg(not(feature = "llama-cpp-runtime-engine"))]
fn lora_stack_ops_backend(
    _backend: &LlamaCppBackend,
    model_id: crate::model_runtime::ModelId,
    base_model_tag: String,
) -> Arc<LlamaCppLoraStack> {
    Arc::new(LlamaCppLoraStack::new(model_id, base_model_tag))
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn model_params_from_config(config: &LlamaCppModelLoadConfig) -> LlamaModelParams {
    let params = LlamaModelParams::default();
    let params = match config.gpu_layers {
        GpuLayerOffload::CpuOnly => params.with_n_gpu_layers(0),
        GpuLayerOffload::LayerCount(count) => params.with_n_gpu_layers(count),
        GpuLayerOffload::All => params,
    };

    params
        .with_main_gpu(config.main_gpu)
        .with_vocab_only(config.vocab_only)
        .with_use_mmap(config.use_mmap)
        .with_use_mlock(config.use_mlock)
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn context_params_from_config(
    config: &LlamaCppContextLoadConfig,
    quantization: KvQuantSupport,
) -> LlamaContextParams {
    let n_ctx = NonZeroU32::new(config.n_ctx).or_else(|| NonZeroU32::new(1));
    let attention_type = if config.causal_attn {
        LlamaAttentionType::Causal
    } else {
        LlamaAttentionType::NonCausal
    };
    let (type_k, type_v) = kv_cache_types_for_quantization(quantization);

    LlamaContextParams::default()
        .with_n_ctx(n_ctx)
        .with_n_batch(config.n_batch)
        .with_n_threads(i32::try_from(config.n_threads).unwrap_or(i32::MAX))
        .with_embeddings(config.embeddings)
        .with_attention_type(attention_type)
        .with_pooling_type(pooling_type_for_config(&config.embedding_pooling))
        .with_n_seq_max(config.n_seq_max)
        .with_type_k(type_k)
        .with_type_v(type_v)
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn pooling_type_for_config(pooling: &LlamaCppEmbeddingPooling) -> LlamaPoolingType {
    match pooling {
        LlamaCppEmbeddingPooling::Unspecified => LlamaPoolingType::Unspecified,
        LlamaCppEmbeddingPooling::None => LlamaPoolingType::None,
        LlamaCppEmbeddingPooling::Mean => LlamaPoolingType::Mean,
        LlamaCppEmbeddingPooling::Cls => LlamaPoolingType::Cls,
        LlamaCppEmbeddingPooling::Last => LlamaPoolingType::Last,
        LlamaCppEmbeddingPooling::Rank => LlamaPoolingType::Rank,
    }
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn kv_cache_types_for_quantization(quantization: KvQuantSupport) -> (KvCacheType, KvCacheType) {
    match quantization {
        KvQuantSupport::None => (KvCacheType::F16, KvCacheType::F16),
        KvQuantSupport::Q4 => (KvCacheType::Q4_0, KvCacheType::Q4_0),
        KvQuantSupport::Q8 => (KvCacheType::Q8_0, KvCacheType::Q8_0),
        KvQuantSupport::Q4Q8Mix => (KvCacheType::Q4_0, KvCacheType::Q8_0),
    }
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn scaled_kv_type_bytes(values: u64, cache_type: KvCacheType) -> u64 {
    let (numerator, denominator) = match cache_type {
        KvCacheType::Q4_0 => (9_u64, 16_u64),
        KvCacheType::Q8_0 => (17_u64, 16_u64),
        KvCacheType::F16 => (2_u64, 1_u64),
        KvCacheType::F32 => (4_u64, 1_u64),
        _ => (2_u64, 1_u64),
    };
    let scaled = values.saturating_mul(numerator);
    scaled
        .saturating_add(denominator.saturating_sub(1))
        .saturating_div(denominator)
}

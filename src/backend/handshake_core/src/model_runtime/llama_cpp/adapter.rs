use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

#[cfg(feature = "llama-cpp-runtime-engine")]
use std::time::Instant;

use async_trait::async_trait;

#[cfg(feature = "llama-cpp-runtime-engine")]
use super::artifact_snapshot::{capture_gguf_artifact, CapturedGgufArtifact};
#[cfg(feature = "llama-cpp-runtime-engine")]
use super::tokenizer_cache::LlamaTokenizer;
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
#[cfg(feature = "llama-cpp-runtime-engine")]
use crate::model_runtime::RuntimeKind;
use crate::{
    flight_recorder::FlightRecorder,
    model_runtime::{
        CancellationToken, Embedding, FinishReason, GenerateRequest, KvCacheHandle, KvCacheOps,
        KvCachePolicy, KvQuantSupport, LoadSpec, LoraStackHandle, ModelCapabilities, ModelId,
        ModelRuntime, ModelRuntimeError, RuntimeActivityKind, RuntimeActivityTracker,
        RuntimeArtifactIntegrityReceipt, RuntimeQuiesceError, Score, SteeringHookHandle,
        TokenStream,
    },
};

pub struct LlamaCppRuntime {
    models: HashMap<ModelId, LlamaModelHandle>,
    activity: RuntimeActivityTracker,
    _default_kv_policy: KvCachePolicy,
    tokenizer_cache: TokenizerCache,
    load_config: LlamaCppLoadConfig,
    flight_recorder: Option<Arc<dyn FlightRecorder>>,
}

/// Fully validated result of the official llama.cpp boot-load path. The model
/// and tokenizer enter their runtime caches only after the exact staged GGUF
/// receipt, required capabilities, and UUIDv7 identity validate together.
#[derive(Clone, Debug)]
pub struct AttestedLlamaCppLoad {
    pub model_id: ModelId,
    pub artifact_integrity: RuntimeArtifactIntegrityReceipt,
    pub capabilities: ModelCapabilities,
}

impl LlamaCppRuntime {
    pub fn new(default_kv_policy: KvCachePolicy) -> Self {
        Self::with_load_config(default_kv_policy, LlamaCppLoadConfig::default())
    }

    pub fn with_load_config(
        default_kv_policy: KvCachePolicy,
        mut load_config: LlamaCppLoadConfig,
    ) -> Self {
        // Exact-byte attestation requires eager reads from the retained stage.
        // Normalize the public effective config as well as enforcing the native
        // parameter below, so diagnostics never report mmap as enabled.
        load_config.model.use_mmap = false;
        Self {
            models: HashMap::new(),
            activity: RuntimeActivityTracker::new(),
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

    /// Official production load boundary used by default boot. The configured
    /// source is never passed to llama.cpp; the native engine receives only the
    /// private exact-byte stage retained by the published model handle.
    pub async fn load_attested(
        &mut self,
        spec: LoadSpec,
        required_capabilities: &ModelCapabilities,
    ) -> Result<AttestedLlamaCppLoad, ModelRuntimeError> {
        gguf_loader::validate_llama_cpp_load_spec_fields(&spec)?;
        let expected_sha256 = decode_expected_sha256(&spec.sha256_expected)?;

        #[cfg(not(feature = "llama-cpp-runtime-engine"))]
        {
            let _ = required_capabilities;
            gguf_loader::validate_disabled_backend_artifact(&spec.artifact_path, expected_sha256)?;
            Err(ModelRuntimeError::LoadError(
                super::context::LLAMA_CPP_NATIVE_FEATURE_DISABLED.to_string(),
            ))
        }

        #[cfg(feature = "llama-cpp-runtime-engine")]
        {
            let started = Instant::now();
            let captured = capture_gguf_artifact(&spec.artifact_path, expected_sha256)?;
            let prepared = self.prepare_captured_artifact(&spec, captured, started)?;
            self.attest_and_publish(prepared, required_capabilities, expected_sha256)
        }
    }

    #[cfg(feature = "llama-cpp-runtime-engine")]
    fn prepare_captured_artifact(
        &self,
        spec: &LoadSpec,
        captured: CapturedGgufArtifact,
        started: Instant,
    ) -> Result<PreparedLlamaCppLoad, ModelRuntimeError> {
        let (initial_quantization, prefix_cache_ttl_seconds, max_bytes) = kv_policy_defaults(
            &spec.kv_cache_policy,
            spec.declared_capabilities.supports_kv_quantization,
        );
        let id = ModelId::new_v7();
        let tokenizer = Arc::new(captured.tokenizer().clone());
        let context =
            gguf_loader::load_staged_gguf_context(captured.staged_path(), &self.load_config)?;
        captured.post_verify()?;
        let capabilities =
            llama_cpp_capabilities(&spec.declared_capabilities, &self.load_config, &context)?;
        let sha256_scope = captured.receipt().primary_artifact_sha256().to_string();
        let base_model_tag = llama_cpp_base_model_tag(spec);
        let kv_cache_handle = KvCacheHandle::new(format!("llama_cpp:{id}"));
        let kv_cache = context.kv_cache_ops(
            kv_cache_handle,
            initial_quantization,
            capabilities.supports_kv_quantization,
            prefix_cache_ttl_seconds,
            max_bytes,
            LlamaCppKvCache::scope_for_model(id, &sha256_scope),
        );
        let lora_stack = context.lora_stack_ops(id, base_model_tag);
        let artifact_integrity = captured.receipt().clone();
        let handle = LlamaModelHandle {
            context,
            capabilities,
            cancel: CancellationToken::new(),
            kv_cache,
            lora_stack,
            load_duration_ms: started.elapsed().as_millis().max(1),
            speculative_stats: Arc::new(Mutex::new(None)),
            generation_epoch: Arc::new(AtomicU64::new(0)),
            perf_stats: Arc::new(Mutex::new(LlamaCppPerfStats::default())),
            artifact_integrity,
            _artifact_snapshot: captured,
        };
        Ok(PreparedLlamaCppLoad {
            model_id: id,
            handle,
            tokenizer,
        })
    }

    #[cfg(feature = "llama-cpp-runtime-engine")]
    fn attest_and_publish(
        &mut self,
        prepared: PreparedLlamaCppLoad,
        required_capabilities: &ModelCapabilities,
        expected_sha256: [u8; 32],
    ) -> Result<AttestedLlamaCppLoad, ModelRuntimeError> {
        prepared
            .handle
            .artifact_integrity
            .validate_for_runtime_expected(RuntimeKind::LlamaCpp, expected_sha256)?;
        ensure_llama_cpp_capabilities_satisfy(
            required_capabilities,
            &prepared.handle.capabilities,
        )?;
        if prepared.model_id.as_uuid().get_version_num() != 7 {
            return Err(ModelRuntimeError::LoadError(format!(
                "llama.cpp attested load returned non-UUIDv7 model id {}",
                prepared.model_id
            )));
        }
        if self.models.contains_key(&prepared.model_id) {
            return Err(ModelRuntimeError::LoadError(format!(
                "llama.cpp attested load minted duplicate model id {}",
                prepared.model_id
            )));
        }

        let attested = AttestedLlamaCppLoad {
            model_id: prepared.model_id,
            artifact_integrity: prepared.handle.artifact_integrity.clone(),
            capabilities: prepared.handle.capabilities.clone(),
        };
        // This is the only fallible publication step and it occurs after every
        // attestation check. HashMap insertion below is infallible and no await
        // can expose a tokenizer-only intermediate state.
        self.tokenizer_cache
            .insert_attested(prepared.model_id, prepared.tokenizer)?;
        self.models.insert(prepared.model_id, prepared.handle);
        Ok(attested)
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
    capabilities: ModelCapabilities,
    cancel: CancellationToken,
    kv_cache: Arc<LlamaCppKvCache>,
    lora_stack: Arc<LlamaCppLoraStack>,
    load_duration_ms: u128,
    speculative_stats: Arc<Mutex<Option<super::speculative::SpeculativeStats>>>,
    generation_epoch: Arc<AtomicU64>,
    perf_stats: Arc<Mutex<LlamaCppPerfStats>>,
    artifact_integrity: RuntimeArtifactIntegrityReceipt,
    #[cfg(feature = "llama-cpp-runtime-engine")]
    _artifact_snapshot: CapturedGgufArtifact,
}

#[cfg(feature = "llama-cpp-runtime-engine")]
struct PreparedLlamaCppLoad {
    model_id: ModelId,
    handle: LlamaModelHandle,
    tokenizer: Arc<LlamaTokenizer>,
}

#[async_trait]
impl ModelRuntime for LlamaCppRuntime {
    async fn load(&mut self, spec: LoadSpec) -> Result<ModelId, ModelRuntimeError> {
        let required_capabilities = spec.declared_capabilities.clone();
        self.load_attested(spec, &required_capabilities)
            .await
            .map(|attested| attested.model_id)
    }

    async fn unload(&mut self, id: ModelId) -> Result<(), ModelRuntimeError> {
        if !self.models.contains_key(&id) {
            return Err(ModelRuntimeError::UnloadError(format!(
                "llama.cpp model is not loaded: {id}"
            )));
        }
        self.tokenizer_cache.remove_attested(id)?;
        self.models.remove(&id);
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
            return single_error_stream(ModelRuntimeError::GenerateError(format!(
                "llama.cpp model is not loaded: {}",
                req.id
            )));
        };

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
                    activity_guard,
                )
            }
            Ok(GeneratePreflight::AlreadyCancelled) => {
                drop(activity_guard);
                single_token_stream(terminal_token(FinishReason::Cancelled))
            }
            Ok(GeneratePreflight::LengthCapped) => {
                drop(activity_guard);
                single_token_stream(terminal_token(FinishReason::Length))
            }
            Err(error) => {
                drop(activity_guard);
                single_error_stream(error)
            }
        }
    }

    async fn score(&self, id: ModelId, sequence: Vec<u32>) -> Result<Score, ModelRuntimeError> {
        let handle = self.models.get(&id).ok_or_else(|| {
            ModelRuntimeError::ScoreError(format!("llama.cpp model is not loaded: {id}"))
        })?;
        let activity_guard = self
            .activity
            .try_register(id, RuntimeActivityKind::Score, None)
            .map_err(|error| ModelRuntimeError::ScoreError(error.to_string()))?;
        super::score_embed::score(
            &handle.context,
            handle.kv_cache.quantization(),
            sequence,
            activity_guard,
        )
        .await
    }

    async fn embed(&self, id: ModelId, text: &str) -> Result<Embedding, ModelRuntimeError> {
        let handle = self.models.get(&id).ok_or_else(|| {
            ModelRuntimeError::EmbedError(format!("llama.cpp model is not loaded: {id}"))
        })?;
        let activity_guard = self
            .activity
            .try_register(id, RuntimeActivityKind::Embed, None)
            .map_err(|error| ModelRuntimeError::EmbedError(error.to_string()))?;
        super::score_embed::embed(
            &handle.context,
            handle.kv_cache.quantization(),
            text,
            activity_guard,
        )
        .await
    }

    async fn quiesce(&self, timeout: Duration) -> Result<(), RuntimeQuiesceError> {
        self.activity.quiesce(timeout).await
    }

    fn capabilities(&self, id: ModelId) -> Result<&ModelCapabilities, ModelRuntimeError> {
        self.models
            .get(&id)
            .map(|handle| &handle.capabilities)
            .ok_or_else(|| {
                ModelRuntimeError::LoadError(format!("llama.cpp model is not loaded: {id}"))
            })
    }

    fn artifact_integrity(
        &self,
        id: ModelId,
    ) -> Result<RuntimeArtifactIntegrityReceipt, ModelRuntimeError> {
        self.models
            .get(&id)
            .map(|handle| handle.artifact_integrity.clone())
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

fn decode_expected_sha256(expected: &str) -> Result<[u8; 32], ModelRuntimeError> {
    let trimmed = expected.trim();
    if trimmed.len() != 64 || !trimmed.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ModelRuntimeError::LoadError(
            "llama.cpp expected artifact sha256 must be exactly 64 hexadecimal characters"
                .to_string(),
        ));
    }
    let decoded = hex::decode(trimmed).map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "llama.cpp expected artifact sha256 is not valid hex: {error}"
        ))
    })?;
    decoded.try_into().map_err(|bytes: Vec<u8>| {
        ModelRuntimeError::LoadError(format!(
            "llama.cpp expected artifact sha256 decoded to {} bytes, expected 32",
            bytes.len()
        ))
    })
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn llama_cpp_capabilities(
    declared: &ModelCapabilities,
    load_config: &LlamaCppLoadConfig,
    context: &LlamaCppContext,
) -> Result<ModelCapabilities, ModelRuntimeError> {
    let supports_embedding = declared.supports_embedding && load_config.context.embeddings;
    let embedding_dimension = if supports_embedding {
        Some(context.embedding_dimension()?)
    } else {
        None
    };
    Ok(ModelCapabilities {
        supports_lora: declared.supports_lora,
        supports_kv_prefix_cache: declared.supports_kv_prefix_cache,
        supports_kv_quantization: declared.supports_kv_quantization,
        supports_activation_steering: false,
        supports_subquadratic: false,
        supports_speculative_draft: declared.supports_speculative_draft,
        supports_eagle3: false,
        supports_embedding,
        embedding_dimension,
    })
}

fn ensure_llama_cpp_capabilities_satisfy(
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
            "llama.cpp attested load is missing required capability {name}"
        )));
    }
    if required.supports_kv_quantization != KvQuantSupport::None
        && actual.supports_kv_quantization != required.supports_kv_quantization
    {
        return Err(ModelRuntimeError::LoadError(format!(
            "llama.cpp attested load KV quantization {:?} differs from required {:?}",
            actual.supports_kv_quantization, required.supports_kv_quantization
        )));
    }
    if let Some(required_dimension) = required.embedding_dimension {
        if actual.embedding_dimension != Some(required_dimension) {
            return Err(ModelRuntimeError::LoadError(format!(
                "llama.cpp attested load embedding dimension {:?} differs from required {required_dimension}",
                actual.embedding_dimension
            )));
        }
    }
    Ok(())
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

#[cfg(test)]
mod attestation_contract_tests {
    use super::*;
    use crate::model_runtime::llama_cpp::gguf_loader::LlamaCppModelLoadConfig;

    #[test]
    fn mt013_effective_llama_config_and_native_boundary_force_no_mmap() {
        let requested = LlamaCppLoadConfig {
            model: LlamaCppModelLoadConfig {
                use_mmap: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let runtime = LlamaCppRuntime::with_load_config(KvCachePolicy::default(), requested);
        assert!(!runtime.load_config().model.use_mmap);

        let context_source = include_str!("context.rs");
        let params_start = context_source
            .find("fn model_params_from_config")
            .expect("native model parameter boundary");
        let params = &context_source[params_start..];
        assert!(params.contains(".with_use_mmap(false)"));
        assert!(!params.contains(".with_use_mmap(config.use_mmap)"));
    }

    #[test]
    fn mt013_official_llama_load_uses_only_capture_stage_and_postverify() {
        let source = include_str!("adapter.rs");
        let start = source
            .find("pub async fn load_attested")
            .expect("official attested load");
        let end = source[start..]
            .find("fn prepare_captured_artifact")
            .map(|offset| start + offset)
            .expect("captured preparation boundary");
        let load = &source[start..end];
        assert!(load.contains("capture_gguf_artifact(&spec.artifact_path"));

        let prepare_start = end;
        let prepare_end = source[prepare_start..]
            .find("fn attest_and_publish")
            .map(|offset| prepare_start + offset)
            .expect("attestation boundary");
        let prepare = &source[prepare_start..prepare_end];
        assert!(prepare.contains("captured.staged_path()"));
        assert!(prepare.contains("captured.post_verify()?"));
        assert!(!prepare.contains("spec.artifact_path"));
        assert!(!prepare.contains("get_or_parse"));
    }

    #[test]
    fn mt013_required_capabilities_and_digest_syntax_fail_closed_engine_free() {
        let required = ModelCapabilities {
            supports_eagle3: true,
            ..Default::default()
        };
        let actual = ModelCapabilities::default();
        assert!(ensure_llama_cpp_capabilities_satisfy(&required, &actual)
            .expect_err("missing required capability must fail")
            .to_string()
            .contains("supports_eagle3"));
        assert!(decode_expected_sha256("not-a-digest")
            .expect_err("malformed digest must fail before artifact access")
            .to_string()
            .contains("exactly 64"));
    }
}

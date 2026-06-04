use std::{fs, path::PathBuf, sync::Arc};

use futures::{stream, StreamExt};
use handshake_core::model_runtime::{
    CancellationToken, Embedding, FinishReason, GenPrompt, GenerateRequest, GeneratedToken,
    JsonSchema, KvCacheHandle, KvCachePolicy, KvPrefixHandle, KvQuantSupport, LoadSpec, LoraId,
    LoraStackHandle, ModelCapabilities, ModelId, ModelRuntime, ModelRuntimeError, ProviderKind,
    RuntimeKind, SamplingParams, Score, SteeringHookHandle, SteeringVectorId, TokenStream,
};

struct NoopRuntime {
    loaded: Vec<(ModelId, ModelCapabilities)>,
    kv_cache: KvCacheHandle,
    lora_stack: LoraStackHandle,
    steering_hooks: SteeringHookHandle,
}

impl NoopRuntime {
    fn new() -> Self {
        Self {
            loaded: Vec::new(),
            kv_cache: KvCacheHandle::new("kv-cache"),
            lora_stack: LoraStackHandle::new("lora-stack"),
            steering_hooks: SteeringHookHandle::new("steering-hooks"),
        }
    }
}

#[async_trait::async_trait]
impl ModelRuntime for NoopRuntime {
    async fn load(&mut self, spec: LoadSpec) -> Result<ModelId, ModelRuntimeError> {
        assert_eq!(spec.runtime_kind, RuntimeKind::LlamaCpp);
        assert_eq!(spec.sha256_expected, "abc123");
        assert_eq!(spec.sampling_defaults.temperature, Some(0.2));

        let id = ModelId::new_v7();
        self.loaded.push((id, spec.declared_capabilities));
        Ok(id)
    }

    async fn unload(&mut self, id: ModelId) -> Result<(), ModelRuntimeError> {
        self.loaded.retain(|(loaded_id, _)| *loaded_id != id);
        Ok(())
    }

    fn generate(&self, req: GenerateRequest) -> TokenStream {
        assert!(!req.cancel.is_cancelled());
        assert_eq!(req.prompt, GenPrompt::from("prompt"));
        assert_eq!(req.max_tokens, 8);
        assert_eq!(req.stop_sequences, vec!["</s>".to_string()]);
        assert_eq!(req.lora_overrides.len(), 1);
        assert_eq!(req.lora_overrides[0].as_uuid().get_version_num(), 7);
        assert_eq!(req.steering_overrides[0].as_uuid().get_version_num(), 7);
        let prefix_handle = req
            .kv_prefix_handle
            .as_ref()
            .expect("kv prefix handle is present");
        assert_eq!(prefix_handle.token_count(), 3);
        assert_eq!(
            *prefix_handle.content_hash(),
            KvPrefixHandle::content_hash_for_tokens(&[4, 5, 6])
        );
        assert!(req.structured_decoding.is_some());

        Box::pin(stream::iter([
            Ok(GeneratedToken {
                token_id: 1,
                text: "hello".to_string(),
                logprob: Some(-0.1),
                finish_reason: None,
            }),
            Ok(GeneratedToken {
                token_id: 2,
                text: " done".to_string(),
                logprob: Some(-0.2),
                finish_reason: Some(FinishReason::Stop),
            }),
        ]))
    }

    async fn score(&self, id: ModelId, sequence: Vec<u32>) -> Result<Score, ModelRuntimeError> {
        assert!(self.loaded.iter().any(|(loaded_id, _)| *loaded_id == id));
        assert_eq!(sequence, vec![1, 2]);
        Ok(Score {
            token_logprobs: vec![-0.1, -0.2],
            mean_logprob: -0.15,
        })
    }

    async fn embed(&self, id: ModelId, text: &str) -> Result<Embedding, ModelRuntimeError> {
        assert!(self.loaded.iter().any(|(loaded_id, _)| *loaded_id == id));
        assert_eq!(text, "embed me");
        Ok(Embedding {
            vector: vec![0.25, 0.5, 0.75],
        })
    }

    fn capabilities(&self, id: ModelId) -> Result<&ModelCapabilities, ModelRuntimeError> {
        self.loaded
            .iter()
            .find_map(|(loaded_id, capabilities)| (*loaded_id == id).then_some(capabilities))
            .ok_or_else(|| ModelRuntimeError::LoadError(format!("unknown model {id}")))
    }

    fn kv_cache(&self, id: ModelId) -> Result<KvCacheHandle, ModelRuntimeError> {
        assert!(self.loaded.iter().any(|(loaded_id, _)| *loaded_id == id));
        Ok(self.kv_cache.clone())
    }

    fn lora_stack(&self, id: ModelId) -> Result<LoraStackHandle, ModelRuntimeError> {
        assert!(self.loaded.iter().any(|(loaded_id, _)| *loaded_id == id));
        Ok(self.lora_stack.clone())
    }

    fn steering_hooks(&self, id: ModelId) -> Result<SteeringHookHandle, ModelRuntimeError> {
        assert!(self.loaded.iter().any(|(loaded_id, _)| *loaded_id == id));
        Ok(self.steering_hooks.clone())
    }

    fn cancel(&self, token: CancellationToken) {
        token.cancel();
    }
}

#[test]
fn model_runtime_trait_tests_object_safe_send_sync_and_calls_all_methods() {
    fn assert_object_safe(_: &dyn ModelRuntime) {}
    fn assert_send_sync<T: Send + Sync>() {}

    assert_send_sync::<Box<dyn ModelRuntime>>();
    assert_send_sync::<Arc<dyn ModelRuntime>>();

    let mut runtime = NoopRuntime::new();
    assert_object_safe(&runtime);

    let capabilities = ModelCapabilities {
        supports_lora: true,
        supports_kv_prefix_cache: true,
        supports_kv_quantization: KvQuantSupport::Q4Q8Mix,
        supports_activation_steering: true,
        supports_subquadratic: false,
        supports_speculative_draft: true,
        supports_eagle3: false,
    };

    let sampling = SamplingParams {
        temperature: Some(0.2),
        top_p: Some(0.9),
        top_k: Some(40),
        min_p: Some(0.05),
        repetition_penalty: Some(1.1),
        frequency_penalty: Some(0.1),
        presence_penalty: Some(0.1),
        seed: Some(7),
    };
    let _: Option<u32> = sampling.seed;

    let model_id = futures::executor::block_on(runtime.load(LoadSpec {
        artifact_path: PathBuf::from("models/tiny.gguf"),
        sha256_expected: "abc123".to_string(),
        runtime_kind: RuntimeKind::LlamaCpp,
        sampling_defaults: sampling.clone(),
        kv_cache_policy: KvCachePolicy::default(),
        declared_capabilities: capabilities.clone(),
        provider: ProviderKind::Local,
        engine_origin: None,
        external_engine_import: None,
    }))
    .expect("load succeeds");

    assert_eq!(runtime.capabilities(model_id).unwrap(), &capabilities);
    assert_eq!(
        runtime.kv_cache(model_id).unwrap(),
        KvCacheHandle::new("kv-cache")
    );
    assert_eq!(
        runtime.lora_stack(model_id).unwrap(),
        LoraStackHandle::new("lora-stack")
    );
    assert_eq!(
        runtime.steering_hooks(model_id).unwrap(),
        SteeringHookHandle::new("steering-hooks")
    );

    let cancel = CancellationToken::new();
    let request = GenerateRequest {
        id: model_id,
        prompt: GenPrompt::from("prompt"),
        sampling,
        lora_overrides: vec![LoraId::new_v7()],
        steering_overrides: vec![SteeringVectorId::new_v7()],
        kv_prefix_handle: Some(KvPrefixHandle::from_tokens(&[4, 5, 6]).unwrap()),
        cancel: cancel.clone(),
        max_tokens: 8,
        stop_sequences: vec!["</s>".to_string()],
        speculative_mode: None,
        structured_decoding: Some(JsonSchema::new(serde_json::json!({
            "type": "object"
        }))),
    };

    let generated = futures::executor::block_on(async {
        runtime
            .generate(request)
            .collect::<Vec<Result<GeneratedToken, ModelRuntimeError>>>()
            .await
    });
    assert_eq!(generated.len(), 2);
    assert_eq!(
        generated
            .last()
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .finish_reason,
        Some(FinishReason::Stop)
    );

    let score = futures::executor::block_on(runtime.score(model_id, vec![1, 2])).unwrap();
    assert_eq!(score.mean_logprob, -0.15);

    let embedding = futures::executor::block_on(runtime.embed(model_id, "embed me")).unwrap();
    assert_eq!(embedding.vector, vec![0.25, 0.5, 0.75]);

    runtime.cancel(cancel.clone());
    assert!(cancel.is_cancelled());

    futures::executor::block_on(runtime.unload(model_id)).unwrap();
    assert!(runtime.capabilities(model_id).is_err());
}

#[test]
fn model_runtime_trait_tests_model_id_is_uuid_v7_and_timestamp_prefix_monotonic() {
    let first = ModelId::new_v7();
    let second = ModelId::new_v7();

    assert_eq!(first.as_uuid().get_version_num(), 7);
    assert_eq!(second.as_uuid().get_version_num(), 7);
    assert!(
        uuid_v7_unix_millis(&second) >= uuid_v7_unix_millis(&first),
        "ModelId v7 timestamp prefix must not move backwards"
    );
    assert_ne!(first, second);
}

#[test]
fn model_runtime_trait_tests_surface_is_engine_agnostic() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for relative in [
        ["src", "model_runtime", "trait.rs"],
        ["src", "model_runtime", "types.rs"],
    ] {
        let path = relative
            .iter()
            .fold(manifest_dir.clone(), |acc, item| acc.join(item));
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
        let normalized = source.to_ascii_lowercase();

        for banned in ["llama_cpp_2::", "candle_core::", "candle_transformers::"] {
            assert!(
                !normalized.contains(banned),
                "model_runtime trait/types surface must not leak engine-specific type `{banned}` in {}",
                path.display()
            );
        }
    }
}

fn uuid_v7_unix_millis(id: &ModelId) -> u64 {
    let bytes = *id.as_uuid().as_bytes();
    ((bytes[0] as u64) << 40)
        | ((bytes[1] as u64) << 32)
        | ((bytes[2] as u64) << 24)
        | ((bytes[3] as u64) << 16)
        | ((bytes[4] as u64) << 8)
        | bytes[5] as u64
}

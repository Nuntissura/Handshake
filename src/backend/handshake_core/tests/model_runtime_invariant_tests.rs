use std::{fs, path::PathBuf};

use chrono::{Duration, Utc};
use futures::stream;
use handshake_core::{
    model_runtime::{
        CancellationToken, EngineOriginValidator, ExternalEngineImportRecord, GenerateRequest,
        KvCacheHandle, LoadSpec, LocalModelAdapter, LoraStackHandle, ModelCapabilities, ModelId,
        ModelRuntime, ModelRuntimeError, ProviderKind, RuntimeKind, SamplingParams, Score,
        SteeringHookHandle, TokenStream,
    },
    process_ledger::ProcessEngineKind,
};

struct NoopRuntime;

#[async_trait::async_trait]
impl ModelRuntime for NoopRuntime {
    async fn load(&mut self, _spec: LoadSpec) -> Result<ModelId, ModelRuntimeError> {
        Ok(ModelId::new_v7())
    }

    async fn unload(&mut self, _id: ModelId) -> Result<(), ModelRuntimeError> {
        Ok(())
    }

    fn generate(&self, _req: GenerateRequest) -> TokenStream {
        Box::pin(stream::empty())
    }

    async fn score(&self, _id: ModelId, _sequence: Vec<u32>) -> Result<Score, ModelRuntimeError> {
        Ok(Score {
            token_logprobs: Vec::new(),
            mean_logprob: 0.0,
        })
    }

    async fn embed(
        &self,
        _id: ModelId,
        _text: &str,
    ) -> Result<handshake_core::model_runtime::Embedding, ModelRuntimeError> {
        Ok(handshake_core::model_runtime::Embedding { vector: Vec::new() })
    }

    fn capabilities(&self, _id: ModelId) -> Result<&ModelCapabilities, ModelRuntimeError> {
        static CAPABILITIES: ModelCapabilities = ModelCapabilities {
            supports_lora: false,
            supports_kv_prefix_cache: false,
            supports_kv_quantization: handshake_core::model_runtime::KvQuantSupport::None,
            supports_activation_steering: false,
            supports_subquadratic: false,
            supports_speculative_draft: false,
            supports_eagle3: false,
        };
        Ok(&CAPABILITIES)
    }

    fn kv_cache(&self, _id: ModelId) -> Result<KvCacheHandle, ModelRuntimeError> {
        Ok(KvCacheHandle::new("kv"))
    }

    fn lora_stack(&self, _id: ModelId) -> Result<LoraStackHandle, ModelRuntimeError> {
        Ok(LoraStackHandle::new("lora"))
    }

    fn steering_hooks(&self, _id: ModelId) -> Result<SteeringHookHandle, ModelRuntimeError> {
        Ok(SteeringHookHandle::new("steering"))
    }

    fn cancel(&self, token: CancellationToken) {
        token.cancel();
    }
}

#[test]
fn model_runtime_invariant_tests_rejects_third_party_runtime_authority_without_external_import() {
    let artifact = fixture_model_file("local-authority.gguf");
    let validator = EngineOriginValidator::with_now(Utc::now());

    for origin in [
        "ollama",
        "OLLAMA serve",
        "lm_studio.exe",
        "lm-studio.exe",
        "LMStudio.exe",
        "http://localhost:11434/v1/chat/completions",
        "http://127.0.0.1:1234/v1/models",
        "http://[::1]:1234/v1/models",
        "http://0.0.0.0:11434/v1/models",
    ] {
        let spec = local_spec(&artifact).with_engine_origin(origin);
        let error = validator.validate_load_spec(&spec).unwrap_err();
        assert!(
            matches!(error, ModelRuntimeError::AdapterMismatch { .. }),
            "origin {origin} must fail closed"
        );
    }
}

#[test]
fn model_runtime_invariant_tests_local_provider_requires_explicit_handshake_origin() {
    let artifact = fixture_model_file("origin-required.gguf");
    let validator = EngineOriginValidator::with_now(Utc::now());

    let missing_origin = local_spec_without_origin(&artifact);
    assert!(
        validator.validate_load_spec(&missing_origin).is_err(),
        "local runtime authority must not be accepted without explicit Handshake origin proof"
    );

    let local = local_spec(&artifact);
    let decision = validator.validate_load_spec(&local).unwrap();
    assert_eq!(decision.provider, ProviderKind::Local);
    assert_eq!(
        decision.owned_process_engine_kind,
        Some(ProcessEngineKind::LlamaCpp)
    );

    let mismatched = LoadSpec {
        runtime_kind: RuntimeKind::Candle,
        ..local_spec(&artifact)
    };
    assert!(
        validator.validate_load_spec(&mismatched).is_err(),
        "runtime kind and Handshake origin proof must agree"
    );
}

#[test]
fn model_runtime_invariant_tests_allows_signed_external_compat_lane_but_not_owned_adapter() {
    let now = Utc::now();
    let validator = EngineOriginValidator::with_now(now);
    let spec = LoadSpec {
        provider: ProviderKind::ExternalCompat,
        engine_origin: Some("http://localhost:11434/v1".to_string()),
        external_engine_import: Some(
            ExternalEngineImportRecord::new(
                "http://localhost:11434/v1",
                true,
                now,
                "ilja180520260209",
            )
            .expect("external import record"),
        ),
        ..local_spec(&fixture_model_file("external-compat-placeholder.gguf"))
    };

    let decision = validator.validate_load_spec(&spec).unwrap();
    assert_eq!(decision.provider, ProviderKind::ExternalCompat);
    assert_eq!(decision.owned_process_engine_kind, None);

    let adapter = LocalModelAdapter::new(Box::new(NoopRuntime), spec);
    assert!(
        matches!(adapter, Err(ModelRuntimeError::AdapterMismatch { .. })),
        "ExternalEngineImport is a pass-through lane, not Handshake-owned LocalModelAdapter authority"
    );
}

#[test]
fn model_runtime_invariant_tests_rejects_stale_or_unsigned_external_imports() {
    let now = Utc::now();
    let validator = EngineOriginValidator::with_now(now);
    let valid = ExternalEngineImportRecord::new(
        "http://localhost:1234/v1",
        true,
        now - Duration::days(7),
        "operator-signed",
    )
    .unwrap();
    let stale = ExternalEngineImportRecord::new(
        "http://localhost:1234/v1",
        true,
        now - Duration::days(31),
        "operator-signed",
    )
    .unwrap();

    let mut spec = local_spec(&fixture_model_file("external-valid.gguf"));
    spec.provider = ProviderKind::ExternalCompat;
    spec.engine_origin = Some("http://localhost:1234/v1".to_string());
    spec.external_engine_import = Some(valid);
    assert!(validator.validate_load_spec(&spec).is_ok());

    spec.external_engine_import = Some(stale);
    assert!(validator.validate_load_spec(&spec).is_err());

    let unsigned = ExternalEngineImportRecord::new("http://localhost:1234/v1", true, now, " ");
    assert!(unsigned.is_err());
}

#[test]
fn model_runtime_invariant_tests_external_import_must_match_origin_and_local_http_endpoint() {
    let now = Utc::now();
    let validator = EngineOriginValidator::with_now(now);

    let mut spec = local_spec(&fixture_model_file("external-origin-binding.gguf"));
    spec.provider = ProviderKind::ExternalCompat;
    spec.engine_origin = Some("http://localhost:1234/v1".to_string());
    spec.external_engine_import = Some(
        ExternalEngineImportRecord::new("http://localhost:11434/v1", true, now, "operator-signed")
            .unwrap(),
    );
    assert!(
        validator.validate_load_spec(&spec).is_err(),
        "ExternalEngineImportRecord endpoint must be bound to LoadSpec.engine_origin"
    );

    spec.engine_origin = None;
    spec.external_engine_import = Some(
        ExternalEngineImportRecord::new("http://localhost:1234/v1", true, now, "operator-signed")
            .unwrap(),
    );
    assert!(
        validator.validate_load_spec(&spec).is_err(),
        "ExternalCompat must name the actual endpoint origin"
    );

    assert!(ExternalEngineImportRecord::new(
        "file:///tmp/model.sock",
        true,
        now,
        "operator-signed"
    )
    .is_err());
    assert!(ExternalEngineImportRecord::new(
        "https://api.example.com/v1",
        true,
        now,
        "operator-signed"
    )
    .is_err());
}

#[test]
fn model_runtime_invariant_tests_local_adapter_requires_regular_artifact_file() {
    let artifact = fixture_model_file("safe-local.gguf");
    let validator = EngineOriginValidator::with_now(Utc::now());
    let spec = local_spec(&artifact);

    assert_eq!(
        validator.validate_load_spec(&spec).unwrap().provider,
        ProviderKind::Local
    );
    let adapter = LocalModelAdapter::new(Box::new(NoopRuntime), spec).unwrap();
    assert_eq!(adapter.provider(), ProviderKind::Local);

    let missing = local_spec(&artifact.with_file_name("missing.gguf"));
    assert!(validator.validate_load_spec(&missing).is_err());

    let url_like = LoadSpec {
        artifact_path: PathBuf::from("http://localhost:11434/model.gguf"),
        ..local_spec(&artifact)
    };
    assert!(validator.validate_load_spec(&url_like).is_err());
}

fn local_spec(artifact_path: &PathBuf) -> LoadSpec {
    LoadSpec {
        engine_origin: Some("handshake://model-runtime/llama_cpp".to_string()),
        ..local_spec_without_origin(artifact_path)
    }
}

fn local_spec_without_origin(artifact_path: &PathBuf) -> LoadSpec {
    LoadSpec {
        artifact_path: artifact_path.clone(),
        sha256_expected: "abc123".to_string(),
        runtime_kind: RuntimeKind::LlamaCpp,
        sampling_defaults: SamplingParams::default(),
        kv_cache_policy: Default::default(),
        declared_capabilities: ModelCapabilities::default(),
        provider: ProviderKind::Local,
        engine_origin: None,
        external_engine_import: None,
    }
}

fn fixture_model_file(name: &str) -> PathBuf {
    let root = std::env::var("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir().join("handshake-model-runtime-invariant-tests"));
    fs::create_dir_all(&root).expect("create test artifact dir");
    let path = root.join(name);
    fs::write(&path, b"test model artifact").expect("write test model artifact");
    path
}

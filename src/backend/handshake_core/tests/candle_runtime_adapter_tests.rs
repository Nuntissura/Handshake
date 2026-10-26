use std::{
    fs,
    path::{Path, PathBuf},
};

use futures::StreamExt;
#[cfg(not(feature = "candle-runtime-engine"))]
use handshake_core::model_runtime::candle::CANDLE_NATIVE_FEATURE_DISABLED;
use handshake_core::model_runtime::{
    candle::{
        tokenizer_json_path_for_artifact, CandleDeviceKind, CandleDevicePreference, CandleRuntime,
    },
    CancellationToken, GenPrompt, GenerateRequest, KvCachePolicy, KvQuantSupport, LoadSpec,
    ModelCapabilities, ModelId, ModelRuntime, ProviderKind, RuntimeKind, SamplingParams,
    CANDLE_LOCAL_ENGINE_ORIGIN,
};
use sha2::{Digest, Sha256};

#[test]
fn candle_runtime_default_device_selection_is_cpu_safe() {
    let runtime = CandleRuntime::default();
    let selection = runtime.device_selection();

    assert_eq!(selection.preference(), CandleDevicePreference::Auto);
    assert_eq!(selection.selected(), CandleDeviceKind::Cpu);
}

#[cfg(not(feature = "candle-runtime-engine"))]
#[test]
fn candle_runtime_gpu_preferences_fallback_to_cpu_without_native_engines() {
    for preference in [
        CandleDevicePreference::Cuda { ordinal: 0 },
        CandleDevicePreference::Metal { ordinal: 0 },
    ] {
        let runtime = CandleRuntime::with_device_preference(preference);
        let selection = runtime.device_selection();

        assert_eq!(selection.preference(), preference);
        assert_eq!(selection.selected(), CandleDeviceKind::Cpu);
        assert!(selection.fallback_reason().is_some());
    }
}

#[tokio::test]
async fn candle_load_rejects_non_candle_runtime_before_backend() {
    let fixture = ModelFileFixture::new("model.safetensors", b"not a real candle artifact");
    let mut runtime = CandleRuntime::default();
    let err = runtime
        .load(load_spec(
            fixture.path(),
            fixture.sha256(),
            RuntimeKind::LlamaCpp,
        ))
        .await
        .expect_err("runtime kind mismatch must fail");

    assert!(err.to_string().contains("Candle"), "{err}");
}

#[tokio::test]
async fn candle_load_rejects_sha256_mismatch_before_backend() {
    let fixture = ModelFileFixture::new("model.safetensors", b"not a real candle artifact");
    let mut runtime = CandleRuntime::default();
    let err = runtime
        .load(load_spec(
            fixture.path(),
            "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            RuntimeKind::Candle,
        ))
        .await
        .expect_err("sha mismatch must fail");

    assert!(err.to_string().contains("sha256"), "{err}");
    assert!(err.to_string().contains("mismatch"), "{err}");
}

#[cfg(not(feature = "candle-runtime-engine"))]
#[tokio::test]
async fn candle_load_without_native_feature_returns_typed_disabled_error_after_validation() {
    let fixture = ModelFileFixture::new("model.safetensors", b"not a real candle artifact");
    let mut runtime = CandleRuntime::default();
    let err = runtime
        .load(load_spec(
            fixture.path(),
            fixture.sha256(),
            RuntimeKind::Candle,
        ))
        .await
        .expect_err("native feature disabled must fail after validation");

    assert!(err.to_string().contains(CANDLE_NATIVE_FEATURE_DISABLED));
}

#[tokio::test]
async fn candle_runtime_generate_requires_loaded_model_and_other_methods_are_typed_errors() {
    let runtime = CandleRuntime::default();
    let model_id = ModelId::new_v7();
    let mut stream = runtime.generate(GenerateRequest {
        id: model_id,
        prompt: GenPrompt::from("hello"),
        sampling: SamplingParams::default(),
        lora_overrides: Vec::new(),
        steering_overrides: Vec::new(),
        kv_prefix_handle: None,
        cancel: CancellationToken::new(),
        max_tokens: 8,
        stop_sequences: Vec::new(),
        speculative_mode: None,
        structured_decoding: None,
    });
    let generate_err = stream
        .next()
        .await
        .expect("generate emits placeholder result")
        .expect_err("unknown model must fail before backend work");

    assert!(generate_err.to_string().contains("not loaded"));
    assert!(runtime.score(model_id, vec![1, 2]).await.is_err());
    assert!(runtime.embed(model_id, "hello").await.is_err());
    assert!(runtime.kv_cache(model_id).is_err());
    assert!(runtime.lora_stack(model_id).is_err());
    assert!(runtime.steering_hooks(model_id).is_err());
}

#[test]
fn candle_tokenizer_path_resolves_tokenizer_json_next_to_artifact_or_inside_model_dir() {
    let fixture = ModelFileFixture::new("nested/model.safetensors", b"weights");
    assert_eq!(
        tokenizer_json_path_for_artifact(fixture.path()),
        fixture.path().parent().unwrap().join("tokenizer.json")
    );

    let model_dir = fixture.path().parent().unwrap();
    assert_eq!(
        tokenizer_json_path_for_artifact(model_dir),
        model_dir.join("tokenizer.json")
    );
}

#[test]
fn candle_native_imports_are_contained_inside_candle_module() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_root = manifest_dir.join("src");
    let allowed = source_root.join("model_runtime").join("candle");
    let mut offenders = Vec::new();

    collect_candle_import_offenders(&source_root, &allowed, &mut offenders);

    assert!(
        offenders.is_empty(),
        "candle_core/candle_transformers imports must stay inside model_runtime/candle/**: {offenders:?}"
    );
}

struct ModelFileFixture {
    _tempdir: tempfile::TempDir,
    path: PathBuf,
}

impl ModelFileFixture {
    fn new(relative_path: &str, bytes: &[u8]) -> Self {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join(relative_path);
        fs::create_dir_all(path.parent().expect("fixture parent")).expect("create model dir");
        fs::write(&path, bytes).expect("write model fixture");
        Self {
            _tempdir: tempdir,
            path,
        }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn sha256(&self) -> String {
        sha256_file(&self.path)
    }
}

fn load_spec(artifact_path: &Path, sha256_expected: String, runtime_kind: RuntimeKind) -> LoadSpec {
    LoadSpec {
        artifact_path: artifact_path.to_path_buf(),
        sha256_expected,
        runtime_kind,
        sampling_defaults: SamplingParams::default(),
        kv_cache_policy: KvCachePolicy::Default {
            quant: KvQuantSupport::Q4,
            prefix_cache_ttl_seconds: 0,
            max_bytes: None,
        },
        declared_capabilities: ModelCapabilities {
            supports_lora: true,
            supports_kv_prefix_cache: true,
            supports_kv_quantization: KvQuantSupport::Q4,
            supports_activation_steering: true,
            supports_subquadratic: true,
            supports_speculative_draft: false,
            supports_eagle3: false,
            supports_embedding: false,
            embedding_dimension: None,
        },
        provider: ProviderKind::Local,
        engine_origin: Some(CANDLE_LOCAL_ENGINE_ORIGIN.to_string()),
        external_engine_import: None,
    }
}

fn sha256_file(path: &Path) -> String {
    let bytes = fs::read(path).expect("read fixture");
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

fn collect_candle_import_offenders(root: &Path, allowed: &Path, offenders: &mut Vec<String>) {
    let entries = fs::read_dir(root).unwrap_or_else(|error| {
        panic!("read source dir {}: {error}", root.display());
    });
    for entry in entries {
        let entry = entry.expect("read dir entry");
        let path = entry.path();
        if path.is_dir() {
            collect_candle_import_offenders(&path, allowed, offenders);
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) != Some("rs") {
            continue;
        }
        let source = fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!("read source file {}: {error}", path.display());
        });
        if (source.contains("candle_core::") || source.contains("candle_transformers::"))
            && !path.starts_with(allowed)
        {
            offenders.push(path.display().to_string());
        }
    }
}

#[cfg(feature = "candle-runtime-engine")]
mod qwen3_gguf {
    use super::*;
    use std::{collections::HashMap, io::Cursor};
    use candle_core::quantized::gguf_file::{self, Value};
    use handshake_core::model_runtime::{
        candle::{generate::{CandleGenerationCodec, TokenizerGenerationCodec},
            tokenizer::tokenizer_from_qwen3_gguf_metadata},
        GeneratedToken, ModelRuntimeError, RuntimeArtifactIntegrityReceipt,
    };

    // Metadata fixtures exercise the upstream decoder and host tokenizer, never weights.
    fn metadata() -> HashMap<String, Value> {
        let mut alphabet: Vec<char> = tokenizers::pre_tokenizers::byte_level::ByteLevel::alphabet()
            .into_iter().collect();
        alphabet.sort_unstable();
        let mut tokens: Vec<Value> = alphabet.into_iter().map(|c| Value::String(c.to_string())).collect();
        tokens.extend(["he", "hel", "hell", "hello", "<|im_end|>", "<|user_token|>"]
            .into_iter().map(|s| Value::String(s.into())));
        let eos = tokens.len() as u32 - 2;
        let mut types = vec![Value::I32(1); tokens.len()];
        types[eos as usize] = Value::I32(3);
        types[eos as usize + 1] = Value::I32(4);
        HashMap::from([
            ("general.architecture".into(), Value::String("qwen3".into())),
            ("tokenizer.ggml.model".into(), Value::String("gpt2".into())),
            ("tokenizer.ggml.pre".into(), Value::String("qwen2".into())),
            ("tokenizer.ggml.tokens".into(), Value::Array(tokens)),
            ("tokenizer.ggml.token_type".into(), Value::Array(types)),
            ("tokenizer.ggml.merges".into(), Value::Array(["h e", "he l", "hel l", "hell o"]
                .into_iter().map(|s| Value::String(s.into())).collect())),
            ("tokenizer.ggml.eos_token_id".into(), Value::U32(eos)),
            ("tokenizer.chat_template".into(), Value::String("provenance-only: {{ messages }}".into())),
        ])
    }

    fn gguf_fixture(metadata: &HashMap<String, Value>) -> ModelFileFixture {
        let mut bytes = Cursor::new(Vec::new());
        let mut entries: Vec<_> = metadata.iter().map(|(k, v)| (k.as_str(), v)).collect();
        entries.sort_by_key(|(key, _)| *key);
        gguf_file::write(&mut bytes, &entries, &[]).expect("upstream GGUF writer");
        ModelFileFixture::new("metadata.gguf", &bytes.into_inner())
    }

    #[test]
    fn embedded_qwen2_bpe_preserves_index_merges_specials_and_streamed_utf8() {
        let parsed = tokenizer_from_qwen3_gguf_metadata(&metadata()).expect("valid metadata");
        assert_eq!(parsed.vocab_size, 262);
        assert_eq!(parsed.chat_template, "provenance-only: {{ messages }}");
        assert_eq!(parsed.tokenizer.encode("hello", false).unwrap().get_ids(), &[259]);
        let text = "we're  123\nhéllo 世界🙂";
        let codec = TokenizerGenerationCodec::new(parsed.tokenizer.clone());
        let ids = codec.encode_prompt(text).expect("encode raw prompt");
        assert_eq!(parsed.tokenizer.decode(&ids, false).unwrap(), text);
        let pieces: Vec<_> = ids.into_iter().map(|id| codec.decode_token(id).unwrap()).collect();
        assert!(pieces.iter().all(|piece| !piece.contains('\u{fffd}')));
        assert_eq!(pieces.concat(), text);
        let next = codec.encode_prompt("hello").unwrap();
        assert_eq!(next.into_iter().map(|id| codec.decode_token(id).unwrap()).collect::<String>(), "hello");
        assert_eq!(parsed.tokenizer.encode("<|im_end|>", false).unwrap().get_ids(), parsed.eos_token_ids.as_slice());
        assert_eq!(parsed.tokenizer.decode(&parsed.eos_token_ids, true).unwrap(), "");
        let user = parsed.tokenizer.encode("<|user_token|>", false).unwrap();
        assert_eq!(parsed.tokenizer.decode(user.get_ids(), true).unwrap(), "<|user_token|>");
    }

    #[test]
    fn embedded_tokenizer_rejects_incomplete_or_incompatible_metadata() {
        for key in ["tokenizer.ggml.tokens", "tokenizer.ggml.token_type", "tokenizer.ggml.merges",
            "tokenizer.ggml.eos_token_id", "tokenizer.chat_template"] {
            let mut values = metadata();
            values.remove(key);
            assert!(tokenizer_from_qwen3_gguf_metadata(&values).is_err(), "missing {key}");
        }
        for (key, value) in [
            ("general.architecture", Value::String("llama".into())),
            ("tokenizer.ggml.model", Value::String("llama".into())),
            ("tokenizer.ggml.pre", Value::String("unknown".into())),
            ("tokenizer.ggml.add_bos_token", Value::Bool(true)),
            ("tokenizer.ggml.eos_token_id", Value::U32(u32::MAX)),
            ("tokenizer.ggml.token_type", Value::Array(vec![Value::I32(1)])),
            ("tokenizer.ggml.merges", Value::Array(vec![Value::String("h e".into()); 2])),
        ] {
            let mut values = metadata();
            values.insert(key.into(), value);
            assert!(tokenizer_from_qwen3_gguf_metadata(&values).is_err(), "invalid {key}");
        }
        let mut values = metadata();
        if let Some(Value::Array(tokens)) = values.get_mut("tokenizer.ggml.tokens") {
            tokens[0] = Value::String("missing-byte".into());
        }
        assert!(tokenizer_from_qwen3_gguf_metadata(&values).is_err());
        let mut values = metadata();
        if let Some(Value::Array(types)) = values.get_mut("tokenizer.ggml.token_type") {
            types[260] = Value::I32(1);
        }
        assert!(tokenizer_from_qwen3_gguf_metadata(&values).is_err(), "EOS must be a control token");
    }

    #[tokio::test]
    async fn public_load_rejects_decoded_metadata_and_required_caps_without_publication() {
        let mut runtime = CandleRuntime::default();
        let mut wrong_arch = metadata();
        wrong_arch.insert("general.architecture".into(), Value::String("llama".into()));
        let mut missing_eos = metadata();
        missing_eos.remove("tokenizer.ggml.eos_token_id");
        for values in [wrong_arch, missing_eos] {
            let fixture = gguf_fixture(&values);
            assert!(runtime.load_attested(load_spec(fixture.path(), fixture.sha256(), RuntimeKind::Candle),
                &ModelCapabilities::default()).await.is_err());
            assert_eq!(runtime.tokenizer_cache_len(), 0);
        }
        let malformed = ModelFileFixture::new("broken.gguf", b"GGUF");
        assert!(runtime.load_attested(load_spec(malformed.path(), malformed.sha256(), RuntimeKind::Candle),
            &ModelCapabilities::default()).await.is_err());
        let fixture = gguf_fixture(&metadata());
        let required = ModelCapabilities { supports_lora: true, ..ModelCapabilities::default() };
        let err = runtime.load_attested(load_spec(fixture.path(), fixture.sha256(), RuntimeKind::Candle),
            &required).await.expect_err("unsupported requirement precedes weight construction");
        assert!(err.to_string().contains("missing required capability supports_lora"), "{err}");
        assert_eq!(runtime.tokenizer_cache_len(), 0);
    }

    #[test]
    fn legacy_receipt_json_roundtrips_without_becoming_a_candle_gguf_receipt() {
        // Untrusted representation fixtures test wire compatibility, not attestation.
        let component = serde_json::json!({"sha256": "44".repeat(32), "length_bytes": 32});
        for value in [
            serde_json::json!({"schema_id": "handshake.model_artifact_integrity.candle.v1",
                "bundle_sha256": "55".repeat(32), "weights": component.clone(),
                "config": component.clone(), "tokenizer": null}),
            serde_json::json!({"schema_id": "handshake.model_artifact_integrity.gguf.v1",
                "bundle_sha256": "55".repeat(32), "gguf": component}),
        ] {
            let receipt: RuntimeArtifactIntegrityReceipt = serde_json::from_value(value.clone()).unwrap();
            assert_eq!(serde_json::to_value(&receipt).unwrap(), value);
            match &receipt {
                RuntimeArtifactIntegrityReceipt::Candle(_) => assert!(receipt
                    .validate_for_runtime_expected(RuntimeKind::LlamaCpp, [0x44; 32]).is_err()),
                RuntimeArtifactIntegrityReceipt::LlamaCpp(_) => assert!(receipt
                    .validate_for_runtime_expected(RuntimeKind::Candle, [0x44; 32]).is_err()),
                RuntimeArtifactIntegrityReceipt::CandleGguf(_) => panic!("legacy shape changed variant"),
            }
        }
    }
    fn request(id: ModelId, cancel: CancellationToken) -> GenerateRequest {
        GenerateRequest {
            id, prompt: GenPrompt::from("Continue with one short word: Hello"),
            sampling: SamplingParams { temperature: Some(0.0), seed: Some(17), ..SamplingParams::default() },
            lora_overrides: Vec::new(), steering_overrides: Vec::new(), kv_prefix_handle: None,
            cancel, max_tokens: 8, stop_sequences: Vec::new(), speculative_mode: None, structured_decoding: None,
        }
    }

    async fn collect(runtime: &CandleRuntime, request: GenerateRequest) -> Vec<GeneratedToken> {
        runtime.generate(request).collect::<Vec<_>>().await.into_iter()
            .map(|token| token.expect("real Qwen3 generation")).collect()
    }

    #[tokio::test]
    #[ignore = "requires admitted x64 proof with owner-selected Candle GGUF PATH/SHA256/BINDING"]
    async fn selected_qwen3_real_load_generation_reset_cancel_quiesce_and_unload() {
        let path = std::env::var_os("HANDSHAKE_LOCAL_MODEL_PATH").expect("MissingPrerequisite: HANDSHAKE_LOCAL_MODEL_PATH");
        let expected = std::env::var("HANDSHAKE_LOCAL_MODEL_SHA256").expect("MissingPrerequisite: HANDSHAKE_LOCAL_MODEL_SHA256");
        assert_eq!(std::env::var("HANDSHAKE_LOCAL_MODEL_BINDING").expect("MissingPrerequisite: HANDSHAKE_LOCAL_MODEL_BINDING"), "candle");
        let mut runtime = CandleRuntime::default();
        let attested = runtime.load_attested(load_spec(Path::new(&path), expected.clone(), RuntimeKind::Candle),
            &ModelCapabilities::default()).await.expect("selected captured Qwen3 GGUF must load");
        match &attested.artifact_integrity {
            RuntimeArtifactIntegrityReceipt::CandleGguf(receipt) => {
                assert_eq!(receipt.architecture, "qwen3");
                assert_eq!(receipt.gguf.sha256, expected);
            }
            other => panic!("selected GGUF must produce its native receipt: {other:?}"),
        }
        let digest: [u8; 32] = hex::decode(&expected).expect("canonical SHA256").try_into().expect("32-byte digest");
        attested.artifact_integrity.validate_for_runtime_expected(RuntimeKind::Candle, digest).expect("captured receipt validates configured digest");
        assert!(attested.artifact_integrity.validate_for_runtime_expected(RuntimeKind::LlamaCpp, digest).is_err());
        assert!(!attested.capabilities.supports_lora);
        assert!(!attested.capabilities.supports_activation_steering);
        assert!(!attested.capabilities.supports_embedding);
        assert!(!attested.capabilities.supports_kv_prefix_cache);
        assert!(!attested.capabilities.supports_subquadratic);
        assert!(!attested.capabilities.supports_speculative_draft);
        assert!(!attested.capabilities.supports_eagle3);
        assert_eq!(attested.capabilities.supports_kv_quantization, KvQuantSupport::None);
        assert_eq!(attested.capabilities.embedding_dimension, None);
        let first = collect(&runtime, request(attested.model_id, CancellationToken::new())).await;
        assert!(first.iter().any(|token| !token.text.is_empty()));
        let second = collect(&runtime, request(attested.model_id, CancellationToken::new())).await;
        assert_eq!(first, second, "fresh generation must reset model and decode state");
        let cancel = CancellationToken::new();
        cancel.cancel();
        let mut cancelled = runtime.generate(request(attested.model_id, cancel));
        assert!(matches!(cancelled.next().await, Some(Err(ModelRuntimeError::Cancelled))),
            "pre-cancelled requests return the adapter's existing typed cancellation error");
        assert!(cancelled.next().await.is_none(), "pre-cancelled error stream must terminate");
        drop(cancelled);
        assert_eq!(first, collect(&runtime, request(attested.model_id, CancellationToken::new())).await);
        assert!(runtime.score(attested.model_id, vec![1]).await.is_err());
        assert!(runtime.embed(attested.model_id, "hello").await.is_err());
        assert!(runtime.kv_cache(attested.model_id).is_err());
        assert!(runtime.state_vector(attested.model_id).is_err());
        assert!(runtime.lora_stack(attested.model_id).is_err());
        assert!(runtime.steering_hooks(attested.model_id).is_err());
        runtime.quiesce(std::time::Duration::from_secs(30)).await.expect("real work drained");
        runtime.unload(attested.model_id).await.expect("unload selected model");
        assert!(runtime.capabilities(attested.model_id).is_err());
        assert_eq!(runtime.tokenizer_cache_len(), 0);
    }
}
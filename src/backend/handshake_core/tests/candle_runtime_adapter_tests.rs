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

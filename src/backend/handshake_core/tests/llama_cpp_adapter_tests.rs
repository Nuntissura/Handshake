use std::{fs, path::Path};

use futures::StreamExt;
use handshake_core::model_runtime::{
    llama_cpp::{LlamaCppRuntime, LLAMA_CPP_NATIVE_FEATURE_DISABLED},
    CancellationToken, GenPrompt, GenerateRequest, KvCachePolicy, KvQuantSupport, LoadSpec,
    ModelCapabilities, ModelId, ModelRuntime, ProviderKind, RuntimeKind, SamplingParams,
};
use sha2::{Digest, Sha256};

#[tokio::test]
async fn llama_cpp_load_rejects_non_llamacpp_runtime_before_backend() {
    let fixture = ModelFileFixture::new(b"not a real gguf");
    let mut runtime = LlamaCppRuntime::default();
    let err = runtime
        .load(load_spec(
            fixture.path(),
            fixture.sha256(),
            RuntimeKind::Candle,
            capabilities(false),
        ))
        .await
        .expect_err("runtime kind mismatch must fail");

    assert!(err.to_string().contains("LlamaCpp"), "{err}");
}

#[tokio::test]
async fn llama_cpp_load_rejects_activation_steering_before_backend() {
    let fixture = ModelFileFixture::new(b"not a real gguf");
    let mut runtime = LlamaCppRuntime::default();
    let err = runtime
        .load(load_spec(
            fixture.path(),
            fixture.sha256(),
            RuntimeKind::LlamaCpp,
            capabilities(true),
        ))
        .await
        .expect_err("activation steering must fail for llama.cpp");

    assert!(err.to_string().contains("activation_steering"), "{err}");
}

#[tokio::test]
async fn llama_cpp_load_rejects_sha256_mismatch_before_backend() {
    let fixture = ModelFileFixture::new(&minimal_gguf_bytes());
    let mut runtime = LlamaCppRuntime::default();
    let err = runtime
        .load(load_spec(
            fixture.path(),
            "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            RuntimeKind::LlamaCpp,
            capabilities(false),
        ))
        .await
        .expect_err("sha mismatch must fail");

    assert!(err.to_string().contains("sha256"), "{err}");
    assert!(err.to_string().contains("mismatch"), "{err}");
}

#[cfg(not(feature = "llama-cpp-runtime-engine"))]
#[tokio::test]
async fn llama_cpp_load_without_native_feature_returns_typed_disabled_error_after_validation() {
    let fixture = ModelFileFixture::new(&minimal_gguf_bytes());
    let mut runtime = LlamaCppRuntime::default();
    let err = runtime
        .load(load_spec(
            fixture.path(),
            fixture.sha256(),
            RuntimeKind::LlamaCpp,
            capabilities(false),
        ))
        .await
        .expect_err("native feature disabled must fail after validation");

    assert!(err.to_string().contains(LLAMA_CPP_NATIVE_FEATURE_DISABLED));
}

#[tokio::test]
async fn llama_cpp_runtime_generate_requires_loaded_model_and_other_unimplemented_methods_return_typed_errors(
) {
    let runtime = LlamaCppRuntime::default();
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
fn llama_cpp_native_imports_are_contained_inside_adapter_module() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_root = manifest_dir.join("src");
    let allowed = source_root.join("model_runtime").join("llama_cpp");
    let mut offenders = Vec::new();

    collect_llama_cpp_import_offenders(&source_root, &allowed, &mut offenders);

    assert!(
        offenders.is_empty(),
        "llama_cpp_2 imports must stay inside model_runtime/llama_cpp/**: {offenders:?}"
    );
}

struct ModelFileFixture {
    _tempdir: tempfile::TempDir,
    path: std::path::PathBuf,
}

impl ModelFileFixture {
    fn new(bytes: &[u8]) -> Self {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("tiny.gguf");
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

fn load_spec(
    artifact_path: &Path,
    sha256_expected: String,
    runtime_kind: RuntimeKind,
    declared_capabilities: ModelCapabilities,
) -> LoadSpec {
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
        declared_capabilities,
        provider: ProviderKind::Local,
        engine_origin: None,
        external_engine_import: None,
    }
}

fn capabilities(supports_activation_steering: bool) -> ModelCapabilities {
    ModelCapabilities {
        supports_lora: true,
        supports_kv_prefix_cache: true,
        supports_kv_quantization: KvQuantSupport::Q4,
        supports_activation_steering,
        supports_subquadratic: false,
        supports_speculative_draft: true,
        supports_eagle3: false,
    }
}

fn sha256_file(path: &Path) -> String {
    let bytes = fs::read(path).expect("read fixture");
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

fn minimal_gguf_bytes() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"GGUF");
    bytes.extend_from_slice(&3_u32.to_le_bytes());
    bytes.extend_from_slice(&0_u64.to_le_bytes());
    bytes.extend_from_slice(&0_u64.to_le_bytes());
    bytes
}

fn collect_llama_cpp_import_offenders(root: &Path, allowed: &Path, offenders: &mut Vec<String>) {
    let entries = fs::read_dir(root).unwrap_or_else(|error| {
        panic!("read source dir {}: {error}", root.display());
    });
    for entry in entries {
        let entry = entry.expect("read dir entry");
        let path = entry.path();
        if path.is_dir() {
            collect_llama_cpp_import_offenders(&path, allowed, offenders);
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) != Some("rs") {
            continue;
        }
        let source = fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!("read source file {}: {error}", path.display());
        });
        if source.contains("llama_cpp_2::") && !path.starts_with(allowed) {
            offenders.push(path.display().to_string());
        }
    }
}

use std::{fs, path::Path, sync::Arc};

use handshake_core::model_runtime::{
    llama_cpp::{
        gguf_loader::{
            validate_gguf_magic, GpuLayerOffload, LlamaCppLoadConfig, NOT_GGUF_V2_V3_ARTIFACT,
        },
        tokenizer_cache::{parse_gguf_tokenizer, TokenizerCache},
        LlamaCppRuntime, LLAMA_CPP_NATIVE_FEATURE_DISABLED,
    },
    KvCachePolicy, KvQuantSupport, LoadSpec, ModelCapabilities, ModelId, ModelRuntime,
    ProviderKind, RuntimeKind, SamplingParams,
};
use sha2::{Digest, Sha256};

const GGUF_TYPE_UINT32: u32 = 4;
const GGUF_TYPE_ARRAY: u32 = 9;
const GGUF_TYPE_UINT64: u32 = 10;
const GGUF_TYPE_STRING: u32 = 8;

#[tokio::test]
async fn llama_cpp_loader_rejects_bad_magic_before_native_backend() {
    let fixture = ModelFileFixture::new(b"not a real gguf");
    let mut runtime = LlamaCppRuntime::default();

    let err = runtime
        .load(load_spec(fixture.path(), fixture.sha256()))
        .await
        .expect_err("bad magic must fail before llama_cpp_2 backend load");

    assert!(err.to_string().contains(NOT_GGUF_V2_V3_ARTIFACT), "{err}");
    assert!(
        !err.to_string().contains(LLAMA_CPP_NATIVE_FEATURE_DISABLED),
        "bad magic must not reach native-disabled backend path: {err}"
    );
}

#[test]
fn gguf_loader_accepts_only_v2_or_v3_magic_versions() {
    let v2 = ModelFileFixture::new(&minimal_gguf(2, Vec::new()));
    let v3 = ModelFileFixture::new(&minimal_gguf(3, Vec::new()));
    let v1 = ModelFileFixture::new(&minimal_gguf(1, Vec::new()));

    assert_eq!(validate_gguf_magic(v2.path()).expect("v2 accepted"), 2);
    assert_eq!(validate_gguf_magic(v3.path()).expect("v3 accepted"), 3);

    let err = validate_gguf_magic(v1.path()).expect_err("v1 must be rejected");
    assert!(err.to_string().contains(NOT_GGUF_V2_V3_ARTIFACT), "{err}");
}

#[test]
fn gguf_tokenizer_metadata_extracts_vocab_ids_and_special_tokens() {
    let fixture = ModelFileFixture::new(&minimal_gguf(
        3,
        vec![
            TestGgufKv::array_strings(
                "tokenizer.ggml.tokens",
                vec!["<s>", "hello", "world", "</s>"],
            ),
            TestGgufKv::u32("tokenizer.ggml.bos_token_id", 0),
            TestGgufKv::u64("tokenizer.ggml.eos_token_id", 3),
        ],
    ));

    let tokenizer = parse_gguf_tokenizer(fixture.path()).expect("tokenizer metadata parsed");

    assert_eq!(tokenizer.vocab_size, 4);
    assert_eq!(tokenizer.bos_id, Some(0));
    assert_eq!(tokenizer.eos_id, Some(3));
    assert_eq!(tokenizer.special_tokens, vec!["<s>", "</s>"]);
}

#[test]
fn tokenizer_cache_reuses_parsed_metadata_by_model_id() {
    let fixture = ModelFileFixture::new(&minimal_gguf(
        3,
        vec![TestGgufKv::array_strings(
            "tokenizer.ggml.tokens",
            vec!["<s>", "cached", "</s>"],
        )],
    ));
    let cache = TokenizerCache::default();
    let model_id = ModelId::new_v7();

    let first = cache
        .get_or_parse(model_id, fixture.path())
        .expect("first parse");
    let second = cache
        .get_or_parse(model_id, fixture.path())
        .expect("second fetch");

    assert!(Arc::ptr_eq(&first, &second));
    assert_eq!(cache.parse_count(), 1);
    assert_eq!(cache.get(model_id).expect("cached tokenizer").vocab_size, 3);
}

#[test]
fn llama_cpp_load_config_defaults_are_operator_safe_and_context_declared() {
    let config = LlamaCppLoadConfig::default();

    assert_eq!(config.model.gpu_layers, GpuLayerOffload::CpuOnly);
    assert_eq!(config.model.main_gpu, 0);
    assert!(!config.model.vocab_only);
    assert!(config.model.use_mmap);
    assert!(!config.model.use_mlock);
    assert_eq!(config.context.n_ctx, 8192);
    assert_eq!(config.context.n_batch, 512);
    assert!(config.context.n_threads >= 1);
    assert!(config.context.n_threads <= 8);
    assert!(config.context.embeddings);
    assert!(config.context.causal_attn);
    assert_eq!(config.context.n_seq_max, 1);
}

#[tokio::test]
async fn env_gated_representative_gguf_loads_or_skips_cleanly() {
    let Some(path) = std::env::var_os("HANDSHAKE_TEST_GGUF_PATH").map(std::path::PathBuf::from)
    else {
        return;
    };
    let sha256 = sha256_file(&path);
    let mut runtime = LlamaCppRuntime::default();

    #[cfg(not(feature = "llama-cpp-runtime-engine"))]
    {
        let err = runtime
            .load(load_spec(&path, sha256))
            .await
            .expect_err("native-disabled builds validate then reject the real fixture");

        assert!(
            err.to_string().contains(LLAMA_CPP_NATIVE_FEATURE_DISABLED),
            "{err}"
        );
    }

    #[cfg(feature = "llama-cpp-runtime-engine")]
    {
        let model_id = runtime
            .load(load_spec(&path, sha256))
            .await
            .expect("native-enabled build loads representative GGUF");

        assert_eq!(model_id.as_uuid().get_version_num(), 7);
        assert!(
            runtime
                .tokenizer_cache()
                .get(model_id)
                .expect("tokenizer cached on load")
                .vocab_size
                > 0
        );
        runtime.unload(model_id).await.expect("unload model");
    }
}

struct ModelFileFixture {
    _tempdir: tempfile::TempDir,
    path: std::path::PathBuf,
}

impl ModelFileFixture {
    fn new(bytes: &[u8]) -> Self {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("tiny.gguf");
        fs::write(&path, bytes).expect("write gguf fixture");
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

fn load_spec(artifact_path: &Path, sha256_expected: String) -> LoadSpec {
    LoadSpec {
        artifact_path: artifact_path.to_path_buf(),
        sha256_expected,
        runtime_kind: RuntimeKind::LlamaCpp,
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
            supports_activation_steering: false,
            supports_subquadratic: false,
            supports_speculative_draft: true,
            supports_eagle3: false,
        },
        provider: ProviderKind::Local,
        engine_origin: None,
        external_engine_import: None,
    }
}

fn sha256_file(path: &Path) -> String {
    let bytes = fs::read(path).expect("read fixture");
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

#[derive(Debug)]
struct TestGgufKv {
    key: String,
    value: TestGgufValue,
}

impl TestGgufKv {
    fn u32(key: &str, value: u32) -> Self {
        Self {
            key: key.to_string(),
            value: TestGgufValue::U32(value),
        }
    }

    fn u64(key: &str, value: u64) -> Self {
        Self {
            key: key.to_string(),
            value: TestGgufValue::U64(value),
        }
    }

    fn array_strings(key: &str, value: Vec<&str>) -> Self {
        Self {
            key: key.to_string(),
            value: TestGgufValue::ArrayStrings(
                value.into_iter().map(ToString::to_string).collect(),
            ),
        }
    }
}

#[derive(Debug)]
enum TestGgufValue {
    U32(u32),
    U64(u64),
    ArrayStrings(Vec<String>),
}

fn minimal_gguf(version: u32, metadata: Vec<TestGgufKv>) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"GGUF");
    bytes.extend_from_slice(&version.to_le_bytes());
    bytes.extend_from_slice(&0_u64.to_le_bytes());
    bytes.extend_from_slice(&(metadata.len() as u64).to_le_bytes());

    for entry in metadata {
        write_gguf_string(&mut bytes, &entry.key);
        match entry.value {
            TestGgufValue::U32(value) => {
                bytes.extend_from_slice(&GGUF_TYPE_UINT32.to_le_bytes());
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            TestGgufValue::U64(value) => {
                bytes.extend_from_slice(&GGUF_TYPE_UINT64.to_le_bytes());
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            TestGgufValue::ArrayStrings(values) => {
                bytes.extend_from_slice(&GGUF_TYPE_ARRAY.to_le_bytes());
                bytes.extend_from_slice(&GGUF_TYPE_STRING.to_le_bytes());
                bytes.extend_from_slice(&(values.len() as u64).to_le_bytes());
                for value in values {
                    write_gguf_string(&mut bytes, &value);
                }
            }
        }
    }

    bytes
}

fn write_gguf_string(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u64).to_le_bytes());
    bytes.extend_from_slice(value.as_bytes());
}

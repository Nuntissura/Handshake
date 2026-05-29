use std::{fs, path::Path};

use futures::StreamExt;
use handshake_core::model_runtime::{
    llama_cpp::LlamaCppRuntime, BaseModelTag, CancellationToken, GenPrompt, GenerateRequest,
    KvCachePolicy, KvPrefixHandle, KvQuantSupport, LicenseTag, LoadSpec, LoraDescriptor, LoraId,
    LoraStrength, ModelCapabilities, ModelRuntime, ProviderKind, RuntimeKind, SamplingParams,
};
use sha2::{Digest, Sha256};

#[cfg(feature = "llama-cpp-runtime-engine")]
static LLAMA_CPP_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[tokio::test]
async fn env_gated_loaded_model_exposes_lora_stack_and_validates_base_without_restart() {
    let Some(path) = fixture_gguf_path() else {
        return;
    };
    #[cfg(feature = "llama-cpp-runtime-engine")]
    let _guard = llama_cpp_test_guard();
    let mut runtime = LlamaCppRuntime::default();
    let Some(model_id) =
        load_model_or_skip_native(&mut runtime, &path, "llama-cpp-test-base").await
    else {
        return;
    };

    let stack = runtime
        .lora_stack(model_id)
        .expect("loaded llama.cpp model should expose a LoRA stack");
    let process_id_before = std::process::id();
    let err = stack
        .mount(
            lora_descriptor(&path, "wrong-base"),
            LoraStrength::try_new(1.0).expect("valid strength"),
        )
        .await
        .expect_err("base-model mismatch is rejected before native adapter load");

    assert!(
        err.to_string().contains("base model mismatch"),
        "unexpected error: {err}"
    );
    assert_eq!(
        std::process::id(),
        process_id_before,
        "LoRA validation must not restart the process"
    );
    assert!(
        stack.list_active().is_empty(),
        "failed mount must not leave active LoRA entries"
    );

    runtime.unload(model_id).await.expect("unload model");
}

#[tokio::test]
async fn env_gated_generate_rejects_unmounted_lora_override_before_decoding() {
    let Some(path) = fixture_gguf_path() else {
        return;
    };
    #[cfg(feature = "llama-cpp-runtime-engine")]
    let _guard = llama_cpp_test_guard();
    let mut runtime = LlamaCppRuntime::default();
    let Some(model_id) =
        load_model_or_skip_native(&mut runtime, &path, "llama-cpp-test-base").await
    else {
        return;
    };

    let missing_lora = LoraId::new_v7();
    let mut request = generate_request(model_id);
    request.lora_overrides = vec![missing_lora];
    let mut stream = runtime.generate(request);
    let err = stream
        .next()
        .await
        .expect("LoRA override validation emits one error")
        .expect_err("unmounted LoRA override should fail before decoding");

    assert!(
        err.to_string().contains("not mounted"),
        "unexpected error: {err}"
    );

    runtime.unload(model_id).await.expect("unload model");
}

#[tokio::test]
async fn env_gated_swap_rejects_multi_lora_stack_before_native_adapter_load() {
    let Some(path) = fixture_gguf_path() else {
        return;
    };
    #[cfg(feature = "llama-cpp-runtime-engine")]
    let _guard = llama_cpp_test_guard();
    let mut runtime = LlamaCppRuntime::default();
    let Some(model_id) =
        load_model_or_skip_native(&mut runtime, &path, "llama-cpp-test-base").await
    else {
        return;
    };
    let stack = runtime
        .lora_stack(model_id)
        .expect("loaded llama.cpp model should expose a LoRA stack");

    let err = stack
        .swap(vec![
            (
                lora_descriptor(&path, "llama-cpp-test-base"),
                LoraStrength::try_new(1.0).expect("valid strength"),
            ),
            (
                lora_descriptor(&path, "llama-cpp-test-base"),
                LoraStrength::try_new(0.5).expect("valid strength"),
            ),
        ])
        .await
        .expect_err("llama-cpp-2 wrapper cannot mount multiple active LoRAs");

    assert!(
        err.to_string().contains("one mounted LoRA adapter"),
        "unexpected error: {err}"
    );
    assert!(stack.list_active().is_empty());

    runtime.unload(model_id).await.expect("unload model");
}

#[tokio::test]
async fn env_gated_mount_rejects_second_lora_before_adapter_load() {
    let Some(path) = fixture_gguf_path() else {
        return;
    };
    let Some(lora_path) =
        std::env::var_os("HANDSHAKE_TEST_LORA_PATH").map(std::path::PathBuf::from)
    else {
        return;
    };
    let base_tag = std::env::var("HANDSHAKE_TEST_LORA_BASE_TAG")
        .unwrap_or_else(|_| "llama-cpp-test-base".to_string());
    #[cfg(feature = "llama-cpp-runtime-engine")]
    let _guard = llama_cpp_test_guard();
    let mut runtime = LlamaCppRuntime::default();
    let Some(model_id) = load_model_or_skip_native(&mut runtime, &path, &base_tag).await else {
        return;
    };
    let stack = runtime
        .lora_stack(model_id)
        .expect("loaded llama.cpp model should expose a LoRA stack");
    let mounted_id = LoraId::new_v7();

    stack
        .mount(
            lora_descriptor_with_id(&lora_path, &base_tag, mounted_id),
            LoraStrength::try_new(1.0).expect("valid strength"),
        )
        .await
        .expect("real LoRA fixture mounts");

    let second_id = LoraId::new_v7();
    let missing_path =
        std::env::temp_dir().join(format!("handshake-missing-lora-{second_id}.gguf"));
    let err = stack
        .mount(
            lora_descriptor_without_hash(&missing_path, &base_tag, second_id),
            LoraStrength::try_new(0.5).expect("valid strength"),
        )
        .await
        .expect_err("second active LoRA is rejected before artifact validation");

    assert!(
        err.to_string().contains("one mounted LoRA adapter"),
        "unexpected error: {err}"
    );
    assert!(
        !err.to_string().contains("does not exist"),
        "second mount should fail before touching the missing artifact: {err}"
    );

    stack
        .unmount(mounted_id)
        .await
        .expect("unmount mounted LoRA");
    runtime.unload(model_id).await.expect("unload model");
}

#[tokio::test]
async fn env_gated_unload_drops_mounted_lora_without_model_lifetime_race() {
    let Some(path) = fixture_gguf_path() else {
        return;
    };
    let Some(lora_path) =
        std::env::var_os("HANDSHAKE_TEST_LORA_PATH").map(std::path::PathBuf::from)
    else {
        return;
    };
    let base_tag = std::env::var("HANDSHAKE_TEST_LORA_BASE_TAG")
        .unwrap_or_else(|_| "llama-cpp-test-base".to_string());
    #[cfg(feature = "llama-cpp-runtime-engine")]
    let _guard = llama_cpp_test_guard();
    let mut runtime = LlamaCppRuntime::default();
    let Some(model_id) = load_model_or_skip_native(&mut runtime, &path, &base_tag).await else {
        return;
    };
    let stack = runtime
        .lora_stack(model_id)
        .expect("loaded llama.cpp model should expose a LoRA stack");

    stack
        .mount(
            lora_descriptor_with_id(&lora_path, &base_tag, LoraId::new_v7()),
            LoraStrength::try_new(1.0).expect("valid strength"),
        )
        .await
        .expect("real LoRA fixture mounts");

    runtime
        .unload(model_id)
        .await
        .expect("unload model while LoRA remains mounted");
    drop(stack);
    drop(runtime);
}

#[tokio::test]
async fn env_gated_real_lora_mount_set_override_and_unmount_roundtrip() {
    let Some(path) = fixture_gguf_path() else {
        return;
    };
    let Some(lora_path) =
        std::env::var_os("HANDSHAKE_TEST_LORA_PATH").map(std::path::PathBuf::from)
    else {
        return;
    };
    let base_tag = std::env::var("HANDSHAKE_TEST_LORA_BASE_TAG")
        .unwrap_or_else(|_| "llama-cpp-test-base".to_string());
    #[cfg(feature = "llama-cpp-runtime-engine")]
    let _guard = llama_cpp_test_guard();
    let mut runtime = LlamaCppRuntime::default();
    let Some(model_id) = load_model_or_skip_native(&mut runtime, &path, &base_tag).await else {
        return;
    };
    let stack = runtime
        .lora_stack(model_id)
        .expect("loaded llama.cpp model should expose a LoRA stack");
    let lora_id = LoraId::new_v7();

    stack
        .mount(
            lora_descriptor_with_id(&lora_path, &base_tag, lora_id),
            LoraStrength::try_new(0.75).expect("valid strength"),
        )
        .await
        .expect("real LoRA fixture mounts without process restart");
    let active = stack.list_active();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].id, lora_id);
    assert_eq!(active[0].strength.value(), 0.75);

    stack
        .set_strength(
            lora_id,
            LoraStrength::try_new(1.25).expect("valid strength"),
        )
        .await
        .expect("mounted LoRA strength can be changed");
    assert_eq!(stack.list_active()[0].strength.value(), 1.25);

    let mut request = generate_request(model_id);
    request.lora_overrides = vec![lora_id];
    let mut stream = runtime.generate(request);
    let first = stream
        .next()
        .await
        .expect("LoRA-backed generation emits at least one item")
        .expect("mounted LoRA override should be accepted");
    assert!(
        first.finish_reason.is_some() || !first.text.is_empty(),
        "first LoRA-backed generation item should carry text or finish state"
    );

    let mut kv_request = generate_request(model_id);
    kv_request.lora_overrides = vec![lora_id];
    kv_request.kv_prefix_handle = Some(KvPrefixHandle::from_tokens(&[1]).expect("prefix handle"));
    let mut kv_stream = runtime.generate(kv_request);
    let err = kv_stream
        .next()
        .await
        .expect("LoRA plus KV-prefix emits one error")
        .expect_err("LoRA and KV prefix handles are not yet safely scoped together");
    assert!(
        err.to_string().contains("KV prefix handles are not scoped"),
        "unexpected error: {err}"
    );

    stack.unmount(lora_id).await.expect("unmount mounted LoRA");
    assert!(stack.list_active().is_empty());

    runtime.unload(model_id).await.expect("unload model");
}

fn fixture_gguf_path() -> Option<std::path::PathBuf> {
    std::env::var_os("HANDSHAKE_TEST_GGUF_PATH").map(std::path::PathBuf::from)
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn llama_cpp_test_guard() -> std::sync::MutexGuard<'static, ()> {
    LLAMA_CPP_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

async fn load_model_or_skip_native(
    runtime: &mut LlamaCppRuntime,
    path: &Path,
    base_tag: &str,
) -> Option<handshake_core::model_runtime::ModelId> {
    let spec = load_spec(path, base_tag);
    #[cfg(feature = "llama-cpp-runtime-engine")]
    {
        Some(
            runtime
                .load(spec)
                .await
                .expect("native-enabled build loads representative GGUF"),
        )
    }
    #[cfg(not(feature = "llama-cpp-runtime-engine"))]
    {
        let err = runtime
            .load(spec)
            .await
            .expect_err("native-disabled builds validate then reject the real fixture");
        assert!(
            err.to_string()
                .contains("llama.cpp native engine feature disabled"),
            "{err}"
        );
        None
    }
}

fn generate_request(id: handshake_core::model_runtime::ModelId) -> GenerateRequest {
    GenerateRequest {
        id,
        prompt: GenPrompt::from("Hello"),
        sampling: SamplingParams {
            seed: Some(7),
            ..SamplingParams::default()
        },
        lora_overrides: Vec::new(),
        steering_overrides: Vec::new(),
        kv_prefix_handle: None,
        cancel: CancellationToken::new(),
        max_tokens: 1,
        stop_sequences: vec!["</s>".to_string()],
        speculative_mode: None,
        structured_decoding: None,
    }
}

fn load_spec(artifact_path: &Path, base_tag: &str) -> LoadSpec {
    LoadSpec {
        artifact_path: artifact_path.to_path_buf(),
        sha256_expected: sha256_file(artifact_path),
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
        engine_origin: Some(base_tag.to_string()),
        external_engine_import: None,
    }
}

fn lora_descriptor(path: &Path, base_tag: &str) -> LoraDescriptor {
    lora_descriptor_with_id(path, base_tag, LoraId::new_v7())
}

fn lora_descriptor_with_id(path: &Path, base_tag: &str, id: LoraId) -> LoraDescriptor {
    LoraDescriptor {
        id,
        artifact_path: path.to_path_buf(),
        sha256: sha256_bytes(path),
        rank: 1,
        target_modules: vec!["q_proj".to_string()],
        base_model_compat: BaseModelTag::new(base_tag),
        license_tag: LicenseTag::new("operator-local"),
    }
}

fn lora_descriptor_without_hash(path: &Path, base_tag: &str, id: LoraId) -> LoraDescriptor {
    LoraDescriptor {
        id,
        artifact_path: path.to_path_buf(),
        sha256: [0; 32],
        rank: 1,
        target_modules: vec!["q_proj".to_string()],
        base_model_compat: BaseModelTag::new(base_tag),
        license_tag: LicenseTag::new("operator-local"),
    }
}

fn sha256_file(path: &Path) -> String {
    hex::encode(sha256_bytes(path))
}

fn sha256_bytes(path: &Path) -> [u8; 32] {
    let bytes = fs::read(path).expect("read fixture");
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().into()
}

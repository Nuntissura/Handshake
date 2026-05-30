#![cfg(feature = "candle-runtime-engine")]

use std::{env, fs, path::Path};

use candle_core::{DType, Device, Tensor};
use candle_nn::{VarBuilder, VarMap};
use futures::StreamExt;
use handshake_core::model_runtime::{
    candle::{
        adapter::{candle_rwkv_capabilities, sha256_file, CandleRuntime},
        rwkv_v5::{
            config_value_declares_rwkv_v5, config_value_declares_unversioned_rwkv,
            CandleRwkvV5Model,
        },
        rwkv_v6::{config_value_declares_rwkv_v6, CandleRwkvV6Model},
        CandleSteeringHooks, TransformerModel,
    },
    CancellationToken, GenPrompt, GenerateRequest, KvCachePolicy, KvQuantSupport, LoadSpec, LoraId,
    ModelCapabilities, ModelId, ModelRuntime, ModelRuntimeError, ProviderKind, RuntimeKind,
    SamplingParams, SteeringVectorId,
};

#[test]
fn candle_rwkv_config_detection_accepts_v5_v6_markers_and_rejects_other_arches() {
    let v5 = serde_json::json!({
        "model_type": "rwkv5",
        "architectures": ["Rwkv5ForCausalLM"],
        "hidden_size": 4,
        "num_hidden_layers": 1,
        "vocab_size": 8
    });
    let v6 = serde_json::json!({
        "model_type": "rwkv-v6",
        "architectures": ["Rwkv6ForCausalLM"],
        "hidden_size": 4,
        "num_hidden_layers": 1,
        "vocab_size": 8
    });
    let mamba2 = serde_json::json!({
        "model_type": "mamba2",
        "architectures": ["Mamba2ForCausalLM"],
        "hidden_size": 4,
        "num_hidden_layers": 1,
        "vocab_size": 8
    });
    let generic_rwkv = serde_json::json!({
        "model_type": "rwkv",
        "architectures": ["RwkvForCausalLM"],
        "hidden_size": 4,
        "num_hidden_layers": 1,
        "vocab_size": 8
    });

    assert!(config_value_declares_rwkv_v5(&v5));
    assert!(!config_value_declares_rwkv_v6(&v5));
    assert!(config_value_declares_rwkv_v6(&v6));
    assert!(!config_value_declares_rwkv_v5(&v6));
    assert!(!config_value_declares_rwkv_v5(&mamba2));
    assert!(!config_value_declares_rwkv_v6(&mamba2));
    assert!(config_value_declares_unversioned_rwkv(&generic_rwkv));
    assert!(!config_value_declares_rwkv_v5(&generic_rwkv));
    assert!(!config_value_declares_rwkv_v6(&generic_rwkv));
}

#[test]
fn candle_rwkv_capabilities_are_base_path_only() {
    let declared = ModelCapabilities {
        supports_lora: true,
        supports_kv_prefix_cache: true,
        supports_kv_quantization: KvQuantSupport::Q8,
        supports_activation_steering: true,
        supports_subquadratic: false,
        supports_speculative_draft: true,
        supports_eagle3: true,
    };

    let actual = candle_rwkv_capabilities(&declared);

    // MT-115: LoRA is genuinely wired for the owned RWKV forwards. MT-089/
    // steering-ssm: activation steering is NOT usable end-to-end (capture fails
    // closed via the adapter), so it stays false until SSM capture is wired.
    assert!(actual.supports_lora);
    assert!(!actual.supports_kv_prefix_cache);
    assert_eq!(actual.supports_kv_quantization, KvQuantSupport::None);
    assert!(!actual.supports_activation_steering);
    assert!(actual.supports_subquadratic);
    assert!(!actual.supports_speculative_draft);
    assert!(!actual.supports_eagle3);
}

#[test]
fn candle_rwkv_v5_tiny_model_prefill_step_and_reset_advance_state() {
    let device = Device::Cpu;
    let varmap = VarMap::new();
    let vb = VarBuilder::from_varmap(&varmap, DType::F32, &device);
    let mut model = CandleRwkvV5Model::from_varbuilder_for_model(
        ModelId::new_v7(),
        tiny_rwkv_config(),
        vec![7],
        vb,
        &device,
    )
    .expect("tiny RWKV v5 model constructs");
    let hooks = CandleSteeringHooks::new_for_model(ModelId::new_v7(), 4);
    let prompt = Tensor::new(&[1_u32, 2], &device)
        .and_then(|tensor| tensor.reshape((1, 2)))
        .unwrap();

    let first = model
        .forward(&prompt, &hooks, &[], &[])
        .expect("v5 prefill works");
    assert_eq!(first.dims(), &[8]);
    assert_eq!(model.state_position(), 2);
    assert_eq!(model.state_tensor_count(), 3);

    let step = Tensor::new(&[3_u32], &device)
        .and_then(|tensor| tensor.reshape((1, 1)))
        .unwrap();
    let next = model
        .forward(&step, &hooks, &[], &[])
        .expect("v5 step works");
    assert_eq!(next.dims(), &[8]);
    assert_eq!(model.state_position(), 3);

    model.reset_generation_state().unwrap();
    let replay = model
        .forward(&prompt, &hooks, &[], &[])
        .expect("v5 prefill replays after reset");
    assert_eq!(model.state_position(), 2);
    assert_eq!(
        first.to_vec1::<f32>().unwrap(),
        replay.to_vec1::<f32>().unwrap()
    );
}

#[test]
fn candle_rwkv_v6_tiny_model_prefill_step_and_reset_advance_state() {
    let device = Device::Cpu;
    let varmap = VarMap::new();
    let vb = VarBuilder::from_varmap(&varmap, DType::F32, &device);
    let mut model = CandleRwkvV6Model::from_varbuilder_for_model(
        ModelId::new_v7(),
        tiny_rwkv_config(),
        vec![7],
        vb,
        &device,
    )
    .expect("tiny RWKV v6 model constructs");
    let hooks = CandleSteeringHooks::new_for_model(ModelId::new_v7(), 4);
    let prompt = Tensor::new(&[1_u32, 2], &device)
        .and_then(|tensor| tensor.reshape((1, 2)))
        .unwrap();

    let first = model
        .forward(&prompt, &hooks, &[], &[])
        .expect("v6 prefill works");
    assert_eq!(first.dims(), &[8]);
    assert_eq!(model.state_position(), 2);
    assert_eq!(model.state_tensor_count(), 3);

    let step = Tensor::new(&[3_u32], &device)
        .and_then(|tensor| tensor.reshape((1, 1)))
        .unwrap();
    let next = model
        .forward(&step, &hooks, &[], &[])
        .expect("v6 step works");
    assert_eq!(next.dims(), &[8]);
    assert_eq!(model.state_position(), 3);

    model.reset_generation_state().unwrap();
    let replay = model
        .forward(&prompt, &hooks, &[], &[])
        .expect("v6 prefill replays after reset");
    assert_eq!(model.state_position(), 2);
    assert_eq!(
        first.to_vec1::<f32>().unwrap(),
        replay.to_vec1::<f32>().unwrap()
    );
}

#[test]
fn candle_rwkv_v6_current_candle_state_layout_matches_v5_state_layout() {
    let device = Device::Cpu;
    let v5_varmap = VarMap::new();
    let v6_varmap = VarMap::new();
    let v5 = CandleRwkvV5Model::from_varbuilder_for_model(
        ModelId::new_v7(),
        tiny_rwkv_config(),
        vec![7],
        VarBuilder::from_varmap(&v5_varmap, DType::F32, &device),
        &device,
    )
    .expect("tiny RWKV v5 model constructs");
    let v6 = CandleRwkvV6Model::from_varbuilder_for_model(
        ModelId::new_v7(),
        tiny_rwkv_config(),
        vec![7],
        VarBuilder::from_varmap(&v6_varmap, DType::F32, &device),
        &device,
    )
    .expect("tiny RWKV v6 model constructs");

    assert_eq!(v5.state_tensor_count(), v6.state_tensor_count());
}

#[test]
fn candle_rwkv_models_reject_steering_and_lora_overrides() {
    let device = Device::Cpu;
    let v5_varmap = VarMap::new();
    let v6_varmap = VarMap::new();
    let mut v5 = CandleRwkvV5Model::from_varbuilder_for_model(
        ModelId::new_v7(),
        tiny_rwkv_config(),
        vec![7],
        VarBuilder::from_varmap(&v5_varmap, DType::F32, &device),
        &device,
    )
    .expect("tiny RWKV v5 model constructs");
    let mut v6 = CandleRwkvV6Model::from_varbuilder_for_model(
        ModelId::new_v7(),
        tiny_rwkv_config(),
        vec![7],
        VarBuilder::from_varmap(&v6_varmap, DType::F32, &device),
        &device,
    )
    .expect("tiny RWKV v6 model constructs");
    let hooks = CandleSteeringHooks::new_for_model(ModelId::new_v7(), 4);
    let token = Tensor::new(&[1_u32], &device)
        .and_then(|tensor| tensor.reshape((1, 1)))
        .unwrap();

    assert_override_rejected(v5.forward(&token, &hooks, &[SteeringVectorId::new_v7()], &[]));
    assert_override_rejected(v5.forward(&token, &hooks, &[], &[LoraId::new_v7()]));
    assert_override_rejected(v6.forward(&token, &hooks, &[SteeringVectorId::new_v7()], &[]));
    assert_override_rejected(v6.forward(&token, &hooks, &[], &[LoraId::new_v7()]));
}

/// MT-088/089 (RWKV): prove SSM prefix restore has a real runtime effect for
/// RWKV too — the reset generate() performs before its forward loop must not
/// wipe a freshly restored prefix. Default-CI, real candle RWKV v5 forward.
/// Without the suppress_next_reset fix the restored continuation would equal a
/// fresh forward, not the continuous post-4-token-forward logits.
#[test]
fn candle_rwkv_v5_prefix_restore_survives_generate_reset_and_continues_state() {
    let device = Device::Cpu;
    let varmap = VarMap::new();
    let vb = VarBuilder::from_varmap(&varmap, DType::F32, &device);
    let mut model = CandleRwkvV5Model::from_varbuilder_for_model(
        ModelId::new_v7(),
        tiny_rwkv_config(),
        vec![7],
        vb,
        &device,
    )
    .expect("tiny RWKV v5 model constructs");
    let hooks = CandleSteeringHooks::new_for_model(ModelId::new_v7(), 4);
    let tok = |ids: &[u32]| {
        Tensor::new(ids, &device)
            .and_then(|tensor| tensor.reshape((1, ids.len())))
            .unwrap()
    };

    let _ = model
        .forward(&tok(&[1, 2, 3, 4]), &hooks, &[], &[])
        .expect("prefill 4 tokens");
    let snapshot = model
        .extract_ssm_snapshot()
        .expect("commit SSM snapshot at pos 4");
    let continuous = model
        .forward(&tok(&[5]), &hooks, &[], &[])
        .expect("continuous 5th-token forward")
        .flatten_all()
        .and_then(|t| t.to_vec1::<f32>())
        .unwrap();

    model.reset_generation_state().expect("independent reset");
    let _ = model
        .forward(&tok(&[6, 6, 6]), &hooks, &[], &[])
        .expect("dirtying forward");

    model
        .restore_ssm_snapshot(&snapshot)
        .expect("restore SSM snapshot");
    model
        .reset_generation_state()
        .expect("reset is suppressed for the restored prefix");
    let restored = model
        .forward(&tok(&[5]), &hooks, &[], &[])
        .expect("restored 5th-token forward")
        .flatten_all()
        .and_then(|t| t.to_vec1::<f32>())
        .unwrap();

    assert_eq!(continuous.len(), restored.len());
    for (c, r) in continuous.iter().zip(&restored) {
        assert!(
            (c - r).abs() < 1.0e-4,
            "RWKV v5 restored continuation logits must equal the continuous \
             post-4-token-forward logits (got {c} vs {r}); a non-suppressed reset \
             would have wiped the restored prefix"
        );
    }
}

#[tokio::test]
async fn candle_rwkv_generic_config_returns_version_marker_error_instead_of_llama_fallback() {
    let tempdir = tempfile::tempdir().unwrap();
    let artifact = tempdir.path().join("model.safetensors");
    fs::write(&artifact, b"not a safetensors file").unwrap();
    fs::write(
        tempdir.path().join("config.json"),
        serde_json::json!({
            "model_type": "rwkv",
            "architectures": ["RwkvForCausalLM"],
            "hidden_size": 4,
            "num_hidden_layers": 1,
            "vocab_size": 8
        })
        .to_string(),
    )
    .unwrap();

    let mut runtime = CandleRuntime::default();
    let error = runtime
        .load(load_spec(&artifact))
        .await
        .expect_err("generic RWKV config is rejected before Llama fallback");

    assert!(
        error.to_string().contains("generic RWKV"),
        "unexpected error: {error}"
    );
}

#[tokio::test]
async fn candle_rwkv_v5_env_model_loads_and_can_generate_when_available() {
    load_and_generate_env_rwkv_model("HANDSHAKE_TEST_RWKV_V5_MODEL_DIR").await;
}

#[tokio::test]
async fn candle_rwkv_v6_env_model_loads_and_can_generate_when_available() {
    load_and_generate_env_rwkv_model("HANDSHAKE_TEST_RWKV_V6_MODEL_DIR").await;
}

async fn load_and_generate_env_rwkv_model(env_var: &str) {
    let Some(model_dir) = env::var_os(env_var) else {
        return;
    };
    let model_dir = Path::new(&model_dir);
    let artifact = model_dir.join("model.safetensors");
    let tokenizer = model_dir.join("tokenizer.json");
    if !artifact.is_file() || !tokenizer.is_file() {
        return;
    }
    let mut runtime = CandleRuntime::default();
    let model_id = runtime
        .load(load_spec(&artifact))
        .await
        .expect("env RWKV model loads");
    let capabilities = runtime.capabilities(model_id).unwrap();
    assert!(capabilities.supports_subquadratic);
    assert!(capabilities.supports_lora);
    assert!(!capabilities.supports_kv_prefix_cache);

    let mut stream = runtime.generate(GenerateRequest {
        id: model_id,
        prompt: GenPrompt::from("hello"),
        sampling: SamplingParams {
            temperature: Some(0.0),
            seed: Some(7),
            ..SamplingParams::default()
        },
        lora_overrides: Vec::new(),
        steering_overrides: Vec::new(),
        kv_prefix_handle: None,
        cancel: CancellationToken::new(),
        max_tokens: 1,
        stop_sequences: Vec::new(),
        speculative_mode: None,
        structured_decoding: None,
    });
    let first = stream
        .next()
        .await
        .expect("token result")
        .expect("generate ok");
    assert!(first.finish_reason.is_some() || !first.text.is_empty());
}

// MT-115/116 + MT-089: the owned RWKV forwards now wire LoRA and a steering
// apply-seam, so an override naming an UNREGISTERED steering vector or UNMOUNTED
// LoRA no longer returns CapabilityNotSupported -- it fails because the named
// override is not known/mounted. The safety property under test is unchanged:
// the forward never SILENTLY applies an override it cannot resolve. (SSM steering
// remains gated off as a capability because real-forward capture is fail-closed;
// see candle_rwkv_capabilities and the steering-ssm honesty note.)
fn assert_override_rejected(result: Result<Tensor, ModelRuntimeError>) {
    assert!(
        result.is_err(),
        "forward must reject an unregistered/unmounted override, got Ok: {result:?}"
    );
}

fn tiny_rwkv_config() -> candle_transformers::models::rwkv_v5::Config {
    candle_transformers::models::rwkv_v5::Config {
        vocab_size: 8,
        hidden_size: 4,
        num_hidden_layers: 1,
        attention_hidden_size: 4,
        num_attention_heads: 2,
        head_size: 2,
        intermediate_size: Some(8),
        layer_norm_epsilon: 1e-5,
        rescale_every: 6,
    }
}

fn load_spec(artifact_path: &Path) -> LoadSpec {
    LoadSpec {
        artifact_path: artifact_path.to_path_buf(),
        sha256_expected: sha256_file(artifact_path).unwrap(),
        runtime_kind: RuntimeKind::Candle,
        sampling_defaults: SamplingParams::default(),
        kv_cache_policy: KvCachePolicy::Default {
            quant: KvQuantSupport::None,
            prefix_cache_ttl_seconds: 0,
            max_bytes: None,
        },
        declared_capabilities: ModelCapabilities {
            supports_lora: true,
            supports_kv_prefix_cache: true,
            supports_kv_quantization: KvQuantSupport::Q8,
            supports_activation_steering: true,
            supports_subquadratic: false,
            supports_speculative_draft: true,
            supports_eagle3: true,
        },
        provider: ProviderKind::Local,
        engine_origin: Some(handshake_core::model_runtime::CANDLE_LOCAL_ENGINE_ORIGIN.to_string()),
        external_engine_import: None,
    }
}

#![cfg(feature = "candle-runtime-engine")]

use std::{env, path::Path};

use candle_core::{DType, Device, Tensor};
use candle_nn::{VarBuilder, VarMap};
use futures::StreamExt;
use handshake_core::model_runtime::{
    candle::{
        adapter::{candle_mamba2_capabilities, sha256_file, CandleRuntime},
        mamba2::{config_value_declares_mamba2, CandleMamba2Model},
        CandleSteeringHooks, TransformerModel,
    },
    KvCachePolicy, KvQuantSupport, LoadSpec, ModelCapabilities, ModelId, ModelRuntime,
    ProviderKind, RuntimeKind, SamplingParams,
};

#[test]
fn candle_mamba2_config_detection_accepts_state_spaces_and_transformers_markers() {
    let state_spaces = serde_json::json!({
        "d_model": 2560,
        "n_layer": 64,
        "vocab_size": 50277,
        "ssm_cfg": { "layer": "Mamba2" }
    });
    let transformers = serde_json::json!({
        "model_type": "mamba2",
        "architectures": ["Mamba2ForCausalLM"],
        "hidden_size": 4096,
        "num_hidden_layers": 64,
        "vocab_size": 32768
    });
    let llama = serde_json::json!({
        "model_type": "llama",
        "hidden_size": 8,
        "num_hidden_layers": 1,
        "vocab_size": 16
    });

    assert!(config_value_declares_mamba2(&state_spaces));
    assert!(config_value_declares_mamba2(&transformers));
    assert!(!config_value_declares_mamba2(&llama));
}

#[test]
fn candle_mamba2_capabilities_are_base_path_only() {
    let declared = ModelCapabilities {
        supports_lora: true,
        supports_kv_prefix_cache: true,
        supports_kv_quantization: KvQuantSupport::Q8,
        supports_activation_steering: true,
        supports_subquadratic: false,
        supports_speculative_draft: true,
        supports_eagle3: true,
    };

    let actual = candle_mamba2_capabilities(&declared);

    assert!(!actual.supports_lora);
    assert!(!actual.supports_kv_prefix_cache);
    assert_eq!(actual.supports_kv_quantization, KvQuantSupport::None);
    assert!(!actual.supports_activation_steering);
    assert!(actual.supports_subquadratic);
    assert!(!actual.supports_speculative_draft);
    assert!(!actual.supports_eagle3);
}

#[test]
fn candle_mamba2_tiny_model_prefill_step_and_reset_advance_ssm_state() {
    let device = Device::Cpu;
    let varmap = VarMap::new();
    let vb = VarBuilder::from_varmap(&varmap, DType::F32, &device);
    let config = candle_transformers::models::mamba2::Config {
        d_model: 4,
        n_layer: 1,
        vocab_size: 8,
        d_state: 2,
        expand: 2,
        headdim: 4,
        ngroups: 1,
        pad_vocab_size_multiple: 1,
    };
    let mut model = CandleMamba2Model::from_varbuilder_for_model(
        ModelId::new_v7(),
        config,
        vec![7],
        vb,
        &device,
    )
    .expect("tiny Mamba2 model constructs");
    let hooks = CandleSteeringHooks::new_for_model(ModelId::new_v7(), 4);
    let prompt = Tensor::new(&[1_u32, 2], &device)
        .and_then(|tensor| tensor.reshape((1, 2)))
        .unwrap();

    let first = model
        .forward(&prompt, &hooks, &[], &[])
        .expect("prefill works");
    assert_eq!(first.dims(), &[8]);
    assert_eq!(model.state_position(), 2);

    let step = Tensor::new(&[3_u32], &device)
        .and_then(|tensor| tensor.reshape((1, 1)))
        .unwrap();
    let next = model.forward(&step, &hooks, &[], &[]).expect("step works");
    assert_eq!(next.dims(), &[1, 8]);
    assert_eq!(model.state_position(), 3);

    model.reset_generation_state().unwrap();
    let replay = model
        .forward(&prompt, &hooks, &[], &[])
        .expect("prefill replays after reset");
    assert_eq!(model.state_position(), 2);
    assert_eq!(
        first.to_vec1::<f32>().unwrap(),
        replay.to_vec1::<f32>().unwrap()
    );
}

#[tokio::test]
async fn candle_mamba2_env_model_loads_and_can_generate_when_available() {
    let Some(model_dir) = env::var_os("HANDSHAKE_TEST_MAMBA2_MODEL_DIR") else {
        return;
    };
    let model_dir = Path::new(&model_dir);
    let artifact = model_dir.join("model.safetensors");
    if !artifact.is_file() {
        return;
    }
    let mut runtime = CandleRuntime::default();
    let model_id = runtime
        .load(load_spec(&artifact))
        .await
        .expect("env Mamba2 model loads");
    let capabilities = runtime.capabilities(model_id).unwrap();
    assert!(capabilities.supports_subquadratic);
    assert!(!capabilities.supports_lora);
    assert!(!capabilities.supports_kv_prefix_cache);

    let mut stream = runtime.generate(handshake_core::model_runtime::GenerateRequest {
        id: model_id,
        prompt: handshake_core::model_runtime::GenPrompt::from("hello"),
        sampling: SamplingParams {
            temperature: Some(0.0),
            seed: Some(7),
            ..SamplingParams::default()
        },
        lora_overrides: Vec::new(),
        steering_overrides: Vec::new(),
        kv_prefix_handle: None,
        cancel: handshake_core::model_runtime::CancellationToken::new(),
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

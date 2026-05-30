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

    // MT-115: LoRA is genuinely wired for the owned Mamba2 forward, so lora is
    // honestly supported. MT-089/steering-ssm: activation steering is NOT
    // usable end-to-end (capture fails closed via the adapter), so it stays
    // false until SSM real-forward capture is wired.
    assert!(actual.supports_lora);
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

/// MT-088/089: prove SSM prefix restore has a REAL runtime effect. The reset
/// that `generate()` performs before its forward loop must not wipe a freshly
/// restored prefix. Default-CI, real candle Mamba2 forward (no mock, no env).
///
/// Without the `suppress_next_reset` fix this test fails: the restored-state
/// continuation logits would equal a fresh forward (reset wiped the restore),
/// not the continuous post-4-token-forward logits.
#[test]
fn candle_mamba2_prefix_restore_survives_generate_reset_and_continues_state() {
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

    let tok = |ids: &[u32]| {
        Tensor::new(ids, &device)
            .and_then(|tensor| tensor.reshape((1, ids.len())))
            .unwrap()
    };

    // Continuous reference: forward 4 tokens, snapshot the pos-4 SSM state, then
    // forward a 5th token from that state -> reference continuation logits.
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

    // Dirty the live state with an independent generation pass.
    model.reset_generation_state().expect("independent reset");
    let _ = model
        .forward(&tok(&[6, 6, 6]), &hooks, &[], &[])
        .expect("dirtying forward");

    // Restore the pos-4 snapshot. This arms reset-suppression so the next
    // generate()-style reset preserves the restored prefix.
    model
        .restore_ssm_snapshot(&snapshot)
        .expect("restore SSM snapshot");
    // generate() calls reset_generation_state before forwarding; with the fix it
    // is suppressed exactly once here so the restored prefix survives.
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
            "restored continuation logits must equal the continuous post-4-token-forward \
             logits (got {c} vs {r}); a non-suppressed reset would have wiped the restored prefix"
        );
    }
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
    assert!(capabilities.supports_lora);
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

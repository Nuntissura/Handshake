#![cfg(feature = "candle-runtime-engine")]

use std::{collections::HashMap, env, fs, path::Path};

use candle_core::{DType, Device, Tensor};
use candle_transformers::models::rwkv_v7::ModelVersion;
use futures::StreamExt;
use handshake_core::model_runtime::{
    candle::{
        adapter::{candle_rwkv_capabilities, sha256_file, CandleRuntime},
        rwkv_v7::{config_from_value, config_value_declares_rwkv_v7, CandleRwkvV7Model},
        CandleSteeringHooks, TransformerModel,
    },
    CancellationToken, GenPrompt, GenerateRequest, KvCachePolicy, KvQuantSupport, LoadSpec, LoraId,
    ModelCapabilities, ModelId, ModelRuntime, ModelRuntimeError, ProviderKind, RuntimeKind,
    SamplingParams, SteeringVectorId,
};

#[test]
fn candle_rwkv_v7_config_detection_accepts_hf_goose_markers_and_rejects_older_rwkv() {
    let hf_goose = hf_rwkv7_config();
    let arch_only = serde_json::json!({
        "architectures": ["RWKV-7-GooseForCausalLM"],
        "vocab_size": 8,
        "hidden_size": 4,
        "num_hidden_layers": 1
    });
    let v6 = serde_json::json!({
        "model_type": "rwkv6",
        "architectures": ["Rwkv6ForCausalLM"]
    });

    assert!(config_value_declares_rwkv_v7(&hf_goose));
    assert!(config_value_declares_rwkv_v7(&arch_only));
    assert!(!config_value_declares_rwkv_v7(&v6));
}

#[test]
fn candle_rwkv_v7_config_adapter_maps_current_hf_fields_to_candle_config() {
    let config = config_from_value(&hf_rwkv7_config()).expect("HF rwkv7 config maps");

    assert_eq!(config.version, ModelVersion::V7);
    assert_eq!(config.vocab_size, 65536);
    assert_eq!(config.hidden_size, 768);
    assert_eq!(config.num_hidden_layers, 12);
    assert_eq!(config.head_size, 64);
    assert_eq!(config.intermediate_size, Some(3072));
    assert_eq!(config.rescale_every, 0);
}

#[test]
fn candle_rwkv_v7_config_adapter_rejects_invalid_head_size() {
    let zero_head = serde_json::json!({
        "model_type": "rwkv7",
        "vocab_size": 8,
        "hidden_size": 4,
        "num_hidden_layers": 1,
        "head_dim": 0
    });
    let non_divisible = serde_json::json!({
        "model_type": "rwkv7",
        "vocab_size": 8,
        "hidden_size": 5,
        "num_hidden_layers": 1,
        "head_dim": 2
    });

    assert!(config_from_value(&zero_head)
        .expect_err("zero head rejected")
        .contains("head_size"));
    assert!(config_from_value(&non_divisible)
        .expect_err("non-divisible head rejected")
        .contains("divisible"));
}

#[test]
fn candle_rwkv_v7_tiny_safetensors_prefill_step_and_reset_advance_state() {
    let tempdir = tempfile::tempdir().unwrap();
    let artifact = tempdir.path().join("model.safetensors");
    write_tiny_rwkv7_safetensors(&artifact);
    write_tiny_config(tempdir.path(), "rwkv7", &["RWKV7ForCausalLM"]);

    let device = Device::Cpu;
    let mut model =
        CandleRwkvV7Model::load_safetensors_for_model(ModelId::new_v7(), &artifact, &device)
            .expect("tiny RWKV v7 model loads");
    let hooks = CandleSteeringHooks::new_for_model(ModelId::new_v7(), 4);
    let prompt = Tensor::new(&[1_u32, 2], &device)
        .and_then(|tensor| tensor.reshape((1, 2)))
        .unwrap();

    let first = model
        .forward(&prompt, &hooks, &[], &[])
        .expect("v7 prefill works");
    assert_eq!(first.dims(), &[8]);
    assert_eq!(model.state_position(), 2);
    assert_eq!(model.state_tensor_count(), 3);
    assert!(!model.state_has_dea());

    let step = Tensor::new(&[3_u32], &device)
        .and_then(|tensor| tensor.reshape((1, 1)))
        .unwrap();
    let next = model
        .forward(&step, &hooks, &[], &[])
        .expect("v7 step works");
    assert_eq!(next.dims(), &[8]);
    assert_eq!(model.state_position(), 3);

    model.reset_generation_state().unwrap();
    let replay = model
        .forward(&prompt, &hooks, &[], &[])
        .expect("v7 prefill replays after reset");
    assert_eq!(model.state_position(), 2);
    assert_eq!(
        first.to_vec1::<f32>().unwrap(),
        replay.to_vec1::<f32>().unwrap()
    );
}

#[test]
fn candle_rwkv_v7_rejects_steering_and_lora_overrides() {
    let tempdir = tempfile::tempdir().unwrap();
    let artifact = tempdir.path().join("model.safetensors");
    write_tiny_rwkv7_safetensors(&artifact);
    write_tiny_config(tempdir.path(), "rwkv7", &["RWKV7ForCausalLM"]);

    let device = Device::Cpu;
    let mut model =
        CandleRwkvV7Model::load_safetensors_for_model(ModelId::new_v7(), &artifact, &device)
            .expect("tiny RWKV v7 model loads");
    let hooks = CandleSteeringHooks::new_for_model(ModelId::new_v7(), 4);
    let token = Tensor::new(&[1_u32], &device)
        .and_then(|tensor| tensor.reshape((1, 1)))
        .unwrap();

    assert_override_rejected(model.forward(&token, &hooks, &[SteeringVectorId::new_v7()], &[]));
    assert_override_rejected(model.forward(&token, &hooks, &[], &[LoraId::new_v7()]));
}

#[test]
fn candle_rwkv_v7_rejects_bad_input_shapes() {
    let tempdir = tempfile::tempdir().unwrap();
    let artifact = tempdir.path().join("model.safetensors");
    write_tiny_rwkv7_safetensors(&artifact);
    write_tiny_config(tempdir.path(), "rwkv7", &["RWKV7ForCausalLM"]);

    let device = Device::Cpu;
    let mut model =
        CandleRwkvV7Model::load_safetensors_for_model(ModelId::new_v7(), &artifact, &device)
            .expect("tiny RWKV v7 model loads");
    let hooks = CandleSteeringHooks::new_for_model(ModelId::new_v7(), 4);
    let flat = Tensor::new(&[1_u32, 2], &device).unwrap();
    let multi_batch = Tensor::new(&[1_u32, 2, 3, 4], &device)
        .and_then(|tensor| tensor.reshape((2, 2)))
        .unwrap();
    let empty_seq = Tensor::from_slice(&[] as &[u32], (1, 0), &device).unwrap();

    assert_generate_error(model.forward(&flat, &hooks, &[], &[]));
    assert_generate_error(model.forward(&multi_batch, &hooks, &[], &[]));
    assert_generate_error(model.forward(&empty_seq, &hooks, &[], &[]));
}

#[tokio::test]
async fn candle_runtime_loads_tiny_rwkv_v7_and_clamps_capabilities() {
    let tempdir = tempfile::tempdir().unwrap();
    let artifact = tempdir.path().join("model.safetensors");
    write_tiny_rwkv7_safetensors(&artifact);
    write_tiny_config(tempdir.path(), "rwkv7", &["RWKV7ForCausalLM"]);

    let mut runtime = CandleRuntime::default();
    let id = runtime
        .load(load_spec(&artifact))
        .await
        .expect("tiny RWKV v7 loads through runtime");
    let capabilities = runtime.capabilities(id).unwrap();

    assert_eq!(capabilities, &candle_rwkv_capabilities(&declared_caps()));
    assert!(capabilities.supports_subquadratic);
    // MT-115: LoRA wired for RWKV v7. MT-089/steering-ssm: steering stays false
    // (capture fails closed via the adapter) until SSM capture is wired.
    assert!(capabilities.supports_lora);
    assert!(!capabilities.supports_activation_steering);
}

#[tokio::test]
async fn candle_rwkv_v7_env_model_loads_and_can_generate_when_available() {
    let Some(model_dir) = env::var_os("HANDSHAKE_TEST_RWKV_V7_MODEL_DIR") else {
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
        .expect("env RWKV v7 model loads");
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

fn hf_rwkv7_config() -> serde_json::Value {
    serde_json::json!({
        "architectures": ["RWKV7ForCausalLM"],
        "eos_token_id": 2,
        "head_dim": 64,
        "hidden_ratio": 4.0,
        "hidden_size": 768,
        "intermediate_size": 3072,
        "model_type": "rwkv7",
        "num_hidden_layers": 12,
        "vocab_size": 65536
    })
}

fn write_tiny_config(dir: &Path, model_type: &str, architectures: &[&str]) {
    let config = serde_json::json!({
        "architectures": architectures,
        "eos_token_id": 7,
        "head_dim": 2,
        "hidden_size": 4,
        "intermediate_size": 8,
        "model_type": model_type,
        "num_hidden_layers": 1,
        "vocab_size": 8
    });
    fs::write(dir.join("config.json"), config.to_string()).unwrap();
}

fn write_tiny_rwkv7_safetensors(path: &Path) {
    let device = Device::Cpu;
    let mut tensors = HashMap::new();
    insert_zeros(&mut tensors, "emb.weight", &[8, 4], &device);
    insert_ones(&mut tensors, "blocks.0.ln0.weight", &[4], &device);
    insert_zeros(&mut tensors, "blocks.0.ln0.bias", &[4], &device);
    insert_ones(&mut tensors, "blocks.0.ln1.weight", &[4], &device);
    insert_zeros(&mut tensors, "blocks.0.ln1.bias", &[4], &device);
    insert_ones(&mut tensors, "blocks.0.ln2.weight", &[4], &device);
    insert_zeros(&mut tensors, "blocks.0.ln2.bias", &[4], &device);

    for name in [
        "x_r", "x_w", "x_k", "x_v", "x_a", "x_g", "w0", "a0", "k_k", "k_a",
    ] {
        insert_zeros(
            &mut tensors,
            &format!("blocks.0.att.{name}"),
            &[1, 1, 4],
            &device,
        );
    }
    insert_zeros(&mut tensors, "blocks.0.att.w1", &[4, 2], &device);
    insert_zeros(&mut tensors, "blocks.0.att.w2", &[2, 4], &device);
    insert_zeros(&mut tensors, "blocks.0.att.a1", &[4, 2], &device);
    insert_zeros(&mut tensors, "blocks.0.att.a2", &[2, 4], &device);
    insert_zeros(&mut tensors, "blocks.0.att.v1", &[4, 2], &device);
    insert_zeros(&mut tensors, "blocks.0.att.g1", &[4, 2], &device);
    insert_zeros(&mut tensors, "blocks.0.att.g2", &[2, 4], &device);
    insert_zeros(&mut tensors, "blocks.0.att.r_k", &[2, 2], &device);
    for name in [
        "receptance.weight",
        "key.weight",
        "value.weight",
        "output.weight",
    ] {
        insert_zeros(
            &mut tensors,
            &format!("blocks.0.att.{name}"),
            &[4, 4],
            &device,
        );
    }
    insert_ones(&mut tensors, "blocks.0.att.ln_x.weight", &[4], &device);
    insert_zeros(&mut tensors, "blocks.0.att.ln_x.bias", &[4], &device);

    insert_zeros(&mut tensors, "blocks.0.ffn.x_k", &[1, 1, 4], &device);
    insert_zeros(&mut tensors, "blocks.0.ffn.key.weight", &[8, 4], &device);
    insert_zeros(&mut tensors, "blocks.0.ffn.value.weight", &[4, 8], &device);
    insert_ones(&mut tensors, "ln_out.weight", &[4], &device);
    insert_zeros(&mut tensors, "ln_out.bias", &[4], &device);
    insert_zeros(&mut tensors, "head.weight", &[8, 4], &device);

    candle_core::safetensors::save(&tensors, path).unwrap();
}

fn insert_zeros(
    tensors: &mut HashMap<String, Tensor>,
    name: &str,
    shape: &[usize],
    device: &Device,
) {
    tensors.insert(
        name.to_string(),
        Tensor::zeros(shape, DType::F32, device).unwrap(),
    );
}

fn insert_ones(
    tensors: &mut HashMap<String, Tensor>,
    name: &str,
    shape: &[usize],
    device: &Device,
) {
    tensors.insert(
        name.to_string(),
        Tensor::ones(shape, DType::F32, device).unwrap(),
    );
}

// MT-115/116 + MT-089: the owned RWKV v7 forward now wires LoRA and a steering
// apply-seam, so an override naming an UNREGISTERED steering vector or UNMOUNTED
// LoRA no longer returns CapabilityNotSupported -- it fails because the named
// override is not known/mounted. The safety property is unchanged: the forward
// never SILENTLY applies an override it cannot resolve. (SSM steering stays gated
// off as a capability because real-forward capture is fail-closed.)
fn assert_override_rejected(result: Result<Tensor, ModelRuntimeError>) {
    assert!(
        result.is_err(),
        "forward must reject an unregistered/unmounted override, got Ok: {result:?}"
    );
}

fn assert_generate_error(result: Result<Tensor, ModelRuntimeError>) {
    assert!(
        matches!(result, Err(ModelRuntimeError::GenerateError(_))),
        "expected GenerateError, got {result:?}"
    );
}

fn declared_caps() -> ModelCapabilities {
    ModelCapabilities {
        supports_lora: true,
        supports_kv_prefix_cache: true,
        supports_kv_quantization: KvQuantSupport::Q8,
        supports_activation_steering: true,
        supports_subquadratic: false,
        supports_speculative_draft: true,
        supports_eagle3: true,
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
        declared_capabilities: declared_caps(),
        provider: ProviderKind::Local,
        engine_origin: Some(handshake_core::model_runtime::CANDLE_LOCAL_ENGINE_ORIGIN.to_string()),
        external_engine_import: None,
    }
}

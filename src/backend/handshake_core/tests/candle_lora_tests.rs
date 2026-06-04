#![cfg(feature = "candle-runtime-engine")]

use std::{collections::HashMap, path::Path, sync::Arc};

use candle_core::{DType, Device, Tensor};
use futures::StreamExt;
use handshake_core::model_runtime::{
    candle::{
        generate::{candle_generate_stream, CandleGenerationCodec},
        lora_impl::{apply_lora_delta_to_linear_output, CandleLoraStack},
        CandleSteeringHooks,
    },
    BaseModelTag, CancellationToken, FinishReason, GenPrompt, GenerateRequest, LicenseTag,
    LoraDescriptor, LoraId, LoraStackOps, LoraStrength, ModelId, ModelRuntimeError, SamplingParams,
};
use sha2::{Digest, Sha256};

#[test]
fn candle_lora_delta_applies_peft_linear_formula_to_rank3_input() {
    let device = Device::Cpu;
    let input = Tensor::from_vec(vec![1.0_f32, 2.0, 3.0, 4.0], (1, 2, 2), &device).unwrap();
    let base_output = Tensor::zeros((1, 2, 3), DType::F32, &device).unwrap();
    let a = Tensor::from_vec(vec![1.0_f32, 0.0, 0.0, 1.0], (2, 2), &device).unwrap();
    let b = Tensor::from_vec(
        vec![
            1.0_f32, 0.0, //
            0.0, 1.0, //
            1.0, 1.0,
        ],
        (3, 2),
        &device,
    )
    .unwrap();

    let adjusted =
        apply_lora_delta_to_linear_output(&base_output, &input, &a, &b, 0.5, "test.q_proj")
            .expect("LoRA delta applies");

    assert_eq!(
        adjusted.to_vec3::<f32>().unwrap(),
        vec![vec![vec![0.5, 1.0, 1.5], vec![1.5, 2.0, 3.5],]]
    );
}

#[tokio::test]
async fn candle_lora_stack_mount_validates_targets_and_rolls_back_failed_swap() {
    let tempdir = tempfile::tempdir().unwrap();
    let good_id = LoraId::new_v7();
    let good_path = tempdir.path().join("good.safetensors");
    write_lora_file(
        &good_path,
        "model.layers.0.self_attn.q_proj",
        &[1.0, 0.0],
        (1, 2),
        &[1.0, 0.0],
        (2, 1),
    );
    let bad_id = LoraId::new_v7();
    let bad_path = tempdir.path().join("bad.safetensors");
    write_lora_file(
        &bad_path,
        "model.layers.0.self_attn.missing_proj",
        &[1.0, 0.0],
        (1, 2),
        &[1.0, 0.0],
        (2, 1),
    );
    let stack = CandleLoraStack::new(
        ModelId::new_v7(),
        "tiny-llama",
        vec!["model.layers.0.self_attn.q_proj".to_string()],
    );

    stack
        .mount(
            descriptor(good_id, &good_path, "model.layers.0.self_attn.q_proj"),
            LoraStrength::try_new(0.75).unwrap(),
        )
        .await
        .expect("valid LoRA mounts");
    assert_eq!(stack.list_active().len(), 1);

    let failed = stack
        .swap(vec![(
            descriptor(bad_id, &bad_path, "model.layers.0.self_attn.missing_proj"),
            LoraStrength::try_new(1.0).unwrap(),
        )])
        .await
        .expect_err("bad target rejects");

    assert!(
        failed.to_string().contains("missing target modules"),
        "{failed}"
    );
    assert_eq!(
        stack
            .list_active()
            .into_iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>(),
        vec![good_id]
    );
}

#[tokio::test]
async fn candle_lora_stack_applies_mounted_delta_for_active_or_matching_override_only() {
    let tempdir = tempfile::tempdir().unwrap();
    let q_id = LoraId::new_v7();
    let q_path = tempdir.path().join("q.safetensors");
    write_lora_file(
        &q_path,
        "model.layers.0.self_attn.q_proj",
        &[1.0, 0.0, 0.0, 1.0],
        (2, 2),
        &[1.0, 0.0, 0.0, 1.0],
        (2, 2),
    );
    let up_id = LoraId::new_v7();
    let up_path = tempdir.path().join("up.safetensors");
    write_lora_file(
        &up_path,
        "model.layers.0.mlp.up_proj",
        &[1.0, 0.0],
        (1, 2),
        &[1.0, 0.0],
        (2, 1),
    );
    let stack = CandleLoraStack::new(
        ModelId::new_v7(),
        "tiny-llama",
        vec![
            "model.layers.0.self_attn.q_proj".to_string(),
            "model.layers.0.mlp.up_proj".to_string(),
        ],
    );
    stack
        .mount(
            descriptor_with_rank(q_id, &q_path, "model.layers.0.self_attn.q_proj", 2),
            LoraStrength::try_new(1.0).unwrap(),
        )
        .await
        .unwrap();
    stack
        .mount(
            descriptor(up_id, &up_path, "model.layers.0.mlp.up_proj"),
            LoraStrength::try_new(1.0).unwrap(),
        )
        .await
        .unwrap();
    let input = Tensor::from_vec(vec![1.0_f32, 2.0], (1, 1, 2), &Device::Cpu).unwrap();
    let base = Tensor::zeros((1, 1, 2), DType::F32, &Device::Cpu).unwrap();

    let active = stack
        .apply_to_linear_output("model.layers.0.self_attn.q_proj", &base, &input, &[])
        .unwrap();
    let explicit = stack
        .apply_to_linear_output("model.layers.0.self_attn.q_proj", &base, &input, &[q_id])
        .unwrap();
    let nonmatching_override = stack
        .apply_to_linear_output("model.layers.0.self_attn.q_proj", &base, &input, &[up_id])
        .unwrap();

    assert_eq!(active.to_vec3::<f32>().unwrap(), vec![vec![vec![1.0, 2.0]]]);
    assert_eq!(
        explicit.to_vec3::<f32>().unwrap(),
        vec![vec![vec![1.0, 2.0]]]
    );
    assert_eq!(
        nonmatching_override.to_vec3::<f32>().unwrap(),
        vec![vec![vec![0.0, 0.0]]]
    );
}

#[tokio::test]
async fn candle_lora_stack_loads_peft_prefixed_keys_and_uses_adapter_alpha() {
    let tempdir = tempfile::tempdir().unwrap();
    let id = LoraId::new_v7();
    let path = tempdir.path().join("adapter_model.safetensors");
    write_lora_file(
        &path,
        "base_model.model.model.layers.0.self_attn.q_proj",
        &[1.0, 0.0],
        (1, 2),
        &[1.0, 0.0],
        (2, 1),
    );
    write_adapter_config(tempdir.path(), &["q_proj"], 1, 2.0);
    let stack = CandleLoraStack::new(
        ModelId::new_v7(),
        "tiny-llama",
        vec!["model.layers.0.self_attn.q_proj".to_string()],
    );
    stack
        .mount(
            descriptor(id, &path, "q_proj"),
            LoraStrength::try_new(1.0).unwrap(),
        )
        .await
        .expect("PEFT LoRA mounts");

    let input = Tensor::from_vec(vec![3.0_f32, 4.0], (1, 1, 2), &Device::Cpu).unwrap();
    let base = Tensor::zeros((1, 1, 2), DType::F32, &Device::Cpu).unwrap();
    let adjusted = stack
        .apply_to_linear_output("model.layers.0.self_attn.q_proj", &base, &input, &[id])
        .unwrap();

    assert_eq!(
        adjusted.to_vec3::<f32>().unwrap(),
        vec![vec![vec![6.0, 0.0]]]
    );
}

#[tokio::test]
async fn candle_lora_stack_rejects_extra_lora_target_pairs_and_empty_license() {
    let tempdir = tempfile::tempdir().unwrap();
    let path = tempdir.path().join("adapter_model.safetensors");
    let device = Device::Cpu;
    let mut tensors = HashMap::new();
    tensors.insert(
        "model.layers.0.self_attn.q_proj.lora_A.weight".to_string(),
        Tensor::from_slice(&[1.0_f32, 0.0], (1, 2), &device).unwrap(),
    );
    tensors.insert(
        "model.layers.0.self_attn.q_proj.lora_B.weight".to_string(),
        Tensor::from_slice(&[1.0_f32, 0.0], (2, 1), &device).unwrap(),
    );
    tensors.insert(
        "model.layers.0.self_attn.typo_proj.lora_A.weight".to_string(),
        Tensor::from_slice(&[1.0_f32, 0.0], (1, 2), &device).unwrap(),
    );
    tensors.insert(
        "model.layers.0.self_attn.typo_proj.lora_B.weight".to_string(),
        Tensor::from_slice(&[1.0_f32, 0.0], (2, 1), &device).unwrap(),
    );
    candle_core::safetensors::save(&tensors, &path).unwrap();
    let stack = CandleLoraStack::new(
        ModelId::new_v7(),
        "tiny-llama",
        vec!["model.layers.0.self_attn.q_proj".to_string()],
    );
    let extra_err = stack
        .mount(
            descriptor(LoraId::new_v7(), &path, "model.layers.0.self_attn.q_proj"),
            LoraStrength::try_new(1.0).unwrap(),
        )
        .await
        .expect_err("extra LoRA tensor target rejects");
    assert!(
        extra_err.to_string().contains("extra target modules"),
        "{extra_err}"
    );

    let clean_path = tempdir.path().join("clean.safetensors");
    write_lora_file(
        &clean_path,
        "model.layers.0.self_attn.q_proj",
        &[1.0, 0.0],
        (1, 2),
        &[1.0, 0.0],
        (2, 1),
    );
    let mut value = serde_json::to_value(descriptor(
        LoraId::new_v7(),
        &clean_path,
        "model.layers.0.self_attn.q_proj",
    ))
    .unwrap();
    value["license_tag"] = serde_json::json!("");
    let empty_license_desc: LoraDescriptor = serde_json::from_value(value).unwrap();
    let license_err = stack
        .mount(empty_license_desc, LoraStrength::try_new(1.0).unwrap())
        .await
        .expect_err("empty license tag rejects at mount");
    assert!(
        license_err.to_string().contains("license tag"),
        "{license_err}"
    );
}

#[tokio::test]
async fn candle_generate_stream_allows_mounted_lora_override() {
    let model_id = ModelId::new_v7();
    let lora_id = LoraId::new_v7();
    let mut req = request(model_id, CancellationToken::new(), 2);
    req.lora_overrides = vec![lora_id];
    let mut stream = candle_generate_stream(
        Arc::new(std::sync::Mutex::new(Box::new(
            FakeTransformer::with_mounted_lora(lora_id),
        ))),
        Arc::new(FakeCodec),
        CandleSteeringHooks::new_for_model(model_id, 2),
        req,
        CancellationToken::new(),
    );

    let first = stream.next().await.unwrap().unwrap();
    assert_eq!(first.text, "A");
    let second = stream.next().await.unwrap().unwrap();
    assert_eq!(second.finish_reason, Some(FinishReason::Stop));
}

fn write_lora_file(
    path: &Path,
    target: &str,
    a_values: &[f32],
    a_shape: (usize, usize),
    b_values: &[f32],
    b_shape: (usize, usize),
) {
    let device = Device::Cpu;
    let mut tensors = HashMap::new();
    tensors.insert(
        format!("{target}.lora_A.weight"),
        Tensor::from_slice(a_values, a_shape, &device).unwrap(),
    );
    tensors.insert(
        format!("{target}.lora_B.weight"),
        Tensor::from_slice(b_values, b_shape, &device).unwrap(),
    );
    candle_core::safetensors::save(&tensors, path).unwrap();
}

fn write_adapter_config(dir: &Path, targets: &[&str], rank: u32, alpha: f32) {
    let target_values = targets
        .iter()
        .map(|target| serde_json::Value::String((*target).to_string()))
        .collect::<Vec<_>>();
    let config = serde_json::json!({
        "peft_type": "LORA",
        "target_modules": target_values,
        "r": rank,
        "lora_alpha": alpha,
        "base_model_name_or_path": "tiny-llama"
    });
    std::fs::write(
        dir.join("adapter_config.json"),
        serde_json::to_vec_pretty(&config).unwrap(),
    )
    .unwrap();
}

fn descriptor(id: LoraId, path: &Path, target: &str) -> LoraDescriptor {
    descriptor_with_rank(id, path, target, 1)
}

fn descriptor_with_rank(id: LoraId, path: &Path, target: &str, rank: u32) -> LoraDescriptor {
    LoraDescriptor {
        id,
        artifact_path: path.to_path_buf(),
        sha256: sha256_bytes(path),
        rank,
        target_modules: vec![target.to_string()],
        base_model_compat: BaseModelTag::new("tiny-llama"),
        license_tag: LicenseTag::new("test-license"),
    }
}

fn sha256_bytes(path: &Path) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(std::fs::read(path).unwrap());
    hasher.finalize().into()
}

fn request(id: ModelId, cancel: CancellationToken, max_tokens: u32) -> GenerateRequest {
    GenerateRequest {
        id,
        prompt: GenPrompt::from("prompt"),
        sampling: SamplingParams {
            temperature: Some(0.0),
            top_p: None,
            top_k: None,
            min_p: None,
            repetition_penalty: None,
            frequency_penalty: None,
            presence_penalty: None,
            seed: Some(7),
        },
        lora_overrides: Vec::new(),
        steering_overrides: Vec::new(),
        kv_prefix_handle: None,
        cancel,
        max_tokens,
        stop_sequences: Vec::new(),
        speculative_mode: None,
        structured_decoding: None,
    }
}

#[derive(Default)]
struct FakeTransformer {
    calls: usize,
    mounted_loras: Vec<LoraId>,
}

impl FakeTransformer {
    fn with_mounted_lora(id: LoraId) -> Self {
        Self {
            calls: 0,
            mounted_loras: vec![id],
        }
    }
}

impl handshake_core::model_runtime::candle::TransformerModel for FakeTransformer {
    fn forward(
        &mut self,
        _input_ids: &Tensor,
        _hooks: &CandleSteeringHooks,
        _steering_overrides: &[handshake_core::model_runtime::SteeringVectorId],
        _lora_overrides: &[LoraId],
    ) -> Result<Tensor, ModelRuntimeError> {
        let token = if self.calls == 0 { 2 } else { 3 };
        self.calls += 1;
        let mut logits = vec![0.0_f32; 4];
        logits[token] = 10.0;
        Tensor::from_vec(logits, 4, &Device::Cpu)
            .map_err(|error| ModelRuntimeError::GenerateError(error.to_string()))
    }

    fn n_layers(&self) -> u32 {
        1
    }

    fn hidden_dim(&self) -> u32 {
        2
    }

    fn vocab_size(&self) -> u32 {
        4
    }

    fn eos_token_ids(&self) -> &[u32] {
        &[3]
    }

    fn device(&self) -> Device {
        Device::Cpu
    }

    fn reset_generation_state(&mut self) -> Result<(), ModelRuntimeError> {
        self.calls = 0;
        Ok(())
    }

    fn validate_lora_overrides(&self, ids: &[LoraId]) -> Result<(), ModelRuntimeError> {
        let missing = ids
            .iter()
            .filter(|id| !self.mounted_loras.contains(id))
            .collect::<Vec<_>>();
        if missing.is_empty() {
            Ok(())
        } else {
            Err(ModelRuntimeError::LoraStackError(
                "fake transformer saw unmounted LoRA override".to_string(),
            ))
        }
    }
}

struct FakeCodec;

impl CandleGenerationCodec for FakeCodec {
    fn encode_prompt(&self, _prompt: &str) -> Result<Vec<u32>, ModelRuntimeError> {
        Ok(vec![1])
    }

    fn decode_token(&self, token_id: u32) -> Result<String, ModelRuntimeError> {
        Ok(match token_id {
            2 => "A",
            3 => "",
            _ => "?",
        }
        .to_string())
    }
}

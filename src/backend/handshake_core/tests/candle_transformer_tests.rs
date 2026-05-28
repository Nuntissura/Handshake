#![cfg(feature = "candle-runtime-engine")]

use std::{
    env, fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::llama::Config as LlamaRuntimeConfig;
use futures::StreamExt;
use handshake_core::model_runtime::{
    candle::{
        adapter::candle_transformer_capabilities,
        generate::{candle_generate_stream, CandleGenerationCodec},
        transformer::{CandleLlamaModel, TransformerModel},
        CandleSteeringHooks,
    },
    CancellationToken, FinishReason, GenPrompt, GenerateRequest, HookPoint, KvCachePolicy,
    KvPrefixHandle, KvQuantSupport, LayerIndex, LoadSpec, LoraId, ModelCapabilities, ModelId,
    ModelRuntime, ModelRuntimeError, ProviderKind, RuntimeKind, SamplingParams, SteeringProvenance,
    SteeringVector, SteeringVectorValues, CANDLE_LOCAL_ENGINE_ORIGIN,
};
use sha2::{Digest, Sha256};

#[test]
fn candle_hooks_apply_vector_snapshot_to_real_tensor_rows() {
    let vector = SteeringVector::try_new(
        None,
        "tensor-vector",
        LayerIndex::new(1),
        HookPoint::ResidStream,
        SteeringVectorValues::try_new(vec![1.0, 0.0], 1.0).unwrap(),
        "tensor steering vector",
        Some(SteeringProvenance::Manual {
            author: "test".to_string(),
            notes: "tensor application proof".to_string(),
        }),
    )
    .unwrap();
    let activation = Tensor::from_slice(&[1.0_f32, 2.0, 3.0, 4.0], (2, 2), &Device::Cpu)
        .expect("activation tensor");

    let adjusted = CandleSteeringHooks::apply_vector_snapshot_to_tensor(
        LayerIndex::new(1),
        HookPoint::ResidStream,
        &activation,
        &[vector],
    )
    .expect("tensor steering applies");

    assert_eq!(
        adjusted.to_vec2::<f32>().unwrap(),
        vec![vec![2.0, 2.0], vec![4.0, 4.0]]
    );
}

#[test]
fn candle_llama_forward_records_real_layer_events() {
    let device = Device::Cpu;
    let config = tiny_llama_config();
    let vb = VarBuilder::zeros(DType::F32, &device);
    let mut model = CandleLlamaModel::from_varbuilder(config, vb, &device)
        .expect("tiny zero-weight model loads");
    let model_id = ModelId::new_v7();
    let hooks = CandleSteeringHooks::new_for_model(model_id, 4);
    let input = Tensor::new(&[1_u32, 2], &device)
        .and_then(|tensor| tensor.reshape((1, 2)))
        .expect("input tensor");

    hooks
        .begin_real_capture(&[LayerIndex::new(0), LayerIndex::new(1)])
        .expect("begin capture");
    let _ = model
        .forward(&input, &hooks, &[], &[])
        .expect("forward succeeds");
    let captured = hooks
        .finish_real_capture(&[LayerIndex::new(0), LayerIndex::new(1)])
        .expect("finish capture");

    assert_eq!(hooks.forward_layer_count(LayerIndex::new(0)).unwrap(), 1);
    assert_eq!(hooks.forward_layer_count(LayerIndex::new(1)).unwrap(), 1);
    assert_eq!(captured.tokens_seen, 2);
    assert_eq!(
        captured
            .activations
            .get(&LayerIndex::new(0))
            .unwrap()
            .first()
            .unwrap()
            .len(),
        4
    );
}

#[tokio::test]
async fn candle_generate_stream_uses_fake_transformer_sampling_cancel_and_hooks() {
    let model_id = ModelId::new_v7();
    // MT-082: this test exercises scaffold capture on bare hooks (no real
    // forward), so it opts in explicitly; production bare hooks fail closed.
    let hooks = CandleSteeringHooks::new_for_model(model_id, 2).with_scaffold_capture();
    let vector = SteeringVector::try_new(
        None,
        "test-vector",
        LayerIndex::new(5),
        HookPoint::ResidStream,
        SteeringVectorValues::try_new(vec![10.0, 0.0], 0.5).unwrap(),
        "test steering vector",
        Some(SteeringProvenance::Manual {
            author: "test".to_string(),
            notes: "fake transformer hook proof".to_string(),
        }),
    )
    .unwrap();
    let vector_id = hooks.register_vector(vector).await.unwrap();

    let model = Arc::new(Mutex::new(
        Box::new(FakeTransformer::new(vec![2, 3, 4])) as Box<dyn TransformerModel>
    ));
    let codec = Arc::new(FakeCodec);
    let cancel = CancellationToken::new();
    let mut stream = candle_generate_stream(
        model,
        codec,
        hooks.clone(),
        request(model_id, cancel.clone(), 8, vec![vector_id]),
        cancel,
    );

    let mut tokens = Vec::new();
    while let Some(item) = stream.next().await {
        tokens.push(item.unwrap());
    }

    assert_eq!(
        tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>(),
        ["A", "B", ""]
    );
    assert_eq!(
        tokens.last().unwrap().finish_reason,
        Some(FinishReason::Stop)
    );
    let captured = hooks
        .capture(handshake_core::model_runtime::CaptureSpec {
            prompts: vec!["after generation".to_string()],
            layers: vec![LayerIndex::new(5)],
            hook_point: HookPoint::ResidStream,
        })
        .await
        .unwrap();
    assert!(captured.activations.contains_key(&LayerIndex::new(5)));
}

#[tokio::test]
async fn candle_generate_stream_cancels_before_forward_work() {
    let model_id = ModelId::new_v7();
    let cancel = CancellationToken::new();
    cancel.cancel();
    let model = Arc::new(Mutex::new(
        Box::new(FakeTransformer::new(vec![2])) as Box<dyn TransformerModel>
    ));
    let mut stream = candle_generate_stream(
        model,
        Arc::new(FakeCodec),
        CandleSteeringHooks::new_for_model(model_id, 2),
        request(model_id, cancel.clone(), 8, Vec::new()),
        cancel,
    );

    let token = stream.next().await.unwrap().unwrap();
    assert_eq!(token.finish_reason, Some(FinishReason::Cancelled));
}

#[tokio::test]
async fn candle_generate_stream_rejects_kv_prefix_until_supported() {
    let model_id = ModelId::new_v7();
    let hooks = CandleSteeringHooks::new_for_model(model_id, 2);
    let model = Arc::new(Mutex::new(
        Box::new(FakeTransformer::new(vec![2])) as Box<dyn TransformerModel>
    ));
    let mut kv_request = request(model_id, CancellationToken::new(), 8, Vec::new());
    kv_request.kv_prefix_handle = Some(KvPrefixHandle::from_tokens(&[1, 2]).unwrap());
    let mut stream = candle_generate_stream(
        model,
        Arc::new(FakeCodec),
        hooks,
        kv_request,
        CancellationToken::new(),
    );
    let err = stream.next().await.unwrap().unwrap_err();
    assert!(err.to_string().contains("kv prefix"), "{err}");
}

#[test]
fn candle_transformer_capabilities_match_implemented_ops() {
    let declared = ModelCapabilities {
        supports_lora: true,
        supports_kv_prefix_cache: true,
        supports_kv_quantization: KvQuantSupport::Q8,
        supports_activation_steering: true,
        supports_subquadratic: true,
        supports_speculative_draft: true,
        supports_eagle3: true,
    };

    let actual = candle_transformer_capabilities(&declared);

    assert!(actual.supports_lora);
    assert!(!actual.supports_kv_prefix_cache);
    assert_eq!(actual.supports_kv_quantization, KvQuantSupport::None);
    assert!(actual.supports_activation_steering);
    assert!(!actual.supports_subquadratic);
    assert!(!actual.supports_speculative_draft);
    assert!(!actual.supports_eagle3);
}

#[tokio::test]
async fn candle_llama_load_from_env_model_dir_when_present() {
    let Some(model_dir) = env::var_os("HANDSHAKE_TEST_CANDLE_MODEL_DIR").map(PathBuf::from) else {
        return;
    };
    let artifact = model_dir.join("model.safetensors");
    if !artifact.is_file() {
        return;
    }

    let _loaded = CandleLlamaModel::load_safetensors(&artifact, &Device::Cpu)
        .unwrap_or_else(|error| panic!("load {}: {error}", artifact.display()));
}

#[tokio::test]
async fn candle_runtime_load_env_model_generates_when_present() {
    let Some(model_dir) = env::var_os("HANDSHAKE_TEST_CANDLE_MODEL_DIR").map(PathBuf::from) else {
        return;
    };
    let artifact = model_dir.join("model.safetensors");
    if !artifact.is_file() || !model_dir.join("tokenizer.json").is_file() {
        return;
    }

    let mut runtime = handshake_core::model_runtime::candle::CandleRuntime::default();
    let id = runtime.load(load_spec(&artifact)).await.unwrap();
    let mut stream = runtime.generate(GenerateRequest {
        id,
        prompt: GenPrompt::from("Hello"),
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
        cancel: CancellationToken::new(),
        max_tokens: 2,
        stop_sequences: Vec::new(),
        speculative_mode: None,
        structured_decoding: None,
    });

    let first = stream.next().await.expect("stream emits token");
    assert!(first.is_ok(), "{first:?}");

    let capture = runtime
        .steering_hooks(id)
        .unwrap()
        .capture(handshake_core::model_runtime::CaptureSpec {
            prompts: vec!["Hello".to_string()],
            layers: vec![LayerIndex::new(0)],
            hook_point: HookPoint::ResidStream,
        })
        .await
        .expect("runtime hook capture drives real forward");
    assert!(capture.tokens_seen > 0);
    assert!(capture.activations.contains_key(&LayerIndex::new(0)));
}

struct FakeTransformer {
    scripted_tokens: Vec<u32>,
    calls: usize,
}

impl FakeTransformer {
    fn new(scripted_tokens: Vec<u32>) -> Self {
        Self {
            scripted_tokens,
            calls: 0,
        }
    }
}

#[async_trait]
impl TransformerModel for FakeTransformer {
    fn forward(
        &mut self,
        _input_ids: &Tensor,
        hooks: &CandleSteeringHooks,
        steering_overrides: &[handshake_core::model_runtime::SteeringVectorId],
        _lora_overrides: &[LoraId],
    ) -> Result<Tensor, ModelRuntimeError> {
        hooks.run_resid_stream_forward_harness(
            [(LayerIndex::new(5), vec![vec![1.0, 2.0]])]
                .into_iter()
                .collect(),
            &[LayerIndex::new(5)],
            steering_overrides,
        )?;
        let token = self.scripted_tokens.get(self.calls).copied().unwrap_or(4);
        self.calls += 1;
        let mut logits = vec![0.0_f32; 5];
        logits[token as usize] = 10.0;
        Tensor::from_vec(logits, 5, &Device::Cpu)
            .map_err(|error| ModelRuntimeError::GenerateError(error.to_string()))
    }

    fn n_layers(&self) -> u32 {
        6
    }

    fn hidden_dim(&self) -> u32 {
        2
    }

    fn vocab_size(&self) -> u32 {
        5
    }

    fn eos_token_ids(&self) -> &[u32] {
        &[4]
    }

    fn device(&self) -> Device {
        Device::Cpu
    }

    fn reset_generation_state(&mut self) -> Result<(), ModelRuntimeError> {
        self.calls = 0;
        Ok(())
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
            3 => "B",
            4 => "",
            _ => "?",
        }
        .to_string())
    }
}

fn request(
    id: ModelId,
    cancel: CancellationToken,
    max_tokens: u32,
    steering_overrides: Vec<handshake_core::model_runtime::SteeringVectorId>,
) -> GenerateRequest {
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
            seed: Some(42),
        },
        lora_overrides: Vec::new(),
        steering_overrides,
        kv_prefix_handle: None,
        cancel,
        max_tokens,
        stop_sequences: Vec::new(),
        speculative_mode: None,
        structured_decoding: None,
    }
}

fn tiny_llama_config() -> LlamaRuntimeConfig {
    LlamaRuntimeConfig {
        hidden_size: 4,
        intermediate_size: 8,
        vocab_size: 8,
        num_hidden_layers: 2,
        num_attention_heads: 1,
        num_key_value_heads: 1,
        use_flash_attn: false,
        rms_norm_eps: 1e-5,
        rope_theta: 10_000.0,
        bos_token_id: None,
        eos_token_id: None,
        rope_scaling: None,
        max_position_embeddings: 16,
        tie_word_embeddings: false,
    }
}

fn load_spec(artifact_path: &Path) -> LoadSpec {
    LoadSpec {
        artifact_path: artifact_path.to_path_buf(),
        sha256_expected: sha256_file(artifact_path),
        runtime_kind: RuntimeKind::Candle,
        sampling_defaults: SamplingParams::default(),
        kv_cache_policy: KvCachePolicy::Default {
            quant: KvQuantSupport::Q4,
            prefix_cache_ttl_seconds: 0,
            max_bytes: None,
        },
        declared_capabilities: ModelCapabilities {
            supports_activation_steering: true,
            supports_kv_prefix_cache: true,
            ..ModelCapabilities::default()
        },
        provider: ProviderKind::Local,
        engine_origin: Some(CANDLE_LOCAL_ENGINE_ORIGIN.to_string()),
        external_engine_import: None,
    }
}

fn sha256_file(path: &Path) -> String {
    let bytes = fs::read(path).expect("read model artifact");
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

use std::{fs, path::Path};

use futures::StreamExt;
use handshake_core::model_runtime::{
    llama_cpp::{
        generate::{
            generation_preflight, GeneratePreflight, StopSequenceDetector,
            LLAMA_CPP_STRUCTURED_DECODING_UNSUPPORTED,
        },
        sampler::{sampler_plan, SamplerStep, DEFAULT_LLAMA_CPP_SEED},
        LlamaCppRuntime,
    },
    CancellationToken, FinishReason, GenPrompt, GenerateRequest, JsonSchema, KvCachePolicy,
    KvQuantSupport, LoadSpec, ModelCapabilities, ModelId, ModelRuntime, ProviderKind, RuntimeKind,
    SamplingParams,
};
use sha2::{Digest, Sha256};

#[test]
fn llama_cpp_sampler_plan_defaults_to_greedy() {
    let plan = sampler_plan(&SamplingParams::default());

    assert_eq!(plan.seed(), DEFAULT_LLAMA_CPP_SEED);
    assert_eq!(plan.steps(), &[SamplerStep::Greedy]);
}

#[test]
fn llama_cpp_sampler_plan_uses_seeded_distribution_for_stochastic_params() {
    let plan = sampler_plan(&SamplingParams {
        temperature: Some(0.7),
        top_p: Some(0.9),
        top_k: Some(40),
        min_p: Some(0.05),
        repetition_penalty: Some(1.1),
        frequency_penalty: Some(0.2),
        presence_penalty: Some(0.3),
        seed: Some(99),
    });

    assert_eq!(plan.seed(), 99);
    assert_eq!(
        plan.steps(),
        &[
            SamplerStep::Penalties {
                penalty_last_n: -1,
                repetition: 1.1,
                frequency: 0.2,
                presence: 0.3,
            },
            SamplerStep::TopK(40),
            SamplerStep::TopP {
                probability: 0.9,
                min_keep: 1,
            },
            SamplerStep::MinP {
                probability: 0.05,
                min_keep: 1,
            },
            SamplerStep::Temperature(0.7),
            SamplerStep::Dist(99),
        ]
    );
}

#[test]
fn stop_sequence_detector_buffers_cross_chunk_boundaries_without_leaking_stop_text() {
    let mut detector = StopSequenceDetector::new(vec!["</s>".to_string()]);

    let first = detector.push("hello<");
    assert_eq!(first.text, "hello");
    assert!(!first.stopped);

    let second = detector.push("/s>");
    assert_eq!(second.text, "");
    assert!(second.stopped);
    assert_eq!(second.matched_stop.as_deref(), Some("</s>"));
    assert_eq!(detector.flush(), "");
}

#[test]
fn generation_preflight_rejects_structured_decoding_before_backend_work() {
    let err = generation_preflight(&generate_request(
        ModelId::new_v7(),
        CancellationToken::new(),
        8,
        Some(JsonSchema::new(serde_json::json!({ "type": "object" }))),
    ))
    .expect_err("structured decoding is not supported by the llama.cpp adapter yet");

    assert!(
        err.to_string()
            .contains(LLAMA_CPP_STRUCTURED_DECODING_UNSUPPORTED),
        "{err}"
    );
}

#[test]
fn generation_preflight_short_circuits_cancel_and_zero_length_requests() {
    let cancel = CancellationToken::new();
    cancel.cancel();
    assert_eq!(
        generation_preflight(&generate_request(ModelId::new_v7(), cancel, 8, None,))
            .expect("cancel is a terminal preflight result"),
        GeneratePreflight::AlreadyCancelled
    );

    assert_eq!(
        generation_preflight(&generate_request(
            ModelId::new_v7(),
            CancellationToken::new(),
            0,
            None,
        ))
        .expect("zero max tokens is a terminal preflight result"),
        GeneratePreflight::LengthCapped
    );
}

#[tokio::test]
async fn llama_cpp_generate_unknown_model_returns_typed_error() {
    let runtime = LlamaCppRuntime::new(KvCachePolicy::default());
    let mut stream = runtime.generate(generate_request(
        ModelId::new_v7(),
        CancellationToken::new(),
        8,
        None,
    ));

    let err = stream
        .next()
        .await
        .expect("generate emits one error")
        .expect_err("unknown model must fail before backend work");

    assert!(err.to_string().contains("not loaded"), "{err}");
    assert!(!err.to_string().contains("not implemented"), "{err}");
}

#[test]
fn terminal_generated_tokens_carry_finish_reasons() {
    let cancelled =
        handshake_core::model_runtime::llama_cpp::generate::terminal_token(FinishReason::Cancelled);
    let length =
        handshake_core::model_runtime::llama_cpp::generate::terminal_token(FinishReason::Length);

    assert_eq!(cancelled.text, "");
    assert_eq!(cancelled.finish_reason, Some(FinishReason::Cancelled));
    assert_eq!(length.finish_reason, Some(FinishReason::Length));
}

#[tokio::test]
async fn env_gated_representative_gguf_streams_native_generation_or_skips_cleanly() {
    let Some(path) = std::env::var_os("HANDSHAKE_TEST_GGUF_PATH").map(std::path::PathBuf::from)
    else {
        return;
    };
    let mut runtime = LlamaCppRuntime::default();
    let sha256 = sha256_file(&path);

    #[cfg(not(feature = "llama-cpp-runtime-engine"))]
    {
        let err = runtime
            .load(load_spec(&path, sha256))
            .await
            .expect_err("native-disabled builds validate then reject the real fixture");

        assert!(
            err.to_string().contains(
                handshake_core::model_runtime::llama_cpp::LLAMA_CPP_NATIVE_FEATURE_DISABLED
            ),
            "{err}"
        );
    }

    #[cfg(feature = "llama-cpp-runtime-engine")]
    {
        let model_id = runtime
            .load(load_spec(&path, sha256))
            .await
            .expect("native-enabled build loads representative GGUF");

        let cancel = CancellationToken::new();
        cancel.cancel();
        let mut cancelled_stream = runtime.generate(generate_request(model_id, cancel, 8, None));
        let cancelled = cancelled_stream
            .next()
            .await
            .expect("cancelled generation emits terminal token")
            .expect("cancelled generation should not error");
        assert_eq!(cancelled.finish_reason, Some(FinishReason::Cancelled));
        assert!(
            cancelled_stream.next().await.is_none(),
            "cancelled generation should terminate after one terminal token"
        );

        let mut stream = runtime.generate(GenerateRequest {
            id: model_id,
            prompt: GenPrompt::from("Hello"),
            sampling: SamplingParams {
                seed: Some(7),
                ..SamplingParams::default()
            },
            lora_overrides: Vec::new(),
            steering_overrides: Vec::new(),
            kv_prefix_handle: None,
            cancel: CancellationToken::new(),
            max_tokens: 4,
            stop_sequences: vec!["</s>".to_string()],
            speculative_mode: None,
            structured_decoding: None,
        });

        let mut terminal = None;
        let mut emitted = String::new();
        let mut token_count = 0_u32;
        while let Some(item) = stream.next().await {
            let token = item.expect("native generation should stream tokens without error");
            emitted.push_str(&token.text);
            token_count += 1;
            if token.finish_reason.is_some() {
                terminal = token.finish_reason;
                break;
            }
            assert!(
                token_count <= 8,
                "native generation emitted too many non-terminal tokens"
            );
        }

        assert!(
            token_count > 0,
            "native generation should emit at least one token"
        );
        assert!(
            matches!(
                terminal,
                Some(FinishReason::Length) | Some(FinishReason::Stop)
            ),
            "native generation should end with length or stop, got {terminal:?}"
        );
        assert!(
            !emitted.contains("</s>"),
            "configured stop sequence should not leak into emitted text"
        );

        runtime.unload(model_id).await.expect("unload model");
    }
}

fn generate_request(
    id: ModelId,
    cancel: CancellationToken,
    max_tokens: u32,
    structured_decoding: Option<JsonSchema>,
) -> GenerateRequest {
    GenerateRequest {
        id,
        prompt: GenPrompt::from("hello"),
        sampling: SamplingParams::default(),
        lora_overrides: Vec::new(),
        steering_overrides: Vec::new(),
        kv_prefix_handle: None,
        cancel,
        max_tokens,
        stop_sequences: Vec::new(),
        speculative_mode: None,
        structured_decoding,
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

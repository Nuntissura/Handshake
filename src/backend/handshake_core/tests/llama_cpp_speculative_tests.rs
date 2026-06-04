#[cfg(feature = "llama-cpp-runtime-engine")]
use std::{
    fs,
    path::Path,
    sync::{Arc, Mutex},
};

#[cfg(feature = "llama-cpp-runtime-engine")]
use futures::StreamExt;
#[cfg(feature = "llama-cpp-runtime-engine")]
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, FlightRecorderEventType, RecorderError,
};
use handshake_core::model_runtime::{
    llama_cpp::generate::{
        generation_preflight, GeneratePreflight, LLAMA_CPP_EAGLE3_UNSUPPORTED,
        LLAMA_CPP_SPECULATIVE_DECODE_UNSUPPORTED,
    },
    llama_cpp::speculative::SpeculativeStats,
    CancellationToken, GenPrompt, GenerateRequest, ModelId, ModelRuntimeError, SamplingParams,
    SpeculativeMode,
};

#[cfg(feature = "llama-cpp-runtime-engine")]
use handshake_core::model_runtime::{
    llama_cpp::LlamaCppRuntime, FinishReason, KvCachePolicy, KvQuantSupport, LoadSpec,
    ModelCapabilities, ModelRuntime, ProviderKind, RuntimeKind,
};

#[cfg(feature = "llama-cpp-runtime-engine")]
use sha2::{Digest, Sha256};

#[test]
fn generation_preflight_accepts_valid_ngram_speculation() {
    assert_preflight_ready(
        generation_preflight(&generate_request(
            ModelId::new_v7(),
            Some(SpeculativeMode::Ngram {
                lookback: 64,
                max_draft: 4,
            }),
        )),
        "valid ngram speculation",
    );
}

#[test]
fn generation_preflight_accepts_valid_draft_model_speculation_without_placeholder_gate() {
    let target_id = ModelId::new_v7();
    let draft_id = ModelId::new_v7();

    assert_preflight_ready(
        generation_preflight(&generate_request(
            target_id,
            Some(SpeculativeMode::DraftModel {
                draft_id,
                max_draft: 4,
            }),
        )),
        "valid draft-model speculation",
    );
}

#[test]
fn generation_preflight_rejects_eagle3_with_explicit_capability_gate() {
    let err = generation_preflight(&generate_request(
        ModelId::new_v7(),
        Some(SpeculativeMode::Eagle3 { max_draft: 4 }),
    ))
    .expect_err("EAGLE3 is explicitly out of scope for this adapter revision");

    assert_capability_error(err, LLAMA_CPP_EAGLE3_UNSUPPORTED);
}

#[test]
fn generation_preflight_rejects_invalid_ngram_parameters_with_validation_error() {
    for (label, mode, expected_terms) in [
        (
            "zero ngram lookback",
            SpeculativeMode::Ngram {
                lookback: 0,
                max_draft: 4,
            },
            ["ngram", "lookback"],
        ),
        (
            "zero ngram max_draft",
            SpeculativeMode::Ngram {
                lookback: 64,
                max_draft: 0,
            },
            ["ngram", "max_draft"],
        ),
    ] {
        let err = generation_preflight(&generate_request(ModelId::new_v7(), Some(mode)))
            .expect_err(label);

        assert_validation_error(err, label, &expected_terms);
    }
}

#[test]
fn generation_preflight_rejects_invalid_draft_model_parameters_with_validation_error() {
    let target_id = ModelId::new_v7();

    for (label, mode, expected_terms) in [
        (
            "zero draft max_draft",
            SpeculativeMode::DraftModel {
                draft_id: ModelId::new_v7(),
                max_draft: 0,
            },
            ["draft", "max_draft"],
        ),
        (
            "draft model matches target model",
            SpeculativeMode::DraftModel {
                draft_id: target_id,
                max_draft: 4,
            },
            ["draft", "target"],
        ),
    ] {
        let err = generation_preflight(&generate_request(target_id, Some(mode))).expect_err(label);

        assert_validation_error(err, label, &expected_terms);
    }
}

#[test]
fn speculative_stats_default_is_empty_and_copyable_without_native_runtime() {
    let stats = SpeculativeStats::default();

    assert_eq!(stats.draft_calls, 0);
    assert_eq!(stats.generated_tokens, 0);
    assert_eq!(stats.accepted_tokens, 0);
    assert_eq!(stats.rejected_tokens, 0);
    assert_eq!(stats.accepted_drafts, 0);
    assert_eq!(stats.rejected_drafts, 0);

    let copied = stats;
    assert_eq!(copied, stats);
}

#[cfg(feature = "llama-cpp-runtime-engine")]
#[tokio::test]
async fn env_gated_llama_cpp_exposes_declared_speculative_capabilities_when_runtime_available() {
    let Some(path) = std::env::var_os("HANDSHAKE_TEST_GGUF_PATH").map(std::path::PathBuf::from)
    else {
        return;
    };
    let sha256 = sha256_file(&path);
    let mut runtime = LlamaCppRuntime::default();
    let model_id = runtime
        .load(load_spec(&path, sha256))
        .await
        .expect("native-enabled build loads representative GGUF");

    let capabilities = runtime
        .capabilities(model_id)
        .expect("loaded model reports effective capabilities");

    assert!(
        capabilities.supports_speculative_draft,
        "llama.cpp must expose declared speculative draft support for MT-077 ngram/draft-model modes"
    );
    assert!(
        !capabilities.supports_eagle3,
        "EAGLE3 remains deferred in this adapter revision"
    );

    runtime.unload(model_id).await.expect("unload model");
}

#[cfg(feature = "llama-cpp-runtime-engine")]
#[tokio::test]
async fn env_gated_ngram_speculation_matches_non_speculative_output_when_runtime_available() {
    let Some(path) = std::env::var_os("HANDSHAKE_TEST_GGUF_PATH").map(std::path::PathBuf::from)
    else {
        return;
    };
    let sha256 = sha256_file(&path);
    let mut runtime = LlamaCppRuntime::default();
    let model_id = runtime
        .load(load_spec(&path, sha256))
        .await
        .expect("native-enabled build loads representative GGUF");

    let baseline = collect_generation(
        &runtime,
        generate_request_with_options(model_id, "alpha beta alpha beta", None, Some(17), 64),
    )
    .await
    .expect("baseline generation streams");
    let speculative = collect_generation(
        &runtime,
        generate_request_with_options(
            model_id,
            "alpha beta alpha beta",
            Some(SpeculativeMode::Ngram {
                lookback: 2,
                max_draft: 2,
            }),
            Some(17),
            64,
        ),
    )
    .await
    .expect("ngram speculative generation streams");

    assert_eq!(
        speculative, baseline,
        "ngram speculative mode must preserve deterministic generation output"
    );
    assert_terminal_finish(baseline.finish);
    let stats = runtime
        .last_speculative_stats(model_id)
        .expect("speculative stats lookup succeeds")
        .expect("speculative generation records stats");
    assert!(
        stats.draft_calls > 0,
        "ngram speculative generation should execute at least one draft round, got {stats:?}"
    );
    assert!(
        stats.generated_tokens > 0,
        "ngram speculative generation should propose draft tokens, got {stats:?}"
    );
    assert!(
        stats.accepted_tokens + stats.rejected_tokens > 0,
        "ngram speculative generation should verify proposed tokens, got {stats:?}"
    );

    runtime.unload(model_id).await.expect("unload model");
}

#[cfg(feature = "llama-cpp-runtime-engine")]
#[tokio::test]
async fn env_gated_draft_model_speculation_matches_non_speculative_output_when_runtime_available() {
    let Some(path) = std::env::var_os("HANDSHAKE_TEST_GGUF_PATH").map(std::path::PathBuf::from)
    else {
        return;
    };
    let sha256 = sha256_file(&path);
    let mut runtime = LlamaCppRuntime::default();
    let target_id = runtime
        .load(load_spec(&path, sha256.clone()))
        .await
        .expect("native-enabled build loads target GGUF");
    let draft_id = runtime
        .load(load_spec(&path, sha256))
        .await
        .expect("native-enabled build loads draft GGUF");

    let baseline = collect_generation(
        &runtime,
        generate_request_with_options(target_id, "hello", None, Some(23), 64),
    )
    .await
    .expect("baseline generation streams");
    let speculative = collect_generation(
        &runtime,
        generate_request_with_options(
            target_id,
            "hello",
            Some(SpeculativeMode::DraftModel {
                draft_id,
                max_draft: 2,
            }),
            Some(23),
            64,
        ),
    )
    .await
    .expect("draft-model speculative generation streams");

    assert_eq!(
        speculative, baseline,
        "draft-model speculative mode must preserve deterministic generation output"
    );
    assert_terminal_finish(baseline.finish);
    let stats = runtime
        .last_speculative_stats(target_id)
        .expect("speculative stats lookup succeeds")
        .expect("speculative generation records stats");
    assert!(
        stats.draft_calls > 0,
        "draft-model speculative generation should execute at least one draft round, got {stats:?}"
    );
    assert!(
        stats.generated_tokens > 0,
        "draft-model speculative generation should propose draft tokens, got {stats:?}"
    );
    assert!(
        stats.accepted_tokens > 0,
        "same-GGUF draft-model speculation should accept at least one verified draft token, got {stats:?}"
    );

    runtime.unload(draft_id).await.expect("unload draft model");
    runtime
        .unload(target_id)
        .await
        .expect("unload target model");
}

#[cfg(feature = "llama-cpp-runtime-engine")]
#[tokio::test]
async fn env_gated_draft_model_error_preserves_default_speculative_stats_when_runtime_available() {
    let Some(path) = std::env::var_os("HANDSHAKE_TEST_GGUF_PATH").map(std::path::PathBuf::from)
    else {
        return;
    };
    let sha256 = sha256_file(&path);
    let mut runtime = LlamaCppRuntime::default();
    let target_id = runtime
        .load(load_spec(&path, sha256))
        .await
        .expect("native-enabled build loads target GGUF");
    let missing_draft_id = ModelId::new_v7();

    let err = collect_generation(
        &runtime,
        generate_request_with_options(
            target_id,
            "hello",
            Some(SpeculativeMode::DraftModel {
                draft_id: missing_draft_id,
                max_draft: 2,
            }),
            Some(29),
            8,
        ),
    )
    .await
    .expect_err("missing draft model must fail before generation");

    assert!(
        err.to_string().contains("draft model is not loaded"),
        "{err}"
    );
    assert_eq!(
        runtime
            .last_speculative_stats(target_id)
            .expect("speculative stats lookup succeeds"),
        Some(SpeculativeStats::default()),
        "speculative stats should preserve a restartable diagnostic record on draft setup errors"
    );

    runtime
        .unload(target_id)
        .await
        .expect("unload target model");
}

#[cfg(feature = "llama-cpp-runtime-engine")]
#[tokio::test]
async fn env_gated_separate_draft_model_speculation_matches_non_speculative_output_when_available()
{
    let Some(target_path) =
        std::env::var_os("HANDSHAKE_TEST_GGUF_PATH").map(std::path::PathBuf::from)
    else {
        return;
    };
    let Some(draft_path) =
        std::env::var_os("HANDSHAKE_TEST_DRAFT_GGUF_PATH").map(std::path::PathBuf::from)
    else {
        return;
    };
    if target_path == draft_path {
        return;
    }

    let mut runtime = LlamaCppRuntime::default();
    let target_id = runtime
        .load(load_spec(&target_path, sha256_file(&target_path)))
        .await
        .expect("native-enabled build loads target GGUF");
    let draft_id = runtime
        .load(load_spec(&draft_path, sha256_file(&draft_path)))
        .await
        .expect("native-enabled build loads separate draft GGUF");

    let baseline = collect_generation(
        &runtime,
        generate_request_with_options(target_id, "hello", None, Some(31), 64),
    )
    .await
    .expect("baseline generation streams");
    let speculative = collect_generation(
        &runtime,
        generate_request_with_options(
            target_id,
            "hello",
            Some(SpeculativeMode::DraftModel {
                draft_id,
                max_draft: 4,
            }),
            Some(31),
            64,
        ),
    )
    .await
    .expect("separate draft-model speculative generation streams");

    assert_eq!(
        speculative, baseline,
        "separate draft-model speculative mode must preserve deterministic token sequence"
    );
    let stats = runtime
        .last_speculative_stats(target_id)
        .expect("speculative stats lookup succeeds")
        .expect("speculative generation records stats");
    assert!(
        stats.draft_calls > 0,
        "separate draft-model speculative generation should execute draft rounds, got {stats:?}"
    );

    runtime.unload(draft_id).await.expect("unload draft model");
    runtime
        .unload(target_id)
        .await
        .expect("unload target model");
}

#[cfg(feature = "llama-cpp-runtime-engine")]
#[tokio::test]
async fn env_gated_ngram_speculation_emits_flight_recorder_spec_events_when_runtime_available() {
    let Some(path) = std::env::var_os("HANDSHAKE_TEST_GGUF_PATH").map(std::path::PathBuf::from)
    else {
        return;
    };
    let recorder = Arc::new(SpecEventRecorder::default());
    let flight_recorder: Arc<dyn FlightRecorder> = recorder.clone();
    let mut runtime =
        LlamaCppRuntime::with_flight_recorder(KvCachePolicy::default(), flight_recorder);
    let model_id = runtime
        .load(load_spec(&path, sha256_file(&path)))
        .await
        .expect("native-enabled build loads representative GGUF");

    collect_generation(
        &runtime,
        generate_request_with_options(
            model_id,
            "alpha beta alpha beta",
            Some(SpeculativeMode::Ngram {
                lookback: 2,
                max_draft: 2,
            }),
            Some(37),
            64,
        ),
    )
    .await
    .expect("ngram speculative generation streams");

    let stats = runtime
        .last_speculative_stats(model_id)
        .expect("speculative stats lookup succeeds")
        .expect("speculative generation records stats");
    let events = wait_for_speculative_events(&recorder).await;
    let has_accept = events
        .iter()
        .any(|event| event.event_type == FlightRecorderEventType::LlmInferenceSpecAccept);
    let has_reject = events
        .iter()
        .any(|event| event.event_type == FlightRecorderEventType::LlmInferenceSpecReject);

    assert!(
        has_accept || has_reject,
        "speculative generation should emit FR-EVT-LLM-INFER-SPEC-ACCEPT or REJECT events, got stats={stats:?}"
    );
    if stats.accepted_tokens > 0 {
        assert!(has_accept, "accepted tokens must emit SPEC-ACCEPT");
    }
    if stats.rejected_tokens > 0 {
        assert!(has_reject, "rejected tokens must emit SPEC-REJECT");
    }

    let model_id_text = model_id.to_string();
    for event in events {
        assert_eq!(event.model_id.as_deref(), Some(model_id_text.as_str()));
        assert_eq!(event.payload["schema_version"], "hsk.fr.llm_infer_spec@0.1");
        assert_eq!(
            event.payload["model_id"].as_str(),
            Some(model_id_text.as_str())
        );
        assert_eq!(event.payload["adapter"], "llama_cpp");
        assert_eq!(event.payload["mode"], "ngram");
        assert!(
            matches!(
                event.payload["event_id"].as_str(),
                Some("FR-EVT-LLM-INFER-SPEC-ACCEPT") | Some("FR-EVT-LLM-INFER-SPEC-REJECT")
            ),
            "unexpected speculative event payload: {}",
            event.payload
        );
    }

    runtime.unload(model_id).await.expect("unload target model");
}

fn generate_request(id: ModelId, speculative_mode: Option<SpeculativeMode>) -> GenerateRequest {
    GenerateRequest {
        id,
        prompt: GenPrompt::from("hello"),
        sampling: SamplingParams::default(),
        lora_overrides: Vec::new(),
        steering_overrides: Vec::new(),
        kv_prefix_handle: None,
        cancel: CancellationToken::new(),
        max_tokens: 8,
        stop_sequences: Vec::new(),
        speculative_mode,
        structured_decoding: None,
    }
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn generate_request_with_options(
    id: ModelId,
    prompt: &str,
    speculative_mode: Option<SpeculativeMode>,
    seed: Option<u32>,
    max_tokens: u32,
) -> GenerateRequest {
    GenerateRequest {
        id,
        prompt: GenPrompt::from(prompt),
        sampling: SamplingParams {
            seed,
            ..SamplingParams::default()
        },
        lora_overrides: Vec::new(),
        steering_overrides: Vec::new(),
        kv_prefix_handle: None,
        cancel: CancellationToken::new(),
        max_tokens,
        stop_sequences: Vec::new(),
        speculative_mode,
        structured_decoding: None,
    }
}

#[cfg(feature = "llama-cpp-runtime-engine")]
async fn collect_generation(
    runtime: &LlamaCppRuntime,
    req: GenerateRequest,
) -> Result<GenerationTrace, ModelRuntimeError> {
    let mut stream = runtime.generate(req);
    let mut token_ids = Vec::new();
    let mut text = String::new();
    let mut finish = None;
    while let Some(item) = stream.next().await {
        let token = item?;
        token_ids.push(token.token_id);
        text.push_str(&token.text);
        if token.finish_reason.is_some() && finish.is_none() {
            finish = token.finish_reason;
        }
    }
    Ok(GenerationTrace {
        token_ids,
        text,
        finish,
    })
}

#[cfg(feature = "llama-cpp-runtime-engine")]
#[derive(Clone, Debug, PartialEq, Eq)]
struct GenerationTrace {
    token_ids: Vec<u32>,
    text: String,
    finish: Option<FinishReason>,
}

#[cfg(feature = "llama-cpp-runtime-engine")]
#[derive(Default)]
struct SpecEventRecorder {
    events: Mutex<Vec<FlightRecorderEvent>>,
}

#[cfg(feature = "llama-cpp-runtime-engine")]
#[async_trait::async_trait]
impl FlightRecorder for SpecEventRecorder {
    async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
        event.validate()?;
        self.events
            .lock()
            .map_err(|_| RecorderError::LockError)?
            .push(event);
        Ok(())
    }

    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        Ok(0)
    }

    async fn list_events(
        &self,
        _filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Ok(self
            .events
            .lock()
            .map_err(|_| RecorderError::LockError)?
            .clone())
    }
}

#[cfg(feature = "llama-cpp-runtime-engine")]
async fn wait_for_speculative_events(recorder: &SpecEventRecorder) -> Vec<FlightRecorderEvent> {
    for _ in 0..20 {
        let events = recorder
            .list_events(EventFilter::default())
            .await
            .expect("list speculative events");
        let speculative_events: Vec<_> = events
            .into_iter()
            .filter(|event| {
                matches!(
                    event.event_type,
                    FlightRecorderEventType::LlmInferenceSpecAccept
                        | FlightRecorderEventType::LlmInferenceSpecReject
                )
            })
            .collect();
        if !speculative_events.is_empty() {
            return speculative_events;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    Vec::new()
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn assert_terminal_finish(finish: Option<FinishReason>) {
    assert!(
        matches!(
            finish,
            Some(FinishReason::Length) | Some(FinishReason::Stop)
        ),
        "generation should end with length or stop, got {finish:?}"
    );
}

fn assert_preflight_ready(result: Result<GeneratePreflight, ModelRuntimeError>, context: &str) {
    match result {
        Ok(GeneratePreflight::Ready) => {}
        Ok(other) => panic!("{context}: expected Ready preflight, got {other:?}"),
        Err(error) => {
            assert_not_old_speculative_placeholder(&error, context);
            panic!("{context}: expected Ready preflight, got {error:?}");
        }
    }
}

fn assert_capability_error(error: ModelRuntimeError, expected_capability: &str) {
    match error {
        ModelRuntimeError::CapabilityNotSupported {
            capability,
            adapter,
        } => {
            assert_eq!(capability, expected_capability);
            assert_eq!(adapter, "llama_cpp");
        }
        other => panic!("expected capability error, got {other}"),
    }
}

fn assert_validation_error(error: ModelRuntimeError, context: &str, expected_terms: &[&str]) {
    assert_not_old_speculative_placeholder(&error, context);
    assert!(
        !matches!(&error, ModelRuntimeError::CapabilityNotSupported { .. }),
        "{context}: invalid speculative parameters must fail validation, not capability gating: {error:?}"
    );

    let message = error.to_string().to_ascii_lowercase();
    assert!(
        !message.contains("not implemented") && !message.contains("not_supported"),
        "{context}: validation error must not be a placeholder: {error}"
    );

    for expected in expected_terms {
        assert!(
            message.contains(expected),
            "{context}: validation error should mention `{expected}`, got: {error}"
        );
    }
}

fn assert_not_old_speculative_placeholder(error: &ModelRuntimeError, context: &str) {
    if let ModelRuntimeError::CapabilityNotSupported {
        capability,
        adapter,
    } = error
    {
        assert!(
            !(capability == LLAMA_CPP_SPECULATIVE_DECODE_UNSUPPORTED && adapter == "llama_cpp"),
            "{context}: MT-077-supported speculative modes must not return the old not-implemented marker `{LLAMA_CPP_SPECULATIVE_DECODE_UNSUPPORTED}`"
        );
    }
}

#[cfg(feature = "llama-cpp-runtime-engine")]
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
            supports_eagle3: true,
        },
        provider: ProviderKind::Local,
        engine_origin: None,
        external_engine_import: None,
    }
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn sha256_file(path: &Path) -> String {
    let bytes = fs::read(path).expect("read fixture");
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

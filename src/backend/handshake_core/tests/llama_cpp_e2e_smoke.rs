use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use futures::StreamExt;
use handshake_core::{
    flight_recorder::{
        events_llm_infer::{FR_EVT_LLM_INFER_END, FR_EVT_LLM_INFER_START, FR_EVT_LLM_INFER_TOKEN},
        EventFilter, FlightRecorder, FlightRecorderEvent, FlightRecorderEventType, RecorderError,
    },
    model_runtime::{
        llama_cpp::LlamaCppRuntime, BaseModelTag, CancellationToken, FinishReason, GenPrompt,
        GenerateRequest, KvCacheOps, KvCachePolicy, KvQuantSupport, LicenseTag, LoadSpec,
        LoraDescriptor, LoraId, LoraStrength, ModelCapabilities, ModelRuntime, ModelRuntimeError,
        ProviderKind, RuntimeKind, SamplingParams, SpeculativeMode,
    },
};
use sha2::{Digest, Sha256};

#[cfg(not(feature = "llama-cpp-runtime-engine"))]
use handshake_core::model_runtime::llama_cpp::LLAMA_CPP_NATIVE_FEATURE_DISABLED;

const TESTS_README_JSON: &str = include_str!("README.json");

#[cfg(feature = "llama-cpp-runtime-engine")]
static LLAMA_CPP_E2E_TEST_LOCK: Mutex<()> = Mutex::new(());

#[tokio::test]
#[ignore = "requires HANDSHAKE_TEST_GGUF_PATH and optional HANDSHAKE_TEST_LORA_PATH"]
async fn llama_cpp_e2e_smoke_load_generate_lora_kv_spec_score_embed_unload() {
    assert_tests_readme_registry_entry();

    let Some(gguf_path) = fixture_gguf_path() else {
        eprintln!("SKIPPED llama_cpp_e2e_smoke: HANDSHAKE_TEST_GGUF_PATH is not set");
        return;
    };
    let base_tag = std::env::var("HANDSHAKE_TEST_LORA_BASE_TAG")
        .unwrap_or_else(|_| "llama-cpp-e2e-base".to_string());
    let recorder = Arc::new(E2eEventRecorder::default());
    let flight_recorder: Arc<dyn FlightRecorder> = recorder.clone();
    let mut runtime =
        LlamaCppRuntime::with_flight_recorder(KvCachePolicy::default(), flight_recorder);
    let sha256 = sha256_file(&gguf_path);

    #[cfg(not(feature = "llama-cpp-runtime-engine"))]
    {
        let err = runtime
            .load(load_spec(&gguf_path, sha256, &base_tag))
            .await
            .expect_err("native-disabled builds validate then reject the real fixture");
        assert!(
            err.to_string().contains(LLAMA_CPP_NATIVE_FEATURE_DISABLED),
            "{err}"
        );
    }

    #[cfg(feature = "llama-cpp-runtime-engine")]
    {
        let _guard = llama_cpp_e2e_test_guard();
        let model_id = runtime
            .load(load_spec(&gguf_path, sha256, &base_tag))
            .await
            .expect("native-enabled build loads representative GGUF");

        assert_eq!(
            runtime
                .capabilities(model_id)
                .expect("loaded model reports capabilities"),
            &expected_capabilities()
        );

        let mut generation_count = 0_usize;
        let mut longest_generation = 0_usize;

        let baseline = collect_generation(
            &runtime,
            generate_request(model_id, baseline_prompt(), 32, Some(42), None),
        )
        .await
        .expect("baseline generation streams");
        assert_nonempty_generation("baseline", &baseline);
        generation_count += 1;
        longest_generation = longest_generation.max(baseline.generated_token_count());

        if let Some(lora_path) = optional_lora_path() {
            let lora_id = LoraId::new_v7();
            let stack = runtime
                .lora_stack(model_id)
                .expect("loaded model exposes LoRA stack");
            stack
                .mount(
                    lora_descriptor(&lora_path, &base_tag, lora_id),
                    LoraStrength::try_new(0.75).expect("valid LoRA strength"),
                )
                .await
                .expect("operator-supplied LoRA fixture mounts");
            assert_eq!(stack.list_active().len(), 1);

            let mut lora_request =
                generate_request(model_id, baseline_prompt(), 32, Some(42), None);
            lora_request.lora_overrides = vec![lora_id];
            let lora_generation = collect_generation(&runtime, lora_request)
                .await
                .expect("LoRA-backed generation streams");
            assert_nonempty_generation("lora", &lora_generation);
            if lora_difference_assertion_enabled() {
                assert_ne!(
                    lora_generation.token_ids, baseline.token_ids,
                    "curated LoRA fixture should change the deterministic token stream"
                );
            }
            generation_count += 1;
            longest_generation = longest_generation.max(lora_generation.generated_token_count());

            stack.unmount(lora_id).await.expect("unmount LoRA");
            assert!(stack.list_active().is_empty());
        } else {
            eprintln!(
                "SKIPPED llama_cpp_e2e_smoke LoRA substep: HANDSHAKE_TEST_LORA_PATH is not set"
            );
        }

        let cache = runtime
            .llama_cpp_kv_cache(model_id)
            .expect("loaded model exposes native KV cache ops");
        assert_eq!(cache.quantization(), KvQuantSupport::None);
        cache
            .set_quantization(KvQuantSupport::Q4)
            .expect("q4 KV quantization is supported");
        assert_eq!(cache.quantization(), KvQuantSupport::Q4);

        let kv_prompt_tokens = runtime
            .tokenize_prompt(model_id, kv_prompt())
            .expect("loaded model tokenizes KV prompt");
        assert!(
            kv_prompt_tokens.len() > 1,
            "KV prompt needs at least one prefix token and one suffix token"
        );
        let prefix_len = 20_usize.min(kv_prompt_tokens.len() - 1);
        let prefix = cache
            .prefix_commit(&kv_prompt_tokens[..prefix_len])
            .expect("prefix commit captures native KV state");
        assert_eq!(cache.occupancy().prefix_cache_entries, 1);

        let mut first_kv_request = generate_request(model_id, kv_prompt(), 16, Some(77), None);
        first_kv_request.kv_prefix_handle = Some(prefix.clone());
        let first_kv = collect_generation(&runtime, first_kv_request)
            .await
            .expect("KV-prefix generation streams");
        assert_nonempty_generation("kv first", &first_kv);
        generation_count += 1;
        longest_generation = longest_generation.max(first_kv.generated_token_count());

        cache
            .prefix_restore(&prefix)
            .expect("known prefix restores before deterministic replay");
        let mut second_kv_request = generate_request(model_id, kv_prompt(), 16, Some(77), None);
        second_kv_request.kv_prefix_handle = Some(prefix);
        let second_kv = collect_generation(&runtime, second_kv_request)
            .await
            .expect("KV-prefix replay streams");
        assert_eq!(
            second_kv, first_kv,
            "same prompt, seed, quantization, and KV prefix must replay deterministically"
        );
        generation_count += 1;
        longest_generation = longest_generation.max(second_kv.generated_token_count());

        let ngram_prompt = select_ngram_prompt(&runtime, model_id)
            .expect("fixture tokenizer must support a repeated 4-token ngram prompt");
        let ngram_baseline = collect_generation(
            &runtime,
            generate_request(model_id, ngram_prompt, 32, Some(42), None),
        )
        .await
        .expect("non-speculative ngram baseline streams");
        generation_count += 1;
        longest_generation = longest_generation.max(ngram_baseline.generated_token_count());

        let ngram_spec = collect_generation(
            &runtime,
            generate_request(
                model_id,
                ngram_prompt,
                32,
                Some(42),
                Some(SpeculativeMode::Ngram {
                    lookback: 4,
                    max_draft: 4,
                }),
            ),
        )
        .await
        .expect("ngram speculative generation streams");
        assert_eq!(
            ngram_spec, ngram_baseline,
            "ngram speculative mode must preserve deterministic generation output"
        );
        generation_count += 1;
        longest_generation = longest_generation.max(ngram_spec.generated_token_count());

        let spec_stats = runtime
            .last_speculative_stats(model_id)
            .expect("speculative stats lookup succeeds")
            .expect("speculative generation records stats");
        assert!(
            spec_stats.draft_calls > 0,
            "repeated ngram prompt should execute at least one draft round, got {spec_stats:?}"
        );
        assert!(
            spec_stats.accepted_tokens + spec_stats.rejected_tokens > 0,
            "ngram speculative generation should verify proposed tokens, got {spec_stats:?}"
        );

        let ngram_baseline_after_spec = collect_generation(
            &runtime,
            generate_request(model_id, ngram_prompt, 32, Some(42), None),
        )
        .await
        .expect("post-speculative baseline generation streams");
        assert_eq!(
            ngram_baseline_after_spec, ngram_baseline,
            "non-speculative generation should remain deterministic after ngram speculation"
        );
        assert_eq!(
            runtime
                .last_speculative_stats(model_id)
                .expect("post-baseline speculative stats lookup succeeds"),
            Some(Default::default()),
            "non-speculative generation should reset speculative stats"
        );
        generation_count += 1;
        longest_generation =
            longest_generation.max(ngram_baseline_after_spec.generated_token_count());

        let score = runtime
            .score(model_id, (1_u32..=10).collect())
            .await
            .expect("score computes token log probabilities");
        assert_eq!(score.token_logprobs.len(), 9);
        assert!(
            score.token_logprobs.iter().all(|value| value.is_finite()),
            "token logprobs must be finite: {:?}",
            score.token_logprobs
        );
        assert!(
            score.token_logprobs.iter().all(|value| *value <= 0.0),
            "log probabilities must be non-positive: {:?}",
            score.token_logprobs
        );
        assert!(
            score.mean_logprob.is_finite() && score.mean_logprob <= 0.0,
            "mean log probability must be finite and non-positive: {:?}",
            score.mean_logprob
        );

        match runtime
            .embed(model_id, "llama.cpp e2e embedding smoke")
            .await
        {
            Ok(embedding) => {
                assert!(!embedding.vector.is_empty());
                assert!(embedding.vector.iter().all(|value| value.is_finite()));
            }
            Err(ModelRuntimeError::CapabilityNotSupported { capability, .. })
                if capability.contains("embedding") =>
            {
                eprintln!("SKIPPED llama_cpp_e2e_smoke embed substep: embedding context disabled");
            }
            Err(error) => panic!("unexpected embedding error: {error}"),
        }

        let events = wait_for_generation_events(&recorder, generation_count).await;
        assert_has_event_id(&events, FR_EVT_LLM_INFER_START);
        assert_has_event_id(&events, FR_EVT_LLM_INFER_END);
        if longest_generation >= 16 {
            assert_has_event_id(&events, FR_EVT_LLM_INFER_TOKEN);
        }
        if spec_stats.accepted_tokens > 0 {
            assert_has_event_type(&events, FlightRecorderEventType::LlmInferenceSpecAccept);
        }
        if spec_stats.rejected_tokens > 0 {
            assert_has_event_type(&events, FlightRecorderEventType::LlmInferenceSpecReject);
        }
        assert_events_do_not_leak_prompt_or_token_text(&events);

        runtime.unload(model_id).await.expect("unload model");
        assert!(
            runtime.capabilities(model_id).is_err(),
            "unloaded model must be absent from runtime model map"
        );
        assert!(
            runtime.perf_stats(model_id).is_err(),
            "unloaded model must have no perf stats handle"
        );
    }
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn llama_cpp_e2e_test_guard() -> std::sync::MutexGuard<'static, ()> {
    LLAMA_CPP_E2E_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn fixture_gguf_path() -> Option<PathBuf> {
    std::env::var_os("HANDSHAKE_TEST_GGUF_PATH").map(PathBuf::from)
}

fn optional_lora_path() -> Option<PathBuf> {
    std::env::var_os("HANDSHAKE_TEST_LORA_PATH").map(PathBuf::from)
}

fn lora_difference_assertion_enabled() -> bool {
    std::env::var_os("HANDSHAKE_TEST_LORA_EXPECT_DIFFERENT").is_some()
}

fn baseline_prompt() -> &'static str {
    "Handshake local model runtime smoke test:"
}

fn kv_prompt() -> &'static str {
    "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron pi rho sigma tau upsilon phi chi psi omega continuation"
}

const NGRAM_PROMPT_CANDIDATES: &[&str] = &[
    " alpha beta gamma delta alpha beta gamma delta alpha beta gamma delta",
    " red blue green yellow red blue green yellow red blue green yellow",
    " one two three four one two three four one two three four",
    " test test test test test test test test test test test test",
    " repeat repeat repeat repeat repeat repeat repeat repeat repeat repeat repeat repeat",
];

fn generate_request(
    id: handshake_core::model_runtime::ModelId,
    prompt: &str,
    max_tokens: u32,
    seed: Option<u32>,
    speculative_mode: Option<SpeculativeMode>,
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

fn load_spec(artifact_path: &Path, sha256_expected: String, base_tag: &str) -> LoadSpec {
    LoadSpec {
        artifact_path: artifact_path.to_path_buf(),
        sha256_expected,
        runtime_kind: RuntimeKind::LlamaCpp,
        sampling_defaults: SamplingParams::default(),
        kv_cache_policy: KvCachePolicy::Default {
            quant: KvQuantSupport::None,
            prefix_cache_ttl_seconds: 300,
            max_bytes: None,
        },
        declared_capabilities: expected_capabilities(),
        provider: ProviderKind::Local,
        engine_origin: Some(base_tag.to_string()),
        external_engine_import: None,
    }
}

fn expected_capabilities() -> ModelCapabilities {
    ModelCapabilities {
        supports_lora: true,
        supports_kv_prefix_cache: true,
        supports_kv_quantization: KvQuantSupport::Q4Q8Mix,
        supports_activation_steering: false,
        supports_subquadratic: false,
        supports_speculative_draft: true,
        supports_eagle3: false,
    }
}

fn lora_descriptor(path: &Path, base_tag: &str, id: LoraId) -> LoraDescriptor {
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct GenerationTrace {
    token_ids: Vec<u32>,
    text: String,
    finish: Option<FinishReason>,
}

impl GenerationTrace {
    fn generated_token_count(&self) -> usize {
        self.token_ids.len()
    }
}

fn assert_nonempty_generation(label: &str, trace: &GenerationTrace) {
    assert!(
        !trace.token_ids.is_empty(),
        "{label} generation should emit at least one token"
    );
    assert!(
        matches!(
            trace.finish,
            Some(FinishReason::Length) | Some(FinishReason::Stop)
        ),
        "{label} generation should finish with length or stop, got {:?}",
        trace.finish
    );
}

fn select_ngram_prompt(
    runtime: &LlamaCppRuntime,
    model_id: handshake_core::model_runtime::ModelId,
) -> Option<&'static str> {
    NGRAM_PROMPT_CANDIDATES.iter().copied().find(|candidate| {
        runtime
            .tokenize_prompt(model_id, candidate)
            .map(|tokens| has_repeated_suffix_ngram_with_draft(&tokens, 4))
            .unwrap_or(false)
    })
}

fn has_repeated_suffix_ngram_with_draft(tokens: &[u32], lookback: usize) -> bool {
    if tokens.len() <= lookback {
        return false;
    }

    let key_start = tokens.len() - lookback;
    let key = &tokens[key_start..];
    (0..key_start).rev().any(|candidate_start| {
        let candidate_end = candidate_start + lookback;
        &tokens[candidate_start..candidate_end] == key && candidate_end < tokens.len()
    })
}

#[derive(Default)]
struct E2eEventRecorder {
    events: Mutex<Vec<FlightRecorderEvent>>,
}

#[async_trait]
impl FlightRecorder for E2eEventRecorder {
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

async fn wait_for_generation_events(
    recorder: &E2eEventRecorder,
    expected_end_events: usize,
) -> Vec<FlightRecorderEvent> {
    for _ in 0..40 {
        let events = recorder
            .list_events(EventFilter::default())
            .await
            .expect("list e2e events");
        let end_count = events
            .iter()
            .filter(|event| event.payload["event_id"] == FR_EVT_LLM_INFER_END)
            .count();
        if end_count >= expected_end_events {
            return events;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    recorder
        .list_events(EventFilter::default())
        .await
        .expect("list e2e events after wait")
}

fn assert_has_event_id(events: &[FlightRecorderEvent], event_id: &str) {
    assert!(
        events
            .iter()
            .any(|event| event.payload["event_id"] == event_id),
        "missing flight recorder event family {event_id}; got {:?}",
        event_ids(events)
    );
}

fn assert_has_event_type(events: &[FlightRecorderEvent], event_type: FlightRecorderEventType) {
    assert!(
        events.iter().any(|event| event.event_type == event_type),
        "missing flight recorder event type {event_type:?}; got {:?}",
        events
            .iter()
            .map(|event| event.event_type.clone())
            .collect::<Vec<_>>()
    );
}

fn assert_events_do_not_leak_prompt_or_token_text(events: &[FlightRecorderEvent]) {
    for event in events {
        let payload = event.payload.to_string();
        for forbidden in [
            baseline_prompt(),
            kv_prompt(),
            "llama.cpp e2e embedding smoke",
        ] {
            assert!(
                !payload.contains(forbidden),
                "flight recorder payload leaked prompt/token text: {payload}"
            );
        }
        for forbidden in NGRAM_PROMPT_CANDIDATES {
            assert!(
                !payload.contains(forbidden),
                "flight recorder payload leaked ngram prompt text: {payload}"
            );
        }
    }
}

fn event_ids(events: &[FlightRecorderEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| {
            event
                .payload
                .get("event_id")
                .and_then(|value| value.as_str())
                .map(ToString::to_string)
        })
        .collect()
}

fn assert_tests_readme_registry_entry() {
    let registry: serde_json::Value =
        serde_json::from_str(TESTS_README_JSON).expect("tests/README.json parses");
    let tests = registry["tests"]
        .as_array()
        .expect("tests/README.json has a tests array");
    let entry = tests
        .iter()
        .find(|entry| entry["test_id"] == "llama_cpp_e2e_smoke")
        .expect("tests/README.json lists llama_cpp_e2e_smoke");

    assert_eq!(entry["test_file"], "tests/llama_cpp_e2e_smoke.rs");
    assert!(
        entry["run_command"]
            .as_str()
            .expect("llama_cpp_e2e_smoke run_command is a string")
            .contains("--features llama-cpp-runtime-engine"),
        "README run command must enable the native llama.cpp feature"
    );
    assert_json_array_contains(entry, "required_env", "HANDSHAKE_TEST_GGUF_PATH");
    assert_json_array_contains(entry, "optional_env", "HANDSHAKE_TEST_LORA_PATH");
    assert_json_array_contains(
        entry,
        "coverage",
        "kv_quantization_prefix_commit_restore_replay",
    );
    assert_json_array_contains(
        entry,
        "coverage",
        "flight_recorder_generation_and_spec_events",
    );
    assert!(
        entry["skip_policy"]
            .as_str()
            .expect("llama_cpp_e2e_smoke skip_policy is a string")
            .contains("SKIPPED"),
        "README skip policy must document structured skip output"
    );
    assert!(
        entry["artifact_policy"]
            .as_str()
            .expect("llama_cpp_e2e_smoke artifact_policy is a string")
            .contains("../Handshake_Artifacts/handshake-cargo-target"),
        "README artifact policy must keep build output under Handshake_Artifacts"
    );
}

fn assert_json_array_contains(entry: &serde_json::Value, key: &str, expected: &str) {
    let values = entry[key]
        .as_array()
        .unwrap_or_else(|| panic!("llama_cpp_e2e_smoke {key} must be an array"));
    assert!(
        values.iter().any(|value| value == expected),
        "llama_cpp_e2e_smoke README {key} missing {expected}: {values:?}"
    );
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

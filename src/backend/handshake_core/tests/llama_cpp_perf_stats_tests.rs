#[cfg(feature = "llama-cpp-runtime-engine")]
use std::{
    fs,
    path::Path,
    sync::{Arc, Mutex},
};

use chrono::{TimeZone, Utc};
#[cfg(feature = "llama-cpp-runtime-engine")]
use futures::StreamExt;
use handshake_core::{
    flight_recorder::{
        events_llm_infer::{
            infer_end_event, infer_start_event, infer_token_event, new_llm_infer_request_id,
            should_emit_token_event, FR_EVT_LLM_INFER_END, FR_EVT_LLM_INFER_START,
            FR_EVT_LLM_INFER_TOKEN,
        },
        fr_event_registry::{FrEventId, FrEventRegistry},
        FlightRecorderEventType,
    },
    model_runtime::{
        llama_cpp::{LlamaCppPerfStats, LlamaCppPerfStatsUpdate, LLAMA_CPP_PERF_STATS_EMA_ALPHA},
        FinishReason, ModelId,
    },
};

#[cfg(feature = "llama-cpp-runtime-engine")]
use handshake_core::{
    flight_recorder::{EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError},
    model_runtime::{
        llama_cpp::LlamaCppRuntime, CancellationToken, GenPrompt, GenerateRequest, KvCachePolicy,
        KvQuantSupport, LoadSpec, ModelCapabilities, ModelRuntime, ProviderKind, RuntimeKind,
        SamplingParams,
    },
};

#[cfg(feature = "llama-cpp-runtime-engine")]
use sha2::{Digest, Sha256};

#[test]
fn perf_stats_ema_and_time_since_last_call_are_deterministic() {
    let mut stats = LlamaCppPerfStats::default();
    let first_completed = Utc.with_ymd_and_hms(2026, 5, 22, 5, 0, 0).unwrap();
    let second_completed = Utc.with_ymd_and_hms(2026, 5, 22, 5, 0, 2).unwrap();

    stats.record_call(LlamaCppPerfStatsUpdate {
        prompt_eval_ms: 40,
        gen_eval_ms: 100,
        tokens_generated: 20,
        vram_resident_bytes: 0,
        completed_at_utc: first_completed,
    });

    assert_eq!(stats.total_prompts, 1);
    assert_eq!(stats.total_tokens_generated, 20);
    assert_eq!(stats.prompt_eval_ms_total, 40);
    assert_eq!(stats.gen_eval_ms_total, 100);
    assert_eq!(stats.vram_resident_bytes, 0);
    assert_eq!(stats.time_since_last_call_ms, None);
    assert_close(stats.tokens_per_sec_ema, 200.0);

    stats.record_call(LlamaCppPerfStatsUpdate {
        prompt_eval_ms: 60,
        gen_eval_ms: 200,
        tokens_generated: 20,
        vram_resident_bytes: 0,
        completed_at_utc: second_completed,
    });

    let expected_ema = 200.0_f32.mul_add(
        1.0 - LLAMA_CPP_PERF_STATS_EMA_ALPHA,
        100.0 * LLAMA_CPP_PERF_STATS_EMA_ALPHA,
    );
    assert_eq!(stats.total_prompts, 2);
    assert_eq!(stats.total_tokens_generated, 40);
    assert_eq!(stats.prompt_eval_ms_total, 100);
    assert_eq!(stats.gen_eval_ms_total, 300);
    assert_eq!(stats.time_since_last_call_ms, Some(2_000));
    assert_close(stats.tokens_per_sec_ema, expected_ema);
}

#[test]
fn llm_infer_events_validate_use_v7_request_ids_and_sample_tokens() {
    let model_id = ModelId::new_v7();
    let request_id = new_llm_infer_request_id();
    assert_eq!(request_id.get_version_num(), 7);

    assert!(!should_emit_token_event(1));
    assert!(!should_emit_token_event(15));
    assert!(should_emit_token_event(16));
    assert!(should_emit_token_event(32));

    let start = infer_start_event(model_id, request_id, 7, "hello world", "llama_cpp");
    let token = infer_token_event(model_id, request_id, 16, 42, "x", 25, "llama_cpp");
    let end = infer_end_event(
        model_id,
        request_id,
        7,
        32,
        450,
        125,
        300,
        FinishReason::Length,
        "llama_cpp",
    );

    let model_id_text = model_id.to_string();
    for event in [&start, &token, &end] {
        event.validate().expect("llm inference event validates");
        assert_eq!(event.event_type, FlightRecorderEventType::LlmInference);
        assert_eq!(event.model_id.as_deref(), Some(model_id_text.as_str()));
        assert_eq!(event.payload["type"], "llm_inference");
        assert_eq!(event.payload["request_id"], request_id.to_string());
        assert_eq!(event.payload["trace_id"], request_id.to_string());
        assert_eq!(
            event.payload["model_call_correlation_id"],
            request_id.to_string()
        );
        assert_eq!(event.payload["adapter"], "llama_cpp");
    }

    assert_eq!(start.payload["event_id"], FR_EVT_LLM_INFER_START);
    assert_eq!(start.payload["phase"], "start");
    assert_eq!(start.payload["tokens_in_prompt"], 7);
    assert_eq!(start.payload["ordered_index"], 0);

    assert_eq!(token.payload["event_id"], FR_EVT_LLM_INFER_TOKEN);
    assert_eq!(token.payload["phase"], "token");
    assert_eq!(token.payload["token_index"], 16);
    assert_eq!(token.payload["latency_ms"], 25);
    assert_eq!(token.payload["ordered_index"], 16);

    assert_eq!(end.payload["event_id"], FR_EVT_LLM_INFER_END);
    assert_eq!(end.payload["phase"], "end");
    assert_eq!(end.payload["tokens_generated"], 32);
    assert_eq!(end.payload["finish_reason"], "length");
    assert_eq!(end.payload["token_usage"]["total_tokens"], 39);
}

#[test]
fn fr_event_registry_contains_llm_infer_core_event_family() {
    for (id, expected) in [
        (FrEventId::LlmInferStart, FR_EVT_LLM_INFER_START),
        (FrEventId::LlmInferToken, FR_EVT_LLM_INFER_TOKEN),
        (FrEventId::LlmInferEnd, FR_EVT_LLM_INFER_END),
    ] {
        assert_eq!(id.as_str(), expected);
        assert_eq!(FrEventId::from_str_id(expected).unwrap(), id);
    }

    let registry = FrEventRegistry::from_rust_enum();
    for expected in [
        FR_EVT_LLM_INFER_START,
        FR_EVT_LLM_INFER_TOKEN,
        FR_EVT_LLM_INFER_END,
    ] {
        assert!(
            registry.events.iter().any(|entry| entry.id == expected),
            "registry missing {expected}"
        );
    }
}

#[cfg(feature = "llama-cpp-runtime-engine")]
#[tokio::test]
async fn env_gated_generate_updates_perf_stats_and_emits_sampled_events() {
    let Some(path) = std::env::var_os("HANDSHAKE_TEST_GGUF_PATH").map(std::path::PathBuf::from)
    else {
        return;
    };
    let recorder = Arc::new(PerfEventRecorder::default());
    let flight_recorder: Arc<dyn FlightRecorder> = recorder.clone();
    let mut runtime =
        LlamaCppRuntime::with_flight_recorder(KvCachePolicy::default(), flight_recorder);
    let model_id = runtime
        .load(load_spec(&path, sha256_file(&path)))
        .await
        .expect("native-enabled build loads representative GGUF");

    let generated = collect_generation(
        &runtime,
        GenerateRequest {
            id: model_id,
            prompt: GenPrompt::from("alpha beta alpha beta"),
            sampling: SamplingParams {
                seed: Some(43),
                ..SamplingParams::default()
            },
            lora_overrides: Vec::new(),
            steering_overrides: Vec::new(),
            kv_prefix_handle: None,
            cancel: CancellationToken::new(),
            max_tokens: 40,
            stop_sequences: Vec::new(),
            speculative_mode: None,
            structured_decoding: None,
        },
    )
    .await
    .expect("generation streams");

    let stats = runtime
        .perf_stats(model_id)
        .expect("loaded model exposes perf stats");
    assert_eq!(stats.total_prompts, 1);
    assert!(stats.prompt_eval_ms_total > 0);
    assert!(stats.gen_eval_ms_total > 0);
    assert!(stats.total_tokens_generated >= generated.len() as u64);
    assert!(stats.tokens_per_sec_ema.is_finite());
    assert!(stats.tokens_per_sec_ema > 0.0);
    assert_eq!(stats.vram_resident_bytes, 0);
    assert!(stats.last_call_at_utc.is_some());

    let events = wait_for_perf_events(&recorder).await;
    let starts: Vec<_> = events
        .iter()
        .filter(|event| event.payload["event_id"] == FR_EVT_LLM_INFER_START)
        .collect();
    let tokens: Vec<_> = events
        .iter()
        .filter(|event| event.payload["event_id"] == FR_EVT_LLM_INFER_TOKEN)
        .collect();
    let ends: Vec<_> = events
        .iter()
        .filter(|event| event.payload["event_id"] == FR_EVT_LLM_INFER_END)
        .collect();

    assert_eq!(starts.len(), 1, "expected exactly one START event");
    assert_eq!(ends.len(), 1, "expected exactly one END event");
    if generated.len() >= 16 {
        assert!(!tokens.is_empty(), "expected sampled TOKEN events");
    }
    assert!(
        tokens.len() <= generated.len().div_ceil(16),
        "TOKEN events must be sampled every 16th token, not emitted per token"
    );
}

fn assert_close(left: f32, right: f32) {
    let delta = (left - right).abs();
    assert!(
        delta < 0.001,
        "expected {left} to be within 0.001 of {right}; delta={delta}"
    );
}

#[cfg(feature = "llama-cpp-runtime-engine")]
async fn collect_generation(
    runtime: &LlamaCppRuntime,
    req: GenerateRequest,
) -> Result<Vec<u32>, handshake_core::model_runtime::ModelRuntimeError> {
    let mut stream = runtime.generate(req);
    let mut token_ids = Vec::new();
    while let Some(item) = stream.next().await {
        let token = item?;
        if token.finish_reason.is_none() {
            token_ids.push(token.token_id);
        } else {
            break;
        }
    }
    Ok(token_ids)
}

#[cfg(feature = "llama-cpp-runtime-engine")]
#[derive(Default)]
struct PerfEventRecorder {
    events: Mutex<Vec<FlightRecorderEvent>>,
}

#[cfg(feature = "llama-cpp-runtime-engine")]
#[async_trait::async_trait]
impl FlightRecorder for PerfEventRecorder {
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
async fn wait_for_perf_events(recorder: &PerfEventRecorder) -> Vec<FlightRecorderEvent> {
    for _ in 0..20 {
        let events = recorder
            .list_events(EventFilter::default())
            .await
            .expect("list perf events");
        let perf_events: Vec<_> = events
            .into_iter()
            .filter(|event| {
                event.event_type == FlightRecorderEventType::LlmInference && {
                    let event_id = event.payload["event_id"].as_str();
                    event_id == Some(FR_EVT_LLM_INFER_START)
                        || event_id == Some(FR_EVT_LLM_INFER_TOKEN)
                        || event_id == Some(FR_EVT_LLM_INFER_END)
                }
            })
            .collect();
        if perf_events
            .iter()
            .any(|event| event.payload["event_id"] == FR_EVT_LLM_INFER_END)
        {
            return perf_events;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    Vec::new()
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
            supports_eagle3: false,
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

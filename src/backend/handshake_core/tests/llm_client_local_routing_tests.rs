use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use chrono::Utc;
use futures::stream;
use handshake_core::{
    flight_recorder::{
        EventFilter, FlightRecorder, FlightRecorderActor, FlightRecorderEvent,
        FlightRecorderEventType, RecorderError,
    },
    llm::{
        local_router::{LocalModelRuntimeLlmClient, LocalRouter},
        CompletionRequest, CompletionResponse, DisabledLlmClient, EmbeddingRequest, LlmClient,
        LlmError, ModelProfile, TokenUsage,
    },
    model_runtime::{
        BaseModelTag, CancellationToken, Embedding, FinishReason, GenPrompt, GenerateRequest,
        GeneratedToken, KvCacheHandle, LoraStackHandle, ModelCapabilities, ModelId,
        ModelRegistration, ModelRegistry, ModelRuntime, ModelRuntimeError, OperatorId,
        ProviderKind, RuntimeBinding, Score, SteeringHookHandle, TokenStream,
    },
};
use tokio::{sync::Notify, time::Duration};

#[derive(Clone)]
struct RecordingRuntime {
    label: &'static str,
    tokens: Vec<GeneratedToken>,
    capabilities: ModelCapabilities,
    requests: Arc<Mutex<Vec<GenerateRequest>>>,
    cancelled: Arc<Mutex<Vec<CancellationToken>>>,
    release_after_tokens: Option<Arc<Notify>>,
}

impl RecordingRuntime {
    fn new(label: &'static str, chunks: &[&str]) -> Self {
        let tokens = chunks
            .iter()
            .enumerate()
            .map(|(index, text)| GeneratedToken {
                token_id: index as u32,
                text: (*text).to_string(),
                logprob: None,
                finish_reason: (index + 1 == chunks.len()).then_some(FinishReason::Stop),
            })
            .collect();
        Self {
            label,
            tokens,
            capabilities: ModelCapabilities::default(),
            requests: Arc::new(Mutex::new(Vec::new())),
            cancelled: Arc::new(Mutex::new(Vec::new())),
            release_after_tokens: None,
        }
    }

    fn new_blocking(
        label: &'static str,
        chunks: &[&str],
        release_after_tokens: Arc<Notify>,
    ) -> Self {
        let mut runtime = Self::new(label, chunks);
        runtime.release_after_tokens = Some(release_after_tokens);
        runtime
    }

    fn request_count(&self) -> usize {
        self.requests.lock().expect("requests lock").len()
    }

    fn last_request(&self) -> GenerateRequest {
        self.requests
            .lock()
            .expect("requests lock")
            .last()
            .cloned()
            .unwrap_or_else(|| panic!("expected {} runtime request", self.label))
    }

    fn cancel_count(&self) -> usize {
        self.cancelled.lock().expect("cancel lock").len()
    }
}

#[async_trait]
impl ModelRuntime for RecordingRuntime {
    async fn load(
        &mut self,
        _spec: handshake_core::model_runtime::LoadSpec,
    ) -> Result<ModelId, ModelRuntimeError> {
        Ok(ModelId::new_v7())
    }

    async fn unload(&mut self, _id: ModelId) -> Result<(), ModelRuntimeError> {
        Ok(())
    }

    fn generate(&self, req: GenerateRequest) -> TokenStream {
        self.requests.lock().expect("requests lock").push(req);
        let tokens = self.tokens.clone();
        let Some(release_after_tokens) = self.release_after_tokens.clone() else {
            return Box::pin(stream::iter(tokens.into_iter().map(Ok)));
        };

        Box::pin(stream::unfold(
            (0_usize, tokens, release_after_tokens),
            |(index, tokens, release_after_tokens)| async move {
                if index < tokens.len() {
                    return Some((
                        Ok(tokens[index].clone()),
                        (index + 1, tokens, release_after_tokens),
                    ));
                }
                release_after_tokens.notified().await;
                None
            },
        ))
    }

    async fn score(&self, _id: ModelId, _sequence: Vec<u32>) -> Result<Score, ModelRuntimeError> {
        Ok(Score {
            token_logprobs: Vec::new(),
            mean_logprob: 0.0,
        })
    }

    async fn embed(&self, _id: ModelId, _text: &str) -> Result<Embedding, ModelRuntimeError> {
        Ok(Embedding { vector: Vec::new() })
    }

    fn capabilities(&self, _id: ModelId) -> Result<&ModelCapabilities, ModelRuntimeError> {
        Ok(&self.capabilities)
    }

    fn kv_cache(&self, _id: ModelId) -> Result<KvCacheHandle, ModelRuntimeError> {
        Ok(KvCacheHandle::new(format!("{}-kv", self.label)))
    }

    fn lora_stack(&self, _id: ModelId) -> Result<LoraStackHandle, ModelRuntimeError> {
        Ok(LoraStackHandle::new(format!("{}-lora", self.label)))
    }

    fn steering_hooks(&self, _id: ModelId) -> Result<SteeringHookHandle, ModelRuntimeError> {
        Ok(SteeringHookHandle::new(format!("{}-steering", self.label)))
    }

    fn cancel(&self, token: CancellationToken) {
        token.cancel();
        self.cancelled.lock().expect("cancel lock").push(token);
    }
}

#[derive(Clone)]
struct RecordingFallbackClient {
    response: String,
    profile: ModelProfile,
    requests: Arc<Mutex<Vec<CompletionRequest>>>,
    cancelled_model_ids: Arc<Mutex<Vec<String>>>,
}

impl RecordingFallbackClient {
    fn new(response: &str) -> Self {
        Self {
            response: response.to_string(),
            profile: ModelProfile::new("fallback".to_string(), 4096),
            requests: Arc::new(Mutex::new(Vec::new())),
            cancelled_model_ids: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn request_count(&self) -> usize {
        self.requests.lock().expect("requests lock").len()
    }

    fn cancel_count(&self) -> usize {
        self.cancelled_model_ids.lock().expect("cancel lock").len()
    }
}

#[async_trait]
impl LlmClient for RecordingFallbackClient {
    async fn completion(&self, req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        self.requests.lock().expect("requests lock").push(req);
        Ok(CompletionResponse {
            text: self.response.clone(),
            usage: TokenUsage {
                prompt_tokens: 1,
                completion_tokens: 2,
                total_tokens: 3,
            },
            latency_ms: 0,
        })
    }

    fn cancel(&self, model_id: &str, token: CancellationToken) {
        token.cancel();
        self.cancelled_model_ids
            .lock()
            .expect("cancel lock")
            .push(model_id.to_string());
    }

    fn profile(&self) -> &ModelProfile {
        &self.profile
    }
}

#[derive(Clone, Default)]
struct CapturingRecorder {
    events: Arc<Mutex<Vec<FlightRecorderEvent>>>,
}

impl CapturingRecorder {
    fn events(&self) -> Vec<FlightRecorderEvent> {
        self.events.lock().expect("events lock").clone()
    }
}

#[async_trait]
impl FlightRecorder for CapturingRecorder {
    async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
        self.events.lock().expect("events lock").push(event);
        Ok(())
    }

    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        Ok(0)
    }

    async fn list_events(
        &self,
        _filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Ok(self.events())
    }
}

fn capabilities(activation_steering: bool) -> ModelCapabilities {
    ModelCapabilities {
        supports_lora: true,
        supports_kv_prefix_cache: true,
        supports_activation_steering: activation_steering,
        supports_speculative_draft: false,
        supports_eagle3: false,
        ..Default::default()
    }
}

fn registration(model_id: ModelId, binding: RuntimeBinding) -> ModelRegistration {
    ModelRegistration {
        model_id,
        artifact_path: PathBuf::from("fixtures/models/local-routing.gguf"),
        sha256: [9; 32],
        runtime_binding: binding,
        declared_capabilities: capabilities(binding == RuntimeBinding::Candle),
        base_model_tag: BaseModelTag::new("local-routing-base"),
        registered_at_utc: Utc::now(),
        registered_by: OperatorId::new("operator-ilja"),
        provider: ProviderKind::Local,
    }
}

fn client_for_registry(
    registry: ModelRegistry,
    llama: Arc<RecordingRuntime>,
    candle: Arc<RecordingRuntime>,
    fallback: Arc<RecordingFallbackClient>,
    recorder: Arc<CapturingRecorder>,
) -> LocalModelRuntimeLlmClient {
    let router = LocalRouter::new(Arc::new(registry), llama, candle);
    LocalModelRuntimeLlmClient::new(
        router,
        fallback,
        recorder,
        ModelProfile::new("local-router".to_string(), 8192).with_streaming(true),
    )
}

async fn wait_for_runtime_request(runtime: &RecordingRuntime) -> GenerateRequest {
    for _ in 0..100 {
        if runtime.request_count() > 0 {
            return runtime.last_request();
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("timed out waiting for runtime request");
}

#[tokio::test]
async fn local_llamacpp_model_completion_routes_through_model_runtime_and_emits_fr_event() {
    let model_id = ModelId::new_v7();
    let mut registry = ModelRegistry::default();
    registry
        .register(registration(model_id, RuntimeBinding::LlamaCpp))
        .expect("register llama model");
    let llama = Arc::new(RecordingRuntime::new("llama", &["llama ", "ok"]));
    let candle = Arc::new(RecordingRuntime::new("candle", &["wrong"]));
    let fallback = Arc::new(RecordingFallbackClient::new("fallback"));
    let recorder = Arc::new(CapturingRecorder::default());
    let client = client_for_registry(
        registry,
        llama.clone(),
        candle.clone(),
        fallback.clone(),
        recorder.clone(),
    );

    let trace_id = uuid::Uuid::now_v7();
    let req = CompletionRequest::new(
        trace_id,
        "route this locally".to_string(),
        model_id.to_string(),
    )
    .with_max_tokens(8)
    .with_temperature(0.2)
    .with_stop_sequences(vec!["</s>".to_string()]);

    let response = client.completion(req).await.expect("local completion");

    assert_eq!(response.text, "llama ok");
    assert_eq!(response.usage.prompt_tokens, 3);
    assert_eq!(response.usage.completion_tokens, 2);
    assert_eq!(response.usage.total_tokens, 5);
    assert_eq!(llama.request_count(), 1);
    assert_eq!(candle.request_count(), 0);
    assert_eq!(fallback.request_count(), 0);
    let routed_req = llama.last_request();
    assert_eq!(routed_req.id, model_id);
    assert_eq!(routed_req.prompt, GenPrompt::from("route this locally"));
    assert_eq!(routed_req.max_tokens, 8);
    assert_eq!(routed_req.sampling.temperature, Some(0.2));
    assert_eq!(routed_req.stop_sequences, vec!["</s>".to_string()]);

    let events = recorder.events();
    assert_eq!(events.len(), 1);
    assert!(matches!(
        events[0].event_type,
        FlightRecorderEventType::LlmInference
    ));
    assert_eq!(events[0].actor, FlightRecorderActor::Agent);
    assert_eq!(events[0].trace_id, trace_id);
    assert_eq!(
        events[0].model_id.as_deref(),
        Some(model_id.to_string().as_str())
    );
    assert_eq!(events[0].payload["type"], "llm_inference");
    assert_eq!(events[0].payload["model_id"], model_id.to_string());
    assert_eq!(events[0].payload["token_usage"]["prompt_tokens"], 3);
    assert_eq!(events[0].payload["token_usage"]["completion_tokens"], 2);
    assert_eq!(events[0].payload["token_usage"]["total_tokens"], 5);
    assert!(events[0].payload["latency_ms"].as_u64().unwrap_or(0) > 0);
    assert!(events[0].payload["prompt_hash"].is_string());
    assert!(events[0].payload["response_hash"].is_string());
    assert!(events[0].validate().is_ok());
}

#[tokio::test]
async fn local_candle_model_completion_routes_to_candle_runtime() {
    let model_id = ModelId::new_v7();
    let mut registry = ModelRegistry::default();
    registry
        .register(registration(model_id, RuntimeBinding::Candle))
        .expect("register candle model");
    let llama = Arc::new(RecordingRuntime::new("llama", &["wrong"]));
    let candle = Arc::new(RecordingRuntime::new("candle", &["candle"]));
    let fallback = Arc::new(RecordingFallbackClient::new("fallback"));
    let recorder = Arc::new(CapturingRecorder::default());
    let client = client_for_registry(
        registry,
        llama.clone(),
        candle.clone(),
        fallback.clone(),
        recorder,
    );

    let req = CompletionRequest::new(
        uuid::Uuid::now_v7(),
        "route to candle".to_string(),
        model_id.to_string(),
    );

    let response = client.completion(req).await.expect("candle completion");

    assert_eq!(response.text, "candle");
    assert_eq!(llama.request_count(), 0);
    assert_eq!(candle.request_count(), 1);
    assert_eq!(fallback.request_count(), 0);
}

#[tokio::test]
async fn non_uuid_provider_model_ids_stay_on_fallback_llm_client() {
    let registry = ModelRegistry::default();
    let llama = Arc::new(RecordingRuntime::new("llama", &["wrong"]));
    let candle = Arc::new(RecordingRuntime::new("candle", &["wrong"]));
    let fallback = Arc::new(RecordingFallbackClient::new("cloud response"));
    let recorder = Arc::new(CapturingRecorder::default());
    let client = client_for_registry(
        registry,
        llama.clone(),
        candle.clone(),
        fallback.clone(),
        recorder,
    );

    let req = CompletionRequest::new(
        uuid::Uuid::now_v7(),
        "cloud path".to_string(),
        "gpt-4o-mini".to_string(),
    );

    let response = client.completion(req).await.expect("fallback completion");

    assert_eq!(response.text, "cloud response");
    assert_eq!(fallback.request_count(), 1);
    assert_eq!(llama.request_count(), 0);
    assert_eq!(candle.request_count(), 0);
}

#[tokio::test]
async fn uuid_like_non_v7_model_ids_stay_on_fallback_llm_client() {
    let registry = ModelRegistry::default();
    let llama = Arc::new(RecordingRuntime::new("llama", &["wrong"]));
    let candle = Arc::new(RecordingRuntime::new("candle", &["wrong"]));
    let fallback = Arc::new(RecordingFallbackClient::new("uuid fallback response"));
    let recorder = Arc::new(CapturingRecorder::default());
    let client = client_for_registry(
        registry,
        llama.clone(),
        candle.clone(),
        fallback.clone(),
        recorder,
    );

    let uuid_like_model_id = uuid::Uuid::nil().to_string();
    let req = CompletionRequest::new(
        uuid::Uuid::now_v7(),
        "fallback uuid-shaped model id".to_string(),
        uuid_like_model_id.clone(),
    );

    let response = client.completion(req).await.expect("fallback completion");

    assert_eq!(response.text, "uuid fallback response");
    assert_eq!(fallback.request_count(), 1);
    assert_eq!(llama.request_count(), 0);
    assert_eq!(candle.request_count(), 0);

    let fallback_token = CancellationToken::new();
    client.cancel(&uuid_like_model_id, fallback_token.clone());
    assert!(fallback_token.is_cancelled());
    assert_eq!(fallback.cancel_count(), 1);
}

#[test]
fn cancel_uses_same_llm_client_surface_for_local_and_fallback_models() {
    let model_id = ModelId::new_v7();
    let mut registry = ModelRegistry::default();
    registry
        .register(registration(model_id, RuntimeBinding::LlamaCpp))
        .expect("register llama model");
    let llama = Arc::new(RecordingRuntime::new("llama", &["llama"]));
    let candle = Arc::new(RecordingRuntime::new("candle", &["candle"]));
    let fallback = Arc::new(RecordingFallbackClient::new("fallback"));
    let recorder = Arc::new(CapturingRecorder::default());
    let client = client_for_registry(registry, llama.clone(), candle, fallback.clone(), recorder);

    let local_token = CancellationToken::new();
    client.cancel(&model_id.to_string(), local_token.clone());
    assert!(local_token.is_cancelled());
    assert_eq!(llama.cancel_count(), 1);

    let fallback_token = CancellationToken::new();
    client.cancel("gpt-4o-mini", fallback_token.clone());
    assert!(fallback_token.is_cancelled());
    assert_eq!(fallback.cancel_count(), 1);
}

#[tokio::test]
async fn cancel_cancels_the_active_local_generate_request_token() {
    let model_id = ModelId::new_v7();
    let mut registry = ModelRegistry::default();
    registry
        .register(registration(model_id, RuntimeBinding::LlamaCpp))
        .expect("register llama model");
    let release = Arc::new(Notify::new());
    let llama = Arc::new(RecordingRuntime::new_blocking(
        "llama",
        &["partial"],
        release.clone(),
    ));
    let candle = Arc::new(RecordingRuntime::new("candle", &["candle"]));
    let fallback = Arc::new(RecordingFallbackClient::new("fallback"));
    let recorder = Arc::new(CapturingRecorder::default());
    let client = Arc::new(client_for_registry(
        registry,
        llama.clone(),
        candle,
        fallback,
        recorder,
    ));

    let req = CompletionRequest::new(
        uuid::Uuid::now_v7(),
        "cancel while active".to_string(),
        model_id.to_string(),
    );
    let completion_task = tokio::spawn({
        let client = Arc::clone(&client);
        async move { client.completion(req).await }
    });

    let routed_req = wait_for_runtime_request(&llama).await;
    assert!(!routed_req.cancel.is_cancelled());

    let caller_token = CancellationToken::new();
    client.cancel(&model_id.to_string(), caller_token.clone());

    assert!(caller_token.is_cancelled());
    assert!(
        llama.last_request().cancel.is_cancelled(),
        "LlmClient::cancel must cancel the token attached to the active GenerateRequest"
    );

    release.notify_waiters();
    let response = completion_task
        .await
        .expect("completion task joins")
        .expect("completion succeeds after release");
    assert_eq!(response.text, "partial");
}

// ===========================================================================
// WP-1 MT-013: Flight Recorder emission on EVERY LlmClient call path
// (spec §4.2.3.2(3)) — fail-closed/disabled, completion error branches, and the
// embedding lane (success + error). Emitted at CALL TIME, never at construction.
// ===========================================================================

/// A `ModelRuntime` whose behavior is configurable per test: it can succeed with
/// a fixed token stream, fail the generate stream, return a fixed embedding
/// vector, or fail the embed call.
struct ConfigurableRuntime {
    tokens: Vec<GeneratedToken>,
    fail_generate: bool,
    embed_vector: Vec<f32>,
    fail_embed: bool,
    capabilities: ModelCapabilities,
}

impl ConfigurableRuntime {
    fn generating(chunks: &[&str]) -> Self {
        let tokens = chunks
            .iter()
            .enumerate()
            .map(|(index, text)| GeneratedToken {
                token_id: index as u32,
                text: (*text).to_string(),
                logprob: None,
                finish_reason: (index + 1 == chunks.len()).then_some(FinishReason::Stop),
            })
            .collect();
        Self {
            tokens,
            fail_generate: false,
            embed_vector: Vec::new(),
            fail_embed: false,
            capabilities: ModelCapabilities::default(),
        }
    }

    fn stream_error() -> Self {
        Self {
            tokens: Vec::new(),
            fail_generate: true,
            embed_vector: Vec::new(),
            fail_embed: false,
            capabilities: ModelCapabilities::default(),
        }
    }

    fn embedding(vector: Vec<f32>) -> Self {
        Self {
            tokens: Vec::new(),
            fail_generate: false,
            embed_vector: vector,
            fail_embed: false,
            capabilities: ModelCapabilities::default(),
        }
    }

    fn embed_error() -> Self {
        Self {
            tokens: Vec::new(),
            fail_generate: false,
            embed_vector: Vec::new(),
            fail_embed: true,
            capabilities: ModelCapabilities::default(),
        }
    }
}

#[async_trait]
impl ModelRuntime for ConfigurableRuntime {
    async fn load(
        &mut self,
        _spec: handshake_core::model_runtime::LoadSpec,
    ) -> Result<ModelId, ModelRuntimeError> {
        Ok(ModelId::new_v7())
    }

    async fn unload(&mut self, _id: ModelId) -> Result<(), ModelRuntimeError> {
        Ok(())
    }

    fn generate(&self, _req: GenerateRequest) -> TokenStream {
        if self.fail_generate {
            let err: Result<GeneratedToken, ModelRuntimeError> =
                Err(ModelRuntimeError::GenerateError("stream boom".to_string()));
            return Box::pin(stream::iter(vec![err]));
        }
        let tokens = self.tokens.clone();
        Box::pin(stream::iter(tokens.into_iter().map(Ok)))
    }

    async fn score(&self, _id: ModelId, _sequence: Vec<u32>) -> Result<Score, ModelRuntimeError> {
        Ok(Score {
            token_logprobs: Vec::new(),
            mean_logprob: 0.0,
        })
    }

    async fn embed(&self, _id: ModelId, _text: &str) -> Result<Embedding, ModelRuntimeError> {
        if self.fail_embed {
            return Err(ModelRuntimeError::EmbedError("embed boom".to_string()));
        }
        Ok(Embedding {
            vector: self.embed_vector.clone(),
        })
    }

    fn capabilities(&self, _id: ModelId) -> Result<&ModelCapabilities, ModelRuntimeError> {
        Ok(&self.capabilities)
    }

    fn kv_cache(&self, _id: ModelId) -> Result<KvCacheHandle, ModelRuntimeError> {
        Ok(KvCacheHandle::new("cfg-kv"))
    }

    fn lora_stack(&self, _id: ModelId) -> Result<LoraStackHandle, ModelRuntimeError> {
        Ok(LoraStackHandle::new("cfg-lora"))
    }

    fn steering_hooks(&self, _id: ModelId) -> Result<SteeringHookHandle, ModelRuntimeError> {
        Ok(SteeringHookHandle::new("cfg-steering"))
    }

    fn cancel(&self, token: CancellationToken) {
        token.cancel();
    }
}

/// Builds a local client whose registered (Candle) model routes to `candle`.
fn candle_client(
    model_id: ModelId,
    candle: Arc<dyn ModelRuntime>,
    recorder: Arc<CapturingRecorder>,
    max_context_tokens: u32,
) -> LocalModelRuntimeLlmClient {
    let mut registry = ModelRegistry::default();
    registry
        .register(registration(model_id, RuntimeBinding::Candle))
        .expect("register candle model");
    let llama: Arc<dyn ModelRuntime> = Arc::new(RecordingRuntime::new("llama", &["x"]));
    let router = LocalRouter::new(Arc::new(registry), llama, candle);
    let fallback = Arc::new(RecordingFallbackClient::new("fallback"));
    LocalModelRuntimeLlmClient::new(
        router,
        fallback,
        recorder,
        ModelProfile::new("local-router".to_string(), max_context_tokens).with_streaming(true),
    )
}

#[tokio::test]
async fn disabled_client_completion_emits_fr_event_at_call_time_each_call() {
    let recorder = Arc::new(CapturingRecorder::default());
    let disabled = DisabledLlmClient::new_recorded(
        "unknown".to_string(),
        "HSK-LOCAL-DISABLED: test disabled".to_string(),
        recorder.clone(),
    );

    // Construction alone MUST NOT emit — proves call-time, not construction-time.
    assert_eq!(
        recorder.events().len(),
        0,
        "DisabledLlmClient construction must not emit a Flight Recorder event"
    );

    let trace_a = uuid::Uuid::now_v7();
    let err_a = disabled
        .completion(CompletionRequest::new(trace_a, "p".into(), "m".into()))
        .await
        .expect_err("disabled completion must fail closed");
    assert!(matches!(err_a, LlmError::ProviderError(_)));

    let trace_b = uuid::Uuid::now_v7();
    let _ = disabled
        .completion(CompletionRequest::new(trace_b, "p2".into(), "m2".into()))
        .await
        .expect_err("disabled completion must fail closed");

    let events = recorder.events();
    assert_eq!(
        events.len(),
        2,
        "two disabled calls must emit two FR events (call-time, not once at construction)"
    );
    for event in &events {
        assert!(matches!(
            event.event_type,
            FlightRecorderEventType::LlmInference
        ));
        assert_eq!(event.payload["error_kind"], "llm_disabled");
        assert_eq!(event.payload["token_usage"]["total_tokens"], 0);
        assert!(event.payload["reason"].is_string());
    }
    assert_eq!(events[0].trace_id, trace_a);
    assert_eq!(events[1].trace_id, trace_b);
}

#[tokio::test]
async fn local_completion_stream_error_emits_fr_error_event() {
    let model_id = ModelId::new_v7();
    let recorder = Arc::new(CapturingRecorder::default());
    let candle: Arc<dyn ModelRuntime> = Arc::new(ConfigurableRuntime::stream_error());
    let client = candle_client(model_id, candle, recorder.clone(), 8192);

    let trace_id = uuid::Uuid::now_v7();
    let err = client
        .completion(CompletionRequest::new(
            trace_id,
            "boom".into(),
            model_id.to_string(),
        ))
        .await
        .expect_err("stream error must surface as LlmError");
    assert!(matches!(err, LlmError::ProviderError(_)));

    let events = recorder.events();
    assert_eq!(
        events.len(),
        1,
        "a completion stream error must emit exactly one FR event"
    );
    assert!(matches!(
        events[0].event_type,
        FlightRecorderEventType::LlmInference
    ));
    assert_eq!(events[0].payload["error_kind"], "llm_error");
    assert_eq!(events[0].payload["token_usage"]["total_tokens"], 0);
    assert_eq!(events[0].trace_id, trace_id);
}

#[tokio::test]
async fn local_completion_budget_exceeded_emits_fr_error_event() {
    let model_id = ModelId::new_v7();
    let recorder = Arc::new(CapturingRecorder::default());
    let candle: Arc<dyn ModelRuntime> = Arc::new(ConfigurableRuntime::generating(&["a", "b", "c"]));
    let client = candle_client(model_id, candle, recorder.clone(), 8192);

    let req = CompletionRequest::new(
        uuid::Uuid::now_v7(),
        "over budget".into(),
        model_id.to_string(),
    )
    .with_max_tokens(1);
    let err = client
        .completion(req)
        .await
        .expect_err("budget exceeded must surface as LlmError");
    assert!(matches!(err, LlmError::BudgetExceeded(_)));

    let events = recorder.events();
    assert_eq!(
        events.len(),
        1,
        "budget-exceeded must emit the error FR event, not the success event"
    );
    assert!(matches!(
        events[0].event_type,
        FlightRecorderEventType::LlmInference
    ));
    assert_eq!(events[0].payload["error_kind"], "llm_error");
}

#[tokio::test]
async fn local_completion_unregistered_model_emits_fr_error_event() {
    let recorder = Arc::new(CapturingRecorder::default());
    // Empty registry: a UUIDv7 id parses as local but resolves to "not
    // registered" inside run_local_completion (the runtime-resolve error branch).
    let registry = ModelRegistry::default();
    let llama: Arc<dyn ModelRuntime> = Arc::new(RecordingRuntime::new("llama", &["x"]));
    let candle: Arc<dyn ModelRuntime> = Arc::new(RecordingRuntime::new("candle", &["x"]));
    let router = LocalRouter::new(Arc::new(registry), llama, candle);
    let fallback = Arc::new(RecordingFallbackClient::new("fallback"));
    let client = LocalModelRuntimeLlmClient::new(
        router,
        fallback,
        recorder.clone(),
        ModelProfile::new("local-router".to_string(), 8192),
    );

    let unregistered = ModelId::new_v7();
    let trace_id = uuid::Uuid::now_v7();
    let err = client
        .completion(CompletionRequest::new(
            trace_id,
            "p".into(),
            unregistered.to_string(),
        ))
        .await
        .expect_err("unregistered model must fail");
    assert!(matches!(err, LlmError::ProviderError(_)));

    let events = recorder.events();
    assert_eq!(
        events.len(),
        1,
        "runtime-resolve error must emit exactly one FR event"
    );
    assert_eq!(events[0].payload["error_kind"], "llm_error");
    assert_eq!(events[0].trace_id, trace_id);
}

#[tokio::test]
async fn local_embedding_success_emits_data_embedding_computed_event() {
    let model_id = ModelId::new_v7();
    let recorder = Arc::new(CapturingRecorder::default());
    let candle: Arc<dyn ModelRuntime> =
        Arc::new(ConfigurableRuntime::embedding(vec![0.1, 0.2, 0.3]));
    let client = candle_client(model_id, candle, recorder.clone(), 8192);

    let trace_id = uuid::Uuid::now_v7();
    let response = client
        .embedding(EmbeddingRequest::new(
            trace_id,
            "embed me".into(),
            model_id.to_string(),
        ))
        .await
        .expect("embedding success");
    assert_eq!(response.vector, vec![0.1, 0.2, 0.3]);

    let events = recorder.events();
    assert_eq!(
        events.len(),
        1,
        "embedding success must emit exactly one FR event"
    );
    assert!(
        matches!(
            events[0].event_type,
            FlightRecorderEventType::DataEmbeddingComputed
        ),
        "embedding success must reuse DataEmbeddingComputed, got {:?}",
        events[0].event_type
    );
    assert_eq!(events[0].trace_id, trace_id);
    assert_eq!(events[0].payload["embedding_dim"], 3);
    // Embeddings carry NO TokenUsage (product-extension FR shape).
    assert!(
        events[0].payload.get("token_usage").is_none(),
        "embedding FR event must not carry a token_usage field"
    );
}

#[tokio::test]
async fn local_embedding_error_emits_fr_error_event() {
    let model_id = ModelId::new_v7();
    let recorder = Arc::new(CapturingRecorder::default());
    let candle: Arc<dyn ModelRuntime> = Arc::new(ConfigurableRuntime::embed_error());
    let client = candle_client(model_id, candle, recorder.clone(), 8192);

    let trace_id = uuid::Uuid::now_v7();
    let err = client
        .embedding(EmbeddingRequest::new(
            trace_id,
            "embed me".into(),
            model_id.to_string(),
        ))
        .await
        .expect_err("embed failure must surface as LlmError");
    assert!(matches!(err, LlmError::ProviderError(_)));

    let events = recorder.events();
    assert_eq!(
        events.len(),
        1,
        "embedding error must emit exactly one FR event"
    );
    assert!(matches!(
        events[0].event_type,
        FlightRecorderEventType::LlmInference
    ));
    assert_eq!(events[0].payload["error_kind"], "embedding_error");
    assert_eq!(events[0].trace_id, trace_id);
}

#[test]
fn llm_routing_surface_stays_engine_agnostic() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for relative in [["src", "llm", "mod.rs"], ["src", "llm", "local_router.rs"]] {
        let path = relative
            .iter()
            .fold(manifest_dir.clone(), |acc, item| acc.join(item));
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
        let normalized = source.to_ascii_lowercase();

        for banned in ["llama_cpp_2::", "candle_core::", "candle_transformers::"] {
            assert!(
                !normalized.contains(banned),
                "LlmClient local routing surface must not leak engine-specific type `{banned}` in {}",
                path.display()
            );
        }
    }
}

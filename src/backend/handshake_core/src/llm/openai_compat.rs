//! OpenAI-compatible HTTP adapter.
//!
//! This module intentionally keeps the surface minimal:
//! - Completes `LlmClient::completion(...)` via an OpenAI-compatible endpoint.
//! - Emits Flight Recorder `llm_inference` without raw prompts/payloads.
//! - Does not log or persist API keys.

use super::{CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile, ModelTier};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use std::sync::Arc;
// WAIVER [CX-573E]: Instant::now() is required for latency measurement per §4.2.3.2.
// This is non-deterministic but necessary for observability metrics.
use std::time::Instant;

use crate::flight_recorder::{
    FlightRecorder, FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
    LlmInferenceEvent, LlmInferenceTokenUsage, RecorderError,
};

#[derive(Clone)]
pub struct ApiKey(String);

impl ApiKey {
    pub fn from_env(var_name: &str) -> Option<Self> {
        std::env::var(var_name)
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .map(Self)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for ApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<redacted>")
    }
}

impl fmt::Display for ApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<redacted>")
    }
}

pub struct OpenAiCompatAdapter {
    base_url: String,
    profile: ModelProfile,
    client: reqwest::Client,
    flight_recorder: Arc<dyn FlightRecorder>,
    api_key: Option<ApiKey>,
}

impl OpenAiCompatAdapter {
    pub fn new(
        base_url: String,
        model_id: String,
        max_context_tokens: u32,
        tier: ModelTier,
        api_key: Option<ApiKey>,
        flight_recorder: Arc<dyn FlightRecorder>,
    ) -> Self {
        let base_url = base_url.trim_end_matches('/').to_string();
        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
        {
            Ok(client) => client,
            Err(_) => reqwest::Client::new(),
        };
        Self {
            base_url,
            profile: ModelProfile::new(model_id, max_context_tokens).with_tier(tier),
            client,
            flight_recorder,
            api_key,
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    fn chat_completions_url(&self) -> String {
        format!("{}/v1/chat/completions", self.base_url)
    }

    fn compute_hash(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        hex::encode(hasher.finalize())
    }

    async fn emit_llm_inference_event(
        &self,
        req: &CompletionRequest,
        response_text: &str,
        usage: &super::TokenUsage,
        latency_ms: u64,
    ) {
        let payload = LlmInferenceEvent {
            event_type: "llm_inference".to_string(),
            trace_id: req.trace_id,
            model_id: req.model_id.clone(),
            token_usage: LlmInferenceTokenUsage {
                prompt_tokens: usage.prompt_tokens as u64,
                completion_tokens: usage.completion_tokens as u64,
                total_tokens: usage.total_tokens as u64,
            },
            prompt_hash: Some(Self::compute_hash(&req.prompt)),
            response_hash: Some(Self::compute_hash(response_text)),
            latency_ms: Some(latency_ms),
        };

        let payload_value = match serde_json::to_value(&payload) {
            Ok(value) => value,
            Err(_) => serde_json::Value::Null,
        };

        let event = FlightRecorderEvent::new(
            FlightRecorderEventType::LlmInference,
            FlightRecorderActor::Agent,
            req.trace_id,
            payload_value,
        )
        .with_model_id(&req.model_id);

        let record_result: Result<(), RecorderError> = self.flight_recorder.record_event(event).await;
        if let Err(err) = record_result {
            tracing::warn!(
                target: "handshake_core::llm",
                error = %err,
                trace_id = %req.trace_id,
                "Failed to record llm_inference event"
            );
        }
    }
}

#[derive(Debug, Serialize)]
struct OpenAiCompatChatCompletionRequest {
    model: String,
    messages: Vec<OpenAiCompatChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    temperature: f32,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    stop: Vec<String>,
    stream: bool,
}

#[derive(Debug, Serialize)]
struct OpenAiCompatChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiCompatChatCompletionResponse {
    choices: Vec<OpenAiCompatChatChoice>,
    #[serde(default)]
    usage: Option<OpenAiCompatUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAiCompatChatChoice {
    #[serde(default)]
    message: Option<OpenAiCompatChatMessageResponse>,
    #[serde(default)]
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiCompatChatMessageResponse {
    #[serde(default)]
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiCompatUsage {
    #[serde(default)]
    prompt_tokens: Option<u32>,
    #[serde(default)]
    completion_tokens: Option<u32>,
    #[serde(default)]
    total_tokens: Option<u32>,
}

#[async_trait]
impl LlmClient for OpenAiCompatAdapter {
    async fn completion(&self, req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let started = Instant::now(); // WAIVER [CX-573E] duration/timeout bookkeeping only

        let model_id = if req.model_id.trim().is_empty() {
            self.profile.model_id.clone()
        } else {
            req.model_id.clone()
        };

        let body = OpenAiCompatChatCompletionRequest {
            model: model_id.clone(),
            messages: vec![OpenAiCompatChatMessage {
                role: "user".to_string(),
                content: req.prompt.clone(),
            }],
            max_tokens: req.max_tokens,
            temperature: req.temperature,
            stop: req.stop_sequences.clone(),
            stream: false,
        };

        let url = self.chat_completions_url();
        let mut builder = self.client.post(&url).json(&body);
        if let Some(api_key) = &self.api_key {
            builder = builder.bearer_auth(api_key.as_str());
        }

        let response = builder
            .send()
            .await
            .map_err(|err| LlmError::ProviderError(format!("OpenAI compat request error: {err}")))?;

        let status = response.status();
        if status.as_u16() == 429 {
            return Err(LlmError::RateLimit);
        }
        if !status.is_success() {
            let error_text = match response.text().await {
                Ok(text) => text,
                Err(_) => String::new(),
            };
            return Err(LlmError::ProviderError(format!(
                "OpenAI compat error ({}): {}",
                status, error_text
            )));
        }

        let parsed: OpenAiCompatChatCompletionResponse = response.json().await.map_err(|err| {
            LlmError::ProviderError(format!("OpenAI compat response parse error: {err}"))
        })?;

        let first_choice = parsed.choices.first().ok_or_else(|| {
            LlmError::ProviderError("OpenAI compat response missing choices[0]".to_string())
        })?;

        let extracted_text = match &first_choice.message {
            Some(message) => match &message.content {
                Some(content) if !content.trim().is_empty() => content.clone(),
                _ => String::new(),
            },
            None => match &first_choice.text {
                Some(text) if !text.trim().is_empty() => text.clone(),
                _ => String::new(),
            },
        };

        if extracted_text.trim().is_empty() {
            return Err(LlmError::ProviderError(
                "OpenAI compat response missing assistant content".to_string(),
            ));
        }

        let prompt_tokens = match parsed.usage.as_ref().and_then(|u| u.prompt_tokens) {
            Some(value) => value,
            None => 0,
        };
        let completion_tokens = match parsed.usage.as_ref().and_then(|u| u.completion_tokens) {
            Some(value) => value,
            None => 0,
        };
        let total_tokens = match parsed.usage.as_ref().and_then(|u| u.total_tokens) {
            Some(value) => value,
            None => prompt_tokens.saturating_add(completion_tokens),
        };

        if let Some(max_tokens) = req.max_tokens {
            if completion_tokens > max_tokens {
                return Err(LlmError::BudgetExceeded(completion_tokens));
            }
        }

        let usage = super::TokenUsage {
            prompt_tokens,
            completion_tokens,
            total_tokens,
        };

        let latency_ms = started.elapsed().as_millis() as u64;
        self.emit_llm_inference_event(&req, &extracted_text, &usage, latency_ms)
            .await;

        Ok(CompletionResponse {
            text: extracted_text,
            usage,
            latency_ms,
        })
    }

    fn profile(&self) -> &ModelProfile {
        &self.profile
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
    use serde_json::json;
    use std::sync::{Arc, Mutex};
    use tokio::net::TcpListener;
    use uuid::Uuid;

    #[derive(Clone)]
    struct CapturingRecorder {
        events: Arc<Mutex<Vec<FlightRecorderEvent>>>,
    }

    impl CapturingRecorder {
        fn new() -> Self {
            Self {
                events: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn drain(&self) -> Vec<FlightRecorderEvent> {
            let mut guard = match self.events.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            let drained = guard.clone();
            guard.clear();
            drained
        }
    }

    #[async_trait]
    impl FlightRecorder for CapturingRecorder {
        async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
            let mut guard = match self.events.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            guard.push(event);
            Ok(())
        }

        async fn enforce_retention(&self) -> Result<u64, RecorderError> {
            Ok(0)
        }

        async fn list_events(
            &self,
            _filter: crate::flight_recorder::EventFilter,
        ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
            let guard = match self.events.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            Ok(guard.clone())
        }
    }

    #[derive(Clone, Default)]
    struct TestServerConfig {
        completion_text: String,
        usage_prompt_tokens: u32,
        usage_completion_tokens: u32,
    }

    async fn chat_completions_test_handler(
        State(cfg): State<TestServerConfig>,
        Json(_payload): Json<serde_json::Value>,
    ) -> (StatusCode, Json<serde_json::Value>) {
        let total_tokens = cfg
            .usage_prompt_tokens
            .saturating_add(cfg.usage_completion_tokens);
        (
            StatusCode::OK,
            Json(json!({
                "id": "chatcmpl-test",
                "object": "chat.completion",
                "created": 0,
                "model": "test-model",
                "choices": [{
                    "index": 0,
                    "message": { "role": "assistant", "content": cfg.completion_text }
                }],
                "usage": {
                    "prompt_tokens": cfg.usage_prompt_tokens,
                    "completion_tokens": cfg.usage_completion_tokens,
                    "total_tokens": total_tokens
                }
            })),
        )
    }

    async fn start_test_server(
        cfg: TestServerConfig,
    ) -> Result<(String, tokio::task::JoinHandle<()>), String> {
        let app = Router::new()
            .route("/v1/chat/completions", post(chat_completions_test_handler))
            .with_state(cfg);

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|err| err.to_string())?;
        let addr = listener
            .local_addr()
            .map_err(|err| err.to_string())?;

        let handle = tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        Ok((format!("http://{}", addr), handle))
    }

    #[tokio::test]
    async fn openai_compat_completion_hits_test_server_and_emits_llm_inference() {
        let cfg = TestServerConfig {
            completion_text: "Hello from test server".to_string(),
            usage_prompt_tokens: 3,
            usage_completion_tokens: 7,
        };
        let (base_url, _server) = match start_test_server(cfg).await {
            Ok(value) => value,
            Err(err) => {
                assert!(false, "start_test_server failed: {err}");
                return;
            }
        };

        let recorder = CapturingRecorder::new();
        let adapter = OpenAiCompatAdapter::new(
            base_url,
            "test-model".to_string(),
            8192,
            ModelTier::Local,
            None,
            Arc::new(recorder.clone()),
        );

        let trace_id = Uuid::new_v4();
        let req = CompletionRequest::new(trace_id, "Hello".to_string(), "test-model".to_string());
        let resp = match adapter.completion(req).await {
            Ok(resp) => resp,
            Err(err) => {
                assert!(false, "expected completion Ok, got err: {err}");
                return;
            }
        };

        assert_eq!(resp.text, "Hello from test server");
        assert_eq!(resp.usage.prompt_tokens, 3);
        assert_eq!(resp.usage.completion_tokens, 7);
        assert_eq!(resp.usage.total_tokens, 10);

        let events = recorder.drain();
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0].event_type,
            FlightRecorderEventType::LlmInference
        ));
        assert_eq!(events[0].trace_id, trace_id);
        assert_eq!(events[0].payload["type"], "llm_inference");
        assert_eq!(events[0].payload["model_id"], "test-model");
        assert_eq!(events[0].payload["token_usage"]["total_tokens"], 10);
        assert!(events[0].validate().is_ok());
    }

    #[tokio::test]
    async fn openai_compat_budget_exceeded_returns_typed_error() {
        let cfg = TestServerConfig {
            completion_text: "A".to_string(),
            usage_prompt_tokens: 1,
            usage_completion_tokens: 999,
        };
        let (base_url, _server) = match start_test_server(cfg).await {
            Ok(value) => value,
            Err(err) => {
                assert!(false, "start_test_server failed: {err}");
                return;
            }
        };

        let recorder = CapturingRecorder::new();
        let adapter = OpenAiCompatAdapter::new(
            base_url,
            "test-model".to_string(),
            8192,
            ModelTier::Local,
            None,
            Arc::new(recorder),
        );

        let trace_id = Uuid::new_v4();
        let req = CompletionRequest::new(trace_id, "Hello".to_string(), "test-model".to_string())
            .with_max_tokens(10);

        let err = match adapter.completion(req).await {
            Ok(_) => {
                assert!(false, "expected BudgetExceeded error");
                return;
            }
            Err(err) => err,
        };

        assert!(matches!(err, LlmError::BudgetExceeded(999)));
    }
}

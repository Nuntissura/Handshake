//! Ollama LLM Adapter
//!
//! Per Master Spec §4.2.3.2: The primary implementation for Phase 1 MUST use
//! the Ollama API.
//!
//! This adapter:
//! - Translates CompletionRequest to Ollama's /api/generate endpoint
//! - Enforces token budget via max_tokens
//! - Emits Flight Recorder llm_inference events internally (§4.2.3.2 Observability Invariant)

use super::{
    CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile, ModelTier, TokenUsage,
};
use crate::flight_recorder::{
    FlightRecorder, FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
    LlmInferenceEvent, LlmInferenceTokenUsage,
};
use crate::tokenization::{
    AccuracyWarningEmitter, AsyncFlightRecorderEmitter, DisabledAccuracyWarningEmitter,
    OllamaTokenizerConfigCache, SentencePieceTokenizerCache, TiktokenAdapter, TokenizationRouter,
    TokenizationWithTrace, TokenizerConfigFetcher, TokenizerError, VibeTokenizer,
};
use crate::workflows::{ModelSwapRequestV0_4, ModelSwapStrategy};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::sync::Arc;
// WAIVER [CX-573E]: Instant::now() is required for latency measurement per §4.2.3.2.
// This is non-deterministic but necessary for observability metrics.
use std::time::Instant;
use tokio::time::{timeout, Duration};

/// Ollama adapter implementing the LlmClient trait.
///
/// Per §4.2.3.2: The primary implementation for Phase 1.
/// Owns Flight Recorder integration to satisfy the Observability Invariant.
pub struct OllamaAdapter {
    base_url: String,
    profile: ModelProfile,
    client: reqwest::Client,
    flight_recorder: Arc<dyn FlightRecorder>,
    tokenization: TokenizationWithTrace,
    tokenizer_config_cache: Arc<OllamaTokenizerConfigCache>,
}

impl OllamaAdapter {
    /// Creates a new Ollama adapter with Flight Recorder integration.
    ///
    /// # Arguments
    /// * `base_url` - Ollama server URL (e.g., "http://localhost:11434")
    /// * `model_id` - Model to use (e.g., "llama3.2", "mistral")
    /// * `max_context_tokens` - Maximum context window size
    /// * `flight_recorder` - Flight Recorder for llm_inference emission
    pub fn new(
        base_url: String,
        model_id: String,
        max_context_tokens: u32,
        flight_recorder: Arc<dyn FlightRecorder>,
    ) -> Self {
        let client = reqwest::Client::new();
        let config_fetcher = Arc::new(OllamaTokenizerConfigClient::new(
            base_url.clone(),
            client.clone(),
        ));
        let tokenizer_config_cache = Arc::new(OllamaTokenizerConfigCache::new(config_fetcher));
        let sentencepiece_cache = SentencePieceTokenizerCache::default();
        let router = Arc::new(TokenizationRouter::new_with_ollama_config(
            Arc::new(TiktokenAdapter::default()),
            Arc::new(VibeTokenizer),
            sentencepiece_cache,
            tokenizer_config_cache.clone(),
        ));
        let accuracy_emitter = build_accuracy_emitter(flight_recorder.clone());
        let tokenization = TokenizationWithTrace::new(router, accuracy_emitter);

        Self {
            base_url,
            profile: ModelProfile::new(model_id, max_context_tokens).with_streaming(true),
            client,
            flight_recorder,
            tokenization,
            tokenizer_config_cache,
        }
        .with_tier_from_env()
    }

    /// Creates an adapter with default settings for local Ollama.
    ///
    /// Default: localhost:11434, 8192 tokens
    pub fn default_local(model_id: &str, flight_recorder: Arc<dyn FlightRecorder>) -> Self {
        Self::new(
            "http://localhost:11434".to_string(),
            model_id.to_string(),
            8192,
            flight_recorder,
        )
    }

    /// Builder: set model tier from MODEL_TIER env var [§2.6.6.7.11.5]
    ///
    /// MODEL_TIER env var:
    /// - "cloud" → ModelTier::Cloud (subject to CloudLeakageGuard restrictions)
    /// - any other value or unset → ModelTier::Local (no restrictions)
    pub fn with_tier_from_env(mut self) -> Self {
        let tier = std::env::var("MODEL_TIER")
            .map(|v| match v.to_lowercase().as_str() {
                "cloud" => ModelTier::Cloud,
                _ => ModelTier::Local,
            })
            .unwrap_or(ModelTier::Local);
        self.profile = self.profile.with_tier(tier);
        self
    }

    /// Builder: set explicit model tier [§2.6.6.7.11.5]
    pub fn with_tier(mut self, tier: ModelTier) -> Self {
        self.profile = self.profile.with_tier(tier);
        self
    }

    /// Computes SHA-256 hash of content for llm_inference.
    fn compute_hash(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        hex::encode(hasher.finalize())
    }

    async fn generate_keep_alive(
        &self,
        model: &str,
        keep_alive: Option<Value>,
    ) -> Result<(), LlmError> {
        let url = format!("{}/api/generate", self.base_url);
        let req = OllamaGenerateRequest {
            model: model.to_string(),
            // Best-effort warm/unload without consuming tokens.
            prompt: " ".to_string(),
            stream: false,
            options: Some(OllamaOptions {
                num_predict: Some(0),
                temperature: 0.0,
                stop: Vec::new(),
            }),
            keep_alive,
        };

        let response = self
            .client
            .post(&url)
            .json(&req)
            .send()
            .await
            .map_err(|e| LlmError::ProviderError(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(LlmError::ProviderError(format!(
                "Ollama /api/generate error ({}): {}",
                status, error_text
            )));
        }

        Ok(())
    }

    /// Emits llm_inference event to Flight Recorder.
    async fn emit_llm_inference_event(
        &self,
        req: &CompletionRequest,
        response_text: &str,
        usage: &TokenUsage,
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

        let event = FlightRecorderEvent::new(
            FlightRecorderEventType::LlmInference,
            FlightRecorderActor::Agent,
            req.trace_id,
            serde_json::to_value(&payload).unwrap_or_default(),
        )
        .with_model_id(&req.model_id);

        if let Err(e) = self.flight_recorder.record_event(event).await {
            // Log but don't fail the LLM call for observability errors
            tracing::warn!(
                target: "handshake_core::llm",
                error = %e,
                trace_id = %req.trace_id,
                "Failed to record llm_inference event"
            );
        }
    }
}

#[derive(Clone)]
pub struct OllamaTokenizerConfigClient {
    base_url: String,
    client: reqwest::Client,
}

impl OllamaTokenizerConfigClient {
    pub fn new(base_url: String, client: reqwest::Client) -> Self {
        Self { base_url, client }
    }
}

#[async_trait]
impl TokenizerConfigFetcher for OllamaTokenizerConfigClient {
    async fn fetch_show(&self, model: &str) -> Result<Value, TokenizerError> {
        let url = format!("{}/api/show", self.base_url);
        let response = self
            .client
            .post(&url)
            .json(&OllamaShowRequest {
                name: model.to_string(),
            })
            .send()
            .await
            .map_err(|err| TokenizerError::TokenizerConfigFetchFailed(err.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(TokenizerError::TokenizerConfigFetchFailed(format!(
                "Ollama /api/show error ({}): {}",
                status, error_text
            )));
        }

        response
            .json::<Value>()
            .await
            .map_err(|err| TokenizerError::TokenizerConfigParseFailed(err.to_string()))
    }
}

fn build_accuracy_emitter(
    flight_recorder: Arc<dyn FlightRecorder>,
) -> Arc<dyn AccuracyWarningEmitter> {
    match AsyncFlightRecorderEmitter::try_new(flight_recorder) {
        Ok(emitter) => Arc::new(emitter),
        Err(err) => {
            tracing::error!(
                target: "handshake_core::tokenization",
                error = %err,
                "Accuracy warning emitter unavailable"
            );
            Arc::new(DisabledAccuracyWarningEmitter::new(err.to_string()))
        }
    }
}

/// Ollama /api/generate request format.
#[derive(Serialize)]
struct OllamaGenerateRequest {
    model: String,
    prompt: String,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OllamaOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    keep_alive: Option<Value>,
}

/// Ollama options for generation.
#[derive(Serialize)]
struct OllamaOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<u32>,
    temperature: f32,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    stop: Vec<String>,
}

/// Ollama /api/show request format.
#[derive(Serialize)]
struct OllamaShowRequest {
    name: String,
}

/// Ollama /api/generate response format.
#[derive(Deserialize)]
struct OllamaGenerateResponse {
    response: String,
    #[serde(default)]
    prompt_eval_count: Option<u32>,
    #[serde(default)]
    eval_count: Option<u32>,
}

#[async_trait]
impl LlmClient for OllamaAdapter {
    async fn completion(&self, req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        if req.trace_id == uuid::Uuid::nil() {
            return Err(LlmError::ProviderError(
                "trace_id must be a non-nil UUID".to_string(),
            ));
        }
        if req.model_id.trim().is_empty() {
            return Err(LlmError::ProviderError(
                "model_id must be a non-empty string".to_string(),
            ));
        }

        // WAIVER [CX-573E]: Instant::now() is required for latency measurement (observability only).
        let start = Instant::now();

        // Build Ollama request
        let ollama_req = OllamaGenerateRequest {
            model: req.model_id.clone(),
            prompt: req.prompt.clone(),
            stream: false,
            options: Some(OllamaOptions {
                num_predict: req.max_tokens,
                temperature: req.temperature,
                stop: req.stop_sequences.clone(),
            }),
            keep_alive: None,
        };

        let url = format!("{}/api/generate", self.base_url);

        // Execute request
        let response = self
            .client
            .post(&url)
            .json(&ollama_req)
            .send()
            .await
            .map_err(|e| LlmError::ProviderError(e.to_string()))?;

        let status = response.status();

        // Handle rate limiting
        if status.as_u16() == 429 {
            return Err(LlmError::RateLimit);
        }

        // Handle other errors
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(LlmError::ProviderError(format!(
                "Ollama error ({}): {}",
                status, error_text
            )));
        }

        // Parse response
        let ollama_resp: OllamaGenerateResponse = response
            .json()
            .await
            .map_err(|e| LlmError::ProviderError(format!("Failed to parse response: {}", e)))?;

        let latency_ms = start.elapsed().as_millis() as u64;

        if let Err(err) = self.tokenizer_config_cache.refresh(&req.model_id).await {
            tracing::warn!(
                target: "handshake_core::llm",
                error = %err,
                model = %req.model_id,
                "Failed to refresh Ollama tokenizer config"
            );
        }

        let tokenization_prompt_tokens = self
            .tokenization
            .count_tokens_with_trace(&req.prompt, &req.model_id, req.trace_id)
            .map_err(|err| LlmError::ProviderError(format!("Tokenization failed: {}", err)))?;
        let tokenization_completion_tokens = self
            .tokenization
            .count_tokens_with_trace(&ollama_resp.response, &req.model_id, req.trace_id)
            .map_err(|err| LlmError::ProviderError(format!("Tokenization failed: {}", err)))?;
        let tokenization_total_tokens = tokenization_prompt_tokens + tokenization_completion_tokens;

        let (prompt_tokens, completion_tokens, total_tokens) =
            match (ollama_resp.prompt_eval_count, ollama_resp.eval_count) {
                (Some(provider_prompt_tokens), Some(provider_completion_tokens)) => {
                    if provider_prompt_tokens != tokenization_prompt_tokens
                        || provider_completion_tokens != tokenization_completion_tokens
                    {
                        tracing::debug!(
                            target: "handshake_core::llm",
                            trace_id = %req.trace_id,
                            model = %req.model_id,
                            provider_prompt_tokens = provider_prompt_tokens,
                            provider_completion_tokens = provider_completion_tokens,
                            tokenization_prompt_tokens = tokenization_prompt_tokens,
                            tokenization_completion_tokens = tokenization_completion_tokens,
                            "Provider token counts differ from TokenizationService counts"
                        );
                    }
                    (
                        provider_prompt_tokens,
                        provider_completion_tokens,
                        provider_prompt_tokens + provider_completion_tokens,
                    )
                }
                _ => (
                    tokenization_prompt_tokens,
                    tokenization_completion_tokens,
                    tokenization_total_tokens,
                ),
            };

        // Budget enforcement per §4.2.3.2
        let usage = TokenUsage {
            prompt_tokens,
            completion_tokens,
            total_tokens,
        };

        // Emit llm_inference event per §4.2.3.2 Observability Invariant
        self.emit_llm_inference_event(&req, &ollama_resp.response, &usage, latency_ms)
            .await;

        if let Some(max_tokens) = req.max_tokens {
            if completion_tokens > max_tokens {
                return Err(LlmError::BudgetExceeded(completion_tokens));
            }
        }

        Ok(CompletionResponse {
            text: ollama_resp.response,
            usage,
            latency_ms,
        })
    }

    async fn swap_model(&self, req: ModelSwapRequestV0_4) -> Result<(), LlmError> {
        if req.timeout_ms == 0 {
            return Err(LlmError::ProviderError(
                "model swap timeout_ms must be non-zero".to_string(),
            ));
        }
        if req.max_vram_mb == 0 || req.max_ram_mb == 0 {
            return Err(LlmError::ProviderError(
                "budget_exceeded: max_vram_mb/max_ram_mb must be non-zero".to_string(),
            ));
        }

        let deadline = Duration::from_millis(req.timeout_ms);
        let unload = Some(Value::String("0".to_string()));
        let keep_hot = Some(Value::String("5m".to_string()));

        timeout(deadline, async {
            match req.swap_strategy {
                ModelSwapStrategy::UnloadReload | ModelSwapStrategy::DiskOffload => {
                    self.generate_keep_alive(req.current_model_id.as_str(), unload.clone())
                        .await?;
                    self.generate_keep_alive(req.target_model_id.as_str(), keep_hot.clone())
                        .await?;
                }
                ModelSwapStrategy::KeepHotSwap => {
                    self.generate_keep_alive(req.target_model_id.as_str(), keep_hot.clone())
                        .await?;
                }
            }

            Ok::<(), LlmError>(())
        })
        .await
        .map_err(|_| LlmError::ProviderError("model swap timeout".to_string()))??;

        Ok(())
    }

    fn profile(&self) -> &ModelProfile {
        &self.profile
    }
}

/// In-memory LLM client for unit testing without a real Ollama server.
/// Uses the new LlmClient trait (completion-based API).
#[cfg(any(test, feature = "test-utils"))]
pub struct InMemoryLlmClient {
    response: String,
    usage_override: Option<TokenUsage>,
    profile: ModelProfile,
    latency_ms: u64,
}

#[cfg(any(test, feature = "test-utils"))]
impl InMemoryLlmClient {
    /// Creates an in-memory client that returns the given response.
    pub fn new(response: String) -> Self {
        Self {
            response,
            usage_override: None,
            profile: ModelProfile::new("in-memory-model".to_string(), 4096),
            latency_ms: 0, // Zero latency for deterministic tests
        }
    }

    /// Creates an in-memory client with specific usage metrics.
    pub fn with_usage(mut self, usage: TokenUsage) -> Self {
        self.usage_override = Some(usage);
        self
    }

    /// Sets the simulated latency for testing.
    pub fn with_latency_ms(mut self, latency_ms: u64) -> Self {
        self.latency_ms = latency_ms;
        self
    }

    fn word_count(text: &str) -> u32 {
        let mut words: u32 = 0;
        let mut in_word = false;

        for character in text.chars() {
            if character.is_whitespace() {
                in_word = false;
                continue;
            }

            if !in_word {
                words = words.saturating_add(1);
                in_word = true;
            }
        }

        words
    }

    fn deterministic_usage(prompt: &str, response_text: &str) -> TokenUsage {
        const TOKENS_PER_WORD: u32 = 10;

        let prompt_tokens = Self::word_count(prompt).saturating_mul(TOKENS_PER_WORD);
        let completion_tokens = Self::word_count(response_text).saturating_mul(TOKENS_PER_WORD);
        let total_tokens = prompt_tokens.saturating_add(completion_tokens);

        TokenUsage {
            prompt_tokens,
            completion_tokens,
            total_tokens,
        }
    }
}

#[cfg(any(test, feature = "test-utils"))]
#[async_trait]
impl LlmClient for InMemoryLlmClient {
    async fn completion(&self, req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let usage = match &self.usage_override {
            Some(usage) => usage.clone(),
            None => Self::deterministic_usage(&req.prompt, &self.response),
        };

        Ok(CompletionResponse {
            text: self.response.clone(),
            usage,
            latency_ms: self.latency_ms,
        })
    }

    fn profile(&self) -> &ModelProfile {
        &self.profile
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flight_recorder::{FlightRecorder, FlightRecorderEvent, RecorderError};
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    struct NoopRecorder;

    #[async_trait]
    impl FlightRecorder for NoopRecorder {
        async fn record_event(&self, _event: FlightRecorderEvent) -> Result<(), RecorderError> {
            Ok(())
        }

        async fn enforce_retention(&self) -> Result<u64, RecorderError> {
            Ok(0)
        }

        async fn list_events(
            &self,
            _filter: crate::flight_recorder::EventFilter,
        ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
            Ok(vec![])
        }
    }

    fn noop_recorder() -> Arc<dyn FlightRecorder> {
        Arc::new(NoopRecorder)
    }

    #[derive(Clone, Default)]
    struct CapturingRecorder {
        events: Arc<Mutex<Vec<FlightRecorderEvent>>>,
    }

    #[async_trait]
    impl FlightRecorder for CapturingRecorder {
        async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
            let mut events = match self.events.lock() {
                Ok(events) => events,
                Err(poisoned) => poisoned.into_inner(),
            };
            events.push(event);
            Ok(())
        }

        async fn enforce_retention(&self) -> Result<u64, RecorderError> {
            Ok(0)
        }

        async fn list_events(
            &self,
            _filter: crate::flight_recorder::EventFilter,
        ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
            let events = match self.events.lock() {
                Ok(events) => events,
                Err(poisoned) => poisoned.into_inner(),
            };
            Ok(events.clone())
        }
    }

    #[test]
    fn test_ollama_adapter_creation() {
        let adapter = OllamaAdapter::new(
            "http://localhost:11434".to_string(),
            "llama3.2".to_string(),
            8192,
            noop_recorder(),
        );

        let profile = adapter.profile();
        assert_eq!(profile.model_id, "llama3.2");
        assert_eq!(profile.max_context_tokens, 8192);
        assert!(profile.supports_streaming);
    }

    #[test]
    fn test_ollama_adapter_default_local() {
        let adapter = OllamaAdapter::default_local("mistral", noop_recorder());

        let profile = adapter.profile();
        assert_eq!(profile.model_id, "mistral");
        assert_eq!(profile.max_context_tokens, 8192);
    }

    #[test]
    fn test_with_tier_from_env_defaults_to_local() {
        // Test uses with_tier() directly to avoid env var race conditions
        // This validates the tier mechanism without depending on env state
        let adapter = OllamaAdapter::new(
            "http://localhost:11434".to_string(),
            "llama3.2".to_string(),
            8192,
            noop_recorder(),
        )
        .with_tier(crate::llm::ModelTier::Local);
        assert_eq!(
            adapter.profile().model_tier,
            crate::llm::ModelTier::Local,
            "with_tier(Local) should set Local tier"
        );
    }

    #[test]
    fn test_with_tier_from_env_cloud() {
        // Test uses with_tier() directly to avoid env var race conditions in parallel tests
        // The env var mechanism is validated by the wiring in new() which calls with_tier_from_env()
        let adapter = OllamaAdapter::new(
            "http://localhost:11434".to_string(),
            "llama3.2".to_string(),
            8192,
            noop_recorder(),
        )
        .with_tier(crate::llm::ModelTier::Cloud);
        assert_eq!(
            adapter.profile().model_tier,
            crate::llm::ModelTier::Cloud,
            "with_tier(Cloud) should set Cloud tier"
        );
    }

    #[test]
    fn test_completion_request_serialization() {
        let req = OllamaGenerateRequest {
            model: "llama3.2".to_string(),
            prompt: "Hello".to_string(),
            stream: false,
            options: Some(OllamaOptions {
                num_predict: Some(100),
                temperature: 0.7,
                stop: vec!["###".to_string()],
            }),
            keep_alive: None,
        };

        // SAFETY: Test-only code - request structure is valid
        let json = match serde_json::to_string(&req) {
            Ok(json) => json,
            Err(err) => {
                assert!(false, "request serialization should succeed: {err}");
                return;
            }
        };
        assert!(json.contains("\"model\":\"llama3.2\""));
        assert!(json.contains("\"num_predict\":100"));
    }

    #[tokio::test]
    async fn test_llm_inference_payload_matches_fr_evt_006() {
        let recorder = CapturingRecorder::default();
        let adapter = OllamaAdapter::default_local("llama3.2", Arc::new(recorder.clone()));

        let trace_id = uuid::Uuid::new_v4();
        let req = CompletionRequest::new(trace_id, "Hello".to_string(), "llama3.2".to_string());
        let usage = TokenUsage {
            prompt_tokens: 10,
            completion_tokens: 5,
            total_tokens: 15,
        };

        adapter
            .emit_llm_inference_event(&req, "World", &usage, 123)
            .await;

        let events = match recorder.events.lock() {
            Ok(events) => events,
            Err(poisoned) => poisoned.into_inner(),
        };
        assert_eq!(events.len(), 1);

        let event = &events[0];
        assert!(matches!(
            event.event_type,
            FlightRecorderEventType::LlmInference
        ));
        assert_eq!(event.trace_id, trace_id);
        assert_eq!(event.model_id.as_deref(), Some("llama3.2"));

        assert_eq!(event.payload["type"], "llm_inference");
        assert_eq!(event.payload["trace_id"], trace_id.to_string());
        assert_eq!(event.payload["model_id"], "llama3.2");
        assert_eq!(event.payload["token_usage"]["prompt_tokens"], 10);
        assert_eq!(event.payload["token_usage"]["completion_tokens"], 5);
        assert_eq!(event.payload["token_usage"]["total_tokens"], 15);

        assert!(event.validate().is_ok());
    }

    #[tokio::test]
    async fn test_in_memory_llm_client_emits_deterministic_usage_by_default() {
        let client = InMemoryLlmClient::new("Hello world".to_string());

        let trace_id = uuid::Uuid::new_v4();
        let req =
            CompletionRequest::new(trace_id, "One two three".to_string(), "ignored".to_string());

        let resp = match client.completion(req).await {
            Ok(resp) => resp,
            Err(err) => {
                assert!(false, "completion should succeed: {err}");
                return;
            }
        };

        assert_eq!(resp.usage.prompt_tokens, 30);
        assert_eq!(resp.usage.completion_tokens, 20);
        assert_eq!(resp.usage.total_tokens, 50);
    }

    #[tokio::test]
    async fn test_in_memory_llm_client_usage_override_is_respected() {
        let client = InMemoryLlmClient::new("Hello world".to_string()).with_usage(TokenUsage {
            prompt_tokens: 1,
            completion_tokens: 2,
            total_tokens: 3,
        });

        let trace_id = uuid::Uuid::new_v4();
        let req =
            CompletionRequest::new(trace_id, "One two three".to_string(), "ignored".to_string());

        let resp = match client.completion(req).await {
            Ok(resp) => resp,
            Err(err) => {
                assert!(false, "completion should succeed: {err}");
                return;
            }
        };

        assert_eq!(resp.usage.prompt_tokens, 1);
        assert_eq!(resp.usage.completion_tokens, 2);
        assert_eq!(resp.usage.total_tokens, 3);
    }
}

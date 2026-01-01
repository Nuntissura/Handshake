//! Ollama LLM Adapter
//!
//! Per Master Spec §4.2.3.2: The primary implementation for Phase 1 MUST use
//! the Ollama API.
//!
//! This adapter:
//! - Translates CompletionRequest to Ollama's /api/generate endpoint
//! - Enforces token budget via max_tokens
//! - Emits Flight Recorder FR-EVT-002 events internally (§4.2.3.2 Observability Invariant)

use super::{CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile, TokenUsage};
use crate::flight_recorder::{
    FlightRecorder, FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
    FrEvt002LlmInference,
};
use crate::tokenization::{
    AccuracyWarningEmitter, AsyncFlightRecorderEmitter, DisabledAccuracyWarningEmitter,
    OllamaTokenizerConfigCache, SentencePieceTokenizerCache, TiktokenAdapter, TokenizationRouter,
    TokenizationWithTrace, TokenizerConfigFetcher, TokenizerError, VibeTokenizer,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::sync::Arc;
// WAIVER [CX-573E]: Instant::now() is required for latency measurement per §4.2.3.2.
// This is non-deterministic but necessary for observability metrics.
use std::time::Instant;

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
    /// * `flight_recorder` - Flight Recorder for FR-EVT-002 emission
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

    /// Computes SHA-256 hash of content for FR-EVT-002.
    fn compute_hash(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Emits FR-EVT-002 LlmInference event to Flight Recorder.
    async fn emit_llm_inference_event(
        &self,
        req: &CompletionRequest,
        response_text: &str,
        usage: &TokenUsage,
        latency_ms: u64,
    ) {
        let payload = FrEvt002LlmInference {
            model_id: req.model_id.clone(),
            input_tokens: usage.prompt_tokens as u64,
            output_tokens: usage.completion_tokens as u64,
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
                "Failed to record FR-EVT-002 LlmInference event"
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

        let prompt_tokens = self
            .tokenization
            .count_tokens_with_trace(&req.prompt, &req.model_id, req.trace_id)
            .map_err(|err| LlmError::ProviderError(format!("Tokenization failed: {}", err)))?;
        let completion_tokens = self
            .tokenization
            .count_tokens_with_trace(&ollama_resp.response, &req.model_id, req.trace_id)
            .map_err(|err| LlmError::ProviderError(format!("Tokenization failed: {}", err)))?;
        let total_tokens = prompt_tokens + completion_tokens;

        if let (Some(provider_prompt), Some(provider_completion)) =
            (ollama_resp.prompt_eval_count, ollama_resp.eval_count)
        {
            if provider_prompt != prompt_tokens || provider_completion != completion_tokens {
                tracing::debug!(
                    target: "handshake_core::llm",
                    trace_id = %req.trace_id,
                    model = %req.model_id,
                    provider_prompt_tokens = provider_prompt,
                    provider_completion_tokens = provider_completion,
                    tokenization_prompt_tokens = prompt_tokens,
                    tokenization_completion_tokens = completion_tokens,
                    "Provider token counts differ from TokenizationService counts"
                );
            }
        }

        // Budget enforcement per §4.2.3.2
        if let Some(max_tokens) = req.max_tokens {
            if total_tokens > max_tokens {
                return Err(LlmError::BudgetExceeded(total_tokens));
            }
        }

        let usage = TokenUsage {
            prompt_tokens,
            completion_tokens,
            total_tokens,
        };

        // Emit FR-EVT-002 LlmInference event per §4.2.3.2 Observability Invariant
        self.emit_llm_inference_event(&req, &ollama_resp.response, &usage, latency_ms)
            .await;

        Ok(CompletionResponse {
            text: ollama_resp.response,
            usage,
            latency_ms,
        })
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
    usage: TokenUsage,
    profile: ModelProfile,
    latency_ms: u64,
}

#[cfg(any(test, feature = "test-utils"))]
impl InMemoryLlmClient {
    /// Creates an in-memory client that returns the given response.
    pub fn new(response: String) -> Self {
        Self {
            response,
            usage: TokenUsage::default(),
            profile: ModelProfile::new("in-memory-model".to_string(), 4096),
            latency_ms: 0, // Zero latency for deterministic tests
        }
    }

    /// Creates an in-memory client with specific usage metrics.
    pub fn with_usage(mut self, usage: TokenUsage) -> Self {
        self.usage = usage;
        self
    }

    /// Sets the simulated latency for testing.
    pub fn with_latency_ms(mut self, latency_ms: u64) -> Self {
        self.latency_ms = latency_ms;
        self
    }
}

#[cfg(any(test, feature = "test-utils"))]
#[async_trait]
impl LlmClient for InMemoryLlmClient {
    async fn completion(&self, _req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        Ok(CompletionResponse {
            text: self.response.clone(),
            usage: self.usage.clone(),
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
    use std::sync::Arc;

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
        };

        // SAFETY: Test-only code - request structure is valid
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"model\":\"llama3.2\""));
        assert!(json.contains("\"num_predict\":100"));
    }
}

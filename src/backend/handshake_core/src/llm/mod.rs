//! LLM Client Adapter [HSK-TRAIT-004]
//!
//! Per Master Spec §4.2.3: All application code MUST interact with LLMs
//! through the `LlmClient` trait. This ensures provider portability and
//! centralized observability via Flight Recorder.

pub mod ollama;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

// Re-export primary types for convenient access
pub use ollama::OllamaAdapter;

/// HSK-TRAIT-004: LLM Client Adapter
///
/// All LLM interactions MUST go through this trait to satisfy [CX-101].
/// Implementations are responsible for:
/// - Token budget enforcement
/// - Flight Recorder event emission
/// - Provider-specific API translation
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Executes a completion request.
    ///
    /// Returns:
    /// - `Ok(CompletionResponse)`: The generated text and usage metadata.
    /// - `Err(LlmError)`: If the request fails or budget is exceeded.
    ///
    /// Implementers MUST emit a Flight Recorder event with `trace_id`,
    /// `model_id`, and `TokenUsage` per §4.2.3.2.
    async fn completion(&self, req: CompletionRequest) -> Result<CompletionResponse, LlmError>;

    /// Returns the model profile (capabilities, token limits).
    fn profile(&self) -> &ModelProfile;
}

/// Request payload for LLM completion.
///
/// Per §4.2.3.1 with §11.5 traceability requirement.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CompletionRequest {
    /// Unique trace identifier for Flight Recorder correlation.
    /// Required per §11.5: "Every model call MUST emit a Flight Recorder
    /// event containing trace_id."
    pub trace_id: Uuid,
    /// The prompt text to send to the model.
    pub prompt: String,
    /// Model identifier (e.g., "llama3.2", "mistral").
    pub model_id: String,
    /// Maximum tokens to generate. If `None`, uses model default.
    /// Budget enforcement checks this against `ModelProfile::max_context_tokens`.
    pub max_tokens: Option<u32>,
    /// Sampling temperature (0.0 = deterministic, 1.0+ = creative).
    pub temperature: f32,
    /// Sequences that cause generation to stop.
    pub stop_sequences: Vec<String>,
}

impl CompletionRequest {
    /// Creates a new completion request with required trace_id.
    pub fn new(trace_id: Uuid, prompt: String, model_id: String) -> Self {
        Self {
            trace_id,
            prompt,
            model_id,
            max_tokens: None,
            temperature: 0.7,
            stop_sequences: Vec::new(),
        }
    }

    /// Builder: set max_tokens.
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Builder: set temperature.
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }

    /// Builder: set stop sequences.
    pub fn with_stop_sequences(mut self, stop_sequences: Vec<String>) -> Self {
        self.stop_sequences = stop_sequences;
        self
    }
}

/// Response from LLM completion.
///
/// Per §4.2.3.1.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CompletionResponse {
    /// The generated text.
    pub text: String,
    /// Token usage metrics for budgeting and observability.
    pub usage: TokenUsage,
    /// Request latency in milliseconds.
    pub latency_ms: u64,
}

/// Token usage metrics for budgeting and Flight Recorder.
///
/// Per §4.2.3.1.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct TokenUsage {
    /// Tokens consumed by the prompt.
    pub prompt_tokens: u32,
    /// Tokens generated in the completion.
    pub completion_tokens: u32,
    /// Total tokens (prompt + completion).
    pub total_tokens: u32,
}

/// Model capabilities and limits.
///
/// Per §4.2.3.1.
#[derive(Debug, Clone)]
pub struct ModelProfile {
    /// Model identifier.
    pub model_id: String,
    /// Maximum context window size in tokens.
    pub max_context_tokens: u32,
    /// Whether the model supports streaming responses.
    pub supports_streaming: bool,
}

impl ModelProfile {
    /// Creates a new model profile.
    pub fn new(model_id: String, max_context_tokens: u32) -> Self {
        Self {
            model_id,
            max_context_tokens,
            supports_streaming: false,
        }
    }

    /// Builder: set streaming support.
    pub fn with_streaming(mut self, supports_streaming: bool) -> Self {
        self.supports_streaming = supports_streaming;
        self
    }
}

/// LLM error types with stable HSK error codes.
///
/// Per §4.2.3.1.
#[derive(Debug, Error)]
pub enum LlmError {
    /// HSK-429-RATE-LIMIT: Provider rate limit exceeded.
    #[error("HSK-429-RATE-LIMIT: Provider rate limit exceeded")]
    RateLimit,

    /// HSK-402-BUDGET-EXCEEDED: Token budget exceeded.
    /// Contains the number of tokens that exceeded the budget.
    #[error("HSK-402-BUDGET-EXCEEDED: Token budget exceeded: {0}")]
    BudgetExceeded(u32),

    /// HSK-500-LLM: Internal provider error.
    #[error("HSK-500-LLM: Internal provider error: {0}")]
    ProviderError(String),
}

// =============================================================================
// BACKWARD COMPATIBILITY LAYER (Deprecated)
// =============================================================================
// The following types maintain compatibility with existing code that uses
// the chat-based API. New code SHOULD use `LlmClient` with `completion()`.
// These will be removed in a future version.

/// Legacy chat message format.
///
/// **Deprecated:** Use `CompletionRequest` instead for new code.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Legacy LLM client trait using chat paradigm.
///
/// **Deprecated:** Use `LlmClient` with `completion()` instead for new code.
/// This trait is retained for backward compatibility with workflows.rs.
// WAIVER [CX-573E]: Stringly errors (Result<String, String>) are retained in this
// deprecated trait for backward compatibility. Changing the signature would break
// existing callers in workflows.rs. New code MUST use LlmClient with LlmError.
#[async_trait]
pub trait LLMClient: Send + Sync {
    async fn chat(&self, messages: Vec<ChatMessage>) -> Result<String, String>;

    async fn chat_with_budget(
        &self,
        messages: Vec<ChatMessage>,
        _token_budget: u32,
    ) -> Result<String, String> {
        self.chat(messages).await
    }
}

/// Legacy Ollama client using chat paradigm.
///
/// **Deprecated:** Use `OllamaAdapter` instead for new code.
// WAIVER [CX-573E]: Stringly errors (Result<String, String>) are required by
// the legacy LLMClient trait signature for backward compatibility with workflows.rs.
// New code MUST use LlmClient with LlmError instead.
pub struct OllamaClient {
    pub base_url: String,
    pub model: String,
}

impl OllamaClient {
    pub fn new(base_url: String, model: String) -> Self {
        Self { base_url, model }
    }
}

#[derive(Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
}

#[derive(Deserialize)]
struct OllamaChatResponse {
    message: ChatMessage,
}

#[async_trait]
impl LLMClient for OllamaClient {
    async fn chat(&self, messages: Vec<ChatMessage>) -> Result<String, String> {
        let client = reqwest::Client::new();
        let url = format!("{}/api/chat", self.base_url);

        let request = OllamaChatRequest {
            model: self.model.clone(),
            messages,
            stream: false,
        };

        let response = client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let status = response.status();
        if !status.is_success() {
            let error_text: String = response.text().await.unwrap_or_default();
            return Err(format!("Ollama error ({}): {}", status, error_text));
        }

        let chat_response: OllamaChatResponse = response.json().await.map_err(|e| e.to_string())?;

        Ok(chat_response.message.content)
    }
}

/// Test client for unit testing (legacy API).
///
/// **Deprecated:** Use `InMemoryLlmClient` instead for new code.
#[cfg(any(test, feature = "test-utils"))]
pub struct TestLLMClient {
    pub response: String,
}

#[cfg(any(test, feature = "test-utils"))]
#[async_trait]
impl LLMClient for TestLLMClient {
    async fn chat(&self, _messages: Vec<ChatMessage>) -> Result<String, String> {
        Ok(self.response.clone())
    }

    async fn chat_with_budget(
        &self,
        messages: Vec<ChatMessage>,
        _token_budget: u32,
    ) -> Result<String, String> {
        self.chat(messages).await
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completion_request_builder() {
        let trace_id = Uuid::new_v4();
        let req = CompletionRequest::new(
            trace_id,
            "Hello, world!".to_string(),
            "llama3.2".to_string(),
        )
        .with_max_tokens(100)
        .with_temperature(0.5)
        .with_stop_sequences(vec!["###".to_string()]);

        assert_eq!(req.trace_id, trace_id);
        assert_eq!(req.prompt, "Hello, world!");
        assert_eq!(req.model_id, "llama3.2");
        assert_eq!(req.max_tokens, Some(100));
        assert_eq!(req.temperature, 0.5);
        assert_eq!(req.stop_sequences, vec!["###".to_string()]);
    }

    #[test]
    fn test_model_profile_builder() {
        let profile = ModelProfile::new("llama3.2".to_string(), 8192).with_streaming(true);

        assert_eq!(profile.model_id, "llama3.2");
        assert_eq!(profile.max_context_tokens, 8192);
        assert!(profile.supports_streaming);
    }

    #[test]
    fn test_llm_error_display() {
        let rate_limit = LlmError::RateLimit;
        assert_eq!(
            rate_limit.to_string(),
            "HSK-429-RATE-LIMIT: Provider rate limit exceeded"
        );

        let budget = LlmError::BudgetExceeded(1500);
        assert_eq!(
            budget.to_string(),
            "HSK-402-BUDGET-EXCEEDED: Token budget exceeded: 1500"
        );

        let provider = LlmError::ProviderError("Connection timeout".to_string());
        assert_eq!(
            provider.to_string(),
            "HSK-500-LLM: Internal provider error: Connection timeout"
        );
    }
}

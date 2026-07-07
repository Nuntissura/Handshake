//! In-memory LLM client for tests and feature-gated test utilities.
//!
//! This is intentionally separate from provider adapters so tests can keep a
//! deterministic `LlmClient` helper after the WP-1 MT-003 Ollama adapter
//! removal.

use async_trait::async_trait;
use sha2::{Digest, Sha256};

use super::{
    CompletionRequest, CompletionResponse, EmbeddingRequest, EmbeddingResponse, LlmClient,
    LlmError, ModelProfile, TokenUsage,
};

/// In-memory LLM client for unit testing without a real provider server.
pub struct InMemoryLlmClient {
    response: String,
    usage_override: Option<TokenUsage>,
    profile: ModelProfile,
    latency_ms: u64,
    /// When `Some(dim)`, [`LlmClient::embedding`] produces a deterministic dense
    /// vector of this dimensionality from the input text. This is an honest
    /// embedding substitute for tests: the vector is a genuine function of the
    /// text, so semantically-overlapping inputs land closer under pgvector
    /// distance. When `None`, the embedding call declines with the typed
    /// `EmbeddingUnsupported` error, matching a runtime with no embedding model
    /// configured.
    embedding_dim: Option<usize>,
}

impl InMemoryLlmClient {
    /// Creates an in-memory client that returns the given response.
    pub fn new(response: String) -> Self {
        Self {
            response,
            usage_override: None,
            profile: ModelProfile::new("in-memory-model".to_string(), 4096),
            latency_ms: 0,
            embedding_dim: None,
        }
    }

    /// Enables the deterministic embedding path with the given dimensionality.
    pub fn with_embedding_dim(mut self, dim: usize) -> Self {
        self.embedding_dim = Some(dim);
        self
    }

    /// Deterministic bag-of-words embedding used by the test/util embedding
    /// path. Tokens are lowercased; each token hashes to a dimension and
    /// accumulates a sign-stable weight; the vector is L2-normalized.
    pub fn deterministic_embedding(input: &str, dim: usize) -> Vec<f32> {
        let mut vector = vec![0.0f32; dim.max(1)];
        for token in input
            .to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|t| !t.is_empty())
        {
            let mut hasher = Sha256::new();
            hasher.update(token.as_bytes());
            let digest = hasher.finalize();
            let idx = (u32::from_be_bytes([digest[0], digest[1], digest[2], digest[3]]) as usize)
                % vector.len();
            let sign = if digest[4] & 1 == 0 { 1.0 } else { -1.0 };
            vector[idx] += sign;
        }
        let norm: f32 = vector.iter().map(|v| v * v).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in &mut vector {
                *v /= norm;
            }
        }
        vector
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

    async fn embedding(&self, req: EmbeddingRequest) -> Result<EmbeddingResponse, LlmError> {
        match self.embedding_dim {
            Some(dim) => Ok(EmbeddingResponse {
                vector: Self::deterministic_embedding(&req.input, dim),
                model_id: req.model_id,
                latency_ms: self.latency_ms,
            }),
            None => Err(LlmError::EmbeddingUnsupported),
        }
    }

    fn profile(&self) -> &ModelProfile {
        &self.profile
    }
}

//! Tokenization Service (normative) - panic-free token counting and truncation.
//! Aligns with Master Spec §4.6 Tokenization Service and WP-1-Tokenization-Service-v2.

use std::sync::Arc;

use async_trait::async_trait;
use thiserror::Error;
use tracing::warn;

#[derive(Debug, Error, Clone)]
pub enum TokenizerError {
    #[error("Unknown model: {0}")]
    UnknownModel(String),
    #[error("Tokenization failed: {0}")]
    TokenizationFailed(String),
    #[error("Tokenizer unavailable: {0}")]
    TokenizerUnavailable(String),
    #[error("Decode failed: {0}")]
    DecodeFailed(String),
}

#[async_trait]
pub trait Tokenizer: Send + Sync {
    async fn count_tokens(&self, text: &str, model: &str) -> Result<u32, TokenizerError>;
    async fn truncate(&self, text: &str, limit: u32, model: &str)
        -> Result<String, TokenizerError>;
}

/// Tiktoken-backed tokenizer with a cl100k_base fallback for unknown models.
#[derive(Debug, Clone)]
pub struct TiktokenAdapter {
    fallback_model: &'static str,
}

impl Default for TiktokenAdapter {
    fn default() -> Self {
        Self {
            fallback_model: "cl100k_base",
        }
    }
}

fn vibe_estimate_count(text: &str) -> u32 {
    let char_count = text.chars().count() as u32;
    char_count.div_ceil(4)
}

fn vibe_truncate(text: &str, limit: u32) -> String {
    let max_chars = (limit as usize) * 4;
    if text.len() <= max_chars {
        text.to_string()
    } else {
        text.chars().take(max_chars).collect()
    }
}

#[async_trait]
impl Tokenizer for TiktokenAdapter {
    async fn count_tokens(&self, text: &str, model: &str) -> Result<u32, TokenizerError> {
        #[cfg(feature = "tokenization")]
        {
            let text = text.to_owned();
            let model = model.to_owned();
            let fallback = self.fallback_model.to_owned();
            tokio::task::spawn_blocking(move || {
                use tiktoken_rs::get_bpe_from_model;

                let encoder = get_bpe_from_model(&model).or_else(|e| {
                    warn!(
                        model = %model,
                        fallback_model = %fallback,
                        error = %e,
                        "tokenizer fallback to cl100k_base"
                    );
                    get_bpe_from_model(&fallback)
                });

                match encoder {
                    Ok(bpe) => Ok(bpe.encode_ordinary(&text).len() as u32),
                    Err(e) => {
                        warn!(
                            model = %model,
                            error = %e,
                            "tokenizer unavailable; using heuristic estimate"
                        );
                        Ok(vibe_estimate_count(&text))
                    }
                }
            })
            .await
            .map_err(|e| TokenizerError::TokenizerUnavailable(e.to_string()))?
        }

        #[cfg(not(feature = "tokenization"))]
        {
            Ok(vibe_estimate_count(text))
        }
    }

    async fn truncate(
        &self,
        text: &str,
        limit: u32,
        model: &str,
    ) -> Result<String, TokenizerError> {
        #[cfg(feature = "tokenization")]
        {
            let text = text.to_owned();
            let model = model.to_owned();
            let fallback = self.fallback_model.to_owned();
            tokio::task::spawn_blocking(move || {
                use tiktoken_rs::get_bpe_from_model;

                let encoder = get_bpe_from_model(&model).or_else(|e| {
                    warn!(
                        model = %model,
                        fallback_model = %fallback,
                        error = %e,
                        "tokenizer fallback to cl100k_base"
                    );
                    get_bpe_from_model(&fallback)
                });

                let Ok(bpe) = encoder else {
                    return Ok(vibe_truncate(&text, limit));
                };

                let mut tokens = bpe.encode_ordinary(&text);
                if tokens.len() > limit as usize {
                    tokens.truncate(limit as usize);
                }

                bpe.decode(tokens)
                    .map_err(|e| TokenizerError::DecodeFailed(e.to_string()))
                    .or_else(|_| Ok(vibe_truncate(&text, limit)))
            })
            .await
            .map_err(|e| TokenizerError::TokenizerUnavailable(e.to_string()))?
        }

        #[cfg(not(feature = "tokenization"))]
        {
            Ok(vibe_truncate(text, limit))
        }
    }
}

/// Heuristic fallback tokenizer (char_count / 4).
#[derive(Debug, Clone, Copy)]
pub struct VibeTokenizer;

#[async_trait]
impl Tokenizer for VibeTokenizer {
    async fn count_tokens(&self, text: &str, _model: &str) -> Result<u32, TokenizerError> {
        Ok(vibe_estimate_count(text))
    }

    async fn truncate(
        &self,
        text: &str,
        limit: u32,
        _model: &str,
    ) -> Result<String, TokenizerError> {
        Ok(vibe_truncate(text, limit))
    }
}

/// Router that selects the appropriate tokenizer; defaults to fallback.
#[derive(Clone)]
pub struct TokenizationRouter {
    tiktoken: Arc<dyn Tokenizer>,
    fallback: Arc<dyn Tokenizer>,
}

impl TokenizationRouter {
    pub fn new(tiktoken: Arc<dyn Tokenizer>, fallback: Arc<dyn Tokenizer>) -> Self {
        Self { tiktoken, fallback }
    }

    pub fn with_defaults() -> Self {
        Self {
            tiktoken: Arc::new(TiktokenAdapter::default()),
            fallback: Arc::new(VibeTokenizer),
        }
    }

    async fn select(&self, model: &str) -> Arc<dyn Tokenizer> {
        if model.starts_with("gpt-") || model == "cl100k_base" {
            self.tiktoken.clone()
        } else {
            self.fallback.clone()
        }
    }

    pub async fn count_tokens(&self, text: &str, model: &str) -> Result<u32, TokenizerError> {
        let tokenizer = self.select(model).await;
        tokenizer.count_tokens(text, model).await
    }

    pub async fn truncate(
        &self,
        text: &str,
        limit: u32,
        model: &str,
    ) -> Result<String, TokenizerError> {
        let tokenizer = self.select(model).await;
        tokenizer.truncate(text, limit, model).await
    }
}

/// Budget helper that wraps token counting and truncation.
#[derive(Clone)]
pub struct TokenBudget {
    tokenizer: Arc<dyn Tokenizer>,
    pub max_context_tokens: u32,
}

impl TokenBudget {
    pub fn new(tokenizer: Arc<dyn Tokenizer>, max_context_tokens: u32) -> Self {
        Self {
            tokenizer,
            max_context_tokens,
        }
    }

    pub async fn count_prompt(&self, prompt: &str, model: &str) -> Result<u32, TokenizerError> {
        self.tokenizer.count_tokens(prompt, model).await
    }

    pub async fn truncate_to_fit(&self, text: &str, model: &str) -> Result<String, TokenizerError> {
        let current = self.tokenizer.count_tokens(text, model).await?;
        if current <= self.max_context_tokens {
            return Ok(text.to_string());
        }
        self.tokenizer
            .truncate(text, self.max_context_tokens, model)
            .await
    }

    pub fn remaining(&self, used: u32) -> Option<u32> {
        self.max_context_tokens.checked_sub(used)
    }
}

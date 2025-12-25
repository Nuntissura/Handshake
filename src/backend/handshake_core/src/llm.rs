use crate::tokenization::{TokenizationService, UnifiedTokenizationService};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Typed error for budget-related failures per protocol [CX-102]
#[derive(Debug, Clone)]
pub enum BudgetError {
    ExceedsLimit {
        prompt_tokens: u32,
        max_allowed: u32,
        budget: u32,
    },
}

impl fmt::Display for BudgetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BudgetError::ExceedsLimit {
                prompt_tokens,
                max_allowed,
                budget,
            } => {
                write!(
                    f,
                    "Prompt exceeds token budget: {} > {} (total budget: {})",
                    prompt_tokens, max_allowed, budget
                )
            }
        }
    }
}

#[async_trait]
pub trait LLMClient: Send + Sync {
    async fn chat(&self, messages: Vec<ChatMessage>) -> Result<String, String>;

    /// Chat with token budget enforcement per Master Spec §4.6.
    /// Ensures total tokens (prompt + response estimate) do not exceed the provided budget.
    async fn chat_with_budget(
        &self,
        messages: Vec<ChatMessage>,
        _token_budget: u32,
    ) -> Result<String, String> {
        // Default implementation: just call chat without budget enforcement
        // Implementers may override to enforce budget more strictly
        self.chat(messages).await
    }
}

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
            let error_text = match response.text().await {
                Ok(text) => text,
                Err(_) => String::new(),
            };
            let err_msg = format!("Ollama error ({}): {}", status, error_text);
            return Err(err_msg);
        }

        let chat_response: OllamaChatResponse = response.json().await.map_err(|e| e.to_string())?;

        Ok(chat_response.message.content)
    }

    async fn chat_with_budget(
        &self,
        messages: Vec<ChatMessage>,
        token_budget: u32,
    ) -> Result<String, String> {
        // Calculate prompt tokens using tokenization service
        let tokenizer = UnifiedTokenizationService::new();

        // Sum all message tokens
        let mut prompt_tokens = 0u32;
        for msg in &messages {
            match tokenizer.count_tokens(&msg.content, &self.model) {
                Ok(tokens) => prompt_tokens += tokens,
                Err(_) => {
                    // If tokenization fails, use fallback estimate
                    prompt_tokens += (msg.content.len() as u32 + 3) / 4;
                }
            }
        }

        // Check if prompt alone exceeds budget (need room for response)
        // Reserve 25% of budget for response
        let max_prompt_tokens = (token_budget * 3) / 4;
        if prompt_tokens > max_prompt_tokens {
            return Err(BudgetError::ExceedsLimit {
                prompt_tokens,
                max_allowed: max_prompt_tokens,
                budget: token_budget,
            }
            .to_string());
        }

        // Proceed with chat (response will be monitored by caller)
        self.chat(messages).await
    }
}

#[cfg(test)]
pub struct TestLLMClient {
    pub response: String,
}

#[cfg(test)]
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
        // In tests, just return the response without budget enforcement
        self.chat(messages).await
    }
}

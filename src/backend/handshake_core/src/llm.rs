use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[async_trait]
pub trait LLMClient: Send + Sync {
    async fn chat(&self, messages: Vec<ChatMessage>) -> Result<String, String>;
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
            .map_err(|e| format!("Request failed: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = match response.text().await {
                Ok(text) => text,
                Err(_) => String::new(),
            };
            return Err(format!("Ollama error ({}): {}", status, error_text));
        }

        let chat_response: OllamaChatResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        Ok(chat_response.message.content)
    }
}

pub struct MockLLMClient {
    pub response: String,
}

#[async_trait]
impl LLMClient for MockLLMClient {
    async fn chat(&self, _messages: Vec<ChatMessage>) -> Result<String, String> {
        Ok(self.response.clone())
    }
}

//! OpenAI-compatible HTTP adapter.
//!
//! This module intentionally keeps the surface minimal:
//! - Completes `LlmClient::completion(...)` via an OpenAI-compatible endpoint.
//! - Emits Flight Recorder `llm_inference` without raw prompts/payloads.
//! - Does not log or persist API keys.
//!
//! NOTE: Full request/response wiring is implemented after SKELETON approval.

use super::{CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile, ModelTier};
use async_trait::async_trait;
use std::fmt;
use std::sync::Arc;

use crate::flight_recorder::FlightRecorder;

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
    #[allow(dead_code)]
    client: reqwest::Client,
    #[allow(dead_code)]
    flight_recorder: Arc<dyn FlightRecorder>,
    #[allow(dead_code)]
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
        Self {
            base_url,
            profile: ModelProfile::new(model_id, max_context_tokens).with_tier(tier),
            client: reqwest::Client::new(),
            flight_recorder,
            api_key,
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

#[async_trait]
impl LlmClient for OpenAiCompatAdapter {
    async fn completion(&self, _req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        Err(LlmError::ProviderError(
            "HSK-501-UNSUPPORTED: OpenAI-compatible completion not implemented (skeleton)".to_string(),
        ))
    }

    fn profile(&self) -> &ModelProfile {
        &self.profile
    }
}


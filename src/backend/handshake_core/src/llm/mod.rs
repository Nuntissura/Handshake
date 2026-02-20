//! LLM Client Adapter [HSK-TRAIT-004]
//!
//! Per Master Spec §4.2.3: All application code MUST interact with LLMs
//! through the `LlmClient` trait. This ensures provider portability and
//! centralized observability via Flight Recorder.

pub mod guard;
pub mod ollama;
pub mod openai_compat;
pub mod registry;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;
use unicode_normalization::UnicodeNormalization;
use uuid::Uuid;

use crate::workflows::ModelSwapRequestV0_4;
use guard::CloudEscalationBundleV0_4;

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

    /// Swaps the active model in the underlying provider runtime (best-effort),
    /// honoring the Model Swap Protocol budgets and timeout.
    ///
    /// Default implementation returns an "unsupported" provider error so that
    /// non-Ollama clients can compile without implementing swap semantics.
    async fn swap_model(&self, _req: ModelSwapRequestV0_4) -> Result<(), LlmError> {
        Err(LlmError::ProviderError(
            "HSK-501-UNSUPPORTED: model swap unsupported".to_string(),
        ))
    }

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
    /// Cloud escalation consent bundle required for any outbound cloud invocation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cloud_escalation: Option<CloudEscalationBundleV0_4>,
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
            cloud_escalation: None,
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

/// Model deployment tier for security gating [§2.6.6.7.11.5].
///
/// CloudLeakageGuard only enforces leakage restrictions for Cloud tier models.
/// Local models are trusted and not subject to cloud export restrictions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModelTier {
    /// Local/on-premise model - no cloud leakage restrictions
    #[default]
    Local,
    /// Cloud-hosted model - subject to CloudLeakageGuard restrictions
    Cloud,
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
    /// Deployment tier for security gating [HSK-ACE-VAL-100]
    pub model_tier: ModelTier,
}

impl ModelProfile {
    /// Creates a new model profile.
    pub fn new(model_id: String, max_context_tokens: u32) -> Self {
        Self {
            model_id,
            max_context_tokens,
            supports_streaming: false,
            model_tier: ModelTier::Local,
        }
    }

    /// Builder: set streaming support.
    pub fn with_streaming(mut self, supports_streaming: bool) -> Self {
        self.supports_streaming = supports_streaming;
        self
    }

    /// Builder: set model tier for security gating.
    pub fn with_tier(mut self, tier: ModelTier) -> Self {
        self.model_tier = tier;
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

    /// HSK-400-INVALID-BASE-URL: Invalid/unparseable provider base_url configuration.
    #[error("HSK-400-INVALID-BASE-URL: Invalid base_url: {0}")]
    InvalidBaseUrl(String),

    /// HSK-403-SSRF-BLOCKED: base_url blocked by SSRF protections (Cloud tier).
    #[error("HSK-403-SSRF-BLOCKED: base_url blocked by SSRF guard: {0}")]
    SsrBlocked(String),

    /// HSK-403-GOVERNANCE-LOCKED: GovernanceMode LOCKED => cloud escalation denied.
    #[error("HSK-403-GOVERNANCE-LOCKED: GovernanceMode LOCKED; cloud escalation denied")]
    GovernanceLocked,

    /// HSK-403-CLOUD-ESCALATION-DENIED: Cloud escalation disallowed by runtime policy.
    #[error("HSK-403-CLOUD-ESCALATION-DENIED: Cloud escalation disallowed by policy")]
    CloudEscalationDenied,

    /// HSK-403-CLOUD-CONSENT-REQUIRED: Missing consent artifacts for cloud escalation.
    #[error("HSK-403-CLOUD-CONSENT-REQUIRED: Missing ProjectionPlan + ConsentReceipt")]
    CloudConsentRequired,

    /// HSK-403-CLOUD-CONSENT-MISMATCH: Consent artifacts do not bind or hash mismatch.
    #[error("HSK-403-CLOUD-CONSENT-MISMATCH: Consent artifacts invalid: {0}")]
    CloudConsentMismatch(String),

    /// HSK-500-LLM: Internal provider error.
    #[error("HSK-500-LLM: Internal provider error: {0}")]
    ProviderError(String),
}

/// LLM client used when the provider is unavailable at startup.
pub struct DisabledLlmClient {
    reason: String,
    profile: ModelProfile,
}

impl DisabledLlmClient {
    pub fn new(model_id: String, reason: String) -> Self {
        Self {
            reason,
            profile: ModelProfile::new(model_id, 0),
        }
    }
}

#[async_trait]
impl LlmClient for DisabledLlmClient {
    async fn completion(&self, _req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        Err(LlmError::ProviderError(self.reason.clone()))
    }

    fn profile(&self) -> &ModelProfile {
        &self.profile
    }
}

// =============================================================================
// Canonical JSON + Hashing Helpers (Spec §2.6.6.7.0)
// =============================================================================

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

pub(crate) fn canonical_json_bytes_nfc(value: &Value) -> Vec<u8> {
    let mut out = String::new();
    write_canonical_json_value_nfc(&mut out, value);
    out.into_bytes()
}

fn write_canonical_json_value_nfc(out: &mut String, value: &Value) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(v) => out.push_str(if *v { "true" } else { "false" }),
        Value::Number(num) => {
            if let Some(v) = num.as_i64() {
                out.push_str(&v.to_string());
            } else if let Some(v) = num.as_u64() {
                out.push_str(&v.to_string());
            } else if let Some(v) = num.as_f64() {
                // Spec §2.6.6.7.5: fixed float precision (recommend 6 decimals).
                let normalized = if v == 0.0 { 0.0 } else { v };
                out.push_str(&format!("{normalized:.6}"));
            } else {
                out.push_str(&num.to_string());
            }
        }
        Value::String(s) => write_canonical_json_string_nfc(out, s),
        Value::Array(items) => {
            out.push('[');
            for (idx, item) in items.iter().enumerate() {
                if idx > 0 {
                    out.push(',');
                }
                write_canonical_json_value_nfc(out, item);
            }
            out.push(']');
        }
        Value::Object(map) => {
            out.push('{');
            let mut keys: Vec<(&String, String)> = map
                .keys()
                .map(|key| (key, key.nfc().collect::<String>()))
                .collect();
            keys.sort_by(|(a_raw, a_norm), (b_raw, b_norm)| {
                a_norm.cmp(b_norm).then_with(|| a_raw.cmp(b_raw))
            });
            for (idx, (key, _)) in keys.iter().enumerate() {
                if idx > 0 {
                    out.push(',');
                }
                write_canonical_json_string_nfc(out, key);
                out.push(':');
                if let Some(v) = map.get(*key) {
                    write_canonical_json_value_nfc(out, v);
                } else {
                    out.push_str("null");
                }
            }
            out.push('}');
        }
    }
}

fn write_canonical_json_string_nfc(out: &mut String, value: &str) {
    let normalized: String = value.nfc().collect();
    out.push('"');
    for ch in normalized.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0C}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04X}", c as u32));
            }
            c if (c as u32) <= 0x7F => out.push(c),
            c if (c as u32) <= 0xFFFF => {
                out.push_str(&format!("\\u{:04X}", c as u32));
            }
            c => {
                let code = (c as u32) - 0x1_0000;
                let high = 0xD800 + ((code >> 10) & 0x3FF);
                let low = 0xDC00 + (code & 0x3FF);
                out.push_str(&format!("\\u{:04X}\\u{:04X}", high, low));
            }
        }
    }
    out.push('"');
}

pub(crate) fn openai_compat_chat_completion_body_json(
    req: &CompletionRequest,
    resolved_model_id: &str,
) -> Value {
    let mut map = serde_json::Map::new();
    map.insert(
        "model".to_string(),
        Value::String(resolved_model_id.to_string()),
    );
    map.insert(
        "messages".to_string(),
        Value::Array(vec![serde_json::json!({
            "role": "user",
            "content": req.prompt.clone(),
        })]),
    );
    if let Some(max_tokens) = req.max_tokens {
        map.insert("max_tokens".to_string(), serde_json::json!(max_tokens));
    }
    map.insert(
        "temperature".to_string(),
        serde_json::json!(req.temperature),
    );
    if !req.stop_sequences.is_empty() {
        map.insert(
            "stop".to_string(),
            serde_json::to_value(&req.stop_sequences).unwrap_or(Value::Array(Vec::new())),
        );
    }
    map.insert("stream".to_string(), Value::Bool(false));
    Value::Object(map)
}

pub(crate) fn openai_compat_canonical_request_bytes(
    req: &CompletionRequest,
    resolved_model_id: &str,
) -> Vec<u8> {
    canonical_json_bytes_nfc(&openai_compat_chat_completion_body_json(
        req,
        resolved_model_id,
    ))
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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

        let invalid_base_url = LlmError::InvalidBaseUrl("bad".to_string());
        assert_eq!(
            invalid_base_url.to_string(),
            "HSK-400-INVALID-BASE-URL: Invalid base_url: bad"
        );

        let ssrf = LlmError::SsrBlocked("http://127.0.0.1".to_string());
        assert_eq!(
            ssrf.to_string(),
            "HSK-403-SSRF-BLOCKED: base_url blocked by SSRF guard: http://127.0.0.1"
        );

        let locked = LlmError::GovernanceLocked;
        assert_eq!(
            locked.to_string(),
            "HSK-403-GOVERNANCE-LOCKED: GovernanceMode LOCKED; cloud escalation denied"
        );

        let denied = LlmError::CloudEscalationDenied;
        assert_eq!(
            denied.to_string(),
            "HSK-403-CLOUD-ESCALATION-DENIED: Cloud escalation disallowed by policy"
        );

        let consent_required = LlmError::CloudConsentRequired;
        assert_eq!(
            consent_required.to_string(),
            "HSK-403-CLOUD-CONSENT-REQUIRED: Missing ProjectionPlan + ConsentReceipt"
        );

        let mismatch = LlmError::CloudConsentMismatch("hash mismatch".to_string());
        assert_eq!(
            mismatch.to_string(),
            "HSK-403-CLOUD-CONSENT-MISMATCH: Consent artifacts invalid: hash mismatch"
        );

        let provider = LlmError::ProviderError("Connection timeout".to_string());
        assert_eq!(
            provider.to_string(),
            "HSK-500-LLM: Internal provider error: Connection timeout"
        );
    }

    #[test]
    fn canonical_json_bytes_nfc_normalizes_strings() {
        let input = format!("e\u{0301}");
        let value = json!({ "s": input });
        let bytes = canonical_json_bytes_nfc(&value);
        let rendered = String::from_utf8(bytes).expect("expected UTF-8 canonical JSON bytes");

        assert!(
            rendered.contains("\\u00E9"),
            "expected NFC normalization to compose e + combining acute to \\u00E9, got: {rendered}"
        );
        assert!(
            !rendered.contains("\\u0301"),
            "expected combining acute to be removed by NFC normalization, got: {rendered}"
        );
    }

    #[test]
    fn canonical_json_bytes_nfc_formats_floats_with_fixed_precision() {
        let value = json!({ "t": 0.7 });
        let bytes = canonical_json_bytes_nfc(&value);
        let rendered = String::from_utf8(bytes).expect("expected UTF-8 canonical JSON bytes");
        assert!(
            rendered.contains("0.700000"),
            "expected fixed 6-decimal float formatting, got: {rendered}"
        );
    }
}

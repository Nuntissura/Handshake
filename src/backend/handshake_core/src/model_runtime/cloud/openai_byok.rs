//! MT-125: Cloud lane BYOK OpenAI runtime.
//!
//! Implements `OpenAiByokRuntime` as a [`ModelRuntime`] adapter for the
//! BYOK cloud lane. Per HBR-INT-005 lane normalisation the adapter
//! exposes the same trait surface as the local adapters (load /
//! unload / generate / score / embed / capabilities / kv_cache /
//! lora_stack / steering_hooks / cancel). The wire format is the
//! OpenAI Chat Completions HTTP API with SSE streaming.
//!
//! Operationally dormant for the operator (no BYOK credits) but
//! architecturally required: every code path is exercised under
//! [`wiremock`] in the integration tests, which binds a real TCP
//! port answering the documented OpenAI protocol shape. That
//! satisfies Spec-Realism Gate sub-rule 2 (real external resource
//! touch: real reqwest -> real socket -> wiremock answering protocol
//! shape).
//!
//! Invariants enforced here:
//!
//! - The operator API key never leaves the [`ApiKeyProvider`]
//!   boundary in Debug/Display output; the struct holds an
//!   `Arc<dyn ApiKeyProvider>` and the secret is fetched on demand.
//! - Capabilities match BYOK cloud realities (no local LoRA, no
//!   activation steering, no subquadratic, no speculative draft;
//!   prompt caching is implicit, KV quantisation is server-side
//!   opaque) per HBR-INT-005 lane normalisation.
//! - Model name allowlist (prefix-match) gates `load()` so an
//!   un-approved OpenAI model cannot be invoked.
//! - Every call writes a [`CloudInvocationAuditRow`] through the
//!   [`CloudInvocationAuditSink`] trait (Started / Succeeded /
//!   Failed / Cancelled). Durable production sinks must append through the
//!   embedded SurrealDB/EventLedger authority; this runtime remains
//!   sink-agnostic and has no SQL compatibility path.
//! - No [`crate::process_ledger`] row is written: BYOK invocations
//!   do not spawn a Handshake-owned process, so there is no PID to
//!   register (per MT-066 ExternalCompat semantics extended to
//!   ByokCloud).

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, RwLock};
use std::task::{Context, Poll};
use std::time::Duration;

use async_trait::async_trait;
use futures::{stream, Stream, StreamExt};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use zeroize::Zeroizing;

use crate::flight_recorder::events_llm_infer::{
    infer_end_event, infer_start_event, infer_token_event, new_llm_infer_request_id,
    should_emit_token_event,
};
use crate::model_runtime::cloud::CloudLaneObservability;
use crate::model_runtime::{
    error::ModelRuntimeError, CancellationToken, Embedding, FinishReason, GenerateRequest,
    GeneratedToken, KvCacheHandle, LoadSpec, LoraStackHandle, ModelCapabilities, ModelId,
    ModelRuntime, ProviderKind, Score, SteeringHookHandle, TokenStream,
};

/// Allowlist of OpenAI model name prefixes the operator has approved
/// for BYOK cloud invocation. Defaults to the common Chat /
/// Completions / Responses families as of WP-KERNEL-004; operators
/// may extend with [`OpenAiByokRuntime::register_model_name`].
pub const DEFAULT_OPENAI_MODEL_ALLOWLIST: &[&str] = &[
    "gpt-4o",
    "gpt-4-turbo",
    "gpt-4.1",
    "o1",
    "o3",
    "gpt-3.5-turbo",
];

#[derive(Clone, Copy, Debug)]
pub struct ByokInvocationTimeouts {
    pub whole_invocation: Duration,
    pub connect: Duration,
    pub idle: Duration,
}

impl Default for ByokInvocationTimeouts {
    fn default() -> Self {
        Self {
            whole_invocation: Duration::from_secs(300),
            connect: Duration::from_secs(15),
            idle: Duration::from_secs(30),
        }
    }
}

/// Default chat-completions path appended to the runtime's
/// `api_base`. Exposed so tests can compare against the wiremock
/// expectation.
pub const OPENAI_CHAT_COMPLETIONS_PATH: &str = "/chat/completions";

/// Default embeddings path appended to the runtime's `api_base`.
pub const OPENAI_EMBEDDINGS_PATH: &str = "/embeddings";

/// SSE "DONE" sentinel that terminates an OpenAI streaming response.
const OPENAI_SSE_DONE_SENTINEL: &str = "[DONE]";

/// Provider-controlled error bodies can echo request data, including secrets,
/// and can be arbitrarily large. ModelRuntimeError receives only this bounded,
/// metadata-only diagnostic; raw provider body bytes are never surfaced.
const PROVIDER_ERROR_BODY_DIAGNOSTIC_MAX_BYTES: usize = 160;

fn redacted_provider_response_body(response: &reqwest::Response) -> String {
    let diagnostic = match response.content_length() {
        Some(declared_bytes) => {
            format!("<redacted provider response body; declared_bytes={declared_bytes}>")
        }
        None => "<redacted provider response body; declared_bytes=unknown>".to_string(),
    };
    debug_assert!(diagnostic.len() <= PROVIDER_ERROR_BODY_DIAGNOSTIC_MAX_BYTES);
    diagnostic
}

/// Opaque, zeroizing ownership for an operator-managed API key.
///
/// The inner allocation is never cloneable and Debug is always redacted.
pub struct TransientApiKey(Zeroizing<String>);

impl TransientApiKey {
    /// Construct a transient key at an external secret-ingress boundary.
    pub fn from_secret(secret: String) -> Self {
        Self(Zeroizing::new(secret))
    }

    pub(crate) fn from_zeroizing(secret: Zeroizing<String>) -> Self {
        Self(secret)
    }

    pub(crate) fn expose_secret(&self) -> &str {
        self.0.as_str()
    }
}

impl std::fmt::Debug for TransientApiKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("TransientApiKey([REDACTED])")
    }
}

pub trait ApiKeyProvider: Send + Sync {
    fn fetch_api_key(&self) -> Result<TransientApiKey, OpenAiByokError>;
}

/// Per-registered-model handle. Maps the Handshake `ModelId` (UUID
/// v7) to the OpenAI model name string used on the wire.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenAiModelHandle {
    pub model_id: ModelId,
    pub openai_model_name: String,
    pub registered_at_utc: String,
}

/// Audit row written for every cloud call per MT-125 red_team
/// minimum_controls. Durable production wiring belongs behind
/// [`CloudInvocationAuditSink`] and must use embedded SurrealDB/EventLedger;
/// the runtime itself is sink-agnostic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CloudInvocationAuditRow {
    pub model_id: ModelId,
    pub openai_model_name: String,
    pub call_kind: CloudCallKind,
    pub started_at_utc: String,
    pub finished_at_utc: Option<String>,
    pub status: CloudCallStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloudCallKind {
    ChatCompletion,
    Embeddings,
    Score,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloudCallStatus {
    Started,
    Succeeded,
    Failed,
    Cancelled,
}

pub trait CloudInvocationAuditSink: Send + Sync {
    fn record(&self, row: CloudInvocationAuditRow) -> Result<(), OpenAiByokError>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApiKeyFetchCode {
    VaultEmptyLaneId,
    VaultEmptySecretValue,
    VaultNoSecretForLane,
    VaultLockPoisoned,
    VaultKeychainBackend,
    ProviderFailure,
}

impl ApiKeyFetchCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::VaultEmptyLaneId => "vault_empty_lane_id",
            Self::VaultEmptySecretValue => "vault_empty_secret_value",
            Self::VaultNoSecretForLane => "vault_no_secret_for_lane",
            Self::VaultLockPoisoned => "vault_lock_poisoned",
            Self::VaultKeychainBackend => "vault_keychain_backend",
            Self::ProviderFailure => "provider_failure",
        }
    }
}

impl std::fmt::Display for ApiKeyFetchCode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProviderBodyMetadata {
    pub declared_bytes: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SseFailureKind {
    Framing,
    InvalidJson,
    MissingTerminal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderResponseKind {
    OpenAiChatCompletions,
    OpenAiEmbeddings,
    AnthropicMessages,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransportErrorCode {
    Timeout,
    Connect,
    Request,
    Body,
    Decode,
    Unknown,
}

impl TransportErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Timeout => "timeout",
            Self::Connect => "connect",
            Self::Request => "request",
            Self::Body => "body",
            Self::Decode => "decode",
            Self::Unknown => "unknown",
        }
    }
}

impl std::fmt::Display for TransportErrorCode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderOperation {
    Generate,
    Score,
    Embed,
}

impl std::fmt::Display for ProviderOperation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Generate => "generate",
            Self::Score => "score",
            Self::Embed => "embed",
        })
    }
}

fn classify_transport_error(error: &reqwest::Error) -> TransportErrorCode {
    if error.is_timeout() {
        TransportErrorCode::Timeout
    } else if error.is_connect() {
        TransportErrorCode::Connect
    } else if error.is_request() {
        TransportErrorCode::Request
    } else if error.is_body() {
        TransportErrorCode::Body
    } else if error.is_decode() {
        TransportErrorCode::Decode
    } else {
        TransportErrorCode::Unknown
    }
}

#[derive(Debug, Error)]
pub enum OpenAiByokError {
    #[error("OpenAI model name {0} is not in the BYOK allowlist (extend via register_model_name)")]
    ModelNameNotAllowed(String),
    #[error("OpenAI model name must not be empty")]
    EmptyModelName,
    #[error("model_id {0} is not registered with the BYOK runtime")]
    ModelNotRegistered(ModelId),
    #[error("API key fetch failed; code={code}")]
    ApiKeyFetch { code: ApiKeyFetchCode },
    #[error("audit row persistence failed: {0}")]
    AuditPersist(String),
    #[error("internal lock poisoned: {0}")]
    LockPoisoned(String),
    #[error("only ByokCloud provider is supported by OpenAiByokRuntime (got {0:?})")]
    ProviderKindNotSupported(ProviderKind),
    #[error("provider transport failed; operation={operation}; code={code}")]
    RequestFailed {
        operation: ProviderOperation,
        code: TransportErrorCode,
    },
    #[error("HTTP response status {status}; provider body redacted; metadata={metadata:?}")]
    HttpStatus {
        status: u16,
        metadata: ProviderBodyMetadata,
    },
    #[error("SSE stream failed; kind={kind:?}; payload_bytes={payload_bytes:?}")]
    StreamParseFailed {
        kind: SseFailureKind,
        payload_bytes: Option<usize>,
    },
    #[error("JSON processing failed; response_kind={response_kind:?}")]
    JsonFailed { response_kind: ProviderResponseKind },
    #[error("call cancelled before completion")]
    Cancelled,
}

/// JSON shape the runtime POSTs to `/chat/completions`.
#[derive(Debug, Serialize)]
struct ChatCompletionsRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    stop: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

/// JSON shape of a single SSE chunk emitted by OpenAI's
/// `/chat/completions?stream=true` endpoint.
#[derive(Debug, Deserialize)]
struct ChatStreamChunk {
    #[serde(default)]
    choices: Vec<ChatStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatStreamChoice {
    #[serde(default)]
    delta: ChatStreamDelta,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct ChatStreamDelta {
    #[serde(default)]
    content: Option<String>,
}

/// JSON shape of OpenAI's non-streaming `/chat/completions` response
/// (used by [`OpenAiByokRuntime::score`] which sets
/// `stream=false`+`logprobs=true`).
#[derive(Debug, Deserialize)]
struct ChatCompletionsResponse {
    #[serde(default)]
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    #[serde(default)]
    logprobs: Option<ChatChoiceLogprobs>,
}

#[derive(Debug, Deserialize)]
struct ChatChoiceLogprobs {
    #[serde(default)]
    content: Vec<ChatLogprobToken>,
}

#[derive(Debug, Deserialize)]
struct ChatLogprobToken {
    #[serde(default)]
    logprob: f32,
}

/// JSON shape the runtime POSTs to `/embeddings`.
#[derive(Debug, Serialize)]
struct EmbeddingsRequest<'a> {
    model: &'a str,
    input: &'a str,
}

#[derive(Debug, Deserialize)]
struct EmbeddingsResponse {
    #[serde(default)]
    data: Vec<EmbeddingsDatum>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingsDatum {
    #[serde(default)]
    embedding: Vec<f32>,
}

/// BYOK cloud runtime. Holds a `reqwest::Client` for live HTTP /
/// SSE invocation; the struct deliberately does NOT hold the API
/// key string — it holds an `Arc<dyn ApiKeyProvider>` so the
/// secret is fetched on demand from `OperatorSecretsVault` and
/// never serialised into the struct.
pub struct OpenAiByokRuntime {
    api_base: String,
    client: reqwest::Client,
    api_key_provider: Arc<dyn ApiKeyProvider>,
    audit_sink: Arc<dyn CloudInvocationAuditSink>,
    allowlist: RwLock<HashSet<String>>,
    models: RwLock<HashMap<ModelId, OpenAiModelHandle>>,
    declared_capabilities: ModelCapabilities,
    runtime_cancel: CancellationToken,
    timeouts: ByokInvocationTimeouts,
    /// MT-125 remediation: optional shared cloud-lane observability.
    /// When `Some`, the runtime (1) consults the [`ConsentGate`]
    /// before issuing the live HTTP call and (2) emits
    /// `FR-EVT-LLM-INFER-{START,TOKEN,END}` events through the
    /// [`FlightRecorder`] for HBR-INT-005 lane normalisation. When
    /// `None` the runtime preserves its exact prior behaviour
    /// (audit-rows only, no consent gate, no FR events).
    lane_obs: Option<Arc<CloudLaneObservability>>,
}

impl std::fmt::Debug for OpenAiByokRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // GLOBAL: secret material is never surfaced in Debug. The
        // api_key_provider is shown as a placeholder.
        f.debug_struct("OpenAiByokRuntime")
            .field("api_base", &self.api_base)
            .field("api_key_provider", &"<redacted Arc<dyn ApiKeyProvider>>")
            .field("audit_sink", &"<Arc<dyn CloudInvocationAuditSink>>")
            .field("models", &self.models.read().map(|m| m.len()).unwrap_or(0))
            .finish()
    }
}

impl OpenAiByokRuntime {
    pub fn new(
        api_base: impl Into<String>,
        api_key_provider: Arc<dyn ApiKeyProvider>,
        audit_sink: Arc<dyn CloudInvocationAuditSink>,
    ) -> Self {
        let timeouts = ByokInvocationTimeouts::default();
        Self::with_client(
            api_base,
            reqwest::Client::builder()
                .connect_timeout(timeouts.connect)
                .build()
                .expect("build bounded OpenAI BYOK HTTP client"),
            api_key_provider,
            audit_sink,
        )
    }

    /// Like [`Self::new`] but lets the caller inject a configured
    /// `reqwest::Client` (test fixtures use this to bound timeouts).
    pub fn with_client(
        api_base: impl Into<String>,
        client: reqwest::Client,
        api_key_provider: Arc<dyn ApiKeyProvider>,
        audit_sink: Arc<dyn CloudInvocationAuditSink>,
    ) -> Self {
        Self {
            api_base: api_base.into(),
            client,
            api_key_provider,
            audit_sink,
            allowlist: RwLock::new(
                DEFAULT_OPENAI_MODEL_ALLOWLIST
                    .iter()
                    .map(|s| (*s).to_string())
                    .collect(),
            ),
            models: RwLock::new(HashMap::new()),
            declared_capabilities: Self::cloud_capabilities(),
            runtime_cancel: CancellationToken::new(),
            timeouts: ByokInvocationTimeouts::default(),
            lane_obs: None,
        }
    }

    pub fn with_timeouts(mut self, timeouts: ByokInvocationTimeouts) -> Self {
        assert!(!timeouts.whole_invocation.is_zero());
        assert!(!timeouts.connect.is_zero());
        assert!(!timeouts.idle.is_zero());
        self.timeouts = timeouts;
        self
    }

    /// MT-125 remediation: attach a shared
    /// [`CloudLaneObservability`] bundle so the runtime emits
    /// `FR-EVT-LLM-INFER-{START,TOKEN,END}` events and enforces the
    /// operator consent gate before live HTTP calls. Builder-style so
    /// existing construction sites that do not opt in are unaffected.
    pub fn with_lane_observability(mut self, lane_obs: Arc<CloudLaneObservability>) -> Self {
        self.lane_obs = Some(lane_obs);
        self
    }

    /// Extend the allowlist with an additional OpenAI model-name prefix.
    pub fn register_model_name(&self, model_name: &str) -> Result<(), OpenAiByokError> {
        if model_name.trim().is_empty() {
            return Err(OpenAiByokError::EmptyModelName);
        }
        let mut guard = self
            .allowlist
            .write()
            .map_err(|err| OpenAiByokError::LockPoisoned(err.to_string()))?;
        guard.insert(model_name.to_string());
        Ok(())
    }

    /// Register a model handle. Validates the requested OpenAI model
    /// name against the allowlist (prefix-match; supports family
    /// allowlisting like `gpt-4o`).
    pub fn register_handle(
        &self,
        openai_model_name: &str,
        now_utc: &str,
    ) -> Result<OpenAiModelHandle, OpenAiByokError> {
        if openai_model_name.trim().is_empty() {
            return Err(OpenAiByokError::EmptyModelName);
        }
        let allowed = {
            let guard = self
                .allowlist
                .read()
                .map_err(|err| OpenAiByokError::LockPoisoned(err.to_string()))?;
            guard
                .iter()
                .any(|prefix| openai_model_name.starts_with(prefix))
        };
        if !allowed {
            return Err(OpenAiByokError::ModelNameNotAllowed(
                openai_model_name.to_string(),
            ));
        }
        let model_id = ModelId::new_v7();
        let handle = OpenAiModelHandle {
            model_id,
            openai_model_name: openai_model_name.to_string(),
            registered_at_utc: now_utc.to_string(),
        };
        let mut models = self
            .models
            .write()
            .map_err(|err| OpenAiByokError::LockPoisoned(err.to_string()))?;
        models.insert(model_id, handle.clone());
        // Audit the registration as a Started lifecycle row so the
        // operator can correlate which model_ids the kernel attached
        // to the BYOK lane and when.
        self.audit_sink.record(CloudInvocationAuditRow {
            model_id,
            openai_model_name: openai_model_name.to_string(),
            call_kind: CloudCallKind::ChatCompletion,
            started_at_utc: now_utc.to_string(),
            finished_at_utc: None,
            status: CloudCallStatus::Started,
        })?;
        Ok(handle)
    }

    /// Capability declaration for BYOK cloud lane per HBR-INT-005.
    /// Lane realities (server-side opaque): no local LoRA mounting,
    /// no activation steering, no subquadratic; KV prefix cache is
    /// implicit via OpenAI's prompt caching but the kernel cannot
    /// inspect or control quantisation level.
    pub fn cloud_capabilities() -> ModelCapabilities {
        ModelCapabilities {
            supports_lora: false,
            supports_kv_prefix_cache: true,
            supports_kv_quantization: crate::model_runtime::KvQuantSupport::None,
            supports_activation_steering: false,
            supports_subquadratic: false,
            supports_speculative_draft: false,
            supports_eagle3: false,
            ..Default::default()
        }
    }

    /// Look up a registered handle.
    pub fn handle_for(&self, model_id: ModelId) -> Result<OpenAiModelHandle, OpenAiByokError> {
        let models = self
            .models
            .read()
            .map_err(|err| OpenAiByokError::LockPoisoned(err.to_string()))?;
        models
            .get(&model_id)
            .cloned()
            .ok_or(OpenAiByokError::ModelNotRegistered(model_id))
    }

    /// Fetch the secret into zeroizing ownership. The runtime never exposes a
    /// plain owned key after this boundary.
    pub fn fetch_api_key(&self) -> Result<TransientApiKey, OpenAiByokError> {
        self.api_key_provider
            .fetch_api_key()
            .map_err(|err| OpenAiByokError::ApiKeyFetch {
                code: match err {
                    OpenAiByokError::ApiKeyFetch { code } => code,
                    _ => ApiKeyFetchCode::ProviderFailure,
                },
            })
    }

    /// Records an audit row through the sink. Tests use this to
    /// inject lifecycle rows without bringing up the HTTP client;
    /// the live HTTP path emits rows through the same sink.
    pub fn record_audit(&self, row: CloudInvocationAuditRow) -> Result<(), OpenAiByokError> {
        self.audit_sink.record(row)
    }

    /// Live streaming call to OpenAI's Chat Completions endpoint
    /// with `stream=true`. Returns a [`TokenStream`] that yields a
    /// [`GeneratedToken`] per SSE `data:` chunk's
    /// `choices[].delta.content`, terminating on the SSE `[DONE]`
    /// sentinel.
    ///
    /// Audit rows are written at the start (status=Started) and at
    /// the end (Succeeded / Cancelled / Failed) through the runtime's
    /// configured [`CloudInvocationAuditSink`].
    pub fn chat_completions_stream(&self, req: GenerateRequest) -> TokenStream {
        let handle = match self.handle_for(req.id) {
            Ok(handle) => handle,
            Err(err) => {
                return single_error_stream(ModelRuntimeError::GenerateError(format!("{err}")));
            }
        };

        let api_key = match self.fetch_api_key() {
            Ok(key) => key,
            Err(err) => {
                return single_error_stream(ModelRuntimeError::GenerateError(format!("{err}")));
            }
        };

        // MT-125 remediation: per-session/per-lane consent gate. When
        // an observability bundle with a consent context is attached,
        // the operator must have consented (or consent now via the
        // provider) before any cloud bytes leave the process. On
        // denial we surface a GenerateError and issue NO HTTP request.
        if let Some(lane_obs) = self.lane_obs.as_ref() {
            if let Some(consent) = lane_obs.consent.as_ref() {
                if let Err(err) = consent.gate.check_or_prompt(
                    &consent.session_id,
                    self.adapter_name(),
                    consent.provider.as_ref(),
                ) {
                    return single_error_stream(ModelRuntimeError::GenerateError(format!(
                        "OpenAI BYOK cloud consent denied: {err}"
                    )));
                }
            }
        }

        let url = format!("{}{}", self.api_base, OPENAI_CHAT_COMPLETIONS_PATH);
        let body = ChatCompletionsRequest {
            model: &handle.openai_model_name,
            messages: vec![ChatMessage {
                role: "user",
                content: req.prompt.as_str(),
            }],
            stream: true,
            max_tokens: Some(req.max_tokens),
            temperature: req.sampling.temperature,
            top_p: req.sampling.top_p,
            frequency_penalty: req.sampling.frequency_penalty,
            presence_penalty: req.sampling.presence_penalty,
            stop: req.stop_sequences.clone(),
        };

        let body_json = match serde_json::to_vec(&body) {
            Ok(bytes) => bytes,
            Err(err) => {
                return single_error_stream(ModelRuntimeError::GenerateError(format!(
                    "OpenAI BYOK request serialisation failed: {err}"
                )));
            }
        };

        let started_at_utc = chrono::Utc::now().to_rfc3339();
        if let Err(err) = self.audit_sink.record(CloudInvocationAuditRow {
            model_id: handle.model_id,
            openai_model_name: handle.openai_model_name.clone(),
            call_kind: CloudCallKind::ChatCompletion,
            started_at_utc: started_at_utc.clone(),
            finished_at_utc: None,
            status: CloudCallStatus::Started,
        }) {
            return single_error_stream(ModelRuntimeError::GenerateError(format!("{err}")));
        }

        let client = self.client.clone();
        let cancel_req = req.cancel.clone();
        let cancel_runtime = self.runtime_cancel.clone();
        let audit_sink = self.audit_sink.clone();
        let model_id = handle.model_id;
        let openai_model_name = handle.openai_model_name.clone();

        // MT-125 remediation: thread the FlightRecorder (if attached)
        // + a freshly-minted request id + a best-effort prompt-token
        // estimate into the async pipeline so it can emit
        // `FR-EVT-LLM-INFER-{START,TOKEN,END}` events. When no
        // observability bundle is attached, `flight_recorder` is None
        // and the pipeline emits NOTHING (exact prior behaviour).
        let flight_recorder = self
            .lane_obs
            .as_ref()
            .map(|obs| obs.flight_recorder.clone());
        let request_id = new_llm_infer_request_id();
        // Best-effort prompt-token estimate: whitespace-split word
        // count. The FR-event field need not be exact (the recorder
        // does not constrain it for cloud lanes); a server-side exact
        // count is not available before the response.
        let prompt_tokens_estimate = req.prompt.as_str().split_whitespace().count() as u64;

        // The reqwest+SSE pipeline runs inside a spawned task and
        // pumps `GeneratedToken`s onto an mpsc channel; the
        // TokenStream the caller sees is built from the receiver.
        // The pipeline emits one GeneratedToken per
        // `choices[0].delta.content` and terminates on the SSE
        // `[DONE]` sentinel or either cancellation token.
        async_token_stream(
            client,
            url,
            api_key,
            body_json,
            cancel_req,
            cancel_runtime,
            audit_sink,
            CloudInvocationAuditRow {
                model_id,
                openai_model_name,
                call_kind: CloudCallKind::ChatCompletion,
                started_at_utc,
                finished_at_utc: None,
                status: CloudCallStatus::Started,
            },
            LaneFrContext {
                flight_recorder,
                model_id,
                request_id,
                prompt_tokens_estimate,
            },
            self.timeouts,
        )
    }
}

/// MT-125 remediation: bundle of the FlightRecorder emission inputs
/// threaded into the async streaming pipeline. When `flight_recorder`
/// is `None` the pipeline emits no FR events (preserving exact prior
/// behaviour); when `Some` it emits one START, sampled TOKENs, and one
/// END with `adapter == "openai_byok"`.
struct LaneFrContext {
    flight_recorder: Option<Arc<dyn crate::flight_recorder::FlightRecorder>>,
    model_id: ModelId,
    request_id: uuid::Uuid,
    prompt_tokens_estimate: u64,
}

/// Returns a one-shot stream that yields a single error item. Used
/// to surface preflight failures (handle-not-registered, api-key-
/// fetch-failed) inside the [`TokenStream`] contract.
fn single_error_stream(err: ModelRuntimeError) -> TokenStream {
    Box::pin(stream::iter([Err(err)]))
}

/// Drives the live reqwest -> SSE pipeline and pumps
/// [`GeneratedToken`]s to a tokio mpsc channel; the [`TokenStream`]
/// the caller sees is built from that channel via `stream::unfold`.
/// Records the final audit row on completion.
///
/// This shape (channel + unfold) matches the llama.cpp adapter's
/// streaming path. We don't use `async-stream` because it isn't a
/// declared dep of the crate.
#[allow(clippy::too_many_arguments)]
fn async_token_stream(
    client: reqwest::Client,
    url: String,
    api_key: TransientApiKey,
    body_json: Vec<u8>,
    cancel_req: CancellationToken,
    cancel_runtime: CancellationToken,
    audit_sink: Arc<dyn CloudInvocationAuditSink>,
    audit_template: CloudInvocationAuditRow,
    fr_ctx: LaneFrContext,
    timeouts: ByokInvocationTimeouts,
) -> Pin<Box<dyn Stream<Item = Result<GeneratedToken, ModelRuntimeError>> + Send>> {
    let (sender, receiver) =
        tokio::sync::mpsc::unbounded_channel::<Result<GeneratedToken, ModelRuntimeError>>();
    let owner_cancel = CancellationToken::new();
    let task = tokio::spawn(run_live_stream(
        client,
        url,
        api_key,
        body_json,
        cancel_req,
        cancel_runtime,
        audit_sink,
        audit_template,
        fr_ctx,
        sender,
        owner_cancel.clone(),
        timeouts,
    ));
    Box::pin(OwnedCloudTokenStream {
        receiver,
        task: Some(task),
        owner_cancel,
    })
}

struct OwnedCloudTokenStream {
    receiver: tokio::sync::mpsc::UnboundedReceiver<Result<GeneratedToken, ModelRuntimeError>>,
    task: Option<tokio::task::JoinHandle<()>>,
    owner_cancel: CancellationToken,
}

enum ProviderWaitFailure {
    Cancelled,
    Deadline,
}

async fn await_provider_step<F, T>(
    future: F,
    cancel_req: &CancellationToken,
    cancel_runtime: &CancellationToken,
    owner_cancel: &CancellationToken,
    deadline: tokio::time::Instant,
) -> Result<T, ProviderWaitFailure>
where
    F: Future<Output = T>,
{
    tokio::select! {
        biased;
        _ = cancel_req.cancelled() => Err(ProviderWaitFailure::Cancelled),
        _ = cancel_runtime.cancelled() => Err(ProviderWaitFailure::Cancelled),
        _ = owner_cancel.cancelled() => Err(ProviderWaitFailure::Cancelled),
        result = tokio::time::timeout_at(deadline, future) => {
            result.map_err(|_| ProviderWaitFailure::Deadline)
        }
    }
}

async fn await_runtime_step<F, T>(
    future: F,
    cancel_runtime: &CancellationToken,
    deadline: tokio::time::Instant,
) -> Result<T, ProviderWaitFailure>
where
    F: Future<Output = T>,
{
    tokio::select! {
        biased;
        _ = cancel_runtime.cancelled() => Err(ProviderWaitFailure::Cancelled),
        result = tokio::time::timeout_at(deadline, future) => {
            result.map_err(|_| ProviderWaitFailure::Deadline)
        }
    }
}

impl Stream for OwnedCloudTokenStream {
    type Item = Result<GeneratedToken, ModelRuntimeError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.receiver.poll_recv(cx)
    }
}

impl Drop for OwnedCloudTokenStream {
    fn drop(&mut self) {
        self.owner_cancel.cancel();
        let Some(mut task) = self.task.take() else {
            return;
        };
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            runtime.spawn(async move {
                if tokio::time::timeout(Duration::from_millis(250), &mut task)
                    .await
                    .is_err()
                {
                    task.abort();
                    let _ = task.await;
                }
            });
        } else {
            task.abort();
        }
    }
}

/// Async driver. Sends one `Result<GeneratedToken, _>` per
/// SSE chunk into `sender`, terminates on `[DONE]` / cancellation /
/// HTTP error.
#[allow(clippy::too_many_arguments)]
async fn run_live_stream(
    client: reqwest::Client,
    url: String,
    api_key: TransientApiKey,
    body_json: Vec<u8>,
    cancel_req: CancellationToken,
    cancel_runtime: CancellationToken,
    audit_sink: Arc<dyn CloudInvocationAuditSink>,
    audit_template: CloudInvocationAuditRow,
    fr_ctx: LaneFrContext,
    sender: tokio::sync::mpsc::UnboundedSender<Result<GeneratedToken, ModelRuntimeError>>,
    owner_cancel: CancellationToken,
    timeouts: ByokInvocationTimeouts,
) {
    use eventsource_stream::Eventsource;

    let start_instant = std::time::Instant::now();
    let invocation_deadline = tokio::time::Instant::now() + timeouts.whole_invocation;
    let mut last_token_instant = start_instant;

    // MT-125 remediation: emit FR-EVT-LLM-INFER-START once at the
    // start of the pipeline. Recorder failures are logged and ignored
    // — they must never abort generation (HBR-INT-005 observability is
    // best-effort relative to the live call).
    emit_fr_event(
        &fr_ctx,
        infer_start_event(
            fr_ctx.model_id,
            fr_ctx.request_id,
            fr_ctx.prompt_tokens_estimate,
            "",
            "openai_byok",
        ),
    )
    .await;

    // Pre-call cancellation: terminate without issuing the
    // request, audit as Cancelled.
    if cancel_req.is_cancelled() || cancel_runtime.is_cancelled() {
        record_final_audit(&audit_sink, &audit_template, CloudCallStatus::Cancelled);
        emit_fr_end(&fr_ctx, 0, &start_instant, FinishReason::Cancelled).await;
        let _ = sender.send(Ok(terminal_token(FinishReason::Cancelled)));
        return;
    }

    let connect_deadline = invocation_deadline.min(tokio::time::Instant::now() + timeouts.connect);
    let request = client
        .post(&url)
        .bearer_auth(api_key.expose_secret())
        .header("Content-Type", "application/json")
        .body(body_json);
    drop(api_key);
    let response = match await_provider_step(
        request.send(),
        &cancel_req,
        &cancel_runtime,
        &owner_cancel,
        connect_deadline,
    )
    .await
    {
        Ok(Ok(resp)) => resp,
        Ok(Err(err)) => {
            let code = classify_transport_error(&err);
            record_final_audit(&audit_sink, &audit_template, CloudCallStatus::Failed);
            emit_fr_end(&fr_ctx, 0, &start_instant, FinishReason::Error).await;
            let _ = sender.send(Err(ModelRuntimeError::GenerateError(format!(
                "OpenAI BYOK transport failed; operation=generate; code={code}"
            ))));
            return;
        }
        Err(ProviderWaitFailure::Cancelled) => {
            record_final_audit(&audit_sink, &audit_template, CloudCallStatus::Cancelled);
            emit_fr_end(&fr_ctx, 0, &start_instant, FinishReason::Cancelled).await;
            let _ = sender.send(Ok(terminal_token(FinishReason::Cancelled)));
            return;
        }
        Err(ProviderWaitFailure::Deadline) => {
            record_final_audit(&audit_sink, &audit_template, CloudCallStatus::Failed);
            emit_fr_end(&fr_ctx, 0, &start_instant, FinishReason::Error).await;
            let _ = sender.send(Err(ModelRuntimeError::GenerateError(
                "OpenAI BYOK connect/request deadline elapsed".to_string(),
            )));
            return;
        }
    };

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = redacted_provider_response_body(&response);
        record_final_audit(&audit_sink, &audit_template, CloudCallStatus::Failed);
        emit_fr_end(&fr_ctx, 0, &start_instant, FinishReason::Error).await;
        let _ = sender.send(Err(ModelRuntimeError::GenerateError(format!(
            "OpenAI BYOK HTTP {status}: {body}"
        ))));
        return;
    }

    let mut sse = response.bytes_stream().eventsource();
    let mut token_index: u32 = 0;
    let mut hit_done = false;
    let mut pending_finish: Option<FinishReason> = None;
    let mut saw_usable_result = false;

    loop {
        let idle_deadline = invocation_deadline.min(tokio::time::Instant::now() + timeouts.idle);
        let next_event = await_provider_step(
            sse.next(),
            &cancel_req,
            &cancel_runtime,
            &owner_cancel,
            idle_deadline,
        )
        .await;
        let event = match next_event {
            Ok(Some(event)) => event,
            Ok(None) => break,
            Err(ProviderWaitFailure::Cancelled) => {
                record_final_audit(&audit_sink, &audit_template, CloudCallStatus::Cancelled);
                emit_fr_end(
                    &fr_ctx,
                    token_index,
                    &start_instant,
                    FinishReason::Cancelled,
                )
                .await;
                let _ = sender.send(Ok(terminal_token(FinishReason::Cancelled)));
                return;
            }
            Err(ProviderWaitFailure::Deadline) => {
                let detail = if tokio::time::Instant::now() >= invocation_deadline {
                    "whole invocation"
                } else {
                    "SSE idle"
                };
                record_final_audit(&audit_sink, &audit_template, CloudCallStatus::Failed);
                emit_fr_end(&fr_ctx, token_index, &start_instant, FinishReason::Error).await;
                let _ = sender.send(Err(ModelRuntimeError::GenerateError(format!(
                    "OpenAI BYOK {detail} deadline elapsed"
                ))));
                return;
            }
        };

        let event = match event {
            Ok(event) => event,
            Err(err) => {
                let _ = err;
                record_final_audit(&audit_sink, &audit_template, CloudCallStatus::Failed);
                emit_fr_end(&fr_ctx, token_index, &start_instant, FinishReason::Error).await;
                let _ = sender.send(Err(ModelRuntimeError::GenerateError(
                    "OpenAI BYOK SSE framing failure; event_kind=unavailable; payload_bytes=unavailable; provider_payload=<redacted>"
                        .to_string(),
                )));
                return;
            }
        };

        if event.data.trim() == OPENAI_SSE_DONE_SENTINEL {
            hit_done = true;
            break;
        }

        let chunk: ChatStreamChunk = match serde_json::from_str(&event.data) {
            Ok(chunk) => chunk,
            Err(err) => {
                let _ = err;
                let payload_bytes = event.data.len();
                record_final_audit(&audit_sink, &audit_template, CloudCallStatus::Failed);
                emit_fr_end(&fr_ctx, token_index, &start_instant, FinishReason::Error).await;
                let _ = sender.send(Err(ModelRuntimeError::GenerateError(format!(
                    "OpenAI BYOK SSE JSON parse failure; event_kind=chat_chunk; payload_bytes={payload_bytes}; provider_payload=<redacted>"
                ))));
                return;
            }
        };

        if chunk.choices.is_empty() {
            record_final_audit(&audit_sink, &audit_template, CloudCallStatus::Failed);
            emit_fr_end(&fr_ctx, token_index, &start_instant, FinishReason::Error).await;
            let _ = sender.send(Err(ModelRuntimeError::GenerateError(format!(
                "OpenAI BYOK SSE response unusable; event_kind=chat_chunk; reason=empty_choices; payload_bytes={}; provider_payload=<redacted>",
                event.data.len()
            ))));
            return;
        }

        if let Some(choice) = chunk.choices.into_iter().next() {
            let finish_mapped = choice.finish_reason.as_deref().and_then(map_finish_reason);
            if let Some(finish) = finish_mapped {
                pending_finish = Some(finish);
            }
            if let Some(text) = choice.delta.content {
                if !text.is_empty() {
                    saw_usable_result = true;
                    token_index = token_index.saturating_add(1);
                    // MT-125 remediation: emit a sampled
                    // FR-EVT-LLM-INFER-TOKEN. token_id is 0 for the
                    // cloud lane (no local tokenizer ids); latency is
                    // best-effort inter-token wall time.
                    let now = std::time::Instant::now();
                    let latency_ms = now.duration_since(last_token_instant).as_millis() as u64;
                    last_token_instant = now;
                    if should_emit_token_event(token_index) {
                        emit_fr_event(
                            &fr_ctx,
                            infer_token_event(
                                fr_ctx.model_id,
                                fr_ctx.request_id,
                                token_index,
                                0,
                                &text,
                                latency_ms,
                                "openai_byok",
                            ),
                        )
                        .await;
                    }
                    if sender
                        .send(Ok(GeneratedToken {
                            token_id: token_index,
                            text,
                            logprob: None,
                            finish_reason: None,
                        }))
                        .is_err()
                    {
                        // Receiver dropped — caller no longer cares.
                        record_final_audit(
                            &audit_sink,
                            &audit_template,
                            CloudCallStatus::Cancelled,
                        );
                        emit_fr_end(
                            &fr_ctx,
                            token_index,
                            &start_instant,
                            FinishReason::Cancelled,
                        )
                        .await;
                        return;
                    }
                }
            }
        }
    }

    if !hit_done {
        record_final_audit(&audit_sink, &audit_template, CloudCallStatus::Failed);
        emit_fr_end(&fr_ctx, token_index, &start_instant, FinishReason::Error).await;
        let _ = sender.send(Err(ModelRuntimeError::GenerateError(
            "OpenAI BYOK SSE terminal missing; event_kind=done; provider_payload=<redacted>"
                .to_string(),
        )));
        return;
    }
    if !saw_usable_result {
        record_final_audit(&audit_sink, &audit_template, CloudCallStatus::Failed);
        emit_fr_end(&fr_ctx, token_index, &start_instant, FinishReason::Error).await;
        let _ = sender.send(Err(ModelRuntimeError::GenerateError(
            "OpenAI BYOK SSE response unusable; event_kind=done; reason=no_usable_result; provider_payload=<redacted>"
                .to_string(),
        )));
        return;
    }
    record_final_audit(&audit_sink, &audit_template, CloudCallStatus::Succeeded);
    let finish = pending_finish.unwrap_or(FinishReason::Stop);
    emit_fr_end(&fr_ctx, token_index, &start_instant, finish).await;
    let _ = sender.send(Ok(terminal_token(finish)));
}

/// MT-125 remediation: emit a single FR event through the attached
/// recorder. When no recorder is attached this is a no-op (exact prior
/// behaviour). Recorder failures are logged and swallowed — observ-
/// ability must never abort or fail the live generation.
async fn emit_fr_event(fr_ctx: &LaneFrContext, event: crate::flight_recorder::FlightRecorderEvent) {
    if let Some(recorder) = fr_ctx.flight_recorder.as_ref() {
        if let Err(err) = recorder.record_event(event).await {
            tracing::warn!(
                target: "handshake_core::model_runtime::cloud::openai_byok",
                error = %err,
                "OpenAI BYOK FR-EVT-LLM-INFER emit failed; generation unaffected"
            );
        }
    }
}

/// MT-125 remediation: emit the closing FR-EVT-LLM-INFER-END with
/// best-effort timings. `prompt_eval_ms` is reported as 0 (the cloud
/// lane has no separate prompt-eval phase visible to the kernel);
/// `gen_eval_ms` is approximated by total wall time.
async fn emit_fr_end(
    fr_ctx: &LaneFrContext,
    tokens_generated: u32,
    start_instant: &std::time::Instant,
    finish_reason: FinishReason,
) {
    let total_ms = start_instant.elapsed().as_millis() as u64;
    emit_fr_event(
        fr_ctx,
        infer_end_event(
            fr_ctx.model_id,
            fr_ctx.request_id,
            fr_ctx.prompt_tokens_estimate,
            tokens_generated,
            total_ms,
            0,
            total_ms,
            finish_reason,
            "openai_byok",
        ),
    )
    .await;
}

/// Records the closing audit row for a streaming call. Failure to
/// persist is logged via `tracing` but does not break the caller's
/// stream because the call itself already finished or failed.
fn record_final_audit(
    audit_sink: &Arc<dyn CloudInvocationAuditSink>,
    template: &CloudInvocationAuditRow,
    status: CloudCallStatus,
) {
    let finished_at_utc = chrono::Utc::now().to_rfc3339();
    let row = CloudInvocationAuditRow {
        model_id: template.model_id,
        openai_model_name: template.openai_model_name.clone(),
        call_kind: template.call_kind,
        started_at_utc: template.started_at_utc.clone(),
        finished_at_utc: Some(finished_at_utc),
        status,
    };
    if let Err(err) = audit_sink.record(row) {
        tracing::warn!(
            target: "handshake_core::model_runtime::cloud::openai_byok",
            error = %err,
            "OpenAI BYOK final audit row failed to persist; call already completed"
        );
    }
}

fn map_finish_reason(value: &str) -> Option<FinishReason> {
    match value {
        "stop" => Some(FinishReason::Stop),
        "length" => Some(FinishReason::Length),
        "content_filter" | "tool_calls" | "function_call" => Some(FinishReason::Stop),
        "cancelled" => Some(FinishReason::Cancelled),
        _ => None,
    }
}

fn terminal_token(reason: FinishReason) -> GeneratedToken {
    GeneratedToken {
        token_id: 0,
        text: String::new(),
        logprob: None,
        finish_reason: Some(reason),
    }
}

#[async_trait]
impl ModelRuntime for OpenAiByokRuntime {
    fn adapter_name(&self) -> &'static str {
        "openai_byok"
    }

    async fn load(&mut self, spec: LoadSpec) -> Result<ModelId, ModelRuntimeError> {
        if spec.provider != ProviderKind::ByokCloud {
            return Err(ModelRuntimeError::LoadError(format!(
                "OpenAiByokRuntime requires provider=ByokCloud (got {:?})",
                spec.provider
            )));
        }
        let model_name = spec.engine_origin.as_deref().ok_or_else(|| {
            ModelRuntimeError::LoadError(
                "OpenAiByokRuntime.load requires LoadSpec::engine_origin = OpenAI model name"
                    .to_string(),
            )
        })?;
        let now_utc = chrono::Utc::now().to_rfc3339();
        let handle = self
            .register_handle(model_name, &now_utc)
            .map_err(|err| ModelRuntimeError::LoadError(format!("{err}")))?;
        // The runtime's declared capabilities snapshot is updated
        // from the LoadSpec's declared_capabilities so adapter
        // consumers see the BYOK invariants regardless of what the
        // operator hinted at registration time. We intersect with
        // cloud_capabilities() so a spec accidentally claiming
        // supports_lora=true gets normalised back to false.
        let normalised = ModelCapabilities {
            supports_lora: false,
            supports_kv_prefix_cache: spec.declared_capabilities.supports_kv_prefix_cache,
            supports_kv_quantization: crate::model_runtime::KvQuantSupport::None,
            supports_activation_steering: false,
            supports_subquadratic: false,
            supports_speculative_draft: false,
            supports_eagle3: false,
            ..Default::default()
        };
        self.declared_capabilities = normalised;
        Ok(handle.model_id)
    }

    async fn unload(&mut self, id: ModelId) -> Result<(), ModelRuntimeError> {
        let mut models = self
            .models
            .write()
            .map_err(|err| ModelRuntimeError::UnloadError(format!("{err}")))?;
        if models.remove(&id).is_none() {
            return Err(ModelRuntimeError::UnloadError(format!(
                "OpenAiByokRuntime model is not registered: {id}"
            )));
        }
        let now_utc = chrono::Utc::now().to_rfc3339();
        let _ = self.audit_sink.record(CloudInvocationAuditRow {
            model_id: id,
            openai_model_name: String::new(),
            call_kind: CloudCallKind::ChatCompletion,
            started_at_utc: now_utc.clone(),
            finished_at_utc: Some(now_utc),
            status: CloudCallStatus::Succeeded,
        });
        Ok(())
    }

    fn generate(&self, req: GenerateRequest) -> TokenStream {
        self.chat_completions_stream(req)
    }

    async fn score(&self, id: ModelId, sequence: Vec<u32>) -> Result<Score, ModelRuntimeError> {
        let invocation_deadline = tokio::time::Instant::now() + self.timeouts.whole_invocation;
        let handle = self
            .handle_for(id)
            .map_err(|err| ModelRuntimeError::ScoreError(format!("{err}")))?;
        let api_key = self
            .fetch_api_key()
            .map_err(|err| ModelRuntimeError::ScoreError(format!("{err}")))?;
        let url = format!("{}{}", self.api_base, OPENAI_CHAT_COMPLETIONS_PATH);
        // For scoring we render the token sequence as a single
        // prompt and ask OpenAI to return logprobs without
        // streaming. Returning the sequence as raw token ids would
        // require a tokenizer round-trip; the closer-to-real path
        // is letting OpenAI tokenise and return logprobs on the
        // assistant continuation.
        let prompt_text: String = sequence
            .iter()
            .map(|tok| tok.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        let body = serde_json::json!({
            "model": handle.openai_model_name,
            "messages": [{ "role": "user", "content": prompt_text }],
            "stream": false,
            "logprobs": true,
            "max_tokens": 16,
        });
        let started_at_utc = chrono::Utc::now().to_rfc3339();
        let _ = self.audit_sink.record(CloudInvocationAuditRow {
            model_id: handle.model_id,
            openai_model_name: handle.openai_model_name.clone(),
            call_kind: CloudCallKind::Score,
            started_at_utc: started_at_utc.clone(),
            finished_at_utc: None,
            status: CloudCallStatus::Started,
        });
        let request = self
            .client
            .post(&url)
            .bearer_auth(api_key.expose_secret())
            .header("Content-Type", "application/json")
            .json(&body);
        drop(api_key);
        let response = await_runtime_step(
            request.send(),
            &self.runtime_cancel,
            invocation_deadline.min(tokio::time::Instant::now() + self.timeouts.connect),
        )
        .await
        .map_err(|failure| match failure {
            ProviderWaitFailure::Cancelled => ModelRuntimeError::Cancelled,
            ProviderWaitFailure::Deadline => ModelRuntimeError::ScoreError(
                "OpenAI BYOK score connect/request deadline elapsed".to_string(),
            ),
        })?
        .map_err(|err| {
            let code = classify_transport_error(&err);
            let _ = self.audit_sink.record(CloudInvocationAuditRow {
                model_id: handle.model_id,
                openai_model_name: handle.openai_model_name.clone(),
                call_kind: CloudCallKind::Score,
                started_at_utc: started_at_utc.clone(),
                finished_at_utc: Some(chrono::Utc::now().to_rfc3339()),
                status: CloudCallStatus::Failed,
            });
            ModelRuntimeError::ScoreError(format!(
                "OpenAI BYOK transport failed; operation=score; code={code}"
            ))
        })?;
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = redacted_provider_response_body(&response);
            let _ = self.audit_sink.record(CloudInvocationAuditRow {
                model_id: handle.model_id,
                openai_model_name: handle.openai_model_name.clone(),
                call_kind: CloudCallKind::Score,
                started_at_utc: started_at_utc.clone(),
                finished_at_utc: Some(chrono::Utc::now().to_rfc3339()),
                status: CloudCallStatus::Failed,
            });
            return Err(ModelRuntimeError::ScoreError(format!(
                "score HTTP {status}: {body}"
            )));
        }
        let parsed: ChatCompletionsResponse = await_runtime_step(
            response.json(),
            &self.runtime_cancel,
            invocation_deadline,
        )
        .await
        .map_err(|failure| match failure {
            ProviderWaitFailure::Cancelled => ModelRuntimeError::Cancelled,
            ProviderWaitFailure::Deadline => {
                ModelRuntimeError::ScoreError("OpenAI BYOK score body deadline elapsed".to_string())
            }
        })?
        .map_err(|err| {
            let _ = err;
            let _ = self.audit_sink.record(CloudInvocationAuditRow {
                model_id: handle.model_id,
                openai_model_name: handle.openai_model_name.clone(),
                call_kind: CloudCallKind::Score,
                started_at_utc: started_at_utc.clone(),
                finished_at_utc: Some(chrono::Utc::now().to_rfc3339()),
                status: CloudCallStatus::Failed,
            });
            ModelRuntimeError::ScoreError(
                "score response JSON parse failed; response_kind=chat_completions; provider_payload=<redacted>"
                    .to_string(),
            )
        })?;
        let token_logprobs: Vec<f32> = parsed
            .choices
            .into_iter()
            .flat_map(|choice| {
                choice
                    .logprobs
                    .map(|lp| {
                        lp.content
                            .into_iter()
                            .map(|t| t.logprob)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            })
            .collect();
        if token_logprobs.is_empty() {
            let _ = self.audit_sink.record(CloudInvocationAuditRow {
                model_id: handle.model_id,
                openai_model_name: handle.openai_model_name.clone(),
                call_kind: CloudCallKind::Score,
                started_at_utc: started_at_utc.clone(),
                finished_at_utc: Some(chrono::Utc::now().to_rfc3339()),
                status: CloudCallStatus::Failed,
            });
            return Err(ModelRuntimeError::ScoreError(
                "score response unusable; response_kind=chat_completions; reason=empty_logprobs; provider_payload=<redacted>"
                    .to_string(),
            ));
        }
        let mean_logprob = if token_logprobs.is_empty() {
            0.0
        } else {
            token_logprobs.iter().sum::<f32>() / token_logprobs.len() as f32
        };
        let _ = self.audit_sink.record(CloudInvocationAuditRow {
            model_id: handle.model_id,
            openai_model_name: handle.openai_model_name.clone(),
            call_kind: CloudCallKind::Score,
            started_at_utc,
            finished_at_utc: Some(chrono::Utc::now().to_rfc3339()),
            status: CloudCallStatus::Succeeded,
        });
        Ok(Score {
            token_logprobs,
            mean_logprob,
        })
    }

    async fn embed(&self, id: ModelId, text: &str) -> Result<Embedding, ModelRuntimeError> {
        let invocation_deadline = tokio::time::Instant::now() + self.timeouts.whole_invocation;
        let handle = self
            .handle_for(id)
            .map_err(|err| ModelRuntimeError::EmbedError(format!("{err}")))?;
        let api_key = self
            .fetch_api_key()
            .map_err(|err| ModelRuntimeError::EmbedError(format!("{err}")))?;
        let url = format!("{}{}", self.api_base, OPENAI_EMBEDDINGS_PATH);
        let body = EmbeddingsRequest {
            model: &handle.openai_model_name,
            input: text,
        };
        let started_at_utc = chrono::Utc::now().to_rfc3339();
        let _ = self.audit_sink.record(CloudInvocationAuditRow {
            model_id: handle.model_id,
            openai_model_name: handle.openai_model_name.clone(),
            call_kind: CloudCallKind::Embeddings,
            started_at_utc: started_at_utc.clone(),
            finished_at_utc: None,
            status: CloudCallStatus::Started,
        });
        let request = self
            .client
            .post(&url)
            .bearer_auth(api_key.expose_secret())
            .header("Content-Type", "application/json")
            .json(&body);
        drop(api_key);
        let response = await_runtime_step(
            request.send(),
            &self.runtime_cancel,
            invocation_deadline.min(tokio::time::Instant::now() + self.timeouts.connect),
        )
        .await
        .map_err(|failure| match failure {
            ProviderWaitFailure::Cancelled => ModelRuntimeError::Cancelled,
            ProviderWaitFailure::Deadline => ModelRuntimeError::EmbedError(
                "OpenAI BYOK embed connect/request deadline elapsed".to_string(),
            ),
        })?
        .map_err(|err| {
            let code = classify_transport_error(&err);
            let _ = self.audit_sink.record(CloudInvocationAuditRow {
                model_id: handle.model_id,
                openai_model_name: handle.openai_model_name.clone(),
                call_kind: CloudCallKind::Embeddings,
                started_at_utc: started_at_utc.clone(),
                finished_at_utc: Some(chrono::Utc::now().to_rfc3339()),
                status: CloudCallStatus::Failed,
            });
            ModelRuntimeError::EmbedError(format!(
                "OpenAI BYOK transport failed; operation=embed; code={code}"
            ))
        })?;
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = redacted_provider_response_body(&response);
            let _ = self.audit_sink.record(CloudInvocationAuditRow {
                model_id: handle.model_id,
                openai_model_name: handle.openai_model_name.clone(),
                call_kind: CloudCallKind::Embeddings,
                started_at_utc: started_at_utc.clone(),
                finished_at_utc: Some(chrono::Utc::now().to_rfc3339()),
                status: CloudCallStatus::Failed,
            });
            return Err(ModelRuntimeError::EmbedError(format!(
                "embed HTTP {status}: {body}"
            )));
        }
        let parsed: EmbeddingsResponse = await_runtime_step(
            response.json(),
            &self.runtime_cancel,
            invocation_deadline,
        )
        .await
        .map_err(|failure| match failure {
            ProviderWaitFailure::Cancelled => ModelRuntimeError::Cancelled,
            ProviderWaitFailure::Deadline => {
                ModelRuntimeError::EmbedError("OpenAI BYOK embed body deadline elapsed".to_string())
            }
        })?
        .map_err(|err| {
            let _ = err;
            let _ = self.audit_sink.record(CloudInvocationAuditRow {
                model_id: handle.model_id,
                openai_model_name: handle.openai_model_name.clone(),
                call_kind: CloudCallKind::Embeddings,
                started_at_utc: started_at_utc.clone(),
                finished_at_utc: Some(chrono::Utc::now().to_rfc3339()),
                status: CloudCallStatus::Failed,
            });
            ModelRuntimeError::EmbedError(
                "embed response JSON parse failed; response_kind=embeddings; provider_payload=<redacted>"
                    .to_string(),
            )
        })?;
        let vector = parsed
            .data
            .into_iter()
            .next()
            .map(|datum| datum.embedding)
            .unwrap_or_default();
        if vector.is_empty() {
            let _ = self.audit_sink.record(CloudInvocationAuditRow {
                model_id: handle.model_id,
                openai_model_name: handle.openai_model_name.clone(),
                call_kind: CloudCallKind::Embeddings,
                started_at_utc: started_at_utc.clone(),
                finished_at_utc: Some(chrono::Utc::now().to_rfc3339()),
                status: CloudCallStatus::Failed,
            });
            return Err(ModelRuntimeError::EmbedError(
                "embed response unusable; response_kind=embeddings; reason=empty_data; provider_payload=<redacted>"
                    .to_string(),
            ));
        }
        let _ = self.audit_sink.record(CloudInvocationAuditRow {
            model_id: handle.model_id,
            openai_model_name: handle.openai_model_name.clone(),
            call_kind: CloudCallKind::Embeddings,
            started_at_utc,
            finished_at_utc: Some(chrono::Utc::now().to_rfc3339()),
            status: CloudCallStatus::Succeeded,
        });
        Ok(Embedding { vector })
    }

    fn capabilities(&self, id: ModelId) -> Result<&ModelCapabilities, ModelRuntimeError> {
        let _ = self
            .handle_for(id)
            .map_err(|err| ModelRuntimeError::LoadError(format!("{err}")))?;
        Ok(&self.declared_capabilities)
    }

    fn kv_cache(&self, _id: ModelId) -> Result<KvCacheHandle, ModelRuntimeError> {
        Err(ModelRuntimeError::CapabilityNotSupported {
            capability: "kv_cache (BYOK cloud lane is server-side opaque)".to_string(),
            adapter: "openai_byok".to_string(),
        })
    }

    fn lora_stack(&self, _id: ModelId) -> Result<LoraStackHandle, ModelRuntimeError> {
        Err(ModelRuntimeError::CapabilityNotSupported {
            capability: "lora_stack (BYOK cloud lane has no local weights to mount LoRAs onto)"
                .to_string(),
            adapter: "openai_byok".to_string(),
        })
    }

    fn steering_hooks(&self, _id: ModelId) -> Result<SteeringHookHandle, ModelRuntimeError> {
        Err(ModelRuntimeError::CapabilityNotSupported {
            capability: "steering_hooks (BYOK cloud lane has no residual stream to hook)"
                .to_string(),
            adapter: "openai_byok".to_string(),
        })
    }

    fn cancel(&self, token: CancellationToken) {
        token.cancel();
        self.runtime_cancel.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct StaticKeyProvider {
        key: String,
    }
    impl ApiKeyProvider for StaticKeyProvider {
        fn fetch_api_key(&self) -> Result<TransientApiKey, OpenAiByokError> {
            Ok(TransientApiKey::from_secret(self.key.clone()))
        }
    }

    #[derive(Default)]
    struct CapturingSink {
        rows: Mutex<Vec<CloudInvocationAuditRow>>,
    }
    impl CloudInvocationAuditSink for CapturingSink {
        fn record(&self, row: CloudInvocationAuditRow) -> Result<(), OpenAiByokError> {
            self.rows.lock().unwrap().push(row);
            Ok(())
        }
    }

    fn fixture_runtime() -> OpenAiByokRuntime {
        OpenAiByokRuntime::new(
            "https://api.openai.com/v1".to_string(),
            Arc::new(StaticKeyProvider {
                key: "sk-test-NEVER-LOG-THIS".to_string(),
            }),
            Arc::new(CapturingSink::default()),
        )
    }

    #[test]
    fn debug_repr_redacts_api_key_provider() {
        let runtime = fixture_runtime();
        let dbg = format!("{runtime:?}");
        assert!(dbg.contains("<redacted"));
        assert!(!dbg.contains("sk-test-NEVER-LOG-THIS"));
    }

    #[test]
    fn register_handle_accepts_allowlisted_model_family() {
        let runtime = fixture_runtime();
        let handle = runtime
            .register_handle("gpt-4o-2024-08-06", "2026-05-20T05:30:00Z")
            .expect("allowlisted gpt-4o family");
        assert_eq!(handle.openai_model_name, "gpt-4o-2024-08-06");
        let lookup = runtime.handle_for(handle.model_id).expect("registered");
        assert_eq!(lookup, handle);
    }

    #[test]
    fn register_handle_rejects_non_allowlisted_model_name() {
        let runtime = fixture_runtime();
        let err = runtime
            .register_handle("definitely-not-openai", "2026-05-20T05:30:00Z")
            .expect_err("not in allowlist");
        assert!(matches!(err, OpenAiByokError::ModelNameNotAllowed(_)));
    }

    #[test]
    fn register_handle_rejects_empty_model_name() {
        let runtime = fixture_runtime();
        let err = runtime
            .register_handle("  ", "2026-05-20T05:30:00Z")
            .expect_err("empty model name");
        assert!(matches!(err, OpenAiByokError::EmptyModelName));
    }

    #[test]
    fn register_model_name_extends_allowlist() {
        let runtime = fixture_runtime();
        runtime
            .register_model_name("custom-finetune-")
            .expect("extend");
        let handle = runtime
            .register_handle("custom-finetune-v1", "2026-05-20T05:30:00Z")
            .expect("now allowed");
        assert_eq!(handle.openai_model_name, "custom-finetune-v1");
    }

    #[test]
    fn capabilities_match_byok_cloud_realities() {
        let caps = OpenAiByokRuntime::cloud_capabilities();
        assert!(!caps.supports_lora);
        assert!(caps.supports_kv_prefix_cache);
        assert!(!caps.supports_activation_steering);
        assert!(!caps.supports_subquadratic);
        assert!(!caps.supports_speculative_draft);
        assert!(!caps.supports_eagle3);
    }

    #[test]
    fn audit_sink_records_rows_through_the_runtime() {
        let runtime = fixture_runtime();
        let handle = runtime
            .register_handle("gpt-4o", "2026-05-20T05:30:00Z")
            .unwrap();
        runtime
            .record_audit(CloudInvocationAuditRow {
                model_id: handle.model_id,
                openai_model_name: handle.openai_model_name.clone(),
                call_kind: CloudCallKind::ChatCompletion,
                started_at_utc: "2026-05-20T05:30:00Z".to_string(),
                finished_at_utc: None,
                status: CloudCallStatus::Started,
            })
            .expect("audit");
    }

    #[test]
    fn fetch_api_key_returns_provider_value_and_does_not_log_it() {
        let runtime = fixture_runtime();
        let key = runtime.fetch_api_key().expect("fetch");
        assert!(key.expose_secret().starts_with("sk-"));
        assert_eq!(format!("{key:?}"), "TransientApiKey([REDACTED])");
    }

    #[test]
    fn handle_for_unknown_model_returns_not_registered() {
        let runtime = fixture_runtime();
        let unknown = ModelId::new_v7();
        let err = runtime.handle_for(unknown).expect_err("unknown model");
        assert!(matches!(err, OpenAiByokError::ModelNotRegistered(_)));
    }

    #[test]
    fn capabilities_unknown_model_returns_load_error() {
        let runtime = fixture_runtime();
        let unknown = ModelId::new_v7();
        let err = runtime.capabilities(unknown).expect_err("unknown");
        assert!(matches!(err, ModelRuntimeError::LoadError(_)));
    }

    #[test]
    fn cancel_marks_runtime_cancellation_token() {
        let runtime = fixture_runtime();
        let outer = CancellationToken::new();
        runtime.cancel(outer.clone());
        assert!(outer.is_cancelled());
        assert!(runtime.runtime_cancel.is_cancelled());
    }

    #[tokio::test]
    async fn provider_wait_is_bounded_and_cancellation_sensitive() {
        let request_cancel = CancellationToken::new();
        let runtime_cancel = CancellationToken::new();
        let owner_cancel = CancellationToken::new();
        let timeout = await_provider_step(
            std::future::pending::<()>(),
            &request_cancel,
            &runtime_cancel,
            &owner_cancel,
            tokio::time::Instant::now() + Duration::from_millis(1),
        )
        .await;
        assert!(matches!(timeout, Err(ProviderWaitFailure::Deadline)));

        request_cancel.cancel();
        let cancelled = await_provider_step(
            std::future::pending::<()>(),
            &request_cancel,
            &runtime_cancel,
            &owner_cancel,
            tokio::time::Instant::now() + Duration::from_secs(1),
        )
        .await;
        assert!(matches!(cancelled, Err(ProviderWaitFailure::Cancelled)));
    }

    #[tokio::test]
    async fn dropping_cloud_stream_aborts_owned_pipeline_task() {
        use std::sync::atomic::{AtomicBool, Ordering};

        struct DropMark(Arc<AtomicBool>);
        impl Drop for DropMark {
            fn drop(&mut self) {
                self.0.store(true, Ordering::Release);
            }
        }

        let dropped = Arc::new(AtomicBool::new(false));
        let task_dropped = dropped.clone();
        let (sender, receiver) =
            tokio::sync::mpsc::unbounded_channel::<Result<GeneratedToken, ModelRuntimeError>>();
        let task = tokio::spawn(async move {
            let _mark = DropMark(task_dropped);
            let _sender = sender;
            std::future::pending::<()>().await;
        });
        tokio::task::yield_now().await;
        let stream = OwnedCloudTokenStream {
            receiver,
            task: Some(task),
            owner_cancel: CancellationToken::new(),
        };
        drop(stream);
        tokio::time::timeout(Duration::from_secs(1), async {
            while !dropped.load(Ordering::Acquire) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("owned provider task must be released within the teardown bound");
        assert!(
            dropped.load(Ordering::Acquire),
            "stream Drop must abort and release the owned provider task"
        );
    }
}

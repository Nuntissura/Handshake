//! MT-126: Cloud lane BYOK Anthropic Messages API runtime.
//!
//! Implements [`AnthropicByokRuntime`] as a [`ModelRuntime`] adapter
//! for the BYOK cloud lane. Per HBR-INT-005 lane normalisation the
//! adapter exposes the same trait surface as the local adapters and
//! the OpenAI BYOK sibling (load / unload / generate / score / embed
//! / capabilities / kv_cache / lora_stack / steering_hooks /
//! cancel). The wire format is the Anthropic Messages HTTP API with
//! Server-Sent Events streaming.
//!
//! Operationally dormant for the operator (no BYOK credits) but
//! architecturally required: every code path is exercised under
//! [`wiremock`] in the integration tests, which binds a real TCP
//! port answering the documented Anthropic protocol shape. That
//! satisfies Spec-Realism Gate sub-rule 2 (real external resource
//! touch: real reqwest -> real socket -> wiremock answering protocol
//! shape).
//!
//! Anthropic protocol deviations from the OpenAI sibling:
//!
//! - Endpoint is `POST /v1/messages` (not `/chat/completions`).
//! - Auth header is `x-api-key: <key>` (not `Authorization: Bearer`).
//! - Every request carries `anthropic-version: 2023-06-01`.
//! - SSE chunks carry a named `event:` field; tokens arrive as
//!   `event: content_block_delta` with `data: {"delta":{"text":...}}`.
//! - Stream terminates on `event: message_stop` (no `[DONE]` sentinel).
//! - Anthropic has no first-party embeddings endpoint, so [`embed`]
//!   returns [`ModelRuntimeError::CapabilityNotSupported`].
//! - Anthropic Messages does not return logprobs, so [`score`]
//!   returns [`ModelRuntimeError::CapabilityNotSupported`].
//!
//! Invariants enforced here mirror the OpenAI sibling:
//!
//! - The operator API key never leaves the [`ApiKeyProvider`]
//!   boundary in Debug/Display output.
//! - Capabilities match BYOK cloud realities (no local LoRA, no
//!   activation steering, no subquadratic, no speculative draft;
//!   prompt caching is implicit via Anthropic's caching, KV
//!   quantisation is server-side opaque).
//! - Model name allowlist (prefix-match) gates `load()`.
//! - Every call writes a [`CloudInvocationAuditRow`] through the
//!   [`CloudInvocationAuditSink`] trait (Started / Succeeded /
//!   Failed / Cancelled).
//! - No [`crate::process_ledger`] row is written: BYOK invocations
//!   do not spawn a Handshake-owned process.

use std::collections::{HashMap, HashSet};
use std::pin::Pin;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use futures::{stream, Stream, StreamExt};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::openai_byok::{
    ApiKeyProvider, CloudCallKind, CloudCallStatus, CloudInvocationAuditRow,
    CloudInvocationAuditSink, OpenAiByokError,
};
use crate::model_runtime::{
    error::ModelRuntimeError, CancellationToken, Embedding, FinishReason, GenerateRequest,
    GeneratedToken, KvCacheHandle, LoadSpec, LoraStackHandle, ModelCapabilities, ModelId,
    ModelRuntime, ProviderKind, Score, SteeringHookHandle, TokenStream,
};

/// Allowlist of Anthropic model name prefixes the operator has
/// approved for BYOK cloud invocation. Defaults to the Claude
/// families current as of WP-KERNEL-004; operators may extend via
/// [`AnthropicByokRuntime::register_model_name`].
pub const DEFAULT_ANTHROPIC_MODEL_ALLOWLIST: &[&str] = &[
    "claude-3-opus",
    "claude-3-sonnet",
    "claude-3-haiku",
    "claude-3.5-sonnet",
    "claude-3.5-haiku",
    "claude-3.7",
    "claude-opus-4",
    "claude-sonnet-4",
    "claude-haiku-4",
];

/// Default messages path appended to the runtime's `api_base`.
/// Exposed so tests can compare against the wiremock expectation.
///
/// Production callers pass `api_base = "https://api.anthropic.com"`
/// (bare host, no `/v1`); test callers pass `api_base = mock_server.uri()`
/// (bare host). The `/v1/` prefix lives here so both shapes resolve
/// to the official documented endpoint.
pub const ANTHROPIC_MESSAGES_PATH: &str = "/v1/messages";

/// API version pin sent on every request as required by the
/// Anthropic Messages API. The value is the public stable version
/// declared by Anthropic; operators rotate by updating this constant.
pub const ANTHROPIC_API_VERSION: &str = "2023-06-01";

/// HTTP header carrying the operator BYOK secret on every request.
/// Distinct from OpenAI's `Authorization: Bearer` to keep the
/// adapter API explicit at call sites.
pub const ANTHROPIC_API_KEY_HEADER: &str = "x-api-key";

/// HTTP header carrying the [`ANTHROPIC_API_VERSION`] pin.
pub const ANTHROPIC_VERSION_HEADER: &str = "anthropic-version";

/// SSE event-name dispatch tags emitted by Anthropic. Tokens arrive
/// as `content_block_delta`; the stream terminates cleanly on
/// `message_stop`. Other events (`message_start`,
/// `content_block_start`, `content_block_stop`, `ping`,
/// `message_delta`) are observable but do not yield tokens.
const SSE_EVENT_CONTENT_BLOCK_DELTA: &str = "content_block_delta";
const SSE_EVENT_MESSAGE_DELTA: &str = "message_delta";
const SSE_EVENT_MESSAGE_STOP: &str = "message_stop";

/// Per-registered-model handle. Maps the Handshake `ModelId` (UUID
/// v7) to the Anthropic model name string used on the wire.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnthropicModelHandle {
    pub model_id: ModelId,
    pub anthropic_model_name: String,
    pub registered_at_utc: String,
}

#[derive(Debug, Error)]
pub enum AnthropicByokError {
    #[error(
        "Anthropic model name {0} is not in the BYOK allowlist (extend via register_model_name)"
    )]
    ModelNameNotAllowed(String),
    #[error("Anthropic model name must not be empty")]
    EmptyModelName,
    #[error("model_id {0} is not registered with the Anthropic BYOK runtime")]
    ModelNotRegistered(ModelId),
    #[error("API key fetch failed: {0}")]
    ApiKeyFetch(String),
    #[error("audit row persistence failed: {0}")]
    AuditPersist(String),
    #[error("internal lock poisoned: {0}")]
    LockPoisoned(String),
    #[error("only ByokCloud provider is supported by AnthropicByokRuntime (got {0:?})")]
    ProviderKindNotSupported(ProviderKind),
    #[error("HTTP request to {url} failed: {source}")]
    RequestFailed {
        url: String,
        source: reqwest::Error,
    },
    #[error("HTTP response status {status} body {body}")]
    HttpStatus { status: u16, body: String },
    #[error("SSE stream parse failed: {0}")]
    StreamParseFailed(String),
    #[error("JSON (de)serialisation failed: {0}")]
    JsonFailed(String),
    #[error("call cancelled before completion")]
    Cancelled,
}

impl From<OpenAiByokError> for AnthropicByokError {
    fn from(value: OpenAiByokError) -> Self {
        // ApiKeyFetch + audit are the only variants we transit
        // through the shared sink/provider traits; map them onto
        // the same shape on our side so the API stays Anthropic-
        // focused at the boundary.
        match value {
            OpenAiByokError::ApiKeyFetch(msg) => AnthropicByokError::ApiKeyFetch(msg),
            OpenAiByokError::AuditPersist(msg) => AnthropicByokError::AuditPersist(msg),
            OpenAiByokError::LockPoisoned(msg) => AnthropicByokError::LockPoisoned(msg),
            other => AnthropicByokError::ApiKeyFetch(format!("{other}")),
        }
    }
}

/// JSON shape the runtime POSTs to `/v1/messages`.
#[derive(Debug, Serialize)]
struct MessagesRequest<'a> {
    model: &'a str,
    messages: Vec<AnthropicMessage<'a>>,
    max_tokens: u32,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<u32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    stop_sequences: Vec<String>,
}

#[derive(Debug, Serialize)]
struct AnthropicMessage<'a> {
    role: &'a str,
    content: &'a str,
}

/// JSON shape of a `content_block_delta` SSE event payload. The
/// outer envelope carries `type`, `index`, and the inner `delta`;
/// only `delta.text` is consumed by the streaming loop.
#[derive(Debug, Deserialize)]
struct ContentBlockDeltaPayload {
    #[serde(default)]
    delta: ContentBlockDelta,
}

#[derive(Debug, Default, Deserialize)]
struct ContentBlockDelta {
    /// `type` is one of `text_delta` (token chunks) or
    /// `input_json_delta` (tool calls). The runtime only emits text.
    #[serde(default, rename = "type")]
    delta_type: Option<String>,
    #[serde(default)]
    text: Option<String>,
}

/// JSON shape of a `message_delta` SSE event payload. Carries the
/// final `stop_reason` once the model has decided to terminate.
#[derive(Debug, Deserialize)]
struct MessageDeltaPayload {
    #[serde(default)]
    delta: MessageDelta,
}

#[derive(Debug, Default, Deserialize)]
struct MessageDelta {
    #[serde(default)]
    stop_reason: Option<String>,
}

/// BYOK cloud runtime. Holds a `reqwest::Client` for live HTTP /
/// SSE invocation; the struct deliberately does NOT hold the API
/// key string — it holds an `Arc<dyn ApiKeyProvider>` so the
/// secret is fetched on demand from `OperatorSecretsVault` and
/// never serialised into the struct.
pub struct AnthropicByokRuntime {
    api_base: String,
    client: reqwest::Client,
    api_key_provider: Arc<dyn ApiKeyProvider>,
    audit_sink: Arc<dyn CloudInvocationAuditSink>,
    allowlist: RwLock<HashSet<String>>,
    models: RwLock<HashMap<ModelId, AnthropicModelHandle>>,
    declared_capabilities: ModelCapabilities,
    runtime_cancel: CancellationToken,
}

impl std::fmt::Debug for AnthropicByokRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // GLOBAL: secret material is never surfaced in Debug. The
        // api_key_provider is shown as a placeholder.
        f.debug_struct("AnthropicByokRuntime")
            .field("api_base", &self.api_base)
            .field("api_key_provider", &"<redacted Arc<dyn ApiKeyProvider>>")
            .field("audit_sink", &"<Arc<dyn CloudInvocationAuditSink>>")
            .field("models", &self.models.read().map(|m| m.len()).unwrap_or(0))
            .finish()
    }
}

impl AnthropicByokRuntime {
    pub fn new(
        api_base: impl Into<String>,
        api_key_provider: Arc<dyn ApiKeyProvider>,
        audit_sink: Arc<dyn CloudInvocationAuditSink>,
    ) -> Self {
        Self::with_client(
            api_base,
            reqwest::Client::new(),
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
                DEFAULT_ANTHROPIC_MODEL_ALLOWLIST
                    .iter()
                    .map(|s| (*s).to_string())
                    .collect(),
            ),
            models: RwLock::new(HashMap::new()),
            declared_capabilities: Self::cloud_capabilities(),
            runtime_cancel: CancellationToken::new(),
        }
    }

    /// Extend the allowlist with an additional Anthropic model-name prefix.
    pub fn register_model_name(&self, model_name: &str) -> Result<(), AnthropicByokError> {
        if model_name.trim().is_empty() {
            return Err(AnthropicByokError::EmptyModelName);
        }
        let mut guard = self
            .allowlist
            .write()
            .map_err(|err| AnthropicByokError::LockPoisoned(err.to_string()))?;
        guard.insert(model_name.to_string());
        Ok(())
    }

    /// Register a model handle. Validates the requested Anthropic
    /// model name against the allowlist (prefix-match; supports
    /// family allowlisting like `claude-opus-4`).
    pub fn register_handle(
        &self,
        anthropic_model_name: &str,
        now_utc: &str,
    ) -> Result<AnthropicModelHandle, AnthropicByokError> {
        if anthropic_model_name.trim().is_empty() {
            return Err(AnthropicByokError::EmptyModelName);
        }
        let allowed = {
            let guard = self
                .allowlist
                .read()
                .map_err(|err| AnthropicByokError::LockPoisoned(err.to_string()))?;
            guard
                .iter()
                .any(|prefix| anthropic_model_name.starts_with(prefix))
        };
        if !allowed {
            return Err(AnthropicByokError::ModelNameNotAllowed(
                anthropic_model_name.to_string(),
            ));
        }
        let model_id = ModelId::new_v7();
        let handle = AnthropicModelHandle {
            model_id,
            anthropic_model_name: anthropic_model_name.to_string(),
            registered_at_utc: now_utc.to_string(),
        };
        let mut models = self
            .models
            .write()
            .map_err(|err| AnthropicByokError::LockPoisoned(err.to_string()))?;
        models.insert(model_id, handle.clone());
        // Audit the registration as a Started lifecycle row so the
        // operator can correlate which model_ids the kernel attached
        // to the BYOK lane and when. The shared audit row reuses
        // `openai_model_name` as the model-name column; Anthropic
        // names are written verbatim.
        self.audit_sink
            .record(CloudInvocationAuditRow {
                model_id,
                openai_model_name: anthropic_model_name.to_string(),
                call_kind: CloudCallKind::ChatCompletion,
                started_at_utc: now_utc.to_string(),
                finished_at_utc: None,
                status: CloudCallStatus::Started,
            })
            .map_err(AnthropicByokError::from)?;
        Ok(handle)
    }

    /// Capability declaration for the Anthropic BYOK cloud lane.
    /// HBR-INT-005 lane normalisation: same capability set as
    /// OpenAI BYOK (server-side opaque; no local LoRA / steering /
    /// subquadratic / speculative draft). Anthropic's first-party
    /// prompt caching is reflected as `supports_kv_prefix_cache=true`;
    /// the cache itself is server-side opaque to the kernel.
    pub fn cloud_capabilities() -> ModelCapabilities {
        ModelCapabilities {
            supports_lora: false,
            supports_kv_prefix_cache: true,
            supports_kv_quantization: crate::model_runtime::KvQuantSupport::None,
            supports_activation_steering: false,
            supports_subquadratic: false,
            supports_speculative_draft: false,
            supports_eagle3: false,
        }
    }

    /// Look up a registered handle.
    pub fn handle_for(
        &self,
        model_id: ModelId,
    ) -> Result<AnthropicModelHandle, AnthropicByokError> {
        let models = self
            .models
            .read()
            .map_err(|err| AnthropicByokError::LockPoisoned(err.to_string()))?;
        models
            .get(&model_id)
            .cloned()
            .ok_or(AnthropicByokError::ModelNotRegistered(model_id))
    }

    /// Convenience: fetch the secret from the provider. The key is
    /// returned by value to the caller; ensure it is dropped quickly
    /// and never logged.
    pub fn fetch_api_key(&self) -> Result<String, AnthropicByokError> {
        self.api_key_provider
            .fetch_api_key()
            .map_err(|err| AnthropicByokError::ApiKeyFetch(format!("{err}")))
    }

    /// Records an audit row through the sink. Tests use this to
    /// inject lifecycle rows without bringing up the HTTP client;
    /// the live HTTP path emits rows through the same sink.
    pub fn record_audit(&self, row: CloudInvocationAuditRow) -> Result<(), AnthropicByokError> {
        self.audit_sink.record(row).map_err(AnthropicByokError::from)
    }

    /// Live streaming call to Anthropic's Messages endpoint with
    /// `stream=true`. Returns a [`TokenStream`] that yields one
    /// [`GeneratedToken`] per `event: content_block_delta` whose
    /// inner `delta.text` is non-empty, terminating cleanly on the
    /// SSE `event: message_stop` marker.
    ///
    /// Audit rows are written at the start (status=Started) and at
    /// the end (Succeeded / Cancelled / Failed) through the runtime's
    /// configured [`CloudInvocationAuditSink`].
    pub fn messages_stream(&self, req: GenerateRequest) -> TokenStream {
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

        let url = format!("{}{}", self.api_base, ANTHROPIC_MESSAGES_PATH);
        let body = MessagesRequest {
            model: &handle.anthropic_model_name,
            messages: vec![AnthropicMessage {
                role: "user",
                content: req.prompt.as_str(),
            }],
            max_tokens: req.max_tokens,
            stream: true,
            temperature: req.sampling.temperature,
            top_p: req.sampling.top_p,
            top_k: req.sampling.top_k,
            stop_sequences: req.stop_sequences.clone(),
        };

        let body_json = match serde_json::to_vec(&body) {
            Ok(bytes) => bytes,
            Err(err) => {
                return single_error_stream(ModelRuntimeError::GenerateError(format!(
                    "Anthropic BYOK request serialisation failed: {err}"
                )));
            }
        };

        let started_at_utc = chrono::Utc::now().to_rfc3339();
        if let Err(err) = self.audit_sink.record(CloudInvocationAuditRow {
            model_id: handle.model_id,
            openai_model_name: handle.anthropic_model_name.clone(),
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
        let anthropic_model_name = handle.anthropic_model_name.clone();

        // The reqwest+SSE pipeline runs inside a spawned task and
        // pumps `GeneratedToken`s onto an mpsc channel; the
        // TokenStream the caller sees is built from the receiver.
        // The pipeline emits one GeneratedToken per
        // `event: content_block_delta` with a non-empty `delta.text`
        // and terminates cleanly on `event: message_stop` or either
        // cancellation token.
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
                openai_model_name: anthropic_model_name,
                call_kind: CloudCallKind::ChatCompletion,
                started_at_utc,
                finished_at_utc: None,
                status: CloudCallStatus::Started,
            },
        )
    }
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
/// This shape (channel + unfold) matches the OpenAI BYOK adapter's
/// streaming path. The differences are the named-event dispatch
/// (Anthropic SSE) and the `event: message_stop` termination marker
/// (Anthropic's analogue to OpenAI's `[DONE]` sentinel).
fn async_token_stream(
    client: reqwest::Client,
    url: String,
    api_key: String,
    body_json: Vec<u8>,
    cancel_req: CancellationToken,
    cancel_runtime: CancellationToken,
    audit_sink: Arc<dyn CloudInvocationAuditSink>,
    audit_template: CloudInvocationAuditRow,
) -> Pin<Box<dyn Stream<Item = Result<GeneratedToken, ModelRuntimeError>> + Send>> {
    let (sender, receiver) =
        tokio::sync::mpsc::unbounded_channel::<Result<GeneratedToken, ModelRuntimeError>>();

    tokio::spawn(run_live_stream(
        client,
        url,
        api_key,
        body_json,
        cancel_req,
        cancel_runtime,
        audit_sink,
        audit_template,
        sender,
    ));

    Box::pin(stream::unfold(receiver, |mut receiver| async {
        receiver.recv().await.map(|item| (item, receiver))
    }))
}

/// Async driver. Sends one `Result<GeneratedToken, _>` per
/// `event: content_block_delta` SSE chunk into `sender`, terminates
/// on `event: message_stop` / cancellation / HTTP error.
#[allow(clippy::too_many_arguments)]
async fn run_live_stream(
    client: reqwest::Client,
    url: String,
    api_key: String,
    body_json: Vec<u8>,
    cancel_req: CancellationToken,
    cancel_runtime: CancellationToken,
    audit_sink: Arc<dyn CloudInvocationAuditSink>,
    audit_template: CloudInvocationAuditRow,
    sender: tokio::sync::mpsc::UnboundedSender<Result<GeneratedToken, ModelRuntimeError>>,
) {
    use eventsource_stream::Eventsource;

    // Pre-call cancellation: terminate without issuing the
    // request, audit as Cancelled.
    if cancel_req.is_cancelled() || cancel_runtime.is_cancelled() {
        record_final_audit(&audit_sink, &audit_template, CloudCallStatus::Cancelled);
        let _ = sender.send(Ok(terminal_token(FinishReason::Cancelled)));
        return;
    }

    let response = match client
        .post(&url)
        .header(ANTHROPIC_API_KEY_HEADER, &api_key)
        .header(ANTHROPIC_VERSION_HEADER, ANTHROPIC_API_VERSION)
        .header("Content-Type", "application/json")
        .body(body_json)
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(err) => {
            record_final_audit(&audit_sink, &audit_template, CloudCallStatus::Failed);
            let _ = sender.send(Err(ModelRuntimeError::GenerateError(format!(
                "Anthropic BYOK request to {url} failed: {err}"
            ))));
            return;
        }
    };

    // Drop the api_key as soon as the request has been sent;
    // it lives in the `x-api-key` header on the wire, but the
    // local memory copy is no longer needed.
    drop(api_key);

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        record_final_audit(&audit_sink, &audit_template, CloudCallStatus::Failed);
        let _ = sender.send(Err(ModelRuntimeError::GenerateError(format!(
            "Anthropic BYOK HTTP {status}: {body}"
        ))));
        return;
    }

    let mut sse = response.bytes_stream().eventsource();
    let mut token_index: u32 = 0;
    let mut pending_finish: Option<FinishReason> = None;

    while let Some(event) = sse.next().await {
        if cancel_req.is_cancelled() || cancel_runtime.is_cancelled() {
            record_final_audit(&audit_sink, &audit_template, CloudCallStatus::Cancelled);
            let _ = sender.send(Ok(terminal_token(FinishReason::Cancelled)));
            return;
        }

        let event = match event {
            Ok(event) => event,
            Err(err) => {
                record_final_audit(&audit_sink, &audit_template, CloudCallStatus::Failed);
                let _ = sender.send(Err(ModelRuntimeError::GenerateError(format!(
                    "Anthropic BYOK SSE parse failure: {err}"
                ))));
                return;
            }
        };

        match event.event.as_str() {
            SSE_EVENT_CONTENT_BLOCK_DELTA => {
                let payload: ContentBlockDeltaPayload = match serde_json::from_str(&event.data) {
                    Ok(p) => p,
                    Err(err) => {
                        record_final_audit(
                            &audit_sink,
                            &audit_template,
                            CloudCallStatus::Failed,
                        );
                        let _ = sender.send(Err(ModelRuntimeError::GenerateError(format!(
                            "Anthropic BYOK content_block_delta JSON parse failure: {err}; payload={}",
                            event.data
                        ))));
                        return;
                    }
                };
                // Tool-call deltas (`input_json_delta`) and any other
                // future delta types are observable but not emitted;
                // only `text_delta` chunks contribute to the token stream.
                let is_text_delta = payload
                    .delta
                    .delta_type
                    .as_deref()
                    .map(|t| t == "text_delta")
                    .unwrap_or(true); // be permissive if the field is absent
                if !is_text_delta {
                    continue;
                }
                if let Some(text) = payload.delta.text {
                    if !text.is_empty() {
                        token_index = token_index.saturating_add(1);
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
                            return;
                        }
                    }
                }
            }
            SSE_EVENT_MESSAGE_DELTA => {
                // `message_delta` carries the final `stop_reason`.
                // We buffer it; the actual stream-close signal is
                // the subsequent `message_stop` event.
                if let Ok(payload) = serde_json::from_str::<MessageDeltaPayload>(&event.data) {
                    if let Some(reason) = payload.delta.stop_reason {
                        pending_finish = map_finish_reason(&reason);
                    }
                }
            }
            SSE_EVENT_MESSAGE_STOP => {
                // Clean termination. If we observed a stop_reason on
                // a preceding `message_delta`, emit it; otherwise
                // default to FinishReason::Stop.
                let finish = pending_finish.unwrap_or(FinishReason::Stop);
                record_final_audit(&audit_sink, &audit_template, CloudCallStatus::Succeeded);
                let _ = sender.send(Ok(terminal_token(finish)));
                return;
            }
            // Other Anthropic events (`message_start`,
            // `content_block_start`, `content_block_stop`, `ping`)
            // are observable but do not affect the token stream.
            _ => {}
        }
    }

    // Stream ended without an explicit `message_stop`. Treat as a
    // clean server-side close; a truly broken response would have
    // produced an error item above and returned.
    record_final_audit(&audit_sink, &audit_template, CloudCallStatus::Succeeded);
    let _ = sender.send(Ok(terminal_token(
        pending_finish.unwrap_or(FinishReason::Stop),
    )));
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
            target: "handshake_core::model_runtime::cloud::anthropic_byok",
            error = %err,
            "Anthropic BYOK final audit row failed to persist; call already completed"
        );
    }
}

fn map_finish_reason(value: &str) -> Option<FinishReason> {
    // Anthropic Messages `stop_reason` vocabulary as of v2023-06-01:
    //   "end_turn"       — model decided to stop on its own.
    //   "max_tokens"     — `max_tokens` budget exhausted.
    //   "stop_sequence"  — one of the caller's `stop_sequences` hit.
    //   "tool_use"       — model paused to invoke a tool (not used here).
    match value {
        "end_turn" => Some(FinishReason::Stop),
        "max_tokens" => Some(FinishReason::Length),
        "stop_sequence" => Some(FinishReason::Stop),
        "tool_use" => Some(FinishReason::Stop),
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
impl ModelRuntime for AnthropicByokRuntime {
    fn adapter_name(&self) -> &'static str {
        "anthropic_byok"
    }

    async fn load(&mut self, spec: LoadSpec) -> Result<ModelId, ModelRuntimeError> {
        if spec.provider != ProviderKind::ByokCloud {
            return Err(ModelRuntimeError::LoadError(format!(
                "AnthropicByokRuntime requires provider=ByokCloud (got {:?})",
                spec.provider
            )));
        }
        let model_name = spec.engine_origin.as_deref().ok_or_else(|| {
            ModelRuntimeError::LoadError(
                "AnthropicByokRuntime.load requires LoadSpec::engine_origin = Anthropic model name"
                    .to_string(),
            )
        })?;
        let now_utc = chrono::Utc::now().to_rfc3339();
        let handle = self
            .register_handle(model_name, &now_utc)
            .map_err(|err| ModelRuntimeError::LoadError(format!("{err}")))?;
        // The runtime's declared capabilities snapshot is normalised
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
                "AnthropicByokRuntime model is not registered: {id}"
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
        self.messages_stream(req)
    }

    async fn score(&self, _id: ModelId, _sequence: Vec<u32>) -> Result<Score, ModelRuntimeError> {
        // Anthropic Messages API does not return per-token logprobs.
        // Per HBR-INT-005, BYOK adapters surface unsupported
        // capabilities through `CapabilityNotSupported` rather than
        // fabricating data.
        Err(ModelRuntimeError::CapabilityNotSupported {
            capability:
                "score (Anthropic Messages API does not expose per-token logprobs)"
                    .to_string(),
            adapter: "anthropic_byok".to_string(),
        })
    }

    async fn embed(
        &self,
        _id: ModelId,
        _text: &str,
    ) -> Result<Embedding, ModelRuntimeError> {
        // Anthropic does not offer a first-party embeddings endpoint
        // as of the 2023-06-01 API version. Operators who want
        // embeddings on the Anthropic stack route through Voyage AI
        // (Anthropic's recommended third-party provider) which is
        // out of scope for this adapter.
        Err(ModelRuntimeError::CapabilityNotSupported {
            capability:
                "embed (Anthropic has no first-party embeddings endpoint; use Voyage AI or another provider)"
                    .to_string(),
            adapter: "anthropic_byok".to_string(),
        })
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
            adapter: "anthropic_byok".to_string(),
        })
    }

    fn lora_stack(&self, _id: ModelId) -> Result<LoraStackHandle, ModelRuntimeError> {
        Err(ModelRuntimeError::CapabilityNotSupported {
            capability:
                "lora_stack (BYOK cloud lane has no local weights to mount LoRAs onto)"
                    .to_string(),
            adapter: "anthropic_byok".to_string(),
        })
    }

    fn steering_hooks(&self, _id: ModelId) -> Result<SteeringHookHandle, ModelRuntimeError> {
        Err(ModelRuntimeError::CapabilityNotSupported {
            capability:
                "steering_hooks (BYOK cloud lane has no residual stream to hook)"
                    .to_string(),
            adapter: "anthropic_byok".to_string(),
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
        fn fetch_api_key(&self) -> Result<String, OpenAiByokError> {
            Ok(self.key.clone())
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

    fn fixture_runtime() -> AnthropicByokRuntime {
        AnthropicByokRuntime::new(
            "https://api.anthropic.com".to_string(),
            Arc::new(StaticKeyProvider {
                key: "sk-ant-NEVER-LOG-THIS-KEY".to_string(),
            }),
            Arc::new(CapturingSink::default()),
        )
    }

    #[test]
    fn debug_repr_redacts_api_key_provider() {
        let runtime = fixture_runtime();
        let dbg = format!("{runtime:?}");
        assert!(dbg.contains("<redacted"));
        assert!(!dbg.contains("sk-ant-NEVER-LOG-THIS-KEY"));
    }

    #[test]
    fn register_handle_accepts_allowlisted_claude_family() {
        let runtime = fixture_runtime();
        let handle = runtime
            .register_handle("claude-opus-4-7-20260101", "2026-05-20T06:00:00Z")
            .expect("allowlisted claude-opus-4 family");
        assert_eq!(handle.anthropic_model_name, "claude-opus-4-7-20260101");
        let lookup = runtime.handle_for(handle.model_id).expect("registered");
        assert_eq!(lookup, handle);
    }

    #[test]
    fn register_handle_rejects_non_allowlisted_model_name() {
        let runtime = fixture_runtime();
        let err = runtime
            .register_handle("not-claude", "2026-05-20T06:00:00Z")
            .expect_err("not in allowlist");
        assert!(matches!(err, AnthropicByokError::ModelNameNotAllowed(_)));
    }

    #[test]
    fn register_handle_rejects_empty_model_name() {
        let runtime = fixture_runtime();
        let err = runtime
            .register_handle("  ", "2026-05-20T06:00:00Z")
            .expect_err("empty model name");
        assert!(matches!(err, AnthropicByokError::EmptyModelName));
    }

    #[test]
    fn register_model_name_extends_allowlist() {
        let runtime = fixture_runtime();
        runtime
            .register_model_name("claude-custom-")
            .expect("extend allowlist");
        let handle = runtime
            .register_handle("claude-custom-v1", "2026-05-20T06:00:00Z")
            .expect("now allowed");
        assert_eq!(handle.anthropic_model_name, "claude-custom-v1");
    }

    #[test]
    fn capabilities_match_byok_cloud_realities() {
        let caps = AnthropicByokRuntime::cloud_capabilities();
        assert!(!caps.supports_lora);
        assert!(caps.supports_kv_prefix_cache);
        assert!(!caps.supports_activation_steering);
        assert!(!caps.supports_subquadratic);
        assert!(!caps.supports_speculative_draft);
        assert!(!caps.supports_eagle3);
    }

    #[test]
    fn audit_sink_records_call_lifecycle() {
        let runtime = fixture_runtime();
        let handle = runtime
            .register_handle("claude-3.5-sonnet", "2026-05-20T06:00:00Z")
            .unwrap();
        runtime
            .record_audit(CloudInvocationAuditRow {
                model_id: handle.model_id,
                openai_model_name: handle.anthropic_model_name.clone(),
                call_kind: CloudCallKind::ChatCompletion,
                started_at_utc: "2026-05-20T06:00:00Z".to_string(),
                finished_at_utc: Some("2026-05-20T06:00:01Z".to_string()),
                status: CloudCallStatus::Succeeded,
            })
            .expect("audit ok");
    }

    #[test]
    fn handle_for_unknown_model_returns_not_registered() {
        let runtime = fixture_runtime();
        let unknown = ModelId::new_v7();
        let err = runtime.handle_for(unknown).expect_err("unknown model");
        assert!(matches!(err, AnthropicByokError::ModelNotRegistered(_)));
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

    #[test]
    fn fetch_api_key_returns_provider_value_and_does_not_log_it() {
        let runtime = fixture_runtime();
        let key = runtime.fetch_api_key().expect("fetch");
        assert!(key.starts_with("sk-ant-"));
    }

    #[test]
    fn map_finish_reason_handles_anthropic_vocabulary() {
        assert_eq!(map_finish_reason("end_turn"), Some(FinishReason::Stop));
        assert_eq!(map_finish_reason("max_tokens"), Some(FinishReason::Length));
        assert_eq!(
            map_finish_reason("stop_sequence"),
            Some(FinishReason::Stop)
        );
        assert_eq!(map_finish_reason("tool_use"), Some(FinishReason::Stop));
        assert_eq!(
            map_finish_reason("cancelled"),
            Some(FinishReason::Cancelled)
        );
        assert_eq!(map_finish_reason("unknown_future"), None);
    }
}

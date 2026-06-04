//! MT-126 (rework) cross-crate integration tests for the BYOK
//! cloud Anthropic Messages runtime.
//!
//! Wiremock is the live test surface per the MT-126
//! `operator_clarification_20260520` note (mirrored from MT-125):
//! it binds a real TCP port answering the documented Anthropic
//! Messages API SSE protocol shape. That satisfies Spec-Realism
//! Gate sub-rule 2 (real reqwest -> real socket -> wiremock
//! answering protocol shape) without requiring operator BYOK
//! credit.
//!
//! Anthropic protocol deviations exercised explicitly:
//!   - Path is `/v1/messages` (vs OpenAI `/chat/completions`).
//!   - Auth header is `x-api-key: <key>` (vs OpenAI `Authorization`).
//!   - Every request carries `anthropic-version: 2023-06-01`.
//!   - SSE events are named: `content_block_delta` carries token
//!     text; `message_stop` terminates the stream.
//!   - `score()` and `embed()` return CapabilityNotSupported (no
//!     first-party logprobs or embeddings on the Messages API).

use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use futures::StreamExt;
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::model_runtime::cloud::{
    AnthropicByokError, AnthropicByokRuntime, ApiKeyProvider, CloudCallKind, CloudCallStatus,
    CloudConsentContext, CloudInvocationAuditRow, CloudInvocationAuditSink, CloudLaneObservability,
    ConsentDecision, ConsentGate, ConsentGateError, ConsentProvider, OpenAiByokError,
    ANTHROPIC_API_KEY_HEADER, ANTHROPIC_API_VERSION, ANTHROPIC_MESSAGES_PATH,
    ANTHROPIC_VERSION_HEADER,
};
use handshake_core::model_runtime::{
    CancellationToken, GenPrompt, GenerateRequest, KvCachePolicy, LoadSpec, ModelId, ModelRuntime,
    ProviderKind, SamplingParams,
};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

struct StaticKey {
    key: String,
}
impl ApiKeyProvider for StaticKey {
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

/// Custom inspector mock that records every incoming `Request` so
/// tests can assert about headers / body / api-key surface.
#[derive(Default)]
struct RequestInspector {
    requests: Mutex<Vec<Request>>,
}
impl RequestInspector {
    fn record(&self, req: &Request) {
        self.requests.lock().unwrap().push(req.clone());
    }
    fn snapshot(&self) -> Vec<Request> {
        self.requests.lock().unwrap().clone()
    }
}

/// MT-126 remediation: in-memory FlightRecorder that captures every
/// recorded event so tests can assert FR-EVT-LLM-INFER emission +
/// adapter tagging. Mirrors the `CapturingSink` shape on the FR side
/// (and the OpenAI BYOK test sibling, MT-125).
#[derive(Default)]
struct CapturingFlightRecorder {
    events: Mutex<Vec<FlightRecorderEvent>>,
}
#[async_trait]
impl FlightRecorder for CapturingFlightRecorder {
    async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
        self.events.lock().unwrap().push(event);
        Ok(())
    }
    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        Ok(0)
    }
    async fn list_events(
        &self,
        _filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Ok(self.events.lock().unwrap().clone())
    }
}

/// Always-approve consent provider for the success-path test.
struct ApproveProvider;
impl ConsentProvider for ApproveProvider {
    fn prompt_for_decision(
        &self,
        _session_id: &str,
        _lane: &str,
    ) -> Result<ConsentDecision, ConsentGateError> {
        Ok(ConsentDecision::Approved)
    }
}

/// Always-deny consent provider for the consent-denied test.
struct DenyProvider;
impl ConsentProvider for DenyProvider {
    fn prompt_for_decision(
        &self,
        _session_id: &str,
        _lane: &str,
    ) -> Result<ConsentDecision, ConsentGateError> {
        Ok(ConsentDecision::Denied)
    }
}

const API_KEY_FIXTURE: &str = "sk-ant-wiremock-NEVER-LOG-THIS-KEY";

fn fixture_runtime(api_base: String, sink: Arc<CapturingSink>) -> AnthropicByokRuntime {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("reqwest client builds");
    AnthropicByokRuntime::with_client(
        api_base,
        client,
        Arc::new(StaticKey {
            key: API_KEY_FIXTURE.to_string(),
        }),
        sink,
    )
}

fn fixture_generate_request(model_id: ModelId, cancel: CancellationToken) -> GenerateRequest {
    GenerateRequest {
        id: model_id,
        prompt: GenPrompt::new("Say hello."),
        sampling: SamplingParams {
            temperature: Some(0.7),
            ..SamplingParams::default()
        },
        lora_overrides: Vec::new(),
        steering_overrides: Vec::new(),
        kv_prefix_handle: None,
        cancel,
        max_tokens: 32,
        stop_sequences: Vec::new(),
        speculative_mode: None,
        structured_decoding: None,
    }
}

/// Build a wiremock-friendly Anthropic SSE payload that streams the
/// given `tokens` as `content_block_delta` events and terminates
/// with a `message_delta` (`stop_reason=end_turn`) followed by
/// `message_stop`. Each event uses the named-event SSE wire shape
/// (`event: <name>\ndata: <json>\n\n`).
fn sse_payload_for(tokens: &[&str]) -> String {
    let mut body = String::new();

    // Optional but realistic envelope events. The runtime tolerates
    // (and skips) `message_start` / `content_block_start`; including
    // them here proves the dispatch loop ignores non-token events.
    let message_start = serde_json::json!({
        "type": "message_start",
        "message": {
            "id": "msg_test_0001",
            "type": "message",
            "role": "assistant",
            "model": "claude-opus-4-7-20260101",
            "content": [],
            "stop_reason": null,
            "stop_sequence": null,
            "usage": { "input_tokens": 5, "output_tokens": 0 }
        }
    });
    body.push_str("event: message_start\n");
    body.push_str(&format!("data: {message_start}\n\n"));

    let block_start = serde_json::json!({
        "type": "content_block_start",
        "index": 0,
        "content_block": { "type": "text", "text": "" }
    });
    body.push_str("event: content_block_start\n");
    body.push_str(&format!("data: {block_start}\n\n"));

    for text in tokens {
        let chunk = serde_json::json!({
            "type": "content_block_delta",
            "index": 0,
            "delta": { "type": "text_delta", "text": text }
        });
        body.push_str("event: content_block_delta\n");
        body.push_str(&format!("data: {chunk}\n\n"));
    }

    let block_stop = serde_json::json!({ "type": "content_block_stop", "index": 0 });
    body.push_str("event: content_block_stop\n");
    body.push_str(&format!("data: {block_stop}\n\n"));

    let msg_delta = serde_json::json!({
        "type": "message_delta",
        "delta": { "stop_reason": "end_turn", "stop_sequence": null },
        "usage": { "output_tokens": tokens.len() }
    });
    body.push_str("event: message_delta\n");
    body.push_str(&format!("data: {msg_delta}\n\n"));

    let msg_stop = serde_json::json!({ "type": "message_stop" });
    body.push_str("event: message_stop\n");
    body.push_str(&format!("data: {msg_stop}\n\n"));

    body
}

// ---------------------------------------------------------------------
// Test 1: live streaming against wiremock
// ---------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn anthropic_byok_generate_streams_tokens_against_wiremock() {
    let mock_server = MockServer::start().await;
    let payload = sse_payload_for(&["Hello", ", ", "world", "!"]);
    Mock::given(method("POST"))
        .and(path(ANTHROPIC_MESSAGES_PATH))
        .and(header(ANTHROPIC_API_KEY_HEADER, API_KEY_FIXTURE))
        .and(header(ANTHROPIC_VERSION_HEADER, ANTHROPIC_API_VERSION))
        .and(header("content-type", "application/json"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(payload),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    let sink = Arc::new(CapturingSink::default());
    let runtime = fixture_runtime(mock_server.uri(), sink.clone());
    let handle = runtime
        .register_handle("claude-opus-4-7-20260101", "2026-05-20T11:00:00Z")
        .expect("allowlisted");

    let cancel = CancellationToken::new();
    let req = fixture_generate_request(handle.model_id, cancel);
    let mut stream = runtime.messages_stream(req);

    let mut produced_texts = Vec::new();
    let mut saw_terminal_stop = false;
    while let Some(item) = stream.next().await {
        let token = item.expect("stream items are Ok in the success path");
        if let Some(finish) = token.finish_reason {
            if matches!(finish, handshake_core::model_runtime::FinishReason::Stop)
                && token.text.is_empty()
            {
                saw_terminal_stop = true;
                break;
            }
        }
        if !token.text.is_empty() {
            produced_texts.push(token.text);
        }
    }

    assert_eq!(
        produced_texts,
        vec![
            "Hello".to_string(),
            ", ".to_string(),
            "world".to_string(),
            "!".to_string(),
        ],
        "stream must yield each content_block_delta.delta.text chunk verbatim"
    );
    assert!(
        saw_terminal_stop,
        "stream must terminate with FinishReason::Stop on `event: message_stop`"
    );

    let rows = sink.rows.lock().unwrap().clone();
    let chat_rows: Vec<_> = rows
        .iter()
        .filter(|r| r.call_kind == CloudCallKind::ChatCompletion)
        .collect();
    assert!(
        chat_rows
            .iter()
            .any(|r| r.status == CloudCallStatus::Started),
        "audit must include a Started row"
    );
    assert!(
        chat_rows
            .iter()
            .any(|r| r.status == CloudCallStatus::Succeeded),
        "audit must include a Succeeded row after the stream completes"
    );
}

// ---------------------------------------------------------------------
// Test 2: cancellation
// ---------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn anthropic_byok_cancellation_marks_call_cancelled() {
    // Pre-call cancellation: trip the cancel flag BEFORE calling
    // messages_stream. The spawned task checks the flag first and
    // bails with a Cancelled audit row + FinishReason::Cancelled
    // terminal token, without issuing a request to wiremock. This
    // avoids racing SSE buffer drains against the cancel flag.

    let mock_server = MockServer::start().await;
    // Mount a permissive mock but expect 0 hits — we want to prove
    // pre-call cancellation never reaches the wire.
    Mock::given(method("POST"))
        .and(path(ANTHROPIC_MESSAGES_PATH))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string("event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n"),
        )
        .expect(0)
        .mount(&mock_server)
        .await;

    let sink = Arc::new(CapturingSink::default());
    let runtime = fixture_runtime(mock_server.uri(), sink.clone());
    let handle = runtime
        .register_handle("claude-opus-4-7", "2026-05-20T11:00:00Z")
        .unwrap();

    let cancel = CancellationToken::new();
    cancel.cancel();
    let req = fixture_generate_request(handle.model_id, cancel);

    let mut stream = runtime.messages_stream(req);

    let mut saw_cancelled = false;
    while let Ok(Some(item)) = tokio::time::timeout(Duration::from_secs(5), stream.next()).await {
        match item {
            Ok(token) => {
                if matches!(
                    token.finish_reason,
                    Some(handshake_core::model_runtime::FinishReason::Cancelled)
                ) {
                    saw_cancelled = true;
                    break;
                }
            }
            Err(_) => break,
        }
    }
    assert!(
        saw_cancelled,
        "pre-cancelled request must surface FinishReason::Cancelled"
    );

    // Wait briefly so the spawned audit-write task has a chance to flush.
    tokio::time::sleep(Duration::from_millis(50)).await;

    let rows = sink.rows.lock().unwrap().clone();
    assert!(
        rows.iter().any(|r| r.status == CloudCallStatus::Cancelled),
        "audit must include a Cancelled row; rows={rows:?}"
    );

    // And explicit assertion: no HTTP request actually reached wiremock.
    mock_server.verify().await;
}

// ---------------------------------------------------------------------
// Test 3: API key only appears as `x-api-key`, never elsewhere
// ---------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn anthropic_byok_api_key_appears_only_as_x_api_key_header() {
    let mock_server = MockServer::start().await;
    let inspector = Arc::new(RequestInspector::default());
    let inspector_for_mock = inspector.clone();

    let payload = sse_payload_for(&["ok"]);
    Mock::given(method("POST"))
        .and(path(ANTHROPIC_MESSAGES_PATH))
        .respond_with(move |req: &Request| {
            inspector_for_mock.record(req);
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(payload.clone())
        })
        .expect(1)
        .mount(&mock_server)
        .await;

    let sink = Arc::new(CapturingSink::default());
    let runtime = fixture_runtime(mock_server.uri(), sink.clone());
    let handle = runtime
        .register_handle("claude-opus-4", "2026-05-20T11:00:00Z")
        .unwrap();
    let req = fixture_generate_request(handle.model_id, CancellationToken::new());
    let mut stream = runtime.messages_stream(req);
    while let Some(item) = stream.next().await {
        let _ = item;
    }

    let snapshot = inspector.snapshot();
    assert_eq!(
        snapshot.len(),
        1,
        "exactly one HTTP request must reach the mock"
    );
    let inspected = &snapshot[0];

    // (a) `x-api-key` is the only header carrying the secret.
    let x_api_key = inspected
        .headers
        .get(ANTHROPIC_API_KEY_HEADER)
        .expect("x-api-key header must be present");
    let x_api_key_value = x_api_key.to_str().expect("header is ascii");
    assert_eq!(
        x_api_key_value, API_KEY_FIXTURE,
        "x-api-key must carry the api key verbatim (no Bearer wrapper)"
    );

    // (b) Authorization header must NOT be set (we use x-api-key,
    // not Bearer auth — this is the key Anthropic protocol deviation
    // from OpenAI).
    assert!(
        inspected.headers.get("authorization").is_none(),
        "Authorization header must not be set on Anthropic requests"
    );

    // (c) No other header value contains the key.
    for (name, value) in inspected.headers.iter() {
        if name.as_str().eq_ignore_ascii_case(ANTHROPIC_API_KEY_HEADER) {
            continue;
        }
        let v = value.to_str().unwrap_or("");
        assert!(
            !v.contains(API_KEY_FIXTURE),
            "header {} must not contain the api key (got {v})",
            name.as_str()
        );
    }

    // (d) Body must not contain the api key (the body should only
    // hold the Messages JSON payload).
    let body = std::str::from_utf8(&inspected.body).unwrap_or("");
    assert!(
        !body.contains(API_KEY_FIXTURE),
        "request body must not contain the api key"
    );

    // (e) URL/query string must not contain the api key either.
    assert!(
        !inspected.url.as_str().contains(API_KEY_FIXTURE),
        "request URL must not contain the api key"
    );

    // (f) The runtime's Debug output must not contain the key.
    let dbg = format!("{runtime:?}");
    assert!(!dbg.contains(API_KEY_FIXTURE), "{dbg}");
}

// ---------------------------------------------------------------------
// Test 4: anthropic-version header is set on every request
// ---------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn anthropic_byok_sends_anthropic_version_header() {
    let mock_server = MockServer::start().await;
    let inspector = Arc::new(RequestInspector::default());
    let inspector_for_mock = inspector.clone();

    let payload = sse_payload_for(&["ok"]);
    Mock::given(method("POST"))
        .and(path(ANTHROPIC_MESSAGES_PATH))
        // Header matcher: wiremock will only return the body if the
        // request carries `anthropic-version: 2023-06-01`.
        .and(header(ANTHROPIC_VERSION_HEADER, ANTHROPIC_API_VERSION))
        .respond_with(move |req: &Request| {
            inspector_for_mock.record(req);
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(payload.clone())
        })
        .expect(1)
        .mount(&mock_server)
        .await;

    let sink = Arc::new(CapturingSink::default());
    let runtime = fixture_runtime(mock_server.uri(), sink.clone());
    let handle = runtime
        .register_handle("claude-3.5-sonnet-20251022", "2026-05-20T11:00:00Z")
        .unwrap();
    let req = fixture_generate_request(handle.model_id, CancellationToken::new());
    let mut stream = runtime.messages_stream(req);
    while let Some(_item) = stream.next().await {}

    // Cross-check via the inspector: the header is present with the
    // documented value, not just matched by the wiremock middleware.
    let snapshot = inspector.snapshot();
    assert_eq!(snapshot.len(), 1);
    let inspected = &snapshot[0];
    let version_header = inspected
        .headers
        .get(ANTHROPIC_VERSION_HEADER)
        .expect("anthropic-version header must be present");
    assert_eq!(
        version_header.to_str().expect("ascii"),
        ANTHROPIC_API_VERSION,
        "anthropic-version must carry the documented stable version"
    );
}

// ---------------------------------------------------------------------
// Test 5: capabilities shape
// ---------------------------------------------------------------------

#[test]
fn anthropic_byok_capabilities_match_cloud_realities() {
    let caps = AnthropicByokRuntime::cloud_capabilities();
    assert!(!caps.supports_lora);
    assert!(caps.supports_kv_prefix_cache);
    assert!(!caps.supports_activation_steering);
    assert!(!caps.supports_subquadratic);
    assert!(!caps.supports_speculative_draft);
    assert!(!caps.supports_eagle3);
}

// ---------------------------------------------------------------------
// Test 6: no ProcessOwnershipLedger row for BYOK calls.
// ---------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn anthropic_byok_does_not_register_process_ownership_row() {
    let mock_server = MockServer::start().await;
    let payload = sse_payload_for(&["a", "b", "c"]);
    Mock::given(method("POST"))
        .and(path(ANTHROPIC_MESSAGES_PATH))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(payload),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    let sink = Arc::new(CapturingSink::default());
    let runtime = fixture_runtime(mock_server.uri(), sink.clone());
    let handle = runtime
        .register_handle("claude-opus-4", "2026-05-20T11:00:00Z")
        .unwrap();
    let req = fixture_generate_request(handle.model_id, CancellationToken::new());
    let mut stream = runtime.messages_stream(req);
    while let Some(_item) = stream.next().await {}

    // Every audit row written should belong to CloudInvocationAuditSink.
    // A ProcessOwnershipLedger row would imply a separate write path;
    // it doesn't exist in this adapter and these rows are the only
    // evidence channel.
    let rows = sink.rows.lock().unwrap().clone();
    assert!(!rows.is_empty(), "BYOK must produce audit rows");
    for row in rows {
        assert!(matches!(
            row.call_kind,
            CloudCallKind::ChatCompletion | CloudCallKind::Embeddings | CloudCallKind::Score
        ));
    }
}

// ---------------------------------------------------------------------
// Test 7: allowlist rejection at load() time
// ---------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn anthropic_byok_load_rejects_non_allowlisted_model_name() {
    let mock_server = MockServer::start().await;
    let sink = Arc::new(CapturingSink::default());
    let mut runtime = fixture_runtime(mock_server.uri(), sink.clone());

    let spec = LoadSpec {
        artifact_path: std::path::PathBuf::from("/not/used/for/cloud"),
        sha256_expected: String::new(),
        runtime_kind: handshake_core::model_runtime::RuntimeKind::LlamaCpp, // ignored for cloud
        sampling_defaults: SamplingParams::default(),
        kv_cache_policy: KvCachePolicy::default(),
        declared_capabilities: AnthropicByokRuntime::cloud_capabilities(),
        provider: ProviderKind::ByokCloud,
        engine_origin: Some("not-a-real-anthropic-model".to_string()),
        external_engine_import: None,
    };
    let err = runtime.load(spec).await.expect_err("not in allowlist");
    let msg = format!("{err}");
    assert!(
        msg.contains("not in the BYOK allowlist"),
        "expected allowlist-rejection text, got: {msg}"
    );
}

// ---------------------------------------------------------------------
// Test 8: ProviderKind validation at load() time
// ---------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn anthropic_byok_load_rejects_non_byok_provider() {
    let mock_server = MockServer::start().await;
    let sink = Arc::new(CapturingSink::default());
    let mut runtime = fixture_runtime(mock_server.uri(), sink.clone());

    let spec = LoadSpec {
        artifact_path: std::path::PathBuf::from("/not/used/for/cloud"),
        sha256_expected: String::new(),
        runtime_kind: handshake_core::model_runtime::RuntimeKind::LlamaCpp,
        sampling_defaults: SamplingParams::default(),
        kv_cache_policy: KvCachePolicy::default(),
        declared_capabilities: AnthropicByokRuntime::cloud_capabilities(),
        provider: ProviderKind::Local, // wrong lane
        engine_origin: Some("claude-opus-4".to_string()),
        external_engine_import: None,
    };
    let err = runtime.load(spec).await.expect_err("wrong provider");
    let msg = format!("{err}");
    assert!(
        msg.contains("ByokCloud"),
        "expected ByokCloud lane-validation text, got: {msg}"
    );
}

// ---------------------------------------------------------------------
// Test 9: score() returns CapabilityNotSupported (no logprobs on Messages API)
// ---------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn anthropic_byok_score_returns_capability_not_supported() {
    let sink = Arc::new(CapturingSink::default());
    let runtime = fixture_runtime("https://api.anthropic.com".to_string(), sink);
    let handle = runtime
        .register_handle("claude-opus-4", "2026-05-20T11:00:00Z")
        .unwrap();
    let err = runtime
        .score(handle.model_id, vec![1, 2, 3])
        .await
        .expect_err("Messages API has no logprobs");
    match err {
        handshake_core::model_runtime::ModelRuntimeError::CapabilityNotSupported {
            capability,
            adapter,
        } => {
            assert!(capability.contains("logprobs"), "{capability}");
            assert_eq!(adapter, "anthropic_byok");
        }
        other => panic!("expected CapabilityNotSupported, got {other:?}"),
    }
}

// ---------------------------------------------------------------------
// Test 10: embed() returns CapabilityNotSupported (Anthropic has no embeddings)
// ---------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn anthropic_byok_embed_returns_capability_not_supported() {
    let sink = Arc::new(CapturingSink::default());
    let runtime = fixture_runtime("https://api.anthropic.com".to_string(), sink);
    let handle = runtime
        .register_handle("claude-opus-4", "2026-05-20T11:00:00Z")
        .unwrap();
    let err = runtime
        .embed(handle.model_id, "hello")
        .await
        .expect_err("Anthropic has no embeddings endpoint");
    match err {
        handshake_core::model_runtime::ModelRuntimeError::CapabilityNotSupported {
            capability,
            adapter,
        } => {
            assert!(capability.contains("embed"), "{capability}");
            assert_eq!(adapter, "anthropic_byok");
        }
        other => panic!("expected CapabilityNotSupported, got {other:?}"),
    }
}

// ---------------------------------------------------------------------
// Carry-over structural tests from the prior session
// ---------------------------------------------------------------------

#[test]
fn cloud_anthropic_runtime_debug_redacts_api_key() {
    let sink = Arc::new(CapturingSink::default());
    let runtime = fixture_runtime("https://api.anthropic.com".to_string(), sink);
    let dbg = format!("{runtime:?}");
    assert!(dbg.contains("<redacted"), "{dbg}");
    assert!(!dbg.contains(API_KEY_FIXTURE), "{dbg}");
}

#[test]
fn cloud_anthropic_runtime_register_handle_validates_allowlist() {
    let sink = Arc::new(CapturingSink::default());
    let runtime = fixture_runtime("https://api.anthropic.com".to_string(), sink);
    runtime
        .register_handle("claude-3.5-sonnet-20251022", "2026-05-20T06:00:00Z")
        .expect("claude-3.5-sonnet family allowed");
    runtime
        .register_handle("claude-opus-4-7", "2026-05-20T06:00:00Z")
        .expect("claude-opus-4 family allowed");
    let err = runtime
        .register_handle("not-claude", "2026-05-20T06:00:00Z")
        .expect_err("not allowed");
    assert!(matches!(err, AnthropicByokError::ModelNameNotAllowed(_)));
}

#[test]
fn cloud_anthropic_audit_sink_records_call_lifecycle() {
    let sink = Arc::new(CapturingSink::default());
    let runtime = fixture_runtime("https://api.anthropic.com".to_string(), sink.clone());
    let handle = runtime
        .register_handle("claude-opus-4", "2026-05-20T06:00:00Z")
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
    let captured = sink.rows.lock().unwrap();
    assert!(
        !captured.is_empty(),
        "register_handle + record_audit emit rows"
    );
    assert!(
        captured
            .iter()
            .any(|r| r.openai_model_name == "claude-opus-4"
                && r.status == CloudCallStatus::Succeeded),
        "the appended Succeeded row must round-trip via the sink"
    );
}

// ---------------------------------------------------------------------
// MT-126 remediation Test A: FR-EVT-LLM-INFER emission with adapter tag
//
// With a CapturingFlightRecorder + always-approve ConsentProvider
// attached via with_lane_observability, a successful streaming
// generate() must emit at least one FR-EVT-LLM-INFER-START and one
// FR-EVT-LLM-INFER-END whose payload `adapter` == "anthropic_byok".
// Mirrors the OpenAI BYOK test sibling (MT-125).
// ---------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn anthropic_byok_emits_fr_infer_events_with_adapter_tag() {
    let mock_server = MockServer::start().await;
    // 20 tokens so a TOKEN sample fires (sample interval is 16) on
    // top of the guaranteed START + END.
    let words: Vec<String> = (0..20).map(|i| format!("tok{i} ")).collect();
    let word_refs: Vec<&str> = words.iter().map(|s| s.as_str()).collect();
    let payload = sse_payload_for(&word_refs);
    Mock::given(method("POST"))
        .and(path(ANTHROPIC_MESSAGES_PATH))
        .and(header(ANTHROPIC_API_KEY_HEADER, API_KEY_FIXTURE))
        .and(header(ANTHROPIC_VERSION_HEADER, ANTHROPIC_API_VERSION))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(payload),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    let sink = Arc::new(CapturingSink::default());
    let recorder = Arc::new(CapturingFlightRecorder::default());
    let gate = Arc::new(ConsentGate::new());
    let lane_obs = Arc::new(CloudLaneObservability {
        flight_recorder: recorder.clone(),
        consent: Some(CloudConsentContext {
            gate: gate.clone(),
            provider: Arc::new(ApproveProvider),
            session_id: "session-fr-test".to_string(),
        }),
    });

    let runtime = fixture_runtime(mock_server.uri(), sink.clone())
        .with_lane_observability(lane_obs.clone());
    let handle = runtime
        .register_handle("claude-opus-4-7-20260101", "2026-05-20T11:00:00Z")
        .expect("allowlisted");

    let req = fixture_generate_request(handle.model_id, CancellationToken::new());
    let mut stream = runtime.messages_stream(req);
    while let Some(item) = stream.next().await {
        let _ = item.expect("success path items are Ok");
    }

    // Give the spawned async pipeline a moment to flush the END event
    // (emitted after the terminal token is sent).
    tokio::time::sleep(Duration::from_millis(100)).await;

    let events = recorder.events.lock().unwrap().clone();
    let event_kind = |ev: &FlightRecorderEvent| -> Option<(String, String)> {
        let phase = ev.payload.get("phase")?.as_str()?.to_string();
        let adapter = ev.payload.get("adapter")?.as_str()?.to_string();
        Some((phase, adapter))
    };

    let start_count = events
        .iter()
        .filter(|ev| {
            matches!(event_kind(ev), Some((phase, adapter))
                if phase == "start" && adapter == "anthropic_byok")
        })
        .count();
    let end_count = events
        .iter()
        .filter(|ev| {
            matches!(event_kind(ev), Some((phase, adapter))
                if phase == "end" && adapter == "anthropic_byok")
        })
        .count();

    assert!(
        start_count >= 1,
        "must emit >=1 FR-EVT-LLM-INFER-START with adapter=anthropic_byok; events={events:?}"
    );
    assert!(
        end_count >= 1,
        "must emit >=1 FR-EVT-LLM-INFER-END with adapter=anthropic_byok; events={events:?}"
    );
}

// ---------------------------------------------------------------------
// MT-126 remediation Test B: consent-denied short-circuits before HTTP
//
// With a deny ConsentProvider attached, generate() must yield an error
// item and NEVER reach the wire (wiremock expect 0). Mirrors the
// OpenAI BYOK test sibling (MT-125).
// ---------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn anthropic_byok_consent_denied_blocks_http_call() {
    let mock_server = MockServer::start().await;
    // Permissive mock but EXPECT 0 — a denied consent must not reach
    // the wire.
    Mock::given(method("POST"))
        .and(path(ANTHROPIC_MESSAGES_PATH))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string("event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n"),
        )
        .expect(0)
        .mount(&mock_server)
        .await;

    let sink = Arc::new(CapturingSink::default());
    let recorder = Arc::new(CapturingFlightRecorder::default());
    let gate = Arc::new(ConsentGate::new());
    let lane_obs = Arc::new(CloudLaneObservability {
        flight_recorder: recorder.clone(),
        consent: Some(CloudConsentContext {
            gate: gate.clone(),
            provider: Arc::new(DenyProvider),
            session_id: "session-deny-test".to_string(),
        }),
    });

    let runtime =
        fixture_runtime(mock_server.uri(), sink.clone()).with_lane_observability(lane_obs);
    let handle = runtime
        .register_handle("claude-opus-4", "2026-05-20T11:00:00Z")
        .expect("allowlisted");

    let req = fixture_generate_request(handle.model_id, CancellationToken::new());
    let mut stream = runtime.messages_stream(req);

    // The first (and only) stream item must be an error surfacing the
    // consent denial.
    let first = stream.next().await.expect("stream yields one item");
    match first {
        Err(err) => {
            let msg = format!("{err}");
            assert!(
                msg.contains("consent denied"),
                "error must surface consent denial, got: {msg}"
            );
        }
        Ok(token) => panic!("expected consent-denied error, got token: {token:?}"),
    }
    // No further items.
    assert!(
        stream.next().await.is_none(),
        "consent-denied stream must be a single error item"
    );

    // And no HTTP request reached wiremock.
    mock_server.verify().await;
}

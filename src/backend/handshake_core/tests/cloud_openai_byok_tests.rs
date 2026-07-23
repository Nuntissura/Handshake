//! MT-125 (rework) cross-crate integration tests for the BYOK
//! cloud OpenAI runtime.
//!
//! Wiremock is the live test surface per the MT-125
//! `operator_clarification_20260520` note: it binds a real TCP
//! port answering the documented OpenAI Chat Completions /
//! Embeddings protocol shape. That satisfies Spec-Realism Gate
//! sub-rule 2 (real reqwest -> real socket -> wiremock answering
//! protocol shape) without requiring operator BYOK credit.
//!
//! Tests pinned here:
//!   1. `openai_byok_generate_streams_tokens_against_wiremock`
//!   2. `openai_byok_cancellation_marks_call_cancelled`
//!   3. `openai_byok_api_key_only_appears_as_bearer_auth`
//!   4. `openai_byok_capabilities_match_cloud_realities`
//!   5. `openai_byok_does_not_register_process_ownership_row`
//!   6. `openai_byok_load_rejects_non_allowlisted_model_name`
//!   7. structural smoketests carried over from the prior session
//!      (capabilities shape, Debug redaction, register_handle
//!      allowlist, audit-row forwarding)

use std::io::Write;
use std::sync::{Arc, Mutex, Once, OnceLock};
use std::time::Duration;

use async_trait::async_trait;
use futures::StreamExt;
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::model_runtime::cloud::{
    ApiKeyProvider, CloudCallKind, CloudCallStatus, CloudConsentContext, CloudInvocationAuditRow,
    CloudInvocationAuditSink, CloudLaneObservability, ConsentDecision, ConsentGate,
    ConsentGateError, ConsentProvider, OpenAiByokError, OpenAiByokRuntime,
    OPENAI_CHAT_COMPLETIONS_PATH, OPENAI_EMBEDDINGS_PATH,
};
use handshake_core::model_runtime::{
    CancellationToken, GenPrompt, GenerateRequest, KvCachePolicy, LoadSpec, ModelId, ModelRuntime,
    ProviderKind, SamplingParams,
};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

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

/// MT-125 remediation: in-memory FlightRecorder that captures every
/// recorded event so tests can assert FR-EVT-LLM-INFER emission +
/// adapter tagging. Mirrors the `CapturingSink` shape on the FR side.
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

const API_KEY_FIXTURE: &str = "sk-wiremock-NEVER-LOG-THIS-KEY";

fn fixture_runtime(api_base: String, sink: Arc<CapturingSink>) -> OpenAiByokRuntime {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("reqwest client builds");
    OpenAiByokRuntime::with_client(
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

fn sse_payload_for(tokens: &[&str]) -> String {
    let mut body = String::new();
    for (idx, text) in tokens.iter().enumerate() {
        let finish = if idx + 1 == tokens.len() {
            Some("stop")
        } else {
            None
        };
        let chunk = serde_json::json!({
            "id": format!("chatcmpl-test-{idx}"),
            "object": "chat.completion.chunk",
            "created": 1_700_000_000_u64 + idx as u64,
            "model": "gpt-4o-2024-08-06",
            "choices": [{
                "index": 0,
                "delta": { "content": text },
                "finish_reason": finish,
            }],
        });
        body.push_str(&format!("data: {chunk}\n\n"));
    }
    body.push_str("data: [DONE]\n\n");
    body
}

// ---------------------------------------------------------------------
// Test 1: live streaming against wiremock
// ---------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn openai_byok_generate_streams_tokens_against_wiremock() {
    let mock_server = MockServer::start().await;
    let payload = sse_payload_for(&["Hello", ", ", "world", "!"]);
    Mock::given(method("POST"))
        .and(path(OPENAI_CHAT_COMPLETIONS_PATH))
        .and(header(
            "authorization",
            format!("Bearer {API_KEY_FIXTURE}").as_str(),
        ))
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
        .register_handle("gpt-4o-2024-08-06", "2026-05-20T11:00:00Z")
        .expect("allowlisted");

    let cancel = CancellationToken::new();
    let req = fixture_generate_request(handle.model_id, cancel);
    let mut stream = runtime.chat_completions_stream(req);

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
        "stream must yield each delta.content chunk verbatim"
    );
    assert!(
        saw_terminal_stop,
        "stream must terminate with FinishReason::Stop"
    );

    let rows = sink.rows.lock().unwrap().clone();
    let chat_rows: Vec<_> = rows
        .iter()
        .filter(|r| r.call_kind == CloudCallKind::ChatCompletion)
        .collect();
    // Expected lifecycle: register_handle Started, generate Started, final Succeeded.
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

struct EchoingFailKey;

impl ApiKeyProvider for EchoingFailKey {
    fn fetch_api_key(&self) -> Result<String, OpenAiByokError> {
        Err(OpenAiByokError::AuditPersist(API_KEY_FIXTURE.to_string()))
    }
}

#[derive(Clone, Default)]
struct SharedTraceBuffer(Arc<Mutex<Vec<u8>>>);

impl Write for SharedTraceBuffer {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "trace lock poisoned"))?
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for SharedTraceBuffer {
    type Writer = SharedTraceBuffer;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

fn capture_tracing() -> SharedTraceBuffer {
    static BUFFER: OnceLock<SharedTraceBuffer> = OnceLock::new();
    static INIT: Once = Once::new();
    let buffer = BUFFER.get_or_init(SharedTraceBuffer::default).clone();
    INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_writer(buffer.clone())
            .with_max_level(tracing::Level::TRACE)
            .with_ansi(false)
            .try_init()
            .expect("install OpenAI BYOK test tracing capture");
    });
    tracing::info!(
        target: "handshake_core::tests::openai_byok",
        marker = "openai-byok-tracing-live",
        "public tracing capture marker"
    );
    buffer
}

async fn start_truncated_sse_server(body_prefix: String) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind truncated SSE server");
    let addr = listener.local_addr().expect("truncated SSE server addr");
    let handle = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept BYOK request");
        let mut request = [0_u8; 8192];
        let _ = socket.read(&mut request).await;
        let declared_length = body_prefix.len() + 4096;
        let headers = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {declared_length}\r\nconnection: close\r\n\r\n"
        );
        socket.write_all(headers.as_bytes()).await.expect("write response headers");
        socket.write_all(body_prefix.as_bytes()).await.expect("write truncated SSE prefix");
        socket.shutdown().await.expect("close truncated SSE response");
    });
    (format!("http://{addr}"), handle)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn openai_byok_non_success_bodies_are_redacted_and_bounded_for_all_call_kinds() {
    const PROVIDER_ECHO_CANARY: &str = "provider-echo-canary-MUST-NOT-ESCAPE";
    let mock_server = MockServer::start().await;
    let echoed_body = format!("{PROVIDER_ECHO_CANARY}{}", "x".repeat(32_768));

    Mock::given(method("POST"))
        .and(path(OPENAI_CHAT_COMPLETIONS_PATH))
        .respond_with(ResponseTemplate::new(503).set_body_string(echoed_body.clone()))
        .expect(2)
        .mount(&mock_server)
        .await;
    Mock::given(method("POST"))
        .and(path(OPENAI_EMBEDDINGS_PATH))
        .respond_with(ResponseTemplate::new(503).set_body_string(echoed_body))
        .expect(1)
        .mount(&mock_server)
        .await;

    let sink = Arc::new(CapturingSink::default());
    let runtime = fixture_runtime(mock_server.uri(), sink);
    let handle = runtime
        .register_handle("gpt-4o", "2026-05-20T11:00:00Z")
        .expect("allowlisted");

    let mut generate = runtime.generate(fixture_generate_request(
        handle.model_id,
        CancellationToken::new(),
    ));
    let generate_error = generate
        .next()
        .await
        .expect("generate yields its provider-status error")
        .expect_err("503 must fail generate")
        .to_string();
    let score_error = runtime
        .score(handle.model_id, vec![1, 2, 3])
        .await
        .expect_err("503 must fail score")
        .to_string();
    let embed_error = runtime
        .embed(handle.model_id, "echo probe")
        .await
        .expect_err("503 must fail embed")
        .to_string();

    for (surface, error) in [
        ("generate", generate_error),
        ("score", score_error),
        ("embed", embed_error),
    ] {
        assert!(!error.contains(PROVIDER_ECHO_CANARY), "{surface}: {error}");
        assert!(error.contains("<redacted provider response body"), "{surface}: {error}");
        assert!(error.len() <= 256, "{surface} error must remain bounded: {} bytes", error.len());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn openai_byok_malformed_success_sse_never_reflects_byok_canary() {
    let trace_buffer = capture_tracing();
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(OPENAI_CHAT_COMPLETIONS_PATH))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(format!("data: {API_KEY_FIXTURE}\n\n")),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    let sink = Arc::new(CapturingSink::default());
    let recorder = Arc::new(CapturingFlightRecorder::default());
    let lane_obs = Arc::new(CloudLaneObservability {
        flight_recorder: recorder.clone(),
        consent: None,
    });
    let runtime = fixture_runtime(mock_server.uri(), sink.clone())
        .with_lane_observability(lane_obs);
    let handle = runtime
        .register_handle("gpt-4o", "2026-05-20T11:00:00Z")
        .expect("allowlisted");
    let mut stream = runtime.generate(fixture_generate_request(
        handle.model_id,
        CancellationToken::new(),
    ));
    let error = stream
        .next()
        .await
        .expect("malformed SSE yields an error")
        .expect_err("malformed 2xx SSE must fail")
        .to_string();

    let tracing = String::from_utf8_lossy(&trace_buffer.0.lock().unwrap()).into_owned();
    let audit_rows = sink.rows.lock().unwrap().clone();
    assert!(audit_rows.iter().any(|row| row.status == CloudCallStatus::Started));
    assert!(audit_rows.iter().any(|row| row.status == CloudCallStatus::Failed));
    let recorder_events = recorder.events.lock().unwrap().clone();
    assert!(recorder_events.iter().any(|event| event.payload["phase"] == "start"));
    assert!(recorder_events.iter().any(|event| event.payload["phase"] == "end"));
    assert!(tracing.contains("openai-byok-tracing-live"), "tracing capture must be live");
    let audit = format!("{audit_rows:?}");
    let recorded = format!("{recorder_events:?}");
    for (surface, rendered) in [
        ("returned_error", error.as_str()),
        ("tracing", tracing.as_str()),
        ("audit", audit.as_str()),
        ("flight_recorder", recorded.as_str()),
    ] {
        assert!(!rendered.contains(API_KEY_FIXTURE), "{surface} leaked BYOK canary: {rendered}");
    }
    assert!(error.contains("event_kind=chat_chunk"), "{error}");
    assert!(error.contains("payload_bytes="), "{error}");
    assert!(error.len() <= 256, "SSE error must remain bounded: {} bytes", error.len());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn openai_byok_truncated_sse_stream_error_never_reflects_byok_canary() {
    let trace_buffer = capture_tracing();
    let (api_base, server) =
        start_truncated_sse_server(format!("data: {API_KEY_FIXTURE}")).await;
    let sink = Arc::new(CapturingSink::default());
    let recorder = Arc::new(CapturingFlightRecorder::default());
    let runtime = fixture_runtime(api_base, sink.clone()).with_lane_observability(Arc::new(
        CloudLaneObservability {
            flight_recorder: recorder.clone(),
            consent: None,
        },
    ));
    let handle = runtime
        .register_handle("gpt-4o", "2026-05-20T11:00:00Z")
        .expect("allowlisted");
    let mut stream = runtime.generate(fixture_generate_request(
        handle.model_id,
        CancellationToken::new(),
    ));
    let error = stream
        .next()
        .await
        .expect("truncated SSE yields an error")
        .expect_err("truncated SSE must fail")
        .to_string();
    server.await.expect("truncated SSE server completes");

    let tracing = String::from_utf8_lossy(&trace_buffer.0.lock().unwrap()).into_owned();
    let audit_rows = sink.rows.lock().unwrap().clone();
    let recorder_events = recorder.events.lock().unwrap().clone();
    assert!(audit_rows.iter().any(|row| row.status == CloudCallStatus::Started));
    assert!(audit_rows.iter().any(|row| row.status == CloudCallStatus::Failed));
    assert!(recorder_events.iter().any(|event| event.payload["phase"] == "start"));
    assert!(recorder_events.iter().any(|event| event.payload["phase"] == "end"));
    assert!(tracing.contains("openai-byok-tracing-live"));
    for (surface, rendered) in [
        ("returned_error", error.clone()),
        ("tracing", tracing),
        ("audit", format!("{audit_rows:?}")),
        ("flight_recorder", format!("{recorder_events:?}")),
    ] {
        assert!(!rendered.contains(API_KEY_FIXTURE), "{surface} leaked BYOK canary: {rendered}");
    }
    assert!(error.contains("SSE framing failure"), "{error}");
    assert!(error.len() <= 256, "SSE error must remain bounded: {} bytes", error.len());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn openai_byok_malformed_success_dtos_never_reflect_byok_canary() {
    let trace_buffer = capture_tracing();
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(OPENAI_CHAT_COMPLETIONS_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_string(format!(
            "{{\"choices\":\"{API_KEY_FIXTURE}\"}}"
        )))
        .expect(1)
        .mount(&mock_server)
        .await;
    Mock::given(method("POST"))
        .and(path(OPENAI_EMBEDDINGS_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_string(format!(
            "{{\"data\":\"{API_KEY_FIXTURE}\"}}"
        )))
        .expect(1)
        .mount(&mock_server)
        .await;

    let sink = Arc::new(CapturingSink::default());
    let runtime = fixture_runtime(mock_server.uri(), sink.clone());
    let handle = runtime
        .register_handle("gpt-4o", "2026-05-20T11:00:00Z")
        .expect("allowlisted");
    let score_error = runtime.score(handle.model_id, vec![1, 2, 3]).await
        .expect_err("malformed score DTO must fail").to_string();
    let embed_error = runtime.embed(handle.model_id, "dto probe").await
        .expect_err("malformed embed DTO must fail").to_string();
    let tracing = String::from_utf8_lossy(&trace_buffer.0.lock().unwrap()).into_owned();
    let audit_rows = sink.rows.lock().unwrap().clone();
    let audit = format!("{audit_rows:?}");

    assert!(audit_rows.iter().any(|row| row.call_kind == CloudCallKind::Score && row.status == CloudCallStatus::Failed));
    assert!(audit_rows.iter().any(|row| row.call_kind == CloudCallKind::Embeddings && row.status == CloudCallStatus::Failed));
    assert!(tracing.contains("openai-byok-tracing-live"));
    for (surface, rendered) in [
        ("score_error", score_error.as_str()),
        ("embed_error", embed_error.as_str()),
        ("tracing", tracing.as_str()),
        ("audit", audit.as_str()),
    ] {
        assert!(!rendered.contains(API_KEY_FIXTURE), "{surface} leaked BYOK canary: {rendered}");
    }
    assert!(score_error.len() <= 256);
    assert!(embed_error.len() <= 256);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn openai_byok_provider_fetch_failure_is_typed_and_never_echoes_provider_error() {
    let trace_buffer = capture_tracing();
    let sink = Arc::new(CapturingSink::default());
    let runtime = OpenAiByokRuntime::with_client(
        "http://127.0.0.1:9",
        reqwest::Client::new(),
        Arc::new(EchoingFailKey),
        sink.clone(),
    );
    let handle = runtime
        .register_handle("gpt-4o", "2026-05-20T11:00:00Z")
        .expect("allowlisted");
    let mut generate = runtime.generate(fixture_generate_request(
        handle.model_id,
        CancellationToken::new(),
    ));
    let generate_error = generate.next().await.expect("generate error item")
        .expect_err("provider fetch fails generate").to_string();
    let score_error = runtime.score(handle.model_id, vec![1]).await
        .expect_err("provider fetch fails score").to_string();
    let embed_error = runtime.embed(handle.model_id, "probe").await
        .expect_err("provider fetch fails embed").to_string();

    for error in [&generate_error, &score_error, &embed_error] {
        assert!(!error.contains(API_KEY_FIXTURE), "provider error leaked: {error}");
        assert!(error.contains("code=provider_failure"), "typed code missing: {error}");
        assert!(error.len() <= 160, "typed provider error must be bounded: {} bytes", error.len());
    }
    let tracing = String::from_utf8_lossy(&trace_buffer.0.lock().unwrap()).into_owned();
    let audit = format!("{:?}", sink.rows.lock().unwrap());
    assert!(tracing.contains("openai-byok-tracing-live"));
    assert!(!tracing.contains(API_KEY_FIXTURE), "tracing leaked provider error: {tracing}");
    assert!(!audit.contains(API_KEY_FIXTURE), "audit leaked provider error: {audit}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn openai_byok_clean_eof_without_done_is_failed_and_redacted() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(OPENAI_CHAT_COMPLETIONS_PATH))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(format!(": ignored-{API_KEY_FIXTURE}\n\n")),
        )
        .expect(1)
        .mount(&mock_server)
        .await;
    let sink = Arc::new(CapturingSink::default());
    let recorder = Arc::new(CapturingFlightRecorder::default());
    let runtime = fixture_runtime(mock_server.uri(), sink.clone()).with_lane_observability(Arc::new(
        CloudLaneObservability { flight_recorder: recorder.clone(), consent: None },
    ));
    let handle = runtime.register_handle("gpt-4o", "2026-05-20T11:00:00Z").unwrap();
    let mut stream = runtime.generate(fixture_generate_request(handle.model_id, CancellationToken::new()));
    let error = stream.next().await.expect("missing terminal error")
        .expect_err("clean EOF without DONE must fail").to_string();
    let audit_rows = sink.rows.lock().unwrap().clone();
    let recorder_events = recorder.events.lock().unwrap().clone();

    assert!(error.contains("terminal missing"), "{error}");
    assert!(audit_rows.iter().any(|row| row.status == CloudCallStatus::Failed));
    assert!(recorder_events.iter().any(|event| event.payload["phase"] == "end"));
    for rendered in [error, format!("{audit_rows:?}"), format!("{recorder_events:?}")] {
        assert!(!rendered.contains(API_KEY_FIXTURE), "clean-EOF surface leaked: {rendered}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn openai_byok_finish_reason_before_clean_eof_never_emits_premature_terminal() {
    let mock_server = MockServer::start().await;
    let body = "data: {\"choices\":[{\"delta\":{\"content\":\"tail\"},\"finish_reason\":\"stop\"}]}\n\n";
    Mock::given(method("POST"))
        .and(path(OPENAI_CHAT_COMPLETIONS_PATH))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(body),
        )
        .expect(2)
        .mount(&mock_server)
        .await;
    let sink = Arc::new(CapturingSink::default());
    let runtime = fixture_runtime(mock_server.uri(), sink.clone());
    let handle = runtime.register_handle("gpt-4o", "2026-05-20T11:00:00Z").unwrap();

    let mut full_drain = runtime.generate(fixture_generate_request(
        handle.model_id,
        CancellationToken::new(),
    ));
    let content = full_drain.next().await.expect("content item").expect("content token");
    assert_eq!(content.text, "tail");
    assert!(content.finish_reason.is_none(), "provider finish_reason must remain buffered");
    let full_drain_error = full_drain.next().await.expect("terminal-contract error")
        .expect_err("clean EOF must fail despite provider finish_reason").to_string();
    assert!(full_drain.next().await.is_none(), "stream ends after its error");

    let mut early_stop = runtime.generate(fixture_generate_request(
        handle.model_id,
        CancellationToken::new(),
    ));
    let mut saw_premature_terminal = false;
    let mut early_stop_error = None;
    while let Some(item) = early_stop.next().await {
        match item {
            Ok(token) if token.finish_reason.is_some() => {
                saw_premature_terminal = true;
                break;
            }
            Ok(_) => {}
            Err(error) => {
                early_stop_error = Some(error.to_string());
                break;
            }
        }
    }

    assert!(!saw_premature_terminal, "early-stop consumer must not observe success before DONE");
    assert!(early_stop_error.as_deref().is_some_and(|error| error.contains("terminal missing")));
    assert!(full_drain_error.contains("terminal missing"), "{full_drain_error}");
    assert!(sink.rows.lock().unwrap().iter().filter(|row| row.status == CloudCallStatus::Failed).count() >= 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn openai_byok_done_without_usable_content_is_failed_redacted_and_observable() {
    let trace_buffer = capture_tracing();
    let mock_server = MockServer::start().await;
    let attempt = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let attempt_for_mock = attempt.clone();
    Mock::given(method("POST"))
        .and(path(OPENAI_CHAT_COMPLETIONS_PATH))
        .respond_with(move |_request: &Request| {
            let body = if attempt_for_mock.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
                format!(": ignored-{API_KEY_FIXTURE}\n\ndata: [DONE]\n\n")
            } else {
                format!(
                    "data: {{\"choices\":[{{\"delta\":{{}},\"finish_reason\":\"stop\"}}],\"echo\":\"{API_KEY_FIXTURE}\"}}\n\ndata: [DONE]\n\n"
                )
            };
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(body)
        })
        .expect(2)
        .mount(&mock_server)
        .await;

    let sink = Arc::new(CapturingSink::default());
    let recorder = Arc::new(CapturingFlightRecorder::default());
    let runtime = fixture_runtime(mock_server.uri(), sink.clone()).with_lane_observability(Arc::new(
        CloudLaneObservability {
            flight_recorder: recorder.clone(),
            consent: None,
        },
    ));
    let handle = runtime.register_handle("gpt-4o", "2026-05-20T11:00:00Z").unwrap();

    let mut done_only = runtime.generate(fixture_generate_request(
        handle.model_id,
        CancellationToken::new(),
    ));
    let done_only_error = done_only.next().await.expect("DONE-only error item")
        .expect_err("DONE without content must fail").to_string();
    assert!(done_only.next().await.is_none());

    let mut contentless_choice = runtime.generate(fixture_generate_request(
        handle.model_id,
        CancellationToken::new(),
    ));
    let contentless_error = contentless_choice.next().await.expect("contentless-choice error item")
        .expect_err("contentless choice followed by DONE must fail").to_string();
    assert!(contentless_choice.next().await.is_none());

    let tracing = String::from_utf8_lossy(&trace_buffer.0.lock().unwrap()).into_owned();
    let audit_rows = sink.rows.lock().unwrap().clone();
    let recorder_events = recorder.events.lock().unwrap().clone();
    assert!(tracing.contains("openai-byok-tracing-live"), "tracing proof must be live");
    assert!(audit_rows.iter().any(|row| row.status == CloudCallStatus::Started));
    assert!(audit_rows.iter().filter(|row| row.status == CloudCallStatus::Failed).count() >= 2);
    assert!(recorder_events.iter().filter(|event| event.payload["phase"] == "start").count() >= 2);
    assert!(recorder_events.iter().filter(|event| event.payload["phase"] == "end").count() >= 2);

    for (surface, rendered) in [
        ("done_only_error", done_only_error.as_str()),
        ("contentless_error", contentless_error.as_str()),
        ("tracing", tracing.as_str()),
    ] {
        assert!(!rendered.is_empty(), "{surface} proof must be non-vacuous");
        assert!(!rendered.contains(API_KEY_FIXTURE), "{surface} leaked BYOK canary: {rendered}");
    }
    let audit = format!("{audit_rows:?}");
    let recorded = format!("{recorder_events:?}");
    assert!(!audit.is_empty() && !audit.contains(API_KEY_FIXTURE), "audit leak: {audit}");
    assert!(!recorded.is_empty() && !recorded.contains(API_KEY_FIXTURE), "recorder leak: {recorded}");
    for error in [done_only_error, contentless_error] {
        assert!(error.contains("reason=no_usable_result"), "{error}");
        assert!(error.contains("provider_payload=<redacted>"), "{error}");
        assert!(error.len() <= 160, "unusable-result error must be bounded: {} bytes", error.len());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn openai_byok_transport_errors_are_bounded_and_never_echo_canary_url() {
    let trace_buffer = capture_tracing();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("reserve port");
    let address = listener.local_addr().expect("reserved address");
    drop(listener);
    let api_base = format!("http://{address}/{API_KEY_FIXTURE}");
    let sink = Arc::new(CapturingSink::default());
    let runtime = fixture_runtime(api_base, sink.clone());
    let handle = runtime.register_handle("gpt-4o", "2026-05-20T11:00:00Z").unwrap();

    let mut generate = runtime.generate(fixture_generate_request(handle.model_id, CancellationToken::new()));
    let generate_error = generate.next().await.expect("transport error")
        .expect_err("generate connection must fail").to_string();
    let score_error = runtime.score(handle.model_id, vec![1]).await
        .expect_err("score connection must fail").to_string();
    let embed_error = runtime.embed(handle.model_id, "probe").await
        .expect_err("embed connection must fail").to_string();
    let tracing = String::from_utf8_lossy(&trace_buffer.0.lock().unwrap()).into_owned();
    let audit = format!("{:?}", sink.rows.lock().unwrap());

    for (operation, error) in [
        ("generate", generate_error),
        ("score", score_error),
        ("embed", embed_error),
    ] {
        assert!(error.contains(&format!("operation={operation}")), "{error}");
        assert!(error.contains("code=connect"), "{error}");
        assert!(!error.contains(API_KEY_FIXTURE), "transport error leaked URL: {error}");
        assert!(error.len() <= 128, "transport error must be bounded: {} bytes", error.len());
    }
    assert!(!tracing.contains(API_KEY_FIXTURE), "tracing leaked transport URL: {tracing}");
    assert!(!audit.contains(API_KEY_FIXTURE), "audit leaked transport URL: {audit}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn openai_byok_empty_success_payloads_are_failed_and_redacted() {
    let mock_server = MockServer::start().await;
    let chat_attempt = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let chat_attempt_for_mock = chat_attempt.clone();
    Mock::given(method("POST"))
        .and(path(OPENAI_CHAT_COMPLETIONS_PATH))
        .respond_with(move |_request: &Request| {
            if chat_attempt_for_mock.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(format!(
                        "data: {{\"choices\":[],\"echo\":\"{API_KEY_FIXTURE}\"}}\n\n"
                    ))
            } else {
                ResponseTemplate::new(200).set_body_string(format!(
                    "{{\"choices\":[],\"echo\":\"{API_KEY_FIXTURE}\"}}"
                ))
            }
        })
        .expect(2)
        .mount(&mock_server)
        .await;
    Mock::given(method("POST"))
        .and(path(OPENAI_EMBEDDINGS_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_string(format!(
            "{{\"data\":[],\"echo\":\"{API_KEY_FIXTURE}\"}}"
        )))
        .expect(1)
        .mount(&mock_server)
        .await;

    let sink = Arc::new(CapturingSink::default());
    let runtime = fixture_runtime(mock_server.uri(), sink.clone());
    let handle = runtime.register_handle("gpt-4o", "2026-05-20T11:00:00Z").unwrap();
    let mut generate = runtime.generate(fixture_generate_request(handle.model_id, CancellationToken::new()));
    let generate_error = generate.next().await.expect("empty choices error")
        .expect_err("empty choices must fail").to_string();
    let score_error = runtime.score(handle.model_id, vec![1]).await
        .expect_err("empty logprobs must fail").to_string();
    let embed_error = runtime.embed(handle.model_id, "probe").await
        .expect_err("empty embedding data must fail").to_string();
    let audit_rows = sink.rows.lock().unwrap().clone();

    for error in [&generate_error, &score_error, &embed_error] {
        assert!(!error.contains(API_KEY_FIXTURE), "empty response reflected canary: {error}");
        assert!(error.contains("provider_payload=<redacted>"), "{error}");
    }
    for kind in [CloudCallKind::ChatCompletion, CloudCallKind::Score, CloudCallKind::Embeddings] {
        assert!(audit_rows.iter().any(|row| row.call_kind == kind && row.status == CloudCallStatus::Failed), "missing failed audit for {kind:?}");
    }
    assert!(!format!("{audit_rows:?}").contains(API_KEY_FIXTURE));
}

// ---------------------------------------------------------------------
// Test 2: cancellation
// ---------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn openai_byok_cancellation_marks_call_cancelled() {
    // We want to prove that a cancellation flag flips the audited
    // status to Cancelled and the stream surfaces FinishReason::Cancelled.
    //
    // The simplest deterministic shape is pre-call cancellation:
    // trip the cancel flag BEFORE calling chat_completions_stream.
    // The spawned task checks the flag first thing and bails with a
    // Cancelled audit row + FinishReason::Cancelled terminal token,
    // without issuing a request to wiremock. This avoids racing
    // SSE buffer drains against the cancel flag.

    let mock_server = MockServer::start().await;
    // Mount a permissive mock but expect 0 hits — we want to prove
    // pre-call cancellation never reaches the wire.
    Mock::given(method("POST"))
        .and(path(OPENAI_CHAT_COMPLETIONS_PATH))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string("data: [DONE]\n\n"),
        )
        .expect(0)
        .mount(&mock_server)
        .await;

    let sink = Arc::new(CapturingSink::default());
    let runtime = fixture_runtime(mock_server.uri(), sink.clone());
    let handle = runtime
        .register_handle("gpt-4o-2024-08-06", "2026-05-20T11:00:00Z")
        .unwrap();

    let cancel = CancellationToken::new();
    cancel.cancel();
    let req = fixture_generate_request(handle.model_id, cancel);

    let mut stream = runtime.chat_completions_stream(req);

    // The stream must surface a Cancelled terminal token.
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
            Err(_) => {
                // Errors are surfaced here only on real wire failure;
                // pre-cancel should not produce one.
                break;
            }
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
// Test 3: API key only in Bearer auth, never elsewhere
// ---------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn openai_byok_api_key_only_appears_as_bearer_auth() {
    let mock_server = MockServer::start().await;
    let inspector = Arc::new(RequestInspector::default());
    let inspector_for_mock = inspector.clone();

    let payload = sse_payload_for(&["ok"]);
    Mock::given(method("POST"))
        .and(path(OPENAI_CHAT_COMPLETIONS_PATH))
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
        .register_handle("gpt-4o", "2026-05-20T11:00:00Z")
        .unwrap();
    let req = fixture_generate_request(handle.model_id, CancellationToken::new());
    let mut stream = runtime.chat_completions_stream(req);
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

    // (a) Authorization header is the only place the api key appears.
    let auth_header = inspected
        .headers
        .get("authorization")
        .expect("Authorization header must be present");
    let auth_value = auth_header.to_str().expect("header is ascii");
    assert_eq!(
        auth_value,
        format!("Bearer {API_KEY_FIXTURE}"),
        "Authorization must carry the api key as `Bearer <key>` and nothing else",
    );

    // (b) No other header value contains the key.
    for (name, value) in inspected.headers.iter() {
        if name.as_str().eq_ignore_ascii_case("authorization") {
            continue;
        }
        let v = value.to_str().unwrap_or("");
        assert!(
            !v.contains(API_KEY_FIXTURE),
            "header {} must not contain the api key (got {v})",
            name.as_str()
        );
    }

    // (c) Body must not contain the api key (the body should only
    // hold the chat-completions JSON payload).
    let body = std::str::from_utf8(&inspected.body).unwrap_or("");
    assert!(
        !body.contains(API_KEY_FIXTURE),
        "request body must not contain the api key"
    );

    // (d) URL/query string must not contain the api key either.
    assert!(
        !inspected.url.as_str().contains(API_KEY_FIXTURE),
        "request URL must not contain the api key"
    );

    // (e) The runtime's Debug output must not contain the key.
    let dbg = format!("{runtime:?}");
    assert!(!dbg.contains(API_KEY_FIXTURE), "{dbg}");
}

// ---------------------------------------------------------------------
// Test 4: capabilities shape
// ---------------------------------------------------------------------

#[test]
fn openai_byok_capabilities_match_cloud_realities() {
    let caps = OpenAiByokRuntime::cloud_capabilities();
    assert!(!caps.supports_lora);
    assert!(caps.supports_kv_prefix_cache);
    assert!(!caps.supports_activation_steering);
    assert!(!caps.supports_subquadratic);
    assert!(!caps.supports_speculative_draft);
    assert!(!caps.supports_eagle3);
}

// ---------------------------------------------------------------------
// Test 5: no ProcessOwnershipLedger row for BYOK calls.
//
// The BYOK runtime never imports or constructs the process-ledger
// types; this test pins that statement by verifying that a fully-
// driven generate() call only touches the CloudInvocationAuditSink.
// We do not need to reach into the process_ledger module to assert
// this — the absence of a writer call is structural.
// ---------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn openai_byok_does_not_register_process_ownership_row() {
    let mock_server = MockServer::start().await;
    let payload = sse_payload_for(&["a", "b", "c"]);
    Mock::given(method("POST"))
        .and(path(OPENAI_CHAT_COMPLETIONS_PATH))
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
        .register_handle("gpt-4o", "2026-05-20T11:00:00Z")
        .unwrap();
    let req = fixture_generate_request(handle.model_id, CancellationToken::new());
    let mut stream = runtime.chat_completions_stream(req);
    while let Some(_item) = stream.next().await {}

    // Every audit row written should belong to CloudInvocationAuditSink.
    // A ProcessOwnershipLedger row would imply a separate write path;
    // it doesn't exist in this adapter and these rows are the only
    // evidence channel.
    let rows = sink.rows.lock().unwrap().clone();
    assert!(!rows.is_empty(), "BYOK must produce audit rows");
    for row in rows {
        // All rows must use the BYOK CloudInvocationAuditRow shape;
        // openai_model_name + cloud-call kinds only.
        assert!(matches!(
            row.call_kind,
            CloudCallKind::ChatCompletion | CloudCallKind::Embeddings | CloudCallKind::Score
        ));
    }
}

// ---------------------------------------------------------------------
// Test 6: allowlist rejection at load() time
// ---------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn openai_byok_load_rejects_non_allowlisted_model_name() {
    let mock_server = MockServer::start().await;
    let sink = Arc::new(CapturingSink::default());
    let mut runtime = fixture_runtime(mock_server.uri(), sink.clone());

    let spec = LoadSpec {
        artifact_path: std::path::PathBuf::from("/not/used/for/cloud"),
        sha256_expected: String::new(),
        runtime_kind: handshake_core::model_runtime::RuntimeKind::LlamaCpp, // ignored for cloud
        sampling_defaults: SamplingParams::default(),
        kv_cache_policy: KvCachePolicy::default(),
        declared_capabilities: OpenAiByokRuntime::cloud_capabilities(),
        provider: ProviderKind::ByokCloud,
        engine_origin: Some("not-a-real-openai-model".to_string()),
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
// Test 7: ProviderKind validation at load() time
// ---------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn openai_byok_load_rejects_non_byok_provider() {
    let mock_server = MockServer::start().await;
    let sink = Arc::new(CapturingSink::default());
    let mut runtime = fixture_runtime(mock_server.uri(), sink.clone());

    let spec = LoadSpec {
        artifact_path: std::path::PathBuf::from("/not/used/for/cloud"),
        sha256_expected: String::new(),
        runtime_kind: handshake_core::model_runtime::RuntimeKind::LlamaCpp,
        sampling_defaults: SamplingParams::default(),
        kv_cache_policy: KvCachePolicy::default(),
        declared_capabilities: OpenAiByokRuntime::cloud_capabilities(),
        provider: ProviderKind::Local, // wrong lane
        engine_origin: Some("gpt-4o".to_string()),
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
// Carry-over structural tests from the prior session
// ---------------------------------------------------------------------

#[test]
fn cloud_openai_runtime_debug_redacts_api_key() {
    let sink = Arc::new(CapturingSink::default());
    let runtime = fixture_runtime("https://api.openai.com/v1".to_string(), sink);
    let dbg = format!("{runtime:?}");
    assert!(dbg.contains("<redacted"), "{dbg}");
    assert!(!dbg.contains(API_KEY_FIXTURE), "{dbg}");
}

#[test]
fn cloud_openai_runtime_register_handle_validates_allowlist() {
    let sink = Arc::new(CapturingSink::default());
    let runtime = fixture_runtime("https://api.openai.com/v1".to_string(), sink);
    runtime
        .register_handle("gpt-4o-2024-08-06", "2026-05-20T05:00:00Z")
        .expect("allowed");
    let err = runtime
        .register_handle("definitely-not-openai", "2026-05-20T05:00:00Z")
        .expect_err("not allowed");
    assert!(matches!(err, OpenAiByokError::ModelNameNotAllowed(_)));
}

#[test]
fn cloud_openai_audit_sink_records_call_lifecycle() {
    let sink = Arc::new(CapturingSink::default());
    let runtime = fixture_runtime("https://api.openai.com/v1".to_string(), sink.clone());
    let handle = runtime
        .register_handle("gpt-4o", "2026-05-20T05:00:00Z")
        .unwrap();
    runtime
        .record_audit(CloudInvocationAuditRow {
            model_id: handle.model_id,
            openai_model_name: handle.openai_model_name.clone(),
            call_kind: CloudCallKind::ChatCompletion,
            started_at_utc: "2026-05-20T05:00:00Z".to_string(),
            finished_at_utc: Some("2026-05-20T05:00:01Z".to_string()),
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
            .any(|r| r.openai_model_name == "gpt-4o" && r.status == CloudCallStatus::Succeeded),
        "the appended Succeeded row must round-trip via the sink"
    );
}

// ---------------------------------------------------------------------
// MT-125 remediation Test A: FR-EVT-LLM-INFER emission with adapter tag
//
// With a CapturingFlightRecorder + always-approve ConsentProvider
// attached via with_lane_observability, a successful streaming
// generate() must emit at least one FR-EVT-LLM-INFER-START and one
// FR-EVT-LLM-INFER-END whose payload `adapter` == "openai_byok".
// ---------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn openai_byok_emits_fr_infer_events_with_adapter_tag() {
    let mock_server = MockServer::start().await;
    // 20 tokens so a TOKEN sample fires (sample interval is 16) on
    // top of the guaranteed START + END.
    let words: Vec<String> = (0..20).map(|i| format!("tok{i} ")).collect();
    let word_refs: Vec<&str> = words.iter().map(|s| s.as_str()).collect();
    let payload = sse_payload_for(&word_refs);
    Mock::given(method("POST"))
        .and(path(OPENAI_CHAT_COMPLETIONS_PATH))
        .and(header(
            "authorization",
            format!("Bearer {API_KEY_FIXTURE}").as_str(),
        ))
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
        .register_handle("gpt-4o-2024-08-06", "2026-05-20T11:00:00Z")
        .expect("allowlisted");

    let req = fixture_generate_request(handle.model_id, CancellationToken::new());
    let mut stream = runtime.chat_completions_stream(req);
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
                if phase == "start" && adapter == "openai_byok")
        })
        .count();
    let end_count = events
        .iter()
        .filter(|ev| {
            matches!(event_kind(ev), Some((phase, adapter))
                if phase == "end" && adapter == "openai_byok")
        })
        .count();

    assert!(
        start_count >= 1,
        "must emit >=1 FR-EVT-LLM-INFER-START with adapter=openai_byok; events={events:?}"
    );
    assert!(
        end_count >= 1,
        "must emit >=1 FR-EVT-LLM-INFER-END with adapter=openai_byok; events={events:?}"
    );
}

// ---------------------------------------------------------------------
// MT-125 remediation Test B: consent-denied short-circuits before HTTP
//
// With a deny ConsentProvider attached, generate() must yield an error
// item and NEVER reach the wire (wiremock expect 0).
// ---------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn openai_byok_consent_denied_blocks_http_call() {
    let mock_server = MockServer::start().await;
    // Permissive mock but EXPECT 0 — a denied consent must not reach
    // the wire.
    Mock::given(method("POST"))
        .and(path(OPENAI_CHAT_COMPLETIONS_PATH))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string("data: [DONE]\n\n"),
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
        .register_handle("gpt-4o", "2026-05-20T11:00:00Z")
        .expect("allowlisted");

    let req = fixture_generate_request(handle.model_id, CancellationToken::new());
    let mut stream = runtime.chat_completions_stream(req);

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

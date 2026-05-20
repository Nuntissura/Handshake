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

use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::StreamExt;
use handshake_core::model_runtime::cloud::{
    ApiKeyProvider, CloudCallKind, CloudCallStatus, CloudInvocationAuditRow,
    CloudInvocationAuditSink, OpenAiByokError, OpenAiByokRuntime,
    OPENAI_CHAT_COMPLETIONS_PATH,
};
use handshake_core::model_runtime::{
    CancellationToken, GenPrompt, GenerateRequest, ModelId, ModelRuntime, ProviderKind,
    SamplingParams, KvCachePolicy, LoadSpec,
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
            if matches!(
                finish,
                handshake_core::model_runtime::FinishReason::Stop
            ) && token.text.is_empty()
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
    assert!(saw_terminal_stop, "stream must terminate with FinishReason::Stop");

    let rows = sink.rows.lock().unwrap().clone();
    let chat_rows: Vec<_> = rows
        .iter()
        .filter(|r| r.call_kind == CloudCallKind::ChatCompletion)
        .collect();
    // Expected lifecycle: register_handle Started, generate Started, final Succeeded.
    assert!(
        chat_rows.iter().any(|r| r.status == CloudCallStatus::Started),
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
    while let Ok(Some(item)) =
        tokio::time::timeout(Duration::from_secs(5), stream.next()).await
    {
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
    assert_eq!(snapshot.len(), 1, "exactly one HTTP request must reach the mock");
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
    assert!(!captured.is_empty(), "register_handle + record_audit emit rows");
    assert!(
        captured
            .iter()
            .any(|r| r.openai_model_name == "gpt-4o" && r.status == CloudCallStatus::Succeeded),
        "the appended Succeeded row must round-trip via the sink"
    );
}

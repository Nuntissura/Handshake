//! MT-015: BYOK cloud-access secrets-leak negative test + no-consent-on-save.
//!
//! This is the security-critical proof for the operator cloud-model access
//! surface. It stores a CANARY BYOK key through the REAL production settings
//! path ([`CloudModelAccess`] wired to the OS-keychain vault), drives a live
//! cloud call against wiremock using the stored key, and then asserts the
//! canary is ABSENT from every non-keychain surface:
//!
//! * the cloud-invocation audit rows (the `cloud_invocations` surface — the
//!   `CloudInvocationAuditSink`);
//! * captured `tracing` output (which is also where any Flight-Recorder /
//!   EventLedger event that leaked the key would surface, since no PG
//!   `cloud_invocations` table or cloud EventLedger sink exists yet — the
//!   audit sink IS that surface);
//! * every `Debug` of the runtime, the access service, the vault provider,
//!   and the non-secret enumeration JSON;
//! * the HTTP request body (the key may appear ONLY as the `Authorization:
//!   Bearer` header the provider requires).
//!
//! It also proves:
//! * the wired vault is `OsKeychainSecretsVault`, NOT the in-memory impl;
//! * the key still round-trips OUT of the vault for use;
//! * saving a key creates NO consent approval — a fresh `ConsentGate` still
//!   fails closed on first prompt (MT-006 boundary intact).
//!
//! Gated to `os-keychain` + Windows because the round-trip proof talks to the
//! real Windows Credential Manager (matching `cloud_os_keychain_vault_tests`).
//! A UNIQUE service namespace + explicit delete keep the host keychain clean;
//! the key is deleted BEFORE assertions run so a failing assertion never
//! leaves a credential behind.

#![cfg(all(feature = "os-keychain", target_os = "windows"))]

use std::io::Write;
use std::sync::{Arc, Mutex, Once, OnceLock};
use std::time::Duration;

use futures::StreamExt;
use handshake_core::model_runtime::cloud::{
    ApiKeyProvider, ByokProvider, CloudInvocationAuditRow, CloudInvocationAuditSink,
    CloudModelAccess, ConsentDecision, ConsentGate, ConsentGateError, ConsentProvider,
    OpenAiByokError, OpenAiByokRuntime, OsKeychainSecretsVault, SecretsVault, VaultApiKeyProvider,
    OPENAI_CHAT_COMPLETIONS_PATH,
};
use handshake_core::model_runtime::{
    CancellationToken, GenPrompt, GenerateRequest, ModelId, SamplingParams,
};
use secrecy::SecretString;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

/// The canary. If this string appears anywhere it should not, the test fails.
const CANARY_KEY: &str = "sk-CANARY-mt015-NEVER-LEAK-THIS-KEY-0xDEADBEEF";

// ---------------------------------------------------------------------------
// Capturing sinks: audit rows, request inspector, and a global tracing buffer.
// ---------------------------------------------------------------------------

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

/// Shared byte buffer usable as a `tracing_subscriber` writer so we can assert
/// the canary never reaches any tracing sink (Flight Recorder / EventLedger /
/// audit logging all ultimately surface here in the current wiring).
#[derive(Clone, Default)]
struct SharedBuf(Arc<Mutex<Vec<u8>>>);
impl Write for SharedBuf {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for SharedBuf {
    type Writer = SharedBuf;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

/// Install a global TRACE-level capturing subscriber exactly once and return
/// the shared buffer. Captures events from every thread (wiremock/reqwest run
/// on tokio workers), so an accidental key log anywhere is caught.
fn capture_tracing() -> SharedBuf {
    static BUF: OnceLock<SharedBuf> = OnceLock::new();
    static INIT: Once = Once::new();
    let buf = BUF.get_or_init(SharedBuf::default).clone();
    INIT.call_once(|| {
        let _ = tracing_subscriber::fmt()
            .with_writer(buf.clone())
            .with_max_level(tracing::Level::TRACE)
            .with_ansi(false)
            .try_init();
    });
    buf
}

struct AlwaysDeny;
impl ConsentProvider for AlwaysDeny {
    fn prompt_for_decision(
        &self,
        _session_id: &str,
        _lane: &str,
    ) -> Result<ConsentDecision, ConsentGateError> {
        Ok(ConsentDecision::Denied)
    }
}

fn sse_payload() -> String {
    let mut body = String::new();
    for (idx, text) in ["Hel", "lo"].iter().enumerate() {
        let finish = if idx == 1 { Some("stop") } else { None };
        let chunk = serde_json::json!({
            "id": format!("chatcmpl-{idx}"),
            "object": "chat.completion.chunk",
            "created": 1_700_000_000_u64 + idx as u64,
            "model": "gpt-4o-2024-08-06",
            "choices": [{"index": 0, "delta": {"content": text}, "finish_reason": finish}],
        });
        body.push_str(&format!("data: {chunk}\n\n"));
    }
    body.push_str("data: [DONE]\n\n");
    body
}

fn generate_request(model_id: ModelId) -> GenerateRequest {
    GenerateRequest {
        id: model_id,
        prompt: GenPrompt::new("Say hello."),
        sampling: SamplingParams {
            temperature: Some(0.5),
            ..SamplingParams::default()
        },
        lora_overrides: Vec::new(),
        steering_overrides: Vec::new(),
        kv_prefix_handle: None,
        cancel: CancellationToken::new(),
        max_tokens: 16,
        stop_sequences: Vec::new(),
        speculative_mode: None,
        structured_decoding: None,
    }
}

// ---------------------------------------------------------------------------
// The leak proof.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn byok_canary_key_never_leaks_and_round_trips_only_through_os_keychain() {
    let trace_buf = capture_tracing();

    // (1) The PRODUCTION wiring must be the OS keychain, never in-memory.
    let production = CloudModelAccess::production().expect("os-keychain production service");
    assert_eq!(
        production.vault_kind(),
        "OsKeychainSecretsVault",
        "production access service must be wired to the OS keychain, not in-memory"
    );

    // Use a UNIQUE-namespace real keychain vault for the round-trip so cleanup
    // never touches the operator's default Handshake keychain entries.
    let namespace = format!("handshake-mt015-leak-test-{}", ModelId::new_v7());
    let keychain = Arc::new(OsKeychainSecretsVault::new(namespace.clone()));
    let service = CloudModelAccess::with_vault(keychain.clone(), "OsKeychainSecretsVault");

    // (2) Store the canary via the REAL settings path (SecretString end-to-end).
    service
        .store_byok_key(ByokProvider::OpenAi, &SecretString::from(CANARY_KEY.to_string()))
        .expect("store canary in OS keychain");

    // (3) The key round-trips OUT of the vault for use.
    let round_tripped = service
        .fetch_byok_key(ByokProvider::OpenAi)
        .expect("canary round-trips out of the vault");

    // Wire the SAME vault lane into the BYOK runtime via VaultApiKeyProvider —
    // exactly how the MT-006 cloud backend consumes the stored key.
    let vault_provider =
        VaultApiKeyProvider::new(keychain.clone(), ByokProvider::OpenAi.vault_lane());
    let provider_debug = format!("{vault_provider:?}");
    let provider: Arc<dyn ApiKeyProvider> = Arc::new(vault_provider);

    // (4) Drive a live cloud call against wiremock, capturing the request +
    // audit rows.
    let mock_server = MockServer::start().await;
    let inspector = Arc::new(RequestInspector::default());
    let inspector_for_mock = inspector.clone();
    Mock::given(method("POST"))
        .and(path(OPENAI_CHAT_COMPLETIONS_PATH))
        .respond_with(move |req: &Request| {
            inspector_for_mock.record(req);
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(sse_payload())
        })
        .expect(1)
        .mount(&mock_server)
        .await;

    let sink = Arc::new(CapturingSink::default());
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("client");
    let runtime = OpenAiByokRuntime::with_client(mock_server.uri(), client, provider, sink.clone());
    let runtime_debug = format!("{runtime:?}");
    let service_debug = format!("{service:?}");
    let enumeration_json =
        serde_json::to_string(&service.enumerate()).expect("enumeration serialises");

    let handle = runtime
        .register_handle("gpt-4o-2024-08-06", "2026-07-02T00:00:00Z")
        .expect("allowlisted model");
    let mut stream = runtime.chat_completions_stream(generate_request(handle.model_id));
    let mut produced = String::new();
    while let Some(item) = stream.next().await {
        let token = item.expect("success-path stream item");
        produced.push_str(&token.text);
    }

    // (6) No-consent-on-save: a fresh gate still fails closed after the store.
    let gate = ConsentGate::new();
    let first_launch = gate.check_or_prompt(
        "session-mt015",
        ByokProvider::OpenAi.id(),
        &AlwaysDeny,
    );

    // Snapshot everything we need to assert on BEFORE cleanup.
    let audit_rows = sink.rows.lock().unwrap().clone();
    let requests = inspector.snapshot();
    let trace_output = String::from_utf8_lossy(&trace_buf.0.lock().unwrap()).into_owned();

    // ---- CLEAN UP THE KEYCHAIN FIRST (so a failing assertion leaves nothing).
    service
        .remove_byok_key(ByokProvider::OpenAi)
        .expect("remove canary from keychain");
    // Idempotent second delete tolerated.
    let _ = keychain.delete(ByokProvider::OpenAi.vault_lane());

    // ---- ASSERTIONS -------------------------------------------------------

    // Round-trip: the key came back out of the vault usable, and the call
    // actually used it (proving the stored key is live, not a no-op).
    assert_eq!(round_tripped, CANARY_KEY, "key must round-trip out of the vault");
    assert_eq!(produced, "Hello", "cloud call must succeed using the stored key");

    // The canary appears on the wire ONLY as the Authorization bearer header.
    assert_eq!(requests.len(), 1, "exactly one cloud request");
    let req = &requests[0];
    let auth = req
        .headers
        .get("authorization")
        .expect("authorization header present")
        .to_str()
        .expect("ascii header");
    assert_eq!(auth, format!("Bearer {CANARY_KEY}"), "key is the bearer token");
    let body = String::from_utf8_lossy(&req.body);
    assert!(!body.contains(CANARY_KEY), "key must NOT appear in the request body");

    // The canary is absent from every audit row (the cloud_invocations surface).
    for row in &audit_rows {
        let row_debug = format!("{row:?}");
        assert!(
            !row_debug.contains(CANARY_KEY),
            "cloud-invocation audit row leaked the key: {row_debug}"
        );
    }
    assert!(!audit_rows.is_empty(), "the call must have produced audit rows");

    // The canary is absent from captured tracing (FR/EventLedger/audit logging
    // all surface here in the current wiring).
    assert!(
        !trace_output.contains(CANARY_KEY),
        "the key leaked into tracing/Flight-Recorder/EventLedger output"
    );

    // The canary is absent from every Debug / enumeration surface.
    assert!(!runtime_debug.contains(CANARY_KEY), "runtime Debug leaked the key");
    assert!(!service_debug.contains(CANARY_KEY), "access service Debug leaked the key");
    assert!(!provider_debug.contains(CANARY_KEY), "vault provider Debug leaked the key");
    assert!(
        !enumeration_json.contains(CANARY_KEY),
        "enumeration JSON leaked the key"
    );

    // No-consent-on-save: the first lane launch still fails closed.
    assert!(
        matches!(first_launch, Err(ConsentGateError::ConsentDenied { .. })),
        "saving a key must NOT pre-approve consent; first launch must fail closed"
    );

    // After removal the provider reports unavailable again.
    assert_eq!(
        service.byok_status(ByokProvider::OpenAi),
        handshake_core::model_runtime::cloud::ProviderAccessStatus::Unavailable
    );
}

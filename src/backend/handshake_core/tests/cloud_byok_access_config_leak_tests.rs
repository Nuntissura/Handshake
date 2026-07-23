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
//! * captured application `tracing` output. This test does not claim
//!   Flight-Recorder/EventLedger coverage because no real FR/EventLedger sink is
//!   attached here; the explicit cloud invocation audit sink is asserted
//!   separately;
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
//! The full-path leak scenario runs cross-platform against the production
//! in-memory vault boundary. A second Windows-only proof keeps the real
//! Credential Manager round-trip. That proof uses a UNIQUE service namespace
//! and deletes the key BEFORE assertions run so a failing assertion never
//! leaves a credential behind.

use std::io::Write;
use std::sync::{Arc, Mutex, Once, OnceLock};
use std::time::Duration;

use futures::StreamExt;
use handshake_core::model_runtime::cloud::{
    ApiKeyProvider, ByokProvider, CloudInvocationAuditRow, CloudInvocationAuditSink,
    CloudModelAccess, ConsentDecision, ConsentGate, ConsentGateError, ConsentProvider,
    InMemorySecretsVault, OpenAiByokError, OpenAiByokRuntime, SecretsVault,
    VaultApiKeyProvider, OPENAI_CHAT_COMPLETIONS_PATH,
};
#[cfg(all(feature = "os-keychain", target_os = "windows"))]
use handshake_core::model_runtime::cloud::OsKeychainSecretsVault;
use handshake_core::model_runtime::{
    CancellationToken, GenPrompt, GenerateRequest, ModelId, SamplingParams,
};
use secrecy::SecretString;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

/// The canary. If this string appears anywhere it should not, the test fails.
const CANARY_KEY: &str = "sk-CANARY-mt015-NEVER-LEAK-THIS-KEY-0xDEADBEEF";
const TRACE_CAPTURE_MARKER: &str = "mt017-byok-tracing-capture-installed";

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
/// the canary never reaches ordinary application tracing.
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
        tracing_subscriber::fmt()
            .with_writer(buf.clone())
            .with_max_level(tracing::Level::TRACE)
            .with_ansi(false)
            .try_init()
            .expect("install dedicated MT-017 tracing capture subscriber");
    });
    tracing::info!(
        target: "handshake_core::tests::mt017_byok",
        marker = TRACE_CAPTURE_MARKER,
        "MT-017 public tracing capture marker"
    );
    buf
}

/// Armed immediately after a test key is stored. A panic/error anywhere after
/// storage still deletes the key, while `cleanup_now` performs and proves the
/// same idempotent double-delete before assertions inspect post-removal state.
struct StoredVaultKeyCleanup<V: SecretsVault> {
    vault: Arc<V>,
    lane: String,
    armed: bool,
}

impl<V: SecretsVault> StoredVaultKeyCleanup<V> {
    fn new(vault: Arc<V>, lane: impl Into<String>) -> Self {
        Self {
            vault,
            lane: lane.into(),
            armed: true,
        }
    }

    fn cleanup_now(&mut self) {
        self.vault
            .delete(&self.lane)
            .expect("delete stored BYOK key during cleanup");
        match self.vault.delete(&self.lane) {
            Ok(())
            | Err(
                handshake_core::model_runtime::cloud::SecretsVaultError::NoSecretForLane(_),
            ) => {}
            Err(err) => panic!("BYOK idempotent cleanup retry failed: {err}"),
        }
        match self.vault.get(&self.lane) {
            Err(handshake_core::model_runtime::cloud::SecretsVaultError::NoSecretForLane(_)) => {
                self.armed = false;
            }
            Ok(_) => panic!("BYOK cleanup must verify the key is absent before disarming"),
            Err(err) => panic!("BYOK cleanup absence verification failed: {err}"),
        }
    }
}

impl<V: SecretsVault> Drop for StoredVaultKeyCleanup<V> {
    fn drop(&mut self) {
        if self.armed {
            let _ = self.vault.delete(&self.lane);
            let _ = self.vault.delete(&self.lane);
            match self.vault.get(&self.lane) {
                Err(
                    handshake_core::model_runtime::cloud::SecretsVaultError::NoSecretForLane(_),
                ) => {}
                Ok(_) => tracing::error!(
                    target: "handshake_core::tests::mt017_byok_cleanup",
                    credential_id = %self.lane,
                    recovery_action = "retry SecretsVault::delete for credential_id",
                    "BYOK cleanup retry exhausted; credential remains present"
                ),
                Err(_) => tracing::error!(
                    target: "handshake_core::tests::mt017_byok_cleanup",
                    credential_id = %self.lane,
                    recovery_action = "probe credential_id then retry SecretsVault::delete",
                    "BYOK cleanup retry outcome is unverified"
                ),
            }
        }
    }
}

struct FailFirstDeleteVault {
    inner: InMemorySecretsVault,
    delete_attempts: std::sync::atomic::AtomicUsize,
    fail_first: bool,
    fail_all: bool,
}

impl Default for FailFirstDeleteVault {
    fn default() -> Self {
        Self {
            inner: InMemorySecretsVault::default(),
            delete_attempts: std::sync::atomic::AtomicUsize::new(0),
            fail_first: true,
            fail_all: false,
        }
    }
}

impl SecretsVault for FailFirstDeleteVault {
    fn put(
        &self,
        lane: &str,
        secret: &str,
    ) -> Result<(), handshake_core::model_runtime::cloud::SecretsVaultError> {
        self.inner.put(lane, secret)
    }

    fn get(
        &self,
        lane: &str,
    ) -> Result<zeroize::Zeroizing<String>, handshake_core::model_runtime::cloud::SecretsVaultError>
    {
        self.inner.get(lane)
    }

    fn delete(
        &self,
        lane: &str,
    ) -> Result<(), handshake_core::model_runtime::cloud::SecretsVaultError> {
        let attempt = self
            .delete_attempts
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if self.fail_all {
            return Err(handshake_core::model_runtime::cloud::SecretsVaultError::KeychainBackend(
                "injected persistent delete failure".to_string(),
            ));
        }
        if self.fail_first && attempt == 0 {
            return Err(handshake_core::model_runtime::cloud::SecretsVaultError::KeychainBackend(
                "injected first delete failure".to_string(),
            ));
        }
        let repeated_after_success = if self.fail_first {
            attempt > 1
        } else {
            attempt > 0
        };
        if repeated_after_success {
            return Err(
                handshake_core::model_runtime::cloud::SecretsVaultError::NoSecretForLane(
                    lane.to_string(),
                ),
            );
        }
        self.inner.delete(lane)
    }

    fn list_lanes(
        &self,
    ) -> Result<Vec<String>, handshake_core::model_runtime::cloud::SecretsVaultError> {
        self.inner.list_lanes()
    }
}

#[test]
fn cleanup_guard_accepts_second_delete_not_found_only_after_absence_verification() {
    const LANE: &str = "openai-delete-then-not-found";
    let vault = Arc::new(FailFirstDeleteVault {
        inner: InMemorySecretsVault::default(),
        delete_attempts: std::sync::atomic::AtomicUsize::new(0),
        fail_first: false,
        fail_all: false,
    });
    vault.put(LANE, CANARY_KEY).expect("store cleanup proof key");

    {
        let mut cleanup = StoredVaultKeyCleanup::new(vault.clone(), LANE);
        cleanup.cleanup_now();
    }

    assert!(matches!(
        vault.get(LANE),
        Err(handshake_core::model_runtime::cloud::SecretsVaultError::NoSecretForLane(_))
    ));
    assert_eq!(
        vault
            .delete_attempts
            .load(std::sync::atomic::Ordering::SeqCst),
        2,
        "verified absence disarms the guard instead of triggering Drop retries"
    );
}

#[test]
fn cleanup_guard_persistent_failure_is_observable_and_recoverable_without_secret_echo() {
    const LANE: &str = "openai-persistent-cleanup-failure";
    let trace_buffer = capture_tracing();
    let vault = Arc::new(FailFirstDeleteVault {
        inner: InMemorySecretsVault::default(),
        delete_attempts: std::sync::atomic::AtomicUsize::new(0),
        fail_first: false,
        fail_all: true,
    });
    vault.put(LANE, CANARY_KEY).expect("store cleanup proof key");

    {
        let _cleanup = StoredVaultKeyCleanup::new(vault.clone(), LANE);
    }

    assert_eq!(
        vault
            .delete_attempts
            .load(std::sync::atomic::Ordering::SeqCst),
        2,
        "Drop performs two bounded cleanup retries"
    );
    assert_eq!(vault.get(LANE).expect("injected delete failure retains key").as_str(), CANARY_KEY);
    let tracing = String::from_utf8_lossy(&trace_buffer.0.lock().unwrap()).into_owned();
    assert!(tracing.contains(LANE), "cleanup log must identify the non-secret credential id");
    assert!(tracing.contains("retry SecretsVault::delete"), "cleanup log must provide recovery action");
    assert!(!tracing.contains(CANARY_KEY), "cleanup observability must not echo credential material");
}

#[test]
fn cleanup_guard_retries_in_drop_after_first_delete_panics() {
    const LANE: &str = "openai-fail-first-cleanup";
    let vault = Arc::new(FailFirstDeleteVault::default());
    vault.put(LANE, CANARY_KEY).expect("store cleanup proof key");

    let unwind = std::panic::catch_unwind(std::panic::AssertUnwindSafe({
        let vault = vault.clone();
        move || {
            let mut cleanup = StoredVaultKeyCleanup::new(vault, LANE);
            cleanup.cleanup_now();
        }
    }));

    assert!(unwind.is_err(), "injected first deletion must unwind");
    assert!(matches!(
        vault.get(LANE),
        Err(handshake_core::model_runtime::cloud::SecretsVaultError::NoSecretForLane(_))
    ));
    assert_eq!(
        vault
            .delete_attempts
            .load(std::sync::atomic::Ordering::SeqCst),
        3,
        "one failed cleanup attempt plus two idempotent Drop attempts"
    );
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
async fn byok_canary_key_never_leaks_cross_platform_in_memory_vault() {
    assert_byok_canary_never_leaks(
        Arc::new(InMemorySecretsVault::default()),
        "InMemorySecretsVault",
    )
    .await;
}

#[cfg(all(feature = "os-keychain", target_os = "windows"))]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn byok_canary_key_never_leaks_and_round_trips_only_through_os_keychain() {
    // The PRODUCTION wiring must be the OS keychain, never in-memory.
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
    assert_byok_canary_never_leaks(keychain, "OsKeychainSecretsVault").await;
}

async fn assert_byok_canary_never_leaks<V>(keychain: Arc<V>, vault_kind: &'static str)
where
    V: SecretsVault + 'static,
{
    let trace_buf = capture_tracing();
    let service = CloudModelAccess::with_vault(keychain.clone(), vault_kind);

    // (2) Store the canary via the REAL settings path. The key is a SecretString
    // at the trust boundaries; only a transient String exists on the transport.
    service
        .store_byok_key(ByokProvider::OpenAi, &SecretString::from(CANARY_KEY.to_string()))
        .expect("store canary in OS keychain");
    let mut key_cleanup = StoredVaultKeyCleanup::new(
        keychain.clone(),
        ByokProvider::OpenAi.vault_lane(),
    );

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
    key_cleanup.cleanup_now();

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

    assert!(
        trace_output.contains(TRACE_CAPTURE_MARKER),
        "the dedicated tracing capture must contain its public installation marker"
    );
    // The canary is absent from captured ordinary application tracing.
    assert!(
        !trace_output.contains(CANARY_KEY),
        "the key leaked into application tracing output"
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

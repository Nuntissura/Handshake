//! MT-015 (F-i): HTTP route tests for the operator cloud-model access surface.
//!
//! Before this MT the `/model-access` handlers hardcoded
//! `CloudModelAccess::production()` (the OS keychain), so the operator-facing
//! BYOK route was structurally untestable and had ZERO tests. The handlers now
//! resolve the service through a `CloudAccessProvider` seam on
//! `ModelAccessState`. These tests inject:
//!
//! * an in-memory-vault-backed provider (a shared `InMemorySecretsVault`), so
//!   the store/enumerate paths are exercised WITHOUT touching the host keychain
//!   and WITHOUT building a full `AppState` (no PostgreSQL); and
//! * a provider that always reports the keychain as unavailable, so the
//!   fail-closed 503 path is reachable deterministically.
//!
//! Proven here:
//! * `PUT store` -> 200 and the response body NEVER echoes the key (the key is
//!   written only to the vault and round-trips back out);
//! * `PUT gemini` -> 404 (Gemini is excluded by construction);
//! * `PUT` empty key -> 400;
//! * keychain-unavailable -> 503;
//! * `GET providers` reflects the stored key as `configured`, excludes Gemini,
//!   and never carries key material over HTTP.
//!
//! The OS-keychain leak proof (key stored only in the keychain, never in logs /
//! FR / EventLedger / audit rows) lives in `cloud_byok_access_config_leak_tests`.

use std::collections::BTreeMap;
use std::io::Write;
use std::sync::{Arc, Mutex, Once, OnceLock, RwLock};

use handshake_core::api::model_access::{routes, CloudAccessProvider, ModelAccessState};
use handshake_core::model_runtime::cloud::{
    AccessConfigError, ByokProvider, CliBridgeAuthStatus, CliBridgeAuthStatusProbe,
    CliBridgeLoginLaunchError, CliBridgeLoginLauncher, CliBridgeProvider, CloudModelAccess,
    ForegroundCliLaunchHandle, InMemorySecretsVault, SecretsVault, SecretsVaultError,
};
use serde_json::Value;
use zeroize::Zeroizing;

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
            .expect("install model-access route tracing capture");
    });
    tracing::info!(
        target: "handshake_core::tests::model_access",
        marker = "model-access-tracing-live",
        "public tracing capture marker"
    );
    buffer
}

struct EchoingFailVault;

impl SecretsVault for EchoingFailVault {
    fn put(&self, _lane: &str, secret: &str) -> Result<(), SecretsVaultError> {
        Err(SecretsVaultError::KeychainBackend(format!(
            "injected backend echoed submitted value {secret}"
        )))
    }

    fn get(&self, lane: &str) -> Result<Zeroizing<String>, SecretsVaultError> {
        Err(SecretsVaultError::NoSecretForLane(lane.to_string()))
    }

    fn delete(&self, _lane: &str) -> Result<(), SecretsVaultError> {
        Ok(())
    }

    fn list_lanes(&self) -> Result<Vec<String>, SecretsVaultError> {
        Ok(Vec::new())
    }
}

struct EchoingFailProvider;

impl CloudAccessProvider for EchoingFailProvider {
    fn access(&self) -> Result<CloudModelAccess, AccessConfigError> {
        Ok(CloudModelAccess::with_vault(
            Arc::new(EchoingFailVault),
            "EchoingFailVault",
        ))
    }
}

/// Provider backed by a shared in-memory vault so a stored key persists across
/// requests within a test. Never touches the host keychain.
struct InMemoryProvider {
    vault: Arc<InMemorySecretsVault>,
}

impl CloudAccessProvider for InMemoryProvider {
    fn access(&self) -> Result<CloudModelAccess, AccessConfigError> {
        Ok(CloudModelAccess::with_vault(
            self.vault.clone(),
            "InMemorySecretsVault",
        ))
    }
}

/// Provider that always fails as if the OS keychain feature were disabled, so
/// the fail-closed 503 path is testable with default features on.
struct KeychainUnavailableProvider;

impl CloudAccessProvider for KeychainUnavailableProvider {
    fn access(&self) -> Result<CloudModelAccess, AccessConfigError> {
        Err(AccessConfigError::KeychainUnavailable)
    }
}

#[derive(Default)]
struct TypedCliAuthProbe {
    statuses: RwLock<BTreeMap<CliBridgeProvider, CliBridgeAuthStatus>>,
    calls: Mutex<Vec<CliBridgeProvider>>,
}

impl TypedCliAuthProbe {
    fn set_all(&self, status: CliBridgeAuthStatus) {
        let mut statuses = self.statuses.write().expect("auth status lock");
        statuses.clear();
        for provider in CliBridgeProvider::OFFERED {
            statuses.insert(provider, status);
        }
    }

    fn calls(&self) -> Vec<CliBridgeProvider> {
        self.calls.lock().expect("auth calls lock").clone()
    }
}

impl CliBridgeAuthStatusProbe for TypedCliAuthProbe {
    fn auth_status(&self, provider: CliBridgeProvider) -> CliBridgeAuthStatus {
        self.calls.lock().expect("auth calls lock").push(provider);
        self.statuses
            .read()
            .expect("auth status lock")
            .get(&provider)
            .copied()
            .unwrap_or(CliBridgeAuthStatus::Unavailable)
    }
}

#[derive(Default)]
struct CapturingCliLoginLauncher {
    calls: Mutex<Vec<CliBridgeProvider>>,
}

impl CliBridgeLoginLauncher for CapturingCliLoginLauncher {
    fn launch_login(
        &self,
        provider: CliBridgeProvider,
    ) -> Result<ForegroundCliLaunchHandle, CliBridgeLoginLaunchError> {
        self.calls.lock().expect("login calls lock").push(provider);
        Ok(ForegroundCliLaunchHandle { pid: 4242 })
    }
}

fn in_memory_state() -> (ModelAccessState, Arc<InMemorySecretsVault>) {
    let vault = Arc::new(InMemorySecretsVault::default());
    let state = ModelAccessState::with_provider_and_cli_auth_probe(
        Arc::new(InMemoryProvider {
            vault: vault.clone(),
        }),
        Arc::new(TypedCliAuthProbe::default()),
    );
    (state, vault)
}

async fn start_server(state: ModelAccessState) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback listener");
    let addr = listener.local_addr().expect("local addr");
    let app = routes(state);
    let handle = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("model-access server");
    });
    (format!("http://{addr}"), handle)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_login_route_returns_only_backend_owned_launch_handle() {
    let vault = Arc::new(InMemorySecretsVault::default());
    let launcher = Arc::new(CapturingCliLoginLauncher::default());
    let state = ModelAccessState::with_provider_cli_runtime(
        Arc::new(InMemoryProvider { vault }),
        Arc::new(TypedCliAuthProbe::default()),
        launcher.clone(),
    );
    let (base, server) = start_server(state).await;
    let response = reqwest::Client::new()
        .post(format!("{base}/model-access/cli-bridge/codex/login"))
        .json(&serde_json::json!({}))
        .send()
        .await
        .expect("launch login request");
    assert_eq!(response.status().as_u16(), 200);
    let body = response.text().await.expect("login launch body");
    assert!(!body.contains("executable"), "{body}");
    assert!(!body.contains("args"), "{body}");
    assert!(!body.contains("PATH"), "{body}");
    let value: Value = serde_json::from_str(&body).expect("login launch JSON");
    assert_eq!(value["provider"], "codex");
    assert_eq!(value["launch_handle"]["pid"], 4242);
    assert_eq!(
        launcher.calls.lock().expect("login calls lock").as_slice(),
        &[CliBridgeProvider::Codex]
    );
    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn put_store_returns_200_and_never_echoes_the_key() {
    const CANARY: &str = "sk-route-canary-DO-NOT-ECHO-0xDEADBEEF";
    let (state, vault) = in_memory_state();
    let (base, server) = start_server(state).await;

    let client = reqwest::Client::new();
    let resp = client
        .put(format!("{base}/model-access/byok/openai/key"))
        .json(&serde_json::json!({ "api_key": CANARY }))
        .send()
        .await
        .expect("send store request");
    assert_eq!(resp.status().as_u16(), 200, "store must succeed");
    let body = resp.text().await.expect("read body");
    assert!(
        !body.contains(CANARY),
        "the response body must NEVER echo the key: {body}"
    );
    assert!(
        body.contains("\"status\":\"configured\""),
        "store returns a non-secret confirmation: {body}"
    );

    // The store path actually reached the vault (not just returned 200): the key
    // round-trips back OUT of the shared in-memory vault under the provider lane.
    let stored = vault
        .get(ByokProvider::OpenAi.vault_lane())
        .expect("key stored under the openai vault lane");
    assert_eq!(stored.as_str(), CANARY, "the key was written to the vault");

    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn put_store_vault_failure_logs_only_stable_error_code() {
    const CANARY: &str = "sk-route-vault-error-canary-NEVER-ECHO";
    let trace_buffer = capture_tracing();
    let state = ModelAccessState::with_provider(Arc::new(EchoingFailProvider));
    let (base, server) = start_server(state).await;

    let response = reqwest::Client::new()
        .put(format!("{base}/model-access/byok/openai/key"))
        .json(&serde_json::json!({ "api_key": CANARY }))
        .send()
        .await
        .expect("send failing store request");
    assert_eq!(response.status().as_u16(), 500);
    let body = response.text().await.expect("read stable error body");
    let tracing = String::from_utf8_lossy(&trace_buffer.0.lock().unwrap()).into_owned();

    assert!(body.contains("vault_error"), "{body}");
    assert!(tracing.contains("model-access-tracing-live"), "tracing capture must be live");
    assert!(tracing.contains("keychain_backend"), "stable vault error class must be logged");
    assert!(!body.contains(CANARY), "response leaked submitted key: {body}");
    assert!(!tracing.contains(CANARY), "tracing leaked submitted key: {tracing}");

    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn put_gemini_is_404_excluded() {
    let (state, vault) = in_memory_state();
    let (base, server) = start_server(state).await;

    let client = reqwest::Client::new();
    let resp = client
        .put(format!("{base}/model-access/byok/gemini/key"))
        .json(&serde_json::json!({ "api_key": "sk-should-never-be-stored" }))
        .send()
        .await
        .expect("send gemini store request");
    assert_eq!(
        resp.status().as_u16(),
        404,
        "Gemini is not an offered BYOK provider"
    );

    // Nothing was stored for any offered lane.
    assert!(vault.list_lanes().expect("list").is_empty());

    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn put_empty_key_is_400() {
    let (state, _vault) = in_memory_state();
    let (base, server) = start_server(state).await;

    let client = reqwest::Client::new();
    let resp = client
        .put(format!("{base}/model-access/byok/openai/key"))
        .json(&serde_json::json!({ "api_key": "" }))
        .send()
        .await
        .expect("send empty-key request");
    assert_eq!(
        resp.status().as_u16(),
        400,
        "an empty API key is rejected before touching the vault"
    );

    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn keychain_unavailable_is_503() {
    let state = ModelAccessState::with_provider(Arc::new(KeychainUnavailableProvider));
    let (base, server) = start_server(state).await;

    let client = reqwest::Client::new();
    let resp = client
        .put(format!("{base}/model-access/byok/anthropic/key"))
        .json(&serde_json::json!({ "api_key": "sk-real-key-but-no-keychain" }))
        .send()
        .await
        .expect("send store request");
    assert_eq!(
        resp.status().as_u16(),
        503,
        "a disabled keychain fails closed with 503, never a plaintext fallback"
    );
    let body = resp.text().await.expect("read body");
    assert!(
        body.contains("keychain_unavailable"),
        "503 carries the stable error code: {body}"
    );

    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_providers_reflects_configured_and_excludes_gemini() {
    const CANARY: &str = "sk-enum-canary-NEVER-OVER-HTTP";
    let (state, _vault) = in_memory_state();
    let (base, server) = start_server(state).await;

    let client = reqwest::Client::new();
    // Store a key for openai, then enumerate.
    let store = client
        .put(format!("{base}/model-access/byok/openai/key"))
        .json(&serde_json::json!({ "api_key": CANARY }))
        .send()
        .await
        .expect("store");
    assert_eq!(store.status().as_u16(), 200);

    let resp = client
        .get(format!("{base}/model-access/providers"))
        .send()
        .await
        .expect("enumerate");
    assert_eq!(resp.status().as_u16(), 200);
    let body = resp.text().await.expect("body");
    assert!(
        !body.contains(CANARY),
        "the non-secret enumeration must NEVER carry key material: {body}"
    );

    let v: Value = serde_json::from_str(&body).expect("json");
    let byok = v["byok"].as_array().expect("byok array");
    let openai = byok
        .iter()
        .find(|r| r["provider"] == "openai")
        .expect("openai row");
    assert_eq!(
        openai["status"], "configured",
        "the stored key shows configured"
    );
    let anthropic = byok
        .iter()
        .find(|r| r["provider"] == "anthropic")
        .expect("anthropic row");
    assert_eq!(
        anthropic["status"], "unavailable",
        "un-keyed provider is unavailable"
    );
    assert!(
        byok.iter().all(|r| r["provider"] != "gemini"),
        "Gemini is never an offered BYOK row"
    );
    assert!(
        v["excluded"]
            .as_array()
            .expect("excluded array")
            .iter()
            .any(|e| e == "gemini"),
        "Gemini is surfaced only as an explicit exclusion"
    );

    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_bridge_typed_status_wire_mapping_excludes_account_fields_and_gemini() {
    let vault = Arc::new(InMemorySecretsVault::default());
    let probe = Arc::new(TypedCliAuthProbe::default());
    let state = ModelAccessState::with_provider_and_cli_auth_probe(
        Arc::new(InMemoryProvider { vault }),
        probe.clone(),
    );
    let (base, server) = start_server(state).await;
    let client = reqwest::Client::new();

    for (typed_status, wire_status) in [
        (CliBridgeAuthStatus::LoggedIn, "logged_in"),
        (CliBridgeAuthStatus::LoggedOut, "logged_out"),
        (CliBridgeAuthStatus::Expired, "expired"),
    ] {
        probe.set_all(typed_status);
        let response = client
            .get(format!("{base}/model-access/providers"))
            .send()
            .await
            .expect("enumerate CLI auth status");
        assert_eq!(response.status().as_u16(), 200);
        let body = response.text().await.expect("auth status body");
        assert!(
            !body.contains("access_token")
                && !body.contains("refresh_token")
                && !body.contains("\"email\""),
            "the typed fake status surface unexpectedly contained account fields: {body}"
        );
        let value: Value = serde_json::from_str(&body).expect("auth status JSON");
        let rows = value["cli_bridge"].as_array().expect("CLI bridge rows");
        assert_eq!(rows.len(), 2, "only Claude Code and Codex are offered");
        for provider in ["claude_code", "codex"] {
            let row = rows
                .iter()
                .find(|row| row["provider"] == provider)
                .unwrap_or_else(|| panic!("{provider} auth row"));
            assert_eq!(row["auth_status"], wire_status, "{provider} auth state");
        }
        assert!(
            rows.iter()
                .all(|row| row["provider"] != "gemini" && row["provider"] != "gemini_cli"),
            "Gemini must never be offered or probed: {rows:?}"
        );
    }

    let calls = probe.calls();
    assert_eq!(calls.len(), 6, "two offered providers across three probes");
    assert!(
        calls.iter().all(|provider| matches!(
            provider,
            CliBridgeProvider::ClaudeCode | CliBridgeProvider::Codex
        )),
        "the auth probe must never receive Gemini or any unknown provider"
    );
    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_byok_key_is_idempotent_and_updates_status() {
    const CANARY: &str = "sk-delete-canary-NEVER-OVER-HTTP";
    let (state, vault) = in_memory_state();
    let (base, server) = start_server(state).await;

    let client = reqwest::Client::new();
    let store = client
        .put(format!("{base}/model-access/byok/openai/key"))
        .json(&serde_json::json!({ "api_key": CANARY }))
        .send()
        .await
        .expect("store");
    assert_eq!(store.status().as_u16(), 200);
    assert!(
        vault
            .get(ByokProvider::OpenAi.vault_lane())
            .expect("stored key")
            .as_str()
            == CANARY,
        "precondition: the key was stored in the vault"
    );

    for attempt in 1..=2 {
        let resp = client
            .delete(format!("{base}/model-access/byok/openai/key"))
            .send()
            .await
            .unwrap_or_else(|err| panic!("delete attempt {attempt}: {err}"));
        assert_eq!(
            resp.status().as_u16(),
            200,
            "delete attempt {attempt} must be idempotent"
        );
        let body = resp.text().await.expect("read delete body");
        assert!(
            !body.contains(CANARY),
            "delete response must NEVER echo the key: {body}"
        );
        assert!(
            body.contains("\"status\":\"unavailable\""),
            "delete returns non-secret unavailable status: {body}"
        );
    }

    assert!(
        vault.get(ByokProvider::OpenAi.vault_lane()).is_err(),
        "delete removes the key from the vault"
    );

    let providers = client
        .get(format!("{base}/model-access/providers"))
        .send()
        .await
        .expect("enumerate after delete");
    assert_eq!(providers.status().as_u16(), 200);
    let body = providers.text().await.expect("provider body");
    assert!(
        !body.contains(CANARY),
        "enumeration after delete must not expose the old key: {body}"
    );
    let v: Value = serde_json::from_str(&body).expect("json");
    let byok = v["byok"].as_array().expect("byok array");
    let openai = byok
        .iter()
        .find(|r| r["provider"] == "openai")
        .expect("openai row");
    assert_eq!(
        openai["status"], "unavailable",
        "deleted key must show unavailable"
    );

    server.abort();
}

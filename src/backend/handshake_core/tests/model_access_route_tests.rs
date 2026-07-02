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

use std::sync::Arc;

use handshake_core::api::model_access::{routes, CloudAccessProvider, ModelAccessState};
use handshake_core::model_runtime::cloud::{
    AccessConfigError, ByokProvider, CloudModelAccess, InMemorySecretsVault, SecretsVault,
};
use serde_json::Value;

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

fn in_memory_state() -> (ModelAccessState, Arc<InMemorySecretsVault>) {
    let vault = Arc::new(InMemorySecretsVault::default());
    let state = ModelAccessState::with_provider(Arc::new(InMemoryProvider {
        vault: vault.clone(),
    }));
    (state, vault)
}

async fn start_server(state: ModelAccessState) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback listener");
    let addr = listener.local_addr().expect("local addr");
    let app = routes(state);
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("model-access server");
    });
    (format!("http://{addr}"), handle)
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
    assert_eq!(openai["status"], "configured", "the stored key shows configured");
    let anthropic = byok
        .iter()
        .find(|r| r["provider"] == "anthropic")
        .expect("anthropic row");
    assert_eq!(anthropic["status"], "unavailable", "un-keyed provider is unavailable");
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

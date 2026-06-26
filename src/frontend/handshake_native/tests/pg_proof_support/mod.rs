//! WP-KERNEL-012 MT-044 — shared live-PostgreSQL backend client for the E2/E3/E4 parity proofs.
//! Lives in a `tests/` SUBDIRECTORY so Cargo does not compile it as a standalone test binary.
//!
//! ## Honest gating (no mock smuggling, no silent skip)
//!
//! The E2/E3/E4 proofs BIND the handshake_core backend and need a live managed PostgreSQL. This module
//! is the single precondition + HTTP surface they share. [`require_live_backend`] resolves the backend
//! base URL (`HSK_TEST_BASE`, default `http://127.0.0.1:37501`) and the seeded workspace id
//! (`HSK_TEST_WORKSPACE_ID`). When the env is unset OR the backend is unreachable, it PANICS with a
//! descriptive `requires_pg` message — so running these `#[ignore]`d proofs with `--ignored` against no
//! backend fails honestly, never fake-passes. There is NO sqlite and NO in-memory substitute here: the
//! only durable authority is real PostgreSQL behind handshake_core (the PostgreSQL/EventLedger duty).
//!
//! The shared HTTP client is the crate's `handshake_native::backend_client::shared_http_client` (the
//! production async reqwest client), driven on a current-thread tokio runtime — the same async surface
//! the editor uses, so the proofs hit the REAL routes.

#![allow(dead_code)] // each suite calls a subset of the request helpers.

use std::time::Duration;

use handshake_native::backend_client::shared_http_client;

/// The default backend base — the managed handshake_core dev port (matches the WP-011/012 test base).
pub const DEFAULT_BASE: &str = "http://127.0.0.1:37501";

/// A resolved, reachable live backend handle for the PG-binding parity proofs. Construction proves the
/// precondition (env + reachability); the request helpers then hit real routes on a private runtime.
pub struct LiveBackend {
    pub base: String,
    pub workspace_id: String,
    client: reqwest::Client,
    rt: tokio::runtime::Runtime,
}

/// Resolve + verify the live backend, or PANIC with a `requires_pg` message (no fake-pass, no skip).
pub fn require_live_backend() -> LiveBackend {
    let base = std::env::var("HSK_TEST_BASE").unwrap_or_else(|_| DEFAULT_BASE.to_owned());
    let workspace_id = std::env::var("HSK_TEST_WORKSPACE_ID").unwrap_or_default();
    assert!(
        !workspace_id.is_empty(),
        "requires_pg: set HSK_TEST_WORKSPACE_ID to a seeded workspace on a live handshake_core + \
         PostgreSQL backend ({base}). This proof is honestly gated — it never mocks PG."
    );

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build the current-thread runtime for the live-backend proof");
    // The PRODUCTION async reqwest client (the same surface the editor uses).
    let client = shared_http_client();

    // Reachability gate: the backend health endpoint must answer, else requires_pg (honest failure).
    let base_for_health = base.clone();
    let client_for_health = client.clone();
    let healthy = rt.block_on(async move {
        match client_for_health
            .get(format!("{base_for_health}/health"))
            .timeout(Duration::from_secs(3))
            .send()
            .await
        {
            Ok(r) => r.status().is_success(),
            Err(_) => false,
        }
    });
    assert!(
        healthy,
        "requires_pg: handshake_core is not reachable at {base}/health. Start the managed backend + \
         PostgreSQL, then run with --ignored. This proof never fakes PG."
    );

    LiveBackend { base, workspace_id, client, rt }
}

impl LiveBackend {
    /// A real Loom block id from `HSK_TEST_BLOCK_ID`, or a `requires_pg` panic.
    pub fn require_block_id(&self) -> String {
        std::env::var("HSK_TEST_BLOCK_ID")
            .ok()
            .filter(|s| !s.is_empty())
            .expect("requires_pg: set HSK_TEST_BLOCK_ID to a real Loom block id in the seeded workspace")
    }

    pub fn post_json(&self, path: &str, body: &serde_json::Value) -> serde_json::Value {
        let url = format!("{}{path}", self.base);
        let (status, text) = self.rt.block_on(async {
            let resp = self
                .client
                .post(&url)
                .json(body)
                .send()
                .await
                .unwrap_or_else(|e| panic!("requires_pg: POST {url} failed: {e}"));
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            (status, text)
        });
        assert!(status.is_success(), "POST {path} -> {status}: {text}");
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("POST {path} response not JSON ({e}): {text}"))
    }

    pub fn put_json(&self, path: &str, body: &serde_json::Value) -> serde_json::Value {
        let url = format!("{}{path}", self.base);
        let (status, text) = self.rt.block_on(async {
            let resp = self
                .client
                .put(&url)
                .json(body)
                .send()
                .await
                .unwrap_or_else(|e| panic!("requires_pg: PUT {url} failed: {e}"));
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            (status, text)
        });
        assert!(status.is_success(), "PUT {path} -> {status}: {text}");
        if text.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::from_str(&text).unwrap_or(serde_json::Value::String(text))
        }
    }

    pub fn get_json(&self, path: &str) -> serde_json::Value {
        let text = self.get_text(path);
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("GET {path} response not JSON ({e}): {text}"))
    }

    pub fn get_text(&self, path: &str) -> String {
        let url = format!("{}{path}", self.base);
        let (status, text) = self.rt.block_on(async {
            let resp = self
                .client
                .get(&url)
                .send()
                .await
                .unwrap_or_else(|e| panic!("requires_pg: GET {url} failed: {e}"));
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            (status, text)
        });
        assert!(status.is_success(), "GET {path} -> {status}: {text}");
        text
    }

    pub fn get_bytes(&self, path: &str) -> Vec<u8> {
        let url = format!("{}{path}", self.base);
        let (status, bytes) = self.rt.block_on(async {
            let resp = self
                .client
                .get(&url)
                .send()
                .await
                .unwrap_or_else(|e| panic!("requires_pg: GET {url} failed: {e}"));
            let status = resp.status();
            let bytes = resp.bytes().await.map(|b| b.to_vec()).unwrap_or_default();
            (status, bytes)
        });
        assert!(status.is_success(), "GET {path} -> {status}");
        bytes
    }
}

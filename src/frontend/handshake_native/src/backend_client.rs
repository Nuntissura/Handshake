//! Backend HTTP client. Reuses the EXISTING handshake_core backend over its real HTTP API
//! (GET /health, GET/PUT /workspaces/:id/workbench/layout) — the native app never starts or embeds
//! the backend; it assumes it is running. Deserializes via serde_json::Value to avoid a build
//! dependency on the handshake_core crate.

use crate::error::AppError;
use crate::layout_persistence::{LayoutError, LayoutTransport};
use serde_json::Value;
use std::time::Duration;

/// handshake_core listens here (hardcoded in handshake_core/src/main.rs).
pub const BACKEND_BASE_URL: &str = "http://127.0.0.1:37501";
/// Health probe (CONTROL-2). Kept as a full URL for the existing MT-002 health wiring.
pub const HEALTH_URL: &str = "http://127.0.0.1:37501/health";

/// Per-request timeout for the layout endpoint. A save must not hang the UI thread; on timeout the
/// transport returns a TRANSIENT [`LayoutError::Transport`] the persistence manager retries.
const LAYOUT_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
pub struct HealthInfo {
    pub status: String,
    pub db_status: String,
    pub migration_version: Option<i64>,
}

/// GET /health with a 5s timeout (CONTROL-2). Non-success status or a parse failure is an error,
/// never a panic.
pub async fn fetch_health(url: &str) -> Result<HealthInfo, AppError> {
    let client = reqwest::Client::new();
    let resp = client
        .get(url)
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| AppError::Http(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(AppError::Http(format!("non-success status {}", resp.status())));
    }
    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Parse(e.to_string()))?;
    Ok(HealthInfo {
        status: v.get("status").and_then(|x| x.as_str()).unwrap_or("unknown").to_string(),
        db_status: v.get("db_status").and_then(|x| x.as_str()).unwrap_or("unknown").to_string(),
        migration_version: v.get("migration_version").and_then(|x| x.as_i64()),
    })
}

/// REST client for the backend's PostgreSQL-authoritative workbench-layout surface
/// (`GET`/`PUT /workspaces/:workspace_id/workbench/layout`, migration `0323_workbench_layout_state`).
///
/// This is the REAL [`LayoutTransport`] the app wires into its [`LayoutPersistenceManager`]: the
/// native layout persists THROUGH this REST endpoint into PostgreSQL/EventLedger — there is no local
/// file authority (CX-503S / Data Posture). The endpoint stores the snapshot as an opaque JSONB
/// `layout_state` blob, so this client speaks `serde_json::Value` directly and never depends on the
/// `handshake_core` crate's types.
///
/// ## Why a blocking transport over an async client
///
/// reqwest is async, but [`LayoutTransport`] is synchronous so the persistence manager stays a pure,
/// directly-unit-testable state machine. This client holds a tokio runtime [`Handle`] and bridges by
/// `Handle::block_on`. The app calls the transport from a short-lived tokio worker (NOT the egui UI
/// thread — see `HandshakeApp`'s save wiring), so the UI thread is never blocked on the network
/// (HBR-QUIET: background work must not stall the operator).
///
/// [`Handle`]: tokio::runtime::Handle
#[derive(Clone)]
pub struct WorkbenchLayoutClient {
    client: reqwest::Client,
    base_url: String,
    runtime: tokio::runtime::Handle,
}

impl WorkbenchLayoutClient {
    /// Build a client against `base_url` (e.g. [`BACKEND_BASE_URL`]) bridging onto `runtime`.
    pub fn new(base_url: impl Into<String>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            runtime,
        }
    }

    /// The production client: the hardcoded backend base URL, bridging onto the app's runtime handle.
    pub fn production(runtime: tokio::runtime::Handle) -> Self {
        Self::new(BACKEND_BASE_URL, runtime)
    }

    fn layout_url(&self, workspace_id: &str) -> String {
        format!("{}/workspaces/{}/workbench/layout", self.base_url, workspace_id)
    }
}

impl LayoutTransport for WorkbenchLayoutClient {
    /// `GET /workspaces/:id/workbench/layout`. The backend's `WorkbenchLayoutResponse` carries
    /// `layout_state: Option<Value>` — `null`/absent means no layout stored yet (first run -> `Ok(None)`).
    /// A non-success status or a transport error is a TRANSIENT [`LayoutError::Transport`].
    fn load(&self, workspace_id: &str) -> Result<Option<Value>, LayoutError> {
        let url = self.layout_url(workspace_id);
        let client = self.client.clone();
        self.runtime.block_on(async move {
            let resp = client
                .get(&url)
                .timeout(LAYOUT_REQUEST_TIMEOUT)
                .send()
                .await
                .map_err(|e| LayoutError::Transport(e.to_string()))?;
            if !resp.status().is_success() {
                return Err(LayoutError::Transport(format!(
                    "GET layout non-success status {}",
                    resp.status()
                )));
            }
            let body: Value = resp
                .json()
                .await
                .map_err(|e| LayoutError::Transport(e.to_string()))?;
            // WorkbenchLayoutResponse.layout_state is Option<Value>; null/absent => first run.
            match body.get("layout_state") {
                Some(Value::Null) | None => Ok(None),
                Some(v) => Ok(Some(v.clone())),
            }
        })
    }

    /// `PUT /workspaces/:id/workbench/layout` with `SaveWorkbenchLayoutRequest { layout_state }`.
    /// A non-success status or a transport error is a TRANSIENT [`LayoutError::Transport`] the
    /// manager retries; the in-memory layout is unaffected by a save failure.
    fn save(&self, workspace_id: &str, layout_state: Value) -> Result<(), LayoutError> {
        let url = self.layout_url(workspace_id);
        let client = self.client.clone();
        let request_body = serde_json::json!({ "layout_state": layout_state });
        self.runtime.block_on(async move {
            let resp = client
                .put(&url)
                .timeout(LAYOUT_REQUEST_TIMEOUT)
                .json(&request_body)
                .send()
                .await
                .map_err(|e| LayoutError::Transport(e.to_string()))?;
            if !resp.status().is_success() {
                return Err(LayoutError::Transport(format!(
                    "PUT layout non-success status {}",
                    resp.status()
                )));
            }
            Ok(())
        })
    }
}

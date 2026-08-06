//! MT-015: Operator cloud-model access configuration HTTP surface.
//!
//! The native egui settings dialog reaches the WP-KERNEL-004 cloud backends
//! through these routes. They wrap [`crate::model_runtime::cloud::access_config`]:
//!
//! * `GET  /model-access/providers`            — non-secret enumeration for the
//!   model picker (BYOK configured / unavailable; CLI-bridge logged-in /
//!   logged-out / expired / unavailable plus provider-owned login commands;
//!   the deliberately-excluded Gemini). Never returns key material.
//! * `PUT  /model-access/byok/:provider/key`   — store a BYOK API key ONLY in
//!   the OS-keychain vault. Body `{ "api_key": "<secret>" }`. The key is a
//!   [`secrecy::SecretString`] at the trust boundaries (request deserialisation
//!   and the `vault.put` call); a transient plaintext `String` exists only on
//!   the loopback transport (the request body the shell PUTs). It is never
//!   logged, echoed, or persisted anywhere but the keychain. Storing a key
//!   creates NO consent receipt (MT-006 fail-closed gate still applies at first
//!   launch).
//! * `DELETE /model-access/byok/:provider/key` — remove / rotate a key
//!   (idempotent), reusing `vault.delete`.
//! * `POST /model-access/cli-bridge/:provider/login` — start the provider's
//!   exact already-pinned executable graph as an IN-APP login session running
//!   inside a Handshake-hosted pseudo-terminal. No OS console window is opened
//!   and no foreground/Z-order change occurs (HBR-QUIET-001). Returns the
//!   session snapshot; no path, argv, credential, or account metadata.
//! * `GET  /model-access/cli-bridge-login/:session` — poll one login session:
//!   typed status (`pending` / `awaiting_input` / `succeeded` / `failed` /
//!   `timed_out` / `cancelled`), the provider's own terminal transcript, and the
//!   remaining bounded-window budget.
//! * `POST /model-access/cli-bridge-login/:session/input` — deliver one operator
//!   response (the device code or prompt answer) to the login process's stdin.
//!   Body `{ "input": "<text>" }`.
//! * `POST /model-access/cli-bridge-login/:session/cancel` — terminate the login
//!   process and evict the session (idempotent while the session is known).
//!
//! The session routes use the distinct `cli-bridge-login` path root so they can
//! never be confused with a provider id under `cli-bridge/:provider`.
//!
//! ## Leak discipline
//!
//! The request body deserialises directly into a [`secrecy::SecretString`], so
//! its `Debug` is `[REDACTED]` and no accidental `tracing` of the body can
//! surface the key. Handlers return only non-secret status. Error envelopes
//! carry stable codes, never the key. There is no `GET .../key` route — a
//! stored key can be used (by the backend) but never read back over HTTP.
//!
//! ## Testability seam (MT-015 F-i)
//!
//! The service is resolved through a [`CloudAccessProvider`] held on
//! [`ModelAccessState`], NOT hardcoded to [`CloudModelAccess::production`]. The
//! real server wires [`ModelAccessState::production_with_cli_runtime`] after
//! the canonical launch graph is pinned (plus the OS keychain); a route test
//! injects an in-memory-vault-backed provider via
//! [`ModelAccessState::with_provider_and_cli_auth_probe`] and mounts [`routes`]
//! directly — it never builds a full [`crate::AppState`], touches the host
//! keychain, or invokes an installed CLI.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post, put},
    Json, Router,
};
use secrecy::SecretString;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::model_runtime::cloud::access_config::{
    enumerate_with_cli_auth_probe, AccessConfigError, ByokProvider, CliBridgeAuthStatusProbe,
    CliBridgeLoginLaunchError, CliBridgeLoginLauncher, CliBridgeLoginSessionRegistry,
    CliBridgeProvider, CliLoginSessionError, CloudAccessEnumeration, CloudModelAccess,
    InMemoryAccessRegistry,
};
use crate::model_runtime::cloud::{InteractiveLoginTransport, SecretsVaultError};

type ApiError = (StatusCode, Json<Value>);

/// Request body for storing a BYOK key. The key deserialises straight into a
/// [`SecretString`] so it is redacted in `Debug` and cannot be logged by
/// accident. Nothing else is accepted.
#[derive(Deserialize)]
struct StoreKeyBody {
    api_key: SecretString,
}

/// Seam that yields the [`CloudModelAccess`] service the handlers use. Production
/// wires [`CloudModelAccess::production`] (the OS keychain); a test injects an
/// in-memory-vault-backed service so the operator-facing BYOK route is
/// structurally testable without touching the host keychain (MT-015 F-i).
pub trait CloudAccessProvider: Send + Sync {
    fn access(&self) -> Result<CloudModelAccess, AccessConfigError>;
}

/// Default provider: builds the production OS-keychain-backed service per call.
/// Under `--no-default-features` (no `os-keychain`) `production()` returns
/// [`AccessConfigError::KeychainUnavailable`], which the handlers map to a typed
/// 503 — the same fail-closed refusal to persist a key outside the keychain,
/// now reachable through the seam.
#[derive(Clone, Copy, Debug, Default)]
pub struct ProductionCloudAccessProvider;

impl CloudAccessProvider for ProductionCloudAccessProvider {
    fn access(&self) -> Result<CloudModelAccess, AccessConfigError> {
        CloudModelAccess::production()
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct UnavailableCliAuthProbe;

impl CliBridgeAuthStatusProbe for UnavailableCliAuthProbe {
    fn auth_status(
        &self,
        _provider: crate::model_runtime::cloud::CliBridgeProvider,
    ) -> crate::model_runtime::cloud::CliBridgeAuthStatus {
        crate::model_runtime::cloud::CliBridgeAuthStatus::Unavailable
    }
}

impl CliBridgeLoginLauncher for UnavailableCliAuthProbe {
    fn launch_login(
        &self,
        _provider: CliBridgeProvider,
    ) -> Result<Arc<dyn InteractiveLoginTransport>, CliBridgeLoginLaunchError> {
        Err(CliBridgeLoginLaunchError::Unavailable)
    }
}

/// Axum state for the model-access router. Holds the
/// [`CloudAccessProvider`], typed [`CliBridgeAuthStatusProbe`], and backend-owned
/// [`CliBridgeLoginLauncher`] seams—no key material, no [`crate::AppState`]—so a
/// route test can mount [`routes`] with injected providers and never build a
/// full `AppState`, touch the host keychain, or invoke installed CLIs.
#[derive(Clone)]
pub struct ModelAccessState {
    provider: Arc<dyn CloudAccessProvider>,
    cli_auth_probe: Arc<dyn CliBridgeAuthStatusProbe>,
    /// Live in-app login sessions. Shared (not per-request) so the start /
    /// poll / input / cancel routes address the SAME running login process.
    cli_login_sessions: Arc<CliBridgeLoginSessionRegistry>,
}

impl ModelAccessState {
    /// Production keychain wiring with CLI status fail-closed. The real server
    /// uses [`Self::production_with_cli_runtime`] after it has built and
    /// pinned the canonical launch configurations.
    pub fn production() -> Self {
        Self {
            provider: Arc::new(ProductionCloudAccessProvider),
            cli_auth_probe: Arc::new(UnavailableCliAuthProbe),
            cli_login_sessions: unavailable_login_sessions(),
        }
    }

    pub fn production_with_cli_auth_probe(
        cli_auth_probe: Arc<dyn CliBridgeAuthStatusProbe>,
    ) -> Self {
        Self {
            provider: Arc::new(ProductionCloudAccessProvider),
            cli_auth_probe,
            cli_login_sessions: unavailable_login_sessions(),
        }
    }

    pub fn production_with_cli_runtime(
        cli_auth_probe: Arc<dyn CliBridgeAuthStatusProbe>,
        cli_login_launcher: Arc<dyn CliBridgeLoginLauncher>,
    ) -> Self {
        Self {
            provider: Arc::new(ProductionCloudAccessProvider),
            cli_auth_probe,
            cli_login_sessions: Arc::new(CliBridgeLoginSessionRegistry::new(cli_login_launcher)),
        }
    }

    /// Wire an explicit provider. Tests inject an in-memory-vault-backed provider
    /// (200/400/404 paths) or a keychain-unavailable provider (503 path).
    pub fn with_provider(provider: Arc<dyn CloudAccessProvider>) -> Self {
        Self {
            provider,
            cli_auth_probe: Arc::new(UnavailableCliAuthProbe),
            cli_login_sessions: unavailable_login_sessions(),
        }
    }

    /// Wire both seams explicitly. Route tests use a typed fake auth probe, so
    /// they never inspect host credential files or invoke installed CLIs.
    pub fn with_provider_and_cli_auth_probe(
        provider: Arc<dyn CloudAccessProvider>,
        cli_auth_probe: Arc<dyn CliBridgeAuthStatusProbe>,
    ) -> Self {
        Self {
            provider,
            cli_auth_probe,
            cli_login_sessions: unavailable_login_sessions(),
        }
    }

    pub fn with_provider_cli_runtime(
        provider: Arc<dyn CloudAccessProvider>,
        cli_auth_probe: Arc<dyn CliBridgeAuthStatusProbe>,
        cli_login_launcher: Arc<dyn CliBridgeLoginLauncher>,
    ) -> Self {
        Self {
            provider,
            cli_auth_probe,
            cli_login_sessions: Arc::new(CliBridgeLoginSessionRegistry::new(cli_login_launcher)),
        }
    }
}

fn unavailable_login_sessions() -> Arc<CliBridgeLoginSessionRegistry> {
    Arc::new(CliBridgeLoginSessionRegistry::new(Arc::new(
        UnavailableCliAuthProbe,
    )))
}

fn access_config_api_error(err: AccessConfigError) -> ApiError {
    match err {
        AccessConfigError::ProviderNotOffered(detail) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "provider_not_offered", "detail": detail})),
        ),
        AccessConfigError::EmptyKey => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "empty_api_key"})),
        ),
        AccessConfigError::KeychainUnavailable => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "keychain_unavailable"})),
        ),
        AccessConfigError::Vault(vault_err) => {
            let vault_error_code = match vault_err {
                SecretsVaultError::EmptyLaneId => "empty_lane_id",
                SecretsVaultError::EmptySecretValue => "empty_secret_value",
                SecretsVaultError::NoSecretForLane(_) => "no_secret_for_lane",
                SecretsVaultError::LockPoisoned(_) => "lock_poisoned",
                SecretsVaultError::KeychainBackend(_) => "keychain_backend",
            };
            tracing::error!(
                target: "handshake_core::model_access",
                vault_error_code = vault_error_code,
                "model_access_vault_error"
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "vault_error"})),
            )
        }
    }
}

/// Resolve a `:provider` path segment to an offered BYOK provider, or a typed
/// 404. Deliberately-excluded providers (e.g. `gemini`) resolve to `None` and
/// therefore 404 — they can never be configured through this surface.
fn resolve_byok(provider_id: &str) -> Result<ByokProvider, ApiError> {
    ByokProvider::from_id(provider_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "provider_not_offered",
                "detail": format!("{provider_id} is not an offered BYOK provider"),
            })),
        )
    })
}

/// `GET /model-access/providers` — the non-secret enumeration surface.
async fn list_providers(State(state): State<ModelAccessState>) -> Result<Json<Value>, ApiError> {
    // Both keychain access and provider-owned CLI probes may block. Keep them
    // off the async request executor; each CLI child is independently bounded.
    let provider = state.provider.clone();
    let cli_auth_probe = state.cli_auth_probe.clone();
    let enumeration: CloudAccessEnumeration = tokio::task::spawn_blocking(move || {
        // Keychain disabled / backend error: still return a well-formed
        // enumeration (BYOK unavailable, typed CLI auth statuses) rather than
        // erroring, so the picker degrades gracefully.
        match provider.access() {
            Ok(svc) => svc.enumerate_with_cli_auth_probe(cli_auth_probe.as_ref()),
            Err(_) => enumerate_with_cli_auth_probe(
                &InMemoryAccessRegistry::new(),
                cli_auth_probe.as_ref(),
            ),
        }
    })
    .await
    .unwrap_or_else(|_| {
        enumerate_with_cli_auth_probe(&InMemoryAccessRegistry::new(), &UnavailableCliAuthProbe)
    });
    Ok(Json(serde_json::to_value(enumeration).unwrap_or_else(
        |_| json!({"byok": [], "cli_bridge": [], "excluded": ["gemini"]}),
    )))
}

/// `PUT /model-access/byok/:provider/key` — store a BYOK key in the vault.
async fn store_byok_key(
    State(state): State<ModelAccessState>,
    Path(provider_id): Path<String>,
    Json(body): Json<StoreKeyBody>,
) -> Result<Json<Value>, ApiError> {
    let provider = resolve_byok(&provider_id)?;
    let svc = state.provider.access().map_err(access_config_api_error)?;
    svc.store_byok_key(provider, &body.api_key)
        .map_err(access_config_api_error)?;
    // Non-secret confirmation only — never echo the key.
    Ok(Json(json!({
        "provider": provider.id(),
        "status": "configured",
    })))
}

/// `DELETE /model-access/byok/:provider/key` — remove / rotate a key.
async fn remove_byok_key(
    State(state): State<ModelAccessState>,
    Path(provider_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let provider = resolve_byok(&provider_id)?;
    let svc = state.provider.access().map_err(access_config_api_error)?;
    svc.remove_byok_key(provider)
        .map_err(access_config_api_error)?;
    Ok(Json(json!({
        "provider": provider.id(),
        "status": "unavailable",
    })))
}

/// Map a session-lookup/drive failure to a typed envelope. The session id is a
/// server-generated UUID, so echoing it back carries no operator data.
fn login_session_api_error(error: CliLoginSessionError) -> ApiError {
    match error {
        CliLoginSessionError::UnknownSession => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "cli_login_session_not_found"})),
        ),
        CliLoginSessionError::SessionFinished => (
            StatusCode::CONFLICT,
            Json(json!({"error": "cli_login_session_finished"})),
        ),
        CliLoginSessionError::InputFailed => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "cli_login_input_failed"})),
        ),
    }
}

fn login_snapshot_json(
    snapshot: &crate::model_runtime::cloud::CliLoginSessionSnapshot,
) -> Result<Json<Value>, ApiError> {
    serde_json::to_value(snapshot).map(Json).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "cli_login_session_encode_failed"})),
        )
    })
}

/// `POST /model-access/cli-bridge/:provider/login` — start the provider's own
/// official login as an IN-APP pseudo-terminal session.
async fn launch_cli_login(
    State(state): State<ModelAccessState>,
    Path(provider_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let provider = CliBridgeProvider::from_id(&provider_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "provider_not_offered"})),
        )
    })?;
    let sessions = state.cli_login_sessions.clone();
    // Process creation, executable-graph verification, and the ledger START
    // durability wait all block; keep them off the async request executor.
    let snapshot = tokio::task::spawn_blocking(move || sessions.start(provider))
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "cli_login_launch_failed"})),
            )
        })?
        .map_err(|error| match error {
            CliBridgeLoginLaunchError::Unavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"error": "cli_login_unavailable"})),
            ),
            CliBridgeLoginLaunchError::LaunchFailed => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "cli_login_launch_failed"})),
            ),
        })?;
    // MT-015: tee the cloud-access login into the live debug console (non-authoritative
    // observability; carries only the provider id + the server-generated session id — never
    // key material, never the login transcript). `publish_parts` is infallible + non-blocking
    // and can never affect this handler's result.
    crate::console_stream::ConsoleBroadcast::shared().publish_parts(
        crate::console_stream::ConsoleSeverity::Info,
        crate::console_stream::ConsoleCategory::CloudAccess,
        format!("cli-bridge:{}", provider.id()),
        format!(
            "official-CLI login session started in-app (provider={}, session={})",
            provider.id(),
            snapshot.session_id
        ),
        None,
    );
    login_snapshot_json(&snapshot)
}

/// `GET /model-access/cli-bridge-login/:session` — poll one login session.
async fn get_cli_login_session(
    State(state): State<ModelAccessState>,
    Path(session_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let snapshot = state
        .cli_login_sessions
        .snapshot(&session_id)
        .map_err(login_session_api_error)?;
    login_snapshot_json(&snapshot)
}

/// Request body for one operator response typed into the in-app login panel.
/// It is a plain provider prompt answer (a device code, a `y`/`n`, an account
/// selection) — never a Handshake-held credential, and never echoed back.
#[derive(Deserialize)]
struct LoginInputBody {
    input: String,
}

/// `POST /model-access/cli-bridge-login/:session/input`
async fn send_cli_login_input(
    State(state): State<ModelAccessState>,
    Path(session_id): Path<String>,
    Json(body): Json<LoginInputBody>,
) -> Result<Json<Value>, ApiError> {
    let snapshot = state
        .cli_login_sessions
        .send_input(&session_id, &body.input)
        .map_err(login_session_api_error)?;
    login_snapshot_json(&snapshot)
}

/// `POST /model-access/cli-bridge-login/:session/cancel`
async fn cancel_cli_login_session(
    State(state): State<ModelAccessState>,
    Path(session_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let snapshot = state
        .cli_login_sessions
        .cancel(&session_id)
        .map_err(login_session_api_error)?;
    login_snapshot_json(&snapshot)
}

pub fn routes(state: ModelAccessState) -> Router {
    Router::new()
        .route("/model-access/providers", get(list_providers))
        .route(
            "/model-access/byok/:provider/key",
            put(store_byok_key).delete(remove_byok_key),
        )
        .route(
            "/model-access/cli-bridge/:provider/login",
            post(launch_cli_login),
        )
        .route(
            "/model-access/cli-bridge-login/:session",
            get(get_cli_login_session),
        )
        .route(
            "/model-access/cli-bridge-login/:session/input",
            post(send_cli_login_input),
        )
        .route(
            "/model-access/cli-bridge-login/:session/cancel",
            post(cancel_cli_login_session),
        )
        .with_state(state)
}

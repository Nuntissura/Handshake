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
//! real server wires [`ModelAccessState::production`] (OS keychain); a route
//! test injects an in-memory-vault-backed provider via
//! [`ModelAccessState::with_provider_and_cli_auth_probe`] and mounts [`routes`]
//! directly — it never builds a full [`crate::AppState`], touches the host
//! keychain, or invokes an installed CLI.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, put},
    Json, Router,
};
use secrecy::SecretString;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::model_runtime::cloud::access_config::{
    enumerate_with_cli_auth_probe, AccessConfigError, ByokProvider, CliBridgeAuthStatusProbe,
    CloudAccessEnumeration, CloudModelAccess, InMemoryAccessRegistry,
};
use crate::model_runtime::cloud::SecretsVaultError;

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

/// Axum state for the model-access router. Holds the
/// [`CloudAccessProvider`] and typed [`CliBridgeAuthStatusProbe`] seams — no key
/// material, no [`crate::AppState`] — so a route test can mount [`routes`] with
/// injected providers and never build a full `AppState` (which needs a live
/// PostgreSQL pool), touch the host keychain, or invoke installed CLIs.
#[derive(Clone)]
pub struct ModelAccessState {
    provider: Arc<dyn CloudAccessProvider>,
    cli_auth_probe: Arc<dyn CliBridgeAuthStatusProbe>,
}

impl ModelAccessState {
    /// Production keychain wiring with CLI status fail-closed. The real server
    /// uses [`Self::production_with_cli_auth_probe`] after it has built and
    /// pinned the canonical launch configurations.
    pub fn production() -> Self {
        Self {
            provider: Arc::new(ProductionCloudAccessProvider),
            cli_auth_probe: Arc::new(UnavailableCliAuthProbe),
        }
    }

    pub fn production_with_cli_auth_probe(
        cli_auth_probe: Arc<dyn CliBridgeAuthStatusProbe>,
    ) -> Self {
        Self {
            provider: Arc::new(ProductionCloudAccessProvider),
            cli_auth_probe,
        }
    }

    /// Wire an explicit provider. Tests inject an in-memory-vault-backed provider
    /// (200/400/404 paths) or a keychain-unavailable provider (503 path).
    pub fn with_provider(provider: Arc<dyn CloudAccessProvider>) -> Self {
        Self {
            provider,
            cli_auth_probe: Arc::new(UnavailableCliAuthProbe),
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
        }
    }
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

pub fn routes(state: ModelAccessState) -> Router {
    Router::new()
        .route("/model-access/providers", get(list_providers))
        .route(
            "/model-access/byok/:provider/key",
            put(store_byok_key).delete(remove_byok_key),
        )
        .with_state(state)
}

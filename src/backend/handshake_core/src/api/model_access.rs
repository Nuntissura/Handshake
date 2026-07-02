//! MT-015: Operator cloud-model access configuration HTTP surface.
//!
//! The native egui settings dialog reaches the WP-KERNEL-004 cloud backends
//! through these routes. They wrap [`crate::model_runtime::cloud::access_config`]:
//!
//! * `GET  /model-access/providers`            — non-secret enumeration for the
//!   model picker (configured / unavailable per provider; CLI-bridge login
//!   commands; the deliberately-excluded Gemini). Never returns key material.
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
//! [`ModelAccessState::with_provider`] and mounts [`routes`] directly — it never
//! builds a full [`crate::AppState`] and never touches the host keychain.

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
    AccessConfigError, ByokProvider, CloudAccessEnumeration, CloudModelAccess,
};

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

/// Axum state for the model-access router. Holds only the [`CloudAccessProvider`]
/// seam — no key material, no [`crate::AppState`] — so a route test can mount
/// [`routes`] with an injected provider and never build a full `AppState`
/// (which needs a live PostgreSQL pool) or touch the host keychain.
#[derive(Clone)]
pub struct ModelAccessState {
    provider: Arc<dyn CloudAccessProvider>,
}

impl ModelAccessState {
    /// Production wiring (OS keychain). Used by the real server in `api::routes`.
    pub fn production() -> Self {
        Self {
            provider: Arc::new(ProductionCloudAccessProvider),
        }
    }

    /// Wire an explicit provider. Tests inject an in-memory-vault-backed provider
    /// (200/400/404 paths) or a keychain-unavailable provider (503 path).
    pub fn with_provider(provider: Arc<dyn CloudAccessProvider>) -> Self {
        Self { provider }
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
            // The vault error Display never contains key material (it carries a
            // lane id + backend reason only), but we still map to a stable code
            // rather than echoing internals.
            tracing::error!(
                target: "handshake_core::model_access",
                error = %vault_err,
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
    // Keychain disabled / backend error: still return a well-formed enumeration
    // (everything unavailable) rather than erroring, so the picker degrades
    // gracefully.
    let enumeration: CloudAccessEnumeration = match state.provider.access() {
        Ok(svc) => svc.enumerate(),
        Err(_) => crate::model_runtime::cloud::access_config::enumerate(
            &crate::model_runtime::cloud::access_config::InMemoryAccessRegistry::new(),
        ),
    };
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

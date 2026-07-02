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
//!   [`secrecy::SecretString`] end-to-end; it is never logged, echoed, or
//!   persisted anywhere but the keychain. Storing a key creates NO consent
//!   receipt (MT-006 fail-closed gate still applies at first launch).
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
use crate::AppState;

type ApiError = (StatusCode, Json<Value>);

/// Request body for storing a BYOK key. The key deserialises straight into a
/// [`SecretString`] so it is redacted in `Debug` and cannot be logged by
/// accident. Nothing else is accepted.
#[derive(Deserialize)]
struct StoreKeyBody {
    api_key: SecretString,
}

/// Build the production access service, mapping a disabled keychain to a
/// typed 503 rather than a panic.
fn service() -> Result<CloudModelAccess, ApiError> {
    CloudModelAccess::production().map_err(access_config_api_error)
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
async fn list_providers(State(_state): State<AppState>) -> Result<Json<Value>, ApiError> {
    // Keychain disabled: still return a well-formed enumeration (everything
    // unavailable) rather than erroring, so the picker degrades gracefully.
    let enumeration: CloudAccessEnumeration = match service() {
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
    State(_state): State<AppState>,
    Path(provider_id): Path<String>,
    Json(body): Json<StoreKeyBody>,
) -> Result<Json<Value>, ApiError> {
    let provider = resolve_byok(&provider_id)?;
    let svc = service()?;
    svc.store_byok_key(provider, &body.api_key)
        .map_err(access_config_api_error)?;
    // Non-secret confirmation only.
    Ok(Json(json!({
        "provider": provider.id(),
        "status": "configured",
    })))
}

/// `DELETE /model-access/byok/:provider/key` — remove / rotate a key.
async fn remove_byok_key(
    State(_state): State<AppState>,
    Path(provider_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let provider = resolve_byok(&provider_id)?;
    let svc = service()?;
    svc.remove_byok_key(provider)
        .map_err(access_config_api_error)?;
    Ok(Json(json!({
        "provider": provider.id(),
        "status": "unavailable",
    })))
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/model-access/providers", get(list_providers))
        .route(
            "/model-access/byok/:provider/key",
            put(store_byok_key).delete(remove_byok_key),
        )
        .with_state(state)
}
